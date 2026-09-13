//! WebSocket event hub and `/api/v1/events` handler.
//!
//! A single broadcast channel fans events out to every connected client. Event
//! JSON shapes match the legacy `http_server.py` broadcast helpers.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use serde_json::{json, Value};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct EventHub {
    tx: broadcast::Sender<Event>,
}

#[derive(Clone, Debug)]
pub struct Event(pub Value);

impl Event {
    pub fn task(goal_id: &str, event_type: &str, data: Value) -> Self {
        let mut map = serde_json::Map::new();
        map.insert("goal_id".into(), Value::String(goal_id.to_string()));
        map.insert("event".into(), Value::String(event_type.to_string()));
        if let Value::Object(obj) = data {
            for (k, v) in obj {
                map.insert(k, v);
            }
        }
        Self(Value::Object(map))
    }

    pub fn diagnosis(payload: Value) -> Self {
        Self(Self::tagged(payload, "diagnosis"))
    }

    pub fn vitals(payload: Value) -> Self {
        Self(Self::tagged(payload, "vitals"))
    }

    fn tagged(payload: Value, tag: &str) -> Value {
        let mut map = match payload {
            Value::Object(obj) => obj,
            other => {
                let mut m = serde_json::Map::new();
                m.insert("data".into(), other);
                m
            }
        };
        map.insert("event".into(), Value::String(tag.to_string()));
        Value::Object(map)
    }
}

impl EventHub {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    pub fn broadcast(&self, event: Event) {
        // No receivers is not an error.
        let _ = self.tx.send(event);
    }
}

pub async fn ws_handler(
    State(state): State<crate::app::AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, headers))
}

async fn handle_socket(mut socket: WebSocket, state: crate::app::AppState, _headers: HeaderMap) {
    let mut rx = state.hub.subscribe();

    if !state.config.api_token.is_empty() {
        match socket.recv().await {
            Some(Ok(Message::Text(text))) if text.as_str() == state.config.api_token => {}
            _ => {
                let body = json!({"error": {"code": "UNAUTHORIZED", "message": "auth failed"}});
                let _ = socket.send(Message::Text(body.to_string().into())).await;
                return;
            }
        }
    }

    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Ok(event) => {
                    if socket.send(Message::Text(event.0.to_string().into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
            },
        }
    }
}
