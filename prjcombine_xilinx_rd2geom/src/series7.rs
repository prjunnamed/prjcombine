use prjcombine_int::db::IntDb;
use prjcombine_rawdump::{Part, Source};
use prjcombine_series7::expand_grid;
use prjcombine_xilinx_geom::{Bond, DeviceNaming, DisabledPart, ExtraDie, Grid};

use crate::db::{make_device_multi, PreDevice};
use prjcombine_series7_rd2db::{bond, grid, int};
use prjcombine_series7_rdverify::verify_device;

pub fn ingest(rd: &Part) -> (PreDevice, Option<IntDb>) {
    let (grids, grid_master, extras, disabled) = grid::make_grids(rd);
    let int_db = int::make_int_db(rd);
    let grid_refs = grids.map_values(|x| x);
    let mut edev = expand_grid(&grid_refs, grid_master, &extras, &disabled, &int_db);
    if rd.source == Source::Vivado {
        edev.adjust_vivado();
    }
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(rd, pkg, &edev, pins);
        bonds.push((pkg.clone(), Bond::Virtex4(bond)));
    }
    verify_device(&edev, rd);
    let grids = grids.into_map_values(Grid::Virtex4);
    let extras = extras.into_iter().map(ExtraDie::Virtex4).collect();
    let disabled = disabled.into_iter().map(DisabledPart::Virtex4).collect();
    (
        make_device_multi(
            rd,
            grids,
            grid_master,
            extras,
            bonds,
            disabled,
            DeviceNaming::Dummy,
        ),
        Some(int_db),
    )
}
