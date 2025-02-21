use std::collections::HashSet;

use bitvec::vec::BitVec;
use prjcombine_re_collector::{xlat_bit, xlat_bit_wide, xlat_enum, xlat_enum_ocd, OcdMode};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::{BelId, Dir};
use prjcombine_spartan6::grid::Gts;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    if devdata_only {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "PCILOGICSE",
            "PCILOGICSE",
            TileBits::MainAuto,
        );
        fuzz_one!(ctx, "PRESENT", "1", [
            (no_global_opt "PCI_CE_DELAY_LEFT")
        ], [
            (mode "PCILOGICSE")
        ]);
        return;
    }
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    {
        let ctx = FuzzCtx::new(session, backend, "HCLK", "HCLK", TileBits::Hclk);
        for i in 0..16 {
            let gclk_i = format!("GCLK{i}_I");
            let gclk_o_d = format!("GCLK{i}_O_D");
            let gclk_o_u = format!("GCLK{i}_O_U");
            fuzz_one!(
                ctx,
                &gclk_o_d,
                "1",
                [(special TileKV::HclkHasInt(Dir::S))],
                [(pip(pin & gclk_i), (pin & gclk_o_d))]
            );
            fuzz_one!(
                ctx,
                &gclk_o_u,
                "1",
                [(special TileKV::HclkHasInt(Dir::N))],
                [(pip(pin & gclk_i), (pin & gclk_o_u))]
            );
        }
    }
    if let Some(ctx) = FuzzCtx::try_new(
        session,
        backend,
        "HCLK_H_MIDBUF",
        "HCLK_H_MIDBUF",
        TileBits::Null,
    ) {
        for i in 0..16 {
            fuzz_one!(
                ctx,
                format!("GCLK{i}_M"),
                "1",
                [],
                [(
                    pip(pin & format!("GCLK{i}_I")),
                    (pin & format!("GCLK{i}_M"))
                )]
            );
            fuzz_one!(
                ctx,
                format!("GCLK{i}_O"),
                "1",
                [],
                [(
                    pip(pin & format!("GCLK{i}_M")),
                    (pin & format!("GCLK{i}_O"))
                )]
            );
        }
    }
    {
        for i in 0..16 {
            for lr in ['L', 'R'] {
                let mut ctx = FuzzCtx::new(
                    session,
                    backend,
                    "HCLK_ROW",
                    format!("BUFH_{lr}{i}"),
                    TileBits::Spine(1, 1),
                );
                let obel = BelId::from_idx(32);
                fuzz_one!(ctx, "I", "BUFG", [
                    (mutex "I", "BUFG")
                ], [
                    (pip (bel_pin obel, format!("BUFG{i}")), (pin_far "I"))
                ]);
                fuzz_one!(ctx, "I", "CMT", [
                    (mutex "I", "CMT"),
                    (special TileKV::HclkHasCmt)
                ], [
                    (pip (bel_pin obel, format!("CMT{i}")), (pin_far "I"))
                ]);
                ctx.bits = TileBits::Null;
                fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFH")]);
                fuzz_one!(ctx, "OUTPUT", "1", [], [
                    (pip (pin "O"), (pin_far "O"))
                ]);
                fuzz_one!(ctx, "INPUT", "1", [], [
                    (pip (pin_far "I"), (pin "I"))
                ]);
            }
        }
    }
    {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "HCLK_V_MIDBUF",
            "HCLK_V_MIDBUF",
            TileBits::Null,
        );
        for i in 0..16 {
            fuzz_one!(
                ctx,
                format!("GCLK{i}_M"),
                "1",
                [],
                [(
                    pip(pin & format!("GCLK{i}_I")),
                    (pin & format!("GCLK{i}_M"))
                )]
            );
            fuzz_one!(
                ctx,
                format!("GCLK{i}_O"),
                "1",
                [],
                [(
                    pip(pin & format!("GCLK{i}_M")),
                    (pin & format!("GCLK{i}_O"))
                )]
            );
        }
    }
    {
        for i in 0..16 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                "CLKC",
                format!("BUFGMUX{i}"),
                TileBits::MainAuto,
            );
            fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BUFGMUX")]);
            fuzz_inv!(ctx, "S", [(mode "BUFGMUX")]);
            fuzz_enum!(ctx, "CLK_SEL_TYPE", ["SYNC", "ASYNC"], [(mode "BUFGMUX")]);
            fuzz_enum!(ctx, "DISABLE_ATTR", ["LOW", "HIGH"], [(mode "BUFGMUX")]);
        }
        let mut ctx = FuzzCtx::new(session, backend, "CLKC", "CLKC", TileBits::MainAuto);
        for i in 0..16 {
            for inp in [
                format!("CKPIN_H{i}"),
                format!("CKPIN_V{i}"),
                format!("CMT_D{i}"),
                format!("CMT_U{i}"),
            ] {
                fuzz_one!(ctx, format!("IMUX{i}"), &inp, [
                    (tile_mutex format!("IMUX{i}"), &inp)
                ], [
                    (pip (pin &inp), (pin format!("MUX{i}")))
                ]);
            }
        }
        ctx.bel = BelId::from_idx(17);
        for (out, altout) in [
            ("OUTL_CLKOUT0", "OUTL_CLKOUT1"),
            ("OUTL_CLKOUT1", "OUTL_CLKOUT0"),
            ("OUTR_CLKOUT0", "OUTR_CLKOUT1"),
            ("OUTR_CLKOUT1", "OUTR_CLKOUT0"),
            ("OUTD_CLKOUT0", "OUTD_CLKOUT1"),
            ("OUTD_CLKOUT1", "OUTD_CLKOUT0"),
            ("OUTU_CLKOUT0", "OUTU_CLKOUT1"),
            ("OUTU_CLKOUT1", "OUTU_CLKOUT0"),
        ] {
            for inp in [
                "PLL0D_CLKOUT0",
                "PLL0D_CLKOUT1",
                "PLL1D_CLKOUT0",
                "PLL1D_CLKOUT1",
                "PLL0U_CLKOUT0",
                "PLL0U_CLKOUT1",
                "PLL1U_CLKOUT0",
                "PLL1U_CLKOUT1",
            ] {
                if out.starts_with("OUTU") && (inp.starts_with("PLL0U") || inp.starts_with("PLL1U"))
                {
                    continue;
                }
                if out.starts_with("OUTD") && (inp.starts_with("PLL0D") || inp.starts_with("PLL1D"))
                {
                    continue;
                }
                let mut extras = vec![];
                if out.starts_with("OUTD") {
                    let tt = if edev.grid.rows.len() < 128 {
                        "PLL_BUFPLL_OUT1"
                    } else {
                        "PLL_BUFPLL_OUT0"
                    };

                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::BufpllPll(Dir::S, tt),
                        tt,
                        "PLL_BUFPLL",
                        format!("CLKC_CLKOUT{i}", i = &out[11..]),
                        "1",
                    ));
                }
                fuzz_one_extras!(ctx, out, inp, [
                    (global_mutex_site "BUFPLL_CLK"),
                    (tile_mutex out, inp),
                    (tile_mutex altout, inp),
                    (pip (pin inp), (pin altout))
                ], [
                    (pip (pin inp), (pin out))
                ], extras);
            }
        }
        for out in [
            "OUTL_LOCKED0",
            "OUTL_LOCKED1",
            "OUTR_LOCKED0",
            "OUTR_LOCKED1",
            "OUTD_LOCKED",
            "OUTU_LOCKED",
        ] {
            for inp in [
                "PLL0D_LOCKED",
                "PLL1D_LOCKED",
                "PLL0U_LOCKED",
                "PLL1U_LOCKED",
            ] {
                if out.starts_with("OUTU") && (inp.starts_with("PLL0U") || inp.starts_with("PLL1U"))
                {
                    continue;
                }
                if out.starts_with("OUTD") && (inp.starts_with("PLL0D") || inp.starts_with("PLL1D"))
                {
                    continue;
                }
                fuzz_one!(ctx, out, inp, [
                    (tile_mutex out, inp)
                ], [
                    (pip (pin inp), (pin out))
                ]);
            }
        }
    }
    for (i, tile) in ["PLL_BUFPLL_OUT0", "PLL_BUFPLL_OUT1"]
        .into_iter()
        .enumerate()
    {
        if let Some(ctx) =
            FuzzCtx::try_new(session, backend, tile, "PLL_BUFPLL", TileBits::Spine(7, 1))
        {
            for out in ["CLKOUT0", "CLKOUT1", "LOCKED"] {
                for ud in ['U', 'D'] {
                    fuzz_one!(ctx, format!("PLL{i}_{out}_{ud}"), "1", [
                    ], [
                        (pip (pin out), (pin &format!("{out}_{ud}")))
                    ]);
                }
            }
        }
    }
    for tile in [
        "DCM_BUFPLL_BUF_BOT",
        "DCM_BUFPLL_BUF_BOT_MID",
        "DCM_BUFPLL_BUF_TOP",
        "DCM_BUFPLL_BUF_TOP_MID",
    ] {
        if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile, tile, TileBits::Spine(7, 1))
        {
            ctx.bel_name = "DCM_BUFPLL".into();
            ctx.tile_name = "DCM_BUFPLL".into();
            for src in ["PLL0", "PLL1", "CLKC"] {
                let (bufs_d, bufs_u) = match (tile, edev.grid.rows.len() / 64) {
                    ("DCM_BUFPLL_BUF_BOT", _) => (vec![], vec!["PLL_BUFPLL_OUT1"]),
                    ("DCM_BUFPLL_BUF_BOT_MID", 2) => {
                        (vec!["PLL_BUFPLL_OUT1"], vec!["PLL_BUFPLL_OUT0"])
                    }
                    ("DCM_BUFPLL_BUF_BOT_MID", 3) => (
                        vec!["PLL_BUFPLL_OUT1", "PLL_BUFPLL_B"],
                        vec!["PLL_BUFPLL_OUT0", "PLL_BUFPLL_B"],
                    ),
                    ("DCM_BUFPLL_BUF_TOP", 1) => (vec![], vec!["PLL_BUFPLL_OUT1"]),
                    ("DCM_BUFPLL_BUF_TOP", 2 | 3) => (vec![], vec!["PLL_BUFPLL_OUT0"]),
                    ("DCM_BUFPLL_BUF_TOP_MID", 2) => {
                        (vec!["PLL_BUFPLL_OUT0"], vec!["PLL_BUFPLL_OUT1"])
                    }
                    ("DCM_BUFPLL_BUF_TOP_MID", 3) => (
                        vec!["PLL_BUFPLL_OUT0", "PLL_BUFPLL_T"],
                        vec!["PLL_BUFPLL_OUT1", "PLL_BUFPLL_T"],
                    ),
                    _ => unreachable!(),
                };
                for wire in ["CLKOUT0", "CLKOUT1"] {
                    let mut extras = vec![];
                    for (dir, bufs) in [(Dir::S, &bufs_d), (Dir::N, &bufs_u)] {
                        for &buf in bufs {
                            if buf == "PLL_BUFPLL_OUT0" && src == "PLL0" {
                                continue;
                            }
                            if buf == "PLL_BUFPLL_OUT1" && src == "PLL1" {
                                continue;
                            }
                            extras.push(ExtraFeature::new(
                                ExtraFeatureKind::BufpllPll(dir, buf),
                                buf,
                                "PLL_BUFPLL",
                                format!("{src}_{wire}"),
                                "1",
                            ));
                        }
                    }
                    fuzz_one_extras!(ctx, format!("{src}_{wire}"), "1", [
                        (global_mutex_site "BUFPLL_CLK")
                    ], [
                        (pip (pin &format!("{src}_{wire}_I")), (pin &format!("{src}_{wire}_O")))
                    ], extras.clone());
                }
                fuzz_one!(
                    ctx,
                    format!("{src}_LOCKED"),
                    "1",
                    [],
                    [(
                        pip(pin & format!("{src}_LOCKED_I")),
                        (pin & format!("{src}_LOCKED_O"))
                    )]
                );
            }
        }
    }
    for (tile, is_lr) in [
        ("REG_B", false),
        ("REG_T", false),
        ("REG_L", true),
        ("REG_R", true),
    ] {
        for i in 0..8 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("BUFIO2_{i}"),
                TileBits::SpineEnd,
            );
            fuzz_one!(ctx, "PRESENT", "BUFIO2", [], [(mode "BUFIO2")]);
            fuzz_one!(ctx, "PRESENT", "BUFIO2_2CLK", [], [(mode "BUFIO2_2CLK")]);
            fuzz_enum!(ctx, "DIVIDE_BYPASS", ["FALSE", "TRUE"], [
                (mode "BUFIO2"),
                (attr "DIVIDE", "")
            ]);
            fuzz_enum!(ctx, "DIVIDE", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (mode "BUFIO2"),
                (attr "DIVIDE_BYPASS", "FALSE")
            ]);
            for val in ["1", "2", "3", "4", "5", "6", "7", "8"] {
                fuzz_one!(ctx, "DIVIDE.2CLK", val, [
                    (mode "BUFIO2_2CLK"),
                    (attr "POS_EDGE", ""),
                    (attr "NEG_EDGE", ""),
                    (attr "R_EDGE", "")
                ], [
                    (attr "DIVIDE", val)
                ]);
            }
            fuzz_enum!(ctx, "POS_EDGE", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (mode "BUFIO2_2CLK"),
                (attr "DIVIDE", "")
            ]);
            fuzz_enum!(ctx, "NEG_EDGE", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (mode "BUFIO2_2CLK"),
                (attr "DIVIDE", "")
            ]);
            fuzz_enum!(ctx, "R_EDGE", ["FALSE", "TRUE"], [
                (mode "BUFIO2_2CLK"),
                (attr "DIVIDE", "")
            ]);

            for (val, pin) in [
                ("CLKPIN0", format!("CLKPIN{i}")),
                ("CLKPIN1", format!("CLKPIN{ii}", ii = i ^ 1)),
                ("CLKPIN4", format!("CLKPIN{ii}", ii = i ^ 4)),
                ("CLKPIN5", format!("CLKPIN{ii}", ii = i ^ 5)),
                (
                    "DQS0",
                    format!("DQS{pn}{ii}", pn = ['P', 'N'][i & 1], ii = i >> 1),
                ),
                (
                    "DQS2",
                    format!("DQS{pn}{ii}", pn = ['P', 'N'][i & 1], ii = i >> 1 ^ 2),
                ),
                ("DFB", format!("DFB{i}")),
                (
                    "GTPCLK",
                    format!(
                        "GTPCLK{ii}",
                        ii = match (tile, i) {
                            // ???
                            ("REG_L" | "REG_R", 1) => 0,
                            ("REG_L" | "REG_R", 3) => 2,
                            _ => i,
                        }
                    ),
                ),
            ] {
                let obel = BelId::from_idx(20);
                fuzz_one!(ctx, "I", val, [
                    (mode "BUFIO2"),
                    (mutex "I", val)
                ], [
                    (pin "I"),
                    (pip (bel_pin obel, &pin), (pin "I"))
                ]);
            }
            for (val, pin) in [
                ("CLKPIN0", format!("CLKPIN{i}")),
                ("CLKPIN1", format!("CLKPIN{ii}", ii = i ^ 1)),
                ("CLKPIN4", format!("CLKPIN{ii}", ii = i ^ 4)),
                ("CLKPIN5", format!("CLKPIN{ii}", ii = i ^ 5)),
                (
                    "DQS0",
                    format!("DQS{pn}{ii}", pn = ['N', 'P'][i & 1], ii = i >> 1),
                ),
                (
                    "DQS2",
                    format!("DQS{pn}{ii}", pn = ['N', 'P'][i & 1], ii = i >> 1 ^ 2),
                ),
                ("DFB", format!("DFB{ii}", ii = i ^ 1)),
            ] {
                let obel = BelId::from_idx(20);
                fuzz_one!(ctx, "IB", val, [
                    (mode "BUFIO2_2CLK"),
                    (mutex "IB", val)
                ], [
                    (pin "IB"),
                    (pip (bel_pin obel, &pin), (pin "IB"))
                ]);
            }

            fuzz_one!(ctx, "IOCLK_ENABLE", "1", [
                (mode "BUFIO2"),
                (global_mutex "IOCLK_OUT", "TEST")
            ], [
                (pin "IOCLK"),
                (pip (pin "IOCLK"), (pin_far "IOCLK"))
            ]);
            fuzz_one!(ctx, "CMT_ENABLE", "1", [
                (mode "BUFIO2"),
                (global_mutex "BUFIO2_CMT_OUT", "TEST_BUFIO2")
            ], [
                (pin "DIVCLK"),
                (pip (pin "DIVCLK"), (pin_far "DIVCLK")),
                (pip (pin_far "DIVCLK"), (pin "CMT"))
            ]);
            fuzz_one!(ctx, "CKPIN", "DIVCLK", [
                (mode "BUFIO2"),
                (mutex "CKPIN", "DIVCLK")
            ], [
                (pin "DIVCLK"),
                (pip (pin "DIVCLK"), (pin_far "DIVCLK")),
                (pip (pin_far "DIVCLK"), (pin "CKPIN"))
            ]);
            let obel = BelId::from_idx(21);
            fuzz_one!(ctx, "CKPIN", "CLKPIN", [
                (mutex "CKPIN", "CLKPIN")
            ], [
                (pip (bel_pin obel, format!("CLKPIN{i}")), (bel_pin obel, format!("CKPIN{i}")))
            ]);
            let obel_tie = BelId::from_idx(19);
            fuzz_one!(ctx, "CKPIN", "VCC", [
                (mutex "CKPIN", "VCC")
            ], [
                (pip (bel_pin obel_tie, "HARD1"), (pin "CKPIN"))
            ]);

            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("BUFIO2FB_{i}"),
                TileBits::SpineEnd,
            );
            fuzz_one!(ctx, "PRESENT", "BUFIO2FB", [], [(mode "BUFIO2FB")]);
            fuzz_one!(ctx, "PRESENT", "BUFIO2FB_2CLK", [], [(mode "BUFIO2FB_2CLK")]);
            fuzz_enum!(ctx, "DIVIDE_BYPASS", ["FALSE", "TRUE"], [
                (mode "BUFIO2FB")
            ]);

            let obel = BelId::from_idx(20);
            for (val, pin) in [
                ("CLKPIN", format!("CLKPIN{ii}", ii = i ^ 1)),
                ("DFB", format!("DFB{i}")),
                ("CFB", format!("CFB0_{i}")),
                ("GTPFB", format!("GTPFB{i}")),
            ] {
                fuzz_one!(ctx, "I", val, [
                    (mode "BUFIO2FB"),
                    (mutex "I", val)
                ], [
                    (pin "I"),
                    (attr "INVERT_INPUTS", "FALSE"),
                    (pip (bel_pin obel, &pin), (pin "I"))
                ]);
            }
            fuzz_one!(ctx, "I", "CFB_INVERT", [
                (mode "BUFIO2FB"),
                (mutex "I", "CFB_INVERT")
            ], [
                (pin "I"),
                (attr "INVERT_INPUTS", "TRUE"),
                (pip (bel_pin obel, format!("CFB0_{i}")), (pin "I"))
            ]);

            fuzz_one!(ctx, "CMT_ENABLE", "1", [
                (mode "BUFIO2FB"),
                (global_mutex "BUFIO2_CMT_OUT", "TEST_BUFIO2FB")
            ], [
                (pin "O"),
                (pip (pin "O"), (pin "CMT"))
            ]);
        }
        for i in 0..2 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("BUFPLL{i}"),
                TileBits::SpineEnd,
            );
            fuzz_one!(ctx, "PRESENT", "1", [
                (tile_mutex "BUFPLL", "PLAIN")
            ], [
                (mode "BUFPLL")
            ]);
            fuzz_enum!(ctx, "DIVIDE", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (tile_mutex "BUFPLL", "PLAIN"),
                (mode "BUFPLL")
            ]);
            fuzz_enum!(ctx, "DATA_RATE", ["SDR", "DDR"], [
                (tile_mutex "BUFPLL", "PLAIN"),
                (mode "BUFPLL")
            ]);

            fuzz_enum!(ctx, "ENABLE_SYNC", ["FALSE", "TRUE"], [
                (tile_mutex "BUFPLL", "PLAIN"),
                (mode "BUFPLL"),
                (nopin "IOCLK")
            ]);

            let obel_out = BelId::from_idx(23);
            let obel_ins = BelId::from_idx(24);
            fuzz_one!(ctx, "ENABLE_NONE_SYNC", "1", [
                (tile_mutex "BUFPLL", format!("SINGLE{i}")),
                (global_mutex "PLLCLK", "TEST"),
                (mode "BUFPLL"),
                (attr "ENABLE_SYNC", "FALSE")
            ], [
                (pin "IOCLK"),
                (pip (pin "IOCLK"), (bel_pin obel_out, format!("PLLCLK{i}")))
            ]);

            fuzz_one!(ctx, "ENABLE_BOTH_SYNC", "1", [
                (tile_mutex "BUFPLL", format!("SINGLE{i}")),
                (global_mutex "BUFPLL_CLK", "USE"),
                (global_mutex "PLLCLK", "TEST"),
                (mode "BUFPLL"),
                (attr "ENABLE_SYNC", "TRUE"),
                (pip (bel_pin obel_ins, if is_lr {
                    format!("PLLIN{i}_CMT")
                } else {
                    "PLLIN0".to_string()}
                ), (pin "PLLIN"))
            ], [
                (pin "IOCLK"),
                (pip (pin "IOCLK"), (bel_pin obel_out, format!("PLLCLK{i}")))
            ]);

            if !is_lr {
                let obel = BelId::from_idx(16 + (i ^ 1));
                for i in 0..6 {
                    fuzz_one!(ctx, "PLLIN", format!("PLLIN{i}"), [
                        (tile_mutex "BUFPLL", "PLAIN"),
                        (tile_mutex "PLLIN", &ctx.bel_name),
                        (global_mutex "BUFPLL_CLK", "USE"),
                        (mode "BUFPLL"),
                        (mutex "PLLIN", format!("PLLIN{i}")),
                        (attr "ENABLE_SYNC", "FALSE"),
                        (pin "PLLIN"),
                        (pip (bel_pin obel_ins, format!("PLLIN{i}")), (bel_pin obel, "PLLIN"))
                    ], [
                        (pip (bel_pin obel_ins, format!("PLLIN{i}")), (pin "PLLIN"))
                    ]);
                }
                for i in 0..3 {
                    fuzz_one!(ctx, "LOCKED", format!("LOCKED{i}"), [
                        (tile_mutex "BUFPLL", "PLAIN"),
                        (mode "BUFPLL"),
                        (mutex "LOCKED", format!("LOCKED{i}"))
                    ], [
                        (pin "LOCKED"),
                        (pip (bel_pin obel_ins, format!("LOCKED{i}")), (pin "LOCKED"))
                    ]);
                }
            }

            let obel = BelId::from_idx(23);
            fuzz_one!(ctx, "ENABLE", "1", [
                (tile_mutex "BUFPLL", format!("SINGLE_{i}")),
                (mode "BUFPLL")
            ], [
                (pin "IOCLK"),
                (pip (pin "IOCLK"), (bel_pin obel, format!("PLLCLK{i}")))
            ]);
        }
        {
            let ctx = FuzzCtx::new(session, backend, tile, "BUFPLL_MCB", TileBits::SpineEnd);
            fuzz_one!(ctx, "PRESENT", "1", [
                (tile_mutex "BUFPLL", "MCB")
            ], [
                (mode "BUFPLL_MCB")
            ]);
            fuzz_enum!(ctx, "DIVIDE", ["1", "2", "3", "4", "5", "6", "7", "8"], [
                (tile_mutex "BUFPLL", "MCB"),
                (mode "BUFPLL_MCB")
            ]);
            fuzz_enum!(ctx, "LOCK_SRC", ["LOCK_TO_0", "LOCK_TO_1"], [
                (tile_mutex "BUFPLL", "MCB"),
                (mode "BUFPLL_MCB")
            ]);

            if is_lr {
                let obel = BelId::from_idx(24);
                fuzz_one!(ctx, "PLLIN", "GCLK", [
                    (tile_mutex "BUFPLL", "MCB"),
                    (mutex "PLLIN", "GCLK"),
                    (mode "BUFPLL_MCB")
                ], [
                    (pin "PLLIN0"),
                    (pin "PLLIN1"),
                    (pip (bel_pin obel, "PLLIN0_GCLK"), (pin "PLLIN0")),
                    (pip (bel_pin obel, "PLLIN1_GCLK"), (pin "PLLIN1"))
                ]);
                fuzz_one!(ctx, "PLLIN", "CMT", [
                    (tile_mutex "BUFPLL", "MCB"),
                    (mutex "PLLIN", "CMT"),
                    (mode "BUFPLL_MCB")
                ], [
                    (pin "PLLIN0"),
                    (pin "PLLIN1"),
                    (pip (bel_pin obel, "PLLIN0_CMT"), (pin "PLLIN0")),
                    (pip (bel_pin obel, "PLLIN1_CMT"), (pin "PLLIN1"))
                ]);
            }

            let obel = BelId::from_idx(23);
            fuzz_one!(ctx, "ENABLE.0", "1", [
                (tile_mutex "BUFPLL", "MCB_OUT0"),
                (mode "BUFPLL_MCB")
            ], [
                (pin "IOCLK0"),
                (pip (pin "IOCLK0"), (bel_pin obel, "PLLCLK0"))
            ]);
            fuzz_one!(ctx, "ENABLE.1", "1", [
                (tile_mutex "BUFPLL", "MCB_OUT1"),
                (mode "BUFPLL_MCB")
            ], [
                (pin "IOCLK1"),
                (pip (pin "IOCLK1"), (bel_pin obel, "PLLCLK1"))
            ]);
        }
        if !is_lr {
            let ctx = FuzzCtx::new_fake_bel(session, backend, tile, "MISC", TileBits::SpineEnd);
            let n = &tile[4..];
            fuzz_one!(ctx, "MISR_ENABLE", "1", [
                (global_opt "ENABLEMISR", "Y"),
                (global_opt "MISRRESET", "N")
            ], [
                (global_opt format!("MISR_{n}M_EN"), "Y")
            ]);
            fuzz_one!(ctx, "MISR_ENABLE_RESET", "1", [
                (global_opt "ENABLEMISR", "Y"),
                (global_opt "MISRRESET", "Y")
            ], [
                (global_opt format!("MISR_{n}M_EN"), "Y")
            ]);
        }
    }
    {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "PCILOGICSE",
            "PCILOGICSE",
            TileBits::MainAuto,
        );
        fuzz_one!(ctx, "PRESENT", "1", [
            (no_global_opt "PCI_CE_DELAY_LEFT")
        ], [
            (mode "PCILOGICSE")
        ]);
        for val in 2..=31 {
            fuzz_one!(ctx, "PRESENT", format!("TAP{val}"), [
                (global_opt "PCI_CE_DELAY_LEFT", format!("TAP{val}"))
            ], [
                (mode "PCILOGICSE")
            ]);
        }
    }
    for tile in [
        "PCI_CE_TRUNK_BUF",
        "PCI_CE_V_BUF",
        "PCI_CE_SPLIT",
        "PCI_CE_H_BUF",
    ] {
        if let Some(ctx) = FuzzCtx::try_new(session, backend, tile, tile, TileBits::Null) {
            fuzz_one!(ctx, "BUF", "1", [], [
                (pip (pin "PCI_CE_I"), (pin "PCI_CE_O"))
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };
    if devdata_only {
        let tile = "PCILOGICSE";
        let bel = "PCILOGICSE";
        let default = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        let item = ctx.tiledb.item(tile, bel, "PCI_CE_DELAY");
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
                ctx.insert_device_data("PCI_CE_DELAY", k.clone());
                break;
            }
        }
        return;
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..16 {
            ctx.collect_bit(tile, bel, &format!("GCLK{i}_O_D"), "1");
            ctx.collect_bit(tile, bel, &format!("GCLK{i}_O_U"), "1");
        }
        // TODO: mask bits
    }
    {
        let tile = "HCLK_ROW";
        for i in 0..16 {
            for lr in ['L', 'R'] {
                let bel = format!("BUFH_{lr}{i}");
                ctx.collect_enum_default(tile, &bel, "I", &["BUFG", "CMT"], "NONE");
            }
        }
    }
    {
        let tile = "CLKC";
        for i in 0..16 {
            let bel = format!("BUFGMUX{i}");
            ctx.state
                .get_diff(tile, &bel, "PRESENT", "1")
                .assert_empty();
            ctx.collect_enum(tile, &bel, "CLK_SEL_TYPE", &["SYNC", "ASYNC"]);
            ctx.collect_enum_bool(tile, &bel, "DISABLE_ATTR", "LOW", "HIGH");
            ctx.collect_inv(tile, &bel, "S");
        }
        let bel = "CLKC";
        for i in 0..16 {
            ctx.collect_enum_default(
                tile,
                bel,
                &format!("IMUX{i}"),
                &[
                    &format!("CKPIN_H{i}"),
                    &format!("CKPIN_V{i}"),
                    &format!("CMT_D{i}"),
                    &format!("CMT_U{i}"),
                ],
                "NONE",
            );
        }
        for out in [
            "OUTL_CLKOUT0",
            "OUTL_CLKOUT1",
            "OUTR_CLKOUT0",
            "OUTR_CLKOUT1",
        ] {
            ctx.collect_enum(
                tile,
                bel,
                out,
                &[
                    "PLL0U_CLKOUT0",
                    "PLL0U_CLKOUT1",
                    "PLL1U_CLKOUT0",
                    "PLL1U_CLKOUT1",
                    "PLL0D_CLKOUT0",
                    "PLL0D_CLKOUT1",
                    "PLL1D_CLKOUT0",
                    "PLL1D_CLKOUT1",
                ],
            );
        }
        for out in ["OUTD_CLKOUT0", "OUTD_CLKOUT1"] {
            ctx.collect_enum(
                tile,
                bel,
                out,
                &[
                    "PLL0U_CLKOUT0",
                    "PLL0U_CLKOUT1",
                    "PLL1U_CLKOUT0",
                    "PLL1U_CLKOUT1",
                ],
            );
        }
        for out in ["OUTU_CLKOUT0", "OUTU_CLKOUT1"] {
            ctx.collect_enum(
                tile,
                bel,
                out,
                &[
                    "PLL0D_CLKOUT0",
                    "PLL0D_CLKOUT1",
                    "PLL1D_CLKOUT0",
                    "PLL1D_CLKOUT1",
                ],
            );
        }
        for out in [
            "OUTL_LOCKED0",
            "OUTL_LOCKED1",
            "OUTR_LOCKED0",
            "OUTR_LOCKED1",
        ] {
            ctx.collect_enum(
                tile,
                bel,
                out,
                &[
                    "PLL0D_LOCKED",
                    "PLL1D_LOCKED",
                    "PLL0U_LOCKED",
                    "PLL1U_LOCKED",
                ],
            );
        }
        ctx.collect_enum(tile, bel, "OUTD_LOCKED", &["PLL0U_LOCKED", "PLL1U_LOCKED"]);
        ctx.collect_enum(tile, bel, "OUTU_LOCKED", &["PLL0D_LOCKED", "PLL1D_LOCKED"]);
    }
    {
        let tile = "DCM_BUFPLL";
        let bel = "DCM_BUFPLL";
        for attr in [
            "PLL0_CLKOUT0",
            "PLL0_CLKOUT1",
            "PLL1_CLKOUT0",
            "PLL1_CLKOUT1",
            "CLKC_CLKOUT0",
            "CLKC_CLKOUT1",
        ] {
            ctx.collect_bit(tile, bel, attr, "1");
        }
        for attr in ["PLL0_LOCKED", "PLL1_LOCKED", "CLKC_LOCKED"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
    }
    if ctx.has_tile("PLL_BUFPLL_OUT0") {
        let tile = "PLL_BUFPLL_OUT0";
        let bel = "PLL_BUFPLL";
        for attr in [
            "PLL0_CLKOUT0_D",
            "PLL0_CLKOUT1_D",
            "PLL0_CLKOUT0_U",
            "PLL0_CLKOUT1_U",
            "PLL1_CLKOUT0",
            "PLL1_CLKOUT1",
            "CLKC_CLKOUT0",
            "CLKC_CLKOUT1",
        ] {
            ctx.collect_bit(tile, bel, attr, "1");
        }
        for attr in ["PLL0_LOCKED_D", "PLL0_LOCKED_U"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
    }
    {
        let tile = "PLL_BUFPLL_OUT1";
        let bel = "PLL_BUFPLL";
        for attr in [
            "PLL0_CLKOUT0",
            "PLL0_CLKOUT1",
            "PLL1_CLKOUT0_D",
            "PLL1_CLKOUT1_D",
            "PLL1_CLKOUT0_U",
            "PLL1_CLKOUT1_U",
            "CLKC_CLKOUT0",
            "CLKC_CLKOUT1",
        ] {
            ctx.collect_bit(tile, bel, attr, "1");
        }
        for attr in ["PLL1_LOCKED_D", "PLL1_LOCKED_U"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
    }
    for tile in ["PLL_BUFPLL_B", "PLL_BUFPLL_T"] {
        if ctx.has_tile(tile) {
            let bel = "PLL_BUFPLL";
            for attr in [
                "PLL0_CLKOUT0",
                "PLL0_CLKOUT1",
                "PLL1_CLKOUT0",
                "PLL1_CLKOUT1",
                "CLKC_CLKOUT0",
                "CLKC_CLKOUT1",
            ] {
                ctx.collect_bit(tile, bel, attr, "1");
            }
        }
    }
    for (tile, is_lr) in [
        ("REG_B", false),
        ("REG_T", false),
        ("REG_L", true),
        ("REG_R", true),
    ] {
        for i in 0..8 {
            let bel = format!("BUFIO2_{i}");
            let bel_fb = format!("BUFIO2FB_{i}");
            let bel = &bel;
            let bel_fb = &bel_fb;
            ctx.state
                .get_diff(tile, bel, "PRESENT", "BUFIO2")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, bel, "CMT_ENABLE", "1");
            assert_eq!(diff, ctx.state.get_diff(tile, bel_fb, "CMT_ENABLE", "1"));
            ctx.tiledb.insert(tile, bel, "CMT_ENABLE", xlat_bit(diff));
            ctx.collect_bit(tile, bel, "IOCLK_ENABLE", "1");
            ctx.collect_enum(tile, bel, "CKPIN", &["VCC", "DIVCLK", "CLKPIN"]);
            ctx.collect_enum_bool(tile, bel, "R_EDGE", "FALSE", "TRUE");
            ctx.collect_enum_bool(tile, bel, "DIVIDE_BYPASS", "FALSE", "TRUE");
            let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "BUFIO2_2CLK");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "R_EDGE"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DIVIDE_BYPASS"), false, true);
            ctx.tiledb.insert(tile, bel, "ENABLE_2CLK", xlat_bit(diff));

            let mut pos_edge = vec![];
            let mut pos_bits = HashSet::new();
            let mut neg_edge = vec![];
            let mut neg_bits = HashSet::new();
            for i in 1..=8 {
                let val = format!("{i}");
                let diff = ctx.state.get_diff(tile, bel, "POS_EDGE", &val);
                pos_bits.extend(diff.bits.keys().copied());
                pos_edge.push((format!("POS_EDGE_{i}"), diff));
                let diff = ctx.state.get_diff(tile, bel, "NEG_EDGE", &val);
                neg_bits.extend(diff.bits.keys().copied());
                neg_edge.push((format!("NEG_EDGE_{i}"), diff));
            }
            let mut divide = vec![];
            for i in 1..=8 {
                let val = format!("{i}");
                let mut diff = ctx.state.get_diff(tile, bel, "DIVIDE", &val);
                if matches!(i, 2 | 4 | 6 | 8) {
                    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "R_EDGE"), true, false);
                }
                let diff2 = ctx.state.get_diff(tile, bel, "DIVIDE.2CLK", &val);
                assert_eq!(diff, diff2);
                let pos = diff.split_bits(&pos_bits);
                let neg = diff.split_bits(&neg_bits);
                pos_edge.push((format!("DIVIDE_{i}"), pos));
                neg_edge.push((format!("DIVIDE_{i}"), neg));
                divide.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, "POS_EDGE", xlat_enum(pos_edge));
            ctx.tiledb
                .insert(tile, bel, "NEG_EDGE", xlat_enum(neg_edge));
            ctx.tiledb.insert(tile, bel, "DIVIDE", xlat_enum(divide));

            let enable = ctx.state.peek_diff(tile, bel, "I", "CLKPIN0").clone();
            let mut diffs = vec![];
            for val in [
                "CLKPIN0", "CLKPIN1", "CLKPIN4", "CLKPIN5", "DFB", "DQS0", "DQS2", "GTPCLK",
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, "I", val);
                diff = diff.combine(&!&enable);
                diffs.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, "I", xlat_enum_ocd(diffs, OcdMode::BitOrder));
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(enable));
            ctx.collect_enum_ocd(
                tile,
                bel,
                "IB",
                &[
                    "CLKPIN0", "CLKPIN1", "CLKPIN4", "CLKPIN5", "DFB", "DQS0", "DQS2",
                ],
                OcdMode::BitOrder,
            );

            ctx.state
                .get_diff(tile, bel_fb, "PRESENT", "BUFIO2FB")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel_fb, "DIVIDE_BYPASS", "TRUE")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, bel_fb, "DIVIDE_BYPASS", "FALSE");
            ctx.tiledb
                .insert(tile, bel, "FB_DIVIDE_BYPASS", xlat_bit_wide(!diff));

            let enable = ctx.state.peek_diff(tile, bel_fb, "I", "CLKPIN").clone();
            let mut diffs = vec![];
            for val in ["CLKPIN", "DFB", "CFB", "CFB_INVERT", "GTPFB"] {
                let mut diff = ctx.state.get_diff(tile, bel_fb, "I", val);
                diff = diff.combine(&!&enable);
                diffs.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, "FB_I", xlat_enum_ocd(diffs, OcdMode::BitOrder));
            ctx.tiledb.insert(tile, bel, "FB_ENABLE", xlat_bit(enable));

            let mut present = ctx.state.get_diff(tile, bel_fb, "PRESENT", "BUFIO2FB_2CLK");
            present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "FB_DIVIDE_BYPASS"), 0, 0xf);
            present.apply_enum_diff(ctx.tiledb.item(tile, bel, "FB_I"), "CFB", "CLKPIN");
            present.assert_empty();
        }
        for val in ["1", "2", "3", "4", "5", "6", "7", "8"] {
            let diff = ctx.state.get_diff(tile, "BUFPLL_MCB", "DIVIDE", val);
            let diff0 = ctx.state.peek_diff(tile, "BUFPLL0", "DIVIDE", val);
            let diff1 = ctx.state.peek_diff(tile, "BUFPLL1", "DIVIDE", val);
            assert_eq!(diff, diff0.combine(diff1));
        }
        for i in 0..2 {
            let bel = format!("BUFPLL{i}");
            let bel = &bel;
            ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
            ctx.collect_enum(tile, bel, "DATA_RATE", &["SDR", "DDR"]);
            ctx.collect_enum_ocd(
                tile,
                bel,
                "DIVIDE",
                &["1", "2", "3", "4", "5", "6", "7", "8"],
                OcdMode::BitOrder,
            );
            let enable = ctx.extract_bit(tile, bel, "ENABLE", "1");
            let mut diff = ctx.state.get_diff(tile, bel, "ENABLE_NONE_SYNC", "1");
            diff.apply_bit_diff(&enable, true, false);
            ctx.tiledb
                .insert(tile, bel, "ENABLE_NONE_SYNC", xlat_bit_wide(diff));
            let mut diff = ctx.state.get_diff(tile, bel, "ENABLE_BOTH_SYNC", "1");
            diff.apply_bit_diff(&enable, true, false);
            ctx.tiledb
                .insert(tile, bel, "ENABLE_BOTH_SYNC", xlat_bit_wide(diff));
            ctx.tiledb.insert(tile, "BUFPLL_COMMON", "ENABLE", enable);
            ctx.collect_enum_bool(tile, bel, "ENABLE_SYNC", "FALSE", "TRUE");

            if !is_lr {
                ctx.collect_enum(tile, bel, "LOCKED", &["LOCKED0", "LOCKED1", "LOCKED2"]);
                ctx.collect_enum(
                    tile,
                    bel,
                    "PLLIN",
                    &["PLLIN0", "PLLIN1", "PLLIN2", "PLLIN3", "PLLIN4", "PLLIN5"],
                );
            }
        }
        {
            let bel = "BUFPLL_MCB";
            let item = ctx.extract_bit(tile, bel, "ENABLE.0", "1");
            ctx.tiledb.insert(tile, "BUFPLL_COMMON", "ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "ENABLE.1", "1");
            ctx.tiledb.insert(tile, "BUFPLL_COMMON", "ENABLE", item);
            if is_lr {
                let item = ctx.extract_enum(tile, bel, "PLLIN", &["GCLK", "CMT"]);
                ctx.tiledb.insert(tile, "BUFPLL_COMMON", "PLLIN", item);
            }
            let mut diff0 = ctx.state.get_diff(tile, bel, "LOCK_SRC", "LOCK_TO_0");
            diff0.apply_bit_diff(ctx.tiledb.item(tile, "BUFPLL1", "ENABLE_SYNC"), false, true);
            let mut diff1 = ctx.state.get_diff(tile, bel, "LOCK_SRC", "LOCK_TO_1");
            diff1.apply_bit_diff(ctx.tiledb.item(tile, "BUFPLL0", "ENABLE_SYNC"), false, true);
            ctx.tiledb.insert(
                tile,
                bel,
                "LOCK_SRC",
                xlat_enum(vec![("LOCK_TO_0", diff0), ("LOCK_TO_1", diff1)]),
            );
            let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, "BUFPLL0", "ENABLE_BOTH_SYNC"), 7, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, "BUFPLL1", "ENABLE_BOTH_SYNC"), 7, 0);
            diff.assert_empty();
        }
        if !is_lr {
            let bel = "MISC";
            let has_gt = match tile {
                "REG_B" => matches!(edev.grid.gts, Gts::Quad(_, _)),
                "REG_T" => edev.grid.gts != Gts::None,
                _ => unreachable!(),
            };
            if has_gt && !ctx.device.name.starts_with("xa") {
                ctx.collect_bit(tile, bel, "MISR_ENABLE", "1");
                let mut diff = ctx.state.get_diff(tile, bel, "MISR_ENABLE_RESET", "1");
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "MISR_ENABLE"), true, false);
                ctx.tiledb.insert(tile, bel, "MISR_RESET", xlat_bit(diff));
            } else {
                // they're sometimes working, sometimes not, in nonsensical ways; just kill them
                ctx.state.get_diff(tile, bel, "MISR_ENABLE", "1");
                ctx.state.get_diff(tile, bel, "MISR_ENABLE_RESET", "1");
            }
        }
    }
    {
        let tile = "PCILOGICSE";
        let bel = "PCILOGICSE";
        let default = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        let mut diffs = vec![];
        for i in 2..=31 {
            let val = format!("TAP{i}");
            let diff = ctx.state.get_diff(tile, bel, "PRESENT", &val);
            if diff == default {
                ctx.insert_device_data("PCI_CE_DELAY", val.clone());
            }
            diffs.push((val, diff));
        }
        diffs.reverse();
        let mut bits: HashSet<_> = diffs[0].1.bits.keys().copied().collect();
        for (_, diff) in &diffs {
            bits.retain(|b| diff.bits.contains_key(b));
        }
        assert_eq!(bits.len(), 1);
        for (_, diff) in &mut diffs {
            let enable = diff.split_bits(&bits);
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(enable));
        }
        ctx.tiledb
            .insert(tile, bel, "PCI_CE_DELAY", xlat_enum(diffs));
    }
    for (tile, frame, bit, target) in [
        // used in CLEXL (incl. spine) and DSP columns; also used on PCIE sides and left GTP side
        ("HCLK_CLEXL", 16, 0, 21),
        ("HCLK_CLEXL", 17, 0, 22),
        ("HCLK_CLEXL", 18, 0, 23),
        ("HCLK_CLEXL", 19, 0, 24),
        ("HCLK_CLEXL", 16, 1, 26),
        ("HCLK_CLEXL", 17, 1, 27),
        ("HCLK_CLEXL", 18, 1, 28),
        ("HCLK_CLEXL", 19, 1, 29),
        // used in CLEXM columns
        ("HCLK_CLEXM", 16, 0, 21),
        ("HCLK_CLEXM", 17, 0, 22),
        ("HCLK_CLEXM", 18, 0, 24),
        ("HCLK_CLEXM", 19, 0, 25),
        ("HCLK_CLEXM", 16, 1, 27),
        ("HCLK_CLEXM", 17, 1, 28),
        ("HCLK_CLEXM", 18, 1, 29),
        ("HCLK_CLEXM", 19, 1, 30),
        // used in IOI columns
        ("HCLK_IOI", 16, 0, 25),
        ("HCLK_IOI", 18, 0, 23),
        ("HCLK_IOI", 19, 0, 24),
        ("HCLK_IOI", 16, 1, 21),
        ("HCLK_IOI", 17, 1, 27),
        ("HCLK_IOI", 18, 1, 28),
        ("HCLK_IOI", 19, 1, 29),
        // used on right GTP side
        ("HCLK_GTP", 16, 0, 25),
        ("HCLK_GTP", 17, 0, 22),
        ("HCLK_GTP", 18, 0, 23),
        ("HCLK_GTP", 19, 0, 24),
        // BRAM columns do not have masking
    ] {
        ctx.tiledb.insert(
            tile,
            "GLUTMASK",
            format!("FRAME{target}"),
            TileItem::from_bit(TileBit::new(0, frame, bit), false),
        )
    }
}
