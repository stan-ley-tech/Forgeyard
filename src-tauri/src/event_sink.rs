use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

/// Abstracts "emit an event to the frontend" behind a trait so modules
/// that emit events (the process manager, the automation engine) don't
/// need to be generic over Tauri's `Runtime`, or touch any part of the
/// real webview stack, just to be unit-testable — tests plug in `NoopSink`
/// instead of standing up a `tauri::AppHandle`.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

impl<R: Runtime> EventSink for AppHandle<R> {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = Emitter::emit(self, event, payload);
    }
}

pub fn to_json(value: &impl Serialize) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
pub struct NoopSink;

#[cfg(test)]
impl EventSink for NoopSink {
    fn emit(&self, _event: &str, _payload: serde_json::Value) {}
}
