use crate::domain;
use serde::{Deserialize, Serialize};
use util::CommandDto;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearVotesDto {
    entity_id: String,
}

impl Into<CommandDto<domain::clear_votes::ClearVotes>> for ClearVotesDto {
    fn into(self) -> CommandDto<domain::clear_votes::ClearVotes> {
        CommandDto::new(
            self.entity_id.to_string(),
            domain::clear_votes::ClearVotes {},
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddParticipantDto {
    entity_id: String,
    name: String,
}

impl Into<CommandDto<domain::add_participant::AddParticipantCommand>> for AddParticipantDto {
    fn into(self) -> CommandDto<domain::add_participant::AddParticipantCommand> {
        CommandDto::new(
            self.entity_id,
            domain::add_participant::AddParticipantCommand::new(self.name),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveParticipantDto {
    entity_id: String,
    participant_id: String,
}

impl Into<CommandDto<domain::remove_participant::RemoveParticipantCommand>>
    for RemoveParticipantDto
{
    fn into(self) -> CommandDto<domain::remove_participant::RemoveParticipantCommand> {
        CommandDto::new(
            self.entity_id,
            domain::remove_participant::RemoveParticipantCommand::new(self.participant_id),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteDto {
    entity_id: String,
    participant_id: String,
    card_set_id: String,
    card_id: String,
}

impl Into<CommandDto<domain::vote::ParticipantVote>> for VoteDto {
    fn into(self) -> CommandDto<domain::vote::ParticipantVote> {
        CommandDto::new(
            self.entity_id,
            domain::vote::ParticipantVote::new(self.participant_id, self.card_set_id, self.card_id),
        )
    }
}
