# Access

Access crates own the node-to-node control and transport surface.

This domain covers identity, pairing, authentication, session state, peer-link carriers, web control endpoints, and remote access server tasks. It passes authenticated peer transport handles into the node domain; it does not decide route targets or execute business runtime calls.

## Crates

- `runtime`: current aggregate access runtime containing identity, pairing, auth, peer link, remote server, discovery, and web control code.

## Target Split

The target layout separates this aggregate into `identity`, `auth`, `pairing`, `discovery`, `peer-link`, `server`, and `web` crates.
