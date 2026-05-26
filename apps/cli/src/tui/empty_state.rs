use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use std::sync::OnceLock;
use std::time::Instant;

use super::theme;

pub(super) fn render_blue_cat_lines(content_width: usize) -> Vec<Line<'static>> {
    let cat_style = Style::default()
        .fg(theme::ACCENT_STRONG)
        .add_modifier(Modifier::BOLD);
    let brand_style = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::BOLD);
    let hint_style = Style::default().fg(theme::TEXT_SUBTLE);
    let elapsed_ms = empty_state_elapsed_ms();
    let cat_lines = animated_cat_lines(elapsed_ms);

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    for line in cat_lines {
        lines.push(centered_styled_line(*line, content_width, cat_style));
    }
    for line in operit_logo_lines(content_width) {
        lines.push(centered_styled_line(*line, content_width, brand_style));
    }
    lines.push(Line::from(""));
    lines.push(centered_styled_line(
        "Type a message to start.",
        content_width,
        hint_style,
    ));
    lines
}

fn empty_state_elapsed_ms() -> u128 {
    static STARTED_AT: OnceLock<Instant> = OnceLock::new();
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis()
}

fn animated_cat_lines(elapsed_ms: u128) -> &'static [&'static str] {
    let cycle_ms = 20_000;
    let idle_ms = 15_000;
    let cycle_position = (elapsed_ms % cycle_ms) as u64;
    if cycle_position < idle_ms {
        return idle_cat_frame(cycle_position);
    }
    let action_elapsed = cycle_position - idle_ms;
    match pseudo_random_action(elapsed_ms / cycle_ms as u128) {
        0 => turn_cat_frame(action_elapsed),
        1 => wave_cat_frame(action_elapsed),
        2 => tail_cat_frame(action_elapsed),
        3 => stretch_cat_frame(action_elapsed),
        4 => nap_cat_frame(action_elapsed),
        _ => look_cat_frame(action_elapsed),
    }
}

fn pseudo_random_action(slice: u128) -> usize {
    let mut value = slice as u64;
    value ^= 0x9E37_79B9_7F4A_7C15;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    (value as usize) % 6
}

fn idle_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_loop(elapsed_ms, 240, 64) {
        0 => &[r"   /\_/\   ", r"  ( -.- )  ", r"   > ^ <   "],
        1 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        18 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <~  "],
        19 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <~~ "],
        20 => &[r"   /\_/\   ", r"  ( o.o )  ", r"  ~> ^ <   "],
        21 => &[r"   /\_/\   ", r"  ( o.o )  ", r" ~~> ^ <   "],
        32 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        33 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <    "],
        34 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <~   "],
        35 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        36 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        50 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        51 => &[r"    |\_/\  ", r"   ( >.o ) ", r"    > ^ <  "],
        52 => &[r"    |\_/\  ", r"   ( >.o ) ", r"   ~> ^ <  "],
        53 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        54 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        _ => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
    }
}

fn turn_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_index(elapsed_ms, 150, 28) {
        0 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        1 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        2 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <    "],
        3 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <~   "],
        4 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <~~  "],
        5 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <~   "],
        6 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        7 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        8 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        9 => &[r"    |\_/\  ", r"   ( >.o ) ", r"    > ^ <  "],
        10 => &[r"    |\_/\  ", r"   ( >.o ) ", r"   ~> ^ <  "],
        11 => &[r"    |\_/\  ", r"   ( >.o ) ", r"  ~~> ^ <  "],
        12 => &[r"    |\_/\  ", r"   ( >.o ) ", r"   ~> ^ <  "],
        13 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        14 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        15 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        16 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <    "],
        17 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        18 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        19 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        20 => &[r"    |\_/\  ", r"   ( >.o ) ", r"    > ^ <  "],
        21 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        _ => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
    }
}

fn wave_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_index(elapsed_ms, 135, 30) {
        0 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        1 => &[r"   /\_/\   ", r"  ( o.o )  ", r"  / > ^ <  "],
        2 => &[r"   /\_/\  /", r"  ( o.o )/ ", r"   > ^ <   "],
        3 => &[r"  /\_/\   /", r" ( o.o ) / ", r"  > ^ <    "],
        4 => &[r"  /\_/\  / ", r" ( o.o )/  ", r"  > ^ <    "],
        5 => &[r"   /\_/\  /", r"  ( o.o )/ ", r"   > ^ <   "],
        6 => &[r"   /\_/\   ", r"  ( o.o )  ", r"  / > ^ <  "],
        7 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        8 => &[r"   /\_/\   ", r"  ( ^.^ )  ", r"  / > ^ <  "],
        9 => &[r"   /\_/\  /", r"  ( ^.^ )/ ", r"   > ^ <   "],
        10 => &[r"  /\_/\   /", r" ( ^.^ ) / ", r"  > ^ <    "],
        11 => &[r"   /\_/\  /", r"  ( ^.^ )/ ", r"   > ^ <   "],
        12 => &[r"   /\_/\   ", r"  ( ^.^ )  ", r"  / > ^ <  "],
        13 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        _ => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
    }
}

fn tail_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_index(elapsed_ms, 120, 34) {
        0 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <~  "],
        1 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <~~ "],
        2 => &[r"   /\_/\   ", r"  ( o.o )  ", r"    > ^ <~ "],
        3 => &[r"   /\_/\   ", r"  ( o.o )  ", r"    > ^ <  "],
        4 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        5 => &[r"   /\_/\   ", r"  ( o.o )  ", r"  ~> ^ <   "],
        6 => &[r"   /\_/\   ", r"  ( o.o )  ", r" ~~> ^ <   "],
        7 => &[r"   /\_/\   ", r"  ( o.o )  ", r" ~> ^ <    "],
        8 => &[r"   /\_/\   ", r"  ( o.o )  ", r"  > ^ <    "],
        9 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        10 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <~   "],
        11 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <~~  "],
        12 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        _ => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
    }
}

fn stretch_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_index(elapsed_ms, 170, 28) {
        0 => &[r"   /\_/\   ", r"  ( =.= )  ", r"   > ^ <   "],
        1 => &[r"   /\_/\   ", r"  ( -.- )  ", r"  /|___|\  "],
        2 => &[
            r"   /\_/\   ",
            r"  ( -o- )  ",
            r"  /|___|\  ",
            r"   /   \   ",
        ],
        3 => &[
            r"   /\_/\   ",
            r"  ( -o- )  ",
            r" _/|___|\_ ",
            r"   /   \   ",
        ],
        4 => &[
            r"   /\_/\   ",
            r"  ( -.- )  ",
            r" _/|___|\_ ",
            r"   /   \   ",
        ],
        5 => &[
            r"   /\_/\   ",
            r"  ( -.- )  ",
            r"__/|___|\__",
            r"   /   \   ",
        ],
        6 => &[
            r"   /\_/\   ",
            r"  ( -o- )  ",
            r" _/|___|\_ ",
            r"  _/   \_  ",
        ],
        7 => &[
            r"   /\_/\   ",
            r"  ( -.- )  ",
            r"  /|___|\  ",
            r"   /   \   ",
        ],
        8 => &[r"   /\_/\   ", r"  ( =.= )  ", r"  /|___|\  "],
        9 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        _ => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
    }
}

fn nap_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_index(elapsed_ms, 210, 24) {
        0 => &[r"   /\_/\   ", r"  ( -.- )  ", r"   > ^ <   "],
        1 => &[r"   /\_/\   ", r"  ( -.- ) z", r"   > ^ <   "],
        2 => &[r"   /\_/\   ", r"  ( -.- ) Z", r"   > ^ <   "],
        3 => &[r"   /\_/\   ", r"  ( -.- ) Z", r"   > ^ < z "],
        4 => &[r"   /\_/\   ", r"  ( -.- ) Z", r"   > ^ < Z "],
        5 => &[r"   /\_/\   ", r"  ( -.- ) z", r"   > ^ < Z "],
        6 => &[r"   /\_/\   ", r"  ( -.- )  ", r"   > ^ < z "],
        7 => &[r"   /\_/\   ", r"  ( -.- )  ", r"   > ^ <   "],
        8 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        9 => &[r"   /\_/\   ", r"  ( -.- )  ", r"   > ^ <   "],
        _ => &[r"   /\_/\   ", r"  ( -.- )  ", r"   > ^ <   "],
    }
}

fn look_cat_frame(elapsed_ms: u64) -> &'static [&'static str] {
    match frame_index(elapsed_ms, 150, 28) {
        0 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        1 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        2 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <    "],
        3 => &[r"  /\_/|    ", r" ( o.< )   ", r"  > ^ <~   "],
        4 => &[r"  /\_/\    ", r" ( o.o )<  ", r"  > ^ <    "],
        5 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        6 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        7 => &[r"    |\_/\  ", r"   ( >.o ) ", r"    > ^ <  "],
        8 => &[r"    |\_/\  ", r"   ( >.o ) ", r"   ~> ^ <  "],
        9 => &[r"    /\_/\  ", r"  >( o.o ) ", r"    > ^ <  "],
        10 => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
        _ => &[r"   /\_/\   ", r"  ( o.o )  ", r"   > ^ <   "],
    }
}

fn frame_index(elapsed_ms: u64, frame_ms: u64, frame_count: usize) -> usize {
    ((elapsed_ms / frame_ms) as usize).min(frame_count.saturating_sub(1))
}

fn frame_loop(elapsed_ms: u64, frame_ms: u64, frame_count: usize) -> usize {
    ((elapsed_ms / frame_ms) as usize) % frame_count.max(1)
}

fn operit_logo_lines(content_width: usize) -> &'static [&'static str] {
    if content_width < 38 {
        return &["OPERIT"];
    }
    &[
        r"  ___                  _ _   ",
        r" / _ \ _ __   ___ _ __(_) |_ ",
        r"| | | | '_ \ / _ \ '__| | __|",
        r"| |_| | |_) |  __/ |  | | |_ ",
        r" \___/| .__/ \___|_|  |_|\__|",
        r"      |_|                    ",
    ]
}

fn centered_styled_line(content: &str, content_width: usize, style: Style) -> Line<'static> {
    let padding = content_width.saturating_sub(content.chars().count()) / 2;
    Line::from(vec![
        Span::raw(" ".repeat(padding)),
        Span::styled(content.to_string(), style),
    ])
}
