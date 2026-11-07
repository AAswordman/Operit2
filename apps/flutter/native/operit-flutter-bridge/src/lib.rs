#![allow(non_snake_case)]
//! Flutter's transport boundary around the platform-independent Core runtime.
//! PlatformRuntimeFactory selects host construction; PlatformRuntimeAbi selects ABI modules.

mod BridgeCodec;
mod BridgeExports;
mod BridgeRuntime;
mod BridgeTransport;
mod FlutterHostAdapters;
mod FlutterOwnerCapabilities;
#[cfg(not(target_arch = "wasm32"))]
mod LocalRuntimeTransport;
mod PlatformRuntimeAbi;
mod PlatformRuntimeFactory;

pub use BridgeExports::*;
pub use BridgeRuntime::OperitFlutterBridge;

pub(crate) use BridgeRuntime::current_time_millis_u64;
