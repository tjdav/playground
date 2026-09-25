use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::Value;

use crate::adapter::{Connection, TransportAdapter};

pub struct SseAdapter {
    pub base_url: String,
}

impl SseAdapter {
    pub fn new(base_url: String, _conversation: String) -> Self {
        Self { base_url }
    }
}

pub struct SseConnection {
    client: Client,
    base_url: String,
    conversation: String,
    stream: std::pin::Pin<
        Box<
            dyn futures_util::Stream<
                    Item = Result<
                        eventsource_stream::Event,
                        eventsource_stream::EventStreamError<reqwest::Error>,
                    >,
                > + Send,
        >,
    >,
}

#[async_trait]
impl TransportAdapter for SseAdapter {
    async fn connect(&self, conversation_id: &str) -> anyhow::Result<Box<dyn Connection>> {
        let client = Client::builder()
            .build()
            .context("failed to build reqwest client")?;

        let url = format!("{}/api/realtime", self.base_url.trim_end_matches('/'));

        let response = client
            .get(&url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .context("failed to send SSE request")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!(
                "SSE connection failed with status {}: {:?}",
                status,
                response.text().await
            );
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/event-stream") {
            anyhow::bail!(
                "Expected content-type text/event-stream, got {}",
                content_type
            );
        }

        let mut stream = response.bytes_stream().eventsource();

        let client_id = loop {
            match stream.next().await {
                Some(Ok(event)) if event.event == "PB_CONNECT" => {
                    let data: Value = serde_json::from_str(&event.data)?;
                    let id = data["clientId"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("missing clientId in PB_CONNECT"))?
                        .to_string();
                    break id;
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => anyhow::bail!("SSE error before PB_CONNECT: {}", e),
                None => anyhow::bail!("SSE stream closed before PB_CONNECT"),
            }
        };

        let subscription = format!(
            "bench_messages/*?options={{\"query\":{{\"filter\":\"conversation='{}'\"}}}}",
            conversation_id
        );

        let subscribe_url = format!("{}/api/realtime", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "clientId": client_id,
            "subscriptions": [subscription]
        });

        let resp = client.post(&subscribe_url).json(&body).send().await?;

        anyhow::ensure!(
            resp.status() == reqwest::StatusCode::NO_CONTENT,
            "subscription POST failed: {} — {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );

        Ok(Box::new(SseConnection {
            client,
            base_url: self.base_url.clone(),
            conversation: conversation_id.to_string(),
            stream: Box::pin(stream),
        }))
    }
}

#[async_trait]
impl Connection for SseConnection {
    async fn send(&mut self, payload: Bytes) -> anyhow::Result<u64> {
        if payload.len() < 16 {
            anyhow::bail!("payload too small to extract client_message_id");
        }

        let client_message_id = u64::from_be_bytes(
            payload[0..8]
                .try_into()
                .context("failed to read client_message_id")?,
        );

        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let payload_b64 = BASE64.encode(&payload);

        let body = serde_json::json!({
            "conversation": self.conversation,
            "client_message_id": if client_message_id == 0 { 0.000000000000001 } else { client_message_id as f64 },
            "sender_id": 0.000000000000001,
            "payload_b64": payload_b64
        });

        let url = format!(
            "{}/api/collections/bench_messages/records",
            self.base_url.trim_end_matches('/')
        );

        let ts_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("time went backwards")?
            .as_micros() as u64;

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("failed to POST payload to pocketbase")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!(
                "POST failed with status {}: {:?}",
                status,
                response.text().await
            );
        }

        Ok(ts_micros)
    }

    async fn recv(&mut self) -> anyhow::Result<(Bytes, u64)> {
        while let Some(event_result) = self.stream.next().await {
            let event = event_result.context("failed to read SSE event")?;

            if event.event == "PB_CONNECT" {
                continue;
            }

            if event.data.is_empty() {
                continue;
            }

            let json: Value =
                serde_json::from_str(&event.data).context("failed to parse event data as JSON")?;

            let action = json.get("action").and_then(|a| a.as_str());
            if action != Some("create") {
                continue;
            }

            let payload_b64 = json
                .get("record")
                .and_then(|r| r.get("payload_b64"))
                .and_then(|p| p.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing or invalid record.payload_b64 in event"))?;

            use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
            let decoded = BASE64
                .decode(payload_b64)
                .context("invalid base64 in payload_b64")?;

            let ts_micros = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("time went backwards")?
                .as_micros() as u64;

            return Ok((Bytes::from(decoded), ts_micros));
        }

        anyhow::bail!("SSE stream ended")
    }
}
