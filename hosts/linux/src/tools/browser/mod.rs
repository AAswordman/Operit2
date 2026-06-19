#![allow(non_snake_case)]

use std::env;
use std::sync::Arc;

use operit_host_api::{
    BrowserAutomationHost, BrowserAutomationRequest, BrowserAutomationResponse, HostError,
    HostResult, WebVisitHost, WebVisitRequest, WebVisitResult,
};

use crate::chromium_browser::{visit_with_chromium, ChromiumBrowserAutomationHost};

pub struct LinuxWebVisitHost;

impl LinuxWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxWebVisitHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WebVisitHost for LinuxWebVisitHost {
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult> {
        visit_with_chromium(
            request,
            browser_candidates(),
            &["--no-sandbox", "--disable-dev-shm-usage"],
        )
    }
}

pub struct LinuxBrowserAutomationHost(ChromiumBrowserAutomationHost);

impl LinuxBrowserAutomationHost {
    pub fn new() -> Self {
        Self(ChromiumBrowserAutomationHost::new(
            browser_candidates(),
            vec!["--no-sandbox".to_string(), "--disable-dev-shm-usage".to_string()],
        ))
    }
}

impl Default for LinuxBrowserAutomationHost {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserAutomationHost for LinuxBrowserAutomationHost {
    fn executeBrowserTool(
        &self,
        request: BrowserAutomationRequest,
    ) -> HostResult<BrowserAutomationResponse> {
        self.0.executeBrowserTool(request)
    }
}

fn browser_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(path) = env::var("OPERIT_BROWSER_PATH") {
        if !path.trim().is_empty() {
            candidates.push(path);
        }
    }
    candidates.extend(
        [
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/usr/bin/microsoft-edge",
        ]
        .into_iter()
        .map(str::to_string),
    );
    candidates
}
