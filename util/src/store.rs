#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    type Event;
    type Error;
    type Key;
    async fn modify(
        &self,
        key: &Self::Key,
        event: &dyn EventStreamModifier<Self::Event>,
    ) -> Result<Vec<Self::Event>, Self::Error>;
}

pub trait EventStreamModifier<Event>: Send + Sync {
    fn modify(&self, events: Vec<Event>) -> Vec<Event>;
}

impl<T, Event> EventStreamModifier<Event> for T
where
    T: Fn(Vec<Event>) -> Vec<Event> + Send + Sync,
{
    fn modify(&self, events: Vec<Event>) -> Vec<Event> {
        (self)(events)
    }
}
