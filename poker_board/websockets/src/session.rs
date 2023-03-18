use crate::session::output::Response;
use crate::{
    ArcWsServer, BoardId, BoardModifiedMessage, Connect, Disconnect, Replay, ReplayMessage,
    SessionId,
};
use actix::{
    Actor, ActorContext, ActorStreamExt, Addr, AsyncContext, Context, Handler, Message, Recipient,
    Running, StreamHandler,
};
use actix_web_actors::ws;
use actix_web_actors::ws::ProtocolError;
use poker_board::command::event::{BoardModifiedEvent, CombinedEvent};
use poker_board::command::BoardCommand;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use util::entity::HandleEvent;
use util::use_case::UseCase;

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
        self.server.do_send(Connect::new(
            self.id,
            self.board_id.clone(),
            ctx.address().recipient(),
        ));
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.server.do_send(Disconnect::new(self.id));
        Running::Stop
    }
}

#[derive(Debug, Deserialize)]
enum Command {
    Replay,
    Command { key: usize, command: BoardCommand },
}

impl Handler<CommandResultMessage> for Session {
    type Result = ();

    fn handle(&mut self, msg: CommandResultMessage, ctx: &mut Self::Context) -> Self::Result {}
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
                        // self.server.do_send(Replay {
                        //     board_id: self.board_id.clone(),
                        //     addr: ctx.address().recipient(),
                        // });
                    }
                    Ok(Command::Command { key, command }) => {
                        self.use_case_server.do_send(CommandMessage {
                            addr: ctx.address().recipient(),
                            board_id: self.board_id.clone(),
                            command,
                            key,
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

#[derive(Debug, Message)]
#[rtype(result = "()")]
struct CommandMessage {
    board_id: BoardId,
    command: BoardCommand,
    key: usize,
    addr: Recipient<CommandResultMessage>,
}

#[derive(Debug, Message, Serialize)]
#[rtype(result = "()")]
pub enum CommandResultMessage {
    Success {
        events: Vec<BoardModifiedEvent>,
        key: usize,
    },
    Error {
        key: usize,
    },
}

trait SendTo<T>
where
    T: Message + Send + Sync + 'static,
    T::Result: Send + Sync + 'static,
{
    fn send_to(self, addr: &Recipient<T>);
}

impl<T> SendTo<T> for T
where
    T: Message + Send + Sync + 'static,
    T::Result: Send + Sync + 'static,
{
    fn send_to(self, addr: &Recipient<T>) {
        addr.do_send(self);
    }
}

pub struct UseCaseServer {
    use_case: Arc<UseCase<CombinedEvent>>,
}

impl UseCaseServer {
    pub fn new(use_case: Arc<UseCase<CombinedEvent>>) -> Self {
        Self { use_case }
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
                .map(|events| CommandResultMessage::Success {
                    events,
                    key: msg.key,
                })
                .unwrap_or_else(|err| {
                    log::error!("Error: {:?}", err);
                    CommandResultMessage::Error { key: msg.key }
                })
                .send_to(&msg.addr);
        });
    }
}

impl Actor for UseCaseServer {
    type Context = Context<Self>;
}

pub struct CommandQuerySession<T> {
    board_id: BoardId,
    server: Addr<ArcWsServer>,
    command_server: Addr<UseCaseServer>,
    query: QueryState<T>,
    session_id: SessionId,
}

impl<T> CommandQuerySession<T> {
    pub fn new(
        board_id: BoardId,
        server: Addr<ArcWsServer>,
        command_server: Addr<UseCaseServer>,
    ) -> Self {
        Self {
            board_id,
            server,
            query: QueryState::Initial(Vec::default()),
            session_id: SessionId::new(),
            command_server,
        }
    }
}

impl<T> CommandQuerySession<T>
where
    T: Unpin + 'static + HandleEvent<Event = BoardModifiedEvent> + Default + Serialize,
{
    fn handle_event<E: Into<BoardModifiedEvent>>(&mut self, event: E) -> &QueryState<T> {
        match &mut self.query {
            QueryState::Initial(events) => {
                events.push(event.into());
            }
            QueryState::Live(query) => {
                query.apply(&event.into());
            }
        }
        &self.query
    }

    fn replay<E: Into<Vec<BoardModifiedEvent>>>(&mut self, events: E) -> &QueryState<T> {
        match &self.query {
            QueryState::Initial(queued_events) => {
                self.query = QueryState::Live({
                    let mut live_state = T::default();
                    for event in queued_events {
                        live_state.apply(event);
                    }
                    for event in events.into() {
                        live_state.apply(&event);
                    }
                    live_state
                });
            }
            QueryState::Live(_) => {}
        };
        &self.query
    }
}

#[derive(Debug, Clone, PartialEq)]
enum QueryState<T> {
    Initial(Vec<BoardModifiedEvent>),
    Live(T),
}

impl<T> Actor for CommandQuerySession<T>
where
    T: Unpin
        + 'static
        + HandleEvent<Event = BoardModifiedEvent>
        + Default
        + Serialize
        + Debug
        + Send
        + Clone
        + PartialEq,
{
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.server.do_send(Connect::new(
            self.session_id,
            self.board_id.clone(),
            ctx.address().recipient(),
        ));

        self.server.do_send(Replay::new(
            self.board_id.clone(),
            ctx.address().recipient(),
        ));
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.server.do_send(Disconnect::new(self.session_id));
        Running::Stop
    }
}

impl<T> Handler<ReplayMessage> for CommandQuerySession<T>
where
    T: Unpin
        + 'static
        + HandleEvent<Event = BoardModifiedEvent>
        + Default
        + Serialize
        + Debug
        + Send
        + Clone
        + PartialEq,
{
    type Result = ();

    fn handle(&mut self, msg: ReplayMessage, ctx: &mut Self::Context) -> Self::Result {
        self.replay(msg);
        if let QueryState::Live(query) = &self.query {
            ctx.address()
                .do_send(output::Response::QueryUpdate(query.clone()));
        }
    }
}

impl<T> Handler<BoardModifiedMessage> for CommandQuerySession<T>
where
    T: Unpin
        + 'static
        + HandleEvent<Event = BoardModifiedEvent>
        + Default
        + Serialize
        + Debug
        + Send
        + Clone
        + PartialEq,
{
    type Result = ();

    fn handle(&mut self, msg: BoardModifiedMessage, ctx: &mut Self::Context) -> Self::Result {
        let prev_state = self.query.clone();
        self.handle_event(msg);
        if let QueryState::Live(query) = &self.query {
            if prev_state.eq(&self.query) {
                return;
            }
            ctx.address()
                .do_send(output::Response::QueryUpdate(query.clone()));
        }
    }
}

impl<T> Handler<CommandResultMessage> for CommandQuerySession<T>
where
    T: Unpin
        + 'static
        + HandleEvent<Event = BoardModifiedEvent>
        + Default
        + Serialize
        + Debug
        + Send
        + Clone
        + PartialEq,
{
    type Result = ();

    fn handle(&mut self, msg: CommandResultMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.address().do_send(output::Response::Command(msg));
    }
}

mod input {
    use poker_board::command::BoardCommand;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Command {
        pub key: usize,
        #[serde(flatten)]
        pub command: BoardCommand,
    }
}

mod output {

    use super::CommandResultMessage;
    use crate::session::QueryState;
    use actix::Message;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Message)]
    #[rtype(result = "()")]
    pub enum Response<T> {
        Command(CommandResultMessage),
        QueryUpdate(T),
        Error(String),
    }
}

impl<T> Handler<output::Response<T>> for CommandQuerySession<T>
where
    T: Unpin
        + 'static
        + HandleEvent<Event = BoardModifiedEvent>
        + Default
        + Serialize
        + Debug
        + Send
        + Clone
        + PartialEq,
{
    type Result = ();

    fn handle(&mut self, msg: output::Response<T>, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&msg).unwrap_or_else(|err| {
            log::error!("Error: {:?}", err);
            String::default()
        }));
    }
}

impl<T> StreamHandler<Result<ws::Message, ProtocolError>> for CommandQuerySession<T>
where
    T: Unpin
        + 'static
        + HandleEvent<Event = BoardModifiedEvent>
        + Default
        + Serialize
        + Debug
        + Send
        + Clone
        + PartialEq,
{
    fn handle(&mut self, message: Result<ws::Message, ProtocolError>, ctx: &mut Self::Context) {
        match message {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Text(text)) => {
                match serde_json::from_str::<input::Command>(&text).map(|command| CommandMessage {
                    addr: ctx.address().recipient(),
                    board_id: self.board_id.clone(),
                    command: command.command,
                    key: command.key,
                }) {
                    Ok(command) => self.command_server.do_send(command),
                    Err(err) => ctx
                        .address()
                        .do_send(output::Response::Error(format!("{:?}", err))),
                }
            }
            Err(_) => ctx.stop(),
            _ => (),
        }
    }
}
