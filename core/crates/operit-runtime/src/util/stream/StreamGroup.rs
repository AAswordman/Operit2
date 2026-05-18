use crate::util::stream::Stream::{Stream, VecStream};

pub trait StreamProcessor<T, R> {
    fn process(&mut self, stream: &mut dyn Stream<Item = T>) -> R;
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
}

impl<T, R> StreamProcessor<T, R> for CompositeStreamProcessor<T, R>
where
    T: Clone,
{
    fn process(&mut self, stream: &mut dyn Stream<Item = T>) -> R {
        let mut values = Vec::new();
        stream.collect(&mut |value| values.push(value));
        for processor in &mut self.processors {
            let mut clone_stream = VecStream::new(values.clone());
            processor.process(&mut clone_stream);
        }
        let mut final_stream = VecStream::new(values);
        self.final_processor.process(&mut final_stream)
    }
}

pub struct StreamGroup<TAG> {
    pub tag: TAG,
    pub stream: Box<dyn Stream<Item = String>>,
    pub children: Vec<StreamGroup<String>>,
}

impl<TAG> StreamGroup<TAG> {
    pub fn new(tag: TAG, stream: Box<dyn Stream<Item = String>>) -> Self {
        Self {
            tag,
            stream,
            children: Vec::new(),
        }
    }

    pub fn collect(&mut self, collector: &mut dyn FnMut(String)) {
        self.stream.collect(collector);
    }

    pub fn add_child(&mut self, child: StreamGroup<String>) -> &mut Self {
        self.children.push(child);
        self
    }

    pub fn process_recursively(&mut self, action: &mut dyn FnMut(&mut StreamGroup<TAG>)) {
        action(self);
    }

    pub fn to_pair(self) -> (TAG, Box<dyn Stream<Item = String>>) {
        (self.tag, self.stream)
    }
}

pub struct StreamGroupBuilder<TAG> {
    tag: Option<TAG>,
    stream: Option<Box<dyn Stream<Item = String>>>,
    children: Vec<StreamGroup<String>>,
}

impl<TAG> Default for StreamGroupBuilder<TAG> {
    fn default() -> Self {
        Self {
            tag: None,
            stream: None,
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

    pub fn add_child(&mut self, child: StreamGroup<String>) -> &mut Self {
        self.children.push(child);
        self
    }

    pub fn build(mut self) -> StreamGroup<TAG> {
        StreamGroup {
            tag: self.tag.expect("tag must be set"),
            stream: self.stream.expect("stream must be set"),
            children: std::mem::take(&mut self.children),
        }
    }
}

pub fn stream_group<TAG>(init: impl FnOnce(&mut StreamGroupBuilder<TAG>)) -> StreamGroup<TAG> {
    let mut builder = StreamGroupBuilder::default();
    init(&mut builder);
    builder.build()
}

pub struct StreamInterceptor<T, R> {
    values: Vec<T>,
    on_each: Box<dyn FnMut(T) -> R>,
}

impl<T, R> StreamInterceptor<T, R>
where
    T: Clone,
{
    pub fn new(mut source_stream: impl Stream<Item = T>, on_each: impl FnMut(T) -> R + 'static) -> Self {
        let mut values = Vec::new();
        source_stream.collect(&mut |value| values.push(value));
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

    pub fn set_on_each(&mut self, on_each: impl FnMut(T) -> R + 'static) {
        self.on_each = Box::new(on_each);
    }
}
