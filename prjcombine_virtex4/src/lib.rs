use prjcombine_entity::{entity_id, EntityId, EntityIds, EntityPartVec, EntityVec};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub mod bond;
mod expand;

use bond::{PsPin, SharedCfgPin};
pub use expand::expand_grid;

entity_id! {
    pub id RegId u32, delta;
    pub id TileIobId u8;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub kind: GridKind,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GridKind {
    Virtex4,
    Virtex5,
    Virtex6,
    Virtex7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CfgRowKind {
    Dcm,
    Ccm,
    Sysmon,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GtKind {
    Gtp,
    Gtx,
    Gth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IoKind {
    Hpio,
    Hrio,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IoColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, Option<IoKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GtColumn {
    pub col: ColId,
    pub regs: EntityVec<RegId, Option<GtKind>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HardColumn {
    pub col: ColId,
    pub rows_emac: Vec<RowId>,
    pub rows_pcie: Vec<RowId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Pcie2Kind {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Pcie2 {
    pub kind: Pcie2Kind,
    pub col: ColId,
    pub row: RowId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisabledPart {
    Emac(RowId),
    GtxRow(RegId),
    SysMon,
    Gtp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExtraDie {
    Gtz(GtzLoc),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GtzLoc {
    Top,
    Bottom,
}

#[derive(Clone, Debug)]
pub struct DieFrameGeom {
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub spine_frame: EntityVec<RegId, usize>,
}

pub struct ExpandedDevice<'a> {
    pub kind: GridKind,
    pub grids: EntityVec<DieId, &'a Grid>,
    pub grid_master: DieId,
    pub egrid: ExpandedGrid<'a>,
    pub disabled: BTreeSet<DisabledPart>,
    pub extras: Vec<ExtraDie>,
    pub bs_geom: BitstreamGeom,
    pub frames: EntityVec<DieId, DieFrameGeom>,
    pub col_cfg: ColId,
    pub col_clk: ColId,
    pub col_lio: Option<ColId>,
    pub col_rio: Option<ColId>,
    pub col_lcio: Option<ColId>,
    pub col_rcio: Option<ColId>,
    pub col_lgt: Option<ColId>,
    pub col_rgt: Option<ColId>,
    pub col_mgt: Option<(ColId, ColId)>,
    pub row_dcmiob: Option<RowId>,
    pub row_iobdcm: Option<RowId>,
    pub io: Vec<Io>,
    pub gt: Vec<Gt>,
    pub gtz: Vec<Gtz>,
    pub sysmon: Vec<SysMon>,
    pub cfg_io: BTreeMap<SharedCfgPin, IoCoord>,
    pub ps_io: BTreeMap<PsPin, PsIo>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IoDiffKind {
    None,
    P(IoCoord),
    N(IoCoord),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum IoVrKind {
    #[default]
    None,
    VrP,
    VrN,
}

#[derive(Debug)]
pub struct Io {
    pub crd: IoCoord,
    pub name: String,
    pub bank: u32,
    pub biob: u32,
    pub pkgid: u32,
    pub byte: Option<u32>,
    pub kind: IoKind,
    pub diff: IoDiffKind,
    pub is_vref: bool,
    pub is_lc: bool,
    pub is_gc: bool,
    pub is_srcc: bool,
    pub is_mrcc: bool,
    pub is_dqs: bool,
    pub vr: IoVrKind,
}

#[derive(Debug)]
pub struct Gt {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub bank: u32,
    pub kind: GtKind,
    pub pads_clk: Vec<(String, String)>,
    pub pads_tx: Vec<(String, String)>,
    pub pads_rx: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct SysMon {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub bank: u32,
    pub pad_vp: String,
    pub pad_vn: String,
    pub vaux: Vec<Option<(IoCoord, IoCoord)>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct IoCoord {
    pub die: DieId,
    pub col: ColId,
    pub row: RowId,
    pub iob: TileIobId,
}

#[derive(Clone, Debug)]
pub struct PsIo {
    pub bank: u32,
    pub name: String,
}

#[derive(Debug)]
pub struct Gtz {
    pub loc: GtzLoc,
    pub bank: u32,
    pub pads_clk: Vec<(String, String)>,
    pub pads_tx: Vec<(String, String)>,
    pub pads_rx: Vec<(String, String)>,
}

impl Grid {
    #[inline]
    pub fn rows_per_reg(&self) -> usize {
        match self.kind {
            GridKind::Virtex4 => 16,
            GridKind::Virtex5 => 20,
            GridKind::Virtex6 => 40,
            GridKind::Virtex7 => 50,
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
}

impl<'a> ExpandedDevice<'a> {
    pub fn adjust_vivado(&mut self) {
        if self.kind == GridKind::Virtex7 {
            let lvb6 = self
                .egrid
                .db
                .wires
                .iter()
                .find_map(|(k, v)| if v.name == "LVB.6" { Some(k) } else { None })
                .unwrap();
            let mut cursed_wires = HashSet::new();
            for i in 1..self.grids.len() {
                let dieid_s = DieId::from_idx(i - 1);
                let dieid_n = DieId::from_idx(i);
                let die_s = self.egrid.die(dieid_s);
                let die_n = self.egrid.die(dieid_n);
                for col in die_s.cols() {
                    let row_s = die_s.rows().next_back().unwrap() - 49;
                    let row_n = die_n.rows().next().unwrap() + 1;
                    if !die_s[(col, row_s)].nodes.is_empty()
                        && !die_n[(col, row_n)].nodes.is_empty()
                    {
                        cursed_wires.insert((dieid_s, (col, row_s), lvb6));
                    }
                }
            }
            self.egrid.blackhole_wires.extend(cursed_wires);
        }
    }
}
