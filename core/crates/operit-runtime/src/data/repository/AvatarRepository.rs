use std::collections::HashMap;

pub struct AvatarConfig {
    pub id: String,
    pub name: String,
    pub avatarType: String,
    pub path: String,
    pub metadata: HashMap<String, String>,
}

pub struct AvatarInstanceSettings;

pub struct AvatarSettings;

pub trait AvatarPersistenceDelegate {}

pub struct DragonBonesPersistenceDelegate;
pub struct WebPPersistenceDelegate;
pub struct Mp4PersistenceDelegate;
pub struct MmdPersistenceDelegate;
pub struct GltfPersistenceDelegate;
pub struct FbxPersistenceDelegate;

pub struct AvatarRepository;
