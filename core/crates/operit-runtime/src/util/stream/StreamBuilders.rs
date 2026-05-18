use crate::util::stream::Stream::{FnStream, Stream, VecStream};

pub fn empty_stream<T>() -> VecStream<T> {
    VecStream::new(Vec::<T>::new())
}

pub fn stream_of<T>(value: T) -> VecStream<T> {
    VecStream::new(vec![value])
}

pub fn stream_of_many<T>(values: impl IntoIterator<Item = T>) -> VecStream<T> {
    VecStream::new(values)
}

pub fn collection_as_stream<T>(values: impl IntoIterator<Item = T>) -> VecStream<T> {
    VecStream::new(values)
}

pub fn stream<T>(block: impl FnMut(&mut dyn FnMut(T)) + 'static) -> FnStream<T> {
    FnStream::new(block)
}

pub fn range_stream(start: i32, count: usize) -> VecStream<i32> {
    VecStream::new((start..start + count as i32).collect::<Vec<_>>())
}

pub fn stream_error<T>(message: impl Into<String>) -> FnStream<Result<T, String>>
where
    T: 'static,
{
    let message = message.into();
    FnStream::new(move |emit| emit(Err(message.clone())))
}

pub fn collect_to_vec<S>(mut stream: S) -> Vec<S::Item>
where
    S: Stream,
{
    let mut values = Vec::new();
    stream.collect(&mut |value| values.push(value));
    values
}
