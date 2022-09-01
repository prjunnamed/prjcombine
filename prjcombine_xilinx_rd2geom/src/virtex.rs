use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_virtex_rd2db::{bond, grid, int};
use prjcombine_virtex_rdverify::verify_bel;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let int_db = int::make_int_db(rd);
    let edev = grid.expand_grid(&disabled, &int_db);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pins);
        bonds.push((pkg.clone(), Bond::Virtex(bond)));
    }
    verify(rd, &edev.egrid, |vrf, ctx| verify_bel(&edev, vrf, ctx));
    let disabled = disabled.into_iter().map(DisabledPart::Virtex).collect();
    (
        make_device(rd, Grid::Virtex(grid), bonds, disabled),
        Some(int_db),
    )
}
