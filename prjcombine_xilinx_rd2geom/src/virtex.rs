use prjcombine_rawdump::Part;
use prjcombine_virtex_naming::name_device;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_virtex_rd2db::{bond, grid, int};
use prjcombine_virtex_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let (grid, disabled) = grid::make_grid(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let mut bonds = Vec::new();
    let edev = grid.expand_grid(&disabled, &intdb);
    let endev = name_device(&edev, &ndb);
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pins);
        bonds.push((pkg.clone(), Bond::Virtex(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    let disabled = disabled.into_iter().map(DisabledPart::Virtex).collect();
    make_device(
        rd,
        Grid::Virtex(grid),
        bonds,
        disabled,
        "virtex",
        intdb,
        ndb,
    )
}
