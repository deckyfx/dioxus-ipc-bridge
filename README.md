# dioxus-ipc-bridge

[![Crates.io](https://img.shields.io/crates/v/dioxus-ipc-bridge.svg)](https://crates.io/crates/dioxus-ipc-bridge)
[![Documentation](https://docs.rs/dioxus-ipc-bridge/badge.svg)](https://docs.rs/dioxus-ipc-bridge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

A powerful HTTP-like IPC bridge for Dioxus applications that enables **bidirectional communication** between JavaScript/React and Rust across **desktop, web, and mobile** platforms.

## Features

- **HTTP-like API**: Familiar request-response pattern with methods, URLs, path parameters, and query strings
- **Bidirectional Communication**:
  - JavaScript → Rust: Request/response pattern
  - Rust → JavaScript: Event emission via channels
- **Platform-Agnostic**: Works seamlessly on desktop (webview), web (WASM), and mobile
- **Type-Safe**: Full Rust type safety with serde serialization
- **Router System**: URL-based routing with path parameters (`:param`) and query strings
- **Builder API**: Clean, fluent interface for configuration

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dioxus = "0.7.0"
dioxus-ipc-bridge = { path = "../dioxus-ipc-bridge" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Basic Example

Here's a complete working example based on the dxbasic implementation:

```rust
use dioxus::prelude::*;
use dioxus_ipc_bridge::prelude::*;
use serde_json::json;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // 1. Create IPC bridge with timeout configuration
    let bridge = IpcBridge::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    // 2. Set up router with routes
    let router = use_signal(|| {
        IpcRouter::builder()
            // Simple route
            .route("POST", "/ping", Box::new(PingHandler))

            // Route with path parameter
            .route("POST", "/greeting/:name", Box::new(GreetingHandler))

            // Route with state management
            .route("POST", "/counter/increment", Box::new(CounterHandler))
            .route("GET", "/counter/value", Box::new(CounterValueHandler))

            .build()
    });

    // 3. Initialize bridge and start router
    use_effect(move || {
        // Start the router's eval loop to listen for messages from JS
        router.read().start();

        // Optional: Emit events from Rust to JavaScript
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                // Emit heartbeat event to JavaScript
                bridge::emit("rust:heartbeat", json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "message": "Heartbeat from Rust"
                }));
            }
        });
    });

    // 4. Generate and inject bridge script BEFORE other content
    let bridge_script = bridge.generate_script();

    rsx! {
        // IMPORTANT: Inject bridge script first!
        script { dangerous_inner_html: "{bridge_script}" }

        // Your app content
        div { "Dioxus IPC Bridge Ready!" }
    }
}

// ========== Route Handlers ==========

/// Simple ping-pong handler
struct PingHandler;

impl RouteHandler for PingHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let client_time = req.body
            .as_ref()
            .and_then(|b| match b {
                RequestBody::Json(json) => json.get("timestamp").and_then(|t| t.as_str()),
                _ => None
            })
            .unwrap_or("unknown");

        Ok(IpcResponse::ok(json!({
            "message": "pong",
            "server_timestamp": timestamp,
            "client_timestamp": client_time
        })))
    }
}

/// Handler with path parameter
struct GreetingHandler;

impl RouteHandler for GreetingHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        // Extract path parameter
        let name = req.path_param("name")
            .ok_or_else(|| IpcError::BadRequest("Missing name parameter".to_string()))?;

        // Extract query parameter (optional)
        let language = req.query_param("lang").map(|s| s.as_str()).unwrap_or("en");

        let greeting = match language {
            "es" => format!("¡Hola, {}!", name),
            "fr" => format!("Bonjour, {}!", name),
            _ => format!("Hello, {}!", name),
        };

        Ok(IpcResponse::ok(json!({
            "message": greeting,
            "name": name,
            "language": language
        })))
    }
}

/// Handler with global state (simplified - use proper state management in production)
static mut GLOBAL_COUNTER: i32 = 0;

struct CounterHandler;

impl RouteHandler for CounterHandler {
    fn handle(&self, _req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        unsafe {
            GLOBAL_COUNTER += 1;

            // Emit event to all JavaScript listeners
            bridge::emit("rust:counter:update", json!({
                "count": GLOBAL_COUNTER,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }));

            Ok(IpcResponse::ok(json!({
                "count": GLOBAL_COUNTER,
                "message": "Counter incremented"
            })))
        }
    }
}

struct CounterValueHandler;

impl RouteHandler for CounterValueHandler {
    fn handle(&self, _req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        unsafe {
            Ok(IpcResponse::ok(json!({
                "count": GLOBAL_COUNTER
            })))
        }
    }
}
```

### JavaScript/React Usage

```typescript
// Simple GET/POST request
const response = await window.dioxusBridge.fetch('ipc://ping', {
  method: 'POST',
  body: { timestamp: new Date().toISOString() }
});
console.log(response.body.message); // "pong"

// Request with path parameter
const response = await window.dioxusBridge.fetch('ipc://greeting/World', {
  method: 'POST'
});
console.log(response.body.message); // "Hello, World!"

// Request with query string
const response = await window.dioxusBridge.fetch('ipc://greeting/World?lang=es', {
  method: 'POST'
});
console.log(response.body.message); // "¡Hola, World!"

// Listen to events from Rust
const subscription = window.dioxusBridge.IPCBridge.on('rust:heartbeat').subscribe({
  next: (data) => {
    console.log('Heartbeat:', data.message);
  },
  error: (err) => console.error('Error:', err)
});

// Cleanup when done
subscription.unsubscribe();
```

## Core Concepts

### 1. IpcBridge

The bridge manages the JavaScript-Rust communication layer:

```rust
let bridge = IpcBridge::builder()
    .timeout(Duration::from_secs(30))  // Request timeout
    .build();

// Generate JavaScript initialization code
let bridge_script = bridge.generate_script();
```

**Important**: The bridge script must be injected **before** any JavaScript code that uses `window.dioxusBridge`.

### 2. IpcRouter

Routes IPC requests to appropriate handlers:

```rust
let router = IpcRouter::builder()
    .route("GET", "/path", Box::new(GetHandler))
    .route("POST", "/path", Box::new(PostHandler))
    .route("GET", "/users/:id", Box::new(UserHandler))  // Path parameter
    .build();

// Start listening for requests
router.start();
```

### 3. Route Handlers

Implement the `RouteHandler` trait to handle requests:

```rust
struct MyHandler;

impl RouteHandler for MyHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        // Extract path parameters
        let id = req.path_param("id")?;

        // Extract query parameters
        let filter = req.query_param("filter").unwrap_or(&"all".to_string());

        // Access request body
        if let Some(RequestBody::Json(data)) = &req.body {
            let name = data["name"].as_str().unwrap();
        }

        // Return response
        Ok(IpcResponse::ok(json!({
            "status": "success",
            "data": { "id": id }
        })))
    }
}
```

### 4. Event Emission (Rust → JavaScript)

Send events from Rust to JavaScript listeners:

```rust
use dioxus_ipc_bridge::bridge;

// Emit event
bridge::emit("event:name", json!({
    "data": "value"
}));

// Emit event with channel namespace
bridge::emit("rust:counter:update", json!({
    "count": 42
}));
```

### 5. EnrichedRequest API

The `EnrichedRequest` provides convenient accessors:

```rust
fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
    // Path parameters (from :param in route)
    let user_id = req.path_param("id")?;

    // Query parameters (from ?key=value)
    let page = req.query_param("page")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);

    // Headers
    let content_type = req.header("Content-Type")?;

    // Request body (JSON, URL-encoded, or Multipart)
    match &req.body {
        Some(RequestBody::Json(json)) => {
            let name = json["name"].as_str().unwrap();
        }
        Some(RequestBody::UrlEncoded(fields)) => {
            let email = fields.get("email").unwrap();
        }
        _ => {}
    }

    Ok(IpcResponse::ok(json!({ "status": "ok" })))
}
```

## Advanced Patterns

### Hybrid Request-Response + Event Pattern

For operations that need immediate response AND broadcast updates:

```rust
impl RouteHandler for CounterHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        let new_value = increment_counter();

        // 1. Emit event to ALL listeners (broadcast)
        bridge::emit("counter:updated", json!({
            "count": new_value
        }));

        // 2. Return response to requester
        Ok(IpcResponse::ok(json!({
            "count": new_value
        })))
    }
}
```

JavaScript side:

```typescript
// Listen for broadcasts (all tabs/windows get this)
window.dioxusBridge.IPCBridge.on('counter:updated').subscribe({
  next: (data) => setCounter(data.count)
});

// Make request (gets response + triggers broadcast)
const response = await window.dioxusBridge.fetch('ipc://counter/increment', {
  method: 'POST'
});
```

### Platform-Specific Code

Handle desktop vs web differences:

```rust
/// Get current timestamp (works on both desktop and WASM)
fn get_timestamp() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        chrono::Utc::now().to_rfc3339()
    }

    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::new_0().to_iso_string().as_string().unwrap_or_default()
    }
}

// Use platform-specific sleep
#[cfg(not(target_arch = "wasm32"))]
tokio::time::sleep(Duration::from_secs(2)).await;

#[cfg(target_arch = "wasm32")]
{
    use gloo_timers::future::TimeoutFuture;
    TimeoutFuture::new(2000).await;
}
```

### Error Handling

```rust
impl RouteHandler for MyHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        // Bad request (400)
        let id = req.path_param("id")
            .ok_or_else(|| IpcError::BadRequest("Missing id".to_string()))?;

        // Not found (404)
        if !user_exists(id) {
            return Err(IpcError::NotFound(format!("User {} not found", id)));
        }

        // Custom error with status code
        Err(IpcError::Custom {
            status: 403,
            message: "Access denied".to_string()
        })
    }
}
```

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Desktop (Windows/macOS/Linux) | ✅ Fully Supported | Uses `dioxus::document::eval()` with webview |
| Web (WASM) | ✅ Fully Supported | Uses `js_sys` for web APIs |
| Mobile (iOS/Android) | ✅ Fully Supported | Same as desktop with mobile webview |

### WASM-Specific Dependencies

For web builds, add these to your `Cargo.toml`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console"] }
gloo-timers = { version = "0.3", features = ["futures"] }
```

## Common Patterns from dxbasic

### 1. Dynamic Asset Loading

```rust
struct AssetHandler;

impl RouteHandler for AssetHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        let name = req.path_param("name")?;

        match name.as_str() {
            "image" => {
                let asset = asset!("/assets/sample.png");
                Ok(IpcResponse::ok(json!({
                    "name": "sample.png",
                    "type": "image/png",
                    "data": asset.to_string()
                })))
            }
            _ => Err(IpcError::NotFound("Asset not found".into()))
        }
    }
}
```

### 2. Echo Service

```rust
struct EchoHandler;

impl RouteHandler for EchoHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        let message = req.body
            .as_ref()
            .and_then(|b| match b {
                RequestBody::Json(json) => json.get("message").and_then(|m| m.as_str()),
                _ => None
            })
            .unwrap_or("(empty message)");

        Ok(IpcResponse::ok(json!({
            "echo": message,
            "length": message.len(),
            "reversed": message.chars().rev().collect::<String>(),
            "uppercase": message.to_uppercase()
        })))
    }
}
```

### 3. Background Event Emitter

```rust
use_effect(move || {
    spawn(async move {
        let mut count = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            count += 1;

            bridge::emit("rust:heartbeat", json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": format!("Heartbeat #{}", count),
                "count": count
            }));
        }
    });
});
```

## Troubleshooting

### IPC Timeout Errors

If requests timeout:
1. Verify bridge script loads first: Check console for `[Rust] window.dioxusBridge ready`
2. Ensure router is started: `router.read().start()`
3. Check route registration: Routes must match exactly (case-sensitive)

### Events Not Received

If JavaScript doesn't receive events:
1. Use Observable pattern: `.on(channel).subscribe({ next: ... })`
2. Check channel names match between Rust and JavaScript
3. Verify listeners registered before events emitted

### WASM Build Errors

For web builds:
1. Add `web-sys` dependency with required features
2. Use platform-specific code (`#[cfg(target_arch = "wasm32")]`)
3. Replace `std::fs`, `tokio`, `chrono` with web alternatives

## API Reference

### Core Types

- **`IpcBridge`**: Bridge configuration and script generation
- **`IpcRouter`**: Route management and request dispatching
- **`IpcRequest`**: Incoming request from JavaScript
- **`IpcResponse`**: Response to JavaScript
- **`EnrichedRequest`**: Parsed request with convenient accessors
- **`RouteHandler`**: Trait for implementing handlers
- **`IpcError`**: Error types (BadRequest, NotFound, Custom, etc.)
- **`RequestBody`**: Body variants (Json, UrlEncoded, Multipart)

### Builder APIs

```rust
// IpcBridge builder
IpcBridge::builder()
    .timeout(Duration::from_secs(30))
    .build();

// IpcRouter builder
IpcRouter::builder()
    .route(method, path, handler)
    .build();
```

### Response Helpers

```rust
// Success responses
IpcResponse::ok(json!({ "data": "value" }))
IpcResponse::created(json!({ "id": 123 }))

// Error responses
IpcResponse::bad_request("Invalid input")
IpcResponse::not_found("Resource not found")
IpcResponse::custom(403, "Forbidden")
```

## Examples

See the [dioxus-react-example](https://github.com/deckyfx/dioxus-react-example) for a complete working example with:
- IPC bridge initialization and route handlers
- Ping-pong request/response
- Echo service with JSON body processing
- State management with counter (atomic + event broadcast)
- Heartbeat event emission (Rust → React)
- React app with Tailwind CSS loaded via folder assets

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Related

- [dioxus-react-example](https://github.com/deckyfx/dioxus-react-example) - Complete working example app
- [dioxus-react-integration](https://github.com/deckyfx/LearningDioxus) - Serve React apps in Dioxus with `ReactApp` component
- [dioxus-react-bridge](https://github.com/deckyfx/dioxus-react-bridge) - React hooks and components for IPC communication
- [dioxus-ipc-bridge-macros](https://github.com/deckyfx/LearningDioxus) - Procedural macros for route handlers
- [Dioxus](https://dioxuslabs.com/) - Rust GUI framework
