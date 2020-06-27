use std::collections::BTreeMap;
use std::iter;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use futures::future::join_all;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusoto_core::Region;
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio::runtime::Builder;
use trust_dns_client::client::{AsyncClient, ClientHandle};
use trust_dns_client::op::DnsResponse;
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_client::udp::UdpClientStream;

/// A thread-safe data structure for pooling S3 endpoints (IP addresses)
/// and recording the usage of them.
///
pub struct S3IpPool {
    // a map of IP addresses that have been identified as active S3 servers, and
    // the count of the number of times we have chosen them for use
    pub ips: Mutex<BTreeMap<String, u32>>,

    // a system wide mutex to indicate we are done (if lots of async activity that
    // may not have an easy trigger to stop)
    pub done: Mutex<bool>,
}

impl S3IpPool {
    /// Returns a new thread-safe S3 IP address pool, initially populated with
    /// no endpoints.
    pub fn new() -> S3IpPool {
        S3IpPool {
            ips: Mutex::new(BTreeMap::new()),
            done: Mutex::new(false),
        }
    }

    /// Returns the count of S3 IP addresses currently in our pool.
    ///
    pub fn ip_count(&self) -> u16 {
        return self.ips.lock().unwrap().len() as u16;
    }

    /// Returns an IP address from our pool that has been used the least, and the
    /// current count for that IP address.
    ///
    pub fn use_least_used_ip(&self) -> (IpAddr, u32) {
        let mut ips_unmutex = self.ips.lock().unwrap();

        // our S3 endpoint with the lowest usage so far
        let lowest_usage = ips_unmutex.iter_mut().min_by_key(|x| *x.1);

        // access the whole entry
        let (ip, count) = lowest_usage.unwrap();

        // bump the count
        *count = *count + 1;

        return (ip.parse::<IpAddr>().unwrap(), *count);
    }

    /// Populates the pool with entries we fetch in parallel from a DNS server and
    /// returns the number of entries we ended up with.
    ///
    pub fn populate_ips(
        &self,
        region: &Region,
        dns_server: &str,
        desired: u16,
        rounds: u16,
        concurrency: u16,
        round_delay: Duration,
    ) -> u16 {
        // we use tokio for this bit because the DNS lookup library that used
        // tokio gave the most control
        let mut dns_rt = Builder::new()
            .enable_all()
            .threaded_scheduler()
            .build()
            .unwrap();

        for round in 0..rounds {
            let mut dns_futures = Vec::new();

            for _c in 0..concurrency {
                dns_futures.push(self.populate_a_dns(dns_server, region.name().as_ref()));
            }

            let res: Vec<Result<u32, std::io::Error>> = dns_rt.block_on(join_all(dns_futures));

            print!("DNS round {:>2}: ", round+1);

            for r in res {
                if r.is_ok() {
                    print!("{:>3}", r.unwrap());
                } else {
                    print!("{:>3}", "x");
                }
            }

            let now_count = self.ip_count();

            if now_count < desired {
                println!(" (end total {})", now_count);
                sleep(round_delay);
            } else {
                println!();
                return now_count;
            }
        }

        // if we fall through to here then we've given up on reaching our 'desired' count
        // but we don't want to waste any more time looking
        return self.ip_count();
    }

    /// Populates the S3 IP pool with random S3 IP addresses
    /// from *one* attempt to lookup an S3 host, and returns the number of new
    /// IP addresses that were added.
    ///
    /// # Examples
    ///
    /// ```
    /// populate_a_dns("8.8.8.8:53", "ap-southeast-2")
    /// ```
    async fn populate_a_dns(&self, dns_server: &str, bucket_region: &str) -> io::Result<u32> {
        let socket_addr = dns_server.parse().unwrap();

        // the best feature of trust_dns is the timeout that means that we won't hang around
        // waiting for packets not coming back
        let stream =
            UdpClientStream::<UdpSocket>::with_timeout(socket_addr, Duration::from_millis(50));

        // this little snippet should get better as tokio settles down.. its not a particularly
        // pretty pattern that trust_dns uses (though the results are good)
        let (mut client, mut bg) = AsyncClient::connect(stream).await?;

        tokio::spawn(bg);

        let name = Name::from_ascii(random_s3_fqdn(bucket_region))?;

        // send the query and get a message response, see RecordType for all supported options
        let query = client.query(name, DNSClass::IN, RecordType::A).await;

        let mut added_count = 0u32;

        if query.is_ok() {
            let response: DnsResponse = query.unwrap();

            // we will not necessarily only get the DNS answer we ask for
            let answers: &[Record] = response.answers();

            // quick processing of the answers whilst we hold a lock on our db mutex
            {
                let mut ips = self.ips.lock().unwrap();

                for ans in answers {
                    // where the answer fits into the A data structure
                    // we see if this is a new IP and if so, add it
                    if let &RData::A(ref ip) = ans.rdata() {
                        if !ips.contains_key(&ip.to_string()) {
                            ips.insert(ip.to_string(), 0);
                            added_count += 1;
                        }
                    }
                }
            }
        }

        Ok(added_count)
    }
}

/// Create a random bucket name in the S3 domain space - hoping that
/// this will maximise our chance of getting new round robin IP addresses for S3 targets
///
fn random_s3_fqdn(br: &str) -> String {
    let mut rng = thread_rng();

    // using a random bucket name increases our chances of avoiding DNS caches along the way
    // (s3 buckets are meant to be lowercase so we do the same)
    let chars: String = iter::repeat(())
        .map(|()| rng.sample(Alphanumeric).to_ascii_lowercase())
        .take(7)
        .collect();

    return format!("{}.s3-{}.amazonaws.com.", chars, br);
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
