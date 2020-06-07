extern crate rand;

use std::{iter};
use std::fs::File;
use std::io::{BufRead, BufReader as IoBufReader};
use std::sync::{Arc};
use std::time::{Duration};

use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use tokio::net::{lookup_host, TcpStream, UdpSocket};
use tokio::prelude::*;
use tokio::time;
use trust_dns_client::client::{AsyncClient, ClientHandle};
use trust_dns_client::op::{DnsResponse};
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_client::udp::{UdpClientStream};

use crate::datatype::ConnectionTracker;
use crate::config::Config;


/// Populates the connection tracker database with random S3 IP addresses
/// from one attempt to lookup an S3 host.
///
/// # Examples
///
/// ```
/// ```
pub async fn populate_a_dns(connection_tracker: &Arc<ConnectionTracker>, cfg: &Config) -> io::Result<()> {
    fn random_s3_fqdn(c: &Config) -> String {
        let mut rng = thread_rng();

        // using a random bucket name increases our chances of avoiding DNS caches along the way
        // because this is going to a resolve we construct FQDN s3 names
        let chars: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(7)
            .collect();

        return format!("{}.s3-{}.amazonaws.com.", chars, c.input_bucket_region);
    }

    let socket_addr = cfg.dns_server.parse().unwrap();

    let stream = UdpClientStream::<UdpSocket>::with_timeout(socket_addr, Duration::from_millis(50));

    let mut client = AsyncClient::connect(stream).await.unwrap();

    tokio::spawn(client.1);

    let name = Name::from_ascii(random_s3_fqdn(cfg)).unwrap();

    // Send the query and get a message response, see RecordType for all supported options
    let query = client.0.query(name, DNSClass::IN, RecordType::A).await;

    // let q = runtime.block_on(query);

    if query.is_ok() {
        let response: DnsResponse = query.unwrap();

        // we will not necessarily only get the DNS answer we ask for
        let answers: &[Record] = response.answers();

        // quick process of the answers whilst we insert into our shared db
        {
            let mut ips = connection_tracker.ips.lock().unwrap();

            for ans in answers {
                // where the answer fits into the A data structure
                if let &RData::A(ref ip) = ans.rdata() {
                    if !ips.contains_key(&ip.to_string()) {
                        ips.insert(ip.to_string(), 0);
                    }
                }
            }
        }
    }

    Ok(())
}
