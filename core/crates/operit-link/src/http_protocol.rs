use serde::{Deserialize, Serialize};

use crate::{
    CoreCallRequest, CoreEvent, CorePushRequest, CoreWatchRequest,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkCallEnvelope {
    pub request: CoreCallRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchEnvelope {
    pub request: CoreWatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelEnvelope {
    pub channelId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelOpenEnvelope {
    pub channelId: String,
    pub subscriptionId: String,
    pub request: CoreWatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelCloseEnvelope {
    pub channelId: String,
    pub subscriptionId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelOpenResponse {
    pub subscriptionId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelCloseResponse {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelEvent {
    pub subscriptionId: String,
    pub event: CoreEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushOpenEnvelope {
    pub pushId: String,
    pub request: CorePushRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushCloseEnvelope {
    pub pushId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushOpenResponse {
    pub pushId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushItemResponse {
    pub pushId: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushCloseResponse {
    pub pushId: String,
}
