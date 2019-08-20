#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;

use mun_symbols::prelude::*;

lazy_static! {
    static ref SYMBOLS: ModuleInfo = {
        let fields = HashMap::new();

        let mut methods = HashMap::new();

        let load = MethodInfo::new("load", Privacy::Public, Vec::new(), None);
        methods.insert(load.name().to_string(), load);

        let unload = MethodInfo::new("unload", Privacy::Public, Vec::new(), None);
        methods.insert(unload.name().to_string(), unload);

        let init = MethodInfo::new("init", Privacy::Public, Vec::new(), None);
        methods.insert(init.name().to_string(), init);

        let deinit = MethodInfo::new("deinit", Privacy::Public, Vec::new(), None);
        methods.insert(deinit.name().to_string(), deinit);

        let add = MethodInfo::new(
            "add",
            Privacy::Public,
            vec![f32::type_info(), f32::type_info()],
            Some(f32::type_info()),
        );
        methods.insert(add.name().to_string(), add);

        let modules = HashMap::new();
        let structs = HashMap::new();

        ModuleInfo::new("mun_test", fields, methods, modules, structs)
    };
}

#[no_mangle]
pub fn symbols() -> &'static ModuleInfo {
    &SYMBOLS
}

#[no_mangle]
pub fn load() {}

#[no_mangle]
pub fn unload() {}

#[no_mangle]
pub fn init() {}

#[no_mangle]
pub fn deinit() {}

#[no_mangle]
pub fn add(a: f32, b: f32) -> f32 {
    a + b
}
