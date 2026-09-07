use ratatui::layout::Rect;

/// ASCII glyphs stay one cell wide on CJK terminals; block and triangle glyphs do not.
pub(super) const THUMB_SYMBOL: &str = "#";
pub(super) const TRACK_SYMBOL: &str = "|";
pub(super) const BEGIN_SYMBOL: &str = "^";
pub(super) const END_SYMBOL: &str = "v";

/// Which part of the vertical-right transcript scrollbar a pointer is on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ScrollbarHit {
    Begin,
    Track,
    End,
}

/// Transcript inner panes: message body and a reserved one-column scrollbar strip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TranscriptSplit {
    pub content: Rect,
    pub scrollbar: Rect,
}

/// Splits the bordered transcript `area` into content and an inner-right scrollbar column.
pub(super) fn split_transcript_inner(area: Rect) -> TranscriptSplit {
    let inner = bordered_inner(area);
    if inner.width == 0 || inner.height == 0 {
        return TranscriptSplit {
            content: inner,
            scrollbar: Rect::new(0, 0, 0, 0),
        };
    }
    TranscriptSplit {
        content: Rect {
            width: inner.width.saturating_sub(1),
            ..inner
        },
        scrollbar: Rect {
            x: inner.x.saturating_add(inner.width.saturating_sub(1)),
            y: inner.y,
            width: 1,
            height: inner.height,
        },
    }
}

/// Returns the one-column area occupied by the inner transcript scrollbar.
pub(super) fn scrollbar_hit_area(area: Rect) -> Rect {
    split_transcript_inner(area).scrollbar
}

/// Returns whether the pointer sits on the transcript scrollbar column.
pub(super) fn pointer_hits_scrollbar(column: u16, row: u16, area: Rect) -> bool {
    let hit = scrollbar_hit_area(area);
    hit.width > 0 && column == hit.x && row >= hit.y && row < hit.y.saturating_add(hit.height)
}

/// Returns the scrollbar part under `row` when the pointer is on the bar.
pub(super) fn scrollbar_hit_part(row: u16, area: Rect) -> ScrollbarHit {
    let hit = scrollbar_hit_area(area);
    let last = hit.y.saturating_add(hit.height.saturating_sub(1));
    if row == hit.y {
        ScrollbarHit::Begin
    } else if row == last {
        ScrollbarHit::End
    } else {
        ScrollbarHit::Track
    }
}

/// Maps a pointer row onto `[0, max_scroll]` along the track between the arrow glyphs.
pub(super) fn scroll_position_for_pointer(row: u16, area: Rect, max_scroll: u16) -> u16 {
    let hit = scrollbar_hit_area(area);
    let track_top = hit.y.saturating_add(1);
    let track_height = hit.height.saturating_sub(2);
    if track_height <= 1 {
        return 0;
    }
    let track_bottom = track_top.saturating_add(track_height.saturating_sub(1));
    let clamped = row.clamp(track_top, track_bottom);
    let offset = u32::from(clamped.saturating_sub(track_top));
    let span = u32::from(track_height.saturating_sub(1));
    ((u64::from(offset) * u64::from(max_scroll)) / u64::from(span)) as u16
}

/// Returns the rectangle inside a `Borders::ALL` block.
fn bordered_inner(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        pointer_hits_scrollbar, scroll_position_for_pointer, scrollbar_hit_area,
        scrollbar_hit_part, split_transcript_inner, ScrollbarHit,
    };
    use ratatui::layout::Rect;

    #[test]
    fn scrollbar_uses_inner_right_column() {
        let area = Rect::new(10, 2, 20, 12);
        let split = split_transcript_inner(area);
        assert_eq!(split.content, Rect::new(11, 3, 17, 10));
        assert_eq!(split.scrollbar, Rect::new(28, 3, 1, 10));
        assert_eq!(scrollbar_hit_area(area), split.scrollbar);
        assert!(pointer_hits_scrollbar(28, 5, area));
        assert!(!pointer_hits_scrollbar(29, 5, area));
        assert!(!pointer_hits_scrollbar(28, 2, area));
    }

    #[test]
    fn arrows_and_track_are_distinct_hits() {
        let area = Rect::new(0, 0, 8, 10);
        assert_eq!(scrollbar_hit_part(1, area), ScrollbarHit::Begin);
        assert_eq!(scrollbar_hit_part(8, area), ScrollbarHit::End);
        assert_eq!(scrollbar_hit_part(4, area), ScrollbarHit::Track);
    }

    #[test]
    fn pointer_row_maps_to_scroll_ends() {
        let area = Rect::new(0, 0, 8, 12);
        assert_eq!(scroll_position_for_pointer(2, area, 100), 0);
        assert_eq!(scroll_position_for_pointer(9, area, 100), 100);
    }
}
