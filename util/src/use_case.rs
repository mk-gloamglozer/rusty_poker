use crate::command::Command;
use crate::transaction::{EventTransactionStore, Transaction};
use std::fmt::Display;

pub trait EventSourced {
    type Event;
    fn source(events: &Vec<Self::Event>) -> Self;
}

pub trait HandleEvent {
    type Event;
    fn apply(&mut self, event: &Self::Event);
}

impl<T, E> EventSourced for T
where
    T: HandleEvent<Event = E> + Default,
{
    type Event = E;

    fn source(events: &Vec<Self::Event>) -> Self {
        let mut state = Self::default();
        for event in events {
            state.apply(event);
        }
        state
    }
}

pub struct UseCase<Event, Entity, Key, Error> {
    transaction:
        Box<dyn EventTransactionStore<Event = Event, Entity = Entity, Key = Key, Error = Error>>,
}

impl<Event, Entity, Key, Error> UseCase<Event, Entity, Key, Error>
where
    Key: Send + Sync,
    Entity: EventSourced<Event = Event> + Default + Send + Sync,
    Event: Send + Sync,
    Error: Display + Send + Sync,
{
    pub fn new<T>(transaction: T) -> Self
    where
        T: EventTransactionStore<Event = Event, Entity = Entity, Key = Key, Error = Error>
            + 'static,
    {
        Self {
            transaction: Box::new(transaction),
        }
    }

    pub async fn execute<Cmd>(&self, command: &Cmd, entity_id: &Key) -> Result<Vec<Event>, String>
    where
        Cmd: Command<Entity = Entity, Event = Event> + Send + Sync,
        Cmd::Entity: EventSourced<Event = Event>,
    {
        let command = UseCaseCommand::new(command, entity_id);
        let result = self.transaction.perform_modification(&command).await;
        result.map_err(|e| e.to_string())
    }
}

struct UseCaseCommand<Command, Key> {
    command: Command,
    key: Key,
}

impl<Command, Key> UseCaseCommand<Command, Key> {
    pub fn new(command: Command, key: Key) -> Self {
        Self { command, key }
    }
}

#[async_trait::async_trait]
impl<Entity, Event, Command, Key, Error> Transaction<Entity, Event, Key, Error>
    for UseCaseCommand<&Command, &Key>
where
    Entity: EventSourced<Event = Event> + Send + Sync,
    Error: Send + Sync,
    Event: Send + Sync,
    Key: Send + Sync,
    Command: self::Command<Entity = Entity, Event = Event> + Send + Sync,
{
    async fn modify(
        &self,
        load_entity: &dyn super::transaction::LoadEntity<Entity, Key = Key, Error = Error>,
        save_events: &dyn super::transaction::SaveEvents<Event, Key = Key, Error = Error>,
    ) -> Result<Vec<Event>, Error> {
        let entity = load_entity.load(&self.key).await?;
        let events = self.command.apply(entity);
        save_events.save(&self.key, events).await
    }
}
