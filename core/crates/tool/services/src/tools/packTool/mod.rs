#[path = "PackageDebugRefreshReceiver.rs"]
pub mod PackageDebugRefreshReceiver;

#[path = "RuntimePackageManager.rs"]
pub mod RuntimePackageManager;

#[path = "ToolPkgDebugInstallReceiver.rs"]
pub mod ToolPkgDebugInstallReceiver;

#[path = "ToolPkgDesktopWidgetService.rs"]
mod ToolPkgDesktopWidgetService;

/// Exposes typed public-API owner discovery metadata without defining a provider framework.
pub mod ToolPkgPublicApiUiCatalog;
