use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_int::db::BelId;
use prjcombine_int::grid::{ColId, ExpandedGrid, RowId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod expand;

pub mod io;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
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
    pub vref: BTreeSet<IoCoord>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub kind: ColumnKind,
    pub bio: ColumnIoKind,
    pub tio: ColumnIoKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnKind {
    Io,
    CleXL,
    CleXM,
    CleClk,
    Bram,
    Dsp,
    DspPlus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ColumnIoKind {
    None,
    Both,
    Inner,
    Outer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub lio: bool,
    pub rio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Gts {
    None,
    Single(ColId),
    Double(ColId, ColId),
    Quad(ColId, ColId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McbIo {
    pub row: RowId,
    pub bel: BelId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Gtp,
    Mcb,
    ClbColumn(ColId),
    BramRegion(ColId, u32),
    DspRegion(ColId, u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgPin {
    Tck,
    Tdi,
    Tdo,
    Tms,
    CmpCsB,
    Done,
    ProgB,
    Suspend,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtPin {
    TxP(u8),
    TxN(u8),
    RxP(u8),
    RxN(u8),
    ClkP(u8),
    ClkN(u8),
    AVcc,
    AVccPll(u8),
    VtTx,
    VtRx,
    RRef,
    AVttRCal,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
    Nc,
    Gnd,
    VccInt,
    VccAux,
    VccO(u32),
    VccBatt,
    Vfs,
    RFuse,
    Cfg(CfgPin),
    Gt(u32, GtPin),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
    // device bank -> pkg bank
    pub io_banks: BTreeMap<u32, u32>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct IoCoord {
    pub col: ColId,
    pub row: RowId,
    pub bel: BelId,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: &'a BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub bonded_ios: Vec<((ColId, RowId), BelId)>,
}

impl Grid {
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

    pub fn row_hclk(&self, row: RowId) -> RowId {
        RowId::from_idx(row.to_idx() / 16 * 16 + 8)
    }

    pub fn is_25(&self) -> bool {
        self.rows.len() % 32 == 16
    }
}
