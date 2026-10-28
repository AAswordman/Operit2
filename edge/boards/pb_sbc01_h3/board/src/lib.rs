#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    CapabilityOperation, CapabilityScope, HostCapability, HostEnvironmentDescriptor, HostError,
    HostOnboardingRequirement, HostRequirementAction, HostRequirementStatus, HostResult,
    RobotFaceExpressionRequest, RobotFaceHost, RobotFaceState,
};
use serde::{Deserialize, Serialize};

pub const PB_SBC01_H3_BOARD_ID: &str = "PB_SBC01_H3";
pub const PB_SBC01_H3_BOARD_DISPLAY_NAME: &str = "PB_SBC01_H3";
pub const PB_SBC01_H3_FACE_STATE_FILE: &str = "robot_face_state.json";

const INITIAL_EXPRESSION: &str = "neutral";
const SUPPORTED_EXPRESSIONS: &[&str] = &[
    "neutral",
    "booting",
    "online",
    "listening",
    "thinking",
    "speaking",
    "happy",
    "sleeping",
    "error",
];

/// Configures board-specific state locations for the PB_SBC01_H3 host.
#[derive(Clone, Debug)]
pub struct PbSbc01H3BoardConfig {
    pub stateDirectory: PathBuf,
}

impl PbSbc01H3BoardConfig {
    /// Creates a PB_SBC01_H3 board configuration from an explicit state directory.
    pub fn new(stateDirectory: impl Into<PathBuf>) -> Self {
        Self {
            stateDirectory: stateDirectory.into(),
        }
    }
}

/// Owns the PB_SBC01_H3 board capability implementations.
pub struct PbSbc01H3Board {
    robotFaceHost: Arc<PbSbc01H3RobotFaceHost>,
}

impl PbSbc01H3Board {
    /// Creates the PB_SBC01_H3 board profile and initializes its state outputs.
    pub fn new(config: PbSbc01H3BoardConfig) -> HostResult<Self> {
        let robotFaceHost = Arc::new(PbSbc01H3RobotFaceHost::new(config.stateDirectory)?);
        Ok(Self { robotFaceHost })
    }

    /// Returns the board-owned robot face host implementation.
    pub fn robotFaceHost(&self) -> Arc<dyn RobotFaceHost> {
        self.robotFaceHost.clone()
    }

    /// Adds PB_SBC01_H3 board capabilities to a Linux HostManager.
    pub fn installIntoHostManager(&self, hostManager: HostManager) -> HostManager {
        hostManager
            .withRobotFaceHost(self.robotFaceHost())
            .withHostEnvironment(pbSbc01H3HostEnvironment())
    }

    /// Returns the robot face state file consumed by display renderers.
    pub fn robotFaceStatePath(&self) -> PathBuf {
        self.robotFaceHost.statePath()
    }
}

/// Builds the PB_SBC01_H3 Linux board host environment descriptor.
pub fn pbSbc01H3HostEnvironment() -> HostEnvironmentDescriptor {
    let mut descriptor = HostEnvironmentDescriptor::linux();
    descriptor.id = "pb_sbc01_h3".to_string();
    descriptor.displayName = PB_SBC01_H3_BOARD_DISPLAY_NAME.to_string();
    descriptor.capabilities.push("robot.face".to_string());
    descriptor.structuredCapabilities.push(HostCapability {
        id: "robot.face".to_string(),
        displayName: "机器人表情屏".to_string(),
        scope: CapabilityScope::Device,
        operations: vec![CapabilityOperation::Read, CapabilityOperation::Write],
    });
    descriptor
        .onboardingRequirements
        .push(HostOnboardingRequirement {
            id: "board.pb_sbc01_h3.face".to_string(),
            title: "PB_SBC01_H3 robot face".to_string(),
            description: "显示当前机器人表情屏的板级服务状态。".to_string(),
            capabilityIds: vec!["robot.face".to_string()],
            isRequired: true,
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::HostManaged,
        });
    descriptor
}

/// Stores PB_SBC01_H3 robot face state for the local display renderer.
pub struct PbSbc01H3RobotFaceHost {
    statePath: PathBuf,
    state: Mutex<RobotFaceState>,
}

impl PbSbc01H3RobotFaceHost {
    /// Creates a robot face host backed by a board-local state file.
    pub fn new(stateDirectory: impl Into<PathBuf>) -> HostResult<Self> {
        let stateDirectory = stateDirectory.into();
        fs::create_dir_all(&stateDirectory).map_err(HostError::from)?;
        let statePath = stateDirectory.join(PB_SBC01_H3_FACE_STATE_FILE);
        let state = RobotFaceState {
            expression: INITIAL_EXPRESSION.to_string(),
        };
        writeFaceState(&statePath, &state)?;
        Ok(Self {
            statePath,
            state: Mutex::new(state),
        })
    }

    /// Returns the state file used by the face renderer.
    pub fn statePath(&self) -> PathBuf {
        self.statePath.clone()
    }
}

impl RobotFaceHost for PbSbc01H3RobotFaceHost {
    /// Writes one validated expression to the board face state file.
    fn setExpression(&self, request: RobotFaceExpressionRequest) -> HostResult<RobotFaceState> {
        validateExpression(&request.expression)?;
        let state = RobotFaceState {
            expression: request.expression,
        };
        writeFaceState(&self.statePath, &state)?;
        let mut current = self
            .state
            .lock()
            .map_err(|error| HostError::new(format!("robot face state lock poisoned: {error}")))?;
        *current = state.clone();
        Ok(state)
    }

    /// Reads the current expression committed by this board host.
    fn getExpression(&self) -> HostResult<RobotFaceState> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|error| HostError::new(format!("robot face state lock poisoned: {error}")))
    }
}

/// Describes the state file format consumed by local face renderers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PbSbc01H3FaceStateFile {
    pub boardId: String,
    pub expression: String,
    pub updatedAtUnixMillis: u128,
}

/// Writes one face state file for the board display process.
fn writeFaceState(path: &Path, state: &RobotFaceState) -> HostResult<()> {
    let content = PbSbc01H3FaceStateFile {
        boardId: PB_SBC01_H3_BOARD_ID.to_string(),
        expression: state.expression.clone(),
        updatedAtUnixMillis: currentUnixMillis()?,
    };
    let bytes = serde_json::to_vec_pretty(&content)
        .map_err(|error| HostError::new(format!("serialize robot face state failed: {error}")))?;
    fs::write(path, bytes).map_err(HostError::from)
}

/// Returns the current Unix timestamp in milliseconds.
fn currentUnixMillis() -> HostResult<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| HostError::new(format!("system time before Unix epoch: {error}")))
}

/// Validates one robot face expression identifier against the PB_SBC01_H3 profile.
fn validateExpression(expression: &str) -> HostResult<()> {
    let isSupported = SUPPORTED_EXPRESSIONS
        .iter()
        .any(|supported| *supported == expression);
    if !isSupported {
        return Err(HostError::new(format!(
            "unsupported robot face expression: {expression}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a unique temporary state directory for one board test.
    fn testStateDirectory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "operit-pb-sbc01-h3-{name}-{}",
            currentUnixMillis().expect("test clock must be available")
        ))
    }

    /// Verifies the robot face host writes the state file used by renderers.
    #[test]
    fn robotFaceHostWritesStateFile() {
        let dir = testStateDirectory("face-state");
        let host = PbSbc01H3RobotFaceHost::new(&dir).expect("robot face host must initialize");
        let state = host
            .setExpression(RobotFaceExpressionRequest {
                expression: "happy".to_string(),
            })
            .expect("robot face expression must commit");
        let file = fs::read_to_string(host.statePath()).expect("state file must be readable");
        let parsed: PbSbc01H3FaceStateFile =
            serde_json::from_str(&file).expect("state file JSON must decode");

        assert_eq!(state.expression, "happy");
        assert_eq!(parsed.boardId, PB_SBC01_H3_BOARD_ID);
        assert_eq!(parsed.expression, "happy");

        fs::remove_dir_all(dir).expect("test state directory must be removable");
    }

    /// Verifies unknown expression identifiers are rejected by the board profile.
    #[test]
    fn robotFaceHostRejectsUnknownExpression() {
        let dir = testStateDirectory("face-validation");
        let host = PbSbc01H3RobotFaceHost::new(&dir).expect("robot face host must initialize");
        let error = host
            .setExpression(RobotFaceExpressionRequest {
                expression: "unknown".to_string(),
            })
            .expect_err("unknown expression must be rejected");

        assert_eq!(error.message, "unsupported robot face expression: unknown");

        fs::remove_dir_all(dir).expect("test state directory must be removable");
    }
}
