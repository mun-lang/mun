#[cfg(test)]
mod tests {
    use crate::{Config, DisplayColor, Driver, PathOrInline, RelativePathBuf};
    use std::io::Cursor;

    /// Compile passed source code and return all compilation errors
    fn compilation_errors(source_code: &str) -> String {
        let config = Config {
            display_color: DisplayColor::Disable,
            ..Config::default()
        };

        let input = PathOrInline::Inline {
            rel_path: RelativePathBuf::from("main.mun"),
            contents: source_code.to_owned(),
        };

        let (driver, _) = Driver::with_file(config, input).unwrap();

        let mut compilation_errors = Vec::<u8>::new();

        let _ = driver
            .emit_diagnostics(&mut Cursor::new(&mut compilation_errors))
            .unwrap();

        String::from_utf8(compilation_errors).unwrap()
    }

    #[test]
    fn test_syntax_error() {
        insta::assert_display_snapshot!(compilation_errors("\n\nfn main(\n struct Foo\n"));
    }

    #[test]
    fn test_unresolved_value_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet b = a;\n\nlet d = c;\n}"
        ));
    }

    #[test]
    fn test_unresolved_type_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a = Foo{};\n\nlet b = Bar{};\n}"
        ));
    }

    #[test]
    fn test_expected_function_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a = Foo();\n\nlet b = Bar();\n}"
        ));
    }

    #[test]
    fn test_mismatched_type_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a: f64 = false;\n\nlet b: bool = 22;\n}"
        ));
    }

    #[test]
    fn test_duplicate_definition_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn foo(){}\n\nfn foo(){}\n\nstruct Bar;\n\nstruct Bar;\n\nfn BAZ(){}\n\nstruct BAZ;"
        ));
    }

    #[test]
    fn test_possibly_uninitialized_variable_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nfn main() {\nlet a;\nif 5>6 {\na = 5\n}\nlet b = a;\n}"
        ));
    }

    #[test]
    fn test_access_unknown_field_error() {
        insta::assert_display_snapshot!(compilation_errors(
            "\n\nstruct Foo {\ni: bool\n}\n\nfn main() {\nlet a = Foo { i: false };\nlet b = a.t;\n}"
        ));
    }
}
