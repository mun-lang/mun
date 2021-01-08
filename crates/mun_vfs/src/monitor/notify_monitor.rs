use super::{Monitor, MonitorConfig, MonitorDirectories, MonitorEntry, MonitorMessage};
use crate::{AbsPath, AbsPathBuf};
use crossbeam_channel::{never, select, unbounded, Receiver, Sender};
use notify::{RecursiveMode, Watcher};
use std::{convert::TryFrom, thread};
use walkdir::WalkDir;

/// A message that can be sent from the "foreground" to the background thread.
#[derive(Debug)]
enum ForegroundMessage {
    /// Notifies the background tasks that the configuration has changed
    ConfigChanged(MonitorConfig),

    /// Notifies the background tasks that the specified path should be reloaded
    Reload(AbsPathBuf),
}

#[derive(Debug)]
pub struct NotifyMonitor {
    sender: Sender<ForegroundMessage>,
    thread: thread::JoinHandle<()>,
}

impl Monitor for NotifyMonitor {
    fn new(sender: super::Sender) -> Self
    where
        Self: Sized,
    {
        let background_thread = NotifyThread::new(sender);
        let (sender, receiver) = unbounded::<ForegroundMessage>();
        let thread = thread::Builder::new()
            .spawn(move || background_thread.run(receiver))
            .expect("failed to spawn notify background thread");
        NotifyMonitor { sender, thread }
    }

    fn set_config(&mut self, config: MonitorConfig) {
        self.sender
            .send(ForegroundMessage::ConfigChanged(config))
            .expect("could not send new configuration to background thread");
    }

    fn reload(&mut self, path: &AbsPath) {
        self.sender
            .send(ForegroundMessage::Reload(path.to_path_buf()))
            .expect("could not send reload message to background thread");
    }
}

type NotifyEvent = notify::Result<notify::Event>;

/// A struct that manages the notify watchers and processes the changes.
struct NotifyThread {
    sender: super::Sender,
    watched_entries: Vec<MonitorEntry>,
    watcher: Option<(notify::RecommendedWatcher, Receiver<NotifyEvent>)>,
}

/// A message to be processed by the `NotifyThread`.
enum NotifyThreadEvent {
    ForegroundMessage(ForegroundMessage),
    NotifyEvent(NotifyEvent),
}

impl NotifyThread {
    /// Constructs a new instance of `Self`
    pub fn new(sender: super::Sender) -> Self {
        NotifyThread {
            sender,
            watched_entries: Vec::new(),
            watcher: None,
        }
    }

    /// Returns the next event to process.
    fn next_event(&self, receiver: &Receiver<ForegroundMessage>) -> Option<NotifyThreadEvent> {
        let watcher_receiver = self.watcher.as_ref().map(|(_, receiver)| receiver);
        select! {
            recv(receiver) -> it => it.ok().map(NotifyThreadEvent::ForegroundMessage),
            recv(watcher_receiver.unwrap_or(&never())) -> it => Some(NotifyThreadEvent::NotifyEvent(it.unwrap())),
        }
    }

    /// Runs the background thread until there are no more messages to receive
    pub fn run(mut self, receiver: Receiver<ForegroundMessage>) {
        while let Some(event) = self.next_event(&receiver) {
            match event {
                NotifyThreadEvent::ForegroundMessage(message) => match message {
                    ForegroundMessage::ConfigChanged(config) => self.set_config(config),
                    ForegroundMessage::Reload(path) => {
                        let contents = read(&path);
                        let files = vec![(path, contents)];
                        self.send(MonitorMessage::Loaded { files });
                    }
                },
                NotifyThreadEvent::NotifyEvent(event) => {
                    if let Some(event) = log_notify_error(event) {
                        let files = event
                            .paths
                            .into_iter()
                            .map(|path| {
                                AbsPathBuf::try_from(path)
                                    .expect("could not convert notify event path to absolute path")
                            })
                            .filter_map(|path| {
                                if path.is_dir()
                                    && self
                                        .watched_entries
                                        .iter()
                                        .any(|entry| entry.contains_dir(&path))
                                {
                                    self.watch(path);
                                    None
                                } else if !path.is_file()
                                    || !self
                                        .watched_entries
                                        .iter()
                                        .any(|entry| entry.contains_file(&path))
                                {
                                    None
                                } else {
                                    let contents = read(&path);
                                    Some((path, contents))
                                }
                            })
                            .collect::<Vec<_>>();
                        if !files.is_empty() {
                            self.send(MonitorMessage::Loaded { files });
                        }
                    }
                }
            }
        }
    }

    /// Updates the configuration to `config`
    fn set_config(&mut self, config: MonitorConfig) {
        // Reset the previous watcher and possibly construct a new one
        self.watcher = None;
        if !config.watch.is_empty() {
            let (watcher_sender, watcher_receiver) = unbounded();
            let watcher = log_notify_error(Watcher::new_immediate(move |event| {
                watcher_sender
                    .send(event)
                    .expect("unable to send notify event over channel")
            }));
            self.watcher = watcher.map(|it| (it, watcher_receiver));
        }

        // Update progress
        let total_entries = config.load.len();
        self.send(MonitorMessage::Progress {
            total: total_entries,
            done: 0,
        });

        // Update the current set of entries
        self.watched_entries.clear();
        for (i, entry) in config.load.into_iter().enumerate() {
            let watch = config.watch.contains(&i);
            if watch {
                self.watched_entries.push(entry.clone());
            }

            let files = self.load_entry(entry, watch);
            self.send(MonitorMessage::Loaded { files });
            self.send(MonitorMessage::Progress {
                total: total_entries,
                done: i + 1,
            });
        }
    }

    /// Loads all the files from the given entry and optionally adds to the watched entries
    fn load_entry(
        &mut self,
        entry: MonitorEntry,
        watch: bool,
    ) -> Vec<(AbsPathBuf, Option<Vec<u8>>)> {
        match entry {
            MonitorEntry::Files(files) => self.load_files_entry(files, watch),
            MonitorEntry::Directories(dirs) => self.load_directories_entry(dirs, watch),
        }
    }

    /// Loads all the files and optionally adds to watched entries
    fn load_files_entry(
        &mut self,
        files: Vec<AbsPathBuf>,
        watch: bool,
    ) -> Vec<(AbsPathBuf, Option<Vec<u8>>)> {
        files
            .into_iter()
            .map(|file| {
                if watch {
                    self.watch(&file);
                }
                let contents = read(&file);
                (file, contents)
            })
            .collect()
    }

    /// Loads all the files from the specified directories and optionally starts watching them.
    fn load_directories_entry(
        &mut self,
        dirs: MonitorDirectories,
        watch: bool,
    ) -> Vec<(AbsPathBuf, Option<Vec<u8>>)> {
        let mut result = Vec::new();
        for root in dirs.include.iter() {
            let walkdir = WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_entry(|entry| {
                    if !entry.file_type().is_dir() {
                        true
                    } else {
                        let path = AbsPath::assert_new(entry.path());
                        root == path
                            || dirs
                                .exclude
                                .iter()
                                .chain(&dirs.include)
                                .all(|dir| dir != path)
                    }
                });

            let files = walkdir.filter_map(Result::ok).filter_map(|entry| {
                let is_dir = entry.file_type().is_dir();
                let is_file = entry.file_type().is_file();
                let abs_path = AbsPathBuf::try_from(entry.into_path())
                    .expect("could not convert walkdir entry to absolute path");
                if is_dir && watch {
                    self.watch(&abs_path);
                }
                if !is_file {
                    None
                } else {
                    let ext = abs_path.extension().unwrap_or_default();
                    if dirs.extensions.iter().all(|entry| entry.as_str() != ext) {
                        None
                    } else {
                        Some(abs_path)
                    }
                }
            });

            result.extend(files.map(|file| {
                let contents = read(&file);
                (file, contents)
            }));
        }

        result
    }

    /// Sends a message to the foreground.
    fn send(&mut self, message: MonitorMessage) {
        (self.sender)(message);
    }

    /// Start watching the file at the specified path
    fn watch(&mut self, path: impl AsRef<AbsPath>) {
        if let Some((watcher, _)) = &mut self.watcher {
            log_notify_error(watcher.watch(path.as_ref(), RecursiveMode::NonRecursive));
        }
    }
}

/// A helper function that reads the contents of the specified file and returns it.
fn read(path: impl AsRef<AbsPath>) -> Option<Vec<u8>> {
    std::fs::read(path.as_ref()).ok()
}

/// A helper function to load a warning for a "notify" error.
fn log_notify_error<T>(res: notify::Result<T>) -> Option<T> {
    res.map_err(|err| log::warn!("notify error: {}", err)).ok()
}

#[cfg(test)]
mod tests {
    use super::{Monitor, NotifyMonitor};

    #[test]
    fn construct() {
        let _monitor = NotifyMonitor::new(Box::new(|_| {}));
    }
}
