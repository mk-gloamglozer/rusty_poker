use std::fmt::Display;

mod message;
pub mod sidecar;
pub mod store;
pub mod websocket;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn boxed_error<E>(error: E) -> Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    Box::new(error)
}

#[derive(Debug)]
struct BasicError {
    message: String,
}

impl Display for BasicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BasicError {}

pub fn as_basic_error<E>(error: E) -> Error
where
    E: std::error::Error,
{
    Box::new(BasicError {
        message: error.to_string(),
    })
}
