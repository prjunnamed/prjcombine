use prjcombine_int::db::IntDb;
use prjcombine_rawdump::{Part, Source};
use prjcombine_virtex4::expand_grid;
use prjcombine_virtex4_naming::name_device;
use prjcombine_xilinx_geom::{Bond, DeviceNaming, DisabledPart, Grid, Interposer};
use prjcombine_xilinx_naming::db::NamingDb;

use crate::db::{make_device_multi, PreDevice};
use prjcombine_virtex7_rd2db::{bond, grid, int};
use prjcombine_virtex7_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, String, IntDb, NamingDb) {
    let (grids, interposer, disabled) = grid::make_grids(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let grid_refs = grids.map_values(|x| x);
    let mut edev = expand_grid(
        &grid_refs,
        Some(&interposer),
        &disabled,
        &intdb,
    );
    if rd.source == Source::Vivado {
        edev.adjust_vivado();
    }
    let endev = name_device(&edev, &ndb);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(rd, pkg, &endev, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    let grids = grids.into_map_values(Grid::Virtex4);
    let disabled = disabled.into_iter().map(DisabledPart::Virtex4).collect();
    (
        make_device_multi(
            rd,
            grids,
            Interposer::Virtex4(interposer),
            bonds,
            disabled,
            DeviceNaming::Dummy,
        ),
        "virtex7".into(),
        intdb,
        ndb,
    )
}
