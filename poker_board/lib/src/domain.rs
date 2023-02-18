use crate::event::BoardModifiedEvent;
use std::collections::HashMap;
use util::{FromEventStream, HandleEvent};

#[derive(Debug, PartialEq, Clone)]
pub struct Board {
    id: String,
    participants: HashMap<String, Participant>,
}

impl Board {
    pub fn new(id: String) -> Self {
        Self {
            id,
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

    fn apply(&mut self, event: Self::Event) {
        match event {
            BoardModifiedEvent::ParticipantAdded {
                participant_id,
                participant_name,
            } => {
                let participant = Participant::new(participant_name);
                self.participants.insert(participant_id, participant);
            }
            BoardModifiedEvent::ParticipantRemoved { participant_id } => {
                self.participants.remove(&participant_id);
            }
            BoardModifiedEvent::ParticipantCouldNotBeRemoved { .. } => {}
            BoardModifiedEvent::ParticipantVoted {
                participant_id,
                card_set_id,
                card_id,
            } => {
                if let Some(participant) = self.participants.get_mut(&participant_id) {
                    participant.vote = Some(Vote::new(card_set_id, card_id));
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

impl FromEventStream for Board {
    type Event = BoardModifiedEvent;

    fn from_event_stream(entity: String, events: Vec<Self::Event>) -> Self {
        let mut board = Board::new(entity);
        for event in events {
            board.apply(event);
        }
        board
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ParticipantNotRemovedReason, ParticipantNotVotedReason};

    #[test]
    pub fn it_should_add_a_participant() {
        let mut board = Board::new("test".to_string());
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(event);
        assert_eq!(board.participants.len(), 1);
    }

    #[test]
    pub fn it_should_remove_a_participant() {
        let mut board = Board::new("test".to_string());
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(event);
        let event = BoardModifiedEvent::ParticipantRemoved {
            participant_id: "test".to_string(),
        };
        board.apply(event);
        assert_eq!(board.participants.len(), 0);
    }

    #[test]
    pub fn it_should_add_a_vote() {
        let mut board = Board::new("test".to_string());
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(event);
        let event = BoardModifiedEvent::ParticipantVoted {
            participant_id: "test".to_string(),
            card_set_id: "test".to_string(),
            card_id: "test".to_string(),
        };
        board.apply(event);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
    }

    #[test]
    pub fn it_should_not_apply_participant_could_not_vote() {
        let mut board = Board::new("test".to_string());
        let expected = board.clone();
        let event = BoardModifiedEvent::ParticipantCouldNotVote {
            participant_id: "test".to_string(),
            reason: ParticipantNotVotedReason::DoesNotExist,
        };
        board.apply(event);
        assert_eq!(board, expected);
    }

    #[test]
    pub fn it_should_clear_votes() {
        let mut board = Board::new("test".to_string());
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(event);
        let event = BoardModifiedEvent::ParticipantVoted {
            participant_id: "test".to_string(),
            card_set_id: "test".to_string(),
            card_id: "test".to_string(),
        };
        board.apply(event);
        let event = BoardModifiedEvent::VotesCleared;
        board.apply(event);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_none());
    }

    #[test]
    pub fn it_should_not_respond_to_participant_could_not_be_removed() {
        let mut board = Board::new("test".to_string());
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(event);

        let expected = board.clone();

        let event = BoardModifiedEvent::ParticipantCouldNotBeRemoved {
            participant_id: "test".to_string(),
            reason: ParticipantNotRemovedReason::DoesNotExist,
        };
        board.apply(event);
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
        let board = Board::from_event_stream("test".to_string(), events);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
    }
}
