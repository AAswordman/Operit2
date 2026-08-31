#[derive(Clone, Copy)]
pub struct BundledExternalSkillAsset {
    pub skill_name: &'static str,
    pub path: &'static str,
    pub bytes: &'static [u8],
}

pub static BUNDLED_EXTERNAL_SKILL_ASSETS: &[BundledExternalSkillAsset] = &[
];
