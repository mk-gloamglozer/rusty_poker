use crate::command::Command;
use crate::store::EventStore;
use serde::Deserialize;

pub trait EventSourced {
    type Event;
    fn source(events: Vec<Self::Event>) -> Self;
}

pub trait HandleEvent {
    type Event;
    fn apply(&mut self, event: Self::Event);
}

impl<T, E> EventSourced for T
where
    T: HandleEvent<Event = E> + Default,
{
    type Event = E;

    fn source(events: Vec<Self::Event>) -> Self {
        let mut state = Self::default();
        for event in events {
            state.apply(event);
        }
        state
    }
}

pub trait ResponseHandler<Event>: Send + Sync {
    fn handle(&self, events: Vec<Event>) -> Result<(), String>;
}

impl<T, Event> ResponseHandler<Event> for T
where
    T: Fn(Vec<Event>) -> Result<(), String> + Send + Sync + 'static,
{
    fn handle(&self, events: Vec<Event>) -> Result<(), String> {
        (self)(events)
    }
}

pub struct Handler<'a, Event, Error> {
    store: Box<dyn EventStore<Key = String, Event = Event, Error = Error> + 'a>,
    response_handler: Box<dyn ResponseHandler<Event> + 'a>,
}

impl<'a, Event, Error> Handler<'a, Event, Error> {
    pub fn new<Store, RHandler>(store: Store, response_handler: RHandler) -> Self
    where
        Store: EventStore<Key = String, Event = Event, Error = Error> + 'a,
        RHandler: ResponseHandler<Event> + 'a,
    {
        Self {
            store: Box::new(store),
            response_handler: Box::new(response_handler),
        }
    }
}

impl<'a, Event, Error> Handler<'a, Event, Error>
where
    Error: std::fmt::Display,
{
    pub async fn execute<Cmd>(&self, command: &Cmd, entity_id: &String) -> Result<(), String>
    where
        Cmd: Command<Event = Event> + Send + Sync,
        Cmd::Entity: EventSourced<Event = Event>,
    {
        self.store
            .modify(entity_id, &|events: Vec<Event>| -> Vec<Event> {
                let entity = Cmd::Entity::source(events);
                command.apply(entity)
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|events| self.response_handler.handle(events))
    }
}
