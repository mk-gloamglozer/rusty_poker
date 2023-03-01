use crate::domain::Board;
use crate::event::BoardModifiedEvent;
use crate::port::{Attempt, GetEntityPort, ModifyEntityPort, ModifyError};
use async_trait::async_trait;
use util::{CommandDto, FromEventStream, HandleCommand, UseCase};

pub struct ModifyingService<'a, Command, Event> {
    phantom: std::marker::PhantomData<Command>,
    modify_port: Box<dyn ModifyEntityPort<'a, Vec<Event>>>,
}

impl<'a, Command, Event> ModifyingService<'a, Command, Event> {
    pub fn new(modify_port: Box<dyn ModifyEntityPort<'a, Vec<Event>>>) -> Self {
        Self {
            phantom: std::marker::PhantomData,
            modify_port,
        }
    }
}

#[async_trait]
impl<'a, Command> UseCase for ModifyingService<'a, Command, BoardModifiedEvent>
where
    Command: Send + Sync + Clone + 'a,
    Board: FromEventStream + HandleCommand<Command, Event = BoardModifiedEvent> + Send + Sync,
{
    type Error = ModifyError;
    type Command = Command;
    async fn execute(&self, command_dto: CommandDto<Command>) -> Result<(), Self::Error> {
        let entity = command_dto.entity.clone();
        let attempt = Attempt::<Vec<BoardModifiedEvent>>::new(move |events| {
            let mut initial_events = events.clone();
            let board = Board::from_event_stream(command_dto.entity.clone(), events);
            initial_events.extend(board.execute(command_dto.command.clone()));
            initial_events
        });

        self.modify_port.modify_entity(entity, attempt).await
    }
}

pub struct QueryingService<Event> {
    get_port: Box<dyn GetEntityPort<Vec<Event>>>,
}

#[cfg(test)]
mod test_modifying_service {
    use super::*;
    use crate::domain::add_participant::AddParticipantCommand;
    use mockall::{mock, predicate, PredicateBooleanExt};

    mock! {
        pub ModifyEntityAdapter {
            fn persist_entity(&self, entity: String, events: Vec<BoardModifiedEvent>) -> Result<(), ModifyError>;
            fn events(&self) -> Vec<BoardModifiedEvent>;
        }
    }

    #[async_trait]
    impl<'a> ModifyEntityPort<'a, Vec<BoardModifiedEvent>> for MockModifyEntityAdapter {
        async fn modify_entity(
            &self,
            entity: String,
            attempt: Attempt<'a, Vec<BoardModifiedEvent>>,
        ) -> Result<(), ModifyError> {
            self.persist_entity(entity, attempt.attempt(self.events()))
        }
    }

    #[tokio::test]
    pub async fn it_should_persist_updated_list_of_events() {
        let mut mock_modify_entity_adapter = MockModifyEntityAdapter::new();
        let id = "test-id".to_string();

        let participant_name = "participant_name".to_string();
        let add_participant_command = AddParticipantCommand::new(participant_name.clone());

        let correct_participant_added = predicate::function(|events: &Vec<BoardModifiedEvent>| {
            if let Some(BoardModifiedEvent::ParticipantAdded {
                participant_name, ..
            }) = events.get(1)
            {
                return &participant_name == &participant_name;
            }
            false
        });

        let has_two_events =
            predicate::function(|events: &Vec<BoardModifiedEvent>| events.len() == 2);

        mock_modify_entity_adapter
            .expect_events()
            .times(1)
            .return_once(move || {
                vec![BoardModifiedEvent::ParticipantAdded {
                    participant_id: "test-id".to_string(),
                    participant_name: "test-name".to_string(),
                }]
            });

        mock_modify_entity_adapter
            .expect_persist_entity()
            .with(
                predicate::eq(id.to_string()),
                has_two_events.and(correct_participant_added),
            )
            .times(1)
            .return_once(move |_, _| Ok(()));

        let service = ModifyingService::<AddParticipantCommand, BoardModifiedEvent> {
            phantom: std::marker::PhantomData,
            modify_port: Box::new(mock_modify_entity_adapter),
        };

        let command_dto = CommandDto::new(id.to_string(), add_participant_command);
        let result = service.execute(command_dto).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn it_should_return_error_when_persisting_fails() {
        let mut mock_modify_entity_adapter = MockModifyEntityAdapter::new();
        let id = "test-id".to_string();

        let participant_name = "participant_name".to_string();
        let add_participant_command = AddParticipantCommand::new(participant_name.clone());

        mock_modify_entity_adapter
            .expect_events()
            .times(1)
            .return_once(move || vec![]);

        mock_modify_entity_adapter
            .expect_persist_entity()
            .times(1)
            .return_once(|_, _| Err(ModifyError::ConnectionError("test".to_string())));

        let service = ModifyingService::<AddParticipantCommand, BoardModifiedEvent> {
            phantom: std::marker::PhantomData,
            modify_port: Box::new(mock_modify_entity_adapter),
        };

        let command_dto = CommandDto::new(id.to_string(), add_participant_command);
        let result = service.execute(command_dto).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ModifyError::ConnectionError(x) if x == "test"));
    }
}
