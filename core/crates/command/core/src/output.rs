use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreCommandOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(skip)]
    jsonMode: bool,
    #[serde(skip)]
    jsonStdoutDocument: Option<serde_json::Value>,
}

impl CoreCommandOutput {
    /// Creates an empty command output buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables structured JSON output for this invocation.
    pub fn setJsonMode(&mut self, enabled: bool) {
        self.jsonMode = enabled;
    }

    /// Returns whether the command is producing structured JSON.
    pub fn isJsonMode(&self) -> bool {
        self.jsonMode
    }

    /// Sets the explicit JSON result for stdout.
    pub fn setJsonStdout(&mut self, value: serde_json::Value) {
        self.jsonStdoutDocument = Some(value);
    }

    /// Appends one command result line.
    pub fn push_stdout_line(&mut self, line: impl AsRef<str>) {
        let line = line.as_ref();
        self.stdout.push_str(line);
        self.stdout.push('\n');
    }

    /// Appends raw command result text.
    pub fn push_stdout(&mut self, value: impl AsRef<str>) {
        self.stdout.push_str(value.as_ref());
    }

    /// Appends one command error line.
    pub fn push_stderr_line(&mut self, line: impl AsRef<str>) {
        let line = line.as_ref();
        self.stderr.push_str(line);
        self.stderr.push('\n');
    }

    /// Finalizes the explicit JSON document returned to the caller.
    pub fn finalizeJson(&mut self) -> Result<(), String> {
        if !self.jsonMode {
            return Ok(());
        }
        let stdout = self
            .jsonStdoutDocument
            .take()
            .ok_or_else(|| "command did not set JSON stdout".to_string())?;
        self.stdout = serde_json::to_string(&stdout).expect("command JSON must serialize");
        self.stderr.clear();
        Ok(())
    }
}
