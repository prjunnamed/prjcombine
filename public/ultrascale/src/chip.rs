use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{
    EntityId, EntityRange, EntityVec,
    id::{EntityIdU8, EntityTag, EntityTagArith},
};
use prjcombine_interconnect::{
    dir::DirH,
    grid::{ColId, DieId, RowId, TileIobId},
};
use std::collections::BTreeSet;

pub struct RegTag;
impl EntityTag for RegTag {
    const PREFIX: &'static str = "REG";
}
impl EntityTagArith for RegTag {}
pub type RegId = EntityIdU8<RegTag>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ChipKind {
    Ultrascale,
    UltrascalePlus,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Ultrascale => write!(f, "ultrascale"),
            ChipKind::UltrascalePlus => write!(f, "ultrascaleplus"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Interposer {
    pub primary: DieId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_fsr_gap: BTreeSet<ColId>,
    pub cols_hard: Vec<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub regs: usize,
    pub ps: Option<Ps>,
    pub has_hbm: bool,
    pub config_kind: ConfigKind,
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ConfigKind {
    Config,
    Csec,
    CsecV2,
}

impl ConfigKind {
    pub fn is_csec(self) -> bool {
        matches!(self, Self::Csec | Self::CsecV2)
    }
}

impl std::fmt::Display for ConfigKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigKind::Config => write!(f, "config"),
            ConfigKind::Csec => write!(f, "csec"),
            ConfigKind::CsecV2 => write!(f, "csec_v2"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ColumnKind {
    // both W and E; W can only be plain
    CleL(CleLKind),
    // W only
    CleM(CleMKind),
    // W only
    Bram(BramKind),
    // E only
    Dsp(DspKind),
    // E only; creates a ContUram on the next W
    Uram,
    // E only; creates a ContHard on the next W (unless this is the final column)
    Hard(HardKind, usize),
    // both W and E
    Io(usize),
    // both W and E
    Gt(usize),
    // W only
    Sdfec,
    // E only
    DfeB,
    // E only; creates a ContHard on the next W
    DfeC,
    // E only; creates a ContHard on the next W
    DfeDF,
    // E only; creates a ContHard on the next W
    DfeE,
    // E only; creates a ContHard on the next W
    HdioS,
    // URAM continuation, W only
    ContUram,
    // hard or DFE continuation, W only
    ContHard,
}

impl std::fmt::Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnKind::CleL(CleLKind::Plain) => write!(f, "clel"),
            ColumnKind::CleL(CleLKind::Dcg10) => write!(f, "clel[dcg10]"),
            ColumnKind::CleM(CleMKind::Plain) => write!(f, "clem"),
            ColumnKind::CleM(CleMKind::ClkBuf) => write!(f, "clem + clkbuf"),
            ColumnKind::CleM(CleMKind::Laguna) => write!(f, "clem + laguna"),
            ColumnKind::Bram(BramKind::Plain) => write!(f, "bram"),
            ColumnKind::Bram(BramKind::Td) => write!(f, "bram[td]"),
            ColumnKind::Bram(BramKind::AuxClmp) => write!(f, "bram + auxclmp"),
            ColumnKind::Bram(BramKind::AuxClmpMaybe) => write!(f, "bram + auxclmp?"),
            ColumnKind::Bram(BramKind::BramClmp) => write!(f, "bram + bramclmp"),
            ColumnKind::Bram(BramKind::BramClmpMaybe) => write!(f, "bram + bramclmp?"),
            ColumnKind::Dsp(DspKind::Plain) => write!(f, "dsp"),
            ColumnKind::Dsp(DspKind::ClkBuf) => write!(f, "dsp + clkbuf"),
            ColumnKind::Uram => write!(f, "uram"),
            ColumnKind::Hard(HardKind::Clk, i) => write!(f, "hard[{i}, clk]"),
            ColumnKind::Hard(HardKind::NonClk, i) => write!(f, "hard[{i}, !clk]"),
            ColumnKind::Hard(HardKind::Term, i) => write!(f, "hard[{i}, term]"),
            ColumnKind::Io(i) => write!(f, "io[{i}]"),
            ColumnKind::Gt(i) => write!(f, "gt[{i}]"),
            ColumnKind::Sdfec => write!(f, "sdfec"),
            ColumnKind::DfeB => write!(f, "dfe_b"),
            ColumnKind::DfeC => write!(f, "dfe_c"),
            ColumnKind::DfeDF => write!(f, "dfe_df"),
            ColumnKind::DfeE => write!(f, "dfe_e"),
            ColumnKind::HdioS => write!(f, "hdios"),
            ColumnKind::ContUram => write!(f, "cont_uram"),
            ColumnKind::ContHard => write!(f, "cont_hard"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum CleMKind {
    Plain,
    ClkBuf,
    Laguna,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum BramKind {
    Plain,
    AuxClmp,
    BramClmp,
    AuxClmpMaybe,
    BramClmpMaybe,
    Td,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum CleLKind {
    Plain,
    Dcg10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum HardKind {
    Clk,
    NonClk,
    Term,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum DspKind {
    Plain,
    ClkBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Column {
    pub kind: ColumnKind,
    pub clk: [Option<u8>; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum HardRowKind {
    None,
    Cfg,
    Ams,
    Hdio,
    HdioAms,
    HdioL,
    Pcie,
    Pcie4C,
    Pcie4CE,
    Cmac,
    Ilkn,
    DfeA,
    DfeG,
}

impl std::fmt::Display for HardRowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardRowKind::None => write!(f, "none"),
            HardRowKind::Cfg => write!(f, "cfg"),
            HardRowKind::Ams => write!(f, "ams"),
            HardRowKind::Pcie => write!(f, "pcie"),
            HardRowKind::Pcie4C => write!(f, "pcie4c"),
            HardRowKind::Pcie4CE => write!(f, "pcie4ce"),
            HardRowKind::Cmac => write!(f, "cmac"),
            HardRowKind::Ilkn => write!(f, "ilkn"),
            HardRowKind::DfeA => write!(f, "dfe_a"),
            HardRowKind::DfeG => write!(f, "dfe_g"),
            HardRowKind::Hdio => write!(f, "hdio"),
            HardRowKind::HdioAms => write!(f, "hdio[ams]"),
            HardRowKind::HdioL => write!(f, "hdiol"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    HdioL,
    Xp5io,
    Gth,
    Gty,
    Gtm,
    Gtf,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

impl std::fmt::Display for IoRowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoRowKind::None => write!(f, "none"),
            IoRowKind::Hpio => write!(f, "hpio"),
            IoRowKind::Hrio => write!(f, "hrio"),
            IoRowKind::HdioL => write!(f, "hdiol"),
            IoRowKind::Xp5io => write!(f, "cp5io"),
            IoRowKind::Gth => write!(f, "gth"),
            IoRowKind::Gty => write!(f, "gty"),
            IoRowKind::Gtm => write!(f, "gtm"),
            IoRowKind::Gtf => write!(f, "gtf"),
            IoRowKind::HsAdc => write!(f, "hsadc"),
            IoRowKind::HsDac => write!(f, "hsdac"),
            IoRowKind::RfAdc => write!(f, "rfadc"),
            IoRowKind::RfDac => write!(f, "rfdac"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, IoRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Ps {
    pub col: ColId,
    pub has_vcu: bool,
    pub intf_kind: PsIntfKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum PsIntfKind {
    Alto,
    Da6,
    Da7,
    Da8,
    Dc12,
    Mx8,
}

impl std::fmt::Display for PsIntfKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsIntfKind::Alto => write!(f, "alto"),
            PsIntfKind::Da6 => write!(f, "da6"),
            PsIntfKind::Da7 => write!(f, "da7"),
            PsIntfKind::Da8 => write!(f, "da8"),
            PsIntfKind::Dc12 => write!(f, "dc12"),
            PsIntfKind::Mx8 => write!(f, "mx8"),
        }
    }
}

impl Ps {
    pub fn height(self) -> usize {
        if self.has_vcu { 240 } else { 180 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub enum DisabledPart {
    Region(DieId, RegId),
    TopRow(DieId, RegId),
    HardIp(DieId, ColId, RegId),
    Gt(DieId, ColId, RegId),
    GtBufs(DieId, ColId, RegId),
    GtmSpareBufs(DieId, ColId, RegId),
    HdioIob(DieId, ColId, RegId, TileIobId),
    HpioIob(DieId, ColId, RegId, TileIobId),
    HpioDci(DieId, ColId, RegId),
    Dfe,
    Sdfec,
    Ps,
    Vcu,
    HbmLeft,
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::Region(die, reg) => write!(f, "region {die} {reg}"),
            DisabledPart::TopRow(die, reg) => write!(f, "top_row {die} {reg}"),
            DisabledPart::HardIp(die, col, reg) => write!(f, "hard_ip {die} {col} {reg}"),
            DisabledPart::Gt(die, col, reg) => write!(f, "gt {die} {col} {reg}"),
            DisabledPart::GtBufs(die, col, reg) => write!(f, "gt_bufs {die} {col} {reg}"),
            DisabledPart::GtmSpareBufs(die, col, reg) => {
                write!(f, "gtm_spare_bufs {die} {col} {reg}")
            }
            DisabledPart::HdioIob(die, col, reg, iob) => write!(f, "hdio {die} {col} {reg} {iob}"),
            DisabledPart::HpioIob(die, col, reg, iob) => write!(f, "hpio {die} {col} {reg} {iob}"),
            DisabledPart::HpioDci(die, col, reg) => write!(f, "hpio_dci {die} {col} {reg}"),
            DisabledPart::Dfe => write!(f, "dfe"),
            DisabledPart::Sdfec => write!(f, "sdfec"),
            DisabledPart::Ps => write!(f, "ps"),
            DisabledPart::Vcu => write!(f, "vcu"),
            DisabledPart::HbmLeft => write!(f, "hbm_left"),
        }
    }
}

impl Chip {
    pub const ROWS_PER_REG: usize = 60;

    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / Chip::ROWS_PER_REG)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * Chip::ROWS_PER_REG)
    }

    pub fn row_reg_rclk(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * Chip::ROWS_PER_REG + Chip::ROWS_PER_REG / 2)
    }

    pub fn row_rclk(&self, row: RowId) -> RowId {
        self.row_reg_rclk(self.row_to_reg(row))
    }

    pub fn regs(&self) -> EntityRange<RegId> {
        EntityRange::new(0, self.regs)
    }

    pub fn rows(&self) -> EntityRange<RowId> {
        EntityRange::new(0, self.regs * Chip::ROWS_PER_REG)
    }

    pub fn is_laguna_row(&self, row: RowId) -> bool {
        let reg = self.row_to_reg(row);
        (reg.to_idx() == 0 && !self.has_hbm) || reg.to_idx() == self.regs - 1
    }

    pub fn col_cfg(&self) -> ColId {
        self.cols_hard
            .iter()
            .find(|hc| hc.regs.values().any(|&x| x == HardRowKind::Cfg))
            .unwrap()
            .col
    }

    pub fn row_ams(&self) -> RowId {
        for hc in &self.cols_hard {
            for (reg, &kind) in &hc.regs {
                if kind == HardRowKind::Ams {
                    return self.row_reg_rclk(reg);
                }
            }
        }
        unreachable!()
    }

    pub fn reg_cfg(&self) -> RegId {
        for hc in &self.cols_hard {
            for (reg, &kind) in &hc.regs {
                if kind == HardRowKind::Cfg {
                    return reg;
                }
            }
        }
        unreachable!()
    }

    pub fn is_dc12(&self) -> bool {
        if let Some(ps) = self.ps {
            matches!(ps.intf_kind, PsIntfKind::Dc12 | PsIntfKind::Mx8)
        } else {
            false
        }
    }

    pub fn is_nocfg(&self) -> bool {
        let reg_cfg = self.reg_cfg();
        !self
            .cols_io
            .iter()
            .any(|x| matches!(x.regs[reg_cfg], IoRowKind::Hpio | IoRowKind::Hrio))
    }

    pub fn col_side(&self, col: ColId) -> DirH {
        if col.to_idx().is_multiple_of(2) {
            DirH::W
        } else {
            DirH::E
        }
    }

    pub fn in_int_hole(&self, col: ColId, row: RowId) -> bool {
        if let Some(ps) = self.ps
            && row.to_idx() < ps.height()
            && col < ps.col
        {
            return true;
        }
        false
    }

    pub fn in_site_hole(&self, col: ColId, row: RowId) -> bool {
        if let Some(ps) = self.ps
            && row.to_idx() < ps.height()
            && col <= ps.col
        {
            return true;
        }
        if self.has_hbm && matches!(self.columns[col].kind, ColumnKind::Dsp(_)) && row.to_idx() < 15
        {
            return true;
        }
        false
    }
}

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tkind {};", self.kind)?;
        if let Some(ps) = self.ps {
            write!(o, "\tps {col}: {v}", v = ps.intf_kind, col = ps.col)?;
            if ps.has_vcu {
                write!(o, " + vcu")?;
            }
            writeln!(o, ";")?;
        }
        if self.has_hbm {
            writeln!(o, "\thbm;")?;
        }
        writeln!(o, "\tconfig {};", self.config_kind)?;
        if self.is_dmc {
            writeln!(o, "\tdmc;")?;
        }
        if self.is_alt_cfg {
            writeln!(o, "\tconfig_alt;")?;
        }
        writeln!(o, "\tcolumns {{")?;
        for (col, cd) in &self.columns {
            if self.cols_vbrk.contains(&col) {
                writeln!(o, "\t\t// break")?;
            }
            if self.cols_fsr_gap.contains(&col) {
                writeln!(o, "\t\t// FSR gap")?;
            }
            if matches!(cd.kind, ColumnKind::ContUram | ColumnKind::ContHard) {
                continue;
            }
            write!(o, "\t\t{}", cd.kind)?;
            if let ColumnKind::Io(idx) | ColumnKind::Gt(idx) = cd.kind {
                writeln!(o, " [")?;
                let ioc = &self.cols_io[idx];
                for (reg, kind) in &ioc.regs {
                    writeln!(o, "\t\t\t{kind}, // {y}", y = self.row_reg_bot(reg))?;
                }
                write!(o, "\t\t]")?;
            }
            if let ColumnKind::Hard(_, idx) = cd.kind {
                writeln!(o, " [")?;
                let hc = &self.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    writeln!(o, "\t\t\t{kind}, // {y}", y = self.row_reg_bot(reg))?;
                }
                write!(o, "\t\t]")?;
            }
            if cd.clk.iter().any(|x| x.is_some()) {
                write!(o, " + clk [")?;
                let mut first = true;
                for v in cd.clk {
                    if !first {
                        write!(o, ", ")?;
                    }
                    first = false;
                    if let Some(v) = v {
                        write!(o, "{v}")?;
                    } else {
                        write!(o, "-")?;
                    }
                }
                write!(o, "]")?;
            }
            if matches!(
                cd.kind,
                ColumnKind::Uram
                    | ColumnKind::Hard(_, _)
                    | ColumnKind::DfeC
                    | ColumnKind::DfeDF
                    | ColumnKind::DfeE
            ) {
                write!(o, ", // {col}-{cr}", cr = col + 1)?;
            } else {
                write!(o, ", // {col}")?;
            }
            if let Some(ps) = self.ps
                && ps.col == col
            {
                write!(o, " PS")?;
            }
            writeln!(o)?;
        }
        writeln!(o, "\t}}")?;
        if !self.cols_vbrk.is_empty() {
            writeln!(
                o,
                "\tcols_vbrk {};",
                self.cols_vbrk.iter().map(|x| x.to_string()).join(", ")
            )?;
        }
        if !self.cols_fsr_gap.is_empty() {
            writeln!(
                o,
                "\tcols_fsr_gap {};",
                self.cols_fsr_gap.iter().map(|x| x.to_string()).join(", ")
            )?;
        }

        writeln!(o, "\tregs {r};", r = self.regs)?;

        Ok(())
    }
}

impl Interposer {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tprimary {};", self.primary)?;
        Ok(())
    }
}
