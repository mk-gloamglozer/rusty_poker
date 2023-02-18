use super::*;
use crate::event::ParticipantNotVotedReason;
use util::HandleCommand;

#[derive(Debug, PartialEq, Clone)]
pub struct ParticipantVote {
    pub participant_id: String,
    pub card_set_id: String,
    pub card_id: String,
}

impl HandleCommand<ParticipantVote> for Board {
    type Event = BoardModifiedEvent;

    fn execute(&self, command: ParticipantVote) -> Vec<Self::Event> {
        let ParticipantVote {
            participant_id,
            card_set_id,
            card_id,
        } = command;

        if self.participants.contains_key(&participant_id) {
            vec![BoardModifiedEvent::ParticipantVoted {
                participant_id,
                card_set_id,
                card_id,
            }]
        } else {
            vec![BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id,
                reason: ParticipantNotVotedReason::DoesNotExist,
            }]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_should_vote_for_a_participant() {
        let events = vec![BoardModifiedEvent::ParticipantAdded {
            participant_id: "test".to_string(),
            participant_name: "test".to_string(),
        }];
        let board = Board::from_event_stream("test".to_string(), events);
        let command = ParticipantVote {
            participant_id: board.participants.keys().next().unwrap().to_string(),
            card_set_id: "test".to_string(),
            card_id: "test".to_string(),
        };
        let events = board.execute(command);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantVoted {
                participant_id: "test".to_string(),
                card_set_id: "test".to_string(),
                card_id: "test".to_string(),
            }
        );
    }

    #[test]
    pub fn it_should_not_vote_for_a_participant_that_does_not_exist() {
        let board = Board::new("test".to_string());
        let command = ParticipantVote {
            participant_id: "test".to_string(),
            card_set_id: "test".to_string(),
            card_id: "test".to_string(),
        };
        let events = board.execute(command);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            BoardModifiedEvent::ParticipantCouldNotVote {
                participant_id: "test".to_string(),
                reason: ParticipantNotVotedReason::DoesNotExist,
            }
        );
    }
}
