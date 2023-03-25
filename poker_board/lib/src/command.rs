use crate::command::domain::add_participant::AddParticipantCommand;
use crate::command::domain::clear_votes::ClearVotes;
use crate::command::domain::remove_participant::RemoveParticipantCommand;
use crate::command::domain::vote::ParticipantVote;
pub use crate::command::domain::Board;
use crate::command::domain::CombinedDomain;
use crate::command::event::{BoardModifiedEvent, Vote, VoteValue};
use serde::Deserialize;
use util::command::Command;

pub mod adapter;
mod domain;
pub mod event;

#[derive(Debug, Clone, Deserialize)]
pub enum BoardCommand {
    AddParticipant(AddParticipantCommand),
    ClearVotes(ClearVotes),
    RemoveParticipant(RemoveParticipantCommand),
    Vote(ParticipantVote),
    Noop,
}

impl Command for BoardCommand {
    type Entity = CombinedDomain;
    type Event = BoardModifiedEvent;

    fn apply(&self, entity: &Self::Entity) -> Vec<Self::Event> {
        match self {
            BoardCommand::AddParticipant(command) => command.apply(entity.board()),
            BoardCommand::ClearVotes(command) => command.apply(entity.board()),
            BoardCommand::RemoveParticipant(command) => command.apply(entity.board()),
            BoardCommand::Vote(command) => command.apply(entity),
            BoardCommand::Noop => vec![],
        }
    }
}

pub fn vote(vote: u8, vote_type: String, participant_id: String) -> BoardCommand {
    let vote = Vote::new(vote_type, VoteValue::Number(vote));
    BoardCommand::Vote(ParticipantVote::new(participant_id, vote))
}

pub fn add_participant(name: String, id: String) -> BoardCommand {
    BoardCommand::AddParticipant(AddParticipantCommand::with_id(name, id))
}

pub fn remove_participant(id: String) -> BoardCommand {
    BoardCommand::RemoveParticipant(RemoveParticipantCommand::new(id))
}
