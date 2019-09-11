#[macro_use]
extern crate lazy_static;

use mun_symbols::prelude::*;

lazy_static! {
    static ref ADD_METHOD_FACTORY: TwoArgsMethodFactory<f32, f32, f32> =
        TwoArgsMethodFactory::new();
    static ref EMPTY_METHOD_FACTORY: NoArgsMethodFactory<()> = NoArgsMethodFactory::new();
    static ref ADD_TYPES: [&'static TypeInfo; 2] = [f32::type_info(), f32::type_info()];
    static ref METHODS: [MethodInfo; 5] = [
        MethodInfo::new("load", Privacy::Public, &[], None, &*EMPTY_METHOD_FACTORY),
        MethodInfo::new("unload", Privacy::Public, &[], None, &*EMPTY_METHOD_FACTORY),
        MethodInfo::new("init", Privacy::Public, &[], None, &*EMPTY_METHOD_FACTORY),
        MethodInfo::new("deinit", Privacy::Public, &[], None, &*EMPTY_METHOD_FACTORY),
        MethodInfo::new(
            "add",
            Privacy::Public,
            &ADD_TYPES[..],
            Some(f32::type_info()),
            &*ADD_METHOD_FACTORY,
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
pub fn load() {
    println!("load");
}

#[no_mangle]
pub fn unload() {
    println!("unload");
}

#[no_mangle]
pub fn init() {
    println!("init");
}

#[no_mangle]
pub fn deinit() {
    println!("deinit");
}

#[no_mangle]
pub fn add(a: f32, b: f32) -> f32 {
    a + b
}
