pub struct AIForegroundService;

impl AIForegroundService {
    pub const STATE_IDLE: &'static str = "idle";
    pub const STATE_RUNNING: &'static str = "running";
    pub const EXTRA_STATE: &'static str = "state";
    pub const EXTRA_CHARACTER_NAME: &'static str = "character_name";
    pub const EXTRA_AVATAR_URI: &'static str = "avatar_uri";

    pub fn notify_reply_completed() {}
}
