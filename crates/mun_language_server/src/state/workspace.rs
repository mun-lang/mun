use super::LanguageServerState;
use crate::{change::AnalysisChange, config::FilesWatcher};
use mun_paths::{AbsPathBuf, RelativePath};
use std::{
    convert::{TryFrom, TryInto},
    sync::Arc,
};

impl LanguageServerState {
    /// Called to update all workspaces from the files
    pub(crate) fn fetch_workspaces(&mut self) {
        // Load all the manifests as packages
        let packages = self
            .config
            .discovered_projects
            .clone()
            .into_iter()
            .flatten()
            .filter_map(
                |project| match mun_project::Package::from_file(project.path) {
                    Ok(package) => Some(package),
                    Err(err) => {
                        self.show_message(
                            lsp_types::MessageType::ERROR,
                            format!("mun failed to load package: {err:#}"),
                        );
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        // If these packages are the same as the ones we already had, there is little to do.
        if *self.packages == packages {
            return;
        }

        // If we use the client to watch for file changes, communicate a request to the client
        if self.config.watcher == FilesWatcher::Client {
            let registration_options = lsp_types::DidChangeWatchedFilesRegistrationOptions {
                watchers: packages
                    .iter()
                    .map(|package| format!("{}/**/*.mun", package.source_directory().display()))
                    .map(|glob_pattern| lsp_types::FileSystemWatcher {
                        glob_pattern: lsp_types::GlobPattern::String(glob_pattern),
                        kind: None,
                    })
                    .collect(),
            };

            let registration = lsp_types::Registration {
                id: "file-watcher".to_string(),
                method: "workspace/didChangeWatchedFiles".to_string(),
                register_options: Some(serde_json::to_value(registration_options).unwrap()),
            };
            self.send_request::<lsp_types::request::RegisterCapability>(
                lsp_types::RegistrationParams {
                    registrations: vec![registration],
                },
                |_, _| {},
            );
        }

        let mut change = AnalysisChange::new();

        // Construct the set of files to pass to the vfs loader
        let entries_to_load = packages
            .iter()
            .map(|package| {
                let source_dir: AbsPathBuf = package
                    .source_directory()
                    .try_into()
                    .expect("could not convert package root to absolute path");
                mun_vfs::MonitorEntry::Directories(mun_vfs::MonitorDirectories {
                    extensions: vec!["mun".to_owned()],
                    include: vec![source_dir],
                    exclude: vec![],
                })
            })
            .collect::<Vec<_>>();

        let monitor_config = mun_vfs::MonitorConfig {
            watch: match self.config.watcher {
                FilesWatcher::Client => vec![],
                FilesWatcher::Notify => (0..entries_to_load.len()).collect(),
            },
            load: entries_to_load,
        };

        self.vfs_monitor.set_config(monitor_config);

        // Create the set of packages
        let mut package_set = mun_hir::PackageSet::default();
        for (idx, _package) in packages.iter().enumerate() {
            package_set.add_package(mun_hir::SourceRootId(idx as u32));
        }
        change.set_packages(package_set);

        // Store the current set of packages and update the source roots
        self.packages = Arc::new(packages);
        change.set_roots(self.recompute_source_roots());

        // Apply all changes to the database
        self.analysis.apply_change(change);
    }

    /// Recomputes all the source roots based on the `packages`
    pub(crate) fn recompute_source_roots(&self) -> Vec<mun_hir::SourceRoot> {
        // Iterate over all sources and see to which package they belong
        let mut source_roots = vec![mun_hir::SourceRoot::default(); self.packages.len()];

        // Source directories
        let source_dirs = self
            .packages
            .iter()
            .map(|p| {
                AbsPathBuf::try_from(p.source_directory())
                    .expect("must be able to convert source dir to absolute path")
            })
            .collect::<Vec<_>>();

        // Iterate over all files and find to which source directory they belong, including their
        // relative path
        let vfs = &*self.vfs.read();
        for (file_id, path) in vfs.iter() {
            if let Some((idx, relative_path)) =
                source_dirs
                    .iter()
                    .enumerate()
                    .find_map(|(index, source_dir)| {
                        path.strip_prefix(source_dir)
                            .ok()
                            .and_then(|path| RelativePath::from_path(path).ok())
                            .map(|relative| (index, relative))
                    })
            {
                source_roots[idx].insert_file(mun_hir::FileId(file_id.0), relative_path);
            }
        }

        source_roots
    }
}
