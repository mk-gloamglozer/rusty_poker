use super::*;
use crate::command::event::BoardModifiedEvent::ParticipantVoted;
use crate::command::event::{ParticipantNotVotedReason, Vote, VoteValue};
use serde::Deserialize;
use util::command::Command;
use util::validate::ValidateCommand;

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
    fn valid_vote(&self, vote: &VoteValue) -> Option<ParticipantNotVotedReason> {
        match self {
            VoteValidation::AnyNumber => {
                if let VoteValue::Number(_) = vote {
                    None
                } else {
                    Some(ParticipantNotVotedReason::InvalidVote {
                        expected: self.clone(),
                        received: vote.clone(),
                    })
                }
            }
        }
    }
}

impl From<BoardModifiedEvent> for Vec<BoardModifiedEvent> {
    fn from(val: BoardModifiedEvent) -> Self {
        vec![val]
    }
}

fn be_valid_vote(
    entity: &CombinedDomain,
    command: &ParticipantVote,
) -> Option<ParticipantNotVotedReason> {
    entity
        .0
        .vote_types
        .get(&command.vote.vote_type_id)
        .map(|v| v.validation.valid_vote(&command.vote.value))
        .unwrap_or(Some(ParticipantNotVotedReason::VoteTypeDoesNotExist(
            command.vote.vote_type_id.clone(),
        )))
}

fn have_existing_participant(
    entity: &CombinedDomain,
    command: &ParticipantVote,
) -> Option<ParticipantNotVotedReason> {
    match entity.1.participants.contains_key(&command.participant_id) {
        true => None,
        false => Some(ParticipantNotVotedReason::DoesNotExist),
    }
}

impl Command for ParticipantVote {
    type Entity = CombinedDomain;
    type Event = BoardModifiedEvent;

    fn apply(&self, entity: &Self::Entity) -> Vec<Self::Event> {
        self.should(be_valid_vote)
            .should(have_existing_participant)
            .validate_against(entity)
            .map(|_| ParticipantVoted {
                participant_id: self.participant_id.clone(),
                vote: self.vote.clone(),
            })
            .unwrap_or_else(|(_, errors)| BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: self.participant_id.clone(),
                reasons: errors,
            })
            .into()
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
        let vote_type_list = VoteTypeList { vote_types };
        let combined_domain = CombinedDomain(vote_type_list, board.clone());
        let command = ParticipantVote {
            participant_id: board.participants.keys().next().unwrap().to_string(),
            vote: Vote::new("test".to_string(), VoteValue::Number(1)),
        };

        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            ParticipantVoted {
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
        let vote_type_list = VoteTypeList { vote_types };
        let combined_domain = CombinedDomain(vote_type_list, board);
        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reasons: vec![ParticipantNotVotedReason::DoesNotExist],
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
        let vote_type_list = VoteTypeList { vote_types };
        let combined_domain = CombinedDomain(vote_type_list, board);
        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reasons: vec![ParticipantNotVotedReason::VoteTypeDoesNotExist(
                    "not_present".to_string()
                )],
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
        let vote_type_list = VoteTypeList { vote_types };
        let combined_domain = CombinedDomain(vote_type_list, board);
        let events = command.apply(&combined_domain);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reasons: vec![ParticipantNotVotedReason::InvalidVote {
                    expected: VoteValidation::AnyNumber,
                    received: VoteValue::String("test".to_string()),
                }]
            }
        );
    }
}
