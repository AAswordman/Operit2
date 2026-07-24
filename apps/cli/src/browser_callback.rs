use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use reqwest::Url;

type CallbackResult<T> = Result<T, String>;

/// Coordinates the CLI's loopback OAuth completion callback.
pub(crate) struct CliOAuthCallback;

/// Holds the one loopback listener reserved for a CLI OAuth transaction.
pub(crate) struct PreparedOAuthCallback {
    completionRedirectUri: Url,
    listener: TcpListener,
}

impl CliOAuthCallback {
    /// Reserves a loopback listener and returns the redirect URI for a new Core transaction.
    pub(crate) fn prepare(
        requestedCompletionRedirect: &str,
    ) -> CallbackResult<PreparedOAuthCallback> {
        let requestedRedirect = parseRequestedCompletionRedirect(requestedCompletionRedirect)?;
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| {
            format!("browser callback loopback listener could not start: {error}")
        })?;
        listener.set_nonblocking(true).map_err(|error| {
            format!("browser callback loopback listener could not configure: {error}")
        })?;
        let port = listener
            .local_addr()
            .map_err(|error| {
                format!("browser callback loopback listener has no local address: {error}")
            })?
            .port();
        let completionRedirectUri = loopbackCompletionRedirect(&requestedRedirect, port)?;
        Ok(PreparedOAuthCallback {
            completionRedirectUri,
            listener,
        })
    }
}

impl PreparedOAuthCallback {
    /// Returns the loopback callback destination for Core's broker transaction.
    pub(crate) fn completionRedirectUri(&self) -> String {
        self.completionRedirectUri.to_string()
    }

    /// Waits for the browser to reach the reserved loopback callback destination.
    pub(crate) fn waitForCompletion(self, expiresAtMillis: i64) -> CallbackResult<String> {
        if currentMillis() >= expiresAtMillis {
            return Err("browser callback has expired".to_string());
        }
        waitForLoopbackCompletion(
            &self.listener,
            &self.completionRedirectUri,
            expiresAtMillis,
        )
    }
}

/// Parses the callback path that an upstream transaction will redirect to.
fn parseRequestedCompletionRedirect(raw: &str) -> CallbackResult<Url> {
    let url = Url::parse(raw).map_err(|error| {
        format!("browser callback completion destination is invalid: {error}")
    })?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.path().is_empty()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("browser callback completion destination is invalid".to_string());
    }
    Ok(url)
}

/// Creates the loopback destination preserving the requested callback path.
fn loopbackCompletionRedirect(requestedRedirect: &Url, port: u16) -> CallbackResult<Url> {
    let mut loopback = Url::parse("http://127.0.0.1").map_err(|error| {
        format!("browser callback loopback URL could not initialize: {error}")
    })?;
    loopback
        .set_port(Some(port))
        .map_err(|()| "browser callback loopback port is invalid".to_string())?;
    loopback.set_path(requestedRedirect.path());
    Ok(loopback)
}

/// Waits for an accepted loopback callback before the caller-defined expiration time.
fn waitForLoopbackCompletion(
    listener: &TcpListener,
    completionRedirectUri: &Url,
    expiresAtMillis: i64,
) -> CallbackResult<String> {
    loop {
        if currentMillis() >= expiresAtMillis {
            return Err("browser callback timed out".to_string());
        }
        match listener.accept() {
            Ok((stream, _)) => {
                if let Some(completionUrl) = readLoopbackCompletion(stream, completionRedirectUri)?
                {
                    return Ok(completionUrl);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                return Err(format!(
                    "browser callback loopback listener could not accept request: {error}"
                ));
            }
        }
    }
}

/// Reads one loopback request and returns it only when it targets the prepared callback path.
fn readLoopbackCompletion(
    mut stream: TcpStream,
    completionRedirectUri: &Url,
) -> CallbackResult<Option<String>> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| {
            format!("browser callback loopback request could not configure: {error}")
        })?;
    let mut buffer = [0_u8; 8192];
    let byteCount = stream.read(&mut buffer).map_err(|error| {
        format!("browser callback loopback request could not be read: {error}")
    })?;
    let request = std::str::from_utf8(&buffer[..byteCount]).map_err(|error| {
        format!("browser callback loopback request is not UTF-8: {error}")
    })?;
    let requestTarget = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "browser callback loopback request has no target".to_string())?;
    let completionUrl = loopbackRequestUrl(completionRedirectUri, requestTarget)?;
    if !sameCallbackDestination(&completionUrl, completionRedirectUri) {
        writeLoopbackResponse(
            &mut stream,
            "404 Not Found",
            "Browser callback destination is not registered.",
        )?;
        return Ok(None);
    }
    writeLoopbackResponse(
        &mut stream,
        "200 OK",
        "Browser callback complete. You can return to Operit.",
    )?;
    Ok(Some(completionUrl.to_string()))
}

/// Resolves an HTTP request target against the prepared loopback origin.
fn loopbackRequestUrl(completionRedirectUri: &Url, requestTarget: &str) -> CallbackResult<Url> {
    if !requestTarget.starts_with('/') {
        return Err("browser callback loopback request target is invalid".to_string());
    }
    let origin = completionRedirectUri.origin().ascii_serialization();
    Url::parse(&format!("{origin}{requestTarget}"))
        .map_err(|error| format!("browser callback loopback URL is invalid: {error}"))
}

/// Checks that a received URL matches the registered scheme, host, port, and path.
fn sameCallbackDestination(received: &Url, registered: &Url) -> bool {
    received.scheme() == registered.scheme()
        && received.host_str() == registered.host_str()
        && received.port_or_known_default() == registered.port_or_known_default()
        && received.path() == registered.path()
}

/// Writes the small browser-visible response returned by a loopback callback endpoint.
fn writeLoopbackResponse(stream: &mut TcpStream, status: &str, message: &str) -> CallbackResult<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n<!doctype html><html><body>{message}</body></html>"
    );
    stream.write_all(response.as_bytes()).map_err(|error| {
        format!("browser callback loopback response could not be written: {error}")
    })?;
    stream.flush().map_err(|error| {
        format!("browser callback loopback response could not be flushed: {error}")
    })
}

/// Returns the current Unix epoch in milliseconds for CLI callback deadline checks.
fn currentMillis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves the upstream callback path when reserving a loopback destination.
    #[test]
    fn oauth_callback_reserves_loopback_completion_path() {
        let requested = Url::parse("https://api.operit.app/oauth/github/complete")
            .expect("requested callback URL should parse");
        let actual = loopbackCompletionRedirect(&requested, 43120)
            .expect("loopback callback URL should build");

        assert_eq!(
            actual.as_str(),
            "http://127.0.0.1:43120/oauth/github/complete"
        );
    }

    /// Gives Core the temporary CLI loopback destination for a broker transaction.
    #[test]
    fn cli_oauth_callback_prepares_loopback_completion_destination() {
        let callback = CliOAuthCallback::prepare("https://api.operit.app/oauth/github/complete")
            .expect("CLI callback should reserve a loopback listener");
        let completion = Url::parse(&callback.completionRedirectUri())
            .expect("CLI callback completion URL should parse");

        assert_eq!(completion.scheme(), "http");
        assert_eq!(completion.host_str(), Some("127.0.0.1"));
        assert_eq!(completion.path(), "/oauth/github/complete");
        assert!(completion.port().is_some());
    }

    /// Rejects a loopback request that targets a path outside its prepared callback destination.
    #[test]
    fn oauth_callback_rejects_unregistered_path() {
        let registered = Url::parse("http://127.0.0.1:43120/oauth/github/complete")
            .expect("registered callback URL should parse");
        let received = loopbackRequestUrl(&registered, "/other?status=complete")
            .expect("received callback URL should parse");

        assert!(!sameCallbackDestination(&received, &registered));
    }
}
