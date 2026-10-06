#![allow(non_snake_case)]

use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};

use operit_host_api::{
    HostError, HostResult, RobotFaceExpressionRequest, RobotFaceHost, RobotFaceState,
};

use crate::face::{faceLayout, validateExpression, FaceLayout, FaceRect, INITIAL_EXPRESSION};

/// Draws robot face rectangles onto a board-owned pixel target.
pub trait FaceCanvas: Send {
    /// Logical canvas width in pixels.
    fn width(&self) -> u16;

    /// Logical canvas height in pixels.
    fn height(&self) -> u16;

    /// Fills the entire canvas with one RGB565 color.
    fn fill(&mut self, color: u16) -> HostResult<()>;

    /// Fills one rectangle with one RGB565 color.
    fn fillRect(&mut self, rect: FaceRect, color: u16) -> HostResult<()>;

    /// Flushes a self-drawn UI RGB565 region to the physical display.
    fn flushRgb565(&mut self, _rect: FaceRect, _pixels: &[u8]) -> HostResult<()> {
        Err(HostError::new("canvas does not support self-drawn UI flush"))
    }
}

/// In-memory RGB565 canvas used by Host tests and as a drawing model.
pub struct MemoryFaceCanvas {
    width: u16,
    height: u16,
    pixels: Vec<u16>,
}

impl MemoryFaceCanvas {
    /// Creates a zeroed RGB565 canvas of the requested size.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; usize::from(width) * usize::from(height)],
        }
    }

    /// Returns one pixel in row-major order.
    pub fn pixel(&self, x: u16, y: u16) -> Option<u16> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels
            .get(usize::from(y) * usize::from(self.width) + usize::from(x))
            .copied()
    }
}

impl FaceCanvas for MemoryFaceCanvas {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn fill(&mut self, color: u16) -> HostResult<()> {
        for pixel in &mut self.pixels {
            *pixel = color;
        }
        Ok(())
    }

    fn fillRect(&mut self, rect: FaceRect, color: u16) -> HostResult<()> {
        let maxX = self.width;
        let maxY = self.height;
        for y in rect.y..rect.y.saturating_add(rect.height).min(maxY) {
            for x in rect.x..rect.x.saturating_add(rect.width).min(maxX) {
                let index = usize::from(y) * usize::from(self.width) + usize::from(x);
                self.pixels[index] = color;
            }
        }
        Ok(())
    }
}

/// Renders validated robot face expressions through a board display canvas.
pub struct Esp32RobotFaceHost<C: FaceCanvas> {
    canvas: Mutex<C>,
    state: Mutex<RobotFaceState>,
    uiActive: AtomicBool,
}

impl<C: FaceCanvas> Esp32RobotFaceHost<C> {
    /// Creates a robot face host around one display canvas.
    pub fn new(canvas: C) -> HostResult<Self> {
        let host = Self {
            canvas: Mutex::new(canvas),
            state: Mutex::new(RobotFaceState {
                expression: INITIAL_EXPRESSION.to_string(),
            }),
            uiActive: AtomicBool::new(false),
        };
        host.paint(INITIAL_EXPRESSION)?;
        Ok(host)
    }

    /// Transfers physical screen ownership from the legacy face renderer to self-drawn UI.
    pub fn activateUi(&self) {
        self.uiActive.store(true, Ordering::Release);
    }

    /// Paints the empty plugin shelf over the face canvas.
    pub fn paintPluginShelf(&self) -> HostResult<()> {
        let mut canvas = self
            .canvas
            .lock()
            .map_err(|error| HostError::new(format!("robot face canvas lock poisoned: {error}")))?;
        crate::shell::paintPluginShelf(&mut *canvas)
    }

    /// Paints the launcher home surface over the face canvas.
    pub fn paintHome(&self) -> HostResult<()> {
        let mut canvas = self
            .canvas
            .lock()
            .map_err(|error| HostError::new(format!("robot face canvas lock poisoned: {error}")))?;
        crate::shell::paintHome(&mut *canvas)
    }

    /// Paints the settings app surface.
    pub fn paintSettings(&self) -> HostResult<()> {
        let mut canvas = self
            .canvas
            .lock()
            .map_err(|error| HostError::new(format!("robot face canvas lock poisoned: {error}")))?;
        crate::shell::paintSettings(&mut *canvas)
    }

    /// Paints the terminal app surface.
    pub fn paintTerminal(&self) -> HostResult<()> {
        let mut canvas = self
            .canvas
            .lock()
            .map_err(|error| HostError::new(format!("robot face canvas lock poisoned: {error}")))?;
        crate::shell::paintTerminal(&mut *canvas)
    }

    /// Sends one self-drawn UI partial framebuffer region through the board canvas.
    pub fn flushRgb565(&self, rect: FaceRect, pixels: &[u8]) -> HostResult<()> {
        let mut canvas = self
            .canvas
            .lock()
            .map_err(|error| HostError::new(format!("robot face canvas lock poisoned: {error}")))?;
        canvas.flushRgb565(rect, pixels)
    }

    /// Paints one already-validated expression onto the canvas.
    fn paint(&self, expression: &str) -> HostResult<FaceLayout> {
        let mut canvas = self
            .canvas
            .lock()
            .map_err(|error| HostError::new(format!("robot face canvas lock poisoned: {error}")))?;
        let layout = faceLayout(expression, canvas.width(), canvas.height())?;
        if self.uiActive.load(Ordering::Acquire) {
            return Ok(layout);
        }
        canvas.fill(layout.background)?;
        canvas.fillRect(layout.leftEye, layout.accent)?;
        canvas.fillRect(layout.rightEye, layout.accent)?;
        canvas.fillRect(layout.mouth, layout.accent)?;
        Ok(layout)
    }
}

impl Esp32RobotFaceHost<MemoryFaceCanvas> {
    /// Creates a robot face host backed by an in-memory canvas.
    pub fn withMemoryCanvas(width: u16, height: u16) -> HostResult<Self> {
        Self::new(MemoryFaceCanvas::new(width, height))
    }
}

impl<C: FaceCanvas> RobotFaceHost for Esp32RobotFaceHost<C> {
    /// Writes one validated expression and paints it on the board display.
    fn setExpression(&self, request: RobotFaceExpressionRequest) -> HostResult<RobotFaceState> {
        validateExpression(&request.expression)?;
        self.paint(&request.expression)?;
        let state = RobotFaceState {
            expression: request.expression,
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies a supported expression is stored and painted onto the canvas.
    #[test]
    fn paintsSupportedExpression() {
        let host =
            Esp32RobotFaceHost::withMemoryCanvas(240, 320).expect("memory face host must start");
        let state = host
            .setExpression(RobotFaceExpressionRequest {
                expression: "online".to_string(),
            })
            .expect("online expression must commit");
        assert_eq!(state.expression, "online");
        assert_eq!(
            host.getExpression()
                .expect("committed expression must be readable")
                .expression,
            "online"
        );
    }

    /// Verifies unknown expressions do not replace the committed face.
    #[test]
    fn rejectsUnknownExpression() {
        let host =
            Esp32RobotFaceHost::withMemoryCanvas(240, 320).expect("memory face host must start");
        let error = host
            .setExpression(RobotFaceExpressionRequest {
                expression: "unknown".to_string(),
            })
            .expect_err("unknown expression must be rejected");
        assert_eq!(error.message, "unsupported robot face expression: unknown");
        assert_eq!(
            host.getExpression()
                .expect("initial expression must remain")
                .expression,
            INITIAL_EXPRESSION
        );
    }

    #[test]
    fn paintsPluginShelfWithoutChangingExpression() {
        let host =
            Esp32RobotFaceHost::withMemoryCanvas(240, 320).expect("memory face host must start");
        host.paintPluginShelf().expect("plugin shelf must paint");
        assert_eq!(
            host.getExpression()
                .expect("face expression must stay committed")
                .expression,
            INITIAL_EXPRESSION
        );
    }
}
