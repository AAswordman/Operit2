use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

const OPERIT_SERVICE_TYPE: &str = "_operit._tcp.local.";
type MdnsIpv4Rank = (u8, [u8; 4]);

/// Describes one transport endpoint resolved from an Operit mDNS service record.
#[derive(Clone, Debug)]
pub(crate) struct RuntimeRemoteDiscoveryEndpoint {
    pub deviceId: String,
    pub baseUrl: String,
    pub hostname: String,
    pub port: u16,
    pub tokenHash: String,
    pub version: String,
}

/// Discovers Link-enabled runtimes visible through the native mDNS transport.
#[allow(non_snake_case)]
pub(crate) fn discoverRemoteDevices(
    timeoutMs: u64,
) -> Result<Vec<RuntimeRemoteDiscoveryEndpoint>, String> {
    let daemon = ServiceDaemon::new().map_err(|error| error.to_string())?;
    let receiver = daemon
        .browse(OPERIT_SERVICE_TYPE)
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut devices = BTreeMap::<String, (MdnsIpv4Rank, RuntimeRemoteDiscoveryEndpoint)>::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let Some((selectedRank, device)) = discoveryEndpointFromServiceInfo(&info)? else {
                    continue;
                };
                let fullName = info.get_fullname().to_string();
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

    Ok(devices.into_values().map(|(_, device)| device).collect())
}

/// Subscribes to Link-enabled runtime announcements from the native mDNS transport.
#[allow(non_snake_case)]
pub(crate) fn subscribeRemoteDeviceAnnouncements(
    onDevice: impl Fn(RuntimeRemoteDiscoveryEndpoint) + Send + 'static,
) -> Result<(), String> {
    let daemon = ServiceDaemon::new().map_err(|error| error.to_string())?;
    let receiver = daemon
        .browse(OPERIT_SERVICE_TYPE)
        .map_err(|error| error.to_string())?;
    std::thread::Builder::new()
        .name("operit-mdns-link-announcements".to_string())
        .spawn(move || {
            let _daemon = daemon;
            while let Ok(event) = receiver.recv() {
                let ServiceEvent::ServiceResolved(info) = event else {
                    continue;
                };
                let endpoint = match discoveryEndpointFromServiceInfo(&info) {
                    Ok(Some((_, endpoint))) => endpoint,
                    Ok(None) => continue,
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::w(
                            "RuntimeRemoteLinkDiscovery",
                            &format!("mDNS announcement ignored: {error}"),
                        );
                        continue;
                    }
                };
                onDevice(endpoint);
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Converts one resolved mDNS service into a reachable Link endpoint.
#[allow(non_snake_case)]
fn discoveryEndpointFromServiceInfo(
    info: &ServiceInfo,
) -> Result<Option<(MdnsIpv4Rank, RuntimeRemoteDiscoveryEndpoint)>, String> {
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
        return Ok(None);
    }
    addresses.sort_by_key(mdnsIpv4Rank);
    let selectedAddress = addresses[0];
    let selectedRank = mdnsIpv4Rank(&selectedAddress);
    let properties = info.get_properties();
    Ok(Some((
        selectedRank,
        RuntimeRemoteDiscoveryEndpoint {
            deviceId: requiredMdnsProperty(properties, "deviceId", &fullName)?,
            baseUrl: format!("http://{}:{}", selectedAddress, info.get_port()),
            hostname: info.get_hostname().to_string(),
            port: info.get_port(),
            tokenHash: requiredMdnsProperty(properties, "tokenHash", &fullName)?,
            version: requiredMdnsProperty(properties, "version", &fullName)?,
        },
    )))
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
