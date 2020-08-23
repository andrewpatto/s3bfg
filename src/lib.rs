#[macro_use]
extern crate lazy_static;
extern crate nix;
#[macro_use]
extern crate simple_error;

// we are not actually building a library for general usage - this is just
// exposing the code used by the CLI tests to the integration tests
pub mod asynchronous_download;
pub mod built_info;
pub mod config;
pub mod copy_exact;
pub mod download_block;
pub mod empty_file;
pub mod metric_names;
pub mod metric_observer_progress;
pub mod metric_observer_ui;
pub mod s3_info;
pub mod s3_ip_pool;
pub mod s3_request_signed;
pub mod s3_uris;
pub mod setup_aws_credentials;
pub mod setup_metrics;
pub mod setup_tokio;
pub mod ui_console;
