use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_spartan6_naming::name_device;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};
use prjcombine_xilinx_naming::db::NamingDb;

use crate::db::{make_device, PreDevice};
use prjcombine_spartan6_rd2db::{bond, grid, int};
use prjcombine_spartan6_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, String, IntDb, NamingDb) {
    let (grid, disabled) = grid::make_grid(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let edev = grid.expand_grid(&intdb, &disabled);
    let endev = name_device(&edev, &ndb);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pins);
        bonds.push((pkg.clone(), Bond::Spartan6(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    let disabled = disabled.into_iter().map(DisabledPart::Spartan6).collect();
    (
        make_device(rd, Grid::Spartan6(grid), bonds, disabled),
        "spartan6".into(),
        intdb,
        ndb,
    )
}
