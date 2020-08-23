use crate::config::Config;
use metrics_runtime::Receiver;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Returns a metrics reciever and metrics level based on our config.
///
pub fn create_metrics(_config: &Config) -> (Receiver, u8) {
    let receiver = Receiver::builder()
        // 2 hrs worth of room for stats!
        .histogram(Duration::from_secs(2 * 60 * 60), Duration::from_secs(60))
        .build()
        .expect("failed to create receiver");

    (receiver, 1)
}
