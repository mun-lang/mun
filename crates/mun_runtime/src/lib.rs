extern crate cargo;
extern crate failure;
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

    pub fn add_manifest(&mut self, manifest_path: &Path) -> Result<()> {
        let mut module = Module::new(manifest_path)?;

        if self.modules.contains_key(module.manifest_path()) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "A module with the same name already exists.",
            )
            .into());
        }

        let mut source_path = module.manifest_path().parent().unwrap().to_path_buf();
        source_path.push("src");

        let output_path = module.compile()?;
        module.load(&output_path)?;

        self.modules
            .insert(module.manifest_path().to_path_buf(), module);

        self.watcher
            .watch(source_path.clone(), RecursiveMode::Recursive)?;

        println!("Watching directory: {}", source_path.to_string_lossy());
        Ok(())
    }

    pub fn remove_module(&mut self, src: &Path) {
        self.modules.remove(src);
    }

    pub fn get_symbol<T>(&self, manifest_path: &Path, name: &str) -> Result<Symbol<T>> {
        let manifest_path = manifest_path.canonicalize()?;

        if let Some(module) = self.modules.get(&manifest_path) {
            match module.library() {
                Some(ref lib) => lib.get_fn(name),
                None => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "The library at path '{}' has not been loaded.",
                        manifest_path.to_string_lossy()
                    ),
                )
                .into()),
            }
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "The module at path '{}' cannot be found.",
                    manifest_path.to_string_lossy()
                ),
            )
            .into())
        }
    }

    pub fn update(&mut self) {
        while let Ok(event) = self.watcher_rx.try_recv() {
            use notify::DebouncedEvent::*;
            match event {
                Write(ref path) | Create(ref path) => {
                    for ancestor in path.ancestors() {
                        let mut manifest_path = ancestor.to_path_buf();
                        manifest_path.push("Cargo.toml");

                        if let Some(module) = self.modules.get_mut(&manifest_path) {
                            module.unload();
                            match module.compile() {
                                Ok(ref output_path) => {
                                    if let Err(e) = module.load(output_path) {
                                        println!(
                                            "An error occured while loading library '{}': {:?}",
                                            module.manifest_path().to_string_lossy(),
                                            e,
                                        )
                                    }
                                }
                                Err(e) => println!(
                                    "An error occured while compiling library '{}': {:?}",
                                    module.manifest_path().to_string_lossy(),
                                    e,
                                ),
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
