use crate::message::{LoadEvents, SaveEvents};
use actix::{Actor, Addr, AsyncContext, Handler, Message, MessageResponse};

use poker_board::command::event::BoardModifiedEvent;
use std::collections::HashMap;

use std::sync::Arc;

use util::store::{LoadEntity, SaveEntity};

struct EventUpdates {
    store: HashMap<String, Board>,
    self_address: Option<Addr<Self>>,
}

impl EventUpdates {
    fn new() -> Self {
        Self {
            store: HashMap::new(),
            self_address: None,
        }
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;

impl Actor for EventUpdates {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.self_address = Some(ctx.address());
    }
}

impl Handler<LoadEvents> for EventUpdates {
    type Result = Result<Option<Vec<BoardModifiedEvent>>, Error>;

    fn handle(&mut self, msg: LoadEvents, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.store.get(&msg.key).map(|b| b.events.clone()))
    }
}

impl Handler<SaveEvents> for EventUpdates {
    type Result = Result<Vec<BoardModifiedEvent>, Error>;

    fn handle(&mut self, msg: SaveEvents, _ctx: &mut Self::Context) -> Self::Result {
        self.store
            .entry(msg.key.clone())
            .or_insert(Board::new())
            .update_events(msg.event.clone());

        Ok(msg.event)
    }
}

impl Handler<WaitForEvents> for EventUpdates {
    type Result = Result<UpdateRequest, Error>;

    fn handle(&mut self, msg: WaitForEvents, _ctx: &mut Self::Context) -> Self::Result {
        self.store
            .entry(msg.key)
            .or_insert(Board::new())
            .get_update(msg.last_event)
    }
}

#[derive(Clone)]
pub struct StoreInterface {
    store_addr: Addr<EventUpdates>,
}

impl StoreInterface {
    fn new(store_addr: Addr<EventUpdates>) -> Self {
        Self { store_addr }
    }
}

#[async_trait::async_trait]
impl SaveEntity<Vec<BoardModifiedEvent>> for StoreInterface {
    type Key = String;
    type Error = Error;

    async fn save(
        &self,
        key: &Self::Key,
        entity: Vec<BoardModifiedEvent>,
    ) -> Result<Vec<BoardModifiedEvent>, Self::Error> {
        self.store_addr
            .send(SaveEvents {
                key: key.clone(),
                event: entity.clone(),
            })
            .await
            .unwrap_or_else(|e| Err(Box::new(e)))
    }
}

struct Board {
    events: Vec<BoardModifiedEvent>,
    update_senders: Vec<UpdateChannel>,
}

struct UpdateChannel {
    update_sender: tokio::sync::oneshot::Sender<Vec<BoardModifiedEvent>>,
    position: usize,
}

impl UpdateChannel {
    fn new(
        update_sender: tokio::sync::oneshot::Sender<Vec<BoardModifiedEvent>>,
        position: usize,
    ) -> Self {
        Self {
            update_sender,
            position,
        }
    }

    fn send(self, events: &[BoardModifiedEvent]) -> Result<(), Vec<BoardModifiedEvent>> {
        self.update_sender
            .send(events.iter().skip(self.position).cloned().collect())
    }
}

impl Board {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            update_senders: Vec::new(),
        }
    }

    fn update_events(&mut self, events: Vec<BoardModifiedEvent>) {
        self.events
            .extend(events.into_iter().skip(self.events.len()));
        self.update_senders.drain(..).for_each(|sender| {
            sender.send(&self.events).unwrap_or_else(|e| {
                for event in e {
                    log::info!("Event {} could not be sent, channel closed", event);
                }
            })
        });
    }

    fn get_update(&mut self, last_event: usize) -> Result<UpdateRequest, Error> {
        match self.events.len() {
            len if len > last_event => {
                let events = self.events.iter().skip(last_event).cloned().collect();
                Ok(UpdateRequest::Fulfilled(events))
            }
            len if len == last_event => {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                self.update_senders
                    .push(UpdateChannel::new(sender, last_event));
                Ok(UpdateRequest::Pending(receiver))
            }
            _ => {
                let err_msg = format!(
                    "Invalid event index {} events len {}",
                    last_event,
                    self.events.len()
                );
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err_msg,
                )))
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<UpdateRequest, Error>")]
#[derive(Debug, Clone)]
pub struct WaitForEvents {
    pub key: String,
    pub last_event: usize,
}

#[derive(MessageResponse)]
pub enum UpdateRequest {
    Pending(tokio::sync::oneshot::Receiver<Vec<BoardModifiedEvent>>),
    Fulfilled(Vec<BoardModifiedEvent>),
}

impl UpdateRequest {
    async fn get(self) -> Vec<BoardModifiedEvent> {
        match self {
            Self::Pending(receiver) => receiver.await.unwrap_or_else(|e| {
                log::error!("Error getting request {}", e);
                Vec::new()
            }),
            Self::Fulfilled(events) => events,
        }
    }
}

#[async_trait::async_trait]
pub trait LoadUpdate<T>: Send + Sync {
    type Key;
    type Error;

    async fn load_update(&self, key: &Self::Key, last_version: usize) -> Result<T, Self::Error>;
}

#[async_trait::async_trait]
impl<T, Key, Error> LoadUpdate<T> for Box<dyn LoadUpdate<T, Key = Key, Error = Error>>
where
    T: Send + Sync,
    Key: Send + Sync,
    Error: Send + Sync,
{
    type Key = Key;
    type Error = Error;

    async fn load_update(&self, key: &Self::Key, last_version: usize) -> Result<T, Self::Error> {
        self.as_ref().load_update(key, last_version).await
    }
}

#[async_trait::async_trait]
impl<T, Key, Error> LoadUpdate<T> for Arc<dyn LoadUpdate<T, Key = Key, Error = Error>>
where
    T: Send + Sync,
    Key: Send + Sync,
    Error: Send + Sync,
{
    type Key = Key;
    type Error = Error;

    async fn load_update(&self, key: &Self::Key, last_version: usize) -> Result<T, Self::Error> {
        self.as_ref().load_update(key, last_version).await
    }
}

#[async_trait::async_trait]
impl LoadUpdate<Vec<BoardModifiedEvent>> for StoreInterface {
    type Key = String;
    type Error = Error;

    async fn load_update(
        &self,
        key: &Self::Key,
        last_version: usize,
    ) -> Result<Vec<BoardModifiedEvent>, Self::Error> {
        Ok(self
            .store_addr
            .send(WaitForEvents {
                key: key.clone(),
                last_event: last_version,
            })
            .await
            .unwrap_or_else(|e| Err(Box::new(e)))?
            .get()
            .await)
    }
}

#[async_trait::async_trait]
impl LoadEntity<Vec<BoardModifiedEvent>> for StoreInterface {
    type Key = String;
    type Error = Error;

    async fn load(&self, key: &Self::Key) -> Result<Option<Vec<BoardModifiedEvent>>, Self::Error> {
        self.store_addr
            .send(LoadEvents { key: key.clone() })
            .await
            .unwrap_or_else(|e| Err(Box::new(e)))
    }
}

pub fn create_store() -> StoreInterface {
    let store_addr = EventUpdates::new().start();
    StoreInterface::new(store_addr)
}
