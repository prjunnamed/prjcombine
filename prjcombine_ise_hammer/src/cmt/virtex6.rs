use bitvec::prelude::*;
use prjcombine_collector::{extract_bitvec_val_part, xlat_bit, xlat_enum, Diff, OcdMode};
use prjcombine_hammer::Session;
use prjcombine_int::db::{BelId, Dir};
use prjcombine_types::tiledb::{TileBit, TileItem};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    if devdata_only {
        for i in 0..2 {
            let ctx = FuzzCtx::new(session, backend, "CMT", format!("MMCM{i}"), TileBits::Cmt);
            fuzz_enum!(ctx, "COMPENSATION", ["ZHOLD"], [
                (mutex "MODE", "COMP"),
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mode "MMCM_ADV"),
                (attr "HROW_DLY_SET", "000")
            ]);
        }
        return;
    }
    let bel_mmcm0 = BelId::from_idx(24);
    let bel_cmt = BelId::from_idx(27);
    for i in 0..12 {
        for lr in ['L', 'R'] {
            let bel_other =
                BelId::from_idx(if i < 6 { i + 6 } else { i - 6 } + if lr == 'L' { 0 } else { 12 });
            let ctx = FuzzCtx::new(
                session,
                backend,
                "CMT",
                format!("BUFHCE_{lr}{i}"),
                TileBits::Cmt,
            );
            fuzz_one!(ctx, "ENABLE", "1", [], [(mode "BUFHCE")]);
            fuzz_inv!(ctx, "CE", [(mode "BUFHCE")]);
            fuzz_enum!(ctx, "INIT_OUT", ["0", "1"], [(mode "BUFHCE")]);

            for pin in ["BUFH_TEST_L", "BUFH_TEST_R"] {
                fuzz_one!(ctx, "MUX.I", pin, [
                    (mutex "MUX.I", pin)
                ], [
                    (pip (bel_pin bel_cmt, pin), (pin "I"))
                ]);
            }
            for j in 0..4 {
                for ilr in ['L', 'R'] {
                    fuzz_one!(ctx, "MUX.I", format!("CCIO{j}_{ilr}"), [
                        (tile_mutex "CCIO", "USE"),
                        (mutex "MUX.I", format!("CCIO{j}_{ilr}")),
                        (bel_mutex bel_other, "MUX.I", format!("CCIO{j}_{ilr}")),
                        (pip (bel_pin bel_cmt, format!("CCIO{j}_{ilr}")), (bel_pin bel_other, "I"))
                    ], [
                        (pip (bel_pin bel_cmt, format!("CCIO{j}_{ilr}")), (pin "I"))
                    ]);
                    if i == 0 && lr == 'L' {
                        fuzz_one!(ctx, "MUX.I", format!("CCIO{j}_{ilr}.EXCL"), [
                            (tile_mutex "CCIO", "TEST"),
                            (mutex "MUX.I", format!("CCIO{j}_{ilr}"))
                        ], [
                            (pip (bel_pin bel_cmt, format!("CCIO{j}_{ilr}")), (pin "I"))
                        ]);
                    }
                }
            }
            for j in 0..32 {
                fuzz_one!(ctx, "MUX.I", format!("GCLK{j}"), [
                    (global_mutex "GCLK", "USE"),
                    (mutex "MUX.I", format!("GCLK{j}")),
                    (bel_mutex bel_other, "MUX.I", format!("GCLK{j}")),
                    (pip (bel_pin bel_cmt, format!("GCLK{j}")), (bel_pin bel_other, "I"))
                ], [
                    (pip (bel_pin bel_cmt, format!("GCLK{j}")), (pin "I"))
                ]);
            }
            for j in 0..2 {
                for (k, pin) in [
                    "CLKOUT0",
                    "CLKOUT0B",
                    "CLKOUT1",
                    "CLKOUT1B",
                    "CLKOUT2",
                    "CLKOUT2B",
                    "CLKOUT3",
                    "CLKOUT3B",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKOUT6",
                    "CLKFBOUT",
                    "CLKFBOUTB",
                    "TMUXOUT",
                ]
                .into_iter()
                .enumerate()
                {
                    fuzz_one!(ctx, "MUX.I", format!("MMCM{j}_{pin}"), [
                        (mutex "MUX.I", format!("MMCM{j}_{pin}"))
                    ], [
                        (pip (bel_pin bel_cmt, format!("MMCM{j}_OUT{k}")), (pin "I"))
                    ]);
                }
            }
            for pin in ["CKINT0", "CKINT1"] {
                fuzz_one!(ctx, "MUX.I", pin, [
                    (tile_mutex "BUFHCE_CKINT", "USE"),
                    (mutex "MUX.I", pin),
                    (bel_mutex bel_other, "MUX.I", pin),
                    (pip (bel_pin bel_cmt, format!("BUFHCE_{lr}_{pin}")), (bel_pin bel_other, "I"))
                ], [
                    (pip (bel_pin bel_cmt, format!("BUFHCE_{lr}_{pin}")), (pin "I"))
                ]);
                if i == 0 {
                    fuzz_one!(ctx, "MUX.I", format!("{pin}.EXCL"), [
                        (tile_mutex "BUFHCE_CKINT", "TEST"),
                        (mutex "MUX.I", pin)
                    ], [
                        (pip (bel_pin bel_cmt, format!("BUFHCE_{lr}_{pin}")), (pin "I"))
                    ]);
                }
            }
        }
    }
    for i in 0..2 {
        let oi = 1 - i;
        let bel_other = BelId::from_idx(24 + oi);
        let ctx = FuzzCtx::new(session, backend, "CMT", format!("MMCM{i}"), TileBits::Cmt);

        fuzz_one!(ctx, "ENABLE", "1", [
            (global_xy "MMCMADV_*_USE_CALC", "NO")
        ], [
            (mode "MMCM_ADV")
        ]);

        for pin in ["RST", "PWRDWN", "PSINCDEC", "PSEN", "CLKINSEL"] {
            fuzz_inv!(ctx, pin, [
                (mutex "MODE", "PIN"),
                (mode "MMCM_ADV")
            ]);
        }

        for attr in [
            "CASC_LOCK_EN",
            "CLKBURST_ENABLE",
            "CLKBURST_REPEAT",
            "CLKFBOUT_EN",
            "CLKOUT0_EN",
            "CLKOUT1_EN",
            "CLKOUT2_EN",
            "CLKOUT3_EN",
            "CLKOUT4_EN",
            "CLKOUT5_EN",
            "CLKOUT6_EN",
            "DIRECT_PATH_CNTRL",
            "CLOCK_HOLD",
            "EN_VCO_DIV1",
            "EN_VCO_DIV6",
            "HVLF_STEP",
            "HVLF_CNT_TEST_EN",
            "IN_DLY_EN",
            "STARTUP_WAIT",
            "VLF_HIGH_DIS_B",
            "VLF_HIGH_PWDN_B",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mutex "MODE", "TEST"),
                (mode "MMCM_ADV")
            ]);
        }
        fuzz_enum!(ctx, "CLKOUT4_CASCADE", ["FALSE", "TRUE"], [
            (global_xy "MMCMADV_*_USE_CALC", "NO"),
            (mutex "MODE", "TEST"),
            (mode "MMCM_ADV"),
            (attr "CLKOUT6_EN", "TRUE"),
            (attr "CLKOUT4_USE_FINE_PS", ""),
            (attr "CLKOUT4_MX", "")
        ]);
        fuzz_enum!(ctx, "GTS_WAIT", ["FALSE", "TRUE"], [
            (global_xy "MMCMADV_*_USE_CALC", "NO"),
            (mutex "MODE", "TEST"),
            (mode "MMCM_ADV"),
            (attr "STARTUP_WAIT", "FALSE")
        ]);
        for attr in [
            "CLKOUT0_USE_FINE_PS",
            "CLKOUT1_USE_FINE_PS",
            "CLKOUT2_USE_FINE_PS",
            "CLKOUT3_USE_FINE_PS",
            "CLKOUT4_USE_FINE_PS",
            "CLKOUT5_USE_FINE_PS",
            "CLKOUT6_USE_FINE_PS",
            "CLKFBOUT_USE_FINE_PS",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mutex "MODE", "TEST"),
                (mode "MMCM_ADV"),
                (attr "CLKFBOUT_MX", ""),
                (attr "CLKOUT0_MX", ""),
                (attr "CLKOUT1_MX", ""),
                (attr "CLKOUT2_MX", ""),
                (attr "CLKOUT3_MX", ""),
                (attr "CLKOUT4_MX", ""),
                (attr "CLKOUT5_MX", ""),
                (attr "CLKOUT6_MX", ""),
                (attr "INTERP_EN", "00000000")
            ]);
        }
        for attr in ["CLKOUT0_FRAC_EN", "CLKFBOUT_FRAC_EN"] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mutex "MODE", "TEST"),
                (mode "MMCM_ADV"),
                (attr "CLKOUT5_EN", "TRUE"),
                (attr "CLKOUT6_EN", "TRUE"),
                (attr "INTERP_EN", "00000000")
            ]);
        }

        for (attr, width) in [
            ("ANALOG_MISC", 4),
            ("AVDD_COMP_SET", 2),
            ("AVDD_VBG_PD", 3),
            ("AVDD_VBG_SEL", 4),
            ("CLKFBIN_HT", 6),
            ("CLKFBIN_LT", 6),
            ("CLKFBOUT_DT", 6),
            ("CLKFBOUT_FRAC", 3),
            ("CLKFBOUT_HT", 6),
            ("CLKFBOUT_LT", 6),
            ("CLKFBOUT_MX", 2),
            ("CLKFBOUT_FRAC", 3),
            ("CLKOUT0_DT", 6),
            ("CLKOUT0_HT", 6),
            ("CLKOUT0_LT", 6),
            ("CLKOUT0_MX", 2),
            ("CLKOUT0_FRAC", 3),
            ("CLKOUT1_DT", 6),
            ("CLKOUT1_HT", 6),
            ("CLKOUT1_LT", 6),
            ("CLKOUT1_MX", 2),
            ("CLKOUT2_DT", 6),
            ("CLKOUT2_HT", 6),
            ("CLKOUT2_LT", 6),
            ("CLKOUT2_MX", 2),
            ("CLKOUT3_DT", 6),
            ("CLKOUT3_HT", 6),
            ("CLKOUT3_LT", 6),
            ("CLKOUT3_MX", 2),
            ("CLKOUT4_DT", 6),
            ("CLKOUT4_HT", 6),
            ("CLKOUT4_LT", 6),
            ("CLKOUT4_MX", 2),
            ("CLKOUT5_DT", 6),
            ("CLKOUT5_HT", 6),
            ("CLKOUT5_LT", 6),
            ("CLKOUT5_MX", 2),
            ("CLKOUT6_DT", 6),
            ("CLKOUT6_HT", 6),
            ("CLKOUT6_LT", 6),
            ("CLKOUT6_MX", 2),
            ("CONTROL_0", 16),
            ("CONTROL_1", 16),
            ("CONTROL_2", 16),
            ("CONTROL_3", 16),
            ("CONTROL_4", 16),
            ("CONTROL_5", 16),
            ("CP_BIAS_TRIP_SET", 1),
            ("CP_RES", 2),
            ("DIVCLK_HT", 6),
            ("DIVCLK_LT", 6),
            ("DVDD_COMP_SET", 2),
            ("DVDD_VBG_PD", 3),
            ("DVDD_VBG_SEL", 4),
            ("INTERP_EN", 8),
            ("IN_DLY_MX_CVDD", 6),
            ("IN_DLY_MX_DVDD", 6),
            ("LF_NEN", 2),
            ("LF_PEN", 2),
            ("MAN_LF", 3),
            ("PFD", 7),
            ("TMUX_MUX_SEL", 2),
        ] {
            fuzz_multi_attr_bin!(ctx, attr, width, [
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mutex "MODE", "TEST"),
                (mode "MMCM_ADV"),
                (attr "CLKOUT0_DIVIDE_F", "1.5"),
                (attr "CLKFBOUT_MULT_F", "1.5")
            ]);
        }
        for (attr, width) in [
            ("CLKFBOUT_PM", 3),
            ("CLKOUT0_PM", 3),
            ("CLKOUT1_PM", 3),
            ("CLKOUT2_PM", 3),
            ("CLKOUT3_PM", 3),
            ("CLKOUT4_PM", 3),
            ("CLKOUT5_PM", 3),
            ("CLKOUT6_PM", 3),
            ("FINE_PS_FRAC", 6),
        ] {
            fuzz_multi_attr_bin!(ctx, attr, width, [
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mutex "MODE", "TEST"),
                (mode "MMCM_ADV"),
                (attr "INTERP_EN", "00000000")
            ]);
        }
        for (attr, width) in [
            ("CLKBURST_CNT", 4),
            ("CP", 4),
            ("HROW_DLY_SET", 3),
            ("HVLF_CNT_TEST", 6),
            ("LFHF", 2),
            ("LOCK_CNT", 10),
            ("LOCK_FB_DLY", 5),
            ("LOCK_REF_DLY", 5),
            ("LOCK_SAT_HIGH", 10),
            ("RES", 4),
            ("UNLOCK_CNT", 10),
        ] {
            fuzz_multi_attr_dec!(ctx, attr, width, [
                (global_xy "MMCMADV_*_USE_CALC", "NO"),
                (mutex "MODE", "TEST"),
                (mode "MMCM_ADV")
            ]);
        }

        for mult in 1..=64 {
            for bandwidth in ["LOW", "HIGH"] {
                fuzz_one!(ctx, "TABLES", format!("{mult}.{bandwidth}"), [
                    (mutex "MODE", "CALC"),
                    (global_xy "MMCMADV_*_USE_CALC", "NO"),
                    (mode "MMCM_ADV")
                ], [
                    (attr "CLKFBOUT_MULT_F", format!("{mult}")),
                    (attr "BANDWIDTH", bandwidth)
                ]);
            }
        }
        fuzz_enum!(ctx, "COMPENSATION", ["ZHOLD", "EXTERNAL", "INTERNAL", "BUF_IN", "CASCADE"], [
            (mutex "MODE", "COMP"),
            (global_xy "MMCMADV_*_USE_CALC", "NO"),
            (mode "MMCM_ADV"),
            (attr "HROW_DLY_SET", "000")
        ]);

        for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
            for j in 0..8 {
                fuzz_one!(ctx, format!("MUX.{pin}_IO"), format!("GIO{j}"), [
                    (global_mutex "GIO", "USE"),
                    (mutex format!("MUX.{pin}_IO"), format!("GIO{j}")),
                    (bel_mutex bel_other, format!("MUX.{pin}_IO"), format!("GIO{j}")),
                    (pip (bel_pin bel_cmt, format!("GIO{j}")), (bel_pin bel_cmt, format!("MMCM{oi}_{pin}_IO")))
                ], [
                    (pip (bel_pin bel_cmt, format!("GIO{j}")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_IO")))
                ]);
            }
            for j in 0..4 {
                for lr in ['L', 'R'] {
                    fuzz_one!(ctx, format!("MUX.{pin}_IO"), format!("CCIO{j}_{lr}"), [
                        (tile_mutex "CCIO", "USE"),
                        (mutex format!("MUX.{pin}_IO"), format!("CCIO{j}_{lr}")),
                        (bel_mutex bel_other, format!("MUX.{pin}_IO"), format!("CCIO{j}_{lr}")),
                        (pip (bel_pin bel_cmt, format!("CCIO{j}_{lr}")), (bel_pin bel_cmt, format!("MMCM{oi}_{pin}_IO")))
                    ], [
                        (pip (bel_pin bel_cmt, format!("CCIO{j}_{lr}")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_IO")))
                    ]);
                }
            }
        }
        for pin in ["CLKIN1", "CLKIN2"] {
            for j in 0..10 {
                for lr in ['L', 'R'] {
                    fuzz_one!(ctx, format!("MUX.{pin}_MGT"), format!("MGT{j}_{lr}"), [
                        (row_mutex "MGT", "USE"),
                        (mutex format!("MUX.{pin}_MGT"), format!("MGT{j}_{lr}")),
                        (bel_mutex bel_other, format!("MUX.{pin}_MGT"), format!("MGT{j}_{lr}")),
                        (pip (bel_pin bel_cmt, format!("MGT{j}_{lr}")), (bel_pin bel_cmt, format!("MMCM{oi}_{pin}_MGT")))
                    ], [
                        (pip (bel_pin bel_cmt, format!("MGT{j}_{lr}")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_MGT")))
                    ]);
                    if i == 0 && pin == "CLKIN1" {
                        fuzz_one!(ctx, format!("MUX.{pin}_MGT"), format!("MGT{j}_{lr}.EXCL"), [
                            (row_mutex "MGT", "TEST"),
                            (mutex format!("MUX.{pin}_MGT"), format!("MGT{j}_{lr}"))
                        ], [
                            (pip (bel_pin bel_cmt, format!("MGT{j}_{lr}")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_MGT")))
                        ]);
                    }
                }
            }
        }
        for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
            for lr in ['L', 'R'] {
                fuzz_one!(ctx, format!("MUX.{pin}_HCLK"), format!("{pin}_HCLK_{lr}"), [
                    (mutex format!("MUX.{pin}_HCLK"), format!("{pin}_HCLK_{lr}"))
                ], [
                    (pip (bel_pin bel_cmt, format!("MMCM{i}_{pin}_HCLK_{lr}")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_HCLK")))
                ]);
                for j in 0..12 {
                    fuzz_one!(ctx, format!("MUX.{pin}_HCLK_{lr}"), format!("HCLK{j}_{lr}"), [
                        (global_mutex "HCLK", "USE"),
                        (mutex format!("MUX.{pin}_HCLK_{lr}"), format!("HCLK{j}_{lr}")),
                        (bel_mutex bel_other, format!("MUX.{pin}_HCLK_{lr}"), format!("HCLK{j}_{lr}")),
                        (pip (bel_pin bel_cmt, format!("HCLK{j}_{lr}_I")), (bel_pin bel_cmt, format!("MMCM{oi}_{pin}_HCLK_{lr}")))
                    ], [
                        (pip (bel_pin bel_cmt, format!("HCLK{j}_{lr}_I")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_HCLK_{lr}")))
                    ]);
                }
                for j in 0..6 {
                    fuzz_one!(ctx, format!("MUX.{pin}_HCLK_{lr}"), format!("RCLK{j}_{lr}"), [
                        (global_mutex "RCLK", "USE"),
                        (mutex format!("MUX.{pin}_HCLK_{lr}"), format!("RCLK{j}_{lr}")),
                        (bel_mutex bel_other, format!("MUX.{pin}_HCLK_{lr}"), format!("RCLK{j}_{lr}")),
                        (pip (bel_pin bel_cmt, format!("RCLK{j}_{lr}_I")), (bel_pin bel_cmt, format!("MMCM{oi}_{pin}_HCLK_{lr}")))
                    ], [
                        (pip (bel_pin bel_cmt, format!("RCLK{j}_{lr}_I")), (bel_pin bel_cmt, format!("MMCM{i}_{pin}_HCLK_{lr}")))
                    ]);
                }
            }
        }

        for pin in [
            "CLKIN1_HCLK",
            "CLKIN1_IO",
            "CLKIN1_MGT",
            "CASC_IN",
            "CLKIN1_CKINT",
        ] {
            fuzz_one!(ctx, "MUX.CLKIN1", pin, [
                (mutex "MUX.CLKIN1", pin)
            ], [
                (pip (pin pin), (pin "CLKIN1"))
            ]);
        }
        for pin in ["CLKIN2_HCLK", "CLKIN2_IO", "CLKIN2_MGT", "CLKIN2_CKINT"] {
            fuzz_one!(ctx, "MUX.CLKIN2", pin, [
                (mutex "MUX.CLKIN2", pin)
            ], [
                (pip (pin pin), (pin "CLKIN2"))
            ]);
        }
        for pin in [
            "CLKFBIN_HCLK",
            "CLKFBIN_IO",
            "CLKFBIN_CKINT",
            "CLKFBOUT",
            "CASC_OUT",
        ] {
            let ipin = if pin == "CLKFBOUT" { "CLKFB" } else { pin };
            fuzz_one!(ctx, "MUX.CLKFBIN", pin, [
                (mutex "MUX.CLKFBIN", pin)
            ], [
                (pip (pin ipin), (pin "CLKFBIN"))
            ]);
        }
        for pin in [
            "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKOUT6", "CLKFBOUT",
        ] {
            fuzz_one!(ctx, "MUX.CASC_OUT", pin, [
                (mutex "MUX.CASC_OUT", pin)
            ], [
                (pip (pin pin), (pin "CASC_OUT"))
            ]);
        }
        for j in 0..4 {
            for which in ["IL", "IR", "OL", "OR"] {
                let jj = if which.starts_with('O') { j ^ 1 } else { j };
                for k in 0..4 {
                    fuzz_one!(ctx, format!("MUX.PERF{j}.{which}"), format!("CLKOUT{k}"), [
                        (tile_mutex format!("MUX.PERF{j}"), format!("MMCM{i}.{which}.CLKOUT{k}"))
                    ], [
                        (pip (pin format!("CLKOUT{k}")), (pin format!("PERF{j}"))),
                        (pip (pin format!("PERF{j}")), (pin format!("PERF{jj}_{which}")))
                    ]);
                }
            }
        }
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT", "PPR_FRAME", TileBits::Null);
        fuzz_one!(ctx, "ENABLE", "1", [], [(mode "PPR_FRAME")]);
    }
    {
        let ctx = FuzzCtx::new(session, backend, "CMT", "CMT", TileBits::Cmt);

        for i in 0..32 {
            let bel_bufhce = BelId::from_idx(i % 16);
            fuzz_one!(ctx, format!("BUF.GCLK{i}_TEST"), "1", [
                (global_mutex "GCLK", "USE"),
                (mutex format!("GCLK{i}_TEST"), "BUF"),
                (bel_mutex bel_bufhce, "MUX.I", format!("GCLK{i}")),
                (pip (pin format!("GCLK{i}")), (bel_pin bel_bufhce, "I"))
            ], [
                (pip (pin format!("GCLK{i}")), (pin format!("GCLK{i}_NOINV")))
            ]);
            fuzz_one!(ctx, format!("INV.GCLK{i}_TEST"), "1", [
                (global_mutex "GCLK", "USE"),
                (mutex format!("GCLK{i}_TEST"), "INV"),
                (bel_mutex bel_bufhce, "MUX.I", format!("GCLK{i}")),
                (pip (pin format!("GCLK{i}")), (bel_pin bel_bufhce, "I"))
            ], [
                (pip (pin format!("GCLK{i}")), (pin format!("GCLK{i}_INV")))
            ]);
        }
        for lr in ['L', 'R'] {
            fuzz_one!(ctx, format!("BUF.BUFH_TEST_{lr}"), "1", [
                (mutex format!("BUFH_TEST_{lr}"), "BUF")
            ], [
                (pip (pin format!("BUFH_TEST_{lr}_PRE")), (pin format!("BUFH_TEST_{lr}_NOINV")))
            ]);
            fuzz_one!(ctx, format!("INV.BUFH_TEST_{lr}"), "1", [
                (mutex format!("BUFH_TEST_{lr}"), "INV")
            ], [
                (pip (pin format!("BUFH_TEST_{lr}_PRE")), (pin format!("BUFH_TEST_{lr}_INV")))
            ]);
            for i in 0..12 {
                fuzz_one!(ctx, format!("MUX.BUFH_TEST_{lr}"), format!("HCLK{i}_{lr}"), [
                    (global_mutex "HCLK", "USE"),
                    (row_mutex "BUFH_TEST", "NOPE"),
                    (mutex format!("MUX.BUFH_TEST_{lr}"), format!("HCLK{i}_{lr}")),
                    (bel_mutex bel_mmcm0, format!("MUX.CLKIN1_HCLK_{lr}"), format!("HCLK{i}_L")),
                    (pip (pin format!("HCLK{i}_{lr}_I")), (pin format!("MMCM0_CLKIN1_HCLK_{lr}")))
                ], [
                    (pip (pin format!("HCLK{i}_{lr}_I")), (pin format!("BUFH_TEST_{lr}_PRE")))
                ]);
            }
            for i in 0..6 {
                fuzz_one!(ctx, format!("MUX.BUFH_TEST_{lr}"), format!("RCLK{i}_{lr}"), [
                    (global_mutex "RCLK", "USE"),
                    (row_mutex "BUFH_TEST", "NOPE"),
                    (mutex format!("MUX.BUFH_TEST_{lr}"), format!("RCLK{i}_{lr}")),
                    (bel_mutex bel_mmcm0, format!("MUX.CLKIN1_HCLK_{lr}"), format!("RCLK{i}_L")),
                    (pip (pin format!("RCLK{i}_{lr}_I")), (pin format!("MMCM0_CLKIN1_HCLK_{lr}")))
                ], [
                    (pip (pin format!("RCLK{i}_{lr}_I")), (pin format!("BUFH_TEST_{lr}_PRE")))
                ]);
            }
            for i in 0..12 {
                fuzz_one!(ctx, format!("MUX.BUFH_TEST_{lr}"), format!("HCLK{i}_{lr}.EXCL"), [
                    (global_mutex "HCLK", "TEST"),
                    (row_mutex "BUFH_TEST", "NOPE"),
                    (mutex format!("MUX.BUFH_TEST_{lr}"), format!("HCLK{i}_{lr}"))
                ], [
                    (pip (pin format!("HCLK{i}_{lr}_I")), (pin format!("BUFH_TEST_{lr}_PRE")))
                ]);
            }
            for i in 0..6 {
                let extras = vec![ExtraFeature::new(
                    ExtraFeatureKind::HclkIoiInnerSide(if lr == 'L' { Dir::W } else { Dir::E }),
                    "HCLK_IOI",
                    "HCLK_IOI",
                    format!("ENABLE.RCLK{i}"),
                    "1",
                )];
                fuzz_one_extras!(ctx, format!("MUX.BUFH_TEST_{lr}"), format!("RCLK{i}_{lr}.EXCL"), [
                    (global_mutex "RCLK", format!("TEST{i}")),
                    (row_mutex "BUFH_TEST", "NOPE"),
                    (mutex format!("MUX.BUFH_TEST_{lr}"), format!("RCLK{i}_{lr}"))
                ], [
                    (pip (pin format!("RCLK{i}_{lr}_I")), (pin format!("BUFH_TEST_{lr}_PRE")))
                ], extras);
            }
        }
        for i in 0..32 {
            let oi = i ^ 1;
            fuzz_one!(ctx, format!("MUX.CASCO{i}"), "CASCI", [
                (mutex format!("MUX.CASCO{i}"), "CASCI")
            ], [
                (pip (pin format!("CASCI{i}")), (pin format!("CASCO{i}")))
            ]);
            fuzz_one!(ctx, format!("MUX.CASCO{i}"), "GCLK_TEST", [
                (mutex format!("MUX.CASCO{i}"), "GCLK_TEST")
            ], [
                (pip (pin format!("GCLK{i}_TEST")), (pin format!("CASCO{i}")))
            ]);
            for lr in ['L', 'R'] {
                fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("BUFH_TEST_{lr}"), [
                    (mutex format!("MUX.CASCO{i}"), format!("BUFH_TEST_{lr}"))
                ], [
                    (pip (pin format!("BUFH_TEST_{lr}")), (pin format!("CASCO{i}")))
                ]);
                for j in 0..4 {
                    fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("CCIO{j}_{lr}"), [
                        (tile_mutex "CCIO", "USE"),
                        (mutex format!("MUX.CASCO{i}"), format!("CCIO{j}_{lr}")),
                        (mutex format!("MUX.CASCO{oi}"), format!("CCIO{j}_{lr}")),
                        (pip (pin format!("CCIO{j}_{lr}")), (pin format!("CASCO{oi}")))
                    ], [
                        (pip (pin format!("CCIO{j}_{lr}")), (pin format!("CASCO{i}")))
                    ]);
                }
                for j in 0..10 {
                    fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("MGT{j}_{lr}"), [
                        (row_mutex "MGT", "USE"),
                        (mutex format!("MUX.CASCO{i}"), format!("MGT{j}_{lr}")),
                        (mutex format!("MUX.CASCO{oi}"), format!("MGT{j}_{lr}")),
                        (pip (pin format!("MGT{j}_{lr}")), (pin format!("CASCO{oi}")))
                    ], [
                        (pip (pin format!("MGT{j}_{lr}")), (pin format!("CASCO{i}")))
                    ]);
                }
                for j in 0..6 {
                    fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}"), [
                        (global_mutex "RCLK", "USE"),
                        (mutex format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}")),
                        (mutex format!("MUX.CASCO{oi}"), format!("RCLK{j}_{lr}")),
                        (pip (pin format!("RCLK{j}_{lr}_I")), (pin format!("CASCO{oi}")))
                    ], [
                        (pip (pin format!("RCLK{j}_{lr}_I")), (pin format!("CASCO{i}")))
                    ]);
                }
            }
            for j in 0..2 {
                for (k, pin) in [
                    "CLKOUT0",
                    "CLKOUT0B",
                    "CLKOUT1",
                    "CLKOUT1B",
                    "CLKOUT2",
                    "CLKOUT2B",
                    "CLKOUT3",
                    "CLKOUT3B",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKOUT6",
                    "CLKFBOUT",
                    "CLKFBOUTB",
                    "TMUXOUT",
                ]
                .into_iter()
                .enumerate()
                {
                    fuzz_one!(ctx, format!("MUX.CASCO{i}"), format!("MMCM{j}_{pin}"), [
                        (mutex format!("MUX.CASCO{i}"), format!("MMCM{j}_{pin}"))
                    ], [
                        (pip (pin format!("MMCM{j}_OUT{k}")), (pin format!("CASCO{i}")))
                    ]);
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tile = "CMT";
    if devdata_only {
        for bel in ["MMCM0", "MMCM1"] {
            let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "ZHOLD");
            let dly_val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IN_DLY_SET"),
                &bitvec![0; 5],
                &mut diff,
            );
            ctx.insert_device_data("MMCM:IN_DLY_SET", dly_val);
        }
        return;
    }
    for i in 0..12 {
        for lr in ['L', 'R'] {
            let bel = &format!("BUFHCE_{lr}{i}");
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_inv(tile, bel, "CE");
            ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

            if i == 0 {
                for j in 0..2 {
                    let diff = ctx
                        .state
                        .get_diff(tile, bel, "MUX.I", format!("CKINT{j}.EXCL"))
                        .combine(&!ctx.state.peek_diff(tile, bel, "MUX.I", format!("CKINT{j}")));
                    ctx.tiledb.insert(
                        tile,
                        "CMT",
                        format!("ENABLE.BUFHCE_{lr}_CKINT{j}"),
                        xlat_bit(diff),
                    );
                }
                if lr == 'L' {
                    for ilr in ['L', 'R'] {
                        for j in 0..4 {
                            let diff = ctx
                                .state
                                .get_diff(tile, bel, "MUX.I", format!("CCIO{j}_{ilr}.EXCL"))
                                .combine(&!ctx.state.peek_diff(
                                    tile,
                                    bel,
                                    "MUX.I",
                                    format!("CCIO{j}_{ilr}"),
                                ));
                            ctx.tiledb.insert(
                                tile,
                                "CMT",
                                format!("ENABLE.CCIO{j}_{ilr}"),
                                xlat_bit(diff),
                            );
                        }
                    }
                }
            }

            let mut vals = vec![];
            for j in 0..32 {
                vals.push(format!("GCLK{j}"));
            }
            for j in 0..4 {
                vals.push(format!("CCIO{j}_L"));
                vals.push(format!("CCIO{j}_R"));
            }
            for j in 0..2 {
                vals.push(format!("CKINT{j}"));
            }
            vals.push("BUFH_TEST_L".to_string());
            vals.push("BUFH_TEST_R".to_string());
            for i in 0..2 {
                for out in [
                    "CLKOUT0",
                    "CLKOUT0B",
                    "CLKOUT1",
                    "CLKOUT1B",
                    "CLKOUT2",
                    "CLKOUT2B",
                    "CLKOUT3",
                    "CLKOUT3B",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKOUT6",
                    "CLKFBOUT",
                    "CLKFBOUTB",
                    "TMUXOUT",
                ] {
                    vals.push(format!("MMCM{i}_{out}"));
                }
            }
            ctx.collect_enum_default_ocd(tile, bel, "MUX.I", &vals, "NONE", OcdMode::Mux);
        }
    }
    for i in 0..2 {
        let bel = &format!("MMCM{i}");

        fn mmcm_drp_bit(which: usize, reg: usize, bit: usize) -> TileBit {
            let tile = if which == 0 {
                17 - (reg >> 3)
            } else {
                22 + (reg >> 3)
            };
            let frame = 26 + (bit & 1);
            let bit = (bit >> 1) | (reg & 7) << 3;
            let bit = if which == 0 { bit ^ 0x3f } else { bit };
            TileBit::new(tile, frame, bit)
        }
        for reg in 0..0x80 {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec(
                    (0..16).map(|bit| mmcm_drp_bit(i, reg, bit)).collect(),
                    false,
                ),
            );
        }

        for pin in ["RST", "PWRDWN", "CLKINSEL", "PSEN", "PSINCDEC"] {
            ctx.collect_inv(tile, bel, pin);
        }

        for attr in [
            "CASC_LOCK_EN",
            "CLKBURST_ENABLE",
            "CLKBURST_REPEAT",
            "CLKFBOUT_EN",
            "CLKOUT0_EN",
            "CLKOUT1_EN",
            "CLKOUT2_EN",
            "CLKOUT3_EN",
            "CLKOUT4_EN",
            "CLKOUT5_EN",
            "CLKOUT6_EN",
            "CLKFBOUT_USE_FINE_PS",
            "CLKOUT0_USE_FINE_PS",
            "CLKOUT1_USE_FINE_PS",
            "CLKOUT2_USE_FINE_PS",
            "CLKOUT3_USE_FINE_PS",
            "CLKOUT4_USE_FINE_PS",
            "CLKOUT5_USE_FINE_PS",
            "CLKOUT6_USE_FINE_PS",
            "CLKFBOUT_FRAC_EN",
            "CLKOUT0_FRAC_EN",
            "CLKOUT4_CASCADE",
            "CLOCK_HOLD",
            "DIRECT_PATH_CNTRL",
            "EN_VCO_DIV1",
            "EN_VCO_DIV6",
            "HVLF_STEP",
            "HVLF_CNT_TEST_EN",
            "IN_DLY_EN",
            "STARTUP_WAIT",
            "VLF_HIGH_DIS_B",
            "VLF_HIGH_PWDN_B",
        ] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for attr in [
            "ANALOG_MISC",
            "AVDD_COMP_SET",
            "AVDD_VBG_PD",
            "AVDD_VBG_SEL",
            "CLKBURST_CNT",
            "CLKFBIN_HT",
            "CLKFBIN_LT",
            "CLKFBOUT_DT",
            "CLKFBOUT_HT",
            "CLKFBOUT_LT",
            "CLKFBOUT_MX",
            "CLKFBOUT_PM",
            "CLKFBOUT_FRAC",
            "CLKOUT0_DT",
            "CLKOUT0_HT",
            "CLKOUT0_LT",
            "CLKOUT0_MX",
            "CLKOUT0_PM",
            "CLKOUT0_FRAC",
            "CLKOUT1_DT",
            "CLKOUT1_HT",
            "CLKOUT1_LT",
            "CLKOUT1_MX",
            "CLKOUT1_PM",
            "CLKOUT2_DT",
            "CLKOUT2_HT",
            "CLKOUT2_LT",
            "CLKOUT2_MX",
            "CLKOUT2_PM",
            "CLKOUT3_DT",
            "CLKOUT3_HT",
            "CLKOUT3_LT",
            "CLKOUT3_MX",
            "CLKOUT3_PM",
            "CLKOUT4_DT",
            "CLKOUT4_HT",
            "CLKOUT4_LT",
            "CLKOUT4_MX",
            "CLKOUT4_PM",
            "CLKOUT5_DT",
            "CLKOUT5_HT",
            "CLKOUT5_LT",
            "CLKOUT5_MX",
            "CLKOUT5_PM",
            "CLKOUT6_DT",
            "CLKOUT6_HT",
            "CLKOUT6_LT",
            "CLKOUT6_MX",
            "CLKOUT6_PM",
            "CONTROL_0",
            "CONTROL_1",
            "CONTROL_2",
            "CONTROL_3",
            "CONTROL_4",
            "CONTROL_5",
            "CP",
            "CP_BIAS_TRIP_SET",
            "CP_RES",
            "DIVCLK_HT",
            "DIVCLK_LT",
            "DVDD_COMP_SET",
            "DVDD_VBG_PD",
            "DVDD_VBG_SEL",
            "FINE_PS_FRAC",
            "HROW_DLY_SET",
            "HVLF_CNT_TEST",
            "INTERP_EN",
            "IN_DLY_MX_CVDD",
            "IN_DLY_MX_DVDD",
            "LF_NEN",
            "LF_PEN",
            "LFHF",
            "MAN_LF",
            "LOCK_CNT",
            "LOCK_FB_DLY",
            "LOCK_REF_DLY",
            "LOCK_SAT_HIGH",
            "PFD",
            "RES",
            "TMUX_MUX_SEL",
            "UNLOCK_CNT",
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
        }

        for (addr, name) in [(0x16, "DIVCLK"), (0x17, "CLKFBIN")] {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_NOCOUNT"),
                TileItem::from_bit(mmcm_drp_bit(i, addr, 12), false),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit(mmcm_drp_bit(i, addr, 13), false),
            );
        }
        for (addr, name) in [
            (0x07, "CLKOUT5"),
            (0x09, "CLKOUT0"),
            (0x13, "CLKOUT6"),
            (0x15, "CLKFBOUT"),
        ] {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_FRAC_WF"),
                TileItem::from_bit(mmcm_drp_bit(i, addr, 10), false),
            );
        }
        for (addr, name) in [
            (0x07, "CLKOUT5"),
            (0x09, "CLKOUT0"),
            (0x0b, "CLKOUT1"),
            (0x0d, "CLKOUT2"),
            (0x0f, "CLKOUT3"),
            (0x11, "CLKOUT4"),
            (0x13, "CLKOUT6"),
            (0x15, "CLKFBOUT"),
        ] {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_NOCOUNT"),
                TileItem::from_bit(mmcm_drp_bit(i, addr, 6), false),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{name}_EDGE"),
                TileItem::from_bit(mmcm_drp_bit(i, addr, 7), false),
            );
        }

        ctx.tiledb.insert(
            tile,
            bel,
            "SYNTH_CLK_DIV",
            TileItem::from_bitvec(
                vec![mmcm_drp_bit(i, 0x02, 0), mmcm_drp_bit(i, 0x02, 1)],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "IN_DLY_SET",
            TileItem::from_bitvec(
                vec![
                    mmcm_drp_bit(i, 0x05, 10),
                    mmcm_drp_bit(i, 0x05, 11),
                    mmcm_drp_bit(i, 0x05, 12),
                    mmcm_drp_bit(i, 0x05, 13),
                    mmcm_drp_bit(i, 0x05, 14),
                ],
                false,
            ),
        );

        ctx.state
            .get_diff(tile, bel, "GTS_WAIT", "FALSE")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "GTS_WAIT", "TRUE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "SYNTH_CLK_DIV"), 1, 0);
        ctx.tiledb.insert(tile, bel, "GTS_WAIT", xlat_bit(diff));

        ctx.tiledb.insert(
            tile,
            bel,
            "MMCM_EN",
            TileItem::from_bit(mmcm_drp_bit(i, 0x74, 0), false),
        );

        let mut enable = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        enable.apply_bit_diff(ctx.tiledb.item(tile, bel, "MMCM_EN"), true, false);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "RES"), 0xf, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CP"), 0x5, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "INTERP_EN"), 0x10, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBIN_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBIN_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DIVCLK_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DIVCLK_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBOUT_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKFBOUT_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT0_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT0_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT1_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT1_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT2_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT2_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT3_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT3_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT4_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT4_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT5_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT5_LT"), 0x3f, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT6_HT"), 1, 0);
        enable.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKOUT6_LT"), 0x3f, 0);
        assert_eq!(enable.bits.len(), 1);
        let drp_mask = enable.filter_tiles(&[40]);
        assert_eq!(drp_mask.bits.len(), 1);
        ctx.tiledb.insert(
            "HCLK",
            "HCLK",
            ["DRP_MASK_BELOW", "DRP_MASK_ABOVE"][i],
            xlat_bit(drp_mask),
        );

        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "BUF_IN");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "CASCADE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "CASC_LOCK_EN"), true, false);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x0a, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "EXTERNAL");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x31, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "INTERNAL");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x2f, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x12, 0);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "COMPENSATION", "ZHOLD");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_DVDD"), 0x01, 0);
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IN_DLY_MX_CVDD"), 0x18, 0);
        let dly_val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IN_DLY_SET"),
            &bitvec![0; 5],
            &mut diff,
        );
        ctx.insert_device_data("MMCM:IN_DLY_SET", dly_val);
        diff.assert_empty();

        for mult in 1..=64 {
            for bandwidth in ["LOW", "HIGH"] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, "TABLES", format!("{mult}.{bandwidth}"));
                for (attr, base) in [
                    ("CP", bitvec![1, 0, 1, 0]),
                    ("RES", bitvec![1, 1, 1, 1]),
                    ("LFHF", bitvec![0, 0]),
                ] {
                    let val =
                        extract_bitvec_val_part(ctx.tiledb.item(tile, bel, attr), &base, &mut diff);
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.tiledb
                        .insert_misc_data(format!("MMCM:{attr}:{bandwidth}:{mult}"), ival);
                }
                for (attr, width) in [
                    ("LOCK_REF_DLY", 5),
                    ("LOCK_FB_DLY", 5),
                    ("LOCK_CNT", 10),
                    ("LOCK_SAT_HIGH", 10),
                    ("UNLOCK_CNT", 10),
                ] {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, attr),
                        &BitVec::repeat(false, width),
                        &mut diff,
                    );
                    let mut ival = 0;
                    for (i, v) in val.into_iter().enumerate() {
                        if v {
                            ival |= 1 << i;
                        }
                    }
                    ctx.tiledb
                        .insert_misc_data(format!("MMCM:{attr}:{mult}"), ival);
                }
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_NOCOUNT"));
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_EDGE"));
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_LT"));
                diff.discard_bits(ctx.tiledb.item(tile, bel, "CLKFBOUT_HT"));
                diff.assert_empty();
            }
        }

        if i == 0 {
            for j in 0..10 {
                for lr in ['L', 'R'] {
                    let diff = ctx
                        .state
                        .get_diff(tile, bel, "MUX.CLKIN1_MGT", format!("MGT{j}_{lr}.EXCL"))
                        .combine(&!ctx.state.peek_diff(
                            tile,
                            bel,
                            "MUX.CLKIN1_MGT",
                            format!("MGT{j}_{lr}"),
                        ));
                    ctx.tiledb
                        .insert(tile, "CMT", format!("ENABLE.MGT{j}_{lr}"), xlat_bit(diff));
                }
            }
        }
        let mut vals = vec![];
        for j in 0..8 {
            vals.push(format!("GIO{j}"));
        }
        for j in 0..4 {
            for lr in ['L', 'R'] {
                vals.push(format!("CCIO{j}_{lr}"));
            }
        }
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKIN1_IO", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKIN2_IO", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKFBIN_IO", &vals, "NONE", OcdMode::Mux);
        let mut vals = vec![];
        for j in 0..10 {
            for lr in ['L', 'R'] {
                vals.push(format!("MGT{j}_{lr}"));
            }
        }
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKIN1_MGT", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKIN2_MGT", &vals, "NONE", OcdMode::Mux);
        for lr in ['L', 'R'] {
            let mut vals = vec![];
            for j in 0..12 {
                vals.push(format!("HCLK{j}_{lr}"));
            }
            for j in 0..6 {
                vals.push(format!("RCLK{j}_{lr}"));
            }
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.CLKIN1_HCLK_{lr}"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.CLKIN2_HCLK_{lr}"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.CLKFBIN_HCLK_{lr}"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
        }
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKFBIN_HCLK",
            &["CLKFBIN_HCLK_L", "CLKFBIN_HCLK_R"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKIN1_HCLK",
            &["CLKIN1_HCLK_L", "CLKIN1_HCLK_R"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKIN2_HCLK",
            &["CLKIN2_HCLK_L", "CLKIN2_HCLK_R"],
        );

        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKIN1",
            &["CLKIN1_IO", "CLKIN1_HCLK", "CLKIN1_MGT", "CLKIN1_CKINT"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKIN2",
            &["CLKIN2_IO", "CLKIN2_HCLK", "CLKIN2_MGT", "CLKIN2_CKINT"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKFBIN",
            &["CLKFBIN_IO", "CLKFBIN_HCLK", "CLKFBIN_CKINT"],
        );
        // ???
        ctx.state
            .get_diff(tile, bel, "MUX.CLKFBIN", "CASC_OUT")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "MUX.CLKFBIN", "CLKFBOUT")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "MUX.CLKIN1", "CASC_IN")
            .assert_empty();
        ctx.collect_enum(
            tile,
            bel,
            "MUX.CASC_OUT",
            &[
                "CLKOUT0", "CLKOUT1", "CLKOUT2", "CLKOUT3", "CLKOUT4", "CLKOUT5", "CLKOUT6",
                "CLKFBOUT",
            ],
        );
        for j in 0..4 {
            let jj = j ^ 1;
            let mut diffs = vec![];
            for k in 0..4 {
                let diff_ol =
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PERF{j}.OL"), format!("CLKOUT{k}"));
                let diff_or =
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PERF{j}.OR"), format!("CLKOUT{k}"));
                let diff_il =
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PERF{j}.IL"), format!("CLKOUT{k}"));
                let diff_ir =
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.PERF{j}.IR"), format!("CLKOUT{k}"));
                let (diff_ol, diff_il, diff_l) = Diff::split(diff_ol, diff_il);
                let (diff_or, diff_ir, diff_r) = Diff::split(diff_or, diff_ir);
                assert_eq!(diff_l, diff_r);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.PERF{jj}_OL"), xlat_bit(diff_ol));
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.PERF{jj}_OR"), xlat_bit(diff_or));
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.PERF{j}_IL"), xlat_bit(diff_il));
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.PERF{j}_IR"), xlat_bit(diff_ir));
                diffs.push((format!("CLKOUT{k}"), diff_l));
            }
            diffs.push(("NONE".to_string(), Diff::default()));
            ctx.tiledb
                .insert(tile, bel, format!("MUX.PERF{j}"), xlat_enum(diffs));
        }
    }
    {
        let bel = "CMT";
        ctx.state
            .get_diff(tile, bel, "BUF.BUFH_TEST_L", "1")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "BUF.BUFH_TEST_R", "1")
            .assert_empty();
        ctx.collect_bit(tile, bel, "INV.BUFH_TEST_L", "1");
        ctx.collect_bit(tile, bel, "INV.BUFH_TEST_R", "1");
        for i in 0..32 {
            ctx.collect_bit(tile, bel, &format!("BUF.GCLK{i}_TEST"), "1");
            let mut diff = ctx
                .state
                .get_diff(tile, bel, format!("INV.GCLK{i}_TEST"), "1");
            diff.apply_bit_diff(
                ctx.tiledb.item(tile, bel, &format!("BUF.GCLK{i}_TEST")),
                true,
                false,
            );
            // FUCKERY MURDER HORSESHIT ISE
            match i {
                6 | 14 => {
                    assert_eq!(diff.bits.len(), 2);
                    let diff_n = diff.split_bits_by(|bit| bit.frame == 31);
                    ctx.tiledb
                        .insert(tile, bel, format!("INV.GCLK{i}_TEST"), xlat_bit(diff));
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        format!("INV.GCLK{}_TEST", i + 1),
                        xlat_bit(diff_n),
                    );
                }
                7 | 15 => {
                    diff.assert_empty();
                }
                _ => {
                    ctx.tiledb
                        .insert(tile, bel, format!("INV.GCLK{i}_TEST"), xlat_bit(diff));
                }
            }
        }
        for lr in ['L', 'R'] {
            for j in 0..12 {
                let diff = ctx
                    .state
                    .get_diff(
                        tile,
                        bel,
                        format!("MUX.BUFH_TEST_{lr}"),
                        format!("HCLK{j}_{lr}.EXCL"),
                    )
                    .combine(&!ctx.state.peek_diff(
                        tile,
                        bel,
                        format!("MUX.BUFH_TEST_{lr}"),
                        format!("HCLK{j}_{lr}"),
                    ));
                ctx.tiledb
                    .insert(tile, "CMT", format!("ENABLE.HCLK{j}_{lr}"), xlat_bit(diff));
            }
            for j in 0..6 {
                let diff = ctx
                    .state
                    .get_diff(
                        tile,
                        bel,
                        format!("MUX.BUFH_TEST_{lr}"),
                        format!("RCLK{j}_{lr}.EXCL"),
                    )
                    .combine(&!ctx.state.peek_diff(
                        tile,
                        bel,
                        format!("MUX.BUFH_TEST_{lr}"),
                        format!("RCLK{j}_{lr}"),
                    ));
                ctx.tiledb
                    .insert(tile, "CMT", format!("ENABLE.RCLK{j}_{lr}"), xlat_bit(diff));
            }
            let mut vals = vec![];
            for i in 0..12 {
                vals.push(format!("HCLK{i}_{lr}"));
            }
            for i in 0..6 {
                vals.push(format!("RCLK{i}_{lr}"));
            }
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.BUFH_TEST_{lr}"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
        }
        for i in 0..32 {
            let mut vals = vec!["CASCI".to_string()];
            for j in 0..2 {
                for out in [
                    "CLKOUT0",
                    "CLKOUT0B",
                    "CLKOUT1",
                    "CLKOUT1B",
                    "CLKOUT2",
                    "CLKOUT2B",
                    "CLKOUT3",
                    "CLKOUT3B",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKOUT6",
                    "CLKFBOUT",
                    "CLKFBOUTB",
                    "TMUXOUT",
                ] {
                    vals.push(format!("MMCM{j}_{out}"));
                }
            }
            for lr in ['L', 'R'] {
                for j in 0..4 {
                    vals.push(format!("CCIO{j}_{lr}"));
                }
                for j in 0..10 {
                    vals.push(format!("MGT{j}_{lr}"));
                }
                for j in 0..6 {
                    vals.push(format!("RCLK{j}_{lr}"));
                }
            }
            vals.extend([
                "GCLK_TEST".to_string(),
                "BUFH_TEST_L".to_string(),
                "BUFH_TEST_R".to_string(),
            ]);
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.CASCO{i}"),
                &vals,
                "NONE",
                OcdMode::Mux,
            );
        }
    }
    {
        let tile = "HCLK_IOI";
        let bel = "HCLK_IOI";
        let diffs: [_; 6] = core::array::from_fn(|i| {
            ctx.state
                .get_diff(tile, bel, format!("ENABLE.RCLK{i}"), "1")
        });
        let mut all = Diff::default();
        for diff in &diffs {
            for (&k, &v) in &diff.bits {
                all.bits.insert(k, v);
            }
        }
        for i in 0..6 {
            let diff = all.combine(&!&diffs[i]);
            ctx.tiledb
                .insert(tile, bel, format!("UNUSED.RCLK{i}"), xlat_bit(diff));
        }
    }
}
