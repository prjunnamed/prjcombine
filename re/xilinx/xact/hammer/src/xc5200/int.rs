use std::collections::HashMap;

use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelInfo, PolTileWireCoord, SwitchBoxItem, TileWireCoord},
    grid::{BelCoord, TileCoord, WireCoord},
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit_raw, xlat_enum_raw};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_xc2000::xc5200::{bcls, bslots, tcls, tslots, wires};
use prjcombine_xilinx_bitstream::BitRect;

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

fn wire_to_outpin(wire: WireCoord) -> Option<(BelCoord, &'static str)> {
    match wire.slot {
        wires::OUT_PROGTIE => Some((wire.bel(bslots::LC[0]), "CV")),
        wires::OUT_CLKIOB => Some((wire.bel(bslots::CLKIOB), "I")),
        wires::OUT_RDBK_RIP => Some((wire.bel(bslots::RDBK), "RIP")),
        wires::OUT_RDBK_DATA => Some((wire.bel(bslots::RDBK), "DATA")),
        wires::OUT_STARTUP_DONEIN => Some((wire.bel(bslots::STARTUP), "DONEIN")),
        wires::OUT_STARTUP_Q1Q4 => Some((wire.bel(bslots::STARTUP), "Q1Q4")),
        wires::OUT_STARTUP_Q2 => Some((wire.bel(bslots::STARTUP), "Q2")),
        wires::OUT_STARTUP_Q3 => Some((wire.bel(bslots::STARTUP), "Q3")),
        wires::OUT_BSCAN_DRCK => Some((wire.bel(bslots::BSCAN), "DRCK")),
        wires::OUT_BSCAN_IDLE => Some((wire.bel(bslots::BSCAN), "IDLE")),
        wires::OUT_BSCAN_RESET => Some((wire.bel(bslots::BSCAN), "RESET")),
        wires::OUT_BSCAN_SEL1 => Some((wire.bel(bslots::BSCAN), "SEL1")),
        wires::OUT_BSCAN_SEL2 => Some((wire.bel(bslots::BSCAN), "SEL2")),
        wires::OUT_BSCAN_SHIFT => Some((wire.bel(bslots::BSCAN), "SHIFT")),
        wires::OUT_BSCAN_UPDATE => Some((wire.bel(bslots::BSCAN), "UPDATE")),
        wires::OUT_BSUPD => Some((wire.bel(bslots::OSC_NE), "BSUPD")),
        wires::OUT_OSC_OSC1 => Some((wire.bel(bslots::OSC_NE), "OSC1")),
        wires::OUT_OSC_OSC2 => Some((wire.bel(bslots::OSC_NE), "OSC2")),
        wires::OUT_TOP_COUT => Some((wire.delta(0, -1).bel(bslots::LC[0]), "CO")),
        _ => {
            if let Some(idx) = wires::OUT_IO_I.index_of(wire.slot) {
                Some((wire.bel(bslots::IO[idx]), "I"))
            } else if let Some(idx) = wires::OUT_LC_X.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.X", "LC1.X", "LC2.X", "LC3.X"][idx],
                ))
            } else if let Some(idx) = wires::OUT_LC_Q.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.Q", "LC1.Q", "LC2.Q", "LC3.Q"][idx],
                ))
            } else if let Some(idx) = wires::OUT_LC_DO.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.DO", "LC1.DO", "LC2.DO", "LC3.DO"][idx],
                ))
            } else if let Some(idx) = wires::OUT_TBUF.index_of(wire.slot) {
                Some((wire.bel(bslots::TBUF[idx]), "O"))
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
    let grid = backend.edev.chip;
    let mut cell = wire_target.cell;
    let wt = wire_target.slot;
    let wtn = &backend.edev.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if let Some((bel, pin)) = wire_to_outpin(wire_target) {
        let tcrd = bel.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            &ntile.bels[bel.slot][0],
            pin,
        );
    } else if wire_target.slot == wires::TIE_0 {
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            &ntile.tie_names[0],
            "O",
        );
    } else if matches!(
        wire_target.slot,
        wires::GCLK_W | wires::GCLK_E | wires::GCLK_S | wires::GCLK_N
    ) {
        let tcrd = cell.tile(tslots::MAIN);
        let nwt = backend
            .edev
            .resolve_tile_wire(tcrd, TileWireCoord::new_idx(0, wires::IMUX_GIN))
            .unwrap();
        let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
        let fuzzer = fuzzer.base(Key::WireMutex(wire_target), nwt);
        let crd = backend.ngrid.bel_pip(cell.bel(bslots::BUFR), "BUF");
        let fuzzer = fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()));
        return (fuzzer, block, pin);
    } else if matches!(
        wire_target.slot,
        wires::GCLK_SW | wires::GCLK_SE | wires::GCLK_NW | wires::GCLK_NE
    ) {
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        return (
            fuzzer.base(Key::WireMutex(wire_target), "SHARED_ROOT"),
            &ntile.bels[bslots::BUFG][0],
            "O",
        );
    } else if wire_target.slot == wires::IMUX_GIN {
        (
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, wires::TIE_0),
        )
    } else if let Some(idx) = wires::IO_M_BUF.index_of(wire_target.slot) {
        (
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, wires::IO_M[idx]),
        )
    } else if let Some(idx) = wires::CLB_M_BUF.index_of(wire_target.slot) {
        (
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, wires::CLB_M[idx]),
        )
    } else if let Some(idx) = wires::OMUX_BUF.index_of(wire_target.slot) {
        (
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, wires::OMUX[idx]),
        )
    } else if wires::OMUX.contains(wire_target.slot) {
        let nwt = if cell.col == grid.col_w()
            || cell.col == grid.col_e()
            || cell.row == grid.row_s()
            || cell.row == grid.row_n()
        {
            wires::OUT_IO_I[0]
        } else {
            wires::OUT_PROGTIE
        };
        (
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, nwt),
        )
    } else if wires::LONG_H.contains(wire_target.slot) || wires::LONG_V.contains(wire_target.slot) {
        if wires::LONG_H.contains(wire_target.slot) {
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
        let idx = wires::LONG_H
            .index_of(wire_target.slot)
            .unwrap_or_else(|| wires::LONG_V.index_of(wire_target.slot).unwrap());
        (
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wire_target.slot),
            TileWireCoord::new_idx(0, wires::OUT_TBUF[idx % 4]),
        )
    } else if wires::CLB_M.contains(wire_target.slot) || wires::IO_M.contains(wire_target.slot) {
        let tcrd = cell.tile(tslots::MAIN);
        let tile = &backend.edev[tcrd];
        let tcls_index = &backend.edev.db_index[tile.class];
        'a: {
            for &inp in &tcls_index.pips_bwd[&TileWireCoord::new_idx(0, wire_target.slot)] {
                if backend.edev.db.wires.key(inp.wire).starts_with("LONG")
                    || backend.edev.db.wires.key(inp.wire).starts_with("GCLK")
                {
                    break 'a (tcrd, TileWireCoord::new_idx(0, wire_target.slot), inp.tw);
                }
            }
            panic!("ummm no long?")
        }
    } else if wtn.starts_with("SINGLE") || wtn.starts_with("DBL") {
        'a: {
            for w in backend.edev.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = &backend.edev[tcrd];
                let tcls_index = &backend.edev.db_index[tile.class];
                if let Some(ins) = tcls_index.pips_bwd.get(&TileWireCoord::new_idx(0, w.slot)) {
                    for &inp in ins {
                        if backend.edev.db.wires.key(inp.wire).starts_with("CLB_M")
                            || backend.edev.db.wires.key(inp.wire).starts_with("IO_M")
                        {
                            let rwf = backend.edev.resolve_tile_wire(tcrd, inp.tw).unwrap();
                            if rwf != wire_avoid {
                                break 'a (tcrd, TileWireCoord::new_idx(0, w.slot), inp.tw);
                            }
                        }
                    }
                }
            }
            panic!("ummm no m?")
        }
    } else {
        panic!("umm wtf is {wtn}")
    };
    let nwt = backend.edev.resolve_tile_wire(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn wire_to_inpin(wire: WireCoord) -> Option<(BelCoord, &'static str)> {
    match wire.slot {
        wires::IMUX_CLB_RST => Some((wire.bel(bslots::LC[0]), "CLR")),
        wires::IMUX_CLB_CE => Some((wire.bel(bslots::LC[0]), "CE")),
        wires::IMUX_CLB_CLK => Some((wire.bel(bslots::LC[0]), "CK")),
        wires::IMUX_RDBK_RCLK => Some((wire.bel(bslots::RDBK), "CK")),
        wires::IMUX_RDBK_TRIG => Some((wire.bel(bslots::RDBK), "TRIG")),
        wires::IMUX_STARTUP_SCLK => Some((wire.bel(bslots::STARTUP), "CK")),
        wires::IMUX_STARTUP_GRST => Some((wire.bel(bslots::STARTUP), "GCLR")),
        wires::IMUX_STARTUP_GTS => Some((wire.bel(bslots::STARTUP), "GTS")),
        wires::IMUX_BSCAN_TDO1 => Some((wire.bel(bslots::BSCAN), "TDO1")),
        wires::IMUX_BSCAN_TDO2 => Some((wire.bel(bslots::BSCAN), "TDO2")),
        wires::IMUX_OSC_OCLK => Some((wire.bel(bslots::OSC_NE), "CK")),
        wires::IMUX_BUFG => Some((wire.bel(bslots::BUFG), "I")),
        wires::IMUX_BOT_CIN => Some((wire.delta(0, 1).bel(bslots::LC[0]), "CI")),
        _ => {
            if let Some(idx) = wires::IMUX_LC_F1.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.F1", "LC1.F1", "LC2.F1", "LC3.F1"][idx],
                ))
            } else if let Some(idx) = wires::IMUX_LC_F2.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.F2", "LC1.F2", "LC2.F2", "LC3.F2"][idx],
                ))
            } else if let Some(idx) = wires::IMUX_LC_F3.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.F3", "LC1.F3", "LC2.F3", "LC3.F3"][idx],
                ))
            } else if let Some(idx) = wires::IMUX_LC_F4.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.F4", "LC1.F4", "LC2.F4", "LC3.F4"][idx],
                ))
            } else if let Some(idx) = wires::IMUX_LC_DI.index_of(wire.slot) {
                Some((
                    wire.bel(bslots::LC[0]),
                    ["LC0.DI", "LC1.DI", "LC2.DI", "LC3.DI"][idx],
                ))
            } else if let Some(idx) = wires::IMUX_IO_T.index_of(wire.slot) {
                Some((wire.bel(bslots::IO[idx]), "T"))
            } else if let Some(idx) = wires::IMUX_IO_O.index_of(wire.slot) {
                Some((wire.bel(bslots::IO[idx]), "O"))
            } else {
                None
            }
        }
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
    let Some((bel, pin)) = wire_to_inpin(wire) else {
        return fuzzer;
    };
    let tcrd = bel.tile(tslots::MAIN);
    let ntile = &backend.ngrid.tiles[&tcrd];
    let block = &ntile.bels[bel.slot][0];
    if wires::IMUX_IO_O.contains(wire.slot) {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BlockConfig(block, "OUT".into(), "O".into()), true);
        if inv {
            fuzzer = fuzzer.fuzz(
                Key::BlockConfig(block, "OUT".into(), "NOT".into()),
                false,
                true,
            );
        }
    }
    if bel.slot == bslots::STARTUP && pin == "CK" {
        fuzzer = fuzzer.base(Key::GlobalOpt("STARTUPCLK".into()), "CCLK");
    }
    if bel.slot == bslots::OSC_NE && pin == "CK" {
        fuzzer = fuzzer.base(Key::GlobalOpt("OSCCLK".into()), "CCLK");
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
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::WireMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(tcrd, self.wire_to, self.wire_from.tw);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin, self.wire_from.inv);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct AllColumnIo;

impl<'b> FuzzerProp<'b, XactBackend<'b>> for AllColumnIo {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let id = fuzzer.info.features.pop().unwrap().key;
        for row in backend.edev.rows(tcrd.die) {
            if row == backend.edev.chip.row_s() || row == backend.edev.chip.row_n() {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: id.clone(),
                rects: EntityVec::from_iter([
                    BitRect::Null,
                    backend.edev.btile_main(tcrd.col, row),
                ]),
            });
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.edev.db;
    for (tcid, _, tcls) in &intdb.tile_classes {
        let tcls_index = &backend.edev.db_index[tcid];
        if tcls_index.pips_bwd.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            for &wire_from in ins {
                if matches!(wire_from.wire, wires::IMUX_BUFG | wires::IMUX_GIN) {
                    continue;
                }
                if wires::IMUX_IO_O.contains(wire_from.wire) {
                    continue;
                }
                if let Some(idx) = wires::IMUX_IO_O_SN.index_of(wire_to.wire) {
                    let wire_to = TileWireCoord {
                        wire: wires::IMUX_IO_O[idx],
                        ..wire_to
                    };
                    ctx.build()
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                        .prop(IntPip::new(wire_to, wire_from))
                        .commit();
                    continue;
                }
                let mut f = ctx
                    .build()
                    .test_raw(DiffKey::Routing(tcid, wire_to, wire_from))
                    .prop(IntPip::new(wire_to, wire_from));
                if wire_to.wire == wires::LONG_V[2] && matches!(tcid, tcls::LLV_W | tcls::LLV_E) {
                    f = f.prop(AllColumnIo);
                }
                f.commit();
                if wires::IMUX_IO_O.contains(wire_to.wire)
                    && matches!(tcid, tcls::IO_S | tcls::IO_N)
                {
                    ctx.build()
                        .test_raw(DiffKey::Routing(tcid, wire_to, !wire_from))
                        .prop(IntPip::new(wire_to, !wire_from))
                        .commit();
                }
            }
        }
        if tcls.bels.contains_id(bslots::TBUF[0]) {
            for i in 0..4 {
                let mut bctx = ctx.bel(bslots::TBUF[i]);
                bctx.build()
                    .input_mutex_exclusive(bcls::TBUF::T)
                    .test_bel_attr_bits(bcls::TBUF::T_ENABLE)
                    .pip_pin("T", "T")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        for (bslot, bel) in &tcls.bels {
            if matches!(bslot, bslots::BUFG | bslots::BUFR) {
                continue;
            }
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let mut mux_io_o_sn = HashMap::new();
            for item in &sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    continue;
                };
                if wires::IMUX_IO_O_SN.contains(mux.dst.wire) {
                    mux_io_o_sn.insert(mux.dst.wire, mux);
                }
            }
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        if wires::IMUX_IO_O_SN.contains(mux.dst.wire) {
                            continue;
                        }
                        if matches!(tcid, tcls::IO_S | tcls::IO_N)
                            && let Some(idx) = wires::IMUX_IO_O.index_of(mux.dst.wire)
                        {
                            let omux = mux_io_o_sn[&wires::IMUX_IO_O_SN[idx]];
                            let mut inps = vec![];
                            let mut oinps = vec![(Some(mux.dst.pos()), Diff::default())];
                            let mut got_empty = false;
                            for &wire_from in mux.src.keys() {
                                let diff =
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wire_from));
                                let diff_i =
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, !wire_from));
                                if diff.bits.is_empty() {
                                    got_empty = true;
                                }
                                oinps.push((Some(mux.dst.neg()), diff_i.combine(&!&diff)));
                                inps.push((Some(wire_from), diff));
                            }
                            for &wire_from in omux.src.keys() {
                                if wire_from.tw == mux.dst {
                                    continue;
                                }
                                let diff =
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wire_from));
                                oinps.push((Some(wire_from), diff));
                            }
                            assert!(got_empty);
                            let item = xlat_enum_raw(inps, OcdMode::Mux);
                            if item.bits.is_empty() {
                                println!(
                                    "UMMM MUX {tcname} {mux_name} is empty",
                                    mux_name = mux.dst.to_string(ctx.edev.db, &ctx.edev.db[tcid])
                                );
                            }
                            ctx.insert_mux(tcid, mux.dst, item);
                            oinps.sort_by_key(|x| x.0);
                            let item = xlat_enum_raw(oinps, OcdMode::Mux);
                            ctx.insert_mux(tcid, omux.dst, item);
                        } else {
                            let mut inps = vec![];
                            let mut got_empty = false;
                            for &wire_from in mux.src.keys() {
                                let diff =
                                    ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, wire_from));
                                if diff.bits.is_empty() {
                                    got_empty = true;
                                }
                                inps.push((Some(wire_from), diff));
                            }
                            for (rtile, rwire, rbel, rattr) in [
                                (
                                    tcls::CNR_SE,
                                    wires::IMUX_STARTUP_GTS,
                                    bslots::STARTUP,
                                    bcls::STARTUP::GTS_ENABLE,
                                ),
                                (
                                    tcls::CNR_SE,
                                    wires::IMUX_STARTUP_GRST,
                                    bslots::STARTUP,
                                    bcls::STARTUP::GR_ENABLE,
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
                            let item = xlat_enum_raw(inps, OcdMode::Mux);
                            if item.bits.is_empty() {
                                println!(
                                    "UMMM MUX {tcname} {mux_name} is empty",
                                    mux_name = mux.dst.to_string(ctx.edev.db, &ctx.edev.db[tcid])
                                );
                            }
                            ctx.insert_mux(tcid, mux.dst, item);
                        }
                    }
                    SwitchBoxItem::Pass(pass) => {
                        ctx.collect_pass(tcid, pass.dst, pass.src);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        let diff = ctx.get_diff_raw(&DiffKey::Routing(tcid, buf.dst, buf.src));
                        diff.assert_empty();
                    }
                    SwitchBoxItem::ProgInv(_) => (),
                    _ => unreachable!(),
                }
            }
        }
        if tcls.bels.contains_id(bslots::TBUF[0]) {
            for i in 0..4 {
                ctx.collect_bel_attr(tcid, bslots::TBUF[i], bcls::TBUF::T_ENABLE);
            }
        }
    }
}
