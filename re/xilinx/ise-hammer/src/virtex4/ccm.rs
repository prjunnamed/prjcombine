use prjcombine_re_fpga_hammer::{OcdMode, xlat_enum_ocd};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::bels;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, "CCM") else {
        return;
    };
    for idx in 0..2 {
        let mut bctx = ctx.bel(bels::PMCD[idx]);
        bctx.test_manual("PRESENT", "1").mode("PMCD").commit();
        for pin in ["CLKA", "CLKB", "CLKC", "CLKD"] {
            bctx.mode("PMCD")
                .test_manual(format!("{pin}_ENABLE"), "1")
                .pin(pin)
                .commit();
        }
        for pin in ["REL", "RST"] {
            bctx.mode("PMCD").test_inv(pin);
        }
        bctx.mode("PMCD").test_enum("EN_REL", &["FALSE", "TRUE"]);
        bctx.mode("PMCD")
            .test_enum("RST_DEASSERT_CLK", &["CLKA", "CLKB", "CLKC", "CLKD"]);
        bctx.mode("PMCD")
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_enum("CCM_VREG_ENABLE", &["FALSE", "TRUE"]);
        bctx.mode("PMCD")
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_multi_attr_bin("CCM_VBG_SEL", 4);
        bctx.mode("PMCD")
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_multi_attr_bin("CCM_VBG_PD", 2);
        bctx.mode("PMCD")
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_multi_attr_bin("CCM_VREG_PHASE_MARGIN", 3);

        for (pin, abc) in [
            ("CLKA", 'A'),
            ("CLKB", 'A'),
            ("CLKC", 'B'),
            ("CLKD", 'B'),
            ("REL", 'C'),
        ] {
            let bel_ccm = bels::CCM;
            let bel_other = bels::PMCD[idx ^ 1];
            let opin = if pin == "REL" { "CLKA" } else { "REL" };
            for rpin in [pin.to_string(), format!("{pin}_TEST")] {
                for i in 0..8 {
                    bctx.mode("PMCD")
                        .global_mutex("HCLK_DCM", "USE")
                        .pin(pin)
                        .pin(opin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("HCLK{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("HCLK{i}"))
                        .pip(opin, (bel_ccm, format!("HCLK{i}")))
                        .test_manual(&rpin, format!("HCLK{i}"))
                        .pip(&rpin, (bel_ccm, format!("HCLK{i}")))
                        .commit();
                }
                for i in 0..16 {
                    bctx.mode("PMCD")
                        .global_mutex("HCLK_DCM", "USE")
                        .pin(pin)
                        .pin(opin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("GIOB{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("GIOB{i}"))
                        .pip(opin, (bel_ccm, format!("GIOB{i}")))
                        .test_manual(&rpin, format!("GIOB{i}"))
                        .pip(&rpin, (bel_ccm, format!("GIOB{i}")))
                        .commit();
                }
                for i in 0..4 {
                    bctx.mode("PMCD")
                        .global_mutex("HCLK_DCM", "USE")
                        .pin(pin)
                        .pin(opin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("MGT{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("MGT{i}"))
                        .pip(opin, (bel_ccm, format!("MGT{i}")))
                        .test_manual(&rpin, format!("MGT{i}"))
                        .pip(&rpin, (bel_ccm, format!("MGT{i}")))
                        .commit();
                }
                for i in 0..24 {
                    bctx.mode("PMCD")
                        .pin(pin)
                        .pin(opin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("BUSIN{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("BUSIN{i}"))
                        .pip(opin, (bel_ccm, format!("BUSIN{i}")))
                        .test_manual(&rpin, format!("BUSIN{i}"))
                        .pip(&rpin, (bel_ccm, format!("BUSIN{i}")))
                        .commit();
                }
                for i in 0..4 {
                    bctx.mode("PMCD")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("CKINT{abc}{i}"))
                        .tile_mutex(format!("CKINT{abc}{i}"), format!("PMCD{idx}_{rpin}"))
                        .test_manual(&rpin, format!("CKINT{abc}{i}"))
                        .pip(&rpin, format!("CKINT{abc}{i}"))
                        .commit();
                }
                if abc != 'C' {
                    bctx.mode("PMCD")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), "CLKA1D8")
                        .test_manual(&rpin, "CLKA1D8")
                        .pip(&rpin, (bel_other, "CLKA1D8"))
                        .commit();
                }
            }
        }
        bctx.mode("PMCD")
            .pin("REL")
            .mutex("REL_OUT", "REL")
            .mutex("REL_IN", "REL_INT")
            .test_manual("REL", "REL_INT")
            .pip("REL", "REL_INT")
            .commit();
    }
    {
        let mut bctx = ctx.bel(bels::DPM);
        bctx.build()
            .test_manual("PRESENT", "1")
            .mode("DPM")
            .commit();
        for pin in [
            "ENOSC0", "ENOSC1", "ENOSC2", "OUTSEL0", "OUTSEL1", "OUTSEL2", "HFSEL0", "HFSEL1",
            "HFSEL2", "RST", "SELSKEW", "FREEZE",
        ] {
            bctx.mode("DPM").test_inv(pin);
        }
        bctx.mode("DPM")
            .tile_mutex("VREG", "DPM")
            .test_enum("CCM_VREG_ENABLE", &["FALSE", "TRUE"]);
        bctx.mode("DPM")
            .tile_mutex("VREG", "DPM")
            .test_multi_attr_bin("CCM_VBG_SEL", 4);
        bctx.mode("DPM")
            .tile_mutex("VREG", "DPM")
            .test_multi_attr_bin("CCM_VBG_PD", 2);
        bctx.mode("DPM")
            .tile_mutex("VREG", "DPM")
            .test_multi_attr_bin("CCM_VREG_PHASE_MARGIN", 3);

        for (pin, abc) in [("REFCLK", 'A'), ("TESTCLK1", 'B'), ("TESTCLK2", 'B')] {
            let bel_ccm = bels::CCM;
            let opin = if pin == "REFCLK" {
                "TESTCLK1"
            } else {
                "REFCLK"
            };
            for rpin in [pin.to_string(), format!("{pin}_TEST")] {
                for i in 0..8 {
                    bctx.mode("DPM")
                        .global_mutex("HCLK_DCM", "USE")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("HCLK{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("HCLK{i}"))
                        .pip(opin, (bel_ccm, format!("HCLK{i}")))
                        .test_manual(&rpin, format!("HCLK{i}"))
                        .pip(&rpin, (bel_ccm, format!("HCLK{i}")))
                        .commit();
                }
                for i in 0..16 {
                    bctx.mode("DPM")
                        .global_mutex("HCLK_DCM", "USE")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("GIOB{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("GIOB{i}"))
                        .pip(opin, (bel_ccm, format!("GIOB{i}")))
                        .test_manual(&rpin, format!("GIOB{i}"))
                        .pip(&rpin, (bel_ccm, format!("GIOB{i}")))
                        .commit();
                }
                for i in 0..4 {
                    bctx.mode("DPM")
                        .global_mutex("HCLK_DCM", "USE")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("MGT{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("MGT{i}"))
                        .pip(opin, (bel_ccm, format!("MGT{i}")))
                        .test_manual(&rpin, format!("MGT{i}"))
                        .pip(&rpin, (bel_ccm, format!("MGT{i}")))
                        .commit();
                }
                for i in 0..24 {
                    bctx.mode("DPM")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("BUSIN{i}"))
                        .mutex(format!("{opin}_OUT"), "HOLD")
                        .mutex(format!("{opin}_IN"), format!("BUSIN{i}"))
                        .pip(opin, (bel_ccm, format!("BUSIN{i}")))
                        .test_manual(&rpin, format!("BUSIN{i}"))
                        .pip(&rpin, (bel_ccm, format!("BUSIN{i}")))
                        .commit();
                }
                for i in 0..4 {
                    bctx.mode("DPM")
                        .pin(pin)
                        .mutex(format!("{pin}_OUT"), &rpin)
                        .mutex(format!("{pin}_IN"), format!("CKINT{abc}{i}"))
                        .mutex(format!("CKINT{abc}{i}"), &rpin)
                        .test_manual(&rpin, format!("CKINT{abc}{i}"))
                        .pip(&rpin, format!("CKINT{abc}{i}"))
                        .commit();
                }
            }
        }
    }
    {
        let mut bctx = ctx.bel(bels::CCM);
        for i in 0..12 {
            let opin = format!("TO_BUFG{i}");
            for (name, bel, pin) in [
                ("PMCD0_CLKA1", bels::PMCD0, "CLKA1"),
                ("PMCD0_CLKA1D2", bels::PMCD0, "CLKA1D2"),
                ("PMCD0_CLKA1D4", bels::PMCD0, "CLKA1D4"),
                ("PMCD0_CLKA1D8", bels::PMCD0, "CLKA1D8"),
                ("PMCD0_CLKB1", bels::PMCD0, "CLKB1"),
                ("PMCD0_CLKC1", bels::PMCD0, "CLKC1"),
                ("PMCD0_CLKD1", bels::PMCD0, "CLKD1"),
                ("PMCD1_CLKA1", bels::PMCD1, "CLKA1"),
                ("PMCD1_CLKA1D2", bels::PMCD1, "CLKA1D2"),
                ("PMCD1_CLKA1D4", bels::PMCD1, "CLKA1D4"),
                ("PMCD1_CLKA1D8", bels::PMCD1, "CLKA1D8"),
                ("PMCD1_CLKB1", bels::PMCD1, "CLKB1"),
                ("PMCD1_CLKC1", bels::PMCD1, "CLKC1"),
                ("PMCD1_CLKD1", bels::PMCD1, "CLKD1"),
                ("DPM_REFCLKOUT", bels::DPM, "REFCLKOUT"),
                ("DPM_OSCOUT1", bels::DPM, "OSCOUT1"),
                ("DPM_OSCOUT2", bels::DPM, "OSCOUT2"),
                ("CKINT", bels::CCM, "CKINT"),
            ] {
                bctx.build()
                    .tile_mutex(&opin, name)
                    .test_manual(format!("MUX.TO_BUFG{i}"), name)
                    .pip(&opin, (bel, pin))
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let ccm = edev.egrid.db.get_tile_class("CCM");
    if edev.egrid.tile_index[ccm].is_empty() {
        return;
    }
    let tile = "CCM";
    for bel in ["PMCD0", "PMCD1"] {
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_int_inv(&["INT"; 4], tile, bel, "RST", false);
        ctx.collect_inv(tile, bel, "REL");
        ctx.collect_bit_wide(tile, bel, "CLKA_ENABLE", "1");
        ctx.collect_bit(tile, bel, "CLKB_ENABLE", "1");
        ctx.collect_bit(tile, bel, "CLKC_ENABLE", "1");
        ctx.collect_bit(tile, bel, "CLKD_ENABLE", "1");
        ctx.collect_enum_bool(tile, bel, "EN_REL", "FALSE", "TRUE");
        ctx.collect_enum(
            tile,
            bel,
            "RST_DEASSERT_CLK",
            &["CLKA", "CLKB", "CLKC", "CLKD"],
        );

        for (pin, abc) in [
            ("CLKA", 'A'),
            ("CLKB", 'A'),
            ("CLKC", 'B'),
            ("CLKD", 'B'),
            ("REL", 'C'),
        ] {
            let mut diffs = vec![];
            for i in 0..8 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("HCLK{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("HCLK{i}"))
                );
                diffs.push((format!("HCLK{i}"), diff));
            }
            for i in 0..16 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("GIOB{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("GIOB{i}"))
                );
                diffs.push((format!("GIOB{i}"), diff));
            }
            for i in 0..4 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("MGT{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("MGT{i}"))
                );
                diffs.push((format!("MGT{i}"), diff));
            }
            for i in 0..24 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("BUSIN{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("BUSIN{i}"))
                );
                diffs.push((format!("BUSIN{i}"), diff));
            }
            for i in 0..4 {
                let mut diff = ctx.state.get_diff(tile, bel, pin, format!("CKINT{abc}{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("CKINT{abc}{i}"))
                );
                if i < 2 {
                    let item = ctx.item_int_inv(&["INT"; 4], tile, bel, &format!("CKINT{abc}{i}"));
                    diff.apply_bit_diff(&item, false, true);
                }
                diffs.push((format!("CKINT{abc}{i}"), diff));
            }
            if abc != 'C' {
                let diff = ctx.state.get_diff(tile, bel, pin, "CLKA1D8");
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), "CLKA1D8")
                );
                diffs.push(("CLKA1D8".to_string(), diff));
            } else {
                let diff = ctx.state.get_diff(tile, bel, pin, "REL_INT");
                diffs.push(("REL_INT".to_string(), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{pin}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
    }
    {
        let bel = "DPM";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_int_inv(&["INT"; 4], tile, bel, "RST", false);
        for pin in [
            "ENOSC0", "ENOSC1", "ENOSC2", "OUTSEL0", "OUTSEL1", "OUTSEL2", "HFSEL0", "HFSEL1",
            "HFSEL2", "SELSKEW", "FREEZE",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }
        for (pin, abc) in [("REFCLK", 'A'), ("TESTCLK1", 'B'), ("TESTCLK2", 'B')] {
            let mut diffs = vec![];
            for i in 0..8 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("HCLK{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("HCLK{i}"))
                );
                diffs.push((format!("HCLK{i}"), diff));
            }
            for i in 0..16 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("GIOB{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("GIOB{i}"))
                );
                diffs.push((format!("GIOB{i}"), diff));
            }
            for i in 0..4 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("MGT{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("MGT{i}"))
                );
                diffs.push((format!("MGT{i}"), diff));
            }
            for i in 0..24 {
                let diff = ctx.state.get_diff(tile, bel, pin, format!("BUSIN{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("BUSIN{i}"))
                );
                diffs.push((format!("BUSIN{i}"), diff));
            }
            for i in 0..4 {
                let mut diff = ctx.state.get_diff(tile, bel, pin, format!("CKINT{abc}{i}"));
                assert_eq!(
                    diff,
                    ctx.state
                        .get_diff(tile, bel, format!("{pin}_TEST"), format!("CKINT{abc}{i}"))
                );
                if i < 2 {
                    let item = ctx.item_int_inv(&["INT"; 4], tile, bel, &format!("CKINT{abc}{i}"));
                    diff.apply_bit_diff(&item, false, true);
                }
                diffs.push((format!("CKINT{abc}{i}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.{pin}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
    }
    for bel in ["PMCD0", "PMCD1", "DPM"] {
        let vreg_enable = ctx.extract_enum_bool(tile, bel, "CCM_VREG_ENABLE", "FALSE", "TRUE");
        ctx.tiledb.insert(tile, "CCM", "VREG_ENABLE", vreg_enable);
        // ???
        for attr in ["CCM_VBG_SEL", "CCM_VBG_PD", "CCM_VREG_PHASE_MARGIN"] {
            for diff in ctx.state.get_diffs(tile, bel, attr, "") {
                diff.assert_empty();
            }
        }
    }
    {
        let bel = "CCM";
        for i in 0..12 {
            ctx.collect_enum_ocd(
                tile,
                bel,
                &format!("MUX.TO_BUFG{i}"),
                &[
                    "PMCD0_CLKA1",
                    "PMCD0_CLKA1D2",
                    "PMCD0_CLKA1D4",
                    "PMCD0_CLKA1D8",
                    "PMCD0_CLKB1",
                    "PMCD0_CLKC1",
                    "PMCD0_CLKD1",
                    "PMCD1_CLKA1",
                    "PMCD1_CLKA1D2",
                    "PMCD1_CLKA1D4",
                    "PMCD1_CLKA1D8",
                    "PMCD1_CLKB1",
                    "PMCD1_CLKC1",
                    "PMCD1_CLKD1",
                    "DPM_REFCLKOUT",
                    "DPM_OSCOUT1",
                    "DPM_OSCOUT2",
                    "CKINT",
                ],
                OcdMode::Mux,
            );
        }
    }
}
