use std::sync::Mutex;
use std::collections::BTreeMap;

pub struct BlockToStream {
    pub start: u64,
    pub length: u64
}

pub struct ConnectionTracker {
    // a map of IP addresses that have been identified as active S3 servers, and
    // the count of the number of times we have connected to them
    pub ips: Mutex<BTreeMap<String, u32>>,

    // a system wide mutex to indicate we are done (if lots of async activity that
    // may not have an easy trigger to stop)
    pub done: Mutex<bool>,
}

impl ConnectionTracker {
    pub fn new() -> ConnectionTracker {
        ConnectionTracker {
            ips: Mutex::new(BTreeMap::new()),
            done: Mutex::new(false),
        }
    }
}
