use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelPin, IntDb, PairMux, SwitchBoxItem, TileWireCoord},
    dir::{Dir, DirMap, DirV},
};
use prjcombine_re_xilinx_rawdump::Part;

use prjcombine_re_xilinx_naming::db::{
    BelNaming, NamingDb, PipNaming, RawTileId, TileClassNaming, WireNaming,
};
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};
use prjcombine_spartan6::defs::{self, bslots, tcls, wires};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => defs::ccls::PASS_W,
        Dir::E => defs::ccls::PASS_E,
        Dir::S => defs::ccls::PASS_S,
        Dir::N => defs::ccls::PASS_N,
    }));

    builder.wire_names(wires::PULLUP, &["KEEP1_WIRE"]);
    builder.wire_names(wires::TIE_0, &["GND_WIRE"]);
    builder.wire_names(
        wires::TIE_1,
        &["VCC_WIRE", "REGL_VCC", "REGR_VCC", "REGB_VCC", "REGT_VCC"],
    );

    for i in 0..16 {
        builder.wire_names(
            wires::HCLK[i],
            &[format!("GCLK{i}"), format!("GCLK{i}_BRK")],
        );
        builder.extra_name_sub(format!("HCLK_GCLK{i}_INT"), 1, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_GCLK{i}_INT_FOLD"), 1, wires::HCLK_ROW[i]);
        // a bit of a lie to make mux extraction work
        builder.extra_name_sub(format!("CLKV_GCLKH_L{i}"), 0, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("CLKV_GCLKH_R{i}"), 1, wires::HCLK_ROW[i]);
    }

    for (lr, w0, w1, dir, dend) in [
        (
            "L",
            wires::SNG_E0,
            wires::SNG_E1,
            Dir::E,
            Some((0, Dir::S, wires::SNG_E1_S0)),
        ),
        (
            "R",
            wires::SNG_E0,
            wires::SNG_E1,
            Dir::E,
            Some((3, Dir::N, wires::SNG_E1_N7)),
        ),
        (
            "L",
            wires::SNG_W0,
            wires::SNG_W1,
            Dir::W,
            Some((3, Dir::N, wires::SNG_W1_N3)),
        ),
        (
            "R",
            wires::SNG_W0,
            wires::SNG_W1,
            Dir::W,
            Some((0, Dir::S, wires::SNG_W1_S4)),
        ),
        (
            "L",
            wires::SNG_N0,
            wires::SNG_N1,
            Dir::N,
            Some((0, Dir::S, wires::SNG_N1_S0)),
        ),
        ("R", wires::SNG_N0, wires::SNG_N1, Dir::N, None),
        ("L", wires::SNG_S0, wires::SNG_S1, Dir::S, None),
        (
            "R",
            wires::SNG_S0,
            wires::SNG_S1,
            Dir::S,
            Some((3, Dir::N, wires::SNG_S1_N7)),
        ),
    ] {
        for i in 0..4 {
            let ii = if lr == "L" { i } else { i + 4 };
            builder.wire_names(w0[ii], &[format!("{dir}{lr}1B{i}")]);
            builder.wire_names(w1[ii], &[format!("{dir}{lr}1E{i}")]);
            if let Some((xi, dend, wend)) = dend
                && i == xi
            {
                builder.wire_names(wend, &[format!("{dir}{lr}1E_{dend}{i}")]);
            }
        }
    }

    for (da, db, w0, w1, w2, dend) in [
        (
            Dir::E,
            Dir::E,
            wires::DBL_EE0,
            wires::DBL_EE1,
            wires::DBL_EE2,
            None,
        ),
        (
            Dir::W,
            Dir::W,
            wires::DBL_WW0,
            wires::DBL_WW1,
            wires::DBL_WW2,
            Some((3, Dir::N, wires::DBL_WW2_N3)),
        ),
        (
            Dir::N,
            Dir::N,
            wires::DBL_NN0,
            wires::DBL_NN1,
            wires::DBL_NN2,
            Some((0, Dir::S, wires::DBL_NN2_S0)),
        ),
        (
            Dir::N,
            Dir::E,
            wires::DBL_NE0,
            wires::DBL_NE1,
            wires::DBL_NE2,
            Some((0, Dir::S, wires::DBL_NE2_S0)),
        ),
        (
            Dir::N,
            Dir::W,
            wires::DBL_NW0,
            wires::DBL_NW1,
            wires::DBL_NW2,
            Some((0, Dir::S, wires::DBL_NW2_S0)),
        ),
        (
            Dir::S,
            Dir::S,
            wires::DBL_SS0,
            wires::DBL_SS1,
            wires::DBL_SS2,
            Some((3, Dir::N, wires::DBL_SS2_N3)),
        ),
        (
            Dir::S,
            Dir::E,
            wires::DBL_SE0,
            wires::DBL_SE1,
            wires::DBL_SE2,
            None,
        ),
        (
            Dir::S,
            Dir::W,
            wires::DBL_SW0,
            wires::DBL_SW1,
            wires::DBL_SW2,
            Some((3, Dir::N, wires::DBL_SW2_N3)),
        ),
    ] {
        for i in 0..4 {
            builder.wire_names(w0[i], &[format!("{da}{db}2B{i}")]);
            builder.wire_names(w1[i], &[format!("{da}{db}2M{i}")]);
            builder.wire_names(w2[i], &[format!("{da}{db}2E{i}")]);
            if let Some((xi, dend, wend)) = dend
                && i == xi
            {
                builder.wire_names(wend, &[format!("{da}{db}2E_{dend}{i}")]);
            }
        }
    }

    for (da, db, w0, w1, w2, w3, w4, dend) in [
        (
            Dir::E,
            Dir::E,
            wires::QUAD_EE0,
            wires::QUAD_EE1,
            wires::QUAD_EE2,
            wires::QUAD_EE3,
            wires::QUAD_EE4,
            None,
        ),
        (
            Dir::W,
            Dir::W,
            wires::QUAD_WW0,
            wires::QUAD_WW1,
            wires::QUAD_WW2,
            wires::QUAD_WW3,
            wires::QUAD_WW4,
            Some((0, Dir::S, wires::QUAD_WW4_S0)),
        ),
        (
            Dir::N,
            Dir::N,
            wires::QUAD_NN0,
            wires::QUAD_NN1,
            wires::QUAD_NN2,
            wires::QUAD_NN3,
            wires::QUAD_NN4,
            None,
        ),
        (
            Dir::N,
            Dir::E,
            wires::QUAD_NE0,
            wires::QUAD_NE1,
            wires::QUAD_NE2,
            wires::QUAD_NE3,
            wires::QUAD_NE4,
            None,
        ),
        (
            Dir::N,
            Dir::W,
            wires::QUAD_NW0,
            wires::QUAD_NW1,
            wires::QUAD_NW2,
            wires::QUAD_NW3,
            wires::QUAD_NW4,
            Some((0, Dir::S, wires::QUAD_NW4_S0)),
        ),
        (
            Dir::S,
            Dir::S,
            wires::QUAD_SS0,
            wires::QUAD_SS1,
            wires::QUAD_SS2,
            wires::QUAD_SS3,
            wires::QUAD_SS4,
            Some((3, Dir::N, wires::QUAD_SS4_N3)),
        ),
        (
            Dir::S,
            Dir::E,
            wires::QUAD_SE0,
            wires::QUAD_SE1,
            wires::QUAD_SE2,
            wires::QUAD_SE3,
            wires::QUAD_SE4,
            None,
        ),
        (
            Dir::S,
            Dir::W,
            wires::QUAD_SW0,
            wires::QUAD_SW1,
            wires::QUAD_SW2,
            wires::QUAD_SW3,
            wires::QUAD_SW4,
            Some((3, Dir::N, wires::QUAD_SW4_N3)),
        ),
    ] {
        for i in 0..4 {
            builder.wire_names(w0[i], &[format!("{da}{db}4B{i}")]);
            builder.wire_names(w1[i], &[format!("{da}{db}4A{i}")]);
            builder.wire_names(w2[i], &[format!("{da}{db}4M{i}")]);
            builder.wire_names(w3[i], &[format!("{da}{db}4C{i}")]);
            builder.wire_names(w4[i], &[format!("{da}{db}4E{i}")]);
            if let Some((xi, dend, wend)) = dend
                && i == xi
            {
                builder.wire_names(wend, &[format!("{da}{db}4E_{dend}{i}")]);
            }
        }
    }

    for i in 0..2 {
        builder.wire_names(
            wires::IMUX_GFAN[i],
            &[format!("GFAN{i}"), format!("INT_IOI_GFAN{i}")],
        );
    }
    for i in 0..2 {
        builder.wire_names(
            wires::IMUX_CLK[i],
            &[format!("CLK{i}"), format!("INT_TERM_CLK{i}")],
        );
    }
    for i in 0..2 {
        builder.wire_names(
            wires::IMUX_SR[i],
            &[format!("SR{i}"), format!("INT_TERM_SR{i}")],
        );
    }
    for i in 0..63 {
        builder.wire_names(
            wires::IMUX_LOGICIN[i],
            &[format!("LOGICIN_B{i}"), format!("INT_TERM_LOGICIN_B{i}")],
        );
        let (dir, bounce, sn) = match i {
            20 => (
                Dir::S,
                wires::IMUX_LOGICIN20_BOUNCE,
                wires::IMUX_LOGICIN20_S,
            ),
            36 => (
                Dir::S,
                wires::IMUX_LOGICIN36_BOUNCE,
                wires::IMUX_LOGICIN36_S,
            ),
            44 => (
                Dir::S,
                wires::IMUX_LOGICIN44_BOUNCE,
                wires::IMUX_LOGICIN44_S,
            ),
            62 => (
                Dir::S,
                wires::IMUX_LOGICIN62_BOUNCE,
                wires::IMUX_LOGICIN62_S,
            ),
            21 => (
                Dir::N,
                wires::IMUX_LOGICIN21_BOUNCE,
                wires::IMUX_LOGICIN21_N,
            ),
            28 => (
                Dir::N,
                wires::IMUX_LOGICIN28_BOUNCE,
                wires::IMUX_LOGICIN28_N,
            ),
            52 => (
                Dir::N,
                wires::IMUX_LOGICIN52_BOUNCE,
                wires::IMUX_LOGICIN52_N,
            ),
            60 => (
                Dir::N,
                wires::IMUX_LOGICIN60_BOUNCE,
                wires::IMUX_LOGICIN60_N,
            ),
            _ => continue,
        };
        builder.mark_permabuf(bounce);
        builder.wire_names(bounce, &[format!("LOGICIN{i}")]);
        builder.wire_names(sn, &[&format!("LOGICIN_{dir}{i}")]);
    }
    builder.wire_names(wires::IMUX_LOGICIN[63], &["FAN_B"]);

    for i in 0..24 {
        builder.wire_names(
            wires::OUT[i],
            &[format!("LOGICOUT{i}"), format!("INT_TERM_LOGICOUT{i}")],
        );
        builder.mark_test_mux_in(wires::OUT_BEL[i], wires::OUT[i]);
        builder.mark_test_mux_in_test(wires::OUT_TEST[i], wires::OUT[i]);
    }

    for i in 0..2 {
        builder.extra_name_sub(format!("BUFPLL_BOT_GCLK{i}"), 1, wires::IMUX_CLK_GCLK[i]);
        builder.extra_name_sub(format!("BUFPLL_TOP_GCLK{i}"), 0, wires::IMUX_CLK_GCLK[i]);
    }

    for i in 0..4 {
        for (wires, prefix) in [
            (wires::IMUX_BUFIO2_I, "I_BUFIO2"),
            (wires::IMUX_BUFIO2_IB, "IB_BUFIO2"),
            (wires::IMUX_BUFIO2FB, "IFB_BUFIO2FB"),
        ] {
            builder.extra_name_sub(format!("{prefix}_TOP_SITE{i}"), 0, wires[i]);
            builder.extra_name_sub(format!("{prefix}_TOP_SITE{ii}", ii = i + 4), 2, wires[i]);
            builder.extra_name_sub(format!("{prefix}_RIGHT_SITE{i}"), 2, wires[i]);
            builder.extra_name_sub(format!("{prefix}_RIGHT_SITE{ii}", ii = i + 4), 1, wires[i]);
            builder.extra_name_sub(format!("{prefix}_BOT_SITE{i}"), 3, wires[i]);
            builder.extra_name_sub(format!("{prefix}_BOT_SITE{ii}", ii = i + 4), 1, wires[i]);
            builder.extra_name_sub(format!("{prefix}_LEFT_SITE{i}"), 1, wires[i]);
            builder.extra_name_sub(format!("{prefix}_LEFT_SITE{ii}", ii = i + 4), 2, wires[i]);
        }
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_TOP_SITE{i}"),
            0,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_TOP_SITE{ii}", ii = i + 4),
            2,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_RIGHT_SITE{i}"),
            2,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_RIGHT_SITE{ii}", ii = i + 4),
            1,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_BOT_SITE{i}"),
            3,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_BOT_SITE{ii}", ii = i + 4),
            1,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_LEFT_SITE{i}"),
            1,
            wires::IMUX_BUFIO2FB[i],
        );
        builder.alt_name_sub(
            format!("IB_BUFIO2FB_LEFT_SITE{ii}", ii = i + 4),
            2,
            wires::IMUX_BUFIO2FB[i],
        );

        builder.wire_names(
            wires::IOCLK[i],
            &[
                format!("BIOI_INNER_IOCLK{i}"),
                format!("TIOI_INNER_IOCLK{i}"),
                format!("TIOI_IOCLK{i}"),
                format!("IOI_IOCLK{i}"),
                format!("IOI_IOCLK{i}_BRK"),
                format!("RIOI_IOCLK{i}"),
                format!("RIOI_IOCLK{i}_BRK"),
            ],
        );
        builder.extra_name_sub(format!("REGT_IOCLKOUT{i}"), 0, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGT_IOCLKOUT{ii}", ii = i + 4), 2, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGR_IOCLKOUT{i}"), 2, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGR_IOCLKOUT{ii}", ii = i + 4), 1, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGB_IOCLKOUT{i}"), 3, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGB_IOCLKOUT{ii}", ii = i + 4), 1, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGL_IOCLKOUT{i}"), 1, wires::IOCLK[i]);
        builder.extra_name_sub(format!("REGL_IOCLKOUT{ii}", ii = i + 4), 2, wires::IOCLK[i]);

        builder.extra_name_sub(format!("REGT_CKPIN_OUT{i}"), 0, wires::OUT_DIVCLK[i]);
        builder.extra_name_sub(
            format!("REGT_CKPIN_OUT{ii}", ii = i + 4),
            2,
            wires::OUT_DIVCLK[i],
        );
        builder.extra_name_sub(format!("REGR_CKPIN_OUT{i}"), 2, wires::OUT_DIVCLK[i]);
        builder.extra_name_sub(
            format!("REGR_CKPIN_OUT{ii}", ii = i + 4),
            1,
            wires::OUT_DIVCLK[i],
        );
        builder.extra_name_sub(format!("REGB_CKPIN_OUT{i}"), 3, wires::OUT_DIVCLK[i]);
        builder.extra_name_sub(
            format!("REGB_CKPIN_OUT{ii}", ii = i + 4),
            1,
            wires::OUT_DIVCLK[i],
        );
        builder.extra_name_sub(format!("REGL_CKPIN_OUT{i}"), 1, wires::OUT_DIVCLK[i]);
        builder.extra_name_sub(
            format!("REGL_CKPIN_OUT{ii}", ii = i + 4),
            2,
            wires::OUT_DIVCLK[i],
        );

        builder.wire_names(
            wires::IOCE[i],
            &[
                format!("BIOI_INNER_IOCE{i}"),
                format!("TIOI_INNER_IOCE{i}"),
                format!("TIOI_IOCE{i}"),
                format!("IOI_IOCE{i}"),
                format!("IOI_IOCE{i}_BRK"),
                format!("RIOI_IOCE{i}"),
                format!("RIOI_IOCE{i}_BRK"),
            ],
        );
        builder.extra_name_sub(format!("REGT_IOCEOUT{i}"), 0, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGT_IOCEOUT{ii}", ii = i + 4), 2, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGR_IOCEOUT{i}"), 2, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGR_IOCEOUT{ii}", ii = i + 4), 1, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGB_IOCEOUT{i}"), 3, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGB_IOCEOUT{ii}", ii = i + 4), 1, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGL_IOCEOUT{i}"), 1, wires::IOCE[i]);
        builder.extra_name_sub(format!("REGL_IOCEOUT{ii}", ii = i + 4), 2, wires::IOCE[i]);

        builder.wire_names(
            wires::GTPCLK[i],
            &[
                format!("GTPDUAL_GTPCLKOUT{i}"),
                format!("GTPDUAL_BOT_GTPCLKOUT{i}"),
            ],
        );
        builder.extra_name_sub(format!("REGT_GTPCLK{i}"), 0, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGT_GTPCLK{ii}", ii = i + 4), 2, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGR_GTPCLK{i}"), 2, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGR_GTPCLK{ii}", ii = i + 4), 1, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGB_GTPCLK{i}"), 3, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGB_GTPCLK{ii}", ii = i + 4), 1, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGL_GTPCLK{i}"), 1, wires::GTPCLK[i]);
        builder.extra_name_sub(format!("REGL_GTPCLK{ii}", ii = i + 4), 2, wires::GTPCLK[i]);

        if i < 2 {
            builder.wire_names(
                wires::GTPFB[i],
                &[
                    format!("GTPDUAL_GTPCLKFBWEST{i}"),
                    format!("GTPDUAL_BOT_GTPCLKFBWEST{i}"),
                ],
            );
        } else {
            builder.wire_names(
                wires::GTPFB[i],
                &[
                    format!("GTPDUAL_GTPCLKFBEAST{ii}", ii = i - 2),
                    format!("GTPDUAL_BOT_GTPCLKFBEAST{ii}", ii = i - 2),
                ],
            );
        }
        builder.extra_name_sub(format!("REGT_GTPFB{i}"), 0, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGT_GTPFB{ii}", ii = i + 4), 2, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGR_GTPFB{i}"), 2, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGR_GTPFB{ii}", ii = i + 4), 1, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGB_GTPFB{i}"), 3, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGB_GTPFB{ii}", ii = i + 4), 1, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGL_GTPFB{i}"), 1, wires::GTPFB[i]);
        builder.extra_name_sub(format!("REGL_GTPFB{ii}", ii = i + 4), 2, wires::GTPFB[i]);
    }
    for i in 0..2 {
        builder.wire_names(
            wires::PLLCLK[i],
            &[
                format!("REGH_LTERM_PLL_CLKOUT{i}"),
                format!("REGH_RTERM_PLL_CLKOUT{i}"),
                format!("REGB_BTERM_PLL_CLKOUT{i}"),
                format!("REGT_TTERM_PLL_CLKOUT{i}"),
                format!("BIOI_INNER_PLLCLK{i}"),
                format!("TIOI_INNER_PLLCLK{i}"),
                format!("TIOI_PLLCLK{i}"),
                format!("IOI_PLLCLK{i}"),
                format!("IOI_PLLCLK{i}_BRK"),
                format!("RIOI_PLLCLK{i}"),
                format!("RIOI_PLLCLK{i}_BRK"),
                format!("GTPDUAL_PLLCLK{i}"),
                format!("GTPDUAL_BOT_PLLCLK{i}"),
                format!("HCLK_MCB_PLLCLKOUT{i}_W"),
            ],
        );
        builder.wire_names(
            wires::PLLCE[i],
            &[
                format!("REGH_LTERM_PLL_CEOUT{i}"),
                format!("REGH_RTERM_PLL_CEOUT{i}"),
                format!("REGB_BTERM_PLL_CEOUT{i}"),
                format!("REGT_TTERM_PLL_CEOUT{i}"),
                format!("BIOI_INNER_PLLCE{i}"),
                format!("TIOI_INNER_PLLCE{i}"),
                format!("TIOI_PLLCE{i}"),
                format!("IOI_PLLCE{i}"),
                format!("IOI_PLLCE{i}_BRK"),
                format!("RIOI_PLLCE{i}"),
                format!("RIOI_PLLCE{i}_BRK"),
                format!("HCLK_MCB_PLLCEOUT{i}_W"),
            ],
        );
    }
    for i in 0..8 {
        builder.extra_name_sub(format!("REGL_CKPIN{i}"), 2, wires::DIVCLK_CLKC[i]);
        builder.extra_name_sub(format!("REGR_CKPIN{i}"), 2, wires::DIVCLK_CLKC[i]);
        builder.extra_name_sub(format!("REGB_CKPIN{i}"), 1, wires::DIVCLK_CLKC[i]);
        builder.extra_name_sub(format!("REGT_CKPIN{i}"), 0, wires::DIVCLK_CLKC[i]);
        builder.extra_name_sub(format!("CLKC_CKLR{i}"), 3, wires::DIVCLK_CLKC[i]);
        builder.extra_name_sub(
            format!("CLKC_CKLR{ii}", ii = i + 8),
            2,
            wires::DIVCLK_CLKC[i],
        );
        builder.extra_name_sub(format!("CLKC_CKTB{i}"), 5, wires::DIVCLK_CLKC[i]);
        builder.extra_name_sub(
            format!("CLKC_CKTB{ii}", ii = i + 8),
            4,
            wires::DIVCLK_CLKC[i],
        );

        builder.extra_name_sub(format!("REGB_CLK_INDIRECT{i}"), 1, wires::DIVCLK_CMT_V[i]);
        builder.extra_name_sub(format!("REGT_CLK_INDIRECT{i}"), 0, wires::DIVCLK_CMT_V[i]);
        builder.extra_name_sub(format!("REGB_CLK_FEEDBACK{i}"), 1, wires::IOFBCLK_CMT_V[i]);
        builder.extra_name_sub(format!("REGT_CLK_FEEDBACK{i}"), 0, wires::IOFBCLK_CMT_V[i]);

        builder.extra_name_sub(
            format!("REGL_CLK_INDIRECT{i}"),
            1 + i / 4,
            wires::DIVCLK_CMT_W[i % 4],
        );
        builder.extra_name_sub(
            format!("REGL_CLK_FEEDBACK{i}"),
            1 + i / 4,
            wires::IOFBCLK_CMT_W[i % 4],
        );
        builder.extra_name_sub(
            format!("REGR_CLK_INDIRECT{i}"),
            2 - i / 4,
            wires::DIVCLK_CMT_E[i % 4],
        );
        builder.extra_name_sub(
            format!("REGR_CLK_FEEDBACK{i}"),
            2 - i / 4,
            wires::IOFBCLK_CMT_E[i % 4],
        );
    }

    for tkn in ["CMT_DCM_BOT", "CMT_DCM2_BOT"] {
        for pref in ["DCM", "DCM2"] {
            for i in 0..8 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_INDIRECT_TB_BOT{i}"),
                    1,
                    wires::DIVCLK_CMT_V[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_FEEDBACK_TB_BOT{i}"),
                    1,
                    wires::IOFBCLK_CMT_V[i],
                );
            }
            for i in 0..4 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_INDIRECT_LR_TOP{i}"),
                    1,
                    wires::DIVCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_FEEDBACK_LR_TOP{i}"),
                    1,
                    wires::IOFBCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_INDIRECT_LR_TOP{ii}", ii = i + 4),
                    1,
                    wires::DIVCLK_CMT_W[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_FEEDBACK_LR_TOP{ii}", ii = i + 4),
                    1,
                    wires::IOFBCLK_CMT_W[i],
                );
            }
        }
    }
    for tkn in ["CMT_DCM_TOP", "CMT_DCM2_TOP"] {
        for pref in ["DCM", "DCM2"] {
            for i in 0..8 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_INDIRECT_LR_TOP{i}"),
                    1,
                    wires::DIVCLK_CMT_V[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_FEEDBACK_LR_TOP{i}"),
                    1,
                    wires::IOFBCLK_CMT_V[i],
                );
            }
            for i in 0..4 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_INDIRECT_TB_BOT{i}"),
                    1,
                    wires::DIVCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_FEEDBACK_TB_BOT{i}"),
                    1,
                    wires::IOFBCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_INDIRECT_TB_BOT{ii}", ii = i + 4),
                    1,
                    wires::DIVCLK_CMT_W[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("{pref}_CLK_FEEDBACK_TB_BOT{ii}", ii = i + 4),
                    1,
                    wires::IOFBCLK_CMT_W[i],
                );
            }
        }
    }
    for tkn in [
        "CMT_PLL_BOT",
        "CMT_PLL1_BOT",
        "CMT_PLL2_BOT",
        "CMT_PLL3_BOT",
    ] {
        for pll in ["PLL", "PLL2"] {
            for i in 0..8 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("CMT_{pll}_CLK_INDIRECT_LRBOT{i}"),
                    1,
                    wires::DIVCLK_CMT_V[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("CMT_{pll}_CLK_FEEDBACK_LRBOT{i}"),
                    1,
                    wires::IOFBCLK_CMT_V[i],
                );
            }
            for i in 0..4 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("PLL_CLK_INDIRECT_TB{i}"),
                    1,
                    wires::DIVCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("PLL_CLK_FEEDBACK_TB{i}"),
                    1,
                    wires::IOFBCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("PLL_CLK_INDIRECT_TB{ii}", ii = i + 4),
                    1,
                    wires::DIVCLK_CMT_W[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("PLL_CLK_FEEDBACK_TB{ii}", ii = i + 4),
                    1,
                    wires::IOFBCLK_CMT_W[i],
                );
            }
        }
    }
    for tkn in ["CMT_PLL_TOP", "CMT_PLL2_TOP", "CMT_PLL3_TOP"] {
        for pll in ["PLL", "PLL2"] {
            for i in 0..8 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("PLL_CLK_INDIRECT_TB{i}"),
                    1,
                    wires::DIVCLK_CMT_V[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("PLL_CLK_FEEDBACK_TB{i}"),
                    1,
                    wires::IOFBCLK_CMT_V[i],
                );
            }
            for i in 0..4 {
                builder.extra_name_tile_sub(
                    tkn,
                    format!("CMT_{pll}_CLK_INDIRECT_LRBOT{i}"),
                    1,
                    wires::DIVCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("CMT_{pll}_CLK_FEEDBACK_LRBOT{i}"),
                    1,
                    wires::IOFBCLK_CMT_E[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("CMT_{pll}_CLK_INDIRECT_LRBOT{ii}", ii = i + 4),
                    1,
                    wires::DIVCLK_CMT_W[i],
                );
                builder.extra_name_tile_sub(
                    tkn,
                    format!("CMT_{pll}_CLK_FEEDBACK_LRBOT{ii}", ii = i + 4),
                    1,
                    wires::IOFBCLK_CMT_W[i],
                );
            }
        }
    }

    for i in 0..16 {
        builder.extra_name_sub(format!("CLKC_GCLK{i}"), 1, wires::IMUX_BUFG[i]);
        builder.extra_name_sub(format!("CLKC_GCLK_MAIN{i}"), 1, wires::GCLK[i]);
        builder.extra_name(format!("CLKV_GCLKH_MAIN{i}_FOLD"), wires::GCLK[i]);

        for tkn in ["CMT_DCM_BOT", "CMT_DCM2_BOT"] {
            builder.extra_name_tile_sub(
                tkn,
                format!("PLL_CLK_CASC_BOT{i}"),
                1,
                wires::CMT_CLKC_I[i],
            );
            builder.extra_name_tile_sub(
                tkn,
                format!("PLL_CLK_CASC_TOP{i}"),
                1,
                wires::CMT_CLKC_O[i],
            );
        }
        for tkn in ["CMT_DCM_TOP", "CMT_DCM2_TOP"] {
            builder.extra_name_tile_sub(
                tkn,
                format!("PLL_CLK_CASC_TOP{i}"),
                1,
                wires::CMT_CLKC_I[i],
            );
            builder.extra_name_tile_sub(
                tkn,
                format!("PLL_CLK_CASC_BOT{i}"),
                1,
                wires::CMT_CLKC_O[i],
            );
        }
        for tkn in [
            "CMT_PLL_BOT",
            "CMT_PLL1_BOT",
            "CMT_PLL2_BOT",
            "CMT_PLL3_BOT",
        ] {
            builder.extra_name_tile_sub(
                tkn,
                format!("CLK_PLLCASC_OUT{i}"),
                1,
                wires::CMT_CLKC_I[i],
            );
            builder.extra_name_tile_sub(
                tkn,
                format!("PLL_CLK_CASC_IN{i}"),
                1,
                wires::CMT_CLKC_O[i],
            );
        }
        for tkn in ["CMT_PLL_TOP", "CMT_PLL2_TOP", "CMT_PLL3_TOP"] {
            builder.extra_name_tile_sub(
                tkn,
                format!("PLL_CLK_CASC_IN{i}"),
                1,
                wires::CMT_CLKC_I[i],
            );
            builder.extra_name_tile_sub(
                tkn,
                format!("CLK_PLLCASC_OUT{i}"),
                1,
                wires::CMT_CLKC_O[i],
            );
        }
        builder.extra_name_sub(format!("CLKC_PLL_L{i}"), 0, wires::CMT_CLKC_I[i]);
        builder.extra_name_sub(format!("CLKC_PLL_U{i}"), 1, wires::CMT_CLKC_I[i]);
        builder.extra_name_sub(format!("REGV_PLL_HCLK{i}"), 0, wires::CMT_OUT[i]);
        builder.extra_name_sub(format!("CMT_PLL_HCLK{i}"), 1, wires::CMT_OUT[i]);
        builder.alt_name_sub(format!("CMT_PLL_HCLK{i}_E"), 1, wires::CMT_OUT[i]);
        builder.extra_name_sub(format!("DCM_HCLK{i}"), 1, wires::CMT_OUT[i]);
        builder.alt_name_sub(format!("DCM_HCLK{i}_N"), 1, wires::CMT_OUT[i]);
        builder.stub_out(format!("CMT_FABRIC_CLK{i}"));
        builder.stub_out(format!("DCM_FABRIC_CLK{i}"));
    }
    builder.stub_out("DCM1_CLK_FROM_BUFG0");
    builder.stub_out("DCM1_CLK_FROM_BUFG1");
    builder.stub_out("DCM1_SE_CLK_IN0");
    builder.stub_out("DCM1_SE_CLK_IN1");
    builder.stub_out("DCM2_CLK_FROM_BUFG0");
    builder.stub_out("DCM2_CLK_FROM_BUFG1");
    builder.stub_out("DCM2_SE_CLK_IN0");
    builder.stub_out("DCM2_SE_CLK_IN1");
    builder.stub_out("CMT_CLK_FROM_BUFG0");
    builder.stub_out("CMT_CLK_FROM_BUFG1");
    builder.stub_out("CMT_CLK_FROM_BUFG2");
    builder.stub_out("CMT_SE_CLKIN0");
    builder.stub_out("CMT_SE_CLKIN1");

    for i in 0..2 {
        builder.wire_names(
            wires::OUT_CLKPAD_I[i],
            &[
                format!("LIOB_IBUF{ii}", ii = 1 - i),
                format!("RIOB_IBUF{ii}", ii = 1 - i),
                format!("BIOB_IBUF{ii}", ii = 1 - i),
                format!("BIOB_IBUF{ii}", ii = i + 2),
                format!("TIOB_IBUF{ii}", ii = 1 - i),
                format!("TIOB_IBUF{ii}", ii = 3 - i),
            ],
        );
    }
    for (wires, pin) in [
        (wires::OUT_CLKPAD_DFB, "DFB"),
        (wires::OUT_CLKPAD_CFB0, "CFB0"),
        (wires::OUT_CLKPAD_CFB1, "CFB1"),
    ] {
        builder.wire_names(
            wires[0],
            &[
                format!("{pin}_ILOGIC_SITE_S"),
                format!("{pin}_ILOGIC_UNUSED_SITE_S"),
            ],
        );
        builder.wire_names(
            wires[1],
            &[
                format!("{pin}_ILOGIC_SITE"),
                format!("{pin}_ILOGIC_UNUSED_SITE"),
            ],
        );
    }
    builder.wire_names(
        wires::OUT_CLKPAD_DQSP,
        &["OUTP_IODELAY_SITE", "OUTP_IODELAY_UNUSED_SITE"],
    );
    builder.wire_names(
        wires::OUT_CLKPAD_DQSN,
        &["OUTN_IODELAY_SITE", "OUTN_IODELAY_UNUSED_SITE"],
    );
    for (wires, name) in [
        (wires::OUT_CLKPAD_I, "CLKPIN"),
        (wires::OUT_CLKPAD_CFB0, "CFB"),
        (wires::OUT_CLKPAD_CFB1, "CFB1_"),
        (wires::OUT_CLKPAD_DFB, "DFB"),
    ] {
        for i in 0..8 {
            builder.extra_name_sub(
                format!("REGL_{name}{i}"),
                if i < 4 { i / 2 } else { i / 2 + 2 },
                wires[1 - i % 2],
            );
            builder.extra_name_sub(
                format!("REGR_{name}{i}"),
                if i < 4 { 5 - i / 2 } else { 3 - i / 2 },
                wires[1 - i % 2],
            );
            builder.extra_name_sub(format!("REGB_{name}{i}"), 3 - i / 2, wires[1 - i % 2]);
            builder.extra_name_sub(format!("REGT_{name}{i}"), i / 2, wires[1 - i % 2]);
        }
    }
    for (wire, name) in [
        (wires::OUT_CLKPAD_DQSP, "DQSP"),
        (wires::OUT_CLKPAD_DQSN, "DQSN"),
    ] {
        for i in 0..4 {
            builder.extra_name_sub(
                format!("REGL_{name}{i}"),
                if i < 2 { i } else { i + 2 },
                wire,
            );
            builder.extra_name_sub(
                format!("REGR_{name}{i}"),
                if i < 2 { 5 - i } else { 3 - i },
                wire,
            );
            builder.extra_name_sub(format!("REGB_{name}{i}"), 3 - i, wire);
            builder.extra_name_sub(format!("REGT_{name}{i}"), i, wire);
        }
    }

    for i in 0..2 {
        for (lr, ci) in [('L', 2), ('R', 3)] {
            builder.extra_name_sub(
                format!("REGC_CLKPLL_IO_{lr}T{i}"),
                ci,
                wires::CMT_BUFPLL_H_CLKOUT[i],
            );
            builder.extra_name_sub(
                format!("CLK_PLL_LOCK_{lr}T{i}"),
                ci,
                wires::CMT_BUFPLL_H_LOCKED[i],
            );
            builder.extra_name_sub(
                format!("REG{lr}_CLKPLL{i}"),
                2,
                wires::CMT_BUFPLL_H_CLKOUT[i],
            );
            builder.extra_name_sub(
                format!("REG{lr}_LOCKED{i}"),
                2,
                wires::CMT_BUFPLL_H_LOCKED[i],
            );
        }
    }
    for i in 0..3 {
        builder.extra_name_sub(
            format!("PLL_LOCK_BOT{i}"),
            1,
            wires::CMT_BUFPLL_V_LOCKED_S[i],
        );
        builder.extra_name_sub(
            format!("PLL_LOCK_TOP{i}"),
            1,
            wires::CMT_BUFPLL_V_LOCKED_N[i],
        );
    }
    for i in 0..4 {
        builder.extra_name_sub(
            format!("REGC_PLLCLK_DN_IN{i}"),
            1,
            wires::CMT_BUFPLL_V_CLKOUT_S[i],
        );
        builder.extra_name_sub(
            format!("REGC_PLLCLK_UP_IN{i}"),
            1,
            wires::CMT_BUFPLL_V_CLKOUT_N[i],
        );
    }
    for i in 0..2 {
        builder.extra_name_sub(
            format!("REGC_PLLCLK_DN_OUT{i}"),
            1,
            wires::CMT_BUFPLL_V_CLKOUT_S[i + 4],
        );
        builder.extra_name_sub(
            format!("REGC_PLLCLK_UP_OUT{i}"),
            1,
            wires::CMT_BUFPLL_V_CLKOUT_N[i + 4],
        );
    }

    for i in 0..3 {
        builder.extra_name_sub(
            format!("REGB_LOCKIN{i}"),
            1,
            wires::CMT_BUFPLL_V_LOCKED_N[i],
        );
        builder.extra_name_sub(
            format!("REGT_LOCKIN{i}"),
            0,
            wires::CMT_BUFPLL_V_LOCKED_S[i],
        );
    }
    for i in 0..6 {
        builder.extra_name_sub(
            format!("REGB_PLL_IOCLK_DOWN{i}"),
            1,
            wires::CMT_BUFPLL_V_CLKOUT_N[i],
        );
        builder.extra_name_sub(
            format!("REGT_PLL_IOCLK_UP{i}"),
            0,
            wires::CMT_BUFPLL_V_CLKOUT_S[i],
        );
    }

    for kind in ["DCM", "PLL", "PLL2"] {
        for i in 0..6 {
            builder.extra_name(
                format!("{kind}_IOCLK_DOWN{i}"),
                wires::CMT_BUFPLL_V_CLKOUT_S[i],
            );
            builder.extra_name(
                format!("{kind}_IOCLK_DN{i}"),
                wires::CMT_BUFPLL_V_CLKOUT_S[i],
            );
            builder.extra_name(
                format!("{kind}_IOCLK_UP{i}"),
                wires::CMT_BUFPLL_V_CLKOUT_N[i],
            );
        }
        for i in 0..3 {
            builder.extra_name(
                format!("CMT_{kind}_LOCK_DN{i}"),
                wires::CMT_BUFPLL_V_LOCKED_S[i],
            );
            builder.extra_name(
                format!("CMT_{kind}_LOCK_UP{i}"),
                wires::CMT_BUFPLL_V_LOCKED_N[i],
            );
        }
    }
    builder.extra_name_sub("PLL_LOCKED", 1, wires::OUT_PLL_LOCKED);
    builder.extra_name_sub("CMT_PLL_LOCKED", 1, wires::OUT_PLL_LOCKED);
    for i in 0..2 {
        builder.extra_name_sub(format!("PLLCASC_CLKOUT{i}"), 1, wires::OUT_PLL_CLKOUT[i]);
    }

    for i in 0..6 {
        builder.extra_name_sub(format!("CMT_PLL_CLKOUT{i}"), 1, wires::OUT_PLL_CLKOUT[i]);
        builder.extra_name_sub(
            format!("CMT_PLL_CLKOUTDCM{i}"),
            1,
            wires::OUT_PLL_CLKOUTDCM[i],
        );
    }
    builder.extra_name_sub("CMT_CLKFB", 1, wires::OUT_PLL_CLKFBOUT);
    builder.extra_name_sub("CMT_PLL_CLKFBDCM", 1, wires::OUT_PLL_CLKFBDCM);
    builder.alt_name_sub("CMT_PLL_CLKFBDCM_TEST", 1, wires::OUT_PLL_CLKFBDCM);
    builder.extra_name_sub("CMT_CLKMUX_CLKFB", 1, wires::IMUX_PLL_CLKFB);
    builder.alt_name_sub("CMT_CLKMUX_CLKFB_TEST", 1, wires::IMUX_PLL_CLKFB);
    builder.extra_name_sub("CMT_CLKMUX_CLKREF", 1, wires::IMUX_PLL_CLKIN1);
    builder.extra_name_sub("CMT_CLKMUX_CLKIN2", 1, wires::IMUX_PLL_CLKIN2);
    builder.extra_name_sub("CMT_CLKMUX_CLKREF_TEST", 1, wires::TEST_PLL_CLKIN);
    builder.extra_name_sub("CMT_TEST_CLK", 1, wires::CMT_TEST_CLK);
    builder.alt_name_sub("CMT_SE_CLK_OUT", 1, wires::CMT_TEST_CLK);
    for (i, (wires, name)) in [
        (wires::OUT_DCM_CLK0, "CLK0"),
        (wires::OUT_DCM_CLK90, "CLK90"),
        (wires::OUT_DCM_CLK180, "CLK180"),
        (wires::OUT_DCM_CLK270, "CLK270"),
        (wires::OUT_DCM_CLK2X, "CLK2X"),
        (wires::OUT_DCM_CLK2X180, "CLK2X180"),
        (wires::OUT_DCM_CLKDV, "CLKDV"),
        (wires::OUT_DCM_CLKFX, "CLKFX"),
        (wires::OUT_DCM_CLKFX180, "CLKFX180"),
        (wires::OUT_DCM_CONCUR, "CONCUR"),
    ]
    .into_iter()
    .enumerate()
    {
        builder.extra_name_sub(format!("DCM2_CLKOUT{i}"), 1, wires[0]);
        builder.extra_name_sub(format!("DCM1_CLKOUT{i}"), 1, wires[1]);
        builder.extra_name_sub(format!("DCM2_{name}"), 1, wires[0]);
        builder.extra_name_sub(format!("DCM1_{name}"), 1, wires[1]);
        builder.alt_name_sub(format!("DCM2_{name}_TEST"), 1, wires[0]);
        builder.alt_name_sub(format!("DCM1_{name}_TEST"), 1, wires[1]);
    }
    builder.extra_name_sub("DCM2_CLKIN", 1, wires::IMUX_DCM_CLKIN[0]);
    builder.extra_name_sub("DCM1_CLKIN", 1, wires::IMUX_DCM_CLKIN[1]);
    builder.extra_name_sub("DCM2_CLKFB", 1, wires::IMUX_DCM_CLKFB[0]);
    builder.extra_name_sub("DCM1_CLKFB", 1, wires::IMUX_DCM_CLKFB[1]);
    builder.extra_name_sub("DCM2_CLK_TO_PLL", 1, wires::OMUX_DCM_SKEWCLKIN1[0]);
    builder.extra_name_sub("DCM1_CLK_TO_PLL", 1, wires::OMUX_DCM_SKEWCLKIN1[1]);
    builder.extra_name_sub("DCM_0_TESTCLK_PINWIRE", 1, wires::OMUX_DCM_SKEWCLKIN2[0]);
    builder.extra_name_sub("DCM_1_TESTCLK_PINWIRE", 1, wires::OMUX_DCM_SKEWCLKIN2[1]);
    builder.extra_name_sub("DCM2_CLK_FROM_PLL", 1, wires::OMUX_PLL_SKEWCLKIN2_BUF);
    builder.extra_name_sub("DCM1_CLK_FROM_PLL", 1, wires::OMUX_PLL_SKEWCLKIN1_BUF);

    builder.extra_name_sub("CMT_CLK_FROM_DCM2", 2, wires::OMUX_DCM_SKEWCLKIN1[0]);
    builder.extra_name_sub("CMT_CLK_FROM_DCM1", 2, wires::OMUX_DCM_SKEWCLKIN1[1]);
    builder.extra_name_sub("CMT_CLK_TO_DCM2", 1, wires::OMUX_PLL_SKEWCLKIN2);
    builder.extra_name_sub("CMT_CLK_TO_DCM1", 1, wires::OMUX_PLL_SKEWCLKIN1);
    builder.extra_name_sub("CMT_DCM2_CLKIN", 2, wires::IMUX_DCM_CLKIN[0]);
    builder.extra_name_sub("CMT_DCM1_CLKIN", 2, wires::IMUX_DCM_CLKIN[1]);
    builder.extra_name_sub("CMT_DCM2_CLKFB", 2, wires::IMUX_DCM_CLKFB[0]);
    builder.extra_name_sub("CMT_DCM1_CLKFB", 2, wires::IMUX_DCM_CLKFB[1]);

    builder.extract_int_id(tcls::INT, bslots::INT, "INT", "INT", &[]);
    builder.extract_int_id(tcls::INT, bslots::INT, "INT_BRK", "INT_BRK", &[]);
    builder.extract_int_id(tcls::INT, bslots::INT, "INT_BRAM", "INT", &[]);
    builder.extract_int_id(tcls::INT, bslots::INT, "INT_BRAM_BRK", "INT_BRK", &[]);
    builder.extract_int_id(tcls::INT, bslots::INT, "INT_GCLK", "INT", &[]);
    builder.extract_int_id(tcls::INT, bslots::INT, "INT_TERM", "INT_TERM", &[]);
    builder.extract_int_id(tcls::INT, bslots::INT, "INT_TERM_BRK", "INT_TERM_BRK", &[]);
    builder.extract_int_id(tcls::INT_IOI, bslots::INT, "IOI_INT", "INT_IOI", &[]);
    builder.extract_int_id(tcls::INT_IOI, bslots::INT, "LIOI_INT", "INT_IOI", &[]);
    builder.extract_int_id(
        tcls::INT_IOI,
        bslots::INT,
        "LIOI_INT_BRK",
        "INT_IOI_BRK",
        &[],
    );

    for tkn in [
        "CNR_TL_LTERM",
        "IOI_LTERM",
        "IOI_LTERM_LOWER_BOT",
        "IOI_LTERM_LOWER_TOP",
        "IOI_LTERM_UPPER_BOT",
        "IOI_LTERM_UPPER_TOP",
    ] {
        builder.extract_term_buf_id(defs::ccls::TERM_W, Dir::W, tkn, "TERM_W", &[]);
    }
    builder.extract_term_buf_id(
        defs::ccls::TERM_W,
        Dir::W,
        "INT_INTERFACE_LTERM",
        "TERM_W_INTF",
        &[],
    );

    for &term_xy in rd.tiles_by_kind_name("INT_LTERM") {
        let int_xy = builder.walk_to_int(term_xy, Dir::E, false).unwrap();
        // sigh.
        if int_xy.x == term_xy.x + 3 {
            continue;
        }
        builder.extract_term_buf_tile_id(
            defs::ccls::TERM_W,
            Dir::W,
            term_xy,
            "TERM_W_INTF",
            int_xy,
            &[],
        );
    }
    for tkn in [
        "CNR_TL_RTERM",
        "IOI_RTERM",
        "IOI_RTERM_LOWER_BOT",
        "IOI_RTERM_LOWER_TOP",
        "IOI_RTERM_UPPER_BOT",
        "IOI_RTERM_UPPER_TOP",
    ] {
        builder.extract_term_buf_id(defs::ccls::TERM_E, Dir::E, tkn, "TERM_E", &[]);
    }
    for tkn in ["INT_RTERM", "INT_INTERFACE_RTERM"] {
        builder.extract_term_buf_id(defs::ccls::TERM_E, Dir::E, tkn, "TERM_E_INTF", &[]);
    }
    for tkn in [
        "CNR_BR_BTERM",
        "IOI_BTERM",
        "IOI_BTERM_BUFPLL",
        "CLB_INT_BTERM",
        "DSP_INT_BTERM",
        // NOTE: RAMB_BOT_BTERM is *not* a terminator — it's empty
    ] {
        builder.extract_term_buf_id(defs::ccls::TERM_S, Dir::S, tkn, "TERM_S", &[]);
    }
    for tkn in [
        "CNR_TR_TTERM",
        "IOI_TTERM",
        "IOI_TTERM_BUFPLL",
        "DSP_INT_TTERM",
        "RAMB_TOP_TTERM",
    ] {
        builder.extract_term_buf_id(defs::ccls::TERM_N, Dir::N, tkn, "TERM_N", &[]);
    }

    for (dir, tkn, naming) in [
        (Dir::E, "INT_INTERFACE", "INTF"),
        (Dir::E, "INT_INTERFACE_REGC", "INTF_REGC"),
        (Dir::W, "INT_INTERFACE_LTERM", "INTF_LTERM"),
        (Dir::E, "INT_INTERFACE_RTERM", "INTF_RTERM"),
        (Dir::E, "LL", "INTF_CNR"),
        (Dir::E, "UL", "INTF_CNR"),
        (Dir::E, "LR_LOWER", "INTF_CNR"),
        (Dir::E, "LR_UPPER", "INTF_CNR"),
        (Dir::E, "UR_LOWER", "INTF_CNR"),
        (Dir::E, "UR_UPPER", "INTF_CNR"),
    ] {
        builder.extract_intf_id(
            tcls::INTF,
            dir,
            tkn,
            naming,
            bslots::INTF_TESTMUX,
            Some(bslots::INTF_INT),
            true,
            false,
        );
    }
    builder.extract_intf_id(
        tcls::INTF_CMT,
        Dir::E,
        "INT_INTERFACE_CARRY",
        "INTF",
        bslots::INTF_TESTMUX,
        Some(bslots::INTF_INT),
        true,
        false,
    );
    for tkn in ["INT_INTERFACE_IOI", "INT_INTERFACE_IOI_DCMBOT"] {
        builder.extract_intf_id(
            tcls::INTF_CMT_IOI,
            Dir::E,
            tkn,
            "INTF",
            bslots::INTF_TESTMUX,
            Some(bslots::INTF_INT),
            true,
            false,
        );
    }
    for tkn in [
        "LIOI",
        "LIOI_BRK",
        "RIOI",
        "RIOI_BRK",
        "TIOI_INNER",
        "TIOI_OUTER",
        "BIOI_INNER",
        "BIOI_OUTER",
    ] {
        builder.extract_intf_id(
            tcls::INTF_IOI,
            Dir::E,
            tkn,
            "INTF_IOI",
            bslots::INTF_TESTMUX,
            Some(bslots::INTF_INT),
            true,
            false,
        );
    }

    for (tcid, tkn) in [(tcls::CLEXL, "CLEXL"), (tcls::CLEXM, "CLEXM")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bels = [
                builder
                    .bel_xy(bslots::SLICE[0], "SLICE", 0, 0)
                    .pins_name_only(&["CIN"])
                    .pin_name_only("COUT", 1),
                builder.bel_xy(bslots::SLICE[1], "SLICE", 1, 0),
            ];
            builder
                .xtile_id(tcid, tkn, xy)
                .num_cells(1)
                .bels(bels)
                .ref_int(xy.delta(-1, 0), 0)
                .extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAMSITE2").iter().next() {
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_tile_class_naming("INTF");
        for dy in 0..4 {
            intf_xy.push((xy.delta(-1, dy), n));
        }
        let mut bels = vec![];
        for i in 0..2 {
            let mut bel = builder.bel_xy(bslots::BRAM[i], "RAMB8", 0, i).manual();
            for (ab, rw) in [('A', "WR"), ('B', "RD")] {
                for j in 0..13 {
                    bel = bel.pin_rename(format!("ADDR{ab}{rw}ADDR{j}"), format!("ADDR{ab}{j}"));
                }
                for j in 0..16 {
                    bel = bel
                        .pin_rename(format!("DI{ab}DI{j}"), format!("DI{ab}{j}"))
                        .pin_rename(format!("DO{ab}DO{j}"), format!("DO{ab}{j}"));
                }
                for j in 0..2 {
                    bel = bel
                        .pin_rename(format!("DIP{ab}DIP{j}"), format!("DIP{ab}{j}"))
                        .pin_rename(format!("DOP{ab}DOP{j}"), format!("DOP{ab}{j}"));
                }
                for pin in ["CLK", "EN"] {
                    bel = bel.pin_rename(format!("{pin}{ab}{rw}{pin}"), format!("{pin}{ab}"));
                }
            }
            bel = bel
                .pin_rename("WEAWEL0", "WEA0")
                .pin_rename("WEAWEL1", "WEA1")
                .pin_rename("WEBWEU0", "WEB0")
                .pin_rename("WEBWEU1", "WEB1")
                .pin_rename("RSTBRST", "RSTB")
                .pin_rename("REGCEBREGCE", "REGCEB");
            if i == 0 {
                bel = bel.sub_xy(rd, "RAMB16", 0, 0);
                for ab in ['A', 'B'] {
                    for j in 0..14 {
                        bel = bel.pin_rename(format!("ADDR{ab}{j}"), format!("RAMB16_ADDR{ab}{j}"));
                    }
                    for j in 0..32 {
                        bel = bel
                            .pin_rename(format!("DI{ab}{j}"), format!("RAMB16_DI{ab}{j}"))
                            .pin_rename(format!("DO{ab}{j}"), format!("RAMB16_DO{ab}{j}"));
                    }
                    for j in 0..4 {
                        bel = bel
                            .pin_rename(format!("DIP{ab}{j}"), format!("RAMB16_DIP{ab}{j}"))
                            .pin_rename(format!("DOP{ab}{j}"), format!("RAMB16_DOP{ab}{j}"))
                            .pin_rename(format!("WE{ab}{j}"), format!("RAMB16_WE{ab}{j}"));
                    }
                    for pin in ["CLK", "EN", "REGCE", "RST"] {
                        bel = bel.pin_rename(format!("{pin}{ab}"), format!("RAMB16_{pin}{ab}"));
                    }
                }
            }
            bels.push(bel);
        }
        let mut xt = builder
            .xtile_id(tcls::BRAM, "BRAM", xy)
            .num_cells(4)
            .bels(bels);
        for (i, &(xy, naming)) in intf_xy.iter().enumerate() {
            xt = xt.ref_single(xy, i, naming);
        }
        let mut xt = xt.extract();
        for ab in ['A', 'B'] {
            for i in 0..14 {
                let pin = xt.bels[0]
                    .0
                    .pins
                    .remove(&format!("RAMB16_ADDR{ab}{i}"))
                    .unwrap();
                assert_eq!(
                    pin,
                    xt.bels[i / 13].0.pins[&format!("ADDR{ab}{ii}", ii = i % 13)]
                );
            }
            for i in 0..32 {
                for pn in ["DI", "DO"] {
                    let pin = xt.bels[0]
                        .0
                        .pins
                        .remove(&format!("RAMB16_{pn}{ab}{i}"))
                        .unwrap();
                    assert_eq!(
                        pin,
                        xt.bels[i / 16].0.pins[&format!("{pn}{ab}{ii}", ii = i % 16)]
                    );
                }
            }
            for i in 0..4 {
                for pn in ["DIP", "DOP", "WE"] {
                    let pin = xt.bels[0]
                        .0
                        .pins
                        .remove(&format!("RAMB16_{pn}{ab}{i}"))
                        .unwrap();
                    assert_eq!(
                        pin,
                        xt.bels[i / 2].0.pins[&format!("{pn}{ab}{ii}", ii = i % 2)]
                    );
                }
            }
            for pn in ["CLK", "EN", "REGCE", "RST"] {
                let pin = xt.bels[0]
                    .0
                    .pins
                    .remove(&format!("RAMB16_{pn}{ab}"))
                    .unwrap();
                assert_eq!(pin, xt.bels[0].0.pins[&format!("{pn}{ab}")]);
            }
        }
        for (i, (bel, naming)) in xt.bels.into_iter().enumerate() {
            builder.insert_tcls_bel(tcls::BRAM, bslots::BRAM[i], BelInfo::Legacy(bel));
            builder.insert_bel_naming("BRAM", bslots::BRAM[i], naming);
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("MACCSITE2").iter().next() {
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_tile_class_naming("INTF");
        for dy in 0..4 {
            intf_xy.push((xy.delta(-1, dy), n));
        }
        let mut bel_dsp = builder
            .bel_xy(bslots::DSP, "DSP48", 0, 0)
            .pin_name_only("CARRYIN", 0)
            .pin_name_only("CARRYOUT", 1);
        for i in 0..18 {
            bel_dsp = bel_dsp.pin_name_only(&format!("BCIN{i}"), 0);
            bel_dsp = bel_dsp.pin_name_only(&format!("BCOUT{i}"), 1);
        }
        for i in 0..48 {
            bel_dsp = bel_dsp.pin_name_only(&format!("PCIN{i}"), 0);
            bel_dsp = bel_dsp.pin_name_only(&format!("PCOUT{i}"), 1);
        }
        builder.extract_xtile_bels_intf_id(tcls::DSP, xy, &[], &[], &intf_xy, "DSP", &[bel_dsp]);
    }

    let intf_cnr = builder.ndb.get_tile_class_naming("INTF_CNR");
    for (tcid, naming, tkn, bels) in [
        (
            tcls::CNR_SW,
            "CNR_SW",
            "LL",
            vec![
                builder.bel_xy(bslots::OCT_CAL[2], "OCT_CAL", 0, 0),
                builder.bel_xy(bslots::OCT_CAL[3], "OCT_CAL", 0, 1),
                builder.bel_virtual(bslots::MISR_CNR_H),
                builder.bel_virtual(bslots::MISR_CNR_V),
                builder.bel_virtual(bslots::MISC_SW),
                builder.bel_virtual(bslots::BANK[2]),
                builder.bel_virtual(bslots::BANK[3]),
            ],
        ),
        (
            tcls::CNR_NW,
            "CNR_NW",
            "UL",
            vec![
                builder.bel_xy(bslots::OCT_CAL[0], "OCT_CAL", 0, 0),
                builder.bel_xy(bslots::OCT_CAL[4], "OCT_CAL", 0, 1),
                builder.bel_single(bslots::PMV, "PMV"),
                builder.bel_single(bslots::DNA_PORT, "DNA_PORT"),
                builder.bel_virtual(bslots::MISR_CNR_H),
                builder.bel_virtual(bslots::MISR_CNR_V),
                builder.bel_virtual(bslots::MISC_NW),
                builder.bel_virtual(bslots::BANK[0]),
                builder.bel_virtual(bslots::BANK[4]),
            ],
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcid, naming, xy)
                .ref_single(xy, 0, intf_cnr)
                .bels(bels)
                .extract();
        }
    }
    if let Some(&xy) = rd.tiles_by_kind_name("LR_LOWER").iter().next() {
        let bels = vec![
            builder.bel_xy(bslots::OCT_CAL[1], "OCT_CAL", 0, 0),
            builder.bel_xy(bslots::ICAP, "ICAP", 0, 0),
            builder.bel_single(bslots::SPI_ACCESS, "SPI_ACCESS"),
            builder
                .bel_single(bslots::SUSPEND_SYNC, "SUSPEND_SYNC")
                .raw_tile(1),
            builder
                .bel_single(bslots::POST_CRC_INTERNAL, "POST_CRC_INTERNAL")
                .raw_tile(1),
            builder.bel_single(bslots::STARTUP, "STARTUP").raw_tile(1),
            builder
                .bel_single(bslots::SLAVE_SPI, "SLAVE_SPI")
                .raw_tile(1),
            builder.bel_virtual(bslots::MISR_CNR_H),
            builder.bel_virtual(bslots::MISR_CNR_V),
            builder.bel_virtual(bslots::MISC_SE),
            builder.bel_virtual(bslots::BANK[1]),
        ];
        builder
            .xtile_id(tcls::CNR_SE, "CNR_SE", xy)
            .num_cells(2)
            .raw_tile(xy.delta(0, 1))
            .ref_single(xy, 0, intf_cnr)
            .ref_single(xy.delta(0, 1), 1, intf_cnr)
            .bels(bels)
            .extract();
    }
    if let Some(&xy) = rd.tiles_by_kind_name("UR_LOWER").iter().next() {
        let bels = vec![
            builder.bel_xy(bslots::OCT_CAL[5], "OCT_CAL", 0, 0),
            builder.bel_xy(bslots::BSCAN[0], "BSCAN", 0, 0).raw_tile(1),
            builder.bel_xy(bslots::BSCAN[1], "BSCAN", 0, 1).raw_tile(1),
            builder.bel_xy(bslots::BSCAN[2], "BSCAN", 0, 0),
            builder.bel_xy(bslots::BSCAN[3], "BSCAN", 0, 1),
            builder.bel_virtual(bslots::MISR_CNR_H),
            builder.bel_virtual(bslots::MISR_CNR_V),
            builder.bel_virtual(bslots::MISC_NE),
            builder.bel_virtual(bslots::BANK[5]),
        ];
        builder
            .xtile_id(tcls::CNR_NE, "CNR_NE", xy)
            .num_cells(2)
            .raw_tile(xy.delta(0, 1))
            .ref_single(xy, 0, intf_cnr)
            .ref_single(xy.delta(0, 1), 1, intf_cnr)
            .bels(bels)
            .extract();
    }

    let intf_ioi = builder.ndb.get_tile_class_naming("INTF_IOI");
    for (tcid, tkn) in [
        (tcls::IOI_WE, "LIOI"),
        (tcls::IOI_WE, "LIOI_BRK"),
        (tcls::IOI_WE, "RIOI"),
        (tcls::IOI_WE, "RIOI_BRK"),
        (tcls::IOI_SN, "BIOI_INNER"),
        (tcls::IOI_SN, "BIOI_OUTER"),
        (tcls::IOI_SN, "TIOI_INNER"),
        (tcls::IOI_SN, "TIOI_OUTER"),
        (tcls::IOI_SN, "BIOI_INNER_UNUSED"),
        (tcls::IOI_SN, "BIOI_OUTER_UNUSED"),
        (tcls::IOI_SN, "TIOI_INNER_UNUSED"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let ms = match i {
                    0 => 'S',
                    1 => 'M',
                    _ => unreachable!(),
                };
                let mut bel = builder
                    .bel_xy(bslots::ILOGIC[i], "ILOGIC", 0, i)
                    .pins_name_only(&[
                        "D", "DDLY", "DDLY2", "CLK0", "CLK1", "IOCE", "OFB", "TFB", "SHIFTIN",
                        "SHIFTOUT", "SR",
                    ])
                    .extra_int_in(
                        "SR_INT",
                        &[if i == 0 {
                            "IOI_LOGICINB20"
                        } else {
                            "IOI_LOGICINB36"
                        }],
                    )
                    .extra_wire("MCB_FABRICOUT", &[format!("IOI_MCB_INBYP_{ms}")])
                    .extra_wire(
                        "IOB_I",
                        &[
                            format!("BIOI_INNER_IBUF{}", i ^ 1),
                            format!("BIOI_OUTER_IBUF{}", i ^ 1),
                            format!("TIOI_INNER_IBUF{}", i ^ 1),
                            format!("TIOI_OUTER_IBUF{}", i ^ 1),
                            format!("LIOI_IOB_IBUF{}", i ^ 1),
                            format!("RIOI_IOB_IBUF{}", i ^ 1),
                        ],
                    )
                    .extra_wire(
                        "D_MUX",
                        &[
                            if i == 0 {
                                "D_ILOGIC_IDATAIN_IODELAY_S"
                            } else {
                                "D_ILOGIC_IDATAIN_IODELAY"
                            },
                            if i == 0 {
                                "D_ILOGIC_IDATAIN_IODELAY_UNUSED_S"
                            } else {
                                "D_ILOGIC_IDATAIN_IODELAY_UNUSED"
                            },
                        ],
                    );
                if i == 0 {
                    bel = bel.pins_name_only(&["INCDEC", "VALID"]);
                }
                bels.push(bel);
            }
            for i in 0..2 {
                let ms = match i {
                    0 => 'S',
                    1 => 'M',
                    _ => unreachable!(),
                };
                let bel = builder
                    .bel_xy(bslots::OLOGIC[i], "OLOGIC", 0, i)
                    .pins_name_only(&[
                        "CLK0",
                        "CLK1",
                        "IOCE",
                        "SHIFTIN1",
                        "SHIFTIN2",
                        "SHIFTIN3",
                        "SHIFTIN4",
                        "SHIFTOUT1",
                        "SHIFTOUT2",
                        "SHIFTOUT3",
                        "SHIFTOUT4",
                        "OQ",
                        "TQ",
                    ])
                    .extra_wire(
                        "IOB_O",
                        &[
                            format!("BIOI_INNER_O{}", i ^ 1),
                            format!("BIOI_OUTER_O{}", i ^ 1),
                            format!("TIOI_INNER_O{}", i ^ 1),
                            format!("TIOI_OUTER_O{}", i ^ 1),
                            format!("LIOI_IOB_O{}", i ^ 1),
                            format!("RIOI_IOB_O{}", i ^ 1),
                        ],
                    )
                    .extra_wire(
                        "IOB_T",
                        &[
                            format!("BIOI_INNER_T{}", i ^ 1),
                            format!("BIOI_OUTER_T{}", i ^ 1),
                            format!("TIOI_INNER_T{}", i ^ 1),
                            format!("TIOI_OUTER_T{}", i ^ 1),
                            format!("LIOI_IOB_T{}", i ^ 1),
                            format!("RIOI_IOB_T{}", i ^ 1),
                        ],
                    )
                    .extra_wire("MCB_D1", &[format!("IOI_MCB_OUTP_{ms}")])
                    .extra_wire("MCB_D2", &[format!("IOI_MCB_OUTN_{ms}")]);
                bels.push(bel);
            }
            for i in 0..2 {
                let ms = match i {
                    0 => 'S',
                    1 => 'M',
                    _ => unreachable!(),
                };
                let mut bel = builder
                    .bel_xy(bslots::IODELAY[i], "IODELAY", 0, i)
                    .pins_name_only(&[
                        "IOCLK0",
                        "IOCLK1",
                        "IDATAIN",
                        "ODATAIN",
                        "T",
                        "TOUT",
                        "DOUT",
                        "DATAOUT",
                        "DATAOUT2",
                        "AUXSDO",
                        "AUXSDOIN",
                        "AUXADDR0",
                        "AUXADDR1",
                        "AUXADDR2",
                        "AUXADDR3",
                        "AUXADDR4",
                        "READEN",
                        "MEMUPDATE",
                    ])
                    .extra_wire("MCB_DQSOUTP", &[format!("IOI_MCB_IN_{ms}")])
                    .extra_wire_force("MCB_AUXADDR0", format!("AUXADDR0_MCBTOIO_{ms}"))
                    .extra_wire_force("MCB_AUXADDR1", format!("AUXADDR1_MCBTOIO_{ms}"))
                    .extra_wire_force("MCB_AUXADDR2", format!("AUXADDR2_MCBTOIO_{ms}"))
                    .extra_wire_force("MCB_AUXADDR3", format!("AUXADDR3_MCBTOIO_{ms}"))
                    .extra_wire_force("MCB_AUXADDR4", format!("AUXADDR4_MCBTOIO_{ms}"))
                    .extra_wire_force("MCB_AUXSDOIN", format!("AUXSDOIN_MCBTOIO_{ms}"))
                    .extra_wire_force("MCB_AUXSDO", format!("AUXSDO_IOTOMCB_{ms}"))
                    .extra_wire_force("MCB_MEMUPDATE", format!("MEMUPDATE_MCBTOIO_{ms}"));
                if i == 0 {
                    bel = bel.pins_name_only(&["DQSOUTP", "DQSOUTN"]);
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy(bslots::TIEOFF_IOI, "TIEOFF", 0, 0)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
            );
            for i in 0..2 {
                let ms = match i {
                    0 => 'S',
                    1 => 'M',
                    _ => unreachable!(),
                };
                let bel = builder
                    .bel_virtual(bslots::IOICLK[i])
                    .extra_wire("CLK0INTER", &[format!("IOI_CLK0INTER_{ms}")])
                    .extra_wire("CLK1INTER", &[format!("IOI_CLK1INTER_{ms}")])
                    .extra_wire("CLK2INTER", &[format!("IOI_CLK2INTER_{ms}")])
                    .extra_int_in("CKINT0", &[format!("IOI_CLK{}", i ^ 1)])
                    .extra_int_in("CKINT1", &[format!("IOI_GFAN{}", i ^ 1)])
                    .extra_wire("CLK0_ILOGIC", &[format!("IOI_CLKDIST_CLK0_ILOGIC_{ms}")])
                    .extra_wire("CLK0_OLOGIC", &[format!("IOI_CLKDIST_CLK0_OLOGIC_{ms}")])
                    .extra_wire("CLK1", &[format!("IOI_CLKDIST_CLK1_{ms}")])
                    .extra_wire("IOCE0", &[format!("IOI_CLKDIST_IOCE0_{ms}")])
                    .extra_wire("IOCE1", &[format!("IOI_CLKDIST_IOCE1_{ms}")]);
                bels.push(bel);
            }
            let mut bel_ioi = builder
                .bel_virtual(bslots::IOI)
                .extra_wire("MCB_DRPADD", &["IOI_MCB_DRPADD"])
                .extra_wire("MCB_DRPBROADCAST", &["IOI_MCB_DRPBROADCAST"])
                .extra_wire("MCB_DRPCLK", &["IOI_MCB_DRPCLK"])
                .extra_wire("MCB_DRPCS", &["IOI_MCB_DRPCS"])
                .extra_wire("MCB_DRPSDI", &["IOI_MCB_DRPSDI"])
                .extra_wire("MCB_DRPSDO", &["IOI_MCB_DRPSDO"])
                .extra_wire("MCB_DRPTRAIN", &["IOI_MCB_DRPTRAIN"])
                .extra_wire("MCB_T1", &["IOI_MCB_DQIEN_S"])
                .extra_wire("MCB_T2", &["IOI_MCB_DQIEN_M"])
                .extra_wire("PCI_CE", &["IOI_PCI_CE"]);
            for i in 0..4 {
                bel_ioi = bel_ioi
                    .extra_int_in(
                        format!("IOCLK{i}"),
                        &[
                            format!("BIOI_INNER_IOCLK{i}"),
                            format!("TIOI_INNER_IOCLK{i}"),
                            format!("TIOI_IOCLK{i}"),
                            format!("IOI_IOCLK{i}"),
                            format!("IOI_IOCLK{i}_BRK"),
                            format!("RIOI_IOCLK{i}"),
                            format!("RIOI_IOCLK{i}_BRK"),
                        ],
                    )
                    .extra_int_in(
                        format!("IOCE{i}"),
                        &[
                            format!("BIOI_INNER_IOCE{i}"),
                            format!("TIOI_INNER_IOCE{i}"),
                            format!("TIOI_IOCE{i}"),
                            format!("IOI_IOCE{i}"),
                            format!("IOI_IOCE{i}_BRK"),
                            format!("RIOI_IOCE{i}"),
                            format!("RIOI_IOCE{i}_BRK"),
                        ],
                    );
            }
            for i in 0..2 {
                bel_ioi = bel_ioi
                    .extra_int_in(
                        format!("PLLCLK{i}"),
                        &[
                            format!("BIOI_INNER_PLLCLK{i}"),
                            format!("TIOI_INNER_PLLCLK{i}"),
                            format!("TIOI_PLLCLK{i}"),
                            format!("IOI_PLLCLK{i}"),
                            format!("IOI_PLLCLK{i}_BRK"),
                            format!("RIOI_PLLCLK{i}"),
                            format!("RIOI_PLLCLK{i}_BRK"),
                        ],
                    )
                    .extra_int_in(
                        format!("PLLCE{i}"),
                        &[
                            format!("BIOI_INNER_PLLCE{i}"),
                            format!("TIOI_INNER_PLLCE{i}"),
                            format!("TIOI_PLLCE{i}"),
                            format!("IOI_PLLCE{i}"),
                            format!("IOI_PLLCE{i}_BRK"),
                            format!("RIOI_PLLCE{i}"),
                            format!("RIOI_PLLCE{i}_BRK"),
                        ],
                    );
            }
            bels.push(bel_ioi);
            builder
                .xtile_id(tcid, tkn, xy)
                .ref_single(xy, 0, intf_ioi)
                .bels(bels)
                .extract();
        }
    }

    for (tkn, naming, idx) in [
        ("LIOB", "LIOB", [1, 0]),
        ("LIOB_RDY", "LIOB_RDY", [1, 0]),
        ("LIOB_PCI", "LIOB_PCI", [1, 0]),
        ("RIOB", "RIOB", [1, 0]),
        ("RIOB_RDY", "RIOB_RDY", [1, 0]),
        ("RIOB_PCI", "RIOB_PCI", [1, 0]),
        ("BIOB", "BIOB_OUTER", [2, 3]),
        ("BIOB_SINGLE_ALT", "BIOB_OUTER", [2, 3]),
        ("BIOB", "BIOB_INNER", [1, 0]),
        ("BIOB_SINGLE", "BIOB_INNER", [1, 0]),
        ("TIOB", "TIOB_OUTER", [1, 0]),
        ("TIOB_SINGLE", "TIOB_OUTER", [1, 0]),
        ("TIOB", "TIOB_INNER", [3, 2]),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_indexed(bslots::IOB[i], "IOB", idx[i])
                    .pins_name_only(&["PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN", "PCI_RDY"])
                    .pin_name_only("O", 1)
                    .pin_name_only("T", 1);
                if (tkn.ends_with("RDY") && i == 1) || (tkn.ends_with("PCI") && i == 0) {
                    bel = bel.pin_name_only("PCI_RDY", 1);
                }
                bels.push(bel);
            }
            builder.xtile_id(tcls::IOB, naming, xy).bels(bels).extract();
        }
    }

    for tkn in ["REGH_LIOI_INT", "REGH_LIOI_INT_BOT25"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_xy(bslots::PCILOGICSE, "PCILOGIC", 0, 0)
                .pin_name_only("PCI_CE", 1)
                .pin_name_only("IRDY", 1)
                .pin_name_only("TRDY", 1);
            builder
                .xtile_id(tcls::PCILOGICSE, "PCILOGICSE_L", xy)
                .raw_tile(xy.delta(-2, 0))
                .raw_tile(xy.delta(1, 0))
                .raw_tile(xy.delta(0, 1))
                .ref_int(xy.delta(0, 1), 0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["REGH_RIOI", "REGH_RIOI_BOT25"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_xy(bslots::PCILOGICSE, "PCILOGIC", 0, 0)
                .pin_name_only("PCI_CE", 1)
                .pin_name_only("IRDY", 1)
                .pin_name_only("TRDY", 1);
            builder
                .xtile_id(tcls::PCILOGICSE, "PCILOGICSE_R", xy)
                .raw_tile(xy.delta(3, 0))
                .raw_tile(xy.delta(-1, 1))
                .ref_int(xy.delta(-1, 1), 0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["MCB_L", "MCB_L_BOT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming("INTF");
            let mut bel = builder
                .bel_xy(bslots::MCB, "MCB", 0, 0)
                .pin_name_only("IOIDRPSDI", 1)
                .pin_name_only("IOIDRPSDO", 1)
                .pin_name_only("IOIDRPTRAIN", 1)
                .pin_name_only("IOIDRPCS", 1)
                .pin_name_only("IOIDRPCLK", 1)
                .pin_name_only("IOIDRPBROADCAST", 1)
                .pin_name_only("IOIDRPADD", 1)
                .pin_name_only("IOIDRPUPDATE", 1)
                .pin_name_only("IOIDRPADDR0", 1)
                .pin_name_only("IOIDRPADDR1", 1)
                .pin_name_only("IOIDRPADDR2", 1)
                .pin_name_only("IOIDRPADDR3", 1)
                .pin_name_only("IOIDRPADDR4", 1)
                .pin_name_only("LDMN", 1)
                .pin_name_only("LDMP", 1)
                .pin_name_only("UDMN", 1)
                .pin_name_only("UDMP", 1)
                .pin_name_only("CAS", 1)
                .pin_name_only("RAS", 1)
                .pin_name_only("WE", 1)
                .pin_name_only("RST", 1)
                .pin_name_only("CKE", 1)
                .pin_name_only("ODT", 1)
                .pin_name_only("DQSIOIP", 1)
                .pin_name_only("DQSIOIN", 1)
                .pin_name_only("UDQSIOIP", 1)
                .pin_name_only("UDQSIOIN", 1)
                .pin_name_only("DQIOWEN0", 1)
                .pin_name_only("DQSIOWEN90P", 1)
                .pin_name_only("DQSIOWEN90N", 1);
            for i in 0..15 {
                bel = bel.pin_name_only(&format!("ADDR{i}"), 1);
            }
            for i in 0..16 {
                bel = bel.pin_name_only(&format!("DQOP{i}"), 1);
                bel = bel.pin_name_only(&format!("DQON{i}"), 1);
                bel = bel.pin_name_only(&format!("DQI{i}"), 1);
            }
            for i in 0..3 {
                bel = bel.pin_name_only(&format!("BA{i}"), 1);
            }
            bel = bel
                .sub_xy(rd, "TIEOFF", 0, 0)
                .raw_tile(2)
                .pin_rename("HARD0", "TIE_CLK_HARD0")
                .pin_rename("HARD1", "TIE_CLK_HARD1")
                .pin_rename("KEEP1", "TIE_CLK_KEEP1")
                .pins_name_only(&["TIE_CLK_HARD0", "TIE_CLK_HARD1", "TIE_CLK_KEEP1"])
                .extra_wire("TIE_CLK_OUTP0", &["MCB_BOT_MOUTP_GND"])
                .extra_wire("TIE_CLK_OUTN0", &["MCB_BOT_MOUTN_VCC"])
                .extra_wire("TIE_CLK_OUTP1", &["MCB_BOT_SOUTP_VCC"])
                .extra_wire("TIE_CLK_OUTN1", &["MCB_BOT_SOUTN_GND"])
                .sub_xy(rd, "TIEOFF", 0, 0)
                .raw_tile(3)
                .pin_rename("HARD0", "TIE_DQS0_HARD0")
                .pin_rename("HARD1", "TIE_DQS0_HARD1")
                .pin_rename("KEEP1", "TIE_DQS0_KEEP1")
                .pins_name_only(&["TIE_DQS0_HARD0", "TIE_DQS0_HARD1", "TIE_DQS0_KEEP1"])
                .extra_wire("TIE_DQS0_OUTP0", &["MCB_BOT_MOUTP_GND"])
                .extra_wire("TIE_DQS0_OUTN0", &["MCB_BOT_MOUTN_VCC"])
                .extra_wire("TIE_DQS0_OUTP1", &["MCB_BOT_SOUTP_VCC"])
                .extra_wire("TIE_DQS0_OUTN1", &["MCB_BOT_SOUTN_GND"])
                .sub_xy(rd, "TIEOFF", 0, 0)
                .raw_tile(4)
                .pin_rename("HARD0", "TIE_DQS1_HARD0")
                .pin_rename("HARD1", "TIE_DQS1_HARD1")
                .pin_rename("KEEP1", "TIE_DQS1_KEEP1")
                .pins_name_only(&["TIE_DQS1_HARD0", "TIE_DQS1_HARD1", "TIE_DQS1_KEEP1"])
                .extra_wire("TIE_DQS1_OUTP0", &["MCB_BOT_MOUTP_GND"])
                .extra_wire("TIE_DQS1_OUTN0", &["MCB_BOT_MOUTN_VCC"])
                .extra_wire("TIE_DQS1_OUTP1", &["MCB_BOT_SOUTP_VCC"])
                .extra_wire("TIE_DQS1_OUTN1", &["MCB_BOT_SOUTN_GND"]);
            let mut muis = vec![];
            let mut mui_xy = xy;
            let mut clk_xy = None;
            for _ in 0..8 {
                loop {
                    mui_xy = mui_xy.delta(0, -1);
                    let tile = &rd.tiles[&mui_xy];
                    if rd.tile_kinds.key(tile.kind) == "MCB_CAP_CLKPN" {
                        clk_xy = Some(mui_xy);
                    }
                    if rd.tile_kinds.key(tile.kind).starts_with("MCB_MUI") {
                        break;
                    }
                }
                muis.push(mui_xy);
            }
            let mut xn = builder
                .xtile_id(tcls::MCB, tkn, xy)
                .num_cells(28)
                .raw_tile(xy.delta(0, -7))
                .raw_tile(clk_xy.unwrap())
                .raw_tile(muis[5].delta(0, -1))
                .raw_tile(muis[0].delta(0, -1));
            for i in 0..12 {
                xn = xn.ref_single(xy.delta(-1, -6 + i as i32), i, intf);
            }
            for (i, &mxy) in muis.iter().enumerate() {
                xn = xn.raw_tile(mxy);
                for j in 0..2 {
                    xn = xn.ref_single(mxy.delta(-1, j as i32), 12 + i * 2 + j, intf);
                }
            }
            xn.bel(bel).extract();
        }
    }

    for (tkn, naming) in [
        ("HCLK_CLB_XL_INT", "HCLK"),
        ("HCLK_CLB_XM_INT", "HCLK"),
        ("HCLK_CLB_XL_INT_FOLD", "HCLK_FOLD"),
        ("HCLK_CLB_XM_INT_FOLD", "HCLK_FOLD"),
        ("DSP_INT_HCLK_FEEDTHRU", "HCLK"),
        ("DSP_INT_HCLK_FEEDTHRU_FOLD", "HCLK_FOLD"),
        ("BRAM_HCLK_FEEDTHRU", "HCLK"),
        ("BRAM_HCLK_FEEDTHRU_FOLD", "HCLK_FOLD"),
        ("HCLK_IOIL_INT", "HCLK"),
        ("HCLK_IOIR_INT", "HCLK"),
        ("HCLK_IOIL_INT_FOLD", "HCLK_FOLD"),
        ("HCLK_IOIR_INT_FOLD", "HCLK_FOLD"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_s = xy.delta(0, -1);
            let xy_n = xy.delta(0, 1);
            if !rd.tile_kinds.key(rd.tiles[&xy_s].kind).starts_with("INT") {
                continue;
            }
            if !rd.tile_kinds.key(rd.tiles[&xy_n].kind).starts_with("INT") {
                continue;
            }
            builder
                .xtile_id(tcls::HCLK, naming, xy)
                .num_cells(2)
                .ref_int(xy.delta(0, -1), 0)
                .ref_int(xy.delta(0, 1), 1)
                .switchbox(bslots::HCLK)
                .optin_muxes(&wires::HCLK[..])
                .extract();
            break;
        }
    }
    let pips = builder.pips.get_mut(&(tcls::HCLK, bslots::HCLK)).unwrap();
    for mode in pips.pips.values_mut() {
        *mode = PipMode::Buf;
    }

    if let Some(&xy) = rd.tiles_by_kind_name("REG_V_HCLK").iter().next() {
        let mut bels = vec![];
        for (i, we) in ['W', 'E'].into_iter().enumerate() {
            for j in 0..16 {
                bels.push(
                    builder
                        .bel_xy(bslots::HCLK_ROW, "BUFH", i * 3, (1 - i) * 16 + j)
                        .manual()
                        .pin_rename("I", format!("I_{we}{j}"))
                        .pin_rename("O", format!("O_{we}{j}"))
                        .pin_name_only(&format!("I_{we}{j}"), 1)
                        .pin_name_only(&format!("O_{we}{j}"), 1),
                );
            }
        }
        let xt = builder
            .xtile_id(tcls::HCLK_ROW, "HCLK_ROW", xy)
            .switchbox(bslots::HCLK_ROW)
            .optin_muxes(&wires::HCLK_ROW[..])
            .num_cells(2)
            .bels(bels)
            .extract();

        let mut naming = BelNaming::default();
        for (_, bn) in xt.bels {
            naming.tiles.extend(bn.tiles);
            naming.pins.extend(bn.pins);
        }
        let mut tn = TileClassNaming::default();
        tn.bels.insert(bslots::HCLK_ROW, naming);
        builder.insert_tcls_naming("HCLK_ROW", tn);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CLKC").iter().next() {
        let mut bels = vec![];
        for i in 0..16 {
            bels.push(builder.bel_xy(bslots::BUFGMUX[i], "BUFGMUX", usize::from((i & 4) != 0), i));
        }
        builder
            .xtile_id(tcls::CLKC, "CLKC", xy)
            .num_cells(6)
            .switchbox(bslots::CLKC_INT)
            .optin_muxes(&wires::IMUX_BUFG[..])
            .optin_muxes(&wires::CMT_BUFPLL_H_CLKOUT[..])
            .optin_muxes(&wires::CMT_BUFPLL_H_LOCKED[..])
            .optin_muxes(&wires::CMT_BUFPLL_V_CLKOUT_S[..])
            .optin_muxes(&wires::CMT_BUFPLL_V_LOCKED_S[..])
            .optin_muxes(&wires::CMT_BUFPLL_V_CLKOUT_N[..])
            .optin_muxes(&wires::CMT_BUFPLL_V_LOCKED_N[..])
            .raw_tile(xy.delta(-1, 0))
            .ref_int(xy.delta(-3, 1), 1)
            .bels(bels)
            .extract();
    }

    for (tcid, naming, tkn, e, bio2, bpll) in [
        (
            tcls::CLK_W,
            "CLK_W",
            "REG_L",
            'L',
            [
                (1, 0),
                (1, 1),
                (1, 6),
                (1, 7),
                (0, 8),
                (0, 9),
                (0, 14),
                (0, 15),
            ],
            [1, 0],
        ),
        (
            tcls::CLK_E,
            "CLK_E",
            "REG_R",
            'R',
            [
                (1, 10),
                (1, 11),
                (1, 8),
                (1, 9),
                (0, 2),
                (0, 3),
                (0, 0),
                (0, 1),
            ],
            [1, 0],
        ),
        (
            tcls::CLK_S,
            "CLK_S",
            "REG_B",
            'B',
            [
                (2, 0),
                (2, 1),
                (2, 6),
                (2, 7),
                (0, 0),
                (0, 1),
                (0, 6),
                (0, 7),
            ],
            [0, 1],
        ),
        (
            tcls::CLK_N,
            "CLK_N",
            "REG_T",
            'T',
            [
                (0, 2),
                (0, 3),
                (0, 0),
                (0, 1),
                (2, 2),
                (2, 3),
                (2, 0),
                (2, 1),
            ],
            [1, 0],
        ),
    ] {
        let xy = *rd.tiles_by_kind_name(tkn).iter().next().unwrap();
        let mut bels = vec![];
        for i in 0..8 {
            bels.push(
                builder
                    .bel_xy(bslots::BUFIO2[i], "BUFIO2", bio2[i].0, bio2[i].1)
                    .extra_int_out("DIVCLK_CMT", &[format!("REG{e}_CLK_INDIRECT{i}")])
                    .extra_wire("TIE_0", &[format!("REG{e}_GND")])
                    .extra_wire("TIE_1", &[format!("REG{e}_VCC")]),
            );
        }
        for i in 0..8 {
            bels.push(
                builder
                    .bel_xy(bslots::BUFIO2FB[i], "BUFIO2FB", bio2[i].0, bio2[i].1)
                    .pins_name_only(&["IB"])
                    .extra_wire("TIE_1", &[format!("REG{e}_VCC")]),
            );
        }
        let mut bel = builder
            .bel_xy(bslots::BUFPLL, "BUFPLL_MCB", 0, 0)
            .extra_int_out(
                "PLLCE0",
                &[
                    "REGL_PLL_CEOUT0_LEFT",
                    "REGR_CEOUT0",
                    "REGB_CEOUT0",
                    "REGT_CEOUT0",
                ],
            )
            .extra_int_out(
                "PLLCE1",
                &[
                    "REGL_PLL_CEOUT1_LEFT",
                    "REGR_CEOUT1",
                    "REGB_CEOUT1",
                    "REGT_CEOUT1",
                ],
            )
            .extra_int_out(
                "PLLCLK0",
                &[
                    "REGL_PLL_CLKOUT0_LEFT",
                    "REGR_PLLCLK0",
                    "REGB_PLLCLK0",
                    "REGT_PLLCLK0",
                ],
            )
            .extra_int_out(
                "PLLCLK1",
                &[
                    "REGL_PLL_CLKOUT1_LEFT",
                    "REGR_PLLCLK1",
                    "REGB_PLLCLK1",
                    "REGT_PLLCLK1",
                ],
            )
            .extra_int_out("LOCK0", &[format!("REG{e}_LOCK0")])
            .extra_int_out("LOCK1", &[format!("REG{e}_LOCK1")])
            .extra_wire("TIE_1", &[format!("REG{e}_VCC")]);
        if matches!(e, 'L' | 'R') {
            bel = bel
                .extra_int_in("GCLK0", &[format!("REG{e}_GCLK0")])
                .extra_int_in("GCLK1", &[format!("REG{e}_GCLK1")])
                .extra_int_in("PLLIN_GCLK0", &[format!("REG{e}_GCLK2")])
                .extra_int_in("PLLIN_GCLK1", &[format!("REG{e}_GCLK3")])
                .extra_int_in("PLLIN_CMT0", &[format!("REG{e}_CLKPLL0")])
                .extra_int_in("PLLIN_CMT1", &[format!("REG{e}_CLKPLL1")])
                .extra_int_in("LOCKED0", &[format!("REG{e}_LOCKED0")])
                .extra_int_in("LOCKED1", &[format!("REG{e}_LOCKED1")]);
        } else {
            bel = bel
                .extra_int_in("GCLK0", &[format!("REG{e}_GCLK0")])
                .extra_int_in("GCLK1", &[format!("REG{e}_GCLK1")])
                .manual();
            for i in 0..6 {
                bel = bel.extra_wire(
                    format!("PLLIN_SN{i}"),
                    &[
                        format!("REGB_PLL_IOCLK_DOWN{i}"),
                        format!("REGT_PLL_IOCLK_UP{i}"),
                    ],
                );
            }
            for i in 0..3 {
                bel = bel.extra_wire(format!("LOCKED_SN{i}"), &[format!("REG{e}_LOCKIN{i}")]);
            }
            bels.push(builder.bel_virtual(bslots::MISR_CLK));
        }

        for pin in [
            "PLLIN0",
            "PLLIN1",
            "IOCLK0",
            "IOCLK1",
            "SERDESSTROBE0",
            "SERDESSTROBE1",
            "LOCKED",
            "LOCK",
            "GCLK",
        ] {
            let qpin = &format!("BUFPLL_MCB_{pin}");
            bel = bel.pin_rename(pin, qpin).pins_name_only(&[qpin]);
        }
        for i in 0..2 {
            bel = bel.sub_xy(rd, "BUFPLL", 0, bpll[i]);
            for pin in ["PLLIN", "IOCLK", "SERDESSTROBE", "LOCKED", "LOCK", "GCLK"] {
                let qpin = &format!("BUFPLL{i}_{pin}");
                bel = bel.pin_rename(pin, qpin).pins_name_only(&[qpin]);
            }
        }
        bels.push(bel);

        let mut xn = builder
            .xtile_id(tcid, naming, xy)
            .force_test_mux_in()
            .switchbox(bslots::CLK_INT)
            .optin_muxes(&wires::DIVCLK_CLKC[..])
            .optin_muxes(&wires::IMUX_BUFIO2_I[..])
            .optin_muxes(&wires::IMUX_BUFIO2_IB[..])
            .optin_muxes(&wires::IMUX_BUFIO2FB[..]);
        match tkn {
            "REG_L" => {
                xn = xn
                    .num_cells(6)
                    .raw_tile_single(xy.delta(1, 0), 2)
                    .raw_tile_single(xy.delta(2, 1), 2)
                    .raw_tile_single(xy.delta(2, 2), 3);
            }
            "REG_R" => {
                xn = xn
                    .num_cells(6)
                    .raw_tile_single(xy.delta(-1, 0), 2)
                    .raw_tile_single(xy.delta(-4, 1), 2)
                    .raw_tile_single(xy.delta(-4, 2), 3);
            }
            "REG_B" => {
                xn = xn
                    .num_cells(4)
                    .raw_tile_single(xy.delta(0, 1), 1)
                    .raw_tile(xy.delta(2, 1)) // BUFPLL mux
                    .raw_tile_single(xy.delta(2, 3), 2)
                    .optin_muxes(&wires::IMUX_CLK_GCLK[..]);
            }
            "REG_T" => {
                xn = xn
                    .num_cells(4)
                    .raw_tile_single(xy.delta(0, -1), 0)
                    .raw_tile(xy.delta(2, -1)) // BUFPLL mux
                    .raw_tile_single(xy.delta(2, -2), 2)
                    .optin_muxes(&wires::IMUX_CLK_GCLK[..]);
            }
            _ => unreachable!(),
        }
        let mut xt = xn.bels(bels).extract();
        let tn = builder.ndb.tile_class_namings.get_mut(naming).unwrap().1;
        tn.wires.insert(
            TileWireCoord::new_idx(0, wires::TIE_0),
            WireNaming {
                name: format!("REG{e}_GND"),
                alt_name: None,
                alt_pips_to: Default::default(),
                alt_pips_from: Default::default(),
            },
        );
        tn.wires.insert(
            TileWireCoord::new_idx(0, wires::PULLUP),
            WireNaming {
                name: format!("REG{e}_KEEP1_STUB"),
                alt_name: None,
                alt_pips_to: Default::default(),
                alt_pips_from: Default::default(),
            },
        );
        let pips = builder.pips.get_mut(&(tcid, bslots::CLK_INT)).unwrap();
        let mut new_pips = vec![];
        for &(wt, wf) in pips.pips.keys() {
            if let Some(idx) = wires::OUT_CLKPAD_CFB0.index_of(wf.wire) {
                new_pips.push((
                    wt,
                    TileWireCoord {
                        wire: wires::OUT_CLKPAD_CFB1[idx],
                        cell: wf.cell,
                    }
                    .pos(),
                ));
            }
        }
        pips.pips.retain(|&(wt, wf), _| {
            if wires::IMUX_BUFIO2_IB.contains(wt.wire) && wf.wire == wires::TIE_1 {
                false
            } else if let Some(idx) = wires::IMUX_BUFIO2_I.index_of(wt.wire)
                && idx % 2 == 1
                && wires::GTPCLK.contains(wf.wire)
                && ((tcid == tcls::CLK_W && wt.cell.to_idx() == 1)
                    || (tcid == tcls::CLK_E && wt.cell.to_idx() == 2))
            {
                new_pips.push((
                    wt,
                    TileWireCoord {
                        cell: wf.cell,
                        wire: wires::GTPCLK[idx],
                    }
                    .pos(),
                ));
                false
            } else {
                true
            }
        });
        for pip in new_pips {
            pips.pips.insert(pip, PipMode::Mux);
        }
        if matches!(e, 'B' | 'T') {
            let ci = if e == 'B' { 1 } else { 0 };
            let (mut bel, bn) = xt.bels.pop().unwrap();
            for i in 0..2 {
                bel.pins.insert(
                    format!("PLLIN_CMT{i}"),
                    BelPin::new_in(TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_PLLIN[i])),
                );
                bel.pins.insert(
                    format!("LOCKED{i}"),
                    BelPin::new_in(TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_LOCKED[i])),
                );
            }
            builder.insert_tcls_bel(tcid, bslots::BUFPLL, BelInfo::Legacy(bel));
            builder.insert_bel_naming(naming, bslots::BUFPLL, bn);
            let pips = builder.pips.get_mut(&(tcid, bslots::CLK_INT)).unwrap();
            for i in 0..2 {
                for j in 0..6 {
                    pips.pips.insert(
                        (
                            TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_PLLIN[i]),
                            TileWireCoord::new_idx(
                                ci,
                                if e == 'T' {
                                    wires::CMT_BUFPLL_V_CLKOUT_S[j]
                                } else {
                                    wires::CMT_BUFPLL_V_CLKOUT_N[j]
                                },
                            )
                            .pos(),
                        ),
                        PipMode::Mux,
                    );
                }
                for j in 0..3 {
                    pips.pips.insert(
                        (
                            TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_LOCKED[i]),
                            TileWireCoord::new_idx(
                                ci,
                                if e == 'T' {
                                    wires::CMT_BUFPLL_V_LOCKED_S[j]
                                } else {
                                    wires::CMT_BUFPLL_V_LOCKED_N[j]
                                },
                            )
                            .pos(),
                        ),
                        PipMode::Mux,
                    );
                }
            }
        }
    }

    let intf = builder.ndb.get_tile_class_naming("INTF");
    for (tkn, tcid, kind) in [
        ("CMT_DCM_BOT", tcls::DCM_BUFPLL_BUF_S, "DCM_BUFPLL_BUF_S"),
        (
            "CMT_DCM2_BOT",
            tcls::DCM_BUFPLL_BUF_S_MID,
            "DCM_BUFPLL_BUF_S_MID",
        ),
        ("CMT_DCM_TOP", tcls::DCM_BUFPLL_BUF_N, "DCM_BUFPLL_BUF_N"),
        (
            "CMT_DCM2_TOP",
            tcls::DCM_BUFPLL_BUF_N_MID,
            "DCM_BUFPLL_BUF_N_MID",
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let ii = 2 - i;
                let mut bel = builder
                    .bel_xy(bslots::DCM[i], "DCM", 0, i)
                    .extra_wire("CLKIN_TEST", &[format!("DCM{ii}_CLKIN_TOPLL")])
                    .extra_wire("CLKFB_TEST", &[format!("DCM{ii}_CLKFB_TOPLL")])
                    .extra_wire("CLKFB_CKINT0", &[format!("DCM{ii}_CLK_FROM_BUFG0")])
                    .extra_wire("CLKIN_CKINT0", &[format!("DCM{ii}_CLK_FROM_BUFG1")]);
                if tkn.ends_with("_BOT") {
                    for i in 0..8 {
                        bel = bel
                            .extra_wire(
                                format!("BUFIO2_BT{i}"),
                                &[
                                    format!("DCM_CLK_INDIRECT_TB_BOT{i}"),
                                    format!("DCM2_CLK_INDIRECT_TB_BOT{i}"),
                                ],
                            )
                            .extra_wire(
                                format!("BUFIO2_LR{i}"),
                                &[
                                    format!("DCM_CLK_INDIRECT_LR_TOP{i}"),
                                    format!("DCM2_CLK_INDIRECT_LR_TOP{i}"),
                                ],
                            );
                    }
                } else {
                    for i in 0..8 {
                        bel = bel
                            .extra_wire(
                                format!("BUFIO2_LR{i}"),
                                &[
                                    format!("DCM_CLK_INDIRECT_TB_BOT{i}"),
                                    format!("DCM2_CLK_INDIRECT_TB_BOT{i}"),
                                ],
                            )
                            .extra_wire(
                                format!("BUFIO2_BT{i}"),
                                &[
                                    format!("DCM_CLK_INDIRECT_LR_TOP{i}"),
                                    format!("DCM2_CLK_INDIRECT_LR_TOP{i}"),
                                ],
                            );
                    }
                }
                bels.push(bel);
            }
            bels.push(builder.bel_virtual(bslots::CMT_VREG));
            let mut bel_int_fixup = builder.bel_virtual(bslots::CMT_INT).manual();
            for i in 0..16 {
                bel_int_fixup = bel_int_fixup
                    .extra_int_in(format!("HCLK{i}_IN"), &[format!("DCM_FABRIC_CLK{i}")]);
            }
            for i in 0..2 {
                let ii = 2 - i;
                bel_int_fixup = bel_int_fixup
                    .extra_int_in(
                        format!("DCM{i}_CLKFB_CKINT0"),
                        &[format!("DCM{ii}_CLK_FROM_BUFG0")],
                    )
                    .extra_int_in(
                        format!("DCM{i}_CLKIN_CKINT0"),
                        &[format!("DCM{ii}_CLK_FROM_BUFG1")],
                    )
                    .extra_int_in(
                        format!("DCM{i}_CLKFB_CKINT1"),
                        &[format!("DCM{ii}_SE_CLK_IN0")],
                    )
                    .extra_int_in(
                        format!("DCM{i}_CLKIN_CKINT1"),
                        &[format!("DCM{ii}_SE_CLK_IN1")],
                    );
            }
            bels.push(bel_int_fixup);
            let mut xt = builder
                .xtile_id(tcls::CMT_DCM, tkn, xy)
                .num_cells(3)
                .switchbox(bslots::CMT_INT)
                .optin_muxes(&wires::CMT_OUT[..])
                .optin_muxes(&wires::CMT_CLKC_O[..])
                .optin_muxes(&wires::IMUX_DCM_CLKIN[..])
                .optin_muxes(&wires::IMUX_DCM_CLKFB[..])
                .ref_single(xy.delta(-1, -2), 0, intf)
                .ref_single(xy.delta(-1, 0), 1, intf)
                .bels(bels)
                .extract();
            {
                let (bel, naming) = xt.bels.pop().unwrap();
                let pips = builder
                    .pips
                    .get_mut(&(tcls::CMT_DCM, bslots::CMT_INT))
                    .unwrap();
                let mut tn = TileClassNaming::default();
                for i in 0..16 {
                    let pname = format!("HCLK{i}_IN");
                    let pin = &bel.pins[&pname];
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.iter().next().copied().unwrap();
                    let pn = &naming.pins[&pname];
                    let wire_out = TileWireCoord::new_idx(1, wires::CMT_OUT[i]);
                    pips.pips.insert((wire_out, wire.pos()), PipMode::Mux);
                    tn.wires.insert(
                        wire,
                        WireNaming {
                            name: pn.name_far.clone(),
                            alt_name: Some(pn.name.clone()),
                            alt_pips_to: Default::default(),
                            alt_pips_from: BTreeSet::from_iter([wire_out]),
                        },
                    );
                }
                for i in 0..2 {
                    for (pin, out) in [
                        ("CLKIN", wires::IMUX_DCM_CLKIN[i]),
                        ("CLKFB", wires::IMUX_DCM_CLKFB[i]),
                    ] {
                        for j in 0..2 {
                            let pname = format!("DCM{i}_{pin}_CKINT{j}");
                            let pin = &bel.pins[&pname];
                            assert_eq!(pin.wires.len(), 1);
                            let wire = pin.wires.iter().next().copied().unwrap();
                            let pn = &naming.pins[&pname];
                            let wire_out = TileWireCoord::new_idx(1, out);
                            pips.pips.insert((wire_out, wire.pos()), PipMode::Mux);
                            tn.wires.insert(
                                wire,
                                WireNaming {
                                    name: pn.name_far.clone(),
                                    alt_name: Some(pn.name.clone()),
                                    alt_pips_to: Default::default(),
                                    alt_pips_from: BTreeSet::from_iter([wire_out]),
                                },
                            );
                        }
                    }
                }
                for i in 0..2 {
                    let ii = 2 - i;
                    for (out, pin) in [
                        (wires::OUT_DCM_CLK0[i], "CLK0"),
                        (wires::OUT_DCM_CLK90[i], "CLK90"),
                        (wires::OUT_DCM_CLK180[i], "CLK180"),
                        (wires::OUT_DCM_CLK270[i], "CLK270"),
                        (wires::OUT_DCM_CLK2X[i], "CLK2X"),
                        (wires::OUT_DCM_CLK2X180[i], "CLK2X180"),
                        (wires::OUT_DCM_CLKDV[i], "CLKDV"),
                        (wires::OUT_DCM_CLKFX[i], "CLKFX"),
                        (wires::OUT_DCM_CLKFX180[i], "CLKFX180"),
                        (wires::OUT_DCM_CONCUR[i], "CONCUR"),
                    ] {
                        for (omux, oname, iname) in [
                            (
                                wires::OMUX_DCM_SKEWCLKIN1[i],
                                format!("DCM{ii}_CLK_TO_PLL"),
                                format!("DCM{ii}_{pin}"),
                            ),
                            (
                                wires::OMUX_DCM_SKEWCLKIN2[i],
                                format!("DCM_{i}_TESTCLK_PINWIRE"),
                                format!("DCM{ii}_{pin}_TEST"),
                            ),
                        ] {
                            let wt = TileWireCoord::new_idx(1, omux);
                            let wf = TileWireCoord::new_idx(1, out);
                            pips.pips.insert((wt, wf.pos()), PipMode::Mux);
                            tn.ext_pips.insert(
                                (wt, wf),
                                PipNaming {
                                    tile: RawTileId::from_idx(0),
                                    wire_to: oname,
                                    wire_from: iname,
                                },
                            );
                        }
                    }
                }
                for (wt, wf) in [
                    (wires::OMUX_PLL_SKEWCLKIN1_BUF, wires::OMUX_PLL_SKEWCLKIN1),
                    (wires::OMUX_PLL_SKEWCLKIN2_BUF, wires::OMUX_PLL_SKEWCLKIN2),
                ] {
                    let wt = TileWireCoord::new_idx(1, wt);
                    let wf = TileWireCoord::new_idx(2, wf);
                    pips.pips.insert((wt, wf.pos()), PipMode::Buf);
                }
                builder.insert_tcls_naming(tkn, tn);
            }

            builder
                .xtile_id(tcid, kind, xy)
                .switchbox(bslots::CMT_BUF)
                .optin_muxes(&wires::CMT_BUFPLL_V_CLKOUT_S[..])
                .optin_muxes(&wires::CMT_BUFPLL_V_LOCKED_S[..])
                .optin_muxes(&wires::CMT_BUFPLL_V_CLKOUT_N[..])
                .optin_muxes(&wires::CMT_BUFPLL_V_LOCKED_N[..])
                .num_cells(1)
                .extract();
            let pips = builder.pips.entry((tcid, bslots::CMT_BUF)).or_default();
            for ((wt, _), mode) in &mut pips.pips {
                *mode = if wires::CMT_BUFPLL_V_LOCKED_S.contains(wt.wire)
                    || wires::CMT_BUFPLL_V_LOCKED_N.contains(wt.wire)
                {
                    PipMode::PermaBuf
                } else {
                    PipMode::Buf
                };
            }
        }
    }
    for (tkn, bt, out) in [
        ("CMT_PLL_BOT", 'B', Some(1)),
        ("CMT_PLL1_BOT", 'B', Some(1)),
        ("CMT_PLL2_BOT", 'B', Some(0)),
        ("CMT_PLL3_BOT", 'B', None),
        ("CMT_PLL_TOP", 'T', Some(1)),
        ("CMT_PLL2_TOP", 'T', Some(0)),
        ("CMT_PLL3_TOP", 'T', None),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel_pll = builder
                .bel_xy(bslots::PLL, "PLL_ADV", 0, 0)
                .manual()
                .pins_name_only(&["REL"])
                .extra_wire("CLKIN1_CKINT0", &["CMT_CLK_FROM_BUFG0"])
                .extra_wire("CLKIN2_CKINT0", &["CMT_CLK_FROM_BUFG1"])
                .extra_wire("CLKIN2_CKINT1", &["CMT_SE_CLKIN0"])
                .extra_wire("CLKFBIN_CKINT0", &["CMT_CLK_FROM_BUFG2"])
                .extra_wire("CLKFBIN_CKINT1", &["CMT_SE_CLKIN1"])
                .extra_wire("CLK_FROM_DCM0", &["CMT_CLK_FROM_DCM2"])
                .extra_wire("CLK_FROM_DCM1", &["CMT_CLK_FROM_DCM1"])
                .extra_int_out("TEST_CLKIN", &["CMT_CLKMUX_CLKREF_TEST"])
                .extra_wire("CLKFBIN_TEST", &["CMT_CLKMUX_CLKFB_TEST"])
                .extra_wire("CLKFBDCM_TEST", &["CMT_PLL_CLKFBDCM_TEST"])
                .extra_wire("TEST_CLK", &["CMT_TEST_CLK"])
                .extra_wire("DCM0_CLKIN_TEST", &["CMT_DCM2_CLKIN"])
                .extra_wire("DCM0_CLKFB_TEST", &["CMT_DCM2_CLKFB"])
                .extra_wire("DCM1_CLKIN_TEST", &["CMT_DCM1_CLKIN"])
                .extra_wire("DCM1_CLKFB_TEST", &["CMT_DCM1_CLKFB"])
                .sub_xy(rd, "TIEOFF", 0, 0)
                .pin_rename("HARD0", "TIE_PLL_HARD0")
                .pin_rename("HARD1", "TIE_PLL_HARD1")
                .pin_rename("KEEP1", "TIE_PLL_KEEP1")
                .pins_name_only(&["TIE_PLL_HARD0", "TIE_PLL_HARD1", "TIE_PLL_KEEP1"]);
            let mut bel_int_fixup = builder
                .bel_virtual(bslots::CMT_INT)
                .manual()
                .extra_int_in("CLKIN1_CKINT0", &["CMT_CLK_FROM_BUFG0"])
                .extra_int_in("CLKIN2_CKINT0", &["CMT_CLK_FROM_BUFG1"])
                .extra_int_in("CLKIN2_CKINT1", &["CMT_SE_CLKIN0"])
                .extra_int_in("CLKFBIN_CKINT0", &["CMT_CLK_FROM_BUFG2"])
                .extra_int_in("CLKFBIN_CKINT1", &["CMT_SE_CLKIN1"]);
            for i in 0..16 {
                bel_int_fixup = bel_int_fixup
                    .extra_int_in(format!("HCLK{i}_IN"), &[format!("CMT_FABRIC_CLK{i}")]);
            }
            let mut xt = builder
                .xtile_id(tcls::CMT_PLL, tkn, xy)
                .num_cells(3)
                .switchbox(bslots::CMT_INT)
                .optin_muxes(&wires::CMT_OUT[..])
                .optin_muxes(&wires::CMT_CLKC_O[..])
                .optin_muxes(&[wires::IMUX_PLL_CLKIN1])
                .optin_muxes(&[wires::IMUX_PLL_CLKIN2])
                .optin_muxes(&[wires::IMUX_PLL_CLKFB])
                .optin_muxes(&[wires::OMUX_PLL_SKEWCLKIN1])
                .optin_muxes(&[wires::OMUX_PLL_SKEWCLKIN2])
                .optin_muxes(&[wires::CMT_TEST_CLK])
                .force_skip_pip(
                    TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKFB),
                    TileWireCoord::new_idx(1, wires::OUT_PLL_CLKOUT[0]),
                )
                .ref_single(xy.delta(-1, -2), 0, intf)
                .ref_single(xy.delta(-1, 0), 1, intf)
                .bel(bel_pll)
                .bel(bel_int_fixup)
                .extract();
            let wire_test_int = TileWireCoord::new_idx(0, wires::OUT_BEL[17]);
            {
                let (bel, naming) = xt.bels.pop().unwrap();
                let pips = builder
                    .pips
                    .get_mut(&(tcls::CMT_PLL, bslots::CMT_INT))
                    .unwrap();
                let mut tn = TileClassNaming::default();
                for i in 0..16 {
                    let pname = format!("HCLK{i}_IN");
                    let pin = &bel.pins[&pname];
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.iter().next().copied().unwrap();
                    let pn = &naming.pins[&pname];
                    let wire_out = TileWireCoord::new_idx(1, wires::CMT_OUT[i]);
                    pips.pips.insert((wire_out, wire.pos()), PipMode::Mux);
                    tn.wires.insert(
                        wire,
                        WireNaming {
                            name: pn.name_far.clone(),
                            alt_name: Some(pn.name.clone()),
                            alt_pips_to: Default::default(),
                            alt_pips_from: BTreeSet::from_iter([wire_out]),
                        },
                    );
                }
                for j in 0..2 {
                    let pname = format!("CLKFBIN_CKINT{j}");
                    let pin = &bel.pins[&pname];
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.iter().next().copied().unwrap();
                    let pn = &naming.pins[&pname];
                    let wire_out = TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKFB);
                    pips.pips.insert((wire_out, wire.pos()), PipMode::Mux);
                    tn.wires.insert(
                        wire,
                        WireNaming {
                            name: pn.name_far.clone(),
                            alt_name: Some(pn.name.clone()),
                            alt_pips_to: Default::default(),
                            alt_pips_from: BTreeSet::from_iter([wire_out]),
                        },
                    );
                }
                for j in 0..2 {
                    let pname = format!("CLKIN2_CKINT{j}");
                    let pin = &bel.pins[&pname];
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.iter().next().copied().unwrap();
                    let pn = &naming.pins[&pname];
                    let wire_out = TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKIN2);
                    pips.pips.insert((wire_out, wire.pos()), PipMode::Mux);
                    tn.wires.insert(
                        wire,
                        WireNaming {
                            name: pn.name_far.clone(),
                            alt_name: Some(pn.name.clone()),
                            alt_pips_to: Default::default(),
                            alt_pips_from: BTreeSet::from_iter([wire_out]),
                        },
                    );
                }
                {
                    let pname = "CLKIN1_CKINT0";
                    let pin = &bel.pins[pname];
                    assert_eq!(pin.wires.len(), 1);
                    let wire = pin.wires.iter().next().copied().unwrap();
                    let pn = &naming.pins[pname];
                    let wire_out = TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKIN1);
                    pips.pips.insert((wire_out, wire.pos()), PipMode::Mux);
                    tn.wires.insert(
                        wire,
                        WireNaming {
                            name: pn.name_far.clone(),
                            alt_name: Some(pn.name.clone()),
                            alt_pips_to: Default::default(),
                            alt_pips_from: BTreeSet::from_iter([wire_out]),
                        },
                    );
                }
                tn.wires.insert(
                    wire_test_int,
                    WireNaming {
                        name: "PLL_CLB1_LOGICOUT17".into(),
                        alt_name: None,
                        alt_pips_to: Default::default(),
                        alt_pips_from: Default::default(),
                    },
                );
                pips.pips.insert(
                    (
                        wire_test_int,
                        TileWireCoord::new_idx(1, wires::CMT_TEST_CLK).pos(),
                    ),
                    PipMode::PermaBuf,
                );

                let mut clkin1_ins = BTreeSet::new();
                let mut clkin2_ins = BTreeSet::new();
                pips.pips.retain(|&(wt, wf), _| match wt.wire {
                    wires::IMUX_PLL_CLKIN1 => {
                        clkin1_ins.insert(wf);
                        false
                    }
                    wires::IMUX_PLL_CLKIN2 => {
                        clkin2_ins.insert(wf);
                        false
                    }
                    _ => true,
                });
                let mut mux = PairMux {
                    dst: [
                        TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKIN1),
                        TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKIN2),
                    ],
                    bits: Default::default(),
                    src: Default::default(),
                };
                for in2 in clkin2_ins {
                    let in1 = if let Some(idx) = wires::DIVCLK_CMT_V.index_of(in2.wire) {
                        Some(wires::DIVCLK_CMT_V[idx - 4])
                    } else if let Some(idx) = wires::DIVCLK_CMT_W.index_of(in2.wire) {
                        Some(wires::DIVCLK_CMT_E[idx])
                    } else if in2.wire == wires::IMUX_CLK[1] {
                        Some(wires::IMUX_CLK[0])
                    } else {
                        None
                    };
                    let in1 = in1.map(|w| {
                        TileWireCoord {
                            cell: in2.cell,
                            wire: w,
                        }
                        .pos()
                    });
                    if let Some(in1) = in1 {
                        assert!(clkin1_ins.remove(&in1));
                    }
                    mux.src.insert([in1, Some(in2)], Default::default());
                }
                assert!(clkin1_ins.is_empty());
                pips.specials.insert(SwitchBoxItem::PairMux(mux));
                builder.insert_tcls_naming(tkn, tn);
            }
            {
                let (mut bel, mut naming) = xt.bels.pop().unwrap();
                bel.pins
                    .get_mut("LOCKED")
                    .unwrap()
                    .wires
                    .insert(TileWireCoord::new_idx(0, wires::OUT_BEL[18]));
                naming.pins.get_mut("LOCKED").unwrap().int_pips.insert(
                    TileWireCoord::new_idx(0, wires::OUT_BEL[18]),
                    PipNaming {
                        tile: RawTileId::from_idx(0),
                        wire_to: "PLL_CLB1_LOGICOUT18".into(),
                        wire_from: "PLL_LOCKED".into(),
                    },
                );
                builder.insert_tcls_bel(tcls::CMT_PLL, bslots::PLL, BelInfo::Legacy(bel));
                let mut tn = TileClassNaming::default();
                tn.bels.insert(bslots::PLL, naming);
                builder.insert_tcls_naming(tkn, tn);
            }
            let (tcid, tcname) = match (out, bt) {
                (Some(0), 'B') => (tcls::PLL_BUFPLL_OUT0_S, "PLL_BUFPLL_OUT0_S"),
                (Some(0), 'T') => (tcls::PLL_BUFPLL_OUT0_N, "PLL_BUFPLL_OUT0_N"),
                (Some(1), 'B') => (tcls::PLL_BUFPLL_OUT1_S, "PLL_BUFPLL_OUT1_S"),
                (Some(1), 'T') => (tcls::PLL_BUFPLL_OUT1_N, "PLL_BUFPLL_OUT1_N"),
                (None, 'B') => (tcls::PLL_BUFPLL_S, "PLL_BUFPLL_S"),
                (None, 'T') => (tcls::PLL_BUFPLL_N, "PLL_BUFPLL_N"),
                _ => unreachable!(),
            };
            if out.is_some() {
                builder
                    .xtile_id(tcid, tcname, xy)
                    .num_cells(1)
                    .switchbox(bslots::CMT_BUF)
                    .optin_muxes(&wires::CMT_BUFPLL_V_CLKOUT_S[..])
                    .optin_muxes(&wires::CMT_BUFPLL_V_LOCKED_S[..])
                    .optin_muxes(&wires::CMT_BUFPLL_V_CLKOUT_N[..])
                    .optin_muxes(&wires::CMT_BUFPLL_V_LOCKED_N[..])
                    .extract();
            } else {
                builder.xtile_id(tcid, tcname, xy).num_cells(1).extract();
            }
            let inject = match (out, bt) {
                (Some(0), 'B') => [None, Some(DirV::N), Some(DirV::S)],
                (Some(0), 'T') => [None, Some(DirV::S), Some(DirV::N)],
                (Some(1), 'B') => [Some(DirV::S), None, Some(DirV::S)],
                (Some(1), 'T') => [Some(DirV::N), None, Some(DirV::N)],
                (None, 'B') => [Some(DirV::S), Some(DirV::N), Some(DirV::S)],
                (None, 'T') => [Some(DirV::N), Some(DirV::S), Some(DirV::N)],
                _ => unreachable!(),
            };
            let pips = builder.pips.entry((tcid, bslots::CMT_BUF)).or_default();
            for ((wt, _), mode) in &mut pips.pips {
                *mode = if wires::CMT_BUFPLL_V_LOCKED_S.contains(wt.wire)
                    || wires::CMT_BUFPLL_V_LOCKED_N.contains(wt.wire)
                {
                    PipMode::PermaBuf
                } else {
                    PipMode::Buf
                };
            }
            for (i, dir) in inject.into_iter().enumerate() {
                let Some(dir) = dir else { continue };
                let conns = [
                    (
                        wires::CMT_BUFPLL_V_CLKOUT_S[i * 2],
                        wires::CMT_BUFPLL_V_CLKOUT_N[i * 2],
                        PipMode::Buf,
                    ),
                    (
                        wires::CMT_BUFPLL_V_CLKOUT_S[i * 2 + 1],
                        wires::CMT_BUFPLL_V_CLKOUT_N[i * 2 + 1],
                        PipMode::Buf,
                    ),
                    (
                        wires::CMT_BUFPLL_V_LOCKED_S[i],
                        wires::CMT_BUFPLL_V_LOCKED_N[i],
                        PipMode::PermaBuf,
                    ),
                ];
                for (ws, wn, mode) in conns {
                    match dir {
                        DirV::S => {
                            pips.pips.insert(
                                (
                                    TileWireCoord::new_idx(0, ws),
                                    TileWireCoord::new_idx(0, wn).pos(),
                                ),
                                mode,
                            );
                        }
                        DirV::N => {
                            pips.pips.insert(
                                (
                                    TileWireCoord::new_idx(0, wn),
                                    TileWireCoord::new_idx(0, ws).pos(),
                                ),
                                mode,
                            );
                        }
                    }
                }
            }
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE_TOP").iter().next() {
        let mut intf_xy = Vec::new();
        let nr = builder.ndb.get_tile_class_naming("INTF_RTERM");
        let nl = builder.ndb.get_tile_class_naming("INTF_LTERM");
        for dy in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16] {
            intf_xy.push((xy.delta(-5, -9 + dy), nr));
        }
        for dy in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16] {
            intf_xy.push((xy.delta(2, -9 + dy), nl));
        }
        builder.extract_xtile_bels_intf_id(
            tcls::PCIE,
            xy,
            &[],
            &[],
            &intf_xy,
            "PCIE",
            &[builder.bel_xy(bslots::PCIE, "PCIE", 0, 0)],
        );
    }

    for tkn in ["GTPDUAL_BOT", "GTPDUAL_TOP"] {
        let is_b = tkn == "GTPDUAL_BOT";
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf_rterm = builder.ndb.get_tile_class_naming("INTF_RTERM");
            let intf_lterm = builder.ndb.get_tile_class_naming("INTF_LTERM");
            let by = if is_b { 0 } else { -9 };
            let intfs_l: [_; 8] = core::array::from_fn(|i| {
                builder
                    .walk_to_int(xy.delta(0, by + i as i32), Dir::W, false)
                    .unwrap()
                    .delta(1, 0)
            });
            let intfs_r: [_; 8] = core::array::from_fn(|i| {
                builder
                    .walk_to_int(xy.delta(0, by + i as i32), Dir::E, false)
                    .unwrap()
                    .delta(-1, 0)
            });
            let mut bel = builder
                .bel_xy(bslots::GTP, "GTPA1_DUAL", 0, 0)
                .pins_name_only(&[
                    "RXP0",
                    "RXN0",
                    "RXP1",
                    "RXN1",
                    "TXP0",
                    "TXN0",
                    "TXP1",
                    "TXN1",
                    "CLK00",
                    "CLK01",
                    "CLK10",
                    "CLK11",
                    "REFCLKPLL0",
                    "REFCLKPLL1",
                    "CLKINEAST0",
                    "CLKINEAST1",
                    "CLKINWEST0",
                    "CLKINWEST1",
                ])
                .pin_name_only("RXCHBONDI0", 1)
                .pin_name_only("RXCHBONDI1", 1)
                .pin_name_only("RXCHBONDI2", 1)
                .pin_name_only("RXCHBONDO0", 1)
                .pin_name_only("RXCHBONDO1", 1)
                .pin_name_only("RXCHBONDO2", 1)
                .extra_wire("CLKOUT_EW", &["GTP_CLKOUT_EW0", "GTP_BOT_CLKOUT_EW0"])
                .extra_wire(
                    "CLKINEAST",
                    &["GTP_ALT_CLKOUTEAST0", "GTP_BOT_ALT_CLKOUTEAST0"],
                )
                .extra_wire(
                    "CLKINWEST",
                    &["GTP_ALT_CLKOUTWEST0", "GTP_BOT_ALT_CLKOUTWEST0"],
                );
            for i in 0..5 {
                bel = bel
                    .pins_name_only(&[
                        format!("RCALINEAST{i}"),
                        format!("RCALINWEST{i}"),
                        format!("RCALOUTEAST{i}"),
                        format!("RCALOUTWEST{i}"),
                    ])
                    .extra_wire_force(
                        format!("RCALOUTEAST{i}_BUF"),
                        if is_b {
                            format!("GTPDUAL_BOT_RCALOUTEAST{i}")
                        } else {
                            format!("GTPDUAL_RCALOUTEAST{i}")
                        },
                    )
                    .extra_wire_force(
                        format!("RCALINEAST{i}_BUF"),
                        if is_b {
                            format!("GTPDUAL_BOT_RCALINEAST{i}")
                        } else {
                            format!("GTPDUAL_RCALINEAST{i}")
                        },
                    );
            }
            for i in 0..2 {
                bel = bel
                    .sub_xy(rd, "BUFDS", 0, i)
                    .pin_rename("I", format!("BUFDS{i}_I"))
                    .pin_rename("IB", format!("BUFDS{i}_IB"))
                    .pin_rename("O", format!("BUFDS{i}_O"))
                    .pins_name_only(&[
                        format!("BUFDS{i}_I"),
                        format!("BUFDS{i}_IB"),
                        format!("BUFDS{i}_O"),
                    ]);
            }

            for (i, name) in [
                (1, "OPAD_TXP0"),
                (3, "OPAD_TXN0"),
                (0, "OPAD_TXP1"),
                (2, "OPAD_TXN1"),
            ] {
                bel = bel
                    .sub_xy(rd, "OPAD", 0, i)
                    .pin_rename("I", format!("{name}_I"))
                    .pins_name_only(&[format!("{name}_I")]);
            }

            for (i, name) in [
                (2, "IPAD_RXP0"),
                (0, "IPAD_RXN0"),
                (3, "IPAD_RXP1"),
                (1, "IPAD_RXN1"),
                (5, "IPAD_CLKP0"),
                (4, "IPAD_CLKN0"),
                (7, "IPAD_CLKP1"),
                (6, "IPAD_CLKN1"),
            ] {
                bel = bel
                    .sub_xy(rd, "IPAD", 0, i)
                    .pin_rename("O", format!("{name}_O"))
                    .pins_name_only(&[format!("{name}_O")]);
            }

            let mut xn = builder.xtile_id(tcls::GTP, tkn, xy).num_cells(16);
            for i in 0..8 {
                xn = xn.ref_single(intfs_l[i], i, intf_rterm).ref_single(
                    intfs_r[i],
                    8 + i,
                    intf_lterm,
                );
            }
            xn.bel(bel).extract();
        }
    }

    builder.build()
}
