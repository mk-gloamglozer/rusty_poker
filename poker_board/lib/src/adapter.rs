use crate::event::BoardModifiedEvent;
use crate::port::{Attempt, ModifyEntityPort, ModifyError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use util::store::{EventStore, EventStreamModifier};

pub struct Store {
    store: HashMap<String, Vec<BoardModifiedEvent>>,
}

type StoreMutex = Arc<Mutex<Store>>;

pub fn mutex_store() -> StoreMutex {
    Arc::new(Mutex::new(Store::new()))
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

pub struct AdapterConfig {
    store: StoreMutex,
    try_times: u8,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            store: mutex_store(),
            try_times: 3,
        }
    }
}

impl AdapterConfig {
    pub fn with_store(mut self, store: StoreMutex) -> Self {
        self.store = store;
        self
    }

    pub fn with_try_times(mut self, try_times: u8) -> Self {
        self.try_times = try_times;
        self
    }
}

pub struct InMemoryModifyEntityAdapter {
    store: Arc<Mutex<Store>>,
    try_times: u8,
}

impl Default for InMemoryModifyEntityAdapter {
    fn default() -> Self {
        Self::new(AdapterConfig::default())
    }
}

impl InMemoryModifyEntityAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            store: config.store,
            try_times: config.try_times,
        }
    }

    /**
     * This is a naive implementation of a retry mechanism.
     * It will try to lock the store for a given number of times.
     * If it fails to lock the store, it will return a ConnectionError.
     * If it fails to modify the store, it will return an UnableToCompleteError.
     * If it fails to modify the store because the event log has changed, it will return an EventLogChangedError.
     * If it succeeds to modify the store, it will return Ok(()).
     */
    fn _modify(
        &self,
        entity: &String,
        modify: &dyn EventStreamModifier<BoardModifiedEvent>,
        count: u8,
    ) -> Result<Vec<BoardModifiedEvent>, ModifyError> {
        match self.store.clone().lock() {
            Ok(mut store) => {
                let events = store.get(&entity).unwrap_or(&vec![]).clone();
                let updated_events = modify.modify(events.clone());
                for i in 0..events.len() {
                    if updated_events.get(i) != events.get(i) {
                        return Err(ModifyError::EventLogChangedError {
                            original: events.clone(),
                            actual: updated_events.clone(),
                        });
                    }
                }
                store.insert(entity.clone(), updated_events.clone());
                Ok(updated_events)
            }
            Err(_) => {
                if count < self.try_times {
                    return self._modify(entity, modify, count + 1);
                }
                return Err(ModifyError::ConnectionError(
                    "Unable to lock store".to_string(),
                ));
            }
        }
    }
}

#[async_trait]
impl EventStore for InMemoryModifyEntityAdapter {
    type Event = BoardModifiedEvent;
    type Error = ModifyError;
    type Key = String;

    async fn modify(
        &self,
        key: &Self::Key,
        event: &dyn EventStreamModifier<Self::Event>,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        self._modify(key, event, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::add_participant::AddParticipantCommand;
    use crate::domain::clear_votes::ClearVotes;
    use mockall::{mock, predicate};

    #[tokio::test]
    pub async fn it_should_persist_changed_events() {
        let mut store = Arc::new(Mutex::new(Store::new()));
        store.lock().unwrap().insert(
            "test-id".to_string(),
            vec![BoardModifiedEvent::VotesCleared],
        );

        let config = AdapterConfig::default().with_store(store.clone());
        let in_memory_modify_entity_adapter = InMemoryModifyEntityAdapter::new(config);

        let id = "test-id".to_string();

        let participant_name = "participant_name".to_string();
        let add_participant_command = AddParticipantCommand::new(participant_name.clone());

        let map_fn = |events: Vec<BoardModifiedEvent>| {
            let mut events = events.clone();
            events.push(BoardModifiedEvent::ParticipantAdded {
                participant_id: "test-id".to_string(),
                participant_name: participant_name.clone(),
            });
            events
        };

        in_memory_modify_entity_adapter
            .modify(&id, &map_fn)
            .await
            .unwrap();

        assert_eq!(store.lock().unwrap().get(&id).unwrap().len(), 2);
    }

    #[tokio::test]
    pub async fn it_should_return_an_error_if_event_log_was_changed() {
        let mut store = Arc::new(Mutex::new(Store::new()));
        store.lock().unwrap().insert(
            "test-id".to_string(),
            vec![BoardModifiedEvent::VotesCleared],
        );

        let config = AdapterConfig::default().with_store(store.clone());
        let in_memory_modify_entity_adapter = InMemoryModifyEntityAdapter::new(config);

        let id = "test-id".to_string();

        let participant_name = "participant_name".to_string();
        let add_participant_command = AddParticipantCommand::new(participant_name.clone());

        let map_fn = |events: Vec<BoardModifiedEvent>| {
            vec![BoardModifiedEvent::ParticipantAdded {
                participant_id: "test-id".to_string(),
                participant_name: participant_name.clone(),
            }]
        };

        let err = in_memory_modify_entity_adapter
            .modify(&id, &map_fn)
            .await
            .unwrap_err();

        assert_eq!(
            err,
            ModifyError::EventLogChangedError {
                original: vec![BoardModifiedEvent::VotesCleared],
                actual: vec![BoardModifiedEvent::ParticipantAdded {
                    participant_id: "test-id".to_string(),
                    participant_name: "participant_name".to_string()
                }]
            }
        );
    }

    #[tokio::test]
    pub async fn it_should_return_an_error_if_event_log_size_was_reduced() {
        let store = Arc::new(Mutex::new(Store::new()));
        store.lock().unwrap().insert(
            "test-id".to_string(),
            vec![BoardModifiedEvent::VotesCleared],
        );

        let config = AdapterConfig::default().with_store(store.clone());
        let in_memory_modify_entity_adapter = InMemoryModifyEntityAdapter::new(config);

        let id = "test-id".to_string();

        let map_fn = |_: Vec<BoardModifiedEvent>| vec![];

        let err = in_memory_modify_entity_adapter
            .modify(&id, &map_fn)
            .await
            .unwrap_err();

        assert_eq!(
            err,
            ModifyError::EventLogChangedError {
                original: vec![BoardModifiedEvent::VotesCleared],
                actual: vec![]
            }
        );
    }
}
