use metrics_runtime::Controller;
use indicatif::{ProgressStyle, ProgressBar};
use crate::metric_observer_progress::ProgressObserver;
use metrics_core::Observe;
use std::time::Duration;

pub fn progress_worker(controller: Controller, size: u64) {
    //let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template("\r{spinner:.green} [{elapsed_precise}] {bar:20.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
        .progress_chars("#>-");

    let pb = ProgressBar::new(size);
    pb.set_style(sty.clone());
    // pb.println(format!("[+] finished #"));

    loop {
        let mut observer = ProgressObserver::new();

        controller.observe(&mut observer);

        let msg = observer.render();

        pb.set_message(msg.as_str());
        pb.set_position(observer.transferred());

        //print!("{}", msg);
        //std::io::stdout().flush();
        //m.join().unwrap();

        std::thread::sleep(Duration::from_secs(1));
    }
    pb.finish_with_message("done");

    // m.join_and_clear().unwrap();
}
