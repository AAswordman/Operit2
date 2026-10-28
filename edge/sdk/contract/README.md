# operit-edge-contract

Shared address contract for Edge Services.

This crate contains stable Edge object and property identifiers only. It does
not implement Host capabilities, Link dispatch, transport carriers, or app
behavior. Both the Edge Node and typed Edge Proxy may depend on it without
creating a dependency between those implementations.

