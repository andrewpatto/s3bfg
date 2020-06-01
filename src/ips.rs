extern crate rand;
extern crate ureq;

use std::{iter, str};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader as IoBufReader};
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::join;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader as TokioBufReader;
use tokio::net::{lookup_host, TcpStream, UdpSocket};
use tokio::prelude::*;
use tokio::time;
use trust_dns_client::client::{AsyncClient, Client, ClientHandle, SyncClient};
use trust_dns_client::op::{DnsResponse, ResponseCode};
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_client::rr::rdata::key::KEY;
use trust_dns_client::udp::{UdpClientConnection, UdpClientStream};

use crate::datatype::{BlockToStream, ConnectionTracker};
use crate::config::Config;

/*pub fn populate_by_file(connection_tracker: &Arc<ConnectionTracker>, filename: &str) -> io::Result<()> {
    let ips_file = File::open(filename)?;
    let ips_reader = IoBufReader::new(ips_file);

    let mut ips_db = connection_tracker.ips.lock().unwrap();

    for line in ips_reader.lines() {
        ips_db.insert(line.unwrap(), 0);
    }

    Ok(())
}*/

/// Populates the connection tracker database with random S3 IP addresses.
///
/// # Examples
///
/// ```
/// ```
pub async fn populate_a_dns(connection_tracker: &Arc<ConnectionTracker>, cfg: &Config) -> io::Result<()> {
    fn random_s3_fqdn(c: &Config) -> String {
        let mut rng = thread_rng();

        // using a random bucket name  increases our chances of avoiding DNS caches along the way
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

    //let mut runtime = Runtime::new().unwrap();

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
