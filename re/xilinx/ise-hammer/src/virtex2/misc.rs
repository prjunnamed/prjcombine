use std::collections::HashSet;

use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::BelAttributeId,
    dir::DirHV,
    grid::{DieId, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_bi,
    xlat_bit_wide, xlat_bit_wide_bi,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::{ExpandedBond, ExpandedDevice, ExpandedNamedDevice};
use prjcombine_types::{
    bitrect::BitRect as _,
    bits,
    bitvec::BitVec,
    bsdata::{BitRectId, RectBitId, RectFrameId, TileBit},
};
use prjcombine_virtex2::{
    chip::{Chip, ChipKind, IoDiffKind},
    defs::{
        self, bcls, bslots, devdata, enums, spartan3::tcls as tcls_s3, tables::IOB_DATA, tslots,
        virtex2::tcls as tcls_v2,
    },
    iob::IobKind,
};
use prjcombine_xilinx_bitstream::{BitRect, Reg};

use crate::{
    backend::{IseBackend, Key, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx, FuzzCtxBel},
        iostd::{DciKind, DiffKind},
        props::{DynProp, pip::PinFar},
    },
    virtex2::{
        io::{get_iostds, iostd_to_row},
        specials,
    },
};

#[derive(Copy, Clone, Debug)]
struct ExtraTileInt;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTileInt {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(ExtraTileInt)
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcrd = tcrd.tile(tslots::INT);
        let tcid = backend.edev[tcrd].class;
        let key = match fuzzer.info.features[0].key {
            DiffKey::BelAttrBit(_, bslot, attr, bit, val) => {
                DiffKey::BelAttrBit(tcid, bslot, attr, bit, val)
            }
            DiffKey::BelInputInv(_, bslot, inp, val) => DiffKey::BelInputInv(tcid, bslot, inp, val),
            _ => unreachable!(),
        };
        fuzzer.info.features.push(FuzzerFeature {
            key,
            rects: backend.edev.tile_bits(tcrd),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ForceBits(EntityVec<BitRectId, BitRect>);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ForceBits {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        fuzzer.info.features[0].rects = self.0.clone();
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    skip_io: bool,
    devdata_only: bool,
) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
        unreachable!()
    };

    let (cnr_sw, cnr_nw, cnr_se, cnr_ne) = match edev.chip.kind {
        ChipKind::Virtex2 => (
            tcls_v2::CNR_SW_V2,
            tcls_v2::CNR_NW_V2,
            tcls_v2::CNR_SE_V2,
            tcls_v2::CNR_NE_V2,
        ),
        ChipKind::Virtex2P | ChipKind::Virtex2PX => (
            tcls_v2::CNR_SW_V2P,
            tcls_v2::CNR_NW_V2P,
            tcls_v2::CNR_SE_V2P,
            tcls_v2::CNR_NE_V2P,
        ),
        ChipKind::Spartan3 => (
            tcls_s3::CNR_SW_S3,
            tcls_s3::CNR_NW_S3,
            tcls_s3::CNR_SE_S3,
            tcls_s3::CNR_NE_S3,
        ),
        ChipKind::FpgaCore => (
            tcls_s3::CNR_SW_FC,
            tcls_s3::CNR_NW_FC,
            tcls_s3::CNR_SE_FC,
            tcls_s3::CNR_NE_FC,
        ),
        ChipKind::Spartan3E => (
            tcls_s3::CNR_SW_S3E,
            tcls_s3::CNR_NW_S3E,
            tcls_s3::CNR_SE_S3E,
            tcls_s3::CNR_NE_S3E,
        ),
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
            tcls_s3::CNR_SW_S3A,
            tcls_s3::CNR_NW_S3A,
            tcls_s3::CNR_SE_S3A,
            tcls_s3::CNR_NE_S3A,
        ),
    };

    let freeze_dci_btiles = EntityVec::from_iter([
        edev.btile_term_h(edev.chip.corner(DirHV::SW).cell),
        edev.btile_term_v(edev.chip.corner(DirHV::SW).cell),
        edev.btile_term_h(edev.chip.corner(DirHV::SW).cell)
            .to_fixup(),
        edev.btile_term_v(edev.chip.corner(DirHV::SW).cell)
            .to_fixup(),
        BitRect::Reg(DieId::from_idx(0), Reg::FakeFreezeDciNops),
        BitRect::RegPresent(DieId::from_idx(0), Reg::FakeFreezeDciNops),
    ]);

    let global = edev.chip.tile_global();

    if devdata_only {
        let mut ctx = FuzzCtx::new(session, backend, cnr_sw);
        let mut bctx = ctx.bel(bslots::MISC_SW);
        if !edev.chip.kind.is_virtex2() {
            for i in 0..4 {
                for (val, vname) in [(false, "0"), (true, "1")] {
                    let mut builder = bctx.build();
                    if edev.chip.kind.is_spartan3a() {
                        builder = builder.extra_fixed_bel_attr_bits_base_bi(
                            global,
                            bslots::GLOBAL,
                            bcls::GLOBAL::SEND_VGG,
                            i,
                            val,
                        );
                    }
                    builder
                        .test_bel_attr_bits_base_bi(bcls::MISC_SW::SEND_VGG, i, val)
                        .global(format!("SEND_VGG{i}"), vname)
                        .commit();
                }
            }
            for (val, vname) in [(false, "NO"), (true, "YES")] {
                let mut builder = bctx.build();
                if edev.chip.kind.is_spartan3a() {
                    builder = builder.extra_fixed_bel_attr_bits_bi(
                        global,
                        bslots::GLOBAL,
                        bcls::GLOBAL::VGG_SENDMAX,
                        val,
                    );
                }
                builder
                    .test_bel_attr_bits_bi(bcls::MISC_SW::VGG_SENDMAX, val)
                    .global("VGG_SENDMAX", vname)
                    .commit();
            }
        }
        if edev.chip.kind.is_virtex2() {
            bctx.build()
                .prop(ForceBits(freeze_dci_btiles))
                .global_mutex("DCI", "FREEZE")
                .no_global("ENCRYPT")
                .test_bel_special(specials::FREEZE_DCI)
                .global("FREEZEDCI", "YES")
                .commit();
        }

        return;
    }

    fn test_pull(bctx: &mut FuzzCtxBel, attr: BelAttributeId, opt: &'static str) {
        for (val, vname) in [
            (enums::IOB_PULL::NONE, "PULLNONE"),
            (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
            (enums::IOB_PULL::PULLUP, "PULLUP"),
        ] {
            bctx.build()
                .test_bel_attr_val(attr, val)
                .global(opt, vname)
                .commit();
        }
    }
    fn test_pullup(bctx: &mut FuzzCtxBel, attr: BelAttributeId, opt: &'static str) {
        for (val, vname) in [
            (enums::IOB_PULL::NONE, "PULLNONE"),
            (enums::IOB_PULL::PULLUP, "PULLUP"),
        ] {
            bctx.build()
                .test_bel_attr_val(attr, val)
                .global(opt, vname)
                .commit();
        }
    }

    if edev.chip.kind == ChipKind::Spartan3 {
        for tile in [cnr_sw, cnr_nw, cnr_se, cnr_ne] {
            let mut ctx = FuzzCtx::new(session, backend, tile);
            for bel in bslots::DCIRESET {
                let mut bctx = ctx.bel(bel);
                bctx.build()
                    .test_bel_attr_bits(bcls::DCIRESET::ENABLE)
                    .mode("DCIRESET")
                    .commit();
            }
        }
    }

    // LL
    {
        let mut ctx = FuzzCtx::new(session, backend, cnr_sw);
        let mut bctx = ctx.bel(bslots::MISC_SW);
        // MISC
        if edev.chip.kind.is_virtex2() {
            bctx.build().test_global_attr_bool_rename(
                "DISABLEBANDGAP",
                bcls::MISC_SW::DISABLE_BANDGAP,
                "NO",
                "YES",
            );
            bctx.build().test_global_attr_bool_rename(
                "RAISEVGG",
                bcls::MISC_SW::RAISE_VGG,
                "NO",
                "YES",
            );
            for (i, opt) in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"]
                .into_iter()
                .enumerate()
            {
                for (val, vname) in [(false, "0"), (true, "1")] {
                    bctx.build()
                        .test_bel_attr_bits_base_bi(bcls::MISC_SW::BCLK_N_DIV2, i, val)
                        .global(opt, vname)
                        .commit()
                }
            }
            for (i, opt) in ["ZCLK_N2", "ZCLK_N4", "ZCLK_N8", "ZCLK_N16", "ZCLK_N32"]
                .into_iter()
                .enumerate()
            {
                for (val, vname) in [(false, "0"), (true, "1")] {
                    bctx.build()
                        .global_mutex("DCI", "NO")
                        .test_bel_attr_bits_base_bi(bcls::MISC_SW::ZCLK_N_DIV2, i, val)
                        .global(opt, vname)
                        .commit()
                }
            }
            if edev.chip.kind.is_virtex2p() {
                bctx.build().test_global_attr_bool_rename(
                    "DISABLEVGGGENERATION",
                    bcls::MISC_SW::DISABLE_VGG_GENERATION,
                    "NO",
                    "YES",
                );
            }
        } else {
            for i in 0..4 {
                for (val, vname) in [(false, "0"), (true, "1")] {
                    let mut builder = bctx.build();
                    if edev.chip.kind.is_spartan3a() {
                        builder = builder.extra_fixed_bel_attr_bits_base_bi(
                            global,
                            bslots::GLOBAL,
                            bcls::GLOBAL::SEND_VGG,
                            i,
                            val,
                        );
                    }
                    builder
                        .test_bel_attr_bits_base_bi(bcls::MISC_SW::SEND_VGG, i, val)
                        .global(format!("SEND_VGG{i}"), vname)
                        .commit();
                }
            }
            for (attr, gattr, opt) in [
                (
                    bcls::MISC_SW::VGG_SENDMAX,
                    bcls::GLOBAL::VGG_SENDMAX,
                    "VGG_SENDMAX",
                ),
                (
                    bcls::MISC_SW::VGG_ENABLE_OFFCHIP,
                    bcls::GLOBAL::VGG_ENABLE_OFFCHIP,
                    "VGG_ENABLE_OFFCHIP",
                ),
            ] {
                for (val, vname) in [(false, "NO"), (true, "YES")] {
                    let mut builder = bctx.build();
                    if edev.chip.kind.is_spartan3a() {
                        builder = builder.extra_fixed_bel_attr_bits_bi(
                            global,
                            bslots::GLOBAL,
                            gattr,
                            val,
                        );
                    }
                    builder
                        .test_bel_attr_bits_bi(attr, val)
                        .global(opt, vname)
                        .commit();
                }
            }
        }
        if edev.chip.kind == ChipKind::Spartan3 {
            bctx.build().test_global_attr_bool_rename(
                "GATE_GHIGH",
                bcls::MISC_SW::GATE_GHIGH,
                "NO",
                "YES",
            );
            for (i, opt) in ["IDCI_OSC_SEL0", "IDCI_OSC_SEL1", "IDCI_OSC_SEL2"]
                .into_iter()
                .enumerate()
            {
                for (val, vname) in [(false, "0"), (true, "1")] {
                    bctx.build()
                        .test_bel_attr_bits_base_bi(bcls::MISC_SW::DCI_OSC_SEL, i, val)
                        .global(opt, vname)
                        .commit()
                }
            }
        }
        if edev.chip.kind.is_spartan3ea() {
            bctx.build()
                .test_global_attr_rename("TEMPSENSOR", bcls::MISC_SW::TEMP_SENSOR);
        }
        if edev.chip.kind.is_spartan3a() {
            test_pull(&mut bctx, bcls::MISC_SW::CCLK2_PULL, "CCLK2PIN");
            test_pull(&mut bctx, bcls::MISC_SW::MOSI2_PULL, "MOSI2PIN");
        } else if edev.chip.kind != ChipKind::Spartan3E && edev.chip.kind != ChipKind::FpgaCore {
            test_pull(&mut bctx, bcls::MISC_SW::M0_PULL, "M0PIN");
            test_pull(&mut bctx, bcls::MISC_SW::M1_PULL, "M1PIN");
            test_pull(&mut bctx, bcls::MISC_SW::M2_PULL, "M2PIN");
        }
        if edev.chip.kind.is_virtex2() {
            bctx.build()
                .prop(ForceBits(freeze_dci_btiles))
                .global_mutex("DCI", "FREEZE")
                .no_global("ENCRYPT")
                .test_bel_special(specials::FREEZE_DCI)
                .global("FREEZEDCI", "YES")
                .commit();
        }
    }

    // UL
    {
        let mut ctx = FuzzCtx::new(session, backend, cnr_nw);
        let mut bctx = ctx.bel(bslots::MISC_NW);
        if edev.chip.kind != ChipKind::FpgaCore {
            test_pullup(&mut bctx, bcls::MISC_NW::PROG_PULL, "PROGPIN");
            test_pull(&mut bctx, bcls::MISC_NW::TDI_PULL, "TDIPIN");
        }
        if edev.chip.kind.is_spartan3a() {
            test_pull(&mut bctx, bcls::MISC_NW::TMS_PULL, "TMSPIN");
        }
        if !edev.chip.kind.is_spartan3ea() && edev.chip.kind != ChipKind::FpgaCore {
            test_pull(&mut bctx, bcls::MISC_NW::HSWAPEN_PULL, "HSWAPENPIN");
        }
        for (val, vname) in [(false, "NO"), (true, "YES")] {
            let mut builder = bctx.build();
            if edev.chip.kind.is_virtex2() {
                let cnr_ne = edev.chip.corner(DirHV::NE);
                builder = builder.extra_fixed_bel_attr_bits_bi(
                    cnr_ne,
                    bslots::MISC_NE,
                    bcls::MISC_NE::TEST_LL,
                    val,
                );
            }
            builder
                .test_bel_attr_bits_bi(bcls::MISC_NW::TEST_LL, val)
                .global("TESTLL", vname)
                .commit();
        }

        let mut bctx = ctx.bel(bslots::PMV);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMV")
            .commit();
        if edev.chip.kind.is_spartan3a() {
            let mut bctx = ctx.bel(bslots::DNA_PORT);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("DNA_PORT")
                .commit();
        }
    }

    {
        // LR
        let mut ctx = FuzzCtx::new(session, backend, cnr_se);
        let mut bctx = ctx.bel(bslots::MISC_SE);
        if edev.chip.kind != ChipKind::FpgaCore {
            test_pullup(&mut bctx, bcls::MISC_SE::DONE_PULL, "DONEPIN");
        }
        if !edev.chip.kind.is_spartan3a() && edev.chip.kind != ChipKind::FpgaCore {
            test_pullup(&mut bctx, bcls::MISC_SE::CCLK_PULL, "CCLKPIN");
        }
        if edev.chip.kind.is_virtex2() {
            test_pullup(&mut bctx, bcls::MISC_SE::POWERDOWN_PULL, "POWERDOWNPIN");
        }
        if edev.chip.kind == ChipKind::FpgaCore {
            for (i, attr) in ["ABUFF0", "ABUFF1", "ABUFF2", "ABUFF3"]
                .into_iter()
                .enumerate()
            {
                for (val, vname) in [(false, "0"), (true, "1")] {
                    bctx.build()
                        .test_bel_attr_bits_base_bi(bcls::MISC_SE::ABUFF, i, val)
                        .global(attr, vname)
                        .commit();
                }
            }
        }

        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("STARTUP")
            .commit();
        bctx.mode("STARTUP")
            .null_bits()
            .prop(ExtraTileInt)
            .global("STARTUPCLK", "JTAGCLK")
            .test_bel_input_inv_auto(bcls::STARTUP::CLK);
        bctx.mode("STARTUP")
            .prop(ExtraTileInt)
            .no_pin("GSR")
            .test_bel_input_inv_auto(bcls::STARTUP::GTS);
        bctx.mode("STARTUP")
            .prop(ExtraTileInt)
            .no_pin("GTS")
            .test_bel_input_inv_auto(bcls::STARTUP::GSR);
        for (attr, aname) in [
            (bcls::STARTUP::GTS_SYNC, "GTS_SYNC"),
            (bcls::STARTUP::GSR_SYNC, "GSR_SYNC"),
            (bcls::STARTUP::GWE_SYNC, "GWE_SYNC"),
        ] {
            if !edev.chip.kind.is_virtex2() && attr == bcls::STARTUP::GWE_SYNC {
                continue;
            }
            bctx.mode("STARTUP")
                .test_global_attr_bool_rename(aname, attr, "NO", "YES");
        }
        if edev.chip.kind == ChipKind::Spartan3E {
            bctx.mode("STARTUP")
                .null_bits()
                .extra_fixed_bel_attr_bits(global, bslots::GLOBAL, bcls::GLOBAL::MULTIBOOT_ENABLE)
                .test_bel_special(specials::STARTUP_MULTIBOOT_ENABLE)
                .pin("MBT")
                .commit();
        }
        for (val, vname) in [
            (enums::STARTUP_CLOCK::CCLK, "CCLK"),
            (enums::STARTUP_CLOCK::USERCLK, "USERCLK"),
            (enums::STARTUP_CLOCK::JTAGCLK, "JTAGCLK"),
        ] {
            bctx.mode("STARTUP")
                .null_bits()
                .extra_fixed_bel_attr_val(global, bslots::GLOBAL, bcls::GLOBAL::STARTUP_CLOCK, val)
                .pin("CLK")
                .test_bel_special_val(specials::STARTUP_CLOCK, val)
                .global("STARTUPCLK", vname)
                .commit();
        }

        let mut bctx = ctx.bel(bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("CAPTURE")
            .commit();
        bctx.mode("CAPTURE")
            .null_bits()
            .prop(ExtraTileInt)
            .test_bel_input_inv_auto(bcls::CAPTURE::CLK);
        bctx.mode("CAPTURE")
            .null_bits()
            .prop(ExtraTileInt)
            .test_bel_input_inv_auto(bcls::CAPTURE::CAP);
        if edev.chip.kind.is_spartan3a() {
            for val in [false, true] {
                bctx.mode("CAPTURE")
                    .null_bits()
                    .extra_fixed_bel_attr_bits_bi(
                        global,
                        bslots::GLOBAL,
                        bcls::GLOBAL::CAPTURE_ONESHOT,
                        val,
                    )
                    .test_bel_special(specials::CAPTURE_ONESHOT)
                    .attr("ONESHOT", if val { "TRUE" } else { "FALSE" })
                    .commit();
            }
        } else {
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_fixed_bel_attr_bits(global, bslots::GLOBAL, bcls::GLOBAL::CAPTURE_ONESHOT)
                .test_bel_special(specials::CAPTURE_ONESHOT)
                .attr("ONESHOT_ATTR", "ONE_SHOT")
                .commit();
        }

        let mut bctx = ctx.bel(bslots::ICAP);
        if edev.chip.kind.is_spartan3a() {
            bctx.build()
                .null_bits()
                .extra_fixed_bel_attr_bits(global, bslots::GLOBAL, bcls::GLOBAL::ICAP_ENABLE)
                .test_bel_attr_bits(bcls::ICAP::ENABLE)
                .mode("ICAP")
                .commit();
        } else if edev.chip.kind == ChipKind::Spartan3E {
            bctx.build()
                .null_bits()
                .test_bel_attr_bits(bcls::ICAP::ENABLE)
                .mode("ICAP")
                .commit();
        } else {
            bctx.build()
                .test_bel_attr_bits(bcls::ICAP::ENABLE)
                .mode("ICAP")
                .commit();
        }
        if edev.chip.kind == ChipKind::Spartan3E {
            bctx.mode("ICAP")
                .null_bits()
                .test_bel_input_inv_auto(bcls::ICAP::CLK);
            bctx.mode("ICAP")
                .null_bits()
                .test_bel_input_inv_auto(bcls::ICAP::CE);
            bctx.mode("ICAP")
                .null_bits()
                .test_bel_input_inv_auto(bcls::ICAP::WRITE);
        } else {
            bctx.mode("ICAP")
                .null_bits()
                .prop(ExtraTileInt)
                .test_bel_input_inv_auto(bcls::ICAP::CLK);
            bctx.mode("ICAP")
                .null_bits()
                .prop(ExtraTileInt)
                .test_bel_input_inv_auto(bcls::ICAP::CE);
            bctx.mode("ICAP")
                .null_bits()
                .prop(ExtraTileInt)
                .test_bel_input_inv_auto(bcls::ICAP::WRITE);
        }

        if edev.chip.kind.is_spartan3a() {
            let mut bctx = ctx.bel(bslots::SPI_ACCESS);
            bctx.build()
                .prop(ExtraTileInt)
                .test_bel_attr_bits(bcls::SPI_ACCESS::ENABLE)
                .mode("SPI_ACCESS")
                .commit();
        }
    }

    {
        // UR
        let mut ctx = FuzzCtx::new(session, backend, cnr_ne);
        let mut bctx = ctx.bel(bslots::MISC_NE);
        if edev.chip.kind != ChipKind::FpgaCore {
            test_pull(&mut bctx, bcls::MISC_NE::TCK_PULL, "TCKPIN");
            test_pull(&mut bctx, bcls::MISC_NE::TDO_PULL, "TDOPIN");
            if !edev.chip.kind.is_spartan3a() {
                test_pull(&mut bctx, bcls::MISC_NE::TMS_PULL, "TMSPIN");
            } else {
                test_pull(&mut bctx, bcls::MISC_NE::MISO2_PULL, "MISO2PIN");
                test_pull(&mut bctx, bcls::MISC_NE::CSO2_PULL, "CSO2PIN");
            }
        }
        let mut bctx = ctx.bel(bslots::BSCAN);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("BSCAN")
            .commit();
        bctx.build()
            .test_bel_attr_bits(bcls::BSCAN::USERCODE)
            .multi_global("USERID", MultiValue::HexPrefix, 32);
        bctx.mode("BSCAN")
            .no_pin("TDO2")
            .test_bel_attr_bits(bcls::BSCAN::USER_TDO_ENABLE)
            .pin("TDO1")
            .pin_int_pips_input(bcls::BSCAN::TDO1)
            .commit();
        bctx.mode("BSCAN")
            .no_pin("TDO1")
            .test_bel_attr_bits(bcls::BSCAN::USER_TDO_ENABLE)
            .pin("TDO2")
            .pin_int_pips_input(bcls::BSCAN::TDO2)
            .commit();
        if edev.chip.kind.is_virtex2p() {
            let mut bctx = ctx.bel(bslots::JTAGPPC);
            bctx.build()
                .test_bel_attr_bits(bcls::JTAGPPC::ENABLE)
                .mode("JTAGPPC")
                .commit();
        }
    }

    if edev.chip.kind == ChipKind::FpgaCore {
        let mut ctx = FuzzCtx::new_null(session, backend);
        let cnr_ll = edev.chip.corner(DirHV::SW);
        let cnr_ul = edev.chip.corner(DirHV::NW);
        let cnr_lr = edev.chip.corner(DirHV::SE);
        let cnr_ur = edev.chip.corner(DirHV::NE);
        let int_ll = edev.chip.corner(DirHV::SW).cell.tile(defs::tslots::INT);
        let int_ul = edev.chip.corner(DirHV::NW).cell.tile(defs::tslots::INT);
        let int_lr = edev.chip.corner(DirHV::SE).cell.tile(defs::tslots::INT);
        let int_ur = edev.chip.corner(DirHV::NE).cell.tile(defs::tslots::INT);
        for val in [false, true] {
            ctx.build()
                .extra_fixed_bel_attr_bits_bi(
                    cnr_ll,
                    bslots::MISR_FC,
                    bcls::MISR_FC::MISR_RESET,
                    val,
                )
                .extra_fixed_bel_attr_bits_bi(
                    cnr_ul,
                    bslots::MISR_FC,
                    bcls::MISR_FC::MISR_RESET,
                    val,
                )
                .extra_fixed_bel_attr_bits_bi(
                    cnr_lr,
                    bslots::MISR_FC,
                    bcls::MISR_FC::MISR_RESET,
                    val,
                )
                .extra_fixed_bel_attr_bits_bi(
                    cnr_ur,
                    bslots::MISR_FC,
                    bcls::MISR_FC::MISR_RESET,
                    val,
                )
                .test_global_special(specials::MISR_RESET)
                .global("MISRRESET", if val { "YES" } else { "NO" })
                .commit();
        }
        ctx.build()
            .global_mutex("MISR_CLOCK", "YUP")
            .extra_fixed_bel_attr_bits(cnr_ll, bslots::MISR_FC, bcls::MISR_FC::MISR_CLOCK)
            .extra_fixed_bel_attr_bits(cnr_ul, bslots::MISR_FC, bcls::MISR_FC::MISR_CLOCK)
            .extra_fixed_bel_attr_bits(cnr_lr, bslots::MISR_FC, bcls::MISR_FC::MISR_CLOCK)
            .extra_fixed_bel_attr_bits(cnr_ur, bslots::MISR_FC, bcls::MISR_FC::MISR_CLOCK)
            .extra_fixed_bel_special(int_ll, bslots::MISR_FC, specials::MISR_CLOCK_GCLK0)
            .extra_fixed_bel_special(int_ul, bslots::MISR_FC, specials::MISR_CLOCK_GCLK0)
            .extra_fixed_bel_special(int_lr, bslots::MISR_FC, specials::MISR_CLOCK_GCLK0)
            .extra_fixed_bel_special(int_ur, bslots::MISR_FC, specials::MISR_CLOCK_GCLK0)
            .extra_tiles_by_bel_special(bslots::HROW, specials::MISR_CLOCK_GCLK0)
            .extra_tiles_by_bel_special(bslots::HCLK, specials::MISR_CLOCK_GCLK0)
            .test_global_special(specials::MISR_CLOCK_GCLK0)
            .global("MISRCLOCK", "GCLK0")
            .commit();
    }

    // I/O bank misc control
    if !skip_io && edev.chip.kind != ChipKind::FpgaCore {
        let package = backend
            .device
            .bonds
            .values()
            .max_by_key(|bond| {
                let bdata = &backend.db.bonds[bond.bond];
                let prjcombine_re_xilinx_geom::Bond::Virtex2(bdata) = bdata else {
                    unreachable!();
                };
                bdata.pins.len()
            })
            .unwrap();
        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[&package.name] else {
            unreachable!()
        };
        if !edev.chip.kind.is_spartan3ea() {
            for (dir, tile_name, bidx, bank) in [
                (DirHV::NW, cnr_nw, 0, 7),
                (DirHV::NW, cnr_nw, 1, 0),
                (DirHV::NE, cnr_ne, 1, 1),
                (DirHV::NE, cnr_ne, 0, 2),
                (DirHV::SE, cnr_se, 0, 3),
                (DirHV::SE, cnr_se, 1, 4),
                (DirHV::SW, cnr_sw, 1, 5),
                (DirHV::SW, cnr_sw, 0, 6),
            ] {
                let mut ctx = FuzzCtx::new(session, backend, tile_name);
                let mut bctx = ctx.bel(bslots::DCI[bidx]);

                let mut btiles =
                    EntityVec::from_iter([edev.btile_term_h(edev.chip.corner(dir).cell)]);
                if edev.chip.kind.is_virtex2() {
                    btiles.push(edev.btile_term_v(edev.chip.corner(dir).cell));
                }
                let mut site = None;
                let mut site_other = None;
                let mut coords = HashSet::new();
                let other_bank = if bank == 4 { 5 } else { 4 };
                let mut io_vr = None;
                if let Some(&(vrp, vrn)) = edev.chip.dci_io.get(&bank)
                    && ebond.ios.contains_key(&vrp)
                    && ebond.ios.contains_key(&vrn)
                {
                    io_vr = Some((vrp, vrn));
                }
                if io_vr.is_none() {
                    io_vr = Some(edev.chip.dci_io_alt[&bank]);
                }
                let (io_vrp, io_vrn) = io_vr.unwrap();
                let site_vrp = endev.get_io_name(io_vrp);
                let site_vrn = endev.get_io_name(io_vrn);
                for io in edev.chip.get_bonded_ios().into_iter().rev() {
                    let ioinfo = edev.chip.get_io_info(io);
                    let bcrd = edev.chip.get_io_loc(io);
                    if ioinfo.bank == bank && coords.insert(bcrd.cell) {
                        btiles.push(edev.btile_main(bcrd.cell));
                        if bcrd.cell.col == edev.chip.col_w() || bcrd.cell.col == edev.chip.col_e()
                        {
                            btiles.push(edev.btile_term_h(bcrd.cell));
                        } else {
                            btiles.push(edev.btile_term_v(bcrd.cell));
                        }
                    }
                    if ebond.ios.contains_key(&io)
                        && matches!(ioinfo.diff, IoDiffKind::P(_))
                        && ioinfo.pad_kind == Some(IobKind::Iob)
                        && io != io_vrp
                        && io != io_vrn
                    {
                        if ioinfo.bank == bank && site.is_none() {
                            site = Some(endev.get_io_name(io));
                        }
                        if ioinfo.bank == other_bank && site_other.is_none() {
                            site_other = Some(endev.get_io_name(io));
                        }
                    }
                }
                let site = site.unwrap();
                let site_other = site_other.unwrap();
                for std in get_iostds(edev, false) {
                    let rid = iostd_to_row(edev, &std);
                    if std.diff == DiffKind::True {
                        bctx.build()
                            .prop(ForceBits(btiles.clone()))
                            .raw(Key::Package, package.name.clone())
                            .global_mutex("DIFF", "BANK")
                            .global_mutex("VREF", "NO")
                            .global_mutex("DCI", "YES")
                            .test_bel_attr_row(
                                if edev.chip.kind.is_virtex2() {
                                    bcls::DCI::V2_LVDSBIAS
                                } else {
                                    bcls::DCI::S3_LVDSBIAS
                                },
                                rid,
                            )
                            .raw_diff(Key::SiteMode(site), None, "DIFFM")
                            .raw_diff(Key::SiteAttr(site, "OMUX".into()), None, "O1")
                            .raw_diff(Key::SiteAttr(site, "O1INV".into()), None, "O1")
                            .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), None, std.name)
                            .raw_diff(Key::SitePin(site, "O1".into()), None, true)
                            .commit();
                    }
                    if matches!(
                        std.dci,
                        DciKind::InputSplit | DciKind::BiSplit | DciKind::InputVcc | DciKind::BiVcc
                    ) && std.diff == DiffKind::None
                    {
                        bctx.build()
                            .prop(ForceBits(btiles.clone()))
                            .raw(Key::Package, package.name.clone())
                            .global_mutex("VREF", "NO")
                            .global_mutex("DCI", "BANK_TERM")
                            .raw(Key::SiteMode(site_other), "IOB")
                            .raw(Key::SiteAttr(site_other, "OMUX".into()), "O1")
                            .raw(Key::SiteAttr(site_other, "O1INV".into()), "O1")
                            .raw(Key::SiteAttr(site_other, "IOATTRBOX".into()), "LVDCI_33")
                            .raw(Key::SitePin(site_other, "O1".into()), true)
                            .raw(Key::SiteMode(site_vrp), None)
                            .raw(Key::SiteMode(site_vrn), None)
                            .raw(Key::SiteAttr(site, "IMUX".into()), "1")
                            .raw(Key::SitePin(site, "I".into()), true)
                            .test_bel_special_row(specials::DCI_TERM, rid)
                            .raw_diff(Key::SiteMode(site), "IOB", "IOB")
                            .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), "GTL", std.name)
                            .commit();
                    }
                }
                if edev.chip.kind == ChipKind::Spartan3 {
                    for (spec, val) in [
                        (specials::DCI_ASREQUIRED, "ASREQUIRED"),
                        (specials::DCI_CONTINUOUS, "CONTINUOUS"),
                        (specials::DCI_QUIET, "QUIET"),
                    ] {
                        bctx.build()
                            .prop(ForceBits(btiles.clone()))
                            .raw(Key::Package, package.name.clone())
                            .global_mutex("VREF", "NO")
                            .global_mutex("DCI", "BANK")
                            .raw(Key::SiteMode(site_other), "IOB")
                            .raw(Key::SiteAttr(site_other, "OMUX".into()), "O1")
                            .raw(Key::SiteAttr(site_other, "O1INV".into()), "O1")
                            .raw(Key::SiteAttr(site_other, "IOATTRBOX".into()), "LVDCI_33")
                            .raw(Key::SitePin(site_other, "O1".into()), true)
                            .raw(Key::SiteMode(site_vrp), None)
                            .raw(Key::SiteMode(site_vrn), None)
                            .global("DCIUPDATEMODE", val)
                            .test_bel_special(spec)
                            .raw_diff(Key::SiteMode(site), None, "IOB")
                            .raw_diff(Key::SiteAttr(site, "OMUX".into()), None, "O1")
                            .raw_diff(Key::SiteAttr(site, "O1INV".into()), None, "O1")
                            .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), None, "LVDCI_33")
                            .raw_diff(Key::SitePin(site, "O1".into()), None, true)
                            .commit();
                    }
                } else {
                    bctx.build()
                        .prop(ForceBits(btiles.clone()))
                        .raw(Key::Package, package.name.clone())
                        .global_mutex("VREF", "NO")
                        .global_mutex("DCI", "BANK")
                        .raw(Key::SiteMode(site_other), "IOB")
                        .raw(Key::SiteAttr(site_other, "OMUX".into()), "O1")
                        .raw(Key::SiteAttr(site_other, "O1INV".into()), "O1")
                        .raw(Key::SiteAttr(site_other, "IOATTRBOX".into()), "LVDCI_33")
                        .raw(Key::SitePin(site_other, "O1".into()), true)
                        .raw(Key::SiteMode(site_vrp), None)
                        .raw(Key::SiteMode(site_vrn), None)
                        .test_bel_special(specials::DCI_OUT)
                        .raw_diff(Key::SiteMode(site), None, "IOB")
                        .raw_diff(Key::SiteAttr(site, "OMUX".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site, "O1INV".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), None, "LVDCI_33")
                        .raw_diff(Key::SitePin(site, "O1".into()), None, true)
                        .commit();
                }
                if bank == 6 {
                    let mut builder = bctx
                        .build()
                        .prop(ForceBits(btiles.clone()))
                        .raw(Key::Package, package.name.clone())
                        .global_mutex("VREF", "NO")
                        .global_mutex("DCI", "GLOBAL")
                        .global("MATCH_CYCLE", "NOWAIT")
                        .raw(Key::SiteMode(site_vrp), None)
                        .raw(Key::SiteMode(site_vrn), None);
                    if edev.chip.kind != ChipKind::Spartan3 {
                        builder = builder.global("FREEZEDCI", "NO");
                    }
                    builder
                        .test_bel_special(specials::DCI_OUT_ALONE)
                        .raw_diff(Key::SiteMode(site), None, "IOB")
                        .raw_diff(Key::SiteAttr(site, "OMUX".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site, "O1INV".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), None, "LVDCI_33")
                        .raw_diff(Key::SitePin(site, "O1".into()), None, true)
                        .commit();
                } else if bank == 5 && edev.chip.dci_io_alt.contains_key(&5) {
                    let (io_alt_vrp, io_alt_vrn) = edev.chip.dci_io_alt[&5];
                    let site_alt_vrp = endev.get_io_name(io_alt_vrp);
                    let site_alt_vrn = endev.get_io_name(io_alt_vrn);
                    let mut builder = bctx
                        .build()
                        .prop(ForceBits(btiles.clone()))
                        .raw(Key::Package, package.name.clone())
                        .raw(Key::AltVr, true)
                        .global_mutex("VREF", "NO")
                        .global_mutex("DCI", "GLOBAL_ALT")
                        .global("MATCH_CYCLE", "NOWAIT");
                    if site != site_alt_vrp {
                        builder = builder.raw(Key::SiteMode(site_alt_vrp), None);
                    }
                    if site != site_alt_vrn {
                        builder = builder.raw(Key::SiteMode(site_alt_vrn), None);
                    }
                    builder
                        .test_bel_special(specials::DCI_OUT_ALONE)
                        .raw_diff(Key::SiteMode(site), None, "IOB")
                        .raw_diff(Key::SiteAttr(site, "OMUX".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site, "O1INV".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), None, "LVDCI_33")
                        .raw_diff(Key::SitePin(site, "O1".into()), None, true)
                        .commit();
                }
                if edev.chip.kind == ChipKind::Spartan3 {
                    bctx.build()
                        .global_mutex("DCI", "PRESENT")
                        .test_bel_special(specials::PRESENT)
                        .mode("DCI")
                        .commit();
                    bctx.build()
                        .global_mutex("DCI", "PRESENT")
                        .global_mutex("DCI_SELECT", format!("DCI{bidx}"))
                        .mode("DCI")
                        .test_bel_special(specials::DCI_SELECT)
                        .pip((PinFar, "DATA"), "DATA")
                        .commit();
                    for i in 0..13 {
                        let gname = format!("LVDSBIAS_OPT{i}_{bank}");
                        bctx.build()
                            .global_mutex("DIFF", "MANUAL")
                            .test_bel_attr_bits_base(bcls::DCI::S3_LVDSBIAS, i)
                            .global_diff(gname, "0", "1")
                            .commit();
                    }
                } else {
                    bctx.build()
                        .global_mutex("DCI", "PRESENT")
                        .test_bel_special(specials::PRESENT)
                        .mode("DCI")
                        .commit();
                    bctx.build()
                        .global_mutex("DCI", "PRESENT_TEST")
                        .global("TESTDCI", "YES")
                        .test_bel_special(specials::DCI_TEST)
                        .mode("DCI")
                        .commit();
                }
                // ???
                bctx.mode("DCI")
                    .global_mutex("DCI", "PRESENT")
                    .test_bel_attr_bits_bi(bcls::DCI::FORCE_DONE_HIGH, false)
                    .attr("FORCE_DONE_HIGH", "#OFF")
                    .commit();
            }

            if edev.chip.kind.is_virtex2p()
                && !backend.device.name.ends_with("2vp4")
                && !backend.device.name.ends_with("2vp7")
            {
                let mut ctx = FuzzCtx::new(session, backend, cnr_sw);
                let btiles = EntityVec::from_iter([
                    edev.btile_term_v(edev.chip.corner(DirHV::NW).cell),
                    edev.btile_term_v(edev.chip.corner(DirHV::NE).cell),
                    edev.btile_term_h(edev.chip.corner(DirHV::NE).cell),
                    edev.btile_term_h(edev.chip.corner(DirHV::SE).cell),
                    edev.btile_term_v(edev.chip.corner(DirHV::SE).cell),
                    edev.btile_term_v(edev.chip.corner(DirHV::SW).cell),
                    edev.btile_term_h(edev.chip.corner(DirHV::SW).cell),
                    edev.btile_term_h(edev.chip.corner(DirHV::NW).cell),
                ]);
                for (spec, val) in [
                    (specials::DCI_ASREQUIRED, "ASREQUIRED"),
                    (specials::DCI_CONTINUOUS, "CONTINUOUS"),
                ] {
                    ctx.build()
                        .null_bits()
                        .global_mutex("DCI", "GLOBAL_MODE")
                        .test_raw(DiffKey::GlobalSpecial(spec))
                        .global("DCIUPDATEMODE", val)
                        .commit();
                }
                ctx.build()
                    .global_mutex("DCI", "GLOBAL_MODE")
                    .prop(ForceBits(btiles.clone()))
                    .test_raw(DiffKey::GlobalSpecial(specials::DCI_QUIET))
                    .global("DCIUPDATEMODE", "QUIET")
                    .commit();
            }
        } else {
            let banks = if edev.chip.kind == ChipKind::Spartan3E {
                &[
                    (
                        cnr_nw,
                        edev.btile_term_h(edev.chip.corner(DirHV::NW).cell),
                        0,
                    ),
                    (
                        cnr_ne,
                        edev.btile_term_h(edev.chip.corner(DirHV::NE).cell),
                        1,
                    ),
                    (
                        cnr_se,
                        edev.btile_term_h(edev.chip.corner(DirHV::SE).cell),
                        2,
                    ),
                    (
                        cnr_sw,
                        edev.btile_term_h(edev.chip.corner(DirHV::SW).cell),
                        3,
                    ),
                ][..]
            } else {
                &[
                    (
                        cnr_nw,
                        edev.btile_term_h(edev.chip.corner(DirHV::NW).cell),
                        0,
                    ),
                    (
                        cnr_sw,
                        edev.btile_term_h(edev.chip.corner(DirHV::SW).cell),
                        2,
                    ),
                ][..]
            };
            for &(tile_name, btile, bank) in banks {
                let mut ctx = FuzzCtx::new(session, backend, tile_name);
                let mut bctx = ctx.bel(bslots::BANK);
                let mut btiles = EntityVec::from_iter([btile]);
                match bank {
                    0 => {
                        for cell in edev.row(Chip::DIE, edev.chip.row_n()) {
                            if cell.col != edev.chip.col_w() && cell.col != edev.chip.col_e() {
                                btiles.push(edev.btile_main(cell));
                                btiles.push(edev.btile_term_v(cell));
                            }
                        }
                    }
                    1 => {
                        for cell in edev.column(Chip::DIE, edev.chip.col_e()) {
                            if cell.row != edev.chip.row_s() && cell.row != edev.chip.row_n() {
                                btiles.push(edev.btile_main(cell));
                                btiles.push(edev.btile_term_h(cell));
                            }
                        }
                    }
                    2 => {
                        for cell in edev.row(Chip::DIE, edev.chip.row_s()) {
                            if cell.col != edev.chip.col_w() && cell.col != edev.chip.col_e() {
                                btiles.push(edev.btile_main(cell));
                                btiles.push(edev.btile_term_v(cell));
                            }
                        }
                    }
                    3 => {
                        for cell in edev.column(Chip::DIE, edev.chip.col_w()) {
                            if cell.row != edev.chip.row_s() && cell.row != edev.chip.row_n() {
                                btiles.push(edev.btile_main(cell));
                                btiles.push(edev.btile_term_h(cell));
                            }
                        }
                    }
                    _ => unreachable!(),
                }
                let mut ios = vec![];
                for io in edev.chip.get_bonded_ios().into_iter().rev() {
                    let ioinfo = edev.chip.get_io_info(io);
                    if ebond.ios.contains_key(&io)
                        && matches!(ioinfo.diff, IoDiffKind::P(_))
                        && ioinfo.pad_kind == Some(IobKind::Iob)
                        && ioinfo.bank == bank
                    {
                        ios.push(io)
                    }
                }
                assert!(ios.len() >= 2);
                if edev.chip.kind == ChipKind::Spartan3ADsp {
                    ios.reverse();
                }
                let site_a = endev.get_io_name(ios[0]);
                let site_b = endev.get_io_name(ios[1]);
                let diffm = if edev.chip.kind == ChipKind::Spartan3E {
                    "DIFFM"
                } else {
                    "DIFFMTB"
                };
                for std in get_iostds(edev, false) {
                    let rid = iostd_to_row(edev, &std);
                    if std.diff != DiffKind::True {
                        continue;
                    }
                    if std.name != "LVDS_25" || edev.chip.kind.is_spartan3a() {
                        bctx.build()
                            .prop(ForceBits(btiles.clone()))
                            .raw(Key::Package, package.name.clone())
                            .global_mutex("DIFF", "BANK")
                            .global_mutex("VREF", "NO")
                            .test_bel_special_row(specials::BANK_LVDSBIAS_0, rid)
                            .raw_diff(Key::SiteMode(site_a), None, diffm)
                            .raw_diff(Key::SiteAttr(site_a, "OMUX".into()), None, "O1")
                            .raw_diff(Key::SiteAttr(site_a, "O1INV".into()), None, "O1")
                            .raw_diff(Key::SiteAttr(site_a, "IOATTRBOX".into()), None, std.name)
                            .raw_diff(
                                Key::SiteAttr(site_a, "SUSPEND".into()),
                                None,
                                if edev.chip.kind.is_spartan3a() {
                                    "3STATE"
                                } else {
                                    ""
                                },
                            )
                            .raw_diff(Key::SitePin(site_a, "O1".into()), None, true)
                            .commit();
                    }
                    let alt_std = if std.name == "RSDS_25" {
                        "MINI_LVDS_25"
                    } else {
                        "RSDS_25"
                    };
                    bctx.build()
                        .prop(ForceBits(btiles.clone()))
                        .raw(Key::Package, package.name.clone())
                        .global_mutex("DIFF", "BANK")
                        .raw(Key::SiteMode(site_a), diffm)
                        .raw(Key::SiteAttr(site_a, "OMUX".into()), "O1")
                        .raw(Key::SiteAttr(site_a, "O1INV".into()), "O1")
                        .raw(Key::SiteAttr(site_a, "IOATTRBOX".into()), alt_std)
                        .raw(
                            Key::SiteAttr(site_a, "SUSPEND".into()),
                            if edev.chip.kind.is_spartan3a() {
                                "3STATE"
                            } else {
                                ""
                            },
                        )
                        .raw(Key::SitePin(site_a, "O1".into()), true)
                        .test_bel_special_row(specials::BANK_LVDSBIAS_1, rid)
                        .raw_diff(Key::SiteMode(site_b), None, diffm)
                        .raw_diff(Key::SiteAttr(site_b, "OMUX".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site_b, "O1INV".into()), None, "O1")
                        .raw_diff(Key::SiteAttr(site_b, "IOATTRBOX".into()), None, std.name)
                        .raw_diff(
                            Key::SiteAttr(site_b, "SUSPEND".into()),
                            None,
                            if edev.chip.kind.is_spartan3a() {
                                "3STATE"
                            } else {
                                ""
                            },
                        )
                        .raw_diff(Key::SitePin(site_b, "O1".into()), None, true)
                        .commit();
                }
            }
        }
    }

    // config regs
    let tcid = if edev.chip.kind.is_virtex2() {
        tcls_v2::GLOBAL
    } else if edev.chip.kind == ChipKind::Spartan3 {
        tcls_s3::GLOBAL_S3
    } else if edev.chip.kind == ChipKind::FpgaCore {
        tcls_s3::GLOBAL_FC
    } else if edev.chip.kind == ChipKind::Spartan3E {
        tcls_s3::GLOBAL_S3E
    } else {
        tcls_s3::GLOBAL_S3A
    };
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    let mut bctx = ctx.bel(bslots::GLOBAL);

    for (val, vname) in [
        (enums::STARTUP_CYCLE::_1, "1"),
        (enums::STARTUP_CYCLE::_2, "2"),
        (enums::STARTUP_CYCLE::_3, "3"),
        (enums::STARTUP_CYCLE::_4, "4"),
        (enums::STARTUP_CYCLE::_5, "5"),
        (enums::STARTUP_CYCLE::_6, "6"),
        (enums::STARTUP_CYCLE::DONE, "DONE"),
        (enums::STARTUP_CYCLE::KEEP, "KEEP"),
    ] {
        bctx.build()
            .test_bel_attr_val(bcls::GLOBAL::GWE_CYCLE, val)
            .global("GWE_CYCLE", vname)
            .commit();
        bctx.build()
            .test_bel_attr_val(bcls::GLOBAL::GTS_CYCLE, val)
            .global("GTS_CYCLE", vname)
            .commit();
        if val != enums::STARTUP_CYCLE::DONE
            && (val != enums::STARTUP_CYCLE::KEEP || !edev.chip.kind.is_spartan3a())
        {
            bctx.build()
                .test_bel_attr_val(bcls::GLOBAL::DONE_CYCLE, val)
                .global("DONE_CYCLE", vname)
                .commit();
        }
    }
    for (val, vname) in [
        (enums::STARTUP_CYCLE::_0, "0"),
        (enums::STARTUP_CYCLE::_1, "1"),
        (enums::STARTUP_CYCLE::_2, "2"),
        (enums::STARTUP_CYCLE::_3, "3"),
        (enums::STARTUP_CYCLE::_4, "4"),
        (enums::STARTUP_CYCLE::_5, "5"),
        (enums::STARTUP_CYCLE::_6, "6"),
        (enums::STARTUP_CYCLE::NOWAIT, "NOWAIT"),
    ] {
        if edev.chip.kind.is_spartan3a() && val == enums::STARTUP_CYCLE::_0 {
            continue;
        }
        if edev.chip.kind != ChipKind::FpgaCore {
            bctx.build()
                .test_bel_attr_val(bcls::GLOBAL::LOCK_CYCLE, val)
                .global("LCK_CYCLE", vname)
                .commit();
        }
        if !edev.chip.kind.is_spartan3ea() && edev.chip.kind != ChipKind::FpgaCore {
            // option is accepted on S3E, but doesn't do anything
            bctx.build()
                .global_mutex("DCI", "NO")
                .test_bel_attr_val(bcls::GLOBAL::MATCH_CYCLE, val)
                .global("MATCH_CYCLE", vname)
                .commit();
        }
    }
    bctx.build()
        .test_global_attr_bool_rename("DRIVEDONE", bcls::GLOBAL::DRIVE_DONE, "NO", "YES");
    bctx.build()
        .test_global_attr_bool_rename("DONEPIPE", bcls::GLOBAL::DONE_PIPE, "NO", "YES");
    if edev.chip.kind.is_spartan3a() {
        bctx.build().test_global_attr_bool_rename(
            "DRIVE_AWAKE",
            bcls::GLOBAL::DRIVE_AWAKE,
            "NO",
            "YES",
        );
    }
    if edev.chip.kind != ChipKind::FpgaCore && !edev.chip.kind.is_spartan3a() {
        bctx.build().test_global_attr_bool_rename(
            "DCMSHUTDOWN",
            bcls::GLOBAL::DCM_SHUTDOWN,
            "DISABLE",
            "ENABLE",
        );
    }

    if edev.chip.kind.is_virtex2() {
        bctx.build()
            .null_bits()
            .test_bel_special(specials::DCI_SHUTDOWN)
            .global("DCISHUTDOWN", "ENABLE")
            .commit();
        bctx.build()
            .null_bits()
            .test_bel_special(specials::DCI_SHUTDOWN)
            .global("DCISHUTDOWN", "DISABLE")
            .commit();
        bctx.build().test_global_attr_bool_rename(
            "POWERDOWNSTATUS",
            bcls::GLOBAL::POWERDOWN_STATUS,
            "DISABLE",
            "ENABLE",
        );
    }
    bctx.build()
        .test_global_attr_bool_rename("CRC", bcls::GLOBAL::CRC_ENABLE, "DISABLE", "ENABLE");

    if edev.chip.kind.is_virtex2() {
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", bcls::GLOBAL::CONFIG_RATE_V2);
    } else if !edev.chip.kind.is_spartan3ea() {
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", bcls::GLOBAL::CONFIG_RATE_S3);
    } else if edev.chip.kind == ChipKind::Spartan3E {
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", bcls::GLOBAL::CONFIG_RATE_S3E);
    }

    if !edev.chip.kind.is_virtex2() && !edev.chip.kind.is_spartan3a() {
        bctx.build()
            .test_global_attr_rename("BUSCLKFREQ", bcls::GLOBAL::BUSCLK_FREQ);

        if !edev.chip.kind.is_spartan3ea() {
            bctx.build()
                .test_global_attr_rename("VRDSEL", bcls::GLOBAL::S3_VRDSEL);
        } else {
            bctx.build()
                .test_global_attr_rename("VRDSEL", bcls::GLOBAL::S3E_VRDSEL);
        }
    }
    if edev.chip.kind.is_spartan3a() {
        bctx.build()
            .test_bel_attr_bits(bcls::GLOBAL::S3A_VRDSEL)
            .multi_global("VRDSEL", MultiValue::Bin, 3);
        bctx.build()
            .test_global_attr_bool_rename("BPI_DIV8", bcls::GLOBAL::BPI_DIV8, "NO", "YES");
        bctx.build().test_global_attr_bool_rename(
            "RESET_ON_ERR",
            bcls::GLOBAL::RESET_ON_ERR,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "ICAP_BYPASS",
            bcls::GLOBAL::ICAP_BYPASS,
            "NO",
            "YES",
        );
    }

    bctx.build()
        .test_global_attr_bool_rename("GTS_USR_B", bcls::GLOBAL::GTS_USR_B, "NO", "YES");
    bctx.build()
        .test_global_attr_bool_rename("VGG_TEST", bcls::GLOBAL::VGG_TEST, "NO", "YES");
    if !edev.chip.kind.is_spartan3a() {
        bctx.build().test_global_attr_bool_rename(
            "BCLK_TEST",
            bcls::GLOBAL::BCLK_TEST,
            "NO",
            "YES",
        );
        for (val, vname) in [
            (enums::SECURITY::NONE, "NONE"),
            (enums::SECURITY::LEVEL1, "LEVEL1"),
            (enums::SECURITY::LEVEL2, "LEVEL2"),
        ] {
            // disables FreezeDCI?
            if edev.chip.kind == ChipKind::Virtex2 {
                bctx.build()
                    .global_mutex("DCI", "NO")
                    .global("EARLYGHIGH", "YES")
                    .test_bel_attr_val(bcls::GLOBAL::SECURITY, val)
                    .global("SECURITY", vname)
                    .commit();
            } else {
                bctx.build()
                    .global_mutex("DCI", "NO")
                    .test_bel_attr_val(bcls::GLOBAL::SECURITY, val)
                    .global("SECURITY", vname)
                    .commit();
            }
        }

        // persist not fuzzed — too much effort

        if edev.chip.kind.is_virtex2() {
            bctx.build()
                .null_bits()
                .global_mutex("DCI", "NO")
                .test_bel_special(specials::ENCRYPT)
                .global("ENCRYPT", "YES")
                .commit();
        }
    } else {
        bctx.build().test_global_attr_bool_rename(
            "MULTIBOOTMODE",
            bcls::GLOBAL::MULTIBOOT_ENABLE,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_rename("SECURITY", bcls::GLOBAL::SECURITY);

        // CONFIGRATE too annoying.
        for val in 0..4 {
            bctx.build()
                .test_bel_attr_bitvec_u32(bcls::GLOBAL::CCLK_DLY, val)
                .global("CCLK_DLY", val.to_string())
                .commit();
            bctx.build()
                .test_bel_attr_bitvec_u32(bcls::GLOBAL::CCLK_SEP, val)
                .global("CCLK_SEP", val.to_string())
                .commit();
            bctx.build()
                .test_bel_attr_bitvec_u32(bcls::GLOBAL::CLK_SWITCH_OPT, val)
                .global("CLK_SWITCH_OPT", val.to_string())
                .commit();
        }

        bctx.build().test_global_attr_bool_rename(
            "BRAM_SKIP",
            bcls::GLOBAL::BRAM_SKIP,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "TWO_ROUND",
            bcls::GLOBAL::TWO_ROUND,
            "NO",
            "YES",
        );

        for val in 1..16 {
            bctx.build()
                .test_bel_attr_bitvec_u32(bcls::GLOBAL::HC_CYCLE, val)
                .global("HC_CYCLE", val.to_string())
                .commit();
        }

        bctx.build()
            .test_global_attr_rename("SW_CLK", bcls::GLOBAL::SW_CLK);
        bctx.build().test_global_attr_bool_rename(
            "EN_SUSPEND",
            bcls::GLOBAL::EN_SUSPEND,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_bool_rename("EN_PORB", bcls::GLOBAL::EN_PORB, "NO", "YES");
        bctx.build().test_global_attr_bool_rename(
            "SUSPEND_FILTER",
            bcls::GLOBAL::SUSPEND_FILTER,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "EN_SW_GSR",
            bcls::GLOBAL::EN_SW_GSR,
            "NO",
            "YES",
        );
        for val in 1..8 {
            bctx.build()
                .test_bel_attr_bitvec_u32(bcls::GLOBAL::WAKE_DELAY1, val)
                .global("WAKE_DELAY1", val.to_string())
                .commit();
        }
        for val in 1..32 {
            bctx.build()
                .test_bel_attr_bitvec_u32(bcls::GLOBAL::WAKE_DELAY2, val)
                .global("WAKE_DELAY2", val.to_string())
                .commit();
        }

        bctx.build()
            .test_bel_attr_bits(bcls::GLOBAL::SW_GWE_CYCLE)
            .multi_global("SW_GWE_CYCLE", MultiValue::Dec(0), 10);

        bctx.build()
            .test_bel_attr_bits(bcls::GLOBAL::SW_GTS_CYCLE)
            .multi_global("SW_GTS_CYCLE", MultiValue::Dec(0), 10);

        bctx.build().test_global_attr_bool_rename(
            "TESTMODE_EN",
            bcls::GLOBAL::TESTMODE_EN,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "NEXT_CONFIG_NEW_MODE",
            bcls::GLOBAL::NEXT_CONFIG_NEW_MODE,
            "NO",
            "YES",
        );
        bctx.build()
            .test_bel_attr_bits(bcls::GLOBAL::NEXT_CONFIG_BOOT_MODE)
            .multi_global("NEXT_CONFIG_BOOT_MODE", MultiValue::Bin, 3);
        bctx.build()
            .test_bel_attr_bits(bcls::GLOBAL::BOOTVSEL)
            .multi_global("BOOTVSEL", MultiValue::Bin, 3);

        bctx.build()
            .test_bel_attr_bits(bcls::GLOBAL::NEXT_CONFIG_ADDR)
            .multi_global("NEXT_CONFIG_ADDR", MultiValue::HexPrefix, 32);

        bctx.build()
            .test_global_attr_bool_rename("GLUTMASK", bcls::GLOBAL::GLUTMASK, "NO", "YES");
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_KEEP",
            bcls::GLOBAL::POST_CRC_KEEP,
            "NO",
            "YES",
        );

        for val in ["NO", "YES"] {
            ctx.build()
                .null_bits()
                .test_global_special(specials::NULL_SPI2_EN)
                .global("SPI2_EN", val)
                .commit();
            ctx.build()
                .null_bits()
                .test_global_special(specials::NULL_BRAMMASK)
                .global("BRAMMASK", val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, skip_io: bool, devdata_only: bool) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let int_tcid = if edev.chip.kind.is_virtex2() {
        tcls_v2::INT_CNR
    } else if edev.chip.kind == ChipKind::FpgaCore {
        tcls_s3::INT_CLB_FC
    } else {
        tcls_s3::INT_CLB
    };
    let int_tiles = &[int_tcid];
    let vals_pull = &[
        enums::IOB_PULL::NONE,
        enums::IOB_PULL::PULLDOWN,
        enums::IOB_PULL::PULLUP,
    ];
    let vals_pullup = &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP];

    let (cnr_sw, cnr_nw, cnr_se, cnr_ne) = match edev.chip.kind {
        ChipKind::Virtex2 => (
            tcls_v2::CNR_SW_V2,
            tcls_v2::CNR_NW_V2,
            tcls_v2::CNR_SE_V2,
            tcls_v2::CNR_NE_V2,
        ),
        ChipKind::Virtex2P | ChipKind::Virtex2PX => (
            tcls_v2::CNR_SW_V2P,
            tcls_v2::CNR_NW_V2P,
            tcls_v2::CNR_SE_V2P,
            tcls_v2::CNR_NE_V2P,
        ),
        ChipKind::Spartan3 => (
            tcls_s3::CNR_SW_S3,
            tcls_s3::CNR_NW_S3,
            tcls_s3::CNR_SE_S3,
            tcls_s3::CNR_NE_S3,
        ),
        ChipKind::FpgaCore => (
            tcls_s3::CNR_SW_FC,
            tcls_s3::CNR_NW_FC,
            tcls_s3::CNR_SE_FC,
            tcls_s3::CNR_NE_FC,
        ),
        ChipKind::Spartan3E => (
            tcls_s3::CNR_SW_S3E,
            tcls_s3::CNR_NW_S3E,
            tcls_s3::CNR_SE_S3E,
            tcls_s3::CNR_NE_S3E,
        ),
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => (
            tcls_s3::CNR_SW_S3A,
            tcls_s3::CNR_NW_S3A,
            tcls_s3::CNR_SE_S3A,
            tcls_s3::CNR_NE_S3A,
        ),
    };

    if devdata_only {
        let tcid = cnr_sw;
        let bslot = bslots::MISC_SW;
        if !edev.chip.kind.is_virtex2() {
            let sm = ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::VGG_SENDMAX);
            let sv = ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::SEND_VGG);
            if edev.chip.kind.is_spartan3a() {
                let tcid = tcls_s3::GLOBAL_S3A;
                let bslot = bslots::GLOBAL;

                assert_eq!(
                    sm,
                    ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_SENDMAX)
                );
                assert_eq!(
                    sv,
                    ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::SEND_VGG)
                );
            }
            assert!(sm[0]);
            if edev.chip.kind.is_spartan3a() {
                assert_eq!(sv, bits![1, 1, 1, 1]);
            } else if edev.chip.kind == ChipKind::FpgaCore {
                assert_eq!(sv, bits![0, 0, 0, 0]);
            } else {
                assert_eq!(sv, bits![0, 0, 0, 1]);
            }
        }
        if edev.chip.kind.is_virtex2() {
            let diff = ctx.get_diff_bel_special(tcid, bslot, specials::FREEZE_DCI);
            let diff = diff.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(4)]));
            let mut freeze_dci_nops = 0;
            for (bit, val) in diff.bits {
                assert!(val);
                freeze_dci_nops |= 1 << bit.bit.to_idx();
            }
            ctx.insert_devdata_u32(devdata::FREEZE_DCI_NOPS, freeze_dci_nops);

            let is_double_grestore = ctx.empty_bs.die[DieId::from_idx(0)]
                .regs
                .get(&Reg::FakeDoubleGrestore)
                == Some(&1);
            ctx.insert_devdata_bool(devdata::DOUBLE_GRESTORE, is_double_grestore);
        }

        return;
    }

    if edev.chip.kind == ChipKind::Spartan3 {
        for tcid in [cnr_sw, cnr_nw, cnr_se, cnr_ne] {
            for bslot in bslots::DCIRESET {
                ctx.collect_bel_attr(tcid, bslot, bcls::DCIRESET::ENABLE);
            }
        }
    }

    {
        // LL
        let tcid = cnr_sw;
        let bslot = bslots::MISC_SW;
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::DISABLE_BANDGAP);
            let item = xlat_bit_wide_bi(
                ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::MISC_SW::RAISE_VGG, false),
                ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::MISC_SW::RAISE_VGG, true),
            );
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_SW::RAISE_VGG, item);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::BCLK_N_DIV2);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::ZCLK_N_DIV2);
            if edev.chip.kind.is_virtex2p() {
                ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::DISABLE_VGG_GENERATION);
            }
        } else {
            let sm = ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::VGG_SENDMAX);
            let sv = ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::SEND_VGG);
            let veo = ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::VGG_ENABLE_OFFCHIP);
            if edev.chip.kind.is_spartan3a() {
                let tcid = tcls_s3::GLOBAL_S3A;
                let bslot = bslots::GLOBAL;

                assert_eq!(
                    sm,
                    ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_SENDMAX)
                );
                assert_eq!(
                    sv,
                    ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::SEND_VGG)
                );
                assert_eq!(
                    veo,
                    ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_ENABLE_OFFCHIP)
                );
            }
            assert!(sm[0]);
            assert!(!veo[0]);
            if edev.chip.kind.is_spartan3a() {
                assert_eq!(sv, bits![1, 1, 1, 1]);
            } else if edev.chip.kind == ChipKind::FpgaCore {
                assert_eq!(sv, bits![0, 0, 0, 0]);
            } else {
                assert_eq!(sv, bits![0, 0, 0, 1]);
            }
        }
        if edev.chip.kind == ChipKind::Spartan3 {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::GATE_GHIGH);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SW::DCI_OSC_SEL);
        }
        if edev.chip.kind.is_spartan3ea() {
            ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SW::TEMP_SENSOR);
        }
        if edev.chip.kind.is_spartan3a() {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SW::CCLK2_PULL, vals_pull);
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SW::MOSI2_PULL, vals_pull);
        } else if edev.chip.kind != ChipKind::Spartan3E && edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SW::M0_PULL, vals_pull);
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SW::M1_PULL, vals_pull);
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SW::M2_PULL, vals_pull);
        }
        if edev.chip.kind.is_virtex2() {
            let diff = ctx.get_diff_bel_special(tcid, bslot, specials::FREEZE_DCI);
            let diff = diff.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(4)]));
            let mut freeze_dci_nops = 0;
            for (bit, val) in diff.bits {
                assert!(val);
                freeze_dci_nops |= 1 << bit.bit.to_idx();
            }
            ctx.insert_devdata_u32(devdata::FREEZE_DCI_NOPS, freeze_dci_nops);
        }
    }

    {
        // UL
        let tcid = cnr_nw;
        let bslot = bslots::MISC_NW;
        if edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NW::PROG_PULL, vals_pullup);
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NW::TDI_PULL, vals_pull);
        }
        if edev.chip.kind.is_spartan3a() {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NW::TMS_PULL, vals_pull);
        }
        if !edev.chip.kind.is_spartan3ea() && edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NW::HSWAPEN_PULL, vals_pull);
        }
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_NW::TEST_LL);
    }

    // LR
    let tcid = cnr_se;
    {
        let bslot = bslots::MISC_SE;
        if edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SE::DONE_PULL, vals_pullup);
        }
        if !edev.chip.kind.is_spartan3a() && edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SE::CCLK_PULL, vals_pullup);
        }
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_SE::POWERDOWN_PULL, vals_pullup);
        }
        if edev.chip.kind == ChipKind::FpgaCore {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_SE::ABUFF);
        }
    }
    {
        let bslot = bslots::STARTUP;
        for pin in [bcls::STARTUP::CLK, bcls::STARTUP::GTS, bcls::STARTUP::GSR] {
            let item = xlat_bit_bi(
                ctx.get_diff_bel_input_inv(int_tcid, bslot, pin, false),
                ctx.get_diff_bel_input_inv(int_tcid, bslot, pin, true),
            );
            ctx.insert_bel_input_inv_int(int_tiles, tcid, bslot, pin, item);
        }
        let diff0_gts = ctx.get_diff_bel_input_inv(tcid, bslot, bcls::STARTUP::GTS, false);
        let diff1_gts = ctx.get_diff_bel_input_inv(tcid, bslot, bcls::STARTUP::GTS, true);
        assert_eq!(diff0_gts, diff1_gts);
        let diff0_gsr = ctx.get_diff_bel_input_inv(tcid, bslot, bcls::STARTUP::GSR, false);
        let diff1_gsr = ctx.get_diff_bel_input_inv(tcid, bslot, bcls::STARTUP::GSR, true);
        assert_eq!(diff0_gsr, diff1_gsr);
        assert_eq!(diff0_gts, diff0_gsr);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::STARTUP::USER_GTS_GSR_ENABLE,
            xlat_bit(diff0_gsr),
        );
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GTS_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GSR_SYNC);
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GWE_SYNC);
        }
    }
    {
        let bslot = bslots::CAPTURE;
        for pin in [bcls::CAPTURE::CLK, bcls::CAPTURE::CAP] {
            let item = xlat_bit_bi(
                ctx.get_diff_bel_input_inv(int_tcid, bslot, pin, false),
                ctx.get_diff_bel_input_inv(int_tcid, bslot, pin, true),
            );
            ctx.insert_bel_input_inv_int(int_tiles, tcid, bslot, pin, item);
        }
    }
    if edev.chip.kind != ChipKind::Spartan3E {
        let bslot = bslots::ICAP;
        for pin in [bcls::ICAP::CLK, bcls::ICAP::CE, bcls::ICAP::WRITE] {
            let item = xlat_bit_bi(
                ctx.get_diff_bel_input_inv(int_tcid, bslot, pin, false),
                ctx.get_diff_bel_input_inv(int_tcid, bslot, pin, true),
            );
            ctx.insert_bel_input_inv_int(int_tiles, tcid, bslot, pin, item);
        }
        if !edev.chip.kind.is_spartan3a() {
            ctx.collect_bel_attr(tcid, bslot, bcls::ICAP::ENABLE);
        }
    }
    if edev.chip.kind.is_spartan3a() {
        let bslot = bslots::SPI_ACCESS;
        ctx.collect_bel_attr(tcid, bslot, bcls::SPI_ACCESS::ENABLE);
        let mut diff = ctx.get_diff_attr_bit(int_tcid, bslot, bcls::SPI_ACCESS::ENABLE, 0);
        diff.discard_bits(&[ctx
            .item_int_inv(int_tiles, tcid, bslot, bcls::SPI_ACCESS::MOSI)
            .bit]);
        diff.assert_empty();
    }

    // UR
    let tcid = cnr_ne;
    {
        let bslot = bslots::MISC_NE;
        if edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NE::TCK_PULL, vals_pull);
            ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NE::TDO_PULL, vals_pull);
            if !edev.chip.kind.is_spartan3a() {
                ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NE::TMS_PULL, vals_pull);
            } else {
                ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NE::MISO2_PULL, vals_pull);
                ctx.collect_bel_attr_subset(tcid, bslot, bcls::MISC_NE::CSO2_PULL, vals_pull);
            }
        }
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISC_NE::TEST_LL);
        }
    }
    {
        let bslot = bslots::BSCAN;
        ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::USERCODE);
        let diff = ctx.get_diff_attr_bit(tcid, bslot, bcls::BSCAN::USER_TDO_ENABLE, 0);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::BSCAN::USER_TDO_ENABLE,
            xlat_bit_wide(diff),
        );
    }

    if edev.chip.kind.is_virtex2p() {
        let bslot = bslots::JTAGPPC;
        ctx.collect_bel_attr(tcid, bslot, bcls::JTAGPPC::ENABLE);
    }

    if edev.chip.kind == ChipKind::FpgaCore {
        for tcid in [
            tcls_s3::CNR_SW_FC,
            tcls_s3::CNR_NW_FC,
            tcls_s3::CNR_SE_FC,
            tcls_s3::CNR_NE_FC,
        ] {
            let bslot = bslots::MISR_FC;
            ctx.collect_bel_attr(tcid, bslot, bcls::MISR_FC::MISR_CLOCK);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::MISR_FC::MISR_RESET);
        }
        // could be verified, but meh; they just route given GCLK to CLK3 of every corner tile.
        ctx.get_diff_bel_special(
            tcls_s3::INT_CLB_FC,
            bslots::MISR_FC,
            specials::MISR_CLOCK_GCLK0,
        );
        ctx.get_diff_bel_special(tcls_s3::HCLK, bslots::HCLK, specials::MISR_CLOCK_GCLK0);
        ctx.get_diff_bel_special(tcls_s3::CLKQC_S3, bslots::HROW, specials::MISR_CLOCK_GCLK0);
    }

    // I/O bank misc control
    if !skip_io && edev.chip.kind != ChipKind::FpgaCore {
        if !edev.chip.kind.is_spartan3ea() {
            let lvdsbias = if edev.chip.kind.is_virtex2() {
                bcls::DCI::V2_LVDSBIAS
            } else {
                bcls::DCI::S3_LVDSBIAS
            };
            for (tcid, idx) in [
                (cnr_nw, 0),
                (cnr_nw, 1),
                (cnr_ne, 1),
                (cnr_ne, 0),
                (cnr_se, 0),
                (cnr_se, 1),
                (cnr_sw, 1),
                (cnr_sw, 0),
            ] {
                let bslot = bslots::DCI[idx];
                // LVDS
                let mut vals = vec![];
                for std in get_iostds(edev, false) {
                    let rid = iostd_to_row(edev, &std);
                    if std.diff != DiffKind::True {
                        continue;
                    }
                    let diff = ctx.get_diff_bel_attr_row(tcid, bslot, lvdsbias, rid);
                    vals.push((
                        rid,
                        diff.filter_rects(&if edev.chip.kind.is_virtex2() {
                            EntityVec::from_iter([BitRectId::from_idx(0), BitRectId::from_idx(1)])
                        } else {
                            EntityVec::from_iter([BitRectId::from_idx(0)])
                        }),
                    ));
                }
                vals.push((IOB_DATA::OFF, Diff::default()));
                let field = match edev.chip.kind {
                    ChipKind::Virtex2 => IOB_DATA::V2_LVDSBIAS,
                    ChipKind::Virtex2P | ChipKind::Virtex2PX => IOB_DATA::V2P_LVDSBIAS,
                    ChipKind::Spartan3 => IOB_DATA::S3_LVDSBIAS,
                    _ => unreachable!(),
                };

                if edev.chip.kind == ChipKind::Spartan3 {
                    ctx.collect_bel_attr(tcid, bslot, lvdsbias);
                } else {
                    ctx.insert_bel_attr_bitvec(
                        tcid,
                        bslot,
                        lvdsbias,
                        match idx {
                            0 => vec![
                                TileBit::new(0, 3, 48).pos(),
                                TileBit::new(0, 2, 48).pos(),
                                TileBit::new(0, 3, 47).pos(),
                                TileBit::new(0, 2, 47).pos(),
                                TileBit::new(0, 3, 46).pos(),
                                TileBit::new(0, 2, 46).pos(),
                                TileBit::new(0, 3, 45).pos(),
                                TileBit::new(0, 2, 45).pos(),
                                TileBit::new(0, 3, 44).pos(),
                            ],
                            1 => vec![
                                TileBit::new(1, 12, 8).pos(),
                                TileBit::new(1, 12, 6).pos(),
                                TileBit::new(1, 12, 7).pos(),
                                TileBit::new(1, 12, 10).pos(),
                                TileBit::new(1, 12, 11).pos(),
                                TileBit::new(1, 12, 9).pos(),
                                TileBit::new(1, 13, 9).pos(),
                                TileBit::new(1, 13, 11).pos(),
                                TileBit::new(1, 13, 7).pos(),
                            ],
                            _ => unreachable!(),
                        },
                    );
                }
                let item = ctx.bel_attr_bitvec(tcid, bslot, lvdsbias).to_vec();
                let base = BitVec::repeat(false, item.len());
                for (rid, diff) in vals {
                    let val = extract_bitvec_val(&item, &base, diff);
                    ctx.insert_table_bitvec(IOB_DATA, rid, field, val);
                }

                // DCI
                let diff_fdh =
                    !ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::DCI::FORCE_DONE_HIGH, false);
                if edev.chip.kind.is_virtex2() {
                    let diff = ctx
                        .get_diff_bel_special(tcid, bslot, specials::DCI_OUT)
                        .filter_rects(&EntityVec::from_iter([
                            BitRectId::from_idx(0),
                            BitRectId::from_idx(1),
                        ]));
                    let diff_p = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
                    let diff_t = ctx.get_diff_bel_special(tcid, bslot, specials::DCI_TEST);
                    assert_eq!(diff_p, diff.combine(&diff_fdh));
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::ENABLE, xlat_bit(diff));
                    let diff_t = diff_t.combine(&!diff_p);
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::TEST_ENABLE, xlat_bit(diff_t));
                } else {
                    let diff_ar = ctx
                        .get_diff_bel_special(tcid, bslot, specials::DCI_ASREQUIRED)
                        .filter_rects(&EntityVec::from_iter([BitRectId::from_idx(0)]));
                    let diff_c = ctx
                        .get_diff_bel_special(tcid, bslot, specials::DCI_CONTINUOUS)
                        .filter_rects(&EntityVec::from_iter([BitRectId::from_idx(0)]));
                    let diff_q = ctx
                        .get_diff_bel_special(tcid, bslot, specials::DCI_QUIET)
                        .filter_rects(&EntityVec::from_iter([BitRectId::from_idx(0)]));
                    let diff_p = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
                    assert_eq!(diff_c, diff_ar);
                    let diff_q = diff_q.combine(&!&diff_c);
                    let diff_p = diff_p.combine(&!&diff_c).combine(&!&diff_fdh);
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::ENABLE, xlat_bit(diff_c));
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::QUIET, xlat_bit(diff_q));
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::TEST_ENABLE, xlat_bit(diff_p));
                }
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslot,
                    bcls::DCI::FORCE_DONE_HIGH,
                    xlat_bit(diff_fdh),
                );

                // DCI TERM stuff
                let (pmask_term_vcc, pmask_term_split, nmask_term_split) =
                    if edev.chip.kind == ChipKind::Spartan3 {
                        let frame = if tcid == cnr_sw {
                            match idx {
                                0 => 1,
                                1 => 0,
                                _ => unreachable!(),
                            }
                        } else {
                            match idx {
                                0 => 0,
                                1 => 1,
                                _ => unreachable!(),
                            }
                        };
                        (
                            vec![
                                TileBit::new(0, frame, 51).pos(),
                                TileBit::new(0, frame, 52).pos(),
                                TileBit::new(0, frame, 53).pos(),
                                TileBit::new(0, frame, 54).pos(),
                            ],
                            vec![
                                TileBit::new(0, frame, 56).pos(),
                                TileBit::new(0, frame, 57).pos(),
                                TileBit::new(0, frame, 58).pos(),
                                TileBit::new(0, frame, 59).pos(),
                            ],
                            vec![
                                TileBit::new(0, frame, 46).pos(),
                                TileBit::new(0, frame, 47).pos(),
                                TileBit::new(0, frame, 48).pos(),
                                TileBit::new(0, frame, 49).pos(),
                            ],
                        )
                    } else {
                        (
                            match idx {
                                0 => vec![
                                    TileBit::new(0, 3, 36).pos(),
                                    TileBit::new(0, 2, 36).pos(),
                                    TileBit::new(0, 3, 35).pos(),
                                    TileBit::new(0, 2, 35).pos(),
                                    TileBit::new(0, 3, 34).pos(),
                                ],
                                1 => vec![
                                    TileBit::new(1, 8, 8).pos(),
                                    TileBit::new(1, 8, 6).pos(),
                                    TileBit::new(1, 8, 7).pos(),
                                    TileBit::new(1, 8, 11).pos(),
                                    TileBit::new(1, 8, 10).pos(),
                                ],
                                _ => unreachable!(),
                            },
                            match idx {
                                0 => vec![
                                    TileBit::new(0, 2, 34).pos(),
                                    TileBit::new(0, 3, 33).pos(),
                                    TileBit::new(0, 2, 33).pos(),
                                    TileBit::new(0, 3, 32).pos(),
                                    TileBit::new(0, 2, 32).pos(),
                                ],
                                1 => vec![
                                    TileBit::new(1, 8, 9).pos(),
                                    TileBit::new(1, 9, 9).pos(),
                                    TileBit::new(1, 9, 11).pos(),
                                    TileBit::new(1, 9, 7).pos(),
                                    TileBit::new(1, 9, 10).pos(),
                                ],
                                _ => unreachable!(),
                            },
                            match idx {
                                0 => vec![
                                    TileBit::new(0, 2, 39).pos(),
                                    TileBit::new(0, 3, 38).pos(),
                                    TileBit::new(0, 2, 38).pos(),
                                    TileBit::new(0, 3, 37).pos(),
                                    TileBit::new(0, 2, 37).pos(),
                                ],
                                1 => vec![
                                    TileBit::new(1, 11, 11).pos(),
                                    TileBit::new(1, 11, 7).pos(),
                                    TileBit::new(1, 11, 10).pos(),
                                    TileBit::new(1, 11, 8).pos(),
                                    TileBit::new(1, 11, 6).pos(),
                                ],
                                _ => unreachable!(),
                            },
                        )
                    };
                let item_en = ctx.bel_attr_bit(tcid, bslot, bcls::DCI::ENABLE);
                let (
                    attr_pmask_term_vcc,
                    attr_pmask_term_split,
                    attr_nmask_term_split,
                    field_pmask_term_vcc,
                    field_pmask_term_split,
                    field_nmask_term_split,
                ) = match edev.chip.kind {
                    ChipKind::Virtex2 => (
                        bcls::DCI::V2_PMASK_TERM_VCC,
                        bcls::DCI::V2_PMASK_TERM_SPLIT,
                        bcls::DCI::V2_NMASK_TERM_SPLIT,
                        IOB_DATA::V2_PMASK_TERM_VCC,
                        IOB_DATA::V2_PMASK_TERM_SPLIT,
                        IOB_DATA::V2_NMASK_TERM_SPLIT,
                    ),
                    ChipKind::Virtex2P | ChipKind::Virtex2PX => (
                        bcls::DCI::V2_PMASK_TERM_VCC,
                        bcls::DCI::V2_PMASK_TERM_SPLIT,
                        bcls::DCI::V2_NMASK_TERM_SPLIT,
                        IOB_DATA::V2P_PMASK_TERM_VCC,
                        IOB_DATA::V2P_PMASK_TERM_SPLIT,
                        IOB_DATA::V2P_NMASK_TERM_SPLIT,
                    ),
                    ChipKind::Spartan3 => (
                        bcls::DCI::S3_PMASK_TERM_VCC,
                        bcls::DCI::S3_PMASK_TERM_SPLIT,
                        bcls::DCI::S3_NMASK_TERM_SPLIT,
                        IOB_DATA::S3_PMASK_TERM_VCC,
                        IOB_DATA::S3_PMASK_TERM_SPLIT,
                        IOB_DATA::S3_NMASK_TERM_SPLIT,
                    ),
                    _ => unreachable!(),
                };
                for std in get_iostds(edev, false) {
                    let rid = iostd_to_row(edev, &std);
                    if std.name.starts_with("DIFF_") {
                        continue;
                    }
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => (),
                        DciKind::InputVcc | DciKind::BiVcc => {
                            let mut diff = ctx
                                .get_diff_bel_special_row(tcid, bslot, specials::DCI_TERM, rid)
                                .filter_rects(&if edev.chip.kind.is_virtex2() {
                                    EntityVec::from_iter([
                                        BitRectId::from_idx(0),
                                        BitRectId::from_idx(1),
                                    ])
                                } else {
                                    EntityVec::from_iter([BitRectId::from_idx(0)])
                                });
                            diff.apply_bit_diff(item_en, true, false);
                            let val = extract_bitvec_val_part(
                                &pmask_term_vcc,
                                &BitVec::repeat(false, pmask_term_vcc.len()),
                                &mut diff,
                            );
                            ctx.insert_table_bitvec(IOB_DATA, rid, field_pmask_term_vcc, val);
                            diff.assert_empty();
                        }
                        DciKind::InputSplit | DciKind::BiSplit => {
                            if std.diff == DiffKind::True {
                                ctx.insert_table_bitvec(
                                    IOB_DATA,
                                    rid,
                                    field_pmask_term_split,
                                    BitVec::repeat(false, pmask_term_split.len()),
                                );
                                ctx.insert_table_bitvec(
                                    IOB_DATA,
                                    rid,
                                    field_nmask_term_split,
                                    BitVec::repeat(false, nmask_term_split.len()),
                                );
                            } else {
                                let mut diff = ctx
                                    .get_diff_bel_special_row(tcid, bslot, specials::DCI_TERM, rid)
                                    .filter_rects(&if edev.chip.kind.is_virtex2() {
                                        EntityVec::from_iter([
                                            BitRectId::from_idx(0),
                                            BitRectId::from_idx(1),
                                        ])
                                    } else {
                                        EntityVec::from_iter([BitRectId::from_idx(0)])
                                    });
                                diff.apply_bit_diff(item_en, true, false);
                                let val = extract_bitvec_val_part(
                                    &pmask_term_split,
                                    &BitVec::repeat(false, pmask_term_split.len()),
                                    &mut diff,
                                );
                                ctx.insert_table_bitvec(IOB_DATA, rid, field_pmask_term_split, val);
                                let val = extract_bitvec_val_part(
                                    &nmask_term_split,
                                    &BitVec::repeat(false, nmask_term_split.len()),
                                    &mut diff,
                                );
                                ctx.insert_table_bitvec(IOB_DATA, rid, field_nmask_term_split, val);
                                diff.assert_empty();
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                ctx.insert_table_bitvec(
                    IOB_DATA,
                    IOB_DATA::OFF,
                    field_pmask_term_vcc,
                    BitVec::repeat(false, pmask_term_vcc.len()),
                );
                ctx.insert_table_bitvec(
                    IOB_DATA,
                    IOB_DATA::OFF,
                    field_pmask_term_split,
                    BitVec::repeat(false, pmask_term_split.len()),
                );
                ctx.insert_table_bitvec(
                    IOB_DATA,
                    IOB_DATA::OFF,
                    field_nmask_term_split,
                    BitVec::repeat(false, nmask_term_split.len()),
                );

                ctx.insert_bel_attr_bitvec(tcid, bslot, attr_pmask_term_vcc, pmask_term_vcc);
                ctx.insert_bel_attr_bitvec(tcid, bslot, attr_pmask_term_split, pmask_term_split);
                ctx.insert_bel_attr_bitvec(tcid, bslot, attr_nmask_term_split, nmask_term_split);
            }

            if edev.chip.kind == ChipKind::Spartan3 {
                for tcid in [cnr_sw, cnr_nw, cnr_se, cnr_ne] {
                    let item = xlat_bit_bi(
                        ctx.get_diff_bel_special(tcid, bslots::DCI[0], specials::DCI_SELECT),
                        ctx.get_diff_bel_special(tcid, bslots::DCI[1], specials::DCI_SELECT),
                    );
                    ctx.insert_bel_attr_bool(
                        tcid,
                        bslots::MISC_CNR_S3,
                        bcls::MISC_CNR_S3::MUX_DCI_TEST,
                        item,
                    );
                }
            }
            if edev.chip.kind.is_virtex2p()
                && !ctx.device.name.ends_with("2vp4")
                && !ctx.device.name.ends_with("2vp7")
            {
                let diff = ctx.get_diff_raw(&DiffKey::GlobalSpecial(specials::DCI_QUIET));
                let diff0 = diff.filter_rects(&EntityVec::from_iter([
                    BitRectId::from_idx(8),
                    BitRectId::from_idx(0),
                ]));
                let diff1 = diff.filter_rects(&EntityVec::from_iter([
                    BitRectId::from_idx(8),
                    BitRectId::from_idx(1),
                ]));
                let diff2 = diff.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(2)]));
                let diff3 = diff.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(3)]));
                let diff4 = diff.filter_rects(&EntityVec::from_iter([
                    BitRectId::from_idx(8),
                    BitRectId::from_idx(4),
                ]));
                let diff5 = diff.filter_rects(&EntityVec::from_iter([
                    BitRectId::from_idx(8),
                    BitRectId::from_idx(5),
                ]));
                let diff6 = diff.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(6)]));
                let diff7 = diff.filter_rects(&EntityVec::from_iter([BitRectId::from_idx(7)]));
                ctx.insert_bel_attr_bool(cnr_nw, bslots::DCI[1], bcls::DCI::QUIET, xlat_bit(diff0));
                ctx.insert_bel_attr_bool(cnr_ne, bslots::DCI[1], bcls::DCI::QUIET, xlat_bit(diff1));
                ctx.insert_bel_attr_bool(cnr_ne, bslots::DCI[0], bcls::DCI::QUIET, xlat_bit(diff2));
                ctx.insert_bel_attr_bool(cnr_se, bslots::DCI[0], bcls::DCI::QUIET, xlat_bit(diff3));
                ctx.insert_bel_attr_bool(cnr_se, bslots::DCI[1], bcls::DCI::QUIET, xlat_bit(diff4));
                ctx.insert_bel_attr_bool(cnr_sw, bslots::DCI[1], bcls::DCI::QUIET, xlat_bit(diff5));
                ctx.insert_bel_attr_bool(cnr_sw, bslots::DCI[0], bcls::DCI::QUIET, xlat_bit(diff6));
                ctx.insert_bel_attr_bool(cnr_nw, bslots::DCI[0], bcls::DCI::QUIET, xlat_bit(diff7));
            }

            let tcid = cnr_sw;
            let bslot = bslots::DCI[0];
            let mut diff = ctx
                .get_diff_bel_special(tcid, bslot, specials::DCI_OUT_ALONE)
                .filter_rects(&if edev.chip.kind.is_virtex2() {
                    EntityVec::from_iter([BitRectId::from_idx(0), BitRectId::from_idx(1)])
                } else {
                    EntityVec::from_iter([BitRectId::from_idx(0)])
                });
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::DCI::ENABLE),
                true,
                false,
            );
            if edev.chip.dci_io_alt.contains_key(&5) {
                let bslot = bslots::DCI[1];
                let mut alt_diff = ctx
                    .get_diff_bel_special(tcid, bslot, specials::DCI_OUT_ALONE)
                    .filter_rects(&if edev.chip.kind.is_virtex2() {
                        EntityVec::from_iter([BitRectId::from_idx(0), BitRectId::from_idx(1)])
                    } else {
                        EntityVec::from_iter([BitRectId::from_idx(0)])
                    });
                alt_diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, bcls::DCI::ENABLE),
                    true,
                    false,
                );
                alt_diff = alt_diff.combine(&!&diff);
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslots::MISC_SW,
                    bcls::MISC_SW::DCI_ALTVR,
                    xlat_bit(alt_diff),
                );
            }
            if edev.chip.kind.is_virtex2() {
                diff.apply_bitvec_diff(
                    ctx.bel_attr_bitvec(tcid, bslots::MISC_SW, bcls::MISC_SW::ZCLK_N_DIV2),
                    &bits![0, 0, 0, 1, 0],
                    &BitVec::repeat(false, 5),
                );
            }
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_SW,
                bcls::MISC_SW::DCI_CLK_ENABLE,
                xlat_bit(diff),
            );
        } else {
            let banks = if edev.chip.kind == ChipKind::Spartan3E {
                vec![
                    (
                        cnr_nw,
                        vec![
                            TileBit::new(0, 0, 44).pos(),
                            TileBit::new(0, 0, 39).pos(),
                            TileBit::new(0, 0, 38).pos(),
                            TileBit::new(0, 0, 37).pos(),
                            TileBit::new(0, 0, 36).pos(),
                            TileBit::new(0, 0, 27).pos(),
                            TileBit::new(0, 0, 26).pos(),
                            TileBit::new(0, 0, 25).pos(),
                            TileBit::new(0, 0, 24).pos(),
                            TileBit::new(0, 0, 23).pos(),
                            TileBit::new(0, 0, 22).pos(),
                        ],
                        vec![
                            TileBit::new(0, 0, 45).pos(),
                            TileBit::new(0, 0, 43).pos(),
                            TileBit::new(0, 0, 42).pos(),
                            TileBit::new(0, 0, 41).pos(),
                            TileBit::new(0, 0, 40).pos(),
                            TileBit::new(0, 0, 35).pos(),
                            TileBit::new(0, 0, 34).pos(),
                            TileBit::new(0, 0, 33).pos(),
                            TileBit::new(0, 0, 32).pos(),
                            TileBit::new(0, 0, 29).pos(),
                            TileBit::new(0, 0, 28).pos(),
                        ],
                    ),
                    (
                        cnr_ne,
                        vec![
                            TileBit::new(0, 1, 10).pos(),
                            TileBit::new(0, 1, 48).pos(),
                            TileBit::new(0, 1, 47).pos(),
                            TileBit::new(0, 1, 46).pos(),
                            TileBit::new(0, 1, 45).pos(),
                            TileBit::new(0, 1, 38).pos(),
                            TileBit::new(0, 1, 37).pos(),
                            TileBit::new(0, 1, 36).pos(),
                            TileBit::new(0, 1, 35).pos(),
                            TileBit::new(0, 1, 34).pos(),
                            TileBit::new(0, 1, 33).pos(),
                        ],
                        vec![
                            TileBit::new(0, 1, 11).pos(),
                            TileBit::new(0, 1, 9).pos(),
                            TileBit::new(0, 1, 51).pos(),
                            TileBit::new(0, 1, 50).pos(),
                            TileBit::new(0, 1, 49).pos(),
                            TileBit::new(0, 1, 44).pos(),
                            TileBit::new(0, 1, 43).pos(),
                            TileBit::new(0, 1, 42).pos(),
                            TileBit::new(0, 1, 41).pos(),
                            TileBit::new(0, 1, 40).pos(),
                            TileBit::new(0, 1, 39).pos(),
                        ],
                    ),
                    (
                        cnr_se,
                        vec![
                            TileBit::new(0, 1, 12).pos(),
                            TileBit::new(0, 1, 7).pos(),
                            TileBit::new(0, 1, 36).pos(),
                            TileBit::new(0, 1, 35).pos(),
                            TileBit::new(0, 1, 34).pos(),
                            TileBit::new(0, 1, 27).pos(),
                            TileBit::new(0, 1, 26).pos(),
                            TileBit::new(0, 1, 25).pos(),
                            TileBit::new(0, 1, 24).pos(),
                            TileBit::new(0, 1, 23).pos(),
                            TileBit::new(0, 1, 22).pos(),
                        ],
                        vec![
                            TileBit::new(0, 1, 13).pos(),
                            TileBit::new(0, 1, 11).pos(),
                            TileBit::new(0, 1, 10).pos(),
                            TileBit::new(0, 1, 9).pos(),
                            TileBit::new(0, 1, 8).pos(),
                            TileBit::new(0, 1, 33).pos(),
                            TileBit::new(0, 1, 32).pos(),
                            TileBit::new(0, 1, 31).pos(),
                            TileBit::new(0, 1, 30).pos(),
                            TileBit::new(0, 1, 29).pos(),
                            TileBit::new(0, 1, 28).pos(),
                        ],
                    ),
                    (
                        cnr_sw,
                        vec![
                            TileBit::new(0, 1, 31).pos(),
                            TileBit::new(0, 1, 26).pos(),
                            TileBit::new(0, 1, 25).pos(),
                            TileBit::new(0, 1, 24).pos(),
                            TileBit::new(0, 1, 23).pos(),
                            TileBit::new(0, 1, 38).pos(),
                            TileBit::new(0, 1, 37).pos(),
                            TileBit::new(0, 1, 36).pos(),
                            TileBit::new(0, 1, 35).pos(),
                            TileBit::new(0, 1, 34).pos(),
                            TileBit::new(0, 1, 33).pos(),
                        ],
                        vec![
                            TileBit::new(0, 1, 32).pos(),
                            TileBit::new(0, 1, 30).pos(),
                            TileBit::new(0, 1, 29).pos(),
                            TileBit::new(0, 1, 28).pos(),
                            TileBit::new(0, 1, 27).pos(),
                            TileBit::new(0, 1, 22).pos(),
                            TileBit::new(0, 1, 43).pos(),
                            TileBit::new(0, 1, 42).pos(),
                            TileBit::new(0, 1, 41).pos(),
                            TileBit::new(0, 1, 40).pos(),
                            TileBit::new(0, 1, 39).pos(),
                        ],
                    ),
                ]
            } else {
                vec![
                    (
                        cnr_nw,
                        vec![
                            TileBit::new(0, 1, 62).pos(),
                            TileBit::new(0, 1, 60).pos(),
                            TileBit::new(0, 1, 55).pos(),
                            TileBit::new(0, 1, 54).pos(),
                            TileBit::new(0, 1, 53).pos(),
                            TileBit::new(0, 1, 52).pos(),
                            TileBit::new(0, 1, 45).pos(),
                            TileBit::new(0, 1, 44).pos(),
                            TileBit::new(0, 1, 43).pos(),
                            TileBit::new(0, 1, 42).pos(),
                            TileBit::new(0, 1, 41).pos(),
                            TileBit::new(0, 1, 40).pos(),
                        ],
                        vec![
                            TileBit::new(0, 1, 63).pos(),
                            TileBit::new(0, 1, 61).pos(),
                            TileBit::new(0, 1, 59).pos(),
                            TileBit::new(0, 1, 58).pos(),
                            TileBit::new(0, 1, 57).pos(),
                            TileBit::new(0, 1, 56).pos(),
                            TileBit::new(0, 1, 51).pos(),
                            TileBit::new(0, 1, 50).pos(),
                            TileBit::new(0, 1, 49).pos(),
                            TileBit::new(0, 1, 48).pos(),
                            TileBit::new(0, 1, 47).pos(),
                            TileBit::new(0, 1, 46).pos(),
                        ],
                    ),
                    (
                        cnr_sw,
                        vec![
                            TileBit::new(0, 1, 32).pos(),
                            TileBit::new(0, 0, 27).pos(),
                            TileBit::new(0, 0, 31).pos(),
                            TileBit::new(0, 1, 30).pos(),
                            TileBit::new(0, 1, 36).pos(),
                            TileBit::new(0, 1, 28).pos(),
                            TileBit::new(0, 0, 10).pos(),
                            TileBit::new(0, 1, 11).pos(),
                            TileBit::new(0, 1, 34).pos(),
                            TileBit::new(0, 1, 33).pos(),
                            TileBit::new(0, 1, 10).pos(),
                            TileBit::new(0, 0, 9).pos(),
                        ],
                        vec![
                            TileBit::new(0, 1, 27).pos(),
                            TileBit::new(0, 0, 28).pos(),
                            TileBit::new(0, 0, 26).pos(),
                            TileBit::new(0, 1, 26).pos(),
                            TileBit::new(0, 1, 62).pos(),
                            TileBit::new(0, 1, 63).pos(),
                            TileBit::new(0, 0, 30).pos(),
                            TileBit::new(0, 1, 9).pos(),
                            TileBit::new(0, 1, 35).pos(),
                            TileBit::new(0, 0, 29).pos(),
                            TileBit::new(0, 0, 62).pos(),
                            TileBit::new(0, 0, 6).pos(),
                        ],
                    ),
                ]
            };
            for (tcid, lvdsbias_0, lvdsbias_1) in banks {
                let bslot = bslots::BANK;
                let field = if edev.chip.kind == ChipKind::Spartan3E {
                    IOB_DATA::S3E_LVDSBIAS
                } else {
                    IOB_DATA::S3A_LVDSBIAS
                };
                let tcrd = edev.tile_index[tcid][0];
                let btile = edev.btile_term_h(tcrd.cell);
                let base: BitVec = lvdsbias_0
                    .iter()
                    .map(|bit| {
                        ctx.empty_bs
                            .get_bit(btile.xlat_pos_fwd((bit.bit.frame, bit.bit.bit)))
                    })
                    .collect();

                for std in get_iostds(edev, false) {
                    let rid = iostd_to_row(edev, &std);
                    if std.diff != DiffKind::True {
                        continue;
                    }
                    if std.name != "LVDS_25" || edev.chip.kind.is_spartan3a() {
                        let diff_0 = ctx
                            .get_diff_bel_special_row(tcid, bslot, specials::BANK_LVDSBIAS_0, rid)
                            .filter_rects(&EntityVec::from_iter([BitRectId::from_idx(0)]));
                        let val_0 = extract_bitvec_val(&lvdsbias_0, &base, diff_0);
                        ctx.insert_table_bitvec(IOB_DATA, rid, field, val_0);
                    }
                    let diff_1 = ctx
                        .get_diff_bel_special_row(tcid, bslot, specials::BANK_LVDSBIAS_1, rid)
                        .filter_rects(&EntityVec::from_iter([BitRectId::from_idx(0)]));
                    let val_1 = extract_bitvec_val(&lvdsbias_1, &base, diff_1);
                    ctx.insert_table_bitvec(IOB_DATA, rid, field, val_1);
                }
                ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, field, base);
                let attr = if edev.chip.kind == ChipKind::Spartan3E {
                    bcls::BANK::S3E_LVDSBIAS
                } else {
                    bcls::BANK::S3A_LVDSBIAS
                };
                ctx.insert_bel_attr_bitvec(tcid, bslot, attr, [lvdsbias_0, lvdsbias_1].concat());
            }
        }

        if edev.chip.kind.is_spartan3ea() {
            for (tcid, btile) in [
                (cnr_sw, edev.btile_term_h(edev.chip.corner(DirHV::SW).cell)),
                (cnr_nw, edev.btile_term_h(edev.chip.corner(DirHV::NW).cell)),
                (cnr_se, edev.btile_term_h(edev.chip.corner(DirHV::SE).cell)),
                (cnr_ne, edev.btile_term_h(edev.chip.corner(DirHV::NE).cell)),
            ] {
                let mut diff = Diff::default();
                let BitRect::Main(_, _, width, _, height, _) = btile else {
                    unreachable!()
                };
                for rframe in 0..width {
                    let rframe = RectFrameId::from_idx(rframe);
                    for rbit in 0..height {
                        let rbit = RectBitId::from_idx(rbit);
                        let bit = btile.xlat_pos_fwd((rframe, rbit));
                        if ctx.empty_bs.get_bit(bit) {
                            diff.bits.insert(
                                TileBit {
                                    rect: BitRectId::from_idx(0),
                                    frame: rframe,
                                    bit: rbit,
                                },
                                true,
                            );
                        }
                    }
                }
                if tcid == cnr_sw {
                    for attr in [bcls::MISC_SW::SEND_VGG, bcls::MISC_SW::VGG_SENDMAX] {
                        diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslots::MISC_SW, attr));
                    }
                }
                if edev.chip.kind == ChipKind::Spartan3E {
                    diff.discard_polbits(ctx.bel_attr_bitvec(
                        tcid,
                        bslots::BANK,
                        bcls::BANK::S3E_LVDSBIAS,
                    ));
                }
                if !diff.bits.is_empty() {
                    assert_eq!(tcid, cnr_sw);
                    ctx.insert_bel_attr_bitvec(
                        tcid,
                        bslots::MISC_SW,
                        bcls::MISC_SW::UNK_ALWAYS_SET,
                        xlat_bit_wide(diff),
                    );
                }
            }
        }
    }

    // config regs
    if !edev.chip.kind.is_spartan3a() {
        let tcid = if edev.chip.kind.is_virtex2() {
            tcls_v2::GLOBAL
        } else if edev.chip.kind == ChipKind::Spartan3 {
            tcls_s3::GLOBAL_S3
        } else if edev.chip.kind == ChipKind::FpgaCore {
            tcls_s3::GLOBAL_FC
        } else {
            tcls_s3::GLOBAL_S3E
        };
        let bslot = bslots::GLOBAL;
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::GWE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::GTS_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::DONE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );

        if edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                bcls::GLOBAL::LOCK_CYCLE,
                &[
                    enums::STARTUP_CYCLE::_0,
                    enums::STARTUP_CYCLE::_1,
                    enums::STARTUP_CYCLE::_2,
                    enums::STARTUP_CYCLE::_3,
                    enums::STARTUP_CYCLE::_4,
                    enums::STARTUP_CYCLE::_5,
                    enums::STARTUP_CYCLE::_6,
                    enums::STARTUP_CYCLE::NOWAIT,
                ],
            );
        }
        if edev.chip.kind != ChipKind::Spartan3E && edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                bcls::GLOBAL::MATCH_CYCLE,
                &[
                    enums::STARTUP_CYCLE::_0,
                    enums::STARTUP_CYCLE::_1,
                    enums::STARTUP_CYCLE::_2,
                    enums::STARTUP_CYCLE::_3,
                    enums::STARTUP_CYCLE::_4,
                    enums::STARTUP_CYCLE::_5,
                    enums::STARTUP_CYCLE::_6,
                    enums::STARTUP_CYCLE::NOWAIT,
                ],
            );
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::STARTUP_CLOCK);
        if edev.chip.kind == ChipKind::Spartan3E {
            ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::MULTIBOOT_ENABLE);
        }
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::CONFIG_RATE_V2, OcdMode::BitOrder);
        } else if !edev.chip.kind.is_spartan3ea() {
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::CONFIG_RATE_S3, OcdMode::BitOrder);
        } else {
            ctx.collect_bel_attr_ocd(
                tcid,
                bslot,
                bcls::GLOBAL::CONFIG_RATE_S3E,
                OcdMode::BitOrder,
            );
        };
        if !edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::BUSCLK_FREQ, OcdMode::BitOrder);
        }
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DONE_PIPE);
        if edev.chip.kind != ChipKind::FpgaCore {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DCM_SHUTDOWN);
        }
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POWERDOWN_STATUS);
        }
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CRC_ENABLE);
        if matches!(edev.chip.kind, ChipKind::Spartan3 | ChipKind::FpgaCore) {
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::S3_VRDSEL, OcdMode::BitOrder);
        } else if edev.chip.kind == ChipKind::Spartan3E {
            // ??? 70 == 75?
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::S3E_VRDSEL, OcdMode::BitOrder);
        }

        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::CAPTURE_ONESHOT);

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::GTS_USR_B);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_TEST);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::BCLK_TEST);
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::SECURITY,
            &[
                enums::SECURITY::NONE,
                enums::SECURITY::LEVEL1,
                enums::SECURITY::LEVEL2,
            ],
        );
        // these are too much trouble to deal with the normal way.
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::PERSIST,
            TileBit::new(1, 0, 3).pos(),
        );
    } else {
        let tcid = tcls_s3::GLOBAL_S3A;
        let bslot = bslots::GLOBAL;

        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::STARTUP_CLOCK);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_AWAKE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CRC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::S3A_VRDSEL);

        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::GWE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::GTS_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::DONE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
            ],
        );

        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::LOCK_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::NOWAIT,
            ],
        );
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CAPTURE_ONESHOT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::BPI_DIV8);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::RESET_ON_ERR);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::ICAP_BYPASS);

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::GTS_USR_B);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_TEST);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::MULTIBOOT_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SECURITY);
        // too much trouble to deal with in normal ways.
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::PERSIST,
            TileBit::new(2, 0, 3).pos(),
        );
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::ICAP_ENABLE);

        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::CONFIG_RATE_DIV,
            (0..10).map(|i| TileBit::new(3, 0, i).pos()).collect(),
        );
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::CCLK_DLY, 0..4);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::CCLK_SEP, 0..4);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::CLK_SWITCH_OPT, 0..4);

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::BRAM_SKIP);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::TWO_ROUND);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::HC_CYCLE, 1..16);

        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SW_CLK);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::EN_SUSPEND);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::EN_PORB);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::EN_SW_GSR);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::SUSPEND_FILTER);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::WAKE_DELAY1, 1..8);
        ctx.collect_bel_attr_sparse(tcid, bslot, bcls::GLOBAL::WAKE_DELAY2, 1..32);

        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SW_GWE_CYCLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SW_GTS_CYCLE);

        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::BOOTVSEL);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::NEXT_CONFIG_BOOT_MODE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::NEXT_CONFIG_NEW_MODE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::TESTMODE_EN);

        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::NEXT_CONFIG_ADDR);

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::GLUTMASK);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_KEEP);

        // too much effort to include in the automatic fuzzer
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::POST_CRC_EN,
            TileBit::new(11, 0, 0).pos(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::POST_CRC_FREQ_DIV,
            (0..10).map(|i| TileBit::new(11, 0, 4 + i).pos()).collect(),
        );
    }

    if edev.chip.kind.is_virtex2() {
        let is_double_grestore = ctx.empty_bs.die[DieId::from_idx(0)]
            .regs
            .get(&Reg::FakeDoubleGrestore)
            == Some(&1);
        ctx.insert_devdata_bool(devdata::DOUBLE_GRESTORE, is_double_grestore);
    }
}
