use super::*;
use operit_host_api::{
    FileEntry, FileExistence, FileInfo, FindFilesRequest, GrepCodeRequest, GrepCodeResult,
    HostEnvironmentDescriptor, HostResult,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn sessions_share_resources_until_package_revision_changes() {
    let cache = ToolPkgTextResourceCache::default();
    let (first, hit) = cache
        .get_or_load("roles", "v1", || {
            Ok(BTreeMap::from([("screen.js".into(), "old".into())]))
        })
        .unwrap();
    assert!(!hit);
    let (second, hit) = cache
        .get_or_load("roles", "v1", || {
            panic!("repeat session must not reread files")
        })
        .unwrap();
    assert!(hit);
    assert!(Arc::ptr_eq(&first, &second));
    let (updated, hit) = cache
        .get_or_load("roles", "v2", || {
            Ok(BTreeMap::from([("screen.js".into(), "new".into())]))
        })
        .unwrap();
    assert!(!hit);
    assert_eq!(updated["screen.js"], "new");
    assert_eq!(
        first["screen.js"], "old",
        "existing sessions retain their immutable revision"
    );
}

#[test]
fn concurrent_sessions_build_one_package_snapshot() {
    let cache = ToolPkgTextResourceCache::default();
    let loads = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    cache
                        .get_or_load("roles", "v1", || {
                            loads.fetch_add(1, Ordering::SeqCst);
                            Ok(BTreeMap::new())
                        })
                        .unwrap()
                        .0
                })
            })
            .collect();
        let snapshots: Vec<_> = tasks.into_iter().map(|task| task.join().unwrap()).collect();
        assert!(snapshots
            .iter()
            .all(|snapshot| Arc::ptr_eq(snapshot, &snapshots[0])));
    });
    assert_eq!(loads.load(Ordering::SeqCst), 1);
}

#[test]
fn invalidation_during_load_does_not_republish_old_resources() {
    let cache = ToolPkgTextResourceCache::default();
    cache
        .get_or_load("roles", "v1", || {
            cache.invalidate("roles");
            // Other packages can also load while this package's loader is running.
            cache.get_or_load("other", "v1", || Ok(BTreeMap::new()))?;
            Ok(BTreeMap::from([("screen.js".into(), "old".into())]))
        })
        .unwrap();
    let (fresh, hit) = cache
        .get_or_load("roles", "v1", || {
            Ok(BTreeMap::from([("screen.js".into(), "fresh".into())]))
        })
        .unwrap();
    assert!(!hit);
    assert_eq!(fresh["screen.js"], "fresh");
    cache.retain_packages(&BTreeSet::from(["roles".into()]));
    assert!(
        !cache
            .get_or_load("other", "v1", || Ok(BTreeMap::new()))
            .unwrap()
            .1
    );
    cache.clear();
    assert!(
        !cache
            .get_or_load("roles", "v1", || Ok(BTreeMap::new()))
            .unwrap()
            .1
    );
}

#[test]
fn failed_load_is_retried_without_caching_partial_resources() {
    let cache = ToolPkgTextResourceCache::default();
    assert!(cache
        .get_or_load("roles", "v1", || Err("read failed".into()))
        .is_err());
    assert!(
        !cache
            .get_or_load("roles", "v1", || Ok(BTreeMap::new()))
            .unwrap()
            .1
    );
}

#[derive(Default)]
struct ResourceHost {
    reads: Mutex<Vec<String>>,
    fail: bool,
    invalid_name: bool,
    corrupt_protection: bool,
}

impl FileSystemHost for ResourceHost {
    fn envLabel(&self) -> &str {
        "test"
    }
    fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
        HostEnvironmentDescriptor::default()
    }
    /// Accepts host-native separators when listing the virtual fixture directory.
    fn listFiles(&self, path: &str) -> HostResult<Vec<FileEntry>> {
        let normalized = path.replace('\\', "/");
        let path = normalized.as_str();
        if self.invalid_name {
            return Ok(vec![FileEntry {
                name: "../outside".into(),
                isDirectory: true,
                size: 0,
                permissions: String::new(),
                lastModified: String::new(),
            }]);
        }
        let names = match path {
            "/cache" => vec![
                ("dist", true),
                ("image.png", false),
                (super::super::TOOLPKG_CACHE_SIGNATURE_FILE, false),
            ],
            "/cache/dist" => vec![("Screen.js", false), ("protected.json", false)],
            _ => panic!("unexpected directory: {path}"),
        };
        Ok(names
            .into_iter()
            .map(|(name, isDirectory)| FileEntry {
                name: name.into(),
                isDirectory,
                size: 0,
                permissions: String::new(),
                lastModified: String::new(),
            })
            .collect())
    }
    /// Accepts host-native separators when reading a virtual fixture resource.
    fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        let normalized = path.replace('\\', "/");
        let path = normalized.as_str();
        self.reads.lock().unwrap().push(path.into());
        if self.fail {
            return Err(operit_host_api::HostError::new("read failed"));
        }
        Ok(match path {
            "/cache/dist/Screen.js" => b"exports.default = Screen;".to_vec(),
            "/cache/dist/protected.json" => {
                let mut bytes = ToolPkgProtection::encrypt(b"{\"name\":\"role\"}").unwrap();
                if self.corrupt_protection {
                    *bytes.last_mut().unwrap() ^= 1;
                }
                bytes
            }
            "/cache/image.png" => vec![0xff, 0xfe],
            _ => panic!("unexpected resource: {path}"),
        })
    }
    fn validatePath(&self, _: &str, _: &str) -> HostResult<()> {
        panic!("unexpected validation")
    }
    fn readFile(&self, _: &str) -> HostResult<String> {
        panic!("must not reread cache signatures")
    }
    fn readFileWithLimit(&self, _: &str, _: usize) -> HostResult<String> {
        unimplemented!()
    }
    fn writeFile(&self, _: &str, _: &str, _: bool) -> HostResult<()> {
        unimplemented!()
    }
    fn writeFileBytes(&self, _: &str, _: &[u8]) -> HostResult<()> {
        unimplemented!()
    }
    fn deleteFile(&self, _: &str, _: bool) -> HostResult<()> {
        unimplemented!()
    }
    fn fileExists(&self, _: &str) -> HostResult<FileExistence> {
        panic!("must not revalidate a prepared cache per resource")
    }
    fn moveFile(&self, _: &str, _: &str) -> HostResult<()> {
        unimplemented!()
    }
    fn copyFile(&self, _: &str, _: &str, _: bool) -> HostResult<()> {
        unimplemented!()
    }
    fn makeDirectory(&self, _: &str, _: bool) -> HostResult<()> {
        unimplemented!()
    }
    fn findFiles(&self, _: FindFilesRequest) -> HostResult<Vec<String>> {
        unimplemented!()
    }
    fn fileInfo(&self, _: &str) -> HostResult<FileInfo> {
        unimplemented!()
    }
    fn grepCode(&self, _: GrepCodeRequest) -> HostResult<GrepCodeResult> {
        unimplemented!()
    }
    fn zipFiles(&self, _: &str, _: &str) -> HostResult<()> {
        unimplemented!()
    }
    fn unzipFiles(&self, _: &str, _: &str) -> HostResult<()> {
        unimplemented!()
    }
    fn openFile(&self, _: &str) -> HostResult<()> {
        unimplemented!()
    }
    fn shareFile(&self, _: &str, _: &str) -> HostResult<()> {
        unimplemented!()
    }
}

#[test]
fn prepared_directory_reads_each_file_once_and_preserves_protected_resources() {
    let host = ResourceHost::default();
    let cache = ToolPkgTextResourceCache::default();
    let (resources, hit) = cache
        .get_or_load("roles", "v1", || {
            read_text_resources(&host, Path::new("/cache"))
        })
        .unwrap();
    assert!(!hit);
    let (reopened, hit) = cache
        .get_or_load("roles", "v1", || {
            read_text_resources(&host, Path::new("/cache"))
        })
        .unwrap();
    assert!(hit);
    assert!(Arc::ptr_eq(&resources, &reopened));
    assert_eq!(resources.len(), 2);
    assert_eq!(resources["dist/screen.js"], "exports.default = Screen;");
    assert_eq!(resources["dist/protected.json"], "{\"name\":\"role\"}");
    assert_eq!(host.reads.lock().unwrap().len(), 3);
}

#[test]
fn unreadable_resource_fails_snapshot_instead_of_publishing_incomplete_modules() {
    let host = ResourceHost {
        fail: true,
        ..Default::default()
    };
    assert!(read_text_resources(&host, Path::new("/cache"))
        .unwrap_err()
        .contains("read failed"));
}

#[test]
fn malformed_protected_resource_fails_the_snapshot() {
    let host = ResourceHost {
        corrupt_protection: true,
        ..Default::default()
    };
    assert!(read_text_resources(&host, Path::new("/cache"))
        .unwrap_err()
        .contains("Cannot decrypt"));
}

#[test]
fn directory_entries_cannot_escape_the_validated_resource_root() {
    let host = ResourceHost {
        invalid_name: true,
        ..Default::default()
    };
    assert!(read_text_resources(&host, Path::new("/cache"))
        .unwrap_err()
        .contains("Invalid ToolPkg resource name"));
    assert!(host.reads.lock().unwrap().is_empty());
}
