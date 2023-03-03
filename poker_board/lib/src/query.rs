use crate::command::event::BoardModifiedEvent;
use serde::Serialize;
use std::collections::HashMap;
use util::use_case::HandleEvent;

#[derive(Default, Debug, PartialEq, Clone, Serialize)]
pub struct Board {
    participants: HashMap<String, Participant>,
}

impl Board {
    pub fn new() -> Self {
        Self {
            participants: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Participant {
    name: String,
    vote: Option<u8>,
}

impl Participant {
    pub fn new(name: String) -> Self {
        Self { name, vote: None }
    }
}

impl HandleEvent for Board {
    type Event = BoardModifiedEvent;

    fn apply(&mut self, event: &Self::Event) {
        match event {
            BoardModifiedEvent::ParticipantAdded {
                participant_id,
                participant_name,
            } => {
                let participant = Participant::new(participant_name.clone());
                self.participants
                    .insert(participant_id.clone(), participant);
            }
            BoardModifiedEvent::ParticipantRemoved { participant_id } => {
                self.participants.remove(participant_id);
            }
            BoardModifiedEvent::ParticipantCouldNotBeRemoved { .. } => {}
            BoardModifiedEvent::ParticipantVoted {
                participant_id,
                vote,
            } => {
                if let Some(participant) = self.participants.get_mut(participant_id) {
                    participant.vote = Some(*vote);
                }
            }
            BoardModifiedEvent::ParticipantCouldNotVote { .. } => {}
            BoardModifiedEvent::VotesCleared => {
                for participant in self.participants.values_mut() {
                    participant.vote = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::event::{ParticipantNotRemovedReason, ParticipantNotVotedReason};
    use util::use_case::EventSourced;

    #[test]
    pub fn it_should_add_a_participant() {
        let mut board = Board::default();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(&event);
        assert_eq!(board.participants.len(), 1);
    }

    #[test]
    pub fn it_should_remove_a_participant() {
        let mut board = Board::default();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(&event);
        let event = BoardModifiedEvent::ParticipantRemoved {
            participant_id: "test".to_string(),
        };
        board.apply(&event);
        assert_eq!(board.participants.len(), 0);
    }

    #[test]
    pub fn it_should_add_a_vote() {
        let mut board = Board::default();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(&event);
        let event = BoardModifiedEvent::ParticipantVoted {
            participant_id: "test".to_string(),
            vote: 1,
        };
        board.apply(&event);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
        assert_eq!(board.participants.get("test").unwrap().vote.unwrap(), 1);
    }

    #[test]
    pub fn it_should_not_apply_participant_could_not_vote() {
        let mut board = Board::default();
        let expected = board.clone();
        let event = BoardModifiedEvent::ParticipantCouldNotVote {
            participant_id: "test".to_string(),
            reason: ParticipantNotVotedReason::DoesNotExist,
        };
        board.apply(&event);
        assert_eq!(board, expected);
    }

    #[test]
    pub fn it_should_clear_votes() {
        let mut board = Board::default();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(&event);
        let event = BoardModifiedEvent::ParticipantVoted {
            participant_id: "test".to_string(),
            vote: 1,
        };
        board.apply(&event);
        let event = BoardModifiedEvent::VotesCleared;
        board.apply(&event);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_none());
    }

    #[test]
    pub fn it_should_not_respond_to_participant_could_not_be_removed() {
        let mut board = Board::default();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(&event);

        let expected = board.clone();

        let event = BoardModifiedEvent::ParticipantCouldNotBeRemoved {
            participant_id: "test".to_string(),
            reason: ParticipantNotRemovedReason::DoesNotExist,
        };
        board.apply(&event);
        assert_eq!(board, expected);
    }

    #[test]
    pub fn it_should_reconstruct_from_event_stream() {
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: 1,
            },
        ];
        let board = Board::source(&events);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
    }
}
