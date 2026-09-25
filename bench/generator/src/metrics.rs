use std::fs::OpenOptions;
use std::io::Write;

use anyhow::Context;
use hdrhistogram::Histogram;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Serialize)]
pub struct MetricEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_message_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<u32>,
    pub receiver_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_micros: Option<u64>,
}

pub struct MetricsConfig {
    pub receivers: u32,
    pub payload_bytes: usize,
    pub messages: u32,
    pub rate: f64,
    pub transport: String,
    pub output: String,
}

pub fn spawn_metrics_writer(
    mut rx: mpsc::UnboundedReceiver<String>,
    config: MetricsConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        let mut file = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&config.output)
            .context("failed to open output file")
        {
            Ok(f) => f,
            Err(e) => {
                error!("Metrics writer failed: {:#}", e);
                return;
            }
        };

        let mut latency_hist = Histogram::<u64>::new(3).unwrap();
        let mut conn_est_hist = Histogram::<u64>::new(3).unwrap(); // Optional if connection ms is tracked
        let mut received = 0;
        let mut first_conn_ts = None;

        while let Some(line) = rx.blocking_recv() {
            if let Err(e) = writeln!(file, "{}", line) {
                error!("Failed to write to metrics file: {:#}", e);
            }

            // Quick parsing to update histograms
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(latency) = parsed.get("latency_us").and_then(|v| v.as_u64()) {
                    latency_hist.record(latency).unwrap_or(());
                    received += 1;
                }

                if let Some(event) = parsed.get("event").and_then(|v| v.as_str()) {
                    if event == "connection_established" {
                        if let Some(ts) = parsed.get("ts_micros").and_then(|v| v.as_u64()) {
                            if first_conn_ts.is_none() {
                                first_conn_ts = Some(ts);
                            }
                            let last_conn_ts = Some(ts);
                            if let (Some(first), Some(last)) = (first_conn_ts, last_conn_ts) {
                                let elapsed = last.saturating_sub(first) / 1000;
                                conn_est_hist.record(elapsed).unwrap_or(());
                            }
                        }
                    }
                }
            }
        }

        let expected = config.receivers * config.messages;
        let completeness = if expected > 0 {
            (received as f64 / expected as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "transport={} receivers={} payload_bytes={} messages={} rate={}",
            config.transport, config.receivers, config.payload_bytes, config.messages, config.rate
        );
        println!(
            "  sent={}  received={}  expected={}  completeness={:.2}%",
            config.messages, received, expected, completeness
        );
        println!(
            "  latency_us: p50={}  p95={}  p99={}  max={}",
            latency_hist.value_at_quantile(0.50),
            latency_hist.value_at_quantile(0.95),
            latency_hist.value_at_quantile(0.99),
            latency_hist.max()
        );
        println!(
            "  connection_established_p50_ms={}  p95_ms={}",
            conn_est_hist.value_at_quantile(0.50),
            conn_est_hist.value_at_quantile(0.95)
        );
    })
}
