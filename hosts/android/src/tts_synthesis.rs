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
    /// Exports synthesized audio while keeping temporary native files inside the host.
    fn synthesizeSpeechBytes(&self, request: TtsSynthesisRequest) -> HostResult<Vec<u8>> {
        let result = self.synthesizeSpeech(request)?;
        let bytes = std::fs::read(&result.audioPath).map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        std::fs::remove_file(&result.audioPath).map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        Ok(bytes)
    }

    fn synthesizeSpeech(&self, request: TtsSynthesisRequest) -> HostResult<TtsSynthesisResponse> {
        (self.synthesizer)(request)
    }
}
