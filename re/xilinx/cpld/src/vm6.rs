mod parser;
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Write},
};

use crate::types::{ClkPadId, FbnId, FclkId, FoeId, ImuxId, Ut};
use enum_map::EnumMap;
pub use parser::{ParseError, ParseErrorKind};
use prjcombine_types::cpld::{BlockId, IoCoord, IpadId, MacrocellId, ProductTermId};
use unnamed_entity::{EntityId, EntityMap, EntityPartVec, EntityVec, entity_id};

entity_id! {
    pub id NodeId u32, reserve 1;
    pub id IBufId u32, reserve 1;
    pub id Vm6MacrocellId u32, reserve 1;
    pub id OBufId u32, reserve 1;
    pub id UimId u32, reserve 1;
}

#[derive(Debug, Clone)]
pub struct Vm6 {
    pub nds_version: String,
    pub family: String,
    pub dev: String,
    pub devpkg: String,
    pub part: String,
    pub network_name: String,
    pub network_flags: u32,
    pub network_flags2: Option<u32>,
    pub nodes: EntityMap<NodeId, u32, Node>,
    pub ibufs: EntityMap<IBufId, String, IBuf>,
    pub macrocells: EntityMap<Vm6MacrocellId, String, Macrocell>,
    pub obufs: EntityMap<OBufId, String, OBuf>,
    pub uims: EntityMap<UimId, String, Uim>,
    pub fbs: EntityVec<BlockId, Fb>,
    pub ipad_fb: Option<IpadFb>,
    pub global_fclk: EntityPartVec<FclkId, GlobalSig>,
    pub global_fsr: Option<String>,
    pub global_foe: EntityPartVec<FoeId, GlobalSig>,
    pub dge: Option<String>,
    pub iostd_default: Option<String>,
    pub iostd: HashMap<String, String>,
    pub utc: EnumMap<Ut, Option<String>>,
    pub cdr: Option<Cdr>,
    pub prohibit_pins: HashSet<String>,
    pub vref: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct IBuf {
    pub module: String,
    pub flags: u32,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct OBuf {
    pub module: String,
    pub flags: u32,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct Macrocell {
    pub module: String,
    pub flags: u32,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
    pub signal: Option<Signal>,
    pub srff: Option<Srff>,
    pub bufoe: Option<BufOe>,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub name: String,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PTerm {
    pub inputs: Vec<(bool, String)>,
}

#[derive(Debug, Clone)]
pub struct Srff {
    pub name: String,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct BufOe {
    pub name: String,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Node {
    pub is_signal: bool,
    pub name: String,
    pub io_kind: NodeIoKind,
    pub flags: u32,
    pub module: String,
    pub copy_of: Option<String>,
    pub driver: Option<String>,
    pub kind: NodeKind,
    pub terms: Vec<PTerm>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum NodeIoKind {
    None,
    Input,
    Output,
    Inout,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NodeKind {
    None,
    McQ,
    McUim,
    McOe,
    McExport,
    McFbk,
    McComb,
    McGlb,
    UimOut,
    IiImux,
    IiFoe,
    IiFclk,
    IiFsr,
    IiFoeInv,
    IiFclkInv,
    IiFsrInv,
    IiReg,
    OiOut,
    AluF,
    SrffQ,
    McSiD1,
    McSiD2,
    McSiClkf,
    McSiTrst,
    McSiSetf,
    McSiRstf,
    McSiExport,
    McSiCe,
    BufOut,
    CtSi0,
    CtSi1,
    CtSi2,
    CtSi3,
    CtSi4,
    CtSi5,
    CtSi6,
    CtSi7,
    FbnOut,
}

#[derive(Debug, Clone)]
pub struct InputNode {
    pub kind: InputNodeKind,
    pub node: NodeId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InputNodeKind {
    None,
    IiIn,
    OiIn,
    OiOe,
    SrffD,
    SrffC,
    SrffS,
    SrffR,
    SrffCe,
    CtorUnknown,
}

#[derive(Debug, Clone)]
pub struct Uim {
    pub module: String,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
    pub term: PTerm,
}

#[derive(Debug, Clone)]
pub struct Fb {
    pub module: String,
    pub pins: EntityPartVec<MacrocellId, FbPin>,
    pub inputs: Vec<FbInput>,
    pub imux: EntityVec<ImuxId, FbImux>,
    pub ct: Option<Ct>,
    pub pla: Option<Pla>,
    pub fbnands: EntityPartVec<FbnId, Fbnand>,
    pub global_fclk: EntityVec<FclkId, Option<ClkPadId>>,
}

#[derive(Debug, Clone)]
pub struct IpadFb {
    pub module: String,
    pub pins: EntityPartVec<IpadId, IpadFbPin>,
}

#[derive(Debug, Clone)]
pub struct FbPin {
    pub mc: Option<Vm6MacrocellId>,
    pub ibuf: Option<IBufId>,
    pub obuf: Option<OBufId>,
    pub mc_used: bool,
    pub ibuf_used: bool,
    pub obuf_used: bool,
    pub pad: Option<(String, u32)>,
}

#[derive(Debug, Clone)]
pub struct IpadFbPin {
    pub ibuf: Option<IBufId>,
    pub ibuf_used: bool,
    pub pad: Option<(String, u32)>,
}

#[derive(Debug, Clone)]
pub struct FbInput {
    pub index: ImuxId,
    pub name: String,
    pub pad: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FbImux {
    None,
    Plain(u32),
    WireAnd,
}

#[derive(Debug, Clone)]
pub struct Ct {
    pub name: String,
    pub module: String,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
    pub invs: HashSet<NodeKind>,
}

#[derive(Debug, Clone)]
pub struct Fbnand {
    pub name: String,
    pub module: String,
    pub inodes: Vec<InputNode>,
    pub onodes: Vec<NodeId>,
    pub term: PTerm,
}

#[derive(Debug, Clone)]
pub struct Pla {
    pub terms: EntityPartVec<ProductTermId, PTerm>,
}

#[derive(Debug, Clone)]
pub struct GlobalSig {
    pub name: String,
    pub path: u32,
}

#[derive(Debug, Clone)]
pub struct Cdr {
    pub reset: CdrReset,
    pub div: u32,
}

#[derive(Debug, Clone)]
pub enum CdrReset {
    Used(IBufId),
    Unused(String),
}

fn print_node_type(t: NodeKind) -> (u32, u32, &'static str) {
    match t {
        NodeKind::None => (100, 0, "NOTYPE"),
        NodeKind::McQ => (0, 0, "MC_Q"),
        NodeKind::McUim => (0, 1, "MC_UIM"),
        NodeKind::McOe => (0, 2, "MC_OE"),
        NodeKind::McExport => (0, 4, "MC_EXPORT"),
        NodeKind::McFbk => (0, 5, "MC_FBK"),
        NodeKind::McComb => (0, 6, "MC_COMB"),
        NodeKind::McGlb => (0, 7, "MC_GLB"),
        NodeKind::UimOut => (4, 0, "UIM_OUT"),
        NodeKind::IiImux => (5, 0, "II_IMUX"),
        NodeKind::IiFoe => (5, 2, "II_FOE"),
        NodeKind::IiFclk => (5, 3, "II_FCLK"),
        NodeKind::IiFsr => (5, 6, "II_FSR"),
        NodeKind::IiFclkInv => (5, 7, "II_FCLKINV"),
        NodeKind::IiFoeInv => (5, 8, "II_FOEINV"),
        NodeKind::IiFsrInv => (5, 9, "II_FSRINV"),
        NodeKind::IiReg => (5, 10, "II_REG"),
        NodeKind::OiOut => (6, 0, "OI_OUT"),
        NodeKind::AluF => (7, 0, "ALU_F"),
        NodeKind::SrffQ => (8, 0, "SRFF_Q"),
        NodeKind::McSiD1 => (9, 1, "MC_SI_D1"),
        NodeKind::McSiD2 => (9, 2, "MC_SI_D2"),
        NodeKind::McSiClkf => (9, 3, "MC_SI_CLKF"),
        NodeKind::McSiTrst => (9, 4, "MC_SI_TRST"),
        NodeKind::McSiSetf => (9, 5, "MC_SI_SETF"),
        NodeKind::McSiRstf => (9, 6, "MC_SI_RSTF"),
        NodeKind::McSiExport => (9, 7, "MC_SI_EXPORT"),
        NodeKind::McSiCe => (9, 10, "MC_SI_CE"),
        NodeKind::BufOut => (10, 0, "BUF_OUT"),
        NodeKind::CtSi0 => (12, 0, "CT_SI0"),
        NodeKind::CtSi1 => (12, 1, "CT_SI1"),
        NodeKind::CtSi2 => (12, 2, "CT_SI2"),
        NodeKind::CtSi3 => (12, 3, "CT_SI3"),
        NodeKind::CtSi4 => (12, 4, "CT_SI4"),
        NodeKind::CtSi5 => (12, 5, "CT_SI5"),
        NodeKind::CtSi6 => (12, 6, "CT_SI6"),
        NodeKind::CtSi7 => (12, 7, "CT_SI7"),
        NodeKind::FbnOut => (13, 0, "FBN_OUT"),
    }
}

fn print_mc_flags(flags: u32) -> String {
    let mut fs = vec![];
    for (f, n) in [
        (0x1, "LowPow"),
        (0x40, "Latch"),
        (0x80, "HDFB"),
        (0x100, "Inv"),
        (0x200, "PrldHigh"),
        (0x400, "PrldLow"),
        (0x800, "FbkInv"),
        (0x1000, "Tff"),
        (0x4000, "PinTrst"),
        (0x8000, "Merge"),
        (0x800000, "OptxMapped"),
        (0x2000000, "SoftPFbk"),
        (0x4000000, "ClkInv"),
        (0x10000000, "Ce"),
        (0x20000000, "Placed"),
    ] {
        if (flags & f) != 0 {
            fs.push(n);
        }
    }
    if fs.is_empty() {
        "NULL".to_string()
    } else {
        fs.join("+")
    }
}

impl Vm6 {
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        parser::parse(s)
    }

    pub fn get_ibuf_id(&self, io: IoCoord) -> Option<IBufId> {
        match io {
            IoCoord::Ipad(ip) => self.ipad_fb.as_ref()?.pins.get(ip)?.ibuf,
            IoCoord::Macrocell(mc) => self.fbs[mc.block].pins.get(mc.macrocell)?.ibuf,
        }
    }

    fn write_node(&self, f: &mut dyn Write, n: NodeId) -> fmt::Result {
        let &idx = self.nodes.key(n);
        let node = &self.nodes[n];
        let (b, a, c) = print_node_type(node.kind);
        if node.is_signal {
            write!(f, "SIGNAL | ")?;
        }
        let io_kind = match node.io_kind {
            NodeIoKind::None => "?",
            NodeIoKind::Input => "PI",
            NodeIoKind::Output => "PO",
            NodeIoKind::Inout => "PIPO",
        };
        writeln!(
            f,
            "NODE | {n} | {idx} | {io_kind} | 0 | {f} | {m} | NULL | {cof} | {drv} | {a} | {b} | {c}",
            n = node.name,
            f = node.flags,
            m = node.module,
            cof = match node.copy_of {
                None => "NULL",
                Some(ref s) => s,
            },
            drv = match node.driver {
                None => "NULL",
                Some(ref s) => s,
            },
        )?;
        if node.is_signal && node.terms.is_empty() {
            writeln!(f, "SPPTERM | 0 | IV_ZERO")?;
        }
        for term in &node.terms {
            self.write_pterm(f, term)?;
        }
        Ok(())
    }

    fn write_inode(&self, f: &mut dyn Write, n: &InputNode) -> fmt::Result {
        let (b, a, c) = match n.kind {
            InputNodeKind::None => (100, 1, "NOTYPE"),
            InputNodeKind::IiIn => (5, 0, "II_IN"),
            InputNodeKind::OiIn => (6, 0, "OI_IN"),
            InputNodeKind::OiOe => (6, 2, "OI_OE"),
            InputNodeKind::SrffD => (8, 0, "SRFF_D"),
            InputNodeKind::SrffC => (8, 1, "SRFF_C"),
            InputNodeKind::SrffS => (8, 2, "SRFF_S"),
            InputNodeKind::SrffR => (8, 3, "SRFF_R"),
            InputNodeKind::SrffCe => (8, 4, "SRFF_CE"),
            InputNodeKind::CtorUnknown => (10, 0, "CTOR_UNKNOWN"),
        };
        writeln!(f, "INPUT_NODE_TYPE | {a} | {b} | {c}")?;
        self.write_node(f, n.node)
    }

    fn write_onode(&self, f: &mut dyn Write, n: NodeId) -> fmt::Result {
        let node = &self.nodes[n];
        let (b, a, c) = print_node_type(node.kind);
        writeln!(f, "OUTPUT_NODE_TYPE | {a} | {b} | {c}")?;
        self.write_node(f, n)
    }

    fn write_pterm(&self, f: &mut dyn Write, term: &PTerm) -> fmt::Result {
        write!(f, "SPPTERM | {}", term.inputs.len())?;
        if term.inputs.is_empty() {
            write!(f, " | IV_DC")?;
        }
        for inp in &term.inputs {
            write!(
                f,
                " | {p} | {n}",
                p = match inp.0 {
                    false => "IV_FALSE",
                    true => "IV_TRUE",
                },
                n = inp.1,
            )?;
        }
        writeln!(f)?;
        Ok(())
    }

    pub fn write(&self, f: &mut dyn Write) -> fmt::Result {
        writeln!(f, "NDS Database:  version {}", self.nds_version)?;
        writeln!(f)?;
        writeln!(
            f,
            "NDS_INFO | {} | {} | {}",
            self.family, self.devpkg, self.part
        )?;
        writeln!(f)?;
        writeln!(f, "DEVICE | {} | {} | ", self.dev, self.devpkg)?;
        writeln!(f)?;
        write!(
            f,
            "NETWORK | {} | 0 | 0 | {}",
            self.network_name, self.network_flags
        )?;
        if let Some(flags) = self.network_flags2 {
            write!(f, " | {flags}")?;
        }
        writeln!(f)?;
        writeln!(f)?;

        for (_, name, ibuf) in &self.ibufs {
            writeln!(
                f,
                "INPUT_INSTANCE | 0 | 0 | NULL | {name} | {m} | {f} | {ni} | {no}",
                m = ibuf.module,
                f = ibuf.flags,
                ni = ibuf.inodes.len(),
                no = ibuf.onodes.len(),
            )?;
            for n in &ibuf.inodes {
                self.write_inode(f, n)?;
            }
            for &n in &ibuf.onodes {
                self.write_onode(f, n)?;
            }
            writeln!(f)?;
        }

        for (_, name, obuf) in &self.obufs {
            writeln!(
                f,
                "OUTPUT_INSTANCE | 0 | {name} | {m} | {f} | {ni} | {no}",
                m = obuf.module,
                f = obuf.flags,
                ni = obuf.inodes.len(),
                no = obuf.onodes.len(),
            )?;
            for n in &obuf.inodes {
                self.write_inode(f, n)?;
            }
            for &n in &obuf.onodes {
                self.write_onode(f, n)?;
            }
            writeln!(f)?;
        }

        for (_, name, mc) in &self.macrocells {
            let sf = print_mc_flags(mc.flags);
            writeln!(
                f,
                "MACROCELL_INSTANCE | {sf} | {name} | {m} | {f} | {ni} | {no}",
                m = mc.module,
                f = mc.flags,
                ni = mc.inodes.len(),
                no = mc.onodes.len(),
            )?;
            for n in &mc.inodes {
                self.write_inode(f, n)?;
            }
            for &n in &mc.onodes {
                self.write_onode(f, n)?;
            }
            writeln!(f)?;

            if let Some(ref signal) = mc.signal {
                writeln!(
                    f,
                    "SIGNAL_INSTANCE | {n} | {name} | 0 | {ni} | {no}",
                    n = signal.name,
                    ni = signal.inodes.len(),
                    no = signal.onodes.len(),
                )?;
                for n in &signal.inodes {
                    self.write_inode(f, n)?;
                }
                for &n in &signal.onodes {
                    self.write_onode(f, n)?;
                }
                writeln!(f)?;
            }
            if let Some(ref srff) = mc.srff {
                writeln!(
                    f,
                    "SRFF_INSTANCE | {n} | {name} | 0 | {ni} | {no}",
                    n = srff.name,
                    ni = srff.inodes.len(),
                    no = srff.onodes.len(),
                )?;
                for n in &srff.inodes {
                    self.write_inode(f, n)?;
                }
                for &n in &srff.onodes {
                    self.write_onode(f, n)?;
                }
                writeln!(f)?;
            }
            if let Some(ref bufoe) = mc.bufoe {
                writeln!(
                    f,
                    "BUF_INSTANCE | {n} | {name} | 0 | {ni} | {no}",
                    n = bufoe.name,
                    ni = bufoe.inodes.len(),
                    no = bufoe.onodes.len(),
                )?;
                for n in &bufoe.inodes {
                    self.write_inode(f, n)?;
                }
                for &n in &bufoe.onodes {
                    self.write_onode(f, n)?;
                }
                writeln!(f)?;
            }
        }

        for (_, name, uim) in &self.uims {
            writeln!(
                f,
                "UIM_INSTANCE | {name} | {m} | 0 | {ni} | {no}",
                m = uim.module,
                ni = uim.inodes.len(),
                no = uim.onodes.len(),
            )?;
            for n in &uim.inodes {
                self.write_inode(f, n)?;
            }
            for &n in &uim.onodes {
                self.write_onode(f, n)?;
            }
            self.write_pterm(f, &uim.term)?;
            writeln!(f)?;
        }

        for (fbid, fb) in &self.fbs {
            let idx = fbid.to_idx() + 1;
            writeln!(
                f,
                "FB_INSTANCE | FOOBAR{idx}_ | {m} | 0 | 0 | 0",
                m = fb.module
            )?;
            for (mcid, pin) in &fb.pins {
                let idx = mcid.to_idx() + 1;
                write!(
                    f,
                    "FBPIN | {idx} | {mc} | {mcu} | {ib} | {ibu} | {ob} | {obu}",
                    mc = match pin.mc {
                        None => "NULL",
                        Some(n) => self.macrocells.key(n),
                    },
                    mcu = u32::from(pin.mc_used),
                    ib = match pin.ibuf {
                        None => "NULL",
                        Some(n) => self.ibufs.key(n),
                    },
                    ibu = u32::from(pin.ibuf_used),
                    ob = match pin.obuf {
                        None => "NULL",
                        Some(n) => self.obufs.key(n),
                    },
                    obu = u32::from(pin.obuf_used),
                )?;
                if let Some((ref pad, flags)) = pin.pad {
                    write!(f, " | {pad} | {flags}")?;
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        if let Some(ref ipad_fb) = self.ipad_fb {
            let idx = self.fbs.len() + 1;
            writeln!(
                f,
                "FB_INSTANCE | INPUTPINS_FOOBAR{idx}_ | {m} | 0 | 0 | 0",
                m = ipad_fb.module
            )?;
            for (mcid, pin) in &ipad_fb.pins {
                let idx = mcid.to_idx() + 1;
                write!(
                    f,
                    "FBPIN | {idx} | NULL | 0 | {ib} | {ibu} | NULL | 0",
                    ib = match pin.ibuf {
                        None => "NULL",
                        Some(n) => self.ibufs.key(n),
                    },
                    ibu = u32::from(pin.ibuf_used),
                )?;
                if let Some((ref pad, flags)) = pin.pad {
                    write!(f, " | {pad} | {flags}")?;
                }
                writeln!(f)?;
            }
            writeln!(f)?;
        }

        for (fbid, fb) in &self.fbs {
            let idx = fbid.to_idx() + 1;
            if let Some(ref ct) = fb.ct {
                writeln!(
                    f,
                    "CT_INSTANCE | FOOBAR{idx}_ | {ctn} | {m} | 0 | {ni} | {no}",
                    ctn = ct.name,
                    m = ct.module,
                    ni = ct.inodes.len(),
                    no = ct.onodes.len()
                )?;
                for n in &ct.inodes {
                    self.write_inode(f, n)?;
                }
                for &n in &ct.onodes {
                    self.write_onode(f, n)?;
                }
                for &inv in &ct.invs {
                    let (b, a, c) = print_node_type(inv);
                    writeln!(f, "CT_NODE_INV | {a} | {b} | {c}")?;
                }
                writeln!(f)?;
            }
        }
        for (fbid, fb) in &self.fbs {
            let idx = fbid.to_idx() + 1;
            if let Some(ref pla) = fb.pla {
                writeln!(f, "PLA | FOOBAR{idx}_ | {n}", n = pla.terms.iter().count())?;
                for (ptid, term) in &pla.terms {
                    writeln!(f, "PLA_TERM | {} | ", ptid.to_idx())?;
                    self.write_pterm(f, term)?;
                }
                writeln!(f)?;
            }
        }
        for (fbid, fb) in &self.fbs {
            let fbidx = fbid.to_idx() + 1;
            for (fbnid, fbn) in &fb.fbnands {
                writeln!(
                    f,
                    "FBNAND_INSTANCE | FOOBAR{fbidx}_ | {idx} | {n} | {m} | 0 | {ni} | {no}",
                    idx = fbnid.to_idx(),
                    n = fbn.name,
                    m = fbn.module,
                    ni = fbn.inodes.len(),
                    no = fbn.onodes.len()
                )?;
                for n in &fbn.inodes {
                    self.write_inode(f, n)?;
                }
                for &n in &fbn.onodes {
                    self.write_onode(f, n)?;
                }
                self.write_pterm(f, &fbn.term)?;
                writeln!(f)?;
            }
        }
        for (fbid, fb) in &self.fbs {
            let idx = fbid.to_idx() + 1;
            if !fb.global_fclk.is_empty() {
                write!(f, "GLOBAL_FCLK_IDX | FOOBAR{idx}_")?;
                for x in fb.global_fclk.values() {
                    match x {
                        None => write!(f, " | -1")?,
                        Some(n) => write!(f, " | {n}", n = n.to_idx())?,
                    }
                }
                writeln!(f)?;
                writeln!(f)?;
            }
        }

        if let Some(ref std) = self.iostd_default {
            writeln!(f, "IOSTD | {std}")?;
            for (k, v) in &self.iostd {
                writeln!(f, "{k} | {v}")?;
            }
            writeln!(f)?;
            writeln!(f)?;
        }

        if !self.vref.is_empty() {
            write!(f, "VREF")?;
            for p in &self.vref {
                write!(f, " | {p}")?;
            }
            writeln!(f)?;
            writeln!(f)?;
        }

        if self.utc.values().any(Option::is_some) {
            write!(f, "UTC")?;
            for (k, v) in &self.utc {
                if let Some(n) = v {
                    write!(
                        f,
                        " | {n} | {s}",
                        s = match k {
                            Ut::Clk => 0,
                            Ut::Oe => 1,
                            Ut::Set => 2,
                            Ut::Rst => 3,
                        }
                    )?;
                }
            }
            writeln!(f)?;
            writeln!(f)?;
        }

        if let Some(ref cdr) = self.cdr {
            match cdr.reset {
                CdrReset::Unused(ref p) => {
                    writeln!(f, "CDR | {p} | <prohibited> | {div}", div = cdr.div)?;
                }
                CdrReset::Used(ib) => {
                    writeln!(
                        f,
                        "CDR | <assigned> | {n} | {div}",
                        div = cdr.div,
                        n = self.ibufs.key(ib),
                    )?;
                }
            }
            writeln!(f)?;
        }

        if !self.prohibit_pins.is_empty() {
            for p in &self.prohibit_pins {
                writeln!(f, "PROHIBIT_PIN | {p}")?;
            }
            writeln!(f)?;
        }

        for (fbid, fb) in &self.fbs {
            let idx = fbid.to_idx() + 1;
            if !fb.inputs.is_empty() {
                for chunk in fb.inputs.chunks(5) {
                    write!(f, "FB_ORDER_OF_INPUTS | FOOBAR{idx}_")?;
                    for inp in chunk {
                        write!(
                            f,
                            " | {idx} | {n} | {p}",
                            idx = inp.index.to_idx(),
                            n = inp.name,
                            p = match inp.pad {
                                None => "NULL",
                                Some(ref n) => n,
                            }
                        )?;
                    }
                    writeln!(f)?;
                }
                writeln!(f)?;
            }

            if !fb.imux.is_empty() {
                write!(f, "FB_IMUX_INDEX | FOOBAR{idx}_")?;
                for &x in fb.imux.values() {
                    match x {
                        FbImux::None => write!(f, " | -1")?,
                        FbImux::Plain(n) => write!(f, " | {n}")?,
                        FbImux::WireAnd => write!(f, " | 999")?,
                    }
                }
                writeln!(f)?;
                writeln!(f)?;
            }
        }

        if self.global_fclk.iter().next().is_some() {
            write!(f, "GLOBAL_FCLK")?;
            for (fclk, s) in &self.global_fclk {
                write!(
                    f,
                    " | {n} | {f} | {p}",
                    n = s.name,
                    f = fclk.to_idx(),
                    p = s.path
                )?;
            }
            writeln!(f)?;
            writeln!(f)?;
        }

        if self.global_foe.iter().next().is_some() {
            write!(f, "GLOBAL_FOE")?;
            for (foe, s) in &self.global_foe {
                write!(
                    f,
                    " | {n} | {f} | {p}",
                    n = s.name,
                    f = foe.to_idx(),
                    p = s.path
                )?;
            }
            writeln!(f)?;
            writeln!(f)?;
        }

        if let Some(ref n) = self.global_fsr {
            writeln!(f, "GLOBAL_FSR | {n} | 0 | 0")?;
            writeln!(f)?;
        }

        if let Some(ref n) = self.dge {
            writeln!(f, "DGE | {n} | 0 | 0")?;
            writeln!(f)?;
        }

        Ok(())
    }
}
