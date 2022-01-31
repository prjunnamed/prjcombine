use super::*;

struct Lexer<'a> {
    s: &'a str,
    pos: usize,
    line: u32,
    la: Option<Token<'a>>,
}

enum Token<'a> {
    Word(&'a str),
    Str(&'a str),
    Comma,
    Semi,
    Dir(PipDirection),
    End,
}

impl<'a> Lexer<'a> {
    fn new(s: &'a str) -> Self {
        Lexer {
            s,
            pos: 0,
            line: 1,
            la: None,
        }
    }

    fn skip_whitespace(&mut self) {
        let mut comment = false;
        for (p, c) in self.s[self.pos..].char_indices() {
            if c == '\n' {
                self.line += 1;
                comment = false;
            } else if c == '#' {
                comment = true;
            } else if !comment && !c.is_whitespace() {
                self.pos += p;
                return;
            }
        }
        self.pos = self.s.len();
    }

    fn unlex(&mut self, token: Token<'a>) {
        assert!(self.la.is_none());
        self.la = Some(token);
    }

    fn lex(&mut self) -> Result<Token<'a>, ParseError> {
        if let Some(t) = std::mem::replace(&mut self.la, None) {
            return Ok(t);
        }
        self.skip_whitespace();
        let suffix = &self.s[self.pos..];
        if suffix.is_empty() {
            Ok(Token::End)
        } else if suffix.starts_with(',') {
            self.pos += 1;
            Ok(Token::Comma)
        } else if suffix.starts_with(';') {
            self.pos += 1;
            Ok(Token::Semi)
        } else if suffix.starts_with("->") {
            self.pos += 1;
            Ok(Token::Dir(PipDirection::UniBuf))
        } else if suffix.starts_with("=>") {
            self.pos += 1;
            Ok(Token::Dir(PipDirection::BiUniBuf))
        } else if suffix.starts_with("=-") {
            self.pos += 1;
            Ok(Token::Dir(PipDirection::BiBuf))
        } else if suffix.starts_with("==") {
            self.pos += 1;
            Ok(Token::Dir(PipDirection::Unbuf))
        } else if suffix.starts_with('"') {
            let mut bs = false;
            let start = self.pos + 1;
            for (p, c) in self.s[start..].char_indices() {
                if c == '\n' {
                    self.line += 1;
                    bs = false;
                } else if bs {
                    bs = false;
                } else if c == '\\' {
                    bs = true;
                } else if c == '\"' {
                    let end = start + p;
                    self.pos = end + 1;
                    return Ok(Token::Str(&self.s[start..end]));
                }
            }
            self.error(ParseErrorKind::UnclosedString)?
        } else {
            for (p, c) in self.s[self.pos..].char_indices() {
                if c.is_whitespace() || c == ';' || c == ',' {
                    let start = self.pos;
                    self.pos += p;
                    return Ok(Token::Word(&self.s[start..self.pos]));
                }
            }
            let start = self.pos;
            self.pos = self.s.len();
            Ok(Token::Word(&self.s[start..self.pos]))
        }
    }

    fn error<T>(&self, kind: ParseErrorKind) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.line,
            kind,
        })
    }

    fn get_word(&mut self) -> Result<&'a str, ParseError> {
        if let Token::Word(res) = self.lex()? {
            Ok(res)
        } else {
            self.error(ParseErrorKind::ExpectedWord)?
        }
    }

    fn expect_word(&mut self, word: &'static str, err: ParseErrorKind) -> Result<(), ParseError> {
        if let Token::Word(res) = self.lex()? {
            if res == word {
                Ok(())
            } else {
                self.error(err)?
            }
        } else {
            self.error(err)?
        }
    }

    fn get_id(&mut self) -> Result<String, ParseError> {
        Ok(String::from(self.get_word()?))
    }

    fn get_string(&mut self) -> Result<String, ParseError> {
        if let Token::Str(s) = self.lex()? {
            let mut res = String::new();
            let mut bs = false;
            for c in s.chars() {
                if bs {
                    res.push(c);
                    bs = false;
                } else if c == '\\' {
                    bs = true;
                } else {
                    res.push(c);
                }
            }
            Ok(res)
        } else {
            self.error(ParseErrorKind::ExpectedString)?
        }
    }

    fn get_pip_dir(&mut self) -> Result<PipDirection, ParseError> {
        if let Token::Dir(res) = self.lex()? {
            Ok(res)
        } else {
            self.error(ParseErrorKind::ExpectedPipDirection)?
        }
    }

    fn expect_comma(&mut self) -> Result<(), ParseError> {
        if let Token::Comma = self.lex()? {
            Ok(())
        } else {
            self.error(ParseErrorKind::ExpectedComma)?
        }
    }

    fn expect_semi(&mut self) -> Result<(), ParseError> {
        if let Token::Semi = self.lex()? {
            Ok(())
        } else {
            self.error(ParseErrorKind::ExpectedSemi)?
        }
    }

    fn get_semi_comma(&mut self) -> Result<bool, ParseError> {
        match self.lex()? {
            Token::Semi => Ok(false),
            Token::Comma => Ok(true),
            _ => self.error(ParseErrorKind::ExpectedCommaSemi)?,
        }
    }

    fn get_cfg(&mut self) -> Result<Config, ParseError> {
        if let Token::Str(s) = self.lex()? {
            let mut res = Vec::new();
            let mut chunk = Vec::new();
            let mut word = String::new();
            let mut bs = false;
            for c in s.chars() {
                if bs {
                    word.push(c);
                    bs = false;
                } else if c == '\\' {
                    bs = true;
                } else if c == ':' {
                    chunk.push(word);
                    word = String::new();
                } else if c.is_whitespace() {
                    if !chunk.is_empty() || !word.is_empty() {
                        chunk.push(word);
                        word = String::new();
                        res.push(chunk);
                        chunk = Vec::new();
                    }
                } else {
                    word.push(c);
                }
            }
            if !chunk.is_empty() || !word.is_empty() {
                chunk.push(word);
                res.push(chunk);
            }
            Ok(res)
        } else {
            self.error(ParseErrorKind::ExpectedString)?
        }
    }
}

pub fn parse(s: &str) -> Result<Design, ParseError> {
    let mut lexer = Lexer::new(s);
    lexer.expect_word("design", ParseErrorKind::ExpectedDesign)?;
    let name = lexer.get_string()?;
    let part = lexer.get_id()?;
    let version = lexer.get_id()?;
    let cfg = if lexer.get_semi_comma()? {
        lexer.expect_word("cfg", ParseErrorKind::ExpectedCfg)?;
        let c = lexer.get_cfg()?;
        lexer.expect_semi()?;
        c
    } else {
        Vec::new()
    };
    let mut instances = Vec::new();
    let mut nets = Vec::new();
    loop {
        match lexer.lex()? {
            Token::End => break,
            Token::Word("inst" | "instance") => {
                let name = lexer.get_string()?;
                let kind = lexer.get_string()?;
                lexer.expect_comma()?;
                let placement = match lexer.lex()? {
                    Token::Word("placed") => {
                        let tile = lexer.get_id()?;
                        let site = lexer.get_id()?;
                        Placement::Placed { tile, site }
                    }
                    Token::Word("unplaced") => match lexer.lex()? {
                        Token::Word("bonded") => Placement::Bonded,
                        Token::Word("unbonded") => Placement::Unbonded,
                        token => {
                            lexer.unlex(token);
                            Placement::Unplaced
                        }
                    },
                    _ => lexer.error(ParseErrorKind::ExpectedPlacement)?,
                };
                lexer.expect_comma()?;
                lexer.expect_word("cfg", ParseErrorKind::ExpectedCfg)?;
                let cfg = lexer.get_cfg()?;
                lexer.expect_semi()?;
                instances.push(Instance {
                    name,
                    kind,
                    placement,
                    cfg,
                });
            }
            Token::Word("net") => {
                let name = lexer.get_string()?;
                let typ = match lexer.lex()? {
                    Token::Word("gnd") => NetType::Gnd,
                    Token::Word("vcc") => NetType::Vcc,
                    token => {
                        lexer.unlex(token);
                        NetType::Plain
                    }
                };
                let mut cfg = Vec::new();
                let mut outpins = Vec::new();
                let mut inpins = Vec::new();
                let mut pips = Vec::new();
                while lexer.get_semi_comma()? {
                    match lexer.lex()? {
                        Token::Word("cfg") => {
                            cfg = lexer.get_cfg()?;
                        }
                        Token::Word("inpin") => {
                            let inst_name = lexer.get_string()?;
                            let pin = lexer.get_id()?;
                            inpins.push(NetPin { inst_name, pin });
                        }
                        Token::Word("outpin") => {
                            let inst_name = lexer.get_string()?;
                            let pin = lexer.get_id()?;
                            outpins.push(NetPin { inst_name, pin });
                        }
                        Token::Word("pip") => {
                            let tile = lexer.get_id()?;
                            let wire_from = lexer.get_id()?;
                            let dir = lexer.get_pip_dir()?;
                            let wire_to = lexer.get_id()?;
                            pips.push(NetPip {
                                tile,
                                wire_from,
                                dir,
                                wire_to,
                            });
                        }
                        Token::Semi => break,
                        _ => lexer.error(ParseErrorKind::ExpectedNetItem)?,
                    }
                }
                nets.push(Net {
                    name,
                    typ,
                    cfg,
                    outpins,
                    inpins,
                    pips,
                });
            }
            _ => lexer.error(ParseErrorKind::ExpectedTop)?,
        }
    }
    Ok(Design {
        name,
        part,
        version,
        cfg,
        instances,
        nets,
    })
}
