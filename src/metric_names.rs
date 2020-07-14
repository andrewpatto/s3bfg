pub const METRIC_OVERALL_TRANSFER_STARTED: &str = "transfer_started";
pub const METRIC_OVERALL_TRANSFER_BYTES: &str = "transfer_bytes";

pub const METRIC_OVERALL_NETWORK_READ_OP_SIZE: &str = "network_read_size";
pub const METRIC_OVERALL_DISK_WRITE_OP_SIZE: &str = "disk_write_size";

pub const METRIC_SLOT_RATE_BYTES_PER_SEC: &str = "slot_rate_bytess_per_sec";
pub const METRIC_SLOT_STATE_SETUP: &str = "slot_state_setup_timing";
pub const METRIC_SLOT_TCP_SETUP: &str = "slot_tcp_setup_timing";
pub const METRIC_SLOT_SSL_SETUP: &str = "slot_ssl_setup_timing";
pub const METRIC_SLOT_REQUEST: &str = "slot_request_timing";
pub const METRIC_SLOT_RESPONSE: &str = "slot_response_timing";

// I own up - I don't really understand the correct way to do lifetimes in Rust!
// It makes sense that metrics wants it labels to be `static - but not sure how
// I can do that without literally instantiated like this
pub const THREAD_LABELS: &[&str] = &[
    "thread0", "thread1", "thread2", "thread3", "thread4", "thread5", "thread6", "thread7",
];
