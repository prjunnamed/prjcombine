use prjcombine_interconnect::{
    db::{IntDb, WireKind},
    dir::Dir,
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;
use prjcombine_virtex2::{
    bels,
    expanded::{REGION_HCLK, REGION_LEAF},
    tslots,
};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(rd);

    assert_eq!(builder.db.region_slots.insert("HCLK".into()).0, REGION_HCLK);
    assert_eq!(builder.db.region_slots.insert("LEAF".into()).0, REGION_LEAF);

    builder.db.init_slots(tslots::SLOTS, bels::SLOTS);

    builder.wire(
        "PULLUP",
        WireKind::TiePullup,
        &[
            "VCC_PINWIRE",
            "IOIS_VCC_WIRE",
            "BRAM_VCC_WIRE",
            "BRAM_IOIS_VCC_WIRE",
            "CNR_VCC_WIRE",
            "GIGABIT_INT_VCC_WIRE",
            "CLKB_VCC_WIRE",
            "CLKT_VCC_WIRE",
        ],
    );

    for i in 0..8 {
        builder.wire(
            format!("GCLK{i}"),
            WireKind::Regional(REGION_LEAF),
            &[format!("GCLK{i}")],
        );
    }
    for i in 0..8 {
        builder.logic_out(
            format!("DCM.CLKPAD{i}"),
            &[
                format!("BRAM_IOIS_DLL_CLKPAD{i}"),
                format!("GIGABIT_INT_DLL_CLKPAD{i}"),
            ],
        );
    }

    for (i, da1, da2, db, xname) in [
        (0, Dir::S, None, None, Some(0)),
        (1, Dir::W, Some(Dir::S), None, None),
        (2, Dir::E, None, Some(Dir::S), None),
        (3, Dir::S, Some(Dir::E), None, Some(1)),
        (4, Dir::S, None, None, Some(2)),
        (5, Dir::S, Some(Dir::W), None, Some(3)),
        (6, Dir::W, None, None, None),
        (7, Dir::E, Some(Dir::S), None, None),
        (8, Dir::E, Some(Dir::N), None, None),
        (9, Dir::W, None, None, None),
        (10, Dir::N, Some(Dir::W), None, Some(0)),
        (11, Dir::N, None, None, Some(1)),
        (12, Dir::N, Some(Dir::E), None, Some(2)),
        (13, Dir::E, None, Some(Dir::N), None),
        (14, Dir::W, Some(Dir::N), None, None),
        (15, Dir::N, None, None, Some(3)),
    ] {
        let omux = builder.mux_out(
            format!("OMUX{i}"),
            &[format!("OMUX{i}"), format!("LPPC_INT_OMUX{i}")],
        );
        let omux_da1 = builder.branch(
            omux,
            da1,
            format!("OMUX{i}.{da1}"),
            &[format!("OMUX_{da1}{i}"), format!("LPPC_INT_OMUX_{da1}{i}")],
        );
        match (xname, da1) {
            (None, _) => (),
            (Some(i), Dir::N) => {
                builder.extra_name_sub(format!("CLKB_TO_OMUXL{i}"), 0, omux_da1);
                builder.extra_name_sub(format!("CLKB_TO_OMUXR{i}"), 1, omux_da1);
            }
            (Some(i), Dir::S) => {
                builder.extra_name_sub(format!("CLKT_TO_OMUXL{i}"), 0, omux_da1);
                builder.extra_name_sub(format!("CLKT_TO_OMUXR{i}"), 1, omux_da1);
            }
            _ => unreachable!(),
        }
        if let Some(da2) = da2 {
            builder.branch(
                omux_da1,
                da2,
                format!("OMUX{i}.{da1}{da2}"),
                &[
                    format!("OMUX_{da1}{da2}{i}"),
                    format!("LPPC_INT_OMUX_{da1}{da2}{i}"),
                ],
            );
        }
        if let Some(db) = db {
            builder.branch(
                omux,
                db,
                format!("OMUX{i}.{db}"),
                &[format!("OMUX_{db}{i}"), format!("LPPC_INT_OMUX_{db}{i}")],
            );
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.mux_out(
                format!("DBL.{dir}{i}.0"),
                &[format!("{dir}2BEG{i}"), format!("LPPC_INT_{dir}2BEG{i}")],
            );
            let mid = builder.branch(
                beg,
                dir,
                format!("DBL.{dir}{i}.1"),
                &[format!("{dir}2MID{i}"), format!("LPPC_INT_{dir}2MID{i}")],
            );
            let end = builder.branch(
                mid,
                dir,
                format!("DBL.{dir}{i}.2"),
                &[format!("{dir}2END{i}"), format!("LPPC_INT_{dir}2END{i}")],
            );
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(
                    end,
                    Dir::S,
                    format!("DBL.{dir}{i}.3"),
                    &[
                        format!("{dir}2END_S{i}"),
                        format!("LPPC_INT_{dir}2END_S{i}"),
                    ],
                );
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(
                    end,
                    Dir::N,
                    format!("DBL.{dir}{i}.3"),
                    &[
                        format!("{dir}2END_N{i}"),
                        format!("LPPC_INT_{dir}2END_N{i}"),
                    ],
                );
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let mut last = builder.mux_out(
                format!("HEX.{dir}{i}.0"),
                &[
                    format!("{dir}6BEG{i}"),
                    format!("LR_IOIS_{dir}6BEG{i}"),
                    format!("TB_IOIS_{dir}6BEG{i}"),
                    format!("LPPC_INT_{dir}6BEG{i}"),
                ],
            );
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
                    &[
                        format!("{dir}6{seg}{i}"),
                        format!("LR_IOIS_{dir}6{seg}{i}"),
                        format!("TB_IOIS_{dir}6{seg}{i}"),
                        format!("LPPC_INT_{dir}6{seg}{i}"),
                    ],
                );
            }
            if matches!(dir, Dir::E | Dir::S) && i < 2 {
                builder.branch(
                    last,
                    Dir::S,
                    format!("HEX.{dir}{i}.7"),
                    &[
                        format!("{dir}6END_S{i}"),
                        format!("LR_IOIS_{dir}6END_S{i}"),
                        format!("TB_IOIS_{dir}6END_S{i}"),
                        format!("LPPC_INT_{dir}6END_S{i}"),
                    ],
                );
            }
            if matches!(dir, Dir::W | Dir::N) && i >= 8 {
                builder.branch(
                    last,
                    Dir::N,
                    format!("HEX.{dir}{i}.7"),
                    &[
                        format!("{dir}6END_N{i}"),
                        format!("LR_IOIS_{dir}6END_N{i}"),
                        format!("TB_IOIS_{dir}6END_N{i}"),
                        format!("LPPC_INT_{dir}6END_N{i}"),
                    ],
                );
            }
        }
    }

    let lh: Vec<_> = (0..24)
        .map(|i| {
            builder.wire(
                format!("LH.{i}"),
                WireKind::MultiBranch(builder.term_slots[Dir::W]),
                &[format!("LH{i}"), format!("LPPC_INT_LH{i}")],
            )
        })
        .collect();
    for i in 0..24 {
        builder.conn_branch(lh[i], Dir::E, lh[(i + 1) % 24]);
    }

    let lv: Vec<_> = (0..24)
        .map(|i| {
            builder.wire(
                format!("LV.{i}"),
                WireKind::MultiBranch(builder.term_slots[Dir::S]),
                &[format!("LV{i}")],
            )
        })
        .collect();
    for i in 0..24 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 23) % 24]);
    }

    for i in 0..4 {
        let wire = builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[
                format!("CLK{i}"),
                format!("IOIS_CK{j}_B{k}", j = [2, 1, 2, 1][i], k = [1, 1, 3, 3][i]),
                format!("BRAM_CLK{i}"),
                format!("CNR_CLK{i}"),
                format!("LRPPC_INT_CLK{i}"),
                format!("BPPC_INT_CLK{i}"),
                format!("TPPC_INT_CLK{i}"),
            ],
        );
        let name = format!("GIGABIT_INT_CLK{i}");
        for tile in [
            "BGIGABIT_INT0",
            "BGIGABIT_INT1",
            "BGIGABIT_INT2",
            "BGIGABIT_INT3",
            "TGIGABIT_INT0",
            "TGIGABIT_INT1",
            "TGIGABIT_INT2",
            "TGIGABIT_INT3",
            "BGIGABIT10_INT0",
            "BGIGABIT10_INT1",
            "BGIGABIT10_INT2",
            "BGIGABIT10_INT3",
            "BGIGABIT10_INT4",
            "BGIGABIT10_INT5",
            "BGIGABIT10_INT6",
            "BGIGABIT10_INT7",
            "TGIGABIT10_INT0",
            "TGIGABIT10_INT1",
            "TGIGABIT10_INT2",
            "TGIGABIT10_INT3",
            "TGIGABIT10_INT4",
            "TGIGABIT10_INT5",
            "TGIGABIT10_INT6",
            "TGIGABIT10_INT7",
        ] {
            builder.extra_name_tile(tile, &name, wire)
        }
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.IOI.ICLK{i}"),
            &[format!(
                "IOIS_CK{j}_B{k}",
                j = [2, 1, 2, 1][i],
                k = [0, 0, 2, 2][i]
            )],
        );
    }
    for i in 0..4 {
        let wire = builder.mux_out(
            format!("IMUX.DCMCLK{i}"),
            &[["BRAM_IOIS_CLKFB", "BRAM_IOIS_CLKIN", "BRAM_IOIS_PSCLK", ""][i].to_string()],
        );
        let name = format!("GIGABIT_INT_CLK{i}");
        for tile in [
            "BGIGABIT_INT4",
            "TGIGABIT_INT4",
            "BGIGABIT10_INT8",
            "TGIGABIT10_INT8",
        ] {
            builder.extra_name_tile(tile, &name, wire)
        }
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[
                format!("SR{i}"),
                format!("IOIS_SR_B{j}", j = [1, 2, 0, 3][i]),
                format!("BRAM_SR{i}"),
                format!("BRAM_IOIS_SR{i}"),
                format!("CNR_SR{i}"),
                format!("LRPPC_INT_SR{i}"),
                format!("BPPC_INT_SR{i}"),
                format!("TPPC_INT_SR{i}"),
                format!("GIGABIT_INT_SR{i}"),
            ],
        );
    }
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CE{i}"),
            &[
                format!("CE_B{i}"),
                format!("OCE_B{j}", j = [1, 0, 3, 2][i]),
                format!("BRAM_CE_B{i}"),
                // only 2, 3 actually exist
                format!("BRAM_IOIS_CE_B{i}"),
                format!("CNR_CE_B{i}"),
                format!("LRPPC_INT_CE_B{i}"),
                format!("BPPC_INT_CE_B{i}"),
                format!("TPPC_INT_CE_B{i}"),
                format!("GIGABIT_INT_CE_B{i}"),
            ],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.TI{i}"),
            &[
                format!("TI{i}"),
                format!("BRAM_TI{i}"),
                format!("BRAM_IOIS_TI{i}"),
                format!("CNR_TI{i}"),
                format!("LRPPC_INT_TI{i}"),
                format!("BPPC_INT_TI{i}"),
                format!("TPPC_INT_TI{i}"),
                format!("GIGABIT_INT_TI{i}"),
            ],
        );
    }
    for i in 0..2 {
        builder.mux_out(
            format!("IMUX.TS{i}"),
            &[
                format!("TS{i}"),
                format!("BRAM_TS{i}"),
                format!("CNR_TS{i}"),
                format!("LRPPC_INT_TS{i}"),
                format!("BPPC_INT_TS{i}"),
                format!("TPPC_INT_TS{i}"),
                format!("GIGABIT_INT_TS{i}"),
            ],
        );
    }

    // CLB inputs
    for i in 0..4 {
        for j in 1..5 {
            builder.mux_out(format!("IMUX.S{i}.F{j}"), &[format!("F{j}_B{i}")]);
        }
        for j in 1..5 {
            builder.mux_out(format!("IMUX.S{i}.G{j}"), &[format!("G{j}_B{i}")]);
        }
        builder.mux_out(format!("IMUX.S{i}.BX"), &[format!("BX{i}")]);
        builder.mux_out(format!("IMUX.S{i}.BY"), &[format!("BY{i}")]);
    }
    // non-CLB inputs
    for i in 0..4 {
        for j in 0..2 {
            let ri = 3 - i;
            builder.mux_out(
                format!("IMUX.G{i}.FAN{j}"),
                &[
                    match (i, j) {
                        (0, 0) => "IOIS_FAN_BX0",
                        (0, 1) => "IOIS_FAN_BX2",
                        (1, 0) => "IOIS_FAN_BY0",
                        (1, 1) => "IOIS_FAN_BY2",
                        (2, 0) => "IOIS_FAN_BY1",
                        (2, 1) => "IOIS_FAN_BY3",
                        (3, 0) => "IOIS_FAN_BX1",
                        (3, 1) => "IOIS_FAN_BX3",
                        _ => unreachable!(),
                    }
                    .to_string(),
                    format!("CNR_FAN{ri}{j}"),
                    format!("BRAM_FAN{ri}{j}"),
                    format!("LRPPC_INT_FAN{ri}{j}"),
                    format!("BPPC_INT_FAN{ri}{j}"),
                    format!("TPPC_INT_FAN{ri}{j}"),
                    format!("GIGABIT_INT_FAN{ri}{j}"),
                ],
            );
        }
        for j in 0..8 {
            builder.mux_out(
                format!("IMUX.G{i}.DATA{j}"),
                &[
                    match (i, j) {
                        (_, 5) => format!("IOIS_REV_B{i}"),
                        (_, 6) => format!("O2_B{i}"),
                        (_, 7) => format!("O1_B{i}"),
                        _ => "".to_string(),
                    },
                    format!("DATA_IN{k}", k = i * 8 + j), // CNR
                    match (i, j) {
                        (0, 2) => "BRAM_DIPB".to_string(),
                        (0, 3) => "BRAM_DIPA".to_string(),
                        (2, 2) => "BRAM_MULTINB16".to_string(),
                        (2, 3) => "BRAM_MULTINB17".to_string(),
                        (3, 2) => "BRAM_MULTINA16".to_string(),
                        (3, 3) => "BRAM_MULTINA17".to_string(),
                        (_, 4) => format!("BRAM_DIB{i}"),
                        (_, 5) => format!("BRAM_DIB{k}", k = 16 + i),
                        (_, 6) => format!("BRAM_DIA{i}"),
                        (_, 7) => format!("BRAM_DIA{k}", k = 16 + i),
                        _ => "".to_string(),
                    },
                    match (i, j) {
                        (0, 0) => "BRAM_IOIS_DSSEN".to_string(),
                        (0, 1) => "BRAM_IOIS_CTLSEL0".to_string(),
                        (0, 2) => "BRAM_IOIS_CTLSEL1".to_string(),
                        (0, 3) => "BRAM_IOIS_CTLSEL2".to_string(),
                        (1, 0) => "BRAM_IOIS_PSEN".to_string(),
                        (1, 1) => "BRAM_IOIS_CTLOSC2".to_string(),
                        (1, 2) => "BRAM_IOIS_CTLOSC1".to_string(),
                        (1, 3) => "BRAM_IOIS_CTLGO".to_string(),
                        (2, 0) => "BRAM_IOIS_PSINCDEC".to_string(),
                        (2, 1) => "BRAM_IOIS_CTLMODE".to_string(),
                        (2, 2) => "BRAM_IOIS_FREEZEDLL".to_string(),
                        (2, 3) => "BRAM_IOIS_FREEZEDFS".to_string(),
                        (3, 0) => "BRAM_IOIS_RST".to_string(),
                        (3, 1) => "BRAM_IOIS_STSADRS0".to_string(),
                        (3, 2) => "BRAM_IOIS_STSADRS1".to_string(),
                        (3, 3) => "BRAM_IOIS_STSADRS2".to_string(),
                        (3, 4) => "BRAM_IOIS_STSADRS3".to_string(),
                        (3, 5) if rd.family == "virtex2p" => "BRAM_IOIS_STSADRS4".to_string(),
                        _ => format!("BRAM_IOIS_DATA{k}", k = i * 8 + j),
                    },
                    format!("LRPPC_INT_DATA_IN{k}", k = j * 4 + i),
                    format!("BPPC_INT_DATA_IN{k}", k = j * 4 + i),
                    format!("TPPC_INT_DATA_IN{k}", k = j * 4 + i),
                    format!("GIGABIT_INT_DATA_IN{k}", k = j * 4 + i),
                ],
            );
        }
    }
    // IOI special inputs
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.TS1{i}"), &[format!("TS1_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.TS2{i}"), &[format!("TS2_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.ICE{i}"), &[format!("ICE_B{i}")]);
    }
    for i in 0..4 {
        builder.mux_out(format!("IMUX.IOI.TCE{i}"), &[format!("TCE_B{i}")]);
    }
    // BRAM special inputs
    let bram_s = builder.make_term_naming("BRAM.S");
    for ab in ['A', 'B'] {
        for i in 0..4 {
            let root = builder.mux_out(
                format!("IMUX.BRAM_ADDR{ab}{i}"),
                &[format!("BRAM_ADDR{ab}_B{i}")],
            );
            for dir in [Dir::S, Dir::N] {
                let mut last = root;
                for j in 1..5 {
                    if dir == Dir::N {
                        builder.name_term_in_far_wire(
                            bram_s,
                            last,
                            format!("BRAMSITE_NADDRIN_{ab}_S{k}", k = (i ^ 3) + (j - 1) * 4),
                        );
                        let idx = i + (4 - j) * 4;
                        if ab == 'A' && idx < 14 {
                            builder.extra_name_sub(format!("PPC_ADR_T00_{idx}"), 40, last);
                            builder.extra_name_sub(format!("PPC_ADR_T01_{idx}"), 47, last);
                        }
                    } else {
                        let idx = i + (j - 1) * 4;
                        if ab == 'A' && idx < 14 {
                            builder.extra_name_sub(format!("PPC_ADR_B00_{idx}"), 32, last);
                            builder.extra_name_sub(format!("PPC_ADR_B01_{idx}"), 39, last);
                        }
                        if ab == 'B' && idx < 14 {
                            builder.extra_name_sub(
                                format!("PPC_ADR_B00_{idx}", idx = idx + 14),
                                32,
                                last,
                            );
                            builder.extra_name_sub(
                                format!("PPC_ADR_B01_{idx}", idx = idx + 14),
                                39,
                                last,
                            );
                        }
                    }
                    if j == 4 {
                        last = builder.branch(
                            last,
                            dir,
                            format!("IMUX.BRAM_ADDR{ab}{i}.{dir}4"),
                            &[format!("BRAM_ADDR{ab}_{dir}END{i}")],
                        );
                    } else {
                        last = builder.branch(
                            last,
                            dir,
                            format!("IMUX.BRAM_ADDR{ab}{i}.{dir}{j}"),
                            &[""],
                        );
                    }
                    if dir == Dir::N {
                        if ab == 'A' {
                            builder.name_term_out_wire(
                                bram_s,
                                last,
                                format!(
                                    "BRAMSITE_NADDRIN_{ab}{k}",
                                    k = 15 - ((i ^ 3) + (j - 1) * 4)
                                ),
                            );
                        } else {
                            builder.name_term_out_wire(
                                bram_s,
                                last,
                                format!("BRAMSITE_NADDRIN_{ab}{k}", k = (i ^ 3) + (j - 1) * 4),
                            );
                        }
                    }
                }
            }
        }
    }

    // logic out stuff
    for i in 0..8 {
        let w = builder.logic_out(
            format!("OUT.FAN{i}"),
            &[
                // In CLBs, used for combinatorial outputs.
                ["X0", "X1", "X2", "X3", "Y0", "Y1", "Y2", "Y3"][i],
                // In IOIS, used for combinatorial inputs.  4-7 are unused.
                ["I0", "I1", "I2", "I3", "", "", "", ""][i],
                // In BRAM, used for low data outputs.
                [
                    "BRAM_DOA2",
                    "BRAM_DOA3",
                    "BRAM_DOA0",
                    "BRAM_DOA1",
                    "BRAM_DOB1",
                    "BRAM_DOB0",
                    "BRAM_DOB3",
                    "BRAM_DOB2",
                ][i],
                &format!("DOUT_FAN{i}"),
                &format!("LRPPC_INT_PPC1{i}"),
                &format!("BPPC_INT_PPC1{i}"),
                &format!("TPPC_INT_PPC1{i}"),
                &format!("GIGABIT_INT_PPC1{i}"),
            ],
        );
        if i == 0 {
            builder.extra_name_tile("MK_T_IOIS", "IOIS_BREFCLK_SE", w);
        }
        if i == 2 {
            builder.extra_name_tile("MK_B_IOIS", "IOIS_BREFCLK_SE", w);
        }
    }

    // We call secondary outputs by their OMUX index.
    for i in 2..24 {
        builder.logic_out(
            format!("OUT.SEC{i}"),
            &[
                [
                    "", "", "", "", "", "", "", "", "YB0", "YB1", "YB3", "YB2", "XB1", "XB2",
                    "XB3", "YQ0", "YQ1", "XB0", "YQ2", "YQ3", "XQ0", "XQ1", "XQ2", "XQ3",
                ][i],
                [
                    "", "", "", "", "", "", "", "", "", "I_Q21", "I_Q23", "", "TS_FDBK1",
                    "TS_FDBK2", "TS_FDBK3", "I_Q20", "", "TS_FDBK0", "I_Q22", "", "I_Q10", "I_Q11",
                    "I_Q12", "I_Q13",
                ][i],
                [
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_DOPA",
                    "BRAM_DOPB",
                    "",
                    "BRAM_MOUT32",
                    "BRAM_MOUT7",
                    "BRAM_MOUT6",
                    "BRAM_MOUT5",
                    "BRAM_MOUT4",
                    "BRAM_MOUT3",
                    "BRAM_MOUT2",
                    "BRAM_MOUT1",
                    "BRAM_MOUT0",
                ][i],
                [
                    "",
                    "",
                    "BRAM_IOIS_CLKFX180",
                    "BRAM_IOIS_CLKFX",
                    "BRAM_IOIS_CLKDV",
                    "BRAM_IOIS_CLK2X180",
                    "BRAM_IOIS_CLK2X",
                    "BRAM_IOIS_CLK270",
                    "BRAM_IOIS_CLK180",
                    "BRAM_IOIS_CLK90",
                    "BRAM_IOIS_CLK0",
                    "BRAM_IOIS_CONCUR",
                    "BRAM_IOIS_PSDONE",
                    "BRAM_IOIS_LOCKED",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                ][i],
                &if (8..16).contains(&i) {
                    format!("LRPPC_INT_PPC2{k}", k = 15 - i)
                } else {
                    String::new()
                },
                &if (8..16).contains(&i) {
                    format!("BPPC_INT_PPC2{k}", k = 15 - i)
                } else {
                    String::new()
                },
                &if (8..16).contains(&i) {
                    format!("TPPC_INT_PPC2{k}", k = 15 - i)
                } else {
                    String::new()
                },
                &if (8..16).contains(&i) {
                    format!("GIGABIT_INT_PPC2{k}", k = 15 - i)
                } else {
                    String::new()
                },
            ],
        );
    }

    // Same for tertiary.
    for i in 8..18 {
        for j in 0..2 {
            builder.logic_out(
                format!("OUT.HALF{i}.{j}"),
                &[
                    format!("DOUT{k}", k = (17 - i) * 2 + j),
                    match (i, j) {
                        (8, 0) => "BRAM_DOA16",
                        (9, 0) => "BRAM_DOA17",
                        (10, 0) => "BRAM_DOA19",
                        (11, 0) => "BRAM_DOA18",
                        (8, 1) => "BRAM_DOB16",
                        (9, 1) => "BRAM_DOB17",
                        (10, 1) => "BRAM_DOB19",
                        (11, 1) => "BRAM_DOB18",
                        (14, 0) => "BRAM_IOIS_STATUS0",
                        (15, 0) => "BRAM_IOIS_STATUS1",
                        (16, 0) => "BRAM_IOIS_STATUS2",
                        (17, 0) => "BRAM_IOIS_STATUS3",
                        (14, 1) => "BRAM_IOIS_STATUS4",
                        (15, 1) => "BRAM_IOIS_STATUS5",
                        (16, 1) => "BRAM_IOIS_STATUS6",
                        (17, 1) => "BRAM_IOIS_STATUS7",
                        _ => "",
                    }
                    .to_string(),
                ],
            );
        }
    }

    for i in 0..16 {
        builder.logic_out(
            format!("OUT.TEST{i}"),
            &[
                format!("LRPPC_INT_TEST{i}"),
                format!("BPPC_INT_TEST{i}"),
                format!("TPPC_INT_TEST{i}"),
                format!("GIGABIT_INT_TEST{i}"),
            ],
        );
    }

    builder.logic_out("OUT.TBUS", &["TBUS"]);
    let w = builder.logic_out(
        "OUT.PCI0",
        &[
            "LTERM_PCI_OUT_D0",
            "LTERM_PCI_OUT_U0",
            "RTERM_PCI_OUT_D0",
            "RTERM_PCI_OUT_U0",
        ],
    );
    builder.extra_name_sub("REG_R_PCI_OUT_D2", 0, w);
    builder.extra_name_sub("REG_R_PCI_OUT_D0", 1, w);
    builder.extra_name_sub("REG_R_PCI_OUT_U0", 2, w);
    builder.extra_name_sub("REG_R_PCI_OUT_U2", 3, w);
    let w = builder.logic_out(
        "OUT.PCI1",
        &[
            "LTERM_PCI_OUT_D1",
            "LTERM_PCI_OUT_U1",
            "RTERM_PCI_OUT_D1",
            "RTERM_PCI_OUT_U1",
        ],
    );
    builder.extra_name_sub("REG_R_PCI_OUT_D1", 1, w);
    builder.extra_name_sub("REG_R_PCI_OUT_U1", 2, w);

    for i in 0..8 {
        builder.mux_out(
            format!("CLK.IMUX.SEL{i}"),
            &[format!("CLKB_SELDUB{i}"), format!("CLKT_SELDUB{i}")],
        );
    }
    for i in 0..8 {
        let ii = i % 4;
        let lr = if i < 4 { 'R' } else { 'L' };
        builder.mux_out(
            format!("CLK.IMUX.CLK{i}"),
            &[
                format!("CLKB_CLKDUB{lr}{ii}"),
                format!("CLKT_CLKDUB{lr}{ii}"),
            ],
        );
    }
    for i in 0..8 {
        builder.logic_out(
            format!("CLK.OUT.{i}"),
            &[format!("CLKB_GCLK_ROOT{i}"), format!("CLKT_GCLK_ROOT{i}")],
        );
    }

    builder.extract_main_passes();

    let bels_int = [builder.bel_xy(bels::RLL, "RLL", 0, 0)];
    let bels_int_sigh = [builder
        .bel_xy(bels::RLL, "RLL", 0, 0)
        .pins_name_only(&["LH0", "LH6", "LH12", "LH18", "LV0", "LV6", "LV12", "LV18"])];
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "CENTER",
        "INT.CLB",
        "INT.CLB",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LR_IOIS",
        "INT.IOI",
        "INT.IOI.LR",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TB_IOIS",
        "INT.IOI",
        "INT.IOI.TB",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "ML_TB_IOIS",
        "INT.IOI",
        "INT.IOI.TB",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "ML_TBS_IOIS",
        "INT.IOI",
        "INT.IOI.TB",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "GIGABIT_IOI",
        "INT.IOI",
        "INT.IOI.TB",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "GIGABIT10_IOI",
        "INT.IOI",
        "INT.IOI.TB",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "MK_B_IOIS",
        "INT.IOI.CLK_B",
        "INT.IOI.CLK_B",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "MK_T_IOIS",
        "INT.IOI.CLK_T",
        "INT.IOI.CLK_T",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BRAM0",
        "INT.BRAM",
        "INT.BRAM",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BRAM1",
        "INT.BRAM",
        "INT.BRAM",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BRAM2",
        "INT.BRAM",
        "INT.BRAM",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BRAM3",
        "INT.BRAM",
        "INT.BRAM",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BRAM_IOIS",
        "INT.DCM.V2",
        "INT.BRAM_IOIS",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "ML_BRAM_IOIS",
        "INT.DCM.V2P",
        "INT.ML_BRAM_IOIS",
        &bels_int_sigh,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LL",
        "INT.CNR",
        "INT.CNR",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LR",
        "INT.CNR",
        "INT.CNR",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "UL",
        "INT.CNR",
        "INT.CNR",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "UR",
        "INT.CNR",
        "INT.CNR",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT_INT0",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT_INT1",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT_INT2",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT_INT3",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT_INT4",
        "INT.GT.CLKPAD",
        "INT.GT.CLKPAD",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT_INT0",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT_INT1",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT_INT2",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT_INT3",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT_INT4",
        "INT.GT.CLKPAD",
        "INT.GT.CLKPAD",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT0",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT1",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT2",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT3",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT4",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT5",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT6",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT7",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BGIGABIT10_INT8",
        "INT.GT.CLKPAD",
        "INT.GT.CLKPAD",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT0",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT1",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT2",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT3",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT4",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT5",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT6",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT7",
        "INT.PPC",
        "INT.GT",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TGIGABIT10_INT8",
        "INT.GT.CLKPAD",
        "INT.GT.CLKPAD",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LPPC_X0Y0_INT",
        "INT.PPC",
        "INT.PPC.L",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LPPC_X1Y0_INT",
        "INT.PPC",
        "INT.PPC.L",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LLPPC_X0Y0_INT",
        "INT.PPC",
        "INT.PPC.L",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "LLPPC_X1Y0_INT",
        "INT.PPC",
        "INT.PPC.L",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "ULPPC_X0Y0_INT",
        "INT.PPC",
        "INT.PPC.L",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "ULPPC_X1Y0_INT",
        "INT.PPC",
        "INT.PPC.L",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "RPPC_X0Y0_INT",
        "INT.PPC",
        "INT.PPC.R",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "RPPC_X1Y0_INT",
        "INT.PPC",
        "INT.PPC.R",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BPPC_X0Y0_INT",
        "INT.PPC",
        "INT.PPC.B",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "BPPC_X1Y0_INT",
        "INT.PPC",
        "INT.PPC.B",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TPPC_X0Y0_INT",
        "INT.PPC",
        "INT.PPC.T",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        bels::INT,
        "TPPC_X1Y0_INT",
        "INT.PPC",
        "INT.PPC.T",
        &bels_int,
    );

    let slice_name_only = [
        "DX", "DY", "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG",
        "DIG", "SLICEWE0", "SLICEWE1", "SLICEWE2", "BXOUT", "BYOUT", "BYINVOUT", "SOPIN", "SOPOUT",
    ];

    let bels_clb = [
        builder
            .bel_xy(bels::SLICE0, "SLICE", 0, 0)
            .pins_name_only(&slice_name_only)
            .extra_wire("BYINVOUT_LOCAL", &["BYINVOUT_LOCAL0"]),
        builder
            .bel_xy(bels::SLICE1, "SLICE", 0, 1)
            .pins_name_only(&slice_name_only)
            .extra_wire("FX_S", &["FX_S1"])
            .extra_wire("COUT_N", &["COUT_N3"])
            .extra_wire("BYOUT_LOCAL", &["BYOUT_LOCAL1"])
            .extra_wire("BYINVOUT_LOCAL", &["BYINVOUT_LOCAL1"]),
        builder
            .bel_xy(bels::SLICE2, "SLICE", 1, 0)
            .pins_name_only(&slice_name_only)
            .extra_wire("SOPOUT_W", &["SOPOUT_W2"]),
        builder
            .bel_xy(bels::SLICE3, "SLICE", 1, 1)
            .pins_name_only(&slice_name_only)
            .extra_wire("COUT_N", &["COUT_N1"])
            .extra_wire("DIG_LOCAL", &["DIG_LOCAL3"])
            .extra_wire("DIG_S", &["DIG_S3"])
            .extra_wire("SOPOUT_W", &["SOPOUT_W3"]),
        builder
            .bel_indexed(bels::TBUF0, "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed(bels::TBUF1, "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual(bels::TBUS)
            .extra_wire("BUS0", &["TBUF0"])
            .extra_wire("BUS1", &["TBUF1"])
            .extra_wire("BUS2", &["TBUF2"])
            .extra_wire("BUS3", &["TBUF3"])
            .extra_wire("BUS3_E", &["TBUF3_E"])
            .extra_int_out("OUT", &["TBUS"]),
    ];
    builder.extract_node_bels(tslots::BEL, "CENTER", "CLB", "CLB", &bels_clb);

    let ioi_name_only = ["DIFFI_IN", "PADOUT", "DIFFO_IN", "DIFFO_OUT"];
    let bels_ioi = [
        builder
            .bel_indexed(bels::IO0, "IOB", 0)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF0"),
        builder
            .bel_indexed(bels::IO1, "IOB", 1)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF1"),
        builder
            .bel_indexed(bels::IO2, "IOB", 2)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF2"),
        builder
            .bel_indexed(bels::IO3, "IOB", 3)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF3"),
    ];
    builder.extract_node_bels(tslots::BEL, "LR_IOIS", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels(tslots::BEL, "TB_IOIS", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels(tslots::BEL, "ML_TB_IOIS", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels(tslots::BEL, "ML_TBS_IOIS", "IOI", "IOI.TBS", &bels_ioi);
    builder.extract_node_bels(tslots::BEL, "GIGABIT_IOI", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels(tslots::BEL, "GIGABIT10_IOI", "IOI", "IOI", &bels_ioi);
    if rd.part.contains("vpx") {
        let bels_ioi_clk_b = [
            bels_ioi[0].clone(),
            bels_ioi[1].clone(),
            builder
                .bel_single(bels::IO[2], "CLKPPAD2")
                .pin_name_only("I", 1),
            builder
                .bel_single(bels::IO[3], "CLKNPAD2")
                .pin_name_only("I", 1),
            builder
                .bel_virtual(bels::BREFCLK_INT)
                .extra_int_out("BREFCLK", &["IOIS_BREFCLK_SE"]),
        ];
        let bels_ioi_clk_t = [
            builder
                .bel_single(bels::IO[0], "CLKPPAD1")
                .pin_name_only("I", 1),
            builder
                .bel_single(bels::IO[1], "CLKNPAD1")
                .pin_name_only("I", 1),
            bels_ioi[2].clone(),
            bels_ioi[3].clone(),
            builder
                .bel_virtual(bels::BREFCLK_INT)
                .extra_int_out("BREFCLK", &["IOIS_BREFCLK_SE"]),
        ];
        builder.extract_node_bels(
            tslots::BEL,
            "MK_B_IOIS",
            "IOI.CLK_B",
            "IOI.CLK_B",
            &bels_ioi_clk_b,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "MK_T_IOIS",
            "IOI.CLK_T",
            "IOI.CLK_T",
            &bels_ioi_clk_t,
        );
    }

    if rd.family == "virtex2p" {
        for (kind, num) in [
            ("IOBS.V2P.B.L2", 2),
            ("IOBS.V2P.B.R2", 2),
            ("IOBS.V2P.B.R2.CLK", 2),
            ("IOBS.V2P.B.L1", 1),
            ("IOBS.V2P.B.L1.ALT", 1),
            ("IOBS.V2P.B.R1", 1),
            ("IOBS.V2P.B.R1.ALT", 1),
            ("IOBS.V2P.T.L2", 2),
            ("IOBS.V2P.T.R2", 2),
            ("IOBS.V2P.T.R2.CLK", 2),
            ("IOBS.V2P.T.L1", 1),
            ("IOBS.V2P.T.L1.ALT", 1),
            ("IOBS.V2P.T.R1", 1),
            ("IOBS.V2P.T.R1.ALT", 1),
            ("IOBS.V2P.L.B2", 2),
            ("IOBS.V2P.L.T2", 2),
            ("IOBS.V2P.R.B2", 2),
            ("IOBS.V2P.R.T2", 2),
        ] {
            builder.make_marker_node(tslots::IOB, kind, num);
        }
    } else {
        for (kind, num) in [
            ("IOBS.V2.B.L2", 2),
            ("IOBS.V2.B.R2", 2),
            ("IOBS.V2.T.L2", 2),
            ("IOBS.V2.T.R2", 2),
            ("IOBS.V2.L.B2", 2),
            ("IOBS.V2.L.T2", 2),
            ("IOBS.V2.R.B2", 2),
            ("IOBS.V2.R.T2", 2),
        ] {
            builder.make_marker_node(tslots::IOB, kind, num);
        }
    }

    let bels_dcm = [builder.bel_xy(bels::DCM, "DCM", 0, 0)];
    builder.extract_node_bels(tslots::BEL, "BRAM_IOIS", "DCM.V2", "DCM.V2", &bels_dcm);
    builder.extract_node_bels(tslots::BEL, "ML_BRAM_IOIS", "DCM.V2P", "DCM.V2P", &bels_dcm);

    let (ll, lr, ul, ur) = if rd.family == "virtex2p" {
        ("LL.V2P", "LR.V2P", "UL.V2P", "UR.V2P")
    } else {
        ("LL.V2", "LR.V2", "UL.V2", "UR.V2")
    };
    builder.extract_node_bels(
        tslots::BEL,
        "LL",
        ll,
        ll,
        &[
            builder.bel_indexed(bels::DCI0, "DCI", 6),
            builder.bel_indexed(bels::DCI1, "DCI", 5),
        ],
    );

    builder.extract_node_bels(
        tslots::BEL,
        "LR",
        lr,
        lr,
        &[
            builder.bel_indexed(bels::DCI0, "DCI", 3),
            builder.bel_indexed(bels::DCI1, "DCI", 4),
            builder.bel_single(bels::STARTUP, "STARTUP"),
            builder.bel_single(bels::CAPTURE, "CAPTURE"),
            builder.bel_single(bels::ICAP, "ICAP"),
        ],
    );
    builder.extract_node_bels(
        tslots::BEL,
        "UL",
        ul,
        ul,
        &[
            builder.bel_indexed(bels::DCI0, "DCI", 7),
            builder.bel_indexed(bels::DCI1, "DCI", 0),
            builder.bel_single(bels::PMV, "PMV"),
        ],
    );
    if rd.family == "virtex2p" {
        builder.extract_node_bels(
            tslots::BEL,
            "UR",
            ur,
            ur,
            &[
                builder.bel_indexed(bels::DCI0, "DCI", 2),
                builder.bel_indexed(bels::DCI1, "DCI", 1),
                builder.bel_single(bels::BSCAN, "BSCAN"),
                builder.bel_single(bels::JTAGPPC, "JTAGPPC"),
            ],
        );
    } else {
        builder.extract_node_bels(
            tslots::BEL,
            "UR",
            ur,
            ur,
            &[
                builder.bel_indexed(bels::DCI0, "DCI", 2),
                builder.bel_indexed(bels::DCI1, "DCI", 1),
                builder.bel_single(bels::BSCAN, "BSCAN"),
            ],
        );
    }

    for (tkn, n) in [
        ("LTERM321", "TERM.W.U"),
        ("LTERM010", "TERM.W.U"),
        ("LTERM323", "TERM.W.D"),
        ("LTERM210", "TERM.W.D"),
        ("LTERM323_PCI", "TERM.W.U"),
        ("LTERM210_PCI", "TERM.W.U"),
        ("CNR_LTERM", "TERM.W"),
    ] {
        builder.extract_term(
            "TERM.W",
            Some((tslots::TERM_H, bels::TERM_W, "TERM.W")),
            Dir::W,
            tkn,
            n,
        );
    }
    for (tkn, n) in [
        ("RTERM321", "TERM.E.U"),
        ("RTERM010", "TERM.E.U"),
        ("RTERM323", "TERM.E.D"),
        ("RTERM210", "TERM.E.D"),
        ("RTERM323_PCI", "TERM.E.U"),
        ("RTERM210_PCI", "TERM.E.U"),
        ("CNR_RTERM", "TERM.E"),
    ] {
        builder.extract_term(
            "TERM.E",
            Some((tslots::TERM_H, bels::TERM_E, "TERM.E")),
            Dir::E,
            tkn,
            n,
        );
    }
    for tkn in [
        "BTERM010",
        "BTERM123",
        "BTERM012",
        "BTERM323",
        "BTERM123_TBS",
        "BTERM012_TBS",
        "BCLKTERM123",
        "BCLKTERM012",
        "ML_BCLKTERM123",
        "ML_BCLKTERM012",
        "ML_BCLKTERM123_MK",
        "BBTERM",
        "BGIGABIT_IOI_TERM",
        "BGIGABIT10_IOI_TERM",
        "BGIGABIT_INT_TERM",
        "BGIGABIT10_INT_TERM",
    ] {
        builder.extract_term(
            "TERM.S",
            Some((tslots::TERM_V, bels::TERM_S, "TERM.S")),
            Dir::S,
            tkn,
            "TERM.S",
        );
    }
    builder.extract_term(
        "TERM.S",
        Some((tslots::TERM_V, bels::TERM_S, "TERM.S")),
        Dir::S,
        "CNR_BTERM",
        "TERM.S.CNR",
    );
    builder.extract_term(
        "TERM.S",
        Some((tslots::TERM_V, bels::TERM_S, "TERM.S")),
        Dir::S,
        "ML_CNR_BTERM",
        "TERM.S.CNR",
    );
    for tkn in [
        "TTERM321",
        "TTERM010",
        "TTERM323",
        "TTERM210",
        "TTERM321_TBS",
        "TTERM210_TBS",
        "TCLKTERM321",
        "TCLKTERM210",
        "ML_TTERM010",
        "ML_TCLKTERM210",
        "ML_TCLKTERM210_MK",
        "BTTERM",
        "TGIGABIT_IOI_TERM",
        "TGIGABIT10_IOI_TERM",
        "TGIGABIT_INT_TERM",
        "TGIGABIT10_INT_TERM",
    ] {
        builder.extract_term(
            "TERM.N",
            Some((tslots::TERM_V, bels::TERM_S, "TERM.N")),
            Dir::N,
            tkn,
            "TERM.N",
        );
    }
    builder.extract_term(
        "TERM.N",
        Some((tslots::TERM_V, bels::TERM_S, "TERM.N")),
        Dir::N,
        "CNR_TTERM",
        "TERM.N.CNR",
    );

    for &xy_b in rd.tiles_by_kind_name("PTERMB") {
        let xy_t = xy_b.delta(0, 14);
        let int_s_xy = builder.walk_to_int(xy_b, Dir::S, false).unwrap();
        let int_n_xy = builder.walk_to_int(xy_t, Dir::N, false).unwrap();
        builder.extract_pass_tile(
            "PPC.S",
            Dir::S,
            int_n_xy,
            Some(xy_t),
            Some(xy_b),
            Some("PPC.S"),
            Some((tslots::TERM_V, bels::PPC_TERM_S, "PPC.S", "PPC.S")),
            None,
            int_s_xy,
            &[],
        );
        builder.extract_pass_tile(
            "PPC.N",
            Dir::N,
            int_s_xy,
            Some(xy_b),
            Some(xy_t),
            Some("PPC.N"),
            Some((tslots::TERM_V, bels::PPC_TERM_N, "PPC.N", "PPC.N")),
            None,
            int_n_xy,
            &[],
        );
    }
    for tkn in ["PTERMR", "PTERMBR", "PTERMTR"] {
        for &xy_r in rd.tiles_by_kind_name(tkn) {
            let int_w_xy = builder.walk_to_int(xy_r, Dir::W, false).unwrap();
            let int_e_xy = builder.walk_to_int(xy_r, Dir::E, false).unwrap();
            builder.extract_pass_tile(
                "PPC.W",
                Dir::W,
                int_e_xy,
                Some(xy_r),
                Some(int_w_xy),
                Some("PPC.W"),
                Some((tslots::TERM_H, bels::PPC_TERM_W, "PPC.W", "PPC.W")),
                None,
                int_w_xy,
                &[],
            );
            builder.extract_pass_tile(
                "PPC.E",
                Dir::E,
                int_w_xy,
                Some(int_w_xy),
                Some(xy_r),
                Some("PPC.E"),
                Some((tslots::TERM_H, bels::PPC_TERM_E, "PPC.E", "PPC.E")),
                None,
                int_e_xy,
                &[],
            );
        }
    }

    for (tkn, name, naming) in [
        ("BGIGABIT_INT0", "INTF.GT.B0", "INTF.GT"),
        ("BGIGABIT_INT1", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT_INT2", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT_INT3", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT_INT4", "INTF.GT.BCLKPAD", "INTF.GT.CLKPAD"),
        ("TGIGABIT_INT0", "INTF.GT.T0", "INTF.GT"),
        ("TGIGABIT_INT1", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT_INT2", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT_INT3", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT_INT4", "INTF.GT.TCLKPAD", "INTF.GT.CLKPAD"),
        ("BGIGABIT10_INT0", "INTF.GT.B0", "INTF.GT"),
        ("BGIGABIT10_INT1", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT10_INT2", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT10_INT3", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT10_INT4", "INTF.GT.B0", "INTF.GT"),
        ("BGIGABIT10_INT5", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT10_INT6", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT10_INT7", "INTF.GT.B123", "INTF.GT"),
        ("BGIGABIT10_INT8", "INTF.GT.BCLKPAD", "INTF.GT.CLKPAD"),
        ("TGIGABIT10_INT0", "INTF.GT.T0", "INTF.GT"),
        ("TGIGABIT10_INT1", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT10_INT2", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT10_INT3", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT10_INT4", "INTF.GT.T0", "INTF.GT"),
        ("TGIGABIT10_INT5", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT10_INT6", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT10_INT7", "INTF.GT.T123", "INTF.GT"),
        ("TGIGABIT10_INT8", "INTF.GT.TCLKPAD", "INTF.GT.CLKPAD"),
        ("LPPC_X0Y0_INT", "INTF.PPC", "INTF.PPC.L"),
        ("LPPC_X1Y0_INT", "INTF.PPC", "INTF.PPC.L"),
        ("LLPPC_X0Y0_INT", "INTF.PPC", "INTF.PPC.L"),
        ("LLPPC_X1Y0_INT", "INTF.PPC", "INTF.PPC.L"),
        ("ULPPC_X0Y0_INT", "INTF.PPC", "INTF.PPC.L"),
        ("ULPPC_X1Y0_INT", "INTF.PPC", "INTF.PPC.L"),
        ("RPPC_X0Y0_INT", "INTF.PPC", "INTF.PPC.R"),
        ("RPPC_X1Y0_INT", "INTF.PPC", "INTF.PPC.R"),
        ("BPPC_X0Y0_INT", "INTF.PPC", "INTF.PPC.B"),
        ("BPPC_X1Y0_INT", "INTF.PPC", "INTF.PPC.B"),
        ("TPPC_X0Y0_INT", "INTF.PPC", "INTF.PPC.T"),
        ("TPPC_X1Y0_INT", "INTF.PPC", "INTF.PPC.T"),
    ] {
        builder.extract_intf(tslots::INTF, name, Dir::E, tkn, naming, false, None);
    }

    for (nn, tkn) in [
        ("CLKB.V2", "CLKB"),
        ("CLKB.V2P", "ML_CLKB"),
        ("CLKB.V2PX", "MK_CLKB"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            let xy_r = xy.delta(1, 0);
            let mut bels = vec![
                builder
                    .bel_indexed(bels::BUFGMUX0, "BUFGMUX", 0)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR0", "ML_CLKB_CKIR0"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL4"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR0"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL0"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR0"])
                    .extra_int_in("CLK", &["CLKB_GCLK00"]),
                builder
                    .bel_indexed(bels::BUFGMUX1, "BUFGMUX", 1)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR1"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL5"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR1"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL1"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR1"])
                    .extra_int_in("CLK", &["CLKB_GCLK01"]),
                builder
                    .bel_indexed(bels::BUFGMUX2, "BUFGMUX", 2)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR2"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL6"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR2"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL2"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR2"])
                    .extra_int_in("CLK", &["CLKB_GCLK02"]),
                builder
                    .bel_indexed(bels::BUFGMUX3, "BUFGMUX", 3)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR3"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL7"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR3"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL3"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR3"])
                    .extra_int_in("CLK", &["CLKB_GCLK03"]),
                builder
                    .bel_indexed(bels::BUFGMUX4, "BUFGMUX", 4)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL0"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL0"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR4"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL4"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR4"])
                    .extra_int_in("CLK", &["CLKB_GCLK04"]),
                builder
                    .bel_indexed(bels::BUFGMUX5, "BUFGMUX", 5)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL1"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL1"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR5"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL5"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR5"])
                    .extra_int_in("CLK", &["CLKB_GCLK05"]),
                builder
                    .bel_indexed(bels::BUFGMUX6, "BUFGMUX", 6)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL2", "ML_CLKB_CKIL2"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL2"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR6"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL6"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR6"])
                    .extra_int_in("CLK", &["CLKB_GCLK06"]),
                builder
                    .bel_indexed(bels::BUFGMUX7, "BUFGMUX", 7)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL3"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL3"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR7"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL7"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR7"])
                    .extra_int_in("CLK", &["CLKB_GCLK07"]),
                builder.bel_virtual(bels::GLOBALSIG_S0),
                builder.bel_virtual(bels::GLOBALSIG_S1),
            ];
            if tkn == "ML_CLKB" {
                bels.push(
                    builder
                        .bel_virtual(bels::BREFCLK)
                        .extra_wire("BREFCLK", &["ML_CLKB_BREFCLK"])
                        .extra_wire("BREFCLK2", &["ML_CLKB_BREFCLK2"]),
                );
            }
            builder.extract_xnode(
                tslots::CLK,
                bels::CLK_INT,
                nn,
                xy,
                &[],
                &[xy_l, xy_r],
                nn,
                &bels,
                &[],
            );
        }
    }
    for (nn, tkn) in [
        ("CLKT.V2", "CLKT"),
        ("CLKT.V2P", "ML_CLKT"),
        ("CLKT.V2PX", "MK_CLKT"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            let xy_r = xy.delta(1, 0);
            let mut bels = vec![
                builder
                    .bel_indexed(bels::BUFGMUX0, "BUFGMUX", 0)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR0"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL4"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR0"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL0"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR0"])
                    .extra_int_in("CLK", &["CLKT_GCLK00"]),
                builder
                    .bel_indexed(bels::BUFGMUX1, "BUFGMUX", 1)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR1"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL5"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR1"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL1"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR1"])
                    .extra_int_in("CLK", &["CLKT_GCLK01"]),
                builder
                    .bel_indexed(bels::BUFGMUX2, "BUFGMUX", 2)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR2", "ML_CLKT_CKIR2"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL6"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR2"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL2"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR2"])
                    .extra_int_in("CLK", &["CLKT_GCLK02"]),
                builder
                    .bel_indexed(bels::BUFGMUX3, "BUFGMUX", 3)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR3"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL7"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR3"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL3"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR3"])
                    .extra_int_in("CLK", &["CLKT_GCLK03"]),
                builder
                    .bel_indexed(bels::BUFGMUX4, "BUFGMUX", 4)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL0", "ML_CLKT_CKIL0"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL0"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR4"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL4"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR4"])
                    .extra_int_in("CLK", &["CLKT_GCLK04"]),
                builder
                    .bel_indexed(bels::BUFGMUX5, "BUFGMUX", 5)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL1"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL1"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR5"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL5"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR5"])
                    .extra_int_in("CLK", &["CLKT_GCLK05"]),
                builder
                    .bel_indexed(bels::BUFGMUX6, "BUFGMUX", 6)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL2"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL2"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR6"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL6"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR6"])
                    .extra_int_in("CLK", &["CLKT_GCLK06"]),
                builder
                    .bel_indexed(bels::BUFGMUX7, "BUFGMUX", 7)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL3"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL3"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR7"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL7"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR7"])
                    .extra_int_in("CLK", &["CLKT_GCLK07"]),
                builder.bel_virtual(bels::GLOBALSIG_N0),
                builder.bel_virtual(bels::GLOBALSIG_N1),
            ];
            if tkn == "ML_CLKT" {
                bels.push(
                    builder
                        .bel_virtual(bels::BREFCLK)
                        .extra_wire("BREFCLK", &["ML_CLKT_BREFCLK"])
                        .extra_wire("BREFCLK2", &["ML_CLKT_BREFCLK2"]),
                );
            }
            builder.extract_xnode(
                tslots::CLK,
                bels::CLK_INT,
                nn,
                xy,
                &[],
                &[xy_l, xy_r],
                nn,
                &bels,
                &[],
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC") {
        let mut bel = builder.bel_virtual(bels::CLKC);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_B{i}"), &[format!("CLKC_GCLKB_IN{i}")])
                .extra_wire(format!("IN_T{i}"), &[format!("CLKC_GCLKT_IN{i}")])
                .extra_wire(format!("OUT_B{i}"), &[format!("CLKC_GCLKB{i}")])
                .extra_wire(format!("OUT_T{i}"), &[format!("CLKC_GCLKT{i}")]);
        }
        builder.extract_xnode_bels(tslots::CLK, "CLKC", xy, &[], &[], "CLKC", &[bel]);
    }

    for &xy in rd.tiles_by_kind_name("GCLKC") {
        for nn in ["GCLKC", "GCLKC.B", "GCLKC.T"] {
            let mut bel = builder.bel_virtual(bels::GCLKC);
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN_B{i}"), &[format!("GCLKC_GCLKB{i}")])
                    .extra_wire(format!("IN_T{i}"), &[format!("GCLKC_GCLKT{i}")])
                    .extra_wire(format!("OUT_L{i}"), &[format!("GCLKC_GCLKL{i}")])
                    .extra_wire(format!("OUT_R{i}"), &[format!("GCLKC_GCLKR{i}")]);
            }
            builder.extract_xnode_bels(tslots::HROW, nn, xy, &[], &[], "GCLKC", &[bel]);
        }
    }

    for tkn in ["GCLKH", "LR_GCLKH"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_s_xy = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let int_n_xy = builder.walk_to_int(xy, Dir::N, false).unwrap();
            let mut bel = builder.bel_virtual(bels::GCLKH);
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN{i}"), &[format!("GCLKH_GCLK_B{i}")])
                    .extra_int_out(format!("OUT_T{i}"), &[format!("GCLKH_GCLK_UP{i}")])
                    .extra_int_out(format!("OUT_B{i}"), &[format!("GCLKH_GCLK_DN{i}")]);
            }
            builder.extract_xnode_bels(
                tslots::HCLK,
                "GCLKH",
                xy,
                &[],
                &[int_s_xy, int_n_xy],
                "GCLKH",
                &[builder.bel_virtual(bels::GLOBALSIG), bel],
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("BRAMSITE") {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xnode_bels(
            tslots::BEL,
            "BRAM",
            xy,
            &[],
            &int_xy,
            "BRAM",
            &[
                builder.bel_xy(bels::BRAM, "RAMB16", 0, 0),
                builder.bel_xy(bels::MULT, "MULT18X18", 0, 0),
            ],
        );
    }

    for (tkn, kind) in [("BBTERM", "DCMCONN.BOT"), ("BTTERM", "DCMCONN.TOP")] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = [xy.delta(0, if kind == "DCMCONN.BOT" { 1 } else { -1 })];
            builder.extract_xnode_bels(
                tslots::CLK,
                kind,
                xy,
                &[],
                &int_xy,
                kind,
                &[builder
                    .bel_virtual(bels::DCMCONN)
                    .extra_wire("OUTBUS0", &["BBTERM_CKMUX0", "TTERM_CKMUX0"])
                    .extra_wire("OUTBUS1", &["BBTERM_CKMUX1", "TTERM_CKMUX1"])
                    .extra_wire("OUTBUS2", &["BBTERM_CKMUX2", "TTERM_CKMUX2"])
                    .extra_wire("OUTBUS3", &["BBTERM_CKMUX3", "TTERM_CKMUX3"])
                    .extra_wire("OUTBUS4", &["BBTERM_CKMUX4", "TTERM_CKMUX4"])
                    .extra_wire("OUTBUS5", &["BBTERM_CKMUX5", "TTERM_CKMUX5"])
                    .extra_wire("OUTBUS6", &["BBTERM_CKMUX6", "TTERM_CKMUX6"])
                    .extra_wire("OUTBUS7", &["BBTERM_CKMUX7", "TTERM_CKMUX7"])
                    .extra_wire("CLKPADBUS0", &["BBTERM_DLL_CLKPAD0", "BTTERM_DLL_CLKPAD0"])
                    .extra_wire("CLKPADBUS1", &["BBTERM_DLL_CLKPAD1", "BTTERM_DLL_CLKPAD1"])
                    .extra_wire("CLKPADBUS2", &["BBTERM_DLL_CLKPAD2", "BTTERM_DLL_CLKPAD2"])
                    .extra_wire("CLKPADBUS3", &["BBTERM_DLL_CLKPAD3", "BTTERM_DLL_CLKPAD3"])
                    .extra_wire("CLKPADBUS4", &["BBTERM_DLL_CLKPAD4", "BTTERM_DLL_CLKPAD4"])
                    .extra_wire("CLKPADBUS5", &["BBTERM_DLL_CLKPAD5", "BTTERM_DLL_CLKPAD5"])
                    .extra_wire("CLKPADBUS6", &["BBTERM_DLL_CLKPAD6", "BTTERM_DLL_CLKPAD6"])
                    .extra_wire("CLKPADBUS7", &["BBTERM_DLL_CLKPAD7", "BTTERM_DLL_CLKPAD7"])
                    .extra_int_out("CLKPAD0", &["BBTERM_CLKPAD0", "BTTERM_CLKPAD0"])
                    .extra_int_out("CLKPAD1", &["BBTERM_CLKPAD1", "BTTERM_CLKPAD1"])
                    .extra_int_out("CLKPAD2", &["BBTERM_CLKPAD2", "BTTERM_CLKPAD2"])
                    .extra_int_out("CLKPAD3", &["BBTERM_CLKPAD3", "BTTERM_CLKPAD3"])
                    .extra_int_out("CLKPAD4", &["BBTERM_CLKPAD4", "BTTERM_CLKPAD4"])
                    .extra_int_out("CLKPAD5", &["BBTERM_CLKPAD5", "BTTERM_CLKPAD5"])
                    .extra_int_out("CLKPAD6", &["BBTERM_CLKPAD6", "BTTERM_CLKPAD6"])
                    .extra_int_out("CLKPAD7", &["BBTERM_CLKPAD7", "BTTERM_CLKPAD7"])
                    .extra_int_in("OUT0", &["BTERM_OMUX0", "BTTERM_OMUX10"])
                    .extra_int_in("OUT1", &["BTERM_OMUX3", "BTTERM_OMUX11"])
                    .extra_int_in("OUT2", &["BTERM_OMUX4", "BTTERM_OMUX12"])
                    .extra_int_in("OUT3", &["BTERM_OMUX5", "BTTERM_OMUX15"])],
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("BGIGABIT") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, -1));
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xnode_bels(
            tslots::BEL,
            "GIGABIT.B",
            xy,
            &[],
            &int_xy,
            "GIGABIT.B",
            &[
                builder
                    .bel_xy(bels::GT, "GT", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN", "TST10B8BICRD0", "TST10B8BICRD1"])
                    .pin_name_only("BREFCLK", 1)
                    .pin_name_only("BREFCLK2", 1),
                builder
                    .bel_indexed(bels::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(bels::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
        );
    }

    for &xy in rd.tiles_by_kind_name("TGIGABIT") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, 4));
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xnode_bels(
            tslots::BEL,
            "GIGABIT.T",
            xy,
            &[],
            &int_xy,
            "GIGABIT.T",
            &[
                builder
                    .bel_xy(bels::GT, "GT", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN", "TST10B8BICRD0", "TST10B8BICRD1"])
                    .pin_name_only("BREFCLK", 1)
                    .pin_name_only("BREFCLK2", 1),
                builder
                    .bel_indexed(bels::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(bels::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
        );
    }

    for &xy in rd.tiles_by_kind_name("BGIGABIT10") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, -1));
        for dy in [0, 1, 2, 3, 5, 6, 7, 8] {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xnode_bels(
            tslots::BEL,
            "GIGABIT10.B",
            xy,
            &[],
            &int_xy,
            "GIGABIT10.B",
            &[
                builder
                    .bel_xy(bels::GT10, "GT10", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN"])
                    .pin_name_only("BREFCLKPIN", 1)
                    .pin_name_only("BREFCLKNIN", 1),
                builder
                    .bel_indexed(bels::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(bels::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
        );
    }

    for &xy in rd.tiles_by_kind_name("TGIGABIT10") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, 9));
        for dy in [0, 1, 2, 3, 5, 6, 7, 8] {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xnode_bels(
            tslots::BEL,
            "GIGABIT10.T",
            xy,
            &[],
            &int_xy,
            "GIGABIT10.T",
            &[
                builder
                    .bel_xy(bels::GT10, "GT10", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN"])
                    .pin_name_only("BREFCLKPIN", 1)
                    .pin_name_only("BREFCLKNIN", 1),
                builder
                    .bel_indexed(bels::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(bels::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(bels::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
        );
    }

    for tkn in ["LBPPC", "RBPPC"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut int_xy = Vec::new();
            for dy in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16] {
                int_xy.push(xy.delta(-6, -9 + dy));
            }
            for dy in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16] {
                int_xy.push(xy.delta(5, -9 + dy));
            }
            for dx in [0, 2, 3, 4, 5, 6, 7, 8] {
                int_xy.push(xy.delta(-5 + dx, -9));
            }
            for dx in [0, 2, 3, 4, 5, 6, 7, 8] {
                int_xy.push(xy.delta(-5 + dx, 7));
            }
            builder.extract_xnode_bels(
                tslots::BEL,
                tkn,
                xy,
                &[],
                &int_xy,
                tkn,
                &[builder.bel_xy(bels::PPC405, "PPC405", 0, 0)],
            );
        }
    }

    for tkn in ["REG_L", "REG_R"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_o = xy.delta(if xy.x == 0 { 1 } else { -1 }, 0);
            let int_s_xy = builder.walk_to_int(xy_o, Dir::S, false).unwrap();
            let int_n_xy = builder.walk_to_int(xy_o, Dir::N, false).unwrap();
            let int_xy = [
                int_s_xy.delta(0, -1),
                int_s_xy,
                int_n_xy,
                int_n_xy.delta(0, 1),
            ];
            let buf_xy = [
                Coord {
                    x: xy.x,
                    y: int_s_xy.y,
                },
                Coord {
                    x: xy.x,
                    y: int_n_xy.y,
                },
            ];
            builder.extract_xnode_bels(
                tslots::CLK,
                tkn,
                xy,
                &buf_xy,
                &int_xy,
                tkn,
                &[builder.bel_xy(bels::PCILOGIC, "PCILOGIC", 0, 0)],
            );
        }
    }

    builder.build()
}
