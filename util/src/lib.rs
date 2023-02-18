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

pub trait FromEventStream {
    type Event;
    fn from_event_stream(entity: String, events: Vec<Self::Event>) -> Self;
}

pub trait HandleCommand<Command> {
    type Event;
    fn execute(&self, command: Command) -> Vec<Self::Event>;
}
