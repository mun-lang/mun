use super::*;

pub(super) const PATTERN_FIRST: TokenSet = expressions::LITERAL_FIRST
    .union(paths::PATH_FIRST)
    .union(token_set![MINUS, UNDERSCORE]);

pub(super) fn pattern(p: &mut Parser) {
    pattern_r(p, PATTERN_FIRST);
}

pub(super) fn pattern_r(p: &mut Parser, recovery_set: TokenSet) {
    atom_pat(p, recovery_set);
}

fn atom_pat(p: &mut Parser, recovery_set: TokenSet) -> Option<CompletedMarker> {
    let t1 = p.nth(0);
    if t1 == IDENT {
        return Some(bind_pat(p));
    }

    let m = match t1 {
        T![_] => placeholder_pat(p),
        _ => {
            p.error_recover("expected pattern", recovery_set);
            return None;
        }
    };
    Some(m)
}

fn placeholder_pat(p: &mut Parser) -> CompletedMarker {
    assert!(p.matches(T![_]));
    let m = p.start();
    p.bump();
    m.complete(p, PLACEHOLDER_PAT)
}

fn bind_pat(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    name(p);
    m.complete(p, BIND_PAT)
}
