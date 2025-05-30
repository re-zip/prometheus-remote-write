use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

mod ruby_bindings;

pub const CRATE_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(prost::Message, Clone, PartialEq)]
pub struct WriteRequest {
    #[prost(message, repeated, tag = "1")]
    pub timeseries: Vec<TimeSeries>,
}

#[derive(prost::Message, Clone, PartialEq)]
pub struct TimeSeries {
    #[prost(message, repeated, tag = "1")]
    pub labels: Vec<Label>,
    #[prost(message, repeated, tag = "2")]
    pub samples: Vec<Sample>,
}

#[derive(prost::Message, Clone, Hash, PartialEq, Eq)]
pub struct Label {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

#[derive(prost::Message, Clone, PartialEq)]
pub struct Sample {
    #[prost(double, tag = "1")]
    pub value: f64,
    #[prost(int64, tag = "2")]
    pub timestamp: i64,
}

impl WriteRequest {
    fn sort(&mut self) {
        for series in &mut self.timeseries {
            series.sort_labels_and_samples();
        }
    }
    pub async fn run(
        mut self,
        url: &str,
        headers: &HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.sort();
        let body = snap::raw::Encoder::new().compress_vec(&prost::Message::encode_to_vec(&self))?;
        let client = reqwest::Client::new();
        client
            .post(url)
            .headers(
                headers
                    .iter()
                    .flat_map(|(k, v)| Some((k.parse().ok()?, v.parse().ok()?)))
                    .collect(),
            )
            .header("Content-Type", "application/x-protobuf")
            .header("X-Prometheus-Remote-Write-Version", "0.1.0")
            .body(body)
            .send()
            .await?;
        Ok(())
    }
}

impl TimeSeries {
    pub fn sort_labels_and_samples(&mut self) {
        self.labels.sort_by(|a, b| a.name.cmp(&b.name));
        self.samples.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }
}

pub async fn remote_write(
    metrics: &HashMap<String, f64>,
    labels: &HashMap<String, String>,
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time travel")
        .as_millis() as i64;
    let write = WriteRequest {
        timeseries: metrics
            .iter()
            .map(|(metric_name, metric_value)| TimeSeries {
                labels: labels
                    .iter()
                    .map(|(label_name, label_value)| Label {
                        name: label_name.to_string(),
                        value: label_value.to_string(),
                    })
                    .chain(vec![Label {
                        name: "__name__".to_string(),
                        value: metric_name.to_string(),
                    }])
                    .collect(),
                samples: vec![Sample {
                    value: *metric_value,
                    timestamp,
                }],
            })
            .collect(),
    };
    write.run(url, headers).await?;
    Ok(())
}
