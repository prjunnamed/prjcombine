use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_entity::{
    EntityId, EntityRange, EntityVec,
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
            ColumnKind::Io => write!(f, "io"),
            ColumnKind::ClbLL => write!(f, "clbll"),
            ColumnKind::ClbLM => write!(f, "clblm"),
            ColumnKind::Bram => write!(f, "bram"),
            ColumnKind::Dsp => write!(f, "dsp"),
            ColumnKind::Gt => write!(f, "gt"),
            ColumnKind::Cmt => write!(f, "cmt"),
            ColumnKind::Clk => write!(f, "clk"),
            ColumnKind::Cfg => write!(f, "cfg"),
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
            CfgRowKind::Dcm => write!(f, "dcm"),
            CfgRowKind::Ccm => write!(f, "ccm"),
            CfgRowKind::Sysmon => write!(f, "sysmon"),
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
            GtKind::Gtp => write!(f, "gtp"),
            GtKind::Gtx => write!(f, "gtx"),
            GtKind::Gth => write!(f, "gth"),
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
            IoKind::Hpio => write!(f, "hpio"),
            IoKind::Hrio => write!(f, "hrio"),
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
pub struct Pcie2 {
    pub side: DirH,
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
    W,
    E,
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

    pub fn regs(&self) -> EntityRange<RegId> {
        EntityRange::new(0, self.regs)
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

    pub fn rows(&self) -> EntityRange<RowId> {
        EntityRange::new(0, self.regs * self.rows_per_reg())
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
            XadcIoLoc::E
        } else if self.cols_io.len() == 1 || self.cols_io[1].regs[self.reg_cfg].is_none() {
            XadcIoLoc::W
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

impl Chip {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tkind {};", self.kind)?;
        if self.has_ps {
            writeln!(o, "\thas_ps;")?;
        }
        if self.has_slr {
            writeln!(o, "\thas_slr;")?;
        }
        if self.has_no_tbuturn {
            writeln!(o, "\tno_tb_uturn;")?;
        }
        writeln!(o, "\tcolumns {{")?;
        for (col, &cd) in &self.columns {
            if self.cols_vbrk.contains(&col) {
                writeln!(o, "\t\t// break")?;
            }
            write!(o, "\t\t{cd}")?;
            if self.cols_mgt_buf.contains(&col) {
                write!(o, " + mgt_buf")?;
            }
            if let Some((cl, cr)) = self.cols_qbuf
                && (col == cl || col == cr)
            {
                write!(o, " + qbuf")?;
            }
            if let Some(ref hard) = self.col_hard
                && hard.col == col
            {
                writeln!(o, " + hard {{")?;
                for &row in &hard.rows_pcie {
                    writeln!(o, "\t\t\tpcie {row};")?;
                }
                for &row in &hard.rows_emac {
                    writeln!(o, "\t\t\temac {row};")?;
                }
                write!(o, "\t\t}}")?;
            }
            for ioc in &self.cols_io {
                if ioc.col == col {
                    writeln!(o, " {{")?;
                    for (reg, kind) in &ioc.regs {
                        if let Some(kind) = kind {
                            writeln!(o, "\t\t\tbank {row} {kind};", row = self.row_reg_bot(reg))?;
                        }
                    }
                    write!(o, "\t\t}}")?;
                }
            }
            for gtc in &self.cols_gt {
                if gtc.col == col {
                    if cd == ColumnKind::Gt {
                        writeln!(o, " {{")?;
                    } else {
                        writeln!(o, " + gt {{")?;
                    }
                    let mid = if gtc.is_middle { "mid " } else { "" };
                    for (reg, kind) in &gtc.regs {
                        if let Some(kind) = kind {
                            writeln!(
                                o,
                                "\t\t\tgt {row} {mid}{kind};",
                                row = self.row_reg_bot(reg)
                            )?;
                        }
                    }
                    write!(o, "\t\t}}")?;
                }
            }
            if cd == ColumnKind::Cfg && !self.rows_cfg.is_empty() {
                writeln!(o, " {{")?;
                for &(row, kind) in &self.rows_cfg {
                    writeln!(o, "\t\t\t{kind} {row};")?;
                }
                write!(o, "\t\t}}")?;
            }
            writeln!(o, ", // {col}")?;
        }
        writeln!(o, "\t}}")?;
        if !self.cols_vbrk.is_empty() {
            writeln!(
                o,
                "\tcols_vbrk {};",
                self.cols_vbrk.iter().map(|x| x.to_string()).join(", ")
            )?;
        }

        writeln!(o, "\tregs {r};", r = self.regs)?;
        writeln!(o, "\treg_cfg {v}", v = self.reg_cfg)?;
        writeln!(o, "\treg_clk {v}", v = self.reg_clk)?;
        for &(col, row) in &self.holes_ppc {
            let (col_r, row_t): (ColId, RowId) = match self.kind {
                ChipKind::Virtex4 => (col + 9, row + 24),
                ChipKind::Virtex5 => (col + 14, row + 40),
                _ => unreachable!(),
            };
            writeln!(o, "\tppc {col}:{col_r} {row}:{row_t};")?;
        }
        for pcie in &self.holes_pcie2 {
            writeln!(
                o,
                "\tpcie2 {side} {col_l}:{col_r} {row_b}:{row_t};",
                side = pcie.side,
                col_l = pcie.col,
                col_r = pcie.col + 4,
                row_b = pcie.row,
                row_t = pcie.row + 25
            )?;
        }
        for &(col, row) in &self.holes_pcie3 {
            writeln!(
                o,
                "\tpcie3 {col_l}:{col_r} {row_b}:{row_t};",
                col_l = col,
                col_r = col + 6,
                row_b = row,
                row_t = row + 50
            )?;
        }
        if self.has_bram_fx {
            writeln!(o, "\thas_bram_fx;")?;
        }
        Ok(())
    }
}

impl Interposer {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(o, "\tprimary {};", self.primary)?;
        if self.gtz_bot {
            writeln!(o, "\tgtz_bot;")?;
        }
        if self.gtz_top {
            writeln!(o, "\tgtz_top;")?;
        }
        Ok(())
    }
}
