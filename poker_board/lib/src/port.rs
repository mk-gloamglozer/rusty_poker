use crate::event::BoardModifiedEvent;
use async_trait::async_trait;

#[derive(Debug, PartialEq, Clone)]
pub enum ModifyError {
    ConnectionError(String),
    UnableToCompleteError(String),
    EventLogChangedError {
        original: Vec<BoardModifiedEvent>,
        actual: Vec<BoardModifiedEvent>,
    },
}

impl core::fmt::Display for ModifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModifyError::ConnectionError(msg) => write!(f, "ConnectionError: {}", msg),
            ModifyError::UnableToCompleteError(msg) => {
                write!(f, "UnableToCompleteError: {}", msg)
            }
            ModifyError::EventLogChangedError { original, actual } => write!(
                f,
                "EventLogChangedError: original: {:?}, actual: {:?}",
                original, actual
            ),
        }
    }
}

type ModifyFn<'a, T> = Box<dyn Fn(T) -> T + Send + Sync + 'a>;

pub struct Attempt<'a, T>
where
    T: Send + Sync,
{
    attempt_fn: ModifyFn<'a, T>,
}

impl<'a, T> Attempt<'a, T>
where
    T: Send + Sync + 'a,
{
    pub fn new(attempt_fn: impl Fn(T) -> T + Send + Sync + 'a) -> Self {
        Self {
            attempt_fn: Box::new(attempt_fn),
        }
    }

    pub fn attempt(&self, entity: T) -> T {
        (self.attempt_fn)(entity)
    }
}

#[async_trait]
pub trait ModifyEntityPort<'a, T>: Send + Sync
where
    T: Send + Sync,
{
    async fn modify_entity(
        &self,
        entity: String,
        attempt: Attempt<'a, T>,
    ) -> Result<(), ModifyError>;
}

#[async_trait]
pub trait GetEntityPort<T>: Send + Sync
where
    T: Send + Sync,
{
    async fn get_entity(&self, entity: String) -> Result<T, ModifyError>;
}
