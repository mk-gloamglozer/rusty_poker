pub trait UpdateWith<T> {
    type UpdateResponse;
    fn update_with(&mut self, update_value: T) -> Self::UpdateResponse;
}
