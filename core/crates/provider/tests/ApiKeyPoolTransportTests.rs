use super::*;
use crate::chat::llmprovider::ApiKeyProvider::ApiKeyPoolState;
#[path = "ApiKeyPoolTestSupport.rs"]
mod support;
use support::{fixture, ScriptHost};

/// Extracts credentials using the protocol's declared authentication location.
fn keys(host: &ScriptHost, auth: &KeyAuthentication) -> Vec<String> {
    match auth {
        KeyAuthentication::Bearer => host.header_values("authorization"),
        KeyAuthentication::Header(name) => host.header_values(name),
        KeyAuthentication::Query(name) => host.query_values(name),
    }
}

/// Uses the exact same request path for fixed keys and respects explicit authentication headers.
#[tokio::test]
async fn fixed_getter_uses_shared_transport_and_encodes_credentials_once() {
    let host = ScriptHost::new(&[(200, false), (401, false), (200, false)]);
    post("https://fixture.invalid")
        .key_getter("Bearer fixed-key".into(), KeyAuthentication::Bearer)
        .send_with_host(host.clone())
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let response = post("https://fixture.invalid")
        .key_getter("ignored-key".into(), KeyAuthentication::Bearer)
        .header(http::header::AUTHORIZATION, "Token explicit")
        .send_with_host(host.clone())
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 401);
    post("https://fixture.invalid?alt=sse")
        .key_getter(
            "key&with=delimiters".into(),
            KeyAuthentication::Query("key".into()),
        )
        .send_with_host(host.clone())
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let requests = host.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[0]
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .unwrap()
            .1,
        "Bearer fixed-key"
    );
    assert_eq!(
        requests[1]
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .unwrap()
            .1,
        "Token explicit"
    );
    let url = url::Url::parse(&requests[2].url).unwrap();
    assert_eq!(
        url.query_pairs().find(|(name, _)| name == "key").unwrap().1,
        "key&with=delimiters"
    );
    assert_eq!(
        url.query_pairs().find(|(name, _)| name == "alt").unwrap().1,
        "sse"
    );
}

/// Uses the same healthy credential until a typed HTTP failure requires switching.
#[tokio::test]
async fn key_pool_affinity_and_failover_cover_all_authentication_protocols() {
    for auth in [
        KeyAuthentication::Bearer,
        KeyAuthentication::Header("x-api-key".into()),
        KeyAuthentication::Query("key".into()),
    ] {
        let (session, state) = fixture();
        let host = ScriptHost::new(&[
            (200, false),
            (200, false),
            (401, false),
            (429, false),
            (200, false),
            (200, false),
        ]);
        for _ in 0..4 {
            post("https://fixture.invalid/v1?alt=sse")
                .key_getter(session.clone(), auth.clone())
                .send_with_host(host.clone())
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
        }
        let expected = if matches!(auth, KeyAuthentication::Bearer) {
            vec![
                "Bearer key-0",
                "Bearer key-0",
                "Bearer key-0",
                "Bearer key-1",
                "Bearer key-2",
                "Bearer key-2",
            ]
        } else {
            vec!["key-0", "key-0", "key-0", "key-1", "key-2", "key-2"]
        };
        assert_eq!(keys(&host, &auth), expected);
        assert_eq!(
            state
                .lock()
                .unwrap()
                .0
                .apiKeyPool
                .iter()
                .map(|key| key.errorCount)
                .sum::<i64>(),
            2
        );
    }
}

/// Keeps request errors and cancellation from poisoning healthy credentials, while disconnects cool them.
#[tokio::test]
async fn key_pool_parameter_error_drop_and_stream_disconnect_have_distinct_outcomes() {
    let (session, state) = fixture();
    let host = ScriptHost::new(&[(400, false), (200, false), (200, true), (200, false)]);
    let auth = KeyAuthentication::Bearer;
    let send = || {
        post("https://fixture.invalid")
            .key_getter(session.clone(), auth.clone())
            .send_with_host(host.clone())
    };
    assert_eq!(send().await.unwrap().status().as_u16(), 400);
    drop(send().await.unwrap());
    assert!(send().await.unwrap().text().await.is_err());
    send().await.unwrap().text().await.unwrap();
    assert_eq!(
        keys(&host, &auth),
        [
            "Bearer key-0",
            "Bearer key-0",
            "Bearer key-0",
            "Bearer key-1"
        ]
    );
    assert_eq!(state.lock().unwrap().0.apiKeyPool[0].errorCount, 1);
}

/// Stops after exhausting distinct credentials and retains the last upstream error response.
#[tokio::test]
async fn key_pool_exhaustion_is_bounded_and_preserves_http_error() {
    let (session, _) = fixture();
    let host = ScriptHost::new(&[(401, false), (403, false), (500, false)]);
    let response = post("https://fixture.invalid")
        .key_getter(session, KeyAuthentication::Bearer)
        .send_with_host(host.clone())
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 500);
    assert_eq!(host.requests.lock().unwrap().len(), 3);
}

/// Honors typed Retry-After metadata without guessing from response text.
#[test]
fn key_pool_status_classification_respects_retry_after() {
    assert_eq!(
        key_outcome(429, &[("retry-after".into(), "120".into())], 0),
        Some(ApiKeyOutcome::RateLimited {
            retry_after_ms: Some(120000)
        })
    );
    assert_eq!(
        key_outcome(
            429,
            &[("Retry-After".into(), "Wed, 21 Oct 2015 07:28:00 GMT".into())],
            1445412420000
        ),
        Some(ApiKeyOutcome::RateLimited {
            retry_after_ms: Some(60000)
        })
    );
    for status in [400, 404, 413, 422] {
        assert_eq!(key_outcome(status, &[], 0), None);
    }
}

/// Shares cooldowns and bounded failover with synchronous model discovery, including transport errors.
#[test]
fn key_pool_discovery_switches_after_transport_and_quota_failures() {
    let (session, state) = fixture();
    let host = ScriptHost::new(&[(200, true), (402, false), (200, false), (200, false)]);
    let request = get("https://fixture.invalid/v1/models").request;
    for _ in 0..2 {
        let response = execute_with_key_getter(
            host.clone(),
            request.clone(),
            session.clone(),
            KeyAuthentication::Bearer,
        )
        .unwrap();
        assert_eq!(response.statusCode, 200);
    }
    assert_eq!(
        keys(&host, &KeyAuthentication::Bearer),
        [
            "Bearer key-0",
            "Bearer key-1",
            "Bearer key-2",
            "Bearer key-2",
        ]
    );
    assert_eq!(state.lock().unwrap().0.apiKeyPool[0].errorCount, 1);
    assert_eq!(state.lock().unwrap().0.apiKeyPool[1].errorCount, 1);
}
