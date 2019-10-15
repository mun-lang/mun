use std::sync::mpsc::channel;
use std::time::Duration;

use failure::Error;
use mun_compiler::{CompilerOptions, PathOrInline};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

pub fn main(options: &CompilerOptions) -> Result<(), Error> {
    // Need to canonicalize path to do comparisons
    let input_path = match &options.input {
        PathOrInline::Path(path) => path.canonicalize()?,
        PathOrInline::Inline(_) => panic!("cannot run compiler with inline path"),
    };

    // Compile at least once
    mun_compiler::main(&options)?;

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(10))?;
    watcher.watch(&input_path, RecursiveMode::NonRecursive)?;

    loop {
        use notify::DebouncedEvent::*;
        match rx.recv() {
            Ok(Write(ref path)) => {
                // TODO: Check whether file contents changed (using sha hash?)
                if *path == input_path {
                    mun_compiler::main(&options)?;
                    println!("Compiled: {}", path.to_string_lossy());
                }
            }
            Ok(_) => (),
            Err(e) => eprintln!("Watcher error: {:?}", e),
        }
    }
}
