use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_spartan6_rd2db::{bond, grid, int};
use prjcombine_spartan6_rdverify::{verify_bel, verify_extra};

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&grid, &disabled, pins);
        bonds.push((pkg.clone(), Bond::Spartan6(bond)));
    }
    let edev = grid.expand_grid(&int_db, &disabled);
    verify(
        rd,
        &edev.egrid,
        |vrf, bel| verify_bel(&edev, vrf, bel),
        |vrf| verify_extra(&edev, vrf),
    );
    let disabled = disabled.into_iter().map(DisabledPart::Spartan6).collect();
    (
        make_device(rd, Grid::Spartan6(grid), bonds, disabled),
        Some(int_db),
    )
}
