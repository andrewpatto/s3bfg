use crate::metric_observer_progress::ProgressObserver;
use indicatif::{ProgressBar, ProgressStyle};
use metrics_core::Observe;
use metrics_runtime::Controller;
use std::time::{Duration, Instant};

const WAIT_SECONDS: u64 = 1;

/// Display a continually updating status of the file copy.
/// Note this worker is blocking and so care must be taken to make sure
/// it runs on a blocking aware worker thread.
///
pub fn progress_worker(controller: Controller, size: u64) {
    let start = Instant::now();

    let sty = ProgressStyle::default_bar()
        .template(
            "\r[{elapsed_precise}] {bar:20.cyan/blue} [{eta_precise}] {bytes}/{total_bytes} {msg}",
        )
        .progress_chars("#>-");

    let pb = ProgressBar::new(size);
    pb.set_style(sty.clone());

    loop {
        let mut observer = ProgressObserver::new();

        controller.observe(&mut observer);

        let msg = observer.render(Instant::now() - start);

        pb.set_message(msg.as_str());
        pb.set_position(observer.transferred());

        std::thread::sleep(Duration::from_secs(WAIT_SECONDS));
    }

    pb.finish_with_message("done");
}
