use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use toolpkg_runtime::{
    errorToolResult, AITool, ExternalToolInvocationBridge, ToolPkgFunctionCall, ToolPkgIpcCall,
    ToolPkgMainHookCall, ToolPkgRuntime, ToolPkgRuntimeOptions, ToolResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct RuntimeServerRequest {
    #[serde(default)]
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RuntimeServerResponse {
    #[serde(default)]
    id: Value,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct RuntimeServerHostToolCallRequest {
    #[serde(rename = "type")]
    messageType: &'static str,
    id: String,
    tool: AITool,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct RuntimeServerHostToolCallResponse {
    #[serde(rename = "type")]
    messageType: String,
    id: String,
    #[serde(default)]
    results: Vec<ToolResult>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct LoadToolPkgFileParams {
    path: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct ReadTextResourceParams {
    containerPackageName: String,
    resourcePath: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct DestroyContextParams {
    contextKey: String,
}

struct RuntimeServerIo {
    stdin: Mutex<io::BufReader<io::Stdin>>,
    stdout: Mutex<io::Stdout>,
    nextHostCallId: AtomicU64,
}

impl RuntimeServerIo {
    fn new() -> Self {
        Self {
            stdin: Mutex::new(io::BufReader::new(io::stdin())),
            stdout: Mutex::new(io::stdout()),
            nextHostCallId: AtomicU64::new(1),
        }
    }

    #[allow(non_snake_case)]
    fn readLine(&self) -> Result<Option<String>, String> {
        let mut line = String::new();
        let bytesRead = self
            .stdin
            .lock()
            .expect("runtime server stdin mutex poisoned")
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytesRead == 0 {
            return Ok(None);
        }
        Ok(Some(line))
    }

    #[allow(non_snake_case)]
    fn writeJson<T>(&self, value: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let responseJson = serde_json::to_string(value).map_err(|error| error.to_string())?;
        let mut stdout = self
            .stdout
            .lock()
            .expect("runtime server stdout mutex poisoned");
        writeln!(stdout, "{responseJson}").map_err(|error| error.to_string())?;
        stdout.flush().map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn nextHostToolCallId(&self) -> String {
        let value = self.nextHostCallId.fetch_add(1, Ordering::SeqCst);
        format!("host-tool-call-{value}")
    }
}

struct RuntimeServerExternalToolBridge {
    io: Arc<RuntimeServerIo>,
}

impl RuntimeServerExternalToolBridge {
    #[allow(non_snake_case)]
    fn invokeHostTool(&self, tool: &AITool) -> Result<Vec<ToolResult>, String> {
        let id = self.io.nextHostToolCallId();
        self.io.writeJson(&RuntimeServerHostToolCallRequest {
            messageType: "hostToolCall",
            id: id.clone(),
            tool: tool.clone(),
        })?;
        let line = self
            .io
            .readLine()?
            .ok_or_else(|| "host tool call response stream closed".to_string())?;
        let response =
            serde_json::from_str::<RuntimeServerHostToolCallResponse>(line.trim())
                .map_err(|error| error.to_string())?;
        if response.messageType != "hostToolCallResult" {
            return Err(format!(
                "unexpected host response type: {}",
                response.messageType
            ));
        }
        if response.id != id {
            return Err(format!(
                "unexpected host response id: expected {id}, got {}",
                response.id
            ));
        }
        if let Some(error) = response.error {
            return Err(error);
        }
        Ok(response.results)
    }
}

impl ExternalToolInvocationBridge for RuntimeServerExternalToolBridge {
    #[allow(non_snake_case)]
    fn invokeTool(&self, tool: &AITool) -> Vec<ToolResult> {
        match self.invokeHostTool(tool) {
            Ok(results) => results,
            Err(error) => vec![errorToolResult(&tool.name, error)],
        }
    }
}

#[allow(non_snake_case)]
pub fn runToolPkgRuntimeServer(languageCode: String) -> Result<(), String> {
    let io = Arc::new(RuntimeServerIo::new());
    let runtime = ToolPkgRuntime::new(ToolPkgRuntimeOptions { languageCode });
    runtime.setExternalToolBridge(Some(Arc::new(RuntimeServerExternalToolBridge {
        io: io.clone(),
    })));

    while let Some(line) = io.readLine()? {
        let response = handleLine(&runtime, &line);
        io.writeJson(&response)?;
    }

    runtime.destroy();
    Ok(())
}

#[allow(non_snake_case)]
fn handleLine(runtime: &ToolPkgRuntime, line: &str) -> RuntimeServerResponse {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return responseError(Value::Null, "request line is empty");
    }
    let request = match serde_json::from_str::<RuntimeServerRequest>(trimmed) {
        Ok(request) => request,
        Err(error) => return responseError(Value::Null, error.to_string()),
    };
    let id = request.id.clone();
    match handleRequest(runtime, request) {
        Ok(result) => RuntimeServerResponse {
            id,
            success: true,
            result: Some(result),
            error: None,
        },
        Err(error) => responseError(id, error),
    }
}

#[allow(non_snake_case)]
fn handleRequest(runtime: &ToolPkgRuntime, request: RuntimeServerRequest) -> Result<Value, String> {
    match request.method.as_str() {
        "loadToolPkgFile" => {
            let params = fromParams::<LoadToolPkgFileParams>(request.params)?;
            serde_json::to_value(runtime.loadToolPkgFile(params.path)?)
                .map_err(|error| error.to_string())
        }
        "readToolPkgTextResource" => {
            let params = fromParams::<ReadTextResourceParams>(request.params)?;
            let value =
                runtime.readToolPkgTextResource(&params.containerPackageName, &params.resourcePath);
            serde_json::to_value(value).map_err(|error| error.to_string())
        }
        "runFunction" => {
            let call = fromParams::<ToolPkgFunctionCall>(request.params)?;
            serde_json::to_value(runtime.runFunction(call)?).map_err(|error| error.to_string())
        }
        "runMainHook" => {
            let call = fromParams::<ToolPkgMainHookCall>(request.params)?;
            serde_json::to_value(runtime.runMainHook(call)?).map_err(|error| error.to_string())
        }
        "dispatchIpc" => {
            let call = fromParams::<ToolPkgIpcCall>(request.params)?;
            serde_json::to_value(runtime.dispatchIpc(call)?).map_err(|error| error.to_string())
        }
        "destroyContext" => {
            let params = fromParams::<DestroyContextParams>(request.params)?;
            Ok(Value::Bool(runtime.destroyContext(&params.contextKey)))
        }
        "destroy" => {
            runtime.destroy();
            Ok(Value::Bool(true))
        }
        method => Err(format!("unknown runtime method: {method}")),
    }
}

#[allow(non_snake_case)]
fn fromParams<T>(params: Value) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(params).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn responseError(id: Value, error: impl Into<String>) -> RuntimeServerResponse {
    RuntimeServerResponse {
        id,
        success: false,
        result: None,
        error: Some(error.into()),
    }
}
