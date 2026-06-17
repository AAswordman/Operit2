use crossterm::event::MouseEvent;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme;

#[derive(Clone, Debug, Default)]
pub(super) struct TranscriptSelectionState {
    anchor: Option<TranscriptPosition>,
    cursor: Option<TranscriptPosition>,
    dragging: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TranscriptPosition {
    line: usize,
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

    fn normalized_range(&self) -> Option<(TranscriptPosition, TranscriptPosition)> {
        let anchor = self.anchor?;
        let cursor = self.cursor?;
        if anchor == cursor {
            return None;
        }
        if anchor < cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
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
    let column = mouse.column.saturating_sub(inner.x) as usize;
    let max_column = lines.get(line)?.text.chars().count();
    Some(TranscriptPosition {
        line,
        column: column.min(max_column),
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
    let column = mouse.column.clamp(inner.x, max_x).saturating_sub(inner.x) as usize;
    let visible_row = mouse.row.clamp(inner.y, max_y).saturating_sub(inner.y);
    let line = transcript_scroll as usize + visible_row as usize;
    let max_column = lines.get(line)?.text.chars().count();
    Some(TranscriptPosition {
        line,
        column: column.min(max_column),
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

fn transcript_inner_area(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
