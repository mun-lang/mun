use std::{
    io::stderr,
    path::Path,
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use mun_compiler::{compute_source_relative_path, is_source_file, Config, DisplayColor, Driver};
use notify::{
    event::{ModifyKind, RenameMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};

/// Compiles and watches the package at the specified path. Recompiles changes
/// that occur.
pub fn compile_and_watch_manifest(
    manifest_path: &Path,
    config: Config,
    display_color: DisplayColor,
) -> Result<bool, anyhow::Error> {
    // Create the compiler driver
    let (package, mut driver) = Driver::with_package_path(manifest_path, config)?;

    // Start watching the source directory
    let (watcher_tx, watcher_rx) = channel();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |event| {
        if let Err(error) = watcher_tx.send(event) {
            log::debug!("failed to forward filesystem watcher event to compiler: {error}");
        }
    })?;
    let source_directory = package.source_directory();

    watcher.watch(&source_directory, RecursiveMode::Recursive)?;
    println!("Watching: {}", source_directory.display());

    // Emit all current errors, and write the assemblies if no errors occured
    if !driver.emit_diagnostics(&mut stderr(), display_color)? {
        driver.write_all_assemblies(false)?;
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
            let event = event?;
            let paths = event.paths;
            match event.kind {
                EventKind::Modify(ModifyKind::Name(RenameMode::Both | RenameMode::Any))
                    if paths.len() >= 2
                        && is_source_file(&paths[0])
                        && is_source_file(&paths[1]) =>
                {
                    let from_relative_path =
                        compute_source_relative_path(&source_directory, &paths[0])?;
                    let to_relative_path =
                        compute_source_relative_path(&source_directory, &paths[1])?;
                    if driver.get_file_id_for_path(&from_relative_path).is_some() {
                        log::info!("Renaming {from_relative_path} to {to_relative_path}");
                        driver.rename(from_relative_path, to_relative_path);
                    } else if paths[1].is_file() {
                        let file_contents = std::fs::read_to_string(&paths[1])?;
                        log::info!("Creating {to_relative_path}");
                        driver.add_file(to_relative_path, file_contents);
                    }
                    if !driver.emit_diagnostics(&mut stderr(), display_color)? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    for path in paths.iter().filter(|path| is_source_file(path)) {
                        let relative_path = compute_source_relative_path(&source_directory, path)?;
                        let file_contents = std::fs::read_to_string(path)?;
                        log::info!("Creating {relative_path}");
                        driver.add_file(relative_path, file_contents);
                    }
                    if !driver.emit_diagnostics(&mut stderr(), display_color)? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    for path in paths.iter().filter(|path| is_source_file(path)) {
                        let relative_path = compute_source_relative_path(&source_directory, path)?;
                        if driver.get_file_id_for_path(&relative_path).is_some() {
                            log::info!("Removing {relative_path}");
                            driver.remove_file(relative_path);
                        }
                    }
                    driver.emit_diagnostics(&mut stderr(), display_color)?;
                }
                EventKind::Modify(ModifyKind::Name(_)) => {
                    for path in paths.iter().filter(|path| is_source_file(path)) {
                        let relative_path = compute_source_relative_path(&source_directory, path)?;
                        if path.is_file() {
                            let file_contents = std::fs::read_to_string(path)?;
                            log::info!("Creating {relative_path}");
                            driver.add_file(relative_path, file_contents);
                        } else if driver.get_file_id_for_path(&relative_path).is_some() {
                            log::info!("Removing {relative_path}");
                            driver.remove_file(relative_path);
                        }
                    }
                    if !driver.emit_diagnostics(&mut stderr(), display_color)? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                EventKind::Modify(_) => {
                    for path in paths
                        .iter()
                        .filter(|path| is_source_file(path) && path.is_file())
                    {
                        let relative_path = compute_source_relative_path(&source_directory, path)?;
                        let file_contents = std::fs::read_to_string(path)?;
                        if driver.get_file_id_for_path(&relative_path).is_some() {
                            log::info!("Modifying {relative_path}");
                            driver.update_file(relative_path, file_contents);
                        } else {
                            log::info!("Creating {relative_path}");
                            driver.add_file(relative_path, file_contents);
                        }
                    }
                    if !driver.emit_diagnostics(&mut stderr(), display_color)? {
                        driver.write_all_assemblies(false)?;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(true)
}
