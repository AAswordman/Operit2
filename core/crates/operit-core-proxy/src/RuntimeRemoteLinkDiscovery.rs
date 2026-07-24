use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent};

use crate::RuntimeRemoteLinkService::RuntimeRemoteDiscoveredDevice;

const OPERIT_SERVICE_TYPE: &str = "_operit._tcp.local.";
type MdnsIpv4Rank = (u8, [u8; 4]);

/// Discovers Link-enabled runtimes visible through the native mDNS transport.
#[allow(non_snake_case)]
pub(crate) fn discoverRemoteDevices(
    timeoutMs: u64,
) -> Result<Vec<RuntimeRemoteDiscoveredDevice>, String> {
    let daemon = ServiceDaemon::new().map_err(|error| error.to_string())?;
    let receiver = daemon
        .browse(OPERIT_SERVICE_TYPE)
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut devices = BTreeMap::<String, (MdnsIpv4Rank, RuntimeRemoteDiscoveredDevice)>::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let fullName = info.get_fullname().to_string();
                let mut addresses = info
                    .get_addresses()
                    .iter()
                    .filter_map(|address| match address {
                        std::net::IpAddr::V4(address) => Some(*address),
                        std::net::IpAddr::V6(_) => None,
                    })
                    .collect::<Vec<_>>();
                if addresses.is_empty() {
                    continue;
                }
                addresses.sort_by_key(mdnsIpv4Rank);
                let selectedAddress = addresses[0];
                let selectedRank = mdnsIpv4Rank(&selectedAddress);
                let properties = info.get_properties();
                let device = RuntimeRemoteDiscoveredDevice {
                    deviceId: requiredMdnsProperty(properties, "deviceId", &fullName)?,
                    displayName: requiredMdnsProperty(properties, "displayName", &fullName)?,
                    platform: requiredMdnsProperty(properties, "platform", &fullName)?,
                    model: requiredMdnsProperty(properties, "model", &fullName)?,
                    baseUrl: format!("http://{}:{}", selectedAddress, info.get_port()),
                    hostname: info.get_hostname().to_string(),
                    port: info.get_port(),
                    tokenHash: requiredMdnsProperty(properties, "tokenHash", &fullName)?,
                    version: requiredMdnsProperty(properties, "version", &fullName)?,
                };
                match devices.get_mut(&fullName) {
                    Some((currentRank, currentDevice)) if selectedRank < *currentRank => {
                        *currentRank = selectedRank;
                        *currentDevice = device;
                    }
                    Some(_) => {}
                    None => {
                        devices.insert(fullName, (selectedRank, device));
                    }
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    Ok(devices
        .into_values()
        .map(|(_, device)| device)
        .collect())
}

/// Reads one required property from a resolved Operit mDNS service record.
#[allow(non_snake_case)]
fn requiredMdnsProperty(
    properties: &mdns_sd::TxtProperties,
    name: &str,
    serviceName: &str,
) -> Result<String, String> {
    properties
        .get(name)
        .map(|property| property.val_str().to_string())
        .ok_or_else(|| format!("mDNS service missing {name}: {serviceName}"))
}

/// Assigns a deterministic preference to private IPv4 addresses for a discovered service.
#[allow(non_snake_case)]
fn mdnsIpv4Rank(address: &Ipv4Addr) -> MdnsIpv4Rank {
    let class = if address.is_link_local() {
        2
    } else if address.is_private() {
        0
    } else if address.is_loopback() {
        3
    } else if address.is_unspecified() {
        4
    } else {
        1
    };
    (class, address.octets())
}
