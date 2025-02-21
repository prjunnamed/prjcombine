use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_naming_virtex4::name_device;
use prjcombine_re_xilinx_geom::{Bond, Grid};
use std::collections::BTreeSet;
use unnamed_entity::EntityVec;

use crate::db::{make_device, PreDevice};
use prjcombine_virtex4::{expand_grid, gtz::GtzDb};
use prjcombine_re_xilinx_rd2db_virtex4::{bond, grid, int};
use prjcombine_re_xilinx_rdverify_virtex4::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let grid = grid::make_grid(rd);
    let grid_refs: EntityVec<_, _> = [&grid].into_iter().collect();
    let disabled = Default::default();
    let (intdb, ndb) = int::make_int_db(rd);
    let gdb = GtzDb::default();
    let edev = expand_grid(&grid_refs, None, &disabled, &intdb, &gdb);
    let endev = name_device(&edev, &ndb);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    make_device(
        rd,
        Grid::Virtex4(grid),
        bonds,
        BTreeSet::new(),
        "virtex4",
        intdb,
        ndb,
    )
}
