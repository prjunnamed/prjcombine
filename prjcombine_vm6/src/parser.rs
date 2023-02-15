use std::{
    collections::HashSet,
    error::Error,
    fmt::{self, Display, Formatter},
};

use enum_map::enum_map;
use prjcombine_entity::{EntityId, EntityMap, EntityPartVec, EntityVec};
use prjcombine_xilinx_cpld::types::{ClkPadId, FbId, FbMcId, FbnId, ImuxId, IpadId, PTermId};

use crate::{
    BufOe, Cdr, CdrReset, Ct, Fb, FbImux, FbInput, FbPin, Fbnand, GlobalSig, IBuf, InputNode,
    InputNodeKind, IpadFb, IpadFbPin, Macrocell, Node, NodeId, NodeIoKind, NodeKind, OBuf, PTerm,
    Pla, Signal, Srff, Uim, Ut, Vm6,
};

#[derive(Debug)]
pub enum ParseErrorKind {
    MissingSig,
    MissingVerb,
    MalformedVerb,
    UnknownVerb,
    InconsistentData,
    UnknownKind,
    RedefinedInstance,
    UnknownMc,
    UnknownIBuf,
    UnknownOBuf,
    UnknownFb,
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: u32,
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let desc = match self.kind {
            ParseErrorKind::MissingSig => "missing signature",
            ParseErrorKind::MissingVerb => "missing verb",
            ParseErrorKind::MalformedVerb => "malformed verb",
            ParseErrorKind::UnknownVerb => "unknown verb",
            ParseErrorKind::InconsistentData => "inconsistent data",
            ParseErrorKind::UnknownKind => "unknown node kind",
            ParseErrorKind::RedefinedInstance => "redefined instance",
            ParseErrorKind::UnknownMc => "unknown mc",
            ParseErrorKind::UnknownIBuf => "unknown ibuf",
            ParseErrorKind::UnknownOBuf => "unknown obuf",
            ParseErrorKind::UnknownFb => "unknown fb",
        };
        write!(f, "parse error in line {}: {}", self.line, desc)
    }
}

struct Parser<'a> {
    lines: Vec<(u32, &'a str)>,
    line: usize,
    errline: u32,
}

impl<'a> Parser<'a> {
    fn error<T>(&self, kind: ParseErrorKind) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.errline,
            kind,
        })
    }

    fn get_raw_line(&mut self) -> Option<&'a str> {
        if self.line == self.lines.len() {
            None
        } else {
            let (el, res) = self.lines[self.line];
            self.errline = el;
            self.line += 1;
            Some(res)
        }
    }

    fn peek_raw_line(&mut self) -> Option<&'a str> {
        if self.line == self.lines.len() {
            None
        } else {
            let (el, res) = self.lines[self.line];
            self.errline = el;
            Some(res)
        }
    }

    fn peek_verb(&mut self, t: &str) -> bool {
        let Some(line) = self.peek_line() else {return false;};
        line[0] == t
    }

    fn get_line(&mut self) -> Option<Vec<&'a str>> {
        let line = self.get_raw_line()?;
        Some(line.split(" | ").collect())
    }

    fn peek_line(&mut self) -> Option<Vec<&'a str>> {
        let line = self.peek_raw_line()?;
        Some(line.split(" | ").collect())
    }

    fn parse_u32(&self, s: &str) -> Result<u32, ParseError> {
        match s.parse() {
            Ok(x) => Ok(x),
            Err(_) => self.error(ParseErrorKind::MalformedVerb),
        }
    }

    fn parse_usize(&self, s: &str) -> Result<usize, ParseError> {
        match s.parse() {
            Ok(x) => Ok(x),
            Err(_) => self.error(ParseErrorKind::MalformedVerb),
        }
    }

    fn parse_bool(&self, s: &str) -> Result<bool, ParseError> {
        match self.parse_u32(s)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => self.error(ParseErrorKind::MalformedVerb),
        }
    }

    fn parse_fbid(&self, s: &str) -> Result<FbId, ParseError> {
        let Some(s) = s.strip_prefix("FOOBAR") else {
            self.error(ParseErrorKind::UnknownFb)?
        };
        let Some(s) = s.strip_suffix('_') else {
            self.error(ParseErrorKind::UnknownFb)?
        };
        let n = self.parse_usize(s)?;
        if n == 0 {
            self.error(ParseErrorKind::UnknownFb)?
        }
        Ok(FbId::from_idx(n - 1))
    }

    fn parse_input_node(
        &mut self,
        nodes: &mut EntityMap<NodeId, u32, Node>,
    ) -> Result<InputNode, ParseError> {
        let Some(line) = self.get_line() else {
            self.error(ParseErrorKind::MissingVerb)?
        };
        if line[0] != "INPUT_NODE_TYPE" || line.len() != 4 {
            self.error(ParseErrorKind::MalformedVerb)?
        }
        let kind = match (line[2], line[1], line[3]) {
            ("5", "0", "II_IN") => InputNodeKind::IiIn,
            ("6", "0", "OI_IN") => InputNodeKind::OiIn,
            ("6", "2", "OI_OE") => InputNodeKind::OiOe,
            ("8", "0", "SRFF_D") => InputNodeKind::SrffD,
            ("8", "1", "SRFF_C") => InputNodeKind::SrffC,
            ("8", "2", "SRFF_S") => InputNodeKind::SrffS,
            ("8", "3", "SRFF_R") => InputNodeKind::SrffR,
            ("8", "4", "SRFF_CE") => InputNodeKind::SrffCe,
            ("10", "0", "CTOR_UNKNOWN") => InputNodeKind::CtorUnknown,
            ("100", "1", "NOTYPE") => InputNodeKind::None,
            _ => self.error(ParseErrorKind::UnknownKind)?,
        };
        let node = self.parse_node(nodes)?;
        Ok(InputNode { kind, node })
    }

    fn parse_output_node(
        &mut self,
        nodes: &mut EntityMap<NodeId, u32, Node>,
    ) -> Result<NodeId, ParseError> {
        let Some(line) = self.get_line() else {
            self.error(ParseErrorKind::MissingVerb)?
        };
        if line[0] != "OUTPUT_NODE_TYPE" || line.len() != 4 {
            self.error(ParseErrorKind::MalformedVerb)?
        }
        let kind = self.get_node_kind(line[2], line[1], line[3])?;
        let nid = self.parse_node(nodes)?;
        let node = &nodes[nid];
        if node.kind != kind {
            self.error(ParseErrorKind::InconsistentData)?;
        }
        Ok(nid)
    }

    fn get_node_kind(&self, a: &str, b: &str, c: &str) -> Result<NodeKind, ParseError> {
        match (a, b, c) {
            ("0", "0", "MC_Q") => Ok(NodeKind::McQ),
            ("0", "1", "MC_UIM") => Ok(NodeKind::McUim),
            ("0", "2", "MC_OE") => Ok(NodeKind::McOe),
            ("0", "4", "MC_EXPORT") => Ok(NodeKind::McExport),
            ("0", "5", "MC_FBK") => Ok(NodeKind::McFbk),
            ("0", "6", "MC_COMB") => Ok(NodeKind::McComb),
            ("4", "0", "UIM_OUT") => Ok(NodeKind::UimOut),
            ("5", "0", "II_IMUX") => Ok(NodeKind::IiImux),
            ("5", "2", "II_FOE") => Ok(NodeKind::IiFoe),
            ("5", "3", "II_FCLK") => Ok(NodeKind::IiFclk),
            ("5", "6", "II_FSR") => Ok(NodeKind::IiFsr),
            ("5", "7", "II_FCLKINV") => Ok(NodeKind::IiFclkInv),
            ("5", "8", "II_FOEINV") => Ok(NodeKind::IiFoeInv),
            ("5", "9", "II_FSRINV") => Ok(NodeKind::IiFsrInv),
            ("5", "10", "II_REG") => Ok(NodeKind::IiReg),
            ("6", "0", "OI_OUT") => Ok(NodeKind::OiOut),
            ("7", "0", "ALU_F") => Ok(NodeKind::AluF),
            ("8", "0", "SRFF_Q") => Ok(NodeKind::SrffQ),
            ("9", "1", "MC_SI_D1") => Ok(NodeKind::McSiD1),
            ("9", "2", "MC_SI_D2") => Ok(NodeKind::McSiD2),
            ("9", "3", "MC_SI_CLKF") => Ok(NodeKind::McSiClkf),
            ("9", "4", "MC_SI_TRST") => Ok(NodeKind::McSiTrst),
            ("9", "5", "MC_SI_SETF") => Ok(NodeKind::McSiSetf),
            ("9", "6", "MC_SI_RSTF") => Ok(NodeKind::McSiRstf),
            ("9", "7", "MC_SI_EXPORT") => Ok(NodeKind::McSiExport),
            ("9", "10", "MC_SI_CE") => Ok(NodeKind::McSiCe),
            ("10", "0", "BUF_OUT") => Ok(NodeKind::BufOut),
            ("12", "0", "CT_SI0") => Ok(NodeKind::CtSi0),
            ("12", "1", "CT_SI1") => Ok(NodeKind::CtSi1),
            ("12", "2", "CT_SI2") => Ok(NodeKind::CtSi2),
            ("12", "3", "CT_SI3") => Ok(NodeKind::CtSi3),
            ("12", "4", "CT_SI4") => Ok(NodeKind::CtSi4),
            ("12", "5", "CT_SI5") => Ok(NodeKind::CtSi5),
            ("12", "6", "CT_SI6") => Ok(NodeKind::CtSi6),
            ("12", "7", "CT_SI7") => Ok(NodeKind::CtSi7),
            ("13", "0", "FBN_OUT") => Ok(NodeKind::FbnOut),
            ("100", "0", "NOTYPE") => Ok(NodeKind::None),
            _ => self.error(ParseErrorKind::UnknownKind)?,
        }
    }

    fn parse_node(
        &mut self,
        nodes: &mut EntityMap<NodeId, u32, Node>,
    ) -> Result<NodeId, ParseError> {
        let Some(line) = self.get_line() else {
            self.error(ParseErrorKind::MissingVerb)?
        };
        let (is_signal, line) = if line[0] == "SIGNAL" {
            (true, &line[1..])
        } else {
            (false, &line[..])
        };
        if line[0] != "NODE" || line.len() != 13 || line[4] != "0" || line[7] != "NULL" {
            self.error(ParseErrorKind::MalformedVerb)?
        }
        let name = line[1].to_string();
        let index = self.parse_u32(line[2])?;
        let io_kind = match line[3] {
            "?" => NodeIoKind::None,
            "PI" => NodeIoKind::Input,
            "PO" => NodeIoKind::Output,
            "PIPO" => NodeIoKind::Inout,
            _ => self.error(ParseErrorKind::UnknownKind)?,
        };
        let flags = self.parse_u32(line[5])?;
        let module = line[6].to_string();
        let copy_of = if line[8] == "NULL" {
            None
        } else {
            Some(line[8].to_string())
        };
        let driver = if line[9] == "NULL" {
            None
        } else {
            Some(line[9].to_string())
        };
        let kind = self.get_node_kind(line[11], line[10], line[12])?;

        let mut terms = vec![];
        if is_signal {
            while self.peek_verb("SPPTERM") {
                match self.parse_pterm()? {
                    None => {
                        if !terms.is_empty() {
                            self.error(ParseErrorKind::MalformedVerb)?;
                        }
                        break;
                    }
                    Some(term) => terms.push(term),
                }
            }
        }

        let node = Node {
            is_signal,
            name,
            io_kind,
            flags,
            module,
            copy_of,
            driver,
            kind,
            terms,
        };

        if let Some((nid, cur)) = nodes.get(&index) {
            if *cur != node {
                eprintln!("NODE MISMATCH {cur:#?} {node:#?}");
                self.error(ParseErrorKind::InconsistentData)?;
            }
            Ok(nid)
        } else {
            Ok(nodes.insert(index, node).0)
        }
    }

    fn parse_pterm(&mut self) -> Result<Option<PTerm>, ParseError> {
        let Some(line) = self.get_line() else {
            self.error(ParseErrorKind::MissingVerb)?
        };
        if line[0] != "SPPTERM" || line.len() < 2 {
            self.error(ParseErrorKind::MalformedVerb)?
        }
        let num = self.parse_usize(line[1])?;
        if num == 0 {
            if line.len() != 3 {
                self.error(ParseErrorKind::MalformedVerb)?
            }
            match line[2] {
                "IV_ZERO" => Ok(None),
                "IV_DC" => Ok(Some(PTerm { inputs: vec![] })),
                _ => self.error(ParseErrorKind::MalformedVerb)?,
            }
        } else {
            if line.len() != 2 + num * 2 {
                self.error(ParseErrorKind::MalformedVerb)?
            }
            let mut inputs = vec![];
            for i in 0..num {
                let polarity = match line[2 + i * 2] {
                    "IV_TRUE" => true,
                    "IV_FALSE" => false,
                    _ => self.error(ParseErrorKind::MalformedVerb)?,
                };
                inputs.push((polarity, line[2 + i * 2 + 1].to_string()));
            }
            Ok(Some(PTerm { inputs }))
        }
    }

    fn parse_gsigs<T: EntityId>(
        &mut self,
        line: Vec<&str>,
    ) -> Result<EntityPartVec<T, GlobalSig>, ParseError> {
        if line.len() % 3 != 1 {
            self.error(ParseErrorKind::MalformedVerb)?;
        }
        let mut res = EntityPartVec::new();
        for i in 0..(line.len() / 3) {
            let name = line[i * 3 + 1].to_string();
            let slot = T::from_idx(self.parse_usize(line[i * 3 + 2])?);
            let path = self.parse_u32(line[i * 3 + 3])?;
            res.insert(slot, GlobalSig { name, path });
        }
        Ok(res)
    }

    fn parse_gsig(&mut self, line: Vec<&str>) -> Result<String, ParseError> {
        if line.len() != 4 {
            self.error(ParseErrorKind::MalformedVerb)?;
        }
        let name = line[1].to_string();
        if line[2] != "0" || line[3] != "0" {
            self.error(ParseErrorKind::MalformedVerb)?;
        }
        Ok(name)
    }
}

pub fn parse(s: &str) -> Result<Vm6, ParseError> {
    let mut lines = vec![];
    for (i, l) in s.lines().enumerate() {
        if !l.is_empty() {
            lines.push((i as u32 + 1, l));
        }
    }
    let mut parser = Parser {
        lines,
        line: 0,
        errline: 0,
    };

    let Some(sig) = parser.get_raw_line() else {
        parser.error(ParseErrorKind::MissingSig)?
    };
    let Some(nds_version) = sig.strip_prefix("NDS Database:  version ") else {
        parser.error(ParseErrorKind::MissingSig)?
    };
    let nds_version = nds_version.to_string();

    let Some(nds_info) = parser.get_line() else {
        parser.error(ParseErrorKind::MissingVerb)?
    };
    if nds_info[0] != "NDS_INFO" {
        parser.error(ParseErrorKind::MissingVerb)?
    }
    if nds_info.len() != 4 {
        parser.error(ParseErrorKind::MalformedVerb)?;
    }
    let family = nds_info[1].to_string();
    let devpkg = nds_info[2].to_string();
    let part = nds_info[3].to_string();

    let Some(device) = parser.get_line() else {
        parser.error(ParseErrorKind::MissingVerb)?
    };
    if device[0] != "DEVICE" {
        parser.error(ParseErrorKind::MissingVerb)?
    }
    if device.len() != 4 || !device[3].is_empty() {
        parser.error(ParseErrorKind::MalformedVerb)?;
    }
    let dev = device[1].to_string();
    if device[2] != devpkg {
        parser.error(ParseErrorKind::InconsistentData)?;
    }

    let Some(network) = parser.get_line() else {
        parser.error(ParseErrorKind::MissingVerb)?
    };
    if network[0] != "NETWORK" {
        parser.error(ParseErrorKind::MissingVerb)?
    }
    if !matches!(network.len(), 5 | 6) || network[2] != "0" || network[3] != "0" {
        parser.error(ParseErrorKind::MalformedVerb)?;
    }
    let network_name = network[1].to_string();
    let network_flags = parser.parse_u32(network[4])?;
    let network_flags2 = if network.len() == 6 {
        Some(parser.parse_u32(network[5])?)
    } else {
        None
    };

    let mut res = Vm6 {
        nds_version,
        family,
        dev,
        devpkg,
        part,
        network_name,
        network_flags,
        network_flags2,
        nodes: Default::default(),
        ibufs: Default::default(),
        macrocells: Default::default(),
        obufs: Default::default(),
        uims: Default::default(),
        fbs: Default::default(),
        ipad_fb: None,
        global_fclk: EntityPartVec::new(),
        global_fsr: None,
        global_foe: EntityPartVec::new(),
        dge: None,
        iostd_default: None,
        iostd: Default::default(),
        utc: enum_map! {_ => None},
        cdr: None,
        prohibit_pins: Default::default(),
        vref: Default::default(),
    };

    while let Some(line) = parser.get_line() {
        match line[0] {
            "INPUT_INSTANCE" => {
                if line.len() != 9 || line[1] != "0" || line[2] != "0" || line[3] != "NULL" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[4].to_string();
                let module = line[5].to_string();
                let flags = parser.parse_u32(line[6])?;
                let ni = parser.parse_usize(line[7])?;
                let no = parser.parse_usize(line[8])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                res.ibufs.insert(
                    name,
                    IBuf {
                        module,
                        flags,
                        inodes,
                        onodes,
                    },
                );
            }
            "OUTPUT_INSTANCE" => {
                if line.len() != 7 || line[1] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[2].to_string();
                let module = line[3].to_string();
                let flags = parser.parse_u32(line[4])?;
                let ni = parser.parse_usize(line[5])?;
                let no = parser.parse_usize(line[6])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                res.obufs.insert(
                    name,
                    OBuf {
                        module,
                        flags,
                        inodes,
                        onodes,
                    },
                );
            }
            "MACROCELL_INSTANCE" => {
                if line.len() != 7 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[2].to_string();
                let module = line[3].to_string();
                let flags = parser.parse_u32(line[4])?;
                let ni = parser.parse_usize(line[5])?;
                let no = parser.parse_usize(line[6])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                res.macrocells.insert(
                    name,
                    Macrocell {
                        module,
                        flags,
                        inodes,
                        onodes,
                        signal: None,
                        srff: None,
                        bufoe: None,
                    },
                );
            }
            "SIGNAL_INSTANCE" => {
                if line.len() != 6 || line[3] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[1].to_string();
                let Some((_, mc)) = res.macrocells.get_mut(line[2]) else {
                    parser.error(ParseErrorKind::UnknownMc)?
                };
                if mc.signal.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                let ni = parser.parse_usize(line[4])?;
                let no = parser.parse_usize(line[5])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                mc.signal = Some(Signal {
                    name,
                    inodes,
                    onodes,
                });
            }
            "SRFF_INSTANCE" => {
                if line.len() != 6 || line[3] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[1].to_string();
                let Some((_, mc)) = res.macrocells.get_mut(line[2]) else {
                    parser.error(ParseErrorKind::UnknownMc)?
                };
                if mc.srff.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                let ni = parser.parse_usize(line[4])?;
                let no = parser.parse_usize(line[5])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                mc.srff = Some(Srff {
                    name,
                    inodes,
                    onodes,
                });
            }
            "BUF_INSTANCE" => {
                if line.len() != 6 || line[3] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[1].to_string();
                let Some((_, mc)) = res.macrocells.get_mut(line[2]) else {
                    parser.error(ParseErrorKind::UnknownMc)?
                };
                if mc.bufoe.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                let ni = parser.parse_usize(line[4])?;
                let no = parser.parse_usize(line[5])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                mc.bufoe = Some(BufOe {
                    name,
                    inodes,
                    onodes,
                });
            }
            "UIM_INSTANCE" => {
                if line.len() != 6 || line[3] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[1].to_string();
                let module = line[2].to_string();
                let ni = parser.parse_usize(line[4])?;
                let no = parser.parse_usize(line[5])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                let term = parser.parse_pterm()?;
                let Some(term) = term else {
                    parser.error(ParseErrorKind::MalformedVerb)?
                };
                res.uims.insert(
                    name,
                    Uim {
                        module,
                        inodes,
                        onodes,
                        term,
                    },
                );
            }
            "FB_INSTANCE" => {
                if line.len() != 6 || line[3] != "0" || line[4] != "0" || line[5] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let name = line[1].to_string();
                let module = line[2].to_string();
                if name.starts_with("INPUTPINS") {
                    let mut pins = EntityPartVec::new();
                    while parser.peek_verb("FBPIN") {
                        let line = parser.get_line().unwrap();
                        if !matches!(line.len(), 8 | 10) {
                            parser.error(ParseErrorKind::MalformedVerb)?;
                        }
                        let index = parser.parse_usize(line[1])? - 1;
                        assert_eq!(line[2], "NULL");
                        assert_eq!(line[3], "0");
                        let ibuf = match line[4] {
                            "NULL" => None,
                            name => {
                                let Some((ibuf, _)) = res.ibufs.get(name) else {
                                parser.error(ParseErrorKind::UnknownIBuf)?
                            };
                                Some(ibuf)
                            }
                        };
                        let ibuf_used = parser.parse_bool(line[5])?;
                        assert_eq!(line[6], "NULL");
                        assert_eq!(line[7], "0");
                        let pad = if line.len() == 8 {
                            None
                        } else {
                            Some((line[8].to_string(), parser.parse_u32(line[9])?))
                        };
                        pins.insert(
                            IpadId::from_idx(index),
                            IpadFbPin {
                                ibuf,
                                pad,
                                ibuf_used,
                            },
                        );
                    }
                    assert!(res.ipad_fb.is_none());
                    res.ipad_fb = Some(IpadFb { module, pins });
                    assert_eq!(
                        name,
                        format!("INPUTPINS_FOOBAR{idx}_", idx = res.fbs.len() + 1)
                    );
                } else {
                    let mut pins = EntityPartVec::new();
                    while parser.peek_verb("FBPIN") {
                        let line = parser.get_line().unwrap();
                        if !matches!(line.len(), 8 | 10) {
                            parser.error(ParseErrorKind::MalformedVerb)?;
                        }
                        let index = parser.parse_usize(line[1])? - 1;
                        let mc = match line[2] {
                            "NULL" => None,
                            name => {
                                let Some((mc, _)) = res.macrocells.get(name) else {
                                parser.error(ParseErrorKind::UnknownMc)?
                            };
                                Some(mc)
                            }
                        };
                        let mc_used = parser.parse_bool(line[3])?;
                        let ibuf = match line[4] {
                            "NULL" => None,
                            name => {
                                let Some((ibuf, _)) = res.ibufs.get(name) else {
                                parser.error(ParseErrorKind::UnknownIBuf)?
                            };
                                Some(ibuf)
                            }
                        };
                        let ibuf_used = parser.parse_bool(line[5])?;
                        let obuf = match line[6] {
                            "NULL" => None,
                            name => {
                                let Some((obuf, _)) = res.obufs.get(name) else {
                                parser.error(ParseErrorKind::UnknownOBuf)?
                            };
                                Some(obuf)
                            }
                        };
                        let obuf_used = parser.parse_bool(line[7])?;
                        let pad = if line.len() == 8 {
                            None
                        } else {
                            Some((line[8].to_string(), parser.parse_u32(line[9])?))
                        };
                        pins.insert(
                            FbMcId::from_idx(index),
                            FbPin {
                                mc,
                                ibuf,
                                obuf,
                                pad,
                                mc_used,
                                ibuf_used,
                                obuf_used,
                            },
                        );
                    }
                    let fbid = res.fbs.push(Fb {
                        module,
                        pins,
                        inputs: vec![],
                        imux: EntityVec::new(),
                        ct: None,
                        pla: None,
                        fbnands: EntityPartVec::new(),
                        global_fclk: EntityVec::new(),
                    });
                    assert_eq!(name, format!("FOOBAR{idx}_", idx = fbid.to_idx() + 1));
                }
            }
            "FB_ORDER_OF_INPUTS" => {
                if line.len() % 3 != 2 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let fb = &mut res.fbs[parser.parse_fbid(line[1])?];
                for i in 0..(line.len() / 3) {
                    let index = ImuxId::from_idx(parser.parse_usize(line[i * 3 + 2])?);
                    let name = line[i * 3 + 3].to_string();
                    let pad = match line[i * 3 + 4] {
                        "NULL" => None,
                        pn => Some(pn.to_string()),
                    };
                    fb.inputs.push(FbInput { index, name, pad });
                }
            }
            "FB_IMUX_INDEX" => {
                if line.len() < 3 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let fb = &mut res.fbs[parser.parse_fbid(line[1])?];
                if !fb.imux.is_empty() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                for &n in &line[2..] {
                    let imux = match n {
                        "-1" => FbImux::None,
                        "999" => FbImux::WireAnd,
                        _ => FbImux::Plain(parser.parse_u32(n)?),
                    };
                    fb.imux.push(imux);
                }
            }
            "CT_INSTANCE" => {
                if line.len() != 7 || line[4] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let fb = &mut res.fbs[parser.parse_fbid(line[1])?];
                if fb.ct.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                let name = line[2].to_string();
                let module = line[3].to_string();
                let ni = parser.parse_usize(line[5])?;
                let no = parser.parse_usize(line[6])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                let mut invs = HashSet::new();
                while parser.peek_verb("CT_NODE_INV") {
                    let line = parser.get_line().unwrap();
                    if line.len() != 4 {
                        parser.error(ParseErrorKind::MalformedVerb)?;
                    }
                    let kind = parser.get_node_kind(line[2], line[1], line[3])?;
                    invs.insert(kind);
                }
                fb.ct = Some(Ct {
                    name,
                    module,
                    inodes,
                    onodes,
                    invs,
                });
            }
            "PLA" => {
                if line.len() != 3 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let fb = &mut res.fbs[parser.parse_fbid(line[1])?];
                if fb.pla.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                let num = parser.parse_usize(line[2])?;
                let mut terms = EntityPartVec::new();
                for _ in 0..num {
                    let Some(line) = parser.get_line() else {
                        parser.error(ParseErrorKind::MissingVerb)?
                    };
                    if line[0] != "PLA_TERM" || line.len() != 3 || !line[2].is_empty() {
                        parser.error(ParseErrorKind::MalformedVerb)?;
                    }
                    let index = PTermId::from_idx(parser.parse_usize(line[1])?);
                    let term = parser.parse_pterm()?;
                    let Some(term) = term else {
                        parser.error(ParseErrorKind::MalformedVerb)?
                    };
                    terms.insert(index, term);
                }
                fb.pla = Some(Pla { terms });
            }
            "FBNAND_INSTANCE" => {
                if line.len() != 8 || line[5] != "0" {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let fb = &mut res.fbs[parser.parse_fbid(line[1])?];
                let index = parser.parse_usize(line[2])?;
                let name = line[3].to_string();
                let module = line[4].to_string();
                let ni = parser.parse_usize(line[6])?;
                let no = parser.parse_usize(line[7])?;
                let mut inodes = vec![];
                for _ in 0..ni {
                    inodes.push(parser.parse_input_node(&mut res.nodes)?);
                }
                let mut onodes = vec![];
                for _ in 0..no {
                    onodes.push(parser.parse_output_node(&mut res.nodes)?);
                }
                let term = parser.parse_pterm()?;
                let Some(term) = term else {
                    parser.error(ParseErrorKind::MalformedVerb)?
                };
                fb.fbnands.insert(
                    FbnId::from_idx(index),
                    Fbnand {
                        name,
                        module,
                        inodes,
                        onodes,
                        term,
                    },
                );
            }
            "GLOBAL_FCLK" => {
                let sigs = parser.parse_gsigs(line)?;
                if res.global_fclk.iter().next().is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                res.global_fclk = sigs;
            }
            "GLOBAL_FSR" => {
                let name = parser.parse_gsig(line)?;
                if res.global_fsr.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                res.global_fsr = Some(name);
            }
            "GLOBAL_FOE" => {
                let sigs = parser.parse_gsigs(line)?;
                if res.global_foe.iter().next().is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                res.global_foe = sigs;
            }
            "DGE" => {
                let name = parser.parse_gsig(line)?;
                if res.dge.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                res.dge = Some(name);
            }
            "UTC" => {
                if line.len() % 2 != 1 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                if res.utc.values().any(Option::is_some) {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                for i in 0..(line.len() / 2) {
                    let name = line[i * 2 + 1].to_string();
                    let slot = parser.parse_u32(line[i * 2 + 2])?;
                    let slot = match slot {
                        0 => Ut::Clk,
                        1 => Ut::Oe,
                        2 => Ut::Set,
                        3 => Ut::Rst,
                        _ => parser.error(ParseErrorKind::MalformedVerb)?,
                    };
                    res.utc[slot] = Some(name);
                }
            }
            "GLOBAL_FCLK_IDX" => {
                if line.len() < 3 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                let fb = &mut res.fbs[parser.parse_fbid(line[1])?];
                if !fb.global_fclk.is_empty() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                for &n in &line[2..] {
                    let n = match n {
                        "-1" => None,
                        _ => Some(ClkPadId::from_idx(parser.parse_usize(n)?)),
                    };
                    fb.global_fclk.push(n);
                }
            }
            "IOSTD" => {
                if line.len() != 2 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                if res.iostd_default.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                res.iostd_default = Some(line[1].to_string());
                loop {
                    let Some(nl) = parser.peek_line() else { break; };
                    if nl.len() != 2 {
                        break;
                    }
                    parser.get_line();
                    res.iostd.insert(nl[0].to_string(), nl[1].to_string());
                }
            }
            "PROHIBIT_PIN" => {
                if line.len() != 2 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                res.prohibit_pins.insert(line[1].to_string());
            }
            "VREF" => {
                for n in &line[1..] {
                    res.vref.insert(n.to_string());
                }
            }
            "CDR" => {
                if line.len() != 4 {
                    parser.error(ParseErrorKind::MalformedVerb)?;
                }
                if res.cdr.is_some() {
                    parser.error(ParseErrorKind::RedefinedInstance)?;
                }
                let reset = if line[1] == "<assigned>" {
                    let Some((ibuf, _)) = res.ibufs.get(line[2]) else {
                        parser.error(ParseErrorKind::UnknownIBuf)?
                    };
                    CdrReset::Used(ibuf)
                } else if line[2] == "<prohibited>" {
                    CdrReset::Unused(line[1].to_string())
                } else {
                    parser.error(ParseErrorKind::MalformedVerb)?
                };
                let div = parser.parse_u32(line[3])?;
                res.cdr = Some(Cdr { reset, div });
            }
            "BUSINFO" => {
                // eh.
            }
            _ => parser.error(ParseErrorKind::UnknownVerb)?,
        }
    }

    Ok(res)
}
