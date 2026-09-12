# operit-proxy-edge

Typed projection for Edge Services.

The Proxy owns Link request construction and response/event decoding internally.
Consumers use typed methods such as `setDigitalOutput` and
`watchDigitalOutput`; they do not construct `CoreCallRequest`, `CoreValue`, or
`CoreEvent` directly. The production crate depends on the shared Edge contract,
Host DTOs, and Link client traits, but not on a local Edge Node implementation.

