use std::collections::BTreeSet;

use prjcombine_interconnect::{
    db::{IntDb, ProgInv, SwitchBoxItem, WireSlotIdExt},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::{NamingDb, WireNaming};
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};
use prjcombine_types::bsdata::PolTileBit;
use prjcombine_virtex4::defs::{
    self, bslots,
    virtex6::{ccls, tcls, wires},
};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::virtex6::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => ccls::PASS_W,
        Dir::E => ccls::PASS_E,
        Dir::S => ccls::PASS_S,
        Dir::N => ccls::PASS_N,
    }));

    builder.wire_names(wires::TIE_0, &["GND_WIRE"]);
    builder.wire_names(wires::TIE_1, &["VCC_WIRE"]);

    for i in 0..8 {
        builder.wire_names(wires::LCLK[i], &[format!("GCLK_B{i}")]);
    }

    for (lr, w0, w1, dir, dbeg, dend) in [
        (
            "L",
            wires::SNG_E0,
            wires::SNG_E1,
            Dir::E,
            Some((3, Dir::N, wires::SNG_E0_N3)),
            Some((0, Dir::S, 3, wires::SNG_E1_S0)),
        ),
        (
            "R",
            wires::SNG_E0,
            wires::SNG_E1,
            Dir::E,
            Some((0, Dir::S, wires::SNG_E0_S4)),
            Some((3, Dir::N, 3, wires::SNG_E1_N7)),
        ),
        (
            "L",
            wires::SNG_W0,
            wires::SNG_W1,
            Dir::W,
            Some((3, Dir::N, wires::SNG_W0_N3)),
            Some((3, Dir::N, 1, wires::SNG_W1_N3)),
        ),
        (
            "R",
            wires::SNG_W0,
            wires::SNG_W1,
            Dir::W,
            Some((0, Dir::S, wires::SNG_W0_S4)),
            Some((0, Dir::S, 1, wires::SNG_W1_S4)),
        ),
        (
            "L",
            wires::SNG_N0,
            wires::SNG_N1,
            Dir::N,
            Some((3, Dir::N, wires::SNG_N0_N3)),
            Some((0, Dir::S, 3, wires::SNG_N1_S0)),
        ),
        ("R", wires::SNG_N0, wires::SNG_N1, Dir::N, None, None),
        ("L", wires::SNG_S0, wires::SNG_S1, Dir::S, None, None),
        (
            "R",
            wires::SNG_S0,
            wires::SNG_S1,
            Dir::S,
            Some((0, Dir::S, wires::SNG_S0_S4)),
            Some((3, Dir::N, 3, wires::SNG_S1_N7)),
        ),
    ] {
        for i in 0..4 {
            let ii = if lr == "L" { i } else { i + 4 };

            if let Some((xi, dbeg, wbeg)) = dbeg
                && xi == i
            {
                builder.wire_names(wbeg, &[format!("{dir}{lr}1BEG_{dbeg}{i}")]);
                if dir == dbeg {
                    continue;
                }
            }

            builder.wire_names(w0[ii], &[format!("{dir}{lr}1BEG{i}")]);
            builder.wire_names(w1[ii], &[format!("{dir}{lr}1END{i}")]);

            if let Some((xi, dend, n, wend)) = dend
                && i == xi
            {
                builder.wire_names(wend, &[format!("{dir}{lr}1END_{dend}{n}_{i}")]);
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
            Some((3, Dir::N, 0, wires::DBL_WW2_N3)),
        ),
        (
            Dir::N,
            Dir::N,
            wires::DBL_NN0,
            wires::DBL_NN1,
            wires::DBL_NN2,
            Some((0, Dir::S, 2, wires::DBL_NN2_S0)),
        ),
        (
            Dir::N,
            Dir::E,
            wires::DBL_NE0,
            wires::DBL_NE1,
            wires::DBL_NE2,
            Some((0, Dir::S, 3, wires::DBL_NE2_S0)),
        ),
        (
            Dir::N,
            Dir::W,
            wires::DBL_NW0,
            wires::DBL_NW1,
            wires::DBL_NW2,
            Some((0, Dir::S, 0, wires::DBL_NW2_S0)),
        ),
        (
            Dir::S,
            Dir::S,
            wires::DBL_SS0,
            wires::DBL_SS1,
            wires::DBL_SS2,
            Some((3, Dir::N, 0, wires::DBL_SS2_N3)),
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
            Some((3, Dir::N, 0, wires::DBL_SW2_N3)),
        ),
    ] {
        for i in 0..4 {
            builder.wire_names(w0[i], &[format!("{da}{db}2BEG{i}")]);
            builder.wire_names(w1[i], &[format!("{da}{db}2A{i}")]);
            builder.wire_names(w2[i], &[format!("{da}{db}2END{i}")]);
            if let Some((xi, dend, n, wend)) = dend
                && i == xi
            {
                builder.wire_names(wend, &[format!("{da}{db}2END_{dend}{n}_{i}")]);
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
            Some((0, Dir::S, 0, wires::QUAD_WW4_S0)),
        ),
        (
            Dir::N,
            Dir::N,
            wires::QUAD_NN0,
            wires::QUAD_NN1,
            wires::QUAD_NN2,
            wires::QUAD_NN3,
            wires::QUAD_NN4,
            Some((0, Dir::S, 1, wires::QUAD_NN4_S0)),
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
            Some((0, Dir::S, 0, wires::QUAD_NW4_S0)),
        ),
        (
            Dir::S,
            Dir::S,
            wires::QUAD_SS0,
            wires::QUAD_SS1,
            wires::QUAD_SS2,
            wires::QUAD_SS3,
            wires::QUAD_SS4,
            Some((3, Dir::N, 0, wires::QUAD_SS4_N3)),
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
            Some((3, Dir::N, 0, wires::QUAD_SW4_N3)),
        ),
    ] {
        for i in 0..4 {
            builder.wire_names(w0[i], &[format!("{da}{db}4BEG{i}")]);
            builder.wire_names(w1[i], &[format!("{da}{db}4A{i}")]);
            builder.wire_names(w2[i], &[format!("{da}{db}4B{i}")]);
            builder.wire_names(w3[i], &[format!("{da}{db}4C{i}")]);
            builder.wire_names(w4[i], &[format!("{da}{db}4END{i}")]);
            if let Some((xi, dend, n, wend)) = dend
                && i == xi
            {
                builder.wire_names(wend, &[format!("{da}{db}4END_{dend}{n}_{i}")]);
            }
        }
    }

    // The long wires.
    for i in 0..17 {
        builder.wire_names(wires::LH[i], &[format!("LH{i}")]);
        builder.wire_names(wires::LV[i], &[format!("LV{i}")]);
    }

    // The control inputs.
    for i in 0..2 {
        builder.wire_names(wires::IMUX_GFAN[i], &[format!("GFAN{i}")]);
    }
    for i in 0..2 {
        builder.wire_names(wires::IMUX_CLK[i], &[format!("CLK_B{i}")]);
    }
    for i in 0..2 {
        builder.wire_names(wires::IMUX_CTRL[i], &[format!("CTRL_B{i}")]);
    }
    for i in 0..8 {
        builder.wire_names(wires::IMUX_BYP[i], &[format!("BYP{i}")]);
        builder.mark_permabuf(wires::IMUX_BYP_SITE[i]);
        builder.mark_permabuf(wires::IMUX_BYP_BOUNCE[i]);
        builder.wire_names(wires::IMUX_BYP_SITE[i], &[format!("BYP_B{i}")]);
        builder.wire_names(wires::IMUX_BYP_BOUNCE[i], &[format!("BYP_BOUNCE{i}")]);
        if matches!(i, 2 | 3 | 6 | 7) {
            builder.wire_names(wires::IMUX_BYP_BOUNCE_N[i], &[format!("BYP_BOUNCE_N3_{i}")]);
        }
    }
    for i in 0..8 {
        builder.wire_names(wires::IMUX_FAN[i], &[format!("FAN{i}")]);
        builder.mark_permabuf(wires::IMUX_FAN_SITE[i]);
        builder.mark_permabuf(wires::IMUX_FAN_BOUNCE[i]);
        builder.wire_names(wires::IMUX_FAN_SITE[i], &[format!("FAN_B{i}")]);
        builder.wire_names(wires::IMUX_FAN_BOUNCE[i], &[format!("FAN_BOUNCE{i}")]);
        if matches!(i, 0 | 2 | 4 | 6) {
            builder.wire_names(wires::IMUX_FAN_BOUNCE_S[i], &[format!("FAN_BOUNCE_S3_{i}")]);
        }
    }
    for i in 0..48 {
        builder.wire_names(wires::IMUX_IMUX[i], &[format!("IMUX_B{i}")]);
        builder.mark_delay(wires::IMUX_IMUX[i], wires::IMUX_IMUX_DELAY[i]);
    }

    for i in 0..24 {
        builder.wire_names(wires::OUT[i], &[format!("LOGIC_OUTS{i}")]);
        builder.mark_test_mux_in(wires::OUT_BEL[i], wires::OUT[i]);
        builder.mark_test_mux_in_test(wires::OUT_TEST[i], wires::OUT[i]);
    }

    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_SPEC[i],
            &[
                format!("INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("EMAC_INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("PCIE_INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("PCIE_INT_INTERFACE_L_BLOCK_OUTS_B{i}"),
                format!("IOI_L_INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("GTX_INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("GT_L_INT_INTERFACE_BLOCK_OUTS_B{i}"),
            ],
        );
    }

    for i in 0..12 {
        builder.extra_name_sub(format!("HCLK_CK_BUFHCLK{i}"), 1, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_QBUF_CK_BUFHCLK{i}"), 1, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_IOI_CK_BUFHCLK{i}"), 4, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_HCLK_L{i}"), 20, wires::HCLK_CMT_W[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_HCLK_R{i}"), 20, wires::HCLK_CMT_E[i]);

        builder.extra_name_sub(format!("HCLK_IOI_LEAF_GCLK{i}"), 4, wires::HCLK_IO[i]);
        builder.extra_name_sub(format!("IOI_LEAF_GCLK{i}"), 0, wires::HCLK_IO[i]);
    }
    for i in 0..6 {
        builder.extra_name_sub(format!("HCLK_CK_BUFRCLK{i}"), 1, wires::RCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_QBUF_CK_BUFRCLK{i}"), 1, wires::RCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_IOI_CK_BUFRCLK{i}"), 4, wires::RCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_RCLK_L{i}"), 20, wires::RCLK_CMT_W[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_RCLK_R{i}"), 20, wires::RCLK_CMT_E[i]);

        builder.extra_name_sub(format!("HCLK_IOI_RCLK_TO_IO{i}"), 4, wires::RCLK_IO[i]);
        builder.extra_name_sub(format!("IOI_RCLK_FORIO{i}"), 0, wires::RCLK_IO[i]);
    }

    for i in 0..4 {
        builder.extra_name_sub(format!("CMT_CK_PERF_INNER_L{i}"), 44, wires::PERF_ROW[i]);
        builder.extra_name_sub(
            format!("CMT_CK_PERF_OUTER_L{i}"),
            44,
            wires::PERF_ROW_OUTER[i],
        );
        builder.extra_name_sub(format!("CMT_CK_PERF_INNER_R{i}"), 52, wires::PERF_ROW[i]);
        builder.extra_name_sub(
            format!("CMT_CK_PERF_OUTER_R{i}"),
            52,
            wires::PERF_ROW_OUTER[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_INNER_IOI",
            format!("HCLK_IOI_CK_PERF_INNER{i}"),
            4,
            wires::PERF_ROW[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_OUTER_IOI",
            format!("HCLK_IOI_CK_PERF_OUTER{i}"),
            4,
            wires::PERF_ROW[i],
        );
        builder.extra_name_sub(format!("HCLK_GTX_PERF_OUTER{i}"), 20, wires::PERF_ROW[i]);
        builder.extra_name_sub(
            format!("HCLK_GTX_LEFT_PERF_OUTER{i}"),
            20,
            wires::PERF_ROW[i],
        );

        builder.extra_name_sub(
            format!("HCLK_IOI_IO_PLL_CLK{ii}_BUFF", ii = i ^ 1),
            4,
            wires::PERF_BUF[i],
        );
        builder.extra_name_sub(
            format!("HCLK_IOI_IO_PLL_CLK{i}_DMUX"),
            4,
            wires::IMUX_BUFIO[i],
        );
    }
    builder.alt_name_sub("HCLK_IOI_RCLK_TOP0", 4, wires::IMUX_BUFIO[0]);
    builder.alt_name_sub("HCLK_IOI_RCLK_TOP1", 4, wires::IMUX_BUFIO[1]);
    builder.alt_name_sub("HCLK_IOI_RCLK_BOT0", 4, wires::IMUX_BUFIO[2]);
    builder.alt_name_sub("HCLK_IOI_RCLK_BOT1", 4, wires::IMUX_BUFIO[3]);
    builder.extra_name_sub("HCLK_IOI_RCLK_IMUX_TOP_B", 3, wires::IMUX_IMUX[4]);
    builder.extra_name_sub("HCLK_IOI_RCLK_IMUX_BOT_B", 4, wires::IMUX_IMUX[4]);

    builder.extra_name_sub("HCLK_IOI_RCLK_TOP_BEFORE_DIV", 4, wires::IMUX_BUFR[0]);
    builder.extra_name_sub("HCLK_IOI_RCLK_BOT_BEFORE_DIV", 4, wires::IMUX_BUFR[1]);
    builder.extra_name_sub(
        "HCLK_IOI_IDELAYCTRL_REFCLK",
        4,
        wires::IMUX_IDELAYCTRL_REFCLK,
    );
    builder.extra_name_sub("HCLK_GTX_PERFCLK", 20, wires::IMUX_GTX_PERFCLK);
    builder.extra_name_sub("HCLK_GTX_LEFT_PERFCLK", 20, wires::IMUX_GTX_PERFCLK);

    for i in 0..4 {
        builder.extra_name_sub(format!("HCLK_IOI_IOCLK{i}"), 4, wires::IOCLK[i]);
        builder.extra_name_sub(format!("HCLK_IOI_IOCLKMULTI{i}"), 4, wires::IOCLK[i + 4]);
    }
    for i in 0..8 {
        builder.extra_name(format!("IOI_IOCLK{i}"), wires::IOCLK[i]);
    }
    builder.extra_name_sub("HCLK_IOI_VIOCLK0", 4, wires::VIOCLK[0]);
    builder.extra_name_sub("HCLK_IOI_VIOCLK1", 4, wires::VIOCLK[1]);
    builder.extra_name_sub("HCLK_IOI_SIOCLK1", 4, wires::SIOCLK[0]);
    builder.extra_name_sub("HCLK_IOI_SIOCLK2", 4, wires::SIOCLK[1]);
    builder.extra_name_sub("HCLK_IOI_VIOCLK_SOUTH0", 4, wires::VIOCLK_S_BUF[0]);
    builder.extra_name_sub("HCLK_IOI_VIOCLK_SOUTH1", 4, wires::VIOCLK_S_BUF[1]);
    builder.extra_name_sub("HCLK_IOI_VIOCLK_NORTH0", 4, wires::VIOCLK_N_BUF[0]);
    builder.extra_name_sub("HCLK_IOI_VIOCLK_NORTH1", 4, wires::VIOCLK_N_BUF[1]);
    for i in 0..2 {
        builder.extra_name_sub(format!("HCLK_IOI_VRCLK{i}"), 4, wires::VRCLK[i]);
        builder.extra_name_sub(format!("HCLK_IOI_VRCLK_SOUTH{i}"), 4, wires::VRCLK_S[i]);
        builder.extra_name_sub(format!("HCLK_IOI_VRCLK_NORTH{i}"), 4, wires::VRCLK_N[i]);

        builder.extra_name_sub(format!("HCLK_IOI_BUFOCLK{i}"), 4, wires::VOCLK[i]);
        builder.extra_name_sub(format!("HCLK_IOI_VBUFOCLK_SOUTH{i}"), 4, wires::VOCLK_S[i]);
        builder.extra_name_sub(format!("HCLK_IOI_VBUFOCLK_NORTH{i}"), 4, wires::VOCLK_N[i]);
        builder.alt_name_sub(
            format!("HCLK_IOI_CLKB_TO_BUFO{i}"),
            4,
            wires::PERF_BUF[i * 3],
        );

        builder.extra_name_sub(format!("HCLK_IOI_BUFO_IN{i}"), 4, wires::OCLK[i]);
        builder.extra_name(format!("IOI_BUFO_CLK{i}"), wires::OCLK[i]);
    }
    for i in 0..10 {
        builder.extra_name_sub(format!("HCLK_IOI_CK_MGT{i}"), 4, wires::MGT_ROW[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_MGT_L{i}"), 20, wires::MGT_CMT_W[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_MGT_R{i}"), 20, wires::MGT_CMT_E[i]);
        builder.extra_name_sub(format!("HCLK_GTX_MGT{i}"), 20, wires::MGT_ROW[i]);
        builder.extra_name_sub(format!("HCLK_GTX_LEFT_MGT{i}"), 20, wires::MGT_ROW[i]);
        builder.extra_name_sub(format!("HCLK_GTH_LEFT_MGT{i}"), 20, wires::MGT_ROW[i]);
        builder.extra_name_sub(format!("HCLK_GTH_RIGHT_MGT{i}"), 20, wires::MGT_ROW[i]);
    }
    for i in 0..2 {
        for tkn in ["LIOI", "RIOI"] {
            builder.extra_name_sub(
                format!("{tkn}_ILOGIC{ii}_CLK", ii = i ^ 1),
                i,
                wires::IMUX_IOI_ICLK[0],
            );
            builder.extra_name_sub(
                format!("{tkn}_ILOGIC{ii}_CLKB", ii = i ^ 1),
                i,
                wires::IMUX_IOI_ICLK[1],
            );
            builder.extra_name_sub(
                format!("IOI_OCLK_{ii}", ii = i ^ 1),
                i,
                wires::IMUX_IOI_OCLK[0],
            );
            builder.extra_name_sub(
                format!("IOI_OCLKM_{ii}", ii = i ^ 1),
                i,
                wires::IMUX_IOI_OCLK[1],
            );
            builder.extra_name_sub(
                format!("{tkn}_OLOGIC{ii}_CLKDIV", ii = i ^ 1),
                i,
                wires::IMUX_IOI_OCLKDIV[0],
            );
            builder.extra_name_sub(
                format!("{tkn}_OLOGIC{ii}_CLKDIVB", ii = i ^ 1),
                i,
                wires::IMUX_IOI_OCLKDIV[1],
            );
            builder.extra_name_sub(
                format!("{tkn}_OLOGIC{ii}_CLKPERF", ii = i ^ 1),
                i,
                wires::IMUX_IOI_OCLKPERF,
            );
        }
    }
    builder.extra_name_sub("HCLK_IOI_I2IOCLK_BOT0", 1, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOI_I2IOCLK_BOT1", 3, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOI_I2IOCLK_TOP0", 5, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOI_I2IOCLK_TOP1", 7, wires::OUT_CLKPAD);

    for i in 0..4 {
        builder.extra_name_sub(format!("HCLK_CMT_CK_CCIO_L{i}"), 20, wires::CCIO_CMT_W[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_CCIO_R{i}"), 20, wires::CCIO_CMT_E[i]);

        builder.extra_name_sub(
            format!("CMT_BUFG_BOT_CK_PADIN{i}"),
            [3, 5, 4, 6][i],
            wires::OUT_CLKPAD,
        );

        builder.extra_name_sub(
            format!("CMT_BUFG_TOP_CK_PADIN{ii}", ii = i + 4),
            [3, 5, 4, 6][i],
            wires::OUT_CLKPAD,
        );
    }
    for i in 0..8 {
        builder.extra_name(format!("CMT_BUFG_BOT_CK_IO_TO_BUFG{i}"), wires::GIOB[i]);
        builder.extra_name(format!("CMT_BUFG_TOP_CK_IO_TO_BUFG{i}"), wires::GIOB[i]);
        builder.mark_permabuf(wires::GIOB[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_IO_TO_CMT{i}"), 20, wires::GIOB_CMT[i]);
    }
    for (tkn, c) in [("CMT_BUFG_BOT", 1), ("CMT_BUFG_TOP", 0)] {
        for i in 0..16 {
            builder.extra_name_tile_sub(
                tkn,
                format!("CMT_BUFG_BUFGCTRL{i}_O"),
                c,
                wires::OUT_BUFG[i],
            );
            builder.extra_name_tile_sub(
                tkn,
                format!("CMT_BUFG_FBG_OUT{i}"),
                c,
                wires::OUT_BUFG_GFB[i],
            );
        }
        for i in 0..32 {
            builder.mark_permabuf(wires::GCLK[i]);
            builder.extra_name_tile_sub(
                tkn,
                format!("CMT_BUFG_BUFGCTRL{j}_I{k}", j = i / 2, k = i % 2),
                c,
                wires::IMUX_BUFG_O[i],
            );
        }
    }
    for i in 0..32 {
        builder.extra_name_sub(format!("HCLK_CMT_CK_GCLK{i}"), 20, wires::GCLK_CMT[i]);
        builder.extra_name(format!("CMT_BUFG_CK_GCLK{i}"), wires::GCLK[i]);

        builder.extra_name_sub(
            format!("CMT_BUFG_BOT_CK_MUXED{i}"),
            1,
            wires::IMUX_BUFG_I[i],
        );
        builder.extra_name(format!("CMT_BUFG_TOP_CK_MUXED{i}"), wires::IMUX_BUFG_I[i]);
        builder.extra_name_sub(
            format!("HCLK_CMT_BOT_CK_BUFG_CASCO{i}"),
            20,
            wires::IMUX_BUFG_O[i],
        );
        builder.extra_name_sub(
            format!("HCLK_CMT_TOP_CK_BUFG_CASCO{i}"),
            20,
            wires::IMUX_BUFG_O[i],
        );
        builder.extra_name_sub(
            format!("HCLK_CMT_BOT_CK_BUFG_CASCIN{i}"),
            20,
            wires::IMUX_BUFG_I[i],
        );
        builder.extra_name_sub(
            format!("HCLK_CMT_TOP_CK_BUFG_CASCIN{i}"),
            20,
            wires::IMUX_BUFG_I[i],
        );
    }

    for i in 0..48 {
        builder.extra_name_tile_sub(
            "CMT_BUFG_BOT",
            format!("CMT_BUFG_BORROWED_IMUX{i}"),
            0,
            wires::IMUX_IMUX[i],
        );
        builder.extra_name_tile_sub(
            "CMT_BUFG_TOP",
            format!("CMT_BUFG_BORROWED_IMUX{i}"),
            2,
            wires::IMUX_IMUX[i],
        );
    }

    for i in 0..12 {
        builder.extra_name_sub(format!("HCLK_CMT_CK_OUT_L{i}"), 20, wires::IMUX_BUFHCE_W[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_OUT_R{i}"), 20, wires::IMUX_BUFHCE_E[i]);
    }
    for i in 0..14 {
        builder.extra_name_sub(format!("HCLK_CMT_CK_CMT_BOT{i}"), 20, wires::OUT_PLL_S[i]);
        builder.extra_name_sub(format!("HCLK_CMT_CK_CMT_TOP{i}"), 20, wires::OUT_PLL_N[i]);
    }
    for i in 0..32 {
        builder.extra_name_sub(format!("HCLK_CMT_CK_GCLK_TEST{i}"), 20, wires::GCLK_TEST[i]);
        //
    }
    for (tkn, outs) in [("CMT_BOT", wires::OUT_PLL_S), ("CMT_TOP", wires::OUT_PLL_N)] {
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT0", 20, outs[0]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT0B", 20, outs[1]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT1", 20, outs[2]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT1B", 20, outs[3]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT2", 20, outs[4]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT2B", 20, outs[5]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT3", 20, outs[6]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT3B", 20, outs[7]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT4", 20, outs[8]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT5", 20, outs[9]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKOUT6", 20, outs[10]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKFBOUT", 20, outs[11]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_CLKFBOUTB", 20, outs[12]);
        builder.extra_name_tile_sub(tkn, "CMT_MMCM_TMUXOUT", 20, outs[13]);
    }
    for i in 0..4 {
        builder.extra_name_tile_sub(
            "CMT_BOT",
            format!("CMT_PERF_CLK_BOUNCE{i}"),
            20,
            wires::OMUX_PLL_PERF_S[i],
        );
        builder.extra_name_tile_sub(
            "CMT_TOP",
            format!("CMT_PERF_CLK_BOUNCE{i}"),
            20,
            wires::OMUX_PLL_PERF_N[i],
        );
    }
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_CASC_OUT", 20, wires::OMUX_PLL_CASC[0]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_CASC_OUT", 20, wires::OMUX_PLL_CASC[1]);
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_CLKIN1", 20, wires::IMUX_PLL_CLKIN1[0]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_CLKIN1", 20, wires::IMUX_PLL_CLKIN1[1]);
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_CLKIN2", 20, wires::IMUX_PLL_CLKIN2[0]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_CLKIN2", 20, wires::IMUX_PLL_CLKIN2[1]);
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_CLKFBIN", 20, wires::IMUX_PLL_CLKFB[0]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_CLKFBIN", 20, wires::IMUX_PLL_CLKFB[1]);
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_IMUX_CLKIN1", 17, wires::IMUX_CLK[0]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_IMUX_CLKIN1", 22, wires::IMUX_CLK[1]);
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_IMUX_CLKIN2", 17, wires::IMUX_CLK[1]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_IMUX_CLKIN2", 22, wires::IMUX_CLK[0]);
    builder.extra_name_tile_sub("CMT_BOT", "CMT_MMCM_IMUX_CLKFB", 18, wires::IMUX_CLK[1]);
    builder.extra_name_tile_sub("CMT_TOP", "CMT_MMCM_IMUX_CLKFB", 21, wires::IMUX_CLK[0]);
    builder.extra_name_sub("HCLK_CMT_CLK_0_B0", 20, wires::BUFH_INT_W[0]);
    builder.extra_name_sub("HCLK_CMT_CLK_0_B1", 20, wires::BUFH_INT_W[1]);
    builder.extra_name_sub("HCLK_CMT_CLK_1_B0", 20, wires::BUFH_INT_E[0]);
    builder.extra_name_sub("HCLK_CMT_CLK_1_B1", 20, wires::BUFH_INT_E[1]);
    builder.extra_name_sub("HCLK_CMT_CLKFB_HCLK_B", 20, wires::IMUX_PLL_CLKFB_HCLK[0]);
    builder.extra_name_sub("HCLK_CMT_CLKFB_HCLK_T", 20, wires::IMUX_PLL_CLKFB_HCLK[1]);
    builder.extra_name_sub("HCLK_CMT_CLKFB_IO_B", 20, wires::IMUX_PLL_CLKFB_IO[0]);
    builder.extra_name_sub("HCLK_CMT_CLKFB_IO_T", 20, wires::IMUX_PLL_CLKFB_IO[1]);
    builder.extra_name_sub("HCLK_CMT_CLKIN1_HCLK_B", 20, wires::IMUX_PLL_CLKIN1_HCLK[0]);
    builder.extra_name_sub("HCLK_CMT_CLKIN1_HCLK_T", 20, wires::IMUX_PLL_CLKIN1_HCLK[1]);
    builder.extra_name_sub("HCLK_CMT_CLKIN2_HCLK_B", 20, wires::IMUX_PLL_CLKIN2_HCLK[0]);
    builder.extra_name_sub("HCLK_CMT_CLKIN2_HCLK_T", 20, wires::IMUX_PLL_CLKIN2_HCLK[1]);
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_L2",
        20,
        wires::IMUX_PLL_CLKIN1_HCLK_W[0],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_EXT_R2",
        20,
        wires::IMUX_PLL_CLKIN1_HCLK_E[0],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_EXT_L2",
        20,
        wires::IMUX_PLL_CLKIN1_HCLK_W[1],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_R2",
        20,
        wires::IMUX_PLL_CLKIN1_HCLK_E[1],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_L1",
        20,
        wires::IMUX_PLL_CLKIN2_HCLK_W[0],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_EXT_R1",
        20,
        wires::IMUX_PLL_CLKIN2_HCLK_E[0],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_EXT_L1",
        20,
        wires::IMUX_PLL_CLKIN2_HCLK_W[1],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_R1",
        20,
        wires::IMUX_PLL_CLKIN2_HCLK_E[1],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_L0",
        20,
        wires::IMUX_PLL_CLKFB_HCLK_W[0],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_EXT_R0",
        20,
        wires::IMUX_PLL_CLKFB_HCLK_E[0],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_EXT_L0",
        20,
        wires::IMUX_PLL_CLKFB_HCLK_W[1],
    );
    builder.extra_name_sub(
        "HCLK_CMT_CK_OUT2CMT_R0",
        20,
        wires::IMUX_PLL_CLKFB_HCLK_E[1],
    );
    builder.extra_name_sub("HCLK_CMT_CLKIN1_IO_B", 20, wires::IMUX_PLL_CLKIN1_IO[0]);
    builder.extra_name_sub("HCLK_CMT_CLKIN1_IO_T", 20, wires::IMUX_PLL_CLKIN1_IO[1]);
    builder.extra_name_sub("HCLK_CMT_CLKIN2_IO_B", 20, wires::IMUX_PLL_CLKIN2_IO[0]);
    builder.extra_name_sub("HCLK_CMT_CLKIN2_IO_T", 20, wires::IMUX_PLL_CLKIN2_IO[1]);
    builder.extra_name_sub("HCLK_CMT_CLKIN1_MGT_B", 20, wires::IMUX_PLL_CLKIN1_MGT[0]);
    builder.extra_name_sub("HCLK_CMT_CLKIN1_MGT_T", 20, wires::IMUX_PLL_CLKIN1_MGT[1]);
    builder.extra_name_sub("HCLK_CMT_CLKIN2_MGT_B", 20, wires::IMUX_PLL_CLKIN2_MGT[0]);
    builder.extra_name_sub("HCLK_CMT_CLKIN2_MGT_T", 20, wires::IMUX_PLL_CLKIN2_MGT[1]);
    builder.extra_name_sub("HCLK_CMT_CK_BUFH_TEST_L", 20, wires::BUFH_TEST_W);
    builder.extra_name_sub("HCLK_CMT_CK_BUFH_TEST_R", 20, wires::BUFH_TEST_E);
    builder.extra_name_sub("HCLK_CMT_CK_BUFH_TEST_OUT_L", 20, wires::BUFH_TEST_W_IN);
    builder.extra_name_sub("HCLK_CMT_CK_BUFH_TEST_OUT_R", 20, wires::BUFH_TEST_E_IN);
    builder.mark_optinv(wires::BUFH_TEST_W_IN, wires::BUFH_TEST_W);
    builder.mark_optinv(wires::BUFH_TEST_E_IN, wires::BUFH_TEST_E);

    builder.int_type_id(tcls::INT, bslots::INT, "INT", "INT");

    builder.extract_term_conn_id(ccls::TERM_W, Dir::W, "L_TERM_INT", &[]);
    builder.extract_term_conn_id(ccls::TERM_E, Dir::E, "R_TERM_INT", &[]);
    builder.extract_term_conn_id(ccls::TERM_S, Dir::S, "BRKH_T_TERM_INT", &[]);
    for &xy in rd.tiles_by_kind_name("PCIE") {
        let int_xy_a = Coord {
            x: xy.x,
            y: xy.y + 11,
        };
        let int_xy_b = Coord {
            x: xy.x + 2,
            y: xy.y + 11,
        };
        builder.extract_term_conn_tile_id(ccls::TERM_S, Dir::S, int_xy_a, &[]);
        builder.extract_term_conn_tile_id(ccls::TERM_S, Dir::S, int_xy_b, &[]);
    }
    builder.extract_term_conn_id(ccls::TERM_N, Dir::N, "BRKH_B_TERM_INT", &[]);

    builder.extract_intf_id(
        tcls::INTF,
        Dir::E,
        "INT_INTERFACE",
        "INTF",
        bslots::INTF_TESTMUX,
        Some(bslots::INTF_INT),
        true,
        false,
    );
    builder.extract_intf_id(
        tcls::INTF,
        Dir::E,
        "IOI_L_INT_INTERFACE",
        "INTF_IOI_L",
        bslots::INTF_TESTMUX,
        Some(bslots::INTF_INT),
        true,
        false,
    );
    for (n, tkn) in [
        ("GT_L", "GT_L_INT_INTERFACE"),
        ("GTX", "GTX_INT_INTERFACE"),
        ("EMAC", "EMAC_INT_INTERFACE"),
        ("PCIE_L", "PCIE_INT_INTERFACE_L"),
        ("PCIE_R", "PCIE_INT_INTERFACE_R"),
    ] {
        builder.extract_intf_id(
            tcls::INTF_DELAY,
            Dir::E,
            tkn,
            format!("INTF_{n}"),
            bslots::INTF_TESTMUX,
            Some(bslots::INTF_INT),
            true,
            true,
        );
    }

    for (tcid, tkn) in [(tcls::CLBLL, "CLBLL"), (tcls::CLBLM, "CLBLM")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = Coord {
                x: xy.x - 1,
                y: xy.y,
            };
            builder.extract_xtile_bels_id(
                tcid,
                xy,
                &[],
                &[int_xy],
                tkn,
                &[
                    builder
                        .bel_xy(bslots::SLICE[0], "SLICE", 0, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                    builder
                        .bel_xy(bslots::SLICE[1], "SLICE", 1, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                ],
                false,
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_tile_class_naming("INTF");
        for dy in 0..5 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }
        let bel_bram_f = builder
            .bel_xy(bslots::BRAM_F, "RAMB36", 0, 0)
            .pins_name_only(&[
                "CASCADEINA",
                "CASCADEINB",
                "TSTOUT1",
                "TSTOUT2",
                "TSTOUT3",
                "TSTOUT4",
            ])
            .pin_name_only("CASCADEOUTA", 1)
            .pin_name_only("CASCADEOUTB", 1);
        let bel_bram_h0 = builder.bel_xy(bslots::BRAM_H[0], "RAMB18", 0, 0);
        let mut bel_bram_h1 = builder
            .bel_xy(bslots::BRAM_H[1], "RAMB18", 0, 1)
            .pins_name_only(&[
                "FULL",
                "EMPTY",
                "ALMOSTFULL",
                "ALMOSTEMPTY",
                "WRERR",
                "RDERR",
            ]);
        for i in 0..12 {
            bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("RDCOUNT{i}"), 0);
            bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("WRCOUNT{i}"), 0);
        }
        builder.extract_xtile_bels_intf_id(
            tcls::BRAM,
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "BRAM",
            &[bel_bram_f, bel_bram_h0, bel_bram_h1],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_tile_class_naming("INTF");
        for dy in 0..15 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + 1 + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + 1 + dy,
                },
                n,
            ));
        }
        let mut bram_xy = Vec::new();
        for dy in [1, 6, 11] {
            bram_xy.push(Coord {
                x: xy.x,
                y: xy.y + dy,
            });
        }
        builder.extract_xtile_bels_intf_id(
            tcls::PMVBRAM,
            xy,
            &bram_xy,
            &int_xy,
            &intf_xy,
            "PMVBRAM",
            &[builder.bel_xy(bslots::PMVBRAM, "PMVBRAM", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_tile_class_naming("INTF");
        for dy in 0..5 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }

        let mut bels_dsp = vec![];
        for i in 0..2 {
            let mut bel = builder.bel_xy(bslots::DSP[i], "DSP48", 0, i);
            let buf_cnt = match i {
                0 => 0,
                1 => 1,
                _ => unreachable!(),
            };
            bel = bel.pin_name_only("MULTSIGNIN", 0);
            bel = bel.pin_name_only("MULTSIGNOUT", buf_cnt);
            bel = bel.pin_name_only("CARRYCASCIN", 0);
            bel = bel.pin_name_only("CARRYCASCOUT", buf_cnt);
            for j in 0..30 {
                bel = bel.pin_name_only(&format!("ACIN{j}"), 0);
                bel = bel.pin_name_only(&format!("ACOUT{j}"), buf_cnt);
            }
            for j in 0..18 {
                bel = bel.pin_name_only(&format!("BCIN{j}"), 0);
                bel = bel.pin_name_only(&format!("BCOUT{j}"), buf_cnt);
            }
            for j in 0..48 {
                bel = bel.pin_name_only(&format!("PCIN{j}"), 0);
                bel = bel.pin_name_only(&format!("PCOUT{j}"), buf_cnt);
            }
            bels_dsp.push(bel);
        }
        bels_dsp.push(
            builder
                .bel_xy(bslots::TIEOFF_DSP, "TIEOFF", 0, 0)
                .pins_name_only(&["HARD0", "HARD1"]),
        );
        builder.extract_xtile_bels_intf_id(tcls::DSP, xy, &[], &int_xy, &intf_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("EMAC").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_tile_class_naming("INTF_EMAC");
        for dy in 0..10 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y + dy,
                },
                n,
            ));
        }
        builder.extract_xtile_bels_intf_id(
            tcls::EMAC,
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "EMAC",
            &[builder.bel_xy(bslots::EMAC, "TEMAC", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let nl = builder.ndb.get_tile_class_naming("INTF_PCIE_L");
        let nr = builder.ndb.get_tile_class_naming("INTF_PCIE_R");
        for dy in 0..20 {
            int_xy.push(Coord {
                x: xy.x - 4,
                y: xy.y - 10 + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 3,
                    y: xy.y - 10 + dy,
                },
                nl,
            ));
        }
        for dy in 0..20 {
            int_xy.push(Coord {
                x: xy.x - 2,
                y: xy.y - 10 + dy,
            });
            intf_xy.push((
                Coord {
                    x: xy.x - 1,
                    y: xy.y - 10 + dy,
                },
                nr,
            ));
        }
        builder.extract_xtile_bels_intf_id(
            tcls::PCIE,
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "PCIE",
            &[builder.bel_xy(bslots::PCIE, "PCIE", 0, 0)],
        );
    }

    for (tkn, naming) in [
        ("HCLK", "HCLK"),
        ("HCLK_QBUF_L", "HCLK_QBUF"),
        ("HCLK_QBUF_R", "HCLK_QBUF"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel_gsig = builder.bel_xy(bslots::GLOBALSIG, "GLOBALSIG", 0, 0);
            let bel_drp = builder.bel_virtual(bslots::HCLK_DRP[0]);
            builder
                .xtile_id(tcls::HCLK, naming, xy)
                .num_cells(2)
                .ref_int(xy.delta(0, -1), 0)
                .ref_int(xy.delta(0, 1), 1)
                .switchbox(bslots::HCLK)
                .optin_muxes(&wires::LCLK[..])
                .bel(bel_gsig)
                .bel(bel_drp)
                .extract();
        }
    }
    let pips = builder.pips.get_mut(&(tcls::HCLK, bslots::HCLK)).unwrap();
    pips.pips.clear();
    for co in 0..2 {
        for o in 0..8 {
            for i in 0..12 {
                pips.pips.insert(
                    (wires::LCLK[o].cell(co), wires::HCLK_BUF[i].cell(1).pos()),
                    PipMode::Mux,
                );
            }
            for i in 0..6 {
                pips.pips.insert(
                    (wires::LCLK[o].cell(co), wires::RCLK_BUF[i].cell(1).pos()),
                    PipMode::Mux,
                );
            }
        }
    }
    for i in 0..12 {
        pips.pips.insert(
            (wires::HCLK_BUF[i].cell(1), wires::HCLK_ROW[i].cell(1).pos()),
            PipMode::Buf,
        );
    }
    for i in 0..6 {
        pips.pips.insert(
            (wires::RCLK_BUF[i].cell(1), wires::RCLK_ROW[i].cell(1).pos()),
            PipMode::Buf,
        );
    }

    for (tkn, naming_l, naming_r) in [
        ("HCLK_INNER_IOI", "HCLK_IO_IL", "HCLK_IO_IR"),
        ("HCLK_OUTER_IOI", "HCLK_IO_OL", "HCLK_IO_OR"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let is_l = rd.tile_kinds.key(rd.tiles[&xy.delta(-1, 0)].kind) == "HCLK_IOB";
            let hclk_xy = if is_l {
                if rd.tile_kinds.key(rd.tiles[&xy.delta(1, 0)].kind) == "HCLK_TERM" {
                    xy.delta(2, 0)
                } else {
                    xy.delta(1, 0)
                }
            } else {
                if rd.tile_kinds.key(rd.tiles[&xy.delta(-1, 0)].kind) == "HCLK_TERM" {
                    xy.delta(-3, 0)
                } else {
                    xy.delta(-2, 0)
                }
            };
            let intf_io =
                builder
                    .ndb
                    .get_tile_class_naming(if is_l { "INTF_IOI_L" } else { "INTF" });
            let mut bels = vec![];
            for i in 0..4 {
                bels.push(builder.bel_xy(bslots::BUFIO[i], "BUFIODQS", 0, i ^ 2));
            }
            for i in 0..2 {
                bels.push(builder.bel_xy(bslots::BUFR[i], "BUFR", 0, i ^ 1));
            }
            bels.push(builder.bel_xy(bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0));
            bels.push(builder.bel_xy(bslots::DCI, "DCI", 0, 0).pins_name_only(&[
                "DCIDATA",
                "DCIADDRESS0",
                "DCIADDRESS1",
                "DCIADDRESS2",
                "DCIIOUPDATE",
                "DCIREFIOUPDATE",
                "DCISCLK",
            ]));
            bels.push(
                builder
                    .bel_xy(bslots::HCLK_IO_INT, "BUFO", 0, 1)
                    .naming_only()
                    .pin_rename("I", "BUFO0_I")
                    .pin_rename("O", "BUFO0_O")
                    .pins_name_only(&["BUFO0_O", "BUFO0_I"])
                    .extra_wire("BUFO0_OCLK", &["HCLK_IOI_OCLK0"])
                    .sub_xy(rd, "BUFO", 0, 0)
                    .pin_rename("I", "BUFO1_I")
                    .pin_rename("O", "BUFO1_O")
                    .pins_name_only(&["BUFO1_O", "BUFO1_I"])
                    .extra_wire("BUFO1_OCLK", &["HCLK_IOI_OCLK1"]),
            );
            bels.push(builder.bel_virtual(bslots::BANK));
            builder
                .xtile_id(tcls::HCLK_IO, if is_l { naming_l } else { naming_r }, xy)
                .raw_tile(xy.delta(0, -2))
                .raw_tile(xy.delta(0, 1))
                .num_cells(8)
                .switchbox(bslots::HCLK_IO_INT)
                .optin_muxes(&wires::HCLK_IO[..])
                .optin_muxes(&wires::RCLK_IO[..])
                .optin_muxes(&wires::RCLK_ROW[..])
                .optin_muxes(&wires::IOCLK[..])
                .optin_muxes(&wires::OCLK[..])
                .optin_muxes(&wires::VOCLK[..])
                .optin_muxes(&wires::PERF_BUF[..])
                .optin_muxes(&wires::IMUX_BUFIO[..])
                .optin_muxes(&wires::IMUX_BUFR[..])
                .optin_muxes(&[wires::IMUX_IDELAYCTRL_REFCLK])
                .skip_edge("HCLK_IOI_IOCLK0", "HCLK_IOI_IOCLK0_DLY")
                .skip_edge("HCLK_IOI_IOCLK1", "HCLK_IOI_IOCLK1_DLY")
                .skip_edge("HCLK_IOI_IOCLK2", "HCLK_IOI_IOCLK2_DLY")
                .skip_edge("HCLK_IOI_IOCLK3", "HCLK_IOI_IOCLK3_DLY")
                .skip_edge("HCLK_IOI_IOCLKMULTI0", "HCLK_IOI_IOCLKMULTI0_DLY")
                .skip_edge("HCLK_IOI_IOCLKMULTI1", "HCLK_IOI_IOCLKMULTI1_DLY")
                .skip_edge("HCLK_IOI_IOCLKMULTI2", "HCLK_IOI_IOCLKMULTI2_DLY")
                .skip_edge("HCLK_IOI_IOCLKMULTI3", "HCLK_IOI_IOCLKMULTI3_DLY")
                .skip_edge("HCLK_IOI_OCLK0", "HCLK_IOI_BUFO0")
                .skip_edge("HCLK_IOI_OCLK1", "HCLK_IOI_BUFO1")
                .ref_int(hclk_xy.delta(0, -1), 3)
                .ref_int(hclk_xy.delta(0, 1), 4)
                .ref_single(hclk_xy.delta(1, -1), 3, intf_io)
                .ref_single(hclk_xy.delta(1, 1), 4, intf_io)
                .bels(bels)
                .extract();
        }
        for nn in [naming_l, naming_r] {
            let Some((_, naming)) = builder.ndb.tile_class_namings.get_mut(nn) else {
                continue;
            };
            for i in 0..8 {
                naming.delay_wires.insert(
                    wires::IOCLK[i].cell(4),
                    if i < 4 {
                        format!("HCLK_IOI_IOCLK{i}_DLY")
                    } else {
                        format!("HCLK_IOI_IOCLKMULTI{ii}_DLY", ii = i - 4)
                    },
                );
            }
        }
    }
    let pips = builder
        .pips
        .get_mut(&(tcls::HCLK_IO, bslots::HCLK_IO_INT))
        .unwrap();
    for i in 0..2 {
        pips.pips.insert(
            (
                wires::VIOCLK_S_BUF[i].cell(4),
                wires::VIOCLK_S[i].cell(4).pos(),
            ),
            PipMode::Buf,
        );
        pips.pips.insert(
            (
                wires::VIOCLK_N_BUF[i].cell(4),
                wires::VIOCLK_N[i].cell(4).pos(),
            ),
            PipMode::Buf,
        );
    }
    for i in 0..6 {
        pips.pips.insert(
            (wires::RCLK_ROW[i].cell(4), wires::PULLUP.cell(4).pos()),
            PipMode::Pass,
        );
    }
    for (&(wt, _), mode) in pips.pips.iter_mut() {
        if wires::IOCLK.contains(wt.wire) {
            *mode = PipMode::Delay
        } else if wt.wire != wires::IMUX_IDELAYCTRL_REFCLK
            && !wires::RCLK_ROW.contains(wt.wire)
            && !wires::IMUX_BUFR.contains(wt.wire)
            && !wires::IMUX_BUFIO.contains(wt.wire)
            && !wires::OCLK.contains(wt.wire)
        {
            *mode = PipMode::Buf;
        }
    }

    for tkn in ["LIOI", "RIOI"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = tkn == "LIOI";
            let lr = if is_l { 'L' } else { 'R' };
            let int_xy = if is_l {
                builder.walk_to_int(xy, Dir::E, false).unwrap()
            } else {
                builder.walk_to_int(xy, Dir::W, false).unwrap()
            };
            let intf_io =
                builder
                    .ndb
                    .get_tile_class_naming(if is_l { "INTF_IOI_L" } else { "INTF" });
            let mut bels = vec![];
            for i in 0..2 {
                let ii = i ^ 1;
                let mut bel = builder
                    .bel_xy(bslots::ILOGIC[i], "ILOGIC", 0, i)
                    .pins_name_only(&[
                        "OCLK",
                        "OCLKB",
                        "D",
                        "DDLY",
                        "OFB",
                        "TFB",
                        "SHIFTIN1",
                        "SHIFTIN2",
                        "SHIFTOUT1",
                        "SHIFTOUT2",
                        "REV",
                    ])
                    .extra_wire(
                        "IOB_I",
                        &[format!("LIOI_IBUF{ii}"), format!("RIOI_IBUF{ii}")],
                    )
                    .extra_wire("IOB_I_BUF", &[format!("LIOI_I{ii}"), format!("RIOI_I{ii}")]);
                if i == 1 {
                    bel = bel
                        .extra_int_out_force(
                            "CLKPAD",
                            wires::OUT_CLKPAD.cell(1),
                            format!("{lr}IOI_I_2IOCLK_BOT1"),
                        )
                        .extra_wire_force("CLKPAD_CMT", format!("{lr}IOI_I_2IOCLK_BOT1_I2GCLK"));
                }
                bels.push(bel);
            }
            for i in 0..2 {
                let ii = i ^ 1;
                bels.push(
                    builder
                        .bel_xy(bslots::OLOGIC[i], "OLOGIC", 0, i)
                        .pin_rename("CLK", "CLK_FAKE")
                        .pin_rename("CLKB", "CLKB_FAKE")
                        .pin_rename("TFB", "TFB_FAKE")
                        .pins_name_only(&[
                            "CLKPERFDELAY",
                            "CLK_FAKE",
                            "CLKB_FAKE",
                            "OFB",
                            "TFB_FAKE",
                            "TQ",
                            "OQ",
                            "SHIFTIN1",
                            "SHIFTIN2",
                            "SHIFTOUT1",
                            "SHIFTOUT2",
                            "REV",
                        ])
                        .extra_int_in("CLK", &[format!("IOI_OCLK_{ii}")])
                        .extra_int_in("CLKB", &[format!("IOI_OCLKM_{ii}")])
                        .extra_int_out(
                            "TFB",
                            &[
                                format!("LIOI_OLOGIC{ii}_TFB_LOCAL"),
                                format!("RIOI_OLOGIC{ii}_TFB_LOCAL"),
                            ],
                        )
                        .extra_wire("IOB_O", &[format!("LIOI_O{ii}"), format!("RIOI_O{ii}")])
                        .extra_wire("IOB_T", &[format!("LIOI_T{ii}"), format!("RIOI_T{ii}")]),
                );
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(bslots::IODELAY[i], "IODELAY", 0, i)
                        .pins_name_only(&["CLKIN", "IDATAIN", "ODATAIN", "DATAOUT", "T"]),
                );
            }
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(bslots::IOB[i], "IOB", 0, i)
                    .raw_tile(1)
                    .pins_name_only(&[
                        "I",
                        "O",
                        "T",
                        "PADOUT",
                        "DIFFI_IN",
                        "DIFFO_OUT",
                        "DIFFO_IN",
                        "O_OUT",
                        "O_IN",
                    ]);
                if i == 1 {
                    bel = bel.pins_name_only(&["DIFF_TERM_INT_EN"]);
                }
                let pn = if i == 1 { 'P' } else { 'N' };
                bel = bel.extra_wire_force("MONITOR", format!("{lr}IOB_MONITOR_{pn}"));
                bels.push(bel);
            }
            builder
                .xtile_id(tcls::IO, tkn, xy)
                .raw_tile(if is_l {
                    xy.delta(-1, 0)
                } else {
                    xy.delta(1, 0)
                })
                .num_cells(2)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&wires::IMUX_IOI_ICLK[..])
                .optin_muxes(&wires::IMUX_IOI_OCLK[..])
                .optin_muxes(&wires::IMUX_IOI_OCLKDIV[..])
                .optin_muxes(&[wires::IMUX_IOI_OCLKPERF])
                .optin_muxes(&wires::IMUX_SPEC[..])
                .ref_int(int_xy, 0)
                .ref_int(int_xy.delta(0, 1), 1)
                .ref_single(int_xy.delta(1, 0), 0, intf_io)
                .ref_single(int_xy.delta(1, 1), 1, intf_io)
                .bels(bels)
                .extract();
        }
    }
    let pips = builder.pips.get_mut(&(tcls::IO, bslots::SPEC_INT)).unwrap();
    for (&(wt, _), mode) in pips.pips.iter_mut() {
        if wires::IMUX_SPEC.contains(wt.wire) {
            *mode = PipMode::PermaBuf;
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER_0").iter().next() {
        let intf = builder.ndb.get_tile_class_naming("INTF");
        let mut bel_sysmon = builder
            .bel_xy(bslots::SYSMON, "SYSMON", 0, 0)
            .raw_tile(2)
            .pins_name_only(&["VP", "VN"]);
        for i in 0..16 {
            bel_sysmon = bel_sysmon
                .pin_name_only(&format!("VAUXP{i}"), 1)
                .pin_name_only(&format!("VAUXN{i}"), 1);
        }
        bel_sysmon = bel_sysmon
            .sub_xy(rd, "IPAD", 0, 0)
            .raw_tile(2)
            .pin_rename("O", "IPAD_VP_O")
            .pins_name_only(&["IPAD_VP_O"])
            .sub_xy(rd, "IPAD", 0, 1)
            .raw_tile(2)
            .pin_rename("O", "IPAD_VN_O")
            .pins_name_only(&["IPAD_VN_O"]);
        let bels = [
            builder.bel_xy(bslots::BSCAN[0], "BSCAN", 0, 0).raw_tile(1),
            builder.bel_xy(bslots::BSCAN[1], "BSCAN", 0, 1).raw_tile(1),
            builder.bel_xy(bslots::BSCAN[2], "BSCAN", 0, 0).raw_tile(2),
            builder.bel_xy(bslots::BSCAN[3], "BSCAN", 0, 1).raw_tile(2),
            builder.bel_xy(bslots::ICAP[0], "ICAP", 0, 0).raw_tile(1),
            builder.bel_xy(bslots::ICAP[1], "ICAP", 0, 0).raw_tile(2),
            builder.bel_xy(bslots::PMV_CFG[0], "PMV", 0, 0).raw_tile(0),
            builder.bel_xy(bslots::PMV_CFG[1], "PMV", 0, 0).raw_tile(3),
            builder.bel_xy(bslots::STARTUP, "STARTUP", 0, 0).raw_tile(1),
            builder.bel_xy(bslots::CAPTURE, "CAPTURE", 0, 0).raw_tile(1),
            builder
                .bel_single(bslots::FRAME_ECC, "FRAME_ECC")
                .raw_tile(1),
            builder
                .bel_xy(bslots::EFUSE_USR, "EFUSE_USR", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(bslots::USR_ACCESS, "USR_ACCESS", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(bslots::DNA_PORT, "DNA_PORT", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(bslots::DCIRESET, "DCIRESET", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(bslots::CFG_IO_ACCESS, "CFG_IO_ACCESS", 0, 0)
                .raw_tile(1),
            bel_sysmon,
            builder.bel_virtual(bslots::MISC_CFG),
        ];
        let mut xn = builder
            .xtile_id(tcls::CFG, "CFG", xy)
            .num_cells(80)
            .raw_tile(xy.delta(0, 21))
            .raw_tile(xy.delta(0, 42))
            .raw_tile(xy.delta(0, 63));
        for i in 0..80 {
            let int_xy = xy.delta(2, -10 + (i + i / 20) as i32);
            xn = xn
                .ref_int(int_xy, i)
                .ref_single(int_xy.delta(1, 0), i, intf);
        }
        xn.bels(bels).extract();
    }

    for (tkn, naming) in [("HCLK_CMT_BOT", "CMT.BOT"), ("HCLK_CMT_TOP", "CMT.TOP")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming("INTF");
            let xy_bot = xy.delta(0, -9);
            let xy_top = xy.delta(0, 10);
            let mut bels = vec![];
            for i in 0..2 {
                let slots = [bslots::BUFHCE_W, bslots::BUFHCE_E][i];
                for j in 0..12 {
                    bels.push(builder.bel_xy(slots[j], "BUFHCE", i, j).raw_tile(2));
                }
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(bslots::PLL[i], "MMCM_ADV", 0, 0)
                        .raw_tile(i)
                        .extra_wire("CLKFB", &["CMT_MMCM_CLKFB"])
                        .extra_int_in("CLKIN_CASC", &["CMT_MMCM_CASC_IN"])
                        .extra_int_in("CLKFB_CASC", &["CMT_MMCM_CASC_OUT"]),
                );
            }
            bels.push(
                builder
                    .bel_xy(bslots::PPR_FRAME, "PPR_FRAME", 0, 0)
                    .raw_tile(1),
            );

            let mut bel = builder
                .bel_virtual(bslots::SPEC_INT)
                .naming_only()
                .raw_tile(2)
                .extra_int_in("BUFH_TEST_L_PRE", &["HCLK_CMT_CK_BUFH_TEST_OUT_L"])
                .extra_wire("BUFH_TEST_L_INV", &["HCLK_CMT_CK_BUFH_TEST_INV_L"])
                .extra_wire("BUFH_TEST_L_NOINV", &["HCLK_CMT_CK_BUFH_TEST_NOINV_L"])
                .extra_int_out("BUFH_TEST_L", &["HCLK_CMT_CK_BUFH_TEST_L"])
                .extra_int_in("BUFH_TEST_R_PRE", &["HCLK_CMT_CK_BUFH_TEST_OUT_R"])
                .extra_wire("BUFH_TEST_R_INV", &["HCLK_CMT_CK_BUFH_TEST_INV_R"])
                .extra_wire("BUFH_TEST_R_NOINV", &["HCLK_CMT_CK_BUFH_TEST_NOINV_R"])
                .extra_int_out("BUFH_TEST_R", &["HCLK_CMT_CK_BUFH_TEST_R"]);
            for i in 0..32 {
                bel = bel
                    .extra_int_in(format!("GCLK{i}"), &[format!("HCLK_CMT_CK_GCLK{i}")])
                    .extra_wire(
                        format!("GCLK{i}_INV"),
                        &[format!("HCLK_CMT_CK_GCLK_INV_TEST{i}")],
                    )
                    .extra_wire(
                        format!("GCLK{i}_NOINV"),
                        &[format!("HCLK_CMT_CK_GCLK_NOINV_TEST{i}")],
                    )
                    .extra_int_out(
                        format!("GCLK{i}_TEST"),
                        &[format!("HCLK_CMT_CK_GCLK_TEST{i}")],
                    );
            }
            bels.push(bel);
            let xy_qw = rd
                .tiles_by_kind_name("HCLK_QBUF_L")
                .iter()
                .copied()
                .find(|xy_qw| xy_qw.y == xy.y)
                .unwrap();
            let xy_qe = rd
                .tiles_by_kind_name("HCLK_QBUF_R")
                .iter()
                .copied()
                .find(|xy_qw| xy_qw.y == xy.y)
                .unwrap();
            let mut xn = builder
                .xtile_id(tcls::CMT, naming, xy_bot)
                .num_cells(56)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&[wires::BUFH_TEST_W_IN, wires::BUFH_TEST_E_IN])
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .optin_muxes(&wires::BUFH_INT_W[..])
                .optin_muxes(&wires::BUFH_INT_E[..])
                .optin_muxes(&wires::IMUX_BUFHCE_W[..])
                .optin_muxes(&wires::IMUX_BUFHCE_E[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN1[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN2[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN1_HCLK[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN2_HCLK[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN1_HCLK_W[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN2_HCLK_W[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN1_HCLK_E[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN2_HCLK_E[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN1_IO[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN2_IO[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN1_MGT[..])
                .optin_muxes(&wires::IMUX_PLL_CLKIN2_MGT[..])
                .optin_muxes(&wires::IMUX_PLL_CLKFB[..])
                .optin_muxes(&wires::IMUX_PLL_CLKFB_HCLK[..])
                .optin_muxes(&wires::IMUX_PLL_CLKFB_HCLK_W[..])
                .optin_muxes(&wires::IMUX_PLL_CLKFB_HCLK_E[..])
                .optin_muxes(&wires::IMUX_PLL_CLKFB_IO[..])
                .optin_muxes(&wires::OMUX_PLL_CASC[..])
                .optin_muxes(&wires::OMUX_PLL_PERF_S[..])
                .optin_muxes(&wires::OMUX_PLL_PERF_N[..])
                .optin_muxes(&wires::PERF_ROW[..])
                .optin_muxes(&wires::PERF_ROW_OUTER[..])
                .skip_edge("CMT_MMCM_CLKFBIN", "CMT_MMCM_CLKFB")
                .skip_edge("CMT_MMCM_CLKFBIN", "CMT_MMCM_CASC_OUT")
                .skip_edge("CMT_MMCM_CLKIN1", "CMT_MMCM_CASC_IN")
                .raw_tile(xy_top)
                .raw_tile(xy)
                .raw_tile_xlat(xy_qw, &[None, Some(44)])
                .raw_tile_xlat(xy_qe, &[None, Some(52)]);
            for i in 0..20 {
                xn = xn.ref_int(xy_bot.delta(-2, -11 + i as i32), i).ref_single(
                    xy_bot.delta(-1, -11 + i as i32),
                    i,
                    intf,
                )
            }
            for i in 0..20 {
                xn = xn
                    .ref_int(xy_top.delta(-2, -9 + i as i32), i + 20)
                    .ref_single(xy_top.delta(-1, -9 + i as i32), i + 20, intf)
            }
            xn.bels(bels).extract();
        }
    }
    let pips = builder
        .pips
        .get_mut(&(tcls::CMT, bslots::SPEC_INT))
        .unwrap();
    for ((wt, _wf), mode) in pips.pips.iter_mut() {
        if wires::BUFH_INT_W.contains(wt.wire)
            || wires::BUFH_INT_E.contains(wt.wire)
            || wires::PERF_ROW.contains(wt.wire)
            || wires::PERF_ROW_OUTER.contains(wt.wire)
        {
            *mode = PipMode::Buf;
        }
    }
    for i in 0..4 {
        pips.pips.insert(
            (
                wires::CCIO_CMT_W[i].cell(20),
                wires::OUT_CLKPAD.cell(41 + i * 2).pos(),
            ),
            PipMode::Buf,
        );
        pips.pips.insert(
            (
                wires::CCIO_CMT_E[i].cell(20),
                wires::OUT_CLKPAD.cell(49 + i * 2).pos(),
            ),
            PipMode::Buf,
        );
    }
    for i in 0..32 {
        pips.pips.insert(
            (wires::GCLK_CMT[i].cell(20), wires::GCLK[i].cell(20).pos()),
            PipMode::Buf,
        );
        pips.pips.insert(
            (
                wires::GCLK_TEST_IN[i].cell(20),
                wires::GCLK_CMT[i].cell(20).pos(),
            ),
            PipMode::Buf,
        );
        pips.specials.insert(SwitchBoxItem::ProgInv(ProgInv {
            dst: wires::GCLK_TEST[i].cell(20),
            src: wires::GCLK_TEST_IN[i].cell(20),
            bit: PolTileBit::DUMMY,
        }));
    }
    for i in 0..8 {
        pips.pips.insert(
            (wires::GIOB_CMT[i].cell(20), wires::GIOB[i].cell(20).pos()),
            PipMode::Buf,
        );
    }
    for i in 0..12 {
        pips.pips.insert(
            (
                wires::HCLK_CMT_W[i].cell(20),
                wires::HCLK_ROW[i].cell(44).pos(),
            ),
            PipMode::Buf,
        );
        pips.pips.insert(
            (
                wires::HCLK_CMT_E[i].cell(20),
                wires::HCLK_ROW[i].cell(52).pos(),
            ),
            PipMode::Buf,
        );
    }
    for i in 0..6 {
        pips.pips.insert(
            (
                wires::RCLK_CMT_W[i].cell(20),
                wires::RCLK_ROW[i].cell(44).pos(),
            ),
            PipMode::Buf,
        );
        pips.pips.insert(
            (
                wires::RCLK_CMT_E[i].cell(20),
                wires::RCLK_ROW[i].cell(52).pos(),
            ),
            PipMode::Buf,
        );
    }
    for i in 0..10 {
        pips.pips.insert(
            (
                wires::MGT_CMT_W[i].cell(20),
                wires::MGT_ROW[i].cell(44).pos(),
            ),
            PipMode::Buf,
        );
        pips.pips.insert(
            (
                wires::MGT_CMT_E[i].cell(20),
                wires::MGT_ROW[i].cell(52).pos(),
            ),
            PipMode::Buf,
        );
    }

    for tkn in ["CMT_PMVA", "CMT_PMVA_BELOW"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming("INTF");
            let bel = builder.bel_xy(bslots::PMVIOB_CLK, "PMVIOB", 0, 0);
            builder
                .xtile_id(tcls::PMVIOB, tkn, xy)
                .num_cells(2)
                .ref_int(xy.delta(-2, 0), 0)
                .ref_int(xy.delta(-2, 1), 1)
                .ref_single(xy.delta(-1, 0), 0, intf)
                .ref_single(xy.delta(-1, 1), 1, intf)
                .bel(bel)
                .extract();
        }
    }

    for (tcid, naming, tkn) in [
        (tcls::CLK_BUFG_S, "CLK_BUFG_S", "CMT_BUFG_BOT"),
        (tcls::CLK_BUFG_N, "CLK_BUFG_N", "CMT_BUFG_TOP"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming("INTF");
            let mut bels = vec![];
            let is_s = tcid == tcls::CLK_BUFG_S;
            let bi = if is_s { 0 } else { 16 };
            let int_xy = xy.delta(-2, if is_s { -1 } else { 0 });
            let cmt_xy = xy.delta(0, if is_s { -9 } else { 11 });
            for i in 0..16 {
                let ii = bi + i;
                bels.push(builder.bel_xy(bslots::BUFGCTRL[ii], "BUFGCTRL", 0, i));
            }
            let mut bel = builder.bel_virtual(bslots::SPEC_INT).naming_only();
            for i in 0..8 {
                bel = bel.extra_wire(
                    format!("GIO{i}_BUFG"),
                    &[
                        format!("CMT_BUFG_BOT_CK_IO_TO_BUFG{i}"),
                        format!("CMT_BUFG_TOP_CK_IO_TO_BUFG{i}"),
                    ],
                );
            }
            if is_s {
                for i in 0..4 {
                    bel = bel
                        .extra_wire(format!("GIO{i}"), &[format!("CMT_BUFG_BOT_CK_PADIN{i}")])
                        .extra_wire(
                            format!("GIO{i}_CMT"),
                            &[
                                format!("CMT_BUFG_BOT_CK_IO_TO_CMT{i}"),
                                format!("CMT_BUFG_TOP_CK_IO_TO_CMT{i}"),
                            ],
                        );
                }
            } else {
                for i in 4..8 {
                    bel = bel
                        .extra_wire(format!("GIO{i}"), &[format!("CMT_BUFG_TOP_CK_PADIN{i}")])
                        .extra_wire(
                            format!("GIO{i}_CMT"),
                            &[
                                format!("CMT_BUFG_BOT_CK_IO_TO_CMT{i}"),
                                format!("CMT_BUFG_TOP_CK_IO_TO_CMT{i}"),
                            ],
                        );
                }
            }
            bels.push(bel);
            let mut xt = builder
                .xtile_id(tcid, naming, xy)
                .raw_tile(cmt_xy)
                .num_cells(7)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&wires::GIOB[..])
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .optin_muxes(&wires::OUT_BUFG_GFB[..])
                .optin_muxes(&wires::GCLK[..])
                .ref_int(int_xy, 0)
                .ref_int(int_xy.delta(0, 1), 1)
                .ref_int(int_xy.delta(0, 2), 2)
                .ref_single(int_xy.delta(1, 0), 0, intf)
                .ref_single(int_xy.delta(1, 1), 1, intf)
                .ref_single(int_xy.delta(1, 2), 2, intf)
                .bels(bels);
            for i in 0..16 {
                for j in 0..2 {
                    xt = xt.skip_edge(
                        &format!("CMT_BUFG_BUFGCTRL{i}_I{j}"),
                        &format!("CMT_BUFG_CK_FB_TEST{j}_{i}"),
                    );
                }
            }
            xt.extract();
        }
        let pips = builder.pips.get_mut(&(tcid, bslots::SPEC_INT)).unwrap();
        let naming = builder.ndb.tile_class_namings.get_mut(naming).unwrap().1;

        let base = if tcid == tcls::CLK_BUFG_S { 1 } else { 0 };
        for i in 0..32 {
            let c = base + i / 16;
            let o = [4, 8, 12, 0, 1, 13, 9, 5, 6, 10, 14, 2, 3, 15, 11, 7][i % 16];
            let wt = wires::OUT_BEL[o].cell(c);
            let wf = wires::IMUX_BUFG_O[i].cell(base);
            pips.pips.insert((wt, wf.pos()), PipMode::Buf);
            naming.wires.insert(
                wt,
                WireNaming {
                    name: format!("CMT_BUFG_LOGIC_OUTS{o}_{cc}", cc = i / 16),
                    alt_name: Some(format!("CMT_BUFG_CK_FB_TEST{k}_{j}", j = i / 2, k = i % 2)),
                    alt_pips_to: Default::default(),
                    alt_pips_from: BTreeSet::from_iter([wf]),
                },
            );
        }

        for ((wt, _wf), mode) in pips.pips.iter_mut() {
            if wires::OUT_BUFG_GFB.contains(wt.wire) {
                *mode = PipMode::Buf;
            }
        }
    }

    for (tkn, nn) in [("HCLK_GTX", "GTX"), ("HCLK_GTX_LEFT", "GTX_LEFT")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = tkn == "HCLK_GTX_LEFT";
            let int_xy = xy.delta(if is_l { 2 } else { -3 }, -20);
            let intf_gt =
                builder
                    .ndb
                    .get_tile_class_naming(if is_l { "INTF_GT_L" } else { "INTF_GTX" });
            let mut bel_hclk_gtx = builder
                .bel_virtual(bslots::HCLK_GTX)
                .extra_wire(
                    "MGTREFCLKOUT0",
                    &["HCLK_GTX_MGTREFCLKOUT0", "HCLK_GTX_LEFT_MGTREFCLKOUT0"],
                )
                .extra_wire(
                    "MGTREFCLKOUT1",
                    &["HCLK_GTX_MGTREFCLKOUT1", "HCLK_GTX_LEFT_MGTREFCLKOUT1"],
                )
                .extra_wire(
                    "MGTREFCLKIN0",
                    &["HCLK_GTX_MGTREFCLKIN0", "HCLK_GTX_LEFT_MGTREFCLKIN0"],
                )
                .extra_wire(
                    "MGTREFCLKIN1",
                    &["HCLK_GTX_MGTREFCLKIN1", "HCLK_GTX_LEFT_MGTREFCLKIN1"],
                )
                .extra_wire(
                    "SOUTHREFCLKIN0",
                    &["HCLK_GTX_SOUTHREFCLKIN0", "HCLK_GTX_LEFT_SOUTHREFCLKIN0"],
                )
                .extra_wire(
                    "SOUTHREFCLKIN1",
                    &["HCLK_GTX_SOUTHREFCLKIN1", "HCLK_GTX_LEFT_SOUTHREFCLKIN1"],
                )
                .extra_wire(
                    "NORTHREFCLKIN0",
                    &["HCLK_GTX_NORTHREFCLKIN0", "HCLK_GTX_LEFT_NORTHREFCLKIN0"],
                )
                .extra_wire(
                    "NORTHREFCLKIN1",
                    &["HCLK_GTX_NORTHREFCLKIN1", "HCLK_GTX_LEFT_NORTHREFCLKIN1"],
                )
                .extra_wire(
                    "SOUTHREFCLKOUT0",
                    &["HCLK_GTX_SOUTHREFCLKOUT0", "HCLK_GTX_LEFT_SOUTHREFCLKOUT0"],
                )
                .extra_wire(
                    "SOUTHREFCLKOUT1",
                    &["HCLK_GTX_SOUTHREFCLKOUT1", "HCLK_GTX_LEFT_SOUTHREFCLKOUT1"],
                )
                .extra_wire(
                    "NORTHREFCLKOUT0",
                    &["HCLK_GTX_NORTHREFCLKOUT0", "HCLK_GTX_LEFT_NORTHREFCLKOUT0"],
                )
                .extra_wire(
                    "NORTHREFCLKOUT1",
                    &["HCLK_GTX_NORTHREFCLKOUT1", "HCLK_GTX_LEFT_NORTHREFCLKOUT1"],
                );
            for i in 0..4 {
                bel_hclk_gtx = bel_hclk_gtx
                    .extra_wire(
                        format!("RXRECCLK{i}"),
                        &[
                            format!("HCLK_GTX_RXRECCLK{i}"),
                            format!("HCLK_GTX_LEFT_RXRECCLK{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("TXOUTCLK{i}"),
                        &[
                            format!("HCLK_GTX_TXOUTCLK{i}"),
                            format!("HCLK_GTX_LEFT_TXOUTCLK{i}"),
                        ],
                    );
            }
            let mut bels = vec![];
            for i in 0..4 {
                bels.extend([
                    builder
                        .bel_xy(bslots::IPAD_RXP[i], "IPAD", 0, 1)
                        .raw_tile(i + 1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(bslots::IPAD_RXN[i], "IPAD", 0, 0)
                        .raw_tile(i + 1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(bslots::OPAD_TXP[i], "OPAD", 0, 1)
                        .raw_tile(i + 1)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_xy(bslots::OPAD_TXN[i], "OPAD", 0, 0)
                        .raw_tile(i + 1)
                        .pins_name_only(&["I"]),
                ]);
            }
            bels.extend([
                builder
                    .bel_xy(bslots::IPAD_CLKP[0], "IPAD", 0, 2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bslots::IPAD_CLKN[0], "IPAD", 0, 3)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bslots::IPAD_CLKP[1], "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bslots::IPAD_CLKN[1], "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
            ]);
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(bslots::GTX[i], "GTXE1", 0, 0)
                        .raw_tile(i + 1)
                        .pins_name_only(&[
                            "RXP",
                            "RXN",
                            "TXP",
                            "TXN",
                            "MGTREFCLKRX0",
                            "MGTREFCLKRX1",
                            "MGTREFCLKTX0",
                            "MGTREFCLKTX1",
                            "NORTHREFCLKRX0",
                            "NORTHREFCLKRX1",
                            "NORTHREFCLKTX0",
                            "NORTHREFCLKTX1",
                            "SOUTHREFCLKRX0",
                            "SOUTHREFCLKRX1",
                            "SOUTHREFCLKTX0",
                            "SOUTHREFCLKTX1",
                        ])
                        .extra_wire(
                            "MGTREFCLKOUT0",
                            &["GTX_MGTREFCLKOUT0", "GTX_LEFT_MGTREFCLKOUT0"],
                        )
                        .extra_wire(
                            "MGTREFCLKOUT1",
                            &["GTX_MGTREFCLKOUT1", "GTX_LEFT_MGTREFCLKOUT1"],
                        )
                        .extra_wire(
                            "NORTHREFCLKIN0",
                            &["GTX_NORTHREFCLKIN0", "GTX_LEFT_NORTHREFCLKIN0"],
                        )
                        .extra_wire(
                            "NORTHREFCLKIN1",
                            &["GTX_NORTHREFCLKIN1", "GTX_LEFT_NORTHREFCLKIN1"],
                        )
                        .extra_wire(
                            "SOUTHREFCLKOUT0",
                            &["GTX_SOUTHREFCLKOUT0", "GTX_LEFT_SOUTHREFCLKOUT0"],
                        )
                        .extra_wire(
                            "SOUTHREFCLKOUT1",
                            &["GTX_SOUTHREFCLKOUT1", "GTX_LEFT_SOUTHREFCLKOUT1"],
                        ),
                );
            }
            bels.extend([
                builder
                    .bel_xy(bslots::BUFDS[0], "IBUFDS_GTXE1", 0, 0)
                    .pins_name_only(&["O", "ODIV2", "I", "IB", "CLKTESTSIG"])
                    .extra_int_out(
                        "HCLK_OUT",
                        &["HCLK_GTX_REFCLKHROW0", "HCLK_GTX_LEFT_REFCLKHROW0"],
                    )
                    .extra_int_in(
                        "CLKTESTSIG_INT",
                        &[
                            "IBUFDS_GTXE1_0_CLKTESTSIG_SEG",
                            // sigh. that is an O.
                            "IBUFDS_GTXE1_LEFT_O_CLKTESTSIG_SEG",
                        ],
                    ),
                builder
                    .bel_xy(bslots::BUFDS[1], "IBUFDS_GTXE1", 0, 1)
                    .pins_name_only(&["O", "ODIV2", "I", "IB", "CLKTESTSIG"])
                    .extra_int_out(
                        "HCLK_OUT",
                        &["HCLK_GTX_REFCLKHROW1", "HCLK_GTX_LEFT_REFCLKHROW1"],
                    )
                    .extra_int_in(
                        "CLKTESTSIG_INT",
                        &[
                            "IBUFDS_GTXE1_1_CLKTESTSIG_SEG",
                            "IBUFDS_GTXE1_LEFT_1_CLKTESTSIG_SEG",
                        ],
                    ),
                bel_hclk_gtx,
            ]);
            let mut xn = builder
                .xtile_id(tcls::GTX, nn, xy)
                .num_cells(40)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&[wires::IMUX_GTX_PERFCLK])
                .raw_tile(xy.delta(0, -20))
                .raw_tile(xy.delta(0, -10))
                .raw_tile(xy.delta(0, 1))
                .raw_tile(xy.delta(0, 11));
            for i in 0..40 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 20) as i32), i)
                    .ref_single(int_xy.delta(1, (i + i / 20) as i32), i, intf_gt);
            }
            xn.bels(bels).extract();
        }
    }
    for tkn in ["HCLK_GTH_LEFT", "HCLK_GTH"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = tkn == "HCLK_GTH_LEFT";
            let int_xy = xy.delta(if is_l { 2 } else { -3 }, 0);
            let intf_gt =
                builder
                    .ndb
                    .get_tile_class_naming(if is_l { "INTF_GT_L" } else { "INTF_GTX" });
            let xy_bot = xy.delta(0, -10);
            let xy_top = xy.delta(0, 11);
            let mut bels = vec![];
            for i in 0..4 {
                bels.extend([
                    builder
                        .bel_xy(bslots::IPAD_RXP[i], "IPAD", 0, (3 - i) * 2 + 1)
                        .raw_tile(1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(bslots::IPAD_RXN[i], "IPAD", 0, (3 - i) * 2)
                        .raw_tile(1)
                        .pins_name_only(&["O"]),
                ]);
            }
            for i in 0..4 {
                bels.extend([
                    builder
                        .bel_xy(bslots::OPAD_TXP[i], "OPAD", 0, (3 - i) * 2 + 1)
                        .raw_tile(1)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_xy(bslots::OPAD_TXN[i], "OPAD", 0, (3 - i) * 2)
                        .raw_tile(1)
                        .pins_name_only(&["I"]),
                ]);
            }
            bels.extend([
                builder
                    .bel_xy(bslots::IPAD_CLKP[0], "IPAD", 0, 1)
                    .raw_tile(2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bslots::IPAD_CLKN[0], "IPAD", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["O"]),
            ]);
            let mut bel_gt = builder
                .bel_xy(bslots::GTH_QUAD, "GTHE1_QUAD", 0, 0)
                .raw_tile(1)
                .pin_name_only("REFCLK", 1)
                .extra_int_in("GREFCLK", &["GTH_LEFT_GREFCLK", "GTHE1_RIGHT_GREFCLK"])
                .extra_wire(
                    "REFCLK_IN",
                    &["GTH_LEFT_IBUF_OUTCLK", "GTHE1_RIGHT_IBUF_OUTCLK"],
                )
                .extra_wire(
                    "REFCLK_SOUTH",
                    &["GTH_LEFT_REFCLKSOUTHIN", "GTHE1_RIGHT_REFCLKSOUTHIN"],
                )
                .extra_wire(
                    "REFCLK_NORTH",
                    &["GTH_LEFT_REFCLKNORTHIN", "GTHE1_RIGHT_REFCLKNORTHIN"],
                )
                .extra_wire("REFCLK_UP", &["GTH_TOP_REFCLKUP", "GTH_LEFT_REFCLK_NORTH"])
                .extra_wire("REFCLK_DN", &["GTH_TOP_REFCLKDN", "GTH_LEFT_REFCLK_SOUTH"]);
            for i in 0..4 {
                bel_gt = bel_gt.pins_name_only(&[
                    format!("RXP{i}"),
                    format!("RXN{i}"),
                    format!("TXP{i}"),
                    format!("TXN{i}"),
                ]);
            }
            bels.push(bel_gt);
            bels.push(
                builder
                    .bel_xy(bslots::BUFDS[0], "IBUFDS_GTHE1", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["I", "IB"])
                    .pin_name_only("O", 1),
            );

            let mut xn = builder
                .xtile_id(tcls::GTH, if is_l { "GTH_W" } else { "GTH_E" }, xy_bot)
                .num_cells(40)
                .raw_tile(xy_top)
                .raw_tile(xy);
            for i in 0..20 {
                xn = xn.ref_int(int_xy.delta(0, -20 + i as i32), i).ref_single(
                    int_xy.delta(1, -20 + i as i32),
                    i,
                    intf_gt,
                )
            }
            for i in 0..20 {
                xn = xn
                    .ref_int(int_xy.delta(0, 1 + i as i32), i + 20)
                    .ref_single(int_xy.delta(1, 1 + i as i32), i + 20, intf_gt)
            }
            xn.bels(bels).extract();
        }
    }

    builder.build()
}
