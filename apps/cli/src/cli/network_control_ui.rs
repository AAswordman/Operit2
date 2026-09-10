use operit_node_runtime::RuntimeRemoteLinkService::{
    RuntimeDeviceSpaceDevice, RuntimeDeviceSpaceTopology,
};
use operit_store::NetworkControlStore::NetworkControlRole;
use operit_store::NetworkControlStore::NetworkControlState;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// Builds the visible label used to identify one device in management commands.
pub(crate) fn network_device_label(device: &RuntimeDeviceSpaceDevice) -> String {
    let mut parts = vec![device.deviceName.clone()];
    if !device.userName.trim().is_empty() {
        parts.push(device.userName.clone());
    }
    if !device.platform.trim().is_empty() {
        parts.push(device.platform.clone());
    }
    if !device.model.trim().is_empty() {
        parts.push(device.model.clone());
    }
    parts.join(" · ")
}

/// Resolves one exact human-facing device label to its runtime identifier.
pub(crate) fn network_device_id(
    topology: &RuntimeDeviceSpaceTopology,
    label: &str,
) -> Result<String, String> {
    if topology
        .devices
        .iter()
        .chain(&topology.removedDevices)
        .any(|device| device.deviceId == label)
    {
        return Ok(label.to_string());
    }
    network_device_labels(topology)
        .into_iter()
        .find_map(|(device_id, device_label)| (device_label == label).then_some(device_id))
        .ok_or_else(|| format!("network device does not exist or is ambiguous: {label}"))
}

/// Resolves one exact human-facing role name to its runtime identifier.
pub(crate) fn network_role_id(state: &NetworkControlState, name: &str) -> Result<String, String> {
    let matches = state
        .roles
        .values()
        .filter(|role| role.displayName == name)
        .map(|role| role.roleId.clone())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [role_id] => Ok(role_id.clone()),
        [] => Err(format!("network role does not exist: {name}")),
        _ => Err(format!("network role name is ambiguous: {name}")),
    }
}

/// Resolves one runtime-owned device identifier to its visible topology label.
pub(crate) fn network_device_label_by_id(
    topology: &RuntimeDeviceSpaceTopology,
    device_id: &str,
) -> Result<String, String> {
    network_device_labels(topology)
        .get(device_id)
        .cloned()
        .ok_or_else(|| "network device is not present in the current topology".to_string())
}

/// Allocates an internal control identifier used only by the runtime protocol.
pub(crate) fn new_network_control_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4().simple())
}

/// Returns display labels for every device indexed by its runtime-owned identifier.
pub(crate) fn network_device_labels(
    topology: &RuntimeDeviceSpaceTopology,
) -> BTreeMap<String, String> {
    let mut counts = BTreeMap::new();
    for device in topology.devices.iter().chain(&topology.removedDevices) {
        *counts.entry(network_device_label(device)).or_insert(0usize) += 1;
    }
    let mut occurrences = BTreeMap::new();
    topology
        .devices
        .iter()
        .chain(&topology.removedDevices)
        .map(|device| {
            let base = network_device_label(device);
            let occurrence = occurrences.entry(base.clone()).or_insert(0usize);
            *occurrence += 1;
            let label = if counts[&base] == 1 {
                format!("{base} ({})", device.deviceId)
            } else {
                format!("{base} · device {occurrence} ({})", device.deviceId)
            };
            (device.deviceId.clone(), label)
        })
        .collect()
}

/// Converts a role into the concise human-facing representation used by listings.
pub(crate) fn network_role_summary(role: &NetworkControlRole) -> String {
    let mut capabilities = role.capabilities.iter().cloned().collect::<Vec<_>>();
    capabilities.sort();
    format!(
        "{} · {}",
        role.displayName,
        capabilities
            .iter()
            .map(|capability| network_capability_label(capability))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Converts one protocol capability into a human-facing CLI label.
fn network_capability_label(capability: &str) -> String {
    match capability {
        "*" => "all permissions".to_string(),
        "network.audit.read" => "view audit history".to_string(),
        "network.relay" => "relay network traffic".to_string(),
        "storage.provide" => "provide storage".to_string(),
        "runtime.execute" => "run tasks".to_string(),
        "network.user" => "use the network".to_string(),
        value => value.to_string(),
    }
}

/// Converts the short capability names accepted by CLI and TUI into protocol values.
pub(crate) fn network_capabilities(values: &[String]) -> Result<BTreeSet<String>, String> {
    values
        .iter()
        .map(|value| {
            let capability = match value.as_str() {
                "all" => "*",
                "audit" => "network.audit.read",
                "relay" => "network.relay",
                "storage" => "storage.provide",
                "execute" => "runtime.execute",
                "network" => "network.user",
                "view" => "network.devices.view",
                "manage-identities" => "network.identity.manage",
                "assign-identity" => "network.identity.assign",
                "approve" => "network.approval",
                _ => return Err(format!("unknown capability: {value}")),
            };
            Ok(capability.to_string())
        })
        .collect()
}
