#[macro_use]
extern crate lazy_static;

use mun_symbols::prelude::*;

lazy_static! {
    static ref METHOD_FACTORY: MethodArg2RetFactory<f32, f32, f32> = MethodArg2RetFactory::new();
    static ref ADD_TYPES: [&'static TypeInfo; 2] = [f32::type_info(), f32::type_info()];
    static ref METHODS: [MethodInfo; 5] = [
        MethodInfo::new("load", Privacy::Public, &[], None, &EmptyMethodFactory),
        MethodInfo::new("unload", Privacy::Public, &[], None, &EmptyMethodFactory),
        MethodInfo::new("init", Privacy::Public, &[], None, &EmptyMethodFactory),
        MethodInfo::new("deinit", Privacy::Public, &[], None, &EmptyMethodFactory),
        MethodInfo::new(
            "add",
            Privacy::Public,
            &ADD_TYPES[..],
            Some(f32::type_info()),
            &*METHOD_FACTORY,
        )
    ];
    static ref SYMBOLS: ModuleInfo = {
        let methods: Vec<&'static MethodInfo> = METHODS.iter().collect();
        ModuleInfo::new("mun_test", &[], &methods[..], &[])
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
