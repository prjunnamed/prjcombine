use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

mod parser;

#[derive(Debug)]
pub struct Design {
    pub name: String,
    pub part: String,
    pub version: String,
    pub cfg: Config,
    pub instances: Vec<Instance>,
    pub nets: Vec<Net>,
}

#[derive(Debug)]
pub struct Instance {
    pub name: String,
    pub kind: String,
    pub placement: Placement,
    pub cfg: Config,
}

#[derive(Debug)]
pub enum Placement {
    Placed { tile: String, site: String },
    Unplaced,
    Bonded,
    Unbonded,
}

type Config = Vec<Vec<String>>;

#[derive(Debug)]
pub struct Net {
    pub name: String,
    pub typ: NetType,
    pub inpins: Vec<NetPin>,
    pub outpins: Vec<NetPin>,
    pub pips: Vec<NetPip>,
    pub cfg: Config,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NetType {
    Plain,
    Gnd,
    Vcc,
}

#[derive(Debug)]
pub struct NetPin {
    pub inst_name: String,
    pub pin: String,
}

#[derive(Debug)]
pub struct NetPip {
    pub tile: String,
    pub wire_from: String,
    pub wire_to: String,
    pub dir: PipDirection,
}

#[derive(Debug)]
pub enum PipDirection {
    Unbuf,
    BiUniBuf,
    BiBuf,
    UniBuf,
}

struct FmtString<'a>(&'a str);

fn fmt_string(s: &str) -> FmtString<'_> {
    FmtString(s)
}

impl Display for FmtString<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        for c in self.0.chars() {
            match c {
                '\\' | '"' => write!(f, "\\{}", c)?,
                _ => write!(f, "{}", c)?,
            }
        }
        write!(f, "\"")?;
        Ok(())
    }
}

struct FmtCfg<'a>(&'a Config);

fn fmt_cfg(c: &Config) -> FmtCfg<'_> {
    FmtCfg(c)
}

impl Display for FmtCfg<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\"")?;
        let mut first = true;
        for chunk in self.0 {
            if !first {
                writeln!(f)?;
            }
            write!(f, "  ")?;
            first = false;
            let mut first_part = true;
            for part in chunk {
                if !first_part {
                    write!(f, ":")?;
                }
                first_part = false;
                for c in part.chars() {
                    match c {
                        '\\' | '"' | ':' | ' ' => write!(f, "\\{}", c)?,
                        _ => write!(f, "{}", c)?,
                    }
                }
            }
        }
        write!(f, "\"")?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ParseErrorKind {
    UnclosedString,
    ExpectedWord,
    ExpectedString,
    ExpectedDesign,
    ExpectedCfg,
    ExpectedCommaSemi,
    ExpectedComma,
    ExpectedSemi,
    ExpectedTop,
    ExpectedPlacement,
    ExpectedNetItem,
    ExpectedPipDirection,
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: u32,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let desc = match self.kind {
            ParseErrorKind::UnclosedString => "unclosed string",
            ParseErrorKind::ExpectedWord => "expected a word",
            ParseErrorKind::ExpectedString => "expected a string",
            ParseErrorKind::ExpectedDesign => "expected `design`",
            ParseErrorKind::ExpectedCfg => "expected `cfg`",
            ParseErrorKind::ExpectedCommaSemi => "expected `,` or `;`",
            ParseErrorKind::ExpectedComma => "expected `,`",
            ParseErrorKind::ExpectedSemi => "expected `;`",
            ParseErrorKind::ExpectedTop => "expected `instance` or `net``",
            ParseErrorKind::ExpectedPlacement => "expected `placed` or `unplaced`",
            ParseErrorKind::ExpectedNetItem => "expected `inpin`, `outpin`, or `pip`",
            ParseErrorKind::ExpectedPipDirection => "expected `==`, `=>`, `=-`, or `->`",
        };
        write!(f, "parse error in line {}: {}", self.line, desc)
    }
}

impl Error for ParseError {}

impl Design {
    pub fn write(&self, f: &mut dyn Write) -> io::Result<()> {
        write!(f, "design {} {}", fmt_string(&self.name), self.part,)?;
        if !self.version.is_empty() {
            write!(f, " {}", self.version)?;
        }
        if !self.cfg.is_empty() {
            write!(f, ", cfg {}", fmt_cfg(&self.cfg))?;
        }
        write!(f, ";\n\n")?;

        for inst in &self.instances {
            write!(
                f,
                "inst {} {}, ",
                fmt_string(&inst.name),
                fmt_string(&inst.kind)
            )?;
            match &inst.placement {
                Placement::Placed { tile, site } => write!(f, "placed {} {}", tile, site)?,
                Placement::Unplaced => write!(f, "unplaced")?,
                Placement::Bonded => write!(f, "unplaced bonded")?,
                Placement::Unbonded => write!(f, "unplaced unbonded")?,
            }
            write!(f, ", cfg {};\n\n", fmt_cfg(&inst.cfg))?;
        }

        for net in &self.nets {
            write!(f, "net {}", fmt_string(&net.name))?;
            match net.typ {
                NetType::Plain => (),
                NetType::Gnd => write!(f, " gnd")?,
                NetType::Vcc => write!(f, " vcc")?,
            }
            if !net.cfg.is_empty() {
                write!(f, ", cfg {}", fmt_cfg(&net.cfg))?;
            }
            writeln!(f, ",")?;
            for pin in &net.outpins {
                writeln!(f, "  outpin {} {},", fmt_string(&pin.inst_name), pin.pin)?;
            }
            for pin in &net.inpins {
                writeln!(f, "  inpin {} {},", fmt_string(&pin.inst_name), pin.pin)?;
            }
            for pip in &net.pips {
                let dir = match pip.dir {
                    PipDirection::Unbuf => "==",
                    PipDirection::BiBuf => "=-",
                    PipDirection::BiUniBuf => "=>",
                    PipDirection::UniBuf => "->",
                };
                writeln!(
                    f,
                    " pip {} {} {} {},",
                    pip.tile, pip.wire_from, dir, pip.wire_to
                )?;
            }
            write!(f, ";\n\n")?;
        }
        Ok(())
    }

    pub fn parse(s: &str) -> Result<Self, ParseError> {
        parser::parse(s)
    }
}

pub fn parse_lut(sz: u8, val: &str) -> Option<u64> {
    let rval = match sz {
        4 => val.strip_prefix("D=")?,
        5 => val.strip_prefix("O5=")?,
        6 => val.strip_prefix("O6=")?,
        _ => panic!("invalid sz"),
    };
    let mask = match sz {
        4 => 0xffff,
        5 => 0xffffffff,
        6 => 0xffffffffffffffff,
        _ => panic!("invalid sz"),
    };
    if let Some(rv) = rval.strip_prefix("0x") {
        u64::from_str_radix(rv, 16).ok()
    } else {
        #[derive(Eq, PartialEq, Copy, Clone, Debug)]
        enum StackEntry {
            Val(u64),
            And,
            Or,
            Xor,
            Not,
            Par,
        }
        let mut stack = Vec::new();
        let mut ch = rval.chars();
        loop {
            let c = ch.next();
            while let &[.., StackEntry::Not, StackEntry::Val(v)] = &stack[..] {
                stack.pop();
                stack.pop();
                stack.push(StackEntry::Val(!v));
            }
            while let &[.., StackEntry::Val(v1), StackEntry::And, StackEntry::Val(v2)] = &stack[..]
            {
                stack.pop();
                stack.pop();
                stack.pop();
                stack.push(StackEntry::Val(v1 & v2));
            }
            if c == Some('*') {
                stack.push(StackEntry::And);
                continue;
            }
            while let &[.., StackEntry::Val(v1), StackEntry::Xor, StackEntry::Val(v2)] = &stack[..]
            {
                stack.pop();
                stack.pop();
                stack.pop();
                stack.push(StackEntry::Val(v1 ^ v2));
            }
            if c == Some('@') {
                stack.push(StackEntry::Xor);
                continue;
            }
            while let &[.., StackEntry::Val(v1), StackEntry::Or, StackEntry::Val(v2)] = &stack[..] {
                stack.pop();
                stack.pop();
                stack.pop();
                stack.push(StackEntry::Val(v1 | v2));
            }
            if c.is_none() {
                break;
            }
            match c.unwrap() {
                '(' => stack.push(StackEntry::Par),
                '0' => stack.push(StackEntry::Val(0)),
                '1' => stack.push(StackEntry::Val(0xffffffffffffffff)),
                'A' => {
                    stack.push(StackEntry::Val(match ch.next()? {
                        '1' => 0xaaaaaaaaaaaaaaaa,
                        '2' => 0xcccccccccccccccc,
                        '3' => 0xf0f0f0f0f0f0f0f0,
                        '4' => 0xff00ff00ff00ff00,
                        '5' => 0xffff0000ffff0000,
                        '6' => 0xffffffff00000000,
                        _ => return None,
                    }));
                }
                '+' => stack.push(StackEntry::Or),
                '~' => stack.push(StackEntry::Not),
                ')' => {
                    if let &[.., StackEntry::Par, StackEntry::Val(v)] = &stack[..] {
                        stack.pop();
                        stack.pop();
                        stack.push(StackEntry::Val(v));
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        if stack.len() == 1 {
            if let StackEntry::Val(r) = stack[0] {
                return Some(r & mask);
            }
        }
        None
    }
}
