use crate::command::event::{BoardModifiedEvent, VoteValue};
use serde::Serialize;
use std::collections::HashMap;
use util::entity::HandleEvent;

pub mod presentation {
    use crate::query::{Board, Participant};
    use serde::Serialize;
    use std::borrow::{Borrow, BorrowMut};
    use util::query::PresentationOf;

    #[derive(Default, Debug, PartialEq, Clone, Serialize)]
    pub struct BoardPresentation {
        participants: Vec<Participant>,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        stats: Option<Stats>,
        voting_complete: bool,
    }

    #[derive(Default, Debug, PartialEq, Clone, Serialize)]
    struct Stats {
        average: usize,
        max: usize,
        min: usize,
    }

    impl PresentationOf for BoardPresentation {
        type Model = Board;
        fn from_model(model: &Self::Model) -> Self {
            BoardPresentation::new(
                model.participants.values().cloned().collect(),
                model.voting_complete,
            )
        }
    }

    fn stats(participants: Vec<Participant>) -> Option<Stats> {
        let mut votes = participants
            .iter()
            .map(|p| p.vote)
            .filter_map(|v| v)
            .collect::<Vec<u8>>();

        let votes = votes.iter().filter(|v| **v != 0);
        let max = votes.clone().max().copied()?;
        let min = votes.clone().min().copied()?;
        let average = average(votes.copied())?;

        Some(Stats {
            average: average as usize,
            max: max as usize,
            min: min as usize,
        })
    }

    fn average<'a>(votes: impl Iterator<Item = u8>) -> Option<u8> {
        let mut votes = votes.collect::<Vec<u8>>();
        if votes.len() == 0 {
            None
        } else {
            votes.sort();
            let middle = (votes.len() / 2);
            Some(votes[middle])
        }
    }

    #[cfg(test)]
    mod presentation_tests {
        use crate::command::event::{BoardModifiedEvent, Vote, VoteValue};
        use crate::query::presentation::BoardPresentation;
        use crate::query::{Board, Participant};
        use std::collections::HashMap;
        use util::entity::HandleEvent;
        use util::query::PresentAs;

        #[test]
        fn it_should_match_the_voting_complete_status_of_the_board() {
            let mut board = Board {
                participants: HashMap::new(),
                voting_complete: false,
                number_voted: 0,
            };

            let presentation: BoardPresentation = board.present_as();
            assert!(!presentation.voting_complete);

            board.voting_complete = true;
            let presentation: BoardPresentation = board.present_as();
            assert!(presentation.voting_complete);
        }

        #[test]
        fn it_should_not_include_stats_if_voting_incomplete() {
            let participants = {
                let mut map = HashMap::new();
                for (i, participant) in vec![
                    Participant::new("John".into()),
                    Participant {
                        name: "Jane".to_string(),
                        vote: Some(1),
                    },
                ]
                .into_iter()
                .enumerate()
                {
                    map.insert(i.to_string(), participant);
                }
                map
            };

            let mut board = Board {
                participants,
                voting_complete: false,
                number_voted: 0,
            };
            let presentation: BoardPresentation = board.present_as();
            assert!(presentation.stats.is_none());
        }

        mod stats {
            use super::super::stats;
            use crate::query::Participant;
            #[test]
            fn it_should_ignore_0_votes() {
                let mut participants = vec![
                    Participant::new("John".into()),
                    Participant::new("Jane".into()),
                    Participant::new("Jack".into()),
                    Participant::new("Jill".into()),
                ];
                participants[0].vote = Some(0);
                participants[1].vote = Some(4);
                participants[2].vote = Some(5);
                participants[3].vote = Some(6);
                let stats = stats(participants);
                assert!(stats.is_some());
                let stats = stats.unwrap();
                assert_eq!(stats.average, 5);
                assert_eq!(stats.max, 6);
                assert_eq!(stats.min, 4);
            }

            #[test]
            fn it_should_return_none_when_no_participants() {
                let participants = vec![];
                let stats = stats(participants);
                assert_eq!(stats, None);
            }

            #[test]
            fn it_should_return_none_when_no_votes() {
                let participants = vec![Participant::new("John".into())];
                let stats = stats(participants);
                assert_eq!(stats, None);
            }

            #[test]
            fn it_should_ignore_non_voted_participants_in_calculation() {
                let mut participants = vec![
                    Participant::new("John".into()),
                    Participant::new("Jane".into()),
                    Participant::new("Jack".into()),
                ];
                participants[0].vote = Some(1);
                participants[1].vote = Some(2);
                let stats = stats(participants);
                assert!(stats.is_some());
                let stats = stats.unwrap();
                assert_eq!(stats.average, 2);
                assert_eq!(stats.max, 2);
                assert_eq!(stats.min, 1);
            }

            #[test]
            fn it_should_return_some_stats_if_all_particpants_have_voted() {
                let mut participants = vec![
                    Participant::new("John".into()),
                    Participant::new("Jane".into()),
                    Participant::new("Jack".into()),
                ];
                participants[0].vote = Some(1);
                participants[1].vote = Some(2);
                participants[2].vote = Some(3);
                let stats = stats(participants);
                assert!(stats.is_some());
                let stats = stats.unwrap();
                assert_eq!(stats.average, 2);
                assert_eq!(stats.max, 3);
                assert_eq!(stats.min, 1);
            }
        }
    }

    impl BoardPresentation {
        pub fn new(participants: Vec<Participant>, voting_complete: bool) -> Self {
            Self {
                stats: voting_complete
                    .then_some(participants.clone())
                    .and_then(stats),
                participants,
                voting_complete,
            }
        }
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct Board {
    participants: HashMap<String, Participant>,
    voting_complete: bool,
    number_voted: usize,
}

impl Board {
    pub fn new() -> Self {
        Self {
            participants: HashMap::new(),
            voting_complete: false,
            number_voted: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Participant {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
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
                    if participant.vote.is_none() {
                        self.number_voted += 1;
                    }
                    participant.vote = match vote.value {
                        VoteValue::Number(number) => Some(number),
                        VoteValue::String(_) => None,
                    };
                }

                if self.number_voted == self.participants.len() {
                    self.voting_complete = true;
                }
            }
            BoardModifiedEvent::ParticipantCouldNotVote { .. } => {}
            BoardModifiedEvent::VotesCleared => {
                for participant in self.participants.values_mut() {
                    participant.vote = None;
                }
                self.number_voted = 0;
                self.voting_complete = false;
            }
            BoardModifiedEvent::ParticipantNotAdded { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::event;
    use crate::command::event::{ParticipantNotRemovedReason, ParticipantNotVotedReason, Vote};
    use util::entity::EventSourced;

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
            vote: Vote::new("test".to_string(), VoteValue::Number(1)),
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
            reasons: vec![ParticipantNotVotedReason::DoesNotExist],
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
            vote: Vote::new("test".to_string(), VoteValue::Number(1)),
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
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
        ];
        let board = Board::source(&events);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
    }

    #[test]
    pub fn it_should_complete_voting_when_all_participants_have_voted() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_1".to_string(),
                participant_name: "test_1".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test_1".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, true);
    }

    #[test]
    fn voting_should_not_be_complete_when_not_all_participants_have_voted() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_1".to_string(),
                participant_name: "test_1".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, false);
    }

    #[test]
    fn voting_should_not_be_complete_when_no_participants_have_voted() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_1".to_string(),
                participant_name: "test_1".to_string(),
            },
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, false);
    }

    #[test]
    fn voting_should_remain_complete_when_participant_changes_vote() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(2)),
            },
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, true);
    }

    #[test]
    fn voting_should_remain_complete_when_a_participant_is_removed() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_1".to_string(),
                participant_name: "test_1".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test_1".to_string(),
                vote: Vote::new("test_1".to_string(), VoteValue::Number(2)),
            },
            BoardModifiedEvent::ParticipantRemoved {
                participant_id: "test".to_string(),
            },
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, true);
    }

    #[test]
    fn voting_should_remain_complete_when_a_participant_is_added() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test_1".to_string(),
                participant_name: "test_1".to_string(),
            },
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, true);
    }

    #[test]
    fn it_should_not_be_complete_after_votes_cleared() {
        let mut board = Board::default();
        let events = vec![
            BoardModifiedEvent::ParticipantAdded {
                participant_id: "test".to_string(),
                participant_name: "test".to_string(),
            },
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                vote: Vote::new("test".to_string(), VoteValue::Number(1)),
            },
            BoardModifiedEvent::VotesCleared,
        ];
        for event in events {
            board.apply(&event);
        }

        assert_eq!(board.voting_complete, false);
    }
}
