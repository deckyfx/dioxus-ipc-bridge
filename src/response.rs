//! IPC Response Types
//!
//! Defines response and error types for IPC communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

/// HTTP-like IPC response from Rust to JavaScript
///
/// # Example
/// ```rust
/// use dioxus_ipc_bridge::response::IpcResponse;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let response = IpcResponse {
///     status: 200,
///     headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
///     body: json!({ "success": true, "message": "Form submitted!" }),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    /// HTTP-like status code
    /// - 200: Success
    /// - 400: Bad Request (invalid input)
    /// - 404: Not Found (no route matches)
    /// - 500: Internal Server Error (handler error)
    pub status: u16,

    /// Response headers
    pub headers: HashMap<String, String>,

    /// Response body (always JSON)
    pub body: Value,
}

impl IpcResponse {
    /// Create a successful (200 OK) response
    ///
    /// # Arguments
    /// * `body` - JSON response body
    ///
    /// # Example
    /// ```rust
    /// use dioxus_ipc_bridge::response::IpcResponse;
    /// use serde_json::json;
    ///
    /// let response = IpcResponse::ok(json!({ "message": "Success!" }));
    /// ```
    pub fn ok(body: Value) -> Self {
        Self {
            status: 200,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body,
        }
    }

    /// Create a bad request (400) error response
    ///
    /// # Arguments
    /// * `message` - Error message
    ///
    /// # Example
    /// ```rust
    /// use dioxus_ipc_bridge::response::IpcResponse;
    ///
    /// let response = IpcResponse::bad_request("Missing 'name' field");
    /// ```
    pub fn bad_request(message: &str) -> Self {
        Self {
            status: 400,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body: serde_json::json!({
                "error": "Bad Request",
                "message": message
            }),
        }
    }

    /// Create a not found (404) error response
    ///
    /// # Arguments
    /// * `path` - The path that was not found
    ///
    /// # Example
    /// ```rust
    /// use dioxus_ipc_bridge::response::IpcResponse;
    ///
    /// let response = IpcResponse::not_found("/unknown/route");
    /// ```
    pub fn not_found(path: &str) -> Self {
        Self {
            status: 404,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body: serde_json::json!({
                "error": "Not Found",
                "message": format!("No route matches '{}'", path)
            }),
        }
    }

    /// Create an internal server error (500) response
    ///
    /// # Arguments
    /// * `error` - Error details
    ///
    /// # Example
    /// ```rust
    /// use dioxus_ipc_bridge::response::IpcResponse;
    ///
    /// let response = IpcResponse::internal_error("Database connection failed");
    /// ```
    pub fn internal_error(error: &str) -> Self {
        Self {
            status: 500,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body: serde_json::json!({
                "error": "Internal Server Error",
                "message": error
            }),
        }
    }

    /// Create a custom response with specific status code
    pub fn with_status(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: HashMap::from([(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )]),
            body,
        }
    }

    /// Add a header to the response
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
}

/// IPC error types
///
/// Represents various error conditions that can occur during IPC routing and handling.
#[derive(Debug, Clone)]
pub enum IpcError {
    /// Invalid request format
    BadRequest(String),

    /// Route not found
    NotFound(String),

    /// Handler execution error
    InternalError(String),

    /// URL parsing error
    ParseError(String),

    /// Unauthorized access
    Unauthorized,

    /// Forbidden access
    Forbidden(String),
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            IpcError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            IpcError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
            IpcError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            IpcError::Unauthorized => write!(f, "Unauthorized"),
            IpcError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
        }
    }
}

impl std::error::Error for IpcError {}

impl From<IpcError> for IpcResponse {
    fn from(error: IpcError) -> Self {
        match error {
            IpcError::BadRequest(msg) => IpcResponse::bad_request(&msg),
            IpcError::NotFound(msg) => IpcResponse::not_found(&msg),
            IpcError::InternalError(msg) => IpcResponse::internal_error(&msg),
            IpcError::ParseError(msg) => {
                IpcResponse::bad_request(&format!("Parse error: {}", msg))
            }
            IpcError::Unauthorized => IpcResponse::with_status(
                401,
                serde_json::json!({
                    "error": "Unauthorized",
                    "message": "Authentication required"
                }),
            ),
            IpcError::Forbidden(msg) => IpcResponse::with_status(
                403,
                serde_json::json!({
                    "error": "Forbidden",
                    "message": msg
                }),
            ),
        }
    }
}
