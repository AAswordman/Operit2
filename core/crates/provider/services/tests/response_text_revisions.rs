use operit_providers::chat::llmprovider::AIService::{collect_response_text, AiServiceError};
use operit_util::stream::HotStream::mutable_shared_stream;
use operit_util::stream::RevisableTextStream::{
    with_ordered_event_channel_and_failure, ResponseStreamItem, TerminalFailureState,
    TextStreamEvent, TextStreamEventType,
};
use operit_util::stream::Stream::VecStream;

/// Summaries use the same final-response collector and must discard interrupted attempts.
#[tokio::test]
async fn summary_collector_keeps_only_the_accepted_attempt() {
    let stream = with_ordered_event_channel_and_failure(VecStream::new(vec![
        ResponseStreamItem::Revision(TextStreamEvent { event_type: TextStreamEventType::Savepoint, id: "summary".into() }),
        ResponseStreamItem::Chunk("<think>interrupted".into()),
        ResponseStreamItem::Revision(TextStreamEvent { event_type: TextStreamEventType::Rollback, id: "summary".into() }),
        ResponseStreamItem::Chunk("摘要😀".into()),
        ResponseStreamItem::Chunk("完成".into()),
    ]), mutable_shared_stream(0), TerminalFailureState::new());
    assert_eq!(collect_response_text(Box::new(stream)).await.unwrap(), "摘要😀完成");
}

/// A failed summary must not expose its partial body as a successful generated result.
#[tokio::test]
async fn summary_collector_reports_the_original_failure() {
    let failure = TerminalFailureState::new();
    failure.set("upstream disconnected".into());
    let stream = with_ordered_event_channel_and_failure(
        VecStream::new(vec![ResponseStreamItem::Chunk("partial".into())]),
        mutable_shared_stream(0), failure,
    );
    assert_eq!(collect_response_text(Box::new(stream)).await,
        Err(AiServiceError::RequestFailed("upstream disconnected".into())));
}
