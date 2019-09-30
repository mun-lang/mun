use super::*;
use crate::T;

pub(super) const DECLARATION_RECOVERY_SET: TokenSet = token_set![FN_KW, EXPORT_KW];

pub(super) fn mod_contents(p: &mut Parser) {
    while !p.matches(EOF) {
        declaration(p);
    }
}

pub(super) fn declaration(p: &mut Parser) {
    let m = p.start();
    let m = match maybe_declaration(p, m) {
        Ok(()) => return,
        Err(m) => m,
    };

    m.abandon(p);
    if p.matches(L_CURLY) {
        error_block(p, "expected a declaration")
    } else if p.matches(R_CURLY) {
        let e = p.start();
        p.error("unmatched }");
        p.bump();
        e.complete(p, ERROR);
    } else if !p.matches(EOF) {
        p.error_and_bump("expected a declaration");
    } else {
        p.error("expected a declaration");
    }
}

pub(super) fn maybe_declaration(p: &mut Parser, m: Marker) -> Result<(), Marker> {
    opt_visibility(p);

    match p.current() {
        FN_KW => {
            fn_def(p);
            m.complete(p, FUNCTION_DEF);
        }
        _ => return Err(m),
    }
    Ok(())
}

pub(super) fn fn_def(p: &mut Parser) {
    assert!(p.matches(FN_KW));
    p.bump();

    name_recovery(p, DECLARATION_RECOVERY_SET.union(token_set![L_PAREN]));

    if p.matches(L_PAREN) {
        params::param_list(p);
    } else {
        p.error("expected function arguments")
    }

    opt_fn_ret_type(p);

    expressions::block(p);
}

fn opt_fn_ret_type(p: &mut Parser) -> bool {
    if p.matches(T![:]) {
        let m = p.start();
        p.bump();
        types::type_(p);
        m.complete(p, RET_TYPE);
        true
    } else {
        false
    }
}
