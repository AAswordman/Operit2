use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::OperitTui;
use super::commands::{complete_command_input, matching_command_specs, TuiCommandSpec};
use super::helpers::{char_to_byte_index, wrap_approx_lines};

impl OperitTui {
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
                self.submit_input().await?;
            }
            (KeyCode::Char('j'), KeyModifiers::CONTROL) => self.insert_char('\n'),
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
        let prefix = self.input.chars().take(self.input_cursor).collect::<String>();
        let lines = wrap_approx_lines(&prefix, width.max(1));
        let visible_height = height.max(1);
        let line_index = lines.len().saturating_sub(1);
        let start = lines.len().saturating_sub(visible_height);
        let visible_line = line_index.saturating_sub(start);
        let col = lines.last().map(|line| line.chars().count()).unwrap_or(0);
        (
            col.min(width.saturating_sub(1)),
            visible_line.min(height.saturating_sub(1)),
        )
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
        let (input, cursor) = complete_command_input(&self.input, suggestions[index].name);
        self.input = input;
        self.input_cursor = cursor;
        self.autocomplete_index = 0;
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
}
