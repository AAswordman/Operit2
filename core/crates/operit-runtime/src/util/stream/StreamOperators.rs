use crate::util::stream::Stream::{Stream, VecStream};
use crate::util::stream::StreamGroup::StreamGroup;
use crate::util::stream::plugins::StreamPlugin::{PluginState, StreamPlugin};

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

pub fn concat_with<S>(left: S, right: S) -> VecStream<S::Item>
where
    S: Stream,
{
    merge(left, right)
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

pub fn split_chars_by(
    mut source: impl Stream<Item = char>,
    mut plugins: Vec<Box<dyn StreamPlugin>>,
) -> VecStream<StreamGroup<Option<String>>> {
    for plugin in &mut plugins {
        plugin.init_plugin();
    }

    let mut groups = Vec::new();
    let mut default_buffer = String::new();
    let mut plugin_buffer = String::new();
    let mut active_plugin: Option<usize> = None;
    let mut at_start_of_line = true;

    source.collect(&mut |ch| {
        let current_at_start = at_start_of_line;
        at_start_of_line = ch == '\n';

        if let Some(index) = active_plugin {
            let should_emit = plugins[index].process_char(ch, current_at_start);
            if should_emit {
                plugin_buffer.push(ch);
            }
            if plugins[index].state() != PluginState::Processing
                && plugins[index].state() != PluginState::WaitFor
            {
                let tag = Some(plugins[index].name().to_string());
                groups.push(StreamGroup::new(
                    tag,
                    Box::new(VecStream::new(vec![plugin_buffer.clone()])),
                ));
                plugin_buffer.clear();
                active_plugin = None;
            }
            return;
        }

        let mut matched = None;
        for (index, plugin) in plugins.iter_mut().enumerate() {
            plugin.process_char(ch, current_at_start);
            if plugin.state() == PluginState::Processing {
                matched = Some(index);
                break;
            }
        }

        if let Some(index) = matched {
            if !default_buffer.is_empty() {
                groups.push(StreamGroup::new(
                    None,
                    Box::new(VecStream::new(vec![std::mem::take(&mut default_buffer)])),
                ));
            }
            active_plugin = Some(index);
            plugin_buffer.push(ch);
            for (other_index, plugin) in plugins.iter_mut().enumerate() {
                if other_index != index {
                    plugin.reset();
                }
            }
        } else if plugins.iter().all(|plugin| plugin.state() != PluginState::Trying) {
            default_buffer.push(ch);
            for plugin in &mut plugins {
                plugin.reset();
            }
        } else {
            default_buffer.push(ch);
        }
    });

    if !plugin_buffer.is_empty() {
        let tag = active_plugin.map(|index| plugins[index].name().to_string());
        groups.push(StreamGroup::new(tag, Box::new(VecStream::new(vec![plugin_buffer]))));
    }
    if !default_buffer.is_empty() {
        groups.push(StreamGroup::new(None, Box::new(VecStream::new(vec![default_buffer]))));
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
