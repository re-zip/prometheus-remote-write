use std::collections::HashMap;

use magnus::{Error, RHash, RString, Ruby, function, value::Qnil};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use serde_magnus::deserialize;
use tokio::runtime::Builder;
use tokio::sync::mpsc;

struct RemoteWriteQueue {
    spawn: mpsc::Sender<RemoteWriteArgs>,
}

impl RemoteWriteQueue {
    pub fn new() -> Self {
        let (send, mut recv) = mpsc::channel::<RemoteWriteArgs>(16);

        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        std::thread::spawn(move || {
            rt.block_on(async move {
                while let Some(args) = recv.recv().await {
                    tokio::spawn(async move {
                        crate::remote_write(&args.metrics, &args.labels, &args.url, &args.headers)
                            .await
                            .unwrap();
                    });
                }
            });
        });

        Self { spawn: send }
    }

    pub fn spawn_task(&self, task: RemoteWriteArgs) {
        match self.spawn.blocking_send(task) {
            Ok(()) => {}
            Err(_) => panic!("The shared runtime has shut down."),
        }
    }
}

static REMOTE_WRITE_QUEUE: Lazy<RemoteWriteQueue> = Lazy::new(|| RemoteWriteQueue::new());

#[derive(Serialize, Deserialize, Debug)]
struct RemoteWriteArgs {
    metrics: HashMap<String, f64>,
    labels: HashMap<String, String>,
    url: String,
    headers: HashMap<String, String>,
}

use crate::CRATE_VERSION;

fn get_version(ruby: &Ruby) -> RString {
    ruby.str_new(CRATE_VERSION)
}

fn remote_write(ruby: &Ruby, args: RHash) -> Result<Qnil, Error> {
    let args: RemoteWriteArgs = deserialize(args)?;
    REMOTE_WRITE_QUEUE.spawn_task(args);
    Ok(ruby.qnil())
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("PrometheusRemoteWrite")?;
    module.define_module_function("get_version", function!(get_version, 0))?;
    module.define_module_function("remote_write", function!(remote_write, 1))?;
    Ok(())
}
