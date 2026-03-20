//! HTTP-like IPC Router
//!
//! Provides URL-based routing system for IPC communication between JavaScript and Rust.
//! Supports path parameters, query strings, and multiple body formats.

use crate::parser::{parse_body, ParsedUrl};
use crate::request::{EnrichedRequest, IpcRequest, RequestBody};
use crate::response::{IpcError, IpcResponse};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for implementing HTTP-style route handlers
///
/// Implement this trait to create custom route handlers that can be
/// registered with the IpcRouter.
///
/// # Example
/// ```rust,no_run
/// use dioxus_ipc_bridge::prelude::*;
///
/// struct UserHandler;
///
/// impl RouteHandler for UserHandler {
///     fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
///         let user_id = req.path_param("id").ok_or(IpcError::BadRequest("Missing ID".into()))?;
///         Ok(IpcResponse::ok(serde_json::json!({
///             "user_id": user_id
///         })))
///     }
/// }
/// ```
pub trait RouteHandler: Send + Sync {
    /// Handles the incoming HTTP-like IPC request
    ///
    /// # Arguments
    /// * `req` - Enriched IPC request with parsed URL, path params, query params, and body
    ///
    /// # Returns
    /// * `Ok(IpcResponse)` - Success with HTTP-like response
    /// * `Err(IpcError)` - Error during processing
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError>;
}

/// Route definition
struct Route {
    /// HTTP method (informational)
    method: String,

    /// Path pattern with optional parameters (e.g., "/user/:id/posts/:postId")
    pattern: String,

    /// Handler for this route
    handler: Arc<dyn RouteHandler>,
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            pattern: self.pattern.clone(),
            handler: Arc::clone(&self.handler),
        }
    }
}

/// HTTP-like IPC Router
///
/// Routes incoming IPC requests to appropriate handlers based on URL patterns.
/// Supports static paths, path parameters, query strings, and multiple body formats.
#[derive(Clone)]
pub struct IpcRouter {
    /// Registered routes
    routes: Vec<Route>,
}

impl IpcRouter {
    /// Creates a new empty IPC router
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }

    /// Create a new router builder
    pub fn builder() -> IpcRouterBuilder {
        IpcRouterBuilder::new()
    }

    /// Registers a new route handler
    ///
    /// # Arguments
    /// * `method` - HTTP method (informational, e.g., "GET", "POST")
    /// * `pattern` - URL pattern with optional parameters (e.g., "/user/:id")
    /// * `handler` - Boxed handler implementing RouteHandler trait
    ///
    /// # Example
    /// ```rust,ignore
    /// router.register("POST", "/form/submit", Box::new(FormSubmitHandler));
    /// router.register("GET", "/user/:id", Box::new(GetUserHandler));
    /// ```
    pub fn register(&mut self, method: &str, pattern: &str, handler: Box<dyn RouteHandler>) {
        self.routes.push(Route {
            method: method.to_string(),
            pattern: pattern.to_string(),
            handler: Arc::from(handler),
        });
    }

    /// Dispatches an IPC request to the appropriate route handler
    ///
    /// # Arguments
    /// * `raw_request` - Raw IPC request from JavaScript
    ///
    /// # Returns
    /// * `IpcResponse` - HTTP-like response (includes 404 if no route matches)
    pub fn dispatch(&self, raw_request: &Value) -> IpcResponse {
        // Parse raw request
        let ipc_request = match self.parse_raw_request(raw_request) {
            Ok(req) => req,
            Err(err) => return err.into(),
        };

        // Parse URL
        let parsed_url = match ParsedUrl::parse(&ipc_request.url) {
            Ok(url) => url,
            Err(err) => return err.into(),
        };

        // Find matching route (checks both method and path pattern)
        for route in &self.routes {
            if !route.method.eq_ignore_ascii_case(&ipc_request.method) {
                continue;
            }
            if let Some(path_params) = parsed_url.match_pattern(&route.pattern) {
                // Route matched! Build enriched request
                let enriched_request = EnrichedRequest::new(
                    ipc_request,
                    parsed_url.path.clone(),
                    path_params,
                    parsed_url.query_params.clone(),
                );

                // Call handler
                return match route.handler.handle(&enriched_request) {
                    Ok(response) => response,
                    Err(err) => err.into(),
                };
            }
        }

        // No route matched
        IpcResponse::not_found(&format!("{} {}", ipc_request.method, parsed_url.path))
    }

    /// Parse raw JSON request from JavaScript into IpcRequest
    fn parse_raw_request(&self, raw: &Value) -> Result<IpcRequest, IpcError> {
        let obj = raw.as_object().ok_or_else(|| {
            IpcError::BadRequest("Request must be an object".to_string())
        })?;

        let id = obj
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| IpcError::BadRequest("Missing 'id' field".to_string()))?;

        let method = obj
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_string();

        let url = obj
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IpcError::BadRequest("Missing 'url' field".to_string()))?
            .to_string();

        // Parse headers
        let headers = if let Some(headers_obj) = obj.get("headers").and_then(|v| v.as_object()) {
            headers_obj
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        } else {
            HashMap::new()
        };

        // Parse body if present
        let body = if let Some(body_value) = obj.get("body") {
            let content_type = headers
                .get("Content-Type")
                .map(|s| s.as_str())
                .unwrap_or("application/json");

            if body_value.is_string() {
                // Body is a string - parse based on Content-Type
                let body_str = body_value.as_str().unwrap();
                Some(parse_body(content_type, body_str)?)
            } else {
                // Body is already JSON object
                Some(RequestBody::Json(body_value.clone()))
            }
        } else {
            None
        };

        Ok(IpcRequest {
            id,
            method,
            url,
            headers,
            body,
        })
    }

    /// Returns a list of all registered routes
    ///
    /// Useful for debugging and introspection.
    pub fn list_routes(&self) -> Vec<(String, String)> {
        self.routes
            .iter()
            .map(|route| (route.method.clone(), route.pattern.clone()))
            .collect()
    }

    /// Start the IPC message handler loop
    ///
    /// This method sets up the eval channel and automatically handles incoming
    /// IPC requests, dispatching them to registered routes and sending responses back.
    ///
    /// # Example
    /// ```rust,ignore
    /// use dioxus::prelude::*;
    /// use dioxus_ipc_bridge::prelude::*;
    ///
    /// fn app() -> Element {
    ///     let router = use_signal(|| {
    ///         let mut r = IpcRouter::new();
    ///         r.register("POST", "/greeting", Box::new(GreetingHandler));
    ///         r
    ///     });
    ///
    ///     use_effect(move || {
    ///         router.read().start();
    ///     });
    ///
    ///     rsx! { /* ... */ }
    /// }
    /// ```
    pub fn start(&self) {
        let ipc_router = self.clone();

        #[cfg(not(target_arch = "wasm32"))]
        {
            use dioxus::prelude::*;
            spawn(async move {
                let mut eval = dioxus::document::eval("window.dioxus = dioxus;");

                loop {
                    match eval.recv::<Value>().await {
                        Ok(msg) => {
                            println!("📨 IPC Router received: {:?}", msg);

                            // Dispatch to router
                            let response = ipc_router.dispatch(&msg);

                            // Send response back via callback
                            if let Some(request_id) = msg.get("id").and_then(|v| v.as_i64()) {
                                let script = format!(
                                    r#"
                                    if (window.dioxusBridge && window.dioxusBridge.callbacks) {{
                                        const callback = window.dioxusBridge.callbacks.get({});
                                        if (callback) {{
                                            callback.resolve({});
                                            window.dioxusBridge.callbacks.delete({});
                                        }}
                                    }}
                                    "#,
                                    request_id,
                                    serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string()),
                                    request_id
                                );

                                let _ = dioxus::document::eval(&script);
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ IPC Router error: {:?}", e);
                            break;
                        }
                    }
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            use dioxus::prelude::*;
            use wasm_bindgen::prelude::*;

            spawn(async move {
                // WASM-specific implementation
                let eval_result = dioxus::document::eval("window.dioxus = dioxus;");
                let mut eval = eval_result;

                loop {
                    match eval.recv::<Value>().await {
                        Ok(msg) => {
                            // Dispatch to router
                            let response = ipc_router.dispatch(&msg);

                            // Send response back
                            if let Some(request_id) = msg.get("id").and_then(|v| v.as_i64()) {
                                let script = format!(
                                    r#"
                                    if (window.dioxusBridge && window.dioxusBridge.callbacks) {{
                                        const callback = window.dioxusBridge.callbacks.get({});
                                        if (callback) {{
                                            callback.resolve({});
                                            window.dioxusBridge.callbacks.delete({});
                                        }}
                                    }}
                                    "#,
                                    request_id,
                                    serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string()),
                                    request_id
                                );

                                let _ = dioxus::document::eval(&script);
                            }
                        }
                        Err(e) => {
                            web_sys::console::error_1(&format!("IPC Router error: {:?}", e).into());
                            break;
                        }
                    }
                }
            });
        }
    }
}

impl Default for IpcRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for IpcRouter
pub struct IpcRouterBuilder {
    routes: Vec<(String, String, Box<dyn RouteHandler>)>,
}

impl IpcRouterBuilder {
    /// Create a new router builder
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route to the router
    ///
    /// # Arguments
    /// * `method` - HTTP method (e.g., "GET", "POST")
    /// * `pattern` - URL pattern (e.g., "/user/:id")
    /// * `handler` - Route handler
    ///
    /// # Example
    /// ```rust,ignore
    /// use dioxus_ipc_bridge::prelude::*;
    ///
    /// let router = IpcRouter::builder()
    ///     .route("GET", "/hello/:name", Box::new(HelloHandler))
    ///     .route("POST", "/submit", Box::new(SubmitHandler))
    ///     .build();
    /// ```
    pub fn route(mut self, method: &str, pattern: &str, handler: Box<dyn RouteHandler>) -> Self {
        self.routes
            .push((method.to_string(), pattern.to_string(), handler));
        self
    }

    /// Build the IpcRouter
    pub fn build(self) -> IpcRouter {
        let mut router = IpcRouter::new();
        for (method, pattern, handler) in self.routes {
            router.register(&method, &pattern, handler);
        }
        router
    }
}

impl Default for IpcRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
