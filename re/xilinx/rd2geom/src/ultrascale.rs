use prjcombine_re_xilinx_geom::{Bond, Chip, DeviceNaming, DisabledPart, Interposer};
use prjcombine_re_xilinx_naming_ultrascale::name_device;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_ultrascale::expand_grid;

use crate::db::{PreDevice, make_device_multi};
use prjcombine_re_xilinx_rd2db_ultrascale::{bond, grid, int_u, int_up};
use prjcombine_re_xilinx_rdverify_ultrascale::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let (grids, interposer, disabled, naming) = grid::make_grids(rd);
    let (intdb, ndb) = if rd.family == "ultrascale" {
        int_u::make_int_db(rd, &naming)
    } else {
        int_up::make_int_db(rd, &naming)
    };
    let grid_refs = grids.map_values(|x| x);
    let edev = expand_grid(&grid_refs, &interposer, &disabled, &intdb);
    let endev = name_device(&edev, &ndb, &naming);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        let bond = bond::make_bond(rd, pkg, &endev, pins);
        bonds.push((pkg.clone(), Bond::Ultrascale(bond)));
    }
    if verify {
        verify_device(&endev, rd);
    }
    let grids = grids.into_map_values(Chip::Ultrascale);
    let disabled = disabled.into_iter().map(DisabledPart::Ultrascale).collect();
    make_device_multi(
        rd,
        grids,
        Interposer::Ultrascale(interposer),
        Default::default(),
        bonds,
        disabled,
        DeviceNaming::Ultrascale(naming),
        rd.family.clone(),
        intdb,
        ndb,
    )
}
