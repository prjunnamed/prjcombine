use prjcombine_interconnect::{
    db::{BelInfo, CellSlotId, ConnectorWire, SwitchBoxItem, TileWireCoord, WireId},
    dir::Dir,
    grid::{TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_enum, xlat_enum_ocd,
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
    let rwf = backend
        .egrid
        .resolve_tile_wire(tcrd, wire_from)
        .unwrap();
    let rwt = backend
        .egrid
        .resolve_tile_wire(tcrd, wire_to)
        .unwrap();
    fuzzer = fuzzer.base(Key::NodeMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(tcrd, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
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
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") || wtn == "GCLK" || wtn == "ACLK" {
        let (slot, pin) = match wtn {
            "OUT.CLB.X" => (bels::CLB, "X"),
            "OUT.CLB.Y" => (bels::CLB, "Y"),
            "OUT.BIOB0.I" => (bels::IO_S0, "I"),
            "OUT.BIOB1.I" => (bels::IO_S1, "I"),
            "OUT.TIOB0.I" => (bels::IO_N0, "I"),
            "OUT.TIOB1.I" => (bels::IO_N1, "I"),
            "OUT.LIOB0.I" => (bels::IO_W0, "I"),
            "OUT.LIOB1.I" => (bels::IO_W1, "I"),
            "OUT.RIOB0.I" => (bels::IO_E0, "I"),
            "OUT.RIOB1.I" => (bels::IO_E1, "I"),
            "OUT.OSC" => (bels::OSC, "O"),
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
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &ntile.bels[slot][0],
            pin,
        );
    } else if wtn.starts_with("SINGLE.V")
        || wtn.starts_with("LONG.V")
        || wtn.starts_with("LONG.RV")
        || wtn == "LONG.IO.L"
        || wtn == "LONG.IO.R"
    {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = backend.egrid.tile(tcrd);
                let tcls = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls.pips_bwd.get(&TileWireCoord {
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
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let tile = backend.egrid.tile(tcrd);
                let tcls = &backend.egrid.db_index.tile_classes[tile.class];
                if let Some(ins) = tcls.pips_bwd.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: w.slot,
                }) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("SINGLE.V") {
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
            panic!("ummm no out for {wtn}?")
        }
    } else if wtn.starts_with("SINGLE.H")
        || wtn == "LONG.H"
        || wtn == "LONG.BH"
        || wtn == "LONG.IO.B"
        || wtn == "LONG.IO.T"
    {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let tcrd = w.cell.tile(tslots::MAIN);
                let node = backend.egrid.tile(tcrd);
                let node_kind = &backend.egrid.db_index.tile_classes[node.class];
                if let Some(ins) = node_kind.pips_bwd.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: w.slot,
                }) {
                    for &inp in ins {
                        if backend.egrid.db.wires.key(inp.wire).starts_with("SINGLE.V") {
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
        panic!("umm wtf is {wtn}")
    };
    let nwt = backend.egrid.resolve_tile_wire(ploc, pwf).unwrap();
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
    let w = wire.slot;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if !wn.starts_with("IMUX") {
        return fuzzer;
    }
    let (slot, pin) = match wn {
        "IMUX.CLB.A" => (bels::CLB, "A"),
        "IMUX.CLB.B" => (bels::CLB, "B"),
        "IMUX.CLB.C" => (bels::CLB, "C"),
        "IMUX.CLB.D" => {
            cell.row += 1;
            (bels::CLB, "D")
        }
        "IMUX.CLB.D.N" => (bels::CLB, "D"),
        "IMUX.CLB.K" => (bels::CLB, "K"),
        "IMUX.BIOB0.O" => (bels::IO_S0, "O"),
        "IMUX.BIOB0.T" => (bels::IO_S0, "T"),
        "IMUX.BIOB1.O" => (bels::IO_S1, "O"),
        "IMUX.BIOB1.T" => (bels::IO_S1, "T"),
        "IMUX.TIOB0.O" => (bels::IO_N0, "O"),
        "IMUX.TIOB0.T" => (bels::IO_N0, "T"),
        "IMUX.TIOB1.O" => (bels::IO_N1, "O"),
        "IMUX.TIOB1.T" => (bels::IO_N1, "T"),
        "IMUX.LIOB0.O" => (bels::IO_W0, "O"),
        "IMUX.LIOB0.T" => (bels::IO_W0, "T"),
        "IMUX.LIOB1.O" => (bels::IO_W1, "O"),
        "IMUX.LIOB1.T" => (bels::IO_W1, "T"),
        "IMUX.RIOB0.O" => (bels::IO_E0, "O"),
        "IMUX.RIOB0.T" => (bels::IO_E0, "T"),
        "IMUX.RIOB1.O" => (bels::IO_E1, "O"),
        "IMUX.RIOB1.T" => (bels::IO_E1, "T"),
        "IMUX.BUFG" => (bels::BUFG, "I"),
        _ => panic!("umm {wn}?"),
    };
    let bel = cell.bel(slot);
    let tcrd = cell.tile(tslots::MAIN);
    let nnode = &backend.ngrid.tiles[&tcrd];
    let block = &nnode.bels[slot][0];
    if slot == bels::CLB && pin == "K" {
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
        let rwt = backend
            .egrid
            .resolve_tile_wire(tcrd, self.wire_to)
            .unwrap();
        let rwf = backend
            .egrid
            .resolve_tile_wire(tcrd, self.wire_from)
            .unwrap();
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::NodeMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(tcrd, self.wire_to, self.wire_from);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin);
        Some((fuzzer, false))
    }
}

#[derive(Debug, Clone)]
struct SingleBidi {
    wire: WireId,
    dir: Dir,
}

impl SingleBidi {
    fn new(wire: WireId, dir: Dir) -> Self {
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
        let wn = backend.egrid.db.wires.key(self.wire);
        let bidi_tile = match self.dir {
            Dir::W | Dir::E => {
                if tcrd.row == backend.edev.chip.row_s() {
                    "BIDIH.B"
                } else if tcrd.row == backend.edev.chip.row_n() {
                    "BIDIH.T"
                } else {
                    "BIDIH"
                }
            }
            Dir::S | Dir::N => {
                if tcrd.col == backend.edev.chip.col_w() {
                    "BIDIV.L"
                } else if tcrd.col == backend.edev.chip.col_e() {
                    "BIDIV.R"
                } else {
                    "BIDIV"
                }
            }
        };
        match self.dir {
            Dir::W => {
                if !backend.edev.chip.cols_bidi.contains(&tcrd.col) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "W".into(),
                    },
                    tiles: vec![backend.edev.btile_llh(tcrd.col, tcrd.row)],
                });
                Some((fuzzer, false))
            }
            Dir::E => {
                if !backend.edev.chip.cols_bidi.contains(&(tcrd.col + 1)) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "E".into(),
                    },
                    tiles: vec![backend.edev.btile_llh(tcrd.col + 1, tcrd.row)],
                });
                Some((fuzzer, false))
            }
            Dir::S => {
                if !backend.edev.chip.rows_bidi.contains(&tcrd.row) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "S".into(),
                    },
                    tiles: vec![backend.edev.btile_llv(tcrd.col, tcrd.row)],
                });
                Some((fuzzer, false))
            }
            Dir::N => {
                if !backend.edev.chip.rows_bidi.contains(&(tcrd.row + 1)) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "N".into(),
                    },
                    tiles: vec![backend.edev.btile_llv(tcrd.col, tcrd.row + 1)],
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
    let intdb = backend.egrid.db;
    for (tcid, tile, _) in &intdb.tile_classes {
        let tcls_index = &backend.egrid.db_index.tile_classes[tcid];
        if tcls_index.pips_bwd.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let mux_name = format!("MUX.{}", intdb.wires.key(wire_to.wire));
            for &wire_from in ins {
                let wire_from = wire_from.tw;
                let wire_from_name = intdb.wires.key(wire_from.wire);
                let in_name = format!("{:#}.{}", wire_from.cell, wire_from_name);
                let mut f = ctx
                    .build()
                    .test_manual("INT", &mux_name, &in_name)
                    .prop(IntPip::new(wire_to, wire_from));
                if mux_name.starts_with("MUX.SINGLE.H")
                    && matches!(&tile[..], "CLB" | "CLB.B" | "CLB.T")
                {
                    let (dir, wire) = if mux_name.ends_with(".E") {
                        let term = intdb.get_conn_class("MAIN.W");
                        let ConnectorWire::Pass(wire) =
                            intdb.conn_classes[term].wires[wire_to.wire]
                        else {
                            unreachable!()
                        };
                        (Dir::W, wire)
                    } else {
                        (Dir::E, wire_to.wire)
                    };
                    f = f.prop(SingleBidi::new(wire, dir));
                }
                if mux_name == "MUX.SINGLE.V.R0" && tile == "CLB.R" {
                    f.prop(HasBidi::new(Dir::S, false)).commit();
                    ctx.build()
                        .test_manual("INT", &mux_name, format!("{in_name}.BIDI_S"))
                        .prop(IntPip::new(wire_to, wire_from))
                        .prop(HasBidi::new(Dir::S, true))
                        .commit();
                    continue;
                } else if mux_name.starts_with("MUX.SINGLE.V")
                    && matches!(&tile[..], "CLB" | "CLB.L" | "CLB.R")
                {
                    let (dir, wire) = if mux_name.ends_with(".S") {
                        let term = intdb.get_conn_class("MAIN.N");
                        let ConnectorWire::Pass(wire) =
                            intdb.conn_classes[term].wires[wire_to.wire]
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

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let intdb = ctx.edev.egrid.db;
    for (tile, stile) in [
        ("BIDIH", "CLB"),
        ("BIDIH.B", "CLB.B"),
        ("BIDIH.T", "CLB.T"),
        ("BIDIV", "CLB"),
        ("BIDIV.L", "CLB.L"),
        ("BIDIV.R", "CLB.R"),
    ] {
        let bel = "INT";
        let filter = if tile.starts_with("BIDIH") {
            "SINGLE.H"
        } else {
            "SINGLE.V"
        };
        let snode = intdb.get_tile_class(stile);
        let snode = &ctx.edev.egrid.db_index.tile_classes[snode];
        for (wire, ins) in &snode.pips_bwd {
            let wn = intdb.wires.key(wire.wire);
            if wn.starts_with(filter) && !wn.ends_with(".E") && !wn.ends_with(".S") {
                let attr = &format!("BIDI.{wn}");
                if wn == "SINGLE.V.R0" {
                    let mux_name = format!("MUX.{wn}");
                    let mut diff_s = None;
                    for &wire_from in ins {
                        let in_name =
                            format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                        let diff =
                            ctx.state
                                .get_diff(stile, bel, &mux_name, format!("{in_name}.BIDI_S"));
                        let diff_base = ctx.state.peek_diff(stile, bel, &mux_name, &in_name);
                        let diff = diff.combine(&!diff_base);
                        if diff_s.is_none() {
                            diff_s = Some(diff)
                        } else {
                            assert_eq!(diff_s, Some(diff));
                        }
                    }
                    let diff_s = diff_s.unwrap().filter_tiles(&[1, 0]);
                    let item = xlat_enum(vec![
                        ("S", diff_s),
                        ("N", ctx.state.get_diff(tile, bel, attr, "N")),
                    ]);
                    ctx.tiledb.insert(tile, bel, attr, item);
                } else {
                    ctx.collect_enum(
                        tile,
                        bel,
                        attr,
                        if tile.starts_with("BIDIH") {
                            &["W", "E"]
                        } else {
                            &["S", "N"]
                        },
                    );
                }
            }
        }
    }
    for (_, tile, node) in &intdb.tile_classes {
        for (bslot, bel) in &node.bels {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let bel = intdb.bel_slots.key(bslot);
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        assert_eq!(mux.dst.cell, CellSlotId::from_idx(0));
                        let out_name = intdb.wires.key(mux.dst.wire);
                        let mux_name = format!("MUX.{out_name}");
                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &wire_from in &mux.src {
                            let in_name =
                                format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                            let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((in_name.to_string(), diff));
                        }
                        if out_name.starts_with("IMUX") && out_name.ends_with("T") {
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
                            let bel = ctx.edev.egrid.db.bel_slots.key(slot);
                            assert!(!got_empty);
                            inps.push(("VCC".to_string(), Diff::default()));
                            inps.push((
                                "GND".to_string(),
                                ctx.state.get_diff(tile, bel, "BUF", "ON"),
                            ));
                        } else if out_name == "IMUX.CLB.K" {
                            assert!(!got_empty);
                            inps.push(("NONE".to_string(), Diff::default()));
                            inps.push((
                                "C".to_string(),
                                ctx.state.get_diff(tile, "CLB", "CLK", "C"),
                            ));
                            inps.push((
                                "G.INV".to_string(),
                                ctx.state
                                    .get_diff(tile, "CLB", "CLK", "G")
                                    .combine(&!ctx.state.peek_diff(tile, "CLB", "CLK", "NOT")),
                            ));
                        } else {
                            assert!(got_empty);
                        }
                        let item = xlat_enum_ocd(inps, OcdMode::Mux);
                        if item.bits.is_empty() {
                            println!("UMMM MUX {tile} {mux_name} is empty");
                        }
                        ctx.tiledb.insert(tile, bel, mux_name, item);
                    }
                    SwitchBoxItem::PermaBuf(_) => (),
                    SwitchBoxItem::Pass(pass) => {
                        assert_eq!(pass.dst.cell, CellSlotId::from_idx(0));
                        assert_eq!(pass.src.cell, CellSlotId::from_idx(0));
                        let out_name = intdb.wires.key(pass.dst.wire);
                        let mux_name = format!("MUX.{out_name}");
                        let in_name = intdb.wires.key(pass.src.wire);
                        let val_name = format!("{:#}.{}", pass.src.cell, in_name);
                        let diff = ctx.state.get_diff(tile, "INT", mux_name, &val_name);
                        let item = xlat_bit(diff);
                        let name = format!("PASS.{out_name}.{in_name}");
                        ctx.tiledb.insert(tile, bel, name, item);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        assert_eq!(pass.a.cell, CellSlotId::from_idx(0));
                        assert_eq!(pass.b.cell, CellSlotId::from_idx(0));
                        let a_name = intdb.wires.key(pass.a.wire);
                        let b_name = intdb.wires.key(pass.b.wire);
                        let name = format!("BIPASS.{a_name}.{b_name}");

                        for (wdst, wsrc) in [(pass.a, pass.b), (pass.b, pass.a)] {
                            let out_name = intdb.wires.key(wdst.wire);
                            let mux_name = format!("MUX.{out_name}");
                            let in_name = intdb.wires.key(wsrc.wire);
                            let val_name = format!("{:#}.{}", wsrc.cell, in_name);
                            let diff = ctx.state.get_diff(tile, "INT", mux_name, &val_name);
                            let item = xlat_bit(diff);
                            ctx.tiledb.insert(tile, bel, &name, item);
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
