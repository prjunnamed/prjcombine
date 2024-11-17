use std::collections::BTreeSet;

use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_virtex2_naming::name_device;
use prjcombine_xilinx_geom::{Bond, Grid};
use prjcombine_xilinx_naming::db::NamingDb;

use crate::db::{make_device, PreDevice};
use prjcombine_virtex2_rd2db::{bond, grid, int_s3, int_v2};
use prjcombine_virtex2_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, String, IntDb, NamingDb) {
    let grid = grid::make_grid(rd);
    let (intdb, ndb) = if rd.family.starts_with("virtex2") {
        int_v2::make_int_db(rd)
    } else {
        int_s3::make_int_db(rd)
    };
    let edev = grid.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pins);
        bonds.push((pkg.clone(), Bond::Virtex2(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    let intdb_name = if grid.kind.is_virtex2() {
        "virtex2"
    } else if grid.kind == GridKind::FpgaCore {
        "fpgacore"
    } else {
        "spartan3"
    };
    (
        make_device(rd, Grid::Virtex2(grid), bonds, BTreeSet::new()),
        intdb_name.into(),
        intdb,
        ndb,
    )
}
