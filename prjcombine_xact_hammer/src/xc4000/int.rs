use std::collections::{btree_map, BTreeMap, HashSet};

use prjcombine_collector::{xlat_bit, xlat_enum, xlat_enum_ocd, Diff, OcdMode};
use prjcombine_hammer::{Fuzzer, Session};
use prjcombine_interconnect::{
    db::{BelId, NodeTileId, NodeWireId},
    grid::{IntWire, LayerId, NodeLoc},
};
use prjcombine_types::tiledb::TileBit;
use prjcombine_xc2000::grid::GridKind;
use unnamed_entity::EntityId;

use crate::{
    backend::{Key, Value, XactBackend},
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
    wire_avoid: IntWire,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let grid = backend.edev.grid;
    let (die, (mut col, mut row), wt) = wire_target;
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (long_tbuf0, long_tbuf1) = if grid.kind == GridKind::Xc4000A {
        ("LONG.H1", "LONG.H2")
    } else {
        ("LONG.H2", "LONG.H3")
    };
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") {
        let (bel, mut pin) = match wtn {
            "OUT.CLB.FX" => ("CLB", "X"),
            "OUT.CLB.GY" => ("CLB", "Y"),
            "OUT.CLB.FXQ" => ("CLB", "XQ"),
            "OUT.CLB.GYQ" => ("CLB", "YQ"),
            "OUT.BT.IOB0.I1" => ("IOB0", "I1"),
            "OUT.BT.IOB0.I2" => ("IOB0", "I2"),
            "OUT.BT.IOB1.I1" if col == grid.col_lio() && row == grid.row_tio() => ("BSCAN", "SEL2"),
            "OUT.BT.IOB1.I2" if col == grid.col_lio() && row == grid.row_tio() => ("BSCAN", "DRCK"),
            "OUT.BT.IOB1.I1" if col == grid.col_lio() && row == grid.row_bio() => ("MD2", "I"),
            "OUT.BT.IOB1.I2" if col == grid.col_lio() && row == grid.row_bio() => ("RDBK", "RIP"),
            "OUT.BT.IOB1.I1" => ("IOB1", "I1"),
            "OUT.BT.IOB1.I2" => ("IOB1", "I2"),
            "OUT.LR.IOB0.I1" => ("IOB0", "I1"),
            "OUT.LR.IOB0.I2" => ("IOB0", "I2"),
            "OUT.LR.IOB1.I1" if col == grid.col_lio() && row == grid.row_tio() => ("BSCAN", "SEL1"),
            "OUT.LR.IOB1.I2" if col == grid.col_lio() && row == grid.row_tio() => ("BSCAN", "IDLE"),
            "OUT.LR.IOB1.I1" if col == grid.col_rio() && row == grid.row_tio() => ("OSC", "F8M"),
            "OUT.LR.IOB1.I2" if col == grid.col_rio() && row == grid.row_tio() => ("OSC", "OUT0"),
            "OUT.LR.IOB1.I1" => ("IOB1", "I1"),
            "OUT.LR.IOB1.I2" => ("IOB1", "I2"),
            "OUT.HIOB0.I" => ("HIOB0", "I"),
            "OUT.HIOB1.I" => ("HIOB1", "I"),
            "OUT.HIOB2.I" => ("HIOB2", "I"),
            "OUT.HIOB3.I" => ("HIOB3", "I"),
            "OUT.MD0.I" => ("MD0", "I"),
            "OUT.STARTUP.DONEIN" => ("STARTUP", "DONEIN"),
            "OUT.STARTUP.Q1Q4" => ("STARTUP", "Q1Q4"),
            "OUT.STARTUP.Q2" => ("STARTUP", "Q2"),
            "OUT.STARTUP.Q3" => ("STARTUP", "Q3"),
            "OUT.RDBK.DATA" => ("RDBK", "DATA"),
            "OUT.UPDATE.O" => ("UPDATE", "O"),
            "OUT.OSC.MUX1" => ("OSC", "OUT1"),
            "OUT.IOB.CLKIN" => (
                if grid.kind == GridKind::Xc4000H {
                    if col == grid.col_lio() {
                        if row < grid.row_mid() {
                            "HIOB3"
                        } else {
                            "HIOB0"
                        }
                    } else if col == grid.col_rio() {
                        if row < grid.row_mid() {
                            "HIOB2"
                        } else {
                            "HIOB0"
                        }
                    } else if row == grid.row_bio() {
                        if col < grid.col_mid() {
                            "HIOB0"
                        } else {
                            "HIOB3"
                        }
                    } else if row == grid.row_tio() {
                        if col < grid.col_mid() {
                            "HIOB0"
                        } else {
                            "HIOB2"
                        }
                    } else {
                        unreachable!()
                    }
                } else {
                    if col == grid.col_lio() {
                        if row < grid.row_mid() {
                            "IOB1"
                        } else {
                            "IOB0"
                        }
                    } else if col == grid.col_rio() {
                        "IOB0"
                    } else if row == grid.row_bio() {
                        if col < grid.col_mid() {
                            "IOB0"
                        } else {
                            "IOB1"
                        }
                    } else if row == grid.row_tio() {
                        "IOB0"
                    } else {
                        unreachable!()
                    }
                },
                "CLKIN",
            ),
            _ => panic!("umm {wtn}"),
        };
        if bel.starts_with("IOB") && grid.kind == GridKind::Xc4000H {
            (
                (die, col, row, LayerId::from_idx(0)),
                (NodeTileId::from_idx(0), wire_target.2),
                (
                    NodeTileId::from_idx(0),
                    backend.egrid.db.get_wire(if bel == "IOB0" {
                        "OUT.HIOB0.I"
                    } else {
                        "OUT.HIOB2.I"
                    }),
                ),
            )
        } else {
            let nloc = (die, col, row, LayerId::from_idx(0));
            let node = backend.egrid.node(nloc);
            let node_kind = &backend.egrid.db.nodes[node.kind];
            let nnode = &backend.ngrid.nodes[&nloc];
            let belid = node_kind.bels.get(bel).unwrap().0;
            let mut block = &nnode.bels[belid][0];
            if pin == "CLKIN" {
                block = &nnode.bels[belid][1];
                pin = "I";
            }
            if bel == "OSC" {
                let crd0 = backend.ngrid.bel_pip(nloc, belid, "OUT0.F500K");
                let crd1 = backend.ngrid.bel_pip(nloc, belid, "OUT1.F500K");
                fuzzer = fuzzer
                    .base(Key::BelMutex(nloc, belid, "MODE".into()), "USE")
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
        let nloc = (die, col, row, LayerId::from_idx(0));
        let nnode = &backend.ngrid.nodes[&nloc];
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &nnode.tie_names[0],
            "O",
        );
    } else if let Some(idx) = wtn.strip_prefix("GCLK") {
        let idx: usize = idx.parse().unwrap();
        let layer = backend
            .egrid
            .find_node_layer(die, (col, grid.row_mid()), |node| node.starts_with("LLV"))
            .unwrap();
        let nloc = (die, col, grid.row_mid(), layer);
        let (block, inp) = [
            ("bufgp_tl", "I.UL.V"),
            ("bufgp_bl", "I.LL.H"),
            ("bufgp_br", "I.LR.V"),
            ("bufgp_tr", "I.UR.H"),
        ][idx];
        let crd = backend
            .ngrid
            .bel_pip(nloc, BelId::from_idx(0), &format!("O{idx}.{inp}"));

        fuzzer = fuzzer
            .base(
                Key::BelMutex(nloc, BelId::from_idx(0), format!("O{idx}")),
                "USE",
            )
            .base(Key::Pip(crd), Value::FromPin(block, "O".into()));
        return (fuzzer, block, "O");
    } else if wtn.starts_with("DEC") {
        if wtn.starts_with("DEC.H") {
            if col == grid.col_lio() {
                col += 1;
            } else if col == grid.col_rio() {
                col -= 1;
            }
        } else {
            if row == grid.row_bio() {
                row += 1;
            } else if row == grid.row_tio() {
                row -= 1;
            }
        }
        let idx: usize = wtn[5..].parse().unwrap();
        let pin = ["O1", "O2", "O3", "O4"][idx];
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let node_kind = &backend.egrid.db.nodes[node.kind];
        let nnode = &backend.ngrid.nodes[&nloc];
        let belid = node_kind.bels.get("DEC0").unwrap().0;
        let block = &nnode.bels[belid][0];
        let crd = backend.ngrid.bel_pip(nloc, belid, pin);
        fuzzer = fuzzer
            .base(Key::Pip(crd), Value::FromPin(block, pin.into()))
            .base(Key::NodeMutex(wire_target), "SHARED_ROOT");
        return (fuzzer, block, pin);
    } else if (wtn == long_tbuf0 || wtn == long_tbuf1)
        && !(row == grid.row_bio() || row == grid.row_tio())
    {
        let bel = if wtn == long_tbuf0 { "TBUF0" } else { "TBUF1" };
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.node(nloc);
        let node_kind = &backend.egrid.db.nodes[node.kind];
        let nnode = &backend.ngrid.nodes[&nloc];
        let belid = node_kind.bels.get(bel).unwrap().0;
        let block = &nnode.bels[belid][0];
        let crd = backend.ngrid.bel_pip(nloc, belid, "O");
        fuzzer = fuzzer
            .base(Key::Pip(crd), Value::FromPin(block, "O".into()))
            .base(Key::NodeMutex(wire_target), "SHARED_ROOT");
        return (fuzzer, block, "O");
    } else if wtn.starts_with("SINGLE") || wtn.starts_with("DOUBLE") {
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
            panic!("ummm no out?")
        }
    } else if wtn.starts_with("LONG") || wtn.starts_with("IO.DOUBLE") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("SINGLE") {
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
    } else if wtn.starts_with("IO.DBUF") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let nloc = (w.0, w.1 .0, w.1 .1, LayerId::from_idx(0));
                let node = backend.egrid.node(nloc);
                let node_kind = &backend.egrid.db.nodes[node.kind];
                if let Some(mux) = node_kind.muxes.get(&(NodeTileId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("IO.DOUBLE") {
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
        panic!("ummm {wtn}?");
    };
    let nwt = backend.egrid.resolve_node_wire_nobuf(ploc, pwf).unwrap();
    let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
    let fuzzer = apply_int_pip(backend, ploc, pwt, pwf, block, pin, fuzzer);
    (fuzzer, block, pin)
}

#[allow(clippy::too_many_arguments)]
fn apply_imux_finish<'a>(
    backend: &XactBackend<'a>,
    wire: IntWire,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
    sblock: &'a str,
    spin: &'static str,
    hiob: usize,
    oq: bool,
    inv: bool,
) -> Fuzzer<XactBackend<'a>> {
    let grid = backend.edev.grid;
    let (die, (mut col, mut row), w) = wire;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if !wn.starts_with("IMUX") {
        return fuzzer;
    }
    let (mut bel, mut pin) = match wn {
        "IMUX.CLB.K" => ("CLB", "K"),
        "IMUX.CLB.F1" => {
            if col == grid.col_rio() {
                ("IOB1", "O2")
            } else {
                ("CLB", "F1")
            }
        }
        "IMUX.CLB.F2" => {
            row += 1;
            if row == grid.row_tio() {
                ("IOB0", "O2")
            } else {
                ("CLB", "F2")
            }
        }
        "IMUX.CLB.F3" => {
            col -= 1;
            if col == grid.col_lio() {
                ("IOB1", "O2")
            } else {
                ("CLB", "F3")
            }
        }
        "IMUX.CLB.F4" => {
            if row == grid.row_bio() {
                ("IOB0", "O2")
            } else {
                ("CLB", "F4")
            }
        }
        "IMUX.CLB.G1" => {
            if col == grid.col_rio() {
                ("IOB0", "O2")
            } else {
                ("CLB", "G1")
            }
        }
        "IMUX.CLB.G2" => {
            row += 1;
            if row == grid.row_tio() {
                ("IOB1", "O2")
            } else {
                ("CLB", "G2")
            }
        }
        "IMUX.CLB.G3" => {
            col -= 1;
            if col == grid.col_lio() {
                ("IOB0", "O2")
            } else {
                ("CLB", "G3")
            }
        }
        "IMUX.CLB.G4" => {
            if row == grid.row_bio() {
                ("IOB1", "O2")
            } else {
                ("CLB", "G4")
            }
        }
        "IMUX.CLB.C1" => {
            if col == grid.col_rio() {
                ("DEC1", "I")
            } else {
                ("CLB", "C1")
            }
        }
        "IMUX.CLB.C2" => {
            row += 1;
            if row == grid.row_tio() {
                ("DEC1", "I")
            } else {
                ("CLB", "C2")
            }
        }
        "IMUX.CLB.C3" => {
            col -= 1;
            if col == grid.col_lio() {
                ("DEC1", "I")
            } else {
                ("CLB", "C3")
            }
        }
        "IMUX.CLB.C4" => {
            if row == grid.row_bio() {
                ("DEC1", "I")
            } else {
                ("CLB", "C4")
            }
        }
        "IMUX.TBUF0.I" => ("TBUF0", "I"),
        "IMUX.TBUF0.TS" => ("TBUF0", "T"),
        "IMUX.TBUF1.I" => ("TBUF1", "I"),
        "IMUX.TBUF1.TS" => ("TBUF1", "T"),
        "IMUX.IOB1.O1" if col == grid.col_lio() && row == grid.row_bio() => ("MD1", "O"),
        "IMUX.IOB1.IK" if col == grid.col_lio() && row == grid.row_bio() => ("MD1", "T"),
        "IMUX.IOB0.OK" if grid.kind == GridKind::Xc4000H => ("HIOB0", "TS"),
        "IMUX.IOB0.IK" if grid.kind == GridKind::Xc4000H => ("HIOB1", "TS"),
        "IMUX.IOB1.IK" if grid.kind == GridKind::Xc4000H => ("HIOB2", "TS"),
        "IMUX.IOB1.OK" if grid.kind == GridKind::Xc4000H => ("HIOB3", "TS"),
        "IMUX.IOB0.TS" if grid.kind == GridKind::Xc4000H => ("HIOB0", "TP"),
        "IMUX.IOB1.TS" if grid.kind == GridKind::Xc4000H => ("HIOB2", "TP"),
        "IMUX.IOB0.IK" => ("IOB0", "IK"),
        "IMUX.IOB1.IK" => ("IOB1", "IK"),
        "IMUX.IOB0.OK" => ("IOB0", "OK"),
        "IMUX.IOB1.OK" => ("IOB1", "OK"),
        "IMUX.IOB0.TS" => ("IOB0", "T"),
        "IMUX.IOB1.TS" => ("IOB1", "T"),
        "IMUX.IOB0.O1" => ("IOB0", "O1"),
        "IMUX.IOB1.O1" => ("IOB1", "O1"),
        "IMUX.READCLK.I" => ("READCLK", "I"),
        "IMUX.RDBK.TRIG" => ("RDBK", "TRIG"),
        "IMUX.TDO.O" => ("TDO", "O"),
        "IMUX.TDO.T" => ("TDO", "T"),
        "IMUX.STARTUP.CLK" => ("STARTUP", "CLK"),
        "IMUX.STARTUP.GSR" => ("STARTUP", "GSR"),
        "IMUX.STARTUP.GTS" => ("STARTUP", "GTS"),
        "IMUX.BSCAN.TDO1" => ("BSCAN", "TDO1"),
        "IMUX.BSCAN.TDO2" => ("BSCAN", "TDO2"),
        "IMUX.BUFG.H" => ("BUFGLS.H", "I"),
        "IMUX.BUFG.V" => ("BUFGLS.V", "I"),
        _ => panic!("umm {wn}?"),
    };
    if grid.kind == GridKind::Xc4000H {
        if bel == "IOB0" {
            bel = ["HIOB0", "HIOB1"][hiob];
        }
        if bel == "IOB1" {
            bel = ["HIOB2", "HIOB3"][hiob];
        }
    }
    let nloc = (die, col, row, LayerId::from_idx(0));
    let node = backend.egrid.node(nloc);
    let node_kind = &backend.egrid.db.nodes[node.kind];
    let nnode = &backend.ngrid.nodes[&nloc];
    let belid = node_kind.bels.get(bel).unwrap().0;
    let block = &nnode.bels[belid][0];
    if bel.starts_with("HIOB") && pin == "TP" {
        let crd = backend.ngrid.bel_pip(nloc, belid, "T1");
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()));
    }
    if bel.starts_with("HIOB") && (pin == "O1" || pin == "O2") {
        let crd = backend.ngrid.bel_pip(nloc, belid, pin);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(sblock, spin.into()));
        pin = "O";
    }
    if bel.starts_with("IOB") && pin == "T" {
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "IO")
            .base(Key::BelMutex(nloc, belid, "OUT".into()), "O")
            .base(Key::BlockConfig(block, "OUT".into(), "O".into()), true)
            .base(Key::BelMutex(nloc, belid, "CLK".into()), "O")
            .base(Key::BlockConfig(block, "OUT".into(), "OK".into()), true)
            .fuzz(
                Key::BlockConfig(block, "TRI".into(), "T".into()),
                false,
                true,
            );
    }
    if bel.starts_with("IOB") && (pin == "O2" || pin == "O1") {
        let opin = if pin == "O1" { "O2" } else { "O1" };
        let opin = node_kind.bels[belid].pins[opin]
            .wires
            .iter()
            .copied()
            .next()
            .unwrap();
        let opin = backend.egrid.resolve_node_wire_nobuf(nloc, opin).unwrap();
        fuzzer = fuzzer
            .base(Key::NodeMutex(opin), "PROHIBIT")
            .base(Key::BlockBase(block), "IO")
            .base(Key::BelMutex(nloc, belid, "OUT".into()), "TEST")
            .base(Key::BelMutex(nloc, belid, "CLK".into()), "O")
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
    if bel.starts_with("TBUF") {
        let mode = if pin == "I" { "WAND" } else { "WANDT" };
        fuzzer = fuzzer
            .base(Key::BlockBase(block), "TBUF")
            .base(Key::BelMutex(nloc, belid, "TBUF".into()), mode)
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
    if bel == "STARTUP" && pin == "CLK" {
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
    wire_to: NodeWireId,
    wire_from: NodeWireId,
    hiob: usize,
    oq: bool,
    inv: bool,
}

impl IntPip {
    pub fn new(
        wire_to: NodeWireId,
        wire_from: NodeWireId,
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
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::NodeMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(nloc, self.wire_to, self.wire_from);
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
    pub wire: NodeWireId,
    pub dx: isize,
    pub dy: isize,
}

impl ClbSpecialMux {
    fn new(attr: &'static str, val: &'static str, wire: NodeWireId, dx: isize, dy: isize) -> Self {
        Self {
            attr,
            val,
            wire,
            dx,
            dy,
        }
    }
}

impl Prop for ClbSpecialMux {
    fn dyn_clone(&self) -> Box<dyn Prop> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &XactBackend<'a>,
        mut nloc: prjcombine_interconnect::grid::NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let imux = backend.egrid.node_wire(nloc, self.wire);
        fuzzer = fuzzer.base(Key::NodeMutex(imux), "PROHIBIT");
        nloc.1 += self.dx;
        nloc.2 += self.dy;
        let nnode = &backend.ngrid.nodes[&nloc];
        let block = &nnode.bels[BelId::from_idx(0)][0];
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
    for (_, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt}.{out_name}", wtt = wire_to.0)
            } else {
                assert_eq!(wire_to.0.to_idx(), 0);
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
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = format!("{}.{}", wire_from.0, wire_from_name);
                if is_iob_o {
                    if backend.edev.grid.kind == GridKind::Xc4000H {
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
                    (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.CLB.F4"),
                    ),
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
                    (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.CLB.G3"),
                    ),
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
                    (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.CLB.G2"),
                    ),
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
    for (_, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        let mut hiob_o_diffs: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        let mut mux_diffs: BTreeMap<NodeWireId, BTreeMap<NodeWireId, Diff>> = BTreeMap::new();
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt}.{out_name}", wtt = wire_to.0)
            } else {
                format!("MUX.{out_name}")
            };
            let mut iob_o = None;
            if out_name.starts_with("IMUX.IO") && out_name.ends_with("O1") && tile.starts_with("IO")
            {
                iob_o = if out_name == "IMUX.IOB0.O1" {
                    Some((&tile[..], "IOB0", "O1", 0))
                } else {
                    Some((&tile[..], "IOB1", "O1", 0))
                };
            }
            if (out_name == "IMUX.CLB.F4" || out_name == "IMUX.CLB.G4") && tile.starts_with("IO.B")
            {
                iob_o = if out_name == "IMUX.CLB.F4" {
                    Some((&tile[..], "IOB0", "O2", 0))
                } else {
                    Some((&tile[..], "IOB1", "O2", 0))
                };
            }
            if (out_name == "IMUX.CLB.F2" || out_name == "IMUX.CLB.G2")
                && tile.starts_with("CLB")
                && tile.ends_with('T')
            {
                iob_o = if out_name == "IMUX.CLB.F2" {
                    Some(("IO.T", "IOB0", "O2", 3))
                } else {
                    Some(("IO.T", "IOB1", "O2", 3))
                };
            }
            if (out_name == "IMUX.CLB.F3" || out_name == "IMUX.CLB.G3") && tile.starts_with("CLB.L")
            {
                iob_o = if out_name == "IMUX.CLB.G3" {
                    Some(("IO.L", "IOB0", "O2", 2))
                } else {
                    Some(("IO.L", "IOB1", "O2", 2))
                };
            }
            if (out_name == "IMUX.CLB.F1" || out_name == "IMUX.CLB.G1") && tile.starts_with("IO.R")
            {
                iob_o = if out_name == "IMUX.CLB.G1" {
                    Some((&tile[..], "IOB0", "O2", 0))
                } else {
                    Some((&tile[..], "IOB1", "O2", 0))
                };
            }
            if let Some((prefix, bel, pin, bt)) = iob_o {
                if ctx.edev.grid.kind == GridKind::Xc4000H {
                    let mut inps = vec![];
                    let mut got_empty = false;
                    for &wire_from in &mux.ins {
                        let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                        let diff0 =
                            ctx.state
                                .get_diff(tile, "INT", &mux_name, format!("{in_name}.HIOB0"));
                        let diff1 =
                            ctx.state
                                .get_diff(tile, "INT", &mux_name, format!("{in_name}.HIOB1"));
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
                        let hiob = if bel == "IOB0" {
                            ["HIOB0", "HIOB1"]
                        } else {
                            ["HIOB2", "HIOB3"]
                        };
                        for (bel, diff) in [(hiob[0], diff0), (hiob[1], diff1)] {
                            match hiob_o_diffs.entry((prefix, bel)).or_default().entry(pin) {
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
                        for &wire_from in &mux.ins {
                            let in_name =
                                format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
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
                for &wire_from in &mux.ins {
                    let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                    t_inps.push((
                        in_name.to_string(),
                        ctx.state.get_diff(tile, "INT", &mux_name, &in_name),
                    ));
                }
                let imux_i = (
                    NodeTileId::from_idx(0),
                    intdb.get_wire(&format!("IMUX.TBUF{idx}.I")),
                );
                let mux_name_i = format!("MUX.IMUX.TBUF{idx}.I");
                let mux_i = &node.muxes[&imux_i];
                let mut i_inps = vec![];
                for &wire_from in &mux_i.ins {
                    let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
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
                ctx.tiledb
                    .insert(tile, format!("TBUF{idx}"), "DRIVE1", xlat_bit(!common));
            } else if !out_name.starts_with("IMUX")
                && !out_name.starts_with("IO.DBUF")
                && !out_name.starts_with("OUT")
            {
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
            } else {
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
                ctx.tiledb.insert(tile, "INT", mux_name, item);
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
                if diff.bits.len() != 1 {
                    continue;
                }
                let bit = *diff.bits.iter().next().unwrap().0;
                let mut unique = true;
                for (&owf, odiff) in ins {
                    if owf != wire_from && odiff.bits.contains_key(&bit) {
                        unique = false;
                    }
                }
                if !unique {
                    continue;
                }
                handled.insert((wire_to, wire_from));
                let diff = diff.clone();
                let oname = if tile.starts_with("LL") {
                    format!("{}.{}", wire_to.0, wtname)
                } else {
                    wtname.to_string()
                };
                let iname = format!("{}.{}", wire_from.0, wfname);
                if wtname.starts_with("SINGLE")
                    || wtname.starts_with("DOUBLE")
                    || wtname.starts_with("IO.DOUBLE")
                {
                    ctx.tiledb
                        .insert(tile, "INT", format!("PASS.{oname}.{iname}"), xlat_bit(diff));
                } else if wtname.starts_with("LONG") {
                    ctx.tiledb
                        .insert(tile, "INT", format!("BUF.{oname}.{iname}"), xlat_bit(diff));
                } else {
                    println!("MEOW {tile} {oname} {iname}");
                }
            }
        }

        for (wire_to, ins) in mux_diffs {
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt}.{out_name}", wtt = wire_to.0)
            } else {
                format!("MUX.{out_name}")
            };
            let mut in_diffs = vec![];
            let mut got_empty = false;
            for (wire_from, diff) in ins {
                if handled.contains(&(wire_to, wire_from)) {
                    continue;
                }
                let wfname = intdb.wires.key(wire_from.1);
                let in_name = format!("{}.{}", wire_from.0, wfname);
                if diff.bits.is_empty() {
                    got_empty = true;
                }
                in_diffs.push((in_name, diff));
            }
            if in_diffs.is_empty() {
                continue;
            }
            if !got_empty {
                in_diffs.push(("NONE".to_string(), Diff::default()));
            }
            ctx.tiledb
                .insert(tile, "INT", mux_name, xlat_enum_ocd(in_diffs, OcdMode::Mux));
        }

        for ((prefix, bel), diffs) in hiob_o_diffs {
            let diffs = Vec::from_iter(diffs);
            let item = xlat_enum(diffs);
            if prefix == tile {
                ctx.tiledb.insert(tile, bel, "MUX.O", item);
            } else {
                for tile in intdb.nodes.keys() {
                    if tile.starts_with(prefix) {
                        ctx.tiledb.insert(tile, bel, "MUX.O", item.clone());
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
        for tile in intdb.nodes.keys() {
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
        for tile in intdb.nodes.keys() {
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
        for tile in intdb.nodes.keys() {
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
        for tile in intdb.nodes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "OMUX", item.clone());
            }
        }

        let item = xlat_bit(diff_off_used);
        for tile in intdb.nodes.keys() {
            if tile.starts_with(prefix) {
                ctx.tiledb.insert(tile, bel, "OFF_USED", item.clone());
            }
        }
    }
}
