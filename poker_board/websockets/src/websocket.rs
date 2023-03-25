use crate::store::LoadUpdate;
use crate::Error;
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, Recipient, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::{ProtocolError, WebsocketContext};
use poker_board::command::event::{BoardModifiedEvent, CombinedEvent};
use poker_board::command::{add_participant, remove_participant, vote, BoardCommand};
use poker_board::{command, query};

use poker_board::query::presentation::BoardPresentation;
use poker_board::query::Board;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;
use util::entity::HandleEvent;
use util::query::PresentationOf;

use crate::websocket::WsCommand::ParticipantVoted;
use util::use_case::UseCase;

#[derive(Clone, Deserialize, Debug)]
struct Command {
    #[serde(flatten)]
    command: WsCommand,
}

#[derive(Clone, Deserialize, Debug)]
enum WsCommand {
    ParticipantVoted { vote: u8 },
}

pub struct WebSocket {
    board_id: String,
    updates: Arc<dyn LoadUpdate<Vec<BoardModifiedEvent>, Key = String, Error = Error>>,
    use_case: Arc<UseCase<CombinedEvent>>,
    task_handle: Option<JoinHandle<()>>,
    id: String,
    name: String,
}

impl WebSocket {
    pub fn new(
        board_id: String,
        udpdates: Arc<dyn LoadUpdate<Vec<BoardModifiedEvent>, Key = String, Error = Error>>,
        use_case: Arc<UseCase<CombinedEvent>>,
        name: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            board_id,
            updates: udpdates,
            use_case,
            task_handle: None,
            name,
        }
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
                            log::debug!("Sent {} events", updates.len());
                            log::debug!("last event: {} ", last_event);
                            updates.iter().for_each(|event| {
                                log::debug!("event: {:?}", event);
                            });
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
enum ServerMessage {
    QueryUpdated(BoardPresentation),
    CommandResult(Vec<BoardModifiedEvent>),
    Error(String),
}

impl ServerMessage {
    fn send_to(self, addr: Recipient<ServerMessage>) {
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
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
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
                        tokio::spawn(async move {
                            use_case
                                .execute(&key, &command)
                                .await
                                .map(ServerMessage::CommandResult)
                                .unwrap_or_else(|err| {
                                    log::error!("Error: {:?}", err);
                                    ServerMessage::Error(
                                        "There was an error processing your command.".to_string(),
                                    )
                                })
                                .send_to(addr);
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
        let addr = ctx.address();
        let updates = self.updates.clone();
        let board_id = self.board_id.clone();

        let id = self.id.clone();
        let name = self.name.clone();
        let use_case = self.use_case.clone();

        let handle = tokio::spawn(async move {
            let add_participant = add_participant(name, id);
            if let Ok(event) = use_case
                .execute(&board_id, &add_participant)
                .await
                .or_else(|err| {
                    log::error!("Error Adding Participant: {:?}", err);
                    Err(err)
                })
            {
                Self::update_loop(addr, updates, &board_id).await;
            }
        });

        self.task_handle = Some(handle);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        let id = self.id.clone();
        let use_case = self.use_case.clone();
        let board_id = self.board_id.clone();

        let handle = tokio::spawn(async move {
            let remove_participant = remove_participant(id.clone());
            if let Err(err) = use_case.execute(&board_id, &remove_participant).await {
                log::error!("Error Removing Participant: {:?}", err);
            }
        });
    }
}
