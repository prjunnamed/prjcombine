use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::{
        BelInfo, BelKind, BelSlotId, CellSlotId, ConnectorWire, SwitchBoxItem, TileWireCoord,
        WireKind, WireSlotId,
    },
    dir::Dir,
    grid::{BelCoord, TileCoord, WireCoord},
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit_bi, xlat_enum_raw};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_types::bsdata::BitRectId;
use prjcombine_xc2000::xc2000::{bcls, bslots, ccls, cslots, tcls, tslots, wires};

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
            } else {
                None
            }
        }
    }
}

fn drive_wire<'a>(
    backend: &XactBackend<'a>,
    fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: WireCoord,
    wire_avoid: WireCoord,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let wt = wire_target.slot;
    let wtn = &backend.edev.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if let Some((bel, pin)) = wire_to_outpin(backend, wire_target) {
        let tcrd = backend.edev.bel_tile(bel);
        let ntile = &backend.ngrid.tiles[&tcrd];
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            &ntile.bels[bel.slot][0],
            pin,
        );
    } else if wtn.starts_with("SINGLE_V")
        || wtn.starts_with("LONG_V")
        || wire_target.slot == wires::LONG_IO_W
        || wire_target.slot == wires::LONG_IO_E
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
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("SINGLE_V") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else if wtn.starts_with("SINGLE_H")
        || wtn == "LONG_H"
        || wtn == "LONG_HS"
        || wtn == "LONG_IO_S"
        || wtn == "LONG_IO_N"
    {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("SINGLE_V") {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no out for {wtn}?")
        }
    } else {
        panic!("umm wtf is {wtn}")
    };
    let nwt = backend.edev.resolve_tile_wire(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: WireCoord,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
) -> Fuzzer<XactBackend<'a>> {
    let mut cell = wire.cell;
    let (slot, pin) = match wire.slot {
        wires::IMUX_CLB_A => (bslots::CLB, "A"),
        wires::IMUX_CLB_B => (bslots::CLB, "B"),
        wires::IMUX_CLB_C => (bslots::CLB, "C"),
        wires::IMUX_CLB_D => {
            cell.row += 1;
            (bslots::CLB, "D")
        }
        wires::IMUX_CLB_D_N => (bslots::CLB, "D"),
        wires::IMUX_CLB_K => (bslots::CLB, "K"),
        wires::IMUX_BUFG => (bslots::BUFG, "I"),
        _ => {
            if let Some(idx) = wires::IMUX_IO_W_O.index_of(wire.slot) {
                (bslots::IO_W[idx], "O")
            } else if let Some(idx) = wires::IMUX_IO_W_T.index_of(wire.slot) {
                (bslots::IO_W[idx], "T")
            } else if let Some(idx) = wires::IMUX_IO_E_O.index_of(wire.slot) {
                (bslots::IO_E[idx], "O")
            } else if let Some(idx) = wires::IMUX_IO_E_T.index_of(wire.slot) {
                (bslots::IO_E[idx], "T")
            } else if let Some(idx) = wires::IMUX_IO_S_O.index_of(wire.slot) {
                (bslots::IO_S[idx], "O")
            } else if let Some(idx) = wires::IMUX_IO_S_T.index_of(wire.slot) {
                (bslots::IO_S[idx], "T")
            } else if let Some(idx) = wires::IMUX_IO_N_O.index_of(wire.slot) {
                (bslots::IO_N[idx], "O")
            } else if let Some(idx) = wires::IMUX_IO_N_T.index_of(wire.slot) {
                (bslots::IO_N[idx], "T")
            } else {
                return fuzzer;
            }
        }
    };
    let bel = cell.bel(slot);
    let tcrd = cell.tile(tslots::MAIN);
    let ntile = &backend.ngrid.tiles[&tcrd];
    let block = &ntile.bels[slot][0];
    if slot == bslots::CLB && pin == "K" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "FG")
            .base(Key::BelMutex(bel, "CLK".into()), pin)
            .fuzz(
                Key::BlockConfig(block, "CLK".into(), pin.into()),
                false,
                true,
            );
    }
    if pin == "T" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BelMutex(bel, "BUF".into()), "TRI")
            .fuzz(
                Key::BlockConfig(block, "BUF".into(), "TRI".into()),
                false,
                true,
            );
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
}

impl IntPip {
    pub fn new(wire_to: TileWireCoord, wire_from: TileWireCoord) -> Self {
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
            .resolve_tile_wire(tcrd, self.wire_from)
            .unwrap();
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::WireMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(tcrd, self.wire_to, self.wire_from);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin);
        Some((fuzzer, false))
    }
}

#[derive(Debug, Clone)]
struct SingleBidi {
    wire: WireSlotId,
    dir: Dir,
}

impl SingleBidi {
    fn new(wire: WireSlotId, dir: Dir) -> Self {
        Self { wire, dir }
    }
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for SingleBidi {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let wire = if let Some(idx) = wires::SINGLE_H.index_of(self.wire) {
            wires::SINGLE_H_E[idx]
        } else if let Some(idx) = wires::SINGLE_HS.index_of(self.wire) {
            wires::SINGLE_HS_E[idx]
        } else if let Some(idx) = wires::SINGLE_HN.index_of(self.wire) {
            wires::SINGLE_HN_E[idx]
        } else if let Some(idx) = wires::SINGLE_V.index_of(self.wire) {
            wires::SINGLE_V_S[idx]
        } else if let Some(idx) = wires::SINGLE_VW.index_of(self.wire) {
            wires::SINGLE_VW_S[idx]
        } else if let Some(idx) = wires::SINGLE_VE.index_of(self.wire) {
            wires::SINGLE_VE_S[idx]
        } else {
            unreachable!()
        };
        let wire = TileWireCoord::new_idx(0, wire);
        let bidi_tcid = match self.dir {
            Dir::W | Dir::E => {
                if tcrd.row == backend.edev.chip.row_s() {
                    tcls::BIDIH_S
                } else if tcrd.row == backend.edev.chip.row_n() {
                    tcls::BIDIH_N
                } else {
                    tcls::BIDIH
                }
            }
            Dir::S | Dir::N => {
                if tcrd.col == backend.edev.chip.col_w() {
                    tcls::BIDIV_W
                } else if tcrd.col == backend.edev.chip.col_e() {
                    tcls::BIDIV_E
                } else {
                    tcls::BIDIV
                }
            }
        };
        match self.dir {
            Dir::W => {
                let bidi_tcrd = tcrd.tile(tslots::BIDIH);
                if !backend.edev.chip.cols_bidi.contains(&bidi_tcrd.col) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingBidi(bidi_tcid, cslots::W, wire, false),
                    rects: backend.edev.tile_bits(bidi_tcrd),
                });
                Some((fuzzer, false))
            }
            Dir::E => {
                let bidi_tcrd = tcrd.delta(1, 0).tile(tslots::BIDIH);
                if !backend.edev.chip.cols_bidi.contains(&bidi_tcrd.col) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingBidi(bidi_tcid, cslots::W, wire, true),
                    rects: backend.edev.tile_bits(bidi_tcrd),
                });
                Some((fuzzer, false))
            }
            Dir::S => {
                let bidi_tcrd = tcrd.tile(tslots::BIDIV);
                if !backend.edev.chip.rows_bidi.contains(&bidi_tcrd.row) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingBidi(bidi_tcid, cslots::N, wire, true),
                    rects: backend.edev.tile_bits(bidi_tcrd),
                });
                Some((fuzzer, false))
            }
            Dir::N => {
                let bidi_tcrd = tcrd.delta(0, 1).tile(tslots::BIDIV);
                if !backend.edev.chip.rows_bidi.contains(&bidi_tcrd.row) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingBidi(bidi_tcid, cslots::N, wire, false),
                    rects: backend.edev.tile_bits(bidi_tcrd),
                });
                Some((fuzzer, false))
            }
        }
    }
}

#[derive(Debug, Clone)]
struct HasBidi {
    dir: Dir,
    val: bool,
}

impl<'b> FuzzerProp<'b, XactBackend<'b>> for HasBidi {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let val = match self.dir {
            Dir::W => backend.edev.chip.cols_bidi.contains(&tcrd.col),
            Dir::E => backend.edev.chip.cols_bidi.contains(&(tcrd.col + 1)),
            Dir::S => backend.edev.chip.rows_bidi.contains(&tcrd.row),
            Dir::N => backend.edev.chip.rows_bidi.contains(&(tcrd.row + 1)),
        };
        if val != self.val {
            return None;
        }
        Some((fuzzer, false))
    }
}

impl HasBidi {
    fn new(dir: Dir, val: bool) -> Self {
        Self { dir, val }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, _, tcls) in &intdb.tile_classes {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for slot in tcls.bels.ids() {
            if backend.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
                continue;
            }
            let mut bctx = ctx.bel(slot);
            bctx.mode("IO")
                .mutex("BUF", "ON")
                .test_bel_special(specials::IO_BUF_ON)
                .cfg("BUF", "ON")
                .commit();
        }
        let tcls_index = &backend.edev.db_index[tcid];
        if tcls_index.pips_bwd.is_empty() {
            continue;
        }
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let wire_to_name = intdb.wires.key(wire_to.wire);
            for &wire_from in ins {
                if matches!(
                    wire_from.wire,
                    wires::TIE_0
                        | wires::TIE_1
                        | wires::SPECIAL_CLB_C
                        | wires::SPECIAL_CLB_G
                        | wires::IMUX_BUFG
                ) {
                    continue;
                }
                let mut f = ctx
                    .build()
                    .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                    .prop(IntPip::new(wire_to, wire_from.tw));
                if wire_to_name.starts_with("SINGLE_H")
                    && matches!(tcid, tcls::CLB | tcls::CLB_S | tcls::CLB_N)
                {
                    let (dir, wire) =
                        if matches!(intdb.wires[wire_to.wire], WireKind::MultiBranch(_)) {
                            let ConnectorWire::Pass(wire) = intdb[ccls::PASS_W].wires[wire_to.wire]
                            else {
                                unreachable!()
                            };
                            (Dir::W, wire)
                        } else {
                            (Dir::E, wire_to.wire)
                        };
                    f = f.prop(SingleBidi::new(wire, dir));
                }
                if wire_to.wire == wires::SINGLE_VE[0] && tcid == tcls::CLB_E {
                    f.prop(HasBidi::new(Dir::S, false)).commit();
                    ctx.build()
                        .test_raw(DiffKey::RoutingPairSpecial(
                            tcid,
                            wire_to,
                            wire_from,
                            specials::BIDI_S,
                        ))
                        .prop(IntPip::new(wire_to, wire_from.tw))
                        .prop(HasBidi::new(Dir::S, true))
                        .commit();
                    continue;
                } else if wire_to_name.starts_with("SINGLE_V")
                    && matches!(tcid, tcls::CLB | tcls::CLB_W | tcls::CLB_E)
                {
                    let (dir, wire) =
                        if matches!(intdb.wires[wire_to.wire], WireKind::MultiBranch(_)) {
                            let ConnectorWire::Pass(wire) = intdb[ccls::PASS_N].wires[wire_to.wire]
                            else {
                                unreachable!()
                            };
                            (Dir::N, wire)
                        } else {
                            (Dir::S, wire_to.wire)
                        };
                    f = f.prop(SingleBidi::new(wire, dir));
                }
                f.commit();
            }
        }
    }
}

fn wire_as_imux_io_t(wire: WireSlotId) -> Option<BelSlotId> {
    if let Some(idx) = wires::IMUX_IO_W_T.index_of(wire) {
        Some(bslots::IO_W[idx])
    } else if let Some(idx) = wires::IMUX_IO_E_T.index_of(wire) {
        Some(bslots::IO_E[idx])
    } else if let Some(idx) = wires::IMUX_IO_S_T.index_of(wire) {
        Some(bslots::IO_S[idx])
    } else if let Some(idx) = wires::IMUX_IO_N_T.index_of(wire) {
        Some(bslots::IO_N[idx])
    } else {
        None
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for tcid in [
        tcls::BIDIH,
        tcls::BIDIH_S,
        tcls::BIDIH_N,
        tcls::BIDIV,
        tcls::BIDIV_W,
        tcls::BIDIV_E,
    ] {
        let tcls = &intdb[tcid];
        let bslot = if tcls.slot == tslots::BIDIH {
            bslots::BIDIH
        } else {
            bslots::BIDIV
        };
        let BelInfo::SwitchBox(ref sb) = tcls.bels[bslot] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::Bidi(bidi) = item else {
                unreachable!()
            };
            if bidi.wire.wire == wires::SINGLE_VE_S[0] {
                let stcls = &ctx.edev.db_index[tcls::CLB_E];
                let wire_to = TileWireCoord::new_idx(0, wires::SINGLE_VE[0]);
                let ins = &stcls.pips_bwd[&wire_to];
                let mut diff_s = None;
                for &wire_from in ins {
                    let diff = ctx.get_diff_raw(&DiffKey::RoutingPairSpecial(
                        tcls::CLB_E,
                        wire_to,
                        wire_from,
                        specials::BIDI_S,
                    ));
                    let diff_base =
                        ctx.peek_diff_raw(&DiffKey::Routing(tcls::CLB_E, wire_to, wire_from));
                    let diff = diff.combine(&!diff_base);
                    if diff_s.is_none() {
                        diff_s = Some(diff)
                    } else {
                        assert_eq!(diff_s, Some(diff));
                    }
                }
                let diff_s = diff_s.unwrap().filter_rects(&EntityVec::from_iter([
                    BitRectId::from_idx(1),
                    BitRectId::from_idx(0),
                ]));
                let diff0 =
                    ctx.get_diff_raw(&DiffKey::RoutingBidi(tcid, bidi.conn, bidi.wire, false));
                let diff1 = diff_s;
                let bit = xlat_bit_bi(diff0, diff1);
                ctx.insert_bidi(tcid, bidi.conn, bidi.wire, bit);
            } else {
                ctx.collect_bidi(tcid, bidi.conn, bidi.wire);
            }
        }
    }
    for (tcid, tcname, tcls) in &intdb.tile_classes {
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
                        assert_eq!(mux.dst.cell, CellSlotId::from_idx(0));
                        let out_name = intdb.wires.key(mux.dst.wire);
                        let mux_name = format!("MUX.{out_name}");
                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in mux.src.keys() {
                            if matches!(
                                wire_from.wire,
                                wires::TIE_0
                                    | wires::TIE_1
                                    | wires::SPECIAL_CLB_C
                                    | wires::SPECIAL_CLB_G
                            ) {
                                continue;
                            }
                            let diff =
                                ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wire_from));
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((Some(wire_from), diff));
                        }
                        if let Some(slot) = wire_as_imux_io_t(mux.dst.wire) {
                            assert!(!got_empty);
                            inps.push((
                                Some(TileWireCoord::new_idx(0, wires::TIE_1).pos()),
                                Diff::default(),
                            ));
                            inps.push((
                                Some(TileWireCoord::new_idx(0, wires::TIE_0).pos()),
                                ctx.get_diff_bel_special(tcid, slot, specials::IO_BUF_ON),
                            ));
                        } else if mux.dst.wire == wires::IMUX_CLB_K {
                            assert!(!got_empty);
                            inps.push((None, Diff::default()));
                            inps.push((
                                Some(TileWireCoord::new_idx(0, wires::SPECIAL_CLB_C).pos()),
                                ctx.get_diff_bel_special(tcid, bslots::CLB, specials::CLB_CLK_C),
                            ));
                            inps.push((
                                Some(TileWireCoord::new_idx(0, wires::SPECIAL_CLB_G).neg()),
                                ctx.get_diff_bel_special(tcid, bslots::CLB, specials::CLB_CLK_G)
                                    .combine(&!ctx.peek_diff_raw(&DiffKey::BelInputInv(
                                        tcid,
                                        bslots::CLB,
                                        bcls::CLB::K,
                                        true,
                                    ))),
                            ));
                        } else {
                            assert!(got_empty);
                        }
                        let item = xlat_enum_raw(inps, OcdMode::Mux);
                        if item.bits.is_empty() {
                            println!("UMMM MUX {tcname} {mux_name} is empty");
                        }
                        ctx.insert_mux(tcid, mux.dst, item);
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    SwitchBoxItem::Pass(pass) => {
                        ctx.collect_pass(tcid, pass.dst, pass.src);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    SwitchBoxItem::Bidi(_) => (),
                    _ => unreachable!(),
                }
            }
        }
    }
}
