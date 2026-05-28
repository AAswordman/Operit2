use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreCommandOutput {
    pub stdout: String,
    pub stderr: String,
}

impl CoreCommandOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_stdout_line(&mut self, line: impl AsRef<str>) {
        self.stdout.push_str(line.as_ref());
        self.stdout.push('\n');
    }

    pub fn push_stdout(&mut self, value: impl AsRef<str>) {
        self.stdout.push_str(value.as_ref());
    }

    pub fn push_stderr_line(&mut self, line: impl AsRef<str>) {
        self.stderr.push_str(line.as_ref());
        self.stderr.push('\n');
    }
}
