use crate::command::adapter::StoreError::CouldNotLockMutex;

use crate::command::event::{BoardModifiedEvent, CombinedEvent, VoteTypeEvent};

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

#[derive(Clone)]
pub struct DefaultStore<T> {
    default: Vec<T>,
}

impl<T> DefaultStore<T> {
    pub fn new(default: Vec<T>) -> Self {
        Self { default }
    }
}

#[async_trait]
impl<T> LoadEntity<Vec<T>> for DefaultStore<T>
where
    T: Send + Sync + 'static + Clone,
{
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(&self, _key: &Self::Key) -> Result<Option<Vec<T>>, Self::Error> {
        Ok(Some(self.default.clone()))
    }
}

#[async_trait]
impl<T> SaveEntity<Vec<T>> for DefaultStore<T>
where
    T: Send + Sync + 'static + Clone,
{
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn save(&self, _key: &Self::Key, entity: Vec<T>) -> Result<Vec<T>, Self::Error> {
        Ok(entity)
    }
}

pub struct CombinedEventStore {
    board_modified_load_store: Box<
        dyn LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Box<dyn Error + Send + Sync>>,
    >,
    vote_type_list_load_store:
        Box<dyn LoadEntity<Vec<VoteTypeEvent>, Key = String, Error = Box<dyn Error + Send + Sync>>>,
    board_modified_save_store: Box<
        dyn SaveEntity<Vec<BoardModifiedEvent>, Key = String, Error = Box<dyn Error + Send + Sync>>,
    >,
}

impl CombinedEventStore {
    pub fn new(
        board_modified_load_store: impl LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Box<dyn Error + Send + Sync>>
            + 'static,
        vote_type_list_load_store: impl LoadEntity<Vec<VoteTypeEvent>, Key = String, Error = Box<dyn Error + Send + Sync>>
            + 'static,
        board_modified_save_store: impl SaveEntity<Vec<BoardModifiedEvent>, Key = String, Error = Box<dyn Error + Send + Sync>>
            + 'static,
    ) -> Self {
        Self {
            board_modified_load_store: Box::new(board_modified_load_store),
            vote_type_list_load_store: Box::new(vote_type_list_load_store),
            board_modified_save_store: Box::new(board_modified_save_store),
        }
    }
}

#[async_trait]
impl LoadEntity<Vec<BoardModifiedEvent>> for CombinedEventStore {
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(&self, key: &Self::Key) -> Result<Option<Vec<BoardModifiedEvent>>, Self::Error> {
        self.board_modified_load_store.load(key).await
    }
}

#[async_trait]
impl LoadEntity<Vec<CombinedEvent>> for CombinedEventStore {
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(&self, key: &Self::Key) -> Result<Option<Vec<CombinedEvent>>, Self::Error> {
        let board_modified_events = self
            .board_modified_load_store
            .load_events::<CombinedEvent>(key)
            .await?;

        let mut vote_type_events = self
            .vote_type_list_load_store
            .load_events::<CombinedEvent>(key)
            .await?;

        vote_type_events.extend(board_modified_events);
        Ok(Some(vote_type_events))
    }
}

#[async_trait]
impl SaveEntity<Vec<CombinedEvent>> for CombinedEventStore {
    type Key = String;
    type Error = Box<dyn Error + Send + Sync>;

    async fn save(
        &self,
        key: &Self::Key,
        entity: Vec<CombinedEvent>,
    ) -> Result<Vec<CombinedEvent>, Self::Error> {
        let board_modified_events = entity
            .iter()
            .filter_map(|event| match event {
                CombinedEvent::BoardModifiedEvent(event) => Some(event.clone()),
                _ => None,
            })
            .collect();

        self.board_modified_save_store
            .save(key, board_modified_events)
            .await?;

        Ok(entity)
    }
}

#[async_trait]
trait LoadEvent<T> {
    async fn load_events<U>(&self, key: &str) -> Result<Vec<U>, Box<dyn Error + Send + Sync>>
    where
        U: From<T>;
}

#[async_trait]
impl<T> LoadEvent<T>
    for Box<dyn LoadEntity<Vec<T>, Key = String, Error = Box<dyn Error + Send + Sync>>>
{
    async fn load_events<U>(&self, key: &str) -> Result<Vec<U>, Box<dyn Error + Send + Sync>>
    where
        U: From<T>,
    {
        let result = self
            .load(&key.to_string())
            .await?
            .unwrap_or_default()
            .into_iter()
            .map(|event| event.into())
            .collect();

        Ok(result)
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
