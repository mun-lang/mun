use crate::{
    parsing::{event::Event, token_set::TokenSet, ParseError, TokenSource},
    SyntaxKind::{self, *},
};
use drop_bomb::DropBomb;
use std::cell::Cell;

/// `Parser` struct provides the low-level API for navigating through the stream of tokens and
/// constructing the parse tree. The actual parsing happens in the `grammar` module.
///
/// However, the result of this `Parser` is not a real tree, but rather a flat stream of
/// events of the form 'start expression, consume number literal, finish espression'. See `Event`
/// docs for more info.
pub(crate) struct Parser<'t> {
    token_source: &'t dyn TokenSource,
    token_pos: usize,
    events: Vec<Event>,
    steps: Cell<u32>,
}

impl<'t> Parser<'t> {
    pub(super) fn new(token_source: &'t dyn TokenSource) -> Parser<'t> {
        Parser {
            token_source,
            token_pos: 0,
            events: Vec::new(),
            steps: Cell::new(0),
        }
    }

    pub(crate) fn finish(self) -> Vec<Event> {
        self.events
    }

    /// Returns the kind of the current token.
    /// If the parser has already reach the end of the input the special `EOF` kind is returned.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Returns the kind of the current two tokens, if they are not separated by trivia.
    pub(crate) fn current2(&self) -> Option<(SyntaxKind, SyntaxKind)> {
        let c1 = self.nth(0);
        let c2 = self.nth(1);

        if self.token_source.is_token_joint_to_next(self.token_pos) {
            Some((c1, c2))
        } else {
            None
        }
    }

    /// Lookahead operation: returns the kind of the next nth token.
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        let steps = self.steps.get();
        assert!(steps <= 10_000, "the parser seems stuck");
        self.steps.set(steps + 1);

        let mut i = 0;
        let mut count = 0;
        loop {
            let mut kind = self.token_source.token_kind(self.token_pos + i);
            if let Some((composited, step)) = self.is_composite(kind, i) {
                kind = composited;
                i += step;
            } else {
                i += 1;
            }

            match kind {
                EOF => return EOF,
                _ if count == n => return kind,
                _ => count += 1,
            }
        }
    }

    /// Checks if the current token is `kind`
    pub(crate) fn matches(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Checks if the current tokens is in `kinds`
    pub(crate) fn matches_any(&self, kinds: TokenSet) -> bool {
        kinds.contains(self.current())
    }

    /// Starts a new node in the syntax tree. All nodes and tokens consumed between the `start` and
    /// the corresponding `Marker::complete` belong to the same node.
    pub(crate) fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.push_event(Event::tombstone());
        Marker::new(pos)
    }

    pub(crate) fn bump(&mut self) {
        let kind = self.nth(0);
        if kind == EOF {
            return;
        }

        use SyntaxKind::*;
        match kind {
            DOTDOTDOT => {
                self.bump_compound(kind, 3);
            }
            DOTDOT | COLONCOLON | EQEQ => {
                self.bump_compound(kind, 2);
            }
            _ => {
                self.do_bump(kind, 1);
            }
        }
    }

    pub fn is_composite(&self, kind: SyntaxKind, n: usize) -> Option<(SyntaxKind, usize)> {
        let joint1 = self.token_source.is_token_joint_to_next(self.token_pos + n);
        let kind1 = self.token_source.token_kind(self.token_pos + n + 1);
        let joint2 = self
            .token_source
            .is_token_joint_to_next(self.token_pos + n + 1);
        let kind2 = self.token_source.token_kind(self.token_pos + n + 2);

        use SyntaxKind::*;

        // This does not match all the multi character symbols because they are still context
        // sensitive (for example `+=` and `..=`).
        match kind {
            DOT if joint1 && kind1 == DOT && joint2 && kind2 == DOT => Some((DOTDOTDOT, 3)),
            DOT if joint1 && kind1 == DOT => Some((DOTDOT, 2)),

            COLON if joint1 && kind1 == COLON => Some((COLONCOLON, 2)),
            EQ if joint2 && kind1 == EQ => Some((EQEQ, 2)),

            _ => None,
        }
    }

    /// Advances the parser by `n` tokens, remapping its kind. This is useful to create compound
    /// tokens from parts. For example an `::` is two consecutive remapped `:` tokens.
    pub(crate) fn bump_compound(&mut self, kind: SyntaxKind, n: u8) {
        self.do_bump(kind, n);
    }

    fn do_bump(&mut self, kind: SyntaxKind, n_raw_tokens: u8) {
        self.token_pos += n_raw_tokens as usize;
        self.push_event(Event::Token { kind, n_raw_tokens });
    }

    /// Emit error with the `message`
    pub(crate) fn error<T: Into<String>>(&mut self, message: T) {
        let msg = ParseError(message.into());
        self.push_event(Event::Error { msg });
    }

    /// Create an error node and consume the next token.
    pub(crate) fn error_and_bump(&mut self, message: &str) {
        self.error_recover(message, TokenSet::empty())
    }

    /// Create an error node and consume the next token.
    pub(crate) fn error_recover(&mut self, message: &str, recovery: TokenSet) {
        if self.matches(L_CURLY) || self.matches(R_CURLY) || self.matches_any(recovery) {
            self.error(message);
        } else {
            let m = self.start();
            self.error(message);
            self.bump();
            m.complete(self, ERROR);
        }
    }

    fn push_event(&mut self, event: Event) {
        self.events.push(event)
    }

    /// Consume the next token if `kind` matches.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.matches(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the next token if it is `kind` or emit an error otherwise.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error(format!("expected {:?}", kind));
        false
    }
}

/// See `Parser::start`
pub(crate) struct Marker {
    pos: u32,
    bomb: DropBomb,
}

impl Marker {
    fn new(pos: u32) -> Marker {
        Marker {
            pos,
            bomb: DropBomb::new("Marker must be either completed or abandoned"),
        }
    }

    /// Finishes the syntax tree node and assigns `kind` to it, and create a `CompletedMarker` for
    /// possible future operation like `.precede()` to deal with forward_parent.
    pub(crate) fn complete(mut self, p: &mut Parser, kind: SyntaxKind) -> CompletedMarker {
        self.bomb.defuse();
        let idx = self.pos as usize;
        match p.events[idx] {
            Event::Start {
                kind: ref mut slot, ..
            } => {
                *slot = kind;
            }
            _ => unreachable!(),
        }
        let finish_pos = p.events.len() as u32;
        p.push_event(Event::Finish);
        CompletedMarker::new(self.pos, finish_pos, kind)
    }

    /// Abandons the syntax tree node. All its children are attached to its parent instead.
    pub(crate) fn abandon(mut self, p: &mut Parser) {
        self.bomb.defuse();
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            match p.events.pop() {
                Some(Event::Start {
                    kind: TOMBSTONE,
                    forward_parent: None,
                }) => (),
                _ => unreachable!(),
            }
        }
    }
}

pub(crate) struct CompletedMarker {
    start_pos: u32,
    finish_pos: u32,
    kind: SyntaxKind,
}

impl CompletedMarker {
    fn new(start_pos: u32, finish_pos: u32, kind: SyntaxKind) -> Self {
        CompletedMarker {
            start_pos,
            finish_pos,
            kind,
        }
    }

    /// This method allows to create a new node which starts *before* the current one. That is,
    /// the parser could start node `A`, then complete it, and then after parsing the whole `A`,
    /// decide that it should have started some node `B` before starting `A`. `precede` allows to
    /// do exactly that. See also docs about `forward_parent` in `Event::Start`.
    ///
    /// Given completed events `[START, FINISH]` and its corresponding `CompletedMarker(pos: 0, _)`,
    /// append a new `START` event as `[START, FINISH, NEWSTART]`, then mark `NEWSTART` as `START`'s
    /// parent with saving its relative distance to `NEWSTART` into forward_parent(=2 in this case).
    pub(crate) fn precede(self, p: &mut Parser) -> Marker {
        let new_pos = p.start();
        let idx = self.start_pos as usize;
        match p.events[idx] {
            Event::Start {
                ref mut forward_parent,
                ..
            } => {
                *forward_parent = Some(new_pos.pos - self.start_pos);
            }
            _ => unreachable!(),
        }
        new_pos
    }

    /// Undo this completion and turns into a `Marker`
    pub(crate) fn undo_completion(self, p: &mut Parser) -> Marker {
        let start_idx = self.start_pos as usize;
        let finish_idx = self.finish_pos as usize;
        match p.events[start_idx] {
            Event::Start {
                ref mut kind,
                forward_parent: None,
            } => *kind = TOMBSTONE,
            _ => unreachable!(),
        }
        match p.events[finish_idx] {
            ref mut slot @ Event::Finish => *slot = Event::tombstone(),
            _ => unreachable!(),
        }
        Marker::new(self.start_pos)
    }

    pub(crate) fn kind(&self) -> SyntaxKind {
        self.kind
    }
}
