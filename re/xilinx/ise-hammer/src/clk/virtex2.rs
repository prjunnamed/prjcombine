use bitvec::vec::BitVec;
use prjcombine_re_collector::{xlat_bit, xlat_enum, xlat_enum_default};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::{BelId, PinDir};
use prjcombine_types::tiledb::TileItemKind;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    let (edev, grid_kind) = match backend.edev {
        ExpandedDevice::Virtex2(edev) => (edev, edev.grid.kind),
        _ => unreachable!(),
    };

    if devdata_only {
        if grid_kind.is_spartan3a() {
            // CLK[LR]
            let (clkl, clkr) = match grid_kind {
                GridKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
                GridKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
                GridKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
                _ => unreachable!(),
            };
            for tile in [clkl, clkr] {
                let ctx = FuzzCtx::new(session, backend, tile, "PCILOGICSE", TileBits::ClkLR);
                fuzz_one!(ctx, "PRESENT", "1", [
                    (global_mutex_none "PCILOGICSE")
                ], [(mode "PCILOGICSE")]);
            }
        }
        return;
    }

    // CLK[BT]
    let (clkb, clkt) = match grid_kind {
        GridKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
        GridKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
        GridKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
        GridKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
        GridKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
        GridKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
        GridKind::Spartan3A => ("CLKB.S3A", "CLKT.S3A"),
        GridKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        for i in 0..bufg_num {
            if edev.grid.kind != GridKind::FpgaCore {
                let ctx = FuzzCtx::new(
                    session,
                    backend,
                    tile,
                    format!("BUFGMUX{i}"),
                    TileBits::SpineEnd,
                );
                fuzz_one!(ctx, "PRESENT", "1", [(special TileKV::StabilizeGclkc)], [(mode "BUFGMUX")]);
                fuzz_inv!(ctx, "S", [
                    (global_mutex "BUFG", "TEST"),
                    (mode "BUFGMUX"),
                    (attr "DISABLE_ATTR", "LOW")
                ]);
                fuzz_enum!(ctx, "DISABLE_ATTR", ["HIGH", "LOW"], [
                    (global_mutex "BUFG", "TEST"),
                    (mode "BUFGMUX"),
                    (pin "S")
                ]);
                let inps = if grid_kind.is_spartan3ea() {
                    &["CKIL", "CKIR", "DCM_OUT_L", "DCM_OUT_R"][..]
                } else {
                    &["CKI", "DCM_OUT_L", "DCM_OUT_R"]
                };
                for &inp in inps {
                    fuzz_one!(ctx, "MUX.CLK", inp, [
                        (mutex "MUX.CLK", inp)
                    ], [(pip (pin inp), (pin "CLK"))]);
                }
                fuzz_one!(ctx, "MUX.CLK", "INT", [
                    (mutex "MUX.CLK", "INT")
                ], [(pip (pin_far "CLK"), (pin "CLK"))]);
            } else {
                let ctx = FuzzCtx::new(
                    session,
                    backend,
                    tile,
                    format!("BUFG{i}"),
                    TileBits::SpineEnd,
                );
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFG")]);
                fuzz_one!(ctx, "MUX.CLK", "CKI", [
                    (mutex "MUX.CLK", "CKI")
                ], [(pip (pin "CKI"), (pin "CLK"))]);
                fuzz_one!(ctx, "MUX.CLK", "INT", [
                    (mutex "MUX.CLK", "INT")
                ], [(pip (pin_far "CLK"), (pin "CLK"))]);
            }
        }
        if grid_kind.is_virtex2() {
            let bels = if tile.starts_with("CLKB") {
                ["GLOBALSIG.B0", "GLOBALSIG.B1"]
            } else {
                ["GLOBALSIG.T0", "GLOBALSIG.T1"]
            };
            for bel in bels {
                let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::Null);
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
                for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                    fuzz_enum!(ctx, attr, ["0", "1"], [(mode "GLOBALSIG")]);
                }
            }
        } else {
            let bel = if tile.starts_with("CLKB") {
                "GLOBALSIG.B"
            } else {
                "GLOBALSIG.T"
            };
            let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::Null);
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
            for i in 0..8 {
                let ctx = FuzzCtx::new(
                    session,
                    backend,
                    tile,
                    format!("BUFGMUX{i}"),
                    TileBits::ClkLR,
                );
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGMUX")]);
                fuzz_inv!(ctx, "S", [(mode "BUFGMUX"), (attr "DISABLE_ATTR", "LOW")]);
                fuzz_enum!(ctx, "DISABLE_ATTR", ["HIGH", "LOW"], [(mode "BUFGMUX"), (pin "S")]);
                for inp in ["CKI", "DCM_OUT"] {
                    fuzz_one!(ctx, "MUX.CLK", inp, [
                    (mutex "MUX.CLK", inp)
                ], [(pip (pin inp), (pin "CLK"))]);
                }
                fuzz_one!(ctx, "MUX.CLK", "INT", [
                    (mutex "MUX.CLK", "INT")
                ], [(pip (pin_far "CLK"), (pin "CLK"))]);
            }
            let ctx = FuzzCtx::new(session, backend, tile, "PCILOGICSE", TileBits::ClkLR);
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

            let ctx = FuzzCtx::new(session, backend, tile, "GLOBALSIG.LR", TileBits::Null);
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
            fuzz_enum!(ctx, "ENABLE_GLOBALS", ["0", "1"], [(mode "GLOBALSIG")]);
        }
    }

    if grid_kind.is_virtex2() {
        // CLKC
        let ctx = FuzzCtx::new(session, backend, "CLKC", "CLKC", TileBits::Null);
        for i in 0..8 {
            for bt in ["B", "T"] {
                fuzz_one!(ctx, format!("FWD_{bt}{i}"), "1", [], [
                    (pip (pin format!("IN_{bt}{i}")), (pin format!("OUT_{bt}{i}")))
                ]);
            }
        }

        // GCLKC
        for tile in ["GCLKC", "GCLKC.B", "GCLKC.T"] {
            if let Some(ctx) = FuzzCtx::try_new(session, backend, tile, "GCLKC", TileBits::Gclkc) {
                for i in 0..8 {
                    for lr in ["L", "R"] {
                        let out_name = format!("OUT_{lr}{i}");
                        for bt in ["B", "T"] {
                            let inp_name = format!("IN_{bt}{i}");
                            fuzz_one!(
                                ctx,
                                format!("MUX.OUT_{lr}{i}"),
                                &inp_name,
                                [
                                    (global_mutex "BUFG", "USE"),
                                    (tile_mutex &out_name, &inp_name)
                                ],
                                [
                                    (pip(pin &inp_name), (pin &out_name))
                                ]
                            );
                        }
                    }
                }
            }
        }
    } else if edev.grid.cols_clkv.is_none() {
        // CLKC_50A
        let ctx = FuzzCtx::new(session, backend, "CLKC_50A", "CLKC_50A", TileBits::Clkc);
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
                fuzz_one!(ctx, format!("MUX.{out}"), inp, [
                    (tile_mutex out, inp)
                ], [
                    (pip (pin inp), (pin out))
                ]);
            }
        }
    } else {
        // CLKC
        let ctx = FuzzCtx::new(session, backend, "CLKC", "CLKC", TileBits::Null);
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
            let ctx = FuzzCtx::new(session, backend, "GCLKVM.S3E", "GCLKVM", TileBits::Gclkvm);
            for i in 0..8 {
                for bt in ["B", "T"] {
                    let out_name = format!("OUT_{bt}{i}");
                    for lr in ["LR", "CORE"] {
                        let inp_name = format!("IN_{lr}{i}");
                        fuzz_one!(
                            ctx,
                            format!("MUX.{out_name}"),
                            &inp_name,
                            [(tile_mutex & out_name, &inp_name)],
                            [(pip(pin & inp_name), (pin & out_name))]
                        );
                    }
                }
            }
        } else {
            let ctx = FuzzCtx::new(session, backend, "GCLKVM.S3", "GCLKVM", TileBits::Gclkvm);
            for i in 0..8 {
                for bt in ["B", "T"] {
                    let out_name = format!("OUT_{bt}{i}");
                    let inp_name = format!("IN_CORE{i}");
                    fuzz_one!(ctx, format!("BUF.{out_name}"), &inp_name, [
                        (global_mutex_none "MISR_CLOCK")
                    ], [
                        (pip(pin & inp_name), (pin & out_name))
                    ]);
                }
            }
        }

        // GCLKVC
        let ctx = FuzzCtx::new(session, backend, "GCLKVC", "GCLKVC", TileBits::Null);
        for i in 0..8 {
            let inp_name = format!("IN{i}");
            for lr in ["L", "R"] {
                let out_name = format!("OUT_{lr}{i}");
                fuzz_one!(
                    ctx,
                    &out_name,
                    &inp_name,
                    [(tile_mutex & out_name, &inp_name)],
                    [(pip(pin & inp_name), (pin & out_name))]
                );
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
        if tile != "GCLKH" && (grid_kind.is_virtex2() || grid_kind == GridKind::FpgaCore) {
            continue;
        }
        let node_kind = backend.egrid.db.get_node(tile);
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        if tile != "GCLKH.0" && tile != "GCLKH.DSP" {
            let ctx = FuzzCtx::new_force_bel(
                session,
                backend,
                tile,
                "GCLKH",
                TileBits::Hclk,
                BelId::from_idx(1),
            );
            for i in 0..8 {
                let inp_name = format!("IN{i}");
                for bt in ["B", "T"] {
                    if bt == "T" && tile.ends_with(".S") {
                        continue;
                    }
                    if bt == "B" && tile.ends_with(".N") {
                        continue;
                    }
                    let out_name = format!("OUT_{bt}{i}");
                    fuzz_one!(ctx, &out_name, &inp_name, [
                        (global_mutex_none "MISR_CLOCK"),
                        (tile_mutex &inp_name, &out_name)
                    ], [
                        (pip (pin &inp_name), (pin &out_name))
                    ]);
                }
            }
        }
        let bel_name = if tile == "GCLKH.DSP" {
            "GLOBALSIG.DSP"
        } else {
            "GLOBALSIG"
        };
        let ctx = FuzzCtx::new(session, backend, tile, bel_name, TileBits::Null);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "GLOBALSIG")]);
        if grid_kind.is_virtex2() {
            for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                fuzz_enum!(ctx, attr, ["0", "1"], [(mode "GLOBALSIG")]);
            }
        } else {
            fuzz_enum!(ctx, "ENABLE_GLOBALS", ["0", "1"], [(mode "GLOBALSIG")]);
        }
    }

    if !grid_kind.is_spartan3ea() && grid_kind != GridKind::FpgaCore {
        // DCMCONN
        for tile in ["DCMCONN.BOT", "DCMCONN.TOP"] {
            let mut ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                "DCMCONN",
                if grid_kind.is_virtex2() {
                    TileBits::BTTerm
                } else {
                    TileBits::Null
                },
            );
            let num_bus = if grid_kind.is_virtex2() { 8 } else { 4 };
            for i in 0..num_bus {
                let out_name = format!("OUTBUS{i}");
                let in_name = format!("OUT{ii}", ii = i % 4);
                fuzz_one!(ctx, format!("BUF.{out_name}"), "1", [], [
                    (row_mutex "DCMCONN"),
                    (pip (pin &in_name), (pin &out_name))
                ]);
            }
            ctx.bits = TileBits::Null;
            for i in 0..num_bus {
                let out_name = format!("CLKPAD{i}");
                let in_name = format!("CLKPADBUS{i}");
                fuzz_one!(
                    ctx,
                    &out_name,
                    "1",
                    [],
                    [(pip(pin & in_name), (pin & out_name))]
                );
            }
        }
    }
    if grid_kind.is_spartan3ea() {
        // PCI_CE_*
        for tile in ["PCI_CE_S", "PCI_CE_N", "PCI_CE_W", "PCI_CE_E", "PCI_CE_CNR"] {
            if let Some(ctx) = FuzzCtx::try_new(session, backend, tile, tile, TileBits::Null) {
                fuzz_one!(ctx, "O", "1", [], [
                    (row_mutex "DCMCONN"),
                    (pip (pin "I"), (pin "O"))
                ]);
            }
        }
    }

    if !grid_kind.is_virtex2() && grid_kind != GridKind::FpgaCore {
        // PTE2OMUX
        for tile in ["INT.DCM", "INT.DCM.S3E.DUMMY"] {
            let node_kind = backend.egrid.db.get_node(tile);
            if backend.egrid.node_index[node_kind].is_empty() {
                continue;
            }
            for i in 0..4 {
                let ctx = FuzzCtx::new_force_bel(
                    session,
                    backend,
                    tile,
                    "PTE2OMUX",
                    TileBits::MainAuto,
                    BelId::from_idx(i + 1),
                );
                let bel_data = &backend.egrid.db.nodes[ctx.node_kind].bels[ctx.bel];
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    fuzz_one!(ctx, format!("MUX.PTE2OMUX{i}"), pin_name, [
                        (global_mutex "PSCLK", "PTE2OMUX")
                    ], [
                        (row_mutex "PTE2OMUX"),
                        (pip (pin pin_name), (pin "OUT"))
                    ]);
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let (edev, grid_kind) = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => (edev, edev.grid.kind),
        _ => unreachable!(),
    };
    let intdb = ctx.edev.egrid().db;

    if devdata_only {
        if grid_kind.is_spartan3a() {
            // CLK[LR]
            let (clkl, clkr) = match grid_kind {
                GridKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
                GridKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
                GridKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
                _ => unreachable!(),
            };
            for tile in [clkl, clkr] {
                let bel = "PCILOGICSE";
                let default = ctx.state.get_diff(tile, bel, "PRESENT", "1");
                let item = ctx.tiledb.item(tile, bel, "DELAY");
                let val: BitVec = item
                    .bits
                    .iter()
                    .map(|bit| default.bits.contains_key(bit))
                    .collect();
                let TileItemKind::Enum { ref values } = item.kind else {
                    unreachable!()
                };
                for (k, v) in values {
                    if *v == val {
                        ctx.insert_device_data("PCILOGICSE:DELAY_DEFAULT", k.clone());
                        break;
                    }
                }
            }
        }
        return;
    }

    // CLK[BT]
    let (clkb, clkt) = match grid_kind {
        GridKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
        GridKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
        GridKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
        GridKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
        GridKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
        GridKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
        GridKind::Spartan3A => ("CLKB.S3A", "CLKT.S3A"),
        GridKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        for i in 0..bufg_num {
            if edev.grid.kind != GridKind::FpgaCore {
                let node_kind = intdb.get_node(tile);
                let bel = &intdb.nodes[node_kind].bels[BelId::from_idx(i)];
                let pin = &bel.pins["S"];
                let bel = format!("BUFGMUX{i}");
                let bel = &bel;
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
                ctx.collect_enum(tile, bel, "MUX.CLK", inps);
            } else {
                let bel = format!("BUFG{i}");
                let bel = &bel;
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_enum(tile, bel, "MUX.CLK", &["INT", "CKI"]);
            }
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
                let bel = format!("BUFGMUX{i}");
                ctx.state
                    .get_diff(tile, &bel, "PRESENT", "1")
                    .assert_empty();
                ctx.collect_inv(tile, &bel, "S");
                ctx.collect_enum(tile, &bel, "DISABLE_ATTR", &["HIGH", "LOW"]);
                ctx.collect_enum(tile, &bel, "MUX.CLK", &["INT", "CKI", "DCM_OUT"]);
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
                ctx.insert_device_data("PCILOGICSE:DELAY_DEFAULT", default.to_string());
                let item = xlat_enum(diffs);
                present.discard_bits(&item);
                ctx.tiledb.insert(tile, bel, "DELAY", item);
            }
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(present));
        }
    }

    if grid_kind.is_virtex2() {
        // GCLKC
        for tile in ["GCLKC", "GCLKC.B", "GCLKC.T"] {
            let bel = "GCLKC";
            if !ctx.has_tile(tile) {
                continue;
            }
            for i in 0..8 {
                for lr in ["L", "R"] {
                    let out_name = format!("MUX.OUT_{lr}{i}");
                    ctx.collect_enum(
                        tile,
                        bel,
                        &out_name,
                        &[&format!("IN_B{i}"), &format!("IN_T{i}")],
                    );
                }
            }
        }
    } else if edev.grid.cols_clkv.is_none() {
        // CLKC_50A
        let tile = "CLKC_50A";
        let bel = "CLKC_50A";
        for (out_l, out_r, in_l, in_r, in_bt) in [
            ("MUX.OUT_L0", "MUX.OUT_R0", "IN_L0", "IN_R0", "IN_B0"),
            ("MUX.OUT_L1", "MUX.OUT_R1", "IN_L1", "IN_R1", "IN_B1"),
            ("MUX.OUT_L2", "MUX.OUT_R2", "IN_L2", "IN_R2", "IN_B2"),
            ("MUX.OUT_L3", "MUX.OUT_R3", "IN_L3", "IN_R3", "IN_B3"),
            ("MUX.OUT_L4", "MUX.OUT_R4", "IN_L4", "IN_R4", "IN_T0"),
            ("MUX.OUT_L5", "MUX.OUT_R5", "IN_L5", "IN_R5", "IN_T1"),
            ("MUX.OUT_L6", "MUX.OUT_R6", "IN_L6", "IN_R6", "IN_T2"),
            ("MUX.OUT_L7", "MUX.OUT_R7", "IN_L7", "IN_R7", "IN_T3"),
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
                let out_name = format!("MUX.OUT_{bt}{i}");
                ctx.collect_enum_default(
                    tile,
                    bel,
                    &out_name,
                    &[&format!("IN_LR{i}"), &format!("IN_CORE{i}")],
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
                ctx.collect_bit(
                    tile,
                    bel,
                    &format!("BUF.OUT_{bt}{i}"),
                    &format!("IN_CORE{i}"),
                );
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
        if tile != "GCLKH" && (grid_kind.is_virtex2() || grid_kind == GridKind::FpgaCore) {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "GCLKH";
        for i in 0..8 {
            let inp_name = format!("IN{i}");
            let uni_name = format!("OUT{i}");
            for bt in ["B", "T"] {
                if bt == "T" && tile.ends_with(".S") {
                    continue;
                }
                if bt == "B" && tile.ends_with(".N") {
                    continue;
                }
                let out_name = format!("OUT_{bt}{i}");
                let attr = if tile.starts_with("GCLKH.UNI") {
                    format!("BUF.{uni_name}")
                } else {
                    format!("BUF.{out_name}")
                };
                let item = ctx.extract_bit(tile, bel, &out_name, &inp_name);
                ctx.tiledb.insert(tile, bel, attr, item);
            }
        }
    }

    // DCMCONN
    if grid_kind.is_virtex2() {
        for tile in ["DCMCONN.BOT", "DCMCONN.TOP"] {
            let bel = "DCMCONN";
            for i in 0..8 {
                ctx.collect_bit(tile, bel, &format!("BUF.OUTBUS{i}"), "1");
            }
        }
    }

    if !grid_kind.is_virtex2() && grid_kind != GridKind::FpgaCore {
        // PTE2OMUX
        for tile in ["INT.DCM", "INT.DCM.S3E.DUMMY"] {
            if !ctx.has_tile(tile) {
                continue;
            }
            let node_kind = intdb.get_node(tile);
            let bel = "PTE2OMUX";
            for i in 0..4 {
                let bel_id = BelId::from_idx(1 + i);
                let bel_data = &intdb.nodes[node_kind].bels[bel_id];
                let mux_name = intdb.nodes[node_kind].bels.key(bel_id);
                let mut diffs = vec![];
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    let mut diff =
                        ctx.state
                            .get_diff(tile, bel, format!("MUX.{mux_name}"), pin_name);
                    if matches!(&pin_name[..], "CLKFB" | "CLKIN" | "PSCLK") {
                        diff.discard_bits(&ctx.item_int_inv(&[tile], tile, mux_name, pin_name));
                    }
                    diffs.push((pin_name.to_string(), diff));
                }
                ctx.tiledb.insert(
                    tile,
                    bel,
                    format!("MUX.{mux_name}"),
                    xlat_enum_default(diffs, "NONE"),
                );
            }
        }
    }
}
