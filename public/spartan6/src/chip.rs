use prjcombine_interconnect::{
    db::BelId,
    grid::{ColId, EdgeIoCoord, RowId, TileIobId},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use unnamed_entity::{EntityId, EntityIds, EntityVec, entity_id};

entity_id! {
    pub id RegId u32, delta;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Chip {
    pub columns: EntityVec<ColId, Column>,
    pub col_clk: ColId,
    pub cols_clk_fold: Option<(ColId, ColId)>,
    pub cols_reg_buf: (ColId, ColId),
    pub rows: EntityVec<RowId, Row>,
    pub rows_midbuf: (RowId, RowId),
    pub rows_hclkbuf: (RowId, RowId),
    pub rows_pci_ce_split: (RowId, RowId),
    pub rows_bank_split: Option<(RowId, RowId)>,
    pub row_mcb_split: Option<RowId>,
    pub gts: Gts,
    pub mcbs: Vec<Mcb>,
    pub cfg_io: BTreeMap<SharedCfgPin, EdgeIoCoord>,
    pub has_encrypt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SharedCfgPin {
    // ×16
    // 0 doubles as DIN, MISO, MISO1
    // 1 doubles as MISO2
    // 2 doubles as MISO3
    Data(u8),
    CsoB,
    InitB,
    RdWrB,
    FcsB,
    FoeB,
    FweB,
    Ldc,
    Hdc,
    Addr(u8),
    Dout, // doubles as BUSY
    Mosi, // doubles as CSI_B, MISO0
    M0,   // doubles as CMPMISO
    M1,
    Cclk,
    UserCclk,
    HswapEn,
    CmpClk,
    CmpMosi,
    Awake,
    Scp(u8), // ×8
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub bio: ColumnIoKind,
    pub tio: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    CleXL,
    CleXM,
    CleClk,
    Bram,
    Dsp,
    DspPlus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ColumnIoKind {
    None,
    Both,
    Inner,
    Outer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Row {
    pub lio: bool,
    pub rio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Gts {
    None,
    Single(ColId),
    Double(ColId, ColId),
    Quad(ColId, ColId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct McbIo {
    pub row: RowId,
    pub iob: TileIobId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Mcb {
    pub row_mcb: RowId,
    pub row_mui: [RowId; 8],
    pub iop_dq: [RowId; 8],
    pub iop_dqs: [RowId; 2],
    pub io_dm: [McbIo; 2],
    pub iop_clk: RowId,
    pub io_addr: [McbIo; 15],
    pub io_ba: [McbIo; 3],
    pub io_ras: McbIo,
    pub io_cas: McbIo,
    pub io_we: McbIo,
    pub io_odt: McbIo,
    pub io_cke: McbIo,
    pub io_reset: McbIo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DisabledPart {
    Gtp,
    Mcb,
    ClbColumn(ColId),
    BramRegion(ColId, RegId),
    DspRegion(ColId, RegId),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum DcmKind {
    Bot,
    BotMid,
    Top,
    TopMid,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PllKind {
    BotOut0,
    BotOut1,
    BotNoOut,
    TopOut0,
    TopOut1,
    TopNoOut,
}

impl Chip {
    pub fn col_lio(&self) -> ColId {
        ColId::from_idx(0)
    }

    pub fn col_rio(&self) -> ColId {
        ColId::from_idx(self.columns.len() - 1)
    }

    pub fn row_bot(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_top(&self) -> RowId {
        self.rows.next_id()
    }

    pub fn row_bio_outer(&self) -> RowId {
        RowId::from_idx(0)
    }

    pub fn row_bio_inner(&self) -> RowId {
        RowId::from_idx(1)
    }

    pub fn row_tio_outer(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 1)
    }

    pub fn row_tio_inner(&self) -> RowId {
        RowId::from_idx(self.rows.len() - 2)
    }

    pub fn get_mcb(&self, row: RowId) -> &Mcb {
        for mcb in &self.mcbs {
            if mcb.row_mcb == row {
                return mcb;
            }
        }
        unreachable!()
    }

    pub fn row_clk(&self) -> RowId {
        RowId::from_idx(self.rows.len() / 2)
    }

    #[inline]
    pub fn row_to_reg(&self, row: RowId) -> RegId {
        RegId::from_idx(row.to_idx() / 16)
    }

    #[inline]
    pub fn row_reg_bot(&self, reg: RegId) -> RowId {
        RowId::from_idx(reg.to_idx() * 16)
    }

    #[inline]
    pub fn row_reg_hclk(&self, reg: RegId) -> RowId {
        self.row_reg_bot(reg) + 8
    }

    pub fn regs(&self) -> EntityIds<RegId> {
        EntityIds::new(self.rows.len() / 16)
    }

    #[inline]
    pub fn row_hclk(&self, row: RowId) -> RowId {
        self.row_reg_hclk(self.row_to_reg(row))
    }

    pub fn is_25(&self) -> bool {
        self.rows.len() % 32 == 16
    }

    pub fn get_dcms(&self) -> Vec<(RowId, DcmKind)> {
        match self.rows.len() {
            64 | 80 => vec![
                (self.row_bot() + 8, DcmKind::Bot),
                (self.row_top() - 24, DcmKind::Top),
            ],
            128 => vec![
                (self.row_bot() + 8, DcmKind::Bot),
                (self.row_bot() + 40, DcmKind::BotMid),
                (self.row_top() - 56, DcmKind::Top),
                (self.row_top() - 24, DcmKind::TopMid),
            ],
            192 => vec![
                (self.row_bot() + 8, DcmKind::Bot),
                (self.row_bot() + 40, DcmKind::BotMid),
                (self.row_bot() + 72, DcmKind::BotMid),
                (self.row_top() - 88, DcmKind::Top),
                (self.row_top() - 56, DcmKind::TopMid),
                (self.row_top() - 24, DcmKind::TopMid),
            ],
            _ => unreachable!(),
        }
    }

    pub fn get_plls(&self) -> Vec<(RowId, PllKind)> {
        match self.rows.len() {
            64 | 80 => vec![
                (self.row_bot() + 24, PllKind::BotOut1),
                (self.row_top() - 8, PllKind::TopOut1),
            ],
            128 => vec![
                (self.row_bot() + 24, PllKind::BotOut1),
                (self.row_bot() + 56, PllKind::BotOut0),
                (self.row_top() - 40, PllKind::TopOut0),
                (self.row_top() - 8, PllKind::TopOut1),
            ],
            192 => vec![
                (self.row_bot() + 24, PllKind::BotOut1),
                (self.row_bot() + 56, PllKind::BotNoOut),
                (self.row_bot() + 88, PllKind::BotOut0),
                (self.row_top() - 72, PllKind::TopOut0),
                (self.row_top() - 40, PllKind::TopNoOut),
                (self.row_top() - 8, PllKind::TopOut1),
            ],
            _ => unreachable!(),
        }
    }

    pub fn get_io_crd(&self, col: ColId, row: RowId, bel: BelId) -> EdgeIoCoord {
        if col == self.col_lio() {
            EdgeIoCoord::L(row, TileIobId::from_idx(bel.to_idx()))
        } else if col == self.col_rio() {
            EdgeIoCoord::R(row, TileIobId::from_idx(bel.to_idx()))
        } else if row == self.row_bio_inner() {
            EdgeIoCoord::B(col, TileIobId::from_idx(bel.to_idx()))
        } else if row == self.row_bio_outer() {
            EdgeIoCoord::B(col, TileIobId::from_idx(bel.to_idx() + 2))
        } else if row == self.row_tio_inner() {
            EdgeIoCoord::T(col, TileIobId::from_idx(bel.to_idx()))
        } else if row == self.row_tio_outer() {
            EdgeIoCoord::T(col, TileIobId::from_idx(bel.to_idx() + 2))
        } else {
            unreachable!()
        }
    }

    pub fn get_io_loc(&self, io: EdgeIoCoord) -> (ColId, RowId, BelId) {
        match io {
            EdgeIoCoord::T(col, iob) => {
                if iob.to_idx() < 2 {
                    (col, self.row_tio_inner(), BelId::from_idx(iob.to_idx()))
                } else {
                    (col, self.row_tio_outer(), BelId::from_idx(iob.to_idx() - 2))
                }
            }
            EdgeIoCoord::R(row, iob) => (self.col_rio(), row, BelId::from_idx(iob.to_idx())),
            EdgeIoCoord::B(col, iob) => {
                if iob.to_idx() < 2 {
                    (col, self.row_bio_inner(), BelId::from_idx(iob.to_idx()))
                } else {
                    (col, self.row_bio_outer(), BelId::from_idx(iob.to_idx() - 2))
                }
            }
            EdgeIoCoord::L(row, iob) => (self.col_lio(), row, BelId::from_idx(iob.to_idx())),
        }
    }

    pub fn get_io_bank(&self, io: EdgeIoCoord) -> u32 {
        match io {
            EdgeIoCoord::T(_, _) => 0,
            EdgeIoCoord::R(row, _) => {
                if let Some((_, rs)) = self.rows_bank_split {
                    if row < rs { 1 } else { 5 }
                } else {
                    1
                }
            }
            EdgeIoCoord::B(_, _) => 2,
            EdgeIoCoord::L(row, _) => {
                if let Some((rs, _)) = self.rows_bank_split {
                    if row < rs { 3 } else { 4 }
                } else {
                    3
                }
            }
        }
    }

    pub fn get_bonded_ios(&self) -> Vec<EdgeIoCoord> {
        let mut res = vec![];
        // TIO
        for (col, &cd) in &self.columns {
            if cd.tio == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // outer
                (3, cd.tio == ColumnIoKind::Inner),
                (2, cd.tio == ColumnIoKind::Inner),
                // inner
                (1, cd.tio == ColumnIoKind::Outer),
                (0, cd.tio == ColumnIoKind::Outer),
            ] {
                if !unused {
                    res.push(EdgeIoCoord::T(col, TileIobId::from_idx(iob)));
                }
            }
        }
        // RIO
        for (row, &rd) in self.rows.iter().rev() {
            if rd.rio {
                for iob in [1, 0] {
                    res.push(EdgeIoCoord::R(row, TileIobId::from_idx(iob)));
                }
            }
        }
        // BIO
        for (col, &cd) in self.columns.iter().rev() {
            if cd.bio == ColumnIoKind::None {
                continue;
            }
            for (iob, unused) in [
                // outer
                (3, cd.bio == ColumnIoKind::Inner),
                (2, cd.bio == ColumnIoKind::Inner),
                // inner
                (1, cd.bio == ColumnIoKind::Outer),
                (0, cd.bio == ColumnIoKind::Outer),
            ] {
                if !unused {
                    res.push(EdgeIoCoord::B(col, TileIobId::from_idx(iob)));
                }
            }
        }
        // LIO
        for (row, &rd) in &self.rows {
            if rd.lio {
                for iob in [1, 0] {
                    res.push(EdgeIoCoord::L(row, TileIobId::from_idx(iob)));
                }
            }
        }
        res
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "columns": Vec::from_iter(self.columns.values().map(|column| {
                json!({
                    "kind": match column.kind {
                        ColumnKind::Io => "IO",
                        ColumnKind::CleXL => "CLEXL",
                        ColumnKind::CleXM => "CLEXM",
                        ColumnKind::CleClk => "CLEXL_CLK",
                        ColumnKind::Bram => "BRAM",
                        ColumnKind::Dsp => "DSP",
                        ColumnKind::DspPlus => "DSP_PLUS",
                    },
                    "bio": match column.bio {
                        ColumnIoKind::None => "NONE",
                        ColumnIoKind::Both => "BOTH",
                        ColumnIoKind::Outer => "OUTER",
                        ColumnIoKind::Inner => "INNER",
                    },
                    "tio": match column.tio {
                        ColumnIoKind::None => "NONE",
                        ColumnIoKind::Both => "BOTH",
                        ColumnIoKind::Outer => "OUTER",
                        ColumnIoKind::Inner => "INNER",
                    },
                })
            })),
            "col_clk": self.col_clk,
            "cols_clk_fold": self.cols_clk_fold,
            "cols_reg_buf": self.cols_reg_buf,
            "rows": self.rows,
            "rows_midbuf": self.rows_midbuf,
            "rows_hclkbuf": self.rows_hclkbuf,
            "rows_pci_ce_split": self.rows_pci_ce_split,
            "rows_bank_split": self.rows_bank_split,
            "row_mcb_split": self.row_mcb_split,
            "gts": match self.gts {
                Gts::None => serde_json::Value::Null,
                Gts::Single(col_l) => json!({
                    "num": 1,
                    "col_l": col_l,
                }),
                Gts::Double(col_l, col_r) => json!({
                    "num": 2,
                    "col_l": col_l,
                    "col_r": col_r,
                }),
                Gts::Quad(col_l, col_r) => json!({
                    "num": 4,
                    "col_l": col_l,
                    "col_r": col_r,
                }),
            },
            "mcbs": self.mcbs,
            "cfg_io": serde_json::Map::from_iter(self.cfg_io.iter().map(|(k, io)| {
                (match k {
                    SharedCfgPin::Data(i) => format!("D{i}"),
                    SharedCfgPin::Addr(i) => format!("A{i}"),
                    SharedCfgPin::Scp(i) => format!("SCP{i}"),
                    SharedCfgPin::CsoB => "CSO_B".to_string(),
                    SharedCfgPin::RdWrB => "RDWR_B".to_string(),
                    SharedCfgPin::Dout => "DOUT".to_string(),
                    SharedCfgPin::InitB => "INIT_B".to_string(),
                    SharedCfgPin::Cclk => "CCLK".to_string(),
                    SharedCfgPin::UserCclk => "USER_CCLK".to_string(),
                    SharedCfgPin::Mosi => "MOSI".to_string(),
                    SharedCfgPin::CmpMosi => "CMP_MOSI".to_string(),
                    SharedCfgPin::CmpClk => "CMP_CLK".to_string(),
                    SharedCfgPin::FcsB => "FCS_B".to_string(),
                    SharedCfgPin::FoeB => "FOE_B".to_string(),
                    SharedCfgPin::FweB => "FWE_B".to_string(),
                    SharedCfgPin::Ldc => "LDC".to_string(),
                    SharedCfgPin::M0 => "M0".to_string(),
                    SharedCfgPin::M1 => "M1".to_string(),
                    SharedCfgPin::Hdc => "HDC".to_string(),
                    SharedCfgPin::HswapEn => "HSWAP_EN".to_string(),
                    SharedCfgPin::Awake => "AWAKE".to_string(),
                }, io.to_string().into())
            })),
            "has_encrypt": self.has_encrypt,
        })
    }
}

impl std::fmt::Display for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\tKIND: Spartan6")?;
        writeln!(f, "\tCOLS:")?;
        for (col, cd) in &self.columns {
            write!(f, "\t\tX{col}: ")?;
            match cd.kind {
                ColumnKind::Io => write!(f, "IO")?,
                ColumnKind::CleXL => write!(f, "CLEXL")?,
                ColumnKind::CleXM => write!(f, "CLEXM")?,
                ColumnKind::CleClk => write!(f, "CLEXL+CLK")?,
                ColumnKind::Bram => write!(f, "BRAM")?,
                ColumnKind::Dsp => write!(f, "DSP")?,
                ColumnKind::DspPlus => write!(f, "DSP*")?,
            }
            match cd.bio {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => write!(f, " BIO: I-")?,
                ColumnIoKind::Outer => write!(f, " BIO: -O")?,
                ColumnIoKind::Both => write!(f, " BIO: IO")?,
            }
            match cd.tio {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => write!(f, " TIO: I-")?,
                ColumnIoKind::Outer => write!(f, " TIO: -O")?,
                ColumnIoKind::Both => write!(f, " TIO: IO")?,
            }
            if let Some((cl, cr)) = self.cols_clk_fold {
                if col == cl || col == cr {
                    write!(f, " FOLD")?;
                }
            }
            if col == self.cols_reg_buf.0 || col == self.cols_reg_buf.1 {
                write!(f, " REGBUF")?;
            }
            if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = self.gts {
                if col == cl {
                    write!(f, " LGT")?;
                }
            }
            if let Gts::Double(_, cr) | Gts::Quad(_, cr) = self.gts {
                if col == cr {
                    write!(f, " RGT")?;
                }
            }
            writeln!(f,)?;
        }
        writeln!(f, "\tROWS:")?;
        for (row, rd) in &self.rows {
            if row.to_idx() != 0 && row.to_idx() % 16 == 0 {
                writeln!(f, "\t\t--- clock break")?;
            }
            if row.to_idx() % 16 == 8 {
                writeln!(f, "\t\t--- clock row")?;
            }
            if row == self.row_clk() {
                writeln!(f, "\t\t--- spine row")?;
            }
            if let Some((rl, rr)) = self.rows_bank_split {
                if row == rl {
                    writeln!(f, "\t\t--- left bank split")?;
                }
                if row == rr {
                    writeln!(f, "\t\t--- right bank split")?;
                }
            }
            if Some(row) == self.row_mcb_split {
                writeln!(f, "\t\t--- MCB split")?;
            }
            write!(f, "\t\tY{r}: ", r = row.to_idx())?;
            if rd.lio {
                write!(f, " LIO")?;
            }
            if rd.rio {
                write!(f, " RIO")?;
            }
            if row == self.rows_midbuf.0 || row == self.rows_midbuf.1 {
                write!(f, " MIDBUF")?;
            }
            if row == self.rows_hclkbuf.0 || row == self.rows_hclkbuf.1 {
                write!(f, " HCLKBUF")?;
            }
            for (i, mcb) in self.mcbs.iter().enumerate() {
                if row == mcb.row_mcb {
                    write!(f, " MCB{i}.MCB")?;
                }
                for (j, &r) in mcb.row_mui.iter().enumerate() {
                    if row == r {
                        write!(f, " MCB{i}.MUI{j}")?;
                    }
                }
                for (j, &r) in mcb.iop_dq.iter().enumerate() {
                    if row == r {
                        write!(f, " MCB{i}.DQ({jj0},{jj1})", jj0 = j * 2, jj1 = j * 2 + 1)?;
                    }
                }
                for (j, &r) in mcb.iop_dqs.iter().enumerate() {
                    if row == r {
                        write!(f, " MCB{i}.DQS{j}")?;
                    }
                }
                if row == mcb.iop_clk {
                    write!(f, " MCB{i}.CLK")?;
                }
                let mut pins: [Option<&'static str>; 2] = [None, None];
                for (pin, io) in [
                    ("DM0", mcb.io_dm[0]),
                    ("DM1", mcb.io_dm[1]),
                    ("A0", mcb.io_addr[0]),
                    ("A1", mcb.io_addr[1]),
                    ("A2", mcb.io_addr[2]),
                    ("A3", mcb.io_addr[3]),
                    ("A4", mcb.io_addr[4]),
                    ("A5", mcb.io_addr[5]),
                    ("A6", mcb.io_addr[6]),
                    ("A7", mcb.io_addr[7]),
                    ("A8", mcb.io_addr[8]),
                    ("A9", mcb.io_addr[9]),
                    ("A10", mcb.io_addr[10]),
                    ("A11", mcb.io_addr[11]),
                    ("A12", mcb.io_addr[12]),
                    ("A13", mcb.io_addr[13]),
                    ("A14", mcb.io_addr[14]),
                    ("BA0", mcb.io_ba[0]),
                    ("BA1", mcb.io_ba[1]),
                    ("BA2", mcb.io_ba[2]),
                    ("RAS", mcb.io_ras),
                    ("CAS", mcb.io_cas),
                    ("WE", mcb.io_we),
                    ("ODT", mcb.io_odt),
                    ("CKE", mcb.io_cke),
                    ("RST", mcb.io_reset),
                ] {
                    if row == io.row {
                        pins[io.iob.to_idx()] = Some(pin);
                    }
                }
                if pins.iter().any(|x| x.is_some()) {
                    write!(
                        f,
                        " MCB{i}.({p0},{p1})",
                        p0 = pins[0].unwrap(),
                        p1 = pins[1].unwrap()
                    )?;
                }
            }
            writeln!(f)?;
        }
        match self.gts {
            Gts::None => (),
            Gts::Single(..) => writeln!(f, "\tGTS: SINGLE")?,
            Gts::Double(..) => writeln!(f, "\tGTS: DOUBLE")?,
            Gts::Quad(..) => writeln!(f, "\tGTS: QUAD")?,
        }
        writeln!(f, "\tCFG PINS:")?;
        for (k, v) in &self.cfg_io {
            writeln!(f, "\t\t{k:?}: {v}")?;
        }
        if self.has_encrypt {
            writeln!(f, "\tHAS ENCRYPT")?;
        }
        Ok(())
    }
}
