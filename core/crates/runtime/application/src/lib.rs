#[path = "R.rs"]
pub mod R;
pub mod core;
pub mod data;
pub mod plugins;
pub mod services;
pub mod ui;

/// Exposes the runtime package version used by CoreNode compatibility checks.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use core::chat::AIMessageManager::AIMessageManager;
pub use operit_providers::chat::EnhancedAIService::EnhancedAIService;
