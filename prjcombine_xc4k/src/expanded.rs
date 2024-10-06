use prjcombine_int::grid::{ColId, ExpandedGrid, RowId};
use prjcombine_virtex_bitstream::BitstreamGeom;
use unnamed_entity::EntityVec;

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
    pub bs_geom: BitstreamGeom,
    pub spine_frame: usize,
    pub quarter_frame: Option<(usize, usize)>,
    pub col_frame: EntityVec<ColId, usize>,
    pub spine_framebit: usize,
    pub quarter_framebit: Option<(usize, usize)>,
    pub row_framebit: EntityVec<RowId, usize>,
}
