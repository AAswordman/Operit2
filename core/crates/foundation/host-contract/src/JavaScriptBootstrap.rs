use std::sync::OnceLock;

use crate::{HostError, HostResult};

/// Shares only trusted bootstrap bytecode; each runtime owns its executed globals and heap.
#[derive(Debug, Default)]
pub struct HostJavaScriptBootstrap {
    compiled: OnceLock<(&'static str, HostResult<Vec<u8>>)>,
}

impl HostJavaScriptBootstrap {
    /// Compiles one immutable source once and preserves exact compilation failures.
    pub fn bytecode(
        &self,
        source: &'static str,
        compile: impl FnOnce() -> HostResult<Vec<u8>>,
    ) -> HostResult<&[u8]> {
        let (identity, result) = self.compiled.get_or_init(|| (source, compile()));
        if *identity != source {
            return Err(HostError::new(
                "A JavaScript host cannot change its bootstrap source",
            ));
        }
        result.as_ref().map(Vec::as_slice).map_err(Clone::clone)
    }
}
