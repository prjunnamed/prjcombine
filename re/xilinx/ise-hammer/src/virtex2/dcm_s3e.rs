use std::collections::HashSet;

use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::db::BelAttributeEnum;
use prjcombine_re_collector::diff::{Diff, extract_bitvec_val, xlat_bit, xlat_bitvec};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec};
use prjcombine_virtex2::{
    chip::Dcms,
    defs::{self, bcls, bslots, devdata, enums, spartan3::tcls},
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::DeltaSlot,
    },
    virtex2::specials,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    if devdata_only {
        let mut ctx = FuzzCtx::new(session, backend, tcls::DCM_S3E_NE);
        let mut bctx = ctx.bel(bslots::DCM);
        bctx.build()
            .global_mutex("DCM", "ENABLE")
            .test_bel_special(specials::PRESENT)
            .mode("DCM")
            .commit();
        return;
    }
    for (tcid, vreg) in [
        (
            tcls::DCM_S3E_SW,
            Some((tcls::DCM_S3E_SE, DeltaSlot::new(1, 0, defs::tslots::BEL))),
        ),
        (tcls::DCM_S3E_SE, None),
        (
            tcls::DCM_S3E_NW,
            Some((tcls::DCM_S3E_NE, DeltaSlot::new(1, 0, defs::tslots::BEL))),
        ),
        (tcls::DCM_S3E_NE, None),
        (
            tcls::DCM_S3E_WS,
            Some((tcls::DCM_S3E_WN, DeltaSlot::new(0, 1, defs::tslots::BEL))),
        ),
        (tcls::DCM_S3E_WN, None),
        (tcls::DCM_S3E_ES, None),
        (
            tcls::DCM_S3E_EN,
            Some((tcls::DCM_S3E_ES, DeltaSlot::new(0, -1, defs::tslots::BEL))),
        ),
    ] {
        let vreg_tcid = if let Some((vreg, _)) = vreg {
            vreg
        } else {
            tcid
        };
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::DCM);
        let mode = "DCM";

        let mut builder = bctx.build().global_mutex("DCM", "ENABLE").global_mutex(
            backend.edev.db.tile_classes.key(vreg_tcid),
            backend.edev.db.tile_classes.key(tcid),
        );
        if let Some((_, ref vreg)) = vreg {
            builder = builder.extra_tile_bel_special(
                vreg.clone(),
                bslots::DCM,
                specials::DCM_VREG_ENABLE,
            );
        }
        builder
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();

        if vreg.is_none() {
            bctx.build()
                .global_mutex("DCM", "ENABLE_OPT")
                .global("VBG_SEL0", "0")
                .global("VBG_SEL1", "0")
                .global("VBG_SEL2", "0")
                .global("VBG_SEL3", "0")
                .test_bel_special(specials::DCM_OPT_BASE)
                .mode(mode)
                .commit();
            for spec in [
                specials::DCM_VBG_SEL0,
                specials::DCM_VBG_SEL1,
                specials::DCM_VBG_SEL2,
                specials::DCM_VBG_SEL3,
            ] {
                bctx.build()
                    .global_mutex("DCM", "ENABLE_OPT")
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
                        "VBG_SEL3",
                        if spec == specials::DCM_VBG_SEL3 {
                            "1"
                        } else {
                            "0"
                        },
                    )
                    .test_bel_special(spec)
                    .mode(mode)
                    .commit();
            }
        }

        bctx.build()
            .global_mutex("DCM", "CFG")
            .mode(mode)
            .test_bel_attr_bits(bcls::DCM::S3E_REG_DLL_C)
            .multi_global_xy("CFG_DLL_C_*", MultiValue::Bin, 32);
        bctx.build()
            .global_mutex("DCM", "CFG")
            .mode(mode)
            .test_bel_attr_bits(bcls::DCM::S3E_REG_DLL_S)
            .multi_global_xy("CFG_DLL_S_*", MultiValue::Bin, 32);
        bctx.build()
            .global_mutex("DCM", "CFG")
            .mode(mode)
            .test_bel_attr_bits(bcls::DCM::S3E_REG_DFS_C)
            .multi_global_xy("CFG_DFS_C_*", MultiValue::Bin, 12);
        bctx.build()
            .global_mutex("DCM", "CFG")
            .mode(mode)
            .test_bel_attr_bits(bcls::DCM::S3E_REG_DFS_S)
            .multi_global_xy("CFG_DFS_S_*", MultiValue::Bin, 76);
        bctx.build()
            .global_mutex("DCM", "CFG")
            .mode(mode)
            .test_bel_attr_bits(bcls::DCM::S3E_REG_INTERFACE)
            .multi_global_xy("CFG_INTERFACE_*", MultiValue::Bin, 16);
        if vreg.is_none() {
            bctx.build()
                .global_mutex("DCM", "CFG")
                .mode(mode)
                .test_bel_attr_bits(bcls::DCM::S3E_REG_VREG)
                .multi_global_xy("CFG_REG_*", MultiValue::Bin, 36);
        }
        for pin in [
            bcls::DCM::RST,
            bcls::DCM::PSCLK,
            bcls::DCM::PSEN,
            bcls::DCM::PSINCDEC,
            bcls::DCM::DSSEN,
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
            bctx.mode(mode)
                .global_mutex("DCM", "USE")
                .global_mutex("PSCLK", "DCM")
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
                .global_mutex("DCM", "PINS")
                .mutex("PIN", pin)
                .no_pin("CLKFB")
                .test_bel_attr_bits(attr)
                .pin(pin)
                .commit();
            bctx.mode(mode)
                .global_mutex("DCM", "PINS")
                .mutex("PIN", pin)
                .pin("CLKFB")
                .test_bel_attr_special(attr, specials::DCM_PIN_CLKFB)
                .pin(pin)
                .commit();
            if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
                bctx.mode(mode)
                    .global_mutex("DCM", "PINS")
                    .mutex("PIN", format!("{pin}.CLKFX"))
                    .pin("CLKFX")
                    .pin("CLKFB")
                    .test_bel_attr_special(attr, specials::DCM_PIN_CLKFX)
                    .pin(pin)
                    .commit();
            }
        }
        bctx.mode(mode)
            .null_bits()
            .global_mutex("DCM", "PINS")
            .test_bel_attr_bits(bcls::DCM::CLKFB_ENABLE)
            .pin("CLKFB")
            .commit();
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKFB", PinFromKind::Bufg)
            .test_bel_attr_bits(bcls::DCM::CLKIN_IOB)
            .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKIN", PinFromKind::Bufg)
            .test_bel_attr_bits(bcls::DCM::CLKFB_IOB)
            .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();

        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_auto(bcls::DCM::DLL_FREQUENCY_MODE);
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_auto(bcls::DCM::DFS_FREQUENCY_MODE);
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_bel_attr_bits(bcls::DCM::STARTUP_WAIT)
            .attr("STARTUP_WAIT", "STARTUP_WAIT")
            .commit();
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_bool_rename(
                "DUTY_CYCLE_CORRECTION",
                bcls::DCM::S3E_DUTY_CYCLE_CORRECTION,
                "FALSE",
                "TRUE",
            );
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_multi(bcls::DCM::DESKEW_ADJUST, MultiValue::Dec(0));
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_bits(bcls::DCM::CLKIN_DIVIDE_BY_2)
            .attr("CLKIN_DIVIDE_BY_2", "CLKIN_DIVIDE_BY_2")
            .commit();
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_bool_rename("CLK_FEEDBACK", bcls::DCM::CLK_FEEDBACK_2X, "1X", "2X");
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_bits(bcls::DCM::S3E_CLKFX_MULTIPLY)
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_bits(bcls::DCM::S3E_CLKFX_DIVIDE)
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode(mode)
            .null_bits()
            .global_mutex("DCM", "USE")
            .pin("CLK0")
            .no_pin("CLKFB")
            .test_bel_special(specials::DCM_VERY_HIGH_FREQUENCY)
            .attr("VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY")
            .commit();
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .pin("CLK0")
            .pin("CLKFB")
            .test_bel_special(specials::DCM_VERY_HIGH_FREQUENCY_CLKFB)
            .attr("VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY")
            .commit();
        for (spec, val) in [
            (specials::DCM_CLKOUT_PHASE_SHIFT_NONE, "NONE"),
            (specials::DCM_CLKOUT_PHASE_SHIFT_FIXED, "FIXED"),
            (specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE, "VARIABLE"),
        ] {
            bctx.mode(mode)
                .global_mutex("DCM", "USE")
                .pin("CLK0")
                .test_bel_special(spec)
                .attr("CLKOUT_PHASE_SHIFT", val)
                .commit();
        }
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_attr_bits(bcls::DCM::PHASE_SHIFT)
            .multi_attr("PHASE_SHIFT", MultiValue::Dec(0), 7);
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_special(specials::DCM_PHASE_SHIFT_N1)
            .attr("PHASE_SHIFT", "-1")
            .commit();
        bctx.mode(mode)
            .global_mutex("DCM", "USE")
            .test_bel_special(specials::DCM_PHASE_SHIFT_N255)
            .attr("PHASE_SHIFT", "-255")
            .commit();

        for val in 2..=16 {
            bctx.mode(mode)
                .global_mutex("DCM", "USE")
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
                    .global_mutex("DCM", "USE")
                    .attr("DLL_FREQUENCY_MODE", dll_mode)
                    .attr("X_CLKIN_PERIOD", "")
                    .test_bel_special_u32(spec, val)
                    .attr("CLKDV_DIVIDE", format!("{val}_5"))
                    .commit();
            }
        }
        for (spec, val) in [
            (specials::DCM_X_CLKIN_PERIOD_1P0, "1.0"),
            (specials::DCM_X_CLKIN_PERIOD_4P99, "4.99"),
            (specials::DCM_X_CLKIN_PERIOD_5P0, "5.0"),
            (specials::DCM_X_CLKIN_PERIOD_24P99, "24.99"),
            (specials::DCM_X_CLKIN_PERIOD_25P0, "25.0"),
            (specials::DCM_X_CLKIN_PERIOD_200P99, "200.99"),
        ] {
            bctx.mode(mode)
                .global_mutex("DCM", "USE")
                .attr("CLKIN_DIVIDE_BY_2", "")
                .attr("CLKFX_MULTIPLY", "")
                .attr("CLKFX_DIVIDE", "")
                .pin("CLK0")
                .no_pin("CLKFX")
                .test_bel_special(spec)
                .attr("X_CLKIN_PERIOD", val)
                .commit();
        }
        if vreg.is_none() {
            bctx.mode(mode)
                .global_mutex("DCM", "USE_VREG")
                .pin("CLK0")
                .test_bel_special(specials::DCM_X_CLKIN_PERIOD_201P0)
                .attr("X_CLKIN_PERIOD", "201.0")
                .commit();
        }

        // junk
        for pin in [
            "STATUS0", "STATUS1", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6", "STATUS7",
        ] {
            bctx.mode(mode)
                .null_bits()
                .global_mutex("DCM", "USE")
                .test_bel_special(specials::DCM_PIN_DUMMY)
                .pin(pin)
                .commit();
        }
        bctx.mode(mode)
            .null_bits()
            .global_mutex("DCM", "USE")
            .test_bel_attr_auto(bcls::DCM::DSS_MODE);
        bctx.mode(mode)
            .null_bits()
            .global_mutex("DCM", "USE")
            .test_bel_attr_bits_bi(bcls::DCM::DSS_ENABLE, false)
            .attr("DSS_MODE", "NONE")
            .commit();
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
                .null_bits()
                .global_mutex("DCM", "USE")
                .test_bel_attr_u32(bcls::DCM::FACTORY_JF1, val)
                .attr("FACTORY_JF1", vname)
                .commit();
            bctx.mode(mode)
                .null_bits()
                .global_mutex("DCM", "USE")
                .test_bel_attr_u32(bcls::DCM::FACTORY_JF2, val)
                .attr("FACTORY_JF2", vname)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    if devdata_only {
        let tcid = tcls::DCM_S3E_NE;
        let bslot = bslots::DCM;
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let item = ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
        let val = extract_bitvec_val(
            item,
            &bits![0; 4],
            present.split_bits(&item.iter().map(|bit| bit.bit).collect()),
        );
        ctx.insert_devdata_bitvec(devdata::DCM_DESKEW_ADJUST, val);
        return;
    }
    for (tcid, vreg) in [
        (tcls::DCM_S3E_SW, Some(tcls::DCM_S3E_SE)),
        (tcls::DCM_S3E_SE, None),
        (tcls::DCM_S3E_NW, Some(tcls::DCM_S3E_NE)),
        (tcls::DCM_S3E_NE, None),
        (tcls::DCM_S3E_WS, Some(tcls::DCM_S3E_WN)),
        (tcls::DCM_S3E_WN, None),
        (tcls::DCM_S3E_ES, None),
        (tcls::DCM_S3E_EN, Some(tcls::DCM_S3E_ES)),
    ] {
        let bslot = bslots::DCM;
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for pin in [
            bcls::DCM::RST,
            bcls::DCM::PSEN,
            bcls::DCM::PSINCDEC,
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
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT_DCM], tcid, bslot, bcls::DCM::PSCLK);
        ctx.get_diff_bel_input_inv(tcid, bslot, bcls::DCM::DSSEN, false)
            .assert_empty();
        ctx.get_diff_bel_input_inv(tcid, bslot, bcls::DCM::DSSEN, true)
            .assert_empty();

        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);

        // TODO: VREG ENABLE etc
        if vreg.is_none() {
            let base = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_OPT_BASE);
            let mut diffs = vec![];
            for spec in [
                specials::DCM_VBG_SEL0,
                specials::DCM_VBG_SEL1,
                specials::DCM_VBG_SEL2,
                specials::DCM_VBG_SEL3,
            ] {
                diffs.push(ctx.get_diff_bel_special(tcid, bslot, spec).combine(&!&base));
            }
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_VBG_SEL, xlat_bitvec(diffs));

            let mut cfg_vreg = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::S3E_REG_VREG, 36);
            for i in 0..16 {
                cfg_vreg[i].assert_empty();
            }
            let mut cfg_vreg = cfg_vreg.split_off(16);
            cfg_vreg.reverse();
            let vreg_bits: HashSet<_> = cfg_vreg
                .iter()
                .flat_map(|x| x.bits.keys().copied())
                .collect();
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_VREG, xlat_bitvec(cfg_vreg));

            let mut vreg_enable = present.split_bits(&vreg_bits);
            if edev.chip.kind.is_spartan3a() || edev.chip.dcms != Some(Dcms::Two) {
                let diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VREG_ENABLE);
                assert_eq!(vreg_enable, diff);
            }

            vreg_enable.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_VBG_SEL),
                &bits![0, 1, 0, 1],
                &bits![0; 4],
            );

            let mut base_vreg = BitVec::repeat(false, 20);
            base_vreg.set(0, true);
            base_vreg.set(6, true);
            vreg_enable.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_VREG),
                &base_vreg,
                &bits![0; 20],
            );

            vreg_enable.assert_empty();
        }

        let mut cfg_dll_c = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::S3E_REG_DLL_C, 32);
        let mut cfg_dll_s = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::S3E_REG_DLL_S, 32);
        let mut cfg_dfs_c = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::S3E_REG_DFS_C, 12);
        let mut cfg_dfs_s = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::S3E_REG_DFS_S, 76);
        let mut cfg_interface =
            ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::S3E_REG_INTERFACE, 16);

        for i in 0..9 {
            cfg_dfs_c[i].assert_empty();
        }
        let mut cfg_dfs_c = cfg_dfs_c.split_off(9);
        cfg_dll_c.reverse();
        cfg_dll_s.reverse();
        cfg_dfs_c.reverse();
        cfg_dfs_s.reverse();
        cfg_interface.reverse();
        let cfg_dll_c = xlat_bitvec(cfg_dll_c);
        let cfg_dll_s = xlat_bitvec(cfg_dll_s);
        let cfg_dfs_c = xlat_bitvec(cfg_dfs_c);
        let cfg_dfs_s = xlat_bitvec(cfg_dfs_s);
        let cfg_interface = xlat_bitvec(cfg_interface);

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::S3E_DUTY_CYCLE_CORRECTION);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::STARTUP_WAIT);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKIN_DIVIDE_BY_2);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::CLK_FEEDBACK_2X);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DLL_FREQUENCY_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DFS_FREQUENCY_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::S3E_CLKFX_MULTIPLY);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::S3E_CLKFX_DIVIDE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKIN_IOB);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKFB_IOB);

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
            let diff_fx = diff_fx.combine(&!&diff_fb);
            let diff_fb = diff_fb.combine(&!&diff);
            ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DLL_ENABLE, xlat_bit(diff_fb));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DFS_FEEDBACK, xlat_bit(diff_fx));
        }

        let diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VERY_HIGH_FREQUENCY_CLKFB);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DLL_ENABLE, xlat_bit(!diff));

        let (_, _, dfs_en) = Diff::split(
            ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLKFX_ENABLE, 0)
                .clone(),
            ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CONCUR_ENABLE, 0)
                .clone(),
        );
        for attr in [
            bcls::DCM::OUT_CLKFX_ENABLE,
            bcls::DCM::OUT_CLKFX180_ENABLE,
            bcls::DCM::OUT_CONCUR_ENABLE,
        ] {
            let diff = ctx.get_diff_attr_bit(tcid, bslot, attr, 0);
            let diff_fb = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFB);
            assert_eq!(diff, diff_fb);
            let diff = diff.combine(&!&dfs_en);
            let attr = if attr == bcls::DCM::OUT_CONCUR_ENABLE {
                attr
            } else {
                bcls::DCM::OUT_CLKFX_ENABLE
            };
            ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
        }
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DFS_ENABLE, xlat_bit(dfs_en));

        let item = ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
        let val = extract_bitvec_val(
            item,
            &bits![0; 4],
            present.split_bits(&item.iter().map(|bit| bit.bit).collect()),
        );
        ctx.insert_devdata_bitvec(devdata::DCM_DESKEW_ADJUST, val);

        let mut diffs = vec![ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N255)];
        diffs.extend(ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::PHASE_SHIFT, 7));
        let item = xlat_bitvec(diffs);
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N1);
        diff.apply_bitvec_diff_int(&item, 2, 0);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::PHASE_SHIFT, item);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::PHASE_SHIFT_NEGATIVE, xlat_bit(diff));

        ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_NONE)
            .assert_empty();
        let diff_f = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_FIXED);
        let diff_v =
            ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKOUT_PHASE_SHIFT_VARIABLE);
        let diff_v = diff_v.combine(&!&diff_f);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::PS_ENABLE, xlat_bit(diff_f));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::PS_VARIABLE, xlat_bit(diff_v));

        for (attr, bits) in [
            (bcls::DCM::CLKDV_COUNT_MAX, &cfg_dll_c[1..5]),
            (bcls::DCM::CLKDV_COUNT_FALL, &cfg_dll_c[5..9]),
            (bcls::DCM::CLKDV_COUNT_FALL_2, &cfg_dll_c[9..13]),
            (bcls::DCM::CLKDV_PHASE_RISE, &cfg_dll_c[13..15]),
            (bcls::DCM::CLKDV_PHASE_FALL, &cfg_dll_c[15..17]),
        ] {
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits.to_vec());
        }
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::DCM::CLKDV_MODE,
            BelAttributeEnum {
                bits: vec![cfg_dll_c[17].bit],
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

        ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_1P0)
            .assert_empty();
        ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_4P99)
            .assert_empty();
        let diff_a = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_5P0);
        assert_eq!(
            diff_a,
            ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_24P99)
        );
        let diff_b = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_25P0);
        assert_eq!(
            diff_b,
            ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_200P99)
        );
        if vreg.is_none() {
            let diff_c = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_X_CLKIN_PERIOD_201P0);
            let mut diff_c = diff_c.combine(&!&diff_b);
            diff_c.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_VBG_SEL),
                &bits![0, 1, 1, 0],
                &bits![0, 1, 0, 1],
            );
            diff_c.assert_empty();
        }
        let mut diff_b = diff_b.combine(&!&diff_a);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::UNK_PERIOD_NOT_HF, xlat_bit(!diff_a));
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCM::UNK_PERIOD_LF,
            vec![cfg_dll_s[7], cfg_dll_s[17]],
        );
        diff_b.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::UNK_PERIOD_LF),
            &bits![1; 2],
            &bits![0; 2],
        );
        diff_b.assert_empty();

        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_DLL_C, cfg_dll_c);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_DLL_S, cfg_dll_s);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_DFS_C, cfg_dfs_c);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_DFS_S, cfg_dfs_s);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_INTERFACE, cfg_interface);

        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::S3E_DUTY_CYCLE_CORRECTION),
            true,
            false,
        );
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
        present.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::UNK_PERIOD_NOT_HF),
            true,
            false,
        );

        let mut base_interface = BitVec::repeat(false, 16);
        base_interface.set(9, true);
        base_interface.set(10, true);
        base_interface.set(13, true);
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_INTERFACE),
            &base_interface,
            &bits![0; 16],
        );

        let mut base_dfs_s = BitVec::repeat(false, 76);
        base_dfs_s.set(17, true);
        base_dfs_s.set(21, true);
        base_dfs_s.set(32, true);
        base_dfs_s.set(33, true);
        base_dfs_s.set(37, true);
        base_dfs_s.set(41, true);
        base_dfs_s.set(43, true);
        base_dfs_s.set(45, true);
        base_dfs_s.set(52, true);
        base_dfs_s.set(64, true);
        base_dfs_s.set(68, true);
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_DFS_S),
            &base_dfs_s,
            &bits![0; 76],
        );

        let mut base_dll_s = BitVec::repeat(false, 32);
        base_dll_s.set(0, true);
        base_dll_s.set(6, true);
        present.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::S3E_REG_DLL_S),
            &base_dll_s,
            &bits![0; 32],
        );

        present.assert_empty();
    }
}
