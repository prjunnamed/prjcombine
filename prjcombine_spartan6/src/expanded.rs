use prjcombine_int::grid::{ColId, ExpandedGrid, Rect, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use std::collections::BTreeSet;

use crate::grid::{DisabledPart, Grid, IoCoord};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
    pub site_holes: Vec<Rect>,
    pub bs_geom: BitstreamGeom,
    pub io: Vec<Io>,
    pub gt: Vec<Gt>,
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
