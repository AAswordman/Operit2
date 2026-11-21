use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use operit_host_api::ProfileTerminalHost::ProfileTerminalHost;
use operit_host_api::RuntimeStorageLayout::{
    runtimeStorageOwnership, RuntimeStorageOwnership, TERMINAL_PROFILES_PATH,
};
use operit_host_api::*;

#[derive(Default)]
struct MemoryStorage(Mutex<BTreeMap<String, Vec<u8>>>, AtomicBool);

impl RuntimeStorageHost for MemoryStorage {
    /// Keeps the fixture independent of native filesystem paths.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        None
    }
    /// Keeps the fixture independent of native workspaces.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        None
    }
    /// Reads exactly the requested durable record.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        self.0
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| HostError::new("missing record"))
    }
    /// Writes the complete record in one fixture transaction.
    fn writeBytes(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        self.0.lock().unwrap().insert(path.into(), bytes.into());
        Ok(())
    }
    /// Models a durable atomic replacement.
    fn writeBytesAtomically(&self, path: &str, bytes: &[u8]) -> HostResult<()> {
        if self.1.load(Ordering::Relaxed) {
            return Err(HostError::new("Profile storage is full"));
        }
        self.writeBytes(path, bytes)
    }
    /// Rejects append operations outside this registry's storage contract.
    fn appendBytes(&self, _: &str, _: &[u8]) -> HostResult<()> {
        panic!("unexpected append")
    }
    /// Models the in-memory store's completed flush.
    fn flushFile(&self, _: &str) -> HostResult<()> {
        Ok(())
    }
    /// Rejects destructive operations outside this registry's storage contract.
    fn delete(&self, _: &str, _: bool) -> HostResult<()> {
        panic!("unexpected delete")
    }
    /// Checks whether profiles have ever been persisted on this device.
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.0.lock().unwrap().contains_key(path))
    }
    /// Rejects storage scans; the registry must read its exact declared path.
    fn list(&self, _: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        panic!("unexpected scan")
    }
}

static NEXT_SESSION: AtomicUsize = AtomicUsize::new(1);

#[derive(Default)]
struct TestTerminal {
    rootfs: String,
    sessions: Mutex<BTreeMap<String, TerminalSessionListEntry>>,
    installations: Arc<Mutex<std::collections::BTreeSet<String>>>,
}

impl TerminalHost for TestTerminal {
    /// Publishes a deterministic fixture instance without performing network or filesystem IO.
    fn installTerminalDistribution(
        &self,
        _: &str,
        instanceId: &str,
        name: &str,
        _: HttpDownloadControl,
        _: TerminalInstallProgressCallback,
    ) -> HostResult<TerminalProfile> {
        assert!(self.installations.lock().unwrap().insert(instanceId.into()));
        let mut installed = profile("/downloaded-rootfs");
        installed.id = instanceId.into();
        installed.name = name.into();
        installed.terminalType = "shell".into();
        Ok(installed)
    }
    /// Records rollback so storage failures cannot leave an unregistered installed instance.
    fn discardTerminalDistribution(&self, instanceId: &str) -> HostResult<()> {
        assert!(self.installations.lock().unwrap().remove(instanceId));
        Ok(())
    }
    /// Exposes two implementations sharing one interpreter to exercise exact backend selection.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "test".into(),
            terminal: "native".into(),
            terminalType: "shell".into(),
            types: ["native", "alternate"]
                .into_iter()
                .map(|terminal| TerminalTypeInfo {
                    terminal: terminal.into(),
                    terminalType: "shell".into(),
                    available: true,
                    description: terminal.into(),
                })
                .collect(),
        })
    }
    /// Declares one host-owned field for custom environment tests.
    fn terminalProfileSupport(&self) -> HostResult<Vec<TerminalProfileSupport>> {
        Ok(self
            .terminalInfo()?
            .types
            .into_iter()
            .map(|entry| TerminalProfileSupport {
                terminal: entry.terminal,
                terminalType: entry.terminalType,
                fields: vec!["rootfs".into()],
            })
            .collect())
    }
    /// Creates an isolated environment with an immutable launch value.
    fn configuredTerminal(&self, profile: &TerminalProfile) -> HostResult<Arc<dyn TerminalHost>> {
        Ok(Arc::new(Self {
            rootfs: profile.options["rootfs"].clone(),
            sessions: Mutex::default(),
            installations: self.installations.clone(),
        }))
    }
    /// Labels directory resolution with the exact configured environment.
    fn resolveWorkingDirectory(
        &self,
        _: &str,
        _: &str,
        directory: &str,
        resolve: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        resolve(directory).map(|directory| format!("{}{directory}", self.rootfs))
    }
    /// Allocates a distinct session ID and records the selected implementation.
    fn startPtySession(
        &self,
        name: &str,
        terminal: &str,
        kind: &str,
        directory: &str,
        _: u16,
        _: u16,
    ) -> HostResult<String> {
        let id = format!("session-{}", NEXT_SESSION.fetch_add(1, Ordering::Relaxed));
        self.sessions.lock().unwrap().insert(
            id.clone(),
            TerminalSessionListEntry {
                sessionId: id.clone(),
                sessionName: name.into(),
                platform: "test".into(),
                terminal: terminal.into(),
                terminalType: kind.into(),
                sessionKind: "pty".into(),
                workingDir: directory.into(),
                commandRunning: false,
            },
        );
        Ok(id)
    }
    /// Returns a value identifying the environment that owns this session.
    fn readPtySession(&self, id: &str) -> HostResult<Vec<u8>> {
        assert!(self.sessions.lock().unwrap().contains_key(id));
        Ok(self.rootfs.as_bytes().to_vec())
    }
    /// Counts accepted bytes for retained-owner routing checks.
    fn writePtySession(&self, _: &str, bytes: &[u8]) -> HostResult<usize> {
        Ok(bytes.len())
    }
    /// Accepts resize requests in the fixture.
    fn resizePtySession(&self, _: &str, _: u16, _: u16) -> HostResult<()> {
        Ok(())
    }
    /// Keeps test sessions running until explicitly closed.
    fn pollPtyExitCode(&self, _: &str) -> HostResult<Option<i32>> {
        Ok(None)
    }
    /// Rejects duplicate closes, matching strict production hosts.
    fn closePtySession(&self, id: &str) -> HostResult<()> {
        self.sessions
            .lock()
            .unwrap()
            .remove(id)
            .ok_or_else(|| HostError::new("session already closed"))?;
        Ok(())
    }
    /// Lists sessions belonging to this exact environment.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        Ok(self.sessions.lock().unwrap().values().cloned().collect())
    }
    /// Starts the fixture's canonical native interpreter.
    fn createOrGetSession(&self, name: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(name, "shell")
    }
    /// Starts the canonical implementation for a typed request.
    fn createOrGetTypedSession(&self, name: &str, kind: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTerminalSession(name, "native", kind)
    }
    /// Reuses a name only inside the same immutable environment and exact implementation.
    fn createOrGetTerminalSession(
        &self,
        name: &str,
        terminal: &str,
        kind: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let existing = self
            .listSessions()?
            .into_iter()
            .find(|entry| entry.sessionName == name && entry.terminal == terminal);
        let (id, is_new) = match existing {
            Some(entry) => (entry.sessionId, false),
            None => (
                self.startPtySession(name, terminal, kind, "/", 24, 80)?,
                true,
            ),
        };
        Ok(TerminalSessionInfo {
            sessionId: id,
            sessionName: name.into(),
            platform: "test".into(),
            terminal: terminal.into(),
            terminalType: kind.into(),
            isNewSession: is_new,
        })
    }
    /// Returns the environment identity as command output.
    fn executeInSession(
        &self,
        id: &str,
        command: &str,
        _: u64,
    ) -> HostResult<TerminalCommandOutput> {
        let entry = self.sessions.lock().unwrap()[id].clone();
        Ok(TerminalCommandOutput {
            command: command.into(),
            output: self.rootfs.clone(),
            exitCode: 0,
            sessionId: id.into(),
            platform: entry.platform,
            terminal: entry.terminal,
            terminalType: entry.terminalType,
            timedOut: false,
        })
    }
    /// Rejects bypassing the configured profile through the raw host's default hidden executor.
    fn executeHiddenCommand(
        &self,
        _: &str,
        _: &str,
        _: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        panic!("profile default was bypassed")
    }
    /// Accepts fixture input without changing the owning environment.
    fn inputInSession(
        &self,
        id: &str,
        _: Option<&str>,
        _: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        Ok(TerminalInputOutput {
            sessionId: id.into(),
            acceptedChars: 0,
        })
    }
    /// Closes a named fixture session using the same strict release behavior.
    fn closeSession(&self, id: &str) -> HostResult<TerminalCloseOutput> {
        self.closePtySession(id)?;
        Ok(TerminalCloseOutput {
            sessionId: id.into(),
            success: true,
            message: "closed".into(),
        })
    }
    /// Rejects unused screen queries in these profile tests.
    fn getSessionScreen(&self, _: &str) -> HostResult<TerminalScreenOutput> {
        panic!("unexpected screen query")
    }
}

/// Creates a custom environment profile with stable public interpreter identities.
fn profile(rootfs: &str) -> TerminalProfile {
    TerminalProfile {
        id: "ubuntu".into(),
        name: "Ubuntu".into(),
        terminal: "native".into(),
        terminalType: "shell".into(),
        options: BTreeMap::from([("rootfs".into(), rootfs.into())]),
    }
}

/// Exercises real registry routing and persistence across edits, deletion, and restart.
#[test]
fn changed_and_removed_profiles_retain_session_owners() {
    let storage = Arc::new(MemoryStorage::default());
    let registry = ProfileTerminalHost::new(Arc::new(TestTerminal::default()), storage.clone());
    let mut original = registry.terminalProfileSettings().unwrap();
    let mut settings = original.clone();
    settings.profiles.push(profile("/ubuntu-a"));
    settings.defaultProfileId = "ubuntu".into();
    registry
        .saveTerminalProfileSettings(settings.clone())
        .unwrap();
    settings = registry.terminalProfileSettings().unwrap();
    let a = registry.createOrGetSession("agent").unwrap();
    assert_eq!(
        registry
            .executeHiddenCommand("pwd", "agent", 100)
            .unwrap()
            .output,
        "/ubuntu-a"
    );
    settings
        .profiles
        .last_mut()
        .unwrap()
        .options
        .insert("rootfs".into(), "/ubuntu-b".into());
    registry
        .saveTerminalProfileSettings(settings.clone())
        .unwrap();
    settings = registry.terminalProfileSettings().unwrap();
    let b = registry.createOrGetSession("agent").unwrap();
    assert_ne!(a.sessionId, b.sessionId);
    assert_eq!(registry.readPtySession(&a.sessionId).unwrap(), b"/ubuntu-a");
    assert_eq!(registry.readPtySession(&b.sessionId).unwrap(), b"/ubuntu-b");
    let reopened = ProfileTerminalHost::new(Arc::new(TestTerminal::default()), storage.clone());
    assert_eq!(reopened.terminalProfileSettings().unwrap(), settings);
    original.revision = settings.revision;
    registry.saveTerminalProfileSettings(original).unwrap();
    assert_eq!(registry.readPtySession(&a.sessionId).unwrap(), b"/ubuntu-a");
    assert_eq!(registry.writePtySession(&b.sessionId, b"exit").unwrap(), 4);
    registry.closePtySession(&a.sessionId).unwrap();
    assert!(registry.closePtySession(&a.sessionId).is_err());
    assert_eq!(
        runtimeStorageOwnership(TERMINAL_PROFILES_PATH).unwrap(),
        RuntimeStorageOwnership::CoreNode
    );
}

/// Verifies exact implementation selection for defaults sharing the shell interpreter.
#[test]
fn default_selection_preserves_explicit_backend() {
    let registry = ProfileTerminalHost::new(
        Arc::new(TestTerminal::default()),
        Arc::new(MemoryStorage::default()),
    );
    let mut settings = registry.terminalProfileSettings().unwrap();
    settings.defaultProfileId = settings.profiles[1].id.clone();
    registry.saveTerminalProfileSettings(settings).unwrap();
    assert_eq!(
        registry.terminalInfo().unwrap().types[0].terminal,
        "alternate"
    );
    assert_eq!(
        registry
            .createOrGetTypedSession("agent", "shell")
            .unwrap()
            .terminal,
        "alternate"
    );
    assert_eq!(
        registry
            .executeHiddenCommand("pwd", "hidden", 100)
            .unwrap()
            .terminal,
        "alternate"
    );
}

/// Invalid profiles must neither replace durable bytes nor silently select another default.
#[test]
fn invalid_registry_does_not_overwrite_saved_configuration() {
    let storage = Arc::new(MemoryStorage::default());
    let registry = ProfileTerminalHost::new(Arc::new(TestTerminal::default()), storage.clone());
    let mut original = registry.terminalProfileSettings().unwrap();
    registry
        .saveTerminalProfileSettings(original.clone())
        .unwrap();
    original = registry.terminalProfileSettings().unwrap();
    let bytes = storage.readBytes(TERMINAL_PROFILES_PATH).unwrap();
    let mut invalid = original.clone();
    invalid.defaultProfileId = "missing".into();
    assert!(registry.saveTerminalProfileSettings(invalid).is_err());
    let mut invalid = original.clone();
    invalid.profiles[0]
        .options
        .insert("unsupported".into(), "value".into());
    assert!(registry.saveTerminalProfileSettings(invalid).is_err());
    assert_eq!(registry.terminalProfileSettings().unwrap(), original);
    assert_eq!(storage.readBytes(TERMINAL_PROFILES_PATH).unwrap(), bytes);
    storage
        .writeBytes(TERMINAL_PROFILES_PATH, b"{broken")
        .unwrap();
    let reopened = ProfileTerminalHost::new(Arc::new(TestTerminal::default()), storage);
    assert!(reopened.terminalProfileSettings().is_err());
}

/// Rejects stale saves instead of deleting profiles added by another editor or installer.
#[test]
fn stale_registry_cannot_overwrite_newer_profiles() {
    let registry = ProfileTerminalHost::new(
        Arc::new(TestTerminal::default()),
        Arc::new(MemoryStorage::default()),
    );
    let stale = registry.terminalProfileSettings().unwrap();
    let mut changed = stale.clone();
    changed.profiles.push(profile("/ubuntu-a"));
    registry.saveTerminalProfileSettings(changed).unwrap();
    assert!(registry
        .saveTerminalProfileSettings(stale)
        .unwrap_err()
        .message
        .contains("reload"));
    let current = registry.terminalProfileSettings().unwrap();
    assert_eq!(current.revision, 1);
    assert!(current.profiles.iter().any(|entry| entry.id == "ubuntu"));
}

/// Keeps same-interpreter profile instances isolated even when both use the built-in host.
#[test]
fn same_interpreter_profiles_have_independent_named_sessions() {
    let registry = ProfileTerminalHost::new(
        Arc::new(TestTerminal::default()),
        Arc::new(MemoryStorage::default()),
    );
    let mut settings = registry.terminalProfileSettings().unwrap();
    let first = settings.profiles[0].clone();
    let mut second = first.clone();
    second.id = "second".into();
    second.name = "Second shell".into();
    settings.profiles.push(second);
    registry.saveTerminalProfileSettings(settings).unwrap();
    let a = registry
        .createOrGetProfileSession("agent", &first.id)
        .unwrap();
    let b = registry
        .createOrGetProfileSession("agent", "second")
        .unwrap();
    assert_ne!(a.sessionId, b.sessionId);
    assert_eq!(a.sessionName, "agent");
    assert_eq!(b.sessionName, "agent");
    assert_eq!(
        registry
            .createOrGetProfileSession("agent", &first.id)
            .unwrap()
            .sessionId,
        a.sessionId
    );
    assert!(registry
        .createOrGetProfileSession("agent", "missing")
        .is_err());
}

/// Registers successful downloads and rolls back only the new instance when persistence fails.
#[test]
fn installation_registration_is_transactional_and_does_not_change_default() {
    let host = Arc::new(TestTerminal::default());
    let storage = Arc::new(MemoryStorage::default());
    let registry = ProfileTerminalHost::new(host.clone(), storage.clone());
    let original = registry.terminalProfileSettings().unwrap();
    registry
        .installTerminalDistribution(
            "ubuntu",
            "instance_a",
            "Ubuntu A",
            HttpDownloadControl::new(),
            Arc::new(|_| {}),
        )
        .unwrap();
    let installed = registry.terminalProfileSettings().unwrap();
    assert_eq!(installed.defaultProfileId, original.defaultProfileId);
    assert!(installed
        .profiles
        .iter()
        .any(|entry| entry.id == "instance_a"));
    storage.1.store(true, Ordering::Relaxed);
    let error = registry
        .installTerminalDistribution(
            "ubuntu",
            "instance_b",
            "Ubuntu B",
            HttpDownloadControl::new(),
            Arc::new(|_| {}),
        )
        .unwrap_err();
    assert!(error.message.contains("storage is full"));
    assert_eq!(registry.terminalProfileSettings().unwrap(), installed);
    assert_eq!(
        *host.installations.lock().unwrap(),
        std::collections::BTreeSet::from(["instance_a".to_string()])
    );
}
