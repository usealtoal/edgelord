//! Polymarket WebSocket handler.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

use super::messages::{SubscribeMessage, WsMessage};
use crate::error::Result;

pub struct WebSocketHandler {
    url: String,
}

impl WebSocketHandler {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub async fn connect(&self) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        info!(url = %self.url, "Connecting to WebSocket");

        let (ws_stream, response) = connect_async(&self.url).await?;

        info!(
            status = %response.status(),
            "WebSocket connected"
        );

        Ok(ws_stream)
    }

    pub async fn subscribe(
        ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        asset_ids: Vec<String>,
    ) -> Result<()> {
        let msg = SubscribeMessage::new(asset_ids.clone());
        let json = serde_json::to_string(&msg)?;

        info!(assets = ?asset_ids, "Subscribing to assets");
        ws.send(Message::Text(json)).await?;

        Ok(())
    }

    pub async fn run<F>(&self, asset_ids: Vec<String>, mut on_message: F) -> Result<()>
    where
        F: FnMut(WsMessage),
    {
        let mut ws = self.connect().await?;

        Self::subscribe(&mut ws, asset_ids).await?;

        info!("Listening for messages...");

        while let Some(msg_result) = ws.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    debug!(raw = %text, "Received message");

                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(ws_msg) => on_message(ws_msg),
                        Err(e) => {
                            warn!(
                                error = %e,
                                raw = %text,
                                "Failed to parse message"
                            );
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    debug!("Received ping");
                    ws.send(Message::Pong(data)).await?;
                }
                Ok(Message::Close(frame)) => {
                    info!(frame = ?frame, "WebSocket closed by server");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!(error = %e, "WebSocket error");
                    break;
                }
            }
        }

        Ok(())
    }
}
