use prjcombine_interconnect::grid::{DieId, EdgeIoCoord, LayerId};
use prjcombine_xc2000::{
    expanded::ExpandedDevice,
    grid::{Grid, GridKind},
};
use prjcombine_xilinx_naming::{db::NamingDb, grid::ExpandedGridNaming};
use unnamed_entity::EntityId;

pub struct ExpandedNamedDevice<'a> {
    pub edev: &'a ExpandedDevice<'a>,
    pub ngrid: ExpandedGridNaming<'a>,
    pub grid: &'a Grid,
}

impl<'a> ExpandedNamedDevice<'a> {
    pub fn get_io_name(&'a self, io: EdgeIoCoord) -> &'a str {
        let die = self.edev.egrid.die(DieId::from_idx(0));
        let (col, row, bel) = self.grid.get_io_loc(io);
        let nnode = &self.ngrid.nodes[&(die.die, col, row, LayerId::from_idx(0))];
        &nnode.bels[bel]
    }
}

mod xc4000;
mod xc5200;

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    match edev.grid.kind {
        GridKind::Xc2000 | GridKind::Xc3000 | GridKind::Xc3000A => unreachable!(),
        GridKind::Xc4000
        | GridKind::Xc4000A
        | GridKind::Xc4000H
        | GridKind::Xc4000E
        | GridKind::Xc4000Ex
        | GridKind::Xc4000Xla
        | GridKind::Xc4000Xv
        | GridKind::SpartanXl => xc4000::name_device(edev, ndb),
        GridKind::Xc5200 => xc5200::name_device(edev, ndb),
    }
}
