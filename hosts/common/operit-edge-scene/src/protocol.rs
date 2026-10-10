//! Version 1 of the bounded device presentation protocol, not a ToolPkg runtime.
pub const PLUGIN_ID: &str = "display.scene";
pub const VERSION: u32 = 1;
pub const ACTIONS: &[&str] = &[
    "capabilities",
    "lease.open",
    "lease.renew",
    "lease.close",
    "pack.begin",
    "pack.chunk",
    "pack.commit",
    "pack.abort",
    "pack.remove",
    "scene.set",
    "events.poll",
];
pub const MAX_REQUEST: usize = 1024;
pub const MAX_REPLY: usize = 4096;
pub const MAX_PACK: usize = 4096;
pub const MAX_CACHE: usize = 8192;
pub const MAX_PACKS: usize = 2;
pub const MAX_ASSETS: usize = 4;
pub const MAX_LAYERS: usize = 4;
pub const MAX_CHUNK: usize = 256;
pub const MAX_EVENTS: usize = 16;

/// Push batches share the same bounded Edge entry, not an arbitrary RPC address.
pub const MAX_PUSH_EVENTS: usize = 4;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneEvent {
    pub seq: u64,
    pub r#type: String,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub target: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SceneEventBatch {
    pub v: u32,
    pub lease: String,
    pub events: Vec<SceneEvent>,
    pub next: u64,
    pub lost_before: u64,
    pub closed: Option<String>,
}
impl SceneEventBatch {
    pub fn valid(&self) -> bool {
        let closed = |s: &str| matches!(s, "system.exit" | "lease.closed" | "lease.expired");
        self.v == VERSION
            && self.lease.len() == 36
            && self
                .lease
                .bytes()
                .all(|c| c.is_ascii_hexdigit() || c == b'-')
            && !self.events.is_empty()
            && self.events.len() <= MAX_PUSH_EVENTS
            && self.events.windows(2).all(|e| e[0].seq < e[1].seq)
            && self.events[0].seq > self.lost_before
            && self.next <= 9_007_199_254_740_991
            && self.events.last().is_some_and(|e| e.seq == self.next)
            && self.closed.as_deref().is_none_or(|reason| {
                closed(reason) && self.events.last().is_some_and(|e| e.r#type == reason)
            })
            && self.events.iter().all(|e| {
                e.seq > 0
                    && if e.r#type == "target.tap" {
                        e.x.is_some()
                            && e.y.is_some()
                            && e.target.as_deref().is_some_and(|s| {
                                !s.is_empty()
                                    && s.len() <= 24
                                    && s.bytes()
                                        .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
                            })
                    } else {
                        closed(&e.r#type) && e.x.is_none() && e.y.is_none() && e.target.is_none()
                    }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn batch() -> SceneEventBatch {
        SceneEventBatch {
            v: 1,
            lease: "12345678-1234-1234-1234-123456789abc".into(),
            next: 1,
            lost_before: 0,
            closed: None,
            events: vec![SceneEvent {
                seq: 1,
                r#type: "target.tap".into(),
                x: Some(1),
                y: Some(2),
                target: Some("pet".into()),
            }],
        }
    }
    #[test]
    fn push_batch_rejects_invalid_version_order_types_close_and_unsafe_sequences() {
        let good = batch();
        assert!(good.valid());
        let mut b = good.clone();
        b.v = 2;
        assert!(!b.valid());
        let mut b = good.clone();
        b.events.clear();
        assert!(!b.valid());
        let mut b = good.clone();
        b.events.push(b.events[0].clone());
        assert!(!b.valid());
        let mut b = good.clone();
        b.events[0].r#type = "execute_tool".into();
        assert!(!b.valid());
        let mut b = good.clone();
        b.closed = Some("system.exit".into());
        assert!(!b.valid());
        let mut b = good.clone();
        b.events[0].target = Some("../secret".into());
        assert!(!b.valid());
        let mut b = good.clone();
        b.events[0].seq = 9_007_199_254_740_992;
        b.next = b.events[0].seq;
        assert!(!b.valid());
        let mut b = good.clone();
        b.lost_before = 1;
        assert!(!b.valid());
        let mut b = good.clone();
        b.events[0] = SceneEvent {
            seq: 1,
            r#type: "system.exit".into(),
            x: None,
            y: None,
            target: None,
        };
        b.closed = Some("system.exit".into());
        assert!(b.valid());
    }
}
