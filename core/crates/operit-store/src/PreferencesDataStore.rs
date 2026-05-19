use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreferencesDataStoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
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
}

impl<T> Flow<T> {
    pub fn new<F>(producer: F) -> Self
    where
        F: Fn() -> FlowResult<T> + Send + Sync + 'static,
    {
        Self {
            producer: Arc::new(producer),
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
        Flow::new(move || producer().map(&transform))
    }

    pub fn catch<F>(&self, handler: F) -> Flow<T>
    where
        T: 'static,
        F: Fn(PreferencesDataStoreError) -> FlowResult<T> + Send + Sync + 'static,
    {
        let producer = Arc::clone(&self.producer);
        Flow::new(move || match producer() {
            Ok(value) => Ok(value),
            Err(error) => handler(error),
        })
    }

    pub fn stateIn(&self, _scope: CoroutineScope, _started: SharingStarted, initialValue: T) -> StateFlow<T>
    where
        T: Clone + Send + 'static,
    {
        StateFlow::new(self.clone(), initialValue)
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
    source: Flow<T>,
    value: Arc<Mutex<T>>,
}

impl<T> StateFlow<T>
where
    T: Clone,
{
    pub fn new(source: Flow<T>, initialValue: T) -> Self {
        let value = source.first().unwrap_or(initialValue);
        Self {
            source,
            value: Arc::new(Mutex::new(value)),
        }
    }

    pub fn value(&self) -> T {
        self.value
            .lock()
            .expect("StateFlow value mutex must not be poisoned")
            .clone()
    }

    pub fn first(&self) -> FlowResult<T> {
        let value = self.source.first()?;
        *self
            .value
            .lock()
            .expect("StateFlow value mutex must not be poisoned") = value.clone();
        Ok(value)
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
        let value = self.first()?;
        collector(value);
        Ok(())
    }
}

impl<T> FlowLike<T> for StateFlow<T>
where
    T: Clone,
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
    T: Clone,
{
    fn value(&self) -> T {
        StateFlow::value(self)
    }
}

#[derive(Clone)]
pub struct MutableStateFlow<T> {
    value: Arc<Mutex<T>>,
}

impl<T> MutableStateFlow<T>
where
    T: Clone + PartialEq,
{
    pub fn new(initialValue: T) -> Self {
        Self {
            value: Arc::new(Mutex::new(initialValue)),
        }
    }

    pub fn value(&self) -> T {
        self.value
            .lock()
            .expect("MutableStateFlow value mutex must not be poisoned")
            .clone()
    }

    pub fn first(&self) -> FlowResult<T> {
        Ok(self.value())
    }

    pub fn collect<F>(&self, collector: F) -> FlowResult<()>
    where
        F: Fn(T),
    {
        collector(self.value());
        Ok(())
    }

    pub fn set_value(&self, value: T) {
        *self
            .value
            .lock()
            .expect("MutableStateFlow value mutex must not be poisoned") = value;
    }

    pub fn compare_and_set(&self, expect: T, update: T) -> bool {
        let mut guard = self
            .value
            .lock()
            .expect("MutableStateFlow value mutex must not be poisoned");
        if *guard == expect {
            *guard = update;
            true
        } else {
            false
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

#[derive(Clone, Debug)]
pub struct PreferencesDataStore {
    path: PathBuf,
}

impl PreferencesDataStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn data(&self) -> Result<Preferences, PreferencesDataStoreError> {
        if !self.path.exists() {
            return Ok(emptyPreferences());
        }
        let content = fs::read_to_string(&self.path)?;
        if content.trim().is_empty() {
            return Ok(emptyPreferences());
        }
        Ok(serde_json::from_str(&content)?)
    }

    pub fn dataFlow(&self) -> Flow<Preferences> {
        let store = self.clone();
        Flow::new(move || store.data())
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
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(preferences)?;
        fs::write(&self.path, content)?;
        Ok(())
    }
}
