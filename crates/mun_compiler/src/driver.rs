//! `Driver` is a stateful compiler frontend that enables incremental compilation by retaining state
//! from previous compilation.

use crate::{
    compute_source_relative_path, db::CompilerDatabase, ensure_package_output_dir, is_source_file,
    PathOrInline, RelativePath,
};
use mun_codegen::{AssemblyIr, CodeGenDatabase, ModuleGroup, TargetAssembly};
use mun_hir::{
    AstDatabase, DiagnosticSink, FileId, Module, PackageSet, SourceDatabase, SourceRoot,
    SourceRootId, Upcast,
};
use mun_paths::RelativePathBuf;

mod config;
mod display_color;

pub use self::config::Config;
pub use self::display_color::DisplayColor;

use crate::diagnostics_snippets::{emit_hir_diagnostic, emit_syntax_error};
use mun_project::{Package, LOCKFILE_NAME};
use std::{
    collections::HashMap, convert::TryInto, io::Cursor, path::Path, path::PathBuf, sync::Arc,
    time::Duration,
};
use walkdir::WalkDir;

pub const WORKSPACE: SourceRootId = SourceRootId(0);

pub struct Driver {
    db: CompilerDatabase,
    out_dir: PathBuf,

    source_root: SourceRoot,
    path_to_file_id: HashMap<RelativePathBuf, FileId>,
    file_id_to_path: HashMap<FileId, RelativePathBuf>,
    next_file_id: usize,

    module_to_temp_assembly_path: HashMap<Module, PathBuf>,

    emit_ir: bool,
}

impl Driver {
    /// Constructs a driver with a specific configuration.
    pub fn with_config(config: Config, out_dir: PathBuf) -> Self {
        Self {
            db: CompilerDatabase::new(&config),
            out_dir,
            source_root: Default::default(),
            path_to_file_id: Default::default(),
            file_id_to_path: Default::default(),
            next_file_id: 0,
            module_to_temp_assembly_path: Default::default(),
            emit_ir: config.emit_ir,
        }
    }

    /// Constructs a driver with a configuration and a single file.
    pub fn with_file(config: Config, path: PathOrInline) -> anyhow::Result<(Driver, FileId)> {
        let out_dir = config.out_dir.clone().unwrap_or_else(|| {
            std::env::current_dir().expect("could not determine current working directory")
        });

        let mut driver = Driver::with_config(config, out_dir);

        // Get the path and contents of the path
        let (rel_path, text) = match path {
            PathOrInline::Path(p) => (
                RelativePathBuf::from_path("mod.mun").unwrap(),
                std::fs::read_to_string(p)?,
            ),
            PathOrInline::Inline { rel_path, contents } => (rel_path, contents),
        };

        // Store the file information in the database together with the source root
        let file_id = FileId(driver.next_file_id as u32);
        driver.next_file_id += 1;
        driver.db.set_file_text(file_id, Arc::from(text));
        driver.db.set_file_source_root(file_id, WORKSPACE);
        driver.source_root.insert_file(file_id, rel_path.clone());
        driver
            .db
            .set_source_root(WORKSPACE, Arc::new(driver.source_root.clone()));

        let mut package_set = PackageSet::default();
        package_set.add_package(WORKSPACE);
        driver.db.set_packages(Arc::new(package_set));

        driver.path_to_file_id.insert(rel_path, file_id);

        Ok((driver, file_id))
    }

    /// Constructs a driver with a package manifest directory
    pub fn with_package_path<P: AsRef<Path>>(
        package_path: P,
        config: Config,
    ) -> Result<(Package, Driver), anyhow::Error> {
        // Load the manifest file as a package
        let package = Package::from_file(package_path)?;

        // Determine output directory
        let output_dir = ensure_package_output_dir(&package, &config)
            .map_err(|e| anyhow::anyhow!("could not create package output directory: {}", e))?;

        // Construct the driver
        let mut driver = Driver::with_config(config, output_dir);

        // Iterate over all files in the source directory of the package and store their information in
        // the database
        let source_directory = package.source_directory();
        if !source_directory.is_dir() {
            anyhow::bail!("the source directory does not exist")
        }

        for source_file_path in iter_source_files(&source_directory) {
            let relative_path = compute_source_relative_path(&source_directory, &source_file_path)?;

            // Load the contents of the file
            let file_contents = std::fs::read_to_string(&source_file_path).map_err(|e| {
                anyhow::anyhow!(
                    "could not read contents of '{}': {}",
                    source_file_path.display(),
                    e
                )
            })?;

            let file_id = driver.alloc_file_id(&relative_path)?;
            driver.db.set_file_text(file_id, Arc::from(file_contents));
            driver.db.set_file_source_root(file_id, WORKSPACE);
            driver
                .source_root
                .insert_file(file_id, relative_path.clone());
        }

        // Store the source root in the database
        driver
            .db
            .set_source_root(WORKSPACE, Arc::new(driver.source_root.clone()));

        let mut package_set = PackageSet::default();
        package_set.add_package(WORKSPACE);
        driver.db.set_packages(Arc::new(package_set));

        Ok((package, driver))
    }
}

impl Driver {
    /// Returns a file id for the file with the given `relative_path`. This function reuses FileId's
    /// for paths to keep the cache as valid as possible.
    ///
    /// The allocation of an id might fail if more file IDs exist than can be allocated.
    pub fn alloc_file_id<P: AsRef<RelativePath>>(
        &mut self,
        relative_path: P,
    ) -> Result<FileId, anyhow::Error> {
        // Re-use existing id to get better caching performance
        if let Some(id) = self.path_to_file_id.get(relative_path.as_ref()) {
            return Ok(*id);
        }

        // Allocate a new id
        // TODO: See if we can figure out if the compiler cleared the cache of a certain file, at
        //  which point we can sort of reset the `next_file_id`
        let id = FileId(
            self.next_file_id
                .try_into()
                .map_err(|_e| anyhow::anyhow!("too many active source files"))?,
        );
        self.next_file_id += 1;

        // Update bookkeeping
        self.path_to_file_id
            .insert(relative_path.as_ref().to_relative_path_buf(), id);
        self.file_id_to_path
            .insert(id, relative_path.as_ref().to_relative_path_buf());

        Ok(id)
    }
}

impl Driver {
    /// Sets the contents of a specific file.
    pub fn set_file_text(
        &mut self,
        path: impl AsRef<RelativePath>,
        text: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let file_id = self
            .path_to_file_id
            .get(path.as_ref())
            .ok_or_else(|| anyhow::anyhow!("the path '{}' is unknown", path.as_ref()))?;
        self.db
            .set_file_text(*file_id, Arc::from(text.as_ref().to_owned()));
        Ok(())
    }
}

impl Driver {
    /// Emits all diagnostic messages currently in the database; returns true if errors were
    /// emitted.
    pub fn emit_diagnostics(
        &self,
        writer: &mut dyn std::io::Write,
        display_color: DisplayColor,
    ) -> Result<bool, anyhow::Error> {
        let emit_colors = display_color.should_enable();
        let mut has_error = false;

        for package in mun_hir::Package::all(self.db.upcast()) {
            for module in package.modules(self.db.upcast()) {
                if let Some(file_id) = module.file_id(self.db.upcast()) {
                    let parse = self.db.parse(file_id);
                    let source_code = self.db.file_text(file_id);
                    let relative_file_path = self.db.file_relative_path(file_id);
                    let line_index = self.db.line_index(file_id);

                    // Emit all syntax diagnostics
                    for syntax_error in parse.errors().iter() {
                        emit_syntax_error(
                            syntax_error,
                            relative_file_path.as_str(),
                            &source_code,
                            &line_index,
                            emit_colors,
                            writer,
                        )?;
                        has_error = true;
                    }

                    // Emit all HIR diagnostics
                    let mut error = None;
                    module.diagnostics(
                        self.db.upcast(),
                        &mut DiagnosticSink::new(|d| {
                            has_error = true;
                            if let Err(e) =
                                emit_hir_diagnostic(d, &self.db, file_id, emit_colors, writer)
                            {
                                error = Some(e)
                            };
                        }),
                    );

                    // If an error occurred when emitting HIR diagnostics, return early with the error.
                    if let Some(e) = error {
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(has_error)
    }

    /// Returns all diagnostics as a human readable string
    pub fn emit_diagnostics_to_string(
        &self,
        display_color: DisplayColor,
    ) -> anyhow::Result<Option<String>> {
        let mut compiler_errors: Vec<u8> = Vec::new();
        if !self.emit_diagnostics(&mut Cursor::new(&mut compiler_errors), display_color)? {
            Ok(None)
        } else {
            Ok(Some(String::from_utf8(compiler_errors).map_err(|e| {
                anyhow::anyhow!(
                    "could not convert compiler diagnostics to valid UTF8: {}",
                    e
                )
            })?))
        }
    }
}

impl Driver {
    /// Get the path where the driver will write the assembly for the specified file.
    pub fn assembly_output_path_from_file(&self, file_id: FileId) -> PathBuf {
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_file(file_id)
            .expect("could not find file in module parition");
        self.path_for_module_group(&module_partition[module_group_id])
            .with_extension(TargetAssembly::EXTENSION)
    }

    /// Get the path where the driver will write the IR for the specified file.
    pub fn ir_output_path_from_file(&self, file_id: FileId) -> PathBuf {
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_file(file_id)
            .expect("could not find file in module parition");
        self.path_for_module_group(&module_partition[module_group_id])
            .with_extension(AssemblyIr::EXTENSION)
    }

    /// Get the path where the driver will write the assembly for the specified module.
    pub fn assembly_output_path(&self, module: Module) -> PathBuf {
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_module(module)
            .expect("could not find file in module parition");
        self.path_for_module_group(&module_partition[module_group_id])
            .with_extension(TargetAssembly::EXTENSION)
    }

    /// Get the path where the driver will write the IR for the specified module.
    pub fn ir_output_path(&self, module: Module) -> PathBuf {
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_module(module)
            .expect("could not find file in module parition");
        self.path_for_module_group(&module_partition[module_group_id])
            .with_extension(AssemblyIr::EXTENSION)
    }

    /// Returns the output path for the specified module group without an extension
    fn path_for_module_group(&self, module_group: &ModuleGroup) -> PathBuf {
        module_group.relative_file_path().to_path(&self.out_dir)
    }

    /// Writes all assemblies. If `force` is false, the binary will not be written if there are no
    /// changes since last time it was written.
    pub fn write_all_assemblies(&mut self, force: bool) -> Result<(), anyhow::Error> {
        let _lock = self.acquire_filesystem_output_lock();

        // Create a copy of all current files
        for package in mun_hir::Package::all(self.db.upcast()) {
            for module in package.modules(self.db.upcast()) {
                if self.emit_ir {
                    self.write_assembly_ir(module)?;
                } else {
                    self.write_target_assembly(module, force)?;
                }
            }
        }

        Ok(())
    }

    /// Acquires a filesystem lock on the output directory. This ensures that multiple instances
    /// cannot write to the same output directory and that the runtime does not start reading before
    /// we finished writing.
    fn acquire_filesystem_output_lock(&self) -> lockfile::Lockfile {
        loop {
            match lockfile::Lockfile::create(self.out_dir.join(LOCKFILE_NAME)) {
                Ok(lockfile) => break lockfile,
                Err(_) => {
                    // TODO(#313): Implement/abstract a better way to emit warnings/errors from the
                    //  driver. Directly writing to stdout accesses global state. The driver is not
                    //  aware of how to output logging information.
                    // if self.display_color.should_enable() {
                    //     eprintln!(
                    //         "{} on acquiring lock on output directory",
                    //         yansi_term::Color::Cyan.paint("Blocked")
                    //     )
                    // } else {
                    //     eprintln!("Blocked on acquiring lock on output directory")
                    // }
                    std::thread::sleep(Duration::from_secs(1))
                }
            };
        }
    }

    /// Generates an assembly for the target machine and specified module and stores it in the
    /// output location. If `force` is false, the binary will not be written if there are no
    /// changes since last time it was written. Returns `true` if the assembly was written, `false`
    /// if it was up to date.
    fn write_target_assembly(
        &mut self,
        module: Module,
        force: bool,
    ) -> Result<bool, anyhow::Error> {
        log::trace!("writing target assembly for {:?}", module);

        // Find the module group to which the module belongs
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_module(module)
            .expect("could not find the module in the module partition");
        let module_group = &module_partition[module_group_id];

        // Get the compiled assembly
        let assembly = self.db.target_assembly(module_group_id);

        // Determine the filename of the group
        let assembly_path = self
            .path_for_module_group(module_group)
            .with_extension(TargetAssembly::EXTENSION);

        // Did the assembly change since last time?
        if !force
            && assembly_path.is_file()
            && self
                .module_to_temp_assembly_path
                .get(&module)
                .map(AsRef::as_ref)
                == Some(assembly.path())
        {
            return Ok(false);
        }

        // It did change or we are forced, so write it to disk
        assembly.copy_to(&assembly_path)?;

        // Store the information so we maybe don't have to write it next time
        self.module_to_temp_assembly_path
            .insert(module, assembly.path().to_path_buf());

        Ok(true)
    }

    /// Generates IR for the specified module and stores it in the output location.
    fn write_assembly_ir(&mut self, module: mun_hir::Module) -> Result<(), anyhow::Error> {
        log::trace!("writing assembly IR for {:?}", module);

        // Find the module group to which the module belongs
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_module(module)
            .expect("could not find the module in the module partition");
        let module_group = &module_partition[module_group_id];

        // Get the compiled assembly
        let assembly_ir = self.db.assembly_ir(module_group_id);

        // Determine the filename of the group
        let assembly_path = self
            .path_for_module_group(module_group)
            .with_extension(AssemblyIr::EXTENSION);

        // Write to disk
        assembly_ir.copy_to(assembly_path)?;

        Ok(())
    }
}

impl Driver {
    /// Returns the `FileId` of the file with the given relative path
    pub fn get_file_id_for_path<P: AsRef<RelativePath>>(&self, path: P) -> Option<FileId> {
        self.path_to_file_id.get(path.as_ref()).copied()
    }

    /// Tells the driver that the file at the specified `path` has changed its contents. Returns the
    /// `FileId` of the modified file.
    pub fn update_file<P: AsRef<RelativePath>>(&mut self, path: P, contents: String) -> FileId {
        let file_id = *self
            .path_to_file_id
            .get(path.as_ref())
            .expect("writing to a file that is not part of the source root should never happen");
        self.db.set_file_text(file_id, Arc::from(contents));
        file_id
    }

    /// Adds a new file to the driver. Returns the `FileId` of the new file.
    pub fn add_file<P: AsRef<RelativePath>>(&mut self, path: P, contents: String) -> FileId {
        let file_id = self.alloc_file_id(path.as_ref()).unwrap();

        // Insert the new file
        self.db.set_file_text(file_id, Arc::from(contents));
        self.db.set_file_source_root(file_id, WORKSPACE);

        // Update the source root
        self.source_root
            .insert_file(file_id, path.as_ref().to_relative_path_buf());
        self.db
            .set_source_root(WORKSPACE, Arc::new(self.source_root.clone()));

        file_id
    }

    /// Removes the specified file from the driver.
    pub fn remove_file<P: AsRef<RelativePath>>(&mut self, path: P) -> FileId {
        let file_id = *self
            .path_to_file_id
            .get(path.as_ref())
            .expect("removing to a file that is not part of the source root should never happen");

        // Update the source root
        self.source_root.remove_file(file_id);
        self.db
            .set_source_root(WORKSPACE, Arc::new(self.source_root.clone()));

        file_id
    }

    /// Renames the specified file to the specified path
    pub fn rename<P1: AsRef<RelativePath>, P2: AsRef<RelativePath>>(
        &mut self,
        from: P1,
        to: P2,
    ) -> FileId {
        let file_id = *self
            .path_to_file_id
            .get(from.as_ref())
            .expect("renaming from a file that is not part of the source root should never happen");
        if let Some(previous) = self.path_to_file_id.get(to.as_ref()) {
            // If there was some other file with this path in the database, forget about it.
            self.file_id_to_path.remove(previous);
        }

        self.file_id_to_path
            .insert(file_id, to.as_ref().to_relative_path_buf());
        self.path_to_file_id.remove(from.as_ref()); // FileId now belongs to to

        self.source_root.remove_file(file_id);
        self.source_root
            .insert_file(file_id, to.as_ref().to_relative_path_buf());
        self.db
            .set_source_root(WORKSPACE, Arc::new(self.source_root.clone()));

        file_id
    }
}

pub fn iter_source_files(source_dir: &Path) -> impl Iterator<Item = PathBuf> {
    WalkDir::new(source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| is_source_file(e.path()))
        .map(|e| e.path().to_path_buf())
}
