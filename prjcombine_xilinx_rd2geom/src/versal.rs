use prjcombine_int::db::IntDb;
use prjcombine_rawdump::Part;
use prjcombine_versal::expand::expand_grid;
use prjcombine_xilinx_geom::{Bond, DeviceNaming, DisabledPart, Grid};

use crate::db::{make_device_multi, PreDevice};
use prjcombine_versal_rd2db::{grid, int};
use prjcombine_versal_rdverify::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> (PreDevice, Option<IntDb>) {
    let (grids, grid_master, disabled, naming) = grid::make_grids(rd);
    let int_db = int::make_int_db(rd, &naming);
    let mut bonds = Vec::new();
    for (pkg, _) in rd.packages.iter() {
        bonds.push((pkg.clone(), Bond::Versal(prjcombine_versal::bond::Bond {})));
    }
    let grid_refs = grids.map_values(|x| x);
    let edev = expand_grid(&grid_refs, &disabled, &naming, &int_db);
    if verify {
        verify_device(&edev, rd);
    }
    let grids = grids.into_map_values(Grid::Versal);
    let disabled = disabled.into_iter().map(DisabledPart::Versal).collect();
    (
        make_device_multi(
            rd,
            grids,
            grid_master,
            Vec::new(),
            bonds,
            disabled,
            DeviceNaming::Versal(naming),
        ),
        Some(int_db),
    )
}
