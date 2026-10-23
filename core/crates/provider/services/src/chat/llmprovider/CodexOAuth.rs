use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::chat::llmprovider::AIService::AiServiceError;

/// Public Codex CLI OAuth client id used by ChatGPT subscription login.
pub const CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_OAUTH_ISSUER: &str = "https://auth.openai.com";
const CODEX_OAUTH_SCOPE: &str = "openid profile email offline_access";
const CODEX_BROWSER_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const CODEX_DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const CODEX_DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";
const CODEX_TOKEN_REFRESH_SKEW_MILLIS: i64 = 60_000;

/// Tokens persisted after a ChatGPT Codex login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CodexOAuthTokens {
    pub accessToken: String,
    pub refreshToken: String,
    pub idToken: String,
    pub accountId: String,
    pub email: String,
    pub expiresAtMillis: i64,
}

/// Authorization URL and verifier for one browser login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CodexBrowserAuthorization {
    pub authorizationUrl: String,
    pub redirectUri: String,
    pub state: String,
    pub codeVerifier: String,
}

/// User code shown while a headless device login is pending.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CodexDeviceAuthorization {
    pub verificationUrl: String,
    pub userCode: String,
    pub deviceAuthId: String,
    pub intervalSeconds: i64,
}

#[derive(Deserialize)]
struct CodexTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
    #[serde(default)]
    id_token: String,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct CodexDeviceUserCodeResponse {
    device_auth_id: String,
    #[serde(default)]
    user_code: String,
    #[serde(default)]
    usercode: String,
    #[serde(default)]
    interval: Value,
}

#[derive(Deserialize)]
struct CodexDeviceTokenResponse {
    authorization_code: String,
    code_verifier: String,
}

/// Builds the browser authorization request for the fixed Codex callback port.
pub fn browser_authorization() -> Result<CodexBrowserAuthorization, AiServiceError> {
    let codeVerifier = random_pkce_verifier()?;
    let state = random_token(32)?;
    let challenge = pkce_challenge(&codeVerifier);
    let mut url = reqwest::Url::parse(&format!("{CODEX_OAUTH_ISSUER}/oauth/authorize"))
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    url.query_pairs_mut()
        .append_pair("client_id", CODEX_OAUTH_CLIENT_ID)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", CODEX_BROWSER_REDIRECT_URI)
        .append_pair("scope", CODEX_OAUTH_SCOPE)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("prompt", "login")
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true")
        .append_pair("originator", "operit");
    Ok(CodexBrowserAuthorization {
        authorizationUrl: url.to_string(),
        redirectUri: CODEX_BROWSER_REDIRECT_URI.to_string(),
        state,
        codeVerifier,
    })
}

/// Exchanges one browser callback code for Codex tokens.
pub async fn exchange_browser_code(
    code: &str,
    codeVerifier: &str,
) -> Result<CodexOAuthTokens, AiServiceError> {
    exchange_authorization_code(code, CODEX_BROWSER_REDIRECT_URI, codeVerifier).await
}

/// Starts a headless device-code login.
pub async fn start_device_authorization() -> Result<CodexDeviceAuthorization, AiServiceError> {
    let response = super::ChatHttpClient::chatHttpClient()
        .post(format!(
            "{CODEX_OAUTH_ISSUER}/api/accounts/deviceauth/usercode"
        ))
        .json(&serde_json::json!({ "client_id": CODEX_OAUTH_CLIENT_ID }))
        .send()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    if !status.is_success() {
        return Err(AiServiceError::RequestFailed(format!(
            "Codex device authorization failed: {status}: {body}"
        )));
    }
    let parsed: CodexDeviceUserCodeResponse = serde_json::from_str(&body)
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    let userCode = if parsed.user_code.trim().is_empty() {
        parsed.usercode
    } else {
        parsed.user_code
    };
    if userCode.trim().is_empty() || parsed.device_auth_id.trim().is_empty() {
        return Err(AiServiceError::RequestFailed(
            "Codex device authorization did not return a user code".to_string(),
        ));
    }
    Ok(CodexDeviceAuthorization {
        verificationUrl: CODEX_DEVICE_VERIFICATION_URL.to_string(),
        userCode,
        deviceAuthId: parsed.device_auth_id,
        intervalSeconds: device_poll_interval_seconds(&parsed.interval),
    })
}

/// Polls one device login once. `Ok(None)` means the user has not approved it yet.
pub async fn poll_device_authorization_once(
    deviceAuthId: &str,
    userCode: &str,
) -> Result<Option<CodexOAuthTokens>, AiServiceError> {
    let response = super::ChatHttpClient::chatHttpClient()
        .post(format!(
            "{CODEX_OAUTH_ISSUER}/api/accounts/deviceauth/token"
        ))
        .json(&serde_json::json!({
            "device_auth_id": deviceAuthId,
            "user_code": userCode,
        }))
        .send()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    let status = response.status();
    if status.as_u16() == 403 || status.as_u16() == 404 {
        return Ok(None);
    }
    let body = response
        .text()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    if !status.is_success() {
        return Err(AiServiceError::RequestFailed(format!(
            "Codex device token polling failed: {status}: {body}"
        )));
    }
    let parsed: CodexDeviceTokenResponse = serde_json::from_str(&body)
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    exchange_authorization_code(
        &parsed.authorization_code,
        CODEX_DEVICE_REDIRECT_URI,
        &parsed.code_verifier,
    )
    .await
    .map(Some)
}

/// Refreshes an access token when it is missing or inside the refresh window.
pub async fn ensure_fresh_tokens(
    tokens: CodexOAuthTokens,
) -> Result<CodexOAuthTokens, AiServiceError> {
    let now = current_millis()?;
    if !tokens.accessToken.trim().is_empty()
        && tokens.expiresAtMillis > now + CODEX_TOKEN_REFRESH_SKEW_MILLIS
    {
        return Ok(tokens);
    }
    if tokens.refreshToken.trim().is_empty() {
        return Err(AiServiceError::RequestFailed(
            "Codex authorization has expired".to_string(),
        ));
    }
    let response = super::ChatHttpClient::chatHttpClient()
        .post(format!("{CODEX_OAUTH_ISSUER}/oauth/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", tokens.refreshToken.as_str()),
            ("client_id", CODEX_OAUTH_CLIENT_ID),
        ])
        .send()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    if !status.is_success() {
        return Err(AiServiceError::RequestFailed(format!(
            "Codex token refresh failed: {status}: {body}"
        )));
    }
    let parsed: CodexTokenResponse =
        serde_json::from_str(&body).map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    tokens_from_response(parsed, Some(&tokens))
}

/// Reads the ChatGPT account id required by the Codex responses endpoint.
pub fn account_id_from_tokens(tokens: &CodexOAuthTokens) -> Result<String, AiServiceError> {
    if !tokens.accountId.trim().is_empty() {
        return Ok(tokens.accountId.clone());
    }
    for token in [&tokens.idToken, &tokens.accessToken] {
        if let Some(accountId) = account_id_from_jwt(token) {
            return Ok(accountId);
        }
    }
    Err(AiServiceError::RequestFailed(
        "Codex authorization is missing the ChatGPT account id".to_string(),
    ))
}

async fn exchange_authorization_code(
    code: &str,
    redirectUri: &str,
    codeVerifier: &str,
) -> Result<CodexOAuthTokens, AiServiceError> {
    let response = super::ChatHttpClient::chatHttpClient()
        .post(format!("{CODEX_OAUTH_ISSUER}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CODEX_OAUTH_CLIENT_ID),
            ("code", code),
            ("redirect_uri", redirectUri),
            ("code_verifier", codeVerifier),
        ])
        .send()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
    if !status.is_success() {
        return Err(AiServiceError::RequestFailed(format!(
            "Codex token exchange failed: {status}: {body}"
        )));
    }
    let parsed: CodexTokenResponse =
        serde_json::from_str(&body).map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    tokens_from_response(parsed, None)
}

fn tokens_from_response(
    response: CodexTokenResponse,
    previous: Option<&CodexOAuthTokens>,
) -> Result<CodexOAuthTokens, AiServiceError> {
    let mut tokens = CodexOAuthTokens {
        accessToken: response.access_token,
        refreshToken: response.refresh_token,
        idToken: response.id_token,
        accountId: String::new(),
        email: String::new(),
        expiresAtMillis: current_millis()?
            + response.expires_in.unwrap_or(3600).max(0) * 1000,
    };
    if tokens.refreshToken.trim().is_empty() {
        if let Some(previous) = previous {
            tokens.refreshToken = previous.refreshToken.clone();
        }
    }
    if let Some(claims) = jwt_claims(&tokens.idToken).or_else(|| jwt_claims(&tokens.accessToken)) {
        tokens.accountId = account_id_from_claims(&claims).unwrap_or_default();
        tokens.email = claims
            .get("email")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
    }
    if tokens.accountId.trim().is_empty() {
        if let Some(previous) = previous {
            tokens.accountId = previous.accountId.clone();
        }
    }
    if tokens.email.trim().is_empty() {
        if let Some(previous) = previous {
            tokens.email = previous.email.clone();
        }
    }
    if tokens.accountId.trim().is_empty() {
        return Err(AiServiceError::RequestFailed(
            "Codex token response did not include a ChatGPT account id".to_string(),
        ));
    }
    Ok(tokens)
}

fn account_id_from_jwt(token: &str) -> Option<String> {
    jwt_claims(token).and_then(|claims| account_id_from_claims(&claims))
}

fn account_id_from_claims(claims: &Value) -> Option<String> {
    claims
        .get("https://api.openai.com/auth")
        .and_then(|auth| auth.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            claims
                .get("chatgpt_account_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            claims
                .get("https://api.openai.com/auth")
                .and_then(|auth| auth.pointer("/organizations/0/id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            claims
                .pointer("/organizations/0/id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|value| !value.trim().is_empty())
}

fn jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| {
            let mut padded = payload.to_string();
            while padded.len() % 4 != 0 {
                padded.push('=');
            }
            URL_SAFE.decode(padded)
        })
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn device_poll_interval_seconds(value: &Value) -> i64 {
    match value {
        Value::Number(number) => number.as_i64().filter(|seconds| *seconds > 0).unwrap_or(5),
        Value::String(text) => text.trim().parse::<i64>().ok().filter(|seconds| *seconds > 0).unwrap_or(5),
        _ => 5,
    }
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn random_pkce_verifier() -> Result<String, AiServiceError> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let bytes = random_bytes(64)?;
    Ok(bytes
        .into_iter()
        .map(|byte| ALPHABET[(byte as usize) % ALPHABET.len()] as char)
        .collect())
}

fn random_token(length: usize) -> Result<String, AiServiceError> {
    Ok(URL_SAFE_NO_PAD.encode(random_bytes(length)?))
}

fn random_bytes(length: usize) -> Result<Vec<u8>, AiServiceError> {
    let mut bytes = vec![0_u8; length];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
    Ok(bytes)
}

fn current_millis() -> Result<i64, AiServiceError> {
    operit_host_api::TimeUtils::tryCurrentTimeMillis().map_err(AiServiceError::RequestFailed)
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    use super::{
        account_id_from_claims, jwt_claims, tokens_from_response, CodexOAuthTokens,
        CodexTokenResponse,
    };

    fn jwt_with_payload(payload: &str) -> String {
        format!(
            "header.{}.signature",
            URL_SAFE_NO_PAD.encode(payload.as_bytes())
        )
    }

    #[test]
    fn account_id_comes_from_openai_auth_claim() {
        let claims = serde_json::json!({
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acct_nested",
                "organizations": [{ "id": "org_nested" }]
            },
            "organizations": [{ "id": "org_top" }]
        });
        assert_eq!(
            account_id_from_claims(&claims).as_deref(),
            Some("acct_nested")
        );
    }

    #[test]
    fn padded_jwt_payload_decodes() {
        let payload = r#"{"https://api.openai.com/auth":{"chatgpt_account_id":"acct_pad"},"email":"a@b.c"}"#;
        let token = jwt_with_payload(payload);
        let claims = jwt_claims(&token).expect("claims");
        assert_eq!(
            account_id_from_claims(&claims).as_deref(),
            Some("acct_pad")
        );
    }

    #[test]
    fn refresh_keeps_previous_refresh_token_and_account() {
        let previous = CodexOAuthTokens {
            accessToken: "old-access".to_string(),
            refreshToken: "keep-refresh".to_string(),
            idToken: String::new(),
            accountId: "acct_saved".to_string(),
            email: "saved@example.com".to_string(),
            expiresAtMillis: 1,
        };
        let tokens = tokens_from_response(
            CodexTokenResponse {
                access_token: "new-access".to_string(),
                refresh_token: String::new(),
                id_token: String::new(),
                expires_in: Some(3600),
            },
            Some(&previous),
        )
        .expect("tokens");
        assert_eq!(tokens.refreshToken, "keep-refresh");
        assert_eq!(tokens.accountId, "acct_saved");
        assert_eq!(tokens.email, "saved@example.com");
        assert_eq!(tokens.accessToken, "new-access");
    }
}
