//! Generic directory protocol. All browsing and candidate selection lives in discovery.rs.
use super::*;
use crate::PeerStateStore::PeerDiscoveryConfig;
use bytes::Bytes;
use http_body_util::{BodyExt, Full, Limited};
use operit_host_api::{HttpRequestData, HttpServer::*, ServiceDiscovery::DiscoveredService};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::sync::oneshot;

const PATH: &str = "/operit/discovery/v1";
pub(super) const LEASE_MS: u64 = 45_000;
pub(super) const MAX_PEERS: usize = 64;
const MAX_RESPONSE: usize = 64 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Announcement {
    version: u8,
    nodeId: String,
    displayName: String,
    endpoints: Vec<String>,
    pub(super) ttlMs: u64,
}
impl Announcement {
    fn valid(&self) -> bool {
        self.version == 1
            && (1..=128).contains(&self.nodeId.len())
            && (1..=256).contains(&self.displayName.len())
            && self.ttlMs > 0
            && (1..=8).contains(&self.endpoints.len())
            && self.endpoints.iter().all(|endpoint| parseEndpoint(endpoint).is_some())
    }
    pub(super) fn records(&self) -> Vec<DiscoveredService> {
        self.endpoints
            .iter()
            .filter_map(|endpoint| {
                let (ip, port, mode) = parseEndpoint(endpoint)?;
                Some(DiscoveredService {
                    fullName: format!("directory:{}:{endpoint}", self.nodeId),
                    hostname: String::new(),
                    addresses: vec![ip],
                    port,
                    properties: [
                        ("nodeId".into(), self.nodeId.clone()),
                        ("displayName".into(), self.displayName.clone()),
                        ("transports".into(), transportName(mode).into()),
                    ]
                    .into(),
                })
            })
            .collect()
    }
}
// The connection-code parser remains the authority for publishable endpoint syntax.
fn parseEndpoint(endpoint: &str) -> Option<(IpAddr, u16, PeerTransport)> {
    let mode = crate::PeerConnectionCode::endpointTransport(endpoint).ok()?;
    let url = url::Url::parse(endpoint).ok()?;
    if url.scheme() != transportName(mode) { return None; }
    let ip: IpAddr = url.host_str()?.trim_matches(['[', ']']).parse().ok()?;
    Some((ip, url.port_or_known_default()?, mode))
}

#[derive(Default)]
struct Registrations(BTreeMap<String, (u64, Announcement)>);
impl Registrations {
    fn snapshot(&mut self, now: u64) -> Vec<Announcement> {
        self.0.retain(|_, (expires, _)| *expires > now);
        self.0
            .values()
            .map(|(expires, record)| Announcement {
                ttlMs: expires - now,
                ..record.clone()
            })
            .collect()
    }
    fn put(&mut self, now: u64, record: Announcement) -> Result<(), CoreLinkError> {
        if !record.valid() {
            return Err(error("Invalid discovery announcement"));
        }
        self.0.retain(|_, (expires, _)| *expires > now);
        if self.0.len() >= MAX_PEERS - 1 && !self.0.contains_key(&record.nodeId) {
            return Err(error("Discovery directory is full"));
        }
        self.0.insert(
            record.nodeId.clone(),
            (now.saturating_add(record.ttlMs.min(LEASE_MS)), record),
        );
        Ok(())
    }
}
struct Worker {
    stop: oneshot::Sender<()>,
    done: oneshot::Receiver<()>,
}
#[derive(Default)]
pub(super) struct Directory {
    // Server registrations must never be renewed by republishing client query results.
    registered: Mutex<Registrations>,
    worker: Mutex<Option<Worker>>,
    refresh: AsyncMutex<()>,
    scope: Mutex<Option<PeerDiscoveryConfig>>,
}
impl Directory {
    pub(super) fn configure(&self, config: &Option<PeerDiscoveryConfig>) -> bool {
        let mut scope = self.scope.lock().unwrap_or_else(|p| p.into_inner());
        if &*scope == config {
            return false;
        }
        self.registered.lock().unwrap_or_else(|p| p.into_inner()).0.clear();
        *scope = config.clone();
        true
    }
}
fn response(status: u16, body: Vec<u8>) -> ServerResponse {
    let mut reply = ServerResponse::new(
        Full::new(Bytes::from(body))
            .map_err(|never| match never {})
            .boxed_unsync(),
    );
    *reply.status_mut() = http::StatusCode::from_u16(status).unwrap();
    reply
        .headers_mut()
        .insert("content-type", http::HeaderValue::from_static("application/json"));
    reply
        .headers_mut()
        .insert("cache-control", http::HeaderValue::from_static("no-store"));
    reply
}
fn key(secret: &str) -> ring::hmac::Key {
    ring::hmac::Key::new(ring::hmac::HMAC_SHA256, secret.as_bytes())
}
fn authorization(secret: &str) -> String {
    format!(
        "Bearer {}",
        BASE64.encode(ring::hmac::sign(&key(secret), PATH.as_bytes()).as_ref())
    )
}
fn authorized(secret: &str, header: &str) -> bool {
    !secret.is_empty()
        && header
            .strip_prefix("Bearer ")
            .and_then(|value| BASE64.decode(value).ok())
            .is_some_and(|tag| ring::hmac::verify(&key(secret), PATH.as_bytes(), &tag).is_ok())
}

impl HostRuntimePeerService {
    async fn ownAnnouncement(&self) -> Result<Option<Announcement>, CoreLinkError> {
        let config = self
            .state
            .store
            .hostConfig()
            .map_err(|failure| error(failure.to_string()))?
            .ok_or_else(|| error("Listener config is missing"))?;
        if !config.discoveryEnabled {
            return Ok(None);
        }
        let endpoints = match self.listeningAddresses().await {
            Ok(endpoints) => endpoints,
            Err(failure) if failure.code == "PAIRING_QR_UNAVAILABLE" => return Ok(None),
            Err(failure) => return Err(failure),
        };
        Ok(Some(Announcement {
            version: 1,
            nodeId: self.state.nodeId.clone(),
            displayName: self.state.info.displayName(),
            endpoints,
            ttlMs: LEASE_MS,
        }))
    }

    pub(super) async fn startDirectory(&self) -> Result<(), CoreLinkError> {
        let Some(config) = self.directoryConfig()?.filter(|config| config.directoryUrl.is_empty()) else {
            return Ok(());
        };
        let state = &self.state.discovery.directory;
        {
            let mut worker = state.worker.lock().unwrap_or_else(|p| p.into_inner());
            if worker
                .as_mut()
                .is_some_and(|worker| matches!(worker.done.try_recv(), Err(oneshot::error::TryRecvError::Empty)))
            {
                return Ok(());
            }
            worker.take();
        }
        let host = self
            .state
            .host
            .httpServerHost
            .as_ref()
            .ok_or_else(|| error("Directory requires an HTTP server Host"))?;
        let hostConfig = self
            .state
            .store
            .hostConfig()
            .map_err(|failure| error(failure.to_string()))?
            .ok_or_else(|| error("Listener config is missing"))?;
        let socket: SocketAddr = hostConfig
            .bindAddress
            .parse()
            .map_err(|_| error("Directory requires an IP bind address"))?;
        let listener = host
            .bind(&SocketAddr::new(socket.ip(), config.listenPort).to_string())
            .await
            .map_err(|failure| error(failure.to_string()))?;
        let scheduler = self
            .state
            .host
            .hostRuntimeTaskSchedulerHost
            .as_ref()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let state = Arc::downgrade(&self.state);
        let handler: HttpServerHandler = Arc::new(move |request| {
            let state = state.clone();
            Box::pin(async move {
                let Some(state) = state.upgrade() else {
                    return response(503, vec![]);
                };
                match (HostRuntimePeerService { state }).directoryRequest(request).await {
                    Ok(reply) => reply,
                    Err(failure) => {
                        operit_util::AppLogger::AppLogger::w(
                            "RuntimePeerService",
                            &format!("Directory request failed: {failure}"),
                        );
                        response(503, vec![])
                    }
                }
            })
        });
        let (stop, stopped) = oneshot::channel();
        let (done, finished) = oneshot::channel();
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "peer-directory",
                Box::new(move || {
                    Box::pin(async move {
                        if let Err(failure) = listener
                            .serve(
                                handler,
                                Box::pin(async move {
                                    let _ = stopped.await;
                                }),
                            )
                            .await
                        {
                            operit_util::AppLogger::AppLogger::w(
                                "RuntimePeerService",
                                &format!("Directory listener stopped: {failure}"),
                            );
                        }
                        let _ = done.send(());
                    })
                }),
            )
            .map_err(|failure| error(failure.to_string()))?;
        *self
            .state
            .discovery
            .directory
            .worker
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(Worker { stop, done: finished });
        Ok(())
    }

    pub(super) async fn stopDirectory(&self) {
        let worker = self
            .state
            .discovery
            .directory
            .worker
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(Worker { stop, done }) = worker {
            let _ = stop.send(());
            let _ = done.await;
        }
        self.state
            .discovery
            .directory
            .registered
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .0
            .clear();
    }

    async fn directoryRequest(&self, request: ServerRequest) -> Result<ServerResponse, CoreLinkError> {
        if request.uri().path() != PATH {
            return Ok(response(404, vec![]));
        }
        if request.method() != http::Method::POST {
            return Ok(response(405, vec![]));
        }
        let Some(config) = self.directoryConfig()?.filter(|config| config.directoryUrl.is_empty()) else {
            return Ok(response(503, vec![]));
        };
        if !authorized(
            &config.groupKey,
            request
                .headers()
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
        ) {
            return Ok(response(401, vec![]));
        }
        let body = match self
            .timeout(
                Duration::from_secs(3),
                Limited::new(request.into_body(), 4096).collect(),
            )
            .await
        {
            Ok(Ok(body)) => body.to_bytes(),
            _ => return Ok(response(413, vec![])),
        };
        let announcement: Option<Announcement> = match serde_json::from_slice(&body) {
            Ok(value) => value,
            Err(_) => return Ok(response(400, vec![])),
        };
        if announcement
            .as_ref()
            .is_some_and(|record| !record.valid() || record.nodeId == self.state.nodeId)
        {
            return Ok(response(400, vec![]));
        }
        if self.directoryConfig()?.as_ref() != Some(&config) {
            return Ok(response(503, vec![]));
        }
        let now = self.discoveryNow()?;
        let mut records = {
            let mut registrations = self
                .state
                .discovery
                .directory
                .registered
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if let Some(record) = announcement {
                if registrations.put(now, record).is_err() {
                    return Ok(response(429, vec![]));
                }
            }
            registrations.snapshot(now)
        };
        self.rememberDirectoryRecords(&records)?;
        if let Some(own) = self.ownAnnouncement().await? {
            records.insert(0, own);
        }
        loop {
            let body = serde_json::to_vec(&records).map_err(|failure| error(failure.to_string()))?;
            if body.len() <= MAX_RESPONSE {
                return Ok(response(200, body));
            }
            records.pop();
        }
    }

    /// Returns whether a directory source was queried; absence is not a successful query.
    pub(super) async fn refreshDirectory(&self, timeoutMs: u64) -> Result<bool, CoreLinkError> {
        let Some(config) = self.directoryConfig()? else {
            return Ok(false);
        };
        if config.directoryUrl.is_empty() {
            return Ok(true);
        }
        self.timeout(Duration::from_millis(timeoutMs.max(1)), async {
            let _guard = self.state.discovery.directory.refresh.lock().await;
            let own = self.ownAnnouncement().await?;
            let records = self
                .directoryHttp(
                    format!("{}{PATH}", config.directoryUrl.trim_end_matches('/')),
                    &config.groupKey,
                    serde_json::to_vec(&own).map_err(|failure| error(failure.to_string()))?,
                )
                .await?;
            if records.len() > MAX_PEERS || records.iter().any(|record| !record.valid()) {
                return Err(error("Invalid directory response"));
            }
            if self.directoryConfig()?.as_ref() != Some(&config) {
                return Err(error("Discovery group changed"));
            }
            self.rememberDirectoryRecords(&records)?;
            Ok(true)
        })
        .await
        .map_err(|failure| error(failure.to_string()))?
    }
    async fn directoryHttp(
        &self,
        url: String,
        secret: &str,
        body: Vec<u8>,
    ) -> Result<Vec<Announcement>, CoreLinkError> {
        let host = self
            .state
            .host
            .httpHost
            .clone()
            .ok_or_else(|| error("HTTP Host is not installed"))?;
        let id = format!("discovery-{}", uuid::Uuid::new_v4());
        struct Stream {
            host: Arc<dyn operit_host_api::HttpHost>,
            id: String,
        }
        impl Drop for Stream {
            fn drop(&mut self) {
                let _ = self.host.closeHttpByteStream(&self.id);
            }
        }
        let _stream = Stream {
            host: host.clone(),
            id: id.clone(),
        };
        let (sender, receiver) = oneshot::channel();
        let sender = Arc::new(Mutex::new(Some(sender)));
        let data = Arc::new(Mutex::new(Vec::new()));
        let opened = sender.clone();
        let chunks = sender.clone();
        let bytes = data.clone();
        let headers = vec![
            ("authorization".into(), authorization(secret)),
            ("content-type".into(), "application/json".into()),
        ];
        host.openHttpResponseStream(
            id,
            HttpRequestData {
                url,
                method: "POST".into(),
                headers,
                body,
                formFields: vec![],
                fileParts: vec![],
                connectTimeoutSeconds: 2,
                readTimeoutSeconds: 3,
                followRedirects: false,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            },
            Arc::new(move |metadata| {
                if metadata.statusCode != 200 {
                    if let Some(sender) = opened.lock().unwrap().take() {
                        let _ = sender.send(Err(error(format!("Discovery HTTP status {}", metadata.statusCode))));
                    }
                }
            }),
            Arc::new(move |chunk| {
                let mut data = bytes.lock().unwrap();
                if data.len().saturating_add(chunk.len()) > MAX_RESPONSE {
                    if let Some(sender) = chunks.lock().unwrap().take() {
                        let _ = sender.send(Err(error("Discovery response exceeds size limit")));
                    }
                } else {
                    data.extend_from_slice(&chunk);
                }
            }),
            Arc::new(move |result| {
                let completed = sender.lock().unwrap().take();
                if let Some(sender) = completed {
                    let result = result.map_err(|failure| error(failure.to_string())).and_then(|_| {
                        serde_json::from_slice(&data.lock().unwrap()).map_err(|failure| error(failure.to_string()))
                    });
                    let _ = sender.send(result);
                }
            }),
        )
        .map_err(|failure| error(failure.to_string()))?;
        receiver.await.map_err(|_| error("Discovery request cancelled"))?
    }
}

#[cfg(test)]
#[path = "directory_tests.rs"]
mod tests;
