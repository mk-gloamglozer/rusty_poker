use async_trait::async_trait;

#[async_trait]
pub trait LoadEntity<Entity>: Send + Sync {
    type Key: Send + Sync + 'static;
    type Error: Send + Sync + 'static;
    async fn load(&self, key: &Self::Key) -> Result<Option<Entity>, Self::Error>;
}

#[async_trait]
pub trait SaveEntity<Entity>: Send + Sync {
    type Key: Send + Sync + 'static;
    type Error: Send + Sync + 'static;
    async fn save(&self, key: &Self::Key, entity: Entity) -> Result<Entity, Self::Error>;
}
