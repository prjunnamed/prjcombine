use prjcombine_int::grid::ExpandedGrid;
use prjcombine_virtex_bitstream::BitstreamGeom;
use std::collections::BTreeSet;

use crate::grid::{DisabledPart, Grid, IoCoord};

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub disabled: BTreeSet<DisabledPart>,
    pub egrid: ExpandedGrid<'a>,
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
