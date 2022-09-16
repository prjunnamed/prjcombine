use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::Part;

use enum_map::enum_map;

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("ultrascaleplus", rd);

    let d2n = enum_map!(
        Dir::N => 0,
        Dir::S => 1,
        Dir::E => 2,
        Dir::W => 3,
    );

    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..16 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::ClkOut(i),
            &[format!("GCLK_B_0_{i}")],
        );
    }

    for (ih, h) in ["E", "W"].into_iter().enumerate() {
        for i in 0..96 {
            match i {
                0 | 2 => {
                    let w = builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("SDQNODE_{h}_{i}_FT1")],
                    );
                    builder.branch(
                        w,
                        Dir::S,
                        format!("SDQNODE.{h}.{i}.S"),
                        &[format!("SDQNODE_{h}_BLS_{i}_FT0")],
                    );
                }
                91 | 93 | 95 => {
                    let w = builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("SDQNODE_{h}_{i}_FT0")],
                    );
                    builder.branch(
                        w,
                        Dir::N,
                        format!("SDQNODE.{h}.{i}.N"),
                        &[format!("SDQNODE_{h}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 33, 44, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17,
                        18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38,
                        39, 40, 41, 42, 43, 45, 46, 47,
                    ][i >> 1];
                    let aa = a + ih * 48;
                    let b = i & 1;
                    builder.mux_out(
                        format!("SDQNODE.{h}.{i}"),
                        &[format!("INT_NODE_SDQ_{aa}_INT_OUT{b}")],
                    );
                }
            }
        }
    }
    for (dir, name, l, ll, fts, ftn) in [
        (Dir::E, "SNG", 1, 1, false, false),
        (Dir::W, "SNG", 1, 1, false, true),
        (Dir::N, "SNG", 1, 1, false, false),
        (Dir::S, "SNG", 1, 1, false, false),
        (Dir::E, "DBL", 1, 2, false, false),
        (Dir::W, "DBL", 1, 2, true, false),
        (Dir::N, "DBL", 2, 2, false, false),
        (Dir::S, "DBL", 2, 2, false, false),
        (Dir::E, "QUAD", 2, 4, false, false),
        (Dir::W, "QUAD", 2, 4, false, false),
        (Dir::N, "QUAD", 4, 4, false, true),
        (Dir::S, "QUAD", 4, 4, true, false),
    ] {
        let ftd = d2n[!dir];
        for ew in ['E', 'W'] {
            for i in 0..8 {
                match (ll, dir, ew) {
                    (1, Dir::E, 'W') => {
                        let (a, b) = [
                            (60, 1),
                            (4, 0),
                            (61, 1),
                            (5, 0),
                            (62, 1),
                            (6, 0),
                            (63, 1),
                            (7, 0),
                        ][i];
                        builder.mux_out(
                            format!("{name}.{dir}.{ew}.{i}.0"),
                            &[format!("INT_INT_SDQ_{a}_INT_OUT{b}")],
                        );
                    }
                    (1, Dir::W, 'E') => {
                        if i == 7 {
                            let w = builder.mux_out(
                                format!("{name}.{dir}.{ew}.{i}.0"),
                                &[format!("{dir}{dir}{ll}_{ew}_{i}_FT0")],
                            );
                            builder.branch(
                                w,
                                Dir::N,
                                format!("{name}.{dir}.{ew}.{i}.{l}.N"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLN_{i}_FT1")],
                            );
                        } else {
                            let (a, b) = [
                                (72, 0),
                                (32, 1),
                                (73, 0),
                                (33, 1),
                                (74, 0),
                                (34, 1),
                                (75, 0),
                            ][i];
                            builder.mux_out(
                                format!("{name}.{dir}.{ew}.{i}.0"),
                                &[format!("INT_INT_SDQ_{a}_INT_OUT{b}")],
                            );
                        }
                    }
                    _ => {
                        let mut w = builder.mux_out(
                            format!("{name}.{dir}.{ew}.{i}.0"),
                            &[format!("{dir}{dir}{ll}_{ew}_BEG{i}")],
                        );
                        for j in 1..l {
                            let nn = (b'A' + (j - 1)) as char;
                            w = builder.branch(
                                w,
                                dir,
                                format!("{name}.{dir}.{ew}.{i}.{j}"),
                                &[format!("{dir}{dir}{ll}_{ew}_{nn}_FT{ftd}_{i}")],
                            );
                        }
                        w = builder.branch(
                            w,
                            dir,
                            format!("{name}.{dir}.{ew}.{i}.{l}"),
                            &[format!("{dir}{dir}{ll}_{ew}_END{i}")],
                        );
                        if i == 0 && fts {
                            builder.branch(
                                w,
                                Dir::S,
                                format!("{name}.{dir}.{ew}.{i}.{l}.S"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLS_{i}_FT0")],
                            );
                        }
                        if i == 7 && ftn {
                            builder.branch(
                                w,
                                Dir::N,
                                format!("{name}.{dir}.{ew}.{i}.{l}.N"),
                                &[format!("{dir}{dir}{ll}_{ew}_BLN_{i}_FT1")],
                            );
                        }
                    }
                }
            }
        }
    }

    for (dir, name, l, fts, ftn) in [
        (Dir::E, "LONG", 6, true, true),
        (Dir::W, "LONG", 6, false, false),
        (Dir::N, "LONG", 12, false, false),
        (Dir::S, "LONG", 12, false, false),
    ] {
        let ftd = d2n[!dir];
        for i in 0..8 {
            let mut w = builder.mux_out(
                format!("{name}.{dir}.{i}.0"),
                &[format!("{dir}{dir}12_BEG{i}")],
            );
            for j in 1..l {
                let nn = (b'A' + (j - 1)) as char;
                w = builder.branch(
                    w,
                    dir,
                    format!("{name}.{dir}.{i}.{j}"),
                    &[format!("{dir}{dir}12_{nn}_FT{ftd}_{i}")],
                );
            }
            w = builder.branch(
                w,
                dir,
                format!("{name}.{dir}.{i}.{l}"),
                &[format!("{dir}{dir}12_END{i}")],
            );
            if i == 0 && fts {
                builder.branch(
                    w,
                    Dir::S,
                    format!("{name}.{dir}.{i}.{l}.S"),
                    &[format!("{dir}{dir}12_BLS_{i}_FT0")],
                );
            }
            if i == 7 && ftn {
                builder.branch(
                    w,
                    Dir::N,
                    format!("{name}.{dir}.{i}.{l}.N"),
                    &[format!("{dir}{dir}12_BLN_{i}_FT1")],
                );
            }
        }
    }

    for i in 0..16 {
        for j in 0..2 {
            builder.mux_out(
                format!("INT_NODE_GLOBAL.{i}.{j}"),
                &[format!("INT_NODE_GLOBAL_{i}_INT_OUT{j}")],
            );
        }
    }
    for i in 0..8 {
        builder.mux_out(format!("IMUX.E.CTRL.{i}"), &[format!("CTRL_E{i}")]);
    }
    for i in 0..10 {
        builder.mux_out(format!("IMUX.W.CTRL.{i}"), &[format!("CTRL_W{i}")]);
    }

    for (ih, h) in ['E', 'W'].into_iter().enumerate() {
        for i in 0..64 {
            match i {
                1 | 3 | 5 | 9 => {
                    let w =
                        builder.mux_out(format!("INODE.{h}.{i}"), &[format!("INODE_{h}_{i}_FT1")]);
                    builder.branch(
                        w,
                        Dir::S,
                        format!("INODE.{h}.{i}.S"),
                        &[format!("INODE_{h}_BLS_{i}_FT0")],
                    );
                }
                54 | 58 | 60 | 62 => {
                    let w =
                        builder.mux_out(format!("INODE.{h}.{i}"), &[format!("INODE_{h}_{i}_FT0")]);
                    builder.branch(
                        w,
                        Dir::N,
                        format!("INODE.{h}.{i}.N"),
                        &[format!("INODE_{h}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 30, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17,
                        18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29,
                    ][i >> 1];
                    let aa = a + ih * 32;
                    let b = i & 1;
                    let w = builder.mux_out(format!("INODE.{h}.{i}"), &[""]);
                    builder.extra_name_tile("INT", format!("INT_NODE_IMUX_{aa}_INT_OUT{b}"), w);
                }
            }
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..16 {
            match i {
                0 | 2 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FT1")],
                    );
                    builder.branch(
                        w,
                        Dir::S,
                        format!("IMUX.{ew}.BYP.{i}.S"),
                        &[format!("BOUNCE_{ew}_BLS_{i}_FT0")],
                    );
                }
                13 | 15 => {
                    let w = builder.mux_out(
                        format!("IMUX.{ew}.BYP.{i}"),
                        &[format!("BOUNCE_{ew}_{i}_FT0")],
                    );
                    builder.branch(
                        w,
                        Dir::N,
                        format!("IMUX.{ew}.BYP.{i}.N"),
                        &[format!("BOUNCE_{ew}_BLN_{i}_FT1")],
                    );
                }
                _ => {
                    builder.mux_out(format!("IMUX.{ew}.BYP.{i}"), &[format!("BYPASS_{ew}{i}")]);
                }
            }
        }
    }
    for ew in ['E', 'W'] {
        for i in 0..48 {
            builder.mux_out(format!("IMUX.{ew}.IMUX.{i}"), &[format!("IMUX_{ew}{i}")]);
        }
    }

    for ew in ['E', 'W'] {
        for i in 0..32 {
            builder.logic_out(format!("OUT.{ew}.{i}"), &[format!("LOGIC_OUTS_{ew}{i}")]);
        }
    }

    for i in 0..32 {
        builder.mux_out(
            format!("RCLK.IMUX.CE.{i}"),
            &[format!("CLK_LEAF_SITES_{i}_CE_INT")],
        );
    }
    builder.mux_out("RCLK.IMUX.ENSEL_PROG", &["CLK_LEAF_SITES_0_ENSEL_PROG"]);
    builder.mux_out("RCLK.IMUX.CLK_CASC_IN", &["CLK_LEAF_SITES_0_CLK_CASC_IN"]);
    for i in 0..2 {
        for j in 0..4 {
            builder.mux_out(
                format!("RCLK.IMUX.LEFT.{i}.{j}"),
                &[format!("INT_RCLK_TO_CLK_LEFT_{i}_{j}")],
            );
        }
    }
    for i in 0..2 {
        for j in 0..3 {
            builder.mux_out(
                format!("RCLK.IMUX.RIGHT.{i}.{j}"),
                &[format!("INT_RCLK_TO_CLK_RIGHT_{i}_{j}")],
            );
        }
    }
    for i in 0..2 {
        for j in 0..24 {
            let w = builder.mux_out(format!("RCLK.INODE.{i}.{j}"), &[""]);
            builder.extra_name_tile("RCLK_INT_L", format!("INT_NODE_IMUX_{j}_INT_OUT{i}"), w);
            builder.extra_name_tile("RCLK_INT_R", format!("INT_NODE_IMUX_{j}_INT_OUT{i}"), w);
        }
    }
    for i in 0..48 {
        let w = builder.wire(format!("RCLK.GND.{i}"), WireKind::Tie0, &[""]);
        builder.extra_name_tile("RCLK_INT_L", format!("GND_WIRE{i}"), w);
        builder.extra_name_tile("RCLK_INT_R", format!("GND_WIRE{i}"), w);
    }

    builder.extract_main_passes();

    builder.node_type("INT", "INT", "INT");

    builder.extract_term_conn("TERM.W", Dir::W, "INT_INTF_L_TERM_GT", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INT_INTF_LEFT_TERM_PSS", &[]);
    builder.extract_term_conn("TERM.W", Dir::W, "INT_INTF_LEFT_TERM_IO_FT", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INT_INTF_R_TERM_GT", &[]);
    builder.extract_term_conn("TERM.E", Dir::E, "INT_INTF_RIGHT_TERM_IO", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_TERM_B", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_TERM_P", &[]);
    builder.extract_term_conn("TERM.S", Dir::S, "INT_INT_TERM_H_FT", &[]);
    builder.extract_term_conn("TERM.N", Dir::N, "INT_TERM_T", &[]);

    for (dir, tkn) in [(Dir::W, "INT_INTF_L"), (Dir::E, "INT_INTF_R")] {
        builder.extract_intf(format!("INTF.{dir}"), dir, tkn, format!("INTF.{dir}"), true);
    }

    builder.extract_intf(
        "INTF.W.IO",
        Dir::W,
        "INT_INTF_LEFT_TERM_PSS",
        "INTF.PSS",
        true,
    );
    for (dir, tkn) in [
        (Dir::W, "INT_INTF_LEFT_TERM_IO_FT"),
        (Dir::W, "INT_INTF_L_CMT"),
        (Dir::W, "INT_INTF_L_IO"),
        (Dir::E, "INT_INTF_RIGHT_TERM_IO"),
    ] {
        builder.extract_intf(
            format!("INTF.{dir}.IO"),
            dir,
            tkn,
            format!("INTF.{dir}.IO"),
            true,
        );
    }

    for (dir, n, tkn) in [
        (Dir::W, "PCIE", "INT_INTF_L_PCIE4"),
        (Dir::E, "PCIE", "INT_INTF_R_PCIE4"),
        (Dir::W, "GT", "INT_INTF_L_TERM_GT"),
        (Dir::E, "GT", "INT_INTF_R_TERM_GT"),
    ] {
        builder.extract_intf(
            format!("INTF.{dir}.DELAY"),
            dir,
            tkn,
            format!("INTF.{dir}.{n}"),
            true,
        );
    }

    builder.extract_pass_simple("IO", Dir::W, "INT_IBRK_FSR2IO", &[]);

    for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = xy.delta(0, 1);
            builder.extract_xnode("RCLK", xy, &[], &[int_xy], "RCLK", &[], &[]);
        }
    }

    for (tkn, kind, key) in [
        ("CLEL_L", "CLEL_L", "SLICE_L"),
        ("CLEL_R", "CLEL_R", "SLICE_R"),
        ("CLEM", "CLEM", "SLICE_L"),
        ("CLEM_R", "CLEM", "SLICE_L"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = xy.delta(if key == "SLICE_L" { 1 } else { -1 }, 0);
            builder.extract_xnode_bels(
                kind,
                xy,
                &[],
                &[int_xy],
                kind,
                &[builder
                    .bel_xy(key, "SLICE", 0, 0)
                    .pin_name_only("CIN", 1)
                    .pin_name_only("COUT", 0)],
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF.W");
        for dy in 0..5 {
            int_xy.push(xy.delta(2, dy));
            intf_xy.push((xy.delta(1, dy), n));
        }
        let mut bel_bram_f = builder
            .bel_xy("BRAM_F", "RAMB36", 0, 0)
            .pin_name_only("CASINSBITERR", 1)
            .pin_name_only("CASINDBITERR", 1)
            .pin_name_only("CASOUTSBITERR", 0)
            .pin_name_only("CASOUTDBITERR", 0)
            .pin_name_only("CASPRVEMPTY", 1)
            .pin_name_only("CASPRVRDEN", 1)
            .pin_name_only("CASNXTEMPTY", 1)
            .pin_name_only("CASNXTRDEN", 1)
            .pin_name_only("CASMBIST12OUT", 0)
            .pin_name_only("ENABLE_BIST", 1)
            .pin_name_only("START_RSR_NEXT", 0);
        let mut bel_bram_h0 = builder
            .bel_xy("BRAM_H0", "RAMB18", 0, 0)
            .pin_name_only("CASPRVEMPTY", 0)
            .pin_name_only("CASPRVRDEN", 0)
            .pin_name_only("CASNXTEMPTY", 0)
            .pin_name_only("CASNXTRDEN", 0);
        let mut bel_bram_h1 = builder.bel_xy("BRAM_H1", "RAMB18", 0, 1);
        for ab in ['A', 'B'] {
            for ul in ['U', 'L'] {
                for i in 0..16 {
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDI{ab}{ul}{i}"), 1);
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDO{ab}{ul}{i}"), 1);
                }
                for i in 0..2 {
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDIP{ab}{ul}{i}"), 1);
                    bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDOP{ab}{ul}{i}"), 1);
                }
            }
            for i in 0..16 {
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDI{ab}L{i}"), 0);
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDO{ab}L{i}"), 0);
            }
            for i in 0..2 {
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDIP{ab}L{i}"), 0);
                bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDOP{ab}L{i}"), 0);
            }
            for i in 0..16 {
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDI{ab}U{i}"), 0);
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDO{ab}U{i}"), 0);
            }
            for i in 0..2 {
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDIP{ab}U{i}"), 0);
                bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDOP{ab}U{i}"), 0);
            }
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

    for tkn in [
        "RCLK_BRAM_INTF_L",
        "RCLK_BRAM_INTF_TD_L",
        "RCLK_BRAM_INTF_TD_R",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let n = builder.db.get_node_naming("INTF.W");
            let int_xy = xy.delta(2, 1);
            let intf_xy = (xy.delta(1, 1), n);

            let mut bels = vec![];
            for (i, x, y) in [(0, 0, 0), (1, 0, 1), (2, 1, 0), (3, 1, 1)] {
                bels.push(builder.bel_xy(format!("HARD_SYNC{i}"), "HARD_SYNC", x, y));
            }
            builder.extract_xnode_bels_intf(
                "HARD_SYNC",
                xy,
                &[],
                &[int_xy],
                &[intf_xy],
                "HARD_SYNC",
                &bels,
            );
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("DSP").iter().next() {
        let mut int_xy = Vec::new();
        let mut intf_xy = Vec::new();
        let n = builder.db.get_node_naming("INTF.E");
        for dy in 0..5 {
            int_xy.push(xy.delta(-2, dy));
            intf_xy.push((xy.delta(-1, dy), n));
        }

        let mut bels_dsp = vec![];
        for i in 0..2 {
            let mut bel = builder.bel_xy(format!("DSP{i}"), "DSP48E2", 0, i);
            let buf_cnt = match i {
                0 => 1,
                1 => 0,
                _ => unreachable!(),
            };
            bel = bel.pin_name_only("MULTSIGNIN", buf_cnt);
            bel = bel.pin_name_only("MULTSIGNOUT", 0);
            bel = bel.pin_name_only("CARRYCASCIN", buf_cnt);
            bel = bel.pin_name_only("CARRYCASCOUT", 0);
            for j in 0..30 {
                bel = bel.pin_name_only(&format!("ACIN_B{j}"), buf_cnt);
                bel = bel.pin_name_only(&format!("ACOUT_B{j}"), 0);
            }
            for j in 0..18 {
                bel = bel.pin_name_only(&format!("BCIN_B{j}"), buf_cnt);
                bel = bel.pin_name_only(&format!("BCOUT_B{j}"), 0);
            }
            for j in 0..48 {
                bel = bel.pin_name_only(&format!("PCIN{j}"), buf_cnt);
                bel = bel.pin_name_only(&format!("PCOUT{j}"), 0);
            }
            bels_dsp.push(bel);
        }
        builder.extract_xnode_bels_intf("DSP", xy, &[], &int_xy, &intf_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BLI_BLI_FT").iter().next() {
        let intf = builder.db.get_node_naming("INTF.E");
        let bels = [
            builder.bel_xy("BLI_HBM_APB_INTF", "BLI_HBM_APB_INTF", 0, 0),
            builder.bel_xy("BLI_HBM_AXI_INTF", "BLI_HBM_AXI_INTF", 0, 0),
        ];
        let mut xn = builder.xnode("BLI", "BLI", xy).num_tiles(15);
        for i in 0..15 {
            xn = xn
                .ref_int(xy.delta(-2, i as i32), i)
                .ref_single(xy.delta(-1, i as i32), i, intf)
        }
        xn.bels(bels).extract();
    }

    for tkn in ["URAM_URAM_FT", "URAM_URAM_DELAY_FT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            let mut intf_xy = Vec::new();
            let nr = builder.db.get_node_naming("INTF.E");
            let nl = builder.db.get_node_naming("INTF.W");
            for dy in 0..15 {
                int_xy.push(xy.delta(-2, dy));
                intf_xy.push((xy.delta(-1, dy), nr));
            }
            for dy in 0..15 {
                int_xy.push(xy.delta(2, dy));
                intf_xy.push((xy.delta(1, dy), nl));
            }

            let mut bels = vec![];
            for i in 0..4 {
                let mut bel = builder.bel_xy(format!("URAM{i}"), "URAM288", 0, i);
                let buf_cnt = match i {
                    0 => 1,
                    _ => 0,
                };
                for ab in ['A', 'B'] {
                    for j in 0..23 {
                        bel = bel.pin_name_only(&format!("CAS_IN_ADDR_{ab}{j}"), buf_cnt);
                        bel = bel.pin_name_only(&format!("CAS_OUT_ADDR_{ab}{j}"), 0);
                    }
                    for j in 0..9 {
                        bel = bel.pin_name_only(&format!("CAS_IN_BWE_{ab}{j}"), buf_cnt);
                        bel = bel.pin_name_only(&format!("CAS_OUT_BWE_{ab}{j}"), 0);
                    }
                    for j in 0..72 {
                        bel = bel.pin_name_only(&format!("CAS_IN_DIN_{ab}{j}"), buf_cnt);
                        bel = bel.pin_name_only(&format!("CAS_OUT_DIN_{ab}{j}"), 0);
                        bel = bel.pin_name_only(&format!("CAS_IN_DOUT_{ab}{j}"), buf_cnt);
                        bel = bel.pin_name_only(&format!("CAS_OUT_DOUT_{ab}{j}"), 0);
                    }
                    for pin in ["EN", "RDACCESS", "RDB_WR", "DBITERR", "SBITERR"] {
                        bel = bel.pin_name_only(&format!("CAS_IN_{pin}_{ab}"), buf_cnt);
                        bel = bel.pin_name_only(&format!("CAS_OUT_{pin}_{ab}"), 0);
                    }
                }
                bels.push(bel);
            }
            builder.extract_xnode_bels_intf("URAM", xy, &[], &int_xy, &intf_xy, "URAM", &bels);
        }
    }

    'a: {
        if let Some(&xy) = rd.tiles_by_kind_name("LAG_LAG").iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                break 'a;
            }
            let mut bels = vec![];
            for i in 0..4 {
                let mut bel = builder.bel_xy(&format!("LAGUNA{i}"), "LAGUNA", i >> 1, i & 1);
                for j in 0..6 {
                    bel = bel
                        .pin_name_only(&format!("RXQ{j}"), 0)
                        .pin_name_only(&format!("RXD{j}"), 0)
                        .pin_name_only(&format!("TXQ{j}"), 0)
                        .extra_int_out(&format!("RXOUT{j}"), &[format!("RXD{ii}", ii = i * 6 + j)])
                        .extra_wire(
                            &format!("TXOUT{j}"),
                            &[format!(
                                "LAG_MUX_ATOM_{ii}_TXOUT",
                                ii = match (i, j) {
                                    (0, 0) => 0,
                                    (0, 1) => 11,
                                    (0, 2) => 16,
                                    (0, 3) => 17,
                                    (0, 4) => 18,
                                    (0, 5) => 19,
                                    (1, 0) => 20,
                                    (1, 1) => 21,
                                    (1, 2) => 22,
                                    (1, 3) => 23,
                                    (1, 4) => 1,
                                    (1, 5) => 2,
                                    (2, 0) => 3,
                                    (2, 1) => 4,
                                    (2, 2) => 5,
                                    (2, 3) => 6,
                                    (2, 4) => 7,
                                    (2, 5) => 8,
                                    (3, 0) => 9,
                                    (3, 1) => 10,
                                    (3, 2) => 12,
                                    (3, 3) => 13,
                                    (3, 4) => 14,
                                    (3, 5) => 15,
                                    _ => unreachable!(),
                                }
                            )],
                        )
                        .extra_wire(
                            &format!("UBUMP{j}"),
                            &[format!("UBUMP{ii}", ii = i * 6 + j)],
                        );
                }
                bels.push(bel);
            }
            bels.push(builder.bel_virtual("VCC").extra_wire("VCC", &["VCC_WIRE"]));
            builder.extract_xnode_bels("LAGUNA", xy, &[], &[xy.delta(2, 0)], "LAGUNA", &bels);
        }
    }

    for (kind, tkn, bk) in [
        ("PCIE4", "PCIE4_PCIE4_FT", "PCIE40E4"),
        ("PCIE4C", "PCIE4C_PCIE4C_FT", "PCIE4CE4"),
        ("CMAC", "CMAC", "CMACE4"),
        ("ILKN", "ILKN_ILKN_FT", "ILKNE4"),
        ("DFE_A", "DFE_DFE_TILEA_FT", "DFE_A"),
        ("DFE_C", "DFE_DFE_TILEC_FT", "DFE_C"),
        ("DFE_D", "DFE_DFE_TILED_FT", "DFE_D"),
        ("DFE_E", "DFE_DFE_TILEE_FT", "DFE_E"),
        ("DFE_F", "DFE_DFE_TILEF_FT", "DFE_F"),
        ("DFE_G", "DFE_DFE_TILEG_FT", "DFE_G"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                continue;
            }
            let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
            let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
            let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
            let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
            let mut bel = builder.bel_xy(kind, bk, 0, 0);
            if matches!(kind, "PCIE4" | "PCIE4C") {
                bel = bel.pin_dummy("MCAP_PERST0_B").pin_dummy("MCAP_PERST1_B");
            }
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(120);
            for i in 0..60 {
                xn = xn
                    .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                    .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
            }
            xn.bel(bel).extract();
        }
    }

    'a: {
        if let Some(&xy) = rd.tiles_by_kind_name("DFE_DFE_TILEB_FT").iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                break 'a;
            }
            let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
            let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
            let bel = builder.bel_xy("DFE_B", "DFE_B", 0, 0);
            let mut xn = builder.xnode("DFE_B", "DFE_B", xy).num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l);
            }
            xn.bel(bel).extract();
        }
    }

    'a: {
        if let Some(&xy) = rd.tiles_by_kind_name("FE_FE_FT").iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                break 'a;
            }
            let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
            let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
            let bel = builder.bel_xy("FE", "FE", 0, 0);
            let mut xn = builder.xnode("FE", "FE", xy).num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r);
            }
            xn.bel(bel).extract();
        }
    }

    'a: {
        if let Some(&xy) = rd.tiles_by_kind_name("PSS_ALTO").iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                break 'a;
            }
            let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
            let intf_r = builder.db.get_node_naming("INTF.PSS");
            let mut bel = builder.bel_xy("PS", "PS8", 0, 0).pins_name_only(&[
                "DP_AUDIO_REF_CLK",
                "DP_VIDEO_REF_CLK",
                "DDR_DTO0",
                "DDR_DTO1",
                "APLL_TEST_CLK_OUT0",
                "APLL_TEST_CLK_OUT1",
                "RPLL_TEST_CLK_OUT0",
                "RPLL_TEST_CLK_OUT1",
                "DPLL_TEST_CLK_OUT0",
                "DPLL_TEST_CLK_OUT1",
                "IOPLL_TEST_CLK_OUT0",
                "IOPLL_TEST_CLK_OUT1",
                "VPLL_TEST_CLK_OUT0",
                "VPLL_TEST_CLK_OUT1",
                "FMIO_GEM0_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM0_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM1_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM1_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM2_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM2_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM3_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM3_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM_TSU_CLK_TO_PL_BUFG",
                "PL_CLK0",
                "PL_CLK1",
                "PL_CLK2",
                "PL_CLK3",
                "O_DBG_L0_RXCLK",
                "O_DBG_L0_TXCLK",
                "O_DBG_L1_RXCLK",
                "O_DBG_L1_TXCLK",
                "O_DBG_L2_RXCLK",
                "O_DBG_L2_TXCLK",
                "O_DBG_L3_RXCLK",
                "O_DBG_L3_TXCLK",
                "PS_PL_SYSOSC_CLK",
                "BSCAN_RESET_TAP_B",
                "BSCAN_CLOCKDR",
                "BSCAN_SHIFTDR",
                "BSCAN_UPDATEDR",
                "BSCAN_INTEST",
                "BSCAN_EXTEST",
                "BSCAN_INIT_MEMORY",
                "BSCAN_AC_TEST",
                "BSCAN_AC_MODE",
                "BSCAN_MISR_JTAG_LOAD",
                "PSS_CFG_RESET_B",
                "PSS_FST_CFG_B",
                "PSS_GTS_CFG_B",
                "PSS_GTS_USR_B",
                "PSS_GHIGH_B",
                "PSS_GPWRDWN_B",
                "PCFG_POR_B",
            ]);

            for pin in [
                "IDCODE15",
                "IDCODE16",
                "IDCODE17",
                "IDCODE18",
                "IDCODE20",
                "IDCODE21",
                "IDCODE28",
                "IDCODE29",
                "IDCODE30",
                "IDCODE31",
                "PS_VERSION_0",
                "PS_VERSION_2",
                "PS_VERSION_3",
            ] {
                bel = bel.pin_dummy(pin);
            }
            let mut xn = builder.xnode("PS", "PS", xy).num_tiles(180);
            for i in 0..180 {
                xn = xn
                    .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r);
            }
            xn.bel(bel).extract();
        }
    }

    'a: {
        if let Some(&xy) = rd.tiles_by_kind_name("VCU_VCU_FT").iter().next() {
            let tk = &rd.tile_kinds[rd.tiles[&xy].kind];
            if tk.sites.is_empty() {
                break 'a;
            }
            let int_r_xy = builder.walk_to_int(xy.delta(0, 2), Dir::E).unwrap();
            let intf_r = builder.db.get_node_naming("INTF.PSS");
            let bel = builder
                .bel_xy("VCU", "VCU", 0, 0)
                .pins_name_only(&["VCU_PLL_TEST_CLK_OUT0", "VCU_PLL_TEST_CLK_OUT1"]);
            let mut xn = builder.xnode("VCU", "VCU", xy).num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf_r);
            }
            xn.bel(bel).extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CONFIG").iter().next() {
        let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let bels = [
            builder.bel_xy("CFG", "CONFIG_SITE", 0, 0),
            builder
                .bel_xy("ABUS_SWITCH", "ABUS_SWITCH", 0, 0)
                .pins_name_only(&["TEST_ANALOGBUS_SEL_B"]),
        ];
        let mut xn = builder.xnode("CFG", "CFG", xy).num_tiles(120);
        for i in 0..60 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFGIO_IOB20").iter().next() {
        let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let bels = [
            builder.bel_xy("PMV", "PMV", 0, 0),
            builder.bel_xy("PMV2", "PMV2", 0, 0),
            builder.bel_xy("PMVIOB", "PMVIOB", 0, 0),
            builder.bel_xy("MTBF3", "MTBF3", 0, 0),
            builder.bel_xy("CFGIO_SITE", "CFGIO_SITE", 0, 0),
        ];
        let mut xn = builder.xnode("CFGIO", "CFGIO", xy).num_tiles(60);
        for i in 0..30 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("AMS").iter().next() {
        let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let mut bel = builder.bel_xy("SYSMON", "SYSMONE4", 0, 0);
        for i in 0..16 {
            bel = bel.pins_name_only(&[format!("VP_AUX{i}"), format!("VN_AUX{i}")]);
        }
        let mut xn = builder.xnode("AMS", "AMS", xy).num_tiles(60);
        for i in 0..30 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bel(bel).extract();
    }

    builder.build()
}
