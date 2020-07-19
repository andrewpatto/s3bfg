use crate::metric_observer_progress::ProgressObserver;
use indicatif::{ProgressBar, ProgressStyle};
use metrics_core::Observe;
use metrics_runtime::Controller;
use std::time::{Duration, Instant};

pub fn progress_worker(controller: Controller, size: u64) {
    let start = Instant::now();

    //let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template(
            "\r[{elapsed_precise}] {bar:20.cyan/blue} [{eta_precise}] {bytes}/{total_bytes} {msg}",
        )
        .progress_chars("#>-");

    let pb = ProgressBar::new(size);
    pb.set_style(sty.clone());
    // pb.println(format!("[+] finished #"));

    loop {
        let mut observer = ProgressObserver::new();

        controller.observe(&mut observer);

        let current = Instant::now();
        let elapsed = current - start;

        let msg = observer.render(elapsed);

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
