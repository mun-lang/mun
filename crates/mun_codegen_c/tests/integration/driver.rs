use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use mun_codegen::CodeGenDatabase;
use mun_codegen_c::{CCodegenDatabase, HeaderAndSourceFiles};
use mun_diagnostics_output::{emit_diagnostics_to_string, DisplayColor};
use mun_hir::{Module, Upcast};
use mun_hir_input::{FileId, Fixture, PackageSet, SourceDatabase as _, SourceRoot, SourceRootId};
use mun_paths::{RelativePath, RelativePathBuf};

use super::{config::Config, db::CompilerDatabase};

pub const WORKSPACE: SourceRootId = SourceRootId(0);
pub const HEADER_EXTENSION: &str = "h";
pub const SOURCE_EXTENSION: &str = "c";

pub struct TranspiledFile {
    pub module_path: RelativePathBuf,
    pub transpiled: Arc<HeaderAndSourceFiles>,
}

pub struct Driver {
    db: CompilerDatabase,
    out_dir: PathBuf,

    source_root: SourceRoot,
    path_to_file_id: HashMap<RelativePathBuf, FileId>,
    file_id_to_path: HashMap<FileId, RelativePathBuf>,
    next_file_id: usize,
}

impl Driver {
    /// Constructs a driver with a specific configuration.
    pub fn with_config(config: Config, out_dir: PathBuf) -> Self {
        Self {
            db: CompilerDatabase::new(&config),
            out_dir,
            source_root: SourceRoot::default(),
            path_to_file_id: HashMap::default(),
            file_id_to_path: HashMap::default(),
            next_file_id: 0,
        }
    }

    pub fn with_fixture(text: &str) -> Self {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let out_dir = temp_dir.path().to_path_buf();

        let mut driver = Driver::with_config(Config::default(), out_dir);

        for Fixture {
            relative_path,
            text,
        } in Fixture::parse(text)
        {
            if relative_path != "mun.toml" {
                let relative_path = relative_path.strip_prefix("src/").unwrap_or_else(|_| {
                    panic!("Could not determine relative path for '{relative_path}'")
                });

                let file_id = driver
                    .alloc_file_id(relative_path)
                    .expect("Failed to allocate file id");

                driver.db.set_file_text(file_id, Arc::from(text));
                driver.db.set_file_source_root(file_id, WORKSPACE);
                driver.source_root.insert_file(file_id, relative_path);
            }
        }

        driver
            .db
            .set_source_root(WORKSPACE, Arc::new(driver.source_root.clone()));

        let mut package_set = PackageSet::default();
        package_set.add_package(WORKSPACE);
        driver.db.set_packages(Arc::new(package_set));

        if let Some(compiler_errors) = emit_diagnostics_to_string(&driver.db, DisplayColor::Disable)
            .expect("could not create diagnostics")
        {
            panic!("compiler errors:\n{compiler_errors}")
        }

        driver
    }

    /// Constructs a driver with a configuration and a single file.
    pub fn with_text(text: &str) -> (Self, FileId) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let out_dir = temp_dir.path().to_path_buf();

        let mut driver = Driver::with_config(Config::default(), out_dir);
        let rel_path = RelativePathBuf::from("mod.mun");

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

        if let Some(compiler_errors) = emit_diagnostics_to_string(&driver.db, DisplayColor::Disable)
            .expect("could not create diagnostics")
        {
            panic!("compiler errors:\n{compiler_errors}")
        }

        (driver, file_id)
    }

    pub fn transpile_all_packages(
        &mut self,
    ) -> anyhow::Result<HashMap<RelativePathBuf, Arc<HeaderAndSourceFiles>>> {
        let packages = mun_hir::Package::all(self.db.upcast());
        let mut units = HashMap::with_capacity(packages.len());

        for package in packages {
            for module in package.modules(self.db.upcast()) {
                let TranspiledFile {
                    module_path,
                    transpiled,
                } = self.transpile_module(module);

                units.insert(module_path, transpiled);
            }
        }

        Ok(units)
    }

    pub fn transpile_module(&mut self, module: Module) -> TranspiledFile {
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_module(module)
            .expect("Could not find the module in the module partition");

        let transpiled = self.db.transpile_to_c(module_group_id);

        let module_group = &module_partition[module_group_id];
        let module_path = module_group.relative_file_path();

        TranspiledFile {
            module_path,
            transpiled,
        }
    }

    pub fn write_all_packages(&mut self) -> anyhow::Result<()> {
        let packages = mun_hir::Package::all(self.db.upcast());
        let mut units = Vec::with_capacity(packages.len());

        for package in packages {
            for module in package.modules(self.db.upcast()) {
                units.push(self.write_module(module));
            }
        }

        Ok(())
    }

    pub fn write_module(&mut self, module: Module) -> io::Result<()> {
        let TranspiledFile {
            module_path,
            transpiled,
        } = self.transpile_module(module);

        let output_path = module_path.to_path(&self.out_dir);

        let header_file = output_path.with_extension(HEADER_EXTENSION);
        let source_file = output_path.with_extension(SOURCE_EXTENSION);

        std::fs::write(header_file, &transpiled.header)?;
        std::fs::write(source_file, &transpiled.source)?;

        Ok(())
    }

    /// Returns a file id for the file with the given `relative_path`. This
    /// function reuses `FileId`'s for paths to keep the cache as valid as
    /// possible.
    ///
    /// The allocation of an id might fail if more file IDs exist than can be
    /// allocated.
    fn alloc_file_id<P: AsRef<RelativePath>>(
        &mut self,
        relative_path: P,
    ) -> Result<FileId, anyhow::Error> {
        // Re-use existing id to get better caching performance
        if let Some(id) = self.path_to_file_id.get(relative_path.as_ref()) {
            return Ok(*id);
        }

        // Allocate a new id
        // TODO: See if we can figure out if the compiler cleared the cache of a certain
        // file, at  which point we can sort of reset the `next_file_id`
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
