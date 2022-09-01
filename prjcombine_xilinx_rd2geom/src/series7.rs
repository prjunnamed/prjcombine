use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_series7::expand_grid;
use prjcombine_xilinx_geom::{Bond, DisabledPart, ExtraDie, Grid};

use crate::db::{make_device_multi, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_series7_rd2db::{bond, grid, int};
use prjcombine_series7_rdverify::verify_bel;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grids, grid_master, extras, disabled) = grid::make_grids(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(rd, pkg, &grids, grid_master, &extras, pins);
        bonds.push((pkg.clone(), Bond::Series7(bond)));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &extras, &disabled, &int_db);
    verify(rd, &eint, |vrf, bel| verify_bel(&grids, vrf, bel));
    let grids = grids.into_map_values(Grid::Series7);
    let extras = extras.into_iter().map(ExtraDie::Series7).collect();
    let disabled = disabled.into_iter().map(DisabledPart::Series7).collect();
    (
        make_device_multi(rd, grids, grid_master, extras, bonds, disabled),
        Some(int_db),
    )
}
