use super::*;

pub(super) fn struct_def(p: &mut Parser, m: Marker) {
    assert!(p.at(T![struct]));
    p.bump(T![struct]);

    name_recovery(p, declarations::DECLARATION_RECOVERY_SET);
    match p.current() {
        T![;] => {
            p.bump(T![;]);
        }
        T!['{'] => record_field_def_list(p),
        T!['('] => tuple_field_def_list(p),
        _ => {
            p.error("expected a ';', or '{'");
        }
    }
    m.complete(p, STRUCT_DEF);
}

pub(super) fn record_field_def_list(p: &mut Parser) {
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

pub(super) fn tuple_field_def_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    while !p.at(T![')']) && !p.at(EOF) {
        let m = p.start();
        if !p.at_ts(types::TYPE_FIRST) {
            m.abandon(p);
            p.error_and_bump("expected a type");
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

fn record_field_def(p: &mut Parser) {
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
