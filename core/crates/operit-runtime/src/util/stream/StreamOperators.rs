use crate::util::stream::Stream::{Stream, VecStream};
use crate::util::stream::StreamGroup::StreamGroup;
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};
use std::collections::VecDeque;
use std::thread;
use std::time::{Duration, Instant};

pub fn map<S, R>(mut source: S, mut transform: impl FnMut(S::Item) -> R) -> VecStream<R>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| values.push(transform(value)));
    VecStream::new(values)
}

pub fn filter<S>(mut source: S, mut predicate: impl FnMut(&S::Item) -> bool) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| {
        if predicate(&value) {
            values.push(value);
        }
    });
    VecStream::new(values)
}

pub fn take<S>(mut source: S, count: usize) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| {
        if values.len() < count {
            values.push(value);
        }
    });
    VecStream::new(values)
}

pub fn drop<S>(mut source: S, count: usize) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut skipped = 0;
    let mut values = Vec::new();
    source.collect(&mut |value| {
        if skipped < count {
            skipped += 1;
        } else {
            values.push(value);
        }
    });
    VecStream::new(values)
}

pub fn flat_map<S, R, Inner>(
    mut source: S,
    mut transform: impl FnMut(S::Item) -> Inner,
) -> VecStream<R>
where
    S: Stream,
    Inner: Stream<Item = R>,
{
    let mut values = Vec::new();
    source.collect(&mut |value| {
        let mut inner = transform(value);
        inner.collect(&mut |item| values.push(item));
    });
    VecStream::new(values)
}

pub fn on_each<S>(mut source: S, mut action: impl FnMut(&S::Item)) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| {
        action(&value);
        values.push(value);
    });
    VecStream::new(values)
}

pub fn merge<S>(mut left: S, mut right: S) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    left.collect(&mut |value| values.push(value));
    right.collect(&mut |value| values.push(value));
    VecStream::new(values)
}

pub fn combine<L, R, O>(
    mut left: L,
    mut right: R,
    mut transform: impl FnMut(L::Item, R::Item) -> O,
) -> VecStream<O>
where
    L: Stream,
    R: Stream,
    L::Item: Clone,
    R::Item: Clone,
{
    let mut left_values = Vec::new();
    let mut right_values = Vec::new();
    left.collect(&mut |value| left_values.push(value));
    right.collect(&mut |value| right_values.push(value));

    let mut values = Vec::new();
    let mut latest_left = None::<L::Item>;
    let mut latest_right = None::<R::Item>;
    for value in left_values {
        latest_left = Some(value);
        if let (Some(left_value), Some(right_value)) = (latest_left.clone(), latest_right.clone()) {
            values.push(transform(left_value, right_value));
        }
    }
    for value in right_values {
        latest_right = Some(value);
        if let (Some(left_value), Some(right_value)) = (latest_left.clone(), latest_right.clone()) {
            values.push(transform(left_value, right_value));
        }
    }
    VecStream::new(values)
}

pub fn concat_with<S>(left: S, right: S) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    let mut left = left;
    let mut right = right;
    left.collect(&mut |value| values.push(value));
    right.collect(&mut |value| values.push(value));
    VecStream::new(values)
}

pub fn catch<S>(mut source: S, _action: impl FnMut(String)) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| values.push(value));
    VecStream::new(values)
}

pub fn finally<S>(mut source: S, mut action: impl FnMut()) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| values.push(value));
    action();
    VecStream::new(values)
}

pub fn throttle_first<S>(mut source: S, window_duration: Duration) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    let mut last_emit_time: Option<Instant> = None;
    source.collect(&mut |value| {
        let current_time = Instant::now();
        if last_emit_time
            .map(|last| current_time.duration_since(last) >= window_duration)
            .unwrap_or(true)
        {
            last_emit_time = Some(current_time);
            values.push(value);
        }
    });
    VecStream::new(values)
}

pub fn distinct_until_changed<S>(mut source: S) -> VecStream<S::Item>
where
    S: Stream,
    S::Item: PartialEq + Clone,
{
    let mut last_value: Option<S::Item> = None;
    let mut values = Vec::new();
    source.collect(&mut |value| {
        if last_value.as_ref() != Some(&value) {
            last_value = Some(value.clone());
            values.push(value);
        }
    });
    VecStream::new(values)
}

pub fn chunked<S>(mut source: S, size: usize) -> VecStream<Vec<S::Item>>
where
    S: Stream,
{
    assert!(size > 0, "Size must be positive.");
    let mut values = Vec::new();
    let mut current = Vec::new();
    source.collect(&mut |value| {
        current.push(value);
        if current.len() == size {
            values.push(std::mem::take(&mut current));
        }
    });
    if !current.is_empty() {
        values.push(current);
    }
    VecStream::new(values)
}

pub fn delay<S>(mut source: S, duration: Duration) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| {
        thread::sleep(duration);
        values.push(value);
    });
    VecStream::new(values)
}

pub fn debounce<S>(mut source: S, _timeout: Duration) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut last_value = None;
    source.collect(&mut |value| {
        last_value = Some(value);
    });
    VecStream::new(last_value.into_iter().collect::<Vec<_>>())
}

pub fn sample<S>(mut source: S, _period: Duration) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| values.push(value));
    VecStream::new(values)
}

pub fn throttle_last<S>(mut source: S, window_duration: Duration) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    let mut last_emit_time: Option<Instant> = None;
    let mut pending_value = None;
    source.collect(&mut |value| {
        let current_time = Instant::now();
        pending_value = Some(value);
        if last_emit_time
            .map(|last| current_time.duration_since(last) >= window_duration)
            .unwrap_or(true)
        {
            last_emit_time = Some(current_time);
            if let Some(value) = pending_value.take() {
                values.push(value);
            }
        }
    });
    VecStream::new(values)
}

pub fn fixed_rate<S>(mut source: S, period: Duration) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    let mut next_emit_time = None::<Instant>;
    source.collect(&mut |value| {
        let current_time = Instant::now();
        match next_emit_time {
            None => {
                next_emit_time = Some(current_time + period);
                values.push(value);
            }
            Some(next_time) if current_time >= next_time => {
                next_emit_time = Some(current_time + period);
                values.push(value);
            }
            Some(next_time) => {
                thread::sleep(next_time.saturating_duration_since(current_time));
                next_emit_time = Some(Instant::now() + period);
                values.push(value);
            }
        }
    });
    VecStream::new(values)
}

pub fn timeout_trigger<S>(mut source: S, _timeout_duration: Duration, timeout_value: Option<S::Item>) -> VecStream<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    source.collect(&mut |value| values.push(value));
    if values.is_empty() {
        if let Some(value) = timeout_value {
            values.push(value);
        }
    }
    VecStream::new(values)
}

pub fn split_chars_by(
    mut source: impl Stream<Item = char>,
    mut plugins: Vec<Box<dyn StreamPlugin>>,
) -> VecStream<StreamGroup<Option<String>>> {
    for plugin in &mut plugins {
        plugin.init_plugin();
    }

    let mut upstream_chars = VecDeque::new();
    source.collect(&mut |ch| upstream_chars.push_back(ch));

    let mut groups: Vec<StreamGroup<Option<String>>> = Vec::new();
    let mut default_text_buffer = String::new();
    let mut active_plugin: Option<usize> = None;
    let mut active_plugin_buffer = String::new();
    let mut evaluation_buffer: Vec<char> = Vec::new();
    let mut evaluation_should_emit: Vec<Vec<bool>> = Vec::new();
    let mut at_start_of_line = true;

    while let Some(ch) = upstream_chars.pop_front() {
        let current_at_start = at_start_of_line;
        at_start_of_line = ch == '\n';

        if let Some(index) = active_plugin {
            let should_emit = plugins[index].process_char(ch, current_at_start);
            if should_emit {
                active_plugin_buffer.push(ch);
            }

            if plugins[index].state() != PluginState::Processing {
                if plugins[index].state() == PluginState::WaitFor {
                    let mut waitfor_buffer = Vec::new();
                    if should_emit {
                        waitfor_buffer.push(ch);
                    }

                    if let Some(next_ch) = upstream_chars.pop_front() {
                        let next_at_start = ch == '\n';
                        let next_should_emit = plugins[index].process_char(next_ch, next_at_start);
                        if plugins[index].state() == PluginState::Processing {
                            if next_should_emit {
                                active_plugin_buffer.push(next_ch);
                            }
                            at_start_of_line = next_ch == '\n';
                            continue;
                        }

                        upstream_chars.push_front(next_ch);
                        for buffered in waitfor_buffer.into_iter().rev() {
                            upstream_chars.push_front(buffered);
                        }
                    } else {
                        for buffered in waitfor_buffer.into_iter().rev() {
                            upstream_chars.push_front(buffered);
                        }
                    }
                }

                let tag = Some(plugins[index].name().to_string());
                groups.push(StreamGroup::new(
                    tag,
                    Box::new(VecStream::new(vec![std::mem::take(&mut active_plugin_buffer)])),
                ));
                active_plugin = None;
            }
            continue;
        }

        evaluation_buffer.push(ch);
        let mut should_emit_map = Vec::with_capacity(plugins.len());
        for (index, plugin) in plugins.iter_mut().enumerate() {
            let should_emit = plugin.process_char(ch, current_at_start);
            let _ = index;
            should_emit_map.push(should_emit);
        }
        evaluation_should_emit.push(should_emit_map);

        let successful_plugin = plugins
            .iter()
            .position(|plugin| plugin.state() == PluginState::Processing);

        if let Some(index) = successful_plugin {
            if !default_text_buffer.is_empty() {
                groups.push(StreamGroup::new(
                    None,
                    Box::new(VecStream::new(vec![std::mem::take(&mut default_text_buffer)])),
                ));
            }

            active_plugin = Some(index);
            for (buffer_index, buffered_ch) in evaluation_buffer.iter().copied().enumerate() {
                if evaluation_should_emit
                    .get(buffer_index)
                    .and_then(|map| map.get(index))
                    .copied()
                    .unwrap_or(false)
                {
                    active_plugin_buffer.push(buffered_ch);
                }
            }
            evaluation_buffer.clear();
            evaluation_should_emit.clear();

            for (other_index, plugin) in plugins.iter_mut().enumerate() {
                if other_index != index {
                    plugin.reset();
                }
            }
        } else if plugins.iter().all(|plugin| plugin.state() != PluginState::Trying) {
            for buffered_ch in evaluation_buffer.drain(..) {
                default_text_buffer.push(buffered_ch);
            }
            evaluation_should_emit.clear();
            for plugin in &mut plugins {
                plugin.reset();
            }
        }
    }

    if !active_plugin_buffer.is_empty() {
        let tag = active_plugin.map(|index| plugins[index].name().to_string());
        groups.push(StreamGroup::new(tag, Box::new(VecStream::new(vec![active_plugin_buffer]))));
    }
    if !evaluation_buffer.is_empty() {
        for buffered_ch in evaluation_buffer {
            default_text_buffer.push(buffered_ch);
        }
    }
    if !default_text_buffer.is_empty() {
        groups.push(StreamGroup::new(None, Box::new(VecStream::new(vec![default_text_buffer]))));
    }

    VecStream::new(groups)
}

pub fn split_strings_by(
    mut source: impl Stream<Item = String>,
    plugins: Vec<Box<dyn StreamPlugin>>,
) -> VecStream<StreamGroup<Option<String>>> {
    let mut chars = Vec::new();
    source.collect(&mut |chunk| chars.extend(chunk.chars()));
    split_chars_by(VecStream::new(chars), plugins)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TimeoutException {
    pub message: String,
}
