use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

#[derive(Clone)]
pub struct Flow<T> {
    producer: Arc<dyn Fn() -> FlowResult<T> + Send + Sync>,
    waitChanged: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl<T> Flow<T> {
    pub fn new<F>(producer: F) -> Self
    where
        F: Fn() -> FlowResult<T> + Send + Sync + 'static,
    {
        Self {
            producer: Arc::new(producer),
            waitChanged: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn newObserved<F, W>(producer: F, waitChanged: W) -> Self
    where
        F: Fn() -> FlowResult<T> + Send + Sync + 'static,
        W: Fn() + Send + Sync + 'static,
    {
        Self {
            producer: Arc::new(producer),
            waitChanged: Some(Arc::new(waitChanged)),
        }
    }

    pub fn first(&self) -> FlowResult<T> {
        (self.producer)()
    }

    pub fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        collector(self.first()?);
        if let Some(waitChanged) = &self.waitChanged {
            loop {
                waitChanged();
                collector(self.first()?);
            }
        }
        Ok(())
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
        }
    }

    #[allow(non_snake_case)]
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
        }
    }

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
        }
    }

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
        if let Ok(value) = self.first() {
            stateFlow.set_value(value);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.waitChanged.is_some() {
            let flow = self.clone();
            let stateFlowForThread = stateFlow.clone();
            std::thread::spawn(move || {
                let _ = flow.collect(|value| {
                    stateFlowForThread.set_value(value);
                });
            });
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
            }),
        }
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

#[derive(Debug)]
struct PreferencesDataStoreChangeSignal {
    version: Mutex<u64>,
    changed: Condvar,
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
            syncOperationStore: Some(SyncOperationStore::native(RuntimeStorePaths::new(rootDir))),
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
        Flow::newObserved(
            move || store.data(),
            move || {
                let versionGuard = signal
                    .version
                    .lock()
                    .expect("PreferencesDataStore version mutex must not be poisoned");
                let observedVersion = *versionGuard;
                let _versionGuard = signal
                    .changed
                    .wait_while(versionGuard, |version| *version == observedVersion)
                    .expect("PreferencesDataStore version mutex must not be poisoned");
            },
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
    });
    signals.insert(path.to_path_buf(), Arc::downgrade(&signal));
    signal
}
