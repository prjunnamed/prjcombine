use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::int::IntDb;
use prjcombine_xilinx_geom::ultrascale::expand_grid;
use prjcombine_xilinx_geom::Grid;

use crate::db::{make_device_multi, PreDevice};
use crate::verify::verify;

mod bond;
mod grid;
mod int_u;
mod int_up;
mod verify;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grids, grid_master, disabled) = grid::make_grids(rd);
    let int_db = if rd.family == "ultrascale" {
        int_u::make_int_db(rd)
    } else {
        int_up::make_int_db(rd)
    };
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            bond::make_bond(rd, pkg, &grids, grid_master, &disabled, pins),
        ));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &disabled, &int_db);
    verify(rd, &eint, |vrf, bel| verify::verify_bel(&grids, vrf, bel));
    let grids = grids.into_map_values(Grid::Ultrascale);
    (
        make_device_multi(rd, grids, grid_master, Vec::new(), bonds, disabled),
        Some(int_db),
    )
}
