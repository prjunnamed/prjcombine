use prjcombine_interconnect::grid::{DieId, EdgeIoCoord, LayerId};
use prjcombine_re_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    expanded::ExpandedDevice,
};
use unnamed_entity::EntityId;

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub chip: &'a Chip,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let die = self.edev.egrid.die(DieId::from_idx(0));
        let (col, row, bel) = self.chip.get_io_loc(io);
        let nnode = &self.ngrid.nodes[&(die.die, col, row, LayerId::from_idx(0))];
        &nnode.bels[bel]
    }
}

mod xc4000;
mod xc5200;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    match edev.chip.kind {
        ChipKind::Xc2000 | ChipKind::Xc3000 | ChipKind::Xc3000A => unreachable!(),
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
