#![allow(non_snake_case)]

use operit_host_api::{HostError, HostResult};

/// Supported robot face expressions on ESP32-2432S028.
pub const SUPPORTED_EXPRESSIONS: &[&str] = &[
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

/// Initial expression shown before Edge services publish a later state.
pub const INITIAL_EXPRESSION: &str = "neutral";

/// Axis-aligned rectangle in logical display pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Drawable geometry for one robot face expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceLayout {
    pub background: u16,
    pub accent: u16,
    pub leftEye: FaceRect,
    pub rightEye: FaceRect,
    pub mouth: FaceRect,
}

/// Packs an RGB888 color into RGB565.
pub fn rgb565(red: u8, green: u8, blue: u8) -> u16 {
    ((u16::from(red) & 0xf8) << 8) | ((u16::from(green) & 0xfc) << 3) | (u16::from(blue) >> 3)
}

/// Validates one robot face expression identifier against this board profile.
pub fn validateExpression(expression: &str) -> HostResult<()> {
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

/// Builds one face layout in the logical display coordinate space.
pub fn faceLayout(expression: &str, width: u16, height: u16) -> HostResult<FaceLayout> {
    validateExpression(expression)?;
    let eyeWidth = (width / 5).max(16);
    let eyeHeight = (height / 6).max(16);
    let eyeY = height / 4;
    let leftEyeX = width / 5;
    let rightEyeX = width.saturating_sub(width / 5 + eyeWidth);
    let mouthWidth = width / 3;
    let mouthX = (width.saturating_sub(mouthWidth)) / 2;
    let mouthY = (height * 5) / 8;
    Ok(FaceLayout {
        background: rgb565(8, 12, 24),
        accent: faceAccent(expression),
        leftEye: FaceRect {
            x: leftEyeX,
            y: eyeY,
            width: eyeWidth,
            height: eyeHeightFor(expression, eyeHeight),
        },
        rightEye: FaceRect {
            x: rightEyeX,
            y: eyeY + rightEyeOffset(expression, eyeHeight),
            width: eyeWidth,
            height: eyeHeightFor(expression, eyeHeight),
        },
        mouth: FaceRect {
            x: mouthX,
            y: mouthY,
            width: mouthWidth,
            height: mouthHeightFor(expression, height),
        },
    })
}

/// Selects the accent color for one expression.
fn faceAccent(expression: &str) -> u16 {
    match expression {
        "error" => rgb565(220, 48, 48),
        "happy" | "online" => rgb565(72, 196, 120),
        "booting" | "thinking" => rgb565(240, 176, 48),
        "sleeping" => rgb565(88, 112, 168),
        _ => rgb565(80, 168, 255),
    }
}

/// Selects eye height for one expression.
fn eyeHeightFor(expression: &str, eyeHeight: u16) -> u16 {
    match expression {
        "sleeping" | "booting" => (eyeHeight / 4).max(4),
        _ => eyeHeight,
    }
}

/// Shifts the right eye for the thinking expression.
fn rightEyeOffset(expression: &str, eyeHeight: u16) -> u16 {
    match expression {
        "thinking" => eyeHeight / 3,
        _ => 0,
    }
}

/// Selects mouth height for one expression.
fn mouthHeightFor(expression: &str, height: u16) -> u16 {
    let base = (height / 18).max(8);
    match expression {
        "happy" | "online" | "speaking" => base * 2,
        "sleeping" => (base / 2).max(4),
        "error" => base * 3,
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies unknown expressions are rejected before layout is computed.
    #[test]
    fn rejectsUnknownExpression() {
        let error = validateExpression("unknown").expect_err("unknown expression must fail");
        assert_eq!(error.message, "unsupported robot face expression: unknown");
    }

    /// Verifies the portrait face keeps both eyes inside 240x320.
    #[test]
    fn portraitLayoutStaysInBounds() {
        let layout = faceLayout("happy", 240, 320).expect("happy layout must exist");
        assert!(layout.leftEye.x + layout.leftEye.width <= 240);
        assert!(layout.rightEye.x + layout.rightEye.width <= 240);
        assert!(layout.mouth.y + layout.mouth.height <= 320);
        assert_ne!(layout.background, layout.accent);
    }
}
