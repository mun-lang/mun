#![no_main]
use libfuzzer_sys::fuzz_target;
use mun_compiler::{CompilerOptions, Driver};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let options = CompilerOptions {
            input: mun_compiler::PathOrInline::Inline {
                rel_path: "".into(),
                contents: s.to_string(),
            },
            config: Default::default(),
        };
        Driver::with_file(options.config, options.input).unwrap();
    }
});
