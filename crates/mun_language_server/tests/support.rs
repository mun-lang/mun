use async_std::future::timeout;
use futures::{SinkExt, StreamExt};
use lsp_types::{notification::Exit, request::Shutdown};
use mun_language_server::protocol::{Connection, Message, Notification, Request};
use mun_language_server::{main_loop, Config};
use paths::AbsPathBuf;
use serde::Serialize;
use serde_json::Value;
use std::convert::TryFrom;
use std::time::Duration;

/// An object that runs the language server main loop and enables sending and receiving messages
/// to and from it.
pub struct Server {
    next_request_id: u64,
    worker: Option<std::thread::JoinHandle<()>>,
    client: Connection,
    _temp_path: tempdir::TempDir,
}

impl Server {
    /// Constructs and initializes a new `Server`
    pub fn new() -> Self {
        let (connection, client) = Connection::memory();

        let temp_path = tempdir::TempDir::new("mun_language_server")
            .expect("unable to create temporary directory");

        let config = Config::new(
            AbsPathBuf::try_from(temp_path.path().to_path_buf())
                .expect("temp_path is not an absolute path"),
        );
        let worker = std::thread::spawn(move || {
            async_std::task::block_on(async move {
                main_loop(connection, config).await.unwrap();
            })
        });

        Self {
            next_request_id: Default::default(),
            worker: Some(worker),
            client,
            _temp_path: temp_path,
        }
    }

    /// Sends a request to the main loop and expects the specified value to be returned
    async fn assert_request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        expected_response: Value,
    ) where
        R::Params: Serialize,
    {
        let result = self.send_request::<R>(params).await;
        assert_eq!(result, expected_response);
    }

    /// Sends a request to main loop, returning the response
    async fn send_request<R: lsp_types::request::Request>(&mut self, params: R::Params) -> Value
    where
        R::Params: Serialize,
    {
        let id = self.next_request_id;
        self.next_request_id += 1;

        let r = Request::new(id.into(), R::METHOD.to_string(), params);
        self.send_and_receive(r).await
    }

    /// Sends an LSP notification to the main loop.
    async fn notification<N: lsp_types::notification::Notification>(&mut self, params: N::Params)
    where
        N::Params: Serialize,
    {
        let r = Notification::new(N::METHOD.to_string(), params);
        self.send_notification(r).await
    }

    /// Sends a server notification to the main loop
    async fn send_notification(&mut self, not: Notification) {
        self.client
            .sender
            .send(Message::Notification(not))
            .await
            .unwrap();
    }

    /// Sends a request to the main loop and receives its response
    async fn send_and_receive(&mut self, r: Request) -> Value {
        let id = r.id.clone();
        self.client.sender.send(r.into()).await.unwrap();
        while let Some(msg) = self.recv().await {
            match msg {
                Message::Request(req) => panic!(
                    "did not expect a request as a response to a request: {:?}",
                    req
                ),
                Message::Notification(_) => (),
                Message::Response(res) => {
                    assert_eq!(res.id, id);
                    if let Some(err) = res.error {
                        panic!(
                            "received error response as a response to a request: {:#?}",
                            err
                        );
                    }
                    return res.result.unwrap();
                }
            }
        }
        panic!("did not receive a response to our request");
    }

    /// Receives a message from the message or timeout.
    async fn recv(&mut self) -> Option<Message> {
        let duration = Duration::from_secs(60);
        timeout(duration, self.client.receiver.next())
            .await
            .unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Send a shutdown request
        async_std::task::block_on(async {
            // Send the proper shutdown sequence to ensure the main loop terminates properly
            self.assert_request::<Shutdown>((), Value::Null).await;
            self.notification::<Exit>(()).await;

            // Cancel the main_loop
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        });
    }
}
