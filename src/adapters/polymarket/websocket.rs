//! Polymarket WebSocket handler.
//!
//! This module provides a WebSocket client for connecting to the Polymarket
//! real-time data feed. It handles the full connection lifecycle including
//! establishing connections, subscribing to asset updates, and processing
//! incoming messages.
//!
//! # Connection Lifecycle
//!
//! 1. **Initialization**: Create a `PolymarketWebSocketHandler` with the target URL
//! 2. **Connection**: Establish a WebSocket connection via `connect()`
//! 3. **Subscription**: Subscribe to specific asset IDs via `subscribe()`
//! 4. **Message Loop**: Process incoming messages until close or error
//! 5. **Termination**: Clean up when connection closes or error occurs
//!
//! # Usage
//!
//! ```ignore
//! let handler = PolymarketWebSocketHandler::new(url);
//! handler.run(asset_ids, |msg| {
//!     // Process each incoming message
//! }).await?;
//! ```

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, trace, warn};

use super::message::{PolymarketSubscribeMessage, PolymarketTaggedMessage, PolymarketWsMessage};
use crate::domain::TokenId;
use crate::error::Result;
use crate::runtime::exchange::{MarketDataStream, MarketEvent};

/// WebSocket handler for Polymarket real-time data feed.
///
/// Manages the WebSocket connection lifecycle and message processing.
/// The handler is stateless after construction - connection state is
/// managed within the `run()` method.
pub struct PolymarketWebSocketHandler {
    /// The WebSocket URL to connect to (e.g., <wss://ws-subscriptions-clob.polymarket.com/ws/market>)
    url: String,
}

impl PolymarketWebSocketHandler {
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
        let msg = PolymarketSubscribeMessage::new(asset_ids.clone());
        let json = serde_json::to_string(&msg)?;

        // Log a truncated view of assets to avoid spam
        let total = asset_ids.len();
        if total <= 5 {
            info!(assets = ?asset_ids, "Subscribing to assets");
        } else {
            let preview: Vec<_> = asset_ids.iter().take(5).collect();
            info!(assets = ?preview, more = total - 5, "Subscribing to assets");
        }
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
    /// - **Text messages**: Parsed as `PolymarketWsMessage` and passed to the callback.
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
        F: FnMut(PolymarketWsMessage),
    {
        // Phase 1: Establish connection
        let mut ws = self.connect().await?;

        // Phase 2: Subscribe to requested assets
        Self::subscribe(&mut ws, asset_ids).await?;

        // Phase 3: Enter message loop
        debug!("Entering WebSocket message loop");

        // The message loop processes incoming WebSocket frames until:
        // - The server sends a Close frame
        // - A WebSocket error occurs
        // - The stream ends (connection lost)
        while let Some(msg_result) = ws.next().await {
            match msg_result {
                // Text message: Parse and dispatch to callback
                Ok(Message::Text(text)) => {
                    trace!(bytes = text.len(), "Received WebSocket text frame");

                    match serde_json::from_str::<PolymarketWsMessage>(&text) {
                        Ok(ws_msg) => on_message(ws_msg),
                        Err(e) => {
                            // Log parse failures but continue processing
                            // This allows the stream to recover from malformed messages
                            warn!(
                                error = %e,
                                bytes = text.len(),
                                "Failed to parse message"
                            );
                        }
                    }
                }
                // Ping/Pong: Respond to keep connection alive
                // The WebSocket protocol requires pong responses to contain
                // the same application data as the ping frame
                Ok(Message::Ping(data)) => {
                    trace!("Received WebSocket ping");
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

/// Polymarket market data stream implementing the `MarketDataStream` trait.
///
/// Provides an async iterator-style interface for receiving market events.
pub struct PolymarketDataStream {
    url: String,
    ws: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    /// Buffer for pending book messages when multiple arrive in one frame.
    pending_books: Vec<super::message::PolymarketBookMessage>,
}

impl PolymarketDataStream {
    /// Create a new data stream for the given WebSocket URL.
    #[must_use]
    pub fn new(url: String) -> Self {
        Self {
            url,
            ws: None,
            pending_books: Vec::new(),
        }
    }
}

#[async_trait]
impl MarketDataStream for PolymarketDataStream {
    async fn connect(&mut self) -> Result<()> {
        info!(url = %self.url, "Connecting to WebSocket");
        let (ws_stream, response) = connect_async(&self.url).await?;
        info!(status = %response.status(), "WebSocket connected");
        self.ws = Some(ws_stream);
        Ok(())
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<()> {
        let ws = self
            .ws
            .as_mut()
            .ok_or_else(|| crate::error::Error::Connection("Not connected".into()))?;

        let asset_ids: Vec<String> = token_ids.iter().map(|t| t.as_str().to_string()).collect();
        let msg = PolymarketSubscribeMessage::new(asset_ids.clone());
        let json = serde_json::to_string(&msg)?;

        // Log a truncated view of assets to avoid spam
        let total = asset_ids.len();
        if total <= 5 {
            info!(assets = ?asset_ids, "Subscribing to assets");
        } else {
            let preview: Vec<_> = asset_ids.iter().take(5).collect();
            info!(assets = ?preview, more = total - 5, "Subscribing to assets");
        }
        ws.send(Message::Text(json)).await?;
        Ok(())
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        // First, check if we have pending books from a previous message
        if let Some(book) = self.pending_books.pop() {
            let order_book = book.to_orderbook();
            let token_id = TokenId::from(book.asset_id);
            return Some(MarketEvent::OrderBookSnapshot {
                token_id,
                book: order_book,
            });
        }

        let ws = self.ws.as_mut()?;

        loop {
            match ws.next().await? {
                Ok(Message::Text(text)) => {
                    trace!(bytes = text.len(), "Received WebSocket text frame");
                    match serde_json::from_str::<PolymarketWsMessage>(&text) {
                        Ok(PolymarketWsMessage::Books(mut books)) => {
                            // Store all but the first book for later
                            if let Some(book) = books.pop() {
                                // Save remaining books (in reverse order so pop gives correct order)
                                self.pending_books = books;
                                let order_book = book.to_orderbook();
                                let token_id = TokenId::from(book.asset_id);
                                return Some(MarketEvent::OrderBookSnapshot {
                                    token_id,
                                    book: order_book,
                                });
                            }
                            // Empty array, continue
                            continue;
                        }
                        Ok(PolymarketWsMessage::Tagged(tagged)) => {
                            // Handle legacy tagged messages
                            match tagged {
                                PolymarketTaggedMessage::Book(book) => {
                                    let order_book = book.to_orderbook();
                                    let token_id = TokenId::from(book.asset_id);
                                    return Some(MarketEvent::OrderBookSnapshot {
                                        token_id,
                                        book: order_book,
                                    });
                                }
                                PolymarketTaggedMessage::PriceChange(_) => continue,
                                PolymarketTaggedMessage::TickSizeChange(_) => continue,
                                PolymarketTaggedMessage::Unknown => continue,
                            }
                        }
                        Ok(PolymarketWsMessage::Unknown(_)) => continue,
                        Err(e) => {
                            warn!(error = %e, bytes = text.len(), "Failed to parse message");
                            continue;
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    trace!("Received WebSocket ping");
                    if ws.send(Message::Pong(data)).await.is_err() {
                        return Some(MarketEvent::Disconnected {
                            reason: "Failed to send pong".into(),
                        });
                    }
                }
                Ok(Message::Close(frame)) => {
                    info!(frame = ?frame, "WebSocket closed by server");
                    return Some(MarketEvent::Disconnected {
                        reason: frame.map(|f| f.reason.to_string()).unwrap_or_default(),
                    });
                }
                Ok(_) => continue,
                Err(e) => {
                    error!(error = %e, "WebSocket error");
                    return Some(MarketEvent::Disconnected {
                        reason: e.to_string(),
                    });
                }
            }
        }
    }

    fn exchange_name(&self) -> &'static str {
        "Polymarket"
    }
}
