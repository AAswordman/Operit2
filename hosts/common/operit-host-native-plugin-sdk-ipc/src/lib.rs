#![allow(non_snake_case)]

mod framing;
mod memory;
pub mod pipes;
pub mod stream;

pub use framing::{readPluginSdkIpcFrame, writePluginSdkIpcFrame, PLUGIN_SDK_IPC_MAX_FRAME_BYTES};
pub use memory::MemoryPluginSdkIpcHost;

/// Qualifies a native IPC address with a reversible, transport-safe instance suffix.
/// An empty instance preserves the installed platform endpoint.
pub fn pluginSdkIpcInstanceName(base: &str, instance: &str) -> String {
    use std::fmt::Write;
    let mut name = base.to_owned();
    if !instance.is_empty() {
        name.push_str(".i");
        for byte in instance.bytes() {
            write!(name, "{byte:02x}").expect("writing to a String cannot fail");
        }
    }
    name
}

#[cfg(test)]
mod instance_tests {
    use super::pluginSdkIpcInstanceName;

    #[test]
    fn instance_names_preserve_defaults_and_do_not_alias() {
        assert_eq!(
            pluginSdkIpcInstanceName("org.operit.PluginSdk", ""),
            "org.operit.PluginSdk"
        );
        let a = pluginSdkIpcInstanceName("org.operit.PluginSdk", "a/b");
        let b = pluginSdkIpcInstanceName("org.operit.PluginSdk", "a_b");
        assert_ne!(a, b);
        assert_eq!(a, pluginSdkIpcInstanceName("org.operit.PluginSdk", "a/b"));
        assert!(a.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.'));
        assert_ne!(
            pluginSdkIpcInstanceName("sdk", "手机"),
            pluginSdkIpcInstanceName("sdk", "手机2")
        );
    }
}
