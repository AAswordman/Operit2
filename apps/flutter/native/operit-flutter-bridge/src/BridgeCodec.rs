use operit_link::{
    CoreCallRequest, CoreEvent, CoreEventKind, CoreLinkError, CorePushItem, CorePushRequest,
    CoreWatchRequest,
};
use serde::Serialize;

/// Decodes one compact CoreProxy request into the shared Link model.
pub(crate) fn decode_native_call_request(
    request_bytes: &[u8],
) -> Result<CoreCallRequest, CoreLinkError> {
    let (request_id, object_id, method_name, args): (
        String,
        String,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact core request: {error}"),
        )
    })?;
    Ok(CoreCallRequest::new(
        request_id,
        object_id,
        method_name,
        args,
    ))
}

/// Decodes one compact CoreProxy push-open request.
pub(crate) fn decode_native_push_open_request(
    request_bytes: &[u8],
) -> Result<CorePushRequest, CoreLinkError> {
    let (request_id, object_id, method_name, args): (
        String,
        String,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact push-open request: {error}"),
        )
    })?;
    Ok(CorePushRequest::new(request_id, object_id, method_name).withArgs(args))
}

/// Decodes one compact CoreProxy push item.
pub(crate) fn decode_native_push_item(request_bytes: &[u8]) -> Result<CorePushItem, CoreLinkError> {
    let (push_id, sequence, args): (String, u64, operit_link::CoreValue) =
        operit_link::decodeLink(request_bytes).map_err(|error| {
            CoreLinkError::new(
                "flutter-bridge-invalid-request",
                format!("invalid compact push item: {error}"),
            )
        })?;
    Ok(CorePushItem {
        pushId: push_id,
        sequence,
        args,
    })
}

/// Decodes one compact CoreProxy watch snapshot request.
pub(crate) fn decode_native_watch_snapshot_request(
    request_bytes: &[u8],
) -> Result<CoreWatchRequest, CoreLinkError> {
    let (request_id, object_id, property_name, args): (
        String,
        String,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact watch snapshot request: {error}"),
        )
    })?;
    Ok(CoreWatchRequest::new(
        request_id,
        object_id,
        property_name,
        args,
    ))
}

/// Decodes one compact CoreProxy watch stream open request.
pub(crate) fn decode_native_watch_stream_request(
    request_bytes: &[u8],
) -> Result<(String, CoreWatchRequest), CoreLinkError> {
    let (subscription_id, request_id, object_id, property_name, args): (
        String,
        String,
        String,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact watch stream request: {error}"),
        )
    })?;
    Ok((
        subscription_id,
        CoreWatchRequest::new(request_id, object_id, property_name, args),
    ))
}

/// Encodes one compact CoreProxy result without a field-name map.
pub(crate) fn native_result_vec<T>(result: Result<T, CoreLinkError>) -> Vec<u8>
where
    T: Serialize,
{
    let mut bytes = Vec::new();
    native_result_into(result, &mut bytes);
    bytes
}

/// Encodes a result directly after any header already present in the destination.
fn native_result_into<T: Serialize>(result: Result<T, CoreLinkError>, bytes: &mut Vec<u8>) {
    match result {
        Ok(value) => operit_link::codec::appendLink((0u8, value), bytes)
            .expect("compact native success response must encode"),
        Err(error) => operit_link::codec::appendLink(
            (
                1u8,
                error.code,
                error.message,
                error.details,
                error
                    .location
                    .map(|location| (location.file, location.line, location.column)),
                error.backtrace,
            ),
            bytes,
        )
        .expect("compact native error response must encode"),
    }
}

/// Encodes one compact CoreProxy error result.
pub(crate) fn native_result_error_vec(code: &str, message: impl Into<String>) -> Vec<u8> {
    native_result_vec(Err::<(), _>(CoreLinkError::new(code, message.into())))
}

/// Encodes one compact CoreProxy watch channel event.
pub(crate) fn native_watch_event_vec(subscription_id: &str, event: CoreEvent) -> Vec<u8> {
    let mut bytes = Vec::new();
    native_watch_event_into(subscription_id, event, &mut bytes);
    bytes
}

/// Serializes a watch event once into its final destination, retaining profiling.
fn native_watch_event_into(subscription_id: &str, event: CoreEvent, bytes: &mut Vec<u8>) {
    let profile = std::env::var_os("OPERIT_PROFILE_COMPOSE_STAGES").is_some();
    let started = profile.then(std::time::Instant::now);
    operit_link::codec::appendLink((subscription_id, native_watch_event_payload(event)), bytes)
        .expect("compact native watch event must encode");
    if let Some(started) = started {
        let line = format!("event=compose_pipeline_stage stage=rust-messagepack durationMs={:.3} wallMicros={} bytes={}\n",
            started.elapsed().as_secs_f64() * 1000.0,
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_micros(), bytes.len());
        let _ = std::io::Write::write_all(&mut std::io::stderr().lock(), line.as_bytes());
    }
}

/// Owns ABI-v2 control metadata and retained shared binary allocations.
pub(crate) struct NativeFrame(Vec<u8>, Vec<operit_link::CoreBytes>);

impl NativeFrame {
    /// Creates the unchanged nine-byte header for one delivery.
    pub(crate) fn empty(kind: u8, request: i64) -> Self {
        let mut bytes = Vec::with_capacity(128);
        bytes.push(kind);
        bytes.extend_from_slice(&request.to_le_bytes());
        Self(bytes, Vec::new())
    }

    /// Encodes one reply directly into the allocation transferred to Dart.
    pub(crate) fn result<T: Serialize>(
        kind: u8,
        request: i64,
        result: Result<T, CoreLinkError>,
    ) -> Self {
        let mut frame = Self::empty(kind, request);
        let (_, buffers) = operit_link::codec::captureLinkBuffers(|| native_result_into(result, &mut frame.0));
        frame.1 = buffers;
        frame
    }

    /// Encodes an explicit operation failure into its final reply frame.
    pub(crate) fn error(kind: u8, request: i64, code: &str, message: impl Into<String>) -> Self {
        Self::result(
            kind,
            request,
            Err::<(), _>(CoreLinkError::new(code, message.into())),
        )
    }

    /// Encodes one watch event into its final shared allocation.
    pub(crate) fn watch(request: i64, subscription_id: &str, event: CoreEvent) -> Self {
        let mut frame = Self::empty(1, request);
        let (_, buffers) = operit_link::codec::captureLinkBuffers(|| native_watch_event_into(subscription_id, event, &mut frame.0));
        frame.1 = buffers;
        frame
    }

    /// Transfers the complete allocation without moving its payload bytes.
    pub(crate) fn into_parts(self) -> (Vec<u8>, Vec<operit_link::CoreBytes>) {
        (self.0, self.1)
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;

    /// Verifies direct frame encoding preserves every ABI header and result byte.
    #[test]
    fn direct_frames_preserve_success_error_and_watch_bytes() {
        let value = operit_link::CoreValue::Bytes(vec![0xa5; 1024 * 1024].into());
        let (frame, buffers) = NativeFrame::result(0, -17, Ok(&value)).into_parts();
        assert_eq!(frame[0], 0);
        assert_eq!(&frame[1..9], &(-17i64).to_le_bytes());
        assert!(frame.len() < 64);
        let decoded: (u8, operit_link::CoreValue) = operit_link::codec::withLinkBuffers(buffers.clone().into(), || operit_link::decodeLink(&frame[9..]).unwrap());
        assert_eq!(decoded, (0, value.clone()));
        let operit_link::CoreValue::Bytes(original) = &value else { unreachable!() };
        assert_eq!(buffers[0].as_ptr(), original.as_ptr());
        let (error, buffers) = NativeFrame::error(2, 23, "TEST", "failure").into_parts();
        assert!(buffers.is_empty());
        assert_eq!(error[0], 2);
        assert_eq!(&error[1..9], &23i64.to_le_bytes());
        assert_eq!(&error[9..], native_result_error_vec("TEST", "failure"));
        let event = CoreEvent {
            requestId: None,
            target: "test".into(),
            propertyName: "bytes".into(),
            kind: CoreEventKind::Changed,
            value,
        };
        let (frame, buffers) = NativeFrame::watch(31, "watch", event.clone()).into_parts();
        assert_eq!(frame[0], 1);
        assert_eq!(&frame[1..9], &31i64.to_le_bytes());
        let decoded: (String, (Option<String>, String, String, String, operit_link::CoreValue)) =
            operit_link::codec::withLinkBuffers(buffers.into(), || operit_link::decodeLink(&frame[9..]).unwrap());
        assert_eq!(decoded.0, "watch");
        assert_eq!(decoded.1.4, event.value);
        assert_eq!(NativeFrame::empty(3, 31).into_parts().0.len(), 9);
    }
}

/// Converts one CoreProxy event into its compact native payload tuple.
pub(crate) fn native_watch_event_payload(
    event: CoreEvent,
) -> (
    Option<String>,
    String,
    String,
    &'static str,
    operit_link::CoreValue,
) {
    let CoreEvent {
        requestId,
        target,
        propertyName,
        kind,
        value,
    } = event;
    (
        requestId.map(|request_id| request_id.0),
        target,
        propertyName,
        native_event_kind_name(kind),
        value,
    )
}

/// Converts one native CoreProxy event kind into its direct wire literal.
fn native_event_kind_name(kind: CoreEventKind) -> &'static str {
    match kind {
        CoreEventKind::Snapshot => "Snapshot",
        CoreEventKind::Changed => "Changed",
        CoreEventKind::Delta => "Delta",
        CoreEventKind::Completed => "Completed",
    }
}
