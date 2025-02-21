use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_re_xilinx_naming_spartan6::name_device;
use prjcombine_re_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_re_xilinx_rd2db_spartan6::{bond, grid, int};
use prjcombine_re_xilinx_rdverify_spartan6::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
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

    make_device(
        rd,
        Grid::Spartan6(grid),
        bonds,
        disabled,
        "spartan6",
        intdb,
        ndb,
    )
}
