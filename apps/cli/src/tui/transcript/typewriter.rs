use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const KT_RENDER_INTERVAL: Duration = Duration::from_millis(200);
const MIN_CHAR_INTERVAL: Duration = Duration::from_millis(8);
const MAX_CHAR_INTERVAL: Duration = Duration::from_millis(32);
const MAX_LAG_CHARS: usize = 240;

#[derive(Clone, Debug)]
struct TypewriterEntry {
    visible_chars: usize,
    last_tick: Instant,
    last_total_chars: usize,
    char_interval: Duration,
}

#[derive(Clone, Debug, Default)]
pub(super) struct TypewriterState {
    entries: HashMap<i64, TypewriterEntry>,
}

#[derive(Clone, Debug)]
pub(super) struct TypewriterFrame {
    pub(super) content: String,
    pub(super) pending_char: Option<char>,
}

impl TypewriterState {
    pub(super) fn frame(
        &mut self,
        message_timestamp: i64,
        full_content: &str,
        is_streaming: bool,
    ) -> TypewriterFrame {
        if !is_streaming {
            self.entries.remove(&message_timestamp);
            return TypewriterFrame {
                content: full_content.to_string(),
                pending_char: None,
            };
        }

        let total_chars = full_content.chars().count();
        if total_chars == 0 {
            let now = Instant::now();
            self.entries
                .entry(message_timestamp)
                .or_insert(TypewriterEntry {
                    visible_chars: 0,
                    last_tick: now,
                    last_total_chars: 0,
                    char_interval: MAX_CHAR_INTERVAL,
                });
            return TypewriterFrame {
                content: String::new(),
                pending_char: None,
            };
        }

        let now = Instant::now();
        let entry = self
            .entries
            .entry(message_timestamp)
            .or_insert(TypewriterEntry {
                visible_chars: 0,
                last_tick: now,
                last_total_chars: total_chars,
                char_interval: interval_for_chunk(total_chars),
            });

        if total_chars < entry.visible_chars {
            entry.visible_chars = total_chars;
        }
        if total_chars != entry.last_total_chars {
            entry.char_interval =
                interval_for_chunk(total_chars.saturating_sub(entry.last_total_chars));
            entry.last_total_chars = total_chars;
        }

        let elapsed = now.saturating_duration_since(entry.last_tick);
        let char_interval_ms = entry.char_interval.as_millis().max(1);
        let steps = (elapsed.as_millis() / char_interval_ms) as usize;
        if steps > 0 {
            entry.visible_chars = entry.visible_chars.saturating_add(steps).min(total_chars);
            entry.last_tick = now;
        }

        if total_chars.saturating_sub(entry.visible_chars) > MAX_LAG_CHARS {
            entry.visible_chars = total_chars.saturating_sub(MAX_LAG_CHARS);
            entry.last_tick = now;
        }

        let content = take_chars(full_content, entry.visible_chars);
        let pending_char = full_content.chars().nth(entry.visible_chars);
        TypewriterFrame {
            content,
            pending_char,
        }
    }

    pub(super) fn retain_messages(&mut self, message_timestamps: &HashSet<i64>) {
        self.entries
            .retain(|timestamp, _| message_timestamps.contains(timestamp));
    }
}

fn take_chars(value: &str, char_count: usize) -> String {
    value.chars().take(char_count).collect()
}

fn interval_for_chunk(chunk_chars: usize) -> Duration {
    if chunk_chars == 0 {
        return MAX_CHAR_INTERVAL;
    }
    let interval_ms = (KT_RENDER_INTERVAL.as_millis() / chunk_chars as u128)
        .clamp(MIN_CHAR_INTERVAL.as_millis(), MAX_CHAR_INTERVAL.as_millis());
    Duration::from_millis(interval_ms as u64)
}
