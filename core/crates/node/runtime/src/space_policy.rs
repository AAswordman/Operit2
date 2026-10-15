//! Shared Space eligibility; transport credentials and local execution stay with their owners.
use operit_link::CoreLinkError;
use operit_store::CoreSpaceStore::CoreSpaceStore;
use operit_store::NetworkControlStore::NetworkControlStore;

/// Pairing alone cannot authorize a Space return channel. Both endpoints must
/// be admitted and active in the same initialized control projection.
pub(crate) fn spaceChannelScope(
    spaceStore: &CoreSpaceStore,
    controlStore: &NetworkControlStore,
    local: &str,
    peer: &str,
) -> Result<Option<String>, CoreLinkError> {
    let space = spaceStore.space().map_err(CoreLinkError::internal)?;
    let control = controlStore
        .currentState()
        .map_err(CoreLinkError::internal)?;
    if peer == local || !control.initialized || control.spaceId != space.spaceId {
        return Ok(None);
    }
    for node in [local, peer] {
        if !space.members.iter().any(|member| member == node)
            || !control.memberNodeIds.contains(node)
            || control.disconnectedNodeIds.contains(node)
        {
            return Ok(None);
        }
    }
    Ok(Some(space.spaceId))
}

/// One control projection is used for target, first-hop and transit eligibility.
#[cfg(feature = "full")]
pub(crate) struct RoutingPolicy {
    pub(crate) blocked: std::collections::BTreeSet<String>,
    transit: std::collections::BTreeSet<String>,
}

#[cfg(feature = "full")]
impl RoutingPolicy {
    pub(crate) fn load(store: &NetworkControlStore) -> Result<Self, String> {
        let (blocked, transit) = store.routingPolicySnapshot()?;
        Ok(Self { blocked, transit })
    }

    pub(crate) fn routePlan(
        &self,
        local: &str,
        store: &CoreSpaceStore,
        target: &str,
        peers: &std::collections::BTreeSet<String>,
    ) -> Result<Option<operit_store::CoreSpaceStore::CoreSpaceRoutePlan>, String> {
        if target == local || self.blocked.contains(target) || !store.contains(target.to_owned())? {
            return Ok(None);
        }
        store.routePlanThroughPeersWithTransitNodes(
            target.to_owned(),
            peers.difference(&self.blocked).cloned().collect(),
            self.transit.difference(&self.blocked).cloned().collect(),
        )
    }
}
