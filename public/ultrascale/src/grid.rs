use enum_map::Enum;
use prjcombine_interconnect::grid::{ColId, DieId, RowId, TileIobId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use unnamed_entity::{EntityId, EntityIds, EntityVec, entity_id};

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GridKind {
    Ultrascale,
    UltrascalePlus,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize, Enum,
)]
pub enum ColSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Interposer {
    pub primary: DieId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
    pub columns: EntityVec<ColId, Column>,
    pub cols_vbrk: BTreeSet<ColId>,
    pub cols_fsr_gap: BTreeSet<ColId>,
    pub cols_hard: Vec<HardColumn>,
    pub cols_io: Vec<IoColumn>,
    pub regs: usize,
    pub ps: Option<Ps>,
    pub has_hbm: bool,
    pub has_csec: bool,
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnKindLeft {
    CleL,
    CleM(CleMKind),
    Bram(BramKind),
    Uram,
    Hard(HardKind, usize),
    Io(usize),
    Gt(usize),
    Sdfec,
    DfeC,
    DfeDF,
    DfeE,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CleMKind {
    Plain,
    ClkBuf,
    Laguna,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum BramKind {
    Plain,
    AuxClmp,
    BramClmp,
    AuxClmpMaybe,
    BramClmpMaybe,
    Td,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnKindRight {
    CleL(CleLKind),
    Dsp(DspKind),
    Uram,
    Hard(HardKind, usize),
    Io(usize),
    Gt(usize),
    DfeB,
    DfeC,
    DfeDF,
    DfeE,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CleLKind {
    Plain,
    Dcg10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum HardKind {
    Clk,
    NonClk,
    Term,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DspKind {
    Plain,
    ClkBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Column {
    pub l: ColumnKindLeft,
    pub r: ColumnKindRight,
    pub clk_l: [Option<u8>; 4],
    pub clk_r: [Option<u8>; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Enum)]
pub enum HardRowKind {
    None,
    Cfg,
    Ams,
    Hdio,
    HdioAms,
    HdioLc,
    Pcie,
    PciePlus,
    Cmac,
    Ilkn,
    DfeA,
    DfeG,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Enum)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    HdioLc,
    Gth,
    Gty,
    Gtm,
    Gtf,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub side: ColSide,
    pub regs: EntityVec<RegId, IoRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Ps {
    pub col: ColId,
    pub has_vcu: bool,
    pub intf_kind: PsIntfKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PsIntfKind {
    Alto,
    Da6,
    Da7,
    Da8,
    Dc12,
    Mx8,
}

impl Ps {
    pub fn height(self) -> usize {
        if self.has_vcu { 240 } else { 180 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

impl Grid {
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / 60)
    }

    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 60)
    }

    pub fn row_reg_rclk(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 60 + 30)
    }

    pub fn row_rclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 60 * 60 + 30)
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.regs)
    }

    pub fn rows(&self) -> EntityIds<RowId> {
        EntityIds::new(self.regs * 60)
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

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "kind": match self.kind {
                GridKind::Ultrascale => "ultrascale",
                GridKind::UltrascalePlus => "ultrascaleplus",
            },
            "columns": Vec::from_iter(self.columns.values().map(|column| json!({
                "l": match column.l {
                    ColumnKindLeft::CleL => "CLEL".to_string(),
                    ColumnKindLeft::CleM(CleMKind::Plain) => "CLEM".to_string(),
                    ColumnKindLeft::CleM(CleMKind::ClkBuf) => "CLEM:CLKBUF".to_string(),
                    ColumnKindLeft::CleM(CleMKind::Laguna) => "CLEM:LAGUNA".to_string(),
                    ColumnKindLeft::Bram(BramKind::Plain) => "BRAM".to_string(),
                    ColumnKindLeft::Bram(BramKind::Td) => "BRAM:TD".to_string(),
                    ColumnKindLeft::Bram(BramKind::AuxClmp) => "BRAM:AUXCLMP".to_string(),
                    ColumnKindLeft::Bram(BramKind::AuxClmpMaybe) => "BRAM:AUXCLMP_MAYBE".to_string(),
                    ColumnKindLeft::Bram(BramKind::BramClmp) => "BRAM:BRAMCLMP".to_string(),
                    ColumnKindLeft::Bram(BramKind::BramClmpMaybe) => "BRAM:BRAMCLMP_MAYBE".to_string(),
                    ColumnKindLeft::Uram => "URAM".to_string(),
                    ColumnKindLeft::Hard(HardKind::Clk, i) => format!("HARD:CLK:{i}"),
                    ColumnKindLeft::Hard(HardKind::NonClk, i) => format!("HARD:NON_CLK:{i}"),
                    ColumnKindLeft::Hard(HardKind::Term, i) => format!("HARD:TERM:{i}"),
                    ColumnKindLeft::Io(i) => format!("IO:{i}"),
                    ColumnKindLeft::Gt(i) => format!("GT:{i}"),
                    ColumnKindLeft::Sdfec => "SDFEC".to_string(),
                    ColumnKindLeft::DfeC => "DFE_C".to_string(),
                    ColumnKindLeft::DfeDF => "DFE_DF".to_string(),
                    ColumnKindLeft::DfeE => "DFE_E".to_string(),
                },
                "r": match column.r {
                    ColumnKindRight::CleL(CleLKind::Plain) => "CLEL".to_string(),
                    ColumnKindRight::CleL(CleLKind::Dcg10) => "CLEL:DCG10".to_string(),
                    ColumnKindRight::Dsp(DspKind::Plain) => "DSP".to_string(),
                    ColumnKindRight::Dsp(DspKind::ClkBuf) => "DSP:CLKBUF".to_string(),
                    ColumnKindRight::Uram => "URAM".to_string(),
                    ColumnKindRight::Hard(HardKind::Clk, i) => format!("HARD:CLK:{i}"),
                    ColumnKindRight::Hard(HardKind::NonClk, i) => format!("HARD:NON_CLK:{i}"),
                    ColumnKindRight::Hard(HardKind::Term, i) => format!("HARD:TERM:{i}"),
                    ColumnKindRight::Io(i) => format!("IO:{i}"),
                    ColumnKindRight::Gt(i) => format!("GT:{i}"),
                    ColumnKindRight::DfeB => "DFE_B".to_string(),
                    ColumnKindRight::DfeC => "DFE_C".to_string(),
                    ColumnKindRight::DfeDF => "DFE_DF".to_string(),
                    ColumnKindRight::DfeE => "DFE_E".to_string(),
                },
                "clk_l": column.clk_l,
                "clk_r": column.clk_r,
            }))),
            "cols_vbrk": self.cols_vbrk,
            "cols_fsr_gap": self.cols_fsr_gap,
            "cols_hard": Vec::from_iter(self.cols_hard.iter().map(|hcol| json!({
                "col": hcol.col,
                "regs": Vec::from_iter(hcol.regs.values().map(|kind| match kind {
                    HardRowKind::None => serde_json::Value::Null,
                    HardRowKind::Cfg => "CFG".into(),
                    HardRowKind::Ams => "AMS".into(),
                    HardRowKind::Pcie => "PCIE".into(),
                    HardRowKind::PciePlus => "PCIE4C".into(),
                    HardRowKind::Cmac => "CMAC".into(),
                    HardRowKind::Ilkn => "ILKN".into(),
                    HardRowKind::DfeA => "DFE_A".into(),
                    HardRowKind::DfeG => "DFE_G".into(),
                    HardRowKind::Hdio => "HDIO".into(),
                    HardRowKind::HdioAms => "HDIO:AMS".into(),
                    HardRowKind::HdioLc => "HDIOLC".into(),
                })),
            }))),
            "cols_io": Vec::from_iter(self.cols_io.iter().map(|iocol| json!({
                "col": iocol.col,
                "regs": Vec::from_iter(iocol.regs.values().map(|kind| match kind {
                    IoRowKind::None => serde_json::Value::Null,
                    IoRowKind::Hpio => "HPIO".into(),
                    IoRowKind::Hrio => "HRIO".into(),
                    IoRowKind::HdioLc => "HDIOLC".into(),
                    IoRowKind::Gth => "GTH".into(),
                    IoRowKind::Gty => "GTY".into(),
                    IoRowKind::Gtm => "GTM".into(),
                    IoRowKind::Gtf => "GTF".into(),
                    IoRowKind::HsAdc => "HSADC".into(),
                    IoRowKind::HsDac => "HSDAC".into(),
                    IoRowKind::RfAdc => "RFADC".into(),
                    IoRowKind::RfDac => "RFDAC".into(),
                })),
            }))),
            "regs": self.regs,
            "ps": match self.ps {
                None => serde_json::Value::Null,
                Some(ps) => json!({
                    "col": ps.col,
                    "has_vcu": ps.has_vcu,
                }),
            },
            "has_hbm": self.has_hbm,
            "has_csec": self.has_csec,
            "is_alt_cfg": self.is_alt_cfg,
            "is_dmc": self.is_dmc,
        })
    }
}

impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: {v:?}", v = self.kind)?;
        if let Some(ps) = self.ps {
            write!(f, "\tPS {v:?}", v = ps.intf_kind)?;
            if ps.has_vcu {
                write!(f, " VCU")?;
            }
            writeln!(f)?;
        }
        if self.has_hbm {
            writeln!(f, "\tHAS HBM")?;
        }
        if self.has_csec {
            writeln!(f, "\tHAS CSEC")?;
        }
        if self.is_dmc {
            writeln!(f, "\tIS DMC")?;
        }
        if self.is_alt_cfg {
            writeln!(f, "\tIS ALT CFG")?;
        }
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            if self.cols_vbrk.contains(&col) {
                writeln!(f, "\t\t--- break")?;
            }
            if self.cols_fsr_gap.contains(&col) {
                writeln!(f, "\t\t--- FSR gap")?;
            }
            if matches!(
                cd.l,
                ColumnKindLeft::Uram
                    | ColumnKindLeft::Hard(_, _)
                    | ColumnKindLeft::DfeC
                    | ColumnKindLeft::DfeDF
                    | ColumnKindLeft::DfeE
            ) {
                write!(f, "\t\tX{cl}.R-X{c}.L: ", cl = col - 1, c = col)?;
            } else {
                write!(f, "\t\tX{c}.L: ", c = col.to_idx())?;
            }
            match cd.l {
                ColumnKindLeft::Io(_) => write!(f, "IO")?,
                ColumnKindLeft::Gt(_) => write!(f, "GT")?,
                ColumnKindLeft::CleL => write!(f, "CLEL")?,
                ColumnKindLeft::CleM(CleMKind::Plain) => write!(f, "CLEM")?,
                ColumnKindLeft::CleM(CleMKind::ClkBuf) => write!(f, "CLEM.CLK")?,
                ColumnKindLeft::CleM(CleMKind::Laguna) => write!(f, "CLEM.LAGUNA")?,
                ColumnKindLeft::Bram(BramKind::Plain) => write!(f, "BRAM")?,
                ColumnKindLeft::Bram(BramKind::AuxClmp) => write!(f, "BRAM.AUX_CLMP")?,
                ColumnKindLeft::Bram(BramKind::BramClmp) => write!(f, "BRAM.BRAM_CLMP")?,
                ColumnKindLeft::Bram(BramKind::AuxClmpMaybe) => write!(f, "BRAM.AUX_CLMP*")?,
                ColumnKindLeft::Bram(BramKind::BramClmpMaybe) => write!(f, "BRAM.BRAM_CLMP*")?,
                ColumnKindLeft::Bram(BramKind::Td) => write!(f, "BRAM.TD")?,
                ColumnKindLeft::Uram => write!(f, "URAM")?,
                ColumnKindLeft::Hard(hk, _) => {
                    write!(f, "HARD{}", if hk == HardKind::Clk { " CLK" } else { "" })?
                }
                ColumnKindLeft::Sdfec => write!(f, "SDFEC")?,
                ColumnKindLeft::DfeC => write!(f, "DFE_C")?,
                ColumnKindLeft::DfeDF => write!(f, "DFE_DF")?,
                ColumnKindLeft::DfeE => write!(f, "DFE_E")?,
            }
            if cd.clk_l.iter().any(|x| x.is_some()) {
                write!(f, " CLK")?;
                for v in cd.clk_l {
                    if let Some(v) = v {
                        write!(f, " {v}")?;
                    } else {
                        write!(f, " -")?;
                    }
                }
            }
            if let Some(ps) = self.ps {
                if ps.col == col {
                    write!(f, " PS")?;
                }
            }
            writeln!(f,)?;
            if let ColumnKindLeft::Io(idx) | ColumnKindLeft::Gt(idx) = cd.l {
                let ioc = &self.cols_io[idx];
                for (reg, kind) in &ioc.regs {
                    writeln!(
                        f,
                        "\t\t\tY{y}: {kind:?}",
                        y = self.row_reg_bot(reg).to_idx()
                    )?;
                }
            }
            if let ColumnKindLeft::Hard(_, idx) = cd.l {
                let hc = &self.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    writeln!(
                        f,
                        "\t\t\tY{y}: {kind:?}",
                        y = self.row_reg_bot(reg).to_idx()
                    )?;
                }
            }
            if matches!(
                cd.r,
                ColumnKindRight::Uram
                    | ColumnKindRight::Hard(HardKind::Clk | HardKind::NonClk, _)
                    | ColumnKindRight::DfeC
                    | ColumnKindRight::DfeDF
                    | ColumnKindRight::DfeE
            ) {
                continue;
            }
            write!(f, "\t\tX{c}.R: ", c = col.to_idx())?;
            match cd.r {
                ColumnKindRight::Io(_) => write!(f, "IO")?,
                ColumnKindRight::Gt(_) => write!(f, "GT")?,
                ColumnKindRight::CleL(CleLKind::Plain) => write!(f, "CLEL")?,
                ColumnKindRight::CleL(CleLKind::Dcg10) => write!(f, "CLEL.DCG10")?,
                ColumnKindRight::Dsp(DspKind::Plain) => write!(f, "DSP")?,
                ColumnKindRight::Dsp(DspKind::ClkBuf) => write!(f, "DSP.CLK")?,
                ColumnKindRight::Uram => write!(f, "URAM")?,
                ColumnKindRight::Hard(_, _) => write!(f, "HARD TERM")?,
                ColumnKindRight::DfeB => write!(f, "DFE_B")?,
                ColumnKindRight::DfeC => write!(f, "DFE_C")?,
                ColumnKindRight::DfeDF => write!(f, "DFE_DF")?,
                ColumnKindRight::DfeE => write!(f, "DFE_E")?,
            }
            if cd.clk_r.iter().any(|x| x.is_some()) {
                write!(f, " CLK")?;
                for v in cd.clk_r {
                    if let Some(v) = v {
                        write!(f, " {v}")?;
                    } else {
                        write!(f, " -")?;
                    }
                }
            }
            writeln!(f)?;
            if let ColumnKindRight::Io(idx) | ColumnKindRight::Gt(idx) = cd.r {
                let ioc = &self.cols_io[idx];
                for (reg, kind) in &ioc.regs {
                    writeln!(
                        f,
                        "\t\t\tY{y}: {kind:?}",
                        y = self.row_reg_bot(reg).to_idx()
                    )?;
                }
            }
            if let ColumnKindRight::Hard(__, idx) = cd.r {
                let hc = &self.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    writeln!(
                        f,
                        "\t\t\tY{y}: {kind:?}",
                        y = self.row_reg_bot(reg).to_idx()
                    )?;
                }
            }
        }
        writeln!(f, "\tREGS: {r}", r = self.regs)?;

        Ok(())
    }
}

impl std::fmt::Display for Interposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tPRIMARY: D{}", self.primary)?;
        Ok(())
    }
}
