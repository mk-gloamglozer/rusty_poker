use super::*;
use crate::command::event::ParticipantNotRemovedReason;
use serde::Deserialize;
use util::command::Command;
use util::HandleCommand;

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct RemoveParticipantCommand {
    participant_id: String,
}

impl RemoveParticipantCommand {
    pub fn new(participant_id: String) -> Self {
        Self { participant_id }
    }
}

impl Command for RemoveParticipantCommand {
    type Event = BoardModifiedEvent;
    type Entity = Board;

    fn apply(&self, entity: Self::Entity) -> Vec<Self::Event> {
        let RemoveParticipantCommand { participant_id } = self.clone();

        if !entity.participants.contains_key(&participant_id) {
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
    use util::use_case::EventSourced;

    #[test]
    pub fn it_should_remove_a_participant() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];

        let board = Board::source(&events);
        let command = RemoveParticipantCommand {
            participant_id: board.participants.keys().next().unwrap().to_string(),
        };
        let events = command.apply(board);
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
        let board = Board::new();
        let command = RemoveParticipantCommand {
            participant_id: "test".to_string(),
        };
        let events = command.apply(board);
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
