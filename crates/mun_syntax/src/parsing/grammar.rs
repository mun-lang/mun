mod adt;
mod declarations;
mod expressions;
mod params;
mod paths;
mod patterns;
mod traits;
mod types;

use super::{
    parser::{CompletedMarker, Marker, Parser},
    token_set::TokenSet,
    SyntaxKind::{
        self, ARG_LIST, ARRAY_EXPR, ARRAY_TYPE, BIND_PAT, BIN_EXPR, BLOCK_EXPR, BREAK_EXPR,
        CALL_EXPR, CONDITION, ENUM_DEF, ENUM_VARIANT, ENUM_VARIANT_LIST, EOF, ERROR, EXPR_STMT,
        EXTERN, FIELD_EXPR, FLOAT_NUMBER, FUNCTION_DEF, GC_KW, IDENT, IF_EXPR, INDEX, INDEX_EXPR,
        INT_NUMBER, LET_STMT, LITERAL, LOOP_EXPR, MEMORY_TYPE_SPECIFIER, NAME, NAME_REF,
        NEVER_TYPE, PARAM, PARAM_LIST, PAREN_EXPR, PATH, PATH_EXPR, PATH_SEGMENT, PATH_TYPE,
        PLACEHOLDER_PAT, PREFIX_EXPR, RECORD_FIELD, RECORD_FIELD_DEF, RECORD_FIELD_DEF_LIST,
        RECORD_FIELD_LIST, RECORD_LIT, RENAME, RETURN_EXPR, RET_TYPE, SELF_PARAM, SOURCE_FILE,
        STRING, STRUCT_DEF, TUPLE_FIELD_DEF, TUPLE_FIELD_DEF_LIST, TYPE_ALIAS_DEF, USE, USE_TREE,
        USE_TREE_LIST, VALUE_KW, VISIBILITY, WHILE_EXPR,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockLike {
    Block,
    NotBlock,
}

impl BlockLike {
    fn is_block(self) -> bool {
        self == BlockLike::Block
    }
}

pub(crate) fn root(p: &mut Parser<'_>) {
    let m = p.start();
    declarations::mod_contents(p);
    m.complete(p, SOURCE_FILE);
}

//pub(crate) fn pattern(p: &mut Parser<'_>) {
//    patterns::pattern(p)
//}
//
//pub(crate) fn expr(p: &mut Parser<'_>) {
//    expressions::expr(p);
//}
//
//pub(crate) fn type_(p: &mut Parser<'_>) {
//    types::type_(p)
//}

fn name_recovery(p: &mut Parser<'_>, recovery: TokenSet) {
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME);
    } else {
        p.error_recover("expected a name", recovery);
    }
}

fn name(p: &mut Parser<'_>) {
    name_recovery(p, TokenSet::empty());
}

fn name_ref(p: &mut Parser<'_>) {
    if p.at(IDENT) {
        let m = p.start();
        p.bump(IDENT);
        m.complete(p, NAME_REF);
    } else {
        p.error_and_bump("expected identifier");
    }
}

fn name_ref_or_index(p: &mut Parser<'_>) {
    assert!(p.at(IDENT) || p.at(INT_NUMBER));
    let m = p.start();
    p.bump_any();
    m.complete(p, NAME_REF);
}

fn opt_visibility(p: &mut Parser<'_>) -> bool {
    match p.current() {
        T![pub] => {
            let m = p.start();
            p.bump(T![pub]);
            if p.at(T!['(']) {
                match p.nth(1) {
                    T![package] | T![super] => {
                        p.bump_any();
                        p.bump_any();
                        p.expect(T![')']);
                    }
                    _ => (),
                }
            }
            m.complete(p, VISIBILITY);
            true
        }
        _ => false,
    }
}

fn error_block(p: &mut Parser<'_>, message: &str) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.error(message);
    p.bump(T!['{']);
    expressions::expr_block_contents(p);
    p.eat(T!['}']);
    m.complete(p, ERROR);
}
