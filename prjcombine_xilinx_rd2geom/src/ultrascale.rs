use crate::verify::Verifier;
use prjcombine_xilinx_geom::int::IntDb;
use prjcombine_xilinx_geom::ultrascale::expand_grid;
use prjcombine_xilinx_geom::Grid;
use prjcombine_xilinx_rawdump::Part;

use crate::db::{make_device_multi, PreDevice};

mod bond;
mod grid;
mod int_u;
mod int_up;

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
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    let grids = grids.into_map_values(Grid::Ultrascale);
    (
        make_device_multi(rd, grids, grid_master, Vec::new(), bonds, disabled),
        Some(int_db),
    )
}
