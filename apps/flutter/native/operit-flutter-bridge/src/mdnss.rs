use std::collections::HashMap;

use mdns_sd::{ServiceDaemon, ServiceInfo};

pub const OPERIT_SERVICE_TYPE: &str = "_operit._tcp.local.";

pub struct MdnsHandle {
    daemon: ServiceDaemon,
    service_type: String,
    fullname: Option<String>,
}

impl MdnsHandle {
    pub fn new() -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        Ok(Self {
            daemon,
            service_type: OPERIT_SERVICE_TYPE.to_string(),
            fullname: None,
        })
    }

    /// Publishes the CoreNode endpoint under its stable device identity.
    pub fn register(
        &mut self,
        device_id: &str,
        port: u16,
        properties: HashMap<String, String>,
    ) -> Result<(), String> {
        let hostname = operit_mdns_hostname(device_id);
        let instance_name = operit_mdns_instance_name(device_id);
        let service_info = ServiceInfo::new(
            &self.service_type,
            &instance_name,
            &hostname,
            "",
            port,
            properties,
        )
        .map_err(|e| e.to_string())?
        .enable_addr_auto();
        self.fullname = Some(service_info.get_fullname().to_string());
        self.daemon
            .register(service_info)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn unregister(&self) -> Result<(), String> {
        let fullname = self
            .fullname
            .as_ref()
            .ok_or_else(|| "mDNS service is not registered".to_string())?;
        self.daemon
            .unregister(fullname)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Derives a stable hostname from the CoreNode identity.
fn operit_mdns_hostname(device_id: &str) -> String {
    format!("operit-{device_id}.local.")
}

/// Derives a service name that cannot collide merely because PID and port match.
fn operit_mdns_instance_name(device_id: &str) -> String {
    format!("operit-{device_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies independent CoreNodes retain distinct mDNS identities on one port.
    #[test]
    fn mdns_names_use_device_identity() {
        let first = "core-1789368866635-838e5f2bc3f04c99a8d056c0dfefd31d";
        let second = "core-1789836891334-3114a1ee43cc48f787a5e2603684b828";
        assert_ne!(operit_mdns_instance_name(first), operit_mdns_instance_name(second));
        assert_eq!(operit_mdns_hostname(first), format!("operit-{first}.local."));
    }
}
