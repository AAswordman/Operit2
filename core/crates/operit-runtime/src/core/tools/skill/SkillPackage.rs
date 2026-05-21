use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkillPackage {
    pub name: String,
    pub description: String,
    pub directory: PathBuf,
    pub skillFile: PathBuf,
}
