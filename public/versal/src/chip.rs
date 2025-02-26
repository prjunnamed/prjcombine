use prjcombine_interconnect::{
    dir::Dir,
    grid::{ColId, DieId, RowId},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityIds, EntityVec, entity_id};

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Chip {
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_cpipe: BTreeSet<ColId>,
    pub cols_hard: Vec<HardColumn>,
    pub regs: usize,
    pub regs_gt_left: EntityVec<RegId, GtRowKind>,
    pub ps: PsKind,
    pub cpm: CpmKind,
    pub has_xram_top: bool,
    pub is_vr: bool,
    pub top: TopKind,
    pub bottom: BotKind,
    pub right: RightKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum InterposerKind {
    Single,
    Column,
    MirrorSquare,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Interposer {
    pub kind: InterposerKind,
    pub sll_columns: EntityVec<DieId, Vec<ColId>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub has_bli_s: bool,
    pub has_bli_n: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CleKind {
    Plain,
    Sll,
    Sll2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BramKind {
    Plain,
    ClkBuf,
    ClkBufNoPd,
    MaybeClkBufNoPd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnKind {
    Cle(CleKind),
    Bram(BramKind),
    Uram,
    Dsp,
    Hard,
    Gt,
    Cfrm,
    VNoc,
    VNoc2,
    VNoc4,
    ContDsp,
    ContVNoc,
    ContHard,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PsKind {
    Ps9,
    PsX,
    PsXc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CpmKind {
    None,
    Cpm4,
    Cpm5,
    Cpm5N,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum HardRowKind {
    None,
    Hdio,
    Pcie4,
    Pcie5,
    Mrmac,
    SdfecA,
    DfeCfcB,
    DfeCfcT,
    IlknB,
    IlknT,
    DcmacB,
    DcmacT,
    HscB,
    HscT,
    CpmExt,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GtRowKind {
    None,
    Gty,
    Gtyp,
    Gtm,
    RfAdc,
    RfDac,
    Xram,
    Vdu,
    BfrB,
    Isp2,
    Vcu2B,
    Vcu2T,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BotKind {
    Xpio(usize),
    Ssit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TopKind {
    Xpio(usize),
    Ssit,
    Me,
    Ai(usize, usize),
    AiMl(usize, usize, usize),
    Hbm,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RightKind {
    Term,
    Term2,
    Gt(EntityVec<RegId, GtRowKind>),
    HNicX,
    Cidb,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum NocEndpoint {
    // tile idx, switch idx, port idx
    BotNps(usize, usize, usize),
    TopNps(usize, usize, usize),
    Ncrb(usize, usize, usize),
    // column, region, switch idx, port idx
    VNocNps(ColId, usize, usize, usize),
    VNocEnd(ColId, usize, usize),
    Pmc(usize),
    Me(usize, usize),
    // tile idx, port idx
    BotDmc(usize, usize),
    TopDmc(usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    HardIp(DieId, ColId, RegId),
    HardIpSite(DieId, ColId, RegId),
    HdioDpll(DieId, ColId, RegId),
    Column(DieId, ColId),
    GtRight(DieId, RegId),
    Region(DieId, RegId),
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::HardIp(die, col, reg) => write!(f, "HARD_IP:{die}:{col}:{reg}"),
            DisabledPart::HardIpSite(die, col, reg) => write!(f, "HARD_IP_SITE:{die}:{col}:{reg}"),
            DisabledPart::HdioDpll(die, col, reg) => write!(f, "HDIO_DPLL:{die}:{col}:{reg}"),
            DisabledPart::Column(die, col) => write!(f, "COLUMN:{die}:{col}"),
            DisabledPart::GtRight(die, reg) => write!(f, "GT_RIGHT:{die}:{reg}"),
            DisabledPart::Region(die, reg) => write!(f, "REGION:{die}:{reg}"),
        }
    }
}

impl Chip {
    pub const ROWS_PER_REG: usize = 48;

    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / Self::ROWS_PER_REG)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * Self::ROWS_PER_REG)
    }

    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        let reg = if self.is_reg_top(reg) { reg } else { reg + 1 };
        self.row_reg_bot(reg)
    }

    pub fn is_reg_top(&self, reg: RegId) -> bool {
        reg.to_idx() == self.regs - 1 || reg.to_idx() % 2 == 1
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.regs * Self::ROWS_PER_REG)
    }

    pub fn get_col_hard(&self, col: ColId) -> Option<&HardColumn> {
        self.cols_hard.iter().find(|x| x.col == col)
    }

    pub fn get_ps_height(&self) -> usize {
        match (self.ps, self.cpm) {
            (PsKind::Ps9, CpmKind::None) => Self::ROWS_PER_REG * 2,
            (PsKind::Ps9, CpmKind::Cpm4) => Self::ROWS_PER_REG * 3,
            (PsKind::Ps9, CpmKind::Cpm5) => Self::ROWS_PER_REG * 6,
            (PsKind::PsX, CpmKind::Cpm5N) => Self::ROWS_PER_REG * 9,
            (PsKind::PsXc, CpmKind::None) => Self::ROWS_PER_REG * 6,
            _ => unreachable!(),
        }
    }

    pub fn col_side(&self, col: ColId) -> Dir {
        if col.to_idx() % 2 == 0 {
            Dir::W
        } else {
            Dir::E
        }
    }

    pub fn col_cfrm(&self) -> ColId {
        self.columns
            .iter()
            .find(|(_, cd)| cd.kind == ColumnKind::Cfrm)
            .unwrap()
            .0
    }

    pub fn in_int_hole(&self, col: ColId, row: RowId) -> bool {
        row.to_idx() < self.get_ps_height() && col < self.col_cfrm()
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: Versal")?;
        writeln!(f, "\tPS: {v:?}", v = self.ps)?;
        writeln!(f, "\tCPM: {v:?}", v = self.cpm)?;
        writeln!(f, "\tXRAM TOP: {v:?}", v = self.has_xram_top)?;
        writeln!(f, "\tIS VR: {v:?}", v = self.is_vr)?;
        writeln!(f, "\tTOP: {v:?}", v = self.top)?;
        writeln!(f, "\tBOTTOM: {v:?}", v = self.bottom)?;
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            if self.cols_vbrk.contains(&col) {
                writeln!(f, "\t\t--- break")?;
            }
            if self.cols_cpipe.contains(&col) {
                writeln!(f, "\t\t--- CPIPE")?;
            }
            if matches!(
                cd.kind,
                ColumnKind::ContDsp | ColumnKind::ContVNoc | ColumnKind::ContHard
            ) {
                continue;
            }
            if matches!(
                cd.kind,
                ColumnKind::Dsp
                    | ColumnKind::Hard
                    | ColumnKind::VNoc
                    | ColumnKind::VNoc2
                    | ColumnKind::VNoc4
            ) {
                write!(f, "\t\tX{col}-X{col1}: ", col1 = col + 1)?;
            } else {
                write!(f, "\t\tX{col}: ")?;
            }
            match cd.kind {
                ColumnKind::None => write!(f, "---")?,
                ColumnKind::Cle(CleKind::Plain) => write!(f, "CLE")?,
                ColumnKind::Cle(CleKind::Sll) => write!(f, "CLE.SLL")?,
                ColumnKind::Cle(CleKind::Sll2) => write!(f, "CLE.SLL2")?,
                ColumnKind::Dsp => write!(f, "DSP")?,
                ColumnKind::Bram(BramKind::Plain) => write!(f, "BRAM")?,
                ColumnKind::Bram(BramKind::ClkBuf) => write!(f, "BRAM.CLKBUF")?,
                ColumnKind::Bram(BramKind::ClkBufNoPd) => write!(f, "BRAM.CLKBUF.NOPD")?,
                ColumnKind::Bram(BramKind::MaybeClkBufNoPd) => write!(f, "BRAM.MAYBE.CLKBUF.NOPD")?,
                ColumnKind::Uram => write!(f, "URAM")?,
                ColumnKind::Hard => write!(f, "HARD")?,
                ColumnKind::Gt => write!(f, "GT")?,
                ColumnKind::Cfrm => write!(f, "CFRM")?,
                ColumnKind::VNoc => write!(f, "VNOC")?,
                ColumnKind::VNoc2 => write!(f, "VNOC2")?,
                ColumnKind::VNoc4 => write!(f, "VNOC4")?,
                ColumnKind::ContDsp | ColumnKind::ContVNoc | ColumnKind::ContHard => unreachable!(),
            }
            if cd.has_bli_s {
                write!(f, " BLI.S")?;
            }
            if cd.has_bli_n {
                write!(f, " BLI.N")?;
            }
            writeln!(f)?;
            for hc in &self.cols_hard {
                if hc.col == col {
                    for (reg, kind) in &hc.regs {
                        writeln!(f, "\t\t\tY{y}: {kind:?}", y = self.row_reg_bot(reg))?;
                    }
                }
            }
        }
        writeln!(f, "\tGT LEFT:")?;
        for (reg, kind) in &self.regs_gt_left {
            writeln!(f, "\t\tY{y}: {kind:?}", y = self.row_reg_bot(reg))?;
        }
        match self.right {
            RightKind::Term => {
                writeln!(f, "\tRIGHT: TERM")?;
            }
            RightKind::Term2 => {
                writeln!(f, "\tRIGHT: TERM2")?;
            }
            RightKind::Gt(ref regs_gt_right) => {
                writeln!(f, "\tRIGHT: GT:\n")?;
                for (reg, kind) in regs_gt_right {
                    writeln!(f, "\t\tY{y}: {kind:?}", y = self.row_reg_bot(reg))?;
                }
            }
            RightKind::HNicX => {
                writeln!(f, "\tRIGHT: HNIC")?;
            }
            RightKind::Cidb => {
                writeln!(f, "\tRIGHT: CIDB")?;
            }
        }
        writeln!(f, "\tREGS: {r}", r = self.regs)?;
        Ok(())
    }
}

impl std::fmt::Display for Interposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {:?}", self.kind)?;
        for (die, die_sll_columns) in &self.sll_columns {
            if !die_sll_columns.is_empty() {
                write!(f, "\tSLL COLUMNS D{die}:")?;
                for &col in die_sll_columns {
                    write!(f, " X{col}")?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
