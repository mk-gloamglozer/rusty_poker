use crate::event::BoardModifiedEvent;
use async_trait::async_trait;
use mockall::automock;
use std::ops::Deref;

#[derive(Debug, PartialEq, Clone)]
pub enum PortError {
    LoadError(LoadError),
    SaveError(SaveError),
}

impl From<LoadError> for PortError {
    fn from(error: LoadError) -> Self {
        Self::LoadError(error)
    }
}

impl From<SaveError> for PortError {
    fn from(error: SaveError) -> Self {
        Self::SaveError(error)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LoadError {
    ConnectionError,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SaveError {
    ConnectionError,
}

#[async_trait]
pub trait LoadEventsPort: Send + Sync {
    async fn load_events(
        &self,
        entity: &String,
    ) -> Result<Box<dyn PersistableEvent<BoardModifiedEvent>>, LoadError>;
}

#[async_trait]
pub trait PersistableEvent<T>: Send + Sync
where
    T: Send + Sync,
{
    async fn persist(&self) -> Result<(), SaveError>;
    fn events(&self) -> Vec<T>;
    fn with_events(self: Box<Self>, events: Vec<T>) -> Box<dyn PersistableEvent<T>>;
}
