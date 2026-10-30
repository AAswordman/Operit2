use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;

use crate::PreferencesEncryption::PreferencesEncryption;
use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, runtimeStoragePath};
use crate::SyncOperationStore::{
    NewSyncOperation, SyncOperation, SyncOperationSemantics, SyncOperationStore,
    SyncOperationStoreError,
};

#[derive(Debug, Error)]
/// Error type for preference files, preference flows, and sync application.
pub enum PreferencesDataStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("host error: {0}")]
    Host(#[from] operit_host_api::HostError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("encryption error: {0}")]
    Encryption(String),
    #[error("sync operation store error: {0}")]
    Sync(#[from] SyncOperationStoreError),
    #[error("invalid preferences schema version: {value}")]
    InvalidSchemaVersion { value: String },
    #[error("preferences schema version {actual} is newer than runtime version {expected}")]
    SchemaVersionTooNew { actual: u32, expected: u32 },
    #[error("missing preferences migration from version {from} to {to}")]
    MissingMigration { from: u32, to: u32 },
    #[error("{0}")]
    Message(String),
}

/// One serializer specialization for measuring bounded JSON and emitting it.
/// A constrained write reserves the measured size once, avoiding Vec growth's
/// simultaneous old/new buffers; an unexpected size change returns an error.
struct PreferencesJsonBuffer {
    bytes: Vec<u8>,
    measured: usize,
    countOnly: bool,
    bounded: bool,
}
impl std::io::Write for PreferencesJsonBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.countOnly {
            self.measured = self.measured.checked_add(bytes.len())
                .ok_or(std::io::ErrorKind::OutOfMemory)?;
        } else {
            if self.bounded && bytes.len() > self.bytes.capacity() - self.bytes.len() {
                return Err(std::io::ErrorKind::OutOfMemory.into());
            }
            self.bytes.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}

const PREFERENCES_REPLICATION_KEY: &str = "__operit_preferences_replication";

pub const PREFERENCES_SCHEMA_VERSION_KEY_NAME: &str = "__operit_preferences_schema_version";

/// Result alias used by Flow-like preference APIs.
pub type FlowResult<T> = Result<T, PreferencesDataStoreError>;

/// Minimal Kotlin Flow-like contract for one-shot reads and collection.
pub trait FlowLike<T>: Clone
where
    T: Clone,
{
    /// Reads the first emitted value.
    fn first(&self) -> FlowResult<T>;

    /// Collects values emitted by this flow.
    fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T);
}

/// Flow contract that also exposes the current value synchronously.
pub trait StateFlowLike<T>: FlowLike<T>
where
    T: Clone,
{
    /// Returns the currently held value.
    fn value(&self) -> T;
}

/// Mutable state flow contract for value updates and compare-and-set writes.
pub trait MutableStateFlowLike<T>: StateFlowLike<T>
where
    T: Clone + PartialEq,
{
    /// Replaces the current value.
    fn set_value(&self, value: T);

    /// Replaces the current value only when it equals `expect`.
    fn compare_and_set(&self, expect: T, update: T) -> bool;

    /// Applies an atomic transformation to the current value.
    fn update<F>(&self, update: F)
    where
        F: FnMut(&mut T);
}

/// Observed value source with Kotlin Flow-like collection semantics.
///
/// `Flow` is used for values backed by storage, such as preferences. Calling
/// `collect` emits the current value and then keeps waiting for upstream changes
/// until the source completes. UI/watch subscriptions should use
/// `subscribeWithCancellation`, which registers with the shared observation
/// source without creating one blocking collector thread per subscription.
#[derive(Clone)]
pub struct Flow<T> {
    producer: Arc<dyn Fn() -> FlowResult<T> + Send + Sync>,
    waitChanged: Option<Arc<dyn Fn(&FlowCancellation) -> bool + Send + Sync>>,
    observation: Option<Arc<FlowObservation>>,
}

/// Cancellation token for a single `Flow::collectWithCancellation` invocation.
///
/// The token represents the collector lifetime, not the lifetime of the upstream
/// data source. `cancel` marks the collector as cancelled and runs registered
/// hooks, typically to wake a thread parked in a condition variable wait.
#[derive(Clone)]
pub struct FlowCancellation {
    inner: Arc<FlowCancellationInner>,
}

struct FlowCancellationInner {
    cancelled: AtomicBool,
    hooks: Mutex<FlowCancellationHooks>,
}

struct FlowCancellationHooks {
    nextId: usize,
    callbacks: HashMap<usize, Arc<dyn Fn() + Send + Sync>>,
}

/// Guard that unregisters a flow cancellation hook when dropped.
pub struct FlowCancellationHook {
    cancellation: Weak<FlowCancellationInner>,
    id: Option<usize>,
}

#[derive(Clone)]
struct FlowObservation {
    subscribe:
        Arc<dyn Fn(Arc<dyn Fn() + Send + Sync>) -> FlowObservationSubscription + Send + Sync>,
}

trait FlowObservationGuard: Send {}

impl<T> FlowObservationGuard for T where T: Send {}

/// Guard that keeps a shared flow observation subscription active.
pub struct FlowObservationSubscription {
    _guard: Box<dyn FlowObservationGuard>,
}

/// Handle returned by a live Flow subscription.
pub struct FlowSubscription {
    cancellation: FlowCancellation,
    _observation: Option<FlowObservationSubscription>,
}

impl FlowSubscription {
    /// Cancels this Flow subscription and unregisters it from the shared source.
    pub fn cancel(self) {
        self.cancellation.cancel();
    }
}

impl Drop for FlowSubscription {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

impl FlowCancellation {
    /// Creates an uncancelled collector lifetime token.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(FlowCancellationInner {
                cancelled: AtomicBool::new(false),
                hooks: Mutex::new(FlowCancellationHooks {
                    nextId: 0,
                    callbacks: HashMap::new(),
                }),
            }),
        }
    }

    /// Cancels this collector lifetime and invokes all active cancellation hooks.
    ///
    /// Calling this more than once has no additional effect.
    pub fn cancel(&self) {
        if self.inner.cancelled.swap(true, Ordering::SeqCst) {
            return;
        }
        let callbacks = self
            .inner
            .hooks
            .lock()
            .expect("FlowCancellation hooks mutex must not be poisoned")
            .callbacks
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for callback in callbacks {
            callback();
        }
    }

    #[allow(non_snake_case)]
    /// Returns whether this collector lifetime has been cancelled.
    pub fn isCancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    #[allow(non_snake_case)]
    /// Registers a hook invoked when `cancel` is called.
    ///
    /// The returned guard unregisters the hook when dropped. If the token is
    /// already cancelled, the callback is invoked immediately and the guard does
    /// not register anything.
    pub fn addCancelHook(
        &self,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> FlowCancellationHook {
        let callback = Arc::new(callback);
        let mut hooks = self
            .inner
            .hooks
            .lock()
            .expect("FlowCancellation hooks mutex must not be poisoned");
        if self.isCancelled() {
            drop(hooks);
            callback();
            return FlowCancellationHook {
                cancellation: Arc::downgrade(&self.inner),
                id: None,
            };
        }
        let id = hooks.nextId;
        hooks.nextId += 1;
        hooks.callbacks.insert(id, callback);
        FlowCancellationHook {
            cancellation: Arc::downgrade(&self.inner),
            id: Some(id),
        }
    }
}

impl Drop for FlowCancellationHook {
    fn drop(&mut self) {
        let Some(id) = self.id.take() else {
            return;
        };
        let Some(inner) = self.cancellation.upgrade() else {
            return;
        };
        {
            let hooks = inner.hooks.lock();
            if let Ok(mut hooks) = hooks {
                hooks.callbacks.remove(&id);
            }
        }
    }
}

impl<T> Flow<T> {
    /// Creates a one-shot flow that emits the value returned by `producer`.
    pub fn new<F>(producer: F) -> Self
    where
        F: Fn() -> FlowResult<T> + Send + Sync + 'static,
    {
        Self {
            producer: Arc::new(producer),
            waitChanged: None,
            observation: None,
        }
    }

    #[allow(non_snake_case)]
    /// Creates an observed flow.
    ///
    /// `producer` reads the current value. `waitChanged` blocks until the next
    /// upstream change or until the supplied `FlowCancellation` is cancelled.
    /// It must return `true` when a new value should be read from `producer`, and
    /// `false` when collection should finish without reading again.
    pub fn newObserved<F, W>(producer: F, waitChanged: W) -> Self
    where
        F: Fn() -> FlowResult<T> + Send + Sync + 'static,
        W: Fn(&FlowCancellation) -> bool + Send + Sync + 'static,
    {
        Self {
            producer: Arc::new(producer),
            waitChanged: Some(Arc::new(waitChanged)),
            observation: None,
        }
    }

    #[allow(non_snake_case)]
    fn newObservedWithObservation<F, W>(
        producer: F,
        waitChanged: W,
        observation: FlowObservation,
    ) -> Self
    where
        F: Fn() -> FlowResult<T> + Send + Sync + 'static,
        W: Fn(&FlowCancellation) -> bool + Send + Sync + 'static,
    {
        Self {
            producer: Arc::new(producer),
            waitChanged: Some(Arc::new(waitChanged)),
            observation: Some(Arc::new(observation)),
        }
    }

    /// Reads the current value once.
    pub fn first(&self) -> FlowResult<T> {
        (self.producer)()
    }

    /// Collects this flow without an external cancellation token.
    ///
    /// For observed flows this may wait indefinitely for future changes. Runtime
    /// watch code should prefer `collectWithCancellation`.
    pub fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        self.collectWithCancellation(FlowCancellation::new(), collector)
    }

    #[allow(non_snake_case)]
    /// Collects this flow until the upstream completes or `cancellation` is cancelled.
    ///
    /// The current value is emitted first. For observed flows, each later
    /// emission is produced after `waitChanged` reports a real upstream change.
    pub fn collectWithCancellation<F>(
        &self,
        cancellation: FlowCancellation,
        collector: F,
    ) -> FlowResult<()>
    where
        F: Fn(T),
    {
        if cancellation.isCancelled() {
            return Ok(());
        }
        collector(self.first()?);
        if let Some(waitChanged) = &self.waitChanged {
            while !cancellation.isCancelled() {
                if !waitChanged(&cancellation) {
                    break;
                }
                if cancellation.isCancelled() {
                    break;
                }
                collector(self.first()?);
            }
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Subscribes to this flow through its shared observation source.
    ///
    /// The subscriber receives the current value first. When this flow is backed
    /// by a shared observed source, later source changes invoke `subscriber`
    /// without creating a blocking collector thread for this subscription.
    pub fn subscribeWithCancellation<F>(
        &self,
        cancellation: FlowCancellation,
        subscriber: F,
    ) -> FlowResult<FlowSubscription>
    where
        T: Send + 'static,
        F: Fn(T) + Send + Sync + 'static,
    {
        if cancellation.isCancelled() {
            return Ok(FlowSubscription {
                cancellation,
                _observation: None,
            });
        }

        subscriber(self.first()?);

        if cancellation.isCancelled() {
            return Ok(FlowSubscription {
                cancellation,
                _observation: None,
            });
        }

        let observation = self.observation.as_ref().map(|observation| {
            let producer = Arc::clone(&self.producer);
            let subscriber = Arc::new(subscriber);
            let cancellationForCallback = cancellation.clone();
            (observation.subscribe)(Arc::new(move || {
                if cancellationForCallback.isCancelled() {
                    return;
                }
                if let Ok(value) = producer() {
                    if !cancellationForCallback.isCancelled() {
                        subscriber(value);
                    }
                }
            }))
        });

        Ok(FlowSubscription {
            cancellation,
            _observation: observation,
        })
    }

    /// Reads the current value and returns it only when `predicate` matches.
    pub fn firstWhere<P>(&self, predicate: P) -> FlowResult<Option<T>>
    where
        P: Fn(&T) -> bool,
    {
        let value = self.first()?;
        if predicate(&value) {
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Maps each collected value into another value.
    pub fn map<U, F>(&self, transform: F) -> Flow<U>
    where
        T: 'static,
        U: 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let producer = Arc::clone(&self.producer);
        Flow {
            producer: Arc::new(move || producer().map(&transform)),
            waitChanged: self.waitChanged.clone(),
            observation: self.observation.clone(),
        }
    }

    #[allow(non_snake_case)]
    /// Maps each collected value with a transform that may fail.
    pub fn mapResult<U, F>(&self, transform: F) -> Flow<U>
    where
        T: 'static,
        U: 'static,
        F: Fn(T) -> FlowResult<U> + Send + Sync + 'static,
    {
        let producer = Arc::clone(&self.producer);
        Flow {
            producer: Arc::new(move || transform(producer()?)),
            waitChanged: self.waitChanged.clone(),
            observation: self.observation.clone(),
        }
    }

    /// Replaces producer errors with a handler result.
    pub fn catch<F>(&self, handler: F) -> Flow<T>
    where
        T: 'static,
        F: Fn(PreferencesDataStoreError) -> FlowResult<T> + Send + Sync + 'static,
    {
        let producer = Arc::clone(&self.producer);
        Flow {
            producer: Arc::new(move || match producer() {
                Ok(value) => Ok(value),
                Err(error) => handler(error),
            }),
            waitChanged: self.waitChanged.clone(),
            observation: self.observation.clone(),
        }
    }

    /// Converts this flow into a `StateFlow`.
    ///
    /// This mirrors Kotlin `stateIn` at the simplified runtime level: the returned
    /// state starts from `initialValue` and is immediately updated from the
    /// upstream flow. Observed flows subscribe through the shared observation
    /// source, so `stateIn` shares the same parked dispatcher used by watch
    /// subscriptions instead of creating its own blocking collector thread.
    pub fn stateIn(
        &self,
        _scope: CoroutineScope,
        _started: SharingStarted,
        initialValue: T,
    ) -> StateFlow<T>
    where
        T: Clone + PartialEq + Send + 'static,
    {
        let stateFlow = StateFlow::new(initialValue);
        let cancellation = FlowCancellation::new();
        // The sharing subscription belongs to the resulting state, not vice
        // versa. Its callback must not retain its owner and form a cycle.
        let target = Arc::downgrade(&stateFlow.inner);
        if let Ok(subscription) = self.subscribeWithCancellation(cancellation, move |value| {
            if let Some(inner) = target.upgrade() {
                StateFlow { inner }.set_value(value);
            }
        }) {
            stateFlow.setUpstreamSubscription(subscription);
        }
        stateFlow
    }
}

impl<T> FlowLike<T> for Flow<T>
where
    T: Clone,
{
    fn first(&self) -> FlowResult<T> {
        Flow::first(self)
    }

    fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        Flow::collect(self, collector)
    }
}

#[derive(Clone, Debug)]
/// Placeholder scope type used to mirror Kotlin `stateIn` call sites.
pub struct CoroutineScope;

#[derive(Clone, Debug)]
/// Supported sharing policy for Flow-to-StateFlow conversion.
pub enum SharingStarted {
    Lazily,
}

#[derive(Clone)]
/// Mutable observable value with Kotlin StateFlow-like semantics.
pub struct StateFlow<T> {
    inner: Arc<StateFlowInner<T>>,
}

struct StateFlowInner<T> {
    value: Mutex<T>,
    version: Mutex<u64>,
    changed: Condvar,
    subscribers: Mutex<StateFlowSubscribers<T>>,
    upstreamSubscription: Mutex<Option<FlowSubscription>>,
    upstreamStateSubscriptions: Mutex<Vec<StateFlowUpstreamSubscription>>,
}

struct StateFlowSubscribers<T> {
    nextId: usize,
    callbacks: HashMap<usize, Arc<Mutex<dyn FnMut(T) + Send>>>,
}

struct StateFlowUpstreamSubscription {
    cancel: Option<Box<dyn FnOnce() + Send>>,
}

impl StateFlowUpstreamSubscription {
    fn new(cancel: impl FnOnce() + Send + 'static) -> Self {
        Self {
            cancel: Some(Box::new(cancel)),
        }
    }
}

impl Drop for StateFlowUpstreamSubscription {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel();
        }
    }
}

impl<T> StateFlow<T>
where
    T: Clone + PartialEq,
{
    /// Creates a state flow with an initial value.
    pub fn new(initialValue: T) -> Self {
        Self {
            inner: Arc::new(StateFlowInner {
                value: Mutex::new(initialValue),
                version: Mutex::new(0),
                changed: Condvar::new(),
                subscribers: Mutex::new(StateFlowSubscribers {
                    nextId: 0,
                    callbacks: HashMap::new(),
                }),
                upstreamSubscription: Mutex::new(None),
                upstreamStateSubscriptions: Mutex::new(Vec::new()),
            }),
        }
    }

    #[allow(non_snake_case)]
    fn setUpstreamSubscription(&self, subscription: FlowSubscription) {
        *self
            .inner
            .upstreamSubscription
            .lock()
            .expect("StateFlow upstream subscription mutex must not be poisoned") =
            Some(subscription);
    }

    #[allow(non_snake_case)]
    fn addUpstreamStateSubscription(&self, subscription: StateFlowUpstreamSubscription) {
        self.inner
            .upstreamStateSubscriptions
            .lock()
            .expect("StateFlow upstream state subscription mutex must not be poisoned")
            .push(subscription);
    }

    /// Binds an upstream collector's cancellation to this derived state's owner.
    /// Uses the same subscription ownership as `Flow::stateIn`; the returned
    /// callback updates the state without keeping an abandoned sharing job alive.
    #[allow(non_snake_case)]
    pub fn upstreamCollector(
        &self,
        cancellation: FlowCancellation,
    ) -> impl Fn(T) -> bool + Send + Sync + 'static
    where
        T: Send + 'static,
    {
        self.setUpstreamSubscription(FlowSubscription {
            cancellation,
            _observation: None,
        });
        let target = Arc::downgrade(&self.inner);
        move |value| {
            let Some(inner) = target.upgrade() else {
                return false;
            };
            StateFlow { inner }.set_value(value);
            true
        }
    }

    /// Returns the current state value.
    pub fn value(&self) -> T {
        self.inner
            .value
            .lock()
            .expect("StateFlow value mutex must not be poisoned")
            .clone()
    }

    /// Reads the current state value.
    pub fn first(&self) -> FlowResult<T> {
        Ok(self.value())
    }

    /// Reads the current value when it satisfies `predicate`.
    pub fn firstWhere<P>(&self, predicate: P) -> FlowResult<Option<T>>
    where
        P: Fn(&T) -> bool,
    {
        let value = self.first()?;
        if predicate(&value) {
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Emits the current value and then emits each subsequent change.
    pub fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        let mut observedVersion = *self
            .inner
            .version
            .lock()
            .expect("StateFlow version mutex must not be poisoned");
        collector(self.value());
        loop {
            let versionGuard = self
                .inner
                .version
                .lock()
                .expect("StateFlow version mutex must not be poisoned");
            let versionGuard = self
                .inner
                .changed
                .wait_while(versionGuard, |version| *version == observedVersion)
                .expect("StateFlow version mutex must not be poisoned");
            observedVersion = *versionGuard;
            drop(versionGuard);
            collector(self.value());
        }
    }

    #[allow(non_snake_case)]
    /// Collects values until `shouldStop` returns true for an emitted value.
    pub fn collectUntil<F, P>(&self, mut collector: F, shouldStop: P) -> FlowResult<()>
    where
        F: FnMut(T),
        P: Fn(&T) -> bool,
    {
        let mut observedVersion = *self
            .inner
            .version
            .lock()
            .expect("StateFlow version mutex must not be poisoned");
        let current = self.value();
        collector(current.clone());
        if shouldStop(&current) {
            return Ok(());
        }
        loop {
            let versionGuard = self
                .inner
                .version
                .lock()
                .expect("StateFlow version mutex must not be poisoned");
            let versionGuard = self
                .inner
                .changed
                .wait_while(versionGuard, |version| *version == observedVersion)
                .expect("StateFlow version mutex must not be poisoned");
            observedVersion = *versionGuard;
            drop(versionGuard);
            let current = self.value();
            collector(current.clone());
            if shouldStop(&current) {
                return Ok(());
            }
        }
    }

    /// Updates the current value and notifies collectors when it changed.
    pub fn set_value(&self, value: T) {
        let mut guard = self
            .inner
            .value
            .lock()
            .expect("StateFlow value mutex must not be poisoned");
        if *guard == value {
            return;
        }
        *guard = value.clone();
        drop(guard);
        let mut version = self
            .inner
            .version
            .lock()
            .expect("StateFlow version mutex must not be poisoned");
        *version += 1;
        self.inner.changed.notify_all();
        drop(version);
        let subscribers = self
            .inner
            .subscribers
            .lock()
            .expect("StateFlow subscribers mutex must not be poisoned")
            .callbacks
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for subscriber in subscribers {
            if let Ok(mut subscriber) = subscriber.lock() {
                subscriber(value.clone());
            }
        }
    }

    /// Updates the current value only when the current value equals `expect`.
    pub fn compare_and_set(&self, expect: T, update: T) -> bool {
        let mut guard = self
            .inner
            .value
            .lock()
            .expect("StateFlow value mutex must not be poisoned");
        if *guard == expect {
            *guard = update.clone();
            drop(guard);
            let mut version = self
                .inner
                .version
                .lock()
                .expect("StateFlow version mutex must not be poisoned");
            *version += 1;
            self.inner.changed.notify_all();
            drop(version);
            let subscribers = self
                .inner
                .subscribers
                .lock()
                .expect("StateFlow subscribers mutex must not be poisoned")
                .callbacks
                .values()
                .cloned()
                .collect::<Vec<_>>();
            for subscriber in subscribers {
                if let Ok(mut subscriber) = subscriber.lock() {
                    subscriber(update.clone());
                }
            }
            true
        } else {
            false
        }
    }

    /// Subscribes to value changes and immediately emits the current value.
    pub fn subscribe<F>(&self, subscriber: F) -> usize
    where
        F: FnMut(T) + Send + 'static,
    {
        let callback = Arc::new(Mutex::new(subscriber));
        if let Ok(mut subscriber) = callback.lock() {
            subscriber(self.value());
        }
        let mut subscribers = self
            .inner
            .subscribers
            .lock()
            .expect("StateFlow subscribers mutex must not be poisoned");
        let id = subscribers.nextId;
        subscribers.nextId += 1;
        subscribers.callbacks.insert(id, callback);
        id
    }

    /// Removes a previously registered state-flow subscriber.
    pub fn unsubscribe(&self, subscriptionId: usize) {
        let mut subscribers = self
            .inner
            .subscribers
            .lock()
            .expect("StateFlow subscribers mutex must not be poisoned");
        subscribers.callbacks.remove(&subscriptionId);
    }

    /// Derives a new state flow by transforming each source value.
    pub fn map<U, F>(&self, transform: F) -> StateFlow<U>
    where
        T: Send + 'static,
        U: Clone + PartialEq + Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let transform = Arc::new(transform);
        let initialValue = self.value();
        let latest = Arc::new(Mutex::new(initialValue.clone()));
        let stateFlow = StateFlow::new(transform(initialValue));
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        let subscriptionId = self.subscribe(move |value| {
            let Some(inner) = target.upgrade() else {
                return;
            };
            {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow map latest mutex must not be poisoned");
                if *latest == value {
                    return;
                }
                *latest = value.clone();
            }
            StateFlow { inner }.set_value(transformForSubscriber(value));
        });
        let source = self.clone();
        stateFlow.addUpstreamStateSubscription(StateFlowUpstreamSubscription::new(move || {
            source.unsubscribe(subscriptionId);
        }));
        stateFlow
    }
}

fn setCombinedStateFlowValue<T>(target: &Weak<StateFlowInner<T>>, value: T)
where
    T: Clone + PartialEq,
{
    let Some(inner) = target.upgrade() else {
        return;
    };
    StateFlow { inner }.set_value(value);
}

fn subscribeCombinedSource<S, R, F>(stateFlow: &StateFlow<R>, source: &StateFlow<S>, subscriber: F)
where
    S: Clone + PartialEq + Send + 'static,
    R: Clone + PartialEq,
    F: FnMut(S) + Send + 'static,
{
    let subscriptionId = source.subscribe(subscriber);
    let sourceForSubscription = source.clone();
    stateFlow.addUpstreamStateSubscription(StateFlowUpstreamSubscription::new(move || {
        sourceForSubscription.unsubscribe(subscriptionId);
    }));
}

/// Combines two state flows into one derived state flow.
pub fn combine2<A, B, R, F>(
    source1: &StateFlow<A>,
    source2: &StateFlow<B>,
    transform: F,
) -> StateFlow<R>
where
    A: Clone + PartialEq + Send + 'static,
    B: Clone + PartialEq + Send + 'static,
    R: Clone + PartialEq + Send + 'static,
    F: Fn(A, B) -> R + Send + Sync + 'static,
{
    let transform = Arc::new(transform);
    let latest = Arc::new(Mutex::new((source1.value(), source2.value())));
    let initialValue = {
        let latest = latest
            .lock()
            .expect("StateFlow combine latest mutex must not be poisoned");
        transform(latest.0.clone(), latest.1.clone())
    };
    let stateFlow = StateFlow::new(initialValue);

    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source1, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.0 == value {
                    return;
                }
                latest.0 = value;
                transformForSubscriber(latest.0.clone(), latest.1.clone())
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source2, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.1 == value {
                    return;
                }
                latest.1 = value;
                transformForSubscriber(latest.0.clone(), latest.1.clone())
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }

    stateFlow
}

/// Combines three state flows into one derived state flow.
pub fn combine3<A, B, C, R, F>(
    source1: &StateFlow<A>,
    source2: &StateFlow<B>,
    source3: &StateFlow<C>,
    transform: F,
) -> StateFlow<R>
where
    A: Clone + PartialEq + Send + 'static,
    B: Clone + PartialEq + Send + 'static,
    C: Clone + PartialEq + Send + 'static,
    R: Clone + PartialEq + Send + 'static,
    F: Fn(A, B, C) -> R + Send + Sync + 'static,
{
    let transform = Arc::new(transform);
    let latest = Arc::new(Mutex::new((
        source1.value(),
        source2.value(),
        source3.value(),
    )));
    let initialValue = {
        let latest = latest
            .lock()
            .expect("StateFlow combine latest mutex must not be poisoned");
        transform(latest.0.clone(), latest.1.clone(), latest.2.clone())
    };
    let stateFlow = StateFlow::new(initialValue);

    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source1, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.0 == value {
                    return;
                }
                latest.0 = value;
                transformForSubscriber(latest.0.clone(), latest.1.clone(), latest.2.clone())
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source2, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.1 == value {
                    return;
                }
                latest.1 = value;
                transformForSubscriber(latest.0.clone(), latest.1.clone(), latest.2.clone())
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source3, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.2 == value {
                    return;
                }
                latest.2 = value;
                transformForSubscriber(latest.0.clone(), latest.1.clone(), latest.2.clone())
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }

    stateFlow
}

/// Combines four state flows into one derived state flow.
pub fn combine4<A, B, C, D, R, F>(
    source1: &StateFlow<A>,
    source2: &StateFlow<B>,
    source3: &StateFlow<C>,
    source4: &StateFlow<D>,
    transform: F,
) -> StateFlow<R>
where
    A: Clone + PartialEq + Send + 'static,
    B: Clone + PartialEq + Send + 'static,
    C: Clone + PartialEq + Send + 'static,
    D: Clone + PartialEq + Send + 'static,
    R: Clone + PartialEq + Send + 'static,
    F: Fn(A, B, C, D) -> R + Send + Sync + 'static,
{
    let transform = Arc::new(transform);
    let latest = Arc::new(Mutex::new((
        source1.value(),
        source2.value(),
        source3.value(),
        source4.value(),
    )));
    let initialValue = {
        let latest = latest
            .lock()
            .expect("StateFlow combine latest mutex must not be poisoned");
        transform(
            latest.0.clone(),
            latest.1.clone(),
            latest.2.clone(),
            latest.3.clone(),
        )
    };
    let stateFlow = StateFlow::new(initialValue);

    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source1, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.0 == value {
                    return;
                }
                latest.0 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source2, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.1 == value {
                    return;
                }
                latest.1 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source3, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.2 == value {
                    return;
                }
                latest.2 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source4, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.3 == value {
                    return;
                }
                latest.3 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }

    stateFlow
}

/// Combines five state flows into one derived state flow.
pub fn combine5<A, B, C, D, E, R, F>(
    source1: &StateFlow<A>,
    source2: &StateFlow<B>,
    source3: &StateFlow<C>,
    source4: &StateFlow<D>,
    source5: &StateFlow<E>,
    transform: F,
) -> StateFlow<R>
where
    A: Clone + PartialEq + Send + 'static,
    B: Clone + PartialEq + Send + 'static,
    C: Clone + PartialEq + Send + 'static,
    D: Clone + PartialEq + Send + 'static,
    E: Clone + PartialEq + Send + 'static,
    R: Clone + PartialEq + Send + 'static,
    F: Fn(A, B, C, D, E) -> R + Send + Sync + 'static,
{
    let transform = Arc::new(transform);
    let latest = Arc::new(Mutex::new((
        source1.value(),
        source2.value(),
        source3.value(),
        source4.value(),
        source5.value(),
    )));
    let initialValue = {
        let latest = latest
            .lock()
            .expect("StateFlow combine latest mutex must not be poisoned");
        transform(
            latest.0.clone(),
            latest.1.clone(),
            latest.2.clone(),
            latest.3.clone(),
            latest.4.clone(),
        )
    };
    let stateFlow = StateFlow::new(initialValue);

    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source1, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.0 == value {
                    return;
                }
                latest.0 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                    latest.4.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source2, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.1 == value {
                    return;
                }
                latest.1 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                    latest.4.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source3, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.2 == value {
                    return;
                }
                latest.2 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                    latest.4.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source4, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.3 == value {
                    return;
                }
                latest.3 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                    latest.4.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }
    {
        let target = Arc::downgrade(&stateFlow.inner);
        let latestForSubscriber = Arc::clone(&latest);
        let transformForSubscriber = Arc::clone(&transform);
        subscribeCombinedSource(&stateFlow, source5, move |value| {
            if target.upgrade().is_none() {
                return;
            }
            let combinedValue = {
                let mut latest = latestForSubscriber
                    .lock()
                    .expect("StateFlow combine latest mutex must not be poisoned");
                if latest.4 == value {
                    return;
                }
                latest.4 = value;
                transformForSubscriber(
                    latest.0.clone(),
                    latest.1.clone(),
                    latest.2.clone(),
                    latest.3.clone(),
                    latest.4.clone(),
                )
            };
            setCombinedStateFlowValue(&target, combinedValue);
        });
    }

    stateFlow
}

impl<T> FlowLike<T> for StateFlow<T>
where
    T: Clone + PartialEq,
{
    fn first(&self) -> FlowResult<T> {
        StateFlow::first(self)
    }

    fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        StateFlow::collect(self, collector)
    }
}

impl<T> StateFlowLike<T> for StateFlow<T>
where
    T: Clone + PartialEq,
{
    fn value(&self) -> T {
        StateFlow::value(self)
    }
}

#[derive(Clone)]
/// Mutable wrapper around `StateFlow` used by preferences and runtime state.
pub struct MutableStateFlow<T> {
    state: StateFlow<T>,
}

impl<T> MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    /// Creates a mutable state flow with an initial value.
    pub fn new(initialValue: T) -> Self {
        Self {
            state: StateFlow::new(initialValue),
        }
    }

    #[allow(non_snake_case)]
    /// Returns the read-oriented state-flow view.
    pub fn asStateFlow(&self) -> StateFlow<T> {
        self.state.clone()
    }

    /// Returns the current value.
    pub fn value(&self) -> T {
        self.state.value()
    }

    /// Reads the current value.
    pub fn first(&self) -> FlowResult<T> {
        Ok(self.value())
    }

    /// Emits the current value and then emits each subsequent change.
    pub fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        self.state.collect(collector)
    }

    #[allow(non_snake_case)]
    /// Collects values until `shouldStop` accepts an emitted value.
    pub fn collectUntil<F, P>(&self, collector: F, shouldStop: P) -> FlowResult<()>
    where
        F: FnMut(T),
        P: Fn(&T) -> bool,
    {
        self.state.collectUntil(collector, shouldStop)
    }

    /// Updates the current value and notifies subscribers when it changed.
    pub fn set_value(&self, value: T) {
        self.state.set_value(value);
    }

    /// Subscribes to value changes and immediately emits the current value.
    pub fn subscribe<F>(&self, subscriber: F) -> usize
    where
        F: FnMut(T) + Send + 'static,
    {
        self.state.subscribe(subscriber)
    }

    /// Removes a previously registered subscriber.
    pub fn unsubscribe(&self, subscriptionId: usize) {
        self.state.unsubscribe(subscriptionId);
    }

    /// Updates the current value only when the current value equals `expect`.
    pub fn compare_and_set(&self, expect: T, update: T) -> bool {
        self.state.compare_and_set(expect, update)
    }

    /// Applies an atomic transformation to the current value.
    pub fn update<F>(&self, update: F)
    where
        F: FnMut(&mut T),
    {
        let mut update = update;
        loop {
            let current = self.value();
            let mut next = current.clone();
            update(&mut next);
            if current == next {
                return;
            }
            if self.compare_and_set(current, next) {
                return;
            }
        }
    }
}

impl<T> FlowLike<T> for MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    fn first(&self) -> FlowResult<T> {
        MutableStateFlow::first(self)
    }

    fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        MutableStateFlow::collect(self, collector)
    }
}

impl<T> StateFlowLike<T> for MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    fn value(&self) -> T {
        MutableStateFlow::value(self)
    }
}

impl<T> MutableStateFlowLike<T> for MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    fn set_value(&self, value: T) {
        MutableStateFlow::set_value(self, value);
    }

    fn compare_and_set(&self, expect: T, update: T) -> bool {
        MutableStateFlow::compare_and_set(self, expect, update)
    }

    fn update<F>(&self, update: F)
    where
        F: FnMut(&mut T),
    {
        MutableStateFlow::update(self, update);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Strongly typed key for entries in a `Preferences` value.
pub struct PreferencesKey {
    pub name: String,
}

#[allow(non_snake_case)]
/// Creates a string preferences key.
pub fn stringPreferencesKey(name: &str) -> PreferencesKey {
    PreferencesKey {
        name: name.to_string(),
    }
}

#[derive(Clone, Debug, Default)]
/// In-memory representation of a preferences file.
pub struct Preferences {
    pub(crate) values: Arc<HashMap<String, Value>>,
    pub(crate) pendingMutations: BTreeMap<String, PreferencesTransactionMutation>,
    pub(crate) commit: Option<crate::Transaction::kernel::FileCommit>,
    pub(crate) legacy: Option<Value>,
    pub(crate) view: Option<crate::Transaction::Data>,
}

impl PartialEq for Preferences {
    /// Compares persisted values independently of transient edit intentions.
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl Eq for Preferences {}

impl Preferences {
    /// Identifies the exact immutable snapshot, including intervening writes that restore old values.
    pub(crate) fn sharesSnapshot(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.values, &other.values)
    }

    /// Reads a preference value by key.
    pub fn get(&self, key: &PreferencesKey) -> Option<&Value> {
        self.values.get(&key.name)
    }

    /// Decodes one optional entry using its declared JSON type and rejects mismatches.
    pub fn getTyped<T: serde::de::DeserializeOwned>(
        &self,
        key: &PreferencesKey,
    ) -> Result<Option<T>, PreferencesDataStoreError> {
        self.get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(Into::into)
    }

    /// Copy retained entries only: a replaced/deleted value never enters the
    /// new snapshot. Other readers keep their immutable snapshot for rollback.
    fn writableValuesExcluding(&mut self, key: &str) -> &mut HashMap<String, Value> {
        if Arc::strong_count(&self.values) > 1 {
            self.values = Arc::new(self.values.iter()
                .filter(|(name, _)| name.as_str() != key)
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect());
        }
        // A weak mutation witness must not force a deep clone. make_mut moves
        // the unique map and disconnects weak observers without cloning values.
        Arc::make_mut(&mut self.values)
    }

    /// Updates a preference value and clones the shared map only when needed.
    pub fn set(&mut self, key: &PreferencesKey, value: impl Into<Value>) {
        let value = value.into();
        if self.values.get(&key.name) == Some(&value) {
            return;
        }
        self.pendingMutations.insert(
            key.name.clone(),
            PreferencesTransactionMutation {
                key: key.name.clone(),
                operation: "set".into(),
                // The final value already belongs to this snapshot. Resolve it
                // when exporting the mutation instead of keeping a second tree.
                value: Value::Null,
            },
        );
        self.writableValuesExcluding(&key.name).insert(key.name.clone(), value);
    }

    /// Removes a preference value and clones the shared map only when needed.
    pub fn remove(&mut self, key: &PreferencesKey) {
        if !self.values.contains_key(&key.name) {
            return;
        }
        self.pendingMutations.insert(
            key.name.clone(),
            PreferencesTransactionMutation {
                key: key.name.clone(),
                operation: "delete".into(),
                value: Value::Null,
            },
        );
        self.writableValuesExcluding(&key.name).remove(&key.name);
    }

    /// Adds one integer delta and records the operation rather than an absolute replacement.
    pub fn incrementInteger(
        &mut self,
        key: &PreferencesKey,
        delta: i64,
    ) -> Result<(), PreferencesDataStoreError> {
        let current = match self.get(key) {
            None => 0,
            Some(value) => value.as_i64().ok_or_else(|| {
                PreferencesDataStoreError::Message(
                    "Integer increment requires an integer entry".into(),
                )
            })?,
        };
        let value = current.checked_add(delta).ok_or_else(|| {
            PreferencesDataStoreError::Message("Preference integer overflow".into())
        })?;
        let mutation = match self.pendingMutations.get(&key.name) {
            None => PreferencesTransactionMutation {
                key: key.name.clone(),
                operation: "increment".into(),
                value: delta.into(),
            },
            Some(mutation) if mutation.operation == "increment" => {
                let previous = mutation.value.as_i64().ok_or_else(|| {
                    PreferencesDataStoreError::Message("Invalid pending increment".into())
                })?;
                let combined = previous.checked_add(delta).ok_or_else(|| {
                    PreferencesDataStoreError::Message("Preference integer delta overflow".into())
                })?;
                PreferencesTransactionMutation {
                    key: key.name.clone(),
                    operation: "increment".into(),
                    value: combined.into(),
                }
            }
            Some(_) => PreferencesTransactionMutation {
                key: key.name.clone(),
                operation: "set".into(),
                value: value.into(),
            },
        };
        Arc::make_mut(&mut self.values).insert(key.name.clone(), value.into());
        self.pendingMutations.insert(key.name.clone(), mutation);
        Ok(())
    }

    /// Assigns one object field without replacing concurrent edits to other fields.
    pub fn setObjectField(
        &mut self,
        key: &PreferencesKey,
        field: &str,
        value: Value,
    ) -> Result<(), PreferencesDataStoreError> {
        let mut object = match self.get(key) {
            None => serde_json::Map::new(),
            Some(value) => value.as_object().cloned().ok_or_else(|| {
                PreferencesDataStoreError::Message(
                    "Object field update requires an object entry".into(),
                )
            })?,
        };
        if object.get(field) == Some(&value) {
            return Ok(());
        }
        object.insert(field.to_owned(), value.clone());
        let pending = self.pendingMutations.get(&key.name);
        let mutation = match pending {
            _ if pending.is_none()
                || pending.is_some_and(|mutation| mutation.operation == "patch_object") =>
            {
                let mut patch = match pending {
                    None => PreferencesObjectPatch {
                        set: BTreeMap::new(),
                        delete: Vec::new(),
                    },
                    Some(mutation) => {
                        serde_json::from_value::<PreferencesObjectPatch>(mutation.value.clone())?
                    }
                };
                patch.delete.retain(|key| key != field);
                patch.set.insert(field.to_owned(), value);
                PreferencesTransactionMutation {
                    key: key.name.clone(),
                    operation: "patch_object".into(),
                    value: serde_json::to_value(patch)?,
                }
            }
            _ => PreferencesTransactionMutation {
                key: key.name.clone(),
                operation: "set".into(),
                value: Value::Object(object.clone()),
            },
        };
        Arc::make_mut(&mut self.values).insert(key.name.clone(), Value::Object(object));
        self.pendingMutations.insert(key.name.clone(), mutation);
        Ok(())
    }

    /// Checks whether a preference key exists.
    pub fn contains(&self, key: &PreferencesKey) -> bool {
        self.values.contains_key(&key.name)
    }

    /// Borrows entries without copying every serialized record for read-only scans.
    pub fn iterEntries(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.values.iter().map(|(key, value)| (key.as_str(), value))
    }

    /// Returns all preference entries as owned key-value pairs.
    pub fn entries(&self) -> Vec<(String, Value)> {
        self.values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

impl Serialize for Preferences {
    /// Serializes preferences as a flat map of native JSON values.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.values.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Preferences {
    /// Deserializes a flat JSON value map without guessing string contents.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut values = HashMap::<String, Value>::deserialize(deserializer)?;
        let legacy = values.remove(PREFERENCES_REPLICATION_KEY);
        let commit = values.remove("__operit_transaction_commit").map(serde_json::from_value)
            .transpose().map_err(serde::de::Error::custom)?;
        Ok(Self {
            values: Arc::new(values),
            pendingMutations: BTreeMap::new(),
            commit,
            legacy,
            view: None,
        })
    }
}

#[allow(non_snake_case)]
/// Creates an empty preferences value.
pub fn emptyPreferences() -> Preferences {
    Preferences::default()
}

#[allow(non_snake_case)]
/// Creates a mutable state flow.
pub fn mutableStateFlow<T>(initialValue: T) -> MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    MutableStateFlow::new(initialValue)
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// Carries independent object-field assignments and explicit deletions.
struct PreferencesObjectPatch {
    set: BTreeMap<String, Value>,
    delete: Vec<String>,
}

#[derive(Clone)]
/// File-backed preferences store with Flow-like observation and optional encryption.
pub struct PreferencesDataStore {
    pub(crate) path: PathBuf,
    pub(crate) storagePath: String,
    pub(crate) storageHost: Arc<dyn RuntimeStorageHost>,
    pub(crate) encryption: Option<PreferencesEncryption>,
    pub(crate) syncOperationStore: Option<SyncOperationStore>,
    pub(crate) syncDescriptor: Option<PreferencesSyncDescriptor>,
    changeSignal: Arc<PreferencesDataStoreChangeSignal>,
    sharedState: Arc<PreferencesDataStoreSharedState>,
    pub(crate) schema: Option<PreferencesSchema>,
}

pub(crate) fn preferenceOwner(path: &str, format: u32) -> crate::Transaction::Owner {
    crate::Transaction::Owner { backend: crate::Transaction::Backend::DataStore, path: path.into(), format, layout: 1 }
}

#[derive(Clone)]
pub(crate) struct PreferencesSchema {
    pub(crate) currentVersion: u32,
    pub(crate) migrate: Arc<crate::Transaction::migration::Definition>,
}

#[derive(Clone)]
/// Stores persistent state that belongs only to one CoreNode.
pub struct CoreNodeStateStore {
    inner: PreferencesDataStore,
}

impl CoreNodeStateStore {
    /// Returns the process-local publication revision shared by all handles to this record.
    pub(crate) fn revision(&self) -> u64 {
        *self.inner.changeSignal.version.lock()
            .expect("PreferencesDataStore version mutex must not be poisoned")
    }

    /// Reads node-local records directly on constrained hosts. Validate the
    /// schema first; migrations and retained/live caches use the regular path.
    pub fn typedEntries<T: serde::de::DeserializeOwned>(&self) -> Result<Vec<(String, T)>, PreferencesDataStoreError> {
        self.typedEntriesMatching(None)
    }

    /// Decode only the requested node-local record; unrelated payloads are
    /// skipped without constructing JSON trees or typed records.
    pub fn typedEntry<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, PreferencesDataStoreError> {
        self.typedEntriesMatching(Some(key)).map(|mut entries| entries.pop().map(|(_, record)| record))
    }

    /// Checks encoded schema metadata without decoding unrelated application records.
    fn encodedNodeSchemaIsCurrent(&self, bytes: &[u8]) -> Result<bool, PreferencesDataStoreError> {
        #[derive(Deserialize)]
        struct Header {
            #[serde(default, rename = "__operit_preferences_schema_version")]
            version: Option<Value>,
        }
        let header: Header = serde_json::from_slice(bytes)?;
        self.nodeSchemaIsCurrent(header.version)
    }

    fn nodeSchemaIsCurrent(&self, version: Option<Value>) -> Result<bool, PreferencesDataStoreError> {
        let mut probe = emptyPreferences();
        if let Some(version) = version {
            probe.set(&PreferencesDataStore::schemaVersionKey(), version);
        }
        self.inner.schemaIsCurrent(&probe)
    }

    /// Reads selected records without initializing an absent node-local collection.
    fn typedEntriesMatching<T: serde::de::DeserializeOwned>(&self, only: Option<&str>) -> Result<Vec<(String, T)>, PreferencesDataStoreError> {
        let transaction = self.inner.sharedState.transaction.lock()
            .expect("PreferencesDataStore transaction mutex must not be poisoned");
        if !self.inner.storageHost.exists(&self.inner.storagePath)? { return Ok(Vec::new()); }
        if !self.inner.storageHost.retainPreferencesCache() {
            self.inner.releaseNodeLocalCacheUnlocked();
            let bytes = self.inner.storageHost.readBytes(&self.inner.storagePath)?;
            if !bytes.iter().all(u8::is_ascii_whitespace) {
                if self.encodedNodeSchemaIsCurrent(&bytes)? {
                    return deserializeTypedNodeEntries(&bytes, only).map_err(Into::into);
                }
            }
        }
        drop(transaction);
        self.data()?.iterEntries()
            .filter(|(key, _)| *key != PREFERENCES_SCHEMA_VERSION_KEY_NAME && only.is_none_or(|selected| *key == selected))
            .map(|(key, value)| T::deserialize(value).map(|value| (key.to_owned(), value)).map_err(Into::into))
            .collect()
    }

    /// Inspect a consistent encoded snapshot without copying its payloads.
    /// The transaction ends before the callback; borrowed values remain tied to
    /// these immutable bytes, and cannot escape through the returned result.
    pub fn withEncodedEntries<R>(
        &self, read: impl FnOnce(BTreeMap<String, &serde_json::value::RawValue>) -> R,
    ) -> Result<R, PreferencesDataStoreError> {
        let current = if !self.inner.storageHost.retainPreferencesCache() {
            let _transaction = self.inner.sharedState.transaction.lock()
                .expect("PreferencesDataStore transaction mutex must not be poisoned");
            self.inner.releaseNodeLocalCacheUnlocked();
            if self.inner.storageHost.exists(&self.inner.storagePath)? {
                let bytes = self.inner.storageHost.readBytes(&self.inner.storagePath)?;
                if bytes.iter().all(u8::is_ascii_whitespace) { None } else {
                    self.encodedNodeSchemaIsCurrent(&bytes)?.then_some(bytes)
                }
            } else { None }
        } else { None };
        let bytes = match current {
            Some(bytes) => bytes,
            None => {
                let preferences = self.data()?;
                serde_json::to_vec(&preferences.iterEntries().collect::<BTreeMap<_, _>>())?
            }
        };
        let mut entries: BTreeMap<String, &serde_json::value::RawValue> = serde_json::from_slice(&bytes)?;
        entries.remove(PREFERENCES_SCHEMA_VERSION_KEY_NAME);
        entries.remove(PREFERENCES_REPLICATION_KEY);
        Ok(read(entries))
    }

    /// Visit one raw entry at a time through bounded Host range reads. The
    /// schema is checked in a first pass before invoking the read-only callback;
    /// both passes share the same transaction. Legacy/cache paths keep migration.
    pub fn visitEncodedEntries(&self, mut visit: impl FnMut(&str, &serde_json::value::RawValue) -> Result<(), String>) -> Result<(), PreferencesDataStoreError> {
        if !self.inner.storageHost.retainPreferencesCache() {
            let transaction = self.inner.sharedState.transaction.lock()
                .expect("PreferencesDataStore transaction mutex must not be poisoned");
            self.inner.releaseNodeLocalCacheUnlocked();
            if self.inner.storageHost.exists(&self.inner.storagePath)? {
                #[derive(Deserialize)]
                struct Header {
                    #[serde(default, rename = "__operit_preferences_schema_version")]
                    version: Option<Value>,
                    #[serde(default, rename = "__operit_preferences_replication", deserialize_with = "nodeEntryFieldPresent")]
                    replication: bool,
                }
                let header = (|| -> Result<Header, serde_json::Error> {
                    let mut decoder = serde_json::Deserializer::from_reader(NodePreferenceReader::new(
                        self.inner.storageHost.as_ref(), &self.inner.storagePath));
                    let header = Header::deserialize(&mut decoder)?;
                    decoder.end()?;
                    Ok(header)
                })();
                match header {
                    Ok(header) => {
                        if self.nodeSchemaIsCurrent(header.version)? && !header.replication {
                            let mut decoder = serde_json::Deserializer::from_reader(NodePreferenceReader::new(
                                self.inner.storageHost.as_ref(), &self.inner.storagePath));
                            serde::de::Deserializer::deserialize_map(&mut decoder, EncodedNodeEntryVisitor(&mut visit))?;
                            decoder.end()?;
                            return Ok(());
                        }
                    }
                    Err(error) if error.is_eof() => {} // Empty legacy files use the regular path.
                    Err(error) => return Err(error.into()),
                }
            }
            drop(transaction);
        }
        self.withEncodedEntries(|entries| {
            for (key, value) in entries { visit(&key, value)?; }
            Ok::<_, String>(())
        })?.map_err(PreferencesDataStoreError::Message)
    }

    /// Replace one record without expanding unrelated JSON on constrained hosts.
    /// The shared transaction and atomic host publication remain the commit boundary.
    pub fn setTypedEntry<T: Serialize>(&self, key: &str, value: &T) -> Result<(), PreferencesDataStoreError> {
        self.editTypedEntry(key, Some(value), false)
    }

    /// Atomically merge immutable records while retaining unrelated payloads as raw JSON.
    /// Every existing value and duplicate in the batch must agree before publication.
    pub fn mergeImmutableEntries<T: Serialize + serde::de::DeserializeOwned + PartialEq>(
        &self, incoming: &[(&str, &T)],
    ) -> Result<(), PreferencesDataStoreError> {
        let conflict = || PreferencesDataStoreError::Message("Conflicting immutable entry".into());
        for (key, _) in incoming {
            if matches!(*key, PREFERENCES_SCHEMA_VERSION_KEY_NAME | PREFERENCES_REPLICATION_KEY) {
                return Err(PreferencesDataStoreError::Message("Reserved immutable entry key".into()));
            }
        }
        if !self.inner.storageHost.retainPreferencesCache() {
            let transaction = self.inner.sharedState.transaction.lock()
                .expect("PreferencesDataStore transaction mutex must not be poisoned");
            self.inner.releaseNodeLocalCacheUnlocked();
            let bytes = if self.inner.storageHost.exists(&self.inner.storagePath)? {
                self.inner.storageHost.readBytes(&self.inner.storagePath)?
            } else { Vec::new() };
            let entries: BTreeMap<String, &serde_json::value::RawValue> = if bytes.iter().all(u8::is_ascii_whitespace) {
                BTreeMap::new()
            } else { serde_json::from_slice(&bytes)? };
            let version = entries.get(PREFERENCES_SCHEMA_VERSION_KEY_NAME)
                .map(|version| serde_json::from_str(version.get())).transpose()?;
            if self.nodeSchemaIsCurrent(version)? && !entries.contains_key(PREFERENCES_REPLICATION_KEY) {
                let mut added = BTreeMap::new();
                for &(key, value) in incoming {
                    if let Some(old) = entries.get(key) {
                        let old: T = serde_json::from_str(old.get())?;
                        if &old != value { return Err(conflict()); }
                    } else if let Some(old) = added.insert(key, value) {
                        if old != value { return Err(conflict()); }
                    }
                }
                if added.is_empty() { return Ok(()); }
                let merged = NodeEntryBatch { entries: &entries, added: &added };
                let mut content = PreferencesJsonBuffer { bytes: Vec::new(), measured: 0, countOnly: true, bounded: true };
                let encode = |output: &mut PreferencesJsonBuffer| merged.serialize(&mut serde_json::Serializer::with_formatter(
                    output, serde_json::ser::CompactFormatter));
                encode(&mut content)?;
                content.bytes.try_reserve_exact(content.measured)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
                content.countOnly = false;
                encode(&mut content)?;
                drop(entries);
                drop(bytes);
                self.inner.storageHost.writeBytesAtomically(&self.inner.storagePath, &content.bytes)?;
                drop(transaction);
                self.inner.notifyChanged();
                return Ok(());
            }
            drop(transaction);
        }
        self.try_edit_result(|preferences| {
            for &(key, value) in incoming {
                let key = stringPreferencesKey(key);
                if let Some(old) = preferences.get(&key) {
                    if T::deserialize(old)? != *value { return Err(conflict()); }
                } else { preferences.set(&key, serde_json::to_value(value)?); }
            }
            Ok(())
        })
    }

    /// Remove one record through the same transaction without decoding its neighbours.
    pub fn removeEntry(&self, key: &str) -> Result<(), PreferencesDataStoreError> {
        self.editTypedEntry::<Value>(key, None, false)
    }

    /// Compact legacy whitespace in mutable application records in the same
    /// atomic replacement. Opt-in only: immutable/raw-addressed entries retain
    /// their original bytes through setTypedEntry and mergeImmutableEntries.
    pub fn setTypedEntryCompactingJson<T: Serialize>(&self, key: &str, value: &T) -> Result<(), PreferencesDataStoreError> {
        self.editTypedEntry(key, Some(value), true)
    }

    fn editTypedEntry<T: Serialize>(&self, key: &str, value: Option<&T>, compactEntries: bool) -> Result<(), PreferencesDataStoreError> {
        if !self.inner.storageHost.retainPreferencesCache()
            && !matches!(key, PREFERENCES_SCHEMA_VERSION_KEY_NAME | PREFERENCES_REPLICATION_KEY) {
            let transaction = self.inner.sharedState.transaction.lock()
                .expect("PreferencesDataStore transaction mutex must not be poisoned");
            self.inner.releaseNodeLocalCacheUnlocked();
            let bytes = if self.inner.storageHost.exists(&self.inner.storagePath)? {
                self.inner.storageHost.readBytes(&self.inner.storagePath)?
            } else { Vec::new() };
            let entries: BTreeMap<String, &serde_json::value::RawValue> = if bytes.iter().all(u8::is_ascii_whitespace) {
                BTreeMap::new()
            } else { serde_json::from_slice(&bytes)? };
            let version = entries.get(PREFERENCES_SCHEMA_VERSION_KEY_NAME)
                .map(|version| serde_json::from_str(version.get())).transpose()?;
            if self.nodeSchemaIsCurrent(version)? && !entries.contains_key(PREFERENCES_REPLICATION_KEY) {
                let replacement = NodeEntryReplacement { entries: &entries, key, value };
                let mut content = PreferencesJsonBuffer { bytes: Vec::new(), measured: 0, countOnly: true, bounded: true };
                let encode = |output: &mut PreferencesJsonBuffer| {
                    if compactEntries {
                        replacement.serialize(&mut serde_json::Serializer::with_formatter(
                            CompactJsonWriter { output, inString: false, escaped: false }, serde_json::ser::CompactFormatter))
                    } else {
                        replacement.serialize(&mut serde_json::Serializer::with_formatter(output, serde_json::ser::CompactFormatter))
                    }
                };
                encode(&mut content)?;
                content.bytes.try_reserve_exact(content.measured)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
                content.countOnly = false;
                encode(&mut content)?;
                if content.bytes == bytes { return Ok(()); }
                drop(entries);
                drop(bytes);
                self.inner.storageHost.writeBytesAtomically(&self.inner.storagePath, &content.bytes)?;
                drop(transaction);
                self.inner.notifyChanged();
                return Ok(());
            }
            drop(transaction);
        }
        let encoded = value.map(serde_json::to_value).transpose()?;
        self.edit(|preferences| match encoded {
            Some(value) => preferences.set(&stringPreferencesKey(key), value),
            None => { preferences.remove(&stringPreferencesKey(key)); }
        })
    }

    /// Deletes a node-local record and its shared preferences snapshot together.
    /// A raw host deletion would leave an identical later import cached and
    /// silently skip recreating the durable file. Failed deletion preserves cache.
    pub fn delete(&self) -> Result<(), PreferencesDataStoreError> {
        let transaction = self.inner.lockTransaction();
        if self.inner.storageHost.exists(&self.inner.storagePath)? {
            self.inner
                .storageHost
                .delete(&self.inner.storagePath, false)?;
        }
        let mut loaded = self
            .inner
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.loaded = true;
        loaded.preferences = emptyPreferences();
        drop(loaded);
        drop(transaction);
        self.inner.notifyChanged();
        Ok(())
    }

    /// Declares a one-time schema migration for node-local state.
    #[allow(non_snake_case)]
    pub fn withSchema<F>(mut self, currentVersion: u32, migrate: F) -> Self
    where F: Fn(u32, &mut Preferences) -> Result<(), PreferencesDataStoreError> + Send + Sync + 'static {
        self.inner.schema = Some(PreferencesSchema { currentVersion,
            migrate: Arc::new(crate::Transaction::migration::Definition::from_preferences(migrate)) });
        self
    }

    /// Opens node-local state through an explicit runtime storage host.
    #[allow(non_snake_case)]
    pub fn newWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        Self {
            inner: PreferencesDataStore::newNodeLocalWithStorage(storageHost, storagePath, false),
        }
    }
}

fn nodeEntryFieldPresent<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<bool, D::Error> {
    serde::de::IgnoredAny::deserialize(decoder)?;
    Ok(true)
}

/// Keep decompression bounded to the Host's range window and a 512-byte
/// buffer; a page export never needs the complete expanded policy allocation.
struct NodePreferenceReader<'a> {
    storage: &'a dyn RuntimeStorageHost,
    pub(crate) path: &'a str,
    buffer: Vec<u8>,
    next: usize,
    offset: u64,
}
impl<'a> NodePreferenceReader<'a> {
    fn new(storage: &'a dyn RuntimeStorageHost, path: &'a str) -> Self {
        Self { storage, path, buffer: Vec::new(), next: 0, offset: 0 }
    }
}
impl std::io::Read for NodePreferenceReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if output.is_empty() { return Ok(0); }
        if self.next == self.buffer.len() {
            self.buffer = Vec::new(); // Release the old block before Host decompression.
            self.buffer = self.storage.readBytesRange(self.path, self.offset, 512)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            self.offset += self.buffer.len() as u64;
            self.next = 0;
        }
        let count = output.len().min(self.buffer.len() - self.next);
        output[..count].copy_from_slice(&self.buffer[self.next..self.next + count]);
        self.next += count;
        Ok(count)
    }
}
struct EncodedNodeEntryVisitor<'a, F>(&'a mut F);
impl<'de, F: FnMut(&str, &serde_json::value::RawValue) -> Result<(), String>> serde::de::Visitor<'de> for EncodedNodeEntryVisitor<'_, F> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result { formatter.write_str("node-local entry map") }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            let value: Box<serde_json::value::RawValue> = map.next_value()?;
            if !matches!(key.as_str(), PREFERENCES_SCHEMA_VERSION_KEY_NAME | PREFERENCES_REPLICATION_KEY) {
                (self.0)(&key, &value).map_err(serde::de::Error::custom)?;
            }
        }
        Ok(())
    }
}

/// RawValue bypasses serde's formatter. Filter only JSON whitespace outside
/// strings, preserving every token, escape, number spelling and key order.
/// State spans write calls; input has already passed serde's JSON validation.
struct CompactJsonWriter<W> {
    output: W,
    inString: bool,
    escaped: bool,
}
impl<W: std::io::Write> std::io::Write for CompactJsonWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        for &byte in bytes {
            if self.inString {
                if self.escaped { self.escaped = false; }
                else if byte == b'\\' { self.escaped = true; }
                else if byte == b'"' { self.inString = false; }
            } else if byte == b'"' {
                self.inString = true;
            } else if matches!(byte, b' ' | b'\n' | b'\r' | b'\t') {
                continue;
            }
            self.output.write_all(&[byte])?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { self.output.flush() }
}

/// Borrow old JSON payloads and serialize only the replaced typed value.
struct NodeEntryReplacement<'a, T> {
    entries: &'a BTreeMap<String, &'a serde_json::value::RawValue>,
    key: &'a str,
    value: Option<&'a T>,
}
impl<T: Serialize> Serialize for NodeEntryReplacement<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let count = self.entries.len() - usize::from(self.entries.contains_key(self.key)) + usize::from(self.value.is_some());
        let mut map = serializer.serialize_map(Some(count))?;
        let mut inserted = false;
        for (key, value) in self.entries {
            if !inserted && key.as_str() >= self.key {
                if let Some(value) = self.value { map.serialize_entry(self.key, value)?; }
                inserted = true;
            }
            if key != self.key { map.serialize_entry(key, value)?; }
        }
        if !inserted { if let Some(value) = self.value { map.serialize_entry(self.key, value)?; } }
        map.end()
    }
}

/// Merge two sorted maps without cloning any unchanged business value.
struct NodeEntryBatch<'a, T> {
    entries: &'a BTreeMap<String, &'a serde_json::value::RawValue>,
    added: &'a BTreeMap<&'a str, &'a T>,
}
impl<T: Serialize> Serialize for NodeEntryBatch<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries.len() + self.added.len()))?;
        let mut added = self.added.iter().peekable();
        for (key, value) in self.entries {
            while added.peek().is_some_and(|(next, _)| **next < key.as_str()) {
                let (key, value) = added.next().unwrap();
                map.serialize_entry(key, value)?;
            }
            map.serialize_entry(key, value)?;
        }
        for (key, value) in added { map.serialize_entry(key, value)?; }
        map.end()
    }
}

/// Stream typed records without constructing an intermediate JSON Value tree.
fn deserializeTypedNodeEntries<T: serde::de::DeserializeOwned>(bytes: &[u8], only: Option<&str>) -> Result<Vec<(String, T)>, serde_json::Error> {
    struct Entries<'a, T>(Option<&'a str>, std::marker::PhantomData<T>);
    impl<'de, T: serde::de::DeserializeOwned> serde::de::Visitor<'de> for Entries<'_, T> {
        type Value = Vec<(String, T)>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a node-local preferences map")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut entries = Vec::new();
            while let Some(key) = map.next_key::<String>()? {
                if matches!(key.as_str(), PREFERENCES_SCHEMA_VERSION_KEY_NAME | PREFERENCES_REPLICATION_KEY)
                    || self.0.is_some_and(|selected| key != selected) {
                    map.next_value::<serde::de::IgnoredAny>()?;
                } else {
                    entries.push((key, map.next_value::<T>()?));
                }
            }
            Ok(entries)
        }
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let entries = serde::de::Deserializer::deserialize_map(&mut deserializer, Entries(only, std::marker::PhantomData))?;
    deserializer.end()?;
    Ok(entries)
}

impl std::ops::Deref for CoreNodeStateStore {
    type Target = PreferencesDataStore;

    /// Exposes the shared preferences API without exposing replication controls.
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone)]
/// Stores encrypted credentials and secrets owned by one CoreNode.
pub struct CoreNodeSecretStore {
    inner: PreferencesDataStore,
}

impl CoreNodeSecretStore {
    /// Declares the owning secret module's versioned migration.
    pub fn withSchema<F>(mut self, currentVersion: u32, migrate: F) -> Self
    where
        F: Fn(u32, &mut Preferences) -> Result<(), PreferencesDataStoreError>
            + Send
            + Sync
            + 'static,
    {
        self.inner = self.inner.withSchema(currentVersion, migrate);
        self
    }

    /// Opens encrypted node-local secrets through the default runtime storage host.
    pub fn new(path: PathBuf) -> Self {
        let storageHost = defaultRuntimeStorageHost();
        let storagePath = runtimeStoragePath(&path);
        Self {
            inner: PreferencesDataStore::newNodeLocalWithResolvedPath(
                storageHost,
                path,
                storagePath,
                true,
            ),
        }
    }

    /// Opens encrypted node-local secrets through an explicit runtime storage host.
    #[allow(non_snake_case)]
    pub fn newWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        Self {
            inner: PreferencesDataStore::newNodeLocalWithStorage(storageHost, storagePath, true),
        }
    }
}

impl std::ops::Deref for CoreNodeSecretStore {
    type Target = PreferencesDataStore;

    /// Exposes the shared preferences API without exposing replication controls.
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Debug)]
/// Sync metadata describing how a preferences file participates in runtime sync.
pub struct PreferencesSyncDescriptor {
    pub domain: String,
    pub entityType: String,
    pub storagePath: String,
}

impl PreferencesSyncDescriptor {
    /// Creates an explicit preferences sync descriptor.
    pub fn new(
        domain: impl Into<String>,
        entityType: impl Into<String>,
        storagePath: impl Into<String>,
    ) -> Self {
        Self {
            domain: domain.into(),
            entityType: entityType.into(),
            storagePath: storagePath.into(),
        }
    }

    #[allow(non_snake_case)]
    /// Builds the standard sync descriptor for a host-relative storage path.
    pub fn forStoragePath(storagePath: &str) -> Self {
        let path = Path::new(storagePath);
        let fileName = path
            .file_name()
            .expect("preferences storage path must include a file name")
            .to_string_lossy()
            .to_string();
        let entityType = fileName
            .trim_end_matches(".preferences.json")
            .trim_end_matches(".json")
            .to_string();
        Self::new("preferences", entityType, storagePath.to_string())
    }
}

/// Returns the explicit historical payload version for operations written before native JSON.
fn legacyPreferencePayloadVersion() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Carries one independently mergeable preference entry mutation.
pub(crate) struct PreferencesSyncEntryPayload {
    pub(crate) storagePath: String,
    key: String,
    value: Value,
    encrypted: bool,
    #[serde(default = "legacyPreferencePayloadVersion")]
    formatVersion: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schemaVersion: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    migrationSnapshot: bool,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    importCoverage: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    jsonPath: Vec<PreferencesSyncJsonPathSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    jsonMutation: Option<PreferencesSyncJsonMutation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// Carries one complete entry mutation inside an atomic preference transaction.
pub(crate) struct PreferencesTransactionMutation {
    key: String,
    operation: String,
    value: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
/// Carries all related entry changes committed by one local preference edit.
pub(crate) struct PreferencesSyncTransactionPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contractVersion: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    logicalTime: Option<i64>,
    pub(crate) storagePath: String,
    encrypted: bool,
    formatVersion: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schemaVersion: Option<u32>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    migrationSnapshot: bool,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    importCoverage: BTreeSet<String>,
    entries: Vec<PreferencesTransactionMutation>,
}

#[derive(Clone, Debug, PartialEq)]
/// Stores one validated incoming preferences mutation ready for batched application.
pub struct PreferencesSyncedEntry {
    pub(crate) storagePath: String,
    encrypted: bool,
    schemaVersion: Option<u32>,
    mutation: PreferencesSyncedMutation,
    id: String,
    revision: crate::Transaction::Revision,
}

#[derive(Clone, Debug, PartialEq)]
enum PreferencesSyncedMutation {
    ReplaceRecords { prefix: String, records: BTreeMap<String,Value> },
    Transaction(Vec<PreferencesSyncedMutation>),
    AliasEntry {
        key: String,
        target: String,
    },
    SetEntry {
        key: String,
        value: Value,
    },
    DeleteEntry {
        key: String,
    },
    IncrementEntry {
        key: String,
        delta: i64,
    },
    PatchObject {
        key: String,
        patch: PreferencesObjectPatch,
    },
    SetJson {
        key: String,
        path: Vec<PreferencesSyncJsonPathSegment>,
        value: Value,
    },
    DeleteJson {
        key: String,
        path: Vec<PreferencesSyncJsonPathSegment>,
    },
}

impl PreferencesSyncedMutation {
    fn visit<F>(&mut self, visitor: &mut F) -> Result<(), PreferencesDataStoreError>
    where F: FnMut(&mut Self) -> Result<(), PreferencesDataStoreError> {
        match self {
            Self::Transaction(mutations) => {
                for mutation in mutations { mutation.visit(visitor)?; }
                Ok(())
            }
            mutation => visitor(mutation),
        }
    }

    /// Enumerates root entities without interpreting encoded string contents.
    fn keys(&self) -> Vec<&str> {
        match self {
            Self::Transaction(mutations) => mutations.iter().flat_map(Self::keys).collect(),
            Self::AliasEntry { key, target } => vec![key.as_str(), target.as_str()],
            Self::ReplaceRecords {prefix,..} => vec![prefix.as_str()],
            Self::SetEntry { key, .. }
            | Self::DeleteEntry { key }
            | Self::IncrementEntry { key, .. }
            | Self::PatchObject { key, .. }
            | Self::SetJson { key, .. }
            | Self::DeleteJson { key, .. } => vec![key.as_str()],
        }
    }
}

impl PreferencesSyncedEntry {
    /// Validates and decodes one generic synchronization operation as a preferences mutation.
    #[allow(non_snake_case)]
    pub fn fromOperation(operation: &SyncOperation) -> Result<Self, PreferencesDataStoreError> {
        if operation.domain != "preferences" {
            return Err(PreferencesDataStoreError::Message(format!(
                "sync operation does not belong to preferences: {}",
                operation.domain
            )));
        }
        if let Some(version) = operation.payload.get("contractVersion") {
            if version.as_u64() != Some(1)
                || operation
                    .payload
                    .get("logicalTime")
                    .and_then(Value::as_i64)
                    .is_none_or(|time| time < 1)
            {
                return Err(PreferencesDataStoreError::Message(
                    "Invalid preference contract version or logical time".into(),
                ));
            }
        }
        if operation.payload.get("key").and_then(Value::as_str) == Some(PREFERENCES_REPLICATION_KEY)
        {
            return Err(PreferencesDataStoreError::Message(
                "Replication metadata cannot be synchronized as business data".into(),
            ));
        }
        if operation.operation == "transaction" {
            if operation.semantics != SyncOperationSemantics::Transaction {
                return Err(PreferencesDataStoreError::Message(
                    "Preference transactions require transaction semantics".into(),
                ));
            }
            let payload: PreferencesSyncTransactionPayload =
                serde_json::from_value(operation.payload.clone())?;
            validatePluginPreferenceSyncPath(&payload.storagePath)?;
            if payload.formatVersion != 2 || payload.entries.len() < 2 {
                return Err(PreferencesDataStoreError::Message(
                    "Invalid preference transaction version or entry count".into(),
                ));
            }
            let mut mutations = Vec::new();
            let mut keys = BTreeSet::new();
            for entry in payload.entries {
                if entry.operation == "alias"
                    || matches!(
                        entry.key.as_str(),
                        PREFERENCES_SCHEMA_VERSION_KEY_NAME | PREFERENCES_REPLICATION_KEY
                    )
                    || !keys.insert(entry.key.clone())
                {
                    return Err(PreferencesDataStoreError::Message(
                        "Preference transactions must have unique business keys".into(),
                    ));
                }
                let entryPayload = PreferencesSyncEntryPayload {
                    storagePath: payload.storagePath.clone(),
                    key: entry.key,
                    value: entry.value,
                    encrypted: payload.encrypted,
                    formatVersion: 2,
                    schemaVersion: payload.schemaVersion,
                    migrationSnapshot: payload.migrationSnapshot,
                    importCoverage: payload.importCoverage.clone(),
                    jsonPath: Vec::new(),
                    jsonMutation: None,
                };
                mutations.push(decodePreferencesSyncedMutation(
                    &entry.operation,
                    &entryPayload,
                )?);
            }
            let expected = serde_json::to_string(&(payload.storagePath.as_str(), keys))?;
            if operation.entityId != expected {
                return Err(PreferencesDataStoreError::Message(
                    "Preference transaction identity does not match its entries".into(),
                ));
            }
            return Ok(Self {
                id: operation.opId.clone(),
                revision: crate::Transaction::Revision::fromOperation(operation),
                storagePath: payload.storagePath,
                encrypted: payload.encrypted,
                schemaVersion: payload.schemaVersion,
                mutation: PreferencesSyncedMutation::Transaction(mutations),
            });
        }
        let payload: PreferencesSyncEntryPayload =
            serde_json::from_value(operation.payload.clone())?;
        validatePluginPreferenceSyncPath(&payload.storagePath)?;
        match payload.formatVersion {
            1 => {}
            2 => {
                if matches!(
                    payload.key.as_str(),
                    PREFERENCES_SCHEMA_VERSION_KEY_NAME | PREFERENCES_REPLICATION_KEY
                ) {
                    return Err(PreferencesDataStoreError::Message(
                        "Schema metadata cannot be synchronized as a business entry".into(),
                    ));
                }
                if !payload.jsonPath.is_empty() || payload.jsonMutation.is_some() {
                    return Err(PreferencesDataStoreError::Message(
                        "Native JSON preferences must synchronize complete entries".into(),
                    ));
                }
            }
            version => {
                return Err(PreferencesDataStoreError::Message(format!(
                    "Unsupported preference payload version: {version}"
                )))
            }
        }
        let expectedEntityId =
            preferenceMutationEntityId(&payload.storagePath, &payload.key, &payload.jsonPath)?;
        if operation.entityId != expectedEntityId {
            return Err(PreferencesDataStoreError::Message(
                "preference sync entity id does not match its payload".to_string(),
            ));
        }
        if matches!(
            operation.operation.as_str(),
            "increment" | "patch_object" | "alias"
        ) && operation.semantics != SyncOperationSemantics::Transaction
        {
            return Err(PreferencesDataStoreError::Message(
                "Preference deltas and aliases require transaction semantics".into(),
            ));
        }
        let mutation =
            if payload.formatVersion == 1 && payload.key == PREFERENCES_SCHEMA_VERSION_KEY_NAME {
                PreferencesSyncedMutation::Transaction(Vec::new())
            } else {
                decodePreferencesSyncedMutation(&operation.operation, &payload)?
            };
        Ok(Self {
                id: operation.opId.clone(),
                revision: crate::Transaction::Revision::fromOperation(operation),
            storagePath: payload.storagePath,
            encrypted: payload.encrypted,
            schemaVersion: payload.schemaVersion,
            mutation,
        })
    }

    /// Reads a validated dataset identity without requiring business payload decoding.
    pub(crate) fn operationPath(
        operation: &SyncOperation,
    ) -> Result<&str, PreferencesDataStoreError> {
        if operation.domain != "preferences" {
            return Err(PreferencesDataStoreError::Message(
                "Not a preference operation".into(),
            ));
        }
        let path = operation
            .payload
            .get("storagePath")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                PreferencesDataStoreError::Message("Preference dataset path is missing".into())
            })?;
        validatePluginPreferenceSyncPath(path)?;
        Ok(path)
    }

    pub(crate) fn routeTo(&mut self, path: &str) { self.storagePath = path.into(); }

    /// Exposes an immutable identity decision to the schema-validated catalog materializer.
    pub(crate) fn identityAlias(&self) -> Option<(&str, &str)> {
        match &self.mutation {
            PreferencesSyncedMutation::AliasEntry { key, target } => Some((key, target)),
            _ => None,
        }
    }

    /// Returns the explicit preference schema carried by this operation.
    pub(crate) fn schemaVersion(&self) -> Option<u32> {
        self.schemaVersion
    }

    /// Rejects a different preference representation before any values or receipts are committed.
    pub(crate) fn requireSchemaVersion(&self, local: u32) -> Result<(), PreferencesDataStoreError> {
        let remote = self.schemaVersion();
        if remote.unwrap_or(0) > local {
            let remote = remote.map(|version| version.to_string()).unwrap_or_else(|| "missing".into());
            return Err(PreferencesDataStoreError::Message(format!(
                "preference schema version mismatch: path={} local={local} remote={remote}; synchronization skipped",
                self.storagePath
            )));
        }
        Ok(())
    }

    /// Returns the root entity keys affected by one validated operation.
    pub(crate) fn keys(&self) -> Vec<&str> {
        self.mutation.keys()
    }

    /// Rewrites complete root entries through a validated caller-owned identity mapping.
    pub(crate) fn rewriteRootEntries<F>(
        &mut self,
        mut rewrite: F,
    ) -> Result<(), PreferencesDataStoreError>
    where
        F: FnMut(&mut String, Option<&mut Value>) -> Result<(), PreferencesDataStoreError>,
    {
        /// Visits atomic root mutations without inspecting business fields.
        fn visit<F>(
            mutation: &mut PreferencesSyncedMutation,
            rewrite: &mut F,
        ) -> Result<(), PreferencesDataStoreError>
        where
            F: FnMut(&mut String, Option<&mut Value>) -> Result<(), PreferencesDataStoreError>,
        {
            match mutation {
                PreferencesSyncedMutation::Transaction(entries) => {
                    for entry in entries {
                        visit(entry, rewrite)?;
                    }
                    Ok(())
                }
                PreferencesSyncedMutation::ReplaceRecords {prefix,records} => {
                    let mut rewritten = BTreeMap::new();
                    for (id,mut value) in std::mem::take(records) {
                        let mut key = format!("{prefix}{id}");
                        rewrite(&mut key,Some(&mut value))?;
                        let id = key.strip_prefix(prefix.as_str()).filter(|id|!id.is_empty())
                            .ok_or_else(||PreferencesDataStoreError::Message("Alias moved a record outside its namespace".into()))?;
                        rewritten.insert(id.to_owned(),value);
                    }
                    *records = rewritten;
                    Ok(())
                }
                PreferencesSyncedMutation::SetEntry { key, value } => rewrite(key, Some(value)),
                PreferencesSyncedMutation::DeleteEntry { key } => rewrite(key, None),
                _ => Ok(()),
            }
        }
        visit(&mut self.mutation, &mut rewrite)
    }

    /// Returns the virtual storage path owning this preferences mutation.
    #[allow(non_snake_case)]
    pub fn storagePath(&self) -> &str {
        &self.storagePath
    }

    /// Reports the validated storage encryption policy to the shared materializer.
    pub(crate) fn encrypted(&self) -> bool {
        self.encrypted
    }

    pub(crate) fn transaction(&self) -> Result<crate::Transaction::Transaction, PreferencesDataStoreError> {
        use crate::Transaction::{Edit, Target, Transaction};
        fn convert(mutation: &PreferencesSyncedMutation, edits: &mut Vec<Edit>) -> Result<(), PreferencesDataStoreError> {
            match mutation {
                PreferencesSyncedMutation::Transaction(mutations) => for mutation in mutations { convert(mutation, edits)?; },
                PreferencesSyncedMutation::ReplaceRecords {prefix,records} => edits.push(Edit::ReplaceRecords {prefix:prefix.clone(),records:records.clone()}),
                PreferencesSyncedMutation::AliasEntry { .. } => return Err(PreferencesDataStoreError::Message("Identity aliases require a declared catalog materializer".into())),
                PreferencesSyncedMutation::SetEntry { key, value } => edits.push(Edit::Set { key: key.clone(), path: vec![], value: value.clone() }),
                PreferencesSyncedMutation::DeleteEntry { key } => edits.push(Edit::Delete { key: key.clone(), path: vec![] }),
                PreferencesSyncedMutation::IncrementEntry { key, delta } => edits.push(Edit::Increment { key: key.clone(), path: vec![], delta: *delta }),
                PreferencesSyncedMutation::PatchObject { key, patch } => edits.push(Edit::Patch { key: key.clone(), set: patch.set.clone(), delete: patch.delete.iter().cloned().collect() }),
                PreferencesSyncedMutation::SetJson { key, path, value } => edits.push(Edit::Set { key: key.clone(), path: path.clone(), value: value.clone() }),
                PreferencesSyncedMutation::DeleteJson { key, path } => edits.push(Edit::Delete { key: key.clone(), path: path.clone() }),
            }
            Ok(())
        }
        let mut edits = Vec::new();
        convert(&self.mutation, &mut edits)?;
        Ok(Transaction { id: self.id.clone(), revision: self.revision.clone(), target: Target::Data(preferenceOwner(&self.storagePath, self.schemaVersion.unwrap_or(0))), edits })
    }
}

type PreferencesSyncJsonPathSegment = crate::Transaction::Path;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum PreferencesSyncJsonMutation { Set(Value), Delete }

struct PreferencesDataStoreChangeSignal {
    version: Mutex<u64>,
    changed: Condvar,
    subscribers: Mutex<PreferencesDataStoreFlowSubscribers>,
}

struct PreferencesDataStoreFlowSubscribers {
    nextId: usize,
    callbacks: HashMap<usize, Arc<dyn Fn() + Send + Sync>>,
}

struct PreferencesDataStoreFlowSubscription {
    signal: Weak<PreferencesDataStoreChangeSignal>,
    id: usize,
}

#[derive(Default)]
struct PreferencesDataStoreSharedState {
    transaction: Mutex<()>,
    preferences: Mutex<PreferencesDataStoreLoadedPreferences>,
}

#[derive(Default)]
struct PreferencesDataStoreLoadedPreferences {
    loaded: bool,
    preferences: Preferences,
}

struct PreferencesDataStoreRegistryKey {
    pub(crate) storageHost: Arc<dyn RuntimeStorageHost>,
    pub(crate) storagePath: String,
}

impl PartialEq for PreferencesDataStoreRegistryKey {
    /// Compares registry keys by storage-host identity and virtual storage path.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storageHost, &other.storageHost) && self.storagePath == other.storagePath
    }
}

impl Eq for PreferencesDataStoreRegistryKey {}

impl Hash for PreferencesDataStoreRegistryKey {
    /// Hashes registry keys by storage-host identity and virtual storage path.
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.storageHost) as *const () as usize).hash(state);
        self.storagePath.hash(state);
    }
}

enum StoredEncryptedPreferences {
    LegacyPlaintext(Preferences),
    EncryptedEnvelope(Vec<u8>),
}

impl Drop for PreferencesDataStoreFlowSubscription {
    fn drop(&mut self) {
        let Some(signal) = self.signal.upgrade() else {
            return;
        };
        if let Ok(mut subscribers) = signal.subscribers.lock() {
            subscribers.callbacks.remove(&self.id);
        }
        signal.changed.notify_all();
    }
}

impl PreferencesDataStore {
    /// Creates a preferences store backed by the default runtime storage host.
    pub fn new(path: PathBuf) -> Self {
        let storageHost = defaultRuntimeStorageHost();
        let storagePath = runtimeStoragePath(&path);
        Self::newSpaceWithResolvedPath(storageHost, path, storagePath, false)
    }

    #[allow(non_snake_case)]
    /// Creates an encrypted Space preferences store backed by the default runtime storage host.
    pub fn newEncrypted(path: PathBuf) -> Self {
        let storageHost = defaultRuntimeStorageHost();
        let storagePath = runtimeStoragePath(&path);
        Self::newSpaceWithResolvedPath(storageHost, path, storagePath, true)
    }

    #[allow(non_snake_case)]
    /// Creates a Space preferences store backed by an explicit storage host and path.
    pub fn newWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        Self::newSpaceWithResolvedPath(storageHost, path, storagePath, false)
    }

    #[allow(non_snake_case)]
    /// Creates an encrypted Space preferences store backed by an explicit storage host and path.
    pub fn newEncryptedWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        Self::newSpaceWithResolvedPath(storageHost, path, storagePath, true)
    }

    /// Builds one synchronized preferences store after resolving its virtual storage path.
    #[allow(non_snake_case)]
    fn newSpaceWithResolvedPath(
        storageHost: Arc<dyn RuntimeStorageHost>,
        path: PathBuf,
        storagePath: String,
        encrypted: bool,
    ) -> Self {
        let changeSignal = preferencesDataStoreChangeSignal(&storageHost, &storagePath);
        let sharedState = preferencesDataStoreSharedState(&storageHost, &storagePath);
        let encryption = encrypted.then(|| {
            PreferencesEncryption::load_or_create(storageHost.as_ref())
                .expect("preferences encryption key must be available")
        });
        let syncDescriptor = PreferencesSyncDescriptor::forStoragePath(&storagePath);
        Self {
            path,
            storagePath,
            storageHost: storageHost.clone(),
            encryption,
            syncOperationStore: Some(SyncOperationStore::new(
                storageHost,
                RUNTIME_SYNC_DIR_PATH.to_string(),
            )),
            syncDescriptor: Some(syncDescriptor),
            changeSignal,
            sharedState,
            schema: None,
        }
    }

    /// Builds one node-local store from an explicit virtual storage path.
    #[allow(non_snake_case)]
    fn newNodeLocalWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
        encrypted: bool,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        Self::newNodeLocalWithResolvedPath(storageHost, path, storagePath, encrypted)
    }

    /// Builds one node-local store after resolving its virtual storage path.
    #[allow(non_snake_case)]
    fn newNodeLocalWithResolvedPath(
        storageHost: Arc<dyn RuntimeStorageHost>,
        path: PathBuf,
        storagePath: String,
        encrypted: bool,
    ) -> Self {
        let changeSignal = preferencesDataStoreChangeSignal(&storageHost, &storagePath);
        let sharedState = preferencesDataStoreSharedState(&storageHost, &storagePath);
        let encryption = encrypted.then(|| {
            PreferencesEncryption::load_or_create(storageHost.as_ref())
                .expect("preferences encryption key must be available")
        });
        Self {
            path,
            storagePath,
            storageHost,
            encryption,
            syncOperationStore: None,
            syncDescriptor: None,
            changeSignal,
            sharedState,
            schema: None,
        }
    }

    #[allow(non_snake_case)]
    /// Attaches a current schema version and a dispatcher for each one-step migration.
    pub fn withSchema<F>(mut self, currentVersion: u32, migrate: F) -> Self
    where
        F: Fn(u32, &mut Preferences) -> Result<(), PreferencesDataStoreError>
            + Send
            + Sync
            + 'static,
    {
        self.schema = Some(PreferencesSchema {
            currentVersion,
            migrate: Arc::new(crate::Transaction::migration::Definition::from_preferences(migrate)),
        });
        self
    }

    /// Uses the transaction module's one migration interface for state and immutable operation views.
    pub fn withMigration<F>(mut self, currentVersion: u32, migrate: F) -> Self
    where F: Fn(u32, &mut crate::Transaction::migration::Context) -> Result<(), PreferencesDataStoreError> + Send + Sync + 'static {
        self.schema = Some(PreferencesSchema { currentVersion,
            migrate: Arc::new(crate::Transaction::migration::Definition::new(migrate)) });
        self
    }

    /// Returns the logical path associated with this preferences store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(non_snake_case)]
    /// Returns the schema version after applying this store's declared migrations.
    pub fn schemaVersion(&self) -> Result<u32, PreferencesDataStoreError> {
        Self::readSchemaVersion(&self.data()?)
    }

    /// Resolves the owner's replay version without publishing a migrated or absent snapshot.
    pub(crate) fn syncSchemaVersion(&self) -> Result<u32, PreferencesDataStoreError> {
        let _transaction = self.lockTransaction();
        self.loadUnlocked()?;
        let preferences = self.loadedPreferences().expect("dataset loaded");
        let storedVersion = Self::readSchemaVersion(&preferences)?;
        let Some(schema) = &self.schema else {
            return Ok(storedVersion);
        };
        if storedVersion > schema.currentVersion {
            return Err(PreferencesDataStoreError::SchemaVersionTooNew {
                actual: storedVersion,
                expected: schema.currentVersion,
            });
        }
        Ok(schema.currentVersion)
    }

    pub(crate) fn publishCommitted(&self, preferences: &Preferences) {
        self.sharedState.preferences.lock().expect("preferences cache mutex").preferences = preferences.clone();
    }

    /// Reads the current preferences snapshot.
    pub fn data(&self) -> Result<Preferences, PreferencesDataStoreError> {
        let retain = self.syncDescriptor.is_some() || self.storageHost.retainPreferencesCache();
        if retain {
            if let Some(preferences) = self.loadedPreferences() {
                if self.schemaIsCurrent(&preferences)? {
                    return Ok(preferences);
                }
            }
        }
        let _transaction = self.lockTransaction();
        self.loadUnlocked()?;
        self.migrateSchemaUnlocked()?;
        let preferences = self.loadedPreferences()
            .expect("PreferencesDataStore must be loaded after a successful load");
        self.releaseNodeLocalCacheUnlocked();
        Ok(preferences)
    }

    /// Called under the transaction lock, after reading or an unchanged edit.
    /// Immutable snapshots held by callers remain valid independently.
    fn releaseNodeLocalCacheUnlocked(&self) {
        if self.syncDescriptor.is_none() && !self.storageHost.retainPreferencesCache() {
            let mut loaded = self.sharedState.preferences.lock()
                .expect("PreferencesDataStore shared state mutex must not be poisoned");
            loaded.loaded = false;
            loaded.preferences = emptyPreferences();
        }
    }

    /// Mutation callbacks work on private snapshots. After an unwind, reload the durable commit
    /// instead of retaining a possibly stale cache or poisoning all future use of this file.
    pub(crate) fn lockTransaction(&self) -> std::sync::MutexGuard<'_, ()> {
        match self.sharedState.transaction.lock() {
            Ok(transaction) => transaction,
            Err(poisoned) => {
                let transaction = poisoned.into_inner();
                let mut loaded = self.sharedState.preferences.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                loaded.loaded = false;
                self.sharedState.preferences.clear_poison();
                self.sharedState.transaction.clear_poison();
                transaction
            }
        }
    }

    /// Returns the current cached snapshot when this store has already loaded it.
    pub(crate) fn loadedPreferences(&self) -> Option<Preferences> {
        if self.sharedState.transaction.is_poisoned() { return None; }
        let loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.loaded.then(|| loaded.preferences.clone())
    }

    /// Loads the storage snapshot while the caller owns the shared transaction lock.
    pub(crate) fn loadUnlocked(&self) -> Result<(), PreferencesDataStoreError> {
        if self.loadedPreferences().is_some() {
            return Ok(());
        }
        let preferences = self.readFromStorage()?;
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.loaded = true;
        loaded.preferences = preferences;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Returns whether the loaded snapshot already matches the schema declared by this store.
    fn schemaIsCurrent(
        &self,
        preferences: &Preferences,
    ) -> Result<bool, PreferencesDataStoreError> {
        let Some(schema) = &self.schema else {
            return Ok(true);
        };
        let version = Self::readSchemaVersion(preferences)?;
        if version > schema.currentVersion {
            return Err(PreferencesDataStoreError::SchemaVersionTooNew {
                actual: version,
                expected: schema.currentVersion,
            });
        }
        Ok(version == schema.currentVersion)
    }

    #[allow(non_snake_case)]
    /// Migrates the loaded snapshot to the declared schema while the transaction lock is held.
    fn migrateSchemaUnlocked(&self) -> Result<(), PreferencesDataStoreError> {
        let currentPreferences = self
            .loadedPreferences()
            .expect("PreferencesDataStore must be loaded before schema migration");
        let mut preferences = currentPreferences.clone();
        if self.syncDescriptor.is_some() { crate::Transaction::preference::recover(self, &mut preferences)?; }

        if !self.migratePreferencesToCurrentSchema(&mut preferences)? {
            return Ok(());
        }
        self.writeStoredPreferencesUnlocked(&preferences)?;
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.preferences = preferences;
        drop(loaded);
        self.notifyChanged();
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Applies every required one-step migration to an in-memory preferences snapshot.
    pub(crate) fn migratePreferencesToCurrentSchema(
        &self,
        preferences: &mut Preferences,
    ) -> Result<bool, PreferencesDataStoreError> {
        let Some(schema) = &self.schema else {
            return Ok(false);
        };
        let mut version = Self::readSchemaVersion(preferences)?;
        if version > schema.currentVersion {
            return Err(PreferencesDataStoreError::SchemaVersionTooNew {
                actual: version,
                expected: schema.currentVersion,
            });
        }
        if version == schema.currentVersion {
            return Ok(false);
        }
        while version < schema.currentVersion {
            schema.migrate.state(version, preferences)?;
            version += 1;
        }
        preferences.set(&Self::schemaVersionKey(), schema.currentVersion);
        Ok(true)
    }

    #[allow(non_snake_case)]
    /// Reads the schema version stored in a preferences snapshot, treating legacy files as version zero.
    pub(crate) fn readSchemaVersion(preferences: &Preferences) -> Result<u32, PreferencesDataStoreError> {
        let Some(value) = preferences.get(&Self::schemaVersionKey()) else {
            return Ok(0);
        };
        let decoded = match value {
            Value::String(version) => version.parse::<u32>().ok(),
            Value::Number(version) => version
                .as_u64()
                .and_then(|version| u32::try_from(version).ok()),
            _ => None,
        };
        decoded.ok_or_else(|| PreferencesDataStoreError::InvalidSchemaVersion {
            value: value.to_string(),
        })
    }

    #[allow(non_snake_case)]
    /// Returns the reserved key used to persist a preferences schema version.
    pub(crate) fn schemaVersionKey() -> PreferencesKey {
        stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME)
    }

    pub(crate) fn readFromStorage(&self) -> Result<Preferences, PreferencesDataStoreError> {
        if !self.storageHost.exists(&self.storagePath)? {
            return Ok(emptyPreferences());
        }
        let storedContent = self.storageHost.readBytes(&self.storagePath)?;
        let contentBytes = match &self.encryption {
            Some(encryption) => match classifyStoredEncryptedPreferences(&storedContent)? {
                StoredEncryptedPreferences::LegacyPlaintext(preferences) => {
                    let encrypted = encryption.encrypt(&self.storagePath, &storedContent)?;
                    self.storageHost.writeBytes(&self.storagePath, &encrypted)?;
                    return Ok(preferences);
                }
                StoredEncryptedPreferences::EncryptedEnvelope(content) => {
                    encryption.decrypt(&self.storagePath, &content)?
                }
            },
            None => storedContent,
        };
        let content = String::from_utf8(contentBytes)
            .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?;
        if content.trim().is_empty() {
            return Ok(emptyPreferences());
        }
        Ok(serde_json::from_str(&content)?)
    }

    /// Returns an observed flow of preference snapshots.
    pub fn dataFlow(&self) -> Flow<Preferences> {
        let store = self.clone();
        let signal = Arc::clone(&self.changeSignal);
        let observation = preferencesDataStoreFlowObservation(Arc::clone(&signal));
        Flow::newObservedWithObservation(
            move || store.data(),
            move |cancellation| {
                let signalForCancel = Arc::clone(&signal);
                let _cancelHook = cancellation.addCancelHook(move || {
                    signalForCancel.changed.notify_all();
                });
                let mut versionGuard = signal
                    .version
                    .lock()
                    .expect("PreferencesDataStore version mutex must not be poisoned");
                let observedVersion = *versionGuard;
                // This mirrors a cancellable DataStore Flow collect: normally it
                // sleeps until edit/applySyncedPreferences bumps the version, while
                // watch close wakes it through the cancellation hook above.
                while *versionGuard == observedVersion && !cancellation.isCancelled() {
                    versionGuard = signal
                        .changed
                        .wait(versionGuard)
                        .expect("PreferencesDataStore version mutex must not be poisoned");
                }
                !cancellation.isCancelled()
            },
            observation,
        )
    }

    /// Edits preferences in memory and persists the updated snapshot.
    pub fn edit<F>(&self, transform: F) -> Result<(), PreferencesDataStoreError>
    where
        F: FnOnce(&mut Preferences),
    {
        self.try_edit_result(|preferences| {
            transform(preferences);
            Ok(())
        })
    }

    /// Edits preferences and returns a value produced by the edit closure.
    pub fn edit_result<F, T>(&self, transform: F) -> Result<T, PreferencesDataStoreError>
    where
        F: FnOnce(&mut Preferences) -> T,
    {
        self.try_edit_result(|preferences| Ok(transform(preferences)))
    }

    /// Edits preferences with a caller-defined error type.
    pub fn try_edit_result<F, T, E>(&self, transform: F) -> Result<T, E>
    where
        F: FnOnce(&mut Preferences) -> Result<T, E>,
        E: From<PreferencesDataStoreError>,
    {
        self.try_edit_result_internal(transform, true)
    }

    /// Applies a local migration or repair without emitting synchronization operations.
    pub fn migrate<F, T, E>(&self, transform: F) -> Result<T, E>
    where
        F: FnOnce(&mut Preferences) -> Result<T, E>,
        E: From<PreferencesDataStoreError>,
    {
        self.try_edit_result_internal(transform, false)
    }

    /// Applies one preference mutation with explicit synchronization behavior.
    fn try_edit_result_internal<F, T, E>(
        &self,
        transform: F,
        recordSyncOperations: bool,
    ) -> Result<T, E>
    where
        F: FnOnce(&mut Preferences) -> Result<T, E>,
        E: From<PreferencesDataStoreError>,
    {
        let _transaction = self.lockTransaction();
        self.loadUnlocked().map_err(E::from)?;
        if self.syncDescriptor.is_none() && !self.storageHost.retainPreferencesCache() {
            // The durable snapshot supplies rollback. Move the parsed state out
            // before editing so a new record does not copy every retained tree.
            let mut preferences = {
                let mut loaded = self.sharedState.preferences.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                loaded.loaded = false;
                std::mem::replace(&mut loaded.preferences, emptyPreferences())
            };
            preferences.pendingMutations.clear();
            let before = Arc::downgrade(&preferences.values);
            let migrated = self.migratePreferencesToCurrentSchema(&mut preferences).map_err(E::from)?;
            let result = transform(&mut preferences)?;
            let unchanged = before.upgrade().is_some_and(|old| Arc::ptr_eq(&old, &preferences.values));
            if !migrated && unchanged { return Ok(result); }
            self.writeStoredPreferencesUnlocked(&preferences).map_err(E::from)?;
            drop(_transaction);
            self.notifyChanged();
            return Ok(result);
        }
        let mut currentPreferences = self
            .loadedPreferences()
            .expect("PreferencesDataStore must be loaded before editing");
        if self.syncDescriptor.is_some() {
            crate::Transaction::preference::recover(self, &mut currentPreferences).map_err(E::from)?;
        }
        // Stage schema changes with the edit. A failed first write must not
        // leave a schema-only file where readers expect a complete record.
        let migrated = self.migratePreferencesToCurrentSchema(&mut currentPreferences).map_err(E::from)?;
        let mut preferences = currentPreferences.clone();
        preferences.pendingMutations.clear();
        let result = transform(&mut preferences)?;
        if !migrated && (Arc::ptr_eq(&preferences.values, &currentPreferences.values)
            || preferences == currentPreferences) {
            self.releaseNodeLocalCacheUnlocked();
            return Ok(result);
        }
        if recordSyncOperations {
            self.persistEditUnlocked(currentPreferences, &mut preferences)
                .map_err(E::from)?;
        } else {
            self.writeStoredPreferencesUnlocked(&preferences)
                .map_err(E::from)?;
        }
        preferences.pendingMutations.clear();
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        let retain = self.syncDescriptor.is_some() || self.storageHost.retainPreferencesCache();
        loaded.loaded = retain;
        loaded.preferences = if retain { preferences } else { emptyPreferences() };
        drop(loaded);
        drop(_transaction);
        self.notifyChanged();
        Ok(result)
    }

    /// Replaces the full preferences snapshot and notifies observers.
    pub fn replace(&self, mut preferences: Preferences) -> Result<(), PreferencesDataStoreError> {
        let _transaction = self.lockTransaction();
        if self.syncDescriptor.is_none() && self.encryption.is_none() && !self.storageHost.retainPreferencesCache() {
            self.releaseNodeLocalCacheUnlocked();
            let bytes = if self.storageHost.exists(&self.storagePath)? {
                self.storageHost.readBytes(&self.storagePath)?
            } else { Vec::new() };
            let entries: BTreeMap<String, &serde_json::value::RawValue> = if bytes.iter().all(u8::is_ascii_whitespace) {
                BTreeMap::new()
            } else { serde_json::from_slice(&bytes)? };
            let mut probe = emptyPreferences();
            if let Some(version) = entries.get(PREFERENCES_SCHEMA_VERSION_KEY_NAME) {
                probe.set(&Self::schemaVersionKey(), serde_json::from_str::<Value>(version.get())?);
            }
            if self.schemaIsCurrent(&probe)? {
                if let Some(schema) = &self.schema {
                    preferences.set(&Self::schemaVersionKey(), schema.currentVersion);
                }

                drop(entries);
                let content = self.encodeStoredPreferences(&preferences)?;
                if content == bytes { return Ok(()); }
                drop(bytes);
                self.storageHost.writeBytesAtomically(&self.storagePath, &content)?;
                drop(_transaction);
                self.notifyChanged();
                return Ok(());
            }
        }
        self.loadUnlocked()?;
        let mut currentPreferences = self
            .loadedPreferences()
            .expect("PreferencesDataStore must be loaded before replacement");
        let migrated = self.migratePreferencesToCurrentSchema(&mut currentPreferences)?;
        if let Some(schema) = &self.schema {
            preferences.set(&Self::schemaVersionKey(), schema.currentVersion);
        }
        if !migrated && currentPreferences == preferences {
            self.releaseNodeLocalCacheUnlocked();
            return Ok(());
        }
        self.persistEditUnlocked(currentPreferences, &mut preferences)?;
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        let retain = self.syncDescriptor.is_some() || self.storageHost.retainPreferencesCache();
        loaded.loaded = retain;
        loaded.preferences = if retain { preferences } else { emptyPreferences() };
        drop(loaded);
        drop(_transaction);
        self.notifyChanged();
        Ok(())
    }

    /// Replaces a complete record through the same atomic state-and-outbox commit as ordinary edits.
    #[allow(non_snake_case)]
    pub fn replaceRecoverably(
        &self,
        preferences: Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        self.replace(preferences)
    }

    /// Writes one preferences snapshot while the caller owns the shared transaction lock.
    pub(crate) fn writeStoredPreferencesUnlocked(
        &self,
        preferences: &Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        let content = self.encodeStoredPreferences(preferences)?;
        self.storageHost.writeBytesAtomically(&self.storagePath, &content)?;
        Ok(())
    }

    fn encodeStoredPreferences(&self, preferences: &Preferences) -> Result<Vec<u8>, PreferencesDataStoreError> {
        if preferences.values.contains_key(PREFERENCES_REPLICATION_KEY) {
            return Err(PreferencesDataStoreError::Message(
                "Replication metadata is reserved for the dataset commit protocol".into(),
            ));
        }
        // Preserve the existing sorted JSON encoding without copying business
        // value trees while both the old and new snapshots are still live.
        let mut values = preferences.iterEntries().collect::<BTreeMap<_, _>>();
        let commit = preferences.commit.as_ref().map(serde_json::to_value).transpose()?;
        if let Some(ref commit) = commit { values.insert("__operit_transaction_commit", commit); }

        let bounded = self.syncDescriptor.is_none() && !self.storageHost.retainPreferencesCache();
        let mut content = PreferencesJsonBuffer { bytes: Vec::new(), measured: 0, countOnly: bounded, bounded };
        // PrettyFormatter with an empty indent still emits newlines and spaces.
        // Node-local constrained storage needs compact JSON; retained Core and
        // replicated datasets keep their existing human-readable encoding.
        let encode = |output: &mut PreferencesJsonBuffer| {
            if bounded { values.serialize(&mut serde_json::Serializer::new(output)) }
            else { values.serialize(&mut serde_json::Serializer::pretty(output)) }
        };
        if bounded {
            encode(&mut content)?;
            content.bytes.try_reserve_exact(content.measured)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
            content.countOnly = false;
        }
        encode(&mut content)?;
        let storedContent = match &self.encryption {
            Some(encryption) => encryption.encrypt(&self.storagePath, &content.bytes)?,
            None => content.bytes,
        };
        Ok(storedContent)
    }

    /// Node-local edits on constrained Hosts may discard the old parsed cache
    /// while the transaction lock protects publication. Durable rollback bytes
    /// remain authoritative; after failure all handles reload them on demand.
    fn persistEditUnlocked(
        &self,
        previous: Preferences,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        if self.syncDescriptor.is_none() && !self.storageHost.retainPreferencesCache() {
            preferences.commit = previous.commit.clone();
            {
                let mut loaded = self.sharedState.preferences.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                loaded.loaded = false;
                loaded.preferences = emptyPreferences();
            }
            drop(previous);
            self.writeStoredPreferencesUnlocked(preferences)
        } else {
            self.persistUnlocked(&previous, preferences)
        }
    }

    /// Persists imported initial values and the new business edit at one publication point.
    fn persistUnlocked(&self, previous: &Preferences, preferences: &mut Preferences) -> Result<(), PreferencesDataStoreError> {
        if self.syncDescriptor.is_some() {
            crate::Transaction::preference::commit_local(self, previous, preferences)
        } else {
            self.writeStoredPreferencesUnlocked(preferences)
        }
    }

    #[allow(non_snake_case)]
    pub(crate) fn notifyChanged(&self) {
        let mut version = self
            .changeSignal
            .version
            .lock()
            .expect("PreferencesDataStore version mutex must not be poisoned");
        *version += 1;
        self.changeSignal.changed.notify_all();
        drop(version);
        notifyPreferencesDataStoreSubscribers(&self.changeSignal);
        // Wake host SDK observers after publishing the new cache, including native edits and replay.
        crate::SyncOperationStore::publishSyncMutation();
    }
}

/// Classifies encrypted-store bytes as a legacy plaintext preferences map or an envelope.
fn classifyStoredEncryptedPreferences(
    content: &[u8],
) -> Result<StoredEncryptedPreferences, PreferencesDataStoreError> {
    let value: serde_json::Value = serde_json::from_slice(content)?;
    if value.get("format").and_then(Value::as_str) == Some("operit.preferences.encrypted") {
        return Ok(StoredEncryptedPreferences::EncryptedEnvelope(
            content.to_vec(),
        ));
    }
    Ok(StoredEncryptedPreferences::LegacyPlaintext(
        serde_json::from_value(value)?,
    ))
}

/// Decodes one validated wire payload into its exact in-memory mutation.
#[allow(non_snake_case)]
fn decodePreferencesSyncedMutation(
    operation: &str,
    payload: &PreferencesSyncEntryPayload,
) -> Result<PreferencesSyncedMutation, PreferencesDataStoreError> {
    if !payload.jsonPath.is_empty() {
        if !payload.value.is_null() {
            return Err(PreferencesDataStoreError::Message(
                "structured preference operation must not include an entry value".to_string(),
            ));
        }
        let jsonMutation = payload.jsonMutation.clone().ok_or_else(|| {
            PreferencesDataStoreError::Message(
                "structured preference operation is missing its JSON mutation".to_string(),
            )
        })?;
        return match (operation, jsonMutation) {
            ("set", PreferencesSyncJsonMutation::Set(value)) => {
                Ok(PreferencesSyncedMutation::SetJson {
                    key: payload.key.clone(),
                    path: payload.jsonPath.clone(),
                    value,
                })
            }
            ("delete", PreferencesSyncJsonMutation::Delete) => {
                Ok(PreferencesSyncedMutation::DeleteJson {
                    key: payload.key.clone(),
                    path: payload.jsonPath.clone(),
                })
            }
            (operation, mutation) => Err(PreferencesDataStoreError::Message(format!(
                "preference sync operation does not match its structured mutation: {operation}/{mutation:?}"
            ))),
        };
    }
    if payload.jsonMutation.is_some() {
        return Err(PreferencesDataStoreError::Message(
            "preference entry operation must not include a structured JSON mutation".to_string(),
        ));
    }
    match operation {
        "alias" if payload.formatVersion == 2 => {
            let target = payload
                .value
                .as_str()
                .filter(|target| !target.is_empty() && *target != payload.key)
                .ok_or_else(|| {
                    PreferencesDataStoreError::Message("Invalid preference identity alias".into())
                })?;
            Ok(PreferencesSyncedMutation::AliasEntry {
                key: payload.key.clone(),
                target: target.into(),
            })
        }
        "increment" if payload.formatVersion == 2 => {
            let delta = payload.value.as_i64().ok_or_else(|| {
                PreferencesDataStoreError::Message("Invalid integer increment".into())
            })?;
            Ok(PreferencesSyncedMutation::IncrementEntry {
                key: payload.key.clone(),
                delta,
            })
        }
        "patch_object" if payload.formatVersion == 2 => {
            let patch: PreferencesObjectPatch = serde_json::from_value(payload.value.clone())?;
            let mut keys = BTreeSet::new();
            if patch
                .delete
                .iter()
                .any(|key| patch.set.contains_key(key) || !keys.insert(key))
            {
                return Err(PreferencesDataStoreError::Message(
                    "Conflicting object patch fields".into(),
                ));
            }
            Ok(PreferencesSyncedMutation::PatchObject {
                key: payload.key.clone(),
                patch,
            })
        }
        "set" => Ok(PreferencesSyncedMutation::SetEntry {
            key: payload.key.clone(),
            value: payload.value.clone(),
        }),
        "delete" => {
            if !payload.value.is_null() {
                return Err(PreferencesDataStoreError::Message(
                    "preference delete operation must not include a value".to_string(),
                ));
            }
            Ok(PreferencesSyncedMutation::DeleteEntry {
                key: payload.key.clone(),
            })
        }
        other => Err(PreferencesDataStoreError::Message(format!(
            "unsupported preference sync operation: {other}"
        ))),
    }
}

/// Enforces plugin data ownership without changing the separate native preference namespace contract.
fn validatePluginPreferenceSyncPath(path: &str) -> Result<(), PreferencesDataStoreError> {
    use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
    if path.starts_with("runtime/plugin_data/") {
        let canonical = operit_util::PluginStoragePaths::resolveStoragePath(path)
            .map_err(PreferencesDataStoreError::Message)?;
        if canonical != path || !path.ends_with(".preferences.json") {
            return Err(PreferencesDataStoreError::Message("Invalid synchronized plugin preference path".into()));
        }
        match runtimeStorageOwnership(path).map_err(PreferencesDataStoreError::Message)? {
            RuntimeStorageOwnership::Space => {},
            RuntimeStorageOwnership::CoreNode | RuntimeStorageOwnership::Ephemeral => return Err(PreferencesDataStoreError::Message("Synchronized preferences require Space-owned storage".into())),
        }
    }
    Ok(())
}

/// Encodes a preference entry identity without delimiter or path parsing.
#[allow(non_snake_case)]
fn preferenceEntryEntityId(
    storagePath: &str,
    key: &str,
) -> Result<String, PreferencesDataStoreError> {
    serde_json::to_string(&(storagePath, key)).map_err(PreferencesDataStoreError::from)
}

/// Encodes either one complete preference entry or one structured JSON location.
#[allow(non_snake_case)]
fn preferenceMutationEntityId(
    storagePath: &str,
    key: &str,
    jsonPath: &[PreferencesSyncJsonPathSegment],
) -> Result<String, PreferencesDataStoreError> {
    if jsonPath.is_empty() {
        return preferenceEntryEntityId(storagePath, key);
    }
    serde_json::to_string(&(storagePath, key, jsonPath)).map_err(PreferencesDataStoreError::from)
}

#[allow(non_snake_case)]
/// Builds the process-local registry key for one storage host and preference path.
fn preferencesDataStoreRegistryKey(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    storagePath: &str,
) -> PreferencesDataStoreRegistryKey {
    PreferencesDataStoreRegistryKey {
        storageHost: Arc::clone(storageHost),
        storagePath: storagePath.to_string(),
    }
}

enum PreferencesCacheEntry {
    Retained(Arc<PreferencesDataStoreSharedState>),
    Live(Weak<PreferencesDataStoreSharedState>),
}
impl PreferencesCacheEntry {
    fn upgrade(&self) -> Option<Arc<PreferencesDataStoreSharedState>> {
        match self {
            Self::Retained(state) => Some(state.clone()),
            Self::Live(state) => state.upgrade(),
        }
    }
    fn isLive(&self) -> bool {
        match self {
            Self::Retained(_) => true,
            Self::Live(state) => state.strong_count() > 0,
        }
    }
}

#[allow(non_snake_case)]
/// Returns the preference cache shared by stores using the same storage host and path.
fn preferencesDataStoreSharedState(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    storagePath: &str,
) -> Arc<PreferencesDataStoreSharedState> {
    static SHARED_STATES: OnceLock<
        Mutex<HashMap<PreferencesDataStoreRegistryKey, PreferencesCacheEntry>>,
    > = OnceLock::new();
    let states = SHARED_STATES.get_or_init(|| Mutex::new(HashMap::new()));
    let key = preferencesDataStoreRegistryKey(storageHost, storagePath);
    let mut states = states
        .lock()
        .expect("PreferencesDataStore shared state registry mutex must not be poisoned");
    if let Some(state) = states.get(&key).and_then(PreferencesCacheEntry::upgrade) {
        return state;
    }
    states.retain(|_, state| state.isLive());
    let state = Arc::new(PreferencesDataStoreSharedState::default());
    let entry = if storageHost.retainPreferencesCache() {
        PreferencesCacheEntry::Retained(state.clone())
    } else {
        PreferencesCacheEntry::Live(Arc::downgrade(&state))
    };
    states.insert(key, entry);
    state
}

#[allow(non_snake_case)]
/// Returns the change signal shared by stores using the same storage host and path.
fn preferencesDataStoreChangeSignal(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    storagePath: &str,
) -> Arc<PreferencesDataStoreChangeSignal> {
    static CHANGE_SIGNALS: OnceLock<
        Mutex<HashMap<PreferencesDataStoreRegistryKey, Weak<PreferencesDataStoreChangeSignal>>>,
    > = OnceLock::new();
    let signals = CHANGE_SIGNALS.get_or_init(|| Mutex::new(HashMap::new()));
    let key = preferencesDataStoreRegistryKey(storageHost, storagePath);
    let mut signals = signals
        .lock()
        .expect("PreferencesDataStore change signal registry mutex must not be poisoned");
    if let Some(signal) = signals.get(&key).and_then(Weak::upgrade) {
        return signal;
    }
    let signal = Arc::new(PreferencesDataStoreChangeSignal {
        version: Mutex::new(0),
        changed: Condvar::new(),
        subscribers: Mutex::new(PreferencesDataStoreFlowSubscribers {
            nextId: 0,
            callbacks: HashMap::new(),
        }),
    });
    signals.insert(key, Arc::downgrade(&signal));
    signal
}

#[allow(non_snake_case)]
fn preferencesDataStoreFlowObservation(
    signal: Arc<PreferencesDataStoreChangeSignal>,
) -> FlowObservation {
    FlowObservation {
        subscribe: Arc::new(move |callback| {
            let id = {
                let mut subscribers = signal
                    .subscribers
                    .lock()
                    .expect("PreferencesDataStore subscribers mutex must not be poisoned");
                let id = subscribers.nextId;
                subscribers.nextId += 1;
                subscribers.callbacks.insert(id, callback);
                id
            };
            FlowObservationSubscription {
                _guard: Box::new(PreferencesDataStoreFlowSubscription {
                    signal: Arc::downgrade(&signal),
                    id,
                }),
            }
        }),
    }
}

#[allow(non_snake_case)]
/// Schedules every active preference observer after a successful persistence change.
#[allow(non_snake_case)]
fn notifyPreferencesDataStoreSubscribers(signal: &PreferencesDataStoreChangeSignal) {
    let callbacks = signal
        .subscribers
        .lock()
        .expect("PreferencesDataStore subscribers mutex must not be poisoned")
        .callbacks
        .values()
        .cloned()
        .collect::<Vec<_>>();
    if callbacks.is_empty() {
        return;
    }
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeTask(
            "operit-preferences-observers",
            Box::new(move || {
                for callback in callbacks {
                    callback();
                }
            }),
        )
        .expect("preferences observer task must be scheduled");
}

/// Installs the synchronous observation host used by repository preference tests.
#[cfg(test)]
pub(crate) fn installPreferenceObserverTestHost() {
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
        PreferencesDataStoreTests::ImmediatePreferenceObserverScheduler,
    ));
}

#[cfg(test)]
#[path = "tests/PreferencesDataStoreTests.rs"]
mod PreferencesDataStoreTests;

#[path = "transaction/PreferenceCodec.rs"]
mod preference_codec;
pub(crate) use preference_codec::local_mutation;

impl PreferencesDataStore {
    pub fn prepareSync(&self) -> Result<(), PreferencesDataStoreError> { crate::Transaction::preference::prepare(self) }
    pub fn isSyncPrepared(&self) -> Result<bool, PreferencesDataStoreError> { crate::Transaction::preference::is_prepared(self) }
    pub fn syncSnapshot(&self) -> Result<crate::Transaction::Snapshot, PreferencesDataStoreError> { crate::Transaction::preference::snapshot(self) }
    pub fn repairSyncedSnapshot(&self, expected: &str, source: &crate::Transaction::Snapshot) -> Result<bool, PreferencesDataStoreError> { crate::Transaction::preference::repair(self, expected, source) }
}

impl Preferences {
    pub(crate) fn mergeAliases(&self) -> &BTreeMap<String, String> {
        static EMPTY: OnceLock<BTreeMap<String, String>> = OnceLock::new();
        self.view.as_ref().map(|view| &view.aliases).unwrap_or_else(|| EMPTY.get_or_init(BTreeMap::new))
    }
    pub(crate) fn setMergeAliases(&mut self, aliases: BTreeMap<String, String>) {
        self.view.as_mut().expect("owner materialization view").aliases = aliases;
    }
}

impl PreferencesSyncedEntry {
    pub(crate) fn fromTransaction(transaction: crate::Transaction::Transaction, encrypted: bool) -> Result<Self, PreferencesDataStoreError> {
        use crate::Transaction::{Edit, Target};
        let Target::Data(owner) = transaction.target else { return Err(PreferencesDataStoreError::Message("Expected a data transaction".into())); };
        let mut mutations = Vec::new();
        for edit in transaction.edits {
            mutations.push(match edit {
                Edit::Set { key, path, value } if path.is_empty() => PreferencesSyncedMutation::SetEntry { key, value },
                Edit::Set { key, path, value } => PreferencesSyncedMutation::SetJson { key, path, value },
                Edit::Delete { key, path } if path.is_empty() => PreferencesSyncedMutation::DeleteEntry { key },
                Edit::Delete { key, path } => PreferencesSyncedMutation::DeleteJson { key, path },
                Edit::Increment { key, path, delta } if path.is_empty() => PreferencesSyncedMutation::IncrementEntry { key, delta },
                Edit::Increment { .. } => return Err(PreferencesDataStoreError::Message("Nested preference increment requires an explicit migration codec".into())),
                Edit::ReplaceRecords {prefix,records} => PreferencesSyncedMutation::ReplaceRecords {prefix,records},
                Edit::Patch { key, set, delete } => PreferencesSyncedMutation::PatchObject { key, patch: PreferencesObjectPatch { set, delete: delete.into_iter().collect() } },
            });
        }
        Ok(Self { storagePath: owner.path, encrypted, schemaVersion: Some(owner.format), mutation: PreferencesSyncedMutation::Transaction(mutations), id: transaction.id, revision: transaction.revision })
    }

    pub(crate) fn replay(entries: &[Self], preferences: &mut Preferences) -> Result<(), PreferencesDataStoreError> {
        let mut data = preferences.view.clone().ok_or_else(|| PreferencesDataStoreError::Message("Owner replay context is missing".into()))?;
        for entry in entries { data.apply(&entry.transaction()?).map_err(PreferencesDataStoreError::Message)?; }
        preferences.values = Arc::new(data.values.iter().map(|(k,v)| (k.clone(),v.clone())).collect());
        preferences.set(&PreferencesDataStore::schemaVersionKey(), data.owner.format);
        preferences.view = Some(data);
        Ok(())
    }
}

impl PreferencesDataStore {
    pub(crate) fn publishOutbox(&self, _journal: &SyncOperationStore) -> Result<(), PreferencesDataStoreError> {
        let _lock = self.lockTransaction();
        self.loadUnlocked()?;
        let mut values = self.loadedPreferences().expect("loaded file");
        crate::Transaction::preference::recover(self, &mut values)
    }
    pub(crate) fn migrateSyncedEntries(&self, version: u32, entries: &mut Vec<PreferencesSyncedEntry>) -> Result<(), PreferencesDataStoreError> {
        crate::Transaction::preference::migrate_entries(self,version,entries)
    }
}
