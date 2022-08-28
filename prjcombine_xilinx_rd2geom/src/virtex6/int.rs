use prjcombine_rawdump::{Coord, Part};
use prjcombine_xilinx_geom::int::{Dir, IntDb, WireKind};

use crate::intb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex6", rd);

    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..8 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK_B{i}")],
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
        (Dir::N, Dir::N, Some((0, Dir::S, 1))),
        (Dir::N, Dir::E, None),
        (Dir::N, Dir::W, Some((0, Dir::S, 0))),
        (Dir::S, Dir::S, Some((3, Dir::N, 0))),
        (Dir::S, Dir::E, None),
        (Dir::S, Dir::W, Some((3, Dir::N, 0))),
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

    // The long wires.
    let mid = builder.wire("LH.8", WireKind::MultiOut, &["LH8"]);
    let mut prev = mid;
    for i in (0..8).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mut prev = mid;
    for i in 9..17 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mid = builder.wire("LV.8", WireKind::MultiOut, &["LV8"]);
    let mut prev = mid;
    let mut lv_bh_n = Vec::new();
    for i in (0..8).rev() {
        prev = builder.multi_branch(prev, Dir::S, format!("LV.{i}"), &[format!("LV{i}")]);
        lv_bh_n.push(prev);
    }
    let mut prev = mid;
    let mut lv_bh_s = Vec::new();
    for i in 9..17 {
        prev = builder.multi_branch(prev, Dir::N, format!("LV.{i}"), &[format!("LV{i}")]);
        lv_bh_s.push(prev);
    }

    // The control inputs.
    for i in 0..2 {
        builder.mux_out(format!("IMUX.GFAN{i}"), &[format!("GFAN{i}")]);
    }
    for i in 0..2 {
        builder.mux_out(format!("IMUX.CLK{i}"), &[format!("CLK_B{i}")]);
    }
    for i in 0..2 {
        builder.mux_out(format!("IMUX.CTRL{i}"), &[format!("CTRL_B{i}")]);
    }
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP{i}")]);
        builder.buf(w, format!("IMUX.BYP{i}.SITE"), &[format!("BYP_B{i}")]);
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
        let w = builder.mux_out(format!("IMUX.FAN{i}"), &[format!("FAN{i}")]);
        builder.buf(w, format!("IMUX.FAN{i}.SITE"), &[format!("FAN_B{i}")]);
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
        builder.mux_out(format!("IMUX.IMUX{i}"), &[format!("IMUX_B{i}")]);
    }

    for i in 0..24 {
        builder.logic_out(format!("OUT{i}"), &[format!("LOGIC_OUTS{i}")]);
    }

    for i in 0..4 {
        builder.test_out(
            format!("TEST{i}"),
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

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");

    builder.extract_term_conn("TERM.W", Dir::W, "L_TERM_INT", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "R_TERM_INT", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "BRKH_T_TERM_INT", &[]);
    for &xy in rd.tiles_by_kind_name("PCIE") {
        let int_xy_a = Coord {
            x: xy.x,
            y: xy.y + 11,
        };
        let int_xy_b = Coord {
            x: xy.x + 2,
            y: xy.y + 11,
        };
        builder.extract_term_conn_tile("TERM.S", Dir::S, int_xy_a, &[]);
        builder.extract_term_conn_tile("TERM.S", Dir::S, int_xy_b, &[]);
    }
    builder.extract_term_conn("TERM.N", Dir::N, "BRKH_B_TERM_INT", &[]);
    builder.make_blackhole_term("TERM.S.HOLE", Dir::S, &lv_bh_s);
    builder.make_blackhole_term("TERM.N.HOLE", Dir::N, &lv_bh_n);

    builder.extract_intf("INTF", Dir::E, "INT_INTERFACE", "INTF", true);
    builder.extract_intf("INTF", Dir::E, "IOI_L_INT_INTERFACE", "INTF.IOI_L", true);
    for (n, tkn) in [
        ("GT_L", "GT_L_INT_INTERFACE"),
        ("GTX", "GTX_INT_INTERFACE"),
        ("EMAC", "EMAC_INT_INTERFACE"),
        ("PCIE_L", "PCIE_INT_INTERFACE_L"),
        ("PCIE_R", "PCIE_INT_INTERFACE_R"),
    ] {
        builder.extract_intf("INTF.DELAY", Dir::E, tkn, format!("INTF.{n}"), true);
    }

    for tkn in ["CLBLL", "CLBLM"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = Coord {
                x: xy.x - 1,
                y: xy.y,
            };
            builder.extract_xnode(
                tkn,
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
                &[],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_intf_naming("INTF");
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
            .bel_xy("BRAM_F", "RAMB36", 0, 0)
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
        let bel_bram_h0 = builder.bel_xy("BRAM_H0", "RAMB18", 0, 0);
        let mut bel_bram_h1 = builder.bel_xy("BRAM_H1", "RAMB18", 0, 1).pins_name_only(&[
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
        builder.extract_xnode_bels_intf(
            "BRAM",
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
        let n = builder.db.get_intf_naming("INTF");
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

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_intf_naming("INTF");
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
        builder.extract_xnode_bels_intf("DSP", xy, &[], &int_xy, &intf_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("EMAC").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_intf_naming("INTF.EMAC");
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
        builder.extract_xnode_bels_intf(
            "EMAC",
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "EMAC",
            &[builder.bel_xy("EMAC", "TEMAC", 0, 0)],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("PCIE").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let nl = builder.db.get_intf_naming("INTF.PCIE_L");
        let nr = builder.db.get_intf_naming("INTF.PCIE_R");
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
        builder.extract_xnode_bels_intf(
            "PCIE",
            xy,
            &[],
            &int_xy,
            &intf_xy,
            "PCIE",
            &[builder.bel_xy("PCIE", "PCIE", 0, 0)],
        );
    }

    builder.build()
}
