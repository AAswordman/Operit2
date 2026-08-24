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

const PREFERENCES_SCHEMA_VERSION_KEY_NAME: &str = "__operit_preferences_schema_version";

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
        let stateFlowForSubscription = stateFlow.clone();
        if let Ok(subscription) = self.subscribeWithCancellation(cancellation, move |value| {
            stateFlowForSubscription.set_value(value);
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// In-memory representation of a preferences file.
pub struct Preferences {
    values: Arc<HashMap<String, String>>,
}

impl Preferences {
    /// Reads a preference value by key.
    pub fn get(&self, key: &PreferencesKey) -> Option<&String> {
        self.values.get(&key.name)
    }

    /// Updates a preference value and clones the shared map only when needed.
    pub fn set(&mut self, key: &PreferencesKey, value: String) {
        if self.values.get(&key.name) == Some(&value) {
            return;
        }
        Arc::make_mut(&mut self.values).insert(key.name.clone(), value);
    }

    /// Removes a preference value and clones the shared map only when needed.
    pub fn remove(&mut self, key: &PreferencesKey) {
        if !self.values.contains_key(&key.name) {
            return;
        }
        Arc::make_mut(&mut self.values).remove(&key.name);
    }

    /// Checks whether a preference key exists.
    pub fn contains(&self, key: &PreferencesKey) -> bool {
        self.values.contains_key(&key.name)
    }

    /// Returns all preference entries as owned key-value pairs.
    pub fn entries(&self) -> Vec<(String, String)> {
        self.values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

impl Serialize for Preferences {
    /// Serializes preferences as the existing flat JSON object.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.values.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Preferences {
    /// Deserializes the existing flat JSON object into a shared snapshot.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = HashMap::<String, String>::deserialize(deserializer)?;
        Ok(Self {
            values: Arc::new(values),
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

#[derive(Clone)]
/// File-backed preferences store with Flow-like observation and optional encryption.
pub struct PreferencesDataStore {
    path: PathBuf,
    storagePath: String,
    storageHost: Arc<dyn RuntimeStorageHost>,
    encryption: Option<PreferencesEncryption>,
    syncOperationStore: Option<SyncOperationStore>,
    syncDescriptor: Option<PreferencesSyncDescriptor>,
    changeSignal: Arc<PreferencesDataStoreChangeSignal>,
    sharedState: Arc<PreferencesDataStoreSharedState>,
    schema: Option<PreferencesSchema>,
    structuredJsonSync: bool,
}

type PreferencesMigration =
    dyn Fn(u32, &mut Preferences) -> Result<(), PreferencesDataStoreError> + Send + Sync;

#[derive(Clone)]
/// Declares the current schema and one-step migration dispatcher for a preferences file.
struct PreferencesSchema {
    currentVersion: u32,
    migrate: Arc<PreferencesMigration>,
}

#[derive(Clone)]
/// Stores persistent state that belongs only to one CoreNode.
pub struct CoreNodeStateStore {
    inner: PreferencesDataStore,
}

impl CoreNodeStateStore {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Carries one independently mergeable preference entry mutation.
struct PreferencesSyncEntryPayload {
    storagePath: String,
    key: String,
    value: Option<String>,
    encrypted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    jsonPath: Vec<PreferencesSyncJsonPathSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    jsonMutation: Option<PreferencesSyncJsonMutation>,
}

#[derive(Clone, Debug, PartialEq)]
/// Stores one validated incoming preferences mutation ready for batched application.
pub struct PreferencesSyncedEntry {
    storagePath: String,
    encrypted: bool,
    mutation: PreferencesSyncedMutation,
}

#[derive(Clone, Debug, PartialEq)]
enum PreferencesSyncedMutation {
    SetEntry {
        key: String,
        value: String,
    },
    DeleteEntry {
        key: String,
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
        let payload: PreferencesSyncEntryPayload =
            serde_json::from_value(operation.payload.clone())?;
        let expectedEntityId =
            preferenceMutationEntityId(&payload.storagePath, &payload.key, &payload.jsonPath)?;
        if operation.entityId != expectedEntityId {
            return Err(PreferencesDataStoreError::Message(
                "preference sync entity id does not match its payload".to_string(),
            ));
        }
        let mutation = decodePreferencesSyncedMutation(&operation.operation, &payload)?;
        Ok(Self {
            storagePath: payload.storagePath,
            encrypted: payload.encrypted,
            mutation,
        })
    }

    /// Returns the virtual storage path owning this preferences mutation.
    #[allow(non_snake_case)]
    pub fn storagePath(&self) -> &str {
        &self.storagePath
    }

    /// Applies this decoded mutation to one in-memory preferences snapshot.
    fn apply(&self, preferences: &mut Preferences) -> Result<(), PreferencesDataStoreError> {
        match &self.mutation {
            PreferencesSyncedMutation::SetEntry { key, value } => {
                preferences.set(&stringPreferencesKey(key), value.clone());
                Ok(())
            }
            PreferencesSyncedMutation::DeleteEntry { key } => {
                preferences.remove(&stringPreferencesKey(key));
                Ok(())
            }
            PreferencesSyncedMutation::SetJson { key, path, value } => {
                applyStructuredJsonPreferenceMutation(
                    preferences,
                    &stringPreferencesKey(key),
                    path,
                    Some(value.clone()),
                )
            }
            PreferencesSyncedMutation::DeleteJson { key, path } => {
                applyStructuredJsonPreferenceMutation(
                    preferences,
                    &stringPreferencesKey(key),
                    path,
                    None,
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
/// Identifies one independently synchronized location inside a JSON preference value.
enum PreferencesSyncJsonPathSegment {
    Field(String),
    Item(String),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
/// Distinguishes an explicit JSON null assignment from a path deletion.
enum PreferencesSyncJsonMutation {
    Set(Value),
    Delete,
}

#[derive(Clone, Debug, PartialEq)]
/// Carries one JSON mutation produced by structural comparison.
struct PreferencesStructuredJsonMutation {
    path: Vec<PreferencesSyncJsonPathSegment>,
    value: Option<Value>,
}

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
    storageHost: Arc<dyn RuntimeStorageHost>,
    storagePath: String,
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
            structuredJsonSync: false,
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
            structuredJsonSync: false,
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
            migrate: Arc::new(migrate),
        });
        self
    }

    #[allow(non_snake_case)]
    /// Enables recursive JSON-field synchronization for values changed in this store.
    pub fn withStructuredJsonSync(mut self) -> Self {
        self.structuredJsonSync = true;
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

    /// Applies validated synchronized entries for one file in one transaction and notification.
    #[allow(non_snake_case)]
    pub fn applySyncedEntriesWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        entries: &[PreferencesSyncedEntry],
    ) -> Result<(), PreferencesDataStoreError> {
        let Some(first) = entries.first() else {
            return Ok(());
        };
        if entries.iter().any(|entry| {
            entry.storagePath != first.storagePath || entry.encrypted != first.encrypted
        }) {
            return Err(PreferencesDataStoreError::Message(
                "batched preference sync entries must share one storage path and encryption mode"
                    .to_string(),
            ));
        }
        let store =
            Self::newNodeLocalWithStorage(storageHost, first.storagePath.clone(), first.encrypted);
        store.try_edit_result(|preferences| {
            for entry in entries {
                entry.apply(preferences)?;
            }
            Ok(())
        })
    }

    /// Reads the current preferences snapshot.
    pub fn data(&self) -> Result<Preferences, PreferencesDataStoreError> {
        if let Some(preferences) = self.loadedPreferences() {
            if self.schemaIsCurrent(&preferences)? {
                return Ok(preferences);
            }
        }
        let _transaction = self
            .sharedState
            .transaction
            .lock()
            .expect("PreferencesDataStore transaction mutex must not be poisoned");
        self.loadUnlocked()?;
        self.migrateSchemaUnlocked()?;
        Ok(self
            .loadedPreferences()
            .expect("PreferencesDataStore must be loaded after a successful load"))
    }

    /// Returns the current cached snapshot when this store has already loaded it.
    fn loadedPreferences(&self) -> Option<Preferences> {
        let loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.loaded.then(|| loaded.preferences.clone())
    }

    /// Loads the storage snapshot while the caller owns the shared transaction lock.
    fn loadUnlocked(&self) -> Result<(), PreferencesDataStoreError> {
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
    fn migratePreferencesToCurrentSchema(
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
            (schema.migrate)(version, preferences)?;
            version += 1;
        }
        preferences.set(&Self::schemaVersionKey(), schema.currentVersion.to_string());
        Ok(true)
    }

    #[allow(non_snake_case)]
    /// Reads the schema version stored in a preferences snapshot, treating legacy files as version zero.
    fn readSchemaVersion(preferences: &Preferences) -> Result<u32, PreferencesDataStoreError> {
        let Some(value) = preferences.get(&Self::schemaVersionKey()) else {
            return Ok(0);
        };
        value
            .parse::<u32>()
            .map_err(|_| PreferencesDataStoreError::InvalidSchemaVersion {
                value: value.clone(),
            })
    }

    #[allow(non_snake_case)]
    /// Returns the reserved key used to persist a preferences schema version.
    fn schemaVersionKey() -> PreferencesKey {
        stringPreferencesKey(PREFERENCES_SCHEMA_VERSION_KEY_NAME)
    }

    fn readFromStorage(&self) -> Result<Preferences, PreferencesDataStoreError> {
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
        let _transaction = self
            .sharedState
            .transaction
            .lock()
            .expect("PreferencesDataStore transaction mutex must not be poisoned");
        self.loadUnlocked().map_err(E::from)?;
        self.migrateSchemaUnlocked().map_err(E::from)?;
        let currentPreferences = self
            .loadedPreferences()
            .expect("PreferencesDataStore must be loaded before editing");
        let mut preferences = currentPreferences.clone();
        let result = transform(&mut preferences)?;
        if Arc::ptr_eq(&preferences.values, &currentPreferences.values) {
            return Ok(result);
        }
        if preferences == currentPreferences {
            return Ok(result);
        }
        if recordSyncOperations {
            self.persistUnlocked(&currentPreferences, &preferences)
                .map_err(E::from)?;
        } else {
            self.writeStoredPreferencesUnlocked(&preferences)
                .map_err(E::from)?;
        }
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.preferences = preferences;
        drop(loaded);
        self.notifyChanged();
        Ok(result)
    }

    /// Replaces the full preferences snapshot and notifies observers.
    pub fn replace(&self, mut preferences: Preferences) -> Result<(), PreferencesDataStoreError> {
        let _transaction = self
            .sharedState
            .transaction
            .lock()
            .expect("PreferencesDataStore transaction mutex must not be poisoned");
        self.loadUnlocked()?;
        self.migrateSchemaUnlocked()?;
        self.migratePreferencesToCurrentSchema(&mut preferences)?;
        let currentPreferences = self
            .loadedPreferences()
            .expect("PreferencesDataStore must be loaded before replacement");
        if currentPreferences == preferences {
            return Ok(());
        }
        self.persistUnlocked(&currentPreferences, &preferences)?;
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        loaded.loaded = true;
        loaded.preferences = preferences;
        drop(loaded);
        self.notifyChanged();
        Ok(())
    }

    /// Writes one preferences snapshot while the caller owns the shared transaction lock.
    fn writeStoredPreferencesUnlocked(
        &self,
        preferences: &Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        let content = serde_json::to_string_pretty(preferences)?;
        let storedContent = match &self.encryption {
            Some(encryption) => encryption.encrypt(&self.storagePath, content.as_bytes())?,
            None => content.into_bytes(),
        };
        self.storageHost
            .writeBytes(&self.storagePath, &storedContent)?;
        Ok(())
    }

    /// Persists one user-owned snapshot while the caller owns the shared transaction lock.
    fn persistUnlocked(
        &self,
        previous: &Preferences,
        preferences: &Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        self.writeStoredPreferencesUnlocked(preferences)?;
        self.recordSyncOperations(previous, preferences)?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Records independently mergeable mutations for every changed preference key.
    fn recordSyncOperations(
        &self,
        previous: &Preferences,
        preferences: &Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        let Some(syncOperationStore) = &self.syncOperationStore else {
            return Ok(());
        };
        let Some(descriptor) = &self.syncDescriptor else {
            return Ok(());
        };
        let deviceId = syncOperationStore.localDeviceId()?;
        let previousEntries = previous.entries().into_iter().collect::<BTreeMap<_, _>>();
        let nextEntries = preferences
            .entries()
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let mut keys = previousEntries
            .keys()
            .chain(nextEntries.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let schemaVersionChanged = keys.remove(PREFERENCES_SCHEMA_VERSION_KEY_NAME);
        for key in keys
            .into_iter()
            .chain(schemaVersionChanged.then(|| PREFERENCES_SCHEMA_VERSION_KEY_NAME.to_string()))
        {
            let previousValue = previousEntries.get(&key);
            let nextValue = nextEntries.get(&key);
            if previousValue == nextValue {
                continue;
            }
            if self.structuredJsonSync {
                if let (Some(previousValue), Some(nextValue)) = (previousValue, nextValue) {
                    let previousJson: Value = serde_json::from_str(previousValue)?;
                    let nextJson: Value = serde_json::from_str(nextValue)?;
                    let mutations = structuredJsonMutations(&previousJson, &nextJson);
                    if !mutations.is_empty()
                        && mutations.iter().all(|mutation| !mutation.path.is_empty())
                    {
                        for mutation in mutations {
                            self.appendPreferenceSyncOperation(
                                syncOperationStore,
                                descriptor,
                                &deviceId,
                                &key,
                                None,
                                mutation.path,
                                mutation.value,
                            )?;
                        }
                        continue;
                    }
                }
            }
            self.appendPreferenceSyncOperation(
                syncOperationStore,
                descriptor,
                &deviceId,
                &key,
                nextValue.cloned(),
                Vec::new(),
                None,
            )?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Appends one entry-level or structured JSON preferences operation.
    fn appendPreferenceSyncOperation(
        &self,
        syncOperationStore: &SyncOperationStore,
        descriptor: &PreferencesSyncDescriptor,
        deviceId: &str,
        key: &str,
        value: Option<String>,
        jsonPath: Vec<PreferencesSyncJsonPathSegment>,
        jsonValue: Option<Value>,
    ) -> Result<(), PreferencesDataStoreError> {
        let structured = !jsonPath.is_empty();
        let jsonMutation = if structured {
            if value.is_some() {
                return Err(PreferencesDataStoreError::Message(
                    "structured preference operation must not include an entry value".to_string(),
                ));
            }
            Some(match jsonValue {
                Some(value) => PreferencesSyncJsonMutation::Set(value),
                None => PreferencesSyncJsonMutation::Delete,
            })
        } else {
            if jsonValue.is_some() {
                return Err(PreferencesDataStoreError::Message(
                    "preference entry operation must not include a structured JSON value"
                        .to_string(),
                ));
            }
            None
        };
        let operation = if value.is_some()
            || matches!(&jsonMutation, Some(PreferencesSyncJsonMutation::Set(_)))
        {
            "set"
        } else {
            "delete"
        };
        syncOperationStore.appendLocalOperation(
            deviceId,
            NewSyncOperation {
                domain: descriptor.domain.clone(),
                entityType: descriptor.entityType.clone(),
                entityId: preferenceMutationEntityId(&descriptor.storagePath, key, &jsonPath)?,
                operation: operation.to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: serde_json::to_value(PreferencesSyncEntryPayload {
                    storagePath: descriptor.storagePath.clone(),
                    key: key.to_string(),
                    value,
                    encrypted: self.encryption.is_some(),
                    jsonPath,
                    jsonMutation,
                })?,
            },
        )?;
        Ok(())
    }

    #[allow(non_snake_case)]
    fn notifyChanged(&self) {
        let mut version = self
            .changeSignal
            .version
            .lock()
            .expect("PreferencesDataStore version mutex must not be poisoned");
        *version += 1;
        self.changeSignal.changed.notify_all();
        drop(version);
        notifyPreferencesDataStoreSubscribers(&self.changeSignal);
    }
}

/// Classifies encrypted-store bytes as a legacy plaintext preferences map or an envelope.
fn classifyStoredEncryptedPreferences(
    content: &[u8],
) -> Result<StoredEncryptedPreferences, PreferencesDataStoreError> {
    let value: serde_json::Value = serde_json::from_slice(content)?;
    let isLegacyPlaintext = value
        .as_object()
        .map(|object| object.values().all(serde_json::Value::is_string))
        .unwrap_or(false);
    if isLegacyPlaintext {
        return Ok(StoredEncryptedPreferences::LegacyPlaintext(
            serde_json::from_value(value)?,
        ));
    }
    Ok(StoredEncryptedPreferences::EncryptedEnvelope(
        content.to_vec(),
    ))
}

/// Decodes one validated wire payload into its exact in-memory mutation.
#[allow(non_snake_case)]
fn decodePreferencesSyncedMutation(
    operation: &str,
    payload: &PreferencesSyncEntryPayload,
) -> Result<PreferencesSyncedMutation, PreferencesDataStoreError> {
    if !payload.jsonPath.is_empty() {
        if payload.value.is_some() {
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
        "set" => Ok(PreferencesSyncedMutation::SetEntry {
            key: payload.key.clone(),
            value: payload.value.clone().ok_or_else(|| {
                PreferencesDataStoreError::Message(
                    "preference set operation is missing its value".to_string(),
                )
            })?,
        }),
        "delete" => {
            if payload.value.is_some() {
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

/// Produces independently mergeable mutations for every changed JSON location.
#[allow(non_snake_case)]
fn structuredJsonMutations(
    previous: &Value,
    next: &Value,
) -> Vec<PreferencesStructuredJsonMutation> {
    let mut mutations = Vec::new();
    collectStructuredJsonMutations(previous, next, &mut Vec::new(), &mut mutations);
    mutations
}

/// Recursively compares JSON objects and stable-id arrays without relying on field-name patterns.
#[allow(non_snake_case)]
fn collectStructuredJsonMutations(
    previous: &Value,
    next: &Value,
    path: &mut Vec<PreferencesSyncJsonPathSegment>,
    mutations: &mut Vec<PreferencesStructuredJsonMutation>,
) {
    if previous == next {
        return;
    }
    match (previous, next) {
        (Value::Object(previousObject), Value::Object(nextObject)) => {
            let fields = previousObject
                .keys()
                .chain(nextObject.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            for field in fields {
                path.push(PreferencesSyncJsonPathSegment::Field(field.clone()));
                match (previousObject.get(&field), nextObject.get(&field)) {
                    (Some(previousValue), Some(nextValue)) => {
                        collectStructuredJsonMutations(previousValue, nextValue, path, mutations)
                    }
                    (None, Some(nextValue)) => mutations.push(PreferencesStructuredJsonMutation {
                        path: path.clone(),
                        value: Some(nextValue.clone()),
                    }),
                    (Some(_), None) => mutations.push(PreferencesStructuredJsonMutation {
                        path: path.clone(),
                        value: None,
                    }),
                    (None, None) => unreachable!("JSON field union must contain each field"),
                }
                path.pop();
            }
        }
        (Value::Array(previousArray), Value::Array(nextArray)) => {
            match (
                stableJsonItemsById(previousArray),
                stableJsonItemsById(nextArray),
            ) {
                (Some(previousItems), Some(nextItems)) => {
                    let itemIds = previousItems
                        .keys()
                        .chain(nextItems.keys())
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    for itemId in itemIds {
                        path.push(PreferencesSyncJsonPathSegment::Item(itemId.clone()));
                        match (previousItems.get(&itemId), nextItems.get(&itemId)) {
                            (Some(previousValue), Some(nextValue)) => {
                                collectStructuredJsonMutations(
                                    previousValue,
                                    nextValue,
                                    path,
                                    mutations,
                                )
                            }
                            (None, Some(nextValue)) => {
                                mutations.push(PreferencesStructuredJsonMutation {
                                    path: path.clone(),
                                    value: Some((*nextValue).clone()),
                                })
                            }
                            (Some(_), None) => mutations.push(PreferencesStructuredJsonMutation {
                                path: path.clone(),
                                value: None,
                            }),
                            (None, None) => {
                                unreachable!("JSON item union must contain each item")
                            }
                        }
                        path.pop();
                    }
                }
                _ => mutations.push(PreferencesStructuredJsonMutation {
                    path: path.clone(),
                    value: Some(next.clone()),
                }),
            }
        }
        _ => mutations.push(PreferencesStructuredJsonMutation {
            path: path.clone(),
            value: Some(next.clone()),
        }),
    }
}

/// Indexes a JSON array whose elements all expose distinct non-empty string ids.
#[allow(non_snake_case)]
fn stableJsonItemsById(values: &[Value]) -> Option<BTreeMap<String, &Value>> {
    let mut items = BTreeMap::new();
    for value in values {
        let id = value.as_object()?.get("id")?.as_str()?;
        if id.is_empty() || items.insert(id.to_string(), value).is_some() {
            return None;
        }
    }
    Some(items)
}

/// Applies one structured JSON mutation to an existing preference entry.
#[allow(non_snake_case)]
fn applyStructuredJsonPreferenceMutation(
    preferences: &mut Preferences,
    key: &PreferencesKey,
    path: &[PreferencesSyncJsonPathSegment],
    value: Option<Value>,
) -> Result<(), PreferencesDataStoreError> {
    if path.is_empty() {
        return Err(PreferencesDataStoreError::Message(
            "structured preference path must not be empty".to_string(),
        ));
    }
    let encoded = preferences.get(key).ok_or_else(|| {
        PreferencesDataStoreError::Message(format!(
            "structured preference entry is missing: {}",
            key.name
        ))
    })?;
    let mut root: Value = serde_json::from_str(encoded)?;
    applyStructuredJsonMutation(&mut root, path, value)?;
    preferences.set(key, serde_json::to_string(&root)?);
    Ok(())
}

/// Traverses a structured JSON path and applies its terminal set or delete operation.
#[allow(non_snake_case)]
fn applyStructuredJsonMutation(
    root: &mut Value,
    path: &[PreferencesSyncJsonPathSegment],
    value: Option<Value>,
) -> Result<(), PreferencesDataStoreError> {
    let (terminal, parents) = path
        .split_last()
        .expect("structured JSON path must contain a terminal segment");
    let mut current = root;
    for segment in parents {
        current = match segment {
            PreferencesSyncJsonPathSegment::Field(field) => current
                .as_object_mut()
                .and_then(|object| object.get_mut(field))
                .ok_or_else(|| {
                    PreferencesDataStoreError::Message(format!(
                        "structured JSON field is missing: {field}"
                    ))
                })?,
            PreferencesSyncJsonPathSegment::Item(itemId) => current
                .as_array_mut()
                .and_then(|items| {
                    items.iter_mut().find(|item| {
                        item.as_object()
                            .and_then(|object| object.get("id"))
                            .and_then(Value::as_str)
                            == Some(itemId.as_str())
                    })
                })
                .ok_or_else(|| {
                    PreferencesDataStoreError::Message(format!(
                        "structured JSON item is missing: {itemId}"
                    ))
                })?,
        };
    }
    match terminal {
        PreferencesSyncJsonPathSegment::Field(field) => {
            let object = current.as_object_mut().ok_or_else(|| {
                PreferencesDataStoreError::Message(
                    "structured JSON field parent is not an object".to_string(),
                )
            })?;
            match value {
                Some(value) => {
                    object.insert(field.clone(), value);
                }
                None => {
                    object.remove(field);
                }
            }
        }
        PreferencesSyncJsonPathSegment::Item(itemId) => {
            let items = current.as_array_mut().ok_or_else(|| {
                PreferencesDataStoreError::Message(
                    "structured JSON item parent is not an array".to_string(),
                )
            })?;
            let existingIndex = items.iter().position(|item| {
                item.as_object()
                    .and_then(|object| object.get("id"))
                    .and_then(Value::as_str)
                    == Some(itemId.as_str())
            });
            match (existingIndex, value) {
                (Some(index), Some(value)) => items[index] = value,
                (None, Some(value)) => {
                    let valueId = value
                        .as_object()
                        .and_then(|object| object.get("id"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            PreferencesDataStoreError::Message(
                                "structured JSON array item is missing its id".to_string(),
                            )
                        })?;
                    if valueId != itemId {
                        return Err(PreferencesDataStoreError::Message(
                            "structured JSON array item id does not match its path".to_string(),
                        ));
                    }
                    items.push(value);
                }
                (Some(index), None) => {
                    items.remove(index);
                }
                (None, None) => {}
            }
        }
    }
    Ok(())
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

#[allow(non_snake_case)]
/// Returns the preference cache shared by stores using the same storage host and path.
fn preferencesDataStoreSharedState(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    storagePath: &str,
) -> Arc<PreferencesDataStoreSharedState> {
    static SHARED_STATES: OnceLock<
        Mutex<HashMap<PreferencesDataStoreRegistryKey, Arc<PreferencesDataStoreSharedState>>>,
    > = OnceLock::new();
    let states = SHARED_STATES.get_or_init(|| Mutex::new(HashMap::new()));
    let key = preferencesDataStoreRegistryKey(storageHost, storagePath);
    let mut states = states
        .lock()
        .expect("PreferencesDataStore shared state registry mutex must not be poisoned");
    if let Some(state) = states.get(&key) {
        return Arc::clone(state);
    }
    let state = Arc::new(PreferencesDataStoreSharedState::default());
    states.insert(key, Arc::clone(&state));
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

#[cfg(test)]
#[path = "tests/PreferencesDataStoreTests.rs"]
mod PreferencesDataStoreTests;
