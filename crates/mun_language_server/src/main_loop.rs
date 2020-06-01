use crate::config::{Config, FilesWatcher};
use crate::protocol::{Connection, Message, Notification, Request, RequestId};
use crate::Result;
use anyhow::anyhow;
use async_std::sync::RwLock;
use futures::channel::mpsc::UnboundedReceiver;
use futures::{SinkExt, StreamExt};
use ra_vfs::{RootEntry, Vfs};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug)]
enum Event {
    Msg(Message),
    Vfs(ra_vfs::VfsTask),
}

/// State for the language server
struct LanguageServerState {
    /// Interface to the vfs, a virtual filesystem that supports the overlaying of files
    pub vfs: Arc<RwLock<Vfs>>,

    /// Receiver channel to apply filesystem changes on `vfs`
    pub vfs_task_receiver: UnboundedReceiver<ra_vfs::VfsTask>,
}

/// State maintained for the connection. This includes everything that is required to be able to
/// properly communicate with the client but has nothing to do with any Mun related state.
struct ConnectionState {
    connection: Connection,

    next_request_id: u64,
    pending_responses: HashSet<RequestId>,
}

impl ConnectionState {
    /// Constructs a new `ConnectionState`
    fn new(connection: Connection) -> Self {
        Self {
            connection,
            next_request_id: 0,
            pending_responses: Default::default(),
        }
    }

    /// Constructs a new request ID and stores that we are still awaiting a response.
    fn next_request_id(&mut self) -> RequestId {
        self.next_request_id += 1;
        let res: RequestId = self.next_request_id.into();
        let inserted = self.pending_responses.insert(res.clone());
        assert!(inserted);
        res
    }
}

/// Filter for to choose which files the ra_vfs should ignore
struct MunFilter {}

/// Implement the filter provided by ra_vfs
impl ra_vfs::Filter for MunFilter {
    fn include_dir(&self, _dir_path: &ra_vfs::RelativePath) -> bool {
        true
    }

    fn include_file(&self, file_path: &ra_vfs::RelativePath) -> bool {
        file_path.extension() == Some("mun")
    }
}

impl LanguageServerState {
    pub fn new(config: Config) -> Self {
        // Create a channel for use by the vfs
        let (task_sender, task_receiver) = futures::channel::mpsc::unbounded();

        // Create the vfs
        let task_sender = Box::new(move |t| task_sender.unbounded_send(t).unwrap());
        let vfs = Vfs::new(
            config
                .workspace_roots
                .into_iter()
                .map(|root| RootEntry::new(root, Box::new(MunFilter {})))
                .collect(),
            task_sender,
            ra_vfs::Watch(config.watcher == FilesWatcher::Notify),
        );

        LanguageServerState {
            vfs: Arc::new(RwLock::new(vfs.0)),
            vfs_task_receiver: task_receiver,
        }
    }
}

/// Registers file watchers with the client to monitor all mun files in the workspaces
async fn register_client_file_watcher(connection_state: &mut ConnectionState, config: &Config) {
    let registration_options = lsp_types::DidChangeWatchedFilesRegistrationOptions {
        watchers: config
            .workspace_roots
            .iter()
            .map(|root| format!("{}/**/*.mun", root.display()))
            .map(|glob_pattern| lsp_types::FileSystemWatcher {
                glob_pattern,
                kind: None,
            })
            .collect(),
    };
    let registration = lsp_types::Registration {
        id: "file-watcher".to_string(),
        method: "workspace/didChangeWatchedFiles".to_string(),
        register_options: Some(serde_json::to_value(registration_options).unwrap()),
    };
    let params = lsp_types::RegistrationParams {
        registrations: vec![registration],
    };
    let request = build_request::<lsp_types::request::RegisterCapability>(
        connection_state.next_request_id(),
        params,
    );
    connection_state
        .connection
        .sender
        .send(request.into())
        .await
        .unwrap();
}

/// Runs the main loop of the language server. This will receive requests and handle them.
pub async fn main_loop(connection: Connection, config: Config) -> Result<()> {
    log::info!("initial config: {:#?}", config);

    // Subscribe with file watchers of the client if enabled
    let mut connection_state = ConnectionState::new(connection);
    if config.watcher == FilesWatcher::Client {
        register_client_file_watcher(&mut connection_state, &config).await
    }

    // Create the state for the language server
    let mut state = LanguageServerState::new(config);
    loop {
        // Determine what to do next. This selects from different channels, the first message to
        // arrive is returned. If an error occurs on one of the channel the main loop is shutdown
        // with an error.
        let event = futures::select! {
            msg = connection_state.connection.receiver.next() => match msg {
                Some(msg) => Event::Msg(msg),
                None => return Err(anyhow::anyhow!("client exited without shutdown")),
            },
            task = state.vfs_task_receiver.next() =>  match task {
                Some(task) => Event::Vfs(task),
                None => return Err(anyhow::anyhow!("vfs has died")),
            }
        };

        // Handle the event
        match handle_event(event, &mut connection_state, &mut state).await? {
            LoopState::Continue => {}
            LoopState::Shutdown => {
                break;
            }
        }
    }

    Ok(())
}

/// A `LoopState` enumerator determines the state of the main loop
enum LoopState {
    Continue,
    Shutdown,
}

/// Handles a received request
async fn handle_request(request: Request, connection: &mut ConnectionState) -> Result<LoopState> {
    if connection.connection.handle_shutdown(&request).await? {
        return Ok(LoopState::Shutdown);
    };
    Ok(LoopState::Continue)
}

/// Handles a received notification
async fn on_notification(
    notification: Notification,
    connection: &mut ConnectionState,
    state: &LanguageServerState,
) -> Result<LoopState> {
    let notification =
        // When a a text document is opened
        match cast_notification::<lsp_types::notification::DidOpenTextDocument>(notification) {
            Ok(params) => {
                // Get the uri
                let uri = params.text_document.uri;
                // And convert into a file path
                let path = uri
                    .to_file_path()
                    .map_err(|()| anyhow!("invalid uri: {}", uri))?;
                if let Some(_) = state
                    .vfs
                    .write()
                    .await
                    .add_file_overlay(&path, params.text_document.text)
                {
                    //loop_state.subscriptions.add_sub(FileId(file_id.0));
                }
                return Ok(LoopState::Continue);
            }
            Err(not) => not,
        };

    // When a text document is closed
    let notification =
        match cast_notification::<lsp_types::notification::DidCloseTextDocument>(notification) {
            Ok(params) => {
                let uri = params.text_document.uri;
                let path = uri
                    .to_file_path()
                    .map_err(|()| anyhow!("invalid uri: {}", uri))?;
                if let Some(_) = state.vfs.write().await.remove_file_overlay(path.as_path()) {
                    //loop_state.subscriptions.remove_sub(FileId(file_id.0));
                }
                let params = lsp_types::PublishDiagnosticsParams {
                    uri,
                    diagnostics: Vec::new(),
                    version: None,
                };
                let not = build_notification::<lsp_types::notification::PublishDiagnostics>(params);
                connection.connection.sender.try_send(not.into()).unwrap();
                return Ok(LoopState::Continue);
            }
            Err(not) => not,
        };

    let notification =
        match cast_notification::<lsp_types::notification::DidChangeTextDocument>(notification) {
            Ok(params) => {
                let lsp_types::DidChangeTextDocumentParams {
                    text_document,
                    content_changes,
                } = params;
                //let world = state.snapshot();
                //let file_id = from_proto::file_id(&world, &text_document.uri)?;
                //let line_index = world.analysis().file_line_index(file_id)?;
                let uri = text_document.uri;
                let path = uri
                    .to_file_path()
                    .map_err(|()| anyhow!("invalid uri: {}", uri))?;
                // TODO: I assume that since we are using *FULL* as the support change mode, that get
                // the text as a single change
                state
                    .vfs
                    .write()
                    .await
                    .change_file_overlay(&path, |old_text| {
                        // TODO: Change this to incremental later
                        *old_text = content_changes.get(0).unwrap().text.clone();
                    });
                return Ok(LoopState::Continue);
            }
            Err(not) => not,
        };

    let _notification =
        match cast_notification::<lsp_types::notification::DidChangeWatchedFiles>(notification) {
            Ok(params) => {
                let mut vfs = state.vfs.write().await;
                for change in params.changes {
                    let uri = change.uri;
                    let path = uri
                        .to_file_path()
                        .map_err(|()| anyhow::anyhow!("invalid uri: {}", uri))?;
                    vfs.notify_changed(path)
                }
                return Ok(LoopState::Continue);
            }
            Err(not) => not,
        };

    Ok(LoopState::Continue)
}

/// Handles an incoming event. Returns a `LoopState` state which determines whether processing
/// should continue.
async fn handle_event(
    event: Event,
    connection_state: &mut ConnectionState,
    state: &LanguageServerState,
) -> Result<LoopState> {
    log::info!("handling event: {:?}", event);
    let loop_state = match event {
        Event::Msg(msg) => handle_lsp_message(msg, connection_state, state).await?,
        Event::Vfs(task) => handle_vfs_task(task, state).await?,
    };

    let _vfs_changes = state.vfs.write().await.commit_changes();
    // TODO: Do something will all the vfs changes

    Ok(loop_state)
}

/// Handles a change to the underlying virtual file system.
async fn handle_vfs_task(task: ra_vfs::VfsTask, state: &LanguageServerState) -> Result<LoopState> {
    let mut vfs = state.vfs.write().await;
    vfs.handle_task(task);
    Ok(LoopState::Continue)
}

/// Handles an incoming message via the language server protocol.
async fn handle_lsp_message(
    msg: Message,
    connection_state: &mut ConnectionState,
    state: &LanguageServerState,
) -> Result<LoopState> {
    match msg {
        Message::Request(req) => handle_request(req, connection_state).await,
        Message::Response(response) => {
            let removed = connection_state.pending_responses.remove(&response.id);
            if !removed {
                log::error!("unexpected response: {:?}", response)
            }

            Ok(LoopState::Continue)
        }
        Message::Notification(notification) => {
            on_notification(notification, connection_state, state).await
        }
    }
}

/// Constructs a new request with the generic type R and the given parameters.
fn build_request<R>(id: RequestId, params: R::Params) -> Request
where
    R: lsp_types::request::Request,
    R::Params: Serialize,
{
    Request::new(id, R::METHOD.to_string(), params)
}

/// Create an new notification with the specified parameters
fn build_notification<N>(params: N::Params) -> Notification
where
    N: lsp_types::notification::Notification,
    N::Params: Serialize,
{
    Notification::new(N::METHOD.to_string(), params)
}

/// Cast a notification to a specific type
fn cast_notification<N>(notification: Notification) -> std::result::Result<N::Params, Notification>
where
    N: lsp_types::notification::Notification,
    N::Params: DeserializeOwned,
{
    notification.try_extract(N::METHOD)
}
