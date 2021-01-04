use super::{Message, ProtocolError, Request, RequestId, Response};
use async_std::future::{timeout, TimeoutError};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use std::time::Duration;

/// Represents a connection between a language server server and a language server client.
pub struct Connection {
    pub sender: mpsc::Sender<Message>,
    pub receiver: mpsc::Receiver<Message>,
}

impl Connection {
    /// Creates a connection that communicates over stdout and stdin. This enables inter-process
    /// communication.
    pub fn stdio() -> Connection {
        let (sender, receiver) = super::stdio::stdio_transport();
        Connection { sender, receiver }
    }

    /// Creates a pair of connected connections. This enables in-process communication, especially
    /// useful for testing.
    pub fn memory() -> (Connection, Connection) {
        let (s1, r1) = mpsc::channel(1);
        let (s2, r2) = mpsc::channel(1);
        (
            Connection {
                sender: s1,
                receiver: r2,
            },
            Connection {
                sender: s2,
                receiver: r1,
            },
        )
    }

    /// Starts the initialization process by waiting for an initialize request from the client.
    pub async fn initialize_start(
        &mut self,
    ) -> Result<(RequestId, serde_json::Value), ProtocolError> {
        let req = match self.receiver.next().await {
            Some(Message::Request(req)) => {
                if req.is_initialize() {
                    req
                } else {
                    return Err(ProtocolError::UnexpectedMessage {
                        expected: "initialize".to_owned(),
                        received: Some(Message::Request(req)),
                    });
                }
            }
            msg => {
                return Err(ProtocolError::UnexpectedMessage {
                    expected: "initialize".to_owned(),
                    received: msg,
                })
            }
        };
        Ok((req.id, req.params))
    }

    /// Finishes the initialization process by sending an `InitializeResult` to the client
    pub async fn initialize_finish(
        &mut self,
        initialize_id: RequestId,
        initialize_result: serde_json::Value,
    ) -> Result<(), ProtocolError> {
        let resp = Response::new_ok(initialize_id, initialize_result);
        self.sender.send(resp.into()).await.unwrap();
        match self.receiver.next().await {
            Some(Message::Notification(n)) if n.is_initialized() => (),
            m => {
                return Err(ProtocolError::UnexpectedMessage {
                    expected: "initialized".to_owned(),
                    received: m,
                })
            }
        };
        Ok(())
    }

    /// If `req` is a `Shutdown`, responds to it and returns `true`, otherwise returns `false`.
    pub async fn handle_shutdown(&mut self, req: &Request) -> Result<bool, ProtocolError> {
        if !req.is_shutdown() {
            return Ok(false);
        }
        let resp = Response::new_ok(req.id.clone(), ());
        let _ = self.sender.send(resp.into()).await;
        match timeout(Duration::from_secs(30), self.receiver.next()).await {
            Ok(Some(Message::Notification(n))) if n.is_exit() => {}
            Err(TimeoutError { .. }) => {
                return Err(ProtocolError::Timeout {
                    waiting_for: "exit".to_owned(),
                })
            }
            Ok(m) => {
                return Err(ProtocolError::UnexpectedMessage {
                    expected: "exit".to_owned(),
                    received: m,
                })
            }
        }
        Ok(true)
    }
}
