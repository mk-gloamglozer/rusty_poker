use super::*;
use crate::command::event::ParticipantNotVotedReason;
use serde::Deserialize;
use util::command::Command;

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct ParticipantVote {
    pub participant_id: String,
    pub vote: u8,
}

impl ParticipantVote {
    pub fn new(participant_id: String, vote: u8) -> Self {
        Self {
            participant_id,
            vote,
        }
    }
}

impl Command for ParticipantVote {
    type Entity = Board;
    type Event = BoardModifiedEvent;

    fn apply(&self, entity: &Self::Entity) -> Vec<Self::Event> {
        let ParticipantVote {
            participant_id,
            vote,
        } = self.clone();

        if entity.participants.contains_key(&participant_id) {
            vec![BoardModifiedEvent::ParticipantVoted {
                participant_id,
                vote,
            }]
        } else {
            vec![BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id,
                reason: ParticipantNotVotedReason::DoesNotExist,
            }]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::entity::EventSourced;

    #[test]
    pub fn it_should_vote_for_a_participant() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];
        let board = Board::source(&events);
        let command = ParticipantVote {
            participant_id: board.participants.keys().next().unwrap().to_string(),
            vote: 1,
        };
        let events = command.apply(&board);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: 1,
            }
        );
    }

    #[test]
    pub fn it_should_not_vote_for_a_participant_that_does_not_exist() {
        let board = Board::new();
        let command = ParticipantVote {
            participant_id: "test".to_string(),
            vote: 1,
        };
        let events = command.apply(&board);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reason: ParticipantNotVotedReason::DoesNotExist,
            }
        );
    }
}
