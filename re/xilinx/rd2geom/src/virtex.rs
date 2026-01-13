use prjcombine_interconnect::db::IntDb;
use prjcombine_re_xilinx_geom::{Bond, Chip, DisabledPart};
use prjcombine_re_xilinx_naming_virtex::name_device;
use prjcombine_re_xilinx_rawdump::Part;

use crate::db::{PreDevice, make_device};
use prjcombine_re_xilinx_rd2db_virtex::{bond, grid, int};
use prjcombine_re_xilinx_rdverify_virtex::verify_device;

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
        Chip::Virtex(grid),
        bonds,
        disabled,
        "virtex",
        IntDb::default(),
        intdb,
        ndb,
    )
}
