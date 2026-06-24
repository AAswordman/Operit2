use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDisplayWindowState {
    pub hasOlderDisplayHistory: bool,
    pub hasNewerDisplayHistory: bool,
    pub isLoadingDisplayWindow: bool,
}
