use crate::util::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType, NativeMarkdownSplitter,
};
use crate::util::stream::Stream::{Stream, VecStream};
use crate::util::stream::StreamGroup::StreamGroup;

#[allow(non_snake_case)]
pub trait NativeMarkdownStreamOperators {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByBlockGroups(&self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
    fn nativeMarkdownSplitByInlineGroups(&self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
}

#[allow(non_snake_case)]
impl NativeMarkdownStreamOperators for str {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable> {
        NativeMarkdownSplitter::native_markdown_split_by_block(self)
    }

    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable> {
        NativeMarkdownSplitter::native_markdown_split_by_inline(self)
    }

    fn nativeMarkdownSplitByBlockGroups(&self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        NativeMarkdownSplitter::native_markdown_split_by_block_groups(self)
    }

    fn nativeMarkdownSplitByInlineGroups(&self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        NativeMarkdownSplitter::native_markdown_split_by_inline_groups(self)
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

    fn nativeMarkdownSplitByBlockGroups(&self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        self.as_str().nativeMarkdownSplitByBlockGroups()
    }

    fn nativeMarkdownSplitByInlineGroups(&self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        self.as_str().nativeMarkdownSplitByInlineGroups()
    }
}

#[allow(non_snake_case)]
pub trait NativeMarkdownCharStreamOperators: Stream<Item = char> {
    fn nativeMarkdownSplitByBlockStream(&mut self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByInlineStream(&mut self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByBlockGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
    fn nativeMarkdownSplitByInlineGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
}

#[allow(non_snake_case)]
impl<S> NativeMarkdownCharStreamOperators for S
where
    S: Stream<Item = char>,
{
    fn nativeMarkdownSplitByBlockStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chars = Vec::new();
        self.collect(&mut |ch| chars.push(ch));
        NativeMarkdownSplitter::native_markdown_split_stream_by_block(
            crate::util::stream::Stream::VecStream::new(chars),
        )
    }

    fn nativeMarkdownSplitByInlineStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chars = Vec::new();
        self.collect(&mut |ch| chars.push(ch));
        NativeMarkdownSplitter::native_markdown_split_stream_by_inline(
            crate::util::stream::Stream::VecStream::new(chars),
        )
    }

    fn nativeMarkdownSplitByBlockGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |ch| content.push(ch));
        NativeMarkdownSplitter::native_markdown_split_by_block_groups(&content)
    }

    fn nativeMarkdownSplitByInlineGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |ch| content.push(ch));
        NativeMarkdownSplitter::native_markdown_split_by_inline_groups(&content)
    }
}

#[allow(non_snake_case)]
pub trait NativeMarkdownStringStreamOperators: Stream<Item = String> {
    fn nativeMarkdownSplitByBlockStringStream(&mut self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByInlineStringStream(&mut self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByBlockStringGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
    fn nativeMarkdownSplitByInlineStringGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
}

#[allow(non_snake_case)]
impl<S> NativeMarkdownStringStreamOperators for S
where
    S: Stream<Item = String>,
{
    fn nativeMarkdownSplitByBlockStringStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chunks = Vec::new();
        self.collect(&mut |chunk| chunks.push(chunk));
        NativeMarkdownSplitter::native_markdown_split_string_stream_by_block(
            crate::util::stream::Stream::VecStream::new(chunks),
        )
    }

    fn nativeMarkdownSplitByInlineStringStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chunks = Vec::new();
        self.collect(&mut |chunk| chunks.push(chunk));
        NativeMarkdownSplitter::native_markdown_split_string_stream_by_inline(
            crate::util::stream::Stream::VecStream::new(chunks),
        )
    }

    fn nativeMarkdownSplitByBlockStringGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |chunk| content.push_str(&chunk));
        NativeMarkdownSplitter::native_markdown_split_by_block_groups(&content)
    }

    fn nativeMarkdownSplitByInlineStringGroupStream(&mut self) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |chunk| content.push_str(&chunk));
        NativeMarkdownSplitter::native_markdown_split_by_inline_groups(&content)
    }
}
