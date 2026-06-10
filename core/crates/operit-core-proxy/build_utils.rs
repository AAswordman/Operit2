use std::path::Path;

pub(crate) fn full_type_for_source(
    runtime_src: &Path,
    source_path: &Path,
    type_name: &str,
) -> String {
    format!(
        "{}::{type_name}",
        module_path_for_source(runtime_src, source_path)
    )
}

pub(crate) fn module_path_for_source(runtime_src: &Path, source_path: &Path) -> String {
    let relative = source_path
        .strip_prefix(runtime_src)
        .expect("source path must be inside runtime src");
    let mut module_path = Vec::from(["operit_runtime".to_string()]);
    for component in relative.with_extension("").components() {
        module_path.push(component.as_os_str().to_string_lossy().to_string());
    }
    module_path.join("::")
}

pub(crate) fn dispatch_name_from_schema_key(schema_key: &str) -> String {
    identifier_words(schema_key)
        .into_iter()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("_")
}

pub(crate) fn identifier_words(name: &str) -> Vec<String> {
    let mut words = Vec::new();
    for segment in name.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if segment.is_empty() {
            continue;
        }
        words.extend(split_identifier_segment(segment));
    }
    collapse_duplicate_words(merge_acronym_words(words))
}

fn split_identifier_segment(segment: &str) -> Vec<String> {
    let chars = segment.chars().collect::<Vec<_>>();
    let mut words = Vec::new();
    let mut start = 0usize;
    for index in 1..chars.len() {
        let previous = chars[index - 1];
        let current = chars[index];
        let next = chars.get(index + 1).copied();
        let lower_to_upper = previous.is_ascii_lowercase() && current.is_ascii_uppercase();
        let acronym_to_word = previous.is_ascii_uppercase()
            && current.is_ascii_uppercase()
            && next.map(|ch| ch.is_ascii_lowercase()).unwrap_or(false);
        let digit_boundary = previous.is_ascii_digit() != current.is_ascii_digit();
        if lower_to_upper || acronym_to_word || digit_boundary {
            words.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    words.push(chars[start..].iter().collect::<String>());
    words
}

fn merge_acronym_words(words: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < words.len() {
        if index + 1 < words.len()
            && words[index].len() == 1
            && words[index].chars().all(|ch| ch.is_ascii_lowercase())
            && words[index + 1].chars().all(|ch| ch.is_ascii_uppercase())
        {
            out.push(format!(
                "{}{}",
                words[index].to_ascii_uppercase(),
                words[index + 1]
            ));
            index += 2;
        } else {
            out.push(words[index].clone());
            index += 1;
        }
    }
    out
}

fn collapse_duplicate_words(words: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for word in words {
        let duplicate = out
            .last()
            .map(|previous: &String| previous.eq_ignore_ascii_case(&word))
            .unwrap_or(false);
        if !duplicate {
            out.push(word);
        }
    }
    out
}

pub(crate) fn lower_first(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.push(first.to_ascii_lowercase());
    out.extend(chars);
    out
}

pub(crate) fn parent_module_path(full_type: &str) -> &str {
    full_type
        .rsplit_once("::")
        .map(|(module, _)| module)
        .expect("object full_type must include module path")
}
