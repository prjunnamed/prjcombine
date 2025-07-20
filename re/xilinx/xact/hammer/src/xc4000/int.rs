use std::collections::{BTreeMap, HashSet, btree_map};

use prjcombine_interconnect::{
    db::{BelInfo, CellSlotId, SwitchBoxItem, TileWireCoord},
    grid::{TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_bit, xlat_enum, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind, tslots};
use unnamed_entity::EntityId;

use crate::{
    backend::{Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    props::DynProp,
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
    let rwf = backend.egrid.resolve_tile_wire(tcrd, wire_from).unwrap();
    let rwt = backend.egrid.resolve_tile_wire(tcrd, wire_to).unwrap();
    fuzzer = fuzzer.base(Key::NodeMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(tcrd, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
}

fn drive_wire<'a>(
    backend: &XactBackend<'a>,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: WireCoord,
    wire_avoid: WireCoord,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let grid = backend.edev.chip;
    let mut cell = wire_target.cell;
    let wt = wire_target.slot;
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (long_tbuf0, long_tbuf1) = if grid.kind == ChipKind::Xc4000A {
        ("LONG.H1", "LONG.H2")
    } else {
        ("LONG.H2", "LONG.H3")
    };
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") {
        let (slot, mut pin) = match wtn {
            "OUT.CLB.FX" => (bels::CLB, "X"),
            "OUT.CLB.GY" => (bels::CLB, "Y"),
            "OUT.CLB.FXQ" => (bels::CLB, "XQ"),
            "OUT.CLB.GYQ" => (bels::CLB, "YQ"),
            "OUT.BT.IOB0.I1" => (bels::IO0, "I1"),
            "OUT.BT.IOB0.I2" => (bels::IO0, "I2"),
            "OUT.BT.IOB1.I1" if cell.col == grid.col_w() && cell.row == grid.row_n() => {
                (bels::BSCAN, "SEL2")
            }
            "OUT.BT.IOB1.I2" if cell.col == grid.col_w() && cell.row == grid.row_n() => {
                (bels::BSCAN, "DRCK")
            }
            "OUT.BT.IOB1.I1" if cell.col == grid.col_w() && cell.row == grid.row_s() => {
                (bels::MD2, "I")
            }
            "OUT.BT.IOB1.I2" if cell.col == grid.col_w() && cell.row == grid.row_s() => {
                (bels::RDBK, "RIP")
            }
            "OUT.BT.IOB1.I1" => (bels::IO1, "I1"),
            "OUT.BT.IOB1.I2" => (bels::IO1, "I2"),
            "OUT.LR.IOB0.I1" => (bels::IO0, "I1"),
            "OUT.LR.IOB0.I2" => (bels::IO0, "I2"),
            "OUT.LR.IOB1.I1" if cell.col == grid.col_w() && cell.row == grid.row_n() => {
                (bels::BSCAN, "SEL1")
            }
            "OUT.LR.IOB1.I2" if cell.col == grid.col_w() && cell.row == grid.row_n() => {
                (bels::BSCAN, "IDLE")
            }
            "OUT.LR.IOB1.I1" if cell.col == grid.col_e() && cell.row == grid.row_n() => {
                (bels::OSC, "F8M")
            }
            "OUT.LR.IOB1.I2" if cell.col == grid.col_e() && cell.row == grid.row_n() => {
                (bels::OSC, "OUT0")
            }
            "OUT.LR.IOB1.I1" => (bels::IO1, "I1"),
            "OUT.LR.IOB1.I2" => (bels::IO1, "I2"),
            "OUT.HIOB0.I" => (bels::HIO0, "I"),
            "OUT.HIOB1.I" => (bels::HIO1, "I"),
            "OUT.HIOB2.I" => (bels::HIO2, "I"),
            "OUT.HIOB3.I" => (bels::HIO3, "I"),
            "OUT.MD0.I" => (bels::MD0, "I"),
            "OUT.STARTUP.DONEIN" => (bels::STARTUP, "DONEIN"),
            "OUT.STARTUP.Q1Q4" => (bels::STARTUP, "Q1Q4"),
            "OUT.STARTUP.Q2" => (bels::STARTUP, "Q2"),
            "OUT.STARTUP.Q3" => (bels::STARTUP, "Q3"),
            "OUT.RDBK.DATA" => (bels::RDBK, "DATA"),
            "OUT.UPDATE.O" => (bels::UPDATE, "O"),
            "OUT.OSC.MUX1" => (bels::OSC, "OUT1"),
            "OUT.IOB.CLKIN" => (
                if grid.kind == ChipKind::Xc4000H {
                    if cell.col == grid.col_w() {
                        if cell.row < grid.row_mid() {
                            bels::HIO3
                        } else {
                            bels::HIO0
                        }
                    } else if cell.col == grid.col_e() {
                        if cell.row < grid.row_mid() {
                            bels::HIO2
                        } else {
                            bels::HIO0
                        }
                    } else if cell.row == grid.row_s() {
                        if cell.col < grid.col_mid() {
                            bels::HIO0
                        } else {
                            bels::HIO3
                        }
                    } else if cell.row == grid.row_n() {
                        if cell.col < grid.col_mid() {
                            bels::HIO0
                        } else {
                            bels::HIO2
                        }
                    } else {
                        unreachable!()
                    }
                } else {
                    if cell.col == grid.col_w() {
                        if cell.row < grid.row_mid() {
                            bels::IO1
                        } else {
                            bels::IO0
                        }
                    } else if cell.col == grid.col_e() {
                        bels::IO0
                    } else if cell.row == grid.row_s() {
                        if cell.col < grid.col_mid() {
                            bels::IO0
                        } else {
                            bels::IO1
                        }
                    } else if cell.row == grid.row_n() {
                        bels::IO0
                    } else {
                        unreachable!()
                    }
                },
                "CLKIN",
            ),
            _ => panic!("umm {wtn}"),
        };
        let bel = cell.bel(slot);
        let slot_name = backend.egrid.db.bel_slots.key(slot).as_str();
        if slot_name.starts_with("IO") && grid.kind == ChipKind::Xc4000H {
            (
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wire_target.slot,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(if slot == bels::IO0 {
                        "OUT.HIOB0.I"
                    } else {
                        "OUT.HIOB2.I"
                    }),
                },
            )
        } else {
            let tcrd = cell.tile(tslots::MAIN);
            let nnode = &backend.ngrid.tiles[&tcrd];
            let mut block = &nnode.bels[slot][0];
            if pin == "CLKIN" {
                block = &nnode.bels[slot][1];
                pin = "I";
            }
            if slot == bels::OSC {
                let crd0 = backend.ngrid.bel_pip(bel, "OUT0.F500K");
                let crd1 = backend.ngrid.bel_pip(bel, "OUT1.F500K");
                fuzzer = fuzzer
                    .base(Key::BelMutex(bel, "MODE".into()), "USE")
                    .base(Key::Pip(crd0), Value::FromPin(block, "F500K".into()))
                    .base(Key::Pip(crd1), Value::FromPin(block, "F500K".into()));
                if pin == "OUT0" || pin == "OUT1" {
                    pin = "F500K";
                }
            }
            return (
                fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
                block,
                pin,
            );
        }
    } else if wtn == "GND" {
        let tcrd = cell.tile(tslots::MAIN);
        let nnode = &backend.ngrid.tiles[&tcrd];
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &nnode.tie_names[0],
            "O",
        );
    } else if let Some(idx) = wtn.strip_prefix("GCLK") {
        let idx: usize = idx.parse().unwrap();
        let bel = cell.with_row(grid.row_mid()).bel(bels::CLKH);
        let (block, inp) = [
            ("bufgp_tl", "I.UL.V"),
            ("bufgp_bl", "I.LL.H"),
            ("bufgp_br", "I.LR.V"),
            ("bufgp_tr", "I.UR.H"),
        ][idx];
        let crd = backend.ngrid.bel_pip(bel, &format!("O{idx}.{inp}"));

        fuzzer = fuzzer
            .base(Key::BelMutex(bel, format!("O{idx}")), "USE")
            .base(Key::Pip(crd), Value::FromPin(block, "O".into()));
        return (fuzzer, block, "O");
    } else if wtn.starts_with("DEC") {
        if wtn.starts_with("DEC.H") {
            if cell.col == grid.col_w() {
                cell.col += 1;
            } else if cell.col == grid.col_e() {
                cell.col -= 1;
            }
        } else {
            if cell.row == grid.row_s() {
                cell.row += 1;
            } else if cell.row == grid.row_n() {
                cell.row -= 1;
            }
        }
        let idx: usize = wtn[5..].parse().unwrap();
        let pin = ["O1", "O2", "O3", "O4"][idx];
        let tcrd = cell.tile(tslots::MAIN);
        let nnode = &backend.ngrid.tiles[&tcrd];
        let bel = cell.bel(bels::DEC0);
        let block = &nnode.bels[bels::DEC0][0];
        let crd = backend.ngrid.bel_pip(bel, pin);
        fuzzer = fuzzer
            .base(Key::Pip(crd), Value::FromPin(block, pin.into()))
            .base(Key::NodeMutex(wire_target), "SHARED_ROOT");
        return (fuzzer, block, pin);
    } else if (wtn == long_tbuf0 || wtn == long_tbuf1)
        && !(cell.row == grid.row_s() || cell.row == grid.row_n())
    {
        let slot = if wtn == long_tbuf0 {
            bels::TBUF0
        } else {
            bels::TBUF1
        };
        let tcrd = cell.tile(tslots::MAIN);
        let bel = cell.bel(slot);
        let nnode = &backend.ngrid.tiles[&tcrd];
        let block = &nnode.bels[slot][0];
        let crd = backend.ngrid.bel_pip(bel, "O");
        fuzzer = fuzzer
            .base(Key::Pip(crd), Value::FromPin(block, "O".into()))
            .base(Key::NodeMutex(wire_target), "SHARED_ROOT");
        return (fuzzer, block, "O");
    } else if wtn.starts_with("SINGLE") || wtn.starts_with("DOUBLE") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: w.slot,
                }) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("OUT") {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (
                                    tcrd,
                                    TileWireCoord {
                                        cell: CellSlotId::from_idx(0),
                                        wire: w.slot,
                                    },
                                    inp.tw,
                                );
                            }
                        }
                    }
                }
            }
            panic!("ummm no out?")
        }
    } else if wtn.starts_with("LONG") || wtn.starts_with("IO.DOUBLE") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: w.slot,
                }) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("SINGLE") {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (
                                    tcrd,
                                    TileWireCoord {
                                        cell: CellSlotId::from_idx(0),
                                        wire: w.slot,
                                    },
                                    inp.tw,
                                );
                            }
                        }
                    }
                }
            }
            panic!("ummm no out?")
        }
    } else if wtn.starts_with("IO.DBUF") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: w.slot,
                }) {
                    for &inp in ins {
                        if backend
                            .egrid
                            .db
                            .wires
                            .key(inp.wire)
                            .starts_with("IO.DOUBLE")
                        {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (
                                    tcrd,
                                    TileWireCoord {
                                        cell: CellSlotId::from_idx(0),
                                        wire: w.slot,
                                    },
                                    inp.tw,
                                );
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
    let nwt = backend.egrid.resolve_tile_wire(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
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
    let grid = backend.edev.chip;
    let mut cell = wire.cell;
    let w = wire.slot;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if !wn.starts_with("IMUX") {
        return fuzzer;
    }
    let (mut slot, mut pin) = match wn {
        "IMUX.CLB.K" => (bels::CLB, "K"),
        "IMUX.CLB.F1" => {
            if cell.col == grid.col_e() {
                (bels::IO1, "O2")
            } else {
                (bels::CLB, "F1")
            }
        }
        "IMUX.CLB.F2" => {
            cell.row += 1;
            if cell.row == grid.row_n() {
                (bels::IO0, "O2")
            } else {
                (bels::CLB, "F2")
            }
        }
        "IMUX.CLB.F3" => {
            cell.col -= 1;
            if cell.col == grid.col_w() {
                (bels::IO1, "O2")
            } else {
                (bels::CLB, "F3")
            }
        }
        "IMUX.CLB.F4" => {
            if cell.row == grid.row_s() {
                (bels::IO0, "O2")
            } else {
                (bels::CLB, "F4")
            }
        }
        "IMUX.CLB.G1" => {
            if cell.col == grid.col_e() {
                (bels::IO0, "O2")
            } else {
                (bels::CLB, "G1")
            }
        }
        "IMUX.CLB.G2" => {
            cell.row += 1;
            if cell.row == grid.row_n() {
                (bels::IO1, "O2")
            } else {
                (bels::CLB, "G2")
            }
        }
        "IMUX.CLB.G3" => {
            cell.col -= 1;
            if cell.col == grid.col_w() {
                (bels::IO0, "O2")
            } else {
                (bels::CLB, "G3")
            }
        }
        "IMUX.CLB.G4" => {
            if cell.row == grid.row_s() {
                (bels::IO1, "O2")
            } else {
                (bels::CLB, "G4")
            }
        }
        "IMUX.CLB.C1" => {
            if cell.col == grid.col_e() {
                (bels::DEC1, "I")
            } else {
                (bels::CLB, "C1")
            }
        }
        "IMUX.CLB.C2" => {
            cell.row += 1;
            if cell.row == grid.row_n() {
                (bels::DEC1, "I")
            } else {
                (bels::CLB, "C2")
            }
        }
        "IMUX.CLB.C3" => {
            cell.col -= 1;
            if cell.col == grid.col_w() {
                (bels::DEC1, "I")
            } else {
                (bels::CLB, "C3")
            }
        }
        "IMUX.CLB.C4" => {
            if cell.row == grid.row_s() {
                (bels::DEC1, "I")
            } else {
                (bels::CLB, "C4")
            }
        }
        "IMUX.TBUF0.I" => (bels::TBUF0, "I"),
        "IMUX.TBUF0.TS" => (bels::TBUF0, "T"),
        "IMUX.TBUF1.I" => (bels::TBUF1, "I"),
        "IMUX.TBUF1.TS" => (bels::TBUF1, "T"),
        "IMUX.IOB1.O1" if cell.col == grid.col_w() && cell.row == grid.row_s() => (bels::MD1, "O"),
        "IMUX.IOB1.IK" if cell.col == grid.col_w() && cell.row == grid.row_s() => (bels::MD1, "T"),
        "IMUX.IOB0.OK" if grid.kind == ChipKind::Xc4000H => (bels::HIO0, "TS"),
        "IMUX.IOB0.IK" if grid.kind == ChipKind::Xc4000H => (bels::HIO1, "TS"),
        "IMUX.IOB1.IK" if grid.kind == ChipKind::Xc4000H => (bels::HIO2, "TS"),
        "IMUX.IOB1.OK" if grid.kind == ChipKind::Xc4000H => (bels::HIO3, "TS"),
        "IMUX.IOB0.TS" if grid.kind == ChipKind::Xc4000H => (bels::HIO0, "TP"),
        "IMUX.IOB1.TS" if grid.kind == ChipKind::Xc4000H => (bels::HIO2, "TP"),
        "IMUX.IOB0.IK" => (bels::IO0, "IK"),
        "IMUX.IOB1.IK" => (bels::IO1, "IK"),
        "IMUX.IOB0.OK" => (bels::IO0, "OK"),
        "IMUX.IOB1.OK" => (bels::IO1, "OK"),
        "IMUX.IOB0.TS" => (bels::IO0, "T"),
        "IMUX.IOB1.TS" => (bels::IO1, "T"),
        "IMUX.IOB0.O1" => (bels::IO0, "O1"),
        "IMUX.IOB1.O1" => (bels::IO1, "O1"),
        "IMUX.READCLK.I" => (bels::READCLK, "I"),
        "IMUX.RDBK.TRIG" => (bels::RDBK, "TRIG"),
        "IMUX.TDO.O" => (bels::TDO, "O"),
        "IMUX.TDO.T" => (bels::TDO, "T"),
        "IMUX.STARTUP.CLK" => (bels::STARTUP, "CLK"),
        "IMUX.STARTUP.GSR" => (bels::STARTUP, "GSR"),
        "IMUX.STARTUP.GTS" => (bels::STARTUP, "GTS"),
        "IMUX.BSCAN.TDO1" => (bels::BSCAN, "TDO1"),
        "IMUX.BSCAN.TDO2" => (bels::BSCAN, "TDO2"),
        "IMUX.BUFG.H" => (bels::BUFGLS_H, "I"),
        "IMUX.BUFG.V" => (bels::BUFGLS_V, "I"),
        _ => panic!("umm {wn}?"),
    };
    if grid.kind == ChipKind::Xc4000H {
        if slot == bels::IO0 {
            slot = [bels::HIO0, bels::HIO1][hiob];
        }
        if slot == bels::IO1 {
            slot = [bels::HIO2, bels::HIO3][hiob];
        }
    }
    let bel = cell.bel(slot);
    let tcrd = cell.tile(tslots::MAIN);
    let tile = &backend.egrid[tcrd];
    let tcls = &backend.egrid.db.tile_classes[tile.class];
    let nnode = &backend.ngrid.tiles[&tcrd];
    let block = &nnode.bels[slot][0];
    if bels::HIO.contains(&slot) && pin == "TP" {
        let crd = backend.ngrid.bel_pip(bel, "T1");
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()));
    }
    if bels::HIO.contains(&slot) && (pin == "O1" || pin == "O2") {
        let crd = backend.ngrid.bel_pip(bel, pin);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()));
        pin = "O";
    }
    if bels::IO.contains(&slot) && pin == "T" {
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
    if bels::IO.contains(&slot) && (pin == "O2" || pin == "O1") {
        let opin = if pin == "O1" { "O2" } else { "O1" };
        let BelInfo::Bel(ref bel_data) = tcls.bels[slot] else {
            unreachable!()
        };
        let opin = bel_data.pins[opin].wires.iter().copied().next().unwrap();
        let opin = backend.egrid.resolve_tile_wire(tcrd, opin).unwrap();
        fuzzer = fuzzer
            .base(Key::NodeMutex(opin), "PROHIBIT")
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
    if bels::TBUF.contains(&slot) {
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
    if slot == bels::STARTUP && pin == "CLK" {
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
        let rwt = backend.egrid.resolve_tile_wire(tcrd, self.wire_to).unwrap();
        let rwf = backend
            .egrid
            .resolve_tile_wire(tcrd, self.wire_from)
            .unwrap();
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::NodeMutex(rwt), false, true);
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
        let imux = backend.egrid.tile_wire(tcrd, self.wire);
        fuzzer = fuzzer.base(Key::NodeMutex(imux), "PROHIBIT");
        tcrd.col += self.dx;
        tcrd.row += self.dy;
        let nnode = &backend.ngrid.tiles[&tcrd];
        let block = &nnode.bels[bels::CLB][0];
        fuzzer = fuzzer.base(Key::BlockBase(block), "FG").fuzz(
            Key::BlockConfig(block, self.attr.into(), self.val.into()),
            false,
            true,
        );
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.egrid.db;
    for (tcid, tile, _) in &intdb.tile_classes {
        let tcls_index = &backend.egrid.db_index.tile_classes[tcid];
        if tcls_index.pips_bwd.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let out_name = intdb.wires.key(wire_to.wire);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt:#}.{out_name}", wtt = wire_to.cell)
            } else {
                assert_eq!(wire_to.cell.to_idx(), 0);
                format!("MUX.{out_name}")
            };
            let mut is_iob_o = false;
            if out_name.starts_with("IMUX.IO") && out_name.ends_with("O1") && tile.starts_with("IO")
            {
                is_iob_o = true;
            }
            if (out_name == "IMUX.CLB.F4" || out_name == "IMUX.CLB.G4") && tile.starts_with("IO.B")
            {
                is_iob_o = true;
            }
            if (out_name == "IMUX.CLB.F2" || out_name == "IMUX.CLB.G2")
                && tile.starts_with("CLB")
                && tile.ends_with('T')
            {
                is_iob_o = true;
            }
            if (out_name == "IMUX.CLB.F3" || out_name == "IMUX.CLB.G3") && tile.starts_with("CLB.L")
            {
                is_iob_o = true;
            }
            if (out_name == "IMUX.CLB.F1" || out_name == "IMUX.CLB.G1") && tile.starts_with("IO.R")
            {
                is_iob_o = true;
            }
            for &wire_from in ins {
                let wire_from = wire_from.tw;
                let wire_from_name = intdb.wires.key(wire_from.wire);
                let in_name = format!("{:#}.{}", wire_from.cell, wire_from_name);
                if is_iob_o {
                    if backend.edev.chip.kind == ChipKind::Xc4000H {
                        for i in 0..2 {
                            ctx.build()
                                .test_manual("INT", &mux_name, format!("{in_name}.HIOB{i}"))
                                .prop(IntPip::new(wire_to, wire_from, i, false, false))
                                .commit();
                        }
                    } else {
                        ctx.build()
                            .test_manual("INT", &mux_name, format!("{in_name}.O"))
                            .prop(IntPip::new(wire_to, wire_from, 0, false, false))
                            .commit();
                        ctx.build()
                            .test_manual("INT", &mux_name, format!("{in_name}.OQ"))
                            .prop(IntPip::new(wire_to, wire_from, 0, true, false))
                            .commit();
                        ctx.build()
                            .test_manual("INT", &mux_name, format!("{in_name}.O.NOT"))
                            .prop(IntPip::new(wire_to, wire_from, 0, false, true))
                            .commit();
                        ctx.build()
                            .test_manual("INT", &mux_name, format!("{in_name}.OQ.NOT"))
                            .prop(IntPip::new(wire_to, wire_from, 0, true, true))
                            .commit();
                    }
                } else {
                    ctx.build()
                        .test_manual("INT", &mux_name, &in_name)
                        .prop(IntPip::new(wire_to, wire_from, 0, false, false))
                        .commit();
                }
            }
        }
        if tile.starts_with("CLB") {
            ctx.build()
                .test_manual("INT", "MUX.IMUX.CLB.F4", "CIN")
                .prop(ClbSpecialMux::new(
                    "F4",
                    "CIN",
                    TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire("IMUX.CLB.F4"),
                    },
                    0,
                    0,
                ))
                .commit();
        }
        if tile.starts_with("IO.R")
            || matches!(
                &tile[..],
                "CLB" | "CLB.B" | "CLB.T" | "CLB.R" | "CLB.RB" | "CLB.RT"
            )
        {
            ctx.build()
                .test_manual("INT", "MUX.IMUX.CLB.G3", "CIN")
                .prop(ClbSpecialMux::new(
                    "G3",
                    "CIN",
                    TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire("IMUX.CLB.G3"),
                    },
                    -1,
                    0,
                ))
                .commit();
        }
        if tile.starts_with("IO.B")
            || matches!(
                &tile[..],
                "CLB" | "CLB.B" | "CLB.L" | "CLB.LB" | "CLB.R" | "CLB.RB"
            )
        {
            ctx.build()
                .test_manual("INT", "MUX.IMUX.CLB.G2", "COUT0")
                .prop(ClbSpecialMux::new(
                    "G2",
                    "COUT0",
                    TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire("IMUX.CLB.G2"),
                    },
                    0,
                    1,
                ))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.egrid.db;
    let mut iob_o_diffs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    for (_, tile, node) in &intdb.tile_classes {
        for (bslot, bel) in &node.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let bel = intdb.bel_slots.key(bslot);
            let mut hiob_o_diffs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        let out_name = intdb.wires.key(mux.dst.wire);
                        let mux_name = if tile.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = mux.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };
                        let mut iob_o = None;
                        if out_name.starts_with("IMUX.IO")
                            && out_name.ends_with("O1")
                            && tile.starts_with("IO")
                        {
                            iob_o = if out_name == "IMUX.IOB0.O1" {
                                Some((&tile[..], "IO0", "O1", 0))
                            } else {
                                Some((&tile[..], "IO1", "O1", 0))
                            };
                        }
                        if (out_name == "IMUX.CLB.F4" || out_name == "IMUX.CLB.G4")
                            && tile.starts_with("IO.B")
                        {
                            iob_o = if out_name == "IMUX.CLB.F4" {
                                Some((&tile[..], "IO0", "O2", 0))
                            } else {
                                Some((&tile[..], "IO1", "O2", 0))
                            };
                        }
                        if (out_name == "IMUX.CLB.F2" || out_name == "IMUX.CLB.G2")
                            && tile.starts_with("CLB")
                            && tile.ends_with('T')
                        {
                            iob_o = if out_name == "IMUX.CLB.F2" {
                                Some(("IO.T", "IO0", "O2", 3))
                            } else {
                                Some(("IO.T", "IO1", "O2", 3))
                            };
                        }
                        if (out_name == "IMUX.CLB.F3" || out_name == "IMUX.CLB.G3")
                            && tile.starts_with("CLB.L")
                        {
                            iob_o = if out_name == "IMUX.CLB.G3" {
                                Some(("IO.L", "IO0", "O2", 2))
                            } else {
                                Some(("IO.L", "IO1", "O2", 2))
                            };
                        }
                        if (out_name == "IMUX.CLB.F1" || out_name == "IMUX.CLB.G1")
                            && tile.starts_with("IO.R")
                        {
                            iob_o = if out_name == "IMUX.CLB.G1" {
                                Some((&tile[..], "IO0", "O2", 0))
                            } else {
                                Some((&tile[..], "IO1", "O2", 0))
                            };
                        }
                        if let Some((prefix, bel, pin, bt)) = iob_o {
                            if ctx.edev.chip.kind == ChipKind::Xc4000H {
                                let mut inps = vec![];
                                let mut got_empty = false;
                                for &wire_from in &mux.src {
                                    let in_name = format!(
                                        "{:#}.{}",
                                        wire_from.cell,
                                        intdb.wires.key(wire_from.wire)
                                    );
                                    let diff0 = ctx.state.get_diff(
                                        tile,
                                        "INT",
                                        &mux_name,
                                        format!("{in_name}.HIOB0"),
                                    );
                                    let diff1 = ctx.state.get_diff(
                                        tile,
                                        "INT",
                                        &mux_name,
                                        format!("{in_name}.HIOB1"),
                                    );
                                    let (mut diff0, mut diff1, diff) = Diff::split(diff0, diff1);
                                    if diff.bits.is_empty() {
                                        got_empty = true;
                                    }
                                    inps.push((in_name.to_string(), diff));
                                    if bt != 0 {
                                        for diff in [&mut diff0, &mut diff1] {
                                            *diff = Diff {
                                                bits: diff
                                                    .bits
                                                    .iter()
                                                    .map(|(&bit, &val)| {
                                                        assert_eq!(bit.tile, bt);
                                                        (TileBit { tile: 0, ..bit }, val)
                                                    })
                                                    .collect(),
                                            };
                                        }
                                    }
                                    let hiob = if bel == "IO0" {
                                        ["HIO0", "HIO1"]
                                    } else {
                                        ["HIO2", "HIO3"]
                                    };
                                    for (bel, diff) in [(hiob[0], diff0), (hiob[1], diff1)] {
                                        match hiob_o_diffs
                                            .entry((prefix, bel))
                                            .or_default()
                                            .entry(pin)
                                        {
                                            btree_map::Entry::Vacant(entry) => {
                                                entry.insert(diff);
                                            }
                                            btree_map::Entry::Occupied(entry) => {
                                                assert_eq!(*entry.get(), diff);
                                            }
                                        }
                                    }
                                }
                                if pin == "O1" {
                                    assert!(!got_empty);
                                    inps.push(("GND".to_string(), Diff::default()));
                                } else {
                                    assert!(got_empty);
                                }
                                let item = xlat_enum_ocd(inps, OcdMode::Mux);
                                if item.bits.is_empty() {
                                    println!("UMMM MUX {tile} {mux_name} is empty");
                                }
                                ctx.tiledb.insert(tile, "INT", mux_name, item);
                            } else {
                                for suffix in ["O", "OQ", "O.NOT", "OQ.NOT"] {
                                    let mut inps = vec![];
                                    for &wire_from in &mux.src {
                                        let in_name = format!(
                                            "{:#}.{}",
                                            wire_from.cell,
                                            intdb.wires.key(wire_from.wire)
                                        );
                                        let diff = ctx.state.get_diff(
                                            tile,
                                            "INT",
                                            &mux_name,
                                            format!("{in_name}.{suffix}"),
                                        );
                                        inps.push((in_name.to_string(), diff));
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
                                        inps.push(("GND".to_string(), Diff::default()));
                                    } else {
                                        assert!(got_empty);
                                    }
                                    let item = xlat_enum_ocd(inps, OcdMode::Mux);
                                    if item.bits.is_empty() {
                                        println!("UMMM MUX {tile} {mux_name} is empty");
                                    }
                                    ctx.tiledb.insert(tile, "INT", &mux_name, item);
                                    if bt != 0 {
                                        common = Diff {
                                            bits: common
                                                .bits
                                                .iter()
                                                .map(|(&bit, &val)| {
                                                    assert_eq!(bit.tile, bt);
                                                    (TileBit { tile: 0, ..bit }, val)
                                                })
                                                .collect(),
                                        };
                                    }
                                    match iob_o_diffs
                                        .entry((&prefix[..4], bel))
                                        .or_default()
                                        .entry(format!("{pin}.{suffix}"))
                                    {
                                        btree_map::Entry::Vacant(entry) => {
                                            entry.insert(common);
                                        }
                                        btree_map::Entry::Occupied(entry) => {
                                            assert_eq!(*entry.get(), common);
                                        }
                                    }
                                }
                            }
                        } else if out_name.starts_with("IMUX.TBUF") {
                            if out_name.ends_with("I") {
                                continue;
                            }
                            let idx = if out_name == "IMUX.TBUF0.TS" {
                                0
                            } else if out_name == "IMUX.TBUF1.TS" {
                                1
                            } else {
                                unreachable!()
                            };
                            let mut t_inps = vec![];
                            for &wire_from in &mux.src {
                                let in_name = format!(
                                    "{:#}.{}",
                                    wire_from.cell,
                                    intdb.wires.key(wire_from.wire)
                                );
                                t_inps.push((
                                    in_name.to_string(),
                                    ctx.state.get_diff(tile, "INT", &mux_name, &in_name),
                                ));
                            }
                            let imux_i = TileWireCoord {
                                cell: CellSlotId::from_idx(0),
                                wire: intdb.get_wire(&format!("IMUX.TBUF{idx}.I")),
                            };
                            let mux_name_i = format!("MUX.IMUX.TBUF{idx}.I");
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
                            for &wire_from in &mux_i.src {
                                let in_name = format!(
                                    "{:#}.{}",
                                    wire_from.cell,
                                    intdb.wires.key(wire_from.wire)
                                );
                                i_inps.push((
                                    in_name.to_string(),
                                    ctx.state.get_diff(tile, "INT", &mux_name_i, &in_name),
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
                                t_inps.push(("GND".to_string(), t_diff));
                            }
                            i_inps.push(("GND".to_string(), Diff::default()));
                            let item_i = xlat_enum_ocd(i_inps, OcdMode::Mux);
                            if item_i.bits.is_empty() {
                                println!("UMMM MUX {tile} {mux_name_i} is empty");
                            }
                            ctx.tiledb.insert(tile, "INT", mux_name_i, item_i);
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
                            assert!(!got_empty, "fuckup on {tile} {mux_name}");
                            t_inps.push(("VCC".to_string(), Diff::default()));
                            let item_t = xlat_enum_ocd(t_inps, OcdMode::Mux);
                            if item_t.bits.is_empty() {
                                println!("UMMM MUX {tile} {mux_name} is empty");
                            }
                            ctx.tiledb.insert(tile, "INT", mux_name, item_t);
                            ctx.tiledb.insert(
                                tile,
                                format!("TBUF{idx}"),
                                "DRIVE1",
                                xlat_bit(!common),
                            );
                        } else {
                            let mut inps = vec![];
                            let mut got_empty = false;
                            for &wire_from in &mux.src {
                                let in_name = format!(
                                    "{:#}.{}",
                                    wire_from.cell,
                                    intdb.wires.key(wire_from.wire)
                                );
                                let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                                if diff.bits.is_empty() {
                                    got_empty = true;
                                }
                                inps.push((in_name.to_string(), diff));
                            }
                            if tile.starts_with("CLB") && out_name == "IMUX.CLB.F4" {
                                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "CIN");
                                inps.push(("CIN".to_string(), diff));
                            }
                            if (tile.starts_with("IO.B")
                                || matches!(
                                    &tile[..],
                                    "CLB" | "CLB.L" | "CLB.R" | "CLB.B" | "CLB.LB" | "CLB.RB"
                                ))
                                && out_name == "IMUX.CLB.G2"
                            {
                                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "COUT0");
                                inps.push(("COUT0".to_string(), diff));
                            }
                            if (tile.starts_with("IO.R")
                                || matches!(
                                    &tile[..],
                                    "CLB" | "CLB.B" | "CLB.T" | "CLB.R" | "CLB.RB" | "CLB.RT"
                                ))
                                && out_name == "IMUX.CLB.G3"
                            {
                                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "CIN");
                                inps.push(("CIN".to_string(), diff));
                            }
                            if out_name == "IMUX.IOB0.TS" || out_name == "IMUX.IOB1.TS" {
                                inps.push(("GND".to_string(), Diff::default()));
                                got_empty = true;
                            }

                            for (rtile, rwire, rbel, rattr) in [
                                ("CNR.BL", "IMUX.IOB1.IK", "MD1", "ENABLE.T"),
                                ("CNR.BL", "IMUX.IOB1.O1", "MD1", "ENABLE.O"),
                                ("CNR.BL", "IMUX.RDBK.TRIG", "RDBK", "ENABLE"),
                                ("CNR.BR", "IMUX.STARTUP.GTS", "STARTUP", "ENABLE.GTS"),
                                ("CNR.BR", "IMUX.STARTUP.GSR", "STARTUP", "ENABLE.GSR"),
                                ("CNR.TR", "IMUX.TDO.T", "TDO", "ENABLE.T"),
                                ("CNR.TR", "IMUX.TDO.O", "TDO", "ENABLE.O"),
                            ] {
                                if tile == rtile && out_name == rwire {
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
                                    ctx.tiledb.insert(tile, rbel, rattr, xlat_bit(common));
                                }
                            }
                            if !got_empty {
                                inps.push(("NONE".to_string(), Diff::default()));
                            }
                            let item = xlat_enum_ocd(inps, OcdMode::Mux);
                            if item.bits.is_empty() {
                                println!("UMMM MUX {tile} {mux_name} is empty");
                            }
                            ctx.tiledb.insert(tile, bel, mux_name, item);
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        let out_name = intdb.wires.key(buf.dst.wire);
                        let mux_name = if tile.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = buf.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };
                        let wfname = intdb.wires.key(buf.src.wire);
                        let in_name = format!("{:#}.{}", buf.src.cell, wfname);
                        let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                        if diff.bits.is_empty() {
                            panic!("weird lack of bits: {tile} {out_name} {wfname}");
                        }

                        let oname = if tile.starts_with("LL") {
                            format!("{:#}.{}", buf.dst.cell, out_name)
                        } else {
                            out_name.to_string()
                        };
                        let iname = format!("{:#}.{}", buf.src.cell, wfname);
                        ctx.tiledb.insert(
                            tile,
                            "INT",
                            format!("BUF.{oname}.{iname}"),
                            xlat_bit(diff),
                        );
                    }
                    SwitchBoxItem::Pass(pass) => {
                        let out_name = intdb.wires.key(pass.dst.wire);
                        let mux_name = if tile.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = pass.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };
                        let wfname = intdb.wires.key(pass.src.wire);
                        let in_name = format!("{:#}.{}", pass.src.cell, wfname);
                        let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                        if diff.bits.is_empty() {
                            panic!("weird lack of bits: {tile} {out_name} {wfname}");
                        }

                        let oname = if tile.starts_with("LL") {
                            format!("{:#}.{}", pass.dst.cell, out_name)
                        } else {
                            out_name.to_string()
                        };
                        let iname = format!("{:#}.{}", pass.src.cell, wfname);
                        ctx.tiledb.insert(
                            tile,
                            "INT",
                            format!("PASS.{oname}.{iname}"),
                            xlat_bit(diff),
                        );
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        let aname = intdb.wires.key(pass.a.wire);
                        let bname = intdb.wires.key(pass.b.wire);
                        let name = if tile.starts_with("LL") {
                            format!(
                                "BIPASS.{:#}.{}.{:#}.{}",
                                pass.a.cell, aname, pass.b.cell, bname
                            )
                        } else {
                            assert_eq!(pass.a.cell.to_idx(), 0);
                            assert_eq!(pass.b.cell.to_idx(), 0);
                            format!("BIPASS.{aname}.{bname}")
                        };

                        for (wdst, wsrc) in [(pass.a, pass.b), (pass.b, pass.a)] {
                            let out_name = intdb.wires.key(wdst.wire);
                            let mux_name = if tile.starts_with("LL") {
                                format!("MUX.{wtt:#}.{out_name}", wtt = wdst.cell)
                            } else {
                                format!("MUX.{out_name}")
                            };
                            let wfname = intdb.wires.key(wsrc.wire);
                            let in_name = format!("{:#}.{}", wsrc.cell, wfname);
                            let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);

                            assert_eq!(diff.bits.len(), 1);
                            ctx.tiledb.insert(tile, bel, &name, xlat_bit(diff));
                        }
                    }
                    _ => unreachable!(),
                }
            }

            for ((prefix, bel), diffs) in hiob_o_diffs {
                let diffs = Vec::from_iter(diffs);
                let item = xlat_enum(diffs);
                if prefix == tile {
                    ctx.tiledb.insert(tile, bel, "MUX.O", item);
                } else {
                    for tile in intdb.tile_classes.keys() {
                        if tile.starts_with(prefix) {
                            ctx.tiledb.insert(tile, bel, "MUX.O", item.clone());
                        }
                    }
                }
            }
        }
    }
    for ((prefix, bel), mut diffs) in iob_o_diffs {
        assert_eq!(diffs.len(), 8);

        let mut common = diffs.values().next().unwrap().clone();
        for diff in diffs.values() {
            common.bits.retain(|bit, _| diff.bits.contains_key(bit));
        }
        for diff in diffs.values_mut() {
            *diff = diff.combine(&!&common);
        }
        let item = xlat_bit(!common);
        for tile in intdb.tile_classes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "INV.T", item.clone());
            }
        }

        let diff_inv_off_d = diffs["O1.OQ.NOT"].combine(&!&diffs["O1.OQ"]);
        for key in ["O1.O.NOT", "O2.O.NOT", "O1.OQ.NOT", "O2.OQ.NOT"] {
            let diff = diffs.get_mut(key).unwrap();
            *diff = diff.combine(&!&diff_inv_off_d);
        }
        let item = xlat_bit(diff_inv_off_d);
        for tile in intdb.tile_classes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "INV.OFF_D", item.clone());
            }
        }
        assert_eq!(diffs["O1.OQ.NOT"], diffs["O1.OQ"]);
        assert_eq!(diffs["O2.OQ.NOT"], diffs["O2.OQ"]);
        diffs.remove("O1.OQ.NOT");
        diffs.remove("O2.OQ.NOT");

        let diff_mux_off_d_o2 = diffs["O2.OQ"].combine(&!&diffs["O1.OQ"]);
        for key in ["O2.O", "O2.O.NOT", "O2.OQ"] {
            let diff = diffs.get_mut(key).unwrap();
            *diff = diff.combine(&!&diff_mux_off_d_o2);
        }
        let item = xlat_enum(vec![("O1", Diff::default()), ("O2", diff_mux_off_d_o2)]);
        for tile in intdb.tile_classes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "MUX.OFF_D", item.clone());
            }
        }
        assert_eq!(diffs["O1.OQ"], diffs["O2.OQ"]);
        diffs.remove("O2.OQ");

        let mut diff_off_used = diffs["O1.OQ"].clone();
        for key in ["O1.O", "O1.O.NOT", "O2.O", "O2.O.NOT"] {
            let diff = &diffs[key];
            diff_off_used
                .bits
                .retain(|bit, _| !diff.bits.contains_key(bit));
        }
        let item = xlat_enum(vec![
            ("OFF", diffs["O1.OQ"].combine(&!&diff_off_used)),
            ("O1", diffs["O1.O"].clone()),
            ("O1.INV", diffs["O1.O.NOT"].clone()),
            ("O2", diffs["O2.O"].clone()),
            ("O2.INV", diffs["O2.O.NOT"].clone()),
        ]);
        for tile in intdb.tile_classes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "OMUX", item.clone());
            }
        }

        let item = xlat_bit(diff_off_used);
        for tile in intdb.tile_classes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "OFF_USED", item.clone());
            }
        }
    }
}
