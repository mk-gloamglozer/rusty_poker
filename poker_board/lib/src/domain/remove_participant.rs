use super::*;
use crate::event::ParticipantNotRemovedReason;
use util::HandleCommand;

#[derive(Debug, PartialEq, Clone)]
pub struct RemoveParticipantCommand {
    participant_id: String,
}

impl HandleCommand<RemoveParticipantCommand> for Board {
    type Event = BoardModifiedEvent;

    fn execute(&self, command: RemoveParticipantCommand) -> Vec<Self::Event> {
        let RemoveParticipantCommand { participant_id } = command;

        if !self.participants.contains_key(&participant_id) {
            return vec![BoardModifiedEvent::ParticipantCouldNotBeRemoved {
                participant_id,
                reason: ParticipantNotRemovedReason::DoesNotExist,
            }];
        }

        vec![BoardModifiedEvent::ParticipantRemoved { participant_id }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_should_remove_a_participant() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];

        let board = Board::from_event_stream("test".to_string(), events);
        let command = RemoveParticipantCommand {
            participant_id: board.participants.keys().next().unwrap().to_string(),
        };
        let events = board.execute(command);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantRemoved {
                participant_id: "test".to_string(),
            }
        );
    }

    #[test]
    pub fn it_should_not_remove_a_participant_that_does_not_exist() {
        let board = Board::new("test".to_string());
        let command = RemoveParticipantCommand {
            participant_id: "test".to_string(),
        };
        let events = board.execute(command);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotBeRemoved {
                participant_id: "test".to_string(),
                reason: ParticipantNotRemovedReason::DoesNotExist,
            }
        );
    }
}
