use crossterm::event::MouseEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::helpers::cell_column_to_char_index;
use super::scrollbar::split_transcript_inner;
use super::theme;

#[derive(Clone, Debug, Default)]
pub(super) struct TranscriptSelectionState {
    anchor: Option<TranscriptPosition>,
    cursor: Option<TranscriptPosition>,
    dragging: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TranscriptPosition {
    pub(super) line: usize,
    column: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct TranscriptCopyLine {
    text: String,
    soft_wrap_continuation: bool,
    copy_start_column: usize,
}

impl TranscriptSelectionState {
    pub(super) fn begin(&mut self, position: TranscriptPosition) {
        self.anchor = Some(position);
        self.cursor = Some(position);
        self.dragging = true;
    }

    pub(super) fn drag_to(&mut self, position: TranscriptPosition) {
        if self.dragging {
            self.cursor = Some(position);
        }
    }

    pub(super) fn end(&mut self, position: TranscriptPosition) {
        if self.dragging {
            self.cursor = Some(position);
        }
        self.dragging = false;
        if self.normalized_range().is_none() {
            self.clear();
        }
    }

    /// Returns whether the current pointer gesture is a click without a drag range.
    pub(super) fn is_click(&self) -> bool {
        matches!((self.anchor, self.cursor), (Some(anchor), Some(cursor)) if anchor == cursor)
    }

    /// Whether a press-drag gesture is currently in progress.
    pub(super) fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub(super) fn clear(&mut self) {
        self.anchor = None;
        self.cursor = None;
        self.dragging = false;
    }

    pub(super) fn selected_text(&self, lines: &[TranscriptCopyLine]) -> Option<String> {
        let (start, end) = self.normalized_range()?;
        let mut selected = String::new();
        for line_index in start.line..=end.line {
            let line = lines.get(line_index)?;
            let start_column = if line_index == start.line {
                start.column
            } else {
                0
            };
            let end_column = if line_index == end.line {
                end.column
            } else {
                line.text.chars().count()
            };
            if line_index > start.line && !line.soft_wrap_continuation {
                selected.push('\n');
            }
            selected.push_str(&line.selection_text(start_column, end_column));
        }
        Some(selected)
    }

    /// Returns the normalized (start ≤ end) selection range.
    ///
    /// The range spans exactly the characters under the press and release
    /// pointers, both included, regardless of drag direction. A gesture that
    /// starts and ends on the same character therefore selects exactly that
    /// one character.
    pub(super) fn normalized_range(&self) -> Option<(TranscriptPosition, TranscriptPosition)> {
        let anchor = self.anchor?;
        let cursor = self.cursor?;
        if anchor <= cursor {
            Some((anchor, inclusive_end(cursor)))
        } else {
            Some((cursor, inclusive_end(anchor)))
        }
    }
}

pub(super) fn transcript_copy_line(line: &Line<'static>) -> TranscriptCopyLine {
    let mut text = String::new();
    let mut soft_wrap_continuation = false;
    let mut copy_start_column = 0usize;
    for span in &line.spans {
        if is_soft_wrap_marker(span) {
            soft_wrap_continuation = true;
            copy_start_column = text.chars().count();
        } else {
            text.push_str(span.content.as_ref());
        }
    }
    TranscriptCopyLine {
        text,
        soft_wrap_continuation,
        copy_start_column,
    }
}

pub(super) fn mark_soft_wrap_continuation(line: &mut Line<'static>) {
    line.spans.insert(
        0,
        Span::styled("", Style::default().add_modifier(Modifier::HIDDEN)),
    );
}

fn is_soft_wrap_marker(span: &Span<'static>) -> bool {
    span.content.is_empty() && span.style.add_modifier.contains(Modifier::HIDDEN)
}

impl TranscriptCopyLine {
    fn selection_text(&self, start_column: usize, end_column: usize) -> String {
        let start_column = if self.soft_wrap_continuation {
            start_column.max(self.copy_start_column)
        } else {
            start_column
        };
        let end_column = if self.soft_wrap_continuation {
            end_column.max(self.copy_start_column)
        } else {
            end_column
        };
        slice_columns(&self.text, start_column, end_column)
    }
}

fn line_text_column_count(line: &Line<'static>) -> usize {
    line.spans
        .iter()
        .filter(|span| !is_soft_wrap_marker(span))
        .map(|span| span.content.chars().count())
        .sum()
}

pub(super) fn mouse_transcript_position(
    mouse: MouseEvent,
    transcript_area: Rect,
    transcript_scroll: u16,
    lines: &[TranscriptCopyLine],
) -> Option<TranscriptPosition> {
    let inner = transcript_inner_area(transcript_area);
    if inner.width == 0 || inner.height == 0 {
        return None;
    }
    if mouse.column < inner.x || mouse.column >= inner.x.saturating_add(inner.width) {
        return None;
    }
    let visible_row = mouse.row.saturating_sub(inner.y).min(inner.height - 1);
    let line = transcript_scroll as usize + visible_row as usize;
    let text = &lines.get(line)?.text;
    let cell_column = mouse.column.saturating_sub(inner.x) as usize;
    Some(TranscriptPosition {
        line,
        column: cell_column_to_char_index(text, cell_column),
    })
}

pub(super) fn mouse_drag_transcript_position(
    mouse: MouseEvent,
    transcript_area: Rect,
    transcript_scroll: u16,
    lines: &[TranscriptCopyLine],
) -> Option<TranscriptPosition> {
    let inner = transcript_inner_area(transcript_area);
    if inner.width == 0 || inner.height == 0 {
        return None;
    }
    let max_x = inner.x.saturating_add(inner.width).saturating_sub(1);
    let max_y = inner.y.saturating_add(inner.height).saturating_sub(1);
    let visible_row = mouse.row.clamp(inner.y, max_y).saturating_sub(inner.y);
    let line = transcript_scroll as usize + visible_row as usize;
    let text = &lines.get(line)?.text;
    let cell_column = mouse.column.clamp(inner.x, max_x).saturating_sub(inner.x) as usize;
    Some(TranscriptPosition {
        line,
        column: cell_column_to_char_index(text, cell_column),
    })
}

pub(super) fn apply_transcript_selection(
    lines: &mut [Line<'static>],
    selection: &TranscriptSelectionState,
) {
    let Some((start, end)) = selection.normalized_range() else {
        return;
    };
    for line_index in start.line..=end.line {
        let Some(line) = lines.get_mut(line_index) else {
            continue;
        };
        let start_column = if line_index == start.line {
            start.column
        } else {
            0
        };
        let end_column = if line_index == end.line {
            end.column
        } else {
            line_text_column_count(line)
        };
        highlight_line(line, start_column, end_column);
    }
}

fn inclusive_end(position: TranscriptPosition) -> TranscriptPosition {
    TranscriptPosition {
        line: position.line,
        column: position.column + 1,
    }
}

fn transcript_inner_area(area: Rect) -> Rect {
    split_transcript_inner(area).content
}

fn slice_columns(line: &str, start_column: usize, end_column: usize) -> String {
    line.chars()
        .enumerate()
        .filter_map(|(index, ch)| (index >= start_column && index < end_column).then_some(ch))
        .collect()
}

fn highlight_line(line: &mut Line<'static>, start_column: usize, end_column: usize) {
    if start_column >= end_column {
        return;
    }
    let mut next_spans = Vec::new();
    let mut column = 0usize;
    for span in line.spans.drain(..) {
        let mut selected_text = String::new();
        let mut normal_text = String::new();
        for ch in span.content.chars() {
            let selected = column >= start_column && column < end_column;
            if selected {
                flush_span(&mut next_spans, &mut normal_text, span.style);
                selected_text.push(ch);
            } else {
                flush_span(
                    &mut next_spans,
                    &mut selected_text,
                    selection_style(span.style),
                );
                normal_text.push(ch);
            }
            column += 1;
        }
        flush_span(&mut next_spans, &mut normal_text, span.style);
        flush_span(
            &mut next_spans,
            &mut selected_text,
            selection_style(span.style),
        );
    }
    line.spans = next_spans;
}

fn flush_span(spans: &mut Vec<Span<'static>>, text: &mut String, style: Style) {
    if text.is_empty() {
        return;
    }
    spans.push(Span::styled(std::mem::take(text), style));
}

fn selection_style(style: Style) -> Style {
    style.bg(theme::SELECTION_BG).fg(theme::SELECTION_TEXT)
}

/// Captures the rendered cell symbols of a popup's content area, one row per
/// visual line with trailing blank cells trimmed.
pub(super) fn popup_rows_from_buffer(buffer: &Buffer, area: Rect) -> Vec<Vec<String>> {
    let mut rows = Vec::with_capacity(area.height as usize);
    for y in area.y..area.bottom() {
        let mut row = Vec::with_capacity(area.width as usize);
        for x in area.x..area.right() {
            row.push(buffer[(x, y)].symbol().to_string());
        }
        while row
            .last()
            .is_some_and(|cell| cell.is_empty() || cell == " ")
        {
            row.pop();
        }
        rows.push(row);
    }
    rows
}

/// Joins the selected cell range across popup rows; rows are newline separated.
pub(super) fn popup_selected_text(
    rows: &[Vec<String>],
    selection: &TranscriptSelectionState,
) -> Option<String> {
    let (start, end) = selection.normalized_range()?;
    let mut selected = String::new();
    for line_index in start.line..=end.line {
        let row = rows.get(line_index)?;
        if line_index > start.line {
            selected.push('\n');
        }
        let start_column = if line_index == start.line {
            start.column.min(row.len())
        } else {
            0
        };
        let end_column = if line_index == end.line {
            end.column.min(row.len())
        } else {
            row.len()
        };
        for cell in row.iter().take(end_column).skip(start_column) {
            selected.push_str(cell);
        }
    }
    Some(selected)
}

/// Restyles the selected cell range of a popup with the transcript selection colors.
pub(super) fn apply_popup_selection_highlight(
    buffer: &mut Buffer,
    area: Rect,
    selection: &TranscriptSelectionState,
) {
    let Some((start, end)) = selection.normalized_range() else {
        return;
    };
    for line_index in start.line..=end.line {
        let Ok(row_offset) = u16::try_from(line_index) else {
            continue;
        };
        if row_offset >= area.height {
            continue;
        }
        let y = area.y + row_offset;
        let start_column = if line_index == start.line {
            start.column
        } else {
            0
        };
        let end_column = if line_index == end.line {
            end.column
        } else {
            area.width as usize
        };
        let start_column = (start_column as u16).min(area.width);
        let end_column = (end_column as u16).min(area.width);
        if start_column >= end_column {
            continue;
        }
        let row = Rect {
            x: area.x + start_column,
            y,
            width: end_column - start_column,
            height: 1,
        };
        buffer.set_style(row, selection_style(Style::default()));
    }
}

/// Maps a pointer inside a popup's content area to a cell position.
pub(super) fn mouse_popup_position(
    column: u16,
    row: u16,
    area: Rect,
    rows: &[Vec<String>],
) -> Option<TranscriptPosition> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    if column < area.x
        || column >= area.x.saturating_add(area.width)
        || row < area.y
        || row >= area.y.saturating_add(area.height)
    {
        return None;
    }
    let line = usize::from(row - area.y);
    let row_cells = rows.get(line)?;
    let cell = usize::from(column - area.x);
    Some(TranscriptPosition {
        line,
        column: cell.min(row_cells.len()),
    })
}

/// Like [`mouse_popup_position`] but clamps outside pointers to the popup edges,
/// matching transcript drag behavior.
pub(super) fn mouse_popup_drag_position(
    column: u16,
    row: u16,
    area: Rect,
    rows: &[Vec<String>],
) -> Option<TranscriptPosition> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let max_x = area.x.saturating_add(area.width).saturating_sub(1);
    let max_y = area.y.saturating_add(area.height).saturating_sub(1);
    let line = usize::from(row.clamp(area.y, max_y) - area.y);
    let row_cells = rows.get(line)?;
    let cell = usize::from(column.clamp(area.x, max_x) - area.x);
    Some(TranscriptPosition {
        line,
        column: cell.min(row_cells.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    fn left_click_at(column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    #[test]
    fn transcript_mouse_position_maps_cells_to_char_indices() {
        let lines = vec![transcript_copy_line(&Line::from("中文abc"))];
        let area = Rect {
            x: 2,
            y: 3,
            width: 12,
            height: 4,
        };
        let inner = split_transcript_inner(area).content;
        let position_at = |cell: u16| {
            mouse_transcript_position(left_click_at(inner.x + cell, inner.y), area, 0, &lines)
        };
        // "中" and "文" each occupy two cells; "a", "b", "c" occupy one each.
        assert_eq!(
            position_at(0),
            Some(TranscriptPosition { line: 0, column: 0 })
        );
        assert_eq!(
            position_at(1),
            Some(TranscriptPosition { line: 0, column: 0 })
        );
        assert_eq!(
            position_at(2),
            Some(TranscriptPosition { line: 0, column: 1 })
        );
        assert_eq!(
            position_at(4),
            Some(TranscriptPosition { line: 0, column: 2 })
        );
        assert_eq!(
            position_at(6),
            Some(TranscriptPosition { line: 0, column: 4 })
        );
        // Past the end clamps to the char count.
        assert_eq!(
            position_at(8),
            Some(TranscriptPosition { line: 0, column: 5 })
        );
    }

    #[test]
    fn transcript_mouse_drag_selects_wide_char_range() {
        let lines = vec![transcript_copy_line(&Line::from("中文abc"))];
        let area = Rect {
            x: 0,
            y: 0,
            width: 12,
            height: 4,
        };
        let inner = split_transcript_inner(area).content;
        let drag_position_at = |cell: u16| {
            mouse_drag_transcript_position(
                MouseEvent {
                    kind: MouseEventKind::Drag(MouseButton::Left),
                    column: inner.x + cell,
                    row: inner.y,
                    modifiers: KeyModifiers::empty(),
                },
                area,
                0,
                &lines,
            )
        };

        let mut selection = TranscriptSelectionState::default();
        selection.begin(drag_position_at(2).expect("position inside transcript"));
        selection.end(drag_position_at(6).expect("position inside transcript"));
        assert_eq!(selection.selected_text(&lines).as_deref(), Some("文abc"));
    }

    #[test]
    fn transcript_drag_selects_characters_under_both_pointers() {
        let lines = vec![transcript_copy_line(&Line::from("中文abc"))];

        // Press on "文" (col 1), release on "中" (col 0): both endpoint
        // characters are selected regardless of drag direction.
        let mut leftward = TranscriptSelectionState::default();
        leftward.begin(TranscriptPosition { line: 0, column: 2 });
        leftward.end(TranscriptPosition { line: 0, column: 0 });
        assert_eq!(
            leftward.selected_text(&lines).as_deref(),
            Some("中文a")
        );

        // Press on "文", release one character right on "a": the span covers
        // exactly the two endpoint characters.
        let mut rightward = TranscriptSelectionState::default();
        rightward.begin(TranscriptPosition { line: 0, column: 1 });
        rightward.end(TranscriptPosition { line: 0, column: 2 });
        assert_eq!(rightward.selected_text(&lines).as_deref(), Some("文a"));
    }

    #[test]
    fn transcript_click_selects_single_character() {
        let lines = vec![transcript_copy_line(&Line::from("中文abc"))];
        let mut selection = TranscriptSelectionState::default();
        selection.begin(TranscriptPosition { line: 0, column: 1 });
        selection.end(TranscriptPosition { line: 0, column: 1 });
        assert_eq!(selection.selected_text(&lines).as_deref(), Some("文"));
    }

    #[test]
    fn selected_text_joins_soft_wrap_continuation_without_visual_indent() {
        let first = Line::from("  D:/Code/prog/assista");
        let mut second = Line::from("nce2/apps/cli");
        mark_soft_wrap_continuation(&mut second);
        second.spans.insert(0, Span::raw("  "));
        let lines = vec![transcript_copy_line(&first), transcript_copy_line(&second)];

        let mut selection = TranscriptSelectionState::default();
        selection.begin(TranscriptPosition { line: 0, column: 2 });
        selection.end(TranscriptPosition {
            line: 1,
            column: lines[1].text.chars().count(),
        });

        assert_eq!(
            selection.selected_text(&lines).as_deref(),
            Some("D:/Code/prog/assistance2/apps/cli")
        );
    }

    #[test]
    fn popup_rows_snapshot_and_copy_cell_ranges() {
        let area = Rect {
            x: 2,
            y: 3,
            width: 6,
            height: 2,
        };
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        });
        buffer[(2, 3)].set_symbol("a");
        buffer[(3, 3)].set_symbol("b");
        buffer[(4, 3)].set_symbol("中");
        buffer[(2, 4)].set_symbol("x");
        buffer[(3, 4)].set_symbol("y");

        let rows = popup_rows_from_buffer(&buffer, area);
        assert_eq!(rows[0], vec!["a", "b", "中"]);
        assert_eq!(rows[1], vec!["x", "y"]);

        let mut selection = TranscriptSelectionState::default();
        selection.begin(TranscriptPosition { line: 0, column: 2 });
        selection.end(TranscriptPosition {
            line: 1,
            column: 2,
        });
        assert_eq!(
            popup_selected_text(&rows, &selection).as_deref(),
            Some("中\nxy")
        );

        let mut release_inclusive = TranscriptSelectionState::default();
        release_inclusive.begin(TranscriptPosition { line: 0, column: 2 });
        release_inclusive.end(TranscriptPosition { line: 1, column: 0 });
        assert_eq!(
            popup_selected_text(&rows, &release_inclusive).as_deref(),
            Some("中\nx")
        );

        let mut single_cell = TranscriptSelectionState::default();
        single_cell.begin(TranscriptPosition { line: 0, column: 2 });
        single_cell.end(TranscriptPosition { line: 0, column: 2 });
        assert_eq!(popup_selected_text(&rows, &single_cell).as_deref(), Some("中"));
    }

    #[test]
    fn popup_mouse_positions_clamp_to_row_content() {
        let area = Rect {
            x: 1,
            y: 1,
            width: 5,
            height: 2,
        };
        let rows = vec![vec!["a".to_string(), "b".to_string()], Vec::new()];
        assert!(mouse_popup_position(0, 1, area, &rows).is_none());
        assert_eq!(
            mouse_popup_position(4, 2, area, &rows),
            Some(TranscriptPosition {
                line: 1,
                column: 0
            })
        );
        assert_eq!(
            mouse_popup_drag_position(99, 99, area, &rows),
            Some(TranscriptPosition {
                line: 1,
                column: 0
            })
        );
    }
}
