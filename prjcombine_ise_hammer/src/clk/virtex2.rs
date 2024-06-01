use prjcombine_hammer::Session;
use prjcombine_int::db::{BelId, PinDir};
use prjcombine_virtex2::grid::GridKind;
use prjcombine_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bitvec, xlat_enum, xlat_enum_default, CollectorCtx},
    fgen::{TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let (edev, grid_kind) = match backend.edev {
        ExpandedDevice::Virtex2(ref edev) => (edev, edev.grid.kind),
        _ => unreachable!(),
    };

    // CLK[BT]
    let (clkb, clkt) = match grid_kind {
        GridKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
        GridKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
        GridKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
        GridKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
        GridKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
        GridKind::Spartan3A => ("CLKB.S3A", "CLKT.S3A"),
        GridKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        let node_kind = backend.egrid.db.get_node(tile);
        for i in 0..bufg_num {
            let bel = BelId::from_idx(i);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::BTSpine,
                tile_name: tile,
                bel,
                bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
            };
            fuzz_one!(ctx, "PRESENT", "1", [(special TileKV::StabilizeGclkc)], [(mode "BUFGMUX")]);
            fuzz_enum!(ctx, "SINV", ["S", "S_B"], [(mode "BUFGMUX"), (pin "S"), (attr "DISABLE_ATTR", "LOW")]);
            fuzz_enum!(ctx, "DISABLE_ATTR", ["HIGH", "LOW"], [(mode "BUFGMUX"), (pin "S")]);
            let inps = if grid_kind.is_spartan3ea() {
                &["CKIL", "CKIR", "DCM_OUT_L", "DCM_OUT_R"][..]
            } else {
                &["CKI", "DCM_OUT_L", "DCM_OUT_R"]
            };
            for inp in inps {
                fuzz_one!(ctx, "CLKMUX", inp, [
                    (mutex "CLKMUX", inp)
                ], [(pip (pin inp), (pin "CLK"))]);
            }
            fuzz_one!(ctx, "CLKMUX", "INT", [
                (mutex "CLKMUX", "INT")
            ], [(pip (pin_far "CLK"), (pin "CLK"))]);
        }
        if grid_kind.is_virtex2() {
            for i in 0..2 {
                let bel = BelId::from_idx(8 + i);
                let ctx = FuzzCtx {
                    session,
                    node_kind,
                    bits: TileBits::Null,
                    tile_name: tile,
                    bel,
                    bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
                };
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
                for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                    fuzz_enum!(ctx, attr, ["0", "1"], [(mode "GLOBALSIG")]);
                }
            }
        } else {
            let bel = BelId::from_idx(4);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Null,
                tile_name: tile,
                bel,
                bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
            };
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
            fuzz_enum!(ctx, "ENABLE_GLOBALS", ["0", "1"], [(mode "GLOBALSIG")]);
        }
    }

    if grid_kind.is_spartan3ea() {
        // CLK[LR]
        let (clkl, clkr) = match grid_kind {
            GridKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
            GridKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
            GridKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
            _ => unreachable!(),
        };
        for tile in [clkl, clkr] {
            let node_kind = backend.egrid.db.get_node(tile);
            for i in 0..8 {
                let bel = BelId::from_idx(i);
                let ctx = FuzzCtx {
                    session,
                    node_kind,
                    bits: TileBits::ClkLR,
                    tile_name: tile,
                    bel,
                    bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
                };
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGMUX")]);
                fuzz_enum!(ctx, "SINV", ["S", "S_B"], [(mode "BUFGMUX"), (pin "S"), (attr "DISABLE_ATTR", "LOW")]);
                fuzz_enum!(ctx, "DISABLE_ATTR", ["HIGH", "LOW"], [(mode "BUFGMUX"), (pin "S")]);
                for inp in ["CKI", "DCM_OUT"] {
                    fuzz_one!(ctx, "CLKMUX", inp, [
                    (mutex "CLKMUX", inp)
                ], [(pip (pin inp), (pin "CLK"))]);
                }
                fuzz_one!(ctx, "CLKMUX", "INT", [
                    (mutex "CLKMUX", "INT")
                ], [(pip (pin_far "CLK"), (pin "CLK"))]);
            }
            let bel = BelId::from_idx(8);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::ClkLR,
                tile_name: tile,
                bel,
                bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
            };
            fuzz_one!(ctx, "PRESENT", "1", [
                (global_mutex_none "PCILOGICSE")
            ], [(mode "PCILOGICSE")]);
            if grid_kind.is_spartan3a() {
                for val in ["LOW", "MED", "HIGH", "NILL"] {
                    fuzz_one!(ctx, "DELAY", val, [
                        (global_mutex_site "PCILOGICSE"),
                        (mode "PCILOGICSE")
                    ], [
                        (global_opt "pci_ce_delay_left", val),
                        (global_opt "pci_ce_delay_right", val)
                    ]);
                }
            }

            let bel = BelId::from_idx(10);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Null,
                tile_name: tile,
                bel,
                bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
            };
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
            fuzz_enum!(ctx, "ENABLE_GLOBALS", ["0", "1"], [(mode "GLOBALSIG")]);
        }
    }

    if grid_kind.is_virtex2() {
        // CLKC
        let bel = BelId::from_idx(0);
        let node_kind = backend.egrid.db.get_node("CLKC");
        let tile = "CLKC";
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Null,
            tile_name: tile,
            bel,
            bel_name: "CLKC",
        };
        for i in 0..8 {
            for bt in ["B", "T"] {
                fuzz_one!(ctx, format!("FWD_{bt}{i}").leak(), "1", [], [
                    (pip (pin format!("IN_{bt}{i}").leak()), (pin format!("OUT_{bt}{i}").leak()))
                ]);
            }
        }

        // GCLKC
        for tile in ["GCLKC", "GCLKC.B", "GCLKC.T"] {
            let node_kind = backend.egrid.db.get_node(tile);
            let bel = BelId::from_idx(0);
            if backend.egrid.node_index[node_kind].is_empty() {
                continue;
            }
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Gclkc,
                tile_name: tile,
                bel,
                bel_name: "GCLKC",
            };
            for i in 0..8 {
                for lr in ["L", "R"] {
                    let out_name = &*format!("OUT_{lr}{i}").leak();
                    for bt in ["B", "T"] {
                        let inp_name = &*format!("IN_{bt}{i}").leak();
                        fuzz_one!(ctx, out_name, inp_name, [
                            (tile_mutex out_name, inp_name)
                        ], [
                            (pip (pin inp_name), (pin out_name))
                        ]);
                    }
                }
            }
        }
    } else if edev.grid.cols_clkv.is_none() {
        // CLKC_50A
        let bel = BelId::from_idx(0);
        let node_kind = backend.egrid.db.get_node("CLKC_50A");
        let tile = "CLKC_50A";
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Clkc,
            tile_name: tile,
            bel,
            bel_name: "CLKC_50A",
        };
        for (out_l, out_r, in_l, in_r, in_bt) in [
            ("OUT_L0", "OUT_R0", "IN_L0", "IN_R0", "IN_B0"),
            ("OUT_L1", "OUT_R1", "IN_L1", "IN_R1", "IN_B1"),
            ("OUT_L2", "OUT_R2", "IN_L2", "IN_R2", "IN_B2"),
            ("OUT_L3", "OUT_R3", "IN_L3", "IN_R3", "IN_B3"),
            ("OUT_L4", "OUT_R4", "IN_L4", "IN_R4", "IN_T0"),
            ("OUT_L5", "OUT_R5", "IN_L5", "IN_R5", "IN_T1"),
            ("OUT_L6", "OUT_R6", "IN_L6", "IN_R6", "IN_T2"),
            ("OUT_L7", "OUT_R7", "IN_L7", "IN_R7", "IN_T3"),
        ] {
            for (out, inp) in [(out_l, in_l), (out_l, in_bt), (out_r, in_r), (out_r, in_bt)] {
                fuzz_one!(ctx, out, inp, [
                    (tile_mutex out, inp)
                ], [
                    (pip (pin inp), (pin out))
                ]);
            }
        }
    } else {
        // CLKC
        let bel = BelId::from_idx(0);
        let node_kind = backend.egrid.db.get_node("CLKC");
        let tile = "CLKC";
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Null,
            tile_name: tile,
            bel,
            bel_name: "CLKC",
        };
        for (out, inp) in [
            ("OUT0", "IN_B0"),
            ("OUT1", "IN_B1"),
            ("OUT2", "IN_B2"),
            ("OUT3", "IN_B3"),
            ("OUT4", "IN_T0"),
            ("OUT5", "IN_T1"),
            ("OUT6", "IN_T2"),
            ("OUT7", "IN_T3"),
        ] {
            fuzz_one!(ctx, out, inp, [], [
                (pip (pin inp), (pin out))
            ]);
        }

        // GCLKVM
        if grid_kind.is_spartan3ea() {
            let tile = "GCLKVM.S3E";
            let node_kind = backend.egrid.db.get_node(tile);
            let bel = BelId::from_idx(0);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Gclkvm,
                tile_name: tile,
                bel,
                bel_name: "GCLKVM",
            };
            for i in 0..8 {
                for bt in ["B", "T"] {
                    let out_name = &*format!("OUT_{bt}{i}").leak();
                    for lr in ["LR", "CORE"] {
                        let inp_name = &*format!("IN_{lr}{i}").leak();
                        fuzz_one!(ctx, out_name, inp_name, [
                            (tile_mutex out_name, inp_name)
                        ], [
                            (pip (pin inp_name), (pin out_name))
                        ]);
                    }
                }
            }
        } else {
            let tile = "GCLKVM.S3";
            let node_kind = backend.egrid.db.get_node(tile);
            let bel = BelId::from_idx(0);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Gclkvm,
                tile_name: tile,
                bel,
                bel_name: "GCLKVM",
            };
            for i in 0..8 {
                for bt in ["B", "T"] {
                    let out_name = &*format!("OUT_{bt}{i}").leak();
                    let inp_name = &*format!("IN_CORE{i}").leak();
                    fuzz_one!(ctx, out_name, inp_name, [], [
                        (pip (pin inp_name), (pin out_name))
                    ]);
                }
            }
        }

        // GCLKVC
        let tile = "GCLKVC";
        let node_kind = backend.egrid.db.get_node(tile);
        let bel = BelId::from_idx(0);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Null,
            tile_name: tile,
            bel,
            bel_name: "GCLKVC",
        };
        for i in 0..8 {
            let inp_name = &*format!("IN{i}").leak();
            for lr in ["L", "R"] {
                let out_name = &*format!("OUT_{lr}{i}").leak();
                fuzz_one!(ctx, out_name, inp_name, [
                    (tile_mutex out_name, inp_name)
                ], [
                    (pip (pin inp_name), (pin out_name))
                ]);
            }
        }
    }

    // GCLKH
    for tile in [
        "GCLKH",
        "GCLKH.S",
        "GCLKH.N",
        "GCLKH.UNI",
        "GCLKH.UNI.S",
        "GCLKH.UNI.N",
        "GCLKH.0",
        "GCLKH.DSP",
    ] {
        if tile != "GCLKH" && grid_kind.is_virtex2() {
            continue;
        }
        let node_kind = backend.egrid.db.get_node(tile);
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        if tile != "GCLKH.0" && tile != "GCLKH.DSP" {
            let bel = BelId::from_idx(1);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Hclk,
                tile_name: tile,
                bel,
                bel_name: "GCLKH",
            };
            for i in 0..8 {
                let inp_name = &*format!("IN{i}").leak();
                for bt in ["B", "T"] {
                    if bt == "T" && tile.ends_with(".S") {
                        continue;
                    }
                    if bt == "B" && tile.ends_with(".N") {
                        continue;
                    }
                    let out_name = &*format!("OUT_{bt}{i}").leak();
                    fuzz_one!(ctx, out_name, inp_name, [
                        (tile_mutex inp_name, out_name)
                    ], [
                        (pip (pin inp_name), (pin out_name))
                    ]);
                }
            }
        }
        let bel = BelId::from_idx(0);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Null,
            tile_name: tile,
            bel,
            bel_name: backend.egrid.db.nodes[node_kind].bels.key(bel),
        };
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
        if grid_kind.is_virtex2() {
            for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                fuzz_enum!(ctx, attr, ["0", "1"], [(mode "GLOBALSIG")]);
            }
        } else {
            fuzz_enum!(ctx, "ENABLE_GLOBALS", ["0", "1"], [(mode "GLOBALSIG")]);
        }
    }

    if !grid_kind.is_spartan3ea() {
        // DCMCONN
        for tile in ["DCMCONN.BOT", "DCMCONN.TOP"] {
            let node_kind = backend.egrid.db.get_node(tile);
            let bel = BelId::from_idx(0);
            let mut ctx = FuzzCtx {
                session,
                node_kind,
                bits: if grid_kind.is_virtex2() {
                    TileBits::BTTerm
                } else {
                    TileBits::Null
                },
                tile_name: tile,
                bel,
                bel_name: "DCMCONN",
            };
            let num_bus = if grid_kind.is_virtex2() { 8 } else { 4 };
            for i in 0..num_bus {
                let out_name = &*format!("OUTBUS{i}").leak();
                let in_name = &*format!("OUT{ii}", ii = i % 4).leak();
                fuzz_one!(ctx, out_name, "1", [], [
                    (row_mutex "DCMCONN"),
                    (pip (pin in_name), (pin out_name))
                ]);
            }
            ctx.bits = TileBits::Null;
            for i in 0..num_bus {
                let out_name = &*format!("CLKPAD{i}").leak();
                let in_name = &*format!("CLKPADBUS{i}").leak();
                fuzz_one!(ctx, out_name, "1", [], [
                    (pip (pin in_name), (pin out_name))
                ]);
            }
        }
    } else {
        // PCI_CE_*
        for tile in ["PCI_CE_S", "PCI_CE_N", "PCI_CE_W", "PCI_CE_E", "PCI_CE_CNR"] {
            let node_kind = backend.egrid.db.get_node(tile);
            if backend.egrid.node_index[node_kind].is_empty() {
                continue;
            }
            let bel = BelId::from_idx(0);
            let ctx = FuzzCtx {
                session,
                node_kind,
                bits: TileBits::Null,
                tile_name: tile,
                bel,
                bel_name: "PCI_CE",
            };
            fuzz_one!(ctx, "O", "1", [], [
                (row_mutex "DCMCONN"),
                (pip (pin "I"), (pin "O"))
            ]);
        }
    }

    if !grid_kind.is_virtex2() {
        // PTE2OMUX
        for tile in ["INT.DCM", "INT.DCM.S3E.DUMMY"] {
            let node_kind = backend.egrid.db.get_node(tile);
            if backend.egrid.node_index[node_kind].is_empty() {
                continue;
            }
            for i in 1..5 {
                let bel = BelId::from_idx(i);
                let bel_data = &backend.egrid.db.nodes[node_kind].bels[bel];
                let ctx = FuzzCtx {
                    session,
                    node_kind,
                    bits: TileBits::Main(1),
                    tile_name: tile,
                    bel,
                    bel_name: "PTE2OMUX",
                };
                let mux_name = backend.egrid.db.nodes[node_kind].bels.key(bel);
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    fuzz_one!(ctx, mux_name, pin_name, [], [
                        (row_mutex "PTE2OMUX"),
                        (pip (pin pin_name), (pin "OUT"))
                    ]);
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let (edev, grid_kind) = match ctx.edev {
        ExpandedDevice::Virtex2(ref edev) => (edev, edev.grid.kind),
        _ => unreachable!(),
    };
    let intdb = ctx.edev.egrid().db;

    // CLK[BT]
    let (clkb, clkt) = match grid_kind {
        GridKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
        GridKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
        GridKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
        GridKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
        GridKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
        GridKind::Spartan3A => ("CLKB.S3A", "CLKT.S3A"),
        GridKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        for i in 0..bufg_num {
            let node_kind = intdb.get_node(tile);
            let bel = &intdb.nodes[node_kind].bels[BelId::from_idx(i)];
            let pin = &bel.pins["S"];
            let bel = format!("BUFGMUX{i}").leak();
            ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
            assert_eq!(pin.wires.len(), 1);
            let wire = pin.wires.first().unwrap();
            let sinv = ctx.extract_enum_bool(tile, bel, "SINV", "S", "S_B");
            ctx.tiledb.insert(
                tile,
                "INT",
                format!("INV.{}.{}", wire.0, intdb.wires.key(wire.1)),
                sinv,
            );
            ctx.collect_enum(tile, bel, "DISABLE_ATTR", &["HIGH", "LOW"]);
            let inps = if grid_kind.is_spartan3ea() {
                &["INT", "CKIL", "CKIR", "DCM_OUT_L", "DCM_OUT_R"][..]
            } else {
                &["INT", "CKI", "DCM_OUT_L", "DCM_OUT_R"]
            };
            ctx.collect_enum(tile, bel, "CLKMUX", inps);
        }
    }

    if grid_kind.is_spartan3ea() {
        // CLK[LR]
        let (clkl, clkr) = match grid_kind {
            GridKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
            GridKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
            GridKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
            _ => unreachable!(),
        };
        for tile in [clkl, clkr] {
            for i in 0..8 {
                let bel = &*format!("BUFGMUX{i}").leak();
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_inv(tile, bel, "S");
                ctx.collect_enum(tile, bel, "DISABLE_ATTR", &["HIGH", "LOW"]);
                ctx.collect_enum(tile, bel, "CLKMUX", &["INT", "CKI", "DCM_OUT"]);
            }
            let bel = "PCILOGICSE";
            let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            if grid_kind.is_spartan3a() {
                let mut diffs = vec![];
                let mut default = None;
                for val in ["LOW", "MED", "HIGH", "NILL"] {
                    let diff = ctx.state.get_diff(tile, bel, "DELAY", val);
                    if diff.bits.is_empty() {
                        default = Some(val);
                    }
                    diffs.push((val.to_string(), diff));
                }
                let default = default.unwrap();
                ctx.tiledb.insert_device_data(
                    &ctx.device.name,
                    "PCILOGICSE:DELAY_DEFAULT",
                    default.to_string(),
                );
                let item = xlat_enum(diffs);
                present.discard_bits(&item);
                ctx.tiledb.insert(tile, bel, "DELAY", item);
            }
            ctx.tiledb
                .insert(tile, bel, "ENABLE", xlat_bitvec(vec![present]));
        }
    }

    if grid_kind.is_virtex2() {
        // GCLKC
        for tile in ["GCLKC", "GCLKC.B", "GCLKC.T"] {
            let node_kind = intdb.get_node(tile);
            let bel = "GCLKC";
            if ctx.edev.egrid().node_index[node_kind].is_empty() {
                continue;
            }
            for i in 0..8 {
                for lr in ["L", "R"] {
                    let out_name = &*format!("OUT_{lr}{i}").leak();
                    ctx.collect_enum(
                        tile,
                        bel,
                        out_name,
                        &[&*format!("IN_B{i}").leak(), &*format!("IN_T{i}").leak()],
                    );
                }
            }
        }
    } else if edev.grid.cols_clkv.is_none() {
        // CLKC_50A
        let tile = "CLKC_50A";
        let bel = "CLKC_50A";
        for (out_l, out_r, in_l, in_r, in_bt) in [
            ("OUT_L0", "OUT_R0", "IN_L0", "IN_R0", "IN_B0"),
            ("OUT_L1", "OUT_R1", "IN_L1", "IN_R1", "IN_B1"),
            ("OUT_L2", "OUT_R2", "IN_L2", "IN_R2", "IN_B2"),
            ("OUT_L3", "OUT_R3", "IN_L3", "IN_R3", "IN_B3"),
            ("OUT_L4", "OUT_R4", "IN_L4", "IN_R4", "IN_T0"),
            ("OUT_L5", "OUT_R5", "IN_L5", "IN_R5", "IN_T1"),
            ("OUT_L6", "OUT_R6", "IN_L6", "IN_R6", "IN_T2"),
            ("OUT_L7", "OUT_R7", "IN_L7", "IN_R7", "IN_T3"),
        ] {
            ctx.collect_enum(tile, bel, out_l, &[in_l, in_bt]);
            ctx.collect_enum(tile, bel, out_r, &[in_r, in_bt]);
        }
    } else if grid_kind.is_spartan3ea() {
        // GCLKVM
        let tile = "GCLKVM.S3E";
        let bel = "GCLKVM";
        for i in 0..8 {
            for bt in ["B", "T"] {
                let out_name = &*format!("OUT_{bt}{i}").leak();
                ctx.collect_enum_default(
                    tile,
                    bel,
                    out_name,
                    &[format!("IN_LR{i}").leak(), format!("IN_CORE{i}").leak()],
                    "NONE",
                );
            }
        }
    } else {
        // GCLKVM
        let tile = "GCLKVM.S3";
        let bel = "GCLKVM";
        for i in 0..8 {
            for bt in ["B", "T"] {
                let out_name = &*format!("OUT_{bt}{i}").leak();
                let in_name = &*format!("IN_CORE{i}").leak();
                let diff = ctx.state.get_diff(tile, bel, out_name, in_name);
                ctx.tiledb
                    .insert(tile, bel, out_name, xlat_bitvec(vec![diff]));
            }
        }
    }

    // GCLKH
    for tile in [
        "GCLKH",
        "GCLKH.S",
        "GCLKH.N",
        "GCLKH.UNI",
        "GCLKH.UNI.S",
        "GCLKH.UNI.N",
    ] {
        if tile != "GCLKH" && grid_kind.is_virtex2() {
            continue;
        }
        let node_kind = intdb.get_node(tile);
        if ctx.edev.egrid().node_index[node_kind].is_empty() {
            continue;
        }
        let bel = "GCLKH";
        for i in 0..8 {
            let inp_name = &*format!("IN{i}").leak();
            let uni_name = &*format!("OUT{i}").leak();
            for bt in ["B", "T"] {
                if bt == "T" && tile.ends_with(".S") {
                    continue;
                }
                if bt == "B" && tile.ends_with(".N") {
                    continue;
                }
                let out_name = &*format!("OUT_{bt}{i}").leak();
                let attr = if tile.starts_with("GCLKH.UNI") {
                    uni_name
                } else {
                    out_name
                };
                let diff = ctx.state.get_diff(tile, bel, out_name, inp_name);
                let item = xlat_bitvec(vec![diff]);
                ctx.tiledb.insert(tile, bel, attr, item);
            }
        }
    }

    // DCMCONN
    if grid_kind.is_virtex2() {
        for tile in ["DCMCONN.BOT", "DCMCONN.TOP"] {
            let bel = "DCMCONN";
            for attr in [
                "OUTBUS0", "OUTBUS1", "OUTBUS2", "OUTBUS3", "OUTBUS4", "OUTBUS5", "OUTBUS6",
                "OUTBUS7",
            ] {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    attr,
                    xlat_bitvec(vec![ctx.state.get_diff(tile, bel, attr, "1")]),
                );
            }
        }
    }

    if !grid_kind.is_virtex2() {
        // PTE2OMUX
        for tile in ["INT.DCM", "INT.DCM.S3E.DUMMY"] {
            let node_kind = intdb.get_node(tile);
            if ctx.edev.egrid().node_index[node_kind].is_empty() {
                continue;
            }
            let bel = "PTE2OMUX";
            for i in 1..5 {
                let bel_id = BelId::from_idx(i);
                let bel_data = &intdb.nodes[node_kind].bels[bel_id];
                let mux_name = intdb.nodes[node_kind].bels.key(bel_id);
                let mut diffs = vec![];
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    let mut diff = ctx.state.get_diff(tile, bel, mux_name, pin_name);
                    if matches!(&pin_name[..], "CLKFB" | "CLKIN" | "PSCLK") {
                        diff.discard_bits(&ctx.item_int_inv(&[tile], tile, mux_name, pin_name));
                    }
                    diffs.push((pin_name.to_string(), diff));
                }
                ctx.tiledb
                    .insert(tile, bel, mux_name, xlat_enum_default(diffs, "NONE"));
            }
        }
    }
}
