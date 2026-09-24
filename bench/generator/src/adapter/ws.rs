use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::adapter::{Connection, TransportAdapter};

pub struct WsAdapter {
    pub base_url: String,
}

impl WsAdapter {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl TransportAdapter for WsAdapter {
    async fn connect(&self, conversation_id: &str) -> anyhow::Result<Box<dyn Connection>> {
        // Construct the WebSocket URL. Assuming base_url is something like ws://127.0.0.1:8080
        let url = format!(
            "{}/ws/{}",
            self.base_url.trim_end_matches('/'),
            conversation_id
        );

        let (ws_stream, _) = connect_async(&url)
            .await
            .context("failed to connect to WebSocket")?;

        Ok(Box::new(WsConnection { stream: ws_stream }))
    }
}

pub struct WsConnection {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

#[async_trait]
impl Connection for WsConnection {
    async fn send(&mut self, payload: Bytes) -> anyhow::Result<u64> {
        let ts_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("time went backwards")?
            .as_micros() as u64;

        self.stream
            .send(Message::Binary(payload))
            .await
            .context("failed to send WebSocket message")?;

        Ok(ts_micros)
    }

    async fn recv(&mut self) -> anyhow::Result<(Bytes, u64)> {
        while let Some(msg_result) = self.stream.next().await {
            let msg = msg_result.context("failed to receive WebSocket message")?;

            let ts_micros = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("time went backwards")?
                .as_micros() as u64;

            match msg {
                Message::Binary(data) => {
                    return Ok((data, ts_micros));
                }
                Message::Close(_) => {
                    anyhow::bail!("WebSocket connection closed");
                }
                _ => {
                    // Ignore other frame types like Ping/Pong/Text for now.
                    continue;
                }
            }
        }
        anyhow::bail!("WebSocket stream ended")
    }
}
