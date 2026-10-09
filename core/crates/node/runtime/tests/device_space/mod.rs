// Device-space contract tests share the real router test Host and scheduler.
// Included as library unit tests so private protocol boundaries need not be made public.
use super::*;
use crate::RuntimeRemoteLinkService::{RuntimeRemoteLinkService, SpaceJoinStatus};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/device_space/fixtures.rs"
));

mod join_lifecycle {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/join_lifecycle.rs"
    ));
}
mod transports {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/transports.rs"
    ));
}
mod reviewer_assignment {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/reviewer_assignment.rs"
    ));
}
mod space_merge {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/space_merge.rs"
    ));
}
mod cancellation {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/cancellation.rs"
    ));
}
mod protocol_boundaries {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/protocol_boundaries.rs"
    ));
}
mod persistence_contracts {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/persistence_contracts.rs"
    ));
}
mod routing_contracts {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/routing_contracts.rs"
    ));
}
mod policy_contracts {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/policy_contracts.rs"
    ));
}
mod approving_exit {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/approving_exit.rs"
    ));
}
mod doctor {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/device_space/doctor.rs"
    ));
}
