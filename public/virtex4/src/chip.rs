use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityIds, EntityVec, entity_id};

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ChipKind {
    Virtex4,
    Virtex5,
    Virtex6,
    Virtex7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CfgRowKind {
    Dcm,
    Ccm,
    Sysmon,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GtKind {
    Gtp,
    Gtx,
    Gth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GtColumn {
    pub col: ColId,
    pub is_middle: bool,
    pub regs: EntityVec<RegId, Option<GtKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Pcie2Kind {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Pcie2 {
    pub kind: Pcie2Kind,
    pub col: ColId,
    pub row: RowId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interposer {
    pub primary: DieId,
    pub gtz_bot: bool,
    pub gtz_top: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "kind": match self.kind {
                ChipKind::Virtex4 => "virtex4",
                ChipKind::Virtex5 => "virtex5",
                ChipKind::Virtex6 => "virtex6",
                ChipKind::Virtex7 => "virtex7",
            },
            "columns": Vec::from_iter(self.columns.values().map(|kind| match kind {
                ColumnKind::ClbLL => "CLBLL".to_string(),
                ColumnKind::ClbLM => "CLBLM".to_string(),
                ColumnKind::Bram => "BRAM".to_string(),
                ColumnKind::Dsp => "DSP".to_string(),
                ColumnKind::Io => "IO".to_string(),
                ColumnKind::Cfg => "CFG".to_string(),
                ColumnKind::Gt => "GT".to_string(),
                ColumnKind::Cmt => "CMT".to_string(),
                ColumnKind::Clk => "CLK".to_string(),
            })),
            "cols_vbrk": self.cols_vbrk,
            "cols_mgt_buf": self.cols_mgt_buf,
            "cols_qbuf": self.cols_qbuf,
            "col_hard": self.col_hard,
            "cols_io": Vec::from_iter(self.cols_io.iter().map(|iocol| json!({
                "col": iocol.col,
                "regs": Vec::from_iter(iocol.regs.values().map(|kind| match kind {
                    None => serde_json::Value::Null,
                    Some(IoKind::Hpio) => "HPIO".into(),
                    Some(IoKind::Hrio) => "HRIO".into(),
                })),
            }))),
            "cols_gt": Vec::from_iter(self.cols_gt.iter().map(|gtcol| json!({
                "col": gtcol.col,
                "is_middle": gtcol.is_middle,
                "regs": Vec::from_iter(gtcol.regs.values().map(|kind| match kind {
                    None => serde_json::Value::Null,
                    Some(GtKind::Gtp) => "GTP".into(),
                    Some(GtKind::Gtx) => "GTX".into(),
                    Some(GtKind::Gth) => "GTH".into(),
                })),
            }))),
            "regs": self.regs,
            "reg_cfg": self.reg_cfg,
            "reg_clk": self.reg_clk,
            "rows_cfg": serde_json::Map::from_iter(self.rows_cfg.iter().map(|(row, kind)|
                (row.to_string(), match kind {
                    CfgRowKind::Dcm => "DCM",
                    CfgRowKind::Ccm => "CCM",
                    CfgRowKind::Sysmon => "SYSMON",
                }.into())
            )),
            "holes_ppc": self.holes_ppc,
            "holes_pcie2": Vec::from_iter(self.holes_pcie2.iter().map(|hole| json!({
                "kind": match hole.kind {
                    Pcie2Kind::Left => "LEFT",
                    Pcie2Kind::Right => "RIGHT",
                },
                "col": hole.col,
                "row": hole.row,
            }))),
            "holes_pcie3": self.holes_pcie3,
            "has_ps": self.has_ps,
            "has_slr": self.has_slr,
            "has_no_tbuturn": self.has_no_tbuturn,
        })
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {v:?}", v = self.kind)?;
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
            write!(f, "\t\tX{c}: ", c = col.to_idx())?;
            match cd {
                ColumnKind::Io => write!(f, "IO")?,
                ColumnKind::ClbLL => write!(f, "CLBLL")?,
                ColumnKind::ClbLM => write!(f, "CLBLM")?,
                ColumnKind::Bram => write!(f, "BRAM")?,
                ColumnKind::Dsp => write!(f, "DSP")?,
                ColumnKind::Gt => write!(f, "GT")?,
                ColumnKind::Cmt => write!(f, "CMT")?,
                ColumnKind::Clk => write!(f, "CLK")?,
                ColumnKind::Cfg => write!(f, "CFG")?,
            }
            if self.cols_mgt_buf.contains(&col) {
                write!(f, " MGT_BUF")?;
            }
            if let Some((cl, cr)) = self.cols_qbuf {
                if col == cl || col == cr {
                    write!(f, " QBUF")?;
                }
            }
            writeln!(f)?;
            if let Some(ref hard) = self.col_hard {
                if hard.col == col {
                    for &row in &hard.rows_pcie {
                        writeln!(f, "\t\t\tY{y}: PCIE", y = row.to_idx())?;
                    }
                    for &row in &hard.rows_emac {
                        writeln!(f, "\t\t\tY{y}: EMAC", y = row.to_idx())?;
                    }
                }
            }
            for ioc in &self.cols_io {
                if ioc.col == col {
                    for (reg, kind) in &ioc.regs {
                        if let Some(kind) = kind {
                            writeln!(
                                f,
                                "\t\t\tY{y}: {kind:?}",
                                y = self.row_reg_bot(reg).to_idx()
                            )?;
                        }
                    }
                }
            }
            for gtc in &self.cols_gt {
                if gtc.col == col {
                    let mid = if gtc.is_middle { "MID " } else { "" };
                    for (reg, kind) in &gtc.regs {
                        if let Some(kind) = kind {
                            writeln!(
                                f,
                                "\t\t\tY{y}: {mid}{kind:?}",
                                y = self.row_reg_bot(reg).to_idx()
                            )?;
                        }
                    }
                }
            }
            if cd == ColumnKind::Cfg {
                for &(row, kind) in &self.rows_cfg {
                    writeln!(f, "\t\t\tY{y}: {kind:?}", y = row.to_idx())?;
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
            writeln!(
                f,
                "\tPPC: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col_r.to_idx(),
                yb = row.to_idx(),
                yt = row_t.to_idx(),
            )?;
        }
        for pcie in &self.holes_pcie2 {
            writeln!(
                f,
                "\tPCIE2.{lr}: X{xl}:X{xr} Y{yb}:Y{yt}",
                lr = match pcie.kind {
                    Pcie2Kind::Left => 'L',
                    Pcie2Kind::Right => 'R',
                },
                xl = pcie.col.to_idx(),
                xr = pcie.col.to_idx() + 4,
                yb = pcie.row.to_idx(),
                yt = pcie.row.to_idx() + 25
            )?;
        }
        for &(col, row) in &self.holes_pcie3 {
            writeln!(
                f,
                "\tPCIE3: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col.to_idx() + 6,
                yb = row.to_idx(),
                yt = row.to_idx() + 50
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
