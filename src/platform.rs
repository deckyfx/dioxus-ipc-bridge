//! Platform-agnostic abstractions for cross-platform JavaScript execution
//!
//! This module provides unified APIs that work across desktop (webview),
//! web (WASM), and mobile platforms.

/// Execute JavaScript code in the current runtime context
///
/// Works across desktop (webview) and web (browser) platforms
///
/// # Arguments
/// * `script` - The JavaScript code to execute
///
/// # Platform Behavior
/// - **Desktop**: Uses `dioxus::document::eval()` to execute in webview context
/// - **Web/WASM**: Uses `js_sys::eval()` to execute in browser context
pub fn eval_js(script: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Desktop: Use dioxus document::eval (webview context)
        dioxus::document::eval(script);
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Web: Use js_sys::eval which works in WASM
        let _ = js_sys::eval(script);
    }
}

/// Platform-agnostic async sleep
///
/// # Arguments
/// * `duration` - How long to sleep
///
/// # Platform Behavior
/// - **Desktop**: Uses `tokio::time::sleep()`
/// - **Web/WASM**: Uses `gloo_timers::future::sleep()`
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: std::time::Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: std::time::Duration) {
    gloo_timers::future::sleep(duration).await;
}

/// Get current timestamp as ISO 8601 string
///
/// Works across all platforms (desktop, web, mobile)
///
/// # Returns
/// ISO 8601 formatted timestamp string
///
/// # Platform Behavior
/// - **Desktop**: Uses `chrono::Utc::now().to_rfc3339()`
/// - **Web/WASM**: Uses JavaScript `Date` object
#[cfg(not(target_arch = "wasm32"))]
pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(target_arch = "wasm32")]
pub fn now_iso8601() -> String {
    // Use JavaScript Date for WASM
    let date = js_sys::Date::new_0();
    date.to_iso_string().as_string().unwrap_or_default()
}
