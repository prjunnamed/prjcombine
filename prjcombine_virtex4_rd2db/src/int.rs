use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::{Coord, Part};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex4", rd);

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..8 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK{i}")],
        );
    }
    for i in 0..2 {
        builder.wire(
            format!("RCLK{i}"),
            WireKind::ClkOut(8 + i),
            &[format!("RCLK{i}")],
        );
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
        let omux = builder.mux_out(format!("OMUX{i}"), &[format!("OMUX{i}")]);
        let omux_da1 = builder.branch(
            omux,
            da1,
            format!("OMUX{i}.{da1}"),
            &[format!("OMUX_{da1}{i}")],
        );
        if let Some(da2) = da2 {
            builder.branch(
                omux_da1,
                da2,
                format!("OMUX{i}.{da1}{da2}"),
                &[format!("OMUX_{da1}{da2}{i}")],
            );
        }
        if let Some(db) = db {
            builder.branch(
                omux,
                db,
                format!("OMUX{i}.{db}"),
                &[format!("OMUX_{db}{i}")],
            );
        }
        if i == 0 {
            builder.branch(omux, Dir::S, "OMUX0.S.ALT".to_string(), &["OUT_S"]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.mux_out(format!("DBL.{dir}{i}.0"), &[format!("{dir}2BEG{i}")]);
            let mid = builder.branch(
                beg,
                dir,
                format!("DBL.{dir}{i}.1"),
                &[format!("{dir}2MID{i}")],
            );
            let end = builder.branch(
                mid,
                dir,
                format!("DBL.{dir}{i}.2"),
                &[format!("{dir}2END{i}")],
            );
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(
                    end,
                    Dir::S,
                    format!("DBL.{dir}{i}.3"),
                    &[format!("{dir}2END_S{i}")],
                );
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(
                    end,
                    Dir::N,
                    format!("DBL.{dir}{i}.3"),
                    &[format!("{dir}2END_N{i}")],
                );
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let mut last = builder.mux_out(format!("HEX.{dir}{i}.0"), &[format!("{dir}6BEG{i}")]);
            for (j, seg) in [
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                last = builder.branch(
                    last,
                    dir,
                    format!("HEX.{dir}{i}.{j}"),
                    &[format!("{dir}6{seg}{i}")],
                );
            }
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(
                    last,
                    Dir::S,
                    format!("HEX.{dir}{i}.7"),
                    &[format!("{dir}6END_S{i}")],
                );
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(
                    last,
                    Dir::N,
                    format!("HEX.{dir}{i}.7"),
                    &[format!("{dir}6END_N{i}")],
                );
            }
        }
    }

    // The long wires.
    let mid = builder.wire("LH.12", WireKind::MultiOut, &["LH12"]);
    let mut prev = mid;
    for i in (0..12).rev() {
        prev = builder.multi_branch(prev, Dir::E, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mut prev = mid;
    for i in 13..25 {
        prev = builder.multi_branch(prev, Dir::W, format!("LH.{i}"), &[format!("LH{i}")]);
    }
    let mid = builder.wire("LV.12", WireKind::MultiOut, &["LV12"]);
    let mut prev = mid;
    for i in (0..12).rev() {
        prev = builder.multi_branch(prev, Dir::N, format!("LV.{i}"), &[format!("LV{i}")]);
    }
    let mut prev = mid;
    for i in 13..25 {
        prev = builder.multi_branch(prev, Dir::S, format!("LV.{i}"), &[format!("LV{i}")]);
    }

    // The control inputs.
    for i in 0..4 {
        builder.mux_out(format!("IMUX.SR{i}"), &[format!("SR_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.BOUNCE{i}"), &[format!("BOUNCE{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[format!("CLK_B{i}"), format!("CLK_B{i}_DCM0")],
        );
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.CE{i}"), &[format!("CE_B{i}")]);
    }

    // The data inputs.
    for i in 0..8 {
        let w = builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP_INT_B{i}")]);
        builder.buf(
            w,
            format!("IMUX.BYP{i}.BOUNCE"),
            &[format!("BYP_BOUNCE{i}")],
        );
    }

    for i in 0..32 {
        builder.mux_out(format!("IMUX.IMUX{i}"), &[format!("IMUX_B{i}")]);
    }

    for i in 0..8 {
        builder.logic_out(format!("OUT.BEST{i}"), &[format!("BEST_LOGIC_OUTS{i}")]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.SEC{i}"), &[format!("SECONDARY_LOGIC_OUTS{i}")]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.HALF.BOT{i}"), &[format!("HALF_OMUX_BOT{i}")]);
    }
    for i in 0..8 {
        builder.logic_out(format!("OUT.HALF.TOP{i}"), &[format!("HALF_OMUX_TOP{i}")]);
    }

    for i in 0..4 {
        let w = builder.test_out(
            format!("TEST{i}"),
            &[match i {
                0 => "IOIS_OCLKP_1",
                1 => "IOIS_ICLKP_1",
                2 => "IOIS_OCLKP_0",
                3 => "IOIS_ICLKP_0",
                _ => unreachable!(),
            }],
        );
        if i == 0 {
            builder.extra_name_sub("MONITOR_CONVST_TEST", 4, w);
        }
        for j in 0..16 {
            builder.extra_name_sub(format!("LOGIC_CREATED_INPUT_B{i}_INT{j}"), j, w);
        }
    }

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");
    builder.node_type("INT_SO", "INT", "INT");
    builder.node_type("INT_SO_DCM0", "INT", "INT.DCM0");

    builder.extract_term("TERM.W", None, Dir::W, "L_TERM_INT", "TERM.W");
    builder.extract_term("TERM.E", None, Dir::E, "R_TERM_INT", "TERM.E");
    builder.extract_term("TERM.S", None, Dir::S, "B_TERM_INT", "TERM.S");
    builder.extract_term("TERM.N", None, Dir::N, "T_TERM_INT", "TERM.N");
    for tkn in ["MGT_AL_BOT", "MGT_AL_MID", "MGT_AL", "MGT_BL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            for (i, delta) in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16]
                .into_iter()
                .enumerate()
            {
                let int_xy = Coord {
                    x: xy.x + 1,
                    y: xy.y - 9 + delta,
                };
                builder.extract_term_tile(
                    "TERM.W",
                    None,
                    Dir::W,
                    xy,
                    format!("TERM.W.MGT{i}"),
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
                let int_xy = Coord {
                    x: xy.x - 1,
                    y: xy.y - 9 + delta,
                };
                builder.extract_term_tile(
                    "TERM.E",
                    None,
                    Dir::E,
                    xy,
                    format!("TERM.E.MGT{i}"),
                    int_xy,
                );
            }
        }
    }

    builder.extract_pass_simple("BRKH", Dir::S, "BRKH", &[]);
    builder.extract_pass_buf("CLB_BUFFER", Dir::W, "CLB_BUFFER", "PASS.CLB_BUFFER", &[]);

    builder.stub_out("PB_OMUX11_B5");
    builder.stub_out("PB_OMUX11_B6");

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = Coord {
            x: pb_xy.x,
            y: pb_xy.y + 18,
        };
        for (i, delta) in [
            0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = Coord {
                x: pb_xy.x - 1,
                y: pb_xy.y - 3 + delta,
            };
            let int_e_xy = Coord {
                x: pb_xy.x + 15,
                y: pb_xy.y - 3 + delta,
            };
            let naming_w = format!("TERM.PPC.W{i}");
            let naming_e = format!("TERM.PPC.E{i}");
            let xy = if i < 11 { pb_xy } else { pt_xy };
            builder.extract_pass_tile(
                "PPC.W",
                Dir::W,
                int_e_xy,
                Some(xy),
                Some(xy),
                Some(&naming_w),
                None,
                int_w_xy,
                &[],
            );
            builder.extract_pass_tile(
                "PPC.E",
                Dir::E,
                int_w_xy,
                Some(xy),
                Some(xy),
                Some(&naming_e),
                None,
                int_e_xy,
                &[],
            );
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y - 4,
            };
            let int_n_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y + 22,
            };
            let ab = if i < 5 { 'A' } else { 'B' };
            let naming_s = format!("TERM.PPC.S{i}");
            let naming_n = format!("TERM.PPC.N{i}");
            builder.extract_pass_tile(
                format!("PPC{ab}.S"),
                Dir::S,
                int_n_xy,
                Some(pt_xy),
                Some(pb_xy),
                Some(&naming_s),
                None,
                int_s_xy,
                &[],
            );
            builder.extract_pass_tile(
                format!("PPC{ab}.N"),
                Dir::N,
                int_s_xy,
                Some(pb_xy),
                Some(pt_xy),
                Some(&naming_n),
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
                let int_xy = Coord {
                    x: xy.x - 1,
                    y: xy.y + i,
                };
                builder.extract_intf_tile("INTF", xy, int_xy, format!("INTF.{n}.{i}"), false);
            }
        }
    }
    for tkn in ["IOIS_LC", "IOIS_NC"] {
        builder.extract_intf("INTF", Dir::E, tkn, "INTF.IOIS", false);
    }
    for &xy in rd.tiles_by_kind_name("CFG_CENTER") {
        for i in 0..16 {
            let int_xy = Coord {
                x: xy.x - 1,
                y: if i < 8 {
                    xy.y - 8 + i
                } else {
                    xy.y + 1 + i - 8
                },
            };
            builder.extract_intf_tile("INTF", xy, int_xy, format!("INTF.CFG.{i}"), false);
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
                let int_xy = Coord {
                    x: if dir == Dir::E { xy.x - 1 } else { xy.x + 1 },
                    y: if i < 8 { xy.y - 9 + i } else { xy.y + i - 8 },
                };
                builder.extract_intf_tile("INTF", xy, int_xy, format!("INTF.MGT.{i}"), false);
            }
        }
    }

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = Coord {
            x: pb_xy.x,
            y: pb_xy.y + 18,
        };
        for (i, delta) in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = Coord {
                x: pb_xy.x - 1,
                y: pb_xy.y - 4 + delta,
            };
            let int_e_xy = Coord {
                x: pb_xy.x + 15,
                y: pb_xy.y - 4 + delta,
            };
            let xy = if i < 12 { pb_xy } else { pt_xy };
            builder.extract_intf_tile("INTF", xy, int_w_xy, format!("INTF.PPC.L{i}"), false);
            builder.extract_intf_tile("INTF", xy, int_e_xy, format!("INTF.PPC.R{i}"), false);
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y - 4,
            };
            let int_n_xy = Coord {
                x: pb_xy.x + delta,
                y: pb_xy.y + 22,
            };
            builder.extract_intf_tile("INTF", pb_xy, int_s_xy, format!("INTF.PPC.B{i}"), false);
            builder.extract_intf_tile("INTF", pt_xy, int_n_xy, format!("INTF.PPC.T{i}"), false);
        }
    }

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    if let Some(&xy) = rd.tiles_by_kind_name("CLB").iter().next() {
        let int_xy = Coord {
            x: xy.x - 1,
            y: xy.y,
        };
        builder.extract_xnode_bels(
            "CLB",
            xy,
            &[],
            &[int_xy],
            "CLB",
            &[
                builder
                    .bel_xy("SLICE0", "SLICE", 0, 0)
                    .pins_name_only(&slicem_name_only),
                builder
                    .bel_xy("SLICE1", "SLICE", 1, 0)
                    .pins_name_only(&slicel_name_only),
                builder
                    .bel_xy("SLICE2", "SLICE", 0, 1)
                    .pins_name_only(&slicem_name_only)
                    .extra_wire("COUT_N", &["COUT_N1"])
                    .extra_wire("FX_S", &["FX_S2"]),
                builder
                    .bel_xy("SLICE3", "SLICE", 1, 1)
                    .pins_name_only(&slicel_name_only)
                    .extra_wire("COUT_N", &["COUT_N3"]),
            ],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(Coord {
                x: xy.x - 1,
                y: xy.y + dy,
            });
        }
        builder.extract_xnode_bels(
            "BRAM",
            xy,
            &[],
            &int_xy,
            "BRAM",
            &[
                builder
                    .bel_xy("BRAM", "RAMB16", 0, 0)
                    .pins_name_only(&["CASCADEOUTA", "CASCADEOUTB"])
                    .pin_name_only("CASCADEINA", 1)
                    .pin_name_only("CASCADEINB", 1),
                builder.bel_xy("FIFO", "FIFO16", 0, 0),
            ],
        );
    }

    let mut bels_dsp = vec![];
    for i in 0..2 {
        let mut bel = builder.bel_xy(format!("DSP{i}"), "DSP48", 0, i);
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
            int_xy.push(Coord {
                x: xy.x - 1,
                y: xy.y + dy,
            });
        }
        builder.extract_xnode_bels("DSP", xy, &[], &int_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER").iter().next() {
        let mut bels = vec![];
        for i in 0..16 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCTRL{i}"), "BUFGCTRL", 0, i)
                    .raw_tile(1)
                    .pins_name_only(&["I0", "I1", "O"])
                    .extra_wire("GCLK", &[format!("CLK_BUFGCTRL_GCLKP{i}")])
                    .extra_wire("GFB", &[format!("CLK_BUFGCTRL_GFB_P{i}")])
                    .extra_int_out("I0MUX", &[format!("CLK_BUFGCTRL_I0P{i}")])
                    .extra_int_out("I1MUX", &[format!("CLK_BUFGCTRL_I1P{i}")])
                    .extra_int_in("CKINT0", &[format!("CLK_BUFGCTRL_CKINT0{i}")])
                    .extra_int_in("CKINT1", &[format!("CLK_BUFGCTRL_CKINT1{i}")])
                    .extra_wire(
                        "MUXBUS0",
                        &[format!("CLK_BUFGCTRL_MUXED_CLK{ii}", ii = i * 2)],
                    )
                    .extra_wire(
                        "MUXBUS1",
                        &[format!("CLK_BUFGCTRL_MUXED_CLK{ii}", ii = i * 2 + 1)],
                    ),
            );
        }
        for i in 0..16 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCTRL{ii}", ii = i + 16), "BUFGCTRL", 0, i)
                    .raw_tile(2)
                    .pins_name_only(&["I0", "I1", "O"])
                    .extra_wire("GCLK", &[format!("CLK_BUFGCTRL_GCLKP{ii}", ii = i + 16)])
                    .extra_wire("GFB", &[format!("CLK_BUFGCTRL_GFB_P{i}")])
                    .extra_int_out("I0MUX", &[format!("CLK_BUFGCTRL_I0P{i}")])
                    .extra_int_out("I1MUX", &[format!("CLK_BUFGCTRL_I1P{i}")])
                    .extra_int_in("CKINT0", &[format!("CLK_BUFGCTRL_CKINT0{ii}", ii = 15 - i)])
                    .extra_int_in("CKINT1", &[format!("CLK_BUFGCTRL_CKINT1{ii}", ii = 15 - i)])
                    .extra_wire(
                        "MUXBUS0",
                        &[format!("CLK_BUFGCTRL_MUXED_CLK{ii}", ii = i * 2)],
                    )
                    .extra_wire(
                        "MUXBUS1",
                        &[format!("CLK_BUFGCTRL_MUXED_CLK{ii}", ii = i * 2 + 1)],
                    ),
            );
        }
        bels.extend([
            builder.bel_xy("BSCAN0", "BSCAN", 0, 0),
            builder.bel_xy("BSCAN1", "BSCAN", 0, 1),
            builder.bel_xy("BSCAN2", "BSCAN", 0, 2),
            builder.bel_xy("BSCAN3", "BSCAN", 0, 3),
            builder.bel_xy("ICAP0", "ICAP", 0, 0),
            builder.bel_xy("ICAP1", "ICAP", 0, 1),
            builder.bel_single("PMV", "PMV"),
            builder.bel_single("STARTUP", "STARTUP"),
            builder
                .bel_single("JTAGPPC", "JTAGPPC")
                .pin_name_only("TDOTSPPC", 0),
            builder.bel_single("FRAME_ECC", "FRAME_ECC"),
            builder.bel_single("DCIRESET", "DCIRESET"),
            builder.bel_single("CAPTURE", "CAPTURE"),
            builder.bel_single("USR_ACCESS", "USR_ACCESS_SITE"),
            builder
                .bel_virtual("BUFG_MGTCLK_B")
                .raw_tile(1)
                .extra_wire("MGT_L0", &["CLK_BUFGCTRL_MGT_L0"])
                .extra_wire("MGT_L1", &["CLK_BUFGCTRL_MGT_L1"])
                .extra_wire("MGT_R0", &["CLK_BUFGCTRL_MGT_R0"])
                .extra_wire("MGT_R1", &["CLK_BUFGCTRL_MGT_R1"]),
            builder
                .bel_virtual("BUFG_MGTCLK_T")
                .raw_tile(2)
                .extra_wire("MGT_L0", &["CLK_BUFGCTRL_MGT_L0"])
                .extra_wire("MGT_L1", &["CLK_BUFGCTRL_MGT_L1"])
                .extra_wire("MGT_R0", &["CLK_BUFGCTRL_MGT_R0"])
                .extra_wire("MGT_R1", &["CLK_BUFGCTRL_MGT_R1"]),
            builder
                .bel_virtual("BUFG_MGTCLK_B_HROW")
                .raw_tile(3)
                .extra_wire_force("MGT_L0_I", "CLK_HROW_H_MGT_L0")
                .extra_wire_force("MGT_L1_I", "CLK_HROW_H_MGT_L1")
                .extra_wire_force("MGT_R0_I", "CLK_HROW_H_MGT_R0")
                .extra_wire_force("MGT_R1_I", "CLK_HROW_H_MGT_R1")
                .extra_wire_force("MGT_L0_O", "CLK_HROW_V_MGT_L0")
                .extra_wire_force("MGT_L1_O", "CLK_HROW_V_MGT_L1")
                .extra_wire_force("MGT_R0_O", "CLK_HROW_V_MGT_R0")
                .extra_wire_force("MGT_R1_O", "CLK_HROW_V_MGT_R1"),
            builder
                .bel_virtual("BUFG_MGTCLK_T_HROW")
                .raw_tile(4)
                .extra_wire_force("MGT_L0_I", "CLK_HROW_H_MGT_L0")
                .extra_wire_force("MGT_L1_I", "CLK_HROW_H_MGT_L1")
                .extra_wire_force("MGT_R0_I", "CLK_HROW_H_MGT_R0")
                .extra_wire_force("MGT_R1_I", "CLK_HROW_H_MGT_R1")
                .extra_wire_force("MGT_L0_O", "CLK_HROW_V_MGT_L0")
                .extra_wire_force("MGT_L1_O", "CLK_HROW_V_MGT_L1")
                .extra_wire_force("MGT_R0_O", "CLK_HROW_V_MGT_R0")
                .extra_wire_force("MGT_R1_O", "CLK_HROW_V_MGT_R1"),
            builder
                .bel_virtual("BUFG_MGTCLK_B_HCLK")
                .raw_tile(5)
                .extra_wire_force("MGT_L0_I", "HCLK_MGT_CLKL0")
                .extra_wire_force("MGT_L1_I", "HCLK_MGT_CLKL1")
                .extra_wire_force("MGT_R0_I", "HCLK_MGT_CLKR0")
                .extra_wire_force("MGT_R1_I", "HCLK_MGT_CLKR1")
                .extra_wire_force("MGT_L0_O", "HCLK_CENTER_MGT0")
                .extra_wire_force("MGT_L1_O", "HCLK_CENTER_MGT1")
                .extra_wire_force("MGT_R0_O", "HCLK_CENTER_MGT2")
                .extra_wire_force("MGT_R1_O", "HCLK_CENTER_MGT3"),
            builder
                .bel_virtual("BUFG_MGTCLK_T_HCLK")
                .raw_tile(6)
                .extra_wire_force("MGT_L0_I", "HCLK_MGT_CLKL0")
                .extra_wire_force("MGT_L1_I", "HCLK_MGT_CLKL1")
                .extra_wire_force("MGT_R0_I", "HCLK_MGT_CLKR0")
                .extra_wire_force("MGT_R1_I", "HCLK_MGT_CLKR1")
                .extra_wire_force("MGT_L0_O", "HCLK_CENTER_MGT0")
                .extra_wire_force("MGT_L1_O", "HCLK_CENTER_MGT1")
                .extra_wire_force("MGT_R0_O", "HCLK_CENTER_MGT2")
                .extra_wire_force("MGT_R1_O", "HCLK_CENTER_MGT3"),
        ]);
        let xy_bufg_b = Coord {
            x: xy.x + 1,
            y: xy.y - 8,
        };
        let xy_bufg_t = Coord {
            x: xy.x + 1,
            y: xy.y + 1,
        };
        let xy_hrow_b = Coord {
            x: xy.x + 1,
            y: xy.y - 9,
        };
        let xy_hrow_t = Coord {
            x: xy.x + 1,
            y: xy.y + 9,
        };
        let xy_hclk_b = Coord {
            x: xy.x,
            y: xy.y - 9,
        };
        let xy_hclk_t = Coord {
            x: xy.x,
            y: xy.y + 9,
        };
        let mut xn = builder
            .xnode("CFG", "CFG", xy)
            .raw_tile(xy_bufg_b)
            .raw_tile(xy_bufg_t)
            .raw_tile(xy_hrow_b)
            .raw_tile(xy_hrow_t)
            .raw_tile(xy_hclk_b)
            .raw_tile(xy_hclk_t)
            .num_tiles(16);
        for i in 0..8 {
            xn = xn.ref_int(
                Coord {
                    x: xy.x - 1,
                    y: xy.y - 8 + (i as u16),
                },
                i,
            );
        }
        for i in 0..8 {
            xn = xn.ref_int(
                Coord {
                    x: xy.x - 1,
                    y: xy.y + 1 + (i as u16),
                },
                i + 8,
            );
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();
    }

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = Coord {
            x: pb_xy.x,
            y: pb_xy.y + 18,
        };
        let mut int_xy = vec![];
        for dy in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ] {
            int_xy.push(Coord {
                x: pb_xy.x - 1,
                y: pb_xy.y - 4 + dy,
            });
        }
        for dy in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ] {
            int_xy.push(Coord {
                x: pb_xy.x + 15,
                y: pb_xy.y - 4 + dy,
            });
        }
        for dx in [1, 3, 5, 7, 9, 11, 13] {
            int_xy.push(Coord {
                x: pb_xy.x + dx,
                y: pb_xy.y - 4,
            });
        }
        for dx in [1, 3, 5, 7, 9, 11, 13] {
            int_xy.push(Coord {
                x: pb_xy.x + dx,
                y: pb_xy.y + 22,
            });
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
        builder.extract_xnode_bels(
            "PPC",
            pb_xy,
            &[pt_xy],
            &int_xy,
            "PPC",
            &[
                builder
                    .bel_xy("PPC", "PPC405_ADV", 0, 0)
                    .pins_name_only(&dcr_pins),
                builder
                    .bel_xy("EMAC", "EMAC", 0, 0)
                    .pins_name_only(&dcr_pins),
            ],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CLK_HROW").iter().next() {
        let mut bel = builder.bel_virtual("CLK_HROW");
        for i in 0..32 {
            bel = bel.extra_wire(format!("GCLK{i}"), &[format!("CLK_HROW_GCLK_BUFP{i}")]);
        }
        for i in 0..8 {
            bel = bel.extra_wire(format!("OUT_L{i}"), &[format!("CLK_HROW_HCLK_LP{i}")]);
            bel = bel.extra_wire(format!("OUT_R{i}"), &[format!("CLK_HROW_HCLK_RP{i}")]);
        }
        builder.xnode("CLK_HROW", "CLK_HROW", xy).bel(bel).extract();
    }

    for tkn in ["CLK_IOB_B", "CLK_IOB_T"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("CLK_IOB");
            for i in 0..16 {
                bel = bel.extra_wire(format!("PAD{i}"), &[format!("CLK_IOB_PAD_CLKP{i}")]);
                bel = bel.extra_wire(format!("PAD_BUF{i}"), &[format!("CLK_IOB_IOB_BUFCLKP{i}")]);
                bel = bel.extra_wire(format!("GIOB{i}"), &[format!("CLK_IOB_IOB_CLKP{i}")]);
            }
            for i in 0..32 {
                bel = bel.extra_wire(
                    format!("MUXBUS_I{i}"),
                    &[format!("CLK_IOB_MUXED_CLKP_IN{i}")],
                );
                bel = bel.extra_wire(format!("MUXBUS_O{i}"), &[format!("CLK_IOB_MUXED_CLKP{i}")]);
            }
            builder.xnode("CLK_IOB", "CLK_IOB", xy).bel(bel).extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK").iter().next() {
        let mut bel = builder.bel_virtual("HCLK");
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("GCLK_I{i}"), &[format!("HCLK_G_HCLKP{i}")])
                .extra_int_out(format!("GCLK_O{i}"), &[format!("HCLK_LEAF_GCLK{i}")]);
        }
        for i in 0..2 {
            bel = bel
                .extra_wire(format!("RCLK_I{i}"), &[format!("HCLK_RCLK{i}")])
                .extra_int_out(format!("RCLK_O{i}"), &[format!("HCLK_LEAF_RCLK{i}")]);
        }
        builder
            .xnode("HCLK", "HCLK", xy)
            .ref_int(
                Coord {
                    x: xy.x,
                    y: xy.y + 1,
                },
                0,
            )
            .bel(bel)
            .extract();
    }

    let bel_ioclk = builder
        .bel_virtual("IOCLK")
        .extra_wire("IOCLK0", &["HCLK_IOIS_IOCLKP0"])
        .extra_wire("IOCLK1", &["HCLK_IOIS_IOCLKP1"])
        .extra_wire_force("IOCLK_N0", "HCLK_IOIS_IOCLKP_N0")
        .extra_wire_force("IOCLK_N1", "HCLK_IOIS_IOCLKP_N1")
        .extra_wire_force("IOCLK_S0", "HCLK_IOIS_IOCLKP_S0")
        .extra_wire_force("IOCLK_S1", "HCLK_IOIS_IOCLKP_S1")
        .extra_wire("VIOCLK0", &["HCLK_IOIS_VIOCLKP0"])
        .extra_wire("VIOCLK1", &["HCLK_IOIS_VIOCLKP1"])
        .extra_wire_force("VIOCLK_N0", "HCLK_IOIS_VIOCLKP_N0")
        .extra_wire_force("VIOCLK_N1", "HCLK_IOIS_VIOCLKP_N1")
        .extra_wire_force("VIOCLK_S0", "HCLK_IOIS_VIOCLKP_S0")
        .extra_wire_force("VIOCLK_S1", "HCLK_IOIS_VIOCLKP_S1")
        .extra_wire("GCLK_IN0", &["HCLK_IOIS_G_HCLKP0", "HCLK_DCM_G_HCLKP0"])
        .extra_wire("GCLK_IN1", &["HCLK_IOIS_G_HCLKP1", "HCLK_DCM_G_HCLKP1"])
        .extra_wire("GCLK_IN2", &["HCLK_IOIS_G_HCLKP2", "HCLK_DCM_G_HCLKP2"])
        .extra_wire("GCLK_IN3", &["HCLK_IOIS_G_HCLKP3", "HCLK_DCM_G_HCLKP3"])
        .extra_wire("GCLK_IN4", &["HCLK_IOIS_G_HCLKP4", "HCLK_DCM_G_HCLKP4"])
        .extra_wire("GCLK_IN5", &["HCLK_IOIS_G_HCLKP5", "HCLK_DCM_G_HCLKP5"])
        .extra_wire("GCLK_IN6", &["HCLK_IOIS_G_HCLKP6", "HCLK_DCM_G_HCLKP6"])
        .extra_wire("GCLK_IN7", &["HCLK_IOIS_G_HCLKP7", "HCLK_DCM_G_HCLKP7"])
        .extra_wire(
            "GCLK_OUT0",
            &["HCLK_IOIS_LEAF_GCLK_P0", "HCLK_DCM_LEAF_GCLK_P0"],
        )
        .extra_wire(
            "GCLK_OUT1",
            &["HCLK_IOIS_LEAF_GCLK_P1", "HCLK_DCM_LEAF_GCLK_P1"],
        )
        .extra_wire(
            "GCLK_OUT2",
            &["HCLK_IOIS_LEAF_GCLK_P2", "HCLK_DCM_LEAF_GCLK_P2"],
        )
        .extra_wire(
            "GCLK_OUT3",
            &["HCLK_IOIS_LEAF_GCLK_P3", "HCLK_DCM_LEAF_GCLK_P3"],
        )
        .extra_wire(
            "GCLK_OUT4",
            &["HCLK_IOIS_LEAF_GCLK_P4", "HCLK_DCM_LEAF_GCLK_P4"],
        )
        .extra_wire(
            "GCLK_OUT5",
            &["HCLK_IOIS_LEAF_GCLK_P5", "HCLK_DCM_LEAF_GCLK_P5"],
        )
        .extra_wire(
            "GCLK_OUT6",
            &["HCLK_IOIS_LEAF_GCLK_P6", "HCLK_DCM_LEAF_GCLK_P6"],
        )
        .extra_wire(
            "GCLK_OUT7",
            &["HCLK_IOIS_LEAF_GCLK_P7", "HCLK_DCM_LEAF_GCLK_P7"],
        )
        .extra_wire("RCLK_IN0", &["HCLK_IOIS_RCLK0", "HCLK_DCM_RCLK0"])
        .extra_wire("RCLK_IN1", &["HCLK_IOIS_RCLK1", "HCLK_DCM_RCLK1"])
        .extra_wire(
            "RCLK_OUT0",
            &["HCLK_IOIS_RCLK_FORIO_P0", "HCLK_DCM_RCLK_FORIO_P0"],
        )
        .extra_wire(
            "RCLK_OUT1",
            &["HCLK_IOIS_RCLK_FORIO_P1", "HCLK_DCM_RCLK_FORIO_P1"],
        );
    for tkn in ["HCLK_IOIS_DCI", "HCLK_IOIS_LVDS"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![
                builder
                    .bel_xy("BUFR0", "BUFR", 0, 0)
                    .pins_name_only(&["O", "I"]),
                builder
                    .bel_xy("BUFR1", "BUFR", 0, 1)
                    .pins_name_only(&["O", "I"]),
                builder
                    .bel_xy("BUFIO0", "BUFIO", 0, 0)
                    .pins_name_only(&["O", "I"])
                    .extra_wire("PAD", &["HCLK_IOIS_I2IOCLK_TOP_P"]),
                builder
                    .bel_xy("BUFIO1", "BUFIO", 0, 1)
                    .pins_name_only(&["O", "I"])
                    .extra_wire("PAD", &["HCLK_IOIS_I2IOCLK_BOT_P"]),
                builder
                    .bel_xy("IDELAYCTRL", "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
            ];
            if tkn == "HCLK_IOIS_DCI" {
                bels.push(builder.bel_xy("DCI", "DCI", 0, 0));
            }
            bels.extend([
                builder
                    .bel_virtual("RCLK")
                    .extra_int_in("CKINT0", &["HCLK_IOIS_INT_RCLKMUX_N"])
                    .extra_int_in("CKINT1", &["HCLK_IOIS_INT_RCLKMUX_S"])
                    .extra_wire("RCLK0", &["HCLK_IOIS_RCLK0"])
                    .extra_wire("RCLK1", &["HCLK_IOIS_RCLK1"])
                    .extra_wire("VRCLK0", &["HCLK_IOIS_VRCLK0"])
                    .extra_wire("VRCLK1", &["HCLK_IOIS_VRCLK1"])
                    .extra_wire("VRCLK_N0", &["HCLK_IOIS_VRCLK_N0"])
                    .extra_wire("VRCLK_N1", &["HCLK_IOIS_VRCLK_N1"])
                    .extra_wire("VRCLK_S0", &["HCLK_IOIS_VRCLK_S0"])
                    .extra_wire("VRCLK_S1", &["HCLK_IOIS_VRCLK_S1"]),
                bel_ioclk.clone(),
            ]);
            let mut xn = builder
                .xnode(tkn, tkn, xy)
                .num_tiles(3)
                .raw_tile(Coord {
                    x: xy.x,
                    y: xy.y - 2,
                })
                .raw_tile(Coord {
                    x: xy.x,
                    y: xy.y - 1,
                })
                .raw_tile(Coord {
                    x: xy.x,
                    y: xy.y + 1,
                })
                .ref_int(
                    Coord {
                        x: xy.x - 1,
                        y: xy.y - 2,
                    },
                    0,
                )
                .ref_int(
                    Coord {
                        x: xy.x - 1,
                        y: xy.y - 1,
                    },
                    1,
                )
                .ref_int(
                    Coord {
                        x: xy.x - 1,
                        y: xy.y + 1,
                    },
                    2,
                );
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }
    let mut bel_hclk_dcm_hrow = builder.bel_virtual("HCLK_DCM_HROW");
    for i in 0..16 {
        bel_hclk_dcm_hrow = bel_hclk_dcm_hrow
            .extra_wire(format!("GIOB_I{i}"), &[format!("CLK_HROW_IOB_BUFCLKP{i}")])
            .extra_wire(
                format!("GIOB_O{i}"),
                &[format!("CLK_HROW_IOB_H_BUFCLKP{i}")],
            );
    }
    for (tkn, ioloc, dcmloc) in [
        ("HCLK_CENTER", 'S', '_'),
        ("HCLK_CENTER_ABOVE_CFG", 'N', '_'),
        ("HCLK_DCMIOB", 'N', 'S'),
        ("HCLK_IOBDCM", 'S', 'N'),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel_ioclk = bel_ioclk.clone();
            match dcmloc {
                'S' => {
                    bel_ioclk = bel_ioclk
                        .extra_wire("IOCLK0", &["HCLK_DCMIOB_IOCLKP0"])
                        .extra_wire("IOCLK1", &["HCLK_DCMIOB_IOCLKP1"])
                        .extra_wire_force("IOCLK_N0", "HCLK_DCMIOB_IOCLKP_N0")
                        .extra_wire_force("IOCLK_N1", "HCLK_DCMIOB_IOCLKP_N1")
                        .extra_wire_force("IOCLK_S0", "HCLK_DCMIOB_IOCLKP_S0")
                        .extra_wire_force("IOCLK_S1", "HCLK_DCMIOB_IOCLKP_S1")
                        .extra_wire("VIOCLK0", &["HCLK_DCMIOB_VIOCLKP0"])
                        .extra_wire("VIOCLK1", &["HCLK_DCMIOB_VIOCLKP1"])
                        .extra_wire_force("VIOCLK_N0", "HCLK_DCMIOB_VIOCLKP_N0")
                        .extra_wire_force("VIOCLK_N1", "HCLK_DCMIOB_VIOCLKP_N1")
                        .extra_wire_force("VIOCLK_S0", "HCLK_DCMIOB_VIOCLKP_S0")
                        .extra_wire_force("VIOCLK_S1", "HCLK_DCMIOB_VIOCLKP_S1");
                }
                'N' => {
                    bel_ioclk = bel_ioclk
                        .extra_wire("IOCLK0", &["HCLK_IOBDCM_IOCLKP0"])
                        .extra_wire("IOCLK1", &["HCLK_IOBDCM_IOCLKP1"])
                        .extra_wire_force("IOCLK_N0", "HCLK_IOBDCM_IOCLKP_N0")
                        .extra_wire_force("IOCLK_N1", "HCLK_IOBDCM_IOCLKP_N1")
                        .extra_wire_force("IOCLK_S0", "HCLK_IOBDCM_IOCLKP_S0")
                        .extra_wire_force("IOCLK_S1", "HCLK_IOBDCM_IOCLKP_S1")
                        .extra_wire("VIOCLK0", &["HCLK_IOBDCM_VIOCLKP0"])
                        .extra_wire("VIOCLK1", &["HCLK_IOBDCM_VIOCLKP1"])
                        .extra_wire_force("VIOCLK_N0", "HCLK_IOBDCM_VIOCLKP_N0")
                        .extra_wire_force("VIOCLK_N1", "HCLK_IOBDCM_VIOCLKP_N1")
                        .extra_wire_force("VIOCLK_S0", "HCLK_IOBDCM_VIOCLKP_S0")
                        .extra_wire_force("VIOCLK_S1", "HCLK_IOBDCM_VIOCLKP_S1");
                }
                _ => (),
            }
            let mut bels = vec![
                builder
                    .bel_xy("BUFIO0", "BUFIO", 0, 0)
                    .pins_name_only(&["O", "I"])
                    .extra_wire_force("PAD", "HCLK_IOIS_I2IOCLK_TOP_P"),
                builder
                    .bel_xy("BUFIO1", "BUFIO", 0, 1)
                    .pins_name_only(&["O", "I"])
                    .extra_wire_force("PAD", "HCLK_IOIS_I2IOCLK_BOT_P"),
                builder
                    .bel_xy("IDELAYCTRL", "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
                builder.bel_xy("DCI", "DCI", 0, 0),
                bel_ioclk,
            ];
            match dcmloc {
                'S' => {
                    let mut bel = builder.bel_virtual("HCLK_DCM_S");
                    for i in 0..8 {
                        bel = bel
                            .extra_wire(format!("GCLK_I{i}"), &[format!("HCLK_DCM_G_HCLKP{i}")])
                            .extra_wire(
                                format!("GCLK_O_D{i}"),
                                &[format!("HCLK_DCM_LEAF_DIRECT_HCLKP{i}")],
                            );
                    }
                    for i in 0..16 {
                        bel = bel
                            .extra_wire(format!("GIOB_I{i}"), &[format!("HCLK_DCM_IOB_CLKP{i}")])
                            .extra_wire(
                                format!("GIOB_O_D{i}"),
                                &[format!("HCLK_DCM_IOB_CLKP_OUT{i}")],
                            );
                    }
                    for i in 0..4 {
                        bel = bel.extra_wire_force(format!("MGT_O_D{i}"), format!("HCLK_MGT{i}"));
                    }
                    bel = bel
                        .extra_wire_force("MGT_I0", "HCLK_MGT_CLKL0")
                        .extra_wire_force("MGT_I1", "HCLK_MGT_CLKL1")
                        .extra_wire_force("MGT_I2", "HCLK_MGT_CLKR0")
                        .extra_wire_force("MGT_I3", "HCLK_MGT_CLKR1");
                    bels.extend([bel, bel_hclk_dcm_hrow.clone().raw_tile(3)]);
                }
                'N' => {
                    let mut bel = builder.bel_virtual("HCLK_DCM_N");
                    for i in 0..8 {
                        bel = bel
                            .extra_wire(format!("GCLK_I{i}"), &[format!("HCLK_DCM_G_HCLKP{i}")])
                            .extra_wire(
                                format!("GCLK_O_U{i}"),
                                &[format!("HCLK_DCM_LEAF_DIRECT_HCLKP{i}")],
                            )
                    }
                    for i in 0..16 {
                        bel = bel
                            .extra_wire(format!("GIOB_I{i}"), &[format!("HCLK_DCM_IOB_CLKP{i}")])
                            .extra_wire(
                                format!("GIOB_O_U{i}"),
                                &[format!("HCLK_DCM_IOB_CLKP_OUT{i}")],
                            )
                    }
                    for i in 0..4 {
                        bel = bel.extra_wire_force(format!("MGT_O_U{i}"), format!("HCLK_MGT{i}"));
                    }
                    bel = bel
                        .extra_wire_force("MGT_I0", "HCLK_MGT_CLKL0")
                        .extra_wire_force("MGT_I1", "HCLK_MGT_CLKL1")
                        .extra_wire_force("MGT_I2", "HCLK_MGT_CLKR0")
                        .extra_wire_force("MGT_I3", "HCLK_MGT_CLKR1");
                    bels.extend([bel, bel_hclk_dcm_hrow.clone().raw_tile(3)]);
                }
                _ => (),
            }
            let mut xn = builder.xnode(tkn, tkn, xy).num_tiles(2);
            if ioloc == 'S' {
                xn = xn
                    .raw_tile(Coord {
                        x: xy.x,
                        y: xy.y - 2,
                    })
                    .raw_tile(Coord {
                        x: xy.x,
                        y: xy.y - 1,
                    })
                    .ref_int(
                        Coord {
                            x: xy.x - 1,
                            y: xy.y - 2,
                        },
                        0,
                    )
                    .ref_int(
                        Coord {
                            x: xy.x - 1,
                            y: xy.y - 1,
                        },
                        1,
                    );
            } else {
                xn = xn
                    .raw_tile(Coord {
                        x: xy.x,
                        y: xy.y + 1,
                    })
                    .raw_tile(Coord {
                        x: xy.x,
                        y: xy.y + 2,
                    })
                    .ref_int(
                        Coord {
                            x: xy.x - 1,
                            y: xy.y + 1,
                        },
                        0,
                    )
                    .ref_int(
                        Coord {
                            x: xy.x - 1,
                            y: xy.y + 2,
                        },
                        1,
                    );
            }
            if dcmloc != '_' {
                xn = xn.raw_tile(Coord {
                    x: xy.x + 1,
                    y: xy.y,
                });
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }
    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_DCM").iter().next() {
        let mut bel = builder.bel_virtual("HCLK_DCM");
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("GCLK_I{i}"), &[format!("HCLK_DCM_G_HCLKP{i}")])
                .extra_wire(
                    format!("GCLK_O_U{i}"),
                    &[format!("HCLK_DCM_LEAF_DIRECT_UP_HCLKP{i}")],
                )
                .extra_wire(
                    format!("GCLK_O_D{i}"),
                    &[format!("HCLK_DCM_LEAF_DIRECT_HCLKP{i}")],
                );
        }
        for i in 0..16 {
            bel = bel
                .extra_wire(format!("GIOB_I{i}"), &[format!("HCLK_DCM_IOB_CLKP{i}")])
                .extra_wire(
                    format!("GIOB_O_U{i}"),
                    &[format!("HCLK_DCM_IOB_CLKP_UP_OUT{i}")],
                )
                .extra_wire(
                    format!("GIOB_O_D{i}"),
                    &[format!("HCLK_DCM_IOB_CLKP_DOWN_OUT{i}")],
                );
        }
        for i in 0..4 {
            bel = bel
                .extra_wire(format!("MGT{i}"), &[format!("HCLK_DCM_MGT{i}")])
                .extra_wire(format!("MGT_O_U{i}"), &[format!("HCLK_DCM_UP_MGT{i}")])
                .extra_wire(format!("MGT_O_D{i}"), &[format!("HCLK_DCM_DN_MGT{i}")]);
        }
        bel = bel
            .extra_wire_force("MGT_I0", "HCLK_MGT_CLKL0")
            .extra_wire_force("MGT_I1", "HCLK_MGT_CLKL1")
            .extra_wire_force("MGT_I2", "HCLK_MGT_CLKR0")
            .extra_wire_force("MGT_I3", "HCLK_MGT_CLKR1");
        builder
            .xnode("HCLK_DCM", "HCLK_DCM", xy)
            .raw_tile(Coord {
                x: xy.x + 1,
                y: xy.y,
            })
            .bel(bel)
            .bel(bel_hclk_dcm_hrow.raw_tile(1))
            .extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("SYS_MON").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..8 {
            int_xy.push(Coord {
                x: xy.x - 1,
                y: xy.y + dy,
            });
        }
        let mut bel = builder
            .bel_xy("SYSMON", "MONITOR", 0, 0)
            .pins_name_only(&["CONVST", "VP", "VN"])
            .extra_int_in("CONVST_INT_IMUX", &["IMUX_B0_INT0"])
            .extra_int_in("CONVST_INT_CLK", &["CLK_B1_INT0"])
            .extra_int_out("CONVST_TEST", &["MONITOR_CONVST_TEST"]);
        for i in 1..8 {
            bel = bel
                .pin_name_only(&format!("VP{i}"), 1)
                .pin_name_only(&format!("VN{i}"), 1);
        }
        for i in 0..16 {
            bel = bel.extra_wire(format!("GIOB{i}"), &[format!("SYS_MON_GIOB{i}")]);
        }
        builder.extract_xnode_bels(
            "SYSMON",
            xy,
            &[],
            &int_xy,
            "SYSMON",
            &[
                bel,
                builder.bel_xy("IPAD0", "IPAD", 0, 0).pins_name_only(&["O"]),
                builder.bel_xy("IPAD1", "IPAD", 0, 1).pins_name_only(&["O"]),
            ],
        );
    }

    builder.build()
}
