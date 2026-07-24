#![allow(non_snake_case)]

use operit_host_api::HostManager::HostManager;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{HostSecretStore, HttpHost, HttpRequestData};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::data::preferences::GitHubAuthPreferences::{GitHubAuthPreferences, GitHubUser};

const GITHUB_OAUTH_BROKER_START_URL: &str = "https://api.operit.app/oauth/github/start";
const GITHUB_OAUTH_BROKER_CLAIM_URL: &str = "https://api.operit.app/oauth/github/claim";
const GITHUB_OAUTH_PENDING_SECRET_KEY: &str = "market.github-oauth-pending";

/// Starts and completes the Core-owned portion of one GitHub OAuth broker transaction.
#[derive(Clone)]
pub struct GitHubOAuthBrokerService {
    context: HostManager,
}

/// Contains the public transaction details an application needs to continue login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct GitHubOAuthBrokerLoginStart {
    pub attemptId: String,
    pub authorizationUrl: String,
    pub expiresAt: i64,
}

/// Identifies the browser callback that Core must validate and claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct GitHubOAuthBrokerLoginCompletion {
    pub attemptId: String,
    pub completionUrl: String,
}

/// Describes the GitHub identity persisted after a successful one-time broker claim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct GitHubOAuthBrokerLoginResult {
    pub githubId: String,
    pub login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubOAuthBrokerStartResponse {
    ok: bool,
    transaction_id: String,
    delivery_credential: String,
    authorization_url: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubOAuthBrokerClaimResponse {
    ok: bool,
    status: String,
    access_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    user: Option<GitHubOAuthBrokerUser>,
}

#[derive(Debug, Deserialize)]
struct GitHubOAuthBrokerUser {
    id: u64,
    login: String,
    avatar_url: String,
}

#[derive(Debug)]
struct GitHubOAuthBrokerClaimResult {
    access_token: String,
    token_type: String,
    scope: String,
    user: GitHubOAuthBrokerUser,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubOAuthPendingAttempt {
    attempt_id: String,
    transaction_id: String,
    delivery_credential: String,
    completion_redirect_uri: String,
    expires_at: i64,
}

#[derive(Debug)]
enum GitHubOAuthCompletionStatus {
    Complete,
    Denied,
    Error(String),
}

impl GitHubOAuthBrokerService {
    /// Creates the broker service with the Core context that owns HTTP and secret storage.
    pub fn getInstance(context: &HostManager) -> Self {
        Self {
            context: context.clone(),
        }
    }

    /// Starts one broker transaction after an application prepares its completion destination.
    pub fn startLogin(
        &self,
        completionRedirectUri: String,
    ) -> Result<GitHubOAuthBrokerLoginStart, String> {
        let completion_redirect_uri = parse_completion_redirect_uri(&completionRedirectUri)?;
        let transaction = self.start_transaction(completion_redirect_uri.as_str())?;
        if transaction.expires_at <= currentTimeMillis() {
            return Err("GitHub OAuth broker returned an expired transaction".to_string());
        }
        let attempt = GitHubOAuthPendingAttempt {
            attempt_id: Uuid::new_v4().to_string(),
            transaction_id: transaction.transaction_id,
            delivery_credential: transaction.delivery_credential,
            completion_redirect_uri: completion_redirect_uri.to_string(),
            expires_at: transaction.expires_at,
        };
        self.write_pending_attempt(&attempt)?;
        Ok(GitHubOAuthBrokerLoginStart {
            attemptId: attempt.attempt_id,
            authorizationUrl: transaction.authorization_url,
            expiresAt: attempt.expires_at,
        })
    }

    /// Validates one browser completion URL, claims its transaction once, and persists the session.
    pub fn completeLogin(
        &self,
        completion: GitHubOAuthBrokerLoginCompletion,
    ) -> Result<GitHubOAuthBrokerLoginResult, String> {
        let attempt = self.read_pending_attempt()?;
        if attempt.attempt_id != completion.attemptId {
            return Err("GitHub OAuth callback attempt does not match".to_string());
        }
        if attempt.expires_at <= currentTimeMillis() {
            self.clear_pending_attempt()?;
            return Err("GitHub OAuth callback has expired".to_string());
        }
        match validate_completion_url(
            &completion.completionUrl,
            &attempt.completion_redirect_uri,
            &attempt.transaction_id,
        )? {
            GitHubOAuthCompletionStatus::Complete => {}
            GitHubOAuthCompletionStatus::Denied => {
                self.clear_pending_attempt()?;
                return Err("GitHub authorization was cancelled".to_string());
            }
            GitHubOAuthCompletionStatus::Error(error) => {
                self.clear_pending_attempt()?;
                return Err(error);
            }
        }
        self.clear_pending_attempt()?;
        let result =
            self.claim_transaction(&attempt.transaction_id, &attempt.delivery_credential)?;
        let user_info = GitHubUser {
            id: result.user.id.to_string(),
            login: result.user.login,
            avatarUrl: result.user.avatar_url,
            ..GitHubUser::default()
        };
        GitHubAuthPreferences::getInstance()
            .saveAuthInfo(
                &result.access_token,
                &result.token_type,
                Some(&user_info),
                Some(&result.scope),
            )
            .map_err(|error| error.to_string())?;
        Ok(GitHubOAuthBrokerLoginResult {
            githubId: user_info.id,
            login: user_info.login,
        })
    }

    /// Sends one JSON request through the Core-owned HTTP host.
    fn request_json<T: DeserializeOwned>(
        &self,
        url: &str,
        payload: serde_json::Value,
    ) -> Result<T, String> {
        let body = serde_json::to_vec(&payload)
            .map_err(|error| format!("GitHub OAuth request could not encode: {error}"))?;
        let response = self
            .http_host()?
            .executeHttpRequest(HttpRequestData {
                url: url.to_string(),
                method: "POST".to_string(),
                headers: vec![
                    ("Accept".to_string(), "application/json".to_string()),
                    ("Content-Type".to_string(), "application/json".to_string()),
                ],
                body,
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 30,
                readTimeoutSeconds: 30,
                followRedirects: false,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            })
            .map_err(|error| format!("GitHub OAuth broker request failed: {error}"))?;
        if response.statusCode != 200 {
            return Err(format!(
                "GitHub OAuth broker request failed: HTTP {}",
                response.statusCode
            ));
        }
        serde_json::from_slice(&response.body)
            .map_err(|error| format!("GitHub OAuth broker response is invalid: {error}"))
    }

    /// Starts one short-lived GitHub OAuth broker transaction.
    fn start_transaction(
        &self,
        completion_redirect_uri: &str,
    ) -> Result<GitHubOAuthBrokerStartResponse, String> {
        let response = self.request_json(
            GITHUB_OAUTH_BROKER_START_URL,
            serde_json::json!({ "completionRedirectUri": completion_redirect_uri }),
        )?;
        validate_start_response(response)
    }

    /// Claims one completed GitHub OAuth broker transaction.
    fn claim_transaction(
        &self,
        transaction_id: &str,
        delivery_credential: &str,
    ) -> Result<GitHubOAuthBrokerClaimResult, String> {
        let response = self.request_json(
            GITHUB_OAUTH_BROKER_CLAIM_URL,
            serde_json::json!({
                "transactionId": transaction_id,
                "deliveryCredential": delivery_credential,
            }),
        )?;
        validate_claim_response(response)
    }

    /// Returns the Core-owned HTTP host.
    fn http_host(&self) -> Result<&dyn HttpHost, String> {
        self.context
            .httpHost
            .as_deref()
            .ok_or_else(|| "HttpHost is required for GitHub authorization".to_string())
    }

    /// Returns the Core-owned secret store for the pending delivery credential.
    fn secret_store(&self) -> Result<&dyn HostSecretStore, String> {
        self.context
            .hostSecretStore
            .as_deref()
            .ok_or_else(|| "HostSecretStore is required for GitHub authorization".to_string())
    }

    /// Writes the only pending callback attempt, replacing an abandoned earlier transaction.
    fn write_pending_attempt(&self, attempt: &GitHubOAuthPendingAttempt) -> Result<(), String> {
        let content = serde_json::to_vec(attempt)
            .map_err(|error| format!("GitHub OAuth pending attempt could not encode: {error}"))?;
        self.secret_store()?
            .writeSecret(GITHUB_OAUTH_PENDING_SECRET_KEY, &content)
            .map_err(|error| format!("GitHub OAuth pending attempt could not save: {error}"))
    }

    /// Reads the pending callback attempt whose delivery credential remains inside Core storage.
    fn read_pending_attempt(&self) -> Result<GitHubOAuthPendingAttempt, String> {
        let content = self
            .secret_store()?
            .readSecret(GITHUB_OAUTH_PENDING_SECRET_KEY)
            .map_err(|error| format!("GitHub OAuth pending attempt could not read: {error}"))?
            .ok_or_else(|| "GitHub OAuth callback attempt is not available".to_string())?;
        serde_json::from_slice(&content)
            .map_err(|error| format!("GitHub OAuth pending attempt is invalid: {error}"))
    }

    /// Removes the locally stored delivery credential after the callback reaches a terminal state.
    fn clear_pending_attempt(&self) -> Result<(), String> {
        self.secret_store()?
            .deleteSecret(GITHUB_OAUTH_PENDING_SECRET_KEY)
            .map_err(|error| format!("GitHub OAuth pending attempt could not clear: {error}"))
    }
}

/// Parses and validates the callback destination prepared by an application.
fn parse_completion_redirect_uri(raw: &str) -> Result<Url, String> {
    let redirect_uri = Url::parse(raw)
        .map_err(|error| format!("GitHub OAuth completion destination is invalid: {error}"))?;
    if !matches!(redirect_uri.scheme(), "http" | "https")
        || redirect_uri.host_str().is_none()
        || redirect_uri.path().is_empty()
        || redirect_uri.query().is_some()
        || redirect_uri.fragment().is_some()
        || !redirect_uri.username().is_empty()
        || redirect_uri.password().is_some()
    {
        return Err("GitHub OAuth completion destination is invalid".to_string());
    }
    Ok(redirect_uri)
}

/// Validates one broker start response before its delivery credential enters Core secret storage.
fn validate_start_response(
    response: GitHubOAuthBrokerStartResponse,
) -> Result<GitHubOAuthBrokerStartResponse, String> {
    if !response.ok {
        return Err("GitHub OAuth broker rejected the start transaction".to_string());
    }
    require_value(&response.transaction_id, "transactionId")?;
    require_value(&response.delivery_credential, "deliveryCredential")?;
    require_value(&response.authorization_url, "authorizationUrl")?;
    if response.expires_at <= 0 {
        return Err("GitHub OAuth broker start response is missing expiresAt".to_string());
    }
    Ok(response)
}

/// Validates one completed broker claim response before a GitHub session is persisted.
fn validate_claim_response(
    response: GitHubOAuthBrokerClaimResponse,
) -> Result<GitHubOAuthBrokerClaimResult, String> {
    if !response.ok {
        return Err("GitHub OAuth broker rejected the transaction".to_string());
    }
    if response.status != "complete" {
        return Err(format!(
            "Unsupported GitHub OAuth broker status: {}",
            response.status
        ));
    }
    let access_token = require_optional_value(response.access_token, "accessToken")?;
    let token_type = require_optional_value(response.token_type, "tokenType")?;
    let scope = require_optional_value(response.scope, "scope")?;
    let user = response
        .user
        .ok_or_else(|| "GitHub OAuth broker response is missing user".to_string())?;
    if user.id == 0 {
        return Err("GitHub OAuth broker response has invalid user.id".to_string());
    }
    require_value(&user.login, "user.login")?;
    require_value(&user.avatar_url, "user.avatar_url")?;
    Ok(GitHubOAuthBrokerClaimResult {
        access_token,
        token_type,
        scope,
        user,
    })
}

/// Validates the callback URL returned by an application.
fn validate_completion_url(
    completion_url: &str,
    completion_redirect_uri: &str,
    transaction_id: &str,
) -> Result<GitHubOAuthCompletionStatus, String> {
    let completion_url = Url::parse(completion_url)
        .map_err(|error| format!("GitHub OAuth completion URL is invalid: {error}"))?;
    let completion_redirect_uri = parse_completion_redirect_uri(completion_redirect_uri)?;
    if completion_url.scheme() != completion_redirect_uri.scheme()
        || completion_url.host_str() != completion_redirect_uri.host_str()
        || completion_url.port_or_known_default() != completion_redirect_uri.port_or_known_default()
        || completion_url.path() != completion_redirect_uri.path()
    {
        return Err("GitHub OAuth completion destination does not match".to_string());
    }
    if completion_url
        .query_pairs()
        .find_map(|(key, value)| (key == "transactionId").then_some(value.into_owned()))
        .as_deref()
        != Some(transaction_id)
    {
        return Err("GitHub OAuth completion transaction does not match".to_string());
    }
    let status = completion_url
        .query_pairs()
        .find_map(|(key, value)| (key == "status").then_some(value.into_owned()));
    match status.as_deref() {
        Some("complete") => Ok(GitHubOAuthCompletionStatus::Complete),
        Some("denied") => Ok(GitHubOAuthCompletionStatus::Denied),
        Some("error") => completion_url
            .query_pairs()
            .find_map(|(key, value)| (key == "error").then_some(value.into_owned()))
            .map(GitHubOAuthCompletionStatus::Error)
            .ok_or_else(|| "GitHub OAuth completion error is missing".to_string()),
        _ => Err("GitHub OAuth completion status is invalid".to_string()),
    }
}

/// Requires one non-empty string field from a broker response.
fn require_value(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("GitHub OAuth broker response is missing {name}"));
    }
    Ok(())
}

/// Requires one present non-empty optional field from a completed broker response.
fn require_optional_value(value: Option<String>, name: &str) -> Result<String, String> {
    let value = value.ok_or_else(|| format!("GitHub OAuth broker response is missing {name}"))?;
    require_value(&value, name)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{
        HostError, HostResult, HttpDownloadControl, HttpDownloadProgressCallback,
        HttpDownloadRequest, HttpDownloadResult, HttpResponseData,
    };
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Stores the pending delivery credential entirely in test memory.
    #[derive(Default)]
    struct TestHostSecretStore {
        content: Mutex<Option<Vec<u8>>>,
    }

    impl HostSecretStore for TestHostSecretStore {
        /// Reads the only secret used by the broker test.
        fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
            if key != GITHUB_OAUTH_PENDING_SECRET_KEY {
                return Err(HostError::new("unexpected test secret key"));
            }
            Ok(self
                .content
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .clone())
        }

        /// Replaces the only secret used by the broker test.
        fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
            if key != GITHUB_OAUTH_PENDING_SECRET_KEY {
                return Err(HostError::new("unexpected test secret key"));
            }
            *self
                .content
                .lock()
                .map_err(|error| HostError::new(error.to_string()))? = Some(content.to_vec());
            Ok(())
        }

        /// Clears the only secret used by the broker test.
        fn deleteSecret(&self, key: &str) -> HostResult<()> {
            if key != GITHUB_OAUTH_PENDING_SECRET_KEY {
                return Err(HostError::new("unexpected test secret key"));
            }
            *self
                .content
                .lock()
                .map_err(|error| HostError::new(error.to_string()))? = None;
            Ok(())
        }
    }

    /// Supplies fixed broker HTTP responses and records each request.
    struct TestHttpHost {
        responses: Mutex<VecDeque<HttpResponseData>>,
        requests: Mutex<Vec<HttpRequestData>>,
    }

    impl TestHttpHost {
        /// Creates a test HTTP host with ordered broker responses.
        fn new(responses: Vec<HttpResponseData>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl HttpHost for TestHttpHost {
        /// Records one broker request and returns the next configured response.
        fn executeHttpRequest(&self, request: HttpRequestData) -> HostResult<HttpResponseData> {
            self.requests
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .push(request);
            self.responses
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .pop_front()
                .ok_or_else(|| HostError::new("unexpected broker HTTP request"))
        }

        /// Rejects file-download calls because the broker test only issues buffered requests.
        fn downloadFiles(
            &self,
            _request: HttpDownloadRequest,
            _control: HttpDownloadControl,
            _onProgress: HttpDownloadProgressCallback,
        ) -> HostResult<HttpDownloadResult> {
            Err(HostError::new("unexpected broker download request"))
        }
    }

    /// Builds a successful HTTP response containing one JSON broker payload.
    fn json_broker_response(payload: serde_json::Value) -> HttpResponseData {
        HttpResponseData {
            finalUrl: GITHUB_OAUTH_BROKER_START_URL.to_string(),
            statusCode: 200,
            statusMessage: "OK".to_string(),
            headers: Vec::new(),
            body: serde_json::to_vec(&payload).expect("test broker payload should encode"),
        }
    }

    /// Creates a service with only the HTTP and secret capabilities needed by the broker.
    fn test_service(
        http_host: Arc<TestHttpHost>,
        secret_store: Arc<TestHostSecretStore>,
    ) -> GitHubOAuthBrokerService {
        let mut context = HostManager::new();
        context.httpHost = Some(http_host);
        context.hostSecretStore = Some(secret_store);
        GitHubOAuthBrokerService::getInstance(&context)
    }

    /// Decodes the transaction identifiers and authorization URL returned by the broker.
    #[test]
    fn parses_github_oauth_broker_start_response() {
        let response: GitHubOAuthBrokerStartResponse = serde_json::from_str(
            r#"{
                "ok": true,
                "transactionId": "transaction-123",
                "deliveryCredential": "credential-456",
                "authorizationUrl": "https://github.com/login/oauth/authorize?state=state-789",
                "expiresAt": 1770000000000
            }"#,
        )
        .expect("broker start response should decode");
        let response =
            validate_start_response(response).expect("broker start response should validate");

        assert_eq!(response.transaction_id, "transaction-123");
        assert_eq!(response.delivery_credential, "credential-456");
        assert_eq!(response.expires_at, 1770000000000);
    }

    /// Decodes a complete broker claim only when all required fields are present.
    #[test]
    fn parses_github_oauth_broker_claim_response() {
        let response: GitHubOAuthBrokerClaimResponse = serde_json::from_str(
            r#"{
                "ok": true,
                "status": "complete",
                "accessToken": "github-token",
                "tokenType": "bearer",
                "scope": "notifications,public_repo,user:email,read:user",
                "user": {
                    "id": 12345,
                    "login": "operit-user",
                    "avatar_url": "https://avatars.githubusercontent.com/u/12345"
                }
            }"#,
        )
        .expect("broker claim response should decode");
        let response =
            validate_claim_response(response).expect("broker claim response should validate");

        assert_eq!(response.access_token, "github-token");
        assert_eq!(response.user.login, "operit-user");
    }

    /// Accepts a completed callback URL for the registered destination and current transaction.
    #[test]
    fn validates_github_oauth_broker_completion_url() {
        let status = validate_completion_url(
            "https://api.operit.app/oauth/github/complete?transactionId=transaction-123&status=complete",
            "https://api.operit.app/oauth/github/complete",
            "transaction-123",
        )
        .expect("completion URL should validate");

        assert!(matches!(status, GitHubOAuthCompletionStatus::Complete));
    }

    /// Rejects a callback URL that belongs to another transaction.
    #[test]
    fn rejects_github_oauth_broker_completion_url_for_another_transaction() {
        let error = validate_completion_url(
            "https://api.operit.app/oauth/github/complete?transactionId=other-transaction&status=complete",
            "https://api.operit.app/oauth/github/complete",
            "transaction-123",
        )
        .expect_err("completion URL must belong to the current transaction");

        assert!(error.contains("does not match"), "error was: {error}");
    }

    /// Rejects a completion URL that does not reach the registered callback destination.
    #[test]
    fn rejects_github_oauth_broker_completion_url_for_another_destination() {
        let error = validate_completion_url(
            "https://example.com/oauth/github/complete?transactionId=transaction-123&status=complete",
            "https://api.operit.app/oauth/github/complete",
            "transaction-123",
        )
        .expect_err("completion URL must reach the registered destination");

        assert!(
            error.contains("destination does not match"),
            "error was: {error}"
        );
    }

    /// Stores the delivery credential only in the secret store and clears it after cancellation.
    #[test]
    fn starts_and_cancels_github_oauth_broker_attempt() {
        let expires_at = currentTimeMillis() + 60_000;
        let http_host = Arc::new(TestHttpHost::new(vec![json_broker_response(
            serde_json::json!({
                "ok": true,
                "transactionId": "transaction-123",
                "deliveryCredential": "credential-456",
                "authorizationUrl": "https://github.com/login/oauth/authorize?state=state-789",
                "expiresAt": expires_at,
            }),
        )]));
        let secret_store = Arc::new(TestHostSecretStore::default());
        let service = test_service(http_host.clone(), secret_store.clone());

        let start = service
            .startLogin("https://api.operit.app/oauth/github/complete".to_string())
            .expect("broker start should succeed");
        let pending = secret_store
            .readSecret(GITHUB_OAUTH_PENDING_SECRET_KEY)
            .expect("pending attempt should be readable")
            .expect("pending attempt should exist");
        let pending: GitHubOAuthPendingAttempt =
            serde_json::from_slice(&pending).expect("pending attempt should decode");
        assert_eq!(pending.attempt_id, start.attemptId);
        assert_eq!(pending.transaction_id, "transaction-123");
        assert_eq!(pending.delivery_credential, "credential-456");
        assert_eq!(pending.expires_at, expires_at);

        let error = service
            .completeLogin(GitHubOAuthBrokerLoginCompletion {
                attemptId: start.attemptId,
                completionUrl: "https://api.operit.app/oauth/github/complete?transactionId=transaction-123&status=denied".to_string(),
            })
            .expect_err("denied callback should stop the broker attempt");
        assert_eq!(error, "GitHub authorization was cancelled");
        assert!(
            secret_store
                .readSecret(GITHUB_OAUTH_PENDING_SECRET_KEY)
                .expect("pending attempt should be readable")
                .is_none(),
            "denied callback should clear the delivery credential"
        );

        let requests = http_host
            .requests
            .lock()
            .expect("recorded broker requests should be readable");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, GITHUB_OAUTH_BROKER_START_URL);
        let request_payload: serde_json::Value =
            serde_json::from_slice(&requests[0].body).expect("start request should decode");
        assert_eq!(
            request_payload["completionRedirectUri"],
            "https://api.operit.app/oauth/github/complete"
        );
    }

    /// Keeps the pending credential key compatible with every native host secret store.
    #[test]
    fn uses_platform_compatible_github_oauth_broker_pending_secret_key() {
        assert!(
            !GITHUB_OAUTH_PENDING_SECRET_KEY.is_empty()
                && GITHUB_OAUTH_PENDING_SECRET_KEY.chars().all(|character| {
                    character.is_ascii_alphanumeric()
                        || character == '.'
                        || character == '_'
                        || character == '-'
                }),
            "pending secret key must be accepted by native host secret stores"
        );
    }
}
