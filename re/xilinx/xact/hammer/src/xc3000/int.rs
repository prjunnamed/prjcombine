use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::{BelInfo, BelKind, BelSlotId, SwitchBoxItem, TileWireCoord, WireSlotId},
    dir::Dir,
    grid::{BelCoord, TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, DiffKey, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_xc2000::xc2000::{
    bslots, tslots,
    xc3000::{bcls, wires},
};

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
    let rwf = backend.edev.resolve_tile_wire(tcrd, wire_from).unwrap();
    let rwt = backend.edev.resolve_tile_wire(tcrd, wire_to).unwrap();
    fuzzer = fuzzer.base(Key::WireMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(tcrd, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
}

fn wire_to_outpin(backend: &XactBackend, wire: WireCoord) -> Option<(BelCoord, &'static str)> {
    let chip = backend.edev.chip;
    match wire.slot {
        wires::OUT_CLB_X => Some((wire.bel(bslots::CLB), "X")),
        wires::OUT_CLB_Y => Some((wire.bel(bslots::CLB), "Y")),
        wires::OUT_OSC => Some((wire.bel(bslots::OSC), "O")),
        wires::OUT_CLKIOB => Some((wire.bel(bslots::CLKIOB), "I")),
        wires::GCLK => Some((
            wire.with_cr(chip.col_w(), chip.row_n()).bel(bslots::BUFG),
            "O",
        )),
        wires::ACLK => Some((
            wire.with_cr(chip.col_e(), chip.row_s()).bel(bslots::BUFG),
            "O",
        )),
        _ => {
            if let Some(idx) = wires::OUT_IO_W_I.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_W[idx]), "I"))
            } else if let Some(idx) = wires::OUT_IO_E_I.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_E[idx]), "I"))
            } else if let Some(idx) = wires::OUT_IO_S_I.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_S[idx]), "I"))
            } else if let Some(idx) = wires::OUT_IO_N_I.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_N[idx]), "I"))
            } else if let Some(idx) = wires::OUT_IO_W_Q.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_W[idx]), "Q"))
            } else if let Some(idx) = wires::OUT_IO_E_Q.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_E[idx]), "Q"))
            } else if let Some(idx) = wires::OUT_IO_S_Q.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_S[idx]), "Q"))
            } else if let Some(idx) = wires::OUT_IO_N_Q.index_of(wire.slot) {
                Some((wire.bel(bslots::IO_N[idx]), "Q"))
            } else {
                None
            }
        }
    }
}

fn wire_to_inpin(wire: WireSlotId) -> Option<(BelSlotId, &'static str)> {
    match wire {
        wires::IMUX_CLB_A => Some((bslots::CLB, "A")),
        wires::IMUX_CLB_B => Some((bslots::CLB, "B")),
        wires::IMUX_CLB_C => Some((bslots::CLB, "C")),
        wires::IMUX_CLB_D => Some((bslots::CLB, "D")),
        wires::IMUX_CLB_E => Some((bslots::CLB, "E")),
        wires::IMUX_CLB_DI => Some((bslots::CLB, "DI")),
        wires::IMUX_CLB_EC => Some((bslots::CLB, "EC")),
        wires::IMUX_CLB_RD => Some((bslots::CLB, "RD")),
        wires::IMUX_CLB_K => Some((bslots::CLB, "K")),
        wires::IMUX_BUFG => Some((bslots::BUFG, "I")),
        _ => {
            if let Some(idx) = wires::IMUX_IO_W_O.index_of(wire) {
                Some((bslots::IO_W[idx], "O"))
            } else if let Some(idx) = wires::IMUX_IO_E_O.index_of(wire) {
                Some((bslots::IO_E[idx], "O"))
            } else if let Some(idx) = wires::IMUX_IO_S_O.index_of(wire) {
                Some((bslots::IO_S[idx], "O"))
            } else if let Some(idx) = wires::IMUX_IO_N_O.index_of(wire) {
                Some((bslots::IO_N[idx], "O"))
            } else if let Some(idx) = wires::IMUX_IO_W_T.index_of(wire) {
                Some((bslots::IO_W[idx], "T"))
            } else if let Some(idx) = wires::IMUX_IO_E_T.index_of(wire) {
                Some((bslots::IO_E[idx], "T"))
            } else if let Some(idx) = wires::IMUX_IO_S_T.index_of(wire) {
                Some((bslots::IO_S[idx], "T"))
            } else if let Some(idx) = wires::IMUX_IO_N_T.index_of(wire) {
                Some((bslots::IO_N[idx], "T"))
            } else if let Some(idx) = wires::IMUX_IO_W_IK.index_of(wire) {
                Some((bslots::IO_W[idx], "IK"))
            } else if let Some(idx) = wires::IMUX_IO_E_IK.index_of(wire) {
                Some((bslots::IO_E[idx], "IK"))
            } else if let Some(idx) = wires::IMUX_IO_S_IK.index_of(wire) {
                Some((bslots::IO_S[idx], "IK"))
            } else if let Some(idx) = wires::IMUX_IO_N_IK.index_of(wire) {
                Some((bslots::IO_N[idx], "IK"))
            } else if let Some(idx) = wires::IMUX_IO_W_OK.index_of(wire) {
                Some((bslots::IO_W[idx], "OK"))
            } else if let Some(idx) = wires::IMUX_IO_E_OK.index_of(wire) {
                Some((bslots::IO_E[idx], "OK"))
            } else if let Some(idx) = wires::IMUX_IO_S_OK.index_of(wire) {
                Some((bslots::IO_S[idx], "OK"))
            } else if let Some(idx) = wires::IMUX_IO_N_OK.index_of(wire) {
                Some((bslots::IO_N[idx], "OK"))
            } else if let Some(idx) = wires::IMUX_TBUF_I.index_of(wire) {
                Some((
                    if idx < 2 {
                        bslots::TBUF[idx]
                    } else {
                        bslots::TBUF_E[idx - 2]
                    },
                    "I",
                ))
            } else if let Some(idx) = wires::IMUX_TBUF_T.index_of(wire) {
                Some((
                    if idx < 2 {
                        bslots::TBUF[idx]
                    } else {
                        bslots::TBUF_E[idx - 2]
                    },
                    "T",
                ))
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
    wire_avoid: &mut Vec<WireCoord>,
    root: bool,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let cell = wire_target.cell;
    let wt = wire_target.slot;
    let wtn = &backend.edev.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if let Some((bel, pin)) = wire_to_outpin(backend, wire_target) {
        let tcrd = backend.edev.get_tile_by_bel(bel);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[bel.slot][0];
        if bel.slot == bslots::OSC {
            fuzzer = fuzzer.base(Key::GlobalOpt("XTALOSC".into()), "ENABLE");
            if root {
                let wires = [wires::SINGLE_VE[3], wires::SINGLE_HS[3]];
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
                    .edev
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
    } else if let Some(idx) = wires::LONG_H.index_of(wire_target.slot) {
        let slot = bslots::TBUF[idx];
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
    } else if matches!(wire_target.slot, wires::ACLK_V | wires::GCLK_V) || wtn.starts_with("IOCLK")
    {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if matches!(inp.wire, wires::GCLK | wires::ACLK) {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if (wtn.starts_with("SINGLE") && wtn.contains("_STUB"))
        || wtn.starts_with("LONG")
        || wtn.starts_with("IMUX_IOCLK")
    {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("SINGLE") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if wtn.starts_with("SINGLE_V") && !wtn.contains("_STUB") {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("OUT")
                            || backend.edev.db.wires.key(inp.wire).starts_with("LONG_H")
                        {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if wtn.starts_with("SINGLE_H") && !wtn.contains("_STUB") {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if (backend.edev.db.wires.key(inp.wire).starts_with("SINGLE_V")
                            && !backend.edev.db.wires.key(inp.wire).contains("_STUB"))
                            || backend.edev.db.wires.key(inp.wire).starts_with("OUT")
                        {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("SINGLE_H") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
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
    let nwt = backend.edev.resolve_tile_wire(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid, false);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn wire_as_ioclk(wire: WireSlotId) -> Option<(Dir, usize)> {
    if let Some(idx) = wires::IOCLK_W.index_of(wire) {
        Some((Dir::W, idx))
    } else if let Some(idx) = wires::IOCLK_E.index_of(wire) {
        Some((Dir::E, idx))
    } else if let Some(idx) = wires::IOCLK_S.index_of(wire) {
        Some((Dir::S, idx))
    } else if let Some(idx) = wires::IOCLK_N.index_of(wire) {
        Some((Dir::N, idx))
    } else {
        None
    }
}

fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: WireCoord,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
    inv: bool,
) -> Fuzzer<XactBackend<'a>> {
    let chip = backend.edev.chip;
    let cell = wire.cell;
    let w = wire.slot;
    let wn = &backend.edev.db.wires.key(w)[..];
    if let Some((side, idx)) = wire_as_ioclk(wire.slot) {
        let slot = match side {
            Dir::W => bslots::IO_W[0],
            Dir::E => bslots::IO_E[0],
            Dir::S => bslots::IO_S[0],
            Dir::N => bslots::IO_N[0],
        };
        let pin = if idx == 0 { bcls::IO::OK } else { bcls::IO::IK };
        let (col, row) = match (side, idx) {
            (Dir::W, 0) => (chip.col_w(), chip.row_n()),
            (Dir::W, 1) => (chip.col_w(), chip.row_s()),
            (Dir::E, 0) => (chip.col_e(), chip.row_s()),
            (Dir::E, 1) => (chip.col_e(), chip.row_n()),
            (Dir::S, 0) => (chip.col_w(), chip.row_s()),
            (Dir::S, 1) => (chip.col_e(), chip.row_s()),
            (Dir::N, 0) => (chip.col_e(), chip.row_n()),
            (Dir::N, 1) => (chip.col_w(), chip.row_n()),
            _ => unreachable!(),
        };
        let cell = cell.with_cr(col, row);
        let tcrd = cell.tile(tslots::MAIN);
        let tile = &backend.edev[tcrd];
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[slot][0];
        let wire_pin = backend.edev.get_bel_input(cell.bel(slot), pin);
        let wire_pin = TileWireCoord::new_idx(0, wire_pin.slot);
        let crd = backend
            .ngrid
            .int_pip(tcrd, wire_pin, TileWireCoord::new_idx(0, wire.slot));
        if let DiffKey::Legacy(ref id) = fuzzer.info.features[0].key
            && &id.tile != backend.edev.db.tile_classes.key(tile.class)
        {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: backend.edev.db.tile_classes.key(tile.class).clone(),
                    bel: "INT".into(),
                    attr: format!("INV.{wn}"),
                    val: if inv { "1" } else { "0" }.into(),
                }),
                rects: EntityVec::from_iter([backend.edev.btile_main(col, row)]),
            });
        }
        let pin_name = backend.edev.db.bel_classes[bcls::IO].inputs.key(pin).0;
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
                    if pin == bcls::IO::IK { "IN" } else { "OUT" }.into(),
                    format!("{pin_name}NOT"),
                ),
                false,
                inv,
            )
            .fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()))
            .fuzz(
                Key::BlockPin(block, pin_name.to_string()),
                None,
                Value::FromPin(sblock, spin.into()),
            );
    }
    let Some((slot, pin)) = wire_to_inpin(wire.slot) else {
        return fuzzer;
    };
    let bel = wire.bel(slot);
    let tcrd = backend.edev.get_tile_by_bel(bel);
    let ntile = &backend.ngrid.tiles[&tcrd];
    let block = &ntile.bels[bel.slot][0];
    if pin == "T" && wn.starts_with("IMUX_IO") {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BelMutex(bel, "TRI".into()), "T")
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
            .base(Key::BelMutex(bel, "TRI".into()), "GND")
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
        let rwt = backend.edev.resolve_tile_wire(tcrd, self.wire_to).unwrap();
        let rwf = backend
            .edev
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
        let rw = backend.edev.resolve_tile_wire(tcrd, self.wire).unwrap();
        fuzzer = fuzzer.base(Key::WireMutex(rw), "PROHIBIT");
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, tile, tcls) in &intdb.tile_classes {
        let tcls_index = &backend.edev.db_index[tcid];
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
                if wire_from.wire == wires::IMUX_BUFG {
                    continue;
                }
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
            if let Some(idx) = wires::IMUX_TBUF_I.index_of(wire_to.wire) {
                let wire_long = wires::LONG_H[idx % 2];
                let wire_t = wires::IMUX_TBUF_T[idx];
                let wire_long = TileWireCoord::new_idx(0, wire_long);
                let wire_t = TileWireCoord::new_idx(0, wire_t);
                let t_name = intdb.wires.key(wire_t.wire);
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
            if backend.edev.db.bel_slots[slot].kind == BelKind::Class(bcls::IO) {
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
            if backend.edev.db.bel_slots[slot].kind == BelKind::Class(bcls::PULLUP) {
                let mut bctx = ctx.bel(slot);
                bctx.build()
                    .bidir_mutex_exclusive(bcls::PULLUP::O)
                    .test_manual("ENABLE", "1")
                    .pip_pin("O", "O")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    let grid = ctx.edev.chip;
    for (_, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcname) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            if bslot == bslots::BUFG {
                continue;
            }
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
                        for &wire_from in mux.src.keys() {
                            let in_name =
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                            let diff = ctx.state.get_diff(tcname, "INT", &mux_name, &in_name);
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((in_name.to_string(), diff));
                        }
                        if wires::IMUX_TBUF_T.contains(mux.dst.wire) {
                            let diff = ctx.state.get_diff(tcname, "INT", &mux_name, "GND");
                            inps.push(("GND".to_string(), diff));
                            inps.push(("VCC".to_string(), Diff::default()));
                            got_empty = true;
                        } else if let Some((slot, pin)) = wire_to_inpin(mux.dst.wire)
                            && pin == "T"
                        {
                            let bel = ctx.edev.db.bel_slots.key(slot).as_str();
                            let diff = ctx.state.get_diff(tcname, bel, "OUT", "O");
                            inps.push(("GND".to_string(), diff));

                            let mut diff_i = ctx.state.get_diff(tcname, bel, "IN", "I");
                            let mut diff_pullup = ctx.state.get_diff(tcname, bel, "IN", "PULLUP");
                            if tcname.starts_with("CLB_SE")
                                && (slot == bslots::IO_S[1] || slot == bslots::IO_E[0])
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
                                ctx.tiledb
                                    .insert(tcname, bel, "PULLUP", xlat_bit(!diff_i_spec));
                            }
                            assert_eq!(diff_i, !&diff_pullup);
                            inps.push(("PULLUP".to_string(), diff_pullup));
                            inps.push(("VCC".to_string(), Diff::default()));
                            got_empty = true;
                        }
                        if let Some(idx) = wires::IMUX_IOCLK.index_of(mux.dst.wire) {
                            let val = match (&tcname[..6], idx, grid.is_small) {
                                ("CLB_SW", 0, false) => "GCLK",
                                ("CLB_SW", 1, false) => "ACLK",
                                ("CLB_SE", 0, false) => "ACLK",
                                ("CLB_SE", 1, false) => "ACLK",
                                ("CLB_NW", 0, false) => "GCLK",
                                ("CLB_NW", 1, false) => "GCLK",
                                ("CLB_NE", 0, false) => "ACLK",
                                ("CLB_NE", 1, false) => "GCLK",
                                (_, 0, true) => "ACLK",
                                (_, 1, true) => "GCLK",
                                _ => unreachable!(),
                            };
                            inps.push((val.to_string(), Diff::default()));
                            got_empty = true;
                        }
                        if !got_empty {
                            let Some((_slot, pin)) = wire_to_inpin(mux.dst.wire) else {
                                unreachable!()
                            };
                            assert_eq!(pin, "O");
                            inps.push(("NONE".to_string(), Diff::default()));
                        }
                        let item = xlat_enum_ocd(inps, OcdMode::Mux);
                        if item.bits.is_empty() {
                            if wires::IMUX_TBUF_I.contains(mux.dst.wire) {
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
                                && (wires::IOCLK_S.contains(buf.dst.wire)
                                    || wires::IOCLK_W.contains(buf.dst.wire));
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

        if tcname == "CLB_SW2_S" {
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK_S[0]", "0", "1");
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK_W[1]", "0", "1");
        }
        if tcname == "CLB_SE0_S" {
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK_S[1]", "0", "1");
        }
        if tcname == "CLB_NW0_S" {
            ctx.collect_enum_bool(tcname, "INT", "INV.IOCLK_W[0]", "0", "1");
        }

        for slot in tcls.bels.ids() {
            let bel = ctx.edev.db.bel_slots.key(slot);
            if bel.starts_with("PULLUP_TBUF") {
                ctx.collect_bit(tcname, bel, "ENABLE", "1");
            }
        }
    }
}
