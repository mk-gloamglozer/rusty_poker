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

#[derive(Debug, Clone, PartialEq)]
pub enum ParticipantNotRemovedReason {
    DoesNotExist,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParticipantNotVotedReason {
    DoesNotExist,
}
