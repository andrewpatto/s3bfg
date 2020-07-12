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
use hdrhistogram::Histogram;
use metrics_core::{Builder, Drain, Key, Label, Observer};
use metrics_util::{parse_quantiles, MetricsTree, Quantile};
use std::collections::HashMap;
use std::io;
use termion::{input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{BarChart, Block, Borders},
    Terminal,
};
use tui::widgets::Gauge;
use crate::config::Config;

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
            histos: HashMap::new(),
            transferred: 0
        }
    }
}

//impl Default for UiBuilder {
//    fn default() -> Self {
//        Self::new()
//    }
//}

/// Observess metrics in YAML format.
pub struct UiObserver {
    pub(crate) quantiles: Vec<Quantile>,
    pub(crate) tree: MetricsTree,
    pub(crate) histos: HashMap<Key, Histogram<u64>>,

    transferred: u64,
}

impl UiObserver {
    pub fn render(&mut self, config: &Config) {
        for (key, h) in self.histos.drain() {
            let (levels, name) = key_to_parts(key);
            let values = hist_to_values(name, h.clone(), &self.quantiles);
            println!("{:?}", values);
            self.tree.insert_values(levels, values);
        }

        let data = vec![
            ("B1", 9),
            ("B2", 12),
            ("B3", 5),
            ("B4", 8),
            ("B5", 2),
            ("B6", 4),
            ("B7", 5),
            ("B8", 9),
            ("B9", 14),
            ("B10", 15),
            ("B11", 1),
            ("B12", 0),
            ("B13", 4),
            ("B14", 6),
            ("B15", 4),
            ("B16", 6),
            ("B17", 4),
            ("B18", 7),
            ("B19", 13),
            ("B20", 8),
            ("B21", 11),
            ("B22", 9),
            ("B23", 3),
            ("B24", 5),
        ];

        //let stdout = io::stdout().into_raw_mode().unwrap();
        let stdout = AlternateScreen::from(io::stdout());
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());
            let barchart = BarChart::default()
                .block(Block::default().title("Data1").borders(Borders::ALL))
                .data(&data)
                .bar_width(9)
                .style(Style::default().fg(Color::Yellow))
                .value_style(Style::default().fg(Color::Black).bg(Color::Yellow));
            f.render_widget(barchart, chunks[0]);

            let progress = Gauge::default()
                .block(Block::default().title("Gauge1").borders(Borders::ALL))
                .style(Style::default().fg(Color::Yellow))
                .ratio((self.transferred as f64) / (config.file_size_bytes as f64));


            f.render_widget(progress, chunks[1]);
        });

        self.tree.clear();
    }
}

impl Observer for UiObserver {

    fn observe_counter(&mut self, key: Key, value: u64) {
        let (levels, name) = key_to_parts(key);

        if name == "bytes_transferred" {
            self.transferred = value;
        }
    }

    fn observe_gauge(&mut self, key: Key, value: i64) {
        let (levels, name) = key_to_parts(key);
        self.tree.insert_value(levels, name, value);
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

impl Drain<String> for UiObserver {
    fn drain(&mut self) -> String {
        for (key, h) in self.histos.drain() {
            let (levels, name) = key_to_parts(key);
            let values = hist_to_values(name, h.clone(), &self.quantiles);
            self.tree.insert_values(levels, values);
        }

        let rendered = String::from("hi"); // serde_yaml::to_string(&self.tree).expect("failed to render yaml output");
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
) -> Vec<(String, u64)> {
    let mut values = Vec::new();

    values.push((format!("{} count", name), hist.len()));
    for quantile in quantiles {
        let value = hist.value_at_quantile(quantile.value());
        values.push((format!("{} {}", name, quantile.label()), value));
    }

    values
}
