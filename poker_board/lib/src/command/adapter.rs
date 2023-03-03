use crate::command::domain::Board;
use crate::command::event::BoardModifiedEvent;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use util::store::{LoadEntity, SaveEntity};
use util::transaction::{EventTransactionStore, Transaction};
use util::use_case::EventSourced;

pub struct Store {
    store: HashMap<String, Vec<BoardModifiedEvent>>,
}

impl Store {
    fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Vec<BoardModifiedEvent>> {
        self.store.get(key)
    }

    fn insert(&mut self, key: String, value: Vec<BoardModifiedEvent>) {
        self.store.insert(key, value);
    }
}

#[derive(Clone)]
struct MutexStore(Arc<Mutex<Store>>);

pub fn in_memory_store() -> impl LoadEntity<Board, Error = String, Key = String>
       + SaveEntity<Vec<BoardModifiedEvent>, Error = String, Key = String>
       + Send
       + Sync
       + Clone {
    MutexStore(Arc::new(Mutex::new(Store::new())))
}

#[async_trait]
impl LoadEntity<Board> for MutexStore {
    type Key = String;
    type Error = String;

    async fn load(&self, key: &Self::Key) -> Result<Option<Board>, Self::Error> {
        match self.0.lock().unwrap().get(key) {
            Some(events) => Ok(Some(Board::source(events))),
            None => Ok(None),
        }
    }
}

#[async_trait]
impl SaveEntity<Vec<BoardModifiedEvent>> for MutexStore {
    type Key = String;
    type Error = String;

    async fn save(
        &self,
        key: &Self::Key,
        entity: Vec<BoardModifiedEvent>,
    ) -> Result<Vec<BoardModifiedEvent>, Self::Error> {
        self.0.lock().unwrap().insert(key.clone(), entity.clone());
        Ok(entity)
    }
}

pub struct MemoryTransactionStore<Store, Entity, Event>(
    Store,
    std::marker::PhantomData<(Entity, Event)>,
);

#[async_trait]
impl<Store, Entity, Event, Key, Error> EventTransactionStore
    for MemoryTransactionStore<Store, Entity, Event>
where
    Store: util::transaction::LoadEntity<Entity, Key = Key, Error = Error>
        + util::transaction::SaveEvents<Event, Key = Key, Error = Error>
        + Send
        + Sync,
    Entity: Send + Sync + 'static,
    Event: Send + Sync + 'static,
    Key: Send + Sync + 'static,
    Error: Send + Sync + 'static,
{
    type Entity = Entity;
    type Event = Event;
    type Key = Key;
    type Error = Error;

    async fn perform_modification(
        &self,
        transaction: &dyn Transaction<Self::Entity, Self::Event, Self::Key, Self::Error>,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        transaction.modify(&self.0, &self.0).await
    }
}

pub fn basic_transaction_store<Store, Entity, Event, Key, Error>(
    store: Store,
) -> impl EventTransactionStore<Entity = Entity, Event = Event, Key = Key, Error = Error>
where
    Store: util::transaction::LoadEntity<Entity, Key = Key, Error = Error>
        + util::transaction::SaveEvents<Event, Key = Key, Error = Error>
        + Send
        + Sync,
    Entity: Send + Sync + 'static,
    Event: Send + Sync + 'static,
    Key: Send + Sync + 'static,
    Error: Send + Sync + 'static,
{
    MemoryTransactionStore(store, std::marker::PhantomData)
}
