use super::*;
use crate::UpdateHttp::tests::Callbacks;

enum Reply {
    Bytes(Vec<u8>),
    Interrupted,
}

struct Source {
    sample: Option<Vec<u8>>,
    delay: usize,
    reply: Reply,
}

#[derive(Default)]
struct SourcesHost {
    sources: BTreeMap<String, Source>,
    clock: Arc<Clock>,
    downloads: Mutex<Vec<String>>,
    pending: Mutex<BTreeMap<String, Callbacks>>,
    opened: AtomicUsize,
    closed: AtomicUsize,
}

impl HttpStreamHost for SourcesHost {
    /// Rejects transports without response metadata.
    fn openHttpByteStream(
        &self,
        _: String,
        _: HttpRequestData,
        _: HttpStreamOpenedCallback,
        _: HttpStreamChunkCallback,
        _: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        panic!("response metadata required")
    }

    /// Separates speed samples from actual downloads and can leave probes pending for cancellation tests.
    fn openHttpResponseStream(
        &self,
        id: String,
        request: HttpRequestData,
        onHead: HttpStreamResponseCallback,
        onBytes: HttpStreamChunkCallback,
        onClosed: HttpStreamClosedCallback,
    ) -> HostResult<()> {
        self.opened.fetch_add(1, Ordering::SeqCst);
        let probing = request.headers.iter().any(|(name, _)| name == "Range");
        if !probing {
            self.downloads.lock().unwrap().push(request.url.clone());
        }
        let source = self
            .sources
            .get(&request.url)
            .ok_or_else(|| HostError::new("unreachable fixture source"))?;
        if probing {
            assert!(request
                .headers
                .iter()
                .any(|(name, value)| name == "Accept-Encoding" && value == "identity"));
            let Some(sample) = &source.sample else {
                self.pending.lock().unwrap().insert(
                    id,
                    Callbacks {
                        head: onHead,
                        bytes: onBytes,
                        closed: onClosed,
                    },
                );
                return Ok(());
            };
            self.clock.0.fetch_add(source.delay, Ordering::SeqCst);
            onHead(head(sample.len()));
            onBytes(sample.clone());
            onClosed(Ok(()));
        } else {
            match &source.reply {
                Reply::Bytes(body) => {
                    onHead(head(body.len()));
                    onBytes(body.clone());
                    onClosed(Ok(()));
                }
                Reply::Interrupted => {
                    onHead(head(MSI.len()));
                    onBytes(MSI[..8].to_vec());
                    onClosed(Err("connection interrupted".into()));
                }
            }
        }
        Ok(())
    }

    /// Tracks closure of every completed, timed-out, rejected, or cancelled source.
    fn closeHttpByteStream(&self, id: &str) -> HostResult<()> {
        self.closed.fetch_add(1, Ordering::SeqCst);
        self.pending.lock().unwrap().remove(id);
        Ok(())
    }
}

impl HttpHost for SourcesHost {
    /// Keeps test data in memory.
    fn imageDelivery(&self) -> HttpImageDelivery {
        HttpImageDelivery::Bytes
    }
    /// Rejects synchronous network requests.
    fn executeHttpRequest(&self, _: HttpRequestData) -> HostResult<HttpResponseData> {
        panic!("async only")
    }
    /// Rejects downloads outside transactional runtime storage.
    fn downloadFiles(
        &self,
        _: HttpDownloadRequest,
        _: HttpDownloadControl,
        _: HttpDownloadProgressCallback,
    ) -> HostResult<HttpDownloadResult> {
        panic!("streaming only")
    }
}

/// Creates a reachable source with a controlled sample latency and independent transfer result.
fn source(delay: usize, reply: Reply) -> Source {
    Source {
        sample: Some(MSI.to_vec()),
        delay,
        reply,
    }
}

/// Fast source interruptions and valid-looking corrupt packages advance once to the next fastest source.
#[tokio::test]
async fn retries_in_speed_order_and_discards_failed_files_before_switching() {
    let fixture = Fixture::new();
    let original = packageUrl();
    let mirror1 = format!("https://ghfast.top/{original}");
    let mirror2 = format!("https://ghproxy.com/{original}");
    let unused = format!("https://ghproxy.net/{original}");
    let html = format!("https://gh-proxy.com/{original}");
    let host = Arc::new(SourcesHost {
        sources: BTreeMap::from([
            (original.clone(), source(1, Reply::Interrupted)),
            (
                mirror1.clone(),
                source(40, Reply::Bytes([MSI, b"corrupt"].concat())),
            ),
            (mirror2.clone(), source(100, Reply::Bytes(MSI.to_vec()))),
            (unused, source(200, Reply::Bytes(MSI.to_vec()))),
            (
                html,
                Source {
                    sample: Some(b"<html>gateway</html>".to_vec()),
                    delay: 0,
                    reply: Reply::Interrupted,
                },
            ),
        ]),
        ..Default::default()
    });
    let context = fixture
        .context
        .clone()
        .withHttpHost(host.clone())
        .withHostRuntimeTaskSchedulerHost(host.clock.clone());
    let output = format!("{RUNTIME_APPLICATION_UPDATES_DIR_PATH}/source-test/{NAME}");
    let hash = format!("{:x}", Sha256::digest(MSI));
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    downloadUpdatePackage(
        &original,
        &output,
        NAME,
        &context,
        HttpDownloadControl::new(),
        Some(&hash),
        move |event| sink.lock().unwrap().push(event),
    )
    .await
    .unwrap();
    assert_eq!(
        *host.downloads.lock().unwrap(),
        [original, mirror1, mirror2]
    );
    assert_eq!(fixture.storage.readBytes(&output).unwrap(), MSI);
    assert_eq!(fixture.storage.pending.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.storage.discarded.load(Ordering::SeqCst), 2);
    assert_eq!(
        host.opened.load(Ordering::SeqCst),
        host.closed.load(Ordering::SeqCst)
    );
    assert_eq!(
        events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches!(
                event,
                FullUpdateProgressEvent::StageChanged {
                    stage: FullUpdateStage::SelectingSource,
                    ..
                }
            ))
            .count(),
        3
    );
}

/// All probes are concurrent and cancellation closes every pending request before any package is written.
#[tokio::test]
async fn cancellation_stops_parallel_probes_without_starting_a_download() {
    let fixture = Fixture::new();
    let url = packageUrl();
    let host = Arc::new(SourcesHost {
        sources: BTreeMap::from([
            (
                url.clone(),
                Source {
                    sample: None,
                    delay: 0,
                    reply: Reply::Interrupted,
                },
            ),
            (
                format!("https://ghfast.top/{url}"),
                Source {
                    sample: None,
                    delay: 0,
                    reply: Reply::Interrupted,
                },
            ),
        ]),
        ..Default::default()
    });
    let context = fixture
        .context
        .clone()
        .withHttpHost(host.clone())
        .withHostRuntimeTaskSchedulerHost(host.clock.clone());
    let control = HttpDownloadControl::new();
    let mut pending = Box::pin(GithubReleaseUtil::downloadAndPrepareFullUpdateControlled(
        url,
        NAME.into(),
        &context,
        control.clone(),
        |_| {},
    ));
    assertPending(pending.as_mut());
    assert_eq!(host.pending.lock().unwrap().len(), 2);
    assert_eq!(host.opened.load(Ordering::SeqCst), 35);
    control.cancel();
    assert_eq!(pending.await.unwrap_err(), "UPDATE_CANCELLED");
    assert!(host.pending.lock().unwrap().is_empty());
    assert!(host.downloads.lock().unwrap().is_empty());
    assert_eq!(
        host.opened.load(Ordering::SeqCst),
        host.closed.load(Ordering::SeqCst)
    );
    fixture.assertOnlyOtherTask();
}

/// Exhausted sources never publish a package whose hash disagrees with authenticated release metadata.
#[tokio::test]
async fn exhausted_corrupt_sources_leave_no_installable_package() {
    let fixture = Fixture::new();
    let url = packageUrl();
    let host = Arc::new(SourcesHost {
        sources: BTreeMap::from([(url.clone(), source(1, Reply::Bytes([MSI, b"bad"].concat())))]),
        ..Default::default()
    });
    let context = fixture
        .context
        .clone()
        .withHttpHost(host.clone())
        .withHostRuntimeTaskSchedulerHost(host.clock.clone());
    let output = format!("{RUNTIME_APPLICATION_UPDATES_DIR_PATH}/source-test/{NAME}");
    let error = downloadUpdatePackage(
        &url,
        &output,
        NAME,
        &context,
        HttpDownloadControl::new(),
        Some(&format!("{:x}", Sha256::digest(MSI))),
        |_| {},
    )
    .await
    .unwrap_err();
    assert!(error.starts_with("UPDATE_INTEGRITY:"));
    assert!(!fixture.storage.exists(&output).unwrap());
    assert_eq!(fixture.storage.discarded.load(Ordering::SeqCst), 1);
    fixture.assertOnlyOtherTask();
}

/// A Host deadline terminates an unresponsive probe even when the transport emits no callbacks.
#[tokio::test]
async fn host_deadline_closes_a_stalled_probe() {
    let fixture = Fixture::new();
    let url = packageUrl();
    let host = Arc::new(SourcesHost {
        sources: BTreeMap::from([(
            url.clone(),
            Source {
                sample: None,
                delay: 0,
                reply: Reply::Interrupted,
            },
        )]),
        ..Default::default()
    });
    let context = fixture
        .context
        .clone()
        .withHttpHost(host.clone())
        .withHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
        ));
    let error =
        crate::UpdateSources::ranked(&url, NAME, &context, HttpDownloadControl::new(), false)
            .await
            .unwrap_err();
    assert!(error.starts_with("Download source timed out"));
    assert!(host.pending.lock().unwrap().is_empty());
    assert_eq!(
        host.opened.load(Ordering::SeqCst),
        host.closed.load(Ordering::SeqCst)
    );
}
