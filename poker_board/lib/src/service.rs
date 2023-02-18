use crate::domain;
use crate::domain::add_participant::AddParticipantCommand;
use crate::domain::Board;
use crate::event::BoardModifiedEvent;
use crate::port::{LoadError, LoadEventsPort, PersistableEvent, PortError};
use async_trait::async_trait;
use util::{CommandDto, FromEventStream, HandleCommand, UseCase};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
struct AddParticipantDto {
    participant_name: String,
}

impl AddParticipantDto {
    pub fn new(participant_name: String) -> Self {
        Self { participant_name }
    }
}

struct Service<Command> {
    phantom: std::marker::PhantomData<Command>,
    load_port: Box<dyn LoadEventsPort>,
}

#[async_trait]
impl<Command> UseCase for Service<Command>
where
    Command: Send + Sync,
    Board: FromEventStream + HandleCommand<Command, Event = BoardModifiedEvent> + Send + Sync,
{
    type Error = PortError;
    type Command = Command;

    async fn execute(&self, command_dto: CommandDto<Self::Command>) -> Result<(), Self::Error> {
        let persistable = self.load_port.load_events(&command_dto.entity).await?;

        let board = Board::from_event_stream(command_dto.entity, persistable.events());
        let events = board.execute(command_dto.command);

        persistable
            .with_events(events)
            .persist()
            .await
            .map_err(PortError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::*;
    use mockall::{mock, predicate};
    use std::pin::Pin;
    use uuid::Variant::Future;

    mock! {
        pub LoadEventsAdapter {
            fn internal_load_events(&self, entity: &String) -> Result<Box<dyn PersistableEvent<BoardModifiedEvent>>, LoadError>;
        }

    }

    mock! {
        pub PersistableEventAdapter {
            fn internal_persist(&self) -> Result<(), SaveError>;
            fn internal_events(&self) -> Vec<BoardModifiedEvent>;
            fn internal_with_events(&self, events: Vec<BoardModifiedEvent>) -> Box<dyn PersistableEvent<BoardModifiedEvent>>;
        }
    }

    #[async_trait]
    impl LoadEventsPort for MockLoadEventsAdapter {
        async fn load_events(
            &self,
            entity: &String,
        ) -> Result<Box<dyn PersistableEvent<BoardModifiedEvent>>, LoadError> {
            self.internal_load_events(entity)
        }
    }

    #[async_trait]
    impl PersistableEvent<BoardModifiedEvent> for MockPersistableEventAdapter {
        async fn persist(&self) -> Result<(), SaveError> {
            self.internal_persist()
        }

        fn events(&self) -> Vec<BoardModifiedEvent> {
            self.internal_events()
        }

        fn with_events(
            self: Box<Self>,
            events: Vec<BoardModifiedEvent>,
        ) -> Box<dyn PersistableEvent<BoardModifiedEvent>> {
            self.internal_with_events(events)
        }
    }

    #[tokio::test]
    pub async fn it_should_persist_changed_events() {
        let mut mock_load_events_adapter = MockLoadEventsAdapter::new();
        let mut mock_persistable_event_adapter = MockPersistableEventAdapter::new();
        let mut mock_persistable_event_adapter2 = MockPersistableEventAdapter::new();

        let id = "test-id".to_string();
        let participant_name = "participant_name".to_string();
        let add_participant_command = AddParticipantCommand::new(participant_name.clone());

        let correct_participant_added = predicate::function(|events: &Vec<BoardModifiedEvent>| {
            if let Some(BoardModifiedEvent::ParticipantAdded {
                participant_name, ..
            }) = events.get(0)
            {
                return &participant_name == &participant_name;
            }
            false
        });

        mock_persistable_event_adapter2
            .expect_internal_persist()
            .times(1)
            .return_once(move || Ok(()));

        mock_persistable_event_adapter
            .expect_internal_events()
            .times(1)
            .return_once(move || vec![]);

        mock_persistable_event_adapter
            .expect_internal_with_events()
            .with(correct_participant_added)
            .times(1)
            .return_once(move |events| Box::new(mock_persistable_event_adapter2));

        mock_load_events_adapter
            .expect_internal_load_events()
            .with(predicate::eq(id.to_string()))
            .times(1)
            .return_once(move |_| Ok(Box::new(mock_persistable_event_adapter)));

        let service = Service::<AddParticipantCommand> {
            phantom: std::marker::PhantomData,
            load_port: Box::new(mock_load_events_adapter),
        };

        let command_dto = CommandDto::new(id.to_string(), add_participant_command);
        let result = service.execute(command_dto).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    pub async fn it_should_return_error_when_persisting_fails() {
        let mut mock_load_events_adapter = MockLoadEventsAdapter::new();
        let mut mock_persistable_event_adapter = MockPersistableEventAdapter::new();

        let id = "test-id".to_string();
        let participant_name = "participant_name".to_string();
        let add_participant_command = AddParticipantCommand::new(participant_name.clone());

        mock_persistable_event_adapter
            .expect_internal_events()
            .times(1)
            .return_once(move || vec![]);

        mock_persistable_event_adapter
            .expect_internal_with_events()
            .times(1)
            .return_once(move |events| {
                Box::new({
                    let mut mock = MockPersistableEventAdapter::new();
                    mock.expect_internal_persist()
                        .times(1)
                        .return_once(move || Err(SaveError::ConnectionError));
                    mock
                })
            });

        mock_load_events_adapter
            .expect_internal_load_events()
            .with(predicate::eq(id.to_string()))
            .times(1)
            .return_once(move |_| Ok(Box::new(mock_persistable_event_adapter)));

        let service = Service::<AddParticipantCommand> {
            phantom: std::marker::PhantomData,
            load_port: Box::new(mock_load_events_adapter),
        };

        let command_dto = CommandDto::new(id.to_string(), add_participant_command);
        let result = service.execute(command_dto).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PortError::SaveError(SaveError::ConnectionError)
        ));
    }
}
