use operit_host_api::HostManager::HostManager;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;

/// Execution budget for ToolPkg hooks dispatched through the shared notification queue.
///
/// Every ToolPkg notification is awaited sequentially, so one stalled hook delays unrelated
/// work: a stalled message hook blocks the next send, a stalled chat view hook blocks the
/// chat screen, and a stalled interval hook blocks the provider call queued behind it.
/// Bounding the budget keeps one misbehaving container from stalling the application for the
/// interactive default of one minute while still leaving room for real hook work.
///
/// These are background notifications, so they are not governed by the user facing
/// `ToolPkgPreHookTimeout` preference that bounds the hooks running before a send.
pub const TOOLPKG_NOTIFICATION_HOOK_TIMEOUT_MILLIS: u64 = 10_000;

/// Holds the ToolPkg execution dependencies owned by one runtime instance.
#[derive(Clone)]
pub struct ToolPkgBridgeRuntime {
    tool_handler: AIToolHandler,
    host_manager: HostManager,
}

impl ToolPkgBridgeRuntime {
    /// Creates bridge runtime state for one application runtime.
    pub fn new(tool_handler: AIToolHandler, host_manager: HostManager) -> Self {
        Self {
            tool_handler,
            host_manager,
        }
    }

    /// Returns a snapshot of this runtime's package manager.
    pub fn package_manager(&self) -> RuntimePackageManager {
        self.tool_handler
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .clone()
    }

    /// Returns this runtime's tool handler.
    pub fn tool_handler(&self) -> AIToolHandler {
        self.tool_handler.clone()
    }

    /// Returns the host capabilities attached to this ToolPkg runtime.
    pub fn host_manager(&self) -> HostManager {
        self.host_manager.clone()
    }
}

/// Enqueues notifications in order on one Host-owned asynchronous consumer.
pub fn scheduleToolPkgNotification(
    task_name: &'static str,
    task: impl FnOnce() -> operit_plugin_sdk::javascript::JsExecutionFuture<()> + Send + 'static,
) {
    type Notification =
        Box<dyn FnOnce() -> operit_plugin_sdk::javascript::JsExecutionFuture<()> + Send>;
    static QUEUE: std::sync::OnceLock<
        Result<tokio::sync::mpsc::UnboundedSender<Notification>, String>,
    > = std::sync::OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Notification>();
        operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "operit-toolpkg-notifications",
                Box::new(move || {
                    Box::pin(async move {
                        while let Some(notification) = receiver.recv().await {
                            notification().await;
                        }
                    })
                }),
            )
            .map_err(|error| error.to_string())?;
        Ok(sender)
    });
    let result = match queue {
        Ok(sender) => sender
            .send(Box::new(task))
            .map_err(|error| error.to_string()),
        Err(error) => Err(error.clone()),
    };
    if let Err(error) = result {
        operit_util::AppLogger::AppLogger::e(
            "ToolPkgHookBridge",
            &format!("enqueue {task_name} failed: {error}"),
        );
    }
}
