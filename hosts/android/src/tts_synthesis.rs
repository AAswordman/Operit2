use std::sync::Arc;

use operit_host_api::{HostResult, TtsSynthesisHost, TtsSynthesisRequest, TtsSynthesisResponse};

pub type AndroidTtsSynthesizer =
    Arc<dyn Fn(TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> + Send + Sync>;

#[derive(Clone)]
pub struct AndroidTtsSynthesisHost {
    synthesizer: AndroidTtsSynthesizer,
}

impl AndroidTtsSynthesisHost {
    pub fn fromSynthesizer(synthesizer: AndroidTtsSynthesizer) -> Self {
        Self { synthesizer }
    }
}

impl TtsSynthesisHost for AndroidTtsSynthesisHost {
    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        (self.synthesizer)(request)
    }
}
