use super::*;
use crate::command::event::ParticipantNotAddedReason;
use serde::Deserialize;
use util::command::Command;
use util::validate::ValidateCommand;
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct AddParticipantCommand {
    participant_name: String,
    participant_id: Option<String>,
}

impl AddParticipantCommand {
    pub fn new(participant_name: String) -> Self {
        Self {
            participant_name,
            participant_id: None,
        }
    }

    pub fn with_id(participant_name: String, participant_id: String) -> Self {
        Self {
            participant_name,
            participant_id: Some(participant_id),
        }
    }
}

fn have_unique_id(
    entity: &Board,
    command: &AddParticipantCommand,
) -> Option<ParticipantNotAddedReason> {
    if entity
        .participants
        .contains_key(&command.participant_id.clone().unwrap_or("".to_string()))
    {
        Some(ParticipantNotAddedReason::AlreadyExists)
    } else {
        None
    }
}

impl Command for AddParticipantCommand {
    type Entity = Board;
    type Event = BoardModifiedEvent;

    fn apply(&self, entity: &Self::Entity) -> Vec<Self::Event> {
        self.should(have_unique_id)
            .validate_against(entity)
            .map(|command| BoardModifiedEvent::ParticipantAdded {
                participant_id: command
                    .participant_id
                    .clone()
                    .unwrap_or(Uuid::new_v4().to_string()),
                participant_name: command.participant_name.clone(),
            })
            .unwrap_or_else(
                |(_command, reasons)| BoardModifiedEvent::ParticipantNotAdded {
                    participant_id: self.participant_id.clone().unwrap_or("".to_string()),
                    reason: reasons[0].clone(),
                },
            )
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::entity::EventSourced;

    #[test]
    pub fn it_should_add_a_participant() {
        let board = Board::new();
        let command = AddParticipantCommand {
            participant_name: "test".to_string(),
            participant_id: None,
        };
        let events = command.apply(&board);
        assert_eq!(events.len(), 1);
    }

    #[test]
    pub fn it_should_add_a_participant_with_id() {
        let board = Board::new();
        let command = AddParticipantCommand {
            participant_name: "test".to_string(),
            participant_id: Some("test".to_string()),
        };
        let events = command.apply(&board);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            }
        );
    }

    #[test]
    pub fn it_should_not_add_a_participant_with_id_that_already_exists() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];
        let board = Board::source(&events);
        let command = AddParticipantCommand {
            participant_name: "test".to_string(),
            participant_id: Some("test".to_string()),
        };
        let events = command.apply(&board);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantNotAdded {
                participant_id: "test".to_string(),
                reason: ParticipantNotAddedReason::AlreadyExists,
            }
        );
    }
}
