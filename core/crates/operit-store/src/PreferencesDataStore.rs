use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use operit_host_api::RuntimeStorageHost;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, runtimeStoragePath};
use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SyncOperationStore::{NewSyncOperation, SyncOperationStore, SyncOperationStoreError};

#[derive(Debug, Error)]
pub enum PreferencesDataStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("host error: {0}")]
    Host(#[from] operit_host_api::HostError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("sync operation store error: {0}")]
    Sync(#[from] SyncOperationStoreError),
    #[error("{0}")]
    Message(String),
}

pub type FlowResult<T> = Result<T, PreferencesDataStoreError>;

pub trait FlowLike<T>: Clone
where
    T: Clone,
{
    fn first(&self) -> FlowResult<T>;

    fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T);
}

pub trait StateFlowLike<T>: FlowLike<T>
where
    T: Clone,
{
    fn value(&self) -> T;
}

pub trait MutableStateFlowLike<T>: StateFlowLike<T>
where
    T: Clone + PartialEq,
{
    fn set_value(&self, value: T);

    fn compare_and_set(&self, expect: T, update: T) -> bool;
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

pub struct FlowObservationSubscription {
    _guard: Box<dyn FlowObservationGuard>,
}

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
    pub fn addCancelHook(&self, callback: impl Fn() + Send + Sync + 'static) -> FlowCancellationHook {
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
        if let Ok(subscription) =
            self.subscribeWithCancellation(cancellation, move |value| {
                stateFlowForSubscription.set_value(value);
            })
        {
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
pub struct CoroutineScope;

#[derive(Clone, Debug)]
pub enum SharingStarted {
    Lazily,
}

#[derive(Clone)]
pub struct StateFlow<T> {
    inner: Arc<StateFlowInner<T>>,
}

struct StateFlowInner<T> {
    value: Mutex<T>,
    version: Mutex<u64>,
    changed: Condvar,
    subscribers: Mutex<StateFlowSubscribers<T>>,
    upstreamSubscription: Mutex<Option<FlowSubscription>>,
}

struct StateFlowSubscribers<T> {
    nextId: usize,
    callbacks: HashMap<usize, Arc<Mutex<dyn FnMut(T) + Send>>>,
}

impl<T> StateFlow<T>
where
    T: Clone + PartialEq,
{
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

    pub fn value(&self) -> T {
        self.inner
            .value
            .lock()
            .expect("StateFlow value mutex must not be poisoned")
            .clone()
    }

    pub fn first(&self) -> FlowResult<T> {
        Ok(self.value())
    }

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

    pub fn unsubscribe(&self, subscriptionId: usize) {
        let mut subscribers = self
            .inner
            .subscribers
            .lock()
            .expect("StateFlow subscribers mutex must not be poisoned");
        subscribers.callbacks.remove(&subscriptionId);
    }

    pub fn map<U, F>(&self, transform: F) -> StateFlow<U>
    where
        T: Send + 'static,
        U: Clone + PartialEq + Send + 'static,
        F: Fn(T) -> U + Send + Sync + 'static,
    {
        let transform = Arc::new(transform);
        let stateFlow = StateFlow::new(transform(self.value()));
        let stateFlowForSubscriber = stateFlow.clone();
        let transformForSubscriber = Arc::clone(&transform);
        self.subscribe(move |value| {
            stateFlowForSubscriber.set_value(transformForSubscriber(value));
        });
        stateFlow
    }
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
pub struct MutableStateFlow<T> {
    state: StateFlow<T>,
}

impl<T> MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    pub fn new(initialValue: T) -> Self {
        Self {
            state: StateFlow::new(initialValue),
        }
    }

    #[allow(non_snake_case)]
    pub fn asStateFlow(&self) -> StateFlow<T> {
        self.state.clone()
    }

    pub fn value(&self) -> T {
        self.state.value()
    }

    pub fn first(&self) -> FlowResult<T> {
        Ok(self.value())
    }

    pub fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        self.state.collect(collector)
    }

    #[allow(non_snake_case)]
    pub fn collectUntil<F, P>(&self, collector: F, shouldStop: P) -> FlowResult<()>
    where
        F: FnMut(T),
        P: Fn(&T) -> bool,
    {
        self.state.collectUntil(collector, shouldStop)
    }

    pub fn set_value(&self, value: T) {
        self.state.set_value(value);
    }

    pub fn subscribe<F>(&self, subscriber: F) -> usize
    where
        F: FnMut(T) + Send + 'static,
    {
        self.state.subscribe(subscriber)
    }

    pub fn unsubscribe(&self, subscriptionId: usize) {
        self.state.unsubscribe(subscriptionId);
    }

    pub fn compare_and_set(&self, expect: T, update: T) -> bool {
        self.state.compare_and_set(expect, update)
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreferencesKey {
    pub name: String,
}

#[allow(non_snake_case)]
pub fn stringPreferencesKey(name: &str) -> PreferencesKey {
    PreferencesKey {
        name: name.to_string(),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Preferences {
    values: HashMap<String, String>,
}

impl Preferences {
    pub fn get(&self, key: &PreferencesKey) -> Option<&String> {
        self.values.get(&key.name)
    }

    pub fn set(&mut self, key: &PreferencesKey, value: String) {
        self.values.insert(key.name.clone(), value);
    }

    pub fn remove(&mut self, key: &PreferencesKey) {
        self.values.remove(&key.name);
    }

    pub fn contains(&self, key: &PreferencesKey) -> bool {
        self.values.contains_key(&key.name)
    }

    pub fn entries(&self) -> Vec<(String, String)> {
        self.values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

#[allow(non_snake_case)]
pub fn emptyPreferences() -> Preferences {
    Preferences::default()
}

#[allow(non_snake_case)]
pub fn mutableStateFlow<T>(initialValue: T) -> MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    MutableStateFlow::new(initialValue)
}

#[derive(Clone)]
pub struct PreferencesDataStore {
    path: PathBuf,
    storagePath: String,
    storageHost: Arc<dyn RuntimeStorageHost>,
    syncOperationStore: Option<SyncOperationStore>,
    syncDescriptor: Option<PreferencesSyncDescriptor>,
    changeSignal: Arc<PreferencesDataStoreChangeSignal>,
}

#[derive(Clone, Debug)]
pub struct PreferencesSyncDescriptor {
    pub domain: String,
    pub entityType: String,
    pub entityId: String,
    pub operation: String,
}

impl PreferencesSyncDescriptor {
    pub fn new(
        domain: impl Into<String>,
        entityType: impl Into<String>,
        entityId: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            domain: domain.into(),
            entityType: entityType.into(),
            entityId: entityId.into(),
            operation: operation.into(),
        }
    }

    #[allow(non_snake_case)]
    pub fn forPreferencesPath(path: &Path) -> Self {
        let fileName = path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let entityId = runtimeStoragePath(path);
        let entityType = fileName
            .trim_end_matches(".preferences.json")
            .trim_end_matches(".json")
            .to_string();
        Self::new("preferences", entityType, entityId, "upsert")
    }
}

struct PreferencesDataStoreChangeSignal {
    version: Mutex<u64>,
    changed: Condvar,
    subscribers: Mutex<PreferencesDataStoreFlowSubscribers>,
    dispatcherStarted: AtomicBool,
}

struct PreferencesDataStoreFlowSubscribers {
    nextId: usize,
    callbacks: HashMap<usize, Arc<dyn Fn() + Send + Sync>>,
}

struct PreferencesDataStoreFlowSubscription {
    signal: Weak<PreferencesDataStoreChangeSignal>,
    id: usize,
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
    pub fn new(path: PathBuf) -> Self {
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let rootDir = path
            .parent()
            .map(Path::to_path_buf)
            .expect("preferences path must include a parent directory");
        Self {
            storagePath: runtimeStoragePath(&path),
            storageHost: defaultRuntimeStorageHost(),
            syncOperationStore: Some(SyncOperationStore::adjacentTo(RuntimeStorePaths::new(
                rootDir,
            ))),
            syncDescriptor: Some(PreferencesSyncDescriptor::forPreferencesPath(&path)),
            path,
            changeSignal,
        }
    }

    #[allow(non_snake_case)]
    pub fn newWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        Self {
            path,
            storagePath,
            storageHost,
            syncOperationStore: None,
            syncDescriptor: None,
            changeSignal,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(non_snake_case)]
    pub fn applySyncedPreferences(
        entityId: &str,
        payload: serde_json::Value,
    ) -> Result<(), PreferencesDataStoreError> {
        let preferences: Preferences = serde_json::from_value(payload)?;
        let content = serde_json::to_string_pretty(&preferences)?;
        defaultRuntimeStorageHost().writeBytes(entityId, content.as_bytes())?;
        let path = RuntimeStorePaths::default().root_dir().join(entityId);
        let signal = preferencesDataStoreChangeSignal(&path);
        let mut version = signal
            .version
            .lock()
            .expect("PreferencesDataStore version mutex must not be poisoned");
        *version += 1;
        signal.changed.notify_all();
        Ok(())
    }

    pub fn data(&self) -> Result<Preferences, PreferencesDataStoreError> {
        if !self.storageHost.exists(&self.storagePath)? {
            return Ok(emptyPreferences());
        }
        let content = String::from_utf8(self.storageHost.readBytes(&self.storagePath)?)
            .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?;
        if content.trim().is_empty() {
            return Ok(emptyPreferences());
        }
        Ok(serde_json::from_str(&content)?)
    }

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

    pub fn edit<F>(&self, transform: F) -> Result<(), PreferencesDataStoreError>
    where
        F: FnOnce(&mut Preferences),
    {
        let mut preferences = self.data()?;
        transform(&mut preferences);
        self.write(&preferences)
    }

    pub fn edit_result<F, T>(&self, transform: F) -> Result<T, PreferencesDataStoreError>
    where
        F: FnOnce(&mut Preferences) -> T,
    {
        let mut preferences = self.data()?;
        let result = transform(&mut preferences);
        self.write(&preferences)?;
        Ok(result)
    }

    fn write(&self, preferences: &Preferences) -> Result<(), PreferencesDataStoreError> {
        let content = serde_json::to_string_pretty(preferences)?;
        self.storageHost
            .writeBytes(&self.storagePath, content.as_bytes())?;
        self.recordSyncOperation(preferences)?;
        self.notifyChanged();
        Ok(())
    }

    #[allow(non_snake_case)]
    fn recordSyncOperation(
        &self,
        preferences: &Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        let Some(syncOperationStore) = &self.syncOperationStore else {
            return Ok(());
        };
        let Some(descriptor) = &self.syncDescriptor else {
            return Ok(());
        };
        let deviceId = syncOperationStore.localDeviceId()?;
        syncOperationStore.appendLocalOperation(
            &deviceId,
            NewSyncOperation {
                domain: descriptor.domain.clone(),
                entityType: descriptor.entityType.clone(),
                entityId: descriptor.entityId.clone(),
                operation: descriptor.operation.clone(),
                payload: serde_json::to_value(preferences)?,
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
    }
}

#[allow(non_snake_case)]
fn preferencesDataStoreChangeSignal(path: &Path) -> Arc<PreferencesDataStoreChangeSignal> {
    static CHANGE_SIGNALS: OnceLock<
        Mutex<HashMap<PathBuf, Weak<PreferencesDataStoreChangeSignal>>>,
    > = OnceLock::new();
    let signals = CHANGE_SIGNALS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut signals = signals
        .lock()
        .expect("PreferencesDataStore change signal registry mutex must not be poisoned");
    if let Some(signal) = signals.get(path).and_then(Weak::upgrade) {
        return signal;
    }
    let signal = Arc::new(PreferencesDataStoreChangeSignal {
        version: Mutex::new(0),
        changed: Condvar::new(),
        subscribers: Mutex::new(PreferencesDataStoreFlowSubscribers {
            nextId: 0,
            callbacks: HashMap::new(),
        }),
        dispatcherStarted: AtomicBool::new(false),
    });
    signals.insert(path.to_path_buf(), Arc::downgrade(&signal));
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
            startPreferencesDataStoreFlowDispatcher(Arc::clone(&signal));
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
fn startPreferencesDataStoreFlowDispatcher(signal: Arc<PreferencesDataStoreChangeSignal>) {
    if signal.dispatcherStarted.swap(true, Ordering::SeqCst) {
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        let mut observedVersion = *signal
            .version
            .lock()
            .expect("PreferencesDataStore version mutex must not be poisoned");

        loop {
            let mut versionGuard = signal
                .version
                .lock()
                .expect("PreferencesDataStore version mutex must not be poisoned");
            while *versionGuard == observedVersion
                && !preferencesDataStoreFlowSubscribersEmpty(&signal)
            {
                versionGuard = signal
                    .changed
                    .wait(versionGuard)
                    .expect("PreferencesDataStore version mutex must not be poisoned");
            }

            if preferencesDataStoreFlowDispatcherShouldExit(&signal) {
                return;
            }

            observedVersion = *versionGuard;
            drop(versionGuard);

            let callbacks = signal
                .subscribers
                .lock()
                .expect("PreferencesDataStore subscribers mutex must not be poisoned")
                .callbacks
                .values()
                .cloned()
                .collect::<Vec<_>>();
            for callback in callbacks {
                callback();
            }
        }
    });
}

#[allow(non_snake_case)]
fn preferencesDataStoreFlowSubscribersEmpty(
    signal: &PreferencesDataStoreChangeSignal,
) -> bool {
    signal
        .subscribers
        .lock()
        .expect("PreferencesDataStore subscribers mutex must not be poisoned")
        .callbacks
        .is_empty()
}

#[allow(non_snake_case)]
fn preferencesDataStoreFlowDispatcherShouldExit(
    signal: &PreferencesDataStoreChangeSignal,
) -> bool {
    let subscribers = signal
        .subscribers
        .lock()
        .expect("PreferencesDataStore subscribers mutex must not be poisoned");
    if subscribers.callbacks.is_empty() {
        signal.dispatcherStarted.store(false, Ordering::SeqCst);
        true
    } else {
        false
    }
}
