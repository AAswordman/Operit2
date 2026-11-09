use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use operit_link::CorePushRequest;

/// Exercises the generated constructor-backed repository and binary reverse stream
/// used by Flutter, including a payload above the desktop's single-frame budget.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn large_fonts_roundtrip_through_generated_storage_push_and_chunked_calls() {
    let _guard = SHARED_CONCURRENCY_TEST_LOCK.lock().await;
    let storage = register_test_runtime_roots();
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostSecretStore = Some(Arc::new(TestSecretStore::default()));
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    ui_resource_tests::configure_package_hosts(&mut hosts);
    let proxy = LocalCoreProxy::new(OperitApplication::newWithContext(hosts));
    operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone())
        .writeNodeId("font-proxy-test".into())
        .unwrap();
    let target =
        LocalCoreProxy::generatedTargetForSchema("repository.runtimeStorageRepository").unwrap();
    let response = CoreLinkSharedClient::call(
        &proxy,
        CoreCallRequest::new(
            "font-directory",
            target,
            "themeAssetsDirPath",
            CoreValue::emptyMap(),
        ),
    )
    .await;
    let directory: String = fromCoreValue(response.result.unwrap()).unwrap();
    let path = format!("{directory}/large-proxy-font.ttc");
    let chunk_bytes = 1024 * 1024;
    let size = 65 * chunk_bytes;
    let request = CorePushRequest::new("large-font-upload", target, "writeBytesStream")
        .withArgs(toCoreValue(json!({"path":path, "expectedByteLength":size})).unwrap());
    assert!(proxy.isReverseStreamRequest(&request));
    tokio::time::timeout(Duration::from_secs(60), async {
        let mut stream = CoreLinkSharedClient::openPush(&proxy, request)
            .await
            .unwrap();
        for index in 0..65 {
            stream
                .send(CoreValue::Bytes(vec![index as u8; chunk_bytes].into()))
                .await
                .unwrap();
        }
        stream.close().await.unwrap();
        for index in 0..65 {
            let response = CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    format!("font-read-{index}"),
                    target,
                    "readBase64Chunk",
                    toCoreValue(
                        json!({"path":path, "offset": index * chunk_bytes, "length":chunk_bytes}),
                    )
                    .unwrap(),
                ),
            )
            .await;
            let encoded: Option<String> = fromCoreValue(response.result.unwrap()).unwrap();
            let chunk = STANDARD.decode(encoded.unwrap()).unwrap();
            assert_eq!(chunk.len(), chunk_bytes);
            assert!(chunk.iter().all(|value| *value == index as u8));
        }
        // An incomplete replacement must fail at close and retain the large font.
        let request = CorePushRequest::new("incomplete-font", target, "writeBytesStream")
            .withArgs(toCoreValue(json!({"path":path, "expectedByteLength":3})).unwrap());
        let mut stream = CoreLinkSharedClient::openPush(&proxy, request)
            .await
            .unwrap();
        stream.send(CoreValue::Bytes(vec![9].into())).await.unwrap();
        assert!(stream.close().await.is_err());
        let bytes = storage.readBytes(&path).unwrap();
        assert_eq!(bytes.len(), size);
        assert_eq!(bytes[chunk_bytes], 1);
        assert_eq!(bytes[size - 1], 64);
    })
    .await
    .expect("font proxy transfer must complete");
}
