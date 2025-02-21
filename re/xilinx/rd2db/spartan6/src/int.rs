use prjcombine_interconnect::db::{Dir, IntDb, WireKind};
use prjcombine_re_xilinx_rawdump::Part;

use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;
use prjcombine_re_xilinx_naming::db::NamingDb;

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(rd);

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..16 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK{i}"), format!("GCLK{i}_BRK")],
        );
    }

    for (lr, dir, dend) in [
        ("L", Dir::E, Some((0, Dir::S))),
        ("R", Dir::E, Some((3, Dir::N))),
        ("L", Dir::W, Some((3, Dir::N))),
        ("R", Dir::W, Some((0, Dir::S))),
        ("L", Dir::N, Some((0, Dir::S))),
        ("R", Dir::N, None),
        ("L", Dir::S, None),
        ("R", Dir::S, Some((3, Dir::N))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("SNG.{dir}{lr}{i}.0"), &[format!("{dir}{lr}1B{i}")]);
            let e = builder.branch(
                b,
                dir,
                format!("SNG.{dir}{lr}{i}.1"),
                &[format!("{dir}{lr}1E{i}")],
            );
            if let Some((xi, dend)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("SNG.{dir}{lr}{i}.2"),
                        &[format!("{dir}{lr}1E_{dend}{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((3, Dir::N))),
        (Dir::N, Dir::N, Some((0, Dir::S))),
        (Dir::N, Dir::E, Some((0, Dir::S))),
        (Dir::N, Dir::W, Some((0, Dir::S))),
        (Dir::S, Dir::S, Some((3, Dir::N))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("DBL.{da}{db}{i}.0"), &[format!("{da}{db}2B{i}")]);
            let m = builder.branch(
                b,
                da,
                format!("DBL.{da}{db}{i}.1"),
                &[format!("{da}{db}2M{i}")],
            );
            let e = builder.branch(
                m,
                db,
                format!("DBL.{da}{db}{i}.2"),
                &[format!("{da}{db}2E{i}")],
            );
            if let Some((xi, dend)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("DBL.{da}{db}{i}.3"),
                        &[format!("{da}{db}2E_{dend}{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((0, Dir::S))),
        (Dir::N, Dir::N, None),
        (Dir::N, Dir::E, None),
        (Dir::N, Dir::W, Some((0, Dir::S))),
        (Dir::S, Dir::S, Some((3, Dir::N))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("QUAD.{da}{db}{i}.0"), &[format!("{da}{db}4B{i}")]);
            let a = builder.branch(
                b,
                da,
                format!("QUAD.{da}{db}{i}.1"),
                &[format!("{da}{db}4A{i}")],
            );
            let m = builder.branch(
                a,
                da,
                format!("QUAD.{da}{db}{i}.2"),
                &[format!("{da}{db}4M{i}")],
            );
            let c = builder.branch(
                m,
                db,
                format!("QUAD.{da}{db}{i}.3"),
                &[format!("{da}{db}4C{i}")],
            );
            let e = builder.branch(
                c,
                db,
                format!("QUAD.{da}{db}{i}.4"),
                &[format!("{da}{db}4E{i}")],
            );
            if let Some((xi, dend)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("QUAD.{da}{db}{i}.5"),
                        &[format!("{da}{db}4E_{dend}{i}")],
                    );
                }
            }
        }
    }

    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.GFAN{i}"),
            &[format!("GFAN{i}"), format!("INT_IOI_GFAN{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK{i}"), format!("INT_TERM_CLK{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[format!("SR{i}"), format!("INT_TERM_SR{i}")],
        );
    }
    for i in 0..63 {
        let w = builder.mux_out(
            format!("IMUX.LOGICIN{i}"),
            &[format!("LOGICIN_B{i}"), format!("INT_TERM_LOGICIN_B{i}")],
        );
        let dir = match i {
            20 | 36 | 44 | 62 => Dir::S,
            21 | 28 | 52 | 60 => Dir::N,
            _ => continue,
        };
        let b = builder.buf(
            w,
            format!("IMUX.LOGICIN{i}.BOUNCE"),
            &[format!("LOGICIN{i}")],
        );
        builder.branch(
            b,
            dir,
            format!("IMUX.LOGICIN{i}.BOUNCE.{dir}"),
            &[&format!("LOGICIN_{dir}{i}")],
        );
    }
    builder.mux_out("IMUX.LOGICIN63", &["FAN_B"]);

    for i in 0..24 {
        builder.logic_out(
            format!("OUT{i}"),
            &[format!("LOGICOUT{i}"), format!("INT_TERM_LOGICOUT{i}")],
        );
    }

    let regb_clk: Vec<_> = (0..2)
        .map(|i| {
            builder.mux_out(
                format!("IMUX.REGB.GCLK{i}"),
                &[format!("BUFPLL_BOT_GCLK{i}")],
            )
        })
        .collect();
    let regt_clk: Vec<_> = (0..2)
        .map(|i| {
            builder.mux_out(
                format!("IMUX.REGT.GCLK{i}"),
                &[format!("BUFPLL_TOP_GCLK{i}")],
            )
        })
        .collect();

    builder.extract_main_passes();

    builder.extract_node("INT", "INT", "INT", &[]);
    builder.extract_node("INT_BRK", "INT", "INT.BRK", &[]);
    builder.extract_node("INT_BRAM", "INT", "INT", &[]);
    builder.extract_node("INT_BRAM_BRK", "INT", "INT.BRK", &[]);
    builder.extract_node("INT_GCLK", "INT", "INT", &[]);
    builder.extract_node("INT_TERM", "INT", "INT.TERM", &[]);
    builder.extract_node("INT_TERM_BRK", "INT", "INT.TERM.BRK", &[]);
    builder.extract_node("IOI_INT", "INT.IOI", "INT.IOI", &[]);
    builder.extract_node("LIOI_INT", "INT.IOI", "INT.IOI", &[]);
    builder.extract_node("LIOI_INT_BRK", "INT.IOI", "INT.IOI.BRK", &[]);

    for tkn in [
        "CNR_TL_LTERM",
        "IOI_LTERM",
        "IOI_LTERM_LOWER_BOT",
        "IOI_LTERM_LOWER_TOP",
        "IOI_LTERM_UPPER_BOT",
        "IOI_LTERM_UPPER_TOP",
    ] {
        builder.extract_term_buf("TERM.W", Dir::W, tkn, "TERM.W", &[]);
    }
    builder.extract_term_buf("TERM.W", Dir::W, "INT_INTERFACE_LTERM", "TERM.W.INTF", &[]);

    for &term_xy in rd.tiles_by_kind_name("INT_LTERM") {
        let int_xy = builder.walk_to_int(term_xy, Dir::E, false).unwrap();
        // sigh.
        if int_xy.x == term_xy.x + 3 {
            continue;
        }
        builder.extract_term_buf_tile("TERM.W", Dir::W, term_xy, "TERM.W.INTF", int_xy, &[]);
    }
    for tkn in [
        "CNR_TL_RTERM",
        "IOI_RTERM",
        "IOI_RTERM_LOWER_BOT",
        "IOI_RTERM_LOWER_TOP",
        "IOI_RTERM_UPPER_BOT",
        "IOI_RTERM_UPPER_TOP",
    ] {
        builder.extract_term_buf("TERM.E", Dir::E, tkn, "TERM.E", &[]);
    }
    for tkn in ["INT_RTERM", "INT_INTERFACE_RTERM"] {
        builder.extract_term_buf("TERM.E", Dir::E, tkn, "TERM.E.INTF", &[]);
    }
    for tkn in [
        "CNR_BR_BTERM",
        "IOI_BTERM",
        "IOI_BTERM_BUFPLL",
        "CLB_INT_BTERM",
        "DSP_INT_BTERM",
        // NOTE: RAMB_BOT_BTERM is *not* a terminator — it's empty
    ] {
        builder.extract_term_buf("TERM.S", Dir::S, tkn, "TERM.S", &[]);
    }
    for tkn in [
        "CNR_TR_TTERM",
        "IOI_TTERM",
        "IOI_TTERM_BUFPLL",
        "DSP_INT_TTERM",
        "RAMB_TOP_TTERM",
    ] {
        builder.extract_term_buf("TERM.N", Dir::N, tkn, "TERM.N", &[]);
    }

    for (dir, tkn, naming) in [
        (Dir::E, "INT_INTERFACE", "INTF"),
        (Dir::E, "INT_INTERFACE_REGC", "INTF.REGC"),
        (Dir::W, "INT_INTERFACE_LTERM", "INTF.LTERM"),
        (Dir::E, "INT_INTERFACE_RTERM", "INTF.RTERM"),
        (Dir::E, "LL", "INTF.CNR"),
        (Dir::E, "UL", "INTF.CNR"),
        (Dir::E, "LR_LOWER", "INTF.CNR"),
        (Dir::E, "LR_UPPER", "INTF.CNR"),
        (Dir::E, "UR_LOWER", "INTF.CNR"),
        (Dir::E, "UR_UPPER", "INTF.CNR"),
    ] {
        builder.extract_intf("INTF", dir, tkn, naming, true);
    }
    builder.extract_intf("INTF.CMT", Dir::E, "INT_INTERFACE_CARRY", "INTF", true);
    for tkn in ["INT_INTERFACE_IOI", "INT_INTERFACE_IOI_DCMBOT"] {
        builder.extract_intf("INTF.CMT.IOI", Dir::E, tkn, "INTF", true);
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
        builder.extract_intf("INTF.IOI", Dir::E, tkn, "INTF.IOI", true);
    }

    for tkn in ["CLEXL", "CLEXM"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder.extract_xnode_bels(
                tkn,
                xy,
                &[],
                &[xy.delta(-1, 0)],
                tkn,
                &[
                    builder
                        .bel_xy("SLICE0", "SLICE", 0, 0)
                        .pins_name_only(&["CIN"])
                        .pin_name_only("COUT", 1),
                    builder.bel_xy("SLICE1", "SLICE", 1, 0),
                ],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAMSITE2").iter().next() {
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_node_naming("INTF");
        for dy in 0..4 {
            intf_xy.push((xy.delta(-1, dy), n));
        }
        builder.extract_xnode_bels_intf(
            "BRAM",
            xy,
            &[],
            &[],
            &intf_xy,
            "BRAM",
            &[
                builder.bel_xy("BRAM_F", "RAMB16", 0, 0),
                builder.bel_xy("BRAM_H0", "RAMB8", 0, 0),
                builder.bel_xy("BRAM_H1", "RAMB8", 0, 1),
            ],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("MACCSITE2").iter().next() {
        let mut intf_xy = Vec::new();
        let n = builder.ndb.get_node_naming("INTF");
        for dy in 0..4 {
            intf_xy.push((xy.delta(-1, dy), n));
        }
        let mut bel_dsp = builder
            .bel_xy("DSP", "DSP48", 0, 0)
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
        builder.extract_xnode_bels_intf("DSP", xy, &[], &[], &intf_xy, "DSP", &[bel_dsp]);
    }

    let intf_cnr = builder.ndb.get_node_naming("INTF.CNR");
    for (tkn, bels) in [
        (
            "LL",
            vec![
                builder.bel_xy("OCT_CAL2", "OCT_CAL", 0, 0),
                builder.bel_xy("OCT_CAL3", "OCT_CAL", 0, 1),
            ],
        ),
        (
            "UL",
            vec![
                builder.bel_xy("OCT_CAL0", "OCT_CAL", 0, 0),
                builder.bel_xy("OCT_CAL4", "OCT_CAL", 0, 1),
                builder.bel_single("PMV", "PMV"),
                builder.bel_single("DNA_PORT", "DNA_PORT"),
            ],
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder
                .xnode(tkn, tkn, xy)
                .ref_single(xy, 0, intf_cnr)
                .bels(bels)
                .extract();
        }
    }
    if let Some(&xy) = rd.tiles_by_kind_name("LR_LOWER").iter().next() {
        let bels = vec![
            builder.bel_xy("OCT_CAL1", "OCT_CAL", 0, 0),
            builder.bel_xy("ICAP", "ICAP", 0, 0),
            builder.bel_single("SPI_ACCESS", "SPI_ACCESS"),
            builder
                .bel_single("SUSPEND_SYNC", "SUSPEND_SYNC")
                .raw_tile(1),
            builder
                .bel_single("POST_CRC_INTERNAL", "POST_CRC_INTERNAL")
                .raw_tile(1),
            builder.bel_single("STARTUP", "STARTUP").raw_tile(1),
            builder.bel_single("SLAVE_SPI", "SLAVE_SPI").raw_tile(1),
        ];
        builder
            .xnode("LR", "LR", xy)
            .raw_tile(xy.delta(0, 1))
            .ref_single(xy, 0, intf_cnr)
            .ref_single(xy.delta(0, 1), 1, intf_cnr)
            .bels(bels)
            .extract();
    }
    if let Some(&xy) = rd.tiles_by_kind_name("UR_LOWER").iter().next() {
        let bels = vec![
            builder.bel_xy("OCT_CAL5", "OCT_CAL", 0, 0),
            builder.bel_xy("BSCAN0", "BSCAN", 0, 0).raw_tile(1),
            builder.bel_xy("BSCAN1", "BSCAN", 0, 1).raw_tile(1),
            builder.bel_xy("BSCAN2", "BSCAN", 0, 0),
            builder.bel_xy("BSCAN3", "BSCAN", 0, 1),
        ];
        builder
            .xnode("UR", "UR", xy)
            .raw_tile(xy.delta(0, 1))
            .ref_single(xy, 0, intf_cnr)
            .ref_single(xy.delta(0, 1), 1, intf_cnr)
            .bels(bels)
            .extract();
    }

    let intf_ioi = builder.ndb.get_node_naming("INTF.IOI");
    for (nn, tkn, naming, is_bt) in [
        ("IOI.LR", "LIOI", "LIOI", false),
        ("IOI.LR", "LIOI_BRK", "LIOI", false),
        ("IOI.LR", "RIOI", "RIOI", false),
        ("IOI.LR", "RIOI_BRK", "RIOI", false),
        ("IOI.BT", "BIOI_INNER", "BIOI_INNER", true),
        ("IOI.BT", "BIOI_OUTER", "BIOI_OUTER", true),
        ("IOI.BT", "TIOI_INNER", "TIOI_INNER", true),
        ("IOI.BT", "TIOI_OUTER", "TIOI_OUTER", true),
        ("IOI.BT", "BIOI_INNER_UNUSED", "BIOI_INNER_UNUSED", true),
        ("IOI.BT", "BIOI_OUTER_UNUSED", "BIOI_OUTER_UNUSED", true),
        ("IOI.BT", "TIOI_INNER_UNUSED", "TIOI_INNER_UNUSED", true),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let unused = tkn.contains("UNUSED");
            let mut bels = vec![];
            for i in 0..2 {
                let ms = match i {
                    0 => 'S',
                    1 => 'M',
                    _ => unreachable!(),
                };
                let mut bel = builder
                    .bel_xy(format!("ILOGIC{i}"), "ILOGIC", 0, i)
                    .pins_name_only(&[
                        "D", "DDLY", "DDLY2", "CLK0", "CLK1", "IOCE", "DFB", "CFB0", "CFB1", "OFB",
                        "TFB", "SHIFTIN", "SHIFTOUT", "SR",
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
                if !unused {
                    bel = bel
                        .extra_wire_force(
                            "CFB0_OUT",
                            if is_bt {
                                format!("{naming}_CFB_{ms}")
                            } else {
                                format!("{naming}_CFB_{ms}_ILOGIC")
                            },
                        )
                        .extra_wire_force(
                            "CFB1_OUT",
                            if is_bt {
                                format!("{naming}_CFB1_{ms}")
                            } else {
                                format!("{naming}_CFB1_{ms}_ILOGIC")
                            },
                        )
                        .extra_wire_force(
                            "DFB_OUT",
                            if is_bt {
                                format!("{naming}_DFB_{ms}")
                            } else {
                                format!("{naming}_DFB_{ms}_ILOGIC")
                            },
                        );
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
                    .bel_xy(format!("OLOGIC{i}"), "OLOGIC", 0, i)
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
                    .bel_xy(format!("IODELAY{i}"), "IODELAY", 0, i)
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
                        "DQSOUTP",
                        "DQSOUTN",
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
                if !unused && i == 1 {
                    bel = bel
                        .extra_wire_force(
                            "DQSOUTP_OUT",
                            if naming == "TIOI_OUTER" {
                                "TIOI_UPPER_OUTP".to_string()
                            } else {
                                format!("{naming}_OUTP")
                            },
                        )
                        .extra_wire_force(
                            "DQSOUTN_OUT",
                            if naming == "TIOI_OUTER" {
                                "TIOI_UPPER_OUTN".to_string()
                            } else {
                                format!("{naming}_OUTN")
                            },
                        );
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy("TIEOFF.IOI", "TIEOFF", 0, 0)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
            );
            for i in 0..2 {
                let ms = match i {
                    0 => 'S',
                    1 => 'M',
                    _ => unreachable!(),
                };
                let bel = builder
                    .bel_virtual(format!("IOICLK{i}"))
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
                .bel_virtual("IOI")
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
                    .extra_wire(
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
                    .extra_wire(
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
                    .extra_wire(
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
                    .extra_wire(
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
                .xnode(nn, tkn, xy)
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
                    .bel_indexed(format!("IOB{i}"), "IOB", idx[i])
                    .pins_name_only(&["PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN", "PCI_RDY"])
                    .pin_name_only("I", 1)
                    .pin_name_only("O", 1)
                    .pin_name_only("T", 1);
                if (tkn.ends_with("RDY") && i == 1) || (tkn.ends_with("PCI") && i == 0) {
                    bel = bel.pin_name_only("PCI_RDY", 1);
                }
                bels.push(bel);
            }
            builder
                .xnode("IOB", naming, xy)
                .num_tiles(0)
                .bels(bels)
                .extract();
        }
    }

    for tkn in ["REGH_LIOI_INT", "REGH_LIOI_INT_BOT25"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_xy("PCILOGICSE", "PCILOGIC", 0, 0)
                .pin_name_only("PCI_CE", 1)
                .pin_name_only("IRDY", 1)
                .pin_name_only("TRDY", 1);
            builder
                .xnode("PCILOGICSE", "PCILOGICSE_L", xy)
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
                .bel_xy("PCILOGICSE", "PCILOGIC", 0, 0)
                .pin_name_only("PCI_CE", 1)
                .pin_name_only("IRDY", 1)
                .pin_name_only("TRDY", 1);
            builder
                .xnode("PCILOGICSE", "PCILOGICSE_R", xy)
                .raw_tile(xy.delta(3, 0))
                .raw_tile(xy.delta(-1, 1))
                .ref_int(xy.delta(-1, 1), 0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, naming) in [
        ("IOI_BTERM_CLB", "BIOI_CLK"),
        ("IOI_BTERM_REGB", "BIOI_CLK"),
        ("IOI_TTERM_CLB", "TIOI_CLK"),
        ("IOI_TTERM_REGT", "TIOI_CLK"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder
                .bel_virtual("BTIOI_CLK")
                .extra_wire("PCI_CE_I", &["BTERM_CLB_PCICE", "TTERM_CLB_PCICE"])
                .extra_wire("PCI_CE_O", &["BTERM_CLB_PCICE_N", "TTERM_CLB_PCICE_S"]);
            for i in 0..4 {
                bel = bel
                    .extra_wire(
                        format!("IOCLK{i}_I"),
                        &[
                            format!("BTERM_CLB_CLKOUT{i}"),
                            format!("TTERM_CLB_IOCLK{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("IOCLK{i}_O"),
                        &[
                            format!("BTERM_CLB_CLKOUT{i}_N"),
                            format!("TTERM_CLB_IOCLK{i}_S"),
                        ],
                    )
                    .extra_wire(
                        format!("IOCE{i}_I"),
                        &[format!("BTERM_CLB_CEOUT{i}"), format!("TTERM_CLB_IOCE{i}")],
                    )
                    .extra_wire(
                        format!("IOCE{i}_O"),
                        &[
                            format!("BTERM_CLB_CEOUT{i}_N"),
                            format!("TTERM_CLB_IOCE{i}_S"),
                        ],
                    );
            }
            for i in 0..2 {
                bel = bel
                    .extra_wire(
                        format!("PLLCLK{i}_I"),
                        &[
                            format!("BTERM_CLB_PLLCLKOUT{i}"),
                            format!("TTERM_CLB_PLLCLK{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("PLLCLK{i}_O"),
                        &[
                            format!("BTERM_CLB_PLLCLKOUT{i}_N"),
                            format!("TTERM_CLB_PLLCLK{i}_S"),
                        ],
                    )
                    .extra_wire(
                        format!("PLLCE{i}_I"),
                        &[
                            format!("BTERM_CLB_PLLCEOUT{i}"),
                            format!("TTERM_CLB_PLLCE{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("PLLCE{i}_O"),
                        &[
                            format!("BTERM_CLB_PLLCEOUT{i}_N"),
                            format!("TTERM_CLB_PLLCE{i}_S"),
                        ],
                    );
            }
            builder
                .xnode("BTIOI_CLK", naming, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, trunk_naming, is_trunk_b, v_naming, is_v_dn) in [
        (
            "HCLK_IOIL_BOT_DN",
            "PCI_CE_TRUNK_BUF_BOT",
            true,
            "PCI_CE_V_BUF_DN",
            true,
        ),
        (
            "HCLK_IOIL_BOT_UP",
            "PCI_CE_TRUNK_BUF_BOT",
            true,
            "PCI_CE_V_BUF_UP",
            false,
        ),
        (
            "HCLK_IOIL_TOP_DN",
            "PCI_CE_TRUNK_BUF_TOP",
            false,
            "PCI_CE_V_BUF_DN",
            true,
        ),
        (
            "HCLK_IOIL_TOP_UP",
            "PCI_CE_TRUNK_BUF_TOP",
            false,
            "PCI_CE_V_BUF_UP",
            false,
        ),
        (
            "HCLK_IOIR_BOT_DN",
            "PCI_CE_TRUNK_BUF_BOT",
            true,
            "PCI_CE_V_BUF_DN",
            true,
        ),
        (
            "HCLK_IOIR_BOT_UP",
            "PCI_CE_TRUNK_BUF_BOT",
            true,
            "PCI_CE_V_BUF_UP",
            false,
        ),
        (
            "HCLK_IOIR_TOP_DN",
            "PCI_CE_TRUNK_BUF_TOP",
            false,
            "PCI_CE_V_BUF_DN",
            true,
        ),
        (
            "HCLK_IOIR_TOP_UP",
            "PCI_CE_TRUNK_BUF_TOP",
            false,
            "PCI_CE_V_BUF_UP",
            false,
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_virtual("PCI_CE_TRUNK_BUF")
                .extra_wire(
                    "PCI_CE_I",
                    &[if is_trunk_b {
                        "HCLK_PCI_CE_TRUNK_OUT"
                    } else {
                        "HCLK_PCI_CE_TRUNK_IN"
                    }],
                )
                .extra_wire(
                    "PCI_CE_O",
                    &[if is_trunk_b {
                        "HCLK_PCI_CE_TRUNK_IN"
                    } else {
                        "HCLK_PCI_CE_TRUNK_OUT"
                    }],
                );
            builder
                .xnode("PCI_CE_TRUNK_BUF", trunk_naming, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
            let bel = builder
                .bel_virtual("PCI_CE_V_BUF")
                .extra_wire(
                    "PCI_CE_I",
                    &[if is_v_dn {
                        "HCLK_PCI_CE_OUT"
                    } else {
                        "HCLK_PCI_CE_IN"
                    }],
                )
                .extra_wire(
                    "PCI_CE_O",
                    &[if is_v_dn {
                        "HCLK_PCI_CE_IN"
                    } else {
                        "HCLK_PCI_CE_OUT"
                    }],
                );
            builder
                .xnode("PCI_CE_V_BUF", v_naming, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in [
        "HCLK_IOIL_BOT_SPLIT",
        "HCLK_IOIL_TOP_SPLIT",
        "HCLK_IOIR_BOT_SPLIT",
        "HCLK_IOIR_TOP_SPLIT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_virtual("PCI_CE_SPLIT")
                .extra_wire("PCI_CE_I", &["HCLK_PCI_CE_SPLIT"])
                .extra_wire("PCI_CE_O", &["HCLK_PCI_CE_INOUT"]);
            builder
                .xnode("PCI_CE_SPLIT", "PCI_CE_SPLIT", xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, naming, lr) in [
        ("HCLK_IOIL_BOT_DN", "LRIOI_CLK.L", 'L'),
        ("HCLK_IOIL_BOT_SPLIT", "LRIOI_CLK.L", 'L'),
        ("HCLK_IOIL_BOT_UP", "LRIOI_CLK.L", 'L'),
        ("HCLK_IOIL_TOP_DN", "LRIOI_CLK.L", 'L'),
        ("HCLK_IOIL_TOP_SPLIT", "LRIOI_CLK.L", 'L'),
        ("HCLK_IOIL_TOP_UP", "LRIOI_CLK.L", 'L'),
        ("HCLK_IOIR_BOT_DN", "LRIOI_CLK.R", 'R'),
        ("HCLK_IOIR_BOT_SPLIT", "LRIOI_CLK.R", 'R'),
        ("HCLK_IOIR_BOT_UP", "LRIOI_CLK.R", 'R'),
        ("HCLK_IOIR_TOP_DN", "LRIOI_CLK.R", 'R'),
        ("HCLK_IOIR_TOP_SPLIT", "LRIOI_CLK.R", 'R'),
        ("HCLK_IOIR_TOP_UP", "LRIOI_CLK.R", 'R'),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("LRIOI_CLK");
            for i in 0..4 {
                bel = bel
                    .extra_wire_force(format!("IOCLK{i}_I"), format!("HCLK_IOIL_IOCLK{i}"))
                    .extra_wire_force(format!("IOCLK{i}_O_D"), format!("HCLK_IOIL_IOCLK{i}_DOWN"))
                    .extra_wire_force(format!("IOCLK{i}_O_U"), format!("HCLK_IOIL_IOCLK{i}_UP"))
                    .extra_wire_force(format!("IOCE{i}_I"), format!("HCLK_IOIL_IOCE{i}"))
                    .extra_wire_force(format!("IOCE{i}_O_D"), format!("HCLK_IOIL_IOCE{i}_DOWN"))
                    .extra_wire_force(format!("IOCE{i}_O_U"), format!("HCLK_IOIL_IOCE{i}_UP"));
            }
            for i in 0..2 {
                bel = bel
                    .extra_wire_force(format!("PLLCLK{i}_I"), format!("HCLK_IOIL_PLLCLK{i}"))
                    .extra_wire_force(
                        format!("PLLCLK{i}_O_D"),
                        format!("HCLK_IOIL_PLLCLK{i}_DOWN"),
                    )
                    .extra_wire_force(format!("PLLCLK{i}_O_U"), format!("HCLK_IOIL_PLLCLK{i}_UP"))
                    .extra_wire_force(format!("PLLCE{i}_I"), format!("HCLK_IOIL_PLLCE{i}"))
                    .extra_wire_force(format!("PLLCE{i}_O_D"), format!("HCLK_IOIL_PLLCE{i}_DOWN"))
                    .extra_wire_force(format!("PLLCE{i}_O_U"), format!("HCLK_IOIL_PLLCE{i}_UP"));
            }
            let mut bel_term = builder.bel_virtual("LRIOI_CLK_TERM").raw_tile(1);
            for i in 0..4 {
                if lr == 'L' {
                    bel_term = bel_term
                        .extra_wire_force(format!("IOCLK{i}_I"), format!("HCLK_IOI_LTERM_IOCLK{i}"))
                        .extra_wire_force(
                            format!("IOCLK{i}_O"),
                            format!("HCLK_IOI_LTERM_IOCLK{i}_E"),
                        )
                        .extra_wire_force(format!("IOCE{i}_I"), format!("HCLK_IOI_LTERM_IOCE{i}"))
                        .extra_wire_force(
                            format!("IOCE{i}_O"),
                            format!("HCLK_IOI_LTERM_IOCE{i}_E"),
                        );
                } else {
                    bel_term = bel_term
                        .extra_wire_force(format!("IOCLK{i}_I"), format!("HCLK_IOI_RTERM_IOCLK{i}"))
                        .extra_wire_force(
                            format!("IOCLK{i}_O"),
                            format!("HCLK_IOI_RTERM_IOCLK{ii}_W", ii = i ^ 3),
                        )
                        .extra_wire_force(format!("IOCE{i}_I"), format!("HCLK_IOI_RTERM_IOCE{i}"))
                        .extra_wire_force(
                            format!("IOCE{i}_O"),
                            format!("HCLK_IOI_RTERM_IOCE{ii}_W", ii = i ^ 3),
                        );
                }
            }
            for i in 0..2 {
                if lr == 'L' {
                    bel_term = bel_term
                        .extra_wire_force(
                            format!("PLLCLK{i}_I"),
                            format!("HCLK_IOI_LTERM_PLLCLK{i}"),
                        )
                        .extra_wire_force(
                            format!("PLLCLK{i}_O"),
                            format!("HCLK_IOI_LTERM_PLLCLK{i}_E"),
                        )
                        .extra_wire_force(format!("PLLCE{i}_I"), format!("HCLK_IOI_LTERM_PLLCE{i}"))
                        .extra_wire_force(
                            format!("PLLCE{i}_O"),
                            format!("HCLK_IOI_LTERM_PLLCE{i}_E"),
                        );
                } else {
                    bel_term = bel_term
                        .extra_wire_force(
                            format!("PLLCLK{i}_I"),
                            format!("HCLK_IOI_RTERM_PLLCLKOUT{i}"),
                        )
                        .extra_wire_force(
                            format!("PLLCLK{i}_O"),
                            format!("HCLK_IOI_RTERM_PLLCLKOUT{i}_W"),
                        )
                        .extra_wire_force(
                            format!("PLLCE{i}_I"),
                            format!("HCLK_IOI_RTERM_PLLCEOUT{i}"),
                        )
                        .extra_wire_force(
                            format!("PLLCE{i}_O"),
                            format!("HCLK_IOI_RTERM_PLLCEOUT{i}_W"),
                        );
                }
            }
            builder
                .xnode("LRIOI_CLK", naming, xy)
                .raw_tile(xy) // dummy
                .num_tiles(0)
                .bel(bel)
                .bel(bel_term)
                .extract();
        }
    }

    for (tkn, naming) in [
        ("IOI_PCI_CE_LEFT", "PCI_CE_H_BUF_CNR"),
        ("IOI_PCI_CE_RIGHT", "PCI_CE_H_BUF_CNR"),
        ("BRAM_BOT_BTERM_L", "PCI_CE_H_BUF_BRAM"),
        ("BRAM_BOT_BTERM_R", "PCI_CE_H_BUF_BRAM"),
        ("BRAM_TOP_TTERM_L", "PCI_CE_H_BUF_BRAM"),
        ("BRAM_TOP_TTERM_R", "PCI_CE_H_BUF_BRAM"),
        ("DSP_BOT_BTERM_L", "PCI_CE_H_BUF_DSP"),
        ("DSP_BOT_BTERM_R", "PCI_CE_H_BUF_DSP"),
        ("DSP_TOP_TTERM_L", "PCI_CE_H_BUF_DSP"),
        ("DSP_TOP_TTERM_R", "PCI_CE_H_BUF_DSP"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_virtual("PCI_CE_H_BUF")
                .extra_wire(
                    "PCI_CE_I",
                    &[
                        "IOI_PCICE_TB",
                        "BRAM_TTERM_PCICE_IN",
                        "MACCSITE2_TTERM_PCICE_IN",
                    ],
                )
                .extra_wire(
                    "PCI_CE_O",
                    &[
                        "IOI_PCICE_EW",
                        "BRAM_TTERM_PCICE_OUT",
                        "MACCSITE2_TTERM_PCICE_OUT",
                    ],
                );
            builder
                .xnode("PCI_CE_H_BUF", naming, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in ["MCB_L", "MCB_L_BOT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_node_naming("INTF");
            let mut bels = vec![];
            let mut bel = builder
                .bel_xy("MCB", "MCB", 0, 0)
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
                .pin_name_only("DQSIOWEN90N", 1)
                .pin_name_only("PLLCLK0", 1)
                .pin_name_only("PLLCLK1", 1)
                .pin_name_only("PLLCE0", 1)
                .pin_name_only("PLLCE1", 1);
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
            bels.push(bel);
            bels.extend([
                builder
                    .bel_xy("TIEOFF.CLK", "TIEOFF", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
                builder
                    .bel_xy("TIEOFF.DQS0", "TIEOFF", 0, 0)
                    .raw_tile(3)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
                builder
                    .bel_xy("TIEOFF.DQS1", "TIEOFF", 0, 0)
                    .raw_tile(4)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
                builder
                    .bel_virtual("MCB_TIE.CLK")
                    .raw_tile(2)
                    .extra_wire("OUTP0", &["MCB_BOT_MOUTP_GND"])
                    .extra_wire("OUTN0", &["MCB_BOT_MOUTN_VCC"])
                    .extra_wire("OUTP1", &["MCB_BOT_SOUTP_VCC"])
                    .extra_wire("OUTN1", &["MCB_BOT_SOUTN_GND"]),
                builder
                    .bel_virtual("MCB_TIE.DQS0")
                    .raw_tile(3)
                    .extra_wire("OUTP0", &["MCB_BOT_MOUTP_GND"])
                    .extra_wire("OUTN0", &["MCB_BOT_MOUTN_VCC"])
                    .extra_wire("OUTP1", &["MCB_BOT_SOUTP_VCC"])
                    .extra_wire("OUTN1", &["MCB_BOT_SOUTN_GND"]),
                builder
                    .bel_virtual("MCB_TIE.DQS1")
                    .raw_tile(4)
                    .extra_wire("OUTP0", &["MCB_BOT_MOUTP_GND"])
                    .extra_wire("OUTN0", &["MCB_BOT_MOUTN_VCC"])
                    .extra_wire("OUTP1", &["MCB_BOT_SOUTP_VCC"])
                    .extra_wire("OUTN1", &["MCB_BOT_SOUTN_GND"]),
            ]);
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
                .xnode("MCB", tkn, xy)
                .num_tiles(28)
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
            xn.bels(bels).extract();
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
            let mut bel = builder.bel_virtual("HCLK");
            for i in 0..16 {
                bel = bel
                    .extra_int_out(
                        format!("GCLK{i}_O_D"),
                        &[format!("HCLK_GCLK{i}"), format!("HCLK_GCLK{i}_FOLD")],
                    )
                    .extra_int_out(
                        format!("GCLK{i}_O_U"),
                        &[format!("HCLK_GCLK_UP{i}"), format!("HCLK_GCLK_UP{i}_FOLD")],
                    )
                    .extra_wire(
                        format!("GCLK{i}_I"),
                        &[
                            format!("HCLK_GCLK{i}_INT"),
                            format!("HCLK_GCLK{i}_INT_FOLD"),
                        ],
                    );
            }
            builder
                .xnode("HCLK", naming, xy)
                .num_tiles(2)
                .ref_int(xy.delta(0, -1), 0)
                .ref_int(xy.delta(0, 1), 1)
                .bel(bel)
                .extract();
            break;
        }
    }

    for tkn in ["DSP_HCLK_GCLK_FOLD", "GTPDUAL_DSP_FEEDTHRU"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("HCLK_H_MIDBUF");
            for i in 0..16 {
                bel = bel
                    .extra_wire(
                        format!("GCLK{i}_I"),
                        &[
                            format!("HCLK_GCLK{i}_DSP_NOFOLD"),
                            format!("GTP_DSP_FEEDTHRU_HCLK_GCLK{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("GCLK{i}_M"),
                        &[
                            format!("HCLK_MIDBUF_GCLK{i}"),
                            format!("GTP_MIDBUF_GCLK{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("GCLK{i}_O"),
                        &[
                            format!("HCLK_GCLK{i}_DSP_FOLD"),
                            format!("HCLK_GCLK{i}_GTPDSP_FOLD"),
                        ],
                    );
            }
            builder
                .xnode("HCLK_H_MIDBUF", tkn, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("REG_V_HCLK").iter().next() {
        let mut bels = vec![];
        for i in 0..2 {
            let lr = if i == 0 { 'L' } else { 'R' };
            for j in 0..16 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFH_{lr}{j}"), "BUFH", i * 3, (1 - i) * 16 + j)
                        .pin_name_only("I", 1)
                        .pin_name_only("O", 1),
                );
            }
        }
        let mut bel = builder.bel_virtual("HCLK_ROW");
        for i in 0..16 {
            bel = bel
                .extra_wire(format!("BUFG{i}"), &[format!("CLKV_GCLKH_MAIN{i}_FOLD")])
                .extra_wire(format!("CMT{i}"), &[format!("REGV_PLL_HCLK{i}")]);
        }
        bels.push(bel);
        builder
            .xnode("HCLK_ROW", "HCLK_ROW", xy)
            .num_tiles(0)
            .bels(bels)
            .extract();
    }

    for tkn in ["REG_V_HCLKBUF_BOT", "REG_V_HCLKBUF_TOP"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("HCLK_V_MIDBUF");
            for i in 0..16 {
                bel = bel
                    .extra_wire(format!("GCLK{i}_O"), &[format!("CLKV_GCLK_MAIN{i}_BUF")])
                    .extra_wire(format!("GCLK{i}_M"), &[format!("CLKV_MIDBUF_GCLK{i}")])
                    .extra_wire(format!("GCLK{i}_I"), &[format!("CLKV_GCLK_MAIN{i}")])
            }
            builder
                .xnode("HCLK_V_MIDBUF", "HCLK_V_MIDBUF", xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CLKC").iter().next() {
        let mut bels = vec![];
        for i in 0..16 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGMUX{i}"), "BUFGMUX", u8::from((i & 4) != 0), i)
                    .pin_name_only("O", 1)
                    .pins_name_only(&["I0", "I1"]),
            );
        }
        let mut bel = builder.bel_virtual("CLKC");
        for i in 0..16 {
            bel = bel
                .extra_wire(format!("MUX{i}"), &[format!("CLKC_GCLK{i}")])
                .extra_wire(format!("CKPIN_H{i}"), &[format!("CLKC_CKLR{i}")])
                .extra_wire(format!("CKPIN_V{i}"), &[format!("CLKC_CKTB{i}")])
                .extra_wire(format!("CMT_U{i}"), &[format!("CLKC_PLL_U{i}")])
                .extra_wire(format!("CMT_D{i}"), &[format!("CLKC_PLL_L{i}")]);
        }
        bels.push(bel);
        let bel = builder
            .bel_virtual("CLKC_BUFPLL")
            .raw_tile(1)
            .extra_wire("PLL0D_CLKOUT0", &["REGC_PLLCLK_DN_IN0"])
            .extra_wire("PLL0D_CLKOUT1", &["REGC_PLLCLK_DN_IN1"])
            .extra_wire("PLL1D_CLKOUT0", &["REGC_PLLCLK_DN_IN2"])
            .extra_wire("PLL1D_CLKOUT1", &["REGC_PLLCLK_DN_IN3"])
            .extra_wire("PLL0U_CLKOUT0", &["REGC_PLLCLK_UP_IN0"])
            .extra_wire("PLL0U_CLKOUT1", &["REGC_PLLCLK_UP_IN1"])
            .extra_wire("PLL1U_CLKOUT0", &["REGC_PLLCLK_UP_IN2"])
            .extra_wire("PLL1U_CLKOUT1", &["REGC_PLLCLK_UP_IN3"])
            .extra_wire("OUTD_CLKOUT0", &["REGC_PLLCLK_DN_OUT0"])
            .extra_wire("OUTD_CLKOUT1", &["REGC_PLLCLK_DN_OUT1"])
            .extra_wire("OUTU_CLKOUT0", &["REGC_PLLCLK_UP_OUT0"])
            .extra_wire("OUTU_CLKOUT1", &["REGC_PLLCLK_UP_OUT1"])
            .extra_wire("OUTL_CLKOUT0", &["REGC_CLKPLL_IO_LT0"])
            .extra_wire("OUTL_CLKOUT1", &["REGC_CLKPLL_IO_LT1"])
            .extra_wire("OUTR_CLKOUT0", &["REGC_CLKPLL_IO_RT0"])
            .extra_wire("OUTR_CLKOUT1", &["REGC_CLKPLL_IO_RT1"])
            .extra_wire("PLL0D_LOCKED", &["PLL_LOCK_BOT0"])
            .extra_wire("PLL1D_LOCKED", &["PLL_LOCK_BOT1"])
            .extra_wire("PLL0U_LOCKED", &["PLL_LOCK_TOP0"])
            .extra_wire("PLL1U_LOCKED", &["PLL_LOCK_TOP1"])
            .extra_wire("OUTD_LOCKED", &["PLL_LOCK_BOT2"])
            .extra_wire("OUTU_LOCKED", &["PLL_LOCK_TOP2"])
            .extra_wire("OUTL_LOCKED0", &["CLK_PLL_LOCK_LT0"])
            .extra_wire("OUTL_LOCKED1", &["CLK_PLL_LOCK_LT1"])
            .extra_wire("OUTR_LOCKED0", &["CLK_PLL_LOCK_RT0"])
            .extra_wire("OUTR_LOCKED1", &["CLK_PLL_LOCK_RT1"]);
        bels.push(bel);
        builder
            .xnode("CLKC", "CLKC", xy)
            .raw_tile(xy.delta(-1, 0))
            .ref_int(xy.delta(-3, 1), 0)
            .bels(bels)
            .extract();
    }

    for tkn in ["REG_V_MIDBUF_BOT", "REG_V_MIDBUF_TOP"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CKPIN_V_MIDBUF");
            for i in 0..8 {
                bel = bel
                    .extra_wire(
                        format!("CKPIN{i}_O"),
                        &[
                            format!("CLKV_CKPIN_BOT_BUF{i}"),
                            format!("CLKV_MIDBUF_TOP_CKPIN{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("CKPIN{i}_I"),
                        &[
                            format!("CLKV_CKPIN_BUF{i}"),
                            format!("CLKV_MIDBUF_BOT_CKPIN{i}"),
                        ],
                    )
            }
            builder
                .xnode("CKPIN_V_MIDBUF", tkn, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for tkn in [
        "REGH_DSP_L",
        "REGH_DSP_R",
        "REGH_CLEXL_INT_CLK",
        "REGH_CLEXM_INT_GCLKL",
        "REGH_BRAM_FEEDTHRU_L_GCLK",
        "REGH_BRAM_FEEDTHRU_R_GCLK",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CKPIN_H_MIDBUF");
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("CKPIN{i}_O"), &[format!("REGH_DSP_OUT_CKPIN{i}")])
                    .extra_wire(format!("CKPIN{i}_I"), &[format!("REGH_DSP_IN_CKPIN{i}")])
            }
            builder
                .xnode("CKPIN_H_MIDBUF", "CKPIN_H_MIDBUF", xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, e, bio2, bpll) in [
        (
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
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..8 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFIO2_{i}"), "BUFIO2", bio2[i].0, bio2[i].1)
                        .pins_name_only(&["I", "IB"])
                        .pin_name_only("DIVCLK", 1)
                        .pin_name_only("IOCLK", 1)
                        .pin_name_only("SERDESSTROBE", 1)
                        .extra_wire("CMT", &[format!("REG{e}_CLK_INDIRECT{i}")])
                        .extra_wire("CKPIN", &[format!("REG{e}_CKPIN{i}")]),
                );
            }
            for i in 0..8 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFIO2FB_{i}"), "BUFIO2FB", bio2[i].0, bio2[i].1)
                        .pins_name_only(&["I", "IB", "O"])
                        .extra_wire("CMT", &[format!("REG{e}_CLK_FEEDBACK{i}")]),
                );
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFPLL{i}"), "BUFPLL", 0, bpll[i])
                        .pins_name_only(&[
                            "PLLIN",
                            "IOCLK",
                            "SERDESSTROBE",
                            "LOCKED",
                            "LOCK",
                            "GCLK",
                        ]),
                );
            }
            bels.push(
                builder
                    .bel_xy("BUFPLL_MCB", "BUFPLL_MCB", 0, 0)
                    .pins_name_only(&[
                        "PLLIN0",
                        "PLLIN1",
                        "IOCLK0",
                        "IOCLK1",
                        "SERDESSTROBE0",
                        "SERDESSTROBE1",
                        "LOCKED",
                        "LOCK",
                        "GCLK",
                    ]),
            );

            bels.push(
                builder
                    .bel_xy("TIEOFF.REG", "TIEOFF", 0, 0)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
            );
            let mut bel = builder.bel_virtual("BUFIO2_INS");
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("CLKPIN{i}"), &[format!("REG{e}_CLKPIN{i}")])
                    .extra_wire(format!("DFB{i}"), &[format!("REG{e}_DFB{i}")])
                    .extra_wire(format!("CFB0_{i}"), &[format!("REG{e}_CFB{i}")])
                    .extra_wire(format!("CFB1_{i}"), &[format!("REG{e}_CFB1_{i}")])
                    .extra_wire(format!("GTPCLK{i}"), &[format!("REG{e}_GTPCLK{i}")])
                    .extra_wire(format!("GTPFB{i}"), &[format!("REG{e}_GTPFB{i}")]);
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("DQSP{i}"), &[format!("REG{e}_DQSP{i}")])
                    .extra_wire(format!("DQSN{i}"), &[format!("REG{e}_DQSN{i}")]);
            }
            bels.push(bel);
            let mut bel = builder.bel_virtual("BUFIO2_CKPIN").raw_tile(1);
            for i in 0..8 {
                bel = bel
                    .extra_wire(
                        format!("CKPIN{i}"),
                        &[
                            format!("REGH_LTERM_CKPIN{i}"),
                            format!("REGH_RTERM_CKPIN{i}"),
                            format!("REGB_BTERM_CKPIN{i}"),
                            format!("REGT_TTERM_CKPIN{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("CLKPIN{i}"),
                        &[
                            format!("REGH_LTERM_CLKPIN{i}"),
                            format!("REGH_RTERM_CLKPIN{i}"),
                            format!("REGB_BTERM_CLKPIN{i}"),
                            format!("REGT_TTERM_CLKPIN{i}"),
                        ],
                    );
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual("BUFPLL_BUF")
                    .raw_tile(1)
                    .extra_wire(
                        "PLLCE0_O",
                        &[
                            "REGH_LTERM_PLL_CEOUT0",
                            "REGH_RTERM_PLL_CEOUT0",
                            "REGB_BTERM_PLL_CEOUT0",
                            "REGT_TTERM_PLL_CEOUT0",
                        ],
                    )
                    .extra_wire(
                        "PLLCE1_O",
                        &[
                            "REGH_LTERM_PLL_CEOUT1",
                            "REGH_RTERM_PLL_CEOUT1",
                            "REGB_BTERM_PLL_CEOUT1",
                            "REGT_TTERM_PLL_CEOUT1",
                        ],
                    )
                    .extra_wire(
                        "PLLCE0_I",
                        &[
                            "REGH_LTERM_PLL_CEOUT0_W",
                            "REGH_RTERM_PLL_CEOUT0_E",
                            "REGB_BTERM_PLL_CEOUT0_S",
                            "REGT_TTERM_PLL_CEOUT0_N",
                        ],
                    )
                    .extra_wire(
                        "PLLCE1_I",
                        &[
                            "REGH_LTERM_PLL_CEOUT1_W",
                            "REGH_RTERM_PLL_CEOUT1_E",
                            "REGB_BTERM_PLL_CEOUT1_S",
                            "REGT_TTERM_PLL_CEOUT1_N",
                        ],
                    )
                    .extra_wire(
                        "PLLCLK0_O",
                        &[
                            "REGH_LTERM_PLL_CLKOUT0",
                            "REGH_RTERM_PLL_CLKOUT0",
                            "REGB_BTERM_PLL_CLKOUT0",
                            "REGT_TTERM_PLL_CLKOUT0",
                        ],
                    )
                    .extra_wire(
                        "PLLCLK1_O",
                        &[
                            "REGH_LTERM_PLL_CLKOUT1",
                            "REGH_RTERM_PLL_CLKOUT1",
                            "REGB_BTERM_PLL_CLKOUT1",
                            "REGT_TTERM_PLL_CLKOUT1",
                        ],
                    )
                    .extra_wire(
                        "PLLCLK0_I",
                        &[
                            "REGH_LTERM_PLL_CLKOUT0_W",
                            "REGH_RTERM_PLL_CLKOUT0_E",
                            "REGB_BTERM_PLL_CLKOUT0_S",
                            "REGT_TTERM_PLL_CLKOUT0_N",
                        ],
                    )
                    .extra_wire(
                        "PLLCLK1_I",
                        &[
                            "REGH_LTERM_PLL_CLKOUT1_W",
                            "REGH_RTERM_PLL_CLKOUT1_E",
                            "REGB_BTERM_PLL_CLKOUT1_S",
                            "REGT_TTERM_PLL_CLKOUT1_N",
                        ],
                    ),
            );
            bels.push(
                builder
                    .bel_virtual("BUFPLL_OUT")
                    .extra_wire(
                        "PLLCE0",
                        &[
                            "REGL_PLL_CEOUT0_LEFT",
                            "REGR_CEOUT0",
                            "REGB_CEOUT0",
                            "REGT_CEOUT0",
                        ],
                    )
                    .extra_wire(
                        "PLLCE1",
                        &[
                            "REGL_PLL_CEOUT1_LEFT",
                            "REGR_CEOUT1",
                            "REGB_CEOUT1",
                            "REGT_CEOUT1",
                        ],
                    )
                    .extra_wire(
                        "PLLCLK0",
                        &[
                            "REGL_PLL_CLKOUT0_LEFT",
                            "REGR_PLLCLK0",
                            "REGB_PLLCLK0",
                            "REGT_PLLCLK0",
                        ],
                    )
                    .extra_wire(
                        "PLLCLK1",
                        &[
                            "REGL_PLL_CLKOUT1_LEFT",
                            "REGR_PLLCLK1",
                            "REGB_PLLCLK1",
                            "REGT_PLLCLK1",
                        ],
                    )
                    .extra_int_out("LOCK0", &[format!("REG{e}_LOCK0")])
                    .extra_int_out("LOCK1", &[format!("REG{e}_LOCK1")]),
            );
            if matches!(e, 'L' | 'R') {
                bels.push(
                    builder
                        .bel_virtual("BUFPLL_INS_LR")
                        .extra_int_in("GCLK0", &[format!("REG{e}_GCLK0")])
                        .extra_int_in("GCLK1", &[format!("REG{e}_GCLK1")])
                        .extra_int_in("PLLIN0_GCLK", &[format!("REG{e}_GCLK2")])
                        .extra_int_in("PLLIN1_GCLK", &[format!("REG{e}_GCLK3")])
                        .extra_wire("PLLIN0_CMT", &[format!("REG{e}_CLKPLL0")])
                        .extra_wire("PLLIN1_CMT", &[format!("REG{e}_CLKPLL1")])
                        .extra_wire("LOCKED0", &[format!("REG{e}_LOCKED0")])
                        .extra_wire("LOCKED1", &[format!("REG{e}_LOCKED1")]),
                );
            } else {
                let mut bel = builder
                    .bel_virtual("BUFPLL_INS_BT")
                    .extra_int_in("GCLK0", &[format!("REG{e}_GCLK0")])
                    .extra_int_in("GCLK1", &[format!("REG{e}_GCLK1")]);
                for i in 0..6 {
                    bel = bel.extra_wire(
                        format!("PLLIN{i}"),
                        &[
                            format!("REGB_PLL_IOCLK_DOWN{i}"),
                            format!("REGT_PLL_IOCLK_UP{i}"),
                        ],
                    );
                }
                for i in 0..3 {
                    bel = bel.extra_wire(format!("LOCKED{i}"), &[format!("REG{e}_LOCKIN{i}")]);
                }
                bels.push(bel);
                let mut bel = builder
                    .bel_virtual("GTP_H_BUF")
                    .raw_tile(1)
                    .extra_wire("CLKINEAST_L", &[format!("REG{e}_{e}TERM_GTP_CLKINEAST0")])
                    .extra_wire("CLKINWEST_L", &[format!("REG{e}_{e}TERM_GTP_CLKINWEST0")])
                    .extra_wire(
                        "CLKINEAST_R",
                        &[format!("REG{e}_{e}TERM_ALTGTP_CLKINEAST0")],
                    )
                    .extra_wire(
                        "CLKINWEST_R",
                        &[format!("REG{e}_{e}TERM_ALTGTP_CLKINWEST0")],
                    )
                    .extra_wire("CLKOUT_EW_L", &[format!("REG{e}_{e}TERM_GTP_CLKOUTEW0")])
                    .extra_wire("CLKOUT_EW_R", &[format!("REG{e}_{e}TERM_ALTGTP_CLKOUTEW0")]);
                for i in 0..3 {
                    bel = bel
                        .extra_wire_force(
                            format!("RXCHBONDI{i}_L"),
                            format!("REG{e}_{e}TERM_GTP_RXCHBONDO{i}"),
                        )
                        .extra_wire_force(
                            format!("RXCHBONDO{i}_L"),
                            format!("REG{e}_{e}TERM_GTP_RXCHBONDI{i}"),
                        )
                        .extra_wire_force(
                            format!("RXCHBONDI{i}_R"),
                            format!("REG{e}_{e}TERM_ALTGTP_RXCHBONDO{i}"),
                        )
                        .extra_wire_force(
                            format!("RXCHBONDO{i}_R"),
                            format!("REG{e}_{e}TERM_ALTGTP_RXCHBONDI{i}"),
                        );
                }
                for i in 0..5 {
                    bel = bel
                        .extra_wire_force(
                            format!("RCALOUTEAST{i}_L"),
                            format!("REG{e}_{e}TERM_GTP_RCALOUTEAST{i}"),
                        )
                        .extra_wire_force(
                            format!("RCALINEAST{i}_R"),
                            format!("REG{e}_{e}TERM_ALTGTP_RCALINEAST{i}"),
                        );
                }
                bels.push(bel);
            }
            let mut xn = builder.xnode(tkn, tkn, xy);
            match tkn {
                "REG_L" => {
                    xn = xn
                        .raw_tile(xy.delta(1, 0))
                        .raw_tile_single(xy.delta(2, 1), 0)
                        .raw_tile_single(xy.delta(2, 2), 1);
                }
                "REG_R" => {
                    xn = xn
                        .raw_tile(xy.delta(-1, 0))
                        .raw_tile_single(xy.delta(-4, 1), 0)
                        .raw_tile_single(xy.delta(-4, 2), 1);
                }
                "REG_B" => {
                    xn = xn
                        .raw_tile(xy.delta(0, 1))
                        .raw_tile(xy.delta(2, 1)) // BUFPLL mux
                        .raw_tile_single(xy.delta(2, 3), 0)
                        .optin_muxes(&regb_clk);
                }
                "REG_T" => {
                    xn = xn
                        .raw_tile(xy.delta(0, -1))
                        .raw_tile(xy.delta(2, -1)) // BUFPLL mux
                        .raw_tile_single(xy.delta(2, -2), 0)
                        .optin_muxes(&regt_clk);
                }
                _ => unreachable!(),
            }
            xn.bels(bels).extract();
        }
    }

    for (tkn, naming, lr, is_top) in [
        ("IOI_LTERM_LOWER_BOT", "CLKPIN_BUF.L.BOT", 'L', false),
        ("IOI_LTERM_LOWER_TOP", "CLKPIN_BUF.L.TOP", 'L', true),
        ("IOI_LTERM_UPPER_BOT", "CLKPIN_BUF.L.BOT", 'L', false),
        ("IOI_LTERM_UPPER_TOP", "CLKPIN_BUF.L.TOP", 'L', true),
        ("IOI_RTERM_LOWER_BOT", "CLKPIN_BUF.R.BOT", 'R', false),
        ("IOI_RTERM_LOWER_TOP", "CLKPIN_BUF.R.TOP", 'R', true),
        ("IOI_RTERM_UPPER_BOT", "CLKPIN_BUF.R.BOT", 'R', false),
        ("IOI_RTERM_UPPER_TOP", "CLKPIN_BUF.R.TOP", 'R', true),
    ] {
        let ew = match lr {
            'L' => 'E',
            'R' => 'W',
            _ => unreachable!(),
        };
        let bi = if is_top {
            u8::from(lr == 'L')
        } else {
            u8::from(lr == 'R')
        };
        let bt = if is_top { "TOP" } else { "BOT" };
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_virtual("CLKPIN_BUF")
                .extra_wire(
                    "CLKPIN0_O",
                    &[format!("IOI_{lr}TERM_CLKPIN{ii}", ii = bi * 2)],
                )
                .extra_wire(
                    "CLKPIN1_O",
                    &[format!("IOI_{lr}TERM_CLKPIN{ii}", ii = bi * 2 + 1)],
                )
                .extra_wire("CLKPIN0_I", &[format!("{lr}TERM_IOB_IBUF0")])
                .extra_wire("CLKPIN1_I", &[format!("{lr}TERM_IOB_IBUF1")])
                .extra_wire(
                    "DFB0_O",
                    &[format!("IOI_{lr}TERM_{bt}_DFB{ii}", ii = bi * 2)],
                )
                .extra_wire(
                    "DFB1_O",
                    &[format!("IOI_{lr}TERM_{bt}_DFB{ii}", ii = bi * 2 + 1)],
                )
                .extra_wire(
                    "DFB0_I",
                    &[format!("IOI_{lr}TERM_{bt}_DFB{ii}_{ew}", ii = bi * 2)],
                )
                .extra_wire(
                    "DFB1_I",
                    &[format!("IOI_{lr}TERM_{bt}_DFB{ii}_{ew}", ii = bi * 2 + 1)],
                )
                .extra_wire(
                    "CFB0_0_O",
                    &[format!("IOI_{lr}TERM_{bt}_CFB{ii}", ii = bi * 2)],
                )
                .extra_wire(
                    "CFB0_1_O",
                    &[format!("IOI_{lr}TERM_{bt}_CFB{ii}", ii = bi * 2 + 1)],
                )
                .extra_wire(
                    "CFB0_0_I",
                    &[format!("IOI_{lr}TERM_{bt}_CFB{ii}_{ew}", ii = bi * 2)],
                )
                .extra_wire(
                    "CFB0_1_I",
                    &[format!("IOI_{lr}TERM_{bt}_CFB{ii}_{ew}", ii = bi * 2 + 1)],
                )
                .extra_wire(
                    "CFB1_0_O",
                    &[format!("IOI_{lr}TERM_{bt}_CFB1_{ii}", ii = bi * 2)],
                )
                .extra_wire(
                    "CFB1_1_O",
                    &[format!("IOI_{lr}TERM_{bt}_CFB1_{ii}", ii = bi * 2 + 1)],
                )
                .extra_wire(
                    "CFB1_0_I",
                    &[format!("IOI_{lr}TERM_{bt}_CFB1_{ii}_{ew}", ii = bi * 2)],
                )
                .extra_wire(
                    "CFB1_1_I",
                    &[format!("IOI_{lr}TERM_{bt}_CFB1_{ii}_{ew}", ii = bi * 2 + 1)],
                )
                .extra_wire("DQSP_O", &[format!("IOI_{lr}TERM_{bt}_DQSP{bi}")])
                .extra_wire("DQSP_I", &[format!("IOI_{lr}TERM_{bt}_DQSP{bi}_{ew}")])
                .extra_wire("DQSN_O", &[format!("IOI_{lr}TERM_{bt}_DQSN{bi}")])
                .extra_wire("DQSN_I", &[format!("IOI_{lr}TERM_{bt}_DQSN{bi}_{ew}")]);
            builder
                .xnode("CLKPIN_BUF", naming, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, naming, prefix, ibuf_prefix, bi, ns) in [
        (
            "IOI_BTERM_REGB",
            "CLKPIN_BUF.B.BOT",
            "BTERM_CLB",
            "BTERM_IOIBOT",
            2,
            'N',
        ),
        (
            "IOI_BTERM_REGB",
            "CLKPIN_BUF.B.TOP",
            "BTERM_CLB",
            "BTERM_IOIUP",
            3,
            'N',
        ),
        (
            "IOI_TTERM_REGT",
            "CLKPIN_BUF.T.BOT",
            "IOI_REGT",
            "TTERM_IOIBOT",
            1,
            'S',
        ),
        (
            "IOI_TTERM_REGT",
            "CLKPIN_BUF.T.TOP",
            "IOI_REGT",
            "TTERM_IOIUP",
            0,
            'S',
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder
                .bel_virtual("CLKPIN_BUF")
                .extra_wire("CLKPIN0_O", &[format!("{prefix}_CLKPIN{ii}", ii = bi * 2)])
                .extra_wire(
                    "CLKPIN1_O",
                    &[format!("{prefix}_CLKPIN{ii}", ii = bi * 2 + 1)],
                )
                .extra_wire("CLKPIN0_I", &[format!("{ibuf_prefix}_IBUF0")])
                .extra_wire("CLKPIN1_I", &[format!("{ibuf_prefix}_IBUF1")])
                .extra_wire(
                    "DFB0_O",
                    &[
                        format!("{prefix}_DFB{ii}", ii = bi * 2),
                        format!("{prefix}_DFB_M{ii}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "DFB1_O",
                    &[
                        format!("{prefix}_DFB{ii}", ii = bi * 2 + 1),
                        format!("{prefix}_DFB_S{ii}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "DFB0_I",
                    &[
                        format!("{prefix}_DFB{ii}_{ns}", ii = bi * 2),
                        format!("{prefix}_DFB_M{ii}_{ns}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "DFB1_I",
                    &[
                        format!("{prefix}_DFB{ii}_{ns}", ii = bi * 2 + 1),
                        format!("{prefix}_DFB_S{ii}_{ns}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB0_0_O",
                    &[
                        format!("{prefix}_CFB{ii}", ii = bi * 2),
                        format!("{prefix}_CFB_M{ii}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB0_1_O",
                    &[
                        format!("{prefix}_CFB{ii}", ii = bi * 2 + 1),
                        format!("{prefix}_CFB_S{ii}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB0_0_I",
                    &[
                        format!("{prefix}_CFB{ii}_{ns}", ii = bi * 2),
                        format!("{prefix}_CFB_M{ii}_{ns}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB0_1_I",
                    &[
                        format!("{prefix}_CFB{ii}_{ns}", ii = bi * 2 + 1),
                        format!("{prefix}_CFB_S{ii}_{ns}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB1_0_O",
                    &[
                        format!("{prefix}_CFB1_{ii}", ii = bi * 2),
                        format!("{prefix}_CFB1_M{ii}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB1_1_O",
                    &[
                        format!("{prefix}_CFB1_{ii}", ii = bi * 2 + 1),
                        format!("{prefix}_CFB1_S{ii}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB1_0_I",
                    &[
                        format!("{prefix}_CFB1_{ii}_{ns}", ii = bi * 2),
                        format!("{prefix}_CFB1_M{ii}_{ns}", ii = bi + 1),
                    ],
                )
                .extra_wire(
                    "CFB1_1_I",
                    &[
                        format!("{prefix}_CFB1_{ii}_{ns}", ii = bi * 2 + 1),
                        format!("{prefix}_CFB1_S{ii}_{ns}", ii = bi + 1),
                    ],
                )
                .extra_wire("DQSP_O", &[format!("{prefix}_DQSP{bi}")])
                .extra_wire("DQSP_I", &[format!("{prefix}_DQSP{bi}_{ns}")])
                .extra_wire("DQSN_O", &[format!("{prefix}_DQSN{bi}")])
                .extra_wire("DQSN_I", &[format!("{prefix}_DQSN{bi}_{ns}")]);
            builder
                .xnode("CLKPIN_BUF", naming, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    let intf = builder.ndb.get_node_naming("INTF");
    for (tkn, bt, bkind, d0, d1, d2) in [
        ("CMT_DCM_BOT", 'B', "DCM_BUFPLL_BUF_BOT", 'D', 'D', 'D'),
        ("CMT_DCM2_BOT", 'B', "DCM_BUFPLL_BUF_BOT_MID", 'D', 'U', 'D'),
        ("CMT_DCM_TOP", 'T', "DCM_BUFPLL_BUF_TOP", 'D', 'D', 'U'),
        ("CMT_DCM2_TOP", 'T', "DCM_BUFPLL_BUF_TOP_MID", 'U', 'D', 'U'),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let ii = 2 - i;
                let mut bel = builder
                    .bel_xy(format!("DCM{i}"), "DCM", 0, i)
                    .pins_name_only(&["CLKIN", "CLKFB", "SKEWCLKIN1", "SKEWCLKIN2"])
                    .extra_int_in("CLKFB_CKINT0", &[format!("DCM{ii}_CLK_FROM_BUFG0")])
                    .extra_int_in("CLKIN_CKINT0", &[format!("DCM{ii}_CLK_FROM_BUFG1")])
                    .extra_int_in("CLKFB_CKINT1", &[format!("DCM{ii}_SE_CLK_IN0")])
                    .extra_int_in("CLKIN_CKINT1", &[format!("DCM{ii}_SE_CLK_IN1")])
                    .extra_wire("CLKIN_TEST", &[format!("DCM{ii}_CLKIN_TOPLL")])
                    .extra_wire("CLKFB_TEST", &[format!("DCM{ii}_CLKFB_TOPLL")])
                    .extra_wire("CLK_TO_PLL", &[format!("DCM{ii}_CLK_TO_PLL")])
                    .extra_wire("CLK_FROM_PLL", &[format!("DCM{ii}_CLK_FROM_PLL")]);
                for (j, pin) in [
                    "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                    "CLKFX180", "CONCUR",
                ]
                .into_iter()
                .enumerate()
                {
                    bel = bel
                        .pin_name_only(pin, 0)
                        .extra_wire(format!("{pin}_TEST"), &[format!("DCM{ii}_{pin}_TEST")])
                        .extra_wire(format!("{pin}_OUT"), &[format!("DCM{ii}_CLKOUT{j}")]);
                }
                bels.push(bel);
            }
            let mut bel = builder.bel_virtual("CMT");
            for i in 0..16 {
                bel = bel
                    .extra_int_in(format!("HCLK{i}_CKINT"), &[format!("DCM_FABRIC_CLK{i}")])
                    .extra_wire(format!("HCLK{i}"), &[format!("DCM_HCLK{i}")])
                    .extra_wire(format!("HCLK{i}_BUF"), &[format!("DCM_HCLK{i}_N")]);
                if bt == 'B' {
                    bel = bel
                        .extra_wire(format!("CASC{i}_O"), &[format!("PLL_CLK_CASC_TOP{i}")])
                        .extra_wire(format!("CASC{i}_I"), &[format!("PLL_CLK_CASC_BOT{i}")]);
                } else {
                    bel = bel
                        .extra_wire(format!("CASC{i}_O"), &[format!("PLL_CLK_CASC_BOT{i}")])
                        .extra_wire(format!("CASC{i}_I"), &[format!("PLL_CLK_CASC_TOP{i}")]);
                }
            }
            if bt == 'B' {
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
                        )
                        .extra_wire(
                            format!("BUFIO2FB_BT{i}"),
                            &[
                                format!("DCM_CLK_FEEDBACK_TB_BOT{i}"),
                                format!("DCM2_CLK_FEEDBACK_TB_BOT{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("BUFIO2FB_LR{i}"),
                            &[
                                format!("DCM_CLK_FEEDBACK_LR_TOP{i}"),
                                format!("DCM2_CLK_FEEDBACK_LR_TOP{i}"),
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
                        )
                        .extra_wire(
                            format!("BUFIO2FB_LR{i}"),
                            &[
                                format!("DCM_CLK_FEEDBACK_TB_BOT{i}"),
                                format!("DCM2_CLK_FEEDBACK_TB_BOT{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("BUFIO2FB_BT{i}"),
                            &[
                                format!("DCM_CLK_FEEDBACK_LR_TOP{i}"),
                                format!("DCM2_CLK_FEEDBACK_LR_TOP{i}"),
                            ],
                        );
                }
            }
            bels.push(bel);
            builder
                .xnode("CMT_DCM", tkn, xy)
                .num_tiles(2)
                .ref_single(xy.delta(-1, -2), 0, intf)
                .ref_single(xy.delta(-1, 0), 1, intf)
                .bels(bels)
                .extract();

            let mut bel = builder.bel_virtual(bkind);
            if d0 == 'D' {
                bel = bel
                    .extra_wire("PLL0_LOCKED_O", &["CMT_DCM_LOCK_DN0"])
                    .extra_wire("PLL0_LOCKED_I", &["CMT_DCM_LOCK_UP0"])
                    .extra_wire("PLL0_CLKOUT0_O", &["DCM_IOCLK_DOWN0"])
                    .extra_wire("PLL0_CLKOUT1_O", &["DCM_IOCLK_DOWN1"])
                    .extra_wire("PLL0_CLKOUT0_I", &["DCM_IOCLK_UP0"])
                    .extra_wire("PLL0_CLKOUT1_I", &["DCM_IOCLK_UP1"]);
            } else {
                bel = bel
                    .extra_wire("PLL0_LOCKED_O", &["CMT_DCM_LOCK_UP0"])
                    .extra_wire("PLL0_LOCKED_I", &["CMT_DCM_LOCK_DN0"])
                    .extra_wire("PLL0_CLKOUT0_O", &["DCM_IOCLK_UP0"])
                    .extra_wire("PLL0_CLKOUT1_O", &["DCM_IOCLK_UP1"])
                    .extra_wire("PLL0_CLKOUT0_I", &["DCM_IOCLK_DOWN0"])
                    .extra_wire("PLL0_CLKOUT1_I", &["DCM_IOCLK_DOWN1"]);
            }
            if d1 == 'D' {
                bel = bel
                    .extra_wire("PLL1_LOCKED_O", &["CMT_DCM_LOCK_DN1"])
                    .extra_wire("PLL1_LOCKED_I", &["CMT_DCM_LOCK_UP1"])
                    .extra_wire("PLL1_CLKOUT0_O", &["DCM_IOCLK_DOWN2"])
                    .extra_wire("PLL1_CLKOUT1_O", &["DCM_IOCLK_DOWN3"])
                    .extra_wire("PLL1_CLKOUT0_I", &["DCM_IOCLK_UP2"])
                    .extra_wire("PLL1_CLKOUT1_I", &["DCM_IOCLK_UP3"]);
            } else {
                bel = bel
                    .extra_wire("PLL1_LOCKED_O", &["CMT_DCM_LOCK_UP1"])
                    .extra_wire("PLL1_LOCKED_I", &["CMT_DCM_LOCK_DN1"])
                    .extra_wire("PLL1_CLKOUT0_O", &["DCM_IOCLK_UP2"])
                    .extra_wire("PLL1_CLKOUT1_O", &["DCM_IOCLK_UP3"])
                    .extra_wire("PLL1_CLKOUT0_I", &["DCM_IOCLK_DOWN2"])
                    .extra_wire("PLL1_CLKOUT1_I", &["DCM_IOCLK_DOWN3"]);
            }
            if d2 == 'D' {
                bel = bel
                    .extra_wire("CLKC_LOCKED_O", &["CMT_DCM_LOCK_DN2"])
                    .extra_wire("CLKC_LOCKED_I", &["CMT_DCM_LOCK_UP2"])
                    .extra_wire("CLKC_CLKOUT0_O", &["DCM_IOCLK_DOWN4"])
                    .extra_wire("CLKC_CLKOUT1_O", &["DCM_IOCLK_DOWN5"])
                    .extra_wire("CLKC_CLKOUT0_I", &["DCM_IOCLK_UP4"])
                    .extra_wire("CLKC_CLKOUT1_I", &["DCM_IOCLK_UP5"]);
            } else {
                bel = bel
                    .extra_wire("CLKC_LOCKED_O", &["CMT_DCM_LOCK_UP2"])
                    .extra_wire("CLKC_LOCKED_I", &["CMT_DCM_LOCK_DN2"])
                    .extra_wire("CLKC_CLKOUT0_O", &["DCM_IOCLK_UP4"])
                    .extra_wire("CLKC_CLKOUT1_O", &["DCM_IOCLK_UP5"])
                    .extra_wire("CLKC_CLKOUT0_I", &["DCM_IOCLK_DOWN4"])
                    .extra_wire("CLKC_CLKOUT1_I", &["DCM_IOCLK_DOWN5"]);
            }

            builder
                .xnode(bkind, bkind, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
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
                .bel_xy("PLL", "PLL_ADV", 0, 0)
                .pins_name_only(&[
                    "REL",
                    "SKEWCLKIN1",
                    "SKEWCLKIN2",
                    "CLKOUT0",
                    "CLKOUT1",
                    "CLKOUT2",
                    "CLKOUT3",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKFBDCM",
                    "CLKOUTDCM0",
                    "CLKOUTDCM1",
                    "CLKOUTDCM2",
                    "CLKOUTDCM3",
                    "CLKOUTDCM4",
                    "CLKOUTDCM5",
                ])
                .pin_name_only("CLKFBOUT", 1)
                .pin_name_only("CLKIN1", 1)
                .pin_name_only("CLKIN2", 1)
                .pin_name_only("CLKFBIN", 1)
                .extra_int_in("CLKIN1_CKINT0", &["CMT_CLK_FROM_BUFG0"])
                .extra_int_in("CLKIN2_CKINT0", &["CMT_CLK_FROM_BUFG1"])
                .extra_int_in("CLKIN2_CKINT1", &["CMT_SE_CLKIN0"])
                .extra_int_in("CLKFBIN_CKINT0", &["CMT_CLK_FROM_BUFG2"])
                .extra_int_in("CLKFBIN_CKINT1", &["CMT_SE_CLKIN1"])
                .extra_wire("CLK_TO_DCM0", &["CMT_CLK_TO_DCM2"])
                .extra_wire("CLK_TO_DCM1", &["CMT_CLK_TO_DCM1"])
                .extra_wire("CLK_FROM_DCM0", &["CMT_CLK_FROM_DCM2"])
                .extra_wire("CLK_FROM_DCM1", &["CMT_CLK_FROM_DCM1"])
                .extra_wire("CLKIN1_TEST", &["CMT_CLKMUX_CLKREF_TEST"])
                .extra_wire("CLKFBIN_TEST", &["CMT_CLKMUX_CLKFB_TEST"])
                .extra_wire("CLKFBDCM_TEST", &["CMT_PLL_CLKFBDCM_TEST"])
                .extra_int_out("TEST_CLK", &["CMT_TEST_CLK"])
                .extra_wire("TEST_CLK_OUT", &["CMT_SE_CLK_OUT"])
                .extra_wire("DCM0_CLKIN_TEST", &["CMT_DCM2_CLKIN"])
                .extra_wire("DCM0_CLKFB_TEST", &["CMT_DCM2_CLKFB"])
                .extra_wire("DCM1_CLKIN_TEST", &["CMT_DCM1_CLKIN"])
                .extra_wire("DCM1_CLKFB_TEST", &["CMT_DCM1_CLKFB"]);
            let bel_tie = builder
                .bel_xy("TIEOFF.PLL", "TIEOFF", 0, 0)
                .pins_name_only(&["HARD0", "HARD1", "KEEP1"]);
            let mut bel = builder.bel_virtual("CMT");
            for i in 0..16 {
                bel = bel
                    .extra_int_in(format!("HCLK{i}_CKINT"), &[format!("CMT_FABRIC_CLK{i}")])
                    .extra_wire(format!("HCLK{i}"), &[format!("CMT_PLL_HCLK{i}")])
                    .extra_wire(format!("HCLK{i}_BUF"), &[format!("CMT_PLL_HCLK{i}_E")]);
                if bt == 'B' {
                    bel = bel
                        .extra_wire(format!("CASC{i}_O"), &[format!("PLL_CLK_CASC_IN{i}")])
                        .extra_wire(format!("CASC{i}_I"), &[format!("CLK_PLLCASC_OUT{i}")]);
                } else {
                    bel = bel
                        .extra_wire(format!("CASC{i}_O"), &[format!("CLK_PLLCASC_OUT{i}")])
                        .extra_wire(format!("CASC{i}_I"), &[format!("PLL_CLK_CASC_IN{i}")]);
                }
            }
            if bt == 'B' {
                for i in 0..8 {
                    bel = bel
                        .extra_wire(
                            format!("BUFIO2_BT{i}"),
                            &[
                                format!("CMT_PLL_CLK_INDIRECT_LRBOT{i}"),
                                format!("CMT_PLL2_CLK_INDIRECT_LRBOT{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("BUFIO2_LR{i}"),
                            &[format!("PLL_CLK_INDIRECT_TB{i}")],
                        )
                        .extra_wire(
                            format!("BUFIO2FB_BT{i}"),
                            &[
                                format!("CMT_PLL_CLK_FEEDBACK_LRBOT{i}"),
                                format!("CMT_PLL2_CLK_FEEDBACK_LRBOT{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("BUFIO2FB_LR{i}"),
                            &[format!("PLL_CLK_FEEDBACK_TB{i}")],
                        );
                }
            } else {
                for i in 0..8 {
                    bel = bel
                        .extra_wire(
                            format!("BUFIO2_LR{i}"),
                            &[
                                format!("CMT_PLL_CLK_INDIRECT_LRBOT{i}"),
                                format!("CMT_PLL2_CLK_INDIRECT_LRBOT{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("BUFIO2_BT{i}"),
                            &[format!("PLL_CLK_INDIRECT_TB{i}")],
                        )
                        .extra_wire(
                            format!("BUFIO2FB_LR{i}"),
                            &[
                                format!("CMT_PLL_CLK_FEEDBACK_LRBOT{i}"),
                                format!("CMT_PLL2_CLK_FEEDBACK_LRBOT{i}"),
                            ],
                        )
                        .extra_wire(
                            format!("BUFIO2FB_BT{i}"),
                            &[format!("PLL_CLK_FEEDBACK_TB{i}")],
                        );
                }
            }
            builder
                .xnode("CMT_PLL", tkn, xy)
                .num_tiles(2)
                .ref_single(xy.delta(-1, -2), 0, intf)
                .ref_single(xy.delta(-1, 0), 1, intf)
                .bel(bel_pll)
                .bel(bel_tie)
                .bel(bel)
                .extract();
            if let Some(out) = out {
                let node = match out {
                    0 => "PLL_BUFPLL_OUT0",
                    1 => "PLL_BUFPLL_OUT1",
                    _ => unreachable!(),
                };
                let mut bel = builder
                    .bel_virtual("PLL_BUFPLL")
                    .extra_wire("CLKOUT0", &["PLLCASC_CLKOUT0"])
                    .extra_wire("CLKOUT1", &["PLLCASC_CLKOUT1"])
                    .extra_wire("LOCKED", &["CMT_PLL_LOCKED"]);
                if out == 0 {
                    bel = bel
                        .extra_wire("CLKOUT0_D", &["PLL2_IOCLK_DN0"])
                        .extra_wire("CLKOUT1_D", &["PLL2_IOCLK_DN1"])
                        .extra_wire("CLKOUT0_U", &["PLL2_IOCLK_UP0"])
                        .extra_wire("CLKOUT1_U", &["PLL2_IOCLK_UP1"])
                        .extra_wire("LOCKED_D", &["CMT_PLL2_LOCK_DN0"])
                        .extra_wire("LOCKED_U", &["CMT_PLL2_LOCK_UP0"]);
                } else {
                    bel = bel
                        .extra_wire("CLKOUT0_D", &["PLL_IOCLK_DN2"])
                        .extra_wire("CLKOUT1_D", &["PLL_IOCLK_DN3"])
                        .extra_wire("CLKOUT0_U", &["PLL_IOCLK_UP2"])
                        .extra_wire("CLKOUT1_U", &["PLL_IOCLK_UP3"])
                        .extra_wire("LOCKED_D", &["CMT_PLL_LOCK_DN1"])
                        .extra_wire("LOCKED_U", &["CMT_PLL_LOCK_UP1"]);
                }
                builder
                    .xnode(node, node, xy)
                    .num_tiles(0)
                    .bel(bel)
                    .extract();
            } else {
                builder
                    .xnode("PLL_BUFPLL_B", "PLL_BUFPLL_B", xy)
                    .num_tiles(0)
                    .extract();
                builder
                    .xnode("PLL_BUFPLL_T", "PLL_BUFPLL_T", xy)
                    .num_tiles(0)
                    .extract();
            }
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE_TOP").iter().next() {
        let mut intf_xy = Vec::new();
        let nr = builder.ndb.get_node_naming("INTF.RTERM");
        let nl = builder.ndb.get_node_naming("INTF.LTERM");
        for dy in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16] {
            intf_xy.push((xy.delta(-5, -9 + dy), nr));
        }
        for dy in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16] {
            intf_xy.push((xy.delta(2, -9 + dy), nl));
        }
        builder.extract_xnode_bels_intf(
            "PCIE",
            xy,
            &[],
            &[],
            &intf_xy,
            "PCIE",
            &[builder.bel_xy("PCIE", "PCIE", 0, 0)],
        );
    }

    for tkn in ["GTPDUAL_BOT", "GTPDUAL_TOP"] {
        let is_b = tkn == "GTPDUAL_BOT";
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf_rterm = builder.ndb.get_node_naming("INTF.RTERM");
            let intf_lterm = builder.ndb.get_node_naming("INTF.LTERM");
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
            let mut bels = vec![];
            for (i, key) in [
                (2, "IPAD.RXP0"),
                (0, "IPAD.RXN0"),
                (3, "IPAD.RXP1"),
                (1, "IPAD.RXN1"),
                (5, "IPAD.CLKP0"),
                (4, "IPAD.CLKN0"),
                (7, "IPAD.CLKP1"),
                (6, "IPAD.CLKN1"),
            ] {
                bels.push(builder.bel_xy(key, "IPAD", 0, i).pins_name_only(&["O"]));
            }
            for (i, key) in [
                (1, "OPAD.TXP0"),
                (3, "OPAD.TXN0"),
                (0, "OPAD.TXP1"),
                (2, "OPAD.TXN1"),
            ] {
                bels.push(builder.bel_xy(key, "OPAD", 0, i).pins_name_only(&["I"]));
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFDS{i}"), "BUFDS", 0, i)
                        .pins_name_only(&["I", "IB", "O"]),
                );
            }
            let mut bel = builder
                .bel_xy("GTP", "GTPA1_DUAL", 0, 0)
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
                    "PLLCLK00",
                    "PLLCLK01",
                    "PLLCLK10",
                    "PLLCLK11",
                    "REFCLKPLL0",
                    "REFCLKPLL1",
                    "CLKINEAST0",
                    "CLKINEAST1",
                    "CLKINWEST0",
                    "CLKINWEST1",
                ])
                .pin_name_only("GTPCLKOUT00", 1)
                .pin_name_only("GTPCLKOUT01", 1)
                .pin_name_only("GTPCLKOUT10", 1)
                .pin_name_only("GTPCLKOUT11", 1)
                .pin_name_only("GTPCLKFBEAST0", 1)
                .pin_name_only("GTPCLKFBEAST1", 1)
                .pin_name_only("GTPCLKFBWEST0", 1)
                .pin_name_only("GTPCLKFBWEST1", 1)
                .pin_name_only("RXCHBONDI0", 1)
                .pin_name_only("RXCHBONDI1", 1)
                .pin_name_only("RXCHBONDI2", 1)
                .pin_name_only("RXCHBONDO0", 1)
                .pin_name_only("RXCHBONDO1", 1)
                .pin_name_only("RXCHBONDO2", 1)
                .extra_wire("PLLCLK0", &["GTPDUAL_PLLCLK0", "GTPDUAL_BOT_PLLCLK0"])
                .extra_wire("PLLCLK1", &["GTPDUAL_PLLCLK1", "GTPDUAL_BOT_PLLCLK1"])
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
            bels.push(bel);
            let mut bel = builder
                .bel_virtual("GTP_BUF")
                .raw_tile(1)
                .extra_wire(
                    "PLLCLK0_O",
                    &["BRAM_BTERM_PLLCLK0_S", "BRAM_TTERM_PLLCLK0_N"],
                )
                .extra_wire(
                    "PLLCLK1_O",
                    &["BRAM_BTERM_PLLCLK1_S", "BRAM_TTERM_PLLCLK1_N"],
                )
                .extra_wire("PLLCLK0_I", &["IOI_BTERM_PLLCLKOUT0", "BRAM_TTERM_PLLCLK0"])
                .extra_wire("PLLCLK1_I", &["IOI_BTERM_PLLCLKOUT1", "BRAM_TTERM_PLLCLK1"])
                .extra_wire(
                    "CLKINEAST_O",
                    &["BRAM_BTERM_CLKOUTEAST0_S", "BRAM_TTERM_CLKOUTEAST0_N"],
                )
                .extra_wire(
                    "CLKINEAST_I",
                    &["IOI_BTERM_CLKOUTEAST0", "BRAM_TTERM_CLKOUTEAST0"],
                )
                .extra_wire(
                    "CLKINWEST_O",
                    &["BRAM_BTERM_CLKOUTWEST0_S", "BRAM_TTERM_CLKOUTWEST0_N"],
                )
                .extra_wire(
                    "CLKINWEST_I",
                    &["IOI_BTERM_CLKOUTWEST0", "BRAM_TTERM_CLKOUTWEST0"],
                )
                .extra_wire(
                    "CLKOUT_EW_O",
                    &["IOI_BTERM_CLKOUT_EW0", "BRAM_TTERM_CLKOUT_EW0"],
                )
                .extra_wire(
                    "CLKOUT_EW_I",
                    &["BRAM_BTERM_CLKOUT_EW0_S", "BRAM_TTERM_CLKOUT_EW0_N"],
                );
            for i in 0..4 {
                bel = bel
                    .extra_wire(
                        format!("GTPCLK{i}_I"),
                        &[
                            format!("BRAM_BTERM_GTPCLK{i}_S"),
                            format!("BRAM_TTERM_GTPCLK{i}_N"),
                        ],
                    )
                    .extra_wire(
                        format!("GTPCLK{i}_O"),
                        &[
                            format!("IOI_BTERM_GTPCLK{i}"),
                            format!("BRAM_TTERM_GTPCLK{ii}", ii = i + 4),
                        ],
                    )
                    .extra_wire(
                        format!("GTPFB{i}_I"),
                        &[
                            format!("BRAM_BTERM_GTPFB{i}_S"),
                            format!("BRAM_TTERM_GTPCLKFB{i}_N"),
                        ],
                    )
                    .extra_wire(
                        format!("GTPFB{i}_O"),
                        &[
                            format!("IOI_BTERM_GTPFB{i}"),
                            format!("BRAM_TTERM_GTPFB{ii}", ii = i + 4),
                        ],
                    )
            }
            for i in 0..3 {
                bel = bel
                    .extra_wire_force(
                        format!("RXCHBONDO{i}_I"),
                        if is_b {
                            format!("BRAM_BTERM_RXCHBONDI{i}_S")
                        } else {
                            format!("BRAM_TTERM_RXCHBONDI{i}_N")
                        },
                    )
                    .extra_wire_force(
                        format!("RXCHBONDO{i}_O"),
                        if is_b {
                            format!("IOI_BTERM_RXCHBONDI{i}")
                        } else {
                            format!("BRAM_TTERM_RXCHBONDI{i}")
                        },
                    )
                    .extra_wire_force(
                        format!("RXCHBONDI{i}_I"),
                        if is_b {
                            format!("IOI_BTERM_RXCHBONDO{i}")
                        } else {
                            format!("BRAM_TTERM_RXCHBONDO{i}")
                        },
                    )
                    .extra_wire_force(
                        format!("RXCHBONDI{i}_O"),
                        if is_b {
                            // I FUCKING HATE SPARTAN 6 IT IS A PIECE OF SHIT
                            if i == 0 {
                                format!("BRAM_BTERM_RXCHBONDO{i}_S")
                            } else {
                                format!("BRAM_BTERM_RXCHBOND0{i}_S")
                            }
                        } else {
                            format!("BRAM_TTERM_RXCHBONDO{i}_N")
                        },
                    )
            }
            for i in 0..5 {
                bel = bel
                    .extra_wire_force(
                        format!("RCALINEAST{i}_I"),
                        if is_b {
                            format!("IOI_BTERM_RCALINEAST{i}")
                        } else {
                            format!("BRAM_TTERM_RCALINEAST{i}")
                        },
                    )
                    .extra_wire_force(
                        format!("RCALINEAST{i}_O"),
                        if is_b {
                            format!("BRAM_BTERM_RCALINEAST{i}_S")
                        } else {
                            format!("BRAM_TTERM_RCALINEAST{i}_N")
                        },
                    )
                    .extra_wire_force(
                        format!("RCALOUTEAST{i}_I"),
                        if is_b {
                            format!("BRAM_BTERM_RCALOUTEAST{i}_S")
                        } else {
                            format!("BRAM_TTERM_RCALOUTEAST{i}_N")
                        },
                    )
                    .extra_wire_force(
                        format!("RCALOUTEAST{i}_O"),
                        if is_b {
                            format!("IOI_BTERM_RCALOUTEAST{i}")
                        } else {
                            format!("BRAM_TTERM_RCALOUTEAST{i}")
                        },
                    )
            }
            bels.push(bel);
            let mut xn = builder
                .xnode("GTP", tkn, xy)
                .num_tiles(8)
                .raw_tile(xy.delta(
                    0,
                    match tkn {
                        "GTPDUAL_BOT" => -10,
                        "GTPDUAL_TOP" => 8,
                        _ => unreachable!(),
                    },
                ));
            for i in 0..8 {
                xn = xn.ref_single(intfs_l[i], i, intf_rterm).ref_single(
                    intfs_r[i],
                    8 + i,
                    intf_lterm,
                );
            }
            xn.bels(bels).extract();
        }
    }

    builder.build()
}
