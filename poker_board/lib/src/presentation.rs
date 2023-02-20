use crate::domain::{add_participant, clear_votes};
use actix_web::HttpResponse;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use util::{CommandDto, UseCase};

pub trait CommandDeserializer: Send + Sync {
    type Command;
    fn deserialize_command(&self, command: String) -> Result<Self::Command, String>;
}

trait EntityCommand {
    fn entity_id(&self) -> String;
}

#[async_trait]
pub trait CommandHandler<Command>: Send + Sync {
    async fn handle_command(&self, command: Command) -> Result<(), String>;
}

#[async_trait]
impl<T, U, C, E> CommandHandler<U> for T
where
    U: Into<CommandDto<C>> + Send + Sync + 'static,
    T: UseCase<Command = C, Error = E>,
    C: Send + Sync,
    E: Display,
{
    async fn handle_command(&self, command: U) -> Result<(), String> {
        self.execute(command.into())
            .await
            .map_err(|e| e.to_string())
    }
}

impl<F, T, E> CommandDeserializer for F
where
    F: Fn(String) -> Result<T, E> + Send + Sync + 'static,
    E: Display,
{
    type Command = T;
    fn deserialize_command(&self, command: String) -> Result<Self::Command, String> {
        (self)(command).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearVotesDto {
    entity_id: String,
}

impl Into<CommandDto<clear_votes::ClearVotes>> for ClearVotesDto {
    fn into(self) -> CommandDto<clear_votes::ClearVotes> {
        CommandDto::new(self.entity_id.to_string(), clear_votes::ClearVotes {})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AddParticipantDto {
    entity_id: String,
    name: String,
}

impl Into<CommandDto<add_participant::AddParticipantCommand>> for AddParticipantDto {
    fn into(self) -> CommandDto<add_participant::AddParticipantCommand> {
        CommandDto::new(
            self.entity_id,
            add_participant::AddParticipantCommand::new(self.name),
        )
    }
}

pub struct Controller<Command> {
    deserializer: Box<dyn CommandDeserializer<Command = Command>>,
    handler: Box<dyn CommandHandler<Command>>,
}

impl<Command> Controller<Command> {
    pub fn new(
        handler: Box<dyn CommandHandler<Command>>,
        deserializer: Box<dyn CommandDeserializer<Command = Command>>,
    ) -> Self {
        Self {
            handler,
            deserializer,
        }
    }

    pub async fn handle(&self, req_body: String) -> HttpResponse {
        match self
            .deserializer
            .deserialize_command(req_body)
            .map_err(|e| HttpResponse::BadRequest().body(e.to_string()))
            .map(|dto| self.handler.handle_command(dto))
            .map(|result| async {
                match result.await {
                    Ok(_) => Ok(HttpResponse::Ok().finish()),
                    Err(e) => Err(HttpResponse::InternalServerError().body(e.to_string())),
                }
            }) {
            Ok(result) => result.await.unwrap_or_else(|e| e),
            Err(e) => e,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::{mock, predicate};

    mock! {
        pub AddParticipantUseCase{
            fn execute_internal(&self, command: CommandDto<add_participant::AddParticipantCommand>) -> Result<(), String>;
        }
    }

    #[async_trait]
    impl UseCase for MockAddParticipantUseCase {
        type Error = String;
        type Command = add_participant::AddParticipantCommand;

        async fn execute(&self, command: CommandDto<Self::Command>) -> Result<(), Self::Error> {
            self.execute_internal(command)
        }
    }

    #[tokio::test]
    pub async fn it_should_handle_add_participant_request() {
        let mut mock_add_participant_use_case = MockAddParticipantUseCase::new();
        mock_add_participant_use_case
            .expect_execute_internal()
            .with(predicate::eq(CommandDto::new(
                "test-id".to_string(),
                add_participant::AddParticipantCommand::new("test-name".to_string()),
            )))
            .returning(|_| Ok(()));

        let deserializer =
            Box::new(|req_body: String| serde_json::from_str(&req_body).map_err(|e| e.to_string()));

        let controller: Controller<AddParticipantDto> =
            Controller::new(Box::new(mock_add_participant_use_case), deserializer);

        let req_body = r#"{"entity_id": "test-id", "name": "test-name"}"#.to_string();

        let response = controller.handle(req_body).await;

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    pub async fn it_should_return_bad_request_when_add_participant_request_is_invalid() {
        let mut mock_add_participant_use_case = MockAddParticipantUseCase::new();
        mock_add_participant_use_case
            .expect_execute_internal()
            .returning(|_| Ok(()));

        let deserializer =
            Box::new(|req_body: String| serde_json::from_str(&req_body).map_err(|e| e.to_string()));

        let controller: Controller<AddParticipantDto> =
            Controller::new(Box::new(mock_add_participant_use_case), deserializer);

        let req_body = r#"{"entity_id": "test-id"}"#.to_string();

        let response = controller.handle(req_body).await;

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    pub async fn it_should_return_server_error_if_add_participant_use_case_fails() {
        let mut mock_add_participant_use_case = MockAddParticipantUseCase::new();
        mock_add_participant_use_case
            .expect_execute_internal()
            .returning(|_| Err("test-error".to_string()));

        let deserializer =
            Box::new(|req_body: String| serde_json::from_str(&req_body).map_err(|e| e.to_string()));

        let controller: Controller<AddParticipantDto> =
            Controller::new(Box::new(mock_add_participant_use_case), deserializer);

        let req_body = r#"{"entity_id": "test-id", "name": "test-name"}"#.to_string();

        let response = controller.handle(req_body).await;

        assert_eq!(response.status(), 500);
    }
}
