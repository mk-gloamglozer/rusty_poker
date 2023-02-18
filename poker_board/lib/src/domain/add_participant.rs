use super::*;
use util::HandleCommand;
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone)]
struct AddParticipantCommand {
    participant_name: String,
}

impl HandleCommand<AddParticipantCommand> for Board {
    type Event = BoardModifiedEvent;

    fn execute(&self, command: AddParticipantCommand) -> Vec<Self::Event> {
        let AddParticipantCommand { participant_name } = command;

        vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: Uuid::new_v4().to_string(),
            participant_name,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_should_add_a_participant() {
        let board = Board::new("test".to_string());
        let command = AddParticipantCommand {
            participant_name: "test".to_string(),
        };
        let events = board.execute(command);
        assert_eq!(events.len(), 1);
    }
}
