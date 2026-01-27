use prjcombine_interconnect::{
    db::{IntDb, TileWireCoord},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{IntfWireInNaming, NamingDb};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_virtex4::{
    defs,
    defs::virtex5::{ccls, tcls, wires},
};

use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;

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
    }

    for i in 0..4 {
        builder.wire_names(
            wires::TEST[i],
            &[
                format!("INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("PPC_L_INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("PPC_R_INT_INTERFACE_BLOCK_INPS_B{i}"),
                format!("GTX_LEFT_INT_INTERFACE_BLOCK_INPS_B{i}"),
            ],
        );
    }

    builder.int_type_id(tcls::INT, defs::bslots::INT, "INT", "INT");

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
        defs::bslots::INTF_TESTMUX,
        Some(defs::bslots::INTF_INT),
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
            defs::bslots::INTF_TESTMUX,
            Some(defs::bslots::INTF_INT),
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
                        .bel_xy(defs::bslots::SLICE[0], "SLICE", 0, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                    builder
                        .bel_xy(defs::bslots::SLICE[1], "SLICE", 1, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                ],
                false,
            );
        }
    }

    let intf = builder.ndb.get_tile_class_naming("INTF");

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), intf));
        }
        builder.extract_xtile_bels_intf_id(
            tcls::BRAM,
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "BRAM",
            &[builder
                .bel_xy(defs::bslots::BRAM, "RAMB36", 0, 0)
                .pins_name_only(&[
                    "CASCADEOUTLATA",
                    "CASCADEOUTLATB",
                    "CASCADEOUTREGA",
                    "CASCADEOUTREGB",
                ])
                .pin_name_only("CASCADEINLATA", 1)
                .pin_name_only("CASCADEINLATB", 1)
                .pin_name_only("CASCADEINREGA", 1)
                .pin_name_only("CASCADEINREGB", 1)],
        );
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
            &[builder.bel_xy(defs::bslots::PMVBRAM, "PMVBRAM", 0, 0)],
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
            let mut bel = builder.bel_xy(defs::bslots::DSP[i], "DSP48", 0, i);
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
            &[builder.bel_xy(defs::bslots::EMAC, "TEMAC", 0, 0)],
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
            &[builder.bel_xy(defs::bslots::PCIE, "PCIE", 0, 0)],
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
            &[builder.bel_xy(defs::bslots::PPC, "PPC440", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER").iter().next() {
        let mut bels = vec![];
        let mut bel_sysmon = builder
            .bel_xy(defs::bslots::SYSMON, "SYSMON", 0, 0)
            .pins_name_only(&["VP", "VN"]);
        for i in 0..16 {
            bel_sysmon = bel_sysmon
                .pin_name_only(&format!("VAUXP{i}"), 1)
                .pin_name_only(&format!("VAUXN{i}"), 1);
        }
        bels.extend([
            builder.bel_xy(defs::bslots::BSCAN[0], "BSCAN", 0, 0),
            builder.bel_xy(defs::bslots::BSCAN[1], "BSCAN", 0, 1),
            builder.bel_xy(defs::bslots::BSCAN[2], "BSCAN", 0, 2),
            builder.bel_xy(defs::bslots::BSCAN[3], "BSCAN", 0, 3),
            builder.bel_xy(defs::bslots::ICAP[0], "ICAP", 0, 0),
            builder.bel_xy(defs::bslots::ICAP[1], "ICAP", 0, 1),
            builder.bel_single(defs::bslots::PMV_CFG[0], "PMV"),
            builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
            builder.bel_single(defs::bslots::JTAGPPC, "JTAGPPC"),
            builder.bel_single(defs::bslots::FRAME_ECC, "FRAME_ECC"),
            builder.bel_single(defs::bslots::DCIRESET, "DCIRESET"),
            builder.bel_single(defs::bslots::CAPTURE, "CAPTURE"),
            builder.bel_single(defs::bslots::USR_ACCESS, "USR_ACCESS_SITE"),
            builder.bel_single(defs::bslots::KEY_CLEAR, "KEY_CLEAR"),
            builder.bel_single(defs::bslots::EFUSE_USR, "EFUSE_USR"),
            bel_sysmon,
            builder
                .bel_xy(defs::bslots::IPAD_VP, "IPAD", 0, 0)
                .pins_name_only(&["O"]),
            builder
                .bel_xy(defs::bslots::IPAD_VN, "IPAD", 0, 1)
                .pins_name_only(&["O"]),
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
                    .bel_xy(defs::bslots::BUFGCTRL[i], "BUFGCTRL", 0, i)
                    .raw_tile(1)
                    .pins_name_only(&["I0", "I1", "O"])
                    .extra_wire("GCLK", &[format!("CLK_BUFGMUX_GCLKP{i}")])
                    .extra_wire(
                        "GFB",
                        &[if i < 16 {
                            format!("CLK_BUFGMUX_GFB_BOT_{i}")
                        } else {
                            format!("CLK_BUFGMUX_GFB_TOP_{i}", i = i - 16)
                        }],
                    )
                    .extra_int_out("I0MUX", &[format!("CLK_BUFGMUX_PREMUX0_CLK{i}")])
                    .extra_int_out("I1MUX", &[format!("CLK_BUFGMUX_PREMUX1_CLK{i}")])
                    .extra_int_in(
                        "CKINT0",
                        &[format!(
                            "CLK_BUFGMUX_CKINT0_{ii}",
                            ii = if i < 16 { i } else { i ^ 15 }
                        )],
                    )
                    .extra_int_in(
                        "CKINT1",
                        &[format!(
                            "CLK_BUFGMUX_CKINT1_{ii}",
                            ii = if i < 16 { i } else { i ^ 15 }
                        )],
                    )
                    .extra_wire(
                        "MUXBUS0",
                        &[if i < 16 {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKB_P{ii}", ii = i * 2)
                        } else {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKT_P{ii}", ii = i * 2 - 32)
                        }],
                    )
                    .extra_wire(
                        "MUXBUS1",
                        &[if i < 16 {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKB_P{ii}", ii = i * 2 + 1)
                        } else {
                            format!("CLK_BUFGMUX_MUXED_IN_CLKT_P{ii}", ii = i * 2 - 32 + 1)
                        }],
                    ),
            );
        }
        let mut bel_mgtclk_b = builder.bel_virtual(defs::bslots::BUFG_MGTCLK_S).raw_tile(1);
        let mut bel_mgtclk_t = builder.bel_virtual(defs::bslots::BUFG_MGTCLK_N).raw_tile(1);
        for i in 0..5 {
            for lr in ['L', 'R'] {
                let ii = if lr == 'L' { 4 - i } else { i };
                bel_mgtclk_b = bel_mgtclk_b
                    .extra_wire(
                        format!("MGT_O_{lr}{i}"),
                        &[format!("CLK_BUFGMUX_{lr}MGT_CLK_BOT{ii}")],
                    )
                    .extra_wire_force(
                        format!("MGT_I_{lr}{i}"),
                        format!("CLK_BUFGMUX_MGT_CLKP_{lr}BOT{ii}"),
                    );
                bel_mgtclk_t = bel_mgtclk_t
                    .extra_wire(
                        format!("MGT_O_{lr}{i}"),
                        &[format!("CLK_BUFGMUX_{lr}MGT_CLK_TOP{ii}")],
                    )
                    .extra_wire_force(
                        format!("MGT_I_{lr}{i}"),
                        format!("CLK_BUFGMUX_MGT_CLKP_{lr}TOP{ii}"),
                    );
            }
        }
        bels.extend([bel_mgtclk_b, bel_mgtclk_t]);
        let mut xn = builder
            .xtile_id(tcls::CLK_BUFG, "CLK_BUFG", xy)
            .raw_tile(xy.delta(1, 0))
            .num_cells(20);
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
    }

    for tkn in ["LIOB", "LIOB_MON", "CIOB", "RIOB"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let ioi_xy = xy.delta(if tkn == "LIOB" { 4 } else { -1 }, 0);
            let int_xy = builder.walk_to_int(ioi_xy, Dir::W, false).unwrap();
            let intf_xy = int_xy.delta(1, 0);
            let bel_ilogic0 = builder
                .bel_xy(defs::bslots::ILOGIC[0], "ILOGIC", 0, 0)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "D",
                    "DDLY",
                    "TFB",
                    "OFB",
                    "CLK",
                    "CLKB",
                    "OCLK",
                ])
                .extra_wire("I_IOB", &["IOI_IBUF1"]);
            let bel_ilogic1 = builder
                .bel_xy(defs::bslots::ILOGIC[1], "ILOGIC", 0, 1)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "D",
                    "DDLY",
                    "TFB",
                    "OFB",
                    "CLK",
                    "CLKB",
                    "OCLK",
                ])
                .extra_wire("I_IOB", &["IOI_IBUF0"])
                .extra_wire_force("CLKOUT", "IOI_I_2GCLK0");
            let bel_ologic0 = builder
                .bel_xy(defs::bslots::OLOGIC[0], "OLOGIC", 0, 0)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "CLK",
                    "CLKDIV",
                    "OQ",
                ])
                .extra_wire("T_IOB", &["IOI_T1"])
                .extra_wire("O_IOB", &["IOI_O1"])
                .extra_int_out("CLKMUX", &["IOI_OCLKP_1"])
                .extra_int_out("CLKDIVMUX", &["IOI_OCLKDIV1"])
                .extra_int_in("CKINT", &["IOI_IMUX_B4"])
                .extra_int_in("CKINT_DIV", &["IOI_IMUX_B1"]);
            let bel_ologic1 = builder
                .bel_xy(defs::bslots::OLOGIC[1], "OLOGIC", 0, 1)
                .pins_name_only(&[
                    "SHIFTIN1",
                    "SHIFTIN2",
                    "SHIFTOUT1",
                    "SHIFTOUT2",
                    "CLK",
                    "CLKDIV",
                    "OQ",
                ])
                .extra_wire("T_IOB", &["IOI_T0"])
                .extra_wire("O_IOB", &["IOI_O0"])
                .extra_int_out("CLKMUX", &["IOI_OCLKP_0"])
                .extra_int_out("CLKDIVMUX", &["IOI_OCLKDIV0"])
                .extra_int_in("CKINT", &["IOI_IMUX_B10"])
                .extra_int_in("CKINT_DIV", &["IOI_IMUX_B7"]);
            let bel_iodelay0 = builder
                .bel_xy(defs::bslots::IODELAY[0], "IODELAY", 0, 0)
                .pins_name_only(&["IDATAIN", "ODATAIN", "T", "DATAOUT"]);
            let bel_iodelay1 = builder
                .bel_xy(defs::bslots::IODELAY[1], "IODELAY", 0, 1)
                .pins_name_only(&["IDATAIN", "ODATAIN", "T", "DATAOUT"]);

            let mut bel_iob0 = builder
                .bel_xy(defs::bslots::IOB[0], "IOB", 0, 0)
                .raw_tile(1)
                .pins_name_only(&["I", "O", "T", "PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN"]);
            let mut bel_iob1 = builder
                .bel_xy(defs::bslots::IOB[1], "IOB", 0, 1)
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
            let mut bel_ioi_clk = builder
                .bel_virtual(defs::bslots::IOI)
                .extra_int_in("CKINT0", &["IOI_IMUX_B5"])
                .extra_int_in("CKINT1", &["IOI_IMUX_B11"])
                .extra_wire("ICLK0", &["IOI_ICLKP_1"])
                .extra_wire("ICLK1", &["IOI_ICLKP_0"]);
            for i in 0..4 {
                bel_ioi_clk =
                    bel_ioi_clk.extra_wire(format!("IOCLK{i}"), &[format!("IOI_IOCLKP{i}")]);
            }
            for i in 0..4 {
                bel_ioi_clk =
                    bel_ioi_clk.extra_wire(format!("RCLK{i}"), &[format!("IOI_RCLK_FORIO_P{i}")]);
            }
            for i in 0..10 {
                bel_ioi_clk =
                    bel_ioi_clk.extra_wire(format!("HCLK{i}"), &[format!("IOI_LEAF_GCLK_P{i}")]);
            }
            builder
                .xtile_id(tcls::IO, tkn, ioi_xy)
                .raw_tile(xy)
                .ref_int(int_xy, 0)
                .ref_single(intf_xy, 0, intf)
                .bel(bel_ilogic0)
                .bel(bel_ilogic1)
                .bel(bel_ologic0)
                .bel(bel_ologic1)
                .bel(bel_iodelay0)
                .bel(bel_iodelay1)
                .bel(bel_iob0)
                .bel(bel_iob1)
                .bel(bel_ioi_clk)
                .extract();
        }
    }

    for tkn in ["CMT_BOT", "CMT_TOP"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(defs::bslots::DCM[i], "DCM_ADV", 0, i)
                        .pins_name_only(&[
                            "CLK0",
                            "CLK90",
                            "CLK180",
                            "CLK270",
                            "CLK2X",
                            "CLK2X180",
                            "CLKDV",
                            "CLKFX",
                            "CLKFX180",
                            "CONCUR",
                            "CLKIN",
                            "CLKFB",
                            "SKEWCLKIN1",
                            "SKEWCLKIN2",
                        ])
                        .extra_int_in("CKINT0", &[format!("CMT_DCM_{i}_SE_CLK_IN0")])
                        .extra_int_in("CKINT1", &[format!("CMT_DCM_{i}_SE_CLK_IN1")])
                        .extra_int_in("CKINT2", &[format!("CMT_DCM_{i}_SE_CLK_IN2")])
                        .extra_wire("CLKIN_TEST", &[format!("CMT_DCM_{i}_CLKIN_TEST")])
                        .extra_wire("CLKFB_TEST", &[format!("CMT_DCM_{i}_CLKFB_TEST")])
                        .extra_wire("MUXED_CLK", &[format!("CMT_DCM_{i}_MUXED_CLK")]),
                );
            }
            bels.push(
                builder
                    .bel_xy(defs::bslots::PLL, "PLL_ADV", 0, 0)
                    .pins_name_only(&[
                        "CLKIN1",
                        "CLKIN2",
                        "CLKFBIN",
                        "SKEWCLKIN1",
                        "SKEWCLKIN2",
                        "CLKOUT0",
                        "CLKOUT1",
                        "CLKOUT2",
                        "CLKOUT3",
                        "CLKOUT4",
                        "CLKOUT5",
                        "CLKFBOUT",
                        "CLKOUTDCM0",
                        "CLKOUTDCM1",
                        "CLKOUTDCM2",
                        "CLKOUTDCM3",
                        "CLKOUTDCM4",
                        "CLKOUTDCM5",
                        "CLKFBDCM",
                    ])
                    .extra_int_in("CKINT0", &["CMT_PLL_SE_CLK_IN0"])
                    .extra_int_in("CKINT1", &["CMT_PLL_SE_CLK_IN1"])
                    .extra_wire("CLKIN1_TEST", &["CMT_PLL_CLKIN1_TEST"])
                    .extra_wire("CLKINFB_TEST", &["CMT_PLL_CLKINFB_TEST"])
                    .extra_wire("CLKFBDCM_TEST", &["CMT_PLL_CLKFBDCM_TEST"])
                    .extra_wire("CLK_DCM_MUX", &["CMT_PLL_CLK_DCM_MUX"])
                    .extra_wire("CLK_FB_FROM_DCM", &["CMT_PLL_CLK_FB_FROM_DCM"])
                    .extra_wire("CLK_TO_DCM0", &["CMT_PLL_CLK_TO_DCM0"])
                    .extra_wire("CLK_TO_DCM1", &["CMT_PLL_CLK_TO_DCM1"]),
            );
            let mut bel = builder.bel_virtual(defs::bslots::CMT);
            for i in 0..10 {
                bel = bel
                    .extra_wire(format!("GIOB{i}"), &[format!("CMT_GIOB{i}")])
                    .extra_wire(
                        format!("HCLK{i}"),
                        &[
                            format!("CMT_BUFG{i}"),
                            format!("CMT_BUFG{i}_BOT"),
                            format!("CMT_BUFG{i}_TOP"),
                        ],
                    );
                if i < 5 {
                    continue;
                }
                if i == 5 && tkn == "CMT_BOT" {
                    continue;
                }
                if i == 6 && tkn == "CMT_TOP" {
                    continue;
                }
                bel = bel.extra_wire(
                    format!("HCLK{i}_TO_CLKIN2"),
                    &[format!("CMT_BUFG{i}_TO_CLKIN2")],
                );
            }
            for i in 0..28 {
                if i == 10 {
                    bel = bel.extra_int_out(format!("OUT{i}"), &[format!("CMT_CLK_{i:02}")]);
                } else {
                    bel = bel.extra_wire(format!("OUT{i}"), &[format!("CMT_CLK_{i:02}")]);
                }
                if !(10..18).contains(&i) {
                    bel = bel.extra_wire(format!("OUT{i}_TEST"), &[format!("CMT_CLK_{i:02}_TEST")]);
                }
            }
            bels.push(bel);
            let mut xn = builder.xtile_id(tcls::CMT, tkn, xy).num_cells(10);
            for i in 0..10 {
                xn = xn.ref_int(xy.delta(-3, i as i32), i);
                xn = xn.ref_single(xy.delta(-2, i as i32), i, intf);
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    for tkn in ["CLK_HROW", "CLK_HROW_MGT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::CLK_HROW);
            for i in 0..32 {
                bel = bel.extra_wire(format!("GCLK{i}"), &[format!("CLK_HROW_GCLK_BUF{i}")]);
            }
            for i in 0..10 {
                bel = bel.extra_wire(format!("HCLK_L{i}"), &[format!("CLK_HROW_HCLKL_P{i}")]);
                bel = bel.extra_wire(format!("HCLK_R{i}"), &[format!("CLK_HROW_HCLKR_P{i}")]);
            }
            for i in 0..5 {
                bel = bel
                    .extra_wire_force(format!("MGT_I_L{i}"), format!("CLK_HROW_MGT_CLK_P{i}_LEFT"))
                    .extra_wire_force(format!("MGT_I_R{i}"), format!("CLK_HROW_MGT_CLK_P{i}"))
                    .extra_wire_force(format!("MGT_O_L{i}"), format!("CLK_HROW_MGT_CLKV{i}_LEFT"))
                    .extra_wire_force(format!("MGT_O_R{i}"), format!("CLK_HROW_MGT_CLKV{i}"));
            }
            builder
                .xtile_id(tcls::CLK_HROW, "CLK_HROW", xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["HCLK", "HCLK_GT3", "HCLK_GTX", "HCLK_GTX_LEFT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::HCLK);
            for i in 0..10 {
                bel = bel
                    .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_G_HCLK_P{i}")])
                    .extra_int_out(format!("HCLK_O{i}"), &[format!("HCLK_LEAF_GCLK{i}")]);
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("RCLK_I{i}"), &[format!("HCLK_RCLK{i}")])
                    .extra_int_out(format!("RCLK_O{i}"), &[format!("HCLK_LEAF_RCLK{i}")]);
            }
            let bel_gsig = builder.bel_xy(defs::bslots::GLOBALSIG, "GLOBALSIG", 0, 0);
            builder
                .xtile_id(tcls::HCLK, "HCLK", xy)
                .ref_int(xy.delta(0, 1), 0)
                .bel(bel_gsig)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, tcid, naming, has_bufr, has_io_s, has_io_n, has_rclk) in [
        ("HCLK_IOI", tcls::HCLK_IO, "HCLK_IO", true, true, true, true),
        (
            "HCLK_IOI_CENTER",
            tcls::HCLK_IO_CENTER,
            "HCLK_IO_CENTER",
            false,
            true,
            true,
            true,
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
            false,
        ),
        (
            "HCLK_IOI_CMT_MGT",
            tcls::HCLK_IO_CMT_N,
            "HCLK_IO_CMT_N",
            false,
            false,
            true,
            false,
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
                    bels.push(
                        builder
                            .bel_xy(
                                defs::bslots::BUFIO[i],
                                "BUFIO",
                                0,
                                if has_io_s { i ^ 2 } else { i },
                            )
                            .pin_name_only("I", 1)
                            .pins_name_only(&["O"]),
                    )
                }
            }
            if has_io_s {
                for i in 2..4 {
                    bels.push(
                        builder
                            .bel_xy(defs::bslots::BUFIO[i], "BUFIO", 0, i ^ 3)
                            .pin_name_only("I", 1)
                            .pins_name_only(&["O"]),
                    )
                }
            }
            if has_bufr {
                for i in 0..2 {
                    bels.push(
                        builder
                            .bel_xy(defs::bslots::BUFR[i], "BUFR", 0, i)
                            .pins_name_only(&["O", "I"]),
                    )
                }
            }
            bels.push(
                builder
                    .bel_xy(defs::bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
            );
            bels.push(builder.bel_xy(defs::bslots::DCI, "DCI", 0, 0));
            let mut bel_ioclk = builder.bel_virtual(defs::bslots::IOCLK);
            for i in 0..4 {
                bel_ioclk = bel_ioclk
                    .extra_wire(format!("RCLK_I{i}"), &[format!("HCLK_IOI_RCLK{i}")])
                    .extra_wire(format!("RCLK_O{i}"), &[format!("HCLK_IOI_RCLK_FORIO_P{i}")]);
            }
            for i in 0..4 {
                if (has_io_s && i >= 2) || (has_io_n && i < 2) {
                    bel_ioclk =
                        bel_ioclk.extra_wire(format!("IOCLK{i}"), &[format!("HCLK_IOI_IOCLKP{i}")]);
                }
            }
            for i in 0..10 {
                bel_ioclk = bel_ioclk
                    .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_IOI_G_HCLK_P{i}")])
                    .extra_wire(format!("HCLK_O{i}"), &[format!("HCLK_IOI_LEAF_GCLK_P{i}")]);
            }
            if has_rclk {
                bel_ioclk = bel_ioclk
                    .extra_wire("VRCLK0", &["HCLK_IOI_VRCLK0"])
                    .extra_wire("VRCLK1", &["HCLK_IOI_VRCLK1"])
                    .extra_wire("VRCLK_S0", &["HCLK_IOI_VRCLK_S0"])
                    .extra_wire("VRCLK_S1", &["HCLK_IOI_VRCLK_S1"])
                    .extra_wire("VRCLK_N0", &["HCLK_IOI_VRCLK_N0"])
                    .extra_wire("VRCLK_N1", &["HCLK_IOI_VRCLK_N1"]);
            }
            bels.push(bel_ioclk);
            if has_bufr {
                bels.push(
                    builder
                        .bel_virtual(defs::bslots::RCLK)
                        .extra_wire("MGT0", &["HCLK_IOI_MGT_CLK_P0"])
                        .extra_wire("MGT1", &["HCLK_IOI_MGT_CLK_P1"])
                        .extra_wire("MGT2", &["HCLK_IOI_MGT_CLK_P2"])
                        .extra_wire("MGT3", &["HCLK_IOI_MGT_CLK_P3"])
                        .extra_wire("MGT4", &["HCLK_IOI_MGT_CLK_P4"])
                        .extra_int_in("CKINT0", &["HCLK_IOI_INT_RCLKMUX_B_N"])
                        .extra_int_in("CKINT1", &["HCLK_IOI_INT_RCLKMUX_B_S"]),
                );
            }

            let mut xn = builder.xtile_id(tcid, naming, xy);
            let mut t = 0;
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
                t = 2;
            }
            if has_io_n {
                xn = xn
                    .raw_tile(xy.delta(0, 1))
                    .ref_int(
                        Coord {
                            x: int_x,
                            y: xy.y + 1,
                        },
                        t,
                    )
                    .ref_single(
                        Coord {
                            x: int_x + 1,
                            y: xy.y + 1,
                        },
                        t,
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
                            t + 1,
                        )
                        .ref_single(
                            Coord {
                                x: int_x + 1,
                                y: xy.y + 2,
                            },
                            t + 1,
                            intf,
                        );
                }
            }
            if has_bufr {
                xn = xn.num_cells(4);
            } else {
                xn = xn.num_cells(2);
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
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
            let mut bel_hclk = builder.bel_virtual(defs::bslots::HCLK_CMT_HCLK);
            for i in 0..10 {
                bel_hclk = bel_hclk
                    .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_IOB_CMT_GCLK_B{i}")])
                    .extra_wire(format!("HCLK_O{i}"), &[format!("HCLK_IOB_CMT_BUFG{i}")]);
            }
            let mut bel_giob = builder.bel_virtual(defs::bslots::HCLK_CMT_GIOB).raw_tile(1);
            for i in 0..10 {
                bel_giob = bel_giob
                    .extra_wire(format!("GIOB_I{i}"), &[format!("CLK_HROW_CLK_METAL9_{i}")])
                    .extra_wire(
                        format!("GIOB_O{i}"),
                        &[format!("CLK_HROW_CLK_H_METAL9_{i}")],
                    );
            }
            builder
                .xtile_id(tcls::HCLK_CMT, "HCLK_CMT", xy)
                .num_cells(0)
                .raw_tile(xy.delta(1, 0))
                .bel(bel_hclk)
                .bel(bel_giob)
                .extract();
        }
    }

    for (tcid, naming, tkn) in [
        (tcls::CLK_IOB_S, "CLK_IOB_S", "CLK_IOB_B"),
        (tcls::CLK_IOB_N, "CLK_IOB_N", "CLK_IOB_T"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::CLK_IOB);
            for i in 0..10 {
                bel = bel.extra_wire(format!("PAD{i}"), &[format!("CLK_IOB_PAD_CLK{i}")]);
                bel = bel.extra_wire(format!("PAD_BUF{i}"), &[format!("CLK_IOB_CLK_BUF{i}")]);
                bel = bel.extra_wire(format!("GIOB{i}"), &[format!("CLK_IOB_IOB_CLKP{i}")]);
            }
            for i in 0..5 {
                bel = bel.extra_wire(
                    format!("MGT_L{i}"),
                    &[
                        format!("CLK_IOB_B_CLK_BUF{ii}", ii = 15 + i),
                        format!("CLK_IOB_T_CLK_BUF{ii}", ii = 15 + i),
                    ],
                );
                bel = bel.extra_wire(
                    format!("MGT_R{i}"),
                    &[
                        format!("CLK_IOB_B_CLK_BUF{ii}", ii = 14 - i),
                        format!("CLK_IOB_T_CLK_BUF{ii}", ii = 14 - i),
                    ],
                );
            }
            for i in 0..32 {
                bel = bel.extra_wire(format!("MUXBUS_I{i}"), &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLK_IOB_MUXED_CLKOUT{i}")],
                );
            }
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, tcid, naming) in [
        ("CLK_CMT_BOT", tcls::CLK_CMT_S, "CLK_CMT_S"),
        ("CLK_CMT_BOT_MGT", tcls::CLK_CMT_S, "CLK_CMT_S"),
        ("CLK_CMT_TOP", tcls::CLK_CMT_N, "CLK_CMT_N"),
        ("CLK_CMT_TOP_MGT", tcls::CLK_CMT_N, "CLK_CMT_N"),
    ] {
        let bt = if tcid == tcls::CLK_CMT_S { 'B' } else { 'T' };
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::CLK_CMT);
            for i in 0..28 {
                bel = bel.extra_wire(format!("CMT_CLK{i}"), &[format!("CLK_CMT_CMT_CLK_{i:02}")]);
            }
            for i in 0..5 {
                bel = bel.extra_wire_force(
                    format!("MGT_L{i}"),
                    format!("CLK_IOB_{bt}_CLK_BUF{ii}", ii = 15 + i),
                );
                bel = bel.extra_wire(
                    format!("MGT_R{i}"),
                    &[
                        format!("CLK_IOB_B_CLK_BUF{ii}", ii = 14 - i),
                        format!("CLK_IOB_T_CLK_BUF{ii}", ii = 14 - i),
                    ],
                );
            }
            for i in 0..32 {
                bel = bel.extra_wire(format!("MUXBUS_I{i}"), &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLK_IOB_MUXED_CLKOUT{i}")],
                );
            }
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, tcid, naming) in [
        ("CLK_MGT_BOT", tcls::CLK_MGT_S, "CLK_MGT_S"),
        ("CLK_MGT_BOT_MGT", tcls::CLK_MGT_S, "CLK_MGT_S"),
        ("CLK_MGT_TOP", tcls::CLK_MGT_N, "CLK_MGT_N"),
        ("CLK_MGT_TOP_MGT", tcls::CLK_MGT_N, "CLK_MGT_N"),
    ] {
        let bt = if tcid == tcls::CLK_MGT_S { 'B' } else { 'T' };
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::CLK_MGT);
            for i in 0..5 {
                bel =
                    bel.extra_wire_force(format!("MGT_L{i}"), format!("CLK_MGT_{bt}_CLK{i}_LEFT"));
                bel = bel.extra_wire(
                    format!("MGT_R{i}"),
                    &[format!("CLK_MGT_B_CLK{i}"), format!("CLK_MGT_T_CLK{i}")],
                );
            }
            for i in 0..32 {
                bel = bel.extra_wire(format!("MUXBUS_I{i}"), &[format!("CLK_IOB_MUXED_CLKIN{i}")]);
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLK_IOB_MUXED_CLKOUT{i}")],
                );
            }
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["HCLK_BRAM_MGT", "HCLK_BRAM_MGT_LEFT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::HCLK_MGT_BUF);
            for i in 0..5 {
                bel = bel
                    .extra_wire(format!("MGT_I{i}"), &[format!("HCLK_BRAM_MGT_CLK_IN_P{i}")])
                    .extra_wire(
                        format!("MGT_O{i}"),
                        &[format!("HCLK_BRAM_MGT_CLK_OUT_P{i}")],
                    );
            }
            builder
                .xtile_id(tcls::HCLK_MGT_BUF, "HCLK_BRAM_MGT", xy)
                .num_cells(0)
                .bel(bel)
                .extract();
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
                (defs::bslots::GTP_DUAL, "GTP_DUAL")
            } else {
                (defs::bslots::GTX_DUAL, "GTX_DUAL")
            };
            let bels = [
                builder
                    .bel_xy(slot, gtkind, 0, 0)
                    .pins_name_only(&[
                        "RXP0", "RXN0", "RXP1", "RXN1", "TXP0", "TXN0", "TXP1", "TXN1", "CLKIN",
                    ])
                    .extra_wire("CLKOUT_NORTH_S", &["GT3_CLKOUT_NORTH"])
                    .extra_wire("CLKOUT_NORTH", &["GT3_CLKOUT_NORTH_N"])
                    .extra_wire("CLKOUT_SOUTH", &["GT3_CLKOUT_SOUTH"])
                    .extra_wire("CLKOUT_SOUTH_N", &["GT3_CLKOUT_SOUTH_N"])
                    .extra_wire("MGT0", &["GT3_MGT_CLK_P0"])
                    .extra_wire("MGT1", &["GT3_MGT_CLK_P1"])
                    .extra_wire("MGT2", &["GT3_MGT_CLK_P2"])
                    .extra_wire("MGT3", &["GT3_MGT_CLK_P3"])
                    .extra_wire("MGT4", &["GT3_MGT_CLK_P4"])
                    .extra_int_in(
                        "GREFCLK",
                        &["GT3_GREFCLK", "GTX_GREFCLK", "GTX_LEFT_GREFCLK"],
                    ),
                builder
                    .bel_xy(defs::bslots::BUFDS[0], "BUFDS", 0, 0)
                    .pins_name_only(&["IP", "IN", "O"]),
                builder.bel_xy(defs::bslots::CRC64[0], "CRC64", 0, 0),
                builder.bel_xy(defs::bslots::CRC64[1], "CRC64", 0, 1),
                builder.bel_xy(defs::bslots::CRC32[0], "CRC32", 0, 0),
                builder.bel_xy(defs::bslots::CRC32[1], "CRC32", 0, 1),
                builder.bel_xy(defs::bslots::CRC32[2], "CRC32", 0, 2),
                builder.bel_xy(defs::bslots::CRC32[3], "CRC32", 0, 3),
                builder
                    .bel_xy(defs::bslots::IPAD_RXP[0], "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_RXN[0], "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_RXP[1], "IPAD", 0, 3)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_RXN[1], "IPAD", 0, 2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_CLKP[0], "IPAD", 0, 5)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_CLKN[0], "IPAD", 0, 4)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::OPAD_TXP[0], "OPAD", 0, 1)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy(defs::bslots::OPAD_TXN[0], "OPAD", 0, 0)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy(defs::bslots::OPAD_TXP[1], "OPAD", 0, 3)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy(defs::bslots::OPAD_TXN[1], "OPAD", 0, 2)
                    .pins_name_only(&["I"]),
            ];

            let mut xn = builder.xtile_id(tcid, tkn, xy).num_cells(20);
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
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    builder.build()
}
