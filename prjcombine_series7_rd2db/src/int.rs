use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::{Coord, Part};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("series7", rd);

    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..6 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK_B{i}_EAST"), format!("GCLK_L_B{i}")],
        );
    }
    for i in 6..12 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
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
                builder.find_wire(format!("LH.{}", i)),
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
        .filter_map(|(w, wi)| {
            if wi.name.starts_with("SNG.S") || wi.name.starts_with("SNG.N") {
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
                        .bel_xy("SLICE0", "SLICE", 0, 0)
                        .pin_name_only("CIN", 0)
                        .pin_name_only("COUT", 1),
                    builder
                        .bel_xy("SLICE1", "SLICE", 1, 0)
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
            let n = builder
                .db
                .get_node_naming(if tkn == "BRAM_L" { "INTF.L" } else { "INTF.R" });
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
                .bel_xy("BRAM_F", "RAMB36", 0, 0)
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
            let mut bel_bram_h0 = builder.bel_xy("BRAM_H0", "RAMB18", 0, 0);
            let mut bel_bram_h1 = builder.bel_xy("BRAM_H1", "RAMB18", 0, 1).pins_name_only(&[
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
            let mut bel_bram_addr = builder.bel_virtual("BRAM_ADDR");
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
                                &format!("IMUX_ADDR{ab}ADDR{ul}{i}"),
                                &[
                                    &format!("BRAM_IMUX_ADDR{ab}ADDR{ul}{i}"),
                                    &format!("BRAM_R_IMUX_ADDR{ab}ADDR{ul}{i}"),
                                ],
                            )
                            .extra_wire(
                                &format!("UTURN_ADDR{ab}ADDR{ul}{i}"),
                                &[&format!("BRAM_UTURN_ADDR{ab}ADDR{ul}{i}")],
                            )
                            .extra_wire(
                                &format!("ADDR{ab}ADDR{ul}{i}"),
                                &[&format!("BRAM_ADDR{ab}ADDR{ul}{i}")],
                            );
                        if ul == 'U' {
                            bel_bram_addr = bel_bram_addr
                                .extra_wire(
                                    &format!("CASCINBOT_ADDR{ab}ADDR{ul}{i}"),
                                    &[&format!("BRAM_CASCINBOT_ADDR{ab}ADDR{ul}{i}")],
                                )
                                .extra_wire(
                                    &format!("CASCINTOP_ADDR{ab}ADDR{ul}{i}"),
                                    &[&format!("BRAM_CASCINTOP_ADDR{ab}ADDR{ul}{i}")],
                                )
                                .extra_wire(
                                    &format!("CASCOUT_ADDR{ab}ADDR{ul}{i}"),
                                    &[&format!("BRAM_CASCOUT_ADDR{ab}ADDR{ul}{i}")],
                                );
                        }
                    }
                }
                bel_bram_addr = bel_bram_addr
                    .extra_int_in(
                        &format!("IMUX_ADDR{ab}ADDRL15"),
                        &[
                            &format!("BRAM_IMUX_ADDR{ab}ADDRL15"),
                            &format!("BRAM_IMUX_R_ADDR{ab}ADDRL15"),
                        ],
                    )
                    .extra_wire(
                        &format!("UTURN_ADDR{ab}ADDRL15"),
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
            let n = builder.db.get_node_naming("INTF.L");
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
            let n = builder.db.get_node_naming("INTF.R");
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
            &[builder.bel_xy("PMVBRAM", "PMVBRAM", 0, 0)],
        );
    }

    for tkn in ["DSP_L", "DSP_R"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let n = builder
                .db
                .get_node_naming(if tkn == "DSP_L" { "INTF.L" } else { "INTF.R" });
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
                let mut bel = builder.bel_xy(format!("DSP{i}"), "DSP48", 0, i);
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
                    .bel_xy("TIEOFF", "TIEOFF", 0, 0)
                    .pins_name_only(&["HARD0", "HARD1"]),
            );
            builder.extract_xnode_bels_intf("DSP", xy, &[], &int_xy, &intf_xy, tkn, &bels_dsp);
        }
    }

    for (kind, tkn) in [("PCIE_L", "PCIE_BOT_LEFT"), ("PCIE_R", "PCIE_BOT")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            if rd.tile_kinds[rd.tiles[&xy].kind].sites.is_empty() {
                continue;
            }
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let nl = builder.db.get_node_naming(if kind == "PCIE_L" {
                "INTF.PCIE_LEFT_L"
            } else {
                "INTF.PCIE_L"
            });
            let nr = builder.db.get_node_naming("INTF.PCIE_R");
            for dy in 0..25 {
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
            for dy in 0..25 {
                int_xy.push(Coord {
                    x: xy.x + 6,
                    y: xy.y - 10 + dy,
                });
                intf_xy.push((
                    Coord {
                        x: xy.x + 5,
                        y: xy.y - 10 + dy,
                    },
                    nl,
                ));
            }
            let t_xy = Coord {
                x: xy.x,
                y: xy.y + 10,
            };
            builder.extract_xnode_bels_intf(
                kind,
                xy,
                &[t_xy],
                &int_xy,
                &intf_xy,
                kind,
                &[builder.bel_xy("PCIE", "PCIE", 0, 0)],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE3_RIGHT").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let nl = builder.db.get_node_naming("INTF.PCIE3_L");
        let nr = builder.db.get_node_naming("INTF.PCIE3_R");
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
            &[builder.bel_xy("PCIE3", "PCIE3", 0, 0)],
        );
    }

    builder.build()
}
