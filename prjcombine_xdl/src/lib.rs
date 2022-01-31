#![allow(clippy::write_with_newline)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

mod parser;

pub struct Design {
    pub name: String,
    pub part: String,
    pub version: String,
    pub cfg: Config,
    pub instances: Vec<Instance>,
    pub nets: Vec<Net>,
}

pub struct Instance {
    pub name: String,
    pub kind: String,
    pub placement: Placement,
    pub cfg: Config,
}

pub enum Placement {
    Placed { tile: String, site: String },
    Unplaced,
    Bonded,
    Unbonded,
}

type Config = Vec<Vec<String>>;

pub struct ConfigValue {
    pub cell: String,
    pub value: String,
    pub secondary: Option<String>,
}

pub struct Net {
    pub name: String,
    pub typ: NetType,
    pub inpins: Vec<NetPin>,
    pub outpins: Vec<NetPin>,
    pub pips: Vec<NetPip>,
    pub cfg: Config,
}

pub enum NetType {
    Plain,
    Gnd,
    Vcc,
}

pub struct NetPin {
    pub inst_name: String,
    pub pin: String,
}

pub struct NetPip {
    pub tile: String,
    pub wire_from: String,
    pub wire_to: String,
    pub dir: PipDirection,
}

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
        write!(f, "\"\n")?;
        let mut first = true;
        for chunk in self.0 {
            if !first {
                write!(f, "\n")?;
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
        write!(
            f,
            "design {} {} {}",
            fmt_string(&self.name),
            self.part,
            self.version
        )?;
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
            write!(f, ",\n")?;
            for pin in &net.outpins {
                write!(f, "  outpin {} {},\n", fmt_string(&pin.inst_name), pin.pin)?;
            }
            for pin in &net.inpins {
                write!(f, "  inpin {} {},\n", fmt_string(&pin.inst_name), pin.pin)?;
            }
            for pip in &net.pips {
                let dir = match pip.dir {
                    PipDirection::Unbuf => "==",
                    PipDirection::BiBuf => "=-",
                    PipDirection::BiUniBuf => "=>",
                    PipDirection::UniBuf => "->",
                };
                write!(
                    f,
                    " pip {} {} {} {},\n",
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
