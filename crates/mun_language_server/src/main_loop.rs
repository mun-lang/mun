use crate::{
    analysis::{Analysis, AnalysisSnapshot, Cancelable},
    change::AnalysisChange,
    config::Config,
    conversion::{convert_range, convert_uri, url_from_path_with_drive_lowercasing},
    protocol::{Connection, Message, Notification, Request, RequestId},
    Result,
};
use async_std::sync::RwLock;
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};
use lsp_types::{notification::PublishDiagnostics, PublishDiagnosticsParams, Url};
use paths::AbsPathBuf;
use rustc_hash::FxHashSet;
use serde::{de::DeserializeOwned, Serialize};
use std::{cell::RefCell, collections::HashSet, ops::Deref, sync::Arc};
use vfs::VirtualFileSystem;

/// A `Task` is something that is send from async tasks to the entry point for processing. This
/// enables synchronizing resources like the connection with the client.
#[derive(Debug)]
enum Task {
    Notify(Notification),
}

#[derive(Debug)]
enum Event {
    Msg(Message),
    Vfs(vfs::MonitorMessage),
    Task(Task),
}

/// State for the language server
pub(crate) struct LanguageServerState {
    /// The connection with the client
    pub connection: ConnectionState,

    /// The configuration passed by the client
    pub config: Config,

    /// Thread pool for async execution
    pub thread_pool: rayon::ThreadPool,

    /// The virtual filesystem that holds all the file contents
    pub vfs: Arc<RwLock<VirtualFileSystem>>,

    /// The vfs monitor
    pub vfs_monitor: Box<dyn vfs::Monitor>,

    /// The receiver of vfs monitor messages
    pub vfs_monitor_receiver: UnboundedReceiver<vfs::MonitorMessage>,

    /// Documents that are currently kept in memory from the client
    pub open_docs: FxHashSet<AbsPathBuf>,

    /// Holds the state of the analysis process
    pub analysis: Analysis,

    /// All the packages known to the server
    pub packages: Arc<Vec<project::Package>>,
}

/// A snapshot of the state of the language server
pub(crate) struct LanguageServerSnapshot {
    /// The virtual filesystem that holds all the file contents
    pub vfs: Arc<RwLock<VirtualFileSystem>>,

    /// Holds the state of the analysis process
    pub analysis: AnalysisSnapshot,

    /// All the packages known to the server
    pub packages: Arc<Vec<project::Package>>,
}

/// State maintained for the connection. This includes everything that is required to be able to
/// properly communicate with the client but has nothing to do with any Mun related state.
pub(crate) struct ConnectionState {
    pub(crate) connection: Connection,

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

impl LanguageServerState {
    pub fn new(connection: Connection, config: Config) -> Self {
        // Construct the virtual filesystem monitor
        let (vfs_monitor_sender, vfs_monitor_receiver) = unbounded::<vfs::MonitorMessage>();
        let vfs_monitor_sender = RefCell::new(vfs_monitor_sender);
        let vfs_monitor: vfs::NotifyMonitor = vfs::Monitor::new(Box::new(move |msg| {
            async_std::task::block_on(vfs_monitor_sender.borrow_mut().send(msg))
                .expect("error sending vfs monitor message to foreground")
        }));
        let vfs_monitor = Box::new(vfs_monitor) as Box<dyn vfs::Monitor>;

        // Create a thread pool to dispatch the async commands
        // Use the num_cpus to get a nice thread count estimation
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .expect("unable to spin up thread pool");

        // Apply the initial changes
        let mut change = AnalysisChange::new();
        change.set_packages(Default::default());
        change.set_roots(Default::default());

        // Construct the state that will hold all the analysis
        let mut analysis = Analysis::new();
        analysis.apply_change(change);

        LanguageServerState {
            connection: ConnectionState::new(connection),
            config,
            vfs: Arc::new(RwLock::new(Default::default())),
            vfs_monitor,
            vfs_monitor_receiver,
            open_docs: FxHashSet::default(),
            thread_pool,
            analysis,
            packages: Arc::new(Vec::new()),
        }
    }

    /// Runs the language server to completion
    pub async fn run(mut self) -> Result<()> {
        // Start by updating the current workspace
        self.fetch_workspaces();

        // Process events as the pass
        let (task_sender, mut task_receiver) = futures::channel::mpsc::unbounded::<Task>();
        loop {
            // Determine what to do next. This selects from different channels, the first message to
            // arrive is returned. If an error occurs on one of the channel the main loop is shutdown
            // with an error.
            let event = futures::select! {
                msg = self.connection.connection.receiver.next() => match msg {
                    Some(msg) => Event::Msg(msg),
                    None => return Err(anyhow::anyhow!("client exited without shutdown")),
                },
                msg = self.vfs_monitor_receiver.next() => match msg {
                    Some(msg) => Event::Vfs(msg),
                    None => return Err(anyhow::anyhow!("client exited without shutdown")),
                },
                task = task_receiver.next() => Event::Task(task.unwrap()),
            };

            // Handle the event
            match handle_event(event, &task_sender, &mut self).await? {
                LoopState::Continue => {}
                LoopState::Shutdown => {
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Runs the main loop of the language server. This will receive requests and handle them.
pub async fn main_loop(connection: Connection, config: Config) -> Result<()> {
    log::info!("initial config: {:#?}", config);
    LanguageServerState::new(connection, config).run().await
}

/// A `LoopState` enumerator determines the state of the main loop
enum LoopState {
    Continue,
    Shutdown,
}

/// Handles a received request
async fn handle_request(request: Request, state: &mut LanguageServerState) -> Result<LoopState> {
    if state
        .connection
        .connection
        .handle_shutdown(&request)
        .await?
    {
        return Ok(LoopState::Shutdown);
    };
    Ok(LoopState::Continue)
}

/// Handles a received notification
async fn on_notification(
    notification: Notification,
    state: &mut LanguageServerState,
) -> Result<LoopState> {
    let notification =
        // When a a text document is opened
        match cast_notification::<lsp_types::notification::DidOpenTextDocument>(notification) {
            Ok(params) => {
                if let Ok(path) = convert_uri(&params.text_document.uri) {
                    state.open_docs.insert(path.clone());
                    state.vfs.write().await.set_file_contents(&path, Some(params.text_document.text.into_bytes()));
                }
                return Ok(LoopState::Continue);
            }
            Err(not) => not,
        };

    // When a text document is closed
    let notification =
        match cast_notification::<lsp_types::notification::DidCloseTextDocument>(notification) {
            Ok(params) => {
                if let Ok(path) = convert_uri(&params.text_document.uri) {
                    state.open_docs.remove(&path);
                    state.vfs_monitor.reload(&path);
                }
                let params = lsp_types::PublishDiagnosticsParams {
                    uri: params.text_document.uri,
                    diagnostics: Vec::new(),
                    version: None,
                };
                let not = build_notification::<lsp_types::notification::PublishDiagnostics>(params);
                state
                    .connection
                    .connection
                    .sender
                    .try_send(not.into())
                    .unwrap();
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
                if let Ok(path) = convert_uri(&text_document.uri) {
                    let new_content = content_changes.get(0).unwrap().text.clone();
                    state
                        .vfs
                        .write()
                        .await
                        .set_file_contents(&path, Some(new_content.into_bytes()));
                }
                return Ok(LoopState::Continue);
            }
            Err(not) => not,
        };

    let _notification =
        match cast_notification::<lsp_types::notification::DidChangeWatchedFiles>(notification) {
            Ok(params) => {
                for change in params.changes {
                    if let Ok(path) = convert_uri(&change.uri) {
                        state.vfs_monitor.reload(&path);
                    }
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
    state: &mut LanguageServerState,
) -> Result<LoopState> {
    log::info!("handling event: {:?}", event);

    // Process the incoming event
    let loop_state = match event {
        Event::Task(task) => handle_task(task, state).await?,
        Event::Msg(msg) => handle_lsp_message(msg, state).await?,
        Event::Vfs(task) => handle_vfs_task(task, state).await?,
    };

    // Process any changes to the vfs
    let state_changed = state.process_vfs_changes().await;
    dbg!(state_changed);
    if state_changed {
        let snapshot = state.snapshot();
        let task_sender = task_sender.clone();
        // Spawn the diagnostics in the threadpool
        state.thread_pool.spawn(move || {
            let _result = async_std::task::block_on(handle_diagnostics(snapshot, task_sender));
        });
    }

    Ok(loop_state)
}

/// Send all diagnostics of all files
async fn handle_diagnostics(
    state: LanguageServerSnapshot,
    mut sender: UnboundedSender<Task>,
) -> Cancelable<()> {
    dbg!(&state.packages);

    // Iterate over all files
    for (idx, _package) in state.packages.iter().enumerate() {
        let package_id = hir::PackageId(idx as u32);

        // Get all the files
        let files = state.analysis.package_source_files(package_id)?;

        // Publish all diagnostics
        for file in files {
            let line_index = state.analysis.file_line_index(file)?;
            let uri = state.file_id_to_uri(file).await.unwrap();
            let diagnostics = state.analysis.diagnostics(file)?;

            let diagnostics = {
                let mut lsp_diagnostics = Vec::with_capacity(diagnostics.len());
                for d in diagnostics {
                    lsp_diagnostics.push(lsp_types::Diagnostic {
                        range: convert_range(d.range, &line_index),
                        severity: Some(lsp_types::DiagnosticSeverity::Error),
                        code: None,
                        source: Some("mun".to_string()),
                        message: d.message,
                        related_information: {
                            let mut annotations =
                                Vec::with_capacity(d.additional_annotations.len());
                            for annotation in d.additional_annotations {
                                annotations.push(lsp_types::DiagnosticRelatedInformation {
                                    location: lsp_types::Location {
                                        uri: state
                                            .file_id_to_uri(annotation.range.file_id)
                                            .await
                                            .unwrap(),
                                        range: convert_range(
                                            annotation.range.value,
                                            state
                                                .analysis
                                                .file_line_index(annotation.range.file_id)?
                                                .deref(),
                                        ),
                                    },
                                    message: annotation.message,
                                });
                            }
                            if annotations.is_empty() {
                                None
                            } else {
                                Some(annotations)
                            }
                        },
                        tags: None,
                    });
                }
                lsp_diagnostics
            };

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
    Ok(())
}

/// Handles a task send by another async task
async fn handle_task(task: Task, state: &mut LanguageServerState) -> Result<LoopState> {
    match task {
        Task::Notify(notification) => {
            state
                .connection
                .connection
                .sender
                .send(notification.into())
                .await?
        }
    }

    Ok(LoopState::Continue)
}

/// Handles a change to the underlying virtual file system.
async fn handle_vfs_task(
    mut task: vfs::MonitorMessage,
    state: &mut LanguageServerState,
) -> Result<LoopState> {
    loop {
        match task {
            vfs::MonitorMessage::Progress { .. } => {}
            vfs::MonitorMessage::Loaded { files } => {
                let vfs = &mut *state.vfs.write().await;
                for (path, contents) in files {
                    vfs.set_file_contents(&path, contents);
                }
            }
        }

        // Coalesce many VFS events into a single loop turn
        task = match state.vfs_monitor_receiver.try_next() {
            Ok(Some(task)) => task,
            _ => break,
        }
    }
    Ok(LoopState::Continue)
}

/// Handles an incoming message via the language server protocol.
async fn handle_lsp_message(msg: Message, state: &mut LanguageServerState) -> Result<LoopState> {
    match msg {
        Message::Request(req) => handle_request(req, state).await,
        Message::Response(response) => {
            let removed = state.connection.pending_responses.remove(&response.id);
            if !removed {
                log::error!("unexpected response: {:?}", response)
            }

            Ok(LoopState::Continue)
        }
        Message::Notification(notification) => on_notification(notification, state).await,
    }
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
    /// Sends a new request to the client
    pub fn send_request<R: lsp_types::request::Request>(&mut self, params: R::Params) {
        let request = Request::new(
            self.connection.next_request_id(),
            R::METHOD.to_string(),
            params,
        );
        async_std::task::block_on(self.connection.connection.sender.send(request.into())).unwrap();
    }
}

impl LanguageServerState {
    /// Creates a snapshot of the state
    pub fn snapshot(&self) -> LanguageServerSnapshot {
        LanguageServerSnapshot {
            vfs: self.vfs.clone(),
            analysis: self.analysis.snapshot(),
            packages: self.packages.clone(),
        }
    }

    /// Processes any and all changes that have been applied to the virtual filesystem. Generates
    /// an `AnalysisChange` and applies it if there are changes. True is returned if things changed,
    /// otherwise false.
    pub async fn process_vfs_changes(&mut self) -> bool {
        // Get all the changes since the last time we processed
        let changed_files = {
            let mut vfs = self.vfs.write().await;
            vfs.take_changes()
        };
        if changed_files.is_empty() {
            return false;
        }

        // Construct an AnalysisChange to apply to the analysis
        let vfs = self.vfs.read().await;
        let mut analysis_change = AnalysisChange::new();
        let mut has_created_or_deleted_entries = false;
        for file in changed_files {
            // If the file was deleted or created we have to remember that so that we update the
            // source roots as well.
            if file.is_created_or_deleted() {
                has_created_or_deleted_entries = true;
            }

            // Convert the contents of the file to a string
            let bytes = vfs
                .file_contents(file.file_id)
                .map(Vec::from)
                .unwrap_or_default();
            let text = match String::from_utf8(bytes).ok() {
                Some(text) => Some(Arc::from(text)),
                None => None,
            };

            // Notify the database about this change
            analysis_change.change_file(hir::FileId(file.file_id.0), text);
        }

        // If an entry was created or deleted we have to recreate all source roots
        if has_created_or_deleted_entries {
            analysis_change.set_roots(self.recompute_source_roots());
        }

        // Apply the change
        self.analysis.apply_change(analysis_change);
        true
    }
}

impl LanguageServerSnapshot {
    /// Converts the specified `hir::FileId` to a `Url`
    pub async fn file_id_to_uri(&self, id: hir::FileId) -> Result<Url> {
        let vfs = self.vfs.read().await;
        let path = vfs.file_path(vfs::FileId(id.0));
        let url = url_from_path_with_drive_lowercasing(path)?;

        Ok(url)
    }
}
