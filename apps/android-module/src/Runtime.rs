//! Privileged Android Host lifecycle, called through JNI from the shared Android artifact.

use crate::LocalRuntimeTransport::ModuleEndpoint;
use crate::PlatformRuntimeFactory::{BridgeStartup, StartupMetadata};
use fs2::FileExt;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    os::unix::fs::{chown, symlink, MetadataExt, OpenOptionsExt, PermissionsExt},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

static STOPPING: AtomicBool = AtomicBool::new(false);
const CLIENT_DATA_DIRECTORY: &str = "/data/user/0/app.operit.module";

/// Enters the privileged platform lifecycle from the shared APK's app_process entry point.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_ModuleMain_runRuntime(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    deployment: jni::objects::JString,
    data_root: jni::objects::JString,
    model: jni::objects::JString,
    language: jni::objects::JString,
    client_uid: jni::sys::jint,
) {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
            let deployment: String = env
                .get_string(&deployment)
                .map_err(|e| e.to_string())?
                .into();
            let data_root: String = env
                .get_string(&data_root)
                .map_err(|e| e.to_string())?
                .into();
            let model: String = env.get_string(&model).map_err(|e| e.to_string())?.into();
            let language: String = env.get_string(&language).map_err(|e| e.to_string())?.into();
            let client_uid = u32::try_from(client_uid).map_err(|e| e.to_string())?;
            run(
                deployment.into(),
                data_root.into(),
                model,
                language,
                client_uid,
            )
        }))
        .unwrap_or_else(|_| Err("Privileged Android Host panicked".into()));
    if let Err(error) = result {
        let _ = env.throw_new("java/lang/IllegalStateException", error);
    }
}

/// Copies the package's full SELinux label, including its per-UID MLS categories.
fn inherit_application_label(path: &Path, application: &Path) -> Result<(), String> {
    let source = std::ffi::CString::new(application.as_os_str().as_encoded_bytes())
        .map_err(|e| e.to_string())?;
    let target =
        std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
    let attribute = b"security.selinux\0";
    let mut label = [0u8; 256];
    let length = unsafe {
        libc::getxattr(
            source.as_ptr(),
            attribute.as_ptr().cast(),
            label.as_mut_ptr().cast(),
            label.len(),
        )
    };
    if length <= 0 {
        return Err(format!(
            "Cannot read application SELinux label: {}",
            std::io::Error::last_os_error()
        ));
    }
    let result = unsafe {
        libc::setxattr(
            target.as_ptr(),
            attribute.as_ptr().cast(),
            label.as_ptr().cast(),
            length as usize,
            0,
        )
    };
    if result != 0 {
        return Err(format!(
            "Cannot apply module client SELinux label: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

/// Records termination without allocating or calling non-signal-safe runtime services.
extern "C" fn terminate(_: libc::c_int) {
    STOPPING.store(true, Ordering::Release);
}

/// Publishes the client credential only inside the installed package's UID-owned sandbox.
fn publish_client(bytes: &[u8], uid: u32) -> Result<(), String> {
    let application = Path::new(CLIENT_DATA_DIRECTORY);
    let metadata = fs::metadata(application).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != uid || !(10000..100000).contains(&uid) {
        return Err("Module client UID does not own the installed application sandbox".into());
    }
    let files = application.join("files");
    if !files.exists() {
        private_dir(&files)?;
        chown(&files, Some(uid), Some(uid)).map_err(|e| e.to_string())?;
        inherit_application_label(&files, application)?;
    }
    let directory = files.join("host-runtime");
    private_dir(&directory)?;
    chown(&directory, Some(uid), Some(uid)).map_err(|e| e.to_string())?;
    let endpoint = directory.join("client.json");
    write_private(&endpoint, bytes)?;
    chown(&endpoint, Some(uid), Some(uid)).map_err(|e| e.to_string())?;
    inherit_application_label(&directory, application)?;
    inherit_application_label(&endpoint, application)?;
    Ok(())
}

/// Creates a private root-owned directory and refuses symlink-based deployment redirection.
fn private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!("Not a real module directory: {}", path.display()));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())
}

/// Commits module control files through an exclusive private staging file.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&temporary, path).map_err(|e| e.to_string())
}

/// Reports lifecycle state without including any transport or storage credentials.
fn status(root: &Path, state: &str, detail: &str) -> Result<(), String> {
    write_private(
        &root.join("run/status.json"),
        &serde_json::to_vec(&serde_json::json!({
            "state": state, "detail": detail, "pid": std::process::id(),
            "updatedAt": operit_host_api::TimeUtils::currentTimeMillis(),
        }))
        .map_err(|e| e.to_string())?,
    )
}

/// Verifies the packaged environment before installing it once, preserving later user changes.
fn prepare_environment(
    module: &Path,
    user: &Path,
    runtime: &Path,
    workspace: &Path,
) -> Result<(), String> {
    let host = user.join("host");
    private_dir(&host)?;
    let runtime_dir = host.join("android-runtime");
    private_dir(&runtime_dir)?;
    let rootfs = host.join("rootfs");
    let libraries = module.join("system/app/Operit/lib/arm64");
    let archive = module.join("runtime/assets/rootfs.tar.gz");
    let packaged = fs::read_to_string(module.join("runtime/assets/rootfs.tar.gz.sha256"))
        .map_err(|e| e.to_string())?;
    let expected = packaged
        .split_whitespace()
        .next()
        .ok_or("Missing rootfs checksum")?;
    let marker = host.join("rootfs.sha256");
    if marker.exists() {
        if fs::read_to_string(&marker)
            .map_err(|e| e.to_string())?
            .trim()
            != expected
            || !rootfs.is_dir()
        {
            return Err(
                "Installed rootfs does not match this module; preserving the existing environment"
                    .into(),
            );
        }
    } else {
        if rootfs.exists() {
            return Err("Uncommitted rootfs exists; inspect it before installing".into());
        }
        let mut file = fs::File::open(&archive).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            let length = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if length == 0 {
                break;
            }
            hasher.update(&buffer[..length]);
        }
        if format!("{:x}", hasher.finalize()) != expected {
            return Err("Packaged rootfs checksum mismatch".into());
        }
        let staging = host.join(format!("rootfs.install-{}", uuid::Uuid::new_v4()));
        private_dir(&staging)?;
        let exit = Command::new(libraries.join("liboperit_busybox.so"))
            .arg0("busybox")
            .args(["tar", "-xzf"])
            .arg(&archive)
            .arg("-C")
            .arg(&staging)
            .status()
            .map_err(|e| e.to_string())?;
        if !exit.success() {
            fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
            return Err(format!("Rootfs extraction failed: {exit}"));
        }
        fs::rename(&staging, &rootfs).map_err(|e| e.to_string())?;
        write_private(&marker, expected.as_bytes())?;
    }
    for (name, library) in [
        ("busybox", "liboperit_busybox.so"),
        ("bash", "libbash.so"),
        ("proot", "liboperit_proot.so"),
        ("loader", "liboperit_loader.so"),
    ] {
        let source = libraries.join(library);
        let target = runtime_dir.join(name);
        if !source.is_file() {
            return Err(format!("Missing module executable: {library}"));
        }
        match fs::symlink_metadata(&target) {
            Ok(metadata) => {
                if !metadata.file_type().is_symlink() {
                    return Err(format!(
                        "Unexpected runtime executable: {}",
                        target.display()
                    ));
                }
                if fs::read_link(&target).map_err(|e| e.to_string())? != source {
                    let staging = runtime_dir.join(format!(".{name}-{}", uuid::Uuid::new_v4()));
                    symlink(&source, &staging).map_err(|e| e.to_string())?;
                    fs::rename(staging, &target).map_err(|e| e.to_string())?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                symlink(source, target).map_err(|e| e.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let temporary = runtime_dir.join("tmp");
    private_dir(&temporary)?;
    for (name, path) in [
        ("OPERIT_ANDROID_RUNTIME_DIR", runtime_dir.clone()),
        ("OPERIT_ANDROID_ROOTFS_DIR", rootfs.clone()),
        ("OPERIT_ANDROID_NATIVE_LIBRARY_DIR", libraries),
        ("OPERIT_ANDROID_INTERNAL_ROOT", host.clone()),
        ("OPERIT_ANDROID_RUNTIME_TMP", temporary),
        ("OPERIT_ANDROID_RUNTIME_ROOT", runtime.to_path_buf()),
        ("OPERIT_ANDROID_WORKSPACE_ROOT", workspace.to_path_buf()),
        ("OPERIT_ANDROID_BUSYBOX", runtime_dir.join("busybox")),
        ("OPERIT_ANDROID_BASH", runtime_dir.join("bash")),
        ("OPERIT_ANDROID_PROOT", runtime_dir.join("proot")),
        ("OPERIT_ANDROID_LOADER", runtime_dir.join("loader")),
    ] {
        std::env::set_var(name, path);
    }
    for path in [&host, runtime, workspace, Path::new("/data/local/tmp")] {
        fs::create_dir_all(rootfs.join(path.strip_prefix("/").map_err(|e| e.to_string())?))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Runs the shared Core with privileged Android Hosts and explicitly supplied deployment roots.
pub(crate) fn run(
    deployment: PathBuf,
    root: PathBuf,
    model: String,
    language: String,
    client_uid: u32,
) -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("The Android module must run as Root".into());
    }
    if model.trim().is_empty() || language.trim().is_empty() {
        return Err("Privileged Android Host requires a device model and language".into());
    }
    let module = fs::canonicalize(deployment).map_err(|e| e.to_string())?;
    if !root.is_absolute() {
        return Err("Module data root must be absolute".into());
    }
    private_dir(&root)?;
    private_dir(&root.join("run"))?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(root.join("run/core.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|_| "This module Core is already running")?;
    write_private(
        &root.join("run/core.pid"),
        std::process::id().to_string().as_bytes(),
    )?;
    status(&root, "starting", "Preparing independent module Core")?;
    let result = run_core(&module, &root, model, language, client_uid);
    match &result {
        Ok(()) => status(&root, "stopped", "Module Core stopped")?,
        Err(error) => status(&root, "failed", error)?,
    }
    let _ = fs::remove_file(root.join("run/client.json"));
    let _ = fs::remove_file(Path::new(CLIENT_DATA_DIRECTORY).join("files/host-runtime/client.json"));
    let _ = fs::remove_file(root.join("run/core.pid"));
    result
}

/// Builds one Core using the shared Android host factory and existing frontend adapters.
fn run_core(
    module: &Path,
    root: &Path,
    model: String,
    language: String,
    client_uid: u32,
) -> Result<(), String> {
    let user = root.join("users/0");
    private_dir(&user)?;
    let base_runtime = user.join("runtime");
    let base_workspace = user.join("workspaces");
    let bootstrap_file = user.join("bootstrap.json");
    let bootstrap = if bootstrap_file.exists() {
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(&bootstrap_file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?
    } else {
        let now = operit_host_api::TimeUtils::currentTimeMillis();
        let value = serde_json::json!({"confirmed":true, "runtimeRoot":base_runtime, "workspaceRoot":base_workspace,
            "identities":[{"id":"identity-module", "name":"Module", "createdAt":now}],
            "activeIdentityId":"identity-module", "updatedAt":now});
        write_private(
            &bootstrap_file,
            &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
        )?;
        value
    };
    let identity = bootstrap["activeIdentityId"]
        .as_str()
        .ok_or("Missing module identity")?;
    if !identity.starts_with("identity-")
        || !identity
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("Invalid module identity".into());
    }
    if bootstrap["runtimeRoot"] != serde_json::json!(base_runtime)
        || bootstrap["workspaceRoot"] != serde_json::json!(base_workspace)
    {
        return Err("Module bootstrap storage roots are inconsistent".into());
    }
    let runtime = base_runtime.join("identities").join(identity);
    let workspace = base_workspace.join("identities").join(identity);
    private_dir(&runtime)?;
    private_dir(&workspace)?;
    prepare_environment(module, &user, &runtime, &workspace)?;
    let bridge = Arc::new(crate::PlatformRuntimeFactory::startBridge(BridgeStartup {
        runtimeRoot: runtime,
        workspaceRoot: workspace,
        metadata: StartupMetadata::AndroidPrivileged {
            model,
            language,
            secretRoot: user.join("secrets"),
        },
    })?);
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let mut random = [0; 32];
    OsRng.fill_bytes(&mut random);
    let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let endpoint = ModuleEndpoint {
        protocol: 1,
        port: listener.local_addr().map_err(|e| e.to_string())?.port(),
        token: token.clone(),
        bootstrap: bootstrap.clone(),
    };
    let endpoint_file = root.join("run/client.json");
    write_private(
        &endpoint_file,
        &serde_json::to_vec(&endpoint).map_err(|e| e.to_string())?,
    )?;
    publish_client(
        &serde_json::to_vec(&endpoint).map_err(|e| e.to_string())?,
        client_uid,
    )?;
    let expected = bootstrap.clone();
    let control_root = root.to_path_buf();
    let control = Arc::new(move |content: &[u8]| -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_slice(content).map_err(|e| e.to_string())?;
        for key in [
            "runtimeRoot",
            "workspaceRoot",
            "activeIdentityId",
            "confirmed",
        ] {
            if value[key] != expected[key] {
                return Err(format!("Running module cannot change {key}"));
            }
        }
        let ids = |v: &serde_json::Value| -> Result<Vec<String>, String> {
            v["identities"]
                .as_array()
                .ok_or("Invalid module identities".into())
                .and_then(|items| {
                    items
                        .iter()
                        .map(|i| {
                            i["id"]
                                .as_str()
                                .map(str::to_owned)
                                .ok_or("Invalid module identity".into())
                        })
                        .collect()
                })
        };
        if ids(&value)? != ids(&expected)? {
            return Err("Running module cannot replace identities".into());
        }
        write_private(&bootstrap_file, content)?;
        let endpoint_file = control_root.join("run/client.json");
        let mut endpoint: ModuleEndpoint =
            serde_json::from_slice(&fs::read(&endpoint_file).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        endpoint.bootstrap = value;
        write_private(
            &endpoint_file,
            &serde_json::to_vec(&endpoint).map_err(|e| e.to_string())?,
        )?;
        publish_client(
            &serde_json::to_vec(&endpoint).map_err(|e| e.to_string())?,
            client_uid,
        )?;
        Ok("ok".into())
    });
    unsafe {
        libc::signal(libc::SIGTERM, terminate as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, terminate as *const () as libc::sighandler_t);
    }
    status(
        root,
        "ready",
        "Independent module Core is listening on loopback",
    )?;
    let result =
        crate::LocalRuntimeTransport::serve(listener, token, bridge.clone(), control, &STOPPING);
    if let Some(application) = bridge
        .coreApplication
        .lock()
        .map_err(|_| "Core shutdown lock is poisoned")?
        .take()
    {
        application.shutdownNow();
    }
    result
}
