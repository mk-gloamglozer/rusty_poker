use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Input<Command> {
    entity_id: String,
    #[serde(flatten)]
    command: Command,
}

impl<Command> Input<Command> {
    pub fn id(&self) -> &String {
        &self.entity_id
    }

    pub fn command(&self) -> &Command {
        &self.command
    }
}

pub trait Command {
    type Entity;
    type Event;
    fn apply(&self, entity: Self::Entity) -> Vec<Self::Event>;
}

impl<Entity, Cmd, Event> Command for Input<Cmd>
where
    Cmd: Command<Entity = Entity, Event = Event>,
{
    type Entity = Entity;
    type Event = Event;
    fn apply(&self, entity: Entity) -> Vec<Self::Event> {
        self.command.apply(entity)
    }
}
