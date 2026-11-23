use operit_host_api::{HostError, HostResult, WebVisitHost, WebVisitRequest, WebVisitResult};

#[derive(Clone, Debug, Default)]
pub struct AndroidWebVisitHost;

impl AndroidWebVisitHost {
    /// Creates the Android process-owned browser visit capability.
    pub fn new() -> Self {
        Self
    }
}

impl WebVisitHost for AndroidWebVisitHost {
    /// Executes through the Java application Host without involving Flutter.
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult> {
        let encoded = serde_json::json!({
            "url": request.url,
            "headers": request.headers.into_iter().map(|(name, value)| {
                serde_json::json!({"name": name, "value": value})
            }).collect::<Vec<_>>(),
            "userAgent": request.userAgent,
            "includeImageLinks": request.includeImageLinks,
        });
        decode_response(&execute(encoded.to_string())?)
    }
}

/// Calls the registered process Host on the current Core worker.
#[cfg(target_os = "android")]
fn execute(request: String) -> HostResult<String> {
    use jni::objects::{JObject, JString, JValue};
    let bridge = crate::secret_store::androidHostSecretStoreBridge()?;
    let mut env =
        crate::attachCurrentThread(&bridge.vm).map_err(|e| HostError::new(e.to_string()))?;
    let encoded = JObject::from(
        env.new_string(request)
            .map_err(|e| HostError::new(e.to_string()))?,
    );
    let result = env.call_method(
        bridge.host.as_obj(),
        "webVisitOperation",
        "(Ljava/lang/String;)Ljava/lang/String;",
        &[JValue::Object(&encoded)],
    );
    let value = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = env.exception_clear();
            return Err(HostError::new(format!(
                "Android web visit JNI call failed: {error}"
            )));
        }
    };
    let object = value.l().map_err(|e| HostError::new(e.to_string()))?;
    if object.is_null() {
        return Err(HostError::new("Android web visit returned null"));
    }
    let text: String = env
        .get_string(&JString::from(object))
        .map_err(|e| HostError::new(e.to_string()))?
        .into();
    Ok(text)
}

/// Rejects Android execution on non-Android hosts explicitly.
#[cfg(not(target_os = "android"))]
fn execute(_request: String) -> HostResult<String> {
    Err(HostError::new(
        "Android web visits require the Android application Host",
    ))
}

/// Validates the terminal browser response before exposing it to shared tools.
fn decode_response(encoded: &str) -> HostResult<WebVisitResult> {
    use serde_json::Value;
    let response: Value =
        serde_json::from_str(encoded).map_err(|e| HostError::new(e.to_string()))?;
    if response.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(HostError::new(
            response
                .get("error")
                .and_then(Value::as_str)
                .ok_or_else(|| HostError::new("Android web visit error is missing"))?,
        ));
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResultData {
        url: String,
        title: String,
        content: String,
        metadata: std::collections::BTreeMap<String, String>,
        links: Vec<Link>,
        image_links: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct Link {
        url: String,
        text: String,
    }
    let data: ResultData = serde_json::from_value(
        response
            .get("result")
            .cloned()
            .ok_or_else(|| HostError::new("Android web visit result is missing"))?,
    )
    .map_err(|e| HostError::new(e.to_string()))?;
    Ok(WebVisitResult {
        url: data.url,
        title: data.title,
        content: data.content,
        metadata: data.metadata.into_iter().collect(),
        links: data
            .links
            .into_iter()
            .map(|link| operit_host_api::WebVisitLinkData {
                url: link.url,
                text: link.text,
            })
            .collect(),
        imageLinks: data.image_links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keeps transport failures and incomplete terminal envelopes from becoming empty success results.
    #[test]
    fn rejects_failed_and_malformed_native_results() {
        assert_eq!(
            decode_response(r#"{"success":false,"error":"HTTP 403 Forbidden"}"#)
                .unwrap_err()
                .to_string(),
            "HTTP 403 Forbidden"
        );
        assert!(decode_response(r#"{"success":true}"#).is_err());
        assert!(decode_response(r#"{"success":true,"result":{"content":"partial"}}"#).is_err());
        assert!(decode_response("not JSON").is_err());
    }

    /// Preserves the public webpage result schema across the Java JNI boundary.
    #[test]
    fn preserves_native_dom_results() {
        let result = decode_response(
            r#"{"success":true,"result":{
            "url":"https://example.test/final","title":"Page","content":"Actual DOM",
            "metadata":{"description":"Summary"},
            "links":[{"url":"https://example.test/next","text":"Next"}],
            "imageLinks":["https://example.test/image.png"]}}"#,
        )
        .unwrap();
        assert_eq!(result.content, "Actual DOM");
        assert_eq!(result.links[0].text, "Next");
        assert_eq!(
            result.metadata,
            vec![("description".into(), "Summary".into())]
        );
        assert_eq!(result.imageLinks, vec!["https://example.test/image.png"]);
    }
}
