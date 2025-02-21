use prjcombine_re_collector::{xlat_enum_ocd, OcdMode};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::BelId;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_inv,
    fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for bel in ["PMCD0", "PMCD1"] {
        if let Some(ctx) = FuzzCtx::try_new(session, backend, "CCM", bel, TileBits::MainAuto) {
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PMCD")]);
            for pin in ["CLKA", "CLKB", "CLKC", "CLKD"] {
                fuzz_one!(ctx, format!("{pin}_ENABLE"), "1", [
                    (mode "PMCD")
                ], [
                    (pin pin)
                ]);
            }
            for pin in ["REL", "RST"] {
                fuzz_inv!(ctx, pin, [(mode "PMCD")]);
            }
            fuzz_enum!(ctx, "EN_REL", ["FALSE", "TRUE"], [(mode "PMCD")]);
            fuzz_enum!(ctx, "RST_DEASSERT_CLK", ["CLKA", "CLKB", "CLKC", "CLKD"], [(mode "PMCD")]);
            fuzz_enum!(ctx, "CCM_VREG_ENABLE", ["FALSE", "TRUE"], [
                (tile_mutex "VREG", bel),
                (mode "PMCD")
            ]);
            fuzz_multi!(ctx, "CCM_VBG_SEL", "", 4, [
                (tile_mutex "VREG", bel),
                (mode "PMCD")
            ], (attr_bin "CCM_VBG_SEL"));
            fuzz_multi!(ctx, "CCM_VBG_PD", "", 2, [
                (tile_mutex "VREG", bel),
                (mode "PMCD")
            ], (attr_bin "CCM_VBG_PD"));
            fuzz_multi!(ctx, "CCM_VREG_PHASE_MARGIN", "", 3, [
                (tile_mutex "VREG", bel),
                (mode "PMCD")
            ], (attr_bin "CCM_VREG_PHASE_MARGIN"));

            for (pin, abc) in [
                ("CLKA", 'A'),
                ("CLKB", 'A'),
                ("CLKC", 'B'),
                ("CLKD", 'B'),
                ("REL", 'C'),
            ] {
                let bel_ccm = BelId::from_idx(3);
                let bel_other = BelId::from_idx(if bel == "PMCD1" { 0 } else { 1 });
                let opin = if pin == "REL" { "CLKA" } else { "REL" };
                for rpin in [pin.to_string(), format!("{pin}_TEST")] {
                    for i in 0..8 {
                        fuzz_one!(ctx, &rpin, format!("HCLK{i}"), [
                            (global_mutex "HCLK_DCM", "USE"),
                            (mode "PMCD"),
                            (pin pin),
                            (pin opin),
                            (mutex format!("{pin}_OUT"), &rpin),
                            (mutex format!("{pin}_IN"), format!("HCLK{i}")),
                            (mutex format!("{opin}_OUT"), "HOLD"),
                            (mutex format!("{opin}_IN"), format!("HCLK{i}")),
                            (pip (bel_pin bel_ccm, format!("HCLK{i}")), (pin opin))
                        ], [
                            (pip (bel_pin bel_ccm, format!("HCLK{i}")), (pin rpin))
                        ]);
                    }
                    for i in 0..16 {
                        fuzz_one!(ctx, &rpin, format!("GIOB{i}"), [
                            (global_mutex "HCLK_DCM", "USE"),
                            (mode "PMCD"),
                            (pin pin),
                            (pin opin),
                            (mutex format!("{pin}_OUT"), &rpin),
                            (mutex format!("{pin}_IN"), format!("GIOB{i}")),
                            (mutex format!("{opin}_OUT"), "HOLD"),
                            (mutex format!("{opin}_IN"), format!("GIOB{i}")),
                            (pip (bel_pin bel_ccm, format!("GIOB{i}")), (pin opin))
                        ], [
                            (pip (bel_pin bel_ccm, format!("GIOB{i}")), (pin rpin))
                        ]);
                    }
                    for i in 0..4 {
                        fuzz_one!(ctx, &rpin, format!("MGT{i}"), [
                            (global_mutex "HCLK_DCM", "USE"),
                            (mode "PMCD"),
                            (pin pin),
                            (pin opin),
                            (mutex format!("{pin}_OUT"), &rpin),
                            (mutex format!("{pin}_IN"), format!("MGT{i}")),
                            (mutex format!("{opin}_OUT"), "HOLD"),
                            (mutex format!("{opin}_IN"), format!("MGT{i}")),
                            (pip (bel_pin bel_ccm, format!("MGT{i}")), (pin opin))
                        ], [
                            (pip (bel_pin bel_ccm, format!("MGT{i}")), (pin rpin))
                        ]);
                    }
                    for i in 0..24 {
                        fuzz_one!(ctx, &rpin, format!("BUSIN{i}"), [
                            (mode "PMCD"),
                            (pin pin),
                            (pin opin),
                            (mutex format!("{pin}_OUT"), &rpin),
                            (mutex format!("{pin}_IN"), format!("BUSIN{i}")),
                            (mutex format!("{opin}_OUT"), "HOLD"),
                            (mutex format!("{opin}_IN"), format!("BUSIN{i}")),
                            (pip (bel_pin bel_ccm, format!("BUSIN{i}")), (pin opin))
                        ], [
                            (pip (bel_pin bel_ccm, format!("BUSIN{i}")), (pin rpin))
                        ]);
                    }
                    for i in 0..4 {
                        fuzz_one!(ctx, &rpin, format!("CKINT{abc}{i}"), [
                            (mode "PMCD"),
                            (pin pin),
                            (mutex format!("{pin}_OUT"), &rpin),
                            (mutex format!("{pin}_IN"), format!("CKINT{abc}{i}")),
                            (tile_mutex format!("CKINT{abc}{i}"), format!("{bel}_{rpin}"))
                        ], [
                            (pip (pin format!("CKINT{abc}{i}")), (pin rpin))
                        ]);
                    }
                    if abc != 'C' {
                        fuzz_one!(ctx, &rpin, "CLKA1D8", [
                            (mode "PMCD"),
                            (pin pin),
                            (mutex format!("{pin}_OUT"), &rpin),
                            (mutex format!("{pin}_IN"), "CLKA1D8")
                        ], [
                            (pip (bel_pin bel_other, "CLKA1D8"), (pin rpin))
                        ]);
                    }
                }
            }
            fuzz_one!(ctx, "REL", "REL_INT", [
                (mode "PMCD"),
                (pin "REL"),
                (mutex "REL_OUT", "REL"),
                (mutex "REL_IN", "REL_INT")
            ], [
                (pip (pin "REL_INT"), (pin "REL"))
            ]);
        }
    }
    if let Some(ctx) = FuzzCtx::try_new(session, backend, "CCM", "DPM", TileBits::MainAuto) {
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "DPM")]);
        for pin in [
            "ENOSC0", "ENOSC1", "ENOSC2", "OUTSEL0", "OUTSEL1", "OUTSEL2", "HFSEL0", "HFSEL1",
            "HFSEL2", "RST", "SELSKEW", "FREEZE",
        ] {
            fuzz_inv!(ctx, pin, [(mode "DPM")]);
        }
        fuzz_enum!(ctx, "CCM_VREG_ENABLE", ["FALSE", "TRUE"], [
            (tile_mutex "VREG", "DPM"),
            (mode "DPM")
        ]);
        fuzz_multi!(ctx, "CCM_VBG_SEL", "", 4, [
            (tile_mutex "VREG", "DPM"),
            (mode "DPM")
        ], (attr_bin "CCM_VBG_SEL"));
        fuzz_multi!(ctx, "CCM_VBG_PD", "", 2, [
            (tile_mutex "VREG", "DPM"),
            (mode "DPM")
        ], (attr_bin "CCM_VBG_PD"));
        fuzz_multi!(ctx, "CCM_VREG_PHASE_MARGIN", "", 3, [
            (tile_mutex "VREG", "DPM"),
            (mode "DPM")
        ], (attr_bin "CCM_VREG_PHASE_MARGIN"));

        for (pin, abc) in [("REFCLK", 'A'), ("TESTCLK1", 'B'), ("TESTCLK2", 'B')] {
            let bel_ccm = BelId::from_idx(3);
            let opin = if pin == "REFCLK" {
                "TESTCLK1"
            } else {
                "REFCLK"
            };
            for rpin in [pin.to_string(), format!("{pin}_TEST")] {
                for i in 0..8 {
                    fuzz_one!(ctx, &rpin, format!("HCLK{i}"), [
                        (global_mutex "HCLK_DCM", "USE"),
                        (mode "DPM"),
                        (pin pin),
                        (mutex format!("{pin}_OUT"), &rpin),
                        (mutex format!("{pin}_IN"), format!("HCLK{i}")),
                        (mutex format!("{opin}_OUT"), "HOLD"),
                        (mutex format!("{opin}_IN"), format!("HCLK{i}")),
                        (pip (bel_pin bel_ccm, format!("HCLK{i}")), (pin opin))
                    ], [
                        (pip (bel_pin bel_ccm, format!("HCLK{i}")), (pin rpin))
                    ]);
                }
                for i in 0..16 {
                    fuzz_one!(ctx, &rpin, format!("GIOB{i}"), [
                        (global_mutex "HCLK_DCM", "USE"),
                        (mode "DPM"),
                        (pin pin),
                        (mutex format!("{pin}_OUT"), &rpin),
                        (mutex format!("{pin}_IN"), format!("GIOB{i}")),
                        (mutex format!("{opin}_OUT"), "HOLD"),
                        (mutex format!("{opin}_IN"), format!("GIOB{i}")),
                        (pip (bel_pin bel_ccm, format!("GIOB{i}")), (pin opin))
                    ], [
                        (pip (bel_pin bel_ccm, format!("GIOB{i}")), (pin rpin))
                    ]);
                }
                for i in 0..4 {
                    fuzz_one!(ctx, &rpin, format!("MGT{i}"), [
                        (global_mutex "HCLK_DCM", "USE"),
                        (mode "DPM"),
                        (pin pin),
                        (mutex format!("{pin}_OUT"), &rpin),
                        (mutex format!("{pin}_IN"), format!("MGT{i}")),
                        (mutex format!("{opin}_OUT"), "HOLD"),
                        (mutex format!("{opin}_IN"), format!("MGT{i}")),
                        (pip (bel_pin bel_ccm, format!("MGT{i}")), (pin opin))
                    ], [
                        (pip (bel_pin bel_ccm, format!("MGT{i}")), (pin rpin))
                    ]);
                }
                for i in 0..24 {
                    fuzz_one!(ctx, &rpin, format!("BUSIN{i}"), [
                        (mode "DPM"),
                        (pin pin),
                        (mutex format!("{pin}_OUT"), &rpin),
                        (mutex format!("{pin}_IN"), format!("BUSIN{i}")),
                        (mutex format!("{opin}_OUT"), "HOLD"),
                        (mutex format!("{opin}_IN"), format!("BUSIN{i}")),
                        (pip (bel_pin bel_ccm, format!("BUSIN{i}")), (pin opin))
                    ], [
                        (pip (bel_pin bel_ccm, format!("BUSIN{i}")), (pin rpin))
                    ]);
                }
                for i in 0..4 {
                    fuzz_one!(ctx, &rpin, format!("CKINT{abc}{i}"), [
                        (mode "DPM"),
                        (pin pin),
                        (mutex format!("{pin}_OUT"), &rpin),
                        (mutex format!("{pin}_IN"), format!("CKINT{abc}{i}")),
                        (mutex format!("CKINT{abc}{i}"), &rpin)
                    ], [
                        (pip (pin format!("CKINT{abc}{i}")), (pin rpin))
                    ]);
                }
            }
        }
    }
    if let Some(ctx) = FuzzCtx::try_new(session, backend, "CCM", "CCM", TileBits::MainAuto) {
        for i in 0..12 {
            let opin = format!("TO_BUFG{i}");
            for (name, bel, pin) in [
                ("PMCD0_CLKA1", BelId::from_idx(0), "CLKA1"),
                ("PMCD0_CLKA1D2", BelId::from_idx(0), "CLKA1D2"),
                ("PMCD0_CLKA1D4", BelId::from_idx(0), "CLKA1D4"),
                ("PMCD0_CLKA1D8", BelId::from_idx(0), "CLKA1D8"),
                ("PMCD0_CLKB1", BelId::from_idx(0), "CLKB1"),
                ("PMCD0_CLKC1", BelId::from_idx(0), "CLKC1"),
                ("PMCD0_CLKD1", BelId::from_idx(0), "CLKD1"),
                ("PMCD1_CLKA1", BelId::from_idx(1), "CLKA1"),
                ("PMCD1_CLKA1D2", BelId::from_idx(1), "CLKA1D2"),
                ("PMCD1_CLKA1D4", BelId::from_idx(1), "CLKA1D4"),
                ("PMCD1_CLKA1D8", BelId::from_idx(1), "CLKA1D8"),
                ("PMCD1_CLKB1", BelId::from_idx(1), "CLKB1"),
                ("PMCD1_CLKC1", BelId::from_idx(1), "CLKC1"),
                ("PMCD1_CLKD1", BelId::from_idx(1), "CLKD1"),
                ("DPM_REFCLKOUT", BelId::from_idx(2), "REFCLKOUT"),
                ("DPM_OSCOUT1", BelId::from_idx(2), "OSCOUT1"),
                ("DPM_OSCOUT2", BelId::from_idx(2), "OSCOUT2"),
                ("CKINT", BelId::from_idx(3), "CKINT"),
            ] {
                fuzz_one!(ctx, format!("MUX.TO_BUFG{i}"), name, [
                    (tile_mutex &opin, name)
                ], [
                    (pip (bel_pin bel, pin), (pin &opin))
                ]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let ccm = edev.egrid.db.get_node("CCM");
    if edev.egrid.node_index[ccm].is_empty() {
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
