//! Optional provider adapter. No HTTP client or model runtime is linked here;
//! the manufacturer supplies its existing streaming HTTP Host.
use crate::agent::{AgentError, AgentFuture, Backend, Request, Step};
use operit_host_api::{HttpHost, HttpRequestData};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

pub struct HttpAgentConfig {
    /// Full OpenAI-compatible chat completions URL.
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub max_response_bytes: usize,
    pub max_request_bytes: usize,
    pub timeout_seconds: u64,
}

pub struct HttpBackend {
    host: Arc<dyn HttpHost>,
    config: HttpAgentConfig,
}
impl HttpBackend {
    pub fn new(host: Arc<dyn HttpHost>, config: HttpAgentConfig) -> Result<Self, AgentError> {
        if !(config.endpoint.starts_with("https://") || config.endpoint.starts_with("http://"))
            || config.endpoint.len() > 1024
            || config.model.is_empty()
            || config.model.len() > 128
            || config.api_key.len() > 512
            || config.api_key.contains(['\r', '\n'])
            || config.max_response_bytes == 0
            || config.max_request_bytes == 0
            || config.timeout_seconds == 0
        {
            return Err(AgentError::InvalidLimits);
        }
        Ok(Self { host, config })
    }
}
struct StreamLease {
    host: Arc<dyn HttpHost>,
    id: String,
}
impl Drop for StreamLease {
    fn drop(&mut self) {
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}
#[derive(Default)]
struct Response {
    status: i32,
    bytes: Vec<u8>,
}
impl Backend for HttpBackend {
    fn next<'a>(&'a self, request: Request<'a>) -> AgentFuture<'a> {
        Box::pin(async move {
            let mut messages = vec![
                json!({"role":"system","content":request.instruction}),
                json!({"role":"user","content":request.prompt}),
            ];
            for (index, observation) in request.observations.iter().enumerate() {
                let id = format!("device-tool-{index}");
                messages.push(
                    json!({"role":"assistant","tool_calls":[{"id":id,"type":"function",
                    "function":{"name":observation.tool,"arguments":observation.arguments}}]}),
                );
                messages
                    .push(json!({"role":"tool","tool_call_id":id,"content":observation.result}));
            }
            let mut body = json!({"model":self.config.model,"messages":messages,"stream":false,"max_tokens":256});
            if !request.tools.is_empty() {
                let tools = request.tools.iter().map(|tool| {
                    let schema: Value = serde_json::from_str(tool.input_schema)
                        .map_err(|e| AgentError::Tool(e.to_string()))?;
                    Ok(json!({"type":"function","function":{"name":tool.name,"description":tool.description,"parameters":schema}}))
                }).collect::<Result<Vec<_>, AgentError>>()?;
                body["tools"] = tools.into();
                body["parallel_tool_calls"] = false.into();
            }
            let body = serde_json::to_vec(&body).map_err(|e| AgentError::Backend(e.to_string()))?;
            if body.len() > self.config.max_request_bytes {
                return Err(AgentError::InputTooLarge);
            }
            let mut headers = vec![("Content-Type".into(), "application/json".into())];
            if !self.config.api_key.is_empty() {
                headers.push((
                    "Authorization".into(),
                    format!("Bearer {}", self.config.api_key),
                ));
            }
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let lease = StreamLease {
                host: self.host.clone(),
                id: format!("edge-agent-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)),
            };
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let sender = Arc::new(Mutex::new(Some(sender)));
            let response = Arc::new(Mutex::new(Response::default()));
            let head = response.clone();
            let chunk = response.clone();
            let close = response.clone();
            let limit = self.config.max_response_bytes;
            let overflow = sender.clone();
            self.host
                .openHttpResponseStream(
                    lease.id.clone(),
                    HttpRequestData {
                        url: self.config.endpoint.clone(),
                        method: "POST".into(),
                        headers,
                        body,
                        formFields: vec![],
                        fileParts: vec![],
                        connectTimeoutSeconds: self.config.timeout_seconds.min(10),
                        readTimeoutSeconds: self.config.timeout_seconds,
                        followRedirects: false,
                        ignoreSsl: false,
                        proxyHost: String::new(),
                        proxyPort: 0,
                    },
                    Arc::new(move |value| {
                        head.lock().unwrap_or_else(|p| p.into_inner()).status = value.statusCode;
                    }),
                    Arc::new(move |bytes| {
                        let mut response = chunk.lock().unwrap_or_else(|p| p.into_inner());
                        if response.bytes.len().saturating_add(bytes.len()) > limit {
                            if let Some(sender) =
                                overflow.lock().unwrap_or_else(|p| p.into_inner()).take()
                            {
                                let _ = sender.send(Err(AgentError::OutputTooLarge));
                            }
                        } else {
                            response.bytes.extend_from_slice(&bytes);
                        }
                    }),
                    Arc::new(move |result| {
                        let response =
                            std::mem::take(&mut *close.lock().unwrap_or_else(|p| p.into_inner()));
                        let result = result.map_err(AgentError::Backend).and_then(|_| {
                            if !(200..300).contains(&response.status) {
                                return Err(AgentError::Backend(format!(
                                    "HTTP {}",
                                    response.status
                                )));
                            }
                            Ok(response.bytes)
                        });
                        if let Some(sender) =
                            sender.lock().unwrap_or_else(|p| p.into_inner()).take()
                        {
                            let _ = sender.send(result);
                        }
                    }),
                )
                .map_err(|e| AgentError::Backend(e.to_string()))?;
            let bytes = tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_seconds),
                receiver,
            )
            .await
            .map_err(|_| AgentError::Backend("Agent request timed out".into()))?
            .map_err(|_| AgentError::Backend("Agent stream closed without a result".into()))??;
            drop(lease);
            decode_step(&bytes)
        })
    }
}
fn decode_step(bytes: &[u8]) -> Result<Step, AgentError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| AgentError::Backend(e.to_string()))?;
    let message = &value["choices"][0]["message"];
    if let Some(calls) = message["tool_calls"]
        .as_array()
        .filter(|calls| !calls.is_empty())
    {
        if calls.len() != 1 {
            return Err(AgentError::Backend(
                "Expected one tool call per step".into(),
            ));
        }
        let function = &calls[0]["function"];
        let tool = function["name"]
            .as_str()
            .ok_or_else(|| AgentError::Backend("Missing tool name".into()))?;
        let arguments = function["arguments"]
            .as_str()
            .ok_or_else(|| AgentError::Backend("Missing tool arguments".into()))?;
        return Ok(Step::Call {
            tool: tool.into(),
            arguments: arguments.into(),
        });
    }
    message["content"]
        .as_str()
        .map(|text| Step::Reply(text.into()))
        .ok_or_else(|| AgentError::Backend("Missing assistant content".into()))
}
