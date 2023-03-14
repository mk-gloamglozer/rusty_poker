use super::*;
use crate::command::event::{ParticipantNotVotedReason, Vote, VoteValue};
use mockall::predicate::path::exists;
use serde::Deserialize;
use std::default::Default;
use util::command::Command;

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct ParticipantVote {
    pub participant_id: String,
    pub vote: Vote,
}

impl ParticipantVote {
    pub fn new(participant_id: String, vote: Vote) -> Self {
        Self {
            participant_id,
            vote,
        }
    }
}

impl VoteValidation {
    pub fn valid_vote(&self, vote: &VoteValue) -> bool {
        match self {
            VoteValidation::AnyNumber => {
                if let VoteValue::Number(_) = vote {
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl Command for ParticipantVote {
    type Entity = CombinedDomain;
    type Event = BoardModifiedEvent;

    fn apply(&self, entity: &Self::Entity) -> Vec<Self::Event> {
        let ParticipantVote {
            participant_id,
            vote,
        } = self.clone();

        let mut events = Vec::<BoardModifiedEvent>::default();
        let vote_type_exists = entity.0.vote_types.contains_key(&vote.vote_type_id);
        let participant_exists = entity.1.participants.contains_key(&participant_id);

        if !participant_exists {
            events.push(BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: participant_id.clone(),
                reason: ParticipantNotVotedReason::DoesNotExist,
            });
        }

        if !vote_type_exists {
            events.push(BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: participant_id.clone(),
                reason: ParticipantNotVotedReason::VoteTypeDoesNotExist(vote.vote_type_id.clone()),
            });
        } else {
            let validation = entity
                .0
                .vote_types
                .get(&vote.vote_type_id)
                .unwrap()
                .clone()
                .validation;

            if !validation.valid_vote(&vote.value) {
                events.push(BoardModifiedEvent::ParticipantCouldNotVote {
                    participant_id: participant_id.clone(),
                    reason: ParticipantNotVotedReason::InvalidVote {
                        expected: validation,
                        received: vote.value.clone(),
                    },
                });
            }
        }

        if events.is_empty() {
            events.push(BoardModifiedEvent::ParticipantVoted {
                participant_id,
                vote,
            });
        }

        events
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
        let mut vote_types = HashMap::new();
        vote_types.insert(
            "test".to_string(),
            VoteType {
                id: "test".to_string(),
                validation: VoteValidation::AnyNumber,
            },
        );
        let vote_type_list = VoteTypeList {
            vote_types: vote_types,
        };
        let combined_domain = CombinedDomain(vote_type_list, board.clone());
        let command = ParticipantVote {
            participant_id: board.participants.keys().next().unwrap().to_string(),
            vote: Vote::new("test".to_string(), VoteValue::Number(1)),
        };

        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            }
        );
    }

    #[test]
    pub fn it_should_not_vote_for_a_participant_that_does_not_exist() {
        let board = Board::new();
        let command = ParticipantVote {
            participant_id: "test".to_string(),
            vote: Vote::new("test".to_string(), VoteValue::Number(1)),
        };
        let mut vote_types = HashMap::new();
        vote_types.insert(
            "test".to_string(),
            VoteType {
                id: "test".to_string(),
                validation: VoteValidation::AnyNumber,
            },
        );
        let vote_type_list = VoteTypeList {
            vote_types: vote_types,
        };
        let combined_domain = CombinedDomain(vote_type_list, board);
        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reason: ParticipantNotVotedReason::DoesNotExist,
            }
        );
    }

    #[test]
    pub fn it_should_not_vote_when_vote_type_id_does_not_exist() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];
        let board = Board::source(&events);
        let command = ParticipantVote {
            participant_id: "test".to_string(),
            vote: Vote::new("not_present".to_string(), VoteValue::Number(1)),
        };
        let mut vote_types = HashMap::new();
        vote_types.insert(
            "test".to_string(),
            VoteType {
                id: "test".to_string(),
                validation: VoteValidation::AnyNumber,
            },
        );
        let vote_type_list = VoteTypeList {
            vote_types: vote_types,
        };
        let combined_domain = CombinedDomain(vote_type_list, board);
        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reason: ParticipantNotVotedReason::VoteTypeDoesNotExist("not_present".to_string()),
            }
        );
    }

    #[test]
    pub fn it_should_not_vote_when_vote_is_invalid() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];
        let board = Board::source(&events);
        let command = ParticipantVote {
            participant_id: "test".to_string(),
            vote: Vote::new("test".to_string(), VoteValue::String("test".to_string())),
        };
        let mut vote_types = HashMap::new();
        vote_types.insert(
            "test".to_string(),
            VoteType {
                id: "test".to_string(),
                validation: VoteValidation::AnyNumber,
            },
        );
        let vote_type_list = VoteTypeList {
            vote_types: vote_types,
        };
        let combined_domain = CombinedDomain(vote_type_list, board);
        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reason: ParticipantNotVotedReason::InvalidVote {
                    expected: VoteValidation::AnyNumber,
                    received: VoteValue::String("test".to_string()),
                }
            }
        );
    }
}
