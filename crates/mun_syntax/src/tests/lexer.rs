use std::fmt::Write;

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

fn lex_snapshot(text: &str) {
    let text = text.trim().replace("\n    ", "\n");
    let tokens = crate::tokenize(&text);
    insta::assert_snapshot!(
        insta::_macro_support::AutoName,
        dump_tokens(&tokens, &text),
        &text
    );
}

#[test]
fn numbers() {
    lex_snapshot(
        r#"
    1.34
    0x3Af
    1e-3
    100_000
    0x3a_u32
    1f32"#,
    )
}

#[test]
fn comments() {
    lex_snapshot(
        r#"
    // hello, world!
    /**/
    /* block comment */
    /* multi
       line
       comment */
    /* /* nested */ */
    /* unclosed comment"#,
    )
}

#[test]
fn whitespace() {
    lex_snapshot(
        r#"
    h e ll  o
    w

    o r     l   d"#,
    )
}

#[test]
fn ident() {
    lex_snapshot(
        r#"
    hello world_ _a2 _ __ x 即可编著课程
    "#,
    )
}

#[test]
fn symbols() {
    lex_snapshot(
        r#"
    # ( ) { } [ ] ; ,
    = ==
    !=
    < <=
    > >=
    . .. ... ..=
    + +=
    - -=
    * *=
    / /=
    ^ ^=
    % %=
    : ::
    ->
    "#,
    )
}

#[test]
fn strings() {
    lex_snapshot(
        r#"
    "Hello, world!"
    'Hello, world!'
    "\n"
    "\"\\"
    "multi
    line"
    "#,
    )
}

#[test]
fn keywords() {
    lex_snapshot(
        r#"
    and break do else false for fn if in nil
    return true while let mut struct class
    never loop pub super self package
    "#,
    )
}

#[test]
fn unclosed_string() {
    lex_snapshot(
        r#"
    "test
    "#,
    )
}

#[test]
fn binary_cmp() {
    lex_snapshot(
        r#"
    a==b
    a ==b
    a== b
    a == b
    "#,
    )
}
