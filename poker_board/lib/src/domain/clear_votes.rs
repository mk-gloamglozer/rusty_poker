use super::*;
use util::HandleCommand;

#[derive(Debug, Clone, PartialEq)]
pub struct ClearVotes {}

impl ClearVotes {
    pub fn new() -> Self {
        Self {}
    }
}

impl HandleCommand<ClearVotes> for Board {
    type Event = BoardModifiedEvent;

    fn execute(&self, _command: ClearVotes) -> Vec<Self::Event> {
        vec![BoardModifiedEvent::VotesCleared]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_should_clear_votes() {
        let board = Board::new();
        let command = ClearVotes {};
        let events = board.execute(command);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], BoardModifiedEvent::VotesCleared);
    }
}
