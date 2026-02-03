use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit, xlat_enum_raw};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex2::{
    chip::ChipKind,
    defs::{
        self, bcls, bslots, devdata, enums,
        spartan3::{tcls as tcls_s3, wires as wires_s3},
        virtex2::{tcls as tcls_v2, wires as wires_v2},
    },
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::FuzzIntPip,
        props::{
            DynProp,
            mutex::{IntMutex, WireMutexExclusive, WireMutexShared},
            pip::{BasePip, PipWire},
            relation::NoopRelation,
        },
    },
    virtex2::specials,
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
        for tcid in [tcls_v2::HROW, tcls_v2::HROW_S, tcls_v2::HROW_N] {
            for &tcrd in &backend.edev.tile_index[tcid] {
                for dc in 0..2 {
                    for i in 0..8 {
                        let dst = TileWireCoord::new_idx(dc, wires_v2::GCLK_ROW[i]);
                        let src = TileWireCoord::new_idx(0, wires_v2::GCLK_S[i]);
                        (fuzzer, _) =
                            BasePip::new(NoopRelation, PipWire::Int(dst), PipWire::Int(src))
                                .apply(backend, tcrd, fuzzer)?;
                        (fuzzer, _) = WireMutexExclusive::new(dst).apply(backend, tcrd, fuzzer)?;
                        (fuzzer, _) = WireMutexShared::new(src).apply(backend, tcrd, fuzzer)?;
                    }
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
            let (clk_w, clk_e) = match chip_kind {
                ChipKind::Spartan3E => (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E),
                ChipKind::Spartan3A => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
                ChipKind::Spartan3ADsp => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
                _ => unreachable!(),
            };
            for tile in [clk_w, clk_e] {
                let mut ctx = FuzzCtx::new(session, backend, tile);
                let mut bctx = ctx.bel(defs::bslots::PCILOGICSE);
                bctx.build()
                    .global_mutex("PCILOGICSE", "NONE")
                    .test_bel_special(specials::PRESENT)
                    .mode("PCILOGICSE")
                    .commit();
            }
        }
        return;
    }

    // CLK[BT]
    let (tcid_s, tcid_n) = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
            (tcls_v2::CLK_S, tcls_v2::CLK_N)
        }
        ChipKind::Spartan3 => (tcls_s3::CLK_S_S3, tcls_s3::CLK_N_S3),
        ChipKind::FpgaCore => (tcls_s3::CLK_S_FC, tcls_s3::CLK_N_FC),
        ChipKind::Spartan3E => (tcls_s3::CLK_S_S3E, tcls_s3::CLK_N_S3E),
        ChipKind::Spartan3A => (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A),
        ChipKind::Spartan3ADsp => (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A),
    };
    let bufg_num = if chip_kind.is_virtex2() { 8 } else { 4 };
    for tcid in [tcid_s, tcid_n] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..bufg_num {
            let mut bctx = ctx.bel(defs::bslots::BUFGMUX[i]);
            if edev.chip.kind != ChipKind::FpgaCore {
                if edev.chip.kind.is_virtex2() {
                    bctx.build()
                        .prop(StabilizeGclkc)
                        .null_bits()
                        .test_bel_special(specials::PRESENT)
                        .mode("BUFGMUX")
                        .commit();
                } else {
                    bctx.build()
                        .null_bits()
                        .test_bel_special(specials::PRESENT)
                        .mode("BUFGMUX")
                        .commit();
                }
                bctx.mode("BUFGMUX")
                    .global_mutex("BUFG", "TEST")
                    .attr("DISABLE_ATTR", "LOW")
                    .test_bel_input_inv_auto(bcls::BUFGMUX::S);
                bctx.mode("BUFGMUX")
                    .global_mutex("BUFG", "TEST")
                    .pin("S")
                    .test_bel_attr_bool_rename(
                        "DISABLE_ATTR",
                        bcls::BUFGMUX::INIT_OUT,
                        "LOW",
                        "HIGH",
                    );
            } else {
                bctx.build()
                    .null_bits()
                    .test_bel_special(specials::PRESENT)
                    .mode("BUFG")
                    .commit();
            }
        }
        if chip_kind.is_virtex2() {
            for bel in defs::bslots::GLOBALSIG_BUFG {
                let mut bctx = ctx.bel(bel);
                bctx.build()
                    .null_bits()
                    .test_bel_special(specials::PRESENT)
                    .mode("GLOBALSIG")
                    .commit();
                for attr in ["DOWN1MUX", "UP1MUX", "DOWN2MUX", "UP2MUX"] {
                    bctx.mode("GLOBALSIG")
                        .null_bits()
                        .test_bel_attr_bool_rename(
                            attr,
                            bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                            "0",
                            "1",
                        );
                }
            }
        } else {
            let mut bctx = ctx.bel(defs::bslots::GLOBALSIG_BUFG[0]);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("GLOBALSIG")
                .commit();
            bctx.mode("GLOBALSIG")
                .null_bits()
                .test_bel_attr_bool_rename(
                    "ENABLE_GLOBALS",
                    bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                    "0",
                    "1",
                );
        }
    }

    if chip_kind.is_spartan3ea() {
        // CLK[LR]
        let (clk_w, clk_e) = match chip_kind {
            ChipKind::Spartan3E => (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E),
            ChipKind::Spartan3A => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
            ChipKind::Spartan3ADsp => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
            _ => unreachable!(),
        };
        for tcid in [clk_w, clk_e] {
            let mut ctx = FuzzCtx::new(session, backend, tcid);
            for i in 0..8 {
                let mut bctx = ctx.bel(defs::bslots::BUFGMUX[i]);
                bctx.build()
                    .null_bits()
                    .test_bel_special(specials::PRESENT)
                    .mode("BUFGMUX")
                    .commit();

                bctx.mode("BUFGMUX")
                    .attr("DISABLE_ATTR", "LOW")
                    .test_bel_input_inv_auto(bcls::BUFGMUX::S);
                bctx.mode("BUFGMUX").pin("S").test_bel_attr_bool_rename(
                    "DISABLE_ATTR",
                    bcls::BUFGMUX::INIT_OUT,
                    "LOW",
                    "HIGH",
                );
            }
            let mut bctx = ctx.bel(defs::bslots::PCILOGICSE);
            bctx.build()
                .global_mutex("PCILOGICSE", "NONE")
                .test_bel_special(specials::PRESENT)
                .mode("PCILOGICSE")
                .commit();
            if chip_kind.is_spartan3a() {
                for (val, vname) in &backend.edev.db[enums::PCILOGICSE_DELAY].values {
                    bctx.mode("PCILOGICSE")
                        .global_mutex_here("PCILOGICSE")
                        .test_bel_attr_val(bcls::PCILOGICSE::DELAY, val)
                        .global("pci_ce_delay_left", vname)
                        .global("pci_ce_delay_right", vname)
                        .commit();
                }
            }

            let mut bctx = ctx.bel(defs::bslots::GLOBALSIG_BUFG[0]);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("GLOBALSIG")
                .commit();
            bctx.mode("GLOBALSIG")
                .null_bits()
                .test_bel_attr_bool_rename(
                    "ENABLE_GLOBALS",
                    bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                    "0",
                    "1",
                );
        }
    }

    // HCLK
    for &tcid in if chip_kind.is_virtex2() {
        [tcls_v2::HCLK].as_slice()
    } else {
        [tcls_s3::HCLK, tcls_s3::HCLK_UNI].as_slice()
    } {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(defs::bslots::GLOBALSIG_HCLK);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("GLOBALSIG")
            .commit();
        if chip_kind.is_virtex2() {
            for (attr, aname) in [
                (bcls::GLOBALSIG_HCLK_V2::GWE_GHIGH_S_ENABLE, "DOWN1MUX"),
                (bcls::GLOBALSIG_HCLK_V2::GSR_N_ENABLE, "UP1MUX"),
                (bcls::GLOBALSIG_HCLK_V2::GSR_S_ENABLE, "DOWN2MUX"),
                (bcls::GLOBALSIG_HCLK_V2::GWE_GHIGH_N_ENABLE, "UP2MUX"),
            ] {
                bctx.mode("GLOBALSIG")
                    .null_bits()
                    .test_bel_attr_bool_rename(aname, attr, "0", "1");
            }
        } else {
            bctx.mode("GLOBALSIG")
                .null_bits()
                .test_bel_attr_bool_rename(
                    "ENABLE_GLOBALS",
                    bcls::GLOBALSIG_HCLK_S3::ENABLE,
                    "0",
                    "1",
                );
        }
    }

    if !chip_kind.is_virtex2() && chip_kind != ChipKind::FpgaCore {
        // PTE2OMUX
        for tcid in [tcls_s3::INT_DCM, tcls_s3::INT_DCM_S3E_DUMMY] {
            let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
                continue;
            };
            let BelInfo::SwitchBox(ref sb) = backend.edev.db[tcid].bels[bslots::PTE2OMUX] else {
                unreachable!()
            };
            for item in &sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    unreachable!()
                };
                for &src in mux.src.keys() {
                    ctx.build()
                        .prop(IntMutex::new("PTE2OMUX".into()))
                        .global_mutex("PSCLK", "PTE2OMUX")
                        .prop(WireMutexShared::new(src.tw))
                        .prop(WireMutexExclusive::new(mux.dst))
                        .test_raw(DiffKey::Routing(tcid, mux.dst, src))
                        .prop(FuzzIntPip::new(mux.dst, src.tw))
                        .commit();
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let (edev, chip_kind) = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => (edev, edev.chip.kind),
        _ => unreachable!(),
    };

    if devdata_only {
        if chip_kind.is_spartan3a() {
            // CLK_[WE]
            let (clk_w, clk_e) = match chip_kind {
                ChipKind::Spartan3E => (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E),
                ChipKind::Spartan3A => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
                ChipKind::Spartan3ADsp => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
                _ => unreachable!(),
            };
            for tcid in [clk_w, clk_e] {
                let bslot = bslots::PCILOGICSE;
                let default = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
                let item = ctx.bel_attr_enum(tcid, bslot, bcls::PCILOGICSE::DELAY);
                let val: BitVec = item
                    .bits
                    .iter()
                    .map(|bit| default.bits.contains_key(bit))
                    .collect();
                for (k, v) in &item.values {
                    if *v == val {
                        ctx.insert_devdata_enum(devdata::PCILOGICSE_DELAY, k);
                        break;
                    }
                }
            }
        }
        return;
    }

    // CLK_[SN]
    let (clk_s, clk_n) = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => {
            (tcls_v2::CLK_S, tcls_v2::CLK_N)
        }
        ChipKind::Spartan3 => (tcls_s3::CLK_S_S3, tcls_s3::CLK_N_S3),
        ChipKind::FpgaCore => (tcls_s3::CLK_S_FC, tcls_s3::CLK_N_FC),
        ChipKind::Spartan3E => (tcls_s3::CLK_S_S3E, tcls_s3::CLK_N_S3E),
        ChipKind::Spartan3A => (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A),
        ChipKind::Spartan3ADsp => (tcls_s3::CLK_S_S3A, tcls_s3::CLK_N_S3A),
    };
    let bufg_num = if chip_kind.is_virtex2() { 8 } else { 4 };
    for tcid in [clk_s, clk_n] {
        for i in 0..bufg_num {
            let bslot = bslots::BUFGMUX[i];
            if edev.chip.kind != ChipKind::FpgaCore {
                ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::BUFGMUX::S);
                ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGMUX::INIT_OUT);
            }
        }
    }

    if chip_kind.is_virtex2() {
        for (tcid, bit) in [(tcls_v2::CLK_S, 6), (tcls_v2::CLK_N, 8)] {
            for i in 0..2 {
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslots::GLOBALSIG_BUFG[i],
                    bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                    TileBit::new(1, i * 3, bit).neg(),
                );
            }
        }
    } else {
        for (tcid, bit) in [(clk_s, 63), (clk_n, 0)] {
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::GLOBALSIG_BUFG[0],
                bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                TileBit::new(0, 0, bit).neg(),
            );
        }

        if chip_kind == ChipKind::Spartan3E {
            ctx.insert_bel_attr_bool(
                tcls_s3::CLK_W_S3E,
                bslots::GLOBALSIG_BUFG[0],
                bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                TileBit::new(4, 0, 12).neg(),
            );
            ctx.insert_bel_attr_bool(
                tcls_s3::CLK_E_S3E,
                bslots::GLOBALSIG_BUFG[0],
                bcls::GLOBALSIG_BUFG::GWE_ENABLE,
                TileBit::new(4, 0, 4).neg(),
            );
        }
    }

    if chip_kind.is_spartan3ea() {
        // CLK_[WE]
        let (clkl, clkr) = match chip_kind {
            ChipKind::Spartan3E => (tcls_s3::CLK_W_S3E, tcls_s3::CLK_E_S3E),
            ChipKind::Spartan3A => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
            ChipKind::Spartan3ADsp => (tcls_s3::CLK_W_S3A, tcls_s3::CLK_E_S3A),
            _ => unreachable!(),
        };
        for tcid in [clkl, clkr] {
            for i in 0..8 {
                let bslot = bslots::BUFGMUX[i];
                ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::BUFGMUX::S);
                ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGMUX::INIT_OUT);
            }
            let bslot = bslots::PCILOGICSE;
            let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            if chip_kind.is_spartan3a() {
                ctx.collect_bel_attr(tcid, bslot, bcls::PCILOGICSE::DELAY);
                let item = ctx.bel_attr_enum(tcid, bslot, bcls::PCILOGICSE::DELAY);
                let val: BitVec = item
                    .bits
                    .iter()
                    .map(|bit| match present.bits.remove(bit) {
                        Some(true) => true,
                        None => false,
                        _ => unreachable!(),
                    })
                    .collect();
                for (k, v) in &item.values {
                    if *v == val {
                        ctx.insert_devdata_enum(devdata::PCILOGICSE_DELAY, k);
                        break;
                    }
                }
            }
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::PCILOGICSE::ENABLE, xlat_bit(present));
        }
    }

    if chip_kind.is_virtex2() {
        let tcid = tcls_v2::HCLK;
        for (attr, frame) in [
            (bcls::GLOBALSIG_HCLK_V2::GWE_GHIGH_N_ENABLE, 18),
            (bcls::GLOBALSIG_HCLK_V2::GWE_GHIGH_S_ENABLE, 19),
            (bcls::GLOBALSIG_HCLK_V2::GSR_N_ENABLE, 20),
            (bcls::GLOBALSIG_HCLK_V2::GSR_S_ENABLE, 21),
        ] {
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::GLOBALSIG_HCLK,
                attr,
                TileBit::new(0, frame, 0).neg(),
            );
        }
    } else {
        for tcid in [tcls_s3::HCLK, tcls_s3::HCLK_UNI] {
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::GLOBALSIG_HCLK,
                bcls::GLOBALSIG_HCLK_S3::ENABLE,
                TileBit::new(0, 0, 0).neg(),
            );
        }
    }

    if !chip_kind.is_virtex2() && chip_kind != ChipKind::FpgaCore {
        // PTE2OMUX
        for tcid in [tcls_s3::INT_DCM, tcls_s3::INT_DCM_S3E_DUMMY] {
            if !ctx.has_tcls(tcid) {
                continue;
            }
            let BelInfo::SwitchBox(ref sb) = ctx.edev.db[tcid].bels[bslots::PTE2OMUX] else {
                unreachable!()
            };
            for item in &sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    unreachable!()
                };
                let mut diffs = vec![];
                for &src in mux.src.keys() {
                    let mut diff = ctx.get_diff_raw(&DiffKey::Routing(tcid, mux.dst, src));
                    if wires_s3::IMUX_CLK_OPTINV.contains(src.wire) {
                        diff.discard_bits(&[ctx.sb_inv(tcid, src.tw).bit]);
                    }
                    assert!(!diff.bits.is_empty());
                    diffs.push((Some(src), diff));
                }
                diffs.push((None, Diff::default()));

                ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
            }
        }
    }
}
