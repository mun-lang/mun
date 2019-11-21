use super::*;
use crate::parsing::grammar::paths::PATH_FIRST;

pub(crate) const LITERAL_FIRST: TokenSet =
    token_set![TRUE_KW, FALSE_KW, INT_NUMBER, FLOAT_NUMBER, STRING];

const EXPR_RECOVERY_SET: TokenSet = token_set![LET_KW];

const ATOM_EXPR_FIRST: TokenSet = LITERAL_FIRST
    .union(PATH_FIRST)
    .union(token_set![IDENT, L_PAREN, L_CURLY, IF_KW, RETURN_KW,]);

const LHS_FIRST: TokenSet = ATOM_EXPR_FIRST.union(token_set![EXCLAMATION, MINUS]);

const EXPR_FIRST: TokenSet = LHS_FIRST;

pub(crate) fn expr_block_contents(p: &mut Parser) {
    while !p.at(EOF) && !p.at(T!['}']) {
        if p.eat(T![;]) {
            continue;
        }

        stmt(p);
    }
}

/// Parses a block statement
pub(crate) fn block(p: &mut Parser) {
    if !p.at(T!['{']) {
        p.error("expected a block");
        return;
    }
    block_expr(p);
}

fn block_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['{']));
    let m = p.start();
    p.bump(T!['{']);
    expr_block_contents(p);
    p.expect(T!['}']);
    m.complete(p, BLOCK_EXPR)
}

/// Parses a general statement: (let, expr, etc.)
pub(super) fn stmt(p: &mut Parser) {
    let m = p.start();

    // Encounters let keyword, so we know it's a let stmt
    if p.at(T![let]) {
        let_stmt(p, m);
        return;
    }

    let cm = expr_stmt(p);
    let kind = cm.as_ref().map(|cm| cm.kind()).unwrap_or(ERROR);

    if p.at(T!['}']) {
        if let Some(cm) = cm {
            cm.undo_completion(p).abandon(p);
            m.complete(p, kind);
        } else {
            m.abandon(p);
        }
    } else {
        p.eat(T![;]);
        m.complete(p, EXPR_STMT);
    }
}

fn let_stmt(p: &mut Parser, m: Marker) {
    assert!(p.at(T![let]));
    p.bump(T![let]);
    patterns::pattern(p);
    if p.at(T![:]) {
        types::ascription(p);
    }
    if p.eat(T![=]) {
        expressions::expr(p);
    }

    p.eat(T![;]); // Semicolon at the end of statement belongs to the statement
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
        p.bump(op);

        expr_bp(p, op_bp + 1);
        lhs = m.complete(p, BIN_EXPR);
    }

    Some(lhs)
}

fn current_op(p: &Parser) -> (u8, SyntaxKind) {
    match p.current() {
        T![+] if p.at(T![+=]) => (1, T![+=]),
        T![+] => (10, T![+]),
        T![-] if p.at(T![-=]) => (1, T![-=]),
        T![-] => (10, T![-]),
        T![*] if p.at(T![*=]) => (1, T![*=]),
        T![*] => (11, T![*]),
        T![/] if p.at(T![/=]) => (1, T![/=]),
        T![/] => (11, T![/]),
        T![=] if p.at(T![==]) => (5, T![==]),
        T![=] => (1, T![=]),
        T![!] if p.at(T![!=]) => (5, T![!=]),
        T![>] if p.at(T![>=]) => (5, T![>=]),
        T![>] => (5, T![>]),
        T![<] if p.at(T![<=]) => (5, T![<=]),
        T![<] => (5, T![<]),
        _ => (0, T![_]),
    }
}

fn lhs(p: &mut Parser) -> Option<CompletedMarker> {
    let m;
    let kind = match p.current() {
        T![-] | T![!] => {
            m = p.start();
            p.bump_any();
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
            T!['('] => call_expr(p, lhs),
            _ => break,
        }
    }
    lhs
}

fn call_expr(p: &mut Parser, lhs: CompletedMarker) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = lhs.precede(p);
    arg_list(p);
    m.complete(p, CALL_EXPR)
}

fn arg_list(p: &mut Parser) {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    while !p.at(T![')']) && !p.at(EOF) {
        if !p.at_ts(EXPR_FIRST) {
            p.error("expected expression");
            break;
        }

        expr(p);
        if !p.at(T![')']) && !p.expect(T![,]) {
            break;
        }
    }
    p.eat(T![')']);
    m.complete(p, ARG_LIST);
}

fn atom_expr(p: &mut Parser) -> Option<CompletedMarker> {
    if let Some(m) = literal(p) {
        return Some(m);
    }

    if paths::is_path_start(p) {
        return Some(path_expr(p));
    }

    let marker = match p.current() {
        T!['('] => paren_expr(p),
        T!['{'] => block_expr(p),
        T![if] => if_expr(p),
        T![loop] => loop_expr(p),
        T![return] => ret_expr(p),
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
    if !p.at_ts(LITERAL_FIRST) {
        return None;
    }
    let m = p.start();
    p.bump_any();
    Some(m.complete(p, LITERAL))
}

fn paren_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T!['(']));
    let m = p.start();
    p.bump(T!['(']);
    expr(p);
    p.expect(T![')']);
    m.complete(p, PAREN_EXPR)
}

fn if_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![if]));
    let m = p.start();
    p.bump(T![if]);
    cond(p);
    block(p);
    if p.at(T![else]) {
        p.bump(T![else]);
        if p.at(T![if]) {
            if_expr(p);
        } else {
            block(p);
        }
    }
    m.complete(p, IF_EXPR)
}

fn loop_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![loop]));
    let m = p.start();
    p.bump(T![loop]);
    block(p);
    m.complete(p, LOOP_EXPR)
}

fn cond(p: &mut Parser) {
    let m = p.start();
    expr(p);
    m.complete(p, CONDITION);
}

fn ret_expr(p: &mut Parser) -> CompletedMarker {
    assert!(p.at(T![return]));
    let m = p.start();
    p.bump(T![return]);
    if p.at_ts(EXPR_FIRST) {
        expr(p);
    }
    m.complete(p, RETURN_EXPR)
}
