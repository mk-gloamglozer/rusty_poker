use crate::websocket::{ServerMessage, UseCaseMessage};
use crate::{as_basic_error, Error};
use poker_board::command::event::CombinedEvent;
use std::sync::{Arc, Mutex};
use util::use_case::UseCase;

pub fn start_usecase_sidecar(
    use_case: Arc<UseCase<CombinedEvent>>,
) -> std::sync::mpsc::Sender<UseCaseMessage> {
    let (tx, rx) = std::sync::mpsc::channel::<UseCaseMessage>();

    tokio::spawn(async move {
        let rx = Arc::new(Mutex::new(rx));
        loop {
            let rx = rx.clone();
            match tokio::task::spawn_blocking(move || -> Result<UseCaseMessage, Error> {
                rx.lock()
                    .map_err(as_basic_error)?
                    .recv()
                    .map_err(as_basic_error)
            })
            .await
            {
                Ok(Ok(message)) => {
                    let UseCaseMessage {
                        board_id,
                        command,
                        receiver,
                    } = message;
                    use_case
                        .execute(&board_id, &command)
                        .await
                        .map(ServerMessage::CommandResult)
                        .unwrap_or_else(|err| {
                            log::error!("Error: {:?}", err);
                            ServerMessage::Error(
                                "There was an error processing your command.".to_string(),
                            )
                        })
                        .send_to(receiver);
                    log::info!("Command executed: {:?}", command)
                }
                Ok(_) => {
                    break;
                }
                Err(err) => {
                    log::error!("Error: {:?}", err);
                    break;
                }
            }
        }
    });

    tx
}
