use std::sync::mpsc::channel;
use std::time::Duration;

use mun_compiler::{compute_source_relative_path, is_source_file, Config, Driver};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use std::io::stderr;
use std::path::Path;
use std::sync::Arc;

/// Compiles and watches the package at the specified path. Recompiles changes that occur.
pub fn compile_and_watch_manifest(
    manifest_path: &Path,
    config: Config,
) -> Result<bool, anyhow::Error> {
    // Create the compiler driver
    let (package, mut driver) = Driver::with_package_path(manifest_path, config)?;

    // Start watching the source directory
    let (watcher_tx, watcher_rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(watcher_tx, Duration::from_millis(10))?;
    let source_directory = package.source_directory();

    watcher.watch(&source_directory, RecursiveMode::Recursive)?;
    println!("Watching: {}", source_directory.display());

    // Emit all current errors, and write the assemblies if no errors occured
    if !driver.emit_diagnostics(&mut stderr())? {
        driver.write_all_assemblies(false)?
    }

    // Insert Ctrl+C handler so we can gracefully quit
    let should_quit = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let r = should_quit.clone();
    ctrlc::set_handler(move || {
        r.store(true, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("error setting ctrl-c handler");

    // Start watching filesystem events.
    while !should_quit.load(std::sync::atomic::Ordering::SeqCst) {
        if let Ok(event) = watcher_rx.recv_timeout(Duration::from_millis(1)) {
            use notify::DebouncedEvent::*;
            match event {
                Write(ref path) if is_source_file(path) => {
                    let relative_path = compute_source_relative_path(&source_directory, path)?;
                    let file_contents = std::fs::read_to_string(path)?;
                    log::info!("Modifying {}", relative_path);
                    driver.update_file(relative_path, file_contents);
                    if !driver.emit_diagnostics(&mut stderr())? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                Create(ref path) if is_source_file(path) => {
                    let relative_path = compute_source_relative_path(&source_directory, path)?;
                    let file_contents = std::fs::read_to_string(path)?;
                    log::info!("Creating {}", relative_path);
                    driver.add_file(relative_path, file_contents);
                    if !driver.emit_diagnostics(&mut stderr())? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                Remove(ref path) if is_source_file(path) => {
                    // Simply remove the source file from the source root
                    let relative_path = compute_source_relative_path(&source_directory, path)?;
                    log::info!("Removing {}", relative_path);
                    // TODO: Remove assembly files if there are no files referencing it.
                    // let assembly_path = driver.assembly_output_path(driver.get_file_id_for_path(&relative_path).expect("cannot remove a file that was not part of the compilation in the first place"));
                    // if assembly_path.is_file() {
                    //     std::fs::remove_file(assembly_path)?;
                    // }
                    driver.remove_file(relative_path);
                    driver.emit_diagnostics(&mut stderr())?;
                }
                Rename(ref from, ref to) => {
                    // Renaming is done by changing the relative path of the original source file but
                    // not modifying any text. This ensures that most of the cache for the renamed file
                    // stays alive. This is effectively a rename of the file_id in the database.
                    let from_relative_path = compute_source_relative_path(&source_directory, from)?;
                    let to_relative_path = compute_source_relative_path(&source_directory, to)?;

                    log::info!("Renaming {} to {}", from_relative_path, to_relative_path,);
                    driver.rename(from_relative_path, to_relative_path);
                    if !driver.emit_diagnostics(&mut stderr())? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(true)
}
