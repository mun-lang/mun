use mun_hir::diagnostics::DiagnosticSink;
use mun_hir::{FileId, HirDatabase, Module};

use std::cell::RefCell;

use annotate_snippets::snippet::Snippet;

use crate::diagnostics_snippets;

/// Constructs diagnostic messages for the given file.
pub fn diagnostics(db: &impl HirDatabase, file_id: FileId) -> Vec<Snippet> {
    let parse = db.parse(file_id);

    let mut result = Vec::new();
    // Replace every `\t` symbol by one whitespace in source code because in console it is
    // displaying like 1-4 spaces(depending on it position) and by this it breaks highlighting.
    // In future here, instead of `replace("\t", " ")`, can be implemented algorithm that
    // correctly replace each `\t` into 1-4 space.
    let source_code = db.file_text(file_id).to_string().replace("\t", " ");

    let relative_file_path = db.file_relative_path(file_id).display().to_string();

    let line_index = db.line_index(file_id);

    result.extend(parse.errors().iter().map(|err| {
        diagnostics_snippets::syntax_error(
            err,
            db,
            &parse,
            &relative_file_path,
            &source_code,
            &line_index,
        )
    }));

    let result = RefCell::new(result);
    let mut sink = DiagnosticSink::new(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::generic_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::UnresolvedValue, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::unresolved_value_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::UnresolvedType, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::unresolved_type_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::ExpectedFunction, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::expected_function_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::MismatchedType, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::mismatched_type_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::DuplicateDefinition, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::duplicate_definition_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::PossiblyUninitializedVariable, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::possibly_uninitialized_variable_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    })
    .on::<mun_hir::diagnostics::AccessUnknownField, _>(|d| {
        result
            .borrow_mut()
            .push(diagnostics_snippets::access_unknown_field_error(
                d,
                db,
                &parse,
                &relative_file_path,
                &source_code,
                &line_index,
            ));
    });

    Module::from(file_id).diagnostics(db, &mut sink);

    drop(sink);

    result.into_inner()
}

#[cfg(test)]
mod tests {
    use crate::{Config, DisplayColor, Driver, PathOrInline, RelativePathBuf};

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

        let _ = driver.emit_diagnostics(&mut compilation_errors).unwrap();

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
