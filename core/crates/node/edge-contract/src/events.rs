//! Generic Edge-to-Core action events. Presentation is only one producer.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERSION: u32 = 1;
pub const METHOD: &str = "chatEdgeEvent";
pub const MAX_REQUEST: usize = 1024;
pub const MAX_BATCH: usize = 4;
pub const MAX_SAFE_SEQUENCE: u64 = 9_007_199_254_740_991;

fn identifier(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

/// An action is data, never an arbitrary RPC method or JavaScript export.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeActionEvent {
    pub seq: u64,
    pub action: String,
    pub data: Value,
}

/// The producer owns a bounded queue; Core acknowledges only a persisted batch.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EdgeEventBatch {
    pub v: u32,
    pub source: String,
    pub stream: String,
    pub events: Vec<EdgeActionEvent>,
    pub next: u64,
    pub lost_before: u64,
}
impl EdgeEventBatch {
    pub fn valid(&self) -> bool {
        self.v == VERSION
            && identifier(&self.source, 108)
            && identifier(&self.stream, 128)
            && !self.events.is_empty()
            && self.events.len() <= MAX_BATCH
            && self.events.windows(2).all(|e| e[0].seq < e[1].seq)
            && self.events[0].seq > self.lost_before
            && self.next <= MAX_SAFE_SEQUENCE
            && self.events.last().is_some_and(|e| e.seq == self.next)
            && self
                .events
                .iter()
                .all(|e| e.seq > 0 && identifier(&e.action, 64) && e.data.is_object())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn batch() -> EdgeEventBatch {
        serde_json::from_value(
            serde_json::json!({"v":1,"source":"sensor.environment","stream":"boot-1",
            "events":[{"seq":1,"action":"temperature.changed","data":{"celsius":23.5}},
                      {"seq":2,"action":"serial.message","data":{"text":"hello"}}],
            "next":2,"lostBefore":0}),
        )
        .unwrap()
    }
    #[test]
    fn generic_events_accept_non_display_actions_and_reject_invalid_envelopes() {
        assert!(batch().valid());
        let mut b = batch();
        b.v = 2;
        assert!(!b.valid());
        let mut b = batch();
        b.events[0].action = "arbitrary RPC".into();
        assert!(!b.valid());
        let mut b = batch();
        b.events[0].data = Value::Null;
        assert!(!b.valid());
        let mut b = batch();
        b.events[1].seq = 1;
        assert!(!b.valid());
        let mut b = batch();
        b.next = 3;
        assert!(!b.valid());
        let mut b = batch();
        b.lost_before = 1;
        assert!(!b.valid());
        let mut b = batch();
        b.stream = String::new();
        assert!(!b.valid());
        let mut b = batch();
        b.events[1].seq = MAX_SAFE_SEQUENCE + 1;
        b.next = MAX_SAFE_SEQUENCE + 1;
        assert!(!b.valid());
        let mut value = serde_json::to_value(batch()).unwrap();
        value["lease"] = Value::String("screen-specific".into());
        assert!(serde_json::from_value::<EdgeEventBatch>(value).is_err());
    }
}
