use crate::SourceFile;
use std::{fmt::Write, path::PathBuf};
use test_utils::{dir_tests, project_dir};

#[test]
fn lexer_tests() {
    dir_tests(&test_data_dir(), &["lexer"], |text, _| {
        let tokens = crate::tokenize(text);
        dump_tokens(&tokens, text)
    });
}

#[test]
fn parser_tests() {
    dir_tests(&test_data_dir(), &["parser/ok"], |text, path| {
        let file = SourceFile::parse(text);
        let errors = file.errors();
        assert_eq!(
            &*errors,
            &[] as &[crate::SyntaxError],
            "There should be no errors in the file {:?}",
            path.display()
        );
        file.debug_dump()
    })
}

fn test_data_dir() -> PathBuf {
    project_dir().join("crates/mun_syntax/tests/data")
}

fn dump_tokens(tokens: &[crate::Token], text: &str) -> String {
    let mut acc = String::new();
    let mut offset = 0;
    for token in tokens {
        let len: u32 = token.len.into();
        let len = len as usize;
        let token_text = &text[offset..offset + len];
        offset += len;
        write!(acc, "{:?} {} {:?}\n", token.kind, token.len, token_text).unwrap()
    }
    acc
}
