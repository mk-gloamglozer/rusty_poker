use actix::{Actor, Context, Handler, Message, Recipient};
use poker_board::command::event::BoardModifiedEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use util::store::LoadEntity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(usize);

impl SessionId {
    pub fn new() -> Self {
        let id = rand::random::<usize>();
        Self(id)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoardId(String);

impl BoardId {
    pub fn new(board_id: String) -> Self {
        Self(board_id)
    }
}
impl ToString for BoardId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct BoardModifiedMessage(BoardModifiedEvent);

impl Into<BoardModifiedEvent> for BoardModifiedMessage {
    fn into(self) -> BoardModifiedEvent {
        self.0
    }
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct ReplayMessage(Vec<BoardModifiedEvent>);

impl Into<Vec<BoardModifiedEvent>> for ReplayMessage {
    fn into(self) -> Vec<BoardModifiedEvent> {
        self.0
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Replay {
    board_id: BoardId,
    addr: Recipient<ReplayMessage>,
}

impl Replay {
    pub fn new(board_id: BoardId, addr: Recipient<ReplayMessage>) -> Self {
        Self { board_id, addr }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    session_id: SessionId,
    board_id: BoardId,
    recipient: Recipient<BoardModifiedMessage>,
}

impl Connect {
    pub fn new(
        session_id: SessionId,
        board_id: BoardId,
        recipient: Recipient<BoardModifiedMessage>,
    ) -> Self {
        Self {
            session_id,
            board_id,
            recipient,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    session_id: SessionId,
}

impl Disconnect {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }
}

#[derive(Clone)]
struct Board {
    events: Vec<BoardModifiedEvent>,
    sessions: HashMap<SessionId, Recipient<BoardModifiedMessage>>,
    loc: usize,
}

#[derive(Clone, Default)]
struct EmptyBoard {
    sessions: HashMap<SessionId, Recipient<BoardModifiedMessage>>,
}

#[derive(Clone, Default)]
struct ReplayBoard {
    sessions: HashMap<SessionId, Recipient<BoardModifiedMessage>>,
    replay_addr: Vec<Recipient<ReplayMessage>>,
}

impl ReplayBoard {
    fn add_replay_addr(&mut self, addr: Recipient<ReplayMessage>) {
        self.replay_addr.push(addr);
    }

    fn replay(&mut self, events: &Vec<BoardModifiedEvent>) {
        for addr in self.replay_addr.iter() {
            addr.do_send(ReplayMessage(events.clone()));
        }
    }
}

#[derive(Clone)]
enum BoardState {
    Empty(EmptyBoard),
    Replay(ReplayBoard),
    Loaded(Board),
}

impl Default for BoardState {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            sessions: HashMap::new(),
            loc: 0,
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardState {
    fn new() -> Self {
        Self::Empty(EmptyBoard {
            sessions: HashMap::new(),
        })
    }
}

impl BoardState {
    fn add_session(&mut self, session_id: SessionId, recipient: Recipient<BoardModifiedMessage>) {
        match self {
            BoardState::Empty(board) => {
                board.sessions.insert(session_id, recipient);
            }
            BoardState::Loaded(board) => {
                board.sessions.insert(session_id, recipient);
            }
            BoardState::Replay(board) => {
                board.sessions.insert(session_id, recipient);
            }
        }
    }

    fn remove_session(&mut self, session_id: &SessionId) {
        match self {
            BoardState::Empty(board) => {
                board.sessions.remove(session_id);
            }
            BoardState::Loaded(board) => {
                board.sessions.remove(session_id);
            }
            BoardState::Replay(board) => {
                board.sessions.remove(session_id);
            }
        }
    }

    fn is_orphaned(&self) -> bool {
        match self {
            BoardState::Empty(board) => board.sessions.is_empty(),
            BoardState::Loaded(board) => board.sessions.is_empty(),
            BoardState::Replay(board) => board.sessions.is_empty(),
        }
    }

    fn update_events(&mut self, events: Vec<BoardModifiedEvent>) {
        match self {
            BoardState::Empty(board) => {
                let mut sessions = HashMap::new();
                std::mem::swap(&mut sessions, &mut board.sessions);
                let loc = events.len();
                *self = BoardState::Loaded(Board {
                    events,
                    sessions,
                    loc,
                });
            }
            BoardState::Replay(board) => {
                board.replay(&events);
                let mut sessions = HashMap::new();
                std::mem::swap(&mut sessions, &mut board.sessions);
                let loc = events.len();
                *self = BoardState::Loaded(Board {
                    events,
                    sessions,
                    loc,
                });
            }
            BoardState::Loaded(board) => {
                board.events = events;
            }
        }
    }

    fn broadcast_changes(&mut self) {
        match self {
            BoardState::Empty(_) => {}
            BoardState::Replay(_) => {}
            BoardState::Loaded(board) => {
                let loc = board.loc;
                for event in board.events.iter().skip(loc) {
                    for (_, recipient) in board.sessions.iter() {
                        recipient.do_send(BoardModifiedMessage(event.clone()));
                    }
                }
                board.loc = board.events.len();
            }
        }
    }

    fn replay_onto(&mut self, recipient: Recipient<ReplayMessage>) {
        match self {
            BoardState::Empty(board) => {
                let mut sessions = HashMap::new();
                std::mem::swap(&mut sessions, &mut board.sessions);
                *self = BoardState::Replay(ReplayBoard {
                    sessions,
                    replay_addr: vec![recipient],
                });
            }
            BoardState::Replay(board) => {
                board.add_replay_addr(recipient);
            }
            BoardState::Loaded(board) => {
                recipient.do_send(ReplayMessage(board.events.clone()));
            }
        }
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type ReadStore = Box<dyn LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Error>>;

struct MutexState(Mutex<HashMap<BoardId, BoardState>>);

impl MutexState {
    pub(crate) fn replay_board_onto(&self, id: BoardId, recipient: Recipient<ReplayMessage>) {
        let mut state = self.0.lock().unwrap();
        if let Some(board) = state.get_mut(&id) {
            board.replay_onto(recipient);
        }
    }
}

impl MutexState {
    fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl MutexState {
    fn insert_session(
        &self,
        board_id: BoardId,
        session_id: SessionId,
        recipient: Recipient<BoardModifiedMessage>,
    ) {
        let mut state = self.0.lock().unwrap();
        state
            .entry(board_id)
            .or_default()
            .add_session(session_id, recipient);
    }

    fn remove_session(&self, session_id: &SessionId) {
        let mut state = self.0.lock().unwrap();

        let mut orphaned_boards = Vec::new();
        for (id, board) in state.iter_mut() {
            board.remove_session(session_id);
            if board.is_orphaned() {
                orphaned_boards.push(id.clone());
            }
        }

        for id in orphaned_boards {
            state.remove(&id);
        }
    }

    fn update_events(&self, board_id: BoardId, events: Vec<BoardModifiedEvent>) {
        let mut state = self.0.lock().unwrap();
        state.entry(board_id).or_default().update_events(events);
    }

    fn broadcast_changes(&self) {
        let mut state = self.0.lock().unwrap();
        for (_id, board) in state.iter_mut() {
            board.broadcast_changes();
        }
    }

    fn boards(&self) -> HashMap<BoardId, BoardState> {
        let state = self.0.lock().unwrap();
        state.clone()
    }
}

pub struct WsServer {
    state: MutexState,
    read_store: ReadStore,
}

impl WsServer {
    pub fn new<T>(read_store: T) -> Self
    where
        T: LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Error> + 'static,
    {
        Self {
            state: MutexState::new(),
            read_store: Box::new(read_store),
        }
    }
}

#[derive(Clone)]
pub struct ArcWsServer(Arc<WsServer>);
impl ArcWsServer {
    pub fn new<T>(read_store: T) -> Self
    where
        T: LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Error> + 'static,
    {
        Self(Arc::new(WsServer::new(read_store)))
    }
}

impl Handler<Connect> for ArcWsServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        self.0
            .state
            .insert_session(msg.board_id, msg.session_id, msg.recipient);
    }
}

impl Handler<Disconnect> for ArcWsServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.0.state.remove_session(&msg.session_id);
    }
}

impl Handler<Replay> for ArcWsServer {
    type Result = ();

    fn handle(&mut self, msg: Replay, _: &mut Context<Self>) {
        self.0.state.replay_board_onto(msg.board_id, msg.addr);
    }
}

impl WsServer {
    async fn try_update(&self) -> Result<(), Error> {
        for (id, _board) in self.state.boards() {
            let events = self
                .read_store
                .load(&id.to_string())
                .await?
                .unwrap_or_default();
            self.state.update_events(id.clone(), events.clone());
        }
        Ok(())
    }

    fn broadcast_changes(&self) {
        self.state.broadcast_changes();
    }
}

impl ArcWsServer {
    async fn try_update(&self) -> Result<(), Error> {
        self.0.try_update().await
    }

    fn broadcast_changes(&self) {
        self.0.broadcast_changes();
    }
}

impl Actor for ArcWsServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("Server started");
        let server = self.clone();
        actix::spawn(async move {
            loop {
                server.try_update().await.unwrap_or_else(|err| {
                    log::error!("Error: {:?}", err);
                });
                server.broadcast_changes();
                actix::clock::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }
}
