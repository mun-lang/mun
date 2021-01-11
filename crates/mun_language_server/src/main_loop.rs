use crate::dispatcher::{NotificationDispatcher, RequestDispatcher};
use crate::{
    analysis::{Analysis, AnalysisSnapshot, Cancelable},
    change::AnalysisChange,
    config::Config,
    conversion::{convert_range, convert_uri, url_from_path_with_drive_lowercasing},
    to_json, Result,
};
use crossbeam_channel::{select, unbounded, Receiver, Sender};
use lsp_server::{Connection, ReqQueue};
use lsp_types::notification::Notification;
use lsp_types::{notification::PublishDiagnostics, PublishDiagnosticsParams, Url};
use parking_lot::RwLock;
use paths::AbsPathBuf;
use rustc_hash::FxHashSet;
use std::time::Instant;
use std::{ops::Deref, sync::Arc};
use vfs::VirtualFileSystem;

/// A `Task` is something that is send from async tasks to the entry point for processing. This
/// enables synchronizing resources like the connection with the client.
#[derive(Debug)]
pub(crate) enum Task {
    Notify(lsp_server::Notification),
}

#[derive(Debug)]
pub(crate) enum Event {
    Vfs(vfs::MonitorMessage),
    Task(Task),
    Lsp(lsp_server::Message),
}

pub(crate) type RequestHandler = fn(&mut LanguageServerState, lsp_server::Response);

/// State for the language server
pub(crate) struct LanguageServerState {
    /// Channel to send language server messages to the client
    sender: Sender<lsp_server::Message>,

    /// The request queue keeps track of all incoming and outgoing requests.
    request_queue: lsp_server::ReqQueue<(String, Instant), RequestHandler>,

    /// The configuration passed by the client
    pub config: Config,

    /// Thread pool for async execution
    pub thread_pool: rayon::ThreadPool,

    /// Channel to send tasks to from background operations
    pub task_sender: Sender<Task>,

    /// Channel to receive tasks on from background operations
    pub task_receiver: Receiver<Task>,

    /// The virtual filesystem that holds all the file contents
    pub vfs: Arc<RwLock<VirtualFileSystem>>,

    /// The vfs monitor
    pub vfs_monitor: Box<dyn vfs::Monitor>,

    /// The receiver of vfs monitor messages
    pub vfs_monitor_receiver: Receiver<vfs::MonitorMessage>,

    /// Documents that are currently kept in memory from the client
    pub open_docs: FxHashSet<AbsPathBuf>,

    /// Holds the state of the analysis process
    pub analysis: Analysis,

    /// All the packages known to the server
    pub packages: Arc<Vec<project::Package>>,

    /// True if the client requested that we shut down
    pub shutdown_requested: bool,
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

impl LanguageServerState {
    pub fn new(sender: Sender<lsp_server::Message>, config: Config) -> Self {
        // Construct the virtual filesystem monitor
        let (vfs_monitor_sender, vfs_monitor_receiver) = unbounded::<vfs::MonitorMessage>();
        let vfs_monitor: vfs::NotifyMonitor = vfs::Monitor::new(Box::new(move |msg| {
            vfs_monitor_sender
                .send(msg)
                .expect("error sending vfs monitor message to foreground")
        }));
        let vfs_monitor = Box::new(vfs_monitor) as Box<dyn vfs::Monitor>;

        // Create a thread pool to dispatch the async commands
        // Use the num_cpus to get a nice thread count estimation
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .expect("unable to spin up thread pool");

        // Construct a task channel
        let (task_sender, task_receiver) = unbounded();

        // Construct the state that will hold all the analysis and apply the initial state
        let mut analysis = Analysis::new();
        let mut change = AnalysisChange::new();
        change.set_packages(Default::default());
        change.set_roots(Default::default());
        analysis.apply_change(change);

        LanguageServerState {
            sender,
            request_queue: ReqQueue::default(),
            config,
            vfs: Arc::new(RwLock::new(Default::default())),
            vfs_monitor,
            vfs_monitor_receiver,
            open_docs: FxHashSet::default(),
            thread_pool,
            task_sender,
            task_receiver,
            analysis,
            packages: Arc::new(Vec::new()),
            shutdown_requested: false,
        }
    }

    /// Blocks until a new event is received from on of the many channels the language server
    /// listens to. Returns the first event that is received.
    fn next_event(&self, receiver: &Receiver<lsp_server::Message>) -> Option<Event> {
        select! {
            recv(receiver) -> msg => msg.ok().map(Event::Lsp),
            recv(self.vfs_monitor_receiver) -> task => Some(Event::Vfs(task.unwrap())),
            recv(self.task_receiver) -> task => Some(Event::Task(task.unwrap()))
        }
    }

    /// Runs the language server to completion
    pub fn run(mut self, receiver: Receiver<lsp_server::Message>) -> Result<()> {
        // Start by updating the current workspace
        self.fetch_workspaces();

        while let Some(event) = self.next_event(&receiver) {
            if let Event::Lsp(lsp_server::Message::Notification(notification)) = &event {
                if notification.method == lsp_types::notification::Exit::METHOD {
                    return Ok(());
                }
            }
            self.handle_event(event)?;
        }

        Ok(())
    }

    /// Handles an event from one of the many sources that the language server subscribes to.
    fn handle_event(&mut self, event: Event) -> Result<()> {
        let start_time = Instant::now();
        log::info!("handling event: {:?}", event);

        // Process the incoming event
        match event {
            Event::Task(task) => handle_task(task, self)?,
            Event::Lsp(msg) => match msg {
                lsp_server::Message::Request(req) => self.on_request(req, start_time)?,
                lsp_server::Message::Response(resp) => self.complete_request(resp),
                lsp_server::Message::Notification(not) => self.on_notification(not)?,
            },
            Event::Vfs(task) => handle_vfs_task(task, self)?,
        };

        // Process any changes to the vfs
        let state_changed = self.process_vfs_changes();
        if state_changed {
            let snapshot = self.snapshot();
            let task_sender = self.task_sender.clone();
            // Spawn the diagnostics in the threadpool
            self.thread_pool.spawn(move || {
                handle_diagnostics(snapshot, task_sender).unwrap();
            });
        }

        Ok(())
    }

    /// Handles a language server protocol request
    fn on_request(
        &mut self,
        request: lsp_server::Request,
        request_received: Instant,
    ) -> Result<()> {
        self.register_request(&request, request_received);

        // If a shutdown was requested earlier, immediately respond with an error
        if self.shutdown_requested {
            self.respond(lsp_server::Response::new_err(
                request.id,
                lsp_server::ErrorCode::InvalidRequest as i32,
                "shutdown was requested".to_owned(),
            ));
            return Ok(());
        }

        // Dispatch the event based on the type of event
        RequestDispatcher::new(self, request)
            .on::<lsp_types::request::Shutdown>(|state, _request| {
                state.shutdown_requested = true;
                Ok(())
            })?
            .finish();

        Ok(())
    }

    /// Handles a notification from the language server client
    fn on_notification(&mut self, notification: lsp_server::Notification) -> Result<()> {
        NotificationDispatcher::new(self, notification)
            .on::<lsp_types::notification::DidOpenTextDocument>(|state, params| {
                if let Ok(path) = convert_uri(&params.text_document.uri) {
                    state.open_docs.insert(path.clone());
                    state
                        .vfs
                        .write()
                        .set_file_contents(&path, Some(params.text_document.text.into_bytes()));
                }
                Ok(())
            })?
            .on::<lsp_types::notification::DidChangeTextDocument>(|state, params| {
                let lsp_types::DidChangeTextDocumentParams {
                    text_document,
                    content_changes,
                } = params;
                if let Ok(path) = convert_uri(&text_document.uri) {
                    let new_content = content_changes.get(0).unwrap().text.clone();
                    state
                        .vfs
                        .write()
                        .set_file_contents(&path, Some(new_content.into_bytes()));
                }
                Ok(())
            })?
            .on::<lsp_types::notification::DidCloseTextDocument>(|state, params| {
                if let Ok(path) = convert_uri(&params.text_document.uri) {
                    state.open_docs.remove(&path);
                    state.vfs_monitor.reload(&path);
                }
                // Clear any diagnostics that we may have send
                state.send_notification::<lsp_types::notification::PublishDiagnostics>(
                    lsp_types::PublishDiagnosticsParams {
                        uri: params.text_document.uri,
                        diagnostics: Vec::new(),
                        version: None,
                    },
                );
                Ok(())
            })?
            .on::<lsp_types::notification::DidChangeWatchedFiles>(|state, params| {
                for change in params.changes {
                    if let Ok(path) = convert_uri(&change.uri) {
                        state.vfs_monitor.reload(&path);
                    }
                }
                Ok(())
            })?
            .finish();
        Ok(())
    }

    /// Registers a request with the server. We register all these request to make sure they all get
    /// handled and so we can measure the time it takes for them to complete from the point of view
    /// of the client.
    fn register_request(&mut self, request: &lsp_server::Request, request_received: Instant) {
        self.request_queue.incoming.register(
            request.id.clone(),
            (request.method.clone(), request_received),
        )
    }

    /// Sends a request to the client and registers the request so that we can handle the response.
    pub(crate) fn send_request<R: lsp_types::request::Request>(
        &mut self,
        params: R::Params,
        handler: RequestHandler,
    ) {
        let request = self
            .request_queue
            .outgoing
            .register(R::METHOD.to_string(), params, handler);
        self.send(request.into());
    }

    /// Sends a notification to the client
    pub(crate) fn send_notification<N: lsp_types::notification::Notification>(
        &mut self,
        params: N::Params,
    ) {
        let not = lsp_server::Notification::new(N::METHOD.to_string(), params);
        self.send(not.into());
    }

    /// Handles a response to a request we made. The response gets forwarded to where we made the
    /// request from.
    fn complete_request(&mut self, response: lsp_server::Response) {
        let handler = self.request_queue.outgoing.complete(response.id.clone());
        handler(self, response)
    }

    /// Sends a response to a request to the client. This method logs the time it took us to reply
    /// to a request from the client.
    pub(crate) fn respond(&mut self, response: lsp_server::Response) {
        if let Some((_method, start)) = self.request_queue.incoming.complete(response.id.clone()) {
            let duration = start.elapsed();
            log::info!("handled req#{} in {:?}", response.id, duration);
            self.send(response.into());
        }
    }

    /// Sends a message to the client
    fn send(&mut self, message: lsp_server::Message) {
        self.sender
            .send(message)
            .expect("error sending lsp message to the outgoing channel")
    }
}

/// Runs the main loop of the language server. This will receive requests and handle them.
pub fn main_loop(connection: Connection, config: Config) -> Result<()> {
    log::info!("initial config: {:#?}", config);
    LanguageServerState::new(connection.sender, config).run(connection.receiver)
}

/// Send all diagnostics of all files
fn handle_diagnostics(state: LanguageServerSnapshot, sender: Sender<Task>) -> Cancelable<()> {
    // Iterate over all files
    for (idx, _package) in state.packages.iter().enumerate() {
        let package_id = hir::PackageId(idx as u32);

        // Get all the files
        let files = state.analysis.package_source_files(package_id)?;

        // Publish all diagnostics
        for file in files {
            let line_index = state.analysis.file_line_index(file)?;
            let uri = state.file_id_to_uri(file).unwrap();
            let diagnostics = state.analysis.diagnostics(file)?;

            let diagnostics = {
                let mut lsp_diagnostics = Vec::with_capacity(diagnostics.len());
                for d in diagnostics {
                    lsp_diagnostics.push(lsp_types::Diagnostic {
                        range: convert_range(d.range, &line_index),
                        severity: Some(lsp_types::DiagnosticSeverity::Error),
                        code: None,
                        code_description: None,
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
                        data: None,
                    });
                }
                lsp_diagnostics
            };

            sender
                .send(Task::Notify(lsp_server::Notification {
                    method: PublishDiagnostics::METHOD.to_owned(),
                    params: to_json(PublishDiagnosticsParams {
                        uri,
                        diagnostics,
                        version: None,
                    })
                    .unwrap(),
                }))
                .unwrap()
        }
    }
    Ok(())
}

/// Handles a task send by another async task
fn handle_task(task: Task, state: &mut LanguageServerState) -> Result<()> {
    match task {
        Task::Notify(notification) => {
            state.send(notification.into());
        }
    }
    Ok(())
}

/// Handles a change to the underlying virtual file system.
fn handle_vfs_task(mut task: vfs::MonitorMessage, state: &mut LanguageServerState) -> Result<()> {
    loop {
        match task {
            vfs::MonitorMessage::Progress { .. } => {}
            vfs::MonitorMessage::Loaded { files } => {
                let vfs = &mut *state.vfs.write();
                for (path, contents) in files {
                    vfs.set_file_contents(&path, contents);
                }
            }
        }

        // Coalesce many VFS events into a single loop turn
        task = match state.vfs_monitor_receiver.try_recv() {
            Ok(task) => task,
            _ => break,
        }
    }
    Ok(())
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
    pub fn process_vfs_changes(&mut self) -> bool {
        // Get all the changes since the last time we processed
        let changed_files = {
            let mut vfs = self.vfs.write();
            vfs.take_changes()
        };
        if changed_files.is_empty() {
            return false;
        }

        // Construct an AnalysisChange to apply to the analysis
        let vfs = self.vfs.read();
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
    pub fn file_id_to_uri(&self, id: hir::FileId) -> Result<Url> {
        let vfs = self.vfs.read();
        let path = vfs.file_path(vfs::FileId(id.0));
        let url = url_from_path_with_drive_lowercasing(path)?;

        Ok(url)
    }
}
