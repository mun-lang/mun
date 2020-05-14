use std::sync::mpsc::channel;
use std::time::Duration;

use anyhow::Result;
use mun_compiler::{CompilerOptions, Driver, PathOrInline};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use std::io::stderr;

pub fn main(options: CompilerOptions) -> Result<()> {
    // Need to canonicalize path to do comparisons
    let input_path = match &options.input {
        PathOrInline::Path(path) => path.canonicalize()?,
        PathOrInline::Inline { .. } => panic!("cannot run compiler with inline path"),
    };

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_millis(10))?;
    watcher.watch(&input_path, RecursiveMode::NonRecursive)?;
    println!("Watching: {}", input_path.display());

    let (mut driver, file_id) = Driver::with_file(options.config, options.input)?;

    // Compile at least once
    if !driver.emit_diagnostics(&mut stderr())? {
        driver.write_assembly(file_id)?;
    }

    loop {
        use notify::DebouncedEvent::*;
        match rx.recv() {
            Ok(Write(ref path)) | Ok(Create(ref path)) if path == &input_path => {
                let contents = std::fs::read_to_string(path)?;
                driver.set_file_text(file_id, &contents);
                if !driver.emit_diagnostics(&mut stderr())? {
                    driver.write_assembly(file_id)?;
                    println!("Successfully compiled: {}", path.display())
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("Watcher error: {:?}", e),
        }
    }
}
