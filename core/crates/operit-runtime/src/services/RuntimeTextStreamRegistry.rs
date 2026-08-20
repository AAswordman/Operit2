use operit_providers::chat::llmprovider::AIService::SharedAiResponseStream;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static TEXT_STREAMS: OnceLock<Mutex<HashMap<String, SharedAiResponseStream>>> = OnceLock::new();

/// Returns the process-wide registry for bridge-visible text streams.
fn textStreams() -> &'static Mutex<HashMap<String, SharedAiResponseStream>> {
    TEXT_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Provides the generic runtime endpoint for message-owned text streams.
pub struct RuntimeTextStreamRegistry;

impl RuntimeTextStreamRegistry {
    /// Creates the stateless generic stream endpoint.
    pub fn new() -> Self {
        Self
    }

    /// Opens one registered text stream without interpreting chat semantics.
    #[allow(non_snake_case)]
    #[operit_core_annotations::operit_core_route(binding = routeKey)]
    pub fn openTextStream(
        &self,
        streamId: String,
        routeKey: String,
    ) -> Option<SharedAiResponseStream> {
        let _ = routeKey;
        textStreams()
            .lock()
            .expect("text stream registry mutex poisoned")
            .get(&streamId)
            .cloned()
    }
}

/// Registers one logical text stream under its transport-independent identifier.
pub fn registerTextStream(streamId: String, stream: SharedAiResponseStream) {
    textStreams()
        .lock()
        .expect("text stream registry mutex poisoned")
        .insert(streamId, stream);
}

/// Removes one completed text stream from the generic bridge registry.
pub fn removeTextStream(streamId: &str) {
    textStreams()
        .lock()
        .expect("text stream registry mutex poisoned")
        .remove(streamId);
}
