use crate::streamnative::NativeMarkdownSplitter::{MarkdownNodeStable, NativeMarkdownSplitter};

#[allow(non_snake_case)]
pub trait NativeMarkdownStreamOperators {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable>;
}

#[allow(non_snake_case)]
impl NativeMarkdownStreamOperators for str {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable> {
        NativeMarkdownSplitter::native_markdown_split_by_block(self)
    }

    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable> {
        NativeMarkdownSplitter::native_markdown_split_by_inline(self)
    }
}

#[allow(non_snake_case)]
impl NativeMarkdownStreamOperators for String {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable> {
        self.as_str().nativeMarkdownSplitByBlock()
    }

    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable> {
        self.as_str().nativeMarkdownSplitByInline()
    }
}
