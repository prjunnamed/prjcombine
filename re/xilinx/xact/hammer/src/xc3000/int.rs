use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    grid::{TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_xc2000::{bels::xc2000 as bels, tslots};
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
    fuzzer = fuzzer.base(Key::WireMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(tcrd, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
}

fn drive_wire<'a>(
    backend: &XactBackend<'a>,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: WireCoord,
    wire_avoid: &mut Vec<WireCoord>,
    root: bool,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let grid = backend.edev.chip;
    let mut cell = wire_target.cell;
    let wt = wire_target.slot;
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") || wtn == "GCLK" || wtn == "ACLK" {
        let (slot, pin) = match wtn {
            "OUT.CLB.X" => (bels::CLB, "X"),
            "OUT.CLB.Y" => (bels::CLB, "Y"),
            "OUT.BIOB0.I" => (bels::IO_S0, "I"),
            "OUT.BIOB0.Q" => (bels::IO_S0, "Q"),
            "OUT.BIOB1.I" => (bels::IO_S1, "I"),
            "OUT.BIOB1.Q" => (bels::IO_S1, "Q"),
            "OUT.TIOB0.I" => (bels::IO_N0, "I"),
            "OUT.TIOB0.Q" => (bels::IO_N0, "Q"),
            "OUT.TIOB1.I" => (bels::IO_N1, "I"),
            "OUT.TIOB1.Q" => (bels::IO_N1, "Q"),
            "OUT.LIOB0.I" => (bels::IO_W0, "I"),
            "OUT.LIOB0.Q" => (bels::IO_W0, "Q"),
            "OUT.LIOB1.I" => (bels::IO_W1, "I"),
            "OUT.LIOB1.Q" => (bels::IO_W1, "Q"),
            "OUT.RIOB0.I" => (bels::IO_E0, "I"),
            "OUT.RIOB0.Q" => (bels::IO_E0, "Q"),
            "OUT.RIOB1.I" => (bels::IO_E1, "I"),
            "OUT.RIOB1.Q" => (bels::IO_E1, "Q"),
            "OUT.OSC" => (bels::OSC, "O"),
            "OUT.CLKIOB" => (bels::CLKIOB, "I"),
            "GCLK" => {
                cell.col = grid.col_w();
                cell.row = grid.row_n();
                (bels::BUFG, "O")
            }
            "ACLK" => {
                cell.col = grid.col_e();
                cell.row = grid.row_s();
                (bels::BUFG, "O")
            }
            _ => panic!("umm {wtn}"),
        };
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[slot][0];
        if slot == bels::OSC {
            fuzzer = fuzzer.base(Key::GlobalOpt("XTALOSC".into()), "ENABLE");
            if root {
                let wires = [
                    backend.egrid.db.get_wire("SINGLE.V.R3"),
                    backend.egrid.db.get_wire("SINGLE.H.B3"),
                ];
                let wire = if wire_avoid[0].slot == wires[0] {
                    wires[1]
                } else {
                    wires[0]
                };
                let crd = backend.ngrid.int_pip(
                    tcrd,
                    TileWireCoord::new_idx(0, wire),
                    TileWireCoord::new_idx(0, wire_target.slot),
                );
                let rw = backend
                    .egrid
                    .resolve_tile_wire(tcrd, TileWireCoord::new_idx(0, wire))
                    .unwrap();
                fuzzer = fuzzer
                    .base(Key::Pip(crd), Value::FromPin(block, pin.into()))
                    .base(Key::WireMutex(rw), "OSC_HOOK");
            }
        }
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            block,
            pin,
        );
    } else if wtn.starts_with("LONG.H") {
        let slot = if wtn == "LONG.H0" {
            bels::TBUF0
        } else {
            bels::TBUF1
        };
        let pin = "O";
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let crd = backend.ngrid.bel_pip(cell.bel(slot), "O");
        let block = &ntile.bels[slot][0];
        fuzzer = fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()));
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            block,
            pin,
        );
    } else if wtn == "ACLK.V" || wtn == "GCLK.V" || wtn.starts_with("IOCLK") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).ends_with("CLK") {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if (wtn.starts_with("SINGLE") && wtn.ends_with(".STUB"))
        || wtn.starts_with("LONG")
        || wtn.starts_with("IMUX.IOCLK")
    {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("SINGLE") {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if wtn.starts_with("SINGLE.V") && !wtn.ends_with(".STUB") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("OUT")
                            || backend.egrid.db.wires.key(inp.wire).starts_with("LONG.H")
                        {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if wtn.starts_with("SINGLE.H") && !wtn.ends_with(".STUB") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if (backend.egrid.db.wires.key(inp.wire).starts_with("SINGLE.V")
                            && !backend.egrid.db.wires.key(inp.wire).ends_with(".STUB"))
                            || backend.egrid.db.wires.key(inp.wire).starts_with("OUT")
                        {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.egrid[tcrd];
                let tcls_index = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("SINGLE.H") {
                            let rwf = backend.egrid.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wire_target:?}?")
        }
    } else {
        panic!("umm wtf is {wtn}")
    };
    wire_avoid.push(wire_target);
    let nwt = backend.egrid.resolve_tile_wire(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid, false);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: WireCoord,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
    inv: bool,
) -> Fuzzer<XactBackend<'a>> {
    let grid = backend.edev.chip;
    let cell = wire.cell;
    let w = wire.slot;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if wn.starts_with("IOCLK") {
        let slot = match &wn[6..7] {
            "L" => bels::IO_W0,
            "R" => bels::IO_E0,
            "B" => bels::IO_S0,
            "T" => bels::IO_N0,
            _ => unreachable!(),
        };
        let pin = if wn.ends_with('0') { "OK" } else { "IK" };
        let (col, row) = match wn {
            "IOCLK.B0" => (grid.col_w(), grid.row_s()),
            "IOCLK.B1" => (grid.col_e(), grid.row_s()),
            "IOCLK.T0" => (grid.col_e(), grid.row_n()),
            "IOCLK.T1" => (grid.col_w(), grid.row_n()),
            "IOCLK.L0" => (grid.col_w(), grid.row_n()),
            "IOCLK.L1" => (grid.col_w(), grid.row_s()),
            "IOCLK.R0" => (grid.col_e(), grid.row_s()),
            "IOCLK.R1" => (grid.col_e(), grid.row_n()),
            _ => unreachable!(),
        };
        let cell = cell.with_cr(col, row);
        let tcrd = cell.tile(tslots::MAIN);
        let tile = &backend.egrid[tcrd];
        let tcls = &backend.egrid.db.tile_classes[tile.class];
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[slot][0];
        let bel_data = &tcls.bels[slot];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let wire_pin = bel_data.pins[pin].wires.iter().copied().next().unwrap();
        let crd = backend
            .ngrid
            .int_pip(tcrd, wire_pin, TileWireCoord::new_idx(0, wire.slot));
        if &fuzzer.info.features[0].id.tile != backend.egrid.db.tile_classes.key(tile.class) {
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: backend.egrid.db.tile_classes.key(tile.class).clone(),
                    bel: "INT".into(),
                    attr: format!("INV.{wn}"),
                    val: if inv { "1" } else { "0" }.into(),
                },
                tiles: vec![backend.edev.btile_main(col, row)],
            });
        }
        return fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BlockConfig(block, "IN".into(), "IQ".into()), true)
            .base(Key::BlockConfig(block, "IN".into(), "FF".into()), true)
            .base(Key::BlockConfig(block, "IN".into(), "LATCH".into()), false)
            .base(Key::BlockConfig(block, "OUT".into(), "O".into()), false)
            .base(Key::BlockConfig(block, "OUT".into(), "OQ".into()), true)
            .base(Key::BelMutex(cell.bel(slot), "TRI".into()), "GND")
            .fuzz(
                Key::BlockConfig(
                    block,
                    if pin == "IK" { "IN" } else { "OUT" }.into(),
                    format!("{pin}NOT"),
                ),
                false,
                inv,
            )
            .fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()))
            .fuzz(
                Key::BlockPin(block, pin.into()),
                None,
                Value::FromPin(sblock, spin.into()),
            );
    }
    if !wn.starts_with("IMUX") || wn.starts_with("IMUX.IOCLK") {
        return fuzzer;
    }
    let (slot, pin) = match wn {
        "IMUX.CLB.A" => (bels::CLB, "A"),
        "IMUX.CLB.B" => (bels::CLB, "B"),
        "IMUX.CLB.C" => (bels::CLB, "C"),
        "IMUX.CLB.D" => (bels::CLB, "D"),
        "IMUX.CLB.E" => (bels::CLB, "E"),
        "IMUX.CLB.DI" => (bels::CLB, "DI"),
        "IMUX.CLB.EC" => (bels::CLB, "EC"),
        "IMUX.CLB.RD" => (bels::CLB, "RD"),
        "IMUX.CLB.K" => (bels::CLB, "K"),
        "IMUX.BIOB0.O" => (bels::IO_S0, "O"),
        "IMUX.BIOB0.T" => (bels::IO_S0, "T"),
        "IMUX.BIOB0.IK" => (bels::IO_S0, "IK"),
        "IMUX.BIOB0.OK" => (bels::IO_S0, "OK"),
        "IMUX.BIOB1.O" => (bels::IO_S1, "O"),
        "IMUX.BIOB1.T" => (bels::IO_S1, "T"),
        "IMUX.BIOB1.IK" => (bels::IO_S1, "IK"),
        "IMUX.BIOB1.OK" => (bels::IO_S1, "OK"),
        "IMUX.TIOB0.O" => (bels::IO_N0, "O"),
        "IMUX.TIOB0.T" => (bels::IO_N0, "T"),
        "IMUX.TIOB0.IK" => (bels::IO_N0, "IK"),
        "IMUX.TIOB0.OK" => (bels::IO_N0, "OK"),
        "IMUX.TIOB1.O" => (bels::IO_N1, "O"),
        "IMUX.TIOB1.T" => (bels::IO_N1, "T"),
        "IMUX.TIOB1.IK" => (bels::IO_N1, "IK"),
        "IMUX.TIOB1.OK" => (bels::IO_N1, "OK"),
        "IMUX.LIOB0.O" => (bels::IO_W0, "O"),
        "IMUX.LIOB0.T" => (bels::IO_W0, "T"),
        "IMUX.LIOB0.IK" => (bels::IO_W0, "IK"),
        "IMUX.LIOB0.OK" => (bels::IO_W0, "OK"),
        "IMUX.LIOB1.O" => (bels::IO_W1, "O"),
        "IMUX.LIOB1.T" => (bels::IO_W1, "T"),
        "IMUX.LIOB1.IK" => (bels::IO_W1, "IK"),
        "IMUX.LIOB1.OK" => (bels::IO_W1, "OK"),
        "IMUX.RIOB0.O" => (bels::IO_E0, "O"),
        "IMUX.RIOB0.T" => (bels::IO_E0, "T"),
        "IMUX.RIOB0.IK" => (bels::IO_E0, "IK"),
        "IMUX.RIOB0.OK" => (bels::IO_E0, "OK"),
        "IMUX.RIOB1.O" => (bels::IO_E1, "O"),
        "IMUX.RIOB1.T" => (bels::IO_E1, "T"),
        "IMUX.RIOB1.IK" => (bels::IO_E1, "IK"),
        "IMUX.RIOB1.OK" => (bels::IO_E1, "OK"),
        "IMUX.TBUF0.I" => (bels::TBUF0, "I"),
        "IMUX.TBUF0.T" => (bels::TBUF0, "T"),
        "IMUX.TBUF1.I" => (bels::TBUF1, "I"),
        "IMUX.TBUF1.T" => (bels::TBUF1, "T"),
        "IMUX.TBUF2.I" => (bels::TBUF0_E, "I"),
        "IMUX.TBUF2.T" => (bels::TBUF0_E, "T"),
        "IMUX.TBUF3.I" => (bels::TBUF1_E, "I"),
        "IMUX.TBUF3.T" => (bels::TBUF1_E, "T"),
        "IMUX.BUFG" => (bels::BUFG, "I"),
        _ => panic!("umm {wn}?"),
    };
    let tcrd = cell.tile(tslots::MAIN);
    let ntile = &backend.ngrid.tiles[&tcrd];
    let block = &ntile.bels[slot][0];
    if pin == "T" && wn.contains("IOB") {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BelMutex(cell.bel(slot), "TRI".into()), "T")
            .fuzz(
                Key::BlockConfig(block, "TRI".into(), "T".into()),
                false,
                true,
            );
    }
    if pin == "IK" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BlockConfig(block, "IN".into(), "IQ".into()), true)
            .base(Key::BlockConfig(block, "IN".into(), "FF".into()), true)
            .base(Key::BlockConfig(block, "IN".into(), "LATCH".into()), false)
            .base(Key::BlockConfig(block, "IN".into(), "IKNOT".into()), false);
    }
    if pin == "OK" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BlockConfig(block, "OUT".into(), "O".into()), false)
            .base(Key::BlockConfig(block, "OUT".into(), "OQ".into()), true)
            .base(Key::BelMutex(cell.bel(slot), "TRI".into()), "GND")
            .base(Key::BlockConfig(block, "OUT".into(), "OKNOT".into()), false);
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
    inv: bool,
}

impl IntPip {
    pub fn new(wire_to: TileWireCoord, wire_from: TileWireCoord, inv: bool) -> Self {
        Self {
            wire_to,
            wire_from,
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
        let mut wire_avoid = vec![rwt];
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, &mut wire_avoid, true);
        fuzzer = fuzzer.fuzz(Key::WireMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(tcrd, self.wire_to, self.wire_from);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin, self.inv);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ProhibitInt {
    wire: TileWireCoord,
}

impl ProhibitInt {
    pub fn new(wire: TileWireCoord) -> Self {
        Self { wire }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for ProhibitInt {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: prjcombine_interconnect::grid::TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let rw = backend.egrid.resolve_tile_wire(tcrd, self.wire).unwrap();
        fuzzer = fuzzer.base(Key::WireMutex(rw), "PROHIBIT");
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.egrid.db;
    for (tcid, tile, tcls) in &intdb.tile_classes {
        let tcls_index = &backend.egrid.db_index.tile_classes[tcid];
        if tcls_index.pips_bwd.is_empty() {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let wire_to_name = intdb.wires.key(wire_to.wire);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{:#}.{}", wire_to.cell, wire_to_name)
            } else {
                format!("MUX.{wire_to_name}")
            };
            for &wire_from in ins {
                let wire_from = wire_from.tw;
                let wire_from_name = intdb.wires.key(wire_from.wire);
                let in_name = format!("{:#}.{}", wire_from.cell, wire_from_name);
                ctx.build()
                    .test_manual("INT", &mux_name, &in_name)
                    .prop(IntPip::new(wire_to, wire_from, false))
                    .commit();
                if wire_to_name.starts_with("IOCLK") {
                    ctx.build()
                        .test_manual("INT", &mux_name, format!("{in_name}.INV"))
                        .prop(IntPip::new(wire_to, wire_from, true))
                        .commit();
                }
            }
            if mux_name.contains("TBUF") && mux_name.ends_with('I') {
                let long_name = if mux_name.contains("TBUF0") || mux_name.contains("TBUF2") {
                    "LONG.H0"
                } else {
                    "LONG.H1"
                };
                let t_name = format!("{}.T", &intdb.wires.key(wire_to.wire)[..10]);
                let wire_long = TileWireCoord::new_idx(0, intdb.get_wire(long_name));
                let wire_t = TileWireCoord::new_idx(0, intdb.get_wire(&t_name));
                for &wire_from in ins {
                    let wire_from = wire_from.tw;
                    let wire_from_name = intdb.wires.key(wire_from.wire);
                    if !wire_from_name.starts_with("SINGLE") {
                        continue;
                    }
                    ctx.build()
                        .test_manual("INT", format!("MUX.{t_name}"), "GND")
                        .prop(ProhibitInt::new(wire_to))
                        .prop(ProhibitInt::new(wire_t))
                        .prop(IntPip::new(wire_long, wire_from, false))
                        .commit();
                }
            }
        }
        for slot in tcls.bels.ids() {
            let slot_name = backend.egrid.db.bel_slots.key(slot).as_str();
            if slot_name.starts_with("IO") {
                let mut bctx = ctx.bel(slot);
                bctx.mode("IO")
                    .global("XTALOSC", "DISABLE")
                    .test_cfg("IN", "I");
                bctx.mode("IO")
                    .global("XTALOSC", "DISABLE")
                    .cfg("IN", "I")
                    .mutex("TRI", "PULLUP")
                    .test_cfg("IN", "PULLUP");
                bctx.mode("IO")
                    .cfg("IN", "I")
                    .mutex("TRI", "GND")
                    .mutex("OUT", "O")
                    .test_cfg("OUT", "O");
            }
            if slot_name.starts_with("PULLUP_TBUF") {
                let mut bctx = ctx.bel(slot);
                bctx.build()
                    .pin_mutex_exclusive("O")
                    .test_manual("ENABLE", "1")
                    .pip_pin("O", "O")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.egrid.db;
    let grid = ctx.edev.chip;
    for (_, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcname) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let bel = intdb.bel_slots.key(bslot);
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        let out_name = intdb.wires.key(mux.dst.wire);
                        let mux_name = if tcname.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = mux.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };
                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in &mux.src {
                            let in_name =
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                            let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((in_name.to_string(), diff));
                        }
                        if out_name.ends_with(".T") {
                            if out_name.starts_with("IMUX.TBUF") {
                                let diff = ctx.state.get_diff(tcname, "INT", &mux_name, "GND");
                                inps.push(("GND".to_string(), diff));
                            } else {
                                let slot = match &out_name[5..10] {
                                    "LIOB0" => bels::IO_W[0],
                                    "LIOB1" => bels::IO_W[1],
                                    "RIOB0" => bels::IO_E[0],
                                    "RIOB1" => bels::IO_E[1],
                                    "BIOB0" => bels::IO_S[0],
                                    "BIOB1" => bels::IO_S[1],
                                    "TIOB0" => bels::IO_N[0],
                                    "TIOB1" => bels::IO_N[1],
                                    _ => unreachable!(),
                                };
                                let bel = ctx.edev.egrid.db.bel_slots.key(slot).as_str();
                                let diff = ctx.state.get_diff(tcname, bel, "OUT", "O");
                                inps.push(("GND".to_string(), diff));

                                let mut diff_i = ctx.state.get_diff(tcname, bel, "IN", "I");
                                let mut diff_pullup =
                                    ctx.state.get_diff(tcname, bel, "IN", "PULLUP");
                                if tcname.starts_with("CLB.BR")
                                    && (bel == "IO_S1" || bel == "IO_E0")
                                {
                                    let mut diff_i_spec = Diff::default();
                                    for (&k, &v) in &diff_i.bits {
                                        if !v {
                                            diff_i_spec.bits.insert(k, v);
                                        }
                                    }
                                    diff_i = diff_i.combine(&!&diff_i_spec);
                                    diff_pullup = diff_pullup.combine(&diff_i_spec);
                                    // umm what is this actually
                                    ctx.tiledb.insert(
                                        tcname,
                                        bel,
                                        "PULLUP",
                                        xlat_bit(!diff_i_spec),
                                    );
                                }
                                assert_eq!(diff_i, !&diff_pullup);
                                inps.push(("PULLUP".to_string(), diff_pullup));
                            }
                            inps.push(("VCC".to_string(), Diff::default()));
                            got_empty = true;
                        }
                        if out_name.starts_with("IMUX.IOCLK") {
                            let val = match (&tcname[..6], &out_name[..], grid.is_small) {
                                ("CLB.BL", "IMUX.IOCLK0", false) => "GCLK",
                                ("CLB.BL", "IMUX.IOCLK1", false) => "ACLK",
                                ("CLB.BR", "IMUX.IOCLK0", false) => "ACLK",
                                ("CLB.BR", "IMUX.IOCLK1", false) => "ACLK",
                                ("CLB.TL", "IMUX.IOCLK0", false) => "GCLK",
                                ("CLB.TL", "IMUX.IOCLK1", false) => "GCLK",
                                ("CLB.TR", "IMUX.IOCLK0", false) => "ACLK",
                                ("CLB.TR", "IMUX.IOCLK1", false) => "GCLK",
                                (_, "IMUX.IOCLK0", true) => "ACLK",
                                (_, "IMUX.IOCLK1", true) => "GCLK",
                                _ => unreachable!(),
                            };
                            inps.push((val.to_string(), Diff::default()));
                            got_empty = true;
                        }
                        if !got_empty {
                            assert!(mux_name.contains("IOB"));
                            assert!(mux_name.ends_with(".O"));
                            inps.push(("NONE".to_string(), Diff::default()));
                        }
                        let item = xlat_enum_ocd(inps, OcdMode::Mux);
                        if item.bits.is_empty() {
                            if mux_name.contains("TBUF") && mux_name.ends_with(".I") {
                                // OK.
                                continue;
                            }
                            println!("UMMM MUX {tcname} {mux_name} is empty");
                        }
                        ctx.tiledb.insert(tcname, bel, mux_name, item);
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        let out_name = intdb.wires.key(buf.dst.wire);
                        let mux_name = if tcname.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = buf.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };
                        let wfname = intdb.wires.key(buf.src.wire);
                        let in_name = format!("{:#}.{}", buf.src.cell, wfname);
                        let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                        if out_name.starts_with("IOCLK") {
                            let empty = grid.is_small
                                && matches!(
                                    &out_name[..],
                                    "IOCLK.B0" | "IOCLK.B1" | "IOCLK.L0" | "IOCLK.L1"
                                );
                            diff.assert_empty();
                            let diff = ctx.state.get_diff(
                                tcname,
                                "INT",
                                &mux_name,
                                format!("{in_name}.INV"),
                            );
                            if in_name.ends_with("CLK") && empty {
                                diff.assert_empty();
                            } else {
                                let item = xlat_bit(diff);
                                ctx.tiledb
                                    .insert(tcname, bel, format!("INV.{out_name}"), item);
                            }
                        } else {
                            if diff.bits.is_empty() {
                                panic!("weird lack of bits: {tcname} {out_name} {wfname}");
                            }

                            assert_eq!(diff.bits.len(), 1);
                            let oname = if tcname.starts_with("LL") {
                                format!("{:#}.{}", buf.dst.cell, out_name)
                            } else {
                                out_name.to_string()
                            };
                            let iname = format!("{:#}.{}", buf.src.cell, wfname);
                            ctx.tiledb.insert(
                                tcname,
                                bel,
                                format!("BUF.{oname}.{iname}"),
                                xlat_bit(diff),
                            );
                        }
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        let out_name = intdb.wires.key(buf.dst.wire);
                        let mux_name = if tcname.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = buf.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };
                        let in_name =
                            format!("{:#}.{}", buf.src.cell, intdb.wires.key(buf.src.wire));
                        let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                        diff.assert_empty();
                    }
                    SwitchBoxItem::Pass(pass) => {
                        let out_name = intdb.wires.key(pass.dst.wire);
                        let mux_name = if tcname.starts_with("LL") {
                            format!("MUX.{wtt:#}.{out_name}", wtt = pass.dst.cell)
                        } else {
                            format!("MUX.{out_name}")
                        };

                        let wfname = intdb.wires.key(pass.src.wire);
                        let in_name = format!("{:#}.{}", pass.src.cell, wfname);
                        let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                        if diff.bits.is_empty() {
                            panic!("weird lack of bits: {tcname} {out_name} {wfname}");
                        }

                        assert_eq!(diff.bits.len(), 1);
                        let oname = if tcname.starts_with("LL") {
                            format!("{:#}.{}", pass.dst.cell, out_name)
                        } else {
                            out_name.to_string()
                        };
                        let iname = format!("{:#}.{}", pass.src.cell, wfname);
                        ctx.tiledb.insert(
                            tcname,
                            bel,
                            format!("PASS.{oname}.{iname}"),
                            xlat_bit(diff),
                        );
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        let aname = intdb.wires.key(pass.a.wire);
                        let bname = intdb.wires.key(pass.b.wire);
                        let name = if tcname.starts_with("LL") {
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
                            let mux_name = if tcname.starts_with("LL") {
                                format!("MUX.{wtt:#}.{out_name}", wtt = wdst.cell)
                            } else {
                                format!("MUX.{out_name}")
                            };

                            let wfname = intdb.wires.key(wsrc.wire);
                            let in_name = format!("{:#}.{}", wsrc.cell, wfname);
                            let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);

                            assert_eq!(diff.bits.len(), 1);
                            ctx.tiledb.insert(tcname, bel, &name, xlat_bit(diff));
                        }
                    }
                    SwitchBoxItem::ProgInv(_) => (),
                    _ => unreachable!(),
                }
            }
        }

        if tcname.starts_with("CLB.BLS") {
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK.B0", "0", "1");
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK.L1", "0", "1");
        }
        if tcname.starts_with("CLB.BRS") {
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK.B1", "0", "1");
        }
        if tcname.starts_with("CLB.TLS") {
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK.L0", "0", "1");
        }

        for slot in tcls.bels.ids() {
            let bel = ctx.edev.egrid.db.bel_slots.key(slot);
            if bel.starts_with("PULLUP_TBUF") {
                ctx.collect_bit(tcname, bel, "ENABLE", "1");
            }
        }
    }
}
