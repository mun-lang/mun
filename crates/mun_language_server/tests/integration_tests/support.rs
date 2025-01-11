use std::{
    cell::{Cell, RefCell},
    convert::TryInto,
    fs,
    time::Duration,
};

use crossbeam_channel::{after, select};
use lsp_server::{Connection, Message, Notification, Request};
use lsp_types::{
    notification::Exit, request::Shutdown, ProgressParams, ProgressParamsValue, Url,
    WorkDoneProgress,
};
use mun_hir_input::Fixture;
use mun_language_server::{main_loop, Config, FilesWatcher};
use mun_paths::AbsPathBuf;
use mun_project::ProjectManifest;
use serde::Serialize;
use serde_json::Value;

/// A `Project` represents a project that a language server can work with. Call
/// the [`server`] method to instantiate a language server that will serve
/// information about the project.
pub struct Project<'a> {
    fixture: &'a str,
    tmp_dir: Option<tempdir::TempDir>,
}

impl Project<'_> {
    /// Constructs a project from a fixture.
    pub fn with_fixture(fixture: &str) -> Project<'_> {
        Project {
            fixture,
            tmp_dir: None,
        }
    }

    /// Instantiates a language server for this project.
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

        let tmp_dir_path: AbsPathBuf = tmp_dir
            .path()
            .to_path_buf()
            .try_into()
            .expect("could not convert temp dir to absolute path");
        let roots = vec![tmp_dir_path.clone()];

        let discovered_projects = ProjectManifest::discover_all(roots.into_iter());

        // Construct a default configuration for the server
        let config = Config {
            discovered_projects: Some(discovered_projects),
            watcher: FilesWatcher::Client,
            ..Config::new(tmp_dir_path)
        };

        // TODO: Provide the ability to modify the configuration externally

        Server::new(tmp_dir, config)
    }
}

/// An object that runs the language server main loop and enables sending and
/// receiving messages to and from it.
pub struct Server {
    next_request_id: Cell<i32>,
    messages: RefCell<Vec<Message>>,
    worker: Option<std::thread::JoinHandle<()>>,
    client: Connection,
    tmp_dir: tempdir::TempDir,
}

impl Server {
    /// Constructs and initializes a new `Server`
    pub fn new(tmp_dir: tempdir::TempDir, config: Config) -> Self {
        let (connection, client) = Connection::memory();

        let worker = std::thread::spawn(move || {
            main_loop(connection, config).unwrap();
        });

        Self {
            next_request_id: Cell::new(1),
            messages: RefCell::new(Vec::new()),
            worker: Some(worker),
            client,
            tmp_dir,
        }
    }

    /// Returns the LSP `TextDocumentIdentifier` for the given path
    pub fn doc_id(&self, rel_path: &str) -> lsp_types::TextDocumentIdentifier {
        let path = self.tmp_dir.path().join(rel_path);
        lsp_types::TextDocumentIdentifier {
            uri: Url::from_file_path(path).unwrap(),
        }
    }

    /// Waits until all projects in the workspace have been loaded
    pub fn wait_until_workspace_is_loaded(self) -> Server {
        self.wait_for_message_cond(1, &|msg: &Message| match msg {
            Message::Notification(n) if n.method == "$/progress" => {
                matches!(n.clone().extract::<ProgressParams>("$/progress").unwrap(),
                    ProgressParams {
                        token: lsp_types::ProgressToken::String(ref token),
                        value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(_)),
                    } if token == "mun/projects scanned")
            }
            _ => false,
        });
        self
    }

    /// A function to wait for a specific message to arrive
    fn wait_for_message_cond(&self, n: usize, cond: &dyn Fn(&Message) -> bool) {
        let mut total = 0;
        for msg in self.messages.borrow().iter() {
            if cond(msg) {
                total += 1;
            }
        }
        while total < n {
            let msg = self.recv().expect("no response");
            if cond(&msg) {
                total += 1;
            }
        }
    }

    /// Sends a request to the main loop and expects the specified value to be
    /// returned
    fn assert_request_returns_value<R: lsp_types::request::Request>(
        &self,
        params: R::Params,
        expected_response: Value,
    ) where
        R::Params: Serialize,
    {
        let result = self.send_request_for_value::<R>(params);
        assert_eq!(result, expected_response);
    }

    /// Sends a request to the language server, returning the response
    pub fn send_request<R: lsp_types::request::Request>(&self, params: R::Params) -> R::Result {
        let value = self.send_request_for_value::<R>(params);
        serde_json::from_value(value).unwrap()
    }

    /// Sends a request to main loop, returning the response
    fn send_request_for_value<R: lsp_types::request::Request>(&self, params: R::Params) -> Value
    where
        R::Params: Serialize,
    {
        let id = self.next_request_id.get();
        self.next_request_id.set(id.wrapping_add(1));

        let r = Request::new(id.into(), R::METHOD.to_string(), params);
        self.send_and_receive(r)
    }

    /// Sends an LSP notification to the main loop.
    fn notification<N: lsp_types::notification::Notification>(&self, params: N::Params)
    where
        N::Params: Serialize,
    {
        let r = Notification::new(N::METHOD.to_string(), params);
        self.send_notification(r);
    }

    /// Sends a server notification to the main loop
    fn send_notification(&self, not: Notification) {
        self.client.sender.send(Message::Notification(not)).unwrap();
    }

    /// Sends a request to the main loop and receives its response
    fn send_and_receive(&self, r: Request) -> Value {
        let id = r.id.clone();
        self.client.sender.send(r.into()).unwrap();
        while let Some(msg) = self.recv() {
            match msg {
                Message::Request(req) => {
                    panic!("did not expect a request as a response to a request: {req:?}")
                }
                Message::Notification(_) => (),
                Message::Response(res) => {
                    assert_eq!(res.id, id);
                    if let Some(err) = res.error {
                        panic!("received error response as a response to a request: {err:#?}");
                    }
                    return res.result.unwrap();
                }
            }
        }
        panic!("did not receive a response to our request");
    }

    /// Receives a message from the message or timeout.
    fn recv(&self) -> Option<Message> {
        let timeout = Duration::from_secs(120);
        let msg = select! {
            recv(self.client.receiver) -> msg => msg.ok(),
            recv(after(timeout)) -> _ => panic!("timed out"),
        };
        if let Some(ref msg) = msg {
            self.messages.borrow_mut().push(msg.clone());
        }
        msg
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // Send the proper shutdown sequence to ensure the main loop terminates properly
        self.assert_request_returns_value::<Shutdown>((), Value::Null);
        self.notification::<Exit>(());

        // Cancel the main_loop
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
