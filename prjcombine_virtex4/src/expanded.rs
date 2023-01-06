use crate::bond::{PsPin, SharedCfgPin};
use crate::grid::{DisabledPart, ExtraDie, Grid, GridKind, GtKind, GtzLoc, IoKind, RegId};
use bimap::BiHashMap;
use prjcombine_entity::{entity_id, EntityId, EntityPartVec, EntityVec};
use prjcombine_int::grid::{ColId, DieId, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use std::collections::{BTreeMap, BTreeSet, HashSet};

entity_id! {
    pub id TileIobId u8;
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
    pub site_holes: EntityVec<DieId, Vec<Rect>>,
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
    pub cfg_io: BiHashMap<SharedCfgPin, IoCoord>,
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

impl<'a> ExpandedDevice<'a> {
    pub fn adjust_vivado(&mut self) {
        if self.kind == GridKind::Virtex7 {
            let lvb6 = self.egrid.db.wires.get("LVB.6").unwrap().0;
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

    pub fn in_site_hole(&self, die: DieId, col: ColId, row: RowId) -> bool {
        for hole in &self.site_holes[die] {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }
}
