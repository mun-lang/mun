use super::{patterns, types, Parser, TokenSet, EOF, PARAM, PARAM_LIST};

pub(super) fn param_list(p: &mut Parser<'_>) {
    list(p);
}

fn list(p: &mut Parser<'_>) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
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
