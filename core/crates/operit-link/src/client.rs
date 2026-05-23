use crate::protocol::{CoreCallRequest, CoreCallResponse, CoreEvent, CoreLinkError, CoreWatchRequest};

pub trait CoreLinkClient {
    fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse;

    #[allow(non_snake_case)]
    fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>;
}
