use prjcombine_interconnect::{
    db::{IntDb, TileWireCoord},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;
use prjcombine_virtex2::{
    defs,
    defs::virtex2::{ccls, tcls, wires},
};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::virtex2::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => ccls::PASS_W,
        Dir::E => ccls::PASS_E,
        Dir::S => ccls::PASS_S,
        Dir::N => ccls::PASS_N,
    }));

    builder.wire_names(
        wires::PULLUP,
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
        builder.wire_names(wires::GCLK[i], &[format!("GCLK{i}")]);
    }
    for i in 0..8 {
        builder.wire_names(
            wires::DCM_CLKPAD[i],
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
        builder.wire_names(
            wires::OMUX[i],
            &[format!("OMUX{i}"), format!("LPPC_INT_OMUX{i}")],
        );
        let omux_da1 = builder.db.get_wire(&format!("OMUX_{da1}{i}"));
        builder.wire_names(
            omux_da1,
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
            let omux_da2 = builder.db.get_wire(&format!("OMUX_{da1}{da2}{i}"));
            builder.wire_names(
                omux_da2,
                &[
                    format!("OMUX_{da1}{da2}{i}"),
                    format!("LPPC_INT_OMUX_{da1}{da2}{i}"),
                ],
            );
        }
        if let Some(db) = db {
            let omux_db = builder.db.get_wire(&format!("OMUX_{db}{i}"));
            builder.wire_names(
                omux_db,
                &[format!("OMUX_{db}{i}"), format!("LPPC_INT_OMUX_{db}{i}")],
            );
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            let beg = builder.db.get_wire(&format!("DBL_{dir}0[{i}]"));
            let mid = builder.db.get_wire(&format!("DBL_{dir}1[{i}]"));
            let end = builder.db.get_wire(&format!("DBL_{dir}2[{i}]"));
            let (end2, e2d) = match dir {
                Dir::W => (wires::DBL_W2_N[i], Dir::N),
                Dir::E => (wires::DBL_E2_S[i], Dir::S),
                Dir::S => (wires::DBL_S3[i], Dir::S),
                Dir::N => (wires::DBL_N3[i], Dir::N),
            };
            builder.wire_names(
                beg,
                &[format!("{dir}2BEG{i}"), format!("LPPC_INT_{dir}2BEG{i}")],
            );
            builder.wire_names(
                mid,
                &[format!("{dir}2MID{i}"), format!("LPPC_INT_{dir}2MID{i}")],
            );
            builder.wire_names(
                end,
                &[format!("{dir}2END{i}"), format!("LPPC_INT_{dir}2END{i}")],
            );
            builder.wire_names(
                end2,
                &[
                    format!("{dir}2END_{e2d}{i}"),
                    format!("LPPC_INT_{dir}2END_{e2d}{i}"),
                ],
            );
        }
    }

    for dir in Dir::DIRS {
        for i in 0..10 {
            for (j, seg) in [
                (0, "BEG"),
                (1, "A"),
                (2, "B"),
                (3, "MID"),
                (4, "C"),
                (5, "D"),
                (6, "END"),
            ] {
                let w = builder.db.get_wire(&format!("HEX_{dir}{j}[{i}]"));
                builder.wire_names(
                    w,
                    &[
                        format!("{dir}6{seg}{i}"),
                        format!("LR_IOIS_{dir}6{seg}{i}"),
                        format!("TB_IOIS_{dir}6{seg}{i}"),
                        format!("LPPC_INT_{dir}6{seg}{i}"),
                    ],
                );
            }
            let (end2, e2d) = match dir {
                Dir::W => (wires::HEX_W6_N[i], Dir::N),
                Dir::E => (wires::HEX_E6_S[i], Dir::S),
                Dir::S => (wires::HEX_S7[i], Dir::S),
                Dir::N => (wires::HEX_N7[i], Dir::N),
            };
            builder.wire_names(
                end2,
                &[
                    format!("{dir}6END_{e2d}{i}"),
                    format!("LR_IOIS_{dir}6END_{e2d}{i}"),
                    format!("TB_IOIS_{dir}6END_{e2d}{i}"),
                    format!("LPPC_INT_{dir}6END_{e2d}{i}"),
                ],
            );
        }
    }

    for i in 0..24 {
        builder.wire_names(wires::LH[i], &[format!("LH{i}"), format!("LPPC_INT_LH{i}")]);
        builder.wire_names(wires::LV[i], &[format!("LV{i}")]);
    }

    for i in 0..4 {
        let wire = wires::IMUX_CLK[i];
        builder.mark_optinv(wire, wires::IMUX_CLK_OPTINV[i]);
        builder.wire_names(
            wire,
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
        builder.wire_names(
            wires::IMUX_IOI_ICLK[i],
            &[format!(
                "IOIS_CK{j}_B{k}",
                j = [2, 1, 2, 1][i],
                k = [0, 0, 2, 2][i]
            )],
        );
    }
    for i in 0..4 {
        let wire = wires::IMUX_DCM_CLK[i];
        builder.mark_optinv(wire, wires::IMUX_DCM_CLK_OPTINV[i]);
        builder.wire_names(
            wire,
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
        builder.mark_optinv(wires::IMUX_SR[i], wires::IMUX_SR_OPTINV[i]);
        builder.wire_names(
            wires::IMUX_SR[i],
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
        builder.mark_optinv(wires::IMUX_CE[i], wires::IMUX_CE_OPTINV[i]);
        builder.wire_names(
            wires::IMUX_CE[i],
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
        builder.mark_optinv(wires::IMUX_TI[i], wires::IMUX_TI_OPTINV[i]);
        builder.wire_names(
            wires::IMUX_TI[i],
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
        builder.mark_optinv(wires::IMUX_TS[i], wires::IMUX_TS_OPTINV[i]);
        builder.wire_names(
            wires::IMUX_TS[i],
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
        builder.wire_names(wires::IMUX_CLB_F1[i], &[format!("F1_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_F2[i], &[format!("F2_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_F3[i], &[format!("F3_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_F4[i], &[format!("F4_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_G1[i], &[format!("G1_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_G2[i], &[format!("G2_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_G3[i], &[format!("G3_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_G4[i], &[format!("G4_B{i}")]);
        builder.wire_names(wires::IMUX_CLB_BX[i], &[format!("BX{i}")]);
        builder.wire_names(wires::IMUX_CLB_BY[i], &[format!("BY{i}")]);
    }
    // non-CLB inputs
    for i in 0..4 {
        for j in 0..2 {
            let ri = 3 - i;
            builder.wire_names(
                builder.db.get_wire(&format!("IMUX_G{i}_FAN[{j}]")),
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
            builder.wire_names(
                builder.db.get_wire(&format!("IMUX_G{i}_DATA[{j}]")),
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
        builder.wire_names(wires::IMUX_IOI_TS1[i], &[format!("TS1_B{i}")]);
    }
    for i in 0..4 {
        builder.wire_names(wires::IMUX_IOI_TS2[i], &[format!("TS2_B{i}")]);
    }
    for i in 0..4 {
        builder.wire_names(wires::IMUX_IOI_ICE[i], &[format!("ICE_B{i}")]);
    }
    for i in 0..4 {
        builder.wire_names(wires::IMUX_IOI_TCE[i], &[format!("TCE_B{i}")]);
    }
    // BRAM special inputs
    let bram_s = builder.make_term_naming("BRAM_S");
    for ab in ['A', 'B'] {
        for i in 0..4 {
            let root = builder.db.get_wire(&format!("IMUX_BRAM_ADDR{ab}[{i}]"));
            builder.wire_names(root, &[format!("BRAM_ADDR{ab}_B{i}")]);
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
                    last = builder
                        .db
                        .get_wire(&format!("IMUX_BRAM_ADDR{ab}_{dir}{j}[{i}]"));
                    if j == 4 {
                        builder.wire_names(last, &[format!("BRAM_ADDR{ab}_{dir}END{i}")]);
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
        let w = wires::OUT_FAN[i];
        builder.wire_names(
            w,
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
        builder.mark_test_mux_in(wires::OUT_FAN_TMIN[i], w);
        if i == 0 {
            builder.extra_name_tile("MK_T_IOIS", "IOIS_BREFCLK_SE", w);
        }
        if i == 2 {
            builder.extra_name_tile("MK_B_IOIS", "IOIS_BREFCLK_SE", w);
        }
    }

    // We call secondary outputs by their OMUX index.
    for i in 2..24 {
        let w = wires::OUT_SEC[i];
        builder.wire_names(
            w,
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
        builder.mark_test_mux_in(wires::OUT_SEC_TMIN[i], w);
    }

    // Same for tertiary.
    for i in 8..18 {
        for j in 0..2 {
            builder.wire_names(
                [wires::OUT_HALF0, wires::OUT_HALF1][j][i],
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
        builder.wire_names(
            wires::OUT_TEST[i],
            &[
                format!("LRPPC_INT_TEST{i}"),
                format!("BPPC_INT_TEST{i}"),
                format!("TPPC_INT_TEST{i}"),
                format!("GIGABIT_INT_TEST{i}"),
            ],
        );
    }

    builder.wire_names(wires::OUT_TBUS, &["TBUS"]);
    let w = wires::OUT_PCI[0];
    builder.wire_names(
        w,
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
    let w = wires::OUT_PCI[1];
    builder.wire_names(
        w,
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
        builder.wire_names(
            wires::IMUX_BUFG_SEL[i],
            &[format!("CLKB_SELDUB{i}"), format!("CLKT_SELDUB{i}")],
        );
    }
    for i in 0..8 {
        let ii = i % 4;
        let lr = if i < 4 { 'R' } else { 'L' };
        builder.wire_names(
            wires::IMUX_BUFG_CLK[i],
            &[
                format!("CLKB_CLKDUB{lr}{ii}"),
                format!("CLKT_CLKDUB{lr}{ii}"),
            ],
        );
    }
    for i in 0..8 {
        builder.wire_names(
            wires::OUT_BUFG[i],
            &[format!("CLKB_GCLK_ROOT{i}"), format!("CLKT_GCLK_ROOT{i}")],
        );
    }

    let bels_int = [builder.bel_xy(defs::bslots::RLL, "RLL", 0, 0)];
    let bels_int_sigh = [builder
        .bel_xy(defs::bslots::RLL, "RLL", 0, 0)
        .pins_name_only(&["LH0", "LH6", "LH12", "LH18", "LV0", "LV6", "LV12", "LV18"])];
    builder.extract_int_id(
        tcls::INT_CLB,
        defs::bslots::INT,
        "CENTER",
        "INT_CLB",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI,
        defs::bslots::INT,
        "LR_IOIS",
        "INT_IOI_WE",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI,
        defs::bslots::INT,
        "TB_IOIS",
        "INT_IOI_SN",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI,
        defs::bslots::INT,
        "ML_TB_IOIS",
        "INT_IOI_SN",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI,
        defs::bslots::INT,
        "ML_TBS_IOIS",
        "INT_IOI_SN",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI,
        defs::bslots::INT,
        "GIGABIT_IOI",
        "INT_IOI_SN",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI,
        defs::bslots::INT,
        "GIGABIT10_IOI",
        "INT_IOI_SN",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI_CLK_S,
        defs::bslots::INT,
        "MK_B_IOIS",
        "INT_IOI_CLK_S",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_IOI_CLK_N,
        defs::bslots::INT,
        "MK_T_IOIS",
        "INT_IOI_CLK_N",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_BRAM,
        defs::bslots::INT,
        "BRAM0",
        "INT_BRAM",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_BRAM,
        defs::bslots::INT,
        "BRAM1",
        "INT_BRAM",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_BRAM,
        defs::bslots::INT,
        "BRAM2",
        "INT_BRAM",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_BRAM,
        defs::bslots::INT,
        "BRAM3",
        "INT_BRAM",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_DCM_V2,
        defs::bslots::INT,
        "BRAM_IOIS",
        "INT_DCM_V2",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_DCM_V2P,
        defs::bslots::INT,
        "ML_BRAM_IOIS",
        "INT_DCM_V2P",
        &bels_int_sigh,
    );
    builder.extract_int_id(tcls::INT_CNR, defs::bslots::INT, "LL", "INT_CNR", &bels_int);
    builder.extract_int_id(tcls::INT_CNR, defs::bslots::INT, "LR", "INT_CNR", &bels_int);
    builder.extract_int_id(tcls::INT_CNR, defs::bslots::INT, "UL", "INT_CNR", &bels_int);
    builder.extract_int_id(tcls::INT_CNR, defs::bslots::INT, "UR", "INT_CNR", &bels_int);
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT_INT0",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT_INT1",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT_INT2",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT_INT3",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_GT_CLKPAD,
        defs::bslots::INT,
        "BGIGABIT_INT4",
        "INT_GT_CLKPAD",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT_INT0",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT_INT1",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT_INT2",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT_INT3",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_GT_CLKPAD,
        defs::bslots::INT,
        "TGIGABIT_INT4",
        "INT_GT_CLKPAD",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT0",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT1",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT2",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT3",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT4",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT5",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT6",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BGIGABIT10_INT7",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_GT_CLKPAD,
        defs::bslots::INT,
        "BGIGABIT10_INT8",
        "INT_GT_CLKPAD",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT0",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT1",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT2",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT3",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT4",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT5",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT6",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TGIGABIT10_INT7",
        "INT_GT",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_GT_CLKPAD,
        defs::bslots::INT,
        "TGIGABIT10_INT8",
        "INT_GT_CLKPAD",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "LPPC_X0Y0_INT",
        "INT_PPC_W",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "LPPC_X1Y0_INT",
        "INT_PPC_W",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "LLPPC_X0Y0_INT",
        "INT_PPC_W",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "LLPPC_X1Y0_INT",
        "INT_PPC_W",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "ULPPC_X0Y0_INT",
        "INT_PPC_W",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "ULPPC_X1Y0_INT",
        "INT_PPC_W",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "RPPC_X0Y0_INT",
        "INT_PPC_E",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "RPPC_X1Y0_INT",
        "INT_PPC_E",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BPPC_X0Y0_INT",
        "INT_PPC_S",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "BPPC_X1Y0_INT",
        "INT_PPC_S",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TPPC_X0Y0_INT",
        "INT_PPC_N",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_PPC,
        defs::bslots::INT,
        "TPPC_X1Y0_INT",
        "INT_PPC_N",
        &bels_int,
    );

    if let Some(pips) = builder
        .pips
        .get_mut(&(tcls::INT_GT_CLKPAD, defs::bslots::INT))
    {
        for w in [
            wires::IMUX_CE[0],
            wires::IMUX_CE[1],
            wires::IMUX_TS[0],
            wires::IMUX_TS[1],
        ] {
            pips.pips.remove(&(
                TileWireCoord::new_idx(0, w),
                TileWireCoord::new_idx(0, wires::PULLUP),
            ));
        }
    }

    let slice_name_only = [
        "DX", "DY", "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG",
        "DIG", "SLICEWE0", "SLICEWE1", "SLICEWE2", "BXOUT", "BYOUT", "BYINVOUT", "SOPIN", "SOPOUT",
    ];

    let bels_clb = [
        builder
            .bel_xy(defs::bslots::SLICE[0], "SLICE", 0, 0)
            .pins_name_only(&slice_name_only)
            .extra_wire("BYINVOUT_LOCAL", &["BYINVOUT_LOCAL0"]),
        builder
            .bel_xy(defs::bslots::SLICE[1], "SLICE", 0, 1)
            .pins_name_only(&slice_name_only)
            .extra_wire("FX_S", &["FX_S1"])
            .extra_wire("COUT_N", &["COUT_N3"])
            .extra_wire("BYOUT_LOCAL", &["BYOUT_LOCAL1"])
            .extra_wire("BYINVOUT_LOCAL", &["BYINVOUT_LOCAL1"]),
        builder
            .bel_xy(defs::bslots::SLICE[2], "SLICE", 1, 0)
            .pins_name_only(&slice_name_only)
            .extra_wire("SOPOUT_W", &["SOPOUT_W2"]),
        builder
            .bel_xy(defs::bslots::SLICE[3], "SLICE", 1, 1)
            .pins_name_only(&slice_name_only)
            .extra_wire("COUT_N", &["COUT_N1"])
            .extra_wire("DIG_LOCAL", &["DIG_LOCAL3"])
            .extra_wire("DIG_S", &["DIG_S3"])
            .extra_wire("SOPOUT_W", &["SOPOUT_W3"]),
        builder
            .bel_indexed(defs::bslots::TBUF[0], "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed(defs::bslots::TBUF[1], "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual(defs::bslots::TBUS)
            .extra_wire("BUS0", &["TBUF0"])
            .extra_wire("BUS1", &["TBUF1"])
            .extra_wire("BUS2", &["TBUF2"])
            .extra_wire("BUS3", &["TBUF3"])
            .extra_wire("BUS3_E", &["TBUF3_E"])
            .extra_int_out("OUT", &["TBUS"]),
    ];
    builder.extract_int_bels_id(tcls::CLB, "CENTER", "CLB", &bels_clb);

    let ioi_name_only = ["DIFFI_IN", "PADOUT", "DIFFO_IN", "DIFFO_OUT"];
    let bels_ioi = [
        builder
            .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF0"),
        builder
            .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF1"),
        builder
            .bel_indexed(defs::bslots::IOI[2], "IOB", 2)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF2"),
        builder
            .bel_indexed(defs::bslots::IOI[3], "IOB", 3)
            .pins_name_only(&ioi_name_only)
            .extra_wire_force("IBUF", "IOIS_IBUF3"),
    ];
    builder.extract_int_bels_id(tcls::IOI, "LR_IOIS", "IOI", &bels_ioi);
    builder.extract_int_bels_id(tcls::IOI, "TB_IOIS", "IOI", &bels_ioi);
    builder.extract_int_bels_id(tcls::IOI, "ML_TB_IOIS", "IOI", &bels_ioi);
    builder.extract_int_bels_id(tcls::IOI, "ML_TBS_IOIS", "IOI_TBS", &bels_ioi);
    builder.extract_int_bels_id(tcls::IOI, "GIGABIT_IOI", "IOI", &bels_ioi);
    builder.extract_int_bels_id(tcls::IOI, "GIGABIT10_IOI", "IOI", &bels_ioi);
    if rd.part.contains("vpx") {
        let bels_ioi_clk_b = [
            bels_ioi[0].clone(),
            bels_ioi[1].clone(),
            builder
                .bel_single(defs::bslots::IOI[2], "CLKPPAD2")
                .pin_name_only("I", 1),
            builder
                .bel_single(defs::bslots::IOI[3], "CLKNPAD2")
                .pin_name_only("I", 1),
            builder
                .bel_virtual(defs::bslots::BREFCLK_INT)
                .extra_int_out("BREFCLK", &["IOIS_BREFCLK_SE"]),
        ];
        let bels_ioi_clk_t = [
            builder
                .bel_single(defs::bslots::IOI[0], "CLKPPAD1")
                .pin_name_only("I", 1),
            builder
                .bel_single(defs::bslots::IOI[1], "CLKNPAD1")
                .pin_name_only("I", 1),
            bels_ioi[2].clone(),
            bels_ioi[3].clone(),
            builder
                .bel_virtual(defs::bslots::BREFCLK_INT)
                .extra_int_out("BREFCLK", &["IOIS_BREFCLK_SE"]),
        ];
        builder.extract_int_bels_id(tcls::IOI_CLK_S, "MK_B_IOIS", "IOI_CLK_S", &bels_ioi_clk_b);
        builder.extract_int_bels_id(tcls::IOI_CLK_N, "MK_T_IOIS", "IOI_CLK_N", &bels_ioi_clk_t);
    }

    let bels_dcm = [builder.bel_xy(defs::bslots::DCM, "DCM", 0, 0)];
    builder.extract_int_bels_id(tcls::DCM_V2, "BRAM_IOIS", "DCM_V2", &bels_dcm);
    builder.extract_int_bels_id(tcls::DCM_V2P, "ML_BRAM_IOIS", "DCM_V2P", &bels_dcm);

    let (ll, lr, ul, ur) = if rd.family == "virtex2p" {
        (
            tcls::CNR_SW_V2P,
            tcls::CNR_SE_V2P,
            tcls::CNR_NW_V2P,
            tcls::CNR_NE_V2P,
        )
    } else {
        (
            tcls::CNR_SW_V2,
            tcls::CNR_SE_V2,
            tcls::CNR_NW_V2,
            tcls::CNR_NE_V2,
        )
    };
    let (n_ll, n_lr, n_ul, n_ur) = if rd.family == "virtex2p" {
        ("CNR_SW_V2P", "CNR_SE_V2P", "CNR_NW_V2P", "CNR_NE_V2P")
    } else {
        ("CNR_SW_V2", "CNR_SE_V2", "CNR_NW_V2", "CNR_NE_V2")
    };
    builder.extract_int_bels_id(
        ll,
        "LL",
        n_ll,
        &[
            builder.bel_indexed(defs::bslots::DCI[0], "DCI", 6),
            builder.bel_indexed(defs::bslots::DCI[1], "DCI", 5),
        ],
    );

    builder.extract_int_bels_id(
        lr,
        "LR",
        n_lr,
        &[
            builder.bel_indexed(defs::bslots::DCI[0], "DCI", 3),
            builder.bel_indexed(defs::bslots::DCI[1], "DCI", 4),
            builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
            builder.bel_single(defs::bslots::CAPTURE, "CAPTURE"),
            builder.bel_single(defs::bslots::ICAP, "ICAP"),
        ],
    );
    builder.extract_int_bels_id(
        ul,
        "UL",
        n_ul,
        &[
            builder.bel_indexed(defs::bslots::DCI[0], "DCI", 7),
            builder.bel_indexed(defs::bslots::DCI[1], "DCI", 0),
            builder.bel_single(defs::bslots::PMV, "PMV"),
        ],
    );
    if rd.family == "virtex2p" {
        builder.extract_int_bels_id(
            ur,
            "UR",
            n_ur,
            &[
                builder.bel_indexed(defs::bslots::DCI[0], "DCI", 2),
                builder.bel_indexed(defs::bslots::DCI[1], "DCI", 1),
                builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
                builder.bel_single(defs::bslots::JTAGPPC, "JTAGPPC"),
            ],
        );
    } else {
        builder.extract_int_bels_id(
            ur,
            "UR",
            n_ur,
            &[
                builder.bel_indexed(defs::bslots::DCI[0], "DCI", 2),
                builder.bel_indexed(defs::bslots::DCI[1], "DCI", 1),
                builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
            ],
        );
    }

    for (tkn, n) in [
        ("LTERM321", "TERM_W_U"),
        ("LTERM010", "TERM_W_U"),
        ("LTERM323", "TERM_W_D"),
        ("LTERM210", "TERM_W_D"),
        ("LTERM323_PCI", "TERM_W_U"),
        ("LTERM210_PCI", "TERM_W_U"),
        ("CNR_LTERM", "TERM_W"),
    ] {
        builder.extract_term_id(
            defs::virtex2::ccls::TERM_W,
            Some((tcls::TERM_W, defs::bslots::TERM_W)),
            Dir::W,
            tkn,
            n,
        );
    }
    for (tkn, n) in [
        ("RTERM321", "TERM_E_U"),
        ("RTERM010", "TERM_E_U"),
        ("RTERM323", "TERM_E_D"),
        ("RTERM210", "TERM_E_D"),
        ("RTERM323_PCI", "TERM_E_U"),
        ("RTERM210_PCI", "TERM_E_U"),
        ("CNR_RTERM", "TERM_E"),
    ] {
        builder.extract_term_id(
            defs::virtex2::ccls::TERM_E,
            Some((tcls::TERM_E, defs::bslots::TERM_E)),
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
        builder.extract_term_id(
            defs::virtex2::ccls::TERM_S,
            Some((tcls::TERM_S, defs::bslots::TERM_S)),
            Dir::S,
            tkn,
            "TERM_S",
        );
    }
    builder.extract_term_id(
        defs::virtex2::ccls::TERM_S,
        Some((tcls::TERM_S, defs::bslots::TERM_S)),
        Dir::S,
        "CNR_BTERM",
        "TERM_S_CNR",
    );
    builder.extract_term_id(
        defs::virtex2::ccls::TERM_S,
        Some((tcls::TERM_S, defs::bslots::TERM_S)),
        Dir::S,
        "ML_CNR_BTERM",
        "TERM_S_CNR",
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
        builder.extract_term_id(
            defs::virtex2::ccls::TERM_N,
            Some((tcls::TERM_N, defs::bslots::TERM_N)),
            Dir::N,
            tkn,
            "TERM_N",
        );
    }
    builder.extract_term_id(
        defs::virtex2::ccls::TERM_N,
        Some((tcls::TERM_N, defs::bslots::TERM_N)),
        Dir::N,
        "CNR_TTERM",
        "TERM_N_CNR",
    );

    for &xy_b in rd.tiles_by_kind_name("PTERMB") {
        let xy_t = xy_b.delta(0, 14);
        let int_s_xy = builder.walk_to_int(xy_b, Dir::S, false).unwrap();
        let int_n_xy = builder.walk_to_int(xy_t, Dir::N, false).unwrap();
        builder.extract_pass_tile_id(
            defs::virtex2::ccls::PPC_S,
            Dir::S,
            int_n_xy,
            Some(xy_t),
            Some(xy_b),
            Some("PPC_S"),
            Some((tcls::PPC_TERM_S, defs::bslots::PPC_TERM_S, "PPC_TERM_S")),
            None,
            int_s_xy,
            &[],
        );
        builder.extract_pass_tile_id(
            defs::virtex2::ccls::PPC_N,
            Dir::N,
            int_s_xy,
            Some(xy_b),
            Some(xy_t),
            Some("PPC_N"),
            Some((tcls::PPC_TERM_N, defs::bslots::PPC_TERM_N, "PPC_TERM_N")),
            None,
            int_n_xy,
            &[],
        );
    }
    for tkn in ["PTERMR", "PTERMBR", "PTERMTR"] {
        for &xy_r in rd.tiles_by_kind_name(tkn) {
            let int_w_xy = builder.walk_to_int(xy_r, Dir::W, false).unwrap();
            let int_e_xy = builder.walk_to_int(xy_r, Dir::E, false).unwrap();
            builder.extract_pass_tile_id(
                defs::virtex2::ccls::PPC_W,
                Dir::W,
                int_e_xy,
                Some(xy_r),
                Some(int_w_xy),
                Some("PPC_W"),
                Some((tcls::PPC_TERM_W, defs::bslots::PPC_TERM_W, "PPC_TERM_W")),
                None,
                int_w_xy,
                &[],
            );
            builder.extract_pass_tile_id(
                defs::virtex2::ccls::PPC_E,
                Dir::E,
                int_w_xy,
                Some(int_w_xy),
                Some(xy_r),
                Some("PPC_E"),
                Some((tcls::PPC_TERM_E, defs::bslots::PPC_TERM_E, "PPC_TERM_E")),
                None,
                int_e_xy,
                &[],
            );
        }
    }

    for (tkn, tcls, naming) in [
        ("BGIGABIT_INT0", tcls::INTF_GT_S0, "INTF_GT"),
        ("BGIGABIT_INT1", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT_INT2", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT_INT3", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT_INT4", tcls::INTF_GT_S_CLKPAD, "INTF_GT_CLKPAD"),
        ("TGIGABIT_INT0", tcls::INTF_GT_N0, "INTF_GT"),
        ("TGIGABIT_INT1", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT_INT2", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT_INT3", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT_INT4", tcls::INTF_GT_N_CLKPAD, "INTF_GT_CLKPAD"),
        ("BGIGABIT10_INT0", tcls::INTF_GT_S0, "INTF_GT"),
        ("BGIGABIT10_INT1", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT10_INT2", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT10_INT3", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT10_INT4", tcls::INTF_GT_S0, "INTF_GT"),
        ("BGIGABIT10_INT5", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT10_INT6", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT10_INT7", tcls::INTF_GT_S123, "INTF_GT"),
        ("BGIGABIT10_INT8", tcls::INTF_GT_S_CLKPAD, "INTF_GT_CLKPAD"),
        ("TGIGABIT10_INT0", tcls::INTF_GT_N0, "INTF_GT"),
        ("TGIGABIT10_INT1", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT10_INT2", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT10_INT3", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT10_INT4", tcls::INTF_GT_N0, "INTF_GT"),
        ("TGIGABIT10_INT5", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT10_INT6", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT10_INT7", tcls::INTF_GT_N123, "INTF_GT"),
        ("TGIGABIT10_INT8", tcls::INTF_GT_N_CLKPAD, "INTF_GT_CLKPAD"),
        ("LPPC_X0Y0_INT", tcls::INTF_PPC, "INTF_PPC_W"),
        ("LPPC_X1Y0_INT", tcls::INTF_PPC, "INTF_PPC_W"),
        ("LLPPC_X0Y0_INT", tcls::INTF_PPC, "INTF_PPC_W"),
        ("LLPPC_X1Y0_INT", tcls::INTF_PPC, "INTF_PPC_W"),
        ("ULPPC_X0Y0_INT", tcls::INTF_PPC, "INTF_PPC_W"),
        ("ULPPC_X1Y0_INT", tcls::INTF_PPC, "INTF_PPC_W"),
        ("RPPC_X0Y0_INT", tcls::INTF_PPC, "INTF_PPC_E"),
        ("RPPC_X1Y0_INT", tcls::INTF_PPC, "INTF_PPC_E"),
        ("BPPC_X0Y0_INT", tcls::INTF_PPC, "INTF_PPC_S"),
        ("BPPC_X1Y0_INT", tcls::INTF_PPC, "INTF_PPC_S"),
        ("TPPC_X0Y0_INT", tcls::INTF_PPC, "INTF_PPC_N"),
        ("TPPC_X1Y0_INT", tcls::INTF_PPC, "INTF_PPC_N"),
    ] {
        builder.extract_intf_id(
            tcls,
            Dir::E,
            tkn,
            naming,
            defs::bslots::INTF_TESTMUX,
            false,
            None,
        );
    }

    for (tcls, nn, tkn) in [
        (tcls::CLK_S_V2, "CLK_S_V2", "CLKB"),
        (tcls::CLK_S_V2P, "CLK_S_V2P", "ML_CLKB"),
        (tcls::CLK_S_V2PX, "CLK_S_V2PX", "MK_CLKB"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            let xy_r = xy.delta(1, 0);
            let mut bels = vec![
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[0], "BUFGMUX", 0)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR0", "ML_CLKB_CKIR0"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL4"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR0"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL0"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR0"])
                    .extra_int_in("CLK", &["CLKB_GCLK00"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[1], "BUFGMUX", 1)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR1"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL5"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR1"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL1"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR1"])
                    .extra_int_in("CLK", &["CLKB_GCLK01"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[2], "BUFGMUX", 2)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR2"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL6"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR2"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL2"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR2"])
                    .extra_int_in("CLK", &["CLKB_GCLK02"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[3], "BUFGMUX", 3)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIR3"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL7"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR3"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL3"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR3"])
                    .extra_int_in("CLK", &["CLKB_GCLK03"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[4], "BUFGMUX", 4)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL0"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL0"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR4"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL4"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR4"])
                    .extra_int_in("CLK", &["CLKB_GCLK04"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[5], "BUFGMUX", 5)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL1"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL1"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR5"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL5"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR5"])
                    .extra_int_in("CLK", &["CLKB_GCLK05"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[6], "BUFGMUX", 6)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL2", "ML_CLKB_CKIL2"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL2"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR6"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL6"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR6"])
                    .extra_int_in("CLK", &["CLKB_GCLK06"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[7], "BUFGMUX", 7)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKB_CKIL3"])
                    .extra_wire("DCM_PAD_L", &["CLKB_DLL_CLKPADL3"])
                    .extra_wire("DCM_PAD_R", &["CLKB_DLL_CLKPADR7"])
                    .extra_wire("DCM_OUT_L", &["CLKB_DLLOUTL7"])
                    .extra_wire("DCM_OUT_R", &["CLKB_DLLOUTR7"])
                    .extra_int_in("CLK", &["CLKB_GCLK07"]),
                builder.bel_virtual(defs::bslots::GLOBALSIG_S[0]),
                builder.bel_virtual(defs::bslots::GLOBALSIG_S[1]),
            ];
            if tkn == "ML_CLKB" {
                bels.push(
                    builder
                        .bel_virtual(defs::bslots::BREFCLK)
                        .extra_wire("BREFCLK", &["ML_CLKB_BREFCLK"])
                        .extra_wire("BREFCLK2", &["ML_CLKB_BREFCLK2"]),
                );
            }
            builder.extract_xtile_id(
                tcls,
                defs::bslots::CLK_INT,
                xy,
                &[],
                &[xy_l, xy_r],
                nn,
                &bels,
                &[],
            );
        }
    }
    for (tcls, nn, tkn) in [
        (tcls::CLK_N_V2, "CLK_N_V2", "CLKT"),
        (tcls::CLK_N_V2P, "CLK_N_V2P", "ML_CLKT"),
        (tcls::CLK_N_V2PX, "CLK_N_V2PX", "MK_CLKT"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            let xy_r = xy.delta(1, 0);
            let mut bels = vec![
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[0], "BUFGMUX", 0)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR0"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL4"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR0"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL0"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR0"])
                    .extra_int_in("CLK", &["CLKT_GCLK00"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[1], "BUFGMUX", 1)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR1"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL5"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR1"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL1"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR1"])
                    .extra_int_in("CLK", &["CLKT_GCLK01"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[2], "BUFGMUX", 2)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR2", "ML_CLKT_CKIR2"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL6"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR2"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL2"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR2"])
                    .extra_int_in("CLK", &["CLKT_GCLK02"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[3], "BUFGMUX", 3)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIR3"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL7"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR3"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL3"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR3"])
                    .extra_int_in("CLK", &["CLKT_GCLK03"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[4], "BUFGMUX", 4)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL0", "ML_CLKT_CKIL0"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL0"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR4"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL4"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR4"])
                    .extra_int_in("CLK", &["CLKT_GCLK04"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[5], "BUFGMUX", 5)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL1"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL1"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR5"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL5"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR5"])
                    .extra_int_in("CLK", &["CLKT_GCLK05"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[6], "BUFGMUX", 6)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL2"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL2"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR6"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL6"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR6"])
                    .extra_int_in("CLK", &["CLKT_GCLK06"]),
                builder
                    .bel_indexed(defs::bslots::BUFGMUX[7], "BUFGMUX", 7)
                    .pins_name_only(&["I0", "I1"])
                    .extra_wire("CKI", &["CLKT_CKIL3"])
                    .extra_wire("DCM_PAD_L", &["CLKT_DLL_CLKPADL3"])
                    .extra_wire("DCM_PAD_R", &["CLKT_DLL_CLKPADR7"])
                    .extra_wire("DCM_OUT_L", &["CLKT_DLLOUTL7"])
                    .extra_wire("DCM_OUT_R", &["CLKT_DLLOUTR7"])
                    .extra_int_in("CLK", &["CLKT_GCLK07"]),
                builder.bel_virtual(defs::bslots::GLOBALSIG_N[0]),
                builder.bel_virtual(defs::bslots::GLOBALSIG_N[1]),
            ];
            if tkn == "ML_CLKT" {
                bels.push(
                    builder
                        .bel_virtual(defs::bslots::BREFCLK)
                        .extra_wire("BREFCLK", &["ML_CLKT_BREFCLK"])
                        .extra_wire("BREFCLK2", &["ML_CLKT_BREFCLK2"]),
                );
            }
            builder.extract_xtile_id(
                tcls,
                defs::bslots::CLK_INT,
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
        let mut bel = builder.bel_virtual(defs::bslots::CLKC);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_B{i}"), &[format!("CLKC_GCLKB_IN{i}")])
                .extra_wire(format!("IN_T{i}"), &[format!("CLKC_GCLKT_IN{i}")])
                .extra_wire(format!("OUT_B{i}"), &[format!("CLKC_GCLKB{i}")])
                .extra_wire(format!("OUT_T{i}"), &[format!("CLKC_GCLKT{i}")]);
        }
        builder.extract_xtile_bels_id(tcls::CLKC, xy, &[], &[], "CLKC", &[bel], false);
    }

    for &xy in rd.tiles_by_kind_name("GCLKC") {
        for tcls in [tcls::HROW, tcls::HROW_S, tcls::HROW_N] {
            let mut bel = builder.bel_virtual(defs::bslots::HROW);
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN_B{i}"), &[format!("GCLKC_GCLKB{i}")])
                    .extra_wire(format!("IN_T{i}"), &[format!("GCLKC_GCLKT{i}")])
                    .extra_wire(format!("OUT_L{i}"), &[format!("GCLKC_GCLKL{i}")])
                    .extra_wire(format!("OUT_R{i}"), &[format!("GCLKC_GCLKR{i}")]);
            }
            builder.extract_xtile_bels_id(tcls, xy, &[], &[], "GCLKC", &[bel], false);
        }
    }

    for tkn in ["GCLKH", "LR_GCLKH"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_s_xy = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let int_n_xy = builder.walk_to_int(xy, Dir::N, false).unwrap();
            let mut bel = builder.bel_virtual(defs::bslots::HCLK);
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN{i}"), &[format!("GCLKH_GCLK_B{i}")])
                    .extra_int_out(format!("OUT_T{i}"), &[format!("GCLKH_GCLK_UP{i}")])
                    .extra_int_out(format!("OUT_B{i}"), &[format!("GCLKH_GCLK_DN{i}")]);
            }
            builder.extract_xtile_bels_id(
                tcls::HCLK,
                xy,
                &[],
                &[int_s_xy, int_n_xy],
                "HCLK",
                &[builder.bel_virtual(defs::bslots::GLOBALSIG), bel],
                false,
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("BRAMSITE") {
        let mut int_xy = Vec::new();
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels_id(
            tcls::BRAM,
            xy,
            &[],
            &int_xy,
            "BRAM",
            &[
                builder.bel_xy(defs::bslots::BRAM, "RAMB16", 0, 0),
                builder.bel_xy(defs::bslots::MULT, "MULT18X18", 0, 0),
            ],
            false,
        );
    }

    for (tkn, tcls, kind) in [
        ("BBTERM", tcls::DCMCONN_S, "DCMCONN_S"),
        ("BTTERM", tcls::DCMCONN_N, "DCMCONN_N"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = [xy.delta(0, if kind == "DCMCONN_S" { 1 } else { -1 })];
            builder.extract_xtile_bels_id(
                tcls,
                xy,
                &[],
                &int_xy,
                kind,
                &[builder
                    .bel_virtual(defs::bslots::DCMCONN)
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
                false,
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("BGIGABIT") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, -1));
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels_id(
            tcls::GIGABIT_S,
            xy,
            &[],
            &int_xy,
            "GIGABIT_S",
            &[
                builder
                    .bel_xy(defs::bslots::GT, "GT", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN", "TST10B8BICRD0", "TST10B8BICRD1"])
                    .pin_name_only("BREFCLK", 1)
                    .pin_name_only("BREFCLK2", 1),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
            true,
        );
    }

    for &xy in rd.tiles_by_kind_name("TGIGABIT") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, 4));
        for dy in 0..4 {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels_id(
            tcls::GIGABIT_N,
            xy,
            &[],
            &int_xy,
            "GIGABIT_N",
            &[
                builder
                    .bel_xy(defs::bslots::GT, "GT", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN", "TST10B8BICRD0", "TST10B8BICRD1"])
                    .pin_name_only("BREFCLK", 1)
                    .pin_name_only("BREFCLK2", 1),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
            true,
        );
    }

    for &xy in rd.tiles_by_kind_name("BGIGABIT10") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, -1));
        for dy in [0, 1, 2, 3, 5, 6, 7, 8] {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels_id(
            tcls::GIGABIT10_S,
            xy,
            &[],
            &int_xy,
            "GIGABIT10_S",
            &[
                builder
                    .bel_xy(defs::bslots::GT10, "GT10", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN"])
                    .pin_name_only("BREFCLKPIN", 1)
                    .pin_name_only("BREFCLKNIN", 1),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
            true,
        );
    }

    for &xy in rd.tiles_by_kind_name("TGIGABIT10") {
        let mut int_xy = Vec::new();
        int_xy.push(xy.delta(-1, 9));
        for dy in [0, 1, 2, 3, 5, 6, 7, 8] {
            int_xy.push(xy.delta(-1, dy));
        }
        builder.extract_xtile_bels_id(
            tcls::GIGABIT10_N,
            xy,
            &[],
            &int_xy,
            "GIGABIT10_N",
            &[
                builder
                    .bel_xy(defs::bslots::GT10, "GT10", 0, 0)
                    .pins_name_only(&["RXP", "RXN", "TXP", "TXN"])
                    .pin_name_only("BREFCLKPIN", 1)
                    .pin_name_only("BREFCLKNIN", 1),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXP, "GTIPAD", 0)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::IPAD_RXN, "GTIPAD", 1)
                    .pin_name_only("I", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXP, "GTOPAD", 0)
                    .pin_name_only("O", 0),
                builder
                    .bel_indexed(defs::bslots::OPAD_TXN, "GTOPAD", 1)
                    .pin_name_only("O", 0),
            ],
            true,
        );
    }

    for (tcls, nn, tkn) in [
        (tcls::PPC_W, "PPC_W", "LBPPC"),
        (tcls::PPC_E, "PPC_E", "RBPPC"),
    ] {
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
            builder.extract_xtile_bels_id(
                tcls,
                xy,
                &[],
                &int_xy,
                nn,
                &[builder.bel_xy(defs::bslots::PPC405, "PPC405", 0, 0)],
                true,
            );
        }
    }

    for (tcls, tkn) in [(tcls::PCI_W, "REG_L"), (tcls::PCI_E, "REG_R")] {
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
            builder.extract_xtile_bels_id(
                tcls,
                xy,
                &buf_xy,
                &int_xy,
                tkn,
                &[builder.bel_xy(defs::bslots::PCILOGIC, "PCILOGIC", 0, 0)],
                false,
            );
        }
    }

    builder.build()
}
