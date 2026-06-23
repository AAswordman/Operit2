use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::TerminalHost;
use serde::{Deserialize, Serialize};

use operit_context::OperitApplicationContext::OperitApplicationContext;
use crate::util::stream::HotStream::MutableSharedStreamImpl;
use operit_util::stream::Stream::Stream;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTerminalSessionInfo {
    pub sessionId: String,
    pub sessionName: String,
    pub terminalType: String,
    pub sessionKind: String,
    pub workingDir: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTerminalScreen {
    pub sessionId: String,
    pub terminalType: String,
    pub rows: i32,
    pub cols: i32,
    pub content: String,
    pub commandRunning: bool,
}

pub struct RuntimeTerminalService {
    terminalHost: Arc<dyn TerminalHost>,
}

#[derive(Clone, Debug)]
pub struct RuntimeTerminalPtyOutputStream {
    upstream: MutableSharedStreamImpl<String>,
}

impl RuntimeTerminalPtyOutputStream {
    pub fn new(upstream: MutableSharedStreamImpl<String>) -> Self {
        Self { upstream }
    }
}

impl Stream for RuntimeTerminalPtyOutputStream {
    type Item = String;

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.upstream.collect(collector);
    }
}

#[derive(Clone)]
struct TerminalPtyOutputEntry {
    stream: MutableSharedStreamImpl<String>,
}

static TERMINAL_PTY_OUTPUT_STREAMS: OnceLock<Mutex<HashMap<String, TerminalPtyOutputEntry>>> =
    OnceLock::new();

fn terminal_pty_output_streams() -> &'static Mutex<HashMap<String, TerminalPtyOutputEntry>> {
    TERMINAL_PTY_OUTPUT_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn close_terminal_pty_output_stream(sessionId: &str) {
    let entry = terminal_pty_output_streams()
        .lock()
        .expect("terminal pty output streams mutex poisoned")
        .remove(sessionId);
    if let Some(entry) = entry {
        entry.stream.close();
    }
}

fn start_terminal_pty_output_reader(
    terminalHost: Arc<dyn TerminalHost>,
    sessionId: String,
    stream: MutableSharedStreamImpl<String>,
) {
    thread::spawn(move || loop {
        match terminalHost.readPtySession(&sessionId) {
            Ok(data) => {
                if !data.is_empty() {
                    stream.emit(STANDARD.encode(data));
                }
            }
            Err(_) => {
                close_terminal_pty_output_stream(&sessionId);
                break;
            }
        }

        match terminalHost.pollPtyExitCode(&sessionId) {
            Ok(Some(_)) => {
                close_terminal_pty_output_stream(&sessionId);
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(_) => {
                close_terminal_pty_output_stream(&sessionId);
                break;
            }
        }
    });
}

impl RuntimeTerminalService {
    #[allow(non_snake_case)]
    pub fn getInstance(context: &OperitApplicationContext) -> Self {
        Self {
            terminalHost: context
                .terminalHost
                .clone()
                .expect("TerminalHost must be configured for RuntimeTerminalService"),
        }
    }

    #[allow(non_snake_case)]
    pub fn listTerminalSessions(&self) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
        self.terminalHost
            .listSessions()
            .map_err(|error| error.message)
            .map(|sessions| {
                sessions
                    .into_iter()
                    .map(|session| RuntimeTerminalSessionInfo {
                        sessionId: session.sessionId,
                        sessionName: session.sessionName,
                        terminalType: session.terminalType,
                        sessionKind: session.sessionKind,
                        workingDir: session.workingDir,
                        commandRunning: session.commandRunning,
                    })
                    .collect()
            })
    }

    #[allow(non_snake_case)]
    pub fn startTerminalPty(
        &self,
        sessionName: String,
        workingDir: String,
        rows: i32,
        cols: i32,
    ) -> Result<String, String> {
        self.terminalHost
            .startPtySession(&sessionName, &workingDir, rows as u16, cols as u16)
            .map_err(|error| error.message)
            .inspect(|sessionId| {
                self.ensureTerminalPtyOutputStream(sessionId.clone());
            })
    }

    #[allow(non_snake_case)]
    pub fn terminalPtyOutput(&self, sessionId: String) -> RuntimeTerminalPtyOutputStream {
        RuntimeTerminalPtyOutputStream::new(self.ensureTerminalPtyOutputStream(sessionId))
    }

    #[allow(non_snake_case)]
    pub fn writeTerminalPty(&self, sessionId: String, dataBase64: String) -> Result<i32, String> {
        let data = STANDARD
            .decode(dataBase64.as_bytes())
            .map_err(|error| error.to_string())?;
        self.terminalHost
            .writePtySession(&sessionId, &data)
            .map(|count| count as i32)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn resizeTerminalPty(&self, sessionId: String, rows: i32, cols: i32) -> Result<(), String> {
        self.terminalHost
            .resizePtySession(&sessionId, rows as u16, cols as u16)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn pollTerminalPtyExit(&self, sessionId: String) -> Result<Option<i32>, String> {
        self.terminalHost
            .pollPtyExitCode(&sessionId)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn closeTerminalPty(&self, sessionId: String) -> Result<(), String> {
        close_terminal_pty_output_stream(&sessionId);
        self.terminalHost
            .closePtySession(&sessionId)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn inputTerminalSession(&self, sessionId: String, input: String) -> Result<i32, String> {
        self.terminalHost
            .inputInSession(&sessionId, Some(&input), None)
            .map(|output| output.acceptedChars as i32)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    pub fn getTerminalSessionScreen(
        &self,
        sessionId: String,
    ) -> Result<RuntimeTerminalScreen, String> {
        self.terminalHost
            .getSessionScreen(&sessionId)
            .map_err(|error| error.message)
            .map(|screen| RuntimeTerminalScreen {
                sessionId: screen.sessionId,
                terminalType: screen.terminalType,
                rows: screen.rows as i32,
                cols: screen.cols as i32,
                content: screen.content,
                commandRunning: screen.commandRunning,
            })
    }

    #[allow(non_snake_case)]
    fn ensureTerminalPtyOutputStream(&self, sessionId: String) -> MutableSharedStreamImpl<String> {
        let mut streams = terminal_pty_output_streams()
            .lock()
            .expect("terminal pty output streams mutex poisoned");
        if let Some(entry) = streams.get(&sessionId) {
            return entry.stream.clone();
        }
        let stream = MutableSharedStreamImpl::new(512);
        streams.insert(
            sessionId.clone(),
            TerminalPtyOutputEntry {
                stream: stream.clone(),
            },
        );
        start_terminal_pty_output_reader(self.terminalHost.clone(), sessionId, stream.clone());
        stream
    }
}
