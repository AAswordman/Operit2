#[path = "R.rs"]
pub mod R;
pub mod core;
pub mod data;
pub mod plugins;
pub mod services;
pub mod ui;

pub use operit_providers::chat::EnhancedAIService::EnhancedAIService;
pub use core::chat::AIMessageManager::AIMessageManager;
