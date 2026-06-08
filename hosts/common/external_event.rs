#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::{
    ExternalRuntimeEvent, ExternalRuntimeEventBusConfig, ExternalRuntimeEventHandler,
    ExternalRuntimeEventHost, ExternalRuntimeEventRegistration, ExternalRuntimeEventResponse,
    ExternalRuntimeRegistrationDescriptor, HostError, HostResult,
};
use uuid::Uuid;

#[derive(Clone, Debug, Default)]
pub struct LocalExternalRuntimeEventHost;

impl LocalExternalRuntimeEventHost {
    pub fn new() -> Self {
        Self
    }
}

impl ExternalRuntimeEventHost for LocalExternalRuntimeEventHost {
    fn externalRuntimeEventRegistryDir(&self) -> HostResult<PathBuf> {
        Ok(externalRuntimeEventRegistryDir())
    }

    fn startExternalRuntimeEventBus(
        &self,
        config: ExternalRuntimeEventBusConfig,
        handler: Arc<ExternalRuntimeEventHandler>,
    ) -> HostResult<Box<dyn ExternalRuntimeEventRegistration>> {
        startExternalRuntimeEventBus(config, handler)
            .map(|registration| Box::new(registration) as Box<dyn ExternalRuntimeEventRegistration>)
    }
}

pub struct LocalExternalRuntimeEventRegistration {
    runtimeId: String,
    descriptorPath: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ExternalRuntimeEventRegistration for LocalExternalRuntimeEventRegistration {
    fn runtimeId(&self) -> &str {
        &self.runtimeId
    }
}

impl Drop for LocalExternalRuntimeEventRegistration {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        let _ = fs::remove_file(&self.descriptorPath);
    }
}

pub fn externalRuntimeEventRegistryDir() -> PathBuf {
    std::env::var_os("OPERIT_EXTERNAL_EVENT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir()
                .join("operit2")
                .join("external-runtime-events")
        })
}

pub fn startExternalRuntimeEventBus(
    config: ExternalRuntimeEventBusConfig,
    handler: Arc<ExternalRuntimeEventHandler>,
) -> HostResult<LocalExternalRuntimeEventRegistration> {
    let registryDir = externalRuntimeEventRegistryDir();
    let registrationsDir = registryDir.join("registrations");
    let inboxesDir = registryDir.join("inboxes");
    createPrivateDirectory(&registryDir)?;
    createPrivateDirectory(&registrationsDir)?;
    createPrivateDirectory(&inboxesDir)?;

    let runtimeId = Uuid::new_v4().to_string();
    let inboxDir = inboxesDir.join(&runtimeId);
    let eventsDir = inboxDir.join("events");
    let responsesDir = inboxDir.join("responses");
    createPrivateDirectory(&inboxDir)?;
    createPrivateDirectory(&eventsDir)?;
    createPrivateDirectory(&responsesDir)?;

    let descriptor = ExternalRuntimeRegistrationDescriptor {
        protocolVersion: 1,
        runtimeId: runtimeId.clone(),
        processId: std::process::id(),
        processKind: config.processKind.clone(),
        inboxDir: normalizePath(&inboxDir),
        eventsDir: normalizePath(&eventsDir),
        responsesDir: normalizePath(&responsesDir),
        capabilities: config.capabilities.clone(),
        createdAtMillis: currentTimeMillis()?,
    };
    let descriptorPath = registrationsDir.join(format!("{runtimeId}.json"));
    writeJsonFile(&descriptorPath, &descriptor)?;

    let stop = Arc::new(AtomicBool::new(false));
    let threadStop = stop.clone();
    let threadRuntimeId = runtimeId.clone();
    let threadCapabilities = config.capabilities.clone();
    let threadPollInterval = config.pollInterval;
    let threadEventsDir = eventsDir.clone();
    let threadResponsesDir = responsesDir.clone();
    let thread = thread::spawn(move || {
        while !threadStop.load(Ordering::SeqCst) {
            if let Err(error) = processExternalRuntimeEvents(
                &threadRuntimeId,
                &threadCapabilities,
                &threadEventsDir,
                &threadResponsesDir,
                &handler,
            ) {
                eprintln!("external runtime event error: {}", error.message);
            }
            thread::sleep(threadPollInterval);
        }
    });

    Ok(LocalExternalRuntimeEventRegistration {
        runtimeId,
        descriptorPath,
        stop,
        thread: Some(thread),
    })
}

fn processExternalRuntimeEvents(
    runtimeId: &str,
    capabilities: &[String],
    eventsDir: &Path,
    responsesDir: &Path,
    handler: &Arc<ExternalRuntimeEventHandler>,
) -> HostResult<()> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(eventsDir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            entries.push(path);
        }
    }
    entries.sort();
    for eventPath in entries {
        if let Err(error) = processExternalRuntimeEventFile(
            runtimeId,
            capabilities,
            &eventPath,
            responsesDir,
            handler,
        ) {
            eprintln!(
                "external runtime event file error: path={}, error={}",
                eventPath.to_string_lossy(),
                error.message
            );
        }
    }
    Ok(())
}

fn processExternalRuntimeEventFile(
    runtimeId: &str,
    capabilities: &[String],
    eventPath: &Path,
    responsesDir: &Path,
    handler: &Arc<ExternalRuntimeEventHandler>,
) -> HostResult<()> {
    let eventText = fs::read_to_string(eventPath)?;
    let event: ExternalRuntimeEvent =
        serde_json::from_str(&eventText).map_err(|error| HostError::new(error.to_string()))?;
    let responseFileName = eventPath
        .file_name()
        .ok_or_else(|| HostError::new("external runtime event path has no file name"))?;
    let responsePath = responsesDir.join(responseFileName);
    let response = if capabilities
        .iter()
        .any(|capability| capability == &event.name)
    {
        match handler(event.clone()) {
            Ok(result) => ExternalRuntimeEventResponse {
                eventId: event.id,
                runtimeId: runtimeId.to_string(),
                accepted: true,
                result: Some(result),
                error: None,
                handledAtMillis: currentTimeMillis()?,
            },
            Err(error) => ExternalRuntimeEventResponse {
                eventId: event.id,
                runtimeId: runtimeId.to_string(),
                accepted: false,
                result: None,
                error: Some(error.message),
                handledAtMillis: currentTimeMillis()?,
            },
        }
    } else {
        ExternalRuntimeEventResponse {
            eventId: event.id,
            runtimeId: runtimeId.to_string(),
            accepted: false,
            result: None,
            error: Some(format!("unsupported external runtime event: {}", event.name)),
            handledAtMillis: currentTimeMillis()?,
        }
    };
    writeJsonFile(&responsePath, &response)?;
    fs::remove_file(eventPath)?;
    Ok(())
}

fn createPrivateDirectory(path: &Path) -> HostResult<()> {
    fs::create_dir_all(path)?;
    setPrivateDirectoryPermissions(path)
}

#[cfg(unix)]
fn setPrivateDirectoryPermissions(path: &Path) -> HostResult<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn setPrivateDirectoryPermissions(_path: &Path) -> HostResult<()> {
    Ok(())
}

fn writeJsonFile<T: serde::Serialize>(path: &Path, value: &T) -> HostResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| HostError::new(error.to_string()))?;
    fs::write(path, bytes)?;
    setPrivateFilePermissions(path)
}

#[cfg(unix)]
fn setPrivateFilePermissions(path: &Path) -> HostResult<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn setPrivateFilePermissions(_path: &Path) -> HostResult<()> {
    Ok(())
}

fn normalizePath(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn currentTimeMillis() -> HostResult<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| HostError::new(error.to_string()))?;
    Ok(duration.as_millis() as u64)
}
