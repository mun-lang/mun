#![allow(dead_code)]

use async_std::future::timeout;
use futures::{SinkExt, StreamExt};
use lsp_types::{notification::Exit, request::Shutdown, Url};
use mun_language_server::{
    main_loop,
    protocol::{Connection, Message, Notification, Request},
    Config, FilesWatcher,
};
use mun_test::Fixture;
use serde::Serialize;
use serde_json::Value;
use std::{fs, time::Duration};

/// A `Project` represents a project that a language server can work with. Call the `server` method
/// to instantiate a language server that will serve information about the project.
pub struct Project<'a> {
    fixture: &'a str,
    tmp_dir: Option<tempdir::TempDir>,
}

impl<'a> Project<'a> {
    /// Construct a project from a fixture.
    pub fn with_fixture(fixture: &str) -> Project {
        Project {
            fixture,
            tmp_dir: None,
        }
    }

    /// Instantiate a language server for this project.
    pub fn server(self) -> Server {
        // Get or create a temporary directory
        let tmp_dir = self
            .tmp_dir
            .unwrap_or_else(|| tempdir::TempDir::new("testdir").unwrap());

        // Write all fixtures to a folder
        for entry in Fixture::parse(self.fixture) {
            let path = entry.relative_path.to_path(tmp_dir.path());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path.as_path(), entry.text.as_bytes()).unwrap();
        }

        // Construct a default configuration for the server
        let tmp_dir_path = tmp_dir.path().to_path_buf();
        let config = Config {
            watcher: FilesWatcher::Client,
            workspace_roots: vec![tmp_dir_path],
            ..Config::default()
        };

        // TODO: Provide the ability to modify the configuration externally

        Server::new(tmp_dir, config)
    }
}

/// An object that runs the language server main loop and enables sending and receiving messages
/// to and from it.
pub struct Server {
    next_request_id: u64,
    worker: Option<std::thread::JoinHandle<()>>,
    client: Connection,
    tmp_dir: tempdir::TempDir,
}

impl Server {
    /// Constructs and initializes a new `Server`
    pub fn new(tmp_dir: tempdir::TempDir, config: Config) -> Self {
        let (connection, client) = Connection::memory();

        let worker = std::thread::spawn(move || {
            async_std::task::block_on(async move {
                main_loop(connection, config).await.unwrap();
            })
        });

        Self {
            next_request_id: Default::default(),
            worker: Some(worker),
            client,
            tmp_dir,
        }
    }

    /// Returns the LSP TextDocumentIdentifier for the given path
    pub fn doc_id(&self, rel_path: &str) -> lsp_types::TextDocumentIdentifier {
        let path = self.tmp_dir.path().join(rel_path);
        lsp_types::TextDocumentIdentifier {
            uri: Url::from_file_path(path).unwrap(),
        }
    }

    /// Sends a request to the main loop and expects the specified value to be returned
    async fn assert_request_value<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        expected_response: Value,
    ) where
        R::Params: Serialize,
    {
        let result = self.send_request_for_value::<R>(params).await;
        assert_eq!(result, expected_response);
    }

    /// Sends a request to main loop, returning the response
    pub async fn send_request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
    ) -> R::Result {
        let value = self.send_request_for_value::<R>(params).await;
        serde_json::from_value(value).unwrap()
    }

    /// Sends a request to main loop, returning the response
    async fn send_request_for_value<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
    ) -> Value {
        let id = self.next_request_id;
        self.next_request_id += 1;

        let r = Request::new(id.into(), R::METHOD.to_string(), params);
        let value = self.send_and_receive(r).await;
        serde_json::from_value(value).unwrap()
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
                Message::Request(req) => match self.handle_request(req) {
                    Err(req) => {
                        panic!(
                            "did not expect a request as a response to a request: {:?}",
                            req
                        )
                    }
                    Ok(_) => continue,
                },
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

    /// Handles any known requests that we know the language server might send at any point. Returns
    /// a result with Err(Request) if the request could not be handled.
    fn handle_request(&mut self, request: Request) -> Result<(), Request> {
        if request.method == "client/registerCapability" {
            let params = request.params.to_string();
            if ["workspace/didChangeWatchedFiles"]
                .iter()
                .any(|&it| params.contains(it))
            {
                return Ok(());
            }
        }

        Err(request)
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
            self.assert_request_value::<Shutdown>((), Value::Null).await;
            self.notification::<Exit>(()).await;

            // Cancel the main_loop
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        });
    }
}
