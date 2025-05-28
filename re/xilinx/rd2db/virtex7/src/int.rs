use prjcombine_interconnect::{
    db::{IntDb, WireKind},
    dir::Dir,
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;
use prjcombine_virtex4::{
    bels,
    expanded::{REGION_HCLK, REGION_LEAF},
};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(rd);

    assert_eq!(builder.db.region_slots.insert("HCLK".into()).0, REGION_HCLK);
    assert_eq!(builder.db.region_slots.insert("LEAF".into()).0, REGION_LEAF);

    for &slot in bels::SLOTS {
        builder.db.bel_slots.insert(slot.into());
    }

    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..6 {
        builder.wire(
            format!("LCLK{i}"),
            WireKind::LogicOut,
            &[format!("GCLK_B{i}_EAST"), format!("GCLK_L_B{i}")],
        );
    }
    for i in 6..12 {
        builder.wire(
            format!("LCLK{i}"),
            WireKind::LogicOut,
            &[format!("GCLK_B{i}"), format!("GCLK_L_B{i}_WEST")],
        );
    }

    for (lr, dir, dbeg, dend) in [
        ("L", Dir::E, Some((3, Dir::N)), Some((0, Dir::S, 3))),
        ("R", Dir::E, Some((0, Dir::S)), Some((3, Dir::N, 3))),
        ("L", Dir::W, Some((3, Dir::N)), Some((3, Dir::N, 1))),
        ("R", Dir::W, Some((0, Dir::S)), Some((0, Dir::S, 1))),
        ("L", Dir::N, Some((3, Dir::N)), Some((0, Dir::S, 3))),
        ("R", Dir::N, None, None),
        ("L", Dir::S, None, None),
        ("R", Dir::S, Some((0, Dir::S)), Some((3, Dir::N, 3))),
    ] {
        for i in 0..4 {
            let beg;
            if let Some((xi, dbeg)) = dbeg {
                if xi == i {
                    let beg_x = builder.mux_out(
                        format!("SNG.{dir}{lr}{i}.0.{dbeg}"),
                        &[format!("{dir}{lr}1BEG_{dbeg}{i}")],
                    );
                    if dir == dbeg {
                        continue;
                    }
                    beg = builder.branch(
                        beg_x,
                        !dbeg,
                        format!("SNG.{dir}{lr}{i}.0"),
                        &[format!("{dir}{lr}1BEG{i}")],
                    );
                } else {
                    beg = builder.mux_out(
                        format!("SNG.{dir}{lr}{i}.0"),
                        &[format!("{dir}{lr}1BEG{i}")],
                    );
                }
            } else {
                beg = builder.mux_out(
                    format!("SNG.{dir}{lr}{i}.0"),
                    &[format!("{dir}{lr}1BEG{i}")],
                );
            }
            let end = builder.branch(
                beg,
                dir,
                format!("SNG.{dir}{lr}{i}.1"),
                &[format!("{dir}{lr}1END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        end,
                        dend,
                        format!("SNG.{dir}{lr}{i}.2"),
                        &[format!("{dir}{lr}1END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((3, Dir::N, 0))),
        (Dir::N, Dir::N, Some((0, Dir::S, 2))),
        (Dir::N, Dir::E, Some((0, Dir::S, 3))),
        (Dir::N, Dir::W, Some((0, Dir::S, 0))),
        (Dir::S, Dir::S, Some((3, Dir::N, 0))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N, 0))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("DBL.{da}{db}{i}.0"), &[format!("{da}{db}2BEG{i}")]);
            let m = builder.branch(
                b,
                da,
                format!("DBL.{da}{db}{i}.1"),
                &[format!("{da}{db}2A{i}")],
            );
            let e = builder.branch(
                m,
                db,
                format!("DBL.{da}{db}{i}.2"),
                &[format!("{da}{db}2END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("DBL.{da}{db}{i}.3"),
                        &[format!("{da}{db}2END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::E, Dir::E, None),
        (Dir::W, Dir::W, Some((0, Dir::S, 0))),
    ] {
        for i in 0..4 {
            let b = builder.mux_out(format!("QUAD.{da}{db}{i}.0"), &[format!("{da}{db}4BEG{i}")]);
            let a = builder.branch(
                b,
                db,
                format!("QUAD.{da}{db}{i}.1"),
                &[format!("{da}{db}4A{i}")],
            );
            let m = builder.branch(
                a,
                da,
                format!("QUAD.{da}{db}{i}.2"),
                &[format!("{da}{db}4B{i}")],
            );
            let c = builder.branch(
                m,
                da,
                format!("QUAD.{da}{db}{i}.3"),
                &[format!("{da}{db}4C{i}")],
            );
            let e = builder.branch(
                c,
                db,
                format!("QUAD.{da}{db}{i}.4"),
                &[format!("{da}{db}4END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        e,
                        dend,
                        format!("QUAD.{da}{db}{i}.5"),
                        &[format!("{da}{db}4END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    for (da, db, dend) in [
        (Dir::N, Dir::N, Some((0, Dir::S, 1))),
        (Dir::N, Dir::E, None),
        (Dir::N, Dir::W, Some((0, Dir::S, 0))),
        (Dir::S, Dir::S, Some((3, Dir::N, 0))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N, 0))),
    ] {
        for i in 0..4 {
            let beg = builder.mux_out(format!("HEX.{da}{db}{i}.0"), &[format!("{da}{db}6BEG{i}")]);
            let a = builder.branch(
                beg,
                db,
                format!("HEX.{da}{db}{i}.1"),
                &[format!("{da}{db}6A{i}")],
            );
            let b = builder.branch(
                a,
                da,
                format!("HEX.{da}{db}{i}.2"),
                &[format!("{da}{db}6B{i}")],
            );
            let c = builder.branch(
                b,
                da,
                format!("HEX.{da}{db}{i}.3"),
                &[format!("{da}{db}6C{i}")],
            );
            let d = builder.branch(
                c,
                da,
                format!("HEX.{da}{db}{i}.4"),
                &[format!("{da}{db}6D{i}")],
            );
            let e = builder.branch(
                d,
                da,
                format!("HEX.{da}{db}{i}.5"),
                &[format!("{da}{db}6E{i}")],
            );
            let end = builder.branch(
                e,
                db,
                format!("HEX.{da}{db}{i}.6"),
                &[format!("{da}{db}6END{i}")],
            );
            if let Some((xi, dend, n)) = dend {
                if i == xi {
                    builder.branch(
                        end,
                        dend,
                        format!("HEX.{da}{db}{i}.7"),
                        &[format!("{da}{db}6END_{dend}{n}_{i}")],
                    );
                }
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.6", WireKind::MultiOut, &["LH6"]);
    let mut prev = mid;
    for i in (0..6).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mut prev = mid;
    for i in 7..13 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
    }

    let mut lv_bh_n = Vec::new();
    let mut lv_bh_s = Vec::new();

    let mid = builder.wire("LV.9", WireKind::MultiOut, &["LV9", "LV_L9"]);
    let mut prev = mid;
    for i in (0..9).rev() {
        prev = builder.multi_branch(
            prev,
            Dir::S,
            format!("LV.{i}"),
            &[format!("LV{i}"), format!("LV_L{i}")],
        );
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    for i in 10..19 {
        prev = builder.multi_branch(
            prev,
            Dir::N,
            format!("LV.{i}"),
            &[format!("LV{i}"), format!("LV_L{i}")],
        );
        lv_bh_s.push(prev);
    }
    let mid = builder.wire(
        "LVB.6",
        WireKind::MultiOut,
        &["LVB6", "LVB_L6", "LVB6_SLV", "LVB_L6_SLV"],
    );
    let mut prev = mid;
    for i in (0..6).rev() {
        prev = builder.multi_branch(
            prev,
            Dir::S,
            format!("LVB.{i}"),
            &[format!("LVB{i}"), format!("LVB_L{i}")],
        );
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    for i in 7..13 {
        prev = builder.multi_branch(
            prev,
            Dir::N,
            format!("LVB.{i}"),
            &[format!("LVB{i}"), format!("LVB_L{i}")],
        );
        lv_bh_s.push(prev);
    }

    // The control inputs.
    for i in 0..2 {
        builder.mux_out(format!("IMUX.GFAN{i}"), &[format!("GFAN{i}")]);
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK{i}"), format!("CLK_L{i}")],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.CTRL{i}"),
            &[format!("CTRL{i}"), format!("CTRL_L{i}")],
        );
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP_ALT{i}")]);
        builder.buf(
            w,
            format!("IMUX.BYP{i}.SITE"),
            &[format!("BYP{i}"), format!("BYP_L{i}")],
        );
        let b = builder.buf(
            w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
        if matches!(i, 2 | 3 | 6 | 7) {
            builder.branch(
                b,
                Dir::N,
                format!("IMUX.BYP{i}.BOUNCE.N"),
                &[format!("BYP_BOUNCE_N3_{i}")],
            );
        }
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.FAN{i}"), &[format!("FAN_ALT{i}")]);
        builder.buf(
            w,
            format!("IMUX.FAN{i}.SITE"),
            &[format!("FAN{i}"), format!("FAN_L{i}")],
        );
        let b = builder.buf(
            w,
            format!("IMUX.FAN{i}.BOUNCE"),
            &[format!("FAN_BOUNCE{i}")],
        );
        if matches!(i, 0 | 2 | 4 | 6) {
            builder.branch(
                b,
                Dir::S,
                format!("IMUX.FAN{i}.BOUNCE.S"),
                &[format!("FAN_BOUNCE_S3_{i}")],
            );
        }
    }
    for i in 0..48 {
        builder.mux_out(
            format!("IMUX.IMUX{i}"),
            &[format!("IMUX{i}"), format!("IMUX_L{i}")],
        );
    }
    for i in 0..48 {
        builder.test_out(
            format!("IMUX.BRAM{i}"),
            &[
                format!("INT_INTERFACE_BRAM_UTURN_IMUX{i}"),
                format!("INT_INTERFACE_BRAM_UTURN_R_IMUX{i}"),
            ],
        );
    }

    for i in 0..24 {
        builder.logic_out(
            format!("OUT{i}"),
            &[format!("LOGIC_OUTS{i}"), format!("LOGIC_OUTS_L{i}")],
        );
    }

    for i in 0..4 {
        builder.test_out(
            format!("TEST{i}"),
            &[
                format!("INT_INTERFACE_BLOCK_OUTS_B{i}"),
                format!("INT_INTERFACE_BLOCK_OUTS_L_B{i}"),
                format!("INT_INTERFACE_PSS_BLOCK_OUTS_L_B{i}"),
            ],
        );
    }

    builder.extract_main_passes();

    builder.node_type("INT_L", "INT", "INT.L");
    builder.node_type("INT_R", "INT", "INT.R");
    builder.node_type("INT_L_SLV_FLY", "INT", "INT.L");
    builder.node_type("INT_R_SLV_FLY", "INT", "INT.R");
    builder.node_type("INT_L_SLV", "INT", "INT.L.SLV");
    builder.node_type("INT_R_SLV", "INT", "INT.R.SLV");

    let forced: Vec<_> = (0..6)
        .map(|i| {
            (
                builder.find_wire(format!("LH.{i}")),
                builder.find_wire(format!("LH.{}", 11 - i)),
            )
        })
        .collect();
    for tkn in [
        "L_TERM_INT",
        "L_TERM_INT_BRAM",
        "INT_INTERFACE_PSS_L",
        "GTP_INT_INTERFACE_L",
        "GTP_INT_INT_TERM_L",
    ] {
        builder.extract_term_conn("TERM.W", Dir::W, tkn, &forced);
    }
    let forced: Vec<_> = (0..6)
        .map(|i| {
            (
                builder.find_wire(format!("LH.{}", 12 - i)),
                builder.find_wire(format!("LH.{}", i + 1)),
            )
        })
        .collect();
    for tkn in [
        "R_TERM_INT",
        "R_TERM_INT_GTX",
        "GTP_INT_INTERFACE_R",
        "GTP_INT_INT_TERM_R",
    ] {
        builder.extract_term_conn("TERM.E", Dir::E, tkn, &forced);
    }
    let forced = [
        (
            builder.find_wire("SNG.WL3.2"),
            builder.find_wire("SNG.WR0.1"),
        ),
        (
            builder.find_wire("SNG.ER0.0"),
            builder.find_wire("SNG.EL3.0.N"),
        ),
        (
            builder.find_wire("DBL.NW0.1"),
            builder.find_wire("DBL.SW3.0"),
        ),
        (
            builder.find_wire("DBL.NE0.1"),
            builder.find_wire("DBL.SE3.0"),
        ),
        (
            builder.find_wire("HEX.SW3.7"),
            builder.find_wire("HEX.NW0.6"),
        ),
        (
            builder.find_wire("HEX.NE0.5"),
            builder.find_wire("HEX.SE3.4"),
        ),
    ];
    for tkn in [
        "B_TERM_INT",
        "B_TERM_INT_SLV",
        "BRKH_B_TERM_INT",
        "HCLK_L_BOT_UTURN",
        "HCLK_R_BOT_UTURN",
    ] {
        builder.extract_term_conn("TERM.S", Dir::S, tkn, &forced);
    }
    let forced = [
        (
            builder.find_wire("SNG.EL3.0"),
            builder.find_wire("SNG.ER0.0.S"),
        ),
        (
            builder.find_wire("SNG.WR0.2"),
            builder.find_wire("SNG.WL3.1"),
        ),
        (
            builder.find_wire("DBL.SE3.1"),
            builder.find_wire("DBL.NE0.0"),
        ),
        (
            builder.find_wire("HEX.SE3.5"),
            builder.find_wire("HEX.NE0.4"),
        ),
    ];
    for tkn in [
        "T_TERM_INT",
        "T_TERM_INT_SLV",
        "BRKH_TERM_INT",
        "BRKH_INT_PSS",
        "HCLK_L_TOP_UTURN",
        "HCLK_R_TOP_UTURN",
    ] {
        builder.extract_term_conn("TERM.N", Dir::N, tkn, &forced);
    }
    // TODO: this enough?
    builder.make_blackhole_term("TERM.S.HOLE", Dir::S, &lv_bh_s);
    builder.make_blackhole_term("TERM.N.HOLE", Dir::N, &lv_bh_n);

    for (dir, n, tkn) in [
        (Dir::W, "L", "INT_INTERFACE_L"),
        (Dir::E, "R", "INT_INTERFACE_R"),
        (Dir::W, "L", "IO_INT_INTERFACE_L"),
        (Dir::E, "R", "IO_INT_INTERFACE_R"),
        (Dir::W, "PSS", "INT_INTERFACE_PSS_L"),
    ] {
        builder.extract_intf("INTF", dir, tkn, format!("INTF.{n}"), true);
    }
    for (dir, n, tkn) in [
        (Dir::W, "L", "BRAM_INT_INTERFACE_L"),
        (Dir::E, "R", "BRAM_INT_INTERFACE_R"),
    ] {
        builder.extract_intf("INTF.BRAM", dir, tkn, format!("INTF.{n}"), true);
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
        builder.extract_intf("INTF.DELAY", dir, tkn, format!("INTF.{n}"), true);
    }

    let forced: Vec<_> = builder
        .db
        .wires
        .iter()
        .filter_map(|(w, wn, _)| {
            if wn.starts_with("SNG.S") || wn.starts_with("SNG.N") {
                None
            } else {
                Some(w)
            }
        })
        .collect();

    builder.extract_pass_buf("BRKH", Dir::S, "BRKH_INT", "BRKH", &forced);

    for (tkn, kind) in [
        ("CLBLL_L", "CLBLL"),
        ("CLBLL_R", "CLBLL"),
        ("CLBLM_L", "CLBLM"),
        ("CLBLM_R", "CLBLM"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
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
            builder.extract_xnode_bels(
                kind,
                xy,
                &[],
                &[int_xy],
                tkn,
                &[
                    builder
                        .bel_xy(bels::SLICE0, "SLICE", 0, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                    builder
                        .bel_xy(bels::SLICE1, "SLICE", 1, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                ],
            );
        }
    }

    for tkn in ["BRAM_L", "BRAM_R"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let n = builder.ndb.get_tile_class_naming(if tkn == "BRAM_L" {
                "INTF.L"
            } else {
                "INTF.R"
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
            let mut bel_bram_f = builder
                .bel_xy(bels::BRAM_F, "RAMB36", 0, 0)
                .pins_name_only(&["CASCADEINA", "CASCADEINB"])
                .pin_name_only("CASCADEOUTA", 1)
                .pin_name_only("CASCADEOUTB", 1);
            for ab in ["ARD", "BWR"] {
                for ul in ['U', 'L'] {
                    for i in 0..16 {
                        if i == 15 && ul == 'U' {
                            continue;
                        }
                        bel_bram_f = bel_bram_f.pin_name_only(&format!("ADDR{ab}ADDR{ul}{i}"), 0);
                    }
                }
            }
            let mut bel_bram_h0 = builder.bel_xy(bels::BRAM_H0, "RAMB18", 0, 0);
            let mut bel_bram_h1 = builder
                .bel_xy(bels::BRAM_H1, "RAMB18", 0, 1)
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
            let mut bel_bram_addr = builder.bel_virtual(bels::BRAM_ADDR);
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
            builder.extract_xnode_bels_intf(
                "BRAM",
                xy,
                &[],
                &int_xy,
                &intf_xy,
                tkn,
                &[bel_bram_f, bel_bram_h0, bel_bram_h1, bel_bram_addr],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_BRAM").iter().next() {
        let mut bram_xy = Vec::new();
        for dy in [1, 6, 11] {
            bram_xy.push(Coord {
                x: xy.x,
                y: xy.y + dy,
            });
        }
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        if rd.tile_kinds.key(rd.tiles[&bram_xy[0]].kind) == "BRAM_L" {
            let n = builder.ndb.get_tile_class_naming("INTF.L");
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
            let n = builder.ndb.get_tile_class_naming("INTF.R");
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
        builder.extract_xnode_bels_intf(
            "PMVBRAM",
            xy,
            &bram_xy,
            &int_xy,
            &intf_xy,
            "PMVBRAM",
            &[builder.bel_xy(bels::PMVBRAM, "PMVBRAM", 0, 0)],
        );

        let bel = builder
            .bel_xy(bels::PMVBRAM_NC, "PMVBRAM", 0, 0)
            .pins_name_only(&[
                "O", "ODIV2", "ODIV4", "SELECT1", "SELECT2", "SELECT3", "SELECT4",
            ]);
        builder
            .xnode("PMVBRAM_NC", "PMVBRAM_NC", xy)
            .num_tiles(0)
            .bel(bel)
            .extract();
    }

    for tkn in ["DSP_L", "DSP_R"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let n =
                builder
                    .ndb
                    .get_tile_class_naming(if tkn == "DSP_L" { "INTF.L" } else { "INTF.R" });
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
                let mut bel = builder.bel_xy(bels::DSP[i], "DSP48", 0, i);
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
                    .bel_xy(bels::TIEOFF_DSP, "TIEOFF", 0, 0)
                    .pins_name_only(&["HARD0", "HARD1"]),
            );
            builder.extract_xnode_bels_intf("DSP", xy, &[], &int_xy, &intf_xy, tkn, &bels_dsp);
        }
    }

    for (kind, tkn) in [("PCIE_L", "PCIE_BOT_LEFT"), ("PCIE_R", "PCIE_BOT")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let (int_cols, intf_cols, intf_namings) = if kind == "PCIE_R" {
                (
                    [xy.x - 2, xy.x + 6],
                    [xy.x - 1, xy.x + 5],
                    [
                        builder.ndb.get_tile_class_naming("INTF.PCIE_R"),
                        builder.ndb.get_tile_class_naming("INTF.PCIE_L"),
                    ],
                )
            } else {
                (
                    [xy.x + 6, xy.x - 2],
                    [xy.x + 5, xy.x - 1],
                    [
                        builder.ndb.get_tile_class_naming("INTF.PCIE_LEFT_L"),
                        builder.ndb.get_tile_class_naming("INTF.PCIE_R"),
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
            builder.extract_xnode_bels_intf(
                "PCIE",
                xy,
                &[t_xy],
                &int_xy,
                &intf_xy,
                kind,
                &[builder.bel_xy(bels::PCIE, "PCIE", 0, 0)],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE3_RIGHT").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let nl = builder.ndb.get_tile_class_naming("INTF.PCIE3_L");
        let nr = builder.ndb.get_tile_class_naming("INTF.PCIE3_R");
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
        builder.extract_xnode_bels_intf(
            "PCIE3",
            xy,
            &[b_xy, t_xy],
            &int_xy,
            &intf_xy,
            "PCIE3",
            &[builder.bel_xy(bels::PCIE3, "PCIE3", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("INT_L").iter().next() {
        let mut bel_l = builder.bel_virtual(bels::INT_LCLK_W);
        let mut bel_r = builder.bel_virtual(bels::INT_LCLK_E).raw_tile(1);
        for i in 6..12 {
            bel_l = bel_l
                .extra_wire(format!("LCLK{i}_I"), &[format!("GCLK_L_B{i}")])
                .extra_int_out(format!("LCLK{i}_O_L"), &[format!("GCLK_L_B{i}_WEST")])
                .extra_int_out(format!("LCLK{i}_O_R"), &[format!("GCLK_L_B{i}_EAST")]);
        }
        for i in 0..6 {
            bel_r = bel_r
                .extra_wire(format!("LCLK{i}_I"), &[format!("GCLK_B{i}")])
                .extra_int_out(format!("LCLK{i}_O_L"), &[format!("GCLK_B{i}_WEST")])
                .extra_int_out(format!("LCLK{i}_O_R"), &[format!("GCLK_B{i}_EAST")]);
        }
        builder
            .xnode("INT_LCLK", "INT_LCLK", xy)
            .raw_tile_single(xy.delta(1, 0), 1)
            .num_tiles(2)
            .bel(bel_l)
            .bel(bel_r)
            .extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_L").iter().next() {
        let mut bel_l = builder.bel_virtual(bels::HCLK_W);
        let mut bel_r = builder.bel_virtual(bels::HCLK_E).raw_tile(1);
        for i in 6..12 {
            bel_l = bel_l
                .extra_wire(
                    format!("LCLK{i}_D"),
                    &[format!("HCLK_LEAF_CLK_B_BOTL{ii}", ii = i - 6)],
                )
                .extra_wire(
                    format!("LCLK{i}_U"),
                    &[format!("HCLK_LEAF_CLK_B_TOPL{ii}", ii = i - 6)],
                );
        }
        for i in 0..6 {
            bel_r = bel_r
                .extra_wire(format!("LCLK{i}_D"), &[format!("HCLK_LEAF_CLK_B_BOT{i}")])
                .extra_wire(format!("LCLK{i}_U"), &[format!("HCLK_LEAF_CLK_B_TOP{i}")]);
        }
        for i in 0..8 {
            bel_r = bel_r
                .extra_wire(format!("HCLK{i}"), &[format!("HCLK_CK_BUFHCLK{i}")])
                .extra_wire(format!("HCLK{i}_O"), &[format!("HCLK_CK_INOUT_R{i}")]);
            bel_l = bel_l.extra_wire(format!("HCLK{i}_I"), &[format!("HCLK_CK_OUTIN_L{i}")]);
        }
        for i in 8..12 {
            bel_l = bel_l
                .extra_wire(format!("HCLK{i}"), &[format!("HCLK_CK_BUFHCLK{i}")])
                .extra_wire(
                    format!("HCLK{i}_O"),
                    &[format!("HCLK_CK_INOUT_L{ii}", ii = i - 8)],
                );
            bel_r = bel_r.extra_wire(
                format!("HCLK{i}_I"),
                &[format!("HCLK_CK_OUTIN_R{ii}", ii = i - 4)],
            );
        }
        for i in 0..4 {
            bel_l = bel_l
                .extra_wire(format!("RCLK{i}"), &[format!("HCLK_CK_BUFRCLK{i}")])
                .extra_wire(
                    format!("RCLK{i}_O"),
                    &[format!("HCLK_CK_INOUT_L{ii}", ii = i + 4)],
                );
            bel_r = bel_r.extra_wire(format!("RCLK{i}_I"), &[format!("HCLK_CK_OUTIN_R{i}")]);
        }
        builder
            .xnode("HCLK", "HCLK", xy)
            .raw_tile(xy.delta(1, 0))
            .num_tiles(0)
            .bel(bel_l)
            .bel(bel_r)
            .extract();
    }

    for (node, tkn, num_tiles) in [
        ("CLK_BUFG_REBUF", "CLK_BUFG_REBUF", 2),
        ("CLK_BALI_REBUF", "CLK_BALI_REBUF", 16),
        ("CLK_BALI_REBUF", "CLK_BALI_REBUF_GTZ_TOP", 16),
        ("CLK_BALI_REBUF", "CLK_BALI_REBUF_GTZ_BOT", 16),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            let (bkd, bku, xd, xu, swz) = match tkn {
                "CLK_BUFG_REBUF" => ("GCLK_TEST_BUF", "GCLK_TEST_BUF", 0, 1, false),
                "CLK_BALI_REBUF" => ("GCLK_TEST_BUF", "GCLK_TEST_BUF", 0, 2, true),
                "CLK_BALI_REBUF_GTZ_BOT" => ("GCLK_TEST_BUF", "BUFG_LB", 0, 0, true),
                "CLK_BALI_REBUF_GTZ_TOP" => ("BUFG_LB", "GCLK_TEST_BUF", 0, 0, true),
                _ => unreachable!(),
            };
            for i in 0..16 {
                let y = if swz {
                    (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3
                } else {
                    i
                };
                bels.push(
                    builder
                        .bel_xy(bels::GCLK_TEST_BUF_REBUF_S[i], bkd, xd, y)
                        .pins_name_only(&["CLKIN", "CLKOUT"]),
                );
            }
            for i in 0..16 {
                let y = if swz {
                    (i & 3) << 2 | (i & 4) >> 1 | (i & 8) >> 3
                } else {
                    i
                };
                bels.push(
                    builder
                        .bel_xy(bels::GCLK_TEST_BUF_REBUF_N[i], bku, xu, y)
                        .pins_name_only(&["CLKIN", "CLKOUT"]),
                );
            }
            let mut bel = builder.bel_virtual(bels::CLK_REBUF);
            for i in 0..32 {
                bel = bel
                    .extra_wire(
                        format!("GCLK{i}_D"),
                        &[
                            format!("CLK_BUFG_REBUF_R_CK_GCLK{i}_BOT"),
                            format!("CLK_BALI_REBUF_R_GCLK{i}_BOT"),
                        ],
                    )
                    .extra_wire(
                        format!("GCLK{i}_U"),
                        &[
                            format!("CLK_BUFG_REBUF_R_CK_GCLK{i}_TOP"),
                            format!("CLK_BALI_REBUF_R_GCLK{i}_TOP"),
                        ],
                    );
            }
            bels.push(bel);
            builder
                .xnode(node, node, xy)
                .num_tiles(num_tiles)
                .bels(bels)
                .extract();
        }
    }

    for tkn in ["CLK_HROW_BOT_R", "CLK_HROW_TOP_R"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..32 {
                bels.push(
                    builder
                        .bel_xy(
                            bels::GCLK_TEST_BUF_HROW_GCLK[i],
                            "GCLK_TEST_BUF",
                            i >> 4,
                            i & 0xf ^ 0xf,
                        )
                        .pins_name_only(&["CLKIN", "CLKOUT"]),
                );
            }
            for i in 0..12 {
                bels.push(
                    builder
                        .bel_xy(bels::BUFHCE_W[i], "BUFHCE", 0, i)
                        .pins_name_only(&["I", "O"]),
                );
            }
            for i in 0..12 {
                bels.push(
                    builder
                        .bel_xy(bels::BUFHCE_E[i], "BUFHCE", 1, i)
                        .pins_name_only(&["I", "O"]),
                );
            }
            bels.extend([
                builder
                    .bel_xy(bels::GCLK_TEST_BUF_HROW_BUFH_W, "GCLK_TEST_BUF", 3, 1)
                    .pins_name_only(&["CLKIN", "CLKOUT"]),
                builder
                    .bel_xy(bels::GCLK_TEST_BUF_HROW_BUFH_E, "GCLK_TEST_BUF", 3, 0)
                    .pins_name_only(&["CLKIN", "CLKOUT"]),
            ]);
            let mut bel = builder
                .bel_virtual(bels::CLK_HROW)
                .extra_wire("HCLK_TEST_IN_L", &["CLK_HROW_CK_IN_L_TEST_IN"])
                .extra_wire("HCLK_TEST_IN_R", &["CLK_HROW_CK_IN_R_TEST_IN"])
                .extra_wire("HCLK_TEST_OUT_L", &["CLK_HROW_CK_IN_L_TEST_OUT"])
                .extra_wire("HCLK_TEST_OUT_R", &["CLK_HROW_CK_IN_R_TEST_OUT"])
                .extra_int_in("BUFHCE_CKINT0", &["CLK_HROW_CK_INT_0_0"])
                .extra_int_in("BUFHCE_CKINT1", &["CLK_HROW_CK_INT_0_1"])
                .extra_int_in("BUFHCE_CKINT2", &["CLK_HROW_CK_INT_1_0"])
                .extra_int_in("BUFHCE_CKINT3", &["CLK_HROW_CK_INT_1_1"]);
            for i in 0..32 {
                bel = bel
                    .extra_wire(format!("GCLK{i}"), &[format!("CLK_HROW_R_CK_GCLK{i}")])
                    .extra_wire(
                        format!("GCLK{i}_TEST_IN"),
                        &[format!("CLK_HROW_CK_GCLK_IN_TEST{i}")],
                    )
                    .extra_wire(
                        format!("GCLK{i}_TEST_OUT"),
                        &[format!("CLK_HROW_CK_GCLK_OUT_TEST{i}")],
                    )
                    .extra_wire(
                        format!("GCLK_TEST{i}"),
                        &[format!("CLK_HROW_CK_GCLK_TEST{i}")],
                    )
                    .extra_wire(
                        format!("CASCO{i}"),
                        &[
                            format!("CLK_HROW_BOT_R_CK_BUFG_CASCO{i}"),
                            format!("CLK_HROW_TOP_R_CK_BUFG_CASCO{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("CASCI{i}"),
                        &[
                            format!("CLK_HROW_BOT_R_CK_BUFG_CASCIN{i}"),
                            format!("CLK_HROW_TOP_R_CK_BUFG_CASCIN{i}"),
                        ],
                    );
            }
            for lr in ['L', 'R'] {
                for i in 0..12 {
                    bel = bel.extra_wire(
                        format!("HCLK{i}_{lr}"),
                        &[format!("CLK_HROW_CK_BUFHCLK_{lr}{i}")],
                    );
                }
                for i in 0..4 {
                    bel = bel.extra_wire(
                        format!("RCLK{i}_{lr}"),
                        &[format!("CLK_HROW_CK_BUFRCLK_{lr}{i}")],
                    );
                }
                for i in 0..14 {
                    bel = bel
                        .extra_wire(format!("HIN{i}_{lr}"), &[format!("CLK_HROW_CK_IN_{lr}{i}")]);
                }
            }
            bels.push(bel);
            builder
                .xnode("CLK_HROW", tkn, xy)
                .num_tiles(2)
                .ref_int(xy.delta(-2, -1), 0)
                .ref_int(xy.delta(-2, 1), 1)
                .bels(bels)
                .extract();
        }
    }

    for tkn in ["CLK_BUFG_BOT_R", "CLK_BUFG_TOP_R"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..16 {
                bels.push(
                    builder
                        .bel_xy(bels::BUFGCTRL[i], "BUFGCTRL", 0, i)
                        .pins_name_only(&["I0", "I1", "O"])
                        .extra_wire(
                            "CASCI0",
                            &[
                                format!("CLK_BUFG_BOT_R_CK_MUXED{ii}", ii = i * 2),
                                format!("CLK_BUFG_TOP_R_CK_MUXED{ii}", ii = i * 2),
                            ],
                        )
                        .extra_wire(
                            "CASCI1",
                            &[
                                format!("CLK_BUFG_BOT_R_CK_MUXED{ii}", ii = i * 2 + 1),
                                format!("CLK_BUFG_TOP_R_CK_MUXED{ii}", ii = i * 2 + 1),
                            ],
                        )
                        .extra_int_in(
                            "CKINT0",
                            &[format!("CLK_BUFG_IMUX{j}_{k}", j = 24 + (i % 4), k = i / 4)],
                        )
                        .extra_int_in(
                            "CKINT1",
                            &[format!("CLK_BUFG_IMUX{j}_{k}", j = 28 + (i % 4), k = i / 4)],
                        )
                        .extra_wire("FB", &[format!("CLK_BUFG_R_FBG_OUT{i}")])
                        .extra_wire(
                            "GCLK",
                            &[format!(
                                "CLK_BUFG_CK_GCLK{ii}",
                                ii = if tkn == "CLK_BUFG_TOP_R" { i + 16 } else { i }
                            )],
                        )
                        .extra_int_out("FB_TEST0", &[format!("CLK_BUFG_R_CK_FB_TEST0_{i}")])
                        .extra_int_out("FB_TEST1", &[format!("CLK_BUFG_R_CK_FB_TEST1_{i}")]),
                );
            }
            let intf = builder.ndb.get_tile_class_naming("INTF.R");
            builder
                .xnode("CLK_BUFG", tkn, xy)
                .num_tiles(4)
                .ref_int(xy.delta(-2, 0), 0)
                .ref_single(xy.delta(-1, 0), 0, intf)
                .ref_int(xy.delta(-2, 1), 1)
                .ref_single(xy.delta(-1, 1), 1, intf)
                .ref_int(xy.delta(-2, 2), 2)
                .ref_single(xy.delta(-1, 2), 2, intf)
                .ref_int(xy.delta(-2, 3), 3)
                .ref_single(xy.delta(-1, 3), 3, intf)
                .bels(bels)
                .extract();
        }
    }

    for (tkn, slot, sslot, dy) in [
        ("CLK_PMV", bels::PMV0, "PMV", 3),
        ("CLK_PMVIOB", bels::PMVIOB, "PMVIOB", 0),
        ("CLK_PMV2_SVT", bels::PMV2_SVT, "PMV", 0),
        ("CLK_PMV2", bels::PMV2, "PMV", 0),
        ("CLK_MTBF2", bels::MTBF2, "MTBF2", 0),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let bel = builder.bel_xy(slot, sslot, 0, 0);
            let intf = builder.ndb.get_tile_class_naming("INTF.R");
            builder
                .xnode(tkn, tkn, xy)
                .ref_int(xy.delta(-2, dy), 0)
                .ref_single(xy.delta(-1, dy), 0, intf)
                .bel(bel)
                .extract();
        }
    }

    for (tkn, nn) in [("HCLK_IOI", "HCLK_IOI_HP"), ("HCLK_IOI3", "HCLK_IOI_HR")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = rd
                .tile_kinds
                .key(rd.tiles[&xy.delta(0, 1)].kind)
                .starts_with('L');
            let int_xy = if is_l {
                if rd.tile_kinds.key(rd.tiles[&xy.delta(1, 0)].kind) == "HCLK_TERM" {
                    xy.delta(3, 0)
                } else {
                    xy.delta(2, 0)
                }
            } else {
                if rd.tile_kinds.key(rd.tiles[&xy.delta(-1, 0)].kind) == "HCLK_TERM" {
                    xy.delta(-3, 0)
                } else {
                    xy.delta(-2, 0)
                }
            };
            let intf_xy = int_xy.delta(if is_l { -1 } else { 1 }, 0);
            let intf = builder
                .ndb
                .get_tile_class_naming(if is_l { "INTF.L" } else { "INTF.R" });

            let mut bels = vec![];
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(bels::BUFIO[i], "BUFIO", 0, i ^ 2)
                        .pins_name_only(&["I", "O"]),
                );
            }
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(bels::BUFR[i], "BUFR", 0, i ^ 2)
                        .pins_name_only(&["I", "O"]),
                );
            }
            bels.push(
                builder
                    .bel_xy(bels::IDELAYCTRL, "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
            );
            if tkn == "HCLK_IOI" {
                bels.push(builder.bel_xy(bels::DCI, "DCI", 0, 0).pins_name_only(&[
                    "DCIDATA",
                    "DCISCLK",
                    "DCIADDRESS0",
                    "DCIADDRESS1",
                    "DCIADDRESS2",
                    "DCIIOUPDATE",
                    "DCIREFIOUPDATE",
                ]));
            }
            let mut bel = builder.bel_virtual(bels::HCLK_IOI);
            for i in 0..12 {
                bel = bel
                    .extra_wire(format!("HCLK{i}"), &[format!("HCLK_IOI_CK_BUFHCLK{i}")])
                    .extra_wire(format!("HCLK{i}_BUF"), &[format!("HCLK_IOI_CK_IGCLK{i}")]);
            }
            for i in 0..6 {
                bel = bel
                    .extra_wire(
                        format!("HCLK_IO_D{i}"),
                        &[format!("HCLK_IOI_LEAF_GCLK_BOT{i}")],
                    )
                    .extra_wire(
                        format!("HCLK_IO_U{i}"),
                        &[format!("HCLK_IOI_LEAF_GCLK_TOP{i}")],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("RCLK{i}"), &[format!("HCLK_IOI_CK_BUFRCLK{i}")])
                    .extra_wire(format!("RCLK{i}_IO"), &[format!("HCLK_IOI_RCLK2IO{i}")])
                    .extra_wire(format!("RCLK{i}_PRE"), &[format!("HCLK_IOI_RCLK2RCLK{i}")])
                    .extra_wire(format!("IOCLK{i}"), &[format!("HCLK_IOI_IOCLK{i}")])
                    .extra_wire(
                        format!("IOCLK_IN{i}"),
                        &[format!("HCLK_IOI_IO_PLL_CLK{i}_DMUX")],
                    )
                    .extra_wire(format!("IOCLK_IN{i}_BUFR"), &[format!("HCLK_IOI_RCLK{i}")])
                    .extra_wire(
                        format!("IOCLK_IN{i}_PERF"),
                        &[format!("HCLK_IOI_IOCLK_PLL{i}")],
                    )
                    .extra_wire(
                        format!("IOCLK_IN{i}_PAD"),
                        &[if i < 2 {
                            format!("HCLK_IOI_I2IOCLK_TOP{i}")
                        } else {
                            format!("HCLK_IOI_I2IOCLK_BOT{ii}", ii = i - 2)
                        }],
                    )
                    .extra_int_in(
                        format!("BUFR_CKINT{i}"),
                        &[format!("HCLK_IOI_RCLK_IMUX{i}")],
                    );
            }
            bels.push(bel);
            let mut xn = builder
                .xnode(nn, nn, xy)
                .raw_tile(xy.delta(0, -4))
                .raw_tile(xy.delta(0, -2))
                .raw_tile(xy.delta(0, 1))
                .raw_tile(xy.delta(0, 3))
                .num_tiles(8);
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
        }
    }

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
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_hpio = !tkn.contains('3');
            let is_l = tkn.starts_with('L');
            let is_sing = tkn.contains("SING");
            let lr = if is_l { 'L' } else { 'R' };
            let iob_xy = xy.delta(if is_l { -1 } else { 1 }, 0);
            let int_xy = if is_l {
                if rd.tile_kinds.key(rd.tiles[&xy.delta(1, 0)].kind) == "L_TERM_INT" {
                    xy.delta(3, 0)
                } else {
                    xy.delta(2, 0)
                }
            } else {
                if rd.tile_kinds.key(rd.tiles[&xy.delta(-1, 0)].kind) == "R_TERM_INT" {
                    xy.delta(-3, 0)
                } else {
                    xy.delta(-2, 0)
                }
            };
            let intf_xy = int_xy.delta(if is_l { -1 } else { 1 }, 0);
            let intf = builder
                .ndb
                .get_tile_class_naming(if is_l { "INTF.L" } else { "INTF.R" });
            let mut bels = vec![];
            let num = if is_sing { 1 } else { 2 };
            for i in 0..num {
                let ix = if is_sing { i } else { i ^ 1 };
                let mut bel = builder
                    .bel_xy(bels::ILOGIC[i], "ILOGIC", 0, i)
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
                    ])
                    .pin_dummy("REV")
                    .extra_wire(
                        "IOB_I",
                        &[format!("LIOI_IBUF{ix}"), format!("RIOI_IBUF{ix}")],
                    )
                    .extra_wire("IOB_I_BUF", &[format!("LIOI_I{ix}"), format!("RIOI_I{ix}")])
                    .extra_int_in("CKINT0", &[format!("IOI_IMUX20_{i}")])
                    .extra_int_in("CKINT1", &[format!("IOI_IMUX22_{i}")])
                    .extra_wire(
                        "PHASER_ICLK",
                        &[if i == 0 {
                            "IOI_PHASER_TO_IO_ICLK"
                        } else {
                            "IOI_PHASER_TO_IO_ICLK_0"
                        }],
                    )
                    .extra_wire(
                        "PHASER_ICLKDIV",
                        &[if i == 0 {
                            "IOI_PHASER_TO_IO_ICLKDIV"
                        } else {
                            "IOI_PHASER_TO_IO_ICLKDIV_0"
                        }],
                    );
                if i == 1 || is_sing {
                    bel = bel.pin_dummy("SHIFTIN1").pin_dummy("SHIFTIN2");
                }
                if i == 1 {
                    bel = bel.extra_wire_force("CLKOUT", format!("{lr}IOI_I2GCLK_TOP0"))
                }
                bels.push(bel);
            }
            for i in 0..num {
                let ix = if is_sing { i } else { i ^ 1 };
                let mut bel = builder
                    .bel_xy(bels::OLOGIC[i], "OLOGIC", 0, i)
                    .pins_name_only(&[
                        "CLK",
                        "CLKB",
                        "CLKDIVB",
                        "CLKDIVF",
                        "CLKDIVFB",
                        "OFB",
                        "TFB",
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
                    .extra_int_out("CLKDIV", &[format!("IOI_OLOGIC{ix}_CLKDIV")])
                    .extra_int_in("CLKDIV_CKINT", &[format!("IOI_IMUX8_{i}")])
                    .extra_int_in("CLK_CKINT", &[format!("IOI_IMUX31_{i}")])
                    .extra_int_out("CLK_MUX", &[format!("IOI_OCLK_{ix}")])
                    .extra_wire("CLKM", &[format!("IOI_OCLKM_{ix}")])
                    .extra_int_out(
                        "TFB_BUF",
                        &[
                            format!("LIOI_OLOGIC{ix}_TFB_LOCAL"),
                            format!("RIOI_OLOGIC{ix}_TFB_LOCAL"),
                        ],
                    )
                    .extra_wire("IOB_O", &[format!("LIOI_O{ix}"), format!("RIOI_O{ix}")])
                    .extra_wire("IOB_T", &[format!("LIOI_T{ix}"), format!("RIOI_T{ix}")])
                    .extra_wire(
                        "PHASER_OCLK",
                        &[if i == 0 {
                            "IOI_PHASER_TO_IO_OCLK"
                        } else {
                            "IOI_PHASER_TO_IO_OCLK_0"
                        }],
                    )
                    .extra_wire(
                        "PHASER_OCLK90",
                        &[if i == 0 {
                            "IOI_PHASER_TO_IO_OCLK1X_90"
                        } else {
                            "IOI_PHASER_TO_IO_OCLK1X_90_0"
                        }],
                    )
                    .extra_wire(
                        "PHASER_OCLKDIV",
                        &[if i == 0 {
                            "IOI_PHASER_TO_IO_OCLKDIV"
                        } else {
                            "IOI_PHASER_TO_IO_OCLKDIV_0"
                        }],
                    );
                if i == 0 {
                    bel = bel.pin_dummy("SHIFTIN1").pin_dummy("SHIFTIN2");
                }
                bels.push(bel);
            }
            for i in 0..num {
                bels.push(
                    builder
                        .bel_xy(bels::IDELAY[i], "IDELAY", 0, i)
                        .pins_name_only(&["IDATAIN", "DATAOUT"]),
                );
            }
            if is_hpio {
                for i in 0..num {
                    bels.push(
                        builder
                            .bel_xy(bels::ODELAY[i], "ODELAY", 0, i)
                            .pins_name_only(&["ODATAIN", "CLKIN"]),
                    );
                }
            }
            for i in 0..num {
                let mut bel = builder
                    .bel_xy(bels::IOB[i], "IOB", 0, i)
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
            let mut bel = builder.bel_virtual(bels::IOI).extra_wire(
                "TBYTEIN",
                &["IOI_TBYTEIN", "IOI_SING_TBYTEIN", "IOI_TBYTEIN_TERM"],
            );
            for i in 0..4 {
                bel = bel.extra_wire(
                    format!("IOCLK{i}"),
                    &[format!("IOI_IOCLK{i}"), format!("IOI_SING_IOCLK{i}")],
                )
            }
            for i in 0..6 {
                bel = bel.extra_wire(
                    format!("HCLK{i}"),
                    &[
                        format!("IOI_LEAF_GCLK{i}"),
                        format!("IOI_SING_LEAF_GCLK{i}"),
                    ],
                )
            }
            for i in 0..4 {
                bel = bel.extra_wire(
                    format!("RCLK{i}"),
                    &[
                        format!("IOI_RCLK_FORIO{i}"),
                        format!("IOI_SING_RCLK_FORIO{i}"),
                    ],
                )
            }
            bels.push(bel);

            if is_sing {
                builder
                    .xnode(if is_hpio { "IO_HP_BOT" } else { "IO_HR_BOT" }, tkn, xy)
                    .raw_tile(iob_xy)
                    .ref_int(int_xy, 0)
                    .ref_single(intf_xy, 0, intf)
                    .bels(bels.clone())
                    .extract();
                builder
                    .xnode(if is_hpio { "IO_HP_TOP" } else { "IO_HR_TOP" }, tkn, xy)
                    .raw_tile(iob_xy)
                    .ref_int(int_xy, 0)
                    .ref_single(intf_xy, 0, intf)
                    .bels(bels)
                    .extract();
            } else {
                builder
                    .xnode(if is_hpio { "IO_HP_PAIR" } else { "IO_HR_PAIR" }, tkn, xy)
                    .raw_tile(iob_xy)
                    .num_tiles(2)
                    .ref_int(int_xy, 0)
                    .ref_single(intf_xy, 0, intf)
                    .ref_int(int_xy.delta(0, 1), 1)
                    .ref_single(intf_xy.delta(0, 1), 1, intf)
                    .bels(bels)
                    .extract();
            }
        }
    }

    for tkn in ["CMT_TOP_L_LOWER_B", "CMT_TOP_R_LOWER_B"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = tkn == "CMT_TOP_L_LOWER_B";
            let int_xy = xy.delta(if is_l { 3 } else { -3 }, -8);
            let intf_xy = xy.delta(if is_l { 2 } else { -2 }, -8);
            let intf = builder
                .ndb
                .get_tile_class_naming(if is_l { "INTF.L" } else { "INTF.R" });
            let mut bels = vec![];
            for i in 0..4 {
                let abcd = ['A', 'B', 'C', 'D'][i];
                let mut bel = builder
                    .bel_xy(bels::PHASER_IN[i], "PHASER_IN_PHY", 0, i % 2)
                    .raw_tile(1 + i / 2)
                    .pins_name_only(&[
                        "MEMREFCLK",
                        "FREQREFCLK",
                        "SYNCIN",
                        "ICLK",
                        "ICLKDIV",
                        "WRENABLE",
                    ])
                    .extra_wire(
                        "DQS_PAD",
                        &[[
                            "CMT_PHASER_DOWN_DQS_TO_PHASER_A",
                            "CMT_PHASER_DOWN_DQS_TO_PHASER_B",
                            "CMT_PHASER_UP_DQS_TO_PHASER_C",
                            "CMT_PHASER_UP_DQS_TO_PHASER_D",
                        ][i]],
                    )
                    .extra_wire("IO_ICLK", &[format!("CMT_PHASER_IN_{abcd}_ICLK")])
                    .extra_wire("IO_ICLKDIV", &[format!("CMT_PHASER_IN_{abcd}_ICLKDIV")])
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
                    "PHASEREFCLK",
                    "RCLK",
                ] {
                    bel = bel.pin_name_only(pin, 1);
                }
                bels.push(bel);
            }
            for i in 0..4 {
                let abcd = ['A', 'B', 'C', 'D'][i];
                let mut bel = builder
                    .bel_xy(bels::PHASER_OUT[i], "PHASER_OUT_PHY", 0, i % 2)
                    .raw_tile(1 + i / 2)
                    .pins_name_only(&[
                        "MEMREFCLK",
                        "FREQREFCLK",
                        "SYNCIN",
                        "OCLK",
                        "OCLKDELAYED",
                        "OCLKDIV",
                        "RDENABLE",
                    ])
                    .extra_wire("IO_OCLK", &[format!("CMT_PHASER_OUT_{abcd}_OCLK")])
                    .extra_wire("IO_OCLK90", &[format!("CMT_PHASER_OUT_{abcd}_OCLK1X_90")])
                    .extra_wire("IO_OCLKDIV", &[format!("CMT_PHASER_OUT_{abcd}_OCLKDIV")])
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
                for pin in [
                    "ENCALIBPHY0",
                    "ENCALIBPHY1",
                    "BURSTPENDINGPHY",
                    "PHASEREFCLK",
                ] {
                    bel = bel.pin_name_only(pin, 1);
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy(bels::PHASER_REF, "PHASER_REF", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["CLKIN"])
                    .pin_name_only("CLKOUT", 1)
                    .pin_name_only("TMUXOUT", 1),
            );
            let mut bel_pc = builder
                .bel_xy(bels::PHY_CONTROL, "PHY_CONTROL", 0, 0)
                .raw_tile(2)
                .pins_name_only(&["MEMREFCLK", "SYNCIN"])
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
                "PHYCTLEMPTY",
                "PHYCTLMSTREMPTY",
            ] {
                bel_pc = bel_pc.pin_name_only(pin, 1);
            }
            bels.push(bel_pc);
            let mut bel_mmcm = builder
                .bel_xy(bels::MMCM0, "MMCME2_ADV", 0, 0)
                .raw_tile(0)
                .pins_name_only(&[
                    "CLKIN1",
                    "CLKIN2",
                    "CLKFBIN",
                    "CLKFBOUT",
                    "CLKFBOUTB",
                    "CLKOUT0",
                    "CLKOUT0B",
                    "CLKOUT1",
                    "CLKOUT1B",
                    "CLKOUT2",
                    "CLKOUT2B",
                    "CLKOUT3",
                    "CLKOUT3B",
                    "CLKOUT3",
                    "CLKOUT4",
                    "CLKOUT5",
                    "CLKOUT6",
                    "TMUXOUT",
                ])
                .extra_wire("CLKFB", &["CMT_LR_LOWER_B_CLKFBOUT2IN"])
                .extra_wire(
                    "CLKIN1_HCLK",
                    &["CMT_L_LOWER_B_CLK_IN1_HCLK", "CMT_R_LOWER_B_CLK_IN1_HCLK"],
                )
                .extra_int_in(
                    "CLKIN1_CKINT",
                    &["CMT_L_LOWER_B_CLK_IN1_INT", "CMT_R_LOWER_B_CLK_IN1_INT"],
                )
                .extra_wire(
                    "CLKIN2_HCLK",
                    &["CMT_L_LOWER_B_CLK_IN2_HCLK", "CMT_R_LOWER_B_CLK_IN2_HCLK"],
                )
                .extra_int_in(
                    "CLKIN2_CKINT",
                    &["CMT_L_LOWER_B_CLK_IN2_INT", "CMT_R_LOWER_B_CLK_IN2_INT"],
                )
                .extra_wire(
                    "CLKFBIN_HCLK",
                    &["CMT_L_LOWER_B_CLK_IN3_HCLK", "CMT_R_LOWER_B_CLK_IN3_HCLK"],
                )
                .extra_int_in(
                    "CLKFBIN_CKINT",
                    &["CMT_L_LOWER_B_CLK_IN3_INT", "CMT_R_LOWER_B_CLK_IN3_INT"],
                );
            for i in 0..4 {
                bel_mmcm = bel_mmcm
                    .extra_wire(
                        format!("PERF{i}"),
                        &[
                            format!("CMT_L_LOWER_B_CLK_PERF{i}"),
                            format!("CMT_R_LOWER_B_CLK_PERF{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("FREQ_BB{i}_IN"),
                        &[
                            format!("CMT_L_LOWER_B_CLK_FREQ_BB{ii}", ii = i ^ 3),
                            format!("CMT_R_LOWER_B_CLK_FREQ_BB{ii}", ii = i ^ 3),
                        ],
                    )
                    .extra_wire(
                        format!("FREQ_BB_OUT{i}"),
                        &[format!("MMCMOUT_CLK_FREQ_BB_{i}")],
                    );
            }
            for i in 0..14 {
                bel_mmcm = bel_mmcm.extra_wire(
                    format!("OUT{i}"),
                    &[
                        format!("CMT_L_LOWER_B_CLK_MMCM{i}"),
                        format!("CMT_R_LOWER_B_CLK_MMCM{i}"),
                    ],
                );
            }
            bels.push(bel_mmcm);
            let mut bel_pll = builder
                .bel_xy(bels::PLL, "PLLE2_ADV", 0, 0)
                .raw_tile(3)
                .pins_name_only(&[
                    "CLKIN1", "CLKIN2", "CLKFBIN", "CLKFBOUT", "CLKOUT0", "CLKOUT1", "CLKOUT2",
                    "CLKOUT3", "CLKOUT4", "CLKOUT5", "TMUXOUT",
                ])
                .extra_wire("CLKFB", &["CMT_TOP_L_CLKFBOUT2IN", "CMT_TOP_R_CLKFBOUT2IN"])
                .extra_wire(
                    "CLKIN1_HCLK",
                    &["CMT_TOP_L_UPPER_T_CLKIN1", "CMT_TOP_R_UPPER_T_CLKIN1"],
                )
                .extra_int_in(
                    "CLKIN1_CKINT",
                    &[
                        "CMT_TOP_L_UPPER_T_PLLE2_CLK_IN1_INT",
                        "CMT_TOP_R_UPPER_T_PLLE2_CLK_IN1_INT",
                    ],
                )
                .extra_wire(
                    "CLKIN2_HCLK",
                    &["CMT_TOP_L_UPPER_T_CLKIN2", "CMT_TOP_R_UPPER_T_CLKIN2"],
                )
                .extra_int_in(
                    "CLKIN2_CKINT",
                    &[
                        "CMT_TOP_L_UPPER_T_PLLE2_CLK_IN2_INT",
                        "CMT_TOP_R_UPPER_T_PLLE2_CLK_IN2_INT",
                    ],
                )
                .extra_wire(
                    "CLKFBIN_HCLK",
                    &["CMT_TOP_L_UPPER_T_CLKFBIN", "CMT_TOP_R_UPPER_T_CLKFBIN"],
                )
                .extra_int_in(
                    "CLKFBIN_CKINT",
                    &[
                        "CMT_TOP_L_UPPER_T_PLLE2_CLK_FB_INT",
                        "CMT_TOP_R_UPPER_T_PLLE2_CLK_FB_INT",
                    ],
                );
            for i in 0..4 {
                bel_pll = bel_pll
                    .extra_wire(
                        format!("FREQ_BB{i}_IN"),
                        &[
                            format!("CMT_TOP_L_UPPER_T_FREQ_BB{i}"),
                            format!("CMT_TOP_R_UPPER_T_FREQ_BB{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("FREQ_BB_OUT{i}"),
                        &[format!("PLLOUT_CLK_FREQ_BB_{i}")],
                    );
            }
            for i in 0..8 {
                bel_pll = bel_pll.extra_wire(
                    format!("OUT{i}"),
                    &[
                        format!("CMT_TOP_L_UPPER_T_CLKPLL{i}"),
                        format!("CMT_TOP_R_UPPER_T_CLKPLL{i}"),
                    ],
                );
            }
            bels.push(bel_pll);
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(bels::BUFMRCE[i], "BUFMRCE", 0, i)
                        .raw_tile(4)
                        .pins_name_only(&["I", "O"]),
                );
            }
            let mut bel = builder
                .bel_virtual(bels::CMT_A)
                .raw_tile(0)
                .extra_wire_force("SYNC_BB", "CMT_MMCM_PHYCTRL_SYNC_BB_UP")
                .extra_wire_force("SYNC_BB_S", "CMT_MMCM_PHYCTRL_SYNC_BB_DN")
                .extra_wire("IO8_OCLK90", &["CMT_TOP_OCLK1X_90_8"])
                .extra_wire("PHASER_A_ICLK", &["CMT_MMCM_PHASER_IN_A_ICLK"])
                .extra_wire("PHASER_A_ICLKDIV", &["CMT_MMCM_PHASER_IN_A_ICLKDIV"])
                .extra_wire("PHASER_A_OCLK", &["CMT_MMCM_PHASER_OUT_A_OCLK"])
                .extra_wire("PHASER_A_OCLK90", &["CMT_MMCM_PHASER_OUT_A_OCLK1X_90"])
                .extra_wire("PHASER_A_OCLKDIV", &["CMT_MMCM_PHASER_OUT_A_OCLKDIV"])
                .extra_wire("PHASER_A_ICLK_BUF", &["CMT_PHASER_A_ICLK_TOIOI"])
                .extra_wire("PHASER_A_ICLKDIV_BUF", &["CMT_PHASER_A_ICLKDIV_TOIOI"])
                .extra_wire("PHASER_A_OCLK_BUF", &["CMT_PHASER_A_OCLK_TOIOI"])
                .extra_wire("PHASER_A_OCLK90_BUF", &["CMT_PHASER_A_OCLK90_TOIOI"])
                .extra_wire("PHASER_A_OCLKDIV_BUF", &["CMT_PHASER_A_OCLKDIV_TOIOI"])
                .extra_wire("PHASER_B_ICLK", &["CMT_MMCM_PHASER_IN_B_ICLK"])
                .extra_wire("PHASER_B_ICLKDIV", &["CMT_MMCM_PHASER_IN_B_ICLKDIV"])
                .extra_wire("PHASER_B_OCLK", &["CMT_MMCM_PHASER_OUT_B_OCLK"])
                .extra_wire("PHASER_B_OCLKDIV", &["CMT_MMCM_PHASER_OUT_B_OCLKDIV"]);
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("FREQ_BB{i}"), &[format!("MMCM_CLK_FREQ_BB_NS{i}")])
                    .extra_wire_force(
                        format!("FREQ_BB{i}_S"),
                        format!("MMCM_CLK_FREQ_BB_REBUF{i}_NS"),
                    );
            }
            for i in 0..16 {
                bel = bel
                    .extra_wire(format!("IO{i}_ICLK"), &[format!("CMT_TOP_ICLK_{i}")])
                    .extra_wire(format!("IO{i}_ICLKDIV"), &[format!("CMT_TOP_ICLKDIV_{i}")])
                    .extra_wire(format!("IO{i}_OCLK"), &[format!("CMT_TOP_OCLK_{i}")])
                    .extra_wire(format!("IO{i}_OCLKDIV"), &[format!("CMT_TOP_OCLKDIV_{i}")])
            }
            bels.push(bel);
            let mut bel = builder
                .bel_virtual(bels::CMT_B)
                .raw_tile(1)
                .extra_wire("FREQREFCLK", &["CMT_PHASER_BOT_REFMUX_0"])
                .extra_wire("MEMREFCLK", &["CMT_PHASER_BOT_REFMUX_1"])
                .extra_wire("SYNCIN", &["CMT_PHASER_BOT_REFMUX_2"])
                .extra_wire("IO20_OCLK90", &["CMT_TOP_OCLK1X_90_4"])
                .extra_wire("PHASER_B_ICLK_BUF", &["CMT_PHASER_B_ICLK_TOIOI"])
                .extra_wire("PHASER_B_ICLKDIV_BUF", &["CMT_PHASER_B_ICLKDIV_TOIOI"])
                .extra_wire("PHASER_B_OCLK_BUF", &["CMT_PHASER_B_OCLK_TOIOI"])
                .extra_wire("PHASER_B_OCLK90_BUF", &["CMT_PHASER_B_OCLK90_TOIOI"])
                .extra_wire("PHASER_B_OCLKDIV_BUF", &["CMT_PHASER_B_OCLKDIV_TOIOI"])
                .extra_wire("PHASER_B_ICLK_A", &["CMT_PHASER_B_TOMMCM_ICLK"])
                .extra_wire("PHASER_B_ICLKDIV_A", &["CMT_PHASER_B_TOMMCM_ICLKDIV"])
                .extra_wire("PHASER_B_OCLK_A", &["CMT_PHASER_B_TOMMCM_OCLK"])
                .extra_wire("PHASER_B_OCLKDIV_A", &["CMT_PHASER_B_TOMMCM_OCLKDIV"]);
            for i in 0..2 {
                bel = bel
                    .extra_wire(
                        format!("MRCLK{i}"),
                        &[format!("CMT_PHASER_DOWN_PHASERREF{i}")],
                    )
                    .extra_wire(
                        format!("MRCLK{i}_S"),
                        &[format!("CMT_PHASER_DOWN_PHASERREF_ABOVE{i}")],
                    )
                    .extra_wire(
                        format!("MRCLK{i}_N"),
                        &[format!("CMT_PHASER_DOWN_PHASERREF_BELOW{i}")],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(
                        format!("FREQ_BB{i}"),
                        &[format!("MMCM_CLK_FREQBB_REBUFOUT{i}")],
                    )
                    .extra_wire(
                        format!("FREQ_BB{i}_MUX"),
                        &[format!("MMCMOUT_CLK_FREQ_BB_REBUFOUT{i}")],
                    )
                    .extra_wire(
                        format!("MMCM_FREQ_BB{i}"),
                        &[format!("MMCMOUT_CLK_FREQ_BB_REBUFIN{i}")],
                    );
            }
            for i in 16..25 {
                let ii = i - 16;
                bel = bel
                    .extra_wire(format!("IO{i}_ICLK"), &[format!("CMT_TOP_ICLK_{ii}")])
                    .extra_wire(format!("IO{i}_ICLKDIV"), &[format!("CMT_TOP_ICLKDIV_{ii}")])
                    .extra_wire(format!("IO{i}_OCLK"), &[format!("CMT_TOP_OCLK_{ii}")])
                    .extra_wire(format!("IO{i}_OCLKDIV"), &[format!("CMT_TOP_OCLKDIV_{ii}")])
            }
            bels.push(bel);
            let mut bel = builder
                .bel_virtual(bels::CMT_C)
                .raw_tile(2)
                .extra_wire("FREQREFCLK", &["CMT_FREQ_PHASER_REFMUX_0"])
                .extra_wire("MEMREFCLK", &["CMT_FREQ_PHASER_REFMUX_1"])
                .extra_wire("SYNCIN", &["CMT_FREQ_PHASER_REFMUX_2"])
                .extra_wire("IO32_OCLK90", &["CMT_TOP_OCLK1X_90_7"])
                .extra_wire("PHASER_C_ICLK_BUF", &["CMT_PHASER_C_ICLK_TOIOI"])
                .extra_wire("PHASER_C_ICLKDIV_BUF", &["CMT_PHASER_C_ICLKDIV_TOIOI"])
                .extra_wire("PHASER_C_OCLK_BUF", &["CMT_PHASER_C_OCLK_TOIOI"])
                .extra_wire("PHASER_C_OCLK90_BUF", &["CMT_PHASER_C_OCLK90_TOIOI"])
                .extra_wire("PHASER_C_OCLKDIV_BUF", &["CMT_PHASER_C_OCLKDIV_TOIOI"]);
            for i in 0..2 {
                bel = bel
                    .extra_wire(
                        format!("MRCLK{i}"),
                        &[format!("CMT_PHASER_UP_PHASERREF{i}")],
                    )
                    .extra_wire(
                        format!("MRCLK{i}_S"),
                        &[format!("CMT_PHASER_UP_PHASERREF_ABOVE{i}")],
                    )
                    .extra_wire(
                        format!("MRCLK{i}_N"),
                        &[format!("CMT_PHASER_UP_PHASERREF_BELOW{i}")],
                    );
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(
                        format!("FREQ_BB{i}"),
                        &[format!("PLL_CLK_FREQBB_REBUFOUT{i}")],
                    )
                    .extra_wire(
                        format!("FREQ_BB{i}_MUX"),
                        &[format!("PLLOUT_CLK_FREQ_BB_REBUFOUT{i}")],
                    )
                    .extra_wire(
                        format!("FREQ_BB{i}_REF"),
                        &[format!("CMT_FREQ_BB_PREF_IN{i}")],
                    )
                    .extra_wire(
                        format!("PLL_FREQ_BB{i}"),
                        &[format!("PLLOUT_CLK_FREQ_BB_REBUFIN{i}")],
                    );
            }
            for i in 25..37 {
                let ii = i - 25;
                bel = bel
                    .extra_wire(format!("IO{i}_ICLK"), &[format!("CMT_TOP_ICLK_{ii}")])
                    .extra_wire(format!("IO{i}_ICLKDIV"), &[format!("CMT_TOP_ICLKDIV_{ii}")])
                    .extra_wire(format!("IO{i}_OCLK"), &[format!("CMT_TOP_OCLK_{ii}")])
                    .extra_wire(format!("IO{i}_OCLKDIV"), &[format!("CMT_TOP_OCLKDIV_{ii}")])
            }
            bels.push(bel);
            let mut bel = builder
                .bel_virtual(bels::CMT_D)
                .raw_tile(3)
                .extra_wire_force("SYNC_BB", "CMT_PLL_PHYCTRL_SYNC_BB_DN")
                .extra_wire_force("SYNC_BB_N", "CMT_PLL_PHYCTRL_SYNC_BB_UP")
                .extra_wire("IO44_OCLK90", &["CMT_TOP_OCLK1X_90_7"])
                .extra_wire("PHASER_D_ICLK_BUF", &["CMT_PHASER_D_ICLK_TOIOI"])
                .extra_wire("PHASER_D_ICLKDIV_BUF", &["CMT_PHASER_D_ICLKDIV_TOIOI"])
                .extra_wire("PHASER_D_OCLK_BUF", &["CMT_PHASER_D_OCLK_TOIOI"])
                .extra_wire("PHASER_D_OCLK90_BUF", &["CMT_PHASER_D_OCLK90_TOIOI"])
                .extra_wire("PHASER_D_OCLKDIV_BUF", &["CMT_PHASER_D_OCLKDIV_TOIOI"])
                .extra_wire("PHASER_D_ICLK", &["CMT_PLL_PHASER_IN_D_ICLK"])
                .extra_wire("PHASER_D_ICLKDIV", &["CMT_PLL_PHASER_IN_D_ICLKDIV"])
                .extra_wire("PHASER_D_OCLK", &["CMT_PLL_PHASER_OUT_D_OCLK"])
                .extra_wire("PHASER_D_OCLK90", &["CMT_PLL_PHASER_OUT_D_OCLK1X_90"])
                .extra_wire("PHASER_D_OCLKDIV", &["CMT_PLL_PHASER_OUT_D_OCLKDIV"]);
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("FREQ_BB{i}"), &[format!("PLL_CLK_FREQ_BB{i}_NS")])
                    .extra_wire_force(
                        format!("FREQ_BB{i}_N"),
                        format!("PLL_CLK_FREQ_BB_BUFOUT_NS{i}"),
                    );
            }
            for i in 37..50 {
                let ii = i - 37;
                bel = bel
                    .extra_wire(format!("IO{i}_ICLK"), &[format!("CMT_TOP_ICLK_{ii}")])
                    .extra_wire(format!("IO{i}_ICLKDIV"), &[format!("CMT_TOP_ICLKDIV_{ii}")])
                    .extra_wire(format!("IO{i}_OCLK"), &[format!("CMT_TOP_OCLK_{ii}")])
                    .extra_wire(format!("IO{i}_OCLKDIV"), &[format!("CMT_TOP_OCLKDIV_{ii}")])
            }
            bels.push(bel);
            let mut bel = builder
                .bel_virtual(bels::HCLK_CMT)
                .raw_tile(4)
                .extra_wire("MMCM_CLKIN1", &["HCLK_CMT_MUX_MMCM_CLKIN1"])
                .extra_wire("MMCM_CLKIN2", &["HCLK_CMT_MUX_MMCM_CLKIN2"])
                .extra_wire("MMCM_CLKFBIN", &["HCLK_CMT_MUX_MMCM_CLKFBIN"])
                .extra_wire("PLL_CLKIN1", &["HCLK_CMT_MUX_PLLE2_CLKIN1"])
                .extra_wire("PLL_CLKIN2", &["HCLK_CMT_MUX_PLLE2_CLKIN2"])
                .extra_wire("PLL_CLKFBIN", &["HCLK_CMT_MUX_PLLE2_CLKFBIN"])
                .extra_wire("PHASER_REF_CLKOUT", &["HCLK_CMT_PREF_CLKOUT"])
                .extra_wire("PHASER_REF_TMUXOUT", &["HCLK_CMT_PREF_TMUXOUT"]);
            for i in 0..12 {
                bel = bel.extra_wire(format!("HCLK{i}"), &[format!("HCLK_CMT_CK_BUFHCLK{i}")]);
            }
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("RCLK{i}"), &[format!("HCLK_CMT_CK_BUFRCLK{i}")])
                    .extra_wire(format!("CCIO{i}"), &[format!("HCLK_CMT_CCIO{i}")])
                    .extra_wire(format!("FREQ_BB{i}"), &[format!("HCLK_CMT_FREQ_REF_NS{i}")])
                    .extra_wire(
                        format!("FREQ_BB{i}_MUX"),
                        &[format!("HCLK_CMT_MUX_OUT_FREQ_REF{i}")],
                    )
                    .extra_int_in(format!("CKINT{i}"), &[format!("HCLK_CMT_MUX_CLKINT_{i}")])
                    .extra_wire(
                        format!("PHASER_IN_RCLK{i}"),
                        &[format!("HCLK_CMT_PHASERIN_RCLK{i}")],
                    )
                    .extra_wire(
                        format!("PERF{i}"),
                        &[format!("HCLK_CMT_MUX_PHSR_PERFCLK{i}")],
                    )
                    .extra_wire(
                        format!("MMCM_PERF{i}"),
                        &[format!("HCLK_CMT_MUX_MMCM_MUXED{i}")],
                    )
                    .extra_wire(
                        format!("PHASER_REF_BOUNCE{i}"),
                        &[format!("HCLK_CMT_PREF_BOUNCE{i}")],
                    );
            }
            for i in 0..2 {
                bel = bel.extra_wire(
                    format!("MRCLK{i}"),
                    &[format!("HCLK_CMT_BUFMR_PHASEREF{i}")],
                );
            }
            for i in 0..14 {
                bel = bel.extra_wire(
                    format!("HOUT{i}"),
                    &[if is_l {
                        format!("HCLK_CMT_CK_IN{i}")
                    } else {
                        format!("HCLK_CMT_MUX_CLK_{i}")
                    }],
                );
            }
            for i in 4..14 {
                bel = bel.extra_wire_force(
                    format!("HIN{i}"),
                    if is_l {
                        format!("HCLK_CMT_MUX_CLK_{i}")
                    } else {
                        format!("HCLK_CMT_CK_IN{i}")
                    },
                );
            }
            for i in 0..2 {
                bel = bel
                    .extra_wire(
                        format!("LCLK{i}_CMT_D"),
                        &[format!("HCLK_CMT_MUX_CLK_LEAF_DN{i}")],
                    )
                    .extra_wire(
                        format!("LCLK{i}_CMT_U"),
                        &[format!("HCLK_CMT_MUX_CLK_LEAF_UP{i}")],
                    );
            }
            for i in 0..14 {
                bel = bel.extra_wire(
                    format!("MMCM_OUT{i}"),
                    &[format!("HCLK_CMT_MUX_CLK_MMCM{i}")],
                )
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("PLL_OUT{i}"), &[format!("HCLK_CMT_MUX_CLK_PLL{i}")])
            }
            bels.push(bel);
            let mut xn = builder
                .xnode("CMT", if is_l { "CMT.L" } else { "CMT.R" }, xy)
                .num_tiles(50)
                .raw_tile(xy.delta(0, 9))
                .raw_tile(xy.delta(0, 22))
                .raw_tile(xy.delta(0, 35))
                .raw_tile(xy.delta(0, 17));
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
        }
    }

    for tkn in ["CMT_FIFO_L", "CMT_FIFO_R"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = tkn == "CMT_FIFO_L";
            let int_xy = xy.delta(if is_l { 2 } else { -2 }, -6);
            let intf_xy = xy.delta(if is_l { 1 } else { -1 }, -6);
            let intf = builder
                .ndb
                .get_tile_class_naming(if is_l { "INTF.L" } else { "INTF.R" });
            let bels = [
                builder
                    .bel_xy(bels::IN_FIFO, "IN_FIFO", 0, 0)
                    .extra_wire("PHASER_WRCLK", &["CMT_FIFO_L_PHASER_WRCLK"])
                    .extra_wire("PHASER_WREN", &["CMT_FIFO_L_PHASER_WRENABLE"]),
                builder
                    .bel_xy(bels::OUT_FIFO, "OUT_FIFO", 0, 0)
                    .extra_wire("PHASER_RDCLK", &["CMT_FIFO_L_PHASER_RDCLK"])
                    .extra_wire("PHASER_RDEN", &["CMT_FIFO_L_PHASER_RDENABLE"]),
            ];
            let mut xn = builder.xnode("CMT_FIFO", tkn, xy).num_tiles(12);
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

    if let Some(&xy_m) = rd.tiles_by_kind_name("CFG_CENTER_MID").iter().next() {
        let xy_b = xy_m.delta(0, -21);
        let xy_t = xy_m.delta(0, 10);
        let intf = builder.ndb.get_tile_class_naming("INTF.L");
        let bels = [
            builder.bel_xy(bels::BSCAN0, "BSCAN", 0, 0).raw_tile(1),
            builder.bel_xy(bels::BSCAN1, "BSCAN", 0, 1).raw_tile(1),
            builder.bel_xy(bels::BSCAN2, "BSCAN", 0, 2).raw_tile(1),
            builder.bel_xy(bels::BSCAN3, "BSCAN", 0, 3).raw_tile(1),
            builder.bel_xy(bels::ICAP0, "ICAP", 0, 0).raw_tile(1),
            builder.bel_xy(bels::ICAP1, "ICAP", 0, 1).raw_tile(1),
            builder.bel_xy(bels::STARTUP, "STARTUP", 0, 0).raw_tile(1),
            builder.bel_xy(bels::CAPTURE, "CAPTURE", 0, 0).raw_tile(1),
            builder
                .bel_xy(bels::FRAME_ECC, "FRAME_ECC", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(bels::USR_ACCESS, "USR_ACCESS", 0, 0)
                .raw_tile(1),
            builder
                .bel_xy(bels::CFG_IO_ACCESS, "CFG_IO_ACCESS", 0, 0)
                .raw_tile(1),
            builder.bel_xy(bels::PMVIOB, "PMVIOB", 0, 0).raw_tile(1),
            builder.bel_xy(bels::DCIRESET, "DCIRESET", 0, 0).raw_tile(1),
            builder.bel_xy(bels::DNA_PORT, "DNA_PORT", 0, 0).raw_tile(2),
            builder
                .bel_xy(bels::EFUSE_USR, "EFUSE_USR", 0, 0)
                .raw_tile(2),
        ];
        let mut xn = builder
            .xnode("CFG", "CFG", xy_b)
            .raw_tile(xy_m)
            .raw_tile(xy_t)
            .num_tiles(50);
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

    for (tkn, naming) in [
        ("MONITOR_BOT", "SYSMON.LR"),
        ("MONITOR_BOT_FUJI2", "SYSMON.L"),
        ("MONITOR_BOT_PELE1", "SYSMON.R"),
    ] {
        if let Some(&xy_b) = rd.tiles_by_kind_name(tkn).iter().next() {
            let xy_m = xy_b.delta(0, 10);
            let xy_t = xy_b.delta(0, 20);
            let intf = builder.ndb.get_tile_class_naming("INTF.L");
            let mut bel_xadc = builder
                .bel_xy(bels::SYSMON, "XADC", 0, 0)
                .pins_name_only(&["VP", "VN"]);
            for i in 0..16 {
                if naming == "SYSMON.L" && matches!(i, 6 | 7 | 13 | 14 | 15) {
                    bel_xadc = bel_xadc
                        .pin_dummy(format!("VAUXP{i}"))
                        .pin_dummy(format!("VAUXN{i}"));
                } else {
                    bel_xadc = bel_xadc
                        .pin_name_only(&format!("VAUXP{i}"), 2)
                        .pin_name_only(&format!("VAUXN{i}"), 2);
                }
            }
            let bels = [
                builder
                    .bel_xy(bels::IPAD_VP, "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_VN, "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                bel_xadc,
            ];
            let mut xn = builder
                .xnode("SYSMON", naming, xy_b)
                .raw_tile(xy_m)
                .raw_tile(xy_t)
                .num_tiles(25);
            if naming == "SYSMON.R" {
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
            xn.bels(bels).extract();
        }
    }

    if let Some(&xy_pss0) = rd.tiles_by_kind_name("PSS0").iter().next() {
        let int_xy = xy_pss0.delta(19, -10);
        let xy_pss1 = xy_pss0.delta(0, 21);
        let xy_pss2 = xy_pss0.delta(0, 42);
        let xy_pss3 = xy_pss0.delta(0, 62);
        let xy_pss4 = xy_pss0.delta(0, 83);
        let intf = builder.ndb.get_tile_class_naming("INTF.PSS");
        let mut pins = vec![];
        pins.push((bels::IOPAD_DDRWEB, 1));
        pins.push((bels::IOPAD_DDRVRN, 2));
        pins.push((bels::IOPAD_DDRVRP, 3));
        for i in 0..13 {
            pins.push((bels::IOPAD_DDRA[i], 4 + i));
        }
        pins.push((bels::IOPAD_DDRA14, 17));
        pins.push((bels::IOPAD_DDRA13, 18));
        for i in 0..3 {
            pins.push((bels::IOPAD_DDRBA[i], 19 + i));
        }
        pins.push((bels::IOPAD_DDRCASB, 22));
        pins.push((bels::IOPAD_DDRCKE, 23));
        pins.push((bels::IOPAD_DDRCKN, 24));
        pins.push((bels::IOPAD_DDRCKP, 25));
        pins.push((bels::IOPAD_PSCLK, 26));
        pins.push((bels::IOPAD_DDRCSB, 27));
        for i in 0..4 {
            pins.push((bels::IOPAD_DDRDM[i], 28 + i));
        }
        for i in 0..32 {
            pins.push((bels::IOPAD_DDRDQ[i], 32 + i));
        }
        for i in 0..4 {
            pins.push((bels::IOPAD_DDRDQSN[i], 64 + i));
        }
        for i in 0..4 {
            pins.push((bels::IOPAD_DDRDQSP[i], 68 + i));
        }
        pins.push((bels::IOPAD_DDRDRSTB, 72));
        for i in 0..54 {
            pins.push((bels::IOPAD_MIO[i], 77 + i));
        }
        pins.push((bels::IOPAD_DDRODT, 131));
        pins.push((bels::IOPAD_PSPORB, 132));
        pins.push((bels::IOPAD_DDRRASB, 133));
        pins.push((bels::IOPAD_PSSRSTB, 134));
        let mut bel_ps = builder
            .bel_xy(bels::PS, "PS7", 0, 0)
            .raw_tile(2)
            .pins_name_only(&["FCLKCLK0", "FCLKCLK1", "FCLKCLK2", "FCLKCLK3"])
            .extra_int_out("FCLKCLK0_INT", &["PSS1_LOGIC_OUTS1_39"])
            .extra_int_out("FCLKCLK1_INT", &["PSS1_LOGIC_OUTS2_39"])
            .extra_int_out("FCLKCLK2_INT", &["PSS2_LOGIC_OUTS0_61"])
            .extra_int_out("FCLKCLK3_INT", &["PSS2_LOGIC_OUTS1_61"])
            .extra_wire("FCLKCLK0_HOUT", &["PSS_FCLKCLK0"])
            .extra_wire("FCLKCLK1_HOUT", &["PSS_FCLKCLK1"])
            .extra_wire("FCLKCLK2_HOUT", &["PSS2_FCLKCLK2"])
            .extra_wire("FCLKCLK3_HOUT", &["PSS2_FCLKCLK3"]);
        for pin in [
            "TESTPLLNEWCLK0",
            "TESTPLLNEWCLK1",
            "TESTPLLNEWCLK2",
            "TESTPLLCLKOUT0",
            "TESTPLLCLKOUT1",
            "TESTPLLCLKOUT2",
        ] {
            bel_ps = bel_ps.pin_name_only(pin, 1);
        }
        for &(slot, _) in &pins {
            let pin = builder.db.bel_slots[slot].strip_prefix("IOPAD_").unwrap();
            bel_ps = bel_ps.pins_name_only(&[pin]);
        }
        let mut bels = vec![bel_ps];
        for &(slot, y) in &pins {
            bels.push(
                builder
                    .bel_xy(slot, "IOPAD", 0, y - 1)
                    .raw_tile(2)
                    .pins_name_only(&["IO"]),
            );
        }
        let mut bel_lo = builder.bel_virtual(bels::HCLK_PS_S).raw_tile(1);
        for i in 0..4 {
            bel_lo = bel_lo
                .extra_wire(format!("FCLKCLK{i}"), &[format!("PSS_FCLKCLK{i}")])
                .extra_wire(format!("HOUT{i}"), &[format!("PSS_HCLK_CK_IN{i}")])
        }
        let mut bel_hi = builder.bel_virtual(bels::HCLK_PS_N).raw_tile(3);
        for i in 0..3 {
            bel_hi = bel_hi
                .extra_wire(
                    format!("TESTPLLNEWCLK{i}"),
                    &[format!("PSS3_TESTPLLNEWCLK{i}_IN")],
                )
                .extra_wire(format!("HOUT{i}"), &[format!("PSS3_TESTPLLNEWCLK{i}_OUT")])
                .extra_wire(
                    format!("TESTPLLCLKOUT{i}"),
                    &[format!("PSS3_TESTPLLCLKOUT{i}_IN")],
                )
                .extra_wire(
                    format!("HOUT{ii}", ii = i + 3),
                    &[format!("PSS3_TESTPLLCLKOUT{i}_OUT")],
                )
        }
        bels.extend([bel_lo, bel_hi]);
        let mut xn = builder
            .xnode("PS", "PS", xy_pss0)
            .raw_tile(xy_pss1)
            .raw_tile(xy_pss2)
            .raw_tile(xy_pss3)
            .raw_tile(xy_pss4)
            .num_tiles(100);
        for i in 0..4 {
            for j in 0..25 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i * 26 + j) as i32), i * 25 + j)
                    .ref_single(int_xy.delta(-1, (i * 26 + j) as i32), i * 25 + j, intf);
            }
        }
        xn.bels(bels).extract();
    }

    for (nn, tkn, int_dx, intf_dx, intf_kind) in [
        ("GTP_CHANNEL", "GTP_CHANNEL_0", -4, -3, "INTF.GTP"),
        ("GTP_CHANNEL", "GTP_CHANNEL_1", -4, -3, "INTF.GTP"),
        ("GTP_CHANNEL", "GTP_CHANNEL_2", -4, -3, "INTF.GTP"),
        ("GTP_CHANNEL", "GTP_CHANNEL_3", -4, -3, "INTF.GTP"),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_0_MID_LEFT",
            -14,
            -13,
            "INTF.GTP_R",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_1_MID_LEFT",
            -14,
            -13,
            "INTF.GTP_R",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_2_MID_LEFT",
            -14,
            -13,
            "INTF.GTP_R",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_3_MID_LEFT",
            -14,
            -13,
            "INTF.GTP_R",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_0_MID_RIGHT",
            19,
            18,
            "INTF.GTP_L",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_1_MID_RIGHT",
            19,
            18,
            "INTF.GTP_L",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_2_MID_RIGHT",
            19,
            18,
            "INTF.GTP_L",
        ),
        (
            "GTP_CHANNEL_MID",
            "GTP_CHANNEL_3_MID_RIGHT",
            19,
            18,
            "INTF.GTP_L",
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming(intf_kind);
            let bels = [
                builder
                    .bel_xy(bels::GTP_CHANNEL, "GTPE2_CHANNEL", 0, 0)
                    .pins_name_only(&[
                        "GTPRXP", "GTPRXN", "GTPTXP", "GTPTXN", "RXOUTCLK", "TXOUTCLK",
                    ])
                    .pin_name_only("PLL0CLK", 1)
                    .pin_name_only("PLL1CLK", 1)
                    .pin_name_only("PLL0REFCLK", 1)
                    .pin_name_only("PLL1REFCLK", 1)
                    .pin_name_only("RXOUTCLK", 1)
                    .pin_name_only("TXOUTCLK", 1),
                builder
                    .bel_xy(bels::IPAD_RXP0, "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_RXN0, "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::OPAD_TXP0, "OPAD", 0, 1)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy(bels::OPAD_TXN0, "OPAD", 0, 0)
                    .pins_name_only(&["I"]),
            ];
            let mut xn = builder.xnode(nn, tkn, xy).num_tiles(11);
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
    for (nn, tkn, int_dx, intf_dx, intf_kind) in [
        ("GTP_COMMON", "GTP_COMMON", -4, -3, "INTF.GTP"),
        (
            "GTP_COMMON_MID",
            "GTP_COMMON_MID_LEFT",
            -14,
            -13,
            "INTF.GTP_R",
        ),
        (
            "GTP_COMMON_MID",
            "GTP_COMMON_MID_RIGHT",
            19,
            18,
            "INTF.GTP_L",
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let intf = builder.ndb.get_tile_class_naming(intf_kind);
            let mut bel = builder
                .bel_xy(bels::GTP_COMMON, "GTPE2_COMMON", 0, 0)
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
            if tkn == "GTP_COMMON" {
                for i in 0..10 {
                    bel = bel.extra_wire(
                        format!("HOUT{ii}", ii = i + 4),
                        &[format!("GTPE2_COMMON_MGT_CLK{i}")],
                    );
                }
            } else {
                bel = bel
                    .extra_wire("MGTCLKOUT0_BUF", &["IBUFDS_GTPE2_0_MGTCLKOUT_MUX"])
                    .extra_wire("MGTCLKOUT1_BUF", &["IBUFDS_GTPE2_1_MGTCLKOUT_MUX"]);
                for i in 0..4 {
                    bel = bel
                        .extra_wire(
                            format!("RXOUTCLK{i}_BUF"),
                            &[format!("GTPE2_COMMON_RXOUTCLK_MUX_{i}")],
                        )
                        .extra_wire(
                            format!("TXOUTCLK{i}_BUF"),
                            &[format!("GTPE2_COMMON_TXOUTCLK_MUX_{i}")],
                        );
                }
                for i in 0..14 {
                    bel = bel
                        .extra_wire(format!("HOUT{i}"), &[format!("HCLK_GTP_CK_IN{i}")])
                        .extra_wire(format!("HIN{i}"), &[format!("HCLK_GTP_CK_MUX{i}")]);
                }
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

            let bels = [
                bel,
                builder
                    .bel_xy(bels::BUFDS0, "IBUFDS_GTE2", 0, 0)
                    .pins_name_only(&["I", "IB", "O", "ODIV2"])
                    .extra_wire("MGTCLKOUT", &["IBUFDS_GTPE2_0_MGTCLKOUT"]),
                builder
                    .bel_xy(bels::BUFDS1, "IBUFDS_GTE2", 0, 1)
                    .pins_name_only(&["I", "IB", "O", "ODIV2"])
                    .extra_wire("MGTCLKOUT", &["IBUFDS_GTPE2_1_MGTCLKOUT"]),
                builder
                    .bel_xy(bels::IPAD_CLKP0, "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKN0, "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKP1, "IPAD", 0, 2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKN1, "IPAD", 0, 3)
                    .pins_name_only(&["O"]),
            ];
            let mut xn = builder.xnode(nn, tkn, xy).num_tiles(6);
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

    for (tkn, kind, slot, bslot, intf_l_kind, intf_r_kind) in [
        (
            "GTX_CHANNEL_0",
            "GTX_CHANNEL",
            bels::GTX_CHANNEL,
            "GTXE2_CHANNEL",
            "INTF.GTX_L",
            "INTF.GTX",
        ),
        (
            "GTX_CHANNEL_1",
            "GTX_CHANNEL",
            bels::GTX_CHANNEL,
            "GTXE2_CHANNEL",
            "INTF.GTX_L",
            "INTF.GTX",
        ),
        (
            "GTX_CHANNEL_2",
            "GTX_CHANNEL",
            bels::GTX_CHANNEL,
            "GTXE2_CHANNEL",
            "INTF.GTX_L",
            "INTF.GTX",
        ),
        (
            "GTX_CHANNEL_3",
            "GTX_CHANNEL",
            bels::GTX_CHANNEL,
            "GTXE2_CHANNEL",
            "INTF.GTX_L",
            "INTF.GTX",
        ),
        (
            "GTH_CHANNEL_0",
            "GTH_CHANNEL",
            bels::GTH_CHANNEL,
            "GTHE2_CHANNEL",
            "INTF.GTH_L",
            "INTF.GTH",
        ),
        (
            "GTH_CHANNEL_1",
            "GTH_CHANNEL",
            bels::GTH_CHANNEL,
            "GTHE2_CHANNEL",
            "INTF.GTH_L",
            "INTF.GTH",
        ),
        (
            "GTH_CHANNEL_2",
            "GTH_CHANNEL",
            bels::GTH_CHANNEL,
            "GTHE2_CHANNEL",
            "INTF.GTH_L",
            "INTF.GTH",
        ),
        (
            "GTH_CHANNEL_3",
            "GTH_CHANNEL",
            bels::GTH_CHANNEL,
            "GTHE2_CHANNEL",
            "INTF.GTH_L",
            "INTF.GTH",
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_gtx = kind == "GTX_CHANNEL";
            let is_l = xy.x == 0;
            let intf =
                builder
                    .ndb
                    .get_tile_class_naming(if is_l { intf_l_kind } else { intf_r_kind });
            let bels = [
                builder
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
                    .pin_name_only("QPLLREFCLK", 1)
                    .pin_name_only("RXOUTCLK", 1)
                    .pin_name_only("TXOUTCLK", 1),
                builder
                    .bel_xy(bels::IPAD_RXP0, "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_RXN0, "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::OPAD_TXP0, "OPAD", 0, 1)
                    .pins_name_only(&["I"]),
                builder
                    .bel_xy(bels::OPAD_TXN0, "OPAD", 0, 0)
                    .pins_name_only(&["I"]),
            ];
            let mut xn = builder.xnode(kind, tkn, xy).num_tiles(11);
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

    for (tkn, slot, bslot, intf_l_kind, intf_r_kind) in [
        (
            "GTX_COMMON",
            bels::GTX_COMMON,
            "GTXE2_COMMON",
            "INTF.GTX_L",
            "INTF.GTX",
        ),
        (
            "GTH_COMMON",
            bels::GTH_COMMON,
            "GTHE2_COMMON",
            "INTF.GTH_L",
            "INTF.GTH",
        ),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_l = xy.x == 0;
            let intf =
                builder
                    .ndb
                    .get_tile_class_naming(if is_l { intf_l_kind } else { intf_r_kind });
            let mut bel = builder
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
            for i in 0..10 {
                bel = bel.extra_wire(
                    format!("HOUT{ii}", ii = i + 4),
                    &[
                        format!("GTXE2_COMMON_MGT_CLK{i}"),
                        format!("GTHE2_COMMON_MGT_CLK{i}"),
                    ],
                );
            }

            let bels = [
                bel,
                builder
                    .bel_xy(bels::BUFDS0, "IBUFDS_GTE2", 0, 0)
                    .pins_name_only(&["I", "IB", "O", "ODIV2"])
                    .extra_wire(
                        "MGTCLKOUT",
                        &["IBUFDS_GTE2_0_MGTCLKOUT", "IBUFDS_GTHE2_0_MGTCLKOUT"],
                    ),
                builder
                    .bel_xy(bels::BUFDS1, "IBUFDS_GTE2", 0, 1)
                    .pins_name_only(&["I", "IB", "O", "ODIV2"])
                    .extra_wire(
                        "MGTCLKOUT",
                        &["IBUFDS_GTE2_1_MGTCLKOUT", "IBUFDS_GTHE2_1_MGTCLKOUT"],
                    ),
                builder
                    .bel_xy(bels::IPAD_CLKP0, "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKN0, "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKP1, "IPAD", 0, 2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKN1, "IPAD", 0, 3)
                    .pins_name_only(&["O"]),
            ];
            let mut xn = builder.xnode(tkn, tkn, xy).num_tiles(6);
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
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRKH_GTX").iter().next() {
        let bel = builder
            .bel_virtual(bels::BRKH_GTX)
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
        builder
            .xnode("BRKH_GTX", "BRKH_GTX", xy)
            .num_tiles(0)
            .bel(bel)
            .extract();
    }

    builder.build()
}
