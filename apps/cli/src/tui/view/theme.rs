use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};

pub(super) const ACCENT: Color = Color::Rgb(96, 165, 250);
pub(super) const ACCENT_STRONG: Color = Color::Rgb(147, 197, 253);
pub(super) const ACCENT_DIM: Color = Color::Rgb(59, 130, 246);
pub(super) const ACCENT_BG: Color = Color::Rgb(30, 64, 105);

pub(super) const TEXT: Color = Color::Rgb(226, 232, 240);
pub(super) const TEXT_MUTED: Color = Color::Rgb(148, 163, 184);
pub(super) const TEXT_SUBTLE: Color = Color::Rgb(100, 116, 139);
/// Prose inside an expanded fold body: readable, but one step below the
/// conversation text it was folded out of.
pub(super) const FOLD_BODY_TEXT: Color = Color::Rgb(178, 188, 202);
pub(super) const TEXT_INVERTED: Color = Color::Rgb(15, 23, 42);

/// Chrome border for panels that are visible but not focused (slate-600).
pub(super) const BORDER: Color = Color::Rgb(71, 85, 105);
/// Quietest chrome border, for large passive surfaces (slate-700).
pub(super) const BORDER_MUTED: Color = Color::Rgb(51, 65, 85);

pub(super) const SELECTION_BG: Color = Color::Rgb(92, 92, 92);
pub(super) const SELECTION_TEXT: Color = Color::Rgb(245, 245, 245);

pub(super) const USER_CARD_BG: Color = Color::Rgb(55, 65, 81);
pub(super) const TOOL_RESULT: Color = Color::Rgb(129, 140, 156);
pub(super) const ERROR: Color = Color::Rgb(248, 113, 113);
pub(super) const ERROR_DIM: Color = Color::Rgb(252, 165, 165);

/// Diff palette shared by the standalone diff viewer and inline `diff` code
/// blocks in the transcript.
pub(super) const DIFF_ADD_FG: Color = Color::Rgb(134, 239, 172);
pub(super) const DIFF_ADD_BG: Color = Color::Rgb(18, 48, 28);
pub(super) const DIFF_ADD_HIGHLIGHT: Color = Color::Rgb(36, 96, 48);
pub(super) const DIFF_DELETE_FG: Color = Color::Rgb(252, 165, 165);
pub(super) const DIFF_DELETE_BG: Color = Color::Rgb(56, 20, 24);
pub(super) const DIFF_DELETE_HIGHLIGHT: Color = Color::Rgb(112, 32, 40);
pub(super) const DIFF_HUNK_FG: Color = Color::Rgb(56, 189, 248);

/// Border color for widgets that can take keyboard focus.
pub(super) fn border_style(focused: bool) -> Style {
    Style::default().fg(if focused { ACCENT } else { BORDER_MUTED })
}

/// Panel chrome (transcript, chat list, input editor): rounded corners,
/// border color expresses focus state.
pub(super) fn panel_block(focused: bool) -> Block<'static> {
    base_block()
        .title_style(title_style())
        .border_style(border_style(focused))
}

/// Modal/popup chrome: rounded corners, accent border, styled titles.
pub(super) fn popup_block() -> Block<'static> {
    base_block()
        .title_style(title_style())
        .border_style(Style::default().fg(ACCENT))
}

fn base_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
}

fn title_style() -> Style {
    Style::default()
        .fg(ACCENT_STRONG)
        .add_modifier(Modifier::BOLD)
}

/// Muted separator between status-line segments.
pub(super) fn status_separator_span() -> Span<'static> {
    Span::styled(" │ ", Style::default().fg(BORDER))
}
