use crate::{
    diagnostics::DiagnosticSink, expr::BodySourceMap, mock::MockDatabase,
    with_fixture::WithFixture, HirDisplay, InferenceResult, ModuleDef, Package,
};
use std::{fmt::Write, sync::Arc};

#[test]
fn issue_354() {
    insta::assert_snapshot!(infer(
        r#"
    fn value() -> i64 { 6 }

    pub fn main() {
        let t = 2;
        t = loop { break value(); };
    }"#),
    @r###"
    18..23 '{ 6 }': i64
    20..21 '6': i64
    39..90 '{     ...; }; }': ()
    49..50 't': i64
    53..54 '2': i64
    60..61 't': i64
    60..87 't = lo...e(); }': ()
    64..87 'loop {...e(); }': i64
    69..87 '{ brea...e(); }': never
    71..84 'break value()': never
    77..82 'value': function value() -> i64
    77..84 'value()': i64
    "###);
}

#[test]
fn array_element_assignment() {
    insta::assert_snapshot!(infer(
        r"
    fn main() {
        let a = [1,2,3,4,5]
        a[2] = 4u8
    }",
    ), @r###"
    10..52 '{     ... 4u8 }': ()
    20..21 'a': [u8]
    24..35 '[1,2,3,4,5]': [u8]
    25..26 '1': u8
    27..28 '2': u8
    29..30 '3': u8
    31..32 '4': u8
    33..34 '5': u8
    40..41 'a': [u8]
    40..44 'a[2]': u8
    40..50 'a[2] = 4u8': ()
    42..43 '2': i32
    47..50 '4u8': u8
    "###)
}

#[test]
fn array_is_place_expr() {
    insta::assert_snapshot!(infer(
        r"
    fn main() {
        let a = [1,2,3,4,5]
        a = [5,6,7]
        a[0] = 0;
        [1,2,3][0] = 4
    }",
    ), @r###"
    10..86 '{     ... = 4 }': ()
    20..21 'a': [i32]
    24..35 '[1,2,3,4,5]': [i32]
    25..26 '1': i32
    27..28 '2': i32
    29..30 '3': i32
    31..32 '4': i32
    33..34 '5': i32
    40..41 'a': [i32]
    40..51 'a = [5,6,7]': ()
    44..51 '[5,6,7]': [i32]
    45..46 '5': i32
    47..48 '6': i32
    49..50 '7': i32
    56..57 'a': [i32]
    56..60 'a[0]': i32
    56..64 'a[0] = 0': ()
    58..59 '0': i32
    63..64 '0': i32
    70..77 '[1,2,3]': [i32]
    70..80 '[1,2,3][0]': i32
    70..84 '[1,2,3][0] = 4': ()
    71..72 '1': i32
    73..74 '2': i32
    75..76 '3': i32
    78..79 '0': i32
    83..84 '4': i32
    "###)
}

#[test]
fn infer_array_structs() {
    insta::assert_snapshot!(infer(
        r"
        struct Foo;

    fn main() -> Foo {
        let a = [Foo, Foo, Foo];
        a[2]
    }",
    ), @r###"
    34..75 '{     ...a[2] }': Foo
    44..45 'a': [Foo]
    48..63 '[Foo, Foo, Foo]': [Foo]
    49..52 'Foo': Foo
    54..57 'Foo': Foo
    59..62 'Foo': Foo
    69..70 'a': [Foo]
    69..73 'a[2]': Foo
    71..72 '2': i32
    "###)
}

#[test]
fn infer_array() {
    insta::assert_snapshot!(infer(
        r"
    fn main() -> u8 {
        let b = 5;
        let c = 3;
        let a = [1,b,4,c];
        a[3]
    }",
    ), @r###"
    16..81 '{     ...a[3] }': u8
    26..27 'b': u8
    30..31 '5': u8
    41..42 'c': u8
    45..46 '3': u8
    56..57 'a': [u8]
    60..69 '[1,b,4,c]': [u8]
    61..62 '1': u8
    63..64 'b': u8
    65..66 '4': u8
    67..68 'c': u8
    75..76 'a': [u8]
    75..79 'a[3]': u8
    77..78 '3': i32
    "###)
}

#[test]
fn private_access() {
    insta::assert_snapshot!(infer(
        r#"
    //- /foo.mun
    struct Foo {};
    struct Bar(i64);
    struct Baz;
    fn foo() {}
    type FooBar = Foo;

    pub struct PubFoo {};
    pub struct PubBar(i64);
    pub struct PubBaz;
    pub fn pub_foo() {}
    pub type PubFooBar = PubFoo;

    pub(super) struct PubSupFoo {};
    pub(super) struct PubSupBar(i64);
    pub(super) struct PubSupBaz;
    pub(super) fn pub_sup_foo() {}
    pub(super) type PubSupFooBar = PubSupFoo;

    pub(package) struct PubPackageFoo {};
    pub(package) struct PubPackageBar(i64);
    pub(package) struct PubPackageBaz;
    pub(package) fn pub_package_foo() {}
    pub(package) type PubPackageFooBar = PubPackageFoo;

    //- /bar.mun
    fn main() {
        let a = package::foo::Foo {}; // private access
        let a = package::foo::Bar(3); // private access
        let a = package::foo::Baz; // private access
        let a = package::foo::FooBar{}}; // private access

        let a = super::foo::Foo {}; // private access
        let a = super::foo::Bar(3); // private access
        let a = super::foo::Baz; // private access
        let a = super::foo::FooBar{}; // private access

        package::foo::foo(); // private access
        super::foo::foo(); // private access

        let a = package::foo::PubFoo {};
        let a = package::foo::PubBar(3);
        let a = package::foo::PubBaz;
        let a = package::foo::PubFooBar{};

        let a = super::foo::PubFoo {};
        let a = super::foo::PubBar(3);
        let a = super::foo::PubBaz;
        let a = super::foo::PubFooBar{};

        package::foo::pub_foo();
        super::foo::pub_foo();

        let a = package::foo::PubSupFoo {};
        let a = package::foo::PubSupBar(3);
        let a = package::foo::PubSupBaz;
        let a = package::foo::PubSupFooBar{};

        let a = super::foo::PubSupFoo {};
        let a = super::foo::PubSupBar(3);
        let a = super::foo::PubSupBaz;
        let a = super::foo::PubSupFooBar{};

        package::foo::pub_sup_foo();
        super::foo::pub_sup_foo();

        let a = package::foo::PubPackageFoo {};
        let a = package::foo::PubPackageBar(3);
        let a = package::foo::PubPackageBaz;
        let a = package::foo::PubPackageFooBar{};

        let a = super::foo::PubPackageFoo {};
        let a = super::foo::PubPackageBar(3);
        let a = super::foo::PubPackageBaz;
        let a = super::foo::PubPackageFooBar{};

        package::foo::pub_package_foo();
        super::foo::pub_package_foo();
    }

    //- /foo/baz.mun
    fn main() {
        let a = package::foo::Foo {};
        let a = package::foo::Bar(3);
        let a = package::foo::Baz;
        let a = package::foo::FooBar{};

        let a = super::Foo {};
        let a = super::Bar(3);
        let a = super::Baz;
        let a = super::FooBar{};

        package::foo::foo();
        super::foo();
    }

    //- /mod.mun
    fn main() {
        let a = package::foo::Foo {}; // private access
        let a = package::foo::Bar(3); // private access
        let a = package::foo::Baz; // private access
        let a = package::foo::FooBar{}; // private access

        let a = foo::Foo {}; // private access
        let a = foo::Bar(3); // private access
        let a = foo::Baz; // private access
        let a = foo::FooBar{}; // private access

        package::foo::foo(); // private access
        foo::foo(); // private access

        let a = package::foo::PubSupFoo {};
        let a = package::foo::PubSupBar(3);
        let a = package::foo::PubSupBaz;
        let a = package::foo::PubSupFooBar{};

        let a = foo::PubSupFoo {};
        let a = foo::PubSupBar(3);
        let a = foo::PubSupBaz;
        let a = foo::PubSupFooBar{};

        package::foo::pub_sup_foo();
        foo::pub_sup_foo();
    }
    "#),
    @r###"
    24..41: access of private type
    76..93: access of private type
    128..145: access of private type
    177..197: access of private type
    232..240: access of private type
    275..283: access of private type
    318..326: access of private type
    358..369: access of private type
    396..413: access of private type
    439..447: access of private type
    24..41: access of private type
    76..93: access of private type
    128..145: access of private type
    177..197: access of private type
    10..812 '{     ...o(); }': ()
    20..21 'a': Foo
    24..44 'packag...Foo {}': Foo
    72..73 'a': Bar
    76..93 'packag...o::Bar': ctor Bar(i64) -> Bar
    76..96 'packag...Bar(3)': Bar
    94..95 '3': i64
    124..125 'a': Baz
    128..145 'packag...o::Baz': Baz
    173..174 'a': Foo
    177..199 'packag...oBar{}': Foo
    228..229 'a': Foo
    232..243 'foo::Foo {}': Foo
    271..272 'a': Bar
    275..283 'foo::Bar': ctor Bar(i64) -> Bar
    275..286 'foo::Bar(3)': Bar
    284..285 '3': i64
    314..315 'a': Baz
    318..326 'foo::Baz': Baz
    354..355 'a': Foo
    358..371 'foo::FooBar{}': Foo
    396..413 'packag...o::foo': function foo() -> ()
    396..415 'packag...:foo()': ()
    439..447 'foo::foo': function foo() -> ()
    439..449 'foo::foo()': ()
    478..479 'a': PubSupFoo
    482..508 'packag...Foo {}': PubSupFoo
    518..519 'a': PubSupBar
    522..545 'packag...SupBar': ctor PubSupBar(i64) -> PubSupBar
    522..548 'packag...Bar(3)': PubSupBar
    546..547 '3': i64
    558..559 'a': PubSupBaz
    562..585 'packag...SupBaz': PubSupBaz
    595..596 'a': PubSupFoo
    599..627 'packag...oBar{}': PubSupFoo
    638..639 'a': PubSupFoo
    642..659 'foo::P...Foo {}': PubSupFoo
    669..670 'a': PubSupBar
    673..687 'foo::PubSupBar': ctor PubSupBar(i64) -> PubSupBar
    673..690 'foo::P...Bar(3)': PubSupBar
    688..689 '3': i64
    700..701 'a': PubSupBaz
    704..718 'foo::PubSupBaz': PubSupBaz
    728..729 'a': PubSupFoo
    732..751 'foo::P...oBar{}': PubSupFoo
    758..783 'packag...up_foo': function pub_sup_foo() -> ()
    758..785 'packag..._foo()': ()
    791..807 'foo::p...up_foo': function pub_sup_foo() -> ()
    791..809 'foo::p..._foo()': ()
    10..200 '{     ...Bar{}}': ()
    20..21 'a': Foo
    24..44 'packag...Foo {}': Foo
    72..73 'a': Bar
    76..93 'packag...o::Bar': ctor Bar(i64) -> Bar
    76..96 'packag...Bar(3)': Bar
    94..95 '3': i64
    124..125 'a': Baz
    128..145 'packag...o::Baz': Baz
    173..174 'a': Foo
    177..199 'packag...oBar{}': Foo
    53..55 '{}': ()
    158..160 '{}': ()
    314..316 '{}': ()
    507..509 '{}': ()
    10..300 '{     ...o(); }': ()
    20..21 'a': Foo
    24..44 'packag...Foo {}': Foo
    54..55 'a': Bar
    58..75 'packag...o::Bar': ctor Bar(i64) -> Bar
    58..78 'packag...Bar(3)': Bar
    76..77 '3': i64
    88..89 'a': Baz
    92..109 'packag...o::Baz': Baz
    119..120 'a': Foo
    123..145 'packag...oBar{}': Foo
    156..157 'a': Foo
    160..173 'super::Foo {}': Foo
    183..184 'a': Bar
    187..197 'super::Bar': ctor Bar(i64) -> Bar
    187..200 'super::Bar(3)': Bar
    198..199 '3': i64
    210..211 'a': Baz
    214..224 'super::Baz': Baz
    234..235 'a': Foo
    238..253 'super::FooBar{}': Foo
    260..277 'packag...o::foo': function foo() -> ()
    260..279 'packag...:foo()': ()
    285..295 'super::foo': function foo() -> ()
    285..297 'super::foo()': ()
    "###);
}

#[test]
fn scoped_path() {
    insta::assert_snapshot!(infer(
        r"
    //- /mod.mun
    struct Foo;

    fn main() -> self::Foo {
        Foo
    }

    fn bar() -> Foo {
        super::Foo  // undefined value
    }

    fn baz() -> Foo {
        package::Foo
    }

    //- /foo.mun
    struct Foo;

    fn bar() -> Foo {
        super::Foo  // mismatched type
    }

    fn baz() -> package::Foo {
        super::Foo
    }

    fn nested() -> self::Foo {
        package::foo::Foo
    }
    "),
    @r###"
    71..81: undefined value
    35..45: mismatched type
    29..67: mismatched type
    36..47 '{     Foo }': Foo
    42..45 'Foo': Foo
    65..103 '{     ...alue }': Foo
    71..81 'super::Foo': {unknown}
    121..141 '{     ...:Foo }': Foo
    127..139 'package::Foo': Foo
    29..67 '{     ...type }': Foo
    35..45 'super::Foo': Foo
    94..112 '{     ...:Foo }': Foo
    100..110 'super::Foo': Foo
    139..164 '{     ...:Foo }': Foo
    145..162 'packag...o::Foo': Foo
    "###);
}

#[test]
fn comparison_not_implemented_for_struct() {
    insta::assert_snapshot!(infer(
        r"
    struct Foo;

    fn main() -> bool {
        Foo == Foo
    }"),
    @r###"
    37..47: cannot apply binary operator
    31..49 '{     ... Foo }': bool
    37..40 'Foo': Foo
    37..47 'Foo == Foo': bool
    44..47 'Foo': Foo
    "###);
}

#[test]
fn infer_literals() {
    insta::assert_snapshot!(infer(
        r"
        fn integer() -> i32 {
            0
        }

        fn large_unsigned_integer() -> u128 {
            0
        }

        fn with_let() -> u16 {
            let b = 4;
            let a = 4;
            a
        }
    "),
    @r###"
    20..29 '{     0 }': i32
    26..27 '0': i32
    67..76 '{     0 }': u128
    73..74 '0': u128
    99..138 '{     ...   a }': u16
    109..110 'b': i32
    113..114 '4': i32
    124..125 'a': u16
    128..129 '4': u16
    135..136 'a': u16
    "###);
}

#[test]
fn infer_suffix_literals() {
    insta::assert_snapshot!(infer(
        r"
    fn main(){
        123;
        123u8;
        123u16;
        123u32;
        123u64;
        123u128;
        1_000_000_u32;
        123i8;
        123i16;
        123i32;
        123i64;
        123i128;
        1_000_000_i32;
        1_000_123.0e-2;
        1_000_123.0e-2f32;
        1_000_123.0e-2f64;
        9999999999999999999999999999999999999999999_f64;
    }

    fn add(a:u32) -> u32 {
        a + 12u32
    }

    fn errors() {
        0b22222; // invalid literal
        0b00010_f32; // non-10 base f64
        0o71234_f32; // non-10 base f64
        1234_foo; // invalid suffix
        1234.0_bar; // invalid suffix
        9999999999999999999999999999999999999999999; // too large
        256_u8; // literal out of range for `u8`
        128_i8; // literal out of range for `i8`
        12712371237123_u32; // literal out of range `u32`
        9999999999999999999999999; // literal out of range `i32`
    }
    "),
    @r###"
    358..365: invalid literal value
    390..401: binary float literal is not supported
    426..437: octal float literal is not supported
    462..470: invalid suffix `foo`
    494..504: invalid suffix `bar`
    528..571: int literal is too large
    590..596: literal out of range for `u8`
    635..641: literal out of range for `i8`
    680..698: literal out of range for `u32`
    734..759: literal out of range for `i32`
    9..298 '{     ...f64; }': ()
    15..18 '123': i32
    24..29 '123u8': u8
    35..41 '123u16': u16
    47..53 '123u32': u32
    59..65 '123u64': u64
    71..78 '123u128': u128
    84..97 '1_000_000_u32': u32
    103..108 '123i8': i8
    114..120 '123i16': i16
    126..132 '123i32': i32
    138..144 '123i64': i64
    150..157 '123i128': i128
    163..176 '1_000_000_i32': i32
    182..196 '1_000_123.0e-2': f64
    202..219 '1_000_...e-2f32': f32
    225..242 '1_000_...e-2f64': f64
    248..295 '999999...99_f64': f64
    307..308 'a': u32
    321..338 '{     ...2u32 }': u32
    327..328 'a': u32
    327..336 'a + 12u32': u32
    331..336 '12u32': u32
    352..792 '{     ...i32` }': ()
    358..365 '0b22222': i32
    390..401 '0b00010_f32': f32
    426..437 '0o71234_f32': f32
    462..470 '1234_foo': i32
    494..504 '1234.0_bar': f64
    528..571 '999999...999999': i32
    590..596 '256_u8': u8
    635..641 '128_i8': i8
    680..698 '127123...23_u32': u32
    734..759 '999999...999999': i32
    "###);
}

#[test]
fn infer_invalid_struct_type() {
    insta::assert_snapshot!(infer(
        r"
    fn main(){
        let a = Foo {b: 3};
    }"),
    @r###"
    23..26: undefined type
    9..36 '{     ... 3}; }': ()
    19..20 'a': {unknown}
    23..33 'Foo {b: 3}': {unknown}
    31..32 '3': i32
    "###);
}

#[test]
fn infer_conditional_return() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo(a:int)->i32 {
        if a > 4 {
            return 4;
        }
        a
    }

    fn bar(a:i32)->i32 {
        if a > 4 {
            return 4;
        } else {
            return 1;
        }
    }
    "#),
    @r###"
    9..12: undefined type
    7..8 'a': {unknown}
    19..67 '{     ...   a }': i32
    25..59 'if a >...     }': ()
    28..29 'a': {unknown}
    28..33 'a > 4': bool
    32..33 '4': i32
    34..59 '{     ...     }': never
    44..52 'return 4': never
    51..52 '4': i32
    64..65 'a': {unknown}
    76..77 'a': i32
    88..161 '{     ...   } }': i32
    94..159 'if a >...     }': i32
    97..98 'a': i32
    97..102 'a > 4': bool
    101..102 '4': i32
    103..128 '{     ...     }': never
    113..121 'return 4': never
    120..121 '4': i32
    134..159 '{     ...     }': never
    144..152 'return 1': never
    151..152 '1': i32
    "###);
}

#[test]
fn infer_return() {
    insta::assert_snapshot!(infer(
        r#"
    fn test()->i32 {
        return; // error: mismatched type
        return 5;
    }
    "#),
    @r###"
    21..27: `return;` in a function whose return type is not `()`
    15..70 '{     ...n 5; }': never
    21..27 'return': never
    59..67 'return 5': never
    66..67 '5': i32
    "###);
}

#[test]
fn infer_basics() {
    insta::assert_snapshot!(infer(
        r#"
    fn test(a:i32, b:f64, c:never, d:bool) -> bool {
        a;
        b;
        c;
        d
    }
    "#),
    @r###"
    8..9 'a': i32
    15..16 'b': f64
    22..23 'c': never
    31..32 'd': bool
    47..77 '{     ...   d }': never
    53..54 'a': i32
    60..61 'b': f64
    67..68 'c': never
    74..75 'd': bool
    "###);
}

#[test]
fn infer_branching() {
    insta::assert_snapshot!(infer(
        r#"
    fn test() {
        let a = if true { 3 } else { 4 }
        let b = if true { 3 }               // Missing else branch
        let c = if true { 3; }
        let d = if true { 5 } else if false { 3 } else { 4 }
        let e = if true { 5.0 } else { 5 }  // Mismatched branches
    }
    "#),
    @r###"
    61..74: missing else branch
    208..234: mismatched branches
    10..260 '{     ...ches }': ()
    20..21 'a': i32
    24..48 'if tru... { 4 }': i32
    27..31 'true': bool
    32..37 '{ 3 }': i32
    34..35 '3': i32
    43..48 '{ 4 }': i32
    45..46 '4': i32
    57..58 'b': ()
    61..74 'if true { 3 }': ()
    64..68 'true': bool
    69..74 '{ 3 }': i32
    71..72 '3': i32
    120..121 'c': ()
    124..138 'if true { 3; }': ()
    127..131 'true': bool
    132..138 '{ 3; }': ()
    134..135 '3': i32
    147..148 'd': i32
    151..195 'if tru... { 4 }': i32
    154..158 'true': bool
    159..164 '{ 5 }': i32
    161..162 '5': i32
    170..195 'if fal... { 4 }': i32
    173..178 'false': bool
    179..184 '{ 3 }': i32
    181..182 '3': i32
    190..195 '{ 4 }': i32
    192..193 '4': i32
    204..205 'e': f64
    208..234 'if tru... { 5 }': f64
    211..215 'true': bool
    216..223 '{ 5.0 }': f64
    218..221 '5.0': f64
    229..234 '{ 5 }': i32
    231..232 '5': i32
    "###);
}

#[test]
fn void_return() {
    insta::assert_snapshot!(infer(
        r#"
    fn bar() {
        let a = 3;
    }
    fn foo(a:i32) {
        let c = bar()
    }
    "#),
    @r###"
    9..27 '{     ...= 3; }': ()
    19..20 'a': i32
    23..24 '3': i32
    35..36 'a': i32
    42..63 '{     ...ar() }': ()
    52..53 'c': ()
    56..59 'bar': function bar() -> ()
    56..61 'bar()': ()
    "###);
}

#[test]
fn place_expressions() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo(a:i32) {
        a += 3;
        3 = 5; // error: invalid left hand side of expression
    }
    "#),
    @r###"
    32..33: invalid left hand side of expression
    7..8 'a': i32
    14..87 '{     ...sion }': ()
    20..21 'a': i32
    20..26 'a += 3': ()
    25..26 '3': i32
    32..33 '3': i32
    32..37 '3 = 5': ()
    36..37 '5': i32
    "###);
}

#[test]
fn update_operators() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo(a:i32, b:f64) {
        a += 3;
        a -= 3;
        a *= 3;
        a /= 3;
        a %= 3;
        b += 3.0;
        b -= 3.0;
        b *= 3.0;
        b /= 3.0;
        b %= 3.0;
        a *= 3.0; // mismatched type
        b *= 3; // mismatched type
    }
    "#),
    @r###"
    162..165: mismatched type
    195..196: mismatched type
    7..8 'a': i32
    14..15 'b': f64
    21..218 '{     ...type }': ()
    27..28 'a': i32
    27..33 'a += 3': ()
    32..33 '3': i32
    39..40 'a': i32
    39..45 'a -= 3': ()
    44..45 '3': i32
    51..52 'a': i32
    51..57 'a *= 3': ()
    56..57 '3': i32
    63..64 'a': i32
    63..69 'a /= 3': ()
    68..69 '3': i32
    75..76 'a': i32
    75..81 'a %= 3': ()
    80..81 '3': i32
    87..88 'b': f64
    87..95 'b += 3.0': ()
    92..95 '3.0': f64
    101..102 'b': f64
    101..109 'b -= 3.0': ()
    106..109 '3.0': f64
    115..116 'b': f64
    115..123 'b *= 3.0': ()
    120..123 '3.0': f64
    129..130 'b': f64
    129..137 'b /= 3.0': ()
    134..137 '3.0': f64
    143..144 'b': f64
    143..151 'b %= 3.0': ()
    148..151 '3.0': f64
    157..158 'a': i32
    157..165 'a *= 3.0': ()
    162..165 '3.0': f64
    190..191 'b': f64
    190..196 'b *= 3': ()
    195..196 '3': i32
    "###);
}

#[test]
fn infer_unary_ops() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo(a: i32, b: bool) {
        a = -a;
        b = !b;
    }
        "#),
    @r###"
    7..8 'a': i32
    15..16 'b': bool
    24..51 '{     ... !b; }': ()
    30..31 'a': i32
    30..36 'a = -a': ()
    34..36 '-a': i32
    35..36 'a': i32
    42..43 'b': bool
    42..48 'b = !b': ()
    46..48 '!b': bool
    47..48 'b': bool
    "###);
}

#[test]
fn invalid_unary_ops() {
    insta::assert_snapshot!(infer(
        r#"
    fn bar(a: f64, b: bool) {
        a = !a; // mismatched type
        b = -b; // mismatched type
    }
        "#),
    @r###"
    35..36: cannot apply unary operator
    66..67: cannot apply unary operator
    7..8 'a': f64
    15..16 'b': bool
    24..89 '{     ...type }': ()
    30..31 'a': f64
    30..36 'a = !a': ()
    34..36 '!a': {unknown}
    35..36 'a': f64
    61..62 'b': bool
    61..67 'b = -b': ()
    65..67 '-b': {unknown}
    66..67 'b': bool
    "###);
}

#[test]
fn infer_loop() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo() {
        loop {}
    }
    "#),
    @r###"
    9..24 '{     loop {} }': never
    15..22 'loop {}': never
    20..22 '{}': ()
    "###);
}

#[test]
fn infer_break() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo()->i32 {
        break; // error: not in a loop
        loop { break 3; break 3.0; } // error: mismatched type
        let a:i32 = loop { break 3.0; } // error: mismatched type
        loop { break 3; }
        let a:i32 = loop { break loop { break 3; } }
        loop { break loop { break 3.0; } } // error: mismatched type
    }
    "#),
    @r###"
    20..25: `break` outside of a loop
    71..80: mismatched type
    133..142: mismatched type
    267..276: mismatched type
    14..309 '{     ...type }': never
    20..25 'break': never
    55..83 'loop {...3.0; }': i32
    60..83 '{ brea...3.0; }': never
    62..69 'break 3': never
    68..69 '3': i32
    71..80 'break 3.0': never
    77..80 '3.0': f64
    118..119 'a': i32
    126..145 'loop {...3.0; }': i32
    131..145 '{ break 3.0; }': never
    133..142 'break 3.0': never
    139..142 '3.0': f64
    176..193 'loop {...k 3; }': i32
    181..193 '{ break 3; }': never
    183..190 'break 3': never
    189..190 '3': i32
    202..203 'a': i32
    210..242 'loop {...3; } }': i32
    215..242 '{ brea...3; } }': never
    217..240 'break ...k 3; }': never
    223..240 'loop {...k 3; }': i32
    228..240 '{ break 3; }': never
    230..237 'break 3': never
    236..237 '3': i32
    247..281 'loop {...0; } }': i32
    252..281 '{ brea...0; } }': never
    254..279 'break ...3.0; }': never
    260..279 'loop {...3.0; }': i32
    265..279 '{ break 3.0; }': never
    267..276 'break 3.0': never
    273..276 '3.0': f64
    "###);
}

#[test]
fn infer_while() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo() {
        let n = 0;
        while n < 3 { n += 1; };
        while n < 3 { n += 1; break; };
        while n < 3 { break 3; };   // error: break with value can only appear in a loop
        while n < 3 { loop { break 3; }; };
    }
    "#),
    @r###"
    109..116: `break` with value can only appear in a `loop`
    9..217 '{     ...; }; }': ()
    19..20 'n': i32
    23..24 '0': i32
    30..53 'while ...= 1; }': ()
    36..37 'n': i32
    36..41 'n < 3': bool
    40..41 '3': i32
    42..53 '{ n += 1; }': ()
    44..45 'n': i32
    44..50 'n += 1': ()
    49..50 '1': i32
    59..89 'while ...eak; }': ()
    65..66 'n': i32
    65..70 'n < 3': bool
    69..70 '3': i32
    71..89 '{ n +=...eak; }': never
    73..74 'n': i32
    73..79 'n += 1': ()
    78..79 '1': i32
    81..86 'break': never
    95..119 'while ...k 3; }': ()
    101..102 'n': i32
    101..106 'n < 3': bool
    105..106 '3': i32
    107..119 '{ break 3; }': never
    109..116 'break 3': never
    180..214 'while ...; }; }': ()
    186..187 'n': i32
    186..191 'n < 3': bool
    190..191 '3': i32
    192..214 '{ loop...; }; }': ()
    194..211 'loop {...k 3; }': i32
    199..211 '{ break 3; }': never
    201..208 'break 3': never
    207..208 '3': i32
    "###);
}

#[test]
fn invalid_binary_ops() {
    insta::assert_snapshot!(infer(
        r#"
    fn foo() {
        let b = false;
        let n = 1;
        let _ = b + n; // error: invalid binary operation
    }
    "#),
    @r###"
    57..62: cannot apply binary operator
    9..100 '{     ...tion }': ()
    19..20 'b': bool
    23..28 'false': bool
    38..39 'n': i32
    42..43 '1': i32
    57..58 'b': bool
    57..62 'b + n': i32
    61..62 'n': i32
    "###);
}

#[test]
fn struct_decl() {
    insta::assert_snapshot!(infer(
        r#"
    struct Foo;
    struct(gc) Bar {
        f: f64,
        i: i32,
    }
    struct(value) Baz(f64, i32);


    fn main() {
        let foo: Foo;
        let bar: Bar;
        let baz: Baz;
    }
    "#),
    @r###"
    96..153 '{     ...Baz; }': ()
    106..109 'foo': Foo
    124..127 'bar': Bar
    142..145 'baz': Baz
    "###);
}

#[test]
fn struct_lit() {
    insta::assert_snapshot!(infer(
        r#"
    struct Foo;
    struct Bar {
        a: f64,
    }
    struct Baz(f64, i32);

    fn main() {
        let a: Foo = Foo;
        let b: Bar = Bar { a: 1.23, };
        let c = Baz(1.23, 1);

        let a = Foo{}; // error: mismatched struct literal kind. expected `unit struct`, found `record`
        let a = Foo(); // error: mismatched struct literal kind. expected `unit struct`, found `tuple`
        let b = Bar; // error: mismatched struct literal kind. expected `record`, found `unit struct`
        let b = Bar(); // error: mismatched struct literal kind. expected `record`, found `tuple`
        let b = Bar{}; // error: missing record fields: a
        let c = Baz; // error: mismatched struct literal kind. expected `tuple`, found `unit struct`
        let c = Baz{}; // error: mismatched struct literal kind. expected `tuple`, found `record`
        let c = Baz(); // error: this tuple struct literal has 2 fields but 0 fields were supplied
    }
    "#),
    @r###"
    170..175: mismatched struct literal kind. expected `unit struct`, found `record`
    270..275: mismatched struct literal kind. expected `unit struct`, found `tuple`
    369..372: mismatched struct literal kind. expected `record`, found `unit struct`
    467..470: mismatched struct literal kind. expected `record`, found `tuple`
    561..566: missing record fields:
    - a

    615..618: mismatched struct literal kind. expected `tuple`, found `unit struct`
    712..717: mismatched struct literal kind. expected `tuple`, found `record`
    806..811: this tuple struct literal has 2 fields but 0 fields were supplied
    72..890 '{     ...lied }': ()
    82..83 'a': Foo
    91..94 'Foo': Foo
    104..105 'b': Bar
    113..129 'Bar { ....23, }': Bar
    122..126 '1.23': f64
    139..140 'c': Baz
    143..146 'Baz': ctor Baz(f64, i32) -> Baz
    143..155 'Baz(1.23, 1)': Baz
    147..151 '1.23': f64
    153..154 '1': i32
    166..167 'a': Foo
    170..175 'Foo{}': Foo
    266..267 'a': Foo
    270..273 'Foo': Foo
    270..275 'Foo()': Foo
    365..366 'b': Bar
    369..372 'Bar': Bar
    463..464 'b': Bar
    467..470 'Bar': Bar
    467..472 'Bar()': Bar
    557..558 'b': Bar
    561..566 'Bar{}': Bar
    611..612 'c': ctor Baz(f64, i32) -> Baz
    615..618 'Baz': ctor Baz(f64, i32) -> Baz
    708..709 'c': Baz
    712..717 'Baz{}': Baz
    802..803 'c': Baz
    806..809 'Baz': ctor Baz(f64, i32) -> Baz
    806..811 'Baz()': Baz
    "###);
}

#[test]
fn struct_field_index() {
    insta::assert_snapshot!(infer(
        r#"
    struct Foo {
        a: f64,
        b: i32,
    }
    struct Bar(f64, i32)
    struct Baz;

    fn main() {
        let foo = Foo { a: 1.23, b: 4 };
        foo.a
        foo.b
        foo.c // error: attempted to access a non-existent field in a struct.
        let bar = Bar(1.23, 4);
        bar.0
        bar.1
        bar.2 // error: attempted to access a non-existent field in a struct.
        let baz = Baz;
        baz.a // error: attempted to access a non-existent field in a struct.
        let f = 1.0
        f.0; // error: attempted to access a field on a primitive type.
    }
    "#),
    @r###"
    146..151: attempted to access a non-existent field in a struct.
    268..273: attempted to access a non-existent field in a struct.
    361..366: attempted to access a non-existent field in a struct.
    451..452: attempted to access a field on a primitive type.
    83..516 '{     ...ype. }': ()
    93..96 'foo': Foo
    99..120 'Foo { ...b: 4 }': Foo
    108..112 '1.23': f64
    117..118 '4': i32
    126..129 'foo': Foo
    126..131 'foo.a': f64
    136..139 'foo': Foo
    136..141 'foo.b': i32
    146..149 'foo': Foo
    146..151 'foo.c': {unknown}
    224..227 'bar': Bar
    230..233 'Bar': ctor Bar(f64, i32) -> Bar
    230..242 'Bar(1.23, 4)': Bar
    234..238 '1.23': f64
    240..241 '4': i32
    248..251 'bar': Bar
    248..253 'bar.0': f64
    258..261 'bar': Bar
    258..263 'bar.1': i32
    268..271 'bar': Bar
    268..273 'bar.2': {unknown}
    346..349 'baz': Baz
    352..355 'Baz': Baz
    361..364 'baz': Baz
    361..366 'baz.a': {unknown}
    439..440 'f': f64
    443..446 '1.0': f64
    451..452 'f': f64
    451..454 'f.0': {unknown}
    "###);
}

#[test]
fn primitives() {
    insta::assert_snapshot!(infer(
        r#"
    fn unsigned_primitives(a: u8, b: u16, c: u32, d: u64, e: u128, f: usize, g: u32) -> u8 { a }
    fn signed_primitives(a: i8, b: i16, c: i32, d: i64, e: i128, f: isize, g: i32) -> i8 { a }
    fn float_primitives(a: f32, b: f64, c: f64) -> f32 { a }
    "#),
    @r###"
    23..24 'a': u8
    30..31 'b': u16
    38..39 'c': u32
    46..47 'd': u64
    54..55 'e': u128
    63..64 'f': usize
    73..74 'g': u32
    87..92 '{ a }': u8
    89..90 'a': u8
    114..115 'a': i8
    121..122 'b': i16
    129..130 'c': i32
    137..138 'd': i64
    145..146 'e': i128
    154..155 'f': isize
    164..165 'g': i32
    178..183 '{ a }': i8
    180..181 'a': i8
    204..205 'a': f32
    212..213 'b': f64
    220..221 'c': f64
    235..240 '{ a }': f32
    237..238 'a': f32
    "###);
}

#[test]
fn extern_fn() {
    insta::assert_snapshot!(infer(
        r#"
    extern fn foo(a:i32, b:i32) -> i32;
    fn main() {
        foo(3,4);
    }

    extern fn with_body() {}    // extern functions cannot have bodies

    struct S;
    extern fn with_non_primitive(s:S);  // extern functions can only have primitives as parameters
    extern fn with_non_primitive_return() -> S;  // extern functions can only have primitives as parameters
    "#),
    @r###"
    65..89: extern functions cannot have bodies
    174..175: extern functions can only have primitives as parameter- and return types
    279..280: extern functions can only have primitives as parameter- and return types
    14..15 'a': i32
    21..22 'b': i32
    46..63 '{     ...,4); }': ()
    52..55 'foo': function foo(i32, i32) -> i32
    52..60 'foo(3,4)': i32
    56..57 '3': i32
    58..59 '4': i32
    87..89 '{}': ()
    172..173 's': S
    "###);
}

#[test]
fn infer_type_alias() {
    insta::assert_snapshot!(infer(
        r#"
    type Foo = i32;
    type Bar = Foo;
    type Baz = UnknownType;  // error: undefined type

    fn main(a: Foo) {
        let b: Bar = a;
    }
    "#),
    @r###"
    43..54: undefined type
    91..92 'a': i32
    99..122 '{     ...= a; }': ()
    109..110 'b': i32
    118..119 'a': i32
    "###);
}

#[test]
fn recursive_alias() {
    insta::assert_snapshot!(infer(
        r#"
    struct Foo {}
    type Foo = Foo;

    type A = B;
    type B = A;

    fn main() {
        let a: Foo;  // error: unknown type
        let b: A;    // error: unknown type
        let c: B;    // error: unknown type
    }
    "#),
    @r###"
    14..29: the name `Foo` is defined multiple times
    40..41: cyclic type
    52..53: cyclic type
    119..120: cyclic type
    159..160: cyclic type
    66..189 '{     ...type }': ()
    76..77 'a': Foo
    116..117 'b': {unknown}
    156..157 'c': {unknown}
    "###);
}

fn infer(content: &str) -> String {
    let db = MockDatabase::with_files(content);

    let mut acc = String::new();

    let mut infer_def = |infer_result: Arc<InferenceResult>,
                         body_source_map: Arc<BodySourceMap>| {
        let mut types = Vec::new();

        for (pat, ty) in infer_result.type_of_pat.iter() {
            let syntax_ptr = match body_source_map.pat_syntax(pat) {
                Some(sp) => sp.map(|ast| ast.syntax_node_ptr()),
                None => continue,
            };
            types.push((syntax_ptr, ty));
        }

        for (expr, ty) in infer_result.type_of_expr.iter() {
            let syntax_ptr = match body_source_map.expr_syntax(expr) {
                Some(sp) => {
                    sp.map(|ast| ast.either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr()))
                }
                None => continue,
            };
            types.push((syntax_ptr, ty));
        }

        // Sort ranges for consistency
        types.sort_by_key(|(src_ptr, _)| {
            (src_ptr.value.range().start(), src_ptr.value.range().end())
        });
        for (src_ptr, ty) in &types {
            let node = src_ptr.value.to_node(&src_ptr.file_syntax(&db));

            let (range, text) = (
                src_ptr.value.range(),
                node.text().to_string().replace('\n', " "),
            );
            writeln!(
                acc,
                "{:?} '{}': {}",
                range,
                ellipsize(text, 15),
                ty.display(&db)
            )
            .unwrap();
        }
    };

    let mut diags = String::new();

    let mut diag_sink = DiagnosticSink::new(|diag| {
        writeln!(diags, "{:?}: {}", diag.highlight_range(), diag.message()).unwrap();
    });

    for package in Package::all(&db).iter() {
        for module in package.modules(&db).iter() {
            module.diagnostics(&db, &mut diag_sink);
        }
    }

    for item in Package::all(&db)
        .iter()
        .flat_map(|pkg| pkg.modules(&db))
        .flat_map(|module| module.declarations(&db))
    {
        if let ModuleDef::Function(fun) = item {
            let source_map = fun.body_source_map(&db);
            let infer_result = fun.infer(&db);
            infer_def(infer_result, source_map);
        }
    }

    drop(diag_sink);

    acc.truncate(acc.trim_end().len());
    diags.truncate(diags.trim_end().len());
    [diags, acc].join("\n").trim().to_string()
}

fn ellipsize(mut text: String, max_len: usize) -> String {
    if text.len() <= max_len {
        return text;
    }
    let ellipsis = "...";
    let e_len = ellipsis.len();
    let mut prefix_len = (max_len - e_len) / 2;
    while !text.is_char_boundary(prefix_len) {
        prefix_len += 1;
    }
    let mut suffix_len = max_len - e_len - prefix_len;
    while !text.is_char_boundary(text.len() - suffix_len) {
        suffix_len += 1;
    }
    text.replace_range(prefix_len..text.len() - suffix_len, ellipsis);
    text
}
