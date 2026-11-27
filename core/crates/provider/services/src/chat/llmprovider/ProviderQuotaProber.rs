#![allow(non_snake_case)]

use super::ProviderHttp::{self, RequestBuilder};
use http::header::{HeaderName, HeaderValue, ACCEPT, AUTHORIZATION};
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_model::ModelConfigData::{ApiProviderType, ProviderProfile};
use operit_model::ProviderQuota::{
    ProviderBalanceProtocol, ProviderQuotaProbeConfig, ProviderQuotaProbeOutcome,
    QuotaAccountKind, QuotaBalanceBreakdown, QuotaProbeStatus, QuotaWindow, QuotaWindowKind,
};
use serde_json::Value;
use url::Url;

const DEFAULT_DEEPSEEK_ENDPOINT: &str = "https://api.deepseek.com/v1/chat/completions";
const DEFAULT_OPENROUTER_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_ZHIPU_ENDPOINT: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";
const DEFAULT_MINIMAX_ENDPOINT: &str = "https://api.minimaxi.com/v1/chat/completions";
const DEFAULT_MOONSHOT_ENDPOINT: &str = "https://api.moonshot.cn/v1/chat/completions";
const DEFAULT_SILICONFLOW_ENDPOINT: &str = "https://api.siliconflow.cn/v1/chat/completions";
const CODEX_WHAM_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const NEW_API_QUOTA_UNITS_PER_USD: f64 = 500_000.0;

/// Resolves the deterministic balance/quota protocol for a provider without implicit fallback.
pub fn resolve_effective_protocol(
    provider: &ProviderProfile,
    config: Option<&ProviderQuotaProbeConfig>,
) -> ProviderBalanceProtocol {
    if let Some(cfg) = config {
        if cfg.protocol != ProviderBalanceProtocol::AUTO {
            return cfg.protocol;
        }
    }
    let endpoint = provider.endpoint.trim().to_ascii_lowercase();
    match provider.providerType {
        ApiProviderType::DEEPSEEK => ProviderBalanceProtocol::DEEPSEEK_BALANCE,
        ApiProviderType::SILICONFLOW => ProviderBalanceProtocol::SILICONFLOW_USER_INFO,
        ApiProviderType::OPENROUTER => ProviderBalanceProtocol::OPENROUTER_KEY,
        ApiProviderType::OPENAI_CODEX => ProviderBalanceProtocol::CODEX_WHAM_USAGE,
        ApiProviderType::ZHIPU => {
            if endpoint.contains("/coding") {
                ProviderBalanceProtocol::ZHIPU_CODING_PLAN
            } else {
                ProviderBalanceProtocol::ZHIPU_ACCOUNT_REPORT
            }
        }
        ApiProviderType::MINIMAX => ProviderBalanceProtocol::MINIMAX_TOKEN_PLAN,
        ApiProviderType::MOONSHOT => {
            if endpoint.contains("api.kimi.com/coding") {
                ProviderBalanceProtocol::KIMI_CODING_USAGES
            } else {
                ProviderBalanceProtocol::MOONSHOT_BALANCE
            }
        }
        ApiProviderType::LOCAL_MODEL
        | ApiProviderType::OLLAMA
        | ApiProviderType::LMSTUDIO
        | ApiProviderType::OPENAI_LOCAL => ProviderBalanceProtocol::LOCAL_COMPUTE,
        _ => ProviderBalanceProtocol::UNSUPPORTED,
    }
}

/// Returns the official billing/usage console URL for a provider profile when known.
pub fn console_url_for_provider(provider: &ProviderProfile) -> Option<String> {
    let endpoint = provider.endpoint.trim().to_ascii_lowercase();
    let url = match provider.providerType {
        ApiProviderType::DEEPSEEK => "https://platform.deepseek.com/usage",
        ApiProviderType::OPENROUTER => "https://openrouter.ai/settings/keys",
        ApiProviderType::ZHIPU => {
            if endpoint.contains("z.ai") {
                "https://z.ai/manage-apikey/apikey-list"
            } else {
                "https://bigmodel.cn/usercenter/proj-mgmt/apikeys"
            }
        }
        ApiProviderType::MINIMAX => {
            if endpoint.contains("minimax.io") {
                "https://platform.minimax.io/user-center/payment/balance"
            } else {
                "https://platform.minimaxi.com/user-center/payment/balance"
            }
        }
        ApiProviderType::MOONSHOT => {
            if endpoint.contains("kimi.com") {
                "https://www.kimi.com/code/console"
            } else if endpoint.contains("moonshot.ai") {
                "https://platform.moonshot.ai/console/account"
            } else {
                "https://platform.moonshot.cn/console/account"
            }
        }
        ApiProviderType::SILICONFLOW => "https://cloud.siliconflow.cn/expensebill",
        ApiProviderType::OPENAI_CODEX => "https://chatgpt.com/codex/settings/usage",
        ApiProviderType::OPENAI | ApiProviderType::OPENAI_RESPONSES => {
            "https://platform.openai.com/settings/organization/billing/overview"
        }
        ApiProviderType::ANTHROPIC => "https://console.anthropic.com/settings/billing",
        ApiProviderType::GOOGLE => "https://aistudio.google.com/spend",
        ApiProviderType::DOUBAO => "https://console.volcengine.com/finance/fund/management",
        ApiProviderType::ALIYUN => "https://billing-cost.console.aliyun.com/home",
        ApiProviderType::MISTRAL => "https://console.mistral.ai/billing",
        ApiProviderType::XAI => "https://console.x.ai/",
        _ => return None,
    };
    Some(url.to_string())
}

/// Optional OAuth credentials required when probing ChatGPT Codex usage quotas.
#[derive(Clone, Debug)]
pub struct CodexQuotaCredentials {
    pub accessToken: String,
    pub accountId: String,
    pub email: String,
}

/// Executes one deterministic HTTP quota/balance probe for a single key without mutating pool state.
pub async fn probe_single_key(
    provider: &ProviderProfile,
    api_key: &str,
    config: Option<&ProviderQuotaProbeConfig>,
    codex: Option<&CodexQuotaCredentials>,
) -> ProviderQuotaProbeOutcome {
    let protocol = resolve_effective_protocol(provider, config);
    let console_url = console_url_for_provider(provider);
    let key = api_key.trim();

    match protocol {
        ProviderBalanceProtocol::LOCAL_COMPUTE => ProviderQuotaProbeOutcome {
            accountKind: QuotaAccountKind::LOCAL_COMPUTE,
            protocol,
            status: QuotaProbeStatus::OK,
            plan: Some("Local Compute".to_string()),
            accountLabel: Some(provider.name.clone()),
            balance: None,
            windows: Vec::new(),
            consoleUrl: None,
            error: None,
        },
        ProviderBalanceProtocol::UNSUPPORTED => ProviderQuotaProbeOutcome {
            accountKind: QuotaAccountKind::UNSUPPORTED,
            protocol,
            status: QuotaProbeStatus::UNSUPPORTED,
            plan: None,
            accountLabel: None,
            balance: None,
            windows: Vec::new(),
            consoleUrl: console_url,
            error: Some(
                "This provider does not expose a key-queryable balance API; configure a Monthly Budget Cap or open the console."
                    .to_string(),
            ),
        },
        ProviderBalanceProtocol::CODEX_WHAM_USAGE => {
            let Some(creds) = codex else {
                return ProviderQuotaProbeOutcome {
                    accountKind: QuotaAccountKind::SUBSCRIPTION,
                    protocol,
                    status: QuotaProbeStatus::UNAUTHORIZED,
                    plan: None,
                    accountLabel: None,
                    balance: None,
                    windows: Vec::new(),
                    consoleUrl: console_url,
                    error: Some("Codex OAuth session is not connected".to_string()),
                };
            };
            let mut builder = ProviderHttp::get(CODEX_WHAM_USAGE_URL)
                .header(ACCEPT, HeaderValue::from_static("application/json"));
            if let Ok(auth_val) =
                HeaderValue::from_str(&format!("Bearer {}", creds.accessToken.trim()))
            {
                builder = builder.header(AUTHORIZATION, auth_val);
            }
            if let (Ok(acct_name), Ok(acct_val)) = (
                HeaderName::from_bytes(b"chatgpt-account-id"),
                HeaderValue::from_str(creds.accountId.trim()),
            ) {
                if !creds.accountId.trim().is_empty() {
                    builder = builder.header(acct_name, acct_val);
                }
            }
            match execute_get_text(builder).await {
                Ok((status, body)) => {
                    parse_codex_wham_usage(status, &body, &creds.email, console_url)
                }
                Err(error) => error_outcome(
                    QuotaAccountKind::SUBSCRIPTION,
                    protocol,
                    console_url,
                    error,
                ),
            }
        }
        ProviderBalanceProtocol::DEEPSEEK_BALANCE => {
            let origin = match origin_from_endpoint(&provider.endpoint, DEFAULT_DEEPSEEK_ENDPOINT)
            {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error)
                }
            };
            match send_bearer_get(&format!("{origin}/user/balance"), key).await {
                Ok((status, body)) => parse_deepseek_balance(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::OPENROUTER_KEY => {
            let origin =
                match origin_from_endpoint(&provider.endpoint, DEFAULT_OPENROUTER_ENDPOINT) {
                    Ok(origin) => origin,
                    Err(error) => {
                        return error_outcome(
                            QuotaAccountKind::BALANCE,
                            protocol,
                            console_url,
                            error,
                        )
                    }
                };
            match send_bearer_get(&format!("{origin}/api/v1/key"), key).await {
                Ok((status, body)) => parse_openrouter_key(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::OPENROUTER_CREDITS => {
            let origin =
                match origin_from_endpoint(&provider.endpoint, DEFAULT_OPENROUTER_ENDPOINT) {
                    Ok(origin) => origin,
                    Err(error) => {
                        return error_outcome(
                            QuotaAccountKind::BALANCE,
                            protocol,
                            console_url,
                            error,
                        )
                    }
                };
            match send_bearer_get(&format!("{origin}/api/v1/credits"), key).await {
                Ok((status, body)) => parse_openrouter_credits(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::ZHIPU_CODING_PLAN => {
            let origin = match origin_from_endpoint(&provider.endpoint, DEFAULT_ZHIPU_ENDPOINT) {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(
                        QuotaAccountKind::SUBSCRIPTION,
                        protocol,
                        console_url,
                        error,
                    )
                }
            };
            match send_bearer_get(&format!("{origin}/api/monitor/usage/quota/limit"), key).await {
                Ok((status, body)) => parse_zhipu_coding_plan(status, &body, console_url),
                Err(error) => error_outcome(
                    QuotaAccountKind::SUBSCRIPTION,
                    protocol,
                    console_url,
                    error,
                ),
            }
        }
        ProviderBalanceProtocol::ZHIPU_ACCOUNT_REPORT => {
            let origin = match origin_from_endpoint(&provider.endpoint, DEFAULT_ZHIPU_ENDPOINT) {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error)
                }
            };
            match send_bearer_get(
                &format!("{origin}/api/biz/account/query-customer-account-report"),
                key,
            )
            .await
            {
                Ok((status, body)) => parse_zhipu_account_report(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::MINIMAX_TOKEN_PLAN => {
            let origin = match origin_from_endpoint(&provider.endpoint, DEFAULT_MINIMAX_ENDPOINT)
            {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(
                        QuotaAccountKind::SUBSCRIPTION,
                        protocol,
                        console_url,
                        error,
                    )
                }
            };
            match send_bearer_get(&format!("{origin}/v1/token_plan/remains"), key).await {
                Ok((status, body)) => parse_minimax_token_plan(status, &body, console_url),
                Err(error) => error_outcome(
                    QuotaAccountKind::SUBSCRIPTION,
                    protocol,
                    console_url,
                    error,
                ),
            }
        }
        ProviderBalanceProtocol::MOONSHOT_BALANCE => {
            let origin =
                match origin_from_endpoint(&provider.endpoint, DEFAULT_MOONSHOT_ENDPOINT) {
                    Ok(origin) => origin,
                    Err(error) => {
                        return error_outcome(
                            QuotaAccountKind::BALANCE,
                            protocol,
                            console_url,
                            error,
                        )
                    }
                };
            match send_bearer_get(&format!("{origin}/v1/users/me/balance"), key).await {
                Ok((status, body)) => parse_moonshot_balance(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::KIMI_CODING_USAGES => {
            let origin = match origin_from_endpoint(
                &provider.endpoint,
                "https://api.kimi.com/coding/v1/chat/completions",
            ) {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(
                        QuotaAccountKind::SUBSCRIPTION,
                        protocol,
                        console_url,
                        error,
                    )
                }
            };
            let url = if provider
                .endpoint
                .to_ascii_lowercase()
                .contains("/coding")
            {
                format!("{origin}/coding/v1/usages")
            } else {
                format!("{origin}/v1/usages")
            };
            match send_bearer_get(&url, key).await {
                Ok((status, body)) => parse_kimi_coding_usages(status, &body, console_url),
                Err(error) => error_outcome(
                    QuotaAccountKind::SUBSCRIPTION,
                    protocol,
                    console_url,
                    error,
                ),
            }
        }
        ProviderBalanceProtocol::SILICONFLOW_USER_INFO => {
            let origin =
                match origin_from_endpoint(&provider.endpoint, DEFAULT_SILICONFLOW_ENDPOINT) {
                    Ok(origin) => origin,
                    Err(error) => {
                        return error_outcome(
                            QuotaAccountKind::BALANCE,
                            protocol,
                            console_url,
                            error,
                        )
                    }
                };
            match send_bearer_get(&format!("{origin}/v1/user/info"), key).await {
                Ok((status, body)) => parse_siliconflow_user_info(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::NEW_API_TOKEN => {
            let origin = match origin_from_endpoint(&provider.endpoint, "") {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error)
                }
            };
            match send_bearer_get(&format!("{origin}/api/usage/token"), key).await {
                Ok((status, body)) => parse_new_api_token(status, &body, console_url),
                Err(error) => error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error),
            }
        }
        ProviderBalanceProtocol::OPENAI_LEGACY_BILLING => {
            let origin = match origin_from_endpoint(&provider.endpoint, "") {
                Ok(origin) => origin,
                Err(error) => {
                    return error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, error)
                }
            };
            let sub_res =
                send_bearer_get(&format!("{origin}/v1/dashboard/billing/subscription"), key).await;
            let (start_date, end_date) = rolling_month_date_range_utc();
            let usage_url = format!(
                "{origin}/v1/dashboard/billing/usage?start_date={start_date}&end_date={end_date}"
            );
            let usage_res = send_bearer_get(&usage_url, key).await;
            match (sub_res, usage_res) {
                (Ok((sub_status, sub_body)), Ok((usage_status, usage_body))) => {
                    parse_openai_legacy_billing(
                        sub_status,
                        &sub_body,
                        usage_status,
                        &usage_body,
                        console_url,
                    )
                }
                (Err(err), _) | (_, Err(err)) => {
                    error_outcome(QuotaAccountKind::BALANCE, protocol, console_url, err)
                }
            }
        }
        ProviderBalanceProtocol::CUSTOM_HTTP_JSON | ProviderBalanceProtocol::AUTO => {
            let Some(cfg) = config else {
                return error_outcome(
                    QuotaAccountKind::BALANCE,
                    ProviderBalanceProtocol::CUSTOM_HTTP_JSON,
                    console_url,
                    "Custom HTTP probe requires customPath and customAmountJsonPath".to_string(),
                );
            };
            let path = cfg.customPath.as_deref().unwrap_or("").trim();
            let amount_path = cfg.customAmountJsonPath.as_deref().unwrap_or("").trim();
            if path.is_empty() || amount_path.is_empty() {
                return error_outcome(
                    QuotaAccountKind::BALANCE,
                    ProviderBalanceProtocol::CUSTOM_HTTP_JSON,
                    console_url,
                    "Custom HTTP probe requires non-empty path and JSON amount path".to_string(),
                );
            }
            let url = if path.starts_with("http://") || path.starts_with("https://") {
                path.to_string()
            } else {
                let origin = match origin_from_endpoint(&provider.endpoint, "") {
                    Ok(origin) => origin,
                    Err(error) => {
                        return error_outcome(
                            QuotaAccountKind::BALANCE,
                            ProviderBalanceProtocol::CUSTOM_HTTP_JSON,
                            console_url,
                            error,
                        )
                    }
                };
                if path.starts_with('/') {
                    format!("{origin}{path}")
                } else {
                    format!("{origin}/{path}")
                }
            };
            match send_bearer_get(&url, key).await {
                Ok((status, body)) => parse_custom_http_json(
                    status,
                    &body,
                    amount_path,
                    cfg.customCurrencyJsonPath.as_deref(),
                    cfg.customCurrency.as_deref(),
                    console_url,
                ),
                Err(error) => error_outcome(
                    QuotaAccountKind::BALANCE,
                    ProviderBalanceProtocol::CUSTOM_HTTP_JSON,
                    console_url,
                    error,
                ),
            }
        }
    }
}

async fn send_bearer_get(url: &str, key: &str) -> Result<(u16, String), String> {
    if key.is_empty() {
        return Err("API key is empty".to_string());
    }
    let mut builder =
        ProviderHttp::get(url).header(ACCEPT, HeaderValue::from_static("application/json"));
    let auth_value = HeaderValue::from_str(&format!("Bearer {key}"))
        .map_err(|error| format!("invalid API key header: {error}"))?;
    builder = builder.header(AUTHORIZATION, auth_value);
    execute_get_text(builder).await
}

async fn execute_get_text(builder: RequestBuilder) -> Result<(u16, String), String> {
    let response = builder.send().await.map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(|error| error.to_string())?;
    Ok((status, body))
}

fn origin_from_endpoint(endpoint: &str, default_endpoint: &str) -> Result<String, String> {
    let raw = if endpoint.trim().is_empty() {
        default_endpoint
    } else {
        endpoint.trim()
    };
    if raw.is_empty() {
        return Err("Provider API endpoint is empty".to_string());
    }
    let parsed = Url::parse(raw).map_err(|error| format!("Invalid provider URL {raw}: {error}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("Provider URL missing host: {raw}"))?;
    let mut origin = format!("{}://{host}", parsed.scheme());
    if let Some(port) = parsed.port() {
        origin.push_str(&format!(":{port}"));
    }
    Ok(origin)
}

fn error_outcome(
    account_kind: QuotaAccountKind,
    protocol: ProviderBalanceProtocol,
    console_url: Option<String>,
    error: String,
) -> ProviderQuotaProbeOutcome {
    ProviderQuotaProbeOutcome {
        accountKind: account_kind,
        protocol,
        status: QuotaProbeStatus::ERROR,
        plan: None,
        accountLabel: None,
        balance: None,
        windows: Vec::new(),
        consoleUrl: console_url,
        error: Some(error),
    }
}

fn http_status_failure(
    account_kind: QuotaAccountKind,
    protocol: ProviderBalanceProtocol,
    console_url: Option<String>,
    status: u16,
    body: &str,
) -> ProviderQuotaProbeOutcome {
    let probe_status = if status == 401 || status == 403 {
        QuotaProbeStatus::UNAUTHORIZED
    } else {
        QuotaProbeStatus::ERROR
    };
    let detail = extract_error_message(body)
        .unwrap_or_else(|| format!("HTTP {status}"));
    ProviderQuotaProbeOutcome {
        accountKind: account_kind,
        protocol,
        status: probe_status,
        plan: None,
        accountLabel: None,
        balance: None,
        windows: Vec::new(),
        consoleUrl: console_url,
        error: Some(format!("HTTP {status}: {detail}")),
    }
}

fn extract_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .pointer("/error/message")
        .or_else(|| value.pointer("/base_resp/status_msg"))
        .or_else(|| value.get("message"))
        .or_else(|| value.get("msg"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

pub fn parse_deepseek_balance(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::DEEPSEEK_BALANCE;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let is_available = root
        .get("is_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let infos = root
        .get("balance_infos")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut windows = Vec::new();
    let mut primary_currency = String::from("CNY");
    let mut primary_total = None;
    let mut primary_cash = None;
    let mut primary_granted = None;
    for info in &infos {
        let currency = info
            .get("currency")
            .and_then(Value::as_str)
            .unwrap_or("CNY")
            .to_string();
        let total = info.get("total_balance").and_then(value_to_f64);
        let granted = info.get("granted_balance").and_then(value_to_f64);
        let topped = info.get("topped_up_balance").and_then(value_to_f64);
        if primary_total.is_none() || (currency == "CNY" && primary_currency != "CNY") {
            primary_currency = currency.clone();
            primary_total = total;
            primary_cash = topped;
            primary_granted = granted;
        }
        if let Some(total_val) = total {
            windows.push(QuotaWindow {
                name: format!("{currency} Balance"),
                kind: QuotaWindowKind::BALANCE,
                display: format_currency_amount(&currency, total_val),
                used: None,
                remaining: Some(total_val),
                limit: None,
                usedPercent: None,
                unlimited: false,
                resetsAtMs: None,
                resetSecs: None,
                spanLabel: Some(currency.clone()),
                model: None,
                unit: Some(currency),
            });
        }
    }
    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("DeepSeek Pay-As-You-Go".to_string()),
        accountLabel: None,
        balance: Some(QuotaBalanceBreakdown {
            currency: primary_currency,
            totalBalance: primary_total,
            cashBalance: primary_cash,
            grantedBalance: primary_granted,
            usedAmount: None,
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: is_available,
            isFreeTier: false,
        }),
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_openrouter_key(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::OPENROUTER_KEY;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let data = root.get("data").unwrap_or(&root);
    let label = data
        .get("label")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let limit = data.get("limit").and_then(value_to_f64);
    let remaining = data.get("limit_remaining").and_then(value_to_f64);
    let limit_reset = data
        .get("limit_reset")
        .and_then(Value::as_str)
        .map(str::to_string);
    let usage = data.get("usage").and_then(value_to_f64);
    let usage_daily = data.get("usage_daily").and_then(value_to_f64);
    let usage_monthly = data.get("usage_monthly").and_then(value_to_f64);
    let is_free_tier = data
        .get("is_free_tier")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut windows = Vec::new();
    if let Some(cap) = limit {
        if cap > 0.0 {
            let rem = remaining.unwrap_or_else(|| (cap - usage.unwrap_or(0.0)).max(0.0));
            let used = (cap - rem).max(0.0);
            let pct = ((used / cap) * 100.0).clamp(0.0, 100.0);
            let kind = match limit_reset.as_deref() {
                Some("daily") => QuotaWindowKind::DAY_1,
                Some("weekly") => QuotaWindowKind::DAY_7,
                Some("monthly") => QuotaWindowKind::MONTH,
                _ => QuotaWindowKind::TOTAL,
            };
            windows.push(QuotaWindow {
                name: match limit_reset.as_deref() {
                    Some(cadence) => format!("Key Limit ({cadence})"),
                    None => "Key Spending Limit".to_string(),
                },
                kind,
                display: format!("{} / {}", format_currency_amount("USD", rem), format_currency_amount("USD", cap)),
                used: Some(used),
                remaining: Some(rem),
                limit: Some(cap),
                usedPercent: Some(pct),
                unlimited: false,
                resetsAtMs: None,
                resetSecs: None,
                spanLabel: limit_reset.clone(),
                model: None,
                unit: Some("USD".to_string()),
            });
        }
    }

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some(if is_free_tier {
            "OpenRouter Free Tier".to_string()
        } else {
            "OpenRouter Key".to_string()
        }),
        accountLabel: label,
        balance: Some(QuotaBalanceBreakdown {
            currency: "USD".to_string(),
            totalBalance: remaining,
            cashBalance: None,
            grantedBalance: None,
            usedAmount: usage,
            dailyUsed: usage_daily,
            monthlyUsed: usage_monthly,
            isAvailable: remaining.is_none_or(|r| r > 0.0),
            isFreeTier: is_free_tier,
        }),
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_openrouter_credits(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::OPENROUTER_CREDITS;
    if status == 403 {
        return ProviderQuotaProbeOutcome {
            accountKind: QuotaAccountKind::BALANCE,
            protocol,
            status: QuotaProbeStatus::UNAUTHORIZED,
            plan: None,
            accountLabel: None,
            balance: None,
            windows: Vec::new(),
            consoleUrl: console_url,
            error: Some(
                "OpenRouter /credits requires a Management Key (switch protocol to OPENROUTER_KEY for standard inference keys)."
                    .to_string(),
            ),
        };
    }
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let data = root.get("data").unwrap_or(&root);
    let total_credits = data.get("total_credits").and_then(value_to_f64).unwrap_or(0.0);
    let total_usage = data.get("total_usage").and_then(value_to_f64).unwrap_or(0.0);
    let remaining = (total_credits - total_usage).max(0.0);
    let pct = if total_credits > 0.0 {
        ((total_usage / total_credits) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("OpenRouter Credits".to_string()),
        accountLabel: None,
        balance: Some(QuotaBalanceBreakdown {
            currency: "USD".to_string(),
            totalBalance: Some(remaining),
            cashBalance: Some(remaining),
            grantedBalance: None,
            usedAmount: Some(total_usage),
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: remaining > 0.0,
            isFreeTier: false,
        }),
        windows: vec![QuotaWindow {
            name: "Account Credits".to_string(),
            kind: QuotaWindowKind::BALANCE,
            display: format!(
                "{} left of {}",
                format_currency_amount("USD", remaining),
                format_currency_amount("USD", total_credits)
            ),
            used: Some(total_usage),
            remaining: Some(remaining),
            limit: Some(total_credits),
            usedPercent: Some(pct),
            unlimited: false,
            resetsAtMs: None,
            resetSecs: None,
            spanLabel: Some("total".to_string()),
            model: None,
            unit: Some("USD".to_string()),
        }],
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_zhipu_coding_plan(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::ZHIPU_CODING_PLAN;
    if !(200..300).contains(&status) {
        return http_status_failure(
            QuotaAccountKind::SUBSCRIPTION,
            protocol,
            console_url,
            status,
            body,
        );
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::SUBSCRIPTION,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    if let Some(code) = root.get("code").and_then(Value::as_i64) {
        if code != 200 && code != 0 {
            let msg = root
                .get("msg")
                .or_else(|| root.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Zhipu Coding Plan returned an error");
            return ProviderQuotaProbeOutcome {
                accountKind: QuotaAccountKind::SUBSCRIPTION,
                protocol,
                status: if code == 401 || code == 403 {
                    QuotaProbeStatus::UNAUTHORIZED
                } else {
                    QuotaProbeStatus::ERROR
                },
                plan: None,
                accountLabel: None,
                balance: None,
                windows: Vec::new(),
                consoleUrl: console_url,
                error: Some(format!("Code {code}: {msg}")),
            };
        }
    }
    let data = root.get("data").unwrap_or(&root);
    let now_ms = currentTimeMillis();
    let mut windows = Vec::new();

    if let Some(limits) = data.get("limits").and_then(Value::as_array) {
        for item in limits {
            let limit_type = item.get("type").and_then(Value::as_str).unwrap_or("");
            let number = item.get("number").and_then(Value::as_i64).unwrap_or(0);
            let unit_code = item.get("unit").and_then(Value::as_i64).unwrap_or(0);
            let used_val = item.get("currentValue").and_then(value_to_f64);
            let total_val = item.get("usage").and_then(value_to_f64);
            let pct = item.get("percentage").and_then(value_to_f64).or_else(|| {
                match (used_val, total_val) {
                    (Some(u), Some(t)) if t > 0.0 => Some(((u / t) * 100.0).clamp(0.0, 100.0)),
                    _ => None,
                }
            });
            let remaining_val = match (total_val, used_val) {
                (Some(t), Some(u)) => Some((t - u).max(0.0)),
                _ => None,
            };
            let resets_ms = item
                .get("nextResetTime")
                .and_then(Value::as_i64)
                .map(normalize_epoch_ms);
            let reset_secs = resets_ms.map(|ms| ((ms - now_ms) / 1000).max(0));
            let (kind, span, name) = if limit_type == "TOKENS_LIMIT" || unit_code == 3 {
                (
                    QuotaWindowKind::HOUR_5,
                    if number > 0 { format!("{number}h") } else { "5h".to_string() },
                    "5h Token Quota".to_string(),
                )
            } else if unit_code == 6 || number == 7 {
                (
                    QuotaWindowKind::DAY_7,
                    "7d".to_string(),
                    "Weekly Quota".to_string(),
                )
            } else {
                (
                    QuotaWindowKind::MONTH,
                    "cycle".to_string(),
                    "Coding Plan Quota".to_string(),
                )
            };
            windows.push(QuotaWindow {
                name,
                kind,
                display: format!(
                    "{:.0}% left",
                    (100.0 - pct.unwrap_or(0.0)).clamp(0.0, 100.0)
                ),
                used: used_val,
                remaining: remaining_val,
                limit: total_val,
                usedPercent: pct,
                unlimited: false,
                resetsAtMs: resets_ms,
                resetSecs: reset_secs,
                spanLabel: Some(span),
                model: None,
                unit: Some(if limit_type == "TIME_LIMIT" {
                    "prompts".to_string()
                } else {
                    "tokens".to_string()
                }),
            });
        }
    }

    if windows.is_empty() {
        if let Some(raw_windows) = data.get("windows").and_then(Value::as_array) {
            for item in raw_windows {
                let raw_type = item.get("type").and_then(Value::as_str).unwrap_or("5h");
                let total = item.get("total").and_then(value_to_f64).unwrap_or(0.0);
                let remaining = item.get("remaining").and_then(value_to_f64).unwrap_or(0.0);
                let used = (total - remaining).max(0.0);
                let pct = if total > 0.0 {
                    Some(((used / total) * 100.0).clamp(0.0, 100.0))
                } else {
                    None
                };
                let resets_ms = item
                    .get("reset_time")
                    .and_then(Value::as_i64)
                    .map(normalize_epoch_ms);
                let reset_secs = resets_ms.map(|ms| ((ms - now_ms) / 1000).max(0));
                let kind = if raw_type.contains('5') || raw_type.eq_ignore_ascii_case("5h") {
                    QuotaWindowKind::HOUR_5
                } else if raw_type.to_ascii_lowercase().contains("week") {
                    QuotaWindowKind::DAY_7
                } else {
                    QuotaWindowKind::CUSTOM
                };
                windows.push(QuotaWindow {
                    name: format!("GLM Coding ({raw_type})"),
                    kind,
                    display: format!(
                        "{:.0}% left",
                        (100.0 - pct.unwrap_or(0.0)).clamp(0.0, 100.0)
                    ),
                    used: Some(used),
                    remaining: Some(remaining),
                    limit: Some(total),
                    usedPercent: pct,
                    unlimited: false,
                    resetsAtMs: resets_ms,
                    resetSecs: reset_secs,
                    spanLabel: Some(raw_type.to_string()),
                    model: None,
                    unit: Some("tokens".to_string()),
                });
            }
        }
    }

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::SUBSCRIPTION,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: data
            .get("level")
            .or_else(|| data.get("plan"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some("GLM Coding Plan".to_string())),
        accountLabel: None,
        balance: None,
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_zhipu_account_report(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::ZHIPU_ACCOUNT_REPORT;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let data = root.get("data").unwrap_or(&root);
    let available = data
        .get("availableBalance")
        .or_else(|| data.get("available_balance"))
        .or_else(|| data.get("balance"))
        .and_then(value_to_f64);
    let Some(amount) = available else {
        return ProviderQuotaProbeOutcome {
            accountKind: QuotaAccountKind::BALANCE,
            protocol,
            status: QuotaProbeStatus::ERROR,
            plan: None,
            accountLabel: None,
            balance: None,
            windows: Vec::new(),
            consoleUrl: console_url,
            error: Some(
                extract_error_message(body)
                    .unwrap_or_else(|| "Zhipu response did not contain availableBalance".to_string()),
            ),
        };
    };
    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("Zhipu BigModel".to_string()),
        accountLabel: None,
        balance: Some(QuotaBalanceBreakdown {
            currency: "CNY".to_string(),
            totalBalance: Some(amount),
            cashBalance: Some(amount),
            grantedBalance: None,
            usedAmount: None,
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: amount > 0.0,
            isFreeTier: false,
        }),
        windows: vec![QuotaWindow {
            name: "CNY Balance".to_string(),
            kind: QuotaWindowKind::BALANCE,
            display: format_currency_amount("CNY", amount),
            used: None,
            remaining: Some(amount),
            limit: None,
            usedPercent: None,
            unlimited: false,
            resetsAtMs: None,
            resetSecs: None,
            spanLabel: Some("CNY".to_string()),
            model: None,
            unit: Some("CNY".to_string()),
        }],
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_minimax_token_plan(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::MINIMAX_TOKEN_PLAN;
    if !(200..300).contains(&status) {
        return http_status_failure(
            QuotaAccountKind::SUBSCRIPTION,
            protocol,
            console_url,
            status,
            body,
        );
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::SUBSCRIPTION,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    if let Some(status_code) = root.pointer("/base_resp/status_code").and_then(Value::as_i64) {
        if status_code != 0 {
            let msg = root
                .pointer("/base_resp/status_msg")
                .and_then(Value::as_str)
                .unwrap_or("MiniMax API error");
            if status_code == 1004 || msg.to_ascii_lowercase().contains("not support") {
                return ProviderQuotaProbeOutcome {
                    accountKind: QuotaAccountKind::UNSUPPORTED,
                    protocol,
                    status: QuotaProbeStatus::UNSUPPORTED,
                    plan: None,
                    accountLabel: None,
                    balance: None,
                    windows: Vec::new(),
                    consoleUrl: console_url,
                    error: Some(format!(
                        "MiniMax key returned code {status_code} ({msg}); standard Pay-As-You-Go keys do not expose HTTP balance — use Monthly Budget Cap or Console."
                    )),
                };
            }
            return ProviderQuotaProbeOutcome {
                accountKind: QuotaAccountKind::SUBSCRIPTION,
                protocol,
                status: QuotaProbeStatus::ERROR,
                plan: None,
                accountLabel: None,
                balance: None,
                windows: Vec::new(),
                consoleUrl: console_url,
                error: Some(format!("Code {status_code}: {msg}")),
            };
        }
    }

    let mut windows = Vec::new();
    let now_ms = currentTimeMillis();

    // MiniMax model_remains quirks: current_interval_usage_count / current_weekly_usage_count
    // actually contain REMAINING quota, not used quota.
    if let Some(items) = root.get("model_remains").and_then(Value::as_array) {
        for item in items {
            let model_name = item
                .get("model_name")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string);
            let end_ms = item
                .get("end_time")
                .and_then(Value::as_i64)
                .map(normalize_epoch_ms);
            let remains_secs = item
                .get("remains_time")
                .and_then(Value::as_i64)
                .map(|v| if v > 10_000_000 { v / 1000 } else { v })
                .or_else(|| end_ms.map(|ms| ((ms - now_ms) / 1000).max(0)));

            if let Some(total) = item
                .get("current_interval_total_count")
                .and_then(value_to_f64)
            {
                if total > 0.0 {
                    let remaining = item
                        .get("current_interval_usage_count")
                        .and_then(value_to_f64)
                        .unwrap_or(total)
                        .clamp(0.0, total);
                    let used = (total - remaining).max(0.0);
                    let pct = ((used / total) * 100.0).clamp(0.0, 100.0);
                    windows.push(QuotaWindow {
                        name: model_name
                            .as_ref()
                            .map(|m| format!("5h Window ({m})"))
                            .unwrap_or_else(|| "5h Window".to_string()),
                        kind: QuotaWindowKind::HOUR_5,
                        display: format!("{:.0}% left", (100.0 - pct).clamp(0.0, 100.0)),
                        used: Some(used),
                        remaining: Some(remaining),
                        limit: Some(total),
                        usedPercent: Some(pct),
                        unlimited: false,
                        resetsAtMs: end_ms,
                        resetSecs: remains_secs,
                        spanLabel: Some("5h".to_string()),
                        model: model_name.clone(),
                        unit: Some("prompts".to_string()),
                    });
                }
            }
            if let Some(w_total) = item
                .get("current_weekly_total_count")
                .and_then(value_to_f64)
            {
                if w_total > 0.0 {
                    let w_remaining = item
                        .get("current_weekly_usage_count")
                        .and_then(value_to_f64)
                        .unwrap_or(w_total)
                        .clamp(0.0, w_total);
                    let w_used = (w_total - w_remaining).max(0.0);
                    let w_pct = ((w_used / w_total) * 100.0).clamp(0.0, 100.0);
                    windows.push(QuotaWindow {
                        name: model_name
                            .as_ref()
                            .map(|m| format!("Weekly Window ({m})"))
                            .unwrap_or_else(|| "Weekly Window".to_string()),
                        kind: QuotaWindowKind::DAY_7,
                        display: format!("{:.0}% left", (100.0 - w_pct).clamp(0.0, 100.0)),
                        used: Some(w_used),
                        remaining: Some(w_remaining),
                        limit: Some(w_total),
                        usedPercent: Some(w_pct),
                        unlimited: false,
                        resetsAtMs: None,
                        resetSecs: None,
                        spanLabel: Some("7d".to_string()),
                        model: model_name,
                        unit: Some("prompts".to_string()),
                    });
                }
            }
        }
    }

    if windows.is_empty() {
        let remains_secs = root
            .get("remains_time")
            .and_then(Value::as_i64)
            .map(|v| if v > 10_000_000 { v / 1000 } else { v });
        if let Some(rem_5h) = root
            .get("current_5h_remaining_percent")
            .and_then(value_to_f64)
        {
            let rem = rem_5h.clamp(0.0, 100.0);
            windows.push(QuotaWindow {
                name: "5h Window".to_string(),
                kind: QuotaWindowKind::HOUR_5,
                display: format!("{rem:.0}% left"),
                used: Some((100.0 - rem).max(0.0)),
                remaining: Some(rem),
                limit: Some(100.0),
                usedPercent: Some((100.0 - rem).clamp(0.0, 100.0)),
                unlimited: false,
                resetsAtMs: remains_secs.map(|s| now_ms + s * 1000),
                resetSecs: remains_secs,
                spanLabel: Some("5h".to_string()),
                model: None,
                unit: Some("%".to_string()),
            });
        }
        if let Some(rem_week) = root
            .get("current_weekly_remaining_percent")
            .and_then(value_to_f64)
        {
            let rem = rem_week.clamp(0.0, 100.0);
            windows.push(QuotaWindow {
                name: "Weekly Window".to_string(),
                kind: QuotaWindowKind::DAY_7,
                display: format!("{rem:.0}% left"),
                used: Some((100.0 - rem).max(0.0)),
                remaining: Some(rem),
                limit: Some(100.0),
                usedPercent: Some((100.0 - rem).clamp(0.0, 100.0)),
                unlimited: false,
                resetsAtMs: None,
                resetSecs: None,
                spanLabel: Some("7d".to_string()),
                model: None,
                unit: Some("%".to_string()),
            });
        }
    }

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::SUBSCRIPTION,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("MiniMax Token Plan".to_string()),
        accountLabel: None,
        balance: None,
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_moonshot_balance(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::MOONSHOT_BALANCE;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let data = root.get("data").unwrap_or(&root);
    let available = data.get("available_balance").and_then(value_to_f64);
    let cash = data.get("cash_balance").and_then(value_to_f64);
    let voucher = data.get("voucher_balance").and_then(value_to_f64);

    let mut windows = Vec::new();
    if let Some(total) = available {
        windows.push(QuotaWindow {
            name: "Available Balance".to_string(),
            kind: QuotaWindowKind::BALANCE,
            display: format_currency_amount("CNY", total),
            used: None,
            remaining: Some(total),
            limit: None,
            usedPercent: None,
            unlimited: false,
            resetsAtMs: None,
            resetSecs: None,
            spanLabel: Some("CNY".to_string()),
            model: None,
            unit: Some("CNY".to_string()),
        });
    }

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("Moonshot Kimi API".to_string()),
        accountLabel: None,
        balance: Some(QuotaBalanceBreakdown {
            currency: "CNY".to_string(),
            totalBalance: available,
            cashBalance: cash,
            grantedBalance: voucher,
            usedAmount: None,
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: available.is_some_and(|val| val > 0.0),
            isFreeTier: false,
        }),
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_kimi_coding_usages(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::KIMI_CODING_USAGES;
    if !(200..300).contains(&status) {
        return http_status_failure(
            QuotaAccountKind::SUBSCRIPTION,
            protocol,
            console_url,
            status,
            body,
        );
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::SUBSCRIPTION,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let data = root.get("data").unwrap_or(&root);
    let plan = data
        .get("membership_level")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| Some("Kimi Code".to_string()));
    let now_ms = currentTimeMillis();
    let mut windows = Vec::new();
    if let Some(items) = data.get("usage_windows").and_then(Value::as_array) {
        for item in items {
            let win_type = item.get("window_type").and_then(Value::as_str).unwrap_or("5h");
            let consumed = item.get("consumed_tokens").and_then(value_to_f64).unwrap_or(0.0);
            let max_tokens = item.get("max_tokens").and_then(value_to_f64).unwrap_or(0.0);
            let remaining = (max_tokens - consumed).max(0.0);
            let pct = if max_tokens > 0.0 {
                Some(((consumed / max_tokens) * 100.0).clamp(0.0, 100.0))
            } else {
                None
            };
            let resets_ms = item
                .get("reset_timestamp")
                .and_then(Value::as_i64)
                .map(normalize_epoch_ms);
            let reset_secs = resets_ms.map(|ms| ((ms - now_ms) / 1000).max(0));
            let kind = if win_type.contains("5h") {
                QuotaWindowKind::HOUR_5
            } else if win_type.to_ascii_lowercase().contains("week") {
                QuotaWindowKind::DAY_7
            } else {
                QuotaWindowKind::CUSTOM
            };
            windows.push(QuotaWindow {
                name: format!("Kimi Code ({win_type})"),
                kind,
                display: format!(
                    "{:.0}% left",
                    (100.0 - pct.unwrap_or(0.0)).clamp(0.0, 100.0)
                ),
                used: Some(consumed),
                remaining: Some(remaining),
                limit: Some(max_tokens),
                usedPercent: pct,
                unlimited: false,
                resetsAtMs: resets_ms,
                resetSecs: reset_secs,
                spanLabel: Some(win_type.to_string()),
                model: None,
                unit: Some("tokens".to_string()),
            });
        }
    }

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::SUBSCRIPTION,
        protocol,
        status: QuotaProbeStatus::OK,
        plan,
        accountLabel: None,
        balance: None,
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_siliconflow_user_info(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::SILICONFLOW_USER_INFO;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let data = root.get("data").unwrap_or(&root);
    let label = data
        .get("email")
        .or_else(|| data.get("name"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let total = data.get("totalBalance").and_then(value_to_f64);
    let cash = data.get("chargeBalance").and_then(value_to_f64);
    let granted = data.get("balance").and_then(value_to_f64);
    let effective_total = total.or(granted).or(cash);
    let status_text = data.get("status").and_then(Value::as_str).unwrap_or("normal");

    let mut windows = Vec::new();
    if let Some(amount) = effective_total {
        windows.push(QuotaWindow {
            name: "Total Balance".to_string(),
            kind: QuotaWindowKind::BALANCE,
            display: format_currency_amount("CNY", amount),
            used: None,
            remaining: Some(amount),
            limit: None,
            usedPercent: None,
            unlimited: false,
            resetsAtMs: None,
            resetSecs: None,
            spanLabel: Some("CNY".to_string()),
            model: None,
            unit: Some("CNY".to_string()),
        });
    }

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("SiliconFlow".to_string()),
        accountLabel: label,
        balance: Some(QuotaBalanceBreakdown {
            currency: "CNY".to_string(),
            totalBalance: effective_total,
            cashBalance: cash,
            grantedBalance: granted,
            usedAmount: None,
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: status_text.eq_ignore_ascii_case("normal")
                && effective_total.is_some_and(|v| v > 0.0),
            isFreeTier: false,
        }),
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_codex_wham_usage(
    status: u16,
    body: &str,
    fallback_email: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::CODEX_WHAM_USAGE;
    if !(200..300).contains(&status) {
        return http_status_failure(
            QuotaAccountKind::SUBSCRIPTION,
            protocol,
            console_url,
            status,
            body,
        );
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::SUBSCRIPTION,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let email = root
        .get("email")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(fallback_email)
        .to_string();
    let plan = root
        .get("plan_type")
        .and_then(Value::as_str)
        .map(|s| format!("ChatGPT {}", s.to_ascii_uppercase()));
    let mut windows = Vec::new();
    let now_ms = currentTimeMillis();
    if let Some(primary) = root.pointer("/rate_limit/primary_window") {
        windows.push(parse_codex_window("5h Session Window", QuotaWindowKind::HOUR_5, primary, now_ms));
    }
    if let Some(secondary) = root.pointer("/rate_limit/secondary_window") {
        windows.push(parse_codex_window("7d Weekly Window", QuotaWindowKind::DAY_7, secondary, now_ms));
    }
    if let Some(review_primary) = root.pointer("/code_review_rate_limit/primary_window") {
        windows.push(parse_codex_window(
            "Code Review Quota",
            QuotaWindowKind::DAY_7,
            review_primary,
            now_ms,
        ));
    }

    let credit_balance = root.pointer("/credits/balance").and_then(value_to_f64);
    let has_credits = root
        .pointer("/credits/has_credits")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::SUBSCRIPTION,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: plan.or_else(|| Some("ChatGPT Codex".to_string())),
        accountLabel: if email.is_empty() { None } else { Some(email) },
        balance: if has_credits || credit_balance.is_some() {
            Some(QuotaBalanceBreakdown {
                currency: "USD".to_string(),
                totalBalance: credit_balance,
                cashBalance: credit_balance,
                grantedBalance: None,
                usedAmount: None,
                dailyUsed: None,
                monthlyUsed: None,
                isAvailable: credit_balance.is_none_or(|b| b > 0.0),
                isFreeTier: false,
            })
        } else {
            None
        },
        windows,
        consoleUrl: console_url,
        error: None,
    }
}

fn parse_codex_window(name: &str, kind: QuotaWindowKind, obj: &Value, now_ms: i64) -> QuotaWindow {
    let used_pct = obj
        .get("used_percent")
        .and_then(value_to_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);
    let remaining_pct = (100.0 - used_pct).max(0.0);
    let reset_secs = obj.get("reset_after_seconds").and_then(Value::as_i64);
    let resets_ms = obj
        .get("reset_at")
        .and_then(Value::as_i64)
        .map(normalize_epoch_ms)
        .or_else(|| reset_secs.map(|s| now_ms + s.max(0) * 1000));
    let span_secs = obj.get("limit_window_seconds").and_then(Value::as_i64);
    let span_label = span_secs.map(|s| {
        if s >= 86400 {
            format!("{}d", s / 86400)
        } else {
            format!("{}h", (s / 3600).max(1))
        }
    });
    QuotaWindow {
        name: name.to_string(),
        kind,
        display: format!("{remaining_pct:.0}% left"),
        used: Some(used_pct),
        remaining: Some(remaining_pct),
        limit: Some(100.0),
        usedPercent: Some(used_pct),
        unlimited: false,
        resetsAtMs: resets_ms,
        resetSecs: reset_secs,
        spanLabel: span_label,
        model: None,
        unit: Some("%".to_string()),
    }
}

pub fn parse_new_api_token(
    status: u16,
    body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::NEW_API_TOKEN;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    if root.get("success").and_then(Value::as_bool) == Some(false) {
        let msg = root
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("New-API token query failed");
        return error_outcome(
            QuotaAccountKind::BALANCE,
            protocol,
            console_url,
            msg.to_string(),
        );
    }
    let data = root.get("data").unwrap_or(&root);
    let label = data
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let unlimited = data
        .get("unlimited_quota")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let available_usd = data
        .get("total_available")
        .and_then(value_to_f64)
        .map(|u| u / NEW_API_QUOTA_UNITS_PER_USD);
    let used_usd = data
        .get("total_used")
        .and_then(value_to_f64)
        .map(|u| u / NEW_API_QUOTA_UNITS_PER_USD);
    let limit_usd = match (available_usd, used_usd) {
        (Some(a), Some(u)) => Some((a + u).max(0.0)),
        _ => None,
    };
    let expires_ms = data
        .get("expires_at")
        .and_then(Value::as_i64)
        .filter(|v| *v > 0)
        .map(normalize_epoch_ms);
    let pct = match (used_usd, limit_usd) {
        (Some(u), Some(l)) if l > 0.0 => Some(((u / l) * 100.0).clamp(0.0, 100.0)),
        _ => None,
    };

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("New-API / One-API Relay".to_string()),
        accountLabel: label,
        balance: Some(QuotaBalanceBreakdown {
            currency: "USD".to_string(),
            totalBalance: if unlimited { None } else { available_usd },
            cashBalance: available_usd,
            grantedBalance: None,
            usedAmount: used_usd,
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: unlimited || available_usd.is_none_or(|v| v > 0.0),
            isFreeTier: false,
        }),
        windows: vec![QuotaWindow {
            name: "Relay Token Quota".to_string(),
            kind: QuotaWindowKind::TOTAL,
            display: if unlimited {
                "Unlimited".to_string()
            } else {
                available_usd
                    .map(|v| format_currency_amount("USD", v))
                    .unwrap_or_default()
            },
            used: used_usd,
            remaining: available_usd,
            limit: limit_usd,
            usedPercent: pct,
            unlimited,
            resetsAtMs: expires_ms,
            resetSecs: None,
            spanLabel: None,
            model: None,
            unit: Some("USD".to_string()),
        }],
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_openai_legacy_billing(
    sub_status: u16,
    sub_body: &str,
    usage_status: u16,
    usage_body: &str,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::OPENAI_LEGACY_BILLING;
    if !(200..300).contains(&sub_status) {
        return http_status_failure(
            QuotaAccountKind::BALANCE,
            protocol,
            console_url,
            sub_status,
            sub_body,
        );
    }
    let sub: Value = match serde_json::from_str(sub_body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let hard_limit = sub
        .get("hard_limit_usd")
        .or_else(|| sub.get("system_hard_limit_usd"))
        .and_then(value_to_f64)
        .unwrap_or(0.0);
    let used_usd = if (200..300).contains(&usage_status) {
        serde_json::from_str::<Value>(usage_body)
            .ok()
            .and_then(|u| u.get("total_usage").and_then(value_to_f64))
            .map(|cents| cents / 100.0)
            .unwrap_or(0.0)
    } else {
        0.0
    };
    let remaining = (hard_limit - used_usd).max(0.0);
    let pct = if hard_limit > 0.0 {
        Some(((used_usd / hard_limit) * 100.0).clamp(0.0, 100.0))
    } else {
        None
    };

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: sub
            .get("plan")
            .and_then(|p| p.get("title"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some("OpenAI Billing Relay".to_string())),
        accountLabel: sub
            .get("account_name")
            .and_then(Value::as_str)
            .map(str::to_string),
        balance: Some(QuotaBalanceBreakdown {
            currency: "USD".to_string(),
            totalBalance: Some(remaining),
            cashBalance: Some(remaining),
            grantedBalance: None,
            usedAmount: Some(used_usd),
            dailyUsed: None,
            monthlyUsed: Some(used_usd),
            isAvailable: remaining > 0.0,
            isFreeTier: false,
        }),
        windows: vec![QuotaWindow {
            name: "Billing Grant".to_string(),
            kind: QuotaWindowKind::TOTAL,
            display: format!(
                "{} / {}",
                format_currency_amount("USD", remaining),
                format_currency_amount("USD", hard_limit)
            ),
            used: Some(used_usd),
            remaining: Some(remaining),
            limit: Some(hard_limit),
            usedPercent: pct,
            unlimited: false,
            resetsAtMs: sub
                .get("access_until")
                .and_then(Value::as_i64)
                .map(normalize_epoch_ms),
            resetSecs: None,
            spanLabel: None,
            model: None,
            unit: Some("USD".to_string()),
        }],
        consoleUrl: console_url,
        error: None,
    }
}

pub fn parse_custom_http_json(
    status: u16,
    body: &str,
    amount_path: &str,
    currency_path: Option<&str>,
    fallback_currency: Option<&str>,
    console_url: Option<String>,
) -> ProviderQuotaProbeOutcome {
    let protocol = ProviderBalanceProtocol::CUSTOM_HTTP_JSON;
    if !(200..300).contains(&status) {
        return http_status_failure(QuotaAccountKind::BALANCE, protocol, console_url, status, body);
    }
    let root: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(error) => {
            return error_outcome(
                QuotaAccountKind::BALANCE,
                protocol,
                console_url,
                error.to_string(),
            )
        }
    };
    let Some(amount) = value_at_dot_path(&root, amount_path).and_then(value_to_f64) else {
        return error_outcome(
            QuotaAccountKind::BALANCE,
            protocol,
            console_url,
            format!("Custom probe response missing numeric field at {amount_path}"),
        );
    };
    let currency = currency_path
        .and_then(|path| value_at_dot_path(&root, path))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .or_else(|| fallback_currency.filter(|s| !s.trim().is_empty()))
        .unwrap_or("USD")
        .to_string();

    ProviderQuotaProbeOutcome {
        accountKind: QuotaAccountKind::BALANCE,
        protocol,
        status: QuotaProbeStatus::OK,
        plan: Some("Custom Balance Probe".to_string()),
        accountLabel: None,
        balance: Some(QuotaBalanceBreakdown {
            currency: currency.clone(),
            totalBalance: Some(amount),
            cashBalance: Some(amount),
            grantedBalance: None,
            usedAmount: None,
            dailyUsed: None,
            monthlyUsed: None,
            isAvailable: amount > 0.0,
            isFreeTier: false,
        }),
        windows: vec![QuotaWindow {
            name: "Custom Balance".to_string(),
            kind: QuotaWindowKind::BALANCE,
            display: format_currency_amount(&currency, amount),
            used: None,
            remaining: Some(amount),
            limit: None,
            usedPercent: None,
            unlimited: false,
            resetsAtMs: None,
            resetSecs: None,
            spanLabel: Some(currency.clone()),
            model: None,
            unit: Some(currency),
        }],
        consoleUrl: console_url,
        error: None,
    }
}

fn value_at_dot_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for segment in path.trim().trim_start_matches("$.").split('.') {
        let key = segment.trim();
        if key.is_empty() {
            continue;
        }
        if let Ok(index) = key.parse::<usize>() {
            current = current.get(index).or_else(|| current.get(key))?;
        } else {
            current = current.get(key)?;
        }
    }
    Some(current)
}

fn value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(num) => num.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn normalize_epoch_ms(raw: i64) -> i64 {
    if raw > 10_000_000_000 {
        raw
    } else {
        raw.saturating_mul(1000)
    }
}

fn format_currency_amount(currency: &str, amount: f64) -> String {
    match currency.to_ascii_uppercase().as_str() {
        "CNY" | "RMB" => format!("¥{amount:.2}"),
        "USD" => format!("${amount:.2}"),
        other => format!("{amount:.2} {other}"),
    }
}

fn rolling_month_date_range_utc() -> (String, String) {
    let secs = currentTimeMillis() / 1000;
    let start_secs = secs.saturating_sub(30 * 86400);
    let start = chrono::DateTime::from_timestamp(start_secs, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string());
    let end = chrono::DateTime::from_timestamp(secs.saturating_add(86400), 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-02-01".to_string());
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_deepseek_balance_with_multi_currency() {
        let body = r#"{
            "is_available": true,
            "balance_infos": [
                {"currency": "CNY", "total_balance": "42.50", "granted_balance": "2.50", "topped_up_balance": "40.00"},
                {"currency": "USD", "total_balance": "1.20", "granted_balance": "0.00", "topped_up_balance": "1.20"}
            ]
        }"#;
        let out = parse_deepseek_balance(200, body, None);
        assert_eq!(out.status, QuotaProbeStatus::OK);
        let bal = out.balance.unwrap();
        assert_eq!(bal.currency, "CNY");
        assert_eq!(bal.totalBalance, Some(42.5));
        assert_eq!(bal.grantedBalance, Some(2.5));
        assert_eq!(bal.cashBalance, Some(40.0));
        assert!(bal.isAvailable);
        assert_eq!(out.windows.len(), 2);
    }

    #[test]
    fn parses_openrouter_key_and_rejects_non_management_credits_explicitly() {
        let key_body = r#"{
            "data": {
                "label": "sk-or-v1-prod",
                "limit": 50.0,
                "limit_remaining": 38.25,
                "limit_reset": "monthly",
                "usage": 11.75,
                "usage_daily": 1.2,
                "usage_monthly": 11.75,
                "is_free_tier": false
            }
        }"#;
        let out = parse_openrouter_key(200, key_body, None);
        assert_eq!(out.status, QuotaProbeStatus::OK);
        assert_eq!(out.accountLabel.as_deref(), Some("sk-or-v1-prod"));
        assert_eq!(out.balance.as_ref().unwrap().totalBalance, Some(38.25));
        assert_eq!(out.windows.len(), 1);
        assert_eq!(out.windows[0].kind, QuotaWindowKind::MONTH);

        let err = parse_openrouter_credits(403, r#"{"error":{"message":"Management key required"}}"#, None);
        assert_eq!(err.status, QuotaProbeStatus::UNAUTHORIZED);
        assert!(err.error.unwrap().contains("Management Key"));
    }

    #[test]
    fn parses_minimax_token_plan_remaining_count_semantics() {
        let plan_body = r#"{
            "base_resp": {"status_code": 0, "status_msg": "success"},
            "model_remains": [
                {
                    "model_name": "MiniMax-M2.5",
                    "current_interval_total_count": 100,
                    "current_interval_usage_count": 80,
                    "current_weekly_total_count": 500,
                    "current_weekly_usage_count": 400,
                    "remains_time": 7200
                }
            ]
        }"#;
        let out = parse_minimax_token_plan(200, plan_body, None);
        assert_eq!(out.status, QuotaProbeStatus::OK);
        assert_eq!(out.windows.len(), 2);
        // current_interval_usage_count=80 out of 100 means 80 remaining and 20 used (20% used).
        assert_eq!(out.windows[0].remaining, Some(80.0));
        assert_eq!(out.windows[0].used, Some(20.0));
        assert_eq!(out.windows[0].usedPercent, Some(20.0));

        let payg_err = r#"{"base_resp":{"status_code":1004,"status_msg":"api key not authorized for token_plan"}}"#;
        let unsup = parse_minimax_token_plan(200, payg_err, None);
        assert_eq!(unsup.status, QuotaProbeStatus::UNSUPPORTED);
    }

    #[test]
    fn parses_zhipu_coding_plan_and_codex_wham_windows() {
        let zhipu_body = r#"{
            "code": 200,
            "data": {
                "level": "pro",
                "limits": [
                    {"type": "TOKENS_LIMIT", "unit": 3, "number": 5, "percentage": 35, "currentValue": 3500, "usage": 10000, "nextResetTime": 1784500000}
                ]
            }
        }"#;
        let zhipu = parse_zhipu_coding_plan(200, zhipu_body, None);
        assert_eq!(zhipu.status, QuotaProbeStatus::OK);
        assert_eq!(zhipu.plan.as_deref(), Some("pro"));
        assert_eq!(zhipu.windows[0].kind, QuotaWindowKind::HOUR_5);
        assert_eq!(zhipu.windows[0].usedPercent, Some(35.0));

        let codex_body = r#"{
            "email": "dev@example.com",
            "plan_type": "plus",
            "rate_limit": {
                "primary_window": {"used_percent": 15, "limit_window_seconds": 18000, "reset_after_seconds": 9000, "reset_at": 1784530149},
                "secondary_window": {"used_percent": 40, "limit_window_seconds": 604800, "reset_after_seconds": 300000, "reset_at": 1784950000}
            }
        }"#;
        let codex = parse_codex_wham_usage(200, codex_body, "", None);
        assert_eq!(codex.status, QuotaProbeStatus::OK);
        assert_eq!(codex.accountLabel.as_deref(), Some("dev@example.com"));
        assert_eq!(codex.windows.len(), 2);
        assert_eq!(codex.windows[0].usedPercent, Some(15.0));
        assert_eq!(codex.windows[1].kind, QuotaWindowKind::DAY_7);
    }
}
