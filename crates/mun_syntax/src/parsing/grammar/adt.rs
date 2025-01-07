use super::{
    declarations, error_block, name, name_recovery, opt_visibility, types, Marker, Parser, EOF,
    GC_KW, IDENT, MEMORY_TYPE_SPECIFIER, RECORD_FIELD_DEF, RECORD_FIELD_DEF_LIST, STRUCT_DEF,
    TUPLE_FIELD_DEF, TUPLE_FIELD_DEF_LIST, TYPE_ALIAS_DEF, VALUE_KW, VISIBILITY_FIRST,
};
use crate::{
    parsing::{grammar::types::TYPE_FIRST, token_set::TokenSet},
    SyntaxKind::ERROR,
};

const TUPLE_FIELD_FIRST: TokenSet = types::TYPE_FIRST.union(VISIBILITY_FIRST);

pub(super) fn struct_def(p: &mut Parser<'_>, m: Marker) {
    assert!(p.at(T![struct]));
    p.bump(T![struct]);
    opt_memory_type_specifier(p);
    name_recovery(p, declarations::DECLARATION_RECOVERY_SET);
    match p.current() {
        T![;] => {
            p.bump(T![;]);
        }
        T!['{'] => record_field_def_list(p),
        T!['('] => tuple_field_def_list(p),
        _ => {
            p.error("expected a ';', '{', or '('");
        }
    }
    m.complete(p, STRUCT_DEF);
}

pub(super) fn type_alias_def(p: &mut Parser<'_>, m: Marker) {
    assert!(p.at(T![type]));
    p.bump(T![type]);
    name(p);
    if p.eat(T![=]) {
        types::type_(p);
    }
    p.expect(T![;]);
    m.complete(p, TYPE_ALIAS_DEF);
}

pub(super) fn record_field_def_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    while !p.at(T!['}']) && !p.at(EOF) {
        if p.at(T!['{']) {
            error_block(p, "expected a field");
            continue;
        }
        record_field_def(p);
        if !p.at(T!['}']) {
            p.expect(T![,]);
        }
    }
    p.expect(T!['}']);
    p.eat(T![;]);
    m.complete(p, RECORD_FIELD_DEF_LIST);
}

fn opt_memory_type_specifier(p: &mut Parser<'_>) {
    if p.at(T!['(']) {
        let m = p.start();
        p.bump(T!['(']);
        if p.at(IDENT) {
            if p.at_contextual_kw("gc") {
                p.bump_remap(GC_KW);
            } else if p.at_contextual_kw("value") {
                p.bump_remap(VALUE_KW);
            } else {
                p.error_and_bump("expected memory type specifier");
            }
        } else {
            p.error("expected memory type specifier");
        }
        p.expect(T![')']);
        m.complete(p, MEMORY_TYPE_SPECIFIER);
    }
}

pub(super) fn tuple_field_def_list(p: &mut Parser<'_>) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    while !p.at(T![')']) && !p.at(EOF) {
        let m = p.start();
        if !p.at_ts(TUPLE_FIELD_FIRST) {
            m.abandon(p);
            p.error_and_bump("expected a tuple field");
            break;
        }
        let has_vis = opt_visibility(p);
        if !p.at_ts(TYPE_FIRST) {
            p.error("expected a type");
            if has_vis {
                m.complete(p, ERROR);
            } else {
                m.abandon(p);
            }
            break;
        }
        types::type_(p);
        m.complete(p, TUPLE_FIELD_DEF);

        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    p.eat(T![;]);
    m.complete(p, TUPLE_FIELD_DEF_LIST);
}

fn record_field_def(p: &mut Parser<'_>) {
    let m = p.start();
    opt_visibility(p);
    if p.at(IDENT) {
        name(p);
        p.expect(T![:]);
        types::type_(p);
        m.complete(p, RECORD_FIELD_DEF);
    } else {
        m.abandon(p);
        p.error_and_bump("expected a field declaration");
    }
}
