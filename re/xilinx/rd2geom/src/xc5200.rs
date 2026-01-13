use prjcombine_interconnect::db::IntDb;
use prjcombine_re_xilinx_geom::{Bond, Chip};
use prjcombine_re_xilinx_naming_xc2000::name_device;
use prjcombine_re_xilinx_rawdump::Part;
use std::collections::{BTreeMap, BTreeSet};

use crate::db::{PreDevice, make_device};
use prjcombine_re_xilinx_rd2db_xc5200::{bond, grid, int};
use prjcombine_re_xilinx_rdverify_xc5200::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let mut grid = grid::make_grid(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);
    let mut cfg_io = BTreeMap::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pkg, pins, &mut cfg_io);
        bonds.push((pkg.clone(), Bond::Xc2000(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    grid.cfg_io = cfg_io;
    make_device(
        rd,
        Chip::Xc2000(grid),
        bonds,
        BTreeSet::new(),
        "xc5200",
        IntDb::default(),
        intdb,
        ndb,
    )
}
