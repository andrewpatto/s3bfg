pub const METRIC_OVERALL_TRANSFERRED_BYTES: &str = "overall-transferred_bytes";

pub const METRIC_OVERALL_NETWORK_READ_OP_SIZE: &str = "network_read_size";
pub const METRIC_OVERALL_DISK_WRITE_OP_SIZE: &str = "disk_write_size";

pub const BYTES_PER_SEC_SUFFIX: &str = "bytes_per_sec";
pub const TIMING_NANOSEC_SUFFIX: &str = "timing_nanosec";

// I'd really like to do this with some how referring to the above constants..
pub const METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC: &str = concat!("transfer_", "bytes_per_sec");
pub const METRIC_SLOT_NETWORK_RATE_BYTES_PER_SEC: &str = concat!("network_", "bytes_per_sec");
pub const METRIC_SLOT_DISK_RATE_BYTES_PER_SEC: &str = concat!("disk_", "bytes_per_sec");

pub const METRIC_SLOT_STATE_SETUP: &str = "slot_state_setup_timing_nanosec";
pub const METRIC_SLOT_TCP_SETUP: &str = "slot_tcp_setup_timing_nanosec";
pub const METRIC_SLOT_SSL_SETUP: &str = "slot_ssl_setup_timing_sec";
pub const METRIC_SLOT_REQUEST: &str = "slot_request_timing_sec";
pub const METRIC_SLOT_RESPONSE: &str = "slot_response_timing_sec";
