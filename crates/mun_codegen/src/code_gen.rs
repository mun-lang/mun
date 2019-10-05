use crate::IrDatabase;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target};
use mun_hir::FileId;
use std::io::Write;
use std::path::Path;

mod linker;
pub fn write_module_shared_object(db: &impl IrDatabase, file_id: FileId) -> bool {
    let module = db.module_ir(file_id);
    let target = db.target();

    // Clone the module so we can modify it safely
    let llvm_module = module.llvm_module.clone();

    // Generate the `get_symbols` method.
    symbols::gen_symbols(db, &module.functions, &llvm_module);

    Target::initialize_x86(&InitializationConfig::default());

    let llvm_target = Target::from_triple(&target.llvm_target).unwrap();
    let target_machine = llvm_target
        .create_target_machine(
            &target.llvm_target,
            &target.options.cpu,
            &target.options.features,
            db.optimization_lvl(),
            RelocMode::PIC,
            CodeModel::Default,
        )
        .unwrap();

    let relative_path = db.file_relative_path(file_id);
    let original_filename = Path::new(relative_path.file_name().unwrap());

    // Generate object file
    let obj_file = {
        let obj = target_machine
            .write_to_memory_buffer(&llvm_module, FileType::Object)
            .unwrap();
        let mut obj_file = tempfile::NamedTempFile::new().unwrap();
        obj_file.write(obj.as_slice()).unwrap();
        obj_file
    };

    // Construct a linker for the target
    let mut linker = linker::create_with_target(&target);
    linker.add_object(obj_file.path());

    // Determine output file
    let dll_extension = if target.options.dll_suffix.starts_with(".") {
        &target.options.dll_suffix[1..]
    } else {
        &target.options.dll_suffix
    };
    let output_file_path = original_filename.with_extension(dll_extension);

    // Link the object
    linker.build_shared_object(&output_file_path);

    let mut cmd = linker.finalize();
    cmd.spawn().unwrap().wait().unwrap();

    true
}

pub mod symbols;
