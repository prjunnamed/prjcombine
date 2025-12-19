use prjcombine_entity::EntityVec;
use prjcombine_re_xilinx_geom::{Bond, Chip, DisabledPart};
use prjcombine_re_xilinx_naming_virtex4::name_device;
use prjcombine_re_xilinx_rawdump::Part;

use crate::db::{PreDevice, make_device};
use prjcombine_re_xilinx_rd2db_virtex6::{bond, grid, int};
use prjcombine_re_xilinx_rdverify_virtex6::verify_device;
use prjcombine_virtex4::{expand_grid, gtz::GtzDb};

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let (grid, disabled) = grid::make_grid(rd);
    let grid_refs: EntityVec<_, _> = [&grid].into_iter().collect();
    let (intdb, ndb) = int::make_int_db(rd);
    let gdb = GtzDb::default();
    let edev = expand_grid(&grid_refs, None, &disabled, &intdb, &gdb);
    let endev = name_device(&edev, &ndb);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&endev, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    let disabled = disabled.into_iter().map(DisabledPart::Virtex4).collect();
    make_device(
        rd,
        Chip::Virtex4(grid),
        bonds,
        disabled,
        "virtex6",
        intdb,
        ndb,
    )
}
