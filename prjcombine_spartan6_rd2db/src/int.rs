#![allow(clippy::needless_range_loop)]

use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::Part;

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("spartan6", rd);

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
    builder.mux_out(&"IMUX.LOGICIN63".to_string(), &["FAN_B"]);

    for i in 0..24 {
        builder.logic_out(
            format!("OUT{i}"),
            &[format!("LOGICOUT{i}"), format!("INT_TERM_LOGICOUT{i}")],
        );
    }

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
        let int_xy = builder.walk_to_int(term_xy, Dir::E).unwrap();
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
        (Dir::E, "INT_INTERFACE_CARRY", "INTF"),
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
    for tkn in ["INT_INTERFACE_IOI", "INT_INTERFACE_IOI_DCMBOT"] {
        builder.extract_intf("INTF.IOI", Dir::E, tkn, "INTF", true);
    }
    for tkn in ["LIOI", "LIOI_BRK", "RIOI", "RIOI_BRK"] {
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
        let n = builder.db.get_node_naming("INTF");
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
        let n = builder.db.get_node_naming("INTF");
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

    let intf_cnr = builder.db.get_node_naming("INTF.CNR");
    for (tkn, bels) in [
        (
            "LL",
            vec![
                builder.bel_xy("OCT_CAL0", "OCT_CAL", 0, 0),
                builder.bel_xy("OCT_CAL1", "OCT_CAL", 0, 1),
            ],
        ),
        (
            "LR_LOWER",
            vec![
                builder.bel_xy("OCT_CAL", "OCT_CAL", 0, 0),
                builder.bel_xy("ICAP", "ICAP", 0, 0),
                builder.bel_single("SPI_ACCESS", "SPI_ACCESS"),
            ],
        ),
        (
            "LR_UPPER",
            vec![
                builder.bel_single("SUSPEND_SYNC", "SUSPEND_SYNC"),
                builder.bel_single("POST_CRC_INTERNAL", "POST_CRC_INTERNAL"),
                builder.bel_single("STARTUP", "STARTUP"),
                builder.bel_single("SLAVE_SPI", "SLAVE_SPI"),
            ],
        ),
        (
            "UL",
            vec![
                builder.bel_xy("OCT_CAL0", "OCT_CAL", 0, 0),
                builder.bel_xy("OCT_CAL1", "OCT_CAL", 0, 1),
                builder.bel_single("PMV", "PMV"),
                builder.bel_single("DNA_PORT", "DNA_PORT"),
            ],
        ),
        (
            "UR_LOWER",
            vec![
                builder.bel_xy("OCT_CAL", "OCT_CAL", 0, 0),
                builder.bel_xy("BSCAN0", "BSCAN", 0, 0),
                builder.bel_xy("BSCAN1", "BSCAN", 0, 1),
            ],
        ),
        (
            "UR_UPPER",
            vec![
                builder.bel_xy("BSCAN0", "BSCAN", 0, 0),
                builder.bel_xy("BSCAN1", "BSCAN", 0, 1),
            ],
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut xn = builder.xnode(tkn, tkn, xy).ref_single(xy, 0, intf_cnr);
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    let intf_ioi = builder.db.get_node_naming("INTF.IOI");
    for (tkn, naming) in [
        ("LIOI", "LIOI"),
        ("LIOI_BRK", "LIOI"),
        ("RIOI", "RIOI"),
        ("RIOI_BRK", "RIOI"),
        ("BIOI_INNER", "BIOI_INNER"),
        ("BIOI_OUTER", "BIOI_OUTER"),
        ("TIOI_INNER", "TIOI_INNER"),
        ("TIOI_OUTER", "TIOI_OUTER"),
        ("BIOI_INNER_UNUSED", "BIOI_INNER_UNUSED"),
        ("BIOI_OUTER_UNUSED", "BIOI_OUTER_UNUSED"),
        ("TIOI_INNER_UNUSED", "TIOI_INNER_UNUSED"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let unused = tkn.contains("UNUSED");
            let mut bels = vec![];
            for i in 0..2 {
                let ms = match i {
                    0 => 'M',
                    1 => 'S',
                    _ => unreachable!(),
                };
                let mut bel = builder
                    .bel_xy(&format!("ILOGIC{i}"), "ILOGIC", 0, i ^ 1)
                    .pins_name_only(&[
                        "D", "DDLY", "DDLY2", "CLK0", "CLK1", "IOCE", "DFB", "CFB0", "CFB1", "OFB",
                        "TFB", "SHIFTIN", "SHIFTOUT", "SR",
                    ])
                    .extra_int_in(
                        "SR_INT",
                        &[if i == 0 {
                            "IOI_LOGICINB36"
                        } else {
                            "IOI_LOGICINB20"
                        }],
                    )
                    .extra_wire("MCB_FABRICOUT", &[format!("IOI_MCB_INBYP_{ms}")])
                    .extra_wire(
                        "IOB_I",
                        &[
                            format!("BIOI_INNER_IBUF{i}"),
                            format!("BIOI_OUTER_IBUF{i}"),
                            format!("TIOI_INNER_IBUF{i}"),
                            format!("TIOI_OUTER_IBUF{i}"),
                            format!("LIOI_IOB_IBUF{i}"),
                            format!("RIOI_IOB_IBUF{i}"),
                        ],
                    )
                    .extra_wire(
                        "D_MUX",
                        &[
                            if i == 0 {
                                "D_ILOGIC_IDATAIN_IODELAY"
                            } else {
                                "D_ILOGIC_IDATAIN_IODELAY_S"
                            },
                            if i == 0 {
                                "D_ILOGIC_IDATAIN_IODELAY_UNUSED"
                            } else {
                                "D_ILOGIC_IDATAIN_IODELAY_UNUSED_S"
                            },
                        ],
                    );
                if i == 1 {
                    bel = bel.pins_name_only(&["INCDEC", "VALID"]);
                }
                if !unused {
                    bel = bel
                        .extra_wire_force("CFB0_OUT", format!("{naming}_CFB_{ms}"))
                        .extra_wire_force("CFB1_OUT", format!("{naming}_CFB1_{ms}"))
                        .extra_wire_force("DFB_OUT", format!("{naming}_DFB_{ms}"));
                }
                bels.push(bel);
            }
            for i in 0..2 {
                let ms = match i {
                    0 => 'M',
                    1 => 'S',
                    _ => unreachable!(),
                };
                let bel = builder
                    .bel_xy(&format!("OLOGIC{i}"), "OLOGIC", 0, i ^ 1)
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
                            format!("BIOI_INNER_O{i}"),
                            format!("BIOI_OUTER_O{i}"),
                            format!("TIOI_INNER_O{i}"),
                            format!("TIOI_OUTER_O{i}"),
                            format!("LIOI_IOB_O{i}"),
                            format!("RIOI_IOB_O{i}"),
                        ],
                    )
                    .extra_wire(
                        "IOB_T",
                        &[
                            format!("BIOI_INNER_T{i}"),
                            format!("BIOI_OUTER_T{i}"),
                            format!("TIOI_INNER_T{i}"),
                            format!("TIOI_OUTER_T{i}"),
                            format!("LIOI_IOB_T{i}"),
                            format!("RIOI_IOB_T{i}"),
                        ],
                    )
                    .extra_wire("MCB_D1", &[format!("IOI_MCB_OUTP_{ms}")])
                    .extra_wire("MCB_D2", &[format!("IOI_MCB_OUTN_{ms}")]);
                bels.push(bel);
            }
            for i in 0..2 {
                let ms = match i {
                    0 => 'M',
                    1 => 'S',
                    _ => unreachable!(),
                };
                let mut bel = builder
                    .bel_xy(&format!("IODELAY{i}"), "IODELAY", 0, i ^ 1)
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
                if !unused && i == 0 {
                    bel = bel
                        .extra_wire_force("DQSOUTP_OUT", format!("{naming}_OUTP"))
                        .extra_wire_force("DQSOUTN_OUT", format!("{naming}_OUTN"));
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy("TIEOFF", "TIEOFF", 0, 0)
                    .pins_name_only(&["HARD0", "HARD1", "KEEP1"]),
            );
            for i in 0..2 {
                let ms = match i {
                    0 => 'M',
                    1 => 'S',
                    _ => unreachable!(),
                };
                let bel = builder
                    .bel_virtual(&format!("IOICLK{i}"))
                    .extra_wire("CLK0INTER", &[format!("IOI_CLK0INTER_{ms}")])
                    .extra_wire("CLK1INTER", &[format!("IOI_CLK1INTER_{ms}")])
                    .extra_wire("CLK2INTER", &[format!("IOI_CLK2INTER_{ms}")])
                    .extra_int_in("CKINT0", &[format!("IOI_CLK{i}")])
                    .extra_int_in("CKINT1", &[format!("IOI_GFAN{i}")])
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
            let mut xn = builder.xnode("IOI", tkn, xy).ref_single(xy, 0, intf_ioi);
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    for (tkn, naming, idx) in [
        ("LIOB", "LIOB", [0, 1]),
        ("LIOB_RDY", "LIOB_RDY", [0, 1]),
        ("LIOB_PCI", "LIOB_PCI", [0, 1]),
        ("RIOB", "RIOB", [0, 1]),
        ("RIOB_RDY", "RIOB_RDY", [0, 1]),
        ("RIOB_PCI", "RIOB_PCI", [0, 1]),
        ("BIOB", "BIOB_OUTER", [3, 2]),
        ("BIOB_SINGLE_ALT", "BIOB_OUTER", [3, 2]),
        ("BIOB", "BIOB_INNER", [0, 1]),
        ("BIOB_SINGLE", "BIOB_INNER", [0, 1]),
        ("TIOB", "TIOB_OUTER", [0, 1]),
        ("TIOB_SINGLE", "TIOB_OUTER", [0, 1]),
        ("TIOB", "TIOB_INNER", [2, 3]),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_indexed(&format!("IOB{i}"), "IOB", idx[i])
                    .pins_name_only(&["PADOUT", "DIFFI_IN", "DIFFO_OUT", "DIFFO_IN", "PCI_RDY"])
                    .pin_name_only("I", 1)
                    .pin_name_only("O", 1)
                    .pin_name_only("T", 1);
                if (tkn.ends_with("RDY") && i == 0) || (tkn.ends_with("PCI") && i == 1) {
                    bel = bel.pin_name_only("PCI_RDY", 1);
                }
                bels.push(bel);
            }
            let mut xn = builder.xnode("IOB", naming, xy).num_tiles(0);
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
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
            let intf = builder.db.get_node_naming("INTF");
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
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
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
            let mut bel = builder.bel_virtual("HCLK_FOLD_BUF");
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
                .xnode("HCLK_FOLD_BUF", tkn, xy)
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
        let mut bel = builder.bel_virtual("HCLK_ROOT");
        for i in 0..16 {
            bel = bel
                .extra_wire(format!("BUFG{i}"), &[format!("CLKV_GCLKH_MAIN{i}_FOLD")])
                .extra_wire(format!("CMT{i}"), &[format!("REGV_PLL_HCLK{i}")]);
        }
        bels.push(bel);
        let mut xn = builder.xnode("HCLK_ROOT", "HCLK_ROOT", xy).num_tiles(0);
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE_TOP").iter().next() {
        let mut intf_xy = Vec::new();
        let nr = builder.db.get_node_naming("INTF.RTERM");
        let nl = builder.db.get_node_naming("INTF.LTERM");
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

    builder.build()
}
