pub struct Lexer<'a> {
    pos: usize,
    source: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    BasicId,
    ExtId,
    DecimalInt,
    DecimalFloat,
    BasedInt,
    BasedFloat,
    Character,
    String,
    BitString,
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Comma,     // ,
    Dot,       // .
    Colon,     // :
    Semi,      // ;
    Equals,    // =
    Lt,        // <
    Gt,        // >
    VBar,      // |
    Ampersand, // &
    Tick,      // '
    Backtick,  // `
    Question,  // ?
    At,        // @
    Arrow,     // =>
    Exp,       // **
    Assign,    // :=
    SlashEq,   // /=
    GtEq,      // >=
    LtEq,      // <=
    LtGt,      // <>
    Cond,      // ??
    MatchEq,   // ?=
    MatchNeq,  // ?/=
    MatchLt,   // ?<
    MatchLtEq, // ?<=
    MatchGt,   // ?>
    MatchGtEq, // ?>=
    LtLt,      // <<
    GtGt,      // >>
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    Unknown,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { pos: 0, source }
    }

    fn decimal(&self, pos: usize) -> Option<usize> {
        let mut ch = self.source[pos..].chars();
        let c = ch.next()?;
        if !c.is_ascii_digit() {
            return None;
        }
        let mut end = self.source.len() - ch.as_str().len();
        while let Some(c) = ch.next() {
            if c != '-' && !c.is_ascii_digit() {
                break;
            }
            end = self.source.len() - ch.as_str().len();
        }
        Some(end)
    }

    fn exponent(&self, pos: usize) -> Option<usize> {
        let mut ch = self.source[pos..].chars();
        let c0 = ch.next();
        let c1 = ch.next();
        if !matches!(c0, Some('e' | 'E')) {
            return None;
        }
        let pos = if matches!(c1, Some('+' | '-')) {
            pos + 2
        } else {
            pos + 1
        };
        self.decimal(pos)
    }

    pub fn lex(&mut self) -> Token {
        let mut ch = self.source[self.pos..].chars();
        let c0 = ch.next();
        let c1 = ch.next();
        let c2 = ch.next();
        let start = self.pos;
        let (kind, delta) = match (c0, c1, c2) {
            (None, _, _) => (TokenKind::Eof, 0),
            (Some('('), _, _) => (TokenKind::LParen, 1),
            (Some(')'), _, _) => (TokenKind::RParen, 1),
            (Some('['), _, _) => (TokenKind::LBracket, 1),
            (Some(']'), _, _) => (TokenKind::RBracket, 1),
            (Some('+'), _, _) => (TokenKind::Plus, 1),
            (Some('-'), Some('-'), _) => {
                let mut ch = self.source[self.pos..].chars();
                let mut end = self.pos;
                while let Some(c) = ch.next() {
                    if matches!(c, '\r' | '\n') {
                        break;
                    }
                    end = self.source.len() - ch.as_str().len();
                }
                (TokenKind::LineComment, end - self.pos)
            }
            (Some('-'), _, _) => (TokenKind::Minus, 1),
            (Some('*'), Some('*'), _) => (TokenKind::Exp, 2),
            (Some('*'), _, _) => (TokenKind::Star, 1),
            (Some('/'), Some('='), _) => (TokenKind::SlashEq, 2),
            (Some('/'), Some('*'), _) => {
                if let Some(delta) = self.source[self.pos + 2..].find("*/") {
                    (TokenKind::BlockComment, 2 + delta)
                } else {
                    (TokenKind::BlockComment, self.source.len() - self.pos)
                }
            }
            (Some('/'), _, _) => (TokenKind::Slash, 1),
            (Some(','), _, _) => (TokenKind::Comma, 1),
            (Some('.'), _, _) => (TokenKind::Dot, 1),
            (Some(':'), Some('='), _) => (TokenKind::Assign, 2),
            (Some(':'), _, _) => (TokenKind::Colon, 1),
            (Some(';'), _, _) => (TokenKind::Semi, 1),
            (Some('='), Some('>'), _) => (TokenKind::Arrow, 2),
            (Some('='), _, _) => (TokenKind::Equals, 1),
            (Some('<'), Some('='), _) => (TokenKind::LtEq, 2),
            (Some('<'), Some('<'), _) => (TokenKind::LtLt, 2),
            (Some('<'), Some('>'), _) => (TokenKind::LtGt, 2),
            (Some('<'), _, _) => (TokenKind::Lt, 1),
            (Some('>'), Some('='), _) => (TokenKind::GtEq, 2),
            (Some('>'), Some('>'), _) => (TokenKind::GtGt, 2),
            (Some('>'), _, _) => (TokenKind::Gt, 1),
            (Some('|'), _, _) => (TokenKind::VBar, 1),
            (Some('&'), _, _) => (TokenKind::Ampersand, 1),
            (Some('\''), _, Some('\'')) => (TokenKind::Character, 3),
            (Some('\''), _, _) => (TokenKind::Tick, 1),
            (Some('`'), _, _) => (TokenKind::Backtick, 1),
            (Some('?'), Some('?'), _) => (TokenKind::Cond, 2),
            (Some('?'), Some('='), _) => (TokenKind::MatchEq, 2),
            (Some('?'), Some('/'), Some('=')) => (TokenKind::MatchNeq, 3),
            (Some('?'), Some('<'), Some('=')) => (TokenKind::MatchLtEq, 3),
            (Some('?'), Some('<'), _) => (TokenKind::MatchLt, 2),
            (Some('?'), Some('>'), Some('=')) => (TokenKind::MatchGtEq, 3),
            (Some('?'), Some('>'), _) => (TokenKind::MatchGt, 2),
            (Some('?'), _, _) => (TokenKind::Question, 1),
            (Some('@'), _, _) => (TokenKind::At, 1),
            (Some('\r'), Some('\n'), _) => (TokenKind::Newline, 2),
            (Some('\r' | '\n' | '\x0c' | '\x0b'), _, _) => (TokenKind::Newline, 1),
            (Some('\\'), _, _) => {
                let mut pos = self.pos + 1;
                loop {
                    let Some(delta) = self.source[pos..].find(['\\', '\n', '\r', '\x0c', '\x0b'])
                    else {
                        break (TokenKind::ExtId, self.source.len() - self.pos);
                    };
                    pos += delta;
                    if !self.source[pos..].starts_with('\\') {
                        break (TokenKind::ExtId, pos - self.pos);
                    }
                    pos += 1;
                    if !self.source[pos..].starts_with('\\') {
                        break (TokenKind::ExtId, pos - self.pos);
                    }
                    pos += 1;
                }
            }
            (Some('"'), _, _) => {
                let mut pos = self.pos + 1;
                loop {
                    let Some(delta) = self.source[pos..].find(['"', '\n', '\r', '\x0c', '\x0b'])
                    else {
                        break (TokenKind::String, self.source.len() - self.pos);
                    };
                    pos += delta;
                    if !self.source[pos..].starts_with('\\') {
                        break (TokenKind::String, pos - self.pos);
                    }
                    pos += 1;
                    if !self.source[pos..].starts_with('\\') {
                        break (TokenKind::String, pos - self.pos);
                    }
                    pos += 1;
                }
            }
            (Some(c), _, _) if c.is_whitespace() => {
                let mut ch = self.source[self.pos..].chars();
                let mut end = self.pos;
                while let Some(c) = ch.next() {
                    if matches!(c, '\r' | '\n' | '\x0c' | '\x0b') || !c.is_whitespace() {
                        break;
                    }
                    end = self.source.len() - ch.as_str().len();
                }
                (TokenKind::Whitespace, end - self.pos)
            }
            (Some(c), _, _) if c.is_alphabetic() => {
                // TODO basic id or bitstring
                todo!()
            }
            (Some(c), _, _) if c.is_ascii_digit() => {
                let pos = self.decimal(self.pos).unwrap();
                if let Some(pos_e) = self.exponent(pos) {
                    (TokenKind::DecimalInt, pos_e - self.pos)
                } else {
                    let mut ch = self.source[pos..].chars();
                    let c0 = ch.next();
                    let c1 = ch.next();
                    match (c0, c1, c2) {
                        (Some('.'), Some(c1)) if c1.is_ascii_digit() => {
                            let pos = self.decimal(pos + 1).unwrap();
                            if let Some(pos_e) = self.exponent(pos) {
                                (TokenKind::DecimalFloat, pos_e - self.pos)
                            } else {
                                (TokenKind::DecimalFloat, pos - self.pos)
                            }
                        }
                        (Some('#'), Some(c1)) if c1.is_alphanumeric() => {
                            // TODO based int/float
                            todo!()
                        }
                        // TODO bitstring
                        _ => (TokenKind::DecimalInt, pos - self.pos),
                    }
                }
            }
            _ => (TokenKind::Unknown, 1),
        };
        self.pos += delta;
        let end = self.pos;
        Token { kind, start, end }
    }
}
