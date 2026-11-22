//! Shared terminal transcript rendering, independent of platform I/O and scheduling.
#![allow(non_snake_case)]

use crate::{HostError, HostResult};
use std::collections::VecDeque;
use std::sync::Mutex;

/// Finds the final complete protocol marker without matching an empty delimiter.
pub fn rfindBytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).rposition(|window| window == needle)
}

/// Retains output up to the owning terminal's explicit byte capacity.
pub fn appendPtyOutput(output: &Mutex<VecDeque<u8>>, data: &[u8], capacity: usize) {
    if let Ok(mut buffer) = output.lock() {
        buffer.extend(data.iter().copied());
        while buffer.len() > capacity {
            buffer.pop_front();
        }
    }
}

/// Removes the submitted command's single-line echo from a POSIX transcript.
pub fn dropCommandEcho(value: String, command: &str) -> String {
    let mut lines = value.lines().map(str::to_string).collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    if lines.first().is_some_and(|line| {
        let line = line.trim_end();
        let command = command.trim();
        line == command || line.ends_with(command)
    }) {
        lines.remove(0);
    }
    lines.join("\n").trim().to_string()
}

/// Parses the shell protocol's signed exit status while preserving its diagnostic.
pub fn parseExitCode(exitCodeText: &str) -> HostResult<i32> {
    exitCodeText.trim().parse::<i32>().map_err(|error| {
        HostError::new(format!("Invalid terminal exit code marker '{exitCodeText}': {error}"))
    })
}

/// Trims a required terminal text parameter and retains its public error message.
pub fn nonBlank(value: &str, paramName: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!("{paramName} parameter is required")));
    }
    Ok(trimmed.to_string())
}

/// Tracks the cursor used to answer terminal position queries.
#[derive(Debug)]
pub struct PtyCursor {
    pub row: usize,
    pub col: usize,
}

/// Advances the terminal cursor for ordinary output bytes.
pub fn updatePtyCursor(cursorState: &Mutex<PtyCursor>, value: u8) {
    if let Ok(mut cursor) = cursorState.lock() {
        match value {
            b'\r' => cursor.col = 1,
            b'\n' => {
                cursor.row += 1;
                cursor.col = 1;
            }
            8 => {
                if cursor.col > 1 {
                    cursor.col -= 1;
                }
            }
            0x20..=0x7e => cursor.col += 1,
            _ => {}
        }
    }
}

/// Visits printable output and cursor queries while skipping shell markers and escape payloads.
pub fn scanPtyOutput(
    data: &[u8],
    mut onCursorQuery: impl FnMut() -> HostResult<()>,
    mut onByte: impl FnMut(u8),
) -> HostResult<()> {
    let mut index = 0usize;
    while index < data.len() {
        if data[index..].starts_with(b"\x1b[6n") {
            onCursorQuery()?;
            index += 4;
            continue;
        }
        if data[index..].starts_with(b"\x1b]133;OperitPrompt=") {
            let payloadStart = index + b"\x1b]133;OperitPrompt=".len();
            if let Some(relativeEnd) = data[payloadStart..].iter().position(|value| *value == 7) {
                index = payloadStart + relativeEnd + 1;
                continue;
            }
        }
        if data[index] == b'\x1b' && index + 1 < data.len() {
            match data[index + 1] {
                b'[' => {
                    index += 2;
                    while index < data.len() {
                        let value = data[index];
                        index += 1;
                        if value >= b'@' && value <= b'~' {
                            break;
                        }
                    }
                    continue;
                }
                b']' => {
                    index += 2;
                    while index < data.len() {
                        let value = data[index];
                        index += 1;
                        if value == 7 {
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        onByte(data[index]);
        index += 1;
    }
    Ok(())
}

/// Removes complete shell protocol markers while preserving partial terminal frames.
pub fn stripPtyPromptMarkers(raw: &[u8]) -> Vec<u8> {
    const PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
    let mut output = Vec::with_capacity(raw.len());
    let mut index = 0usize;
    while index < raw.len() {
        if raw[index..].starts_with(PREFIX) {
            let payloadStart = index + PREFIX.len();
            if let Some(relativeEnd) = raw[payloadStart..].iter().position(|value| *value == 7) {
                index = payloadStart + relativeEnd + 1;
                continue;
            }
        }
        output.push(raw[index]);
        index += 1;
    }
    output
}

/// Removes cursor position queries from captured terminal transcripts.
pub fn stripPtyDeviceStatusReports(raw: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw.len());
    let mut index = 0usize;
    while index < raw.len() {
        if raw[index..].starts_with(b"\x1b[6n") {
            index += 4;
            continue;
        }
        output.push(raw[index]);
        index += 1;
    }
    output
}

#[derive(Default)]
struct TerminalTextRenderer {
    lines: Vec<Vec<char>>,
    row: usize,
    col: usize,
    savedRow: usize,
    savedCol: usize,
}

impl TerminalTextRenderer {
    /// Starts a transcript at its first empty line.
    fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            row: 0,
            col: 0,
            savedRow: 0,
            savedCol: 0,
        }
    }

    /// Applies printable characters and control sequences in their original order.
    fn write(&mut self, value: &str) {
        let chars = value.chars().collect::<Vec<_>>();
        let mut index = 0usize;
        while index < chars.len() {
            let ch = chars[index];
            if ch == '\x1b' {
                index = self.consumeEscape(&chars, index);
                continue;
            }
            self.writeChar(ch);
            index += 1;
        }
    }

    /// Returns the rendered transcript with explicit line separators.
    fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Dispatches escape sequences and cursor save or restore commands.
    fn consumeEscape(&mut self, chars: &[char], index: usize) -> usize {
        if index + 1 >= chars.len() {
            return index + 1;
        }
        match chars[index + 1] {
            '[' => self.consumeCsi(chars, index + 2),
            ']' => self.consumeOsc(chars, index + 2),
            '7' => {
                self.savedRow = self.row;
                self.savedCol = self.col;
                index + 2
            }
            '8' => {
                self.row = self.savedRow;
                self.col = self.savedCol;
                self.ensureCursorLine();
                index + 2
            }
            _ => index + 2,
        }
    }

    /// Consumes terminal metadata through its bell or string terminator.
    fn consumeOsc(&mut self, chars: &[char], mut index: usize) -> usize {
        while index < chars.len() {
            let ch = chars[index];
            index += 1;
            if ch == '\x07' {
                break;
            }
            if ch == '\x1b' && index < chars.len() && chars[index] == '\\' {
                index += 1;
                break;
            }
        }
        index
    }

    /// Parses one complete cursor or erasure command.
    fn consumeCsi(&mut self, chars: &[char], mut index: usize) -> usize {
        let start = index;
        while index < chars.len() {
            let ch = chars[index];
            index += 1;
            if ch >= '@' && ch <= '~' {
                let params = chars[start..index - 1].iter().collect::<String>();
                self.applyCsi(&params, ch);
                break;
            }
        }
        index
    }

    /// Applies the existing transcript cursor and erasure semantics.
    fn applyCsi(&mut self, params: &str, command: char) {
        let values = csiParams(params);
        match command {
            'A' => self.moveRowUp(csiValue(&values, 0, 1)),
            'B' => self.moveRowDown(csiValue(&values, 0, 1)),
            'C' => self.col += csiValue(&values, 0, 1),
            'D' => self.col = self.col.saturating_sub(csiValue(&values, 0, 1)),
            'G' => self.col = csiValue(&values, 0, 1).saturating_sub(1),
            'H' | 'f' => {
                self.row = csiValue(&values, 0, 1).saturating_sub(1);
                self.col = csiValue(&values, 1, 1).saturating_sub(1);
                self.ensureCursorLine();
            }
            'J' => self.eraseDisplay(csiValue(&values, 0, 0)),
            'K' => self.eraseLine(csiValue(&values, 0, 0)),
            'X' => self.eraseChars(csiValue(&values, 0, 1)),
            's' => {
                self.savedRow = self.row;
                self.savedCol = self.col;
            }
            'u' => {
                self.row = self.savedRow;
                self.col = self.savedCol;
                self.ensureCursorLine();
            }
            _ => {}
        }
    }

    /// Handles terminal controls before adding printable characters.
    fn writeChar(&mut self, ch: char) {
        match ch {
            '\r' => self.col = 0,
            '\n' => {
                self.row += 1;
                self.col = 0;
                self.ensureCursorLine();
            }
            '\x08' => {
                self.col = self.col.saturating_sub(1);
            }
            '\t' => {
                let nextStop = ((self.col / 8) + 1) * 8;
                while self.col < nextStop {
                    self.putChar(' ');
                }
            }
            _ if ch.is_control() => {}
            _ => self.putChar(ch),
        }
    }

    /// Writes a cell at the current cursor and advances one column.
    fn putChar(&mut self, ch: char) {
        self.ensureCursorLine();
        let line = &mut self.lines[self.row];
        while line.len() < self.col {
            line.push(' ');
        }
        if line.len() == self.col {
            line.push(ch);
        } else {
            line[self.col] = ch;
        }
        self.col += 1;
    }

    /// Moves upward without crossing the beginning of the transcript.
    fn moveRowUp(&mut self, count: usize) {
        self.row = self.row.saturating_sub(count);
        self.ensureCursorLine();
    }

    /// Moves downward while retaining intervening empty lines.
    fn moveRowDown(&mut self, count: usize) {
        self.row += count;
        self.ensureCursorLine();
    }

    /// Erases the requested transcript region using the existing cursor rules.
    fn eraseDisplay(&mut self, mode: usize) {
        match mode {
            0 => {
                self.eraseLine(0);
                self.lines.truncate(self.row + 1);
            }
            1 => {
                for row in 0..self.row {
                    self.lines[row].clear();
                }
                self.eraseLine(1);
            }
            2 | 3 => {
                self.lines.clear();
                self.lines.push(Vec::new());
                self.row = 0;
                self.col = 0;
            }
            _ => {}
        }
    }

    /// Erases the selected part of the current line.
    fn eraseLine(&mut self, mode: usize) {
        self.ensureCursorLine();
        let line = &mut self.lines[self.row];
        match mode {
            0 => {
                if self.col < line.len() {
                    line.truncate(self.col);
                }
            }
            1 => {
                let end = self.col.min(line.len());
                for cell in &mut line[..end] {
                    *cell = ' ';
                }
            }
            2 => line.clear(),
            _ => {}
        }
    }

    /// Replaces existing cells in the requested range with spaces.
    fn eraseChars(&mut self, count: usize) {
        self.ensureCursorLine();
        let line = &mut self.lines[self.row];
        let start = self.col.min(line.len());
        let end = start.saturating_add(count).min(line.len());
        for cell in &mut line[start..end] {
            *cell = ' ';
        }
    }

    /// Materializes empty lines up to the current cursor row.
    fn ensureCursorLine(&mut self) {
        while self.lines.len() <= self.row {
            self.lines.push(Vec::new());
        }
    }
}

/// Renders terminal controls independently of platform I/O and scheduling.
pub fn renderTerminalText(value: &str) -> String {
    let mut renderer = TerminalTextRenderer::new();
    renderer.write(value);
    renderer.text()
}

/// Decodes optional CSI parameters without changing their default semantics.
pub fn csiParams(params: &str) -> Vec<Option<usize>> {
    let params = params
        .trim_start_matches('?')
        .trim_start_matches('>')
        .trim_start_matches('!');
    params
        .split(';')
        .map(|value| {
            if value.is_empty() {
                None
            } else {
                value.parse::<usize>().ok()
            }
        })
        .collect()
}

/// Applies the terminal default when a CSI value is omitted.
pub fn csiValue(values: &[Option<usize>], index: usize, defaultValue: usize) -> usize {
    values
        .get(index)
        .and_then(|value| *value)
        .unwrap_or(defaultValue)
}
