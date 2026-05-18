use std::collections::{HashMap, HashSet};

pub struct ResolvedCharacterCardToolAccess {
    pub customEnabled: bool,
    pub effectiveBuiltinToolVisibility: HashMap<String, bool>,
    pub allowedPackageNames: HashSet<String>,
    pub allowedSkillNames: HashSet<String>,
    pub allowedMcpServerNames: HashSet<String>,
    pub canUsePackageSystem: bool,
    pub hasAnyAllowedExternalSource: bool,
}

pub struct CharacterCardToolAccessResolver;
