use crate::SourceFile;

#[test]
fn method_call() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
        fn main() {
            a.foo();
            a.0.foo();
            a.0.0.foo();
            a.0 .f32();
        }
        "#
    )
    .debug_dump());
}

#[test]
fn impl_block() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
        impl Foo {}
        impl Bar {
            fn bar() {}
            struct Baz {}
        }
        pub impl FooBar {}
        "#).debug_dump(), @r###"
    SOURCE_FILE@0..135
      WHITESPACE@0..9 "\n        "
      IMPL@9..20
        IMPL_KW@9..13 "impl"
        WHITESPACE@13..14 " "
        PATH_TYPE@14..17
          PATH@14..17
            PATH_SEGMENT@14..17
              NAME_REF@14..17
                IDENT@14..17 "Foo"
        WHITESPACE@17..18 " "
        ASSOCIATED_ITEM_LIST@18..20
          L_CURLY@18..19 "{"
          R_CURLY@19..20 "}"
      WHITESPACE@20..29 "\n        "
      IMPL@29..99
        IMPL_KW@29..33 "impl"
        WHITESPACE@33..34 " "
        PATH_TYPE@34..37
          PATH@34..37
            PATH_SEGMENT@34..37
              NAME_REF@34..37
                IDENT@34..37 "Bar"
        WHITESPACE@37..38 " "
        ASSOCIATED_ITEM_LIST@38..99
          L_CURLY@38..39 "{"
          FUNCTION_DEF@39..63
            WHITESPACE@39..52 "\n            "
            FN_KW@52..54 "fn"
            WHITESPACE@54..55 " "
            NAME@55..58
              IDENT@55..58 "bar"
            PARAM_LIST@58..60
              L_PAREN@58..59 "("
              R_PAREN@59..60 ")"
            WHITESPACE@60..61 " "
            BLOCK_EXPR@61..63
              L_CURLY@61..62 "{"
              R_CURLY@62..63 "}"
          WHITESPACE@63..76 "\n            "
          STRUCT_DEF@76..89
            STRUCT_KW@76..82 "struct"
            WHITESPACE@82..83 " "
            NAME@83..86
              IDENT@83..86 "Baz"
            WHITESPACE@86..87 " "
            RECORD_FIELD_DEF_LIST@87..89
              L_CURLY@87..88 "{"
              R_CURLY@88..89 "}"
          WHITESPACE@89..98 "\n        "
          R_CURLY@98..99 "}"
      WHITESPACE@99..108 "\n        "
      IMPL@108..126
        VISIBILITY@108..111
          PUB_KW@108..111 "pub"
        WHITESPACE@111..112 " "
        IMPL_KW@112..116 "impl"
        WHITESPACE@116..117 " "
        PATH_TYPE@117..123
          PATH@117..123
            PATH_SEGMENT@117..123
              NAME_REF@117..123
                IDENT@117..123 "FooBar"
        WHITESPACE@123..124 " "
        ASSOCIATED_ITEM_LIST@124..126
          L_CURLY@124..125 "{"
          R_CURLY@125..126 "}"
      WHITESPACE@126..135 "\n        "
    error Range(76..89): only functions are allowed in impl blocks
    error Range(108..111): visibility is not allowed on impl blocks
    "###);
}

#[test]
fn array_type() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main(a: [int]) {
        let a:[[bool]];
    }"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..54
      FUNCTION_DEF@0..54
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..22
          L_PAREN@12..13 "("
          PARAM@13..21
            BIND_PAT@13..14
              NAME@13..14
                IDENT@13..14 "a"
            COLON@14..15 ":"
            WHITESPACE@15..16 " "
            ARRAY_TYPE@16..21
              L_BRACKET@16..17 "["
              PATH_TYPE@17..20
                PATH@17..20
                  PATH_SEGMENT@17..20
                    NAME_REF@17..20
                      IDENT@17..20 "int"
              R_BRACKET@20..21 "]"
          R_PAREN@21..22 ")"
        WHITESPACE@22..23 " "
        BLOCK_EXPR@23..54
          L_CURLY@23..24 "{"
          WHITESPACE@24..33 "\n        "
          LET_STMT@33..48
            LET_KW@33..36 "let"
            WHITESPACE@36..37 " "
            BIND_PAT@37..38
              NAME@37..38
                IDENT@37..38 "a"
            COLON@38..39 ":"
            ARRAY_TYPE@39..47
              L_BRACKET@39..40 "["
              ARRAY_TYPE@40..46
                L_BRACKET@40..41 "["
                PATH_TYPE@41..45
                  PATH@41..45
                    PATH_SEGMENT@41..45
                      NAME_REF@41..45
                        IDENT@41..45 "bool"
                R_BRACKET@45..46 "]"
              R_BRACKET@46..47 "]"
            SEMI@47..48 ";"
          WHITESPACE@48..53 "\n    "
          R_CURLY@53..54 "}"
    "#
    );
}

#[test]
fn index_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        let a = [1,2,3,4]
        let b = a[0];
        let c = a[b];
        a[0] = c;
        let a = { [3,4,5] }[1];
    }"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..142
      FUNCTION_DEF@0..142
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..142
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          LET_STMT@25..42
            LET_KW@25..28 "let"
            WHITESPACE@28..29 " "
            BIND_PAT@29..30
              NAME@29..30
                IDENT@29..30 "a"
            WHITESPACE@30..31 " "
            EQ@31..32 "="
            WHITESPACE@32..33 " "
            ARRAY_EXPR@33..42
              L_BRACKET@33..34 "["
              LITERAL@34..35
                INT_NUMBER@34..35 "1"
              COMMA@35..36 ","
              LITERAL@36..37
                INT_NUMBER@36..37 "2"
              COMMA@37..38 ","
              LITERAL@38..39
                INT_NUMBER@38..39 "3"
              COMMA@39..40 ","
              LITERAL@40..41
                INT_NUMBER@40..41 "4"
              R_BRACKET@41..42 "]"
          WHITESPACE@42..51 "\n        "
          LET_STMT@51..64
            LET_KW@51..54 "let"
            WHITESPACE@54..55 " "
            BIND_PAT@55..56
              NAME@55..56
                IDENT@55..56 "b"
            WHITESPACE@56..57 " "
            EQ@57..58 "="
            WHITESPACE@58..59 " "
            INDEX_EXPR@59..63
              PATH_EXPR@59..60
                PATH@59..60
                  PATH_SEGMENT@59..60
                    NAME_REF@59..60
                      IDENT@59..60 "a"
              L_BRACKET@60..61 "["
              LITERAL@61..62
                INT_NUMBER@61..62 "0"
              R_BRACKET@62..63 "]"
            SEMI@63..64 ";"
          WHITESPACE@64..73 "\n        "
          LET_STMT@73..86
            LET_KW@73..76 "let"
            WHITESPACE@76..77 " "
            BIND_PAT@77..78
              NAME@77..78
                IDENT@77..78 "c"
            WHITESPACE@78..79 " "
            EQ@79..80 "="
            WHITESPACE@80..81 " "
            INDEX_EXPR@81..85
              PATH_EXPR@81..82
                PATH@81..82
                  PATH_SEGMENT@81..82
                    NAME_REF@81..82
                      IDENT@81..82 "a"
              L_BRACKET@82..83 "["
              PATH_EXPR@83..84
                PATH@83..84
                  PATH_SEGMENT@83..84
                    NAME_REF@83..84
                      IDENT@83..84 "b"
              R_BRACKET@84..85 "]"
            SEMI@85..86 ";"
          WHITESPACE@86..95 "\n        "
          EXPR_STMT@95..104
            BIN_EXPR@95..103
              INDEX_EXPR@95..99
                PATH_EXPR@95..96
                  PATH@95..96
                    PATH_SEGMENT@95..96
                      NAME_REF@95..96
                        IDENT@95..96 "a"
                L_BRACKET@96..97 "["
                LITERAL@97..98
                  INT_NUMBER@97..98 "0"
                R_BRACKET@98..99 "]"
              WHITESPACE@99..100 " "
              EQ@100..101 "="
              WHITESPACE@101..102 " "
              PATH_EXPR@102..103
                PATH@102..103
                  PATH_SEGMENT@102..103
                    NAME_REF@102..103
                      IDENT@102..103 "c"
            SEMI@103..104 ";"
          WHITESPACE@104..113 "\n        "
          LET_STMT@113..132
            LET_KW@113..116 "let"
            WHITESPACE@116..117 " "
            BIND_PAT@117..118
              NAME@117..118
                IDENT@117..118 "a"
            WHITESPACE@118..119 " "
            EQ@119..120 "="
            WHITESPACE@120..121 " "
            BLOCK_EXPR@121..132
              L_CURLY@121..122 "{"
              WHITESPACE@122..123 " "
              ARRAY_EXPR@123..130
                L_BRACKET@123..124 "["
                LITERAL@124..125
                  INT_NUMBER@124..125 "3"
                COMMA@125..126 ","
                LITERAL@126..127
                  INT_NUMBER@126..127 "4"
                COMMA@127..128 ","
                LITERAL@128..129
                  INT_NUMBER@128..129 "5"
                R_BRACKET@129..130 "]"
              WHITESPACE@130..131 " "
              R_CURLY@131..132 "}"
          EXPR_STMT@132..136
            ARRAY_EXPR@132..135
              L_BRACKET@132..133 "["
              LITERAL@133..134
                INT_NUMBER@133..134 "1"
              R_BRACKET@134..135 "]"
            SEMI@135..136 ";"
          WHITESPACE@136..141 "\n    "
          R_CURLY@141..142 "}"
    "#
    );
}

#[test]
fn array_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        let a = [1,2,3,]
        let a = []
        let a = [call(123)]
        let a = [Struct { }, Struct { }]
    }"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..135
      FUNCTION_DEF@0..135
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..135
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          LET_STMT@25..41
            LET_KW@25..28 "let"
            WHITESPACE@28..29 " "
            BIND_PAT@29..30
              NAME@29..30
                IDENT@29..30 "a"
            WHITESPACE@30..31 " "
            EQ@31..32 "="
            WHITESPACE@32..33 " "
            ARRAY_EXPR@33..41
              L_BRACKET@33..34 "["
              LITERAL@34..35
                INT_NUMBER@34..35 "1"
              COMMA@35..36 ","
              LITERAL@36..37
                INT_NUMBER@36..37 "2"
              COMMA@37..38 ","
              LITERAL@38..39
                INT_NUMBER@38..39 "3"
              COMMA@39..40 ","
              R_BRACKET@40..41 "]"
          WHITESPACE@41..50 "\n        "
          LET_STMT@50..60
            LET_KW@50..53 "let"
            WHITESPACE@53..54 " "
            BIND_PAT@54..55
              NAME@54..55
                IDENT@54..55 "a"
            WHITESPACE@55..56 " "
            EQ@56..57 "="
            WHITESPACE@57..58 " "
            ARRAY_EXPR@58..60
              L_BRACKET@58..59 "["
              R_BRACKET@59..60 "]"
          WHITESPACE@60..69 "\n        "
          LET_STMT@69..88
            LET_KW@69..72 "let"
            WHITESPACE@72..73 " "
            BIND_PAT@73..74
              NAME@73..74
                IDENT@73..74 "a"
            WHITESPACE@74..75 " "
            EQ@75..76 "="
            WHITESPACE@76..77 " "
            ARRAY_EXPR@77..88
              L_BRACKET@77..78 "["
              CALL_EXPR@78..87
                PATH_EXPR@78..82
                  PATH@78..82
                    PATH_SEGMENT@78..82
                      NAME_REF@78..82
                        IDENT@78..82 "call"
                ARG_LIST@82..87
                  L_PAREN@82..83 "("
                  LITERAL@83..86
                    INT_NUMBER@83..86 "123"
                  R_PAREN@86..87 ")"
              R_BRACKET@87..88 "]"
          WHITESPACE@88..97 "\n        "
          LET_STMT@97..129
            LET_KW@97..100 "let"
            WHITESPACE@100..101 " "
            BIND_PAT@101..102
              NAME@101..102
                IDENT@101..102 "a"
            WHITESPACE@102..103 " "
            EQ@103..104 "="
            WHITESPACE@104..105 " "
            ARRAY_EXPR@105..129
              L_BRACKET@105..106 "["
              RECORD_LIT@106..116
                PATH_TYPE@106..112
                  PATH@106..112
                    PATH_SEGMENT@106..112
                      NAME_REF@106..112
                        IDENT@106..112 "Struct"
                WHITESPACE@112..113 " "
                RECORD_FIELD_LIST@113..116
                  L_CURLY@113..114 "{"
                  WHITESPACE@114..115 " "
                  R_CURLY@115..116 "}"
              COMMA@116..117 ","
              WHITESPACE@117..118 " "
              RECORD_LIT@118..128
                PATH_TYPE@118..124
                  PATH@118..124
                    PATH_SEGMENT@118..124
                      NAME_REF@118..124
                        IDENT@118..124 "Struct"
                WHITESPACE@124..125 " "
                RECORD_FIELD_LIST@125..128
                  L_CURLY@125..126 "{"
                  WHITESPACE@126..127 " "
                  R_CURLY@127..128 "}"
              R_BRACKET@128..129 "]"
          WHITESPACE@129..134 "\n    "
          R_CURLY@134..135 "}"
    "#
    );
}

#[test]
fn missing_field_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        var.
    }"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..34
      FUNCTION_DEF@0..34
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..34
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          FIELD_EXPR@24..28
            PATH_EXPR@24..27
              PATH@24..27
                PATH_SEGMENT@24..27
                  NAME_REF@24..27
                    IDENT@24..27 "var"
            DOT@27..28 "."
          WHITESPACE@28..33 "\n    "
          R_CURLY@33..34 "}"
    error Offset(28): expected field name or number
    "#);
}

#[test]
fn empty() {
    insta::assert_snapshot!(SourceFile::parse(r#""#).debug_dump(), @"SOURCE_FILE@0..0
");
}

#[test]
fn function() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    // Source file comment

    // Comment that belongs to the function
    fn a() {}
    fn b(value:number) {}
    pub fn d() {}
    pub fn c()->never {}
    fn b(value:number)->number {}"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..189
      WHITESPACE@0..5 "\n    "
      COMMENT@5..27 "// Source file comment"
      WHITESPACE@27..33 "\n\n    "
      FUNCTION_DEF@33..86
        COMMENT@33..72 "// Comment that belon ..."
        WHITESPACE@72..77 "\n    "
        FN_KW@77..79 "fn"
        WHITESPACE@79..80 " "
        NAME@80..81
          IDENT@80..81 "a"
        PARAM_LIST@81..83
          L_PAREN@81..82 "("
          R_PAREN@82..83 ")"
        WHITESPACE@83..84 " "
        BLOCK_EXPR@84..86
          L_CURLY@84..85 "{"
          R_CURLY@85..86 "}"
      FUNCTION_DEF@86..112
        WHITESPACE@86..91 "\n    "
        FN_KW@91..93 "fn"
        WHITESPACE@93..94 " "
        NAME@94..95
          IDENT@94..95 "b"
        PARAM_LIST@95..109
          L_PAREN@95..96 "("
          PARAM@96..108
            BIND_PAT@96..101
              NAME@96..101
                IDENT@96..101 "value"
            COLON@101..102 ":"
            PATH_TYPE@102..108
              PATH@102..108
                PATH_SEGMENT@102..108
                  NAME_REF@102..108
                    IDENT@102..108 "number"
          R_PAREN@108..109 ")"
        WHITESPACE@109..110 " "
        BLOCK_EXPR@110..112
          L_CURLY@110..111 "{"
          R_CURLY@111..112 "}"
      FUNCTION_DEF@112..130
        WHITESPACE@112..117 "\n    "
        VISIBILITY@117..120
          PUB_KW@117..120 "pub"
        WHITESPACE@120..121 " "
        FN_KW@121..123 "fn"
        WHITESPACE@123..124 " "
        NAME@124..125
          IDENT@124..125 "d"
        PARAM_LIST@125..127
          L_PAREN@125..126 "("
          R_PAREN@126..127 ")"
        WHITESPACE@127..128 " "
        BLOCK_EXPR@128..130
          L_CURLY@128..129 "{"
          R_CURLY@129..130 "}"
      FUNCTION_DEF@130..155
        WHITESPACE@130..135 "\n    "
        VISIBILITY@135..138
          PUB_KW@135..138 "pub"
        WHITESPACE@138..139 " "
        FN_KW@139..141 "fn"
        WHITESPACE@141..142 " "
        NAME@142..143
          IDENT@142..143 "c"
        PARAM_LIST@143..145
          L_PAREN@143..144 "("
          R_PAREN@144..145 ")"
        RET_TYPE@145..152
          THIN_ARROW@145..147 "->"
          NEVER_TYPE@147..152
            NEVER_KW@147..152 "never"
        WHITESPACE@152..153 " "
        BLOCK_EXPR@153..155
          L_CURLY@153..154 "{"
          R_CURLY@154..155 "}"
      FUNCTION_DEF@155..189
        WHITESPACE@155..160 "\n    "
        FN_KW@160..162 "fn"
        WHITESPACE@162..163 " "
        NAME@163..164
          IDENT@163..164 "b"
        PARAM_LIST@164..178
          L_PAREN@164..165 "("
          PARAM@165..177
            BIND_PAT@165..170
              NAME@165..170
                IDENT@165..170 "value"
            COLON@170..171 ":"
            PATH_TYPE@171..177
              PATH@171..177
                PATH_SEGMENT@171..177
                  NAME_REF@171..177
                    IDENT@171..177 "number"
          R_PAREN@177..178 ")"
        RET_TYPE@178..186
          THIN_ARROW@178..180 "->"
          PATH_TYPE@180..186
            PATH@180..186
              PATH_SEGMENT@180..186
                NAME_REF@180..186
                  IDENT@180..186 "number"
        WHITESPACE@186..187 " "
        BLOCK_EXPR@187..189
          L_CURLY@187..188 "{"
          R_CURLY@188..189 "}"
    "#);
}

#[test]
fn block() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        let a;
        let b:i32;
        let c:string;
    }"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..77
      FUNCTION_DEF@0..77
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..77
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          LET_STMT@24..30
            LET_KW@24..27 "let"
            WHITESPACE@27..28 " "
            BIND_PAT@28..29
              NAME@28..29
                IDENT@28..29 "a"
            SEMI@29..30 ";"
          WHITESPACE@30..39 "\n        "
          LET_STMT@39..49
            LET_KW@39..42 "let"
            WHITESPACE@42..43 " "
            BIND_PAT@43..44
              NAME@43..44
                IDENT@43..44 "b"
            COLON@44..45 ":"
            PATH_TYPE@45..48
              PATH@45..48
                PATH_SEGMENT@45..48
                  NAME_REF@45..48
                    IDENT@45..48 "i32"
            SEMI@48..49 ";"
          WHITESPACE@49..58 "\n        "
          LET_STMT@58..71
            LET_KW@58..61 "let"
            WHITESPACE@61..62 " "
            BIND_PAT@62..63
              NAME@62..63
                IDENT@62..63 "c"
            COLON@63..64 ":"
            PATH_TYPE@64..70
              PATH@64..70
                PATH_SEGMENT@64..70
                  NAME_REF@64..70
                    IDENT@64..70 "string"
            SEMI@70..71 ";"
          WHITESPACE@71..76 "\n    "
          R_CURLY@76..77 "}"
    "#);
}

#[test]
fn literals() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        let a = true;
        let b = false;
        let c = 1;
        let d = 1.12;
        let e = "Hello, world!"
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..144
      FUNCTION_DEF@0..139
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..139
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          LET_STMT@24..37
            LET_KW@24..27 "let"
            WHITESPACE@27..28 " "
            BIND_PAT@28..29
              NAME@28..29
                IDENT@28..29 "a"
            WHITESPACE@29..30 " "
            EQ@30..31 "="
            WHITESPACE@31..32 " "
            LITERAL@32..36
              TRUE_KW@32..36 "true"
            SEMI@36..37 ";"
          WHITESPACE@37..46 "\n        "
          LET_STMT@46..60
            LET_KW@46..49 "let"
            WHITESPACE@49..50 " "
            BIND_PAT@50..51
              NAME@50..51
                IDENT@50..51 "b"
            WHITESPACE@51..52 " "
            EQ@52..53 "="
            WHITESPACE@53..54 " "
            LITERAL@54..59
              FALSE_KW@54..59 "false"
            SEMI@59..60 ";"
          WHITESPACE@60..69 "\n        "
          LET_STMT@69..79
            LET_KW@69..72 "let"
            WHITESPACE@72..73 " "
            BIND_PAT@73..74
              NAME@73..74
                IDENT@73..74 "c"
            WHITESPACE@74..75 " "
            EQ@75..76 "="
            WHITESPACE@76..77 " "
            LITERAL@77..78
              INT_NUMBER@77..78 "1"
            SEMI@78..79 ";"
          WHITESPACE@79..88 "\n        "
          LET_STMT@88..101
            LET_KW@88..91 "let"
            WHITESPACE@91..92 " "
            BIND_PAT@92..93
              NAME@92..93
                IDENT@92..93 "d"
            WHITESPACE@93..94 " "
            EQ@94..95 "="
            WHITESPACE@95..96 " "
            LITERAL@96..100
              FLOAT_NUMBER@96..100 "1.12"
            SEMI@100..101 ";"
          WHITESPACE@101..110 "\n        "
          LET_STMT@110..133
            LET_KW@110..113 "let"
            WHITESPACE@113..114 " "
            BIND_PAT@114..115
              NAME@114..115
                IDENT@114..115 "e"
            WHITESPACE@115..116 " "
            EQ@116..117 "="
            WHITESPACE@117..118 " "
            LITERAL@118..133
              STRING@118..133 "\"Hello, world!\""
          WHITESPACE@133..138 "\n    "
          R_CURLY@138..139 "}"
      WHITESPACE@139..144 "\n    "
    "#);
}

#[test]
fn struct_def() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    struct Foo      // error: expected a ';', or a '{'
    struct Foo;
    struct Foo;;    // error: expected a declaration
    struct Foo {}
    struct Foo {};
    struct Foo {,}; // error: expected a field declaration
    struct Foo {
        a: f64,
    }
    struct Foo {
        a: f64,
        b: i32,
    };
    struct Foo()
    struct Foo();
    struct Foo(,);  // error: expected a type
    struct Foo(f64)
    struct Foo(f64,);
    struct Foo(f64, i32)
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..468
      WHITESPACE@0..5 "\n    "
      STRUCT_DEF@5..15
        STRUCT_KW@5..11 "struct"
        WHITESPACE@11..12 " "
        NAME@12..15
          IDENT@12..15 "Foo"
      WHITESPACE@15..21 "      "
      COMMENT@21..55 "// error: expected a  ..."
      WHITESPACE@55..60 "\n    "
      STRUCT_DEF@60..71
        STRUCT_KW@60..66 "struct"
        WHITESPACE@66..67 " "
        NAME@67..70
          IDENT@67..70 "Foo"
        SEMI@70..71 ";"
      WHITESPACE@71..76 "\n    "
      STRUCT_DEF@76..87
        STRUCT_KW@76..82 "struct"
        WHITESPACE@82..83 " "
        NAME@83..86
          IDENT@83..86 "Foo"
        SEMI@86..87 ";"
      ERROR@87..88
        SEMI@87..88 ";"
      WHITESPACE@88..92 "    "
      COMMENT@92..124 "// error: expected a  ..."
      WHITESPACE@124..129 "\n    "
      STRUCT_DEF@129..142
        STRUCT_KW@129..135 "struct"
        WHITESPACE@135..136 " "
        NAME@136..139
          IDENT@136..139 "Foo"
        WHITESPACE@139..140 " "
        RECORD_FIELD_DEF_LIST@140..142
          L_CURLY@140..141 "{"
          R_CURLY@141..142 "}"
      WHITESPACE@142..147 "\n    "
      STRUCT_DEF@147..161
        STRUCT_KW@147..153 "struct"
        WHITESPACE@153..154 " "
        NAME@154..157
          IDENT@154..157 "Foo"
        WHITESPACE@157..158 " "
        RECORD_FIELD_DEF_LIST@158..161
          L_CURLY@158..159 "{"
          R_CURLY@159..160 "}"
          SEMI@160..161 ";"
      WHITESPACE@161..166 "\n    "
      STRUCT_DEF@166..181
        STRUCT_KW@166..172 "struct"
        WHITESPACE@172..173 " "
        NAME@173..176
          IDENT@173..176 "Foo"
        WHITESPACE@176..177 " "
        RECORD_FIELD_DEF_LIST@177..181
          L_CURLY@177..178 "{"
          ERROR@178..179
            COMMA@178..179 ","
          R_CURLY@179..180 "}"
          SEMI@180..181 ";"
      WHITESPACE@181..182 " "
      COMMENT@182..220 "// error: expected a  ..."
      WHITESPACE@220..225 "\n    "
      STRUCT_DEF@225..259
        STRUCT_KW@225..231 "struct"
        WHITESPACE@231..232 " "
        NAME@232..235
          IDENT@232..235 "Foo"
        WHITESPACE@235..236 " "
        RECORD_FIELD_DEF_LIST@236..259
          L_CURLY@236..237 "{"
          WHITESPACE@237..246 "\n        "
          RECORD_FIELD_DEF@246..252
            NAME@246..247
              IDENT@246..247 "a"
            COLON@247..248 ":"
            WHITESPACE@248..249 " "
            PATH_TYPE@249..252
              PATH@249..252
                PATH_SEGMENT@249..252
                  NAME_REF@249..252
                    IDENT@249..252 "f64"
          COMMA@252..253 ","
          WHITESPACE@253..258 "\n    "
          R_CURLY@258..259 "}"
      WHITESPACE@259..264 "\n    "
      STRUCT_DEF@264..315
        STRUCT_KW@264..270 "struct"
        WHITESPACE@270..271 " "
        NAME@271..274
          IDENT@271..274 "Foo"
        WHITESPACE@274..275 " "
        RECORD_FIELD_DEF_LIST@275..315
          L_CURLY@275..276 "{"
          WHITESPACE@276..285 "\n        "
          RECORD_FIELD_DEF@285..291
            NAME@285..286
              IDENT@285..286 "a"
            COLON@286..287 ":"
            WHITESPACE@287..288 " "
            PATH_TYPE@288..291
              PATH@288..291
                PATH_SEGMENT@288..291
                  NAME_REF@288..291
                    IDENT@288..291 "f64"
          COMMA@291..292 ","
          WHITESPACE@292..301 "\n        "
          RECORD_FIELD_DEF@301..307
            NAME@301..302
              IDENT@301..302 "b"
            COLON@302..303 ":"
            WHITESPACE@303..304 " "
            PATH_TYPE@304..307
              PATH@304..307
                PATH_SEGMENT@304..307
                  NAME_REF@304..307
                    IDENT@304..307 "i32"
          COMMA@307..308 ","
          WHITESPACE@308..313 "\n    "
          R_CURLY@313..314 "}"
          SEMI@314..315 ";"
      WHITESPACE@315..320 "\n    "
      STRUCT_DEF@320..332
        STRUCT_KW@320..326 "struct"
        WHITESPACE@326..327 " "
        NAME@327..330
          IDENT@327..330 "Foo"
        TUPLE_FIELD_DEF_LIST@330..332
          L_PAREN@330..331 "("
          R_PAREN@331..332 ")"
      WHITESPACE@332..337 "\n    "
      STRUCT_DEF@337..350
        STRUCT_KW@337..343 "struct"
        WHITESPACE@343..344 " "
        NAME@344..347
          IDENT@344..347 "Foo"
        TUPLE_FIELD_DEF_LIST@347..350
          L_PAREN@347..348 "("
          R_PAREN@348..349 ")"
          SEMI@349..350 ";"
      WHITESPACE@350..355 "\n    "
      STRUCT_DEF@355..369
        STRUCT_KW@355..361 "struct"
        WHITESPACE@361..362 " "
        NAME@362..365
          IDENT@362..365 "Foo"
        TUPLE_FIELD_DEF_LIST@365..369
          L_PAREN@365..366 "("
          ERROR@366..367
            COMMA@366..367 ","
          R_PAREN@367..368 ")"
          SEMI@368..369 ";"
      WHITESPACE@369..371 "  "
      COMMENT@371..396 "// error: expected a  ..."
      WHITESPACE@396..401 "\n    "
      STRUCT_DEF@401..416
        STRUCT_KW@401..407 "struct"
        WHITESPACE@407..408 " "
        NAME@408..411
          IDENT@408..411 "Foo"
        TUPLE_FIELD_DEF_LIST@411..416
          L_PAREN@411..412 "("
          TUPLE_FIELD_DEF@412..415
            PATH_TYPE@412..415
              PATH@412..415
                PATH_SEGMENT@412..415
                  NAME_REF@412..415
                    IDENT@412..415 "f64"
          R_PAREN@415..416 ")"
      WHITESPACE@416..421 "\n    "
      STRUCT_DEF@421..438
        STRUCT_KW@421..427 "struct"
        WHITESPACE@427..428 " "
        NAME@428..431
          IDENT@428..431 "Foo"
        TUPLE_FIELD_DEF_LIST@431..438
          L_PAREN@431..432 "("
          TUPLE_FIELD_DEF@432..435
            PATH_TYPE@432..435
              PATH@432..435
                PATH_SEGMENT@432..435
                  NAME_REF@432..435
                    IDENT@432..435 "f64"
          COMMA@435..436 ","
          R_PAREN@436..437 ")"
          SEMI@437..438 ";"
      WHITESPACE@438..443 "\n    "
      STRUCT_DEF@443..463
        STRUCT_KW@443..449 "struct"
        WHITESPACE@449..450 " "
        NAME@450..453
          IDENT@450..453 "Foo"
        TUPLE_FIELD_DEF_LIST@453..463
          L_PAREN@453..454 "("
          TUPLE_FIELD_DEF@454..457
            PATH_TYPE@454..457
              PATH@454..457
                PATH_SEGMENT@454..457
                  NAME_REF@454..457
                    IDENT@454..457 "f64"
          COMMA@457..458 ","
          WHITESPACE@458..459 " "
          TUPLE_FIELD_DEF@459..462
            PATH_TYPE@459..462
              PATH@459..462
                PATH_SEGMENT@459..462
                  NAME_REF@459..462
                    IDENT@459..462 "i32"
          R_PAREN@462..463 ")"
      WHITESPACE@463..468 "\n    "
    error Offset(15): expected a ';', '{', or '('
    error Offset(87): expected a declaration
    error Offset(178): expected a field declaration
    error Offset(366): expected a type
    "#);
}

#[test]
fn unary_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        let a = --3;
        let b = !!true;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..71
      FUNCTION_DEF@0..66
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..66
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          LET_STMT@24..36
            LET_KW@24..27 "let"
            WHITESPACE@27..28 " "
            BIND_PAT@28..29
              NAME@28..29
                IDENT@28..29 "a"
            WHITESPACE@29..30 " "
            EQ@30..31 "="
            WHITESPACE@31..32 " "
            PREFIX_EXPR@32..35
              MINUS@32..33 "-"
              PREFIX_EXPR@33..35
                MINUS@33..34 "-"
                LITERAL@34..35
                  INT_NUMBER@34..35 "3"
            SEMI@35..36 ";"
          WHITESPACE@36..45 "\n        "
          LET_STMT@45..60
            LET_KW@45..48 "let"
            WHITESPACE@48..49 " "
            BIND_PAT@49..50
              NAME@49..50
                IDENT@49..50 "b"
            WHITESPACE@50..51 " "
            EQ@51..52 "="
            WHITESPACE@52..53 " "
            PREFIX_EXPR@53..59
              EXCLAMATION@53..54 "!"
              PREFIX_EXPR@54..59
                EXCLAMATION@54..55 "!"
                LITERAL@55..59
                  TRUE_KW@55..59 "true"
            SEMI@59..60 ";"
          WHITESPACE@60..65 "\n    "
          R_CURLY@65..66 "}"
      WHITESPACE@66..71 "\n    "
    "#);
}

#[test]
fn binary_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        let a = 3+4*5
        let b = 3*4+10/2
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..73
      FUNCTION_DEF@0..68
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..68
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          LET_STMT@24..37
            LET_KW@24..27 "let"
            WHITESPACE@27..28 " "
            BIND_PAT@28..29
              NAME@28..29
                IDENT@28..29 "a"
            WHITESPACE@29..30 " "
            EQ@30..31 "="
            WHITESPACE@31..32 " "
            BIN_EXPR@32..37
              LITERAL@32..33
                INT_NUMBER@32..33 "3"
              PLUS@33..34 "+"
              BIN_EXPR@34..37
                LITERAL@34..35
                  INT_NUMBER@34..35 "4"
                STAR@35..36 "*"
                LITERAL@36..37
                  INT_NUMBER@36..37 "5"
          WHITESPACE@37..46 "\n        "
          LET_STMT@46..62
            LET_KW@46..49 "let"
            WHITESPACE@49..50 " "
            BIND_PAT@50..51
              NAME@50..51
                IDENT@50..51 "b"
            WHITESPACE@51..52 " "
            EQ@52..53 "="
            WHITESPACE@53..54 " "
            BIN_EXPR@54..62
              BIN_EXPR@54..57
                LITERAL@54..55
                  INT_NUMBER@54..55 "3"
                STAR@55..56 "*"
                LITERAL@56..57
                  INT_NUMBER@56..57 "4"
              PLUS@57..58 "+"
              BIN_EXPR@58..62
                LITERAL@58..60
                  INT_NUMBER@58..60 "10"
                SLASH@60..61 "/"
                LITERAL@61..62
                  INT_NUMBER@61..62 "2"
          WHITESPACE@62..67 "\n    "
          R_CURLY@67..68 "}"
      WHITESPACE@68..73 "\n    "
    "#);
}

#[test]
fn expression_statement() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        let a = "hello"
        let b = "world"
        let c
        b = "Hello, world!"
        !-5+2*(a+b);
        -3
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..148
      FUNCTION_DEF@0..143
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..143
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          LET_STMT@24..39
            LET_KW@24..27 "let"
            WHITESPACE@27..28 " "
            BIND_PAT@28..29
              NAME@28..29
                IDENT@28..29 "a"
            WHITESPACE@29..30 " "
            EQ@30..31 "="
            WHITESPACE@31..32 " "
            LITERAL@32..39
              STRING@32..39 "\"hello\""
          WHITESPACE@39..48 "\n        "
          LET_STMT@48..63
            LET_KW@48..51 "let"
            WHITESPACE@51..52 " "
            BIND_PAT@52..53
              NAME@52..53
                IDENT@52..53 "b"
            WHITESPACE@53..54 " "
            EQ@54..55 "="
            WHITESPACE@55..56 " "
            LITERAL@56..63
              STRING@56..63 "\"world\""
          WHITESPACE@63..72 "\n        "
          LET_STMT@72..77
            LET_KW@72..75 "let"
            WHITESPACE@75..76 " "
            BIND_PAT@76..77
              NAME@76..77
                IDENT@76..77 "c"
          WHITESPACE@77..86 "\n        "
          EXPR_STMT@86..105
            BIN_EXPR@86..105
              PATH_EXPR@86..87
                PATH@86..87
                  PATH_SEGMENT@86..87
                    NAME_REF@86..87
                      IDENT@86..87 "b"
              WHITESPACE@87..88 " "
              EQ@88..89 "="
              WHITESPACE@89..90 " "
              LITERAL@90..105
                STRING@90..105 "\"Hello, world!\""
          WHITESPACE@105..114 "\n        "
          EXPR_STMT@114..126
            BIN_EXPR@114..125
              PREFIX_EXPR@114..117
                EXCLAMATION@114..115 "!"
                PREFIX_EXPR@115..117
                  MINUS@115..116 "-"
                  LITERAL@116..117
                    INT_NUMBER@116..117 "5"
              PLUS@117..118 "+"
              BIN_EXPR@118..125
                LITERAL@118..119
                  INT_NUMBER@118..119 "2"
                STAR@119..120 "*"
                PAREN_EXPR@120..125
                  L_PAREN@120..121 "("
                  BIN_EXPR@121..124
                    PATH_EXPR@121..122
                      PATH@121..122
                        PATH_SEGMENT@121..122
                          NAME_REF@121..122
                            IDENT@121..122 "a"
                    PLUS@122..123 "+"
                    PATH_EXPR@123..124
                      PATH@123..124
                        PATH_SEGMENT@123..124
                          NAME_REF@123..124
                            IDENT@123..124 "b"
                  R_PAREN@124..125 ")"
            SEMI@125..126 ";"
          WHITESPACE@126..135 "\n        "
          PREFIX_EXPR@135..137
            MINUS@135..136 "-"
            LITERAL@136..137
              INT_NUMBER@136..137 "3"
          WHITESPACE@137..142 "\n    "
          R_CURLY@142..143 "}"
      WHITESPACE@143..148 "\n    "
    "#);
}

#[test]
fn function_calls() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn bar(i:number) { }
    fn foo(i:number) {
      bar(i+1)
    }
    fn baz(self) { }
    fn qux(self, i:number) { }
    fn foo(self i:number) { } // error: expected comma
    "#,
    ).debug_dump(), @r###"
    SOURCE_FILE@0..181
      FUNCTION_DEF@0..25
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "bar"
        PARAM_LIST@11..21
          L_PAREN@11..12 "("
          PARAM@12..20
            BIND_PAT@12..13
              NAME@12..13
                IDENT@12..13 "i"
            COLON@13..14 ":"
            PATH_TYPE@14..20
              PATH@14..20
                PATH_SEGMENT@14..20
                  NAME_REF@14..20
                    IDENT@14..20 "number"
          R_PAREN@20..21 ")"
        WHITESPACE@21..22 " "
        BLOCK_EXPR@22..25
          L_CURLY@22..23 "{"
          WHITESPACE@23..24 " "
          R_CURLY@24..25 "}"
      FUNCTION_DEF@25..69
        WHITESPACE@25..30 "\n    "
        FN_KW@30..32 "fn"
        WHITESPACE@32..33 " "
        NAME@33..36
          IDENT@33..36 "foo"
        PARAM_LIST@36..46
          L_PAREN@36..37 "("
          PARAM@37..45
            BIND_PAT@37..38
              NAME@37..38
                IDENT@37..38 "i"
            COLON@38..39 ":"
            PATH_TYPE@39..45
              PATH@39..45
                PATH_SEGMENT@39..45
                  NAME_REF@39..45
                    IDENT@39..45 "number"
          R_PAREN@45..46 ")"
        WHITESPACE@46..47 " "
        BLOCK_EXPR@47..69
          L_CURLY@47..48 "{"
          WHITESPACE@48..55 "\n      "
          CALL_EXPR@55..63
            PATH_EXPR@55..58
              PATH@55..58
                PATH_SEGMENT@55..58
                  NAME_REF@55..58
                    IDENT@55..58 "bar"
            ARG_LIST@58..63
              L_PAREN@58..59 "("
              BIN_EXPR@59..62
                PATH_EXPR@59..60
                  PATH@59..60
                    PATH_SEGMENT@59..60
                      NAME_REF@59..60
                        IDENT@59..60 "i"
                PLUS@60..61 "+"
                LITERAL@61..62
                  INT_NUMBER@61..62 "1"
              R_PAREN@62..63 ")"
          WHITESPACE@63..68 "\n    "
          R_CURLY@68..69 "}"
      FUNCTION_DEF@69..90
        WHITESPACE@69..74 "\n    "
        FN_KW@74..76 "fn"
        WHITESPACE@76..77 " "
        NAME@77..80
          IDENT@77..80 "baz"
        PARAM_LIST@80..86
          L_PAREN@80..81 "("
          SELF_PARAM@81..85
            NAME@81..85
              SELF_KW@81..85 "self"
          R_PAREN@85..86 ")"
        WHITESPACE@86..87 " "
        BLOCK_EXPR@87..90
          L_CURLY@87..88 "{"
          WHITESPACE@88..89 " "
          R_CURLY@89..90 "}"
      FUNCTION_DEF@90..121
        WHITESPACE@90..95 "\n    "
        FN_KW@95..97 "fn"
        WHITESPACE@97..98 " "
        NAME@98..101
          IDENT@98..101 "qux"
        PARAM_LIST@101..117
          L_PAREN@101..102 "("
          SELF_PARAM@102..106
            NAME@102..106
              SELF_KW@102..106 "self"
          COMMA@106..107 ","
          WHITESPACE@107..108 " "
          PARAM@108..116
            BIND_PAT@108..109
              NAME@108..109
                IDENT@108..109 "i"
            COLON@109..110 ":"
            PATH_TYPE@110..116
              PATH@110..116
                PATH_SEGMENT@110..116
                  NAME_REF@110..116
                    IDENT@110..116 "number"
          R_PAREN@116..117 ")"
        WHITESPACE@117..118 " "
        BLOCK_EXPR@118..121
          L_CURLY@118..119 "{"
          WHITESPACE@119..120 " "
          R_CURLY@120..121 "}"
      FUNCTION_DEF@121..151
        WHITESPACE@121..126 "\n    "
        FN_KW@126..128 "fn"
        WHITESPACE@128..129 " "
        NAME@129..132
          IDENT@129..132 "foo"
        PARAM_LIST@132..147
          L_PAREN@132..133 "("
          SELF_PARAM@133..137
            NAME@133..137
              SELF_KW@133..137 "self"
          WHITESPACE@137..138 " "
          PARAM@138..146
            BIND_PAT@138..139
              NAME@138..139
                IDENT@138..139 "i"
            COLON@139..140 ":"
            PATH_TYPE@140..146
              PATH@140..146
                PATH_SEGMENT@140..146
                  NAME_REF@140..146
                    IDENT@140..146 "number"
          R_PAREN@146..147 ")"
        WHITESPACE@147..148 " "
        BLOCK_EXPR@148..151
          L_CURLY@148..149 "{"
          WHITESPACE@149..150 " "
          R_CURLY@150..151 "}"
      WHITESPACE@151..152 " "
      COMMENT@152..176 "// error: expected comma"
      WHITESPACE@176..181 "\n    "
    error Offset(137): expected COMMA
    "###);
}

#[test]
fn patterns() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main(_:number) {
       let a = 0;
       let _ = a;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..71
      FUNCTION_DEF@0..66
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..22
          L_PAREN@12..13 "("
          PARAM@13..21
            PLACEHOLDER_PAT@13..14
              UNDERSCORE@13..14 "_"
            COLON@14..15 ":"
            PATH_TYPE@15..21
              PATH@15..21
                PATH_SEGMENT@15..21
                  NAME_REF@15..21
                    IDENT@15..21 "number"
          R_PAREN@21..22 ")"
        WHITESPACE@22..23 " "
        BLOCK_EXPR@23..66
          L_CURLY@23..24 "{"
          WHITESPACE@24..32 "\n       "
          LET_STMT@32..42
            LET_KW@32..35 "let"
            WHITESPACE@35..36 " "
            BIND_PAT@36..37
              NAME@36..37
                IDENT@36..37 "a"
            WHITESPACE@37..38 " "
            EQ@38..39 "="
            WHITESPACE@39..40 " "
            LITERAL@40..41
              INT_NUMBER@40..41 "0"
            SEMI@41..42 ";"
          WHITESPACE@42..50 "\n       "
          LET_STMT@50..60
            LET_KW@50..53 "let"
            WHITESPACE@53..54 " "
            PLACEHOLDER_PAT@54..55
              UNDERSCORE@54..55 "_"
            WHITESPACE@55..56 " "
            EQ@56..57 "="
            WHITESPACE@57..58 " "
            PATH_EXPR@58..59
              PATH@58..59
                PATH_SEGMENT@58..59
                  NAME_REF@58..59
                    IDENT@58..59 "a"
            SEMI@59..60 ";"
          WHITESPACE@60..65 "\n    "
          R_CURLY@65..66 "}"
      WHITESPACE@66..71 "\n    "
    "#);
}

#[test]
fn arithmetic_operands() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        let _ = a + b;
        let _ = a - b;
        let _ = a * b;
        let _ = a / b;
        let _ = a % b;
        let _ = a << b;
        let _ = a >> b;
        let _ = a & b;
        let _ = a | b;
        let _ = a ^ b;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..259
      FUNCTION_DEF@0..254
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..254
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          LET_STMT@25..39
            LET_KW@25..28 "let"
            WHITESPACE@28..29 " "
            PLACEHOLDER_PAT@29..30
              UNDERSCORE@29..30 "_"
            WHITESPACE@30..31 " "
            EQ@31..32 "="
            WHITESPACE@32..33 " "
            BIN_EXPR@33..38
              PATH_EXPR@33..34
                PATH@33..34
                  PATH_SEGMENT@33..34
                    NAME_REF@33..34
                      IDENT@33..34 "a"
              WHITESPACE@34..35 " "
              PLUS@35..36 "+"
              WHITESPACE@36..37 " "
              PATH_EXPR@37..38
                PATH@37..38
                  PATH_SEGMENT@37..38
                    NAME_REF@37..38
                      IDENT@37..38 "b"
            SEMI@38..39 ";"
          WHITESPACE@39..48 "\n        "
          LET_STMT@48..62
            LET_KW@48..51 "let"
            WHITESPACE@51..52 " "
            PLACEHOLDER_PAT@52..53
              UNDERSCORE@52..53 "_"
            WHITESPACE@53..54 " "
            EQ@54..55 "="
            WHITESPACE@55..56 " "
            BIN_EXPR@56..61
              PATH_EXPR@56..57
                PATH@56..57
                  PATH_SEGMENT@56..57
                    NAME_REF@56..57
                      IDENT@56..57 "a"
              WHITESPACE@57..58 " "
              MINUS@58..59 "-"
              WHITESPACE@59..60 " "
              PATH_EXPR@60..61
                PATH@60..61
                  PATH_SEGMENT@60..61
                    NAME_REF@60..61
                      IDENT@60..61 "b"
            SEMI@61..62 ";"
          WHITESPACE@62..71 "\n        "
          LET_STMT@71..85
            LET_KW@71..74 "let"
            WHITESPACE@74..75 " "
            PLACEHOLDER_PAT@75..76
              UNDERSCORE@75..76 "_"
            WHITESPACE@76..77 " "
            EQ@77..78 "="
            WHITESPACE@78..79 " "
            BIN_EXPR@79..84
              PATH_EXPR@79..80
                PATH@79..80
                  PATH_SEGMENT@79..80
                    NAME_REF@79..80
                      IDENT@79..80 "a"
              WHITESPACE@80..81 " "
              STAR@81..82 "*"
              WHITESPACE@82..83 " "
              PATH_EXPR@83..84
                PATH@83..84
                  PATH_SEGMENT@83..84
                    NAME_REF@83..84
                      IDENT@83..84 "b"
            SEMI@84..85 ";"
          WHITESPACE@85..94 "\n        "
          LET_STMT@94..108
            LET_KW@94..97 "let"
            WHITESPACE@97..98 " "
            PLACEHOLDER_PAT@98..99
              UNDERSCORE@98..99 "_"
            WHITESPACE@99..100 " "
            EQ@100..101 "="
            WHITESPACE@101..102 " "
            BIN_EXPR@102..107
              PATH_EXPR@102..103
                PATH@102..103
                  PATH_SEGMENT@102..103
                    NAME_REF@102..103
                      IDENT@102..103 "a"
              WHITESPACE@103..104 " "
              SLASH@104..105 "/"
              WHITESPACE@105..106 " "
              PATH_EXPR@106..107
                PATH@106..107
                  PATH_SEGMENT@106..107
                    NAME_REF@106..107
                      IDENT@106..107 "b"
            SEMI@107..108 ";"
          WHITESPACE@108..117 "\n        "
          LET_STMT@117..131
            LET_KW@117..120 "let"
            WHITESPACE@120..121 " "
            PLACEHOLDER_PAT@121..122
              UNDERSCORE@121..122 "_"
            WHITESPACE@122..123 " "
            EQ@123..124 "="
            WHITESPACE@124..125 " "
            BIN_EXPR@125..130
              PATH_EXPR@125..126
                PATH@125..126
                  PATH_SEGMENT@125..126
                    NAME_REF@125..126
                      IDENT@125..126 "a"
              WHITESPACE@126..127 " "
              PERCENT@127..128 "%"
              WHITESPACE@128..129 " "
              PATH_EXPR@129..130
                PATH@129..130
                  PATH_SEGMENT@129..130
                    NAME_REF@129..130
                      IDENT@129..130 "b"
            SEMI@130..131 ";"
          WHITESPACE@131..140 "\n        "
          LET_STMT@140..155
            LET_KW@140..143 "let"
            WHITESPACE@143..144 " "
            PLACEHOLDER_PAT@144..145
              UNDERSCORE@144..145 "_"
            WHITESPACE@145..146 " "
            EQ@146..147 "="
            WHITESPACE@147..148 " "
            BIN_EXPR@148..154
              PATH_EXPR@148..149
                PATH@148..149
                  PATH_SEGMENT@148..149
                    NAME_REF@148..149
                      IDENT@148..149 "a"
              WHITESPACE@149..150 " "
              SHL@150..152 "<<"
              WHITESPACE@152..153 " "
              PATH_EXPR@153..154
                PATH@153..154
                  PATH_SEGMENT@153..154
                    NAME_REF@153..154
                      IDENT@153..154 "b"
            SEMI@154..155 ";"
          WHITESPACE@155..164 "\n        "
          LET_STMT@164..179
            LET_KW@164..167 "let"
            WHITESPACE@167..168 " "
            PLACEHOLDER_PAT@168..169
              UNDERSCORE@168..169 "_"
            WHITESPACE@169..170 " "
            EQ@170..171 "="
            WHITESPACE@171..172 " "
            BIN_EXPR@172..178
              PATH_EXPR@172..173
                PATH@172..173
                  PATH_SEGMENT@172..173
                    NAME_REF@172..173
                      IDENT@172..173 "a"
              WHITESPACE@173..174 " "
              SHR@174..176 ">>"
              WHITESPACE@176..177 " "
              PATH_EXPR@177..178
                PATH@177..178
                  PATH_SEGMENT@177..178
                    NAME_REF@177..178
                      IDENT@177..178 "b"
            SEMI@178..179 ";"
          WHITESPACE@179..188 "\n        "
          LET_STMT@188..202
            LET_KW@188..191 "let"
            WHITESPACE@191..192 " "
            PLACEHOLDER_PAT@192..193
              UNDERSCORE@192..193 "_"
            WHITESPACE@193..194 " "
            EQ@194..195 "="
            WHITESPACE@195..196 " "
            BIN_EXPR@196..201
              PATH_EXPR@196..197
                PATH@196..197
                  PATH_SEGMENT@196..197
                    NAME_REF@196..197
                      IDENT@196..197 "a"
              WHITESPACE@197..198 " "
              AMP@198..199 "&"
              WHITESPACE@199..200 " "
              PATH_EXPR@200..201
                PATH@200..201
                  PATH_SEGMENT@200..201
                    NAME_REF@200..201
                      IDENT@200..201 "b"
            SEMI@201..202 ";"
          WHITESPACE@202..211 "\n        "
          LET_STMT@211..225
            LET_KW@211..214 "let"
            WHITESPACE@214..215 " "
            PLACEHOLDER_PAT@215..216
              UNDERSCORE@215..216 "_"
            WHITESPACE@216..217 " "
            EQ@217..218 "="
            WHITESPACE@218..219 " "
            BIN_EXPR@219..224
              PATH_EXPR@219..220
                PATH@219..220
                  PATH_SEGMENT@219..220
                    NAME_REF@219..220
                      IDENT@219..220 "a"
              WHITESPACE@220..221 " "
              PIPE@221..222 "|"
              WHITESPACE@222..223 " "
              PATH_EXPR@223..224
                PATH@223..224
                  PATH_SEGMENT@223..224
                    NAME_REF@223..224
                      IDENT@223..224 "b"
            SEMI@224..225 ";"
          WHITESPACE@225..234 "\n        "
          LET_STMT@234..248
            LET_KW@234..237 "let"
            WHITESPACE@237..238 " "
            PLACEHOLDER_PAT@238..239
              UNDERSCORE@238..239 "_"
            WHITESPACE@239..240 " "
            EQ@240..241 "="
            WHITESPACE@241..242 " "
            BIN_EXPR@242..247
              PATH_EXPR@242..243
                PATH@242..243
                  PATH_SEGMENT@242..243
                    NAME_REF@242..243
                      IDENT@242..243 "a"
              WHITESPACE@243..244 " "
              CARET@244..245 "^"
              WHITESPACE@245..246 " "
              PATH_EXPR@246..247
                PATH@246..247
                  PATH_SEGMENT@246..247
                    NAME_REF@246..247
                      IDENT@246..247 "b"
            SEMI@247..248 ";"
          WHITESPACE@248..253 "\n    "
          R_CURLY@253..254 "}"
      WHITESPACE@254..259 "\n    "
    "#);
}

#[test]
fn assignment_operands() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        let a = b;
        a += b;
        a -= b;
        a *= b;
        a /= b;
        a %= b;
        a <<= b;
        a >>= b;
        a &= b;
        a |= b;
        a ^= b;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..208
      FUNCTION_DEF@0..203
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..203
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          LET_STMT@25..35
            LET_KW@25..28 "let"
            WHITESPACE@28..29 " "
            BIND_PAT@29..30
              NAME@29..30
                IDENT@29..30 "a"
            WHITESPACE@30..31 " "
            EQ@31..32 "="
            WHITESPACE@32..33 " "
            PATH_EXPR@33..34
              PATH@33..34
                PATH_SEGMENT@33..34
                  NAME_REF@33..34
                    IDENT@33..34 "b"
            SEMI@34..35 ";"
          WHITESPACE@35..44 "\n        "
          EXPR_STMT@44..51
            BIN_EXPR@44..50
              PATH_EXPR@44..45
                PATH@44..45
                  PATH_SEGMENT@44..45
                    NAME_REF@44..45
                      IDENT@44..45 "a"
              WHITESPACE@45..46 " "
              PLUSEQ@46..48 "+="
              WHITESPACE@48..49 " "
              PATH_EXPR@49..50
                PATH@49..50
                  PATH_SEGMENT@49..50
                    NAME_REF@49..50
                      IDENT@49..50 "b"
            SEMI@50..51 ";"
          WHITESPACE@51..60 "\n        "
          EXPR_STMT@60..67
            BIN_EXPR@60..66
              PATH_EXPR@60..61
                PATH@60..61
                  PATH_SEGMENT@60..61
                    NAME_REF@60..61
                      IDENT@60..61 "a"
              WHITESPACE@61..62 " "
              MINUSEQ@62..64 "-="
              WHITESPACE@64..65 " "
              PATH_EXPR@65..66
                PATH@65..66
                  PATH_SEGMENT@65..66
                    NAME_REF@65..66
                      IDENT@65..66 "b"
            SEMI@66..67 ";"
          WHITESPACE@67..76 "\n        "
          EXPR_STMT@76..83
            BIN_EXPR@76..82
              PATH_EXPR@76..77
                PATH@76..77
                  PATH_SEGMENT@76..77
                    NAME_REF@76..77
                      IDENT@76..77 "a"
              WHITESPACE@77..78 " "
              STAREQ@78..80 "*="
              WHITESPACE@80..81 " "
              PATH_EXPR@81..82
                PATH@81..82
                  PATH_SEGMENT@81..82
                    NAME_REF@81..82
                      IDENT@81..82 "b"
            SEMI@82..83 ";"
          WHITESPACE@83..92 "\n        "
          EXPR_STMT@92..99
            BIN_EXPR@92..98
              PATH_EXPR@92..93
                PATH@92..93
                  PATH_SEGMENT@92..93
                    NAME_REF@92..93
                      IDENT@92..93 "a"
              WHITESPACE@93..94 " "
              SLASHEQ@94..96 "/="
              WHITESPACE@96..97 " "
              PATH_EXPR@97..98
                PATH@97..98
                  PATH_SEGMENT@97..98
                    NAME_REF@97..98
                      IDENT@97..98 "b"
            SEMI@98..99 ";"
          WHITESPACE@99..108 "\n        "
          EXPR_STMT@108..115
            BIN_EXPR@108..114
              PATH_EXPR@108..109
                PATH@108..109
                  PATH_SEGMENT@108..109
                    NAME_REF@108..109
                      IDENT@108..109 "a"
              WHITESPACE@109..110 " "
              PERCENTEQ@110..112 "%="
              WHITESPACE@112..113 " "
              PATH_EXPR@113..114
                PATH@113..114
                  PATH_SEGMENT@113..114
                    NAME_REF@113..114
                      IDENT@113..114 "b"
            SEMI@114..115 ";"
          WHITESPACE@115..124 "\n        "
          EXPR_STMT@124..132
            BIN_EXPR@124..131
              PATH_EXPR@124..125
                PATH@124..125
                  PATH_SEGMENT@124..125
                    NAME_REF@124..125
                      IDENT@124..125 "a"
              WHITESPACE@125..126 " "
              SHLEQ@126..129 "<<="
              WHITESPACE@129..130 " "
              PATH_EXPR@130..131
                PATH@130..131
                  PATH_SEGMENT@130..131
                    NAME_REF@130..131
                      IDENT@130..131 "b"
            SEMI@131..132 ";"
          WHITESPACE@132..141 "\n        "
          EXPR_STMT@141..149
            BIN_EXPR@141..148
              PATH_EXPR@141..142
                PATH@141..142
                  PATH_SEGMENT@141..142
                    NAME_REF@141..142
                      IDENT@141..142 "a"
              WHITESPACE@142..143 " "
              SHREQ@143..146 ">>="
              WHITESPACE@146..147 " "
              PATH_EXPR@147..148
                PATH@147..148
                  PATH_SEGMENT@147..148
                    NAME_REF@147..148
                      IDENT@147..148 "b"
            SEMI@148..149 ";"
          WHITESPACE@149..158 "\n        "
          EXPR_STMT@158..165
            BIN_EXPR@158..164
              PATH_EXPR@158..159
                PATH@158..159
                  PATH_SEGMENT@158..159
                    NAME_REF@158..159
                      IDENT@158..159 "a"
              WHITESPACE@159..160 " "
              AMPEQ@160..162 "&="
              WHITESPACE@162..163 " "
              PATH_EXPR@163..164
                PATH@163..164
                  PATH_SEGMENT@163..164
                    NAME_REF@163..164
                      IDENT@163..164 "b"
            SEMI@164..165 ";"
          WHITESPACE@165..174 "\n        "
          EXPR_STMT@174..181
            BIN_EXPR@174..180
              PATH_EXPR@174..175
                PATH@174..175
                  PATH_SEGMENT@174..175
                    NAME_REF@174..175
                      IDENT@174..175 "a"
              WHITESPACE@175..176 " "
              PIPEEQ@176..178 "|="
              WHITESPACE@178..179 " "
              PATH_EXPR@179..180
                PATH@179..180
                  PATH_SEGMENT@179..180
                    NAME_REF@179..180
                      IDENT@179..180 "b"
            SEMI@180..181 ";"
          WHITESPACE@181..190 "\n        "
          EXPR_STMT@190..197
            BIN_EXPR@190..196
              PATH_EXPR@190..191
                PATH@190..191
                  PATH_SEGMENT@190..191
                    NAME_REF@190..191
                      IDENT@190..191 "a"
              WHITESPACE@191..192 " "
              CARETEQ@192..194 "^="
              WHITESPACE@194..195 " "
              PATH_EXPR@195..196
                PATH@195..196
                  PATH_SEGMENT@195..196
                    NAME_REF@195..196
                      IDENT@195..196 "b"
            SEMI@196..197 ";"
          WHITESPACE@197..202 "\n    "
          R_CURLY@202..203 "}"
      WHITESPACE@203..208 "\n    "
    "#);
}

#[test]
fn compare_operands() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        let _ = a == b;
        let _ = a == b;
        let _ = a != b;
        let _ = a < b;
        let _ = a > b;
        let _ = a <= b;
        let _ = a >= b;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..193
      FUNCTION_DEF@0..188
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..188
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          LET_STMT@25..40
            LET_KW@25..28 "let"
            WHITESPACE@28..29 " "
            PLACEHOLDER_PAT@29..30
              UNDERSCORE@29..30 "_"
            WHITESPACE@30..31 " "
            EQ@31..32 "="
            WHITESPACE@32..33 " "
            BIN_EXPR@33..39
              PATH_EXPR@33..34
                PATH@33..34
                  PATH_SEGMENT@33..34
                    NAME_REF@33..34
                      IDENT@33..34 "a"
              WHITESPACE@34..35 " "
              EQEQ@35..37 "=="
              WHITESPACE@37..38 " "
              PATH_EXPR@38..39
                PATH@38..39
                  PATH_SEGMENT@38..39
                    NAME_REF@38..39
                      IDENT@38..39 "b"
            SEMI@39..40 ";"
          WHITESPACE@40..49 "\n        "
          LET_STMT@49..64
            LET_KW@49..52 "let"
            WHITESPACE@52..53 " "
            PLACEHOLDER_PAT@53..54
              UNDERSCORE@53..54 "_"
            WHITESPACE@54..55 " "
            EQ@55..56 "="
            WHITESPACE@56..57 " "
            BIN_EXPR@57..63
              PATH_EXPR@57..58
                PATH@57..58
                  PATH_SEGMENT@57..58
                    NAME_REF@57..58
                      IDENT@57..58 "a"
              WHITESPACE@58..59 " "
              EQEQ@59..61 "=="
              WHITESPACE@61..62 " "
              PATH_EXPR@62..63
                PATH@62..63
                  PATH_SEGMENT@62..63
                    NAME_REF@62..63
                      IDENT@62..63 "b"
            SEMI@63..64 ";"
          WHITESPACE@64..73 "\n        "
          LET_STMT@73..88
            LET_KW@73..76 "let"
            WHITESPACE@76..77 " "
            PLACEHOLDER_PAT@77..78
              UNDERSCORE@77..78 "_"
            WHITESPACE@78..79 " "
            EQ@79..80 "="
            WHITESPACE@80..81 " "
            BIN_EXPR@81..87
              PATH_EXPR@81..82
                PATH@81..82
                  PATH_SEGMENT@81..82
                    NAME_REF@81..82
                      IDENT@81..82 "a"
              WHITESPACE@82..83 " "
              NEQ@83..85 "!="
              WHITESPACE@85..86 " "
              PATH_EXPR@86..87
                PATH@86..87
                  PATH_SEGMENT@86..87
                    NAME_REF@86..87
                      IDENT@86..87 "b"
            SEMI@87..88 ";"
          WHITESPACE@88..97 "\n        "
          LET_STMT@97..111
            LET_KW@97..100 "let"
            WHITESPACE@100..101 " "
            PLACEHOLDER_PAT@101..102
              UNDERSCORE@101..102 "_"
            WHITESPACE@102..103 " "
            EQ@103..104 "="
            WHITESPACE@104..105 " "
            BIN_EXPR@105..110
              PATH_EXPR@105..106
                PATH@105..106
                  PATH_SEGMENT@105..106
                    NAME_REF@105..106
                      IDENT@105..106 "a"
              WHITESPACE@106..107 " "
              LT@107..108 "<"
              WHITESPACE@108..109 " "
              PATH_EXPR@109..110
                PATH@109..110
                  PATH_SEGMENT@109..110
                    NAME_REF@109..110
                      IDENT@109..110 "b"
            SEMI@110..111 ";"
          WHITESPACE@111..120 "\n        "
          LET_STMT@120..134
            LET_KW@120..123 "let"
            WHITESPACE@123..124 " "
            PLACEHOLDER_PAT@124..125
              UNDERSCORE@124..125 "_"
            WHITESPACE@125..126 " "
            EQ@126..127 "="
            WHITESPACE@127..128 " "
            BIN_EXPR@128..133
              PATH_EXPR@128..129
                PATH@128..129
                  PATH_SEGMENT@128..129
                    NAME_REF@128..129
                      IDENT@128..129 "a"
              WHITESPACE@129..130 " "
              GT@130..131 ">"
              WHITESPACE@131..132 " "
              PATH_EXPR@132..133
                PATH@132..133
                  PATH_SEGMENT@132..133
                    NAME_REF@132..133
                      IDENT@132..133 "b"
            SEMI@133..134 ";"
          WHITESPACE@134..143 "\n        "
          LET_STMT@143..158
            LET_KW@143..146 "let"
            WHITESPACE@146..147 " "
            PLACEHOLDER_PAT@147..148
              UNDERSCORE@147..148 "_"
            WHITESPACE@148..149 " "
            EQ@149..150 "="
            WHITESPACE@150..151 " "
            BIN_EXPR@151..157
              PATH_EXPR@151..152
                PATH@151..152
                  PATH_SEGMENT@151..152
                    NAME_REF@151..152
                      IDENT@151..152 "a"
              WHITESPACE@152..153 " "
              LTEQ@153..155 "<="
              WHITESPACE@155..156 " "
              PATH_EXPR@156..157
                PATH@156..157
                  PATH_SEGMENT@156..157
                    NAME_REF@156..157
                      IDENT@156..157 "b"
            SEMI@157..158 ";"
          WHITESPACE@158..167 "\n        "
          LET_STMT@167..182
            LET_KW@167..170 "let"
            WHITESPACE@170..171 " "
            PLACEHOLDER_PAT@171..172
              UNDERSCORE@171..172 "_"
            WHITESPACE@172..173 " "
            EQ@173..174 "="
            WHITESPACE@174..175 " "
            BIN_EXPR@175..181
              PATH_EXPR@175..176
                PATH@175..176
                  PATH_SEGMENT@175..176
                    NAME_REF@175..176
                      IDENT@175..176 "a"
              WHITESPACE@176..177 " "
              GTEQ@177..179 ">="
              WHITESPACE@179..180 " "
              PATH_EXPR@180..181
                PATH@180..181
                  PATH_SEGMENT@180..181
                    NAME_REF@180..181
                      IDENT@180..181 "b"
            SEMI@181..182 ";"
          WHITESPACE@182..187 "\n    "
          R_CURLY@187..188 "}"
      WHITESPACE@188..193 "\n    "
    "#);
}

#[test]
fn logic_operands() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        let _ = a || b;
        let _ = a && b;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..75
      FUNCTION_DEF@0..70
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..70
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          LET_STMT@25..40
            LET_KW@25..28 "let"
            WHITESPACE@28..29 " "
            PLACEHOLDER_PAT@29..30
              UNDERSCORE@29..30 "_"
            WHITESPACE@30..31 " "
            EQ@31..32 "="
            WHITESPACE@32..33 " "
            BIN_EXPR@33..39
              PATH_EXPR@33..34
                PATH@33..34
                  PATH_SEGMENT@33..34
                    NAME_REF@33..34
                      IDENT@33..34 "a"
              WHITESPACE@34..35 " "
              PIPEPIPE@35..37 "||"
              WHITESPACE@37..38 " "
              PATH_EXPR@38..39
                PATH@38..39
                  PATH_SEGMENT@38..39
                    NAME_REF@38..39
                      IDENT@38..39 "b"
            SEMI@39..40 ";"
          WHITESPACE@40..49 "\n        "
          LET_STMT@49..64
            LET_KW@49..52 "let"
            WHITESPACE@52..53 " "
            PLACEHOLDER_PAT@53..54
              UNDERSCORE@53..54 "_"
            WHITESPACE@54..55 " "
            EQ@55..56 "="
            WHITESPACE@56..57 " "
            BIN_EXPR@57..63
              PATH_EXPR@57..58
                PATH@57..58
                  PATH_SEGMENT@57..58
                    NAME_REF@57..58
                      IDENT@57..58 "a"
              WHITESPACE@58..59 " "
              AMPAMP@59..61 "&&"
              WHITESPACE@61..62 " "
              PATH_EXPR@62..63
                PATH@62..63
                  PATH_SEGMENT@62..63
                    NAME_REF@62..63
                      IDENT@62..63 "b"
            SEMI@63..64 ";"
          WHITESPACE@64..69 "\n    "
          R_CURLY@69..70 "}"
      WHITESPACE@70..75 "\n    "
    "#);
}

#[test]
fn if_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn bar() {
        if true {};
        if true {} else {};
        if true {} else if false {} else {};
        if {true} {} else {}
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..148
      FUNCTION_DEF@0..143
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "bar"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..143
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          EXPR_STMT@24..35
            IF_EXPR@24..34
              IF_KW@24..26 "if"
              WHITESPACE@26..27 " "
              CONDITION@27..31
                LITERAL@27..31
                  TRUE_KW@27..31 "true"
              WHITESPACE@31..32 " "
              BLOCK_EXPR@32..34
                L_CURLY@32..33 "{"
                R_CURLY@33..34 "}"
            SEMI@34..35 ";"
          WHITESPACE@35..44 "\n        "
          EXPR_STMT@44..63
            IF_EXPR@44..62
              IF_KW@44..46 "if"
              WHITESPACE@46..47 " "
              CONDITION@47..51
                LITERAL@47..51
                  TRUE_KW@47..51 "true"
              WHITESPACE@51..52 " "
              BLOCK_EXPR@52..54
                L_CURLY@52..53 "{"
                R_CURLY@53..54 "}"
              WHITESPACE@54..55 " "
              ELSE_KW@55..59 "else"
              WHITESPACE@59..60 " "
              BLOCK_EXPR@60..62
                L_CURLY@60..61 "{"
                R_CURLY@61..62 "}"
            SEMI@62..63 ";"
          WHITESPACE@63..72 "\n        "
          EXPR_STMT@72..108
            IF_EXPR@72..107
              IF_KW@72..74 "if"
              WHITESPACE@74..75 " "
              CONDITION@75..79
                LITERAL@75..79
                  TRUE_KW@75..79 "true"
              WHITESPACE@79..80 " "
              BLOCK_EXPR@80..82
                L_CURLY@80..81 "{"
                R_CURLY@81..82 "}"
              WHITESPACE@82..83 " "
              ELSE_KW@83..87 "else"
              WHITESPACE@87..88 " "
              IF_EXPR@88..107
                IF_KW@88..90 "if"
                WHITESPACE@90..91 " "
                CONDITION@91..96
                  LITERAL@91..96
                    FALSE_KW@91..96 "false"
                WHITESPACE@96..97 " "
                BLOCK_EXPR@97..99
                  L_CURLY@97..98 "{"
                  R_CURLY@98..99 "}"
                WHITESPACE@99..100 " "
                ELSE_KW@100..104 "else"
                WHITESPACE@104..105 " "
                BLOCK_EXPR@105..107
                  L_CURLY@105..106 "{"
                  R_CURLY@106..107 "}"
            SEMI@107..108 ";"
          WHITESPACE@108..117 "\n        "
          IF_EXPR@117..137
            IF_KW@117..119 "if"
            WHITESPACE@119..120 " "
            CONDITION@120..126
              BLOCK_EXPR@120..126
                L_CURLY@120..121 "{"
                LITERAL@121..125
                  TRUE_KW@121..125 "true"
                R_CURLY@125..126 "}"
            WHITESPACE@126..127 " "
            BLOCK_EXPR@127..129
              L_CURLY@127..128 "{"
              R_CURLY@128..129 "}"
            WHITESPACE@129..130 " "
            ELSE_KW@130..134 "else"
            WHITESPACE@134..135 " "
            BLOCK_EXPR@135..137
              L_CURLY@135..136 "{"
              R_CURLY@136..137 "}"
          WHITESPACE@137..142 "\n    "
          R_CURLY@142..143 "}"
      WHITESPACE@143..148 "\n    "
    "#);
}

#[test]
fn block_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn bar() {
        {3}
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..38
      FUNCTION_DEF@0..33
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "bar"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..33
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          BLOCK_EXPR@24..27
            L_CURLY@24..25 "{"
            LITERAL@25..26
              INT_NUMBER@25..26 "3"
            R_CURLY@26..27 "}"
          WHITESPACE@27..32 "\n    "
          R_CURLY@32..33 "}"
      WHITESPACE@33..38 "\n    "
    "#);
}

#[test]
fn return_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        return;
        return 50;
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..61
      FUNCTION_DEF@0..56
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..56
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          EXPR_STMT@24..31
            RETURN_EXPR@24..30
              RETURN_KW@24..30 "return"
            SEMI@30..31 ";"
          WHITESPACE@31..40 "\n        "
          EXPR_STMT@40..50
            RETURN_EXPR@40..49
              RETURN_KW@40..46 "return"
              WHITESPACE@46..47 " "
              LITERAL@47..49
                INT_NUMBER@47..49 "50"
            SEMI@49..50 ";"
          WHITESPACE@50..55 "\n    "
          R_CURLY@55..56 "}"
      WHITESPACE@56..61 "\n    "
    "#);
}

#[test]
fn loop_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        loop {}
    }"#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..37
      FUNCTION_DEF@0..37
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..37
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          LOOP_EXPR@24..31
            LOOP_KW@24..28 "loop"
            WHITESPACE@28..29 " "
            BLOCK_EXPR@29..31
              L_CURLY@29..30 "{"
              R_CURLY@30..31 "}"
          WHITESPACE@31..36 "\n    "
          R_CURLY@36..37 "}"
    "#);
}

#[test]
fn break_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        break;
        if break { 3; }
        if break 4 { 3; }
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..91
      FUNCTION_DEF@0..86
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..86
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          EXPR_STMT@24..30
            BREAK_EXPR@24..29
              BREAK_KW@24..29 "break"
            SEMI@29..30 ";"
          WHITESPACE@30..39 "\n        "
          EXPR_STMT@39..54
            IF_EXPR@39..54
              IF_KW@39..41 "if"
              WHITESPACE@41..42 " "
              CONDITION@42..47
                BREAK_EXPR@42..47
                  BREAK_KW@42..47 "break"
              WHITESPACE@47..48 " "
              BLOCK_EXPR@48..54
                L_CURLY@48..49 "{"
                WHITESPACE@49..50 " "
                EXPR_STMT@50..52
                  LITERAL@50..51
                    INT_NUMBER@50..51 "3"
                  SEMI@51..52 ";"
                WHITESPACE@52..53 " "
                R_CURLY@53..54 "}"
          WHITESPACE@54..63 "\n        "
          IF_EXPR@63..80
            IF_KW@63..65 "if"
            WHITESPACE@65..66 " "
            CONDITION@66..73
              BREAK_EXPR@66..73
                BREAK_KW@66..71 "break"
                WHITESPACE@71..72 " "
                LITERAL@72..73
                  INT_NUMBER@72..73 "4"
            WHITESPACE@73..74 " "
            BLOCK_EXPR@74..80
              L_CURLY@74..75 "{"
              WHITESPACE@75..76 " "
              EXPR_STMT@76..78
                LITERAL@76..77
                  INT_NUMBER@76..77 "3"
                SEMI@77..78 ";"
              WHITESPACE@78..79 " "
              R_CURLY@79..80 "}"
          WHITESPACE@80..85 "\n    "
          R_CURLY@85..86 "}"
      WHITESPACE@86..91 "\n    "
    "#);
}

#[test]
fn while_expr() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        while true {};
        while { true } {};
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..76
      FUNCTION_DEF@0..71
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..71
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          EXPR_STMT@24..38
            WHILE_EXPR@24..37
              WHILE_KW@24..29 "while"
              WHITESPACE@29..30 " "
              CONDITION@30..34
                LITERAL@30..34
                  TRUE_KW@30..34 "true"
              WHITESPACE@34..35 " "
              BLOCK_EXPR@35..37
                L_CURLY@35..36 "{"
                R_CURLY@36..37 "}"
            SEMI@37..38 ";"
          WHITESPACE@38..47 "\n        "
          EXPR_STMT@47..65
            WHILE_EXPR@47..64
              WHILE_KW@47..52 "while"
              WHITESPACE@52..53 " "
              CONDITION@53..61
                BLOCK_EXPR@53..61
                  L_CURLY@53..54 "{"
                  WHITESPACE@54..55 " "
                  LITERAL@55..59
                    TRUE_KW@55..59 "true"
                  WHITESPACE@59..60 " "
                  R_CURLY@60..61 "}"
              WHITESPACE@61..62 " "
              BLOCK_EXPR@62..64
                L_CURLY@62..63 "{"
                R_CURLY@63..64 "}"
            SEMI@64..65 ";"
          WHITESPACE@65..70 "\n    "
          R_CURLY@70..71 "}"
      WHITESPACE@71..76 "\n    "
    "#);
}

#[test]
fn struct_lit() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn foo() {
        U;
        S {};
        S { x, y: 32, };
        S { x: 32, y: 64 };
        TupleStruct { 0: 1 };
        T(1.23);
        T(1.23, 4,)
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..171
      FUNCTION_DEF@0..166
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..11
          IDENT@8..11 "foo"
        PARAM_LIST@11..13
          L_PAREN@11..12 "("
          R_PAREN@12..13 ")"
        WHITESPACE@13..14 " "
        BLOCK_EXPR@14..166
          L_CURLY@14..15 "{"
          WHITESPACE@15..24 "\n        "
          EXPR_STMT@24..26
            PATH_EXPR@24..25
              PATH@24..25
                PATH_SEGMENT@24..25
                  NAME_REF@24..25
                    IDENT@24..25 "U"
            SEMI@25..26 ";"
          WHITESPACE@26..35 "\n        "
          EXPR_STMT@35..40
            RECORD_LIT@35..39
              PATH_TYPE@35..36
                PATH@35..36
                  PATH_SEGMENT@35..36
                    NAME_REF@35..36
                      IDENT@35..36 "S"
              WHITESPACE@36..37 " "
              RECORD_FIELD_LIST@37..39
                L_CURLY@37..38 "{"
                R_CURLY@38..39 "}"
            SEMI@39..40 ";"
          WHITESPACE@40..49 "\n        "
          EXPR_STMT@49..65
            RECORD_LIT@49..64
              PATH_TYPE@49..50
                PATH@49..50
                  PATH_SEGMENT@49..50
                    NAME_REF@49..50
                      IDENT@49..50 "S"
              WHITESPACE@50..51 " "
              RECORD_FIELD_LIST@51..64
                L_CURLY@51..52 "{"
                WHITESPACE@52..53 " "
                RECORD_FIELD@53..54
                  NAME_REF@53..54
                    IDENT@53..54 "x"
                COMMA@54..55 ","
                WHITESPACE@55..56 " "
                RECORD_FIELD@56..61
                  NAME_REF@56..57
                    IDENT@56..57 "y"
                  COLON@57..58 ":"
                  WHITESPACE@58..59 " "
                  LITERAL@59..61
                    INT_NUMBER@59..61 "32"
                COMMA@61..62 ","
                WHITESPACE@62..63 " "
                R_CURLY@63..64 "}"
            SEMI@64..65 ";"
          WHITESPACE@65..74 "\n        "
          EXPR_STMT@74..93
            RECORD_LIT@74..92
              PATH_TYPE@74..75
                PATH@74..75
                  PATH_SEGMENT@74..75
                    NAME_REF@74..75
                      IDENT@74..75 "S"
              WHITESPACE@75..76 " "
              RECORD_FIELD_LIST@76..92
                L_CURLY@76..77 "{"
                WHITESPACE@77..78 " "
                RECORD_FIELD@78..83
                  NAME_REF@78..79
                    IDENT@78..79 "x"
                  COLON@79..80 ":"
                  WHITESPACE@80..81 " "
                  LITERAL@81..83
                    INT_NUMBER@81..83 "32"
                COMMA@83..84 ","
                WHITESPACE@84..85 " "
                RECORD_FIELD@85..90
                  NAME_REF@85..86
                    IDENT@85..86 "y"
                  COLON@86..87 ":"
                  WHITESPACE@87..88 " "
                  LITERAL@88..90
                    INT_NUMBER@88..90 "64"
                WHITESPACE@90..91 " "
                R_CURLY@91..92 "}"
            SEMI@92..93 ";"
          WHITESPACE@93..102 "\n        "
          EXPR_STMT@102..123
            RECORD_LIT@102..122
              PATH_TYPE@102..113
                PATH@102..113
                  PATH_SEGMENT@102..113
                    NAME_REF@102..113
                      IDENT@102..113 "TupleStruct"
              WHITESPACE@113..114 " "
              RECORD_FIELD_LIST@114..122
                L_CURLY@114..115 "{"
                WHITESPACE@115..116 " "
                RECORD_FIELD@116..120
                  NAME_REF@116..117
                    INT_NUMBER@116..117 "0"
                  COLON@117..118 ":"
                  WHITESPACE@118..119 " "
                  LITERAL@119..120
                    INT_NUMBER@119..120 "1"
                WHITESPACE@120..121 " "
                R_CURLY@121..122 "}"
            SEMI@122..123 ";"
          WHITESPACE@123..132 "\n        "
          EXPR_STMT@132..140
            CALL_EXPR@132..139
              PATH_EXPR@132..133
                PATH@132..133
                  PATH_SEGMENT@132..133
                    NAME_REF@132..133
                      IDENT@132..133 "T"
              ARG_LIST@133..139
                L_PAREN@133..134 "("
                LITERAL@134..138
                  FLOAT_NUMBER@134..138 "1.23"
                R_PAREN@138..139 ")"
            SEMI@139..140 ";"
          WHITESPACE@140..149 "\n        "
          CALL_EXPR@149..160
            PATH_EXPR@149..150
              PATH@149..150
                PATH_SEGMENT@149..150
                  NAME_REF@149..150
                    IDENT@149..150 "T"
            ARG_LIST@150..160
              L_PAREN@150..151 "("
              LITERAL@151..155
                FLOAT_NUMBER@151..155 "1.23"
              COMMA@155..156 ","
              WHITESPACE@156..157 " "
              LITERAL@157..158
                INT_NUMBER@157..158 "4"
              COMMA@158..159 ","
              R_PAREN@159..160 ")"
          WHITESPACE@160..165 "\n    "
          R_CURLY@165..166 "}"
      WHITESPACE@166..171 "\n    "
    "#);
}

#[test]
fn struct_field_index() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    fn main() {
        foo.a
        foo.a.b
        foo.0
        foo.0.1
        foo.10
        foo.01  // index: .0
        foo.0 1 // index: .0 
        foo.a.0
    }
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..177
      FUNCTION_DEF@0..172
        WHITESPACE@0..5 "\n    "
        FN_KW@5..7 "fn"
        WHITESPACE@7..8 " "
        NAME@8..12
          IDENT@8..12 "main"
        PARAM_LIST@12..14
          L_PAREN@12..13 "("
          R_PAREN@13..14 ")"
        WHITESPACE@14..15 " "
        BLOCK_EXPR@15..172
          L_CURLY@15..16 "{"
          WHITESPACE@16..25 "\n        "
          EXPR_STMT@25..30
            FIELD_EXPR@25..30
              PATH_EXPR@25..28
                PATH@25..28
                  PATH_SEGMENT@25..28
                    NAME_REF@25..28
                      IDENT@25..28 "foo"
              DOT@28..29 "."
              NAME_REF@29..30
                IDENT@29..30 "a"
          WHITESPACE@30..39 "\n        "
          EXPR_STMT@39..46
            FIELD_EXPR@39..46
              FIELD_EXPR@39..44
                PATH_EXPR@39..42
                  PATH@39..42
                    PATH_SEGMENT@39..42
                      NAME_REF@39..42
                        IDENT@39..42 "foo"
                DOT@42..43 "."
                NAME_REF@43..44
                  IDENT@43..44 "a"
              DOT@44..45 "."
              NAME_REF@45..46
                IDENT@45..46 "b"
          WHITESPACE@46..55 "\n        "
          EXPR_STMT@55..60
            FIELD_EXPR@55..60
              PATH_EXPR@55..58
                PATH@55..58
                  PATH_SEGMENT@55..58
                    NAME_REF@55..58
                      IDENT@55..58 "foo"
              INDEX@58..60 ".0"
          WHITESPACE@60..69 "\n        "
          EXPR_STMT@69..76
            FIELD_EXPR@69..76
              FIELD_EXPR@69..74
                PATH_EXPR@69..72
                  PATH@69..72
                    PATH_SEGMENT@69..72
                      NAME_REF@69..72
                        IDENT@69..72 "foo"
                INDEX@72..74 ".0"
              INDEX@74..76 ".1"
          WHITESPACE@76..85 "\n        "
          EXPR_STMT@85..91
            FIELD_EXPR@85..91
              PATH_EXPR@85..88
                PATH@85..88
                  PATH_SEGMENT@85..88
                    NAME_REF@85..88
                      IDENT@85..88 "foo"
              INDEX@88..91 ".10"
          WHITESPACE@91..100 "\n        "
          EXPR_STMT@100..105
            FIELD_EXPR@100..105
              PATH_EXPR@100..103
                PATH@100..103
                  PATH_SEGMENT@100..103
                    NAME_REF@100..103
                      IDENT@100..103 "foo"
              INDEX@103..105 ".0"
          EXPR_STMT@105..106
            LITERAL@105..106
              INT_NUMBER@105..106 "1"
          WHITESPACE@106..108 "  "
          COMMENT@108..120 "// index: .0"
          WHITESPACE@120..129 "\n        "
          EXPR_STMT@129..134
            FIELD_EXPR@129..134
              PATH_EXPR@129..132
                PATH@129..132
                  PATH_SEGMENT@129..132
                    NAME_REF@129..132
                      IDENT@129..132 "foo"
              INDEX@132..134 ".0"
          WHITESPACE@134..135 " "
          EXPR_STMT@135..136
            LITERAL@135..136
              INT_NUMBER@135..136 "1"
          WHITESPACE@136..137 " "
          COMMENT@137..150 "// index: .0 "
          WHITESPACE@150..159 "\n        "
          FIELD_EXPR@159..166
            FIELD_EXPR@159..164
              PATH_EXPR@159..162
                PATH@159..162
                  PATH_SEGMENT@159..162
                    NAME_REF@159..162
                      IDENT@159..162 "foo"
              DOT@162..163 "."
              NAME_REF@163..164
                IDENT@163..164 "a"
            INDEX@164..166 ".0"
          WHITESPACE@166..171 "\n    "
          R_CURLY@171..172 "}"
      WHITESPACE@172..177 "\n    "
    "#);
}

#[test]
fn memory_type_specifier() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    struct Foo {};
    struct(gc) Baz {};
    struct(value) Baz {};
    struct() Err1 {};    // error: expected memory type specifier
    struct(foo) Err2 {}; // error: expected memory type specifier
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..205
      WHITESPACE@0..5 "\n    "
      STRUCT_DEF@5..19
        STRUCT_KW@5..11 "struct"
        WHITESPACE@11..12 " "
        NAME@12..15
          IDENT@12..15 "Foo"
        WHITESPACE@15..16 " "
        RECORD_FIELD_DEF_LIST@16..19
          L_CURLY@16..17 "{"
          R_CURLY@17..18 "}"
          SEMI@18..19 ";"
      WHITESPACE@19..24 "\n    "
      STRUCT_DEF@24..42
        STRUCT_KW@24..30 "struct"
        MEMORY_TYPE_SPECIFIER@30..34
          L_PAREN@30..31 "("
          GC_KW@31..33 "gc"
          R_PAREN@33..34 ")"
        WHITESPACE@34..35 " "
        NAME@35..38
          IDENT@35..38 "Baz"
        WHITESPACE@38..39 " "
        RECORD_FIELD_DEF_LIST@39..42
          L_CURLY@39..40 "{"
          R_CURLY@40..41 "}"
          SEMI@41..42 ";"
      WHITESPACE@42..47 "\n    "
      STRUCT_DEF@47..68
        STRUCT_KW@47..53 "struct"
        MEMORY_TYPE_SPECIFIER@53..60
          L_PAREN@53..54 "("
          VALUE_KW@54..59 "value"
          R_PAREN@59..60 ")"
        WHITESPACE@60..61 " "
        NAME@61..64
          IDENT@61..64 "Baz"
        WHITESPACE@64..65 " "
        RECORD_FIELD_DEF_LIST@65..68
          L_CURLY@65..66 "{"
          R_CURLY@66..67 "}"
          SEMI@67..68 ";"
      WHITESPACE@68..73 "\n    "
      STRUCT_DEF@73..90
        STRUCT_KW@73..79 "struct"
        MEMORY_TYPE_SPECIFIER@79..81
          L_PAREN@79..80 "("
          R_PAREN@80..81 ")"
        WHITESPACE@81..82 " "
        NAME@82..86
          IDENT@82..86 "Err1"
        WHITESPACE@86..87 " "
        RECORD_FIELD_DEF_LIST@87..90
          L_CURLY@87..88 "{"
          R_CURLY@88..89 "}"
          SEMI@89..90 ";"
      WHITESPACE@90..94 "    "
      COMMENT@94..134 "// error: expected me ..."
      WHITESPACE@134..139 "\n    "
      STRUCT_DEF@139..159
        STRUCT_KW@139..145 "struct"
        MEMORY_TYPE_SPECIFIER@145..150
          L_PAREN@145..146 "("
          ERROR@146..149
            IDENT@146..149 "foo"
          R_PAREN@149..150 ")"
        WHITESPACE@150..151 " "
        NAME@151..155
          IDENT@151..155 "Err2"
        WHITESPACE@155..156 " "
        RECORD_FIELD_DEF_LIST@156..159
          L_CURLY@156..157 "{"
          R_CURLY@157..158 "}"
          SEMI@158..159 ";"
      WHITESPACE@159..160 " "
      COMMENT@160..200 "// error: expected me ..."
      WHITESPACE@200..205 "\n    "
    error Offset(80): expected memory type specifier
    error Offset(146): expected memory type specifier
    "#);
}

#[test]
fn visibility() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    pub struct Foo {};
    pub(package) struct(gc) Baz {};
    pub(super) fn foo() {}
    pub(package) fn bar() {}
    pub fn baz() {}
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..140
      WHITESPACE@0..5 "\n    "
      STRUCT_DEF@5..23
        VISIBILITY@5..8
          PUB_KW@5..8 "pub"
        WHITESPACE@8..9 " "
        STRUCT_KW@9..15 "struct"
        WHITESPACE@15..16 " "
        NAME@16..19
          IDENT@16..19 "Foo"
        WHITESPACE@19..20 " "
        RECORD_FIELD_DEF_LIST@20..23
          L_CURLY@20..21 "{"
          R_CURLY@21..22 "}"
          SEMI@22..23 ";"
      WHITESPACE@23..28 "\n    "
      STRUCT_DEF@28..59
        VISIBILITY@28..40
          PUB_KW@28..31 "pub"
          L_PAREN@31..32 "("
          PACKAGE_KW@32..39 "package"
          R_PAREN@39..40 ")"
        WHITESPACE@40..41 " "
        STRUCT_KW@41..47 "struct"
        MEMORY_TYPE_SPECIFIER@47..51
          L_PAREN@47..48 "("
          GC_KW@48..50 "gc"
          R_PAREN@50..51 ")"
        WHITESPACE@51..52 " "
        NAME@52..55
          IDENT@52..55 "Baz"
        WHITESPACE@55..56 " "
        RECORD_FIELD_DEF_LIST@56..59
          L_CURLY@56..57 "{"
          R_CURLY@57..58 "}"
          SEMI@58..59 ";"
      FUNCTION_DEF@59..86
        WHITESPACE@59..64 "\n    "
        VISIBILITY@64..74
          PUB_KW@64..67 "pub"
          L_PAREN@67..68 "("
          SUPER_KW@68..73 "super"
          R_PAREN@73..74 ")"
        WHITESPACE@74..75 " "
        FN_KW@75..77 "fn"
        WHITESPACE@77..78 " "
        NAME@78..81
          IDENT@78..81 "foo"
        PARAM_LIST@81..83
          L_PAREN@81..82 "("
          R_PAREN@82..83 ")"
        WHITESPACE@83..84 " "
        BLOCK_EXPR@84..86
          L_CURLY@84..85 "{"
          R_CURLY@85..86 "}"
      FUNCTION_DEF@86..115
        WHITESPACE@86..91 "\n    "
        VISIBILITY@91..103
          PUB_KW@91..94 "pub"
          L_PAREN@94..95 "("
          PACKAGE_KW@95..102 "package"
          R_PAREN@102..103 ")"
        WHITESPACE@103..104 " "
        FN_KW@104..106 "fn"
        WHITESPACE@106..107 " "
        NAME@107..110
          IDENT@107..110 "bar"
        PARAM_LIST@110..112
          L_PAREN@110..111 "("
          R_PAREN@111..112 ")"
        WHITESPACE@112..113 " "
        BLOCK_EXPR@113..115
          L_CURLY@113..114 "{"
          R_CURLY@114..115 "}"
      FUNCTION_DEF@115..135
        WHITESPACE@115..120 "\n    "
        VISIBILITY@120..123
          PUB_KW@120..123 "pub"
        WHITESPACE@123..124 " "
        FN_KW@124..126 "fn"
        WHITESPACE@126..127 " "
        NAME@127..130
          IDENT@127..130 "baz"
        PARAM_LIST@130..132
          L_PAREN@130..131 "("
          R_PAREN@131..132 ")"
        WHITESPACE@132..133 " "
        BLOCK_EXPR@133..135
          L_CURLY@133..134 "{"
          R_CURLY@134..135 "}"
      WHITESPACE@135..140 "\n    "
    "#);
}

#[test]
fn extern_fn() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    pub extern fn foo();
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..30
      FUNCTION_DEF@0..25
        WHITESPACE@0..5 "\n    "
        VISIBILITY@5..8
          PUB_KW@5..8 "pub"
        WHITESPACE@8..9 " "
        EXTERN@9..15
          EXTERN_KW@9..15 "extern"
        WHITESPACE@15..16 " "
        FN_KW@16..18 "fn"
        WHITESPACE@18..19 " "
        NAME@19..22
          IDENT@19..22 "foo"
        PARAM_LIST@22..24
          L_PAREN@22..23 "("
          R_PAREN@23..24 ")"
        SEMI@24..25 ";"
      WHITESPACE@25..30 "\n    "
    "#);
}

#[test]
fn type_alias_def() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
    type Foo = i32;
    type Bar = Foo;
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..45
      WHITESPACE@0..5 "\n    "
      TYPE_ALIAS_DEF@5..20
        TYPE_KW@5..9 "type"
        WHITESPACE@9..10 " "
        NAME@10..13
          IDENT@10..13 "Foo"
        WHITESPACE@13..14 " "
        EQ@14..15 "="
        WHITESPACE@15..16 " "
        PATH_TYPE@16..19
          PATH@16..19
            PATH_SEGMENT@16..19
              NAME_REF@16..19
                IDENT@16..19 "i32"
        SEMI@19..20 ";"
      WHITESPACE@20..25 "\n    "
      TYPE_ALIAS_DEF@25..40
        TYPE_KW@25..29 "type"
        WHITESPACE@29..30 " "
        NAME@30..33
          IDENT@30..33 "Bar"
        WHITESPACE@33..34 " "
        EQ@34..35 "="
        WHITESPACE@35..36 " "
        PATH_TYPE@36..39
          PATH@36..39
            PATH_SEGMENT@36..39
              NAME_REF@36..39
                IDENT@36..39 "Foo"
        SEMI@39..40 ";"
      WHITESPACE@40..45 "\n    "
    "#);
}

#[test]
fn function_return_path() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
        fn main() -> self::Foo {}
        fn main1() -> super::Foo {}
        fn main2() -> package::Foo {}
        fn main3() -> package::foo::Foo {}
    "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..156
      FUNCTION_DEF@0..34
        WHITESPACE@0..9 "\n        "
        FN_KW@9..11 "fn"
        WHITESPACE@11..12 " "
        NAME@12..16
          IDENT@12..16 "main"
        PARAM_LIST@16..18
          L_PAREN@16..17 "("
          R_PAREN@17..18 ")"
        WHITESPACE@18..19 " "
        RET_TYPE@19..31
          THIN_ARROW@19..21 "->"
          WHITESPACE@21..22 " "
          PATH_TYPE@22..31
            PATH@22..31
              PATH@22..26
                PATH_SEGMENT@22..26
                  SELF_KW@22..26 "self"
              COLONCOLON@26..28 "::"
              PATH_SEGMENT@28..31
                NAME_REF@28..31
                  IDENT@28..31 "Foo"
        WHITESPACE@31..32 " "
        BLOCK_EXPR@32..34
          L_CURLY@32..33 "{"
          R_CURLY@33..34 "}"
      FUNCTION_DEF@34..70
        WHITESPACE@34..43 "\n        "
        FN_KW@43..45 "fn"
        WHITESPACE@45..46 " "
        NAME@46..51
          IDENT@46..51 "main1"
        PARAM_LIST@51..53
          L_PAREN@51..52 "("
          R_PAREN@52..53 ")"
        WHITESPACE@53..54 " "
        RET_TYPE@54..67
          THIN_ARROW@54..56 "->"
          WHITESPACE@56..57 " "
          PATH_TYPE@57..67
            PATH@57..67
              PATH@57..62
                PATH_SEGMENT@57..62
                  SUPER_KW@57..62 "super"
              COLONCOLON@62..64 "::"
              PATH_SEGMENT@64..67
                NAME_REF@64..67
                  IDENT@64..67 "Foo"
        WHITESPACE@67..68 " "
        BLOCK_EXPR@68..70
          L_CURLY@68..69 "{"
          R_CURLY@69..70 "}"
      FUNCTION_DEF@70..108
        WHITESPACE@70..79 "\n        "
        FN_KW@79..81 "fn"
        WHITESPACE@81..82 " "
        NAME@82..87
          IDENT@82..87 "main2"
        PARAM_LIST@87..89
          L_PAREN@87..88 "("
          R_PAREN@88..89 ")"
        WHITESPACE@89..90 " "
        RET_TYPE@90..105
          THIN_ARROW@90..92 "->"
          WHITESPACE@92..93 " "
          PATH_TYPE@93..105
            PATH@93..105
              PATH@93..100
                PATH_SEGMENT@93..100
                  PACKAGE_KW@93..100 "package"
              COLONCOLON@100..102 "::"
              PATH_SEGMENT@102..105
                NAME_REF@102..105
                  IDENT@102..105 "Foo"
        WHITESPACE@105..106 " "
        BLOCK_EXPR@106..108
          L_CURLY@106..107 "{"
          R_CURLY@107..108 "}"
      FUNCTION_DEF@108..151
        WHITESPACE@108..117 "\n        "
        FN_KW@117..119 "fn"
        WHITESPACE@119..120 " "
        NAME@120..125
          IDENT@120..125 "main3"
        PARAM_LIST@125..127
          L_PAREN@125..126 "("
          R_PAREN@126..127 ")"
        WHITESPACE@127..128 " "
        RET_TYPE@128..148
          THIN_ARROW@128..130 "->"
          WHITESPACE@130..131 " "
          PATH_TYPE@131..148
            PATH@131..148
              PATH@131..143
                PATH@131..138
                  PATH_SEGMENT@131..138
                    PACKAGE_KW@131..138 "package"
                COLONCOLON@138..140 "::"
                PATH_SEGMENT@140..143
                  NAME_REF@140..143
                    IDENT@140..143 "foo"
              COLONCOLON@143..145 "::"
              PATH_SEGMENT@145..148
                NAME_REF@145..148
                  IDENT@145..148 "Foo"
        WHITESPACE@148..149 " "
        BLOCK_EXPR@149..151
          L_CURLY@149..150 "{"
          R_CURLY@150..151 "}"
      WHITESPACE@151..156 "\n    "
    "#);
}

#[test]
fn use_() {
    insta::assert_snapshot!(SourceFile::parse(
        r#"
        // Simple paths
        use package_name;
        use self::item_in_scope_or_package_name;
        use self::module::Item;
        use package::Item;
        use self::some::Struct;
        use package::some_item;

        // Use tree list
        use crate::{Item};
        use self::{Item};

        // Wildcard import
        use *; // Error
        use ::*; // Error
        use crate::*;
        use crate::{*};

        // Renames
        use some::path as some_name;
        use some::{
            other::path as some_other_name,
            different::path as different_name,
            yet::another::path,
            running::out::of::synonyms::for_::different::*
        };
        use Foo as _;
        "#,
    ).debug_dump(), @r#"
    SOURCE_FILE@0..726
      WHITESPACE@0..9 "\n        "
      COMMENT@9..24 "// Simple paths"
      WHITESPACE@24..33 "\n        "
      USE@33..50
        USE_KW@33..36 "use"
        WHITESPACE@36..37 " "
        USE_TREE@37..49
          PATH@37..49
            PATH_SEGMENT@37..49
              NAME_REF@37..49
                IDENT@37..49 "package_name"
        SEMI@49..50 ";"
      WHITESPACE@50..59 "\n        "
      USE@59..99
        USE_KW@59..62 "use"
        WHITESPACE@62..63 " "
        USE_TREE@63..98
          PATH@63..98
            PATH@63..67
              PATH_SEGMENT@63..67
                SELF_KW@63..67 "self"
            COLONCOLON@67..69 "::"
            PATH_SEGMENT@69..98
              NAME_REF@69..98
                IDENT@69..98 "item_in_scope_or_pack ..."
        SEMI@98..99 ";"
      WHITESPACE@99..108 "\n        "
      USE@108..131
        USE_KW@108..111 "use"
        WHITESPACE@111..112 " "
        USE_TREE@112..130
          PATH@112..130
            PATH@112..124
              PATH@112..116
                PATH_SEGMENT@112..116
                  SELF_KW@112..116 "self"
              COLONCOLON@116..118 "::"
              PATH_SEGMENT@118..124
                NAME_REF@118..124
                  IDENT@118..124 "module"
            COLONCOLON@124..126 "::"
            PATH_SEGMENT@126..130
              NAME_REF@126..130
                IDENT@126..130 "Item"
        SEMI@130..131 ";"
      WHITESPACE@131..140 "\n        "
      USE@140..158
        USE_KW@140..143 "use"
        WHITESPACE@143..144 " "
        USE_TREE@144..157
          PATH@144..157
            PATH@144..151
              PATH_SEGMENT@144..151
                PACKAGE_KW@144..151 "package"
            COLONCOLON@151..153 "::"
            PATH_SEGMENT@153..157
              NAME_REF@153..157
                IDENT@153..157 "Item"
        SEMI@157..158 ";"
      WHITESPACE@158..167 "\n        "
      USE@167..190
        USE_KW@167..170 "use"
        WHITESPACE@170..171 " "
        USE_TREE@171..189
          PATH@171..189
            PATH@171..181
              PATH@171..175
                PATH_SEGMENT@171..175
                  SELF_KW@171..175 "self"
              COLONCOLON@175..177 "::"
              PATH_SEGMENT@177..181
                NAME_REF@177..181
                  IDENT@177..181 "some"
            COLONCOLON@181..183 "::"
            PATH_SEGMENT@183..189
              NAME_REF@183..189
                IDENT@183..189 "Struct"
        SEMI@189..190 ";"
      WHITESPACE@190..199 "\n        "
      USE@199..222
        USE_KW@199..202 "use"
        WHITESPACE@202..203 " "
        USE_TREE@203..221
          PATH@203..221
            PATH@203..210
              PATH_SEGMENT@203..210
                PACKAGE_KW@203..210 "package"
            COLONCOLON@210..212 "::"
            PATH_SEGMENT@212..221
              NAME_REF@212..221
                IDENT@212..221 "some_item"
        SEMI@221..222 ";"
      WHITESPACE@222..232 "\n\n        "
      COMMENT@232..248 "// Use tree list"
      WHITESPACE@248..257 "\n        "
      USE@257..275
        USE_KW@257..260 "use"
        WHITESPACE@260..261 " "
        USE_TREE@261..274
          PATH@261..266
            PATH_SEGMENT@261..266
              NAME_REF@261..266
                IDENT@261..266 "crate"
          COLONCOLON@266..268 "::"
          USE_TREE_LIST@268..274
            L_CURLY@268..269 "{"
            USE_TREE@269..273
              PATH@269..273
                PATH_SEGMENT@269..273
                  NAME_REF@269..273
                    IDENT@269..273 "Item"
            R_CURLY@273..274 "}"
        SEMI@274..275 ";"
      WHITESPACE@275..284 "\n        "
      USE@284..301
        USE_KW@284..287 "use"
        WHITESPACE@287..288 " "
        USE_TREE@288..300
          PATH@288..292
            PATH_SEGMENT@288..292
              SELF_KW@288..292 "self"
          COLONCOLON@292..294 "::"
          USE_TREE_LIST@294..300
            L_CURLY@294..295 "{"
            USE_TREE@295..299
              PATH@295..299
                PATH_SEGMENT@295..299
                  NAME_REF@295..299
                    IDENT@295..299 "Item"
            R_CURLY@299..300 "}"
        SEMI@300..301 ";"
      WHITESPACE@301..311 "\n\n        "
      COMMENT@311..329 "// Wildcard import"
      WHITESPACE@329..338 "\n        "
      USE@338..344
        USE_KW@338..341 "use"
        WHITESPACE@341..342 " "
        ERROR@342..343
          STAR@342..343 "*"
        SEMI@343..344 ";"
      WHITESPACE@344..345 " "
      COMMENT@345..353 "// Error"
      WHITESPACE@353..362 "\n        "
      USE@362..367
        USE_KW@362..365 "use"
        WHITESPACE@365..366 " "
        ERROR@366..367
          COLON@366..367 ":"
      ERROR@367..368
        COLON@367..368 ":"
      ERROR@368..369
        STAR@368..369 "*"
      ERROR@369..370
        SEMI@369..370 ";"
      WHITESPACE@370..371 " "
      COMMENT@371..379 "// Error"
      WHITESPACE@379..388 "\n        "
      USE@388..401
        USE_KW@388..391 "use"
        WHITESPACE@391..392 " "
        USE_TREE@392..400
          PATH@392..397
            PATH_SEGMENT@392..397
              NAME_REF@392..397
                IDENT@392..397 "crate"
          COLONCOLON@397..399 "::"
          STAR@399..400 "*"
        SEMI@400..401 ";"
      WHITESPACE@401..410 "\n        "
      USE@410..425
        USE_KW@410..413 "use"
        WHITESPACE@413..414 " "
        USE_TREE@414..424
          PATH@414..419
            PATH_SEGMENT@414..419
              NAME_REF@414..419
                IDENT@414..419 "crate"
          COLONCOLON@419..421 "::"
          USE_TREE_LIST@421..424
            L_CURLY@421..422 "{"
            USE_TREE@422..423
              STAR@422..423 "*"
            R_CURLY@423..424 "}"
        SEMI@424..425 ";"
      WHITESPACE@425..435 "\n\n        "
      COMMENT@435..445 "// Renames"
      WHITESPACE@445..454 "\n        "
      USE@454..482
        USE_KW@454..457 "use"
        WHITESPACE@457..458 " "
        USE_TREE@458..481
          PATH@458..468
            PATH@458..462
              PATH_SEGMENT@458..462
                NAME_REF@458..462
                  IDENT@458..462 "some"
            COLONCOLON@462..464 "::"
            PATH_SEGMENT@464..468
              NAME_REF@464..468
                IDENT@464..468 "path"
          WHITESPACE@468..469 " "
          RENAME@469..481
            AS_KW@469..471 "as"
            WHITESPACE@471..472 " "
            NAME@472..481
              IDENT@472..481 "some_name"
        SEMI@481..482 ";"
      WHITESPACE@482..491 "\n        "
      USE@491..695
        USE_KW@491..494 "use"
        WHITESPACE@494..495 " "
        USE_TREE@495..694
          PATH@495..499
            PATH_SEGMENT@495..499
              NAME_REF@495..499
                IDENT@495..499 "some"
          COLONCOLON@499..501 "::"
          USE_TREE_LIST@501..694
            L_CURLY@501..502 "{"
            WHITESPACE@502..515 "\n            "
            USE_TREE@515..545
              PATH@515..526
                PATH@515..520
                  PATH_SEGMENT@515..520
                    NAME_REF@515..520
                      IDENT@515..520 "other"
                COLONCOLON@520..522 "::"
                PATH_SEGMENT@522..526
                  NAME_REF@522..526
                    IDENT@522..526 "path"
              WHITESPACE@526..527 " "
              RENAME@527..545
                AS_KW@527..529 "as"
                WHITESPACE@529..530 " "
                NAME@530..545
                  IDENT@530..545 "some_other_name"
            COMMA@545..546 ","
            WHITESPACE@546..559 "\n            "
            USE_TREE@559..592
              PATH@559..574
                PATH@559..568
                  PATH_SEGMENT@559..568
                    NAME_REF@559..568
                      IDENT@559..568 "different"
                COLONCOLON@568..570 "::"
                PATH_SEGMENT@570..574
                  NAME_REF@570..574
                    IDENT@570..574 "path"
              WHITESPACE@574..575 " "
              RENAME@575..592
                AS_KW@575..577 "as"
                WHITESPACE@577..578 " "
                NAME@578..592
                  IDENT@578..592 "different_name"
            COMMA@592..593 ","
            WHITESPACE@593..606 "\n            "
            USE_TREE@606..624
              PATH@606..624
                PATH@606..618
                  PATH@606..609
                    PATH_SEGMENT@606..609
                      NAME_REF@606..609
                        IDENT@606..609 "yet"
                  COLONCOLON@609..611 "::"
                  PATH_SEGMENT@611..618
                    NAME_REF@611..618
                      IDENT@611..618 "another"
                COLONCOLON@618..620 "::"
                PATH_SEGMENT@620..624
                  NAME_REF@620..624
                    IDENT@620..624 "path"
            COMMA@624..625 ","
            WHITESPACE@625..638 "\n            "
            USE_TREE@638..684
              PATH@638..681
                PATH@638..670
                  PATH@638..664
                    PATH@638..654
                      PATH@638..650
                        PATH@638..645
                          PATH_SEGMENT@638..645
                            NAME_REF@638..645
                              IDENT@638..645 "running"
                        COLONCOLON@645..647 "::"
                        PATH_SEGMENT@647..650
                          NAME_REF@647..650
                            IDENT@647..650 "out"
                      COLONCOLON@650..652 "::"
                      PATH_SEGMENT@652..654
                        NAME_REF@652..654
                          IDENT@652..654 "of"
                    COLONCOLON@654..656 "::"
                    PATH_SEGMENT@656..664
                      NAME_REF@656..664
                        IDENT@656..664 "synonyms"
                  COLONCOLON@664..666 "::"
                  PATH_SEGMENT@666..670
                    NAME_REF@666..670
                      IDENT@666..670 "for_"
                COLONCOLON@670..672 "::"
                PATH_SEGMENT@672..681
                  NAME_REF@672..681
                    IDENT@672..681 "different"
              COLONCOLON@681..683 "::"
              STAR@683..684 "*"
            WHITESPACE@684..693 "\n        "
            R_CURLY@693..694 "}"
        SEMI@694..695 ";"
      WHITESPACE@695..704 "\n        "
      USE@704..717
        USE_KW@704..707 "use"
        WHITESPACE@707..708 " "
        USE_TREE@708..716
          PATH@708..711
            PATH_SEGMENT@708..711
              NAME_REF@708..711
                IDENT@708..711 "Foo"
          WHITESPACE@711..712 " "
          RENAME@712..716
            AS_KW@712..714 "as"
            WHITESPACE@714..715 " "
            UNDERSCORE@715..716 "_"
        SEMI@716..717 ";"
      WHITESPACE@717..726 "\n        "
    error Offset(342): expected one of `self`, `super`, `package` or an identifier
    error Offset(366): expected one of `self`, `super`, `package` or an identifier
    error Offset(367): expected SEMI
    error Offset(367): expected a declaration
    error Offset(368): expected a declaration
    error Offset(369): expected a declaration
    "#);
}
