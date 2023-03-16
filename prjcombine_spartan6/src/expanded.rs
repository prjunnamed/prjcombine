use enum_map::EnumMap;
use prjcombine_entity::{EntityPartVec, EntityVec};
use prjcombine_int::{
    db::Dir,
    grid::{ColId, ExpandedGrid, Rect, RowId},
};
use prjcombine_virtex_bitstream::BitstreamGeom;
use std::collections::{BTreeSet, HashMap};

use crate::grid::{DisabledPart, Grid, IoCoord, RegId};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub site_holes: Vec<Rect>,
    pub bs_geom: BitstreamGeom,
    pub io: Vec<Io>,
    pub gt: Vec<Gt>,
    pub col_frame: EntityVec<RegId, EntityVec<ColId, usize>>,
    pub bram_frame: EntityVec<RegId, EntityPartVec<ColId, usize>>,
    pub iob_frame: HashMap<(ColId, RowId), usize>,
    pub reg_frame: EnumMap<Dir, usize>,
}

pub struct Io {
    pub crd: IoCoord,
    pub name: String,
    pub bank: u32,
    pub diff: IoDiffKind,
}

pub struct Gt {
    pub bank: u32,
    pub pads_clk: Vec<(String, String)>,
    pub pads_tx: Vec<(String, String)>,
    pub pads_rx: Vec<(String, String)>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IoDiffKind {
    P(IoCoord),
    N(IoCoord),
}

impl ExpandedDevice<'_> {
    pub fn in_site_hole(&self, col: ColId, row: RowId) -> bool {
        for hole in &self.site_holes {
            if hole.contains(col, row) {
                return true;
            }
        }
        false
    }
}
