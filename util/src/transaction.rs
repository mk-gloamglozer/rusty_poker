mod normalise_to;
mod operation;
mod process;
pub mod retry;
mod update_with;

use crate::store::{LoadEntity, SaveEntity};
use crate::transaction::process::process;
use crate::transaction::retry::{Instruction, RetryPolicyService, RetryStrategy};
pub use normalise_to::NormaliseTo;
pub use operation::Operation;
use std::error::Error;
pub use update_with::UpdateWith;

pub struct Transaction<V> {
    retry_policy_service: RetryPolicyService,
    write_store:
        Box<dyn SaveEntity<V, Key = String, Error = Box<dyn Error + Send + Sync + 'static>>>,
    read_store:
        Box<dyn LoadEntity<V, Key = String, Error = Box<dyn Error + Send + Sync + 'static>>>,
}

impl<V> Transaction<V> {
    pub fn new<T: RetryStrategy + Send + Sync + 'static>(
        retry_statergy: T,
        write_store: impl SaveEntity<V, Key = String, Error = Box<dyn Error + Send + Sync + 'static>>
            + 'static,
        read_store: impl LoadEntity<V, Key = String, Error = Box<dyn Error + Send + Sync + 'static>>
            + 'static,
    ) -> Self {
        Self {
            retry_policy_service: RetryPolicyService::new(retry_statergy),
            write_store: Box::new(write_store),
            read_store: Box::new(read_store),
        }
    }

    pub async fn execute<T, U>(
        &self,
        key: &str,
        operation: &impl Operation<T, U>,
    ) -> Result<V::UpdateResponse, Box<dyn Error + Send + Sync>>
    where
        V: NormaliseTo<T> + UpdateWith<U> + Default,
    {
        let mut retry_policy = self.retry_policy_service.generate_policy();
        loop {
            let result: Result<V::UpdateResponse, Box<dyn Error + Send + Sync>> =
                try_operation(&*self.read_store, &*self.write_store, key, operation).await;
            match result {
                Ok(result) => break Ok(result),
                Err(error) => {
                    let instruction = retry_policy.retry();
                    match instruction {
                        Instruction::Retry(delay) => {
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                        Instruction::Abort => break Err(error),
                    }
                }
            }
        }
    }
}

async fn try_operation<V, T, U, E>(
    load_entity: &(impl LoadEntity<V, Key = String, Error = E> + ?Sized),
    save_entity: &(impl SaveEntity<V, Key = String, Error = E> + ?Sized),
    key: &str,
    operation: &impl Operation<T, U>,
) -> Result<V::UpdateResponse, E>
where
    V: NormaliseTo<T> + UpdateWith<U> + Default,
{
    match load_entity
        .load(&key.into())
        .await
        .map(|value| value.unwrap_or_default())
        .map(|value| process(value, operation))
        .map(|process_result| async {
            save_entity
                .save(&key.into(), process_result.value)
                .await
                .map(|_| process_result.update_response)
        }) {
        Ok(result) => result.await,
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod test_try_operation {
    use crate::store::LoadEntity;
    use crate::store::SaveEntity;
    use crate::transaction::normalise_to::NormaliseTo;
    use crate::transaction::operation::Operation;
    use crate::transaction::try_operation;
    use crate::transaction::update_with::UpdateWith;
    use std::error::Error;

    struct TestEntity {
        value: String,
    }

    impl Default for TestEntity {
        fn default() -> Self {
            Self {
                value: "default".to_string(),
            }
        }
    }

    impl NormaliseTo<String> for TestEntity {
        fn render_normalised(&self) -> String {
            self.value.clone()
        }
    }

    impl UpdateWith<String> for TestEntity {
        type UpdateResponse = String;
        fn update_with(&mut self, update: String) -> Self::UpdateResponse {
            self.value = update;
            "update-response".to_string()
        }
    }

    struct TestOperation;

    impl Operation<String, String> for TestOperation {
        fn operate_on(&self, input: &String) -> String {
            input.to_string()
        }
    }

    struct TestLoadEntity;

    #[async_trait::async_trait]
    impl LoadEntity<TestEntity> for TestLoadEntity {
        type Key = String;
        type Error = Box<dyn Error + Send + Sync + 'static>;

        async fn load(
            &self,
            key: &String,
        ) -> Result<Option<TestEntity>, Box<dyn Error + Send + Sync + 'static>> {
            Ok(Some(TestEntity { value: key.clone() }))
        }
    }

    struct TestSaveEntity;

    #[async_trait::async_trait]
    impl SaveEntity<TestEntity> for TestSaveEntity {
        type Key = String;
        type Error = Box<dyn Error + Send + Sync + 'static>;

        async fn save(
            &self,
            _key: &String,
            value: TestEntity,
        ) -> Result<TestEntity, Box<dyn Error + Send + Sync + 'static>> {
            Ok(value)
        }
    }

    #[tokio::test]
    async fn it_should_load_perform_operation_and_save() {
        let load_entity = TestLoadEntity {};
        let save_entity = TestSaveEntity {};
        let result = try_operation(
            &load_entity,
            &save_entity,
            &"key".to_string(),
            &TestOperation,
        )
        .await;
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), "update-response".to_string());
    }

    #[tokio::test]
    async fn it_should_return_error_if_save_errors() {
        struct TestSaveEntityWithError;

        #[async_trait::async_trait]
        impl SaveEntity<TestEntity> for TestSaveEntityWithError {
            type Key = String;
            type Error = Box<dyn Error + Send + Sync + 'static>;

            async fn save(
                &self,
                _key: &String,
                _value: TestEntity,
            ) -> Result<TestEntity, Box<dyn Error + Send + Sync + 'static>> {
                Err("error".to_string().into())
            }
        }

        let load_entity = TestLoadEntity {};
        let save_entity = TestSaveEntityWithError {};
        let result = try_operation(
            &load_entity,
            &save_entity,
            &"key".to_string(),
            &TestOperation,
        )
        .await;
        assert_eq!(result.is_err(), true);
    }

    #[tokio::test]
    async fn it_should_return_an_error_if_load_errors() {
        struct TestLoadEntityWithError;

        #[async_trait::async_trait]
        impl LoadEntity<TestEntity> for TestLoadEntityWithError {
            type Key = String;
            type Error = Box<dyn Error + Send + Sync + 'static>;

            async fn load(
                &self,
                _key: &String,
            ) -> Result<Option<TestEntity>, Box<dyn Error + Send + Sync + 'static>> {
                Err("error".to_string().into())
            }
        }

        let load_entity = TestLoadEntityWithError {};
        let save_entity = TestSaveEntity {};
        let result = try_operation(
            &load_entity,
            &save_entity,
            &"key".to_string(),
            &TestOperation,
        )
        .await;
        assert_eq!(result.is_err(), true);
    }
}

#[cfg(test)]
mod test_transaction {
    use crate::store::LoadEntity;
    use crate::store::SaveEntity;
    use crate::transaction::normalise_to::NormaliseTo;
    use crate::transaction::operation::Operation;
    use crate::transaction::retry::Instruction;
    use crate::transaction::update_with::UpdateWith;
    use crate::transaction::Transaction;
    use std::error::Error;
    use std::sync::Mutex;
    use std::time::Duration;

    struct TestEntity {
        value: String,
    }

    impl Default for TestEntity {
        fn default() -> Self {
            Self {
                value: "default".to_string(),
            }
        }
    }

    impl NormaliseTo<String> for TestEntity {
        fn render_normalised(&self) -> String {
            self.value.clone()
        }
    }

    impl UpdateWith<String> for TestEntity {
        type UpdateResponse = String;
        fn update_with(&mut self, update: String) -> Self::UpdateResponse {
            self.value = update;
            "update-response".to_string()
        }
    }

    struct TestOperation;

    impl Operation<String, String> for TestOperation {
        fn operate_on(&self, _input: &String) -> String {
            "operation-result".to_string()
        }
    }

    struct TestLoadEntity;

    #[async_trait::async_trait]
    impl LoadEntity<TestEntity> for TestLoadEntity {
        type Key = String;
        type Error = Box<dyn Error + Send + Sync + 'static>;

        async fn load(
            &self,
            key: &String,
        ) -> Result<Option<TestEntity>, Box<dyn Error + Send + Sync + 'static>> {
            Ok(Some(TestEntity { value: key.clone() }))
        }
    }

    struct TestSaveEntity;

    #[async_trait::async_trait]
    impl SaveEntity<TestEntity> for TestSaveEntity {
        type Key = String;
        type Error = Box<dyn Error + Send + Sync + 'static>;

        async fn save(
            &self,
            _key: &String,
            value: TestEntity,
        ) -> Result<TestEntity, Box<dyn Error + Send + Sync + 'static>> {
            Ok(value)
        }
    }

    #[tokio::test]
    async fn it_should_load_perform_operation_and_save() {
        let load_entity = TestLoadEntity {};
        let save_entity = TestSaveEntity {};
        let retry_strategy = |_previous_instruction: &Option<Instruction>, _attempt: &u8| {
            Instruction::Retry(Duration::from_millis(0))
        };
        let transaction = Transaction::<TestEntity>::new(retry_strategy, save_entity, load_entity);

        let operation = TestOperation {};
        let result = transaction.execute(&"key".to_string(), &operation).await;
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), "update-response".to_string());
    }

    #[tokio::test]
    async fn it_should_return_error_if_save_errors_and_retry_strategy_says_stop() {
        struct TestSaveEntityWithError;

        #[async_trait::async_trait]
        impl SaveEntity<TestEntity> for TestSaveEntityWithError {
            type Key = String;
            type Error = Box<dyn Error + Send + Sync + 'static>;

            async fn save(
                &self,
                _key: &String,
                _value: TestEntity,
            ) -> Result<TestEntity, Box<dyn Error + Send + Sync + 'static>> {
                Err("error".to_string().into())
            }
        }

        let load_entity = TestLoadEntity {};
        let save_entity = TestSaveEntityWithError {};
        let retry_strategy =
            |_previous_instruction: &Option<Instruction>, _attempt: &u8| Instruction::Abort;
        let transaction = Transaction::<TestEntity>::new(retry_strategy, save_entity, load_entity);

        let operation = TestOperation {};
        let result = transaction.execute(&"key".to_string(), &operation).await;
        assert_eq!(result.is_err(), true);
        assert_eq!(result.unwrap_err().to_string(), "error".to_string());
    }

    #[tokio::test]
    async fn it_should_retry_if_save_errors_and_retry_strategy_says_retry() {
        struct TestSaveEntityWithError(Mutex<Counter>);
        struct Counter(u8);

        impl Counter {
            fn new() -> Self {
                Self(0)
            }

            fn increment(&mut self) {
                self.0 += 1;
            }

            fn value(&self) -> u8 {
                self.0
            }
        }

        #[async_trait::async_trait]
        impl SaveEntity<TestEntity> for TestSaveEntityWithError {
            type Key = String;
            type Error = Box<dyn Error + Send + Sync + 'static>;

            async fn save(
                &self,
                _key: &String,
                _value: TestEntity,
            ) -> Result<TestEntity, Box<dyn Error + Send + Sync + 'static>> {
                let mut count = self.0.lock().unwrap();
                if count.value() == 0 {
                    count.increment();
                    Err("error".to_string().into())
                } else {
                    Ok(TestEntity {
                        value: "saved".to_string(),
                    })
                }
            }
        }

        let load_entity = TestLoadEntity {};
        let save_entity = TestSaveEntityWithError(Mutex::new(Counter::new()));
        let retry_strategy = |_previous_instruction: &Option<Instruction>, _attempt: &u8| {
            Instruction::Retry(Duration::from_millis(0))
        };
        let transaction = Transaction::<TestEntity>::new(retry_strategy, save_entity, load_entity);

        let operation = TestOperation {};
        let result = transaction.execute(&"key".to_string(), &operation).await;
        assert_eq!(result.is_ok(), true);
    }
}
