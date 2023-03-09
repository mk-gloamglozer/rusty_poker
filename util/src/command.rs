pub trait Command {
    type Entity;
    type Event;
    fn apply(&self, entity: &Self::Entity) -> Vec<Self::Event>;
}

pub trait HandleCommand<Command> {
    type Event;
    fn execute(&self, command: Command) -> Vec<Self::Event>;
}
