//! Opt-in network integration test for the production remote MCP client.
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use operit_host_api::HostManager::HostManager;
use operit_tools::tools::mcp_runtime::plugins::{
    MCPBridge::MCPBridge, MCPBridgeClient::MCPBridgeClient,
};
use serde_json::{json, Value};

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

#[tokio::test]
#[ignore = "requires public MCP services and network access"]
async fn public_remote_mcp_services() {
    let context = HostManager {
        httpHost: Some(Arc::new(NativeHttpHost::new())),
        hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeTaskScheduler::new())),
        ..HostManager::default()
    };
    let bridge = MCPBridge::getInstance(&context);
    let cases = [
        (
            "microsoft-learn",
            "https://learn.microsoft.com/api/mcp",
            "microsoft_docs_search",
            json!({"query": "Azure Functions HTTP trigger"}),
        ),
        (
            "deepwiki",
            "https://mcp.deepwiki.com/mcp",
            "read_wiki_structure",
            json!({"repoName": "modelcontextprotocol/python-sdk"}),
        ),
        (
            "cloudflare-docs",
            "https://docs.mcp.cloudflare.com/mcp",
            "search_cloudflare_documentation",
            json!({"query": "Workers KV get"}),
        ),
    ];
    let mut reports = Vec::new();
    for (name, endpoint, tool, arguments) in cases {
        reports.push(check(
            &bridge,
            &context,
            name,
            endpoint,
            "httpStream",
            tool,
            arguments,
            30_000,
        ).await);
    }
    {
        reports.push(check(
            &bridge,
            &context,
            "coingecko-sse",
            "https://mcp.api.coingecko.com/sse",
            "sse",
            "search_docs",
            json!({"query": "simple price", "language": "typescript"}),
            8_000,
        ).await);
    }
    println!("{}", serde_json::to_string_pretty(&reports).unwrap());
    assert!(
        reports.iter().all(|report| report["passed"] == true),
        "remote MCP checks failed: {reports:#?}"
    );
}

#[allow(clippy::too_many_arguments)]
async fn check(
    bridge: &MCPBridge,
    context: &HostManager,
    name: &str,
    endpoint: &str,
    transport: &str,
    tool: &str,
    arguments: Value,
    timeout_ms: u64,
) -> Value {
    let started = Instant::now();
    let id = format!("remote-smoke-{name}");
    eprintln!("Testing {name} ({transport})...");
    let registered = bridge.registerRemoteMcpService(
        id.clone(),
        endpoint.to_string(),
        Some(transport.to_string()),
        None,
        None,
        BTreeMap::new(),
    );
    let client = MCPBridgeClient::new(context.clone(), id.clone());
    let connected = client.connectWithSpawnTimeoutMs(timeout_ms).await;
    let mut report = json!({"name": name, "endpoint": endpoint, "transport": transport,
        "registered": registered["success"], "connected": connected,
        "connection_error": client.getLastConnectionFailureDetail(), "passed": false});
    if connected {
        let info = bridge.getServiceInfo(&id).unwrap();
        report["ready"] = json!(info.ready);
        report["tool_count"] = json!(info.toolCount);
        let discovered = client.getTools();
        report["tools"] = json!(info.toolNames);
        report["client_tool_count"] = json!(discovered.len());
        if info.toolNames.iter().any(|name| name == tool) {
            let call = client.callTool(tool, arguments.clone()).await;
            report["call"] = summarize(&call);
            // A second request checks session/header continuity, not automatic replay.
            let second = client.callTool(tool, arguments).await;
            report["second_call"] = summarize(&second);
            let invalid = client.callTool("operit_smoke_nonexistent_tool", json!({})).await;
            report["unknown_tool_rejected"] = json!(invalid["success"] == false);
            report["passed"] = json!(
                info.ready
                    && discovered.len() == info.toolCount
                    && call["success"] == true
                    && second["success"] == true
                    && call["result"]["content"]
                        .as_array()
                        .is_some_and(|blocks| !blocks.is_empty())
                    && second["result"]["content"]
                        .as_array()
                        .is_some_and(|blocks| !blocks.is_empty())
                    && invalid["success"] == false
            );
        } else {
            report["error"] = json!(format!("Expected tool not advertised: {tool}"));
        }
    }
    let stopped = client.unspawn();
    report["stopped"] = json!(stopped);
    if connected && !stopped {
        report["passed"] = json!(false);
    }
    bridge.unregisterMcpService(&id);
    report["elapsed_ms"] = json!(started.elapsed().as_millis());
    eprintln!(
        "{name}: passed={} elapsed={}ms",
        report["passed"], report["elapsed_ms"]
    );
    report
}

fn summarize(response: &Value) -> Value {
    if response["success"] != true {
        return response.clone();
    }
    let content = response["result"]["content"].as_array();
    json!({"success": true, "content_blocks": content.map(Vec::len).unwrap_or_default(),
        "result_bytes": response["result"].to_string().len(),
        "preview": content.and_then(|blocks| blocks.first()).and_then(|block| block["text"].as_str()).map(|text| text.chars().take(200).collect::<String>())})
}
