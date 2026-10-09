//! Local integration tests for the production Streamable HTTP client.
use operit_host_api::HostManager::HostManager;
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxHostRuntimeTaskSchedulerHost as NativeTaskScheduler, LinuxHttpHost as NativeHttpHost,
};
#[cfg(target_os = "macos")]
use operit_host_macos_native::{
    MacosHostRuntimeTaskSchedulerHost as NativeTaskScheduler, MacosHttpHost as NativeHttpHost,
};
#[cfg(target_os = "windows")]
use operit_host_windows_native::{
    WindowsHostRuntimeTaskSchedulerHost as NativeTaskScheduler, WindowsHttpHost as NativeHttpHost,
};
use operit_tools::tools::mcp_runtime::plugins::{
    MCPBridge::MCPBridge, MCPBridgeClient::MCPBridgeClient,
};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc, time::Instant};
#[tokio::test]
async fn streamed_responses_and_deadlines() {
    let (_fixture, base) = fixture_server();
    let context = HostManager {
        httpHost: Some(Arc::new(NativeHttpHost::new())),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeTaskScheduler::new())),
        ..HostManager::default()
    };
    let bridge = MCPBridge::getInstance(&context);
    let mut results = Vec::new();
    for (path, expected) in [
        ("/json", true),
        ("/sse-short", true),
        ("/sse-held-open", true),
        ("/json-held-open", true),
        ("/headers-timeout", false),
        ("/body-timeout", false),
        ("/sse-eof", false),
        ("/bad-json", false),
        ("/wrong-id", false),
        ("/http401", false),
    ] {
        let name = format!("probe-{path}");
        bridge.registerRemoteMcpService(
            name.clone(),
            format!("{base}{path}"),
            Some("httpStream".into()),
            None,
            None,
            BTreeMap::new(),
        );
        let client = MCPBridgeClient::new(context.clone(), name.clone());
        let start = Instant::now();
        let connected = client.connectWithSpawnTimeoutMs(200).await;
        let elapsed = start.elapsed().as_millis();
        let calls = if connected {
            vec![
                client.callTool("echo", json!({})).await,
                client.callTool("echo", json!({})).await,
            ]
        } else {
            Vec::new()
        };
        let passed = connected == expected
            && elapsed < 500
            && calls.iter().all(|call| {
                call["success"] == true
                    && call["result"]["content"][0]["text"] == "http stream probe ok 中文"
            });
        results.push(
            json!({"path":path,"transport":"httpStream","startup_budget_ms":200,
            "expected_connected":expected,"connected":connected,"connect_elapsed_ms":elapsed,
            "error":client.getLastConnectionFailureDetail(),"calls":calls,"passed":passed}),
        );
        bridge.unregisterMcpService(&name);
    }
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
    assert!(
        results.iter().all(|result| result["passed"] == true),
        "Streamable HTTP checks failed: {results:#?}"
    );
    for result in &results {
        if result["path"] == "/headers-timeout" || result["path"] == "/body-timeout" {
            assert!(result["error"].as_str().unwrap().contains("timed out"));
        }
    }
}

struct FixtureServer(std::process::Child);
impl Drop for FixtureServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn fixture_server() -> (FixtureServer, String) {
    use std::io::BufRead;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut child = std::process::Command::new("python3")
        .arg(root.join("tools/tests/fixtures/mcp_http_streaming_server.py"))
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("local fixture requires python3");
    let stdout = child.stdout.take().unwrap();
    let fixture = FixtureServer(child);
    let mut port = String::new();
    std::io::BufReader::new(stdout)
        .read_line(&mut port)
        .expect("read fixture port");
    let port: u16 = port
        .trim()
        .parse()
        .expect("fixture did not report a valid port");
    (fixture, format!("http://127.0.0.1:{port}"))
}
