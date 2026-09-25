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

use crate::adapter::sockudo::{SockudoAdapter, SockudoConfig};
use crate::adapter::sse::SseAdapter;
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

    #[arg(long, default_value = "http://127.0.0.1:8090")]
    pb_url: String,

    #[arg(long, default_value = "bench")]
    pb_conversation: String,

    #[arg(long, default_value = "http://127.0.0.1:6001")]
    sockudo_url: String,

    #[arg(long, default_value = "ws://127.0.0.1:6001")]
    sockudo_ws_url: String,

    #[arg(long, default_value = "bench-app")]
    sockudo_app_id: String,

    #[arg(long, default_value = "bench-key")]
    sockudo_app_key: String,

    #[arg(long, default_value = "bench-secret")]
    sockudo_app_secret: String,

    #[arg(long, default_value = "bench-channel")]
    sockudo_channel: String,

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

    if args.payload_bytes < 16 {
        anyhow::bail!("payload-bytes must be at least 16.");
    }

    let adapter: Arc<dyn TransportAdapter> = match args.transport.as_str() {
        "ws" => Arc::new(WsAdapter::new(args.url.clone())),
        "sse" => Arc::new(SseAdapter::new(
            args.pb_url.clone(),
            args.pb_conversation.clone(),
        )),
        "sockudo" => Arc::new(SockudoAdapter::new(SockudoConfig {
            http_base: args.sockudo_url.clone(),
            ws_base: args.sockudo_ws_url.clone(),
            app_id: args.sockudo_app_id.clone(),
            app_key: args.sockudo_app_key.clone(),
            app_secret: args.sockudo_app_secret.clone(),
            channel: args.sockudo_channel.clone(),
        })),
        other => anyhow::bail!("unsupported transport: {other}"),
    };

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

        // For SSE, the conversation field in SseAdapter is already pb_conversation,
        // but connect() takes the conversation ID as well.
        let conversation_clone = if args.transport == "sse" {
            args.pb_conversation.clone()
        } else {
            args.conversation.clone()
        };

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
    let conversation_clone = if args.transport == "sse" {
        args.pb_conversation.clone()
    } else {
        args.conversation.clone()
    };

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
