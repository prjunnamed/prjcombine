use prjcombine_int::grid::ExpandedGrid;

use crate::grid::Grid;

pub struct ExpandedDevice<'a> {
    pub grid: &'a Grid,
    pub egrid: ExpandedGrid<'a>,
}
