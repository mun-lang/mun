extern crate cargo;
extern crate failure;
extern crate libloading;

mod error;
mod library;
mod module;

pub use crate::error::*;
pub use crate::library::Library;
pub use crate::module::Module;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use mun_symbols::prelude::*;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

/// A runtime for the Mun scripting language.
pub struct MunRuntime {
    modules: HashMap<PathBuf, Module>,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,
}

impl MunRuntime {
    /// Constructs a new `MunRuntime`, for which the file watcher is triggered with an interval of
    /// `dur`.
    pub fn new(dur: Duration) -> Result<MunRuntime> {
        let (tx, rx) = channel();

        let watcher: RecommendedWatcher = Watcher::new(tx, dur)?;

        Ok(MunRuntime {
            modules: HashMap::new(),
            watcher,
            watcher_rx: rx,
        })
    }

    /// Adds a module corresponding to the manifest at `manifest_path`.
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

    /// Removes the module corresponding to the manifest at `src`.
    pub fn remove_module(&mut self, src: &Path) {
        self.modules.remove(src);
    }

    /// Invokes the method `method_name` with arguments `args`, in the library compiled based on
    /// the manifest at `manifest_path`.
    ///
    /// If an error occurs when invoking the method, an error message is logged. The runtime
    /// continues looping until the cause of the error has been resolved.
    pub fn invoke_library_method<Output: Reflection + Clone + 'static>(
        &mut self,
        manifest_path: &Path,
        method_name: &str,
        args: &[&dyn Reflectable],
    ) -> Output {
        // Initialize `updated` to `true` to guarantee the method is run at least once
        let mut updated = true;
        loop {
            if updated {
                match self.try_invoke_library_method(manifest_path, method_name, args) {
                    Ok(res) => return res,
                    Err(ref e) => {
                        eprintln!("{}", e);
                        updated = false;
                    }
                }
            } else {
                updated = self.update();
            }
        }
    }

    /// Tries to invoke the method `method_name` with arguments `args`, in the library compiled
    /// based on the manifest at `manifest_path`.
    ///
    /// Returns an error message upon failure.
    fn try_invoke_library_method<Output: Reflection + Clone + 'static>(
        &self,
        manifest_path: &Path,
        method_name: &str,
        args: &[&dyn Reflectable],
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

        let result = method.invoke(args)?;

        let result = result.downcast_ref::<Output>().ok_or(format!(
            "Failed to downcast return value. Expected: {}. Found: {}.",
            Output::type_info().name,
            if let Some(return_type) = method_info.returns {
                format!("{}", return_type.name)
            } else {
                "()".to_string()
            }
        ))?;

        Ok(result.clone())
    }

    /// Retrieves the module corresponding to the manifest at `manifest_path`.
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

    /// Updates the state of the runtime. This includes checking for file changes, and consequent
    /// recompilation.
    ///
    /// Currently, the runtime can crash if recompilation fails. Ideally, there is a fallback.
    pub fn update(&mut self) -> bool {
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

                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}
