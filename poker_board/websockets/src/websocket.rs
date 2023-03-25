use crate::store::LoadUpdate;
use crate::Error;
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, Recipient, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::{ProtocolError, WebsocketContext};
use poker_board::command;
use poker_board::command::event::BoardModifiedEvent;
use poker_board::command::{remove_participant, BoardCommand};

use actix_web::{web, HttpResponse};
use poker_board::query::presentation::BoardPresentation;
use poker_board::query::Board;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use util::entity::HandleEvent;
use util::query::PresentationOf;

use crate::websocket::WsCommand::ParticipantVoted;

#[derive(Clone, Deserialize, Debug)]
struct Command {
    #[serde(flatten)]
    command: WsCommand,
}

#[derive(Clone, Deserialize, Debug)]
enum WsCommand {
    ParticipantVoted { vote: u8 },
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct WebSocket {
    board_id: String,
    updates: Arc<dyn LoadUpdate<Vec<BoardModifiedEvent>, Key = String, Error = Error>>,
    use_case: Arc<std::sync::mpsc::Sender<UseCaseMessage>>,
    task_handle: Option<JoinHandle<()>>,
    id: String,
    name: String,
    hb: Instant,
}

#[derive(Debug)]
pub struct UseCaseMessage {
    pub board_id: String,
    pub command: BoardCommand,
    pub receiver: Recipient<ServerMessage>,
}

pub fn start(
    r: actix_web::HttpRequest,
    stream: web::Payload,
    board_id: String,
    updates: Arc<dyn LoadUpdate<Vec<BoardModifiedEvent>, Key = String, Error = Error>>,
    use_case_tx: Arc<std::sync::mpsc::Sender<UseCaseMessage>>,
    name: String,
) -> Result<HttpResponse, actix_web::error::Error> {
    ws::start(
        WebSocket::new(board_id, updates, use_case_tx, name),
        &r,
        stream,
    )
}

impl WebSocket {
    pub fn new(
        board_id: String,
        udpdates: Arc<dyn LoadUpdate<Vec<BoardModifiedEvent>, Key = String, Error = Error>>,
        use_case: Arc<std::sync::mpsc::Sender<UseCaseMessage>>,
        name: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            board_id,
            updates: udpdates,
            use_case,
            task_handle: None,
            name,
            hb: Instant::now(),
        }
    }

    fn hb(&self, ctx: &mut WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }

            ctx.ping(b"heyo");
        });
    }

    async fn update_loop(
        addr: Addr<WebSocket>,
        updates: Arc<dyn LoadUpdate<Vec<BoardModifiedEvent>, Key = String, Error = Error>>,
        board_id: &String,
    ) {
        let mut last_event: usize = 0;
        let mut state = Board::new();

        loop {
            let updates = updates.load_update(&board_id, last_event).await;

            match updates {
                Ok(updates) => {
                    updates.iter().for_each(|event| state.apply(event));
                    let presentation = BoardPresentation::from_model(&state);
                    match addr.send(ServerMessage::QueryUpdated(presentation)).await {
                        Ok(_) => {
                            last_event += updates.len();
                        }
                        Err(e) => {
                            log::error!("Error sending message: {}", e);
                        }
                    };
                }
                Err(e) => {
                    log::error!("Error loading updates: {}", e);
                }
            }
        }
    }
}

#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub enum ServerMessage {
    QueryUpdated(BoardPresentation),
    CommandResult(Vec<BoardModifiedEvent>),
    Error(String),
}

impl ServerMessage {
    pub fn send_to(self, addr: Recipient<ServerMessage>) {
        addr.do_send(self);
    }
}

impl WebSocket {
    fn convert_command(&self, command: WsCommand) -> BoardCommand {
        match command {
            ParticipantVoted { vote } => command::vote(vote, "1".to_string(), self.id.clone()),
        }
    }
}

impl Handler<ServerMessage> for WebSocket {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&msg).unwrap());
    }
}

impl StreamHandler<Result<ws::Message, ProtocolError>> for WebSocket {
    fn handle(&mut self, message: Result<ws::Message, ProtocolError>, ctx: &mut Self::Context) {
        match message {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg)
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Text(text)) => {
                let msg = serde_json::from_str::<Command>(&text);
                match msg {
                    Ok(command) => {
                        let addr = ctx.address().recipient();
                        let key = self.board_id.clone();
                        let command = self.convert_command(command.command);
                        let use_case = self.use_case.clone();
                        use_case
                            .send(UseCaseMessage {
                                board_id: key,
                                command,
                                receiver: addr,
                            })
                            .unwrap_or_else(|err| {
                                log::error!("Error sending command: {:?}", err);
                                ctx.address().do_send(ServerMessage::Error(format!(
                                    "There was an error processing your command {}",
                                    err
                                )));
                            });
                    }
                    Err(err) => {
                        log::error!("Error deserializing command: {:?} {:?}", text, err);
                        ctx.address().do_send(ServerMessage::Error(format!(
                            "There was an error processing your command {}",
                            err
                        )));
                    }
                }
            }
            Err(_) => ctx.stop(),
            _ => (),
        }
    }
}

impl Actor for WebSocket {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        let updates = self.updates.clone();
        let board_id = self.board_id.clone();

        let id = self.id.clone();
        let name = self.name.clone();
        let use_case = self.use_case.clone();

        if let Ok(_) = use_case
            .send(UseCaseMessage {
                board_id: board_id.clone(),
                command: command::add_participant(name.clone(), id.clone()),
                receiver: addr.clone().recipient(),
            })
            .or_else(|err| {
                log::error!("Error Adding Participant: {:?}", err.0);
                Err(err)
            })
        {
            let handle = tokio::spawn(async move {
                {
                    Self::update_loop(addr, updates, &board_id).await;
                }
            });
            self.task_handle = Some(handle);
        };
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        let id = self.id.clone();
        let use_case = self.use_case.clone();
        let board_id = self.board_id.clone();

        let remove_participant = remove_participant(id.clone());
        self.use_case
            .send(UseCaseMessage {
                board_id,
                command: remove_participant,
                receiver: ctx.address().recipient(),
            })
            .unwrap_or_else(|err| {
                log::error!("Error Removing Participant: {:?}", err.0);
            });
    }
}
