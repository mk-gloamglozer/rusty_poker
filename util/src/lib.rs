use std::ops::Deref;

pub struct EventAggregate<T> {
    state: T,
}

impl<T> Deref for EventAggregate<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> From<T> for EventAggregate<T> {
    fn from(state: T) -> Self {
        Self { state }
    }
}

impl<T> EventAggregate<T> {
    pub fn new(state: T) -> Self {
        Self { state }
    }
}

impl<T, E> EventAggregate<T>
where
    T: HandleEvent<Event = E>,
{
    pub fn apply(mut self, event: E) -> Self {
        self.state.apply(event);
        self
    }
}

pub trait HandleEvent {
    type Event;
    fn apply(&mut self, event: Self::Event);
}
