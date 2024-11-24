use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xc2000_naming::name_device;
use prjcombine_xilinx_geom::{Bond, Grid};
use prjcombine_xilinx_naming::db::NamingDb;
use std::collections::BTreeSet;

use crate::db::{make_device, PreDevice};
use prjcombine_xc4000_rd2db::{bond, grid, int};
use prjcombine_xc4000_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, String, IntDb, NamingDb) {
    let mut grid = grid::make_grid(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let mut cfg_io = core::mem::take(&mut grid.cfg_io);
    let edev = grid.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pkg, pins, &mut cfg_io);
        bonds.push((pkg.clone(), Bond::Xc2000(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    grid.cfg_io = cfg_io;
    (
        make_device(rd, Grid::Xc4000(grid), bonds, BTreeSet::new()),
        rd.family.to_string(),
        intdb,
        ndb,
    )
}
