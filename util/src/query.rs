use crate::store::LoadEntity;
use crate::transaction::NormaliseTo;
use std::error::Error;

pub trait PresentationOf {
    type Model;
    fn from_model(model: &Self::Model) -> Self;
}

pub trait PresentAs<T> {
    fn present_as(&self) -> T;
}

impl<T, U> PresentAs<U> for T
where
    U: PresentationOf<Model = T>,
{
    fn present_as(&self) -> U {
        U::from_model(self)
    }
}

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
        Entity: PresentationOf,
        Vec<T>: NormaliseTo<Entity::Model> + Default,
    {
        match self.loader.load(&key.into()).await {
            Ok(entity) => Ok(entity.unwrap_or_default().render_normalised().present_as()),
            Err(e) => Err(e),
        }
    }
}
