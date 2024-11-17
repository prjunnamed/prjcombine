use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xc5200_naming::name_device;
use prjcombine_xilinx_geom::{Bond, Grid};
use prjcombine_xilinx_naming::db::NamingDb;
use std::collections::{BTreeMap, BTreeSet};

use crate::db::{make_device, PreDevice};
use prjcombine_xc5200_rd2db::{bond, grid, int};
use prjcombine_xc5200_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, String, IntDb, NamingDb) {
    let mut grid = grid::make_grid(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);
    let mut cfg_io = BTreeMap::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pkg, pins, &mut cfg_io);
        bonds.push((pkg.clone(), Bond::Xc5200(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    grid.cfg_io = cfg_io;
    (
        make_device(rd, Grid::Xc5200(grid), bonds, BTreeSet::new()),
        "xc5200".into(),
        intdb,
        ndb,
    )
}
