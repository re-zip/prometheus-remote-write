# frozen_string_literal: true

require "rake/testtask"
require "rake/extensiontask"

task default: :test

Rake::ExtensionTask.new("prometheus_remote_write") do |c|
  c.lib_dir = "lib/prometheus_remote_write"
end

task :dev do
  ENV["RB_SYS_CARGO_PROFILE"] = "dev"
end

Rake::TestTask.new do |t|
  t.deps << :dev << :compile
  t.test_files = FileList[File.expand_path("test/*_test.rb", __dir__)]
end
