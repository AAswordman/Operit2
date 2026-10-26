use super::ModelConfigConnectionTester;
use crate::chat::llmprovider::AIService::{response_stream_from_chunks, AiServiceError};
use operit_util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel_and_failure, TerminalFailureState,
};
use operit_util::stream::Stream::FnStream;

/// Rejects deferred producer failures even when partial text arrived without an error callback.
#[tokio::test]
async fn connection_probe_rejects_failure_reported_during_collection() {
    for chunks in [Vec::new(), vec!["unfinished answer".to_string()]] {
        let failure = TerminalFailureState::new();
        let producer_failure = failure.clone();
        let event_channel = empty_revisable_event_channel();
        let producer_events = event_channel.clone();
        let error = AiServiceError::ConnectionFailed("transport closed".to_string());
        let producer_error = error.to_string();
        let stream = FnStream::new(move |emit| {
            let chunks = chunks.clone();
            let failure = producer_failure.clone();
            let events = producer_events.clone();
            let error = producer_error.clone();
            Box::pin(async move {
                for chunk in chunks {
                    emit(chunk);
                }
                failure.set(error);
                events.close();
            })
        });
        let response = Box::new(with_event_channel_and_failure(
            stream,
            event_channel,
            failure,
        ));

        assert_eq!(
            ModelConfigConnectionTester::verifyTestResponse(response).await,
            Err(AiServiceError::RequestFailed(error.to_string()).to_string()),
        );
    }
}

/// Accepts successful completion for both empty responses and responses with text.
#[tokio::test]
async fn connection_probe_accepts_successful_completion() {
    for chunks in [Vec::new(), vec!["answer".to_string()]] {
        assert_eq!(
            ModelConfigConnectionTester::verifyTestResponse(response_stream_from_chunks(chunks))
                .await,
            Ok(()),
        );
    }
}
