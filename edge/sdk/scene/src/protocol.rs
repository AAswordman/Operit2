//! Native device configuration and bounded asset upload protocol.
pub const SERVICE_TARGET: &str = "edge.scene";
pub const VERSION: u32 = 1;
pub const ACTIONS: &[&str] = &[
    "capabilities", "config.read", "config.apply", "config.clear", "state",
    "pack.begin", "pack.chunk", "pack.commit", "pack.abort", "pack.remove",
];
pub const MAX_REQUEST: usize = 1024;
pub const MAX_REPLY: usize = 4096;
pub const MAX_PACK: usize = 4096;
pub const MAX_CACHE: usize = 8192;
pub const MAX_PACKS: usize = 2;
pub const MAX_ASSETS: usize = 4;
pub const MAX_LAYERS: usize = 4;
pub const MAX_CHUNK: usize = 256;
