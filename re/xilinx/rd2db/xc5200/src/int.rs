use prjcombine_interconnect::{
    db::IntDb,
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_rawdump::Part;

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};
use prjcombine_xc2000::xc5200::{self as defs, bslots, ccls, tcls, wires};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => ccls::PASS_W,
        Dir::E => ccls::PASS_E,
        Dir::S => ccls::PASS_S,
        Dir::N => ccls::PASS_N,
    }));

    builder.wire_names(
        wires::TIE_0,
        &[
            "WIRE_PIN_GND_LEFT",
            "WIRE_PIN_GND_RIGHT",
            "WIRE_PIN_GND_BOT",
            "WIRE_PIN_GND_TOP",
            "WIRE_PIN_GND_BL",
            "WIRE_PIN_GND_BR",
            "WIRE_PIN_GNDSRC_TL",
            "WIRE_PIN_GND_SRC_TR",
        ],
    );

    for i in 0..24 {
        builder.wire_names(wires::CLB_M[i], &[format!("WIRE_M{i}_CLB")]);
        builder.wire_names(wires::CLB_M_BUF[i], &[format!("WIRE_BUF{i}_CLB")]);
        builder.mark_permabuf(wires::CLB_M_BUF[i]);
    }
    for i in 0..16 {
        builder.wire_names(
            wires::IO_M[i],
            &[
                format!("WIRE_M{i}_LEFT"),
                format!("WIRE_M{i}_RIGHT"),
                format!("WIRE_M{i}_BOT"),
                format!("WIRE_M{i}_TOP"),
            ],
        );
        builder.wire_names(
            wires::IO_M_BUF[i],
            &[
                format!("WIRE_BUF{i}_LEFT"),
                format!("WIRE_BUF{i}_RIGHT"),
                format!("WIRE_BUF{i}_BOT"),
                format!("WIRE_BUF{i}_TOP"),
            ],
        );
        builder.mark_permabuf(wires::IO_M_BUF[i]);
    }

    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        builder.wire_names(
            wires::SINGLE_E[i],
            &[format!("WIRE_E{i}_CLB"), format!("WIRE_E{i}_LEFT")],
        );
        builder.wire_names(
            wires::SINGLE_W[i],
            &[format!("WIRE_W{i}_CLB"), format!("WIRE_W{i}_RIGHT")],
        );
        builder.wire_names(
            wires::SINGLE_S[i],
            &[format!("WIRE_S{i}_CLB"), format!("WIRE_S{i}_TOP")],
        );
        builder.wire_names(
            wires::SINGLE_N[i],
            &[format!("WIRE_N{i}_CLB"), format!("WIRE_N{i}_BOT")],
        );
    }

    for i in 0..8 {
        builder.wire_names(wires::SINGLE_IO_S_E[i], &[format!("WIRE_E{i}_BOT")]);
        builder.wire_names(
            wires::SINGLE_IO_S_W[i],
            &[format!("WIRE_W{i}_BOT"), format!("WIRE_NW{i}_BR")],
        );
        builder.wire_names(wires::SINGLE_IO_E_N[i], &[format!("WIRE_N{i}_RIGHT")]);
        builder.wire_names(
            wires::SINGLE_IO_E_S[i],
            &[format!("WIRE_S{i}_RIGHT"), format!("WIRE_WS{i}_TR")],
        );
        builder.wire_names(wires::SINGLE_IO_N_W[i], &[format!("WIRE_W{i}_TOP")]);
        builder.wire_names(
            wires::SINGLE_IO_N_E[i],
            &[format!("WIRE_E{i}_TOP"), format!("WIRE_SE{i}_TL")],
        );
        builder.wire_names(wires::SINGLE_IO_W_S[i], &[format!("WIRE_S{i}_LEFT")]);
        builder.wire_names(
            wires::SINGLE_IO_W_N[i],
            &[format!("WIRE_N{i}_LEFT"), format!("WIRE_EN{i}_BL")],
        );
    }

    for i in 0..2 {
        let ii = i * 6;
        builder.wire_names(
            wires::DBL_H_M[i],
            &[
                format!("WIRE_DH{ii}_CLB"),
                format!("WIRE_DH{ii}_LEFT"),
                format!("WIRE_DH{ii}_RIGHT"),
            ],
        );
        builder.wire_names(
            wires::DBL_H_W[i],
            &[format!("WIRE_DE{ii}_CLB"), format!("WIRE_DE{ii}_LEFT")],
        );
        builder.wire_names(
            wires::DBL_H_E[i],
            &[format!("WIRE_DW{ii}_CLB"), format!("WIRE_DW{ii}_RIGHT")],
        );
    }
    for i in 0..2 {
        let ii = i * 6;
        builder.wire_names(
            wires::DBL_V_M[i],
            &[
                format!("WIRE_DV{ii}_CLB"),
                format!("WIRE_DV{ii}_BOT"),
                format!("WIRE_DV{ii}_TOP"),
            ],
        );
        builder.wire_names(
            wires::DBL_V_S[i],
            &[format!("WIRE_DN{ii}_CLB"), format!("WIRE_DN{ii}_BOT")],
        );
        builder.wire_names(
            wires::DBL_V_N[i],
            &[format!("WIRE_DS{ii}_CLB"), format!("WIRE_DS{ii}_TOP")],
        );
    }

    for i in 0..8 {
        builder.wire_names(
            wires::LONG_H[i],
            &[
                format!("WIRE_LH{i}_CLB"),
                format!("WIRE_LH{i}_LEFT"),
                format!("WIRE_LH{i}_RIGHT"),
                format!("WIRE_LH{i}_BOT"),
                format!("WIRE_LH{i}_TOP"),
                format!("WIRE_LH{i}_BL"),
                format!("WIRE_LH{i}_BR"),
                format!("WIRE_LH{i}_TL"),
                format!("WIRE_LH{i}_TR"),
            ],
        );
        builder.allow_mux_to(wires::LONG_H[i]);
    }
    for i in 0..8 {
        builder.wire_names(
            wires::LONG_V[i],
            &[
                format!("WIRE_LV{i}_CLB"),
                format!("WIRE_LV{i}_LEFT"),
                format!("WIRE_LV{i}_RIGHT"),
                format!("WIRE_LV{i}_BOT"),
                format!("WIRE_LV{i}_TOP"),
                format!("WIRE_LV{i}_BL"),
                format!("WIRE_LV{i}_BR"),
                format!("WIRE_LV{i}_TL"),
                format!("WIRE_LV{i}_TR"),
            ],
        );
        builder.allow_mux_to(wires::LONG_V[i]);
    }

    builder.wire_names(wires::GCLK_W, &["WIRE_GH0_CLB", "WIRE_GH0_LEFT"]);
    builder.wire_names(wires::GCLK_E, &["WIRE_GH1_CLB", "WIRE_GH1_RIGHT"]);
    builder.wire_names(wires::GCLK_S, &["WIRE_GV0_CLB", "WIRE_GV0_BOT"]);
    builder.wire_names(wires::GCLK_N, &["WIRE_GV1_CLB", "WIRE_GV1_TOP"]);

    builder.wire_names(wires::GCLK_NW, &["WIRE_GTL_TOP", "WIRE_GTL_TL"]);
    builder.wire_names(wires::GCLK_SE, &["WIRE_GBR_BOT", "WIRE_GBR_BR"]);
    builder.wire_names(wires::GCLK_SW, &["WIRE_GBL_LEFT", "WIRE_GBL_BL"]);
    builder.wire_names(wires::GCLK_NE, &["WIRE_GTR_RIGHT", "WIRE_GTR_TR"]);

    for i in 0..8 {
        // only 4 of these outside CLB
        builder.wire_names(
            wires::OMUX[i],
            &[
                format!("WIRE_OMUX{i}_CLB"),
                format!("WIRE_QIN{i}_LEFT"),
                format!("WIRE_QIN{i}_RIGHT"),
                format!("WIRE_QIN{i}_BOT"),
                format!("WIRE_QIN{i}_TOP"),
            ],
        );
        builder.wire_names(
            wires::OMUX_BUF[i],
            &[
                format!("WIRE_Q{i}_CLB"),
                format!("WIRE_Q{i}_LEFT"),
                format!("WIRE_Q{i}_RIGHT"),
                format!("WIRE_Q{i}_BOT"),
                format!("WIRE_Q{i}_TOP"),
            ],
        );
        builder.mark_permabuf(wires::OMUX_BUF[i]);
        if i < 4 {
            builder.wire_names(
                wires::OMUX_BUF_W[i],
                &[format!("WIRE_QE{i}_CLB"), format!("WIRE_QE{i}_LEFT")],
            );
            builder.wire_names(
                wires::OMUX_BUF_E[i],
                &[format!("WIRE_QW{i}_CLB"), format!("WIRE_QW{i}_RIGHT")],
            );
            builder.wire_names(
                wires::OMUX_BUF_S[i],
                &[format!("WIRE_QN{i}_CLB"), format!("WIRE_QN{i}_BOT")],
            );
            builder.wire_names(
                wires::OMUX_BUF_N[i],
                &[format!("WIRE_QS{i}_CLB"), format!("WIRE_QS{i}_TOP")],
            );
        }
    }

    for i in 0..4 {
        builder.wire_names(wires::OUT_LC_X[i], &[format!("WIRE_LC{i}_X_CLB")]);
        builder.wire_names(wires::OUT_LC_Q[i], &[format!("WIRE_LC{i}_Q_CLB")]);
        builder.wire_names(wires::OUT_LC_DO[i], &[format!("WIRE_LC{i}_DO_CLB")]);
    }
    for i in 0..4 {
        builder.wire_names(
            wires::OUT_TBUF[i],
            &[
                format!("WIRE_TQ{i}_CLB"),
                format!("WIRE_TQ{i}_LEFT"),
                format!("WIRE_TQ{i}_RIGHT"),
                format!("WIRE_TQ{i}_BOT"),
                format!("WIRE_TQ{i}_TOP"),
            ],
        );
    }
    builder.wire_names(wires::OUT_PWRGND, &["WIRE_PWRGND_CLB"]);
    for i in 0..4 {
        builder.wire_names(
            wires::OUT_IO_I[i],
            &[
                format!("WIRE_PIN_IO{i}_I_LEFT"),
                format!("WIRE_PIN_IO{i}_I_RIGHT"),
                format!("WIRE_PIN_IO{i}_I_BOT"),
                format!("WIRE_PIN_IO{i}_I_TOP"),
            ],
        );
    }
    builder.wire_names(
        wires::OUT_CLKIOB,
        &[
            "WIRE_PIN_CLKIOB_BL",
            "WIRE_PIN_CLKIOB_BR",
            "WIRE_PIN_CLKIOB_TL",
            "WIRE_PIN_CLKIOB_TR",
        ],
    );
    builder.wire_names(wires::OUT_RDBK_RIP, &["WIRE_PIN_RIP_BL"]);
    builder.wire_names(wires::OUT_RDBK_DATA, &["WIRE_PIN_DATA_BL"]);
    builder.wire_names(wires::OUT_STARTUP_DONEIN, &["WIRE_PIN_DONEIN_BR"]);
    builder.wire_names(wires::OUT_STARTUP_Q1Q4, &["WIRE_PIN_Q1Q4_BR"]);
    builder.wire_names(wires::OUT_STARTUP_Q2, &["WIRE_PIN_Q2_BR"]);
    builder.wire_names(wires::OUT_STARTUP_Q3, &["WIRE_PIN_Q3_BR"]);
    builder.wire_names(wires::OUT_BSCAN_DRCK, &["WIRE_PIN_DRCK_TL"]);
    builder.wire_names(wires::OUT_BSCAN_IDLE, &["WIRE_PIN_IDLE_TL"]);
    builder.wire_names(wires::OUT_BSCAN_RESET, &["WIRE_PIN_RESET_TL"]);
    builder.wire_names(wires::OUT_BSCAN_SEL1, &["WIRE_PIN_SEL1_TL"]);
    builder.wire_names(wires::OUT_BSCAN_SEL2, &["WIRE_PIN_SEL2_TL"]);
    builder.wire_names(wires::OUT_BSCAN_SHIFT, &["WIRE_PIN_SHIFT_TL"]);
    builder.wire_names(wires::OUT_BSCAN_UPDATE, &["WIRE_PIN_UPDATE_TL"]);
    builder.wire_names(wires::OUT_BSUPD, &["WIRE_PIN_BSUPD_TR"]);
    builder.wire_names(wires::OUT_OSC_OSC1, &["WIRE_PIN_OSC1_TR"]);
    builder.wire_names(wires::OUT_OSC_OSC2, &["WIRE_PIN_OSC2_TR"]);
    builder.wire_names(wires::OUT_TOP_COUT, &["WIRE_COUT_TOP"]);

    for i in 0..4 {
        builder.wire_names(wires::IMUX_LC_F1[i], &[format!("WIRE_PIN_LC{i}_F1_CLB")]);
        builder.wire_names(wires::IMUX_LC_F2[i], &[format!("WIRE_PIN_LC{i}_F2_CLB")]);
        builder.wire_names(wires::IMUX_LC_F3[i], &[format!("WIRE_PIN_LC{i}_F3_CLB")]);
        builder.wire_names(wires::IMUX_LC_F4[i], &[format!("WIRE_PIN_LC{i}_F4_CLB")]);
        builder.wire_names(wires::IMUX_LC_DI[i], &[format!("WIRE_PIN_LC{i}_DI_CLB")]);
    }
    builder.wire_names(wires::IMUX_CLB_CE, &["WIRE_CE_CLB"]);
    builder.wire_names(wires::IMUX_CLB_CLK, &["WIRE_CLK_CLB"]);
    builder.wire_names(wires::IMUX_CLB_RST, &["WIRE_RST_CLB"]);
    builder.wire_names(
        wires::IMUX_TS,
        &[
            "WIRE_TS_CLB",
            "WIRE_TS_LEFT",
            "WIRE_TS_RIGHT",
            "WIRE_TS_BOT",
            "WIRE_TS_TOP",
        ],
    );
    builder.wire_names(
        wires::IMUX_GIN,
        &[
            "WIRE_GIN_LEFT",
            "WIRE_GIN_RIGHT",
            "WIRE_GIN_BOT",
            "WIRE_GIN_TOP",
        ],
    );
    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_IO_O[i],
            &[
                format!("WIRE_PIN_IO{i}_O_LEFT"),
                format!("WIRE_PIN_IO{i}_O_RIGHT"),
                format!("WIRE_PIN_IO{i}_O_BOT"),
                format!("WIRE_PIN_IO{i}_O_TOP"),
            ],
        );
        builder.wire_names(
            wires::IMUX_IO_T[i],
            &[
                format!("WIRE_PIN_IO{i}_T_LEFT"),
                format!("WIRE_PIN_IO{i}_T_RIGHT"),
                format!("WIRE_PIN_IO{i}_T_BOT"),
                format!("WIRE_PIN_IO{i}_T_TOP"),
            ],
        );
    }
    builder.wire_names(wires::IMUX_RDBK_RCLK, &["WIRE_PIN_RCLK_BL"]);
    builder.wire_names(wires::IMUX_RDBK_TRIG, &["WIRE_PIN_TRIG_BL"]);
    builder.wire_names(wires::IMUX_STARTUP_SCLK, &["WIRE_PIN_SCLK_BR"]);
    builder.wire_names(wires::IMUX_STARTUP_GRST, &["WIRE_PIN_GRST_BR"]);
    builder.wire_names(wires::IMUX_STARTUP_GTS, &["WIRE_PIN_GTS_BR"]);
    builder.wire_names(wires::IMUX_BSCAN_TDO1, &["WIRE_PIN_TDO1_TL"]);
    builder.wire_names(wires::IMUX_BSCAN_TDO2, &["WIRE_PIN_TDO2_TL"]);
    builder.wire_names(wires::IMUX_OSC_OCLK, &["WIRE_PIN_OCLK_TR"]);
    builder.wire_names(wires::IMUX_BYPOSC_PUMP, &["WIRE_PIN_PUMP_TR"]);
    builder.wire_names(
        wires::IMUX_BUFG,
        &[
            "WIRE_PIN_BUFGIN_BL",
            "WIRE_PIN_BUFGIN_BR",
            "WIRE_PIN_BUFGIN_TL",
            "WIRE_PIN_BUFGIN_TR",
        ],
    );
    builder.wire_names(wires::IMUX_BOT_CIN, &["WIRE_COUT_BOT"]);

    builder.extract_int_id(
        tcls::CLB,
        bslots::INT,
        "CENTER",
        "CLB",
        &[
            builder
                .bel_indexed(bslots::LC[0], "CLB", 0)
                .pins_name_only(&["CO", "F5I"])
                .pin_name_only("CI", 1),
            builder
                .bel_indexed(bslots::LC[1], "CLB", 1)
                .pins_name_only(&["CI", "CO"]),
            builder
                .bel_indexed(bslots::LC[2], "CLB", 2)
                .pins_name_only(&["CI", "CO", "F5I"]),
            builder
                .bel_indexed(bslots::LC[3], "CLB", 3)
                .pins_name_only(&["CI"])
                .pin_name_only("CO", 1),
            builder.bel_indexed(bslots::TBUF[0], "TBUF", 0),
            builder.bel_indexed(bslots::TBUF[1], "TBUF", 1),
            builder.bel_indexed(bslots::TBUF[2], "TBUF", 2),
            builder.bel_indexed(bslots::TBUF[3], "TBUF", 3),
            builder.bel_single(bslots::VCC_GND, "VCC_GND"),
        ],
    );
    let bels_io = [
        builder
            .bel_indexed(bslots::IO[0], "IOB", 0)
            .pins_name_only(&["CLKIN"]),
        builder
            .bel_indexed(bslots::IO[1], "IOB", 1)
            .pins_name_only(&["CLKIN"]),
        builder
            .bel_indexed(bslots::IO[2], "IOB", 2)
            .pins_name_only(&["CLKIN"]),
        builder
            .bel_indexed(bslots::IO[3], "IOB", 3)
            .pins_name_only(&["CLKIN"]),
        builder.bel_indexed(bslots::TBUF[0], "TBUF", 0),
        builder.bel_indexed(bslots::TBUF[1], "TBUF", 1),
        builder.bel_indexed(bslots::TBUF[2], "TBUF", 2),
        builder.bel_indexed(bslots::TBUF[3], "TBUF", 3),
        builder
            .bel_virtual(bslots::BUFR)
            .extra_int_in(
                "IN",
                &[
                    "WIRE_GIN_LEFT",
                    "WIRE_GIN_RIGHT",
                    "WIRE_GIN_BOT",
                    "WIRE_GIN_TOP",
                ],
            )
            .extra_int_out(
                "OUT",
                &[
                    "WIRE_GH0_LEFT",
                    "WIRE_GH1_RIGHT",
                    "WIRE_GV0_BOT",
                    "WIRE_GV1_TOP",
                ],
            ),
    ];
    let mut bels_io_s = bels_io.to_vec();
    bels_io_s.push(
        builder
            .bel_virtual(bslots::CIN)
            .extra_int_in("IN", &["WIRE_COUT_BOT"]),
    );
    bels_io_s.push(builder.bel_virtual(bslots::SCANTEST));
    let mut bels_io_n = bels_io.to_vec();
    bels_io_n.push(
        builder
            .bel_virtual(bslots::COUT)
            .extra_int_out("OUT", &["WIRE_COUT_TOP"]),
    );
    builder.extract_int_id(tcls::IO_W, bslots::INT, "LEFT", "IO_W", &bels_io);
    builder.extract_int_id(tcls::IO_W, bslots::INT, "LEFTCLK", "IO_W_CLK", &bels_io);
    builder.extract_int_id(tcls::IO_E, bslots::INT, "RIGHT", "IO_E", &bels_io);
    builder.extract_int_id(tcls::IO_E, bslots::INT, "RIGHTCLK", "IO_E_CLK", &bels_io);
    builder.extract_int_id(tcls::IO_S, bslots::INT, "BOT", "IO_S", &bels_io_s);
    builder.extract_int_id(tcls::IO_S, bslots::INT, "BOTCLK", "IO_S_CLK", &bels_io_s);
    builder.extract_int_id(tcls::IO_N, bslots::INT, "TOP", "IO_N", &bels_io_n);
    builder.extract_int_id(tcls::IO_N, bslots::INT, "TOPCLK", "IO_N_CLK", &bels_io_n);
    builder.extract_int_id(
        tcls::CNR_SW,
        bslots::INT,
        "LL",
        "CNR_SW",
        &[
            builder.bel_single(bslots::BUFG, "BUFG_BL"),
            builder
                .bel_virtual(bslots::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_BL"]),
            builder.bel_single(bslots::RDBK, "RDBK"),
        ],
    );
    builder.extract_int_id(
        tcls::CNR_SE,
        bslots::INT,
        "LR",
        "CNR_SE",
        &[
            builder.bel_single(bslots::BUFG, "BUFG_BR"),
            builder
                .bel_virtual(bslots::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_BR"]),
            builder.bel_single(bslots::STARTUP, "STARTUP"),
        ],
    );
    builder.extract_int_id(
        tcls::CNR_NW,
        bslots::INT,
        "UL",
        "CNR_NW",
        &[
            builder.bel_single(bslots::BUFG, "BUFG_TL"),
            builder
                .bel_virtual(bslots::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_TL"]),
            builder.bel_single(bslots::BSCAN, "BSCAN"),
        ],
    );
    builder.extract_int_id(
        tcls::CNR_NE,
        bslots::INT,
        "UR",
        "CNR_NE",
        &[
            builder.bel_single(bslots::BUFG, "BUFG_TR"),
            builder
                .bel_virtual(bslots::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_TR"]),
            builder.bel_single(bslots::OSC, "OSC"),
            builder.bel_single(bslots::BYPOSC, "BYPOSC"),
            builder.bel_single(bslots::BSUPD, "BSUPD"),
        ],
    );

    let pips = builder.pips.get_mut(&(tcls::IO_S, bslots::INT)).unwrap();
    pips.pips
        .retain(|&(_, wf), _| wf.wire != wires::IMUX_BOT_CIN);

    for (tcid, naming, tkn) in [
        (tcls::LLH, "LLH", "CLKV"),
        (tcls::LLH_S, "LLH_S", "CLKB"),
        (tcls::LLH_N, "LLH_N", "CLKT"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_w = builder.walk_to_int(xy, Dir::W, false).unwrap();
            let xy_e = builder.walk_to_int(xy, Dir::E, false).unwrap();
            builder
                .xtile_id(tcid, naming, xy)
                .ref_int(xy_w, 0)
                .ref_int(xy_e, 1)
                .extract_muxes(bslots::LLH)
                .extract();
        }
    }

    for (tcid, naming, tkn) in [
        (tcls::LLV, "LLV", "CLKH"),
        (tcls::LLV_W, "LLV_W", "CLKL"),
        (tcls::LLV_E, "LLV_E", "CLKR"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_s = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let xy_n = builder.walk_to_int(xy, Dir::N, false).unwrap();
            builder
                .xtile_id(tcid, naming, xy)
                .ref_int(xy_s, 0)
                .ref_int(xy_n, 1)
                .extract_muxes(bslots::LLV)
                .extract();
        }
    }

    for pips in builder.pips.values_mut() {
        for (&(wt, _wf), mode) in &mut pips.pips {
            let wtn = builder.db.wires.key(wt.wire);
            if !wtn.starts_with("IMUX") && !wtn.starts_with("OMUX") && *mode != PipMode::PermaBuf {
                *mode = PipMode::Pass;
            }
        }
    }

    builder.build()
}
