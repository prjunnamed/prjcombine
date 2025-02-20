use std::collections::{btree_map, BTreeMap, HashSet};

use prjcombine_collector::{xlat_bit, xlat_enum, xlat_enum_ocd, Diff, FeatureId, OcdMode};
use prjcombine_hammer::Session;
use prjcombine_interconnect::db::{BelId, Dir, NodeTileId, NodeWireId};
use prjcombine_types::tiledb::TileBit;
use prjcombine_xc2000::grid::GridKind;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelFuzzKV, BelKV, TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileRelation, TileWire},
    fuzz::FuzzCtx,
    fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Xc2000(ref edev) = backend.edev else {
        unreachable!()
    };
    let kind = edev.grid.kind;
    let intdb = backend.egrid.db;
    for (node_kind, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        let bits = match &tile[..] {
            "CLB" | "CLB.B" | "CLB.T" | "CLB.L" | "CLB.LB" | "CLB.LT" | "CLB.R" | "CLB.RB"
            | "CLB.RT" | "IO.B" | "IO.BS" | "IO.BS.L" | "IO.B.R" | "IO.T" | "IO.TS" | "IO.TS.L"
            | "IO.T.R" | "IO.R" | "IO.RS" | "IO.RS.B" | "IO.R.T" | "IO.R.FB" | "IO.R.FT"
            | "IO.RS.FB" | "IO.RS.FT" | "IO.L" | "IO.LS" | "IO.LS.B" | "IO.L.T" | "IO.L.FB"
            | "IO.L.FT" | "IO.LS.FB" | "IO.LS.FT" | "CNR.BR" | "CNR.TR" | "CNR.BL" | "CNR.TL" => {
                TileBits::MainXc4000
            }
            "LLH.CLB" | "LLH.CLB.B" | "LLH.IO.B" | "LLH.IO.T" | "LLHC.CLB" | "LLHC.CLB.B"
            | "LLHC.IO.B" | "LLHC.IO.T" | "LLHQ.CLB" | "LLHQ.CLB.B" | "LLHQ.CLB.T"
            | "LLHQ.IO.B" | "LLHQ.IO.T" => TileBits::Llh,
            "LLV.CLB" | "LLV.IO.L" | "LLV.IO.R" | "LLVC.CLB" | "LLVC.IO.L" | "LLVC.IO.R"
            | "LLVQ.CLB" | "LLVQ.IO.L.B" | "LLVQ.IO.L.T" | "LLVQ.IO.R.B" | "LLVQ.IO.R.T" => {
                TileBits::Llv
            }
            _ => panic!("how to {tile}"),
        };
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt}.{out_name}", wtt = wire_to.0)
            } else {
                assert_eq!(wire_to.0.to_idx(), 0);
                format!("MUX.{out_name}")
            };
            if kind == GridKind::SpartanXl {
                if out_name == "IMUX.CLB.C2" && matches!(&tile[..], "CLB.T" | "CLB.LT" | "CLB.RT") {
                    continue;
                }
                if out_name == "IMUX.CLB.C3" && matches!(&tile[..], "CLB.L" | "CLB.LB" | "CLB.LT") {
                    continue;
                }
            }
            if out_name.starts_with("QBUF") || out_name.ends_with("EXCL") {
                let wire_mid = wire_to;
                for &wire_to in &mux.ins {
                    let wtname = format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1));
                    if wtname.contains("CLK") {
                        continue;
                    }
                    for &wire_from in &mux.ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        let wfname = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                        session.add_fuzzer(Box::new(TileFuzzerGen {
                            node: node_kind,
                            bits: bits.clone(),
                            feature: FeatureId {
                                tile: tile.to_string(),
                                bel: "INT".to_string(),
                                attr: format!("DMUX.{out_name}"),
                                val: format!("{wtname}.{wfname}"),
                            },
                            base: vec![
                                TileKV::IntMutexShared("MAIN".to_string()),
                                TileKV::Xc4000DoublePip(wire_from, wire_mid, wire_to),
                            ],
                            fuzz: vec![],
                            extras: vec![],
                        }));
                    }
                }
                continue;
            }
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.1);
                let in_name = format!("{}.{}", wire_from.0, wire_from_name);

                let mut is_bidi = false;
                if let Some(mux) = node.muxes.get(&wire_from) {
                    if mux.ins.contains(&wire_to) {
                        is_bidi = true;
                    }
                }
                let tbuf_i_wire = if wire_from_name == "LONG.H2" {
                    Some("IMUX.TBUF0.I")
                } else if wire_from_name == "LONG.H3" {
                    Some("IMUX.TBUF1.I")
                } else {
                    None
                };
                if let Some(tbuf_i_wire) = tbuf_i_wire {
                    let tbuf_i_wire = backend.egrid.db.get_wire(tbuf_i_wire);
                    if let Some(mux) = node.muxes.get(&(NodeTileId::from_idx(0), tbuf_i_wire)) {
                        if mux.ins.contains(&wire_to) {
                            is_bidi = true;
                        }
                    }
                }

                let mut is_bipass = false;
                let is_wt_sd = out_name.starts_with("SINGLE")
                    || out_name.starts_with("DOUBLE")
                    || out_name.starts_with("QUAD")
                    || out_name.starts_with("IO.DOUBLE");
                let is_wf_sd = wire_from_name.starts_with("SINGLE")
                    || wire_from_name.starts_with("DOUBLE")
                    || wire_from_name.starts_with("QUAD")
                    || wire_from_name.starts_with("IO.DOUBLE");
                if is_wt_sd && is_wf_sd {
                    is_bipass = true;
                }
                if out_name.starts_with("IO.OCTAL") && wire_from_name.starts_with("SINGLE") {
                    is_bipass = true;
                }
                if out_name.starts_with("SINGLE") && wire_from_name.starts_with("IO.OCTAL") {
                    is_bipass = true;
                }
                if out_name.starts_with("DEC") && wire_from_name.starts_with("DEC") {
                    is_bipass = true;
                }

                if wire_from_name.starts_with("QBUF") || wire_from_name.ends_with("EXCL") {
                    continue;
                }

                if is_bidi && !is_bipass {
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: tile.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.clone(),
                            val: in_name.to_string(),
                        },
                        base: vec![
                            TileKV::IntMutexShared("MAIN".to_string()),
                            TileKV::Xc4000BiPip(wire_from, wire_to),
                        ],
                        fuzz: vec![],
                        extras: vec![],
                    }));
                } else {
                    let mut base = vec![
                        TileKV::NodeIntDistinct(wire_to, wire_from),
                        TileKV::NodeIntDstFilter(wire_to),
                        TileKV::NodeIntSrcFilter(wire_from),
                        TileKV::IntMutexShared("MAIN".to_string()),
                    ];
                    let mut fuzz = vec![
                        TileFuzzKV::NodeMutexExclusive(wire_to),
                        TileFuzzKV::NodeMutexExclusive(wire_from),
                        TileFuzzKV::IntPip(wire_from, wire_to),
                    ];
                    if tile == "CNR.TR"
                        && (in_name.contains("OUT.LR.IOB1.I") || in_name.contains("OUT.OSC"))
                    {
                        // sigh.
                        let node_cnr = backend.egrid.db.get_node("CNR.TR");
                        let bel = backend.egrid.db.nodes[node_cnr].bels.get("OSC").unwrap().0;
                        base.push(TileKV::Bel(bel, BelKV::Mutex("MODE".into(), "INT".into())));
                        base.push(TileKV::Pip(
                            TileWire::BelPinNear(bel, "F15".into()),
                            TileWire::BelPinNear(bel, "OUT0".into()),
                        ));
                    }
                    if tile == "IO.R.T"
                        && (in_name.contains("OUT.LR.IOB1.I") && in_name.ends_with(".S"))
                    {
                        // sigh.
                        let node_cnr = backend.egrid.db.get_node("CNR.TR");
                        let bel = backend.egrid.db.nodes[node_cnr].bels.get("OSC").unwrap().0;
                        base.push(TileKV::TileRelated(
                            TileRelation::Delta(0, 1, node_cnr),
                            Box::new(TileKV::Bel(bel, BelKV::Mutex("MODE".into(), "INT".into()))),
                        ));
                        base.push(TileKV::TileRelated(
                            TileRelation::Delta(0, 1, node_cnr),
                            Box::new(TileKV::Pip(
                                TileWire::BelPinNear(bel, "F15".into()),
                                TileWire::BelPinNear(bel, "OUT0".into()),
                            )),
                        ));
                    }

                    if out_name == "IMUX.IOB0.TS" || out_name == "IMUX.IOB1.TS" {
                        let idx = if out_name == "IMUX.IOB0.TS" { 0 } else { 1 };
                        let bel = BelId::from_idx(idx);
                        base.extend([
                            TileKV::Bel(bel, BelKV::Mode("IOB".into())),
                            TileKV::Bel(bel, BelKV::Attr("TRI".into(), "T".into())),
                            TileKV::Bel(bel, BelKV::Pin("T".into(), true)),
                        ]);
                        if edev.grid.kind != GridKind::Xc4000E {
                            base.push(TileKV::Bel(bel, BelKV::Attr("OUTMUX".into(), "O".into())));
                        }
                    }

                    if out_name.starts_with("IMUX.TBUF") {
                        let idx = if out_name.starts_with("IMUX.TBUF0") {
                            0
                        } else {
                            1
                        };
                        let bel = if tile.starts_with("CLB") {
                            BelId::from_idx(1 + idx)
                        } else {
                            BelId::from_idx(2 + idx)
                        };
                        if out_name.ends_with("I") {
                            base.extend([TileKV::Bel(bel, BelKV::Mode("TBUF".into()))]);
                            fuzz.extend([TileFuzzKV::Bel(
                                bel,
                                BelFuzzKV::AttrDiff(
                                    "TBUFATTR".into(),
                                    "WANDT".into(),
                                    "WORAND".into(),
                                ),
                            )]);
                        } else {
                            fuzz.extend([
                                TileFuzzKV::Bel(bel, BelFuzzKV::Mode("TBUF".into())),
                                TileFuzzKV::Bel(
                                    bel,
                                    BelFuzzKV::Attr("TBUFATTR".into(), "WANDT".into()),
                                ),
                            ]);
                        }
                    }

                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: tile.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.clone(),
                            val: in_name.to_string(),
                        },
                        base,
                        fuzz,
                        extras: vec![],
                    }));
                }
            }
        }
        if tile.starts_with("CLB") {
            let bel = BelId::from_idx(0);
            session.add_fuzzer(Box::new(TileFuzzerGen {
                node: node_kind,
                bits: bits.clone(),
                feature: FeatureId {
                    tile: tile.to_string(),
                    bel: "INT".to_string(),
                    attr: "MUX.IMUX.CLB.F4".to_string(),
                    val: "CIN".to_string(),
                },
                base: vec![TileKV::Bel(bel, BelKV::Mode("CLB".into()))],
                fuzz: vec![
                    TileFuzzKV::Bel(bel, BelFuzzKV::Attr("F4MUX".into(), "CIN".into())),
                    TileFuzzKV::NodeMutexExclusive((
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.CLB.F4"),
                    )),
                ],
                extras: vec![],
            }));
        }
        if tile.starts_with("IO.R")
            || matches!(
                &tile[..],
                "CLB" | "CLB.B" | "CLB.T" | "CLB.R" | "CLB.RB" | "CLB.RT"
            )
        {
            let bel = BelId::from_idx(0);
            let tgt_node = backend.egrid.db.get_node(if tile == "CLB.R" {
                "CLB"
            } else if tile == "CLB.RB" {
                "CLB.B"
            } else if tile == "CLB.RT" {
                "CLB.T"
            } else if tile.starts_with("CLB") {
                tile
            } else if tile == "IO.R.T" {
                "CLB.RT"
            } else if tile == "IO.RS.B" {
                "CLB.RB"
            } else {
                "CLB.R"
            });
            session.add_fuzzer(Box::new(TileFuzzerGen {
                node: node_kind,
                bits: bits.clone(),
                feature: FeatureId {
                    tile: tile.to_string(),
                    bel: "INT".to_string(),
                    attr: "MUX.IMUX.CLB.G3".to_string(),
                    val: "CIN".to_string(),
                },
                base: vec![TileKV::TileRelated(
                    TileRelation::Delta(-1, 0, tgt_node),
                    Box::new(TileKV::Bel(bel, BelKV::Mode("CLB".into()))),
                )],
                fuzz: vec![
                    TileFuzzKV::TileRelated(
                        TileRelation::Delta(-1, 0, tgt_node),
                        Box::new(TileFuzzKV::Bel(
                            bel,
                            BelFuzzKV::Attr("G3MUX".into(), "CIN".into()),
                        )),
                    ),
                    TileFuzzKV::NodeMutexExclusive((
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.CLB.G3"),
                    )),
                ],
                extras: vec![],
            }));
        }
        if tile.starts_with("IO.B")
            || matches!(
                &tile[..],
                "CLB" | "CLB.B" | "CLB.L" | "CLB.LB" | "CLB.R" | "CLB.RB"
            )
        {
            let bel = BelId::from_idx(0);
            let tgt_node = backend
                .egrid
                .db
                .get_node(if tile == "CLB" || tile == "CLB.B" {
                    "CLB"
                } else if tile == "CLB.R" || tile == "CLB.RB" {
                    "CLB.R"
                } else if tile == "CLB.L" || tile == "CLB.LB" {
                    "CLB.L"
                } else if tile == "IO.BS.L" {
                    "CLB.LB"
                } else if tile == "IO.B.R" {
                    "CLB.RB"
                } else {
                    "CLB.B"
                });
            session.add_fuzzer(Box::new(TileFuzzerGen {
                node: node_kind,
                bits: bits.clone(),
                feature: FeatureId {
                    tile: tile.to_string(),
                    bel: "INT".to_string(),
                    attr: "MUX.IMUX.CLB.G2".to_string(),
                    val: "COUT0".to_string(),
                },
                base: vec![TileKV::TileRelated(
                    TileRelation::Delta(0, 1, tgt_node),
                    Box::new(TileKV::Bel(bel, BelKV::Mode("CLB".into()))),
                )],
                fuzz: vec![
                    TileFuzzKV::TileRelated(
                        TileRelation::Delta(0, 1, tgt_node),
                        Box::new(TileFuzzKV::Bel(
                            bel,
                            BelFuzzKV::Attr("G2MUX".into(), "COUT0".into()),
                        )),
                    ),
                    TileFuzzKV::NodeMutexExclusive((
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("IMUX.CLB.G2"),
                    )),
                ],
                extras: vec![],
            }));
        }
        if tile.starts_with("CLB") || tile.starts_with("IO.R") || tile.starts_with("IO.L") {
            for idx in 0..2 {
                let bel = if tile.starts_with("CLB") {
                    BelId::from_idx(1 + idx)
                } else {
                    BelId::from_idx(2 + idx)
                };
                session.add_fuzzer(Box::new(TileFuzzerGen {
                    node: node_kind,
                    bits: bits.clone(),
                    feature: FeatureId {
                        tile: tile.to_string(),
                        bel: "INT".to_string(),
                        attr: format!("MUX.IMUX.TBUF{idx}.TS"),
                        val: "GND".to_string(),
                    },
                    base: vec![],
                    fuzz: vec![
                        TileFuzzKV::Bel(bel, BelFuzzKV::Mode("TBUF".into())),
                        TileFuzzKV::Bel(bel, BelFuzzKV::Attr("TBUFATTR".into(), "WAND".into())),
                        TileFuzzKV::NodeMutexExclusive((
                            NodeTileId::from_idx(0),
                            backend.egrid.db.get_wire(&format!("IMUX.TBUF{idx}.TS")),
                        )),
                    ],
                    extras: vec![],
                }));
            }
            for bel in ["TBUF0", "TBUF1"] {
                let ctx = FuzzCtx::new(session, backend, tile, bel, bits.clone());
                if kind.is_clb_xl() && tile.starts_with("CLB") {
                    let wt = (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire(&format!("IMUX.{bel}.TS")),
                    );
                    let wf = (
                        NodeTileId::from_idx(0),
                        backend.egrid.db.get_wire("LONG.V0"),
                    );
                    fuzz_one!(ctx, "DRIVE1", "1", [
                        (mode "TBUF"),
                        (special TileKV::IntPip(wf, wt))
                    ], [
                        (attr_diff "TBUFATTR", "WORAND", "TBUF"),
                        (special TileFuzzKV::NodeMutexExclusive(wt)),
                        (special TileFuzzKV::NodeMutexExclusive(wf))
                    ]);
                } else {
                    fuzz_one!(ctx, "DRIVE1", "1", [
                        (mode "TBUF")
                    ], [
                        (attr_diff "TBUFATTR", "WORAND", "TBUF")
                    ]);
                }
            }
        }
        if tile.starts_with("IO") {
            for idx in 0..2 {
                let bel = BelId::from_idx(idx);
                session.add_fuzzer(Box::new(TileFuzzerGen {
                    node: node_kind,
                    bits: bits.clone(),
                    feature: FeatureId {
                        tile: tile.to_string(),
                        bel: "INT".to_string(),
                        attr: format!("MUX.IMUX.IOB{idx}.TS"),
                        val: "GND".to_string(),
                    },
                    base: vec![
                        TileKV::Bel(bel, BelKV::Mode("IOB".into())),
                        TileKV::Bel(bel, BelKV::Attr("OUTMUX".into(), "O".into())),
                    ],
                    fuzz: vec![TileFuzzKV::Bel(
                        bel,
                        BelFuzzKV::AttrDiff("TRI".into(), "T".into(), "".into()),
                    )],
                    extras: vec![],
                }));
            }
        }
        if tile.starts_with("LLV.") {
            let ctx = FuzzCtx::new(session, backend, tile, "CLKH", bits.clone());
            if edev.grid.kind == GridKind::SpartanXl {
                for opin in ["O0", "O1", "O2", "O3"] {
                    for ipin in [
                        "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H",
                        "I.UR.V",
                    ] {
                        fuzz_one!(ctx, format!("MUX.{opin}"), ipin, [
                            (mutex format!("MUX.{opin}"), ipin),
                            (mutex format!("OUT.{ipin}"), opin)
                        ], [
                            (pip (pin ipin), (pin opin))
                        ]);
                    }
                }
            } else {
                for (opin, ipin_p) in [
                    ("O0", "I.UL.V"),
                    ("O1", "I.LL.H"),
                    ("O2", "I.LR.V"),
                    ("O3", "I.UR.H"),
                ] {
                    for ipin in [ipin_p, "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"] {
                        fuzz_one!(ctx, format!("MUX.{opin}"), ipin, [
                            (mutex format!("MUX.{opin}"), ipin),
                            (mutex format!("OUT.{ipin}"), opin)
                        ], [
                            (pip (pin ipin), (pin opin))
                        ]);
                    }
                }
            }
        }
        if tile.starts_with("CNR") {
            if matches!(
                edev.grid.kind,
                GridKind::Xc4000Xla | GridKind::Xc4000Xv | GridKind::SpartanXl
            ) {
                for (rtile, opt, bel, out, inp) in [
                    (
                        "CNR.TL",
                        "GCLK1",
                        "BUFGLS.V",
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.N",
                    ),
                    (
                        "CNR.BL",
                        "GCLK2",
                        "BUFGLS.V",
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.S",
                    ),
                    (
                        "CNR.BL",
                        "GCLK3",
                        "BUFGLS.H",
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.W",
                    ),
                    (
                        "CNR.BR",
                        "GCLK4",
                        "BUFGLS.H",
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.E",
                    ),
                    (
                        "CNR.BR",
                        "GCLK5",
                        "BUFGLS.V",
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.S",
                    ),
                    (
                        "CNR.TR",
                        "GCLK6",
                        "BUFGLS.V",
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.N",
                    ),
                    (
                        "CNR.TR",
                        "GCLK7",
                        "BUFGLS.H",
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.E",
                    ),
                    (
                        "CNR.TL",
                        "GCLK8",
                        "BUFGLS.H",
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.W",
                    ),
                ] {
                    if rtile != tile {
                        continue;
                    }
                    let ctx = FuzzCtx::new(session, backend, tile, bel, bits.clone());
                    let wt = (NodeTileId::from_idx(0), backend.egrid.db.get_wire(out));
                    let wf = (NodeTileId::from_idx(0), backend.egrid.db.get_wire(inp));

                    fuzz_one!(ctx, "ALT_PAD", "1", [
                        (special TileKV::IntPip(wf, wt))
                    ], [
                        (global_opt opt, "ALTPAD"),
                        (special TileFuzzKV::NodeMutexExclusive(wt)),
                        (special TileFuzzKV::NodeMutexExclusive(wf))
                    ]);
                    fuzz_one!(ctx, "CLK_EN", "1", [
                        (special TileKV::IntPip(wf, wt))
                    ], [
                        (global_opt opt, "CLKEN"),
                        (special TileFuzzKV::NodeMutexExclusive(wt)),
                        (special TileFuzzKV::NodeMutexExclusive(wf))
                    ]);
                }
            }
            if edev.grid.kind != GridKind::SpartanXl {
                for hv in ['H', 'V'] {
                    for i in 0..4 {
                        let ctx = FuzzCtx::new(
                            session,
                            backend,
                            tile,
                            format!("PULLUP.DEC.{hv}{i}"),
                            bits.clone(),
                        );
                        fuzz_one!(ctx, "ENABLE", "1", [], [
                            (pip (pin "O"), (pin_far "O"))
                        ]);
                    }
                }
            }
        }
        if tile.starts_with("IO.L") || tile.starts_with("IO.R") {
            for i in 0..2 {
                let ctx = FuzzCtx::new(
                    session,
                    backend,
                    tile,
                    format!("PULLUP.TBUF{i}"),
                    bits.clone(),
                );
                fuzz_one!(ctx, "ENABLE", "1", [], [
                    (pip (pin "O"), (pin_far "O"))
                ]);
            }
        }
        if matches!(
            &tile[..],
            "LLHC.CLB" | "LLHC.CLB.B" | "LLHQ.CLB" | "LLHQ.CLB.B" | "LLHQ.CLB.T"
        ) {
            for lr in ['L', 'R'] {
                for i in 0..2 {
                    let ctx = FuzzCtx::new(
                        session,
                        backend,
                        tile,
                        format!("PULLUP.TBUF{i}.{lr}"),
                        bits.clone(),
                    );
                    fuzz_one!(ctx, "ENABLE", "1", [], [
                        (pip (pin "O"), (pin_far "O"))
                    ]);
                }
            }
        }
        if edev.grid.kind != GridKind::Xc4000E
            && matches!(
                &tile[..],
                "LLHC.CLB" | "LLHC.CLB.B" | "LLH.CLB" | "LLH.CLB.B"
            )
        {
            for bel in ["TBUF_SPLITTER0", "TBUF_SPLITTER1"] {
                let ctx = FuzzCtx::new(session, backend, tile, bel, bits.clone());
                for (val, dir, buf) in [
                    ("W", Dir::W, false),
                    ("E", Dir::E, false),
                    ("W.BUF", Dir::W, true),
                    ("E.BUF", Dir::E, true),
                ] {
                    fuzz_one!(ctx, "BUF", val, [
                        (bel_special BelKV::Xc4000TbufSplitter(dir, buf))
                    ], []);
                }
            }
        }
        if edev.grid.kind != GridKind::SpartanXl {
            if matches!(&tile[..], "LLVC.IO.L" | "LLVC.IO.R") {
                for bt in ['B', 'T'] {
                    for i in 0..4 {
                        let ctx = FuzzCtx::new(
                            session,
                            backend,
                            tile,
                            format!("PULLUP.DEC.{bt}{i}"),
                            bits.clone(),
                        );
                        fuzz_one!(ctx, "ENABLE", "1", [], [
                            (pip (pin "O"), (pin_far "O"))
                        ]);
                    }
                }
            }
            if matches!(&tile[..], "LLHC.IO.B" | "LLHC.IO.T") {
                for lr in ['L', 'R'] {
                    for i in 0..4 {
                        let ctx = FuzzCtx::new(
                            session,
                            backend,
                            tile,
                            format!("PULLUP.DEC.{lr}{i}"),
                            bits.clone(),
                        );
                        fuzz_one!(ctx, "ENABLE", "1", [], [
                            (pip (pin "O"), (pin_far "O"))
                        ]);
                    }
                }
            }
            if tile.starts_with("IO") {
                for i in 0..3 {
                    let ctx = FuzzCtx::new(session, backend, tile, format!("DEC{i}"), bits.clone());
                    for j in 1..=4 {
                        for val in ["I", "NOT"] {
                            fuzz_one!(ctx, format!("O{j}MUX"), val, [
                                (mode "DECODER"),
                                (pin format!("O{j}")),
                                (pin "I")
                            ], [
                                (attr format!("O{j}MUX"), val),
                                (pip (pin format!("O{j}")), (pin_far format!("O{j}")))
                            ]);
                        }
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(ref edev) = ctx.edev else {
        unreachable!()
    };
    let kind = edev.grid.kind;
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let mut mux_diffs: BTreeMap<NodeWireId, BTreeMap<NodeWireId, Diff>> = BTreeMap::new();
        let mut obuf_diffs: BTreeMap<NodeWireId, BTreeMap<NodeWireId, Diff>> = BTreeMap::new();
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.1);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt}.{out_name}", wtt = wire_to.0)
            } else {
                format!("MUX.{out_name}")
            };

            if out_name.starts_with("QBUF") {
                let wire_mid = wire_to;
                for &wire_to in &mux.ins {
                    let wtname = format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1));
                    let mut diffs = vec![];
                    for &wire_from in &mux.ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        let wfname = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                        let diff = ctx.state.get_diff(
                            tile,
                            "INT",
                            format!("DMUX.{out_name}"),
                            format!("{wtname}.{wfname}"),
                        );
                        diffs.push((wire_from, diff.clone()));
                    }
                    let mut odiff = diffs[0].1.clone();
                    for (_, diff) in &diffs {
                        odiff.bits.retain(|bit, _| diff.bits.contains_key(bit));
                    }
                    for (_, diff) in &mut diffs {
                        *diff = diff.combine(&!&odiff);
                    }
                    mux_diffs
                        .entry(wire_to)
                        .or_default()
                        .insert(wire_mid, odiff);
                    for (wire_from, diff) in diffs {
                        match mux_diffs.entry(wire_mid).or_default().entry(wire_from) {
                            btree_map::Entry::Vacant(entry) => {
                                entry.insert(diff);
                            }
                            btree_map::Entry::Occupied(entry) => {
                                assert_eq!(*entry.get(), diff);
                            }
                        }
                    }
                }
                continue;
            }
            if out_name.ends_with("EXCL") {
                for &wire_to in &mux.ins {
                    let wtname = format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1));
                    if wtname.contains("CLK") {
                        continue;
                    }
                    for &wire_from in &mux.ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        let wfname = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                        let diff = ctx.state.get_diff(
                            tile,
                            "INT",
                            format!("DMUX.{out_name}"),
                            format!("{wtname}.{wfname}"),
                        );
                        if diff.bits.is_empty() {
                            assert!(wfname.contains("CLK"));
                            continue;
                        }
                        mux_diffs
                            .entry(wire_to)
                            .or_default()
                            .insert(wire_from, diff);
                    }
                }
                continue;
            }
            if !out_name.starts_with("IMUX")
                && !out_name.starts_with("VCLK")
                && !out_name.starts_with("ECLK")
                && !out_name.starts_with("GCLK")
                && !out_name.starts_with("IO.DBUF")
            {
                for &wire_from in &mux.ins {
                    let wfname = intdb.wires.key(wire_from.1);
                    if wfname.starts_with("QBUF") || wfname.ends_with("EXCL") {
                        continue;
                    }
                    let in_name = format!("{}.{}", wire_from.0, wfname);
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if out_name.contains("OCTAL")
                        && wfname.contains("OCTAL")
                        && tile.starts_with("IO")
                        && edev.grid.kind == GridKind::Xc4000Xv
                    {
                        obuf_diffs
                            .entry(wire_to)
                            .or_default()
                            .insert(wire_from, diff);
                    } else {
                        if diff.bits.is_empty() {
                            if wfname == "GND" {
                                continue;
                            }
                            if wfname.starts_with("OUT.BUFGE") && out_name.starts_with("BUFGE") {
                                continue;
                            }
                            panic!("weird lack of bits: {tile} {out_name} {wfname}");
                        }
                        mux_diffs
                            .entry(wire_to)
                            .or_default()
                            .insert(wire_from, diff);
                    }
                }
                continue;
            }
            if kind == GridKind::SpartanXl {
                if out_name == "IMUX.CLB.C2" && matches!(&tile[..], "CLB.T" | "CLB.LT" | "CLB.RT") {
                    continue;
                }
                if out_name == "IMUX.CLB.C3" && matches!(&tile[..], "CLB.L" | "CLB.LB" | "CLB.LT") {
                    continue;
                }
            }
            let mut inps = vec![];
            let mut got_empty = false;
            for &wire_from in &mux.ins {
                let in_name = format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1));
                let mut diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                if edev.grid.kind == GridKind::Xc4000E
                    && tile.starts_with("IO.L")
                    && out_name == "IMUX.TBUF1.I"
                    && in_name == "0.DEC.V1"
                {
                    // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                    // found by diffing XC4000E with xact
                    assert!(!diff.bits.contains_key(&TileBit::new(0, 11, 1)));
                    diff.bits.insert(TileBit::new(0, 11, 1), false);
                }
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
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "GND");
                inps.push(("GND".to_string(), diff));
                // ... I fucking can't with this fpga; look, let's just... not think about it
                got_empty = true;
            }
            if out_name == "IMUX.TBUF0.TS" || out_name == "IMUX.TBUF1.TS" {
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "GND");
                inps.push(("GND".to_string(), diff));

                let bel = if out_name == "IMUX.TBUF0.TS" {
                    "TBUF0"
                } else {
                    "TBUF1"
                };
                let drive1 = ctx.extract_bit_wide(tile, bel, "DRIVE1", "1");
                if drive1.bits.len() == 2 {
                    for (_, diff) in &mut inps {
                        diff.apply_bitvec_diff_int(&drive1, 0, 3);
                    }
                } else {
                    assert_eq!(drive1.bits.len(), 1);
                    for (_, diff) in &mut inps {
                        diff.apply_bit_diff(&drive1, false, true);
                    }
                }
                ctx.tiledb.insert(tile, bel, "DRIVE1", drive1);

                inps.push(("VCC".to_string(), Diff::default()));
                assert!(!got_empty);
                got_empty = true;
            }
            if out_name == "IMUX.TBUF0.I"
                || out_name == "IMUX.TBUF1.I"
                || ((out_name == "IMUX.IOB0.O1" || out_name == "IMUX.IOB1.O1")
                    && tile.starts_with("IO"))
            {
                assert!(!got_empty);
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

            if edev.grid.kind == GridKind::Xc4000E {
                let iob_mux_off_d = if tile.starts_with("IO.R") && out_name == "IMUX.CLB.G1" {
                    Some(("IO.R", "IOB0"))
                } else if tile.starts_with("IO.R") && out_name == "IMUX.CLB.F1" {
                    Some(("IO.R", "IOB1"))
                } else if tile.starts_with("IO.B") && out_name == "IMUX.CLB.F4" {
                    Some(("IO.B", "IOB0"))
                } else if tile.starts_with("IO.B") && out_name == "IMUX.CLB.G4" {
                    Some(("IO.B", "IOB1"))
                } else if tile.starts_with("CLB.L") && out_name == "IMUX.CLB.G3" {
                    Some(("IO.L", "IOB0"))
                } else if tile.starts_with("CLB.L") && out_name == "IMUX.CLB.F3" {
                    Some(("IO.L", "IOB1"))
                } else if matches!(&tile[..], "CLB.LT" | "CLB.T" | "CLB.RT")
                    && out_name == "IMUX.CLB.F2"
                {
                    Some(("IO.T", "IOB0"))
                } else if matches!(&tile[..], "CLB.LT" | "CLB.T" | "CLB.RT")
                    && out_name == "IMUX.CLB.G2"
                {
                    Some(("IO.T", "IOB1"))
                } else {
                    None
                };
                if let Some((filter, bel)) = iob_mux_off_d {
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
                    if tile.starts_with("CLB") {
                        let (mut bit, val) = common.bits.into_iter().next().unwrap();
                        assert_ne!(bit.tile, 0);
                        bit.tile = 0;
                        let common = Diff {
                            bits: [(bit, val)].into_iter().collect(),
                        };
                        for iotile in intdb.nodes.keys() {
                            if iotile.starts_with(filter) {
                                ctx.tiledb.insert(
                                    iotile,
                                    bel,
                                    "MUX.OFF_D",
                                    xlat_enum(vec![("CE", Diff::default()), ("O", common.clone())]),
                                );
                            }
                        }
                    } else {
                        assert!(tile.starts_with(filter));
                        ctx.tiledb.insert(
                            tile,
                            bel,
                            "MUX.OFF_D",
                            xlat_enum(vec![("CE", Diff::default()), ("O", common)]),
                        );
                    }
                }
            }

            if !got_empty {
                inps.push(("NONE".to_string(), Diff::default()));
            }
            let item = xlat_enum_ocd(inps, OcdMode::Mux);
            if kind == GridKind::SpartanXl && out_name == "IMUX.BOT.COUT" {
                assert_eq!(mux.ins.len(), 1);
                assert!(item.bits.is_empty());
                continue;
            }
            if item.bits.is_empty() {
                println!("UMMM MUX {tile} {mux_name} is empty");
            }
            ctx.tiledb.insert(tile, "INT", mux_name, item);
        }

        for (wire_to, ins) in obuf_diffs {
            let out_name = edev.egrid.db.wires.key(wire_to.1);
            let mut odiff = ins.iter().next().unwrap().1.clone();
            for diff in ins.values() {
                odiff.bits.retain(|bit, _| diff.bits.contains_key(bit));
            }
            for (wire_from, diff) in ins {
                let wfname = edev.egrid.db.wires.key(wire_from.1);
                let in_name = format!("{}.{}", wire_from.0, wfname);
                let diff = diff.combine(&!&odiff);
                ctx.tiledb
                    .insert(tile, "INT", format!("BUF.OBUF.{in_name}"), xlat_bit(diff));
            }
            ctx.tiledb.insert(
                tile,
                "INT",
                format!("BUF.{out_name}.0.OBUF"),
                xlat_bit(odiff),
            );
        }

        let mut handled = HashSet::new();
        for (&wire_to, ins) in &mux_diffs {
            let wtname = edev.egrid.db.wires.key(wire_to.1);
            for (&wire_from, diff) in ins {
                if handled.contains(&(wire_to, wire_from)) {
                    continue;
                }
                let wfname = edev.egrid.db.wires.key(wire_from.1);
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
                    || wtname.starts_with("QUAD")
                    || wtname.starts_with("IO.DOUBLE")
                {
                    ctx.tiledb
                        .insert(tile, "INT", format!("PASS.{oname}.{iname}"), xlat_bit(diff));
                } else if wtname.starts_with("LONG")
                    || wtname.starts_with("OCTAL")
                    || wtname.starts_with("IO.OCTAL")
                {
                    ctx.tiledb
                        .insert(tile, "INT", format!("BUF.{oname}.{iname}"), xlat_bit(diff));
                } else {
                    println!("MEOW {tile} {oname} {iname}");
                }
            }
        }

        for (wire_to, ins) in mux_diffs {
            let out_name = edev.egrid.db.wires.key(wire_to.1);
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
                let wfname = edev.egrid.db.wires.key(wire_from.1);
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
        if tile.starts_with("IO.L") || tile.starts_with("IO.R") {
            for i in 0..2 {
                let bel = &format!("PULLUP.TBUF{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
            }
        }
        if edev.grid.kind != GridKind::Xc4000E
            && matches!(
                &tile[..],
                "LLHC.CLB" | "LLHC.CLB.B" | "LLH.CLB" | "LLH.CLB.B"
            )
        {
            for bel in ["TBUF_SPLITTER0", "TBUF_SPLITTER1"] {
                let item = ctx.extract_bit(tile, bel, "BUF", "W");
                ctx.tiledb.insert(tile, bel, "PASS", item);
                let item = ctx.extract_bit(tile, bel, "BUF", "E");
                ctx.tiledb.insert(tile, bel, "PASS", item);
                let item = ctx.extract_bit(tile, bel, "BUF", "W.BUF");
                ctx.tiledb.insert(tile, bel, "BUF_W", item);
                let item = ctx.extract_bit(tile, bel, "BUF", "E.BUF");
                ctx.tiledb.insert(tile, bel, "BUF_E", item);
            }
        }
        if matches!(
            &tile[..],
            "LLHC.CLB" | "LLHC.CLB.B" | "LLHQ.CLB" | "LLHQ.CLB.B" | "LLHQ.CLB.T"
        ) {
            for lr in ['L', 'R'] {
                for i in 0..2 {
                    let bel = &format!("PULLUP.TBUF{i}.{lr}");
                    ctx.collect_bit(tile, bel, "ENABLE", "1");
                }
            }
        }
        if tile.starts_with("LLV.") {
            let bel = "CLKH";
            if edev.grid.kind == GridKind::SpartanXl {
                for ipin in [
                    "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H", "I.UR.V",
                ] {
                    let (_, _, diff) = Diff::split(
                        ctx.state.peek_diff(tile, bel, "MUX.O0", ipin).clone(),
                        ctx.state.peek_diff(tile, bel, "MUX.O1", ipin).clone(),
                    );
                    ctx.tiledb
                        .insert(tile, bel, format!("ENABLE.{ipin}"), xlat_bit(diff));
                }
                for opin in ["O0", "O1", "O2", "O3"] {
                    let mut diffs = vec![("NONE", Diff::default())];
                    for ipin in [
                        "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H",
                        "I.UR.V",
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, format!("MUX.{opin}"), ipin);
                        diff.apply_bit_diff(
                            ctx.tiledb.item(tile, bel, &format!("ENABLE.{ipin}")),
                            true,
                            false,
                        );
                        diffs.push((ipin, diff));
                    }
                    ctx.tiledb
                        .insert(tile, bel, format!("MUX.{opin}"), xlat_enum(diffs));
                }
            } else {
                for (opin, ipin_p) in [
                    ("O0", "I.UL.V"),
                    ("O1", "I.LL.H"),
                    ("O2", "I.LR.V"),
                    ("O3", "I.UR.H"),
                ] {
                    ctx.collect_enum_default(
                        tile,
                        bel,
                        &format!("MUX.{opin}"),
                        &[ipin_p, "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"],
                        "NONE",
                    );
                }
            }
        }
        if tile.starts_with("CNR") {
            if matches!(
                edev.grid.kind,
                GridKind::Xc4000Xla | GridKind::Xc4000Xv | GridKind::SpartanXl
            ) {
                for hv in ['H', 'V'] {
                    for attr in ["CLK_EN", "ALT_PAD"] {
                        let item = ctx.extract_bit(tile, &format!("BUFGLS.{hv}"), attr, "1");
                        let bel = if edev.grid.kind == GridKind::SpartanXl {
                            format!("BUFGLS.{hv}")
                        } else {
                            format!("BUFG.{hv}")
                        };
                        ctx.tiledb.insert(tile, bel, attr, item);
                    }
                }
            }
            if edev.grid.kind != GridKind::SpartanXl {
                for hv in ['H', 'V'] {
                    for i in 0..4 {
                        let bel = &format!("PULLUP.DEC.{hv}{i}");
                        ctx.collect_bit(tile, bel, "ENABLE", "1");
                    }
                }
            }
        }
        if edev.grid.kind != GridKind::SpartanXl {
            if matches!(&tile[..], "LLVC.IO.L" | "LLVC.IO.R") {
                for bt in ['B', 'T'] {
                    for i in 0..4 {
                        let bel = &format!("PULLUP.DEC.{bt}{i}");
                        ctx.collect_bit(tile, bel, "ENABLE", "1");
                    }
                }
            }
            if matches!(&tile[..], "LLHC.IO.B" | "LLHC.IO.T") {
                for lr in ['L', 'R'] {
                    for i in 0..4 {
                        let bel = &format!("PULLUP.DEC.{lr}{i}");
                        ctx.collect_bit(tile, bel, "ENABLE", "1");
                    }
                }
            }
            if tile.starts_with("IO") {
                for i in 0..3 {
                    let bel = &format!("DEC{i}");
                    for j in 1..=4 {
                        let item = ctx.extract_bit(tile, bel, &format!("O{j}MUX"), "I");
                        ctx.tiledb.insert(tile, bel, format!("O{j}_P"), item);
                        let item = ctx.extract_bit(tile, bel, &format!("O{j}MUX"), "NOT");
                        ctx.tiledb.insert(tile, bel, format!("O{j}_N"), item);
                    }
                }
            }
        }
    }
}
