use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPackage {
    pub name: String,
    pub description: String,
    #[serde(
        serialize_with = "serialize_virtual_path",
        deserialize_with = "deserialize_virtual_path"
    )]
    pub directory: PathBuf,
    #[serde(
        serialize_with = "serialize_virtual_path",
        deserialize_with = "deserialize_virtual_path"
    )]
    pub skillFile: PathBuf,
}

/// Serializes portable virtual addresses regardless of the host's path separator.
fn serialize_virtual_path<S: serde::Serializer>(
    path: &std::path::Path,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let path =
        operit_vfs::VfsPath::new(&path.to_string_lossy()).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(path.as_str())
}

/// Rejects native and relative paths when restoring a serialized skill package.
fn deserialize_virtual_path<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<PathBuf, D::Error> {
    let path = String::deserialize(deserializer)?;
    let path = operit_vfs::VfsPath::new(&path).map_err(serde::de::Error::custom)?;
    Ok(PathBuf::from(path.as_str()))
}
