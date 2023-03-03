use crate::store::SaveEntity;
use async_trait::async_trait;

#[async_trait]
pub trait SaveEvents<Event>: Send + Sync {
    type Key;
    type Error;
    async fn save(&self, key: &Self::Key, events: Vec<Event>) -> Result<Vec<Event>, Self::Error>;
}

#[async_trait]
impl<T, Event> SaveEvents<Event> for T
where
    T: SaveEntity<Vec<Event>>,
    T::Key: Send + Sync,
    T::Error: Send + Sync,
    Vec<Event>: Send + Sync,
    Event: Send + Sync + 'static,
{
    type Key = T::Key;
    type Error = T::Error;
    async fn save(&self, key: &Self::Key, events: Vec<Event>) -> Result<Vec<Event>, Self::Error> {
        self.save(key, events).await
    }
}

#[async_trait]
pub trait LoadEntity<Entity>: Send + Sync {
    type Key;
    type Error;
    async fn load(&self, key: &Self::Key) -> Result<Entity, Self::Error>;
}

#[async_trait]
impl<T, Entity> LoadEntity<Entity> for T
where
    T: super::store::LoadEntity<Entity>,
    T::Key: Send + Sync,
    Entity: Default + Send + Sync + 'static,
{
    type Key = T::Key;
    type Error = T::Error;
    async fn load(&self, key: &Self::Key) -> Result<Entity, Self::Error> {
        self.load(key)
            .await
            .map(|entity| entity.unwrap_or_default())
    }
}

#[async_trait]
pub trait EventTransactionStore: Send + Sync {
    type Entity: Send + Sync;
    type Event: Send + Sync;
    type Key: Send + Sync;
    type Error: Send + Sync;
    async fn perform_modification(
        &self,
        transaction: &dyn Transaction<Self::Entity, Self::Event, Self::Key, Self::Error>,
    ) -> Result<Vec<Self::Event>, Self::Error>;
}

#[async_trait]
pub trait Transaction<Entity, Event, Key, Error>: Send + Sync {
    async fn modify(
        &self,
        load_entity: &dyn LoadEntity<Entity, Key = Key, Error = Error>,
        save_events: &dyn SaveEvents<Event, Key = Key, Error = Error>,
    ) -> Result<Vec<Event>, Error>;
}
