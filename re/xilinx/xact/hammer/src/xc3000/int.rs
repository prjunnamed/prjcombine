use prjcombine_interconnect::{
    db::{BelInfo, BelKind, BelSlotId, PolTileWireCoord, SwitchBoxItem, TileWireCoord, WireSlotId},
    dir::Dir,
    grid::{BelCoord, TileCoord, WireCoord},
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit, xlat_enum_raw};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_xc2000::xc3000::{bcls, bslots, tslots, wires};

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
        let tcrd = backend.edev.bel_tile(bel);
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
    } else if matches!(wire_target.slot, wires::ACLK_V | wires::GCLK_V)
        || wire_as_ioclk(wire_target.slot).is_some()
    {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                let rw = if let Some((_, idx)) = wire_as_ioclk(wire_target.slot) {
                    if tcls_index
                        .pips_bwd
                        .contains_key(&TileWireCoord::new_idx(0, w.slot))
                    {
                        w.wire(wires::IMUX_IOCLK[idx])
                    } else {
                        continue;
                    }
                } else {
                    w
                };
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, rw.slot)) {
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
        let ntile = &backend.ngrid.tiles[&tcrd];
        let block = &ntile.bels[slot][0];
        let wire_pin = backend.edev.get_bel_input(cell.bel(slot), pin);
        let wire_pin = TileWireCoord::new_idx(0, wire_pin.slot);
        let crd = backend
            .ngrid
            .int_pip(tcrd, wire_pin, TileWireCoord::new_idx(0, wire.slot));
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
    let tcrd = backend.edev.bel_tile(bel);
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
    wire_from: PolTileWireCoord,
}

impl IntPip {
    pub fn new(wire_to: TileWireCoord, wire_from: PolTileWireCoord) -> Self {
        Self { wire_to, wire_from }
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
            .resolve_tile_wire(tcrd, self.wire_from.tw)
            .unwrap();
        let mut wire_avoid = vec![rwt];
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, &mut wire_avoid, true);
        fuzzer = fuzzer.fuzz(Key::WireMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(tcrd, self.wire_to, self.wire_from.tw);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin, self.wire_from.inv);
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
    for (tcid, _, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for bel in tcls.bels.values() {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        for &src in mux.src.keys() {
                            if wires::IMUX_IOCLK.contains(mux.dst.wire)
                                && matches!(src.wire, wires::ACLK | wires::GCLK)
                            {
                                let ioclk = sb
                                    .items
                                    .iter()
                                    .find_map(|item| {
                                        if let SwitchBoxItem::ProgInv(inv) = item {
                                            if inv.src == mux.dst {
                                                Some(inv.dst)
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap();
                                for (wt, wf) in [(ioclk, src), (ioclk, mux.dst.pos())] {
                                    ctx.build()
                                        .test_raw(DiffKey::RoutingInv(tcid, ioclk, false))
                                        .prop(IntPip::new(wt, wf))
                                        .commit();
                                    ctx.build()
                                        .test_raw(DiffKey::RoutingInv(tcid, ioclk, true))
                                        .prop(IntPip::new(wt, !wf))
                                        .commit();
                                }
                                continue;
                            }
                            if matches!(
                                src.wire,
                                wires::TIE_0 | wires::TIE_1 | wires::SPECIAL_IO_PULLUP
                            ) {
                                continue;
                            }
                            ctx.build()
                                .test_raw(DiffKey::Routing(tcid, mux.dst, src))
                                .prop(IntPip::new(mux.dst, src))
                                .commit();
                        }
                        if let Some(idx) = wires::IMUX_TBUF_I.index_of(mux.dst.wire) {
                            let wire_long = wires::LONG_H[idx % 2];
                            let wire_t = wires::IMUX_TBUF_T[idx];
                            let wire_long = TileWireCoord::new_idx(0, wire_long);
                            let wire_t = TileWireCoord::new_idx(0, wire_t);
                            for &src in mux.src.keys() {
                                let wire_from_name = intdb.wires.key(src.wire);
                                if !wire_from_name.starts_with("SINGLE") {
                                    continue;
                                }
                                ctx.build()
                                    .test_raw(DiffKey::Routing(
                                        tcid,
                                        wire_t,
                                        TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                                    ))
                                    .prop(ProhibitInt::new(mux.dst))
                                    .prop(ProhibitInt::new(wire_t))
                                    .prop(IntPip::new(wire_long, src))
                                    .commit();
                            }
                        }
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        ctx.build()
                            .test_raw(DiffKey::Routing(tcid, buf.dst, buf.src))
                            .prop(IntPip::new(buf.dst, buf.src))
                            .commit();
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        if buf.src.wire == wires::IMUX_BUFG {
                            continue;
                        }
                        ctx.build()
                            .null_bits()
                            .test_raw(DiffKey::Routing(tcid, buf.dst, buf.src))
                            .prop(IntPip::new(buf.dst, buf.src))
                            .commit();
                    }
                    SwitchBoxItem::Pass(pass) => {
                        ctx.build()
                            .test_raw(DiffKey::Routing(tcid, pass.dst, pass.src.pos()))
                            .prop(IntPip::new(pass.dst, pass.src.pos()))
                            .commit();
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.build()
                            .test_raw(DiffKey::Routing(tcid, pass.a, pass.b.pos()))
                            .prop(IntPip::new(pass.a, pass.b.pos()))
                            .commit();
                        ctx.build()
                            .test_raw(DiffKey::Routing(tcid, pass.b, pass.a.pos()))
                            .prop(IntPip::new(pass.b, pass.a.pos()))
                            .commit();
                    }
                    SwitchBoxItem::ProgInv(_) => {
                        // handled in mux code
                    }
                    _ => unreachable!(),
                }
            }
        }
        for slot in tcls.bels.ids() {
            if backend.edev.db.bel_slots[slot].kind == BelKind::Class(bcls::IO) {
                let mut bctx = ctx.bel(slot);
                bctx.mode("IO")
                    .global("XTALOSC", "DISABLE")
                    .test_bel_special(specials::IO_IN_I)
                    .cfg("IN", "I")
                    .commit();
                bctx.mode("IO")
                    .global("XTALOSC", "DISABLE")
                    .cfg("IN", "I")
                    .mutex("TRI", "PULLUP")
                    .test_bel_special(specials::IO_IN_PULLUP)
                    .cfg("IN", "PULLUP")
                    .commit();
                bctx.mode("IO")
                    .cfg("IN", "I")
                    .mutex("TRI", "GND")
                    .mutex("OUT", "O")
                    .test_bel_special(specials::IO_OUT_O)
                    .cfg("OUT", "O")
                    .commit();
            }
            if backend.edev.db.bel_slots[slot].kind == BelKind::Class(bcls::PULLUP) {
                let mut bctx = ctx.bel(slot);
                bctx.build()
                    .bidir_mutex_exclusive(bcls::PULLUP::O)
                    .test_bel_attr_bits(bcls::PULLUP::ENABLE)
                    .pip_pin("O", "O")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcid) {
            continue;
        }
        for (bslot, bel) in &tcls.bels {
            if bslot == bslots::BUFG {
                continue;
            }
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
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
                        for &src in mux.src.keys() {
                            if wires::IMUX_IOCLK.contains(mux.dst.wire)
                                && matches!(src.wire, wires::ACLK | wires::GCLK)
                            {
                                inps.push((Some(src), Diff::default()));
                                got_empty = true;
                                continue;
                            }
                            if src.wire == wires::TIE_0
                                && !wires::IMUX_TBUF_T.contains(mux.dst.wire)
                            {
                                let (slot, pin) = wire_to_inpin(mux.dst.wire).unwrap();
                                assert_eq!(pin, "T");
                                let diff = ctx.get_diff_bel_special(tcid, slot, specials::IO_OUT_O);
                                inps.push((Some(src), diff));
                                continue;
                            }
                            if src.wire == wires::SPECIAL_IO_PULLUP {
                                let (slot, pin) = wire_to_inpin(mux.dst.wire).unwrap();
                                assert_eq!(pin, "T");
                                let mut diff_i =
                                    ctx.get_diff_bel_special(tcid, slot, specials::IO_IN_I);
                                let mut diff_pullup =
                                    ctx.get_diff_bel_special(tcid, slot, specials::IO_IN_PULLUP);
                                if tcls.bels.contains_id(bslots::OSC)
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
                                    ctx.insert_bel_attr_bitvec(
                                        tcid,
                                        slot,
                                        bcls::IO::OSC_PULLUP,
                                        vec![xlat_bit(!diff_i_spec)],
                                    );
                                }
                                assert_eq!(diff_i, !&diff_pullup);
                                inps.push((Some(src), diff_pullup));
                                continue;
                            }
                            if src.wire == wires::TIE_1 {
                                inps.push((Some(src), Diff::default()));
                                got_empty = true;
                                continue;
                            }
                            let diff = ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, src));
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((Some(src), diff));
                        }
                        if !got_empty {
                            let Some((_slot, pin)) = wire_to_inpin(mux.dst.wire) else {
                                unreachable!()
                            };
                            assert_eq!(pin, "O");
                            inps.push((None, Diff::default()));
                        }
                        let item = xlat_enum_raw(inps, OcdMode::Mux);
                        if item.bits.is_empty() && !wires::IMUX_TBUF_I.contains(mux.dst.wire) {
                            println!("UMMM MUX {tcname} {mux_name} is empty");
                        }
                        ctx.insert_mux(tcid, mux.dst, item);
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        ctx.collect_progbuf(tcid, buf.dst, buf.src);
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    SwitchBoxItem::Pass(pass) => {
                        ctx.collect_pass(tcid, pass.dst, pass.src);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    SwitchBoxItem::ProgInv(inv) => {
                        ctx.collect_inv_bi(tcid, inv.dst);
                    }
                    _ => unreachable!(),
                }
            }
        }

        for slot in tcls.bels.ids() {
            if ctx.edev.db.bel_slots[slot].kind == BelKind::Class(bcls::PULLUP) {
                ctx.collect_bel_attr(tcid, slot, bcls::PULLUP::ENABLE);
            }
        }
    }
}
