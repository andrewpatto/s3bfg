extern crate num_cpus;
extern crate ureq;

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader as IoBufReader};
use std::net::{Ipv4Addr, SocketAddr};
use std::{str, iter};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::join;
use rand::{Rng, thread_rng};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader as TokioBufReader;
use tokio::net::{lookup_host, TcpStream};
use tokio::net::UdpSocket;
use tokio::prelude::*;
use tokio::runtime::Runtime;
use tokio::time;
use trust_dns_client::client::{AsyncClient, Client, ClientHandle, SyncClient};
use trust_dns_client::op::{DnsResponse, ResponseCode};
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_client::rr::rdata::key::KEY;
use trust_dns_client::udp::{UdpClientConnection, UdpClientStream};

use crate::datatype::{BlockToStream, ConnectionTracker};
use rand::distributions::Alphanumeric;

/*pub fn populate_by_file(connection_tracker: &Arc<ConnectionTracker>, filename: &str) -> io::Result<()> {
    let ips_file = File::open(filename)?;
    let ips_reader = IoBufReader::new(ips_file);

    let mut ips_db = connection_tracker.ips.lock().unwrap();

    for line in ips_reader.lines() {
        ips_db.insert(line.unwrap(), 0);
    }

    Ok(())
}*/

/// Populates the connection tracker database with random S3 ip addresses.
///
/// # Examples
///
/// ```
/// ```
pub fn populate_by_dns(connection_tracker: &Arc<ConnectionTracker>, dns_server_addr: &SocketAddr, dns_threads: usize) -> io::Result<()> {

    fn random_s3_fqdn() -> String {
        let mut rng = thread_rng();

        // using a random bucket name  increases our chances of avoiding DNS caches along the way
        // because this is going to a resolve we construct FQDN s3 names
        let chars: String = iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(7)
            .collect();

        return format!("{}.s3.ap-southeast-2.amazonaws.com.", chars);
    }

    (0..dns_threads).into_par_iter()
        .for_each(|_| {
            let conn = UdpClientConnection::new(*dns_server_addr).unwrap();

            // and then create the Client
            let client = SyncClient::new(conn);

            let name = Name::from_ascii(random_s3_fqdn()).unwrap();

            // Send the query and get a message response, see RecordType for all supported options
            let q = client.query(&name, DNSClass::IN, RecordType::A);

            if q.is_ok() {
                let response: DnsResponse = q.unwrap();

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

        });

    Ok(())
}
