use chrono::{Local, TimeZone};
use operit_host_api::{
    BluetoothBleNotificationData as HostBluetoothBleNotificationData,
    BluetoothBleNotificationEntry as HostBluetoothBleNotificationEntry,
    BluetoothBleServiceData as HostBluetoothBleServiceData,
    BluetoothBleServicesData as HostBluetoothBleServicesData,
    BluetoothDeviceData as HostBluetoothDeviceData, BluetoothReadData as HostBluetoothReadData,
    BluetoothScanResultData as HostBluetoothScanResultData,
    BluetoothScannedDeviceData as HostBluetoothScannedDeviceData,
    BluetoothSessionData as HostBluetoothSessionData, BluetoothStateData as HostBluetoothStateData,
    BluetoothTransferData as HostBluetoothTransferData,
    MusicPlaybackStatus as HostMusicPlaybackStatus, WebVisitLinkData,
};
pub use operit_plugin_sdk::js_sdk::results::*;
use std::collections::{BTreeMap, HashMap};
pub trait FromHostResult<T>: Sized {
    /// Converts a host result into its public plugin SDK representation.
    fn from_host(value: T) -> Self;
}
impl FromHostResult<WebVisitLinkData> for LinkData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: WebVisitLinkData) -> Self {
        Self {
            url: value.url,
            text: value.text,
        }
    }
}
impl FromHostResult<HostMusicPlaybackStatus> for MusicPlaybackResultData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostMusicPlaybackStatus) -> Self {
        Self {
            state: value.state,
            source: JsOptional::from_nullable_option(value.source),
            sourceType: JsOptional::from_nullable_option(value.sourceType),
            title: JsOptional::from_nullable_option(value.title),
            artist: JsOptional::from_nullable_option(value.artist),
            durationMs: JsOptional::from_nullable_option(value.durationMs),
            positionMs: value.positionMs,
            bufferedPositionMs: value.bufferedPositionMs,
            volume: value.volume,
            r#loop: value.loopPlayback,
            message: value.message,
        }
    }
}
impl FromHostResult<HostBluetoothStateData> for BluetoothStateData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothStateData) -> Self {
        Self {
            supported: value.supported,
            enabled: value.enabled,
            state: value.state,
        }
    }
}
impl FromHostResult<HostBluetoothDeviceData> for BluetoothDeviceData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothDeviceData) -> Self {
        Self {
            name: value.name,
            address: value.address,
            r#type: value.r#type,
            bondState: value.bondState,
        }
    }
}
impl FromHostResult<HostBluetoothScannedDeviceData> for BluetoothScannedDeviceData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothScannedDeviceData) -> Self {
        Self {
            name: value.name,
            address: value.address,
            r#type: value.r#type,
            bondState: value.bondState,
            source: value.source,
            rssi: value.rssi,
        }
    }
}
impl FromHostResult<HostBluetoothScanResultData> for BluetoothScanResultData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothScanResultData) -> Self {
        Self {
            devices: value
                .devices
                .into_iter()
                .map(BluetoothScannedDeviceData::from_host)
                .collect(),
            durationMs: value.durationMs,
            includesBle: value.includesBle,
        }
    }
}
impl FromHostResult<HostBluetoothSessionData> for BluetoothSessionData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothSessionData) -> Self {
        Self {
            sessionId: value.sessionId,
            address: value.address,
            mode: value.mode,
        }
    }
}
impl FromHostResult<HostBluetoothTransferData> for BluetoothTransferData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothTransferData) -> Self {
        Self {
            sessionId: value.sessionId,
            bytesWritten: value.bytesWritten,
        }
    }
}
impl FromHostResult<HostBluetoothReadData> for BluetoothReadData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothReadData) -> Self {
        Self {
            sessionId: value.sessionId,
            bytesRead: value.bytesRead,
            text: value.text,
            dataBase64: value.dataBase64,
        }
    }
}
impl FromHostResult<HostBluetoothBleServiceData> for BluetoothBleServiceData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothBleServiceData) -> Self {
        Self {
            uuid: value.uuid,
            characteristics: value
                .characteristics
                .into_iter()
                .map(|item| BluetoothBleCharacteristicData {
                    uuid: item.uuid,
                    properties: item.properties,
                })
                .collect(),
        }
    }
}
impl FromHostResult<HostBluetoothBleServicesData> for BluetoothBleServicesData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothBleServicesData) -> Self {
        Self {
            sessionId: value.sessionId,
            services: value
                .services
                .into_iter()
                .map(BluetoothBleServiceData::from_host)
                .collect(),
        }
    }
}
impl FromHostResult<HostBluetoothBleNotificationEntry> for BluetoothBleNotificationEntry {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothBleNotificationEntry) -> Self {
        Self {
            characteristicUuid: value.characteristicUuid,
            bytesRead: value.bytesRead,
            text: value.text,
            dataBase64: value.dataBase64,
            timestamp: value.timestamp,
        }
    }
}
impl FromHostResult<HostBluetoothBleNotificationData> for BluetoothBleNotificationData {
    ///Converts the host value into its public SDK result type.
    fn from_host(value: HostBluetoothBleNotificationData) -> Self {
        Self {
            sessionId: value.sessionId,
            notifications: value
                .notifications
                .into_iter()
                .map(BluetoothBleNotificationEntry::from_host)
                .collect(),
        }
    }
}
