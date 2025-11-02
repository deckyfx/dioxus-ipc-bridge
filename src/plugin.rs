//! Plugin system for extending bridge functionality
//!
//! Plugins can inject custom JavaScript, intercept requests/responses,
//! and add middleware to the IPC bridge.

use crate::request::IpcRequest;
use crate::response::{IpcError, IpcResponse};

/// Trait for implementing bridge plugins
///
/// Plugins can extend the bridge with custom functionality such as:
/// - Logging and monitoring
/// - Authentication and authorization
/// - Request/response transformation
/// - Custom JavaScript injection
///
/// # Example
/// ```rust,ignore
/// use dioxus_ipc_bridge::plugin::BridgePlugin;
///
/// struct LoggingPlugin;
///
/// impl BridgePlugin for LoggingPlugin {
///     fn name(&self) -> &str {
///         "logging"
///     }
///
///     fn inject_js(&self) -> Option<String> {
///         Some("console.log('[Plugin] Logging enabled');".to_string())
///     }
///
///     fn on_request(&self, req: &mut IpcRequest) -> Result<(), IpcError> {
///         println!("Request: {} {}", req.method, req.url);
///         Ok(())
///     }
/// }
/// ```
pub trait BridgePlugin: Send + Sync {
    /// Plugin identifier name
    fn name(&self) -> &str;

    /// Optional JavaScript code to inject during bridge initialization
    ///
    /// This code will be executed after the core bridge is set up
    fn inject_js(&self) -> Option<String> {
        None
    }

    /// Called before a request is processed
    ///
    /// Plugins can modify the request or return an error to reject it
    fn on_request(&self, _req: &mut IpcRequest) -> Result<(), IpcError> {
        Ok(())
    }

    /// Called after a response is generated
    ///
    /// Plugins can modify the response before it's sent to JavaScript
    fn on_response(&self, _res: &mut IpcResponse) -> Result<(), IpcError> {
        Ok(())
    }
}

/// Trait for implementing middleware
///
/// Middleware provides a more flexible way to intercept and transform
/// requests and responses with a chain-of-responsibility pattern.
///
/// # Example
/// ```rust,ignore
/// use dioxus_ipc_bridge::prelude::*;
///
/// struct AuthMiddleware {
///     token: String,
/// }
///
/// impl Middleware for AuthMiddleware {
///     fn handle(
///         &self,
///         req: &EnrichedRequest,
///         next: &dyn Fn(&EnrichedRequest) -> Result<IpcResponse, IpcError>,
///     ) -> Result<IpcResponse, IpcError> {
///         // Check auth token
///         if req.headers.get("Authorization") != Some(&self.token) {
///             return Err(IpcError::Unauthorized);
///         }
///         // Continue chain
///         next(req)
///     }
/// }
/// ```
pub trait Middleware: Send + Sync {
    /// Process a request through the middleware chain
    ///
    /// # Arguments
    /// * `req` - The enriched request with parsed parameters
    /// * `next` - Function to call the next middleware or route handler
    fn handle(
        &self,
        req: &crate::request::EnrichedRequest,
        next: &dyn Fn(&crate::request::EnrichedRequest) -> Result<IpcResponse, IpcError>,
    ) -> Result<IpcResponse, IpcError>;
}
