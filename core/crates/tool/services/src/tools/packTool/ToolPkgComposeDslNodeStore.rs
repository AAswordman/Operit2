use std::collections::BTreeMap;

use super::ToolPkgComposeDslSession::{ToolPkgComposeDslNodeRecord, ToolPkgComposeDslNodeUpdate};

/// Retains typed node records for hosts that render a finite snapshot or terminal surface.
#[derive(Default)]
pub struct ToolPkgComposeDslNodeStore {
    nodes: BTreeMap<String, ToolPkgComposeDslNodeRecord>,
    root_id: Option<String>,
    revision: u64,
}

impl ToolPkgComposeDslNodeStore {
    /// Applies one ordered commit and validates every changed node reference.
    pub fn apply(&mut self, update: ToolPkgComposeDslNodeUpdate) -> Result<(), String> {
        if !update.reset && update.revision != self.revision + 1 {
            return Err(format!("Compose update revision is out of order: {}, expected {}", update.revision, self.revision + 1));
        }
        if update.reset { self.nodes.clear(); }
        let changed: Vec<String> = update.upserts.iter().map(|record| record.id.clone()).collect();
        for record in update.upserts { self.nodes.insert(record.id.clone(), record); }
        for id in update.removed {
            self.nodes.remove(&id).ok_or_else(|| format!("Compose update removes an unknown node: {id}"))?;
        }
        self.node(&update.rootId)?;
        for id in changed {
            let record = self.node(&id)?;
            for child in record.children.iter().chain(record.slots.values().flatten()) { self.node(child)?; }
        }
        self.root_id = Some(update.rootId);
        self.revision = update.revision;
        Ok(())
    }

    /// Resolves a required node without parsing or constructing a recursive tree.
    pub fn node(&self, id: &str) -> Result<&ToolPkgComposeDslNodeRecord, String> {
        self.nodes.get(id).ok_or_else(|| format!("Compose update references an unknown node: {id}"))
    }

    /// Resolves the root after the first successful composition commit.
    pub fn root(&self) -> Result<&ToolPkgComposeDslNodeRecord, String> {
        self.node(self.root_id.as_deref().ok_or("Compose session has no root")?)
    }

    /// Exports the final flat records once for a finite desktop widget presentation.
    pub fn snapshot(&self) -> Result<ToolPkgComposeDslNodeUpdate, String> {
        Ok(ToolPkgComposeDslNodeUpdate {
            reset: true, revision: self.revision, rootId: self.root()?.id.clone(),
            upserts: self.nodes.values().cloned().collect(), removed: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rejects skipped revisions and applies local record edits without changing other nodes.
    #[test]
    fn retained_updates_require_contiguous_revisions() {
        let record = ToolPkgComposeDslNodeRecord { id: "root".into(), nodeType: "Text".into(), props: BTreeMap::new(), children: Vec::new(), slots: BTreeMap::new() };
        let mut store = ToolPkgComposeDslNodeStore::default();
        let mut update = ToolPkgComposeDslNodeUpdate { reset: true, revision: 1, rootId: "root".into(), upserts: vec![record], removed: Vec::new() };
        store.apply(update.clone()).unwrap();
        update.reset = false;
        update.revision = 3;
        assert!(store.apply(update.clone()).is_err());
        update.revision = 2;
        store.apply(update).unwrap();
        assert_eq!(store.snapshot().unwrap().revision, 2);
    }
}
