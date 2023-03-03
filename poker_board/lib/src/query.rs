use crate::command::event::BoardModifiedEvent;
use std::collections::HashMap;
use util::use_case::HandleEvent;

#[derive(Default, Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub struct Participant {
    name: String,
    vote: Option<Vote>,
}

impl Participant {
    pub fn new(name: String) -> Self {
        Self { name, vote: None }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Vote {
    card_set_id: String,
    card_id: String,
}

impl Vote {
    pub fn new(card_set_id: String, card_id: String) -> Self {
        Self {
            card_set_id,
            card_id,
        }
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
                card_set_id,
                card_id,
            } => {
                if let Some(participant) = self.participants.get_mut(participant_id) {
                    participant.vote = Some(Vote::new(card_set_id.clone(), card_id.clone()));
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
            card_set_id: "test".to_string(),
            card_id: "test".to_string(),
        };
        board.apply(&event);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
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
            card_set_id: "test".to_string(),
            card_id: "test".to_string(),
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
                card_set_id: "test".to_string(),
                card_id: "test".to_string(),
            },
        ];
        let board = Board::source(&events);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
    }
}
