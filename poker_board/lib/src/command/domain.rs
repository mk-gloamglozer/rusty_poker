pub mod add_participant;
pub mod clear_votes;
pub mod remove_participant;
pub mod vote;

use crate::command::event::{BoardModifiedEvent, CombinedEvent, VoteTypeEvent, VoteValidation};
use std::collections::HashMap;
use util::entity::HandleEvent;

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
            BoardModifiedEvent::ParticipantVoted { .. } => {}
            BoardModifiedEvent::ParticipantCouldNotVote { .. } => {}
            BoardModifiedEvent::VotesCleared => {}
            BoardModifiedEvent::ParticipantNotAdded { .. } => {}
        }
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct VoteTypeList {
    pub vote_types: HashMap<String, VoteType>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct VoteType {
    pub id: String,
    pub validation: VoteValidation,
}

impl VoteTypeList {
    pub fn add_vote_type(&mut self, vote_type: VoteType) {
        self.vote_types.insert(vote_type.id.clone(), vote_type);
    }
}

impl VoteType {
    pub fn new(id: String, validation: VoteValidation) -> Self {
        Self { id, validation }
    }
}

impl HandleEvent for VoteTypeList {
    type Event = VoteTypeEvent;

    fn apply(&mut self, event: &Self::Event) {
        match event {
            VoteTypeEvent::VoteTypeAdded {
                vote_type_id,
                vote_validation,
            } => {
                self.add_vote_type(VoteType::new(vote_type_id.clone(), vote_validation.clone()));
            }
        }
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct CombinedDomain(VoteTypeList, Board);

impl CombinedDomain {
    pub fn vote_type_list(&self) -> &VoteTypeList {
        &self.0
    }

    pub fn board(&self) -> &Board {
        &self.1
    }
}

impl HandleEvent for CombinedDomain {
    type Event = CombinedEvent;

    fn apply(&mut self, event: &Self::Event) {
        match event {
            CombinedEvent::VoteTypeEvent(vote_type_event) => {
                self.0.apply(vote_type_event);
            }
            CombinedEvent::BoardModifiedEvent(board_modified_event) => {
                self.1.apply(board_modified_event);
            }
        }
    }
}

#[cfg(test)]
mod board_tests {
    use super::*;
    use crate::command::event::{
        BoardModifiedEvent, ParticipantNotRemovedReason, ParticipantNotVotedReason,
    };
    use util::entity::{EventSourced, HandleEvent};

    #[test]
    pub fn it_should_add_a_participant() {
        let mut board = Board::new();
        let event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        board.apply(&event);
        assert_eq!(board.participants.len(), 1);
    }

    #[test]
    pub fn it_should_remove_a_participant() {
        let mut board = Board::new();
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
    pub fn it_should_not_apply_participant_could_not_vote() {
        let mut board = Board::new();
        let expected = board.clone();
        let event = BoardModifiedEvent::ParticipantCouldNotVote {
            participant_id: "test".to_string(),
            reasons: vec![ParticipantNotVotedReason::DoesNotExist],
        };
        board.apply(&event);
        assert_eq!(board, expected);
    }

    #[test]
    pub fn it_should_not_respond_to_participant_could_not_be_removed() {
        let mut board = Board::new();
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
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_a".to_string(),
                participant_name: "test_a".to_string(),
            },
        ];
        let board = Board::source(&events);
        assert_eq!(board.participants.len(), 2);
        assert!(board.participants.get("test").unwrap().name.eq("test"));
        assert!(board.participants.get("test_a").unwrap().name.eq("test_a"));
    }
}

#[cfg(test)]
mod vote_list_tests {
    use super::*;
    use crate::command::event::VoteTypeEvent;
    use util::entity::{EventSourced, HandleEvent};

    #[test]
    pub fn it_should_add_a_vote_type() {
        let mut vote_type_list = VoteTypeList::default();
        let event = VoteTypeEvent::VoteTypeAdded {
            vote_type_id: "test".to_string(),
            vote_validation: VoteValidation::AnyNumber,
        };
        vote_type_list.apply(&event);
        assert_eq!(vote_type_list.vote_types.len(), 1);
    }

    #[test]
    pub fn it_should_reconstruct_from_event_stream() {
        let events = vec![
            VoteTypeEvent::VoteTypeAdded {
                vote_type_id: "test".to_string(),
                vote_validation: VoteValidation::AnyNumber,
            },
            VoteTypeEvent::VoteTypeAdded {
                vote_type_id: "test_a".to_string(),
                vote_validation: VoteValidation::AnyNumber,
            },
        ];
        let vote_type_list = VoteTypeList::source(&events);
        assert_eq!(vote_type_list.vote_types.len(), 2);
        assert!(vote_type_list
            .vote_types
            .get("test")
            .unwrap()
            .validation
            .eq(&VoteValidation::AnyNumber));
        assert!(vote_type_list
            .vote_types
            .get("test_a")
            .unwrap()
            .validation
            .eq(&VoteValidation::AnyNumber));
    }
}

#[cfg(test)]
mod combined_domain_tests {
    use super::*;
    use crate::command::event::{BoardModifiedEvent, VoteTypeEvent};
    use util::entity::HandleEvent;

    #[test]
    pub fn it_should_apply_events_to_both_vote_type_list_and_board() {
        let mut combined_domain = CombinedDomain::default();
        let vote_type_event = VoteTypeEvent::VoteTypeAdded {
            vote_type_id: "test".to_string(),
            vote_validation: VoteValidation::AnyNumber,
        };
        let board_modified_event = BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        };
        let combined_event = CombinedEvent::VoteTypeEvent(vote_type_event);
        combined_domain.apply(&combined_event);
        let combined_event = CombinedEvent::BoardModifiedEvent(board_modified_event);
        combined_domain.apply(&combined_event);
        assert_eq!(combined_domain.0.vote_types.len(), 1);
        assert_eq!(combined_domain.1.participants.len(), 1);
    }
}
