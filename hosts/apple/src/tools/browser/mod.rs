use std::env;

use operit_host_api::{
    BrowserAutomationHost, BrowserAutomationRequest, BrowserAutomationResponse, HostResult,
    WebVisitHost, WebVisitRequest, WebVisitResult,
};

use crate::chromium_browser::{visit_with_chromium, ChromiumBrowserAutomationHost};

/// Visits web pages through a Chromium browser on macOS.
pub struct AppleWebVisitHost;

impl AppleWebVisitHost {
    /// Creates a macOS Chromium web visit host.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AppleWebVisitHost {
    /// Creates the default macOS Chromium web visit host.
    fn default() -> Self {
        Self::new()
    }
}

impl WebVisitHost for AppleWebVisitHost {
    /// Visits a web page through a temporary Chromium session.
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult> {
        visit_with_chromium(request, browser_candidates(), &[])
    }
}

/// Automates a Chromium browser session on macOS.
pub struct AppleBrowserAutomationHost(ChromiumBrowserAutomationHost);

impl AppleBrowserAutomationHost {
    /// Creates a macOS Chromium browser automation host.
    pub fn new() -> Self {
        Self(ChromiumBrowserAutomationHost::new(
            browser_candidates(),
            Vec::new(),
        ))
    }
}

impl Default for AppleBrowserAutomationHost {
    /// Creates the default macOS Chromium browser automation host.
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserAutomationHost for AppleBrowserAutomationHost {
    /// Executes a browser automation request in the active Chromium session.
    fn executeBrowserTool(
        &self,
        request: BrowserAutomationRequest,
    ) -> HostResult<BrowserAutomationResponse> {
        self.0.executeBrowserTool(request)
    }
}

/// Returns executable paths accepted for the macOS Chromium browser host.
fn browser_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(path) = env::var("OPERIT_BROWSER_PATH") {
        if !path.trim().is_empty() {
            candidates.push(path);
        }
    }
    candidates.extend(
        [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
        .into_iter()
        .map(str::to_string),
    );
    candidates
}
