use core::fmt::Debug;
use core::hash::Hash;
use std::collections::{BTreeMap, HashMap, btree_map, hash_map::Entry};

use crate::types::{
    BankId, CeMuxVal, ClkMuxVal, ClkPadId, ExportDir, FbGroupId, FbnId, FclkId, FoeId, FoeMuxVal,
    IBufMode, ImuxId, ImuxInput, OeMode, OeMuxVal, OePadId, PTermId, RegMode, Slew, SrMuxVal,
    TermMode, Ut, Xc9500McPt, XorMuxVal,
};
use bitvec::vec::BitVec;
use enum_map::EnumMap;
use itertools::Itertools;
use prjcombine_types::{
    tiledb::{TileBit, TileItem, TileItemKind}, FbId, FbMcId, IoId, IpadId
};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

pub type BitPos = (u32, usize);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BitstreamMap {
    pub main: Vec<BitPos>,
    pub usercode: Option<[BitPos; 32]>,
    pub ues: Option<Vec<BitPos>>,
    pub rprot: Vec<BitPos>,
    pub wprot: Vec<BitPos>,
    // cols × rows, abits
    pub dims: Option<(usize, usize, usize)>,
    pub transfer: Vec<usize>,
    pub done: Option<BitPos>,
}

pub type InvBit = (usize, bool);

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    // common
    pub fbs: EntityVec<FbId, FbBits>,
    pub ipads: EntityVec<IpadId, IPadBits>,
    pub fclk_mux: EntityVec<FclkId, EnumData<ClkPadId>>,
    pub fclk_en: EntityVec<FclkId, InvBit>,
    pub fclk_inv: EntityVec<FclkId, InvBit>,
    pub fsr_en: Option<InvBit>,
    pub fsr_inv: Option<InvBit>,
    pub foe_mux: EntityVec<FoeId, EnumData<OePadId>>,
    pub foe_en: EntityVec<FoeId, InvBit>,
    pub foe_inv: EntityVec<FoeId, InvBit>,
    pub foe_mux_xbr: EntityVec<FoeId, EnumData<FoeMuxVal>>,
    // XBR
    pub term_mode: Option<EnumData<TermMode>>,
    pub vref_en: Option<InvBit>,
    pub banks: EntityVec<BankId, BankBits>,
    pub dge_en: Option<InvBit>,
    pub clkdiv_en: Option<InvBit>,
    pub clkdiv_div: Option<EnumData<u8>>,
    pub clkdiv_dly_en: Option<InvBit>,
    // XPLA3
    pub ut: Option<EnumMap<Ut, EnumData<(FbId, PTermId)>>>,
    pub no_isp: Option<InvBit>,
    // XC9500
    pub usercode: Option<[InvBit; 32]>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FbBits {
    // common
    pub imux: EntityVec<ImuxId, EnumData<ImuxInput>>,
    // OG XC9500
    pub uim_mc: EntityVec<ImuxId, EntityVec<FbId, EntityVec<FbMcId, InvBit>>>,
    // XC9500*
    pub en: Option<InvBit>,
    pub exp_en: Option<InvBit>,
    // XPLA3, XBR
    pub pla_and: EntityVec<PTermId, PlaAndTerm>,
    // XPLA3
    pub ct_invert: EntityPartVec<PTermId, InvBit>,
    pub fbclk: Option<EnumData<(Option<ClkPadId>, Option<ClkPadId>)>>,
    // common
    pub mcs: EntityVec<FbMcId, McBits>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct McBits {
    // XC9500*
    pub pt: Option<EnumMap<Xc9500McPt, PtData>>,
    pub exp_dir: Option<EnumData<ExportDir>>,
    pub import: Option<EnumMap<ExportDir, InvBit>>,
    pub inv: Option<InvBit>,
    pub hp: Option<InvBit>,
    pub ff_en: Option<InvBit>,
    // XPLA3, XBR
    pub pla_or: EntityVec<PTermId, InvBit>,
    // XPLA3
    pub lut: Option<[InvBit; 4]>,
    // XBR
    pub xor_mux: Option<EnumData<XorMuxVal>>,
    // XPLA3, XBR
    pub use_ireg: Option<InvBit>,
    pub mc_uim_out: Option<EnumData<McOut>>,
    pub mc_obuf_out: Option<EnumData<McOut>>,
    pub ibuf_uim_out: Option<EnumData<IBufOut>>,
    // all but OG XC9500
    pub clk_inv: Option<InvBit>,
    pub ce_mux: Option<EnumData<CeMuxVal>>,
    // all
    pub clk_mux: EnumData<ClkMuxVal>,
    pub rst_mux: EnumData<SrMuxVal>,
    pub set_mux: EnumData<SrMuxVal>,
    pub reg_mode: EnumData<RegMode>,
    // all but XPLA3
    pub init: Option<InvBit>,
    // XBR
    pub ddr: Option<InvBit>,
    pub term: Option<InvBit>,
    pub ibuf_mode: Option<EnumData<IBufMode>>,
    pub dge_en: Option<InvBit>,
    // all
    pub slew: Option<EnumData<Slew>>,
    pub oe_mux: Option<EnumData<OeMuxVal>>,
    // XC9500
    pub uim_oe_mode: Option<EnumData<OeMode>>,
    pub uim_out_inv: Option<InvBit>,
    pub obuf_oe_mode: Option<EnumData<OeMode>>,
    // XC9500X*
    pub oe_inv: Option<InvBit>,
    // XC9500*
    pub is_gnd: Option<InvBit>,
    // XC95288 only
    pub ibuf_uim_en: Vec<InvBit>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IPadBits {
    // XPLA3
    pub uim_out_en: EntityVec<FbGroupId, InvBit>,
    // XBR
    pub term: Option<InvBit>,
    pub ibuf_mode: Option<EnumData<IBufMode>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BankBits {
    pub ibuf_hv: InvBit,
    pub obuf_hv: InvBit,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnumData<K: Clone + Debug + Eq + PartialEq + Hash> {
    pub bits: Vec<usize>,
    pub items: HashMap<K, BitVec>,
    pub default: BitVec,
}

impl<K: Clone + Debug + Eq + PartialEq + Hash> EnumData<K> {
    pub fn empty() -> Self {
        Self {
            bits: vec![],
            items: HashMap::new(),
            default: BitVec::new(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaAndTerm {
    pub imux: EntityVec<ImuxId, (InvBit, InvBit)>,
    pub fbn: EntityVec<FbnId, InvBit>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PtAlloc {
    OrMain,
    OrExport,
    Special,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum McOut {
    Comb,
    Reg,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IBufOut {
    Pad,
    Reg,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PtData {
    pub and: EntityVec<ImuxId, (InvBit, InvBit)>,
    pub hp: InvBit,
    pub alloc: EnumData<PtAlloc>,
}

impl Bits {
    pub fn known_bits(&self) -> HashMap<usize, String> {
        let mut res = HashMap::new();
        let mut set = |b, s| match res.entry(b) {
            Entry::Occupied(e) => panic!("double claimed bit {b}: {c} and {s}", c = e.get()),
            Entry::Vacant(e) => {
                e.insert(s);
            }
        };
        for (fbid, fb) in &self.fbs {
            let f = fbid.to_idx();
            for (imid, im) in &fb.imux {
                for (i, &bit) in im.bits.iter().enumerate() {
                    set(bit, format!("FB{f}.IMUX{im}.{i}", im = imid.to_idx()));
                }
            }
            for (imid, im) in &fb.uim_mc {
                for (ifbid, ifb) in im {
                    for (imcid, &bit) in ifb {
                        set(
                            bit.0,
                            format!(
                                "FB{f}.IMUX{im}.UIM.FB{ifb}.MC{imc}",
                                im = imid.to_idx(),
                                ifb = ifbid.to_idx(),
                                imc = imcid.to_idx()
                            ),
                        );
                    }
                }
            }
            if let Some(bit) = fb.en {
                set(bit.0, format!("FB{f}.EN"));
            }
            if let Some(bit) = fb.exp_en {
                set(bit.0, format!("FB{f}.EXP_EN"));
            }
            for (ptid, pt) in &fb.pla_and {
                let p = ptid.to_idx();
                for (imid, &(bt, bf)) in &pt.imux {
                    let im = imid.to_idx();
                    set(bt.0, format!("FB{f}.PT{p}.IMUX{im}.T"));
                    set(bf.0, format!("FB{f}.PT{p}.IMUX{im}.F"));
                }
                for (fbnid, &bit) in &pt.fbn {
                    set(
                        bit.0,
                        format!("FB{f}.PT{p}.FBN{fbnid}", fbnid = fbnid.to_idx()),
                    );
                }
            }
            for (ptid, &bit) in &fb.ct_invert {
                set(bit.0, format!("FB{f}.CT{pt}.INV", pt = ptid.to_idx()));
            }
            if let Some(ref data) = fb.fbclk {
                for (i, &bit) in data.bits.iter().enumerate() {
                    set(bit, format!("FB{f}.FBCLK.{i}"));
                }
            }
            for (mcid, mc) in &fb.mcs {
                let m = mcid.to_idx();
                if let Some(ref pts) = mc.pt {
                    for (ptid, pt) in pts {
                        let p = match ptid {
                            Xc9500McPt::Clk => "CLK",
                            Xc9500McPt::Oe => "OE",
                            Xc9500McPt::Rst => "RST",
                            Xc9500McPt::Set => "SET",
                            Xc9500McPt::Xor => "XOR",
                        };
                        for (imid, &(bt, bf)) in &pt.and {
                            let im = imid.to_idx();
                            set(bt.0, format!("FB{f}.MC{m}.PT{p}.IMUX{im}.T"));
                            set(bf.0, format!("FB{f}.MC{m}.PT{p}.IMUX{im}.F"));
                        }
                        set(pt.hp.0, format!("FB{f}.MC{m}.PT{p}.HP"));
                        for (i, &bit) in pt.alloc.bits.iter().enumerate() {
                            set(bit, format!("FB{f}.MC{m}.PT{p}.ALLOC.{i}"));
                        }
                    }
                }
                if let Some(ref data) = mc.exp_dir {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.EXP_DIR.{i}"));
                    }
                }
                if let Some(ref imp) = mc.import {
                    for (k, v) in imp {
                        let d = match k {
                            ExportDir::Up => "UP",
                            ExportDir::Down => "DOWN",
                        };
                        set(v.0, format!("FB{f}.MC{m}.IMPORT.{d}"));
                    }
                }
                if let Some(bit) = mc.inv {
                    set(bit.0, format!("FB{f}.MC{m}.INV"));
                }
                if let Some(bit) = mc.hp {
                    set(bit.0, format!("FB{f}.MC{m}.HP"));
                }
                if let Some(bit) = mc.ff_en {
                    set(bit.0, format!("FB{f}.MC{m}.FF_EN"));
                }
                for (ptid, &bit) in &mc.pla_or {
                    let p = ptid.to_idx();
                    set(bit.0, format!("FB{f}.MC{m}.PLA_OR.PT{p}"));
                }
                if let Some(lut) = mc.lut {
                    for (i, bit) in lut.into_iter().enumerate() {
                        set(bit.0, format!("FB{f}.MC{m}.LUT.{i}"));
                    }
                }
                if let Some(ref data) = mc.xor_mux {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.XOR_MUX.{i}"));
                    }
                }
                if let Some(bit) = mc.use_ireg {
                    set(bit.0, format!("FB{f}.MC{m}.USE_IREG"));
                }
                if let Some(ref data) = mc.mc_uim_out {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.MC_UIM_OUT.{i}"));
                    }
                }
                if let Some(ref data) = mc.mc_obuf_out {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.MC_OBUF_OUT.{i}"));
                    }
                }
                if let Some(ref data) = mc.ibuf_uim_out {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.IBUF_UIM_OUT.{i}"));
                    }
                }
                if let Some(bit) = mc.clk_inv {
                    set(bit.0, format!("FB{f}.MC{m}.CLK_INV"));
                }
                if let Some(ref data) = mc.ce_mux {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.CE_MUX.{i}"));
                    }
                }
                for (i, &bit) in mc.clk_mux.bits.iter().enumerate() {
                    set(bit, format!("FB{f}.MC{m}.CLK_MUX.{i}"));
                }
                for (i, &bit) in mc.rst_mux.bits.iter().enumerate() {
                    set(bit, format!("FB{f}.MC{m}.RST_MUX.{i}"));
                }
                for (i, &bit) in mc.set_mux.bits.iter().enumerate() {
                    set(bit, format!("FB{f}.MC{m}.SET_MUX.{i}"));
                }
                for (i, &bit) in mc.reg_mode.bits.iter().enumerate() {
                    set(bit, format!("FB{f}.MC{m}.REG_MODE.{i}"));
                }
                if let Some(bit) = mc.init {
                    set(bit.0, format!("FB{f}.MC{m}.INIT"));
                }
                if let Some(bit) = mc.ddr {
                    set(bit.0, format!("FB{f}.MC{m}.DDR"));
                }
                if let Some(bit) = mc.term {
                    set(bit.0, format!("FB{f}.MC{m}.TERM"));
                }
                if let Some(ref data) = mc.ibuf_mode {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.IBUF_MODE.{i}"));
                    }
                }
                if let Some(bit) = mc.dge_en {
                    set(bit.0, format!("FB{f}.MC{m}.DGE_EN"));
                }
                if let Some(ref data) = mc.slew {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.SLEW.{i}"));
                    }
                }
                if let Some(ref data) = mc.oe_mux {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.OE_MUX.{i}"));
                    }
                }
                if let Some(ref data) = mc.uim_oe_mode {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.UIM_OE_MODE.{i}"));
                    }
                }
                if let Some(bit) = mc.uim_out_inv {
                    set(bit.0, format!("FB{f}.MC{m}.UIM_OUT_INV"));
                }
                if let Some(ref data) = mc.obuf_oe_mode {
                    for (i, &bit) in data.bits.iter().enumerate() {
                        set(bit, format!("FB{f}.MC{m}.OBUF_OE_MODE.{i}"));
                    }
                }
                if let Some(bit) = mc.oe_inv {
                    set(bit.0, format!("FB{f}.MC{m}.OE_INV"));
                }
                if let Some(bit) = mc.is_gnd {
                    set(bit.0, format!("FB{f}.MC{m}.IS_GND"));
                }
                for (i, &bit) in mc.ibuf_uim_en.iter().enumerate() {
                    set(bit.0, format!("FB{f}.MC{m}.IBUF_UIM_EN.{i}"));
                }
            }
        }

        for (ipid, ipad) in &self.ipads {
            let ip = ipid.to_idx();
            for (fbg, &bit) in &ipad.uim_out_en {
                set(
                    bit.0,
                    format!("IPAD{ip}.UIM_OUT_EN.{fbg}", fbg = fbg.to_idx()),
                );
            }
            if let Some(bit) = ipad.term {
                set(bit.0, format!("IPAD{ip}.TERM"));
            }
            if let Some(ref data) = ipad.ibuf_mode {
                for (i, &bit) in data.bits.iter().enumerate() {
                    set(bit, format!("IPAD{ip}.IBUF_MODE.{i}"));
                }
            }
        }

        for (fclk, data) in &self.fclk_mux {
            let f = fclk.to_idx();
            for (j, &bit) in data.bits.iter().enumerate() {
                set(bit, format!("FCLK{f}.MUX.{j}"));
            }
        }
        for (fclk, &bit) in &self.fclk_en {
            let f = fclk.to_idx();
            set(bit.0, format!("FCLK{f}.EN"));
        }
        for (fclk, &bit) in &self.fclk_inv {
            let f = fclk.to_idx();
            set(bit.0, format!("FCLK{f}.INV"));
        }

        if let Some(bit) = self.fsr_en {
            set(bit.0, "FSR.EN".to_string());
        }
        if let Some(bit) = self.fsr_inv {
            set(bit.0, "FSR.INV".to_string());
        }

        for (foe, data) in &self.foe_mux {
            let f = foe.to_idx();
            for (j, &bit) in data.bits.iter().enumerate() {
                set(bit, format!("FOE{f}.MUX.{j}"));
            }
        }
        for (foe, &bit) in &self.foe_en {
            let f = foe.to_idx();
            set(bit.0, format!("FOE{f}.EN"));
        }
        for (foe, &bit) in &self.foe_inv {
            let f = foe.to_idx();
            set(bit.0, format!("FOE{f}.INV"));
        }
        for (foe, data) in &self.foe_mux_xbr {
            let f = foe.to_idx();
            for (j, &bit) in data.bits.iter().enumerate() {
                set(bit, format!("FOE{f}.MUX.{j}"));
            }
        }

        if let Some(ref data) = self.term_mode {
            for (i, &bit) in data.bits.iter().enumerate() {
                set(bit, format!("TERM.{i}"));
            }
        }
        if let Some(bit) = self.vref_en {
            set(bit.0, "VREF_EN".to_string());
        }
        for (bid, bank) in &self.banks {
            set(
                bank.ibuf_hv.0,
                format!("BANK{bid}.IBUF_HV", bid = bid.to_idx()),
            );
            set(
                bank.obuf_hv.0,
                format!("BANK{bid}.OBUF_HV", bid = bid.to_idx()),
            );
        }
        if let Some(bit) = self.dge_en {
            set(bit.0, "DGE_EN".to_string());
        }
        if let Some(bit) = self.clkdiv_en {
            set(bit.0, "CLKDIV.EN".to_string());
        }
        if let Some(bit) = self.clkdiv_dly_en {
            set(bit.0, "CLKDIV.DLY_EN".to_string());
        }
        if let Some(ref data) = self.clkdiv_div {
            for (i, &bit) in data.bits.iter().enumerate() {
                set(bit, format!("CLKDIV.DIV.{i}"));
            }
        }
        if let Some(ref uts) = self.ut {
            for (ut, data) in uts {
                for (i, &bit) in data.bits.iter().enumerate() {
                    set(
                        bit,
                        format!(
                            "UT{ut}.{i}",
                            ut = match ut {
                                Ut::Clk => "CLK",
                                Ut::Oe => "OE",
                                Ut::Rst => "RST",
                                Ut::Set => "SET",
                            }
                        ),
                    );
                }
            }
        }
        if let Some(bit) = self.no_isp {
            set(bit.0, "NO_ISP".to_string());
        }
        if let Some(bits) = self.usercode {
            for (i, bit) in bits.into_iter().enumerate() {
                set(bit.0, format!("USERCODE.{i}"));
            }
        }

        res
    }

    pub fn print(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (fbid, fb) in &self.fbs {
            writeln!(o, "FB{fbid}:", fbid = fbid.to_idx())?;

            for (imid, data) in &fb.imux {
                write!(o, "\tIMUX IM{imid}: ", imid = imid.to_idx())?;
                write_enum(o, "\t", data, |k| match k {
                    ImuxInput::Ibuf(IoId::Mc((fb, mc))) => format!("MC IBUF FB{fb} MC{mc}"),
                    ImuxInput::Ibuf(IoId::Ipad(ip)) => format!("IPAD{ip}"),
                    ImuxInput::Fbk(mc) => format!("FBK MC{mc}"),
                    ImuxInput::Mc((fb, mc)) => format!("MC FB{fb} MC{mc}"),
                    ImuxInput::Pup => "PUP".to_string(),
                    ImuxInput::Uim => "UIM".to_string(),
                })?;
            }
            for (imid, fbs) in &fb.uim_mc {
                for (ifbid, mcs) in fbs {
                    write!(
                        o,
                        "\tIMUX UIM IM{imid} <- FB{ifbid}:",
                        imid = imid.to_idx(),
                        ifbid = ifbid.to_idx()
                    )?;
                    for (imcid, &b) in mcs {
                        write!(o, " MC{imcid}: ", imcid = imcid.to_idx())?;
                        write_invbit(o, b)?;
                    }
                    writeln!(o)?;
                }
            }

            if let Some(b) = fb.en {
                write!(o, "\tEN: ")?;
                write_invbit(o, b)?;
                writeln!(o)?;
            }

            if let Some(b) = fb.exp_en {
                write!(o, "\tEXP EN: ")?;
                write_invbit(o, b)?;
                writeln!(o)?;
            }

            if !fb.pla_and.is_empty() {
                for (ptid, term) in &fb.pla_and {
                    write!(o, "\tPLA AND PT{ptid}:", ptid = ptid.to_idx())?;
                    for (imid, &(bt, bf)) in &term.imux {
                        write!(o, " IM{imid}: ", imid = imid.to_idx())?;
                        write_invbit(o, bt)?;
                        write!(o, " ")?;
                        write_invbit(o, bf)?;
                    }
                    for (fbnid, &b) in &term.fbn {
                        write!(o, " FBN{fbnid}: ", fbnid = fbnid.to_idx())?;
                        write_invbit(o, b)?;
                    }
                    writeln!(o)?;
                }
            }

            if fb.ct_invert.iter().next().is_some() {
                write!(o, "\tCT INVERT:")?;
                for (ptid, &b) in &fb.ct_invert {
                    write!(o, " PT{ptid}: ", ptid = ptid.to_idx())?;
                    write_invbit(o, b)?;
                }
                writeln!(o)?;
            }

            if let Some(ref data) = fb.fbclk {
                write!(o, "\tFBCLK:")?;
                write_enum(o, "\t", data, |&(c0, c1)| {
                    let c0 = if let Some(c0) = c0 {
                        format!("{c0}", c0 = c0.to_idx())
                    } else {
                        "-".to_string()
                    };
                    let c1 = if let Some(c1) = c1 {
                        format!("{c1}", c1 = c1.to_idx())
                    } else {
                        "-".to_string()
                    };
                    c0 + &c1
                })?;
            }

            for (mcid, mc) in &fb.mcs {
                writeln!(o, "\tMC{mcid}:", mcid = mcid.to_idx())?;

                if let Some(ref pts) = mc.pt {
                    for (ptid, data) in pts {
                        let pt = match ptid {
                            Xc9500McPt::Clk => "CLK",
                            Xc9500McPt::Oe => "OE ",
                            Xc9500McPt::Rst => "RST",
                            Xc9500McPt::Set => "SET",
                            Xc9500McPt::Xor => "XOR",
                        };
                        write!(o, "\t\tPT AND PT{pt}:")?;
                        for (imid, &(bt, bf)) in &data.and {
                            write!(o, " IM{imid}: ", imid = imid.to_idx())?;
                            write_invbit(o, bt)?;
                            write!(o, " ")?;
                            write_invbit(o, bf)?;
                        }
                        writeln!(o)?;
                        write!(o, "\t\tPT HP PT{pt}: ")?;
                        write_invbit(o, data.hp)?;
                        writeln!(o)?;

                        write!(o, "\t\tPT ALLOC PT{pt}: ",)?;
                        write_enum(o, "\t\t", &data.alloc, |k| match k {
                            PtAlloc::OrMain => "OR_MAIN".to_string(),
                            PtAlloc::OrExport => "OR_EXPORT".to_string(),
                            PtAlloc::Special => "SPECIAL".to_string(),
                        })?;
                    }
                }

                if !mc.pla_or.is_empty() {
                    write!(o, "\t\tPLA OR:")?;
                    for (ptid, &b) in &mc.pla_or {
                        write!(o, " PT{ptid}: ", ptid = ptid.to_idx())?;
                        write_invbit(o, b)?;
                    }
                    writeln!(o)?;
                }

                if let Some(ref data) = mc.exp_dir {
                    write!(o, "\t\tEXP DIR: ")?;
                    write_enum(o, "\t\t", data, |d| {
                        match d {
                            ExportDir::Up => "UP",
                            ExportDir::Down => "DOWN",
                        }
                        .to_string()
                    })?;
                }

                if let Some(dirs) = mc.import {
                    write!(o, "\t\tIMPORT:",)?;
                    for (dir, b) in dirs {
                        write!(
                            o,
                            " {dir}: ",
                            dir = match dir {
                                ExportDir::Down => "DOWN",
                                ExportDir::Up => "UP",
                            }
                        )?;
                        write_invbit(o, b)?;
                    }
                    writeln!(o)?;
                }

                if let Some(bit) = mc.inv {
                    write!(o, "\t\tINV: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }
                if let Some(bit) = mc.hp {
                    write!(o, "\t\tHP: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(ref lut) = mc.lut {
                    write!(o, "\t\tMC LUT:")?;
                    for &b in lut {
                        write!(o, " ")?;
                        write_invbit(o, b)?;
                    }
                    writeln!(o)?;
                }

                if let Some(bit) = mc.ff_en {
                    write!(o, "\t\tFF EN: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(ref data) = mc.xor_mux {
                    write!(o, "\t\tXOR MUX: ")?;
                    write_enum(o, "\t\t", data, |k| {
                        match k {
                            XorMuxVal::Gnd => "GND",
                            XorMuxVal::Vcc => "VCC",
                            XorMuxVal::Pt => "PT",
                            XorMuxVal::PtInv => "PT_INV",
                        }
                        .to_string()
                    })?;
                }

                write!(o, "\t\tCLK MUX: ")?;
                write_enum(o, "\t\t", &mc.clk_mux, |k| match k {
                    ClkMuxVal::Pt => "PT".to_string(),
                    ClkMuxVal::Fclk(i) => format!("FCLK{i}", i = i.to_idx()),
                    ClkMuxVal::Ct(pt) => format!("CT{i}", i = pt.to_idx()),
                    ClkMuxVal::Ut => "UT".to_string(),
                })?;

                if let Some(bit) = mc.clk_inv {
                    write!(o, "\t\tCLK INV: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                write!(o, "\t\tRST MUX: ")?;
                write_enum(o, "\t\t", &mc.rst_mux, |k| match k {
                    SrMuxVal::Pt => "PT".to_string(),
                    SrMuxVal::Fsr => "FSR".to_string(),
                    SrMuxVal::Ct(pt) => format!("CT{i}", i = pt.to_idx()),
                    SrMuxVal::Ut => "UT".to_string(),
                    SrMuxVal::Gnd => "GND".to_string(),
                })?;

                write!(o, "\t\tSET MUX: ")?;
                write_enum(o, "\t\t", &mc.set_mux, |k| match k {
                    SrMuxVal::Pt => "PT".to_string(),
                    SrMuxVal::Fsr => "FSR".to_string(),
                    SrMuxVal::Ct(pt) => format!("CT{i}", i = pt.to_idx()),
                    SrMuxVal::Ut => "UT".to_string(),
                    SrMuxVal::Gnd => "GND".to_string(),
                })?;

                if let Some(ref data) = mc.ce_mux {
                    write!(o, "\t\tCE MUX: ")?;
                    write_enum(o, "\t\t", data, |k| match k {
                        CeMuxVal::PtRst => "PTRST".to_string(),
                        CeMuxVal::PtSet => "PTSET".to_string(),
                        CeMuxVal::Pt => "PT".to_string(),
                        CeMuxVal::Ct(pt) => format!("CT{i}", i = pt.to_idx()),
                    })?;
                }

                write!(o, "\t\tREG MODE: ")?;
                write_enum(o, "\t\t", &mc.reg_mode, |k| {
                    match k {
                        RegMode::Dff => "DFF",
                        RegMode::Tff => "TFF",
                        RegMode::Latch => "LATCH",
                        RegMode::DffCe => "DFFCE",
                    }
                    .to_string()
                })?;

                if let Some(bit) = mc.init {
                    write!(o, "\t\tINIT: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(bit) = mc.ddr {
                    write!(o, "\t\tDDR: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(bit) = mc.use_ireg {
                    write!(o, "\t\tUSE IREG: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(ref data) = mc.mc_uim_out {
                    write!(o, "\t\tMC UIM OUT: ")?;
                    write_enum(o, "\t\t", data, |k| {
                        match k {
                            McOut::Comb => "COMB",
                            McOut::Reg => "REG",
                        }
                        .to_string()
                    })?;
                }
                if let Some(ref data) = mc.mc_obuf_out {
                    write!(o, "\t\tMC OBUF OUT: ")?;
                    write_enum(o, "\t\t", data, |k| {
                        match k {
                            McOut::Comb => "COMB",
                            McOut::Reg => "REG",
                        }
                        .to_string()
                    })?;
                }
                if let Some(ref data) = mc.ibuf_uim_out {
                    write!(o, "\t\tIBUF UIM OUT: ")?;
                    write_enum(o, "\t\t", data, |k| {
                        match k {
                            IBufOut::Pad => "PAD",
                            IBufOut::Reg => "REG",
                        }
                        .to_string()
                    })?;
                }

                if let Some(bit) = mc.term {
                    write!(o, "\t\tTERM: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }
                if let Some(ref data) = mc.ibuf_mode {
                    write!(o, "\t\tIBUF MODE: ")?;
                    write_enum(o, "\t\t", data, |k| {
                        match k {
                            IBufMode::Plain => "PLAIN",
                            IBufMode::Schmitt => "SCHMITT",
                            IBufMode::UseVref => "USE_VREF",
                            IBufMode::IsVref => "IS_VREF",
                        }
                        .to_string()
                    })?;
                }

                if let Some(bit) = mc.dge_en {
                    write!(o, "\t\tDGE EN: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(ref data) = mc.slew {
                    write!(o, "\t\tSLEW RATE: ")?;
                    write_enum(o, "\t\t", data, |k| {
                        match k {
                            Slew::Slow => "SLOW",
                            Slew::Fast => "FAST",
                        }
                        .to_string()
                    })?;
                }

                if let Some(ref data) = mc.oe_mux {
                    write!(o, "\t\tOE MUX: ")?;
                    write_enum(o, "\t\t", data, |k| match k {
                        OeMuxVal::Pt => "PT".to_string(),
                        OeMuxVal::Foe(i) => format!("FOE{i}", i = i.to_idx()),
                        OeMuxVal::Ct(pt) => format!("CT{i}", i = pt.to_idx()),
                        OeMuxVal::Ut => "UT".to_string(),
                        OeMuxVal::Gnd => "GND".to_string(),
                        OeMuxVal::Vcc => "VCC".to_string(),
                        OeMuxVal::OpenDrain => "OPEN_DRAIN".to_string(),
                        OeMuxVal::Pullup => "PULLUP".to_string(),
                        OeMuxVal::IsGround => "IS_GND".to_string(),
                    })?;
                }

                if let Some(ref data) = mc.uim_oe_mode {
                    write!(o, "\t\tUIM OE MODE: ")?;
                    write_enum(o, "\t\t", data, |k| match k {
                        OeMode::Gnd => "GND".to_string(),
                        OeMode::Vcc => "VCC".to_string(),
                        OeMode::McOe => "MC_OE".to_string(),
                    })?;
                }

                if let Some(bit) = mc.uim_out_inv {
                    write!(o, "\t\tUIM OUT INV: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(ref data) = mc.obuf_oe_mode {
                    write!(o, "\t\tOBUF OE MODE: ")?;
                    write_enum(o, "\t\t", data, |k| match k {
                        OeMode::Gnd => "GND".to_string(),
                        OeMode::Vcc => "VCC".to_string(),
                        OeMode::McOe => "MC_OE".to_string(),
                    })?;
                }

                if let Some(bit) = mc.oe_inv {
                    write!(o, "\t\tOE INV: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }

                if let Some(bit) = mc.is_gnd {
                    write!(o, "\t\tIS GND: ")?;
                    write_invbit(o, bit)?;
                    writeln!(o)?;
                }
                if !mc.ibuf_uim_en.is_empty() {
                    write!(o, "\t\tIBUF UIM EN:")?;
                    for &bit in &mc.ibuf_uim_en {
                        write!(o, " ")?;
                        write_invbit(o, bit)?;
                    }
                    writeln!(o)?;
                }
            }
        }
        for (ipid, ipad) in &self.ipads {
            writeln!(o, "IPAD{ipid}:", ipid = ipid.to_idx())?;
            for (fbg, &bit) in &ipad.uim_out_en {
                write!(o, "\t\tUIM OUT EN FBG{fbg}: ", fbg = fbg.to_idx())?;
                write_invbit(o, bit)?;
                writeln!(o)?;
            }
            if let Some(bit) = ipad.term {
                write!(o, "\t\tTERM: ")?;
                write_invbit(o, bit)?;
                writeln!(o)?;
            }
            if let Some(ref data) = ipad.ibuf_mode {
                write!(o, "\t\tIBUF MODE: ")?;
                write_enum(o, "\t\t", data, |k| {
                    match k {
                        IBufMode::Plain => "PLAIN",
                        IBufMode::Schmitt => "SCHMITT",
                        IBufMode::UseVref => "USE_VREF",
                        IBufMode::IsVref => "IS_VREF",
                    }
                    .to_string()
                })?;
            }
        }

        for (fclk, data) in &self.fclk_mux {
            write!(o, "FCLK MUX {i}:", i = fclk.to_idx())?;
            write_enum(o, "", data, |k| k.to_idx().to_string())?;
        }

        for (fclk, &bit) in &self.fclk_en {
            write!(o, "FCLK EN {i}: ", i = fclk.to_idx())?;
            write_invbit(o, bit)?;
            writeln!(o)?;
        }

        for (fclk, &bit) in &self.fclk_inv {
            write!(o, "FCLK INV {i}: ", i = fclk.to_idx())?;
            write_invbit(o, bit)?;
            writeln!(o)?;
        }

        if let Some(bit) = self.fsr_en {
            write!(o, "FSR EN: ")?;
            write_invbit(o, bit)?;
            writeln!(o)?;
        }

        if let Some(bit) = self.fsr_inv {
            write!(o, "FSR INV: ")?;
            write_invbit(o, bit)?;
            writeln!(o)?;
        }

        for (foe, data) in &self.foe_mux {
            write!(o, "FOE MUX {i}:", i = foe.to_idx())?;
            write_enum(o, "", data, |k| k.to_idx().to_string())?;
        }

        for (foe, &bit) in &self.foe_en {
            write!(o, "FOE EN {i}: ", i = foe.to_idx())?;
            write_invbit(o, bit)?;
            writeln!(o)?;
        }

        for (foe, &bit) in &self.foe_inv {
            write!(o, "FOE INV {i}: ", i = foe.to_idx())?;
            write_invbit(o, bit)?;
            writeln!(o)?;
        }

        for (foe, data) in &self.foe_mux_xbr {
            write!(o, "FOE MUX {i}:", i = foe.to_idx())?;
            write_enum(o, "", data, |k| {
                match k {
                    FoeMuxVal::Ibuf => "IBUF",
                    FoeMuxVal::IbufInv => "IBUF_INV",
                    FoeMuxVal::Mc => "MC",
                }
                .to_string()
            })?;
        }

        for (bid, bank) in &self.banks {
            writeln!(o, "BANK {bid}:", bid = bid.to_idx())?;

            write!(o, "\tIBUF HV: ")?;
            write_invbit(o, bank.ibuf_hv)?;
            writeln!(o)?;

            write!(o, "\tOBUF HV: ")?;
            write_invbit(o, bank.obuf_hv)?;
            writeln!(o)?;
        }

        if let Some(b) = self.vref_en {
            write!(o, "USE VREF: ")?;
            write_invbit(o, b)?;
            writeln!(o)?;
        }

        if let Some(ref data) = self.term_mode {
            write!(o, "TERM MODE: ")?;
            write_enum(o, "", data, |k| {
                match k {
                    TermMode::Pullup => "PULLUP",
                    TermMode::Keeper => "KEEPER",
                }
                .to_string()
            })?;
        }

        if let Some(b) = self.dge_en {
            write!(o, "DGE EN: ")?;
            write_invbit(o, b)?;
            writeln!(o)?;
        }

        if let Some(b) = self.clkdiv_en {
            write!(o, "CLKDIV EN: ")?;
            write_invbit(o, b)?;
            writeln!(o)?;
        }

        if let Some(ref data) = self.clkdiv_div {
            write!(o, "CLKDIV DIV: ")?;
            write_enum(o, "", data, |k| format!("{k}"))?;
        }

        if let Some(b) = self.clkdiv_dly_en {
            write!(o, "CLKDIV DLY EN: ")?;
            write_invbit(o, b)?;
            writeln!(o)?;
        }

        if let Some(ref uts) = self.ut {
            for (ut, data) in uts {
                write!(
                    o,
                    "UT {ut}: ",
                    ut = match ut {
                        Ut::Clk => "CLK",
                        Ut::Oe => "OE",
                        Ut::Rst => "RST",
                        Ut::Set => "SET",
                    }
                )?;
                write_enum(o, "", data, |k| {
                    format!(
                        "FB{fbid} PT{ptid}",
                        fbid = k.0.to_idx(),
                        ptid = k.1.to_idx()
                    )
                })?;
            }
        }

        if let Some(b) = self.no_isp {
            write!(o, "NO ISP: ")?;
            write_invbit(o, b)?;
            writeln!(o)?;
        }
        if let Some(ref bits) = self.usercode {
            write!(o, "USERCODE:")?;
            for (i, &b) in bits.iter().enumerate() {
                write!(o, " {i:2}: ")?;
                write_invbit(o, b)?;
            }
            writeln!(o)?;
        }

        fn write_invbit(o: &mut dyn std::io::Write, bit: InvBit) -> std::io::Result<()> {
            write!(o, "{b:6}{p}", b = bit.0, p = if bit.1 { "+" } else { "-" })
        }
        fn write_bitvec(o: &mut dyn std::io::Write, bv: &BitVec) -> std::io::Result<()> {
            for b in bv {
                write!(o, "{}", if *b { '1' } else { '0' })?;
            }
            Ok(())
        }
        fn write_enum<K: Debug + Clone + PartialEq + Eq + PartialOrd + Ord + Hash>(
            o: &mut dyn std::io::Write,
            indent: &str,
            data: &EnumData<K>,
            f: impl Fn(&K) -> String,
        ) -> std::io::Result<()> {
            for bit in &data.bits {
                write!(o, " {bit:6}")?;
            }
            writeln!(o)?;
            for (k, v) in data.items.iter().sorted() {
                write!(o, "{indent}\t")?;
                write_bitvec(o, v)?;
                writeln!(o, ": {kk}", kk = f(k))?;
            }
            write!(o, "{indent}\t")?;
            write_bitvec(o, &data.default)?;
            writeln!(o, ": EMPTY")?;
            Ok(())
        }
        Ok(())
    }
}

pub fn extract_bool(bit: InvBit, xlat_bit: impl Fn(usize) -> TileBit) -> TileItem {
    let (bit, pol) = bit;
    let bits = vec![xlat_bit(bit)];
    TileItem {
        bits,
        kind: TileItemKind::BitVec {
            invert: BitVec::from_iter([!pol]),
        },
    }
}

pub fn extract_bitvec(bits: &[InvBit], xlat_bit: impl Fn(usize) -> TileBit) -> TileItem {
    let pol = bits[0].1;
    let new_bits = bits
        .iter()
        .map(|&(bit, p)| {
            assert_eq!(p, pol);
            xlat_bit(bit)
        })
        .collect();
    TileItem {
        bits: new_bits,
        kind: TileItemKind::BitVec {
            invert: BitVec::repeat(!pol, bits.len()),
        },
    }
}

pub fn extract_bool_to_enum(
    bit: InvBit,
    xlat_bit: impl Fn(usize) -> TileBit,
    val_true: impl Into<String>,
    val_false: impl Into<String>,
) -> TileItem {
    let val_true = val_true.into();
    let val_false = val_false.into();
    let (bit, pol) = bit;
    let bits = vec![xlat_bit(bit)];
    TileItem {
        bits,
        kind: TileItemKind::Enum {
            values: [
                (val_true, BitVec::repeat(pol, 1)),
                (val_false, BitVec::repeat(!pol, 1)),
            ]
            .into_iter()
            .collect(),
        },
    }
}

pub fn extract_enum<T: Clone + Debug + Eq + core::hash::Hash>(
    enum_: &EnumData<T>,
    xlat_val: impl Fn(&T) -> String,
    xlat_bit: impl Fn(usize) -> TileBit,
    default: impl Into<String>,
) -> TileItem {
    let default = default.into();
    let bits = enum_.bits.iter().map(|&bit| xlat_bit(bit)).collect();
    let mut values: BTreeMap<_, _> = enum_
        .items
        .iter()
        .map(|(k, v)| (xlat_val(k), v.clone()))
        .collect();
    match values.entry(default.clone()) {
        btree_map::Entry::Vacant(e) => {
            e.insert(enum_.default.clone());
        }
        btree_map::Entry::Occupied(e) => {
            assert_eq!(*e.get(), enum_.default);
        }
    }
    TileItem {
        bits,
        kind: TileItemKind::Enum { values },
    }
}
