//! Exercises the real package/JavaScript SDK path rather than invoking terminal tools directly.
use super::*;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_tools::ToolExecutionManager::{
    ToolCall, ToolExecutionManager, ToolExecutionPolicy, ToolExposureMode, ToolRuntimeContext,
};

#[derive(Default)]
struct DirectoryTerminalHost {
    sessions: Mutex<BTreeMap<String, (TerminalSessionInfo, String)>>,
    commands: Mutex<Vec<String>>,
}

impl DirectoryTerminalHost {
    fn info() -> TerminalInfo {
        TerminalInfo {
            platform: "macos".into(),
            terminal: "native".into(),
            terminalType: "zsh".into(),
            types: ["bash", "zsh", "shell"]
                .into_iter()
                .map(|kind| TerminalTypeInfo {
                    terminal: if kind == "shell" { "arm64" } else { "native" }.into(),
                    terminalType: kind.into(),
                    available: true,
                    description: kind.into(),
                })
                .collect(),
        }
    }
}

impl TerminalHost for DirectoryTerminalHost {
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        _: &str,
        path: &str,
        resolve: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        let native = resolve(path)?;
        Ok(if terminal == "arm64" {
            format!("/guest{native}")
        } else {
            native
        })
    }
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(Self::info())
    }
    fn createOrGetSession(&self, name: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(name, "zsh")
    }
    fn createOrGetTypedSession(&self, name: &str, kind: &str) -> HostResult<TerminalSessionInfo> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some((session, _)) = sessions.get(name) {
            let mut session = session.clone();
            session.isNewSession = false;
            return Ok(session);
        }
        let session = TerminalSessionInfo {
            sessionId: name.into(),
            sessionName: name.into(),
            platform: "macos".into(),
            terminal: if kind == "shell" { "arm64" } else { "native" }.into(),
            terminalType: kind.into(),
            isNewSession: true,
        };
        sessions.insert(name.into(), (session.clone(), "/home".into()));
        Ok(session)
    }
    fn executeInSession(
        &self,
        id: &str,
        command: &str,
        _: u64,
    ) -> HostResult<TerminalCommandOutput> {
        self.commands.lock().unwrap().push(command.into());
        let mut sessions = self.sessions.lock().unwrap();
        let (session, directory) = sessions
            .get_mut(id)
            .ok_or_else(|| HostError::new("Unknown session"))?;
        let mut exit_code = 0;
        let output = if let Some(path) = command
            .strip_prefix("cd '")
            .and_then(|v| v.strip_suffix('\''))
        {
            let path = path.replace("'\\''", "'");
            let native = path.strip_prefix("/guest").unwrap_or(&path);
            if std::path::Path::new(native).is_dir() {
                *directory = path;
                String::new()
            } else {
                exit_code = 1;
                "Missing working directory".into()
            }
        } else if command == "pwd" {
            directory.clone()
        } else if let Some(path) = command.strip_prefix("cat ") {
            let native = directory.strip_prefix("/guest").unwrap_or(directory);
            std::fs::read_to_string(std::path::Path::new(native).join(path))
                .map_err(|error| HostError::new(format!("read from {native}: {error}")))?
        } else {
            return Err(HostError::new(format!(
                "Unexpected test command: {command}"
            )));
        };
        Ok(TerminalCommandOutput {
            command: command.into(),
            output,
            exitCode: exit_code,
            sessionId: id.into(),
            platform: session.platform.clone(),
            terminal: session.terminal.clone(),
            terminalType: session.terminalType.clone(),
            timedOut: false,
        })
    }
    fn closeSession(&self, id: &str) -> HostResult<TerminalCloseOutput> {
        self.sessions.lock().unwrap().remove(id);
        Ok(TerminalCloseOutput {
            sessionId: id.into(),
            success: true,
            message: String::new(),
        })
    }
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        Ok(Vec::new())
    }
    fn startPtySession(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: u16,
        _: u16,
    ) -> HostResult<String> {
        unimplemented!()
    }
    fn readPtySession(&self, _: &str) -> HostResult<Vec<u8>> {
        unimplemented!()
    }
    fn writePtySession(&self, _: &str, _: &[u8]) -> HostResult<usize> {
        unimplemented!()
    }
    fn resizePtySession(&self, _: &str, _: u16, _: u16) -> HostResult<()> {
        unimplemented!()
    }
    fn pollPtyExitCode(&self, _: &str) -> HostResult<Option<i32>> {
        unimplemented!()
    }
    fn closePtySession(&self, _: &str) -> HostResult<()> {
        unimplemented!()
    }
    fn executeHiddenCommand(
        &self,
        _: &str,
        _: &str,
        _: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        unimplemented!()
    }
    fn inputInSession(
        &self,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        unimplemented!()
    }
    fn getSessionScreen(&self, _: &str) -> HostResult<TerminalScreenOutput> {
        unimplemented!()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn super_admin_sdk_keeps_chat_directory_across_javascript_scheduling() {
    let _guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    tokio::task::LocalSet::new().run_until(async {
        let storage = register_test_runtime_roots();
        let terminal = Arc::new(DirectoryTerminalHost::default());
        let mut hosts = HostManager {
            hostEnvironment: operit_host_api::HostEnvironmentDescriptor {
                id: "macos".into(), displayName: "macOS".into(), platform: operit_host_api::HostPlatform::Macos,
                ..operit_host_api::HostEnvironmentDescriptor::linux()
            },
            fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
            runtimeStorageHost: Some(storage.clone()), runtimeSqliteHost: Some(storage.clone()),
            terminalHost: Some(terminal.clone()), hostSecretStore: Some(Arc::new(TestSecretStore::default())),
            hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
            hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
            ..HostManager::default()
        };
        ui_resource_tests::configure_package_hosts(&mut hosts);
        let app = OperitApplication::newWithContext(hosts);
        app.packageManager().lock().unwrap().loadAvailablePackages();
        let mut handler = app.aiToolHandler();
        handler.registerDefaultTools();
        let output = operit_store::ChatOutputStore::ChatOutputStore::new(storage.clone())
            .bind("terminal-chat", 1793740800123, None).unwrap();
        let filesystem = app.hostManager.vfs().unwrap();
        for workspace in [None, Some("/operitapp/workspaces/project with space"), None] {
            let base = workspace.unwrap_or(&output);
            let native = std::path::PathBuf::from(operit_host_api::FileSource::nativePath(&filesystem.file(base).unwrap()).unwrap());
            std::fs::create_dir_all(native.join(".operit/attachments/fixture")).unwrap();
            std::fs::write(native.join(".operit/attachments/fixture/input.txt"), base).unwrap();
            for kind in ["bash", "zsh", "shell"] {
                let context = ToolRuntimeContext {
                    callerChatId: Some("terminal-chat".into()), callerParticipantId: None,
                    workspacePath: workspace.map(str::to_string), workspaceFolders: Vec::new(),
                    toolExposureMode: ToolExposureMode::FULL, extensionOwner: "test-owner".into(), executionData: Default::default(),
                };
                let request = ToolCall { name: "package_proxy".into(), parameters: [
                    ("tool_name".into(), json!(format!("super_admin:{kind}"))),
                    ("params".into(), json!(json!({ "command": "cat .operit/attachments/fixture/input.txt", "timeoutMs": 3000 }).to_string())),
                ].into_iter().collect() };
                let results = ToolExecutionManager::scopeToolRuntimeContext(context,
                    handler.executeToolInvocation(&request, ToolExecutionPolicy::Direct)).await.unwrap();
                let result = results.last().unwrap();
                assert!(result.success, "{kind} {workspace:?}: {:?}; commands={:?}", result.error, terminal.commands.lock().unwrap());
                let payload: Value = serde_json::from_str(&result.result.toString()).unwrap();
                assert_eq!(payload["output"], base, "{kind} {workspace:?}: {payload}");
            }
        }
        assert_eq!(terminal.commands.lock().unwrap().iter().filter(|c| c.starts_with("cd ")).count(), 9);
        for (chat_id, workspace, expected_error) in [
            ("terminal-chat", Some("/operitapp/workspaces/missing"), "Cannot enter terminal working directory"),
            ("unprepared-chat", None, "Current chat file directory is unavailable"),
        ] {
            for kind in ["bash", "zsh", "shell"] {
                let before = terminal.commands.lock().unwrap().len();
                let context = ToolRuntimeContext {
                    callerChatId: Some(chat_id.into()), callerParticipantId: None,
                    workspacePath: workspace.map(str::to_string), workspaceFolders: Vec::new(),
                    toolExposureMode: ToolExposureMode::FULL, extensionOwner: "test-owner".into(), executionData: Default::default(),
                };
                let request = ToolCall { name: "package_proxy".into(), parameters: [
                    ("tool_name".into(), json!(format!("super_admin:{kind}"))),
                    ("params".into(), json!(json!({ "command": "cat .operit/attachments/fixture/input.txt", "timeoutMs": 3000 }).to_string())),
                ].into_iter().collect() };
                let results = ToolExecutionManager::scopeToolRuntimeContext(context,
                    handler.executeToolInvocation(&request, ToolExecutionPolicy::Direct)).await.unwrap();
                let result = results.last().unwrap();
                assert!(!result.success, "unavailable directory must stop {kind}");
                assert!(result.error.as_deref().unwrap().contains(expected_error), "{:?}", result.error);
                assert!(terminal.commands.lock().unwrap()[before..].iter().all(|command| command.starts_with("cd ")),
                    "the requested command ran after the directory change failed");
            }
        }
    }).await;
}
