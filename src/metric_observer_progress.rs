#![deny(missing_docs)]
use crate::config::Config;
use crate::metric_names::{
    METRIC_OVERALL_TRANSFER_BYTES, METRIC_OVERALL_TRANSFER_STARTED, METRIC_SLOT_RATE_BYTES_PER_SEC,
};
use hdrhistogram::Histogram;
use metrics_core::{Builder, Drain, Key, Label, Observer};
use metrics_util::{parse_quantiles, MetricsTree, Quantile};
use std::collections::HashMap;
use std::io;

pub struct ProgressObserver {
    quantiles: Vec<Quantile>,
    histos: HashMap<Key, Histogram<u64>>,

    started: i64,
    transferred: u64,
}

impl ProgressObserver {
    pub fn new() -> Self {
        Self {
            quantiles: parse_quantiles(&[0.0, 0.5, 1.0]),
            histos: HashMap::new(),
            started: 0,
            transferred: 0,
        }
    }

    pub fn render(&mut self, size: u64, now: u64) -> String {
        let mut rates_display: String = String::new();
        {
            // loop through our histograms looking for all those that are 'rate bytes'
            // we then calculate the average of the relevant histogram and return a tuple
            // of (name, avg)
            let mut slot_rates = self
                .histos
                .iter()
                .filter_map(|histo| {
                    let (name, labels) = histo.0.clone().into_parts();
                    if name.ends_with(METRIC_SLOT_RATE_BYTES_PER_SEC) {
                        return Some((
                            name,
                            histo.1.mean() / (1024.0 * 1024.0),
                        ));
                    } else {
                        return None;
                    }
                })
                .collect::<Vec<_>>();

            // we need to keep this sorted by key so that the display is stable (otherwise
            // the relative order of the 'rate' readings would jump around)
            slot_rates.sort_by_key(|a| a.0.clone());

            for a in slot_rates {
                rates_display.push_str(format!("{:5.2} ", a.1).as_str());
            }
        }

        let percent = ((self.transferred as f64 * 100.0) / (size as f64)) as u16;

        let avg_display: String;

        let elapsed_seconds = (now - self.started as u64) as f64 / (1000.0 * 1000.0 * 1000.0);

        if elapsed_seconds > 0.0 {
            let bytes_per_sec = self.transferred as f64 / elapsed_seconds;

            avg_display = format!("{:.2}", bytes_per_sec / (1024.0 * 1024.0));
        } else {
            avg_display = format!("-");
        }

        return format!("\r{:3}% {:5} MiB/s {}", percent, avg_display, rates_display);
    }
}

impl Observer for ProgressObserver {
    fn observe_counter(&mut self, key: Key, value: u64) {
        let (name, labels) = key.into_parts();

        if name.eq(METRIC_OVERALL_TRANSFER_BYTES) {
            self.transferred = value;
        }
    }

    fn observe_gauge(&mut self, key: Key, value: i64) {
        let (name, labels) = key.into_parts();

        if name.eq(METRIC_OVERALL_TRANSFER_STARTED) {
            self.started = value;
        }
    }

    fn observe_histogram(&mut self, key: Key, values: &[u64]) {
        let entry = self
            .histos
            .entry(key)
            .or_insert_with(|| Histogram::<u64>::new(3).expect("failed to create histogram"));

        for value in values {
            entry
                .record(*value)
                .expect("failed to observe histogram value");
        }
    }
}

impl Drain<String> for ProgressObserver {
    fn drain(&mut self) -> String {
        for (key, h) in self.histos.drain() {
            let (levels, name) = key_to_parts(key);
            let values = hist_to_values(name, h.clone(), &self.quantiles);
            //self.tree.insert_values(levels, values);
        }

        let rendered = String::from("hi"); // serde_yaml::to_string(&self.tree).expect("failed to render yaml output");
                                           // self.tree.clear();
        rendered
    }
}

fn key_to_parts(key: Key) -> (Vec<String>, String) {
    let (name, labels) = key.into_parts();
    let mut parts = name.split('.').map(ToOwned::to_owned).collect::<Vec<_>>();
    let name = parts.pop().expect("name didn't have a single part");

    let labels = labels
        .into_iter()
        .map(Label::into_parts)
        .map(|(k, v)| format!("{}=\"{}\"", k, v))
        .collect::<Vec<_>>()
        .join(",");
    let label = if labels.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", labels)
    };

    let fname = format!("{}{}", name, label);

    (parts, fname)
}

fn hist_to_values(
    name: String,
    hist: Histogram<u64>,
    quantiles: &[Quantile],
) -> Vec<(String, u64)> {
    let mut values = Vec::new();

    values.push((format!("{} count", name), hist.len()));
    for quantile in quantiles {
        let value = hist.value_at_quantile(quantile.value());
        values.push((format!("{} {}", name, quantile.label()), value));
    }

    values
}
