//! # Dioxus IPC Bridge
//!
//! A powerful HTTP-like IPC bridge for Dioxus applications that enables bidirectional
//! communication between JavaScript/React and Rust across desktop, web, and mobile platforms.
//!
//! ## Features
//!
//! - **HTTP-like API**: Request-response pattern with methods, URLs, headers, and bodies
//! - **Bidirectional**: JS → Rust requests and Rust → JS event streaming
//! - **Platform-agnostic**: Works on desktop (webview), web (WASM), and mobile
//! - **Type-safe**: Full Rust type safety with serde serialization
//! - **Streaming support**: Long-running tasks with progress updates (optional feature)
//! - **Plugin system**: Extend functionality with custom plugins and middleware
//! - **Macro support**: Ergonomic route handlers with `#[ipc_route]` macro
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use dioxus_ipc_bridge::prelude::*;
//! use dioxus::prelude::*;
//!
//! // Define a route handler
//! #[ipc_route(GET, "/hello/:name")]
//! fn hello_handler(req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
//!     let name = req.path_params.get("name").unwrap();
//!     Ok(IpcResponse::ok(serde_json::json!({
//!         "message": format!("Hello, {}!", name)
//!     })))
//! }
//!
//! fn main() {
//!     // Create and configure the IPC bridge
//!     let bridge = IpcBridge::builder()
//!         .timeout(std::time::Duration::from_secs(30))
//!         .build();
//!
//!     // Configure routes
//!     let router = IpcRouter::builder()
//!         .route("GET", "/hello/:name", Box::new(hello_handler))
//!         .build();
//!
//!     // Use in your Dioxus app
//!     dioxus::launch(App);
//! }
//! ```

// Module declarations - will be implemented in extraction phase
pub mod bridge;
pub mod parser;
pub mod platform;
pub mod plugin;
pub mod request;
pub mod response;
pub mod router;

#[cfg(feature = "streaming")]
pub mod streaming;

// Re-export commonly used types
pub use bridge::IpcBridge;
pub use plugin::{BridgePlugin, Middleware};
pub use request::{EnrichedRequest, IpcRequest, RequestBody};
pub use response::{IpcError, IpcResponse};
pub use router::{IpcRouter, RouteHandler};

#[cfg(feature = "streaming")]
pub use streaming::StreamingTask;

// Prelude module for convenient imports
pub mod prelude {
    pub use crate::bridge::IpcBridge;
    pub use crate::plugin::{BridgePlugin, Middleware};
    pub use crate::request::{EnrichedRequest, IpcRequest, RequestBody};
    pub use crate::response::{IpcError, IpcResponse};
    pub use crate::router::{IpcRouter, RouteHandler};

    #[cfg(feature = "streaming")]
    pub use crate::streaming::StreamingTask;

    // Re-export macro
    pub use deckyfx_dioxus_ipc_bridge_macros::ipc_route;

    // Common external deps
    pub use serde_json::{json, Value};
}
