use crate::command::Command;
use crate::entity::EventSourced;
use crate::transaction::{NormaliseTo, Transaction, UpdateWith};
use std::error::Error;

impl<T, U> NormaliseTo<U> for Vec<T>
where
    U: EventSourced<Event = T>,
{
    fn render_normalised(&self) -> U {
        U::source(self)
    }
}

impl<T> UpdateWith<Vec<T>> for Vec<T>
where
    T: Clone,
{
    type UpdateResponse = Vec<T>;
    fn update_with(&mut self, update_value: Vec<T>) -> Self::UpdateResponse {
        self.extend(update_value.clone());
        update_value
    }
}

pub struct UseCase<T> {
    transaction: Transaction<Vec<T>>,
}

impl<T> UseCase<T>
where
    T: Clone,
{
    pub fn new(transaction: Transaction<Vec<T>>) -> Self {
        Self { transaction }
    }

    pub async fn execute<Cmd, R>(
        &self,
        key: &str,
        command: &Cmd,
    ) -> Result<R, Box<dyn Error + Send + Sync>>
    where
        Cmd: Command<Event = T>,
        Cmd::Entity: EventSourced<Event = T>,
        Vec<T>: NormaliseTo<Cmd::Entity> + UpdateWith<Vec<Cmd::Event>, UpdateResponse = R>,
    {
        let operation = |input: &Cmd::Entity| command.apply(input);
        let result = self.transaction.execute(key, &operation).await;
        result
    }
}
