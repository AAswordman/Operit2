//! A headless product using the manufacturer SDK, independent of the ESP32 UI.
use operit_edge_sdk::{
    agent::*,
    device::EdgeNode,
    host::HostManager::HostManager,
    link::{protocol::LinkDeviceInfo, toCoreValue, CoreCallResponse},
    runtime::{
        NodeServices::{PeerEndpoint, PeerTransport},
        PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore},
    },
    space::{NodeSpaceContext, PeerRuntimeLimits, SpaceDevice},
};
use operit_host_native_common::{NativeRuntimeStorageHost, NativeTcpHost};
use operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost;
use std::{path::PathBuf, sync::Arc};
use tokio::io::{AsyncBufReadExt, BufReader};

struct Temperature;
impl Tool for Temperature {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "temperature.read",
            description: "Read the product temperature sensor",
            input_schema: r#"{"type":"object","properties":{},"additionalProperties":false}"#,
        }
    }
    fn call(&self, arguments: &str) -> Result<String, AgentError> {
        if arguments.trim() != "{}" {
            return Err(AgentError::Tool("Expected an empty object".into()));
        }
        // Replace this deterministic desktop sensor with the manufacturer's HAL.
        Ok(r#"{"celsius":23.5,"source":"desktop-demo"}"#.into())
    }
}
struct OfflineBackend;
impl Backend for OfflineBackend {
    fn next<'a>(&'a self, request: Request<'a>) -> AgentFuture<'a> {
        Box::pin(async move {
            if let Some(reading) = request.observations.last() {
                Ok(Step::Reply(format!("Temperature: {}", reading.result)))
            } else {
                Ok(Step::Call {
                    tool: "temperature.read".into(),
                    arguments: "{}".into(),
                })
            }
        })
    }
}
fn agent() -> Result<Agent<OfflineBackend>, AgentError> {
    let mut agent = Agent::new(OfflineBackend, Limits::default())?;
    agent.register(Temperature)?;
    Ok(agent)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--once") {
        println!("{}", agent()?.run("Read the sensor", "temperature").await?);
        return Ok(());
    }
    let state = std::env::var_os("OPERIT_SENSOR_STATE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".sensor-state"));
    let host = HostManager::default()
        .withRuntimeStorageHost(Arc::new(NativeRuntimeStorageHost::new(
            state.join("runtime"),
            state.join("workspace"),
        )))
        .withTcpHost(Arc::new(NativeTcpHost))
        .withHostRuntimeTaskSchedulerHost(Arc::new(LocalHostRuntimeTaskSchedulerHost::new()?));
    let node = EdgeNode::headless().withCallHandler(Arc::new(|request| {
        Box::pin(async move {
            let result = match (request.target.as_str(), request.methodName.as_str()) {
                ("manufacturer.sensor", "read") => {
                    Temperature.call("{}").map_err(|e| e.to_string())
                }
                ("manufacturer.agent", "run") => match agent() {
                    Ok(agent) => agent
                        .run("Read the sensor", "temperature")
                        .await
                        .map_err(|e| e.to_string()),
                    Err(error) => Err(error.to_string()),
                },
                _ => return None,
            };
            Some(match result {
                Ok(text) => CoreCallResponse::ok(request.requestId, toCoreValue(text).unwrap()),
                Err(error) => CoreCallResponse::err(
                    request.requestId,
                    operit_edge_sdk::link::CoreLinkError::new("SENSOR_ERROR", error),
                ),
            })
        })
    }));
    let device = SpaceDevice::assemble(
        host,
        LinkDeviceInfo {
            platform: "headless".into(),
            model: "Temperature sensor".into(),
            pairingIncarnation: None,
        },
        node,
        PeerRuntimeLimits::constrained(),
    )?;
    device
        .listen(PeerHostConfig {
            bindAddress: "127.0.0.1:0".into(),
            transports: vec![PeerTransport::Tcp],
            discoveryEnabled: false,
            portMode: PeerHostPortMode::Automatic,
            updatedAt: 0,
        })
        .await?;
    let address = PeerStateStore::new(device.space.storage())
        .hostConfig()?
        .ok_or("Missing listener configuration")?
        .bindAddress;
    println!("node={} address={address}", device.node_id);
    println!("Commands: status | agent | pair <address> | confirm <id> <code> | join <node> | refresh <request> | quit");
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        let words: Vec<_> = line.split_whitespace().collect();
        let peers = device.services.peers();
        match words.as_slice() {
            ["quit"] => break,
            ["agent"] => println!("{}", agent()?.run("Read the sensor", "temperature").await?),
            ["status"] => println!(
                "space={} peers={:?} prompts={:?}",
                device.space.spaceStore().space()?.spaceId,
                peers.pairedPeers()?,
                peers.pairingPrompts()?
            ),
            ["pair", address] => println!(
                "{:?}",
                peers
                    .startPairing(
                        PeerEndpoint {
                            nodeId: String::new(),
                            address: (*address).into()
                        },
                        Some(PeerTransport::Tcp)
                    )
                    .await
            ),
            ["confirm", id, code] => println!("{:?}", peers.finishPairing(id, code).await),
            ["join", peer] => println!(
                "{:?}",
                device.space.requestDeviceSpaceJoin((*peer).into()).await
            ),
            ["refresh", id] => println!(
                "{:?}",
                device.space.refreshDeviceSpaceJoin((*id).into()).await
            ),
            _ => println!("Unknown command"),
        }
    }
    device.stop().await?;
    Ok(())
}
