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

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(10))?;
    watcher.watch(&input_path, RecursiveMode::NonRecursive)?;
    println!("Watching: {}", input_path.display());

    // Compile at least once
    if let Err(e) = mun_compiler::main(&options) {
        println!("Compilation failed with error: {}", e);
    }

    loop {
        use notify::DebouncedEvent::*;
        match rx.recv() {
            Ok(Write(ref path)) => {
                // TODO: Check whether file contents changed (using sha hash?)
                match mun_compiler::main(&options) {
                    Ok(_) => println!("Successfully compiled: {}", path.to_string_lossy()),
                    Err(e) => println!("Compilation failed with error: {}", e),
                }
            }
            Ok(_) => (),
            Err(e) => eprintln!("Watcher error: {:?}", e),
        }
    }
}
