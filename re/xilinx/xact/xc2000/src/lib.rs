use std::ops::Range;

use prjcombine_entity::EntityVec;
use prjcombine_interconnect::grid::{ColId, EdgeIoCoord, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub chip: &'a Chip,
    pub col_x: EntityVec<ColId, Range<usize>>,
    pub row_y: EntityVec<RowId, Range<usize>>,
    pub clk_x: Option<Range<usize>>,
    pub clk_y: Option<Range<usize>>,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let bel = self.chip.get_io_loc(io);
        self.ngrid.get_bel_name(bel).unwrap()
    }
}

mod xc2000;
mod xc3000;
mod xc4000;
mod xc5200;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    match edev.chip.kind {
        ChipKind::Xc2000 => xc2000::name_device(edev, ndb),
        ChipKind::Xc3000 | ChipKind::Xc3000A => xc3000::name_device(edev, ndb),
        ChipKind::Xc4000
        | ChipKind::Xc4000A
        | ChipKind::Xc4000H
        | ChipKind::Xc4000E
        | ChipKind::Xc4000Ex
        | ChipKind::Xc4000Xla
        | ChipKind::Xc4000Xv
        | ChipKind::SpartanXl => xc4000::name_device(edev, ndb),
        ChipKind::Xc5200 => xc5200::name_device(edev, ndb),
    }
}
