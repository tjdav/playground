use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::sync::{mpsc, watch, Barrier};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod adapter;
mod client;
mod metrics;
mod scenario;

use crate::adapter::ws::WsAdapter;
use crate::adapter::TransportAdapter;
use crate::metrics::MetricsConfig;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "ws")]
    transport: String,

    #[arg(long, default_value = "ws://127.0.0.1:8080")]
    url: String,

    #[arg(long, default_value = "bench")]
    conversation: String,

    #[arg(long, default_value_t = 10)]
    receivers: u32,

    #[arg(long, default_value_t = 100)]
    messages: u32,

    #[arg(long, default_value_t = 1024)]
    payload_bytes: usize,

    #[arg(long, default_value_t = 10.0)]
    rate: f64,

    #[arg(long, default_value_t = 5)]
    warmup_secs: u64,

    #[arg(long, default_value = "results.jsonl")]
    output: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let args = Args::parse();

    if args.transport != "ws" {
        anyhow::bail!("Only 'ws' transport is supported in this task.");
    }
    if args.payload_bytes < 16 {
        anyhow::bail!("payload-bytes must be at least 16.");
    }

    let adapter: Arc<dyn TransportAdapter> = Arc::new(WsAdapter::new(args.url.clone()));

    let (metrics_tx, metrics_rx) = mpsc::unbounded_channel();
    let metrics_config = MetricsConfig {
        receivers: args.receivers,
        payload_bytes: args.payload_bytes,
        messages: args.messages,
        rate: args.rate,
        transport: args.transport.clone(),
        output: args.output.clone(),
    };

    let writer_handle = metrics::spawn_metrics_writer(metrics_rx, metrics_config);

    // barrier size = receivers + 1 (for sender)
    let barrier = Arc::new(Barrier::new((args.receivers + 1) as usize));
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let mut receiver_handles = Vec::with_capacity(args.receivers as usize);

    for i in 0..args.receivers {
        let adapter_clone = adapter.clone();
        let conversation_clone = args.conversation.clone();
        let barrier_clone = barrier.clone();
        let metrics_tx_clone = metrics_tx.clone();
        let shutdown_rx_clone = shutdown_rx.clone();

        let handle = tokio::spawn(async move {
            let res = client::receiver_task(
                adapter_clone,
                conversation_clone,
                i + 1, // receiver_id starts at 1
                barrier_clone,
                metrics_tx_clone,
                shutdown_rx_clone,
            )
            .await;

            if let Err(e) = res {
                tracing::error!("Receiver task failed: {:#}", e);
                std::process::exit(1);
            }
        });
        receiver_handles.push(handle);
    }

    // Spawn sender task
    let adapter_clone = adapter.clone();
    let conversation_clone = args.conversation.clone();
    let barrier_clone = barrier.clone();
    let messages = args.messages;
    let rate = args.rate;
    let payload_bytes = args.payload_bytes;
    let warmup_secs = args.warmup_secs;

    let sender_handle = tokio::spawn(async move {
        let res = client::sender_task(
            adapter_clone,
            conversation_clone,
            barrier_clone,
            messages,
            rate,
            payload_bytes,
            warmup_secs,
        )
        .await;

        if let Err(e) = res {
            tracing::error!("Sender task failed: {:#}", e);
            std::process::exit(1);
        }
    });

    sender_handle.await?;

    info!("Sender task finished. Waiting for drain timeout (30s) or completion...");
    // Ideally we could track if all expected messages arrived. But a simple drain timeout works well.
    tokio::time::sleep(Duration::from_secs(30)).await;

    info!("Shutting down receivers...");
    let _ = shutdown_tx.send(true);

    // Wait with a small timeout or just drop, we already know they are exiting
    // but a proper await avoids panicked warnings for some tokio versions if they exit early.
    // However the issue states 'run completes without errors'.
    for handle in receiver_handles {
        let _ = handle.await;
    }

    drop(metrics_tx); // Close the channel

    writer_handle.await?;

    info!("Run completed successfully.");

    Ok(())
}
