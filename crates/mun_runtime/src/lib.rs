extern crate libloading;

mod error;
mod library;
mod module;

pub use crate::error::*;
pub use crate::library::{Library, Symbol};

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use crate::module::Module;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

pub struct MunRuntime {
    modules: HashMap<PathBuf, Module>,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,
}

impl MunRuntime {
    pub fn new(dur: Duration) -> Result<MunRuntime> {
        let (tx, rx) = channel();

        let watcher: RecommendedWatcher = Watcher::new(tx, dur)?;

        Ok(MunRuntime {
            modules: HashMap::new(),
            watcher,
            watcher_rx: rx,
        })
    }

    pub fn add_module(&mut self, src: &Path, dst: &Path, recursive: bool) -> Result<()> {
        let module = Module::new(src, dst)?;
        let key = module.src().to_path_buf();
        if self.modules.contains_key(&key) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "A module with the same name already exists.",
            )
            .into());
        }

        self.modules.insert(key.clone(), module);

        self.watcher.watch(
            &key,
            if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            },
        )?;

        println!("Watching directory: {}", src.to_string_lossy());

        // TODO: compile src to dst

        let module = self.modules.get_mut(&key).unwrap();
        module.load()?;

        Ok(())
    }

    pub fn remove_module(&mut self, src: &Path) {
        self.modules.remove(src);
    }

    pub fn get_symbol<T>(&self, module: &Path, name: &str) -> Result<Symbol<T>> {
        let src = module.canonicalize()?;
        if let Some(module) = self.modules.get(&src) {
            match module.library() {
                Some(ref lib) => lib.get_fn(name),
                None => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "The library at path '{}' has not been loaded.",
                        src.to_string_lossy()
                    ),
                )
                .into()),
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "The module at path '{}' cannot be found.",
                    src.to_string_lossy()
                ),
            )
            .into())
        }
    }

    pub fn update(&mut self) {
        while let Ok(event) = self.watcher_rx.try_recv() {
            println!("{:?}", event);
            use notify::DebouncedEvent::*;
            match event {
                NoticeWrite(ref path) | Write(ref path) | Create(ref path) => {
                    for ancestor in path.ancestors() {
                        if let Some(module) = self.modules.get_mut(ancestor) {
                            module.unload();

                            // TODO: compile
                            // For now, you need to manually run 'cargo build' in the command line, and press a key to continue
                            let stdin = io::stdin();
                            let mut input = String::new();
                            stdin.read_line(&mut input);

                            if let Err(e) = module.load() {
                                println!(
                                    "An error occured while loading library '{}': {:?}",
                                    module.dst().to_string_lossy(),
                                    e,
                                )
                            }

                            return;
                        }
                    }
                }
                _ => {}
            }
        }
        // Err(e) => println!("watch error: {:?}", e)
    }
}
