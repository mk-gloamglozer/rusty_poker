use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
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
        card_set_id: String,
        card_id: String,
    },
    ParticipantCouldNotVote {
        participant_id: String,
        reason: ParticipantNotVotedReason,
    },
    VotesCleared,
}

impl Display for BoardModifiedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParticipantNotRemovedReason {
    DoesNotExist,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParticipantNotVotedReason {
    DoesNotExist,
}
