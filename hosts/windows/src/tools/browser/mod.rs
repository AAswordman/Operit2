use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use operit_host_api::{
    HostError, HostResult, WebVisitHost, WebVisitLinkData, WebVisitRequest, WebVisitResult,
};
use serde_json::{json, Value};
use tungstenite::{connect, Message};

pub struct WindowsWebVisitHost;

impl WindowsWebVisitHost {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsWebVisitHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WebVisitHost for WindowsWebVisitHost {
    fn visitWeb(&self, request: WebVisitRequest) -> HostResult<WebVisitResult> {
        visit_with_chromium(request, browser_candidates(), &[])
    }
}

fn browser_candidates() -> Vec<String> {
    let mut candidates = Vec::new();
    if let Ok(path) = env::var("OPERIT_BROWSER_PATH") {
        if !path.trim().is_empty() {
            candidates.push(path);
        }
    }
    candidates.extend(
        [
            "msedge.exe",
            "chrome.exe",
            "chromium.exe",
            "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
            "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
            "C:/Program Files/Google/Chrome/Application/chrome.exe",
            "C:/Program Files (x86)/Google/Chrome/Application/chrome.exe",
        ]
        .into_iter()
        .map(str::to_string),
    );
    candidates
}

fn visit_with_chromium(
    request: WebVisitRequest,
    candidates: Vec<String>,
    extra_args: &[&str],
) -> HostResult<WebVisitResult> {
    let port = allocate_port()?;
    let profile_dir = create_profile_dir()?;
    let mut child = spawn_browser(&candidates, port, &profile_dir, extra_args)?;
    let result = run_cdp_visit(&request, port);
    cleanup_browser(&mut child, &profile_dir);
    result
}

fn allocate_port() -> HostResult<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| HostError::new(format!("Failed to allocate browser debug port: {error}")))?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|error| HostError::new(format!("Failed to read browser debug port: {error}")))
}

fn create_profile_dir() -> HostResult<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| HostError::new(error.to_string()))?
        .as_millis();
    let path = env::temp_dir().join(format!(
        "operit2_browser_{}_{}",
        std::process::id(),
        millis
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn spawn_browser(
    candidates: &[String],
    port: u16,
    profile_dir: &PathBuf,
    extra_args: &[&str],
) -> HostResult<Child> {
    let mut errors = Vec::new();
    for candidate in candidates {
        let mut command = Command::new(candidate);
        command
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-background-networking")
            .arg("--remote-debugging-address=127.0.0.1")
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .args(extra_args)
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) => errors.push(format!("{candidate}: {error}")),
        }
    }
    Err(HostError::new(format!(
        "No Chromium-compatible browser could be launched. Tried: {}",
        errors.join(" | ")
    )))
}

fn cleanup_browser(child: &mut Child, profile_dir: &PathBuf) {
    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(profile_dir);
}

fn run_cdp_visit(request: &WebVisitRequest, port: u16) -> HostResult<WebVisitResult> {
    wait_for_browser(port)?;
    let target = devtools_json(port, "PUT", "/json/new")
        .map_err(|error| HostError::new(format!("Failed to create browser target: {error}")))?;
    let ws_url = target
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new("Browser target did not return a debugger WebSocket URL"))?;
    let mut session = CdpSession::connect(ws_url)?;
    session.command("Page.enable", json!({}))?;
    session.command("Runtime.enable", json!({}))?;
    if !request.headers.is_empty() {
        session.command("Network.enable", json!({}))?;
        let headers = request
            .headers
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect::<serde_json::Map<_, _>>();
        session.command("Network.setExtraHTTPHeaders", json!({ "headers": headers }))?;
    }
    if !request.userAgent.trim().is_empty() {
        session.command(
            "Network.setUserAgentOverride",
            json!({ "userAgent": request.userAgent }),
        )?;
    }
    session.command("Page.navigate", json!({ "url": request.url }))?;
    wait_for_document_ready(&mut session)?;
    thread::sleep(Duration::from_millis(900));
    scroll_page(&mut session)?;
    extract_page(&mut session, request.includeImageLinks)
}

fn wait_for_browser(port: u16) -> HostResult<()> {
    let started = Instant::now();
    loop {
        if devtools_json(port, "GET", "/json/version").is_ok() {
            return Ok(());
        }
        if started.elapsed() >= Duration::from_secs(10) {
            return Err(HostError::new("Timed out waiting for browser DevTools endpoint"));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn devtools_json(port: u16, method: &str, path: &str) -> HostResult<Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|error| HostError::new(format!("Browser DevTools connection failed: {error}")))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| HostError::new(format!("Browser DevTools request failed: {error}")))?;
    let response = read_http_response(&mut stream)?;
    let responseText = String::from_utf8(response)
        .map_err(|error| HostError::new(format!("Browser DevTools response was not UTF-8: {error}")))?;
    let Some((headers, body)) = responseText.split_once("\r\n\r\n") else {
        return Err(HostError::new("Browser DevTools response was malformed"));
    };
    let statusLine = headers.lines().next().unwrap_or_default();
    if !statusLine.contains(" 200 ") {
        return Err(HostError::new(format!(
            "Browser DevTools returned {statusLine}"
        )));
    }
    serde_json::from_str::<Value>(body)
        .map_err(|error| HostError::new(format!("Invalid browser DevTools JSON: {error}")))
}

fn read_http_response(stream: &mut TcpStream) -> HostResult<Vec<u8>> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|error| HostError::new(format!("Browser DevTools response failed: {error}")))?;
        if count == 0 {
            return Ok(response);
        }
        response.extend_from_slice(&buffer[..count]);
        let Some(headerEnd) = find_bytes(&response, b"\r\n\r\n") else {
            continue;
        };
        let headerText = String::from_utf8_lossy(&response[..headerEnd]);
        let contentLength = headerText
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        if response.len() >= headerEnd + 4 + contentLength {
            return Ok(response);
        }
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

struct CdpSession {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    next_id: i64,
}

impl CdpSession {
    fn connect(ws_url: &str) -> HostResult<Self> {
        let (mut socket, _) = connect(ws_url)
            .map_err(|error| HostError::new(format!("Failed to connect browser debugger: {error}")))?;
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
        }
        Ok(Self { socket, next_id: 1 })
    }

    fn command(&mut self, method: &str, params: Value) -> HostResult<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.socket
            .send(Message::Text(
                json!({ "id": id, "method": method, "params": params }).to_string().into(),
            ))
            .map_err(|error| HostError::new(format!("Failed to send browser command {method}: {error}")))?;
        loop {
            let message = self
                .socket
                .read()
                .map_err(|error| HostError::new(format!("Failed to read browser command {method}: {error}")))?;
            let Message::Text(text) = message else {
                continue;
            };
            let value = serde_json::from_str::<Value>(&text)
                .map_err(|error| HostError::new(format!("Invalid browser command response: {error}")))?;
            if value.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(HostError::new(format!("Browser command {method} failed: {error}")));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

fn wait_for_document_ready(session: &mut CdpSession) -> HostResult<()> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        let ready_state = evaluate_string(session, "document.readyState")?;
        if ready_state == "complete" {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(HostError::new("Timed out waiting for document.readyState=complete"))
}

fn scroll_page(session: &mut CdpSession) -> HostResult<()> {
    for _ in 0..4 {
        let _ = session.command(
            "Runtime.evaluate",
            json!({
                "expression": "window.scrollTo(0, document.body ? document.body.scrollHeight : document.documentElement.scrollHeight);",
                "returnByValue": true
            }),
        )?;
        thread::sleep(Duration::from_millis(350));
    }
    Ok(())
}

fn evaluate_string(session: &mut CdpSession, expression: &str) -> HostResult<String> {
    let result = session.command(
        "Runtime.evaluate",
        json!({ "expression": expression, "returnByValue": true }),
    )?;
    Ok(result
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

fn extract_page(session: &mut CdpSession, include_images: bool) -> HostResult<WebVisitResult> {
    let script = extraction_script(include_images);
    let raw = evaluate_string(session, &script)?;
    let value = serde_json::from_str::<Value>(&raw)
        .map_err(|error| HostError::new(format!("Invalid extracted browser JSON: {error}")))?;
    let links = value
        .get("links")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(WebVisitLinkData {
                url: item.get("url")?.as_str()?.to_string(),
                text: item.get("text")?.as_str()?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let metadata = value
        .get("metadata")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| {
            object.iter().filter_map(|(key, value)| {
                value.as_str().map(|value| (key.clone(), value.to_string()))
            })
        })
        .collect::<Vec<_>>();
    let image_links = value
        .get("imageLinks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    Ok(WebVisitResult {
        url: value
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Web Page")
            .to_string(),
        content: value
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        metadata,
        links,
        imageLinks: image_links,
    })
}

fn extraction_script(include_images: bool) -> String {
    format!(
        r#"
JSON.stringify((() => {{
  const includeImages = {include_images};
  const absoluteUrl = (value) => {{
    try {{ return new URL(value, location.href).href; }} catch (_) {{ return ""; }}
  }};
  const cleanText = (value) => String(value || "")
    .replace(/\u00a0/g, " ")
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter((line, index, arr) => line.length > 0 || (index > 0 && arr[index - 1].trim().length > 0))
    .join("\n")
    .trim();
  const metadata = {{}};
  for (const meta of Array.from(document.querySelectorAll("meta"))) {{
    const key = meta.getAttribute("name") || meta.getAttribute("property");
    const content = meta.getAttribute("content");
    if (key && content && !metadata[key]) metadata[key] = content;
  }}
  const seenLinks = new Set();
  const links = [];
  for (const anchor of Array.from(document.querySelectorAll("a[href]"))) {{
    const url = absoluteUrl(anchor.getAttribute("href"));
    const text = cleanText(anchor.innerText || anchor.getAttribute("aria-label") || anchor.getAttribute("title") || url);
    if (!url || !text || seenLinks.has(url + "\n" + text)) continue;
    seenLinks.add(url + "\n" + text);
    links.push({{ url, text }});
  }}
  const imageLinks = [];
  if (includeImages) {{
    const seenImages = new Set();
    for (const image of Array.from(document.querySelectorAll("img"))) {{
      const src = image.currentSrc || image.getAttribute("src") || image.getAttribute("data-src") || "";
      const url = absoluteUrl(src);
      if (!url || url.startsWith("data:") || url.startsWith("blob:") || seenImages.has(url)) continue;
      seenImages.add(url);
      imageLinks.push(url);
    }}
  }}
  const title = cleanText(document.title || (document.querySelector("h1") && document.querySelector("h1").innerText) || "Web Page");
  const content = cleanText(document.body ? document.body.innerText : document.documentElement.innerText);
  return {{ url: location.href, title, content, metadata, links, imageLinks }};
}})())
"#
    )
}
