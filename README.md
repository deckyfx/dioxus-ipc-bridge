# dioxus-ipc-bridge

[![Crates.io](https://img.shields.io/crates/v/dioxus-ipc-bridge.svg)](https://crates.io/crates/dioxus-ipc-bridge)
[![Documentation](https://docs.rs/dioxus-ipc-bridge/badge.svg)](https://docs.rs/dioxus-ipc-bridge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

A powerful HTTP-like IPC bridge for Dioxus applications that enables **bidirectional communication** between JavaScript/React and Rust across **desktop, web, and mobile** platforms.

## Features

- **HTTP-like API**: Familiar request-response pattern with methods, URLs, headers, and bodies
- **Bidirectional**: JavaScript → Rust requests **and** Rust → JavaScript event streaming
- **Platform-Agnostic**: Works seamlessly on desktop (webview), web (WASM), and mobile
- **Type-Safe**: Full Rust type safety with serde serialization
- **Streaming Support**: Long-running tasks with progress updates (optional feature)
- **Plugin System**: Extend functionality with custom plugins and middleware
- **Macro Support**: Ergonomic route handlers with `#[ipc_route]` macro
- **Builder API**: Clean, fluent interface for configuration

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dioxus = "0.7.0-rc.1"
dioxus-ipc-bridge = "0.1"

# Optional: Enable streaming support
# dioxus-ipc-bridge = { version = "0.1", features = ["streaming"] }
```

### Basic Usage

```rust
use dioxus::prelude::*;
use dioxus_ipc_bridge::prelude::*;

// 1. Define a route handler
struct HelloHandler;

impl RouteHandler for HelloHandler {
    fn handle(&self, req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
        let name = req.path_param("name")
            .ok_or(IpcError::BadRequest("Missing name".into()))?;

        Ok(IpcResponse::ok(serde_json::json!({
            "message": format!("Hello, {}!", name)
        })))
    }
}

fn main() {
    // 2. Create and configure the bridge
    let bridge = IpcBridge::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    // 3. Set up routes
    let router = IpcRouter::builder()
        .route("GET", "/hello/:name", Box::new(HelloHandler))
        .build();

    // 4. Use in your Dioxus app
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    use_effect(move || {
        // Initialize bridge
        let bridge_script = generate_dioxus_bridge_script();
        dioxus::document::eval(bridge_script);

        // Listen for requests
        spawn(async move {
            let mut eval_instance = dioxus::document::eval(bridge_script);
            loop {
                if let Ok(request) = eval_instance.recv::<serde_json::Value>().await {
                    let response = router.dispatch(&request);
                    let response_json = serde_json::to_value(&response).unwrap();

                    // Send response back to JavaScript
                    let callback_script = format!(
                        r#"
                        if (window.dioxusBridge.callbacks.has({})) {{
                            window.dioxusBridge.callbacks.get({}).resolve({});
                            window.dioxusBridge.callbacks.delete({});
                        }}
                        "#,
                        request["id"], request["id"], response_json, request["id"]
                    );
                    dioxus::document::eval(&callback_script);
                }
            }
        });
    });

    rsx! {
        div { "Dioxus IPC Bridge Ready!" }
    }
}
```

### JavaScript/React Usage

```typescript
// Simple GET request
const response = await window.dioxusBridge.fetch('ipc://hello/world');
console.log(response.body.message); // "Hello, world!"

// POST request with body
const response = await window.dioxusBridge.fetch('ipc://form/submit', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: { name: 'John', email: 'john@example.com' }
});
```

## Advanced Usage

### Using the Macro (Simpler Syntax)

```rust
use dioxus_ipc_bridge::prelude::*;

#[ipc_route(GET, "/user/:id")]
fn get_user(req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
    let user_id = req.path_param("id").unwrap();

    Ok(IpcResponse::ok(serde_json::json!({
        "id": user_id,
        "name": "John Doe"
    })))
}
```

### Path Parameters and Query Strings

```rust
// URL: ipc://search/users?query=john&limit=10
fn search_users(req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
    let query = req.query_param("query").unwrap_or(&"".to_string());
    let limit = req.query_param("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    // Search logic here...

    Ok(IpcResponse::ok(serde_json::json!({
        "results": [],
        "query": query,
        "limit": limit
    })))
}
```

### Request Body Handling

The bridge supports three body formats:

```rust
fn handle_submission(req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
    match &req.body {
        Some(RequestBody::Json(data)) => {
            // Handle JSON body
            let name = data["name"].as_str().unwrap();
            Ok(IpcResponse::ok(serde_json::json!({ "status": "ok" })))
        },
        Some(RequestBody::UrlEncoded(fields)) => {
            // Handle URL-encoded form data
            let name = fields.get("name").unwrap();
            Ok(IpcResponse::ok(serde_json::json!({ "status": "ok" })))
        },
        Some(RequestBody::Multipart { fields, files }) => {
            // Handle file uploads
            for file in files {
                println!("Uploaded: {}", file.filename);
            }
            Ok(IpcResponse::ok(serde_json::json!({ "status": "ok" })))
        },
        None => Err(IpcError::BadRequest("Missing body".into()))
    }
}
```

### Rust → JavaScript Events

Emit events from Rust to JavaScript:

```rust
use dioxus_ipc_bridge::bridge::emit;

// Emit a notification
emit("notification", serde_json::json!({
    "type": "info",
    "message": "Processing complete!"
}));

// Stream progress updates
emit("progress", serde_json::json!({
    "percent": 75,
    "status": "Almost done..."
}));
```

Listen in JavaScript:

```javascript
// Assuming you have an event system like RxJS
window.dioxusBridge.IPCBridge.on('notification', (data) => {
  console.log('Notification:', data.message);
});

window.dioxusBridge.IPCBridge.on('progress', (data) => {
  console.log(`Progress: ${data.percent}%`);
});
```

### Streaming Support (Optional Feature)

For long-running operations with progress tracking:

```toml
[dependencies]
dioxus-ipc-bridge = { version = "0.1", features = ["streaming"] }
```

```rust
use dioxus_ipc_bridge::streaming::StreamingTask;

fn start_long_operation(req: &EnrichedRequest) -> Result<IpcResponse, IpcError> {
    let task = StreamingTask::new();
    let task_id = task.task_id.clone();

    // Return task ID immediately
    let response = IpcResponse::ok(task.initial_response());

    // Process in background
    spawn(async move {
        for i in 0..=100 {
            task.emit_percent(i as f32);
            sleep(Duration::from_millis(100)).await;
        }
        task.emit_complete(serde_json::json!({ "status": "done" }));
    });

    Ok(response)
}
```

JavaScript side:

```typescript
import { subscribeToStreamingTask } from './types';

const response = await ipcRequest('ipc://process/large-file');

subscribeToStreamingTask(response.task_id, {
  onProgress: (progress) => console.log(`${progress.percent}%`),
  onComplete: (result) => console.log('Done!', result)
});
```

### Plugin System

Create custom plugins to extend functionality:

```rust
use dioxus_ipc_bridge::plugin::BridgePlugin;

struct LoggingPlugin;

impl BridgePlugin for LoggingPlugin {
    fn name(&self) -> &str {
        "logging"
    }

    fn on_request(&self, req: &mut IpcRequest) -> Result<(), IpcError> {
        println!("Request: {} {}", req.method, req.url);
        Ok(())
    }

    fn on_response(&self, res: &mut IpcResponse) -> Result<(), IpcError> {
        println!("Response: {}", res.status);
        Ok(())
    }
}

// Use plugin
let bridge = IpcBridge::builder()
    .plugin(Box::new(LoggingPlugin))
    .build();
```

## API Reference

### Core Types

- **`IpcBridge`**: Bridge configuration and initialization
- **`IpcRouter`**: HTTP-like router for dispatching requests
- **`IpcRequest`**: Request from JavaScript to Rust
- **`IpcResponse`**: Response from Rust to JavaScript
- **`EnrichedRequest`**: Parsed request with path params and query strings
- **`RouteHandler`**: Trait for implementing route handlers
- **`BridgePlugin`**: Trait for creating plugins
- **`StreamingTask`**: Helper for long-running operations (requires `streaming` feature)

### Builder APIs

```rust
// IpcBridge builder
let bridge = IpcBridge::builder()
    .timeout(Duration::from_secs(30))
    .custom_script(my_custom_js)
    .plugin(Box::new(MyPlugin))
    .build();

// IpcRouter builder
let router = IpcRouter::builder()
    .route("GET", "/path/:param", handler)
    .route("POST", "/submit", handler)
    .build();
```

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Desktop (Windows/macOS/Linux) | ✅ Fully Supported | Uses webview with `dioxus::document::eval()` |
| Web (WASM) | ✅ Fully Supported | Uses `js_sys::eval()` |
| Mobile (iOS/Android) | ✅ Fully Supported | Same as desktop |

## Examples

See the `examples/` directory for complete working examples:

- `basic.rs` - Simple GET/POST routes
- `streaming.rs` - Long-running tasks with progress
- `custom_plugin.rs` - Plugin implementation

Run an example:

```bash
cargo run --example basic --features desktop
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
