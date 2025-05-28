use prjcombine_interconnect::{
    db::{TileCellId, TileClassWire, WireKind},
    grid::{WireCoord, LayerId, NodeLoc},
};
use prjcombine_re_fpga_hammer::{
    Diff, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_xc2000::bels::xc5200 as bels;
use prjcombine_xilinx_bitstream::BitTile;
use unnamed_entity::EntityId;

use crate::{
    backend::{Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    props::DynProp,
};

fn apply_int_pip<'a>(
    backend: &XactBackend<'a>,
    nloc: NodeLoc,
    wire_to: TileClassWire,
    wire_from: TileClassWire,
    block: &'a str,
    pin: &'static str,
    mut fuzzer: Fuzzer<XactBackend<'a>>,
) -> Fuzzer<XactBackend<'a>> {
    let rwf = backend
        .egrid
        .resolve_tile_wire_nobuf(nloc, wire_from)
        .unwrap();
    let rwt = backend
        .egrid
        .resolve_tile_wire_nobuf(nloc, wire_to)
        .unwrap();
    fuzzer = fuzzer.base(Key::NodeMutex(rwt), rwf);
    let crd = backend.ngrid.int_pip(nloc, wire_to, wire_from);
    fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()))
}

fn drive_wire<'a>(
    backend: &XactBackend<'a>,
    fuzzer: Fuzzer<XactBackend<'a>>,
    wire_target: WireCoord,
    wire_avoid: WireCoord,
) -> (Fuzzer<XactBackend<'a>>, &'a str, &'static str) {
    let grid = backend.edev.chip;
    let (die, (mut col, mut row), wt) = wire_target;
    let wtn = &backend.egrid.db.wires.key(wt)[..];
    let (ploc, pwt, pwf) = if wtn.starts_with("OUT") {
        let (slot, pin) = match wtn {
            "OUT.LC0.X" => (bels::LC0, "LC0.X"),
            "OUT.LC0.Q" => (bels::LC0, "LC0.Q"),
            "OUT.LC0.DO" => (bels::LC0, "LC0.DO"),
            "OUT.LC1.X" => (bels::LC0, "LC1.X"),
            "OUT.LC1.Q" => (bels::LC0, "LC1.Q"),
            "OUT.LC1.DO" => (bels::LC0, "LC1.DO"),
            "OUT.LC2.X" => (bels::LC0, "LC2.X"),
            "OUT.LC2.Q" => (bels::LC0, "LC2.Q"),
            "OUT.LC2.DO" => (bels::LC0, "LC2.DO"),
            "OUT.LC3.X" => (bels::LC0, "LC3.X"),
            "OUT.LC3.Q" => (bels::LC0, "LC3.Q"),
            "OUT.LC3.DO" => (bels::LC0, "LC3.DO"),
            "OUT.PWRGND" => (bels::LC0, "CV"),
            "OUT.TBUF0" => (bels::TBUF0, "O"),
            "OUT.TBUF1" => (bels::TBUF1, "O"),
            "OUT.TBUF2" => (bels::TBUF2, "O"),
            "OUT.TBUF3" => (bels::TBUF3, "O"),
            "OUT.IO0.I" => (bels::IO0, "I"),
            "OUT.IO1.I" => (bels::IO1, "I"),
            "OUT.IO2.I" => (bels::IO2, "I"),
            "OUT.IO3.I" => (bels::IO3, "I"),
            "OUT.CLKIOB" => (bels::CLKIOB, "I"),
            "OUT.RDBK.RIP" => (bels::RDBK, "RIP"),
            "OUT.RDBK.DATA" => (bels::RDBK, "DATA"),
            "OUT.STARTUP.DONEIN" => (bels::STARTUP, "DONEIN"),
            "OUT.STARTUP.Q1Q4" => (bels::STARTUP, "Q1Q4"),
            "OUT.STARTUP.Q2" => (bels::STARTUP, "Q2"),
            "OUT.STARTUP.Q3" => (bels::STARTUP, "Q3"),
            "OUT.BSCAN.DRCK" => (bels::BSCAN, "DRCK"),
            "OUT.BSCAN.IDLE" => (bels::BSCAN, "IDLE"),
            "OUT.BSCAN.RESET" => (bels::BSCAN, "RESET"),
            "OUT.BSCAN.SEL1" => (bels::BSCAN, "SEL1"),
            "OUT.BSCAN.SEL2" => (bels::BSCAN, "SEL2"),
            "OUT.BSCAN.SHIFT" => (bels::BSCAN, "SHIFT"),
            "OUT.BSCAN.UPDATE" => (bels::BSCAN, "UPDATE"),
            "OUT.BSUPD" => (bels::OSC, "BSUPD"),
            "OUT.OSC.OSC1" => (bels::OSC, "OSC1"),
            "OUT.OSC.OSC2" => (bels::OSC, "OSC2"),
            "OUT.TOP.COUT" => {
                row -= 1;
                (bels::LC0, "CO")
            }
            _ => panic!("umm {wtn}"),
        };
        let nloc = (die, col, row, LayerId::from_idx(0));
        let nnode = &backend.ngrid.nodes[&nloc];
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &nnode.bels[slot][0],
            pin,
        );
    } else if wtn == "GND" {
        let nloc = (die, col, row, LayerId::from_idx(0));
        let nnode = &backend.ngrid.nodes[&nloc];
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &nnode.tie_names[0],
            "O",
        );
    } else if matches!(wtn, "GLOBAL.B" | "GLOBAL.T" | "GLOBAL.L" | "GLOBAL.R") {
        let nloc = (die, col, row, LayerId::from_idx(0));
        let nwt = backend
            .egrid
            .resolve_tile_wire_nobuf(
                nloc,
                (
                    TileCellId::from_idx(0),
                    backend.egrid.db.get_wire("IMUX.GIN"),
                ),
            )
            .unwrap();
        let (fuzzer, block, pin) = drive_wire(backend, fuzzer, nwt, wire_avoid);
        let fuzzer = fuzzer.base(Key::NodeMutex(wire_target), nwt);
        let crd = backend.ngrid.bel_pip((die, (col, row), bels::BUFR), "BUF");
        let fuzzer = fuzzer.base(Key::Pip(crd), Value::FromPin(block, pin.into()));
        return (fuzzer, block, pin);
    } else if matches!(wtn, "GLOBAL.BL" | "GLOBAL.TL" | "GLOBAL.BR" | "GLOBAL.TR") {
        let nloc = (die, col, row, LayerId::from_idx(0));
        let nnode = &backend.ngrid.nodes[&nloc];
        return (
            fuzzer.base(Key::NodeMutex(wire_target), "SHARED_ROOT"),
            &nnode.bels[bels::BUFG][0],
            "O",
        );
    } else if wtn == "IMUX.GIN" {
        (
            (die, col, row, LayerId::from_idx(0)),
            (TileCellId::from_idx(0), wire_target.2),
            (TileCellId::from_idx(0), backend.egrid.db.get_wire("GND")),
        )
    } else if let Some(nwt) = wtn.strip_suffix(".BUF") {
        (
            (die, col, row, LayerId::from_idx(0)),
            (TileCellId::from_idx(0), wire_target.2),
            (TileCellId::from_idx(0), backend.egrid.db.get_wire(nwt)),
        )
    } else if wtn.starts_with("OMUX") {
        let nwt = if col == grid.col_w()
            || col == grid.col_e()
            || row == grid.row_s()
            || row == grid.row_n()
        {
            "OUT.IO0.I"
        } else {
            "OUT.PWRGND"
        };
        (
            (die, col, row, LayerId::from_idx(0)),
            (TileCellId::from_idx(0), wire_target.2),
            (TileCellId::from_idx(0), backend.egrid.db.get_wire(nwt)),
        )
    } else if wtn.starts_with("LONG") {
        if wtn.starts_with("LONG.H") {
            if col == grid.col_w() {
                col += 1;
            } else if col == grid.col_e() {
                col -= 1;
            }
        } else {
            if row == grid.row_s() {
                row += 1;
            } else if row == grid.row_n() {
                row -= 1;
            }
        }
        let idx = wtn[6..].parse::<usize>().unwrap() % 4;
        (
            (die, col, row, LayerId::from_idx(0)),
            (TileCellId::from_idx(0), wire_target.2),
            (
                TileCellId::from_idx(0),
                backend.egrid.db.get_wire(&format!("OUT.TBUF{idx}")),
            ),
        )
    } else if wtn.starts_with("CLB.M") || wtn.starts_with("IO.M") {
        let nloc = (die, col, row, LayerId::from_idx(0));
        let node = backend.egrid.tile(nloc);
        let node_kind = &backend.egrid.db.tile_classes[node.class];
        'a: {
            for &inp in &node_kind.muxes[&(TileCellId::from_idx(0), wire_target.2)].ins {
                if backend.egrid.db.wires.key(inp.1).starts_with("LONG")
                    || backend.egrid.db.wires.key(inp.1).starts_with("GLOBAL")
                {
                    break 'a (nloc, (TileCellId::from_idx(0), wire_target.2), inp);
                }
            }
            panic!("ummm no long?")
        }
    } else if wtn.starts_with("SINGLE") || wtn.starts_with("IO.SINGLE") || wtn.starts_with("DBL") {
        'a: {
            for w in backend.egrid.wire_tree(wire_target) {
                let nloc = (w.0, w.1.0, w.1.1, LayerId::from_idx(0));
                let node = backend.egrid.tile(nloc);
                let node_kind = &backend.egrid.db.tile_classes[node.class];
                if let Some(mux) = node_kind.muxes.get(&(TileCellId::from_idx(0), w.2)) {
                    for &inp in &mux.ins {
                        if backend.egrid.db.wires.key(inp.1).starts_with("CLB.M")
                            || backend.egrid.db.wires.key(inp.1).starts_with("IO.M")
                        {
                            let rwf = backend.egrid.resolve_tile_wire_nobuf(nloc, inp).unwrap();
                            if rwf != wire_avoid {
                                break 'a (nloc, (TileCellId::from_idx(0), w.2), inp);
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
    let nwt = backend.egrid.resolve_tile_wire_nobuf(ploc, pwf).unwrap();
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
    inv: bool,
) -> Fuzzer<XactBackend<'a>> {
    let (die, (col, mut row), w) = wire;
    let wn = &backend.egrid.db.wires.key(w)[..];
    if !wn.starts_with("IMUX") || wn == "IMUX.GIN" || wn == "IMUX.TS" {
        return fuzzer;
    }
    let (slot, pin) = match wn {
        "IMUX.LC0.F1" => (bels::LC0, "LC0.F1"),
        "IMUX.LC0.F2" => (bels::LC0, "LC0.F2"),
        "IMUX.LC0.F3" => (bels::LC0, "LC0.F3"),
        "IMUX.LC0.F4" => (bels::LC0, "LC0.F4"),
        "IMUX.LC0.DI" => (bels::LC0, "LC0.DI"),
        "IMUX.LC1.F1" => (bels::LC0, "LC1.F1"),
        "IMUX.LC1.F2" => (bels::LC0, "LC1.F2"),
        "IMUX.LC1.F3" => (bels::LC0, "LC1.F3"),
        "IMUX.LC1.F4" => (bels::LC0, "LC1.F4"),
        "IMUX.LC1.DI" => (bels::LC0, "LC1.DI"),
        "IMUX.LC2.F1" => (bels::LC0, "LC2.F1"),
        "IMUX.LC2.F2" => (bels::LC0, "LC2.F2"),
        "IMUX.LC2.F3" => (bels::LC0, "LC2.F3"),
        "IMUX.LC2.F4" => (bels::LC0, "LC2.F4"),
        "IMUX.LC2.DI" => (bels::LC0, "LC2.DI"),
        "IMUX.LC3.F1" => (bels::LC0, "LC3.F1"),
        "IMUX.LC3.F2" => (bels::LC0, "LC3.F2"),
        "IMUX.LC3.F3" => (bels::LC0, "LC3.F3"),
        "IMUX.LC3.F4" => (bels::LC0, "LC3.F4"),
        "IMUX.LC3.DI" => (bels::LC0, "LC3.DI"),
        "IMUX.CLB.RST" => (bels::LC0, "CLR"),
        "IMUX.CLB.CE" => (bels::LC0, "CE"),
        "IMUX.CLB.CLK" => (bels::LC0, "CK"),
        "IMUX.IO0.T" => (bels::IO0, "T"),
        "IMUX.IO1.T" => (bels::IO1, "T"),
        "IMUX.IO2.T" => (bels::IO2, "T"),
        "IMUX.IO3.T" => (bels::IO3, "T"),
        "IMUX.IO0.O" => (bels::IO0, "O"),
        "IMUX.IO1.O" => (bels::IO1, "O"),
        "IMUX.IO2.O" => (bels::IO2, "O"),
        "IMUX.IO3.O" => (bels::IO3, "O"),
        "IMUX.RDBK.RCLK" => (bels::RDBK, "CK"),
        "IMUX.RDBK.TRIG" => (bels::RDBK, "TRIG"),
        "IMUX.STARTUP.SCLK" => (bels::STARTUP, "CK"),
        "IMUX.STARTUP.GRST" => (bels::STARTUP, "GCLR"),
        "IMUX.STARTUP.GTS" => (bels::STARTUP, "GTS"),
        "IMUX.BSCAN.TDO1" => (bels::BSCAN, "TDO1"),
        "IMUX.BSCAN.TDO2" => (bels::BSCAN, "TDO2"),
        "IMUX.OSC.OCLK" => (bels::OSC, "CK"),
        "IMUX.BUFG" => (bels::BUFG, "I"),
        "IMUX.BOT.CIN" => {
            row += 1;
            (bels::LC0, "CI")
        }
        _ => panic!("umm {wn}?"),
    };
    let nloc = (die, col, row, LayerId::from_idx(0));
    let nnode = &backend.ngrid.nodes[&nloc];
    let block = &nnode.bels[slot][0];
    if wn.starts_with("IMUX.IO") && pin == "O" {
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
    if slot == bels::STARTUP && pin == "CK" {
        fuzzer = fuzzer.base(Key::GlobalOpt("STARTUPCLK".into()), "CCLK");
    }
    if slot == bels::OSC && pin == "CK" {
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
    wire_to: TileClassWire,
    wire_from: TileClassWire,
    inv: bool,
}

impl IntPip {
    pub fn new(wire_to: TileClassWire, wire_from: TileClassWire, inv: bool) -> Self {
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
        nloc: prjcombine_interconnect::grid::NodeLoc,
        fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let rwt = backend
            .egrid
            .resolve_tile_wire_nobuf(nloc, self.wire_to)
            .unwrap();
        let rwf = backend
            .egrid
            .resolve_tile_wire_nobuf(nloc, self.wire_from)
            .unwrap();
        let (mut fuzzer, block, pin) = drive_wire(backend, fuzzer, rwf, rwt);
        fuzzer = fuzzer.fuzz(Key::NodeMutex(rwt), false, true);
        let crd = backend.ngrid.int_pip(nloc, self.wire_to, self.wire_from);
        fuzzer = fuzzer.fuzz(Key::Pip(crd), None, Value::FromPin(block, pin.into()));
        fuzzer = apply_imux_finish(backend, rwt, fuzzer, block, pin, self.inv);
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
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<XactBackend<'a>>,
    ) -> Option<(Fuzzer<XactBackend<'a>>, bool)> {
        let id = fuzzer.info.features.pop().unwrap().id;
        let (die, col, _, _) = nloc;
        for row in backend.egrid.die(die).rows() {
            if row == backend.edev.chip.row_s() || row == backend.edev.chip.row_n() {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                id: id.clone(),
                tiles: vec![BitTile::Null, backend.edev.btile_main(col, row)],
            });
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let intdb = backend.egrid.db;
    for (_, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.cells.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = if node.cells.len() == 1 {
                    wire_from_name.to_string()
                } else {
                    format!("{}.{}", wire_from.0, wire_from_name)
                };
                let mut f = ctx
                    .build()
                    .test_manual("INT", &mux_name, &in_name)
                    .prop(IntPip::new(wire_to, wire_from, false));
                if mux_name.contains("LONG.V2") && (tile == "CLKL" || tile == "CLKR") {
                    f = f.prop(AllColumnIo);
                }
                f.commit();
                if mux_name.starts_with("MUX.IMUX.IO")
                    && mux_name.ends_with("O")
                    && (tile == "IO.B" || tile == "IO.T")
                {
                    ctx.build()
                        .test_manual("INT", &mux_name, format!("{in_name}.INV"))
                        .prop(IntPip::new(wire_to, wire_from, true))
                        .commit();
                }
            }
        }
        if tile == "CLB" || tile.starts_with("IO") {
            for i in 0..4 {
                let mut bctx = ctx.bel(bels::TBUF[i]);
                bctx.build()
                    .pin_mutex_exclusive("T")
                    .test_manual("TMUX", "T")
                    .pip_pin("T", "T")
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = &ctx.edev.egrid;
    let intdb = egrid.db;
    for (_, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        for (&wire_to, mux) in &node.muxes {
            if intdb.wires[wire_to.1] != WireKind::MuxOut {
                let out_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                for &wire_from in &mux.ins {
                    let in_name = if node.cells.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let diff = ctx
                        .state
                        .get_diff(tile, "INT", format!("MUX.{out_name}"), &in_name);
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
            } else {
                let out_name = if node.cells.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                let mux_name = format!("MUX.{out_name}");
                if (tile == "IO.B" || tile == "IO.T")
                    && mux_name.starts_with("MUX.IMUX.IO")
                    && mux_name.ends_with("O")
                {
                    let mut inps = vec![];
                    let mut omux = vec![("INT", Diff::default())];
                    let mut got_empty = false;
                    let mut got_omux = false;
                    for &wire_from in &mux.ins {
                        let in_name = if node.cells.len() == 1 {
                            intdb.wires.key(wire_from.1).to_string()
                        } else {
                            format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                        };
                        let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                        let diff_i =
                            ctx.state
                                .get_diff(tile, "INT", &mux_name, format!("{in_name}.INV"));
                        if in_name.starts_with("OMUX") {
                            assert!(!got_omux);
                            got_omux = true;
                            omux.push(("OMUX", diff));
                            omux.push(("OMUX.INV", diff_i));
                        } else {
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            omux.push(("INT.INV", diff_i.combine(&!&diff)));
                            inps.push((in_name.to_string(), diff));
                        }
                    }
                    assert!(got_empty);
                    let item = xlat_enum_ocd(inps, OcdMode::Mux);
                    if item.bits.is_empty() {
                        println!("UMMM MUX {tile} {mux_name} is empty");
                    }
                    ctx.tiledb.insert(tile, "INT", mux_name, item);
                    let bel = match &out_name[..] {
                        "IMUX.IO0.O" => "IO0",
                        "IMUX.IO1.O" => "IO1",
                        "IMUX.IO2.O" => "IO2",
                        "IMUX.IO3.O" => "IO3",
                        _ => unreachable!(),
                    };
                    let item = xlat_enum_ocd(omux, OcdMode::Mux);
                    ctx.tiledb.insert(tile, bel, "OMUX", item);
                } else {
                    let mut inps = vec![];
                    let mut got_empty = false;
                    for &wire_from in &mux.ins {
                        let in_name = if node.cells.len() == 1 {
                            intdb.wires.key(wire_from.1).to_string()
                        } else {
                            format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                        };
                        let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                        if diff.bits.is_empty() {
                            got_empty = true;
                        }
                        inps.push((in_name.to_string(), diff));
                    }
                    for (rtile, rwire, rbel, rattr) in [
                        ("CNR.BR", "IMUX.STARTUP.GTS", "STARTUP", "ENABLE.GTS"),
                        ("CNR.BR", "IMUX.STARTUP.GRST", "STARTUP", "ENABLE.GR"),
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
        }
        if tile == "CLB" || tile.starts_with("IO") {
            for i in 0..4 {
                let bel = &format!("TBUF{i}");
                ctx.collect_enum_default(tile, bel, "TMUX", &["T"], "NONE");
            }
        }
    }
}
