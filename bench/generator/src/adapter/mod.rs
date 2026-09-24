use async_trait::async_trait;
use bytes::Bytes;

pub mod ws;

#[async_trait]
pub trait TransportAdapter: Send + Sync {
    /// Connect and return a handle that can send and receive.
    async fn connect(&self, conversation_id: &str) -> anyhow::Result<Box<dyn Connection>>;
}

#[async_trait]
pub trait Connection: Send {
    /// Send a payload. Returns the client-side send timestamp in microseconds.
    async fn send(&mut self, payload: Bytes) -> anyhow::Result<u64>;

    /// Receive the next payload. Returns (payload, receive timestamp in micros).
    async fn recv(&mut self) -> anyhow::Result<(Bytes, u64)>;
}
