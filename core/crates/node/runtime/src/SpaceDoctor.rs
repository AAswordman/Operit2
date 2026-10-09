//! Public facade over the Space doctor so hosts and the CLI can run the
//! shared-Space diagnostics without reaching into crate internals. The
//! diagnosis itself lives with the replication code it inspects.
pub use crate::NodeSpaceService::space_doctor::diagnose;
pub use crate::NodeSpaceService::space_doctor::{
    JoinHealth, JournalHealth, PairHealth, ProjectionHealth, RepairReport,
    SpaceDoctorCounters, SpaceDoctorFinding, SpaceHealthReport,
};
