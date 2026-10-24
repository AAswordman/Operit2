//! Checks the production stream owner without building the Flutter application.
#![allow(non_snake_case)]

#[path = "../src/BridgeTransport.rs"]
mod BridgeTransport;

mod BridgeExports {
    /// Extracts a stream panic message at the same boundary as the production exports.
    pub(crate) fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(message) = payload.downcast_ref::<&str>() {
            return message.to_string();
        }
        if let Some(message) = payload.downcast_ref::<String>() {
            return message.clone();
        }
        "Non-string stream panic".to_string()
    }
}

#[path = "../src/AndroidJniTaskScheduler.rs"]
mod AndroidJniTaskScheduler;
