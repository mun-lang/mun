use crate::analysis::{Analysis, AnalysisSnapshot};
use crate::change::AnalysisChange;
use crate::config::{Config, FilesWatcher};
use crate::conversion::{convert_range, url_from_path_with_drive_lowercasing};
use crate::protocol::{Connection, Message, Notification, Request, RequestId};
use crate::Result;
use anyhow::anyhow;
use async_std::sync::RwLock;
use futures::channel::mpsc::{unbounded, Sender, UnboundedReceiver, UnboundedSender};
use futures::{SinkExt, StreamExt};
use lsp_types::notification::PublishDiagnostics;
use lsp_types::{PublishDiagnosticsParams, Url};
use ra_vfs::{RootEntry, Vfs, VfsChange, VfsFile};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// A `Task` is something that is send from async tasks to the entry point for processing. This
/// enables synchronizing resources like the connection with the client.
#[derive(Debug)]
enum Task {
    Notify(Notification),
}

#[derive(Debug)]
enum Event {
    Msg(Message),
    Vfs(ra_vfs::VfsTask),
    Task(Task),
}

/// State for the language server
struct LanguageServerState {
    /// Interface to the vfs, a virtual filesystem that supports overlaying of files
    pub vfs: Arc<RwLock<Vfs>>,

    /// Receiver channel to apply filesystem changes on `vfs`
    pub vfs_task_receiver: UnboundedReceiver<ra_vfs::VfsTask>,

    /// Holds the state of the analysis process
    pub analysis: Analysis,

    /// All the roots in the workspace
    pub local_source_roots: Vec<hir::SourceRootId>,
}

/// A snapshot of the state of the language server
struct LanguageServerSnapshot {
    /// Interface to the vfs, a virtual filesystem that supports overlaying of files
    pub vfs: Arc<RwLock<Vfs>>,

    /// Holds the state of the analysis process
    pub analysis: AnalysisSnapshot,

    /// All the roots in the workspace
    pub local_source_roots: Vec<hir::SourceRootId>,
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
        debug_assert!(inserted);
        res
    }
}

/// Filter used to choose which files the ra_vfs should ignore
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

        // Apply the initial changes
        let mut source_roots = Vec::new();
        let mut change = AnalysisChange::new();
        for root in vfs.1.iter() {
            change.add_root(hir::SourceRootId(root.0));
            source_roots.push(hir::SourceRootId(root.0));
        }

        // Construct the state that will hold all the analysis
        let mut analysis = Analysis::new();
        analysis.apply_change(change);

        LanguageServerState {
            vfs: Arc::new(RwLock::new(vfs.0)),
            vfs_task_receiver: task_receiver,
            analysis,
            local_source_roots: source_roots,
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

    // Create a thread pool to dispatch the async commands
    // Use the num_cpus to get a nice thread count estimation
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build()?;

    // Create the state for the language server
    let mut state = LanguageServerState::new(config);
    let (task_sender, mut task_receiver) = unbounded::<Task>();
    loop {
        // Determine what to do next. This selects from different channels, the first message to
        // arrive is returned. If an error occurs on one of the channel the main loop is shutdown
        // with an error.
        let event = futures::select! {
            msg = connection_state.connection.receiver.next() => match msg {
                Some(msg) => Event::Msg(msg),
                None => return Err(anyhow::anyhow!("client exited without shutdown")),
            },
            task = state.vfs_task_receiver.next() => match task {
                Some(task) => Event::Vfs(task),
                None => return Err(anyhow::anyhow!("vfs has died")),
            },
            task = task_receiver.next() => Event::Task(task.unwrap())
        };

        // Handle the event
        match handle_event(
            event,
            &task_sender,
            &mut connection_state,
            &pool,
            &mut state,
        )
        .await?
        {
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
                if state
                    .vfs
                    .write()
                    .await
                    .add_file_overlay(&path, params.text_document.text).is_some()
                {
                    // TODO: Keep track of opened files
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
                if state
                    .vfs
                    .write()
                    .await
                    .remove_file_overlay(path.as_path())
                    .is_some()
                {
                    // TODO: Keep track of opened files
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
    task_sender: &UnboundedSender<Task>,
    connection_state: &mut ConnectionState,
    pool: &rayon::ThreadPool,
    state: &mut LanguageServerState,
) -> Result<LoopState> {
    log::info!("handling event: {:?}", event);

    // Process the incoming event
    let loop_state = match event {
        Event::Task(task) => handle_task(task, &mut connection_state.connection.sender).await?,
        Event::Msg(msg) => handle_lsp_message(msg, connection_state, state).await?,
        Event::Vfs(task) => handle_vfs_task(task, state).await?,
    };

    // Process any changes to the vfs
    let state_changed = state.process_vfs_changes().await;
    if state_changed {
        let snapshot = state.snapshot();
        let task_sender = task_sender.clone();
        // Spawn the diagnostics in the threadpool
        pool.spawn(move || {
            async_std::task::block_on(handle_diagnostics(snapshot, task_sender));
        });
    }

    Ok(loop_state)
}

/// Send all diagnostics of all files
async fn handle_diagnostics(state: LanguageServerSnapshot, mut sender: UnboundedSender<Task>) {
    // Iterate over all files
    for root in state.local_source_roots.iter() {
        // Get all the files
        let files = match state.analysis.source_root_files(*root) {
            Ok(files) => files,
            Err(_) => return,
        };

        // Publish all diagnostics
        for file in files {
            let line_index = match state.analysis.file_line_index(file) {
                Ok(line_index) => line_index,
                Err(_) => return,
            };
            let uri = state.file_id_to_uri(file);
            let uri = uri.await.unwrap();
            let diagnostics = match state.analysis.diagnostics(file) {
                Ok(line_index) => line_index,
                Err(_) => return,
            };

            let diagnostics = diagnostics
                .into_iter()
                .map(|d| lsp_types::Diagnostic {
                    range: convert_range(d.range, &line_index),
                    severity: Some(lsp_types::DiagnosticSeverity::Error),
                    code: None,
                    source: Some("mun".to_string()),
                    message: d.message,
                    related_information: None,
                    tags: None,
                })
                .collect();

            sender
                .send(Task::Notify(build_notification::<PublishDiagnostics>(
                    PublishDiagnosticsParams {
                        uri,
                        diagnostics,
                        version: None,
                    },
                )))
                .await
                .unwrap();
        }
    }
}

/// Handles a task send by another async task
async fn handle_task(task: Task, sender: &mut Sender<Message>) -> Result<LoopState> {
    match task {
        Task::Notify(notification) => sender.send(notification.into()).await?,
    }

    Ok(LoopState::Continue)
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

/// Constructs a new notification with the specified parameters.
fn build_notification<N>(params: N::Params) -> Notification
where
    N: lsp_types::notification::Notification,
    N::Params: Serialize,
{
    Notification::new(N::METHOD.to_string(), params)
}

/// Casts a notification to the specified type.
fn cast_notification<N>(notification: Notification) -> std::result::Result<N::Params, Notification>
where
    N: lsp_types::notification::Notification,
    N::Params: DeserializeOwned,
{
    notification.try_extract(N::METHOD)
}

impl LanguageServerState {
    /// Creates a snapshot of the state
    pub fn snapshot(&self) -> LanguageServerSnapshot {
        LanguageServerSnapshot {
            analysis: self.analysis.snapshot(),
            local_source_roots: self.local_source_roots.clone(),
            vfs: self.vfs.clone(),
        }
    }

    /// Processes any and all changes that have been applied to the virtual filesystem. Generates
    /// an `AnalysisChange` and applies it if there are changes. True is returned if things changed,
    /// otherwise false.
    pub async fn process_vfs_changes(&mut self) -> bool {
        // Get all the changes since the last time we processed
        let changes = self.vfs.write().await.commit_changes();
        if changes.is_empty() {
            return false;
        }

        // Construct an AnalysisChange to apply
        let mut analysis_change = AnalysisChange::new();
        for change in changes {
            match change {
                VfsChange::AddRoot { root, files } => {
                    for (file, path, text) in files {
                        analysis_change.add_file(
                            hir::SourceRootId(root.0),
                            hir::FileId(file.0),
                            path,
                            text,
                        );
                    }
                }
                VfsChange::AddFile {
                    root,
                    file,
                    path,
                    text,
                } => {
                    analysis_change.add_file(
                        hir::SourceRootId(root.0),
                        hir::FileId(file.0),
                        path,
                        text,
                    );
                }
                VfsChange::RemoveFile { root, file, path } => analysis_change.remove_file(
                    hir::SourceRootId(root.0),
                    hir::FileId(file.0),
                    path,
                ),
                VfsChange::ChangeFile { file, text } => {
                    analysis_change.change_file(hir::FileId(file.0), text);
                }
            }
        }

        // Apply the change
        self.analysis.apply_change(analysis_change);
        true
    }
}

impl LanguageServerSnapshot {
    /// Converts the specified `FileId` to a `Url`
    pub async fn file_id_to_uri(&self, id: hir::FileId) -> Result<Url> {
        let path = self.vfs.read().await.file2path(VfsFile(id.0));
        let url = url_from_path_with_drive_lowercasing(path)?;

        Ok(url)
    }
}
