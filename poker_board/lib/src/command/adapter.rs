use crate::command::adapter::StoreError::CouldNotLockMutex;
use crate::command::event::BoardModifiedEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use util::store::{LoadEntity, SaveEntity};
use util::transaction::retry::{Instruction, RetryStrategy};

struct Store<T> {
    store: HashMap<String, Vec<T>>,
}

impl<T> Store<T> {
    fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Vec<T>> {
        self.store.get(key)
    }

    fn insert(&mut self, key: &String, value: Vec<T>) {
        self.store.insert(key.to_string(), value);
    }
}

#[derive(Clone)]
pub struct ArcMutexStore<T>(Arc<Mutex<Store<T>>>);

impl<T> ArcMutexStore<T> {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Store::new())))
    }
}

impl<T> Default for ArcMutexStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum StoreError {
    CouldNotLockMutex,
}

impl Error for StoreError {}

impl Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::CouldNotLockMutex => write!(f, "Could not lock mutex"),
        }
    }
}

#[async_trait]
impl<T> LoadEntity<Vec<T>> for ArcMutexStore<T>
where
    T: Send + Sync + 'static + Clone,
{
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(&self, key: &Self::Key) -> Result<Option<Vec<T>>, Self::Error> {
        match self.0.lock() {
            Ok(guard) => Ok(guard.get(key).cloned()),
            Err(_) => Err(CouldNotLockMutex.into()),
        }
    }
}

#[async_trait]
impl<T> SaveEntity<Vec<T>> for ArcMutexStore<T>
where
    T: Send + Sync + 'static + Clone,
{
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn save(&self, key: &Self::Key, entity: Vec<T>) -> Result<Vec<T>, Self::Error> {
        match self.0.lock() {
            Ok(mut guard) => {
                guard.insert(key, entity.clone());
                Ok(entity)
            }
            Err(_) => Err(CouldNotLockMutex.into()),
        }
    }
}

pub struct NoRetry;

impl NoRetry {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoRetry {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryStrategy for NoRetry {
    fn should_retry(
        &self,
        _previous_instruction: &Option<Instruction>,
        _retry_count: &u8,
    ) -> Instruction {
        Instruction::Abort
    }
}
