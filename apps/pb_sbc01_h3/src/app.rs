#![allow(non_snake_case)]

use std::sync::Arc;

use operit_access_runtime::RemoteDeviceInfo;
use operit_board_pb_sbc01_h3::{PbSbc01H3Board, PbSbc01H3BoardConfig, PB_SBC01_H3_BOARD_ID};
use operit_core_application::{CoreApplication, CoreApplicationConfig, CoreRemoteLinkServerConfig};
use operit_host_api::HostManager::HostManager;
use operit_host_linux_native::{createRuntimeHostManager, LinuxWebVisitHost};
use operit_node_edge::{createDeviceIoService, createRobotFaceService, EdgeNode};
use operit_proxy_edge::{EdgeProxy, EdgeRobotFaceClient};

use crate::config::PbSbc01H3Config;

/// Starts full Operit Core with PB_SBC01_H3 board services attached.
pub async fn run() -> Result<(), String> {
    let config = PbSbc01H3Config::parse(std::env::args().skip(1))?;
    let board = PbSbc01H3Board::new(PbSbc01H3BoardConfig::new(config.stateRoot.clone()))
        .map_err(|error| error.message)?;
    let hostManager = createPbSbc01H3HostManager(&config, &board);
    let mut boardProxy = createBoardProxy(hostManager.clone());

    boardProxy
        .setExpression("booting".to_string())
        .await
        .map_err(|error| error.to_string())?;

    let core = CoreApplication::start(CoreApplicationConfig::new(
        hostManager,
        RemoteDeviceInfo {
            platform: "pb_sbc01_h3".to_string(),
            model: PB_SBC01_H3_BOARD_ID.to_string(),
        },
    ))
    .await?;

    boardProxy
        .setExpression("online".to_string())
        .await
        .map_err(|error| error.to_string())?;

    println!(
        "operit-pb-sbc01-h3 serving {} as {}",
        config.bindAddress,
        core.accessIdentity().deviceInfo.displayName()
    );
    println!(
        "PB_SBC01_H3 face state file: {}",
        board.robotFaceStatePath().display()
    );

    core.serveRemoteLink(CoreRemoteLinkServerConfig::new(
        config.bindAddress,
        config.token,
    ))
    .await
}

/// Creates the Linux HostManager with PB_SBC01_H3 board capabilities attached.
fn createPbSbc01H3HostManager(config: &PbSbc01H3Config, board: &PbSbc01H3Board) -> HostManager {
    let hostManager = createRuntimeHostManager(
        config.runtimeRoot.clone(),
        config.workspaceRoot.clone(),
        Arc::new(LinuxWebVisitHost::new()),
    );
    board.installIntoHostManager(hostManager)
}

/// Creates the typed board proxy used by PB_SBC01_H3 local services.
fn createBoardProxy(hostManager: HostManager) -> EdgeProxy<EdgeNode> {
    let node = EdgeNode::new(createDeviceIoService(hostManager.clone()))
        .withRobotFaceService(createRobotFaceService(hostManager));
    EdgeProxy::new(node)
}
