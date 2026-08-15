use crate::runtime::bridge;
use serde_json::Value;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Request {
    pub event: String,
    pub payload: Value,
}

#[derive(Clone, Debug)]
pub enum Response {
    Ok(Value),
    Err(String),
}

pub fn send(
    bridge: &bridge::Bridge,
    request: &Request,
    patience: Option<Duration>,
) -> Result<Response, String> {
    let id = id();
    let mut line = serde_json::to_string(&serde_json::json!({
        "kind": "event",
        "id": id,
        "verb": request.event,
        "payload": request.payload,
    }))
    .map_err(|err| err.to_string())?;
    line.push('\n');

    let raw = bridge.exchange(&line, patience)?;
    parse(&raw, &id)
}

#[doc(hidden)]
pub fn parse(text: &str, expected: &str) -> Result<Response, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("inspect endpoint returned empty response".to_string());
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|err| err.to_string())?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "inspect response missing kind".to_string())?;
    let id = value.get("id").and_then(Value::as_str).unwrap_or_default();
    if id != expected {
        return Err(format!(
            "inspect response id mismatch: expected {expected}, got {id}"
        ));
    }

    match kind {
        "event_response" => Ok(Response::Ok(
            value.get("payload").cloned().unwrap_or(Value::Null),
        )),
        "event_error" => {
            let error = value
                .get("error")
                .map(describe)
                .unwrap_or_else(|| "inspect endpoint returned event_error".to_string());
            Ok(Response::Err(error))
        }
        other => Err(format!(
            "expected event_response/event_error inspect frame, got {other}"
        )),
    }
}

fn describe(value: &Value) -> String {
    let code = value
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or("event_error");
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("inspect endpoint returned event_error");
    format!("{code}: {message}")
}

fn id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or(0);
    format!("{micros}-{count}")
}
