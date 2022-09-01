use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_ultrascale::expand_grid;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device_multi, PreDevice};
use prjcombine_rdverify::verify;
use prjcombine_ultrascale_rd2db::{bond, grid, int_u, int_up};
use prjcombine_ultrascale_rdverify::verify_bel;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grids, grid_master, disabled) = grid::make_grids(rd);
    let int_db = if rd.family == "ultrascale" {
        int_u::make_int_db(rd)
    } else {
        int_up::make_int_db(rd)
    };
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(rd, pkg, &grids, grid_master, &disabled, pins);
        bonds.push((pkg.clone(), Bond::Ultrascale(bond)));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &disabled, &int_db);
    verify(rd, &eint, |vrf, bel| verify_bel(&grids, vrf, bel));
    let grids = grids.into_map_values(Grid::Ultrascale);
    let disabled = disabled.into_iter().map(DisabledPart::Ultrascale).collect();
    (
        make_device_multi(rd, grids, grid_master, Vec::new(), bonds, disabled),
        Some(int_db),
    )
}
