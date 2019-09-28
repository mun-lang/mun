extern crate cargo;
extern crate failure;
extern crate libloading;

mod assembly;
mod error;
mod library;
#[macro_use]
mod macros;

pub use crate::assembly::Assembly;
pub use crate::error::*;
pub use crate::library::Library;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use mun_abi::{FunctionInfo, Reflection};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

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
    pub fn new(library_path: &Path, dur: Duration) -> Result<MunRuntime> {
        let (tx, rx) = channel();

        let watcher: RecommendedWatcher = Watcher::new(tx, dur)?;
        let mut runtime = MunRuntime {
            assemblies: HashMap::new(),
            function_table: HashMap::new(),
            watcher,
            watcher_rx: rx,
        };

        runtime.add_assembly(library_path)?;
        Ok(runtime)
    }

    /// Adds an assembly corresponding to the library at `library_path`.
    fn add_assembly(&mut self, library_path: &Path) -> Result<()> {
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

        println!("Watching assembly: {}", library_path.to_string_lossy());
        Ok(())
    }

    /// Removes the assembly corresponding to the library at `library_path`.
    fn remove_assembly(&mut self, library_path: &Path) {
        self.assemblies.remove(library_path);
    }

    /// Retrieves the function information corresponding to `function_name`, if available.
    pub fn get_function_info(&self, function_name: &str) -> Option<&FunctionInfo> {
        self.function_table.get(function_name)
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
                    println!("{:?}", path);
                    return true;
                    // for ancestor in path.ancestors() {
                    //     let mut library_path = ancestor.to_path_buf();
                    //     library_path.push("Cargo.toml");

                    //     if let Some(module) = self.modules.get_mut(&manifest_path) {
                    //         module.unload();
                    //         match module.compile() {
                    //             Ok(ref output_path) => {
                    //                 if let Err(e) = module.load(output_path) {
                    //                     println!(
                    //                         "An error occured while loading library '{}': {:?}",
                    //                         module.manifest_path().to_string_lossy(),
                    //                         e,
                    //                     )
                    //                 }
                    //             }
                    //             Err(e) => println!(
                    //                 "An error occured while compiling library '{}': {:?}",
                    //                 module.manifest_path().to_string_lossy(),
                    //                 e,
                    //             ),
                    //         }

                    //         return true;
                    //     }
                    // }
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

#[cfg(test)]
mod tests {
    use super::{invoke_fn, MunRuntime};
    use std::path::PathBuf;
    use std::time::Duration;

    fn test_lib_path() -> PathBuf {
        use std::env;

        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        manifest_dir.join("tests/data").join("main.dll")
    }

    #[test]
    fn mun_new_runtime() {
        let _runtime = MunRuntime::new(&test_lib_path(), Duration::from_millis(10))
            .expect("Failed to initialize Mun runtime.");
    }

    #[test]
    fn mun_invoke_fn() {
        let mut runtime = MunRuntime::new(&test_lib_path(), Duration::from_millis(10))
            .expect("Failed to initialize Mun runtime.");

        let a: f64 = 4.0;
        let b: f64 = 2.0;

        let result: f64 = invoke_fn!(runtime, "add", a, b);

        assert_eq!(result, a + b);
    }
}
