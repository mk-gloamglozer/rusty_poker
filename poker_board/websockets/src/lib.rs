mod server;
pub use server::*;

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
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use util::entity::HandleEvent;
use util::store::LoadEntity;
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
                        // self.server.do_send(Replay {
                        //     board_id: self.board_id.clone(),
                        //     addr: ctx.address().recipient(),
                        // });
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

pub struct QuerySession<T> {
    board_id: BoardId,
    server: Addr<ArcWsServer>,
    query: QueryState<T>,
    session_id: SessionId,
}

impl<T> QuerySession<T> {
    pub fn new(board_id: BoardId, server: Addr<ArcWsServer>) -> Self {
        Self {
            board_id,
            server,
            query: QueryState::Initial(Vec::default()),
            session_id: SessionId::new(),
        }
    }
}

impl<T> QuerySession<T>
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

#[derive(Debug)]
enum QueryState<T> {
    Initial(Vec<BoardModifiedEvent>),
    Live(T),
}

impl<T> Actor for QuerySession<T>
where
    T: Unpin + 'static + HandleEvent<Event = BoardModifiedEvent> + Default + Serialize + Debug,
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

impl<T> Handler<ReplayMessage> for QuerySession<T>
where
    T: Unpin + 'static + HandleEvent<Event = BoardModifiedEvent> + Default + Serialize + Debug,
{
    type Result = ();

    fn handle(&mut self, msg: ReplayMessage, ctx: &mut Self::Context) -> Self::Result {
        self.replay(msg);
        if let QueryState::Live(query) = &self.query {
            ctx.text(serde_json::to_string(&query).unwrap_or_else(|err| {
                log::error!("Error: {:?}", err);
                String::default()
            }));
        }
    }
}

impl<T> Handler<BoardModifiedMessage> for QuerySession<T>
where
    T: Unpin + 'static + HandleEvent<Event = BoardModifiedEvent> + Default + Serialize + Debug,
{
    type Result = ();

    fn handle(&mut self, msg: BoardModifiedMessage, ctx: &mut Self::Context) -> Self::Result {
        self.handle_event(msg);
        if let QueryState::Live(query) = &self.query {
            ctx.text(serde_json::to_string(&query).unwrap_or_else(|err| {
                log::error!("Error: {:?}", err);
                String::default()
            }));
        }
    }
}

impl<T> StreamHandler<Result<ws::Message, ProtocolError>> for QuerySession<T>
where
    T: Unpin + 'static + HandleEvent<Event = BoardModifiedEvent> + Default + Serialize + Debug,
{
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
