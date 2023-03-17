use actix::{
    Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, Recipient, Running,
    StreamHandler,
};
use actix_web_actors::ws;
use actix_web_actors::ws::ProtocolError;
use log::log;
use poker_board::command::event::BoardModifiedEvent;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use util::store::LoadEntity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(usize);

impl SessionId {
    pub fn new() -> Self {
        let id = rand::random::<usize>();
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
struct BoardModifiedMessage {
    board_id: String,
    event: BoardModifiedEvent,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Connect {
    session_id: SessionId,
    board_id: BoardId,
    recipient: Recipient<BoardModifiedMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    session_id: SessionId,
}

#[derive(Clone)]
struct Board {
    events: Vec<BoardModifiedEvent>,
    sessions: HashMap<SessionId, Recipient<BoardModifiedMessage>>,
    loc: usize,
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

impl Board {
    fn add_session(&mut self, session_id: SessionId, recipient: Recipient<BoardModifiedMessage>) {
        self.sessions.insert(session_id, recipient);
    }

    fn remove_session(&mut self, session_id: &SessionId) {
        self.sessions.remove(session_id);
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type ReadStore = Box<dyn LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Error>>;

struct State {
    boards: HashMap<BoardId, Board>,
}

impl State {
    fn new() -> Self {
        Self {
            boards: HashMap::new(),
        }
    }
}

struct MutexState(Mutex<HashMap<BoardId, Board>>);

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
            if board.sessions.is_empty() {
                orphaned_boards.push(id.clone());
            }
        }

        for id in orphaned_boards {
            state.remove(&id);
        }
    }

    fn set_position(&self, board_id: BoardId, loc: usize) {
        let mut state = self.0.lock().unwrap();
        state.entry(board_id).or_default().loc = loc;
    }

    fn update_events(&self, board_id: BoardId, events: Vec<BoardModifiedEvent>) {
        let mut state = self.0.lock().unwrap();
        state.entry(board_id).or_default().events = events;
    }

    fn broadcast_changes(&self) {
        let mut state = self.0.lock().unwrap();
        for (id, board) in state.iter_mut() {
            let mut loc = board.loc;
            for event in board.events.iter().skip(loc) {
                for (_, recipient) in board.sessions.iter() {
                    recipient.do_send(BoardModifiedMessage {
                        board_id: id.to_string(),
                        event: event.clone(),
                    });
                }
                loc += 1;
            }
            board.loc = loc;
        }
    }

    fn boards(&self) -> HashMap<BoardId, Board> {
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

impl WsServer {
    async fn try_update(&self) -> Result<(), Error> {
        for (id, board) in self.state.boards() {
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

    fn started(&mut self, ctx: &mut Self::Context) {
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

pub struct Session {
    id: SessionId,
    board_id: BoardId,
    server: Addr<ArcWsServer>,
}

impl Session {
    pub fn new(session_id: SessionId, board_id: BoardId, server: Addr<ArcWsServer>) -> Self {
        Self {
            id: session_id,
            board_id,
            server,
        }
    }
}

impl Handler<BoardModifiedMessage> for Session {
    type Result = ();

    fn handle(&mut self, msg: BoardModifiedMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&msg).unwrap());
    }
}

impl Actor for Session {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.server.do_send(Connect {
            session_id: self.id,
            board_id: self.board_id.clone(),
            recipient: ctx.address().recipient(),
        });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.server.do_send(Disconnect {
            session_id: self.id,
        });
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Session {
    fn handle(&mut self, message: Result<ws::Message, ProtocolError>, ctx: &mut Self::Context) {
        match message {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(_) => ctx.stop(),
            _ => (),
        }
    }
}
