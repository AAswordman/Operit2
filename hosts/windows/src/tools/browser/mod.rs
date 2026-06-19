#![allow(non_snake_case)]

use std::env;
use std::sync::Arc;

use operit_host_api::{
    BrowserAutomationHost, BrowserAutomationRequest, BrowserAutomationResponse, HostError,
    HostResult, WebVisitHost, WebVisitRequest, WebVisitResult,
};

use crate::chromium_browser::{visit_with_chromium, ChromiumBrowserAutomationHost};

pub struct WindowsWebVisitHost;

impl WindowsWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsWebVisitHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WebVisitHost for WindowsWebVisitHost {
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult> {
        visit_with_chromium(request, browser_candidates(), &[])
    }
}

pub struct WindowsBrowserAutomationHost(ChromiumBrowserAutomationHost);

impl WindowsBrowserAutomationHost {
    pub fn new() -> Self {
        Self(ChromiumBrowserAutomationHost::new(
            browser_candidates(),
            Vec::new(),
        ))
    }
}

impl Default for WindowsBrowserAutomationHost {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserAutomationHost for WindowsBrowserAutomationHost {
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
            "msedge.exe",
            "chrome.exe",
            "chromium.exe",
            "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
            "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
            "C:/Program Files/Google/Chrome/Application/chrome.exe",
            "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
        ]
        .into_iter()
        .map(str::to_string),
    );
    candidates
}
