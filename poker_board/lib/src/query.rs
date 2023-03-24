use crate::command::event::{BoardModifiedEvent, VoteValue};
use serde::Serialize;
use std::collections::HashMap;
use util::entity::HandleEvent;

pub mod presentation {
    use crate::query::{Board, Participant};
    use serde::Serialize;
    use util::query::PresentationOf;

    #[derive(Default, Debug, PartialEq, Clone, Serialize)]
    pub struct BoardPresentation {
        participants: Vec<Participant>,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        stats: Option<Stats>,
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
            BoardPresentation::new(model.participants.values().cloned().collect())
        }
    }

    fn stats(participants: Vec<Participant>) -> Option<Stats> {
        let mut votes = participants
            .iter()
            .map(|p| p.vote)
            .collect::<Option<Vec<u8>>>()?;

        let max = max(votes.iter())?;
        let min = min(votes.iter())?;
        let average = average(votes.iter_mut())?;

        Some(Stats {
            average: average as usize,
            max: max as usize,
            min: min as usize,
        })
    }

    fn max<'a>(votes: impl Iterator<Item = &'a u8>) -> Option<u8> {
        votes.max().cloned()
    }

    fn min<'a>(votes: impl Iterator<Item = &'a u8>) -> Option<u8> {
        votes.min().cloned()
    }

    fn average<'a>(votes: impl Iterator<Item = &'a mut u8>) -> Option<u8> {
        let mut votes = votes.collect::<Vec<&mut u8>>();
        if votes.is_empty() {
            None
        } else {
            votes.sort();
            let middle = votes.len() / 2;
            Some(*votes[middle])
        }
    }

    #[cfg(test)]
    mod presentation_tests {
        mod stats {
            use super::super::stats;
            use crate::query::Participant;
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
            fn it_should_return_none_if_not_all_particpants_have_voted() {
                let mut participants = vec![
                    Participant::new("John".into()),
                    Participant::new("Jane".into()),
                    Participant::new("Jack".into()),
                ];
                participants[0].vote = Some(1);
                participants[1].vote = Some(2);
                let stats = stats(participants);
                assert_eq!(stats, None);
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
        pub fn new(participants: Vec<Participant>) -> Self {
            Self {
                stats: stats(participants.clone()),
                participants,
            }
        }
    }
}

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
                    participant.vote = match vote.value {
                        VoteValue::Number(number) => Some(number),
                        VoteValue::String(_) => None,
                    };
                }
            }
            BoardModifiedEvent::ParticipantCouldNotVote { .. } => {}
            BoardModifiedEvent::VotesCleared => {
                for participant in self.participants.values_mut() {
                    participant.vote = None;
                }
            }
            BoardModifiedEvent::ParticipantNotAdded { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::event;
    use crate::command::event::{ParticipantNotRemovedReason, ParticipantNotVotedReason};
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
            vote: event::Vote::new("test".to_string(), VoteValue::Number(1)),
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
            vote: event::Vote::new("test".to_string(), VoteValue::Number(1)),
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
                vote: event::Vote::new("test".to_string(), VoteValue::Number(1)),
            },
        ];
        let board = Board::source(&events);
        assert_eq!(board.participants.len(), 1);
        assert!(board.participants.get("test").unwrap().vote.is_some());
    }
}
