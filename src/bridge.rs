//! IPC Bridge - JavaScript/Rust communication layer
//!
//! This module provides the core bridge between JavaScript and Rust, enabling
//! bidirectional communication using an HTTP-like API.

use crate::platform;
use crate::plugin::BridgePlugin;
use std::time::Duration;

/// Configuration for the IPC bridge
#[derive(Clone, PartialEq)]
pub struct IpcBridge {
    /// Request timeout duration
    pub timeout: Duration,
    /// Custom bridge JavaScript (if provided)
    pub custom_script: Option<String>,
    /// Registered plugins
    plugins: Vec<String>, // Store plugin names for now
}

impl IpcBridge {
    /// Create a new IPC bridge builder
    pub fn builder() -> IpcBridgeBuilder {
        IpcBridgeBuilder::new()
    }

    /// Generate the complete bridge initialization script
    ///
    /// This includes the core bridge script plus any plugin scripts
    pub fn generate_script(&self) -> String {
        let script = if let Some(custom) = &self.custom_script {
            custom.clone()
        } else {
            Self::default_bridge_script(self.timeout.as_millis() as u32)
        };

        // Add plugin scripts (would iterate over actual plugins in full implementation)
        // For now, just return the base script
        script
    }

    /// Initialize the bridge in the JavaScript runtime
    pub fn initialize(&self) {
        let script = self.generate_script();
        platform::eval_js(&script);
    }

    /// Default bridge script generation
    fn default_bridge_script(timeout_ms: u32) -> String {
        format!(r#"
        // Complete Dioxus Bridge - Unified IPC API
        // Preserve existing properties (like IPCBridge from React) if already initialized
        if (!window.dioxusBridge) {{
            window.dioxusBridge = {{}};
        }}

        // Preserve IPCBridge if it was already attached by React
        const existingIPCBridge = window.dioxusBridge.IPCBridge;

        // Preserve or create callbacks Map (MUST be same reference)
        if (!window.dioxusBridge.callbacks) {{
            window.dioxusBridge.callbacks = new Map();
        }}
        const callbacks = window.dioxusBridge.callbacks;

        // Add/update core Rust-provided properties
        Object.assign(window.dioxusBridge, {{
            // Internal callback storage (previously window.dioxusBridgeCallbacks)
            callbacks: callbacks,

            // HTTP-like fetch() method for IPC communication
            fetch: function(url, options = {{}}) {{
                return new Promise((resolve, reject) => {{
                    const requestId = Math.floor(Math.random() * 1000000);

                    // Store callback in namespaced location (use explicit reference)
                    window.dioxusBridge.callbacks.set(requestId, {{resolve, reject}});

                    // Build HTTP-like IPC request
                    const request = {{
                        id: requestId,
                        method: options.method || 'GET',
                        url: url,
                        headers: options.headers || {{}},
                        body: options.body
                    }};

                    // Send to Rust via dioxus.send()
                    if (typeof dioxus !== 'undefined' && typeof dioxus.send === 'function') {{
                        dioxus.send(request);
                    }} else {{
                        reject(new Error('dioxus.send() not available'));
                        return;
                    }}

                    // Configurable timeout
                    setTimeout(() => {{
                        if (window.dioxusBridge && window.dioxusBridge.callbacks && window.dioxusBridge.callbacks.has(requestId)) {{
                            window.dioxusBridge.callbacks.delete(requestId);
                            reject(new Error('Request timeout after {} ms'));
                        }}
                    }}, {});
                }});
            }},

            // Rust → React event emitter (previously window.rustEmit)
            rustEmit: function(channel, data) {{
                if (window.dioxusBridge.IPCBridge && typeof window.dioxusBridge.IPCBridge.emit === 'function') {{
                    window.dioxusBridge.IPCBridge.emit(channel, data);
                }} else {{
                    console.warn('[Rust] IPCBridge not available, event not emitted:', channel);
                }}
            }},

            // Direct IPC interface
            ipc: {{
                send: function(data) {{
                    if (typeof dioxus !== 'undefined' && typeof dioxus.send === 'function') {{
                        dioxus.send(data);
                    }} else {{
                        console.error('[Rust] dioxus.send() not available');
                    }}
                }},
                hasIPCBridge: function() {{
                    return typeof window.dioxusBridge.IPCBridge !== 'undefined';
                }}
            }},

            // Low-level send wrapper
            send: function(data) {{
                if (window.dioxusBridge && window.dioxusBridge.ipc) {{
                    window.dioxusBridge.ipc.send(data);
                }}
            }}
        }});

        // Restore IPCBridge if it existed before
        if (existingIPCBridge) {{
            window.dioxusBridge.IPCBridge = existingIPCBridge;
        }}

        console.log('[Rust] window.dioxusBridge ready (Unified IPC API)');
        new Promise(() => {{}}); // Keep eval alive
        "#, timeout_ms, timeout_ms)
    }
}

/// Builder for IpcBridge configuration
pub struct IpcBridgeBuilder {
    timeout: Duration,
    custom_script: Option<String>,
    plugins: Vec<Box<dyn BridgePlugin>>,
}

impl IpcBridgeBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            custom_script: None,
            plugins: Vec::new(),
        }
    }

    /// Set the request timeout duration
    ///
    /// Default is 10 seconds
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::time::Duration;
    /// use dioxus_ipc_bridge::IpcBridge;
    ///
    /// let bridge = IpcBridge::builder()
    ///     .timeout(Duration::from_secs(30))
    ///     .build();
    /// ```
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    /// Provide a custom bridge script
    ///
    /// Use this to completely replace the default bridge JavaScript
    pub fn custom_script(mut self, script: String) -> Self {
        self.custom_script = Some(script);
        self
    }

    /// Add a plugin to extend bridge functionality
    ///
    /// # Example
    /// ```rust,ignore
    /// let bridge = IpcBridge::builder()
    ///     .plugin(Box::new(LoggingPlugin::new()))
    ///     .plugin(Box::new(AuthPlugin::new()))
    ///     .build();
    /// ```
    pub fn plugin(mut self, plugin: Box<dyn BridgePlugin>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Build the IpcBridge configuration
    pub fn build(self) -> IpcBridge {
        IpcBridge {
            timeout: self.timeout,
            custom_script: self.custom_script,
            plugins: self.plugins.iter().map(|p| p.name().to_string()).collect(),
        }
    }
}

impl Default for IpcBridgeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate the default Dioxus Bridge initialization script
///
/// This creates a window.dioxusBridge object that provides an HTTP-like fetch API
/// for React to communicate with Rust via dioxus.send().
///
/// The bridge handles:
/// - Request/response pairing via unique IDs
/// - Promise-based async communication
/// - Configurable timeout for requests
/// - Error handling for failed calls
/// - HTTP-like request/response format
///
/// # Returns
/// JavaScript code string that sets up window.dioxusBridge
///
/// # Example JavaScript Usage
/// ```javascript
/// // GET request
/// const response = await window.dioxusBridge.fetch('ipc://calculator/fibonacci?number=10');
/// console.log(response.body.result); // 55
///
/// // POST request with body
/// const response = await window.dioxusBridge.fetch('ipc://form/submit', {
///     method: 'POST',
///     headers: { 'Content-Type': 'application/json' },
///     body: { name: 'John', email: 'john@example.com' }
/// });
/// ```
pub fn generate_dioxus_bridge_script() -> &'static str {
    // Default 10-second timeout
    r#"
        // Complete Dioxus Bridge - Unified IPC API
        // Preserve existing properties (like IPCBridge from React) if already initialized
        if (!window.dioxusBridge) {
            window.dioxusBridge = {};
        }

        // Preserve IPCBridge if it was already attached by React
        const existingIPCBridge = window.dioxusBridge.IPCBridge;

        // Preserve or create callbacks Map (MUST be same reference)
        if (!window.dioxusBridge.callbacks) {
            window.dioxusBridge.callbacks = new Map();
        }
        const callbacks = window.dioxusBridge.callbacks;

        // Add/update core Rust-provided properties
        Object.assign(window.dioxusBridge, {
            // Internal callback storage (previously window.dioxusBridgeCallbacks)
            callbacks: callbacks,

            // HTTP-like fetch() method for IPC communication
            fetch: function(url, options = {}) {
                return new Promise((resolve, reject) => {
                    const requestId = Math.floor(Math.random() * 1000000);

                    // Store callback in namespaced location (use explicit reference)
                    window.dioxusBridge.callbacks.set(requestId, {resolve, reject});

                    // Build HTTP-like IPC request
                    const request = {
                        id: requestId,
                        method: options.method || 'GET',
                        url: url,
                        headers: options.headers || {},
                        body: options.body
                    };

                    // Send to Rust via dioxus.send()
                    if (typeof dioxus !== 'undefined' && typeof dioxus.send === 'function') {
                        dioxus.send(request);
                    } else {
                        reject(new Error('dioxus.send() not available'));
                        return;
                    }

                    // 10 second timeout
                    setTimeout(() => {
                        if (window.dioxusBridge && window.dioxusBridge.callbacks && window.dioxusBridge.callbacks.has(requestId)) {
                            window.dioxusBridge.callbacks.delete(requestId);
                            reject(new Error('Request timeout after 10 seconds'));
                        }
                    }, 10000);
                });
            },

            // Rust → React event emitter (previously window.rustEmit)
            rustEmit: function(channel, data) {
                if (window.dioxusBridge.IPCBridge && typeof window.dioxusBridge.IPCBridge.emit === 'function') {
                    window.dioxusBridge.IPCBridge.emit(channel, data);
                } else {
                    console.warn('[Rust] IPCBridge not available, event not emitted:', channel);
                }
            },

            // Direct IPC interface
            ipc: {
                send: function(data) {
                    if (typeof dioxus !== 'undefined' && typeof dioxus.send === 'function') {
                        dioxus.send(data);
                    } else {
                        console.error('[Rust] dioxus.send() not available');
                    }
                },
                hasIPCBridge: function() {
                    return typeof window.dioxusBridge.IPCBridge !== 'undefined';
                }
            },

            // Low-level send wrapper
            send: function(data) {
                if (window.dioxusBridge && window.dioxusBridge.ipc) {
                    window.dioxusBridge.ipc.send(data);
                }
            }
        });

        // Restore IPCBridge if it existed before
        if (existingIPCBridge) {
            window.dioxusBridge.IPCBridge = existingIPCBridge;
        }

        console.log('[Rust] window.dioxusBridge ready (Unified IPC API)');
        new Promise(() => {}); // Keep eval alive
    "#
}

/// Emit an event from Rust to JavaScript/React
///
/// This allows Rust to actively push data to the JavaScript side.
/// React components can subscribe to these events using the bridge's event system.
///
/// # Arguments
/// * `channel` - The event channel name (e.g., "notification", "progress", "data:stream")
/// * `data` - The data to send (must be JSON-serializable)
///
/// # Example
/// ```rust
/// use serde_json::json;
/// use dioxus_ipc_bridge::bridge::emit;
///
/// // Emit a notification
/// emit("notification", json!({
///     "type": "info",
///     "message": "Processing complete!"
/// }));
///
/// // Stream progress updates
/// emit("progress", json!({
///     "percent": 75,
///     "status": "Almost done..."
/// }));
/// ```
pub fn emit(channel: &str, data: serde_json::Value) {
    let data_json = data.to_string();
    let script = format!(
        r#"
        if (window.dioxusBridge && window.dioxusBridge.rustEmit) {{
            window.dioxusBridge.rustEmit('{}', {});
        }}
        "#,
        channel, data_json
    );
    platform::eval_js(&script);
}
