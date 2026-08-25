pub enum ModelType {
    DRAGON_BONES,
    WEBP,
    MP4,
    MMD,
    GLTF,
    FBX,
}

pub struct DragonBonesModel {
    pub id: String,
    pub name: String,
}

pub struct DragonBonesConfig {
    pub models: Vec<DragonBonesModel>,
}
