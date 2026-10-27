//! Read models for consent lists, review routing and applicant responses.
//! These projections carry correlation facts, never admission authority.
use super::{CoreSpace, Record, SpaceJoinRequest};
use serde::{Deserialize, Serialize};

/// Membership acknowledged by a gateway; adoption still verifies the current control export.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct MembershipReceipt {
    pub space: CoreSpace,
}

/// Only member identities are needed to compare a consent with its committed admission.
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct SourceMembership {
    pub space: MemberIds,
}

/// A projection of the source Space without profiles, topology or policy history.
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MemberIds {
    pub members: Vec<String>,
}

/// Durable read model shared by lists, reviewer inboxes and diagnostic queries.
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ConsentProjection {
    pub request: SpaceJoinRequest,
    pub sourceSpaceId: String,
    pub sourceRevision: i64,
    pub targetSpaceId: String,
    pub source: SourceMembership,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailableSince: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approvedDecision: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decisionRevision: Option<i64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cancelRequested: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub submitted: bool,
}

impl From<&Record> for ConsentProjection {
    /// Copies only the request facts required by read-only consumers.
    fn from(record: &Record) -> Self {
        Self {
            request: record.request.clone(),
            sourceSpaceId: record.sourceSpaceId.clone(),
            sourceRevision: record.sourceRevision,
            targetSpaceId: record.targetSpaceId.clone(),
            source: SourceMembership { space: MemberIds { members: record.source.space.members.clone() } },
            unavailableSince: record.unavailableSince,
            approvedDecision: record.approvedDecision,
            decisionRevision: record.decisionRevision,
            cancelRequested: record.cancelRequested,
            submitted: record.submitted,
        }
    }
}

impl ConsentProjection {
    /// Binds this read model to the original applicant and its exact source generation.
    pub fn identity(&self) -> [&str; 5] {
        identity(&self.request, &self.sourceSpaceId, &self.targetSpaceId)
    }
}

/// Applicant-facing status; the applicant already owns its immutable submission.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JoinReply {
    pub request: SpaceJoinRequest,
    pub sourceSpaceId: String,
    pub sourceRevision: i64,
    pub targetSpaceId: String,
    pub accepted: Option<MembershipReceipt>,
    pub approvedDecision: Option<bool>,
    pub decisionRevision: Option<i64>,
}

impl JoinReply {
    /// Checks the response against the locally retained consent identity.
    pub fn validate(&self, expected: &ConsentProjection) -> Result<(), String> {
        if identity(&self.request, &self.sourceSpaceId, &self.targetSpaceId) != expected.identity()
            || self.sourceRevision != expected.sourceRevision {
            return Err("Join response identity mismatch".into());
        }
        Ok(())
    }
}

/// Returns correlation fields without decoding or copying any evidence.
pub(super) fn identity<'a>(request: &'a SpaceJoinRequest, source: &'a str, target: &'a str) -> [&'a str; 5] {
    [&request.requestId, &request.applicantDeviceId, &request.targetDeviceId, target, source]
}
