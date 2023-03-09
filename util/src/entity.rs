pub trait EventSourced {
    type Event;
    fn source(events: &[Self::Event]) -> Self;
}

pub trait HandleEvent {
    type Event;
    fn apply(&mut self, event: &Self::Event);
}

impl<T, E> EventSourced for T
where
    T: HandleEvent<Event = E> + Default,
{
    type Event = E;

    fn source(events: &[Self::Event]) -> Self {
        let mut state = Self::default();
        for event in events {
            state.apply(event);
        }
        state
    }
}
