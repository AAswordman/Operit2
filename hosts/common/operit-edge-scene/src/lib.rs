//! Bounded, device-owned presentation. No scripts, filesystem paths or business state.
use base64::{engine::general_purpose::STANDARD, Engine};
use operit_node_edge::{EdgePlugin, EdgePluginManifest, EdgeServiceError};
pub mod protocol;
use operit_link::CoreValue;
use protocol::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};

type Fault = (&'static str, &'static str);
type Result<T> = std::result::Result<T, Fault>;
fn invalid() -> Fault {
    ("SCENE_ARGS", "invalid or missing arguments")
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 24
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub asset: String,
    pub x: i16,
    pub y: i16,
    #[serde(default = "one")]
    pub scale: u8,
    #[serde(default = "anchor")]
    pub anchor: String,
    #[serde(default)]
    pub frame: u8,
    #[serde(default)]
    pub play: bool,
    #[serde(default = "yes")]
    pub r#loop: bool,
    #[serde(default)]
    pub target: Option<String>,
}
fn one() -> u8 {
    1
}
fn yes() -> bool {
    true
}
fn anchor() -> String {
    "tl".into()
}
#[derive(Clone)]
struct Asset {
    width: u8,
    height: u8,
    frames: u8,
    ms: u16,
    palette: Vec<(u16, u8)>,
    pixels: usize,
}
struct Pack {
    bytes: Vec<u8>,
    sha: String,
    assets: BTreeMap<String, Asset>,
}
struct Upload {
    handle: String,
    id: String,
    bytes: usize,
    sha: String,
    data: Vec<u8>,
}
/// Captured at lease.open; a reconnect may replace the entry, never the Space/chat.
pub use operit_node_edge::events::EdgeEventRoute as SceneEventRoute;
pub type SceneRouteProvider = operit_node_edge::events::EdgeEventRouteProvider;
#[derive(Clone)]
pub struct SceneDelivery {
    pub route: SceneEventRoute,
    pub batch: SceneEventBatch,
}
struct Lease {
    handle: String,
    rect: Rect,
    deadline: u64,
    ttl: u64,
    closed: Option<&'static str>,
    route: Option<SceneEventRoute>,
    acknowledged: u64,
}
struct State {
    changed: Arc<tokio::sync::Notify>,
    lease: Option<Lease>,
    packs: BTreeMap<String, Pack>,
    upload: Option<Upload>,
    layers: Vec<Layer>,
    background: u16,
    started: u64,
    events: VecDeque<Value>,
    sequence: u64,
    revision: u32,
}
impl State {
    fn event(&mut self, kind: &str, x: Option<u16>, y: Option<u16>, target: Option<&str>) {
        self.changed.notify_one();
        self.sequence += 1;
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events
            .push_back(json!({"seq":self.sequence,"type":kind,"x":x,"y":y,"target":target}));
    }
    fn close(&mut self, reason: &'static str) {
        if let Some(l) = &mut self.lease {
            l.closed = Some(reason);
        }
        self.layers.clear();
        self.upload = None;
        self.revision = self.revision.wrapping_add(1);
        self.event(reason, None, None, None);
    }
    fn expire(&mut self, now: u64) {
        if self
            .lease
            .as_ref()
            .is_some_and(|l| l.closed.is_none() && now >= l.deadline)
        {
            self.close("lease.expired");
        }
    }
    fn active(&self) -> bool {
        self.lease.as_ref().is_some_and(|l| l.closed.is_none())
    }
    fn check(&self, a: &Value, allow_closed: bool) -> Result<()> {
        let l = self
            .lease
            .as_ref()
            .ok_or(("SCENE_LEASE", "no display lease"))?;
        if a["lease"].as_str() != Some(&l.handle) {
            return Err(("SCENE_LEASE", "wrong display lease"));
        }
        if !allow_closed {
            if let Some(reason) = l.closed {
                return Err((
                    if reason == "lease.expired" {
                        "SCENE_EXPIRED"
                    } else {
                        "SCENE_CLOSED"
                    },
                    "display lease is closed",
                ));
            }
        }
        Ok(())
    }
    fn resource(&self, name: &str) -> Result<(&Pack, &Asset)> {
        let (p, a) = name
            .split_once(':')
            .ok_or(("SCENE_RESOURCE", "expected packId:assetId"))?;
        let pack = self
            .packs
            .get(p)
            .ok_or(("SCENE_RESOURCE", "pack not cached"))?;
        Ok((
            pack,
            pack.assets
                .get(a)
                .ok_or(("SCENE_RESOURCE", "asset not cached"))?,
        ))
    }
    fn in_use(&self, p: &str) -> bool {
        self.layers
            .iter()
            .any(|l| l.asset.split_once(':').is_some_and(|(pack, _)| pack == p))
    }
}
/// Owns at most 8 KiB committed asset bytes and one 4 KiB upload. Rendering uses
/// the existing RGB565 two-row strip, never a framebuffer or decoded image copy.
pub struct ScenePlugin {
    changed: Arc<tokio::sync::Notify>,
    route_provider: Mutex<Option<Arc<SceneRouteProvider>>>,
    state: Mutex<State>,
    clock: Instant,
    width: u16,
    height: u16,
    reserved_top: u16,
    touch: bool,
}
impl ScenePlugin {
    /// The device application owns its display dimensions and system UI area.
    /// A device without a top system bar may pass zero for reserved_top.
    pub fn new(width: u16, height: u16, reserved_top: u16, touch: bool) -> Self {
        assert!(width > 0 && height > reserved_top);
        let changed = Arc::new(tokio::sync::Notify::new());
        Self {
            changed: changed.clone(),
            route_provider: Mutex::new(None),
            state: Mutex::new(State {
                changed,
                lease: None,
                packs: BTreeMap::new(),
                upload: None,
                layers: vec![],
                background: 0,
                started: 0,
                events: VecDeque::new(),
                sequence: 0,
                revision: 0,
            }),
            clock: Instant::now(),
            width,
            height,
            reserved_top,
            touch,
        }
    }
    pub fn set_route_provider(&self, provider: Arc<SceneRouteProvider>) {
        *self.route_provider.lock().unwrap() = Some(provider);
    }
    pub async fn wait_event(&self) {
        self.changed.notified().await;
    }
    /// Snapshot only; no lock is held during Link I/O. At most one worker per device.
    pub fn pending_delivery(&self) -> Option<SceneDelivery> {
        let s = self.state.lock().unwrap();
        let l = s.lease.as_ref()?;
        let route = l.route.clone()?;
        let events = s
            .events
            .iter()
            .filter(|e| e["seq"].as_u64().unwrap() > l.acknowledged)
            .take(MAX_PUSH_EVENTS)
            .cloned()
            .map(serde_json::from_value)
            .collect::<std::result::Result<Vec<SceneEvent>, _>>()
            .ok()?;
        let next = events.last()?.seq;
        Some(SceneDelivery {
            route,
            batch: SceneEventBatch {
                v: VERSION,
                lease: l.handle.clone(),
                events,
                next,
                lost_before: s.events.front()?.get("seq")?.as_u64()?.saturating_sub(1),
                closed: if next == s.sequence {
                    l.closed.map(str::to_string)
                } else {
                    None
                },
            },
        })
    }
    pub fn disable_delivery(&self, delivery: &SceneDelivery) {
        let mut s = self.state.lock().unwrap();
        if let Some(l) = &mut s.lease {
            if l.handle == delivery.batch.lease && l.route.as_ref() == Some(&delivery.route) {
                l.route = None;
            }
        }
    }
    /// ACK only the exact lease/receiver and a sequence actually sent.
    pub fn acknowledge(&self, delivery: &SceneDelivery, next: u64) -> bool {
        let mut s = self.state.lock().unwrap();
        let Some(l) = s.lease.as_mut() else {
            return false;
        };
        if l.handle != delivery.batch.lease
            || l.route.as_ref() != Some(&delivery.route)
            || next != delivery.batch.next
            || next < l.acknowledged
        {
            return false;
        }
        l.acknowledged = next;
        true
    }
    fn now(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }
    pub fn summary(&self) -> Value {
        let mut s = self.state.lock().unwrap();
        s.expire(self.now());
        json!({"active":s.active(),"revision":s.revision})
    }
    /// Local simulator adapter only. Never exposed through Tools.Edge or containing lease handles.
    pub fn visual_snapshot(&self) -> Value {
        let mut s = self.state.lock().unwrap();
        s.expire(self.now());
        json!({"active":s.active(),"revision":s.revision,"rect":s.lease.as_ref().map(|l|l.rect),"background":s.background,
            "elapsed":self.now().saturating_sub(s.started),"layers":s.layers,"packs":s.packs.iter().map(|(id,p)|json!({"id":id,"data":STANDARD.encode(&p.bytes)})).collect::<Vec<_>>()})
    }
    pub fn system_exit(&self) {
        let mut s = self.state.lock().unwrap();
        s.expire(self.now());
        if s.active() {
            s.close("system.exit");
        }
    }
    pub fn touch(&self, x: u16, y: u16) {
        if !self.touch {
            return;
        }
        let now = self.now();
        let mut s = self.state.lock().unwrap();
        s.expire(now);
        if !s.active() {
            return;
        }
        let r = s.lease.as_ref().unwrap().rect;
        if x < r.x || y < r.y || x >= r.x + r.w || y >= r.y + r.h {
            return;
        }
        let (x, y) = (x - r.x, y - r.y);
        let target = s.layers.iter().rev().find_map(|l| {
            let target = l.target.as_ref()?;
            let (p, a) = s.resource(&l.asset).ok()?;
            sample(p, a, l, x as i32, y as i32, now.saturating_sub(s.started))
                .filter(|(_, alpha)| *alpha > 0)
                .map(|_| target.clone())
        });
        if let Some(target) = target {
            s.event("target.tap", Some(x), Some(y), Some(&target));
        }
    }
    /// tick is frozen at frame start by the C renderer. All pixels blend in RGB565.
    pub fn paint_strip(&self, bytes: &mut [u8], y: u16, rows: u16, tick: u32) {
        let mut s = self.state.lock().unwrap();
        s.expire(self.now());
        if !s.active() || bytes.len() != self.width as usize * rows as usize * 2 {
            return;
        }
        let r = s.lease.as_ref().unwrap().rect;
        let elapsed = tick.wrapping_sub(s.started as u32) as u64;
        for row in 0..rows {
            let sy = y + row;
            if sy < r.y || sy >= r.y + r.h {
                continue;
            }
            for x in r.x..r.x + r.w {
                let mut pixel = s.background;
                for l in &s.layers {
                    if let Ok((p, a)) = s.resource(&l.asset) {
                        if let Some((color, alpha)) =
                            sample(p, a, l, (x - r.x) as i32, (sy - r.y) as i32, elapsed)
                        {
                            pixel = blend(pixel, color, alpha);
                        }
                    }
                }
                let i = ((row as usize * self.width as usize) + x as usize) * 2;
                bytes[i..i + 2].copy_from_slice(&pixel.to_le_bytes());
            }
        }
    }
    pub fn tick(&self) -> u32 {
        self.now() as u32
    }
    fn dispatch(&self, action: &str, a: Value, now: u64) -> Result<Value> {
        if !a.is_object() || serde_json::to_vec(&a).map_err(|_| invalid())?.len() > MAX_REQUEST {
            return Err(("SCENE_LIMIT", "request exceeds 1024 bytes"));
        }
        if a["v"].as_u64() != Some(VERSION as u64) {
            return Err(("SCENE_VERSION", "expected protocol v:1"));
        }
        let fields: &[&str] = match action {
            "capabilities" => &[],
            "lease.open" => &["owner", "rect", "ttlMs", "notify"],
            "lease.renew" => &["lease"],
            "lease.close" => &["lease"],
            "pack.begin" => &["lease", "packId", "bytes", "sha256"],
            "pack.chunk" => &["lease", "upload", "offset", "data"],
            "pack.commit" | "pack.abort" => &["lease", "upload"],
            "pack.remove" => &["lease", "packId"],
            "scene.set" => &["lease", "background", "layers"],
            "events.poll" => &["lease", "after"],
            _ => return Err(invalid()),
        };
        if a.as_object()
            .unwrap()
            .keys()
            .any(|k| k != "v" && !fields.contains(&k.as_str()))
        {
            return Err(invalid());
        }
        let mut s = self.state.lock().unwrap();
        s.expire(now);
        if action == "capabilities" {
            return Ok(
                json!({"protocol":VERSION,"screen":{"width":self.width,"height":self.height,"format":"RGB565_LE"},
            "events":{"push":self.route_provider.lock().unwrap().is_some(),"method":"chatEdgeEvent","maxBatch":MAX_PUSH_EVENTS,"ack":true},
            "reserved":{"x":0,"y":0,"w":self.width,"h":self.reserved_top},"touch":self.touch,"animation":{"local":true,"minFrameMs":50,"maxFrameMs":2000,"maxFrames":8},
            "formats":{"pack":"ESP1","asset":"ESI1","alpha":"straight","paletteMax":16,"maxWidth":96,"maxHeight":96},
            "limits":{"requestBytes":MAX_REQUEST,"replyBytes":MAX_REPLY,"packBytes":MAX_PACK,"cacheBytes":MAX_CACHE,"packs":MAX_PACKS,"assetsPerPack":MAX_ASSETS,"layers":MAX_LAYERS,"chunkBytes":MAX_CHUNK,"scaleMax":8,"events":MAX_EVENTS,"pollPage":8,"uploadBytes":MAX_PACK},
            "cache":{"persistent":false,"usedBytes":s.packs.values().map(|p|p.bytes.len()).sum::<usize>(),"packs":s.packs.iter().map(|(id,p)|json!({"id":id,"bytes":p.bytes.len(),"sha256":p.sha,"assets":p.assets.keys().collect::<Vec<_>>()})).collect::<Vec<_>>()}}),
            );
        }
        if action == "lease.open" {
            if s.active() {
                return Err(("SCENE_BUSY", "display region already leased"));
            }
            if !a["owner"].as_str().is_some_and(id) {
                return Err(invalid());
            }
            let rect: Rect = match a.get("rect") {
                Some(r) => serde_json::from_value(r.clone()).map_err(|_| invalid())?,
                None => Rect {
                    x: 0,
                    y: self.reserved_top,
                    w: self.width,
                    h: self.height - self.reserved_top,
                },
            };
            if rect.y < self.reserved_top
                || rect.w == 0
                || rect.h == 0
                || rect.x as u32 + rect.w as u32 > self.width as u32
                || rect.y as u32 + rect.h as u32 > self.height as u32
            {
                return Err((
                    "SCENE_LIMIT",
                    "region intersects system exit or screen bounds",
                ));
            }
            let ttl = match a.get("ttlMs") {
                None => 30000,
                Some(n) => n.as_u64().ok_or_else(invalid)?,
            };
            if !(1000..=60000).contains(&ttl) {
                return Err(("SCENE_LIMIT", "ttlMs must be 1000..60000"));
            }
            let route = match a.get("notify") {
                None => None,
                Some(notify) => {
                    let package = notify["packageName"].as_str().ok_or_else(invalid)?;
                    if notify.as_object().is_none_or(|m| m.len() != 1)
                        || package.is_empty()
                        || package.len() > 108
                        || !package
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
                    {
                        return Err(invalid());
                    }
                    Some(
                        self.route_provider
                            .lock()
                            .unwrap()
                            .as_ref()
                            .and_then(|p| p(package))
                            .ok_or((
                                "SCENE_NOTIFY",
                                "no authorized Space chat for event delivery",
                            ))?,
                    )
                }
            };
            let handle = uuid::Uuid::new_v4().to_string();
            s.lease = Some(Lease {
                handle: handle.clone(),
                rect,
                deadline: now + ttl,
                ttl,
                closed: None,
                route,
                acknowledged: s.sequence,
            });
            s.events.clear();
            s.background = 0;
            s.started = now;
            s.revision = s.revision.wrapping_add(1);
            return Ok(json!({"lease":handle,"rect":rect,"ttlMs":ttl,"cursor":s.sequence}));
        }
        s.check(&a, action == "events.poll" || action == "lease.close")?;
        match action {
            "lease.renew" => {
                let l = s.lease.as_mut().unwrap();
                l.deadline = now + l.ttl;
                Ok(json!({"ttlMs":l.ttl}))
            }
            "lease.close" => {
                if s.active() {
                    s.close("lease.closed");
                }
                Ok(json!({}))
            }
            "pack.begin" => {
                if s.upload.is_some() {
                    return Err(("SCENE_BUSY", "upload already pending"));
                }
                let name = a["packId"].as_str().filter(|x| id(x)).ok_or_else(invalid)?;
                let size = usize::try_from(a["bytes"].as_u64().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?;
                let sha = a["sha256"]
                    .as_str()
                    .filter(|s| {
                        s.len() == 64
                            && s.bytes()
                                .all(|b| b.is_ascii_digit() || b"abcdef".contains(&b))
                    })
                    .ok_or_else(invalid)?;
                if size < 5 || size > MAX_PACK {
                    return Err(("SCENE_LIMIT", "pack must be 5..4096 bytes"));
                }
                if s.in_use(name) {
                    return Err(("SCENE_IN_USE", "pack is displayed"));
                }
                let used = s
                    .packs
                    .iter()
                    .filter(|(k, _)| k.as_str() != name)
                    .map(|(_, p)| p.bytes.len())
                    .sum::<usize>();
                if used + size > MAX_CACHE
                    || (!s.packs.contains_key(name) && s.packs.len() >= MAX_PACKS)
                {
                    return Err((
                        "SCENE_LIMIT",
                        "cache full; explicitly remove an unused pack",
                    ));
                }
                let handle = uuid::Uuid::new_v4().to_string();
                s.upload = Some(Upload {
                    handle: handle.clone(),
                    id: name.into(),
                    bytes: size,
                    sha: sha.into(),
                    data: Vec::with_capacity(size),
                });
                Ok(json!({"upload":handle,"chunkBytes":MAX_CHUNK}))
            }
            "pack.chunk" | "pack.commit" | "pack.abort" => {
                let u = s
                    .upload
                    .as_ref()
                    .ok_or(("SCENE_RESOURCE", "no pending upload"))?;
                if a["upload"].as_str() != Some(&u.handle) {
                    return Err(("SCENE_RESOURCE", "wrong upload handle"));
                }
                if action == "pack.abort" {
                    s.upload = None;
                    return Ok(json!({}));
                }
                if action == "pack.chunk" {
                    let text = a["data"].as_str().ok_or_else(invalid)?;
                    if text.len() > 344 {
                        return Err(("SCENE_LIMIT", "chunk too large"));
                    }
                    let data = STANDARD.decode(text).map_err(|_| invalid())?;
                    if data.is_empty() || data.len() > MAX_CHUNK {
                        return Err(("SCENE_LIMIT", "chunk must be 1..256 bytes"));
                    }
                    let offset = usize::try_from(a["offset"].as_u64().ok_or_else(invalid)?)
                        .map_err(|_| invalid())?;
                    let u = s.upload.as_mut().unwrap();
                    if offset < u.data.len()
                        && offset
                            .checked_add(data.len())
                            .is_some_and(|end| end <= u.data.len())
                        && u.data[offset..offset + data.len()] == data
                    {
                        return Ok(json!({"offset":u.data.len()}));
                    }
                    if offset != u.data.len() || data.len() > u.bytes - u.data.len() {
                        return Err(("SCENE_OFFSET", "expected next byte offset"));
                    }
                    u.data.extend_from_slice(&data);
                    return Ok(json!({"offset":u.data.len()}));
                }
                if u.data.len() != u.bytes {
                    return Err(("SCENE_OFFSET", "upload incomplete"));
                }
                let digest = ring::digest::digest(&ring::digest::SHA256, &u.data);
                let hash = digest
                    .as_ref()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                if hash != u.sha {
                    return Err(("SCENE_HASH", "SHA-256 mismatch; abort upload"));
                }
                let assets = parse_pack(&u.data)?;
                if s.in_use(&u.id) {
                    return Err(("SCENE_IN_USE", "pack became displayed"));
                }
                let u = s.upload.take().unwrap();
                let name = u.id.clone();
                s.packs.insert(
                    u.id,
                    Pack {
                        bytes: u.data,
                        sha: u.sha,
                        assets,
                    },
                );
                s.revision = s.revision.wrapping_add(1);
                Ok(json!({"packId":name}))
            }
            "pack.remove" => {
                let name = a["packId"].as_str().filter(|s| id(s)).ok_or_else(invalid)?;
                if s.in_use(name) {
                    return Err(("SCENE_IN_USE", "pack is displayed"));
                }
                if s.packs.remove(name).is_none() {
                    return Err(("SCENE_RESOURCE", "pack not cached"));
                }
                s.revision = s.revision.wrapping_add(1);
                Ok(json!({}))
            }
            "scene.set" => {
                let layers = a["layers"].as_array().ok_or_else(invalid)?;
                if layers.len() > MAX_LAYERS {
                    return Err(("SCENE_LIMIT", "too many layers"));
                }
                let layers: Vec<Layer> =
                    serde_json::from_value(a["layers"].clone()).map_err(|_| invalid())?;
                let bg = match a.get("background") {
                    None => 0,
                    Some(v) => v.as_u64().filter(|x| *x <= 65535).ok_or_else(invalid)? as u16,
                };
                for l in &layers {
                    let (_, asset) = s.resource(&l.asset)?;
                    if !(1..=8).contains(&l.scale)
                        || !["tl", "center", "bc"].contains(&l.anchor.as_str())
                        || l.frame >= asset.frames
                        || l.target.as_ref().is_some_and(|t| !id(t))
                    {
                        return Err(invalid());
                    }
                }
                if s.layers != layers || s.background != bg {
                    s.layers = layers;
                    s.background = bg;
                    s.started = now;
                    s.revision = s.revision.wrapping_add(1);
                }
                Ok(json!({"revision":s.revision}))
            }
            "events.poll" => {
                let after = match a.get("after") {
                    None => 0,
                    Some(n) => n.as_u64().ok_or_else(invalid)?,
                };
                if after > s.sequence {
                    return Err(invalid());
                }
                let events = s
                    .events
                    .iter()
                    .filter(|e| e["seq"].as_u64().unwrap() > after)
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>();
                let next = events
                    .last()
                    .and_then(|e| e["seq"].as_u64())
                    .unwrap_or(after);
                Ok(
                    json!({"events":events,"next":next,"lostBefore":s.events.front().map(|e|e["seq"].as_u64().unwrap().saturating_sub(1)).unwrap_or(s.sequence),"closed":s.lease.as_ref().unwrap().closed}),
                )
            }
            _ => Err(invalid()),
        }
    }
}
impl operit_node_edge::events::EdgeEventSource for ScenePlugin {
    fn set_event_route_provider(
        &self,
        provider: Arc<operit_node_edge::events::EdgeEventRouteProvider>,
    ) {
        self.set_route_provider(provider);
    }
    fn pending_event_delivery(&self) -> Option<operit_node_edge::events::EdgeEventDelivery> {
        let delivery = self.pending_delivery()?;
        Some(operit_node_edge::events::EdgeEventDelivery {
            route: delivery.route,
            batch: operit_edge_contract::events::EdgeEventBatch {
                v: VERSION,
                source: PLUGIN_ID.into(),
                stream: delivery.batch.lease,
                next: delivery.batch.next,
                lost_before: delivery.batch.lost_before,
                events: delivery
                    .batch
                    .events
                    .into_iter()
                    .map(|e| {
                        let data = if e.r#type == "target.tap" {
                            json!({"x":e.x,"y":e.y,"target":e.target})
                        } else {
                            json!({})
                        };
                        operit_edge_contract::events::EdgeActionEvent {
                            seq: e.seq,
                            action: e.r#type,
                            data,
                        }
                    })
                    .collect(),
            },
        })
    }
    fn disable_event_delivery(&self, delivery: &operit_node_edge::events::EdgeEventDelivery) {
        let mut s = self.state.lock().unwrap();
        if let Some(l) = &mut s.lease {
            if l.handle == delivery.batch.stream && l.route.as_ref() == Some(&delivery.route) {
                l.route = None;
            }
        }
    }
    fn acknowledge_event_delivery(
        &self,
        delivery: &operit_node_edge::events::EdgeEventDelivery,
        next: u64,
    ) -> bool {
        let mut s = self.state.lock().unwrap();
        let Some(l) = s.lease.as_mut() else {
            return false;
        };
        if delivery.batch.source != PLUGIN_ID
            || l.handle != delivery.batch.stream
            || l.route.as_ref() != Some(&delivery.route)
            || next != delivery.batch.next
            || next < l.acknowledged
        {
            return false;
        }
        l.acknowledged = next;
        true
    }
    fn wait_for_event(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(self.wait_event())
    }
}

impl EdgePlugin for ScenePlugin {
    fn manifest(&self) -> EdgePluginManifest {
        EdgePluginManifest {
            id: PLUGIN_ID.into(),
            name: "Scene display v1".into(),
            actions: ACTIONS.iter().map(|s| s.to_string()).collect(),
        }
    }
    fn invoke(
        &self,
        action: &str,
        args: CoreValue,
    ) -> std::result::Result<CoreValue, EdgeServiceError> {
        let args = operit_link::fromCoreValue::<Value>(args)
            .map_err(|e| EdgeServiceError::new(e.to_string()))?;
        let out = match self.dispatch(action, args, self.now()) {
            Ok(result) => json!({"v":VERSION,"ok":true,"result":result}),
            Err((code, message)) => {
                json!({"v":VERSION,"ok":false,"error":{"code":code,"message":message}})
            }
        };
        if serde_json::to_vec(&out).unwrap().len() > MAX_REPLY {
            return Err(EdgeServiceError::new(
                "scene reply exceeds bounded capacity",
            ));
        }
        operit_link::toCoreValue(out).map_err(|e| EdgeServiceError::new(e.to_string()))
    }
}
fn parse_pack(data: &[u8]) -> Result<BTreeMap<String, Asset>> {
    let format = ("SCENE_FORMAT", "invalid ESP1/ESI1 package");
    let mut at = 0;
    fn take<'a>(data: &'a [u8], at: &mut usize, n: usize) -> Option<&'a [u8]> {
        let end = at.checked_add(n)?;
        let r = data.get(*at..end)?;
        *at = end;
        Some(r)
    }
    if take(data, &mut at, 4) != Some(&b"ESP1"[..]) {
        return Err(format);
    }
    let count = take(data, &mut at, 1).ok_or(format)?[0] as usize;
    if !(1..=MAX_ASSETS).contains(&count) {
        return Err(format);
    }
    let mut assets = BTreeMap::new();
    for _ in 0..count {
        let n = take(data, &mut at, 1).ok_or(format)?[0] as usize;
        let name =
            std::str::from_utf8(take(data, &mut at, n).ok_or(format)?).map_err(|_| format)?;
        if !id(name) || assets.contains_key(name) {
            return Err(format);
        }
        let size =
            u16::from_le_bytes(take(data, &mut at, 2).ok_or(format)?.try_into().unwrap()) as usize;
        let start = at;
        let asset = take(data, &mut at, size).ok_or(format)?;
        if asset.len() < 10 || &asset[..4] != b"ESI1" {
            return Err(format);
        }
        let (w, h, f, p) = (asset[4], asset[5], asset[6], asset[7]);
        let ms = u16::from_le_bytes([asset[8], asset[9]]);
        if w == 0
            || w > 96
            || h == 0
            || h > 96
            || f == 0
            || f > 8
            || p == 0
            || p > 16
            || !(50..=2000).contains(&ms)
        {
            return Err(format);
        }
        let pixels = 10 + p as usize * 3;
        let len = w as usize * h as usize * f as usize;
        if asset.len() != pixels + len || asset[pixels..].iter().any(|i| *i >= p) {
            return Err(format);
        }
        let palette = asset[10..pixels]
            .chunks_exact(3)
            .map(|c| (u16::from_le_bytes([c[0], c[1]]), c[2]))
            .collect();
        assets.insert(
            name.to_string(),
            Asset {
                width: w,
                height: h,
                frames: f,
                ms,
                palette,
                pixels: start + pixels,
            },
        );
    }
    if at != data.len() {
        return Err(format);
    }
    Ok(assets)
}
fn sample(pack: &Pack, a: &Asset, l: &Layer, x: i32, y: i32, elapsed: u64) -> Option<(u16, u8)> {
    let w = a.width as i32 * l.scale as i32;
    let h = a.height as i32 * l.scale as i32;
    let (left, top) = match l.anchor.as_str() {
        "center" => (l.x as i32 - w / 2, l.y as i32 - h / 2),
        "bc" => (l.x as i32 - w / 2, l.y as i32 - h),
        _ => (l.x as i32, l.y as i32),
    };
    let (x, y) = (x - left, y - top);
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    let frame = if !l.play {
        l.frame as u64
    } else {
        let f = l.frame as u64 + elapsed / a.ms as u64;
        if l.r#loop {
            f % a.frames as u64
        } else {
            f.min(a.frames as u64 - 1)
        }
    };
    let offset = a.pixels
        + frame as usize * a.width as usize * a.height as usize
        + (y / l.scale as i32) as usize * a.width as usize
        + (x / l.scale as i32) as usize;
    Some(a.palette[pack.bytes[offset] as usize])
}
fn blend(bg: u16, fg: u16, a: u8) -> u16 {
    let a = a as u32;
    let mix = |b: u16, f: u16| ((f as u32 * a + b as u32 * (255 - a) + 127) / 255) as u16;
    (mix((bg >> 11) & 31, (fg >> 11) & 31) << 11)
        | (mix((bg >> 5) & 63, (fg >> 5) & 63) << 5)
        | mix(bg & 31, fg & 31)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn bytes() -> Vec<u8> {
        // 2x1, two frames, transparent / red / green / half-blue.
        let mut asset = b"ESI1".to_vec();
        asset.extend([2, 1, 2, 4, 50, 0]);
        asset.extend([0, 0, 0, 0, 248, 255, 224, 7, 255, 31, 0, 128]);
        asset.extend([1, 0, 2, 3]);
        let mut pack = b"ESP1\x01\x03pet".to_vec();
        pack.extend((asset.len() as u16).to_le_bytes());
        pack.extend(asset);
        pack
    }
    fn hash(data: &[u8]) -> String {
        ring::digest::digest(&ring::digest::SHA256, data)
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    fn push_plugin() -> ScenePlugin {
        let p = ScenePlugin::new(320, 240, 24, true);
        p.set_route_provider(Arc::new(|package| {
            Some(SceneEventRoute {
                space_id: "space-a".into(),
                chat_id: "chat-a".into(),
                package_name: package.into(),
            })
        }));
        p
    }
    fn push_open(p: &ScenePlugin, now: u64) -> String {
        call(
            p,
            "lease.open",
            json!({"owner":"pet.demo","notify":{"packageName":"com.operit.pet"}}),
            now,
        )
        .unwrap()["lease"]
            .as_str()
            .unwrap()
            .into()
    }
    #[test]
    fn push_idle_is_asleep_and_requires_an_authorized_chat() {
        use std::future::Future;
        let p = ScenePlugin::new(320, 240, 24, true);
        assert!(call(
            &p,
            "lease.open",
            json!({"owner":"pet","notify":{"packageName":"com.operit.pet"}}),
            0
        )
        .is_err());
        let p = push_plugin();
        push_open(&p, 0);
        assert!(p.pending_delivery().is_none());
        let mut wait = Box::pin(p.wait_event());
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(wait.as_mut().poll(&mut cx).is_pending());
        p.system_exit();
        assert!(wait.as_mut().poll(&mut cx).is_ready());
        let d = p.pending_delivery().unwrap();
        assert_eq!(d.batch.events[0].r#type, "system.exit");
        assert!(d.batch.valid());
    }
    #[test]
    fn push_bounded_queue_batches_replay_and_late_lease_ack() {
        let p = push_plugin();
        let lease = push_open(&p, 0);
        {
            let mut s = p.state.lock().unwrap();
            for _ in 0..20 {
                s.event("target.tap", Some(1), Some(2), Some("pet"));
            }
        }
        let d = p.pending_delivery().unwrap();
        assert_eq!(d.batch.lease, lease);
        assert_eq!(d.batch.events.len(), 4);
        assert_eq!(d.batch.events[0].seq, 5);
        assert_eq!(d.batch.lost_before, 4);
        assert_eq!(
            p.pending_delivery().unwrap().batch.next,
            d.batch.next,
            "no ACK means replay"
        );
        assert!(!p.acknowledge(&d, d.batch.next + 1));
        assert!(p.acknowledge(&d, d.batch.next));
        let second = p.pending_delivery().unwrap();
        assert_eq!(second.batch.events[0].seq, 9);
        p.system_exit();
        assert!(
            p.pending_delivery().unwrap().batch.closed.is_none(),
            "do not close before draining final page"
        );
        while let Some(page) = p.pending_delivery() {
            assert!(page.batch.valid());
            assert!(p.acknowledge(&page, page.batch.next));
        }
        assert!(p.pending_delivery().is_none());
        push_open(&p, 1);
        assert!(
            !p.acknowledge(&d, d.batch.next),
            "old ACK must not advance new lease"
        );
        {
            p.state
                .lock()
                .unwrap()
                .event("target.tap", Some(1), Some(2), Some("pet"));
        }
        let next = p.pending_delivery().unwrap();
        assert_eq!(next.route.space_id, "space-a");
        p.disable_delivery(&d);
        assert!(
            p.pending_delivery().is_some(),
            "old invalidation cannot kill new lease"
        );
        p.disable_delivery(&next);
        assert!(p.pending_delivery().is_none());
    }
    #[test]
    fn scene_adapter_uses_generic_actions_and_rejects_late_stream_acks() {
        use operit_node_edge::events::EdgeEventSource;
        let p = push_plugin();
        let lease = push_open(&p, 0);
        p.state
            .lock()
            .unwrap()
            .event("target.tap", Some(1), Some(2), Some("pet"));
        let d = p.pending_event_delivery().unwrap();
        assert!(d.batch.valid());
        assert_eq!(d.batch.source, "display.scene");
        assert_eq!(d.batch.stream, lease);
        assert_eq!(d.batch.events[0].action, "target.tap");
        assert_eq!(d.batch.events[0].data, json!({"x":1,"y":2,"target":"pet"}));
        assert!(!p.acknowledge_event_delivery(&d, d.batch.next + 1));
        assert!(p.acknowledge_event_delivery(&d, d.batch.next));
        p.system_exit();
        let exited = p.pending_event_delivery().unwrap();
        assert_eq!(exited.batch.events[0].action, "system.exit");
        assert_eq!(exited.batch.events[0].data, json!({}));
        push_open(&p, 1);
        assert!(!p.acknowledge_event_delivery(&exited, exited.batch.next));
    }
    #[test]
    fn compatibility_poll_does_not_ack_or_enable_push() {
        let p = ScenePlugin::new(320, 240, 24, true);
        let l = open(&p);
        p.system_exit();
        assert!(p.pending_delivery().is_none());
        assert_eq!(
            call(&p, "events.poll", json!({"lease":l}), 0).unwrap()["events"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    fn call(p: &ScenePlugin, action: &str, mut args: Value, now: u64) -> Result<Value> {
        args["v"] = json!(1);
        p.dispatch(action, args, now)
    }
    fn open(p: &ScenePlugin) -> String {
        call(p, "lease.open", json!({"owner":"pet.demo"}), 0).unwrap()["lease"]
            .as_str()
            .unwrap()
            .into()
    }
    #[test]
    fn device_owned_geometry_supports_other_screens_and_no_system_bar() {
        for (width, height, reserved_top, touch) in [
            (128, 128, 0, false),
            (480, 272, 16, true),
            (64, 48, 5, false),
        ] {
            let p = ScenePlugin::new(width, height, reserved_top, touch);
            let capabilities = call(&p, "capabilities", json!({}), 0).unwrap();
            assert_eq!(capabilities["screen"]["width"], width);
            assert_eq!(capabilities["screen"]["height"], height);
            assert_eq!(capabilities["reserved"]["h"], reserved_top);
            assert_eq!(capabilities["touch"], touch);
            let region = call(&p, "lease.open", json!({"owner":"test.device"}), 0).unwrap();
            assert_eq!(
                region["rect"],
                json!({"x":0,"y":reserved_top,"w":width,"h":height-reserved_top})
            );
            let lease = region["lease"].clone();
            call(
                &p,
                "scene.set",
                json!({"lease":lease,"background":0xf800,"layers":[]}),
                0,
            )
            .unwrap();
            let mut row = vec![0x55; width as usize * 2];
            p.paint_strip(&mut row, reserved_top, 1, 0);
            assert!(row.chunks_exact(2).all(|pixel| pixel == [0, 248]));
            if reserved_top > 0 {
                row.fill(0x55);
                p.paint_strip(&mut row, 0, 1, 0);
                assert!(row.iter().all(|byte| *byte == 0x55));
                call(&p, "lease.close", json!({"lease":lease}), 0).unwrap();
                assert_eq!(
                    call(
                        &p,
                        "lease.open",
                        json!({"owner":"test.device", "rect":{"x":0,"y":0,"w":width,"h":height}}),
                        0
                    )
                    .unwrap_err()
                    .0,
                    "SCENE_LIMIT"
                );
            }
        }
    }
    fn install(p: &ScenePlugin, l: &str, name: &str, data: &[u8]) {
        let u = call(
            p,
            "pack.begin",
            json!({"lease":l,"packId":name,"bytes":data.len(),"sha256":hash(data)}),
            0,
        )
        .unwrap()["upload"]
            .clone();
        for (index, part) in data.chunks(MAX_CHUNK).enumerate() {
            call(
                p,
                "pack.chunk",
                json!({"lease":l,"upload":u,"offset":index*MAX_CHUNK,"data":STANDARD.encode(part)}),
                0,
            )
            .unwrap();
        }
        call(p, "pack.commit", json!({"lease":l,"upload":u}), 0).unwrap();
    }
    fn scene(p: &ScenePlugin, l: &str, layers: Value) {
        call(p, "scene.set", json!({"lease":l,"layers":layers}), 0).unwrap();
    }
    #[test]
    fn parser_rejects_every_truncation_and_bad_palette() {
        let pack = bytes();
        assert!(parse_pack(&pack).is_ok());
        for n in 0..pack.len() {
            assert!(parse_pack(&pack[..n]).is_err(), "prefix {n}");
        }
        let mut bad = pack.clone();
        bad.push(0);
        assert!(parse_pack(&bad).is_err());
        let mut bad = pack;
        *bad.last_mut().unwrap() = 4;
        assert!(parse_pack(&bad).is_err());
    }
    #[test]
    fn leases_protect_system_region_and_expire_without_core() {
        let p = ScenePlugin::new(320, 240, 24, true);
        assert_eq!(
            call(
                &p,
                "lease.open",
                json!({"owner":"pet","rect":{"x":0,"y":0,"w":320,"h":240}}),
                0
            )
            .unwrap_err()
            .0,
            "SCENE_LIMIT"
        );
        let l = open(&p);
        assert_eq!(
            call(&p, "lease.open", json!({"owner":"other"}), 0)
                .unwrap_err()
                .0,
            "SCENE_BUSY"
        );
        assert_eq!(
            call(&p, "lease.renew", json!({"lease":"wrong"}), 0)
                .unwrap_err()
                .0,
            "SCENE_LEASE"
        );
        call(&p, "lease.renew", json!({"lease":l}), 1000).unwrap();
        assert_eq!(
            call(&p, "scene.set", json!({"lease":l,"layers":[]}), 31000)
                .unwrap_err()
                .0,
            "SCENE_EXPIRED"
        );
        let events = call(&p, "events.poll", json!({"lease":l}), 31000).unwrap();
        assert_eq!(events["events"][0]["type"], "lease.expired");
        let next = call(&p, "lease.open", json!({"owner":"new"}), 31000).unwrap();
        assert_ne!(next["lease"], l);
        assert_eq!(
            call(&p, "events.poll", json!({"lease":l}), 31000)
                .unwrap_err()
                .0,
            "SCENE_LEASE"
        );
    }
    #[test]
    fn upload_is_ordered_idempotent_and_atomic() {
        let p = ScenePlugin::new(320, 240, 24, true);
        let l = open(&p);
        let data = bytes();
        let u = call(
            &p,
            "pack.begin",
            json!({"lease":l,"packId":"demo","bytes":data.len(),"sha256":hash(&data)}),
            0,
        )
        .unwrap()["upload"]
            .clone();
        assert_eq!(
            call(
                &p,
                "pack.chunk",
                json!({"lease":l,"upload":u,"offset":1,"data":STANDARD.encode(&data)}),
                0
            )
            .unwrap_err()
            .0,
            "SCENE_OFFSET"
        );
        assert_eq!(
            call(&p, "pack.commit", json!({"lease":l,"upload":u}), 0)
                .unwrap_err()
                .0,
            "SCENE_OFFSET"
        );
        for _ in 0..2 {
            let result = call(
                &p,
                "pack.chunk",
                json!({"lease":l,"upload":u,"offset":0,"data":STANDARD.encode(&data)}),
                0,
            )
            .unwrap();
            assert_eq!(result["offset"], data.len());
        }
        call(&p, "pack.commit", json!({"lease":l,"upload":u}), 0).unwrap();
        let u = call(
            &p,
            "pack.begin",
            json!({"lease":l,"packId":"demo","bytes":data.len(),"sha256":"0".repeat(64)}),
            0,
        )
        .unwrap()["upload"]
            .clone();
        call(
            &p,
            "pack.chunk",
            json!({"lease":l,"upload":u,"offset":0,"data":STANDARD.encode(&data)}),
            0,
        )
        .unwrap();
        assert_eq!(
            call(&p, "pack.commit", json!({"lease":l,"upload":u}), 0)
                .unwrap_err()
                .0,
            "SCENE_HASH"
        );
        assert_eq!(
            call(&p, "capabilities", json!({}), 0).unwrap()["cache"]["packs"][0]["sha256"],
            hash(&data)
        );
        call(&p, "pack.abort", json!({"lease":l,"upload":u}), 0).unwrap();
        install(&p, &l, "two", &data);
        assert_eq!(
            call(
                &p,
                "pack.begin",
                json!({"lease":l,"packId":"three","bytes":data.len(),"sha256":hash(&data)}),
                0
            )
            .unwrap_err()
            .0,
            "SCENE_LIMIT"
        );
    }
    #[test]
    fn scene_updates_are_atomic_and_displayed_assets_cannot_be_replaced() {
        let p = ScenePlugin::new(320, 240, 24, true);
        let l = open(&p);
        let data = bytes();
        install(&p, &l, "demo", &data);
        scene(&p, &l, json!([{"asset":"demo:pet","x":0,"y":0}]));
        let revision = p.summary()["revision"].clone();
        assert!(call(&p,"scene.set",json!({"lease":l,"layers":[{"asset":"demo:pet","x":0,"y":0},{"asset":"missing:pet","x":0,"y":0}]}),0).is_err());
        assert_eq!(p.summary()["revision"], revision);
        assert_eq!(
            call(&p, "pack.remove", json!({"lease":l,"packId":"demo"}), 0)
                .unwrap_err()
                .0,
            "SCENE_IN_USE"
        );
        assert_eq!(
            call(
                &p,
                "pack.begin",
                json!({"lease":l,"packId":"demo","bytes":data.len(),"sha256":hash(&data)}),
                0
            )
            .unwrap_err()
            .0,
            "SCENE_IN_USE"
        );
        scene(&p, &l, json!([]));
        call(&p, "pack.remove", json!({"lease":l,"packId":"demo"}), 0).unwrap();
    }
    #[test]
    fn rgb565_animation_alpha_anchors_clipping_and_touch_golden() {
        let p = ScenePlugin::new(320, 240, 24, true);
        let l = open(&p);
        install(&p, &l, "demo", &bytes());
        scene(
            &p,
            &l,
            json!([{"asset":"demo:pet","x":2,"y":2,"scale":2,"anchor":"bc","play":true,"target":"pet"}]),
        );
        let mut strip = vec![0x55; 1280];
        p.paint_strip(&mut strip, 24, 2, 0);
        assert_eq!(&strip[..4], &[0, 248, 0, 248]);
        assert_eq!(&strip[4..8], &[0, 0, 0, 0]);
        p.paint_strip(&mut strip, 24, 2, 50);
        assert_eq!(&strip[..4], &[224, 7, 224, 7]);
        assert_eq!(&strip[4..8], &[16, 0, 16, 0]);
        p.paint_strip(&mut strip, 24, 2, 100);
        assert_eq!(&strip[..4], &[0, 248, 0, 248]);
        let mut reserved = vec![0x55; 1280];
        p.paint_strip(&mut reserved, 0, 2, 50);
        assert!(reserved.iter().all(|b| *b == 0x55));
        p.touch(0, 0);
        p.touch(0, 24);
        let e = call(&p, "events.poll", json!({"lease":l}), 0).unwrap();
        assert_eq!(e["events"].as_array().unwrap().len(), 1);
        assert_eq!(e["events"][0]["target"], "pet");
        let (_, a) = {
            let s = p.state.lock().unwrap();
            let (pack, a) = s.resource("demo:pet").unwrap();
            assert!(sample(
                pack,
                a,
                &Layer {
                    asset: "demo:pet".into(),
                    x: -1,
                    y: 0,
                    scale: 1,
                    anchor: "tl".into(),
                    frame: 0,
                    play: false,
                    r#loop: true,
                    target: None
                },
                0,
                0,
                0
            )
            .is_some());
            ((), a.clone())
        };
        assert_eq!(a.frames, 2);
    }
    #[test]
    fn event_pages_signal_loss_and_system_exit_remains_pollable() {
        let p = ScenePlugin::new(320, 240, 24, true);
        let l = open(&p);
        {
            let mut s = p.state.lock().unwrap();
            for _ in 0..20 {
                s.event("target.tap", Some(1), Some(2), Some("pet"));
            }
        }
        let e = call(&p, "events.poll", json!({"lease":l,"after":0}), 0).unwrap();
        assert_eq!(e["lostBefore"], 4);
        assert_eq!(e["events"].as_array().unwrap().len(), 8);
        assert_eq!(e["next"], 12);
        p.system_exit();
        assert_eq!(p.summary()["active"], false);
        let e = call(&p, "events.poll", json!({"lease":l,"after":20}), 0).unwrap();
        assert_eq!(e["events"][0]["type"], "system.exit");
        assert_eq!(
            call(&p, "scene.set", json!({"lease":l,"layers":[]}), 0)
                .unwrap_err()
                .0,
            "SCENE_CLOSED"
        );
    }
    #[test]
    fn wire_envelopes_are_bounded_and_strict() {
        let p = ScenePlugin::new(320, 240, 24, false);
        let args = operit_link::toCoreValue(json!({"v":2})).unwrap();
        let reply = p.invoke("capabilities", args).unwrap();
        let reply: Value = operit_link::fromCoreValue(reply).unwrap();
        assert_eq!(reply["error"]["code"], "SCENE_VERSION");
        assert!(call(&p, "capabilities", json!({"surprise":true}), 0).is_err());
        assert_eq!(
            call(&p, "lease.open", json!({"owner":"x".repeat(1024)}), 0)
                .unwrap_err()
                .0,
            "SCENE_LIMIT"
        );
        let l = open(&p);
        install(&p, &l, "demo", &bytes());
        install(&p, &l, "second", &bytes());
        for action in ["capabilities", "events.poll"] {
            let reply = call(
                &p,
                action,
                if action == "capabilities" {
                    json!({})
                } else {
                    json!({"lease":l})
                },
                0,
            )
            .unwrap();
            assert!(serde_json::to_vec(&reply).unwrap().len() < MAX_REPLY);
        }
        assert_eq!(
            call(
                &p,
                "pack.begin",
                json!({"lease":l,"packId":"bad","bytes":4097,"sha256":"0".repeat(64)}),
                0
            )
            .unwrap_err()
            .0,
            "SCENE_LIMIT"
        );
        assert_eq!(
            call(&p, "lease.open", json!({"owner":"bad"}), 0)
                .unwrap_err()
                .0,
            "SCENE_BUSY"
        );
    }
}
