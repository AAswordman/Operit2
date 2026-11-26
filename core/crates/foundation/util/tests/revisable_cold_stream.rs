use operit_util::stream::HotStream::mutable_shared_stream;
use operit_util::stream::RevisableTextStream::{
    collect_revised_text, with_event_channel_shared, with_ordered_event_channel_and_failure,
    DelegatingRevisableSharedTextStream, ResponseStreamItem, RevisableTextStream,
    TerminalFailureState, TextStreamEvent, TextStreamEventType,
};
use operit_util::stream::TextStreamRevisionTracker::TextStreamRevisionTracker;

/// Cold provider wrappers preserve partial-attempt rollback, subsequent text and terminal failure.
#[tokio::test]
async fn ordered_cold_wrapper_preserves_retry_revisions_and_failure() {
    use operit_util::stream::Stream::FnStream;
    let failure = TerminalFailureState::new();
    let producerFailure = failure.clone();
    let stream = FnStream::new(move |emit| {
        emit(ResponseStreamItem::Revision(TextStreamEvent { event_type: TextStreamEventType::Savepoint, id: "attempt".into() }));
        emit(ResponseStreamItem::Chunk("<tool_partial>discard".into()));
        emit(ResponseStreamItem::Revision(TextStreamEvent { event_type: TextStreamEventType::Rollback, id: "attempt".into() }));
        emit(ResponseStreamItem::Chunk("正文".into()));
        producerFailure.set("connection lost after output".into());
        Box::pin(async {})
    });
    let mut wrapped = with_ordered_event_channel_and_failure(stream, mutable_shared_stream(0), failure);
    let mut tracker = TextStreamRevisionTracker::new("");
    wrapped.collect_ordered(&mut |item| match item {
        ResponseStreamItem::Chunk(chunk) => { tracker.append(&chunk); }
        ResponseStreamItem::Revision(event) => match event.event_type {
            TextStreamEventType::Savepoint => tracker.savepoint(&event.id),
            TextStreamEventType::Rollback => { tracker.rollback(&event.id).unwrap(); }
        },
    }).await;
    assert_eq!(tracker.current_content(), "正文");
    assert_eq!(wrapped.terminal_failure().as_deref(), Some("connection lost after output"));
}

/// Retries discard only the failed request while retaining prior tools and Unicode text.
#[tokio::test]
async fn final_text_retains_completed_rounds_across_multiple_retries() {
    use operit_util::stream::Stream::VecStream;
    let revision = |event_type, id: &str| ResponseStreamItem::Revision(TextStreamEvent {
        event_type, id: id.into(),
    });
    let items = vec![
        ResponseStreamItem::Chunk("已完成😀<tool_result_a>saved</tool_result_a>".into()),
        revision(TextStreamEventType::Savepoint, "round-2"),
        ResponseStreamItem::Chunk("<think>discard".into()),
        revision(TextStreamEventType::Rollback, "round-2"),
        ResponseStreamItem::Chunk("<tool_partial name=\"write_file\">".into()),
        revision(TextStreamEventType::Rollback, "round-2"),
        ResponseStreamItem::Chunk("最终摘要".into()),
        revision(TextStreamEventType::Savepoint, "round-3"),
        ResponseStreamItem::Chunk(" obsolete".into()),
        revision(TextStreamEventType::Rollback, "round-3"),
        ResponseStreamItem::Chunk("完成".into()),
    ];
    let mut stream = with_ordered_event_channel_and_failure(
        VecStream::new(items), mutable_shared_stream(0), TerminalFailureState::new(),
    );
    assert_eq!(collect_revised_text(&mut stream).await.unwrap(),
        "已完成😀<tool_result_a>saved</tool_result_a>最终摘要完成");
}

/// Final-text consumers never accept partial text after transport or revision failure.
#[tokio::test]
async fn final_text_rejects_transport_failure_and_unknown_rollback() {
    use operit_util::stream::Stream::VecStream;
    for transport_error in [None, Some("original transport failure")] {
        let failure = TerminalFailureState::new();
        if let Some(error) = transport_error { failure.set(error.into()); }
        let mut stream = with_ordered_event_channel_and_failure(
            VecStream::new(vec![
                ResponseStreamItem::Chunk("partial".into()),
                ResponseStreamItem::Revision(TextStreamEvent {
                    event_type: TextStreamEventType::Rollback, id: "missing".into(),
                }),
            ]), mutable_shared_stream(0), failure,
        );
        let error = collect_revised_text(&mut stream).await.unwrap_err();
        assert_eq!(error, transport_error.unwrap_or("Response rollback savepoint is missing: missing"));
    }
}

/// Plugin providers can finish before the first ordered consumer subscribes.
#[tokio::test]
async fn text_only_shared_adapter_replays_completed_plugin_output() {
    let text = mutable_shared_stream(usize::MAX);
    text.emit("插件".into());
    text.emit("结果😀".into());
    text.close();
    let mut stream = with_event_channel_shared(text, mutable_shared_stream(0));
    assert_eq!(stream.current_render_content(), "插件结果😀");
    assert_eq!(collect_revised_text(&mut stream).await.unwrap(), "插件结果😀");
}

/// Buffered plugin chunks and live chunks cross subscription exactly once.
#[tokio::test]
async fn text_only_shared_adapter_preserves_replay_then_live_output() {
    let text = mutable_shared_stream(usize::MAX);
    text.emit("buffered ".into());
    let producer = text.clone();
    let mut stream = with_event_channel_shared(text, mutable_shared_stream(0));
    let (subscribed, attached) = tokio::sync::oneshot::channel();
    let mut subscribed = Some(subscribed);
    let collect = async {
        let mut chunks = Vec::new();
        stream.collect_ordered(&mut |item| {
            if let ResponseStreamItem::Chunk(chunk) = item { chunks.push(chunk); }
            if let Some(ready) = subscribed.take() { ready.send(()).unwrap(); }
        }).await;
        chunks.concat()
    };
    let drive = async {
        attached.await.unwrap();
        producer.emit("live".into());
        producer.close();
    };
    let (text, ()) = futures_util::future::join(collect, drive).await;
    assert_eq!(text, "buffered live");
}

/// Shared adapters retain chunks emitted through their own producer interface as well.
#[tokio::test]
async fn text_only_shared_producer_retains_its_emitted_chunks() {
    let mut stream = DelegatingRevisableSharedTextStream::new(
        mutable_shared_stream(usize::MAX), mutable_shared_stream(0),
    );
    stream.set_initial_content("prefix ".into());
    stream.emit_chunk("accepted".into());
    stream.close();
    assert_eq!(stream.current_render_content(), "prefix accepted");
    assert_eq!(collect_revised_text(&mut stream).await.unwrap(), "prefix accepted");
}

/// Late subscribers receive active savepoints and apply later retries without losing prior rounds.
#[tokio::test]
async fn late_ordered_subscribers_restore_the_same_accepted_response() {
    let producer = DelegatingRevisableSharedTextStream::new_ordered(
        mutable_shared_stream(usize::MAX), mutable_shared_stream(0),
    );
    producer.set_initial_content("历史😀".into());
    producer.emit_chunk("<tool_result_a>saved</tool_result_a>".into());
    producer.emit_revision(TextStreamEvent { event_type: TextStreamEventType::Savepoint, id: "request".into() });
    producer.emit_chunk("<tool_partial>".into());
    let mut first = producer.clone();
    let mut second = producer.clone();
    let first_collect = collect_revised_text(&mut first);
    let second_collect = collect_revised_text(&mut second);
    tokio::pin!(first_collect, second_collect);
    assert!(futures_util::poll!(&mut first_collect).is_pending());
    assert!(futures_util::poll!(&mut second_collect).is_pending());
    producer.emit_revision(TextStreamEvent { event_type: TextStreamEventType::Rollback, id: "request".into() });
    producer.emit_chunk("完成".into());
    producer.close();
    let expected = "历史😀<tool_result_a>saved</tool_result_a>完成";
    assert_eq!(first_collect.await.unwrap(), expected);
    assert_eq!(second_collect.await.unwrap(), expected);
    let mut after_close = producer.clone();
    assert_eq!(collect_revised_text(&mut after_close).await.unwrap(), expected);
    assert_eq!(producer.current_render_content(), expected);
}
