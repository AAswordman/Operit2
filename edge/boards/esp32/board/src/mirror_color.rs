/// Quantizes RGB565 for the memory-constrained remote mirror only.
pub fn pack_rgb332(color: u16) -> u8 {
    (((color >> 8) & 0xe0) | ((color >> 6) & 0x1c) | ((color >> 3) & 0x03)) as u8
}

/// Expands RGB332 using bit replication, preserving black and white endpoints.
pub fn unpack_rgb332(color: u8) -> u16 {
    let r = u16::from(color >> 5);
    let g = u16::from((color >> 2) & 7);
    let b = u16::from(color & 3);
    (((r << 2) | (r >> 1)) << 11)
        | (((g << 3) | g) << 5)
        | ((b << 3) | (b << 1) | (b >> 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_primary_colors_and_endpoints() {
        for color in [0x0000, 0xffff, 0xf800, 0x07e0, 0x001f] {
            assert_eq!(unpack_rgb332(pack_rgb332(color)), color);
        }
    }

    #[test]
    fn every_palette_entry_round_trips() {
        for color in 0..=255u8 {
            assert_eq!(pack_rgb332(unpack_rgb332(color)), color);
        }
    }
}
