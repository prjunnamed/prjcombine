use prjcombine_interconnect::{
    db::{ConnectorClass, ConnectorWire, IntDb, WireKind},
    dir::Dir,
};
use prjcombine_re_xilinx_rawdump::Part;

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};
use prjcombine_xc2000::{bels::xc5200 as bels, tslots};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(rd);

    builder.db.init_slots(tslots::SLOTS, bels::SLOTS);

    builder.wire(
        "GND",
        WireKind::Tie0,
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
        builder.wire(
            format!("CLB.M{i}"),
            WireKind::MultiOut,
            &[format!("WIRE_M{i}_CLB")],
        );
        builder.permabuf(format!("CLB.M{i}.BUF"), &[format!("WIRE_BUF{i}_CLB")]);
    }
    for i in 0..16 {
        builder.wire(
            format!("IO.M{i}"),
            WireKind::MultiOut,
            &[
                format!("WIRE_M{i}_LEFT"),
                format!("WIRE_M{i}_RIGHT"),
                format!("WIRE_M{i}_BOT"),
                format!("WIRE_M{i}_TOP"),
            ],
        );
        builder.permabuf(
            format!("IO.M{i}.BUF"),
            &[
                format!("WIRE_BUF{i}_LEFT"),
                format!("WIRE_BUF{i}_RIGHT"),
                format!("WIRE_BUF{i}_BOT"),
                format!("WIRE_BUF{i}_TOP"),
            ],
        );
    }

    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w = builder.wire(
            format!("SINGLE.E{i}"),
            WireKind::MultiOut,
            &[format!("WIRE_E{i}_CLB"), format!("WIRE_E{i}_LEFT")],
        );
        builder.multi_branch(
            w,
            Dir::E,
            format!("SINGLE.W{i}"),
            &[format!("WIRE_W{i}_CLB"), format!("WIRE_W{i}_RIGHT")],
        );
    }
    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w = builder.wire(
            format!("SINGLE.S{i}"),
            WireKind::MultiOut,
            &[format!("WIRE_S{i}_CLB"), format!("WIRE_S{i}_TOP")],
        );
        builder.multi_branch(
            w,
            Dir::S,
            format!("SINGLE.N{i}"),
            &[format!("WIRE_N{i}_CLB"), format!("WIRE_N{i}_BOT")],
        );
    }

    let mut term_ll = Vec::new();
    let mut term_lr = Vec::new();
    let mut term_ul = Vec::new();
    let mut term_ur = Vec::new();
    for i in 0..8 {
        let w_be = builder.wire(
            format!("IO.SINGLE.B.E{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
            &[format!("WIRE_E{i}_BOT")],
        );
        let w_bw = builder.multi_branch(
            w_be,
            Dir::E,
            format!("IO.SINGLE.B.W{i}"),
            &[format!("WIRE_W{i}_BOT"), format!("WIRE_NW{i}_BR")],
        );
        let w_rn = builder.wire(
            format!("IO.SINGLE.R.N{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
            &[format!("WIRE_N{i}_RIGHT")],
        );
        let w_rs = builder.multi_branch(
            w_rn,
            Dir::N,
            format!("IO.SINGLE.R.S{i}"),
            &[format!("WIRE_S{i}_RIGHT"), format!("WIRE_WS{i}_TR")],
        );
        let w_tw = builder.wire(
            format!("IO.SINGLE.T.W{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::E]),
            &[format!("WIRE_W{i}_TOP")],
        );
        let w_te = builder.multi_branch(
            w_tw,
            Dir::W,
            format!("IO.SINGLE.T.E{i}"),
            &[format!("WIRE_E{i}_TOP"), format!("WIRE_SE{i}_TL")],
        );
        let w_ls = builder.wire(
            format!("IO.SINGLE.L.S{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::N]),
            &[format!("WIRE_S{i}_LEFT")],
        );
        let w_ln = builder.multi_branch(
            w_ls,
            Dir::S,
            format!("IO.SINGLE.L.N{i}"),
            &[format!("WIRE_N{i}_LEFT"), format!("WIRE_EN{i}_BL")],
        );
        term_ll.push((w_be, w_ln));
        term_lr.push((w_rn, w_bw));
        term_ul.push((w_ls, w_te));
        term_ur.push((w_tw, w_rs));
    }

    for (name, dir, wires) in [
        ("CNR.LL", Dir::W, term_ll),
        ("CNR.LR", Dir::S, term_lr),
        ("CNR.UL", Dir::N, term_ul),
        ("CNR.UR", Dir::E, term_ur),
    ] {
        let term = ConnectorClass {
            slot: builder.term_slots[dir],
            wires: wires
                .into_iter()
                .map(|(a, b)| (a, ConnectorWire::Reflect(b)))
                .collect(),
        };
        builder.db.conn_classes.insert_new(name.to_string(), term);
    }

    for i in [0, 6] {
        let w = builder.wire(
            format!("DBL.H{i}.M"),
            WireKind::MultiOut,
            &[
                format!("WIRE_DH{i}_CLB"),
                format!("WIRE_DH{i}_LEFT"),
                format!("WIRE_DH{i}_RIGHT"),
            ],
        );
        builder.multi_branch(
            w,
            Dir::W,
            format!("DBL.H{i}.W"),
            &[format!("WIRE_DE{i}_CLB"), format!("WIRE_DE{i}_LEFT")],
        );
        builder.multi_branch(
            w,
            Dir::E,
            format!("DBL.H{i}.E"),
            &[format!("WIRE_DW{i}_CLB"), format!("WIRE_DW{i}_RIGHT")],
        );
    }
    for i in [0, 6] {
        let w = builder.wire(
            format!("DBL.V{i}.M"),
            WireKind::MultiOut,
            &[
                format!("WIRE_DV{i}_CLB"),
                format!("WIRE_DV{i}_BOT"),
                format!("WIRE_DV{i}_TOP"),
            ],
        );
        builder.multi_branch(
            w,
            Dir::S,
            format!("DBL.V{i}.S"),
            &[format!("WIRE_DN{i}_CLB"), format!("WIRE_DN{i}_BOT")],
        );
        builder.multi_branch(
            w,
            Dir::N,
            format!("DBL.V{i}.N"),
            &[format!("WIRE_DS{i}_CLB"), format!("WIRE_DS{i}_TOP")],
        );
    }

    for i in 0..8 {
        let w = builder.wire(
            format!("LONG.H{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::W]),
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
        builder.conn_branch(w, Dir::E, w);
    }
    for i in 0..8 {
        let w = builder.wire(
            format!("LONG.V{i}"),
            WireKind::MultiBranch(builder.term_slots[Dir::S]),
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
        builder.conn_branch(w, Dir::N, w);
    }

    let w = builder.wire(
        "GLOBAL.L",
        WireKind::Branch(builder.term_slots[Dir::W]),
        &["WIRE_GH0_CLB", "WIRE_GH0_LEFT"],
    );
    builder.conn_branch(w, Dir::E, w);
    let w = builder.wire(
        "GLOBAL.R",
        WireKind::Branch(builder.term_slots[Dir::E]),
        &["WIRE_GH1_CLB", "WIRE_GH1_RIGHT"],
    );
    builder.conn_branch(w, Dir::W, w);
    let w = builder.wire(
        "GLOBAL.B",
        WireKind::Branch(builder.term_slots[Dir::S]),
        &["WIRE_GV0_CLB", "WIRE_GV0_BOT"],
    );
    builder.conn_branch(w, Dir::N, w);
    let w = builder.wire(
        "GLOBAL.T",
        WireKind::Branch(builder.term_slots[Dir::N]),
        &["WIRE_GV1_CLB", "WIRE_GV1_TOP"],
    );
    builder.conn_branch(w, Dir::S, w);

    let w = builder.wire(
        "GLOBAL.TL",
        WireKind::Branch(builder.term_slots[Dir::W]),
        &["WIRE_GTL_TOP", "WIRE_GTL_TL"],
    );
    builder.conn_branch(w, Dir::E, w);
    let w = builder.wire(
        "GLOBAL.BR",
        WireKind::Branch(builder.term_slots[Dir::E]),
        &["WIRE_GBR_BOT", "WIRE_GBR_BR"],
    );
    builder.conn_branch(w, Dir::W, w);
    let w = builder.wire(
        "GLOBAL.BL",
        WireKind::Branch(builder.term_slots[Dir::S]),
        &["WIRE_GBL_LEFT", "WIRE_GBL_BL"],
    );
    builder.conn_branch(w, Dir::N, w);
    let w = builder.wire(
        "GLOBAL.TR",
        WireKind::Branch(builder.term_slots[Dir::N]),
        &["WIRE_GTR_RIGHT", "WIRE_GTR_TR"],
    );
    builder.conn_branch(w, Dir::S, w);

    for i in 0..8 {
        // only 4 of these outside CLB
        builder.mux_out(
            format!("OMUX{i}"),
            &[
                format!("WIRE_OMUX{i}_CLB"),
                format!("WIRE_QIN{i}_LEFT"),
                format!("WIRE_QIN{i}_RIGHT"),
                format!("WIRE_QIN{i}_BOT"),
                format!("WIRE_QIN{i}_TOP"),
            ],
        );
        let w = builder.permabuf(
            format!("OMUX{i}.BUF"),
            &[
                format!("WIRE_Q{i}_CLB"),
                format!("WIRE_Q{i}_LEFT"),
                format!("WIRE_Q{i}_RIGHT"),
                format!("WIRE_Q{i}_BOT"),
                format!("WIRE_Q{i}_TOP"),
            ],
        );
        if i < 4 {
            builder.branch(
                w,
                Dir::W,
                format!("OMUX{i}.BUF.W"),
                &[format!("WIRE_QE{i}_CLB"), format!("WIRE_QE{i}_LEFT")],
            );
            builder.branch(
                w,
                Dir::E,
                format!("OMUX{i}.BUF.E"),
                &[format!("WIRE_QW{i}_CLB"), format!("WIRE_QW{i}_RIGHT")],
            );
            builder.branch(
                w,
                Dir::S,
                format!("OMUX{i}.BUF.S"),
                &[format!("WIRE_QN{i}_CLB"), format!("WIRE_QN{i}_BOT")],
            );
            builder.branch(
                w,
                Dir::N,
                format!("OMUX{i}.BUF.N"),
                &[format!("WIRE_QS{i}_CLB"), format!("WIRE_QS{i}_TOP")],
            );
        }
    }

    for i in 0..4 {
        for pin in ["X", "Q", "DO"] {
            builder.logic_out(
                format!("OUT.LC{i}.{pin}"),
                &[format!("WIRE_LC{i}_{pin}_CLB")],
            );
        }
    }
    for i in 0..4 {
        builder.logic_out(
            format!("OUT.TBUF{i}"),
            &[
                format!("WIRE_TQ{i}_CLB"),
                format!("WIRE_TQ{i}_LEFT"),
                format!("WIRE_TQ{i}_RIGHT"),
                format!("WIRE_TQ{i}_BOT"),
                format!("WIRE_TQ{i}_TOP"),
            ],
        );
    }
    builder.logic_out("OUT.PWRGND", &["WIRE_PWRGND_CLB"]);
    for i in 0..4 {
        builder.logic_out(
            format!("OUT.IO{i}.I"),
            &[
                format!("WIRE_PIN_IO{i}_I_LEFT"),
                format!("WIRE_PIN_IO{i}_I_RIGHT"),
                format!("WIRE_PIN_IO{i}_I_BOT"),
                format!("WIRE_PIN_IO{i}_I_TOP"),
            ],
        );
    }
    builder.logic_out(
        "OUT.CLKIOB",
        &[
            "WIRE_PIN_CLKIOB_BL",
            "WIRE_PIN_CLKIOB_BR",
            "WIRE_PIN_CLKIOB_TL",
            "WIRE_PIN_CLKIOB_TR",
        ],
    );
    builder.logic_out("OUT.RDBK.RIP", &["WIRE_PIN_RIP_BL"]);
    builder.logic_out("OUT.RDBK.DATA", &["WIRE_PIN_DATA_BL"]);
    builder.logic_out("OUT.STARTUP.DONEIN", &["WIRE_PIN_DONEIN_BR"]);
    builder.logic_out("OUT.STARTUP.Q1Q4", &["WIRE_PIN_Q1Q4_BR"]);
    builder.logic_out("OUT.STARTUP.Q2", &["WIRE_PIN_Q2_BR"]);
    builder.logic_out("OUT.STARTUP.Q3", &["WIRE_PIN_Q3_BR"]);
    builder.logic_out("OUT.BSCAN.DRCK", &["WIRE_PIN_DRCK_TL"]);
    builder.logic_out("OUT.BSCAN.IDLE", &["WIRE_PIN_IDLE_TL"]);
    builder.logic_out("OUT.BSCAN.RESET", &["WIRE_PIN_RESET_TL"]);
    builder.logic_out("OUT.BSCAN.SEL1", &["WIRE_PIN_SEL1_TL"]);
    builder.logic_out("OUT.BSCAN.SEL2", &["WIRE_PIN_SEL2_TL"]);
    builder.logic_out("OUT.BSCAN.SHIFT", &["WIRE_PIN_SHIFT_TL"]);
    builder.logic_out("OUT.BSCAN.UPDATE", &["WIRE_PIN_UPDATE_TL"]);
    builder.logic_out("OUT.BSUPD", &["WIRE_PIN_BSUPD_TR"]);
    builder.logic_out("OUT.OSC.OSC1", &["WIRE_PIN_OSC1_TR"]);
    builder.logic_out("OUT.OSC.OSC2", &["WIRE_PIN_OSC2_TR"]);
    builder.logic_out("OUT.TOP.COUT", &["WIRE_COUT_TOP"]);

    for i in 0..4 {
        for pin in ["F1", "F2", "F3", "F4", "DI"] {
            builder.mux_out(
                format!("IMUX.LC{i}.{pin}"),
                &[format!("WIRE_PIN_LC{i}_{pin}_CLB")],
            );
        }
    }
    for pin in ["CE", "CLK", "RST"] {
        builder.mux_out(format!("IMUX.CLB.{pin}"), &[format!("WIRE_{pin}_CLB")]);
    }
    builder.mux_out(
        "IMUX.TS",
        &[
            "WIRE_TS_CLB",
            "WIRE_TS_LEFT",
            "WIRE_TS_RIGHT",
            "WIRE_TS_BOT",
            "WIRE_TS_TOP",
        ],
    );
    builder.mux_out(
        "IMUX.GIN",
        &[
            "WIRE_GIN_LEFT",
            "WIRE_GIN_RIGHT",
            "WIRE_GIN_BOT",
            "WIRE_GIN_TOP",
        ],
    );
    for i in 0..4 {
        for pin in ["T", "O"] {
            builder.mux_out(
                format!("IMUX.IO{i}.{pin}"),
                &[
                    format!("WIRE_PIN_IO{i}_{pin}_LEFT"),
                    format!("WIRE_PIN_IO{i}_{pin}_RIGHT"),
                    format!("WIRE_PIN_IO{i}_{pin}_BOT"),
                    format!("WIRE_PIN_IO{i}_{pin}_TOP"),
                ],
            );
        }
    }
    builder.mux_out("IMUX.RDBK.RCLK", &["WIRE_PIN_RCLK_BL"]);
    builder.mux_out("IMUX.RDBK.TRIG", &["WIRE_PIN_TRIG_BL"]);
    builder.mux_out("IMUX.STARTUP.SCLK", &["WIRE_PIN_SCLK_BR"]);
    builder.mux_out("IMUX.STARTUP.GRST", &["WIRE_PIN_GRST_BR"]);
    builder.mux_out("IMUX.STARTUP.GTS", &["WIRE_PIN_GTS_BR"]);
    builder.mux_out("IMUX.BSCAN.TDO1", &["WIRE_PIN_TDO1_TL"]);
    builder.mux_out("IMUX.BSCAN.TDO2", &["WIRE_PIN_TDO2_TL"]);
    builder.mux_out("IMUX.OSC.OCLK", &["WIRE_PIN_OCLK_TR"]);
    builder.mux_out("IMUX.BYPOSC.PUMP", &["WIRE_PIN_PUMP_TR"]);
    builder.mux_out(
        "IMUX.BUFG",
        &[
            "WIRE_PIN_BUFGIN_BL",
            "WIRE_PIN_BUFGIN_BR",
            "WIRE_PIN_BUFGIN_TL",
            "WIRE_PIN_BUFGIN_TR",
        ],
    );
    let bot_cin = builder.mux_out("IMUX.BOT.CIN", &["WIRE_COUT_BOT"]);

    builder.extract_main_passes();

    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "CENTER",
        "CLB",
        "CLB",
        &[
            builder
                .bel_indexed(bels::LC0, "CLB", 0)
                .pins_name_only(&["CO", "F5I"])
                .pin_name_only("CI", 1),
            builder
                .bel_indexed(bels::LC1, "CLB", 1)
                .pins_name_only(&["CI", "CO"]),
            builder
                .bel_indexed(bels::LC2, "CLB", 2)
                .pins_name_only(&["CI", "CO", "F5I"]),
            builder
                .bel_indexed(bels::LC3, "CLB", 3)
                .pins_name_only(&["CI"])
                .pin_name_only("CO", 1),
            builder.bel_indexed(bels::TBUF0, "TBUF", 0),
            builder.bel_indexed(bels::TBUF1, "TBUF", 1),
            builder.bel_indexed(bels::TBUF2, "TBUF", 2),
            builder.bel_indexed(bels::TBUF3, "TBUF", 3),
            builder.bel_single(bels::VCC_GND, "VCC_GND"),
        ],
    );
    let bels_io = [
        builder
            .bel_indexed(bels::IO0, "IOB", 0)
            .pins_name_only(&["CLKIN"]),
        builder
            .bel_indexed(bels::IO1, "IOB", 1)
            .pins_name_only(&["CLKIN"]),
        builder
            .bel_indexed(bels::IO2, "IOB", 2)
            .pins_name_only(&["CLKIN"]),
        builder
            .bel_indexed(bels::IO3, "IOB", 3)
            .pins_name_only(&["CLKIN"]),
        builder.bel_indexed(bels::TBUF0, "TBUF", 0),
        builder.bel_indexed(bels::TBUF1, "TBUF", 1),
        builder.bel_indexed(bels::TBUF2, "TBUF", 2),
        builder.bel_indexed(bels::TBUF3, "TBUF", 3),
        builder
            .bel_virtual(bels::BUFR)
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
    let mut bels_io_b = bels_io.to_vec();
    bels_io_b.push(
        builder
            .bel_virtual(bels::CIN)
            .extra_int_in("IN", &["WIRE_COUT_BOT"]),
    );
    let mut bels_io_t = bels_io.to_vec();
    bels_io_t.push(
        builder
            .bel_virtual(bels::COUT)
            .extra_int_out("OUT", &["WIRE_COUT_TOP"]),
    );
    builder.extract_node(tslots::MAIN, bels::INT, "LEFT", "IO.L", "IO.L", &bels_io);
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "LEFTCLK",
        "IO.L",
        "IO.L.CLK",
        &bels_io,
    );
    builder.extract_node(tslots::MAIN, bels::INT, "RIGHT", "IO.R", "IO.R", &bels_io);
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "RIGHTCLK",
        "IO.R",
        "IO.R.CLK",
        &bels_io,
    );
    builder.extract_node(tslots::MAIN, bels::INT, "BOT", "IO.B", "IO.B", &bels_io_b);
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "BOTCLK",
        "IO.B",
        "IO.B.CLK",
        &bels_io_b,
    );
    builder.extract_node(tslots::MAIN, bels::INT, "TOP", "IO.T", "IO.T", &bels_io_t);
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "TOPCLK",
        "IO.T",
        "IO.T.CLK",
        &bels_io_t,
    );
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "LL",
        "CNR.BL",
        "CNR.BL",
        &[
            builder.bel_single(bels::BUFG, "BUFG_BL"),
            builder
                .bel_virtual(bels::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_BL"]),
            builder.bel_single(bels::RDBK, "RDBK"),
        ],
    );
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "LR",
        "CNR.BR",
        "CNR.BR",
        &[
            builder.bel_single(bels::BUFG, "BUFG_BR"),
            builder
                .bel_virtual(bels::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_BR"]),
            builder.bel_single(bels::STARTUP, "STARTUP"),
        ],
    );
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "UL",
        "CNR.TL",
        "CNR.TL",
        &[
            builder.bel_single(bels::BUFG, "BUFG_TL"),
            builder
                .bel_virtual(bels::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_TL"]),
            builder.bel_single(bels::BSCAN, "BSCAN"),
        ],
    );
    builder.extract_node(
        tslots::MAIN,
        bels::INT,
        "UR",
        "CNR.TR",
        "CNR.TR",
        &[
            builder.bel_single(bels::BUFG, "BUFG_TR"),
            builder
                .bel_virtual(bels::CLKIOB)
                .extra_int_out("OUT", &["WIRE_PIN_CLKIOB_TR"]),
            builder.bel_single(bels::OSC, "OSC"),
            builder.bel_single(bels::BYPOSC, "BYPOSC"),
            builder.bel_single(bels::BSUPD, "BSUPD"),
        ],
    );

    let node_bot = builder.db.tile_classes.get_mut("IO.B").unwrap().0;
    let pips = builder.pips.get_mut(&(node_bot, bels::INT)).unwrap();
    pips.pips.retain(|&(_, wf), _| wf.wire != bot_cin);

    for tkn in ["CLKV", "CLKB", "CLKT"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::W, false).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::E, false).unwrap();
            builder.extract_pass_tile(
                "LLH.W",
                Dir::W,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                None,
                Some((tslots::EXTRA_COL, bels::LLH, tkn, tkn)),
                int_fwd_xy,
                &[],
            );
            builder.extract_pass_tile(
                "LLH.E",
                Dir::E,
                int_fwd_xy,
                Some(xy),
                None,
                None,
                None,
                None,
                int_bwd_xy,
                &[],
            );
        }
    }

    for tkn in ["CLKH", "CLKL", "CLKR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::N, false).unwrap();
            builder.extract_pass_tile(
                "LLV.S",
                Dir::S,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                None,
                Some((tslots::EXTRA_ROW, bels::LLV, tkn, tkn)),
                int_fwd_xy,
                &[],
            );
            builder.extract_pass_tile(
                "LLV.N",
                Dir::N,
                int_fwd_xy,
                Some(xy),
                None,
                None,
                None,
                None,
                int_bwd_xy,
                &[],
            );
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
