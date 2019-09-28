use super::*;

pub(super) fn param_list(p: &mut Parser) {
    list(p)
}

fn list(p: &mut Parser) {
    assert!(p.matches(L_PAREN));
    let m = p.start();
    p.bump();
    while !p.matches(EOF) && !p.matches(R_PAREN) {
        if !p.matches_any(VALUE_PARAMETER_FIRST) {
            p.error("expected value parameter");
            break;
        }
        param(p);
        if !p.matches(R_PAREN) {
            p.expect(COMMA);
        }
    }
    p.expect(R_PAREN);
    m.complete(p, PARAM_LIST);
}

const VALUE_PARAMETER_FIRST: TokenSet = patterns::PATTERN_FIRST;

fn param(p: &mut Parser) {
    let m = p.start();
    patterns::pattern(p);
    types::ascription(p);
    m.complete(p, PARAM);
}
