use crate::config::Config;
use crate::metric_names::{
    METRIC_OVERALL_TRANSFERRED_BYTES, METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC,
};
use hdrhistogram::Histogram;
use metrics_core::{Builder, Drain, Key, Label, Observer};
use metrics_util::{parse_quantiles, MetricsTree, Quantile};
use std::collections::HashMap;
use std::io;
use std::time::Duration;

pub struct ProgressObserver {
    // copies of the histogram that we observe at the snapshot time
    histograms: HashMap<Key, Histogram<u64>>,

    // our observation of amount transferred
    transferred: u64,
}

impl ProgressObserver {
    pub fn new() -> Self {
        Self {
            histograms: HashMap::new(),
            transferred: 0,
        }
    }

    pub fn transferred(&mut self) -> u64 {
        return self.transferred;
    }

    pub fn render(&mut self, elapsed: Duration) -> String {

        // we build a result out of whatever we want to show
        let mut render_result: String = String::new();

        {
            let avg_display: String;

            let elapsed_seconds = elapsed.as_secs_f64();

            if elapsed_seconds > 0.0 {
                let bytes_per_sec = self.transferred as f64 / elapsed_seconds;

                avg_display = format!("at {:.2} MiB/s", bytes_per_sec / (1024.0 * 1024.0));
            } else {
                avg_display = format!("at - MiB/s");
            }

            render_result.push_str(avg_display.as_str());
        }
        render_result.push_str(" connections rates ~ ");
        {
            let mut rate_displays = Vec::new();

            // loop through our histograms looking for all those that are 'rate bytes'
            // we then calculate the average of the relevant histogram and return a tuple
            // of (name, avg)
            let mut slot_rates = self
                .histograms
                .iter()
                .filter_map(|histo| {
                    let (name, labels) = histo.0.clone().into_parts();
                    if name.ends_with(METRIC_SLOT_TRANSFER_RATE_BYTES_PER_SEC) {
                        return Some((name, histo.1.mean() / (1024.0 * 1024.0)));
                    } else {
                        return None;
                    }
                })
                .collect::<Vec<_>>();

            // we need to keep this sorted by key so that the display is stable (otherwise
            // the relative order of the 'rate' readings would jump around)
            slot_rates.sort_by_key(|a| a.0.clone());

            for a in slot_rates {
                rate_displays.push(format!("{:.0}", a.1));
            }

            render_result.push_str(&rate_displays.join("/"));
        }

        return render_result;
    }
}

impl Observer for ProgressObserver {
    fn observe_counter(&mut self, key: Key, value: u64) {
        let (name, labels) = key.into_parts();

        if name.eq(METRIC_OVERALL_TRANSFERRED_BYTES) {
            self.transferred = value;
        }
    }

    fn observe_gauge(&mut self, key: Key, value: i64) {
        let (name, labels) = key.into_parts();
    }

    fn observe_histogram(&mut self, key: Key, values: &[u64]) {
        let entry = self
            .histograms
            .entry(key)
            .or_insert_with(|| Histogram::<u64>::new(3).expect("failed to create histogram"));

        for value in values {
            entry
                .record(*value)
                .expect("failed to observe histogram value");
        }
    }
}
