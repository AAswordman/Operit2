use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{
    MODEL_CONFIGS_PREFERENCES_PATH, TTS_CONFIGS_PREFERENCES_PATH,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::PreferencesEncryption::PreferencesEncryption;
use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, runtimeStoragePath};
use crate::RuntimeStorePaths::RuntimeStorePaths;
use crate::SyncOperationStore::{NewSyncOperation, SyncOperationStore, SyncOperationStoreError};

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
    #[error("{0}")]
    Message(String),
}

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
/// In-memory representation of a preferences file.
pub struct Preferences {
    values: HashMap<String, String>,
}

impl Preferences {
    /// Reads a preference value by key.
    pub fn get(&self, key: &PreferencesKey) -> Option<&String> {
        self.values.get(&key.name)
    }

    /// Writes a preference value by key.
    pub fn set(&mut self, key: &PreferencesKey, value: String) {
        self.values.insert(key.name.clone(), value);
    }

    /// Removes a preference value by key.
    pub fn remove(&mut self, key: &PreferencesKey) {
        self.values.remove(&key.name);
    }

    /// Returns whether a value exists for the supplied key.
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
}

#[derive(Clone, Debug)]
/// Sync metadata describing how a preferences file participates in runtime sync.
pub struct PreferencesSyncDescriptor {
    pub domain: String,
    pub entityType: String,
    pub entityId: String,
    pub operation: String,
}

impl PreferencesSyncDescriptor {
    /// Creates an explicit preferences sync descriptor.
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
    /// Builds the standard sync descriptor for a preferences file path.
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
        Self::new("preferences", entityType, storagePath.to_string(), "upsert")
    }
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
    preferences: Mutex<PreferencesDataStoreLoadedPreferences>,
}

#[derive(Default)]
struct PreferencesDataStoreLoadedPreferences {
    loaded: bool,
    preferences: Preferences,
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
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        Self {
            storagePath: runtimeStoragePath(&path),
            storageHost: defaultRuntimeStorageHost(),
            encryption: None,
            syncOperationStore: Some(SyncOperationStore::native(RuntimeStorePaths::default())),
            syncDescriptor: Some(PreferencesSyncDescriptor::forPreferencesPath(&path)),
            path,
            changeSignal,
            sharedState,
        }
    }

    #[allow(non_snake_case)]
    /// Creates an encrypted and synced preferences store backed by the default runtime storage host.
    pub fn newEncryptedSynced(path: PathBuf) -> Self {
        let storageHost = defaultRuntimeStorageHost();
        let storagePath = runtimeStoragePath(&path);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        let encryption = PreferencesEncryption::load_or_create(storageHost.as_ref())
            .expect("preferences encryption key must be available");
        Self {
            path: path.clone(),
            storagePath,
            storageHost,
            encryption: Some(encryption),
            syncOperationStore: Some(SyncOperationStore::native(RuntimeStorePaths::default())),
            syncDescriptor: Some(PreferencesSyncDescriptor::forPreferencesPath(&path)),
            changeSignal,
            sharedState,
        }
    }

    #[allow(non_snake_case)]
    /// Creates an encrypted preferences store backed by the default runtime storage host.
    pub fn newEncrypted(path: PathBuf) -> Self {
        let storageHost = defaultRuntimeStorageHost();
        let storagePath = runtimeStoragePath(&path);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        let encryption = PreferencesEncryption::load_or_create(storageHost.as_ref())
            .expect("preferences encryption key must be available");
        Self {
            path,
            storagePath,
            storageHost,
            encryption: Some(encryption),
            syncOperationStore: None,
            syncDescriptor: None,
            changeSignal,
            sharedState,
        }
    }

    #[allow(non_snake_case)]
    /// Creates a preferences store backed by an explicit storage host and path.
    pub fn newWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        Self {
            path,
            storagePath,
            storageHost,
            encryption: None,
            syncOperationStore: None,
            syncDescriptor: None,
            changeSignal,
            sharedState,
        }
    }

    #[allow(non_snake_case)]
    /// Creates a synced preferences store backed by an explicit storage host and path.
    pub fn newSyncedWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
        syncRootPath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        let syncDescriptor = PreferencesSyncDescriptor::forStoragePath(&storagePath);
        Self {
            path: path.clone(),
            storagePath,
            storageHost: storageHost.clone(),
            encryption: None,
            syncOperationStore: Some(SyncOperationStore::new(storageHost, syncRootPath)),
            syncDescriptor: Some(syncDescriptor),
            changeSignal,
            sharedState,
        }
    }

    #[allow(non_snake_case)]
    /// Creates an encrypted preferences store backed by an explicit storage host and path.
    pub fn newEncryptedWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        let encryption = PreferencesEncryption::load_or_create(storageHost.as_ref())
            .expect("preferences encryption key must be available");
        Self {
            path,
            storagePath,
            storageHost,
            encryption: Some(encryption),
            syncOperationStore: None,
            syncDescriptor: None,
            changeSignal,
            sharedState,
        }
    }

    #[allow(non_snake_case)]
    /// Creates an encrypted and synced preferences store backed by an explicit storage host and path.
    pub fn newEncryptedSyncedWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        storagePath: impl Into<String>,
        syncRootPath: impl Into<String>,
    ) -> Self {
        let storagePath = storagePath.into();
        let path = PathBuf::from(&storagePath);
        let changeSignal = preferencesDataStoreChangeSignal(&path);
        let sharedState = preferencesDataStoreSharedState(&path);
        let syncDescriptor = PreferencesSyncDescriptor::forStoragePath(&storagePath);
        let encryption = PreferencesEncryption::load_or_create(storageHost.as_ref())
            .expect("preferences encryption key must be available");
        Self {
            path: path.clone(),
            storagePath,
            storageHost: storageHost.clone(),
            encryption: Some(encryption),
            syncOperationStore: Some(SyncOperationStore::new(storageHost, syncRootPath)),
            syncDescriptor: Some(syncDescriptor),
            changeSignal,
            sharedState,
        }
    }

    /// Returns the logical path associated with this preferences store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(non_snake_case)]
    /// Applies a synced preferences payload to local storage and observers.
    pub fn applySyncedPreferences(
        entityId: &str,
        payload: serde_json::Value,
    ) -> Result<(), PreferencesDataStoreError> {
        Self::applySyncedPreferencesWithStorage(defaultRuntimeStorageHost(), entityId, payload)
    }

    #[allow(non_snake_case)]
    /// Applies a synced preferences payload with an explicit storage host.
    pub fn applySyncedPreferencesWithStorage(
        storageHost: Arc<dyn RuntimeStorageHost>,
        entityId: &str,
        payload: serde_json::Value,
    ) -> Result<(), PreferencesDataStoreError> {
        let preferences: Preferences = serde_json::from_value(payload)?;
        let content = serde_json::to_string_pretty(&preferences)?;
        let contentBytes = preferencesSyncStorageContent(storageHost.as_ref(), entityId, content)?;
        storageHost.writeBytes(entityId, &contentBytes)?;
        let path = RuntimeStorePaths::default().runtime_storage_path(entityId);
        {
            let sharedState = preferencesDataStoreSharedState(&path);
            let mut loaded = sharedState
                .preferences
                .lock()
                .expect("PreferencesDataStore shared state mutex must not be poisoned");
            loaded.loaded = true;
            loaded.preferences = preferences;
        }
        let signal = preferencesDataStoreChangeSignal(&path);
        let mut version = signal
            .version
            .lock()
            .expect("PreferencesDataStore version mutex must not be poisoned");
        *version += 1;
        signal.changed.notify_all();
        drop(version);
        notifyPreferencesDataStoreSubscribers(&signal);
        Ok(())
    }

    /// Reads the current preferences snapshot.
    pub fn data(&self) -> Result<Preferences, PreferencesDataStoreError> {
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        if !loaded.loaded {
            loaded.preferences = self.readFromStorage()?;
            loaded.loaded = true;
        }
        Ok(loaded.preferences.clone())
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
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        if !loaded.loaded {
            loaded.preferences = self.readFromStorage()?;
            loaded.loaded = true;
        }
        let mut preferences = loaded.preferences.clone();
        let result = transform(&mut preferences)?;
        self.writeUnlocked(&preferences)?;
        loaded.preferences = preferences;
        Ok(result)
    }

    /// Replaces the full preferences snapshot and notifies observers.
    pub fn replace(&self, preferences: Preferences) -> Result<(), PreferencesDataStoreError> {
        let mut loaded = self
            .sharedState
            .preferences
            .lock()
            .expect("PreferencesDataStore shared state mutex must not be poisoned");
        self.writeUnlocked(&preferences)?;
        loaded.loaded = true;
        loaded.preferences = preferences;
        Ok(())
    }

    fn writeUnlocked(&self, preferences: &Preferences) -> Result<(), PreferencesDataStoreError> {
        let content = serde_json::to_string_pretty(preferences)?;
        let storedContent = match &self.encryption {
            Some(encryption) => encryption.encrypt(&self.storagePath, content.as_bytes())?,
            None => content.into_bytes(),
        };
        self.storageHost
            .writeBytes(&self.storagePath, &storedContent)?;
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

#[allow(non_snake_case)]
/// Returns stored bytes for a synced preferences payload.
fn preferencesSyncStorageContent(
    storageHost: &dyn RuntimeStorageHost,
    storagePath: &str,
    content: String,
) -> Result<Vec<u8>, PreferencesDataStoreError> {
    if encryptedSyncedPreferencesPath(storagePath) {
        let encryption = PreferencesEncryption::load_or_create(storageHost)
            .expect("preferences encryption key must be available");
        encryption.encrypt(storagePath, content.as_bytes())
    } else {
        Ok(content.into_bytes())
    }
}

#[allow(non_snake_case)]
/// Reports whether a synced preferences path is stored encrypted.
fn encryptedSyncedPreferencesPath(storagePath: &str) -> bool {
    storagePath == MODEL_CONFIGS_PREFERENCES_PATH || storagePath == TTS_CONFIGS_PREFERENCES_PATH
}

#[allow(non_snake_case)]
fn preferencesDataStoreSharedState(path: &Path) -> Arc<PreferencesDataStoreSharedState> {
    static SHARED_STATES: OnceLock<Mutex<HashMap<PathBuf, Weak<PreferencesDataStoreSharedState>>>> =
        OnceLock::new();
    let states = SHARED_STATES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut states = states
        .lock()
        .expect("PreferencesDataStore shared state registry mutex must not be poisoned");
    if let Some(state) = states.get(path).and_then(Weak::upgrade) {
        return state;
    }
    let state = Arc::new(PreferencesDataStoreSharedState::default());
    states.insert(path.to_path_buf(), Arc::downgrade(&state));
    state
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
