use std::fmt::Display;

pub struct Query<Entity> {
    loader: Box<dyn super::transaction::LoadEntity<Entity, Key = String, Error = String>>,
}

impl<Entity> Query<Entity>
where
    Entity: Default + Send + Sync,
{
    pub fn new<T>(loader: T) -> Self
    where
        T: super::transaction::LoadEntity<Entity, Key = String, Error = String> + 'static,
    {
        Self {
            loader: Box::new(loader),
        }
    }

    pub async fn get(&self, key: &String) -> Result<Entity, String> {
        let result = self.loader.load(key).await;
        result.map_err(|e| e.to_string())
    }
}
