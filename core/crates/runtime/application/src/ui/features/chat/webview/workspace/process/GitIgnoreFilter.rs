use regex::Regex;

const DEFAULT_EXCLUDES: [&str; 3] = ["backup", ".backup", ".operit"];

pub struct GitIgnoreFilter;

impl GitIgnoreFilter {
    pub fn defaultRules() -> Vec<String> {
        DEFAULT_EXCLUDES
            .iter()
            .map(|rule| rule.to_string())
            .collect()
    }

    pub fn normalizePath(path: &str) -> String {
        let mut value = path.trim().replace('\\', "/");
        if value.is_empty() {
            return "/".to_string();
        }
        while value.contains("//") {
            value = value.replace("//", "/");
        }
        value
    }

    pub fn parseRulesFromContent(content: &str) -> Vec<String> {
        let mut rules = Vec::<String>::new();
        for raw in content.lines() {
            let rule = raw.trim();
            if rule.is_empty() || rule.starts_with('#') {
                continue;
            }
            push_unique(&mut rules, rule.to_string());
            if let Some(without_slash) = rule.strip_suffix('/') {
                let trimmed = without_slash.trim();
                if !trimmed.is_empty() {
                    push_unique(&mut rules, trimmed.to_string());
                }
            }
        }
        rules
    }

    pub fn mergeWithDefaults(rules: Vec<String>) -> Vec<String> {
        let mut merged = Self::defaultRules();
        for rule in rules {
            push_unique(&mut merged, rule);
        }
        merged
    }

    pub fn buildRulesFromContent(content: &str) -> Vec<String> {
        Self::mergeWithDefaults(Self::parseRulesFromContent(content))
    }

    pub fn shouldIgnore(
        relativePath: &str,
        fileName: &str,
        isDirectory: bool,
        rules: &[String],
    ) -> bool {
        let rel = relativePath
            .trim_start_matches('/')
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();
        for rule in rules {
            if matchesRule(&rel, fileName, isDirectory, rule) {
                return true;
            }
        }
        false
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn matchesRule(relativePath: &str, fileName: &str, isDirectory: bool, rule: &str) -> bool {
    let mut pattern = rule.trim().to_string();
    if pattern.is_empty() || pattern.starts_with('!') {
        return false;
    }

    let dirOnly = pattern.ends_with('/');
    if dirOnly {
        if !isDirectory {
            return false;
        }
        pattern.truncate(pattern.len().saturating_sub(1));
    }

    let rootOnly = pattern.starts_with('/');
    if rootOnly {
        pattern = pattern.trim_start_matches('/').to_string();
    }

    if rootOnly {
        matchPattern(relativePath, &pattern)
    } else if pattern.contains('/') {
        matchPattern(relativePath, &pattern) || relativePath.ends_with(&format!("/{pattern}"))
    } else {
        fileName == pattern
            || matchPattern(fileName, &pattern)
            || relativePath
                .split('/')
                .any(|part| matchPattern(part, &pattern))
    }
}

fn matchPattern(text: &str, pattern: &str) -> bool {
    if let Some(subPattern) = pattern.strip_prefix("**/") {
        return text.ends_with(subPattern) || matchPattern(text, subPattern);
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return text.starts_with(prefix) || text == prefix;
    }
    if pattern.contains('*') || pattern.contains('?') {
        return matchWildcard(text, pattern);
    }
    text == pattern
}

fn matchWildcard(text: &str, pattern: &str) -> bool {
    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    Regex::new(&regex)
        .expect("gitignore wildcard regex must compile")
        .is_match(text)
}
