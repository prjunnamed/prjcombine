use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{IntDb, ProgInv, SwitchBoxItem, WireSlotIdExt, WireSupport},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::{NamingDb, PipNaming, RawTileId};
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};
use prjcombine_types::bsdata::PolTileBit;
use prjcombine_virtex4::defs::{
    self, bslots,
    virtex7::{ccls, tcls, wires},
};

struct IntMaker<'a> {
    rd: &'a Part,
    builder: IntBuilder<'a>,
}

impl IntMaker<'_> {
    fn fill_int_wires(&mut self) {
        self.builder
            .inject_main_passes(DirMap::from_fn(|dir| match dir {
                Dir::W => ccls::PASS_W,
                Dir::E => ccls::PASS_E,
                Dir::S => ccls::PASS_S,
                Dir::N => ccls::PASS_N,
            }));

        self.builder.wire_names(wires::TIE_0, &["GND_WIRE"]);
        self.builder.wire_names(wires::TIE_1, &["VCC_WIRE"]);

        for i in 0..6 {
            self.builder.wire_names(
                wires::LCLK[i],
                &[format!("GCLK_B{i}_EAST"), format!("GCLK_L_B{i}")],
            );
            self.builder
                .extra_name_sub(format!("HCLK_LEAF_CLK_B_BOT{i}"), 0, wires::LCLK[i]);
            self.builder
                .extra_name_sub(format!("HCLK_LEAF_CLK_B_TOP{i}"), 1, wires::LCLK[i]);
        }
        for i in 6..12 {
            self.builder.wire_names(
                wires::LCLK[i],
                &[format!("GCLK_B{i}"), format!("GCLK_L_B{i}_WEST")],
            );
            self.builder.extra_name_sub(
                format!("HCLK_LEAF_CLK_B_BOTL{ii}", ii = i - 6),
                0,
                wires::LCLK[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_LEAF_CLK_B_TOPL{ii}", ii = i - 6),
                1,
                wires::LCLK[i],
            );
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
                    self.builder
                        .wire_names(wbeg, &[format!("{dir}{lr}1BEG_{dbeg}{i}")]);
                    if dir == dbeg {
                        continue;
                    }
                }

                self.builder
                    .wire_names(w0[ii], &[format!("{dir}{lr}1BEG{i}")]);
                self.builder
                    .wire_names(w1[ii], &[format!("{dir}{lr}1END{i}")]);

                if let Some((xi, dend, n, wend)) = dend
                    && i == xi
                {
                    self.builder
                        .wire_names(wend, &[format!("{dir}{lr}1END_{dend}{n}_{i}")]);
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
                self.builder
                    .wire_names(w0[i], &[format!("{da}{db}2BEG{i}")]);
                self.builder.wire_names(w1[i], &[format!("{da}{db}2A{i}")]);
                self.builder
                    .wire_names(w2[i], &[format!("{da}{db}2END{i}")]);
                if let Some((xi, dend, n, wend)) = dend
                    && i == xi
                {
                    self.builder
                        .wire_names(wend, &[format!("{da}{db}2END_{dend}{n}_{i}")]);
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
        ] {
            for i in 0..4 {
                self.builder
                    .wire_names(w0[i], &[format!("{da}{db}4BEG{i}")]);
                self.builder.wire_names(w1[i], &[format!("{da}{db}4A{i}")]);
                self.builder.wire_names(w2[i], &[format!("{da}{db}4B{i}")]);
                self.builder.wire_names(w3[i], &[format!("{da}{db}4C{i}")]);
                self.builder
                    .wire_names(w4[i], &[format!("{da}{db}4END{i}")]);
                if let Some((xi, dend, n, wend)) = dend
                    && i == xi
                {
                    self.builder
                        .wire_names(wend, &[format!("{da}{db}4END_{dend}{n}_{i}")]);
                }
            }
        }

        for (da, db, w0, w1, w2, w3, w4, w5, w6, dend) in [
            (
                Dir::N,
                Dir::N,
                wires::HEX_NN0,
                wires::HEX_NN1,
                wires::HEX_NN2,
                wires::HEX_NN3,
                wires::HEX_NN4,
                wires::HEX_NN5,
                wires::HEX_NN6,
                Some((0, Dir::S, 1, wires::HEX_NN6_S0)),
            ),
            (
                Dir::N,
                Dir::E,
                wires::HEX_NE0,
                wires::HEX_NE1,
                wires::HEX_NE2,
                wires::HEX_NE3,
                wires::HEX_NE4,
                wires::HEX_NE5,
                wires::HEX_NE6,
                None,
            ),
            (
                Dir::N,
                Dir::W,
                wires::HEX_NW0,
                wires::HEX_NW1,
                wires::HEX_NW2,
                wires::HEX_NW3,
                wires::HEX_NW4,
                wires::HEX_NW5,
                wires::HEX_NW6,
                Some((0, Dir::S, 0, wires::HEX_NW6_S0)),
            ),
            (
                Dir::S,
                Dir::S,
                wires::HEX_SS0,
                wires::HEX_SS1,
                wires::HEX_SS2,
                wires::HEX_SS3,
                wires::HEX_SS4,
                wires::HEX_SS5,
                wires::HEX_SS6,
                Some((3, Dir::N, 0, wires::HEX_SS6_N3)),
            ),
            (
                Dir::S,
                Dir::E,
                wires::HEX_SE0,
                wires::HEX_SE1,
                wires::HEX_SE2,
                wires::HEX_SE3,
                wires::HEX_SE4,
                wires::HEX_SE5,
                wires::HEX_SE6,
                None,
            ),
            (
                Dir::S,
                Dir::W,
                wires::HEX_SW0,
                wires::HEX_SW1,
                wires::HEX_SW2,
                wires::HEX_SW3,
                wires::HEX_SW4,
                wires::HEX_SW5,
                wires::HEX_SW6,
                Some((3, Dir::N, 0, wires::HEX_SW6_N3)),
            ),
        ] {
            for i in 0..4 {
                self.builder
                    .wire_names(w0[i], &[format!("{da}{db}6BEG{i}")]);
                self.builder.wire_names(w1[i], &[format!("{da}{db}6A{i}")]);
                self.builder.wire_names(w2[i], &[format!("{da}{db}6B{i}")]);
                self.builder.wire_names(w3[i], &[format!("{da}{db}6C{i}")]);
                self.builder.wire_names(w4[i], &[format!("{da}{db}6D{i}")]);
                self.builder.wire_names(w5[i], &[format!("{da}{db}6E{i}")]);
                self.builder
                    .wire_names(w6[i], &[format!("{da}{db}6END{i}")]);
                if let Some((xi, dend, n, wend)) = dend
                    && i == xi
                {
                    self.builder
                        .wire_names(wend, &[format!("{da}{db}6END_{dend}{n}_{i}")]);
                }
            }
        }

        // The long wires.
        for i in 0..13 {
            self.builder.wire_names(wires::LH[i], &[format!("LH{i}")]);
            self.builder
                .wire_names(wires::LVB[i], &[format!("LVB{i}"), format!("LVB_L{i}")]);
        }
        for i in 0..19 {
            self.builder
                .wire_names(wires::LV[i], &[format!("LV{i}"), format!("LV_L{i}")]);
        }
        self.builder
            .wire_names(wires::LVB[6], &["LVB6_SLV", "LVB_L6_SLV"]);

        // The control inputs.
        for i in 0..2 {
            self.builder
                .wire_names(wires::IMUX_GFAN[i], &[format!("GFAN{i}")]);
        }
        for i in 0..2 {
            self.builder.wire_names(
                wires::IMUX_CLK[i],
                &[format!("CLK{i}"), format!("CLK_L{i}")],
            );
        }
        for i in 0..2 {
            self.builder.wire_names(
                wires::IMUX_CTRL[i],
                &[format!("CTRL{i}"), format!("CTRL_L{i}")],
            );
        }
        for i in 0..8 {
            self.builder
                .wire_names(wires::IMUX_BYP[i], &[format!("BYP_ALT{i}")]);
            self.builder.mark_permabuf(wires::IMUX_BYP_SITE[i]);
            self.builder.mark_permabuf(wires::IMUX_BYP_BOUNCE[i]);
            self.builder.wire_names(
                wires::IMUX_BYP_SITE[i],
                &[format!("BYP{i}"), format!("BYP_L{i}")],
            );
            self.builder
                .wire_names(wires::IMUX_BYP_BOUNCE[i], &[format!("BYP_BOUNCE{i}")]);
            if matches!(i, 2 | 3 | 6 | 7) {
                self.builder
                    .wire_names(wires::IMUX_BYP_BOUNCE_N[i], &[format!("BYP_BOUNCE_N3_{i}")]);
            }
        }
        for i in 0..8 {
            self.builder
                .wire_names(wires::IMUX_FAN[i], &[format!("FAN_ALT{i}")]);
            self.builder.mark_permabuf(wires::IMUX_FAN_SITE[i]);
            self.builder.mark_permabuf(wires::IMUX_FAN_BOUNCE[i]);
            self.builder.wire_names(
                wires::IMUX_FAN_SITE[i],
                &[format!("FAN{i}"), format!("FAN_L{i}")],
            );
            self.builder
                .wire_names(wires::IMUX_FAN_BOUNCE[i], &[format!("FAN_BOUNCE{i}")]);
            if matches!(i, 0 | 2 | 4 | 6) {
                self.builder
                    .wire_names(wires::IMUX_FAN_BOUNCE_S[i], &[format!("FAN_BOUNCE_S3_{i}")]);
            }
        }
        for i in 0..48 {
            self.builder.wire_names(
                wires::IMUX_IMUX[i],
                &[format!("IMUX{i}"), format!("IMUX_L{i}")],
            );
            self.builder
                .mark_delay(wires::IMUX_IMUX[i], wires::IMUX_IMUX_DELAY[i]);
        }
        for i in 0..48 {
            self.builder.wire_names(
                wires::IMUX_BRAM[i],
                &[
                    format!("INT_INTERFACE_BRAM_UTURN_IMUX{i}"),
                    format!("INT_INTERFACE_BRAM_UTURN_R_IMUX{i}"),
                ],
            );
        }

        for i in 0..24 {
            self.builder.wire_names(
                wires::OUT[i],
                &[format!("LOGIC_OUTS{i}"), format!("LOGIC_OUTS_L{i}")],
            );
            self.builder
                .mark_test_mux_in(wires::OUT_BEL[i], wires::OUT[i]);
            self.builder
                .mark_test_mux_in_test(wires::OUT_TEST[i], wires::OUT[i]);
        }

        for i in 0..4 {
            self.builder.wire_names(
                wires::IMUX_SPEC[i],
                &[
                    format!("INT_INTERFACE_BLOCK_OUTS_B{i}"),
                    format!("INT_INTERFACE_BLOCK_OUTS_L_B{i}"),
                    format!("INT_INTERFACE_PSS_BLOCK_OUTS_L_B{i}"),
                ],
            );
        }
    }

    fn fill_hclk_wires(&mut self) {
        for i in 0..12 {
            self.builder
                .extra_name_sub(format!("HCLK_CK_BUFHCLK{i}"), 1, wires::HCLK_ROW[i]);
            self.builder
                .extra_name_sub(format!("HCLK_IOI_CK_BUFHCLK{i}"), 4, wires::HCLK_ROW[i]);
            self.builder
                .extra_name_sub(format!("HCLK_CMT_CK_BUFHCLK{i}"), 25, wires::HCLK_CMT[i]);
            self.builder
                .extra_name_sub(format!("CLK_HROW_CK_BUFHCLK_L{i}"), 1, wires::HCLK_ROW[i]);
            self.builder
                .extra_name_sub(format!("CLK_HROW_CK_BUFHCLK_R{i}"), 2, wires::HCLK_ROW[i]);

            self.builder
                .extra_name_sub(format!("HCLK_IOI_CK_IGCLK{i}"), 4, wires::HCLK_IO[i]);
        }
        for i in 0..4 {
            self.builder
                .extra_name_sub(format!("HCLK_CK_BUFRCLK{i}"), 1, wires::RCLK_ROW[i]);
            self.builder
                .extra_name_sub(format!("HCLK_IOI_CK_BUFRCLK{i}"), 4, wires::RCLK_ROW[i]);
            self.builder
                .extra_name_sub(format!("HCLK_CMT_CK_BUFRCLK{i}"), 25, wires::RCLK_CMT[i]);
            self.builder.extra_name_sub(
                format!("CLK_HROW_CK_BUFRCLK_L{i}"),
                1,
                wires::RCLK_HROW_W[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_HROW_CK_BUFRCLK_R{i}"),
                1,
                wires::RCLK_HROW_E[i],
            );
        }
        for i in 0..14 {
            self.builder
                .extra_name_sub(format!("CLK_HROW_CK_IN_L{i}"), 1, wires::HROW_I_HROW_W[i]);
            self.builder
                .extra_name_sub(format!("CLK_HROW_CK_IN_R{i}"), 1, wires::HROW_I_HROW_E[i]);
            self.builder.extra_name_tile_sub(
                "HCLK_CMT_L",
                format!("HCLK_CMT_MUX_CLK_{i}"),
                25,
                wires::HROW_I_CMT[i],
            );
            self.builder.extra_name_tile_sub(
                "HCLK_CMT_L",
                format!("HCLK_CMT_CK_IN{i}"),
                25,
                wires::HROW_O[i],
            );
            self.builder.extra_name_tile_sub(
                "HCLK_CMT",
                format!("HCLK_CMT_MUX_CLK_{i}"),
                25,
                wires::HROW_O[i],
            );
            self.builder.extra_name_tile_sub(
                "HCLK_CMT",
                format!("HCLK_CMT_CK_IN{i}"),
                25,
                wires::HROW_I_CMT[i],
            );
        }

        for i in 0..12 {
            self.builder.extra_name_sub(
                format!("CLK_HROW_CK_MUX_OUT_L{i}"),
                1,
                wires::IMUX_BUFHCE_W[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_HROW_CK_MUX_OUT_R{i}"),
                1,
                wires::IMUX_BUFHCE_E[i],
            );
        }

        self.builder
            .extra_name_sub("CLK_HROW_CK_IN_L_TEST_OUT", 1, wires::BUFH_TEST_W);
        self.builder
            .extra_name_sub("CLK_HROW_CK_IN_R_TEST_OUT", 1, wires::BUFH_TEST_E);
        self.builder
            .extra_name_sub("CLK_HROW_CK_IN_L_TEST_IN", 1, wires::BUFH_TEST_W_IN);
        self.builder
            .extra_name_sub("CLK_HROW_CK_IN_R_TEST_IN", 1, wires::BUFH_TEST_E_IN);

        self.builder
            .extra_name_sub("CLK_HROW_CK_INT_0_0", 1, wires::CKINT_HROW[0]);
        self.builder
            .extra_name_sub("CLK_HROW_CK_INT_0_1", 1, wires::CKINT_HROW[1]);
        self.builder
            .extra_name_sub("CLK_HROW_CK_INT_1_0", 1, wires::CKINT_HROW[2]);
        self.builder
            .extra_name_sub("CLK_HROW_CK_INT_1_1", 1, wires::CKINT_HROW[3]);
    }

    fn fill_gclk_wires(&mut self) {
        for i in 0..32 {
            self.builder.extra_name_sub(
                format!("CLK_HROW_BOT_R_CK_BUFG_CASCIN{i}"),
                1,
                wires::IMUX_BUFG_I[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_HROW_TOP_R_CK_BUFG_CASCIN{i}"),
                1,
                wires::IMUX_BUFG_I[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_HROW_BOT_R_CK_BUFG_CASCO{i}"),
                1,
                wires::IMUX_BUFG_O[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_HROW_TOP_R_CK_BUFG_CASCO{i}"),
                1,
                wires::IMUX_BUFG_O[i],
            );
            self.builder
                .extra_name(format!("CLK_BUFG_BOT_R_CK_MUXED{i}"), wires::IMUX_BUFG_I[i]);
            self.builder
                .extra_name(format!("CLK_BUFG_TOP_R_CK_MUXED{i}"), wires::IMUX_BUFG_I[i]);
            self.builder.extra_name(
                format!("CLK_BUFG_BUFGCTRL{j}_I{k}", j = i / 2, k = i % 2),
                wires::IMUX_BUFG_O[i],
            );

            self.builder
                .extra_name(format!("CLK_BUFG_CK_GCLK{i}"), wires::GCLK[i]);

            self.builder.extra_name_sub(
                format!("CLK_BUFG_REBUF_R_CK_GCLK{i}_BOT"),
                0,
                wires::GCLK[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_BUFG_REBUF_R_CK_GCLK{i}_TOP"),
                1,
                wires::GCLK[i],
            );
            self.builder
                .extra_name_sub(format!("CLK_BALI_REBUF_R_GCLK{i}_BOT"), 0, wires::GCLK[i]);
            self.builder
                .extra_name_sub(format!("CLK_BALI_REBUF_R_GCLK{i}_TOP"), 1, wires::GCLK[i]);
            self.builder
                .extra_name_sub(format!("CLK_HROW_R_CK_GCLK{i}"), 1, wires::GCLK_HROW[i]);
            self.builder.extra_name_sub(
                format!("CLK_HROW_CK_GCLK_TEST{i}"),
                1,
                wires::GCLK_TEST[i],
            );
            self.builder.extra_name_sub(
                format!("CLK_HROW_CK_GCLK_IN_TEST{i}",),
                1,
                wires::GCLK_TEST_IN[i],
            );
        }
        for i in 0..16 {
            self.builder
                .extra_name(format!("CLK_BUFG_BUFGCTRL{i}_O"), wires::OUT_BUFG[i]);
            self.builder
                .extra_name(format!("CLK_BUFG_R_FBG_OUT{i}"), wires::OUT_BUFG_GFB[i]);

            self.builder.extra_name_sub(
                format!("CLK_BUFG_R_CK_FB_TEST0_{i}"),
                i / 4,
                wires::OUT_BEL[4 + i % 4],
            );
            self.builder.extra_name_sub(
                format!("CLK_BUFG_R_CK_FB_TEST1_{i}"),
                i / 4,
                wires::OUT_BEL[i % 4],
            );

            self.builder.extra_name(
                format!("GCLK{i0}_{i1}_DN_TEST_RING_OUT", i0 = i * 2, i1 = i * 2 + 1),
                wires::GCLK_REBUF_TEST[2 * i],
            );
            self.builder.extra_name(
                format!("GCLK{i1}_{i0}_UP_TEST_RING_OUT", i0 = i * 2, i1 = i * 2 + 1),
                wires::GCLK_REBUF_TEST[2 * i + 1],
            );

            self.builder.extra_name(
                format!(
                    "CLK_BALI_REBUF_GCLK{i0}_{i1}_DN_TEST_RING_OUT",
                    i0 = i * 2,
                    i1 = i * 2 + 1
                ),
                wires::GCLK_REBUF_TEST[2 * i],
            );
            self.builder.extra_name(
                format!(
                    "CLK_BALI_REBUF_GCLK{i1}_{i0}_UP_TEST_RING_OUT",
                    i0 = i * 2,
                    i1 = i * 2 + 1
                ),
                wires::GCLK_REBUF_TEST[2 * i + 1],
            );
        }
    }

    fn fill_io_wires(&mut self) {
        for i in 0..4 {
            self.builder
                .extra_name(format!("IOI_RCLK_FORIO{i}"), wires::RCLK_IO[i]);
            self.builder
                .extra_name(format!("IOI_SING_RCLK_FORIO{i}"), wires::RCLK_IO[i]);
            self.builder
                .extra_name_sub(format!("HCLK_IOI_RCLK2IO{i}"), 4, wires::RCLK_IO[i]);

            self.builder
                .extra_name(format!("IOI_IOCLK{i}"), wires::IOCLK[i]);
            self.builder
                .extra_name(format!("IOI_SING_IOCLK{i}"), wires::IOCLK[i]);
            self.builder
                .extra_name_sub(format!("HCLK_IOI_IOCLK{i}"), 4, wires::IOCLK[i]);

            self.builder
                .extra_name_sub(format!("HCLK_IOI_IOCLK_PLL{i}"), 4, wires::PERF_IO[i]);
            self.builder.extra_name_sub(
                format!("HCLK_IOI_IO_PLL_CLK{i}_DMUX"),
                4,
                wires::IMUX_BUFIO[i],
            );
            self.builder
                .alt_name_sub(format!("HCLK_IOI_RCLK{i}"), 4, wires::IMUX_BUFIO[i]);
            self.builder.extra_name_sub(
                format!("HCLK_IOI_RCLK_BEFORE_DIV{i}"),
                4,
                wires::IMUX_BUFR[i],
            );
        }
        for i in 0..6 {
            self.builder
                .extra_name(format!("IOI_LEAF_GCLK{i}"), wires::LCLK_IO[i]);
            self.builder
                .extra_name(format!("IOI_SING_LEAF_GCLK{i}"), wires::LCLK_IO[i]);
            self.builder
                .extra_name_sub(format!("HCLK_IOI_LEAF_GCLK_BOT{i}"), 3, wires::LCLK_IO[i]);
            self.builder
                .extra_name_sub(format!("HCLK_IOI_LEAF_GCLK_TOP{i}"), 4, wires::LCLK_IO[i]);
        }
        self.builder
            .extra_name_sub("HCLK_IOI_I2IOCLK_BOT0", 1, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("HCLK_IOI_I2IOCLK_BOT1", 3, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("HCLK_IOI_I2IOCLK_TOP0", 5, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("HCLK_IOI_I2IOCLK_TOP1", 7, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("HCLK_IOI_RCLK_IMUX0", 4, wires::IMUX_BYP_SITE[3]);
        self.builder
            .extra_name_sub("HCLK_IOI_RCLK_IMUX1", 4, wires::IMUX_BYP_SITE[4]);
        self.builder
            .extra_name_sub("HCLK_IOI_RCLK_IMUX2", 3, wires::IMUX_BYP_SITE[4]);
        self.builder
            .extra_name_sub("HCLK_IOI_RCLK_IMUX3", 3, wires::IMUX_BYP_SITE[3]);
        self.builder.extra_name_sub(
            "HCLK_IOI_IDELAYCTRL_REFCLK",
            4,
            wires::IMUX_IDELAYCTRL_REFCLK,
        );

        for (wire, prefix, name) in [
            (wires::IMUX_IOI_ICLK[0], "ILOGIC", "CLK"),
            (wires::IMUX_IOI_ICLK[1], "ILOGIC", "CLKB"),
            (wires::IMUX_IOI_ICLKDIVP, "ILOGIC", "CLKDIVP"),
            (wires::IMUX_IOI_OCLKDIV[0], "OLOGIC", "CLKDIV"),
            (wires::IMUX_IOI_OCLKDIV[1], "OLOGIC", "CLKDIVB"),
            (wires::IMUX_IOI_OCLKDIVF[1], "OLOGIC", "CLKDIVFB"),
        ] {
            self.builder
                .extra_name_sub(format!("IOI_{prefix}1_{name}"), 0, wire);
            self.builder
                .extra_name_sub(format!("IOI_{prefix}0_{name}"), 1, wire);
            for tkn in ["LIOI3_SING", "RIOI3_SING", "LIOI_SING", "RIOI_SING"] {
                self.builder
                    .extra_name_tile_sub(tkn, format!("IOI_{prefix}0_{name}"), 0, wire);
            }
        }
        for prefix in ["LIOI", "RIOI"] {
            self.builder.extra_name_sub(
                format!("{prefix}_OLOGIC1_CLKDIVF"),
                0,
                wires::IMUX_IOI_OCLKDIVF[0],
            );
            self.builder.extra_name_sub(
                format!("{prefix}_OLOGIC0_CLKDIVF"),
                1,
                wires::IMUX_IOI_OCLKDIVF[0],
            );
            for tkn in ["LIOI3_SING", "RIOI3_SING", "LIOI_SING", "RIOI_SING"] {
                self.builder.extra_name_tile_sub(
                    tkn,
                    format!("{prefix}_OLOGIC0_CLKDIVF"),
                    0,
                    wires::IMUX_IOI_OCLKDIVF[0],
                );
            }
        }

        for (wire, name) in [
            (wires::IMUX_IOI_OCLK[0], "IOI_OCLK"),
            (wires::IMUX_IOI_OCLK[1], "IOI_OCLKM"),
        ] {
            self.builder.extra_name_sub(format!("{name}_1"), 0, wire);
            self.builder.extra_name_sub(format!("{name}_0"), 1, wire);
            for tkn in ["LIOI3_SING", "RIOI3_SING", "LIOI_SING", "RIOI_SING"] {
                self.builder
                    .extra_name_tile_sub(tkn, format!("{name}_0"), 0, wire);
            }
        }

        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_ICLK", 0, wires::PHASER_ICLK);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_ICLK_0", 1, wires::PHASER_ICLK);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_ICLKDIV", 0, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_ICLKDIV_0", 1, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_OCLK", 0, wires::PHASER_OCLK);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_OCLK_0", 1, wires::PHASER_OCLK);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_OCLK1X_90", 0, wires::PHASER_OCLK90);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_OCLK1X_90_0", 1, wires::PHASER_OCLK90);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_OCLKDIV", 0, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("IOI_PHASER_TO_IO_OCLKDIV_0", 1, wires::PHASER_OCLKDIV);
    }

    fn fill_cmt_wires(&mut self) {
        for i in 0..4 {
            self.builder
                .extra_name_sub(format!("HCLK_CMT_CCIO{i}"), 25, wires::CCIO_CMT[i]);
            self.builder.extra_name_sub(
                format!("HCLK_CMT_PREF_BOUNCE{i}"),
                25,
                wires::OMUX_CCIO[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_CLKINT_{i}"),
                25,
                wires::CKINT_CMT[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_PHSR_PERFCLK{i}"),
                75,
                wires::PERF[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_PHASERIN_RCLK{i}"),
                25,
                wires::OUT_PHASER_IN_RCLK[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_MMCM_MUXED{i}"),
                25,
                wires::OMUX_PLL_PERF[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_OUT_FREQ_REF{i}"),
                25,
                wires::OMUX_HCLK_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_FREQ_REF_NS{i}"),
                25,
                wires::CMT_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("MMCM_CLK_FREQ_BB_REBUF{i}_NS"),
                25,
                wires::CMT_FREQ_BB_S[i],
            );
            self.builder.extra_name_sub(
                format!("PLL_CLK_FREQ_BB_BUFOUT_NS{i}"),
                25,
                wires::CMT_FREQ_BB_N[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_L_LOWER_B_CLK_FREQ_BB{ii}", ii = i ^ 3),
                25,
                wires::CMT_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_R_LOWER_B_CLK_FREQ_BB{ii}", ii = i ^ 3),
                25,
                wires::CMT_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_TOP_L_UPPER_T_FREQ_BB{i}"),
                25,
                wires::CMT_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_TOP_R_UPPER_T_FREQ_BB{i}"),
                25,
                wires::CMT_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_FREQ_BB_PREF_IN{i}"),
                25,
                wires::CMT_FREQ_BB[i],
            );
            self.builder.extra_name_sub(
                format!("MMCMOUT_CLK_FREQ_BB_{i}"),
                25,
                wires::OUT_PLL_FREQ_BB_S[i],
            );
            self.builder.extra_name_sub(
                format!("PLLOUT_CLK_FREQ_BB_{i}"),
                25,
                wires::OUT_PLL_FREQ_BB_N[i],
            );
            self.builder.extra_name_sub(
                format!("MMCMOUT_CLK_FREQ_BB_REBUFOUT{i}"),
                25,
                wires::OMUX_PLL_FREQ_BB_S[i],
            );
            self.builder.extra_name_sub(
                format!("PLLOUT_CLK_FREQ_BB_REBUFOUT{i}"),
                25,
                wires::OMUX_PLL_FREQ_BB_N[i],
            );
        }
        self.builder
            .extra_name_sub("CMT_MMCM_PHYCTRL_SYNC_BB_UP", 25, wires::CMT_SYNC_BB);
        self.builder
            .extra_name_sub("CMT_PLL_PHYCTRL_SYNC_BB_DN", 25, wires::CMT_SYNC_BB);
        self.builder
            .extra_name_sub("CMT_PHASER_TOP_SYNC_BB", 25, wires::CMT_SYNC_BB);
        self.builder
            .extra_name_sub("CMT_MMCM_PHYCTRL_SYNC_BB_DN", 25, wires::CMT_SYNC_BB_S);
        self.builder
            .extra_name_sub("CMT_PLL_PHYCTRL_SYNC_BB_UP", 25, wires::CMT_SYNC_BB_N);
        self.builder
            .extra_name_sub("CMT_PHASERTOP_PHYCTLEMPTY", 25, wires::OUT_PHY_PHYCTLEMPTY);

        for i in 0..8 {
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_CLK_PLL{i}"),
                25,
                wires::OUT_PLL_N[i],
            );
        }
        for i in 0..14 {
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_CLK_MMCM{i}"),
                25,
                wires::OUT_PLL_S[i],
            );
        }
        self.builder
            .extra_name_sub("CMT_R_LOWER_B_CLK_IN1_INT", 15, wires::IMUX_CLK[0]);
        self.builder
            .extra_name_sub("CMT_R_LOWER_B_CLK_IN2_INT", 15, wires::IMUX_CLK[1]);
        self.builder
            .extra_name_sub("CMT_R_LOWER_B_CLK_IN3_INT", 14, wires::IMUX_CLK[0]);
        self.builder
            .extra_name_sub("CMT_L_LOWER_B_CLK_IN1_INT", 15, wires::IMUX_CLK[0]);
        self.builder
            .extra_name_sub("CMT_L_LOWER_B_CLK_IN2_INT", 15, wires::IMUX_CLK[1]);
        self.builder
            .extra_name_sub("CMT_L_LOWER_B_CLK_IN3_INT", 14, wires::IMUX_CLK[0]);
        self.builder.extra_name_sub(
            "CMT_TOP_R_UPPER_T_PLLE2_CLK_IN1_INT",
            37,
            wires::IMUX_CLK[1],
        );
        self.builder.extra_name_sub(
            "CMT_TOP_R_UPPER_T_PLLE2_CLK_IN2_INT",
            37,
            wires::IMUX_CLK[0],
        );
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLK_FB_INT", 38, wires::IMUX_CLK[0]);
        self.builder.extra_name_sub(
            "CMT_TOP_L_UPPER_T_PLLE2_CLK_IN1_INT",
            37,
            wires::IMUX_CLK[1],
        );
        self.builder.extra_name_sub(
            "CMT_TOP_L_UPPER_T_PLLE2_CLK_IN2_INT",
            37,
            wires::IMUX_CLK[0],
        );
        self.builder
            .extra_name_sub("CMT_TOP_L_UPPER_T_PLLE2_CLK_FB_INT", 38, wires::IMUX_CLK[0]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKIN1", 25, wires::IMUX_PLL_CLKIN1[0]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKIN2", 25, wires::IMUX_PLL_CLKIN2[0]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKFBIN", 25, wires::IMUX_PLL_CLKFB[0]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT0", 25, wires::OUT_PLL_S[0]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT0B", 25, wires::OUT_PLL_S[1]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT1", 25, wires::OUT_PLL_S[2]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT1B", 25, wires::OUT_PLL_S[3]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT2", 25, wires::OUT_PLL_S[4]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT2B", 25, wires::OUT_PLL_S[5]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT3", 25, wires::OUT_PLL_S[6]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT3B", 25, wires::OUT_PLL_S[7]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT4", 25, wires::OUT_PLL_S[8]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT5", 25, wires::OUT_PLL_S[9]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKOUT6", 25, wires::OUT_PLL_S[10]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKFBOUT", 25, wires::OUT_PLL_S[11]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_CLKFBOUTB", 25, wires::OUT_PLL_S[12]);
        self.builder
            .extra_name_sub("CMT_LR_LOWER_B_MMCM_TMUXOUT", 25, wires::OUT_PLL_S[13]);
        self.builder.extra_name_sub(
            "CMT_TOP_R_UPPER_T_PLLE2_CLKIN1",
            25,
            wires::IMUX_PLL_CLKIN1[1],
        );
        self.builder.extra_name_sub(
            "CMT_TOP_R_UPPER_T_PLLE2_CLKIN2",
            25,
            wires::IMUX_PLL_CLKIN2[1],
        );
        self.builder.extra_name_sub(
            "CMT_TOP_R_UPPER_T_PLLE2_CLKFBIN",
            25,
            wires::IMUX_PLL_CLKFB[1],
        );
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKOUT0", 25, wires::OUT_PLL_N[0]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKOUT1", 25, wires::OUT_PLL_N[1]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKOUT2", 25, wires::OUT_PLL_N[2]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKOUT3", 25, wires::OUT_PLL_N[3]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKOUT4", 25, wires::OUT_PLL_N[4]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKOUT5", 25, wires::OUT_PLL_N[5]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_CLKFBOUT", 25, wires::OUT_PLL_N[6]);
        self.builder
            .extra_name_sub("CMT_TOP_R_UPPER_T_PLLE2_TMUXOUT", 25, wires::OUT_PLL_N[7]);
        self.builder.extra_name_sub(
            "HCLK_CMT_MUX_MMCM_CLKIN1",
            25,
            wires::IMUX_PLL_CLKIN1_HCLK[0],
        );
        self.builder.extra_name_sub(
            "HCLK_CMT_MUX_PLLE2_CLKIN1",
            25,
            wires::IMUX_PLL_CLKIN1_HCLK[1],
        );
        self.builder.extra_name_sub(
            "HCLK_CMT_MUX_MMCM_CLKIN2",
            25,
            wires::IMUX_PLL_CLKIN2_HCLK[0],
        );
        self.builder.extra_name_sub(
            "HCLK_CMT_MUX_PLLE2_CLKIN2",
            25,
            wires::IMUX_PLL_CLKIN2_HCLK[1],
        );
        self.builder.extra_name_sub(
            "HCLK_CMT_MUX_MMCM_CLKFBIN",
            25,
            wires::IMUX_PLL_CLKFB_HCLK[0],
        );
        self.builder.extra_name_sub(
            "HCLK_CMT_MUX_PLLE2_CLKFBIN",
            25,
            wires::IMUX_PLL_CLKFB_HCLK[1],
        );
        self.builder
            .extra_name_sub("HCLK_CMT_PREF_CLKOUT", 25, wires::OUT_PHASER_REF_CLKOUT);
        self.builder
            .extra_name_sub("HCLK_CMT_PREF_TMUXOUT", 25, wires::OUT_PHASER_REF_TMUXOUT);
        self.builder.extra_name_sub(
            "CMT_PHASERREF_DOWN_PHASERIN_A",
            25,
            wires::IMUX_PHASER_IN_PHASEREFCLK[0],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_DOWN_PHASERIN_B",
            25,
            wires::IMUX_PHASER_IN_PHASEREFCLK[1],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_PHASERIN_C",
            25,
            wires::IMUX_PHASER_IN_PHASEREFCLK[2],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_PHASERIN_D",
            25,
            wires::IMUX_PHASER_IN_PHASEREFCLK[3],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_DOWN_PHASEROUT_A",
            25,
            wires::IMUX_PHASER_OUT_PHASEREFCLK[0],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_DOWN_PHASEROUT_B",
            25,
            wires::IMUX_PHASER_OUT_PHASEREFCLK[1],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_PHASEROUT_C",
            25,
            wires::IMUX_PHASER_OUT_PHASEREFCLK[2],
        );
        self.builder.extra_name_sub(
            "CMT_PHASERREF_PHASEROUT_D",
            25,
            wires::IMUX_PHASER_OUT_PHASEREFCLK[3],
        );
        self.builder
            .extra_name_sub("CMT_PHASER_BOT_REFMUX_0", 25, wires::IMUX_PHASER_REFMUX[0]);
        self.builder
            .extra_name_sub("CMT_PHASER_BOT_REFMUX_1", 25, wires::IMUX_PHASER_REFMUX[1]);
        self.builder
            .extra_name_sub("CMT_PHASER_BOT_REFMUX_2", 25, wires::IMUX_PHASER_REFMUX[2]);
        self.builder
            .extra_name_sub("CMT_PHASER_DOWN_DQS_TO_PHASER_A", 58, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("CMT_PHASER_DOWN_DQS_TO_PHASER_B", 70, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("CMT_PHASER_UP_DQS_TO_PHASER_C", 82, wires::OUT_CLKPAD);
        self.builder
            .extra_name_sub("CMT_PHASER_UP_DQS_TO_PHASER_D", 94, wires::OUT_CLKPAD);

        self.builder
            .extra_name_sub("CMT_PHASER_IN_A_ICLK", 7, wires::PHASER_ICLK);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_B_ICLK", 19, wires::PHASER_ICLK);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_C_ICLK", 31, wires::PHASER_ICLK);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_D_ICLK", 43, wires::PHASER_ICLK);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_A_WRCLK_TOFIFO", 7, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_B_WRCLK_TOFIFO", 19, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_C_WRCLK_TOFIFO", 31, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_R_PHASER_IN_C_WRCLK_FIFO", 31, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_D_WRCLK_TOFIFO", 43, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_R_PHASER_IN_D_WRCLK_TOFIFO", 43, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_A_WREN_TOFIFO", 7, wires::PHASER_IWREN);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_B_WREN_TOFIFO", 19, wires::PHASER_IWREN);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_C_WRENABLE_FIFO", 31, wires::PHASER_IWREN);
        self.builder
            .extra_name_sub("CMT_PHASER_IN_D_WRENABLE_FIFO", 43, wires::PHASER_IWREN);

        self.builder
            .extra_name_sub("CMT_PHASER_OUT_A_OCLK", 7, wires::PHASER_OCLK);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_B_OCLK", 19, wires::PHASER_OCLK);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_C_OCLK", 31, wires::PHASER_OCLK);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_D_OCLK", 43, wires::PHASER_OCLK);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_A_RDCLK_TOFIFO", 7, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_B_RDCLK_TOFIFO", 19, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_C_RDCLK_TOFIFO", 31, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_R_PHASER_OUT_C_RDCLK_FIFO", 31, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_D_RDCLK_TOFIFO", 43, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_R_PHASER_OUT_D_RDCLK_TOFIFO", 43, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_A_RDEN_TOFIFO", 7, wires::PHASER_ORDEN);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_B_RDEN_TOFIFO", 19, wires::PHASER_ORDEN);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_C_RDENABLE_TOFIFO", 31, wires::PHASER_ORDEN);
        self.builder
            .extra_name_sub("CMT_R_PHASER_OUT_C_RDENABLE_FIFO", 31, wires::PHASER_ORDEN);
        self.builder
            .extra_name_sub("CMT_PHASER_OUT_D_RDENABLE_TOFIFO", 43, wires::PHASER_ORDEN);
        self.builder.extra_name_sub(
            "CMT_R_PHASER_OUT_D_RDENABLE_TOFIFO",
            43,
            wires::PHASER_ORDEN,
        );
        for tkn in ["CMT_TOP_L_LOWER_B", "CMT_TOP_R_LOWER_B"] {
            self.builder
                .extra_name_tile_sub(tkn, "CMT_TOP_OCLK1X_90_8", 58, wires::PHASER_OCLK90);
        }
        for tkn in ["CMT_TOP_L_LOWER_T", "CMT_TOP_R_LOWER_T"] {
            self.builder
                .extra_name_tile_sub(tkn, "CMT_TOP_OCLK1X_90_4", 70, wires::PHASER_OCLK90);
        }
        for tkn in ["CMT_TOP_L_UPPER_B", "CMT_TOP_R_UPPER_B"] {
            self.builder
                .extra_name_tile_sub(tkn, "CMT_TOP_OCLK1X_90_7", 82, wires::PHASER_OCLK90);
        }
        for tkn in ["CMT_TOP_L_UPPER_T", "CMT_TOP_R_UPPER_T"] {
            self.builder
                .extra_name_tile_sub(tkn, "CMT_TOP_OCLK1X_90_7", 94, wires::PHASER_OCLK90);
        }

        for i in 0..2 {
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_CLK_LEAF_DN{i}"),
                25,
                wires::LCLK_CMT_S[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_MUX_CLK_LEAF_UP{i}"),
                25,
                wires::LCLK_CMT_N[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_BUFMR_INP{i}"),
                25,
                wires::IMUX_BUFMRCE[i],
            );
            self.builder.extra_name_sub(
                format!("HCLK_CMT_BUFMR_PHASEREF{i}"),
                25,
                wires::VMRCLK[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_PHASER_UP_PHASERREF_ABOVE{i}"),
                25,
                wires::VMRCLK_S[i],
            );
            self.builder.extra_name_sub(
                format!("CMT_PHASER_UP_PHASERREF_BELOW{i}"),
                25,
                wires::VMRCLK_N[i],
            );
        }
        self.builder
            .extra_name_sub("CMT_IN_FIFO_WRCLK", 6, wires::FIFO_IWRCLK);
        self.builder
            .extra_name_sub("CMT_IN_FIFO_WREN", 6, wires::FIFO_IWREN);
        self.builder
            .extra_name_sub("CMT_OUT_FIFO_RDCLK", 6, wires::FIFO_ORDCLK);
        self.builder
            .extra_name_sub("CMT_OUT_FIFO_RDEN", 6, wires::FIFO_ORDEN);
        self.builder
            .extra_name_sub("CMT_FIFO_L_PHASER_WRCLK", 6, wires::PHASER_ICLKDIV);
        self.builder
            .extra_name_sub("CMT_FIFO_L_PHASER_WRENABLE", 6, wires::PHASER_IWREN);
        self.builder
            .extra_name_sub("CMT_FIFO_L_PHASER_RDCLK", 6, wires::PHASER_OCLKDIV);
        self.builder
            .extra_name_sub("CMT_FIFO_L_PHASER_RDENABLE", 6, wires::PHASER_ORDEN);
    }

    fn fill_gt_wires(&mut self) {
        for i in 0..10 {
            self.builder.extra_name_sub(
                format!("GTPE2_COMMON_MGT_CLK{i}"),
                3,
                wires::HROW_O[i + 4],
            );
            self.builder.extra_name_sub(
                format!("GTXE2_COMMON_MGT_CLK{i}"),
                3,
                wires::HROW_O[i + 4],
            );
            self.builder.extra_name_sub(
                format!("GTHE2_COMMON_MGT_CLK{i}"),
                3,
                wires::HROW_O[i + 4],
            );
        }
        for i in 0..14 {
            self.builder
                .extra_name_sub(format!("HCLK_GTP_CK_IN{i}"), 3, wires::HROW_O[i]);
            self.builder
                .extra_name_sub(format!("HCLK_GTP_CK_MUX{i}"), 3, wires::HROW_I_GTP[i]);
        }
        for prefix in ["GTP", "GTX", "GTH"] {
            for i in 0..4 {
                self.builder.extra_name(
                    format!("{prefix}E2_CHANNEL_TXOUTCLK_{i}"),
                    wires::OUT_GT_TXOUTCLK,
                );
                self.builder.extra_name(
                    format!("{prefix}E2_CHANNEL_RXOUTCLK_{i}"),
                    wires::OUT_GT_RXOUTCLK,
                );
                self.builder.extra_name_sub(
                    format!("{prefix}E2_COMMON_TXOUTCLK_{i}"),
                    6 + i,
                    wires::OUT_GT_TXOUTCLK,
                );
                self.builder.extra_name_sub(
                    format!("{prefix}E2_COMMON_RXOUTCLK_{i}"),
                    6 + i,
                    wires::OUT_GT_RXOUTCLK,
                );
            }
        }
        for prefix in ["GTP", "GT", "GTH"] {
            for i in 0..2 {
                self.builder.extra_name_sub(
                    format!("IBUFDS_{prefix}E2_{i}_MGTCLKOUT"),
                    3,
                    wires::OUT_GT_MGTCLKOUT[i],
                );
            }
        }
        for i in 0..4 {
            self.builder.extra_name_sub(
                format!("GTPE2_COMMON_TXOUTCLK_MUX_{i}"),
                3,
                wires::OUT_GT_TXOUTCLK_HCLK[i],
            );
            self.builder.extra_name_sub(
                format!("GTPE2_COMMON_RXOUTCLK_MUX_{i}"),
                3,
                wires::OUT_GT_RXOUTCLK_HCLK[i],
            );
        }
        for i in 0..2 {
            self.builder.extra_name_sub(
                format!("IBUFDS_GTPE2_{i}_MGTCLKOUT_MUX"),
                3,
                wires::OUT_GT_MGTCLKOUT_HCLK[i],
            );
        }
    }
    fn fill_ps_wires(&mut self) {
        for i in 0..4 {
            self.builder
                .extra_name_sub(format!("PSS_FCLKCLK{i}"), 25, wires::HROW_O[i]);
            self.builder
                .extra_name_sub(format!("PSS2_FCLKCLK{i}"), 25, wires::HROW_O[i]);
        }
        // argh.
        self.builder
            .extra_name_sub("PSS1_LOGIC_OUTS1_39", 39, wires::OUT_BEL[1]);
        self.builder
            .extra_name_sub("PSS1_LOGIC_OUTS2_39", 39, wires::OUT_BEL[2]);
        self.builder
            .extra_name_sub("PSS2_LOGIC_OUTS0_61", 61, wires::OUT_BEL[0]);
        self.builder
            .extra_name_sub("PSS2_LOGIC_OUTS1_61", 61, wires::OUT_BEL[1]);

        for i in 0..3 {
            self.builder
                .extra_name_sub(format!("PSS3_TESTPLLNEWCLK{i}_OUT"), 75, wires::HROW_O[i]);
            self.builder.extra_name_sub(
                format!("PSS3_TESTPLLCLKOUT{i}_OUT"),
                75,
                wires::HROW_O[i + 3],
            );
        }
    }

    fn fill_int_tiles(&mut self) {
        self.builder
            .int_type_id(tcls::INT, bslots::INT, "INT_L", "INT_L");
        self.builder
            .int_type_id(tcls::INT, bslots::INT, "INT_R", "INT_R");
        self.builder
            .int_type_id(tcls::INT, bslots::INT, "INT_L_SLV_FLY", "INT_L");
        self.builder
            .int_type_id(tcls::INT, bslots::INT, "INT_R_SLV_FLY", "INT_R");
        self.builder
            .int_type_id(tcls::INT, bslots::INT, "INT_L_SLV", "INT_L_SLV");
        self.builder
            .int_type_id(tcls::INT, bslots::INT, "INT_R_SLV", "INT_R_SLV");

        let forced: Vec<_> = (0..6).map(|i| (wires::LH[i], wires::LH[11 - i])).collect();
        for tkn in [
            "L_TERM_INT",
            "L_TERM_INT_BRAM",
            "INT_INTERFACE_PSS_L",
            "GTP_INT_INTERFACE_L",
            "GTP_INT_INT_TERM_L",
        ] {
            self.builder
                .extract_term_conn_id(ccls::TERM_W, Dir::W, tkn, &forced);
        }
        let forced: Vec<_> = (0..6)
            .map(|i| (wires::LH[12 - i], wires::LH[i + 1]))
            .collect();
        for tkn in [
            "R_TERM_INT",
            "R_TERM_INT_GTX",
            "GTP_INT_INTERFACE_R",
            "GTP_INT_INT_TERM_R",
        ] {
            self.builder
                .extract_term_conn_id(ccls::TERM_E, Dir::E, tkn, &forced);
        }
        let forced = [
            (wires::SNG_W1_N3, wires::SNG_W1[4]),
            (wires::SNG_E0[4], wires::SNG_E0_N3),
            (wires::DBL_NW1[0], wires::DBL_SW0[3]),
            (wires::DBL_NE1[0], wires::DBL_SE0[3]),
            (wires::HEX_SW6_N3, wires::HEX_NW6[0]),
            (wires::HEX_NE5[0], wires::HEX_SE4[3]),
        ];
        for tkn in [
            "B_TERM_INT",
            "B_TERM_INT_SLV",
            "BRKH_B_TERM_INT",
            "HCLK_L_BOT_UTURN",
            "HCLK_R_BOT_UTURN",
        ] {
            self.builder
                .extract_term_conn_id(ccls::TERM_S, Dir::S, tkn, &forced);
        }
        let forced = [
            (wires::SNG_E0[3], wires::SNG_E0_S4),
            (wires::SNG_W1_S4, wires::SNG_W1[3]),
            (wires::DBL_SE1[3], wires::DBL_NE0[0]),
            (wires::HEX_SE5[3], wires::HEX_NE4[0]),
        ];
        for tkn in [
            "T_TERM_INT",
            "T_TERM_INT_SLV",
            "BRKH_TERM_INT",
            "BRKH_INT_PSS",
            "HCLK_L_TOP_UTURN",
            "HCLK_R_TOP_UTURN",
        ] {
            self.builder
                .extract_term_conn_id(ccls::TERM_N, Dir::N, tkn, &forced);
        }

        for (dir, n, tkn) in [
            (Dir::W, "L", "INT_INTERFACE_L"),
            (Dir::E, "R", "INT_INTERFACE_R"),
            (Dir::W, "L", "IO_INT_INTERFACE_L"),
            (Dir::E, "R", "IO_INT_INTERFACE_R"),
            (Dir::W, "PSS", "INT_INTERFACE_PSS_L"),
        ] {
            self.builder.extract_intf_id(
                tcls::INTF,
                dir,
                tkn,
                format!("INTF_{n}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                true,
                false,
            );
        }
        for (dir, n, tkn) in [
            (Dir::W, "L", "BRAM_INT_INTERFACE_L"),
            (Dir::E, "R", "BRAM_INT_INTERFACE_R"),
        ] {
            self.builder.extract_intf_id(
                tcls::INTF_BRAM,
                dir,
                tkn,
                format!("INTF_{n}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                true,
                false,
            );
        }
        for (dir, n, tkn) in [
            (Dir::E, "GTP", "GTP_INT_INTERFACE"),
            (Dir::W, "GTP_L", "GTP_INT_INTERFACE_L"),
            (Dir::E, "GTP_R", "GTP_INT_INTERFACE_R"),
            (Dir::E, "GTX", "GTX_INT_INTERFACE"),
            (Dir::W, "GTX_L", "GTX_INT_INTERFACE_L"),
            (Dir::E, "GTH", "GTH_INT_INTERFACE"),
            (Dir::W, "GTH_L", "GTH_INT_INTERFACE_L"),
            (Dir::W, "PCIE_L", "PCIE_INT_INTERFACE_L"),
            (Dir::W, "PCIE_LEFT_L", "PCIE_INT_INTERFACE_LEFT_L"),
            (Dir::E, "PCIE_R", "PCIE_INT_INTERFACE_R"),
            (Dir::W, "PCIE3_L", "PCIE3_INT_INTERFACE_L"),
            (Dir::E, "PCIE3_R", "PCIE3_INT_INTERFACE_R"),
        ] {
            self.builder.extract_intf_id(
                tcls::INTF_DELAY,
                dir,
                tkn,
                format!("INTF_{n}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                true,
                true,
            );
        }

        let forced: Vec<_> = self
            .builder
            .db
            .wires
            .iter()
            .filter_map(|(w, wn, _)| {
                if wn.starts_with("SNG_S") || wn.starts_with("SNG_N") {
                    None
                } else {
                    Some(w)
                }
            })
            .collect();

        self.builder.extract_pass_buf_id(
            ccls::BRKH_S,
            ccls::BRKH_N,
            Dir::S,
            "BRKH_INT",
            "BRKH_S",
            "BRKH_N",
            &forced,
        );
    }

    fn fill_clb_tiles(&mut self) {
        for (tkn, tcid) in [
            ("CLBLL_L", tcls::CLBLL),
            ("CLBLL_R", tcls::CLBLL),
            ("CLBLM_L", tcls::CLBLM),
            ("CLBLM_R", tcls::CLBLM),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = if tkn.ends_with("_L") {
                    Coord {
                        x: xy.x + 1,
                        y: xy.y,
                    }
                } else {
                    Coord {
                        x: xy.x - 1,
                        y: xy.y,
                    }
                };
                self.builder.extract_xtile_bels_id(
                    tcid,
                    xy,
                    &[],
                    &[int_xy],
                    tkn,
                    &[
                        self.builder
                            .bel_xy(bslots::SLICE[0], "SLICE", 0, 0)
                            .pin_name_only("CIN", 0)
                            .pin_name_only("COUT", 1),
                        self.builder
                            .bel_xy(bslots::SLICE[1], "SLICE", 1, 0)
                            .pin_name_only("CIN", 0)
                            .pin_name_only("COUT", 1),
                    ],
                    false,
                );
            }
        }
    }

    fn fill_bram_tiles(&mut self) {
        for tkn in ["BRAM_L", "BRAM_R"] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut int_xy = Vec::new();
                let mut intf_xy = Vec::new();
                let n = self.builder.ndb.get_tile_class_naming(if tkn == "BRAM_L" {
                    "INTF_L"
                } else {
                    "INTF_R"
                });
                for dy in 0..5 {
                    if tkn == "BRAM_L" {
                        int_xy.push(Coord {
                            x: xy.x + 2,
                            y: xy.y + dy,
                        });
                        intf_xy.push((
                            Coord {
                                x: xy.x + 1,
                                y: xy.y + dy,
                            },
                            n,
                        ));
                    } else {
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
                }
                let mut bel_bram_f = self
                    .builder
                    .bel_xy(bslots::BRAM_F, "RAMB36", 0, 0)
                    .pins_name_only(&["CASCADEINA", "CASCADEINB"])
                    .pin_name_only("CASCADEOUTA", 1)
                    .pin_name_only("CASCADEOUTB", 1);
                for ab in ["ARD", "BWR"] {
                    for ul in ['U', 'L'] {
                        for i in 0..16 {
                            if i == 15 && ul == 'U' {
                                continue;
                            }
                            bel_bram_f =
                                bel_bram_f.pin_name_only(&format!("ADDR{ab}ADDR{ul}{i}"), 0);
                        }
                    }
                }
                let mut bel_bram_h0 = self.builder.bel_xy(bslots::BRAM_H[0], "RAMB18", 0, 0);
                let mut bel_bram_h1 = self
                    .builder
                    .bel_xy(bslots::BRAM_H[1], "RAMB18", 0, 1)
                    .pins_name_only(&[
                        "FULL",
                        "EMPTY",
                        "ALMOSTFULL",
                        "ALMOSTEMPTY",
                        "WRERR",
                        "RDERR",
                    ]);
                for ab in ["ARD", "BWR"] {
                    for i in 0..14 {
                        bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("ADDR{ab}ADDR{i}"), 0);
                        bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("ADDR{ab}ADDR{i}"), 0);
                    }
                }
                for ab in ['A', 'B'] {
                    for i in 0..2 {
                        bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("ADDR{ab}TIEHIGH{i}"), 0);
                        bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("ADDR{ab}TIEHIGH{i}"), 0);
                    }
                }
                for i in 0..12 {
                    bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("RDCOUNT{i}"), 0);
                    bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("WRCOUNT{i}"), 0);
                }
                let mut bel_bram_addr = self.builder.bel_virtual(bslots::BRAM_ADDR);
                for i in 0..5 {
                    for j in 0..48 {
                        bel_bram_addr = bel_bram_addr
                            .extra_int_in(format!("IMUX_{i}_{j}"), &[&format!("BRAM_IMUX{j}_{i}")])
                            .extra_int_out(
                                format!("IMUX_UTURN_{i}_{j}"),
                                &[&format!("BRAM_IMUX{j}_UTURN_{i}")],
                            );
                    }
                }
                for ab in ["ARD", "BWR"] {
                    for ul in ['U', 'L'] {
                        for i in 0..15 {
                            bel_bram_addr = bel_bram_addr
                                .extra_int_in(
                                    format!("IMUX_ADDR{ab}ADDR{ul}{i}"),
                                    &[
                                        &format!("BRAM_IMUX_ADDR{ab}ADDR{ul}{i}"),
                                        &format!("BRAM_R_IMUX_ADDR{ab}ADDR{ul}{i}"),
                                    ],
                                )
                                .extra_wire(
                                    format!("UTURN_ADDR{ab}ADDR{ul}{i}"),
                                    &[&format!("BRAM_UTURN_ADDR{ab}ADDR{ul}{i}")],
                                )
                                .extra_wire(
                                    format!("ADDR{ab}ADDR{ul}{i}"),
                                    &[&format!("BRAM_ADDR{ab}ADDR{ul}{i}")],
                                );
                            if ul == 'U' {
                                bel_bram_addr = bel_bram_addr
                                    .extra_wire(
                                        format!("CASCINBOT_ADDR{ab}ADDR{ul}{i}"),
                                        &[&format!("BRAM_CASCINBOT_ADDR{ab}ADDR{ul}{i}")],
                                    )
                                    .extra_wire(
                                        format!("CASCINTOP_ADDR{ab}ADDR{ul}{i}"),
                                        &[&format!("BRAM_CASCINTOP_ADDR{ab}ADDR{ul}{i}")],
                                    )
                                    .extra_wire(
                                        format!("CASCOUT_ADDR{ab}ADDR{ul}{i}"),
                                        &[&format!("BRAM_CASCOUT_ADDR{ab}ADDR{ul}{i}")],
                                    );
                            }
                        }
                    }
                    bel_bram_addr = bel_bram_addr
                        .extra_int_in(
                            format!("IMUX_ADDR{ab}ADDRL15"),
                            &[
                                &format!("BRAM_IMUX_ADDR{ab}ADDRL15"),
                                &format!("BRAM_IMUX_R_ADDR{ab}ADDRL15"),
                            ],
                        )
                        .extra_wire(
                            format!("UTURN_ADDR{ab}ADDRL15"),
                            &[&format!("BRAM_UTURN_ADDR{ab}ADDRL15")],
                        );
                }
                self.builder.extract_xtile_bels_intf_id(
                    tcls::BRAM,
                    xy,
                    &[],
                    &int_xy,
                    &intf_xy,
                    tkn,
                    &[bel_bram_f, bel_bram_h0, bel_bram_h1, bel_bram_addr],
                );
            }
        }
        if let Some(&xy) = self.rd.tiles_by_kind_name("HCLK_BRAM").iter().next() {
            let mut bram_xy = Vec::new();
            for dy in [1, 6, 11] {
                bram_xy.push(Coord {
                    x: xy.x,
                    y: xy.y + dy,
                });
            }
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            if self.rd.tile_kinds.key(self.rd.tiles[&bram_xy[0]].kind) == "BRAM_L" {
                let n = self.builder.ndb.get_tile_class_naming("INTF_L");
                for dy in 0..15 {
                    int_xy.push(Coord {
                        x: xy.x + 2,
                        y: xy.y + 1 + dy,
                    });
                    intf_xy.push((
                        Coord {
                            x: xy.x + 1,
                            y: xy.y + 1 + dy,
                        },
                        n,
                    ));
                }
            } else {
                let n = self.builder.ndb.get_tile_class_naming("INTF_R");
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
            }
            self.builder.extract_xtile_bels_intf_id(
                tcls::PMVBRAM,
                xy,
                &bram_xy,
                &int_xy,
                &intf_xy,
                "PMVBRAM",
                &[self.builder.bel_xy(bslots::PMVBRAM, "PMVBRAM", 0, 0)],
            );

            let bel = self
                .builder
                .bel_xy(bslots::PMVBRAM, "PMVBRAM", 0, 0)
                .pins_name_only(&[
                    "O", "ODIV2", "ODIV4", "SELECT1", "SELECT2", "SELECT3", "SELECT4",
                ]);
            self.builder
                .xtile_id(tcls::PMVBRAM_NC, "PMVBRAM_NC", xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    fn fill_dsp_tiles(&mut self) {
        for tkn in ["DSP_L", "DSP_R"] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut int_xy = Vec::new();
                let mut intf_xy = Vec::new();
                let n = self.builder.ndb.get_tile_class_naming(if tkn == "DSP_L" {
                    "INTF_L"
                } else {
                    "INTF_R"
                });
                for dy in 0..5 {
                    if tkn == "DSP_L" {
                        int_xy.push(Coord {
                            x: xy.x + 2,
                            y: xy.y + dy,
                        });
                        intf_xy.push((
                            Coord {
                                x: xy.x + 1,
                                y: xy.y + dy,
                            },
                            n,
                        ));
                    } else {
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
                }

                let mut bels_dsp = vec![];
                for i in 0..2 {
                    let mut bel = self.builder.bel_xy(bslots::DSP[i], "DSP48", 0, i);
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
                    self.builder
                        .bel_xy(bslots::TIEOFF_DSP, "TIEOFF", 0, 0)
                        .pins_name_only(&["HARD0", "HARD1"]),
                );
                self.builder.extract_xtile_bels_intf_id(
                    tcls::DSP,
                    xy,
                    &[],
                    &int_xy,
                    &intf_xy,
                    tkn,
                    &bels_dsp,
                );
            }
        }
    }

    fn fill_pcie_tiles(&mut self) {
        for (kind, tkn) in [("PCIE_L", "PCIE_BOT_LEFT"), ("PCIE_R", "PCIE_BOT")] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut int_xy = Vec::new();
                let mut intf_xy = Vec::new();
                let (int_cols, intf_cols, intf_namings) = if kind == "PCIE_R" {
                    (
                        [xy.x - 2, xy.x + 6],
                        [xy.x - 1, xy.x + 5],
                        [
                            self.builder.ndb.get_tile_class_naming("INTF_PCIE_R"),
                            self.builder.ndb.get_tile_class_naming("INTF_PCIE_L"),
                        ],
                    )
                } else {
                    (
                        [xy.x + 6, xy.x - 2],
                        [xy.x + 5, xy.x - 1],
                        [
                            self.builder.ndb.get_tile_class_naming("INTF_PCIE_LEFT_L"),
                            self.builder.ndb.get_tile_class_naming("INTF_PCIE_R"),
                        ],
                    )
                };
                for dy in 0..25 {
                    int_xy.push(Coord {
                        x: int_cols[0],
                        y: xy.y - 10 + dy,
                    });
                    intf_xy.push((
                        Coord {
                            x: intf_cols[0],
                            y: xy.y - 10 + dy,
                        },
                        intf_namings[0],
                    ));
                }
                for dy in 0..25 {
                    int_xy.push(Coord {
                        x: int_cols[1],
                        y: xy.y - 10 + dy,
                    });
                    intf_xy.push((
                        Coord {
                            x: intf_cols[1],
                            y: xy.y - 10 + dy,
                        },
                        intf_namings[1],
                    ));
                }
                let t_xy = Coord {
                    x: xy.x,
                    y: xy.y + 10,
                };
                self.builder.extract_xtile_bels_intf_id(
                    tcls::PCIE,
                    xy,
                    &[t_xy],
                    &int_xy,
                    &intf_xy,
                    kind,
                    &[self.builder.bel_xy(bslots::PCIE, "PCIE", 0, 0)],
                );
            }
        }
    }

    fn fill_pcie3_tiles(&mut self) {
        if let Some(&xy) = self.rd.tiles_by_kind_name("PCIE3_RIGHT").iter().next() {
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let nl = self.builder.ndb.get_tile_class_naming("INTF_PCIE3_L");
            let nr = self.builder.ndb.get_tile_class_naming("INTF_PCIE3_R");
            for bdy in [0, 26] {
                for dy in 0..25 {
                    int_xy.push(Coord {
                        x: xy.x - 2,
                        y: xy.y - 26 + bdy + dy,
                    });
                    intf_xy.push((
                        Coord {
                            x: xy.x - 1,
                            y: xy.y - 26 + bdy + dy,
                        },
                        nr,
                    ));
                }
            }
            for bdy in [0, 26] {
                for dy in 0..25 {
                    int_xy.push(Coord {
                        x: xy.x + 11,
                        y: xy.y - 26 + bdy + dy,
                    });
                    intf_xy.push((
                        Coord {
                            x: xy.x + 10,
                            y: xy.y - 26 + bdy + dy,
                        },
                        nl,
                    ));
                }
            }
            let b_xy = Coord {
                x: xy.x,
                y: xy.y - 19,
            };
            let t_xy = Coord {
                x: xy.x,
                y: xy.y + 17,
            };
            self.builder.extract_xtile_bels_intf_id(
                tcls::PCIE3,
                xy,
                &[b_xy, t_xy],
                &int_xy,
                &intf_xy,
                "PCIE3",
                &[self.builder.bel_xy(bslots::PCIE3, "PCIE3", 0, 0)],
            );
        }
    }

    fn fill_hclk_tiles(&mut self) {
        if let Some(&xy) = self.rd.tiles_by_kind_name("HCLK_L").iter().next() {
            let bel_w = self.builder.bel_virtual(bslots::HCLK_DRP[0]);
            let bel_e = self.builder.bel_virtual(bslots::HCLK_DRP[1]);
            let mut bel = self.builder.bel_virtual(bslots::HCLK).naming_only();
            for i in 6..12 {
                bel = bel
                    .extra_int_out(
                        format!("LCLK{i}_S"),
                        &[format!("HCLK_LEAF_CLK_B_BOTL{ii}", ii = i - 6)],
                    )
                    .extra_int_out(
                        format!("LCLK{i}_N"),
                        &[format!("HCLK_LEAF_CLK_B_TOPL{ii}", ii = i - 6)],
                    );
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("HCLK{i}_I"), &[format!("HCLK_CK_OUTIN_L{i}")]);
            }
            for i in 8..12 {
                bel = bel
                    .extra_int_in(format!("HCLK{i}"), &[format!("HCLK_CK_BUFHCLK{i}")])
                    .extra_wire(
                        format!("HCLK{i}_O"),
                        &[format!("HCLK_CK_INOUT_L{ii}", ii = i - 8)],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_int_in(format!("RCLK{i}"), &[format!("HCLK_CK_BUFRCLK{i}")])
                    .extra_wire(
                        format!("RCLK{i}_O"),
                        &[format!("HCLK_CK_INOUT_L{ii}", ii = i + 4)],
                    );
            }
            bel = bel.sub_virtual().raw_tile(1);
            for i in 0..6 {
                bel = bel
                    .extra_int_out(format!("LCLK{i}_S"), &[format!("HCLK_LEAF_CLK_B_BOT{i}")])
                    .extra_int_out(format!("LCLK{i}_N"), &[format!("HCLK_LEAF_CLK_B_TOP{i}")]);
            }
            for i in 0..8 {
                bel = bel
                    .extra_int_in(format!("HCLK{i}"), &[format!("HCLK_CK_BUFHCLK{i}")])
                    .extra_wire(format!("HCLK{i}_O"), &[format!("HCLK_CK_INOUT_R{i}")]);
            }
            for i in 8..12 {
                bel = bel.extra_wire(
                    format!("HCLK{i}_I"),
                    &[format!("HCLK_CK_OUTIN_R{ii}", ii = i - 4)],
                );
            }
            for i in 0..4 {
                bel = bel.extra_wire(format!("RCLK{i}_I"), &[format!("HCLK_CK_OUTIN_R{i}")]);
            }
            self.builder
                .xtile_id(tcls::HCLK, "HCLK", xy)
                .raw_tile(xy.delta(1, 0))
                .num_cells(2)
                .bel(bel_w)
                .bel(bel_e)
                .bel(bel)
                .extract();

            let pips = self
                .builder
                .pips
                .entry((tcls::HCLK, bslots::HCLK))
                .or_default();
            for i in 0..12 {
                pips.pips.insert(
                    (wires::HCLK_BUF[i].cell(1), wires::HCLK_ROW[i].cell(1).pos()),
                    PipMode::Buf,
                );
            }
            for i in 0..4 {
                pips.pips.insert(
                    (wires::RCLK_BUF[i].cell(1), wires::RCLK_ROW[i].cell(1).pos()),
                    PipMode::Buf,
                );
            }
            for c in 0..2 {
                for o in 0..12 {
                    let dst = wires::LCLK[o].cell(c);
                    for i in 0..12 {
                        pips.pips
                            .insert((dst, wires::HCLK_BUF[i].cell(1).pos()), PipMode::Mux);
                    }
                    for i in 0..4 {
                        pips.pips
                            .insert((dst, wires::RCLK_BUF[i].cell(1).pos()), PipMode::Mux);
                    }
                }
            }
        }
    }

    fn fill_clk_rebuf_tiles(&mut self) {
        for (tcid, naming, tkn) in [
            (tcls::CLK_BUFG_REBUF, "CLK_BUFG_REBUF", "CLK_BUFG_REBUF"),
            (tcls::CLK_BALI_REBUF, "CLK_BALI_REBUF", "CLK_BALI_REBUF"),
            (
                tcls::CLK_BALI_REBUF,
                "CLK_BALI_REBUF",
                "CLK_BALI_REBUF_GTZ_TOP",
            ),
            (
                tcls::CLK_BALI_REBUF,
                "CLK_BALI_REBUF",
                "CLK_BALI_REBUF_GTZ_BOT",
            ),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let (bkd, bku, xd, xu, swz) = match tkn {
                    "CLK_BUFG_REBUF" => ("GCLK_TEST_BUF", "GCLK_TEST_BUF", 0, 1, false),
                    "CLK_BALI_REBUF" => ("GCLK_TEST_BUF", "GCLK_TEST_BUF", 0, 2, true),
                    "CLK_BALI_REBUF_GTZ_BOT" => ("GCLK_TEST_BUF", "BUFG_LB", 0, 0, true),
                    "CLK_BALI_REBUF_GTZ_TOP" => ("BUFG_LB", "GCLK_TEST_BUF", 0, 0, true),
                    _ => unreachable!(),
                };
                let mut bel = self.builder.bel_virtual(bslots::SPEC_INT);
                for i in 0..16 {
                    let y = if swz {
                        (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3
                    } else {
                        i
                    };
                    if i == 0 {
                        bel = self
                            .builder
                            .bel_xy(bslots::SPEC_INT, bkd, xd, y)
                            .naming_only();
                    } else {
                        bel = bel.sub_xy(self.rd, bkd, xd, y);
                    }
                    bel = bel
                        .pin_rename("CLKIN", format!("BUF{i}_S_CLKIN"))
                        .pin_rename("CLKOUT", format!("BUF{i}_S_CLKOUT"))
                        .pins_name_only(&[format!("BUF{i}_S_CLKIN"), format!("BUF{i}_S_CLKOUT")]);

                    let y = if swz {
                        (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3
                    } else {
                        i
                    };
                    bel = bel
                        .sub_xy(self.rd, bku, xu, y)
                        .pin_rename("CLKIN", format!("BUF{i}_N_CLKIN"))
                        .pin_rename("CLKOUT", format!("BUF{i}_N_CLKOUT"))
                        .pins_name_only(&[format!("BUF{i}_N_CLKIN"), format!("BUF{i}_N_CLKOUT")]);
                }
                for i in 0..32 {
                    bel = bel
                        .extra_wire(
                            format!("GCLK{i}_S"),
                            &[
                                format!("CLK_BUFG_REBUF_R_CK_GCLK{i}_BOT"),
                                format!("CLK_BALI_REBUF_R_GCLK{i}_BOT"),
                            ],
                        )
                        .extra_wire(
                            format!("GCLK{i}_N"),
                            &[
                                format!("CLK_BUFG_REBUF_R_CK_GCLK{i}_TOP"),
                                format!("CLK_BALI_REBUF_R_GCLK{i}_TOP"),
                            ],
                        );
                }
                self.builder
                    .xtile_id(tcid, naming, xy)
                    .num_cells(2)
                    .switchbox(bslots::SPEC_INT)
                    .optin_muxes(&wires::GCLK[..])
                    .bel(bel)
                    .extract();
            }
        }
        for tcid in [tcls::CLK_BUFG_REBUF, tcls::CLK_BALI_REBUF] {
            if let Some(pips) = self.builder.pips.get_mut(&(tcid, bslots::SPEC_INT)) {
                for mode in pips.pips.values_mut() {
                    *mode = PipMode::Buf;
                }
                for i in 0..32 {
                    pips.specials.insert(SwitchBoxItem::ProgInv(ProgInv {
                        dst: wires::GCLK_REBUF_TEST[i].cell(0),
                        src: wires::GCLK[i].cell(i % 2),
                        bit: PolTileBit::DUMMY,
                    }));
                }
                for c in 0..2 {
                    for i in 0..32 {
                        pips.specials
                            .insert(SwitchBoxItem::WireSupport(WireSupport {
                                wires: BTreeSet::from([wires::GCLK[i].cell(c)]),
                                bits: vec![],
                            }));
                    }
                }
            }
        }
    }

    fn fill_clk_hrow_tiles(&mut self) {
        for tkn in ["CLK_HROW_BOT_R", "CLK_HROW_TOP_R"] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut bels = vec![];
                for i in 0..12 {
                    bels.push(self.builder.bel_xy(bslots::BUFHCE_W[i], "BUFHCE", 0, i));
                }
                for i in 0..12 {
                    bels.push(self.builder.bel_xy(bslots::BUFHCE_E[i], "BUFHCE", 1, i));
                }
                let mut bel_int = self.builder.bel_virtual(bslots::SPEC_INT);
                for i in 0..32 {
                    if i == 0 {
                        bel_int = self
                            .builder
                            .bel_xy(bslots::SPEC_INT, "GCLK_TEST_BUF", i >> 4, i & 0xf ^ 0xf)
                            .naming_only();
                    } else {
                        bel_int = bel_int.sub_xy(self.rd, "GCLK_TEST_BUF", i >> 4, i & 0xf ^ 0xf);
                    }
                    let clkin = &format!("GCLK{i}_TEST_CLKIN");
                    let clkout = &format!("GCLK{i}_TEST_CLKOUT");
                    bel_int = bel_int
                        .pin_rename("CLKIN", clkin)
                        .pin_rename("CLKOUT", clkout)
                        .pins_name_only(&[clkin, clkout])
                        .extra_wire(
                            format!("GCLK{i}_TEST_IN"),
                            &[format!("CLK_HROW_CK_GCLK_IN_TEST{i}")],
                        )
                        .extra_wire(format!("GCLK{i}"), &[format!("CLK_HROW_R_CK_GCLK{i}")])
                        .extra_wire(
                            format!("GCLK{i}_TEST_OUT"),
                            &[format!("CLK_HROW_CK_GCLK_OUT_TEST{i}")],
                        )
                        .extra_wire(
                            format!("GCLK_TEST{i}"),
                            &[format!("CLK_HROW_CK_GCLK_TEST{i}")],
                        );
                }
                bel_int = bel_int
                    .sub_xy(self.rd, "GCLK_TEST_BUF", 3, 1)
                    .pin_rename("CLKIN", "HCLK_TEST_W_CLKIN")
                    .pin_rename("CLKOUT", "HCLK_TEST_W_CLKOUT")
                    .pins_name_only(&["HCLK_TEST_W_CLKIN", "HCLK_TEST_W_CLKOUT"])
                    .extra_wire("HCLK_TEST_IN_W", &["CLK_HROW_CK_IN_L_TEST_IN"])
                    .extra_wire("HCLK_TEST_OUT_W", &["CLK_HROW_CK_IN_L_TEST_OUT"])
                    .sub_xy(self.rd, "GCLK_TEST_BUF", 3, 0)
                    .pin_rename("CLKIN", "HCLK_TEST_E_CLKIN")
                    .pin_rename("CLKOUT", "HCLK_TEST_E_CLKOUT")
                    .pins_name_only(&["HCLK_TEST_E_CLKIN", "HCLK_TEST_E_CLKOUT"])
                    .extra_wire("HCLK_TEST_IN_E", &["CLK_HROW_CK_IN_R_TEST_IN"])
                    .extra_wire("HCLK_TEST_OUT_E", &["CLK_HROW_CK_IN_R_TEST_OUT"]);
                bels.push(bel_int);
                self.builder
                    .xtile_id(tcls::CLK_HROW, tkn, xy)
                    .num_cells(3)
                    .switchbox(bslots::SPEC_INT)
                    .optin_muxes(&wires::IMUX_BUFG_O[..])
                    .optin_muxes(&wires::IMUX_BUFHCE_W[..])
                    .optin_muxes(&wires::IMUX_BUFHCE_E[..])
                    .optin_muxes(&[wires::BUFH_TEST_W_IN])
                    .optin_muxes(&[wires::BUFH_TEST_E_IN])
                    .optin_muxes(&wires::CKINT_HROW[..])
                    .optin_muxes(&wires::GCLK_TEST_IN[..])
                    .ref_int(xy.delta(-2, -1), 0)
                    .ref_int(xy.delta(-2, 1), 1)
                    .bels(bels)
                    .extract();
            }
        }
        let pips = self
            .builder
            .pips
            .get_mut(&(tcls::CLK_HROW, bslots::SPEC_INT))
            .unwrap();
        for ((wt, _wf), mode) in pips.pips.iter_mut() {
            if wires::CKINT_HROW.contains(wt.wire) || wires::GCLK_TEST_IN.contains(wt.wire) {
                *mode = PipMode::Buf;
            }
        }
        // fixup GCLK_TEST outs
        for i in 0..32 {
            pips.pips.insert(
                (
                    wires::IMUX_BUFG_O[i].cell(1),
                    wires::GCLK_TEST[i ^ 1].cell(1).pos(),
                ),
                PipMode::Mux,
            );
        }
        // insert buffers
        for i in 0..32 {
            pips.pips.insert(
                (wires::GCLK_HROW[i].cell(1), wires::GCLK[i].cell(1).pos()),
                PipMode::Buf,
            );
        }
        for i in 0..4 {
            pips.pips.insert(
                (
                    wires::RCLK_HROW_W[i].cell(1),
                    wires::RCLK_ROW[i].cell(1).pos(),
                ),
                PipMode::Buf,
            );
        }
        for i in 0..4 {
            pips.pips.insert(
                (
                    wires::RCLK_HROW_E[i].cell(1),
                    wires::RCLK_ROW[i].cell(2).pos(),
                ),
                PipMode::Buf,
            );
        }
        for i in 0..14 {
            pips.pips.insert(
                (
                    wires::HROW_I_HROW_W[i].cell(1),
                    wires::HROW_I[i].cell(1).pos(),
                ),
                PipMode::Buf,
            );
        }
        for i in 0..14 {
            pips.pips.insert(
                (
                    wires::HROW_I_HROW_E[i].cell(1),
                    wires::HROW_I[i].cell(2).pos(),
                ),
                PipMode::Buf,
            );
        }
        pips.specials.insert(SwitchBoxItem::ProgInv(ProgInv {
            dst: wires::BUFH_TEST_W.cell(1),
            src: wires::BUFH_TEST_W_IN.cell(1),
            bit: PolTileBit::DUMMY,
        }));
        pips.specials.insert(SwitchBoxItem::ProgInv(ProgInv {
            dst: wires::BUFH_TEST_E.cell(1),
            src: wires::BUFH_TEST_E_IN.cell(1),
            bit: PolTileBit::DUMMY,
        }));
        for i in 0..32 {
            pips.specials.insert(SwitchBoxItem::ProgInv(ProgInv {
                dst: wires::GCLK_TEST[i].cell(1),
                src: wires::GCLK_TEST_IN[i].cell(1),
                bit: PolTileBit::DUMMY,
            }));
        }
    }

    fn fill_clk_bufg_tiles(&mut self) {
        for (tcid, naming, tkn) in [
            (tcls::CLK_BUFG_S, "CLK_BUFG_S", "CLK_BUFG_BOT_R"),
            (tcls::CLK_BUFG_N, "CLK_BUFG_N", "CLK_BUFG_TOP_R"),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut bels = vec![];
                for i in 0..16 {
                    bels.push(self.builder.bel_xy(bslots::BUFGCTRL[i], "BUFGCTRL", 0, i));
                }
                let intf = self.builder.ndb.get_tile_class_naming("INTF_R");
                let mut xt = self
                    .builder
                    .xtile_id(tcid, naming, xy)
                    .num_cells(4)
                    .switchbox(bslots::SPEC_INT)
                    .optin_muxes(&wires::IMUX_BUFG_O[..])
                    .optin_muxes(&wires::OUT_BUFG_GFB[..])
                    .optin_muxes(&wires::GCLK[..])
                    .bels(bels);
                for i in 0..4 {
                    xt = xt.ref_int(xy.delta(-2, i as i32), i).ref_single(
                        xy.delta(-1, i as i32),
                        i,
                        intf,
                    );
                }
                xt.extract();
                let pips = self
                    .builder
                    .pips
                    .get_mut(&(tcid, bslots::SPEC_INT))
                    .unwrap();
                let mut new_pips = vec![];
                pips.pips.retain(|&(wt, wf), _| {
                    if wires::OUT_BEL.contains(wf.wire) {
                        new_pips.push((wf.tw, wt.pos()));
                        false
                    } else {
                        true
                    }
                });
                for key in new_pips {
                    pips.pips.insert(key, PipMode::Buf);
                }
                for ((wt, _wf), mode) in pips.pips.iter_mut() {
                    if !wires::IMUX_BUFG_O.contains(wt.wire) {
                        *mode = PipMode::Buf;
                    }
                }
            }
        }
    }

    fn fill_clk_misc_tiles(&mut self) {
        for (tcid, tkn, slot, sslot, dy) in [
            (tcls::CLK_PMV, "CLK_PMV", bslots::PMV_CLK, "PMV", 3),
            (
                tcls::CLK_PMVIOB,
                "CLK_PMVIOB",
                bslots::PMVIOB_CLK,
                "PMVIOB",
                0,
            ),
            (
                tcls::CLK_PMV2_SVT,
                "CLK_PMV2_SVT",
                bslots::PMV2_SVT,
                "PMV",
                0,
            ),
            (tcls::CLK_PMV2, "CLK_PMV2", bslots::PMV2, "PMV", 0),
            (tcls::CLK_MTBF2, "CLK_MTBF2", bslots::MTBF2, "MTBF2", 0),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let bel = self.builder.bel_xy(slot, sslot, 0, 0);
                let intf = self.builder.ndb.get_tile_class_naming("INTF_R");
                self.builder
                    .xtile_id(tcid, tkn, xy)
                    .ref_int(xy.delta(-2, dy), 0)
                    .ref_single(xy.delta(-1, dy), 0, intf)
                    .bel(bel)
                    .extract();
            }
        }
    }

    fn fill_hclk_io_tiles(&mut self) {
        for (tkn, tcid, naming) in [
            ("HCLK_IOI", tcls::HCLK_IO_HP, "HCLK_IO_HP"),
            ("HCLK_IOI3", tcls::HCLK_IO_HR, "HCLK_IO_HR"),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_l = self
                    .rd
                    .tile_kinds
                    .key(self.rd.tiles[&xy.delta(0, 1)].kind)
                    .starts_with('L');
                let int_xy = if is_l {
                    if self.rd.tile_kinds.key(self.rd.tiles[&xy.delta(1, 0)].kind) == "HCLK_TERM" {
                        xy.delta(3, 0)
                    } else {
                        xy.delta(2, 0)
                    }
                } else {
                    if self.rd.tile_kinds.key(self.rd.tiles[&xy.delta(-1, 0)].kind) == "HCLK_TERM" {
                        xy.delta(-3, 0)
                    } else {
                        xy.delta(-2, 0)
                    }
                };
                let intf_xy = int_xy.delta(if is_l { -1 } else { 1 }, 0);
                let intf =
                    self.builder
                        .ndb
                        .get_tile_class_naming(if is_l { "INTF_L" } else { "INTF_R" });

                let mut bels = vec![];
                for i in 0..4 {
                    bels.push(self.builder.bel_xy(bslots::BUFIO[i], "BUFIO", 0, i ^ 2));
                }
                for i in 0..4 {
                    bels.push(self.builder.bel_xy(bslots::BUFR[i], "BUFR", 0, i ^ 2));
                }
                bels.push(self.builder.bel_xy(bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0));
                bels.push(self.builder.bel_virtual(bslots::BANK));
                if tcid == tcls::HCLK_IO_HP {
                    bels.push(
                        self.builder
                            .bel_xy(bslots::DCI, "DCI", 0, 0)
                            .pins_name_only(&[
                                "DCIDATA",
                                "DCISCLK",
                                "DCIADDRESS0",
                                "DCIADDRESS1",
                                "DCIADDRESS2",
                                "DCIIOUPDATE",
                                "DCIREFIOUPDATE",
                            ]),
                    );
                }
                let mut xn = self
                    .builder
                    .xtile_id(tcid, naming, xy)
                    .raw_tile(xy.delta(0, -4))
                    .raw_tile(xy.delta(0, -2))
                    .raw_tile(xy.delta(0, 1))
                    .raw_tile(xy.delta(0, 3))
                    .num_cells(8)
                    .switchbox(bslots::HCLK_IO_INT)
                    .optin_muxes(&wires::HCLK_IO[..])
                    .optin_muxes(&wires::RCLK_IO[..])
                    .optin_muxes(&wires::LCLK_IO[..])
                    .optin_muxes(&wires::IMUX_BUFIO[..])
                    .optin_muxes(&wires::IMUX_BUFR[..])
                    .optin_muxes(&[wires::IMUX_IDELAYCTRL_REFCLK]);
                for i in 0..4 {
                    xn = xn.ref_int(int_xy.delta(0, -4 + i as i32), i).ref_single(
                        intf_xy.delta(0, -4 + i as i32),
                        i,
                        intf,
                    );
                }
                for i in 0..4 {
                    xn = xn.ref_int(int_xy.delta(0, 1 + i as i32), i + 4).ref_single(
                        intf_xy.delta(0, 1 + i as i32),
                        i + 4,
                        intf,
                    );
                }
                xn.bels(bels).extract();

                let pips = self
                    .builder
                    .pips
                    .get_mut(&(tcid, bslots::HCLK_IO_INT))
                    .unwrap();
                for ((wt, _wf), mode) in pips.pips.iter_mut() {
                    if wires::RCLK_IO.contains(wt.wire) || wires::HCLK_IO.contains(wt.wire) {
                        *mode = PipMode::Buf;
                    }
                }
                for (wt, wf) in wires::PERF_IO.into_iter().zip(wires::PERF) {
                    let dst = wt.cell(4);
                    let src = wf.cell(4);
                    pips.pips.insert((dst, src.pos()), PipMode::Buf);
                }
            }
        }
    }

    fn fill_io_tiles(&mut self) {
        for tkn in [
            "LIOI",
            "LIOI_TBYTESRC",
            "LIOI_TBYTETERM",
            "LIOI3",
            "LIOI3_TBYTESRC",
            "LIOI3_TBYTETERM",
            "RIOI",
            "RIOI_TBYTESRC",
            "RIOI_TBYTETERM",
            "RIOI3",
            "RIOI3_TBYTESRC",
            "RIOI3_TBYTETERM",
            "LIOI_SING",
            "LIOI3_SING",
            "RIOI_SING",
            "RIOI3_SING",
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_hpio = !tkn.contains('3');
                let is_l = tkn.starts_with('L');
                let is_sing = tkn.contains("SING");
                let lr = if is_l { 'L' } else { 'R' };
                let iob_xy = xy.delta(if is_l { -1 } else { 1 }, 0);
                let int_xy = if is_l {
                    if self.rd.tile_kinds.key(self.rd.tiles[&xy.delta(1, 0)].kind) == "L_TERM_INT" {
                        xy.delta(3, 0)
                    } else {
                        xy.delta(2, 0)
                    }
                } else {
                    if self.rd.tile_kinds.key(self.rd.tiles[&xy.delta(-1, 0)].kind) == "R_TERM_INT"
                    {
                        xy.delta(-3, 0)
                    } else {
                        xy.delta(-2, 0)
                    }
                };
                let intf_xy = int_xy.delta(if is_l { -1 } else { 1 }, 0);
                let intf =
                    self.builder
                        .ndb
                        .get_tile_class_naming(if is_l { "INTF_L" } else { "INTF_R" });
                let mut bels = vec![];
                let num = if is_sing { 1 } else { 2 };
                for i in 0..num {
                    let ix = if is_sing { i } else { i ^ 1 };
                    let mut bel = self
                        .builder
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
                        ])
                        .pin_dummy("REV")
                        .extra_wire(
                            "IOB_I",
                            &[format!("LIOI_IBUF{ix}"), format!("RIOI_IBUF{ix}")],
                        )
                        .extra_wire("IOB_I_BUF", &[format!("LIOI_I{ix}"), format!("RIOI_I{ix}")]);
                    if i == 1 || is_sing {
                        bel = bel.pin_dummy("SHIFTIN1").pin_dummy("SHIFTIN2");
                    }
                    if i == 1 {
                        bel = bel.extra_int_out_force(
                            "CLKPAD",
                            wires::OUT_CLKPAD.cell(1),
                            format!("{lr}IOI_I2GCLK_TOP0"),
                        )
                    }
                    bels.push(bel);
                }
                for i in 0..num {
                    let ix = if is_sing { i } else { i ^ 1 };
                    let mut bel = self
                        .builder
                        .bel_xy(bslots::OLOGIC[i], "OLOGIC", 0, i)
                        .pin_rename("CLK", "CLK_FAKE")
                        .pin_rename("CLKB", "CLKB_FAKE")
                        .pin_rename("TFB", "TFB_FAKE")
                        .pins_name_only(&[
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
                            "TBYTEIN",
                            "TBYTEOUT",
                        ])
                        .pin_dummy("REV")
                        .extra_int_in("CLK", &[format!("IOI_OCLK_{ix}")])
                        .extra_int_in("CLKB", &[format!("IOI_OCLKM_{ix}")])
                        .extra_int_out(
                            "TFB",
                            &[
                                format!("LIOI_OLOGIC{ix}_TFB_LOCAL"),
                                format!("RIOI_OLOGIC{ix}_TFB_LOCAL"),
                            ],
                        )
                        .extra_wire("IOB_O", &[format!("LIOI_O{ix}"), format!("RIOI_O{ix}")])
                        .extra_wire("IOB_T", &[format!("LIOI_T{ix}"), format!("RIOI_T{ix}")])
                        .extra_wire(
                            "TBYTEIN_IOI",
                            &["IOI_TBYTEIN", "IOI_SING_TBYTEIN", "IOI_TBYTEIN_TERM"],
                        );
                    if i == 0 {
                        bel = bel.pin_dummy("SHIFTIN1").pin_dummy("SHIFTIN2");
                    }
                    bels.push(bel);
                }
                for i in 0..num {
                    bels.push(
                        self.builder
                            .bel_xy(bslots::IDELAY[i], "IDELAY", 0, i)
                            .pins_name_only(&["IDATAIN", "DATAOUT"]),
                    );
                }
                if is_hpio {
                    for i in 0..num {
                        bels.push(
                            self.builder
                                .bel_xy(bslots::ODELAY[i], "ODELAY", 0, i)
                                .pins_name_only(&["ODATAIN", "CLKIN"]),
                        );
                    }
                }
                for i in 0..num {
                    let mut bel = self
                        .builder
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
                            "T_OUT",
                            "T_IN",
                        ]);
                    if i == 1 || is_sing {
                        bel = bel
                            .pin_dummy("DIFF_TERM_INT_EN")
                            .pin_dummy("DIFFO_IN")
                            .pin_dummy("O_IN")
                            .pin_dummy("T_IN");
                    }
                    if is_sing {
                        bel = bel.pin_dummy("DIFFI_IN");
                    }
                    let pn = if i == 1 { 'P' } else { 'N' };
                    bel = bel.extra_wire_force("MONITOR", format!("{lr}IOB_MONITOR_{pn}"));
                    bels.push(bel);
                }

                let tcids = if is_sing {
                    if is_hpio {
                        [tcls::IO_HP_S, tcls::IO_HP_N].as_slice()
                    } else {
                        [tcls::IO_HR_S, tcls::IO_HR_N].as_slice()
                    }
                } else {
                    if is_hpio {
                        [tcls::IO_HP_PAIR].as_slice()
                    } else {
                        [tcls::IO_HR_PAIR].as_slice()
                    }
                };
                for &tcid in tcids {
                    let mut xt = self
                        .builder
                        .xtile_id(tcid, tkn, xy)
                        .raw_tile(iob_xy)
                        .switchbox(bslots::SPEC_INT)
                        .optin_muxes(&wires::IMUX_IOI_ICLK[..])
                        .optin_muxes(&[wires::IMUX_IOI_ICLKDIVP])
                        .optin_muxes(&wires::IMUX_IOI_OCLK[..])
                        .optin_muxes(&wires::IMUX_IOI_OCLKDIV[..])
                        .optin_muxes(&wires::IMUX_IOI_OCLKDIVF[..])
                        .optin_muxes(&[wires::IMUX_SPEC[0], wires::IMUX_SPEC[2]])
                        .bels(bels.clone())
                        .num_cells(num);
                    if is_sing {
                        xt = xt.ref_int(int_xy, 0).ref_single(intf_xy, 0, intf)
                    } else {
                        xt = xt
                            .ref_int(int_xy, 0)
                            .ref_single(intf_xy, 0, intf)
                            .ref_int(int_xy.delta(0, 1), 1)
                            .ref_single(intf_xy.delta(0, 1), 1, intf)
                    }
                    xt.extract();
                }
            }
        }
        for (tcid, num_cells) in [
            (tcls::IO_HP_S, 1),
            (tcls::IO_HR_S, 1),
            (tcls::IO_HP_N, 1),
            (tcls::IO_HR_N, 1),
            (tcls::IO_HP_PAIR, 2),
            (tcls::IO_HR_PAIR, 2),
        ] {
            if let Some(pips) = self.builder.pips.get_mut(&(tcid, bslots::SPEC_INT)) {
                pips.pips.retain(|&(wt, wf), _| {
                    !(wires::IMUX_IOI_OCLKDIV.contains(wt.wire) && wf.wire != wires::PHASER_OCLKDIV)
                });
                for ((wt, _wf), mode) in pips.pips.iter_mut() {
                    if wires::IMUX_SPEC.contains(wt.wire) {
                        *mode = PipMode::PermaBuf;
                    }
                }
                for c in 0..num_cells {
                    pips.pips.insert(
                        (
                            wires::IMUX_IOI_OCLK[1].cell(c),
                            wires::PHASER_OCLK90.cell(c).pos(),
                        ),
                        PipMode::Mux,
                    );
                    pips.pips.insert(
                        (
                            wires::IMUX_IOI_OCLKDIV[0].cell(c),
                            wires::IMUX_IOI_OCLKDIVF[0].cell(c).pos(),
                        ),
                        PipMode::Mux,
                    );
                    pips.pips.insert(
                        (
                            wires::IMUX_IOI_OCLKDIV[1].cell(c),
                            wires::IMUX_IOI_OCLKDIVF[1].cell(c).pos(),
                        ),
                        PipMode::Mux,
                    );
                }
            }
        }
    }

    fn fill_cmt_tiles(&mut self) {
        for tkn in ["CMT_TOP_L_LOWER_B", "CMT_TOP_R_LOWER_B"] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_l = tkn == "CMT_TOP_L_LOWER_B";
                let int_xy = xy.delta(if is_l { 3 } else { -3 }, -8);
                let intf_xy = xy.delta(if is_l { 2 } else { -2 }, -8);
                let intf =
                    self.builder
                        .ndb
                        .get_tile_class_naming(if is_l { "INTF_L" } else { "INTF_R" });
                let mut bels = vec![];
                for i in 0..4 {
                    let abcd = ['A', 'B', 'C', 'D'][i];
                    let mut bel = self
                        .builder
                        .bel_xy(bslots::PHASER_IN[i], "PHASER_IN_PHY", 0, i % 2)
                        .raw_tile(1 + i / 2)
                        .extra_wire(
                            "FIFO_WRCLK",
                            &[
                                format!("CMT_PHASER_IN_{abcd}_WRCLK_TOFIFO"),
                                format!("CMT_R_PHASER_IN_{abcd}_WRCLK_TOFIFO"),
                                format!("CMT_R_PHASER_IN_{abcd}_WRCLK_FIFO"),
                            ],
                        )
                        .extra_wire(
                            "FIFO_WREN",
                            &[
                                format!("CMT_PHASER_IN_{abcd}_WREN_TOFIFO"),
                                format!("CMT_PHASER_IN_{abcd}_WRENABLE_FIFO"),
                            ],
                        );
                    for pin in [
                        "ENCALIBPHY0",
                        "ENCALIBPHY1",
                        "RANKSELPHY0",
                        "RANKSELPHY1",
                        "BURSTPENDINGPHY",
                    ] {
                        bel = bel.pin_name_only(pin, 1);
                    }
                    bels.push(bel);
                }
                for i in 0..4 {
                    let abcd = ['A', 'B', 'C', 'D'][i];
                    let mut bel = self
                        .builder
                        .bel_xy(bslots::PHASER_OUT[i], "PHASER_OUT_PHY", 0, i % 2)
                        .raw_tile(1 + i / 2)
                        .extra_wire(
                            "FIFO_RDCLK",
                            &[
                                format!("CMT_PHASER_OUT_{abcd}_RDCLK_TOFIFO"),
                                format!("CMT_R_PHASER_OUT_{abcd}_RDCLK_TOFIFO"),
                                format!("CMT_R_PHASER_OUT_{abcd}_RDCLK_FIFO"),
                            ],
                        )
                        .extra_wire(
                            "FIFO_RDEN",
                            &[
                                format!("CMT_PHASER_OUT_{abcd}_RDEN_TOFIFO"),
                                format!("CMT_PHASER_OUT_{abcd}_RDENABLE_TOFIFO"),
                                format!("CMT_R_PHASER_OUT_{abcd}_RDENABLE_TOFIFO"),
                                format!("CMT_R_PHASER_OUT_{abcd}_RDENABLE_FIFO"),
                            ],
                        );
                    for pin in ["ENCALIBPHY0", "ENCALIBPHY1", "BURSTPENDINGPHY"] {
                        bel = bel.pin_name_only(pin, 1);
                    }
                    bels.push(bel);
                }
                bels.push(
                    self.builder
                        .bel_xy(bslots::PHASER_REF, "PHASER_REF", 0, 0)
                        .raw_tile(2),
                );
                let mut bel_pc = self
                    .builder
                    .bel_xy(bslots::PHY_CONTROL, "PHY_CONTROL", 0, 0)
                    .raw_tile(2)
                    .extra_wire("PHYCTLMSTREMPTY_FAR", &["CMT_PHASERTOP_PHYCTLMSTREMPTY"])
                    .extra_wire("SYNC_BB", &["CMT_PHASER_TOP_SYNC_BB"]);
                for pin in [
                    "INRANKA0",
                    "INRANKA1",
                    "INRANKB0",
                    "INRANKB1",
                    "INRANKC0",
                    "INRANKC1",
                    "INRANKD0",
                    "INRANKD1",
                    "PCENABLECALIB0",
                    "PCENABLECALIB1",
                    "INBURSTPENDING0",
                    "INBURSTPENDING1",
                    "INBURSTPENDING2",
                    "INBURSTPENDING3",
                    "OUTBURSTPENDING0",
                    "OUTBURSTPENDING1",
                    "OUTBURSTPENDING2",
                    "OUTBURSTPENDING3",
                ] {
                    bel_pc = bel_pc.pin_name_only(pin, 1);
                }
                bels.push(bel_pc);
                bels.extend([
                    self.builder
                        .bel_xy(bslots::PLL[0], "MMCME2_ADV", 0, 0)
                        .raw_tile(0)
                        .extra_wire("CLKFB", &["CMT_LR_LOWER_B_CLKFBOUT2IN"]),
                    self.builder
                        .bel_xy(bslots::PLL[1], "PLLE2_ADV", 0, 0)
                        .raw_tile(3)
                        .extra_wire("CLKFB", &["CMT_TOP_L_CLKFBOUT2IN", "CMT_TOP_R_CLKFBOUT2IN"]),
                ]);
                for i in 0..2 {
                    bels.push(
                        self.builder
                            .bel_xy(bslots::BUFMRCE[i], "BUFMRCE", 0, i)
                            .raw_tile(4),
                    );
                }
                let mut bel_int = self
                    .builder
                    .bel_virtual(bslots::SPEC_INT)
                    .naming_only()
                    .raw_tile(0);
                for i in 0..4 {
                    bel_int = bel_int
                        .extra_wire(
                            format!("FREQ_BB{i}_MMCM_IN"),
                            &[
                                format!("CMT_L_LOWER_B_CLK_FREQ_BB{ii}", ii = i ^ 3),
                                format!("CMT_R_LOWER_B_CLK_FREQ_BB{ii}", ii = i ^ 3),
                            ],
                        )
                        .extra_wire(format!("FREQ_BB{i}"), &[format!("MMCM_CLK_FREQ_BB_NS{i}")]);
                }
                bels.push(bel_int);

                let naming = if is_l { "CMT_W" } else { "CMT_E" };
                let mut xn = self
                    .builder
                    .xtile_id(tcls::CMT, naming, xy)
                    .num_cells(50)
                    .force_ext_pips()
                    .raw_tile(xy.delta(0, 9))
                    .raw_tile(xy.delta(0, 22))
                    .raw_tile(xy.delta(0, 35))
                    .raw_tile(xy.delta(0, 17))
                    .switchbox(bslots::SPEC_INT)
                    .optin_muxes(&wires::LCLK_CMT_S[..])
                    .optin_muxes(&wires::LCLK_CMT_N[..])
                    .optin_muxes(&wires::IMUX_BUFMRCE[..])
                    .optin_muxes(&wires::IMUX_PLL_CLKIN1_HCLK[..])
                    .optin_muxes(&wires::IMUX_PLL_CLKIN2_HCLK[..])
                    .optin_muxes(&wires::IMUX_PLL_CLKFB_HCLK[..])
                    .optin_muxes(&wires::IMUX_PLL_CLKIN1[..])
                    .optin_muxes(&wires::IMUX_PLL_CLKIN2[..])
                    .optin_muxes(&wires::IMUX_PLL_CLKFB[..])
                    .optin_muxes(&wires::IMUX_PHASER_REFMUX[..])
                    .optin_muxes(&wires::IMUX_PHASER_IN_PHASEREFCLK[..])
                    .optin_muxes(&wires::IMUX_PHASER_OUT_PHASEREFCLK[..])
                    .optin_muxes(&wires::OMUX_PLL_PERF[..])
                    .optin_muxes(&wires::OMUX_CCIO[..])
                    .optin_muxes(&wires::OMUX_HCLK_FREQ_BB[..])
                    .optin_muxes(&wires::OMUX_PLL_FREQ_BB_S[..])
                    .optin_muxes(&wires::OMUX_PLL_FREQ_BB_N[..])
                    .optin_muxes(&wires::OUT_PLL_FREQ_BB_S[..])
                    .optin_muxes(&wires::OUT_PLL_FREQ_BB_N[..])
                    .optin_muxes(&wires::CMT_FREQ_BB[..])
                    .optin_muxes(&wires::CMT_FREQ_BB_S[..])
                    .optin_muxes(&wires::CMT_FREQ_BB_N[..])
                    .optin_muxes(&[wires::CMT_SYNC_BB])
                    .optin_muxes(&[wires::CMT_SYNC_BB_S])
                    .optin_muxes(&[wires::CMT_SYNC_BB_N])
                    .optin_muxes(&wires::HROW_O[..])
                    .optin_muxes(&wires::CKINT_CMT[..])
                    .optin_muxes(&wires::PERF[..])
                    .skip_edge("CMT_LR_LOWER_B_MMCM_CLKFBIN", "CMT_LR_LOWER_B_CLKFBOUT2IN")
                    .skip_edge("CMT_TOP_R_UPPER_T_PLLE2_CLKFBIN", "CMT_TOP_L_CLKFBOUT2IN")
                    .skip_edge("CMT_TOP_R_UPPER_T_PLLE2_CLKFBIN", "CMT_TOP_R_CLKFBOUT2IN");
                for i in 0..25 {
                    xn = xn.ref_int(int_xy.delta(0, i as i32), i).ref_single(
                        intf_xy.delta(0, i as i32),
                        i,
                        intf,
                    );
                }
                for i in 0..25 {
                    xn = xn
                        .ref_int(int_xy.delta(0, i as i32 + 26), i + 25)
                        .ref_single(intf_xy.delta(0, i as i32 + 26), i + 25, intf);
                }
                xn.bels(bels).extract();
                let naming = self
                    .builder
                    .ndb
                    .tile_class_namings
                    .get_mut(naming)
                    .unwrap()
                    .1;
                for i in 0..4 {
                    let dst = wires::PERF[i].cell(75);
                    for j in 0..4 {
                        let pn = naming
                            .ext_pips
                            .remove(&(dst, wires::OUT_PHASER_IN_RCLK[j].cell(25)))
                            .unwrap();
                        naming
                            .ext_pips
                            .insert((dst, wires::PERF_IN_PHASER[j].cell(25)), pn);
                    }
                    for j in [i, i ^ 1] {
                        let pn = naming
                            .ext_pips
                            .remove(&(dst, wires::OMUX_PLL_PERF[j].cell(25)))
                            .unwrap();
                        naming
                            .ext_pips
                            .insert((dst, wires::PERF_IN_PLL[j].cell(25)), pn);
                    }
                }
                // fixup so they exist even for smol devices.
                for i in 0..4 {
                    naming.ext_pips.insert(
                        (
                            wires::CMT_FREQ_BB[i].cell(25),
                            wires::CMT_FREQ_BB_S[i].cell(25),
                        ),
                        PipNaming {
                            tile: RawTileId::from_idx(0),
                            wire_to: format!("MMCM_CLK_FREQ_BB_NS{i}"),
                            wire_from: format!("MMCM_CLK_FREQ_BB_REBUF{i}_NS"),
                        },
                    );
                    naming.ext_pips.insert(
                        (
                            wires::CMT_FREQ_BB_S[i].cell(25),
                            wires::CMT_FREQ_BB[i].cell(25),
                        ),
                        PipNaming {
                            tile: RawTileId::from_idx(0),
                            wire_to: format!("MMCM_CLK_FREQ_BB_REBUF{i}_NS"),
                            wire_from: format!("MMCM_CLK_FREQ_BB_NS{i}"),
                        },
                    );

                    naming.ext_pips.insert(
                        (
                            wires::CMT_FREQ_BB[i].cell(25),
                            wires::CMT_FREQ_BB_N[i].cell(25),
                        ),
                        PipNaming {
                            tile: RawTileId::from_idx(3),
                            wire_to: format!("PLL_CLK_FREQ_BB{i}_NS"),
                            wire_from: format!("PLL_CLK_FREQ_BB_BUFOUT_NS{i}"),
                        },
                    );
                    naming.ext_pips.insert(
                        (
                            wires::CMT_FREQ_BB_N[i].cell(25),
                            wires::CMT_FREQ_BB[i].cell(25),
                        ),
                        PipNaming {
                            tile: RawTileId::from_idx(3),
                            wire_to: format!("PLL_CLK_FREQ_BB_BUFOUT_NS{i}"),
                            wire_from: format!("PLL_CLK_FREQ_BB{i}_NS"),
                        },
                    );

                    naming.ext_pips.remove(&(
                        wires::CMT_FREQ_BB[i].cell(25),
                        wires::CMT_FREQ_BB[i].cell(25),
                    ));
                }
                naming.ext_pips.insert(
                    (wires::CMT_SYNC_BB.cell(25), wires::CMT_SYNC_BB_S.cell(25)),
                    PipNaming {
                        tile: RawTileId::from_idx(0),
                        wire_to: "CMT_MMCM_PHYCTRL_SYNC_BB_UP".into(),
                        wire_from: "CMT_MMCM_PHYCTRL_SYNC_BB_DN".into(),
                    },
                );
                naming.ext_pips.insert(
                    (wires::CMT_SYNC_BB_S.cell(25), wires::CMT_SYNC_BB.cell(25)),
                    PipNaming {
                        tile: RawTileId::from_idx(0),
                        wire_to: "CMT_MMCM_PHYCTRL_SYNC_BB_DN".into(),
                        wire_from: "CMT_MMCM_PHYCTRL_SYNC_BB_UP".into(),
                    },
                );

                naming.ext_pips.insert(
                    (wires::CMT_SYNC_BB.cell(25), wires::CMT_SYNC_BB_N.cell(25)),
                    PipNaming {
                        tile: RawTileId::from_idx(3),
                        wire_to: "CMT_PLL_PHYCTRL_SYNC_BB_DN".into(),
                        wire_from: "CMT_PLL_PHYCTRL_SYNC_BB_UP".into(),
                    },
                );
                naming.ext_pips.insert(
                    (wires::CMT_SYNC_BB_N.cell(25), wires::CMT_SYNC_BB.cell(25)),
                    PipNaming {
                        tile: RawTileId::from_idx(3),
                        wire_to: "CMT_PLL_PHYCTRL_SYNC_BB_UP".into(),
                        wire_from: "CMT_PLL_PHYCTRL_SYNC_BB_DN".into(),
                    },
                );
            }
        }
        let pips = self
            .builder
            .pips
            .get_mut(&(tcls::CMT, bslots::SPEC_INT))
            .unwrap();
        for ((wt, _wf), mode) in pips.pips.iter_mut() {
            if wires::CKINT_CMT.contains(wt.wire)
                || wires::OUT_PLL_FREQ_BB_S.contains(wt.wire)
                || wires::OUT_PLL_FREQ_BB_N.contains(wt.wire)
                || wt.wire == wires::CMT_SYNC_BB
            {
                *mode = PipMode::Buf;
            }
        }
        // insert buffers
        for i in 0..12 {
            pips.pips.insert(
                (
                    wires::HCLK_CMT[i].cell(25),
                    wires::HCLK_ROW[i].cell(25).pos(),
                ),
                PipMode::Buf,
            );
        }
        for i in 0..4 {
            pips.pips.insert(
                (
                    wires::RCLK_CMT[i].cell(25),
                    wires::RCLK_ROW[i].cell(25).pos(),
                ),
                PipMode::Buf,
            );
        }
        for i in 4..14 {
            pips.pips.insert(
                (
                    wires::HROW_I_CMT[i].cell(25),
                    wires::HROW_I[i].cell(25).pos(),
                ),
                PipMode::Buf,
            );
        }
        for i in 0..4 {
            pips.pips.insert(
                (
                    wires::CCIO_CMT[i].cell(25),
                    wires::OUT_CLKPAD.cell([76, 78, 72, 74][i]).pos(),
                ),
                PipMode::Buf,
            );
        }
        // fix up OMUX_CCIO
        for i in 0..4 {
            pips.pips.insert(
                (
                    wires::OMUX_CCIO[i].cell(25),
                    wires::CCIO_CMT[i].cell(25).pos(),
                ),
                PipMode::Mux,
            );
        }
        for i in 0..14 {
            for j in 0..4 {
                pips.pips
                    .remove(&(wires::HROW_O[i].cell(25), wires::CCIO_CMT[j].cell(25).pos()));
            }
        }
        for wt in [
            wires::IMUX_PLL_CLKIN1_HCLK,
            wires::IMUX_PLL_CLKIN2_HCLK,
            wires::IMUX_PLL_CLKFB_HCLK,
        ]
        .into_iter()
        .flatten()
        {
            for j in 0..4 {
                pips.pips
                    .remove(&(wt.cell(25), wires::CCIO_CMT[j].cell(25).pos()));
            }
        }
        for i in 0..4 {
            pips.pips.insert(
                (
                    wires::PERF_IN_PLL[i].cell(25),
                    wires::OMUX_PLL_PERF[i].cell(25).pos(),
                ),
                PipMode::Buf,
            );
            pips.pips.insert(
                (
                    wires::PERF_IN_PHASER[i].cell(25),
                    wires::OUT_PHASER_IN_RCLK[i].cell(25).pos(),
                ),
                PipMode::Buf,
            );

            let dst = wires::PERF[i].cell(75);
            for j in 0..4 {
                pips.pips
                    .remove(&(dst, wires::OUT_PHASER_IN_RCLK[j].cell(25).pos()));
                pips.pips
                    .insert((dst, wires::PERF_IN_PHASER[j].cell(25).pos()), PipMode::Mux);
            }
            for j in [i, i ^ 1] {
                pips.pips
                    .remove(&(dst, wires::OMUX_PLL_PERF[j].cell(25).pos()));
                pips.pips
                    .insert((dst, wires::PERF_IN_PLL[j].cell(25).pos()), PipMode::Mux);
            }
        }
        // ensure these exist even on smol devices
        for i in 0..4 {
            for (wt, wf) in [
                (wires::CMT_FREQ_BB, wires::CMT_FREQ_BB_S),
                (wires::CMT_FREQ_BB_S, wires::CMT_FREQ_BB),
                (wires::CMT_FREQ_BB, wires::CMT_FREQ_BB_N),
                (wires::CMT_FREQ_BB_N, wires::CMT_FREQ_BB),
            ] {
                pips.pips
                    .insert((wt[i].cell(25), wf[i].cell(25).pos()), PipMode::Mux);
            }
            pips.pips.remove(&(
                wires::CMT_FREQ_BB[i].cell(25),
                wires::CMT_FREQ_BB[i].cell(25).pos(),
            ));
        }
        for (wt, wf) in [
            (wires::CMT_SYNC_BB, wires::CMT_SYNC_BB_S),
            (wires::CMT_SYNC_BB_S, wires::CMT_SYNC_BB),
            (wires::CMT_SYNC_BB, wires::CMT_SYNC_BB_N),
            (wires::CMT_SYNC_BB_N, wires::CMT_SYNC_BB),
        ] {
            pips.pips
                .insert((wt.cell(25), wf.cell(25).pos()), PipMode::Buf);
        }
        for i in 0..4 {
            for w in [
                wires::CMT_FREQ_BB[i],
                wires::CMT_FREQ_BB_S[i],
                wires::CMT_FREQ_BB_N[i],
            ] {
                pips.specials
                    .insert(SwitchBoxItem::WireSupport(WireSupport {
                        wires: BTreeSet::from([w.cell(25)]),
                        bits: vec![],
                    }));
            }
        }
        for w in [
            wires::CMT_SYNC_BB,
            wires::CMT_SYNC_BB_S,
            wires::CMT_SYNC_BB_N,
        ] {
            pips.specials
                .insert(SwitchBoxItem::WireSupport(WireSupport {
                    wires: BTreeSet::from([w.cell(25)]),
                    bits: vec![],
                }));
        }
    }

    fn fill_cmt_fifo_tiles(&mut self) {
        for tkn in ["CMT_FIFO_L", "CMT_FIFO_R"] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_l = tkn == "CMT_FIFO_L";
                let int_xy = xy.delta(if is_l { 2 } else { -2 }, -6);
                let intf_xy = xy.delta(if is_l { 1 } else { -1 }, -6);
                let intf =
                    self.builder
                        .ndb
                        .get_tile_class_naming(if is_l { "INTF_L" } else { "INTF_R" });
                let bels = [
                    self.builder
                        .bel_xy(bslots::IN_FIFO, "IN_FIFO", 0, 0)
                        .extra_wire("PHASER_WRCLK", &["CMT_FIFO_L_PHASER_WRCLK"])
                        .extra_wire("PHASER_WREN", &["CMT_FIFO_L_PHASER_WRENABLE"]),
                    self.builder
                        .bel_xy(bslots::OUT_FIFO, "OUT_FIFO", 0, 0)
                        .extra_wire("PHASER_RDCLK", &["CMT_FIFO_L_PHASER_RDCLK"])
                        .extra_wire("PHASER_RDEN", &["CMT_FIFO_L_PHASER_RDENABLE"]),
                ];
                let mut xn = self
                    .builder
                    .xtile_id(tcls::CMT_FIFO, tkn, xy)
                    .num_cells(12)
                    .switchbox(bslots::CMT_FIFO_INT)
                    .optin_muxes(&[
                        wires::FIFO_IWRCLK,
                        wires::FIFO_IWREN,
                        wires::FIFO_ORDCLK,
                        wires::FIFO_ORDEN,
                    ]);
                for i in 0..12 {
                    xn = xn.ref_int(int_xy.delta(0, i as i32), i).ref_single(
                        intf_xy.delta(0, i as i32),
                        i,
                        intf,
                    );
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_cfg_tiles(&mut self) {
        if let Some(&xy_m) = self.rd.tiles_by_kind_name("CFG_CENTER_MID").iter().next() {
            let xy_b = xy_m.delta(0, -21);
            let xy_t = xy_m.delta(0, 10);
            let intf = self.builder.ndb.get_tile_class_naming("INTF_L");
            let bels = [
                self.builder
                    .bel_xy(bslots::BSCAN[0], "BSCAN", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::BSCAN[1], "BSCAN", 0, 1)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::BSCAN[2], "BSCAN", 0, 2)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::BSCAN[3], "BSCAN", 0, 3)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::ICAP[0], "ICAP", 0, 0)
                    .pin_rename("CSIB", "CSB")
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::ICAP[1], "ICAP", 0, 1)
                    .pin_rename("CSIB", "CSB")
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::STARTUP, "STARTUP", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::CAPTURE, "CAPTURE", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::FRAME_ECC, "FRAME_ECC", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::USR_ACCESS, "USR_ACCESS", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::CFG_IO_ACCESS, "CFG_IO_ACCESS", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::PMVIOB_CFG, "PMVIOB", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::DCIRESET, "DCIRESET", 0, 0)
                    .raw_tile(1),
                self.builder
                    .bel_xy(bslots::DNA_PORT, "DNA_PORT", 0, 0)
                    .raw_tile(2),
                self.builder
                    .bel_xy(bslots::EFUSE_USR, "EFUSE_USR", 0, 0)
                    .raw_tile(2),
            ];
            let mut xn = self
                .builder
                .xtile_id(tcls::CFG, "CFG", xy_b)
                .raw_tile(xy_m)
                .raw_tile(xy_t)
                .num_cells(50);
            for i in 0..25 {
                xn = xn.ref_int(xy_b.delta(3, -10 + i as i32), i).ref_single(
                    xy_b.delta(2, -10 + i as i32),
                    i,
                    intf,
                );
            }
            for i in 0..25 {
                xn = xn.ref_int(xy_b.delta(3, i as i32 + 16), i + 25).ref_single(
                    xy_b.delta(2, i as i32 + 16),
                    i + 25,
                    intf,
                );
            }
            xn.bels(bels).extract();
        }
    }

    fn fill_sysmon_tiles(&mut self) {
        for (tkn, naming) in [
            ("MONITOR_BOT", "SYSMON_WE"),
            ("MONITOR_BOT_FUJI2", "SYSMON_W"),
            ("MONITOR_BOT_PELE1", "SYSMON_E"),
        ] {
            if let Some(&xy_b) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let xy_m = xy_b.delta(0, 10);
                let xy_t = xy_b.delta(0, 20);
                let intf = self.builder.ndb.get_tile_class_naming("INTF_L");
                let mut bel = self
                    .builder
                    .bel_xy(bslots::SYSMON, "XADC", 0, 0)
                    .pins_name_only(&["VP", "VN"]);
                for i in 0..16 {
                    if naming == "SYSMON_W" && matches!(i, 6 | 7 | 13 | 14 | 15) {
                        bel = bel
                            .pin_dummy(format!("VAUXP{i}"))
                            .pin_dummy(format!("VAUXN{i}"));
                    } else {
                        bel = bel
                            .pin_name_only(&format!("VAUXP{i}"), 2)
                            .pin_name_only(&format!("VAUXN{i}"), 2);
                    }
                }
                bel = bel
                    .sub_xy(self.rd, "IPAD", 0, 0)
                    .pin_rename("O", "IPAD_VP_O")
                    .pins_name_only(&["IPAD_VP_O"])
                    .sub_xy(self.rd, "IPAD", 0, 1)
                    .pin_rename("O", "IPAD_VN_O")
                    .pins_name_only(&["IPAD_VN_O"]);
                let mut xn = self
                    .builder
                    .xtile_id(tcls::SYSMON, naming, xy_b)
                    .raw_tile(xy_m)
                    .raw_tile(xy_t)
                    .num_cells(25);
                if naming == "SYSMON_E" {
                    xn = xn
                        .raw_tile(xy_b.delta(0, -26))
                        .raw_tile(xy_b.delta(0, -16))
                        .raw_tile(xy_b.delta(0, -6))
                }
                for i in 0..25 {
                    xn = xn.ref_int(xy_b.delta(3, i as i32), i).ref_single(
                        xy_b.delta(2, i as i32),
                        i,
                        intf,
                    );
                }
                xn.bel(bel).extract();
            }
        }
    }

    fn fill_ps_tiles(&mut self) {
        if let Some(&xy_pss0) = self.rd.tiles_by_kind_name("PSS0").iter().next() {
            let int_xy = xy_pss0.delta(19, -10);
            let xy_pss1 = xy_pss0.delta(0, 21);
            let xy_pss2 = xy_pss0.delta(0, 42);
            let xy_pss3 = xy_pss0.delta(0, 62);
            let xy_pss4 = xy_pss0.delta(0, 83);
            let intf = self.builder.ndb.get_tile_class_naming("INTF_PSS");
            let mut pins = vec![];
            pins.push(("DDRWEB".to_string(), 1));
            pins.push(("DDRVRN".to_string(), 2));
            pins.push(("DDRVRP".to_string(), 3));
            for i in 0..13 {
                pins.push((format!("DDRA{i}"), 4 + i));
            }
            pins.push(("DDRA14".to_string(), 17));
            pins.push(("DDRA13".to_string(), 18));
            for i in 0..3 {
                pins.push((format!("DDRBA{i}"), 19 + i));
            }
            pins.push(("DDRCASB".to_string(), 22));
            pins.push(("DDRCKE".to_string(), 23));
            pins.push(("DDRCKN".to_string(), 24));
            pins.push(("DDRCKP".to_string(), 25));
            pins.push(("PSCLK".to_string(), 26));
            pins.push(("DDRCSB".to_string(), 27));
            for i in 0..4 {
                pins.push((format!("DDRDM{i}"), 28 + i));
            }
            for i in 0..32 {
                pins.push((format!("DDRDQ{i}"), 32 + i));
            }
            for i in 0..4 {
                pins.push((format!("DDRDQSN{i}"), 64 + i));
            }
            for i in 0..4 {
                pins.push((format!("DDRDQSP{i}"), 68 + i));
            }
            pins.push(("DDRDRSTB".to_string(), 72));
            for i in 0..54 {
                pins.push((format!("MIO{i}"), 77 + i));
            }
            pins.push(("DDRODT".to_string(), 131));
            pins.push(("PSPORB".to_string(), 132));
            pins.push(("DDRRASB".to_string(), 133));
            pins.push(("PSSRSTB".to_string(), 134));
            let mut bel = self.builder.bel_xy(bslots::PS, "PS7", 0, 0).raw_tile(2);
            for (pin, _) in &pins {
                bel = bel.pins_name_only(&[pin]);
            }
            for (pin, y) in pins {
                let iopin = &format!("IOPAD_{pin}_IO");
                bel = bel
                    .sub_xy(self.rd, "IOPAD", 0, y - 1)
                    .raw_tile(2)
                    .pin_rename("IO", iopin)
                    .pins_name_only(&[iopin]);
            }
            let mut xn = self
                .builder
                .xtile_id(tcls::PS, "PS", xy_pss0)
                .raw_tile(xy_pss1)
                .raw_tile(xy_pss2)
                .raw_tile(xy_pss3)
                .raw_tile(xy_pss4)
                .num_cells(100);
            for i in 0..4 {
                for j in 0..25 {
                    xn = xn
                        .ref_int(int_xy.delta(0, (i * 26 + j) as i32), i * 25 + j)
                        .ref_single(int_xy.delta(-1, (i * 26 + j) as i32), i * 25 + j, intf);
                }
            }
            xn.bel(bel).extract();
        }
    }

    fn fill_gtp_channel_tiles(&mut self) {
        for (tcid, tkn, int_dx, intf_dx, intf_kind) in [
            (tcls::GTP_CHANNEL, "GTP_CHANNEL_0", -4, -3, "INTF_GTP"),
            (tcls::GTP_CHANNEL, "GTP_CHANNEL_1", -4, -3, "INTF_GTP"),
            (tcls::GTP_CHANNEL, "GTP_CHANNEL_2", -4, -3, "INTF_GTP"),
            (tcls::GTP_CHANNEL, "GTP_CHANNEL_3", -4, -3, "INTF_GTP"),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_0_MID_LEFT",
                -14,
                -13,
                "INTF_GTP_R",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_1_MID_LEFT",
                -14,
                -13,
                "INTF_GTP_R",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_2_MID_LEFT",
                -14,
                -13,
                "INTF_GTP_R",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_3_MID_LEFT",
                -14,
                -13,
                "INTF_GTP_R",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_0_MID_RIGHT",
                19,
                18,
                "INTF_GTP_L",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_1_MID_RIGHT",
                19,
                18,
                "INTF_GTP_L",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_2_MID_RIGHT",
                19,
                18,
                "INTF_GTP_L",
            ),
            (
                tcls::GTP_CHANNEL_MID,
                "GTP_CHANNEL_3_MID_RIGHT",
                19,
                18,
                "INTF_GTP_L",
            ),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let intf = self.builder.ndb.get_tile_class_naming(intf_kind);
                let bels = [
                    self.builder
                        .bel_xy(bslots::GTP_CHANNEL, "GTPE2_CHANNEL", 0, 0)
                        .pins_name_only(&["GTPRXP", "GTPRXN", "GTPTXP", "GTPTXN"])
                        .pin_name_only("PLL0CLK", 1)
                        .pin_name_only("PLL1CLK", 1)
                        .pin_name_only("PLL0REFCLK", 1)
                        .pin_name_only("PLL1REFCLK", 1),
                    self.builder
                        .bel_xy(bslots::IPAD_RXP[0], "IPAD", 0, 1)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_RXN[0], "IPAD", 0, 0)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::OPAD_TXP[0], "OPAD", 0, 1)
                        .pins_name_only(&["I"]),
                    self.builder
                        .bel_xy(bslots::OPAD_TXN[0], "OPAD", 0, 0)
                        .pins_name_only(&["I"]),
                ];
                let mut xn = self.builder.xtile_id(tcid, tkn, xy).num_cells(11);
                for i in 0..11 {
                    xn = xn.ref_int(xy.delta(int_dx, -5 + i as i32), i).ref_single(
                        xy.delta(intf_dx, -5 + i as i32),
                        i,
                        intf,
                    );
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_gtp_common_tiles(&mut self) {
        for (tcid, tkn, int_dx, intf_dx, intf_kind) in [
            (tcls::GTP_COMMON, "GTP_COMMON", -4, -3, "INTF_GTP"),
            (
                tcls::GTP_COMMON_MID,
                "GTP_COMMON_MID_LEFT",
                -14,
                -13,
                "INTF_GTP_R",
            ),
            (
                tcls::GTP_COMMON_MID,
                "GTP_COMMON_MID_RIGHT",
                19,
                18,
                "INTF_GTP_L",
            ),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let intf = self.builder.ndb.get_tile_class_naming(intf_kind);
                let mut bel = self
                    .builder
                    .bel_xy(bslots::GTP_COMMON, "GTPE2_COMMON", 0, 0)
                    .pin_name_only("PLL0OUTCLK", 1)
                    .pin_name_only("PLL1OUTCLK", 1)
                    .pin_name_only("PLL0OUTREFCLK", 1)
                    .pin_name_only("PLL1OUTREFCLK", 1)
                    .pins_name_only(&[
                        "GTREFCLK0",
                        "GTREFCLK1",
                        "GTEASTREFCLK0",
                        "GTEASTREFCLK1",
                        "GTWESTREFCLK0",
                        "GTWESTREFCLK1",
                    ])
                    .extra_wire("REFCLK0", &["GTPE2_COMMON_REFCLK0"])
                    .extra_wire("REFCLK1", &["GTPE2_COMMON_REFCLK1"]);
                for i in 0..4 {
                    bel = bel
                        .extra_wire(
                            format!("RXOUTCLK{i}"),
                            &[format!("GTPE2_COMMON_RXOUTCLK_{i}")],
                        )
                        .extra_wire(
                            format!("TXOUTCLK{i}"),
                            &[format!("GTPE2_COMMON_TXOUTCLK_{i}")],
                        );
                }
                if tkn != "GTP_COMMON_MID_LEFT" {
                    bel = bel.pin_dummy("GTWESTREFCLK0").pin_dummy("GTWESTREFCLK1");
                }
                if tkn != "GTP_COMMON_MID_RIGHT" {
                    bel = bel.pin_dummy("GTEASTREFCLK0").pin_dummy("GTEASTREFCLK1");
                }
                if tkn != "GTP_COMMON" {
                    bel = bel
                        .extra_wire("WESTCLK0", &["HCLK_GTP_REFCK_WESTCLK0"])
                        .extra_wire("WESTCLK1", &["HCLK_GTP_REFCK_WESTCLK1"])
                        .extra_wire("EASTCLK0", &["HCLK_GTP_REFCK_EASTCLK0"])
                        .extra_wire("EASTCLK1", &["HCLK_GTP_REFCK_EASTCLK1"]);
                }

                let mut bels = vec![
                    bel,
                    self.builder
                        .bel_xy(bslots::BUFDS[0], "IBUFDS_GTE2", 0, 0)
                        .pins_name_only(&["I", "IB", "O", "ODIV2"])
                        .extra_int_out("MGTCLKOUT", &["IBUFDS_GTPE2_0_MGTCLKOUT"]),
                    self.builder
                        .bel_xy(bslots::BUFDS[1], "IBUFDS_GTE2", 0, 1)
                        .pins_name_only(&["I", "IB", "O", "ODIV2"])
                        .extra_int_out("MGTCLKOUT", &["IBUFDS_GTPE2_1_MGTCLKOUT"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKP[0], "IPAD", 0, 0)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKN[0], "IPAD", 0, 1)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKP[1], "IPAD", 0, 2)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKN[1], "IPAD", 0, 3)
                        .pins_name_only(&["O"]),
                ];
                if tcid == tcls::GTP_COMMON_MID {
                    bels.push(self.builder.bel_virtual(bslots::HCLK_DRP_GTP_MID));
                }
                let mut xn = self
                    .builder
                    .xtile_id(tcid, tkn, xy)
                    .num_cells(6)
                    .switchbox(bslots::SPEC_INT)
                    .optin_muxes(&wires::HROW_O[..])
                    .optin_muxes(&wires::OUT_GT_MGTCLKOUT_HCLK[..])
                    .optin_muxes(&wires::OUT_GT_RXOUTCLK_HCLK[..])
                    .optin_muxes(&wires::OUT_GT_TXOUTCLK_HCLK[..]);
                for i in 0..3 {
                    xn = xn.ref_int(xy.delta(int_dx, i as i32), i).ref_single(
                        xy.delta(intf_dx, i as i32),
                        i,
                        intf,
                    );
                }
                for i in 0..3 {
                    xn = xn
                        .ref_int(xy.delta(int_dx, 4 + i as i32), i + 3)
                        .ref_single(xy.delta(intf_dx, 4 + i as i32), i + 3, intf);
                }
                xn.bels(bels).extract();
            }
        }
        if let Some(pips) = self
            .builder
            .pips
            .get_mut(&(tcls::GTP_COMMON, bslots::SPEC_INT))
        {
            for mode in pips.pips.values_mut() {
                *mode = PipMode::PermaBuf;
            }
        }
        if let Some(pips) = self
            .builder
            .pips
            .get_mut(&(tcls::GTP_COMMON_MID, bslots::SPEC_INT))
        {
            for ((wt, _wf), mode) in pips.pips.iter_mut() {
                if !wires::HROW_O.contains(wt.wire) {
                    *mode = PipMode::Buf;
                }
            }
            for i in 0..14 {
                pips.pips.insert(
                    (wires::HROW_I_GTP[i].cell(3), wires::HROW_I[i].cell(3).pos()),
                    PipMode::Buf,
                );
            }
        }
    }

    fn fill_gtx_channel_tiles(&mut self) {
        for (tkn, tcid, slot, bslot, intf_l_kind, intf_r_kind) in [
            (
                "GTX_CHANNEL_0",
                tcls::GTX_CHANNEL,
                bslots::GTX_CHANNEL,
                "GTXE2_CHANNEL",
                "INTF_GTX_L",
                "INTF_GTX",
            ),
            (
                "GTX_CHANNEL_1",
                tcls::GTX_CHANNEL,
                bslots::GTX_CHANNEL,
                "GTXE2_CHANNEL",
                "INTF_GTX_L",
                "INTF_GTX",
            ),
            (
                "GTX_CHANNEL_2",
                tcls::GTX_CHANNEL,
                bslots::GTX_CHANNEL,
                "GTXE2_CHANNEL",
                "INTF_GTX_L",
                "INTF_GTX",
            ),
            (
                "GTX_CHANNEL_3",
                tcls::GTX_CHANNEL,
                bslots::GTX_CHANNEL,
                "GTXE2_CHANNEL",
                "INTF_GTX_L",
                "INTF_GTX",
            ),
            (
                "GTH_CHANNEL_0",
                tcls::GTH_CHANNEL,
                bslots::GTH_CHANNEL,
                "GTHE2_CHANNEL",
                "INTF_GTH_L",
                "INTF_GTH",
            ),
            (
                "GTH_CHANNEL_1",
                tcls::GTH_CHANNEL,
                bslots::GTH_CHANNEL,
                "GTHE2_CHANNEL",
                "INTF_GTH_L",
                "INTF_GTH",
            ),
            (
                "GTH_CHANNEL_2",
                tcls::GTH_CHANNEL,
                bslots::GTH_CHANNEL,
                "GTHE2_CHANNEL",
                "INTF_GTH_L",
                "INTF_GTH",
            ),
            (
                "GTH_CHANNEL_3",
                tcls::GTH_CHANNEL,
                bslots::GTH_CHANNEL,
                "GTHE2_CHANNEL",
                "INTF_GTH_L",
                "INTF_GTH",
            ),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_gtx = tcid == tcls::GTX_CHANNEL;
                let is_l = xy.x == 0;
                let intf = self.builder.ndb.get_tile_class_naming(if is_l {
                    intf_l_kind
                } else {
                    intf_r_kind
                });
                let bels = [
                    self.builder
                        .bel_xy(slot, bslot, 0, 0)
                        .pins_name_only(&if is_gtx {
                            ["GTXRXP", "GTXRXN", "GTXTXP", "GTXTXN"]
                        } else {
                            ["GTHRXP", "GTHRXN", "GTHTXP", "GTHTXN"]
                        })
                        .pin_name_only("GTREFCLK0", 1)
                        .pin_name_only("GTREFCLK1", 1)
                        .pin_name_only("GTNORTHREFCLK0", 1)
                        .pin_name_only("GTNORTHREFCLK1", 1)
                        .pin_name_only("GTSOUTHREFCLK0", 1)
                        .pin_name_only("GTSOUTHREFCLK1", 1)
                        .pin_name_only("QPLLCLK", 1)
                        .pin_name_only("QPLLREFCLK", 1),
                    self.builder
                        .bel_xy(bslots::IPAD_RXP[0], "IPAD", 0, 1)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_RXN[0], "IPAD", 0, 0)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::OPAD_TXP[0], "OPAD", 0, 1)
                        .pins_name_only(&["I"]),
                    self.builder
                        .bel_xy(bslots::OPAD_TXN[0], "OPAD", 0, 0)
                        .pins_name_only(&["I"]),
                ];
                let mut xn = self.builder.xtile_id(tcid, tkn, xy).num_cells(11);
                let int_dx = if is_l { 3 } else { -4 };
                let intf_dx = if is_l { 2 } else { -3 };
                for i in 0..11 {
                    xn = xn.ref_int(xy.delta(int_dx, -5 + i as i32), i).ref_single(
                        xy.delta(intf_dx, -5 + i as i32),
                        i,
                        intf,
                    );
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_gtx_common_tiles(&mut self) {
        for (tkn, tcid, slot, bslot, intf_l_kind, intf_r_kind) in [
            (
                "GTX_COMMON",
                tcls::GTX_COMMON,
                bslots::GTX_COMMON,
                "GTXE2_COMMON",
                "INTF_GTX_L",
                "INTF_GTX",
            ),
            (
                "GTH_COMMON",
                tcls::GTH_COMMON,
                bslots::GTH_COMMON,
                "GTHE2_COMMON",
                "INTF_GTH_L",
                "INTF_GTH",
            ),
        ] {
            if let Some(&xy) = self.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_l = xy.x == 0;
                let intf = self.builder.ndb.get_tile_class_naming(if is_l {
                    intf_l_kind
                } else {
                    intf_r_kind
                });
                let mut bel = self
                    .builder
                    .bel_xy(slot, bslot, 0, 0)
                    .pin_name_only("QPLLOUTCLK", 1)
                    .pin_name_only("QPLLOUTREFCLK", 1)
                    .pin_name_only("GTREFCLK0", 1)
                    .pin_name_only("GTREFCLK1", 1)
                    .pin_name_only("GTNORTHREFCLK0", 1)
                    .pin_name_only("GTNORTHREFCLK1", 1)
                    .pin_name_only("GTSOUTHREFCLK0", 1)
                    .pin_name_only("GTSOUTHREFCLK1", 1);
                for i in 0..4 {
                    bel = bel
                        .extra_wire(
                            format!("RXOUTCLK{i}"),
                            &[
                                format!("GTXE2_COMMON_RXOUTCLK_{i}"),
                                format!("GTHE2_COMMON_RXOUTCLK_{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("TXOUTCLK{i}"),
                            &[
                                format!("GTXE2_COMMON_TXOUTCLK_{i}"),
                                format!("GTHE2_COMMON_TXOUTCLK_{i}"),
                            ],
                        );
                }

                let bels = [
                    bel,
                    self.builder
                        .bel_xy(bslots::BUFDS[0], "IBUFDS_GTE2", 0, 0)
                        .pins_name_only(&["I", "IB", "O", "ODIV2"])
                        .extra_int_out(
                            "MGTCLKOUT",
                            &["IBUFDS_GTE2_0_MGTCLKOUT", "IBUFDS_GTHE2_0_MGTCLKOUT"],
                        ),
                    self.builder
                        .bel_xy(bslots::BUFDS[1], "IBUFDS_GTE2", 0, 1)
                        .pins_name_only(&["I", "IB", "O", "ODIV2"])
                        .extra_int_out(
                            "MGTCLKOUT",
                            &["IBUFDS_GTE2_1_MGTCLKOUT", "IBUFDS_GTHE2_1_MGTCLKOUT"],
                        ),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKP[0], "IPAD", 0, 0)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKN[0], "IPAD", 0, 1)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKP[1], "IPAD", 0, 2)
                        .pins_name_only(&["O"]),
                    self.builder
                        .bel_xy(bslots::IPAD_CLKN[1], "IPAD", 0, 3)
                        .pins_name_only(&["O"]),
                ];
                let mut xn = self
                    .builder
                    .xtile_id(tcid, tkn, xy)
                    .num_cells(6)
                    .switchbox(bslots::SPEC_INT)
                    .optin_muxes(&wires::HROW_O[..]);
                let int_dx = if is_l { 3 } else { -4 };
                let intf_dx = if is_l { 2 } else { -3 };
                for i in 0..3 {
                    xn = xn.ref_int(xy.delta(int_dx, i as i32), i).ref_single(
                        xy.delta(intf_dx, i as i32),
                        i,
                        intf,
                    );
                }
                for i in 0..3 {
                    xn = xn
                        .ref_int(xy.delta(int_dx, 4 + i as i32), i + 3)
                        .ref_single(xy.delta(intf_dx, 4 + i as i32), i + 3, intf);
                }
                xn.bels(bels).extract();
                let pips = self
                    .builder
                    .pips
                    .get_mut(&(tcid, bslots::SPEC_INT))
                    .unwrap();
                for mode in pips.pips.values_mut() {
                    *mode = PipMode::PermaBuf;
                }
            }
        }
        if let Some(&xy) = self.rd.tiles_by_kind_name("BRKH_GTX").iter().next() {
            let bel = self
                .builder
                .bel_virtual(bslots::BRKH_GTX)
                .extra_wire("REFCLK0_D", &["BRKH_GTX_REFCLK0_LOWER"])
                .extra_wire("REFCLK1_D", &["BRKH_GTX_REFCLK1_LOWER"])
                .extra_wire("REFCLK0_U", &["BRKH_GTX_REFCLK0_UPPER"])
                .extra_wire("REFCLK1_U", &["BRKH_GTX_REFCLK1_UPPER"])
                .extra_wire("NORTHREFCLK0_D", &["BRKH_GTX_NORTHREFCLK0_LOWER"])
                .extra_wire("NORTHREFCLK1_D", &["BRKH_GTX_NORTHREFCLK1_LOWER"])
                .extra_wire("NORTHREFCLK0_U", &["BRKH_GTX_NORTHREFCLK0_UPPER"])
                .extra_wire("NORTHREFCLK1_U", &["BRKH_GTX_NORTHREFCLK1_UPPER"])
                .extra_wire("SOUTHREFCLK0_D", &["BRKH_GTX_SOUTHREFCLK0_LOWER"])
                .extra_wire("SOUTHREFCLK1_D", &["BRKH_GTX_SOUTHREFCLK1_LOWER"])
                .extra_wire("SOUTHREFCLK0_U", &["BRKH_GTX_SOUTHREFCLK0_UPPER"])
                .extra_wire("SOUTHREFCLK1_U", &["BRKH_GTX_SOUTHREFCLK1_UPPER"]);
            self.builder
                .xtile_id(tcls::BRKH_GTX, "BRKH_GTX", xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }
}

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut maker = IntMaker {
        rd,
        builder: IntBuilder::new(
            rd,
            bincode::decode_from_slice(defs::virtex7::INIT, bincode::config::standard())
                .unwrap()
                .0,
        ),
    };
    maker.fill_int_wires();
    maker.fill_hclk_wires();
    maker.fill_gclk_wires();
    maker.fill_io_wires();
    maker.fill_cmt_wires();
    maker.fill_gt_wires();
    maker.fill_ps_wires();
    maker.fill_int_tiles();
    maker.fill_clb_tiles();
    maker.fill_bram_tiles();
    maker.fill_dsp_tiles();
    maker.fill_pcie_tiles();
    maker.fill_pcie3_tiles();
    maker.fill_hclk_tiles();
    maker.fill_clk_rebuf_tiles();
    maker.fill_clk_hrow_tiles();
    maker.fill_clk_bufg_tiles();
    maker.fill_clk_misc_tiles();
    maker.fill_hclk_io_tiles();
    maker.fill_io_tiles();
    maker.fill_cmt_tiles();
    maker.fill_cmt_fifo_tiles();
    maker.fill_cfg_tiles();
    maker.fill_sysmon_tiles();
    maker.fill_ps_tiles();
    maker.fill_gtp_channel_tiles();
    maker.fill_gtp_common_tiles();
    maker.fill_gtx_channel_tiles();
    maker.fill_gtx_common_tiles();

    maker.builder.build()
}
