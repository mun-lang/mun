#[cfg(test)]
mod test {
    use crate::completion::test_utils::completion_relevance_string;

    #[test]
    fn test_locals_first() {
        insta::assert_snapshot!(completion_relevance_string(
            r#"
            fn a() {};

        fn foo(bar: u32) {
            let zinq = 0;
            z$0
        }
        "#
        ), @r###"
        lc zinq i32
        lc bar  u32
        fn foo  -> ()
        fn a    -> ()
        "###);
    }
}
