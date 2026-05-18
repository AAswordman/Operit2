use crate::data::model::OpenAIModels::ModelOption;

pub struct ModelListFetcher;

impl ModelListFetcher {
    pub fn fetch(_provider_type: &str, _endpoint: &str) -> Vec<ModelOption> {
        Vec::new()
    }
}
