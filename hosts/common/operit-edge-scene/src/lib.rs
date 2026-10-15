//! Bounded, device-owned presentation. No scripts, filesystem paths or business state.
use base64::{engine::general_purpose::STANDARD, Engine};
pub mod protocol;
use operit_link::CoreValue;
use protocol::*;
use operit_host_api::RuntimeStorageHost;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
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

const CONFIG_PATH: &str = "runtime/device/display_scene.json";
const MAX_STORED_BYTES: usize = ((MAX_CACHE + 2) / 3) * 4 + MAX_REQUEST + 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneConfiguration {
    pub enabled: bool,
    pub background: u16,
    pub layers: Vec<Layer>,
}
impl Default for SceneConfiguration {
    fn default() -> Self { Self { enabled: false, background: 0, layers: vec![] } }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredConfiguration {
    v: u32,
    config: SceneConfiguration,
    packs: BTreeMap<String, String>,
}
struct State {
    config: SceneConfiguration,
    active: bool,
    packs: BTreeMap<String, Pack>,
    upload: Option<Upload>,
    layers: Vec<Layer>,
    background: u16,
    started: u64,
    taps: u64,
    revision: u32,
}
impl State {
    fn resource(&self, name: &str) -> Result<(&Pack, &Asset)> {
        let (pack, asset) = name.split_once(':').ok_or_else(invalid)?;
        let pack = self.packs.get(pack).ok_or(("SCENE_RESOURCE", "pack not cached"))?;
        Ok((pack, pack.assets.get(asset).ok_or(("SCENE_RESOURCE", "asset not cached"))?))
    }
    fn in_use(&self, name: &str) -> bool {
        self.config.layers.iter().any(|layer| layer.asset.split_once(':').is_some_and(|(pack, _)| pack == name))
    }
    fn validate(&self, config: &SceneConfiguration) -> Result<()> {
        if config.layers.len() > MAX_LAYERS { return Err(("SCENE_LIMIT", "too many layers")); }
        for layer in &config.layers {
            let (_, asset) = self.resource(&layer.asset)?;
            if !(1..=8).contains(&layer.scale) || !["tl", "center", "bc"].contains(&layer.anchor.as_str())
                || layer.frame >= asset.frames || layer.target.as_ref().is_some_and(|target| !id(target)) {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn apply(&mut self, config: SceneConfiguration, now: u64) {
        self.active = config.enabled;
        self.layers = config.layers.clone();
        self.background = config.background;
        self.config = config;
        self.started = now;
        self.taps = 0;
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Device-owned display configuration and local rendering. Host packages only
/// upload resources and edit configuration; there are no display leases or hooks.
pub struct SceneService {
    state: Mutex<State>,
    storage: Option<Arc<dyn RuntimeStorageHost>>,
    clock: Instant,
    width: u16,
    height: u16,
    reserved_top: u16,
    touch: bool,
}
impl SceneService {
    pub fn new(width: u16, height: u16, reserved_top: u16, touch: bool) -> Self {
        assert!(width > 0 && height > reserved_top);
        Self {
            state: Mutex::new(State { config: SceneConfiguration::default(), active: false,
                packs: BTreeMap::new(), upload: None, layers: vec![], background: 0,
                started: 0, taps: 0, revision: 0 }),
            storage: None, clock: Instant::now(), width, height, reserved_top, touch,
        }
    }
    /// Restore configuration and assets from this device's own durable storage.
    pub fn with_storage(mut self, storage: Arc<dyn RuntimeStorageHost>) -> std::result::Result<Self, String> {
        if storage.exists(CONFIG_PATH).map_err(|error| error.message)? {
            let bytes = storage.readBytes(CONFIG_PATH).map_err(|error| error.message)?;
            if bytes.len() > MAX_STORED_BYTES { return Err("Stored scene exceeds capacity".into()); }
            let record: StoredConfiguration = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if record.v != VERSION || record.packs.len() > MAX_PACKS { return Err("Invalid stored scene version or pack count".into()); }
            let mut state = self.state.lock().unwrap();
            let mut total = 0usize;
            for (name, data) in record.packs {
                let bytes = STANDARD.decode(data).map_err(|error| error.to_string())?;
                total += bytes.len();
                if !id(&name) || bytes.len() > MAX_PACK || total > MAX_CACHE { return Err("Invalid stored scene assets".into()); }
                let assets = parse_pack(&bytes).map_err(|(_, message)| message.to_string())?;
                let sha = pack_hash(&bytes);
                state.packs.insert(name, Pack { bytes, sha, assets });
            }
            state.validate(&record.config).map_err(|(_, message)| message.to_string())?;
            state.apply(record.config, 0);
        }
        self.storage = Some(storage);
        Ok(self)
    }
    fn persist(&self, state: &State, config: &SceneConfiguration) -> Result<()> {
        let Some(storage) = &self.storage else { return Ok(()); };
        let packs = state.packs.iter().filter(|(name, _)| config.layers.iter().any(|layer|
            layer.asset.split_once(':').is_some_and(|(pack, _)| pack == name.as_str())))
            .map(|(name, pack)| (name.clone(), STANDARD.encode(&pack.bytes))).collect();
        let record = StoredConfiguration { v: VERSION, config: config.clone(), packs };
        let bytes = serde_json::to_vec(&record).map_err(|_| invalid())?;
        if bytes.len() > MAX_STORED_BYTES { return Err(("SCENE_LIMIT", "stored scene exceeds capacity")); }
        storage.writeBytesAtomically(CONFIG_PATH, &bytes).map_err(|_| ("SCENE_STORAGE", "failed to persist device configuration"))
    }
    fn now(&self) -> u64 { self.clock.elapsed().as_millis() as u64 }
    fn rect(&self) -> Rect { Rect { x: 0, y: self.reserved_top, w: self.width, h: self.height - self.reserved_top } }
    pub fn summary(&self) -> Value {
        let state = self.state.lock().unwrap();
        json!({"active":state.active,"revision":state.revision,"taps":state.taps})
    }
    /// Local editor mirror only; resource data is not part of the remote config API.
    pub fn visual_snapshot(&self) -> Value {
        let state = self.state.lock().unwrap();
        json!({"active":state.active,"revision":state.revision,"rect":self.rect(),"background":state.background,
            "elapsed":self.now().saturating_sub(state.started),"layers":state.layers,
            "packs":state.packs.iter().map(|(id,pack)|json!({"id":id,"data":STANDARD.encode(&pack.bytes)})).collect::<Vec<_>>()})
    }
    /// Local exit affects the current presentation, not the stored configuration.
    pub fn system_exit(&self) {
        let mut state = self.state.lock().unwrap();
        state.active = false;
        state.revision = state.revision.wrapping_add(1);
    }
    pub fn touch(&self, x: u16, y: u16) -> Option<String> {
        if !self.touch { return None; }
        let now = self.now();
        let mut state = self.state.lock().unwrap();
        let rect = self.rect();
        if !state.active || x < rect.x || y < rect.y || x >= rect.x + rect.w || y >= rect.y + rect.h { return None; }
        let (x, y) = (x - rect.x, y - rect.y);
        let target = state.layers.iter().rev().find_map(|layer| {
            let target = layer.target.as_ref()?;
            let (pack, asset) = state.resource(&layer.asset).ok()?;
            sample(pack, asset, layer, x as i32, y as i32, now.saturating_sub(state.started))
                .filter(|(_, alpha)| *alpha > 0).map(|_| target.clone())
        });
        if target.is_some() { state.taps = state.taps.saturating_add(1); }
        target
    }
    pub fn advance_target_frame(&self, target: &str) {
        let mut state = self.state.lock().unwrap();
        if !state.active { return; }
        let updates = state.layers.iter().enumerate().filter_map(|(index, layer)| {
            if layer.target.as_deref() != Some(target) { return None; }
            let (_, asset) = state.resource(&layer.asset).ok()?;
            Some((index, (layer.frame + 1) % asset.frames))
        }).collect::<Vec<_>>();
        for (index, frame) in updates { state.layers[index].frame = frame; state.layers[index].play = false; }
        state.revision = state.revision.wrapping_add(1);
    }
    pub fn paint_strip(&self, bytes: &mut [u8], y: u16, rows: u16, tick: u32) {
        let state = self.state.lock().unwrap();
        if !state.active || bytes.len() != self.width as usize * rows as usize * 2 { return; }
        let rect = self.rect();
        let elapsed = tick.wrapping_sub(state.started as u32) as u64;
        for row in 0..rows {
            let sy = y + row;
            if sy < rect.y || sy >= rect.y + rect.h { continue; }
            for x in rect.x..rect.x + rect.w {
                let mut pixel = state.background;
                for layer in &state.layers {
                    if let Ok((pack, asset)) = state.resource(&layer.asset) {
                        if let Some((color, alpha)) = sample(pack, asset, layer, (x - rect.x) as i32, (sy - rect.y) as i32, elapsed) {
                            pixel = blend(pixel, color, alpha);
                        }
                    }
                }
                let index = ((row as usize * self.width as usize) + x as usize) * 2;
                bytes[index..index + 2].copy_from_slice(&pixel.to_le_bytes());
            }
        }
    }
    pub fn tick(&self) -> u32 { self.now() as u32 }
    fn dispatch(&self, action: &str, args: Value, now: u64) -> Result<Value> {
        if !args.is_object() || serde_json::to_vec(&args).map_err(|_| invalid())?.len() > MAX_REQUEST { return Err(("SCENE_LIMIT", "request exceeds capacity")); }
        if args["v"].as_u64() != Some(VERSION as u64) { return Err(("SCENE_VERSION", "expected protocol v:1")); }
        let fields: &[&str] = match action {
            "capabilities" | "config.read" | "config.clear" | "state" => &[],
            "config.apply" => &["enabled", "background", "layers"],
            "pack.begin" => &["packId", "bytes", "sha256"],
            "pack.chunk" => &["upload", "offset", "data"],
            "pack.commit" | "pack.abort" => &["upload"],
            "pack.remove" => &["packId"],
            _ => return Err(invalid()),
        };
        if args.as_object().unwrap().keys().any(|field| field != "v" && !fields.contains(&field.as_str())) { return Err(invalid()); }
        let mut s = self.state.lock().unwrap();
        let a = args;
        match action {
            "capabilities" => Ok(json!({"protocol":VERSION,"screen":{"width":self.width,"height":self.height,"format":"RGB565_LE"},
                "input":{"local":true},"reserved":{"x":0,"y":0,"w":self.width,"h":self.reserved_top},"touch":self.touch,
                "configuration":{"persistent":self.storage.is_some(),"hostRequired":false},
                "animation":{"local":true,"minFrameMs":50,"maxFrameMs":2000,"maxFrames":8},
                "formats":{"pack":"ESP1","asset":"ESI1","alpha":"straight","paletteMax":16,"maxWidth":96,"maxHeight":96},
                "limits":{"requestBytes":MAX_REQUEST,"replyBytes":MAX_REPLY,"packBytes":MAX_PACK,"cacheBytes":MAX_CACHE,"packs":MAX_PACKS,"assetsPerPack":MAX_ASSETS,"layers":MAX_LAYERS,"chunkBytes":MAX_CHUNK,"scaleMax":8,"uploadBytes":MAX_PACK},
                "cache":{"persistent":self.storage.is_some(),"usedBytes":s.packs.values().map(|pack|pack.bytes.len()).sum::<usize>(),
                    "packs":s.packs.iter().map(|(id,pack)|json!({"id":id,"bytes":pack.bytes.len(),"sha256":pack.sha,"assets":pack.assets.keys().collect::<Vec<_>>()})).collect::<Vec<_>>()}})),
            "config.read" => serde_json::to_value(&s.config).map_err(|_| invalid()),
            "state" => Ok(json!({"active":s.active,"taps":s.taps,"revision":s.revision})),
            "config.apply" | "config.clear" => {
                let config = if action == "config.clear" { SceneConfiguration::default() } else {
                    let mut fields = a.as_object().unwrap().clone(); fields.remove("v");
                    serde_json::from_value::<SceneConfiguration>(Value::Object(fields)).map_err(|_| invalid())?
                };
                s.validate(&config)?;
                self.persist(&s, &config)?;
                s.apply(config, now);
                Ok(json!({"configured":true,"persistent":self.storage.is_some(),"revision":s.revision}))
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
            _ => Err(invalid()),
        }
    }
    pub fn call(&self, method: &str, args: CoreValue) -> std::result::Result<CoreValue, String> {
        let args = operit_link::fromCoreValue::<Value>(args).map_err(|error| error.to_string())?;
        let reply = match self.dispatch(method, args, self.now()) {
            Ok(result) => json!({"v":VERSION,"ok":true,"result":result}),
            Err((code, message)) => json!({"v":VERSION,"ok":false,"error":{"code":code,"message":message}}),
        };
        if serde_json::to_vec(&reply).unwrap().len() > MAX_REPLY { return Err("Scene reply exceeds capacity".into()); }
        operit_link::toCoreValue(reply).map_err(|error| error.to_string())
    }
}
fn pack_hash(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes).as_ref().iter().map(|byte|format!("{byte:02x}")).collect()
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

    #[derive(Default)]
    struct Storage {files:Mutex<BTreeMap<String,Vec<u8>>>, fail: std::sync::atomic::AtomicBool}
    impl RuntimeStorageHost for Storage {
        fn runtimeRootDir(&self)->Option<std::path::PathBuf> {None}
        fn workspaceRootDir(&self)->Option<std::path::PathBuf> {None}
        fn readBytes(&self,path:&str)->operit_host_api::HostResult<Vec<u8>> {self.files.lock().unwrap().get(path).cloned().ok_or_else(||operit_host_api::HostError::new("missing"))}
        fn writeBytes(&self,path:&str,data:&[u8])->operit_host_api::HostResult<()> {self.files.lock().unwrap().insert(path.into(),data.to_vec());Ok(())}
        fn writeBytesAtomically(&self,path:&str,data:&[u8])->operit_host_api::HostResult<()> {
            if self.fail.swap(false,std::sync::atomic::Ordering::SeqCst) {return Err(operit_host_api::HostError::new("disk failure"));}
            self.writeBytes(path,data)
        }
        fn appendBytes(&self,_:&str,_:&[u8])->operit_host_api::HostResult<()> {unimplemented!()}
        fn delete(&self,path:&str,_:bool)->operit_host_api::HostResult<()> {self.files.lock().unwrap().remove(path);Ok(())}
        fn exists(&self,path:&str)->operit_host_api::HostResult<bool> {Ok(self.files.lock().unwrap().contains_key(path))}
        fn list(&self,_:&str)->operit_host_api::HostResult<Vec<operit_host_api::RuntimeStorageEntry>> {Ok(vec![])}
    }
    fn upload(scene:&SceneService, sha: &str)->Result<Value> {
        let data=bytes();
        let started=scene.dispatch("pack.begin",json!({"v":1,"packId":"demo","bytes":data.len(),"sha256":sha}),0)?;
        let handle=started["upload"].as_str().unwrap();
        scene.dispatch("pack.chunk",json!({"v":1,"upload":handle,"offset":0,"data":STANDARD.encode(data)}),0)?;
        scene.dispatch("pack.commit",json!({"v":1,"upload":handle}),0)
    }
    fn configuration()->Value {json!({"v":1,"enabled":true,"background":0,"layers":[{"asset":"demo:pet","x":4,"y":4,"scale":2,"target":"pet"}]})}
    #[test]
    fn configuration_restores_assets_and_runs_without_a_host_or_lease() {
        let storage=Arc::new(Storage::default());
        let scene=SceneService::new(32,32,2,true).with_storage(storage.clone()).unwrap();
        upload(&scene,&hash(&bytes())).unwrap();
        scene.dispatch("config.apply",configuration(),10).unwrap();
        assert!(scene.dispatch("state",json!({"v":1}),600_000).unwrap()["active"].as_bool().unwrap());
        assert_eq!(scene.touch(4,6).as_deref(),Some("pet"));
        scene.advance_target_frame("pet");
        assert_eq!(scene.summary()["taps"],1);
        let mut strip=vec![0u8;32*2*8];scene.paint_strip(&mut strip,0,8,100);
        assert!(strip.iter().any(|byte|*byte!=0));
        drop(scene);
        let restored=SceneService::new(32,32,2,true).with_storage(storage).unwrap();
        assert_eq!(restored.summary()["active"],true);
        assert_eq!(restored.visual_snapshot()["packs"].as_array().unwrap().len(),1);
        assert_eq!(restored.touch(4,6).as_deref(),Some("pet"));
        restored.system_exit();
        assert_eq!(restored.summary()["active"],false);
        assert_eq!(restored.dispatch("config.read",json!({"v":1}),0).unwrap()["enabled"],true);
    }
    #[test]
    fn failed_atomic_save_and_invalid_config_keep_previous_state() {
        let storage=Arc::new(Storage::default());
        let scene=SceneService::new(32,32,2,true).with_storage(storage.clone()).unwrap();
        upload(&scene,&hash(&bytes())).unwrap();
        scene.dispatch("config.apply",configuration(),0).unwrap();
        let before=storage.readBytes(CONFIG_PATH).unwrap();
        storage.fail.store(true,std::sync::atomic::Ordering::SeqCst);
        assert_eq!(scene.dispatch("config.clear",json!({"v":1}),0).unwrap_err().0,"SCENE_STORAGE");
        assert_eq!(storage.readBytes(CONFIG_PATH).unwrap(),before);
        assert_eq!(scene.summary()["active"],true);
        let mut bad=configuration();bad["layers"][0]["asset"]=json!("missing:asset");
        assert!(scene.dispatch("config.apply",bad,0).is_err());
        assert_eq!(storage.readBytes(CONFIG_PATH).unwrap(),before);
        scene.dispatch("config.clear",json!({"v":1}),0).unwrap();
        let restored=SceneService::new(32,32,2,true).with_storage(storage).unwrap();
        assert_eq!(restored.summary()["active"],false);
    }
    #[test]
    fn uploads_verify_hash_and_enforce_capacity() {
        let scene=SceneService::new(32,32,2,true);
        assert_eq!(upload(&scene,&"0".repeat(64)).unwrap_err().0,"SCENE_HASH");
        assert!(scene.visual_snapshot()["packs"].as_array().unwrap().is_empty());
        assert!(scene.dispatch("pack.begin",json!({"v":1,"packId":"demo","bytes":MAX_PACK+1,"sha256":hash(&bytes())}),0).is_err());
        // Failed hash leaves the upload cancellable so a caller can retry.
        let handle=scene.state.lock().unwrap().upload.as_ref().map(|u|u.handle.clone());
        if let Some(handle)=handle {scene.dispatch("pack.abort",json!({"v":1,"upload":handle}),0).unwrap();}
        upload(&scene,&hash(&bytes())).unwrap();
    }

}
