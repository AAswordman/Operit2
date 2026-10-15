#![allow(non_snake_case)]

use operit_host_api::HostManager::HostManager;
use operit_host_api::HttpRequestData;
use operit_host_api::TimeUtils::currentTimeMillis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;

use crate::data::preferences::PreferenceStorageManager::PreferenceStorageManager;

const POINTER_URL: &str = "https://operit.app/announcements/latest.json";
const PREFERENCES: &str = "remote_announcement_preferences";
const ACKNOWLEDGED_VERSION: &str = "acknowledged_version";

/// Fetches public announcements through the runtime HTTP and storage hosts.
pub struct RemoteAnnouncementService {
    context: HostManager,
}

#[derive(Deserialize)]
struct AnnouncementPointer {
    schemaVersion: i32,
    latestFile: String,
}

#[derive(Deserialize)]
struct AnnouncementPayload {
    schemaVersion: i32,
    id: String,
    version: i32,
    enabled: bool,
    countdownSec: i32,
    content: AnnouncementContent,
}

#[derive(Deserialize)]
struct AnnouncementContent {
    defaultLocale: String,
    locales: BTreeMap<String, AnnouncementLocaleContent>,
}

#[derive(Deserialize)]
struct AnnouncementLocaleContent {
    title: String,
    body: String,
    acknowledge: String,
}

/// Contains the published content of one unacknowledged announcement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteAnnouncementDisplay {
    pub id: String,
    pub version: i32,
    pub title: String,
    pub body: String,
    pub acknowledgeText: String,
    pub countdownSec: i32,
}

impl RemoteAnnouncementService {
    /// Creates an announcement service using the active runtime host context.
    pub fn getInstance(context: &HostManager) -> Self {
        Self {
            context: context.clone(),
        }
    }

    /// Fetches the published announcement for every user without audience filtering.
    pub fn fetchDisplayableAnnouncement(
        &self,
    ) -> Result<Option<RemoteAnnouncementDisplay>, String> {
        let pointerUrl = Url::parse(POINTER_URL).map_err(|error| error.to_string())?;
        let pointer: AnnouncementPointer = serde_json::from_slice(&self.fetch(&pointerUrl)?)
            .map_err(|error| format!("Invalid announcement pointer: {error}"))?;
        if pointer.schemaVersion != 1 {
            return Err(format!(
                "Unsupported announcement pointer schema: {}",
                pointer.schemaVersion
            ));
        }
        if pointer.latestFile.trim().is_empty() {
            return Ok(None);
        }
        let payloadUrl = pointerUrl
            .join(pointer.latestFile.trim())
            .map_err(|error| error.to_string())?;
        let payload: AnnouncementPayload = serde_json::from_slice(&self.fetch(&payloadUrl)?)
            .map_err(|error| format!("Invalid announcement payload: {error}"))?;
        let Some(display) = prepareAnnouncement(payload)? else {
            return Ok(None);
        };
        let acknowledged = PreferenceStorageManager::getInstance()
            .getPreference(PREFERENCES, ACKNOWLEDGED_VERSION)
            .map_err(|error| error.to_string())?;
        if let Some(value) = acknowledged {
            let version = serde_json::from_value::<i32>(value)
                .map_err(|error| format!("Invalid acknowledged announcement version: {error}"))?;
            if display.version <= version {
                return Ok(None);
            }
        }
        Ok(Some(display))
    }

    /// Persists explicit confirmation without decreasing the acknowledged version.
    pub fn acknowledgeAnnouncement(&self, version: i32) -> Result<(), String> {
        if version <= 0 {
            return Err("Announcement version must be positive".to_string());
        }
        let preferences = PreferenceStorageManager::getInstance();
        let current = preferences
            .getPreference(PREFERENCES, ACKNOWLEDGED_VERSION)
            .map_err(|error| error.to_string())?;
        if let Some(value) = current {
            let acknowledged = serde_json::from_value::<i32>(value).map_err(|error| error.to_string())?;
            if acknowledged >= version {
                return Ok(());
            }
        }
        preferences
            .setPreference(PREFERENCES, ACKNOWLEDGED_VERSION, serde_json::json!(version))
            .map_err(|error| error.to_string())
    }

    /// Loads fresh JSON using the same host API on every runtime platform.
    fn fetch(&self, url: &Url) -> Result<Vec<u8>, String> {
        if !matches!(url.scheme(), "https" | "http") {
            return Err("Announcement URL must use HTTP or HTTPS".to_string());
        }
        let mut url = url.clone();
        url.query_pairs_mut()
            .append_pair("ts", &currentTimeMillis().to_string());
        let host = self
            .context
            .httpHost
            .as_deref()
            .ok_or("HttpHost is required for remote announcements")?;
        let response = host
            .executeHttpRequest(HttpRequestData {
                url: url.to_string(),
                method: "GET".to_string(),
                headers: Vec::new(),
                body: Vec::new(),
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 8,
                readTimeoutSeconds: 8,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| format!("Announcement request failed: {error}"))?;
        if !(200..300).contains(&response.statusCode) {
            return Err(format!(
                "Announcement request failed: HTTP {}",
                response.statusCode
            ));
        }
        Ok(response.body)
    }
}

/// Uses the publisher's declared content directly, with no audience or locale selection.
fn prepareAnnouncement(
    payload: AnnouncementPayload,
) -> Result<Option<RemoteAnnouncementDisplay>, String> {
    if payload.schemaVersion != 1 {
        return Err(format!(
            "Unsupported announcement schema: {}",
            payload.schemaVersion
        ));
    }
    if !payload.enabled {
        return Ok(None);
    }
    if payload.version <= 0 {
        return Err("Announcement version must be positive".to_string());
    }
    let mut locales = payload.content.locales;
    let content = locales
        .remove(&payload.content.defaultLocale)
        .ok_or_else(|| {
            format!(
                "Missing published announcement locale: {}",
                payload.content.defaultLocale
            )
        })?;
    if payload.id.trim().is_empty()
        || content.title.trim().is_empty()
        || content.body.trim().is_empty()
        || content.acknowledge.trim().is_empty()
    {
        return Err(
            "Announcement id, title, body and acknowledgement must not be blank".to_string(),
        );
    }
    Ok(Some(RemoteAnnouncementDisplay {
        id: payload.id,
        version: payload.version,
        title: content.title,
        body: content.body,
        acknowledgeText: content.acknowledge,
        countdownSec: payload.countdownSec.clamp(0, 30),
    }))
}
