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

impl<T, U> UpdateWith<Vec<U>> for Vec<T>
where
    T: Clone,
    U: Clone + Into<T>,
{
    type UpdateResponse = Vec<U>;
    fn update_with(&mut self, update_value: Vec<U>) -> Self::UpdateResponse {
        let transformed_update_values: Vec<T> =
            update_value.clone().into_iter().map(|e| e.into()).collect();
        self.extend(transformed_update_values);
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

    pub async fn execute<Cmd>(
        &self,
        key: &str,
        command: &Cmd,
    ) -> Result<<Vec<T> as UpdateWith<Vec<Cmd::Event>>>::UpdateResponse, Box<dyn Error + Send + Sync>>
    where
        Cmd: Command,
        Cmd::Event: Into<T>,
        Cmd::Entity: EventSourced<Event = T>,
        Vec<T>: NormaliseTo<Cmd::Entity> + UpdateWith<Vec<Cmd::Event>>,
    {
        let operation = |input: &Cmd::Entity| command.apply(input);
        let result = self.transaction.execute(key, &operation).await;
        result
    }
}
