//! Observes metrics in YAML format.
//!
//! Metric scopes are used to provide the hierarchy and indentation of metrics.  As an example, for
//! a snapshot with two metrics — `server.msgs_received` and `server.msgs_sent` — we would
//! expect to see this output:
//!
//! ```c
//! server:
//!   msgs_received: 42
//!   msgs_sent: 13
//! ```
//!
//! If we added another metric — `configuration_reloads` — we would expect to see:
//!
//! ```c
//! configuration_reloads: 2
//! server:
//!   msgs_received: 42
//!   msgs_sent: 13
//! ```
//!
//! Metrics are sorted alphabetically.
//!
//! ## Histograms
//!
//! Histograms are rendered with a configurable set of quantiles that are provided when creating an
//! instance of `YamlBuilder`.  They are formatted using human-readable labels when displayed to
//! the user.  For example, 0.0 is rendered as "min", 1.0 as "max", and anything in between using
//! the common "pXXX" format i.e. a quantile of 0.5 or percentile of 50 would be p50, a quantile of
//! 0.999 or percentile of 99.9 would be p999, and so on.
//!
//! All histograms have the sample count of the histogram provided in the output.
//!
//! ```c
//! connect_time count: 15
//! connect_time min: 1334
//! connect_time p50: 1934
//! connect_time p99: 5330
//! connect_time max: 139389
//! ```
//!
#![deny(missing_docs)]
use crate::config::Config;
use hdrhistogram::Histogram;
use humantime::format_duration;
use metrics_core::{Builder, Drain, Key, Label, Observer};
use metrics_util::{parse_quantiles, MetricsTree, Quantile};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::io;

/// Builder for [`YamlObserver`].
pub struct UiBuilder {
    quantiles: Vec<Quantile>,
}

impl UiBuilder {
    /// Creates a new [`UiBuilder`] with default values.
    pub fn new() -> Self {
        let quantiles = parse_quantiles(&[0.0, 0.5, 0.9, 1.0]);

        Self { quantiles }
    }

    /// Sets the quantiles to use when rendering histograms.
    ///
    /// Quantiles represent a scale of 0 to 1, where percentiles represent a scale of 1 to 100, so
    /// a quantile of 0.99 is the 99th percentile, and a quantile of 0.99 is the 99.9th percentile.
    ///
    /// By default, the quantiles will be set to: 0.0, 0.5, 0.9, 0.95, 0.99, 0.999, and 1.0.
    pub fn set_quantiles(mut self, quantiles: &[f64]) -> Self {
        self.quantiles = parse_quantiles(quantiles);
        self
    }
}

impl Builder for UiBuilder {
    type Output = UiObserver;

    fn build(&self) -> Self::Output {
        UiObserver {
            quantiles: self.quantiles.clone(),
            tree: MetricsTree::default(),
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }
}

/// Observess metrics in YAML format.
pub struct UiObserver {
    pub(crate) quantiles: Vec<Quantile>,
    pub(crate) tree: MetricsTree,
    pub(crate) counters: HashMap<Key, u64>,
    pub(crate) gauges: HashMap<Key, i64>,
    pub(crate) histograms: HashMap<Key, Histogram<u64>>,
}

impl Observer for UiObserver {
    fn observe_counter(&mut self, key: Key, value: u64) {
        self.counters.insert(key, value);
        //let (levels, name) = key_to_parts(key);
        //self.tree.insert_value(levels, name, value);
    }

    fn observe_gauge(&mut self, key: Key, value: i64) {
        self.gauges.insert(key, value);
        //let (levels, name) = key_to_parts(key);
        //self.tree.insert_value(levels, name, value);
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

use ordered_float::OrderedFloat;
use crate::metric_names::BYTES_PER_SEC_SUFFIX;

impl Drain<String> for UiObserver {
    fn drain(&mut self) -> String {
        let mut map = BTreeMap::new();

        let mut sorted_histos: Vec<(Key, Histogram<u64>)> = self.histograms.drain().collect();

        sorted_histos.sort_by_cached_key(|a| OrderedFloat(a.1.mean()));

        for (key, h) in sorted_histos {
            if key.name().ends_with(BYTES_PER_SEC_SUFFIX) {
                let mean_bytes_per_sec = h.mean() / (1024.0 * 1024.0);

                map.insert(
                    key.name().to_ascii_lowercase(),
                    format!("avg rate {:.2} MiB/s", mean_bytes_per_sec)
                );

            } else {
                let mean_duration = std::time::Duration::from_nanos(h.mean() as u64);

                map.insert(
                    key.name().to_ascii_lowercase(),
                    format!("avg time {}", format_duration(mean_duration).to_string()),
                );
            }
        }

        for (key, value) in self.counters.drain() {
            map.insert(
                key.name().to_ascii_lowercase(),
                format!("count {}", value));
        }

        for (key, value) in self.gauges.drain() {
            map.insert(
                key.name().to_ascii_lowercase(),
                format!("gauge {}", value));
        }

        let rendered =
            String::from(serde_yaml::to_string(&map).expect("failed to render yaml output"));

        // clear for next time we do an observe
        self.tree.clear();

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
) -> Vec<(String, String)> {
    let mut values = Vec::new();

    let mean_duration = std::time::Duration::from_nanos(hist.mean() as u64);

    values.push((
        (format!("{}", name)),
        (format!("{}", format_duration(mean_duration))),
    ));

    /*values.push((format!("{} count", name), hist.len()));
    hist.mean();
    for quantile in quantiles {
        let value = hist.value_at_quantile(quantile.value());
        values.push((format!("{} {}", name, quantile.label()), value));
    } */

    values
}
