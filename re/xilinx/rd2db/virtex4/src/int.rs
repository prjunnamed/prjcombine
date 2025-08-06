use prjcombine_interconnect::{
    db::{IntDb, WireKind},
    dir::Dir,
};
use prjcombine_re_xilinx_rawdump::Part;

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;
use prjcombine_virtex4::{bels, cslots, regions, tslots};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        IntDb::new(tslots::SLOTS, bels::SLOTS, regions::SLOTS, cslots::SLOTS),
    );

    builder.wire("PULLUP", WireKind::TiePullup, &["KEEP1_WIRE"]);
    builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
    builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

    for i in 0..8 {
        builder.wire(
            format!("HCLK{i}"),
            WireKind::Regional(regions::LEAF),
            &[format!("GCLK{i}")],
        );
    }
    for i in 0..2 {
        builder.wire(
            format!("RCLK{i}"),
            WireKind::Regional(regions::LEAF),
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
        builder.mux_out(format!("IMUX.BYP{i}"), &[format!("BYP_INT_B{i}")]);
        builder.permabuf(format!("IMUX.BYP{i}.BOUNCE"), &[format!("BYP_BOUNCE{i}")]);
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
        match i {
            0 => {
                builder.extra_name_sub("MONITOR_CONVST_TEST", 4, w);

                builder.extra_name_sub("DCM_ADV_CLKFB_TEST", 2, w);
                builder.extra_name_sub("DCM_ADV_CLKIN_TEST", 3, w);

                builder.extra_name_sub("DPM_REFCLK_TEST", 0, w);
                builder.extra_name_sub("PMCD_0_CLKB_TEST", 1, w);
                builder.extra_name_sub("DPM_TESTCLK1_TEST", 2, w);
                builder.extra_name_sub("PMCD_0_CLKD_TEST", 3, w);
            }
            1 => {
                builder.extra_name_sub("PMCD_0_CLKA_TEST", 1, w);
                builder.extra_name_sub("DPM_TESTCLK2_TEST", 2, w);
                builder.extra_name_sub("PMCD_0_CLKC_TEST", 3, w);
            }
            2 => {
                builder.extra_name_sub("PMCD_1_REL_TEST", 0, w);
                builder.extra_name_sub("PMCD_1_CLKB_TEST", 1, w);
                builder.extra_name_sub("PMCD_1_CLKD_TEST", 3, w);
            }
            3 => {
                builder.extra_name_sub("PMCD_0_REL_TEST", 0, w);
                builder.extra_name_sub("PMCD_1_CLKA_TEST", 1, w);
                builder.extra_name_sub("PMCD_1_CLKC_TEST", 3, w);
            }
            _ => unreachable!(),
        }
        for j in 0..16 {
            builder.extra_name_sub(format!("LOGIC_CREATED_INPUT_B{i}_INT{j}"), j, w);
        }
    }

    builder.extract_main_passes();

    builder.int_type(tslots::INT, bels::INT, "INT", "INT", "INT");
    builder.int_type(tslots::INT, bels::INT, "INT_SO", "INT", "INT");
    builder.int_type(tslots::INT, bels::INT, "INT_SO_DCM0", "INT", "INT.DCM0");

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
                let int_xy = xy.delta(1, -9 + delta);
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
                let int_xy = xy.delta(-1, -9 + delta);
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
        let pt_xy = pb_xy.delta(0, 18);
        for (i, delta) in [
            0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = pb_xy.delta(-1, -3 + delta);
            let int_e_xy = pb_xy.delta(15, -3 + delta);
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
                None,
                int_e_xy,
                &[],
            );
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = pb_xy.delta(delta, -4);
            let int_n_xy = pb_xy.delta(delta, 22);
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
                let int_xy = xy.delta(-1, i);
                builder.extract_intf_tile(
                    tslots::INTF,
                    "INTF",
                    xy,
                    int_xy,
                    format!("INTF.{n}.{i}"),
                    false,
                    None,
                );
            }
        }
    }
    for tkn in ["IOIS_LC", "IOIS_NC"] {
        builder.extract_intf(tslots::INTF, "INTF", Dir::E, tkn, "INTF.IOIS", false, None);
    }
    for &xy in rd.tiles_by_kind_name("CFG_CENTER") {
        for i in 0..16 {
            let int_xy = xy.delta(-1, if i < 8 { -8 + i } else { -8 + i + 1 });
            builder.extract_intf_tile(
                tslots::INTF,
                "INTF",
                xy,
                int_xy,
                format!("INTF.CFG.{i}"),
                false,
                None,
            );
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
                let int_xy = xy.delta(
                    if dir == Dir::E { -1 } else { 1 },
                    if i < 8 { -9 + i } else { i - 8 },
                );
                builder.extract_intf_tile(
                    tslots::INTF,
                    "INTF",
                    xy,
                    int_xy,
                    format!("INTF.MGT.{i}"),
                    false,
                    None,
                );
            }
        }
    }

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = pb_xy.delta(0, 18);
        for (i, delta) in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ]
        .into_iter()
        .enumerate()
        {
            let int_w_xy = pb_xy.delta(-1, -4 + delta);
            let int_e_xy = pb_xy.delta(15, -4 + delta);
            let xy = if i < 12 { pb_xy } else { pt_xy };
            builder.extract_intf_tile(
                tslots::INTF,
                "INTF",
                xy,
                int_w_xy,
                format!("INTF.PPC.L{i}"),
                false,
                None,
            );
            builder.extract_intf_tile(
                tslots::INTF,
                "INTF",
                xy,
                int_e_xy,
                format!("INTF.PPC.R{i}"),
                false,
                None,
            );
        }
        for (i, delta) in [1, 3, 5, 7, 9, 11, 13].into_iter().enumerate() {
            let int_s_xy = pb_xy.delta(delta, -4);
            let int_n_xy = pb_xy.delta(delta, 22);
            builder.extract_intf_tile(
                tslots::INTF,
                "INTF",
                pb_xy,
                int_s_xy,
                format!("INTF.PPC.B{i}"),
                false,
                None,
            );
            builder.extract_intf_tile(
                tslots::INTF,
                "INTF",
                pt_xy,
                int_n_xy,
                format!("INTF.PPC.T{i}"),
                false,
                None,
            );
        }
    }

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    if let Some(&xy) = rd.tiles_by_kind_name("CLB").iter().next() {
        let int_xy = xy.delta(-1, 0);
        builder.extract_xtile_bels(
            tslots::BEL,
            "CLB",
            xy,
            &[],
            &[int_xy],
            "CLB",
            &[
                builder
                    .bel_xy(bels::SLICE0, "SLICE", 0, 0)
                    .pins_name_only(&slicem_name_only),
                builder
                    .bel_xy(bels::SLICE1, "SLICE", 1, 0)
                    .pins_name_only(&slicel_name_only),
                builder
                    .bel_xy(bels::SLICE2, "SLICE", 0, 1)
                    .pins_name_only(&slicem_name_only)
                    .extra_wire("COUT_N", &["COUT_N1"])
                    .extra_wire("FX_S", &["FX_S2"]),
                builder
                    .bel_xy(bels::SLICE3, "SLICE", 1, 1)
                    .pins_name_only(&slicel_name_only)
                    .extra_wire("COUT_N", &["COUT_N3"]),
            ],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("BRAM").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels(
            tslots::BEL,
            "BRAM",
            xy,
            &[],
            &int_xy,
            "BRAM",
            &[
                builder
                    .bel_xy(bels::BRAM, "RAMB16", 0, 0)
                    .pins_name_only(&["CASCADEOUTA", "CASCADEOUTB"])
                    .pin_name_only("CASCADEINA", 1)
                    .pin_name_only("CASCADEINB", 1),
                builder.bel_xy(bels::FIFO, "FIFO16", 0, 0),
            ],
        );
    }

    let mut bels_dsp = vec![];
    for i in 0..2 {
        let mut bel = builder.bel_xy(bels::DSP[i], "DSP48", 0, i);
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
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels(tslots::BEL, "DSP", xy, &[], &int_xy, "DSP", &bels_dsp);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CFG_CENTER").iter().next() {
        let mut bels = vec![];
        for i in 0..16 {
            bels.push(
                builder
                    .bel_xy(bels::BUFGCTRL[i], "BUFGCTRL", 0, i)
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
                    .bel_xy(bels::BUFGCTRL[i + 16], "BUFGCTRL", 0, i)
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
            builder.bel_xy(bels::BSCAN0, "BSCAN", 0, 0),
            builder.bel_xy(bels::BSCAN1, "BSCAN", 0, 1),
            builder.bel_xy(bels::BSCAN2, "BSCAN", 0, 2),
            builder.bel_xy(bels::BSCAN3, "BSCAN", 0, 3),
            builder.bel_xy(bels::ICAP0, "ICAP", 0, 0),
            builder.bel_xy(bels::ICAP1, "ICAP", 0, 1),
            builder.bel_single(bels::PMV0, "PMV"),
            builder.bel_single(bels::STARTUP, "STARTUP"),
            builder
                .bel_single(bels::JTAGPPC, "JTAGPPC")
                .pin_name_only("TDOTSPPC", 0),
            builder.bel_single(bels::FRAME_ECC, "FRAME_ECC"),
            builder.bel_single(bels::DCIRESET, "DCIRESET"),
            builder.bel_single(bels::CAPTURE, "CAPTURE"),
            builder.bel_single(bels::USR_ACCESS, "USR_ACCESS_SITE"),
            builder
                .bel_virtual(bels::BUFG_MGTCLK_S)
                .raw_tile(1)
                .extra_wire("MGT_L0", &["CLK_BUFGCTRL_MGT_L0"])
                .extra_wire("MGT_L1", &["CLK_BUFGCTRL_MGT_L1"])
                .extra_wire("MGT_R0", &["CLK_BUFGCTRL_MGT_R0"])
                .extra_wire("MGT_R1", &["CLK_BUFGCTRL_MGT_R1"]),
            builder
                .bel_virtual(bels::BUFG_MGTCLK_N)
                .raw_tile(2)
                .extra_wire("MGT_L0", &["CLK_BUFGCTRL_MGT_L0"])
                .extra_wire("MGT_L1", &["CLK_BUFGCTRL_MGT_L1"])
                .extra_wire("MGT_R0", &["CLK_BUFGCTRL_MGT_R0"])
                .extra_wire("MGT_R1", &["CLK_BUFGCTRL_MGT_R1"]),
            builder
                .bel_virtual(bels::BUFG_MGTCLK_S_HROW)
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
                .bel_virtual(bels::BUFG_MGTCLK_N_HROW)
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
                .bel_virtual(bels::BUFG_MGTCLK_S_HCLK)
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
                .bel_virtual(bels::BUFG_MGTCLK_N_HCLK)
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
        let mut xn = builder
            .xtile(tslots::BEL, "CFG", "CFG", xy)
            .raw_tile(xy.delta(1, -8))
            .raw_tile(xy.delta(1, 1))
            .raw_tile(xy.delta(1, -9))
            .raw_tile(xy.delta(1, 9))
            .raw_tile(xy.delta(0, -9))
            .raw_tile(xy.delta(0, 9))
            .num_tiles(16);
        for i in 0..8 {
            xn = xn.ref_int(xy.delta(-1, -8 + (i as i32)), i);
        }
        for i in 0..8 {
            xn = xn.ref_int(xy.delta(-1, 1 + (i as i32)), i + 8);
        }
        for bel in bels {
            xn = xn.bel(bel);
        }
        xn.extract();
    }

    for &pb_xy in rd.tiles_by_kind_name("PB") {
        let pt_xy = pb_xy.delta(0, 18);
        let mut int_xy = vec![];
        for dy in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ] {
            int_xy.push(pb_xy.delta(-1, -4 + dy));
        }
        for dy in [
            0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26,
        ] {
            int_xy.push(pb_xy.delta(15, -4 + dy));
        }
        for dx in [1, 3, 5, 7, 9, 11, 13] {
            int_xy.push(pb_xy.delta(dx, -4));
        }
        for dx in [1, 3, 5, 7, 9, 11, 13] {
            int_xy.push(pb_xy.delta(dx, 22));
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
        builder.extract_xtile_bels(
            tslots::BEL,
            "PPC",
            pb_xy,
            &[pt_xy],
            &int_xy,
            "PPC",
            &[
                builder
                    .bel_xy(bels::PPC, "PPC405_ADV", 0, 0)
                    .pins_name_only(&dcr_pins),
                builder
                    .bel_xy(bels::EMAC, "EMAC", 0, 0)
                    .pins_name_only(&dcr_pins),
            ],
        );
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CLK_HROW").iter().next() {
        let mut bel = builder.bel_virtual(bels::CLK_HROW);
        for i in 0..32 {
            bel = bel.extra_wire(format!("GCLK{i}"), &[format!("CLK_HROW_GCLK_BUFP{i}")]);
        }
        for i in 0..8 {
            bel = bel.extra_wire(format!("HCLK_L{i}"), &[format!("CLK_HROW_HCLK_LP{i}")]);
            bel = bel.extra_wire(format!("HCLK_R{i}"), &[format!("CLK_HROW_HCLK_RP{i}")]);
        }
        builder
            .xtile(tslots::HROW, "CLK_HROW", "CLK_HROW", xy)
            .num_tiles(0)
            .bel(bel)
            .extract();
    }

    for tkn in ["CLK_IOB_B", "CLK_IOB_T"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(bels::CLK_IOB);
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
            builder
                .xtile(tslots::CLK, tkn, tkn, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }

    for (nn, tkn) in [("CLK_DCM_B", "CLKV_DCM_B"), ("CLK_DCM_T", "CLKV_DCM_T")] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bel = builder.bel_virtual(bels::CLK_DCM);
            for i in 0..12 {
                bel = bel
                    .extra_wire(format!("DCM0_{i}"), &[format!("CLKV_DCM_DCM0_CLKP{i}")])
                    .extra_wire(format!("DCM1_{i}"), &[format!("CLKV_DCM_DCM1_CLKP{i}")]);
            }
            for i in 0..24 {
                bel = bel.extra_wire(format!("DCM{i}"), &[format!("CLKV_DCM_DCM_OUTCLKP{i}")]);
            }
            for i in 0..32 {
                bel = bel
                    .extra_wire_force(format!("MUXBUS_I{i}"), format!("CLK_IOB_MUXED_CLKP_IN{i}"));
                bel = bel.extra_wire(
                    format!("MUXBUS_O{i}"),
                    &[format!("CLKV_DCM_MUXED_CLKP_OUT{i}")],
                );
            }
            builder
                .xtile(tslots::CLK, nn, nn, xy)
                .num_tiles(0)
                .bel(bel)
                .extract();
        }
    }
    builder.make_marker_tile(tslots::HROW, "CLK_TERM", 0);
    builder.make_marker_tile(tslots::HROW, "HCLK_TERM", 0);
    builder.make_marker_tile(tslots::HCLK_BEL, "HCLK_MGT", 0);
    builder.make_marker_tile(tslots::CLK, "HCLK_MGT_REPEATER", 0);

    if let Some(&xy) = rd.tiles_by_kind_name("HCLK").iter().next() {
        let bel_gsig = builder.bel_xy(bels::GLOBALSIG, "GLOBALSIG", 0, 0);
        let mut bel = builder.bel_virtual(bels::HCLK);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_G_HCLKP{i}")])
                .extra_int_out(format!("HCLK_O{i}"), &[format!("HCLK_LEAF_GCLK{i}")]);
        }
        for i in 0..2 {
            bel = bel
                .extra_wire(format!("RCLK_I{i}"), &[format!("HCLK_RCLK{i}")])
                .extra_int_out(format!("RCLK_O{i}"), &[format!("HCLK_LEAF_RCLK{i}")]);
        }
        builder
            .xtile(tslots::HCLK, "HCLK", "HCLK", xy)
            .ref_int(xy.delta(0, 1), 0)
            .bel(bel_gsig)
            .bel(bel)
            .extract();
    }

    let bel_ioclk = builder
        .bel_virtual(bels::IOCLK)
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
        .extra_wire("HCLK_I0", &["HCLK_IOIS_G_HCLKP0", "HCLK_DCM_G_HCLKP0"])
        .extra_wire("HCLK_I1", &["HCLK_IOIS_G_HCLKP1", "HCLK_DCM_G_HCLKP1"])
        .extra_wire("HCLK_I2", &["HCLK_IOIS_G_HCLKP2", "HCLK_DCM_G_HCLKP2"])
        .extra_wire("HCLK_I3", &["HCLK_IOIS_G_HCLKP3", "HCLK_DCM_G_HCLKP3"])
        .extra_wire("HCLK_I4", &["HCLK_IOIS_G_HCLKP4", "HCLK_DCM_G_HCLKP4"])
        .extra_wire("HCLK_I5", &["HCLK_IOIS_G_HCLKP5", "HCLK_DCM_G_HCLKP5"])
        .extra_wire("HCLK_I6", &["HCLK_IOIS_G_HCLKP6", "HCLK_DCM_G_HCLKP6"])
        .extra_wire("HCLK_I7", &["HCLK_IOIS_G_HCLKP7", "HCLK_DCM_G_HCLKP7"])
        .extra_wire(
            "HCLK_O0",
            &["HCLK_IOIS_LEAF_GCLK_P0", "HCLK_DCM_LEAF_GCLK_P0"],
        )
        .extra_wire(
            "HCLK_O1",
            &["HCLK_IOIS_LEAF_GCLK_P1", "HCLK_DCM_LEAF_GCLK_P1"],
        )
        .extra_wire(
            "HCLK_O2",
            &["HCLK_IOIS_LEAF_GCLK_P2", "HCLK_DCM_LEAF_GCLK_P2"],
        )
        .extra_wire(
            "HCLK_O3",
            &["HCLK_IOIS_LEAF_GCLK_P3", "HCLK_DCM_LEAF_GCLK_P3"],
        )
        .extra_wire(
            "HCLK_O4",
            &["HCLK_IOIS_LEAF_GCLK_P4", "HCLK_DCM_LEAF_GCLK_P4"],
        )
        .extra_wire(
            "HCLK_O5",
            &["HCLK_IOIS_LEAF_GCLK_P5", "HCLK_DCM_LEAF_GCLK_P5"],
        )
        .extra_wire(
            "HCLK_O6",
            &["HCLK_IOIS_LEAF_GCLK_P6", "HCLK_DCM_LEAF_GCLK_P6"],
        )
        .extra_wire(
            "HCLK_O7",
            &["HCLK_IOIS_LEAF_GCLK_P7", "HCLK_DCM_LEAF_GCLK_P7"],
        )
        .extra_wire("RCLK_I0", &["HCLK_IOIS_RCLK0", "HCLK_DCM_RCLK0"])
        .extra_wire("RCLK_I1", &["HCLK_IOIS_RCLK1", "HCLK_DCM_RCLK1"])
        .extra_wire(
            "RCLK_O0",
            &["HCLK_IOIS_RCLK_FORIO_P0", "HCLK_DCM_RCLK_FORIO_P0"],
        )
        .extra_wire(
            "RCLK_O1",
            &["HCLK_IOIS_RCLK_FORIO_P1", "HCLK_DCM_RCLK_FORIO_P1"],
        );
    for tkn in ["HCLK_IOIS_DCI", "HCLK_IOIS_LVDS"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![
                builder
                    .bel_xy(bels::BUFR0, "BUFR", 0, 1)
                    .pins_name_only(&["O", "I"]),
                builder
                    .bel_xy(bels::BUFR1, "BUFR", 0, 0)
                    .pins_name_only(&["O", "I"]),
                builder
                    .bel_xy(bels::BUFIO0, "BUFIO", 0, 1)
                    .pins_name_only(&["O", "I"])
                    .extra_wire("PAD", &["HCLK_IOIS_I2IOCLK_TOP_P"]),
                builder
                    .bel_xy(bels::BUFIO1, "BUFIO", 0, 0)
                    .pins_name_only(&["O", "I"])
                    .extra_wire("PAD", &["HCLK_IOIS_I2IOCLK_BOT_P"]),
                builder
                    .bel_xy(bels::IDELAYCTRL, "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
            ];
            if tkn == "HCLK_IOIS_DCI" {
                bels.push(builder.bel_xy(bels::DCI, "DCI", 0, 0));
            }
            bels.extend([
                builder
                    .bel_virtual(bels::RCLK)
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
                .xtile(tslots::HCLK_BEL, tkn, tkn, xy)
                .num_tiles(3)
                .raw_tile(xy.delta(0, -2))
                .raw_tile(xy.delta(0, -1))
                .raw_tile(xy.delta(0, 1))
                .ref_int(xy.delta(-1, -2), 0)
                .ref_int(xy.delta(-1, -1), 1)
                .ref_int(xy.delta(-1, 1), 2);
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }
    let mut bel_hclk_dcm_hrow = builder.bel_virtual(bels::HCLK_DCM_HROW);
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
                    .bel_xy(bels::BUFIO0, "BUFIO", 0, 1)
                    .pins_name_only(&["O", "I"])
                    .extra_wire_force("PAD", "HCLK_IOIS_I2IOCLK_TOP_P"),
                builder
                    .bel_xy(bels::BUFIO1, "BUFIO", 0, 0)
                    .pins_name_only(&["O", "I"])
                    .extra_wire_force("PAD", "HCLK_IOIS_I2IOCLK_BOT_P"),
                builder
                    .bel_xy(bels::IDELAYCTRL, "IDELAYCTRL", 0, 0)
                    .pins_name_only(&["REFCLK"]),
                builder.bel_xy(bels::DCI, "DCI", 0, 0),
                bel_ioclk,
            ];
            match dcmloc {
                'S' => {
                    let mut bel = builder.bel_virtual(bels::HCLK_DCM_S);
                    for i in 0..8 {
                        bel = bel
                            .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_DCM_G_HCLKP{i}")])
                            .extra_wire(
                                format!("HCLK_O_D{i}"),
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
                    let mut bel = builder.bel_virtual(bels::HCLK_DCM_N);
                    for i in 0..8 {
                        bel = bel
                            .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_DCM_G_HCLKP{i}")])
                            .extra_wire(
                                format!("HCLK_O_U{i}"),
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
            let mut xn = builder.xtile(tslots::HCLK_BEL, tkn, tkn, xy).num_tiles(2);
            if ioloc == 'S' {
                xn = xn
                    .raw_tile(xy.delta(0, -2))
                    .raw_tile(xy.delta(0, -1))
                    .ref_int(xy.delta(-1, -2), 0)
                    .ref_int(xy.delta(-1, -1), 1);
            } else {
                xn = xn
                    .raw_tile(xy.delta(0, 1))
                    .raw_tile(xy.delta(0, 2))
                    .ref_int(xy.delta(-1, 1), 0)
                    .ref_int(xy.delta(-1, 2), 1);
            }
            if dcmloc != '_' {
                xn = xn.raw_tile(xy.delta(1, 0));
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }
    if let Some(&xy) = rd.tiles_by_kind_name("HCLK_DCM").iter().next() {
        let mut bel = builder.bel_virtual(bels::HCLK_DCM);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("HCLK_I{i}"), &[format!("HCLK_DCM_G_HCLKP{i}")])
                .extra_wire(
                    format!("HCLK_O_U{i}"),
                    &[format!("HCLK_DCM_LEAF_DIRECT_UP_HCLKP{i}")],
                )
                .extra_wire(
                    format!("HCLK_O_D{i}"),
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
            .xtile(tslots::HCLK_BEL, "HCLK_DCM", "HCLK_DCM", xy)
            .num_tiles(0)
            .raw_tile(xy.delta(1, 0))
            .bel(bel)
            .bel(bel_hclk_dcm_hrow.raw_tile(1))
            .extract();
    }

    for (tkn, naming) in [
        ("IOIS_LC", "IOIS_LC"),
        ("IOIS_LC_L", "IOIS_LC"),
        ("IOIS_NC", "IOIS_NC"),
        ("IOIS_NC_L", "IOIS_NC"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            builder.extract_xtile_bels(
                tslots::BEL,
                "IO",
                xy,
                &[],
                &[xy.delta(-1, 0)],
                naming,
                &[
                    builder
                        .bel_xy(bels::ILOGIC0, "ILOGIC", 0, 0)
                        .pins_name_only(&[
                            "OFB",
                            "TFB",
                            "SHIFTIN1",
                            "SHIFTIN2",
                            "SHIFTOUT1",
                            "SHIFTOUT2",
                            "D",
                            "CLK",
                            "OCLK",
                        ])
                        .extra_int_out("CLKMUX", &["IOIS_ICLKP_1"])
                        .extra_int_in("CLKMUX_INT", &["BYP_INT_B3_INT"]),
                    builder
                        .bel_xy(bels::ILOGIC1, "ILOGIC", 0, 1)
                        .pins_name_only(&[
                            "OFB",
                            "TFB",
                            "SHIFTIN1",
                            "SHIFTIN2",
                            "SHIFTOUT1",
                            "SHIFTOUT2",
                            "D",
                            "CLK",
                            "OCLK",
                        ])
                        .extra_int_out("CLKMUX", &["IOIS_ICLKP_0"])
                        .extra_int_in("CLKMUX_INT", &["BYP_INT_B1_INT"])
                        .extra_wire_force("CLKOUT", "IOIS_I_2GCLK0"),
                    builder
                        .bel_xy(bels::OLOGIC0, "OLOGIC", 0, 0)
                        .pins_name_only(&[
                            "OQ",
                            "SHIFTIN1",
                            "SHIFTIN2",
                            "SHIFTOUT1",
                            "SHIFTOUT2",
                            "CLK",
                        ])
                        .extra_int_out("CLKMUX", &["IOIS_OCLKP_1"])
                        .extra_int_in("CLKMUX_INT", &["BYP_INT_B6_INT"]),
                    builder
                        .bel_xy(bels::OLOGIC1, "OLOGIC", 0, 1)
                        .pins_name_only(&[
                            "OQ",
                            "SHIFTIN1",
                            "SHIFTIN2",
                            "SHIFTOUT1",
                            "SHIFTOUT2",
                            "CLK",
                        ])
                        .extra_int_out("CLKMUX", &["IOIS_OCLKP_0"])
                        .extra_int_in("CLKMUX_INT", &["BYP_INT_B5_INT"]),
                    builder
                        .bel_xy(bels::IOB0, "IOB", 0, 0)
                        .pins_name_only(&[
                            "I",
                            "O",
                            "T",
                            "PADOUT",
                            "DIFFI_IN",
                            "DIFFO_OUT",
                            "DIFFO_IN",
                        ])
                        .extra_wire_force(
                            "MONITOR",
                            if naming == "IOIS_LC" {
                                "IOIS_LC_MONITOR_N"
                            } else {
                                "IOIS_MONITOR_N"
                            },
                        ),
                    builder
                        .bel_xy(bels::IOB1, "IOB", 0, 1)
                        .pins_name_only(&[
                            "I",
                            "O",
                            "T",
                            "PADOUT",
                            "DIFFI_IN",
                            "DIFFO_OUT",
                            "DIFFO_IN",
                        ])
                        .extra_wire_force(
                            "MONITOR",
                            if naming == "IOIS_LC" {
                                "IOIS_LC_MONITOR_P"
                            } else {
                                "IOIS_MONITOR_P"
                            },
                        ),
                    builder
                        .bel_virtual(bels::IOI)
                        .extra_wire("HCLK0", &["IOIS_GCLKP0"])
                        .extra_wire("HCLK1", &["IOIS_GCLKP1"])
                        .extra_wire("HCLK2", &["IOIS_GCLKP2"])
                        .extra_wire("HCLK3", &["IOIS_GCLKP3"])
                        .extra_wire("HCLK4", &["IOIS_GCLKP4"])
                        .extra_wire("HCLK5", &["IOIS_GCLKP5"])
                        .extra_wire("HCLK6", &["IOIS_GCLKP6"])
                        .extra_wire("HCLK7", &["IOIS_GCLKP7"])
                        .extra_wire("RCLK0", &["IOIS_RCLK_FORIO_P0"])
                        .extra_wire("RCLK1", &["IOIS_RCLK_FORIO_P1"])
                        .extra_wire("IOCLK0", &["IOIS_IOCLKP0"])
                        .extra_wire("IOCLK1", &["IOIS_IOCLKP1"])
                        .extra_wire("IOCLK_N0", &["IOIS_IOCLKP_N0"])
                        .extra_wire("IOCLK_N1", &["IOIS_IOCLKP_N1"])
                        .extra_wire("IOCLK_S0", &["IOIS_IOCLKP_S0"])
                        .extra_wire("IOCLK_S1", &["IOIS_IOCLKP_S1"]),
                ],
            );
        }
    }

    for tkn in ["DCM", "DCM_BOT"] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut int_xy = Vec::new();
            for dy in 0..4 {
                int_xy.push(xy.delta(-1, dy));
            }
            let mut bel = builder
                .bel_xy(bels::DCM0, "DCM_ADV", 0, 0)
                .pins_name_only(&["CLKIN", "CLKFB"])
                .extra_int_out("CLKIN_TEST", &["DCM_ADV_CLKIN_TEST"])
                .extra_int_out("CLKFB_TEST", &["DCM_ADV_CLKFB_TEST"])
                .extra_int_in("CKINT0", &["CLK_B0_INT0_DCM0"])
                .extra_int_in("CKINT1", &["CLK_B1_INT0"])
                .extra_int_in("CKINT2", &["CLK_B2_INT0"])
                .extra_int_in("CKINT3", &["CLK_B3_INT0"])
                .extra_int_in("CLK_IN0", &["DCM_CLK_IN0"]);
            for pin in [
                "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX",
                "CLKFX180", "CONCUR", "LOCKED",
            ] {
                bel = bel.extra_wire(format!("{pin}_BUF"), &[format!("DCM_{pin}")]);
            }
            for i in 0..12 {
                bel = bel.extra_wire(format!("TO_BUFG{i}"), &[format!("DCM_TO_BUFG{i}")]);
            }
            for i in 0..24 {
                bel = bel
                    .extra_wire(
                        format!("BUSOUT{i}"),
                        &[format!("DCM_OUT{i}"), format!("DCM_BOT_OUT{i}")],
                    )
                    .extra_wire(
                        format!("BUSIN{i}"),
                        &[format!("DCM_IN{i}"), format!("DCM_BOT_IN{i}")],
                    );
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("HCLK{i}"), &[format!("DCM_BUFG{i}")]);
            }
            for i in 0..16 {
                bel = bel.extra_wire(format!("GIOB{i}"), &[format!("DCM_GIOB{i}")]);
            }
            for i in 0..4 {
                bel = bel.extra_wire(format!("MGT{i}"), &[format!("DCM_MGT{i}")]);
            }
            builder.extract_xtile_bels(tslots::BEL, "DCM", xy, &[], &int_xy, tkn, &[bel]);
        }
    }

    if let Some(&xy) = rd.tiles_by_kind_name("CCM").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        let mut bels = vec![];
        for i in 0..2 {
            let mut bel = builder
                .bel_xy(bels::PMCD[i], "PMCD", 0, i)
                .pins_name_only(&[
                    "CLKA", "CLKB", "CLKC", "CLKD", "REL", "CLKA1", "CLKA1D2", "CLKA1D4",
                    "CLKA1D8", "CLKB1", "CLKC1", "CLKD1",
                ])
                .extra_int_out("CLKA_TEST", &[format!("PMCD_{i}_CLKA_TEST")])
                .extra_int_out("CLKB_TEST", &[format!("PMCD_{i}_CLKB_TEST")])
                .extra_int_out("CLKC_TEST", &[format!("PMCD_{i}_CLKC_TEST")])
                .extra_int_out("CLKD_TEST", &[format!("PMCD_{i}_CLKD_TEST")])
                .extra_int_out("REL_TEST", &[format!("PMCD_{i}_REL_TEST")])
                .extra_int_in("CKINTC0", &["CLK_B0_INT1"])
                .extra_int_in("CKINTC1", &["CLK_B1_INT1"])
                .extra_int_in("CKINTC2", &["IMUX_B8_INT1"])
                .extra_int_in("CKINTC3", &["IMUX_B9_INT1"]);
            if i == 0 {
                bel = bel
                    .extra_int_in("CKINTA0", &["CLK_B2_INT2"])
                    .extra_int_in("CKINTA1", &["CLK_B3_INT2"])
                    .extra_int_in("CKINTA2", &["IMUX_B10_INT2"])
                    .extra_int_in("CKINTA3", &["IMUX_B11_INT2"])
                    .extra_int_in("CKINTB0", &["CLK_B0_INT2"])
                    .extra_int_in("CKINTB1", &["CLK_B1_INT2"])
                    .extra_int_in("CKINTB2", &["IMUX_B8_INT2"])
                    .extra_int_in("CKINTB3", &["IMUX_B9_INT2"])
                    .extra_int_in("REL_INT", &["IMUX_B0_INT0"]);
            } else {
                bel = bel
                    .extra_int_in("CKINTA0", &["CLK_B2_INT3"])
                    .extra_int_in("CKINTA1", &["CLK_B3_INT3"])
                    .extra_int_in("CKINTA2", &["IMUX_B10_INT3"])
                    .extra_int_in("CKINTA3", &["IMUX_B11_INT3"])
                    .extra_int_in("CKINTB0", &["CLK_B0_INT3"])
                    .extra_int_in("CKINTB1", &["CLK_B1_INT3"])
                    .extra_int_in("CKINTB2", &["IMUX_B8_INT3"])
                    .extra_int_in("CKINTB3", &["IMUX_B9_INT3"])
                    .extra_int_in("REL_INT", &["IMUX_B0_INT1"]);
            }
            bels.push(bel);
        }
        bels.push(
            builder
                .bel_xy(bels::DPM, "DPM", 0, 0)
                .pins_name_only(&[
                    "REFCLK",
                    "TESTCLK1",
                    "TESTCLK2",
                    "OSCOUT1",
                    "OSCOUT2",
                    "REFCLKOUT",
                ])
                .extra_int_in("CKINTA0", &["CLK_B0_INT0"])
                .extra_int_in("CKINTA1", &["CLK_B1_INT0"])
                .extra_int_in("CKINTA2", &["IMUX_B8_INT0"])
                .extra_int_in("CKINTA3", &["IMUX_B9_INT0"])
                .extra_int_in("CKINTB0", &["CLK_B2_INT1"])
                .extra_int_in("CKINTB1", &["CLK_B3_INT1"])
                .extra_int_in("CKINTB2", &["IMUX_B10_INT1"])
                .extra_int_in("CKINTB3", &["IMUX_B11_INT1"])
                .extra_int_out("REFCLK_TEST", &["DPM_REFCLK_TEST"])
                .extra_int_out("TESTCLK1_TEST", &["DPM_TESTCLK1_TEST"])
                .extra_int_out("TESTCLK2_TEST", &["DPM_TESTCLK2_TEST"]),
        );
        let mut bel = builder
            .bel_virtual(bels::CCM)
            .extra_int_in("CKINT", &["IMUX_B8_INT3"]);
        for i in 0..12 {
            bel = bel.extra_int_out(format!("TO_BUFG{i}"), &[format!("CCM_TO_BUFG{i}")]);
        }
        for i in 0..24 {
            bel = bel.extra_wire(format!("BUSIN{i}"), &[format!("CCM_DCM{i}")]);
        }
        for i in 0..8 {
            bel = bel.extra_wire(format!("HCLK{i}"), &[format!("CCM_BUFG{i}")]);
        }
        for i in 0..16 {
            bel = bel.extra_wire(format!("GIOB{i}"), &[format!("CCM_GIOB{i}")]);
        }
        for i in 0..4 {
            bel = bel.extra_wire(format!("MGT{i}"), &[format!("CCM_MGT{i}")]);
        }
        bels.push(bel);
        builder.extract_xtile_bels(tslots::BEL, "CCM", xy, &[], &int_xy, "CCM", &bels);
    }

    if let Some(&xy) = rd.tiles_by_kind_name("SYS_MON").iter().next() {
        let mut int_xy = Vec::new();
        for dy in 0..8 {
            int_xy.push(xy.delta(-1, dy));
        }
        let mut bel = builder
            .bel_xy(bels::SYSMON, "MONITOR", 0, 0)
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
        builder.extract_xtile_bels(
            tslots::BEL,
            "SYSMON",
            xy,
            &[],
            &int_xy,
            "SYSMON",
            &[
                bel,
                builder
                    .bel_xy(bels::IPAD_VP, "IPAD", 0, 0)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_VN, "IPAD", 0, 1)
                    .pins_name_only(&["O"]),
            ],
        );
    }

    for (tkn, naming) in [
        ("MGT_AL", "MGT.L"),
        ("MGT_AL_BOT", "MGT.L"),
        ("MGT_AL_MID", "MGT.L"),
        ("MGT_AR", "MGT.R"),
        ("MGT_AR_BOT", "MGT.R"),
        ("MGT_AR_MID", "MGT.R"),
    ] {
        if let Some(&xy) = rd.tiles_by_kind_name(tkn).iter().next() {
            let mut bels = vec![];
            for i in 0..2 {
                let mut bel = builder
                    .bel_xy(bels::GT11[i], "GT11", 0, 0)
                    .raw_tile(i)
                    .pins_name_only(&["TX1P", "TX1N", "RX1P", "RX1N", "RXMCLK"])
                    .pin_name_only("REFCLK1", 1)
                    .pin_name_only("REFCLK2", 1)
                    .pin_name_only("GREFCLK", 1)
                    .pin_name_only("TXPCSHCLKOUT", 1)
                    .pin_name_only("RXPCSHCLKOUT", 1)
                    .extra_wire("REFCLK", &["MGT_REFCLK"])
                    .extra_wire("PMACLK", &["MGT_PMACLK_OUT"])
                    .extra_wire("MGT0", &["MGT_MGT0"])
                    .extra_wire("MGT1", &["MGT_MGT1"])
                    .extra_wire("SYNCLK_OUT", &["MGT_SYNCLK_OUT"])
                    .extra_wire(
                        "SYNCLK1_OUT",
                        &["MGT_SYNCLK1_OUT", "MGT_SYNCLK1_LB", "MGT_SYNCLK1_RB"],
                    )
                    .extra_wire(
                        "SYNCLK2_OUT",
                        &["MGT_SYNCLK2_OUT", "MGT_SYNCLK2_LB", "MGT_SYNCLK2_RB"],
                    )
                    .extra_wire(
                        "FWDCLK0_OUT",
                        &[
                            "MGT_FWDCLK0A_L",
                            "MGT_FWDCLK0A_R",
                            "MGT_FWDCLK0B_L",
                            "MGT_FWDCLK0B_R",
                        ],
                    )
                    .extra_wire(
                        "FWDCLK1_OUT",
                        &[
                            "MGT_FWDCLK1A_L",
                            "MGT_FWDCLK1A_R",
                            "MGT_FWDCLK1B_L",
                            "MGT_FWDCLK1B_R",
                        ],
                    )
                    .extra_wire("FWDCLK1_B", &["MGT_FWDCLK1_B"])
                    .extra_wire("FWDCLK2_B", &["MGT_FWDCLK2_B"])
                    .extra_wire("FWDCLK3_B", &["MGT_FWDCLK3_B"])
                    .extra_wire("FWDCLK4_B", &["MGT_FWDCLK4_B"])
                    .extra_wire("FWDCLK1_T", &["MGT_FWDCLK1_T"])
                    .extra_wire("FWDCLK2_T", &["MGT_FWDCLK2_T"])
                    .extra_wire("FWDCLK3_T", &["MGT_FWDCLK3_T"])
                    .extra_wire("FWDCLK4_T", &["MGT_FWDCLK4_T"]);
                for i in 0..16 {
                    bel = bel.pins_name_only(&[format!("COMBUSIN{i}"), format!("COMBUSOUT{i}")]);
                }
                for i in 0..8 {
                    bel = bel.extra_wire(format!("HCLK{i}"), &[format!("MGT_G_HCLKP{i}")]);
                }
                if i == 0 {
                    bel = bel.pin_name_only("RXMCLK", 1);
                }
                bels.push(bel);
            }
            for i in 0..2 {
                bels.extend([
                    builder
                        .bel_xy(bels::IPAD_RXP[i], "IPAD", 0, 0)
                        .raw_tile(i)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(bels::IPAD_RXN[i], "IPAD", 0, 1)
                        .raw_tile(i)
                        .pins_name_only(&["O"]),
                    builder
                        .bel_xy(bels::OPAD_TXP[i], "OPAD", 0, 0)
                        .raw_tile(i)
                        .pins_name_only(&["I"]),
                    builder
                        .bel_xy(bels::OPAD_TXN[i], "OPAD", 0, 1)
                        .raw_tile(i)
                        .pins_name_only(&["I"]),
                ]);
            }
            let mut bel = builder
                .bel_xy(bels::GT11CLK, "GT11CLK", 0, 0)
                .raw_tile(2)
                .pins_name_only(&[
                    "SYNCLK1IN",
                    "SYNCLK2IN",
                    "SYNCLK1OUT",
                    "SYNCLK2OUT",
                    "RXBCLK",
                    "REFCLK",
                    "MGTCLKP",
                    "MGTCLKN",
                ])
                .extra_wire("PMACLK", &["GT11CLK_PMACLK_L", "GT11CLK_PMACLK_R"])
                .extra_wire("PMACLKA", &["GT11CLK_PMACLKA"])
                .extra_wire("PMACLKB", &["GT11CLK_PMACLKB"])
                .extra_wire("REFCLKA", &["GT11CLK_REFCLKA"])
                .extra_wire("REFCLKB", &["GT11CLK_REFCLKB"])
                .extra_wire(
                    "SYNCLK1_N",
                    &["GT11CLK_SYNCLK1OUT_L", "GT11CLK_SYNCLK1OUT_R"],
                )
                .extra_wire(
                    "SYNCLK2_N",
                    &["GT11CLK_SYNCLK2OUT_L", "GT11CLK_SYNCLK2OUT_R"],
                )
                .extra_wire("SYNCLK1_S", &["GT11CLK_SYNCLK1IN"])
                .extra_wire("SYNCLK2_S", &["GT11CLK_SYNCLK2IN"])
                .extra_wire("NFWDCLK1", &["GT11CLK_NFWDCLK1"])
                .extra_wire("NFWDCLK2", &["GT11CLK_NFWDCLK2"])
                .extra_wire("NFWDCLK3", &["GT11CLK_NFWDCLK3"])
                .extra_wire("NFWDCLK4", &["GT11CLK_NFWDCLK4"])
                .extra_wire("SFWDCLK1", &["GT11CLK_SFWDCLK1"])
                .extra_wire("SFWDCLK2", &["GT11CLK_SFWDCLK2"])
                .extra_wire("SFWDCLK3", &["GT11CLK_SFWDCLK3"])
                .extra_wire("SFWDCLK4", &["GT11CLK_SFWDCLK4"])
                .extra_wire(
                    "FWDCLK0A_OUT",
                    &["GT11CLK_FWDCLK0A_L", "GT11CLK_FWDCLK0A_R"],
                )
                .extra_wire(
                    "FWDCLK1A_OUT",
                    &["GT11CLK_FWDCLK1A_L", "GT11CLK_FWDCLK1A_R"],
                )
                .extra_wire(
                    "FWDCLK0B_OUT",
                    &["GT11CLK_FWDCLK0B_L", "GT11CLK_FWDCLK0B_R"],
                )
                .extra_wire(
                    "FWDCLK1B_OUT",
                    &["GT11CLK_FWDCLK1B_L", "GT11CLK_FWDCLK1B_R"],
                )
                .extra_wire("RXPCSHCLKOUTA", &["GT11CLK_RXPCSHCLKOUTA"])
                .extra_wire("RXPCSHCLKOUTB", &["GT11CLK_RXPCSHCLKOUTB"])
                .extra_wire("TXPCSHCLKOUTA", &["GT11CLK_TXPCSHCLKOUTA"])
                .extra_wire("TXPCSHCLKOUTB", &["GT11CLK_TXPCSHCLKOUTB"]);
            for i in 0..16 {
                bel = bel
                    .extra_wire(
                        format!("COMBUSIN_A{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_IN_AL{i}"),
                            format!("GT11_COMBUS_RCLK_IN_AR{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("COMBUSIN_B{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_IN_BL{i}"),
                            format!("GT11_COMBUS_RCLK_IN_BR{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("COMBUSOUT_A{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_OUT_AL{i}"),
                            format!("GT11_COMBUS_RCLK_OUT_AR{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("COMBUSOUT_B{i}"),
                        &[
                            format!("GT11_COMBUS_LCLK_OUT_BL{i}"),
                            format!("GT11_COMBUS_RCLK_OUT_BR{i}"),
                        ],
                    );
            }
            bels.extend([
                bel,
                builder
                    .bel_xy(bels::IPAD_CLKP0, "IPAD", 0, 1)
                    .raw_tile(2)
                    .pins_name_only(&["O"]),
                builder
                    .bel_xy(bels::IPAD_CLKN0, "IPAD", 0, 0)
                    .raw_tile(2)
                    .pins_name_only(&["O"]),
            ]);

            let mut xn = builder
                .xtile(tslots::BEL, "MGT", naming, xy.delta(0, -18))
                .raw_tile(xy)
                .raw_tile(xy.delta(0, -10))
                .num_tiles(32);
            for i in 0..32 {
                xn = xn.ref_int(
                    xy.delta(if xy.x == 0 { 1 } else { -1 }, -27 + (i + i / 8) as i32),
                    i,
                );
            }
            for bel in bels {
                xn = xn.bel(bel);
            }
            xn.extract();
        }
    }

    builder.build()
}
