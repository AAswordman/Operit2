use crate::util::stream::Stream::VecStream;

pub trait StringExtensions {
    fn stream(&self) -> VecStream<char>;
}

impl StringExtensions for str {
    fn stream(&self) -> VecStream<char> {
        VecStream::new(self.chars().collect::<Vec<_>>())
    }
}

impl StringExtensions for String {
    fn stream(&self) -> VecStream<char> {
        self.as_str().stream()
    }
}

pub fn string_stream(source: &str) -> VecStream<char> {
    source.stream()
}
