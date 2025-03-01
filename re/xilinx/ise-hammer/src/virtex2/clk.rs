use bitvec::vec::BitVec;
use prjcombine_interconnect::{db::PinDir, grid::NodeLoc};
use prjcombine_re_fpga_hammer::{FuzzerProp, xlat_bit, xlat_enum, xlat_enum_default};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::tiledb::TileItemKind;
use prjcombine_virtex2::{bels, chip::ChipKind};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            mutex::IntMutex,
            pip::{BasePip, PinFar, PipWire},
            relation::NoopRelation,
        },
    },
};

#[derive(Clone, Debug)]
struct StabilizeGclkc;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for StabilizeGclkc {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        _nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        for (node_kind, node_name, _) in &backend.egrid.db.nodes {
            if !node_name.starts_with("GCLKC") {
                continue;
            }
            for &nloc in &backend.egrid.node_index[node_kind] {
                for (o, i) in [
                    ("OUT_L0", "IN_B0"),
                    ("OUT_R0", "IN_B0"),
                    ("OUT_L1", "IN_B1"),
                    ("OUT_R1", "IN_B1"),
                    ("OUT_L2", "IN_B2"),
                    ("OUT_R2", "IN_B2"),
                    ("OUT_L3", "IN_B3"),
                    ("OUT_R3", "IN_B3"),
                    ("OUT_L4", "IN_B4"),
                    ("OUT_R4", "IN_B4"),
                    ("OUT_L5", "IN_B5"),
                    ("OUT_R5", "IN_B5"),
                    ("OUT_L6", "IN_B6"),
                    ("OUT_R6", "IN_B6"),
                    ("OUT_L7", "IN_B7"),
                    ("OUT_R7", "IN_B7"),
                ] {
                    fuzzer = fuzzer.base(Key::TileMutex(nloc, o.into()), i);
                    (fuzzer, _) = BasePip::new(
                        NoopRelation,
                        PipWire::BelPinNear(bels::GCLKC, o.into()),
                        PipWire::BelPinNear(bels::GCLKC, i.into()),
                    )
                    .apply(backend, nloc, fuzzer)?;
                }
            }
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let (edev, grid_kind) = match backend.edev {
        ExpandedDevice::Virtex2(edev) => (edev, edev.chip.kind),
        _ => unreachable!(),
    };

    if devdata_only {
        if grid_kind.is_spartan3a() {
            // CLK[LR]
            let (clkl, clkr) = match grid_kind {
                ChipKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
                ChipKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
                ChipKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
                _ => unreachable!(),
            };
            for tile in [clkl, clkr] {
                let mut ctx = FuzzCtx::new(session, backend, tile);
                let mut bctx = ctx.bel(bels::PCILOGICSE);
                bctx.build()
                    .global_mutex("PCILOGICSE", "NONE")
                    .test_manual("PRESENT", "1")
                    .mode("PCILOGICSE")
                    .commit();
            }
        }
        return;
    }

    // CLK[BT]
    let (clkb, clkt) = match grid_kind {
        ChipKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
        ChipKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
        ChipKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
        ChipKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
        ChipKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
        ChipKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
        ChipKind::Spartan3A => ("CLKB.S3A", "CLKT.S3A"),
        ChipKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for i in 0..bufg_num {
            let mut bctx = ctx.bel(bels::BUFGMUX[i]);
            if edev.chip.kind != ChipKind::FpgaCore {
                bctx.build()
                    .prop(StabilizeGclkc)
                    .test_manual("PRESENT", "1")
                    .mode("BUFGMUX")
                    .commit();
                bctx.mode("BUFGMUX")
                    .global_mutex("BUFG", "TEST")
                    .attr("DISABLE_ATTR", "LOW")
                    .test_inv("S");
                bctx.mode("BUFGMUX")
                    .global_mutex("BUFG", "TEST")
                    .pin("S")
                    .test_enum("DISABLE_ATTR", &["HIGH", "LOW"]);
                let inps = if grid_kind.is_spartan3ea() {
                    &["CKIL", "CKIR", "DCM_OUT_L", "DCM_OUT_R"][..]
                } else {
                    &["CKI", "DCM_OUT_L", "DCM_OUT_R"]
                };
                for &inp in inps {
                    bctx.build()
                        .mutex("MUX.CLK", inp)
                        .test_manual("MUX.CLK", inp)
                        .pip("CLK", inp)
                        .commit();
                }
                bctx.build()
                    .mutex("MUX.CLK", "INT")
                    .test_manual("MUX.CLK", "INT")
                    .pip("CLK", (PinFar, "CLK"))
                    .commit();
            } else {
                bctx.test_manual("PRESENT", "1").mode("BUFG").commit();
                bctx.build()
                    .mutex("MUX.CLK", "CKI")
                    .test_manual("MUX.CLK", "CKI")
                    .pip("CLK", "CKI")
                    .commit();
                bctx.build()
                    .mutex("MUX.CLK", "INT")
                    .test_manual("MUX.CLK", "INT")
                    .pip("CLK", (PinFar, "CLK"))
                    .commit();
            }
        }
        if grid_kind.is_virtex2() {
            let bels = if tile.starts_with("CLKB") {
                [bels::GLOBALSIG_S0, bels::GLOBALSIG_S1]
            } else {
                [bels::GLOBALSIG_N0, bels::GLOBALSIG_N1]
            };
            for bel in bels {
                let mut bctx = ctx.bel(bel);
                bctx.build()
                    .null_bits()
                    .test_manual("PRESENT", "1")
                    .mode("GLOBALSIG")
                    .commit();
                for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                    bctx.mode("GLOBALSIG")
                        .null_bits()
                        .test_enum(attr, &["0", "1"]);
                }
            }
        } else {
            let bel = if tile.starts_with("CLKB") {
                bels::GLOBALSIG_S
            } else {
                bels::GLOBALSIG_N
            };
            let mut bctx = ctx.bel(bel);
            bctx.build()
                .null_bits()
                .test_manual("PRESENT", "1")
                .mode("GLOBALSIG")
                .commit();
            bctx.mode("GLOBALSIG")
                .null_bits()
                .test_enum("ENABLE_GLOBALS", &["0", "1"]);
        }
    }

    if grid_kind.is_spartan3ea() {
        // CLK[LR]
        let (clkl, clkr) = match grid_kind {
            ChipKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
            ChipKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
            ChipKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
            _ => unreachable!(),
        };
        for tile in [clkl, clkr] {
            let mut ctx = FuzzCtx::new(session, backend, tile);
            for i in 0..8 {
                let mut bctx = ctx.bel(bels::BUFGMUX[i]);
                bctx.test_manual("PRESENT", "1").mode("BUFGMUX").commit();

                bctx.mode("BUFGMUX")
                    .attr("DISABLE_ATTR", "LOW")
                    .test_inv("S");
                bctx.mode("BUFGMUX")
                    .pin("S")
                    .test_enum("DISABLE_ATTR", &["HIGH", "LOW"]);
                for inp in ["CKI", "DCM_OUT"] {
                    bctx.build()
                        .mutex("MUX.CLK", inp)
                        .test_manual("MUX.CLK", inp)
                        .pip("CLK", inp)
                        .commit();
                }
                bctx.build()
                    .mutex("MUX.CLK", "INT")
                    .test_manual("MUX.CLK", "INT")
                    .pip("CLK", (PinFar, "CLK"))
                    .commit();
            }
            let mut bctx = ctx.bel(bels::PCILOGICSE);
            bctx.build()
                .global_mutex("PCILOGICSE", "NONE")
                .test_manual("PRESENT", "1")
                .mode("PCILOGICSE")
                .commit();
            if grid_kind.is_spartan3a() {
                for val in ["LOW", "MED", "HIGH", "NILL"] {
                    bctx.mode("PCILOGICSE")
                        .global_mutex_here("PCILOGICSE")
                        .test_manual("DELAY", val)
                        .global("pci_ce_delay_left", val)
                        .global("pci_ce_delay_right", val)
                        .commit();
                }
            }

            let mut bctx = ctx.bel(bels::GLOBALSIG_WE);
            bctx.build()
                .null_bits()
                .test_manual("PRESENT", "1")
                .mode("GLOBALSIG")
                .commit();
            bctx.mode("GLOBALSIG")
                .null_bits()
                .test_enum("ENABLE_GLOBALS", &["0", "1"]);
        }
    }

    if grid_kind.is_virtex2() {
        // CLKC
        let mut ctx = FuzzCtx::new(session, backend, "CLKC");
        let mut bctx = ctx.bel(bels::CLKC);
        for i in 0..8 {
            for bt in ["B", "T"] {
                bctx.build()
                    .null_bits()
                    .test_manual(format!("FWD_{bt}{i}"), "1")
                    .pip(format!("OUT_{bt}{i}"), format!("IN_{bt}{i}"))
                    .commit();
            }
        }

        // GCLKC
        for tile in ["GCLKC", "GCLKC.B", "GCLKC.T"] {
            if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) {
                let mut bctx = ctx.bel(bels::GCLKC);
                for i in 0..8 {
                    for lr in ["L", "R"] {
                        let out_name = format!("OUT_{lr}{i}");
                        for bt in ["B", "T"] {
                            let inp_name = format!("IN_{bt}{i}");
                            bctx.build()
                                .global_mutex("BUFG", "USE")
                                .tile_mutex(&out_name, &inp_name)
                                .test_manual(format!("MUX.OUT_{lr}{i}"), &inp_name)
                                .pip(out_name.as_str(), inp_name.as_str())
                                .commit();
                        }
                    }
                }
            }
        }
    } else if edev.chip.cols_clkv.is_none() {
        // CLKC_50A
        let mut ctx = FuzzCtx::new(session, backend, "CLKC_50A");
        let mut bctx = ctx.bel(bels::CLKC_50A);
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
                bctx.build()
                    .tile_mutex(out, inp)
                    .test_manual(format!("MUX.{out}"), inp)
                    .pip(out, inp)
                    .commit();
            }
        }
    } else {
        // CLKC
        let mut ctx = FuzzCtx::new(session, backend, "CLKC");
        let mut bctx = ctx.bel(bels::CLKC);
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
            bctx.build()
                .null_bits()
                .test_manual(out, inp)
                .pip(out, inp)
                .commit();
        }

        // GCLKVM
        if grid_kind.is_spartan3ea() {
            let mut ctx = FuzzCtx::new(session, backend, "GCLKVM.S3E");
            let mut bctx = ctx.bel(bels::GCLKVM);
            for i in 0..8 {
                for bt in ["B", "T"] {
                    let out_name = format!("OUT_{bt}{i}");
                    for lr in ["LR", "CORE"] {
                        let inp_name = format!("IN_{lr}{i}");
                        bctx.build()
                            .tile_mutex(&out_name, &inp_name)
                            .test_manual(format!("MUX.{out_name}"), &inp_name)
                            .pip(out_name.as_str(), inp_name.as_str())
                            .commit();
                    }
                }
            }
        } else {
            let mut ctx = FuzzCtx::new(session, backend, "GCLKVM.S3");
            let mut bctx = ctx.bel(bels::GCLKVM);
            for i in 0..8 {
                for bt in ["B", "T"] {
                    let out_name = format!("OUT_{bt}{i}");
                    let inp_name = format!("IN_CORE{i}");
                    bctx.build()
                        .global_mutex("MISR_CLOCK", "NONE")
                        .test_manual(format!("BUF.{out_name}"), &inp_name)
                        .pip(out_name, inp_name)
                        .commit();
                }
            }
        }

        // GCLKVC
        let mut ctx = FuzzCtx::new(session, backend, "GCLKVC");
        let mut bctx = ctx.bel(bels::GCLKVC);
        for i in 0..8 {
            let inp_name = format!("IN{i}");
            for lr in ["L", "R"] {
                let out_name = format!("OUT_{lr}{i}");
                bctx.build()
                    .tile_mutex(&out_name, &inp_name)
                    .null_bits()
                    .test_manual(&out_name, &inp_name)
                    .pip(out_name.as_str(), inp_name.as_str())
                    .commit();
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
        if tile != "GCLKH" && (grid_kind.is_virtex2() || grid_kind == ChipKind::FpgaCore) {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        if tile != "GCLKH.0" && tile != "GCLKH.DSP" {
            let mut bctx = ctx.bel(bels::GCLKH);
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
                    bctx.build()
                        .global_mutex("MISR_CLOCK", "NONE")
                        .tile_mutex(&inp_name, &out_name)
                        .test_manual(&out_name, &inp_name)
                        .pip(out_name.as_str(), inp_name.as_str())
                        .commit();
                }
            }
        }
        let slot = if tile == "GCLKH.DSP" {
            bels::GLOBALSIG_DSP
        } else {
            bels::GLOBALSIG
        };
        let mut bctx = ctx.bel(slot);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("GLOBALSIG")
            .commit();
        if grid_kind.is_virtex2() {
            for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                bctx.mode("GLOBALSIG")
                    .null_bits()
                    .test_enum(attr, &["0", "1"]);
            }
        } else {
            bctx.mode("GLOBALSIG")
                .null_bits()
                .test_enum("ENABLE_GLOBALS", &["0", "1"]);
        }
    }

    if !grid_kind.is_spartan3ea() && grid_kind != ChipKind::FpgaCore {
        // DCMCONN
        for tile in ["DCMCONN.BOT", "DCMCONN.TOP"] {
            let mut ctx = FuzzCtx::new(session, backend, tile);
            let mut bctx = ctx.bel(bels::DCMCONN);
            let num_bus = if grid_kind.is_virtex2() { 8 } else { 4 };
            for i in 0..num_bus {
                let out_name = format!("OUTBUS{i}");
                let in_name = format!("OUT{ii}", ii = i % 4);
                let mut builder = bctx.build().row_mutex_here("DCMCONN");
                if !grid_kind.is_virtex2() {
                    builder = builder.null_bits();
                }
                builder
                    .test_manual(format!("BUF.{out_name}"), "1")
                    .pip(out_name, in_name)
                    .commit();
            }
            for i in 0..num_bus {
                let out_name = format!("CLKPAD{i}");
                let in_name = format!("CLKPADBUS{i}");
                bctx.build()
                    .null_bits()
                    .test_manual(&out_name, "1")
                    .pip(out_name, in_name)
                    .commit();
            }
        }
    }
    if grid_kind.is_spartan3ea() {
        // PCI_CE_*
        for (tile, bel) in [
            ("PCI_CE_S", bels::PCI_CE_S),
            ("PCI_CE_N", bels::PCI_CE_N),
            ("PCI_CE_W", bels::PCI_CE_W),
            ("PCI_CE_E", bels::PCI_CE_E),
            ("PCI_CE_CNR", bels::PCI_CE_CNR),
        ] {
            if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) {
                let mut bctx = ctx.bel(bel);
                bctx.build()
                    .null_bits()
                    .row_mutex_here("DCMCONN")
                    .test_manual("O", "1")
                    .pip("O", "I")
                    .commit();
            }
        }
    }

    if !grid_kind.is_virtex2() && grid_kind != ChipKind::FpgaCore {
        // PTE2OMUX
        for tile in ["INT.DCM", "INT.DCM.S3E.DUMMY"] {
            let node_kind = backend.egrid.db.get_node(tile);
            if backend.egrid.node_index[node_kind].is_empty() {
                continue;
            }
            for i in 0..4 {
                let mut ctx = FuzzCtx::new(session, backend, tile);
                let node_kind = backend.egrid.db.get_node(tile);
                let mut bctx = ctx.bel(bels::PTE2OMUX[i]);
                let bel_data = &backend.egrid.db.nodes[node_kind].bels[bels::PTE2OMUX[i]];
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    bctx.build()
                        .prop(IntMutex::new("PTE2OMUX".into()))
                        .global_mutex("PSCLK", "PTE2OMUX")
                        .mutex("OUT", pin_name.as_str())
                        .test_manual(format!("MUX.PTE2OMUX{i}"), pin_name)
                        .pip("OUT", pin_name.as_str())
                        .commit();
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let (edev, grid_kind) = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => (edev, edev.chip.kind),
        _ => unreachable!(),
    };
    let intdb = ctx.edev.egrid().db;

    if devdata_only {
        if grid_kind.is_spartan3a() {
            // CLK[LR]
            let (clkl, clkr) = match grid_kind {
                ChipKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
                ChipKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
                ChipKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
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
        ChipKind::Virtex2 => ("CLKB.V2", "CLKT.V2"),
        ChipKind::Virtex2P => ("CLKB.V2P", "CLKT.V2P"),
        ChipKind::Virtex2PX => ("CLKB.V2PX", "CLKT.V2PX"),
        ChipKind::Spartan3 => ("CLKB.S3", "CLKT.S3"),
        ChipKind::FpgaCore => ("CLKB.FC", "CLKT.FC"),
        ChipKind::Spartan3E => ("CLKB.S3E", "CLKT.S3E"),
        ChipKind::Spartan3A => ("CLKB.S3A", "CLKT.S3A"),
        ChipKind::Spartan3ADsp => ("CLKB.S3A", "CLKT.S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        for i in 0..bufg_num {
            if edev.chip.kind != ChipKind::FpgaCore {
                let node_kind = intdb.get_node(tile);
                let bel = &intdb.nodes[node_kind].bels[bels::BUFGMUX[i]];
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
                let bel = format!("BUFGMUX{i}");
                let bel = &bel;
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_enum(tile, bel, "MUX.CLK", &["INT", "CKI"]);
            }
        }
    }

    if grid_kind.is_spartan3ea() {
        // CLK[LR]
        let (clkl, clkr) = match grid_kind {
            ChipKind::Spartan3E => ("CLKL.S3E", "CLKR.S3E"),
            ChipKind::Spartan3A => ("CLKL.S3A", "CLKR.S3A"),
            ChipKind::Spartan3ADsp => ("CLKL.S3A", "CLKR.S3A"),
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
    } else if edev.chip.cols_clkv.is_none() {
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
        if tile != "GCLKH" && (grid_kind.is_virtex2() || grid_kind == ChipKind::FpgaCore) {
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

    if !grid_kind.is_virtex2() && grid_kind != ChipKind::FpgaCore {
        // PTE2OMUX
        for tile in ["INT.DCM", "INT.DCM.S3E.DUMMY"] {
            if !ctx.has_tile(tile) {
                continue;
            }
            let node_kind = intdb.get_node(tile);
            let bel = "PTE2OMUX";
            for i in 0..4 {
                let bel_id = bels::PTE2OMUX[i];
                let bel_data = &intdb.nodes[node_kind].bels[bel_id];
                let mux_name = &intdb.bel_slots[bel_id];
                let mut diffs = vec![];
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    let mut diff = ctx.state.get_diff(
                        tile,
                        format!("PTE2OMUX{i}"),
                        format!("MUX.{mux_name}"),
                        pin_name,
                    );
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
