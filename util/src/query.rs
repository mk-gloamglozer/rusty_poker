use crate::store::LoadEntity;
use crate::transaction::NormaliseTo;
use std::error::Error;

pub struct Query<T> {
    loader: Box<dyn LoadEntity<Vec<T>, Key = String, Error = Box<dyn Error + Send + Sync>>>,
}

impl<T> Query<T>
where
    T: Send + Sync,
{
    pub fn new<U>(loader: U) -> Self
    where
        U: LoadEntity<Vec<T>, Key = String, Error = Box<dyn Error + Send + Sync>> + 'static,
    {
        Self {
            loader: Box::new(loader),
        }
    }

    pub async fn query<Entity>(&self, key: &str) -> Result<Entity, Box<dyn Error + Send + Sync>>
    where
        Vec<T>: NormaliseTo<Entity> + Default,
    {
        match self.loader.load(&key.into()).await {
            Ok(entity) => Ok(entity.unwrap_or_default().render_normalised()),
            Err(e) => Err(e),
        }
    }
}
