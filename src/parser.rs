//! URL and Body Parser for IPC Router
//!
//! Provides utilities to parse IPC URLs (scheme, path, query parameters)
//! and request bodies (JSON, URL-encoded, multipart).

use crate::request::{FileUpload, RequestBody};
use crate::response::IpcError;
use std::collections::HashMap;

/// Parsed IPC URL components
///
/// # Example
/// ```
/// use dioxus_ipc_bridge::parser::ParsedUrl;
///
/// // URL: "ipc://calculator/fibonacci?number=10"
/// let url = ParsedUrl::parse("ipc://calculator/fibonacci?number=10").unwrap();
/// assert_eq!(url.scheme, "ipc");
/// assert_eq!(url.path, "/calculator/fibonacci");
/// assert_eq!(url.query_params.get("number"), Some(&"10".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    /// URL scheme (should be "ipc")
    pub scheme: String,

    /// Path component (e.g., "/calculator/fibonacci")
    pub path: String,

    /// Query parameters extracted from query string
    pub query_params: HashMap<String, String>,
}

impl ParsedUrl {
    /// Parse an IPC URL string
    ///
    /// # Arguments
    /// * `url` - Full IPC URL string (e.g., "ipc://calculator/fibonacci?number=10")
    ///
    /// # Returns
    /// * `Ok(ParsedUrl)` - Successfully parsed URL
    /// * `Err(IpcError)` - Invalid URL format
    pub fn parse(url: &str) -> Result<Self, IpcError> {
        // Split scheme from rest: "ipc://calculator/fibonacci?number=10"
        let parts: Vec<&str> = url.splitn(2, "://").collect();
        if parts.len() != 2 {
            return Err(IpcError::ParseError(format!(
                "Invalid URL format: missing '://' in '{}'",
                url
            )));
        }

        let scheme = parts[0].to_string();
        let rest = parts[1]; // "calculator/fibonacci?number=10"

        // Split path from query string
        let (path_part, query_part) = if let Some(pos) = rest.find('?') {
            (&rest[..pos], Some(&rest[pos + 1..]))
        } else {
            (rest, None)
        };

        // Ensure path starts with /
        let path = if path_part.starts_with('/') {
            path_part.to_string()
        } else {
            format!("/{}", path_part)
        };

        // Parse query parameters
        let query_params = if let Some(query) = query_part {
            parse_query_string(query)
        } else {
            HashMap::new()
        };

        Ok(Self {
            scheme,
            path,
            query_params,
        })
    }

    /// Match this URL against a route pattern with path parameters
    ///
    /// # Arguments
    /// * `pattern` - Route pattern (e.g., "/user/:id/posts/:postId")
    ///
    /// # Returns
    /// * `Some(path_params)` - HashMap of extracted path parameters
    /// * `None` - Pattern doesn't match
    ///
    /// # Example
    /// ```
    /// use dioxus_ipc_bridge::parser::ParsedUrl;
    ///
    /// let url = ParsedUrl::parse("ipc://user/123/posts/456").unwrap();
    /// let params = url.match_pattern("/user/:id/posts/:postId").unwrap();
    /// assert_eq!(params.get("id"), Some(&"123".to_string()));
    /// assert_eq!(params.get("postId"), Some(&"456".to_string()));
    /// ```
    pub fn match_pattern(&self, pattern: &str) -> Option<HashMap<String, String>> {
        let path_segments: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
        let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();

        // Different number of segments = no match
        if path_segments.len() != pattern_segments.len() {
            return None;
        }

        let mut path_params = HashMap::new();

        for (path_seg, pattern_seg) in path_segments.iter().zip(pattern_segments.iter()) {
            if pattern_seg.starts_with(':') {
                // Dynamic segment - extract parameter
                let param_name = &pattern_seg[1..]; // Remove ':'
                path_params.insert(param_name.to_string(), path_seg.to_string());
            } else if path_seg != pattern_seg {
                // Static segment doesn't match
                return None;
            }
        }

        Some(path_params)
    }
}

/// Parse query string into key-value pairs
///
/// # Arguments
/// * `query` - Query string without leading '?' (e.g., "number=10&operation=square")
///
/// # Returns
/// * HashMap of decoded query parameters
///
/// # Example
/// ```
/// use dioxus_ipc_bridge::parser::parse_query_string;
///
/// let params = parse_query_string("name=John%20Doe&age=25");
/// assert_eq!(params.get("name"), Some(&"John Doe".to_string()));
/// assert_eq!(params.get("age"), Some(&"25".to_string()));
/// ```
pub fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    for pair in query.split('&') {
        if let Some(pos) = pair.find('=') {
            let key = url_decode(&pair[..pos]);
            let value = url_decode(&pair[pos + 1..]);
            params.insert(key, value);
        } else if !pair.is_empty() {
            // Key without value (e.g., "flag")
            params.insert(url_decode(pair), String::new());
        }
    }

    params
}

/// Decode URL-encoded string
///
/// # Arguments
/// * `s` - URL-encoded string
///
/// # Returns
/// * Decoded string
///
/// # Example
/// ```
/// use dioxus_ipc_bridge::parser::url_decode;
///
/// assert_eq!(url_decode("Hello%20World"), "Hello World");
/// assert_eq!(url_decode("email%40example.com"), "email@example.com");
/// ```
pub fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Decode %XX hex sequence
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push(ch);
        } else if ch == '+' {
            // '+' represents space in URL encoding
            result.push(' ');
        } else {
            result.push(ch);
        }
    }

    result
}

/// Parse request body based on Content-Type
///
/// # Arguments
/// * `content_type` - Content-Type header value
/// * `body_str` - Raw body string (JSON, URL-encoded, or multipart)
///
/// # Returns
/// * `Ok(RequestBody)` - Successfully parsed body
/// * `Err(IpcError)` - Parse error
///
/// # Example
/// ```
/// use dioxus_ipc_bridge::parser::parse_body;
///
/// // JSON body
/// let body = parse_body("application/json", r#"{"name":"John","age":25}"#).unwrap();
///
/// // URL-encoded body
/// let body = parse_body("application/x-www-form-urlencoded", "name=John&age=25").unwrap();
/// ```
pub fn parse_body(content_type: &str, body_str: &str) -> Result<RequestBody, IpcError> {
    if content_type.contains("application/json") {
        // Parse JSON body
        serde_json::from_str(body_str)
            .map(RequestBody::Json)
            .map_err(|e| IpcError::ParseError(format!("Invalid JSON: {}", e)))
    } else if content_type.contains("application/x-www-form-urlencoded") {
        // Parse URL-encoded body
        Ok(RequestBody::UrlEncoded(parse_query_string(body_str)))
    } else if content_type.contains("multipart/form-data") {
        // Parse multipart body
        parse_multipart_body(content_type, body_str)
    } else {
        Err(IpcError::BadRequest(format!(
            "Unsupported Content-Type: {}",
            content_type
        )))
    }
}

/// Parse multipart/form-data body
///
/// # Arguments
/// * `content_type` - Full Content-Type header with boundary
/// * `body_str` - Raw multipart body
///
/// # Returns
/// * `Ok(RequestBody::Multipart)` - Parsed fields and files
/// * `Err(IpcError)` - Parse error
pub fn parse_multipart_body(content_type: &str, body_str: &str) -> Result<RequestBody, IpcError> {
    // Extract boundary from Content-Type
    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .ok_or_else(|| {
            IpcError::ParseError("Missing boundary in multipart Content-Type".to_string())
        })?
        .trim();

    let mut fields = HashMap::new();
    let mut files = Vec::new();

    // Split by boundary
    let parts: Vec<&str> = body_str.split(&format!("--{}", boundary)).collect();

    for part in parts.iter().skip(1) {
        // Skip empty parts and closing boundary
        if part.trim().is_empty() || part.trim() == "--" {
            continue;
        }

        // Parse part headers and content
        if let Some((headers_str, content)) = part.split_once("\r\n\r\n") {
            let headers = parse_multipart_headers(headers_str);

            if let Some(disposition) = headers.get("Content-Disposition") {
                // Extract name and filename from Content-Disposition
                let name = extract_disposition_value(disposition, "name");
                let filename = extract_disposition_value(disposition, "filename");

                let content = content.trim_end_matches("\r\n").to_string();

                if let Some(filename) = filename {
                    // File upload
                    let content_type = headers
                        .get("Content-Type")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "application/octet-stream".to_string());

                    files.push(FileUpload {
                        name: name.unwrap_or_else(|| "file".to_string()),
                        filename,
                        content_type,
                        data: content,
                    });
                } else if let Some(name) = name {
                    // Text field
                    fields.insert(name, content);
                }
            }
        }
    }

    Ok(RequestBody::Multipart { fields, files })
}

/// Parse multipart part headers
fn parse_multipart_headers(headers_str: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();

    for line in headers_str.lines() {
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }

    headers
}

/// Extract value from Content-Disposition header
fn extract_disposition_value(disposition: &str, param: &str) -> Option<String> {
    let pattern = format!("{}=", param);

    for part in disposition.split(';') {
        let part = part.trim();
        if part.starts_with(&pattern) {
            let value = part[pattern.len()..].trim();
            // Remove surrounding quotes
            return Some(value.trim_matches('"').to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let url = ParsedUrl::parse("ipc://calculator/fibonacci?number=10").unwrap();
        assert_eq!(url.scheme, "ipc");
        assert_eq!(url.path, "/calculator/fibonacci");
        assert_eq!(url.query_params.get("number"), Some(&"10".to_string()));
    }

    #[test]
    fn test_match_pattern() {
        let url = ParsedUrl::parse("ipc://user/123/posts/456").unwrap();
        let params = url.match_pattern("/user/:id/posts/:postId").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("postId"), Some(&"456".to_string()));
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("Hello%20World"), "Hello World");
        assert_eq!(url_decode("email%40example.com"), "email@example.com");
        assert_eq!(url_decode("foo+bar"), "foo bar");
    }

    #[test]
    fn test_parse_query_string() {
        let params = parse_query_string("name=John%20Doe&age=25&city=NYC");
        assert_eq!(params.get("name"), Some(&"John Doe".to_string()));
        assert_eq!(params.get("age"), Some(&"25".to_string()));
        assert_eq!(params.get("city"), Some(&"NYC".to_string()));
    }
}
