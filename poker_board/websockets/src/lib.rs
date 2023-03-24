mod message;
pub mod store;
pub mod websocket;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
