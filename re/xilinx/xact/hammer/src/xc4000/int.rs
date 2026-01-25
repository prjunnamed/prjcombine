use std::collections::{BTreeMap, HashSet, btree_map};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileClassId, TileWireCoord},
    grid::{BelCoord, TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{
    backend::FuzzerProp,
    diff::{Diff, DiffKey, OcdMode, xlat_bit_raw, xlat_enum_attr, xlat_enum_raw},
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_types::bsdata::{BitRectId, TileBit};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    xc4000::{
        bslots, enums, tslots, wires,
        xc4000::{bcls, tcls},
    },
};

use crate::{
    backend::{Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    props::DynProp,
    specials,
};

fn apply_int_pip<'a>(
    backend: &XactBackend<'a>,
    tcrd: TileCoord,
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
    block: &'a str,
    pin: &'static str,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
) -> Fuzzer<XactBackend<'a>> {
    let rwf = backend.edev.resolve_tile_wire(tcrd, wire_from).unwrap();
    let rwt = backend.edev.resolve_tile_wire(tcrd, wire_to).unwrap();
    fuzzer = fuzzer.base(Key::WireMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(tcrd, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
}

fn wire_to_outpin(chip: &Chip, wire: WireCoord) -> Option<(BelCoord, &'static str)> {
    match wire.slot {
        wires::OUT_CLB_X => Some((wire.bel(bslots::CLB), "X")),
        wires::OUT_CLB_Y => Some((wire.bel(bslots::CLB), "Y")),
        wires::OUT_CLB_XQ => Some((wire.bel(bslots::CLB), "XQ")),
        wires::OUT_CLB_YQ => Some((wire.bel(bslots::CLB), "YQ")),
        wires::OUT_MD0_I => Some((wire.bel(bslots::MD0), "I")),
        wires::OUT_STARTUP_DONEIN => Some((wire.bel(bslots::STARTUP), "DONEIN")),
        wires::OUT_STARTUP_Q1Q4 => Some((wire.bel(bslots::STARTUP), "Q1Q4")),
        wires::OUT_STARTUP_Q2 => Some((wire.bel(bslots::STARTUP), "Q2")),
        wires::OUT_STARTUP_Q3 => Some((wire.bel(bslots::STARTUP), "Q3")),
        wires::OUT_RDBK_DATA => Some((wire.bel(bslots::RDBK), "DATA")),
        wires::OUT_UPDATE_O => Some((wire.bel(bslots::UPDATE), "O")),
        wires::OUT_OSC_MUX1 => Some((wire.bel(bslots::OSC), "OUT1")),
        wires::OUT_IO_CLKIN => {
            let slot = if chip.kind == ChipKind::Xc4000H {
                if wire.col == chip.col_w() {
                    if wire.row < chip.row_mid() {
                        bslots::HIO[3]
                    } else {
                        bslots::HIO[0]
                    }
                } else if wire.col == chip.col_e() {
                    if wire.row < chip.row_mid() {
                        bslots::HIO[2]
                    } else {
                        bslots::HIO[0]
                    }
                } else if wire.row == chip.row_s() {
                    if wire.col < chip.col_mid() {
                        bslots::HIO[0]
                    } else {
                        bslots::HIO[3]
                    }
                } else if wire.row == chip.row_n() {
                    if wire.col < chip.col_mid() {
                        bslots::HIO[0]
                    } else {
                        bslots::HIO[2]
                    }
                } else {
                    unreachable!()
                }
            } else {
                if wire.col == chip.col_w() {
                    if wire.row < chip.row_mid() {
                        bslots::IO[1]
                    } else {
                        bslots::IO[0]
                    }
                } else if wire.col == chip.col_e() {
                    bslots::IO[0]
                } else if wire.row == chip.row_s() {
                    if wire.col < chip.col_mid() {
                        bslots::IO[0]
                    } else {
                        bslots::IO[1]
                    }
                } else if wire.row == chip.row_n() {
                    bslots::IO[0]
                } else {
                    unreachable!()
                }
            };
            Some((wire.bel(slot), "CLKIN"))
        }
        _ => {
            if let Some(idx) = wires::BUFGLS.index_of(wire.slot) {
                Some((chip.bel_bufg(idx), "O"))
            } else if let Some(idx) = wires::OUT_IO_SN_I1.index_of(wire.slot) {
                if wire.col == chip.col_w() && wire.row == chip.row_s() && idx == 1 {
                    Some((wire.bel(bslots::MD2), "I"))
                } else if wire.col == chip.col_w() && wire.row == chip.row_n() && idx == 1 {
                    Some((wire.bel(bslots::BSCAN), "SEL2"))
                } else {
                    Some((wire.bel(bslots::IO[idx]), "I1"))
                }
            } else if let Some(idx) = wires::OUT_IO_SN_I2.index_of(wire.slot) {
                if wire.col == chip.col_w() && wire.row == chip.row_s() && idx == 1 {
                    Some((wire.bel(bslots::RDBK), "RIP"))
                } else if wire.col == chip.col_w() && wire.row == chip.row_n() && idx == 1 {
                    Some((wire.bel(bslots::BSCAN), "DRCK"))
                } else {
                    Some((wire.bel(bslots::IO[idx]), "I2"))
                }
            } else if let Some(idx) = wires::OUT_IO_WE_I1.index_of(wire.slot) {
                if wire.col == chip.col_w() && wire.row == chip.row_n() && idx == 1 {
                    Some((wire.bel(bslots::BSCAN), "SEL1"))
                } else if wire.col == chip.col_e() && wire.row == chip.row_n() && idx == 1 {
                    Some((wire.bel(bslots::OSC), "F8M"))
                } else {
                    Some((wire.bel(bslots::IO[idx]), "I1"))
                }
            } else if let Some(idx) = wires::OUT_IO_WE_I2.index_of(wire.slot) {
                if wire.col == chip.col_w() && wire.row == chip.row_n() && idx == 1 {
                    Some((wire.bel(bslots::BSCAN), "IDLE"))
                } else if wire.col == chip.col_e() && wire.row == chip.row_n() && idx == 1 {
                    Some((wire.bel(bslots::OSC), "OUT0"))
                } else {
                    Some((wire.bel(bslots::IO[idx]), "I2"))
                }
            } else if let Some(idx) = wires::OUT_HIO_I.index_of(wire.slot) {
                Some((wire.bel(bslots::HIO[idx]), "I"))
            } else {
                None
            }
        }
    }
}

fn drive_wire<'a>(
    backend: &XactBackend<'a>,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: WireCoord,
    wire_avoid: WireCoord,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let chip = backend.edev.chip;
    let mut cell = wire_target.cell;
    let wt = wire_target.slot;
    let wtn = &backend.edev.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if let Some((bel, mut pin)) = wire_to_outpin(chip, wire_target) {
        cell = bel.cell;
        if let Some(idx) = bslots::IO.index_of(bel.slot)
            && chip.kind == ChipKind::Xc4000H
        {
            (
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wire_target.slot),
                TileWireCoord::new_idx(0, wires::OUT_HIO_I[idx * 2]),
            )
        } else {
            let tcrd = cell.tile(tslots::MAIN);
            let ntile = &backend.ngrid.tiles[&tcrd];
            let mut block = &ntile.bels[bel.slot][0];
            if pin == "CLKIN" {
                block = &ntile.bels[bel.slot][1];
                pin = "I";
            }
            if bel.slot == bslots::OSC {
                let crd0 = backend.ngrid.bel_pip(bel, "OUT0_F500K");
                let crd1 = backend.ngrid.bel_pip(bel, "OUT1_F500K");
                fuzzer = fuzzer
                    .base(Key::BelMutex(bel, "MODE".into()), "USE")
                    .base(Key::Pip(crd0), Value::FromPin(block, "F500K".into()))
                    .base(Key::Pip(crd1), Value::FromPin(block, "F500K".into()));
                if pin == "OUT0" || pin == "OUT1" {
                    pin = "F500K";
                }
            }
            return (
                fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
                block,
                pin,
            );
        }
    } else if wire_target.slot == wires::TIE_0 {
        let tcrd = if wires::GCLK.contains(wire_avoid.slot) {
            cell.with_row(chip.row_mid()).tile(tslots::LLV)
        } else {
            cell.tile(tslots::MAIN)
        };
        let ntile = &backend.ngrid.tiles[&tcrd];
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            &ntile.tie_names[0],
            "O",
        );
    } else if let Some(idx) = wires::GCLK.index_of(wire_target.slot) {
        let tcrd = wire_target.with_row(chip.row_mid()).tile(tslots::LLV);
        (
            tcrd,
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, wires::BUFGLS[idx * 2]),
        )
    } else if wires::DEC_H.contains(wire_target.slot) || wires::DEC_V.contains(wire_target.slot) {
        let idx = if let Some(idx) = wires::DEC_H.index_of(wire_target.slot) {
            if cell.col == chip.col_w() {
                cell.col += 1;
            } else if cell.col == chip.col_e() {
                cell.col -= 1;
            }
            idx
        } else {
            if cell.row == chip.row_s() {
                cell.row += 1;
            } else if cell.row == chip.row_n() {
                cell.row -= 1;
            }
            wires::DEC_V.index_of(wire_target.slot).unwrap()
        };
        let pin = ["O1", "O2", "O3", "O4"][idx];
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let bel = cell.bel(bslots::DEC[0]);
        let block = &ntile.bels[bslots::DEC[0]][0];
        let crd = backend.ngrid.bel_pip(bel, pin);
        fuzzer = fuzzer
            .base(Key::Pip(crd), Value::FromPin(block, pin.into()))
            .base(Key::WireMutex(wire_target), "SHARED_ROOT");
        return (fuzzer, block, pin);
    } else if (wire_target.slot == wires::LONG_H[2] || wire_target.slot == wires::LONG_H[3])
        && !(cell.row == chip.row_s() || cell.row == chip.row_n())
    {
        let slot = if wire_target.slot == wires::LONG_H[2] {
            bslots::TBUF[0]
        } else {
            bslots::TBUF[1]
        };
        let tcrd = cell.tile(tslots::MAIN);
        let bel = cell.bel(slot);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[slot][0];
        let crd = backend.ngrid.bel_pip(bel, "O");
        fuzzer = fuzzer
            .base(Key::Pip(crd), Value::FromPin(block, "O".into()))
            .base(Key::WireMutex(wire_target), "SHARED_ROOT");
        return (fuzzer, block, "O");
    } else if wtn.starts_with("SINGLE")
        || (wtn.starts_with("DOUBLE") && !wtn.starts_with("DOUBLE_IO"))
    {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("OUT") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out?")
        }
    } else if wtn.starts_with("LONG") || wtn.starts_with("DOUBLE_IO") {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("SINGLE") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out?")
        }
    } else if wtn.starts_with("DBUF_IO") {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("DOUBLE_IO") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out?")
        }
    } else {
        panic!("ummm {wtn}?");
    };
    let nwt = backend.edev.resolve_tile_wire(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn wire_to_inpin(chip: &Chip, wire: WireCoord) -> Option<(BelCoord, &'static str)> {
    let mut cell = wire.cell;
    let (slot, pin) = match wire.slot {
        wires::IMUX_CLB_K => (bslots::CLB, "K"),
        wires::IMUX_CLB_F1 => {
            if cell.col == chip.col_e() {
                (bslots::IO[1], "O2")
            } else {
                (bslots::CLB, "F1")
            }
        }
        wires::IMUX_CLB_F2 => {
            cell.row += 1;
            if cell.row == chip.row_n() {
                (bslots::IO[0], "O2")
            } else {
                (bslots::CLB, "F2")
            }
        }
        wires::IMUX_CLB_F3 => {
            cell.col -= 1;
            if cell.col == chip.col_w() {
                (bslots::IO[1], "O2")
            } else {
                (bslots::CLB, "F3")
            }
        }
        wires::IMUX_CLB_F4 => {
            if cell.row == chip.row_s() {
                (bslots::IO[0], "O2")
            } else {
                (bslots::CLB, "F4")
            }
        }
        wires::IMUX_CLB_G1 => {
            if cell.col == chip.col_e() {
                (bslots::IO[0], "O2")
            } else {
                (bslots::CLB, "G1")
            }
        }
        wires::IMUX_CLB_G2 => {
            cell.row += 1;
            if cell.row == chip.row_n() {
                (bslots::IO[1], "O2")
            } else {
                (bslots::CLB, "G2")
            }
        }
        wires::IMUX_CLB_G3 => {
            cell.col -= 1;
            if cell.col == chip.col_w() {
                (bslots::IO[0], "O2")
            } else {
                (bslots::CLB, "G3")
            }
        }
        wires::IMUX_CLB_G4 => {
            if cell.row == chip.row_s() {
                (bslots::IO[1], "O2")
            } else {
                (bslots::CLB, "G4")
            }
        }
        wires::IMUX_CLB_C1 => {
            if cell.col == chip.col_e() {
                (bslots::DEC[1], "I")
            } else {
                (bslots::CLB, "C1")
            }
        }
        wires::IMUX_CLB_C2 => {
            cell.row += 1;
            if cell.row == chip.row_n() {
                (bslots::DEC[1], "I")
            } else {
                (bslots::CLB, "C2")
            }
        }
        wires::IMUX_CLB_C3 => {
            cell.col -= 1;
            if cell.col == chip.col_w() {
                (bslots::DEC[1], "I")
            } else {
                (bslots::CLB, "C3")
            }
        }
        wires::IMUX_CLB_C4 => {
            if cell.row == chip.row_s() {
                (bslots::DEC[1], "I")
            } else {
                (bslots::CLB, "C4")
            }
        }
        wires::IMUX_READCLK_I => (bslots::READCLK, "I"),
        wires::IMUX_RDBK_TRIG => (bslots::RDBK, "TRIG"),
        wires::IMUX_TDO_O => (bslots::TDO, "O"),
        wires::IMUX_TDO_T => (bslots::TDO, "T"),
        wires::IMUX_STARTUP_CLK => (bslots::STARTUP, "CLK"),
        wires::IMUX_STARTUP_GSR => (bslots::STARTUP, "GSR"),
        wires::IMUX_STARTUP_GTS => (bslots::STARTUP, "GTS"),
        wires::IMUX_BSCAN_TDO1 => (bslots::BSCAN, "TDO1"),
        wires::IMUX_BSCAN_TDO2 => (bslots::BSCAN, "TDO2"),
        wires::IMUX_BUFG_H => (bslots::BUFG_H, "I"),
        wires::IMUX_BUFG_V => (bslots::BUFG_V, "I"),
        _ => {
            if let Some(idx) = wires::IMUX_TBUF_I.index_of(wire.slot) {
                (bslots::TBUF[idx], "I")
            } else if let Some(idx) = wires::IMUX_TBUF_T.index_of(wire.slot) {
                (bslots::TBUF[idx], "T")
            } else if let Some(idx) = wires::IMUX_IO_O1.index_of(wire.slot) {
                if cell.col == chip.col_w() && cell.row == chip.row_s() && idx == 1 {
                    (bslots::MD1, "O")
                } else {
                    (bslots::IO[idx], "O1")
                }
            } else if let Some(idx) = wires::IMUX_IO_IK.index_of(wire.slot) {
                if cell.col == chip.col_w() && cell.row == chip.row_s() && idx == 1 {
                    (bslots::MD1, "T")
                } else if chip.kind == ChipKind::Xc4000H {
                    (bslots::HIO[1 + idx], "TS")
                } else {
                    (bslots::IO[idx], "IK")
                }
            } else if let Some(idx) = wires::IMUX_IO_OK.index_of(wire.slot) {
                if chip.kind == ChipKind::Xc4000H {
                    (bslots::HIO[idx * 3], "TS")
                } else {
                    (bslots::IO[idx], "OK")
                }
            } else if let Some(idx) = wires::IMUX_IO_T.index_of(wire.slot) {
                if chip.kind == ChipKind::Xc4000H {
                    (bslots::HIO[idx * 2], "TP")
                } else {
                    (bslots::IO[idx], "T")
                }
            } else {
                return None;
            }
        }
    };
    Some((cell.bel(slot), pin))
}

#[allow(clippy::too_many_arguments)]
fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: WireCoord,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
    hiob: usize,
    oq: bool,
    inv: bool,
) -> Fuzzer<XactBackend<'a>> {
    let chip = backend.edev.chip;
    let Some((mut bel, mut pin)) = wire_to_inpin(chip, wire) else {
        return fuzzer;
    };
    let tcrd = bel.tile(tslots::MAIN);
    if let Some(idx) = bslots::IO.index_of(bel.slot)
        && chip.kind == ChipKind::Xc4000H
    {
        assert!(matches!(pin, "O1" | "O2"));
        let hidx = idx * 2 + hiob;
        bel.slot = bslots::HIO[hidx];
        let crd = backend.ngrid.int_pip(
            tcrd,
            TileWireCoord::new_idx(0, wires::IMUX_HIO_O[hidx]),
            wire_io_o(backend.edev[tcrd].class, idx, pin),
        );
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()));
        pin = "O";
    }
    let ntile = &backend.ngrid.tiles[&tcrd];
    let block = &ntile.bels[bel.slot][0];
    if bslots::HIO.contains(bel.slot) && pin == "TP" {
        let crd = backend.ngrid.bel_pip(bel, "TP");
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()));
    }
    if bslots::IO.contains(bel.slot) && pin == "T" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BelMutex(bel, "OUT".into()), "O")
            .base(Key::BlockConfig(block, "OUT".into(), "O".into()), true)
            .base(Key::BelMutex(bel, "CLK".into()), "O")
            .base(Key::BlockConfig(block, "OUT".into(), "OK".into()), true)
            .fuzz(
                Key::BlockConfig(block, "TRI".into(), "T".into()),
                false,
                true,
            );
    }
    if bslots::IO.contains(bel.slot) && (pin == "O2" || pin == "O1") {
        let opin = if pin == "O1" {
            bcls::IO::O2
        } else {
            bcls::IO::O1
        };
        let opin = backend.edev.get_bel_input(bel, opin).wire;
        fuzzer = fuzzer
            .base(Key::WireMutex(opin), "PROHIBIT")
            .base(Key::BlockBase(block), "IO")
            .base(Key::BelMutex(bel, "OUT".into()), "TEST")
            .base(Key::BelMutex(bel, "CLK".into()), "O")
            .base(Key::BlockConfig(block, "OUT".into(), "OK".into()), true)
            .fuzz(
                Key::BlockConfig(block, "OUT".into(), if oq { "OQ" } else { "O" }.into()),
                false,
                true,
            );
        if inv {
            fuzzer = fuzzer.fuzz(
                Key::BlockConfig(block, "OUT".into(), "NOT".into()),
                false,
                true,
            )
        }
        pin = "O";
    }
    if bslots::TBUF.contains(bel.slot) {
        let mode = if pin == "I" { "WAND" } else { "WANDT" };
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "TBUF")
            .base(Key::BelMutex(bel, "TBUF".into()), mode)
            .fuzz(
                Key::BlockConfig(block, "TBUF".into(), mode.into()),
                false,
                true,
            );
        if pin == "T" {
            fuzzer = fuzzer.fuzz(
                Key::BlockConfig(block, "I".into(), "GND".into()),
                false,
                true,
            );
        }
    }
    if bel.slot == bslots::STARTUP && pin == "CLK" {
        fuzzer = fuzzer.base(Key::GlobalOpt("STARTUPCLK".into()), "CCLK");
    }
    fuzzer.fuzz(
        Key::BlockPin(block, pin.into()),
        None,
        Value::FromPin(sblock, spin.into()),
    )
}

#[derive(Clone, Debug)]
struct IntPip {
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
    hiob: usize,
    oq: bool,
    inv: bool,
}

impl IntPip {
    pub fn new(
        wire_to: TileWireCoord,
        wire_from: TileWireCoord,
        hiob: usize,
        oq: bool,
        inv: bool,
    ) -> Self {
        Self {
            wire_to,
            wire_from,
            hiob,
            oq,
            inv,
        }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for IntPip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: prjcombine_interconnect::grid::TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let rwt = backend.edev.resolve_tile_wire(tcrd, self.wire_to).unwrap();
        let rwf = backend
            .edev
            .resolve_tile_wire(tcrd, self.wire_from)
            .unwrap();
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::WireMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(tcrd, self.wire_to, self.wire_from);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(
            backend, rwt, fuzzer, block, pin, self.hiob, self.oq, self.inv,
        );
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ClbSpecialMux {
    pub attr: &'static str,
    pub val: &'static str,
    pub wire: TileWireCoord,
    pub dx: isize,
    pub dy: isize,
}

impl ClbSpecialMux {
    fn new(
        attr: &'static str,
        val: &'static str,
        wire: TileWireCoord,
        dx: isize,
        dy: isize,
    ) -> Self {
        Self {
            attr,
            val,
            wire,
            dx,
            dy,
        }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for ClbSpecialMux {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        mut tcrd: prjcombine_interconnect::grid::TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let imux = backend.edev.tile_wire(tcrd, self.wire);
        fuzzer = fuzzer.base(Key::WireMutex(imux), "PROHIBIT");
        tcrd.col += self.dx;
        tcrd.row += self.dy;
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[bslots::CLB][0];
        fuzzer = fuzzer.base(Key::BlockBase(block), "FG").fuzz(
            Key::BlockConfig(block, self.attr.into(), self.val.into()),
            false,
            true,
        );
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tcname, _) in &intdb.tile_classes {
        let tcls_index = &backend.edev.db_index[tcid];
        if tcls_index.pips_bwd.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            if let Some(idx) = wires::IMUX_HIO_T.index_of(wire_to.wire) {
                for &wire_from in ins {
                    let val = if wires::IMUX_IO_T.contains(wire_from.wire) {
                        "TP"
                    } else {
                        "TS"
                    };
                    let mut bctx = ctx.bel(bslots::HIO[idx]);
                    bctx.mode("IO")
                        .bonded_io()
                        .cfg("IN", "I")
                        .cfg("OUT", "O")
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                        .cfg_excl("TRI", val)
                        .commit();
                }
                continue;
            }
            if wires::IMUX_HIO_O.contains(wire_to.wire) {
                continue;
            }
            let mut is_iob_o = false;
            if wires::IMUX_IO_O1.contains(wire_to.wire) && tcname.starts_with("IO") {
                is_iob_o = true;
            }
            if matches!(wire_to.wire, wires::IMUX_CLB_F4 | wires::IMUX_CLB_G4)
                && tcname.starts_with("IO_S")
            {
                is_iob_o = true;
            }
            if matches!(wire_to.wire, wires::IMUX_CLB_F2 | wires::IMUX_CLB_G2)
                && matches!(tcid, tcls::CLB_N | tcls::CLB_NW | tcls::CLB_NE)
            {
                is_iob_o = true;
            }
            if matches!(wire_to.wire, wires::IMUX_CLB_F3 | wires::IMUX_CLB_G3)
                && matches!(tcid, tcls::CLB_W | tcls::CLB_SW | tcls::CLB_NW)
            {
                is_iob_o = true;
            }
            if matches!(wire_to.wire, wires::IMUX_CLB_F1 | wires::IMUX_CLB_G1)
                && tcname.starts_with("IO_E")
            {
                is_iob_o = true;
            }
            for &wire_from in ins {
                if (wires::IMUX_IO_O1.contains(wire_to.wire)
                    || wires::IMUX_IO_T.contains(wire_to.wire)
                    || wires::IMUX_TBUF_T.contains(wire_to.wire)
                    || wires::IMUX_TBUF_I.contains(wire_to.wire))
                    && matches!(wire_from.wire, wires::TIE_0 | wires::TIE_1)
                {
                    continue;
                }
                let wire_from = wire_from.tw;
                if matches!(
                    wire_from.wire,
                    wires::SPECIAL_CLB_CIN | wires::SPECIAL_CLB_COUT0
                ) {
                    let (attr, val, dx, dy) = match wire_to.wire {
                        wires::IMUX_CLB_F4 => ("F4", "CIN", 0, 0),
                        wires::IMUX_CLB_G3 => ("G3", "CIN", -1, 0),
                        wires::IMUX_CLB_G2 => ("G2", "COUT0", 0, 1),
                        _ => unreachable!(),
                    };
                    ctx.build()
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(ClbSpecialMux::new(attr, val, wire_to, dx, dy))
                        .commit();
                } else if is_iob_o {
                    if backend.edev.chip.kind == ChipKind::Xc4000H {
                        for i in 0..2 {
                            ctx.build()
                                .test_raw(DiffKey::RoutingPairSpecial(
                                    tcid,
                                    wire_to,
                                    wire_from.pos(),
                                    [specials::INT_HIO0, specials::INT_HIO1][i],
                                ))
                                .prop(IntPip::new(wire_to, wire_from, i, false, false))
                                .commit();
                        }
                    } else {
                        ctx.build()
                            .test_raw(DiffKey::RoutingPairSpecial(
                                tcid,
                                wire_to,
                                wire_from.pos(),
                                specials::INT_IO_O,
                            ))
                            .prop(IntPip::new(wire_to, wire_from, 0, false, false))
                            .commit();
                        ctx.build()
                            .test_raw(DiffKey::RoutingPairSpecial(
                                tcid,
                                wire_to,
                                wire_from.pos(),
                                specials::INT_IO_OQ,
                            ))
                            .prop(IntPip::new(wire_to, wire_from, 0, true, false))
                            .commit();
                        ctx.build()
                            .test_raw(DiffKey::RoutingPairSpecial(
                                tcid,
                                wire_to,
                                wire_from.pos(),
                                specials::INT_IO_O_INV,
                            ))
                            .prop(IntPip::new(wire_to, wire_from, 0, false, true))
                            .commit();
                        ctx.build()
                            .test_raw(DiffKey::RoutingPairSpecial(
                                tcid,
                                wire_to,
                                wire_from.pos(),
                                specials::INT_IO_OQ_INV,
                            ))
                            .prop(IntPip::new(wire_to, wire_from, 0, true, true))
                            .commit();
                    }
                } else {
                    ctx.build()
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(IntPip::new(wire_to, wire_from, 0, false, false))
                        .commit();
                }
            }
        }
    }
}

fn wire_io_o(tcid: TileClassId, idx: usize, pin: &str) -> TileWireCoord {
    TileWireCoord::new_idx(
        0,
        match pin {
            "O1" => wires::IMUX_IO_O1[idx],
            "O2" => match tcid {
                tcls::IO_W0 | tcls::IO_W1 | tcls::IO_W0_N | tcls::IO_W1_S => {
                    [wires::IMUX_CLB_G3_W, wires::IMUX_CLB_F3_W][idx]
                }
                tcls::IO_E0 | tcls::IO_E1 | tcls::IO_E0_N | tcls::IO_E1_S => {
                    [wires::IMUX_CLB_G1, wires::IMUX_CLB_F1][idx]
                }
                tcls::IO_S0 | tcls::IO_S1 | tcls::IO_S0_E | tcls::IO_S1_W => {
                    [wires::IMUX_CLB_F4, wires::IMUX_CLB_G4][idx]
                }
                tcls::IO_N0 | tcls::IO_N1 | tcls::IO_N0_E | tcls::IO_N1_W => {
                    [wires::IMUX_CLB_F2_N, wires::IMUX_CLB_G2_N][idx]
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        },
    )
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    let mut iob_o_diffs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    let mut hiob_o_diffs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        for (_, bel) in &tcls.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        if wires::IMUX_HIO_O.contains(mux.dst.wire) {
                            continue;
                        }
                        let mut iob_o = None;
                        if let Some(idx) = wires::IMUX_IO_O1.index_of(mux.dst.wire)
                            && tcname.starts_with("IO")
                        {
                            iob_o =
                                Some((vec![tcid], bslots::IO[idx], "O1", BitRectId::from_idx(0)));
                        }
                        if tcname.starts_with("IO_S") {
                            if mux.dst.wire == wires::IMUX_CLB_F4 {
                                iob_o =
                                    Some((vec![tcid], bslots::IO[0], "O2", BitRectId::from_idx(0)));
                            }
                            if mux.dst.wire == wires::IMUX_CLB_G4 {
                                iob_o =
                                    Some((vec![tcid], bslots::IO[1], "O2", BitRectId::from_idx(0)));
                            }
                        }
                        if matches!(tcid, tcls::CLB_N | tcls::CLB_NW | tcls::CLB_NE) {
                            if mux.dst.wire == wires::IMUX_CLB_F2 {
                                iob_o = Some((
                                    vec![tcls::IO_N0, tcls::IO_N1, tcls::IO_N0_E, tcls::IO_N1_W],
                                    bslots::IO[0],
                                    "O2",
                                    BitRectId::from_idx(3),
                                ));
                            }
                            if mux.dst.wire == wires::IMUX_CLB_G2 {
                                iob_o = Some((
                                    vec![tcls::IO_N0, tcls::IO_N1, tcls::IO_N0_E, tcls::IO_N1_W],
                                    bslots::IO[1],
                                    "O2",
                                    BitRectId::from_idx(3),
                                ))
                            }
                        }
                        if matches!(tcid, tcls::CLB_W | tcls::CLB_SW | tcls::CLB_NW) {
                            if mux.dst.wire == wires::IMUX_CLB_G3 {
                                iob_o = Some((
                                    vec![tcls::IO_W0, tcls::IO_W1, tcls::IO_W0_N, tcls::IO_W1_S],
                                    bslots::IO[0],
                                    "O2",
                                    BitRectId::from_idx(2),
                                ));
                            }
                            if mux.dst.wire == wires::IMUX_CLB_F3 {
                                iob_o = Some((
                                    vec![tcls::IO_W0, tcls::IO_W1, tcls::IO_W0_N, tcls::IO_W1_S],
                                    bslots::IO[1],
                                    "O2",
                                    BitRectId::from_idx(2),
                                ));
                            }
                        }
                        if tcname.starts_with("IO_E") {
                            if mux.dst.wire == wires::IMUX_CLB_G1 {
                                iob_o =
                                    Some((vec![tcid], bslots::IO[0], "O2", BitRectId::from_idx(0)));
                            }
                            if mux.dst.wire == wires::IMUX_CLB_F1 {
                                iob_o =
                                    Some((vec![tcid], bslots::IO[1], "O2", BitRectId::from_idx(0)));
                            }
                        }
                        if let Some((ref iob_tcids, bel, pin, rect_idx)) = iob_o {
                            if ctx.edev.chip.kind == ChipKind::Xc4000H {
                                let mut inps = vec![];
                                let mut got_empty = false;
                                for &src in mux.src.keys() {
                                    if src.wire == wires::TIE_0 {
                                        continue;
                                    }
                                    let diff0 = ctx.get_diff_raw(&DiffKey::RoutingPairSpecial(
                                        tcid,
                                        mux.dst,
                                        src,
                                        specials::INT_HIO0,
                                    ));
                                    let diff1 = ctx.get_diff_raw(&DiffKey::RoutingPairSpecial(
                                        tcid,
                                        mux.dst,
                                        src,
                                        specials::INT_HIO1,
                                    ));
                                    let (mut diff0, mut diff1, diff) = Diff::split(diff0, diff1);
                                    if diff.bits.is_empty() {
                                        got_empty = true;
                                    }
                                    inps.push((Some(src), diff));
                                    if rect_idx.to_idx() != 0 {
                                        for diff in [&mut diff0, &mut diff1] {
                                            *diff = Diff {
                                                bits: diff
                                                    .bits
                                                    .iter()
                                                    .map(|(&bit, &val)| {
                                                        assert_eq!(bit.rect, rect_idx);
                                                        (
                                                            TileBit {
                                                                rect: BitRectId::from_idx(0),
                                                                ..bit
                                                            },
                                                            val,
                                                        )
                                                    })
                                                    .collect(),
                                            };
                                        }
                                    }
                                    let idx = bslots::IO.index_of(bel).unwrap();
                                    for (hidx, diff) in [(idx * 2, diff0), (idx * 2 + 1, diff1)] {
                                        for &tcid in iob_tcids {
                                            let ioo = wire_io_o(tcid, idx, pin);
                                            match hiob_o_diffs
                                                .entry((
                                                    tcid,
                                                    TileWireCoord::new_idx(
                                                        0,
                                                        wires::IMUX_HIO_O[hidx],
                                                    ),
                                                ))
                                                .or_default()
                                                .entry(ioo.pos())
                                            {
                                                btree_map::Entry::Vacant(entry) => {
                                                    entry.insert(diff.clone());
                                                }
                                                btree_map::Entry::Occupied(entry) => {
                                                    assert_eq!(*entry.get(), diff);
                                                }
                                            }
                                        }
                                    }
                                }
                                if pin == "O1" {
                                    assert!(!got_empty);
                                    inps.push((
                                        Some(TileWireCoord::new_idx(0, wires::TIE_0).pos()),
                                        Diff::default(),
                                    ));
                                } else {
                                    assert!(got_empty);
                                }
                                inps.sort_by_key(|&(k, _)| k);
                                let item = xlat_enum_raw(inps, OcdMode::Mux);
                                if item.bits.is_empty() {
                                    println!(
                                        "UMMM MUX {tcname} {mux_name} is empty",
                                        mux_name =
                                            mux.dst.to_string(intdb, &intdb.tile_classes[tcid])
                                    );
                                }
                                ctx.insert_mux(tcid, mux.dst, item);
                            } else {
                                for spec in [
                                    specials::INT_IO_O,
                                    specials::INT_IO_OQ,
                                    specials::INT_IO_O_INV,
                                    specials::INT_IO_OQ_INV,
                                ] {
                                    let mut inps = vec![];
                                    for &src in mux.src.keys() {
                                        if src.wire == wires::TIE_0 {
                                            continue;
                                        }
                                        let diff = ctx.get_diff_raw(&DiffKey::RoutingPairSpecial(
                                            tcid, mux.dst, src, spec,
                                        ));
                                        inps.push((Some(src), diff));
                                    }
                                    let mut common = inps[0].1.clone();
                                    for (_, diff) in &inps {
                                        common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                                    }
                                    let mut got_empty = false;
                                    for (_, diff) in &mut inps {
                                        *diff = diff.combine(&!&common);
                                        if diff.bits.is_empty() {
                                            got_empty = true;
                                        }
                                    }
                                    if pin == "O1" {
                                        assert!(!got_empty);
                                        inps.push((
                                            Some(TileWireCoord::new_idx(0, wires::TIE_0).pos()),
                                            Diff::default(),
                                        ));
                                    } else {
                                        assert!(got_empty);
                                    }
                                    inps.sort_by_key(|&(k, _)| k);
                                    let item = xlat_enum_raw(inps, OcdMode::Mux);
                                    if item.bits.is_empty() {
                                        println!(
                                            "UMMM MUX {tcname} {mux_name} is empty",
                                            mux_name =
                                                mux.dst.to_string(intdb, &intdb.tile_classes[tcid])
                                        );
                                    }
                                    ctx.insert_mux(tcid, mux.dst, item);
                                    if rect_idx.to_idx() != 0 {
                                        common = Diff {
                                            bits: common
                                                .bits
                                                .iter()
                                                .map(|(&bit, &val)| {
                                                    assert_eq!(bit.rect, rect_idx);
                                                    (
                                                        TileBit {
                                                            rect: BitRectId::from_idx(0),
                                                            ..bit
                                                        },
                                                        val,
                                                    )
                                                })
                                                .collect(),
                                        };
                                    }
                                    for &iob_tcid in iob_tcids {
                                        match iob_o_diffs
                                            .entry((iob_tcid, bel))
                                            .or_default()
                                            .entry((pin, spec))
                                        {
                                            btree_map::Entry::Vacant(entry) => {
                                                entry.insert(common.clone());
                                            }
                                            btree_map::Entry::Occupied(entry) => {
                                                assert_eq!(*entry.get(), common);
                                            }
                                        }
                                    }
                                }
                            }
                        } else if wires::IMUX_TBUF_I.contains(mux.dst.wire) {
                            continue;
                        } else if let Some(idx) = wires::IMUX_TBUF_T.index_of(mux.dst.wire) {
                            let mut t_inps = vec![];
                            for &src in mux.src.keys() {
                                if matches!(src.wire, wires::TIE_0 | wires::TIE_1) {
                                    continue;
                                }
                                t_inps.push((
                                    Some(src),
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, src)),
                                ));
                            }
                            let imux_i = TileWireCoord::new_idx(0, wires::IMUX_TBUF_I[idx]);
                            let mux_i = sb
                                .items
                                .iter()
                                .filter_map(|item| {
                                    if let SwitchBoxItem::Mux(mux) = item {
                                        Some(mux)
                                    } else {
                                        None
                                    }
                                })
                                .find(|mux| mux.dst == imux_i)
                                .unwrap();
                            let mut i_inps = vec![];
                            for &src in mux_i.src.keys() {
                                if src.wire == wires::TIE_0 {
                                    continue;
                                }
                                i_inps.push((
                                    Some(src),
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, imux_i, src)),
                                ));
                            }
                            let mut t_bits = HashSet::new();
                            for (_, diff) in &t_inps {
                                for &bit in diff.bits.keys() {
                                    t_bits.insert(bit);
                                }
                            }
                            for (_, diff) in &mut i_inps {
                                let t_diff = diff.split_bits(&t_bits);
                                t_inps.push((
                                    Some(TileWireCoord::new_idx(0, wires::TIE_0).pos()),
                                    t_diff,
                                ));
                            }
                            i_inps.push((
                                Some(TileWireCoord::new_idx(0, wires::TIE_0).pos()),
                                Diff::default(),
                            ));
                            i_inps.sort_by_key(|&(k, _)| k);
                            let item_i = xlat_enum_raw(i_inps, OcdMode::Mux);
                            if item_i.bits.is_empty() {
                                println!(
                                    "UMMM MUX {tcname} {imux_i} is empty",
                                    imux_i = imux_i.to_string(intdb, &intdb.tile_classes[tcid])
                                );
                            }
                            ctx.insert_mux(tcid, imux_i, item_i);
                            let mut common = t_inps[0].1.clone();
                            for (_, diff) in &t_inps {
                                common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                            }
                            assert_eq!(common.bits.len(), 1);
                            let mut got_empty = false;
                            for (_, diff) in &mut t_inps {
                                *diff = diff.combine(&!&common);
                                if diff.bits.is_empty() {
                                    got_empty = true;
                                }
                            }
                            assert!(
                                !got_empty,
                                "fuckup on {tcname} {mux_name}",
                                mux_name = mux.dst.to_string(intdb, &intdb.tile_classes[tcid])
                            );
                            t_inps.push((
                                Some(TileWireCoord::new_idx(0, wires::TIE_1).pos()),
                                Diff::default(),
                            ));
                            t_inps.sort_by_key(|&(k, _)| k);
                            let item_t = xlat_enum_raw(t_inps, OcdMode::Mux);
                            if item_t.bits.is_empty() {
                                println!(
                                    "UMMM MUX {tcname} {mux_name} is empty",
                                    mux_name = mux.dst.to_string(intdb, &intdb.tile_classes[tcid])
                                );
                            }
                            ctx.insert_mux(tcid, mux.dst, item_t);
                            ctx.insert_bel_attr_bool(
                                tcid,
                                bslots::TBUF[idx],
                                bcls::TBUF::DRIVE1,
                                xlat_bit_raw(!common),
                            );
                        } else {
                            let mut inps = vec![];
                            let mut got_empty = false;
                            for &src in mux.src.keys() {
                                let diff = if wires::IMUX_IO_T.contains(mux.dst.wire)
                                    && src.wire == wires::TIE_0
                                {
                                    Diff::default()
                                } else {
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, src))
                                };
                                if diff.bits.is_empty() {
                                    got_empty = true;
                                }
                                inps.push((Some(src), diff));
                            }

                            for (rtile, rwire, rbel, rattr) in [
                                (
                                    tcls::CNR_SW,
                                    wires::IMUX_IO_IK[1],
                                    bslots::MD1,
                                    bcls::MD1::T_ENABLE,
                                ),
                                (
                                    tcls::CNR_SW,
                                    wires::IMUX_IO_O1[1],
                                    bslots::MD1,
                                    bcls::MD1::O_ENABLE,
                                ),
                                (
                                    tcls::CNR_SW,
                                    wires::IMUX_RDBK_TRIG,
                                    bslots::RDBK,
                                    bcls::RDBK::ENABLE,
                                ),
                                (
                                    tcls::CNR_SE,
                                    wires::IMUX_STARTUP_GTS,
                                    bslots::STARTUP,
                                    bcls::STARTUP::GTS_ENABLE,
                                ),
                                (
                                    tcls::CNR_SE,
                                    wires::IMUX_STARTUP_GSR,
                                    bslots::STARTUP,
                                    bcls::STARTUP::GSR_ENABLE,
                                ),
                                (
                                    tcls::CNR_NE,
                                    wires::IMUX_TDO_T,
                                    bslots::TDO,
                                    bcls::TDO::T_ENABLE,
                                ),
                                (
                                    tcls::CNR_NE,
                                    wires::IMUX_TDO_O,
                                    bslots::TDO,
                                    bcls::TDO::O_ENABLE,
                                ),
                            ] {
                                if tcid == rtile && mux.dst.wire == rwire {
                                    let mut common = inps[0].1.clone();
                                    for (_, diff) in &inps {
                                        common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                                    }
                                    assert_eq!(common.bits.len(), 1);
                                    for (_, diff) in &mut inps {
                                        *diff = diff.combine(&!&common);
                                        if diff.bits.is_empty() {
                                            got_empty = true;
                                        }
                                    }
                                    assert!(got_empty);
                                    ctx.insert_bel_attr_bool(
                                        tcid,
                                        rbel,
                                        rattr,
                                        xlat_bit_raw(common),
                                    );
                                }
                            }
                            if !got_empty {
                                inps.push((None, Diff::default()));
                            }
                            inps.sort_by_key(|&(k, _)| k);
                            let item = xlat_enum_raw(inps, OcdMode::Mux);
                            if item.bits.is_empty() {
                                println!(
                                    "UMMM MUX {tcname} {mux_name} is empty",
                                    mux_name = mux.dst.to_string(intdb, &intdb.tile_classes[tcid])
                                );
                            }
                            ctx.insert_mux(tcid, mux.dst, item);
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        ctx.collect_progbuf(tcid, buf.dst, buf.src);
                    }
                    SwitchBoxItem::Pass(pass) => {
                        ctx.collect_pass(tcid, pass.dst, pass.src);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    for ((tcid, dst), diffs) in hiob_o_diffs {
        let diffs = Vec::from_iter(diffs.into_iter().map(|(k, v)| (Some(k), v)));
        let item = xlat_enum_raw(diffs, OcdMode::Mux);
        ctx.insert_mux(tcid, dst, item);
    }
    for ((tcid, bel), mut diffs) in iob_o_diffs {
        assert_eq!(diffs.len(), 8);

        let mut common = diffs.values().next().unwrap().clone();
        for diff in diffs.values() {
            common.bits.retain(|bit, _| diff.bits.contains_key(bit));
        }
        for diff in diffs.values_mut() {
            *diff = diff.combine(&!&common);
        }
        let item = xlat_bit_raw(!common);
        ctx.insert_bel_input_inv(tcid, bel, bcls::IO::T, item);

        let diff_o1_o = diffs.remove(&("O1", specials::INT_IO_O)).unwrap();
        let diff_o1_oq = diffs.remove(&("O1", specials::INT_IO_OQ)).unwrap();
        let mut diff_o1_oi = diffs.remove(&("O1", specials::INT_IO_O_INV)).unwrap();
        let mut diff_o1_oqi = diffs.remove(&("O1", specials::INT_IO_OQ_INV)).unwrap();
        let mut diff_o2_o = diffs.remove(&("O2", specials::INT_IO_O)).unwrap();
        let mut diff_o2_oq = diffs.remove(&("O2", specials::INT_IO_OQ)).unwrap();
        let mut diff_o2_oi = diffs.remove(&("O2", specials::INT_IO_O_INV)).unwrap();
        let mut diff_o2_oqi = diffs.remove(&("O2", specials::INT_IO_OQ_INV)).unwrap();
        assert!(diffs.is_empty());

        let diff_inv_off_d = diff_o1_oqi.combine(&!&diff_o1_oq);
        diff_o1_oi = diff_o1_oi.combine(&!&diff_inv_off_d);
        diff_o1_oqi = diff_o1_oqi.combine(&!&diff_inv_off_d);
        diff_o2_oi = diff_o2_oi.combine(&!&diff_inv_off_d);
        diff_o2_oqi = diff_o2_oqi.combine(&!&diff_inv_off_d);
        let item = xlat_bit_raw(diff_inv_off_d);
        ctx.insert_bel_attr_bool(tcid, bel, bcls::IO::OFF_D_INV, item);

        assert_eq!(diff_o1_oq, diff_o1_oqi);
        assert_eq!(diff_o2_oq, diff_o2_oqi);

        let diff_mux_off_d_o2 = diff_o2_oq.combine(&!&diff_o1_oq);
        diff_o2_o = diff_o2_o.combine(&!&diff_mux_off_d_o2);
        diff_o2_oi = diff_o2_oi.combine(&!&diff_mux_off_d_o2);
        diff_o2_oq = diff_o2_oq.combine(&!&diff_mux_off_d_o2);
        let item = xlat_enum_attr(vec![
            (enums::IO_MUX_OFF_D::O1, Diff::default()),
            (enums::IO_MUX_OFF_D::O2, diff_mux_off_d_o2),
        ]);
        ctx.insert_bel_attr_raw(tcid, bel, bcls::IO::MUX_OFF_D, item);

        assert_eq!(diff_o1_oq, diff_o2_oq);

        let mut diff_off_used = diff_o1_oq.clone();
        for diff in [&diff_o1_o, &diff_o1_oi, &diff_o2_o, &diff_o2_oi] {
            diff_off_used
                .bits
                .retain(|bit, _| !diff.bits.contains_key(bit));
        }
        let item = xlat_enum_attr(vec![
            (enums::IO_MUX_O::OQ, diff_o1_oq.combine(&!&diff_off_used)),
            (enums::IO_MUX_O::O1, diff_o1_o),
            (enums::IO_MUX_O::O1_INV, diff_o1_oi),
            (enums::IO_MUX_O::O2, diff_o2_o),
            (enums::IO_MUX_O::O2_INV, diff_o2_oi),
        ]);
        ctx.insert_bel_attr_raw(tcid, bel, bcls::IO::MUX_O, item);

        let item = xlat_bit_raw(diff_off_used);
        ctx.insert_bel_attr_bool(tcid, bel, bcls::IO::OFF_USED, item);
    }
}
