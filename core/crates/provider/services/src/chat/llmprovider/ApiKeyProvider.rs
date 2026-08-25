pub trait ApiKeyProvider {
    fn get_api_key(&self) -> String;
}

pub struct SingleApiKeyProvider {
    pub api_key: String,
}

impl ApiKeyProvider for SingleApiKeyProvider {
    fn get_api_key(&self) -> String {
        self.api_key.clone()
    }
}

pub struct MultiApiKeyProvider {
    pub config_id: String,
    pub api_keys: Vec<String>,
    pub cursor: usize,
}

impl ApiKeyProvider for MultiApiKeyProvider {
    fn get_api_key(&self) -> String {
        self.api_keys[self.cursor % self.api_keys.len()].clone()
    }
}
