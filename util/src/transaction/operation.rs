pub trait Operation<T, U> {
    fn operate_on(&self, input: &T) -> U;
}

impl<T, R, U> Operation<R, U> for T
where
    T: Fn(&R) -> U,
{
    fn operate_on(&self, input: &R) -> U {
        self(input)
    }
}
