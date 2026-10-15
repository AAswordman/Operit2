#[path = "PackageDebugRefreshReceiver.rs"]
pub mod PackageDebugRefreshReceiver;

#[path = "RuntimePackageManager.rs"]
pub mod RuntimePackageManager;

#[path = "ToolPkgDebugInstallReceiver.rs"]
pub mod ToolPkgDebugInstallReceiver;

#[path = "ToolPkgDesktopWidgetService.rs"]
pub mod ToolPkgDesktopWidgetService;

/// Exposes typed public-API owner discovery metadata without defining a provider framework.
pub mod ToolPkgPublicApiUiCatalog;

/// Owns typed Compose DSL sessions over existing automatic stream proxies.
pub mod ToolPkgComposeDslSession;

/// Shares retained typed records between finite widget and terminal renderers.
pub mod ToolPkgComposeDslNodeStore;
