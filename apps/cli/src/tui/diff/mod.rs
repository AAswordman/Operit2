use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::fs;
use std::process::Command;

use super::theme;

const ADD_BG: Color = theme::DIFF_ADD_BG;
const ADD_FG: Color = theme::DIFF_ADD_FG;
const ADD_HIGHLIGHT: Color = theme::DIFF_ADD_HIGHLIGHT;
const DELETE_BG: Color = theme::DIFF_DELETE_BG;
const DELETE_FG: Color = theme::DIFF_DELETE_FG;
const DELETE_HIGHLIGHT: Color = theme::DIFF_DELETE_HIGHLIGHT;
const HUNK_FG: Color = theme::DIFF_HUNK_FG;
const LINE_NUMBER_FG: Color = theme::TEXT_SUBTLE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffLineKind {
    Context,
    Delete,
    Insert,
    HunkHeader,
}

#[derive(Clone, Debug)]
struct DiffLine {
    kind: DiffLineKind,
    old_line: Option<usize>,
    new_line: Option<usize>,
    text: String,
    highlight_spans: Vec<(usize, usize)>,
}

#[derive(Clone, Debug)]
struct DiffHunk {
    old_start: usize,
    new_start: usize,
    lines: Vec<DiffLine>,
}

#[derive(Clone, Debug)]
pub struct FileDiffEntry {
    path: String,
    additions: usize,
    deletions: usize,
    hunks: Vec<DiffHunk>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffViewMode {
    Unified,
    SideBySide,
}

#[derive(Clone, Debug)]
pub struct DiffViewerModal {
    files: Vec<FileDiffEntry>,
    selected_file: usize,
    view_mode: DiffViewMode,
    scroll_offset: usize,
}

impl DiffViewerModal {
    pub fn new(files: Vec<FileDiffEntry>) -> Self {
        Self {
            files,
            selected_file: 0,
            view_mode: DiffViewMode::Unified,
            scroll_offset: 0,
        }
    }

    fn current_file(&self) -> Option<&FileDiffEntry> {
        self.files.get(self.selected_file)
    }

    pub(super) fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub(super) fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub(super) fn prev_file(&mut self) {
        self.selected_file = self.selected_file.saturating_sub(1);
        self.scroll_offset = 0;
    }

    pub(super) fn next_file(&mut self) {
        if self.selected_file + 1 < self.files.len() {
            self.selected_file += 1;
            self.scroll_offset = 0;
        }
    }

    pub(super) fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            DiffViewMode::Unified => DiffViewMode::SideBySide,
            DiffViewMode::SideBySide => DiffViewMode::Unified,
        };
        self.scroll_offset = 0;
    }

    pub(super) fn open_external(&mut self) -> Result<(), String> {
        let entry = self.current_file().ok_or_else(|| "no file selected".to_string())?;
        let safe_name = entry
            .path
            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        let old_path = std::env::temp_dir().join(format!("operit_diff_old_{safe_name}"));
        let new_path = std::env::temp_dir().join(format!("operit_diff_new_{safe_name}"));

        let (old_content, new_content) = reconstruct_file_contents(&entry.hunks);
        fs::write(&old_path, old_content).map_err(|error| format!("write old snapshot: {error}"))?;
        fs::write(&new_path, new_content).map_err(|error| format!("write new snapshot: {error}"))?;

        let old_path = old_path.to_string_lossy().to_string();
        let new_path = new_path.to_string_lossy().to_string();
        Command::new("code")
            .arg("--diff")
            .arg(&old_path)
            .arg(&new_path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("launch VS Code diff: {error}"))
    }
}

pub fn parse_file_diffs_from_text(text: &str) -> Vec<FileDiffEntry> {
    let mut files = Vec::new();
    let mut search_from = 0;
    while let Some(open_offset) = text[search_from..].find("<file-diff") {
        let open_index = search_from + open_offset;
        let Some(header_end) = text[open_index..].find('>') else {
            break;
        };
        let Some(close_offset) = text[open_index..].find("</file-diff>") else {
            break;
        };
        let header = &text[open_index..open_index + header_end];
        let body_start = open_index + header_end + 1;
        let body_end = open_index + close_offset;
        let mut body = &text[body_start..body_end];
        if let Some(cdata_start) = body.find("<![CDATA[") {
            if let Some(cdata_end) = body.find("]]>") {
                body = &body[cdata_start + 9..cdata_end];
            }
        }

        let path = xml_attribute(header, "path").unwrap_or_else(|| "unknown".to_string());
        files.push(parse_unified_diff(body.trim(), &path));
        search_from = body_end + "</file-diff>".len();
    }
    files
}

fn xml_attribute(header: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=\"");
    let start = header.find(&prefix)? + prefix.len();
    let rest = &header[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_unified_diff(raw_diff: &str, path: &str) -> FileDiffEntry {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut additions = 0;
    let mut deletions = 0;
    let mut old_line = 1;
    let mut new_line = 1;

    for line in raw_diff.lines() {
        if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            let (next_old, next_new) = parse_hunk_start(line);
            old_line = next_old;
            new_line = next_new;
            current_hunk = Some(DiffHunk {
                old_start: next_old,
                new_start: next_new,
                lines: vec![DiffLine {
                    kind: DiffLineKind::HunkHeader,
                    old_line: None,
                    new_line: None,
                    text: line.to_string(),
                    highlight_spans: Vec::new(),
                }],
            });
            continue;
        }
        if line.starts_with("--- ") || line.starts_with("+++ ") {
            continue;
        }

        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };
        if let Some(text) = line.strip_prefix('+') {
            additions += 1;
            hunk.lines.push(DiffLine {
                kind: DiffLineKind::Insert,
                old_line: None,
                new_line: Some(new_line),
                text: text.to_string(),
                highlight_spans: Vec::new(),
            });
            new_line += 1;
        } else if let Some(text) = line.strip_prefix('-') {
            deletions += 1;
            hunk.lines.push(DiffLine {
                kind: DiffLineKind::Delete,
                old_line: Some(old_line),
                new_line: None,
                text: text.to_string(),
                highlight_spans: Vec::new(),
            });
            old_line += 1;
        } else {
            let text = line.strip_prefix(' ').unwrap_or(line);
            hunk.lines.push(DiffLine {
                kind: DiffLineKind::Context,
                old_line: Some(old_line),
                new_line: Some(new_line),
                text: text.to_string(),
                highlight_spans: Vec::new(),
            });
            old_line += 1;
            new_line += 1;
        }
    }
    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    for hunk in &mut hunks {
        apply_intra_line_highlights(&mut hunk.lines);
    }

    FileDiffEntry {
        path: path.to_string(),
        additions,
        deletions,
        hunks,
    }
}

fn parse_hunk_start(header: &str) -> (usize, usize) {
    let mut parts = header.split_whitespace().skip(1);
    let old = parse_line_number(parts.next(), '-');
    let new = parse_line_number(parts.next(), '+');
    (old, new)
}

fn parse_line_number(value: Option<&str>, prefix: char) -> usize {
    value
        .and_then(|value| value.strip_prefix(prefix))
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.parse().ok())
        .unwrap_or(1)
}

fn apply_intra_line_highlights(lines: &mut [DiffLine]) {
    let mut index = 0;
    while index + 1 < lines.len() {
        if lines[index].kind == DiffLineKind::Delete
            && lines[index + 1].kind == DiffLineKind::Insert
        {
            let old_text = lines[index].text.clone();
            let new_text = lines[index + 1].text.clone();
            if let Some((old_spans, new_spans)) =
                operit_providers::chat::enhance::FileBindingService::compute_intra_line_spans(
                    &old_text,
                    &new_text,
                )
            {
                lines[index].highlight_spans = old_spans;
                lines[index + 1].highlight_spans = new_spans;
            }
            index += 2;
        } else {
            index += 1;
        }
    }
}

fn reconstruct_file_contents(hunks: &[DiffHunk]) -> (String, String) {
    let mut old = String::new();
    let mut new = String::new();
    for hunk in hunks {
        for line in &hunk.lines {
            match line.kind {
                DiffLineKind::Context => {
                    old.push_str(&line.text);
                    old.push('\n');
                    new.push_str(&line.text);
                    new.push('\n');
                }
                DiffLineKind::Delete => {
                    old.push_str(&line.text);
                    old.push('\n');
                }
                DiffLineKind::Insert => {
                    new.push_str(&line.text);
                    new.push('\n');
                }
                DiffLineKind::HunkHeader => {}
            }
        }
    }
    (old, new)
}

pub fn render_inline_diff_card(entry: &FileDiffEntry, max_lines: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        format!("  [diff] {} (+{}, -{})", entry.path, entry.additions, entry.deletions),
        Style::default()
            .fg(theme::ACCENT_STRONG)
            .add_modifier(Modifier::BOLD),
    ))];

    let mut rendered = 0;
    let total: usize = entry.hunks.iter().map(|hunk| hunk.lines.len()).sum();
    for hunk in &entry.hunks {
        for line in &hunk.lines {
            if rendered >= max_lines {
                break;
            }
            rendered += 1;
            lines.push(render_unified_line(line, true));
        }
    }
    if total > max_lines {
        lines.push(Line::from(Span::styled(
            format!("  ... {} more lines; run /diff", total - max_lines),
            Style::default().fg(theme::ACCENT_DIM),
        )));
    }
    lines
}

fn render_unified_line(line: &DiffLine, inline: bool) -> Line<'static> {
    let prefix = if inline { "  " } else { "" };
    match line.kind {
        DiffLineKind::HunkHeader => Line::from(Span::styled(
            format!("{prefix}{}", line.text),
            Style::default().fg(HUNK_FG).add_modifier(Modifier::DIM),
        )),
        DiffLineKind::Context => {
            let old = line.old_line.map(|value| format!("{value:>4}")).unwrap_or_default();
            let new = line.new_line.map(|value| format!("{value:>4}")).unwrap_or_default();
            Line::from(vec![
                Span::styled(format!("{prefix}{old} {new} "), Style::default().fg(LINE_NUMBER_FG)),
                Span::styled(format!(" {}", line.text), Style::default().fg(theme::TEXT_MUTED)),
            ])
        }
        DiffLineKind::Delete => {
            let old = line.old_line.map(|value| format!("{value:>4}")).unwrap_or_default();
            let mut spans = vec![
                Span::styled(format!("{prefix}{old}     "), Style::default().fg(LINE_NUMBER_FG)),
                Span::styled("-".to_string(), Style::default().fg(DELETE_FG)),
            ];
            spans.extend(highlighted_text(
                &line.text,
                &line.highlight_spans,
                DELETE_FG,
                DELETE_BG,
                DELETE_HIGHLIGHT,
            ));
            Line::from(spans)
        }
        DiffLineKind::Insert => {
            let new = line.new_line.map(|value| format!("{value:>4}")).unwrap_or_default();
            let mut spans = vec![
                Span::styled(format!("{prefix}    {new} "), Style::default().fg(LINE_NUMBER_FG)),
                Span::styled("+".to_string(), Style::default().fg(ADD_FG)),
            ];
            spans.extend(highlighted_text(
                &line.text,
                &line.highlight_spans,
                ADD_FG,
                ADD_BG,
                ADD_HIGHLIGHT,
            ));
            Line::from(spans)
        }
    }
}

fn highlighted_text(
    text: &str,
    spans: &[(usize, usize)],
    foreground: Color,
    background: Color,
    highlight_background: Color,
) -> Vec<Span<'static>> {
    if spans.is_empty() {
        return vec![Span::styled(
            text.to_string(),
            Style::default().fg(foreground).bg(background),
        )];
    }

    let mut result = Vec::new();
    let mut cursor = 0;
    for (start, end) in spans {
        if *start > cursor {
            result.push(Span::styled(
                text[cursor..*start].to_string(),
                Style::default().fg(foreground).bg(background),
            ));
        }
        result.push(Span::styled(
            text[(*start..*end)].to_string(),
            Style::default()
                .fg(foreground)
                .bg(highlight_background)
                .add_modifier(Modifier::BOLD),
        ));
        cursor = *end;
    }
    if cursor < text.len() {
        result.push(Span::styled(
            text[cursor..].to_string(),
            Style::default().fg(foreground).bg(background),
        ));
    }
    result
}

pub fn render_diff_viewer_modal(modal: &DiffViewerModal, frame: &mut Frame) {
    let area = centered_rect(92, 88, frame.area());
    frame.render_widget(Clear, area);
    let mode = match modal.view_mode {
        DiffViewMode::Unified => "Unified",
        DiffViewMode::SideBySide => "Split",
    };
    let block = Block::default()
        .title(format!(" Diff [{mode}] · Tab mode · [ ] files · e VS Code · q close "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(24), Constraint::Percentage(76)])
        .split(inner);

    render_file_list(modal, frame, columns[0]);
    render_diff_body(modal, frame, columns[1]);
}

fn render_file_list(modal: &DiffViewerModal, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme::TEXT_SUBTLE));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = modal
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let selected = index == modal.selected_file;
            let name = file.path.rsplit(['/', '\\']).next().unwrap_or(&file.path);
            Line::from(Span::styled(
                format!(
                    "{}{name} +{} -{}",
                    if selected { ">> " } else { "   " },
                    file.additions,
                    file.deletions
                ),
                if selected {
                    Style::default().fg(theme::ACCENT_STRONG).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                },
            ))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn render_diff_body(modal: &DiffViewerModal, frame: &mut Frame, area: Rect) {
    let Some(file) = modal.current_file() else {
        return;
    };
    match modal.view_mode {
        DiffViewMode::Unified => {
            let lines = file
                .hunks
                .iter()
                .flat_map(|hunk| hunk.lines.iter().map(|line| render_unified_line(line, false)))
                .collect::<Vec<_>>();
            frame.render_widget(
                Paragraph::new(lines).scroll((modal.scroll_offset as u16, 0)),
                area,
            );
        }
        DiffViewMode::SideBySide => render_split_view(file, modal.scroll_offset, frame, area),
    }
}

fn render_split_view(file: &FileDiffEntry, scroll: usize, frame: &mut Frame, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let mut left = Vec::new();
    let mut right = Vec::new();

    for hunk in &file.hunks {
        for line in &hunk.lines {
            match line.kind {
                DiffLineKind::HunkHeader => {
                    left.push(Line::from(Span::styled(
                        line.text.clone(),
                        Style::default().fg(HUNK_FG),
                    )));
                    right.push(Line::from(Span::styled(
                        line.text.clone(),
                        Style::default().fg(HUNK_FG),
                    )));
                }
                DiffLineKind::Context => {
                    left.push(Line::from(Span::styled(
                        line.text.clone(),
                        Style::default().fg(theme::TEXT),
                    )));
                    right.push(Line::from(Span::styled(
                        line.text.clone(),
                        Style::default().fg(theme::TEXT),
                    )));
                }
                DiffLineKind::Delete => {
                    left.push(Line::from(highlighted_text(
                        &line.text,
                        &line.highlight_spans,
                        DELETE_FG,
                        DELETE_BG,
                        DELETE_HIGHLIGHT,
                    )));
                    right.push(Line::from(""));
                }
                DiffLineKind::Insert => {
                    left.push(Line::from(""));
                    right.push(Line::from(highlighted_text(
                        &line.text,
                        &line.highlight_spans,
                        ADD_FG,
                        ADD_BG,
                        ADD_HIGHLIGHT,
                    )));
                }
            }
        }
    }

    frame.render_widget(
        Paragraph::new(left).scroll((scroll as u16, 0)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(right).scroll((scroll as u16, 0)),
        columns[1],
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
