use super::{patterns, types, Parser, TokenSet, EOF, NAME, PARAM, PARAM_LIST, SELF_PARAM};

pub(super) fn param_list(p: &mut Parser<'_>) {
    list(p);
}

fn list(p: &mut Parser<'_>) {
    assert!(p.at(T!['(']));

    let m = p.start();
    p.bump(T!['(']);

    opt_self_param(p);

    while !p.at(EOF) && !p.at(T![')']) {
        if !p.at_ts(VALUE_PARAMETER_FIRST) {
            p.error("expected value parameter");
            break;
        }
        param(p);
        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
    p.expect(T![')']);
    m.complete(p, PARAM_LIST);
}

const VALUE_PARAMETER_FIRST: TokenSet = patterns::PATTERN_FIRST;

fn param(p: &mut Parser<'_>) {
    let m = p.start();
    patterns::pattern(p);
    types::ascription(p);
    m.complete(p, PARAM);
}

fn opt_self_param(p: &mut Parser<'_>) {
    if p.at(T![self]) {
        let m = p.start();
        self_as_name(p);
        m.complete(p, SELF_PARAM);

        if !p.at(T![')']) {
            p.expect(T![,]);
        }
    }
}

fn self_as_name(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(T![self]);
    m.complete(p, NAME);
}
