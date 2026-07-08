use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPackage {
    pub name: String,
    pub description: String,
    pub directory: PathBuf,
    pub skillFile: PathBuf,
}
