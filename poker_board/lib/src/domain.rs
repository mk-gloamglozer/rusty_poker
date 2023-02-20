pub mod add_participant;
pub mod clear_votes;
pub mod remove_participant;
pub mod vote;

use crate::event::BoardModifiedEvent;
use std::collections::HashMap;
use util::{FromEventStream, HandleEvent};

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
}

impl Participant {
    pub fn new(name: String) -> Self {
        Self { name }
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
            BoardModifiedEvent::ParticipantVoted { .. } => {}
            BoardModifiedEvent::ParticipantCouldNotVote { .. } => {}
            BoardModifiedEvent::VotesCleared => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ParticipantNotRemovedReason, ParticipantNotVotedReason};

    #[test]
    pub fn it_should_add_a_participant() {
        let mut board = Board::new();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(event);
        assert_eq!(board.participants.len(), 1);
    }

    #[test]
    pub fn it_should_remove_a_participant() {
        let mut board = Board::new();
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
    pub fn it_should_not_apply_participant_could_not_vote() {
        let mut board = Board::new();
        let expected = board.clone();
        let event = BoardModifiedEvent::ParticipantCouldNotVote {
            participant_id: "test".to_string(),
            reason: ParticipantNotVotedReason::DoesNotExist,
        };
        board.apply(event);
        assert_eq!(board, expected);
    }

    #[test]
    pub fn it_should_not_respond_to_participant_could_not_be_removed() {
        let mut board = Board::new();
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
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_a".to_string(),
                participant_name: "test_a".to_string(),
            },
        ];
        let board = Board::from_event_stream("test".to_string(), events);
        assert_eq!(board.participants.len(), 2);
        assert!(board.participants.get("test").unwrap().name.eq("test"));
        assert!(board.participants.get("test_a").unwrap().name.eq("test_a"));
    }
}
