use crate::stream::Stream::{Stream, VecStream};
use crate::stream::StreamGroup::StreamGroup;
use crate::streamnative::NativeMarkdownSplitter::{
    MarkdownNodeStable, MarkdownProcessorType, NativeMarkdownSplitter,
};

#[allow(non_snake_case)]
pub trait NativeMarkdownStreamOperators {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable>;
    fn nativeMarkdownSplitByBlockGroups(
        &self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
    fn nativeMarkdownSplitByInlineGroups(
        &self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
}

#[allow(non_snake_case)]
impl NativeMarkdownStreamOperators for str {
    fn nativeMarkdownSplitByBlock(&self) -> Vec<MarkdownNodeStable> {
        NativeMarkdownSplitter::native_markdown_split_by_block(self)
    }

    fn nativeMarkdownSplitByInline(&self) -> Vec<MarkdownNodeStable> {
        NativeMarkdownSplitter::native_markdown_split_by_inline(self)
    }

    fn nativeMarkdownSplitByBlockGroups(
        &self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        NativeMarkdownSplitter::native_markdown_split_by_block_groups(self)
    }

    fn nativeMarkdownSplitByInlineGroups(
        &self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
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

    fn nativeMarkdownSplitByBlockGroups(
        &self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        self.as_str().nativeMarkdownSplitByBlockGroups()
    }

    fn nativeMarkdownSplitByInlineGroups(
        &self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        self.as_str().nativeMarkdownSplitByInlineGroups()
    }
}

#[allow(non_snake_case)]
pub trait NativeMarkdownCharStreamOperators: Stream<Item = char> {
    async fn nativeMarkdownSplitByBlockStream(&mut self) -> Vec<MarkdownNodeStable>;
    async fn nativeMarkdownSplitByInlineStream(&mut self) -> Vec<MarkdownNodeStable>;
    async fn nativeMarkdownSplitByBlockGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
    async fn nativeMarkdownSplitByInlineGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
}

#[allow(non_snake_case)]
impl<S> NativeMarkdownCharStreamOperators for S
where
    S: Stream<Item = char>,
{
    async fn nativeMarkdownSplitByBlockStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chars = Vec::new();
        self.collect(&mut |ch| chars.push(ch)).await;
        NativeMarkdownSplitter::native_markdown_split_stream_by_block(
            crate::stream::Stream::VecStream::new(chars),
        )
        .await
    }

    async fn nativeMarkdownSplitByInlineStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chars = Vec::new();
        self.collect(&mut |ch| chars.push(ch)).await;
        NativeMarkdownSplitter::native_markdown_split_stream_by_inline(
            crate::stream::Stream::VecStream::new(chars),
        )
        .await
    }

    async fn nativeMarkdownSplitByBlockGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |ch| content.push(ch)).await;
        NativeMarkdownSplitter::native_markdown_split_by_block_groups(&content)
    }

    async fn nativeMarkdownSplitByInlineGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |ch| content.push(ch)).await;
        NativeMarkdownSplitter::native_markdown_split_by_inline_groups(&content)
    }
}

#[allow(non_snake_case)]
pub trait NativeMarkdownStringStreamOperators: Stream<Item = String> {
    async fn nativeMarkdownSplitByBlockStringStream(&mut self) -> Vec<MarkdownNodeStable>;
    async fn nativeMarkdownSplitByInlineStringStream(&mut self) -> Vec<MarkdownNodeStable>;
    async fn nativeMarkdownSplitByBlockStringGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
    async fn nativeMarkdownSplitByInlineStringGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>>;
}

#[allow(non_snake_case)]
impl<S> NativeMarkdownStringStreamOperators for S
where
    S: Stream<Item = String>,
{
    async fn nativeMarkdownSplitByBlockStringStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chunks = Vec::new();
        self.collect(&mut |chunk| chunks.push(chunk)).await;
        NativeMarkdownSplitter::native_markdown_split_string_stream_by_block(
            crate::stream::Stream::VecStream::new(chunks),
        )
        .await
    }

    async fn nativeMarkdownSplitByInlineStringStream(&mut self) -> Vec<MarkdownNodeStable> {
        let mut chunks = Vec::new();
        self.collect(&mut |chunk| chunks.push(chunk)).await;
        NativeMarkdownSplitter::native_markdown_split_string_stream_by_inline(
            crate::stream::Stream::VecStream::new(chunks),
        )
        .await
    }

    async fn nativeMarkdownSplitByBlockStringGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |chunk| content.push_str(&chunk)).await;
        NativeMarkdownSplitter::native_markdown_split_by_block_groups(&content)
    }

    async fn nativeMarkdownSplitByInlineStringGroupStream(
        &mut self,
    ) -> VecStream<StreamGroup<Option<MarkdownProcessorType>>> {
        let mut content = String::new();
        self.collect(&mut |chunk| content.push_str(&chunk)).await;
        NativeMarkdownSplitter::native_markdown_split_by_inline_groups(&content)
    }
}
