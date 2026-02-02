//! Polymarket WebSocket handler.
//!
//! This module provides a WebSocket client for connecting to the Polymarket
//! real-time data feed. It handles the full connection lifecycle including
//! establishing connections, subscribing to asset updates, and processing
//! incoming messages.
//!
//! # Connection Lifecycle
//!
//! 1. **Initialization**: Create a `WebSocketHandler` with the target URL
//! 2. **Connection**: Establish a WebSocket connection via `connect()`
//! 3. **Subscription**: Subscribe to specific asset IDs via `subscribe()`
//! 4. **Message Loop**: Process incoming messages until close or error
//! 5. **Termination**: Clean up when connection closes or error occurs
//!
//! # Usage
//!
//! ```ignore
//! let handler = WebSocketHandler::new(url);
//! handler.run(asset_ids, |msg| {
//!     // Process each incoming message
//! }).await?;
//! ```

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

use super::messages::{SubscribeMessage, WsMessage};
use crate::error::Result;

/// WebSocket handler for Polymarket real-time data feed.
///
/// Manages the WebSocket connection lifecycle and message processing.
/// The handler is stateless after construction - connection state is
/// managed within the `run()` method.
pub struct WebSocketHandler {
    /// The WebSocket URL to connect to (e.g., <wss://ws-subscriptions-clob.polymarket.com/ws/market>)
    url: String,
}

impl WebSocketHandler {
    /// Creates a new WebSocket handler for the given URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL to connect to
    #[must_use] 
    pub const fn new(url: String) -> Self {
        Self { url }
    }

    /// Establishes a WebSocket connection to the configured URL.
    ///
    /// This is the first phase of the connection lifecycle. The connection
    /// is established using TLS if the URL scheme is `wss://`.
    ///
    /// # Returns
    ///
    /// A WebSocket stream that can be used for bidirectional communication.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection fails (network issues, invalid URL,
    /// TLS handshake failure, etc.).
    pub async fn connect(&self) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        info!(url = %self.url, "Connecting to WebSocket");

        let (ws_stream, response) = connect_async(&self.url).await?;

        info!(
            status = %response.status(),
            "WebSocket connected"
        );

        Ok(ws_stream)
    }

    /// Subscribes to real-time updates for the specified assets.
    ///
    /// Sends a subscription message to the Polymarket WebSocket server
    /// to begin receiving updates for the given asset IDs.
    ///
    /// # Arguments
    ///
    /// * `ws` - An active WebSocket connection
    /// * `asset_ids` - List of Polymarket asset IDs to subscribe to
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the message cannot be sent.
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

    /// Runs the WebSocket message loop.
    ///
    /// This is the main entry point that orchestrates the full connection lifecycle:
    ///
    /// 1. **Connect**: Establishes the WebSocket connection
    /// 2. **Subscribe**: Subscribes to the specified asset IDs
    /// 3. **Message Loop**: Processes incoming messages until termination
    ///
    /// # Message Loop
    ///
    /// The message loop continuously reads from the WebSocket and handles
    /// different message types:
    ///
    /// - **Text messages**: Parsed as `WsMessage` and passed to the callback.
    ///   Parse failures are logged but don't terminate the loop.
    /// - **Ping messages**: Automatically responded to with Pong (keepalive).
    /// - **Close messages**: Gracefully terminates the loop.
    /// - **Other messages**: Silently ignored (binary, pong, etc.).
    ///
    /// # Ping/Pong Handling
    ///
    /// WebSocket ping frames are automatically answered with pong frames
    /// containing the same payload. This is required by the WebSocket protocol
    /// to maintain the connection and allow the server to detect dead clients.
    ///
    /// # Error Handling
    ///
    /// - **Parse errors**: Logged as warnings, loop continues
    /// - **WebSocket errors**: Logged as errors, loop terminates
    /// - **Server close**: Logged as info, loop terminates gracefully
    ///
    /// # Reconnection Logic
    ///
    /// **Note**: This implementation does NOT automatically reconnect.
    /// When the connection closes (either by server close frame or error),
    /// the method returns. Callers are responsible for implementing retry
    /// logic if reconnection is desired.
    ///
    /// # Arguments
    ///
    /// * `asset_ids` - List of Polymarket asset IDs to subscribe to
    /// * `on_message` - Callback invoked for each successfully parsed message
    ///
    /// # Errors
    ///
    /// Returns an error if connection, subscription, or pong sending fails.
    pub async fn run<F>(&self, asset_ids: Vec<String>, mut on_message: F) -> Result<()>
    where
        F: FnMut(WsMessage),
    {
        // Phase 1: Establish connection
        let mut ws = self.connect().await?;

        // Phase 2: Subscribe to requested assets
        Self::subscribe(&mut ws, asset_ids).await?;

        // Phase 3: Enter message loop
        info!("Listening for messages...");

        // The message loop processes incoming WebSocket frames until:
        // - The server sends a Close frame
        // - A WebSocket error occurs
        // - The stream ends (connection lost)
        while let Some(msg_result) = ws.next().await {
            match msg_result {
                // Text message: Parse and dispatch to callback
                Ok(Message::Text(text)) => {
                    debug!(raw = %text, "Received message");

                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(ws_msg) => on_message(ws_msg),
                        Err(e) => {
                            // Log parse failures but continue processing
                            // This allows the stream to recover from malformed messages
                            warn!(
                                error = %e,
                                raw = %text,
                                "Failed to parse message"
                            );
                        }
                    }
                }
                // Ping/Pong: Respond to keep connection alive
                // The WebSocket protocol requires pong responses to contain
                // the same application data as the ping frame
                Ok(Message::Ping(data)) => {
                    debug!("Received ping");
                    ws.send(Message::Pong(data)).await?;
                }
                // Close frame: Server is closing the connection
                Ok(Message::Close(frame)) => {
                    info!(frame = ?frame, "WebSocket closed by server");
                    break;
                }
                // Other message types (Binary, Pong, Frame) are ignored
                Ok(_) => {}
                // WebSocket error: Log and terminate
                // Common causes: network issues, protocol violations
                Err(e) => {
                    error!(error = %e, "WebSocket error");
                    break;
                }
            }
        }

        Ok(())
    }
}
