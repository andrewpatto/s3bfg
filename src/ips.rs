extern crate ureq;
extern crate num_cpus;

use std::str;

use crate::datatype::BlockToStream;
use tokio::net::{TcpStream, lookup_host};
use tokio::prelude::*;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncBufReadExt;
use tokio::time;
use tokio::io::BufReader;
use std::net::SocketAddr;
use std::time::{Instant, Duration};
use rand::Rng;
use std::sync::{Mutex, Arc};
use std::collections::BTreeMap;
use futures::join;

struct ConnectionTracker {
    map: Mutex<BTreeMap<String, u32>>,
    done: Mutex<bool>
}

fn random_s3_host() -> String {
    return format!("{}.s3.ap-southeast-2.amazonaws.com:80", rand::thread_rng()
        .gen_ascii_chars()
        .take(5)
        .collect::<String>());
}

#[tokio::main]
pub async fn async_execute(blocks: &Vec<BlockToStream>, bucket_host: &str, bucket_name: &str, bucket_key: &str, write_filename: &str, prevent_duplicates: bool, start_jitter: f64) -> io::Result<()> {

    // we use a round robin collection of S3 IPs
    let mut initial_db = BTreeMap::new();


    // before we start though we initialise with whatever addresses we can get
    for addr in tokio::net::lookup_host(random_s3_host()).await? {
        println!("{}", addr);
        initial_db.insert(addr.to_string(), 0);
    }

    let db = Arc::new(ConnectionTracker {
        map: Mutex::new(initial_db),
        done: Mutex::new(false)
    });

    join!(ip_generator(&db), handle_request(&db));

    //let mut count = 0;

    //loop {


/*        let when = Instant::now() + Duration::from_millis(100);

        let task = tokio::time::Delay::new(when);

        task.and_then(|_| async {


                task.reset(Instant::now() + Duration::from_millis(100));

                Ok(())
            })
            .map_err(|e| panic!("delay errored; err={:?}", e));

        task.await?; */

       // count = count+1;

       // if (count > 100) {
       //     break;
      //      }
    //}

   /* let tcp_stream = tokio::net::TcpStream::connect("127.0.0.1:8080").await?;
    let mut stream = tokio::io::BufReader::new(tcp_stream);

    stream.write_all(b"hello world!").await?;

    let mut line = String::new();
    stream.read_line(&mut line).await.unwrap();

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(false)
        .open(write_filename)
        .await
        .unwrap();

    while let Some(v) = stream.next().await {
        file.write_all(&v).await.unwrap();
    } */


  /*  let fetches = futures::stream::iter(
        blocks.into_iter().map(|block| {
            async move {
                match reqwest::get(&path).await {
                    Ok(resp) => {
                        match resp.text().await {
                            Ok(text) => {
                                println!("RESPONSE: {} bytes from {}", text.len(), path);
                            }
                            Err(_) => println!("ERROR reading {}", path),
                        }
                    }
                    Err(_) => println!("ERROR downloading {}", path),
                }
            }
        })
    ).buffer_unordered(8).collect::<Vec<()>>();

    println!("Waiting...");

    fetches.await;

    println!("got {:?}", res); */
    Ok(())
}

/**
 An IP generator that continually looks for new S3 IP addresses and places them into
 our connection list
 */
async fn ip_generator(connection_tracker: &Arc<ConnectionTracker>) -> () {
    let mut interval = time::interval(Duration::from_millis(100));

    loop {
        interval.tick().await;

        let mut ips = connection_tracker.map.lock().unwrap();

        for addr in tokio::net::lookup_host(random_s3_host()).await.unwrap() {
            if !ips.contains_key(&addr.to_string()) {
                println!("Discovered new S3 IP {}", addr);
                ips.insert(addr.to_string(), 0);
            }
        }

        let d = connection_tracker.done.lock().unwrap();

        if !*d {
            break;
        }

    }
}


async fn handle_request(connection_tracker: &Arc<ConnectionTracker>) -> () {

    for x in 0..10 {
        let mut dest: SocketAddr;

        {
            let mut ips = connection_tracker.map.lock().unwrap();

            let which = rand::thread_rng().gen_range(0, ips.len());

            let choice = ips.iter().nth(which).unwrap();
            let ip = choice.0;
            let count = choice.1;

            println!("Chose {:?}", choice);
        }

        tokio::time::delay_for(time::Duration::new(1,0)).await;
    }

    let mut d = connection_tracker.done.lock().unwrap();

    *d = true;

    let mut ips = connection_tracker.map.lock().unwrap();

    for ip in ips.iter() {
        println!("Ending with {} used {} times", ip.0, ip.1);
    }
}

pub async fn async_process_block(stream_id: &str, tcp_host_addr: SocketAddr, s3_bucket: &str, s3_key: &str, read_start: u64, read_length: u64, write_filename: &str, write_location: u64) {

}