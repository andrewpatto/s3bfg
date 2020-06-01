use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::str;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::time;

use futures::join;
use rand::Rng;
use tokio::io::{
    AsyncRead,
    AsyncReadExt,
    AsyncWrite,
    AsyncWriteExt,
    BufReader,
    BufWriter,
    AsyncBufRead,
    AsyncBufReadExt,
    copy, split
};
use std::io::Write;
use tokio::net::{lookup_host, TcpStream};
use tokio::runtime;
use tokio::stream::Stream;
use tokio::time::{delay_for};
use tokio_rustls::{rustls::ClientConfig, TlsConnector, webpki::DNSNameRef};
use tokio_rustls::client::TlsStream;
use tokio::fs::OpenOptions;


use crate::datatype::BlockToStream;
use futures::io::{Cursor, SeekFrom};
use crate::copy_exact::copy_exact;
use crate::config::Config;

struct ConnectionTracker {
    map: Mutex<BTreeMap<String, u32>>,
    done: Mutex<bool>
}


pub async fn async_execute(ip: &str, blocks: &[BlockToStream], cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {

    // because our start up is expensive we don't even want to go there will be nothing to do
    if blocks.is_empty() {
        return Ok(())
    }

    let s3_ip = (ip, 443)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

    // the underlying tcp stream for our connection to the designated S3 IP can be created first
    let tcp_stream = TcpStream::connect(&s3_ip).await?;

    // the TLS connector is the rusttls layer that will do the TLS handshake for us
    let mut tls_config = ClientConfig::new();

    tls_config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let connector = TlsConnector::from(Arc::new(tls_config));

    let domain_str = format!("{}.s3-{}.amazonaws.com", cfg.input_bucket_name, cfg.input_bucket_region);

    let domain = DNSNameRef::try_from_ascii_str(domain_str.as_str())?;
//        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let mut stream = connector.connect(domain, tcp_stream).await?;

    let (mut reader, mut writer) = split(stream);

    let mut buf_reader = BufReader::new(reader);

    let mut file_writer = OpenOptions::new().write(true).create(false).open(&cfg.output_write_filename).await?;

    let longest = blocks.iter().max_by_key(|b| b.length).unwrap().length;

    let mut memory_buffer = vec![0u8; longest as usize];

    let mut c = 0;

    for b in blocks {
        let block_started = Instant::now();

        // build into a buffer so we can send in one go
        let mut req: Vec<u8> = vec![];

        // request line
        write!(
            req,
            "GET {} HTTP/1.1\r\n",
            cfg.input_bucket_key
        )?;

        // headers
        write!(req, "Host: {}.s3-{}.amazonaws.com\r\n", cfg.input_bucket_name, cfg.input_bucket_region)?;
        write!(req, "User-Agent: s3bigfile\r\n")?;
        write!(req, "Accept: */*\r\n")?;
        write!(req, "Range: bytes={}-{}\r\n", b.start,  b.start + b.length - 1)?;

        // end of headers
        write!(req, "\r\n")?;

        writer.write_all(req.as_slice()).await?;

        loop {
            let mut headers = String::new();
            let line_length = buf_reader.read_line(&mut headers).await?;

            // TODO: should detect if the server returns Connection: closed in which case this is our last possible get

            // otherwise we get this - when we come around *after* the close
            if line_length == 0 {
                return Result::Err(Box::from(io::Error::new(io::ErrorKind::InvalidInput, "connection closed")));
            }

            //print!("{}", headers);

            // our headers will be terminated by a single line "\r\n"
            if line_length == 2 {
                break;
            }
        }

        let mut copied_bytes = 0u64;

        if cfg.memory_only {
            let copied_stats = copy_exact(&mut buf_reader, &mut memory_buffer, b.length).await?;

            copied_bytes = copied_stats.0;
        }
        else {
            file_writer.seek(SeekFrom::Start(b.start)).await?;

            let copied_stats = copy_exact(&mut buf_reader, &mut file_writer, b.length).await?;

            copied_bytes = copied_stats.0;
        }

        let block_duration = Instant::now().duration_since(block_started);

        // let read_count = buf_reader.read_exact(&mut memory_buffer).await?;

        println!("{}-{}: {} in {} at {} MiB/s", ip, c, copied_bytes, block_duration.as_secs_f32(), (copied_bytes as f32 / (1024.0*1024.0)) / block_duration.as_secs_f32());

        c += 1;
       // delay_for(Duration::from_secs(5)).await;

       // println!("sleep ended");

        //let num_bytes = buf_reader.read_line(&mut buf).await;

        //println!("{:?}", num_bytes)

        // stream_http_block(&stream, bucket_name, bucket_key, b.start, b.length).await;
    }



    /*   let content = format!(
          "GET / HTTP/1.0\r\nHost: {}\r\n\r\n",
          "gos-test-cases-public.s3.ap-southeast-2.amazonaws.com"
      );





      let (mut stdin, mut stdout) = (tokio_stdin(), tokio_stdout());



      stream.write_all(content.as_bytes()).await?;

      let (mut reader, mut writer) = split(stream);

         future::select(
              copy(&mut reader, &mut stdout),
              copy(&mut stdin, &mut writer)
          )
              .await
              .factor_first()
              .0?;




    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let res = client.get("https://gos-test-cases-public.s3.ap-southeast-2.amazonaws.com/GIB1_8398.bam".parse()?).await?;

    println!("{:?}", res.status());
    println!("{:?}", res.headers());
 */
    // assert_eq!(res.status(), 200);

    Ok(())



   // join!(ip_generator(&db), handle_request(&db));

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

    let ips = connection_tracker.map.lock().unwrap();

    for ip in ips.iter() {
        println!("Ending with {} used {} times", ip.0, ip.1);
    }
}

pub async fn async_process_block(stream_id: &str, tcp_host_addr: SocketAddr, s3_bucket: &str, s3_key: &str, read_start: u64, read_length: u64, write_filename: &str, write_location: u64) {

}