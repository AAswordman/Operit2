use crate::stream::Stream::{Stream, VecStream};
use std::future::Future;
use std::pin::Pin;

pub trait StreamProcessor<T, R>
where
    T: Send,
{
    /// Processes one stream asynchronously and returns the processor result.
    fn process<'a>(
        &'a mut self,
        stream: &'a mut dyn Stream<Item = T>,
    ) -> Pin<Box<dyn Future<Output = R> + 'a>>;
}

pub struct CompositeStreamProcessor<T, R> {
    processors: Vec<Box<dyn StreamProcessor<T, ()>>>,
    final_processor: Box<dyn StreamProcessor<T, R>>,
}

impl<T, R> CompositeStreamProcessor<T, R> {
    pub fn new(
        processors: Vec<Box<dyn StreamProcessor<T, ()>>>,
        final_processor: Box<dyn StreamProcessor<T, R>>,
    ) -> Self {
        Self {
            processors,
            final_processor,
        }
    }

    pub fn compose(
        processors: Vec<Box<dyn StreamProcessor<T, ()>>>,
        final_processor: Box<dyn StreamProcessor<T, R>>,
    ) -> Self {
        Self::new(processors, final_processor)
    }
}

impl<T, R> StreamProcessor<T, R> for CompositeStreamProcessor<T, R>
where
    T: Clone + Send + 'static,
    R: Send,
{
    /// Runs intermediate processors before the final processor.
    fn process<'a>(
        &'a mut self,
        stream: &'a mut dyn Stream<Item = T>,
    ) -> Pin<Box<dyn Future<Output = R> + 'a>> {
        Box::pin(async move {
            let mut values = Vec::new();
            stream.collect(&mut |value| values.push(value)).await;
            for processor in &mut self.processors {
                let mut clone_stream = VecStream::new(values.clone());
                processor.process(&mut clone_stream).await;
            }
            let mut final_stream = VecStream::new(values);
            self.final_processor.process(&mut final_stream).await
        })
    }
}

pub struct StreamGroup<TAG> {
    pub tag: TAG,
    pub stream: Box<dyn Stream<Item = String>>,
    pub processor: Option<Box<dyn StreamProcessor<String, ()>>>,
    pub children: Vec<StreamGroup<String>>,
}

impl<TAG> StreamGroup<TAG> {
    pub fn new(tag: TAG, stream: Box<dyn Stream<Item = String>>) -> Self {
        Self {
            tag,
            stream,
            processor: None,
            children: Vec::new(),
        }
    }

    pub fn new_with_processor(
        tag: TAG,
        stream: Box<dyn Stream<Item = String>>,
        processor: Option<Box<dyn StreamProcessor<String, ()>>>,
    ) -> Self {
        Self {
            tag,
            stream,
            processor,
            children: Vec::new(),
        }
    }

    pub async fn collect(&mut self, collector: &mut dyn FnMut(String)) {
        self.stream.collect(collector).await;
    }

    pub fn add_child(&mut self, child: StreamGroup<String>) -> &mut Self {
        self.children.push(child);
        self
    }

    pub fn process_recursively(&mut self, action: &mut dyn FnMut(&mut StreamGroup<TAG>)) {
        action(self);
    }

    pub fn process_children_recursively(
        &mut self,
        action: &mut dyn FnMut(&mut StreamGroup<String>),
    ) {
        for child in &mut self.children {
            child.process_recursively_string(action);
        }
    }

    pub async fn process_with_bound_processor(&mut self) -> Option<()> {
        match self.processor.as_mut() {
            Some(processor) => Some(processor.process(&mut *self.stream).await),
            None => None,
        }
    }

    pub fn to_pair(self) -> (TAG, Box<dyn Stream<Item = String>>) {
        (self.tag, self.stream)
    }
}

impl StreamGroup<String> {
    pub fn process_recursively_string(&mut self, action: &mut dyn FnMut(&mut StreamGroup<String>)) {
        action(self);
        for child in &mut self.children {
            child.process_recursively_string(action);
        }
    }
}

pub struct StreamGroupBuilder<TAG> {
    tag: Option<TAG>,
    stream: Option<Box<dyn Stream<Item = String>>>,
    processor: Option<Box<dyn StreamProcessor<String, ()>>>,
    children: Vec<StreamGroup<String>>,
}

impl<TAG> Default for StreamGroupBuilder<TAG> {
    fn default() -> Self {
        Self {
            tag: None,
            stream: None,
            processor: None,
            children: Vec::new(),
        }
    }
}

impl<TAG> StreamGroupBuilder<TAG> {
    pub fn tag(&mut self, tag: TAG) -> &mut Self {
        self.tag = Some(tag);
        self
    }

    pub fn stream(&mut self, stream: Box<dyn Stream<Item = String>>) -> &mut Self {
        self.stream = Some(stream);
        self
    }

    pub fn processor(&mut self, processor: Box<dyn StreamProcessor<String, ()>>) -> &mut Self {
        self.processor = Some(processor);
        self
    }

    pub fn add_child(&mut self, child: StreamGroup<String>) -> &mut Self {
        self.children.push(child);
        self
    }

    pub fn child(&mut self, init: impl FnOnce(&mut StreamGroupBuilder<String>)) -> &mut Self {
        let mut child_builder = StreamGroupBuilder::default();
        init(&mut child_builder);
        self.children.push(child_builder.build());
        self
    }

    pub fn build(mut self) -> StreamGroup<TAG> {
        StreamGroup {
            tag: self.tag.expect("tag must be set"),
            stream: self.stream.expect("stream must be set"),
            processor: self.processor.take(),
            children: std::mem::take(&mut self.children),
        }
    }
}

pub fn stream_group<TAG>(init: impl FnOnce(&mut StreamGroupBuilder<TAG>)) -> StreamGroup<TAG> {
    let mut builder = StreamGroupBuilder::default();
    init(&mut builder);
    builder.build()
}

pub fn as_nested_group<TAG>(
    stream: Box<dyn Stream<Item = String>>,
    tag: TAG,
    processor: Option<Box<dyn StreamProcessor<String, ()>>>,
    init: Option<impl FnOnce(&mut StreamGroupBuilder<TAG>)>,
) -> StreamGroup<TAG> {
    let mut builder = StreamGroupBuilder::default();
    builder.tag(tag).stream(stream);
    if let Some(processor) = processor {
        builder.processor(processor);
    }
    if let Some(init) = init {
        init(&mut builder);
    }
    builder.build()
}

pub fn as_stream_group<TAG>(
    pair: (TAG, Box<dyn Stream<Item = String>>),
    processor: Option<Box<dyn StreamProcessor<String, ()>>>,
) -> StreamGroup<TAG> {
    StreamGroup::new_with_processor(pair.0, pair.1, processor)
}

pub struct StreamInterceptor<T, R> {
    values: Vec<T>,
    on_each: Box<dyn FnMut(T) -> R + Send>,
}

impl<T, R> StreamInterceptor<T, R>
where
    T: Clone + Send,
    R: Send,
{
    pub async fn new(
        mut source_stream: impl Stream<Item = T>,
        on_each: impl FnMut(T) -> R + Send + 'static,
    ) -> Self {
        let mut values = Vec::new();
        source_stream.collect(&mut |value| values.push(value)).await;
        Self {
            values,
            on_each: Box::new(on_each),
        }
    }

    pub fn intercepted_stream(&mut self) -> VecStream<R> {
        VecStream::new(
            self.values
                .clone()
                .into_iter()
                .map(|value| (self.on_each)(value))
                .collect::<Vec<_>>(),
        )
    }

    pub fn set_on_each(&mut self, on_each: impl FnMut(T) -> R + Send + 'static) {
        self.on_each = Box::new(on_each);
    }
}
