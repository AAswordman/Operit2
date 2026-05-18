use std::collections::BTreeMap;

use crate::util::stream::StreamKmpMatchResult::StreamKmpMatchResult;

#[derive(Clone)]
pub enum KmpCondition {
    Char(char),
    Range(char, char),
    Set(Vec<char>),
    Not(Box<KmpCondition>),
    Or(Vec<KmpCondition>),
    And(Vec<KmpCondition>),
    Predicate {
        description: String,
        predicate: fn(char) -> bool,
    },
    GreedyStar(Box<KmpCondition>),
    Group {
        group_id: i32,
        conditions: Vec<KmpCondition>,
    },
}

impl std::fmt::Debug for KmpCondition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.get_description())
    }
}

impl KmpCondition {
    pub fn matches(&self, c: char) -> bool {
        match self {
            KmpCondition::Char(expected) => c == *expected,
            KmpCondition::Range(from, to) => (*from..=*to).contains(&c),
            KmpCondition::Set(chars) => chars.contains(&c),
            KmpCondition::Not(condition) => !condition.matches(c),
            KmpCondition::Or(conditions) => conditions.iter().any(|condition| condition.matches(c)),
            KmpCondition::And(conditions) => conditions.iter().all(|condition| condition.matches(c)),
            KmpCondition::Predicate { predicate, .. } => predicate(c),
            KmpCondition::GreedyStar(condition) => condition.matches(c),
            KmpCondition::Group { .. } => false,
        }
    }

    pub fn get_description(&self) -> String {
        match self {
            KmpCondition::Char(c) => format!("'{c}'"),
            KmpCondition::Range(from, to) => format!("[{from}-{to}]"),
            KmpCondition::Set(chars) => format!("[{}]", chars.iter().collect::<String>()),
            KmpCondition::Not(condition) => format!("not({})", condition.get_description()),
            KmpCondition::Or(conditions) => format!(
                "({})",
                conditions
                    .iter()
                    .map(KmpCondition::get_description)
                    .collect::<Vec<_>>()
                    .join(" OR ")
            ),
            KmpCondition::And(conditions) => format!(
                "({})",
                conditions
                    .iter()
                    .map(KmpCondition::get_description)
                    .collect::<Vec<_>>()
                    .join(" AND ")
            ),
            KmpCondition::Predicate { description, .. } => description.clone(),
            KmpCondition::GreedyStar(condition) => format!("greedy*({})", condition.get_description()),
            KmpCondition::Group { group_id, .. } => format!("GROUP({group_id})"),
        }
    }

    pub fn get_all_group_ids(&self) -> Vec<i32> {
        match self {
            KmpCondition::Group {
                group_id,
                conditions,
            } => {
                let mut ids = vec![*group_id];
                for condition in conditions {
                    ids.extend(condition.get_all_group_ids());
                }
                ids
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KmpNode {
    pub id: usize,
    pub depth: usize,
    pub is_final: bool,
    transitions: Vec<(KmpCondition, usize)>,
    pub failure_node: Option<usize>,
}

impl KmpNode {
    pub fn add_transition(&mut self, condition: KmpCondition, target_node: usize) {
        self.transitions.push((condition, target_node));
    }

    pub fn get_next_node(&self, c: char) -> Option<usize> {
        self.transitions
            .iter()
            .find(|(condition, _)| condition.matches(c))
            .map(|(_, node)| *node)
    }

    pub fn transitions(&self) -> &[(KmpCondition, usize)] {
        &self.transitions
    }
}

#[derive(Debug, Clone, Default)]
pub struct KmpPattern {
    pub conditions: Vec<KmpCondition>,
    pub group_ids: Vec<i32>,
}

impl KmpPattern {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, condition: KmpCondition) {
        self.group_ids.extend(condition.get_all_group_ids());
        self.conditions.push(condition);
    }

    pub fn group(&mut self, id: i32, builder: impl FnOnce(&mut KmpPattern)) {
        let mut sub_pattern = KmpPattern::new();
        builder(&mut sub_pattern);
        self.add(KmpCondition::Group {
            group_id: id,
            conditions: sub_pattern.conditions,
        });
    }

    pub fn char(&mut self, c: char) {
        self.add(KmpCondition::Char(c));
    }

    pub fn char_ignore_case(&mut self, c: char) {
        self.add(KmpCondition::Or(vec![
            KmpCondition::Char(c.to_ascii_lowercase()),
            KmpCondition::Char(c.to_ascii_uppercase()),
        ]));
    }

    pub fn range(&mut self, from: char, to: char) {
        self.add(KmpCondition::Range(from, to));
    }

    pub fn any_of(&mut self, chars: &[char]) {
        self.add(KmpCondition::Set(chars.to_vec()));
    }

    pub fn not_char(&mut self, c: char) {
        self.add(KmpCondition::Not(Box::new(KmpCondition::Char(c))));
    }

    pub fn none_of(&mut self, chars: &[char]) {
        self.add(KmpCondition::Not(Box::new(KmpCondition::Set(chars.to_vec()))));
    }

    pub fn predicate(&mut self, description: &str, predicate: fn(char) -> bool) {
        self.add(KmpCondition::Predicate {
            description: description.to_string(),
            predicate,
        });
    }

    pub fn digit(&mut self) {
        self.predicate("digit", |ch| ch.is_ascii_digit());
    }

    pub fn letter(&mut self) {
        self.predicate("letter", |ch| ch.is_ascii_alphabetic());
    }

    pub fn whitespace(&mut self) {
        self.predicate("whitespace", |ch| ch.is_whitespace());
    }

    pub fn any(&mut self) {
        self.predicate("any", |_| true);
    }

    pub fn literal(&mut self, sequence: &str) {
        for ch in sequence.chars() {
            self.char(ch);
        }
    }

    pub fn greedy_star(&mut self, builder: impl FnOnce(&mut KmpPattern)) {
        let mut sub_pattern = KmpPattern::new();
        builder(&mut sub_pattern);
        if sub_pattern.conditions.is_empty() {
            return;
        }
        let condition = if sub_pattern.conditions.len() == 1 {
            sub_pattern.conditions.remove(0)
        } else {
            KmpCondition::Or(sub_pattern.conditions)
        };
        self.add(KmpCondition::GreedyStar(Box::new(condition)));
    }

    pub fn repeat(&mut self, count: usize, builder: impl Fn(&mut KmpPattern)) {
        let mut sub_pattern = KmpPattern::new();
        builder(&mut sub_pattern);
        for _ in 0..count {
            self.conditions.extend(sub_pattern.conditions.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamKmpGraph {
    nodes: Vec<KmpNode>,
    current_node: usize,
    start_node: usize,
    character_stream_buffer: String,
    current_match_length: usize,
    pattern: Option<KmpPattern>,
}

impl Default for StreamKmpGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamKmpGraph {
    pub fn new() -> Self {
        let mut graph = Self {
            nodes: Vec::new(),
            current_node: 0,
            start_node: 0,
            character_stream_buffer: String::new(),
            current_match_length: 0,
            pattern: None,
        };
        graph.start_node = graph.create_node(0, false);
        graph.current_node = graph.start_node;
        graph
    }

    pub fn create_node(&mut self, depth: usize, is_final: bool) -> usize {
        let id = self.nodes.len();
        self.nodes.push(KmpNode {
            id,
            depth,
            is_final,
            transitions: Vec::new(),
            failure_node: None,
        });
        id
    }

    pub fn add_transition(&mut self, from_node: usize, to_node: usize, condition: KmpCondition) {
        self.nodes[from_node].add_transition(condition, to_node);
    }

    pub fn set_failure(&mut self, node: usize, failure_node: usize) {
        self.nodes[node].failure_node = Some(failure_node);
    }

    pub fn process_char(&mut self, c: char) -> StreamKmpMatchResult {
        self.character_stream_buffer.push(c);
        let previous = self.current_node;
        let mut next = self.nodes[self.current_node].get_next_node(c);

        if next.is_some() {
            self.current_match_length += 1;
        } else {
            let mut search = self.nodes[self.current_node].failure_node;
            while let Some(search_node) = search {
                next = self.nodes[search_node].get_next_node(c);
                if next.is_some() {
                    self.current_match_length = self.nodes[search_node].depth + 1;
                    break;
                }
                if search_node == self.start_node {
                    break;
                }
                search = self.nodes[search_node].failure_node;
            }
            if next.is_none() {
                next = self.nodes[self.start_node].get_next_node(c);
                self.current_match_length = if next.is_some() { 1 } else { 0 };
            }
        }

        self.current_node = next.unwrap_or(self.start_node);

        if self.nodes[self.current_node].is_final {
            return self.perform_match(previous != self.current_node);
        }
        if self.current_node == self.start_node && self.current_match_length == 0 {
            StreamKmpMatchResult::NoMatch
        } else {
            StreamKmpMatchResult::InProgress
        }
    }

    pub fn process_text(&mut self, text: &str) -> Vec<usize> {
        self.reset();
        let mut positions = Vec::new();
        for (index, c) in text.chars().enumerate() {
            if self.process_char(c).is_full_match() {
                positions.push(index + 1);
            }
        }
        positions
    }

    pub fn reset(&mut self) {
        self.current_node = self.start_node;
        self.character_stream_buffer.clear();
        self.current_match_length = 0;
    }

    pub fn get_current_node(&self) -> &KmpNode {
        &self.nodes[self.current_node]
    }

    pub fn get_start_node(&self) -> &KmpNode {
        &self.nodes[self.start_node]
    }

    pub fn get_nodes(&self) -> &[KmpNode] {
        &self.nodes
    }

    pub fn find_matches(&mut self, text: &str) -> Vec<String> {
        let mut matches = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        self.reset();
        for (index, c) in chars.iter().copied().enumerate() {
            if self.process_char(c).is_full_match() {
                let start = index + 1 - self.current_match_length;
                matches.push(chars[start..=index].iter().collect());
            }
        }
        matches
    }

    fn perform_match(&self, is_full_match: bool) -> StreamKmpMatchResult {
        let mut groups = BTreeMap::new();
        if let Some(pattern) = &self.pattern {
            let text: Vec<char> = self.character_stream_buffer.chars().collect();
            let start = text.len().saturating_sub(self.current_match_length);
            capture_groups(&pattern.conditions, &text[start..], &mut groups);
        }
        StreamKmpMatchResult::Match {
            groups,
            is_full_match,
        }
    }
}

pub struct StreamKmpGraphBuilder;

impl StreamKmpGraphBuilder {
    pub fn build(pattern: KmpPattern) -> StreamKmpGraph {
        let mut graph = StreamKmpGraph::new();
        graph.pattern = Some(pattern.clone());
        let start_node = graph.start_node;
        let (final_node, _) = build_recursive(
            &mut graph,
            start_node,
            &pattern.conditions,
            0,
        );
        graph.nodes[final_node].is_final = true;
        setup_failure_transitions(&mut graph);
        graph
    }
}

pub fn kmp_pattern(init: impl FnOnce(&mut KmpPattern)) -> KmpPattern {
    let mut pattern = KmpPattern::new();
    init(&mut pattern);
    pattern
}

fn build_recursive(
    graph: &mut StreamKmpGraph,
    start_node: usize,
    conditions: &[KmpCondition],
    depth: usize,
) -> (usize, usize) {
    let mut current_node = start_node;
    let mut current_depth = depth;
    for (index, condition) in conditions.iter().cloned().enumerate() {
        match condition {
            KmpCondition::Group { conditions, .. } => {
                if conditions.is_empty() {
                    continue;
                }
                let (last, final_depth) =
                    build_recursive(graph, current_node, &conditions, current_depth);
                current_node = last;
                current_depth = final_depth;
            }
            KmpCondition::GreedyStar(inner) => {
                let next = conditions.get(index + 1).cloned();
                let loop_condition = if let Some(next_condition) = next {
                    KmpCondition::And(vec![*inner, KmpCondition::Not(Box::new(next_condition))])
                } else {
                    *inner
                };
                graph.add_transition(current_node, current_node, loop_condition);
            }
            other => {
                current_depth += 1;
                let next_node = graph.create_node(current_depth, false);
                graph.add_transition(current_node, next_node, other);
                current_node = next_node;
            }
        }
    }
    (current_node, current_depth)
}

fn setup_failure_transitions(graph: &mut StreamKmpGraph) {
    for index in 0..graph.nodes.len() {
        graph.nodes[index].failure_node = Some(graph.start_node);
    }
}

fn capture_groups(conditions: &[KmpCondition], text: &[char], groups: &mut BTreeMap<i32, String>) {
    let mut cursor = 0;
    for condition in conditions {
        match condition {
            KmpCondition::Group {
                group_id,
                conditions,
            } => {
                let start = cursor;
                cursor += count_consumed(conditions, &text[cursor..]);
                groups.insert(*group_id, text[start..cursor].iter().collect());
            }
            KmpCondition::GreedyStar(inner) => {
                while cursor < text.len() && inner.matches(text[cursor]) {
                    cursor += 1;
                }
            }
            _ => cursor += usize::from(cursor < text.len()),
        }
    }
}

fn count_consumed(conditions: &[KmpCondition], text: &[char]) -> usize {
    let mut cursor = 0;
    for condition in conditions {
        match condition {
            KmpCondition::GreedyStar(inner) => {
                while cursor < text.len() && inner.matches(text[cursor]) {
                    cursor += 1;
                }
            }
            KmpCondition::Group { conditions, .. } => {
                cursor += count_consumed(conditions, &text[cursor..]);
            }
            _ => cursor += usize::from(cursor < text.len()),
        }
    }
    cursor
}
