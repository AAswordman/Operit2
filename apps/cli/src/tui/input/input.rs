use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};
use operit_runtime::data::model::AttachmentInfo::AttachmentInfo;
use std::fs;
use std::path::PathBuf;

use super::app::{OperitTui, QueuedAttachmentToken, QueuedAttachmentTokenKind};
use super::commands::{complete_command_input, matching_command_specs, TuiCommandSpec};
use super::helpers::{char_to_byte_index, display_width, wrap_approx_lines};
use crate::guess_mime_type;

const PASTE_ATTACHMENT_CHAR_THRESHOLD: usize = 2_048;
const PASTE_ATTACHMENT_LINE_THRESHOLD: usize = 8;

impl OperitTui {
    pub(super) async fn handle_paste(&mut self, text: String) -> Result<(), String> {
        if self.attach_pasted_paths(&text)? {
            return Ok(());
        }
        if is_large_paste(&text) {
            let attachment = self.create_paste_attachment(text);
            let token = format!("@{}", attachment.fileName);
            self.insert_attachment_token(&token);
            self.status_message = self
                .text()
                .attached_pasted_text(&attachment.fileName, &format_bytes(attachment.fileSize));
            self.queued_attachment_tokens.push(QueuedAttachmentToken {
                token,
                kind: QueuedAttachmentTokenKind::Inline {
                    file_path: attachment.filePath.clone(),
                },
            });
            self.queued_inline_attachments.push(attachment);
        } else {
            self.insert_text(&text);
        }
        Ok(())
    }

    pub(super) async fn handle_input_key(&mut self, key: KeyEvent) -> Result<(), String> {
        match (key.code, key.modifiers) {
            (KeyCode::Tab, _) if self.has_command_suggestions() => {
                self.complete_selected_command();
            }
            (KeyCode::Up, _) if self.has_command_suggestions() => {
                self.move_command_selection_up();
            }
            (KeyCode::Down, _) if self.has_command_suggestions() => {
                self.move_command_selection_down();
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if self.should_complete_selected_command_on_enter() {
                    self.complete_selected_command();
                } else {
                    self.submit_input().await?;
                }
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => self.insert_char('\n'),
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                self.attach_clipboard_image()?;
            }
            (KeyCode::Backspace, _) => self.delete_before_cursor(),
            (KeyCode::Delete, _) => self.delete_at_cursor(),
            (KeyCode::Left, _) => self.move_cursor_left(),
            (KeyCode::Right, _) => self.move_cursor_right(),
            (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.move_cursor_home()
            }
            (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.move_cursor_end()
            }
            (KeyCode::Char(ch), modifiers)
                if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
            {
                self.insert_char(ch);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn command_suggestions(&self) -> Vec<TuiCommandSpec> {
        matching_command_specs(&self.input)
    }

    pub(super) fn selected_command_index(&self, suggestions_len: usize) -> usize {
        if suggestions_len == 0 {
            0
        } else {
            self.autocomplete_index.min(suggestions_len - 1)
        }
    }

    pub(super) fn input_view_text(&self, width: usize, height: usize) -> String {
        let lines = wrap_approx_lines(&self.input, width.max(1));
        let visible_height = height.max(1);
        let start = lines.len().saturating_sub(visible_height);
        lines[start..].join("\n")
    }

    pub(super) fn cursor_position(&self, width: usize, height: usize) -> (usize, usize) {
        let prefix = self
            .input
            .chars()
            .take(self.input_cursor)
            .collect::<String>();
        let lines = wrap_approx_lines(&prefix, width.max(1));
        let visible_height = height.max(1);
        let line_index = lines.len().saturating_sub(1);
        let start = lines.len().saturating_sub(visible_height);
        let visible_line = line_index.saturating_sub(start);
        let col = lines.last().map(|line| display_width(line)).unwrap_or(0);
        (
            col.min(width.saturating_sub(1)),
            visible_line.min(height.saturating_sub(1)),
        )
    }

    pub(super) fn queued_attachment_labels(&self) -> Vec<String> {
        let mut labels = self.queued_attachment_paths.clone();
        labels.extend(self.queued_inline_attachments.iter().map(|attachment| {
            format!(
                "@{} ({})",
                attachment.fileName,
                format_bytes(attachment.fileSize)
            )
        }));
        labels
    }

    pub(super) fn clear_queued_attachments(&mut self) {
        let tokens = self
            .queued_attachment_tokens
            .iter()
            .map(|attachment| attachment.token.clone())
            .collect::<Vec<_>>();
        for token in tokens {
            self.input = self.input.replace(&token, "");
        }
        self.input_cursor = self.input_cursor.min(self.input.chars().count());
        self.queued_attachment_paths.clear();
        self.queued_inline_attachments.clear();
        self.queued_attachment_tokens.clear();
        self.autocomplete_index = 0;
    }

    fn has_command_suggestions(&self) -> bool {
        !self.command_suggestions().is_empty()
    }

    fn complete_selected_command(&mut self) {
        let suggestions = self.command_suggestions();
        if suggestions.is_empty() {
            return;
        }
        let index = self.selected_command_index(suggestions.len());
        let (input, cursor) = complete_command_input(&self.input, suggestions[index]);
        self.input = input;
        self.input_cursor = cursor;
        self.autocomplete_index = 0;
    }

    fn should_complete_selected_command_on_enter(&self) -> bool {
        let suggestions = self.command_suggestions();
        if suggestions.is_empty() || self.input.contains('\n') {
            return false;
        }
        let index = self.selected_command_index(suggestions.len());
        let current = self
            .input
            .strip_prefix('/')
            .map(str::trim_start)
            .unwrap_or("")
            .trim_end()
            .to_ascii_lowercase();
        !current.is_empty() && current != suggestions[index].name
    }

    fn move_command_selection_up(&mut self) {
        let suggestions_len = self.command_suggestions().len();
        if suggestions_len == 0 {
            return;
        }
        if self.autocomplete_index == 0 {
            self.autocomplete_index = suggestions_len - 1;
        } else {
            self.autocomplete_index -= 1;
        }
    }

    fn move_command_selection_down(&mut self) {
        let suggestions_len = self.command_suggestions().len();
        if suggestions_len == 0 {
            return;
        }
        self.autocomplete_index = (self.autocomplete_index + 1) % suggestions_len;
    }

    fn insert_char(&mut self, ch: char) {
        let byte_index = char_to_byte_index(&self.input, self.input_cursor);
        self.input.insert(byte_index, ch);
        self.input_cursor += 1;
        self.autocomplete_index = 0;
    }

    fn delete_before_cursor(&mut self) {
        if self.remove_attachment_token_before_cursor() {
            return;
        }
        if self.input_cursor == 0 {
            return;
        }
        let start = char_to_byte_index(&self.input, self.input_cursor - 1);
        let end = char_to_byte_index(&self.input, self.input_cursor);
        self.input.replace_range(start..end, "");
        self.input_cursor -= 1;
        self.autocomplete_index = 0;
    }

    fn delete_at_cursor(&mut self) {
        if self.remove_attachment_token_at_cursor() {
            return;
        }
        if self.input_cursor >= self.input.chars().count() {
            return;
        }
        let start = char_to_byte_index(&self.input, self.input_cursor);
        let end = char_to_byte_index(&self.input, self.input_cursor + 1);
        self.input.replace_range(start..end, "");
        self.autocomplete_index = 0;
    }

    fn move_cursor_left(&mut self) {
        self.input_cursor = self.input_cursor.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        let char_count = self.input.chars().count();
        if self.input_cursor < char_count {
            self.input_cursor += 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.input_cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.input_cursor = self.input.chars().count();
    }

    fn insert_text(&mut self, text: &str) {
        let byte_index = char_to_byte_index(&self.input, self.input_cursor);
        self.input.insert_str(byte_index, text);
        self.input_cursor += text.chars().count();
        self.autocomplete_index = 0;
    }

    fn insert_attachment_token(&mut self, token: &str) {
        let needs_prefix_space = self.input_cursor > 0
            && self
                .input
                .chars()
                .nth(self.input_cursor - 1)
                .map(|ch| !ch.is_whitespace())
                .unwrap_or(false);
        let needs_suffix_space = self
            .input
            .chars()
            .nth(self.input_cursor)
            .map(|ch| !ch.is_whitespace())
            .unwrap_or(false);
        let mut text = String::new();
        if needs_prefix_space {
            text.push(' ');
        }
        text.push_str(token);
        if needs_suffix_space {
            text.push(' ');
        }
        let byte_index = char_to_byte_index(&self.input, self.input_cursor);
        self.input.insert_str(byte_index, &text);
        self.input_cursor += token.chars().count() + usize::from(needs_prefix_space);
        self.autocomplete_index = 0;
    }

    fn create_paste_attachment(&mut self, content: String) -> AttachmentInfo {
        self.paste_attachment_counter += 1;
        let file_name = format!("pasted-text-{}.txt", self.paste_attachment_counter);
        AttachmentInfo {
            filePath: format!("tui-paste:{file_name}"),
            fileName: file_name,
            mimeType: "text/plain".to_string(),
            fileSize: content.as_bytes().len() as i64,
            content,
        }
    }

    fn create_clipboard_image_attachment(&mut self, png: Vec<u8>) -> AttachmentInfo {
        self.paste_attachment_counter += 1;
        let file_name = format!("clipboard-image-{}.png", self.paste_attachment_counter);
        let content = format!("data:image/png;base64,{}", BASE64.encode(&png));
        AttachmentInfo {
            filePath: format!("tui-clipboard:{file_name}"),
            fileName: file_name,
            mimeType: "image/png".to_string(),
            fileSize: png.len() as i64,
            content,
        }
    }

    fn attach_clipboard_image(&mut self) -> Result<(), String> {
        let Ok(mut clipboard) = arboard::Clipboard::new() else {
            self.status_message = self.text().clipboard_unavailable().to_string();
            return Ok(());
        };
        let Ok(image) = clipboard.get_image() else {
            self.status_message = self.text().clipboard_no_image().to_string();
            return Ok(());
        };
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(
                image.bytes.as_ref(),
                image.width as u32,
                image.height as u32,
                ColorType::Rgba8.into(),
            )
            .map_err(|error| error.to_string())?;
        let attachment = self.create_clipboard_image_attachment(png);
        let token = format!("@{}", attachment.fileName);
        self.insert_attachment_token(&token);
        self.status_message = self
            .text()
            .attached_clipboard_image(&attachment.fileName, &format_bytes(attachment.fileSize));
        self.queued_attachment_tokens.push(QueuedAttachmentToken {
            token,
            kind: QueuedAttachmentTokenKind::Inline {
                file_path: attachment.filePath.clone(),
            },
        });
        self.queued_inline_attachments.push(attachment);
        Ok(())
    }

    fn attach_pasted_paths(&mut self, text: &str) -> Result<bool, String> {
        let paths = pasted_file_paths(text);
        if paths.is_empty() {
            return Ok(false);
        }
        for path in paths {
            let display_path = path.to_string_lossy().to_string();
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| self.text().attachment_file_name_invalid(&display_path))?
                .to_string();
            let token = format!("@{file_name}");
            let mime_type = guess_mime_type(&display_path);
            if mime_type.starts_with("image/") {
                let bytes = fs::read(&path).map_err(|error| {
                    self.text()
                        .attachment_read_failed(&display_path, &error.to_string())
                })?;
                self.queued_inline_attachments.push(AttachmentInfo {
                    filePath: display_path.clone(),
                    fileName: file_name.clone(),
                    mimeType: mime_type.to_string(),
                    fileSize: bytes.len() as i64,
                    content: format!("data:{mime_type};base64,{}", BASE64.encode(bytes)),
                });
                self.queued_attachment_tokens.push(QueuedAttachmentToken {
                    token: token.clone(),
                    kind: QueuedAttachmentTokenKind::Inline {
                        file_path: display_path,
                    },
                });
            } else {
                self.queued_attachment_paths.push(display_path.clone());
                self.queued_attachment_tokens.push(QueuedAttachmentToken {
                    token: token.clone(),
                    kind: QueuedAttachmentTokenKind::Path { path: display_path },
                });
            }
            self.insert_attachment_token(&token);
        }
        self.status_message = self
            .text()
            .attached_files_total(self.queued_attachment_tokens.len());
        Ok(true)
    }

    fn remove_attachment_token_before_cursor(&mut self) -> bool {
        let cursor = self.input_cursor;
        let token_range = self
            .attachment_token_ranges()
            .into_iter()
            .find(|range| cursor > range.start && cursor <= range.end);
        self.remove_attachment_token_range(token_range)
    }

    fn remove_attachment_token_at_cursor(&mut self) -> bool {
        let cursor = self.input_cursor;
        let token_range = self
            .attachment_token_ranges()
            .into_iter()
            .find(|range| cursor >= range.start && cursor < range.end);
        self.remove_attachment_token_range(token_range)
    }

    fn remove_attachment_token_range(&mut self, token_range: Option<AttachmentTokenRange>) -> bool {
        let Some(token_range) = token_range else {
            return false;
        };
        let start = char_to_byte_index(&self.input, token_range.start);
        let end = char_to_byte_index(&self.input, token_range.end);
        self.input.replace_range(start..end, "");
        self.input_cursor = token_range.start.min(self.input.chars().count());
        match token_range.kind {
            QueuedAttachmentTokenKind::Path { path } => {
                self.queued_attachment_paths
                    .retain(|queued_path| queued_path != &path);
            }
            QueuedAttachmentTokenKind::Inline { file_path } => {
                self.queued_inline_attachments
                    .retain(|attachment| attachment.filePath.as_str() != file_path.as_str());
            }
        }
        self.queued_attachment_tokens.retain(|attachment_token| {
            attachment_token.token.as_str() != token_range.token.as_str()
        });
        self.autocomplete_index = 0;
        true
    }

    fn attachment_token_ranges(&self) -> Vec<AttachmentTokenRange> {
        let mut ranges = Vec::new();
        for attachment_token in &self.queued_attachment_tokens {
            let token = &attachment_token.token;
            let mut search_start = 0usize;
            while let Some(relative_start) = self.input[search_start..].find(token.as_str()) {
                let byte_start = search_start + relative_start;
                let byte_end = byte_start + token.len();
                let char_start = self.input[..byte_start].chars().count();
                let char_end = char_start + token.chars().count();
                ranges.push(AttachmentTokenRange {
                    start: char_start,
                    end: char_end,
                    token: token.clone(),
                    kind: attachment_token.kind.clone(),
                });
                search_start = byte_end;
            }
        }
        ranges
    }
}

#[derive(Clone, Debug)]
struct AttachmentTokenRange {
    start: usize,
    end: usize,
    token: String,
    kind: QueuedAttachmentTokenKind,
}

fn is_large_paste(text: &str) -> bool {
    text.chars().count() >= PASTE_ATTACHMENT_CHAR_THRESHOLD
        || text.lines().count() > PASTE_ATTACHMENT_LINE_THRESHOLD
}

fn format_bytes(size: i64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
    }
}

fn pasted_file_paths(text: &str) -> Vec<PathBuf> {
    let parts = parse_pasted_path_parts(text);
    if parts.is_empty() {
        return Vec::new();
    }
    let paths = parts.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    if paths.iter().all(|path| path.is_file()) {
        paths
    } else {
        Vec::new()
    }
}

fn parse_pasted_path_parts(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None::<char>;
    for ch in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
        match quote {
            Some(active_quote) => {
                if ch == active_quote {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                } else if ch.is_whitespace() {
                    push_pasted_path_part(&mut parts, &mut current);
                } else {
                    current.push(ch);
                }
            }
        }
    }
    push_pasted_path_part(&mut parts, &mut current);
    parts
}

fn push_pasted_path_part(parts: &mut Vec<String>, current: &mut String) {
    let value = current.trim();
    if !value.is_empty() {
        parts.push(value.to_string());
    }
    current.clear();
}
