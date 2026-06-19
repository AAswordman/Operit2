use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct LocaleFile {
    entries: Vec<(String, String)>,
    by_key: BTreeMap<String, String>,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let locale_dir = manifest_dir
        .join("src")
        .join("tui")
        .join("i18n")
        .join("locales");
    let en_path = locale_dir.join("en.kv");
    let zh_path = locale_dir.join("zh-CN.kv");

    println!("cargo:rerun-if-changed={}", en_path.display());
    println!("cargo:rerun-if-changed={}", zh_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    let english = read_locale_file(&en_path);
    let chinese = read_locale_file(&zh_path);
    validate_locale_files(&english, &chinese);

    let generated = generate_rust(&english, &chinese);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("tui_i18n_generated.rs"), generated)
        .expect("write generated TUI i18n source");
}

fn read_locale_file(path: &Path) -> LocaleFile {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read locale file {}: {error}", path.display()));
    let mut entries = Vec::new();
    let mut by_key = BTreeMap::new();

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (raw_key, raw_value) = trimmed
            .split_once('=')
            .unwrap_or_else(|| panic!("{}:{line_number}: expected key = value", path.display()));
        let key = raw_key.trim();
        validate_key(path, line_number, key);
        let value = decode_value(path, line_number, raw_value.trim());

        if by_key.insert(key.to_string(), value.clone()).is_some() {
            panic!("{}:{line_number}: duplicate key {key}", path.display());
        }
        entries.push((key.to_string(), value));
    }

    LocaleFile { entries, by_key }
}

fn validate_locale_files(english: &LocaleFile, chinese: &LocaleFile) {
    let english_keys = english.by_key.keys().cloned().collect::<BTreeSet<_>>();
    let chinese_keys = chinese.by_key.keys().cloned().collect::<BTreeSet<_>>();

    if english_keys != chinese_keys {
        let missing_in_chinese = english_keys
            .difference(&chinese_keys)
            .cloned()
            .collect::<Vec<_>>();
        let extra_in_chinese = chinese_keys
            .difference(&english_keys)
            .cloned()
            .collect::<Vec<_>>();
        panic!(
            "TUI locale key mismatch; missing in zh-CN: {:?}; extra in zh-CN: {:?}",
            missing_in_chinese, extra_in_chinese
        );
    }

    for key in english_keys {
        let english_placeholders = placeholders(
            english
                .by_key
                .get(&key)
                .expect("validated English locale key must exist"),
        );
        let chinese_placeholders = placeholders(
            chinese
                .by_key
                .get(&key)
                .expect("validated Chinese locale key must exist"),
        );
        if english_placeholders != chinese_placeholders {
            panic!(
                "TUI locale placeholder mismatch for {key}; en={:?}; zh-CN={:?}",
                english_placeholders, chinese_placeholders
            );
        }
    }
}

fn generate_rust(english: &LocaleFile, chinese: &LocaleFile) -> String {
    let mut output = String::new();
    output.push_str("#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]\n");
    output.push_str("pub(super) enum TuiTextKey {\n");
    for (key, _) in &english.entries {
        output.push_str("    ");
        output.push_str(&variant_name(key));
        output.push_str(",\n");
    }
    output.push_str("}\n\n");

    output.push_str(
        "pub(super) fn lookup_text(language: TuiLanguage, key: TuiTextKey) -> &'static str {\n",
    );
    output.push_str("    match language {\n");
    output.push_str("        TuiLanguage::English => match key {\n");
    push_locale_match_arms(&mut output, &english.entries);
    output.push_str("        },\n");
    output.push_str("        TuiLanguage::Chinese => match key {\n");
    let chinese_entries = english
        .entries
        .iter()
        .map(|(key, _)| {
            (
                key.clone(),
                chinese
                    .by_key
                    .get(key)
                    .expect("validated Chinese locale key must exist")
                    .clone(),
            )
        })
        .collect::<Vec<_>>();
    push_locale_match_arms(&mut output, &chinese_entries);
    output.push_str("        },\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");

    let english_help_lines = help_lines(english);
    let chinese_help_lines = help_lines(chinese);
    push_help_lines_const(&mut output, "HELP_LINES_EN", &english_help_lines);
    push_help_lines_const(&mut output, "HELP_LINES_ZH_CN", &chinese_help_lines);

    output.push_str("impl TuiText {\n");
    for (key, value) in &english.entries {
        // Hand-written special-case methods in i18n.rs override these
        match key.as_str() {
            "help_lines" | "context_usage_raw" | "context_usage" | "language_status"
            | "language_updated" => continue,
            _ => {}
        }
        let variant = variant_name(key);
        let method = key;
        let placeholders = placeholders(value);
        if placeholders.is_empty() {
            output.push_str("    pub(super) fn ");
            output.push_str(method);
            output.push_str("(self) -> &'static str {\n");
            output.push_str("        self.raw(TuiTextKey::");
            output.push_str(&variant);
            output.push_str(")\n");
            output.push_str("    }\n\n");
        } else {
            output.push_str("    pub(super) fn ");
            output.push_str(method);
            output.push('(');
            output.push_str("self");
            for placeholder in &placeholders {
                output.push_str(", ");
                output.push_str(placeholder);
                output.push_str(": impl ToString");
            }
            output.push_str(") -> String {\n");
            output.push_str("        self.render(TuiTextKey::");
            output.push_str(&variant);
            output.push_str(", &[\n");
            for placeholder in &placeholders {
                output.push_str("            (");
                output.push_str(&rust_string_literal(placeholder));
                output.push_str(", ");
                output.push_str(placeholder);
                output.push_str(".to_string()),\n");
            }
            output.push_str("        ])\n");
            output.push_str("    }\n\n");
        }
    }
    output.push_str("}\n");

    output
}

fn push_locale_match_arms(output: &mut String, entries: &[(String, String)]) {
    for (key, value) in entries {
        output.push_str("            TuiTextKey::");
        output.push_str(&variant_name(key));
        output.push_str(" => ");
        output.push_str(&rust_string_literal(value));
        output.push_str(",\n");
    }
}

fn push_help_lines_const(output: &mut String, name: &str, lines: &[String]) {
    output.push_str("pub(super) const ");
    output.push_str(name);
    output.push_str(": &[&str] = &[\n");
    for line in lines {
        output.push_str("    ");
        output.push_str(&rust_string_literal(line));
        output.push_str(",\n");
    }
    output.push_str("];\n\n");
}

fn help_lines(locale: &LocaleFile) -> Vec<String> {
    locale
        .by_key
        .get("help_lines")
        .expect("help_lines locale key must exist")
        .split('\n')
        .map(ToString::to_string)
        .collect()
}

fn validate_key(path: &Path, line_number: usize, key: &str) {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        panic!("{}:{line_number}: empty key", path.display());
    };
    if !first.is_ascii_lowercase() {
        panic!(
            "{}:{line_number}: key must start with a-z: {key}",
            path.display()
        );
    }
    for ch in chars {
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') {
            panic!(
                "{}:{line_number}: key must use a-z, 0-9, or underscore: {key}",
                path.display()
            );
        }
    }
}

fn decode_value(path: &Path, line_number: usize, value: &str) -> String {
    let mut decoded = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }
        let escaped = chars
            .next()
            .unwrap_or_else(|| panic!("{}:{line_number}: trailing escape", path.display()));
        match escaped {
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            '\\' => decoded.push('\\'),
            other => panic!(
                "{}:{line_number}: unsupported escape \\{other}",
                path.display()
            ),
        }
    }
    decoded
}

fn placeholders(value: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                let mut name = String::new();
                loop {
                    let next = chars
                        .next()
                        .unwrap_or_else(|| panic!("unterminated placeholder in {value:?}"));
                    if next == '}' {
                        break;
                    }
                    if next == '{' {
                        panic!("nested placeholder in {value:?}");
                    }
                    name.push(next);
                }
                validate_placeholder_name(value, &name);
                names.insert(name);
            }
            '}' => panic!("unmatched closing placeholder in {value:?}"),
            _ => {}
        }
    }
    names.into_iter().collect()
}

fn validate_placeholder_name(value: &str, name: &str) {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        panic!("empty placeholder in {value:?}");
    };
    if !(first.is_ascii_lowercase() || first == '_') {
        panic!("invalid placeholder name {name:?} in {value:?}");
    }
    for ch in chars {
        if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') {
            panic!("invalid placeholder name {name:?} in {value:?}");
        }
    }
}

fn variant_name(key: &str) -> String {
    let mut variant = String::new();
    for piece in key.split('_') {
        let mut chars = piece.chars();
        let first = chars
            .next()
            .unwrap_or_else(|| panic!("invalid empty key segment in {key}"));
        variant.push(first.to_ascii_uppercase());
        variant.extend(chars);
    }
    variant
}

fn rust_string_literal(value: &str) -> String {
    format!("{value:?}")
}
