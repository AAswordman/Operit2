//! Exercises byte-stream buffering independently of Host callback fragmentation.
use super::Inbox;
use std::time::Duration;

/// Collects every accepted byte through a normal end of stream.
async fn drain(inbox: &Inbox) -> Vec<u8> {
    let mut bytes = Vec::new();
    while let Some(chunk) = inbox.read().await.unwrap() {
        bytes.extend(chunk);
    }
    bytes
}

/// A legal frame must survive more than 32 callbacks before its reader runs.
#[tokio::test]
async fn fragmented_frame_survives_callback_burst() {
    let payload = (0..512 * 1024).map(|index| index as u8).collect::<Vec<_>>();
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend(payload);
    let inbox = Inbox::new(frame.len() - 4);
    for chunk in frame.chunks(1024) {
        inbox.put(chunk.to_vec());
    }
    inbox.finish(Ok(()));
    assert_eq!(drain(&inbox).await, frame);
}

/// Callback boundaries cannot change byte order across consecutive frames.
#[tokio::test]
async fn small_frames_share_the_byte_budget() {
    let inbox = Inbox::new(1024);
    let bytes = (0..128u32).flat_map(u32::to_be_bytes).collect::<Vec<_>>();
    for byte in &bytes {
        inbox.put(vec![*byte]);
    }
    inbox.finish(Ok(()));
    assert_eq!(drain(&inbox).await, bytes);
}

/// Several accepted chunks must not collectively exceed the byte ceiling.
#[tokio::test]
async fn accumulated_bytes_are_bounded() {
    let inbox = Inbox::new(16);
    inbox.put(vec![1; 12]);
    assert!(inbox.status().is_ok());
    inbox.put(vec![2; 9]);
    assert!(inbox.status().is_err());
    assert!(inbox.read().await.is_err());
}

/// One oversized callback must terminate the stream before exposing its bytes.
#[tokio::test]
async fn oversized_callback_is_rejected() {
    let inbox = Inbox::new(16);
    inbox.put(vec![1; 21]);
    assert!(inbox.read().await.is_err());
}

/// Consumed bytes release capacity for subsequent Host callbacks.
#[tokio::test]
async fn reading_releases_the_byte_budget() {
    let inbox = Inbox::new(16);
    for value in 0..64u8 {
        inbox.put(vec![value; 20]);
        assert_eq!(inbox.read().await.unwrap(), Some(vec![value; 20]));
    }
    inbox.finish(Ok(()));
    assert_eq!(inbox.read().await.unwrap(), None);
}

/// The first terminal error remains visible after later close notifications.
#[tokio::test]
async fn failure_precedes_buffered_bytes_and_clean_close() {
    let inbox = Inbox::new(16);
    inbox.put(vec![1; 8]);
    inbox.finish(Err("transport failed".into()));
    inbox.finish(Ok(()));
    assert_eq!(inbox.read().await, Err("transport failed".into()));
}

/// A reader waiting for bytes must wake when its transport ends.
#[tokio::test]
async fn closing_wakes_a_waiting_reader() {
    let inbox = Inbox::new(16);
    let (result, ()) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(1), inbox.read()),
        async {
            tokio::task::yield_now().await;
            inbox.finish(Ok(()));
        }
    );
    assert_eq!(result.unwrap().unwrap(), None);
}

/// A reader waiting for data must wake on the next Host callback.
#[tokio::test]
async fn incoming_bytes_wake_a_waiting_reader() {
    let inbox = Inbox::new(16);
    let (result, ()) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(1), inbox.read()),
        async {
            tokio::task::yield_now().await;
            inbox.put(vec![7]);
        }
    );
    assert_eq!(result.unwrap().unwrap(), Some(vec![7]));
}
