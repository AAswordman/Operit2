#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    CapabilityOperation, CapabilityScope, DeviceIoHost, HostCapability, HostEnvironmentDescriptor,
    HostOnboardingRequirement, HostRequirementAction, HostRequirementStatus,
};

#[cfg(target_os = "espidf")]
pub mod link_channel;

pub mod face;
pub mod mirror_color;
pub mod pins;
pub mod robot_face;
pub mod shell;
pub mod touch;

#[cfg(target_os = "espidf")]
mod board;
#[cfg(target_os = "espidf")]
mod display;
#[cfg(target_os = "espidf")]
mod gpio;

#[cfg(target_os = "espidf")]
pub use board::Esp32Board;
#[cfg(target_os = "espidf")]
pub use display::Esp32Ili9341;
#[cfg(target_os = "espidf")]
pub use display::{Esp32ScreenMirror, Esp32ScreenMirrorRect};
#[cfg(target_os = "espidf")]
pub use gpio::Esp32GpioHost;
#[cfg(target_os = "espidf")]
pub use touch::Esp32Touch;

pub use face::{
    faceLayout, rgb565, validateExpression, FaceLayout, FaceRect, INITIAL_EXPRESSION,
    SUPPORTED_EXPRESSIONS,
};
pub use pins::{
    logicalDisplaySize, madctlForRotation, DISPLAY_ROTATION_DEGREES,
    ESP32_2432S028_BOARD_DISPLAY_NAME, ESP32_2432S028_BOARD_ID, LED_BLUE_PIN, LED_GREEN_PIN,
    LED_RED_PIN, PANEL_NATIVE_HEIGHT, PANEL_NATIVE_WIDTH, SD_CS_PIN, TFT_BACKLIGHT_PIN, TFT_CS_PIN,
    TFT_DC_PIN, TFT_MISO_PIN, TFT_MOSI_PIN, TFT_SCLK_PIN, TOUCH_CS_PIN, TOUCH_INT_PIN,
    TOUCH_MISO_PIN, TOUCH_MOSI_PIN, TOUCH_SCLK_PIN,
};
pub use robot_face::{Esp32RobotFaceHost, FaceCanvas, MemoryFaceCanvas};
pub use shell::{
    launcherTile, paintHome, paintPluginShelf, paintSettings, paintTerminal, HOME_BAR_Y_PX,
    LAUNCHER_GAP_PX, LAUNCHER_MARGIN_PX, LAUNCHER_ORIGIN_Y_PX, LAUNCHER_TILE_HEIGHT_PX,
    LAUNCHER_TILE_WIDTH_PX, PLUGIN_SLOT_COUNT,
};
pub use touch::{mapRawToLogical, TouchPoint};

/// Creates the HostManager used by an ESP32 Edge Core app.
pub fn createRuntimeHostManager(deviceIoHost: Arc<dyn DeviceIoHost>) -> HostManager {
    HostManager::new()
        .withDeviceIoHost(deviceIoHost)
        .withHostEnvironment(esp32_2432s028HostEnvironment())
}

/// Builds the ESP32-2432S028 host environment descriptor.
pub fn esp32_2432s028HostEnvironment() -> HostEnvironmentDescriptor {
    let mut descriptor = HostEnvironmentDescriptor::esp32();
    descriptor.id = ESP32_2432S028_BOARD_ID.to_string();
    descriptor.displayName = ESP32_2432S028_BOARD_DISPLAY_NAME.to_string();
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
            id: "board.esp32_2432s028.face".to_string(),
            title: "ESP32-2432S028 robot face".to_string(),
            description: "显示当前机器人表情屏的板级服务状态。".to_string(),
            capabilityIds: vec!["robot.face".to_string()],
            isRequired: true,
            status: HostRequirementStatus::Missing,
            action: HostRequirementAction::HostManaged,
        });
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{RobotFaceExpressionRequest, RobotFaceHost};

    /// Verifies the board environment advertises the robot face capability.
    #[test]
    fn boardEnvironmentIncludesRobotFace() {
        let descriptor = esp32_2432s028HostEnvironment();
        assert_eq!(descriptor.id, ESP32_2432S028_BOARD_ID);
        assert!(descriptor.capabilities.iter().any(|id| id == "robot.face"));
    }

    /// Verifies HostManager construction accepts a memory-backed face host.
    #[test]
    fn hostManagerInstallsMemoryFace() {
        let face = Esp32RobotFaceHost::withMemoryCanvas(240, 320)
            .expect("memory face host must initialize");
        let hostManager = HostManager::new()
            .withRobotFaceHost(Arc::new(face))
            .withHostEnvironment(esp32_2432s028HostEnvironment());
        let state = hostManager
            .robotFaceHost
            .expect("robot face host must be installed")
            .setExpression(RobotFaceExpressionRequest {
                expression: "listening".to_string(),
            })
            .expect("listening expression must commit");
        assert_eq!(state.expression, "listening");
    }
}
