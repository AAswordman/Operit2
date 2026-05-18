use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::util::AppLogger::AppLogger;
use crate::util::ImagePoolManager::{decode_base64, encode_base64};

const TAG: &str = "MediaPoolManager";
const MAX_INPUT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MediaData {
    pub base64: String,
    pub mime_type: String,
}

#[derive(Debug)]
struct PoolState {
    max_pool_size: usize,
    cache_dir: Option<PathBuf>,
    media_pool: HashMap<String, MediaData>,
    order: VecDeque<String>,
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            max_pool_size: 12,
            cache_dir: None,
            media_pool: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

static STATE: OnceLock<Mutex<PoolState>> = OnceLock::new();

fn state() -> &'static Mutex<PoolState> {
    STATE.get_or_init(|| Mutex::new(PoolState::default()))
}

pub struct MediaPoolManager;

impl MediaPoolManager {
    pub fn set_max_pool_size(value: usize) {
        if value > 0 {
            state().lock().expect("MediaPool mutex poisoned").max_pool_size = value;
            AppLogger::d(TAG, &format!("pool size limit updated: {value}"));
        }
    }

    pub fn initialize(cache_dir_path: impl AsRef<Path>, preload_now: bool) {
        let target_dir = cache_dir_path.as_ref().join("media_pool");
        let _ = fs::create_dir_all(&target_dir);
        state().lock().expect("MediaPool mutex poisoned").cache_dir = Some(target_dir);
        if preload_now {
            Self::preload_from_disk();
        }
    }

    pub fn preload_from_disk() {
        let ids = {
            let guard = state().lock().expect("MediaPool mutex poisoned");
            let Some(dir) = &guard.cache_dir else {
                return;
            };
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("meta") {
                        path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        for id in ids {
            if let Some(data) = load_one_from_disk(&id) {
                let mut guard = state().lock().expect("MediaPool mutex poisoned");
                touch_locked(&mut guard, &id);
                guard.media_pool.insert(id, data);
                trim_locked(&mut guard);
            }
        }
    }

    pub fn add_media(file_path: &str, mime_type: &str) -> String {
        let path = Path::new(file_path);
        if !path.is_file() {
            AppLogger::e(TAG, &format!("file does not exist or is not a file: {file_path}"));
            return "error".to_string();
        }
        let Ok(bytes) = fs::read(path) else {
            AppLogger::e(TAG, "read media failed");
            return "error".to_string();
        };
        if bytes.len() > MAX_INPUT_BYTES {
            AppLogger::e(TAG, &format!("media file too large: bytes={}", bytes.len()));
            return "error".to_string();
        }
        Self::insert(MediaData {
            base64: encode_base64(&bytes),
            mime_type: mime_type.to_string(),
        })
    }

    pub fn add_media_from_base64(base64: &str, mime_type: &str) -> String {
        let bytes = decode_base64(base64);
        if bytes.len() > MAX_INPUT_BYTES {
            AppLogger::e(TAG, &format!("media base64 decoded too large: bytes={}", bytes.len()));
            return "error".to_string();
        }
        Self::insert(MediaData {
            base64: encode_base64(&bytes),
            mime_type: mime_type.to_string(),
        })
    }

    pub fn get_media(id: &str) -> Option<MediaData> {
        if let Some(data) = state()
            .lock()
            .expect("MediaPool mutex poisoned")
            .media_pool
            .get(id)
            .cloned()
        {
            return Some(data);
        }
        load_one_from_disk(id)
    }

    pub fn remove_media(id: &str) {
        let mut guard = state().lock().expect("MediaPool mutex poisoned");
        guard.media_pool.remove(id);
        guard.order.retain(|item| item != id);
        delete_from_disk_locked(&guard, id);
    }

    fn insert(data: MediaData) -> String {
        let id = new_id();
        let mut guard = state().lock().expect("MediaPool mutex poisoned");
        touch_locked(&mut guard, &id);
        guard.media_pool.insert(id.clone(), data.clone());
        save_to_disk_locked(&guard, &id, &data);
        trim_locked(&mut guard);
        id
    }
}

fn touch_locked(state: &mut PoolState, id: &str) {
    state.order.retain(|item| item != id);
    state.order.push_back(id.to_string());
}

fn trim_locked(state: &mut PoolState) {
    while state.media_pool.len() > state.max_pool_size {
        if let Some(id) = state.order.pop_front() {
            state.media_pool.remove(&id);
            delete_from_disk_locked(state, &id);
        } else {
            break;
        }
    }
}

fn save_to_disk_locked(state: &PoolState, id: &str, data: &MediaData) {
    let Some(dir) = &state.cache_dir else {
        return;
    };
    let _ = fs::create_dir_all(dir);
    let _ = fs::write(dir.join(format!("{id}.meta")), &data.mime_type);
    let _ = fs::write(dir.join(format!("{id}.b64")), &data.base64);
}

fn load_one_from_disk(id: &str) -> Option<MediaData> {
    let guard = state().lock().expect("MediaPool mutex poisoned");
    let dir = guard.cache_dir.clone()?;
    drop(guard);
    let mime_type = fs::read_to_string(dir.join(format!("{id}.meta"))).ok()?.trim().to_string();
    let base64 = fs::read_to_string(dir.join(format!("{id}.b64"))).ok()?.trim().to_string();
    if mime_type.is_empty() || base64.is_empty() {
        return None;
    }
    Some(MediaData { base64, mime_type })
}

fn delete_from_disk_locked(state: &PoolState, id: &str) {
    if let Some(dir) = &state.cache_dir {
        let _ = fs::remove_file(dir.join(format!("{id}.meta")));
        let _ = fs::remove_file(dir.join(format!("{id}.b64")));
    }
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}
