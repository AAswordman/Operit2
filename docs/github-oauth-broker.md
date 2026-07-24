# GitHub OAuth Broker

## App Callback Process

The OAuth broker is not a browser or login host. Each application owns its browser presentation and completion handling, while Core owns the credential and transaction completion:

1. The application prepares its completion mechanism and tells Core the completion destination.
2. Core creates the broker transaction for that destination.
3. The application presents the returned authorization URL.
4. The application captures navigation to the completion destination.
5. The application returns the completed URL to Core.
6. Core validates the transaction and claims the encrypted result once.

Flutter owns this process through the visible market OAuth WebView. The CLI owns it through a temporary loopback listener and prints the authorization URL for the user to open. This is application code, not an `operit-host-api` capability. Neither client has Worker completion polling, an App Link, a platform intent receiver, or an EventChannel callback receiver.

## Purpose

Operit 2 signs users in through the GitHub OAuth broker at `api.operit.app`. The broker owns the OAuth application secret, PKCE verifier, browser callback, and the short-lived encrypted delivery record.

## Client Flow

1. The application selects and prepares its completion destination.
2. Call the generated `GitHubOAuthBrokerService.startLogin(completionRedirectUri)` API. Core calls `POST /oauth/github/start`, then stores the transaction ID and delivery credential in `HostSecretStore` and returns only `attemptId`, `authorizationUrl`, and `expiresAt`.
3. Present `authorizationUrl` in the application-owned browser surface. The Flutter market login dialog owns a visible `WebViewWidget`; the CLI prints the URL and waits on its temporary loopback listener.
4. Call the generated `GitHubOAuthBrokerService.completeLogin(completion)` API. Core checks the destination, transaction ID, and status, removes the pending credential, calls `POST /oauth/github/claim` once, and saves the GitHub session.

Flutter calls the generated Dart proxy. The CLI calls the generated Rust proxy. Neither client serializes `market auth` command strings, parses command stdout, or constructs a raw CoreLink request. The Flutter dialog intercepts the registered completion navigation before the Worker completion page is rendered. Neither client contains a GitHub OAuth client ID or client secret, and neither accepts a pasted personal access token as its sign-in path.

## Compatibility

The broker endpoints are independent from the legacy Android endpoint at `/market/v2/auth/github`, which remains available for released Android clients.

## Verification

Run the Rust broker service tests from `core/`:

```sh
cargo test -p operit-runtime github_oauth_broker
```
