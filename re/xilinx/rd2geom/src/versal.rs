use prjcombine_re_xilinx_geom::{Bond, Chip, DeviceNaming, DisabledPart, Interposer};
use prjcombine_re_xilinx_naming_versal::name_device;
use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_versal::expand::expand_grid;

use crate::db::{PreDevice, make_device_multi};
use prjcombine_re_xilinx_rd2db_versal::{grid, int};
use prjcombine_re_xilinx_rdverify_versal::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let (grids, interposer, disabled, naming) = grid::make_chips(rd);
    let (intdb, ndb) = int::make_int_db(rd, &naming);
    let mut bonds = Vec::new();
    for (pkg, _) in rd.packages.iter() {
        bonds.push((pkg.clone(), Bond::Versal(prjcombine_versal::bond::Bond {})));
    }
    let grid_refs = grids.map_values(|x| x);
    let edev = expand_grid(&grid_refs, &interposer, &disabled, &intdb);
    let endev = name_device(&edev, &ndb, &naming);
    if verify {
        verify_device(&endev, rd);
    }
    let grids = grids.into_map_values(Chip::Versal);
    let disabled = disabled.into_iter().map(DisabledPart::Versal).collect();
    make_device_multi(
        rd,
        grids,
        Interposer::Versal(interposer),
        Default::default(),
        bonds,
        disabled,
        DeviceNaming::Versal(naming),
        "versal",
        bincode::decode_from_slice(prjcombine_versal::defs::INIT, bincode::config::standard())
            .unwrap()
            .0,
        intdb,
        ndb,
    )
}
