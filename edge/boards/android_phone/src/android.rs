//! JNI is a local management bridge, never a second network protocol.
use crate::{Command, PhoneNode};
use jni::{
    objects::{JClass, JString},
    sys::jstring,
    JNIEnv,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{mpsc, Mutex, OnceLock},
    thread,
};

struct Request {
    command: Command,
    reply: mpsc::SyncSender<Result<Value, String>>,
}
struct Worker {
    sender: tokio::sync::mpsc::Sender<Request>,
    thread: thread::JoinHandle<()>,
}
static WORKER: OnceLock<Mutex<Option<Worker>>> = OnceLock::new();

/// Copies a bounded Java string before entering the native node worker.
fn text(env: &mut JNIEnv, value: &JString, limit: usize) -> Result<String, String> {
    let text: String = env.get_string(value).map_err(|e| e.to_string())?.into();
    if text.len() > limit {
        return Err("Edge input exceeds its byte budget".into());
    }
    Ok(text)
}

/// Encodes native failures explicitly for the UI without throwing away node state.
fn response(env: &mut JNIEnv, result: Result<Value, String>) -> jstring {
    let result = match result {
        Ok(value) => json!({"ok": true, "data": value}),
        Err(error) => json!({"ok": false, "error": error}),
    };
    match env.new_string(result.to_string()) {
        Ok(value) => value.into_raw(),
        Err(error) => {
            let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Starts one affine Rust executor and returns its actual listener status.
#[no_mangle]
pub extern "system" fn Java_com_operit_edgephone_EdgeNative_start(
    mut env: JNIEnv,
    _: JClass,
    root: JString,
    model: JString,
) -> jstring {
    let result = (|| {
        let root = PathBuf::from(text(&mut env, &root, 4096)?);
        let model = text(&mut env, &model, 128)?;
        let mut owner = WORKER
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|e| e.to_string())?;
        if owner.is_some() {
            return Err("Edge node is already running".into());
        }
        let (sender, mut requests) = tokio::sync::mpsc::channel::<Request>(4);
        let (ready, started) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("operit-edge-phone".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready.send(Err(error.to_string()));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let node = match PhoneNode::start(root, model, "0.0.0.0:0".into()).await {
                        Ok(node) => node,
                        Err(error) => {
                            let _ = ready.send(Err(error));
                            return;
                        }
                    };
                    let status = node.status();
                    let failed = status.is_err();
                    let _ = ready.send(status);
                    if !failed {
                        while let Some(request) = requests.recv().await {
                            let stop = matches!(request.command, Command::Stop);
                            let result = node.execute(request.command).await;
                            let _ = request.reply.send(result);
                            if stop {
                                return;
                            }
                        }
                    }
                    let _ = node.execute(Command::Stop).await;
                });
            })
            .map_err(|e| e.to_string())?;
        match started.recv().map_err(|e| e.to_string())? {
            Ok(status) => {
                *owner = Some(Worker { sender, thread });
                Ok(status)
            }
            Err(error) => {
                thread
                    .join()
                    .map_err(|_| "Edge worker failed during startup".to_string())?;
                Err(error)
            }
        }
    })();
    response(&mut env, result)
}

/// Serializes local commands on the sole node owner and joins it on explicit stop.
#[no_mangle]
pub extern "system" fn Java_com_operit_edgephone_EdgeNative_execute(
    mut env: JNIEnv,
    _: JClass,
    input: JString,
) -> jstring {
    let result = (|| {
        let command: Command =
            serde_json::from_str(&text(&mut env, &input, 4096)?).map_err(|e| e.to_string())?;
        let stop = matches!(command, Command::Stop);
        let mut owner = WORKER
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|e| e.to_string())?;
        let worker = owner.as_ref().ok_or("Edge node is not running")?;
        let (reply, result) = mpsc::sync_channel(1);
        worker
            .sender
            .blocking_send(Request { command, reply })
            .map_err(|e| e.to_string())?;
        let result = result.recv().map_err(|e| e.to_string())?;
        if stop {
            owner
                .take()
                .ok_or("Edge worker is missing")?
                .thread
                .join()
                .map_err(|_| "Edge worker failed while stopping".to_string())?;
        }
        result
    })();
    response(&mut env, result)
}
