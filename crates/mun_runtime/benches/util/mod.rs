use mlua::Lua;
use mun_compiler::{Config, DisplayColor, Driver, OptimizationLevel, PathOrInline};
use mun_runtime::Runtime;
use std::path::{Path, PathBuf};
use wasmer::{Instance, Module, Store};

fn compute_resource_path<P: AsRef<Path>>(p: P) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches/resources/")
        .join(p)
}

pub fn runtime_from_file<P: AsRef<Path>>(p: P) -> Runtime {
    let path = PathOrInline::Path(compute_resource_path(p));
    let (mut driver, file_id) = Driver::with_file(
        Config {
            optimization_lvl: OptimizationLevel::Aggressive,
            ..Config::default()
        },
        path,
    )
    .unwrap();
    if let Some(errors) = driver
        .emit_diagnostics_to_string(DisplayColor::Disable)
        .unwrap()
    {
        panic!("compiler errors..\n{}", errors);
    }

    let out_path = driver.assembly_output_path_from_file(file_id);
    driver.write_all_assemblies(false).unwrap();
    let builder = Runtime::builder(out_path);

    // Safety: we compiled the code ourselves, so this is safe.
    unsafe { builder.finish() }.unwrap()
}

pub fn lua_from_file<P: AsRef<Path>>(p: P) -> Lua {
    let lua = Lua::new();
    lua.load(&std::fs::read_to_string(compute_resource_path(p)).unwrap())
        .exec()
        .unwrap();
    lua
}

pub fn wasmer_from_file<P: AsRef<Path>>(store: &mut Store, p: P) -> Instance {
    let wasm_content = std::fs::read(compute_resource_path(p)).unwrap();
    let import_objects = wasmer::imports! {};
    let module = Module::new(store, &wasm_content).unwrap();
    Instance::new(store, &module, &import_objects).unwrap()
}
