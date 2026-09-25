use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use bytes::{Buf, BufMut, BytesMut};
use tokio::sync::{mpsc, watch, Barrier};
use tracing::{error, info};

use crate::adapter::TransportAdapter;
use crate::metrics::MetricEvent;
use crate::scenario;

pub async fn receiver_task(
    adapter: Arc<dyn TransportAdapter>,
    conversation_id: String,
    receiver_id: u32,
    barrier: Arc<Barrier>,
    metrics_tx: mpsc::UnboundedSender<String>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let mut connection = match adapter.connect(&conversation_id).await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Receiver {} failed to connect: {:#}", receiver_id, e);
            anyhow::bail!("Receiver {} failed to connect", receiver_id);
        }
    };

    let connect_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("time went backwards")?
        .as_micros() as u64;

    let event = MetricEvent {
        event: Some("connection_established"),
        client_message_id: None,
        sender_id: None,
        receiver_id,
        latency_us: None,
        payload_bytes: None,
        phase: None,
        ts_micros: Some(connect_ts),
    };
    if let Ok(json) = serde_json::to_string(&event) {
        let _ = metrics_tx.send(json);
    }

    // Signal ready and wait for others
    barrier.wait().await;

    loop {
        tokio::select! {
            result = connection.recv() => {
                match result {
                    Ok((payload, recv_ts)) => {
                        if payload.len() < 16 {
                            error!("Receiver {} got undersized payload: {} bytes", receiver_id, payload.len());
                            continue;
                        }

                        let mut buf = payload.as_ref();
                        let client_message_id = buf.get_u64();
                        let sender_ts = buf.get_u64();
                        let payload_bytes = payload.len();

                        let latency_us = recv_ts.saturating_sub(sender_ts);

                        let event = MetricEvent {
                            event: None,
                            client_message_id: Some(client_message_id),
                            sender_id: Some(0), // sender is always 0
                            receiver_id,
                            latency_us: Some(latency_us),
                            payload_bytes: Some(payload_bytes),
                            phase: Some("measured"),
                            ts_micros: None,
                        };

                        if let Ok(json) = serde_json::to_string(&event) {
                            if metrics_tx.send(json).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Receiver {} error receiving: {:#}", receiver_id, e);
                        anyhow::bail!("Receiver {} disconnected unexpectedly", receiver_id);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }

    let close_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("time went backwards")?
        .as_micros() as u64;

    let event = MetricEvent {
        event: Some("connection_closed"),
        client_message_id: None,
        sender_id: None,
        receiver_id,
        latency_us: None,
        payload_bytes: None,
        phase: None,
        ts_micros: Some(close_ts),
    };
    if let Ok(json) = serde_json::to_string(&event) {
        let _ = metrics_tx.send(json);
    }

    Ok(())
}

use rand::Rng;

#[allow(clippy::too_many_arguments)]
pub async fn sender_task(
    adapter: Arc<dyn TransportAdapter>,
    conversation_id: String,
    barrier: Arc<Barrier>,
    messages: u32,
    rate: f64,
    payload_bytes: usize,
    payload_mix: Option<String>,
    warmup_secs: u64,
) -> anyhow::Result<()> {
    let mut connection = match adapter.connect(&conversation_id).await {
        Ok(conn) => conn,
        Err(e) => {
            error!("Sender failed to connect: {:#}", e);
            anyhow::bail!("Sender failed to connect");
        }
    };

    // Wait for all receivers
    barrier.wait().await;

    info!("All connected. Warming up for {} seconds...", warmup_secs);
    tokio::time::sleep(Duration::from_secs(warmup_secs)).await;
    info!("Starting measurement phase.");

    let interval_duration = Duration::from_secs_f64(1.0 / rate);
    let mut interval = tokio::time::interval(interval_duration);
    // Don't burst if we are behind immediately
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let parsed_mix: Option<Vec<(usize, f64)>> = payload_mix.map(|m| {
        let mut mix = Vec::new();
        for part in m.split(',') {
            let mut iter = part.split(':');
            let size = iter.next().unwrap().parse::<usize>().unwrap();
            let weight = iter.next().unwrap().parse::<f64>().unwrap();
            mix.push((size, weight));
        }
        mix
    });

    for client_message_id in 0..messages {
        interval.tick().await;

        let mut chosen_size = payload_bytes;
        if let Some(ref mix) = parsed_mix {
            let r: f64 = rand::thread_rng().gen_range(0.0..100.0);
            let mut accum = 0.0;
            for &(size, weight) in mix {
                accum += weight;
                if r <= accum {
                    chosen_size = size;
                    break;
                }
            }
        }

        let mut payload_buf = BytesMut::with_capacity(chosen_size);
        payload_buf.put_u64(client_message_id as u64);

        let send_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("time went backwards")?
            .as_micros() as u64;
        payload_buf.put_u64(send_ts);

        // Fill the rest with random bytes
        let mut random_bytes = scenario::generate_payload(chosen_size);
        // Overwrite the first 16 bytes of random payload with header
        random_bytes[0..16].copy_from_slice(&payload_buf[..16]);

        if let Err(e) = connection.send(bytes::Bytes::from(random_bytes)).await {
            error!(
                "Sender failed to send message {}: {:#}",
                client_message_id, e
            );
            anyhow::bail!("Sender disconnected unexpectedly");
        }
    }

    info!("Sender completed sending all {} messages.", messages);
    Ok(())
}
