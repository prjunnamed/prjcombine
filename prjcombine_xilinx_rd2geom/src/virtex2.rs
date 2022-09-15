use std::collections::BTreeSet;

use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::{Bond, Grid};

use crate::db::{make_device, PreDevice};
use prjcombine_virtex2_rd2db::{bond, grid, int_s3, int_v2};
use prjcombine_virtex2_rdverify::verify_device;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let grid = grid::make_grid(rd);
    let int_db = if rd.family.starts_with("virtex2") {
        int_v2::make_int_db(rd)
    } else {
        int_s3::make_int_db(rd)
    };
    let edev = grid.expand_grid(&int_db);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(&edev, pins);
        bonds.push((pkg.clone(), Bond::Virtex2(bond)));
    }

    verify_device(&edev, rd);
    (
        make_device(rd, Grid::Virtex2(grid), bonds, BTreeSet::new()),
        Some(int_db),
    )
}
