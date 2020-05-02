#[macro_use]
extern crate afl;
use mun_compiler::{CompilerOptions, Driver};

fn main() {
    fuzz!(|data: &[u8]| {
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
}
