use prjcombine_entity::EntityVec;
use prjcombine_rawdump::{Part, PkgPin};
use prjcombine_xilinx_geom::pkg::Bond;
use prjcombine_xilinx_geom::versal::Grid;
use prjcombine_xilinx_geom::{DisabledPart, SlrId};
use std::collections::{BTreeMap, BTreeSet};

pub fn make_bond(
    _rd: &Part,
    _pkg: &str,
    _grids: &EntityVec<SlrId, Grid>,
    _grid_master: SlrId,
    _disabled: &BTreeSet<DisabledPart>,
    _pins: &[PkgPin],
) -> Bond {
    let bond_pins = BTreeMap::new();
    Bond {
        pins: bond_pins,
        io_banks: Default::default(),
    }
}
