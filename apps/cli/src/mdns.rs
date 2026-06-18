use std::collections::{BTreeMap, HashMap};
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

pub const OPERIT_SERVICE_TYPE: &str = "_operit._tcp.local.";
type MdnsIpv4Rank = (u8, [u8; 4]);

pub struct MdnsRegistration {
    daemon: ServiceDaemon,
    fullname: String,
}

impl MdnsRegistration {
    pub fn register(port: u16, properties: HashMap<String, String>) -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        let service_type = OPERIT_SERVICE_TYPE.to_string();
        let hostname = operit_mdns_hostname(port);
        let instance_name = operit_mdns_instance_name(port);
        let service_info = ServiceInfo::new(
            &service_type,
            &instance_name,
            &hostname,
            "",
            port,
            properties,
        )
        .map_err(|e| e.to_string())?
        .enable_addr_auto();
        let fullname = service_info.get_fullname().to_string();
        daemon.register(service_info).map_err(|e| e.to_string())?;
        Ok(Self { daemon, fullname })
    }
}

impl Drop for MdnsRegistration {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
    }
}

fn operit_mdns_hostname(port: u16) -> String {
    format!("operit-{}-{port}.local.", std::process::id())
}

fn operit_mdns_instance_name(port: u16) -> String {
    format!("operit-{}-{port}", std::process::id())
}

pub fn discover_devices(timeout_ms: u64) -> Result<Vec<DiscoveredDevice>, String> {
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let receiver = daemon
        .browse(OPERIT_SERVICE_TYPE)
        .map_err(|e| e.to_string())?;

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut devices = BTreeMap::<String, (MdnsIpv4Rank, DiscoveredDevice)>::new();

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let full_name = info.get_fullname().to_string();
                let mut addresses: Vec<Ipv4Addr> = info
                    .get_addresses()
                    .iter()
                    .filter_map(|address| match address {
                        std::net::IpAddr::V4(address) => Some(*address),
                        std::net::IpAddr::V6(_) => None,
                    })
                    .collect();
                if addresses.is_empty() {
                    continue;
                }
                addresses.sort_by_key(mdns_ipv4_rank);
                let selected_address = addresses[0];
                let selected_rank = mdns_ipv4_rank(&selected_address);
                let props = info.get_properties();
                let device_id = props
                    .get("deviceId")
                    .ok_or_else(|| {
                        format!("mDNS service missing deviceId: {}", info.get_fullname())
                    })?
                    .val_str()
                    .to_string();
                let display_name = props
                    .get("displayName")
                    .ok_or_else(|| {
                        format!("mDNS service missing displayName: {}", info.get_fullname())
                    })?
                    .val_str()
                    .to_string();
                let platform = props
                    .get("platform")
                    .ok_or_else(|| {
                        format!("mDNS service missing platform: {}", info.get_fullname())
                    })?
                    .val_str()
                    .to_string();
                let model = props
                    .get("model")
                    .ok_or_else(|| format!("mDNS service missing model: {}", info.get_fullname()))?
                    .val_str()
                    .to_string();
                let token_hash = props
                    .get("tokenHash")
                    .ok_or_else(|| {
                        format!("mDNS service missing tokenHash: {}", info.get_fullname())
                    })?
                    .val_str()
                    .to_string();
                let version = props
                    .get("version")
                    .ok_or_else(|| {
                        format!("mDNS service missing version: {}", info.get_fullname())
                    })?
                    .val_str()
                    .to_string();
                let base_url = format!("http://{}:{}", selected_address, info.get_port());

                let device = DiscoveredDevice {
                    device_id,
                    display_name,
                    platform,
                    model,
                    base_url,
                    hostname: info.get_hostname().to_string(),
                    port: info.get_port(),
                    token_hash,
                    version,
                };
                match devices.get_mut(&full_name) {
                    Some((current_rank, current_device)) if selected_rank < *current_rank => {
                        *current_rank = selected_rank;
                        *current_device = device;
                    }
                    Some(_) => {}
                    None => {
                        devices.insert(full_name, (selected_rank, device));
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
        .collect::<Vec<_>>())
}

fn mdns_ipv4_rank(address: &Ipv4Addr) -> (u8, [u8; 4]) {
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

#[derive(serde::Serialize)]
pub struct DiscoveredDevice {
    pub device_id: String,
    pub display_name: String,
    pub platform: String,
    pub model: String,
    pub base_url: String,
    pub hostname: String,
    pub port: u16,
    pub token_hash: String,
    pub version: String,
}
