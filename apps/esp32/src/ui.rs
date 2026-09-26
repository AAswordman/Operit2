#![allow(non_snake_case)]

use operit_ui::{
    Color, Document, Gesture, Icon, Rect, Screen, ScreenBuilder, TextAlignment, Widget,
};

/// Vertical distance, in logical pixels, that opens or closes the plugin shelf.
pub const SWIPE_THRESHOLD_PX: i16 = 40;

/// Horizontal distance, in logical pixels, that completes a left-edge back swipe.
pub const EDGE_SWIPE_THRESHOLD_PX: i16 = 35;

/// Start zone width, in logical pixels, for a left-edge back swipe.
pub const EDGE_SWIPE_START_PX: i16 = 24;

/// Surfaces owned by the firmware micro-system shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeSurface {
    /// Launcher home screen.
    Home,
    /// Robot face app.
    Face,
    /// Plugin app.
    PluginShelf,
    /// Settings app.
    Settings,
    /// Terminal app.
    Terminal,
}

impl HomeSurface {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Face => "face",
            Self::PluginShelf => "plugins",
            Self::Settings => "settings",
            Self::Terminal => "terminal",
        }
    }

    pub fn fromId(id: &str) -> Option<Self> {
        match id {
            "home" => Some(Self::Home),
            "face" => Some(Self::Face),
            "plugins" => Some(Self::PluginShelf),
            "settings" => Some(Self::Settings),
            "terminal" => Some(Self::Terminal),
            _ => None,
        }
    }
}

/// Gestures the home shell understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiGesture {
    Back,
    SwipeDown,
    SwipeUp,
    Tap { x: u16, y: u16 },
}

impl From<Gesture> for UiGesture {
    fn from(gesture: Gesture) -> Self {
        match gesture {
            Gesture::Back => Self::Back,
            Gesture::SwipeDown => Self::SwipeDown,
            Gesture::SwipeUp => Self::SwipeUp,
            Gesture::Tap { x, y } => Self::Tap { x, y },
        }
    }
}

impl From<UiGesture> for Gesture {
    fn from(gesture: UiGesture) -> Self {
        match gesture {
            UiGesture::Back => Gesture::Back,
            UiGesture::SwipeDown => Gesture::SwipeDown,
            UiGesture::SwipeUp => Gesture::SwipeUp,
            UiGesture::Tap { x, y } => Gesture::Tap { x, y },
        }
    }
}

/// Firmware home-shell state machine. It does not run plugins.
#[derive(Debug)]
pub struct HomeUi {
    surface: HomeSurface,
}

/// Owns the firmware UI document and screen router state.
#[derive(Debug)]
pub struct UiState {
    document: Document,
    home: HomeUi,
}

impl UiState {
    /// Creates a UI runtime state from a document and the shell state machine.
    pub fn new(document: Document, home: HomeUi) -> Self {
        Self { document, home }
    }

    /// Returns the currently visible screen.
    pub fn screen(&self) -> Option<&Screen> {
        let id = self.home.surface().id();
        self.document.screen(id)
    }

    /// Returns the underlying UI document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Replaces the document. This is the entry point used by the web editor.
    pub fn setDocument(&mut self, document: Document) {
        self.document = document;
    }

    /// Returns the surface currently selected by the shell.
    pub fn surface(&self) -> HomeSurface {
        self.home.surface()
    }

    /// Applies one gesture, using document hit-testing for app taps.
    pub fn apply(&mut self, gesture: Gesture) -> bool {
        if let Gesture::Tap { x, y } = gesture {
            let screenId = self.home.surface().id();
            if let Some(target) = self.document.targetAt(screenId, x, y) {
                if let Some(surface) = HomeSurface::fromId(target) {
                    if surface != self.home.surface() {
                        self.home.surface = surface;
                        return true;
                    }
                }
            }
        }
        self.home.apply(gesture.into())
    }
}

/// Builds the default launcher and app screens.
pub fn defaultDocument(width: u16, height: u16) -> Document {
    Document {
        screens: vec![
            homeScreen(width, height),
            faceScreen(width, height),
            pluginsScreen(width, height),
            settingsScreen(width, height),
            terminalScreen(width, height),
        ],
    }
}

fn statusBar(width: u16) -> Vec<Widget> {
    vec![
        Widget::Panel {
            rect: Rect::new(0, 0, width, 28),
            color: Color::rgb565(22, 26, 36),
            radius: 0,
        },
        Widget::Icon {
            rect: Rect::new(10, 8, 20, 14),
            icon: Icon::Wifi,
            color: Color::rgb565(59, 130, 246),
        },
        Widget::Icon {
            rect: Rect::new(width.saturating_sub(34), 8, 20, 14),
            icon: Icon::Battery,
            color: Color::rgb565(148, 163, 184),
        },
    ]
}

fn homeBar(width: u16, height: u16) -> Vec<Widget> {
    vec![Widget::Panel {
        rect: Rect::new(
            width.saturating_sub(40) / 2,
            height.saturating_sub(16),
            40,
            4,
        ),
        color: Color::rgb565(203, 213, 225),
        radius: 0,
    }]
}

fn homeScreen(width: u16, height: u16) -> Screen {
    let mut builder = Screen::builder("home", "Home", Color::rgb565(10, 12, 18));
    builder.widgets.extend(statusBar(width));
    builder.widgets.extend(homeBar(width, height));
    builder
        .button(
            Rect::new(16, 44, 96, 84),
            "FACE",
            "face",
            Color::rgb565(31, 37, 50),
            Some(Icon::Face),
        )
        .button(
            Rect::new(128, 44, 96, 84),
            "PLUG",
            "plugins",
            Color::rgb565(31, 37, 50),
            Some(Icon::Grid),
        )
        .button(
            Rect::new(16, 144, 96, 84),
            "SET",
            "settings",
            Color::rgb565(31, 37, 50),
            Some(Icon::Gear),
        )
        .button(
            Rect::new(128, 144, 96, 84),
            "TERM",
            "terminal",
            Color::rgb565(31, 37, 50),
            Some(Icon::Terminal),
        )
        .build()
}

fn appShell(id: &str, title: &str, width: u16, height: u16) -> ScreenBuilder {
    let mut builder = Screen::builder(id, title, Color::rgb565(10, 12, 18));
    builder
        .panel(Rect::new(0, 0, width, 28), Color::rgb565(22, 26, 36))
        .button(
            Rect::new(4, 6, 16, 16),
            "",
            "home",
            Color::rgb565(0, 0, 0),
            Some(Icon::Back),
        )
        .text(
            Rect::new(28, 10, 120, 8),
            title,
            Color::rgb565(226, 232, 240),
            TextAlignment::Left,
        )
        .panel(
            Rect::new(16, 44, width.saturating_sub(32), height.saturating_sub(72)),
            Color::rgb565(24, 29, 40),
        );
    builder.widgets.extend(homeBar(width, height));
    builder
}

fn faceScreen(width: u16, height: u16) -> Screen {
    appShell("face", "FACE", width, height)
        .icon(
            Rect::new(width.saturating_sub(120) / 2, 64, 120, 64),
            Icon::Face,
            Color::rgb565(74, 222, 128),
        )
        .text(
            Rect::new(24, 148, width.saturating_sub(48), 8),
            "EXPRESSION",
            Color::rgb565(203, 213, 225),
            TextAlignment::Left,
        )
        .build()
}

fn pluginsScreen(width: u16, height: u16) -> Screen {
    let mut builder = appShell("plugins", "PLUGINS", width, height);
    for row in 0..2u16 {
        for column in 0..2u16 {
            builder.panel(
                Rect::new(24 + column * 96, 72 + row * 88, 88, 76),
                Color::rgb565(30, 64, 105),
            );
        }
    }
    builder.build()
}

fn settingsScreen(width: u16, height: u16) -> Screen {
    appShell("settings", "SETTINGS", width, height)
        .text(
            Rect::new(24, 64, 120, 8),
            "WIFI",
            Color::rgb565(226, 232, 240),
            TextAlignment::Left,
        )
        .text(
            Rect::new(24, 104, 120, 8),
            "EDGE",
            Color::rgb565(226, 232, 240),
            TextAlignment::Left,
        )
        .text(
            Rect::new(24, 144, 120, 8),
            "PLUG",
            Color::rgb565(226, 232, 240),
            TextAlignment::Left,
        )
        .build()
}

fn terminalScreen(width: u16, height: u16) -> Screen {
    appShell("terminal", "TERMINAL", width, height)
        .text(
            Rect::new(24, 64, 180, 8),
            "OPERIT",
            Color::rgb565(74, 222, 128),
            TextAlignment::Left,
        )
        .text(
            Rect::new(24, 80, 180, 8),
            "READY",
            Color::rgb565(148, 163, 184),
            TextAlignment::Left,
        )
        .text(
            Rect::new(24, height.saturating_sub(56), 40, 8),
            ">",
            Color::rgb565(74, 222, 128),
            TextAlignment::Left,
        )
        .build()
}

impl HomeUi {
    /// Starts on the launcher home screen.
    pub fn new() -> Self {
        Self {
            surface: HomeSurface::Home,
        }
    }

    /// Returns the surface currently owned by the shell.
    pub fn surface(&self) -> HomeSurface {
        self.surface
    }

    /// Applies one gesture. Returns whether the visible surface changed.
    pub fn apply(&mut self, gesture: UiGesture) -> bool {
        let next = match (self.surface, gesture) {
            (HomeSurface::Home, UiGesture::Tap { x, y }) => {
                homeAppAt(x, y).unwrap_or(HomeSurface::Home)
            }
            (
                HomeSurface::Face
                | HomeSurface::PluginShelf
                | HomeSurface::Settings
                | HomeSurface::Terminal,
                UiGesture::Tap { x: _, y },
            ) => {
                if y >= operit_board_esp32::HOME_BAR_Y_PX {
                    HomeSurface::Home
                } else {
                    self.surface
                }
            }
            (HomeSurface::Home, UiGesture::SwipeDown) => HomeSurface::Face,
            (HomeSurface::Home, UiGesture::SwipeUp) => HomeSurface::PluginShelf,
            (HomeSurface::Face | HomeSurface::PluginShelf, UiGesture::SwipeUp) => HomeSurface::Home,
            (
                HomeSurface::Face
                | HomeSurface::PluginShelf
                | HomeSurface::Settings
                | HomeSurface::Terminal,
                UiGesture::Back,
            ) => HomeSurface::Home,
            (surface, _) => surface,
        };
        if next == self.surface {
            return false;
        }
        self.surface = next;
        true
    }
}

/// Resolves one launcher tap to an app surface.
pub fn homeAppAt(x: u16, y: u16) -> Option<HomeSurface> {
    for (index, surface) in [
        HomeSurface::Face,
        HomeSurface::PluginShelf,
        HomeSurface::Settings,
        HomeSurface::Terminal,
    ]
    .into_iter()
    .enumerate()
    {
        let tile = operit_board_esp32::launcherTile(index);
        if x >= tile.x
            && x < tile.x.saturating_add(tile.width)
            && y >= tile.y
            && y < tile.y.saturating_add(tile.height)
        {
            return Some(surface);
        }
    }
    None
}

/// Tracks one press-drag-release and emits a vertical swipe on lift.
pub struct SwipeTracker {
    origin: Option<(i16, i16)>,
    last: Option<(i16, i16)>,
}

impl SwipeTracker {
    /// Creates an idle tracker.
    pub fn new() -> Self {
        Self {
            origin: None,
            last: None,
        }
    }

    /// Feeds one sample. A swipe is reported when the finger lifts.
    pub fn onSample(&mut self, point: Option<(u16, u16)>) -> Option<UiGesture> {
        match point {
            Some((x, y)) => {
                let sample = (x as i16, y as i16);
                if self.origin.is_none() {
                    self.origin = Some(sample);
                }
                self.last = Some(sample);
                None
            }
            None => match (self.origin.take(), self.last.take()) {
                (Some(origin), Some(last)) => gestureFromSwipe(origin.0, origin.1, last.0, last.1),
                _ => None,
            },
        }
    }
}

/// Classifies a touch release as a tap or vertical swipe.
pub fn gestureFromSwipe(startX: i16, startY: i16, endX: i16, endY: i16) -> Option<UiGesture> {
    let dx = endX.saturating_sub(startX);
    let dy = endY.saturating_sub(startY);
    if dy.abs() < SWIPE_THRESHOLD_PX && dx.abs() < SWIPE_THRESHOLD_PX {
        return Some(UiGesture::Tap {
            x: endX.max(0) as u16,
            y: endY.max(0) as u16,
        });
    }
    if startX <= EDGE_SWIPE_START_PX
        && dx >= EDGE_SWIPE_THRESHOLD_PX
        && dy.abs() < SWIPE_THRESHOLD_PX
    {
        return Some(UiGesture::Back);
    }
    if dy.abs() <= SWIPE_THRESHOLD_PX || dy.abs() <= dx.abs() {
        return None;
    }
    if dy > 0 {
        Some(UiGesture::SwipeDown)
    } else {
        Some(UiGesture::SwipeUp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcherOpensWithTap() {
        let mut home = HomeUi::new();
        assert_eq!(home.surface(), HomeSurface::Home);
        assert!(home.apply(UiGesture::Tap { x: 20, y: 50 }));
        assert_eq!(home.surface(), HomeSurface::Face);
        assert!(home.apply(UiGesture::Back));
        assert!(home.apply(UiGesture::Tap { x: 140, y: 50 }));
        assert_eq!(home.surface(), HomeSurface::PluginShelf);
    }

    #[test]
    fn homeBarReturnsToLauncher() {
        let mut home = HomeUi::new();
        home.apply(UiGesture::Tap { x: 20, y: 50 });
        assert!(home.apply(UiGesture::Tap { x: 120, y: 308 }));
        assert_eq!(home.surface(), HomeSurface::Home);
    }

    #[test]
    fn verticalDragMapsToSwipe() {
        assert_eq!(
            gestureFromSwipe(120, 20, 118, 90),
            Some(UiGesture::SwipeDown)
        );
        assert_eq!(
            gestureFromSwipe(120, 200, 122, 40),
            Some(UiGesture::SwipeUp)
        );
        assert_eq!(gestureFromSwipe(10, 10, 80, 12), Some(UiGesture::Back));
        assert_eq!(
            gestureFromSwipe(120, 20, 120, 50),
            Some(UiGesture::Tap { x: 120, y: 50 })
        );
    }

    #[test]
    fn trackerEmitsSwipeOnRelease() {
        let mut tracker = SwipeTracker::new();
        assert_eq!(tracker.onSample(Some((120, 20))), None);
        assert_eq!(tracker.onSample(Some((118, 90))), None);
        assert_eq!(tracker.onSample(None), Some(UiGesture::SwipeDown));
        assert_eq!(tracker.onSample(None), None);
    }
}
