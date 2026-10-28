#![allow(non_snake_case)]

use operit_host_api::HostResult;

use crate::face::{rgb565, FaceRect};
use crate::robot_face::FaceCanvas;

/// Number of plugin tiles reserved on the minus-one screen.
pub const PLUGIN_SLOT_COUNT: usize = 4;

/// Outer margin of the launcher grid.
pub const LAUNCHER_MARGIN_PX: u16 = 16;
/// Horizontal and vertical gap between launcher tiles.
pub const LAUNCHER_GAP_PX: u16 = 16;
/// Launcher tile width.
pub const LAUNCHER_TILE_WIDTH_PX: u16 = 96;
/// Launcher tile height.
pub const LAUNCHER_TILE_HEIGHT_PX: u16 = 84;
/// Top edge of the first launcher row.
pub const LAUNCHER_ORIGIN_Y_PX: u16 = 44;
/// Top edge of the home-indicator bar.
pub const HOME_BAR_Y_PX: u16 = 304;

/// Paints the empty plugin shelf. Plugin packages fill these tiles later.
pub fn paintPluginShelf(canvas: &mut dyn FaceCanvas) -> HostResult<()> {
    let width = canvas.width();
    let height = canvas.height();
    canvas.fill(rgb565(8, 12, 24))?;
    canvas.fillRect(
        FaceRect {
            x: 0,
            y: 0,
            width,
            height: 28,
        },
        rgb565(15, 23, 42),
    )?;
    canvas.fillRect(
        FaceRect {
            x: 16,
            y: 10,
            width: 48,
            height: 8,
        },
        rgb565(59, 130, 246),
    )?;

    let gap = 12u16;
    let tileWidth = width.saturating_sub(gap * 3) / 2;
    let tileHeight = height.saturating_sub(28 + 36 + gap * 3) / 2;
    let originY = 28 + gap;
    for row in 0..2u16 {
        for col in 0..2u16 {
            let x = gap + col * (tileWidth + gap);
            let y = originY + row * (tileHeight + gap);
            canvas.fillRect(
                FaceRect {
                    x,
                    y,
                    width: tileWidth,
                    height: tileHeight,
                },
                rgb565(30, 64, 105),
            )?;
            if tileWidth > 8 && tileHeight > 8 {
                canvas.fillRect(
                    FaceRect {
                        x: x + 4,
                        y: y + 4,
                        width: tileWidth - 8,
                        height: tileHeight - 8,
                    },
                    rgb565(15, 23, 42),
                )?;
            }
        }
    }

    let pillWidth = 40u16;
    let pillHeight = 6u16;
    canvas.fillRect(
        FaceRect {
            x: width.saturating_sub(pillWidth) / 2,
            y: height.saturating_sub(16),
            width: pillWidth,
            height: pillHeight,
        },
        rgb565(148, 163, 184),
    )?;
    Ok(())
}

/// Paints the launcher home screen. App content is opened from this surface.
pub fn paintHome(canvas: &mut dyn FaceCanvas) -> HostResult<()> {
    let width = canvas.width();
    canvas.fill(rgb565(10, 12, 18))?;
    canvas.fillRect(
        FaceRect {
            x: 0,
            y: 0,
            width,
            height: 28,
        },
        rgb565(22, 26, 36),
    )?;
    canvas.fillRect(
        FaceRect {
            x: width.saturating_sub(28),
            y: 10,
            width: 18,
            height: 8,
        },
        rgb565(148, 163, 184),
    )?;
    canvas.fillRect(
        FaceRect {
            x: 10,
            y: 13,
            width: 6,
            height: 2,
        },
        rgb565(59, 130, 246),
    )?;
    canvas.fillRect(
        FaceRect {
            x: 10,
            y: 16,
            width: 10,
            height: 2,
        },
        rgb565(59, 130, 246),
    )?;

    drawAppTile(canvas, 0, "FACE", LauncherIcon::Face)?;
    drawAppTile(canvas, 1, "PLUG", LauncherIcon::Plugin)?;
    drawAppTile(canvas, 2, "SET", LauncherIcon::Settings)?;
    drawAppTile(canvas, 3, "TERM", LauncherIcon::Terminal)?;

    canvas.fillRect(
        FaceRect {
            x: width.saturating_sub(40) / 2,
            y: HOME_BAR_Y_PX,
            width: 40,
            height: 4,
        },
        rgb565(203, 213, 225),
    )?;
    Ok(())
}

/// Paints a simple settings app surface.
pub fn paintSettings(canvas: &mut dyn FaceCanvas) -> HostResult<()> {
    paintAppShell(canvas, "SETTINGS")?;
    let width = canvas.width();
    for (row, label) in ["WIFI", "EDGE", "PLUG"].iter().enumerate() {
        let y = 64u16.saturating_add(u16::try_from(row).unwrap_or(0).saturating_mul(44));
        if y + 32 > canvas.height() {
            break;
        }
        canvas.fillRect(
            FaceRect {
                x: 16,
                y,
                width: width.saturating_sub(32),
                height: 32,
            },
            rgb565(24, 29, 40),
        )?;
        drawText(canvas, 26, y + 13, label, rgb565(226, 232, 240))?;
        canvas.fillRect(
            FaceRect {
                x: width.saturating_sub(52),
                y: y + 11,
                width: 22,
                height: 10,
            },
            rgb565(59, 130, 246),
        )?;
    }
    Ok(())
}

/// Paints a simple terminal app surface.
pub fn paintTerminal(canvas: &mut dyn FaceCanvas) -> HostResult<()> {
    paintAppShell(canvas, "TERMINAL")?;
    let width = canvas.width();
    canvas.fillRect(
        FaceRect {
            x: 14,
            y: 48,
            width: width.saturating_sub(28),
            height: 230,
        },
        rgb565(9, 12, 16),
    )?;
    drawText(canvas, 22, 56, "OPERIT", rgb565(74, 222, 128))?;
    drawText(canvas, 22, 72, "READY", rgb565(148, 163, 184))?;
    drawText(canvas, 22, 258, ">", rgb565(74, 222, 128))?;
    Ok(())
}

/// Paints the common dark app frame and bottom return bar.
fn paintAppShell(canvas: &mut dyn FaceCanvas, title: &str) -> HostResult<()> {
    let width = canvas.width();
    canvas.fill(rgb565(10, 12, 18))?;
    canvas.fillRect(
        FaceRect {
            x: 0,
            y: 0,
            width,
            height: 28,
        },
        rgb565(22, 26, 36),
    )?;
    drawText(canvas, 14, 11, title, rgb565(226, 232, 240))?;
    canvas.fillRect(
        FaceRect {
            x: width.saturating_sub(40) / 2,
            y: HOME_BAR_Y_PX,
            width: 40,
            height: 4,
        },
        rgb565(203, 213, 225),
    )?;
    Ok(())
}

/// Returns one launcher tile by zero-based index.
pub fn launcherTile(index: usize) -> FaceRect {
    let col = u16::try_from(index % 2).unwrap_or(0);
    let row = u16::try_from(index / 2).unwrap_or(0);
    FaceRect {
        x: LAUNCHER_MARGIN_PX + col.saturating_mul(LAUNCHER_TILE_WIDTH_PX + LAUNCHER_GAP_PX),
        y: LAUNCHER_ORIGIN_Y_PX + row.saturating_mul(LAUNCHER_TILE_HEIGHT_PX + LAUNCHER_GAP_PX),
        width: LAUNCHER_TILE_WIDTH_PX,
        height: LAUNCHER_TILE_HEIGHT_PX,
    }
}

#[derive(Clone, Copy)]
enum LauncherIcon {
    Face,
    Plugin,
    Settings,
    Terminal,
}

fn drawAppTile(
    canvas: &mut dyn FaceCanvas,
    index: usize,
    label: &str,
    icon: LauncherIcon,
) -> HostResult<()> {
    let tile = launcherTile(index);
    canvas.fillRect(FaceRect { ..tile }, rgb565(31, 37, 50))?;
    canvas.fillRect(
        FaceRect {
            x: tile.x + 6,
            y: tile.y + 6,
            width: tile.width.saturating_sub(12),
            height: 50,
        },
        rgb565(15, 19, 28),
    )?;
    drawIcon(canvas, icon, &tile)?;
    let textWidth = u16::try_from(label.len().saturating_mul(4)).unwrap_or(0);
    let textX = tile
        .x
        .saturating_add(tile.width.saturating_sub(textWidth) / 2);
    drawText(canvas, textX, tile.y + 64, label, rgb565(203, 213, 225))?;
    Ok(())
}

fn drawIcon(canvas: &mut dyn FaceCanvas, icon: LauncherIcon, tile: &FaceRect) -> HostResult<()> {
    match icon {
        LauncherIcon::Face => {
            for eyeX in [tile.x + 16, tile.x + 42] {
                canvas.fillRect(
                    FaceRect {
                        x: eyeX,
                        y: tile.y + 10,
                        width: 8,
                        height: 14,
                    },
                    rgb565(74, 222, 128),
                )?;
            }
            canvas.fillRect(
                FaceRect {
                    x: tile.x + 18,
                    y: tile.y + 32,
                    width: 30,
                    height: 5,
                },
                rgb565(74, 222, 128),
            )?;
        }
        LauncherIcon::Plugin => {
            for row in 0..2u16 {
                for col in 0..2u16 {
                    canvas.fillRect(
                        FaceRect {
                            x: tile.x + 14 + col * 28,
                            y: tile.y + 8 + row * 22,
                            width: 20,
                            height: 16,
                        },
                        rgb565(56, 189, 248),
                    )?;
                }
            }
        }
        LauncherIcon::Settings => {
            for barY in [tile.y + 8, tile.y + 20, tile.y + 32] {
                canvas.fillRect(
                    FaceRect {
                        x: tile.x + 12,
                        y: barY,
                        width: 42,
                        height: 4,
                    },
                    rgb565(148, 163, 184),
                )?;
            }
            canvas.fillRect(
                FaceRect {
                    x: tile.x + 26,
                    y: tile.y + 8,
                    width: 10,
                    height: 28,
                },
                rgb565(226, 232, 240),
            )?;
        }
        LauncherIcon::Terminal => {
            canvas.fillRect(
                FaceRect {
                    x: tile.x + 12,
                    y: tile.y + 16,
                    width: 14,
                    height: 3,
                },
                rgb565(74, 222, 128),
            )?;
            canvas.fillRect(
                FaceRect {
                    x: tile.x + 20,
                    y: tile.y + 19,
                    width: 10,
                    height: 3,
                },
                rgb565(74, 222, 128),
            )?;
            canvas.fillRect(
                FaceRect {
                    x: tile.x + 24,
                    y: tile.y + 22,
                    width: 8,
                    height: 3,
                },
                rgb565(74, 222, 128),
            )?;
        }
    }
    Ok(())
}

/// Draws a compact uppercase text string using 3x5 rectangles.
pub fn drawText(
    canvas: &mut dyn FaceCanvas,
    x: u16,
    y: u16,
    text: &str,
    color: u16,
) -> HostResult<()> {
    let mut penX = x;
    for character in text.chars() {
        if character == ' ' {
            penX = penX.saturating_add(4);
            continue;
        }
        let glyph = glyph(character);
        for row in 0..5u16 {
            for col in 0..3u16 {
                if glyph[row as usize] & (4u8 >> col) != 0 {
                    canvas.fillRect(
                        FaceRect {
                            x: penX.saturating_add(col),
                            y: y.saturating_add(row),
                            width: 1,
                            height: 1,
                        },
                        color,
                    )?;
                }
            }
        }
        penX = penX.saturating_add(4);
    }
    Ok(())
}

fn glyph(character: char) -> [u8; 5] {
    match character {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b111, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b111, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'K' => [0b101, 0b110, 0b100, 0b110, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b010, 0b010, 0b010, 0b101],
        _ => [0b000, 0b000, 0b000, 0b000, 0b000],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robot_face::MemoryFaceCanvas;

    #[test]
    fn paintsFourEmptyPluginTiles() {
        let mut canvas = MemoryFaceCanvas::new(240, 320);
        paintPluginShelf(&mut canvas).expect("plugin shelf must paint");
        assert_eq!(PLUGIN_SLOT_COUNT, 4);
        assert_eq!(canvas.pixel(0, 0), Some(rgb565(15, 23, 42)));
        assert_eq!(canvas.pixel(12, 40), Some(rgb565(30, 64, 105)));
        assert_eq!(canvas.pixel(16, 44), Some(rgb565(15, 23, 42)));
        assert_eq!(canvas.pixel(120, 308), Some(rgb565(148, 163, 184)));
    }
}
