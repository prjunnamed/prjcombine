use prjcombine_interconnect::{
    db::{BelInfo, PinDir},
    grid::TileCoord,
};
use prjcombine_re_collector::legacy::{
    xlat_bit_legacy, xlat_enum_default_legacy, xlat_enum_legacy,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bitvec::BitVec, bsdata::TileItemKind};
use prjcombine_virtex2::{
    chip::ChipKind, defs, defs::spartan3::tcls as tcls_s3, defs::virtex2::tcls as tcls_v2,
};

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
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        for (tcid, tcname, _) in &backend.edev.db.tile_classes {
            if !tcname.starts_with("HROW") {
                continue;
            }
            for &tcrd in &backend.edev.tile_index[tcid] {
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
                    fuzzer = fuzzer.base(Key::TileMutex(tcrd, o.into()), i);
                    (fuzzer, _) = BasePip::new(
                        NoopRelation,
                        PipWire::BelPinNear(defs::bslots::HROW, o.into()),
                        PipWire::BelPinNear(defs::bslots::HROW, i.into()),
                    )
                    .apply(backend, tcrd, fuzzer)?;
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
    let (edev, chip_kind) = match backend.edev {
        ExpandedDevice::Virtex2(edev) => (edev, edev.chip.kind),
        _ => unreachable!(),
    };

    if devdata_only {
        if chip_kind.is_spartan3a() {
            // CLK[LR]
            let (clkl, clkr) = match chip_kind {
                ChipKind::Spartan3E => (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E),
                ChipKind::Spartan3A => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
                ChipKind::Spartan3ADsp => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
                _ => unreachable!(),
            };
            for tile in [clkl, clkr] {
                let mut ctx = FuzzCtx::new_id(session, backend, tile);
                let mut bctx = ctx.bel(defs::bslots::PCILOGICSE);
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
    let (clkb, clkt) = match chip_kind {
        ChipKind::Virtex2 => (tcls_v2::CLK_S_V2, tcls_v2::CLK_N_V2),
        ChipKind::Virtex2P => (tcls_v2::CLK_S_V2P, tcls_v2::CLK_N_V2P),
        ChipKind::Virtex2PX => (tcls_v2::CLK_S_V2PX, tcls_v2::CLK_N_V2PX),
        ChipKind::Spartan3 => (tcls_s3::CLK_S_S3, tcls_s3::CLK_N_S3),
        ChipKind::FpgaCore => (tcls_s3::CLK_S_FC, tcls_s3::CLK_N_FC),
        ChipKind::Spartan3E => (tcls_s3::CLK_S_S3E, tcls_s3::CLK_N_S3E),
        ChipKind::Spartan3A => (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A),
        ChipKind::Spartan3ADsp => (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A),
    };
    let bufg_num = if chip_kind.is_virtex2() { 8 } else { 4 };
    for tcid in [clkb, clkt] {
        let mut ctx = FuzzCtx::new_id(session, backend, tcid);
        for i in 0..bufg_num {
            let mut bctx = ctx.bel(defs::bslots::BUFGMUX[i]);
            if edev.chip.kind != ChipKind::FpgaCore {
                if edev.chip.kind.is_virtex2() {
                    bctx.build()
                        .prop(StabilizeGclkc)
                        .test_manual("PRESENT", "1")
                        .mode("BUFGMUX")
                        .commit();
                } else {
                    bctx.build()
                        .test_manual("PRESENT", "1")
                        .mode("BUFGMUX")
                        .commit();
                }
                bctx.mode("BUFGMUX")
                    .global_mutex("BUFG", "TEST")
                    .attr("DISABLE_ATTR", "LOW")
                    .test_inv("S");
                bctx.mode("BUFGMUX")
                    .global_mutex("BUFG", "TEST")
                    .pin("S")
                    .test_enum("DISABLE_ATTR", &["HIGH", "LOW"]);
                let inps = if chip_kind.is_spartan3ea() {
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
        if chip_kind.is_virtex2() {
            let bels = if edev.db.tile_classes[tcid]
                .bels
                .contains_id(defs::bslots::GLOBALSIG_S[0])
            {
                defs::bslots::GLOBALSIG_S
            } else {
                defs::bslots::GLOBALSIG_N
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
            let bel = if edev.db.tile_classes[tcid]
                .bels
                .contains_id(defs::bslots::GLOBALSIG_S[0])
            {
                defs::bslots::GLOBALSIG_S[0]
            } else {
                defs::bslots::GLOBALSIG_N[0]
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

    if chip_kind.is_spartan3ea() {
        // CLK[LR]
        let (clkl, clkr) = match chip_kind {
            ChipKind::Spartan3E => (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E),
            ChipKind::Spartan3A => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
            ChipKind::Spartan3ADsp => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
            _ => unreachable!(),
        };
        for tcid in [clkl, clkr] {
            let mut ctx = FuzzCtx::new_id(session, backend, tcid);
            for i in 0..8 {
                let mut bctx = ctx.bel(defs::bslots::BUFGMUX[i]);
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
            let mut bctx = ctx.bel(defs::bslots::PCILOGICSE);
            bctx.build()
                .global_mutex("PCILOGICSE", "NONE")
                .test_manual("PRESENT", "1")
                .mode("PCILOGICSE")
                .commit();
            if chip_kind.is_spartan3a() {
                for val in ["LOW", "MED", "HIGH", "NILL"] {
                    bctx.mode("PCILOGICSE")
                        .global_mutex_here("PCILOGICSE")
                        .test_manual("DELAY", val)
                        .global("pci_ce_delay_left", val)
                        .global("pci_ce_delay_right", val)
                        .commit();
                }
            }

            let mut bctx = ctx.bel(defs::bslots::GLOBALSIG_WE);
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

    if chip_kind.is_virtex2() {
        // CLKC
        let mut ctx = FuzzCtx::new_id(session, backend, tcls_v2::CLKC);
        let mut bctx = ctx.bel(defs::bslots::CLKC);
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
        for tcid in [tcls_v2::HROW, tcls_v2::HROW_S, tcls_v2::HROW_N] {
            if let Some(mut ctx) = FuzzCtx::try_new_id(session, backend, tcid) {
                let mut bctx = ctx.bel(defs::bslots::HROW);
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
        let mut ctx = FuzzCtx::new_id(session, backend, tcls_s3::CLKC_50A);
        let mut bctx = ctx.bel(defs::bslots::CLKC_50A);
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
        let mut ctx = FuzzCtx::new_id(session, backend, tcls_s3::CLKC);
        let mut bctx = ctx.bel(defs::bslots::CLKC);
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

        // CLKQC
        if chip_kind.is_spartan3ea() {
            let mut ctx = FuzzCtx::new_id(session, backend, tcls_s3::CLKQC_S3E);
            let mut bctx = ctx.bel(defs::bslots::CLKQC);
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
            let mut ctx = FuzzCtx::new_id(session, backend, tcls_s3::CLKQC_S3);
            let mut bctx = ctx.bel(defs::bslots::CLKQC);
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

        // HROW
        let mut ctx = FuzzCtx::new_id(session, backend, tcls_s3::HROW);
        let mut bctx = ctx.bel(defs::bslots::HROW);
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

    // HCLK
    for &(tcid, is_s, is_n, is_0, is_dsp) in if chip_kind.is_virtex2() {
        [(tcls_v2::HCLK, false, false, false, false)].as_slice()
    } else {
        [
            (tcls_s3::HCLK, false, false, false, false),
            (tcls_s3::HCLK_S, true, false, false, false),
            (tcls_s3::HCLK_N, false, true, false, false),
            (tcls_s3::HCLK_UNI, false, false, false, false),
            (tcls_s3::HCLK_UNI_S, true, false, false, false),
            (tcls_s3::HCLK_UNI_N, false, true, false, false),
            (tcls_s3::HCLK_0, false, false, true, false),
            (tcls_s3::HCLK_DSP, false, false, false, true),
        ]
        .as_slice()
    } {
        let Some(mut ctx) = FuzzCtx::try_new_id(session, backend, tcid) else {
            continue;
        };
        if !is_0 && !is_dsp {
            let mut bctx = ctx.bel(defs::bslots::HCLK);
            for i in 0..8 {
                let inp_name = format!("IN{i}");
                for bt in ["B", "T"] {
                    if bt == "T" && is_s {
                        continue;
                    }
                    if bt == "B" && is_n {
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
        let slot = if is_dsp {
            defs::bslots::GLOBALSIG_DSP
        } else {
            defs::bslots::GLOBALSIG
        };
        let mut bctx = ctx.bel(slot);
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode("GLOBALSIG")
            .commit();
        if chip_kind.is_virtex2() {
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

    if !chip_kind.is_spartan3ea() && chip_kind != ChipKind::FpgaCore {
        // DCMCONN
        for tcid in if chip_kind.is_virtex2() {
            [tcls_v2::DCMCONN_S, tcls_v2::DCMCONN_N]
        } else {
            [tcls_s3::DCMCONN_S, tcls_s3::DCMCONN_N]
        } {
            let mut ctx = FuzzCtx::new_id(session, backend, tcid);
            let mut bctx = ctx.bel(defs::bslots::DCMCONN);
            let num_bus = if chip_kind.is_virtex2() { 8 } else { 4 };
            for i in 0..num_bus {
                let out_name = format!("OUTBUS{i}");
                let in_name = format!("OUT{ii}", ii = i % 4);
                let mut builder = bctx.build().row_mutex_here("DCMCONN");
                if !chip_kind.is_virtex2() {
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
    if chip_kind.is_spartan3ea() {
        // PCI_CE_*
        for (tile, bel) in [
            ("PCI_CE_S", defs::bslots::PCI_CE_S),
            ("PCI_CE_N", defs::bslots::PCI_CE_N),
            ("PCI_CE_W", defs::bslots::PCI_CE_W),
            ("PCI_CE_E", defs::bslots::PCI_CE_E),
            ("PCI_CE_CNR", defs::bslots::PCI_CE_CNR),
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

    if !chip_kind.is_virtex2() && chip_kind != ChipKind::FpgaCore {
        // PTE2OMUX
        for tcid in [tcls_s3::INT_DCM, tcls_s3::INT_DCM_S3E_DUMMY] {
            for i in 0..4 {
                let Some(mut ctx) = FuzzCtx::try_new_id(session, backend, tcid) else {
                    continue;
                };
                let mut bctx = ctx.bel(defs::bslots::PTE2OMUX[i]);
                let bel_data = &backend.edev.db[tcid].bels[defs::bslots::PTE2OMUX[i]];
                let BelInfo::Legacy(bel_data) = bel_data else {
                    unreachable!()
                };
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    bctx.build()
                        .prop(IntMutex::new("PTE2OMUX".into()))
                        .global_mutex("PSCLK", "PTE2OMUX")
                        .mutex("OUT", pin_name.as_str())
                        .test_manual(format!("MUX.PTE2OMUX[{i}]"), pin_name)
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
    let intdb = ctx.edev.db;

    if devdata_only {
        if grid_kind.is_spartan3a() {
            // CLK_[WE]
            let (clkl, clkr) = match grid_kind {
                ChipKind::Spartan3E => ("CLK_W_S3E", "CLK_E_S3E"),
                ChipKind::Spartan3A => ("CLK_W_S3A", "CLK_E_S3A"),
                ChipKind::Spartan3ADsp => ("CLK_W_S3A", "CLK_E_S3A"),
                _ => unreachable!(),
            };
            for tile in [clkl, clkr] {
                let bel = "PCILOGICSE";
                let default = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
                let item = ctx.item(tile, bel, "DELAY");
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

    // CLK_[SN]
    let (clkb, clkt) = match grid_kind {
        ChipKind::Virtex2 => ("CLK_S_V2", "CLK_N_V2"),
        ChipKind::Virtex2P => ("CLK_S_V2P", "CLK_N_V2P"),
        ChipKind::Virtex2PX => ("CLK_S_V2PX", "CLK_N_V2PX"),
        ChipKind::Spartan3 => ("CLK_S_S3", "CLK_N_S3"),
        ChipKind::FpgaCore => ("CLK_S_FC", "CLK_N_FC"),
        ChipKind::Spartan3E => ("CLK_S_S3E", "CLK_N_S3E"),
        ChipKind::Spartan3A => ("CLK_S_S3A", "CLK_N_S3A"),
        ChipKind::Spartan3ADsp => ("CLK_S_S3A", "CLK_N_S3A"),
    };
    let bufg_num = if grid_kind.is_virtex2() { 8 } else { 4 };
    for tile in [clkb, clkt] {
        for i in 0..bufg_num {
            if edev.chip.kind != ChipKind::FpgaCore {
                let tcid = intdb.get_tile_class(tile);
                let bel = &intdb[tcid].bels[defs::bslots::BUFGMUX[i]];
                let BelInfo::Legacy(bel) = bel else {
                    unreachable!()
                };
                let pin = &bel.pins["S"];
                let bel = format!("BUFGMUX[{i}]");
                let bel = &bel;
                ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
                    .assert_empty();
                assert_eq!(pin.wires.len(), 1);
                let wire = pin.wires.first().unwrap();
                let sinv = ctx.extract_bit_bi_legacy(tile, bel, "SINV", "S", "S_B");
                ctx.insert(
                    tile,
                    "CLK_INT",
                    format!("INV.{:#}.{}", wire.cell, intdb.wires.key(wire.wire)),
                    sinv,
                );
                ctx.collect_enum_legacy(tile, bel, "DISABLE_ATTR", &["HIGH", "LOW"]);
                let inps = if grid_kind.is_spartan3ea() {
                    &["INT", "CKIL", "CKIR", "DCM_OUT_L", "DCM_OUT_R"][..]
                } else {
                    &["INT", "CKI", "DCM_OUT_L", "DCM_OUT_R"]
                };
                ctx.collect_enum_legacy(tile, bel, "MUX.CLK", inps);
            } else {
                let bel = format!("BUFGMUX[{i}]");
                let bel = &bel;
                ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
                    .assert_empty();
                ctx.collect_enum_legacy(tile, bel, "MUX.CLK", &["INT", "CKI"]);
            }
        }
    }

    if grid_kind.is_spartan3ea() {
        // CLK_[WE]
        let (clkl, clkr) = match grid_kind {
            ChipKind::Spartan3E => ("CLK_W_S3E", "CLK_E_S3E"),
            ChipKind::Spartan3A => ("CLK_W_S3A", "CLK_E_S3A"),
            ChipKind::Spartan3ADsp => ("CLK_W_S3A", "CLK_E_S3A"),
            _ => unreachable!(),
        };
        for tile in [clkl, clkr] {
            for i in 0..8 {
                let bel = format!("BUFGMUX[{i}]");
                ctx.get_diff_legacy(tile, &bel, "PRESENT", "1")
                    .assert_empty();
                ctx.collect_inv(tile, &bel, "S");
                ctx.collect_enum_legacy(tile, &bel, "DISABLE_ATTR", &["HIGH", "LOW"]);
                ctx.collect_enum_legacy(tile, &bel, "MUX.CLK", &["INT", "CKI", "DCM_OUT"]);
            }
            let bel = "PCILOGICSE";
            let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
            if grid_kind.is_spartan3a() {
                let mut diffs = vec![];
                let mut default = None;
                for val in ["LOW", "MED", "HIGH", "NILL"] {
                    let diff = ctx.get_diff_legacy(tile, bel, "DELAY", val);
                    if diff.bits.is_empty() {
                        default = Some(val);
                    }
                    diffs.push((val.to_string(), diff));
                }
                let default = default.unwrap();
                ctx.insert_device_data("PCILOGICSE:DELAY_DEFAULT", default.to_string());
                let item = xlat_enum_legacy(diffs);
                present.discard_bits_legacy(&item);
                ctx.insert(tile, bel, "DELAY", item);
            }
            ctx.insert(tile, bel, "ENABLE", xlat_bit_legacy(present));
        }
    }

    if grid_kind.is_virtex2() {
        // HROW
        for tile in ["HROW", "HROW_S", "HROW_N"] {
            let bel = "HROW";
            if !ctx.has_tile(tile) {
                continue;
            }
            for i in 0..8 {
                for lr in ["L", "R"] {
                    let out_name = format!("MUX.OUT_{lr}{i}");
                    ctx.collect_enum_legacy(
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
            ctx.collect_enum_legacy(tile, bel, out_l, &[in_l, in_bt]);
            ctx.collect_enum_legacy(tile, bel, out_r, &[in_r, in_bt]);
        }
    } else if grid_kind.is_spartan3ea() {
        // CLKQC
        let tile = "CLKQC_S3E";
        let bel = "CLKQC";
        for i in 0..8 {
            for bt in ["B", "T"] {
                let out_name = format!("MUX.OUT_{bt}{i}");
                ctx.collect_enum_default_legacy(
                    tile,
                    bel,
                    &out_name,
                    &[&format!("IN_LR{i}"), &format!("IN_CORE{i}")],
                    "NONE",
                );
            }
        }
    } else {
        // CLKQC
        let tile = "CLKQC_S3";
        let bel = "CLKQC";
        for i in 0..8 {
            for bt in ["B", "T"] {
                ctx.collect_bit_legacy(
                    tile,
                    bel,
                    &format!("BUF.OUT_{bt}{i}"),
                    &format!("IN_CORE{i}"),
                );
            }
        }
    }

    // HCLK
    for tile in [
        "HCLK",
        "HCLK_S",
        "HCLK_N",
        "HCLK_UNI",
        "HCLK_UNI_S",
        "HCLK_UNI_N",
    ] {
        if tile != "HCLK" && (grid_kind.is_virtex2() || grid_kind == ChipKind::FpgaCore) {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "HCLK";
        for i in 0..8 {
            let inp_name = format!("IN{i}");
            let uni_name = format!("OUT{i}");
            for bt in ["B", "T"] {
                if bt == "T" && tile.ends_with("_S") {
                    continue;
                }
                if bt == "B" && tile.ends_with("_N") {
                    continue;
                }
                let out_name = format!("OUT_{bt}{i}");
                let attr = if tile.starts_with("HCLK_UNI") {
                    format!("BUF.{uni_name}")
                } else {
                    format!("BUF.{out_name}")
                };
                let item = ctx.extract_bit_legacy(tile, bel, &out_name, &inp_name);
                ctx.insert(tile, bel, attr, item);
            }
        }
    }

    // DCMCONN
    if grid_kind.is_virtex2() {
        for tile in ["DCMCONN_S", "DCMCONN_N"] {
            let bel = "DCMCONN";
            for i in 0..8 {
                ctx.collect_bit_legacy(tile, bel, &format!("BUF.OUTBUS{i}"), "1");
            }
        }
    }

    if !grid_kind.is_virtex2() && grid_kind != ChipKind::FpgaCore {
        // PTE2OMUX
        for tile in ["INT_DCM", "INT_DCM_S3E_DUMMY"] {
            if !ctx.has_tile(tile) {
                continue;
            }
            let tcid = intdb.get_tile_class(tile);
            let bel = "PTE2OMUX";
            for i in 0..4 {
                let bel_id = defs::bslots::PTE2OMUX[i];
                let bel_data = &intdb[tcid].bels[bel_id];
                let BelInfo::Legacy(bel_data) = bel_data else {
                    unreachable!()
                };
                let mux_name = intdb.bel_slots.key(bel_id);
                let mut diffs = vec![];
                for (pin_name, pin_data) in &bel_data.pins {
                    if pin_data.dir == PinDir::Output {
                        continue;
                    }
                    let mut diff = ctx.get_diff_legacy(
                        tile,
                        format!("PTE2OMUX[{i}]"),
                        format!("MUX.{mux_name}"),
                        pin_name,
                    );
                    if matches!(&pin_name[..], "CLKFB" | "CLKIN" | "PSCLK") {
                        diff.discard_bits_legacy(&ctx.item_int_inv(
                            &[tile],
                            tile,
                            mux_name,
                            pin_name,
                        ));
                    }
                    diffs.push((pin_name.to_string(), diff));
                }
                ctx.insert(
                    tile,
                    bel,
                    format!("MUX.{mux_name}"),
                    xlat_enum_default_legacy(diffs, "NONE"),
                );
            }
        }
    }
}
