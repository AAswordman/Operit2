use tokio::sync::mpsc;

use super::Stream::{CollectFuture, Stream};

/// Receives ordered values supplied by a caller through a reverse stream.
pub struct ReverseStream<T> {
    receiver: mpsc::Receiver<T>,
}

/// Sends values into one reverse stream until the input is completed.
pub struct ReverseStreamSender<T> {
    sender: Option<mpsc::Sender<T>>,
}

impl<T> ReverseStream<T> {
    /// Creates the two endpoints of one caller-owned input stream.
    pub fn channel() -> (ReverseStreamSender<T>, Self) {
        let (sender, receiver) = mpsc::channel(1);
        (
            ReverseStreamSender {
                sender: Some(sender),
            },
            Self { receiver },
        )
    }

    /// Receives the next input value or None after the producer closes.
    pub async fn recv(&mut self) -> Option<T> {
        self.receiver.recv().await
    }
}

impl<T> ReverseStreamSender<T> {
    /// Delivers one value in source order to the runtime consumer.
    pub async fn send(&self, value: T) -> Result<(), String> {
        self.sender
            .as_ref()
            .ok_or_else(|| "reverse stream is closed".to_string())?
            .send(value)
            .await
            .map_err(|_| "reverse stream consumer is closed".to_string())
    }

    /// Completes the input stream and releases its producer endpoint.
    pub fn close(&mut self) {
        self.sender.take();
    }
}

impl<T> Stream for ReverseStream<T>
where
    T: Send,
{
    type Item = T;

    /// Collects every value supplied before the caller completes the input stream.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            while let Some(value) = self.receiver.recv().await {
                collector(value);
            }
        })
    }
}
