use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_re_hammer::{Backend, FuzzerId, Session};
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::bits::Bits;
use prjcombine_re_xilinx_cpld::device::{Device, DeviceKind, Package, PkgPin};
use prjcombine_re_xilinx_cpld::types::{
    BankId, ClkMuxVal, ClkPadId, ExportDir, FbnId, FclkId, FoeId, ImuxId, ImuxInput, OeMuxVal,
    OePadId, SrMuxVal, Ut, Xc9500McPt,
};
use prjcombine_re_xilinx_cpld::vm6::{
    BufOe, Cdr, CdrReset, FbImux, FbInput, Fbnand, GlobalSig, IBuf, InputNode, InputNodeKind,
    Macrocell, Node, NodeIoKind, NodeKind, OBuf, PTerm, Signal, Srff, Uim,
};
use prjcombine_re_xilinx_cpld::vm6_util::{insert_dummy_obuf, insert_node, prep_vm6};
use prjcombine_re_xilinx_cpld::{
    db::{DeviceInfo, ImuxData, Part},
    hprep6::run_hprep6,
};
use prjcombine_types::bitvec::BitVec;
use prjcombine_types::cpld::{BlockId, IoCoord, IpadId, MacrocellCoord, ProductTermId};

use crate::{collect::collect_fuzzers, fuzzers::add_fuzzers};

#[derive(Debug)]
pub struct CpldBackend<'a> {
    pub debug: u8,
    pub tc: &'a Toolchain,
    pub device: &'a Device,
    pub imux: &'a ImuxData,
    pub package: &'a Package,
    pub part: &'a Part,
    pub pin_map: HashMap<IoCoord, &'a str>,
    pub imux_pinning: EntityVec<ImuxId, ImuxInput>,
    pub ibuf_test_imux: HashMap<IoCoord, ImuxId>,
    pub bank_test_iob: EntityVec<BankId, MacrocellCoord>,
    pub oe_pads_remapped: EntityVec<OePadId, IoCoord>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Key {
    McPresent(MacrocellCoord),
    McHasOut(MacrocellCoord, NodeKind),
    McOutUseMutex(MacrocellCoord, NodeKind),
    McFlag(MacrocellCoord, u8),
    McSiPresent(MacrocellCoord),
    McSiHasOut(MacrocellCoord, NodeKind),
    McSiHasTerm(MacrocellCoord, NodeKind),
    McSiTermImux(MacrocellCoord, NodeKind, ImuxId),
    McSiImport(MacrocellCoord, NodeKind, ExportDir),
    McSiPla(MacrocellCoord, NodeKind, ProductTermId),
    McSiMutex(MacrocellCoord),
    McOe(MacrocellCoord),
    McFfPresent(MacrocellCoord),
    McFfInput(MacrocellCoord, InputNodeKind),
    FbImportMutex(BlockId),
    IBufPresent(IoCoord),
    IBufFlag(IoCoord, u8),
    IBufHasOut(IoCoord, NodeKind),
    IBufOutUseMutex(IoCoord, NodeKind),
    OBufPresent(MacrocellCoord),
    OBufFlag(MacrocellCoord, u8),
    FbImux(BlockId, ImuxId),
    UimPath(BlockId, ImuxId, MacrocellCoord),
    FbnPresent(BlockId, FbnId),
    Usercode(u8),
    UsercodePresent,
    NetworkFlag(u8),
    PlaHasTerm(BlockId, ProductTermId),
    PlaTermImux(BlockId, ProductTermId, ImuxId),
    PlaTermFbn(BlockId, ProductTermId, FbnId),
    PlaTermMutex(BlockId),
    CtPresent(BlockId, ProductTermId),
    CtInvert(BlockId, ProductTermId),
    CtUseMutex(BlockId, ProductTermId),
    Fclk(FclkId),
    Fsr,
    Foe(FoeId),
    Dge,
    FbClk(BlockId, FclkId),
    Ut(Ut),
    IsVref(IoCoord),
    Iostd(IoCoord),
    BankVoltage(BankId),
    BankMutex(BankId),
    VrefMutex,
    Cdr,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value {
    None,
    Bool(bool),
    ImuxInput(ImuxInput),
    InputSi(NodeKind),
    InputCt(ProductTermId),
    InputUt(BlockId, ProductTermId),
    InputPad(IoCoord, NodeKind),
    InputMc(MacrocellCoord, NodeKind),
    CopyQ,
    MutexFuzz,
    MutexPin,
    ClkPadNode(NodeKind, ClkPadId, u8),
    OePadNode(NodeKind, OePadId, u8),
    SrPadNode(NodeKind),
    ClkPad(ClkPadId),
    McGlb,
    Ut(BlockId, ProductTermId),
    Ireg,
    CtUseCt,
    CtUseUt(Ut),
    Iostd(Iostd),
    Voltage(Voltage),
    Cdr(u8, bool),
    CopyOe,
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Iostd> for Value {
    fn from(value: Iostd) -> Self {
        Self::Iostd(value)
    }
}

impl From<Voltage> for Value {
    fn from(value: Voltage) -> Self {
        Self::Voltage(value)
    }
}

impl From<ImuxInput> for Value {
    fn from(value: ImuxInput) -> Self {
        Self::ImuxInput(value)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Iostd {
    Lvttl,
    Lvcmos15,
    Lvcmos18,
    Lvcmos18Any,
    Lvcmos25,
    Lvcmos33,
    Sstl2I,
    Sstl3I,
    HstlI,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Voltage {
    V15,
    V18,
    V25,
    V33,
}

impl Iostd {
    pub fn voltage(self) -> Voltage {
        match self {
            Iostd::Lvttl => Voltage::V33,
            Iostd::Lvcmos15 => Voltage::V15,
            Iostd::Lvcmos18 => Voltage::V18,
            Iostd::Lvcmos18Any => Voltage::V18,
            Iostd::Lvcmos25 => Voltage::V25,
            Iostd::Lvcmos33 => Voltage::V33,
            Iostd::Sstl2I => Voltage::V25,
            Iostd::Sstl3I => Voltage::V33,
            Iostd::HstlI => Voltage::V15,
        }
    }
    pub fn is_vref(self) -> bool {
        matches!(self, Iostd::Sstl2I | Iostd::Sstl3I | Iostd::HstlI)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum MultiValue {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum FuzzerInfo {
    Imux(BlockId, ImuxId, ImuxInput),
    ImuxUimMc(BlockId, ImuxId, MacrocellCoord),
    PlaPTermImux(BlockId, ProductTermId, ImuxId, bool),
    PlaPTermFbn(BlockId, ProductTermId, FbnId),
    McPTermImux(MacrocellCoord, Xc9500McPt, ImuxId, bool),
    McOrTerm(MacrocellCoord, NodeKind, Xc9500McPt),
    McOrExp(MacrocellCoord, NodeKind, ExportDir),
    McOrPla(MacrocellCoord, ProductTermId),
    McSiSpec(MacrocellCoord, Xc9500McPt),
    CtInvert(BlockId, ProductTermId),
    McLowPower(MacrocellCoord),
    McInputD2(MacrocellCoord),
    McInputD2B(MacrocellCoord),
    McInputXor(MacrocellCoord),
    McInputXorB(MacrocellCoord),
    McInputD1(MacrocellCoord),
    McInputD1B(MacrocellCoord),
    McInputIreg(MacrocellCoord),
    McComb(MacrocellCoord),
    McUimOut(MacrocellCoord),
    McUimOutInv(MacrocellCoord),
    McClk(MacrocellCoord, ClkMuxVal, bool),
    McRst(MacrocellCoord, SrMuxVal),
    McSet(MacrocellCoord, SrMuxVal),
    McTff(MacrocellCoord),
    McLatch(MacrocellCoord),
    McDdr(MacrocellCoord),
    McInit(MacrocellCoord),
    McCeRst(MacrocellCoord),
    McCeSet(MacrocellCoord),
    McCePt(MacrocellCoord),
    McCeCt(MacrocellCoord),
    McOe(MacrocellCoord, OeMuxVal, bool),
    IBufPresent(IoCoord),
    IBufPresentGnd(IoCoord),
    IBufPresentPullup(IoCoord),
    IBufPresentKeeper(IoCoord),
    IBufSchmitt(IoCoord),
    IBufUseVref(IoCoord),
    IBufIsVref(IoCoord),
    IBufDge(IoCoord),
    IBufIostd(BankId, Iostd),
    IpadUimOutFb(IpadId, BlockId),
    OBufPresentReg(MacrocellCoord),
    OBufPresentComb(MacrocellCoord),
    OBufSlew(MacrocellCoord),
    OBufOpenDrain(MacrocellCoord),
    OBufOe(MacrocellCoord, OeMuxVal, bool),
    OBufIostd(BankId, Iostd),
    Usercode(u8),
    NoIsp,
    Ut(Ut, BlockId, ProductTermId),
    GlobalKeeper,
    Dge,
    ClkDiv(u8),
    ClkDivDelay,
    FbClk(BlockId, Option<ClkPadId>, Option<ClkPadId>),
    Fclk(FclkId, ClkPadId, bool),
    Fsr(bool),
    Foe(FoeId, OePadId, bool),
    FoeMc(FoeId),
    FbPresent(BlockId),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PostProc {}

#[derive(Debug)]
pub struct State {
    pub fuzzers: HashMap<FuzzerInfo, Vec<HashMap<usize, bool>>>,
}

fn iostd(std: Iostd) -> String {
    match std {
        Iostd::Lvttl => "LVTTL",
        Iostd::Lvcmos15 => "LVCMOS15",
        Iostd::Lvcmos18 => "LVCMOS18",
        Iostd::Lvcmos18Any => "LVCMOS18_ANY",
        Iostd::Lvcmos25 => "LVCMOS25",
        Iostd::Lvcmos33 => "LVCMOS33",
        Iostd::Sstl2I => "SSTL2_I",
        Iostd::Sstl3I => "SSTL3_I",
        Iostd::HstlI => "HSTL_I",
    }
    .to_string()
}

impl Backend for CpldBackend<'_> {
    type Key = Key;
    type Value = Value;
    type MultiValue = MultiValue;
    type Bitstream = BitVec;
    type FuzzerInfo = FuzzerInfo;
    type PostProc = PostProc;
    type BitPos = usize;
    type State = State;

    fn make_state(&self) -> State {
        State {
            fuzzers: HashMap::new(),
        }
    }

    fn assemble_multi(v: &MultiValue, _b: &BitVec) -> Value {
        match *v {}
    }

    fn bitgen(&self, kv: &HashMap<Key, Value>) -> BitVec {
        let mut vm6 = prep_vm6(self.part, self.device, self.package, &self.part.speeds[0]);
        let mut usercode: u32 = 0;

        let mut pup_in = None;
        let mut pup_out = None;
        if self.device.kind == DeviceKind::Xpla3 {
            let node_in = insert_node(
                &mut vm6,
                Node {
                    is_signal: false,
                    name: "PUP_IN".to_string(),
                    io_kind: NodeIoKind::Input,
                    flags: 0,
                    module: "top".to_string(),
                    copy_of: None,
                    driver: None,
                    kind: NodeKind::None,
                    terms: vec![],
                },
            );
            let node_out = insert_node(
                &mut vm6,
                Node {
                    is_signal: false,
                    name: "PUP_OUT".to_string(),
                    io_kind: NodeIoKind::Inout,
                    flags: 0,
                    module: "top".to_string(),
                    copy_of: None,
                    driver: Some("PUP".to_string()),
                    kind: NodeKind::IiImux,
                    terms: vec![],
                },
            );
            vm6.ibufs.insert(
                "PUP".to_string(),
                IBuf {
                    module: "top".to_string(),
                    flags: 0,
                    inodes: vec![InputNode {
                        node: node_in,
                        kind: InputNodeKind::IiIn,
                    }],
                    onodes: vec![node_out],
                },
            );
            pup_in = Some(node_in);
            pup_out = Some(node_out);
        }

        let mut uim_lut = HashMap::new();
        let mut uim_node_lut = HashMap::new();
        let mut pad_lut = HashMap::new();
        for (k, v) in kv {
            match *k {
                Key::OBufPresent(mcid) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let name = format!("OBUF_{}_{}", mcid.block.to_idx(), mcid.macrocell.to_idx());

                    let node_pad = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!(
                                "PAD_{}_{}",
                                mcid.block.to_idx(),
                                mcid.macrocell.to_idx()
                            ),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(name.clone()),
                            kind: NodeKind::OiOut,
                            terms: vec![],
                        },
                    );
                    pad_lut.insert(IoCoord::Macrocell(mcid), node_pad);
                    let ob = vm6
                        .obufs
                        .insert(
                            name,
                            OBuf {
                                module: "top".to_string(),
                                flags: 0,
                                inodes: vec![],
                                onodes: vec![node_pad],
                            },
                        )
                        .0;
                    vm6.fbs[mcid.block].pins[mcid.macrocell].obuf = Some(ob);
                    vm6.fbs[mcid.block].pins[mcid.macrocell].obuf_used = true;
                }
                Key::McPresent(mcid) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let mc = vm6
                        .macrocells
                        .insert(
                            format!("MC_{}_{}", mcid.block.to_idx(), mcid.macrocell.to_idx()),
                            Macrocell {
                                module: "top".to_string(),
                                flags: 0,
                                inodes: vec![],
                                onodes: vec![],
                                signal: None,
                                srff: None,
                                bufoe: None,
                            },
                        )
                        .0;
                    vm6.fbs[mcid.block].pins[mcid.macrocell].mc = Some(mc);
                    vm6.fbs[mcid.block].pins[mcid.macrocell].mc_used = true;
                }
                Key::FbImux(fbid, imid) => {
                    if *v != Value::ImuxInput(ImuxInput::Uim) {
                        continue;
                    }
                    let dummy_mc_node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!("UIM_DUMMY_{}_{}", fbid.to_idx(), imid.to_idx()),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(format!("UIM_MC_{}_{}", fbid.to_idx(), imid.to_idx())),
                            kind: NodeKind::McUim,
                            terms: vec![],
                        },
                    );
                    vm6.macrocells.insert(
                        format!("UIM_MC_{}_{}", fbid.to_idx(), imid.to_idx()),
                        Macrocell {
                            module: "top".to_string(),
                            flags: 0,
                            inodes: vec![],
                            onodes: vec![dummy_mc_node],
                            signal: None,
                            srff: None,
                            bufoe: None,
                        },
                    );
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!("UIM_OUT_{}_{}", fbid.to_idx(), imid.to_idx()),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(format!("UIM_{}_{}", fbid.to_idx(), imid.to_idx())),
                            kind: NodeKind::UimOut,
                            terms: vec![],
                        },
                    );
                    uim_node_lut.insert((fbid, imid), node);
                    let uim = vm6
                        .uims
                        .insert(
                            format!("UIM_{}_{}", fbid.to_idx(), imid.to_idx()),
                            Uim {
                                module: "top".to_string(),
                                inodes: vec![InputNode {
                                    node: dummy_mc_node,
                                    kind: InputNodeKind::None,
                                }],
                                onodes: vec![node],
                                term: PTerm {
                                    inputs: vec![(
                                        true,
                                        format!("UIM_DUMMY_{}_{}", fbid.to_idx(), imid.to_idx()),
                                    )],
                                },
                            },
                        )
                        .0;
                    uim_lut.insert((fbid, imid), uim);
                }
                Key::FbnPresent(fb, fbn) => {
                    let name = format!("FBNAND_{}_{}", fb.to_idx(), fbn.to_idx());
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!("FBNAND_OUT_{}_{}", fb.to_idx(), fbn.to_idx()),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(name.clone()),
                            kind: NodeKind::FbnOut,
                            terms: vec![],
                        },
                    );
                    vm6.fbs[fb].fbnands.insert(
                        fbn,
                        Fbnand {
                            name,
                            module: "top".to_string(),
                            inodes: vec![],
                            onodes: vec![node],
                            term: PTerm { inputs: vec![] },
                        },
                    );
                }
                Key::Usercode(i) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    usercode |= 1 << i;
                }
                Key::NetworkFlag(bit) => {
                    let &Value::Bool(v) = v else { unreachable!() };
                    if v {
                        vm6.network_flags |= 1 << bit;
                    } else {
                        vm6.network_flags &= !(1 << bit);
                    }
                }
                Key::PlaHasTerm(fb, pt) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    vm6.fbs[fb]
                        .pla
                        .as_mut()
                        .unwrap()
                        .terms
                        .insert(pt, PTerm { inputs: vec![] });
                }
                Key::IsVref(ioid) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    vm6.vref.insert(self.pin_map[&ioid].to_string());
                }
                // XXX OBUFs
                _ => (),
            }
        }
        for (k, v) in kv {
            match *k {
                Key::McSiPresent(mcid) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    let name = format!("{}.SI", vm6.macrocells.key(rmcid));
                    vm6.macrocells[rmcid].signal = Some(Signal {
                        name,
                        inodes: vec![],
                        onodes: vec![],
                    });
                }
                Key::IBufPresent(ioid) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let name = match ioid {
                        IoCoord::Ipad(ip) => format!("IBUF_IPAD_{ip}", ip = ip.to_idx()),
                        IoCoord::Macrocell(mc) => {
                            format!(
                                "IBUF_{f}_{m}",
                                f = mc.block.to_idx(),
                                m = mc.macrocell.to_idx()
                            )
                        }
                    };
                    let node_pad = pad_lut.get(&ioid).copied().unwrap_or_else(|| {
                        let n = insert_node(
                            &mut vm6,
                            Node {
                                is_signal: false,
                                name: format!("{name}_IN"),
                                io_kind: NodeIoKind::Input,
                                flags: 0,
                                module: "top".to_string(),
                                copy_of: None,
                                driver: None,
                                kind: NodeKind::None,
                                terms: vec![],
                            },
                        );
                        pad_lut.insert(ioid, n);
                        n
                    });
                    let ib = vm6
                        .ibufs
                        .insert(
                            name,
                            IBuf {
                                module: "top".to_string(),
                                flags: 0,
                                inodes: vec![InputNode {
                                    node: node_pad,
                                    kind: InputNodeKind::IiIn,
                                }],
                                onodes: vec![],
                            },
                        )
                        .0;
                    match ioid {
                        IoCoord::Ipad(ip) => {
                            let pin = &mut vm6.ipad_fb.as_mut().unwrap().pins[ip];
                            pin.ibuf = Some(ib);
                            pin.ibuf_used = true;
                        }
                        IoCoord::Macrocell(mc) => {
                            let pin = &mut vm6.fbs[mc.block].pins[mc.macrocell];
                            pin.ibuf = Some(ib);
                            pin.ibuf_used = true;
                        }
                    }
                }
                _ => (),
            }
        }
        let mut ibuf_out_lut = HashMap::new();
        let mut mc_out_lut = HashMap::new();
        for (k, v) in kv {
            match *k {
                Key::IBufHasOut(ioid, kind) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let k = match kind {
                        NodeKind::IiImux => "IMUX",
                        NodeKind::IiReg => "REG",
                        NodeKind::IiFclk => "FCLK",
                        NodeKind::IiFclkInv => "FCLKINV",
                        NodeKind::IiFoe => "FOE",
                        NodeKind::IiFoeInv => "FOEINV",
                        NodeKind::IiFsr => "FSR",
                        NodeKind::IiFsrInv => "FSRINV",
                        _ => unreachable!(),
                    };
                    let ibid = vm6.get_ibuf_id(ioid).unwrap();
                    let ibname = vm6.ibufs.key(ibid).clone();
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!("{ibname}_{k}"),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(ibname),
                            kind,
                            terms: vec![],
                        },
                    );
                    vm6.ibufs[ibid].onodes.push(node);
                    ibuf_out_lut.insert((ioid, kind), node);
                }
                Key::McHasOut(mcid, kind) => {
                    let suf = match *v {
                        Value::Bool(false) => continue,
                        Value::Bool(true) => None,
                        Value::CopyQ => Some("Q"),
                        Value::CopyOe => Some("BUFOE.OUT"),
                        _ => unreachable!(),
                    };
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    let mcname = vm6.macrocells.key(rmcid).clone();
                    let k = match kind {
                        NodeKind::McQ => "Q",
                        NodeKind::McFbk => "FBK",
                        NodeKind::McComb => "COMV",
                        NodeKind::McExport => "EXPORT",
                        NodeKind::McOe => "OE",
                        NodeKind::McUim => "UIM",
                        NodeKind::McGlb => "GLB",
                        _ => unreachable!(),
                    };
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!(
                                "MC_{k}_{}_{}",
                                mcid.block.to_idx(),
                                mcid.macrocell.to_idx()
                            ),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: suf.map(|s| {
                                format!(
                                    "MC_{f}_{m}.{s}",
                                    f = mcid.block.to_idx(),
                                    m = mcid.macrocell.to_idx()
                                )
                            }),
                            driver: Some(mcname),
                            kind,
                            terms: vec![],
                        },
                    );
                    vm6.macrocells[rmcid].onodes.push(node);
                    mc_out_lut.insert((mcid, kind), node);
                    if let Some(obid) = vm6.fbs[mcid.block].pins[mcid.macrocell].obuf {
                        let ik = match kind {
                            NodeKind::McQ => InputNodeKind::OiIn,
                            NodeKind::McOe => InputNodeKind::OiOe,
                            _ => continue,
                        };
                        vm6.obufs[obid].inodes.push(InputNode { kind: ik, node });
                    }
                }
                Key::McSiHasOut(mcid, kind) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    let siname = vm6.macrocells[rmcid].signal.as_ref().unwrap().name.clone();
                    let k = match kind {
                        NodeKind::McSiD1 => "SI_D1",
                        NodeKind::McSiD2 => "SI_D2",
                        NodeKind::McSiClkf => "SI_CLKF",
                        NodeKind::McSiRstf => "SI_RSTF",
                        NodeKind::McSiSetf => "SI_SETF",
                        NodeKind::McSiTrst => "SI_TRST",
                        NodeKind::McSiCe => "SI_CE",
                        NodeKind::McSiExport => "SI_EXPORT",
                        _ => unreachable!(),
                    };
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: true,
                            name: format!(
                                "MC_{k}_{}_{}",
                                mcid.block.to_idx(),
                                mcid.macrocell.to_idx()
                            ),
                            io_kind: NodeIoKind::Inout,
                            flags: 0x1000,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(siname),
                            kind,
                            terms: vec![],
                        },
                    );
                    vm6.macrocells[rmcid]
                        .signal
                        .as_mut()
                        .unwrap()
                        .onodes
                        .push(node);
                    mc_out_lut.insert((mcid, kind), node);
                }
                Key::IBufFlag(ioid, bit) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let ribid = vm6.get_ibuf_id(ioid).unwrap();
                    vm6.ibufs[ribid].flags |= 1 << bit;
                }
                Key::OBufFlag(mcid, bit) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let robid = vm6.fbs[mcid.block].pins[mcid.macrocell].obuf.unwrap();
                    vm6.obufs[robid].flags |= 1 << bit;
                }

                Key::McFlag(mcid, bit) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    vm6.macrocells[rmcid].flags |= 1 << bit;
                }
                Key::Iostd(mc) => {
                    let v = match *v {
                        Value::Iostd(v) => v,
                        Value::None => continue,
                        _ => unreachable!(),
                    };
                    let node = pad_lut[&mc];
                    // XXX extend to obuf
                    let k = vm6.nodes[node].name.clone();
                    vm6.iostd.insert(k, iostd(v));
                }

                _ => (),
            }
        }
        for (k, v) in kv {
            match *k {
                Key::UimPath(fbid, imid, mcid) => {
                    let val = match *v {
                        Value::None => continue,
                        Value::Bool(v) => v,
                        _ => unreachable!(),
                    };
                    let uim = uim_lut[&(fbid, imid)];
                    let node = mc_out_lut[&(mcid, NodeKind::McUim)];
                    let name = vm6.nodes[node].name.clone();
                    vm6.uims[uim].term.inputs.push((val, name));
                    vm6.uims[uim].inodes.push(InputNode {
                        kind: InputNodeKind::None,
                        node,
                    });
                }
                Key::FbImux(fbid, imid) => {
                    let inp = match *v {
                        Value::ImuxInput(inp) => inp,
                        Value::None => continue,
                        _ => unreachable!(),
                    };
                    let (node, val, pad) = match inp {
                        ImuxInput::Uim => (uim_node_lut[&(fbid, imid)], FbImux::WireAnd, None),
                        ImuxInput::Mc(mcid) => (
                            mc_out_lut[&(mcid, NodeKind::McUim)],
                            FbImux::Plain(self.imux[imid][&inp]),
                            None,
                        ),
                        ImuxInput::Fbk(mcid) => (
                            mc_out_lut[&(MacrocellCoord::simple(fbid, mcid), NodeKind::McFbk)],
                            FbImux::Plain(self.imux[imid][&inp]),
                            None,
                        ),
                        ImuxInput::Ibuf(ioid) => (
                            pad_lut[&ioid],
                            FbImux::Plain(self.imux[imid][&inp]),
                            Some(match ioid {
                                IoCoord::Ipad(ip) => vm6.ipad_fb.as_ref().unwrap().pins[ip]
                                    .pad
                                    .as_ref()
                                    .unwrap()
                                    .0
                                    .clone(),
                                IoCoord::Macrocell(mc) => vm6.fbs[mc.block].pins[mc.macrocell]
                                    .pad
                                    .as_ref()
                                    .unwrap()
                                    .0
                                    .clone(),
                            }),
                        ),
                        ImuxInput::Pup => {
                            (pup_in.unwrap(), FbImux::Plain(self.imux[imid][&inp]), None)
                        }
                    };
                    let name = vm6.nodes[node].name.clone();
                    let fb = &mut vm6.fbs[fbid];
                    fb.inputs.push(FbInput {
                        index: imid,
                        name,
                        pad,
                    });
                    fb.imux[imid] = val;
                }
                Key::McSiHasTerm(mcid, kind) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let node = mc_out_lut[&(mcid, kind)];
                    vm6.nodes[node].terms.push(PTerm { inputs: vec![] });
                }
                Key::McFfPresent(mcid) => {
                    let is_ireg = match *v {
                        Value::Bool(false) => continue,
                        Value::Bool(true) => false,
                        Value::Ireg => true,
                        _ => unreachable!(),
                    };
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    let mcname = vm6.macrocells.key(rmcid);
                    let name = format!("{mcname}.REG");
                    let nname = format!("{mcname}.Q");
                    let aname = format!("{mcname}.XOR");
                    let iname = format!("{mcname}.D");
                    let inode = if is_ireg {
                        ibuf_out_lut[&(IoCoord::Macrocell(mcid), NodeKind::IiReg)]
                    } else {
                        insert_node(
                            &mut vm6,
                            Node {
                                is_signal: false,
                                name: iname,
                                io_kind: NodeIoKind::Inout,
                                flags: 0,
                                module: "top".to_string(),
                                copy_of: None,
                                driver: Some(aname),
                                kind: NodeKind::AluF,
                                terms: vec![],
                            },
                        )
                    };
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: nname,
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(name.clone()),
                            kind: NodeKind::SrffQ,
                            terms: vec![],
                        },
                    );
                    vm6.macrocells[rmcid].srff = Some(Srff {
                        name,
                        inodes: vec![InputNode {
                            node: inode,
                            kind: InputNodeKind::SrffD,
                        }],
                        onodes: vec![node],
                    });
                }

                _ => (),
            }
        }
        for (k, v) in kv {
            match *k {
                Key::PlaTermImux(fb, pt, imid) => {
                    if *v == Value::None {
                        continue;
                    }
                    let &Value::Bool(v) = v else { unreachable!() };
                    let Value::ImuxInput(inp) = kv[&Key::FbImux(fb, imid)] else {
                        panic!("imux not set properly");
                    };
                    let node = match inp {
                        ImuxInput::Mc(mc) => mc_out_lut[&(mc, NodeKind::McUim)],
                        ImuxInput::Fbk(mc) => {
                            mc_out_lut[&(MacrocellCoord::simple(fb, mc), NodeKind::McFbk)]
                        }
                        ImuxInput::Ibuf(io) => ibuf_out_lut[&(io, NodeKind::IiImux)],
                        ImuxInput::Uim => uim_node_lut[&(fb, imid)],
                        ImuxInput::Pup => pup_out.unwrap(),
                    };
                    let name = vm6.nodes[node].name.to_owned();
                    let pla = vm6.fbs[fb].pla.as_mut().unwrap();
                    pla.terms[pt].inputs.push((v, name));
                }
                Key::PlaTermFbn(fb, pt, fbn) => {
                    if *v == Value::None {
                        continue;
                    }
                    assert_eq!(*v, Value::Bool(true));
                    let node = vm6.fbs[fb].fbnands[fbn].onodes[0];
                    let name = vm6.nodes[node].name.to_owned();
                    let pla = vm6.fbs[fb].pla.as_mut().unwrap();
                    pla.terms[pt].inputs.push((true, name));
                }
                Key::McSiTermImux(mc, kind, imid) => {
                    if *v == Value::None {
                        continue;
                    }
                    let &Value::Bool(v) = v else { unreachable!() };
                    let Value::ImuxInput(inp) = kv[&Key::FbImux(mc.block, imid)] else {
                        panic!("imux not set properly");
                    };
                    let node = match inp {
                        ImuxInput::Mc(mc) => mc_out_lut[&(mc, NodeKind::McUim)],
                        ImuxInput::Fbk(omc) => {
                            mc_out_lut[&(MacrocellCoord::simple(mc.block, omc), NodeKind::McFbk)]
                        }
                        ImuxInput::Ibuf(io) => ibuf_out_lut[&(io, NodeKind::IiImux)],
                        ImuxInput::Uim => uim_node_lut[&(mc.block, imid)],
                        ImuxInput::Pup => pup_out.unwrap(),
                    };
                    let name = vm6.nodes[node].name.to_owned();
                    let ptnode = mc_out_lut[&(mc, kind)];
                    vm6.nodes[ptnode].terms[0].inputs.push((v, name));
                }
                Key::McSiImport(mcid, kind, dir) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let omcid = self.device.export_source(mcid, dir);
                    let inode = mc_out_lut[&(omcid, NodeKind::McExport)];
                    let iname = vm6.nodes[inode].name.clone();
                    let node = mc_out_lut[&(mcid, kind)];
                    vm6.nodes[node].terms.push(PTerm {
                        inputs: vec![(true, iname)],
                    });
                }
                _ => (),
            }
        }
        let mut ct_lut = HashMap::new();
        for (k, v) in kv {
            match *k {
                Key::McSiPla(mcid, kind, pt) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let pterm = vm6.fbs[mcid.block].pla.as_ref().unwrap().terms[pt].clone();
                    let node = mc_out_lut[&(mcid, kind)];
                    vm6.nodes[node].terms.push(pterm);
                }
                Key::CtPresent(fbid, pt) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let kind = [
                        NodeKind::CtSi0,
                        NodeKind::CtSi1,
                        NodeKind::CtSi2,
                        NodeKind::CtSi3,
                        NodeKind::CtSi4,
                        NodeKind::CtSi5,
                        NodeKind::CtSi6,
                        NodeKind::CtSi7,
                    ][pt.to_idx()];
                    let pterm = vm6.fbs[fbid].pla.as_ref().unwrap().terms[pt].clone();
                    let driver = vm6.fbs[fbid].ct.as_ref().unwrap().name.clone();
                    let node = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: true,
                            name: format!("{driver}/{pt}", pt = pt.to_idx()),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(driver),
                            kind,
                            terms: vec![pterm],
                        },
                    );
                    vm6.fbs[fbid].ct.as_mut().unwrap().onodes.push(node);
                    ct_lut.insert((fbid, pt), node);
                }
                Key::CtInvert(fbid, pt) => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let kind = [
                        NodeKind::CtSi0,
                        NodeKind::CtSi1,
                        NodeKind::CtSi2,
                        NodeKind::CtSi3,
                        NodeKind::CtSi4,
                        NodeKind::CtSi5,
                        NodeKind::CtSi6,
                        NodeKind::CtSi7,
                    ][pt.to_idx()];

                    vm6.fbs[fbid].ct.as_mut().unwrap().invs.insert(kind);
                }
                _ => (),
            }
        }
        for (k, v) in kv {
            match *k {
                Key::McFfInput(mcid, ikind) => {
                    let (node, is_inp) = match *v {
                        Value::None => continue,
                        Value::InputCt(pt) => (ct_lut[&(mcid.block, pt)], true),
                        Value::InputUt(fb, pt) => (ct_lut[&(fb, pt)], true),
                        Value::InputSi(kind) => (mc_out_lut[&(mcid, kind)], false),
                        Value::InputPad(imc, kind) => (ibuf_out_lut[&(imc, kind)], true),
                        _ => unreachable!(),
                    };
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    vm6.macrocells[rmcid]
                        .srff
                        .as_mut()
                        .unwrap()
                        .inodes
                        .push(InputNode { node, kind: ikind });
                    if is_inp {
                        vm6.macrocells[rmcid].inodes.push(InputNode {
                            node,
                            kind: InputNodeKind::None,
                        });
                    }
                }
                Key::McOe(mcid) => {
                    let (node, is_inp) = match *v {
                        Value::None => continue,
                        Value::InputCt(pt) => (ct_lut[&(mcid.block, pt)], true),
                        Value::InputUt(fb, pt) => (ct_lut[&(fb, pt)], true),
                        Value::InputSi(kind) => (mc_out_lut[&(mcid, kind)], false),
                        Value::InputPad(imc, kind) => (ibuf_out_lut[&(imc, kind)], true),
                        Value::InputMc(mc, kind) => (mc_out_lut[&(mc, kind)], true),
                        _ => unreachable!(),
                    };
                    let rmcid = vm6.fbs[mcid.block].pins[mcid.macrocell].mc.unwrap();
                    let mcn = vm6.macrocells.key(rmcid).clone();
                    let name = format!("{mcn}.BUFOE");
                    let onode = insert_node(
                        &mut vm6,
                        Node {
                            is_signal: false,
                            name: format!("{name}.OUT"),
                            io_kind: NodeIoKind::Inout,
                            flags: 0,
                            module: "top".to_string(),
                            copy_of: None,
                            driver: Some(name.clone()),
                            kind: NodeKind::BufOut,
                            terms: vec![],
                        },
                    );
                    vm6.macrocells[rmcid].bufoe = Some(BufOe {
                        name,
                        inodes: vec![InputNode {
                            kind: InputNodeKind::CtorUnknown,
                            node,
                        }],
                        onodes: vec![onode],
                    });
                    if is_inp {
                        vm6.macrocells[rmcid].inodes.push(InputNode {
                            node,
                            kind: InputNodeKind::None,
                        });
                    }
                }
                Key::Fclk(idx) => match *v {
                    Value::None => (),
                    Value::Bool(true) => {
                        let mc = self.device.clk_pads[ClkPadId::from_idx(idx.to_idx())];
                        let node = pad_lut[&mc];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_fclk.insert(
                            idx,
                            GlobalSig {
                                name,
                                path: idx.to_idx() as u32,
                            },
                        );
                    }
                    Value::ClkPadNode(kind, src, path) => {
                        let mc = self.device.clk_pads[src];
                        let node = ibuf_out_lut[&(mc, kind)];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_fclk.insert(
                            idx,
                            GlobalSig {
                                name,
                                path: path as u32,
                            },
                        );
                    }
                    _ => unreachable!(),
                },
                Key::Fsr => match *v {
                    Value::None => (),
                    Value::Bool(true) => {
                        let mc = self.device.sr_pad.unwrap();
                        let node = pad_lut[&mc];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_fsr = Some(name);
                    }
                    Value::SrPadNode(kind) => {
                        let mc = self.device.sr_pad.unwrap();
                        let node = ibuf_out_lut[&(mc, kind)];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_fsr = Some(name);
                    }
                    _ => unreachable!(),
                },
                Key::Foe(idx) => match *v {
                    Value::None => (),
                    Value::Bool(true) => {
                        let mc = self.oe_pads_remapped[OePadId::from_idx(idx.to_idx())];
                        let node = pad_lut[&mc];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_foe.insert(
                            idx,
                            GlobalSig {
                                name,
                                path: idx.to_idx() as u32,
                            },
                        );
                    }
                    Value::McGlb => {
                        let IoCoord::Macrocell(mc) =
                            self.device.oe_pads[OePadId::from_idx(idx.to_idx())]
                        else {
                            unreachable!();
                        };
                        let node = mc_out_lut[&(mc, NodeKind::McGlb)];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_foe.insert(
                            idx,
                            GlobalSig {
                                name,
                                path: idx.to_idx() as u32,
                            },
                        );
                    }
                    Value::OePadNode(kind, src, path) => {
                        let mc = self.oe_pads_remapped[src];
                        let node = ibuf_out_lut[&(mc, kind)];
                        let name = vm6.nodes[node].name.to_string();
                        vm6.global_foe.insert(
                            idx,
                            GlobalSig {
                                name,
                                path: path as u32,
                            },
                        );
                    }
                    _ => unreachable!(),
                },
                Key::Dge => {
                    let Value::Bool(v) = v else { unreachable!() };
                    if !v {
                        continue;
                    }
                    let mc = self.device.dge_pad.unwrap();
                    let node = pad_lut[&mc];
                    let name = vm6.nodes[node].name.clone();
                    vm6.dge = Some(name);
                }
                Key::Cdr => match *v {
                    Value::None => (),
                    Value::Cdr(div, rst_en) => {
                        let cdr = self.device.cdr_pad.unwrap();
                        vm6.cdr = Some(Cdr {
                            reset: if rst_en {
                                let ibid = vm6.get_ibuf_id(cdr).unwrap();
                                CdrReset::Used(ibid)
                            } else {
                                let pin = self.pin_map[&cdr];
                                vm6.prohibit_pins.insert(pin.to_string());
                                CdrReset::Unused(pin.to_string())
                            },
                            div: div as u32,
                        });
                    }
                    _ => unreachable!(),
                },
                Key::FbClk(fbid, idx) => {
                    let tgt = match *v {
                        Value::None => continue,
                        Value::ClkPad(x) => x,
                        _ => unreachable!(),
                    };
                    vm6.fbs[fbid].global_fclk[idx] = Some(tgt);
                }
                Key::Ut(ut) => match *v {
                    Value::None => (),
                    Value::Ut(fb, pt) => {
                        let node = ct_lut[&(fb, pt)];
                        let name = vm6.nodes[node].name.clone();
                        vm6.utc[ut] = Some(name);
                    }
                    _ => unreachable!(),
                },
                _ => (),
            }
        }
        if vm6.obufs.is_empty() {
            insert_dummy_obuf(&mut vm6);
        }
        let usercode = if kv.get(&Key::UsercodePresent) == Some(&Value::Bool(true)) {
            Some(usercode)
        } else {
            None
        };
        run_hprep6(self.tc, &vm6, usercode).unwrap().fuses.unwrap()
    }

    fn diff(bs1: &BitVec, bs2: &BitVec) -> HashMap<usize, bool> {
        assert_eq!(bs1.len(), bs2.len());
        let mut res = HashMap::new();
        for (i, b2) in bs2.iter().enumerate() {
            if bs1[i] != b2 {
                res.insert(i, b2);
            }
        }
        res
    }

    fn return_fuzzer(
        &self,
        s: &mut State,
        f: &FuzzerInfo,
        _fi: FuzzerId,
        bits: Vec<HashMap<usize, bool>>,
    ) -> Option<Vec<FuzzerId>> {
        s.fuzzers.insert(*f, bits);
        None
    }

    fn postproc(
        &self,
        _s: &State,
        _bs: &mut BitVec,
        pp: &PostProc,
        _kv: &HashMap<Key, Value>,
    ) -> bool {
        match *pp {}
    }
}

fn imux_inps_pinning(
    device: &Device,
    imux: &ImuxData,
    pin_map: &HashMap<IoCoord, &str>,
) -> EntityVec<ImuxId, ImuxInput> {
    let mut inps_used = HashSet::new();
    let mut res = EntityVec::new();
    if device.kind == DeviceKind::Xc9500 {
        return res;
    }
    for inps in imux.values() {
        'a: {
            for inp in inps
                .keys()
                .copied()
                .filter(|&x| match x {
                    ImuxInput::Ibuf(io) => pin_map.contains_key(&io),
                    ImuxInput::Fbk(_) => true,
                    ImuxInput::Mc(_) => true,
                    ImuxInput::Pup => true,
                    ImuxInput::Uim => unreachable!(),
                })
                .sorted()
            {
                if !inps_used.contains(&inp) {
                    res.push(inp);
                    inps_used.insert(inp);
                    break 'a;
                }
            }
            panic!(
                "OOPS: {len}, {res:#?}, {inps_used:#?}, {inps:#?}",
                len = res.len()
            );
        }
    }
    res
}

fn ibuf_test_imux(device: &Device, imux: &ImuxData) -> HashMap<IoCoord, ImuxId> {
    device
        .io
        .keys()
        .map(|&io| {
            let imid = imux
                .iter()
                .find(|(_, inps)| inps.contains_key(&ImuxInput::Ibuf(io)))
                .unwrap()
                .0;

            (io, imid)
        })
        .collect()
}

pub fn reverse_cpld(
    tc: &Toolchain,
    part: &Part,
    devinfo: &DeviceInfo,
    package: &Package,
    debug: u8,
) -> Bits {
    let pin_map: HashMap<_, _> = package
        .pins
        .iter()
        .filter_map(|(pin, &info)| {
            if let PkgPin::Io(mc) = info {
                Some((mc, &pin[..]))
            } else {
                None
            }
        })
        .collect();

    let imux_pinning = imux_inps_pinning(&devinfo.device, &devinfo.imux, &pin_map);
    let ibuf_test_imux = ibuf_test_imux(&devinfo.device, &devinfo.imux);
    let bank_test_iob = devinfo
        .device
        .banks()
        .map(|bank| {
            pin_map
                .keys()
                .copied()
                .find_map(|io| {
                    if devinfo.device.io[&io].bank != bank {
                        return None;
                    }
                    match io {
                        IoCoord::Ipad(_) => None,
                        IoCoord::Macrocell(mc) => Some(mc),
                    }
                })
                .unwrap()
        })
        .collect();
    let oe_pads_remapped = devinfo
        .device
        .oe_pads
        .map_values(|&io| package.spec_remap.get(&io).copied().unwrap_or(io));
    let backend = CpldBackend {
        debug,
        tc,
        part,
        device: &devinfo.device,
        imux: &devinfo.imux,
        package,
        pin_map,
        imux_pinning,
        ibuf_test_imux,
        bank_test_iob,
        oe_pads_remapped,
    };
    let mut hammer = Session::new(&backend);
    hammer.debug = debug;

    add_fuzzers(&backend, &mut hammer);
    let state = hammer.run().unwrap();
    collect_fuzzers(&backend, state)
}
