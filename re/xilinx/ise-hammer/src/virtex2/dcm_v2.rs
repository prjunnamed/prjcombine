use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::{
    db::BelAttributeEnum,
    dir::{DirH, DirHV, DirV},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_bi,
    xlat_bit_wide, xlat_bit_wide_bi, xlat_bitvec, xlat_bitvec_sparse_u32, xlat_enum_attr,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex2::{
    chip::{ChipKind, ColumnKind},
    defs::{
        self, bcls, bslots, devdata, enums, spartan3::tcls as tcls_s3, virtex2::tcls as tcls_v2,
    },
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    virtex2::specials,
};

#[derive(Copy, Clone, Debug)]
struct DcmCornerEnable(DirHV, bool);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DcmCornerEnable {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        let required = self.1;
        let we_match = match self.0.h {
            DirH::W => tcrd.col < edev.chip.col_clk,
            DirH::E => tcrd.col >= edev.chip.col_clk,
        };
        let sn_match = match self.0.v {
            DirV::S => tcrd.row < edev.chip.row_mid(),
            DirV::N => tcrd.row >= edev.chip.row_mid(),
        };
        if we_match && sn_match {
            let col = edev.chip.col_edge(self.0.h);
            let tcrd = tcrd.with_col(col).tile(defs::tslots::BEL);
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelAttrBit(
                    edev[tcrd].class,
                    bslots::MISC_CNR_S3,
                    bcls::MISC_CNR_S3::DCM_ENABLE,
                    0,
                    true,
                ),
                rects: edev.tile_bits(tcrd),
            });
            Some((fuzzer, false))
        } else if required {
            None
        } else {
            Some((fuzzer, true))
        }
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let tcid = match edev.chip.kind {
        ChipKind::Virtex2 => tcls_v2::DCM_V2,
        ChipKind::Virtex2P | ChipKind::Virtex2PX => tcls_v2::DCM_V2P,
        ChipKind::Spartan3 => tcls_s3::DCM_S3,
        _ => unreachable!(),
    };

    if devdata_only {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(defs::bslots::DCM);
        let mode = "DCM";
        let mut builder = bctx.build().global_mutex("DCM_OPT", "NO");
        if edev.chip.kind == ChipKind::Spartan3 {
            builder = builder.prop(DcmCornerEnable(DirHV::SW, true));
        }
        builder
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        return;
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    for (val, vname) in &backend.edev.db[enums::DCM_TEST_OSC].values {
        ctx.build()
            .extra_tiles_by_bel_attr_val(defs::bslots::DCM, bcls::DCM::TEST_OSC, val)
            .test_global_special(specials::DCM_TEST_OSC)
            .global("TESTOSC", vname.strip_prefix('_').unwrap())
            .commit();
    }

    let mut ctx = FuzzCtx::new(session, backend, tcid);
    let mut bctx = ctx.bel(defs::bslots::DCM);
    let mode = "DCM";
    let mut props = vec![];
    if edev.chip.kind == ChipKind::Spartan3 {
        props.extend([
            DcmCornerEnable(DirHV::SW, false),
            DcmCornerEnable(DirHV::NW, false),
        ]);
        if edev.chip.columns[edev.chip.columns.last_id().unwrap() - 3].kind == ColumnKind::Bram {
            props.extend([
                DcmCornerEnable(DirHV::SE, false),
                DcmCornerEnable(DirHV::NE, false),
            ]);
        }
    }
    let mut builder = bctx.build().global_mutex("DCM_OPT", "NO");
    for &prop in &props {
        builder = builder.prop(prop);
    }
    builder
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();
    let mut builder = bctx
        .build()
        .global_mutex("DCM_OPT", "YES")
        .global("VBG_SEL0", "0")
        .global("VBG_SEL1", "0")
        .global("VBG_SEL2", "0")
        .global("VBG_PD0", "0")
        .global("VBG_PD1", "0");
    for &prop in &props {
        builder = builder.prop(prop);
    }
    builder
        .test_bel_special(specials::DCM_OPT_BASE)
        .mode(mode)
        .commit();

    for spec in [
        specials::DCM_VBG_SEL0,
        specials::DCM_VBG_SEL1,
        specials::DCM_VBG_SEL2,
        specials::DCM_VBG_PD0,
        specials::DCM_VBG_PD1,
    ] {
        let mut builder = bctx
            .build()
            .global_mutex("DCM_OPT", "YES")
            .global(
                "VBG_SEL0",
                if spec == specials::DCM_VBG_SEL0 {
                    "1"
                } else {
                    "0"
                },
            )
            .global(
                "VBG_SEL1",
                if spec == specials::DCM_VBG_SEL1 {
                    "1"
                } else {
                    "0"
                },
            )
            .global(
                "VBG_SEL2",
                if spec == specials::DCM_VBG_SEL2 {
                    "1"
                } else {
                    "0"
                },
            )
            .global(
                "VBG_PD0",
                if spec == specials::DCM_VBG_PD0 {
                    "1"
                } else {
                    "0"
                },
            )
            .global(
                "VBG_PD1",
                if spec == specials::DCM_VBG_PD1 {
                    "1"
                } else {
                    "0"
                },
            );
        for &prop in &props {
            builder = builder.prop(prop);
        }
        builder.test_bel_special(spec).mode(mode).commit();
    }

    for pin in [
        bcls::DCM::RST,
        bcls::DCM::PSCLK,
        bcls::DCM::PSEN,
        bcls::DCM::PSINCDEC,
        bcls::DCM::DSSEN,
    ] {
        bctx.mode(mode)
            .global_mutex("PSCLK", "DCM")
            .mutex("MODE", "SIMPLE")
            .test_bel_input_inv_auto(pin);
    }
    for pin in [
        bcls::DCM::CTLMODE,
        bcls::DCM::CTLSEL[0],
        bcls::DCM::CTLSEL[1],
        bcls::DCM::CTLSEL[2],
        bcls::DCM::CTLOSC1,
        bcls::DCM::CTLOSC2,
        bcls::DCM::CTLGO,
        bcls::DCM::STSADRS[0],
        bcls::DCM::STSADRS[1],
        bcls::DCM::STSADRS[2],
        bcls::DCM::STSADRS[3],
        bcls::DCM::STSADRS[4],
        bcls::DCM::FREEZEDFS,
        bcls::DCM::FREEZEDLL,
    ] {
        if pin == bcls::DCM::STSADRS[4] && edev.chip.kind == ChipKind::Virtex2 {
            continue;
        }
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .mutex("INV", pin)
            .test_bel_input_inv_auto(pin);
    }

    for (attr, pin) in [
        (bcls::DCM::OUT_CLK0_ENABLE, "CLK0"),
        (bcls::DCM::OUT_CLK90_ENABLE, "CLK90"),
        (bcls::DCM::OUT_CLK180_ENABLE, "CLK180"),
        (bcls::DCM::OUT_CLK270_ENABLE, "CLK270"),
        (bcls::DCM::OUT_CLK2X_ENABLE, "CLK2X"),
        (bcls::DCM::OUT_CLK2X180_ENABLE, "CLK2X180"),
        (bcls::DCM::OUT_CLKDV_ENABLE, "CLKDV"),
        (bcls::DCM::OUT_CLKFX_ENABLE, "CLKFX"),
        (bcls::DCM::OUT_CLKFX180_ENABLE, "CLKFX180"),
        (bcls::DCM::OUT_CONCUR_ENABLE, "CONCUR"),
    ] {
        bctx.mode(mode)
            .mutex("MODE", "PINS")
            .mutex("PIN", pin)
            .no_pin("CLKFB")
            .test_bel_attr_bits(attr)
            .pin(pin)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "PINS")
            .mutex("PIN", pin)
            .pin("CLKFB")
            .test_bel_attr_special(attr, specials::DCM_PIN_CLKFB)
            .pin(pin)
            .commit();
        if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
            bctx.mode(mode)
                .mutex("MODE", "PINS")
                .mutex("PIN", format!("{pin}.CLKFX"))
                .pin("CLKFX")
                .pin("CLKFB")
                .test_bel_attr_special(attr, specials::DCM_PIN_CLKFX)
                .pin(pin)
                .commit();
        }
    }
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bits(bcls::DCM::CLKFB_ENABLE)
        .pin("CLKFB")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_bel_attr_bits(bcls::DCM::CLKIN_IOB)
        .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKIN", PinFromKind::Bufg)
        .test_bel_attr_bits(bcls::DCM::CLKFB_IOB)
        .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();
    for (attr, pin) in [
        (bcls::DCM::STATUS1_ENABLE, "STATUS1"),
        (bcls::DCM::STATUS7_ENABLE, "STATUS7"),
    ] {
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .test_bel_attr_bits(attr)
            .pin(pin)
            .commit();
    }
    for pin in [
        "STATUS0", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6",
    ] {
        bctx.mode(mode)
            .null_bits()
            .mutex("MODE", "SIMPLE")
            .test_bel_special(specials::DCM_PIN_DUMMY)
            .pin(pin)
            .commit();
    }

    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr(bcls::DCM::DLL_FREQUENCY_MODE);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr(bcls::DCM::DFS_FREQUENCY_MODE);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .global("GTS_CYCLE", "1")
        .global("DONE_CYCLE", "1")
        .global("LCK_CYCLE", "NOWAIT")
        .test_bel_attr_bits(bcls::DCM::STARTUP_WAIT)
        .attr("STARTUP_WAIT", "STARTUP_WAIT")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bool_rename(
            "DUTY_CYCLE_CORRECTION",
            bcls::DCM::V2_DUTY_CYCLE_CORRECTION,
            "FALSE",
            "TRUE",
        );
    for (val, vname) in [
        (0x00, "0X80"),
        (0x40, "0XC0"),
        (0x60, "0XE0"),
        (0x70, "0XF0"),
        (0x78, "0XF8"),
        (0x7c, "0XFC"),
        (0x7e, "0XFE"),
        (0x7f, "0XFF"),
    ] {
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .test_bel_attr_u32(bcls::DCM::FACTORY_JF1, val)
            .attr("FACTORY_JF1", vname)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .test_bel_attr_u32(bcls::DCM::FACTORY_JF2, val)
            .attr("FACTORY_JF2", vname)
            .commit();
    }
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bits(bcls::DCM::DESKEW_ADJUST)
        .multi_attr("DESKEW_ADJUST", MultiValue::Dec(0), 4);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bits(bcls::DCM::CLKIN_DIVIDE_BY_2)
        .attr("CLKIN_DIVIDE_BY_2", "CLKIN_DIVIDE_BY_2")
        .commit();
    bctx.mode(mode)
        .attr("DUTY_CYCLE_CORRECTION", "#OFF")
        .mutex("MODE", "SIMPLE")
        .pin("CLK0")
        .test_bel_special(specials::DCM_VERY_HIGH_FREQUENCY)
        .attr("VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_bel_attr(bcls::DCM::DSS_MODE);
    bctx.mode(mode)
        .null_bits()
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_bel_attr_bits_bi(bcls::DCM::DSS_ENABLE, false)
        .attr("DSS_MODE", "NONE")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bool_rename("CLK_FEEDBACK", bcls::DCM::CLK_FEEDBACK_2X, "1X", "2X");
    for (spec, val) in [
        (specials::DCM_CLKOUT_PHASE_SHIFT_NONE, "NONE"),
        (specials::DCM_CLKOUT_PHASE_SHIFT_FIXED, "FIXED"),
        (specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE, "VARIABLE"),
    ] {
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .attr("PHASE_SHIFT", "1")
            .pin("CLK0")
            .test_bel_special(spec)
            .attr("CLKOUT_PHASE_SHIFT", val)
            .commit();
    }
    for (spec, val) in [
        (specials::DCM_CLKOUT_PHASE_SHIFT_FIXED_NEG, "FIXED"),
        (specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_NEG, "VARIABLE"),
    ] {
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .attr("PHASE_SHIFT", "-1")
            .pin("CLK0")
            .test_bel_special(spec)
            .attr("CLKOUT_PHASE_SHIFT", val)
            .commit();
    }
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bits(bcls::DCM::V2_CLKFX_MULTIPLY)
        .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 12);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_bel_attr_bits(bcls::DCM::V2_CLKFX_DIVIDE)
        .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 12);

    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "FIXED")
        .test_bel_attr_bits(bcls::DCM::PHASE_SHIFT)
        .multi_attr("PHASE_SHIFT", MultiValue::Dec(0), 8);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "FIXED")
        .test_bel_special(specials::DCM_PHASE_SHIFT_N255_FIXED)
        .attr("PHASE_SHIFT", "-255")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "VARIABLE")
        .test_bel_special(specials::DCM_PHASE_SHIFT_N255_VARIABLE)
        .attr("PHASE_SHIFT", "-255")
        .commit();

    for val in 2..=16 {
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .test_bel_special_u32(specials::DCM_CLKDV_DIVIDE_INT, val)
            .attr("CLKDV_DIVIDE", val.to_string())
            .commit();
    }
    for (spec, dll_mode) in [
        (specials::DCM_CLKDV_DIVIDE_HALF_LOW, "LOW"),
        (specials::DCM_CLKDV_DIVIDE_HALF_HIGH, "HIGH"),
    ] {
        for val in 1..=7 {
            bctx.mode(mode)
                .mutex("MODE", "SIMPLE")
                .attr("DLL_FREQUENCY_MODE", dll_mode)
                .test_bel_special_u32(spec, val)
                .attr("CLKDV_DIVIDE", format!("{val}_5"))
                .commit();
        }
    }

    bctx.mode(mode)
        .mutex("MODE", "LL_DLLC")
        .no_global("TESTOSC")
        .pin("STATUS1")
        .pin("STATUS7")
        .test_bel_attr_bits(bcls::DCM::V2_REG_DLLC)
        .multi_attr("LL_HEX_DLLC", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_DLLS")
        .test_bel_attr_bits(bcls::DCM::V2_REG_DLLS)
        .multi_attr("LL_HEX_DLLS", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_DFS")
        .test_bel_attr_bits(bcls::DCM::V2_REG_DFS)
        .multi_attr("LL_HEX_DFS", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_COM")
        .test_bel_attr_bits(bcls::DCM::V2_REG_COM)
        .multi_attr("LL_HEX_COM", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_MISC")
        .test_bel_attr_bits(bcls::DCM::V2_REG_MISC)
        .multi_attr("LL_HEX_MISC", MultiValue::Hex(0), 32);
    for val in 0..4 {
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_bel_attr_bitvec_u32(bcls::DCM::COIN_WINDOW, val)
            .global_xy("COINWINDOW_*", val.to_string())
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_bel_attr_bitvec_u32(bcls::DCM::SEL_PL_DLY, val)
            .global_xy("SELPLDLY_*", val.to_string())
            .commit();
    }
    for (val, vname) in [(false, "0"), (true, "1")] {
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_bel_attr_bits_bi(bcls::DCM::EN_OSC_COARSE, val)
            .global_xy("ENOSCCOARSE_*", vname)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .global_xy("NONSTOP_*", "0")
            .test_bel_attr_bits_bi(bcls::DCM::S3_EN_DUMMY_OSC, val)
            .global_xy("ENDUMMYOSC_*", vname)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_bel_attr_bits_bi(bcls::DCM::PL_CENTERED, val)
            .global_xy("PLCENTERED_*", vname)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .global_xy("ENDUMMYOSC_*", "0")
            .test_bel_attr_bits_bi(bcls::DCM::NON_STOP, val)
            .global_xy("NONSTOP_*", vname)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .mutex("ZD2", "PLAIN")
            .test_bel_attr_bits_bi(bcls::DCM::ZD2_BY1, val)
            .global_xy("ZD2_BY1_*", vname)
            .commit();
        if edev.chip.kind.is_virtex2() {
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::PS_CENTERED, val)
                .global_xy("CENTERED_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .mutex("ZD2", "HF")
                .test_bel_attr_bits_bi(bcls::DCM::ZD2_BY1, val)
                .global_xy("ZD2_HF_BY1_*", vname)
                .commit();
        }
        if edev.chip.kind != ChipKind::Virtex2 {
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::ZD1_BY1, val)
                .global_xy("ZD1_BY1_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::RESET_PS_SEL, val)
                .global_xy("RESETPS_SEL_*", vname)
                .commit();
        }
        if edev.chip.kind == ChipKind::Spartan3 {
            for i in 0..2 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_bel_attr_bits_base_bi(bcls::DCM::SPLY_IDC, i, val)
                    .global_xy(format!("SPLY_IDC{i}_*"), vname)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::EXTENDED_FLUSH_TIME, val)
                .global_xy("EXTENDEDFLUSHTIME_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::EXTENDED_HALT_TIME, val)
                .global_xy("EXTENDEDHALTTIME_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::EXTENDED_RUN_TIME, val)
                .global_xy("EXTENDEDRUNTIME_*", vname)
                .commit();
            for i in 0..=8 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_bel_attr_bits_base_bi(bcls::DCM::CFG_DLL_PS, i, val)
                    .global_xy(format!("CFG_DLL_PS{i}_*"), vname)
                    .commit();
            }
            for i in 0..=2 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_bel_attr_bits_base_bi(bcls::DCM::CFG_DLL_LP, i, val)
                    .global_xy(format!("CFG_DLL_LP{i}_*"), vname)
                    .commit();
            }
            for i in 0..=1 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_bel_attr_bits_base_bi(bcls::DCM::SEL_HSYNC_B, i, val)
                    .global_xy(format!("SELHSYNC_B{i}_*"), vname)
                    .commit();
            }
            for i in 0..=1 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_bel_attr_bits_base_bi(bcls::DCM::LPON_B_DFS, i, val)
                    .global_xy(format!("LPON_B_DFS{i}_*"), vname)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::EN_PWCTL, val)
                .global_xy("ENPWCTL_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::M1D1, val)
                .global_xy("M1D1_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::MIS1, val)
                .global_xy("MIS1_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::EN_RELRST_B, val)
                .global_xy("ENRELRST_B_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::EN_OLD_OSCCTL, val)
                .global_xy("ENOLDOSCCTL_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::TRIM_LP_B, val)
                .global_xy("TRIM_LP_B_*", vname)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_bel_attr_bits_bi(bcls::DCM::INVERT_ZD1_CUSTOM, val)
                .global_xy("INVERT_ZD1_CUSTOM_*", vname)
                .commit();
            for i in 0..=4 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_bel_attr_bits_base_bi(bcls::DCM::VREG_PROBE, i, val)
                    .global_xy(format!("VREG_PROBE{i}_*"), vname)
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let tcid = match edev.chip.kind {
        ChipKind::Virtex2 => tcls_v2::DCM_V2,
        ChipKind::Virtex2P | ChipKind::Virtex2PX => tcls_v2::DCM_V2P,
        ChipKind::Spartan3 => tcls_s3::DCM_S3,
        _ => unreachable!(),
    };
    let bslot = bslots::DCM;

    if devdata_only {
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let item = ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
        let val = extract_bitvec_val(
            item,
            &BitVec::repeat(false, 4),
            present.split_bits(&item.iter().map(|bit| bit.bit).collect()),
        );
        ctx.insert_devdata_bitvec(devdata::DCM_DESKEW_ADJUST, val);
        let vbg_sel = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_VBG_SEL),
            &BitVec::repeat(false, 3),
            &mut present,
        );
        let vbg_pd = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_VBG_PD),
            &BitVec::repeat(false, 2),
            &mut present,
        );
        ctx.insert_devdata_bitvec(devdata::DCM_V2_VBG_SEL, vbg_sel);
        ctx.insert_devdata_bitvec(devdata::DCM_V2_VBG_PD, vbg_pd);
        if edev.chip.kind == ChipKind::Spartan3 {
            ctx.collect_bel_attr(
                tcls_s3::CNR_SW_S3,
                bslots::MISC_CNR_S3,
                bcls::MISC_CNR_S3::DCM_ENABLE,
            );
        }
        return;
    }

    let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
    let dllc = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::V2_REG_DLLC, 32);
    let dlls = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::V2_REG_DLLS, 32);
    let dfs = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::V2_REG_DFS, 32);
    let mut com = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::V2_REG_COM, 32);
    let mut misc = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::V2_REG_MISC, 32);

    // sigh. fixups.
    assert!(com[11].bits.is_empty());
    let com9 = *com[9].bits.keys().next().unwrap();
    let com11 = TileBit {
        bit: com9.bit + 2,
        ..com9
    };
    assert_eq!(com[10].bits.remove(&com11), Some(true));
    com[11].bits.insert(com11, true);

    if edev.chip.kind == ChipKind::Spartan3 {
        for diff in &misc[12..31] {
            assert!(diff.bits.is_empty());
        }
        misc.truncate(12);
    }

    let dllc = xlat_bitvec(dllc);
    let dlls = xlat_bitvec(dlls);
    let dfs = xlat_bitvec(dfs);
    let com = xlat_bitvec(com);
    let misc = xlat_bitvec(misc);

    let base = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_OPT_BASE);
    for (attr, specials) in [
        (
            bcls::DCM::V2_VBG_SEL,
            [
                specials::DCM_VBG_SEL0,
                specials::DCM_VBG_SEL1,
                specials::DCM_VBG_SEL2,
            ]
            .as_slice(),
        ),
        (
            bcls::DCM::V2_VBG_PD,
            [specials::DCM_VBG_PD0, specials::DCM_VBG_PD1].as_slice(),
        ),
    ] {
        let mut diffs = vec![];
        for &spec in specials {
            diffs.push(ctx.get_diff_bel_special(tcid, bslot, spec).combine(&!&base));
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr, xlat_bitvec(diffs));
    }
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::TEST_OSC);

    ctx.collect_bel_attr_sparse(tcid, bslot, bcls::DCM::COIN_WINDOW, 0..4);
    ctx.collect_bel_attr_sparse(tcid, bslot, bcls::DCM::SEL_PL_DLY, 0..4);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EN_OSC_COARSE);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::PL_CENTERED);
    if edev.chip.kind.is_virtex2() {
        ctx.get_diff_attr_bit_bi(tcid, bslot, bcls::DCM::NON_STOP, 0, false)
            .assert_empty();
        ctx.get_diff_attr_bit_bi(tcid, bslot, bcls::DCM::S3_EN_DUMMY_OSC, 0, true)
            .assert_empty();
        let en_dummy_osc =
            !ctx.get_diff_attr_bit_bi(tcid, bslot, bcls::DCM::S3_EN_DUMMY_OSC, 0, false);
        let non_stop = ctx.get_diff_attr_bit_bi(tcid, bslot, bcls::DCM::NON_STOP, 0, true);
        let (en_dummy_osc, non_stop, common) = Diff::split(en_dummy_osc, non_stop);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::NON_STOP, xlat_bit(non_stop));
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCM::V2_EN_DUMMY_OSC,
            xlat_bit_wide(en_dummy_osc),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::DCM::EN_DUMMY_OSC_OR_NON_STOP,
            xlat_bit(common),
        );
    } else {
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::S3_EN_DUMMY_OSC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::NON_STOP);
    }
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::ZD2_BY1);
    if edev.chip.kind.is_virtex2() {
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::PS_CENTERED);
    }
    if edev.chip.kind != ChipKind::Virtex2 {
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::ZD1_BY1);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::RESET_PS_SEL);
    }
    if edev.chip.kind == ChipKind::Spartan3 {
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EXTENDED_FLUSH_TIME);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EXTENDED_HALT_TIME);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EXTENDED_RUN_TIME);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::M1D1);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::MIS1);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EN_OLD_OSCCTL);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EN_PWCTL);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::EN_RELRST_B);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::INVERT_ZD1_CUSTOM);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::TRIM_LP_B);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::SPLY_IDC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::VREG_PROBE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::CFG_DLL_PS);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::CFG_DLL_LP);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::SEL_HSYNC_B);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::LPON_B_DFS);
    }

    let int_tiles = &[match edev.chip.kind {
        ChipKind::Virtex2 => tcls_v2::INT_DCM_V2,
        ChipKind::Virtex2P | ChipKind::Virtex2PX => tcls_v2::INT_DCM_V2P,
        ChipKind::Spartan3 => tcls_s3::INT_DCM,
        _ => unreachable!(),
    }];
    ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::DCM::PSCLK);
    for pin in [bcls::DCM::RST, bcls::DCM::PSEN, bcls::DCM::PSINCDEC] {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    if edev.chip.kind == ChipKind::Spartan3 {
        ctx.get_diff_bel_input_inv(tcid, bslot, bcls::DCM::DSSEN, false)
            .assert_empty();
        ctx.get_diff_bel_input_inv(tcid, bslot, bcls::DCM::DSSEN, true)
            .assert_empty();
    } else {
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::DCM::DSSEN);
    }
    for pin in [
        bcls::DCM::CTLMODE,
        bcls::DCM::CTLSEL[0],
        bcls::DCM::CTLSEL[1],
        bcls::DCM::CTLSEL[2],
        bcls::DCM::CTLOSC1,
        bcls::DCM::CTLOSC2,
        bcls::DCM::CTLGO,
        bcls::DCM::STSADRS[0],
        bcls::DCM::STSADRS[1],
        bcls::DCM::STSADRS[2],
        bcls::DCM::STSADRS[3],
        bcls::DCM::STSADRS[4],
        bcls::DCM::FREEZEDFS,
        bcls::DCM::FREEZEDLL,
    ] {
        if pin == bcls::DCM::STSADRS[4] && edev.chip.kind == ChipKind::Virtex2 {
            continue;
        }
        let d0 = ctx.get_diff_bel_input_inv(tcid, bslot, pin, false);
        let d1 = ctx.get_diff_bel_input_inv(tcid, bslot, pin, true);
        let (d0, d1, dc) = Diff::split(d0, d1);
        ctx.insert_bel_input_inv(tcid, bslot, pin, xlat_bit_bi(d0, d1));
        if edev.chip.kind.is_virtex2() {
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::TEST_ENABLE, xlat_bit(dc));
        } else {
            dc.assert_empty();
        }
    }
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::STATUS1_ENABLE);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::STATUS7_ENABLE);
    let (_, _, en_dll) = Diff::split(
        ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLK0_ENABLE, 0)
            .clone(),
        ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLK90_ENABLE, 0)
            .clone(),
    );
    let (_, _, en_dfs) = Diff::split(
        ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLKFX_ENABLE, 0)
            .clone(),
        ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLKFX180_ENABLE, 0)
            .clone(),
    );
    let vhf = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VERY_HIGH_FREQUENCY);
    assert_eq!(en_dll, !vhf);
    for attr in [
        bcls::DCM::OUT_CLK0_ENABLE,
        bcls::DCM::OUT_CLK90_ENABLE,
        bcls::DCM::OUT_CLK180_ENABLE,
        bcls::DCM::OUT_CLK270_ENABLE,
        bcls::DCM::OUT_CLK2X_ENABLE,
        bcls::DCM::OUT_CLK2X180_ENABLE,
        bcls::DCM::OUT_CLKDV_ENABLE,
    ] {
        let diff = ctx.get_diff_attr_bit(tcid, bslot, attr, 0);
        let diff_fb = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFB);
        let diff_fx = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFX);
        assert_eq!(diff, diff_fb);
        assert_eq!(diff, diff_fx);
        let diff = diff.combine(&!&en_dll);
        ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
    }
    for attr in [
        bcls::DCM::OUT_CLKFX_ENABLE,
        bcls::DCM::OUT_CLKFX180_ENABLE,
        bcls::DCM::OUT_CONCUR_ENABLE,
    ] {
        let diff = ctx.get_diff_attr_bit(tcid, bslot, attr, 0);
        let diff_fb = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFB);
        let diff_fb = diff_fb.combine(&!&diff);
        let diff = diff.combine(&!&en_dfs);
        ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DFS_FEEDBACK, xlat_bit(diff_fb));
    }
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DLL_ENABLE, xlat_bit(en_dll));
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DFS_ENABLE, xlat_bit(en_dfs));
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKFB_ENABLE);

    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKIN_IOB);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKFB_IOB);

    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::V2_CLKFX_MULTIPLY);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::V2_CLKFX_DIVIDE);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::PHASE_SHIFT);
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N255_VARIABLE);
    diff.apply_bitvec_diff(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::PHASE_SHIFT),
        &BitVec::repeat(true, 8),
        &BitVec::repeat(false, 8),
    );
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::PHASE_SHIFT_NEGATIVE, xlat_bit(diff));

    ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_NONE)
        .assert_empty();
    let fixed = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_FIXED);
    let fixed_n = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_FIXED_NEG);
    let variable = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE);
    let variable_n =
        ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE_NEG);
    assert_eq!(variable, variable_n);
    let fixed_n = fixed_n.combine(&!&fixed);
    let (fixed, variable, en_ps) = Diff::split(fixed, variable);
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::PS_ENABLE, xlat_bit(en_ps));
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        bcls::DCM::PS_CENTERED,
        xlat_bit_bi(fixed, variable),
    );

    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N255_FIXED);
    diff.apply_bitvec_diff(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::PHASE_SHIFT),
        &BitVec::repeat(true, 8),
        &BitVec::repeat(false, 8),
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::DCM::PHASE_SHIFT_NEGATIVE),
        true,
        false,
    );
    assert_eq!(diff, fixed_n);
    if edev.chip.kind != ChipKind::Virtex2 {
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::RESET_PS_SEL),
            true,
            false,
        );
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        bcls::DCM::PS_MODE,
        xlat_enum_attr(vec![
            (enums::DCM_PS_MODE::CLKFB, diff),
            (enums::DCM_PS_MODE::CLKIN, Diff::default()),
        ]),
    );

    let item = xlat_bit_wide_bi(
        ctx.get_diff_attr_bit_bi(tcid, bslot, bcls::DCM::V2_DUTY_CYCLE_CORRECTION, 0, false),
        ctx.get_diff_attr_bit_bi(tcid, bslot, bcls::DCM::V2_DUTY_CYCLE_CORRECTION, 0, true),
    );
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_DUTY_CYCLE_CORRECTION, item);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::STARTUP_WAIT);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKIN_DIVIDE_BY_2);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::CLK_FEEDBACK_2X);
    ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DFS_FREQUENCY_MODE);
    let low = ctx.get_diff_attr_val(
        tcid,
        bslot,
        bcls::DCM::DLL_FREQUENCY_MODE,
        enums::DCM_FREQUENCY_MODE::LOW,
    );
    let mut high = ctx.get_diff_attr_val(
        tcid,
        bslot,
        bcls::DCM::DLL_FREQUENCY_MODE,
        enums::DCM_FREQUENCY_MODE::HIGH,
    );
    if edev.chip.kind.is_virtex2p() {
        high.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::ZD2_BY1),
            true,
            false,
        );
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        bcls::DCM::DLL_FREQUENCY_MODE,
        xlat_enum_attr(vec![
            (enums::DCM_FREQUENCY_MODE::LOW, low),
            (enums::DCM_FREQUENCY_MODE::HIGH, high),
        ]),
    );

    for (attr, range) in [
        (bcls::DCM::FACTORY_JF1, 8..16),
        (bcls::DCM::FACTORY_JF2, 0..8),
    ] {
        let mut diffs = vec![];
        for val in [0x00, 0x40, 0x60, 0x70, 0x78, 0x7c, 0x7e, 0x7f] {
            let diff = ctx.get_diff_attr_u32(tcid, bslot, attr, val);
            diffs.push((val, diff));
        }
        let bits = xlat_bitvec_sparse_u32(diffs);
        assert_eq!(bits, dlls[range.start..(range.end - 1)]);
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr, dlls[range].to_vec());
    }

    for (attr, bits) in [
        (bcls::DCM::CLKDV_COUNT_MAX, dllc[4..8].to_vec()),
        (bcls::DCM::CLKDV_COUNT_FALL, dllc[8..12].to_vec()),
        (bcls::DCM::CLKDV_COUNT_FALL_2, dllc[12..16].to_vec()),
        (bcls::DCM::CLKDV_PHASE_RISE, dllc[16..18].to_vec()),
        (bcls::DCM::CLKDV_PHASE_FALL, dllc[18..20].to_vec()),
    ] {
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits);
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        bcls::DCM::CLKDV_MODE,
        BelAttributeEnum {
            bits: vec![dllc[20].bit],
            values: EntityPartVec::from_iter([
                (enums::DCM_CLKDV_MODE::HALF, bits![0]),
                (enums::DCM_CLKDV_MODE::INT, bits![1]),
            ]),
        },
    );

    let clkdv_count_max = ctx
        .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_MAX)
        .to_vec();
    let clkdv_count_fall = ctx
        .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_FALL)
        .to_vec();
    let clkdv_count_fall_2 = ctx
        .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_FALL_2)
        .to_vec();
    let clkdv_phase_fall = ctx
        .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_PHASE_FALL)
        .to_vec();
    let clkdv_mode = ctx
        .bel_attr_enum(tcid, bslot, bcls::DCM::CLKDV_MODE)
        .clone();
    for i in 2..=16 {
        let mut diff =
            ctx.get_diff_bel_special_u32(tcid, bslot, specials::DCM_CLKDV_DIVIDE_INT, i as u32);
        diff.apply_bitvec_diff_int(&clkdv_count_max, i - 1, 1);
        diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }
    for i in 1..=7 {
        let mut diff = ctx.get_diff_bel_special_u32(
            tcid,
            bslot,
            specials::DCM_CLKDV_DIVIDE_HALF_LOW,
            i as u32,
        );
        diff.apply_enum_diff(
            &clkdv_mode,
            enums::DCM_CLKDV_MODE::HALF,
            enums::DCM_CLKDV_MODE::INT,
        );
        diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(&clkdv_count_fall, i / 2, 0);
        diff.apply_bitvec_diff_int(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
        diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special_u32(
            tcid,
            bslot,
            specials::DCM_CLKDV_DIVIDE_HALF_HIGH,
            i as u32,
        );
        diff.apply_enum_diff(
            &clkdv_mode,
            enums::DCM_CLKDV_MODE::HALF,
            enums::DCM_CLKDV_MODE::INT,
        );
        diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
        diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }

    if edev.chip.kind.is_virtex2() {
        let mut dss_base = ctx
            .peek_diff_attr_val(
                tcid,
                bslot,
                bcls::DCM::DSS_MODE,
                enums::DCM_DSS_MODE::SPREAD_2,
            )
            .clone();
        let mut diffs = vec![];
        for val in [
            enums::DCM_DSS_MODE::SPREAD_2,
            enums::DCM_DSS_MODE::SPREAD_4,
            enums::DCM_DSS_MODE::SPREAD_6,
            enums::DCM_DSS_MODE::SPREAD_8,
        ] {
            diffs.push((
                val,
                ctx.get_diff_attr_val(tcid, bslot, bcls::DCM::DSS_MODE, val)
                    .combine(&!&dss_base),
            ));
        }
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::DCM::DSS_MODE, xlat_enum_attr(diffs));
        dss_base.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::PS_ENABLE),
            true,
            false,
        );
        dss_base.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::PS_CENTERED),
            true,
            false,
        );
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DSS_ENABLE, xlat_bit(dss_base));
    } else {
        for val in [
            enums::DCM_DSS_MODE::SPREAD_2,
            enums::DCM_DSS_MODE::SPREAD_4,
            enums::DCM_DSS_MODE::SPREAD_6,
            enums::DCM_DSS_MODE::SPREAD_8,
        ] {
            ctx.get_diff_attr_val(tcid, bslot, bcls::DCM::DSS_MODE, val)
                .assert_empty();
        }
    }

    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_DLLC, dllc);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_DLLS, dlls);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_DFS, dfs);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_COM, com);
    if edev.chip.kind.is_virtex2() {
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_MISC, misc);
    } else {
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3_REG_MISC, misc);
    }

    present.apply_bitvec_diff(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::FACTORY_JF2),
        &bits![0, 0, 0, 0, 0, 0, 0, 1],
        &bits![0, 0, 0, 0, 0, 0, 0, 0],
    );
    present.apply_bitvec_diff(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::FACTORY_JF1),
        &bits![0, 0, 0, 0, 0, 0, 1, 1],
        &bits![0, 0, 0, 0, 0, 0, 0, 0],
    );
    let vbg_sel = extract_bitvec_val_part(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_VBG_SEL),
        &BitVec::repeat(false, 3),
        &mut present,
    );
    let vbg_pd = extract_bitvec_val_part(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_VBG_PD),
        &BitVec::repeat(false, 2),
        &mut present,
    );
    ctx.insert_devdata_bitvec(devdata::DCM_V2_VBG_SEL, vbg_sel);
    ctx.insert_devdata_bitvec(devdata::DCM_V2_VBG_PD, vbg_pd);
    for attr in [bcls::DCM::V2_CLKFX_MULTIPLY, bcls::DCM::V2_CLKFX_DIVIDE] {
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, attr),
            &BitVec::repeat(true, 12),
            &BitVec::repeat(false, 12),
        );
    }

    let item = ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
    let val = extract_bitvec_val(
        item,
        &BitVec::repeat(false, 4),
        present.split_bits(&item.iter().map(|bit| bit.bit).collect()),
    );
    ctx.insert_devdata_bitvec(devdata::DCM_DESKEW_ADJUST, val);

    present.apply_bitvec_diff(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_DUTY_CYCLE_CORRECTION),
        &BitVec::repeat(true, 4),
        &BitVec::repeat(false, 4),
    );

    if edev.chip.kind.is_virtex2() {
        present.apply_bit_diff(
            ctx.bel_input_inv(tcid, bslot, bcls::DCM::DSSEN),
            false,
            true,
        );
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::EN_OSC_COARSE),
            true,
            false,
        );
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_EN_DUMMY_OSC),
            &bits![1, 1, 1],
            &bits![0, 0, 0],
        );
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::EN_DUMMY_OSC_OR_NON_STOP),
            true,
            false,
        );
        if !edev.chip.kind.is_virtex2p() {
            present.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::DCM::ZD2_BY1),
                true,
                false,
            );
        }
    } else {
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::EN_PWCTL),
            true,
            false,
        );
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::SEL_HSYNC_B),
            &bits![0, 1],
            &bits![0, 0],
        );
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::CFG_DLL_PS),
            &bits![0, 1, 1, 0, 1, 0, 0, 1, 0],
            &BitVec::repeat(false, 9),
        );
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::ZD1_BY1),
            true,
            false,
        );
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::ZD2_BY1),
            true,
            false,
        );
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::S3_EN_DUMMY_OSC),
            true,
            false,
        );
    }
    present.discard_bits(&ctx.bel_attr_enum(tcid, bslot, bcls::DCM::PS_MODE).bits);
    if edev.chip.kind == ChipKind::Spartan3 {
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::PS_CENTERED),
            true,
            false,
        );
    }
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_MAX),
        1,
        0,
    );
    present.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, bcls::DCM::CLKDV_MODE),
        enums::DCM_CLKDV_MODE::INT,
        enums::DCM_CLKDV_MODE::HALF,
    );

    if edev.chip.kind.is_virtex2() {
        present.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_MISC),
            1,
            0,
        );
    } else {
        present.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3_REG_MISC),
            1,
            0,
        );
    }
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_DFS),
        1 << 26,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::V2_REG_COM),
        0x800a0a,
        0,
    );

    present.assert_empty();

    if edev.chip.kind == ChipKind::Spartan3 {
        ctx.collect_bel_attr(
            tcls_s3::CNR_SW_S3,
            bslots::MISC_CNR_S3,
            bcls::MISC_CNR_S3::DCM_ENABLE,
        );
        ctx.collect_bel_attr(
            tcls_s3::CNR_NW_S3,
            bslots::MISC_CNR_S3,
            bcls::MISC_CNR_S3::DCM_ENABLE,
        );
        if edev.chip.columns[edev.chip.columns.last_id().unwrap() - 3].kind == ColumnKind::Bram {
            ctx.collect_bel_attr(
                tcls_s3::CNR_SE_S3,
                bslots::MISC_CNR_S3,
                bcls::MISC_CNR_S3::DCM_ENABLE,
            );
            ctx.collect_bel_attr(
                tcls_s3::CNR_NE_S3,
                bslots::MISC_CNR_S3,
                bcls::MISC_CNR_S3::DCM_ENABLE,
            );
        }
    }
}
