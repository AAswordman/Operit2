use std::collections::{HashMap, HashSet};

use operit_runtime::util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType,
};

#[derive(Clone, Debug, Default)]
pub(super) struct TuiMarkdownStreamState {
    builder: MarkdownEventNodeBuilder,
}

impl TuiMarkdownStreamState {
    pub(super) fn savepoint(&mut self, id: String) {
        self.builder.savepoint(id);
    }

    pub(super) fn rollback(&mut self, id: &str) {
        self.builder.rollback(id);
    }

    pub(super) fn start_block(&mut self, block_id: u64, node_type: MarkdownProcessorType) {
        self.builder.start_block(block_id, node_type);
    }

    pub(super) fn append_block(&mut self, block_id: u64, content: &str) {
        self.builder.append_block(block_id, content);
    }

    pub(super) fn start_inline(
        &mut self,
        block_id: u64,
        inline_id: u64,
        node_type: MarkdownProcessorType,
    ) {
        self.builder.start_inline(block_id, inline_id, node_type);
    }

    pub(super) fn append_inline(&mut self, block_id: u64, inline_id: u64, content: &str) {
        self.builder.append_inline(block_id, inline_id, content);
    }

    pub(super) fn complete(&mut self) {}

    pub(super) fn stable_nodes(&self) -> Vec<MarkdownNodeStable> {
        self.builder.stable_nodes()
    }
}

#[derive(Clone, Debug, Default)]
struct MarkdownEventNodeBuilder {
    nodes: Vec<MutableMarkdownNode>,
    blocks: HashMap<u64, usize>,
    inlines: HashMap<(u64, u64), usize>,
    html_break_blocks: HashSet<u64>,
    savepoints: HashMap<String, MarkdownEventNodeBuilderSnapshot>,
}

impl MarkdownEventNodeBuilder {
    fn savepoint(&mut self, id: String) {
        self.savepoints
            .insert(id, MarkdownEventNodeBuilderSnapshot::capture(self));
    }

    fn rollback(&mut self, id: &str) {
        if let Some(snapshot) = self.savepoints.get(id).cloned() {
            snapshot.restore(self);
        }
    }

    fn start_block(&mut self, block_id: u64, node_type: MarkdownProcessorType) {
        if node_type == MarkdownProcessorType::HtmlBreak {
            let node = MutableMarkdownNode {
                node_type,
                content: "\n".to_string(),
                children: Vec::new(),
            };
            self.nodes.push(node);
            self.blocks.insert(block_id, self.nodes.len() - 1);
            self.html_break_blocks.insert(block_id);
            return;
        }

        let node = MutableMarkdownNode {
            node_type,
            content: String::new(),
            children: Vec::new(),
        };
        self.nodes.push(node);
        self.blocks.insert(block_id, self.nodes.len() - 1);
    }

    fn append_block(&mut self, block_id: u64, content: &str) {
        if self.html_break_blocks.contains(&block_id) {
            return;
        }
        let index = self
            .blocks
            .get(&block_id)
            .copied()
            .unwrap_or_else(|| panic!("missing markdown block {block_id}"));
        self.nodes[index].content.push_str(content);
    }

    fn start_inline(&mut self, block_id: u64, inline_id: u64, node_type: MarkdownProcessorType) {
        if self.html_break_blocks.contains(&block_id) {
            return;
        }
        let block_index =
            self.blocks.get(&block_id).copied().unwrap_or_else(|| {
                panic!("missing markdown block {block_id} for inline {inline_id}")
            });
        let child = MutableMarkdownNode {
            node_type,
            content: String::new(),
            children: Vec::new(),
        };
        self.nodes[block_index].children.push(child);
        let child_index = self.nodes[block_index].children.len() - 1;
        self.inlines.insert((block_id, inline_id), child_index);
    }

    fn append_inline(&mut self, block_id: u64, inline_id: u64, content: &str) {
        if self.html_break_blocks.contains(&block_id) {
            return;
        }
        let block_index =
            self.blocks.get(&block_id).copied().unwrap_or_else(|| {
                panic!("missing markdown block {block_id} for inline {inline_id}")
            });
        let child_index = self
            .inlines
            .get(&(block_id, inline_id))
            .copied()
            .unwrap_or_else(|| panic!("missing markdown inline {block_id}/{inline_id}"));
        self.nodes[block_index].content.push_str(content);
        self.nodes[block_index].children[child_index]
            .content
            .push_str(content);
    }

    fn stable_nodes(&self) -> Vec<MarkdownNodeStable> {
        self.nodes
            .iter()
            .map(MutableMarkdownNode::to_stable)
            .collect()
    }
}

#[derive(Clone, Debug)]
struct MutableMarkdownNode {
    node_type: MarkdownProcessorType,
    content: String,
    children: Vec<MutableMarkdownNode>,
}

impl MutableMarkdownNode {
    fn to_stable(&self) -> MarkdownNodeStable {
        MarkdownNodeStable {
            r#type: self.node_type,
            content: self.content.clone(),
            children: self
                .children
                .iter()
                .filter(|child| !child.content.is_empty() || !child.children.is_empty())
                .map(MutableMarkdownNode::to_stable)
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct MarkdownEventNodeBuilderSnapshot {
    nodes: Vec<MutableMarkdownNode>,
    blocks: HashMap<u64, usize>,
    inlines: HashMap<(u64, u64), usize>,
    html_break_blocks: HashSet<u64>,
}

impl MarkdownEventNodeBuilderSnapshot {
    fn capture(builder: &MarkdownEventNodeBuilder) -> Self {
        Self {
            nodes: builder.nodes.clone(),
            blocks: builder.blocks.clone(),
            inlines: builder.inlines.clone(),
            html_break_blocks: builder.html_break_blocks.clone(),
        }
    }

    fn restore(self, builder: &mut MarkdownEventNodeBuilder) {
        builder.nodes = self.nodes;
        builder.blocks = self.blocks;
        builder.inlines = self.inlines;
        builder.html_break_blocks = self.html_break_blocks;
    }
}

pub(super) fn markdown_type_from_event_label(label: Option<&str>) -> MarkdownProcessorType {
    match label {
        Some("Header") => MarkdownProcessorType::Header,
        Some("BlockQuote") => MarkdownProcessorType::BlockQuote,
        Some("CodeBlock") => MarkdownProcessorType::CodeBlock,
        Some("OrderedList") => MarkdownProcessorType::OrderedList,
        Some("UnorderedList") => MarkdownProcessorType::UnorderedList,
        Some("HorizontalRule") => MarkdownProcessorType::HorizontalRule,
        Some("BlockLatex") => MarkdownProcessorType::BlockLatex,
        Some("Table") => MarkdownProcessorType::Table,
        Some("XmlBlock") => MarkdownProcessorType::XmlBlock,
        Some("Bold") => MarkdownProcessorType::Bold,
        Some("Italic") => MarkdownProcessorType::Italic,
        Some("InlineCode") => MarkdownProcessorType::InlineCode,
        Some("Link") => MarkdownProcessorType::Link,
        Some("Image") => MarkdownProcessorType::Image,
        Some("Strikethrough") => MarkdownProcessorType::Strikethrough,
        Some("Underline") => MarkdownProcessorType::Underline,
        Some("InlineLatex") => MarkdownProcessorType::InlineLatex,
        Some("HtmlBreak") => MarkdownProcessorType::HtmlBreak,
        None => MarkdownProcessorType::PlainText,
        Some(label) => panic!("unknown markdown node type {label}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_state_builds_block_and_inline_nodes() {
        let mut state = TuiMarkdownStreamState::default();
        state.start_block(1, MarkdownProcessorType::PlainText);
        state.start_inline(1, 1, MarkdownProcessorType::PlainText);
        state.append_inline(1, 1, "hello");

        let nodes = state.stable_nodes();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].r#type, MarkdownProcessorType::PlainText);
        assert_eq!(nodes[0].content, "hello");
        assert_eq!(nodes[0].children.len(), 1);
        assert_eq!(nodes[0].children[0].content, "hello");
    }

    #[test]
    fn stream_state_rolls_back_to_savepoint() {
        let mut state = TuiMarkdownStreamState::default();
        state.start_block(1, MarkdownProcessorType::PlainText);
        state.start_inline(1, 1, MarkdownProcessorType::PlainText);
        state.append_inline(1, 1, "hello");
        state.savepoint("a".to_string());
        state.append_inline(1, 1, " world");
        state.rollback("a");

        let nodes = state.stable_nodes();
        assert_eq!(nodes[0].content, "hello");
        assert_eq!(nodes[0].children[0].content, "hello");
    }
}
