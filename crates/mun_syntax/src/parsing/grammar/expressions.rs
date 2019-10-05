use super::*;

pub(crate) const LITERAL_FIRST: TokenSet =
    token_set![TRUE_KW, FALSE_KW, INT_NUMBER, FLOAT_NUMBER, STRING];

const EXPR_RECOVERY_SET: TokenSet = token_set![LET_KW];

const ATOM_EXPR_FIRST: TokenSet = LITERAL_FIRST.union(token_set![IDENT, L_PAREN]);

const LHS_FIRST: TokenSet = ATOM_EXPR_FIRST.union(token_set![NOT_KW, MINUS]);

const EXPR_FIRST: TokenSet = LHS_FIRST;

pub(crate) fn expr_block_contents(p: &mut Parser) {
    while !p.matches(EOF) && !p.matches(R_CURLY) {
        if p.eat(SEMI) {
            continue;
        }

        stmt(p);
    }
}

/// Parses a block statement
pub(crate) fn block(p: &mut Parser) {
    if !p.matches(L_CURLY) {
        p.error("expected a block");
        return;
    }
    let m = p.start();
    p.bump();
    expr_block_contents(p);
    p.expect(R_CURLY);
    m.complete(p, BLOCK);
}

/// Parses a general statement: (let, expr, etc.)
pub(super) fn stmt(p: &mut Parser) {
    let m = p.start();

    // Encounters let keyword, so we know it's a let stmt
    if p.matches(LET_KW) {
        let_stmt(p, m);
        return;
    }

    let cm = expr_stmt(p);
    let kind = cm.as_ref().map(|cm| cm.kind()).unwrap_or(ERROR);

    if p.matches(R_CURLY) {
        if let Some(cm) = cm {
            cm.undo_completion(p).abandon(p);
            m.complete(p, kind);
        } else {
            m.abandon(p);
        }
    } else {
        p.eat(SEMI);
        m.complete(p, EXPR_STMT);
    }
}

fn let_stmt(p: &mut Parser, m: Marker) {
    assert!(p.matches(LET_KW));
    p.bump();
    patterns::pattern(p);
    if p.matches(COLON) {
        types::ascription(p);
    }
    if p.eat(EQ) {
        expressions::expr(p);
    }

    p.eat(SEMI); // Semicolon at the end of statement belongs to the statement
    m.complete(p, LET_STMT);
}

pub(super) fn expr(p: &mut Parser) {
    expr_bp(p, 1);
}

fn expr_stmt(p: &mut Parser) -> Option<CompletedMarker> {
    expr_bp(p, 1)
}

fn expr_bp(p: &mut Parser, bp: u8) -> Option<CompletedMarker> {
    // Parse left hand side of the expression
    let mut lhs = match lhs(p) {
        Some(lhs) => lhs,
        None => return None,
    };

    loop {
        let (op_bp, op) = current_op(p);
        if op_bp < bp {
            break;
        }

        let m = lhs.precede(p);
        match op {
            Op::Simple => p.bump(),
            Op::Composite(kind, n) => {
                p.bump_compound(kind, n);
            }
        }

        expr_bp(p, op_bp + 1);
        lhs = m.complete(p, BIN_EXPR);
    }

    Some(lhs)
}

enum Op {
    Simple,
    Composite(SyntaxKind, u8),
}

fn current_op(p: &Parser) -> (u8, Op) {
    if let Some(t) = p.current2() {
        match t {
            (PLUS, EQ) => return (1, Op::Composite(PLUSEQ, 2)),
            (MINUS, EQ) => return (1, Op::Composite(MINUSEQ, 2)),
            (STAR, EQ) => return (1, Op::Composite(STAREQ, 2)),
            (SLASH, EQ) => return (1, Op::Composite(SLASHEQ, 2)),
            (CARET, EQ) => return (1, Op::Composite(CARETEQ, 2)),
            (PERCENT, EQ) => return (1, Op::Composite(PERCENTEQ, 2)),
            (LT, EQ) => return (5, Op::Composite(LTEQ, 2)),
            (GT, EQ) => return (5, Op::Composite(GTEQ, 2)),
            _ => (),
        }
    }

    let bp = match p.current() {
        EQ => 1,
        EQEQ | NEQ | LT | GT => 5,
        MINUS | PLUS => 10,
        STAR | SLASH | PERCENT => 11,
        CARET => 12,
        _ => 0,
    };
    (bp, Op::Simple)
}

fn lhs(p: &mut Parser) -> Option<CompletedMarker> {
    let m;
    let kind = match p.current() {
        MINUS | NOT_KW => {
            m = p.start();
            p.bump();
            PREFIX_EXPR
        }
        _ => {
            let lhs = atom_expr(p)?;
            return Some(postfix_expr(p, lhs));
        }
    };
    expr_bp(p, 255);
    Some(m.complete(p, kind))
}

fn postfix_expr(p: &mut Parser, mut lhs: CompletedMarker) -> CompletedMarker {
    loop {
        lhs = match p.current() {
            L_PAREN => call_expr(p, lhs),
            _ => break,
        }
    }
    lhs
}

fn call_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.matches(L_PAREN));
    let m = lhs.precede(p);
    arg_list(p);
    m.complete(p, CALL_EXPR)
}

fn arg_list(p: &mut Parser) {
    assert!(p.matches(L_PAREN));
    let m = p.start();
    p.bump();
    while !p.matches(R_PAREN) && !p.matches(EOF) {
        if !p.matches_any(EXPR_FIRST) {
            p.error("expected expression");
            break;
        }

        expr(p);
        if !p.matches(R_PAREN) && !p.expect(COMMA) {
            break;
        }
    }
    p.eat(R_PAREN);
    m.complete(p, ARG_LIST);
}

fn atom_expr(p: &mut Parser) -> Option<CompletedMarker> {
    if let Some(m) = literal(p) {
        return Some(m);
    }

    if paths::is_path_start(p) {
        return Some(path_expr(p));
    }

    if p.matches(IDENT) {
        let m = p.start();
        p.bump();
        return Some(m.complete(p, NAME_REF));
    }

    let marker = match p.current() {
        L_PAREN => paren_expr(p),
        _ => {
            p.error_recover("expected expression", EXPR_RECOVERY_SET);
            return None;
        }
    };
    Some(marker)
}

fn path_expr(p: &mut Parser) -> CompletedMarker {
    let m = p.start();
    paths::expr_path(p);
    m.complete(p, PATH_EXPR)
}

fn literal(p: &mut Parser) -> Option<CompletedMarker> {
    if !p.matches_any(LITERAL_FIRST) {
        return None;
    }
    let m = p.start();
    p.bump();
    Some(m.complete(p, LITERAL))
}

fn paren_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.matches(L_PAREN));
    let m = p.start();
    p.bump();
    expr(p);
    p.expect(R_PAREN);
    m.complete(p, PAREN_EXPR)
}
