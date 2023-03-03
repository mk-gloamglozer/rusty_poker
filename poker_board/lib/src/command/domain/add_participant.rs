use super::*;
use serde::Deserialize;
use util::command::Command;
use util::HandleCommand;
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct AddParticipantCommand {
    participant_name: String,
}

impl AddParticipantCommand {
    pub fn new(participant_name: String) -> Self {
        Self { participant_name }
    }
}

impl Command for AddParticipantCommand {
    type Entity = Board;
    type Event = BoardModifiedEvent;

    fn apply(&self, entity: Self::Entity) -> Vec<Self::Event> {
        let AddParticipantCommand { participant_name } = self;

        vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: Uuid::new_v4().to_string(),
            participant_name: participant_name.clone(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_should_add_a_participant() {
        let board = Board::new();
        let command = AddParticipantCommand {
            participant_name: "test".to_string(),
        };
        let events = command.apply(board);
        assert_eq!(events.len(), 1);
    }
}
