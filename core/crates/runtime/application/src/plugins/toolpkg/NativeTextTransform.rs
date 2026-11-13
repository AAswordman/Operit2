use std::sync::Arc;

use operit_model::{MessagePart::MessagePartKind, MessagePartCodec::AssistantMarkupStreamState};
pub use operit_plugin_sdk::js_sdk::toolpkg::{
    ToolPkgTextTransformBatch as TextTransformBatch,
    ToolPkgTextTransformProgram as TextTransformProgram,
    ToolPkgTextTransformRule as TextTransformRule,
};
use regex::{Captures, Regex, RegexBuilder};
use regex_automata::{
    dfa::{dense, Automaton},
    util::{primitives::StateID, start, syntax},
};

/// Shares compiled automata across observers while keeping their mutable states separate.
#[derive(Clone, Default)]
pub struct CompiledTextTransformProgram {
    rules: Vec<Arc<CompiledRule>>,
}

struct CompiledRule {
    matcher: CompiledMatcher,
    global: bool,
    kinds: Vec<String>,
}

enum CompiledMatcher {
    Literal { needle: String, replacement: String },
    Automaton {
        regex: Regex,
        dfa: dense::DFA<Vec<u32>>,
        replacement: Vec<ReplacementToken>,
        trim_strings: Vec<String>,
    },
}

enum ReplacementToken {
    Literal(String),
    Capture(usize),
}

impl CompiledTextTransformProgram {
    /// Validates the entire assignment before compiling each shared pipeline exactly once.
    pub fn compile_batch(batch: TextTransformBatch, count: usize) -> Result<Vec<Self>, String> {
        if batch.messagePrograms.len() != count {
            return Err(format!("Native transform returned {} assignments for {count} messages", batch.messagePrograms.len()));
        }
        if batch.messagePrograms.iter().any(|index| *index as usize >= batch.programs.len()) {
            return Err("Native transform program index is out of range".into());
        }
        let programs: Vec<_> = batch.programs.iter().map(Self::compile).collect::<Result<_, _>>()?;
        Ok(batch.messagePrograms.into_iter().map(|index| programs[index as usize].clone()).collect())
    }

    /// Validates flags, scopes, automaton limits, and forward progress before any output begins.
    pub fn compile(program: &TextTransformProgram) -> Result<Self, String> {
        let mut rules = Vec::new();
        for rule in &program.rules {
            if rule.kinds.is_empty()
                || rule
                    .kinds
                    .iter()
                    .any(|kind| !matches!(kind.as_str(), "markdown" | "thinking"))
            {
                return Err("Native text transform kinds must select markdown or thinking".into());
            }
            let mut config = syntax::Config::new();
            let mut builder = RegexBuilder::new(&rule.pattern);
            let mut parser = regex_syntax::ParserBuilder::new();
            let mut seen = Vec::new();
            for flag in rule.flags.chars() {
                if seen.contains(&flag) {
                    return Err(format!("Duplicate native regex flag: {flag}"));
                }
                seen.push(flag);
                match flag {
                    'g' | 'u' => {}
                    'i' => {
                        config = config.case_insensitive(true);
                        builder.case_insensitive(true);
                        parser.case_insensitive(true);
                    }
                    'm' => {
                        config = config.multi_line(true);
                        builder.multi_line(true);
                        parser.multi_line(true);
                    }
                    's' => {
                        config = config.dot_matches_new_line(true);
                        builder.dot_matches_new_line(true);
                        parser.dot_matches_new_line(true);
                    }
                    _ => return Err(format!("Unsupported native regex flag: {flag}")),
                }
            }
            let hir = parser.build().parse(&rule.pattern)
                .map_err(|error| format!("Invalid native regex: {error}"))?;
            let matcher = if let regex_syntax::hir::HirKind::Literal(literal) = hir.kind() {
                let needle = String::from_utf8(literal.0.to_vec()).map_err(|error| error.to_string())?;
                if needle.is_empty() { return Err("Native streaming regex must consume at least one character".into()); }
                let mut replacement = String::new();
                for token in compile_replacement(&rule.replacement, &[None]) {
                    match token {
                        ReplacementToken::Literal(text) => replacement.push_str(&text),
                        ReplacementToken::Capture(_) => {
                            let mut captured = needle.clone();
                            for trim in &rule.trimStrings { captured = captured.replace(trim, ""); }
                            replacement.push_str(&captured);
                        }
                    }
                }
                CompiledMatcher::Literal { needle, replacement }
            } else {
                let regex = builder.build().map_err(|error| format!("Invalid native regex: {error}"))?;
                let dfa = dense::Builder::new()
                .configure(
                    dense::Config::new()
                        .specialize_start_states(true)
                        .dfa_size_limit(Some(8 * 1024 * 1024)),
                )
                .syntax(config)
                .build(&rule.pattern)
                .map_err(|error| format!("Regex cannot run as a native stream: {error}"))?;
                if dfa.has_empty() {
                    return Err("Native streaming regex must consume at least one character".into());
                }
                let names: Vec<_> = regex.capture_names().map(|name| name.map(str::to_owned)).collect();
                let replacement = compile_replacement(&rule.replacement, &names);
                CompiledMatcher::Automaton { regex, dfa, replacement, trim_strings: rule.trimStrings.clone() }
            };
            rules.push(Arc::new(CompiledRule {
                matcher,
                global: rule.flags.chars().any(|flag| flag == 'g'),
                kinds: rule.kinds.clone(),
            }));
        }
        Ok(CompiledTextTransformProgram { rules })
    }
}

/// Classifies canonical markup incrementally and edits only semantic text, never tool payloads.
#[derive(Clone)]
pub struct NativeDisplayStream {
    program: Arc<CompiledTextTransformProgram>,
    markup: AssistantMarkupStreamState,
    active: Option<(MessagePartKind, NativeTextTransformStream)>,
}

impl NativeDisplayStream {
    /// Creates independent observer state sharing only immutable compiled rules.
    pub fn new(program: Arc<CompiledTextTransformProgram>) -> Self {
        Self {
            program,
            markup: AssistantMarkupStreamState::forSegments(),
            active: None,
        }
    }

    /// Consumes one physical provider chunk without reparsing emitted source.
    pub fn push(&mut self, chunk: &str) -> Result<String, String> {
        if self.program.is_empty() {
            return Ok(chunk.into());
        }
        self.markup.push(chunk)?;
        Ok(self.drain())
    }

    /// Settles pending regexes before the existing Markdown renderer completes.
    pub fn finish(&mut self) -> Result<String, String> {
        if self.program.is_empty() {
            return Ok(String::new());
        }
        self.markup.complete()?;
        let mut output = self.drain();
        if let Some((_, mut stream)) = self.active.take() {
            output.push_str(&stream.finish());
        }
        Ok(output)
    }

    /// Flushes at semantic boundaries while preserving original tag spelling and byte order.
    fn drain(&mut self) -> String {
        let mut output = String::new();
        for segment in self.markup.takeSegments() {
            if self.active.as_ref().map(|(kind, _)| kind) != segment.kind.as_ref() {
                if let Some((_, mut stream)) = self.active.take() {
                    output.push_str(&stream.finish());
                }
            }
            match segment.kind {
                Some(kind) => {
                    let (_, stream) = self.active.get_or_insert_with(|| {
                        let scope = if kind == MessagePartKind::Thinking {
                            "thinking"
                        } else {
                            "markdown"
                        };
                        (kind, self.program.stream(scope))
                    });
                    output.push_str(&stream.push(&segment.content));
                }
                None => output.push_str(&segment.content),
            }
        }
        output
    }
}

impl CompiledTextTransformProgram {
    /// Reports the direct path without allocating a transform stream.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Preserves registration order across multiple plugin programs.
    pub fn extend(&mut self, other: Self) {
        self.rules.extend(other.rules);
    }

    /// Creates a fresh ordered pipeline for one canonical text part.
    pub fn stream(&self, kind: &str) -> NativeTextTransformStream {
        NativeTextTransformStream {
            stages: self
                .rules
                .iter()
                .filter(|rule| rule.kinds.iter().any(|scope| scope == kind))
                .map(|rule| RegexStage::new(rule.clone()))
                .collect(),
        }
    }

    /// Uses the same literal or automaton stream for completed display copies.
    pub fn apply(&self, kind: &str, text: &str) -> String {
        let mut stream = self.stream(kind);
        let mut output = stream.push(text);
        output.push_str(&stream.finish());
        output
    }
}

/// Keeps only unresolved matching input and supports cheap state snapshots at explicit savepoints.
#[derive(Clone)]
pub struct NativeTextTransformStream {
    stages: Vec<RegexStage>,
}

impl NativeTextTransformStream {
    /// Runs newly arrived text through the ordered stages without revisiting committed output.
    pub fn push(&mut self, text: &str) -> String {
        let Some((first, later)) = self.stages.split_first_mut() else { return text.to_string(); };
        let mut text = first.process(text, false);
        for stage in later {
            text = stage.process(&text, false);
        }
        text
    }

    /// Flushes each stage through all later stages before closing the part.
    pub fn finish(&mut self) -> String {
        let mut output = String::new();
        for index in 0..self.stages.len() {
            let mut tail = self.stages[index].process("", true);
            for later in &mut self.stages[index + 1..] {
                if tail.is_empty() { break; }
                tail = later.process(&tail, false);
            }
            output.push_str(&tail);
        }
        output
    }

    /// Measures retained unresolved input for regression and performance checks.
    pub fn buffered_bytes(&self) -> usize {
        self.stages
            .iter()
            .map(|stage| stage.buffer.len() - stage.start)
            .sum()
    }
}

#[derive(Clone)]
struct RegexStage {
    rule: Arc<CompiledRule>,
    buffer: String,
    start: usize,
    scanned: usize,
    state: StateID,
    matched: bool,
    done: bool,
}

impl RegexStage {
    /// Starts one unanchored DFA search at the beginning of a semantic text part.
    fn new(rule: Arc<CompiledRule>) -> Self {
        let state = match &rule.matcher {
            CompiledMatcher::Literal { .. } => StateID::ZERO,
            CompiledMatcher::Automaton { dfa, .. } => dfa.start_state(&start::Config::new())
                .expect("unanchored native DFA is configured"),
        };
        Self {
            rule,
            buffer: String::new(),
            start: 0,
            scanned: 0,
            state,
            matched: false,
            done: false,
        }
    }

    /// Carries DFA state across chunks, retaining context only while a match is undecided.
    fn process(&mut self, text: &str, final_input: bool) -> String {
        if self.done {
            return text.to_string();
        }
        let (regex, dfa, replacement, trim_strings) = match &self.rule.matcher {
            CompiledMatcher::Literal { needle, replacement } => return stream_literal(
                &mut self.buffer, &mut self.done, needle, replacement, self.rule.global, text, final_input),
            CompiledMatcher::Automaton { regex, dfa, replacement, trim_strings } => (regex, dfa, replacement, trim_strings),
        };
        self.buffer.push_str(text);
        let mut output = String::new();
        let mut consumed = self.start;
        loop {
            if self.scanned < self.buffer.len() {
                self.state = dfa.next_state(self.state, self.buffer.as_bytes()[self.scanned]);
                self.scanned += 1;
                if dfa.is_match_state(self.state) {
                    self.matched = true;
                }
                if dfa.is_start_state(self.state)
                    && !self.matched
                    && self.buffer.is_char_boundary(self.scanned)
                {
                    output.push_str(&self.buffer[consumed..self.scanned]);
                    consumed = self.scanned;
                }
                if !dfa.is_dead_state(self.state) {
                    continue;
                }
            } else if final_input {
                self.state = dfa.next_eoi_state(self.state);
                if dfa.is_match_state(self.state) {
                    self.matched = true;
                }
            } else {
                break;
            }

            if !self.matched {
                output.push_str(&self.buffer[consumed..]);
                consumed = self.buffer.len();
                self.scanned = consumed;
                self.done = true;
                break;
            }
            let captures = regex.captures_at(&self.buffer, consumed)
                .expect("capture engine agrees with the compiled stream automaton");
            let matched = captures
                .get(0)
                .expect("a successful regex has its complete capture");
            output.push_str(&self.buffer[consumed..matched.start()]);
            append_replacement(
                &mut output,
                replacement,
                &captures,
                trim_strings,
            );
            consumed = matched.end();
            self.scanned = consumed;
            self.matched = false;
            if !self.rule.global {
                output.push_str(&self.buffer[consumed..]);
                consumed = self.buffer.len();
                self.scanned = consumed;
                self.done = true;
                break;
            }
            self.state = dfa.start_state(
                    &start::Config::new().look_behind(
                        consumed
                            .checked_sub(1)
                            .map(|index| self.buffer.as_bytes()[index]),
                    ),
                )
                .expect("unanchored native DFA is configured");
        }
        // Retain one complete character for anchors and boundary assertions when extracting captures.
        let retained_start = self.buffer[..consumed]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index);
        self.buffer.drain(..retained_start);
        self.start = consumed - retained_start;
        self.scanned -= retained_start;
        output
    }
}

/// Resolves replacement references once, outside the streaming match loop.
fn compile_replacement(template: &str, names: &[Option<String>]) -> Vec<ReplacementToken> {
    let mut offset = 0;
    let mut cursor = 0;
    let mut tokens = Vec::new();
    while let Some(relative) = template[cursor..].find('$') {
        let start = cursor + relative;
        let tail = &template[start + 1..];
        let (length, token) = if tail.starts_with('$') {
            (1, Some(ReplacementToken::Literal("$".into())))
        } else if let Some(named) = tail.strip_prefix('<').and_then(|value| value.split_once('>')).filter(|(name, _)| !name.is_empty()) {
            (named.0.len() + 2, names.iter().position(|name| name.as_deref() == Some(named.0)).map(ReplacementToken::Capture))
        } else {
            let length = tail.bytes().take_while(u8::is_ascii_digit).count();
            if length == 0 { cursor = start + 1; continue; }
            (length, tail[..length].parse::<usize>().ok().filter(|index| *index < names.len()).map(ReplacementToken::Capture))
        };
        if offset < start { tokens.push(ReplacementToken::Literal(template[offset..start].into())); }
        if let Some(token) = token { tokens.push(token); }
        offset = start + 1 + length;
        cursor = offset;
    }
    if offset < template.len() {
        tokens.push(ReplacementToken::Literal(template[offset..].into()));
    }
    tokens
}

/// Matches literal text across chunks and retains only a suffix that can complete the next match.
fn stream_literal(buffer: &mut String, done: &mut bool, needle: &str, replacement: &str, global: bool, text: &str, final_input: bool) -> String {
    buffer.push_str(text);
    let mut output = String::with_capacity(buffer.len());
    let mut consumed = 0;
    while let Some(relative) = buffer[consumed..].find(needle) {
        let start = consumed + relative;
        output.push_str(&buffer[consumed..start]);
        output.push_str(replacement);
        consumed = start + needle.len();
        if !global { *done = true; break; }
    }
    let tail = &buffer[consumed..];
    let mut retained = if final_input || *done { 0 } else { tail.len().min(needle.len() - 1) };
    while retained > 0 && (!tail.is_char_boundary(tail.len() - retained) || !needle.starts_with(&tail[tail.len() - retained..])) {
        retained -= 1;
    }
    output.push_str(&tail[..tail.len() - retained]);
    buffer.drain(..buffer.len() - retained);
    output
}

/// Expands precompiled captures without allocating capture strings when no trimming is requested.
fn append_replacement(
    output: &mut String,
    tokens: &[ReplacementToken],
    captures: &Captures<'_>,
    trims: &[String],
) {
    for token in tokens {
        match token {
            ReplacementToken::Literal(text) => output.push_str(text),
            ReplacementToken::Capture(index) => {
                if let Some(capture) = captures.get(*index) {
                    if trims.is_empty() {
                        output.push_str(capture.as_str());
                    } else {
                        let mut value = capture.as_str().to_string();
                        for trim in trims {
                            value = value.replace(trim, "");
                        }
                        output.push_str(&value);
                    }
                }
            }
        }
    }
}
