use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::AppLogger::AppLogger;
use crate::ImagePoolManager::{decode_base64, encode_base64};

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
    media_pool: HashMap<String, MediaData>,
    order: VecDeque<String>,
}

impl Default for PoolState {
    fn default() -> Self {
        Self {
            max_pool_size: 12,
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
            state()
                .lock()
                .expect("MediaPool mutex poisoned")
                .max_pool_size = value;
            AppLogger::d(TAG, &format!("pool size limit updated: {value}"));
        }
    }

    /// Registers media bytes for model input.
    pub fn add_media_bytes(bytes: &[u8], mime_type: &str) -> String {
        if bytes.len() > MAX_INPUT_BYTES {
            AppLogger::e(
                TAG,
                &format!("media input too large: bytes={}", bytes.len()),
            );
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
            AppLogger::e(
                TAG,
                &format!("media base64 decoded too large: bytes={}", bytes.len()),
            );
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
        None
    }

    pub fn remove_media(id: &str) {
        let mut guard = state().lock().expect("MediaPool mutex poisoned");
        guard.media_pool.remove(id);
        guard.order.retain(|item| item != id);
    }

    fn insert(data: MediaData) -> String {
        let id = new_id();
        let mut guard = state().lock().expect("MediaPool mutex poisoned");
        touch_locked(&mut guard, &id);
        guard.media_pool.insert(id.clone(), data.clone());
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
        } else {
            break;
        }
    }
}

fn new_id() -> String {
    let millis = operit_host_api::TimeUtils::currentTimeMillisU128();
    format!("{millis:x}")
}
