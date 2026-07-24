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

    pub fn register(
        &mut self,
        port: u16,
        properties: HashMap<String, String>,
    ) -> Result<(), String> {
        let hostname = operit_mdns_hostname(port);
        let instance_name = operit_mdns_instance_name(port);
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

fn operit_mdns_hostname(port: u16) -> String {
    format!("operit-{}-{port}.local.", std::process::id())
}

fn operit_mdns_instance_name(port: u16) -> String {
    format!("operit-{}-{port}", std::process::id())
}
