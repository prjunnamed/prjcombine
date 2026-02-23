use std::collections::HashSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{DieId, DieIdExt};
use prjcombine_re_xilinx_geom::{Bond, Chip, DeviceNaming, DisabledPart, Interposer};
use prjcombine_re_xilinx_naming_virtex4::name_device;
use prjcombine_re_xilinx_rawdump::{Part, Source};
use prjcombine_virtex4::{
    chip::ChipKind,
    defs::{self, tslots},
    expand_grid,
    expanded::ExpandedDevice,
};

use crate::db::{PreDevice, make_device_multi};
use prjcombine_re_xilinx_rd2db_virtex7::{bond, grid, gtz, int};
use prjcombine_re_xilinx_rdverify_virtex7::verify_device;

fn adjust_vivado(edev: &mut ExpandedDevice) {
    assert_eq!(edev.kind, ChipKind::Virtex7);
    let lvb6 = edev.db.wires.get("LVB.6").unwrap().0;
    let mut cursed_wires = HashSet::new();
    for i in 1..edev.chips.len() {
        let die_s = DieId::from_idx(i - 1);
        let die_n = DieId::from_idx(i);
        for col in edev.cols(die_s) {
            let row_s = edev.rows(die_s).last().unwrap() - 49;
            let row_n = edev.rows(die_n).first().unwrap() + 1;
            let cell_s = die_s.cell(col, row_s);
            let cell_n = die_n.cell(col, row_n);
            if edev[cell_s].tiles.contains_id(tslots::INT)
                && edev[cell_n].tiles.contains_id(tslots::INT)
            {
                cursed_wires.insert(cell_s.wire(lvb6));
            }
        }
    }
    edev.egrid.blackhole_wires.extend(cursed_wires);
}

pub fn ingest(rd: &Part, verify: bool) -> PreDevice {
    let (grids, interposer, disabled) = grid::make_grids(rd);
    let (intdb, ndb) = int::make_int_db(rd);
    let gdb = gtz::extract_gtz(rd);
    let grid_refs = grids.map_values(|x| x);
    let mut edev = expand_grid(&grid_refs, Some(&interposer), &disabled, &intdb, &gdb);
    if rd.source == Source::Vivado {
        adjust_vivado(&mut edev);
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
    let grids = grids.into_map_values(Chip::Virtex4);
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
        bincode::decode_from_slice(defs::virtex7::INIT, bincode::config::standard())
            .unwrap()
            .0,
        intdb,
        ndb,
    )
}
