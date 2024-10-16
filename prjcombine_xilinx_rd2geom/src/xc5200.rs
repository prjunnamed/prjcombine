use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, Grid};
use std::collections::{BTreeMap, BTreeSet};

use crate::db::{make_device, PreDevice};
use prjcombine_xc5200_rd2db::{bond, grid, int};
use prjcombine_xc5200_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, Option<IntDb>) {
    let mut grid = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&int_db);
    let mut cfg_io = BTreeMap::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pkg, pins, &mut cfg_io);
        bonds.push((pkg.clone(), Bond::Xc5200(bond)));
    }
    if verify {
        verify_device(&edev, rd);
    }
    grid.cfg_io = cfg_io;
    (
        make_device(rd, Grid::Xc5200(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
