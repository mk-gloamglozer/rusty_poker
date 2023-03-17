use crate::command::domain::{CombinedDomain, VoteTypeList};
use crate::command::Board;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use util::transaction::NormaliseTo;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BoardModifiedEvent {
    ParticipantAdded {
        participant_id: String,
        participant_name: String,
    },
    ParticipantRemoved {
        participant_id: String,
    },
    ParticipantCouldNotBeRemoved {
        participant_id: String,
        reason: ParticipantNotRemovedReason,
    },
    ParticipantVoted {
        participant_id: String,
        vote: Vote,
    },
    ParticipantCouldNotVote {
        participant_id: String,
        reasons: Vec<ParticipantNotVotedReason>,
    },
    VotesCleared,
}

impl Display for BoardModifiedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Vote {
    pub vote_type_id: String,
    pub value: VoteValue,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum VoteValue {
    Number(u8),
    String(String),
}

impl Vote {
    pub fn new(vote_type_id: String, value: VoteValue) -> Self {
        Self {
            vote_type_id,
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ParticipantNotRemovedReason {
    DoesNotExist,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ParticipantNotVotedReason {
    DoesNotExist,
    VoteTypeDoesNotExist(String),
    InvalidVote {
        expected: VoteValidation,
        received: VoteValue,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum VoteTypeEvent {
    VoteTypeAdded {
        vote_type_id: String,
        vote_validation: VoteValidation,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VoteValidation {
    AnyNumber,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CombinedEvent {
    BoardModifiedEvent(BoardModifiedEvent),
    VoteTypeEvent(VoteTypeEvent),
}

impl From<BoardModifiedEvent> for CombinedEvent {
    fn from(event: BoardModifiedEvent) -> Self {
        Self::BoardModifiedEvent(event)
    }
}

impl From<VoteTypeEvent> for CombinedEvent {
    fn from(event: VoteTypeEvent) -> Self {
        Self::VoteTypeEvent(event)
    }
}
