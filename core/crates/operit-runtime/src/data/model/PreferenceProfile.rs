use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferenceProfile {
    pub id: String,
    pub name: String,
    pub birthDate: i64,
    pub gender: String,
    pub personality: String,
    pub identity: String,
    pub occupation: String,
    pub aiStyle: String,
    pub isInitialized: bool,
}

impl PreferenceProfile {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            birthDate: 0,
            gender: String::new(),
            personality: String::new(),
            identity: String::new(),
            occupation: String::new(),
            aiStyle: String::new(),
            isInitialized: false,
        }
    }
}
