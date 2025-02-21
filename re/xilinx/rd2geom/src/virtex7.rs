use prjcombine_re_xilinx_rawdump::{Part, Source};
use prjcombine_virtex4::expand_grid;
use prjcombine_re_xilinx_naming_virtex4::name_device;
use prjcombine_re_xilinx_geom::{Bond, DeviceNaming, DisabledPart, Grid, Interposer};

use crate::db::{make_device_multi, PreDevice};
use prjcombine_re_xilinx_rd2db_virtex7::{bond, grid, gtz, int};
use prjcombine_re_xilinx_rdverify_virtex7::verify_device;

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let (grids, interposer, disabled) = grid::make_grids(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let gdb = gtz::extract_gtz(rd);
    let grid_refs = grids.map_values(|x| x);
    let mut edev = expand_grid(&grid_refs, Some(&interposer), &disabled, &intdb, &gdb);
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
    make_device_multi(
        rd,
        grids,
        Interposer::Virtex4(interposer),
        gdb,
        bonds,
        disabled,
        DeviceNaming::Dummy,
        "virtex7",
        intdb,
        ndb,
    )
}
