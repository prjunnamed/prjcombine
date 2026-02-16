use std::collections::BTreeSet;

use prjcombine_interconnect::{
    db::{
        BelInfo, BelInput, BelPin, IntDb, LegacyBel, SwitchBoxItem, TileWireCoord, WireSlotIdExt,
        WireSupport,
    },
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_rawdump::Part;

use prjcombine_re_xilinx_naming::db::{BelNaming, NamingDb, TileClassNaming, WireNaming};
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};
use prjcombine_virtex4::defs::{
    self, bcls, bslots,
    virtex4::{ccls, tcls, wires},
};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::virtex4::INIT, bincode::config::standard())
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

    for i in 0..8 {
        builder.wire_names(wires::HCLK[i], &[format!("GCLK{i}")]);
    }
    for i in 0..2 {
        builder.wire_names(wires::RCLK[i], &[format!("RCLK{i}")]);
    }

    for (i, da1, da2, db) in [
        (0, Dir::S, None, None),
        (1, Dir::W, Some(Dir::S), None),
        (2, Dir::E, None, Some(Dir::S)),
        (3, Dir::S, Some(Dir::E), None),
        (4, Dir::S, None, None),
        (5, Dir::S, Some(Dir::W), None),
        (6, Dir::W, None, None),
        (7, Dir::E, Some(Dir::S), None),
        (8, Dir::E, Some(Dir::N), None),
        (9, Dir::W, None, None),
        (10, Dir::N, Some(Dir::W), None),
        (11, Dir::N, None, None),
        (12, Dir::N, Some(Dir::E), None),
        (13, Dir::E, None, Some(Dir::N)),
        (14, Dir::W, Some(Dir::N), None),
        (15, Dir::N, None, None),
    ] {
        builder.wire_names(wires::OMUX[i], &[format!("OMUX{i}")]);
        let omux_da1 = builder.db.get_wire(&format!("OMUX_{da1}{i}"));
        builder.wire_names(omux_da1, &[format!("OMUX_{da1}{i}")]);
        if let Some(da2) = da2 {
            let omux_da2 = builder.db.get_wire(&format!("OMUX_{da1}{da2}{i}"));
            builder.wire_names(omux_da2, &[format!("OMUX_{da1}{da2}{i}")]);
        }
        if let Some(db) = db {
            let omux_db = builder.db.get_wire(&format!("OMUX_{db}{i}"));
            builder.wire_names(omux_db, &[format!("OMUX_{db}{i}")]);
        }
    }
    builder.wire_names(wires::OMUX_S0_ALT, &["OUT_S"]);

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.db.get_wire(&format!("DBL_{dir}0[{i}]"));
            let mid = builder.db.get_wire(&format!("DBL_{dir}1[{i}]"));
            let end = builder.db.get_wire(&format!("DBL_{dir}2[{i}]"));
            builder.wire_names(beg, &[format!("{dir}2BEG{i}")]);
            builder.wire_names(mid, &[format!("{dir}2MID{i}")]);
            builder.wire_names(end, &[format!("{dir}2END{i}")]);
            let (end2, e2d) = match dir {
                Dir::W => (wires::DBL_W2_N[i], Dir::N),
                Dir::E => (wires::DBL_E2_S[i], Dir::S),
                Dir::S => (wires::DBL_S3[i], Dir::S),
                Dir::N => (wires::DBL_N3[i], Dir::N),
            };
            builder.wire_names(end2, &[format!("{dir}2END_{e2d}{i}")]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            for (j, seg) in [
                (0, "BEG"),
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                let w = builder.db.get_wire(&format!("HEX_{dir}{j}[{i}]"));
                builder.wire_names(w, &[format!("{dir}6{seg}{i}")]);
            }
            let (end2, e2d) = match dir {
                Dir::W => (wires::HEX_W6_N[i], Dir::N),
                Dir::E => (wires::HEX_E6_S[i], Dir::S),
                Dir::S => (wires::HEX_S7[i], Dir::S),
                Dir::N => (wires::HEX_N7[i], Dir::N),
            };
            builder.wire_names(end2, &[format!("{dir}6END_{e2d}{i}")]);
        }
    }

    // The long wires.
    for i in 0..25 {
        builder.wire_names(wires::LH[i], &[format!("LH{i}")]);
        builder.wire_names(wires::LV[i], &[format!("LV{i}")]);
    }

    // The control inputs.
    for i in 0..4 {
        builder.wire_names(wires::IMUX_SR[i], &[format!("SR_B{i}")]);
        builder.mark_optinv(wires::IMUX_SR[i], wires::IMUX_SR_OPTINV[i]);
    }
    for i in 0..4 {
        builder.wire_names(wires::IMUX_BOUNCE[i], &[format!("BOUNCE{i}")]);
    }
    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_CLK[i],
            &[format!("CLK_B{i}"), format!("CLK_B{i}_DCM0")],
        );
        builder.mark_optinv(wires::IMUX_CLK[i], wires::IMUX_CLK_OPTINV[i]);
    }
    for i in 0..4 {
        builder.wire_names(wires::IMUX_CE[i], &[format!("CE_B{i}")]);
        builder.mark_optinv(wires::IMUX_CE[i], wires::IMUX_CE_OPTINV[i]);
    }

    // The data inputs.
    for i in 0..8 {
        builder.wire_names(wires::IMUX_BYP[i], &[format!("BYP_INT_B{i}")]);
        builder.wire_names(wires::IMUX_BYP_BOUNCE[i], &[format!("BYP_BOUNCE{i}")]);
        builder.mark_permabuf(wires::IMUX_BYP_BOUNCE[i]);
    }

    for i in 0..32 {
        builder.wire_names(wires::IMUX_IMUX[i], &[format!("IMUX_B{i}")]);
    }

    for i in 0..8 {
        builder.wire_names(wires::OUT_BEST[i], &[format!("BEST_LOGIC_OUTS{i}")]);
        builder.mark_test_mux_in(wires::OUT_BEST_TMIN[i], wires::OUT_BEST[i]);
    }
    for i in 0..8 {
        builder.wire_names(wires::OUT_SEC[i], &[format!("SECONDARY_LOGIC_OUTS{i}")]);
        builder.mark_test_mux_in(wires::OUT_SEC_TMIN[i], wires::OUT_SEC[i]);
    }
    for i in 0..8 {
        builder.wire_names(wires::OUT_HALF0[i], &[format!("HALF_OMUX_BOT{i}")]);
        builder.mark_test_mux_in(wires::OUT_HALF0_BEL[i], wires::OUT_HALF0[i]);
        builder.mark_test_mux_in_test(wires::OUT_HALF0_TEST[i], wires::OUT_HALF0[i]);
    }
    for i in 0..8 {
        builder.wire_names(wires::OUT_HALF1[i], &[format!("HALF_OMUX_TOP{i}")]);
        builder.mark_test_mux_in(wires::OUT_HALF1_BEL[i], wires::OUT_HALF1[i]);
        builder.mark_test_mux_in_test(wires::OUT_HALF1_TEST[i], wires::OUT_HALF1[i]);
    }

    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_SPEC[i],
            &[match i {
                0 => "IOIS_OCLKP_1",
                1 => "IOIS_ICLKP_1",
                2 => "IOIS_OCLKP_0",
                3 => "IOIS_ICLKP_0",
                _ => unreachable!(),
            }],
        );
        match i {
            0 => {
                builder.extra_name_sub("MONITOR_CONVST_TEST", 4, wires::IMUX_SPEC[i]);

                builder.extra_name_sub("DCM_ADV_CLKFB_TEST", 2, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("DCM_ADV_CLKIN_TEST", 3, wires::IMUX_SPEC[i]);

                builder.extra_name_sub("DPM_REFCLK_TEST", 0, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_0_CLKB_TEST", 1, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("DPM_TESTCLK1_TEST", 2, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_0_CLKD_TEST", 3, wires::IMUX_SPEC[i]);
            }
            1 => {
                builder.extra_name_sub("PMCD_0_CLKA_TEST", 1, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("DPM_TESTCLK2_TEST", 2, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_0_CLKC_TEST", 3, wires::IMUX_SPEC[i]);
            }
            2 => {
                builder.extra_name_sub("PMCD_1_REL_TEST", 0, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_1_CLKB_TEST", 1, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_1_CLKD_TEST", 3, wires::IMUX_SPEC[i]);
            }
            3 => {
                builder.extra_name_sub("PMCD_0_REL_TEST", 0, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_1_CLKA_TEST", 1, wires::IMUX_SPEC[i]);
                builder.extra_name_sub("PMCD_1_CLKC_TEST", 3, wires::IMUX_SPEC[i]);
            }
            _ => unreachable!(),
        }
        for j in 0..16 {
            builder.extra_name_sub(
                format!("LOGIC_CREATED_INPUT_B{i}_INT{j}"),
                j,
                wires::IMUX_SPEC[i],
            );
        }
    }
    builder.extra_name("PMCD_0_REL", wires::IMUX_CCM_REL[0]);
    builder.extra_name("PMCD_1_REL", wires::IMUX_CCM_REL[1]);

    for i in 0..8 {
        builder.wire_names(wires::HCLK_ROW[i], &[format!("HCLK_G_HCLKP{i}")]);
        for n in [
            format!("HCLK_IOIS_G_HCLKP{i}"),
            format!("HCLK_DCM_G_HCLKP{i}"),
        ] {
            builder.extra_name_sub(n, 2, wires::HCLK_ROW[i]);
        }
        builder.extra_name_sub(format!("CLK_HROW_HCLK_LP{i}"), 0, wires::HCLK_ROW[i]);
        builder.extra_name_sub(format!("CLK_HROW_HCLK_RP{i}"), 1, wires::HCLK_ROW[i]);
    }
    for i in 0..2 {
        builder.wire_names(wires::RCLK_ROW[i], &[format!("HCLK_RCLK{i}")]);
        for n in [format!("HCLK_IOIS_RCLK{i}"), format!("HCLK_DCM_RCLK{i}")] {
            builder.extra_name_sub(n, 2, wires::RCLK_ROW[i]);
        }
    }

    for n in [
        "HCLK_IOIS_REFCLK",
        "HCLK_IOIS_REFCLK_DCMIOB",
        "HCLK_IOIS_REFCLK_IOBDCM",
    ] {
        builder.extra_name_sub(n, 2, wires::IMUX_IDELAYCTRL_REFCLK);
    }

    builder.extra_name_sub("HCLK_IOIS_BUFIO_OUT0", 2, wires::OUT_CLKPAD);
    builder.extra_name_sub("HCLK_IOIS_BUFIO_OUT1", 1, wires::OUT_CLKPAD);
    for i in 0..16 {
        builder.extra_name_sub(format!("CLK_IOB_IOB_BUFCLKP{i}"), i, wires::OUT_CLKPAD);
        builder.extra_name(format!("CLK_IOB_IOB_CLKP{i}"), wires::GIOB[i]);
        builder.extra_name_sub(format!("HCLK_DCM_IOB_CLKP{i}"), 2, wires::GIOB[i]);
    }

    for i in 0..32 {
        builder.extra_name_sub(format!("CLK_BUFGCTRL_GCLKP{i}"), 8, wires::GCLK[i]);
        builder.mark_permabuf(wires::GCLK[i]);
        builder.extra_name(format!("CLK_HROW_GCLK_BUFP{i}"), wires::GCLK[i]);
    }
    for i in 0..32 {
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_MUXED_CLK{i}"),
            0,
            wires::IMUX_BUFG_I[i],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_MUXED_CLK{i}"),
            8,
            wires::IMUX_BUFG_I[i],
        );
        builder.extra_name(format!("CLK_IOB_MUXED_CLKP{i}"), wires::IMUX_BUFG_O[i]);
        builder.extra_name(format!("CLK_IOB_MUXED_CLKP_IN{i}"), wires::IMUX_BUFG_I[i]);
        builder.extra_name(format!("CLKV_DCM_MUXED_CLKP_OUT{i}"), wires::IMUX_BUFG_O[i]);
        builder.extra_name(format!("CLK_IOB_MUXED_CLKP_IN{i}"), wires::IMUX_BUFG_I[i]);
    }
    for i in 0..16 {
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_GFB_P{i}"),
            8,
            wires::OUT_BUFG[i],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_POSTMUX_GCLKP{i}"),
            8,
            wires::OUT_BUFG[i],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_GFB_P{i}"),
            8,
            wires::OUT_BUFG[i + 16],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_POSTMUX_GCLKP{i}"),
            8,
            wires::OUT_BUFG[i + 16],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_I0P{i}"),
            i / 2,
            wires::IMUX_SPEC[1 + 2 * (i % 2)],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_I1P{i}"),
            i / 2,
            wires::IMUX_SPEC[2 * (i % 2)],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_I0P{i}"),
            8 + (15 - i) / 2,
            wires::IMUX_SPEC[1 + 2 * ((15 - i) % 2)],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_I1P{i}"),
            8 + (15 - i) / 2,
            wires::IMUX_SPEC[2 * ((15 - i) % 2)],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_CKINT0{i}"),
            1 + i / 4,
            wires::IMUX_IMUX[[3, 7, 19, 23][i % 4]],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_CKINT1{i}"),
            1 + i / 4,
            wires::IMUX_IMUX[[11, 15, 27, 31][i % 4]],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_CKINT0{i}"),
            11 + i / 4,
            wires::IMUX_IMUX[[3, 7, 19, 23][i % 4]],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_CKINT1{i}"),
            11 + i / 4,
            wires::IMUX_IMUX[[11, 15, 27, 31][i % 4]],
        );
    }

    for i in 0..2 {
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_MGT_L{i}"),
            0,
            wires::MGT_ROW[i],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_B",
            format!("CLK_BUFGCTRL_MGT_R{i}"),
            16,
            wires::MGT_ROW[i],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_MGT_L{i}"),
            8,
            wires::MGT_ROW[i],
        );
        builder.extra_name_tile_sub(
            "CLK_BUFGCTRL_T",
            format!("CLK_BUFGCTRL_MGT_R{i}"),
            17,
            wires::MGT_ROW[i],
        );
        builder.extra_name_sub(format!("HCLK_MGT_CLKL{i}"), 2, wires::MGT_ROW[i]);
        builder.extra_name_sub(format!("HCLK_MGT_CLKR{i}"), 4, wires::MGT_ROW[i]);
        builder.alt_name_sub(format!("HCLK_DCM_MGT{i}"), 2, wires::MGT_ROW[i]);
        builder.alt_name_sub(
            format!("HCLK_DCM_MGT{ii}", ii = i + 2),
            4,
            wires::MGT_ROW[i],
        );
    }

    for i in 0..8 {
        builder.wire_names(wires::HCLK_IO[i], &[format!("IOIS_GCLKP{i}")]);
        for n in [
            format!("HCLK_IOIS_LEAF_GCLK_P{i}"),
            format!("HCLK_DCM_LEAF_GCLK_P{i}"),
        ] {
            builder.extra_name_sub(n, 2, wires::HCLK_IO[i]);
        }
    }
    for i in 0..2 {
        builder.wire_names(wires::RCLK_IO[i], &[format!("IOIS_RCLK_FORIO_P{i}")]);
        builder.wire_names(wires::IOCLK[i], &[format!("IOIS_IOCLKP{i}")]);
        builder.wire_names(wires::IOCLK_S_IO[i], &[format!("IOIS_IOCLKP_S{i}")]);
        builder.wire_names(wires::IOCLK_N_IO[i], &[format!("IOIS_IOCLKP_N{i}")]);
        for n in [
            format!("HCLK_IOIS_RCLK_FORIO_P{i}"),
            format!("HCLK_DCM_RCLK_FORIO_P{i}"),
        ] {
            builder.extra_name_sub(n, 2, wires::RCLK_IO[i]);
        }
        builder.extra_name_sub(format!("HCLK_IOIS_VRCLK{i}"), 2, wires::VRCLK[i]);
        builder.extra_name_sub(format!("HCLK_IOIS_VRCLK_S{i}"), 2, wires::VRCLK_S[i]);
        builder.extra_name_sub(format!("HCLK_IOIS_VRCLK_N{i}"), 2, wires::VRCLK_N[i]);

        for (wires, name) in [
            (wires::IOCLK, "VIOCLKP"),
            (wires::IOCLK_S, "VIOCLKP_S"),
            (wires::IOCLK_N, "VIOCLKP_N"),
            (wires::IOCLK_S_IO, "IOCLKP_S"),
            (wires::IOCLK_N_IO, "IOCLKP_N"),
        ] {
            for pref in ["HCLK_IOIS", "HCLK_DCMIOB", "HCLK_IOBDCM"] {
                builder.extra_name_sub(format!("{pref}_{name}{i}"), 2, wires[i]);
            }
        }

        builder.extra_name_sub(format!("HCLK_IOIS_BUFR_I{i}"), 2, wires::IMUX_BUFR[i]);
    }
    builder.extra_name_sub("HCLK_IOIS_INT_RCLKMUX_N", 1, wires::IMUX_BYP[4]);
    builder.extra_name_sub("HCLK_IOIS_INT_RCLKMUX_S", 2, wires::IMUX_BYP[4]);

    for tkn in [
        "MGT_AL",
        "MGT_AL_BOT",
        "MGT_AL_MID",
        "MGT_AR",
        "MGT_AR_BOT",
        "MGT_AR_MID",
    ] {
        for i in 0..8 {
            builder.extra_name_tile_sub(tkn, format!("MGT_G_HCLKP{i}"), 24, wires::HCLK_MGT[i]);
        }
        for i in 0..2 {
            builder.extra_name_tile_sub(tkn, format!("MGT_MGT{i}"), 24, wires::MGT_CLK_OUT[i]);
        }
        builder.extra_name_tile_sub(tkn, "MGT_SYNCLK_OUT", 24, wires::MGT_CLK_OUT_SYNCLK);
        builder.extra_name_tile_sub(tkn, "MGT_REFCLK", 16, wires::IMUX_MGT_REFCLK_PRE[1]);
        builder.extra_name_tile_sub(tkn, "MGT_PMACLK_OUT", 16, wires::IMUX_MGT_GREFCLK_PRE[1]);
    }
    for tkn in ["MGT_BL", "MGT_BR"] {
        for i in 0..8 {
            builder.extra_name_tile_sub(tkn, format!("MGT_G_HCLKP{i}"), 8, wires::HCLK_MGT[i]);
        }
        for i in 0..2 {
            builder.extra_name_tile_sub(tkn, format!("MGT_MGT{i}"), 8, wires::MGT_CLK_OUT[i]);
        }
        builder.extra_name_tile_sub(tkn, "MGT_SYNCLK_OUT", 8, wires::MGT_CLK_OUT_SYNCLK);
        builder.extra_name_tile_sub(tkn, "MGT_REFCLK", 16, wires::IMUX_MGT_REFCLK_PRE[0]);
        builder.extra_name_tile_sub(tkn, "MGT_PMACLK_OUT", 16, wires::IMUX_MGT_GREFCLK_PRE[0]);
    }
    builder.extra_name_sub("GT11CLK_PMACLK_L", 16, wires::IMUX_MGT_GREFCLK);
    builder.extra_name_sub("GT11CLK_PMACLK_R", 16, wires::IMUX_MGT_GREFCLK);
    builder.extra_name_sub("GT11CLK_REFCLK_L", 16, wires::IMUX_MGT_REFCLK);
    builder.extra_name_sub("GT11CLK_REFCLK_R", 16, wires::IMUX_MGT_REFCLK);

    for i in 0..24 {
        builder.extra_name_sub(
            format!("CLKV_DCM_DCM_OUTCLKP{i}"),
            if i < 12 { 0 } else { 4 },
            wires::OUT_DCM[i % 12],
        );
    }

    for i in 0..8 {
        builder.wire_names(
            wires::HCLK_DCM[i],
            &[format!("DCM_BUFG{i}"), format!("CCM_BUFG{i}")],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCMIOB",
            format!("HCLK_DCM_LEAF_DIRECT_HCLKP{i}"),
            1,
            wires::HCLK_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_IOBDCM",
            format!("HCLK_DCM_LEAF_DIRECT_HCLKP{i}"),
            2,
            wires::HCLK_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCM",
            format!("HCLK_DCM_LEAF_DIRECT_HCLKP{i}"),
            1,
            wires::HCLK_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCM",
            format!("HCLK_DCM_LEAF_DIRECT_UP_HCLKP{i}"),
            2,
            wires::HCLK_DCM[i],
        );
    }
    for i in 0..16 {
        builder.wire_names(
            wires::GIOB_DCM[i],
            &[
                format!("DCM_GIOB{i}"),
                format!("CCM_GIOB{i}"),
                format!("SYS_MON_GIOB{i}"),
            ],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCMIOB",
            format!("HCLK_DCM_IOB_CLKP_OUT{i}"),
            1,
            wires::GIOB_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_IOBDCM",
            format!("HCLK_DCM_IOB_CLKP_OUT{i}"),
            2,
            wires::GIOB_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCM",
            format!("HCLK_DCM_IOB_CLKP_DOWN_OUT{i}"),
            1,
            wires::GIOB_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCM",
            format!("HCLK_DCM_IOB_CLKP_UP_OUT{i}"),
            2,
            wires::GIOB_DCM[i],
        );
    }
    for i in 0..4 {
        builder.wire_names(
            wires::MGT_DCM[i],
            &[format!("DCM_MGT{i}"), format!("CCM_MGT{i}")],
        );
        builder.extra_name_tile_sub("HCLK_DCMIOB", format!("HCLK_MGT{i}"), 1, wires::MGT_DCM[i]);
        builder.extra_name_tile_sub("HCLK_IOBDCM", format!("HCLK_MGT{i}"), 2, wires::MGT_DCM[i]);
        builder.extra_name_tile_sub(
            "HCLK_DCM",
            format!("HCLK_DCM_DN_MGT{i}"),
            1,
            wires::MGT_DCM[i],
        );
        builder.extra_name_tile_sub(
            "HCLK_DCM",
            format!("HCLK_DCM_UP_MGT{i}"),
            2,
            wires::MGT_DCM[i],
        );
    }
    for i in 0..24 {
        builder.wire_names(
            wires::DCM_DCM_I[i],
            &[
                format!("DCM_IN{i}"),
                format!("DCM_BOT_IN{i}"),
                format!("CCM_DCM{i}"),
            ],
        );
        builder.wire_names(
            wires::DCM_DCM_O[i],
            &[format!("DCM_OUT{i}"), format!("DCM_BOT_OUT{i}")],
        );
    }

    builder.extra_name_sub("GT11CLK_RXPCSHCLKOUTB", 16, wires::OUT_MGT_RXPCSHCLKOUT[0]);
    builder.extra_name_sub("GT11CLK_RXPCSHCLKOUTA", 16, wires::OUT_MGT_RXPCSHCLKOUT[1]);
    builder.extra_name_sub("GT11CLK_TXPCSHCLKOUTB", 16, wires::OUT_MGT_TXPCSHCLKOUT[0]);
    builder.extra_name_sub("GT11CLK_TXPCSHCLKOUTA", 16, wires::OUT_MGT_TXPCSHCLKOUT[1]);

    for n in ["GT11CLK_FWDCLK0A_L", "GT11CLK_FWDCLK0A_R"] {
        builder.extra_name_sub(n, 24, wires::MGT_CLK_OUT_FWDCLK[0])
    }
    for n in ["GT11CLK_FWDCLK1A_L", "GT11CLK_FWDCLK1A_R"] {
        builder.extra_name_sub(n, 24, wires::MGT_CLK_OUT_FWDCLK[1])
    }
    for n in ["GT11CLK_FWDCLK0B_L", "GT11CLK_FWDCLK0B_R"] {
        builder.extra_name_sub(n, 8, wires::MGT_CLK_OUT_FWDCLK[0])
    }
    for n in ["GT11CLK_FWDCLK1B_L", "GT11CLK_FWDCLK1B_R"] {
        builder.extra_name_sub(n, 8, wires::MGT_CLK_OUT_FWDCLK[1])
    }
    for n in ["GT11CLK_SYNCLK1OUT_L", "GT11CLK_SYNCLK1OUT_R"] {
        builder.extra_name_sub(n, 16, wires::OUT_MGT_SYNCLK[0]);
    }
    for n in ["GT11CLK_SYNCLK2OUT_L", "GT11CLK_SYNCLK2OUT_R"] {
        builder.extra_name_sub(n, 16, wires::OUT_MGT_SYNCLK[1]);
    }
    for i in 0..4 {
        let ii = i + 1;
        builder.extra_name_sub(format!("GT11CLK_SFWDCLK{ii}"), 16, wires::MGT_FWDCLK_S[i]);
        builder.extra_name_sub(format!("GT11CLK_NFWDCLK{ii}"), 16, wires::MGT_FWDCLK_N[i]);
    }

    builder.int_type_id(tcls::INT, bslots::INT, "INT", "INT");
    builder.int_type_id(tcls::INT, bslots::INT, "INT_SO", "INT");
    builder.int_type_id(tcls::INT, bslots::INT, "INT_SO_DCM0", "INT_DCM0");

    builder.extract_term_id(ccls::TERM_W, None, Dir::W, "L_TERM_INT", "TERM_W");
    builder.extract_term_id(ccls::TERM_E, None, Dir::E, "R_TERM_INT", "TERM_E");
    builder.extract_term_id(ccls::TERM_S, None, Dir::S, "B_TERM_INT", "TERM_S");
    builder.extract_term_id(ccls::TERM_N, None, Dir::N, "T_TERM_INT", "TERM_N");
    for tkn in ["MGT_AL_BOT", "MGT_AL_MID", "MGT_AL", "MGT_BL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for (i, delta) in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16]
                .into_iter()
                .enumerate()
            {
                let int_xy = xy.delta(1, -9 + delta);
                builder.extract_term_tile_id(
                    ccls::TERM_W,
                    None,
                    Dir::W,
                    xy,
                    format!("TERM_W_MGT{i}"),
                    int_xy,
                );
            }
        }
    }
    for tkn in ["MGT_AR_BOT", "MGT_AR_MID", "MGT_AR", "MGT_BR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for (i, delta) in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16]
                .into_iter()
                .enumerate()
            {
                let int_xy = xy.delta(-1, -9 + delta);
                builder.extract_term_tile_id(
                    ccls::TERM_E,
                    None,
                    Dir::E,
                    xy,
                    format!("TERM_E_MGT{i}"),
                    int_xy,
                );
            }
        }
    }

    builder.extract_pass_simple_id(ccls::BRKH_S, ccls::BRKH_N, Dir::S, "BRKH", &[]);
    builder.extract_pass_buf_id(
        ccls::CLB_BUFFER_W,
        ccls::CLB_BUFFER_E,
        Dir::W,
        "CLB_BUFFER",
        "CLB_BUFFER_W",
        "CLB_BUFFER_E",
        &[],
    );

    builder.stub_out("PB_OMUX11_B5");
    builder.stub_out("PB_OMUX11_B6");

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = pb_xy.delta(0, 18);
        for (i, delta) in [
            0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = pb_xy.delta(-1, -3 + delta);
            let int_e_xy = pb_xy.delta(15, -3 + delta);
            let naming_w = format!("TERM_PPC_W{i}");
            let naming_e = format!("TERM_PPC_E{i}");
            let xy = if i < 11 { pb_xy } else { pt_xy };
            builder.extract_pass_tile_id(
                ccls::PPC_W,
                Dir::W,
                int_e_xy,
                Some(xy),
                Some(xy),
                Some(&naming_w),
                None,
                None,
                int_w_xy,
                &[],
            );
            builder.extract_pass_tile_id(
                ccls::PPC_E,
                Dir::E,
                int_w_xy,
                Some(xy),
                Some(xy),
                Some(&naming_e),
                None,
                None,
                int_e_xy,
                &[],
            );
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = pb_xy.delta(delta, -4);
            let int_n_xy = pb_xy.delta(delta, 22);
            let naming_s = format!("TERM_PPC_S{i}");
            let naming_n = format!("TERM_PPC_N{i}");
            builder.extract_pass_tile_id(
                if i < 5 { ccls::PPC_A_S } else { ccls::PPC_B_S },
                Dir::S,
                int_n_xy,
                Some(pt_xy),
                Some(pb_xy),
                Some(&naming_s),
                None,
                None,
                int_s_xy,
                &[],
            );
            builder.extract_pass_tile_id(
                if i < 5 { ccls::PPC_A_N } else { ccls::PPC_B_N },
                Dir::N,
                int_s_xy,
                Some(pb_xy),
                Some(pt_xy),
                Some(&naming_n),
                None,
                None,
                int_n_xy,
                &[],
            );
        }
    }

    for (tkn, n, height) in [
        ("BRAM", "BRAM", 4),
        ("DSP", "DSP", 4),
        ("CCM", "CCM", 4),
        ("DCM", "DCM", 4),
        ("DCM_BOT", "DCM", 4),
        ("SYS_MON", "SYSMON", 8),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for i in 0..height {
                let int_xy = xy.delta(-1, i);
                builder.extract_intf_tile_id(
                    tcls::INTF,
                    xy,
                    int_xy,
                    format!("INTF_{n}_{i}"),
                    bslots::INTF_TESTMUX,
                    Some(bslots::INTF_INT),
                    false,
                    false,
                );
            }
        }
    }
    for tkn in ["IOIS_LC", "IOIS_NC"] {
        builder.extract_intf_id(
            tcls::INTF,
            Dir::E,
            tkn,
            "INTF_IOIS",
            bslots::INTF_TESTMUX,
            Some(bslots::INTF_INT),
            false,
            false,
        );
    }
    for &xy in rd.tiles_by_kind_name("CFG_CENTER") {
        for i in 0..16 {
            let int_xy = xy.delta(-1, if i < 8 { -8 + i } else { -8 + i + 1 });
            builder.extract_intf_tile_id(
                tcls::INTF,
                xy,
                int_xy,
                format!("INTF_CFG_{i}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                false,
                false,
            );
        }
    }
    for (dir, tkn) in [
        (Dir::W, "MGT_AL"),
        (Dir::W, "MGT_AL_BOT"),
        (Dir::W, "MGT_AL_MID"),
        (Dir::W, "MGT_BL"),
        (Dir::E, "MGT_AR"),
        (Dir::E, "MGT_AR_BOT"),
        (Dir::E, "MGT_AR_MID"),
        (Dir::E, "MGT_BR"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for i in 0..16 {
                let int_xy = xy.delta(
                    if dir == Dir::E { -1 } else { 1 },
                    if i < 8 { -9 + i } else { i - 8 },
                );
                builder.extract_intf_tile_id(
                    tcls::INTF,
                    xy,
                    int_xy,
                    format!("INTF_MGT_{i}"),
                    bslots::INTF_TESTMUX,
                    Some(bslots::INTF_INT),
                    false,
                    false,
                );
            }
        }
    }

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = pb_xy.delta(0, 18);
        for (i, delta) in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = pb_xy.delta(-1, -4 + delta);
            let int_e_xy = pb_xy.delta(15, -4 + delta);
            let xy = if i < 12 { pb_xy } else { pt_xy };
            builder.extract_intf_tile_id(
                tcls::INTF,
                xy,
                int_w_xy,
                format!("INTF_PPC_W{i}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                false,
                false,
            );
            builder.extract_intf_tile_id(
                tcls::INTF,
                xy,
                int_e_xy,
                format!("INTF_PPC_E{i}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                false,
                false,
            );
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = pb_xy.delta(delta, -4);
            let int_n_xy = pb_xy.delta(delta, 22);
            builder.extract_intf_tile_id(
                tcls::INTF,
                pb_xy,
                int_s_xy,
                format!("INTF_PPC_S{i}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                false,
                false,
            );
            builder.extract_intf_tile_id(
                tcls::INTF,
                pt_xy,
                int_n_xy,
                format!("INTF_PPC_N{i}"),
                bslots::INTF_TESTMUX,
                Some(bslots::INTF_INT),
                false,
                false,
            );
        }
    }

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    if let Some(&xy) = rd.tiles_by_kind_name("CLB").iter().next() {
        let int_xy = xy.delta(-1, 0);
        let bels = [
            builder
                .bel_xy(bslots::SLICE[0], "SLICE", 0, 0)
                .pins_name_only(&slicem_name_only),
            builder
                .bel_xy(bslots::SLICE[1], "SLICE", 1, 0)
                .pins_name_only(&slicel_name_only),
            builder
                .bel_xy(bslots::SLICE[2], "SLICE", 0, 1)
                .pins_name_only(&slicem_name_only)
                .extra_wire("COUT_N", &["COUT_N1"])
                .extra_wire("FX_S", &["FX_S2"]),
            builder
                .bel_xy(bslots::SLICE[3], "SLICE", 1, 1)
                .pins_name_only(&slicel_name_only)
                .extra_wire("COUT_N", &["COUT_N3"]),
        ];
        builder
            .xtile_id(tcls::CLB, "CLB", xy)
            .num_cells(1)
            .bels(bels)
            .ref_int(int_xy, 0)
            .extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let bel = builder
            .bel_xy(bslots::BRAM, "RAMB16", 0, 0)
            .pins_name_only(&["CASCADEOUTA", "CASCADEOUTB"])
            .pin_name_only("CASCADEINA", 1)
            .pin_name_only("CASCADEINB", 1)
            .sub_xy(rd, "FIFO16", 0, 0)
            .manual();
        let mut xt = builder
            .xtile_id(tcls::BRAM, "BRAM", xy)
            .num_cells(4)
            .bel(bel);
        for i in 0..4 {
            xt = xt.ref_int(xy.delta(-1, i as i32), i);
        }
        let (mut bel, naming) = xt.extract().bels.pop().unwrap();
        for i in 0..32 {
            let di = bel.pins.remove(&format!("DI{i}")).unwrap();
            assert_eq!(bel.pins[&format!("DIB{i}")], di);
            let do_ = bel.pins.remove(&format!("DO{i}")).unwrap();
            assert_eq!(bel.pins[&format!("DOA{i}")], do_);
        }
        for i in 0..4 {
            let di = bel.pins.remove(&format!("DIP{i}")).unwrap();
            assert_eq!(bel.pins[&format!("DIPB{i}")], di);
            let do_ = bel.pins.remove(&format!("DOP{i}")).unwrap();
            assert_eq!(bel.pins[&format!("DOPA{i}")], do_);
        }
        for i in 0..12 {
            let (ridx, widx) = match i {
                0..4 => (i, i + 16),
                4..8 => (i - 4 + 24, i - 4 + 20),
                8..12 => (i - 8 + 12, i - 8 + 28),
                _ => unreachable!(),
            };
            let rdcount = bel.pins.remove(&format!("RDCOUNT{i}")).unwrap();
            assert_eq!(bel.pins[&format!("DOB{ridx}")], rdcount);
            let wrcount = bel.pins.remove(&format!("WRCOUNT{i}")).unwrap();
            assert_eq!(bel.pins[&format!("DOB{widx}")], wrcount);
        }
        for (idx, pin) in [
            (5, "RDERR"),
            (6, "ALMOSTEMPTY"),
            (7, "EMPTY"),
            (8, "FULL"),
            (9, "ALMOSTFULL"),
            (10, "WRERR"),
        ] {
            let pin = bel.pins.remove(pin).unwrap();
            assert_eq!(bel.pins[&format!("DOB{idx}")], pin);
        }
        for (fpin, bpin) in [
            ("RDEN", "ENA"),
            ("RDCLK", "CLKA"),
            ("WREN", "ENB"),
            ("WRCLK", "CLKB"),
            ("RST", "SSRA"),
        ] {
            let pin = bel.pins.remove(fpin).unwrap();
            assert_eq!(bel.pins[bpin], pin);
        }
        builder.insert_tcls_bel(tcls::BRAM, bslots::BRAM, BelInfo::Legacy(bel));
        builder.insert_tcls_naming(
            "BRAM",
            TileClassNaming {
                wires: Default::default(),
                wire_bufs: Default::default(),
                ext_pips: Default::default(),
                delay_wires: Default::default(),
                bels: [(bslots::BRAM, naming)].into_iter().collect(),
                intf_wires_in: Default::default(),
            },
        );
    }

    let mut bels_dsp = vec![];
    for i in 0..2 {
        let mut bel = builder.bel_xy(bslots::DSP[i], "DSP48", 0, i).manual();
        let buf_cnt = match i {
            0 => 0,
            1 => 1,
            _ => unreachable!(),
        };
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

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        let mut xt = builder
            .xtile_id(tcls::DSP, "DSP", xy)
            .num_cells(4)
            .force_test_mux_in()
            .bels(bels_dsp.clone());
        for (i, &xy) in int_xy.iter().enumerate() {
            xt = xt.ref_int(xy, i);
        }
        let mut bels = vec![];
        let mut namings = vec![];
        for (bel, naming) in xt.extract().bels {
            bels.push(bel);
            namings.push(naming);
        }
        let mut bel_c = LegacyBel::default();
        let mut naming_c = BelNaming {
            tiles: namings[0].tiles.clone(),
            pins: Default::default(),
        };
        let mut pins = vec!["RSTC".to_string(), "CEC".to_string()];
        for i in 0..48 {
            pins.push(format!("C{i}"));
        }
        for pin in pins {
            let inp0 = bels[0].pins.remove(&pin).unwrap();
            let inp1 = bels[1].pins.remove(&pin).unwrap();
            assert_eq!(inp0, inp1);
            bel_c.pins.insert(pin.clone(), inp0);
            let mut npin = namings[0].pins[&pin].clone();
            npin.pips.clear();
            npin.name = npin.name_far.clone();
            naming_c.pins.insert(pin, npin);
        }
        for (i, bel) in bels.into_iter().enumerate() {
            builder.insert_tcls_bel(tcls::DSP, bslots::DSP[i], BelInfo::Legacy(bel));
        }
        builder.insert_tcls_bel(tcls::DSP, bslots::DSP_C, BelInfo::Legacy(bel_c));
        let mut tnaming = TileClassNaming::default();
        for (i, naming) in namings.into_iter().enumerate() {
            tnaming.bels.insert(bslots::DSP[i], naming);
        }
        tnaming.bels.insert(bslots::DSP_C, naming_c);
        builder.insert_tcls_naming("DSP", tnaming);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER").iter().next() {
        let bels = vec![
            builder.bel_xy(bslots::BSCAN[0], "BSCAN", 0, 0),
            builder.bel_xy(bslots::BSCAN[1], "BSCAN", 0, 1),
            builder.bel_xy(bslots::BSCAN[2], "BSCAN", 0, 2),
            builder.bel_xy(bslots::BSCAN[3], "BSCAN", 0, 3),
            builder.bel_xy(bslots::ICAP[0], "ICAP", 0, 0),
            builder.bel_xy(bslots::ICAP[1], "ICAP", 0, 1),
            builder.bel_single(bslots::PMV_CFG[0], "PMV"),
            builder.bel_single(bslots::STARTUP, "STARTUP"),
            builder
                .bel_single(bslots::JTAGPPC, "JTAGPPC")
                .pin_name_only("TDOTSPPC", 0),
            builder.bel_single(bslots::FRAME_ECC, "FRAME_ECC"),
            builder.bel_single(bslots::DCIRESET, "DCIRESET"),
            builder.bel_single(bslots::CAPTURE, "CAPTURE"),
            builder.bel_single(bslots::USR_ACCESS, "USR_ACCESS_SITE"),
            builder.bel_virtual(bslots::MISC_CFG),
        ];
        let mut xn = builder.xtile_id(tcls::CFG, "CFG", xy).num_cells(16);
        for i in 0..8 {
            xn = xn.ref_int(xy.delta(-1, -8 + (i as i32)), i);
        }
        for i in 0..8 {
            xn = xn.ref_int(xy.delta(-1, 1 + (i as i32)), i + 8);
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();
        let tcls = &mut builder.db.tile_classes[tcls::CFG];
        for (bslot, inp) in [
            (bslots::STARTUP, bcls::STARTUP::GTS),
            (bslots::CAPTURE, bcls::CAPTURE::CAP),
        ] {
            let BelInfo::Bel(ref mut bel) = tcls.bels[bslot] else {
                unreachable!()
            };
            let BelInput::Fixed(ref mut wire) = bel.inputs[inp] else {
                unreachable!()
            };
            wire.inv = true;
        }

        let mut bels = vec![];
        for i in 0..16 {
            bels.push(
                builder
                    .bel_xy(bslots::BUFGCTRL[i], "BUFGCTRL", 0, i)
                    .raw_tile(1)
                    .extra_wire("GFB", &[format!("CLK_BUFGCTRL_GFB_P{i}")])
                    .extra_wire("GCLK", &[format!("CLK_BUFGCTRL_GCLKP{i}")]),
            );
        }
        for i in 0..16 {
            bels.push(
                builder
                    .bel_xy(bslots::BUFGCTRL[i + 16], "BUFGCTRL", 0, i)
                    .raw_tile(2)
                    .extra_wire("GFB", &[format!("CLK_BUFGCTRL_GFB_P{i}")])
                    .extra_wire("GCLK", &[format!("CLK_BUFGCTRL_GCLKP{ii}", ii = i + 16)]),
            );
        }
        bels.push(
            builder
                .bel_virtual(bslots::SPEC_INT)
                .naming_only()
                .sub_virtual()
                .raw_tile(1)
                .extra_wire("MGT_SW0_BUFG", &["CLK_BUFGCTRL_MGT_L0"])
                .extra_wire("MGT_SW1_BUFG", &["CLK_BUFGCTRL_MGT_L1"])
                .extra_wire("MGT_SE0_BUFG", &["CLK_BUFGCTRL_MGT_R0"])
                .extra_wire("MGT_SE1_BUFG", &["CLK_BUFGCTRL_MGT_R1"])
                .sub_virtual()
                .raw_tile(2)
                .extra_wire("MGT_NW0_BUFG", &["CLK_BUFGCTRL_MGT_L0"])
                .extra_wire("MGT_NW1_BUFG", &["CLK_BUFGCTRL_MGT_L1"])
                .extra_wire("MGT_NE0_BUFG", &["CLK_BUFGCTRL_MGT_R0"])
                .extra_wire("MGT_NE1_BUFG", &["CLK_BUFGCTRL_MGT_R1"])
                .sub_virtual()
                .raw_tile(3)
                .extra_wire_force("MGT_SW0_HROW_I", "CLK_HROW_H_MGT_L0")
                .extra_wire_force("MGT_SW1_HROW_I", "CLK_HROW_H_MGT_L1")
                .extra_wire_force("MGT_SE0_HROW_I", "CLK_HROW_H_MGT_R0")
                .extra_wire_force("MGT_SE1_HROW_I", "CLK_HROW_H_MGT_R1")
                .extra_wire_force("MGT_SW0_HROW_O", "CLK_HROW_V_MGT_L0")
                .extra_wire_force("MGT_SW1_HROW_O", "CLK_HROW_V_MGT_L1")
                .extra_wire_force("MGT_SE0_HROW_O", "CLK_HROW_V_MGT_R0")
                .extra_wire_force("MGT_SE1_HROW_O", "CLK_HROW_V_MGT_R1")
                .sub_virtual()
                .raw_tile(4)
                .extra_wire_force("MGT_NW0_HROW_I", "CLK_HROW_H_MGT_L0")
                .extra_wire_force("MGT_NW1_HROW_I", "CLK_HROW_H_MGT_L1")
                .extra_wire_force("MGT_NE0_HROW_I", "CLK_HROW_H_MGT_R0")
                .extra_wire_force("MGT_NE1_HROW_I", "CLK_HROW_H_MGT_R1")
                .extra_wire_force("MGT_NW0_HROW_O", "CLK_HROW_V_MGT_L0")
                .extra_wire_force("MGT_NW1_HROW_O", "CLK_HROW_V_MGT_L1")
                .extra_wire_force("MGT_NE0_HROW_O", "CLK_HROW_V_MGT_R0")
                .extra_wire_force("MGT_NE1_HROW_O", "CLK_HROW_V_MGT_R1")
                .sub_virtual()
                .raw_tile(5)
                .extra_wire_force("MGT_SW0_HCLK_I", "HCLK_MGT_CLKL0")
                .extra_wire_force("MGT_SW1_HCLK_I", "HCLK_MGT_CLKL1")
                .extra_wire_force("MGT_SE0_HCLK_I", "HCLK_MGT_CLKR0")
                .extra_wire_force("MGT_SE1_HCLK_I", "HCLK_MGT_CLKR1")
                .extra_wire_force("MGT_SW0_HCLK_O", "HCLK_CENTER_MGT0")
                .extra_wire_force("MGT_SW1_HCLK_O", "HCLK_CENTER_MGT1")
                .extra_wire_force("MGT_SE0_HCLK_O", "HCLK_CENTER_MGT2")
                .extra_wire_force("MGT_SE1_HCLK_O", "HCLK_CENTER_MGT3")
                .sub_virtual()
                .raw_tile(6)
                .extra_wire_force("MGT_NW0_HCLK_I", "HCLK_MGT_CLKL0")
                .extra_wire_force("MGT_NW1_HCLK_I", "HCLK_MGT_CLKL1")
                .extra_wire_force("MGT_NE0_HCLK_I", "HCLK_MGT_CLKR0")
                .extra_wire_force("MGT_NE1_HCLK_I", "HCLK_MGT_CLKR1")
                .extra_wire_force("MGT_NW0_HCLK_O", "HCLK_CENTER_MGT0")
                .extra_wire_force("MGT_NW1_HCLK_O", "HCLK_CENTER_MGT1")
                .extra_wire_force("MGT_NE0_HCLK_O", "HCLK_CENTER_MGT2")
                .extra_wire_force("MGT_NE1_HCLK_O", "HCLK_CENTER_MGT3"),
        );
        let mut xn = builder
            .xtile_id(tcls::CLK_BUFG, "CLK_BUFG", xy)
            .raw_tile(xy.delta(1, -8))
            .raw_tile(xy.delta(1, 1))
            .raw_tile(xy.delta(1, -9))
            .raw_tile(xy.delta(1, 9))
            .raw_tile(xy.delta(0, -9))
            .raw_tile(xy.delta(0, 9))
            .num_cells(16)
            .extract_muxes_rt(bslots::SPEC_INT, 1)
            .extract_muxes_rt(bslots::SPEC_INT, 2)
            .force_test_mux_in();
        for i in 0..8 {
            xn = xn.ref_int(xy.delta(-1, -8 + (i as i32)), i);
        }
        for i in 0..8 {
            xn = xn.ref_int(xy.delta(-1, 1 + (i as i32)), i + 8);
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();

        let pips = builder
            .pips
            .get_mut(&(tcls::CLK_BUFG, bslots::SPEC_INT))
            .unwrap();
        for i in 0..32 {
            pips.specials
                .insert(SwitchBoxItem::WireSupport(WireSupport {
                    wires: BTreeSet::from_iter([wires::OUT_BUFG[i].cell(8)]),
                    bits: vec![],
                }));
        }
        for i in 0..16 {
            for j in [0, 2] {
                pips.specials
                    .insert(SwitchBoxItem::WireSupport(WireSupport {
                        wires: BTreeSet::from_iter([
                            wires::IMUX_SPEC[j].cell(i),
                            wires::IMUX_SPEC[j + 1].cell(i),
                        ]),
                        bits: vec![],
                    }));
            }
        }
        for c in [0, 8, 16, 17] {
            for i in 0..2 {
                pips.specials
                    .insert(SwitchBoxItem::WireSupport(WireSupport {
                        wires: BTreeSet::from_iter([wires::MGT_ROW[i].cell(c)]),
                        bits: vec![],
                    }));
            }
        }
    }

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = pb_xy.delta(0, 18);
        let mut int_xy = vec![];
        for dy in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ] {
            int_xy.push(pb_xy.delta(-1, -4 + dy));
        }
        for dy in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ] {
            int_xy.push(pb_xy.delta(15, -4 + dy));
        }
        for dx in [1, 3, 5, 7, 9, 11, 13] {
            int_xy.push(pb_xy.delta(dx, -4));
        }
        for dx in [1, 3, 5, 7, 9, 11, 13] {
            int_xy.push(pb_xy.delta(dx, 22));
        }
        let mut dcr_pins = vec![
            "EMACDCRACK".to_string(),
            "DCREMACCLK".to_string(),
            "DCREMACREAD".to_string(),
            "DCREMACWRITE".to_string(),
        ];
        for i in 0..32 {
            dcr_pins.push(format!("EMACDCRDBUS{i}"));
            dcr_pins.push(format!("DCREMACDBUS{i}"));
        }
        for i in 8..10 {
            dcr_pins.push(format!("DCREMACABUS{i}"));
        }
        builder.extract_xtile_bels_id(
            tcls::PPC,
            pb_xy,
            &[pt_xy],
            &int_xy,
            "PPC",
            &[
                builder
                    .bel_xy(bslots::PPC, "PPC405_ADV", 0, 0)
                    .pins_name_only(&dcr_pins),
                builder
                    .bel_xy(bslots::EMAC, "EMAC", 0, 0)
                    .pins_name_only(&dcr_pins),
            ],
            true,
        );
    }
    let tcls = &mut builder.db.tile_classes[tcls::PPC];
    for bel in tcls.bels.values_mut() {
        let BelInfo::Bel(bel) = bel else {
            unreachable!()
        };
        for wire in bel.inputs.values_mut() {
            let BelInput::Fixed(wire) = wire else {
                unreachable!()
            };
            if wires::IMUX_IMUX.contains(wire.wire) {
                continue;
            }
            wire.inv = true;
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CLK_HROW").iter().next() {
        builder
            .xtile_id(tcls::CLK_HROW, "CLK_HROW", xy)
            .switchbox(bslots::HROW_INT)
            .optin_muxes(&wires::HCLK_ROW[..])
            .num_cells(2)
            .extract();

        let pips = builder
            .pips
            .get_mut(&(tcls::CLK_HROW, bslots::HROW_INT))
            .unwrap();
        pips.pips.clear();
        for co in 0..2 {
            for o in 0..8 {
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
    }

    builder
        .pips
        .entry((tcls::HCLK_TERM, bslots::HROW_INT))
        .or_default()
        .specials
        .insert(SwitchBoxItem::WireSupport(WireSupport {
            wires: wires::HCLK_ROW.into_iter().map(|w| w.cell(0)).collect(),
            bits: vec![],
        }));

    builder
        .pips
        .entry((tcls::CLK_TERM, bslots::HROW_INT))
        .or_default()
        .specials
        .extend([
            SwitchBoxItem::WireSupport(WireSupport {
                wires: wires::GIOB.into_iter().map(|w| w.cell(0)).collect(),
                bits: vec![],
            }),
            SwitchBoxItem::WireSupport(WireSupport {
                wires: wires::GCLK.into_iter().map(|w| w.cell(0)).collect(),
                bits: vec![],
            }),
        ]);

    for (tcid, naming, tkn) in [
        (tcls::CLK_IOB_S, "CLK_IOB_S", "CLK_IOB_B"),
        (tcls::CLK_IOB_N, "CLK_IOB_N", "CLK_IOB_T"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(16)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::GIOB[..])
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .extract();
        }
    }

    for (tcid, naming, tkn) in [
        (tcls::CLK_DCM_S, "CLK_DCM_S", "CLKV_DCM_B"),
        (tcls::CLK_DCM_N, "CLK_DCM_N", "CLKV_DCM_T"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(8)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::IMUX_BUFG_O[..])
                .extract();
        }
    }
    for i in 0..32 {
        builder
            .pips
            .entry((tcls::CLK_DCM_S, bslots::CLK_INT))
            .or_default()
            .pips
            .insert(
                (
                    wires::IMUX_BUFG_O[i].cell(0),
                    wires::IMUX_BUFG_I[i].cell(0).pos(),
                ),
                PipMode::Mux,
            );
        builder
            .pips
            .entry((tcls::CLK_DCM_N, bslots::CLK_INT))
            .or_default()
            .pips
            .insert(
                (
                    wires::IMUX_BUFG_O[i].cell(0),
                    wires::IMUX_BUFG_I[i].cell(0).pos(),
                ),
                PipMode::Mux,
            );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK").iter().next() {
        let bel_gsig = builder.bel_xy(bslots::GLOBALSIG, "GLOBALSIG", 0, 0);
        builder
            .xtile_id(tcls::HCLK, "HCLK", xy)
            .ref_int(xy.delta(0, 1), 0)
            .extract_muxes(bslots::HCLK)
            .bel(bel_gsig)
            .extract();
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

    let bel_hclk_io_int = builder
        .bel_virtual(bslots::HCLK_IO_INT)
        .naming_only()
        .extra_wire(
            "IOCLK0",
            &[
                "HCLK_IOIS_IOCLKP0",
                "HCLK_DCMIOB_IOCLKP0",
                "HCLK_IOBDCM_IOCLKP0",
            ],
        )
        .extra_wire(
            "IOCLK1",
            &[
                "HCLK_IOIS_IOCLKP1",
                "HCLK_DCMIOB_IOCLKP0",
                "HCLK_IOBDCM_IOCLKP0",
            ],
        )
        .extra_wire(
            "VIOCLK0",
            &[
                "HCLK_IOIS_VIOCLKP0",
                "HCLK_DCMIOB_VIOCLKP0",
                "HCLK_IOBDCM_VIOCLKP0",
            ],
        )
        .extra_wire(
            "VIOCLK1",
            &[
                "HCLK_IOIS_VIOCLKP1",
                "HCLK_DCMIOB_VIOCLKP0",
                "HCLK_IOBDCM_VIOCLKP0",
            ],
        );

    for (tcid, naming, tkn) in [
        (tcls::HCLK_IO_DCI, "HCLK_IO_DCI", "HCLK_IOIS_DCI"),
        (tcls::HCLK_IO_LVDS, "HCLK_IO_LVDS", "HCLK_IOIS_LVDS"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![
                builder.bel_xy(bslots::BUFR[0], "BUFR", 0, 1),
                builder.bel_xy(bslots::BUFR[1], "BUFR", 0, 0),
                builder
                    .bel_xy(bslots::BUFIO[0], "BUFIO", 0, 1)
                    .naming_only()
                    .pins_name_only(&["I", "O"]),
                builder
                    .bel_xy(bslots::BUFIO[1], "BUFIO", 0, 0)
                    .naming_only()
                    .pins_name_only(&["I", "O"]),
                builder.bel_xy(bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0),
                bel_hclk_io_int.clone(),
            ];
            if tkn == "HCLK_IOIS_DCI" {
                bels.push(builder.bel_xy(bslots::DCI, "DCI", 0, 0));
            } else {
                bels.push(builder.bel_virtual(bslots::LVDS));
            }
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(4)
                .raw_tile(xy.delta(0, -2))
                .raw_tile(xy.delta(0, -1))
                .raw_tile(xy.delta(0, 1))
                .ref_int(xy.delta(-1, -2), 0)
                .ref_int(xy.delta(-1, -1), 1)
                .ref_int(xy.delta(-1, 1), 2)
                .ref_int(xy.delta(-1, 2), 2)
                .force_test_mux_in()
                .switchbox(bslots::HCLK_IO_INT)
                .optin_muxes(&[wires::IMUX_IDELAYCTRL_REFCLK])
                .optin_muxes(&wires::HCLK_IO[..])
                .optin_muxes(&wires::RCLK_IO[..])
                .optin_muxes(&wires::IOCLK[..])
                .optin_muxes(&wires::IOCLK_S_IO[..])
                .optin_muxes(&wires::IOCLK_N_IO[..])
                .optin_muxes(&wires::IMUX_BUFR[..])
                .optin_muxes(&wires::RCLK_ROW[..])
                .skip_edge("HCLK_IOIS_IOCLKP0", "HCLK_IOIS_VIOCLKP0")
                .skip_edge("HCLK_IOIS_IOCLKP1", "HCLK_IOIS_VIOCLKP1")
                .bels(bels)
                .extract();
        }
    }
    for (tcid, naming, tkn, ioloc, dcm_oc) in [
        (
            tcls::HCLK_IO_CENTER,
            "HCLK_IO_CENTER",
            "HCLK_CENTER",
            'S',
            None,
        ),
        (
            tcls::HCLK_IO_CFG_N,
            "HCLK_IO_CFG_N",
            "HCLK_CENTER_ABOVE_CFG",
            'N',
            None,
        ),
        (
            tcls::HCLK_IO_DCM_N,
            "HCLK_IO_DCM_N",
            "HCLK_DCMIOB",
            'N',
            Some(1),
        ),
        (
            tcls::HCLK_IO_DCM_S,
            "HCLK_IO_DCM_S",
            "HCLK_IOBDCM",
            'S',
            Some(2),
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bels = [
                builder
                    .bel_xy(bslots::BUFIO[0], "BUFIO", 0, 1)
                    .naming_only()
                    .pins_name_only(&["I", "O"]),
                builder
                    .bel_xy(bslots::BUFIO[1], "BUFIO", 0, 0)
                    .naming_only()
                    .pins_name_only(&["I", "O"]),
                builder.bel_xy(bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0),
                builder.bel_xy(bslots::DCI, "DCI", 0, 0),
                bel_hclk_io_int.clone(),
            ];
            let mut xn = builder
                .xtile_id(tcid, naming, xy)
                .num_cells(4)
                .switchbox(bslots::HCLK_IO_INT)
                .optin_muxes(&[wires::IMUX_IDELAYCTRL_REFCLK])
                .optin_muxes(&wires::HCLK_IO[..])
                .optin_muxes(&wires::RCLK_IO[..])
                .optin_muxes(&wires::IOCLK[..])
                .optin_muxes(&wires::IOCLK_S_IO[..])
                .optin_muxes(&wires::IOCLK_N_IO[..])
                .optin_muxes(&wires::HCLK_DCM[..])
                .optin_muxes(&wires::GIOB_DCM[..])
                .optin_muxes(&wires::MGT_DCM[..])
                .skip_edge("HCLK_IOIS_IOCLKP0", "HCLK_IOIS_VIOCLKP0")
                .skip_edge("HCLK_IOIS_IOCLKP1", "HCLK_IOIS_VIOCLKP1")
                .skip_edge("HCLK_DCMIOB_IOCLKP0", "HCLK_DCMIOB_VIOCLKP0")
                .skip_edge("HCLK_DCMIOB_IOCLKP1", "HCLK_DCMIOB_VIOCLKP1")
                .skip_edge("HCLK_IOBDCM_IOCLKP0", "HCLK_IOBDCM_VIOCLKP0")
                .skip_edge("HCLK_IOBDCM_IOCLKP1", "HCLK_IOBDCM_VIOCLKP1")
                .force_pip(
                    wires::IOCLK_S_IO[0].cell(2),
                    wires::IOCLK_S[0].cell(2).pos(),
                )
                .force_pip(
                    wires::IOCLK_S_IO[1].cell(2),
                    wires::IOCLK_S[1].cell(2).pos(),
                )
                .force_pip(
                    wires::IOCLK_N_IO[0].cell(2),
                    wires::IOCLK_N[0].cell(2).pos(),
                )
                .force_pip(
                    wires::IOCLK_N_IO[1].cell(2),
                    wires::IOCLK_N[1].cell(2).pos(),
                )
                .force_test_mux_in();
            if ioloc == 'S' {
                xn = xn.raw_tile(xy.delta(0, -2)).raw_tile(xy.delta(0, -1))
            } else {
                xn = xn.raw_tile(xy.delta(0, 1)).raw_tile(xy.delta(0, 2))
            }
            xn = xn
                .ref_int(xy.delta(-1, -2), 0)
                .ref_int(xy.delta(-1, -1), 1)
                .ref_int(xy.delta(-1, 1), 2)
                .ref_int(xy.delta(-1, 2), 3);
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();

            if let Some(oc) = dcm_oc {
                let pips = builder.pips.entry((tcid, bslots::HCLK_IO_INT)).or_default();
                for (wt, wf) in [
                    (wires::MGT_DCM[0].cell(oc), wires::MGT_ROW[0].cell(2)),
                    (wires::MGT_DCM[1].cell(oc), wires::MGT_ROW[1].cell(2)),
                    (wires::MGT_DCM[2].cell(oc), wires::MGT_ROW[0].cell(4)),
                    (wires::MGT_DCM[3].cell(oc), wires::MGT_ROW[1].cell(4)),
                ] {
                    pips.pips.insert((wt, wf.pos()), PipMode::Buf);
                }

                pips.specials.extend([
                    SwitchBoxItem::WireSupport(WireSupport {
                        wires: BTreeSet::from_iter(
                            wires::HCLK_DCM
                                .into_iter()
                                .chain(wires::GIOB_DCM)
                                .chain(wires::MGT_DCM)
                                .map(|w| w.cell(oc)),
                        ),
                        bits: vec![],
                    }),
                    SwitchBoxItem::WireSupport(WireSupport {
                        wires: BTreeSet::from_iter(wires::MGT_DCM.into_iter().map(|w| w.cell(oc))),
                        bits: vec![],
                    }),
                ]);
            }
        }
    }
    for tcid in [
        tcls::HCLK_IO_DCI,
        tcls::HCLK_IO_LVDS,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
    ] {
        if let Some(pips) = builder.pips.get_mut(&(tcid, bslots::HCLK_IO_INT)) {
            for (&(wt, _), mode) in pips.pips.iter_mut() {
                if wt.wire != wires::IMUX_IDELAYCTRL_REFCLK
                    && !wires::RCLK_ROW.contains(wt.wire)
                    && !wires::IMUX_BUFR.contains(wt.wire)
                {
                    *mode = PipMode::Buf;
                }
            }
            pips.pips
                .retain(|&(wt, _), _| !wires::IOCLK.contains(wt.wire));
            pips.specials
                .insert(SwitchBoxItem::WireSupport(WireSupport {
                    wires: wires::IOCLK.into_iter().map(|w| w.cell(2)).collect(),
                    bits: vec![],
                }));
        }
    }
    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_DCM").iter().next() {
        builder
            .xtile_id(tcls::HCLK_DCM, "HCLK_DCM", xy)
            .num_cells(4)
            .switchbox(bslots::HCLK_IO_INT)
            .optin_muxes(&wires::HCLK_DCM[..])
            .optin_muxes(&wires::GIOB_DCM[..])
            .optin_muxes(&wires::MGT_DCM[..])
            .extract();
        let pips = builder
            .pips
            .entry((tcls::HCLK_DCM, bslots::HCLK_IO_INT))
            .or_default();
        for mode in pips.pips.values_mut() {
            *mode = PipMode::Buf;
        }
        for (wt, wf) in [
            (wires::MGT_DCM[0].cell(2), wires::MGT_ROW[0].cell(2)),
            (wires::MGT_DCM[1].cell(2), wires::MGT_ROW[1].cell(2)),
            (wires::MGT_DCM[2].cell(2), wires::MGT_ROW[0].cell(4)),
            (wires::MGT_DCM[3].cell(2), wires::MGT_ROW[1].cell(4)),
            (wires::MGT_DCM[0].cell(1), wires::MGT_ROW[0].cell(2)),
            (wires::MGT_DCM[1].cell(1), wires::MGT_ROW[1].cell(2)),
            (wires::MGT_DCM[2].cell(1), wires::MGT_ROW[0].cell(4)),
            (wires::MGT_DCM[3].cell(1), wires::MGT_ROW[1].cell(4)),
        ] {
            pips.pips.insert((wt, wf.pos()), PipMode::Buf);
        }
        pips.specials.extend([
            SwitchBoxItem::WireSupport(WireSupport {
                wires: BTreeSet::from_iter(
                    wires::HCLK_DCM
                        .into_iter()
                        .chain(wires::GIOB_DCM)
                        .chain(wires::MGT_DCM)
                        .flat_map(|w| [w.cell(1), w.cell(2)]),
                ),
                bits: vec![],
            }),
            SwitchBoxItem::WireSupport(WireSupport {
                wires: BTreeSet::from_iter(
                    wires::HCLK_DCM
                        .into_iter()
                        .chain(wires::GIOB_DCM)
                        .flat_map(|w| [w.cell(1), w.cell(2)]),
                ),
                bits: vec![],
            }),
            SwitchBoxItem::WireSupport(WireSupport {
                wires: BTreeSet::from_iter(
                    wires::MGT_DCM
                        .into_iter()
                        .flat_map(|w| [w.cell(1), w.cell(2)]),
                ),
                bits: vec![],
            }),
        ]);
        let naming = builder
            .ndb
            .tile_class_namings
            .get_mut("HCLK_DCM")
            .unwrap()
            .1;
        for i in 0..4 {
            let lr = if i < 2 { 'L' } else { 'R' };
            naming.wires.insert(
                wires::MGT_ROW[i % 2].cell(2 + i / 2 * 2),
                WireNaming {
                    name: format!("HCLK_MGT_CLK{lr}{ii}", ii = i % 2),
                    alt_name: Some(format!("HCLK_DCM_MGT{i}")),
                    alt_pips_to: Default::default(),
                    alt_pips_from: BTreeSet::from_iter([
                        wires::MGT_DCM[i].cell(1),
                        wires::MGT_DCM[i].cell(2),
                    ]),
                },
            );
        }
    }

    for (tkn, naming) in [
        ("IOIS_LC", "IOIS_LC"),
        ("IOIS_LC_L", "IOIS_LC"),
        ("IOIS_NC", "IOIS_NC"),
        ("IOIS_NC_L", "IOIS_NC"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bels = [
                builder
                    .bel_xy(bslots::ILOGIC[0], "ILOGIC", 0, 0)
                    .pins_name_only(&[
                        "OFB",
                        "TFB",
                        "SHIFTIN1",
                        "SHIFTIN2",
                        "SHIFTOUT1",
                        "SHIFTOUT2",
                        "D",
                        "OCLK",
                    ]),
                builder
                    .bel_xy(bslots::ILOGIC[1], "ILOGIC", 0, 1)
                    .pins_name_only(&[
                        "OFB",
                        "TFB",
                        "SHIFTIN1",
                        "SHIFTIN2",
                        "SHIFTOUT1",
                        "SHIFTOUT2",
                        "D",
                        "OCLK",
                    ])
                    .extra_int_out_force("CLKPAD", wires::OUT_CLKPAD.cell(0), "IOIS_I_2GCLK0"),
                builder
                    .bel_xy(bslots::OLOGIC[0], "OLOGIC", 0, 0)
                    .pins_name_only(&["OQ", "SHIFTIN1", "SHIFTIN2", "SHIFTOUT1", "SHIFTOUT2"]),
                builder
                    .bel_xy(bslots::OLOGIC[1], "OLOGIC", 0, 1)
                    .pins_name_only(&["OQ", "SHIFTIN1", "SHIFTIN2", "SHIFTOUT1", "SHIFTOUT2"]),
                builder
                    .bel_xy(bslots::IOB[0], "IOB", 0, 0)
                    .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"])
                    .extra_wire_force(
                        "MONITOR",
                        if naming == "IOIS_LC" {
                            "IOIS_LC_MONITOR_N"
                        } else {
                            "IOIS_MONITOR_N"
                        },
                    ),
                builder
                    .bel_xy(bslots::IOB[1], "IOB", 0, 1)
                    .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"])
                    .extra_wire_force(
                        "MONITOR",
                        if naming == "IOIS_LC" {
                            "IOIS_LC_MONITOR_P"
                        } else {
                            "IOIS_MONITOR_P"
                        },
                    ),
            ];
            builder
                .xtile_id(tcls::IO, naming, xy)
                .bels(bels)
                .force_test_mux_in()
                .ref_int(xy.delta(-1, 0), 0)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&wires::IMUX_SPEC[..])
                .extract();
        }
    }

    for i in 0..12 {
        builder.extra_name(format!("DCM_TO_BUFG{i}"), wires::OUT_DCM[i]);
        builder.extra_name(format!("CCM_TO_BUFG{i}"), wires::OUT_DCM[i]);
    }

    for (i, name) in [
        (1, "DCM_CONCUR"),
        (2, "DCM_CLKFX"),
        (3, "DCM_CLKFX180"),
        (4, "DCM_CLK0"),
        (5, "DCM_CLK180"),
        (6, "DCM_CLK90"),
        (7, "DCM_CLK270"),
        (8, "DCM_CLK2X180"),
        (9, "DCM_CLK2X"),
        (10, "DCM_CLKDV"),
    ] {
        builder.extra_name(name, wires::OUT_DCM[i]);
    }
    builder.extra_name("DCM_LOCKED", wires::OUT_DCM_LOCKED);
    builder.extra_name("DCM_CLK_IN0", wires::IMUX_CLK_OPTINV[0]);

    for i in 0..2 {
        builder.extra_name(format!("PMCD_{i}_CLKA1"), wires::OUT_CCM_CLKA1[i]);
        builder.extra_name(format!("PMCD_{i}_CLKA1D2"), wires::OUT_CCM_CLKA1D2[i]);
        builder.extra_name(format!("PMCD_{i}_CLKA1D4"), wires::OUT_CCM_CLKA1D4[i]);
        builder.extra_name(format!("PMCD_{i}_CLKA1D8"), wires::OUT_CCM_CLKA1D8[i]);
        builder.extra_name(format!("PMCD_{i}_CLKB1"), wires::OUT_CCM_CLKB1[i]);
        builder.extra_name(format!("PMCD_{i}_CLKC1"), wires::OUT_CCM_CLKC1[i]);
        builder.extra_name(format!("PMCD_{i}_CLKD1"), wires::OUT_CCM_CLKD1[i]);
    }
    builder.extra_name("DPM_OSCOUT1", wires::OUT_CCM_OSCOUT1);
    builder.extra_name("DPM_OSCOUT2", wires::OUT_CCM_OSCOUT2);
    builder.extra_name("DPM_REFCLKOUT", wires::OUT_CCM_REFCLKOUT);

    for tkn in ["DCM", "DCM_BOT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            for dy in 0..4 {
                int_xy.push(xy.delta(-1, dy));
            }
            let mut bel = builder
                .bel_xy(bslots::DCM[0], "DCM_ADV", 0, 0)
                .manual()
                .pins_name_only(&["CLKIN", "CLKFB"])
                .extra_int_in("CLKIN_TEST", &["DCM_ADV_CLKIN_TEST"])
                .extra_int_in("CLKFB_TEST", &["DCM_ADV_CLKFB_TEST"]);
            for pin in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR", "LOCKED",
            ] {
                bel = bel.extra_wire(format!("{pin}_BUF"), &[format!("DCM_{pin}")]);
            }
            let mut x = builder
                .xtile_id(tcls::DCM, tkn, xy)
                .num_cells(4)
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&wires::IMUX_SPEC[..])
                .optin_muxes(&wires::DCM_DCM_O[..])
                .bel(bel)
                .force_test_mux_in();
            for (i, &xy) in int_xy.iter().enumerate() {
                x = x.ref_int(xy, i);
            }
            let xt = x.extract();
            for (mut bel, naming) in xt.bels {
                for pin in ["CLKIN", "CLKFB"] {
                    let p = bel.pins.remove(&format!("{pin}_TEST")).unwrap();
                    bel.pins.insert(pin.into(), p);
                }
                builder.insert_tcls_bel(tcls::DCM, bslots::DCM[0], BelInfo::Legacy(bel));
                builder.insert_bel_naming(tkn, bslots::DCM[0], naming);
            }
        }
    }
    let pips = builder
        .pips
        .entry((tcls::DCM, bslots::SPEC_INT))
        .or_default();
    let mut new_pips = vec![];
    let mut moves = vec![];
    pips.pips.retain(|&(wt, wf), _| {
        if let Some(idx) = wires::IMUX_CLK.index_of(wf.wire) {
            let nwf = TileWireCoord {
                wire: wires::IMUX_CLK_OPTINV[idx],
                ..wf.tw
            }
            .pos();
            new_pips.push((wt, nwf));
            moves.push((nwf, wf));
            false
        } else {
            true
        }
    });
    for pip in new_pips {
        pips.pips.insert(pip, PipMode::Mux);
    }
    for tkn in ["DCM", "DCM_BOT"] {
        let naming = builder.ndb.tile_class_namings.get_mut(tkn).unwrap().1;
        for &(nwf, wf) in &moves {
            let wn = naming.wires[&wf.tw].clone();
            naming.wires.insert(nwf.tw, wn);
        }
        naming.wires.insert(
            wires::IMUX_CLK_OPTINV[0].cell(0),
            WireNaming {
                name: "CLK_B0_INT0_DCM0".into(),
                alt_name: Some("DCM_CLK_IN0".into()),
                alt_pips_to: Default::default(),
                alt_pips_from: BTreeSet::from_iter(wires::DCM_DCM_O.into_iter().map(|w| w.cell(0))),
            },
        );
    }
    for w in wires::HCLK_DCM
        .into_iter()
        .chain(wires::GIOB_DCM)
        .chain(wires::MGT_DCM)
    {
        pips.specials
            .insert(SwitchBoxItem::WireSupport(WireSupport {
                wires: BTreeSet::from_iter([w.cell(0)]),
                bits: vec![],
            }));
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CCM").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        let mut bels = vec![];
        for i in 0..2 {
            bels.push(
                builder
                    .bel_xy(bslots::PMCD[i], "PMCD", 0, i)
                    .manual()
                    .pins_name_only(&["CLKA", "CLKB", "CLKC", "CLKD"])
                    .extra_int_in("CLKA_TEST", &[format!("PMCD_{i}_CLKA_TEST")])
                    .extra_int_in("CLKB_TEST", &[format!("PMCD_{i}_CLKB_TEST")])
                    .extra_int_in("CLKC_TEST", &[format!("PMCD_{i}_CLKC_TEST")])
                    .extra_int_in("CLKD_TEST", &[format!("PMCD_{i}_CLKD_TEST")])
                    .extra_wire("REL_TEST", &[format!("PMCD_{i}_REL_TEST")]),
            );
        }
        bels.push(
            builder
                .bel_xy(bslots::DPM, "DPM", 0, 0)
                .manual()
                .pins_name_only(&["REFCLK", "TESTCLK1", "TESTCLK2"])
                .extra_int_in("REFCLK_TEST", &["DPM_REFCLK_TEST"])
                .extra_int_in("TESTCLK1_TEST", &["DPM_TESTCLK1_TEST"])
                .extra_int_in("TESTCLK2_TEST", &["DPM_TESTCLK2_TEST"]),
        );
        bels.push(builder.bel_virtual(bslots::CCM));
        let mut x = builder
            .xtile_id(tcls::CCM, "CCM", xy)
            .num_cells(4)
            .switchbox(bslots::SPEC_INT)
            .optin_muxes(&wires::OUT_DCM[..])
            .optin_muxes(&wires::IMUX_SPEC[..])
            .optin_muxes(&wires::IMUX_CCM_REL[..])
            .force_test_mux_in()
            .bels(bels);
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        let xt = x.extract();
        for (bslot, (mut bel, naming)) in [bslots::PMCD[0], bslots::PMCD[1], bslots::DPM]
            .into_iter()
            .zip(xt.bels)
        {
            let pins = if bslot != bslots::DPM {
                ["CLKA", "CLKB", "CLKC", "CLKD"].as_slice()
            } else {
                ["REFCLK", "TESTCLK1", "TESTCLK2"].as_slice()
            };
            for &pin in pins {
                let p = bel.pins.remove(&format!("{pin}_TEST")).unwrap();
                bel.pins.insert(pin.into(), p);
            }
            builder.insert_tcls_bel(tcls::CCM, bslot, BelInfo::Legacy(bel));
            builder.insert_bel_naming("CCM", bslot, naming);
        }

        let pips = builder
            .pips
            .entry((tcls::CCM, bslots::SPEC_INT))
            .or_default();
        for (rel, rel_test) in [
            (wires::IMUX_CCM_REL[0].cell(0), wires::IMUX_SPEC[3].cell(0)),
            (wires::IMUX_CCM_REL[1].cell(0), wires::IMUX_SPEC[2].cell(0)),
        ] {
            let srcs = Vec::from_iter(
                pips.pips
                    .keys()
                    .filter_map(|&(wt, wf)| if wt == rel_test { Some(wf) } else { None }),
            );
            for src in srcs {
                pips.pips.remove(&(rel, src));
            }
            pips.pips.insert((rel, rel_test.pos()), PipMode::Mux);
        }
        let mut new_pips = vec![];
        let mut moves = vec![];
        pips.pips.retain(|&(wt, wf), _| {
            if let Some(idx) = wires::IMUX_CLK.index_of(wf.wire) {
                let nwf = TileWireCoord {
                    wire: wires::IMUX_CLK_OPTINV[idx],
                    ..wf.tw
                }
                .pos();
                new_pips.push((wt, nwf));
                moves.push((nwf, wf));
                false
            } else {
                true
            }
        });
        for pip in new_pips {
            pips.pips.insert(pip, PipMode::Mux);
        }
        for i in 0..12 {
            pips.pips.insert(
                (
                    wires::OUT_SEC_TMIN[i % 4].cell(i / 4),
                    wires::OUT_DCM[i].cell(0).pos(),
                ),
                PipMode::PermaBuf,
            );
        }
        for w in wires::HCLK_DCM
            .into_iter()
            .chain(wires::GIOB_DCM)
            .chain(wires::MGT_DCM)
        {
            pips.specials
                .insert(SwitchBoxItem::WireSupport(WireSupport {
                    wires: BTreeSet::from_iter([w.cell(0)]),
                    bits: vec![],
                }));
        }
        let naming = builder.ndb.tile_class_namings.get_mut("CCM").unwrap().1;
        for &(nwf, wf) in &moves {
            let wn = naming.wires[&wf.tw].clone();
            naming.wires.insert(nwf.tw, wn);
        }
        for i in 0..4 {
            for j in 0..4 {
                naming.wires.insert(
                    wires::OUT_SEC_TMIN[j].cell(i),
                    WireNaming {
                        name: format!("SECONDARY_LOGIC_OUTS{j}_INT{i}"),
                        alt_name: None,
                        alt_pips_to: Default::default(),
                        alt_pips_from: Default::default(),
                    },
                );
            }
        }
        for i in 0..12 {
            naming.wires.insert(
                wires::OUT_DCM[i].cell(0),
                WireNaming {
                    name: format!("CCM_TO_BUFG{i}"),
                    alt_name: None,
                    alt_pips_to: Default::default(),
                    alt_pips_from: Default::default(),
                },
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("SYS_MON").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..8 {
            int_xy.push(xy.delta(-1, dy));
        }
        let mut bel = builder
            .bel_xy(bslots::SYSMON, "MONITOR", 0, 0)
            .manual()
            .pins_name_only(&["CONVST", "VP", "VN"])
            .extra_wire("CONVST_TEST", &["MONITOR_CONVST_TEST"]);
        for i in 1..8 {
            bel = bel
                .pin_name_only(&format!("VP{i}"), 1)
                .pin_name_only(&format!("VN{i}"), 1);
        }
        bel = bel
            .sub_xy(rd, "IPAD", 0, 0)
            .pin_rename("O", "IPAD_VP_O")
            .pins_name_only(&["IPAD_VP_O"])
            .sub_xy(rd, "IPAD", 0, 1)
            .pin_rename("O", "IPAD_VN_O")
            .pins_name_only(&["IPAD_VN_O"]);
        let mut x = builder
            .xtile_id(tcls::SYSMON, "SYSMON", xy)
            .num_cells(8)
            .bel(bel)
            .switchbox(bslots::SYSMON_INT)
            .optin_muxes(&wires::IMUX_SPEC[..])
            .force_test_mux_in();
        for (i, &xy) in int_xy.iter().enumerate() {
            x = x.ref_int(xy, i);
        }
        let xt = x.extract();
        for (mut bel, naming) in xt.bels {
            bel.pins
                .insert("CONVST".into(), BelPin::new_in(wires::IMUX_SPEC[0].cell(4)));
            builder.insert_tcls_bel(tcls::SYSMON, bslots::SYSMON, BelInfo::Legacy(bel));
            builder.insert_bel_naming("SYSMON", bslots::SYSMON, naming);
        }
        let pips = builder
            .pips
            .entry((tcls::SYSMON, bslots::SYSMON_INT))
            .or_default();
        let wt = wires::IMUX_SPEC[0].cell(4);
        let wf = wires::IMUX_CLK[1].cell(0).pos();
        let wfi = wires::IMUX_CLK_OPTINV[1].cell(0).pos();
        pips.pips.remove(&(wt, wf));
        pips.pips.insert((wt, wfi), PipMode::Mux);
        let naming = builder.ndb.tile_class_namings.get_mut("SYSMON").unwrap().1;
        let wn = naming.wires[&wf.tw].clone();
        naming.wires.insert(wfi.tw, wn);
    }

    for (tkn, naming) in [
        ("MGT_AL", "MGT_W"),
        ("MGT_AL_BOT", "MGT_W"),
        ("MGT_AL_MID", "MGT_W"),
        ("MGT_AR", "MGT_E"),
        ("MGT_AR_BOT", "MGT_E"),
        ("MGT_AR_MID", "MGT_E"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(bslots::GT11[i], "GT11", 0, 0)
                    .raw_tile(i)
                    .pins_name_only(&["TX1P", "TX1N", "RX1P", "RX1N", "RXMCLK"])
                    .extra_wire("FWDCLK1_B", &["MGT_FWDCLK1_B"])
                    .extra_wire("FWDCLK2_B", &["MGT_FWDCLK2_B"])
                    .extra_wire("FWDCLK3_B", &["MGT_FWDCLK3_B"])
                    .extra_wire("FWDCLK4_B", &["MGT_FWDCLK4_B"])
                    .extra_wire("FWDCLK1_T", &["MGT_FWDCLK1_T"])
                    .extra_wire("FWDCLK2_T", &["MGT_FWDCLK2_T"])
                    .extra_wire("FWDCLK3_T", &["MGT_FWDCLK3_T"])
                    .extra_wire("FWDCLK4_T", &["MGT_FWDCLK4_T"]);
                for i in 0..16 {
                    bel = bel.pins_name_only(&[format!("COMBUSIN{i}"), format!("COMBUSOUT{i}")]);
                }
                if i == 0 {
                    bel = bel.pin_name_only("RXMCLK", 1);
                }
                bel = bel
                    .sub_xy(rd, "IPAD", 0, 0)
                    .raw_tile(i)
                    .pin_rename("O", "IPAD_RX1P_O")
                    .pins_name_only(&["IPAD_RX1P_O"])
                    .sub_xy(rd, "IPAD", 0, 1)
                    .raw_tile(i)
                    .pin_rename("O", "IPAD_RX1N_O")
                    .pins_name_only(&["IPAD_RX1N_O"])
                    .sub_xy(rd, "OPAD", 0, 0)
                    .raw_tile(i)
                    .pin_rename("I", "OPAD_TX1P_I")
                    .pins_name_only(&["OPAD_TX1P_I"])
                    .sub_xy(rd, "OPAD", 0, 1)
                    .raw_tile(i)
                    .pin_rename("I", "OPAD_TX1N_I")
                    .pins_name_only(&["OPAD_TX1N_I"]);
                bels.push(bel);
            }
            let mut bel = builder
                .bel_xy(bslots::GT11CLK, "GT11CLK", 0, 0)
                .raw_tile(2)
                .pins_name_only(&[
                    "SYNCLK1IN",
                    "SYNCLK2IN",
                    "SYNCLK1OUT",
                    "SYNCLK2OUT",
                    "RXBCLK",
                    "MGTCLKP",
                    "MGTCLKN",
                ])
                .extra_int_out("SYNCLK1", &["GT11CLK_SYNCLK1OUT_L", "GT11CLK_SYNCLK1OUT_R"])
                .extra_int_out("SYNCLK2", &["GT11CLK_SYNCLK2OUT_L", "GT11CLK_SYNCLK2OUT_R"])
                .extra_wire("SYNCLK1_S", &["GT11CLK_SYNCLK1IN"])
                .extra_wire("SYNCLK2_S", &["GT11CLK_SYNCLK2IN"]);
            for i in 0..16 {
                bel = bel
                    .extra_wire(
                        format!("COMBUSIN_A{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_IN_AL{i}"),
                            format!("GT11_COMBUS_RCLK_IN_AR{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("COMBUSIN_B{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_IN_BL{i}"),
                            format!("GT11_COMBUS_RCLK_IN_BR{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("COMBUSOUT_A{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_OUT_AL{i}"),
                            format!("GT11_COMBUS_RCLK_OUT_AR{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("COMBUSOUT_B{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_OUT_BL{i}"),
                            format!("GT11_COMBUS_RCLK_OUT_BR{i}"),
                        ],
                    );
            }
            bel = bel
                .sub_xy(rd, "IPAD", 0, 1)
                .raw_tile(2)
                .pin_rename("O", "IPAD_MGTCLKP_O")
                .pins_name_only(&["IPAD_MGTCLKP_O"])
                .sub_xy(rd, "IPAD", 0, 0)
                .raw_tile(2)
                .pin_rename("O", "IPAD_MGTCLKN_O")
                .pins_name_only(&["IPAD_MGTCLKN_O"]);
            bels.push(bel);

            let mut xn = builder
                .xtile_id(tcls::MGT, naming, xy.delta(0, -18))
                .raw_tile(xy)
                .raw_tile(xy.delta(0, -10))
                .switchbox(bslots::SPEC_INT)
                .optin_muxes(&[wires::IMUX_MGT_GREFCLK])
                .optin_muxes(&[wires::IMUX_MGT_REFCLK])
                .optin_muxes(&wires::IMUX_MGT_GREFCLK_PRE[..])
                .optin_muxes(&wires::IMUX_MGT_REFCLK_PRE[..])
                .optin_muxes(&wires::MGT_CLK_OUT[..])
                .optin_muxes(&wires::MGT_CLK_OUT_FWDCLK[..])
                .optin_muxes(&wires::MGT_FWDCLK_N[..])
                .optin_muxes(&wires::MGT_FWDCLK_S[..])
                .optin_muxes(&[wires::MGT_CLK_OUT_SYNCLK])
                .skip_edge("MGT_FWDCLK1_T", "MGT_FWDCLK1_B")
                .skip_edge("MGT_FWDCLK2_T", "MGT_FWDCLK2_B")
                .skip_edge("MGT_FWDCLK3_T", "MGT_FWDCLK3_B")
                .skip_edge("MGT_FWDCLK4_T", "MGT_FWDCLK4_B")
                .skip_edge("MGT_FWDCLK1_B", "MGT_FWDCLK1_T")
                .skip_edge("MGT_FWDCLK2_B", "MGT_FWDCLK2_T")
                .skip_edge("MGT_FWDCLK3_B", "MGT_FWDCLK3_T")
                .skip_edge("MGT_FWDCLK4_B", "MGT_FWDCLK4_T")
                .num_cells(32)
                .force_test_mux_in();
            for i in 0..32 {
                xn = xn.ref_int(
                    xy.delta(if xy.x == 0 { 1 } else { -1 }, -27 + (i + i / 8) as i32),
                    i,
                );
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    if let Some(pips) = builder.pips.get_mut(&(tcls::MGT, bslots::SPEC_INT)) {
        pips.specials
            .insert(SwitchBoxItem::WireSupport(WireSupport {
                wires: BTreeSet::from_iter([
                    wires::MGT_CLK_OUT_FWDCLK[0].cell(8),
                    wires::MGT_CLK_OUT_FWDCLK[1].cell(8),
                    wires::MGT_CLK_OUT_FWDCLK[0].cell(24),
                    wires::MGT_CLK_OUT_FWDCLK[1].cell(24),
                ]),
                bits: vec![],
            }));
    }

    builder
        .pips
        .entry((tcls::HCLK_MGT_BUF, bslots::CLK_INT))
        .or_default()
        .specials
        .extend((0..2).map(|i| {
            SwitchBoxItem::WireSupport(WireSupport {
                wires: BTreeSet::from_iter([wires::MGT_ROW[i].cell(0)]),
                bits: vec![],
            })
        }));

    builder.build()
}
