#![allow(clippy::needless_range_loop)]
#![allow(clippy::collapsible_else_if)]

use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::Part;
use prjcombine_ultrascale::DeviceNaming;

use enum_map::enum_map;

use prjcombine_rdintb::IntBuilder;

const XLAT24: [usize; 24] = [
    0, 11, 16, 17, 18, 19, 20, 21, 22, 23, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15,
];

pub fn make_int_db(rd: &Part, dev_naming: &DeviceNaming) -> IntDb {
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
            let mut bels = vec![];
            for ud in ['D', 'U'] {
                for i in 0..16 {
                    let mut bel = builder
                        .bel_xy(
                            format!("BUFCE_LEAF_{ud}{i}"),
                            "BUFCE_LEAF",
                            i & 7,
                            i / 8 + 2 * u8::from(ud == 'U'),
                        )
                        .pins_name_only(&["CLK_CASC_OUT", "CLK_IN"]);
                    if i != 0 || ud == 'U' {
                        bel = bel.pins_name_only(&["CLK_CASC_IN"]);
                    }
                    bels.push(bel);
                }
            }
            let mut bel = builder
                .bel_virtual("RCLK_INT")
                .extra_wire("VCC", &["VCC_WIRE"]);
            for i in 0..24 {
                bel = bel.extra_wire(format!("HDISTR{i}"), &[format!("CLK_HDISTR_FT0_{i}")]);
            }
            bels.push(bel);
            builder
                .xnode("RCLK_INT", "RCLK_INT", xy)
                .num_tiles(2)
                .ref_int(xy.delta(0, 1), 0)
                .ref_int(xy.delta(0, -1), 1)
                .extract_muxes()
                .bels(bels)
                .extract();
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

    if let Some(&xy) = rd.tiles_by_kind_name("LAG_LAG").iter().next() {
        let mut bels = vec![];
        for i in 0..4 {
            let mut bel = builder.bel_xy(format!("LAGUNA{i}"), "LAGUNA", i >> 1, i & 1);
            for j in 0..6 {
                bel = bel
                    .pin_name_only(&format!("RXQ{j}"), 0)
                    .pin_name_only(&format!("RXD{j}"), 0)
                    .pin_name_only(&format!("TXQ{j}"), 0)
                    .extra_int_out(format!("RXOUT{j}"), &[format!("RXD{ii}", ii = i * 6 + j)])
                    .extra_wire(
                        format!("TXOUT{j}"),
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
                    .extra_wire(format!("UBUMP{j}"), &[format!("UBUMP{ii}", ii = i * 6 + j)]);
            }
            bels.push(bel);
        }
        bels.push(
            builder
                .bel_virtual("VCC.LAGUNA")
                .extra_wire("VCC", &["VCC_WIRE"]),
        );
        builder.extract_xnode_bels("LAGUNA", xy, &[], &[xy.delta(2, 0)], "LAGUNA", &bels);
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

    for tkn in [
        "RCLK_INTF_LEFT_TERM_ALTO",
        "RCLK_RCLK_INTF_LEFT_TERM_DA6_FT",
        "RCLK_INTF_LEFT_TERM_DA7",
        "RCLK_RCLK_INTF_LEFT_TERM_DA8_FT",
        "RCLK_RCLK_INTF_LEFT_TERM_DC12_FT",
        "RCLK_RCLK_INTF_LEFT_TERM_MX8_FT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let mut bels = vec![];
            for i in 0..24 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFG_PS{i}"), "BUFG_PS", 0, i as u8)
                        .pins_name_only(&["CLK_IN", "CLK_OUT"])
                        .extra_wire(
                            "CLK_IN_DUMMY",
                            &[format!(
                                "VCC_WIRE{ii}",
                                ii = [
                                    0, 3, 8, 9, 10, 11, 13, 14, 15, 16, 1, 12, 17, 18, 19, 20, 21,
                                    22, 23, 2, 4, 5, 6, 7
                                ][i]
                            )],
                        ),
                );
            }
            let mut bel = builder
                .bel_virtual("RCLK_PS")
                .extra_int_in("CKINT", &["INT_RCLK_TO_CLK_0_FT1_0"]);
            for i in 0..18 {
                bel = bel.extra_wire(format!("PS_TO_PL_CLK{i}"), &[format!("PS_TO_PL_CLK{i}")]);
            }
            for i in 0..24 {
                bel = bel.extra_wire(format!("HROUTE{i}"), &[format!("CLK_HROUTE{i}")]);
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual("VCC.RCLK_PS")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            builder
                .xnode("RCLK_PS", "RCLK_PS", xy)
                .ref_xlat(xy.delta(1, 0), &[Some(0), None], rclk_int)
                .bels(bels)
                .extract();
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
                .bel_xy("ABUS_SWITCH.CFG", "ABUS_SWITCH", 0, 0)
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
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 30)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 30, intf_r)
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
            bel = bel
                .pin_name_only(&format!("VP_AUX{i}"), 1)
                .pin_name_only(&format!("VN_AUX{i}"), 1);
        }
        let mut xn = builder.xnode("AMS", "AMS", xy).num_tiles(60);
        for i in 0..30 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 30)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 30, intf_r)
        }
        xn.bel(bel).extract();
    }

    for tkn in ["HDIO_BOT_RIGHT", "HDIO_TOP_RIGHT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_bot = tkn == "HDIO_BOT_RIGHT";
            let int_l_xy = builder.walk_to_int(xy, Dir::W).unwrap();
            let int_r_xy = builder.walk_to_int(xy, Dir::E).unwrap();
            let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
            let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
            let mut bels = vec![];
            for i in 0..6 {
                bels.extend([
                    builder
                        .bel_xy(format!("HDIOB_M{i}"), "IOB", 0, 2 * i)
                        .pins_name_only(&[
                            "OP",
                            "TSP",
                            "O_B",
                            "TSTATEB",
                            "OUTB_B",
                            "OUTB_B_IN",
                            "TSTATE_IN",
                            "TSTATE_OUT",
                            "LVDS_TRUE",
                            "PAD_RES",
                            "I",
                        ])
                        .pin_name_only("SWITCH_OUT", 1)
                        .pin_dummy("IO"),
                    builder
                        .bel_xy(format!("HDIOB_S{i}"), "IOB", 0, 2 * i + 1)
                        .pins_name_only(&[
                            "OP",
                            "TSP",
                            "O_B",
                            "TSTATEB",
                            "OUTB_B",
                            "OUTB_B_IN",
                            "TSTATE_IN",
                            "TSTATE_OUT",
                            "LVDS_TRUE",
                            "PAD_RES",
                            "I",
                        ])
                        .pin_name_only("SWITCH_OUT", 1)
                        .pin_dummy("IO"),
                ]);
            }
            for i in 0..6 {
                bels.push(
                    builder
                        .bel_xy(format!("HDIODIFFIN{i}"), "HDIOBDIFFINBUF", 0, i)
                        .pins_name_only(&["LVDS_TRUE", "LVDS_COMP", "PAD_RES_0", "PAD_RES_1"]),
                );
            }
            for i in 0..6 {
                bels.extend([
                    builder
                        .bel_xy(format!("HDIOLOGIC_M{i}"), "HDIOLOGIC_M", 0, i)
                        .pins_name_only(&["OPFFM_Q", "TFFM_Q", "IPFFM_D"]),
                    builder
                        .bel_xy(format!("HDIOLOGIC_S{i}"), "HDIOLOGIC_S", 0, i)
                        .pins_name_only(&["OPFFS_Q", "TFFS_Q", "IPFFS_D"]),
                ]);
            }
            bels.push(builder.bel_xy("HDLOGIC_CSSD", "HDLOGIC_CSSD", 0, 0));
            if is_bot {
                bels.push(builder.bel_xy("HDIO_VREF", "HDIO_VREF", 0, 0));
            } else {
                bels.push(builder.bel_xy("HDIO_BIAS", "HDIO_BIAS", 0, 0));
            }
            let kind = if is_bot { "HDIO_BOT" } else { "HDIO_TOP" };
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(60);
            for i in 0..30 {
                xn = xn
                    .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 30)
                    .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 30, intf_r)
            }
            xn.bels(bels).extract();
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("RCLK_HDIO").iter().next() {
        let top_xy = xy.delta(0, -30);
        let int_l_xy = builder.walk_to_int(top_xy, Dir::W).unwrap();
        let int_r_xy = builder.walk_to_int(top_xy, Dir::E).unwrap();
        let intf_l = builder.db.get_node_naming("INTF.E.PCIE");
        let intf_r = builder.db.get_node_naming("INTF.W.PCIE");
        let mut bels = vec![];
        for i in 0..4 {
            bels.push(
                builder
                    .bel_xy(format!("BUFGCE_HDIO{i}"), "BUFGCE_HDIO", i >> 1, i & 1)
                    .pins_name_only(&["CLK_IN", "CLK_OUT"])
                    .extra_wire("CLK_IN_MUX", &[format!("CLK_CMT_MUX_4TO1_{i}_CLK_OUT")]),
            );
        }
        for (i, x, y) in [
            (0, 0, 0),
            (1, 0, 1),
            (2, 1, 0),
            (3, 1, 1),
            (4, 2, 0),
            (5, 2, 1),
            (6, 3, 0),
        ] {
            bels.push(builder.bel_xy(format!("ABUS_SWITCH.HDIO{i}"), "ABUS_SWITCH", x, y));
        }
        let mut bel = builder
            .bel_virtual("RCLK_HDIO")
            .extra_int_in("CKINT", &["CLK_INT_TOP"]);
        for i in 0..4 {
            bel = bel.extra_wire(format!("CCIO{i}"), &[format!("CCIO_IO2RCLK{i}")]);
        }
        for i in 0..24 {
            bel = bel
                .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")])
                .extra_wire(format!("HDISTR{i}"), &[format!("CLK_HDISTR_FT0_{i}")])
                .extra_wire(
                    format!("HROUTE{i}_L_MUX"),
                    &[format!(
                        "CLK_CMT_MUX_2TO1_{ii}_CLK_OUT",
                        ii = XLAT24[i] * 2 + 5
                    )],
                )
                .extra_wire(
                    format!("HROUTE{i}_R_MUX"),
                    &[format!(
                        "CLK_CMT_MUX_2TO1_{ii}_CLK_OUT",
                        ii = XLAT24[i] * 2 + 4
                    )],
                )
                .extra_wire(
                    format!("HDISTR{i}_MUX"),
                    &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = XLAT24[i] + 4)],
                );
        }
        bels.push(bel);
        bels.push(
            builder
                .bel_virtual("VCC.RCLK_HDIO")
                .extra_wire("VCC", &["VCC_WIRE"]),
        );
        let mut xn = builder.xnode("RCLK_HDIO", "RCLK_HDIO", xy).num_tiles(120);
        for i in 0..60 {
            xn = xn
                .ref_int(int_l_xy.delta(0, (i + i / 30) as i32), i)
                .ref_int(int_r_xy.delta(0, (i + i / 30) as i32), i + 60)
                .ref_single(int_l_xy.delta(1, (i + i / 30) as i32), i, intf_l)
                .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_r)
        }
        xn.bels(bels).extract();
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFRM_CFRAME_TERM_H_FT").iter().next() {
        let mut bels = vec![];
        for i in 0..8 {
            bels.push(
                builder
                    .bel_xy(format!("ABUS_SWITCH.HBM{i}"), "ABUS_SWITCH", i >> 1, i & 1)
                    .pins_name_only(&["TEST_ANALOGBUS_SEL_B"]),
            );
        }
        builder
            .xnode("HBM_ABUS_SWITCH", "HBM_ABUS_SWITCH", xy)
            .num_tiles(0)
            .bels(bels)
            .extract();
    }

    for tkn in [
        "PCIE4_PCIE4_FT",
        "PCIE4C_PCIE4C_FT",
        "CMAC",
        "ILKN_ILKN_FT",
        "DFE_DFE_TILEA_FT",
        "DFE_DFE_TILEB_FT",
        "DFE_DFE_TILEE_FT",
        "DFE_DFE_TILEG_FT",
        "CFG_CONFIG",
        "RCLK_AMS_CFGIO",
        "RCLK_CLEM_CLKBUF_L",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("RCLK_HROUTE_SPLITTER");
            for i in 0..24 {
                bel = bel
                    .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                    .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")]);
            }
            let bel_vcc = builder
                .bel_virtual("VCC.RCLK_HROUTE_SPLITTER")
                .extra_wire("VCC", &["VCC_WIRE"]);
            let kind = if tkn == "DFE_DFE_TILEB_FT" {
                "RCLK_HROUTE_SPLITTER_R"
            } else {
                "RCLK_HROUTE_SPLITTER_L"
            };
            builder
                .xnode(kind, "RCLK_HROUTE_SPLITTER", xy)
                .num_tiles(0)
                .bel(bel)
                .bel(bel_vcc)
                .extract();
        }
    }

    if let Some(&xy) = rd
        .tiles_by_kind_name("RCLK_DSP_INTF_CLKBUF_L")
        .iter()
        .next()
    {
        let mut bel = builder.bel_virtual("RCLK_SPLITTER");
        for i in 0..24 {
            bel = bel
                .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_L{i}")])
                .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_R{i}")])
                .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")]);
        }
        let bel_vcc = builder
            .bel_virtual("VCC.RCLK_SPLITTER")
            .extra_wire("VCC", &["VCC_WIRE"]);
        builder
            .xnode("RCLK_SPLITTER", "RCLK_SPLITTER", xy)
            .num_tiles(0)
            .bel(bel)
            .bel(bel_vcc)
            .extract();
    }

    for tkn in [
        "RCLK_CLEL_L_L",
        "RCLK_CLEL_L_R",
        "RCLK_CLEM_L",
        "RCLK_CLEM_DMC_L",
        "RCLK_CLEM_R",
        "RCLK_LAG_L",
        "RCLK_LAG_R",
        "RCLK_LAG_DMC_L",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_alt = dev_naming.rclk_alt_pins[tkn];
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let int_xy = xy.delta(if tkn.starts_with("RCLK_LAG") { 2 } else { 1 }, 0);
            let bels = vec![
                builder
                    .bel_xy("BUFCE_ROW_L0", "BUFCE_ROW_FSR", 0, 0)
                    .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                    .extra_wire("VDISTR_B", &["CLK_VDISTR_BOT"])
                    .extra_wire("VDISTR_T", &["CLK_VDISTR_TOP"])
                    .extra_wire("VROUTE_B", &["CLK_VROUTE_BOT"])
                    .extra_wire("VROUTE_T", &["CLK_VROUTE_TOP"])
                    .extra_wire("HROUTE", &["CLK_HROUTE_CORE_OPT"])
                    .extra_wire("VDISTR_B_MUX", &["CLK_CMT_MUX_3TO1_0_CLK_OUT"])
                    .extra_wire("VDISTR_T_MUX", &["CLK_CMT_MUX_3TO1_1_CLK_OUT"])
                    .extra_wire("VROUTE_B_MUX", &["CLK_CMT_MUX_3TO1_2_CLK_OUT"])
                    .extra_wire("VROUTE_T_MUX", &["CLK_CMT_MUX_3TO1_3_CLK_OUT"])
                    .extra_wire("HROUTE_MUX", &["CLK_CMT_MUX_2TO1_1_CLK_OUT"])
                    .extra_wire(
                        "VDISTR_B_BUF",
                        &["CLK_CMT_DRVR_TRI_ESD_0_CLK_OUT_SCHMITT_B"],
                    )
                    .extra_wire(
                        "VDISTR_T_BUF",
                        &["CLK_CMT_DRVR_TRI_ESD_1_CLK_OUT_SCHMITT_B"],
                    )
                    .extra_wire(
                        "VROUTE_B_BUF",
                        &["CLK_CMT_DRVR_TRI_ESD_2_CLK_OUT_SCHMITT_B"],
                    )
                    .extra_wire(
                        "VROUTE_T_BUF",
                        &["CLK_CMT_DRVR_TRI_ESD_3_CLK_OUT_SCHMITT_B"],
                    ),
                builder
                    .bel_xy("GCLK_TEST_BUF_L0", "GCLK_TEST_BUFE3", 0, 0)
                    .pin_name_only("CLK_OUT", 0)
                    .pin_name_only("CLK_IN", usize::from(is_alt)),
                builder
                    .bel_virtual("VCC.RCLK_V_L")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            ];
            builder
                .xnode(
                    "RCLK_V_SINGLE_L",
                    if is_alt {
                        "RCLK_V_SINGLE_L.ALT"
                    } else {
                        "RCLK_V_SINGLE_L"
                    },
                    xy,
                )
                .ref_xlat(int_xy, &[Some(0), None], rclk_int)
                .bels(bels)
                .extract();
        }
    }

    for tkn in [
        "RCLK_DSP_INTF_L",
        "RCLK_DSP_INTF_R",
        "RCLK_RCLK_DSP_INTF_DC12_L_FT",
        "RCLK_RCLK_DSP_INTF_DC12_R_FT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_alt = dev_naming.rclk_alt_pins[tkn];
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let int_xy = xy.delta(-1, 0);
            let mut bels = vec![];
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFCE_ROW_R{i}"), "BUFCE_ROW_FSR", i, 0)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                        .extra_wire("VDISTR_B", &[format!("CLK_VDISTR_BOT{i}")])
                        .extra_wire("VDISTR_T", &[format!("CLK_VDISTR_TOP{i}")])
                        .extra_wire("VROUTE_B", &[format!("CLK_VROUTE_BOT{i}")])
                        .extra_wire("VROUTE_T", &[format!("CLK_VROUTE_TOP{i}")])
                        .extra_wire("HROUTE", &[format!("CLK_HROUTE_CORE_OPT{i}")])
                        .extra_wire(
                            "VDISTR_B_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4)],
                        )
                        .extra_wire(
                            "VDISTR_T_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 1)],
                        )
                        .extra_wire(
                            "VROUTE_B_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 2)],
                        )
                        .extra_wire(
                            "VROUTE_T_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 3)],
                        )
                        .extra_wire(
                            "HROUTE_MUX",
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 2 + 1)],
                        )
                        .extra_wire(
                            "VDISTR_B_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4
                            )],
                        )
                        .extra_wire(
                            "VDISTR_T_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4 + 1
                            )],
                        )
                        .extra_wire(
                            "VROUTE_B_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4 + 2
                            )],
                        )
                        .extra_wire(
                            "VROUTE_T_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4 + 3
                            )],
                        ),
                );
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("GCLK_TEST_BUF_R{i}"), "GCLK_TEST_BUFE3", i, 0)
                        .pin_name_only("CLK_OUT", 0)
                        .pin_name_only("CLK_IN", usize::from(is_alt)),
                );
            }
            bels.push(
                builder
                    .bel_virtual("VCC.RCLK_V_R")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            builder
                .xnode(
                    "RCLK_V_DOUBLE_R",
                    if is_alt {
                        "RCLK_V_DOUBLE_R.ALT"
                    } else {
                        "RCLK_V_DOUBLE_R"
                    },
                    xy,
                )
                .ref_xlat(int_xy, &[Some(0), None], rclk_int)
                .bels(bels)
                .extract();
        }
    }

    for tkn in [
        "RCLK_BRAM_INTF_L",
        "RCLK_BRAM_INTF_TD_L",
        "RCLK_BRAM_INTF_TD_R",
        "RCLK_RCLK_URAM_INTF_L_FT",
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let is_alt = dev_naming.rclk_alt_pins[tkn];
            let is_uram = tkn == "RCLK_RCLK_URAM_INTF_L_FT";
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let int_xy = xy.delta(if is_uram { 3 } else { 2 }, 0);
            let mut bels = vec![];
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFCE_ROW_L{i}"), "BUFCE_ROW_FSR", i, 0)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                        .extra_wire("VDISTR_B", &[format!("CLK_VDISTR_BOT{i}")])
                        .extra_wire("VDISTR_T", &[format!("CLK_VDISTR_TOP{i}")])
                        .extra_wire("VROUTE_B", &[format!("CLK_VROUTE_BOT{i}")])
                        .extra_wire("VROUTE_T", &[format!("CLK_VROUTE_TOP{i}")])
                        .extra_wire("HROUTE", &[format!("CLK_HROUTE_CORE_OPT{i}")])
                        .extra_wire(
                            "VDISTR_B_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4)],
                        )
                        .extra_wire(
                            "VDISTR_T_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 1)],
                        )
                        .extra_wire(
                            "VROUTE_B_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 2)],
                        )
                        .extra_wire(
                            "VROUTE_T_MUX",
                            &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 3)],
                        )
                        .extra_wire(
                            "HROUTE_MUX",
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 2 + 1)],
                        )
                        .extra_wire(
                            "VDISTR_B_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4
                            )],
                        )
                        .extra_wire(
                            "VDISTR_T_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4 + 1
                            )],
                        )
                        .extra_wire(
                            "VROUTE_B_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4 + 2
                            )],
                        )
                        .extra_wire(
                            "VROUTE_T_BUF",
                            &[format!(
                                "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                ii = i * 4 + 3
                            )],
                        ),
                );
            }
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(format!("GCLK_TEST_BUF_L{i}"), "GCLK_TEST_BUFE3", i, 0)
                        .pin_name_only("CLK_OUT", 0)
                        .pin_name_only("CLK_IN", usize::from(is_alt)),
                );
            }
            for (i, x, y) in [(0, 0, 0), (1, 0, 1), (2, 1, 0)] {
                bels.push(builder.bel_xy(format!("VBUS_SWITCH{i}"), "VBUS_SWITCH", x, y));
            }
            bels.push(
                builder
                    .bel_virtual("VCC.RCLK_V_L")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            builder
                .xnode(
                    "RCLK_V_QUAD_L",
                    if is_uram {
                        if is_alt {
                            "RCLK_V_QUAD_L.URAM.ALT"
                        } else {
                            "RCLK_V_QUAD_L.URAM"
                        }
                    } else {
                        if is_alt {
                            "RCLK_V_QUAD_L.ALT"
                        } else {
                            "RCLK_V_QUAD_L"
                        }
                    },
                    xy,
                )
                .ref_xlat(int_xy, &[Some(0), None], rclk_int)
                .bels(bels)
                .extract();
        }
    }

    for (kind, tkn) in [
        ("CMT_L", "CMT_L"),
        ("CMT_L_HBM", "CMT_LEFT_H"),
        ("CMT_R", "CMT_RIGHT"),
    ] {
        let is_l = tkn != "CMT_RIGHT";
        let is_hbm = tkn == "CMT_LEFT_H";
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder
                .walk_to_int(xy, if is_l { Dir::E } else { Dir::W })
                .unwrap();
            let intf = builder
                .db
                .get_node_naming(if is_l { "INTF.W.IO" } else { "INTF.E.IO" });
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let mut bels = vec![];
            for i in 0..24 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFCE_ROW_IO{i}"), "BUFCE_ROW", 0, i)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"]),
                );
            }
            for i in 0..24 {
                bels.push(
                    builder
                        .bel_xy(
                            format!("GCLK_TEST_BUF_IO{i}"),
                            "GCLK_TEST_BUFE3",
                            0,
                            if i < 18 { i } else { i + 1 },
                        )
                        .pins_name_only(&["CLK_IN", "CLK_OUT"]),
                );
            }
            for i in 0..24 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFGCE{i}"), "BUFGCE", 0, i)
                        .pins_name_only(&["CLK_OUT"])
                        .pin_name_only("CLK_IN", usize::from(matches!(i, 5 | 11 | 17 | 23)))
                        .extra_wire(
                            "CLK_IN_MUX_HROUTE",
                            &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = i * 2 + 1)],
                        )
                        .extra_wire(
                            "CLK_IN_MUX_PLL_CKINT",
                            &[format!(
                                "CLK_CMT_MUX_3TO1_{ii}_CLK_OUT",
                                ii = i % 3 + i / 3 * 5
                            )],
                        )
                        .extra_wire(
                            "CLK_IN_MUX_TEST",
                            &[format!("CLK_CMT_MUX_4TO1_{i}_CLK_OUT")],
                        )
                        .extra_int_in("CLK_IN_CKINT", &[format!("CLK_INT{i}")]),
                );
            }
            for i in 0..8 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFGCTRL{i}"), "BUFGCTRL", 0, i)
                        .pins_name_only(&["CLK_I0", "CLK_I1", "CLK_OUT"]),
                );
            }
            for i in 0..4 {
                bels.push(
                    builder
                        .bel_xy(format!("BUFGCE_DIV{i}"), "BUFGCE_DIV", 0, i)
                        .pins_name_only(&["CLK_IN", "CLK_OUT"]),
                );
            }
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(format!("PLL{i}"), "PLL", 0, i)
                    .pins_name_only(&[
                        "CLKOUT0",
                        "CLKOUT0B",
                        "CLKOUT1",
                        "CLKOUT1B",
                        "CLKFBOUT",
                        "TMUXOUT",
                        "CLKOUTPHY_P",
                        "CLKIN",
                        "CLKFBIN",
                    ])
                    .extra_wire(
                        "CLKIN_MUX_MMCM",
                        &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = 24 + i)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_HDISTR",
                        &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 60 + i * 3)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_HROUTE",
                        &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 61 + i * 3)],
                    )
                    .extra_wire(
                        "CLKIN_MUX_BUFCE_ROW_DLY",
                        &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 62 + i * 3)],
                    );
                if is_hbm {
                    // the muxes are repurposed for HBM reference
                    bel = bel.pin_name_only("CLKFBIN", 1);
                } else {
                    bel = bel
                        .extra_wire(
                            "CLKFBIN_MUX_HDISTR",
                            &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 56 + i * 2)],
                        )
                        .extra_wire(
                            "CLKFBIN_MUX_BUFCE_ROW_DLY",
                            &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 57 + i * 2)],
                        );
                }
                bels.push(bel);
            }
            bels.push(
                builder
                    .bel_xy("MMCM", "MMCM", 0, 0)
                    .pins_name_only(&[
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
                        "CLKIN1",
                        "CLKIN2",
                        "CLKFBIN",
                    ])
                    .extra_wire("CLKFBIN_MUX_HDISTR", &["CLK_CMT_MUX_24_ENC_48_CLK_OUT"])
                    .extra_wire(
                        "CLKFBIN_MUX_BUFCE_ROW_DLY",
                        &["CLK_CMT_MUX_24_ENC_49_CLK_OUT"],
                    )
                    .extra_wire("CLKFBIN_MUX_DUMMY0", &["VCC_WIRE51"])
                    .extra_wire("CLKFBIN_MUX_DUMMY1", &["VCC_WIRE52"])
                    .extra_wire("CLKIN1_MUX_HDISTR", &["CLK_CMT_MUX_24_ENC_50_CLK_OUT"])
                    .extra_wire("CLKIN1_MUX_HROUTE", &["CLK_CMT_MUX_24_ENC_51_CLK_OUT"])
                    .extra_wire(
                        "CLKIN1_MUX_BUFCE_ROW_DLY",
                        &["CLK_CMT_MUX_24_ENC_52_CLK_OUT"],
                    )
                    .extra_wire("CLKIN1_MUX_DUMMY0", &["GND_WIRE0"])
                    .extra_wire("CLKIN2_MUX_HDISTR", &["CLK_CMT_MUX_24_ENC_53_CLK_OUT"])
                    .extra_wire("CLKIN2_MUX_HROUTE", &["CLK_CMT_MUX_24_ENC_54_CLK_OUT"])
                    .extra_wire(
                        "CLKIN2_MUX_BUFCE_ROW_DLY",
                        &["CLK_CMT_MUX_24_ENC_55_CLK_OUT"],
                    )
                    .extra_wire("CLKIN2_MUX_DUMMY0", &["GND_WIRE1"]),
            );
            bels.push(builder.bel_xy("ABUS_SWITCH.CMT", "ABUS_SWITCH", 0, 0));
            if is_hbm {
                for i in 0..2 {
                    bels.push(
                        builder
                            .bel_xy(format!("HBM_REF_CLK{i}"), "HBM_REF_CLK", 0, i)
                            .pins_name_only(&["REF_CLK"])
                            .extra_wire(
                                "REF_CLK_MUX_HDISTR",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 56 + i * 2)],
                            )
                            .extra_wire(
                                "REF_CLK_MUX_BUFCE_ROW_DLY",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 57 + i * 2)],
                            ),
                    );
                }
            }
            let mut bel = builder.bel_virtual("CMT");
            for i in 0..4 {
                bel = bel.extra_wire(format!("CCIO{i}"), &[format!("IOB2CLK_CCIO{i}")]);
            }
            for i in 0..8 {
                bel = bel.extra_wire(
                    format!("FIFO_WRCLK{i}"),
                    &[format!("PHY2RCLK_SS_DIVCLK_{j}_{k}", j = i / 2, k = i % 2)],
                );
            }
            for i in 0..24 {
                let dummy_base = [
                    0, 3, 36, 53, 56, 59, 62, 65, 68, 71, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 39,
                    42, 45, 48,
                ][i];
                bel = bel
                    .extra_wire(format!("VDISTR{i}_B"), &[format!("CLK_VDISTR_BOT{i}")])
                    .extra_wire(format!("VDISTR{i}_T"), &[format!("CLK_VDISTR_TOP{i}")])
                    .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_0_{i}")])
                    .extra_wire(
                        format!("HDISTR{i}_R"),
                        &[format!("CLK_CMT_DRVR_TRI_{ii}_CLK_OUT_B", ii = i * 4)],
                    )
                    .extra_wire(
                        format!("HDISTR{i}_L_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 1 + i * 8)],
                    )
                    .extra_wire(
                        format!("HDISTR{i}_R_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 8)],
                    )
                    .extra_wire(
                        format!("HDISTR{i}_OUT_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 4 + i * 8)],
                    )
                    .extra_wire(
                        format!("VDISTR{i}_B_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 6 + i * 8)],
                    )
                    .extra_wire(
                        format!("VDISTR{i}_T_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 7 + i * 8)],
                    )
                    .extra_wire(
                        format!("OUT_MUX{i}"),
                        &[format!("CLK_CMT_MUX_16_ENC_{i}_CLK_OUT")],
                    )
                    .extra_wire(
                        format!("OUT_MUX{i}_DUMMY0"),
                        &[format!("VCC_WIRE{ii}", ii = dummy_base)],
                    )
                    .extra_wire(
                        format!("OUT_MUX{i}_DUMMY1"),
                        &[format!("VCC_WIRE{ii}", ii = dummy_base + 1)],
                    )
                    .extra_wire(
                        format!("OUT_MUX{i}_DUMMY2"),
                        &[format!("VCC_WIRE{ii}", ii = dummy_base + 2)],
                    );
                if is_l {
                    bel = bel
                        .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_0_{i}")])
                        .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_1_{i}")])
                        .extra_wire(
                            format!("HROUTE{i}_L_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 3 + i * 8)],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_R_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 2 + i * 8)],
                        );
                } else {
                    bel = bel
                        .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_1_{i}")])
                        .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_0_{i}")])
                        .extra_wire(
                            format!("HROUTE{i}_L_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 2 + i * 8)],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_R_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 3 + i * 8)],
                        );
                }
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual("VCC.CMT")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            let mut xn = builder.xnode(kind, kind, xy).num_tiles(60).ref_single(
                int_xy.delta(0, 30),
                30,
                rclk_int,
            );
            for i in 0..60 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(
                        int_xy.delta(if is_l { -1 } else { 1 }, (i + i / 30) as i32),
                        i,
                        intf,
                    )
            }
            xn.bels(bels).extract();
        }
    }

    for (kind, tkn) in [("XIPHY_L", "XIPHY_BYTE_L"), ("XIPHY_R", "XIPHY_BYTE_RIGHT")] {
        let is_l = tkn != "XIPHY_BYTE_RIGHT";
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder
                .walk_to_int(xy, if is_l { Dir::E } else { Dir::W })
                .unwrap();
            let intf = builder
                .db
                .get_node_naming(if is_l { "INTF.W.IO" } else { "INTF.E.IO" });
            let mut bels = vec![];
            for i in 0..13 {
                let mut bel = builder
                    .bel_xy(format!("BITSLICE_RX_TX{i}"), "BITSLICE_RX_TX", 0, i)
                    .pins_name_only(&[
                        "TX_CLK",
                        "TX_OCLK",
                        "TX_DIV2_CLK",
                        "TX_DIV4_CLK",
                        "TX_DDR_CLK",
                        "TX_CTRL_CLK",
                        "TX_CTRL_CE",
                        "TX_CTRL_INC",
                        "TX_CTRL_LD",
                        "TX_OCLKDIV",
                        "TX_TBYTE_IN",
                        "TX_WL_TRAIN",
                        "TX_MUX_360_N_SEL",
                        "TX_MUX_360_P_SEL",
                        "TX_MUX_720_P0_SEL",
                        "TX_MUX_720_P1_SEL",
                        "TX_MUX_720_P2_SEL",
                        "TX_MUX_720_P3_SEL",
                        "TX_VTC_READY",
                        "TX_TOGGLE_DIV2_SEL",
                        "TX_BS_RESET",
                        "TX_REGRST",
                        "TX_RST",
                        "TX_Q",
                        "RX_CLK_C",
                        "RX_CLK_C_B",
                        "RX_CLK_P",
                        "RX_CLK_N",
                        "RX_CTRL_CLK",
                        "RX_CTRL_CE",
                        "RX_CTRL_INC",
                        "RX_CTRL_LD",
                        "RX_RST",
                        "RX_CLKDIV",
                        "RX_DCC0",
                        "RX_DCC1",
                        "RX_DCC2",
                        "RX_DCC3",
                        "RX_VTC_READY",
                        "RX_RESET",
                        "RX_BS_RESET",
                        "RX_DQS_OUT",
                        "TX2RX_CASC_IN",
                        "TX2RX_CASC_OUT",
                        "RX2TX_CASC_RETURN_IN",
                        "PHY2CLB_FIFO_WRCLK",
                        "CLB2PHY_FIFO_CLK",
                        "CTL2BS_FIFO_BYPASS",
                        "CTL2BS_RX_RECALIBRATE_EN",
                        "CTL2BS_TX_DDR_PHASE_SEL",
                        "CTL2BS_DYNAMIC_MODE_EN",
                        "BS2CTL_IDELAY_DELAY_FORMAT",
                        "BS2CTL_ODELAY_DELAY_FORMAT",
                        "BS2CTL_TX_DDR_PHASE_SEL",
                        "BS2CTL_RX_P0_DQ_OUT",
                        "BS2CTL_RX_N0_DQ_OUT",
                        "BS2CTL_RX_DDR_EN_DQS",
                    ])
                    .pin_name_only("RX_CLK", 1)
                    .pin_name_only("RX_D", 1)
                    .extra_wire("DYN_DCI_OUT", &[format!("PHY2IOB_ODT_OUT_BYTE{i}")])
                    .extra_int_in(
                        "DYN_DCI_OUT_INT",
                        &[if i < 6 {
                            format!("CLB2PHY_ODT_LOW{i}")
                        } else {
                            format!("CLB2PHY_ODT_UPP{ii}", ii = i - 6)
                        }],
                    );
                for i in 0..18 {
                    bel = bel.pins_name_only(&[
                        format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{i}"),
                        format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{i}"),
                    ]);
                }
                for i in 0..9 {
                    bel = bel.pins_name_only(&[
                        format!("BS2CTL_RX_CNTVALUEOUT{i}"),
                        format!("BS2CTL_TX_CNTVALUEOUT{i}"),
                        format!("RX_CTRL_DLY{i}"),
                        format!("TX_CTRL_DLY{i}"),
                    ]);
                }
                bels.push(bel);
            }
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(format!("BITSLICE_TX{i}"), "BITSLICE_TX", 0, i)
                    .pins_name_only(&[
                        "CLK",
                        "DIV2_CLK",
                        "DIV4_CLK",
                        "DDR_CLK",
                        "CTRL_CLK",
                        "CTRL_CE",
                        "CTRL_INC",
                        "CTRL_LD",
                        "TX_MUX_360_N_SEL",
                        "TX_MUX_360_P_SEL",
                        "TX_MUX_720_P0_SEL",
                        "TX_MUX_720_P1_SEL",
                        "TX_MUX_720_P2_SEL",
                        "TX_MUX_720_P3_SEL",
                        "TOGGLE_DIV2_SEL",
                        "D0",
                        "D1",
                        "D2",
                        "D3",
                        "D4",
                        "D5",
                        "D6",
                        "D7",
                        "Q",
                        "RST",
                        "REGRST",
                        "BS_RESET",
                        "CDATAIN0",
                        "CDATAIN1",
                        "CDATAOUT",
                        "CTL2BS_TX_DDR_PHASE_SEL",
                        "CTL2BS_DYNAMIC_MODE_EN",
                        "BS2CTL_TX_DDR_PHASE_SEL",
                        "FORCE_OE_B",
                        "VTC_READY",
                    ]);
                for i in 0..9 {
                    bel = bel.pins_name_only(&[
                        format!("BS2CTL_CNTVALUEOUT{i}"),
                        format!("CTRL_DLY{i}"),
                    ]);
                }
                bels.push(bel);
            }
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(format!("BITSLICE_CONTROL{i}"), "BITSLICE_CONTROL", 0, i)
                    .pins_name_only(&[
                        "PDQS_GT_IN",
                        "NDQS_GT_IN",
                        "PDQS_GT_OUT",
                        "NDQS_GT_OUT",
                        "FORCE_OE_B",
                        "PLL_CLK",
                        "PLL_CLK_EN",
                        "REFCLK_DFD",
                        "CLK_TO_EXT_SOUTH",
                        "CLK_TO_EXT_NORTH",
                        "CLB2PHY_CTRL_RST",
                        "LOCAL_DIV_CLK",
                        "BS_RESET_TRI",
                        "TRISTATE_ODELAY_CE_OUT",
                        "TRISTATE_ODELAY_INC_OUT",
                        "TRISTATE_ODELAY_LD_OUT",
                        "TRISTATE_VTC_READY",
                        "SCAN_INT",
                        "RIU2CLB_VALID",
                        "CLK_STOP",
                        "CLK_FROM_EXT",
                    ])
                    .pin_name_only("CLK_FROM_EXT", 1);
                for i in 0..7 {
                    bel = bel.pins_name_only(&[
                        format!("RX_DCC{i:02}_0"),
                        format!("RX_DCC{i:02}_1"),
                        format!("RX_DCC{i:02}_2"),
                        format!("RX_DCC{i:02}_3"),
                        format!("RX_PDQ{i}_IN"),
                        format!("RX_NDQ{i}_IN"),
                        format!("IDELAY_CTRL_CLK{i}"),
                        format!("IDELAY_CE_OUT{i}"),
                        format!("IDELAY_INC_OUT{i}"),
                        format!("IDELAY_LD_OUT{i}"),
                        format!("FIXED_IDELAY{i:02}"),
                        format!("ODELAY_CE_OUT{i}"),
                        format!("ODELAY_INC_OUT{i}"),
                        format!("ODELAY_LD_OUT{i}"),
                        format!("FIXED_ODELAY{i:02}"),
                        format!("VTC_READY_IDELAY{i:02}"),
                        format!("VTC_READY_ODELAY{i:02}"),
                        format!("WL_TRAIN{i}"),
                        format!("DYN_DCI_OUT{i}"),
                        format!("DQS_IN{i}"),
                        format!("RX_BS_RESET{i}"),
                        format!("TX_BS_RESET{i}"),
                        format!("PDQS_OUT{i}"),
                        format!("NDQS_OUT{i}"),
                        format!("REFCLK_EN{i}"),
                        format!("IFIFO_BYPASS{i}"),
                        format!("BS2CTL_RIU_BS_DQS_EN{i}"),
                    ]);
                    for j in 0..9 {
                        bel = bel.pins_name_only(&[
                            format!("IDELAY{i:02}_IN{j}"),
                            format!("IDELAY{i:02}_OUT{j}"),
                            format!("ODELAY{i:02}_IN{j}"),
                            format!("ODELAY{i:02}_OUT{j}"),
                        ]);
                    }
                    for j in 0..18 {
                        bel = bel.pins_name_only(&[
                            format!("FIXDLYRATIO_IDELAY{i:02}_{j}"),
                            format!("FIXDLYRATIO_ODELAY{i:02}_{j}"),
                        ]);
                    }
                }
                for i in 0..8 {
                    bel = bel.pins_name_only(&[
                        format!("ODELAY_CTRL_CLK{i}"),
                        format!("DYNAMIC_MODE_EN{i}"),
                        format!("EN_DIV_DLY_OE{i}"),
                        format!("TOGGLE_DIV2_SEL{i}"),
                        format!("TX_DATA_PHASE{i}"),
                        format!("BS2CTL_RIU_TX_DATA_PHASE{i}"),
                        format!("DIV2_CLK_OUT{i}"),
                        format!("DIV_CLK_OUT{i}"),
                        format!("DDR_CLK_OUT{i}"),
                        format!("PH02_DIV2_360_{i}"),
                        format!("PH13_DIV2_360_{i}"),
                        format!("PH0_DIV_720_{i}"),
                        format!("PH1_DIV_720_{i}"),
                        format!("PH2_DIV_720_{i}"),
                        format!("PH3_DIV_720_{i}"),
                    ]);
                }
                for i in 0..9 {
                    bel = bel.pins_name_only(&[
                        format!("TRISTATE_ODELAY_IN{i}"),
                        format!("TRISTATE_ODELAY_OUT{i}"),
                    ]);
                }
                for i in 0..16 {
                    bel = bel.pins_name_only(&[format!("RIU2CLB_RD_DATA{i}")]);
                }
                bels.push(bel);
            }
            for i in 0..2 {
                bels.push(
                    builder
                        .bel_xy(format!("PLL_SELECT{i}"), "PLL_SELECT_SITE", 0, i)
                        .pins_name_only(&["REFCLK_DFD", "Z", "PLL_CLK_EN"])
                        .pin_name_only("D0", 1)
                        .pin_name_only("D1", 1),
                );
            }
            let mut bel = builder
                .bel_xy("RIU_OR0", "RIU_OR", 0, 0)
                .pins_name_only(&["RIU_RD_VALID_LOW", "RIU_RD_VALID_UPP"]);
            for i in 0..16 {
                bel = bel.pins_name_only(&[
                    format!("RIU_RD_DATA_LOW{i}"),
                    format!("RIU_RD_DATA_UPP{i}"),
                ]);
            }
            bels.push(bel);
            let mut bel = builder
                .bel_xy("XIPHY_FEEDTHROUGH0", "XIPHY_FEEDTHROUGH", 0, 0)
                .pins_name_only(&[
                    "CLB2PHY_CTRL_RST_LOW_SMX",
                    "CLB2PHY_CTRL_RST_UPP_SMX",
                    "CLB2PHY_TRISTATE_ODELAY_RST_SMX0",
                    "CLB2PHY_TRISTATE_ODELAY_RST_SMX1",
                    "CLB2PHY_TXBIT_TRI_RST_SMX0",
                    "CLB2PHY_TXBIT_TRI_RST_SMX1",
                    "SCAN_INT_LOWER",
                    "SCAN_INT_UPPER",
                    "DIV_CLK_OUT_LOW",
                    "DIV_CLK_OUT_UPP",
                    "XIPHY_CLK_STOP_CTRL_LOW",
                    "XIPHY_CLK_STOP_CTRL_UPP",
                    "RCLK2PHY_CLKDR",
                    "RCLK2PHY_SHIFTDR",
                ]);
            for i in 0..13 {
                bel = bel.pins_name_only(&[
                    format!("CLB2PHY_TXBIT_RST_SMX{i}"),
                    format!("CLB2PHY_RXBIT_RST_SMX{i}"),
                    format!("CLB2PHY_FIFO_CLK_SMX{i}"),
                    format!("CLB2PHY_IDELAY_RST_SMX{i}"),
                    format!("CLB2PHY_ODELAY_RST_SMX{i}"),
                ]);
            }
            for i in 0..6 {
                bel = bel.pins_name_only(&[format!("CTL2BS_REFCLK_EN_LOW_SMX{i}")]);
            }
            for i in 0..7 {
                bel = bel.pins_name_only(&[
                    format!("CTL2BS_REFCLK_EN_LOW{i}"),
                    format!("CTL2BS_REFCLK_EN_UPP{i}"),
                    format!("CTL2BS_REFCLK_EN_UPP_SMX{i}"),
                ]);
            }
            bels.push(bel);
            let mut bel = builder.bel_virtual("XIPHY_BYTE");
            for i in 0..6 {
                bel = bel.extra_wire(format!("XIPHY_CLK{i}"), &[format!("GCLK_FT0_{i}")]);
            }
            bels.push(bel);

            let mut xn = builder.xnode(kind, kind, xy).num_tiles(15);
            for i in 0..15 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(
                        int_xy.delta(if is_l { -1 } else { 1 }, (i + i / 30) as i32),
                        i,
                        intf,
                    )
            }
            xn.bels(bels).extract();
        }
    }

    for (tkn, kind) in [
        ("RCLK_RCLK_XIPHY_INNER_FT", "RCLK_XIPHY_L"),
        ("RCLK_XIPHY_OUTER_RIGHT", "RCLK_XIPHY_R"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual("RCLK_XIPHY");
            for i in 0..24 {
                if kind == "RCLK_XIPHY_L" {
                    bel = bel
                        .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_ZERO{i}")])
                        .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_ONE{i}")]);
                } else {
                    bel = bel
                        .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_ONE{i}")])
                        .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_ZERO{i}")]);
                }
            }
            for i in 0..6 {
                bel = bel
                    .extra_wire(
                        format!("XIPHY_CLK{i}_B"),
                        &[format!("CLK_TO_XIPHY_BYTES_BOT{i}")],
                    )
                    .extra_wire(
                        format!("XIPHY_CLK{i}_T"),
                        &[format!("CLK_TO_XIPHY_BYTES_TOP{i}")],
                    );
            }
            let bel_vcc = builder
                .bel_virtual("VCC.RCLK_XIPHY")
                .extra_wire("VCC", &["VCC_WIRE"]);
            builder
                .xnode(kind, kind, xy)
                .num_tiles(0)
                .bel(bel)
                .bel(bel_vcc)
                .extract();
        }
    }

    for (kind, tkn) in [("HPIO_L", "HPIO_L"), ("HPIO_R", "HPIO_RIGHT")] {
        let is_l = tkn != "HPIO_RIGHT";
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder
                .walk_to_int(xy, if is_l { Dir::E } else { Dir::W })
                .unwrap();
            let intf = builder
                .db
                .get_node_naming(if is_l { "INTF.W.IO" } else { "INTF.E.IO" });
            let mut bels = vec![];

            for i in 0..26 {
                let mut bel = builder
                    .bel_xy(format!("HPIOB{i}"), "IOB", 0, i)
                    .pins_name_only(&[
                        "I",
                        "OUTB_B_IN",
                        "OUTB_B",
                        "TSTATE_IN",
                        "TSTATE_OUT",
                        "DOUT",
                        "IO",
                        "LVDS_TRUE",
                        "PAD_RES",
                        "O_B",
                        "TSTATEB",
                        "DYNAMIC_DCI_TS",
                        "VREF",
                    ])
                    .pin_name_only("SWITCH_OUT", 1)
                    .pin_name_only("OP", 1)
                    .pin_name_only("TSP", 1)
                    .pin_dummy("TSDI");
                if matches!(i, 12 | 25) {
                    bel = bel
                        .pin_dummy("IO")
                        .pin_dummy("LVDS_TRUE")
                        .pin_dummy("OUTB_B_IN")
                        .pin_dummy("TSTATE_IN");
                }
                bels.push(bel);
            }
            for i in 0..12 {
                bels.push(
                    builder
                        .bel_xy(format!("HPIODIFFIN{i}"), "HPIOBDIFFINBUF", 0, i)
                        .pins_name_only(&[
                            "LVDS_TRUE",
                            "LVDS_COMP",
                            "PAD_RES_0",
                            "PAD_RES_1",
                            "VREF",
                        ]),
                );
            }
            for i in 0..12 {
                bels.push(
                    builder
                        .bel_xy(format!("HPIODIFFOUT{i}"), "HPIOBDIFFOUTBUF", 0, i)
                        .pins_name_only(&["AOUT", "BOUT", "O_B", "TSTATEB"]),
                );
            }
            for i in 0..2 {
                bels.push(builder.bel_xy(format!("HPIO_DCI{i}"), "HPIOB_DCI_SNGL", 0, i));
            }
            bels.push(
                builder
                    .bel_xy("HPIO_VREF", "HPIO_VREF_SITE", 0, 0)
                    .pins_name_only(&["VREF1", "VREF2"]),
            );
            bels.push(builder.bel_xy("HPIO_BIAS", "BIAS", 0, 0));

            let mut xn = builder.xnode(kind, kind, xy).num_tiles(30);
            for i in 0..30 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(
                        int_xy.delta(if is_l { -1 } else { 1 }, (i + i / 30) as i32),
                        i,
                        intf,
                    )
            }
            xn.bels(bels).extract();
        }
    }

    for tkn in ["RCLK_HPIO_L", "RCLK_HPIO_R"] {
        let is_l = tkn != "RCLK_HPIO_R";
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder
                .walk_to_int(xy.delta(0, -30), if is_l { Dir::E } else { Dir::W })
                .unwrap();
            let intf = builder
                .db
                .get_node_naming(if is_l { "INTF.W.IO" } else { "INTF.E.IO" });
            let mut bels = vec![];
            for i in 0..7 {
                bels.push(builder.bel_xy(format!("ABUS_SWITCH.HPIO{i}"), "ABUS_SWITCH", i, 0));
            }
            bels.push(builder.bel_xy("HPIO_ZMATCH_BLK_HCLK", "HPIO_ZMATCH_BLK_HCLK", 0, 0));
            bels.push(builder.bel_xy("HPIO_RCLK_PRBS", "HPIO_RCLK_PRBS", 0, 0));

            let mut xn = builder.xnode(tkn, tkn, xy).num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(
                        int_xy.delta(if is_l { -1 } else { 1 }, (i + i / 30) as i32),
                        i,
                        intf,
                    )
            }
            xn.bels(bels).extract();
        }
    }

    for (tkn, kind, is_l) in [
        ("GTH_QUAD_LEFT", "GTH_L", true),
        ("GTH_QUAD_RIGHT", "GTH_R", false),
        ("GTY_L", "GTY_L", true),
        ("GTY_R", "GTY_R", false),
        ("GTFY_QUAD_LEFT_FT", "GTF_L", true),
        ("GTFY_QUAD_RIGHT_FT", "GTF_R", false),
        ("GTM_DUAL_LEFT_FT", "GTM_L", true),
        ("GTM_DUAL_RIGHT_FT", "GTM_R", false),
        ("HSADC_HSADC_RIGHT_FT", "HSADC_R", false),
        ("HSDAC_HSDAC_RIGHT_FT", "HSDAC_R", false),
        ("RFADC_RFADC_RIGHT_FT", "RFADC_R", false),
        ("RFDAC_RFDAC_RIGHT_FT", "RFDAC_R", false),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let int_xy = builder
                .walk_to_int(xy, if is_l { Dir::E } else { Dir::W })
                .unwrap();
            let intf = builder
                .db
                .get_node_naming(if is_l { "INTF.W.GT" } else { "INTF.E.GT" });
            let rclk_int = builder.db.get_node_naming("RCLK_INT");
            let mut bels = vec![];
            for i in 0..24 {
                let mut bel = builder
                    .bel_xy(format!("BUFG_GT{i}"), "BUFG_GT", 0, i as u8)
                    .pins_name_only(&["CLK_IN", "CLK_OUT", "CE", "RST_PRE_OPTINV"]);
                if !kind.starts_with("GT") {
                    bel = bel
                        .pin_name_only("DIV0", 1)
                        .pin_name_only("DIV1", 1)
                        .pin_name_only("DIV2", 1);
                }
                if kind.starts_with("GT") {
                    let bi = [
                        (0, 1, 12),
                        (27, 28, 29),
                        (43, 44, 46),
                        (47, 48, 49),
                        (50, 51, 52),
                        (53, 54, 55),
                        (57, 58, 59),
                        (60, 61, 62),
                        (63, 64, 65),
                        (66, 68, 69),
                        (23, 34, 45),
                        (56, 67, 70),
                        (71, 2, 3),
                        (4, 5, 6),
                        (7, 8, 9),
                        (10, 11, 13),
                        (14, 15, 16),
                        (17, 18, 19),
                        (20, 21, 22),
                        (24, 25, 26),
                        (30, 31, 32),
                        (33, 35, 36),
                        (37, 38, 39),
                        (40, 41, 42),
                    ][i];
                    bel = bel
                        .extra_wire("CE_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.0)])
                        .extra_wire("CLK_IN_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.1)])
                        .extra_wire("RST_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.2)]);
                } else {
                    let bi = [
                        (20, 21, 82),
                        (137, 138, 149),
                        (213, 214, 226),
                        (227, 228, 239),
                        (240, 241, 252),
                        (253, 254, 265),
                        (267, 268, 279),
                        (280, 281, 292),
                        (293, 294, 305),
                        (306, 318, 329),
                        (123, 164, 225),
                        (266, 307, 330),
                        (331, 32, 43),
                        (44, 45, 56),
                        (57, 58, 69),
                        (70, 71, 83),
                        (84, 85, 96),
                        (97, 98, 109),
                        (110, 111, 122),
                        (124, 125, 136),
                        (150, 151, 162),
                        (163, 175, 186),
                        (187, 188, 199),
                        (200, 201, 212),
                    ][i];
                    bel = bel
                        .extra_wire("CE_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.0)])
                        .extra_wire("RST_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.2)]);
                    for j in 0..11 {
                        bel = bel.extra_wire(
                            format!("CLK_IN_MUX_DUMMY{j}"),
                            &[format!("VCC_WIRE{ii}", ii = bi.1 + j)],
                        );
                    }
                }
                bels.push(bel);
            }
            for i in 0..15 {
                let mut bel = builder
                    .bel_xy(format!("BUFG_GT_SYNC{i}"), "BUFG_GT_SYNC", 0, i)
                    .pins_name_only(&["CE_OUT", "RST_OUT"]);
                if !kind.starts_with("GT") && (4..14).contains(&i) {
                    bel = bel.pins_name_only(&["CE_IN", "RST_IN"]);
                }
                if i != 14 {
                    bel = bel.pins_name_only(&["CLK_IN"]);
                }
                if kind.starts_with("GTM") && matches!(i, 6 | 13) {
                    bel = bel.extra_wire(
                        "CLK_IN",
                        &[format!(
                            "CLK_BUFG_GT_SYNC_BOTH_{ii}_CLK_IN",
                            ii = if is_l { 24 + i } else { 26 + i }
                        )],
                    );
                }
                bels.push(bel);
            }
            for i in 0..5 {
                bels.push(builder.bel_xy(format!("ABUS_SWITCH.GT{i}"), "ABUS_SWITCH", 0, i));
            }

            if kind.starts_with("GTM") {
                bels.push(
                    builder
                        .bel_xy("GTM_DUAL", "GTM_DUAL", 0, 0)
                        .pins_name_only(&[
                            "CLK_BUFGT_CLK_IN_BOT0",
                            "CLK_BUFGT_CLK_IN_BOT1",
                            "CLK_BUFGT_CLK_IN_BOT2",
                            "CLK_BUFGT_CLK_IN_BOT3",
                            "CLK_BUFGT_CLK_IN_BOT4",
                            "CLK_BUFGT_CLK_IN_BOT5",
                            "CLK_BUFGT_CLK_IN_TOP0",
                            "CLK_BUFGT_CLK_IN_TOP1",
                            "CLK_BUFGT_CLK_IN_TOP2",
                            "CLK_BUFGT_CLK_IN_TOP3",
                            "CLK_BUFGT_CLK_IN_TOP4",
                            "CLK_BUFGT_CLK_IN_TOP5",
                            "HROW_TEST_CK_SA",
                            "MGTREFCLK_CLEAN",
                            "RXRECCLK0_INT",
                            "RXRECCLK1_INT",
                            "REFCLKPDB_SA",
                            "RCALSEL0",
                            "RCALSEL1",
                            "REFCLK_DIST2PLL0",
                            "REFCLK_DIST2PLL1",
                            "REFCLK2HROW",
                        ])
                        .extra_wire("SOUTHCLKOUT", &["SOUTHCLKOUT"])
                        .extra_wire("SOUTHCLKOUT_DUMMY0", &["VCC_WIRE72"])
                        .extra_wire("SOUTHCLKOUT_DUMMY1", &["VCC_WIRE73"])
                        .extra_wire("NORTHCLKOUT", &["NORTHCLKOUT"])
                        .extra_wire("NORTHCLKOUT_DUMMY0", &["VCC_WIRE74"])
                        .extra_wire("NORTHCLKOUT_DUMMY1", &["VCC_WIRE75"]),
                );
                bels.push(
                    builder
                        .bel_xy("GTM_REFCLK", "GTM_REFCLK", 0, 0)
                        .pins_name_only(&[
                            "HROW_TEST_CK_FS",
                            "MGTREFCLK_CLEAN",
                            "REFCLK2HROW",
                            "REFCLKPDB_SA",
                            "RXRECCLK0_INT",
                            "RXRECCLK1_INT",
                            "RXRECCLK2_INT",
                            "RXRECCLK3_INT",
                        ]),
                );
            } else if kind.starts_with("GT") {
                let gtk = &kind[..3];
                let pref = if gtk == "GTF" {
                    "GTF".to_string()
                } else {
                    format!("{gtk}E4")
                };
                for i in 0..4 {
                    bels.push(
                        builder
                            .bel_xy(
                                format!("{gtk}_CHANNEL{i}"),
                                &format!("{pref}_CHANNEL"),
                                0,
                                i,
                            )
                            .pins_name_only(&[
                                "MGTREFCLK0",
                                "MGTREFCLK1",
                                "NORTHREFCLK0",
                                "NORTHREFCLK1",
                                "SOUTHREFCLK0",
                                "SOUTHREFCLK1",
                                "QDCMREFCLK0_INT",
                                "QDCMREFCLK1_INT",
                                "QDPLL0CLK0P_INT",
                                "QDPLL1CLK0P_INT",
                                "RING_OSC_CLK_INT",
                                "RXRECCLKOUT",
                                "RXRECCLK_INT",
                                "TXOUTCLK_INT",
                                "DMONOUTCLK_INT",
                            ]),
                    );
                }
                bels.push(
                    builder
                        .bel_xy(format!("{gtk}_COMMON"), &format!("{pref}_COMMON"), 0, 0)
                        .pins_name_only(&[
                            "RXRECCLK0",
                            "RXRECCLK1",
                            "RXRECCLK2",
                            "RXRECCLK3",
                            "QDCMREFCLK_INT_0",
                            "QDCMREFCLK_INT_1",
                            "QDPLLCLK0P_0",
                            "QDPLLCLK0P_1",
                            "COM0_REFCLKOUT0",
                            "COM0_REFCLKOUT1",
                            "COM0_REFCLKOUT2",
                            "COM0_REFCLKOUT3",
                            "COM0_REFCLKOUT4",
                            "COM0_REFCLKOUT5",
                            "COM2_REFCLKOUT0",
                            "COM2_REFCLKOUT1",
                            "COM2_REFCLKOUT2",
                            "COM2_REFCLKOUT3",
                            "COM2_REFCLKOUT4",
                            "COM2_REFCLKOUT5",
                            "MGTREFCLK0",
                            "MGTREFCLK1",
                            "REFCLK2HROW0",
                            "REFCLK2HROW1",
                            "SARC_CLK0",
                            "SARC_CLK1",
                            "SARC_CLK2",
                            "SARC_CLK3",
                        ])
                        .extra_wire("CLKOUT_NORTH0", &["CLKOUT_NORTH0"])
                        .extra_wire("CLKOUT_NORTH1", &["CLKOUT_NORTH1"])
                        .extra_wire("CLKOUT_SOUTH0", &["CLKOUT_SOUTH0"])
                        .extra_wire("CLKOUT_SOUTH1", &["CLKOUT_SOUTH1"])
                        .extra_wire(
                            "NORTHREFCLK0",
                            &[
                                "GTH_CHANNEL_BLH_0_NORTHREFCLK0",
                                "GTH_CHANNEL_BLH_44_NORTHREFCLK0",
                                "GTF_CHANNEL_BLH_1_NORTHREFCLK0",
                                "GTF_CHANNEL_BLH_45_NORTHREFCLK0",
                                "GTY_CHANNEL_BLH_1_NORTHREFCLK0",
                                "GTY_CHANNEL_BLH_45_NORTHREFCLK0",
                            ],
                        )
                        .extra_wire(
                            "NORTHREFCLK1",
                            &[
                                "GTH_CHANNEL_BLH_0_NORTHREFCLK1",
                                "GTH_CHANNEL_BLH_44_NORTHREFCLK1",
                                "GTF_CHANNEL_BLH_1_NORTHREFCLK1",
                                "GTF_CHANNEL_BLH_45_NORTHREFCLK1",
                                "GTY_CHANNEL_BLH_1_NORTHREFCLK1",
                                "GTY_CHANNEL_BLH_45_NORTHREFCLK1",
                            ],
                        )
                        .extra_wire(
                            "SOUTHREFCLK0",
                            &[
                                "GTH_CHANNEL_BLH_0_SOUTHREFCLK0",
                                "GTH_CHANNEL_BLH_44_SOUTHREFCLK0",
                                "GTF_CHANNEL_BLH_1_SOUTHREFCLK0",
                                "GTF_CHANNEL_BLH_45_SOUTHREFCLK0",
                                "GTY_CHANNEL_BLH_1_SOUTHREFCLK0",
                                "GTY_CHANNEL_BLH_45_SOUTHREFCLK0",
                            ],
                        )
                        .extra_wire(
                            "SOUTHREFCLK1",
                            &[
                                "GTH_CHANNEL_BLH_0_SOUTHREFCLK1",
                                "GTH_CHANNEL_BLH_44_SOUTHREFCLK1",
                                "GTF_CHANNEL_BLH_1_SOUTHREFCLK1",
                                "GTF_CHANNEL_BLH_45_SOUTHREFCLK1",
                                "GTY_CHANNEL_BLH_1_SOUTHREFCLK1",
                                "GTY_CHANNEL_BLH_45_SOUTHREFCLK1",
                            ],
                        ),
                );
            } else {
                let bk = &kind[..5];
                let mut bel = builder
                    .bel_xy(bk, bk, 0, 0)
                    .pins_name_only(&[
                        "SYSREF_OUT_SOUTH_P",
                        "SYSREF_OUT_NORTH_P",
                        "PLL_DMON_OUT",
                        "PLL_REFCLK_OUT",
                    ])
                    .pin_name_only("SYSREF_IN_SOUTH_P", 1)
                    .pin_name_only("SYSREF_IN_NORTH_P", 1);
                if bk.ends_with("ADC") {
                    bel = bel.pins_name_only(&["CLK_ADC", "CLK_ADC_SPARE"]);
                } else {
                    bel = bel.pins_name_only(&["CLK_DAC", "CLK_DAC_SPARE"]);
                }
                if bk.starts_with("RF") {
                    bel = bel
                        .pins_name_only(&[
                            "CLK_DISTR_IN_NORTH",
                            "CLK_DISTR_IN_SOUTH",
                            "CLK_DISTR_OUT_NORTH",
                            "CLK_DISTR_OUT_SOUTH",
                            "T1_ALLOWED_SOUTH",
                            "T1_ALLOWED_NORTH",
                        ])
                        .pin_name_only("CLK_DISTR_IN_NORTH", 1)
                        .pin_name_only("CLK_DISTR_IN_SOUTH", 1)
                        .pin_name_only("T1_ALLOWED_NORTH", 1);
                }
                bels.push(bel);
            }

            let mut bel = builder.bel_virtual(if is_l { "RCLK_GT_L" } else { "RCLK_GT_R" });
            for i in 0..24 {
                if is_l {
                    bel = bel
                        .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_FT1_{i}")])
                        .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_FT1_{i}")]);
                } else {
                    bel = bel
                        .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_FT0_{i}")])
                        .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_FT0_{i}")])
                }
            }
            bels.push(bel);
            bels.push(
                builder
                    .bel_virtual("VCC.GT")
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );

            let mut xn = builder.xnode(kind, kind, xy).num_tiles(60).ref_single(
                int_xy.delta(0, 30),
                30,
                rclk_int,
            );
            for i in 0..60 {
                xn = xn
                    .ref_int(int_xy.delta(0, (i + i / 30) as i32), i)
                    .ref_single(
                        int_xy.delta(if is_l { -1 } else { 1 }, (i + i / 30) as i32),
                        i,
                        intf,
                    )
            }
            xn.bels(bels).extract();
        }
    }

    builder.build()
}
