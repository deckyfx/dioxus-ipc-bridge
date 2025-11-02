//! Streaming Task System - Server-Sent Events style progress and chunked data transfer
//!
//! Provides utilities for long-running operations with progress tracking and large data transfer.
//! This module is only available when the `streaming` feature is enabled.

use crate::bridge;
use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Progress update for a streaming task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingProgress {
    pub percent: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta: Option<u64>, // Estimated time remaining in seconds
}

/// Data chunk for large transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    pub index: usize,
    pub data: String, // Base64 encoded or plain text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_chunks: Option<usize>,
}

/// Streaming task handle for emitting progress and chunks
///
/// # Example
/// ```rust,ignore
/// use dioxus_ipc_bridge::streaming::StreamingTask;
///
/// let task = StreamingTask::new();
///
/// // Emit progress updates
/// task.emit_percent(25.0);
/// task.emit_progress_message(50.0, "Processing...".to_string());
/// task.emit_progress_count(3, 10, Some("Item 3 of 10".to_string()));
///
/// // Emit data chunks
/// task.emit_chunk(0, "base64_data".to_string(), Some(5));
///
/// // Complete the task
/// task.emit_complete(serde_json::json!({"status": "success"}));
/// ```
pub struct StreamingTask {
    pub task_id: String,
}

impl StreamingTask {
    /// Create a new streaming task with a unique ID
    pub fn new() -> Self {
        Self {
            task_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create a streaming task with a specific ID
    pub fn with_id(task_id: String) -> Self {
        Self { task_id }
    }

    /// Emit progress update
    pub fn emit_progress(&self, progress: StreamingProgress) {
        let channel = format!("task:{}:progress", self.task_id);
        let data = serde_json::to_value(&progress).unwrap_or_default();
        bridge::emit(&channel, data);
    }

    /// Emit progress with percentage only
    pub fn emit_percent(&self, percent: f32) {
        self.emit_progress(StreamingProgress {
            percent,
            message: None,
            current: None,
            total: None,
            eta: None,
        });
    }

    /// Emit progress with percentage and message
    pub fn emit_progress_message(&self, percent: f32, message: String) {
        self.emit_progress(StreamingProgress {
            percent,
            message: Some(message),
            current: None,
            total: None,
            eta: None,
        });
    }

    /// Emit progress with current/total tracking
    pub fn emit_progress_count(&self, current: u64, total: u64, message: Option<String>) {
        let percent = (current as f32 / total as f32) * 100.0;
        self.emit_progress(StreamingProgress {
            percent,
            message,
            current: Some(current),
            total: Some(total),
            eta: None,
        });
    }

    /// Emit a data chunk
    pub fn emit_chunk(&self, index: usize, data: String, total_chunks: Option<usize>) {
        let channel = format!("task:{}:chunk", self.task_id);
        let chunk = StreamingChunk {
            index,
            data,
            total_chunks,
        };
        let chunk_json = serde_json::to_value(&chunk).unwrap_or_default();
        bridge::emit(&channel, chunk_json);
    }

    /// Emit chunked data from a byte slice
    pub fn emit_chunked_data(&self, data: &[u8], chunk_size: usize) {
        let total_chunks = (data.len() + chunk_size - 1) / chunk_size;

        for (index, chunk) in data.chunks(chunk_size).enumerate() {
            let encoded = base64::engine::general_purpose::STANDARD.encode(chunk);
            self.emit_chunk(index, encoded, Some(total_chunks));
        }
    }

    /// Emit chunked string data
    pub fn emit_chunked_string(&self, data: &str, chunk_size: usize) {
        let bytes = data.as_bytes();
        self.emit_chunked_data(bytes, chunk_size);
    }

    /// Emit completion with result
    pub fn emit_complete<T: Serialize>(&self, result: T) {
        let channel = format!("task:{}:complete", self.task_id);
        let data = serde_json::json!({
            "result": result
        });
        bridge::emit(&channel, data);
    }

    /// Emit completion without result (for chunk-based transfers)
    pub fn emit_complete_no_result(&self) {
        let channel = format!("task:{}:complete", self.task_id);
        let data = serde_json::json!({
            "result": null
        });
        bridge::emit(&channel, data);
    }

    /// Emit error
    pub fn emit_error(&self, message: String) {
        let channel = format!("task:{}:error", self.task_id);
        let data = serde_json::json!({
            "message": message
        });
        bridge::emit(&channel, data);
    }

    /// Get initial response for task initiation
    pub fn initial_response(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.task_id,
            "message": "Task started"
        })
    }
}

impl Default for StreamingTask {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function: Process items with progress tracking
///
/// # Example
/// ```rust,ignore
/// use dioxus_ipc_bridge::streaming::{StreamingTask, process_with_progress};
///
/// let task = StreamingTask::new();
/// let items = vec![1, 2, 3, 4, 5];
///
/// let results = process_with_progress(&task, items, |num| {
///     Ok(num * 2)
/// }).await?;
/// ```
pub async fn process_with_progress<T, F, R>(
    task: &StreamingTask,
    items: Vec<T>,
    processor: F,
) -> Result<Vec<R>, String>
where
    F: Fn(T) -> Result<R, String>,
{
    let total = items.len() as u64;
    let mut results = Vec::new();

    for (index, item) in items.into_iter().enumerate() {
        let current = (index + 1) as u64;

        // Emit progress
        task.emit_progress_count(
            current,
            total,
            Some(format!("Processing item {}/{}", current, total)),
        );

        // Process item
        match processor(item) {
            Ok(result) => results.push(result),
            Err(e) => {
                task.emit_error(format!("Failed at item {}: {}", current, e));
                return Err(e);
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_task_creation() {
        let task = StreamingTask::new();
        assert!(!task.task_id.is_empty());
    }

    #[test]
    fn test_progress_serialization() {
        let progress = StreamingProgress {
            percent: 50.0,
            message: Some("Half done".to_string()),
            current: Some(5),
            total: Some(10),
            eta: Some(30),
        };

        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["percent"], 50.0);
        assert_eq!(json["message"], "Half done");
    }

    #[test]
    fn test_chunk_serialization() {
        let chunk = StreamingChunk {
            index: 0,
            data: "test_data".to_string(),
            total_chunks: Some(5),
        };

        let json = serde_json::to_value(&chunk).unwrap();
        assert_eq!(json["index"], 0);
        assert_eq!(json["data"], "test_data");
        assert_eq!(json["total_chunks"], 5);
    }
}
