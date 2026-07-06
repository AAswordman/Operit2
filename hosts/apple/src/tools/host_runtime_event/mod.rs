use operit_host_api::{
    HostResult, HostRuntimeEventHost, HostRuntimeEventRegistration, HostRuntimeEventSink,
};

#[derive(Clone, Debug, Default)]
pub struct AppleHostRuntimeEventHost;

impl AppleHostRuntimeEventHost {
    pub fn new() -> Self {
        Self
    }
}

pub struct AppleHostRuntimeEventRegistration;

impl HostRuntimeEventRegistration for AppleHostRuntimeEventRegistration {}

impl HostRuntimeEventHost for AppleHostRuntimeEventHost {
    fn startHostRuntimeEventStream(
        &self,
        _sink: HostRuntimeEventSink,
    ) -> HostResult<Box<dyn HostRuntimeEventRegistration>> {
        Ok(Box::new(AppleHostRuntimeEventRegistration))
    }
}
