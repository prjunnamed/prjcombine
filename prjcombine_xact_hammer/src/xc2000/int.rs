use prjcombine_collector::{xlat_bit, xlat_enum, xlat_enum_ocd, Diff, FeatureId, OcdMode};
use prjcombine_hammer::{Fuzzer, Session};
use prjcombine_int::{
    db::{Dir, NodeTileId, NodeWireId, TermInfo, WireId},
    grid::{IntWire, LayerId, NodeLoc},
};
use unnamed_entity::EntityId;

use crate::{
    backend::{FuzzerFeature, Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    fgen::Prop,
};

fn apply_int_pip<'a>(
    backend: &XactBackend<'a>,
    nloc: NodeLoc,
    wire_to: NodeWireId,
    wire_from: NodeWireId,
    block: &'a str,
    pin: &'static str,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
) -> Fuzzer<XactBackend<'a>> {
    let rwf = backend
        .egrid
        .resolve_node_wire_nobuf(nloc, wire_from)
        .unwrap();
    let rwt = backend
        .egrid
        .resolve_node_wire_nobuf(nloc, wire_to)
        .unwrap();
    fuzzer = fuzzer.base(Key::NodeMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(nloc, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
}

fn drive_wire<'a>(
    backend: &XactBackend<'a>,
    fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: IntWire,
    wire_avoid: IntWire,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let grid = backend.edev.grid;
    let (die, (mut col, mut row), wt) = wire_target;
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") || wtn == "GCLK" || wtn == "ACLK" {
        let (bel, pin) = match wtn {
            "OUT.CLB.X" => ("CLB", "X"),
            "OUT.CLB.Y" => ("CLB", "Y"),
            "OUT.BIOB0.I" => ("BIOB0", "I"),
            "OUT.BIOB1.I" => ("BIOB1", "I"),
            "OUT.TIOB0.I" => ("TIOB0", "I"),
            "OUT.TIOB1.I" => ("TIOB1", "I"),
            "OUT.LIOB0.I" => ("LIOB0", "I"),
            "OUT.LIOB1.I" => ("LIOB1", "I"),
            "OUT.RIOB0.I" => ("RIOB0", "I"),
            "OUT.RIOB1.I" => ("RIOB1", "I"),
            "OUT.OSC" => ("OSC", "O"),
            "GCLK" => {
                col = grid.col_lio();
                row = grid.row_tio();
                ("BUFG", "O")
            }
            "ACLK" => {
                col = grid.col_rio();
                row = grid.row_bio();
                ("BUFG", "O")
            }
            _ => panic!("umm {wtn}"),
        };
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let node_kind = &backend.egrid.db.nodes[node.kind];
        let nnode = &backend.ngrid.nodes[&nloc];
        let bel = node_kind.bels.get(bel).unwrap().0;
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &nnode.bels[bel][0],
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
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("OUT") {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if rwf != wire_avoid {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
                            }
                        }
                    }
                }
            }
            for w in backend.egrid.wire_tree(wire_target) {
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("SINGLE.V") {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if rwf != wire_avoid {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
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
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("SINGLE.V") {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if rwf != wire_avoid {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
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
    let nwt = backend.egrid.resolve_node_wire_nobuf(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: IntWire,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
) -> Fuzzer<XactBackend<'a>> {
    let (die, (col, mut row), w) = wire;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if !wn.starts_with("IMUX") {
        return fuzzer;
    }
    let (bel, pin) = match wn {
        "IMUX.CLB.A" => ("CLB", "A"),
        "IMUX.CLB.B" => ("CLB", "B"),
        "IMUX.CLB.C" => ("CLB", "C"),
        "IMUX.CLB.D" => {
            row += 1;
            ("CLB", "D")
        }
        "IMUX.CLB.D.N" => ("CLB", "D"),
        "IMUX.CLB.K" => ("CLB", "K"),
        "IMUX.BIOB0.O" => ("BIOB0", "O"),
        "IMUX.BIOB0.T" => ("BIOB0", "T"),
        "IMUX.BIOB1.O" => ("BIOB1", "O"),
        "IMUX.BIOB1.T" => ("BIOB1", "T"),
        "IMUX.TIOB0.O" => ("TIOB0", "O"),
        "IMUX.TIOB0.T" => ("TIOB0", "T"),
        "IMUX.TIOB1.O" => ("TIOB1", "O"),
        "IMUX.TIOB1.T" => ("TIOB1", "T"),
        "IMUX.LIOB0.O" => ("LIOB0", "O"),
        "IMUX.LIOB0.T" => ("LIOB0", "T"),
        "IMUX.LIOB1.O" => ("LIOB1", "O"),
        "IMUX.LIOB1.T" => ("LIOB1", "T"),
        "IMUX.RIOB0.O" => ("RIOB0", "O"),
        "IMUX.RIOB0.T" => ("RIOB0", "T"),
        "IMUX.RIOB1.O" => ("RIOB1", "O"),
        "IMUX.RIOB1.T" => ("RIOB1", "T"),
        "IMUX.BUFG" => ("BUFG", "I"),
        _ => panic!("umm {wn}?"),
    };
    let nloc = (die, col, row, LayerId::from_idx(0));
    let node = backend.egrid.node(nloc);
    let node_kind = &backend.egrid.db.nodes[node.kind];
    let nnode = &backend.ngrid.nodes[&nloc];
    let belid = node_kind.bels.get(bel).unwrap().0;
    let block = &nnode.bels[belid][0];
    if bel == "CLB" && pin == "K" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "FG")
            .base(Key::BelMutex(nloc, belid, "CLK".into()), pin)
            .fuzz(
                Key::BlockConfig(block, "CLK".into(), pin.into()),
                false,
                true,
            );
    }
    if pin == "T" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BelMutex(nloc, belid, "BUF".into()), "TRI")
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
    wire_to: NodeWireId,
    wire_from: NodeWireId,
}

impl IntPip {
    pub fn new(wire_to: NodeWireId, wire_from: NodeWireId) -> Self {
        Self { wire_to, wire_from }
    }
}

impl Prop for IntPip {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: prjcombine_int::grid::NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let rwt = backend
            .egrid
            .resolve_node_wire_nobuf(nloc, self.wire_to)
            .unwrap();
        let rwf = backend
            .egrid
            .resolve_node_wire_nobuf(nloc, self.wire_from)
            .unwrap();
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::NodeMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(nloc, self.wire_to, self.wire_from);
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

impl Prop for SingleBidi {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let wn = backend.egrid.db.wires.key(self.wire);
        let bidi_tile = match self.dir {
            Dir::W | Dir::E => {
                if nloc.2 == backend.edev.grid.row_bio() {
                    "BIDIH.B"
                } else if nloc.2 == backend.edev.grid.row_tio() {
                    "BIDIH.T"
                } else {
                    "BIDIH"
                }
            }
            Dir::S | Dir::N => {
                if nloc.1 == backend.edev.grid.col_lio() {
                    "BIDIV.L"
                } else if nloc.1 == backend.edev.grid.col_rio() {
                    "BIDIV.R"
                } else {
                    "BIDIV"
                }
            }
        };
        match self.dir {
            Dir::W => {
                if !backend.edev.grid.cols_bidi.contains(&nloc.1) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "W".into(),
                    },
                    tiles: vec![backend.edev.btile_llh(nloc.1, nloc.2)],
                });
                Some((fuzzer, false))
            }
            Dir::E => {
                if !backend.edev.grid.cols_bidi.contains(&(nloc.1 + 1)) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "E".into(),
                    },
                    tiles: vec![backend.edev.btile_llh(nloc.1 + 1, nloc.2)],
                });
                Some((fuzzer, false))
            }
            Dir::S => {
                if !backend.edev.grid.rows_bidi.contains(&nloc.2) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "S".into(),
                    },
                    tiles: vec![backend.edev.btile_llv(nloc.1, nloc.2)],
                });
                Some((fuzzer, false))
            }
            Dir::N => {
                if !backend.edev.grid.rows_bidi.contains(&(nloc.2 + 1)) {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: bidi_tile.into(),
                        bel: "INT".into(),
                        attr: format!("BIDI.{wn}"),
                        val: "N".into(),
                    },
                    tiles: vec![backend.edev.btile_llv(nloc.1, nloc.2 + 1)],
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

impl Prop for HasBidi {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let val = match self.dir {
            Dir::W => backend.edev.grid.cols_bidi.contains(&nloc.1),
            Dir::E => backend.edev.grid.cols_bidi.contains(&(nloc.1 + 1)),
            Dir::S => backend.edev.grid.rows_bidi.contains(&nloc.2),
            Dir::N => backend.edev.grid.rows_bidi.contains(&(nloc.2 + 1)),
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
    for (_, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (&wire_to, mux) in &node.muxes {
            let mux_name = format!("MUX.{}", intdb.wires.key(wire_to.1));
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = format!("{}.{}", wire_from.0, wire_from_name);
                let mut f = ctx
                    .build()
                    .test_manual("INT", &mux_name, &in_name)
                    .prop(IntPip::new(wire_to, wire_from));
                if mux_name.starts_with("MUX.SINGLE.H")
                    && matches!(&tile[..], "CLB" | "CLB.B" | "CLB.T")
                {
                    let (dir, wire) = if mux_name.ends_with(".E") {
                        let term = intdb.get_term("MAIN.W");
                        let TermInfo::PassFar(wire) = intdb.terms[term].wires[wire_to.1] else {
                            unreachable!()
                        };
                        (Dir::W, wire)
                    } else {
                        (Dir::E, wire_to.1)
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
                        let term = intdb.get_term("MAIN.N");
                        let TermInfo::PassFar(wire) = intdb.terms[term].wires[wire_to.1] else {
                            unreachable!()
                        };
                        (Dir::N, wire)
                    } else {
                        (Dir::S, wire_to.1)
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
        let snode = intdb.get_node(stile);
        let snode = &intdb.nodes[snode];
        for (wire, mux) in &snode.muxes {
            let wn = intdb.wires.key(wire.1);
            if wn.starts_with(filter) && !wn.ends_with(".E") && !wn.ends_with(".S") {
                let attr = &format!("BIDI.{wn}");
                if wn == "SINGLE.V.R0" {
                    let mux_name = format!("MUX.{wn}");
                    let mut diff_s = None;
                    for &wire_from in &mux.ins {
                        let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
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
    for (_, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        for (&wire_to, mux) in &node.muxes {
            assert_eq!(wire_to.0, NodeTileId::from_idx(0));
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = format!("MUX.{out_name}");

            if out_name.starts_with("IMUX") || out_name.starts_with("IOCLK") {
                let mut inps = vec![];
                let mut got_empty = false;
                for &wire_from in &mux.ins {
                    let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if diff.bits.is_empty() {
                        got_empty = true;
                    }
                    inps.push((in_name.to_string(), diff));
                }
                if out_name.starts_with("IMUX") && out_name.ends_with("T") {
                    let bel = &out_name[5..10];
                    assert!(!got_empty);
                    inps.push(("VCC".to_string(), Diff::default()));
                    inps.push((
                        "GND".to_string(),
                        ctx.state.get_diff(tile, bel, "BUF", "ON"),
                    ));
                } else if out_name == "IMUX.CLB.K" {
                    assert!(!got_empty);
                    inps.push(("NONE".to_string(), Diff::default()));
                    inps.push(("C".to_string(), ctx.state.get_diff(tile, "CLB", "CLK", "C")));
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
                ctx.tiledb.insert(tile, "INT", mux_name, item);
            } else {
                for &wire_from in &mux.ins {
                    let in_name = intdb.wires.key(wire_from.1);
                    assert_eq!(wire_from.0, NodeTileId::from_idx(0));
                    let val_name = format!("{}.{}", wire_from.0, in_name);
                    let diff =
                        ctx.state
                            .get_diff(tile, "INT", format!("MUX.{out_name}"), &val_name);
                    let item = xlat_bit(diff);
                    let mut is_bidi = false;
                    if let Some(omux) = node.muxes.get(&wire_from) {
                        if omux.ins.contains(&wire_to) {
                            is_bidi = true;
                        }
                    }
                    let name = if !is_bidi {
                        format!("PASS.{out_name}.{in_name}")
                    } else if wire_from < wire_to {
                        format!("BIPASS.{in_name}.{out_name}")
                    } else {
                        format!("BIPASS.{out_name}.{in_name}")
                    };
                    ctx.tiledb.insert(tile, "INT", name, item);
                }
            }
        }
    }
}
