use bincode::{Decode, Encode};
use jzon::JsonValue;
use prjcombine_entity::{
    EntityId, EntityIds, EntityVec,
    id::{EntityIdU8, EntityTag, EntityTagArith},
};
use prjcombine_interconnect::{
    dir::DirH,
    grid::{ColId, DieId, RowId},
};
use std::collections::BTreeSet;

pub struct RegTag;
impl EntityTag for RegTag {
    const PREFIX: &'static str = "REG";
}
impl EntityTagArith for RegTag {}
pub type RegId = EntityIdU8<RegTag>;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Chip {
    pub kind: ChipKind,
    pub columns: EntityVec<ColId, ColumnKind>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_mgt_buf: BTreeSet<ColId>,
    pub cols_qbuf: Option<(ColId, ColId)>,
    pub col_hard: Option<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub cols_gt: Vec<GtColumn>,
    pub regs: usize,
    pub reg_cfg: RegId,
    pub reg_clk: RegId,
    pub rows_cfg: Vec<(RowId, CfgRowKind)>,
    pub holes_ppc: Vec<(ColId, RowId)>,
    pub holes_pcie2: Vec<Pcie2>,
    pub holes_pcie3: Vec<(ColId, RowId)>,
    pub has_bram_fx: bool,
    pub has_ps: bool,
    pub has_slr: bool,
    pub has_no_tbuturn: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ChipKind {
    Virtex4,
    Virtex5,
    Virtex6,
    Virtex7,
}

impl std::fmt::Display for ChipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChipKind::Virtex4 => write!(f, "virtex4"),
            ChipKind::Virtex5 => write!(f, "virtex5"),
            ChipKind::Virtex6 => write!(f, "virtex6"),
            ChipKind::Virtex7 => write!(f, "virtex7"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum ColumnKind {
    ClbLL,
    ClbLM,
    Bram,
    Dsp,
    Io,
    Cfg,
    Gt,
    Cmt,
    Clk,
}

impl std::fmt::Display for ColumnKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnKind::Io => write!(f, "IO"),
            ColumnKind::ClbLL => write!(f, "CLBLL"),
            ColumnKind::ClbLM => write!(f, "CLBLM"),
            ColumnKind::Bram => write!(f, "BRAM"),
            ColumnKind::Dsp => write!(f, "DSP"),
            ColumnKind::Gt => write!(f, "GT"),
            ColumnKind::Cmt => write!(f, "CMT"),
            ColumnKind::Clk => write!(f, "CLK"),
            ColumnKind::Cfg => write!(f, "CFG"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum CfgRowKind {
    Dcm,
    Ccm,
    Sysmon,
}

impl std::fmt::Display for CfgRowKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfgRowKind::Dcm => write!(f, "DCM"),
            CfgRowKind::Ccm => write!(f, "CCM"),
            CfgRowKind::Sysmon => write!(f, "SYSMON"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum GtKind {
    Gtp,
    Gtx,
    Gth,
}

impl std::fmt::Display for GtKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GtKind::Gtp => write!(f, "GTP"),
            GtKind::Gtx => write!(f, "GTX"),
            GtKind::Gth => write!(f, "GTH"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum IoKind {
    Hpio,
    Hrio,
}

impl std::fmt::Display for IoKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoKind::Hpio => write!(f, "HPIO"),
            IoKind::Hrio => write!(f, "HRIO"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct GtColumn {
    pub col: ColId,
    pub is_middle: bool,
    pub regs: EntityVec<RegId, Option<GtKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub enum Pcie2Kind {
    Left,
    Right,
}

impl std::fmt::Display for Pcie2Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pcie2Kind::Left => write!(f, "LEFT"),
            Pcie2Kind::Right => write!(f, "RIGHT"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Pcie2 {
    pub kind: Pcie2Kind,
    pub col: ColId,
    pub row: RowId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode)]
pub enum DisabledPart {
    Emac(RowId),
    GtxRow(RegId),
    SysMon,
    Gtp,
}

impl std::fmt::Display for DisabledPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisabledPart::Emac(row) => write!(f, "EMAC:{row}"),
            DisabledPart::GtxRow(reg) => write!(f, "GTX:{reg}"),
            DisabledPart::SysMon => write!(f, "SYSMON"),
            DisabledPart::Gtp => write!(f, "GTP"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Encode, Decode)]
pub struct Interposer {
    pub primary: DieId,
    pub gtz_bot: bool,
    pub gtz_top: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub enum XadcIoLoc {
    Left,
    Right,
    Both,
}

impl Chip {
    #[inline]
    pub fn rows_per_reg(&self) -> usize {
        match self.kind {
            ChipKind::Virtex4 => 16,
            ChipKind::Virtex5 => 20,
            ChipKind::Virtex6 => 40,
            ChipKind::Virtex7 => 50,
        }
    }

    #[inline]
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / self.rows_per_reg())
    }

    #[inline]
    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * self.rows_per_reg())
    }

    #[inline]
    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        self.row_reg_bot(reg) + self.rows_per_reg() / 2
    }

    #[inline]
    pub fn row_hclk(&self, row: RowId) -> RowId {
        self.row_reg_hclk(self.row_to_reg(row))
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn row_bufg(&self) -> RowId {
        self.row_reg_bot(self.reg_clk)
    }

    pub fn get_col_io(&self, col: ColId) -> Option<&IoColumn> {
        self.cols_io.iter().find(|ioc| ioc.col == col)
    }

    pub fn get_col_gt(&self, col: ColId) -> Option<&GtColumn> {
        self.cols_gt.iter().find(|gtc| gtc.col == col)
    }

    pub fn col_ps(&self) -> ColId {
        assert!(self.has_ps);
        ColId::from_idx(18)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.regs * self.rows_per_reg())
    }

    pub fn col_side(&self, col: ColId) -> DirH {
        assert_eq!(self.kind, ChipKind::Virtex7);
        if col.to_idx().is_multiple_of(2) {
            DirH::W
        } else {
            DirH::E
        }
    }

    pub fn get_xadc_io_loc(&self) -> XadcIoLoc {
        assert_eq!(self.kind, ChipKind::Virtex7);
        assert!(self.regs > 1);
        if self.has_ps {
            XadcIoLoc::Right
        } else if self.cols_io.len() == 1 || self.cols_io[1].regs[self.reg_cfg].is_none() {
            XadcIoLoc::Left
        } else {
            XadcIoLoc::Both
        }
    }

    pub fn get_cmt_rows(&self) -> Vec<RowId> {
        assert_eq!(self.kind, ChipKind::Virtex5);
        let mut res = vec![];
        if self.reg_cfg.to_idx() > 2 {
            res.push(self.row_reg_bot(self.reg_cfg - 3));
            res.push(self.row_reg_hclk(self.reg_cfg - 3));
        }
        res.push(self.row_reg_bot(self.reg_cfg - 2));
        if self.regs - self.reg_cfg.to_idx() > 1 {
            res.push(self.row_reg_hclk(self.reg_cfg + 1));
        }
        if self.regs - self.reg_cfg.to_idx() > 2 {
            res.push(self.row_reg_bot(self.reg_cfg + 2));
            res.push(self.row_reg_hclk(self.reg_cfg + 2));
        }
        res
    }
}

impl From<&HardColumn> for JsonValue {
    fn from(hc: &HardColumn) -> Self {
        jzon::object! {
            col: hc.col.to_idx(),
            rows_emac: Vec::from_iter(hc.rows_emac.iter().map(|row| row.to_idx())),
            rows_pcie: Vec::from_iter(hc.rows_pcie.iter().map(|row| row.to_idx())),
        }
    }
}

impl From<&IoColumn> for JsonValue {
    fn from(ioc: &IoColumn) -> Self {
        jzon::object! {
            col: ioc.col.to_idx(),
            regs: Vec::from_iter(ioc.regs.values().map(|kind| match kind {
                None => JsonValue::Null,
                Some(kind) => kind.to_string().into(),
            }))
        }
    }
}

impl From<&GtColumn> for JsonValue {
    fn from(gtc: &GtColumn) -> Self {
        jzon::object! {
            col: gtc.col.to_idx(),
            is_middle: gtc.is_middle,
            regs: Vec::from_iter(gtc.regs.values().map(|kind| match kind {
                None => JsonValue::Null,
                Some(kind) => kind.to_string().into(),
            }))
        }
    }
}

impl From<&Chip> for JsonValue {
    fn from(chip: &Chip) -> Self {
        jzon::object! {
            kind: chip.kind.to_string(),
            columns: Vec::from_iter(chip.columns.values().map(|kind| kind.to_string())),
            cols_vbrk: Vec::from_iter(chip.cols_vbrk.iter().map(|col| col.to_idx())),
            cols_mgt_buf: Vec::from_iter(chip.cols_mgt_buf.iter().map(|col| col.to_idx())),
            cols_qbuf: chip.cols_qbuf.map(|(col_l, col_r)| jzon::array![col_l.to_idx(), col_r.to_idx()]),
            col_hard: chip.col_hard.as_ref(),
            cols_io: Vec::from_iter(chip.cols_io.iter()),
            cols_gt: Vec::from_iter(chip.cols_gt.iter()),
            regs: chip.regs,
            reg_cfg: chip.reg_cfg.to_idx(),
            reg_clk: chip.reg_clk.to_idx(),
            rows_cfg: jzon::object::Object::from_iter(chip.rows_cfg.iter().map(|(row, kind)|
                (row.to_string(), kind.to_string())
            )),
            holes_ppc: Vec::from_iter(chip.holes_ppc.iter().map(|(col, row)| jzon::array![col.to_idx(), row.to_idx()])),
            holes_pcie2: Vec::from_iter(chip.holes_pcie2.iter().map(|hole| jzon::object! {
                kind: hole.kind.to_string(),
                col: hole.col.to_idx(),
                row: hole.row.to_idx(),
            })),
            holes_pcie3: Vec::from_iter(chip.holes_pcie3.iter().map(|(col, row)| jzon::array![col.to_idx(), row.to_idx()])),
            has_ps: chip.has_ps,
            has_slr: chip.has_slr,
            has_no_tbuturn: chip.has_no_tbuturn,
        }
    }
}

impl From<&Interposer> for JsonValue {
    fn from(interp: &Interposer) -> Self {
        jzon::object! {
            primary: interp.primary.to_idx(),
            gtz_bot: interp.gtz_bot,
            gtz_top: interp.gtz_top,
        }
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {k}", k = self.kind)?;
        if self.has_ps {
            writeln!(f, "\tHAS PS")?;
        }
        if self.has_slr {
            writeln!(f, "\tHAS SLR")?;
        }
        if self.has_no_tbuturn {
            writeln!(f, "\tHAS NO TB UTURN")?;
        }
        writeln!(f, "\tCOLS:")?;
        for (col, &cd) in &self.columns {
            if self.cols_vbrk.contains(&col) {
                writeln!(f, "\t\t--- break")?;
            }
            write!(f, "\t\t{col}: {cd}")?;
            if self.cols_mgt_buf.contains(&col) {
                write!(f, " MGT_BUF")?;
            }
            if let Some((cl, cr)) = self.cols_qbuf
                && (col == cl || col == cr)
            {
                write!(f, " QBUF")?;
            }
            writeln!(f)?;
            if let Some(ref hard) = self.col_hard
                && hard.col == col
            {
                for &row in &hard.rows_pcie {
                    writeln!(f, "\t\t\t{row}: PCIE")?;
                }
                for &row in &hard.rows_emac {
                    writeln!(f, "\t\t\t{row}: EMAC")?;
                }
            }
            for ioc in &self.cols_io {
                if ioc.col == col {
                    for (reg, kind) in &ioc.regs {
                        if let Some(kind) = kind {
                            writeln!(f, "\t\t\t{row}: {kind}", row = self.row_reg_bot(reg))?;
                        }
                    }
                }
            }
            for gtc in &self.cols_gt {
                if gtc.col == col {
                    let mid = if gtc.is_middle { "MID " } else { "" };
                    for (reg, kind) in &gtc.regs {
                        if let Some(kind) = kind {
                            writeln!(f, "\t\t\t{row}: {mid}{kind}", row = self.row_reg_bot(reg))?;
                        }
                    }
                }
            }
            if cd == ColumnKind::Cfg {
                for &(row, kind) in &self.rows_cfg {
                    writeln!(f, "\t\t\t{row}: {kind}")?;
                }
            }
        }
        writeln!(f, "\tREGS: {r}", r = self.regs)?;
        writeln!(f, "\tCFG REG: {v:?}", v = self.reg_cfg.to_idx())?;
        writeln!(f, "\tCLK REG: {v:?}", v = self.reg_clk.to_idx())?;
        for &(col, row) in &self.holes_ppc {
            let (col_r, row_t): (ColId, RowId) = match self.kind {
                ChipKind::Virtex4 => (col + 9, row + 24),
                ChipKind::Virtex5 => (col + 14, row + 40),
                _ => unreachable!(),
            };
            writeln!(f, "\tPPC: {col}:{col_r} {row}:{row_t}")?;
        }
        for pcie in &self.holes_pcie2 {
            writeln!(
                f,
                "\tPCIE2.{lr}: {col_l}:{col_r} {row_b}:{row_t}",
                lr = match pcie.kind {
                    Pcie2Kind::Left => 'L',
                    Pcie2Kind::Right => 'R',
                },
                col_l = pcie.col,
                col_r = pcie.col + 4,
                row_b = pcie.row,
                row_t = pcie.row + 25
            )?;
        }
        for &(col, row) in &self.holes_pcie3 {
            writeln!(
                f,
                "\tPCIE3: {col_l}:{col_r} {row_b}:{row_t}",
                col_l = col,
                col_r = col + 6,
                row_b = row,
                row_t = row + 50
            )?;
        }
        writeln!(f, "\tHAS BRAM_FX: {v:?}", v = self.has_bram_fx)?;
        Ok(())
    }
}

impl std::fmt::Display for Interposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tPRIMARY: D{}", self.primary)?;
        writeln!(f, "\tGTZ BOT: {}", self.gtz_bot)?;
        writeln!(f, "\tGTZ TOP: {}", self.gtz_top)?;
        Ok(())
    }
}
