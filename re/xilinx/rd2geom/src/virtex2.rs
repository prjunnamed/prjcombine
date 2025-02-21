use std::collections::BTreeSet;

use prjcombine_re_xilinx_geom::{Bond, Grid};
use prjcombine_re_xilinx_naming_virtex2::name_device;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_virtex2::chip::ChipKind;

use crate::db::{PreDevice, make_device};
use prjcombine_re_xilinx_rd2db_virtex2::{bond, grid, int_s3, int_v2};
use prjcombine_re_xilinx_rdverify_virtex2::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
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
    } else if grid.kind == ChipKind::FpgaCore {
        "fpgacore"
    } else {
        "spartan3"
    };
    make_device(
        rd,
        Grid::Virtex2(grid),
        bonds,
        BTreeSet::new(),
        intdb_name,
        intdb,
        ndb,
    )
}
