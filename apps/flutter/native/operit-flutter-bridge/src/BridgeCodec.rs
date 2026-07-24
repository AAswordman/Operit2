use operit_link::{
    CoreCallRequest, CoreEvent, CoreEventKind, CoreLinkError, CorePushItem, CorePushRequest,
    CoreWatchRequest,
};
use serde::Serialize;

/// Decodes one compact CoreProxy request into the shared Link model.
pub(crate) fn decode_native_call_request(
    request_bytes: &[u8],
) -> Result<CoreCallRequest, CoreLinkError> {
    let (request_id, target_segments, method_name, args): (
        String,
        Vec<String>,
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
        operit_link::CoreObjectPath {
            segments: target_segments,
        },
        method_name,
        args,
    ))
}

/// Decodes one compact CoreProxy push-open request.
pub(crate) fn decode_native_push_open_request(
    request_bytes: &[u8],
) -> Result<CorePushRequest, CoreLinkError> {
    let (request_id, target_segments, method_name): (String, Vec<String>, String) =
        operit_link::decodeLink(request_bytes).map_err(|error| {
            CoreLinkError::new(
                "flutter-bridge-invalid-request",
                format!("invalid compact push-open request: {error}"),
            )
        })?;
    Ok(CorePushRequest::new(
        request_id,
        operit_link::CoreObjectPath {
            segments: target_segments,
        },
        method_name,
    ))
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
    let (request_id, target_segments, property_name, args): (
        String,
        Vec<String>,
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
        operit_link::CoreObjectPath {
            segments: target_segments,
        },
        property_name,
        args,
    ))
}

/// Decodes one compact CoreProxy watch stream open request.
pub(crate) fn decode_native_watch_stream_request(
    request_bytes: &[u8],
) -> Result<(String, CoreWatchRequest), CoreLinkError> {
    let (subscription_id, request_id, target_segments, property_name, args): (
        String,
        String,
        Vec<String>,
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
        CoreWatchRequest::new(
            request_id,
            operit_link::CoreObjectPath {
                segments: target_segments,
            },
            property_name,
            args,
        ),
    ))
}

/// Encodes one compact CoreProxy result without a field-name map.
pub(crate) fn native_result_vec<T>(result: Result<T, CoreLinkError>) -> Vec<u8>
where
    T: Serialize,
{
    match result {
        Ok(value) => operit_link::encodeLink((0u8, value))
            .expect("compact native success response must encode"),
        Err(error) => operit_link::encodeLink((
            1u8,
            error.code,
            error.message,
            error.details,
            error
                .location
                .map(|location| (location.file, location.line, location.column)),
            error.backtrace,
        ))
        .expect("compact native error response must encode"),
    }
}

/// Encodes one compact CoreProxy error result.
pub(crate) fn native_result_error_vec(code: &str, message: impl Into<String>) -> Vec<u8> {
    native_result_vec(Err::<(), _>(CoreLinkError::new(code, message.into())))
}

/// Encodes one compact CoreProxy watch channel event.
pub(crate) fn native_watch_event_vec(subscription_id: &str, event: CoreEvent) -> Vec<u8> {
    operit_link::encodeLink((subscription_id, native_watch_event_payload(event)))
        .expect("compact native watch event must encode")
}

/// Converts one CoreProxy event into its compact native payload tuple.
pub(crate) fn native_watch_event_payload(
    event: CoreEvent,
) -> (
    Option<String>,
    Vec<String>,
    String,
    &'static str,
    operit_link::CoreValue,
) {
    let CoreEvent {
        requestId,
        targetPath,
        propertyName,
        kind,
        value,
    } = event;
    (
        requestId.map(|request_id| request_id.0),
        targetPath.segments,
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
        CoreEventKind::Completed => "Completed",
    }
}
