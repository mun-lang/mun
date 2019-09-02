extern crate cargo;
extern crate failure;
extern crate libloading;

mod error;
mod library;
mod module;

pub use crate::error::*;
pub use crate::library::Library;
pub use crate::module::Module;

use core::any::Any;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use mun_symbols::prelude::*;
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

    pub fn invoke_library_method<Output: Clone + 'static>(
        &self,
        manifest_path: &Path,
        method_name: &str,
        args: &[&dyn Any],
    ) -> StdResult<Output, String> {
        let module: &Module = self.get_module(manifest_path).map_err(|_| {
            format!(
                "Unknown module with manifest path: {}",
                manifest_path.to_string_lossy()
            )
        })?;

        let library: &Library = module.library();
        let symbols: &ModuleInfo = library.module_info();

        let method_info = symbols.get_method(method_name).ok_or(format!(
            "Failed to obtain method info for {module}::{method}.",
            module = module.manifest_path().to_string_lossy(),
            method = method_name
        ))?;

        let method = method_info.load(library.inner()).map_err(|_| {
            format!(
                "Failed to load method symbol for {module}::{method}.",
                module = module.manifest_path().to_string_lossy(),
                method = method_name
            )
        })?;

        Ok(method
            .invoke(args)
            .expect("Failed to invoke method.")
            .downcast_ref::<Output>()
            .expect("Failed to downcast return value.")
            .clone())
    }

    pub fn get_module(&self, manifest_path: &Path) -> Result<&Module> {
        let manifest_path = manifest_path.canonicalize()?;

        self.modules.get(&manifest_path).ok_or(
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "The module at path '{}' cannot be found.",
                    manifest_path.to_string_lossy()
                ),
            )
            .into(),
        )
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
