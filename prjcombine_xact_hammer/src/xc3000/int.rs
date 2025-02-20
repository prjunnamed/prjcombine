use std::collections::{BTreeMap, HashSet};

use prjcombine_collector::{xlat_bit, xlat_enum_ocd, Diff, FeatureId, OcdMode};
use prjcombine_hammer::{Fuzzer, Session};
use prjcombine_interconnect::{
    db::{NodeTileId, NodeWireId},
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
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: IntWire,
    wire_avoid: &mut Vec<IntWire>,
    root: bool,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let grid = backend.edev.grid;
    let (die, (mut col, mut row), wt) = wire_target;
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") || wtn == "GCLK" || wtn == "ACLK" {
        let (bel, pin) = match wtn {
            "OUT.CLB.X" => ("CLB", "X"),
            "OUT.CLB.Y" => ("CLB", "Y"),
            "OUT.BIOB0.I" => ("BIOB0", "I"),
            "OUT.BIOB0.Q" => ("BIOB0", "Q"),
            "OUT.BIOB1.I" => ("BIOB1", "I"),
            "OUT.BIOB1.Q" => ("BIOB1", "Q"),
            "OUT.TIOB0.I" => ("TIOB0", "I"),
            "OUT.TIOB0.Q" => ("TIOB0", "Q"),
            "OUT.TIOB1.I" => ("TIOB1", "I"),
            "OUT.TIOB1.Q" => ("TIOB1", "Q"),
            "OUT.LIOB0.I" => ("LIOB0", "I"),
            "OUT.LIOB0.Q" => ("LIOB0", "Q"),
            "OUT.LIOB1.I" => ("LIOB1", "I"),
            "OUT.LIOB1.Q" => ("LIOB1", "Q"),
            "OUT.RIOB0.I" => ("RIOB0", "I"),
            "OUT.RIOB0.Q" => ("RIOB0", "Q"),
            "OUT.RIOB1.I" => ("RIOB1", "I"),
            "OUT.RIOB1.Q" => ("RIOB1", "Q"),
            "OUT.OSC" => ("OSC", "O"),
            "OUT.CLKIOB" => ("CLKIOB", "I"),
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
        let belid = node_kind.bels.get(bel).unwrap().0;
        let block = &nnode.bels[belid][0];
        if bel == "OSC" {
            fuzzer = fuzzer.base(Key::GlobalOpt("XTALOSC".into()), "ENABLE");
            if root {
                let wires = [
                    backend.egrid.db.get_wire("SINGLE.V.R3"),
                    backend.egrid.db.get_wire("SINGLE.H.B3"),
                ];
                let wire = if wire_avoid[0].2 == wires[0] {
                    wires[1]
                } else {
                    wires[0]
                };
                let crd = backend.ngrid.int_pip(
                    nloc,
                    (NodeTileId::from_idx(0), wire),
                    (NodeTileId::from_idx(0), wire_target.2),
                );
                let rw = backend
                    .egrid
                    .resolve_node_wire_nobuf(nloc, (NodeTileId::from_idx(0), wire))
                    .unwrap();
                fuzzer = fuzzer
                    .base(Key::Pip(crd), Value::FromPin(block, pin.into()))
                    .base(Key::NodeMutex(rw), "OSC_HOOK");
            }
        }
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            block,
            pin,
        );
    } else if wtn.starts_with("LONG.H") {
        let bel = if wtn == "LONG.H0" { "TBUF0" } else { "TBUF1" };
        let pin = "O";
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let node_kind = &backend.egrid.db.nodes[node.kind];
        let nnode = &backend.ngrid.nodes[&nloc];
        let belid = node_kind.bels.get(bel).unwrap().0;
        let crd = backend.ngrid.bel_pip(nloc, belid, "O");
        let block = &nnode.bels[belid][0];
        fuzzer = fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()));
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            block,
            pin,
        );
    } else if wtn == "ACLK.V" || wtn == "GCLK.V" || wtn.starts_with("IOCLK") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).ends_with("CLK") {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
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
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("SINGLE") {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
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
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("OUT")
                            || backend.egrid.db.wires.key(inp.1).starts_with("LONG.H")
                        {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
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
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if (backend.egrid.db.wires.key(inp.1).starts_with("SINGLE.V")
                            && !backend.egrid.db.wires.key(inp.1).ends_with(".STUB"))
                            || backend.egrid.db.wires.key(inp.1).starts_with("OUT")
                        {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if !wire_avoid.contains(&rwf) {
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
                        if backend.egrid.db.wires.key(inp.1).starts_with("SINGLE.H") {
                            let rwf = backend.egrid.resolve_node_wire_nobuf(nloc, inp).unwrap();
                            if !wire_avoid.contains(&rwf) {
                                break 'a (nloc, (NodeTileId::from_idx(0), w.2), inp);
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
    let nwt = backend.egrid.resolve_node_wire_nobuf(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid, false);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: IntWire,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
    inv: bool,
) -> Fuzzer<XactBackend<'a>> {
    let grid = backend.edev.grid;
    let (die, (col, row), w) = wire;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if wn.starts_with("IOCLK") {
        let bel = &format!("{l}IOB0", l = &wn[6..7]);
        let pin = if wn.ends_with('0') { "OK" } else { "IK" };
        let (col, row) = match wn {
            "IOCLK.B0" => (grid.col_lio(), grid.row_bio()),
            "IOCLK.B1" => (grid.col_rio(), grid.row_bio()),
            "IOCLK.T0" => (grid.col_rio(), grid.row_tio()),
            "IOCLK.T1" => (grid.col_lio(), grid.row_tio()),
            "IOCLK.L0" => (grid.col_lio(), grid.row_tio()),
            "IOCLK.L1" => (grid.col_lio(), grid.row_bio()),
            "IOCLK.R0" => (grid.col_rio(), grid.row_bio()),
            "IOCLK.R1" => (grid.col_rio(), grid.row_tio()),
            _ => unreachable!(),
        };
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let node_kind = &backend.egrid.db.nodes[node.kind];
        let nnode = &backend.ngrid.nodes[&nloc];
        let belid = node_kind.bels.get(bel).unwrap().0;
        let block = &nnode.bels[belid][0];
        let wire_pin = node_kind.bels[belid].pins[pin]
            .wires
            .iter()
            .copied()
            .next()
            .unwrap();
        let crd = backend
            .ngrid
            .int_pip(nloc, wire_pin, (NodeTileId::from_idx(0), wire.2));
        if &fuzzer.info.features[0].id.tile != backend.egrid.db.nodes.key(node.kind) {
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: backend.egrid.db.nodes.key(node.kind).clone(),
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
            .base(Key::BelMutex(nloc, belid, "TRI".into()), "GND")
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
    let (bel, pin) = match wn {
        "IMUX.CLB.A" => ("CLB", "A"),
        "IMUX.CLB.B" => ("CLB", "B"),
        "IMUX.CLB.C" => ("CLB", "C"),
        "IMUX.CLB.D" => ("CLB", "D"),
        "IMUX.CLB.E" => ("CLB", "E"),
        "IMUX.CLB.DI" => ("CLB", "DI"),
        "IMUX.CLB.EC" => ("CLB", "EC"),
        "IMUX.CLB.RD" => ("CLB", "RD"),
        "IMUX.CLB.K" => ("CLB", "K"),
        "IMUX.BIOB0.O" => ("BIOB0", "O"),
        "IMUX.BIOB0.T" => ("BIOB0", "T"),
        "IMUX.BIOB0.IK" => ("BIOB0", "IK"),
        "IMUX.BIOB0.OK" => ("BIOB0", "OK"),
        "IMUX.BIOB1.O" => ("BIOB1", "O"),
        "IMUX.BIOB1.T" => ("BIOB1", "T"),
        "IMUX.BIOB1.IK" => ("BIOB1", "IK"),
        "IMUX.BIOB1.OK" => ("BIOB1", "OK"),
        "IMUX.TIOB0.O" => ("TIOB0", "O"),
        "IMUX.TIOB0.T" => ("TIOB0", "T"),
        "IMUX.TIOB0.IK" => ("TIOB0", "IK"),
        "IMUX.TIOB0.OK" => ("TIOB0", "OK"),
        "IMUX.TIOB1.O" => ("TIOB1", "O"),
        "IMUX.TIOB1.T" => ("TIOB1", "T"),
        "IMUX.TIOB1.IK" => ("TIOB1", "IK"),
        "IMUX.TIOB1.OK" => ("TIOB1", "OK"),
        "IMUX.LIOB0.O" => ("LIOB0", "O"),
        "IMUX.LIOB0.T" => ("LIOB0", "T"),
        "IMUX.LIOB0.IK" => ("LIOB0", "IK"),
        "IMUX.LIOB0.OK" => ("LIOB0", "OK"),
        "IMUX.LIOB1.O" => ("LIOB1", "O"),
        "IMUX.LIOB1.T" => ("LIOB1", "T"),
        "IMUX.LIOB1.IK" => ("LIOB1", "IK"),
        "IMUX.LIOB1.OK" => ("LIOB1", "OK"),
        "IMUX.RIOB0.O" => ("RIOB0", "O"),
        "IMUX.RIOB0.T" => ("RIOB0", "T"),
        "IMUX.RIOB0.IK" => ("RIOB0", "IK"),
        "IMUX.RIOB0.OK" => ("RIOB0", "OK"),
        "IMUX.RIOB1.O" => ("RIOB1", "O"),
        "IMUX.RIOB1.T" => ("RIOB1", "T"),
        "IMUX.RIOB1.IK" => ("RIOB1", "IK"),
        "IMUX.RIOB1.OK" => ("RIOB1", "OK"),
        "IMUX.TBUF0.I" => ("TBUF0", "I"),
        "IMUX.TBUF0.T" => ("TBUF0", "T"),
        "IMUX.TBUF1.I" => ("TBUF1", "I"),
        "IMUX.TBUF1.T" => ("TBUF1", "T"),
        "IMUX.TBUF2.I" => ("TBUF2", "I"),
        "IMUX.TBUF2.T" => ("TBUF2", "T"),
        "IMUX.TBUF3.I" => ("TBUF3", "I"),
        "IMUX.TBUF3.T" => ("TBUF3", "T"),
        "IMUX.BUFG" => ("BUFG", "I"),
        _ => panic!("umm {wn}?"),
    };
    let nloc = (die, col, row, LayerId::from_idx(0));
    let node = backend.egrid.node(nloc);
    let node_kind = &backend.egrid.db.nodes[node.kind];
    let nnode = &backend.ngrid.nodes[&nloc];
    let belid = node_kind.bels.get(bel).unwrap().0;
    let block = &nnode.bels[belid][0];
    if pin == "T" && bel.contains("IOB") {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BlockConfig(block, "IN".into(), "I".into()), true)
            .base(Key::BelMutex(nloc, belid, "TRI".into()), "T")
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
            .base(Key::BelMutex(nloc, belid, "TRI".into()), "GND")
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
    wire_to: NodeWireId,
    wire_from: NodeWireId,
    inv: bool,
}

impl IntPip {
    pub fn new(wire_to: NodeWireId, wire_from: NodeWireId, inv: bool) -> Self {
        Self {
            wire_to,
            wire_from,
            inv,
        }
    }
}

impl Prop for IntPip {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: prjcombine_interconnect::grid::NodeLoc,
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
        let mut wire_avoid = vec![rwt];
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, &mut wire_avoid, true);
        fuzzer = fuzzer.fuzz(Key::NodeMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(nloc, self.wire_to, self.wire_from);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin, self.inv);
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ProhibitInt {
    wire: NodeWireId,
}

impl ProhibitInt {
    pub fn new(wire: NodeWireId) -> Self {
        Self { wire }
    }
}

impl Prop for ProhibitInt {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        nloc: prjcombine_interconnect::grid::NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let rw = backend
            .egrid
            .resolve_node_wire_nobuf(nloc, self.wire)
            .unwrap();
        fuzzer = fuzzer.base(Key::NodeMutex(rw), "PROHIBIT");
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.egrid.db;
    for (_, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for (&wire_to, mux) in &node.muxes {
            let wire_to_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{}.{}", wire_to.0, wire_to_name)
            } else {
                format!("MUX.{}", wire_to_name)
            };
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = format!("{}.{}", wire_from.0, wire_from_name);
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
                let t_name = format!("{}.T", &intdb.wires.key(wire_to.1)[..10]);
                let wire_long = (NodeTileId::from_idx(0), intdb.get_wire(long_name));
                let wire_t = (NodeTileId::from_idx(0), intdb.get_wire(&t_name));
                for &wire_from in &mux.ins {
                    let wire_from_name = intdb.wires.key(wire_from.1);
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
        for bel in node.bels.keys() {
            if bel.contains("IOB") && bel != "CLKIOB" {
                let mut bctx = ctx.bel(bel);
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
            if bel.contains("PULLUP.TBUF") {
                let mut bctx = ctx.bel(bel);
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
    let grid = ctx.edev.grid;
    for (_, tile, node) in &intdb.nodes {
        if !ctx.has_tile(tile) {
            continue;
        }
        let mut mux_diffs: BTreeMap<NodeWireId, BTreeMap<NodeWireId, Diff>> = BTreeMap::new();
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt}.{out_name}", wtt = wire_to.0)
            } else {
                format!("MUX.{out_name}")
            };
            if out_name.starts_with("IOCLK") {
                let empty = grid.is_small
                    && matches!(
                        &out_name[..],
                        "IOCLK.B0" | "IOCLK.B1" | "IOCLK.L0" | "IOCLK.L1"
                    );
                for &wire_from in &mux.ins {
                    let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    diff.assert_empty();
                    let diff = ctx
                        .state
                        .get_diff(tile, "INT", &mux_name, format!("{in_name}.INV"));
                    if in_name.ends_with("CLK") && empty {
                        diff.assert_empty();
                    } else {
                        let item = xlat_bit(diff);
                        ctx.tiledb
                            .insert(tile, "INT", format!("INV.{out_name}"), item);
                    }
                }
            } else if out_name == "GCLK.V" && grid.is_small {
                for &wire_from in &mux.ins {
                    let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    diff.assert_empty();
                }
            } else if out_name.starts_with("IMUX") {
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
                if out_name.ends_with(".T") {
                    if out_name.starts_with("IMUX.TBUF") {
                        let diff = ctx.state.get_diff(tile, "INT", &mux_name, "GND");
                        inps.push(("GND".to_string(), diff));
                    } else {
                        let bel = &out_name[5..10];
                        let diff = ctx.state.get_diff(tile, bel, "OUT", "O");
                        inps.push(("GND".to_string(), diff));

                        let mut diff_i = ctx.state.get_diff(tile, bel, "IN", "I");
                        let mut diff_pullup = ctx.state.get_diff(tile, bel, "IN", "PULLUP");
                        if tile.starts_with("CLB.BR") && (bel == "BIOB1" || bel == "RIOB0") {
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
                                .insert(tile, bel, "PULLUP", xlat_bit(!diff_i_spec));
                        }
                        assert_eq!(diff_i, !&diff_pullup);
                        inps.push(("PULLUP".to_string(), diff_pullup));
                    }
                    inps.push(("VCC".to_string(), Diff::default()));
                    got_empty = true;
                }
                if out_name.starts_with("IMUX.IOCLK") {
                    let val = match (&tile[..6], &out_name[..], grid.is_small) {
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
                    println!("UMMM MUX {tile} {mux_name} is empty");
                }
                ctx.tiledb.insert(tile, "INT", mux_name, item);
            } else {
                for &wire_from in &mux.ins {
                    let wfname = intdb.wires.key(wire_from.1);
                    let in_name = format!("{}.{}", wire_from.0, wfname);
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if diff.bits.is_empty() {
                        panic!("weird lack of bits: {tile} {out_name} {wfname}");
                    }
                    mux_diffs
                        .entry(wire_to)
                        .or_default()
                        .insert(wire_from, diff);
                }
            }
        }

        let mut handled = HashSet::new();
        for (&wire_to, ins) in &mux_diffs {
            let wtname = intdb.wires.key(wire_to.1);
            for (&wire_from, diff) in ins {
                if handled.contains(&(wire_to, wire_from)) {
                    continue;
                }
                let wfname = intdb.wires.key(wire_from.1);
                assert_eq!(diff.bits.len(), 1);
                if let Some(oins) = mux_diffs.get(&wire_from) {
                    if let Some(odiff) = oins.get(&wire_to) {
                        if odiff == diff {
                            assert_eq!(diff.bits.len(), 1);
                            handled.insert((wire_to, wire_from));
                            handled.insert((wire_from, wire_to));
                            let diff = diff.clone();
                            let name = if tile.starts_with("LL") {
                                format!(
                                    "BIPASS.{}.{}.{}.{}",
                                    wire_to.0, wtname, wire_from.0, wfname
                                )
                            } else {
                                assert_eq!(wire_to.0.to_idx(), 0);
                                assert_eq!(wire_from.0.to_idx(), 0);
                                format!("BIPASS.{}.{}", wtname, wfname)
                            };
                            ctx.tiledb.insert(tile, "INT", name, xlat_bit(diff));
                            continue;
                        }
                    }
                }
                handled.insert((wire_to, wire_from));
                let diff = diff.clone();
                let oname = if tile.starts_with("LL") {
                    format!("{}.{}", wire_to.0, wtname)
                } else {
                    wtname.to_string()
                };
                let iname = format!("{}.{}", wire_from.0, wfname);
                if (wtname.starts_with("SINGLE") && wfname.starts_with("SINGLE"))
                    || wtname.starts_with("LONG")
                    || wtname == "ACLK.V"
                    || wtname == "GCLK.V"
                {
                    ctx.tiledb
                        .insert(tile, "INT", format!("BUF.{oname}.{iname}"), xlat_bit(diff));
                } else {
                    ctx.tiledb
                        .insert(tile, "INT", format!("PASS.{oname}.{iname}"), xlat_bit(diff));
                }
            }
        }

        if tile.starts_with("CLB.BLS") {
            ctx.collect_enum_bool(tile, "INT", "INV.IOCLK.B0", "0", "1");
            ctx.collect_enum_bool(tile, "INT", "INV.IOCLK.L1", "0", "1");
        }
        if tile.starts_with("CLB.BRS") {
            ctx.collect_enum_bool(tile, "INT", "INV.IOCLK.B1", "0", "1");
        }
        if tile.starts_with("CLB.TLS") {
            ctx.collect_enum_bool(tile, "INT", "INV.IOCLK.L0", "0", "1");
        }

        for bel in node.bels.keys() {
            if bel.contains("PULLUP.TBUF") {
                ctx.collect_bit(tile, bel, "ENABLE", "1");
            }
        }
    }
}
