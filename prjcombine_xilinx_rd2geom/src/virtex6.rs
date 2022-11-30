use prjcombine_entity::EntityVec;
use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, DisabledPart, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_virtex6::expand_grid;
use prjcombine_virtex6_rd2db::{bond, grid, int};
use prjcombine_virtex6_rdverify::verify;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grid, disabled) = grid::make_grid(rd);
    let grid_refs: EntityVec<_, _> = [&grid].into_iter().collect();
    let grid_master = grid_refs.first_id().unwrap();
    let extras = [];
    let int_db = int::make_int_db(rd);
    let edev = expand_grid(&grid_refs, grid_master, &extras, &disabled, &int_db);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    verify(&edev, rd);
    let disabled = disabled.into_iter().map(DisabledPart::Virtex4).collect();
    (
        make_device(rd, Grid::Virtex4(grid), bonds, disabled),
        Some(int_db),
    )
}
