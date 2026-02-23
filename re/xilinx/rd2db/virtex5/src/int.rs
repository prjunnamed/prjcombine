use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, IntDb, PairMux, SwitchBoxItem, TileWireCoord, WireSlotIdExt, WireSupport},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{IntfWireInNaming, NamingDb, PipNaming, RawTileId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_virtex4::defs::{
    self, bslots,
    virtex5::{ccls, tcls, wires},
};

use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::virtex5::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => ccls::PASS_W,
        Dir::E => ccls::PASS_E,
        Dir::S => ccls::PASS_S,
        Dir::N => ccls::PASS_N,
    }));

    builder.wire_names(wires::PULLUP, &["KEEP1_WIRE"]);
    builder.wire_names(wires::TIE_0, &["GND_WIRE"]);
    builder.wire_names(wires::TIE_1, &["VCC_WIRE"]);

    for i in 0..10 {
        builder.wire_names(wires::HCLK[i], &[format!("GCLK{i}")]);
    }
    for i in 0..4 {
        builder.wire_names(wires::RCLK[i], &[format!("RCLK{i}")]);
    }

    for (name, base, w0, w1, w2) in [
        (
            "WL",
            0,
            &wires::DBL_WW0[..],
            &wires::DBL_WW1[..],
            &wires::DBL_WW2[..],
        ),
        (
            "WR",
            3,
            &wires::DBL_WW0[..],
            &wires::DBL_WW1[..],
            &wires::DBL_WW2[..],
        ),
        (
            "WS",
            0,
            &wires::DBL_WS0[..],
            &wires::DBL_WS1[..],
            &wires::DBL_WS2[..],
        ),
        (
            "WN",
            0,
            &wires::DBL_WN0[..],
            &wires::DBL_WN1[..],
            &wires::DBL_WN2[..],
        ),
        (
            "EL",
            0,
            &wires::DBL_EE0[..],
            &wires::DBL_EE1[..],
            &wires::DBL_EE2[..],
        ),
        (
            "ER",
            3,
            &wires::DBL_EE0[..],
            &wires::DBL_EE1[..],
            &wires::DBL_EE2[..],
        ),
        (
            "ES",
            0,
            &wires::DBL_ES0[..],
            &wires::DBL_ES1[..],
            &wires::DBL_ES2[..],
        ),
        (
            "EN",
            0,
            &wires::DBL_EN0[..],
            &wires::DBL_EN1[..],
            &wires::DBL_EN2[..],
        ),
        (
            "NL",
            0,
            &wires::DBL_NN0[..],
            &wires::DBL_NN1[..],
            &wires::DBL_NN2[..],
        ),
        (
            "NR",
            3,
            &wires::DBL_NN0[..],
            &wires::DBL_NN1[..],
            &wires::DBL_NN2[..],
        ),
        (
            "NW",
            0,
            &wires::DBL_NW0[..],
            &wires::DBL_NW1[..],
            &wires::DBL_NW2[..],
        ),
        (
            "NE",
            0,
            &wires::DBL_NE0[..],
            &wires::DBL_NE1[..],
            &wires::DBL_NE2[..],
        ),
        (
            "SL",
            0,
            &wires::DBL_SS0[..],
            &wires::DBL_SS1[..],
            &wires::DBL_SS2[..],
        ),
        (
            "SR",
            3,
            &wires::DBL_SS0[..],
            &wires::DBL_SS1[..],
            &wires::DBL_SS2[..],
        ),
        (
            "SW",
            0,
            &wires::DBL_SW0[..],
            &wires::DBL_SW1[..],
            &wires::DBL_SW2[..],
        ),
        (
            "SE",
            0,
            &wires::DBL_SE0[..],
            &wires::DBL_SE1[..],
            &wires::DBL_SE2[..],
        ),
    ] {
        for i in 0..3 {
            builder.wire_names(w0[base + i], &[format!("{name}2BEG{i}")]);
            builder.wire_names(w1[base + i], &[format!("{name}2MID{i}")]);
            builder.wire_names(w2[base + i], &[format!("{name}2END{i}")]);
        }
    }
    builder.wire_names(wires::DBL_WW0_S0, &["WL2BEG_S0"]);
    builder.wire_names(wires::DBL_WW0_N5, &["WR2BEG_N2"]);
    builder.wire_names(wires::DBL_WS1_BUF0, &["WS2MID_FAKE0"]);
    builder.wire_names(wires::DBL_WS1_S0, &["WS2MID_S0"]);
    builder.wire_names(wires::DBL_WN2_S0, &["WN2END_S0"]);
    builder.wire_names(wires::DBL_EE0_S3, &["ER2BEG_S0"]);
    builder.wire_names(wires::DBL_NN0_S0, &["NL2BEG_S0"]);
    builder.wire_names(wires::DBL_NN0_N5, &["NR2BEG_N2"]);
    builder.wire_names(wires::DBL_NE1_BUF2, &["NE2MID_FAKE2"]);
    builder.wire_names(wires::DBL_NE1_N2, &["NE2MID_N2"]);
    builder.wire_names(wires::DBL_NW2_N2, &["NW2END_N2"]);
    builder.wire_names(wires::DBL_SS0_N2, &["SL2BEG_N2"]);
    builder.mark_permabuf(wires::DBL_WS1_BUF0);
    builder.mark_permabuf(wires::DBL_NE1_BUF2);

    for (name, base, w0, w1, w2, w3, w4, w5) in [
        (
            "WL",
            0,
            &wires::PENT_WW0[..],
            &wires::PENT_WW1[..],
            &wires::PENT_WW2[..],
            &wires::PENT_WW3[..],
            &wires::PENT_WW4[..],
            &wires::PENT_WW5[..],
        ),
        (
            "WR",
            3,
            &wires::PENT_WW0[..],
            &wires::PENT_WW1[..],
            &wires::PENT_WW2[..],
            &wires::PENT_WW3[..],
            &wires::PENT_WW4[..],
            &wires::PENT_WW5[..],
        ),
        (
            "WS",
            0,
            &wires::PENT_WS0[..],
            &wires::PENT_WS1[..],
            &wires::PENT_WS2[..],
            &wires::PENT_WS3[..],
            &wires::PENT_WS4[..],
            &wires::PENT_WS5[..],
        ),
        (
            "WN",
            0,
            &wires::PENT_WN0[..],
            &wires::PENT_WN1[..],
            &wires::PENT_WN2[..],
            &wires::PENT_WN3[..],
            &wires::PENT_WN4[..],
            &wires::PENT_WN5[..],
        ),
        (
            "EL",
            0,
            &wires::PENT_EE0[..],
            &wires::PENT_EE1[..],
            &wires::PENT_EE2[..],
            &wires::PENT_EE3[..],
            &wires::PENT_EE4[..],
            &wires::PENT_EE5[..],
        ),
        (
            "ER",
            3,
            &wires::PENT_EE0[..],
            &wires::PENT_EE1[..],
            &wires::PENT_EE2[..],
            &wires::PENT_EE3[..],
            &wires::PENT_EE4[..],
            &wires::PENT_EE5[..],
        ),
        (
            "ES",
            0,
            &wires::PENT_ES0[..],
            &wires::PENT_ES1[..],
            &wires::PENT_ES2[..],
            &wires::PENT_ES3[..],
            &wires::PENT_ES4[..],
            &wires::PENT_ES5[..],
        ),
        (
            "EN",
            0,
            &wires::PENT_EN0[..],
            &wires::PENT_EN1[..],
            &wires::PENT_EN2[..],
            &wires::PENT_EN3[..],
            &wires::PENT_EN4[..],
            &wires::PENT_EN5[..],
        ),
        (
            "NL",
            0,
            &wires::PENT_NN0[..],
            &wires::PENT_NN1[..],
            &wires::PENT_NN2[..],
            &wires::PENT_NN3[..],
            &wires::PENT_NN4[..],
            &wires::PENT_NN5[..],
        ),
        (
            "NR",
            3,
            &wires::PENT_NN0[..],
            &wires::PENT_NN1[..],
            &wires::PENT_NN2[..],
            &wires::PENT_NN3[..],
            &wires::PENT_NN4[..],
            &wires::PENT_NN5[..],
        ),
        (
            "NW",
            0,
            &wires::PENT_NW0[..],
            &wires::PENT_NW1[..],
            &wires::PENT_NW2[..],
            &wires::PENT_NW3[..],
            &wires::PENT_NW4[..],
            &wires::PENT_NW5[..],
        ),
        (
            "NE",
            0,
            &wires::PENT_NE0[..],
            &wires::PENT_NE1[..],
            &wires::PENT_NE2[..],
            &wires::PENT_NE3[..],
            &wires::PENT_NE4[..],
            &wires::PENT_NE5[..],
        ),
        (
            "SL",
            0,
            &wires::PENT_SS0[..],
            &wires::PENT_SS1[..],
            &wires::PENT_SS2[..],
            &wires::PENT_SS3[..],
            &wires::PENT_SS4[..],
            &wires::PENT_SS5[..],
        ),
        (
            "SR",
            3,
            &wires::PENT_SS0[..],
            &wires::PENT_SS1[..],
            &wires::PENT_SS2[..],
            &wires::PENT_SS3[..],
            &wires::PENT_SS4[..],
            &wires::PENT_SS5[..],
        ),
        (
            "SW",
            0,
            &wires::PENT_SW0[..],
            &wires::PENT_SW1[..],
            &wires::PENT_SW2[..],
            &wires::PENT_SW3[..],
            &wires::PENT_SW4[..],
            &wires::PENT_SW5[..],
        ),
        (
            "SE",
            0,
            &wires::PENT_SE0[..],
            &wires::PENT_SE1[..],
            &wires::PENT_SE2[..],
            &wires::PENT_SE3[..],
            &wires::PENT_SE4[..],
            &wires::PENT_SE5[..],
        ),
    ] {
        for i in 0..3 {
            builder.wire_names(w0[base + i], &[format!("{name}5BEG{i}")]);
            builder.wire_names(w1[base + i], &[format!("{name}5A{i}")]);
            builder.wire_names(w2[base + i], &[format!("{name}5B{i}")]);
            builder.wire_names(w3[base + i], &[format!("{name}5MID{i}")]);
            builder.wire_names(w4[base + i], &[format!("{name}5C{i}")]);
            builder.wire_names(w5[base + i], &[format!("{name}5END{i}")]);
        }
    }
    builder.wire_names(wires::PENT_WW0_S0, &["WL5BEG_S0"]);
    builder.wire_names(wires::PENT_NN0_N5, &["NR5BEG_N2"]);
    builder.wire_names(wires::PENT_WS3_BUF0, &["WS5MID_FAKE0"]);
    builder.wire_names(wires::PENT_WS3_S0, &["WS5MID_S0"]);
    builder.wire_names(wires::PENT_NE3_BUF2, &["NE5MID_FAKE2"]);
    builder.wire_names(wires::PENT_NE3_N2, &["NE5MID_N2"]);
    builder.wire_names(wires::PENT_WN5_S0, &["WN5END_S0"]);
    builder.wire_names(wires::PENT_NW5_N2, &["NW5END_N2"]);
    builder.mark_permabuf(wires::PENT_WS3_BUF0);
    builder.mark_permabuf(wires::PENT_NE3_BUF2);

    // The long wires.
    for i in 0..19 {
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
    for i in 0..4 {
        builder.wire_names(wires::IMUX_CTRL[i], &[format!("CTRL{i}")]);
        builder.mark_permabuf(wires::IMUX_CTRL_SITE[i]);
        builder.mark_permabuf(wires::IMUX_CTRL_BOUNCE[i]);
        builder.wire_names(wires::IMUX_CTRL_SITE[i], &[format!("CTRL_B{i}")]);
        builder.wire_names(wires::IMUX_CTRL_BOUNCE[i], &[format!("CTRL_BOUNCE{i}")]);
        let (wire, dir) = match i {
            0 => (wires::IMUX_CTRL_BOUNCE_S0, Dir::S),
            3 => (wires::IMUX_CTRL_BOUNCE_N3, Dir::N),
            _ => continue,
        };
        builder.wire_names(wire, &[format!("CTRL_BOUNCE_{dir}{i}")]);
    }
    for i in 0..8 {
        builder.wire_names(wires::IMUX_BYP[i], &[format!("BYP{i}")]);
        builder.mark_permabuf(wires::IMUX_BYP_SITE[i]);
        builder.mark_permabuf(wires::IMUX_BYP_BOUNCE[i]);
        builder.wire_names(wires::IMUX_BYP_SITE[i], &[format!("BYP_B{i}")]);
        builder.wire_names(wires::IMUX_BYP_BOUNCE[i], &[format!("BYP_BOUNCE{i}")]);
        let (wire, dir) = match i {
            0 => (wires::IMUX_BYP_BOUNCE_S0, Dir::S),
            3 => (wires::IMUX_BYP_BOUNCE_N3, Dir::N),
            4 => (wires::IMUX_BYP_BOUNCE_S4, Dir::S),
            7 => (wires::IMUX_BYP_BOUNCE_N7, Dir::N),
            _ => continue,
        };
        builder.wire_names(wire, &[format!("BYP_BOUNCE_{dir}{i}")]);
    }
    for i in 0..8 {
        builder.wire_names(wires::IMUX_FAN[i], &[format!("FAN{i}")]);
        builder.mark_permabuf(wires::IMUX_FAN_SITE[i]);
        builder.mark_permabuf(wires::IMUX_FAN_BOUNCE[i]);
        builder.wire_names(wires::IMUX_FAN_SITE[i], &[format!("FAN_B{i}")]);
        builder.wire_names(wires::IMUX_FAN_BOUNCE[i], &[format!("FAN_BOUNCE{i}")]);
        let (wire, dir) = match i {
            0 => (wires::IMUX_FAN_BOUNCE_S0, Dir::S),
            7 => (wires::IMUX_FAN_BOUNCE_N7, Dir::N),
            _ => continue,
        };
        builder.wire_names(wire, &[format!("FAN_BOUNCE_{dir}{i}")]);
    }
    for i in 0..48 {
        builder.wire_names(wires::IMUX_IMUX[i], &[format!("IMUX_B{i}")]);
        builder.mark_delay(wires::IMUX_IMUX[i], wires::IMUX_IMUX_DELAY[i]);
    }

    for i in 0..24 {
        builder.wire_names(wires::OUT[i], &[format!("LOGIC_OUTS{i}")]);
        builder.mark_test_mux_in(wires::OUT_BEL[i], wires::OUT[i]);
        builder.mark_test_mux_in_test(wires::OUT_TEST[i], wires::OUT[i]);
        let (wire_dbl, wire_pent, dir) = match i {
            12 => (wires::OUT_S12_DBL, wires::OUT_S12_PENT, Dir::S),
            15 => (wires::OUT_N15_DBL, wires::OUT_N15_PENT, Dir::N),
            17 => (wires::OUT_N17_DBL, wires::OUT_N17_PENT, Dir::N),
            18 => (wires::OUT_S18_DBL, wires::OUT_S18_PENT, Dir::S),
            _ => continue,
        };
        builder.wire_names(wire_dbl, &[format!("LOGIC_OUTS_{dir}{i}")]);
        builder.wire_names(wire_pent, &[format!("LOGIC_OUTS_{dir}1_{i}")]);

        for j in 0..20 {
            builder.extra_name_sub(
                format!("CFG_CENTER_LOGIC_OUTS{i}_{j}"),
                j,
                wires::OUT_BEL[i],
            );
        }
    }

    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_SPEC[i],
            &[
                format!("INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("PPC_L_INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("PPC_R_INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("GTX_LEFT_INT_INTERFACE_BLOCK_INPS_B{i}"),
            ],
        );
    }

    builder.extra_name("IOI_OCLKP_1", wires::IMUX_SPEC[0]);
    builder.extra_name("IOI_OCLKDIV1", wires::IMUX_SPEC[1]);
    builder.extra_name("IOI_OCLKP_0", wires::IMUX_SPEC[2]);
    builder.extra_name("IOI_OCLKDIV0", wires::IMUX_SPEC[3]);
    builder.extra_name("IOI_ICLKP_1", wires::IMUX_IO_ICLK[0]);
    builder.extra_name("IOI_ICLKP_0", wires::IMUX_IO_ICLK[1]);
    builder.extra_name("IOI_ICLK1", wires::IMUX_ILOGIC_CLK[0]);
    builder.extra_name("IOI_ICLK0", wires::IMUX_ILOGIC_CLK[1]);
    builder.extra_name("IOI_ICLKB1", wires::IMUX_ILOGIC_CLKB[0]);
    builder.extra_name("IOI_ICLKB0", wires::IMUX_ILOGIC_CLKB[1]);

    for i in 0..10 {
        builder.wire_names(wires::HCLK_ROW[i], &[format!("HCLK_G_HCLK_P{i}")]);
        builder.extra_name_sub(format!("HCLK_IOI_G_HCLK_P{i}"), 2, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("HCLK_IOB_CMT_GCLK_B{i}"), 2, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("CLK_HROW_HCLKL_P{i}"), 0, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("CLK_HROW_HCLKR_P{i}"), 1, wires::HCLK_ROW[i]);

        builder.wire_names(wires::HCLK_IO[i], &[format!("IOI_LEAF_GCLK_P{i}")]);
        builder.extra_name_sub(format!("HCLK_IOI_LEAF_GCLK_P{i}"), 2, wires::HCLK_IO[i]);

        builder.wire_names(
            wires::HCLK_CMT[i],
            &[
                format!("CMT_BUFG{i}"),
                format!("CMT_BUFG{i}_BOT"),
                format!("CMT_BUFG{i}_TOP"),
            ],
        );
        builder.alt_name(format!("CMT_BUFG{i}_TO_CLKIN2"), wires::HCLK_CMT[i]);

        builder.extra_name_sub(format!("HCLK_IOB_CMT_BUFG{i}"), 2, wires::HCLK_CMT[i]);
    }
    for i in 0..4 {
        builder.wire_names(wires::RCLK_ROW[i], &[format!("HCLK_RCLK{i}")]);
        builder.extra_name_sub(format!("HCLK_IOI_RCLK{i}"), 2, wires::RCLK_ROW[i]);

        builder.wire_names(wires::RCLK_IO[i], &[format!("IOI_RCLK_FORIO_P{i}")]);
        builder.extra_name_sub(format!("HCLK_IOI_RCLK_FORIO_P{i}"), 2, wires::RCLK_IO[i]);

        builder.wire_names(wires::IOCLK[i], &[format!("IOI_IOCLKP{i}")]);
        builder.extra_name_sub(format!("HCLK_IOI_IOCLKP{i}"), 2, wires::IOCLK[i]);
    }

    builder.extra_name_sub("HCLK_IOI_REFCLK", 2, wires::IMUX_IDELAYCTRL_REFCLK);

    for i in 0..2 {
        builder.extra_name_sub(format!("HCLK_IOI_VRCLK{i}"), 2, wires::VRCLK[i]);
        builder.extra_name_sub(format!("HCLK_IOI_VRCLK_S{i}"), 2, wires::VRCLK_S[i]);
        builder.extra_name_sub(format!("HCLK_IOI_VRCLK_N{i}"), 2, wires::VRCLK_N[i]);
        builder.extra_name_sub(format!("HCLK_IOI_BUFR_I{i}"), 2, wires::IMUX_BUFR[i]);
    }

    builder.extra_name_sub("HCLK_IOI_I2CLK_P8", 0, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOI_I2CLK_P9", 1, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOI_I2CLK_P0", 2, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOI_I2CLK_P1", 3, wires::OUT_CLKPAD);

    for i in 0..10 {
        builder.extra_name_sub(format!("CLK_IOB_CLK_BUF{i}"), i, wires::OUT_CLKPAD);
        builder.extra_name_sub(format!("CLK_IOB_IOB_CLKP{i}"), 0, wires::GIOB[i]);
        builder.extra_name_sub(format!("CLK_HROW_CLK_METAL9_{i}"), 2, wires::GIOB[i]);
        builder.extra_name_sub(format!("CLK_HROW_CLK_H_METAL9_{i}"), 2, wires::GIOB_CMT[i]);
        builder.extra_name_sub(format!("CMT_GIOB{i}"), 0, wires::GIOB_CMT[i]);
    }

    for i in 0..5 {
        builder.wire_names(wires::MGT_ROW_I[i], &[format!("HCLK_BRAM_MGT_CLK_IN_P{i}")]);
        builder.wire_names(
            wires::MGT_ROW_O[i],
            &[format!("HCLK_BRAM_MGT_CLK_OUT_P{i}")],
        );

        builder.extra_name_sub(format!("HCLK_IOI_MGT_CLK_P{i}"), 2, wires::MGT_ROW_I[i]);
        builder.extra_name_sub(format!("GT3_MGT_CLK_P{i}"), 10, wires::MGT_ROW_O[i]);

        let ii = 4 - i;
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_MGT_CLKP_LBOT{ii}"),
            0,
            wires::MGT_ROW_I[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_MGT_CLKP_RBOT{i}"),
            20,
            wires::MGT_ROW_I[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_MGT_CLKP_LTOP{ii}"),
            10,
            wires::MGT_ROW_I[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_MGT_CLKP_RTOP{i}"),
            21,
            wires::MGT_ROW_I[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_LMGT_CLK_BOT{ii}"),
            0,
            wires::MGT_BUF[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_RMGT_CLK_BOT{i}"),
            0,
            wires::MGT_BUF[i + 5],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_LMGT_CLK_TOP{ii}"),
            10,
            wires::MGT_BUF[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_RMGT_CLK_TOP{i}"),
            10,
            wires::MGT_BUF[i + 5],
        );

        let ii = 15 + i;
        builder.extra_name_sub(format!("CLK_IOB_B_CLK_BUF{ii}"), 0, wires::MGT_ROW_I[i]);
        builder.extra_name_sub(format!("CLK_IOB_T_CLK_BUF{ii}"), 0, wires::MGT_ROW_I[i]);
        let ii = 14 - i;
        builder.extra_name_sub(format!("CLK_IOB_B_CLK_BUF{ii}"), 10, wires::MGT_ROW_I[i]);
        builder.extra_name_sub(format!("CLK_IOB_T_CLK_BUF{ii}"), 10, wires::MGT_ROW_I[i]);

        builder.extra_name_sub(format!("CLK_MGT_B_CLK{i}_LEFT"), 0, wires::MGT_ROW_I[i]);
        builder.extra_name_sub(format!("CLK_MGT_T_CLK{i}_LEFT"), 0, wires::MGT_ROW_I[i]);
        builder.extra_name_sub(format!("CLK_MGT_B_CLK{i}"), 10, wires::MGT_ROW_I[i]);
        builder.extra_name_sub(format!("CLK_MGT_T_CLK{i}"), 10, wires::MGT_ROW_I[i]);
    }

    for i in 0..32 {
        builder.wire_names(
            wires::GCLK[i],
            &[
                format!("CLK_HROW_GCLK_BUF{i}"),
                format!("CLK_BUFGMUX_GCLKP{i}"),
            ],
        );

        builder.wire_names(wires::IMUX_BUFG_I[i], &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
        builder.wire_names(wires::IMUX_BUFG_O[i], &[format!("CLK_IOB_MUXED_CLKOUT{i}")]);

        builder.extra_name_sub(
            format!("CLK_BUFGMUX_MUXED_IN_CLKB_P{i}"),
            0,
            wires::IMUX_BUFG_I[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_MUXED_IN_CLKT_P{i}"),
            10,
            wires::IMUX_BUFG_I[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_PREMUX{j}_CLK{k}", j = i % 2, k = i / 2),
            0,
            wires::IMUX_BUFG_O[i],
        );
        builder.extra_name_sub(
            format!("CLK_BUFGMUX_PREMUX{j}_CLK{k}", j = i % 2, k = 16 + i / 2),
            10,
            wires::IMUX_BUFG_O[i],
        );
    }
    for i in 0..16 {
        builder.wire_names(wires::OUT_BUFG[i], &[format!("CLK_BUFGMUX_GFB_BOT_{i}")]);
        builder.wire_names(
            wires::OUT_BUFG[i + 16],
            &[format!("CLK_BUFGMUX_GFB_TOP_{i}")],
        );
    }

    for (i, (c, i0, i1)) in [
        (2, 3, 15),
        (2, 9, 21),
        (2, 27, 39),
        (2, 33, 45),
        (3, 3, 15),
        (3, 9, 21),
        (3, 27, 39),
        (3, 33, 45),
        (4, 3, 15),
        (4, 9, 21),
        (4, 27, 39),
        (4, 33, 45),
        (3, 11, 35),
        (3, 23, 47),
        (4, 11, 35),
        (4, 23, 47),
        (15, 3, 15),
        (15, 9, 21),
        (15, 27, 39),
        (15, 33, 45),
        (16, 3, 15),
        (16, 9, 21),
        (16, 27, 39),
        (16, 33, 45),
        (17, 3, 15),
        (17, 9, 21),
        (17, 27, 39),
        (17, 33, 45),
        (16, 11, 35),
        (16, 23, 47),
        (17, 11, 35),
        (17, 23, 47),
    ]
    .into_iter()
    .enumerate()
    {
        builder.extra_name_sub(format!("CFG_CENTER_CKINT0_{i}"), c, wires::IMUX_IMUX[i0]);
        builder.extra_name_sub(format!("CFG_CENTER_CKINT1_{i}"), c, wires::IMUX_IMUX[i1]);
    }

    for i in 0..28 {
        builder.wire_names(
            wires::OUT_CMT[i],
            &[format!("CMT_CLK_{i:02}"), format!("CLK_CMT_CMT_CLK_{i:02}")],
        );
        builder.alt_name(format!("CMT_CLK_{i:02}_TEST"), wires::OUT_CMT[i]);
    }
    for i in 0..2 {
        builder.wire_names(wires::IMUX_DCM_CLKIN[i], &[format!("CMT_DCM_{i}_CLKIN")]);
        builder.wire_names(wires::IMUX_DCM_CLKFB[i], &[format!("CMT_DCM_{i}_CLKFB")]);

        builder.alt_name(format!("CMT_DCM_{i}_CLKIN_TEST"), wires::IMUX_DCM_CLKIN[i]);
        builder.alt_name(format!("CMT_DCM_{i}_CLKFB_TEST"), wires::IMUX_DCM_CLKFB[i]);

        builder.wire_names(
            wires::OMUX_DCM_SKEWCLKIN1[i],
            &[format!("CMT_DCM_{i}_MUXED_CLK")],
        );
        builder.wire_names(
            wires::OMUX_DCM_SKEWCLKIN2[i],
            &[format!("CMT_DCM_{i}_TEST_CLK_PINWIRE")],
        );
    }
    for i in 0..6 {
        builder.wire_names(
            wires::OUT_PLL_CLKOUTDCM[i],
            &[format!("CMT_PLL_CLKOUTDCM{i}")],
        );
    }
    builder.wire_names(wires::IMUX_PLL_CLKIN1, &["CMT_PLL_CLKIN1"]);
    builder.wire_names(wires::IMUX_PLL_CLKIN2, &["CMT_PLL_CLKIN2"]);
    builder.alt_name("CMT_PLL_CLK_DCM_MUX", wires::IMUX_PLL_CLKIN1);
    builder.wire_names(wires::IMUX_PLL_CLKFB, &["CMT_PLL_CLKFBIN"]);
    builder.alt_name("CMT_PLL_CLK_FB_FROM_DCM", wires::IMUX_PLL_CLKFB);
    builder.extra_name("CMT_PLL_CLKINFB_TEST", wires::IMUX_PLL_CLKFB);
    builder.wire_names(wires::OUT_PLL_CLKFBDCM, &["CMT_PLL_CLKFBDCM"]);
    builder.wire_names(wires::OMUX_PLL_SKEWCLKIN1, &["CMT_PLL_CLK_TO_DCM0"]);
    builder.wire_names(wires::OMUX_PLL_SKEWCLKIN2, &["CMT_PLL_CLK_TO_DCM1"]);
    builder.wire_names(wires::TEST_PLL_CLKIN, &["CMT_PLL_CLKIN1_TEST"]);
    builder.alt_name("CMT_PLL_CLKFBDCM_TEST", wires::OUT_PLL_CLKFBDCM);
    builder.extra_name_sub("CMT_DCM_0_SE_CLK_IN0", 0, wires::IMUX_CLK[0]);
    builder.extra_name_sub("CMT_DCM_0_SE_CLK_IN1", 0, wires::IMUX_CLK[1]);
    builder.extra_name_sub("CMT_DCM_0_SE_CLK_IN2", 0, wires::IMUX_IMUX[6]);
    builder.extra_name_sub("CMT_PLL_SE_CLK_IN0", 3, wires::IMUX_CLK[0]);
    builder.extra_name_sub("CMT_PLL_SE_CLK_IN1", 4, wires::IMUX_CLK[0]);
    builder.extra_name_sub("CMT_DCM_1_SE_CLK_IN0", 7, wires::IMUX_CLK[0]);
    builder.extra_name_sub("CMT_DCM_1_SE_CLK_IN1", 7, wires::IMUX_CLK[1]);
    builder.extra_name_sub("CMT_DCM_1_SE_CLK_IN2", 7, wires::IMUX_IMUX[12]);

    builder.int_type_id(tcls::INT, bslots::INT, "INT", "INT");

    builder.extract_term_buf_id(ccls::TERM_W, Dir::W, "L_TERM_INT", "TERM_W", &[]);
    builder.extract_term_buf_id(ccls::TERM_W, Dir::W, "GTX_L_TERM_INT", "TERM_W", &[]);
    builder.extract_term_buf_id(ccls::TERM_E, Dir::E, "R_TERM_INT", "TERM_E", &[]);
    let forced = [
        (wires::PENT_NW5_N2, wires::PENT_WN5[0]),
        (wires::PENT_WN5[0], wires::PENT_WS4[2]),
    ];
    builder.extract_term_buf_id(
        ccls::TERM_S_PPC,
        Dir::S,
        "PPC_T_TERM",
        "TERM_S_PPC",
        &forced,
    );
    let forced = [
        (wires::PENT_NN0[5], wires::PENT_WW0_S0),
        (wires::PENT_SS1[0], wires::PENT_NN0[5]),
    ];
    builder.extract_term_buf_id(
        ccls::TERM_N_PPC,
        Dir::N,
        "PPC_B_TERM",
        "TERM_N_PPC",
        &forced,
    );

    for &xy_l in rd.tiles_by_kind_name("INT_BUFS_L") {
        let mut xy_r = xy_l;
        while !matches!(
            &rd.tile_kinds.key(rd.tiles[&xy_r].kind)[..],
            "INT_BUFS_R" | "INT_BUFS_R_MON"
        ) {
            xy_r.x += 1;
        }
        if xy_l.y < 10 || xy_l.y >= rd.height - 10 {
            // wheeee.
            continue;
        }
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W, false).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E, false).unwrap();
        builder.extract_pass_tile_id(
            ccls::INT_BUFS_W,
            Dir::W,
            int_e_xy,
            Some(xy_r),
            Some(xy_l),
            Some("INT_BUFS_W"),
            None,
            None,
            int_w_xy,
            &wires::LH[..],
        );
        builder.extract_pass_tile_id(
            ccls::INT_BUFS_E,
            Dir::E,
            int_w_xy,
            Some(xy_l),
            Some(xy_r),
            Some("INT_BUFS_E"),
            None,
            None,
            int_e_xy,
            &wires::LH[..],
        );
    }
    for &xy_l in rd.tiles_by_kind_name("L_TERM_PPC") {
        let mut xy_r = xy_l;
        while rd.tile_kinds.key(rd.tiles[&xy_r].kind) != "R_TERM_PPC" {
            xy_r.x += 1;
        }
        let int_w_xy = builder.walk_to_int(xy_l, Dir::W, false).unwrap();
        let int_e_xy = builder.walk_to_int(xy_l, Dir::E, false).unwrap();
        builder.extract_pass_tile_id(
            ccls::PPC_W,
            Dir::W,
            int_e_xy,
            Some(xy_r),
            Some(xy_l),
            Some("PPC_W"),
            None,
            None,
            int_w_xy,
            &wires::LH[..],
        );
        builder.extract_pass_tile_id(
            ccls::PPC_E,
            Dir::E,
            int_w_xy,
            Some(xy_l),
            Some(xy_r),
            Some("PPC_E"),
            None,
            None,
            int_e_xy,
            &wires::LH[..],
        );
    }

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
    for (n, tkn) in [
        ("GTX_LEFT", "GTX_LEFT_INT_INTERFACE"),
        ("GTP", "GTP_INT_INTERFACE"),
        ("EMAC", "EMAC_INT_INTERFACE"),
        ("PCIE", "PCIE_INT_INTERFACE"),
        ("PPC_L", "PPC_L_INT_INTERFACE"),
        ("PPC_R", "PPC_R_INT_INTERFACE"),
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
            let int_xy = xy.delta(-1, 0);
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

    let intf = builder.ndb.get_tile_class_naming("INTF");

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut bel = builder
            .bel_xy(bslots::BRAM, "RAMB36", 0, 0)
            .pin_rename("CLKARDCLKL", "CLKAL")
            .pin_rename("CLKARDCLKU", "CLKAU")
            .pin_rename("CLKBWRCLKL", "CLKBL")
            .pin_rename("CLKBWRCLKU", "CLKBU")
            .pin_rename("ENARDENL", "ENAL")
            .pin_rename("ENBWRENL", "ENBL")
            .pin_rename("SSRARSTL", "SSRAL")
            .pin_rename("REGCLKARDRCLKL", "REGCLKAL")
            .pin_rename("REGCLKARDRCLKU", "REGCLKAU")
            .pin_rename("REGCLKBWRRCLKL", "REGCLKBL")
            .pin_rename("REGCLKBWRRCLKU", "REGCLKBU")
            .pins_name_only(&[
                "CASCADEOUTLATA",
                "CASCADEOUTLATB",
                "CASCADEOUTREGA",
                "CASCADEOUTREGB",
            ])
            .pin_name_only("CASCADEINLATA", 1)
            .pin_name_only("CASCADEINLATB", 1)
            .pin_name_only("CASCADEINREGA", 1)
            .pin_name_only("CASCADEINREGB", 1);
        for ul in ['L', 'U'] {
            for ab in ['A', 'B'] {
                for i in 0..16 {
                    bel = bel.pin_rename(format!("DI{ab}DI{ul}{i}"), format!("DI{ab}{ul}{i}"));
                    bel = bel.pin_rename(format!("DO{ab}DO{ul}{i}"), format!("DO{ab}{ul}{i}"));
                }
                for i in 0..2 {
                    bel = bel.pin_rename(format!("DIP{ab}DIP{ul}{i}"), format!("DIP{ab}{ul}{i}"));
                    bel = bel.pin_rename(format!("DOP{ab}DOP{ul}{i}"), format!("DOP{ab}{ul}{i}"));
                }
            }
        }
        let mut x = builder
            .xtile_id(tcls::BRAM, "BRAM", xy)
            .num_cells(5)
            .bel(bel);
        for dy in 0..5 {
            x = x.ref_int(xy.delta(-2, dy as i32), dy).ref_single(
                xy.delta(-1, dy as i32),
                dy,
                intf,
            );
        }
        x.extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, 1 + dy));
            intf_xy.push((xy.delta(-1, 1 + dy), intf));
        }
        let bram_xy = xy.delta(0, 1);
        builder.extract_xtile_bels_intf_id(
            tcls::PMVBRAM,
            xy,
            &[bram_xy],
            &int_xy,
            &intf_xy,
            "PMVBRAM",
            &[builder.bel_xy(bslots::PMVBRAM, "PMVBRAM", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), intf));
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
        builder.extract_xtile_bels_intf_id(tcls::DSP, xy, &[], &int_xy, &intf_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("EMAC").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let intf_emac = builder.ndb.get_tile_class_naming("INTF_EMAC");
        for dy in 0..10 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), intf_emac));
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

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE_B").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let intf_pcie = builder.ndb.get_tile_class_naming("INTF_PCIE");
        for by in [-11, 0, 11, 22] {
            for dy in 0..10 {
                int_xy.push(xy.delta(-2, by + dy));
                intf_xy.push((xy.delta(-1, by + dy), intf_pcie));
            }
        }
        builder.extract_xtile_bels_intf_id(
            tcls::PCIE,
            xy,
            &[xy.delta(0, 22)],
            &int_xy,
            &intf_xy,
            "PCIE",
            &[builder.bel_xy(bslots::PCIE, "PCIE", 0, 0)],
        );
    }

    if let Some((_, intf)) = builder.ndb.tile_class_namings.get_mut("INTF_PPC_R") {
        intf.intf_wires_in.insert(
            TileWireCoord::new_idx(0, wires::IMUX_CLK[0]),
            IntfWireInNaming::Buf {
                name_out: "PPC_R_INT_INTERFACE_FB_CLK_B0".to_string(),
                name_in: "INT_INTERFACE_CLK_B0".to_string(),
            },
        );
        intf.intf_wires_in.insert(
            TileWireCoord::new_idx(0, wires::IMUX_CLK[1]),
            IntfWireInNaming::Buf {
                name_out: "PPC_R_INT_INTERFACE_FB_CLK_B1".to_string(),
                name_in: "INT_INTERFACE_CLK_B1".to_string(),
            },
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PPC_B").iter().next() {
        let ppc_t_xy = xy.delta(0, 22);
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let intf_ppc_l = builder.ndb.get_tile_class_naming("INTF_PPC_L");
        let intf_ppc_r = builder.ndb.get_tile_class_naming("INTF_PPC_R");
        for by in [-10, 1, 12, 23] {
            for dy in 0..10 {
                int_xy.push(xy.delta(-11, by + dy));
                intf_xy.push((xy.delta(-10, by + dy), intf_ppc_l));
            }
        }
        for by in [-10, 1, 12, 23] {
            for dy in 0..10 {
                int_xy.push(xy.delta(20, by + dy));
                intf_xy.push((xy.delta(21, by + dy), intf_ppc_r));
            }
        }

        builder.extract_xtile_bels_intf_id(
            tcls::PPC,
            xy,
            &[ppc_t_xy],
            &int_xy,
            &intf_xy,
            "PPC",
            &[builder.bel_xy(bslots::PPC, "PPC440", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER").iter().next() {
        let mut bels = vec![];
        let mut bel_sysmon = builder
            .bel_xy(bslots::SYSMON, "SYSMON", 0, 0)
            .pins_name_only(&["VP", "VN"]);
        for i in 0..16 {
            bel_sysmon = bel_sysmon
                .pin_name_only(&format!("VAUXP{i}"), 1)
                .pin_name_only(&format!("VAUXN{i}"), 1);
        }
        bel_sysmon = bel_sysmon
            .sub_xy(rd, "IPAD", 0, 0)
            .pin_rename("O", "IPAD_VP_O")
            .pins_name_only(&["IPAD_VP_O"])
            .sub_xy(rd, "IPAD", 0, 1)
            .pin_rename("O", "IPAD_VN_O")
            .pins_name_only(&["IPAD_VN_O"]);
        bels.extend([
            builder.bel_xy(bslots::BSCAN[0], "BSCAN", 0, 0),
            builder.bel_xy(bslots::BSCAN[1], "BSCAN", 0, 1),
            builder.bel_xy(bslots::BSCAN[2], "BSCAN", 0, 2),
            builder.bel_xy(bslots::BSCAN[3], "BSCAN", 0, 3),
            builder.bel_xy(bslots::ICAP[0], "ICAP", 0, 0),
            builder.bel_xy(bslots::ICAP[1], "ICAP", 0, 1),
            builder.bel_single(bslots::PMV_CFG[0], "PMV"),
            builder.bel_single(bslots::STARTUP, "STARTUP"),
            builder.bel_single(bslots::JTAGPPC, "JTAGPPC"),
            builder.bel_single(bslots::FRAME_ECC, "FRAME_ECC"),
            builder.bel_single(bslots::DCIRESET, "DCIRESET"),
            builder.bel_single(bslots::CAPTURE, "CAPTURE"),
            builder.bel_single(bslots::USR_ACCESS, "USR_ACCESS_SITE"),
            builder.bel_single(bslots::KEY_CLEAR, "KEY_CLEAR"),
            builder.bel_single(bslots::EFUSE_USR, "EFUSE_USR"),
            bel_sysmon,
            builder.bel_virtual(bslots::MISC_CFG),
        ]);
        let mut xn = builder.xtile_id(tcls::CFG, "CFG", xy).num_cells(20);
        for i in 0..10 {
            xn = xn.ref_int(xy.delta(-4, -10 + (i as i32)), i);
            xn = xn.ref_single(xy.delta(-3, -10 + (i as i32)), i, intf);
        }
        for i in 0..10 {
            xn = xn.ref_int(xy.delta(-4, 1 + (i as i32)), i + 10);
            xn = xn.ref_single(xy.delta(-3, 1 + (i as i32)), i + 10, intf);
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();

        let mut bels = vec![];
        for i in 0..32 {
            bels.push(
                builder
                    .bel_xy(bslots::BUFGCTRL[i], "BUFGCTRL", 0, i)
                    .raw_tile(1),
            );
        }
        let mut test_outs = vec![];
        for (cell, range) in [
            (2, 0..12),
            (3, 0..12),
            (4, 0..8),
            (15, 0..12),
            (16, 0..12),
            (17, 0..8),
        ] {
            for i in range {
                test_outs.push(wires::OUT_BEL[i].cell(cell));
            }
        }
        let mut xn = builder
            .xtile_id(tcls::CLK_BUFG, "CLK_BUFG", xy)
            .raw_tile(xy.delta(1, 0))
            .num_cells(20)
            .switchbox(bslots::SPEC_INT)
            .optin_muxes(&wires::MGT_BUF[..])
            .optin_muxes(&wires::IMUX_BUFG_O[..])
            .optin_muxes_tile(&test_outs);
        for i in 0..5 {
            xn = xn
                .force_pip(wires::MGT_BUF[i].cell(0), wires::MGT_ROW_I[i].cell(0).pos())
                .force_pip(
                    wires::MGT_BUF[i + 5].cell(0),
                    wires::MGT_ROW_I[i].cell(20).pos(),
                )
                .force_pip(
                    wires::MGT_BUF[i].cell(10),
                    wires::MGT_ROW_I[i].cell(10).pos(),
                )
                .force_pip(
                    wires::MGT_BUF[i + 5].cell(10),
                    wires::MGT_ROW_I[i].cell(21).pos(),
                );
        }
        for i in 0..10 {
            xn = xn.ref_int(xy.delta(-4, -10 + (i as i32)), i);
            xn = xn.ref_single(xy.delta(-3, -10 + (i as i32)), i, intf);
        }
        for i in 0..10 {
            xn = xn.ref_int(xy.delta(-4, 1 + (i as i32)), i + 10);
            xn = xn.ref_single(xy.delta(-3, 1 + (i as i32)), i + 10, intf);
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();

        let pips = builder
            .pips
            .get_mut(&(tcls::CLK_BUFG, bslots::SPEC_INT))
            .unwrap();
        for (&(wt, _), mode) in pips.pips.iter_mut() {
            if !wires::IMUX_BUFG_O.contains(wt.wire) {
                *mode = PipMode::Buf;
            }
        }

        let naming = builder
            .ndb
            .tile_class_namings
            .get_mut("CLK_BUFG")
            .unwrap()
            .1;
        for i in 0..5 {
            let ii = 4 - i;
            naming.ext_pips.insert(
                (wires::MGT_BUF[i].cell(0), wires::MGT_ROW_I[i].cell(0)),
                PipNaming {
                    tile: RawTileId::from_idx(1),
                    wire_to: format!("CLK_BUFGMUX_LMGT_CLK_BOT{ii}"),
                    wire_from: format!("CLK_BUFGMUX_MGT_CLKP_LBOT{ii}"),
                },
            );
            naming.ext_pips.insert(
                (wires::MGT_BUF[i].cell(10), wires::MGT_ROW_I[i].cell(10)),
                PipNaming {
                    tile: RawTileId::from_idx(1),
                    wire_to: format!("CLK_BUFGMUX_LMGT_CLK_TOP{ii}"),
                    wire_from: format!("CLK_BUFGMUX_MGT_CLKP_LTOP{ii}"),
                },
            );
            naming.ext_pips.insert(
                (wires::MGT_BUF[i + 5].cell(0), wires::MGT_ROW_I[i].cell(20)),
                PipNaming {
                    tile: RawTileId::from_idx(1),
                    wire_to: format!("CLK_BUFGMUX_RMGT_CLK_BOT{i}"),
                    wire_from: format!("CLK_BUFGMUX_MGT_CLKP_RBOT{i}"),
                },
            );
            naming.ext_pips.insert(
                (wires::MGT_BUF[i + 5].cell(10), wires::MGT_ROW_I[i].cell(21)),
                PipNaming {
                    tile: RawTileId::from_idx(1),
                    wire_to: format!("CLK_BUFGMUX_RMGT_CLK_TOP{i}"),
                    wire_from: format!("CLK_BUFGMUX_MGT_CLKP_RTOP{i}"),
                },
            );
        }
    }

    for tkn in ["LIOB", "LIOB_MON", "CIOB", "RIOB"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let ioi_xy = xy.delta(if tkn == "LIOB" { 4 } else { -1 }, 0);
            let int_xy = builder.walk_to_int(ioi_xy, Dir::W, false).unwrap();
            let intf_xy = int_xy.delta(1, 0);
            let bel_ilogic0 = builder
                .bel_xy(bslots::ILOGIC[0], "ILOGIC", 0, 0)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "D",
                    "DDLY",
                    "TFB",
                    "OFB",
                    "OCLK",
                ])
                .extra_wire("I_IOB", &["IOI_IBUF1"]);
            let bel_ilogic1 = builder
                .bel_xy(bslots::ILOGIC[1], "ILOGIC", 0, 1)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "D",
                    "DDLY",
                    "TFB",
                    "OFB",
                    "OCLK",
                ])
                .extra_wire("I_IOB", &["IOI_IBUF0"])
                .extra_int_out_force("CLKPAD", wires::OUT_CLKPAD.cell(0), "IOI_I_2GCLK0");
            let bel_ologic0 = builder
                .bel_xy(bslots::OLOGIC[0], "OLOGIC", 0, 0)
                .pins_name_only(&["SHIFTIN1", "SHIFTIN2", "SHIFTOUT1", "SHIFTOUT2", "OQ"])
                .extra_wire("T_IOB", &["IOI_T1"])
                .extra_wire("O_IOB", &["IOI_O1"]);
            let bel_ologic1 = builder
                .bel_xy(bslots::OLOGIC[1], "OLOGIC", 0, 1)
                .pins_name_only(&["SHIFTIN1", "SHIFTIN2", "SHIFTOUT1", "SHIFTOUT2", "OQ"])
                .extra_wire("T_IOB", &["IOI_T0"])
                .extra_wire("O_IOB", &["IOI_O0"]);
            let bel_iodelay0 = builder
                .bel_xy(bslots::IODELAY[0], "IODELAY", 0, 0)
                .pins_name_only(&["IDATAIN", "ODATAIN", "T", "DATAOUT", "C"]);
            let bel_iodelay1 = builder
                .bel_xy(bslots::IODELAY[1], "IODELAY", 0, 1)
                .pins_name_only(&["IDATAIN", "ODATAIN", "T", "DATAOUT", "C"]);

            let mut bel_iob0 = builder
                .bel_xy(bslots::IOB[0], "IOB", 0, 0)
                .raw_tile(1)
                .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"]);
            let mut bel_iob1 = builder
                .bel_xy(bslots::IOB[1], "IOB", 0, 1)
                .raw_tile(1)
                .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"]);
            match tkn {
                "LIOB" => {
                    bel_iob0 = bel_iob0.extra_wire_force("MONITOR", "LIOB_MONITOR_N");
                    bel_iob1 = bel_iob1.extra_wire_force("MONITOR", "LIOB_MONITOR_P");
                }
                "LIOB_MON" => {
                    bel_iob0 = bel_iob0.extra_wire_force("MONITOR", "LIOB_MON_MONITOR_N");
                    bel_iob1 = bel_iob1.extra_wire_force("MONITOR", "LIOB_MON_MONITOR_P");
                }
                _ => (),
            }
            builder
                .xtile_id(tcls::IO, tkn, ioi_xy)
                .raw_tile(xy)
                .ref_int(int_xy, 0)
                .ref_single(intf_xy, 0, intf)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&wires::IMUX_SPEC[..])
                .optin_muxes(&wires::IMUX_IO_ICLK[..])
                .optin_muxes(&wires::IMUX_ILOGIC_CLK[..])
                .optin_muxes(&wires::IMUX_ILOGIC_CLKB[..])
                .skip_edge("IOI_BLOCK_OUTS_B0", "IOI_OCLKP_1")
                .skip_edge("IOI_BLOCK_OUTS_B1", "IOI_OCLKDIV1")
                .skip_edge("IOI_BLOCK_OUTS_B2", "IOI_OCLKP_0")
                .skip_edge("IOI_BLOCK_OUTS_B3", "IOI_OCLKDIV0")
                .bel(bel_ilogic0)
                .bel(bel_ilogic1)
                .bel(bel_ologic0)
                .bel(bel_ologic1)
                .bel(bel_iodelay0)
                .bel(bel_iodelay1)
                .bel(bel_iob0)
                .bel(bel_iob1)
                .extract();

            let naming = builder.ndb.tile_class_namings.get_mut(tkn).unwrap().1;
            for (wt, wf) in wires::IMUX_IO_ICLK_OPTINV
                .into_iter()
                .zip(wires::IMUX_IO_ICLK)
            {
                let wt = wt.cell(0);
                let wf = wf.cell(0);
                let wn = naming.wires[&wf].clone();
                naming.wires.insert(wt, wn);
            }
        }
    }

    let pips = builder.pips.get_mut(&(tcls::IO, bslots::SPEC_INT)).unwrap();
    let mut new_pips = vec![];
    pips.pips.retain(|&(wt, wf), _| {
        if let Some(idx) = wires::IMUX_IO_ICLK.index_of(wf.wire) {
            new_pips.push((wt, wires::IMUX_IO_ICLK_OPTINV[idx].cell(0).pos()));
            false
        } else {
            true
        }
    });
    for pip in new_pips {
        pips.pips.insert(pip, PipMode::Mux);
    }
    for (wt, wf) in wires::IMUX_IO_ICLK_OPTINV
        .into_iter()
        .zip(wires::IMUX_IO_ICLK)
    {
        let wt = wt.cell(0);
        let wf = wf.cell(0);
        pips.pips.insert((wt, wf.pos()), PipMode::Mux);
        pips.pips.insert((wt, wf.neg()), PipMode::Mux);
    }

    for tkn in ["CMT_BOT", "CMT_TOP"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(bslots::DCM[i], "DCM_ADV", 0, i)
                        .extra_wire("CLKIN_TEST", &[format!("CMT_DCM_{i}_CLKIN_TEST")])
                        .extra_wire("CLKFB_TEST", &[format!("CMT_DCM_{i}_CLKFB_TEST")]),
                );
            }
            bels.push(
                builder
                    .bel_xy(bslots::PLL[0], "PLL_ADV", 0, 0)
                    .extra_wire("CLKIN_ALT", &["CMT_PLL_CLK_DCM_MUX"])
                    .extra_wire("CLKFB_ALT", &["CMT_PLL_CLK_FB_FROM_DCM"])
                    .extra_int_out("TEST_CLKIN", &["CMT_PLL_CLKIN1_TEST"]),
            );
            let mut xn = builder
                .xtile_id(tcls::CMT, tkn, xy)
                .num_cells(10)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&wires::IMUX_DCM_CLKIN[..])
                .optin_muxes(&wires::IMUX_DCM_CLKFB[..])
                .optin_muxes(&wires::OMUX_DCM_SKEWCLKIN1[..])
                .optin_muxes(&wires::OMUX_DCM_SKEWCLKIN2[..])
                .optin_muxes(&[wires::OMUX_PLL_SKEWCLKIN1])
                .optin_muxes(&[wires::OMUX_PLL_SKEWCLKIN2])
                .optin_muxes(&[wires::IMUX_PLL_CLKFB])
                .optin_muxes(&[wires::IMUX_PLL_CLKIN1])
                .optin_muxes(&[wires::IMUX_PLL_CLKIN2])
                .optin_muxes(&[wires::OUT_CMT[10]])
                .optin_muxes_tile(&[wires::OUT_BEL[20].cell(5)])
                .skip_edge("CMT_PLL_CLKINFB_TEST", "CMT_PLL_CLKFBIN")
                .bels(bels);
            for i in 0..10 {
                xn = xn.ref_int(xy.delta(-3, i as i32), i);
                xn = xn.ref_single(xy.delta(-2, i as i32), i, intf);
            }
            xn.extract();

            let naming = builder.ndb.tile_class_namings.get_mut(tkn).unwrap().1;
            naming.ext_pips.insert(
                (wires::OUT_CMT[10].cell(0), wires::IMUX_PLL_CLKFB.cell(0)),
                PipNaming {
                    tile: RawTileId::from_idx(0),
                    wire_to: "CMT_CLK_10".into(),
                    wire_from: "CMT_PLL_CLKINFB_TEST".into(),
                },
            );
        }
    }

    let pips = builder
        .pips
        .get_mut(&(tcls::CMT, bslots::SPEC_INT))
        .unwrap();
    for (&(wt, _), mode) in pips.pips.iter_mut() {
        if wires::OUT_BEL.contains(wt.wire) {
            *mode = PipMode::PermaBuf;
        }
    }
    pips.pips
        .retain(|&(wt, _), _| !matches!(wt.wire, wires::IMUX_PLL_CLKIN1 | wires::IMUX_PLL_CLKIN2));
    pips.specials.extend([SwitchBoxItem::PairMux(PairMux {
        dst: [
            wires::IMUX_PLL_CLKIN1.cell(0),
            wires::IMUX_PLL_CLKIN2.cell(0),
        ],
        bits: vec![],
        src: [
            [
                Some(wires::GIOB_CMT[0].cell(0).pos()),
                Some(wires::GIOB_CMT[5].cell(0).pos()),
            ],
            [
                Some(wires::GIOB_CMT[1].cell(0).pos()),
                Some(wires::GIOB_CMT[6].cell(0).pos()),
            ],
            [
                Some(wires::GIOB_CMT[2].cell(0).pos()),
                Some(wires::GIOB_CMT[7].cell(0).pos()),
            ],
            [
                Some(wires::GIOB_CMT[3].cell(0).pos()),
                Some(wires::GIOB_CMT[8].cell(0).pos()),
            ],
            [
                Some(wires::GIOB_CMT[4].cell(0).pos()),
                Some(wires::GIOB_CMT[9].cell(0).pos()),
            ],
            [
                Some(wires::HCLK_CMT[0].cell(0).pos()),
                Some(wires::HCLK_CMT[5].cell(0).pos()),
            ],
            [
                Some(wires::HCLK_CMT[1].cell(0).pos()),
                Some(wires::HCLK_CMT[6].cell(0).pos()),
            ],
            [
                Some(wires::HCLK_CMT[2].cell(0).pos()),
                Some(wires::HCLK_CMT[7].cell(0).pos()),
            ],
            [
                Some(wires::HCLK_CMT[3].cell(0).pos()),
                Some(wires::HCLK_CMT[8].cell(0).pos()),
            ],
            [
                Some(wires::HCLK_CMT[4].cell(0).pos()),
                Some(wires::HCLK_CMT[9].cell(0).pos()),
            ],
            [None, Some(wires::OUT_PLL_CLKFBDCM.cell(0).pos())],
            [Some(wires::OMUX_DCM_SKEWCLKIN1[0].cell(0).pos()), None],
            [Some(wires::OMUX_DCM_SKEWCLKIN1[1].cell(0).pos()), None],
            [Some(wires::IMUX_CLK[0].cell(3).pos()), None],
        ]
        .into_iter()
        .map(|s| (s, Default::default()))
        .collect(),
    })]);

    for tkn in ["CLK_HROW", "CLK_HROW_MGT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcls::CLK_HROW, "CLK_HROW", xy)
                .switchbox(bslots::HROW_INT)
                .optin_muxes(&wires::HCLK_ROW[..])
                .num_cells(2)
                .extract();
        }
    }

    let pips = builder
        .pips
        .get_mut(&(tcls::CLK_HROW, bslots::HROW_INT))
        .unwrap();
    pips.pips.clear();
    for co in 0..2 {
        for o in 0..10 {
            for i in 0..32 {
                pips.pips.insert(
                    (
                        wires::HCLK_ROW[o].cell(co),
                        wires::GCLK_BUF[i].cell(0).pos(),
                    ),
                    PipMode::Mux,
                );
            }
        }
    }
    for i in 0..32 {
        pips.pips.insert(
            (wires::GCLK_BUF[i].cell(0), wires::GCLK[i].cell(0).pos()),
            PipMode::Buf,
        );
    }

    for tkn in ["HCLK", "HCLK_GT3", "HCLK_GTX", "HCLK_GTX_LEFT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel_gsig = builder.bel_xy(bslots::GLOBALSIG, "GLOBALSIG", 0, 0);
            builder
                .xtile_id(tcls::HCLK, "HCLK", xy)
                .ref_int(xy.delta(0, 1), 0)
                .switchbox(bslots::HCLK)
                .optin_muxes(&wires::HCLK[..])
                .optin_muxes(&wires::RCLK[..])
                .bel(bel_gsig)
                .extract();
        }
    }
    for mode in builder
        .pips
        .get_mut(&(tcls::HCLK, bslots::HCLK))
        .unwrap()
        .pips
        .values_mut()
    {
        *mode = PipMode::Buf;
    }

    for (tkn, tcid, naming, has_bufr, has_io_s, has_io_n, has_cmt) in [
        (
            "HCLK_IOI",
            tcls::HCLK_IO,
            "HCLK_IO",
            true,
            true,
            true,
            false,
        ),
        (
            "HCLK_IOI_CENTER",
            tcls::HCLK_IO_CENTER,
            "HCLK_IO_CENTER",
            false,
            true,
            true,
            false,
        ),
        (
            "HCLK_CMT_IOI",
            tcls::HCLK_IO_CMT_S,
            "HCLK_IO_CMT_S",
            false,
            true,
            false,
            true,
        ),
        (
            "HCLK_IOI_BOTCEN",
            tcls::HCLK_IO_CFG_S,
            "HCLK_IO_CFG_S",
            false,
            true,
            false,
            false,
        ),
        (
            "HCLK_IOI_BOTCEN_MGT",
            tcls::HCLK_IO_CFG_S,
            "HCLK_IO_CFG_S",
            false,
            true,
            false,
            false,
        ),
        (
            "HCLK_IOI_CMT",
            tcls::HCLK_IO_CMT_N,
            "HCLK_IO_CMT_N",
            false,
            false,
            true,
            true,
        ),
        (
            "HCLK_IOI_CMT_MGT",
            tcls::HCLK_IO_CMT_N,
            "HCLK_IO_CMT_N",
            false,
            false,
            true,
            true,
        ),
        (
            "HCLK_IOI_TOPCEN",
            tcls::HCLK_IO_CFG_N,
            "HCLK_IO_CFG_N",
            false,
            false,
            true,
            false,
        ),
        (
            "HCLK_IOI_TOPCEN_MGT",
            tcls::HCLK_IO_CFG_N,
            "HCLK_IO_CFG_N",
            false,
            false,
            true,
            false,
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_x = builder
                .walk_to_int(xy.delta(0, -1), Dir::W, false)
                .unwrap()
                .x;
            let mut bels = vec![];
            if has_io_n {
                for i in 0..2 {
                    bels.push(builder.bel_xy(
                        bslots::BUFIO[i],
                        "BUFIO",
                        0,
                        if has_io_s { i ^ 2 } else { i },
                    ))
                }
            }
            if has_io_s {
                for i in 2..4 {
                    bels.push(builder.bel_xy(bslots::BUFIO[i], "BUFIO", 0, i ^ 3))
                }
            }
            if has_bufr {
                for i in 0..2 {
                    bels.push(builder.bel_xy(bslots::BUFR[i], "BUFR", 0, i))
                }
            }
            bels.push(builder.bel_xy(bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0));
            bels.push(builder.bel_xy(bslots::DCI, "DCI", 0, 0));
            bels.push(builder.bel_virtual(bslots::BANK));

            if has_cmt || tcid == tcls::HCLK_IO_CFG_N {
                bels.push(builder.bel_virtual(bslots::HCLK_CMT_DRP));
            }

            let mut xn = builder
                .xtile_id(tcid, naming, xy)
                .num_cells(4)
                .switchbox(bslots::HCLK_IO_INT)
                .optin_muxes(&wires::HCLK_IO[..])
                .optin_muxes(&wires::RCLK_IO[..])
                .optin_muxes(&wires::RCLK_ROW[..])
                .optin_muxes(&[wires::IMUX_IDELAYCTRL_REFCLK])
                .optin_muxes(&wires::IMUX_BUFR[..])
                .bels(bels);
            if has_io_s {
                xn = xn
                    .raw_tile(xy.delta(0, -2))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y - 2,
                        },
                        0,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y - 2,
                        },
                        0,
                        intf,
                    )
                    .raw_tile(xy.delta(0, -1))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y - 1,
                        },
                        1,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y - 1,
                        },
                        1,
                        intf,
                    );
            }
            if has_io_n {
                xn = xn
                    .raw_tile(xy.delta(0, 1))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y + 1,
                        },
                        2,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y + 1,
                        },
                        2,
                        intf,
                    );
                if !has_io_s || has_bufr {
                    xn = xn
                        .raw_tile(xy.delta(0, 2))
                        .ref_int(
                            Coord {
                                x: int_x,
                                y: xy.y + 2,
                            },
                            3,
                        )
                        .ref_single(
                            Coord {
                                x: int_x + 1,
                                y: xy.y + 2,
                            },
                            3,
                            intf,
                        );
                }
            }
            if has_cmt {
                xn = xn
                    .raw_tile(xy.delta(1, 0))
                    .raw_tile(xy.delta(2, 0))
                    .optin_muxes(&wires::HCLK_CMT[..])
                    .optin_muxes(&wires::GIOB_CMT[..]);
            }
            xn.extract();

            let pips = builder.pips.get_mut(&(tcid, bslots::HCLK_IO_INT)).unwrap();
            for (&(wt, _), mode) in pips.pips.iter_mut() {
                if wt.wire != wires::IMUX_IDELAYCTRL_REFCLK
                    && !wires::RCLK_ROW.contains(wt.wire)
                    && !wires::IMUX_BUFR.contains(wt.wire)
                {
                    *mode = PipMode::Buf;
                }
            }
            pips.specials
                .insert(SwitchBoxItem::WireSupport(WireSupport {
                    wires: wires::IOCLK.into_iter().map(|w| w.cell(2)).collect(),
                    bits: vec![],
                }));
            if !has_bufr {
                pips.pips
                    .retain(|&(wt, _), _| !wires::RCLK_ROW.contains(wt.wire));
                for i in 0..4 {
                    pips.specials
                        .insert(SwitchBoxItem::WireSupport(WireSupport {
                            wires: BTreeSet::from_iter([wires::RCLK_ROW[i].cell(2)]),
                            bits: vec![],
                        }));
                }
            }
        }
    }

    for tkn in [
        "HCLK_IOB_CMT_BOT",
        "HCLK_IOB_CMT_BOT_MGT",
        "HCLK_IOB_CMT_MID",
        "HCLK_IOB_CMT_MID_MGT",
        "HCLK_IOB_CMT_TOP",
        "HCLK_IOB_CMT_TOP_MGT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder.bel_virtual(bslots::HCLK_CMT_DRP);
            builder
                .xtile_id(tcls::HCLK_CMT, "HCLK_CMT", xy)
                .num_cells(4)
                .raw_tile(xy.delta(1, 0))
                .switchbox(bslots::HCLK_IO_INT)
                .optin_muxes(&wires::HCLK_CMT[..])
                .optin_muxes(&wires::GIOB_CMT[..])
                .bel(bel)
                .extract();
        }
    }
    let pips = builder
        .pips
        .get_mut(&(tcls::HCLK_CMT, bslots::HCLK_IO_INT))
        .unwrap();
    for mode in pips.pips.values_mut() {
        *mode = PipMode::Buf;
    }

    for (tcid, naming, tkn) in [
        (tcls::CLK_IOB_S, "CLK_IOB_S", "CLK_IOB_B"),
        (tcls::CLK_IOB_N, "CLK_IOB_N", "CLK_IOB_T"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(10)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::GIOB[..])
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .extract();
        }
    }

    for (tkn, tcid, naming) in [
        ("CLK_CMT_BOT", tcls::CLK_CMT_S, "CLK_CMT_S"),
        ("CLK_CMT_BOT_MGT", tcls::CLK_CMT_S, "CLK_CMT_S"),
        ("CLK_CMT_TOP", tcls::CLK_CMT_N, "CLK_CMT_N"),
        ("CLK_CMT_TOP_MGT", tcls::CLK_CMT_N, "CLK_CMT_N"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(10)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .extract();
        }
    }

    for (tkn, tcid, naming) in [
        ("CLK_MGT_BOT", tcls::CLK_MGT_S, "CLK_MGT_S"),
        ("CLK_MGT_BOT_MGT", tcls::CLK_MGT_S, "CLK_MGT_S"),
        ("CLK_MGT_TOP", tcls::CLK_MGT_N, "CLK_MGT_N"),
        ("CLK_MGT_TOP_MGT", tcls::CLK_MGT_N, "CLK_MGT_N"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(10)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .extract();
        }
    }
    for tcid in [
        tcls::CLK_IOB_S,
        tcls::CLK_IOB_N,
        tcls::CLK_CMT_S,
        tcls::CLK_CMT_N,
        tcls::CLK_MGT_S,
        tcls::CLK_MGT_N,
    ] {
        if let Some(pips) = builder.pips.get_mut(&(tcid, bslots::CLK_INT)) {
            pips.pips
                .retain(|&(_wt, wf), _| !wires::MGT_ROW_I.contains(wf.wire));
            for i in 0..5 {
                pips.pips.insert(
                    (wires::MGT_BUF[i].cell(0), wires::MGT_ROW_I[i].cell(0).pos()),
                    PipMode::Buf,
                );
                pips.pips.insert(
                    (
                        wires::MGT_BUF[i + 5].cell(0),
                        wires::MGT_ROW_I[i].cell(10).pos(),
                    ),
                    PipMode::Buf,
                );
            }
            for i in 0..32 {
                for j in 0..10 {
                    pips.pips.insert(
                        (
                            wires::IMUX_BUFG_O[i].cell(0),
                            wires::MGT_BUF[j].cell(0).pos(),
                        ),
                        PipMode::Mux,
                    );
                }
            }
        }
    }

    for tkn in ["HCLK_BRAM_MGT", "HCLK_BRAM_MGT_LEFT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcls::HCLK_MGT_BUF, "HCLK_BRAM_MGT", xy)
                .num_cells(1)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::MGT_ROW_O[..])
                .extract();
        }
    }
    if let Some(pips) = builder.pips.get_mut(&(tcls::HCLK_MGT_BUF, bslots::CLK_INT)) {
        for mode in pips.pips.values_mut() {
            *mode = PipMode::Buf;
        }
    }

    for (tkn, tcid) in [
        ("GT3", tcls::GTP),
        ("GTX", tcls::GTX),
        ("GTX_LEFT", tcls::GTX),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_dx = if tkn == "GTX_LEFT" { 2 } else { -3 };
            let intf_gt = builder.ndb.get_tile_class_naming(if tkn == "GTX_LEFT" {
                "INTF_GTX_LEFT"
            } else {
                "INTF_GTP"
            });
            let (slot, gtkind) = if tcid == tcls::GTP {
                (bslots::GTP_DUAL, "GTP_DUAL")
            } else {
                (bslots::GTX_DUAL, "GTX_DUAL")
            };
            let mut bels = vec![
                builder
                    .bel_xy(slot, gtkind, 0, 0)
                    .pins_name_only(&[
                        "RXP0", "RXN0", "RXP1", "RXN1", "TXP0", "TXN0", "TXP1", "TXN1", "CLKIN",
                    ])
                    .extra_wire("CLKOUT_NORTH_S", &["GT3_CLKOUT_NORTH"])
                    .extra_wire("CLKOUT_NORTH", &["GT3_CLKOUT_NORTH_N"])
                    .extra_wire("CLKOUT_SOUTH", &["GT3_CLKOUT_SOUTH"])
                    .extra_wire("CLKOUT_SOUTH_N", &["GT3_CLKOUT_SOUTH_N"])
                    .extra_int_in(
                        "GREFCLK",
                        &["GT3_GREFCLK", "GTX_GREFCLK", "GTX_LEFT_GREFCLK"],
                    )
                    .sub_xy(rd, "BUFDS", 0, 0)
                    .pin_rename("IP", "BUFDS_IP")
                    .pin_rename("IN", "BUFDS_IN")
                    .pin_rename("O", "BUFDS_O")
                    .pins_name_only(&["BUFDS_IP", "BUFDS_IN", "BUFDS_O"])
                    .sub_xy(rd, "IPAD", 0, 5)
                    .pin_rename("O", "IPAD_BUFDS_IP_O")
                    .pins_name_only(&["IPAD_BUFDS_IP_O"])
                    .sub_xy(rd, "IPAD", 0, 4)
                    .pin_rename("O", "IPAD_BUFDS_IN_O")
                    .pins_name_only(&["IPAD_BUFDS_IN_O"])
                    .sub_xy(rd, "IPAD", 0, 1)
                    .pin_rename("O", "IPAD_RXP0_O")
                    .pins_name_only(&["IPAD_RXP0_O"])
                    .sub_xy(rd, "IPAD", 0, 0)
                    .pin_rename("O", "IPAD_RXN0_O")
                    .pins_name_only(&["IPAD_RXN0_O"])
                    .sub_xy(rd, "IPAD", 0, 3)
                    .pin_rename("O", "IPAD_RXP1_O")
                    .pins_name_only(&["IPAD_RXP1_O"])
                    .sub_xy(rd, "IPAD", 0, 2)
                    .pin_rename("O", "IPAD_RXN1_O")
                    .pins_name_only(&["IPAD_RXN1_O"])
                    .sub_xy(rd, "OPAD", 0, 1)
                    .pin_rename("I", "OPAD_TXP0_I")
                    .pins_name_only(&["OPAD_TXP0_I"])
                    .sub_xy(rd, "OPAD", 0, 0)
                    .pin_rename("I", "OPAD_TXN0_I")
                    .pins_name_only(&["OPAD_TXN0_I"])
                    .sub_xy(rd, "OPAD", 0, 3)
                    .pin_rename("I", "OPAD_TXP1_I")
                    .pins_name_only(&["OPAD_TXP1_I"])
                    .sub_xy(rd, "OPAD", 0, 2)
                    .pin_rename("I", "OPAD_TXN1_I")
                    .pins_name_only(&["OPAD_TXN1_I"]),
            ];

            for i in 0..4 {
                let mut bel = builder
                    .bel_xy(bslots::CRC32[i], "CRC32", 0, [0, 1, 3, 2][i])
                    .manual();
                if i.is_multiple_of(2) {
                    bel = bel
                        .sub_xy(rd, "CRC64", 0, i / 2)
                        .pin_rename("CRCCLK", "CRC64_CRCCLK")
                        .pin_rename("CRCRESET", "CRC64_CRCRESET")
                        .pin_rename("CRCDATAVALID", "CRC64_CRCDATAVALID");
                    for j in 0..3 {
                        bel = bel.pin_rename(
                            format!("CRCDATAWIDTH{j}"),
                            format!("CRC64_CRCDATAWIDTH{j}"),
                        );
                    }
                    for j in 0..32 {
                        bel = bel.pin_rename(format!("CRCOUT{j}"), format!("CRC64_CRCOUT{j}"));
                    }
                    for j in 0..64 {
                        bel = bel.pin_rename(format!("CRCIN{j}"), format!("CRC64_CRCIN{j}"));
                    }
                }
                bels.push(bel);
            }

            let mut xn = builder.xtile_id(tcid, tkn, xy).num_cells(20).bels(bels);
            for i in 0..10 {
                xn = xn.ref_int(xy.delta(int_dx, -10 + i as i32), i).ref_single(
                    xy.delta(int_dx + 1, -10 + i as i32),
                    i,
                    intf_gt,
                );
            }
            for i in 0..10 {
                xn = xn
                    .ref_int(xy.delta(int_dx, 1 + i as i32), i + 10)
                    .ref_single(xy.delta(int_dx + 1, 1 + i as i32), i + 10, intf_gt);
            }
            let mut xt = xn.extract();
            for i in [0, 2] {
                for j in 0..32 {
                    let pin = xt.bels[i]
                        .0
                        .pins
                        .remove(&format!("CRC64_CRCIN{j}"))
                        .unwrap();
                    assert_eq!(pin, xt.bels[i + 1].0.pins[&format!("CRCIN{j}")]);
                }
                for j in 0..32 {
                    let pin = xt.bels[i]
                        .0
                        .pins
                        .remove(&format!("CRC64_CRCIN{jj}", jj = j + 32))
                        .unwrap();
                    assert_eq!(pin, xt.bels[i].0.pins[&format!("CRCIN{j}")]);
                }
                for j in 0..32 {
                    let pin = xt.bels[i]
                        .0
                        .pins
                        .remove(&format!("CRC64_CRCOUT{j}"))
                        .unwrap();
                    assert_eq!(pin, xt.bels[i].0.pins[&format!("CRCOUT{j}")]);
                }
                for name in [
                    "CRCCLK",
                    "CRCRESET",
                    "CRCDATAVALID",
                    "CRCDATAWIDTH0",
                    "CRCDATAWIDTH1",
                    "CRCDATAWIDTH2",
                ] {
                    let pin = xt.bels[i].0.pins.remove(&format!("CRC64_{name}")).unwrap();
                    assert_eq!(pin, xt.bels[i].0.pins[name]);
                }
            }
            for (bslot, (bel, naming)) in bslots::CRC32.into_iter().zip(xt.bels) {
                builder.insert_tcls_bel(tcid, bslot, BelInfo::Legacy(bel));
                builder.insert_bel_naming(tkn, bslot, naming);
            }
        }
    }

    builder.build()
}
