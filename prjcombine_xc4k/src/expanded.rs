use prjcombine_int::grid::ExpandedGrid;

use crate::grid::{Grid, IoCoord};

#[derive(Debug)]
pub struct Io {
    pub name: String,
    pub crd: IoCoord,
}

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
    pub io: Vec<Io>,
}
