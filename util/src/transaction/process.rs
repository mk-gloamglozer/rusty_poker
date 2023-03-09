use crate::transaction::normalise_to::NormaliseTo;
use crate::transaction::operation::Operation;
use crate::transaction::update_with::UpdateWith;

pub fn process<V, R, U>(
    mut input: V,
    operation: &impl Operation<R, U>,
) -> ProcessResult<V, V::UpdateResponse>
where
    V: NormaliseTo<R> + UpdateWith<U>,
{
    let normalised = input.render_normalised();
    let update_response = operation.operate_on(&normalised);
    let update_result = input.update_with(update_response);
    ProcessResult {
        value: input,
        update_response: update_result,
    }
}

pub struct ProcessResult<V, U> {
    pub value: V,
    pub update_response: U,
}

#[cfg(test)]
mod tests {
    use super::*;

    impl NormaliseTo<i32> for i32 {
        fn render_normalised(&self) -> i32 {
            self.clone()
        }
    }

    impl UpdateWith<i32> for i32 {
        type UpdateResponse = i32;
        fn update_with(&mut self, update_value: i32) -> Self::UpdateResponse {
            *self = update_value;
            *self
        }
    }

    #[test]
    fn test_process() {
        let input = 0;
        let operation = |input: &i32| input + 1;
        let result = process(input, &operation);
        assert_eq!(result.value, 1);
        assert_eq!(result.update_response, 1);
    }
}
