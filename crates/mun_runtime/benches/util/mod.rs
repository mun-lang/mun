use compiler::{Config, DisplayColor, Driver, OptimizationLevel, PathOrInline};
use mlua::Lua;
use mun_runtime::RuntimeBuilder;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};
use wasmer_runtime::{instantiate, Instance};

fn compute_resource_path<P: AsRef<Path>>(p: P) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches/resources/")
        .join(p)
}

pub fn runtime_from_file<P: AsRef<Path>>(p: P) -> Rc<RefCell<mun_runtime::Runtime>> {
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
    RuntimeBuilder::new(out_path).spawn().unwrap()
}

pub fn lua_from_file<P: AsRef<Path>>(p: P) -> Lua {
    let lua = Lua::new();
    lua.load(&std::fs::read_to_string(compute_resource_path(p)).unwrap())
        .exec()
        .unwrap();
    lua
}

pub fn wasmer_from_file<P: AsRef<Path>>(p: P) -> Instance {
    let wasm_content = std::fs::read(compute_resource_path(p)).unwrap();
    let import_objects = wasmer_runtime::imports! {};
    instantiate(&wasm_content, &import_objects).unwrap()
}
