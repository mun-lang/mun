mod assembly;
mod library;
#[macro_use]
mod macros;

pub use crate::assembly::Assembly;
pub use crate::library::Library;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use failure::Error;
use mun_abi::{FunctionInfo, Reflection};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    pub library_path: PathBuf,
    pub delay: Duration,
}

/// A builder for the runtime.
pub struct RuntimeBuilder {
    options: RuntimeOptions,
}

impl RuntimeBuilder {
    pub fn new<P: Into<PathBuf>>(library_path: P) -> Self {
        Self {
            options: RuntimeOptions {
                library_path: library_path.into(),
                delay: Duration::from_millis(10),
            },
        }
    }

    pub fn set_delay(&mut self, delay: Duration) -> &mut Self {
        self.options.delay = delay;
        self
    }

    pub fn spawn(self) -> Result<MunRuntime, Error> {
        MunRuntime::new(self.options)
    }
}

/// A runtime for the Mun scripting language.
pub struct MunRuntime {
    assemblies: HashMap<PathBuf, Assembly>,
    function_table: HashMap<String, FunctionInfo>,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,
}

impl MunRuntime {
    /// Constructs a new `MunRuntime` that loads the library at `library_path` and its
    /// dependencies. The `MunRuntime` contains a file watcher that is triggered with an interval
    /// of `dur`.
    pub fn new(options: RuntimeOptions) -> Result<MunRuntime, Error> {
        let (tx, rx) = channel();

        let watcher: RecommendedWatcher = Watcher::new(tx, options.delay)?;
        let mut runtime = MunRuntime {
            assemblies: HashMap::new(),
            function_table: HashMap::new(),
            watcher,
            watcher_rx: rx,
        };

        runtime.add_assembly(&options.library_path)?;
        Ok(runtime)
    }

    /// Adds an assembly corresponding to the library at `library_path`.
    fn add_assembly(&mut self, library_path: &Path) -> Result<(), Error> {
        let library_path = library_path.canonicalize()?;
        if self.assemblies.contains_key(&library_path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "An assembly with the same name already exists.",
            )
            .into());
        }

        let assembly = Assembly::load(&library_path)?;
        for function in assembly.functions() {
            self.function_table
                .insert(function.name().to_string(), function.clone());
        }

        self.assemblies.insert(library_path.clone(), assembly);

        self.watcher
            .watch(library_path.clone(), RecursiveMode::NonRecursive)?;

        Ok(())
    }

    //    /// Removes the assembly corresponding to the library at `library_path`.
    //    fn remove_assembly(&mut self, library_path: &Path) {
    //        self.assemblies.remove(library_path);
    //    }

    /// Retrieves the function information corresponding to `function_name`, if available.
    pub fn get_function_info(&self, function_name: &str) -> Option<&FunctionInfo> {
        self.function_table.get(function_name)
    }

    /// Updates the state of the runtime. This includes checking for file changes, and reloading
    /// compiled assemblies.
    pub fn update(&mut self) -> bool {
        while let Ok(event) = self.watcher_rx.try_recv() {
            use notify::DebouncedEvent::*;
            match event {
                Write(ref path) => {
                    if let Some(assembly) = self.assemblies.get_mut(path) {
                        for function in assembly.functions() {
                            self.function_table.remove(function.name());
                        }
                        if let Err(e) = assembly.swap(path) {
                            println!(
                                "An error occured while reloading assembly '{}': {:?}",
                                path.to_string_lossy(),
                                e
                            );
                        } else {
                            for function in assembly.functions() {
                                self.function_table
                                    .insert(function.name().to_string(), function.clone());
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

invoke_fn_impl! {
    fn invoke_fn0();
    fn invoke_fn1(a: A);
    fn invoke_fn2(a: A, b: B);
    fn invoke_fn3(a: A, b: B, c: C);
    fn invoke_fn4(a: A, b: B, c: C, d: D);
    fn invoke_fn5(a: A, b: B, c: C, d: D, e: E);
    fn invoke_fn6(a: A, b: B, c: C, d: D, e: E, f: F);
    fn invoke_fn7(a: A, b: B, c: C, d: D, e: E, f: F, g: G);
    fn invoke_fn8(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H);
    fn invoke_fn9(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
    fn invoke_fn10(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
    fn invoke_fn11(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K);
    fn invoke_fn12(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L);
}

#[cfg(all(test, windows))]
mod tests {
    use super::{invoke_fn, RuntimeBuilder};
    use std::path::PathBuf;

    fn test_lib_path() -> PathBuf {
        use std::env;

        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        manifest_dir.join("tests/data").join("main.dll")
    }

    #[test]
    fn mun_new_runtime() {
        let builder = RuntimeBuilder::new(test_lib_path());
        let _runtime = builder.spawn().expect("Failed to initialize Mun runtime.");
    }

    #[test]
    fn mun_invoke_fn() {
        let builder = RuntimeBuilder::new(test_lib_path());
        let mut runtime = builder.spawn().expect("Failed to initialize Mun runtime.");

        let a: f64 = 4.0;
        let b: f64 = 2.0;

        let result: f64 = invoke_fn!(runtime, "add", a, b);

        assert_eq!(result, a + b);
    }
}
