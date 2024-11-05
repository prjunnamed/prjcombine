use std::collections::HashSet;

use prjcombine_hammer::Session;
use prjcombine_int::{
    db::{BelId, NodeTileId, WireKind},
    grid::RowId,
};
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureId, IseBackend},
    diff::{xlat_bit, xlat_enum_ocd, CollectorCtx, Diff, OcdMode},
    fgen::{BelFuzzKV, BelKV, TileBits, TileFuzzKV, TileFuzzerGen, TileKV, TileRelation},
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for (node_kind, name, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bits = match &name[..] {
            "CLB" | "IO.L" | "IO.R" | "IO.B" | "IO.T" | "CNR.BL" | "CNR.BR" | "CNR.TL"
            | "CNR.TR" | "LBRAM" | "RBRAM" | "MBRAM" | "BRAM_BOT" | "BRAM_TOP" | "DLL.BOT"
            | "DLL.TOP" | "DLLP.BOT" | "DLLP.TOP" | "DLLS.BOT" | "DLLS.TOP" => TileBits::MainAuto,
            "CLKL" | "CLKR" => {
                // funnily enough it also works.
                TileBits::MainAuto
            }
            "CLKB" | "CLKT" | "CLKB_4DLL" | "CLKT_4DLL" | "CLKB_2DLL" | "CLKT_2DLL" => {
                TileBits::Spine(0, 1)
            }
            _ => unreachable!(),
        };
        for (&wire_to, mux) in &node.muxes {
            let mux_name = if node.tiles.len() == 1 {
                format!("MUX.{}", intdb.wires.key(wire_to.1))
            } else {
                format!("MUX.{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
            };
            let out_name = intdb.wires.key(wire_to.1);
            let mut base = vec![];
            let mut fuzz = vec![TileFuzzKV::NodeMutexExclusive(wire_to)];
            if out_name.contains("OMUX") {
                if name.starts_with("IO") {
                    for i in 0..4 {
                        base.extend([TileKV::Bel(
                            BelId::from_idx(i),
                            BelKV::Mode(["EMPTYIOB", "IOB", "IOB", "IOB"][i].into()),
                        )]);
                        base.extend([TileKV::Bel(
                            BelId::from_idx(i),
                            BelKV::Pin("I".into(), true),
                        )]);
                    }
                    let clb_id = intdb.get_node("CLB");
                    let clb = &intdb.nodes[clb_id];
                    let wire_name = intdb.wires.key(wire_to.1);
                    let clb_wire = if name == "IO.L" {
                        format!("{wire_name}.W")
                    } else {
                        format!("{wire_name}.E")
                    };
                    let clb_wire = (NodeTileId::from_idx(0), intdb.get_wire(&clb_wire));
                    let wire_pin = 'omux_pin: {
                        for (&wire, mux) in &clb.muxes {
                            if mux.ins.contains(&clb_wire) {
                                break 'omux_pin wire;
                            }
                        }
                        panic!("NO WAY TO PIN {name} {mux_name}");
                    };
                    let relation = if name == "IO.L" {
                        TileRelation::Delta(2, 0, clb_id)
                    } else {
                        TileRelation::Delta(-2, 0, clb_id)
                    };
                    base.extend([TileKV::TileRelated(
                        relation,
                        TileKV::IntPip(clb_wire, wire_pin).into(),
                    )]);
                    fuzz.extend([TileFuzzKV::TileRelated(
                        relation,
                        TileFuzzKV::NodeMutexExclusive(wire_pin).into(),
                    )]);
                } else {
                    let wire_pin = 'omux_pin: {
                        for (&wire, mux) in &node.muxes {
                            if mux.ins.contains(&wire_to) {
                                break 'omux_pin wire;
                            }
                        }
                        panic!("NO WAY TO PIN {name} {mux_name}");
                    };
                    base.extend([TileKV::IntPip(wire_to, wire_pin)]);
                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                }
                for &wire_from in &mux.ins {
                    let in_name = if node.tiles.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let mut fuzz = fuzz.clone();
                    fuzz.push(TileFuzzKV::IntPip(wire_from, wire_to));
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: name.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.to_string(),
                            val: in_name.to_string(),
                        },
                        base: base.clone(),
                        fuzz,
                        extras: vec![],
                    }));
                }
            } else if out_name.starts_with("BRAM.QUAD") {
                let (is_s, wire_to_root) = if let Some(root_name) = out_name.strip_suffix(".S") {
                    (true, (wire_to.0, intdb.get_wire(root_name)))
                } else {
                    (false, wire_to)
                };
                let wire_pin = 'quad_dst_pin: {
                    for (&wire_pin, mux) in &node.muxes {
                        let wire_pin_name = intdb.wires.key(wire_pin.1);
                        if mux.ins.contains(&wire_to_root)
                            && (wire_pin_name.starts_with("IMUX")
                                || wire_pin_name.starts_with("HEX"))
                        {
                            break 'quad_dst_pin wire_pin;
                        }
                    }
                    panic!("NO WAY TO PIN {name} {mux_name}");
                };
                if !is_s {
                    base.extend([TileKV::IntPip(wire_to, wire_pin)]);
                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                } else {
                    let related = TileRelation::Delta(0, 4, node_kind);
                    base.extend([TileKV::TileRelated(
                        related,
                        TileKV::IntPip(wire_to_root, wire_pin).into(),
                    )]);
                    fuzz.extend([TileFuzzKV::TileRelated(
                        related,
                        TileFuzzKV::NodeMutexExclusive(wire_pin).into(),
                    )]);
                }
                if !out_name.starts_with("BRAM.QUAD.DOUT") {
                    // pin every input
                    let mut pins = HashSet::new();
                    for &wire_from in &mux.ins {
                        let in_wire_name = intdb.wires.key(wire_from.1);
                        'quad_src_all_pin: {
                            if in_wire_name.starts_with("SINGLE") {
                                let wire_buf = format!("{in_wire_name}.BUF");
                                let wire_buf = (NodeTileId::from_idx(0), intdb.get_wire(&wire_buf));
                                let related = TileRelation::Delta(
                                    -1,
                                    wire_from.0.to_idx() as isize - 4,
                                    intdb.get_node(if name == "LBRAM" { "IO.L" } else { "CLB" }),
                                );
                                base.extend([TileKV::TileRelated(
                                    related,
                                    TileKV::IntPip(
                                        (NodeTileId::from_idx(0), wire_from.1),
                                        wire_buf,
                                    )
                                    .into(),
                                )]);
                                fuzz.extend([TileFuzzKV::TileRelated(
                                    related,
                                    TileFuzzKV::NodeMutexExclusive(wire_buf).into(),
                                )]);
                                fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                break 'quad_src_all_pin;
                            } else if in_wire_name.starts_with("HEX") {
                                for (&wire_pin, mux) in &node.muxes {
                                    if wire_pin != wire_to
                                        && !pins.contains(&wire_pin)
                                        && mux.ins.contains(&wire_from)
                                    {
                                        base.extend([TileKV::IntPip(wire_from, wire_pin)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                        pins.insert(wire_pin);
                                        break 'quad_src_all_pin;
                                    }
                                }
                            } else {
                                break 'quad_src_all_pin;
                            }
                            panic!("NO WAY TO PIN {name} {mux_name} {in_wire_name}");
                        }
                    }
                }
                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.tiles.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{}.{}", wire_from.0, in_wire_name)
                    };
                    let mut base = base.clone();
                    let mut fuzz = fuzz.clone();
                    if in_wire_name.starts_with("BRAM.QUAD") {
                        'quad_src_pin: {
                            let (is_s, wire_from_root) =
                                if let Some(root_name) = in_wire_name.strip_suffix(".S") {
                                    (true, (wire_from.0, intdb.get_wire(root_name)))
                                } else {
                                    (false, wire_from)
                                };

                            let from_mux = &node.muxes[&wire_from_root];
                            for &wire_pin in &from_mux.ins {
                                let wire_pin_name = intdb.wires.key(wire_pin.1);
                                if intdb.wires.key(wire_pin.1).starts_with("HEX")
                                    || wire_pin_name.starts_with("OUT")
                                {
                                    if !is_s {
                                        base.extend([TileKV::IntPip(wire_pin, wire_from)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                    } else {
                                        let related = TileRelation::Delta(0, 4, node_kind);
                                        base.extend([TileKV::TileRelated(
                                            related,
                                            TileKV::IntPip(wire_pin, wire_from_root).into(),
                                        )]);
                                        fuzz.extend([TileFuzzKV::TileRelated(
                                            related,
                                            TileFuzzKV::NodeMutexExclusive(wire_pin).into(),
                                        )]);
                                        fuzz.extend([TileFuzzKV::TileRelated(
                                            related,
                                            TileFuzzKV::NodeMutexExclusive(wire_from_root).into(),
                                        )]);
                                    }
                                    break 'quad_src_pin;
                                }
                            }
                            panic!("NO WAY TO PIN {name} {mux_name} {in_name}");
                        }
                    }
                    fuzz.push(TileFuzzKV::IntPip(wire_from, wire_to));
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: name.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.to_string(),
                            val: in_name.to_string(),
                        },
                        base: base.clone(),
                        fuzz,
                        extras: vec![],
                    }));
                }
            } else if out_name.starts_with("SINGLE") {
                let wire_buf = format!("{out_name}.BUF");
                let wire_buf = (NodeTileId::from_idx(0), intdb.get_wire(&wire_buf));
                if !name.contains("BRAM") {
                    base.extend([TileKV::IntPip(wire_to, wire_buf)]);
                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_buf)]);
                } else {
                    let related = TileRelation::Delta(
                        -1,
                        wire_to.0.to_idx() as isize - 4,
                        intdb.get_node(if name == "LBRAM" { "IO.L" } else { "CLB" }),
                    );
                    base.extend([TileKV::TileRelated(
                        related,
                        TileKV::IntPip((NodeTileId::from_idx(0), wire_to.1), wire_buf).into(),
                    )]);
                    fuzz.extend([TileFuzzKV::TileRelated(
                        related,
                        TileFuzzKV::NodeMutexExclusive(wire_buf).into(),
                    )]);
                }
                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.tiles.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{}.{}", wire_from.0, in_wire_name)
                    };

                    let mut base = base.clone();
                    let mut fuzz = fuzz.clone();
                    'single_pin: {
                        if in_wire_name.starts_with("SINGLE") {
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                let wire_pin_name = intdb.wires.key(wire_pin.1);
                                if intdb.wires.key(wire_pin.1).starts_with("HEX")
                                    || wire_pin_name.starts_with("OMUX")
                                    || wire_pin_name.starts_with("BRAM.QUAD.DOUT")
                                {
                                    base.extend([TileKV::IntPip(wire_pin, wire_from)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                    break 'single_pin;
                                }
                            }
                        } else {
                            for (&wire_pin, mux) in &node.muxes {
                                let wire_pin_name = intdb.wires.key(wire_pin.1);
                                if wire_pin != wire_to
                                    && mux.ins.contains(&wire_from)
                                    && wire_pin_name.starts_with("SINGLE")
                                {
                                    base.extend([TileKV::IntPip(wire_from, wire_pin)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                    break 'single_pin;
                                }
                            }
                        }
                        panic!("NO WAY TO PIN {name} {mux_name} {in_name}");
                    };

                    fuzz.push(TileFuzzKV::IntPip(wire_from, wire_to));
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: name.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.to_string(),
                            val: in_name.to_string(),
                        },
                        base: base.clone(),
                        fuzz,
                        extras: vec![],
                    }));
                }
            } else if out_name.starts_with("LH")
                || out_name.starts_with("LV")
                || out_name.starts_with("HEX")
            {
                if out_name.starts_with("LH") && matches!(&name[..], "IO.B" | "IO.T") {
                    let wire_buf = format!("{out_name}.FAKE");
                    let wire_buf = (NodeTileId::from_idx(0), intdb.get_wire(&wire_buf));
                    base.extend([TileKV::IntPip(wire_to, wire_buf)]);
                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_buf)]);
                } else if out_name.starts_with("LV") && matches!(&name[..], "BRAM_BOT" | "BRAM_TOP")
                {
                    base.extend([TileKV::VirtexPinBramLv(wire_to)]);
                } else if out_name.starts_with("LH") && name.ends_with("BRAM") {
                    base.extend([TileKV::VirtexPinLh(wire_to)]);
                } else if out_name.starts_with("LH") && name.starts_with("CLK") {
                    base.extend([TileKV::VirtexPinIoLh(wire_to)]);
                } else if out_name.starts_with("HEX.H")
                    || out_name.starts_with("HEX.E")
                    || out_name.starts_with("HEX.W")
                {
                    base.extend([TileKV::VirtexPinHexH(wire_to)]);
                } else if out_name.starts_with("HEX.V")
                    || out_name.starts_with("HEX.S")
                    || out_name.starts_with("HEX.N")
                {
                    base.extend([TileKV::VirtexPinHexV(wire_to)]);
                } else {
                    'll_pin: {
                        for (&wire_pin, mux) in &node.muxes {
                            let wire_pin_name = intdb.wires.key(wire_pin.1);
                            if mux.ins.contains(&wire_to)
                                && (wire_pin_name.starts_with("HEX")
                                    || wire_pin_name.starts_with("IMUX.BRAM"))
                            {
                                base.extend([TileKV::IntPip(wire_to, wire_pin)]);
                                fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                break 'll_pin;
                            }
                        }
                        println!("NO WAY TO PIN {name} {mux_name}");
                    }
                }

                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    'll_src_pin: {
                        if let Some(wire_unbuf) = in_wire_name.strip_suffix(".BUF") {
                            let wire_unbuf = (NodeTileId::from_idx(0), intdb.get_wire(wire_unbuf));
                            base.extend([TileKV::IntPip(wire_unbuf, wire_from)]);
                            fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_unbuf)]);
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OMUX")
                            || in_wire_name.starts_with("BRAM.QUAD.DOUT")
                        {
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                if intdb.wires.key(wire_pin.1).starts_with("OUT") {
                                    base.extend([TileKV::IntPip(wire_pin, wire_from)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                    break 'll_src_pin;
                                }
                            }
                        } else if in_wire_name.starts_with("HEX") {
                            if in_wire_name.starts_with("HEX.E")
                                || in_wire_name.starts_with("HEX.W")
                                || in_wire_name.starts_with("HEX.H")
                            {
                                base.extend([TileKV::VirtexDriveHexH(wire_from)]);
                            } else {
                                base.extend([TileKV::VirtexDriveHexV(wire_from)]);
                            }
                            break 'll_src_pin;
                        } else if let Some(wire_unbuf) = in_wire_name.strip_suffix(".FAKE") {
                            let wire_unbuf = (NodeTileId::from_idx(0), intdb.get_wire(wire_unbuf));
                            base.extend([TileKV::IntPip(wire_unbuf, wire_from)]);
                            fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_unbuf)]);
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH") && name.starts_with("CNR") {
                            // it's fine.
                            base.extend([TileKV::VirtexPinIoLh(wire_from)]);
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("LH") || in_wire_name.starts_with("LV") {
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                if intdb.wires.key(wire_pin.1).starts_with("OMUX")
                                    || intdb.wires.key(wire_pin.1).starts_with("OUT")
                                    || (intdb.wires.key(wire_pin.1).starts_with("HEX")
                                        && name.starts_with("CNR"))
                                {
                                    base.extend([TileKV::IntPip(wire_pin, wire_from)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                    break 'll_src_pin;
                                }
                            }
                        } else if in_wire_name.starts_with("SINGLE") {
                            let wire_buf = format!("{in_wire_name}.BUF");
                            let wire_buf = (NodeTileId::from_idx(0), intdb.get_wire(&wire_buf));
                            if name.ends_with("BRAM") {
                                let related = TileRelation::Delta(
                                    -1,
                                    wire_from.0.to_idx() as isize - 4,
                                    intdb.get_node(if name == "LBRAM" { "IO.L" } else { "CLB" }),
                                );
                                base.extend([TileKV::TileRelated(
                                    related,
                                    TileKV::IntPip(
                                        (NodeTileId::from_idx(0), wire_from.1),
                                        wire_buf,
                                    )
                                    .into(),
                                )]);
                                fuzz.extend([
                                    TileFuzzKV::TileRelated(
                                        related,
                                        TileFuzzKV::NodeMutexExclusive(wire_buf).into(),
                                    ),
                                    TileFuzzKV::NodeMutexExclusive(wire_from),
                                ]);
                            } else {
                                base.extend([TileKV::IntPip(wire_from, wire_buf)]);
                                fuzz.extend([
                                    TileFuzzKV::NodeMutexExclusive(wire_buf),
                                    TileFuzzKV::NodeMutexExclusive(wire_from),
                                ]);
                            }
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("OUT.IO") {
                            for i in 0..4 {
                                base.extend([
                                    TileKV::Bel(
                                        BelId::from_idx(i),
                                        BelKV::Mode(
                                            [
                                                "EMPTYIOB",
                                                "IOB",
                                                "IOB",
                                                if name == "IO.L" || name == "IO.R" {
                                                    "IOB"
                                                } else {
                                                    "EMPTYIOB"
                                                },
                                            ][i]
                                                .into(),
                                        ),
                                    ),
                                    TileKV::Bel(BelId::from_idx(i), BelKV::Pin("I".into(), true)),
                                    TileKV::Bel(BelId::from_idx(i), BelKV::Pin("IQ".into(), true)),
                                ]);
                            }
                            break 'll_src_pin;
                        } else if let Some(pin) = in_wire_name.strip_prefix("OUT.BSCAN.") {
                            base.extend([
                                TileKV::Bel(BelId::from_idx(1), BelKV::Mode("BSCAN".into())),
                                TileKV::Bel(BelId::from_idx(1), BelKV::Pin(pin.into(), true)),
                            ]);
                            break 'll_src_pin;
                        } else if in_wire_name.starts_with("CLK.OUT")
                            || in_wire_name.starts_with("DLL.OUT")
                            || in_wire_name == "PCI_CE"
                        {
                            // already ok
                            break 'll_src_pin;
                        }
                        panic!("NO WAY TO PIN {name} {mux_name} {in_wire_name}");
                    };
                }

                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.tiles.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{}.{}", wire_from.0, in_wire_name)
                    };

                    let mut fuzz = fuzz.clone();
                    fuzz.push(TileFuzzKV::IntPip(wire_from, wire_to));
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: name.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.to_string(),
                            val: in_name.to_string(),
                        },
                        base: base.clone(),
                        fuzz,
                        extras: vec![],
                    }));
                }
            } else if out_name.contains("IMUX") {
                if let Some(pin) = out_name.strip_prefix("IMUX.STARTUP.") {
                    base.extend([TileKV::Bel(
                        BelId::from_idx(0),
                        BelKV::Mode("STARTUP".into()),
                    )]);
                    base.extend([TileKV::Bel(
                        BelId::from_idx(0),
                        BelKV::Pin(pin.into(), true),
                    )]);
                }
                let mut alt_out_wire = None;
                if out_name.starts_with("DLL.IMUX") {
                    for i in 0..4 {
                        for ps in ["", "P", "S"] {
                            base.push(TileKV::GlobalOpt(format!("IDLL{i}{ps}FB2X"), "0".into()))
                        }
                    }
                    if out_name == "DLL.IMUX.CLKIN" {
                        alt_out_wire = Some((
                            NodeTileId::from_idx(0),
                            backend.egrid.db.get_wire("DLL.IMUX.CLKFB"),
                        ));
                    }
                    if out_name == "DLL.IMUX.CLKFB" {
                        alt_out_wire = Some((
                            NodeTileId::from_idx(0),
                            backend.egrid.db.get_wire("DLL.IMUX.CLKIN"),
                        ));
                    }
                }
                if let Some(alt_out) = alt_out_wire {
                    fuzz.push(TileFuzzKV::NodeMutexExclusive(alt_out));
                }
                if out_name.starts_with("CLK.IMUX.BUFGCE.CLK") {
                    fuzz.extend([if out_name.ends_with("1") {
                        TileFuzzKV::Bel(BelId::from_idx(3), BelFuzzKV::Mode("GCLK".into()))
                    } else {
                        TileFuzzKV::Bel(BelId::from_idx(2), BelFuzzKV::Mode("GCLK".into()))
                    }]);
                }
                if (out_name.starts_with("IMUX.TBUF") && out_name.ends_with("I"))
                    || out_name.starts_with("IMUX.BRAM.DI")
                {
                    for &wire_from in &mux.ins {
                        let in_wire_name = intdb.wires.key(wire_from.1);
                        'imux_pin: {
                            if let Some(wire_unbuf) = in_wire_name.strip_suffix(".BUF") {
                                let wire_unbuf =
                                    (NodeTileId::from_idx(0), intdb.get_wire(wire_unbuf));
                                base.extend([TileKV::IntPip(wire_unbuf, wire_from)]);
                                fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_unbuf)]);
                                break 'imux_pin;
                            } else if out_name.starts_with("IMUX.BRAM.DI") {
                                let from_mux = &node.muxes[&wire_from];
                                for &wire_pin in &from_mux.ins {
                                    if intdb.wires.key(wire_pin.1).starts_with("HEX") {
                                        base.extend([TileKV::IntPip(wire_pin, wire_from)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                        break 'imux_pin;
                                    }
                                }
                            } else {
                                for (&wire_pin, mux) in &node.muxes {
                                    if wire_pin != wire_to && mux.ins.contains(&wire_from) {
                                        if let Some(from_mux) = node.muxes.get(&wire_from) {
                                            if from_mux.ins.contains(&wire_pin) {
                                                continue;
                                            }
                                        }
                                        base.extend([TileKV::IntPip(wire_from, wire_pin)]);
                                        fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                        break 'imux_pin;
                                    }
                                }
                            }
                            panic!("NO WAY TO PIN {name} {mux_name} {in_wire_name}");
                        };
                    }
                }
                for &wire_from in &mux.ins {
                    let in_wire_name = intdb.wires.key(wire_from.1);
                    let in_name = if node.tiles.len() == 1 {
                        in_wire_name.to_string()
                    } else {
                        format!("{}.{}", wire_from.0, in_wire_name)
                    };

                    let mut base = base.clone();
                    let mut fuzz = fuzz.clone();
                    'imux_pin: {
                        if in_wire_name.starts_with("GCLK") || in_wire_name.ends_with("BUF") {
                            // no need to pin
                            break 'imux_pin;
                        } else if out_name.starts_with("IMUX.TBUF") && out_name.ends_with("I") {
                            // already pinned above
                            break 'imux_pin;
                        } else if out_name == "PCI.IMUX.I3" {
                            let wire_buf = format!("{in_wire_name}.BUF");
                            let wire_buf = (NodeTileId::from_idx(0), intdb.get_wire(&wire_buf));
                            let related = TileRelation::Delta(
                                0,
                                0,
                                intdb.get_node(if name == "CLKL" { "IO.L" } else { "IO.R" }),
                            );
                            base.extend([TileKV::TileRelated(
                                related,
                                TileKV::IntPip(wire_from, wire_buf).into(),
                            )]);
                            fuzz.extend([TileFuzzKV::TileRelated(
                                related,
                                TileFuzzKV::NodeMutexExclusive(wire_buf).into(),
                            )]);
                            break 'imux_pin;
                        } else if out_name.starts_with("DLL.IMUX") {
                            if in_wire_name.starts_with("HEX") {
                                base.extend([TileKV::VirtexDriveHexH(wire_from)]);
                            } else {
                                // don't bother pinning.
                            }
                            break 'imux_pin;
                        } else {
                            for (&wire_pin, mux) in &node.muxes {
                                if wire_pin != wire_to && mux.ins.contains(&wire_from) {
                                    if let Some(from_mux) = node.muxes.get(&wire_from) {
                                        if from_mux.ins.contains(&wire_pin) {
                                            continue;
                                        }
                                    }
                                    base.extend([TileKV::IntPip(wire_from, wire_pin)]);
                                    fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                    break 'imux_pin;
                                }
                            }
                            // try to drive it instead.
                            let from_mux = &node.muxes[&wire_from];
                            for &wire_pin in &from_mux.ins {
                                if let Some(pin_mux) = node.muxes.get(&wire_pin) {
                                    if pin_mux.ins.contains(&wire_from) {
                                        continue;
                                    }
                                }
                                base.extend([TileKV::IntPip(wire_pin, wire_from)]);
                                fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_from)]);
                                fuzz.extend([TileFuzzKV::NodeMutexExclusive(wire_pin)]);
                                break 'imux_pin;
                            }
                        }
                        panic!("NO WAY TO PIN {name} {mux_name} {in_name}");
                    };

                    fuzz.push(TileFuzzKV::IntPip(wire_from, wire_to));
                    if let Some(alt_out) = alt_out_wire {
                        if in_wire_name.starts_with("CLK.OUT") {
                            session.add_fuzzer(Box::new(TileFuzzerGen {
                                node: node_kind,
                                bits: bits.clone(),
                                feature: FeatureId {
                                    tile: name.to_string(),
                                    bel: "INT".to_string(),
                                    attr: mux_name.to_string(),
                                    val: format!("{in_name}.NOALT"),
                                },
                                base: base.clone(),
                                fuzz: fuzz.clone(),
                                extras: vec![],
                            }));
                            base.push(TileKV::IntPip(wire_from, alt_out));
                        }
                    }
                    session.add_fuzzer(Box::new(TileFuzzerGen {
                        node: node_kind,
                        bits: bits.clone(),
                        feature: FeatureId {
                            tile: name.to_string(),
                            bel: "INT".to_string(),
                            attr: mux_name.to_string(),
                            val: in_name.to_string(),
                        },
                        base,
                        fuzz,
                        extras: vec![],
                    }));
                }
            } else {
                panic!("UNHANDLED MUX: {name} {mux_name}");
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, tile, node) in &intdb.nodes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }

        for (&wire_to, mux) in &node.muxes {
            if matches!(
                intdb.wires[wire_to.1],
                WireKind::PipOut | WireKind::PipBranch(_)
            ) {
                let out_name = if node.tiles.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                for &wire_from in &mux.ins {
                    let in_name = if node.tiles.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let diff = ctx
                        .state
                        .get_diff(tile, "INT", format!("MUX.{out_name}"), &in_name);
                    if in_name.starts_with("OUT.IO0")
                        || matches!(&tile[..], "IO.B" | "IO.T") && in_name.starts_with("OUT.IO3")
                    {
                        diff.assert_empty();
                        continue;
                    }
                    if diff.bits.is_empty() {
                        println!("UMM {out_name} {in_name} PASS IS EMPTY");
                        continue;
                    }
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
                let out_name = if node.tiles.len() == 1 {
                    intdb.wires.key(wire_to.1).to_string()
                } else {
                    format!("{}.{}", wire_to.0, intdb.wires.key(wire_to.1))
                };
                let mux_name = format!("MUX.{out_name}");

                let mut inps = vec![];
                let mut got_empty = false;
                for &wire_from in &mux.ins {
                    let in_name = if node.tiles.len() == 1 {
                        intdb.wires.key(wire_from.1).to_string()
                    } else {
                        format!("{}.{}", wire_from.0, intdb.wires.key(wire_from.1))
                    };
                    let mut diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if mux_name.contains("DLL.IMUX") && in_name.contains("CLK.OUT") {
                        let noalt_diff =
                            ctx.state
                                .get_diff(tile, "INT", &mux_name, format!("{in_name}.NOALT"));
                        let (alt, noalt, common) = Diff::split(diff, noalt_diff);
                        if mux_name.contains("CLKIN") {
                            ctx.tiledb.insert(tile, "DLL", "CLKIN_PAD", xlat_bit(noalt));
                            ctx.tiledb.insert(tile, "DLL", "CLKFB_PAD", xlat_bit(!alt));
                        } else {
                            ctx.tiledb.insert(tile, "DLL", "CLKFB_PAD", xlat_bit(noalt));
                            ctx.tiledb.insert(tile, "DLL", "CLKIN_PAD", xlat_bit(!alt));
                        }
                        diff = common;
                    }
                    if in_name.starts_with("OUT.IO0")
                        || (in_name.starts_with("OUT.IO3") && matches!(&tile[..], "IO.B" | "IO.T"))
                    {
                        diff.assert_empty();
                    } else if (out_name.contains("BRAM.QUAD") && in_name.contains("BRAM.QUAD"))
                        || out_name.contains("BRAM.QUAD.DOUT")
                        || (out_name.contains("HEX.H") && in_name == "PCI_CE")
                        || (tile.starts_with("CNR") && out_name.contains("LV"))
                        || (tile.starts_with("BRAM_") && out_name.contains("LV"))
                    {
                        if diff.bits.is_empty() {
                            println!("UMM {out_name} {in_name} BUF IS EMPTY");
                            continue;
                        }
                        ctx.tiledb.insert(
                            tile,
                            "INT",
                            format!("BUF.{out_name}.{in_name}"),
                            xlat_bit(diff),
                        );
                    } else {
                        if diff.bits.is_empty() {
                            got_empty = true;
                        }
                        inps.push((in_name.to_string(), diff));
                    }
                }
                if inps.is_empty() {
                    continue;
                }
                if out_name.contains("BRAM.QUAD")
                    || out_name.contains("LV")
                    || out_name.contains("LH")
                    || out_name.contains("HEX.H")
                    || out_name.contains("HEX.V")
                {
                    let mut drive_bits: HashSet<_> = inps[0].1.bits.keys().copied().collect();
                    for (_, diff) in &inps {
                        drive_bits.retain(|bit| diff.bits.contains_key(bit));
                    }
                    if drive_bits.len() > 1 {
                        if tile.starts_with("CNR") {
                            // sigh. I give up. those are obtained from looking at left-hand
                            // corners with easier-to-disambiguate muxes, and correlating with
                            // bitstream geometry in right-hand corners. also confirmed by some
                            // manual bitgen tests.
                            drive_bits.retain(|bit| matches!(bit.frame % 6, 0 | 5));
                        } else {
                            let btile = match &tile[..] {
                                "IO.L" => edev.btile_main(edev.grid.col_lio(), RowId::from_idx(1)),
                                "IO.R" => edev.btile_main(edev.grid.col_rio(), RowId::from_idx(1)),
                                _ => panic!(
                                "CAN'T FIGURE OUT DRIVE {tile} {mux_name} {drive_bits:?} {inps:?}"
                            ),
                            };
                            drive_bits.retain(|bit| {
                                !ctx.empty_bs
                                    .get_bit(btile.xlat_pos_fwd((bit.frame, bit.bit)))
                            });
                        }
                    }
                    if drive_bits.len() != 1 {
                        panic!("FUCKY WACKY {tile} {out_name} {inps:?}");
                    }
                    let drive = Diff {
                        bits: drive_bits
                            .into_iter()
                            .map(|bit| (bit, inps[0].1.bits[&bit]))
                            .collect(),
                    };
                    for (_, diff) in &mut inps {
                        *diff = diff.combine(&!&drive);
                    }
                    if inps.iter().all(|(_, diff)| !diff.bits.is_empty()) {
                        inps.push(("NONE".to_string(), Diff::default()));
                    }
                    let item = xlat_enum_ocd(inps, OcdMode::Mux);
                    ctx.tiledb.insert(tile, "INT", mux_name, item);
                    ctx.tiledb
                        .insert(tile, "INT", format!("DRIVE.{out_name}"), xlat_bit(drive));
                } else {
                    if !got_empty {
                        inps.push(("NONE".to_string(), Diff::default()));
                    }
                    let item = xlat_enum_ocd(inps, OcdMode::Mux);
                    if item.bits.is_empty() {
                        if mux_name == "MUX.IMUX.IO0.T" {
                            // empty on Virtex E?
                            continue;
                        }
                        if mux_name.starts_with("MUX.HEX.S") && tile == "IO.T"
                            || mux_name.starts_with("MUX.HEX.N") && tile == "IO.B"
                            || mux_name.starts_with("MUX.HEX.E") && tile == "IO.L"
                            || mux_name.starts_with("MUX.HEX.W") && tile == "IO.R"
                        {
                            continue;
                        }
                        println!("UMMM MUX {tile} {mux_name} is empty");
                    }
                    ctx.tiledb.insert(tile, "INT", mux_name, item);
                }
            }
        }
    }
}
