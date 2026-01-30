use std::collections::BTreeMap;

use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_collector::{
    diff::{Diff, DiffKey, FeatureId, OcdMode},
    legacy::{xlat_bit_legacy, xlat_bitvec_legacy, xlat_enum_default_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_spartan6::defs;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileItem, TileItemKind},
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{DynProp, relation::Delta},
    },
};

#[derive(Copy, Clone, Debug)]
struct AllOtherDcms(&'static str, &'static str, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for AllOtherDcms {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcid = backend.edev.db.get_tile_class("CMT_DCM");
        for &ntcrd in &backend.edev.tile_index[tcid] {
            if tcrd == ntcrd {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "CMT_DCM".into(),
                    bel: self.0.into(),
                    attr: self.1.into(),
                    val: self.2.into(),
                }),
                rects: backend.edev.tile_bits(ntcrd),
            });
        }

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "CMT_DCM");
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::DCM[i]);
        bctx.build()
            .global_mutex("CMT", "PRESENT")
            .global_mutex_here("CMT_PRESENT")
            .prop(AllOtherDcms("CMT", "PRESENT_ANY_DCM", "1"))
            .test_manual_legacy("PRESENT", "DCM")
            .mode("DCM")
            .commit();
        bctx.build()
            .global_mutex("CMT", "PRESENT")
            .global_mutex_here("CMT_PRESENT")
            .prop(AllOtherDcms("CMT", "PRESENT_ANY_DCM", "1"))
            .test_manual_legacy("PRESENT", "DCM_CLKGEN")
            .mode("DCM_CLKGEN")
            .commit();

        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DLL_C", "")
            .multi_global_xy("CFG_DLL_C_*", MultiValue::Bin, 32);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DLL_S", "")
            .multi_global_xy("CFG_DLL_S_*", MultiValue::Bin, 32);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DFS_C", "")
            .multi_global_xy("CFG_DFS_C_*", MultiValue::Bin, 3);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DFS_S", "")
            .multi_global_xy("CFG_DFS_S_*", MultiValue::Bin, 87);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("INTERFACE", "")
            .multi_global_xy("CFG_INTERFACE_*", MultiValue::Bin, 40);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("OPT_INV", "")
            .multi_global_xy("CFG_OPT_INV_*", MultiValue::Bin, 20);
        bctx.mode("DCM")
            .global_mutex("CMT", format!("CFG_DCM{i}"))
            .test_manual_legacy("REG", "")
            .multi_global_xy("CFG_REG_*", MultiValue::Bin, 9);
        bctx.mode("DCM")
            .global_mutex("CMT", format!("CFG_DCM{i}"))
            .test_manual_legacy("BG", "")
            .multi_global_xy("CFG_BG_*", MultiValue::Bin, 11);

        let obel_dcm = defs::bslots::DCM[i ^ 1];
        for opin in ["CLKIN", "CLKIN_TEST"] {
            for (val, pin) in [
                ("CKINT0", "CLKIN_CKINT0"),
                ("CKINT1", "CLKIN_CKINT1"),
                ("CLK_FROM_PLL", "CLK_FROM_PLL"),
            ] {
                let related_pll = Delta::new(0, 16, "CMT_PLL");
                let bel_pll = defs::bslots::PLL;
                bctx.mode("DCM")
                    .global_mutex("CMT", "TEST")
                    .mutex("CLKIN_OUT", opin)
                    .mutex("CLKIN_IN", pin)
                    .tile_mutex("CLKIN_BEL", format!("DCM{i}"))
                    .related_pip(
                        related_pll,
                        (bel_pll, format!("CLK_TO_DCM{i}")),
                        (bel_pll, "CLKOUTDCM0"),
                    )
                    .test_manual_legacy(format!("MUX.{opin}"), val)
                    .pip(opin, pin)
                    .commit();
            }
            for btlr in ["BT", "LR"] {
                for j in 0..8 {
                    bctx.mode("DCM")
                        .global_mutex("CMT", "TEST")
                        .global_mutex("BUFIO2_CMT_OUT", "USE")
                        .mutex("CLKIN_OUT", opin)
                        .mutex("CLKIN_IN", format!("BUFIO2_{btlr}{j}"))
                        .tile_mutex("CLKIN_BEL", format!("DCM{i}"))
                        .pip(
                            (obel_dcm, opin),
                            (defs::bslots::CMT, format!("BUFIO2_{btlr}{j}")),
                        )
                        .test_manual_legacy(format!("MUX.{opin}"), format!("BUFIO2_{btlr}{j}"))
                        .pip(opin, (defs::bslots::CMT, format!("BUFIO2_{btlr}{j}")))
                        .commit();
                }
            }
        }
        for opin in ["CLKFB", "CLKFB_TEST"] {
            for (val, pin) in [("CKINT0", "CLKFB_CKINT0"), ("CKINT1", "CLKFB_CKINT1")] {
                bctx.mode("DCM")
                    .global_mutex("CMT", "TEST")
                    .mutex("CLKIN_OUT", opin)
                    .mutex("CLKIN_IN", pin)
                    .tile_mutex("CLKIN_BEL", format!("DCM{i}"))
                    .test_manual_legacy(format!("MUX.{opin}"), val)
                    .pip(opin, pin)
                    .commit();
            }
            for btlr in ["BT", "LR"] {
                for j in 0..8 {
                    bctx.mode("DCM")
                        .global_mutex("CMT", "TEST")
                        .global_mutex("BUFIO2_CMT_OUT", "USE")
                        .mutex("CLKIN_OUT", opin)
                        .mutex("CLKIN_IN", format!("BUFIO2FB_{btlr}{j}"))
                        .tile_mutex("CLKIN_BEL", format!("DCM{i}"))
                        .pip(
                            (obel_dcm, opin),
                            (defs::bslots::CMT, format!("BUFIO2FB_{btlr}{j}")),
                        )
                        .test_manual_legacy(format!("MUX.{opin}"), format!("BUFIO2FB_{btlr}{j}"))
                        .pip(opin, (defs::bslots::CMT, format!("BUFIO2FB_{btlr}{j}")))
                        .commit();
                }
            }
        }

        for out in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .pin("CLKDV")
                .mutex("CLK_TO_PLL", out)
                .test_manual_legacy("MUX.CLK_TO_PLL", out)
                .pip("CLK_TO_PLL", out)
                .commit();
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .pin("CLKDV")
                .mutex("SKEWCLKIN2", out)
                .test_manual_legacy("MUX.SKEWCLKIN2", out)
                .pip("SKEWCLKIN2", format!("{out}_TEST"))
                .commit();
        }

        for pin in [
            "PSCLK", "PSEN", "PSINCDEC", "RST", "SKEWIN", "CTLGO", "CTLSEL0", "CTLSEL1", "CTLSEL2",
            "SKEWRST",
        ] {
            bctx.mode("DCM").global_mutex("CMT", "TEST").test_inv(pin);
        }
        for pin in ["PROGCLK", "PROGEN", "PROGDATA", "RST"] {
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "TEST")
                .pin(pin)
                .test_manual_legacy(format!("{pin}INV.DCM_CLKGEN"), "0")
                .attr(format!("{pin}INV"), pin)
                .commit();
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "TEST")
                .pin(pin)
                .test_manual_legacy(format!("{pin}INV.DCM_CLKGEN"), "1")
                .attr(format!("{pin}INV"), format!("{pin}_B"))
                .commit();
        }
        for pin in [
            "FREEZEDLL",
            "FREEZEDFS",
            "CTLMODE",
            "CTLOSC1",
            "CTLOSC2",
            "STSADRS0",
            "STSADRS1",
            "STSADRS2",
            "STSADRS3",
            "STSADRS4",
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .test_manual_legacy(format!("PIN.{pin}"), "1")
                .pin(pin)
                .pin_pips(pin)
                .commit();
        }

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "PINS")
                .mutex("PIN", pin)
                .no_pin("CLKFB")
                .test_manual_legacy(pin, "1")
                .pin(pin)
                .commit();
            bctx.mode("DCM")
                .global_mutex("CMT", "PINS")
                .mutex("PIN", pin)
                .pin("CLKFB")
                .test_manual_legacy(pin, "1.CLKFB")
                .pin(pin)
                .commit();
            if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
                bctx.mode("DCM")
                    .global_mutex("CMT", "PINS")
                    .mutex("PIN", format!("{pin}.CLKFX"))
                    .pin("CLKFX")
                    .pin("CLKFB")
                    .test_manual_legacy(pin, "1.CLKFX")
                    .pin(pin)
                    .commit();
            }
        }
        bctx.mode("DCM")
            .global_mutex("CMT", "PINS")
            .test_manual_legacy("CLKFB", "1")
            .pin("CLKFB")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GLUTMASK", "NO")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKFB", PinFromKind::Bufg)
            .test_manual_legacy("CLKIN_IOB", "1")
            .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GLUTMASK", "NO")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKIN", PinFromKind::Bufg)
            .test_manual_legacy("CLKFB_IOB", "1")
            .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGEN")
            .no_pin("PROGDATA")
            .test_manual_legacy("PIN.PROGCLK", "1")
            .pin("PROGCLK")
            .commit();
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGCLK")
            .no_pin("PROGDATA")
            .test_manual_legacy("PIN.PROGEN", "1")
            .pin("PROGEN")
            .commit();
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGCLK")
            .no_pin("PROGEN")
            .test_manual_legacy("PIN.PROGDATA", "1")
            .pin("PROGDATA")
            .commit();

        bctx.mode("DCM").global_mutex("CMT", "TEST").test_enum_legacy(
            "DSS_MODE",
            &["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"],
        );
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_enum_legacy("STARTUP_WAIT", &["FALSE", "TRUE"]);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_enum_suffix("STARTUP_WAIT", "CLKGEN", &["FALSE", "TRUE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DUTY_CYCLE_CORRECTION", &["FALSE", "TRUE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_multi_attr_dec("DESKEW_ADJUST", 4);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("CLKIN_DIVIDE_BY_2", &["FALSE", "TRUE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("CLK_FEEDBACK", &["NONE", "1X", "2X"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_MULTIPLY", "")
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_DIVIDE", "")
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_MULTIPLY.CLKGEN", "")
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_DIVIDE.CLKGEN", "")
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .pin("CLK0")
            .no_pin("CLKFB")
            .test_enum_legacy("VERY_HIGH_FREQUENCY", &["FALSE", "TRUE"]);

        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .pin("CLK0")
            .test_enum_legacy("CLKOUT_PHASE_SHIFT", &["NONE", "FIXED", "VARIABLE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_multi_attr_dec("PHASE_SHIFT", 7);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("PHASE_SHIFT", "-1")
            .attr("PHASE_SHIFT", "-1")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("PHASE_SHIFT", "-255")
            .attr("PHASE_SHIFT", "-255")
            .commit();

        bctx.mode("DCM").global_mutex("CMT", "USE").test_enum_legacy(
            "CLKDV_DIVIDE",
            &[
                "2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
                "13.0", "14.0", "15.0", "16.0",
            ],
        );
        for dll_mode in ["LOW", "HIGH"] {
            for val in ["1.5", "2.5", "3.5", "4.5", "5.5", "6.5", "7.5"] {
                bctx.mode("DCM")
                    .global_mutex("CMT", "USE")
                    .attr("DLL_FREQUENCY_MODE", dll_mode)
                    .attr("CLKIN_PERIOD", "")
                    .test_manual_legacy("CLKDV_DIVIDE", format!("{val}.{dll_mode}"))
                    .attr("CLKDV_DIVIDE", val)
                    .commit();
            }
        }

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("CLKFXDV_DIVIDE", &["2", "4", "8", "16", "32"]);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DFS_BANDWIDTH", &["LOW", "HIGH", "OPTIMIZED"]);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("PROG_MD_BANDWIDTH", &["LOW", "HIGH", "OPTIMIZED"]);

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .no_pin("PROGCLK")
            .no_pin("PROGEN")
            .no_pin("PROGDATA")
            .test_enum_legacy(
                "SPREAD_SPECTRUM",
                &[
                    "NONE",
                    "CENTER_LOW_SPREAD",
                    "CENTER_HIGH_SPREAD",
                    "VIDEO_LINK_M0",
                    "VIDEO_LINK_M1",
                    "VIDEO_LINK_M2",
                ],
            );

        // TODO: CLKIN_PERIOD?
    }

    let mut bctx = ctx.bel(defs::bslots::CMT);
    for i in 0..16 {
        bctx.build()
            .mutex(format!("MUX.CASC{i}"), "PASS")
            .test_manual_legacy(format!("MUX.CASC{i}"), "PASS")
            .pip(format!("CASC{i}_O"), format!("CASC{i}_I"))
            .commit();
        bctx.build()
            .mutex(format!("MUX.CASC{i}"), "HCLK")
            .test_manual_legacy(format!("MUX.CASC{i}"), "HCLK")
            .pip(format!("CASC{i}_O"), format!("HCLK{i}_BUF"))
            .commit();
        bctx.build()
            .mutex(format!("MUX.HCLK{i}"), "CKINT")
            .test_manual_legacy(format!("MUX.HCLK{i}"), "CKINT")
            .pip(format!("HCLK{i}"), format!("HCLK{i}_CKINT"))
            .commit();
        for j in 0..2 {
            let bel_dcm = defs::bslots::DCM[j];
            for out in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ] {
                bctx.build()
                    .mutex(format!("MUX.HCLK{i}"), format!("DCM{j}_{out}"))
                    .test_manual_legacy(format!("MUX.HCLK{i}"), format!("DCM{j}_{out}"))
                    .pip(format!("HCLK{i}"), (bel_dcm, format!("{out}_OUT")))
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CMT_DCM";
    for bel in ["DCM[0]", "DCM[1]"] {
        let mut present_dcm = ctx.get_diff_legacy(tile, bel, "PRESENT", "DCM");
        let mut present_dcm_clkgen = ctx.get_diff_legacy(tile, bel, "PRESENT", "DCM_CLKGEN");

        let mut cfg_interface = ctx.get_diffs_legacy(tile, bel, "INTERFACE", "");
        cfg_interface.reverse();
        let mut cfg_dll_c = ctx.get_diffs_legacy(tile, bel, "DLL_C", "");
        cfg_dll_c.reverse();
        let cfg_dll_c = xlat_bitvec_legacy(cfg_dll_c);
        for attr in ["DLL_S", "DFS_C", "DFS_S"] {
            let mut diffs = ctx.get_diffs_legacy(tile, bel, attr, "");
            diffs.reverse();
            ctx.insert(tile, bel, attr, xlat_bitvec_legacy(diffs));
        }
        for attr in ["REG", "BG"] {
            let mut diffs = ctx.get_diffs_legacy(tile, bel, attr, "");
            diffs.reverse();
            ctx.insert(tile, "CMT", attr, xlat_bitvec_legacy(diffs));
        }
        let mut cfg_opt_inv = ctx.get_diffs_legacy(tile, bel, "OPT_INV", "");
        cfg_opt_inv.reverse();
        ctx.insert(
            tile,
            bel,
            "OPT_INV",
            xlat_bitvec_legacy(cfg_opt_inv[..3].to_vec()),
        );
        for pin in [
            "PSEN", "PSINCDEC", "RST", "SKEWIN", "CTLGO", "CTLSEL0", "CTLSEL1", "CTLSEL2",
            "SKEWRST",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }
        for (hwpin, pin) in [("PSEN", "PROGEN"), ("PSINCDEC", "PROGDATA"), ("RST", "RST")] {
            let item =
                ctx.extract_bit_bi_legacy(tile, bel, &format!("{pin}INV.DCM_CLKGEN"), "0", "1");
            ctx.insert(tile, bel, format!("INV.{hwpin}"), item);
        }
        for pin in [
            "FREEZEDLL",
            "FREEZEDFS",
            "CTLMODE",
            "CTLOSC1",
            "CTLOSC2",
            "STSADRS0",
            "STSADRS1",
            "STSADRS2",
            "STSADRS3",
            "STSADRS4",
        ] {
            let diff = ctx.get_diff_legacy(tile, bel, format!("PIN.{pin}"), "1");
            present_dcm = present_dcm.combine(&diff);
            present_dcm_clkgen = present_dcm.combine(&diff);
            ctx.insert(tile, bel, format!("INV.{pin}"), xlat_bit_legacy(!diff));
        }

        // hrm. concerning.
        ctx.get_diff_legacy(tile, bel, "PSCLKINV", "PSCLK")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "PSCLKINV", "PSCLK_B")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "PROGCLKINV.DCM_CLKGEN", "0")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "PROGCLKINV.DCM_CLKGEN", "1")
            .assert_empty();

        let (_, _, clkin_clkfb_enable) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "MUX.CLKIN", "CKINT0")
                .clone(),
            ctx.peek_diff_legacy(tile, bel, "MUX.CLKFB", "CKINT0")
                .clone(),
        );
        let mut diffs = vec![];
        for out in ["CLKIN", "CLKIN_TEST"] {
            for val in [
                "BUFIO2_LR0",
                "BUFIO2_LR1",
                "BUFIO2_LR2",
                "BUFIO2_LR3",
                "BUFIO2_LR4",
                "BUFIO2_LR5",
                "BUFIO2_LR6",
                "BUFIO2_LR7",
                "BUFIO2_BT0",
                "BUFIO2_BT1",
                "BUFIO2_BT2",
                "BUFIO2_BT3",
                "BUFIO2_BT4",
                "BUFIO2_BT5",
                "BUFIO2_BT6",
                "BUFIO2_BT7",
                "CKINT0",
                "CKINT1",
                "CLK_FROM_PLL",
            ] {
                let mut diff = ctx.get_diff_legacy(tile, bel, format!("MUX.{out}"), val);
                diff = diff.combine(&!&clkin_clkfb_enable);
                diffs.push((val.to_string(), diff));
            }
        }
        ctx.insert(
            tile,
            bel,
            "MUX.CLKIN",
            xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
        );
        let mut diffs = vec![];
        for out in ["CLKFB", "CLKFB_TEST"] {
            for val in [
                "BUFIO2FB_LR0",
                "BUFIO2FB_LR1",
                "BUFIO2FB_LR2",
                "BUFIO2FB_LR3",
                "BUFIO2FB_LR4",
                "BUFIO2FB_LR5",
                "BUFIO2FB_LR6",
                "BUFIO2FB_LR7",
                "BUFIO2FB_BT0",
                "BUFIO2FB_BT1",
                "BUFIO2FB_BT2",
                "BUFIO2FB_BT3",
                "BUFIO2FB_BT4",
                "BUFIO2FB_BT5",
                "BUFIO2FB_BT6",
                "BUFIO2FB_BT7",
                "CKINT0",
                "CKINT1",
            ] {
                let mut diff = ctx.get_diff_legacy(tile, bel, format!("MUX.{out}"), val);
                diff = diff.combine(&!&clkin_clkfb_enable);
                diffs.push((val.to_string(), diff));
            }
        }
        ctx.insert(
            tile,
            bel,
            "MUX.CLKFB",
            xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
        );
        ctx.insert(
            tile,
            bel,
            "CLKIN_CLKFB_ENABLE",
            xlat_bit_legacy(clkin_clkfb_enable),
        );
        ctx.collect_enum_legacy(
            tile,
            bel,
            "MUX.CLK_TO_PLL",
            &[
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ],
        );
        ctx.collect_enum_legacy(
            tile,
            bel,
            "MUX.SKEWCLKIN2",
            &[
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR",
            ],
        );

        ctx.collect_bit_bi_legacy(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "CLKIN_DIVIDE_BY_2", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "CLK_FEEDBACK", &["1X", "2X"]);
        ctx.collect_enum_legacy(tile, bel, "DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_enum_legacy(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_bitvec_legacy(tile, bel, "DESKEW_ADJUST", "");
        ctx.collect_bitvec_legacy(tile, bel, "CLKFX_MULTIPLY", "");
        ctx.collect_bitvec_legacy(tile, bel, "CLKFX_DIVIDE", "");
        let item = ctx.extract_bitvec_legacy(tile, bel, "CLKFX_MULTIPLY.CLKGEN", "");
        ctx.insert(tile, bel, "CLKFX_MULTIPLY", item);
        let item = ctx.extract_bitvec_legacy(tile, bel, "CLKFX_DIVIDE.CLKGEN", "");
        ctx.insert(tile, bel, "CLKFX_DIVIDE", item);
        ctx.collect_bit_legacy(tile, bel, "CLKIN_IOB", "1");
        ctx.collect_bit_legacy(tile, bel, "CLKFB_IOB", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "STARTUP_WAIT", "FALSE", "TRUE");
        let item = ctx.extract_bit_bi_legacy(tile, bel, "STARTUP_WAIT.CLKGEN", "FALSE", "TRUE");
        ctx.insert(tile, bel, "STARTUP_WAIT", item);
        let item = ctx.extract_bit_legacy(tile, bel, "CLK_FEEDBACK", "NONE");
        ctx.insert(tile, bel, "NO_FEEDBACK", item);

        ctx.get_diff_legacy(tile, bel, "CLKFB", "1").assert_empty();

        let (_, _, dll_en) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "CLK0", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "CLK180", "1").clone(),
        );

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
        ] {
            let diff = ctx.get_diff_legacy(tile, bel, pin, "1");
            let diff_fb = ctx.get_diff_legacy(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff_fx = ctx.get_diff_legacy(tile, bel, pin, "1.CLKFX");
            let diff_fx = diff_fx.combine(&!&diff);
            let mut diff = diff.combine(&!&dll_en);
            // hrm.
            if ctx.device.name.ends_with('l') && pin == "CLKDV" {
                diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DLL_S"), 0, 0x40);
            }
            ctx.insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit_legacy(diff));
            ctx.insert(tile, bel, "DFS_FEEDBACK", xlat_bit_legacy(diff_fx));
        }
        ctx.insert(tile, bel, "DLL_ENABLE", xlat_bit_legacy(dll_en));

        ctx.get_diff_legacy(tile, bel, "VERY_HIGH_FREQUENCY", "FALSE")
            .assert_empty();
        let diff = ctx.get_diff_legacy(tile, bel, "VERY_HIGH_FREQUENCY", "TRUE");
        ctx.insert(tile, bel, "DLL_ENABLE", xlat_bit_legacy(!diff));

        for attr in ["PIN.PROGCLK", "PIN.PROGEN", "PIN.PROGDATA"] {
            let item = ctx.extract_bit_legacy(tile, bel, attr, "1");
            ctx.insert(tile, bel, "PROG_ENABLE", item);
        }

        let (_, _, dfs_en) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "CLKFX", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "CONCUR", "1").clone(),
        );
        for pin in ["CLKFX", "CLKFX180", "CONCUR"] {
            let diff = ctx.get_diff_legacy(tile, bel, pin, "1");
            let diff_fb = ctx.get_diff_legacy(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff = diff.combine(&!&dfs_en);
            let pin = if pin == "CONCUR" { pin } else { "CLKFX" };
            ctx.insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit_legacy(diff));
        }
        ctx.insert(tile, bel, "DFS_ENABLE", xlat_bit_legacy(dfs_en));

        let mut diffs = vec![ctx.get_diff_legacy(tile, bel, "PHASE_SHIFT", "-255")];
        diffs.extend(ctx.get_diffs_legacy(tile, bel, "PHASE_SHIFT", ""));
        let item = xlat_bitvec_legacy(diffs);
        let mut diff = ctx.get_diff_legacy(tile, bel, "PHASE_SHIFT", "-1");
        diff.apply_bitvec_diff_int_legacy(&item, 2, 0);
        ctx.insert(tile, bel, "PHASE_SHIFT", item);
        ctx.insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit_legacy(diff));

        ctx.collect_enum_legacy(
            tile,
            bel,
            "CLKOUT_PHASE_SHIFT",
            &["NONE", "FIXED", "VARIABLE"],
        );

        let mut diffs = vec![];
        for val in [
            "NONE",
            "CENTER_LOW_SPREAD",
            "CENTER_HIGH_SPREAD",
            "VIDEO_LINK_M0",
            "VIDEO_LINK_M1",
            "VIDEO_LINK_M2",
        ] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "SPREAD_SPECTRUM", val);
            if val.starts_with("VIDEO_LINK") {
                diff.apply_bit_diff_legacy(ctx.item(tile, bel, "PROG_ENABLE"), true, false);
            }
            diffs.push((val.to_string(), diff));
        }
        ctx.insert(
            tile,
            bel,
            "SPREAD_SPECTRUM",
            xlat_enum_default_legacy(diffs, "DCM"),
        );

        for (attr, bits) in [
            ("CLKDV_COUNT_MAX", &cfg_dll_c.bits[1..5]),
            ("CLKDV_COUNT_FALL", &cfg_dll_c.bits[5..9]),
            ("CLKDV_COUNT_FALL_2", &cfg_dll_c.bits[9..13]),
            ("CLKDV_PHASE_RISE", &cfg_dll_c.bits[13..15]),
            ("CLKDV_PHASE_FALL", &cfg_dll_c.bits[15..17]),
        ] {
            ctx.insert(
                tile,
                bel,
                attr,
                TileItem {
                    bits: bits.to_vec(),
                    kind: TileItemKind::BitVec {
                        invert: BitVec::repeat(false, bits.len()),
                    },
                },
            );
        }
        ctx.insert(
            tile,
            bel,
            "CLKDV_MODE",
            TileItem {
                bits: cfg_dll_c.bits[17..18].to_vec(),
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("HALF".to_string(), bits![0]),
                        ("INT".to_string(), bits![1]),
                    ]),
                },
            },
        );

        let clkdv_count_max = ctx.item(tile, bel, "CLKDV_COUNT_MAX").clone();
        let clkdv_count_fall = ctx.item(tile, bel, "CLKDV_COUNT_FALL").clone();
        let clkdv_count_fall_2 = ctx.item(tile, bel, "CLKDV_COUNT_FALL_2").clone();
        let clkdv_phase_fall = ctx.item(tile, bel, "CLKDV_PHASE_FALL").clone();
        let clkdv_mode = ctx.item(tile, bel, "CLKDV_MODE").clone();
        for i in 2..=16 {
            let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.0"));
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.LOW"));
            diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH"));
            diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }

        for val in ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            ctx.get_diff_legacy(tile, bel, "DSS_MODE", val)
                .assert_empty();
        }
        for val in ["LOW", "HIGH", "OPTIMIZED"] {
            ctx.get_diff_legacy(tile, bel, "DFS_BANDWIDTH", val)
                .assert_empty();
            ctx.get_diff_legacy(tile, bel, "PROG_MD_BANDWIDTH", val)
                .assert_empty();
        }
        let mut item =
            ctx.extract_enum_legacy(tile, bel, "CLKFXDV_DIVIDE", &["32", "16", "8", "4", "2"]);
        assert_eq!(item.bits.len(), 3);
        let TileItemKind::Enum { ref mut values } = item.kind else {
            unreachable!()
        };
        values.insert("NONE".into(), BitVec::repeat(false, 3));
        ctx.insert(tile, bel, "CLKFXDV_DIVIDE", item);

        ctx.insert(tile, bel, "DLL_C", cfg_dll_c);

        present_dcm.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
        present_dcm.apply_enum_diff_legacy(ctx.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
        present_dcm_clkgen.apply_bitvec_diff_int_legacy(
            ctx.item(tile, bel, "CLKDV_COUNT_MAX"),
            1,
            0,
        );
        present_dcm_clkgen.apply_enum_diff_legacy(ctx.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");
        present_dcm.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "DESKEW_ADJUST"), 11, 0);
        present_dcm_clkgen.apply_bitvec_diff_int_legacy(
            ctx.item(tile, bel, "DESKEW_ADJUST"),
            11,
            0,
        );
        present_dcm.apply_enum_diff_legacy(ctx.item(tile, bel, "CLKFXDV_DIVIDE"), "2", "NONE");
        present_dcm_clkgen.apply_enum_diff_legacy(
            ctx.item(tile, bel, "CLKFXDV_DIVIDE"),
            "2",
            "NONE",
        );
        present_dcm.apply_bit_diff_legacy(
            ctx.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present_dcm_clkgen.apply_bit_diff_legacy(
            ctx.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present_dcm.apply_bitvec_diff_legacy(
            ctx.item(tile, "CMT", "REG"),
            &bits![1, 1, 0, 0, 0, 0, 1, 0, 1],
            &bits![0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm_clkgen.apply_bitvec_diff_legacy(
            ctx.item(tile, "CMT", "REG"),
            &bits![1, 1, 0, 0, 0, 0, 1, 0, 1],
            &bits![0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm.apply_bitvec_diff_legacy(
            ctx.item(tile, "CMT", "BG"),
            &bits![0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1],
            &bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm_clkgen.apply_bitvec_diff_legacy(
            ctx.item(tile, "CMT", "BG"),
            &bits![0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1],
            &bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        // ???
        present_dcm_clkgen.apply_bit_diff_legacy(ctx.item(tile, bel, "INV.STSADRS4"), false, true);

        let mut base_dfs_s = BitVec::repeat(false, 87);
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
        base_dfs_s.set(76, true);
        base_dfs_s.set(77, true);
        present_dcm.apply_bitvec_diff_legacy(
            ctx.item(tile, bel, "DFS_S"),
            &base_dfs_s,
            &BitVec::repeat(false, 87),
        );
        present_dcm_clkgen.apply_bitvec_diff_legacy(
            ctx.item(tile, bel, "DFS_S"),
            &base_dfs_s,
            &BitVec::repeat(false, 87),
        );

        let mut base_dll_s = BitVec::repeat(false, 32);
        base_dll_s.set(0, true);
        base_dll_s.set(6, true);
        base_dll_s.set(13, true); // period not hf
        present_dcm.apply_bitvec_diff_legacy(
            ctx.item(tile, bel, "DLL_S"),
            &base_dll_s,
            &BitVec::repeat(false, 32),
        );
        present_dcm_clkgen.apply_bitvec_diff_legacy(
            ctx.item(tile, bel, "DLL_S"),
            &base_dll_s,
            &BitVec::repeat(false, 32),
        );

        present_dcm = present_dcm.combine(&!&cfg_interface[9]);
        present_dcm = present_dcm.combine(&!&cfg_interface[10]);
        present_dcm = present_dcm.combine(&!&cfg_interface[12]);
        present_dcm = present_dcm.combine(&!&cfg_interface[13]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[9]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[10]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[12]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[13]);

        assert_eq!(present_dcm.bits.len(), 1);
        assert_eq!(present_dcm, present_dcm_clkgen);
        cfg_interface[18].assert_empty();
        cfg_interface[18] = present_dcm;
        ctx.insert(tile, bel, "INTERFACE", xlat_bitvec_legacy(cfg_interface));
    }

    let bel = "CMT";
    let mut diff = ctx.get_diff_legacy(tile, bel, "PRESENT_ANY_DCM", "1");
    diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "BG"), 0, 1);
    diff.assert_empty();

    for i in 0..16 {
        ctx.collect_enum_default_legacy_ocd(
            tile,
            bel,
            &format!("MUX.HCLK{i}"),
            &[
                "CKINT",
                "DCM0_CLK0",
                "DCM0_CLK90",
                "DCM0_CLK180",
                "DCM0_CLK270",
                "DCM0_CLK2X",
                "DCM0_CLK2X180",
                "DCM0_CLKDV",
                "DCM0_CLKFX",
                "DCM0_CLKFX180",
                "DCM0_CONCUR",
                "DCM1_CLK0",
                "DCM1_CLK90",
                "DCM1_CLK180",
                "DCM1_CLK270",
                "DCM1_CLK2X",
                "DCM1_CLK2X180",
                "DCM1_CLKDV",
                "DCM1_CLKFX",
                "DCM1_CLKFX180",
                "DCM1_CONCUR",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum_legacy(tile, bel, &format!("MUX.CASC{i}"), &["PASS", "HCLK"]);
    }
}
