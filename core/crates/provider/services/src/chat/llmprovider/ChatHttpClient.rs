// 所有聊天/LLM HTTP 调用共用的 reqwest client。
//
// 之前每个 provider 都现造一个 `reqwest::Client::new()`：既没有任何超时（reqwest 默认无超时），
// 又每次丢掉连接池。后果是 provider 接了 TCP 却不回时，聊天会一直挂着 —— 而且因为
// 不报错，重试逻辑也不会触发。同一个 crate 里的 TTS/STT/市场接口早就显式传了超时，
// 只有主聊天路径漏了。
//
// 这里用的两个超时：
// - connect_timeout(15s)：建连阶段。
// - read_timeout(120s)：**两次读之间**的空闲上限。刻意不用总超时 `.timeout()`，
//   否则长回复/长思考的流会被整体掐断。
use std::sync::OnceLock;
use std::time::Duration;

use operit_util::AppLogger::AppLogger;

const TAG: &str = "ChatHttpClient";

/// 建连上限。DNS/TCP/TLS 都在这个预算内。
pub const CHAT_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// 两次读到数据之间的最长空闲。超时即报错，让重试逻辑生效。
pub const CHAT_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

static CHAT_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 返回共享的聊天用 HTTP client（克隆很便宜，内部是 Arc）。
pub fn chatHttpClient() -> reqwest::Client {
    CHAT_HTTP_CLIENT
        .get_or_init(|| {
            match reqwest::Client::builder()
                .connect_timeout(CHAT_CONNECT_TIMEOUT)
                .read_timeout(CHAT_IDLE_TIMEOUT)
                .build()
            {
                Ok(client) => client,
                Err(error) => {
                    // 正常环境里走不到这里（TLS 后端初始化失败才会）。降级成无超时的默认
                    // client 并**显式记一条警告**：行为不比如今更差，但不会让请求直接失败，
                    // 也不会悄悄把超时丢掉而不留痕迹。
                    AppLogger::w(
                        TAG,
                        &format!(
                            "chat http client build failed, falling back to a default client without timeouts: {error}"
                        ),
                    );
                    reqwest::Client::new()
                }
            }
        })
        .clone()
}
