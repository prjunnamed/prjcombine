use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_virtex4_naming::name_device;
use prjcombine_xilinx_geom::{Bond, Grid};
use prjcombine_xilinx_naming::db::NamingDb;
use std::collections::BTreeSet;
use unnamed_entity::EntityVec;

use crate::db::{make_device, PreDevice};
use prjcombine_virtex4::expand_grid;
use prjcombine_virtex5_rd2db::{bond, grid, int};
use prjcombine_virtex5_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, String, IntDb, NamingDb) {
    let grid = grid::make_grid(rd);
    let grid_refs: EntityVec<_, _> = [&grid].into_iter().collect();
    let disabled = Default::default();
    let (intdb, ndb) = int::make_int_db(rd);
    let edev = expand_grid(&grid_refs, None, &disabled, &intdb);
    let endev = name_device(&edev, &ndb);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    (
        make_device(rd, Grid::Virtex4(grid), bonds, BTreeSet::new()),
        "virtex5".into(),
        intdb,
        ndb,
    )
}
