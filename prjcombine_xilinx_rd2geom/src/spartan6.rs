use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_spartan6_rd2db::{bond, grid, int};
use prjcombine_spartan6_rdverify::verify_device;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let edev = grid.expand_grid(&int_db, &disabled);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pins);
        bonds.push((pkg.clone(), Bond::Spartan6(bond)));
    }
    verify_device(&edev, rd);
    let disabled = disabled.into_iter().map(DisabledPart::Spartan6).collect();
    (
        make_device(rd, Grid::Spartan6(grid), bonds, disabled),
        Some(int_db),
    )
}
