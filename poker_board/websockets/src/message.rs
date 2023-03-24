use super::Error;
use actix::{Message, Recipient};
use poker_board::command::event;
use poker_board::command::event::BoardModifiedEvent;

#[derive(Message)]
#[rtype(result = "Result<Vec<BoardModifiedEvent>, Error>")]
#[derive(Debug, Clone)]
pub struct SaveEvents {
    pub key: String,
    pub event: Vec<BoardModifiedEvent>,
}

#[derive(Message)]
#[rtype(result = "Result<Option<Vec<BoardModifiedEvent>>, Error>")]
#[derive(Debug, Clone)]
pub struct LoadEvents {
    pub key: String,
}

#[derive(Message)]
#[rtype(result = "()")]
#[derive(Debug, Clone)]
pub struct BoardModified {
    pub event: Vec<event::BoardModifiedEvent>,
}

#[derive(Message)]
#[rtype(result = "()")]
#[derive(Debug, Clone)]
pub struct Subscribe {
    pub board_id: String,
    pub address: Recipient<BoardModified>,
}

#[derive(Message)]
#[rtype(result = "Result<Vec<BoardModifiedEvent>, Error>")]
#[derive(Debug, Clone)]
pub struct Command {
    pub board_id: String,
    pub command: poker_board::command::BoardCommand,
}
