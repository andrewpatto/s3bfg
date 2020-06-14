use std::io;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::str;
use std::sync::{Arc};
use std::time::{Duration, Instant};

use futures::io::{SeekFrom};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use tokio::fs::OpenOptions;
use tokio::io::{
    AsyncBufReadExt,
    AsyncReadExt,
    AsyncWriteExt,
    BufReader,
    split
};
use tokio::net::TcpStream;
use tokio::runtime::{Builder, Runtime};
use tokio_rustls::{rustls::ClientConfig, TlsConnector, webpki::DNSNameRef};
use tokio_rustls::client::TlsStream;

use crate::config::Config;
use crate::copy_exact::copy_exact;
use crate::datatype::{BlockToStream, ConnectionTracker};

pub fn async_execute(connection_tracker: &Arc<ConnectionTracker>, blocks: &Vec<BlockToStream>, config: &Config, bucket_region: &str) {
    println!("Starting a tokio asynchronous copy");

    // a tokio runtime we will use for our async io
    // if we are being asked to work in async mode then construct a Tokio runtime
    // builder using any command line args set
    let mut rt: Runtime;
    let mut rt_description: String = String::new();

    if config.asynchronous_basic {
        rt = Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .unwrap();
        rt_description.push_str("basic");
    } else {
        let mut temp = Builder::new();

        temp.enable_all();
        temp.threaded_scheduler();

        rt_description.push_str("threaded ");

        if config.asynchronous_core_threads > 0 {
            temp.core_threads(config.asynchronous_core_threads);
            rt_description.push_str(config.asynchronous_core_threads.to_string().as_str());
            rt_description.push_str("(core)/");
        } else {
            rt_description.push_str("default(core)/");
        }
        if config.asynchronous_max_threads > 0 {
            temp.max_threads(config.asynchronous_max_threads);
            rt_description.push_str(config.asynchronous_max_threads.to_string().as_str());
            rt_description.push_str("(max)");
        } else {
            rt_description.push_str("512(max)");
        }

        rt =  temp.build().unwrap();
    }

    let ips_db = connection_tracker.ips.lock().unwrap();

    let futures = FuturesUnordered::new();

    {
        let mut starter: usize = 0;

        let connectors = if config.s3_connections < ips_db.len() { config.s3_connections } else { ips_db.len() };

        let connection_chunk = blocks.len() / connectors;

        for (count, (ip, _)) in ips_db.iter().enumerate() {
            if count >= connectors {
                break;
            }

            futures.push(async_execute_work(&ip.as_str(), &blocks[starter..starter + connection_chunk], &config, bucket_region));

            starter += connection_chunk;
        }
    }

    println!("Tokio runtime is set up to operate with {} config, across {} potential S3 endpoints", rt_description, futures.len());

    println!("Using {} connections to S3", futures.len());

    // spawn a task waiting on all the async streams to finish
    let res = rt.block_on(futures.into_future());

    println!("err: {:?}", res);

    rt.shutdown_timeout(Duration::from_millis(100));
}

pub async fn async_execute_work(ip: &str, blocks: &[BlockToStream], cfg: &Config, bucket_region: &str) -> Result<(), Box<dyn std::error::Error>> {

    // because our start up is expensive we don't even want to go there will be nothing to do
    if blocks.is_empty() {
        return Ok(())
    }

    // if we want to allocate single buffers then we need to know the max size we will face
    let _longest = blocks.iter().max_by_key(|b| b.length).unwrap().length;

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

    let domain_str = format!("{}.s3-{}.amazonaws.com", cfg.input_bucket_name, bucket_region);

    let domain = DNSNameRef::try_from_ascii_str(domain_str.as_str())?;
//        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    let stream: TlsStream<TcpStream> = connector.connect(domain, tcp_stream).await?;

    let (reader, mut writer) = split(stream);

    let mut buf_reader = BufReader::new(reader);

    // we hope to make this memory buffer only once and then continually read into it
    // (helps us make the zeroing out of the vector content a one off)
    let mut memory_buffer = vec![0u8; _longest as usize];

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
        write!(req, "Host: {}.s3-{}.amazonaws.com\r\n", cfg.input_bucket_name, bucket_region)?;
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

        let copied_bytes: u64;

        if cfg.memory_only {
            if b.length != memory_buffer.len() as u64 {
                memory_buffer.resize(b.length as usize, 0);
            }

            copied_bytes = buf_reader.read_exact(&mut memory_buffer).await? as u64;
        }
        else {
            let mut file_writer = OpenOptions::new().write(true).create(false).open(&cfg.output_write_filename.as_ref().unwrap()).await?;

            file_writer.seek(SeekFrom::Start(b.start)).await?;

            let copied_stats = copy_exact(&mut buf_reader, &mut file_writer, b.length).await?;

            copied_bytes = copied_stats.0;
        }

        assert_eq!(copied_bytes, b.length);

        let block_duration = Instant::now().duration_since(block_started);

        println!("{}-{}: {} in {} at {} MiB/s", ip, c, copied_bytes, block_duration.as_secs_f32(), (copied_bytes as f32 / (1024.0*1024.0)) / block_duration.as_secs_f32());

        c += 1;
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
