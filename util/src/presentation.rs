use crate::command;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Input<Command: command::Command> {
    entity_id: String,
    #[serde(flatten)]
    command: Command,
}

impl<Command> Input<Command>
where
    Command: command::Command,
{
    pub fn id(&self) -> &String {
        &self.entity_id
    }

    pub fn command(&self) -> &Command {
        &self.command
    }
}
