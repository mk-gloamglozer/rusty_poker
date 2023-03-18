use actix::{
    Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, Recipient, Running,
    StreamHandler,
};
use actix_web_actors::ws;
use actix_web_actors::ws::ProtocolError;
use poker_board::command::event::{BoardModifiedEvent, CombinedEvent};
use poker_board::command::BoardCommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use util::store::LoadEntity;
use util::use_case::UseCase;

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
struct BoardModifiedMessage(BoardModifiedEvent);

#[derive(Message)]
#[rtype(result = "()")]
struct Replay {
    board_id: BoardId,
    addr: Recipient<BoardModifiedMessage>,
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

#[derive(Clone, Default)]
struct EmptyBoard {
    sessions: HashMap<SessionId, Recipient<BoardModifiedMessage>>,
}

#[derive(Clone)]
enum BoardState {
    Empty(EmptyBoard),
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
        }
    }

    fn is_orphaned(&self) -> bool {
        match self {
            BoardState::Empty(board) => board.sessions.is_empty(),
            BoardState::Loaded(board) => board.sessions.is_empty(),
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
            BoardState::Loaded(board) => {
                board.events = events;
            }
        }
    }

    fn broadcast_changes(&mut self) {
        match self {
            BoardState::Empty(_) => {}
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

    fn replay_onto(&self, recipient: Recipient<BoardModifiedMessage>) {
        match self {
            BoardState::Empty(_) => {}
            BoardState::Loaded(board) => {
                for event in board.events.iter() {
                    recipient.do_send(BoardModifiedMessage(event.clone()));
                }
            }
        }
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type ReadStore = Box<dyn LoadEntity<Vec<BoardModifiedEvent>, Key = String, Error = Error>>;

struct MutexState(Mutex<HashMap<BoardId, BoardState>>);

impl MutexState {
    pub(crate) fn replay_board_onto(
        &self,
        id: BoardId,
        recipient: Recipient<BoardModifiedMessage>,
    ) {
        let state = self.0.lock().unwrap();
        if let Some(board) = state.get(&id) {
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

pub struct Session {
    id: SessionId,
    board_id: BoardId,
    server: Addr<ArcWsServer>,
    use_case_server: Addr<UseCaseServer>,
}

impl Session {
    pub fn new(
        session_id: SessionId,
        board_id: BoardId,
        server: Addr<ArcWsServer>,
        use_case_server: Addr<UseCaseServer>,
    ) -> Self {
        Self {
            id: session_id,
            board_id,
            server,
            use_case_server,
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

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.server.do_send(Disconnect {
            session_id: self.id,
        });
        Running::Stop
    }
}

#[derive(Debug, Deserialize)]
enum Command {
    Replay,
    Command(BoardCommand),
}

impl StreamHandler<Result<ws::Message, ProtocolError>> for Session {
    fn handle(&mut self, message: Result<ws::Message, ProtocolError>, ctx: &mut Self::Context) {
        match message {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Text(text)) => {
                let msg = serde_json::from_str::<Command>(&text);
                match msg {
                    Ok(Command::Replay) => {
                        self.server.do_send(Replay {
                            board_id: self.board_id.clone(),
                            addr: ctx.address().recipient(),
                        });
                    }
                    Ok(Command::Command(msg)) => {
                        self.use_case_server.do_send(CommandMessage {
                            board_id: self.board_id.clone(),
                            command: msg,
                        });
                    }
                    Err(err) => {
                        log::error!("Error: {:?}", err);
                    }
                }
            }
            Err(_) => ctx.stop(),
            _ => (),
        }
    }
}

#[derive(Debug, Deserialize, Message)]
#[rtype(result = "()")]
struct CommandMessage {
    board_id: BoardId,
    command: BoardCommand,
}

pub struct UseCaseServer {
    use_case: Arc<UseCase<CombinedEvent>>,
}

impl UseCaseServer {
    pub fn new(use_case: UseCase<CombinedEvent>) -> Self {
        Self {
            use_case: Arc::new(use_case),
        }
    }
}

impl Handler<CommandMessage> for UseCaseServer {
    type Result = ();

    fn handle(&mut self, msg: CommandMessage, _ctx: &mut Self::Context) -> Self::Result {
        let use_case = self.use_case.clone();
        actix::spawn(async move {
            use_case
                .execute(&msg.board_id.to_string(), &msg.command)
                .await
                .unwrap_or_else(|err| {
                    log::error!("Error: {:?}", err);
                    Vec::default()
                });
        });
    }
}

impl Actor for UseCaseServer {
    type Context = Context<Self>;
}
