use crate::verify::Verifier;
use prjcombine_xilinx_geom::int::IntDb;
use prjcombine_xilinx_geom::series7::expand_grid;
use prjcombine_xilinx_geom::Grid;
use prjcombine_xilinx_rawdump::Part;
use std::collections::BTreeSet;

use crate::db::{make_device_multi, PreDevice};

mod bond;
mod grid;
mod int;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grids, grid_master, extras) = grid::make_grids(rd);
    let int_db = int::make_int_db(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            bond::make_bond(rd, pkg, &grids, grid_master, &extras, pins),
        ));
    }
    let grid_refs = grids.map_values(|x| x);
    let eint = expand_grid(&grid_refs, grid_master, &extras, &int_db);
    let vrf = Verifier::new(rd, &eint);
    vrf.finish();
    let grids = grids.into_map_values(Grid::Series7);
    (
        make_device_multi(rd, grids, grid_master, extras, bonds, BTreeSet::new()),
        Some(int_db),
    )
}
