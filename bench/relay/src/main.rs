use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use bytes::Bytes;
use futures_util::{stream::StreamExt, SinkExt};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type AppState = Arc<RwLock<HashMap<String, broadcast::Sender<Bytes>>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "relay=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state: AppState = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws/{id}", get(ws_handler))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().expect("Invalid port");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");
    info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, id, state))
}

async fn handle_socket(socket: WebSocket, id: String, state: AppState) {
    info!(conversation_id = %id, "connected");

    let (tx, mut rx) = {
        let mut map = state.write().await;
        let tx = map.entry(id.clone()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(1024);
            tx
        });
        (tx.clone(), tx.subscribe())
    };

    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(Message::Binary(msg)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Slow client; just skip dropped messages and continue
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    let id_for_recv = id.clone();
    let tx_for_recv = tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Binary(bytes) = msg {
                info!(conversation_id = %id_for_recv, bytes = bytes.len(), "broadcast");
                let _ = tx_for_recv.send(bytes);
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let _ = send_task.await;
    let _ = recv_task.await;

    info!(conversation_id = %id, "disconnected");

    let mut map = state.write().await;
    if let Some(channel) = map.get(&id) {
        if channel.receiver_count() == 0 {
            map.remove(&id);
        }
    }
}
