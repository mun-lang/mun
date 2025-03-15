use std::{collections::HashMap, path::PathBuf, sync::Arc};

use mun_codegen::CodeGenDatabase;
use mun_codegen_c::{CCodegenDatabase, HeaderAndSourceFiles};
use mun_diagnostics_output::{emit_diagnostics_to_string, DisplayColor};
use mun_hir::{Module, Upcast};
use mun_hir_input::{FileId, PackageSet, SourceDatabase as _, SourceRoot, SourceRootId};
use mun_paths::RelativePathBuf;

use super::{config::Config, db::CompilerDatabase};

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
            source_root: SourceRoot::default(),
            path_to_file_id: HashMap::default(),
            file_id_to_path: HashMap::default(),
            next_file_id: 0,
            module_to_temp_assembly_path: HashMap::default(),
            emit_ir: config.emit_ir,
        }
    }

    /// Constructs a driver with a configuration and a single file.
    pub fn with_text(text: &str) -> anyhow::Result<(Self, FileId)> {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let config = Config {
            out_dir: Some(out_dir.clone()),
            ..Config::default()
        };

        let mut driver = Driver::with_config(config, out_dir);
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

        Ok((driver, file_id))
    }

    pub fn generate_all(&mut self) -> anyhow::Result<Vec<Arc<HeaderAndSourceFiles>>> {
        let packages = mun_hir::Package::all(self.db.upcast());
        let mut units = Vec::with_capacity(packages.len());

        for package in packages {
            for module in package.modules(self.db.upcast()) {
                units.push(self.generate_c(module));
            }
        }

        Ok(units)
    }

    pub fn generate_c(&mut self, module: Module) -> Arc<HeaderAndSourceFiles> {
        let module_partition = self.db.module_partition();
        let module_group_id = module_partition
            .group_for_module(module)
            .expect("Could not find the module in the module partition");

        self.db.c_unit(module_group_id)
    }
}
