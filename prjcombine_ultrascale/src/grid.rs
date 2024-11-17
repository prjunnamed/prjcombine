use enum_map::Enum;
use prjcombine_int::grid::{ColId, DieId, RowId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use unnamed_entity::{entity_id, EntityId, EntityIds, EntityVec};

entity_id! {
    pub id RegId u32, delta;
    pub id HdioIobId u8;
    pub id HpioIobId u8;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Interposer {
    pub primary: DieId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    pub is_dmc: bool,
    pub is_alt_cfg: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CleMKind {
    Plain,
    ClkBuf,
    Laguna,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BramKind {
    Plain,
    AuxClmp,
    BramClmp,
    AuxClmpMaybe,
    BramClmpMaybe,
    Td,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CleLKind {
    Plain,
    Dcg10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HardKind {
    Clk,
    NonClk,
    Term,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DspKind {
    Plain,
    ClkBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub l: ColumnKindLeft,
    pub r: ColumnKindRight,
    pub clk_l: [Option<u8>; 4],
    pub clk_r: [Option<u8>; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, Enum)]
pub enum HardRowKind {
    None,
    Cfg,
    Ams,
    Hdio,
    HdioAms,
    Pcie,
    PciePlus,
    Cmac,
    Ilkn,
    DfeA,
    DfeG,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, HardRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Enum)]
pub enum IoRowKind {
    None,
    Hpio,
    Hrio,
    Gth,
    Gty,
    Gtm,
    Gtf,
    HsAdc,
    HsDac,
    RfAdc,
    RfDac,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub side: ColSide,
    pub regs: EntityVec<RegId, IoRowKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
        if self.has_vcu {
            240
        } else {
            180
        }
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
    HdioIob(DieId, ColId, RegId, HdioIobId),
    HpioIob(DieId, ColId, RegId, HpioIobId),
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
}
