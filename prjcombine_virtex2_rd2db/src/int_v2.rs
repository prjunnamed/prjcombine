use prjcombine_int::db::{Dir, IntDb, WireKind};
use prjcombine_rawdump::{Coord, Part};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("virtex2", rd);

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
            WireKind::ClkOut(i),
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
                WireKind::MultiBranch(Dir::W),
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
                WireKind::MultiBranch(Dir::S),
                &[format!("LV{i}")],
            )
        })
        .collect();
    for i in 0..24 {
        builder.conn_branch(lv[i], Dir::N, lv[(i + 23) % 24]);
    }

    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[
                format!("CLK{i}"),
                format!("IOIS_CK{j}_B{k}", j = [2, 1, 2, 1][i], k = [1, 1, 3, 3][i]),
                format!("BRAM_CLK{i}"),
                ["BRAM_IOIS_CLKFB", "BRAM_IOIS_CLKIN", "BRAM_IOIS_PSCLK", ""][i].to_string(),
                format!("CNR_CLK{i}"),
                format!("LRPPC_INT_CLK{i}"),
                format!("BPPC_INT_CLK{i}"),
                format!("TPPC_INT_CLK{i}"),
                format!("GIGABIT_INT_CLK{i}"),
            ],
        );
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
                format!("IOIS_CK{j}_B0", j = [2, 1][i]),
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
                format!("IOIS_CK{j}_B2", j = [2, 1][i]),
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

    let bels_int = [builder.bel_xy("RLL", "RLL", 0, 0)];
    let bels_int_sigh = [builder
        .bel_xy("RLL", "RLL", 0, 0)
        .pins_name_only(&["LH0", "LH6", "LH12", "LH18", "LV0", "LV6", "LV12", "LV18"])];
    builder.extract_node("CENTER", "INT.CLB", "INT.CLB", &bels_int);
    builder.extract_node("LR_IOIS", "INT.IOI", "INT.IOI.LR", &bels_int);
    builder.extract_node("TB_IOIS", "INT.IOI", "INT.IOI.TB", &bels_int);
    builder.extract_node("ML_TB_IOIS", "INT.IOI", "INT.IOI.TB", &bels_int);
    builder.extract_node("ML_TBS_IOIS", "INT.IOI", "INT.IOI.TB", &bels_int);
    builder.extract_node("GIGABIT_IOI", "INT.IOI", "INT.IOI.TB", &bels_int);
    builder.extract_node("GIGABIT10_IOI", "INT.IOI", "INT.IOI.TB", &bels_int);
    builder.extract_node("MK_B_IOIS", "INT.IOI.CLK_B", "INT.IOI.CLK_B", &bels_int);
    builder.extract_node("MK_T_IOIS", "INT.IOI.CLK_T", "INT.IOI.CLK_T", &bels_int);
    builder.extract_node("BRAM0", "INT.BRAM", "INT.BRAM", &bels_int);
    builder.extract_node("BRAM1", "INT.BRAM", "INT.BRAM", &bels_int);
    builder.extract_node("BRAM2", "INT.BRAM", "INT.BRAM", &bels_int);
    builder.extract_node("BRAM3", "INT.BRAM", "INT.BRAM", &bels_int);
    builder.extract_node("BRAM_IOIS", "INT.DCM.V2", "INT.BRAM_IOIS", &bels_int);
    builder.extract_node(
        "ML_BRAM_IOIS",
        "INT.DCM.V2P",
        "INT.ML_BRAM_IOIS",
        &bels_int_sigh,
    );
    builder.extract_node("LL", "INT.CNR", "INT.CNR", &bels_int);
    builder.extract_node("LR", "INT.CNR", "INT.CNR", &bels_int);
    builder.extract_node("UL", "INT.CNR", "INT.CNR", &bels_int);
    builder.extract_node("UR", "INT.CNR", "INT.CNR", &bels_int);
    builder.extract_node("BGIGABIT_INT0", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT_INT1", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT_INT2", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT_INT3", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT_INT4", "INT.GT.CLKPAD", "INT.GT.CLKPAD", &bels_int);
    builder.extract_node("TGIGABIT_INT0", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT_INT1", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT_INT2", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT_INT3", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT_INT4", "INT.GT.CLKPAD", "INT.GT.CLKPAD", &bels_int);
    builder.extract_node("BGIGABIT10_INT0", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT1", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT2", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT3", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT4", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT5", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT6", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("BGIGABIT10_INT7", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node(
        "BGIGABIT10_INT8",
        "INT.GT.CLKPAD",
        "INT.GT.CLKPAD",
        &bels_int,
    );
    builder.extract_node("TGIGABIT10_INT0", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT1", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT2", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT3", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT4", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT5", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT6", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node("TGIGABIT10_INT7", "INT.PPC", "INT.GT", &bels_int);
    builder.extract_node(
        "TGIGABIT10_INT8",
        "INT.GT.CLKPAD",
        "INT.GT.CLKPAD",
        &bels_int,
    );
    builder.extract_node("LPPC_X0Y0_INT", "INT.PPC", "INT.PPC.L", &bels_int);
    builder.extract_node("LPPC_X1Y0_INT", "INT.PPC", "INT.PPC.L", &bels_int);
    builder.extract_node("LLPPC_X0Y0_INT", "INT.PPC", "INT.PPC.L", &bels_int);
    builder.extract_node("LLPPC_X1Y0_INT", "INT.PPC", "INT.PPC.L", &bels_int);
    builder.extract_node("ULPPC_X0Y0_INT", "INT.PPC", "INT.PPC.L", &bels_int);
    builder.extract_node("ULPPC_X1Y0_INT", "INT.PPC", "INT.PPC.L", &bels_int);
    builder.extract_node("RPPC_X0Y0_INT", "INT.PPC", "INT.PPC.R", &bels_int);
    builder.extract_node("RPPC_X1Y0_INT", "INT.PPC", "INT.PPC.R", &bels_int);
    builder.extract_node("BPPC_X0Y0_INT", "INT.PPC", "INT.PPC.B", &bels_int);
    builder.extract_node("BPPC_X1Y0_INT", "INT.PPC", "INT.PPC.B", &bels_int);
    builder.extract_node("TPPC_X0Y0_INT", "INT.PPC", "INT.PPC.T", &bels_int);
    builder.extract_node("TPPC_X1Y0_INT", "INT.PPC", "INT.PPC.T", &bels_int);

    let slice_name_only = [
        "DX", "DY", "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG",
        "DIG", "SLICEWE0", "SLICEWE1", "SLICEWE2", "BXOUT", "BYOUT", "BYINVOUT", "SOPIN", "SOPOUT",
    ];

    let bels_clb = [
        builder
            .bel_xy("SLICE0", "SLICE", 0, 0)
            .pins_name_only(&slice_name_only)
            .extra_wire("BYINVOUT_LOCAL", &["BYINVOUT_LOCAL0"]),
        builder
            .bel_xy("SLICE1", "SLICE", 0, 1)
            .pins_name_only(&slice_name_only)
            .extra_wire("FX_S", &["FX_S1"])
            .extra_wire("COUT_N", &["COUT_N3"])
            .extra_wire("BYOUT_LOCAL", &["BYOUT_LOCAL1"])
            .extra_wire("BYINVOUT_LOCAL", &["BYINVOUT_LOCAL1"]),
        builder
            .bel_xy("SLICE2", "SLICE", 1, 0)
            .pins_name_only(&slice_name_only)
            .extra_wire("SOPOUT_W", &["SOPOUT_W2"]),
        builder
            .bel_xy("SLICE3", "SLICE", 1, 1)
            .pins_name_only(&slice_name_only)
            .extra_wire("COUT_N", &["COUT_N1"])
            .extra_wire("DIG_LOCAL", &["DIG_LOCAL3"])
            .extra_wire("DIG_S", &["DIG_S3"])
            .extra_wire("SOPOUT_W", &["SOPOUT_W3"]),
        builder
            .bel_indexed("TBUF0", "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed("TBUF1", "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual("TBUS")
            .extra_wire("BUS0", &["TBUF0"])
            .extra_wire("BUS1", &["TBUF1"])
            .extra_wire("BUS2", &["TBUF2"])
            .extra_wire("BUS3", &["TBUF3"])
            .extra_wire("BUS3_E", &["TBUF3_E"])
            .extra_int_out("OUT", &["TBUS"]),
    ];
    builder.extract_node_bels("CENTER", "CLB", "CLB", &bels_clb);

    let ioi_name_only = ["DIFFI_IN", "PADOUT", "DIFFO_IN", "DIFFO_OUT"];
    let bels_ioi = [
        builder
            .bel_indexed("IOI0", "IOB", 0)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF0"),
        builder
            .bel_indexed("IOI1", "IOB", 1)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF1"),
        builder
            .bel_indexed("IOI2", "IOB", 2)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF2"),
        builder
            .bel_indexed("IOI3", "IOB", 3)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF3"),
    ];
    builder.extract_node_bels("LR_IOIS", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels("TB_IOIS", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels("ML_TB_IOIS", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels("ML_TBS_IOIS", "IOI", "IOI.TBS", &bels_ioi);
    builder.extract_node_bels("GIGABIT_IOI", "IOI", "IOI", &bels_ioi);
    builder.extract_node_bels("GIGABIT10_IOI", "IOI", "IOI", &bels_ioi);
    if rd.part.contains("vpx") {
        let bels_ioi_clk_b = [
            bels_ioi[0].clone(),
            bels_ioi[1].clone(),
            builder
                .bel_single("CLK_P", "CLKPPAD2")
                .pin_name_only("I", 1),
            builder
                .bel_single("CLK_N", "CLKNPAD2")
                .pin_name_only("I", 1),
            builder
                .bel_virtual("BREFCLK_INT")
                .extra_int_out("BREFCLK", &["IOIS_BREFCLK_SE"]),
        ];
        let bels_ioi_clk_t = [
            builder
                .bel_single("CLK_P", "CLKPPAD1")
                .pin_name_only("I", 1),
            builder
                .bel_single("CLK_N", "CLKNPAD1")
                .pin_name_only("I", 1),
            bels_ioi[2].clone(),
            bels_ioi[3].clone(),
            builder
                .bel_virtual("BREFCLK_INT")
                .extra_int_out("BREFCLK", &["IOIS_BREFCLK_SE"]),
        ];
        builder.extract_node_bels("MK_B_IOIS", "IOI.CLK_B", "IOI.CLK_B", &bels_ioi_clk_b);
        builder.extract_node_bels("MK_T_IOIS", "IOI.CLK_T", "IOI.CLK_T", &bels_ioi_clk_t);
    }

    for (kind, num) in [
        ("IOBS.B.L2", 2),
        ("IOBS.B.R2", 2),
        ("IOBS.B.R2.CLK", 2),
        ("IOBS.B.L1", 1),
        ("IOBS.B.R1", 1),
        ("IOBS.T.L2", 2),
        ("IOBS.T.R2", 2),
        ("IOBS.T.R2.CLK", 2),
        ("IOBS.T.L1", 1),
        ("IOBS.T.R1", 1),
        ("IOBS.L.B2", 2),
        ("IOBS.L.T2", 2),
        ("IOBS.R.B2", 2),
        ("IOBS.R.T2", 2),
    ] {
        builder.make_marker_bel(kind, kind, kind, num);
    }

    let bels_dcm = [builder.bel_xy("DCM", "DCM", 0, 0)];
    builder.extract_node_bels("BRAM_IOIS", "DCM.V2", "DCM.V2", &bels_dcm);
    builder.extract_node_bels("ML_BRAM_IOIS", "DCM.V2P", "DCM.V2P", &bels_dcm);

    builder.extract_node_bels(
        "LL",
        "DCI",
        "DCI",
        &[
            builder.bel_indexed("DCI0", "DCI", 6),
            builder.bel_indexed("DCI1", "DCI", 5),
        ],
    );
    builder.extract_node_bels(
        "LR",
        "DCI",
        "DCI",
        &[
            builder.bel_indexed("DCI0", "DCI", 3),
            builder.bel_indexed("DCI1", "DCI", 4),
        ],
    );
    builder.extract_node_bels(
        "UL",
        "DCI",
        "DCI",
        &[
            builder.bel_indexed("DCI0", "DCI", 7),
            builder.bel_indexed("DCI1", "DCI", 0),
        ],
    );
    builder.extract_node_bels(
        "UR",
        "DCI",
        "DCI",
        &[
            builder.bel_indexed("DCI0", "DCI", 2),
            builder.bel_indexed("DCI1", "DCI", 1),
        ],
    );

    builder.extract_node_bels(
        "LR",
        "LR",
        "LR",
        &[
            builder.bel_single("STARTUP", "STARTUP"),
            builder.bel_single("CAPTURE", "CAPTURE"),
            builder.bel_single("ICAP", "ICAP"),
        ],
    );
    builder.extract_node_bels("UL", "PMV", "PMV", &[builder.bel_single("PMV", "PMV")]);
    builder.extract_node_bels(
        "UR",
        "BSCAN",
        "BSCAN",
        &[builder.bel_single("BSCAN", "BSCAN")],
    );
    if rd.family == "virtex2p" {
        builder.extract_node_bels(
            "UR",
            "JTAGPPC",
            "JTAGPPC",
            &[builder.bel_single("JTAGPPC", "JTAGPPC")],
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
        builder.extract_term("TERM.W", Some("TERM.W"), Dir::W, tkn, n);
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
        builder.extract_term("TERM.E", Some("TERM.E"), Dir::E, tkn, n);
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
        builder.extract_term("TERM.S", Some("TERM.S"), Dir::S, tkn, "TERM.S");
    }
    builder.extract_term("TERM.S", Some("TERM.S"), Dir::S, "CNR_BTERM", "TERM.S.CNR");
    builder.extract_term(
        "TERM.S",
        Some("TERM.S"),
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
        builder.extract_term("TERM.N", Some("TERM.N"), Dir::N, tkn, "TERM.N");
    }
    builder.extract_term("TERM.N", Some("TERM.N"), Dir::N, "CNR_TTERM", "TERM.N.CNR");

    for &xy_b in rd.tiles_by_kind_name("PTERMB") {
        let xy_t = xy_b.delta(0, 14);
        let int_s_xy = builder.walk_to_int(xy_b, Dir::S).unwrap();
        let int_n_xy = builder.walk_to_int(xy_t, Dir::N).unwrap();
        builder.extract_pass_tile(
            "PPC.S",
            Dir::S,
            int_n_xy,
            Some(xy_t),
            Some(xy_b),
            Some("PPC.S"),
            Some(("PPC.S", "PPC.S")),
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
            Some(("PPC.N", "PPC.N")),
            int_n_xy,
            &[],
        );
    }
    for tkn in ["PTERMR", "PTERMBR", "PTERMTR"] {
        for &xy_r in rd.tiles_by_kind_name(tkn) {
            let int_w_xy = builder.walk_to_int(xy_r, Dir::W).unwrap();
            let int_e_xy = builder.walk_to_int(xy_r, Dir::E).unwrap();
            builder.extract_pass_tile(
                "PPC.W",
                Dir::W,
                int_e_xy,
                Some(xy_r),
                Some(int_w_xy),
                Some("PPC.W"),
                Some(("PPC.W", "PPC.W")),
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
                Some(("PPC.E", "PPC.E")),
                int_e_xy,
                &[],
            );
        }
    }

    for (tkn, name, naming) in [
        ("BGIGABIT_INT0", "INTF.GT.0", "INTF.GT"),
        ("BGIGABIT_INT1", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT_INT2", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT_INT3", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT_INT4", "INTF.GT.CLKPAD", "INTF.GT.CLKPAD"),
        ("TGIGABIT_INT0", "INTF.GT.0", "INTF.GT"),
        ("TGIGABIT_INT1", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT_INT2", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT_INT3", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT_INT4", "INTF.GT.CLKPAD", "INTF.GT.CLKPAD"),
        ("BGIGABIT10_INT0", "INTF.GT.0", "INTF.GT"),
        ("BGIGABIT10_INT1", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT10_INT2", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT10_INT3", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT10_INT4", "INTF.GT.0", "INTF.GT"),
        ("BGIGABIT10_INT5", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT10_INT6", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT10_INT7", "INTF.GT.123", "INTF.GT"),
        ("BGIGABIT10_INT8", "INTF.GT.CLKPAD", "INTF.GT.CLKPAD"),
        ("TGIGABIT10_INT0", "INTF.GT.0", "INTF.GT"),
        ("TGIGABIT10_INT1", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT10_INT2", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT10_INT3", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT10_INT4", "INTF.GT.0", "INTF.GT"),
        ("TGIGABIT10_INT5", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT10_INT6", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT10_INT7", "INTF.GT.123", "INTF.GT"),
        ("TGIGABIT10_INT8", "INTF.GT.CLKPAD", "INTF.GT.CLKPAD"),
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
        builder.extract_intf(name, Dir::E, tkn, naming, false);
    }

    for tkn in ["CLKB", "ML_CLKB", "MK_CLKB"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            let xy_r = xy.delta(1, 0);
            let mut bels = vec![
                builder
                    .bel_indexed("BUFGMUX0", "BUFGMUX", 0)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR0", "ML_CLKB_CKIR0"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL4"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR0"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL0"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR0"])
                    .extra_int_in("CLK", &["CLKB_GCLK00"]),
                builder
                    .bel_indexed("BUFGMUX1", "BUFGMUX", 1)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR1"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL5"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR1"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL1"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR1"])
                    .extra_int_in("CLK", &["CLKB_GCLK01"]),
                builder
                    .bel_indexed("BUFGMUX2", "BUFGMUX", 2)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR2"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL6"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR2"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL2"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR2"])
                    .extra_int_in("CLK", &["CLKB_GCLK02"]),
                builder
                    .bel_indexed("BUFGMUX3", "BUFGMUX", 3)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR3"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL7"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR3"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL3"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR3"])
                    .extra_int_in("CLK", &["CLKB_GCLK03"]),
                builder
                    .bel_indexed("BUFGMUX4", "BUFGMUX", 4)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL0"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL0"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR4"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL4"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR4"])
                    .extra_int_in("CLK", &["CLKB_GCLK04"]),
                builder
                    .bel_indexed("BUFGMUX5", "BUFGMUX", 5)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL1"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL1"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR5"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL5"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR5"])
                    .extra_int_in("CLK", &["CLKB_GCLK05"]),
                builder
                    .bel_indexed("BUFGMUX6", "BUFGMUX", 6)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL2", "ML_CLKB_CKIL2"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL2"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR6"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL6"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR6"])
                    .extra_int_in("CLK", &["CLKB_GCLK06"]),
                builder
                    .bel_indexed("BUFGMUX7", "BUFGMUX", 7)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL3"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL3"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR7"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL7"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR7"])
                    .extra_int_in("CLK", &["CLKB_GCLK07"]),
                builder.bel_virtual("GLOBALSIG.B0"),
                builder.bel_virtual("GLOBALSIG.B1"),
            ];
            if tkn == "ML_CLKB" {
                bels.push(
                    builder
                        .bel_virtual("BREFCLK")
                        .extra_wire("BREFCLK", &["ML_CLKB_BREFCLK"])
                        .extra_wire("BREFCLK2", &["ML_CLKB_BREFCLK2"]),
                );
            }
            builder.extract_xnode(tkn, xy, &[], &[xy_l, xy_r], tkn, &bels, &[]);
        }
    }
    for tkn in ["CLKT", "ML_CLKT", "MK_CLKT"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            let xy_r = xy.delta(1, 0);
            let mut bels = vec![
                builder
                    .bel_indexed("BUFGMUX0", "BUFGMUX", 0)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR0"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL4"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR0"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL0"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR0"])
                    .extra_int_in("CLK", &["CLKT_GCLK00"]),
                builder
                    .bel_indexed("BUFGMUX1", "BUFGMUX", 1)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR1"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL5"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR1"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL1"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR1"])
                    .extra_int_in("CLK", &["CLKT_GCLK01"]),
                builder
                    .bel_indexed("BUFGMUX2", "BUFGMUX", 2)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR2", "ML_CLKT_CKIR2"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL6"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR2"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL2"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR2"])
                    .extra_int_in("CLK", &["CLKT_GCLK02"]),
                builder
                    .bel_indexed("BUFGMUX3", "BUFGMUX", 3)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR3"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL7"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR3"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL3"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR3"])
                    .extra_int_in("CLK", &["CLKT_GCLK03"]),
                builder
                    .bel_indexed("BUFGMUX4", "BUFGMUX", 4)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL0", "ML_CLKT_CKIL0"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL0"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR4"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL4"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR4"])
                    .extra_int_in("CLK", &["CLKT_GCLK04"]),
                builder
                    .bel_indexed("BUFGMUX5", "BUFGMUX", 5)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL1"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL1"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR5"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL5"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR5"])
                    .extra_int_in("CLK", &["CLKT_GCLK05"]),
                builder
                    .bel_indexed("BUFGMUX6", "BUFGMUX", 6)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL2"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL2"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR6"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL6"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR6"])
                    .extra_int_in("CLK", &["CLKT_GCLK06"]),
                builder
                    .bel_indexed("BUFGMUX7", "BUFGMUX", 7)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL3"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL3"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR7"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL7"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR7"])
                    .extra_int_in("CLK", &["CLKT_GCLK07"]),
                builder.bel_virtual("GLOBALSIG.T0"),
                builder.bel_virtual("GLOBALSIG.T1"),
            ];
            if tkn == "ML_CLKT" {
                bels.push(
                    builder
                        .bel_virtual("BREFCLK")
                        .extra_wire("BREFCLK", &["ML_CLKT_BREFCLK"])
                        .extra_wire("BREFCLK2", &["ML_CLKT_BREFCLK2"]),
                );
            }
            builder.extract_xnode(tkn, xy, &[], &[xy_l, xy_r], tkn, &bels, &[]);
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC") {
        let mut bel = builder.bel_virtual("CLKC");
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_B{i}"), &[format!("CLKC_GCLKB_IN{i}")])
                .extra_wire(format!("IN_T{i}"), &[format!("CLKC_GCLKT_IN{i}")])
                .extra_wire(format!("OUT_B{i}"), &[format!("CLKC_GCLKB{i}")])
                .extra_wire(format!("OUT_T{i}"), &[format!("CLKC_GCLKT{i}")]);
        }
        builder.extract_xnode_bels("CLKC", xy, &[], &[xy], "CLKC", &[bel]);
    }

    for &xy in rd.tiles_by_kind_name("GCLKC") {
        let mut bel = builder.bel_virtual("GCLKC");
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_B{i}"), &[format!("GCLKC_GCLKB{i}")])
                .extra_wire(format!("IN_T{i}"), &[format!("GCLKC_GCLKT{i}")])
                .extra_wire(format!("OUT_L{i}"), &[format!("GCLKC_GCLKL{i}")])
                .extra_wire(format!("OUT_R{i}"), &[format!("GCLKC_GCLKR{i}")]);
        }
        builder.extract_xnode_bels("GCLKC", xy, &[], &[xy], "GCLKC", &[bel]);
    }

    for tkn in ["GCLKH", "LR_GCLKH"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_s_xy = builder.walk_to_int(xy, Dir::S).unwrap();
            let int_n_xy = builder.walk_to_int(xy, Dir::N).unwrap();
            let mut bel = builder.bel_virtual("GCLKH");
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN{i}"), &[format!("GCLKH_GCLK_B{i}")])
                    .extra_int_out(format!("OUT_UP{i}"), &[format!("GCLKH_GCLK_UP{i}")])
                    .extra_int_out(format!("OUT_DN{i}"), &[format!("GCLKH_GCLK_DN{i}")]);
            }
            builder.extract_xnode_bels(
                "GCLKH",
                xy,
                &[],
                &[int_s_xy, int_n_xy],
                "GCLKH",
                &[builder.bel_virtual("GLOBALSIG"), bel],
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("BRAMSITE") {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xnode_bels(
            "BRAM",
            xy,
            &[],
            &int_xy,
            "BRAM",
            &[
                builder.bel_xy("BRAM", "RAMB16", 0, 0),
                builder.bel_xy("MULT", "MULT18X18", 0, 0),
            ],
        );
    }

    for (tkn, kind) in [("BBTERM", "DCMCONN.BOT"), ("BTTERM", "DCMCONN.TOP")] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = [xy.delta(0, if kind == "DCMCONN.BOT" { 1 } else { -1 })];
            builder.extract_xnode_bels(
                kind,
                xy,
                &[],
                &int_xy,
                kind,
                &[builder
                    .bel_virtual("DCMCONN")
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
            "GIGABIT",
            xy,
            &[],
            &int_xy,
            "GIGABIT.B",
            &[
                builder
                    .bel_xy("GT", "GT", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN", "TST10B8BICRD0", "TST10B8BICRD1"])
                    .pin_name_only("BREFCLK", 1)
                    .pin_name_only("BREFCLK2", 1),
                builder
                    .bel_indexed("IPAD.RXP", "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("IPAD.RXN", "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("OPAD.TXP", "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed("OPAD.TXN", "GTOPAD", 1)
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
            "GIGABIT",
            xy,
            &[],
            &int_xy,
            "GIGABIT.T",
            &[
                builder
                    .bel_xy("GT", "GT", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN", "TST10B8BICRD0", "TST10B8BICRD1"])
                    .pin_name_only("BREFCLK", 1)
                    .pin_name_only("BREFCLK2", 1),
                builder
                    .bel_indexed("IPAD.RXP", "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("IPAD.RXN", "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("OPAD.TXP", "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed("OPAD.TXN", "GTOPAD", 1)
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
            "GIGABIT10",
            xy,
            &[],
            &int_xy,
            "GIGABIT10.B",
            &[
                builder
                    .bel_xy("GT10", "GT10", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN"])
                    .pin_name_only("BREFCLKPIN", 1)
                    .pin_name_only("BREFCLKNIN", 1),
                builder
                    .bel_indexed("IPAD.RXP", "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("IPAD.RXN", "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("OPAD.TXP", "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed("OPAD.TXN", "GTOPAD", 1)
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
            "GIGABIT10",
            xy,
            &[],
            &int_xy,
            "GIGABIT10.T",
            &[
                builder
                    .bel_xy("GT10", "GT10", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN"])
                    .pin_name_only("BREFCLKPIN", 1)
                    .pin_name_only("BREFCLKNIN", 1),
                builder
                    .bel_indexed("IPAD.RXP", "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("IPAD.RXN", "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed("OPAD.TXP", "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed("OPAD.TXN", "GTOPAD", 1)
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
                tkn,
                xy,
                &[],
                &int_xy,
                tkn,
                &[builder.bel_xy("PPC405", "PPC405", 0, 0)],
            );
        }
    }

    for tkn in ["REG_L", "REG_R"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_o = xy.delta(if xy.x == 0 { 1 } else { -1 }, 0);
            let int_s_xy = builder.walk_to_int(xy_o, Dir::S).unwrap();
            let int_n_xy = builder.walk_to_int(xy_o, Dir::N).unwrap();
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
                tkn,
                xy,
                &buf_xy,
                &int_xy,
                tkn,
                &[builder.bel_xy("PCILOGIC", "PCILOGIC", 0, 0)],
            );
        }
    }

    builder.build()
}
