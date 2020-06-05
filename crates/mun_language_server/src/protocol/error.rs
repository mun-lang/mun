use super::Message;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ProtocolError {
    #[error("expected '{expected}' request, got '{received:?}'")]
    UnexpectedMessage {
        expected: String,
        received: Option<Message>,
    },

    #[error("timeout while waiting for {waiting_for}")]
    Timeout { waiting_for: String },
}
