use std::sync::Arc;
use std::time::Duration;

pub trait RetryStrategy {
    fn should_retry(
        &self,
        previous_instruction: &Option<Instruction>,
        retry_count: &u8,
    ) -> Instruction;
}

impl<T> RetryStrategy for T
where
    T: Fn(&Option<Instruction>, &u8) -> Instruction,
{
    fn should_retry(
        &self,
        previous_instruction: &Option<Instruction>,
        retry_count: &u8,
    ) -> Instruction {
        self(previous_instruction, retry_count)
    }
}

impl<T> RetryStrategy for Arc<T>
where
    T: RetryStrategy + ?Sized,
{
    fn should_retry(
        &self,
        previous_instruction: &Option<Instruction>,
        retry_count: &u8,
    ) -> Instruction {
        self.as_ref()
            .should_retry(previous_instruction, retry_count)
    }
}

pub struct RetryPolicyService {
    strategy: Arc<dyn RetryStrategy + Send + Sync>,
}

impl RetryPolicyService {
    pub fn new<T: RetryStrategy + Send + Sync + 'static>(strategy: T) -> Self {
        Self {
            strategy: Arc::new(strategy),
        }
    }

    pub fn generate_policy(&self) -> RetryPolicy {
        RetryPolicy::new(self.strategy.clone())
    }
}

pub struct RetryPolicy {
    strategy: Box<dyn RetryStrategy + Send>,
    retry_count: u8,
    instruction: Option<Instruction>,
}

impl RetryPolicy {
    fn new<T: RetryStrategy + Send + 'static>(strategy: T) -> Self {
        Self {
            strategy: Box::new(strategy),
            retry_count: 0,
            instruction: None,
        }
    }
}

impl RetryPolicy {
    pub fn retry(&mut self) -> Instruction {
        let instruction = self
            .strategy
            .should_retry(&self.instruction, &self.retry_count);
        self.retry_count += 1;
        self.instruction = Some(instruction);
        self.instruction.clone().unwrap()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    Retry(Duration),
    Abort,
}

#[cfg(test)]
mod test_retry_policy {
    use super::*;

    #[test]
    pub fn it_should_carry_retry_state_into_next_retry_call() {
        let mut policy =
            RetryPolicy::new(|previous_instruction: &Option<Instruction>, count: &u8| {
                if *count == 0 {
                    Instruction::Retry(Duration::from_secs(1))
                } else {
                    assert_eq!(
                        previous_instruction,
                        &Some(Instruction::Retry(Duration::from_secs(1)))
                    );
                    Instruction::Abort
                }
            });

        {
            let instruction = policy.retry();
            assert_eq!(instruction, Instruction::Retry(Duration::from_secs(1)));
        }

        let instruction_2 = policy.retry();
        assert_eq!(instruction_2, Instruction::Abort);
    }
}

#[cfg(test)]
mod test_retry_service {
    use super::*;

    #[test]
    pub fn it_should_generate_retry_policy() {
        let service =
            RetryPolicyService::new(|_previous_instruction: &Option<Instruction>, _count: &u8| {
                Instruction::Abort
            });

        let mut policy = service.generate_policy();

        let instruction = policy.retry();
        assert_eq!(instruction, Instruction::Abort);
    }
}
