//! IPC Request Types
//!
//! Defines the core request types for HTTP-like IPC communication between JavaScript and Rust.
//! Supports JSON, URL-encoded, and multipart/form-data request bodies.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// HTTP-like IPC request from JavaScript to Rust
///
/// # Example
/// ```rust
/// use dioxus_ipc_bridge::request::*;
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let request = IpcRequest {
///     id: 12345,
///     method: "POST".to_string(),
///     url: "ipc://form/submit?redirect=true".to_string(),
///     headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
///     body: Some(RequestBody::Json(json!({"name": "John", "email": "john@example.com"}))),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    /// Unique request ID for promise resolution
    pub id: u64,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,

    /// Full IPC URL including scheme, path, and query string
    /// Example: "ipc://calculator/fibonacci?number=10"
    pub url: String,

    /// Request headers (Content-Type, Authorization, etc.)
    pub headers: HashMap<String, String>,

    /// Request body (optional for GET-like operations)
    pub body: Option<RequestBody>,
}

/// Request body types
///
/// Supports three common encoding formats for maximum compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RequestBody {
    /// JSON body (application/json)
    ///
    /// Most common format for modern APIs. Supports nested objects and arrays.
    /// # Example
    /// ```json
    /// { "name": "John", "age": 25, "address": { "city": "NYC" } }
    /// ```
    Json(Value),

    /// URL-encoded body (application/x-www-form-urlencoded)
    ///
    /// Traditional HTML form submission format.
    /// # Example
    /// ```
    /// name=John&age=25&city=NYC
    /// ```
    UrlEncoded(HashMap<String, String>),

    /// Multipart form data (multipart/form-data)
    ///
    /// Supports file uploads with content-disposition headers.
    /// # Example
    /// ```
    /// Content-Disposition: form-data; name="file"; filename="photo.jpg"
    /// Content-Type: image/jpeg
    /// [binary data]
    /// ```
    Multipart {
        /// Text fields (key-value pairs)
        fields: HashMap<String, String>,

        /// File uploads
        files: Vec<FileUpload>,
    },
}

/// File upload data
///
/// Represents a single file in a multipart/form-data request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    /// Form field name
    pub name: String,

    /// Original filename
    pub filename: String,

    /// MIME type (e.g., "image/jpeg", "application/pdf")
    pub content_type: String,

    /// File data (base64-encoded for JavaScript transport)
    pub data: String,
}

/// Enriched request with parsed URL components
///
/// This is what route handlers receive after the router parses the URL
/// and extracts path parameters and query strings.
///
/// # Example
/// ```rust
/// use dioxus_ipc_bridge::prelude::*;
/// use std::collections::HashMap;
///
/// fn handler(req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
///     let user_id = req.path_params.get("id").unwrap();
///     let page = req.query_params.get("page").unwrap_or(&"1".to_string());
///     Ok(IpcResponse::ok(serde_json::json!({
///         "user_id": user_id,
///         "page": page
///     })))
/// }
/// ```
#[derive(Debug, Clone)]
pub struct EnrichedRequest {
    /// Original request
    pub original: IpcRequest,

    /// Parsed path parameters (e.g., /user/:id → {"id": "123"})
    pub path_params: HashMap<String, String>,

    /// Parsed query parameters (e.g., ?page=2&sort=name → {"page": "2", "sort": "name"})
    pub query_params: HashMap<String, String>,

    /// Extracted URL path without query string
    /// Example: "ipc://calculator/fibonacci?number=10" → "/calculator/fibonacci"
    pub path: String,

    /// Request headers (convenience accessor)
    pub headers: HashMap<String, String>,

    /// Request body (convenience accessor)
    pub body: Option<RequestBody>,
}

impl EnrichedRequest {
    /// Create a new enriched request from a basic IPC request
    pub fn new(
        original: IpcRequest,
        path: String,
        path_params: HashMap<String, String>,
        query_params: HashMap<String, String>,
    ) -> Self {
        let headers = original.headers.clone();
        let body = original.body.clone();

        Self {
            original,
            path_params,
            query_params,
            path,
            headers,
            body,
        }
    }

    /// Get a path parameter by name
    pub fn path_param(&self, name: &str) -> Option<&String> {
        self.path_params.get(name)
    }

    /// Get a query parameter by name
    pub fn query_param(&self, name: &str) -> Option<&String> {
        self.query_params.get(name)
    }

    /// Get a header by name (case-insensitive)
    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    /// Get the request method
    pub fn method(&self) -> &str {
        &self.original.method
    }

    /// Get the request ID
    pub fn id(&self) -> u64 {
        self.original.id
    }
}
