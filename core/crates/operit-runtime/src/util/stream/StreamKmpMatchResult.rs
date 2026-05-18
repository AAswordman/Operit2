use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StreamKmpMatchResult {
    NoMatch,
    InProgress,
    Match {
        groups: BTreeMap<i32, String>,
        is_full_match: bool,
    },
}

impl StreamKmpMatchResult {
    pub fn is_match(&self) -> bool {
        matches!(self, StreamKmpMatchResult::Match { .. })
    }

    pub fn groups(&self) -> BTreeMap<i32, String> {
        match self {
            StreamKmpMatchResult::Match { groups, .. } => groups.clone(),
            _ => BTreeMap::new(),
        }
    }

    pub fn is_full_match(&self) -> bool {
        match self {
            StreamKmpMatchResult::Match { is_full_match, .. } => *is_full_match,
            _ => false,
        }
    }
}
