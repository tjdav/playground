use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use hmac::{Hmac, KeyInit, Mac};
use md5::{Digest as Md5Digest, Md5};
use sha2::Sha256;

use crate::adapter::{Connection, TransportAdapter};

#[derive(Clone)]
pub struct SockudoConfig {
    pub http_base: String,
    pub ws_base: String,
    pub app_id: String,
    pub app_key: String,
    pub app_secret: String,
    pub channel: String,
}

pub struct SockudoAdapter {
    config: SockudoConfig,
}

impl SockudoAdapter {
    pub fn new(config: SockudoConfig) -> Self {
        Self { config }
    }
}

pub struct SockudoConnection {
    config: SockudoConfig,
    client: Client,
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

#[async_trait]
impl TransportAdapter for SockudoAdapter {
    async fn connect(&self, _conversation_id: &str) -> anyhow::Result<Box<dyn Connection>> {
        let url = format!(
            "{}/app/{}",
            self.config.ws_base.trim_end_matches('/'),
            self.config.app_key
        );

        let (mut stream, _) = connect_async(&url)
            .await
            .context("failed to connect to Sockudo WebSocket")?;

        // 1. Wait for pusher:connection_established
        let msg = stream
            .next()
            .await
            .context("WebSocket stream ended before connection_established")??;
        let text = msg
            .to_text()
            .context("Expected text message for connection_established")?;
        let json: Value =
            serde_json::from_str(text).context("failed to parse connection_established frame")?;

        if json["event"].as_str() != Some("pusher:connection_established") {
            anyhow::bail!("Expected pusher:connection_established, got {}", text);
        }

        // The data is a JSON-encoded string, which we could parse if we needed socket_id,
        // but for public channels we don't strictly need it.

        // 2. Send pusher:subscribe
        let subscribe_msg = serde_json::json!({
            "event": "pusher:subscribe",
            "data": {
                "channel": self.config.channel
            }
        });
        stream
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await
            .context("failed to send subscribe msg")?;

        // 3. Wait for pusher_internal:subscription_succeeded
        loop {
            let msg = stream
                .next()
                .await
                .context("WebSocket stream ended before subscription_succeeded")??;
            if let Message::Text(text) = msg {
                let json: Value = serde_json::from_str(&text)
                    .context("failed to parse frame while waiting for subscription_succeeded")?;
                let event = json["event"].as_str().unwrap_or("");
                if event == "pusher_internal:subscription_succeeded" {
                    break;
                } else if event == "pusher:ping" {
                    let pong = serde_json::json!({"event": "pusher:pong"});
                    stream.send(Message::Text(pong.to_string().into())).await?;
                }
            }
        }

        let client = Client::builder()
            .build()
            .context("failed to build reqwest client")?;

        Ok(Box::new(SockudoConnection {
            config: self.config.clone(),
            client,
            stream,
        }))
    }
}

#[async_trait]
impl Connection for SockudoConnection {
    async fn send(&mut self, payload: Bytes) -> anyhow::Result<u64> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let payload_b64 = BASE64.encode(&payload);

        let body = serde_json::json!({
            "name": "bench-event",
            "channels": [self.config.channel],
            "data": payload_b64
        });

        let body_str = serde_json::to_string(&body)?;

        let mut md5 = Md5::new();
        md5.update(body_str.as_bytes());
        let body_md5 = hex::encode(md5.finalize());

        let ts_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("time went backwards")?
            .as_secs();

        let path = format!("/apps/{}/events", self.config.app_id);
        let query_string = format!(
            "auth_key={}&auth_timestamp={}&auth_version=1.0&body_md5={}",
            self.config.app_key, ts_sec, body_md5
        );
        let string_to_sign = format!("POST\n{}\n{}", path, query_string);

        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.app_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let auth_signature = hex::encode(mac.finalize().into_bytes());

        let url = format!(
            "{}{}?{}&auth_signature={}",
            self.config.http_base.trim_end_matches('/'),
            path,
            query_string,
            auth_signature
        );

        let ts_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("time went backwards")?
            .as_micros() as u64;

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .context("HTTP POST to Sockudo failed")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Sockudo publish failed: {} {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }

        Ok(ts_micros)
    }

    async fn recv(&mut self) -> anyhow::Result<(Bytes, u64)> {
        loop {
            let msg = self
                .stream
                .next()
                .await
                .context("WebSocket stream ended")??;

            let ts_micros = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("time went backwards")?
                .as_micros() as u64;

            match msg {
                Message::Text(text) => {
                    let json: Value =
                        serde_json::from_str(&text).context("failed to parse received msg")?;
                    let event = json["event"].as_str().unwrap_or("");

                    if event == "pusher:ping" {
                        let pong = serde_json::json!({"event": "pusher:pong"});
                        self.stream
                            .send(Message::Text(pong.to_string().into()))
                            .await?;
                        continue;
                    }

                    if event == "pusher:error" {
                        anyhow::bail!("Sockudo pusher error: {}", text);
                    }

                    if event == "bench-event" {
                        let data_str = json["data"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("missing data field"))?;

                        // Depending on pusher implementation, data might be nested JSON string if the sender sent JSON.
                        // But since we send a JSON object with `data: payload_b64`, the `data` field here IS `payload_b64`.

                        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                        let decoded = BASE64.decode(data_str).context("invalid base64 payload")?;

                        return Ok((Bytes::from(decoded), ts_micros));
                    }
                }
                Message::Close(_) => {
                    anyhow::bail!("WebSocket connection closed");
                }
                _ => {}
            }
        }
    }
}
