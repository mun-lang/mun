use crate::{
    analysis::{Analysis, AnalysisSnapshot},
    change::AnalysisChange,
    config::Config,
    conversion::{convert_range, url_from_path_with_drive_lowercasing},
    state::utils::Progress,
    to_json,
};
use crossbeam_channel::{select, unbounded, Receiver, Sender};
use lsp_server::{ReqQueue, Response};
use lsp_types::{
    notification::Notification, notification::PublishDiagnostics, PublishDiagnosticsParams, Url,
};
use parking_lot::RwLock;
use paths::AbsPathBuf;
use rustc_hash::FxHashSet;
use std::{convert::TryFrom, ops::Deref, sync::Arc, time::Instant};
use vfs::VirtualFileSystem;

mod protocol;
mod utils;
mod workspace;

/// A `Task` is something that is send from async tasks to the entry point for processing. This
/// enables synchronizing resources like the connection with the client.
#[derive(Debug)]
pub(crate) enum Task {
    Response(Response),
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
    pub(crate) sender: Sender<lsp_server::Message>,

    /// The request queue keeps track of all incoming and outgoing requests.
    pub(crate) request_queue: lsp_server::ReqQueue<(String, Instant), RequestHandler>,

    /// The configuration passed by the client
    pub config: Config,

    /// Thread pool for async execution
    pub thread_pool: threadpool::ThreadPool,

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
            thread_pool: threadpool::ThreadPool::default(),
            task_sender,
            task_receiver,
            analysis,
            packages: Arc::new(Vec::new()),
            shutdown_requested: false,
        }
    }

    /// Blocks until a new event is received from one of the many channels the language server
    /// listens to. Returns the first event that is received.
    fn next_event(&self, receiver: &Receiver<lsp_server::Message>) -> Option<Event> {
        select! {
            recv(receiver) -> msg => msg.ok().map(Event::Lsp),
            recv(self.vfs_monitor_receiver) -> task => Some(Event::Vfs(task.unwrap())),
            recv(self.task_receiver) -> task => Some(Event::Task(task.unwrap()))
        }
    }

    /// Runs the language server to completion
    pub fn run(mut self, receiver: Receiver<lsp_server::Message>) -> anyhow::Result<()> {
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
    fn handle_event(&mut self, event: Event) -> anyhow::Result<()> {
        let start_time = Instant::now();
        log::info!("handling event: {:?}", event);

        // Process the incoming event
        match event {
            Event::Task(task) => self.handle_task(task)?,
            Event::Lsp(msg) => match msg {
                lsp_server::Message::Request(req) => self.on_request(req, start_time)?,
                lsp_server::Message::Response(resp) => self.complete_request(resp),
                lsp_server::Message::Notification(not) => self.on_notification(not)?,
            },
            Event::Vfs(task) => self.handle_vfs_task(task)?,
        };

        // Process any changes to the vfs
        let state_changed = self.process_vfs_changes();
        if state_changed {
            let snapshot = self.snapshot();
            let task_sender = self.task_sender.clone();
            // Spawn the diagnostics in the threadpool
            self.thread_pool.execute(move || {
                let _result = handle_diagnostics(snapshot, task_sender);
            });
        }

        Ok(())
    }

    /// Handles a task sent by another async task
    fn handle_task(&mut self, task: Task) -> anyhow::Result<()> {
        match task {
            Task::Notify(notification) => {
                self.send(notification.into());
            }
            Task::Response(response) => self.respond(response),
        }
        Ok(())
    }

    /// Handles a change to the underlying virtual file system.
    fn handle_vfs_task(&mut self, mut task: vfs::MonitorMessage) -> anyhow::Result<()> {
        loop {
            match task {
                vfs::MonitorMessage::Progress { total, done } => {
                    let progress_state = if done == 0 {
                        Progress::Begin
                    } else if done < total {
                        Progress::Report
                    } else {
                        Progress::End
                    };
                    self.report_progress(
                        "projects scanned",
                        progress_state,
                        Some(format!("{}/{}", done, total)),
                        Some(Progress::fraction(done, total)),
                    )
                }
                vfs::MonitorMessage::Loaded { files } => {
                    let vfs = &mut *self.vfs.write();
                    for (path, contents) in files {
                        vfs.set_file_contents(&path, contents);
                    }
                }
            }

            // Coalesce many VFS events into a single loop turn
            task = match self.vfs_monitor_receiver.try_recv() {
                Ok(task) => task,
                _ => break,
            }
        }
        Ok(())
    }
}

/// Sends all diagnostics of all files
fn handle_diagnostics(state: LanguageServerSnapshot, sender: Sender<Task>) -> anyhow::Result<()> {
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
                .unwrap();
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
    pub fn file_id_to_uri(&self, id: hir::FileId) -> anyhow::Result<Url> {
        let vfs = self.vfs.read();
        let path = vfs.file_path(vfs::FileId(id.0));
        let url = url_from_path_with_drive_lowercasing(path)?;

        Ok(url)
    }

    /// Converts the specified `Url` to a `hir::FileId`
    pub fn uri_to_file_id(&self, url: &Url) -> anyhow::Result<hir::FileId> {
        url.to_file_path()
            .map_err(|_| anyhow::anyhow!("invalid uri: {}", url))
            .and_then(|path| {
                AbsPathBuf::try_from(path)
                    .map_err(|_| anyhow::anyhow!("url does not refer to absolute path: {}", url))
            })
            .and_then(|path| {
                self.vfs
                    .read()
                    .file_id(&path)
                    .ok_or_else(|| anyhow::anyhow!("url does not refer to a file: {}", url))
                    .map(|id| hir::FileId(id.0))
            })
    }
}

impl Drop for LanguageServerState {
    fn drop(&mut self) {
        self.analysis.request_cancelation();
        self.thread_pool.join();
    }
}
