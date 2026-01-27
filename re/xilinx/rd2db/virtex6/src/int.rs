use prjcombine_interconnect::{
    db::IntDb,
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;
use prjcombine_virtex4::{
    defs,
    defs::virtex6::{ccls, tcls, wires},
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
            wires::TEST[i],
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

    builder.int_type_id(tcls::INT, defs::bslots::INT, "INT", "INT");

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
        defs::bslots::INTF_TESTMUX,
        Some(defs::bslots::INTF_INT),
        true,
        false,
    );
    builder.extract_intf_id(
        tcls::INTF,
        Dir::E,
        "IOI_L_INT_INTERFACE",
        "INTF_IOI_L",
        defs::bslots::INTF_TESTMUX,
        Some(defs::bslots::INTF_INT),
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
            defs::bslots::INTF_TESTMUX,
            Some(defs::bslots::INTF_INT),
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
            .bel_xy(defs::bslots::BRAM_F, "RAMB36", 0, 0)
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
        let bel_bram_h0 = builder.bel_xy(defs::bslots::BRAM_H[0], "RAMB18", 0, 0);
        let mut bel_bram_h1 = builder
            .bel_xy(defs::bslots::BRAM_H[1], "RAMB18", 0, 1)
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
            &[builder.bel_xy(defs::bslots::PMVBRAM, "PMVBRAM", 0, 0)],
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
        bels_dsp.push(
            builder
                .bel_xy(defs::bslots::TIEOFF_DSP, "TIEOFF", 0, 0)
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
            &[builder.bel_xy(defs::bslots::EMAC, "TEMAC", 0, 0)],
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
            &[builder.bel_xy(defs::bslots::PCIE, "PCIE", 0, 0)],
        );
    }

    for (tkn, naming) in [
        ("HCLK", "HCLK"),
        ("HCLK_QBUF_L", "HCLK.QBUF"),
        ("HCLK_QBUF_R", "HCLK.QBUF"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel_gsig = builder.bel_xy(defs::bslots::GLOBALSIG, "GLOBALSIG", 0, 0);
            let mut bel = builder.bel_virtual(defs::bslots::HCLK);
            for i in 0..8 {
                bel = bel
                    .extra_int_out(format!("LCLK{i}_D"), &[format!("HCLK_LEAF_CLK_B_BOT{i}")])
                    .extra_int_out(format!("LCLK{i}_U"), &[format!("HCLK_LEAF_CLK_B_TOP{i}")]);
            }
            for i in 0..12 {
                bel = bel.extra_wire(
                    format!("HCLK{i}"),
                    &[
                        format!("HCLK_CK_BUFHCLK{i}"),
                        format!("HCLK_QBUF_CK_BUFHCLK{i}"),
                    ],
                );
            }
            for i in 0..6 {
                bel = bel.extra_wire(
                    format!("RCLK{i}"),
                    &[
                        format!("HCLK_CK_BUFRCLK{i}"),
                        format!("HCLK_QBUF_CK_BUFRCLK{i}"),
                    ],
                );
            }
            builder
                .xtile_id(tcls::HCLK, naming, xy)
                .num_cells(2)
                .ref_int(xy.delta(0, -1), 0)
                .ref_int(xy.delta(0, 1), 1)
                .bel(bel_gsig)
                .bel(bel)
                .extract();
            if naming == "HCLK.QBUF" {
                let mut bel = builder.bel_virtual(defs::bslots::HCLK_QBUF);
                for i in 0..12 {
                    bel = bel
                        .extra_wire(format!("HCLK{i}_O"), &[format!("HCLK_QBUF_CK_BUFHCLK{i}")])
                        .extra_wire(
                            format!("HCLK{i}_I"),
                            &[format!("HCLK_QBUF_CK_BUFH2QBUF{i}")],
                        );
                }
                builder
                    .xtile_id(tcls::HCLK_QBUF, "HCLK_QBUF", xy)
                    .num_cells(0)
                    .bel(bel)
                    .extract();
            }
        }
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
                bels.push(
                    builder
                        .bel_xy(defs::bslots::BUFIO[i], "BUFIODQS", 0, i ^ 2)
                        .pins_name_only(&["I", "O"]),
                );
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(defs::bslots::BUFR[i], "BUFR", 0, i ^ 1)
                        .pins_name_only(&["I", "O"]),
                );
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(defs::bslots::BUFO[i], "BUFO", 0, i ^ 1)
                        .pins_name_only(&["I", "O"])
                        .extra_wire("VI", &[format!("HCLK_IOI_VBUFOCLK{i}")])
                        .extra_wire("VI_S", &[format!("HCLK_IOI_VBUFOCLK_SOUTH{i}")])
                        .extra_wire("VI_N", &[format!("HCLK_IOI_VBUFOCLK_NORTH{i}")])
                        .extra_wire("I_PRE", &[format!("HCLK_IOI_BUFOCLK{i}")])
                        .extra_wire("I_PRE2", &[format!("HCLK_IOI_CLKB_TO_BUFO{i}")]),
                );
            }
            bels.push(
                builder
                    .bel_xy(defs::bslots::IDELAYCTRL, "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
            );
            bels.push(
                builder
                    .bel_xy(defs::bslots::DCI, "DCI", 0, 0)
                    .pins_name_only(&[
                        "DCIDATA",
                        "DCIADDRESS0",
                        "DCIADDRESS1",
                        "DCIADDRESS2",
                        "DCIIOUPDATE",
                        "DCIREFIOUPDATE",
                        "DCISCLK",
                    ]),
            );
            let mut bel = builder
                .bel_virtual(defs::bslots::HCLK_IO)
                .extra_int_in("BUFR_CKINT0", &["HCLK_IOI_RCLK_IMUX_BOT_B"])
                .extra_int_in("BUFR_CKINT1", &["HCLK_IOI_RCLK_IMUX_TOP_B"]);
            for i in 0..12 {
                bel = bel
                    .extra_wire(format!("HCLK{i}_O"), &[format!("HCLK_IOI_LEAF_GCLK{i}")])
                    .extra_wire(format!("HCLK{i}_I"), &[format!("HCLK_IOI_CK_BUFHCLK{i}")]);
            }
            for i in 0..6 {
                bel = bel
                    .extra_wire(format!("RCLK{i}_O"), &[format!("HCLK_IOI_RCLK_TO_IO{i}")])
                    .extra_wire(format!("RCLK{i}_I"), &[format!("HCLK_IOI_CK_BUFRCLK{i}")]);
            }
            for i in 0..2 {
                bel = bel.extra_wire(format!("OCLK{i}"), &[format!("HCLK_IOI_OCLK{i}")]);
            }
            for i in 0..2 {
                bel = bel.extra_wire(format!("VRCLK{i}"), &[format!("HCLK_IOI_VRCLK{i}")]);
                bel = bel.extra_wire(format!("VRCLK{i}_S"), &[format!("HCLK_IOI_VRCLK_SOUTH{i}")]);
                bel = bel.extra_wire(format!("VRCLK{i}_N"), &[format!("HCLK_IOI_VRCLK_NORTH{i}")]);
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(
                        format!("PERF{i}"),
                        &[if tkn == "HCLK_INNER_IOI" {
                            format!("HCLK_IOI_CK_PERF_INNER{i}")
                        } else {
                            format!("HCLK_IOI_CK_PERF_OUTER{i}")
                        }],
                    )
                    .extra_wire(
                        format!("PERF{i}_BUF"),
                        &[format!("HCLK_IOI_IO_PLL_CLK{ii}_BUFF", ii = i ^ 1)],
                    )
                    .extra_wire(
                        format!("IOCLK_IN{i}"),
                        &[format!("HCLK_IOI_IO_PLL_CLK{i}_DMUX")],
                    )
                    .extra_wire(
                        format!("IOCLK_IN{i}_BUFR"),
                        &[if i < 2 {
                            format!("HCLK_IOI_RCLK_TOP{i}")
                        } else {
                            format!("HCLK_IOI_RCLK_BOT{ii}", ii = i - 2)
                        }],
                    )
                    .extra_wire(
                        format!("IOCLK_PAD{i}"),
                        &[if i < 2 {
                            format!("HCLK_IOI_I2IOCLK_TOP{i}")
                        } else {
                            format!("HCLK_IOI_I2IOCLK_BOT{ii}", ii = i - 2)
                        }],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("IOCLK{i}"), &[format!("HCLK_IOI_IOCLK{i}")])
                    .extra_wire(
                        format!("IOCLK{ii}", ii = i + 4),
                        &[format!("HCLK_IOI_IOCLKMULTI{i}")],
                    )
                    .extra_wire(format!("IOCLK{i}_DLY"), &[format!("HCLK_IOI_IOCLK{i}_DLY")])
                    .extra_wire(
                        format!("IOCLK{ii}_DLY", ii = i + 4),
                        &[format!("HCLK_IOI_IOCLKMULTI{i}_DLY")],
                    );
            }
            bel = bel
                .extra_wire("IOCLK0_PRE", &["HCLK_IOI_VIOCLK0"])
                .extra_wire("IOCLK1_PRE", &["HCLK_IOI_SIOCLK1"])
                .extra_wire("IOCLK2_PRE", &["HCLK_IOI_SIOCLK2"])
                .extra_wire("IOCLK3_PRE", &["HCLK_IOI_VIOCLK1"])
                .extra_wire("IOCLK0_PRE_S", &["HCLK_IOI_VIOCLK_SOUTH0"])
                .extra_wire("IOCLK3_PRE_S", &["HCLK_IOI_VIOCLK_SOUTH1"])
                .extra_wire("IOCLK0_PRE_N", &["HCLK_IOI_VIOCLK_NORTH0"])
                .extra_wire("IOCLK3_PRE_N", &["HCLK_IOI_VIOCLK_NORTH1"]);
            for i in 0..10 {
                bel = bel.extra_wire(format!("MGT{i}"), &[format!("HCLK_IOI_CK_MGT{i}")]);
            }
            bels.push(bel);
            builder
                .xtile_id(tcls::HCLK_IO, if is_l { naming_l } else { naming_r }, xy)
                .raw_tile(xy.delta(0, -2))
                .raw_tile(xy.delta(0, 1))
                .num_cells(2)
                .ref_int(hclk_xy.delta(0, -1), 0)
                .ref_int(hclk_xy.delta(0, 1), 1)
                .ref_single(hclk_xy.delta(1, -1), 0, intf_io)
                .ref_single(hclk_xy.delta(1, 1), 1, intf_io)
                .bels(bels)
                .extract();
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
                    .bel_xy(defs::bslots::ILOGIC[i], "ILOGIC", 0, i)
                    .pins_name_only(&[
                        "CLK",
                        "CLKB",
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
                    .extra_wire("IOB_I_BUF", &[format!("LIOI_I{ii}"), format!("RIOI_I{ii}")])
                    .extra_int_in("CKINT0", &[format!("IOI_IMUX_B14_{i}")])
                    .extra_int_in("CKINT1", &[format!("IOI_IMUX_B15_{i}")]);
                if i == 1 {
                    bel = bel
                        .extra_wire_force("CLKOUT", format!("{lr}IOI_I_2IOCLK_BOT1"))
                        .extra_wire_force("CLKOUT_CMT", format!("{lr}IOI_I_2IOCLK_BOT1_I2GCLK"));
                }
                bels.push(bel);
            }
            for i in 0..2 {
                let ii = i ^ 1;
                bels.push(
                    builder
                        .bel_xy(defs::bslots::OLOGIC[i], "OLOGIC", 0, i)
                        .pins_name_only(&[
                            "CLK",
                            "CLKB",
                            "CLKDIVB",
                            "CLKPERF",
                            "CLKPERFDELAY",
                            "OFB",
                            "TFB",
                            "TQ",
                            "OQ",
                            "SHIFTIN1",
                            "SHIFTIN2",
                            "SHIFTOUT1",
                            "SHIFTOUT2",
                            "REV",
                        ])
                        .extra_int_out(
                            "CLKDIV",
                            &[
                                format!("LIOI_OLOGIC{ii}_CLKDIV"),
                                format!("RIOI_OLOGIC{ii}_CLKDIV"),
                            ],
                        )
                        .extra_int_in("CLKDIV_CKINT", &[format!("IOI_IMUX_B20_{i}")])
                        .extra_int_in("CLK_CKINT", &[format!("IOI_IMUX_B21_{i}")])
                        .extra_int_out("CLK_MUX", &[format!("IOI_OCLK_{ii}")])
                        .extra_wire("CLKM", &[format!("IOI_OCLKM_{ii}")])
                        .extra_int_out(
                            "TFB_BUF",
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
                        .bel_xy(defs::bslots::IODELAY[i], "IODELAY", 0, i)
                        .pins_name_only(&["CLKIN", "IDATAIN", "ODATAIN", "DATAOUT", "T"]),
                );
            }
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(defs::bslots::IOB[i], "IOB", 0, i)
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
            let mut bel = builder.bel_virtual(defs::bslots::IOI);
            for i in 0..2 {
                bel = bel.extra_wire(format!("OCLK{i}"), &[format!("IOI_BUFO_CLK{i}")])
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("IOCLK{i}"), &[format!("IOI_IOCLK{i}")])
            }
            for i in 0..12 {
                bel = bel.extra_wire(format!("HCLK{i}"), &[format!("IOI_LEAF_GCLK{i}")])
            }
            for i in 0..6 {
                bel = bel.extra_wire(format!("RCLK{i}"), &[format!("IOI_RCLK_FORIO{i}")])
            }
            bels.push(bel);
            builder
                .xtile_id(tcls::IO, tkn, xy)
                .raw_tile(if is_l {
                    xy.delta(-1, 0)
                } else {
                    xy.delta(1, 0)
                })
                .num_cells(2)
                .ref_int(int_xy, 0)
                .ref_int(int_xy.delta(0, 1), 1)
                .ref_single(int_xy.delta(1, 0), 0, intf_io)
                .ref_single(int_xy.delta(1, 1), 1, intf_io)
                .bels(bels)
                .extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER_0").iter().next() {
        let intf = builder.ndb.get_tile_class_naming("INTF");
        let mut bel_sysmon = builder
            .bel_xy(defs::bslots::SYSMON, "SYSMON", 0, 0)
            .raw_tile(2)
            .pins_name_only(&["VP", "VN"]);
        for i in 0..16 {
            bel_sysmon = bel_sysmon
                .pin_name_only(&format!("VAUXP{i}"), 1)
                .pin_name_only(&format!("VAUXN{i}"), 1);
        }
        let bels = [
            builder
                .bel_xy(defs::bslots::BSCAN[0], "BSCAN", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::BSCAN[1], "BSCAN", 0, 1)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::BSCAN[2], "BSCAN", 0, 0)
                .raw_tile(2),
            builder
                .bel_xy(defs::bslots::BSCAN[3], "BSCAN", 0, 1)
                .raw_tile(2),
            builder
                .bel_xy(defs::bslots::ICAP[0], "ICAP", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::ICAP[1], "ICAP", 0, 0)
                .raw_tile(2),
            builder
                .bel_xy(defs::bslots::PMV_CFG[0], "PMV", 0, 0)
                .raw_tile(0),
            builder
                .bel_xy(defs::bslots::PMV_CFG[1], "PMV", 0, 0)
                .raw_tile(3),
            builder
                .bel_xy(defs::bslots::STARTUP, "STARTUP", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::CAPTURE, "CAPTURE", 0, 0)
                .raw_tile(1),
            builder
                .bel_single(defs::bslots::FRAME_ECC, "FRAME_ECC")
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::EFUSE_USR, "EFUSE_USR", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::USR_ACCESS, "USR_ACCESS", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::DNA_PORT, "DNA_PORT", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::DCIRESET, "DCIRESET", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(defs::bslots::CFG_IO_ACCESS, "CFG_IO_ACCESS", 0, 0)
                .raw_tile(1),
            bel_sysmon,
            builder
                .bel_xy(defs::bslots::IPAD_VP, "IPAD", 0, 0)
                .raw_tile(2)
                .pins_name_only(&["O"]),
            builder
                .bel_xy(defs::bslots::IPAD_VN, "IPAD", 0, 1)
                .raw_tile(2)
                .pins_name_only(&["O"]),
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
                let slots = [defs::bslots::BUFHCE_W, defs::bslots::BUFHCE_E][i];
                for j in 0..12 {
                    bels.push(
                        builder
                            .bel_xy(slots[j], "BUFHCE", i, j)
                            .raw_tile(2)
                            .pins_name_only(&["I", "O"]),
                    );
                }
            }
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(defs::bslots::MMCM[i], "MMCM_ADV", 0, 0)
                    .raw_tile(i)
                    .pins_name_only(&[
                        "CLKIN1",
                        "CLKIN2",
                        "CLKFBIN",
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
                    ])
                    .extra_wire("CLKIN1_HCLK", &["CMT_CLKIN1_HCLK"])
                    .extra_wire("CLKIN1_IO", &["CMT_CLKIN1_IO"])
                    .extra_wire("CLKIN1_MGT", &["CMT_CLKIN1_MGT"])
                    .extra_int_in("CLKIN1_CKINT", &["CMT_MMCM_IMUX_CLKIN1"])
                    .extra_wire("CLKIN2_HCLK", &["CMT_CLKIN2_HCLK"])
                    .extra_wire("CLKIN2_IO", &["CMT_CLKIN2_IO"])
                    .extra_wire("CLKIN2_MGT", &["CMT_CLKIN2_MGT"])
                    .extra_int_in("CLKIN2_CKINT", &["CMT_MMCM_IMUX_CLKIN2"])
                    .extra_wire("CLKFBIN_HCLK", &["CMT_CLKFB_HCLK"])
                    .extra_wire("CLKFBIN_IO", &["CMT_CLKFB_IO"])
                    .extra_int_in("CLKFBIN_CKINT", &["CMT_MMCM_IMUX_CLKFB"])
                    .extra_wire("CLKFB", &["CMT_MMCM_CLKFB"])
                    .extra_wire("CASC_IN", &["CMT_MMCM_CASC_IN"])
                    .extra_wire("CASC_OUT", &["CMT_MMCM_CASC_OUT"]);
                for i in 0..14 {
                    bel = bel.extra_wire(format!("CMT_OUT{i}"), &[format!("CMT_CK_MMCM_{i}")]);
                }
                for i in 0..4 {
                    bel = bel
                        .extra_wire(format!("PERF{i}"), &[format!("CMT_PERF_CLK_BOUNCE{i}")])
                        .extra_wire(format!("PERF{i}_OL"), &[format!("CMT_CK_PERF_OUTER_L{i}")])
                        .extra_wire(format!("PERF{i}_IL"), &[format!("CMT_CK_PERF_INNER_L{i}")])
                        .extra_wire(format!("PERF{i}_IR"), &[format!("CMT_CK_PERF_INNER_R{i}")])
                        .extra_wire(format!("PERF{i}_OR"), &[format!("CMT_CK_PERF_OUTER_R{i}")]);
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy(defs::bslots::PPR_FRAME, "PPR_FRAME", 0, 0)
                    .raw_tile(1),
            );
            let mut bel = builder
                .bel_virtual(defs::bslots::CMT)
                .raw_tile(2)
                .extra_wire("BUFH_TEST_L_PRE", &["HCLK_CMT_CK_BUFH_TEST_OUT_L"])
                .extra_wire("BUFH_TEST_L_INV", &["HCLK_CMT_CK_BUFH_TEST_INV_L"])
                .extra_wire("BUFH_TEST_L_NOINV", &["HCLK_CMT_CK_BUFH_TEST_NOINV_L"])
                .extra_wire("BUFH_TEST_L", &["HCLK_CMT_CK_BUFH_TEST_L"])
                .extra_wire("BUFH_TEST_R_PRE", &["HCLK_CMT_CK_BUFH_TEST_OUT_R"])
                .extra_wire("BUFH_TEST_R_INV", &["HCLK_CMT_CK_BUFH_TEST_INV_R"])
                .extra_wire("BUFH_TEST_R_NOINV", &["HCLK_CMT_CK_BUFH_TEST_NOINV_R"])
                .extra_wire("BUFH_TEST_R", &["HCLK_CMT_CK_BUFH_TEST_R"])
                .extra_int_in("BUFHCE_L_CKINT0", &["HCLK_CMT_CLK_0_B0"])
                .extra_int_in("BUFHCE_L_CKINT1", &["HCLK_CMT_CLK_0_B1"])
                .extra_int_in("BUFHCE_R_CKINT0", &["HCLK_CMT_CLK_1_B0"])
                .extra_int_in("BUFHCE_R_CKINT1", &["HCLK_CMT_CLK_1_B1"])
                .extra_wire("MMCM0_CLKIN1_HCLK_L", &["HCLK_CMT_CK_OUT2CMT_L2"])
                .extra_wire("MMCM0_CLKIN1_HCLK_R", &["HCLK_CMT_CK_OUT2CMT_EXT_R2"])
                .extra_wire("MMCM1_CLKIN1_HCLK_L", &["HCLK_CMT_CK_OUT2CMT_EXT_L2"])
                .extra_wire("MMCM1_CLKIN1_HCLK_R", &["HCLK_CMT_CK_OUT2CMT_R2"])
                .extra_wire("MMCM0_CLKIN2_HCLK_L", &["HCLK_CMT_CK_OUT2CMT_L1"])
                .extra_wire("MMCM0_CLKIN2_HCLK_R", &["HCLK_CMT_CK_OUT2CMT_EXT_R1"])
                .extra_wire("MMCM1_CLKIN2_HCLK_L", &["HCLK_CMT_CK_OUT2CMT_EXT_L1"])
                .extra_wire("MMCM1_CLKIN2_HCLK_R", &["HCLK_CMT_CK_OUT2CMT_R1"])
                .extra_wire("MMCM0_CLKFBIN_HCLK_L", &["HCLK_CMT_CK_OUT2CMT_L0"])
                .extra_wire("MMCM0_CLKFBIN_HCLK_R", &["HCLK_CMT_CK_OUT2CMT_EXT_R0"])
                .extra_wire("MMCM1_CLKFBIN_HCLK_L", &["HCLK_CMT_CK_OUT2CMT_EXT_L0"])
                .extra_wire("MMCM1_CLKFBIN_HCLK_R", &["HCLK_CMT_CK_OUT2CMT_R0"]);
            for i in 0..32 {
                bel = bel
                    .extra_wire(format!("GCLK{i}"), &[format!("HCLK_CMT_CK_GCLK{i}")])
                    .extra_wire(
                        format!("GCLK{i}_INV"),
                        &[format!("HCLK_CMT_CK_GCLK_INV_TEST{i}")],
                    )
                    .extra_wire(
                        format!("GCLK{i}_NOINV"),
                        &[format!("HCLK_CMT_CK_GCLK_NOINV_TEST{i}")],
                    )
                    .extra_wire(
                        format!("GCLK{i}_TEST"),
                        &[format!("HCLK_CMT_CK_GCLK_TEST{i}")],
                    )
                    .extra_wire(
                        format!("CASCO{i}"),
                        &[
                            format!("HCLK_CMT_BOT_CK_BUFG_CASCO{i}"),
                            format!("HCLK_CMT_TOP_CK_BUFG_CASCO{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("CASCI{i}"),
                        &[
                            format!("HCLK_CMT_BOT_CK_BUFG_CASCIN{i}"),
                            format!("HCLK_CMT_TOP_CK_BUFG_CASCIN{i}"),
                        ],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("CCIO{i}_L"), &[format!("HCLK_CMT_CK_CCIO_L{i}")])
                    .extra_wire(format!("CCIO{i}_R"), &[format!("HCLK_CMT_CK_CCIO_R{i}")]);
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("GIO{i}"), &[format!("HCLK_CMT_CK_IO_TO_CMT{i}")]);
            }
            for i in 0..12 {
                bel = bel
                    .extra_wire(
                        format!("HCLK{i}_L_O"),
                        &[format!("HCLK_CMT_CK_BUFH2QBUF_L{i}")],
                    )
                    .extra_wire(
                        format!("HCLK{i}_R_O"),
                        &[format!("HCLK_CMT_CK_BUFH2QBUF_R{i}")],
                    )
                    .extra_wire(format!("HCLK{i}_L_I"), &[format!("HCLK_CMT_CK_HCLK_L{i}")])
                    .extra_wire(format!("HCLK{i}_R_I"), &[format!("HCLK_CMT_CK_HCLK_R{i}")]);
            }
            for i in 0..6 {
                bel = bel
                    .extra_wire(format!("RCLK{i}_L_I"), &[format!("HCLK_CMT_CK_RCLK_L{i}")])
                    .extra_wire(format!("RCLK{i}_R_I"), &[format!("HCLK_CMT_CK_RCLK_R{i}")]);
            }
            for i in 0..10 {
                bel = bel
                    .extra_wire(format!("MGT{i}_L"), &[format!("HCLK_CMT_CK_MGT_L{i}")])
                    .extra_wire(format!("MGT{i}_R"), &[format!("HCLK_CMT_CK_MGT_R{i}")]);
            }
            for (bt, key) in [('B', "MMCM0"), ('T', "MMCM1")] {
                bel = bel
                    .extra_wire(
                        format!("{key}_CLKIN1_HCLK"),
                        &[format!("HCLK_CMT_CLKIN1_HCLK_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKIN1_IO"),
                        &[format!("HCLK_CMT_CLKIN1_IO_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKIN1_MGT"),
                        &[format!("HCLK_CMT_CLKIN1_MGT_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKIN2_HCLK"),
                        &[format!("HCLK_CMT_CLKIN2_HCLK_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKIN2_IO"),
                        &[format!("HCLK_CMT_CLKIN2_IO_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKIN2_MGT"),
                        &[format!("HCLK_CMT_CLKIN2_MGT_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKFBIN_HCLK"),
                        &[format!("HCLK_CMT_CLKFB_HCLK_{bt}")],
                    )
                    .extra_wire(
                        format!("{key}_CLKFBIN_IO"),
                        &[format!("HCLK_CMT_CLKFB_IO_{bt}")],
                    );
            }
            for i in 0..14 {
                bel = bel
                    .extra_wire(
                        format!("MMCM0_OUT{i}"),
                        &[format!("HCLK_CMT_CK_CMT_BOT{i}")],
                    )
                    .extra_wire(
                        format!("MMCM1_OUT{i}"),
                        &[format!("HCLK_CMT_CK_CMT_TOP{i}")],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(
                        format!("PERF{i}_OL_I"),
                        &[format!("HCLK_CMT_CK_PERF_OUTER_L{i}")],
                    )
                    .extra_wire(
                        format!("PERF{i}_IL_I"),
                        &[format!("HCLK_CMT_CK_PERF_INNER_L{i}")],
                    )
                    .extra_wire(
                        format!("PERF{i}_IR_I"),
                        &[format!("HCLK_CMT_CK_PERF_INNER_R{i}")],
                    )
                    .extra_wire(
                        format!("PERF{i}_OR_I"),
                        &[format!("HCLK_CMT_CK_PERF_OUTER_R{i}")],
                    )
                    .extra_wire(
                        format!("PERF{i}_OL_O"),
                        &[format!("HCLK_CMT_CK_PERF_OUTER_L{i}_LEFT")],
                    )
                    .extra_wire(
                        format!("PERF{i}_IL_O"),
                        &[format!("HCLK_CMT_CK_PERF_INNER_L{i}_LEFT")],
                    )
                    .extra_wire(
                        format!("PERF{i}_IR_O"),
                        &[format!("HCLK_CMT_CK_PERF_INNER_R{i}_RIGHT")],
                    )
                    .extra_wire(
                        format!("PERF{i}_OR_O"),
                        &[format!("HCLK_CMT_CK_PERF_OUTER_R{i}_RIGHT")],
                    );
            }
            bels.push(bel);
            let mut xn = builder
                .xtile_id(tcls::CMT, naming, xy_bot)
                .num_cells(40)
                .raw_tile(xy_top)
                .raw_tile(xy);
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

    for tkn in ["CMT_PMVA", "CMT_PMVA_BELOW"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming("INTF");
            let bel = builder.bel_xy(defs::bslots::PMVIOB_CLK, "PMVIOB", 0, 0);
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

    for tkn in ["CMT_PMVB_BUF_BELOW", "CMT_PMVB_BUF_ABOVE"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::GCLK_BUF);
            for i in 0..32 {
                bel = bel
                    .extra_wire(format!("GCLK{i}_I"), &[format!("CMT_PMVB_CK_GCLK{i}_IN")])
                    .extra_wire(format!("GCLK{i}_O"), &[format!("CMT_PMVB_CK_GCLK{i}_OUT")]);
            }
            for i in 0..8 {
                bel = bel
                    .extra_wire(
                        format!("GIO{i}_I"),
                        &[format!("CMT_PMVB_CK_IO_TO_CMT{i}_IN")],
                    )
                    .extra_wire(
                        format!("GIO{i}_O"),
                        &[format!("CMT_PMVB_CK_IO_TO_CMT{i}_OUT")],
                    );
            }
            builder
                .xtile_id(tcls::GCLK_BUF, "GCLK_BUF", xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    for (tcid, naming, tkn) in [
        (tcls::CMT_BUFG_S, "CMT_BUFG_S", "CMT_BUFG_BOT"),
        (tcls::CMT_BUFG_N, "CMT_BUFG_N", "CMT_BUFG_TOP"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming("INTF");
            let mut bels = vec![];
            let is_s = tcid == tcls::CMT_BUFG_S;
            let bi = if is_s { 0 } else { 16 };
            let int_xy = xy.delta(-2, if is_s { -1 } else { 0 });
            let cmt_xy = xy.delta(0, if is_s { -9 } else { 11 });
            for i in 0..16 {
                let ii = bi + i;
                bels.push(
                    builder
                        .bel_xy(defs::bslots::BUFGCTRL[ii], "BUFGCTRL", 0, i)
                        .pins_name_only(&["I0", "I1", "O"])
                        .extra_int_in(
                            "I0_CKINT",
                            &[[
                                "CMT_BUFG_BORROWED_IMUX38",
                                "CMT_BUFG_BORROWED_IMUX25",
                                "CMT_BUFG_BORROWED_IMUX22",
                                "CMT_BUFG_BORROWED_IMUX9",
                                "CMT_BUFG_BORROWED_IMUX6",
                                "CMT_BUFG_IMUX_B1_0",
                                "CMT_BUFG_IMUX_B25_0",
                                "CMT_BUFG_IMUX_B35_0",
                                "CMT_BUFG_IMUX_B12_0",
                                "CMT_BUFG_IMUX_B38_0",
                                "CMT_BUFG_IMUX_B23_0",
                                "CMT_BUFG_IMUX_B33_1",
                                "CMT_BUFG_IMUX_B10_1",
                                "CMT_BUFG_IMUX_B20_1",
                                "CMT_BUFG_IMUX_B5_1",
                                "CMT_BUFG_IMUX_B31_1",
                                "CMT_BUFG_IMUX_B8_0",
                                "CMT_BUFG_IMUX_B18_0",
                                "CMT_BUFG_IMUX_B42_0",
                                "CMT_BUFG_IMUX_B13_0",
                                "CMT_BUFG_IMUX_B37_0",
                                "CMT_BUFG_IMUX_B16_1",
                                "CMT_BUFG_IMUX_B40_1",
                                "CMT_BUFG_IMUX_B3_1",
                                "CMT_BUFG_IMUX_B27_1",
                                "CMT_BUFG_IMUX_B6_1",
                                "CMT_BUFG_IMUX_B30_1",
                                "CMT_BUFG_BORROWED_IMUX6",
                                "CMT_BUFG_BORROWED_IMUX9",
                                "CMT_BUFG_BORROWED_IMUX22",
                                "CMT_BUFG_BORROWED_IMUX25",
                                "CMT_BUFG_BORROWED_IMUX38",
                            ][ii]],
                        )
                        .extra_int_in(
                            "I1_CKINT",
                            &[[
                                "CMT_BUFG_BORROWED_IMUX39",
                                "CMT_BUFG_BORROWED_IMUX24",
                                "CMT_BUFG_BORROWED_IMUX23",
                                "CMT_BUFG_BORROWED_IMUX8",
                                "CMT_BUFG_BORROWED_IMUX7",
                                "CMT_BUFG_IMUX_B9_0",
                                "CMT_BUFG_IMUX_B17_0",
                                "CMT_BUFG_IMUX_B43_0",
                                "CMT_BUFG_IMUX_B4_0",
                                "CMT_BUFG_IMUX_B7_0",
                                "CMT_BUFG_IMUX_B15_0",
                                "CMT_BUFG_IMUX_B41_1",
                                "CMT_BUFG_IMUX_B2_1",
                                "CMT_BUFG_IMUX_B28_1",
                                "CMT_BUFG_IMUX_B36_1",
                                "CMT_BUFG_IMUX_B39_1",
                                "CMT_BUFG_IMUX_B0_0",
                                "CMT_BUFG_IMUX_B26_0",
                                "CMT_BUFG_IMUX_B34_0",
                                "CMT_BUFG_IMUX_B21_0",
                                "CMT_BUFG_IMUX_B29_0",
                                "CMT_BUFG_IMUX_B24_1",
                                "CMT_BUFG_IMUX_B32_1",
                                "CMT_BUFG_IMUX_B11_1",
                                "CMT_BUFG_IMUX_B19_1",
                                "CMT_BUFG_IMUX_B14_1",
                                "CMT_BUFG_IMUX_B22_1",
                                "CMT_BUFG_BORROWED_IMUX7",
                                "CMT_BUFG_BORROWED_IMUX8",
                                "CMT_BUFG_BORROWED_IMUX23",
                                "CMT_BUFG_BORROWED_IMUX24",
                                "CMT_BUFG_BORROWED_IMUX39",
                            ][ii]],
                        )
                        .extra_wire("GCLK", &[format!("CMT_BUFG_CK_GCLK{ii}")])
                        .extra_wire("FB", &[format!("CMT_BUFG_FBG_OUT{i}")])
                        .extra_wire(
                            "I0_CASCI",
                            &[
                                format!("CMT_BUFG_BOT_CK_MUXED{iii}", iii = i * 2),
                                format!("CMT_BUFG_TOP_CK_MUXED{iii}", iii = i * 2),
                            ],
                        )
                        .extra_wire(
                            "I1_CASCI",
                            &[
                                format!("CMT_BUFG_BOT_CK_MUXED{iii}", iii = i * 2 + 1),
                                format!("CMT_BUFG_TOP_CK_MUXED{iii}", iii = i * 2 + 1),
                            ],
                        )
                        .extra_int_in("I0_FB_TEST", &[format!("CMT_BUFG_CK_FB_TEST0_{i}")])
                        .extra_int_in("I1_FB_TEST", &[format!("CMT_BUFG_CK_FB_TEST1_{i}")]),
                );
            }
            let mut bel = builder.bel_virtual(if is_s {
                defs::bslots::GIO_S
            } else {
                defs::bslots::GIO_N
            });
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
            builder
                .xtile_id(tcid, naming, xy)
                .raw_tile(cmt_xy)
                .num_cells(3)
                .ref_int(int_xy, 0)
                .ref_int(int_xy.delta(0, 1), 1)
                .ref_int(int_xy.delta(0, 2), 2)
                .ref_single(int_xy.delta(1, 0), 0, intf)
                .ref_single(int_xy.delta(1, 1), 1, intf)
                .ref_single(int_xy.delta(1, 2), 2, intf)
                .bels(bels)
                .extract();
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
                .bel_virtual(defs::bslots::HCLK_GTX)
                .extra_wire("PERFCLK", &["HCLK_GTX_PERFCLK", "HCLK_GTX_LEFT_PERFCLK"])
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
            for i in 0..10 {
                bel_hclk_gtx = bel_hclk_gtx.extra_wire(
                    format!("MGT{i}"),
                    &[format!("HCLK_GTX_MGT{i}"), format!("HCLK_GTX_LEFT_MGT{i}")],
                );
            }
            for i in 0..4 {
                bel_hclk_gtx = bel_hclk_gtx.extra_wire(
                    format!("PERF{i}"),
                    &[
                        format!("HCLK_GTX_PERF_OUTER{i}"),
                        format!("HCLK_GTX_LEFT_PERF_OUTER{i}"),
                    ],
                );
            }
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
                        .bel_xy(defs::bslots::IPAD_RXP[i], "IPAD", 0, 1)
                        .raw_tile(i + 1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(defs::bslots::IPAD_RXN[i], "IPAD", 0, 0)
                        .raw_tile(i + 1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(defs::bslots::OPAD_TXP[i], "OPAD", 0, 1)
                        .raw_tile(i + 1)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_xy(defs::bslots::OPAD_TXN[i], "OPAD", 0, 0)
                        .raw_tile(i + 1)
                        .pins_name_only(&["I"]),
                ]);
            }
            bels.extend([
                builder
                    .bel_xy(defs::bslots::IPAD_CLKP[0], "IPAD", 0, 2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_CLKN[0], "IPAD", 0, 3)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_CLKP[1], "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_CLKN[1], "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
            ]);
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(defs::bslots::GTX[i], "GTXE1", 0, 0)
                        .raw_tile(i + 1)
                        .pins_name_only(&[
                            "RXP",
                            "RXN",
                            "TXP",
                            "TXN",
                            "PERFCLKRX",
                            "PERFCLKTX",
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
                        .pin_name_only("TXOUTCLK", 1)
                        .pin_name_only("RXRECCLK", 1)
                        .extra_wire("PERFCLK", &["GTX_PERFCLK", "GTX_LEFT_PERCLK"])
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
                    .bel_xy(defs::bslots::BUFDS[0], "IBUFDS_GTXE1", 0, 0)
                    .pins_name_only(&["O", "ODIV2", "I", "IB", "CLKTESTSIG"])
                    .extra_wire(
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
                    .bel_xy(defs::bslots::BUFDS[1], "IBUFDS_GTXE1", 0, 1)
                    .pins_name_only(&["O", "ODIV2", "I", "IB", "CLKTESTSIG"])
                    .extra_wire(
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
                        .bel_xy(defs::bslots::IPAD_RXP[i], "IPAD", 0, (3 - i) * 2 + 1)
                        .raw_tile(1)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(defs::bslots::IPAD_RXN[i], "IPAD", 0, (3 - i) * 2)
                        .raw_tile(1)
                        .pins_name_only(&["O"]),
                ]);
            }
            for i in 0..4 {
                bels.extend([
                    builder
                        .bel_xy(defs::bslots::OPAD_TXP[i], "OPAD", 0, (3 - i) * 2 + 1)
                        .raw_tile(1)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_xy(defs::bslots::OPAD_TXN[i], "OPAD", 0, (3 - i) * 2)
                        .raw_tile(1)
                        .pins_name_only(&["I"]),
                ]);
            }
            bels.extend([
                builder
                    .bel_xy(defs::bslots::IPAD_CLKP[0], "IPAD", 0, 1)
                    .raw_tile(2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(defs::bslots::IPAD_CLKN[0], "IPAD", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["O"]),
            ]);
            let mut bel_gt = builder
                .bel_xy(defs::bslots::GTH_QUAD, "GTHE1_QUAD", 0, 0)
                .raw_tile(1)
                .pins_name_only(&["TSTPATH", "TSTREFCLKOUT"])
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
                    format!("TXUSERCLKOUT{i}"),
                    format!("RXUSERCLKOUT{i}"),
                ]);
            }
            for i in 0..10 {
                bel_gt = bel_gt.extra_wire(
                    format!("MGT{i}"),
                    &[
                        format!("GTH_LEFT_MGTCLK{i}"),
                        format!("GTHE1_RIGHT_MGTCLK{i}"),
                    ],
                );
            }
            bels.push(bel_gt);
            bels.push(
                builder
                    .bel_xy(defs::bslots::BUFDS[0], "IBUFDS_GTHE1", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["I", "IB"])
                    .pin_name_only("O", 1),
            );
            let mut bel = builder.bel_virtual(defs::bslots::HCLK_GTH).raw_tile(2);
            for i in 0..10 {
                bel = bel
                    .extra_wire(
                        format!("MGT{i}"),
                        &[
                            format!("HCLK_GTH_LEFT_MGT{i}"),
                            format!("HCLK_GTH_RIGHT_MGT{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("MGT{i}_I"),
                        &[
                            format!("HCLK_GTH_LEFT_MGTCLK{i}"),
                            format!("HCLK_GTH_RIGHT_MGTCLK{i}"),
                        ],
                    );
            }
            bels.push(bel);

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

    for (tkn, naming) in [
        ("HCLK_CLBLM_MGT_LEFT", "HCLK_MGT_BUF_W"),
        ("HCLK_CLBLM_MGT", "HCLK_MGT_BUF_E"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(defs::bslots::HCLK_MGT_BUF);
            for i in 0..10 {
                if naming == "HCLK_MGT_BUF_W" {
                    bel = bel
                        .extra_wire(format!("MGT{i}_O"), &[format!("HCLK_CLB_MGT_CK_IN_MGT{i}")])
                        .extra_wire(
                            format!("MGT{i}_I"),
                            &[format!("HCLK_CLB_MGT_CK_OUT_MGT{i}")],
                        );
                } else {
                    bel = bel
                        .extra_wire(
                            format!("MGT{i}_O"),
                            &[format!("HCLK_CLB_MGT_CK_OUT_MGT{i}")],
                        )
                        .extra_wire(format!("MGT{i}_I"), &[format!("HCLK_CLB_MGT_CK_IN_MGT{i}")]);
                }
            }
            builder
                .xtile_id(tcls::HCLK_MGT_BUF, naming, xy)
                .num_cells(0)
                .bel(bel)
                .extract();
        }
    }

    builder.build()
}
