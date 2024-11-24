use prjcombine_int::grid::{ColId, RowId, SimpleIoCoord, TileIobId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use unnamed_entity::{entity_id, EntityId, EntityIds, EntityVec};

entity_id! {
    pub id RegId u32, delta;
}

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
    pub cfg_io: BTreeMap<SharedCfgPin, SimpleIoCoord>,
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
    pub iob: TileIobId,
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
}
