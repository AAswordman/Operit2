use operit_host_api::HostResult;
use std::sync::Arc;

/// Publishes session changes while delegating terminal work to the supplied host.
#[derive(Clone)]
pub(crate) struct RuntimeSessionPublishingTerminalHost {
    inner: Arc<dyn operit_host_api::TerminalHost>,
}

impl RuntimeSessionPublishingTerminalHost {
    /// Creates a terminal host wrapper that publishes runtime session snapshots.
    pub(crate) fn new(inner: Arc<dyn operit_host_api::TerminalHost>) -> Self {
        Self { inner }
    }

    /// Publishes the current terminal session snapshot through RuntimeTerminalService.
    fn publish_sessions(&self) -> operit_host_api::HostResult<()> {
        crate::services::publish_terminal_sessions_for_host(self)
            .map(|_| ())
            .map_err(operit_host_api::HostError::new)
    }
}

impl operit_host_api::TerminalHost for RuntimeSessionPublishingTerminalHost {
    /// Publishes fresh interactive and command processes without shell bootstrap commands.
    fn openTerminalSession(&self, request: &operit_host_api::TerminalSessionRequest) -> HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self.inner.openTerminalSession(request)?;
        self.publish_sessions()?;
        Ok(session)
    }
    /// Forwards the platform-owned editing schema.
    fn terminalProfileSupport(
        &self,
    ) -> operit_host_api::HostResult<Vec<operit_host_api::TerminalProfileSupport>> {
        self.inner.terminalProfileSupport()
    }
    /// Reads the Core-local profile registry.
    fn terminalProfileSettings(
        &self,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalProfileSettings> {
        self.inner.terminalProfileSettings()
    }
    /// Persists profiles without altering running sessions.
    fn saveTerminalProfileSettings(
        &self,
        settings: operit_host_api::TerminalProfileSettings,
    ) -> operit_host_api::HostResult<()> {
        self.inner.saveTerminalProfileSettings(settings)
    }
    /// Forwards exact host launch configuration without introducing another implementation.
    fn configuredTerminal(
        &self,
        profile: &operit_host_api::TerminalProfile,
    ) -> operit_host_api::HostResult<Arc<dyn operit_host_api::TerminalHost>> {
        self.inner.configuredTerminal(profile)
    }
    /// Exposes only distribution downloads supported by the executing host.
    fn terminalDistributions(
        &self,
    ) -> operit_host_api::HostResult<Vec<operit_host_api::TerminalDistribution>> {
        self.inner.terminalDistributions()
    }
    /// Preserves the host's installation rollback boundary.
    fn discardTerminalDistribution(&self, instanceId: &str) -> HostResult<()> {
        self.inner.discardTerminalDistribution(instanceId)
    }
    /// Keeps download ownership in the registry and native host installer.
    fn installTerminalDistribution(
        &self,
        distributionId: &str,
        instanceId: &str,
        name: &str,
        control: operit_host_api::HttpDownloadControl,
        progress: operit_host_api::TerminalInstallProgressCallback,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalProfile> {
        self.inner
            .installTerminalDistribution(distributionId, instanceId, name, control, progress)
    }
    /// Creates an exact profile session and publishes the full registry's session list.
    fn createOrGetProfileSession(
        &self,
        name: &str,
        profileId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self.inner.createOrGetProfileSession(name, profileId)?;
        self.publish_sessions()?;
        Ok(session)
    }
    /// Preserves exact implementation selection when several implementations share a shell.
    fn createOrGetTerminalSession(
        &self,
        name: &str,
        terminal: &str,
        terminalType: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self
            .inner
            .createOrGetTerminalSession(name, terminal, terminalType)?;
        self.publish_sessions()?;
        Ok(session)
    }

    /// Preserves native directory resolution from the selected host.
    fn resolveNativeWorkingDirectory(&self, path: &str) -> operit_host_api::HostResult<String> {
        self.inner.resolveNativeWorkingDirectory(path)
    }

    /// Preserves the wrapped terminal's ownership of its working-directory namespace.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> operit_host_api::HostResult<String>,
    ) -> operit_host_api::HostResult<String> {
        self.inner
            .resolveWorkingDirectory(terminal, terminalType, workingDir, resolveHostDirectory)
    }

    /// Returns terminal capabilities exposed by the wrapped host.
    fn terminalInfo(&self) -> operit_host_api::HostResult<operit_host_api::TerminalInfo> {
        self.inner.terminalInfo()
    }

    /// Starts a PTY session and publishes the updated session list.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> operit_host_api::HostResult<String> {
        let sessionId = self.inner.startPtySession(
            sessionName,
            terminal,
            terminalType,
            workingDir,
            rows,
            cols,
        )?;
        self.publish_sessions()?;
        Ok(sessionId)
    }

    /// Reads buffered PTY output from the wrapped host.
    /// Awaits binary output from the backend that owns this exact session.
    fn nextPtySessionOutput<'a>(&'a self, sessionId: &'a str) -> operit_host_api::TerminalPtyOutputFuture<'a> {
        Box::pin(async move {
            self.inner.nextPtySessionOutput(sessionId).await
        })
    }

    fn readPtySession(&self, sessionId: &str) -> operit_host_api::HostResult<Vec<u8>> {
        self.inner.readPtySession(sessionId)
    }

    /// Writes bytes to a PTY session on the wrapped host.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> operit_host_api::HostResult<usize> {
        self.inner.writePtySession(sessionId, data)
    }

    /// Resizes a PTY session on the wrapped host.
    fn resizePtySession(
        &self,
        sessionId: &str,
        rows: u16,
        cols: u16,
    ) -> operit_host_api::HostResult<()> {
        self.inner.resizePtySession(sessionId, rows, cols)
    }

    /// Polls one PTY session exit state from the wrapped host.
    fn pollPtyExitCode(&self, sessionId: &str) -> operit_host_api::HostResult<Option<i32>> {
        self.inner.pollPtyExitCode(sessionId)
    }

    /// Closes a PTY session and publishes the updated session list.
    fn closePtySession(&self, sessionId: &str) -> operit_host_api::HostResult<()> {
        self.inner.closePtySession(sessionId)?;
        self.publish_sessions()
    }

    /// Lists terminal sessions from the wrapped host.
    fn listSessions(
        &self,
    ) -> operit_host_api::HostResult<Vec<operit_host_api::TerminalSessionListEntry>> {
        self.inner.listSessions()
    }

    /// Creates or returns a named terminal session and publishes the updated session list.
    fn createOrGetSession(
        &self,
        sessionName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self.inner.createOrGetSession(sessionName)?;
        self.publish_sessions()?;
        Ok(session)
    }

    /// Creates or returns an exactly typed session and publishes the updated session list.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalSessionInfo> {
        let session = self
            .inner
            .createOrGetTypedSession(sessionName, terminalType)?;
        self.publish_sessions()?;
        Ok(session)
    }

    /// Executes a command inside an existing terminal session.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalCommandOutput> {
        self.inner.executeInSession(sessionId, command, timeoutMs)
    }

    /// Preserves raw streaming when terminal access passes through the Flutter owner wrapper.
    fn executeInSessionStreaming(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<operit_host_api::TerminalCommandOutput> {
        self.inner
            .executeInSessionStreaming(sessionId, command, timeoutMs, onOutput)
    }

    /// Preserves the exact backend when a caller obtains cooperative command state directly.
    fn beginInSessionStreaming(
        self: Arc<Self>,
        sessionId: String,
        command: String,
        timeoutMs: u64,
    ) -> HostResult<Box<dyn operit_host_api::TerminalCommand::TerminalCommandExecution>> {
        self.inner.clone().beginInSessionStreaming(sessionId, command, timeoutMs)
    }

    /// Retains the concrete Host's scheduling model and publishes command lifecycle changes.
    fn scheduleInSessionStreaming(
        self: Arc<Self>,
        scheduler: Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>,
        sessionId: String,
        command: String,
        timeoutMs: u64,
        mut onOutput: operit_host_api::TerminalOutputCallback,
        onComplete: operit_host_api::TerminalCompletionCallback,
    ) -> HostResult<()> {
        let publisher = self.clone();
        let outputScheduler = scheduler.clone();
        let completionScheduler = scheduler.clone();
        let mut started = false;
        self.inner.clone().scheduleInSessionStreaming(
            scheduler,
            sessionId,
            command,
            timeoutMs,
            Box::new(move |bytes, finalFrame| {
                if !started {
                    let publisher = publisher.clone();
                    outputScheduler.scheduleHostRuntimeTask("terminal-session-started", Box::new(move || {
                        if let Err(error) = publisher.publish_sessions() {
                            operit_host_api::logHostError("terminal-session-started", &error.message);
                        }
                    }))?;
                    started = true;
                }
                onOutput(bytes, finalFrame)
            }),
            Box::new(move |result| {
                let completion = Arc::new(std::sync::Mutex::new(Some(onComplete)));
                let delivered = completion.clone();
                let scheduled = completionScheduler.scheduleHostRuntimeTask("terminal-session-completed", Box::new(move || {
                    let result = result.and_then(|receipt| { self.publish_sessions()?; Ok(receipt) });
                    let complete = delivered.lock().unwrap_or_else(|error| error.into_inner()).take();
                    if let Some(complete) = complete {
                        complete(result);
                    }
                }));
                if let Err(error) = scheduled {
                    let complete = completion.lock().unwrap_or_else(|error| error.into_inner()).take();
                    if let Some(complete) = complete {
                        complete(Err(error));
                    }
                }
            }),
        )
    }

    /// Executes a hidden command through the wrapped host.
    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> operit_host_api::HostResult<operit_host_api::HiddenTerminalCommandOutput> {
        self.inner
            .executeHiddenCommand(command, executorKey, timeoutMs)
    }

    /// Sends text or control input to an existing terminal session.
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalInputOutput> {
        self.inner.inputInSession(sessionId, input, control)
    }

    /// Closes a terminal session and publishes the updated session list.
    fn closeSession(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalCloseOutput> {
        let output = self.inner.closeSession(sessionId)?;
        self.publish_sessions()?;
        Ok(output)
    }

    /// Publishes the session list only after a successful conditional close.
    fn closeSessionIfIdle(
        &self,
        sessionId: &str,
        sessionName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalCloseOutput> {
        let output = self.inner.closeSessionIfIdle(sessionId, sessionName)?;
        self.publish_sessions()?;
        Ok(output)
    }

    /// Reads the current terminal screen from the wrapped host.
    fn getSessionScreen(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::TerminalScreenOutput> {
        self.inner.getSessionScreen(sessionId)
    }
}
