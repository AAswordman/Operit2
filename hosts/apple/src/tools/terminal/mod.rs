#[cfg(target_os = "macos")]
#[path = "terminal_macos.rs"]
mod platform;

#[cfg(target_os = "macos")]
pub use platform::AppleTerminalHost;

#[cfg(target_os = "ios")]
mod ios {
    use operit_host_api::{
        HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput,
        TerminalCommandOutput, TerminalHost, TerminalInfo, TerminalInputOutput,
        TerminalScreenOutput, TerminalSessionInfo, TerminalSessionListEntry, TerminalTypeInfo,
    };

    #[derive(Clone, Default)]
    pub struct AppleTerminalHost;

    impl AppleTerminalHost {
        pub fn new() -> Self {
            Self
        }
    }

    impl TerminalHost for AppleTerminalHost {
        fn terminalInfo(&self) -> HostResult<TerminalInfo> {
            Ok(TerminalInfo {
                platform: "ios".to_string(),
                defaultType: "ios".to_string(),
                types: vec![TerminalTypeInfo {
                    terminalType: "ios".to_string(),
                    available: false,
                    description: "iOS does not expose a local PTY host".to_string(),
                }],
            })
        }

        fn startPtySession(&self, _: &str, _: &str, _: &str, _: u16, _: u16) -> HostResult<String> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn readPtySession(&self, _: &str) -> HostResult<Vec<u8>> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn writePtySession(&self, _: &str, _: &[u8]) -> HostResult<usize> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn resizePtySession(&self, _: &str, _: u16, _: u16) -> HostResult<()> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn pollPtyExitCode(&self, _: &str) -> HostResult<Option<i32>> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn closePtySession(&self, _: &str) -> HostResult<()> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
            Ok(vec![])
        }

        fn createOrGetSession(&self, _: &str, _: &str) -> HostResult<TerminalSessionInfo> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn executeInSession(&self, _: &str, _: &str, _: u64) -> HostResult<TerminalCommandOutput> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn executeHiddenCommand(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: u64,
        ) -> HostResult<HiddenTerminalCommandOutput> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn inputInSession(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> HostResult<TerminalInputOutput> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn closeSession(&self, _: &str) -> HostResult<TerminalCloseOutput> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }

        fn getSessionScreen(&self, _: &str) -> HostResult<TerminalScreenOutput> {
            Err(HostError::new("iOS does not expose a local PTY host"))
        }
    }
}

#[cfg(target_os = "ios")]
pub use ios::AppleTerminalHost;

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
mod non_apple_target {
    use operit_host_api::{
        HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput,
        TerminalCommandOutput, TerminalHost, TerminalInfo, TerminalInputOutput,
        TerminalScreenOutput, TerminalSessionInfo, TerminalSessionListEntry, TerminalTypeInfo,
    };

    #[derive(Clone, Default)]
    pub struct AppleTerminalHost;

    impl AppleTerminalHost {
        pub fn new() -> Self {
            Self
        }
    }

    impl TerminalHost for AppleTerminalHost {
        fn terminalInfo(&self) -> HostResult<TerminalInfo> {
            Ok(TerminalInfo {
                platform: "apple".to_string(),
                defaultType: "apple".to_string(),
                types: vec![TerminalTypeInfo {
                    terminalType: "apple".to_string(),
                    available: false,
                    description: "Apple terminal host is available only on iOS or macOS"
                        .to_string(),
                }],
            })
        }

        fn startPtySession(&self, _: &str, _: &str, _: &str, _: u16, _: u16) -> HostResult<String> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn readPtySession(&self, _: &str) -> HostResult<Vec<u8>> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn writePtySession(&self, _: &str, _: &[u8]) -> HostResult<usize> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn resizePtySession(&self, _: &str, _: u16, _: u16) -> HostResult<()> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn pollPtyExitCode(&self, _: &str) -> HostResult<Option<i32>> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn closePtySession(&self, _: &str) -> HostResult<()> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
            Ok(vec![])
        }

        fn createOrGetSession(&self, _: &str, _: &str) -> HostResult<TerminalSessionInfo> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn executeInSession(&self, _: &str, _: &str, _: u64) -> HostResult<TerminalCommandOutput> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn executeHiddenCommand(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: u64,
        ) -> HostResult<HiddenTerminalCommandOutput> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn inputInSession(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> HostResult<TerminalInputOutput> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn closeSession(&self, _: &str) -> HostResult<TerminalCloseOutput> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }

        fn getSessionScreen(&self, _: &str) -> HostResult<TerminalScreenOutput> {
            Err(HostError::new(
                "Apple terminal host is available only on iOS or macOS",
            ))
        }
    }
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
pub use non_apple_target::AppleTerminalHost;
