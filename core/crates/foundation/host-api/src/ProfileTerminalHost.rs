use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::RuntimeStorageLayout::TERMINAL_PROFILES_PATH;
use crate::*;

/// Routes new sessions through device-local profiles and retains each session's original host.
pub struct ProfileTerminalHost {
    base: Arc<dyn TerminalHost>,
    storage: Arc<dyn RuntimeStorageHost>,
    state: Mutex<ProfileState>,
}

#[derive(Default)]
struct ProfileState {
    settings: Option<TerminalProfileSettings>,
    hosts: BTreeMap<TerminalProfile, Arc<dyn TerminalHost>>,
    sessions: BTreeMap<String, Arc<dyn TerminalHost>>,
    names: BTreeMap<String, (String, String)>,
}

impl ProfileTerminalHost {
    /// Installs a lazy registry without opening storage or launching terminal processes.
    pub fn new(base: Arc<dyn TerminalHost>, storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self {
            base,
            storage,
            state: Mutex::new(ProfileState::default()),
        }
    }

    /// Acquires the registry state without concealing a poisoned lock.
    fn state(&self) -> HostResult<std::sync::MutexGuard<'_, ProfileState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("Terminal profile registry mutex poisoned"))
    }

    /// Initializes untouched devices from host registrations; persisted errors remain visible.
    fn settings(&self) -> HostResult<TerminalProfileSettings> {
        let mut state = self.state()?;
        if let Some(settings) = &state.settings {
            return Ok(settings.clone());
        }
        let settings = if self.storage.exists(TERMINAL_PROFILES_PATH)? {
            TerminalProfileSettings::fromBytes(&self.storage.readBytes(TERMINAL_PROFILES_PATH)?)?
        } else {
            let info = self.base.terminalInfo()?;
            let profiles: Vec<_> = info
                .types
                .iter()
                .enumerate()
                .map(|(index, entry)| TerminalProfile {
                    id: format!("builtin_{index}"),
                    name: entry.description.clone(),
                    terminal: entry.terminal.clone(),
                    terminalType: entry.terminalType.clone(),
                    options: BTreeMap::new(),
                })
                .collect();
            let default = profiles
                .iter()
                .find(|profile| {
                    profile.terminal == info.terminal && profile.terminalType == info.terminalType
                })
                .ok_or_else(|| HostError::new("Host default terminal is not registered"))?;
            TerminalProfileSettings {
                revision: 0,
                defaultProfileId: default.id.clone(),
                profiles,
            }
        };
        settings.validate()?;
        state.settings = Some(settings.clone());
        Ok(settings)
    }

    /// Resolves explicit profile handles or the configured profile for a canonical implementation.
    fn profile(&self, terminal: &str, terminalType: &str) -> HostResult<TerminalProfile> {
        let settings = self.settings()?;
        if let Some(id) = terminal.strip_prefix("profile:") {
            return settings
                .profiles
                .into_iter()
                .find(|profile| profile.id == id && profile.terminalType == terminalType)
                .ok_or_else(|| {
                    HostError::new(format!(
                        "Terminal profile is not registered: {id}/{terminalType}"
                    ))
                });
        }
        let selected = settings
            .profiles
            .iter()
            .find(|profile| profile.id == settings.defaultProfileId)
            .ok_or_else(|| HostError::new("Default terminal profile is missing"))?;
        if selected.terminal == terminal && selected.terminalType == terminalType {
            return Ok(selected.clone());
        }
        // A canonical implementation outside the selected profile remains an explicit built-in choice.
        if self
            .base
            .terminalInfo()?
            .types
            .iter()
            .any(|entry| entry.terminal == terminal && entry.terminalType == terminalType)
        {
            return Ok(TerminalProfile {
                id: String::new(),
                name: String::new(),
                terminal: terminal.into(),
                terminalType: terminalType.into(),
                options: BTreeMap::new(),
            });
        }
        Err(HostError::new(format!(
            "Terminal is not registered: {terminal}/{terminalType}"
        )))
    }

    /// Selects the configured default when the requested interpreter matches it.
    fn typedProfile(&self, terminalType: &str) -> HostResult<TerminalProfile> {
        let settings = self.settings()?;
        let selected = settings
            .profiles
            .iter()
            .find(|profile| profile.id == settings.defaultProfileId)
            .ok_or_else(|| HostError::new("Default terminal profile is missing"))?;
        if selected.terminalType == terminalType {
            return Ok(selected.clone());
        }
        let info = self.base.terminalInfo()?;
        let entry = info
            .types
            .iter()
            .find(|entry| entry.terminalType == terminalType)
            .ok_or_else(|| {
                HostError::new(format!(
                    "Terminal interpreter is not registered: {terminalType}"
                ))
            })?;
        self.profile(&entry.terminal, terminalType)
    }

    /// Caches immutable launch hosts by their complete profile, preserving old sessions after edits.
    fn host(&self, profile: &TerminalProfile) -> HostResult<Arc<dyn TerminalHost>> {
        if profile.options.is_empty() {
            return Ok(self.base.clone());
        }
        let mut state = self.state()?;
        if let Some(host) = state.hosts.get(profile) {
            return Ok(host.clone());
        }
        let host = self.base.configuredTerminal(profile)?;
        state.hosts.insert(profile.clone(), host.clone());
        Ok(host)
    }

    /// Records ownership independently of whether the host includes exited sessions in its list.
    fn remember(&self, id: &str, host: Arc<dyn TerminalHost>) -> HostResult<()> {
        self.state()?.sessions.insert(id.to_string(), host);
        Ok(())
    }

    /// Looks up a retained owner and reports unknown session IDs explicitly.
    fn session(&self, id: &str) -> HostResult<Arc<dyn TerminalHost>> {
        self.state()?
            .sessions
            .get(id)
            .cloned()
            .ok_or_else(|| HostError::new(format!("Terminal session is not registered: {id}")))
    }

    /// Opens a named session in its exact configured implementation.
    fn create(&self, name: &str, profile: TerminalProfile) -> HostResult<TerminalSessionInfo> {
        let host = self.host(&profile)?;
        let prefix = format!("profile_{}:", profile.id);
        let mut result = host.createOrGetTerminalSession(
            &format!("{prefix}{name}"),
            &profile.terminal,
            &profile.terminalType,
        )?;
        self.remember(&result.sessionId, host)?;
        self.state()?
            .names
            .insert(result.sessionId.clone(), (name.into(), prefix));
        result.sessionName = name.into();
        Ok(result)
    }
}

impl TerminalHost for ProfileTerminalHost {
    /// Returns the host's architecture-compatible downloadable distribution catalog.
    fn terminalDistributions(&self) -> HostResult<Vec<TerminalDistribution>> {
        self.base.terminalDistributions()
    }

    /// Registers a fully installed instance without changing the user's default selection.
    fn installTerminalDistribution(
        &self,
        distributionId: &str,
        instanceId: &str,
        name: &str,
        control: HttpDownloadControl,
        progress: TerminalInstallProgressCallback,
    ) -> HostResult<TerminalProfile> {
        self.settings()?;
        let profile = self.base.installTerminalDistribution(
            distributionId,
            instanceId,
            name,
            control,
            progress,
        )?;
        let registration: HostResult<()> = (|| {
            self.base.configuredTerminal(&profile)?;
            let mut state = self.state()?;
            let mut settings = state
                .settings
                .clone()
                .ok_or_else(|| HostError::new("Terminal settings are missing"))?;
            settings.profiles.push(profile.clone());
            settings.revision = settings
                .revision
                .checked_add(1)
                .ok_or_else(|| HostError::new("Terminal profile revision exhausted"))?;
            settings.validate()?;
            self.storage
                .writeBytesAtomically(TERMINAL_PROFILES_PATH, &settings.toBytes()?)?;
            state.settings = Some(settings);
            Ok(())
        })();
        if let Err(error) = registration {
            return match self.base.discardTerminalDistribution(instanceId) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(HostError::new(format!(
                    "{}; terminal installation cleanup failed: {}",
                    error.message, cleanup.message
                ))),
            };
        }
        Ok(profile)
    }

    /// Resolves a saved profile ID directly rather than guessing an interpreter implementation.
    fn createOrGetProfileSession(
        &self,
        name: &str,
        profileId: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let profile = self
            .settings()?
            .profiles
            .into_iter()
            .find(|entry| entry.id == profileId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal profile is not registered: {profileId}"))
            })?;
        self.create(name, profile)
    }

    /// Exposes editable fields declared by the underlying platform only.
    fn terminalProfileSupport(&self) -> HostResult<Vec<TerminalProfileSupport>> {
        self.base.terminalProfileSupport()
    }
    /// Returns the device-local settings snapshot.
    fn terminalProfileSettings(&self) -> HostResult<TerminalProfileSettings> {
        self.settings()
    }
    /// Validates the whole registry before atomically publishing its durable replacement.
    fn saveTerminalProfileSettings(&self, mut settings: TerminalProfileSettings) -> HostResult<()> {
        settings.validate()?;
        let support = self.base.terminalProfileSupport()?;
        let info = self.base.terminalInfo()?;
        for profile in &settings.profiles {
            let entry = support
                .iter()
                .find(|entry| {
                    entry.terminal == profile.terminal && entry.terminalType == profile.terminalType
                })
                .ok_or_else(|| {
                    HostError::new(format!(
                        "Terminal implementation is not registered: {}/{}",
                        profile.terminal, profile.terminalType
                    ))
                })?;
            for key in profile.options.keys() {
                if !entry.fields.contains(key) {
                    return Err(HostError::new(format!(
                        "Terminal option is unsupported: {key}"
                    )));
                }
            }
            if !profile.options.is_empty() {
                self.base.configuredTerminal(profile)?;
            }
            if profile.id == settings.defaultProfileId
                && profile.options.is_empty()
                && !info.types.iter().any(|entry| {
                    entry.terminal == profile.terminal
                        && entry.terminalType == profile.terminalType
                        && entry.available
                })
            {
                return Err(HostError::new("Selected default terminal is unavailable"));
            }
        }
        self.settings()?;
        let mut state = self.state()?;
        let current = state
            .settings
            .as_ref()
            .ok_or_else(|| HostError::new("Terminal settings are missing"))?;
        if settings.revision != current.revision {
            return Err(HostError::new(
                "Terminal profiles changed; reload before saving",
            ));
        }
        settings.revision = settings
            .revision
            .checked_add(1)
            .ok_or_else(|| HostError::new("Terminal profile revision exhausted"))?;
        let bytes = settings.toBytes()?;
        self.storage
            .writeBytesAtomically(TERMINAL_PROFILES_PATH, &bytes)?;
        state.settings = Some(settings);
        // Session owners keep removed configurations alive until their last session closes.
        state.hosts.retain(|_, host| Arc::strong_count(host) > 1);
        Ok(())
    }
    /// Preserves platform-specific resource locator handling.
    fn resolveNativeWorkingDirectory(&self, path: &str) -> HostResult<String> {
        self.base.resolveNativeWorkingDirectory(path)
    }
    /// Dispatches the directory through the same immutable profile used to launch the PTY.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        kind: &str,
        directory: &str,
        resolve: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        let profile = self.profile(terminal, kind)?;
        self.host(&profile)?
            .resolveWorkingDirectory(&profile.terminal, kind, directory, resolve)
    }
    /// Publishes the configured default with stable SDK implementation and interpreter identities.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        let settings = self.settings()?;
        let selected = settings
            .profiles
            .iter()
            .find(|profile| profile.id == settings.defaultProfileId)
            .ok_or_else(|| HostError::new("Default terminal profile is missing"))?;
        let mut info = self.base.terminalInfo()?;
        info.terminal = selected.terminal.clone();
        info.terminalType = selected.terminalType.clone();
        let index = info
            .types
            .iter()
            .position(|entry| {
                entry.terminal == selected.terminal && entry.terminalType == selected.terminalType
            })
            .ok_or_else(|| {
                HostError::new("Configured terminal implementation is no longer registered")
            })?;
        info.types[index].description = selected.name.clone();
        if !selected.options.is_empty() {
            info.types[index].available = true;
        }
        info.types.swap(0, index);
        Ok(info)
    }
    /// Starts the selected profile and records its owner before exposing the session.
    fn startPtySession(
        &self,
        name: &str,
        terminal: &str,
        kind: &str,
        directory: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let profile = self.profile(terminal, kind)?;
        let host = self.host(&profile)?;
        let id = host.startPtySession(name, &profile.terminal, kind, directory, rows, cols)?;
        self.remember(&id, host)?;
        Ok(id)
    }
    /// Drains output from the session's original host.
    fn readPtySession(&self, id: &str) -> HostResult<Vec<u8>> {
        self.session(id)?.readPtySession(id)
    }
    /// Writes input to the session's original host.
    fn writePtySession(&self, id: &str, data: &[u8]) -> HostResult<usize> {
        self.session(id)?.writePtySession(id, data)
    }
    /// Resizes the session's original host PTY.
    fn resizePtySession(&self, id: &str, rows: u16, cols: u16) -> HostResult<()> {
        self.session(id)?.resizePtySession(id, rows, cols)
    }
    /// Polls the session's original process, including retained exited sessions.
    fn pollPtyExitCode(&self, id: &str) -> HostResult<Option<i32>> {
        self.session(id)?.pollPtyExitCode(id)
    }
    /// Removes routing only after the original host closes successfully.
    fn closePtySession(&self, id: &str) -> HostResult<()> {
        self.session(id)?.closePtySession(id)?;
        let mut state = self.state()?;
        state.sessions.remove(id);
        state.names.remove(id);
        Ok(())
    }
    /// Merges distinct host snapshots and adopts sessions created by host-owned integrations.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let mut hosts = vec![self.base.clone()];
        {
            let state = self.state()?;
            for host in state.hosts.values().chain(state.sessions.values()) {
                if !hosts.iter().any(|entry| Arc::ptr_eq(entry, host)) {
                    hosts.push(host.clone());
                }
            }
        }
        let mut sessions = Vec::new();
        for host in hosts {
            for mut session in host.listSessions()? {
                self.remember(&session.sessionId, host.clone())?;
                if let Some((name, _)) = self.state()?.names.get(&session.sessionId) {
                    session.sessionName = name.clone();
                }
                sessions.push(session);
            }
        }
        Ok(sessions)
    }
    /// Creates a named session in the user-selected default profile.
    fn createOrGetSession(&self, name: &str) -> HostResult<TerminalSessionInfo> {
        let settings = self.settings()?;
        let selected = settings
            .profiles
            .into_iter()
            .find(|profile| profile.id == settings.defaultProfileId)
            .ok_or_else(|| HostError::new("Default terminal profile is missing"))?;
        self.create(name, selected)
    }
    /// Resolves exact canonical implementation pairs and internal profile handles consistently.
    fn createOrGetTerminalSession(
        &self,
        name: &str,
        terminal: &str,
        kind: &str,
    ) -> HostResult<TerminalSessionInfo> {
        self.create(name, self.profile(terminal, kind)?)
    }
    /// Creates a typed session using the configured profile for that interpreter.
    fn createOrGetTypedSession(&self, name: &str, kind: &str) -> HostResult<TerminalSessionInfo> {
        self.create(name, self.typedProfile(kind)?)
    }
    /// Runs a command in the session's original environment.
    fn executeInSession(
        &self,
        id: &str,
        command: &str,
        timeout: u64,
    ) -> HostResult<TerminalCommandOutput> {
        self.session(id)?.executeInSession(id, command, timeout)
    }
    /// Streams a command through its original host.
    fn executeInSessionStreaming(
        &self,
        id: &str,
        command: &str,
        timeout: u64,
        output: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        self.session(id)?
            .executeInSessionStreaming(id, command, timeout, output)
    }
    /// Preserves event-loop scheduling supplied by Web and emulator hosts.
    fn scheduleInSessionStreaming(
        self: Arc<Self>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        id: String,
        command: String,
        timeout: u64,
        output: TerminalOutputCallback,
        complete: TerminalCompletionCallback,
    ) -> HostResult<()> {
        self.session(&id)?
            .scheduleInSessionStreaming(scheduler, id, command, timeout, output, complete)
    }
    /// Executes hidden commands through the same configured default as visible sessions.
    fn executeHiddenCommand(
        &self,
        command: &str,
        key: &str,
        timeout: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let session = self.createOrGetSession(&format!("hidden:{key}"))?;
        let result = self.executeInSession(&session.sessionId, command, timeout)?;
        Ok(HiddenTerminalCommandOutput {
            command: result.command,
            output: result.output,
            exitCode: result.exitCode,
            executorKey: key.into(),
            platform: result.platform,
            terminal: result.terminal,
            terminalType: result.terminalType,
            timedOut: result.timedOut,
        })
    }

    /// Sends text or controls through the session owner.
    fn inputInSession(
        &self,
        id: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        self.session(id)?.inputInSession(id, input, control)
    }
    /// Releases a named session after successful host closure.
    fn closeSession(&self, id: &str) -> HostResult<TerminalCloseOutput> {
        let result = self.session(id)?.closeSession(id)?;
        let mut state = self.state()?;
        state.sessions.remove(id);
        state.names.remove(id);
        Ok(result)
    }
    /// Preserves the original host's atomic idle-session ownership check.
    fn closeSessionIfIdle(&self, id: &str, name: &str) -> HostResult<TerminalCloseOutput> {
        let actual = match self.state()?.names.get(id) {
            Some((_, prefix)) => format!("{prefix}{name}"),
            None => name.to_string(),
        };
        let result = self.session(id)?.closeSessionIfIdle(id, &actual)?;
        let mut state = self.state()?;
        state.sessions.remove(id);
        state.names.remove(id);
        Ok(result)
    }
    /// Reads screen state from the original environment after profile changes.
    fn getSessionScreen(&self, id: &str) -> HostResult<TerminalScreenOutput> {
        self.session(id)?.getSessionScreen(id)
    }
}
