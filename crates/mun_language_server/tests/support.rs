use crossbeam_channel::{after, select};
use lsp_server::{Connection, Message, Notification, Request};
use lsp_types::{notification::Exit, request::Shutdown};
use mun_language_server::{main_loop, Config};
use paths::AbsPathBuf;
use serde::Serialize;
use serde_json::Value;
use std::convert::TryFrom;
use std::time::Duration;

/// An object that runs the language server main loop and enables sending and receiving messages
/// to and from it.
pub struct Server {
    next_request_id: i32,
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
            main_loop(connection, config).unwrap();
        });

        Self {
            next_request_id: Default::default(),
            worker: Some(worker),
            client,
            _temp_path: temp_path,
        }
    }

    /// Sends a request to the main loop and expects the specified value to be returned
    fn assert_request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        expected_response: Value,
    ) where
        R::Params: Serialize,
    {
        let result = self.send_request::<R>(params);
        assert_eq!(result, expected_response);
    }

    /// Sends a request to main loop, returning the response
    fn send_request<R: lsp_types::request::Request>(&mut self, params: R::Params) -> Value
    where
        R::Params: Serialize,
    {
        let id = self.next_request_id;
        self.next_request_id += 1;

        let r = Request::new(id.into(), R::METHOD.to_string(), params);
        self.send_and_receive(r)
    }

    /// Sends an LSP notification to the main loop.
    fn notification<N: lsp_types::notification::Notification>(&mut self, params: N::Params)
    where
        N::Params: Serialize,
    {
        let r = Notification::new(N::METHOD.to_string(), params);
        self.send_notification(r)
    }

    /// Sends a server notification to the main loop
    fn send_notification(&mut self, not: Notification) {
        self.client.sender.send(Message::Notification(not)).unwrap();
    }

    /// Sends a request to the main loop and receives its response
    fn send_and_receive(&mut self, r: Request) -> Value {
        let id = r.id.clone();
        self.client.sender.send(r.into()).unwrap();
        while let Some(msg) = self.recv() {
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
    fn recv(&mut self) -> Option<Message> {
        let timeout = Duration::from_secs(120);
        select! {
            recv(self.client.receiver) -> msg => msg.ok(),
            recv(after(timeout)) -> _ => panic!("timed out"),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Send the proper shutdown sequence to ensure the main loop terminates properly
        self.assert_request::<Shutdown>((), Value::Null);
        self.notification::<Exit>(());

        // Cancel the main_loop
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
