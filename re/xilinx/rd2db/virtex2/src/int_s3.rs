use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{BelInfo, BelPin, IntDb, PinDir, TileCellId, WireKind},
    dir::Dir,
};
use prjcombine_re_xilinx_naming::db::{BelNaming, BelPinNaming, NamingDb, PipNaming, RawTileId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_virtex2::{
    bels,
    expanded::{REGION_HCLK, REGION_LEAF},
    tslots,
};
use unnamed_entity::EntityId;

use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;

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
            "MACC_VCC_WIRE",
            "BRAM_IOIS_VCC_WIRE",
            "DCM_VCC_WIRE",
            "CNR_VCC_WIRE",
            "CLKB_VCC_WIRE",
            "CLKT_VCC_WIRE",
        ],
    );

    let mut gclk = vec![];
    for i in 0..8 {
        let w = builder.wire(
            format!("GCLK{i}"),
            WireKind::Regional(REGION_LEAF),
            &[format!("GCLK{i}"), format!("GCLK{i}_BRK")],
        );
        gclk.push(w);
    }
    for i in 0..4 {
        builder.logic_out(
            format!("DCM.CLKPAD{i}"),
            &[
                format!("BRAM_IOIS_DLL_CLKPAD{i}"),
                format!("DCM_DLL_CLKPAD{i}"),
                format!("DCM_H_DLL_CLKPAD{i}"),
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
        (9, Dir::W, None, Some(Dir::N), None),
        (10, Dir::N, Some(Dir::W), None, Some(0)),
        (11, Dir::N, None, None, Some(1)),
        (12, Dir::N, Some(Dir::E), None, Some(2)),
        (13, Dir::E, None, None, None),
        (14, Dir::W, Some(Dir::N), None, None),
        (15, Dir::N, None, None, Some(3)),
    ] {
        let omux = builder.mux_out(format!("OMUX{i}"), &[format!("OMUX{i}")]);
        let omux_da1 = builder.branch(
            omux,
            da1,
            format!("OMUX{i}.{da1}"),
            &[format!("OMUX_{da1}{i}")],
        );
        match (xname, da1) {
            (None, _) => (),
            (Some(i), Dir::N) => {
                builder.extra_name(format!("CLKB_TO_OMUX{i}"), omux_da1);
            }
            (Some(i), Dir::S) => {
                builder.extra_name(format!("CLKT_TO_OMUX{i}"), omux_da1);
            }
            _ => unreachable!(),
        }
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
                &[format!("{db}{da1}_{db}")],
            );
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
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
            if matches!(dir, Dir::W | Dir::N) && i >= 6 {
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
        for i in 0..8 {
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
            if matches!(dir, Dir::W | Dir::N) && i >= 6 {
                builder.branch(
                    last,
                    Dir::N,
                    format!("HEX.{dir}{i}.7"),
                    &[format!("{dir}6END_N{i}")],
                );
            }
        }
    }

    let ll_len = if rd.family == "fpgacore" { 12 } else { 24 };
    let lh: Vec<_> = (0..ll_len)
        .map(|i| {
            builder.wire(
                format!("LH.{i}"),
                WireKind::MultiBranch(builder.term_slots[Dir::W]),
                &[format!("LH{i}")],
            )
        })
        .collect();
    for i in 0..ll_len {
        builder.conn_branch(lh[i], Dir::E, lh[(i + 1) % ll_len]);
    }

    let lv: Vec<_> = (0..ll_len)
        .map(|i| {
            builder.wire(
                format!("LV.{i}"),
                WireKind::MultiBranch(builder.term_slots[Dir::S]),
                &[format!("LV{i}")],
            )
        })
        .collect();
    for i in 0..ll_len {
        builder.conn_branch(lv[i], Dir::N, lv[(i + ll_len - 1) % ll_len]);
    }

    // The set/reset inputs.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[
                format!("SR{i}"),
                format!("IOIS_SR{i}"),
                format!("IOIS_OSR{i}"),
                format!("CNR_SR{i}"),
                format!("BRAM_SR{i}"),
                format!("MACC_SR{i}"),
            ],
        );
    }

    // The clock inputs.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CLK{i}"),
            &[
                format!("CLK{i}"),
                format!("CNR_CLK{i}"),
                format!("BRAM_CLK{i}"),
                format!("MACC_CLK{i}"),
                // these have a different mux
                ["", "BRAM_IOIS_PSCLK", "BRAM_IOIS_CLKIN", "BRAM_IOIS_CLKFB"][i].to_string(),
                ["", "DCM_PSCLK", "DCM_CLKIN", "DCM_CLKFB"][i].to_string(),
                ["", "DCM_PSCLK_STUB", "DCM_CLKIN_STUB", "DCM_CLKFB_STUB"][i].to_string(),
            ],
        );
    }

    for i in 0..8 {
        builder.mux_out(
            format!("IMUX.IOCLK{i}"),
            &[
                format!("IOIS_CLK{i}"),
                if i % 2 == 0 {
                    format!("IOIS_ICLK{ii}", ii = i / 2)
                } else {
                    format!("IOIS_OCLK{ii}", ii = i / 2)
                },
            ],
        );
    }

    // The clock enables.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CE{i}"),
            &[
                format!("CE_B{i}"),
                format!("IOIS_CE_B{i}"),
                format!("IOIS_OCE_B{i}"),
                format!("CNR_CE_B{i}"),
                format!("BRAM_CE_B{i}"),
                format!("MACC_CE_B{i}"),
            ],
        );
    }

    for xy in ['X', 'Y'] {
        for i in 0..4 {
            let w = builder.mux_out(
                format!("IMUX.FAN.B{xy}{i}"),
                &[
                    format!("B{xy}{i}"),
                    format!("IOIS_FAN_B{xy}{i}"),
                    format!("CNR_B{xy}{i}"),
                    if rd.family == "spartan3adsp" {
                        format!("BRAM_B{xy}_B{i}")
                    } else {
                        format!("BRAM_FAN_B{xy}{i}")
                    },
                    format!("MACC_B{xy}_B{i}"),
                    format!("BRAM_IOIS_FAN_B{xy}{i}"),
                    format!("DCM_FAN_B{xy}{i}"),
                ],
            );
            let mut wires = vec![];
            if rd.family == "spartan3adsp" {
                wires.extend([format!("BRAM_FAN_B{xy}{i}"), format!("MACC_FAN_B{xy}{i}")]);
            }
            builder.buf(w, format!("IMUX.FAN.B{xy}{i}.BOUNCE"), &wires);
        }
    }

    let mut lr_di2 = None;
    for i in 0..32 {
        let w = builder.mux_out(
            format!("IMUX.DATA{i}"),
            &[
                format!("{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                format!("IOIS_{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                format!(
                    "TBIOIS_{}{}_B{}",
                    ["F", "G"][i >> 4],
                    (i >> 2 & 3) + 1,
                    i & 3
                ),
                format!(
                    "LRIOIS_{}{}_B{}",
                    ["F", "G"][i >> 4],
                    (i >> 2 & 3) + 1,
                    i & 3
                ),
                // FPGACORE
                [
                    "IOIS_IREV0",
                    "IOIS_IREV1",
                    "IOIS_IREV2",
                    "IOIS_IREV3",
                    "IOIS_OREV0",
                    "IOIS_OREV1",
                    "IOIS_OREV2",
                    "IOIS_OREV3",
                    "IOIS_ICE_B0",
                    "IOIS_ICE_B1",
                    "IOIS_ICE_B2",
                    "IOIS_ICE_B3",
                    "IOIS_ISR0",
                    "IOIS_ISR1",
                    "IOIS_ISR2",
                    "IOIS_ISR3",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "IOIS_O0",
                    "IOIS_O1",
                    "IOIS_O2",
                    "IOIS_O3",
                    "",
                    "",
                    "",
                    "",
                ][i]
                    .to_string(),
                format!("CNR_DATA_IN{i}"),
                [
                    "BRAM_DIA_B18",
                    "BRAM_MULTINA_B15",
                    "BRAM_MULTINB_B17",
                    "BRAM_DIA_B1",
                    "BRAM_ADDRB_B0",
                    "BRAM_DIB_B19",
                    "BRAM_DIB_B0",
                    "BRAM_ADDRA_B3",
                    "BRAM_DIA_B19",
                    "BRAM_DIPB_B",
                    "BRAM_MULTINA_B17",
                    "BRAM_DIA_B0",
                    "BRAM_ADDRB_B1",
                    "BRAM_DIB_B18",
                    "BRAM_DIB_B1",
                    "BRAM_ADDRA_B2",
                    "BRAM_DIA_B2",
                    "BRAM_MULTINA_B14",
                    "BRAM_MULTINB_B16",
                    "BRAM_DIA_B17",
                    "BRAM_ADDRA_B0",
                    "BRAM_DIB_B3",
                    "BRAM_DIB_B16",
                    "BRAM_ADDRB_B3",
                    "BRAM_DIA_B3",
                    "BRAM_DIPA_B",
                    "BRAM_MULTINA_B16",
                    "BRAM_DIA_B16",
                    "BRAM_ADDRA_B1",
                    "BRAM_DIB_B2",
                    "BRAM_DIB_B17",
                    "BRAM_ADDRB_B2",
                ][i]
                    .to_string(),
                // 3A DSP version
                [
                    "",
                    "BRAM_MULTINA_B1",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_MULTINA_B3",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_MULTINA_B0",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "BRAM_MULTINA_B2",
                    "",
                    "",
                    "",
                    "",
                    "",
                ][i]
                    .to_string(),
                [
                    "MACC_DIA_B18",
                    "MACC_MULTINA_B1",
                    "MACC_MULTINB_B17",
                    "MACC_DIA_B1",
                    "MACC_ADDRB_B0",
                    "MACC_DIB_B19",
                    "MACC_DIB_B0",
                    "MACC_ADDRA_B3",
                    "MACC_DIA_B19",
                    "MACC_DIPB_B",
                    "MACC_MULTINA_B3",
                    "MACC_DIA_B0",
                    "MACC_ADDRB_B1",
                    "MACC_DIB_B18",
                    "MACC_DIB_B1",
                    "MACC_ADDRA_B2",
                    "MACC_DIA_B2",
                    "MACC_MULTINA_B0",
                    "MACC_MULTINB_B16",
                    "MACC_DIA_B17",
                    "MACC_ADDRA_B0",
                    "MACC_DIB_B3",
                    "MACC_DIB_B16",
                    "MACC_ADDRB_B3",
                    "MACC_DIA_B3",
                    "MACC_DIPA_B",
                    "MACC_MULTINA_B2",
                    "MACC_DIA_B16",
                    "MACC_ADDRA_B1",
                    "MACC_DIB_B2",
                    "MACC_DIB_B17",
                    "MACC_ADDRB_B2",
                ][i]
                    .to_string(),
                format!(
                    "BRAM_IOIS_{}{}_B{}",
                    ["F", "G"][i >> 4],
                    (i >> 2 & 3) + 1,
                    i & 3
                ),
                format!("DCM_{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                [
                    "",
                    "",
                    "DCM_CTLSEL0_STUB",
                    "DCM_CTLSEL1_STUB",
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
                    "DCM_DSSEN_STUB",
                    "DCM_PSEN_STUB",
                    "DCM_PSINCDEC_STUB",
                    "DCM_RST_STUB",
                    "DCM_STSADRS1_STUB",
                    "DCM_STSADRS2_STUB",
                    "DCM_STSADRS3_STUB",
                    "DCM_STSADRS4_STUB",
                    "DCM_CTLMODE_STUB",
                    "DCM_FREEZEDLL_STUB",
                    "DCM_FREEZEDFS_STUB",
                    "DCM_STSADRS0_STUB",
                    "DCM_CTLSEL2_STUB",
                    "DCM_CTLOSC2_STUB",
                    "DCM_CTLOSC1_STUB",
                    "DCM_CTLG0_STUB",
                ][i]
                    .to_string(),
            ],
        );
        if i == 2 {
            lr_di2 = Some(w);
        }
    }

    for i in 0..8 {
        builder.logic_out(
            format!("OUT.FAN{i}"),
            &[
                // In CLBs, used for combinatorial outputs.
                ["X0", "X1", "X2", "X3", "Y0", "Y1", "Y2", "Y3"][i],
                [
                    "IOIS_X0", "IOIS_X1", "IOIS_X2", "IOIS_X3", "IOIS_Y0", "IOIS_Y1", "IOIS_Y2",
                    "IOIS_Y3",
                ][i],
                ["", "", "", "", "IOIS_I0", "IOIS_I1", "IOIS_I2", "IOIS_I3"][i],
                // In BRAM, used for low data outputs.
                [
                    "BRAM_DOA0",
                    "BRAM_DOA1",
                    "BRAM_DOA2",
                    "BRAM_DOA3",
                    "BRAM_DOB0",
                    "BRAM_DOB1",
                    "BRAM_DOB2",
                    "BRAM_DOB3",
                ][i],
                [
                    "MACC_DOA0",
                    "MACC_DOA1",
                    "MACC_DOA2",
                    "MACC_DOA3",
                    "MACC_DOB0",
                    "MACC_DOB1",
                    "MACC_DOB2",
                    "MACC_DOB3",
                ][i],
                [
                    "BRAM_IOIS_CLK270",
                    "BRAM_IOIS_CLK180",
                    "BRAM_IOIS_CLK90",
                    "BRAM_IOIS_CLK0",
                    "BRAM_IOIS_CLKFX180",
                    "BRAM_IOIS_CLKFX",
                    "BRAM_IOIS_CLK2X180",
                    "BRAM_IOIS_CLK2X",
                ][i],
                [
                    "DCM_CLK270",
                    "DCM_CLK180",
                    "DCM_CLK90",
                    "DCM_CLK0",
                    "DCM_CLKFX180",
                    "DCM_CLKFX",
                    "DCM_CLK2X180",
                    "DCM_CLK2X",
                ][i],
                &format!("CNR_D_O_FAN_B{i}")[..],
            ],
        );
    }

    for i in 0..16 {
        builder.logic_out(
            format!("OUT.SEC{i}"),
            &[
                [
                    "XB0", "XB1", "XB2", "XB3", "YB0", "YB1", "YB2", "YB3", "XQ0", "XQ1", "XQ2",
                    "XQ3", "YQ0", "YQ1", "YQ2", "YQ3",
                ][i],
                [
                    "", "", "", "", "", "", "", "", "IOIS_XQ0", "IOIS_XQ1", "IOIS_XQ2", "IOIS_XQ3",
                    "IOIS_YQ0", "IOIS_YQ1", "IOIS_YQ2", "IOIS_YQ3",
                ][i],
                [
                    "", "", "", "", "", "", "", "", "IOIS_IQ0", "IOIS_IQ1", "IOIS_IQ2", "IOIS_IQ3",
                    "", "", "", "",
                ][i],
                // sigh. this does not appear to actually be true.
                [
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
                    "",
                    "",
                    "MACC_DOPA",
                    "MACC_DOPB",
                    "",
                    "MACC_MOUT32",
                    "MACC_MOUT7",
                    "MACC_MOUT6",
                    "MACC_MOUT5",
                    "MACC_MOUT4",
                    "MACC_MOUT3",
                    "MACC_MOUT2",
                    "MACC_MOUT1",
                    "MACC_MOUT0",
                ][i],
                [
                    "BRAM_IOIS_PSDONE",
                    "BRAM_IOIS_CONCUR",
                    "BRAM_IOIS_LOCKED",
                    "BRAM_IOIS_CLKDV",
                    "BRAM_IOIS_STATUS4",
                    "BRAM_IOIS_STATUS5",
                    "BRAM_IOIS_STATUS6",
                    "BRAM_IOIS_STATUS7",
                    "BRAM_IOIS_STATUS0",
                    "BRAM_IOIS_STATUS1",
                    "BRAM_IOIS_STATUS2",
                    "BRAM_IOIS_STATUS3",
                    "BRAM_IOIS_PTE2OMUX0",
                    "BRAM_IOIS_PTE2OMUX1",
                    "BRAM_IOIS_PTE2OMUX2",
                    "BRAM_IOIS_PTE2OMUX3",
                ][i],
                [
                    "DCM_PSDONE",
                    "DCM_CONCUR",
                    "DCM_LOCKED",
                    "DCM_CLKDV",
                    "DCM_STATUS4",
                    "DCM_STATUS5",
                    "DCM_STATUS6",
                    "DCM_STATUS7",
                    "DCM_STATUS0",
                    "DCM_STATUS1",
                    "DCM_STATUS2",
                    "DCM_STATUS3",
                    "DCM_PTE2OMUX0",
                    "DCM_PTE2OMUX1",
                    "DCM_PTE2OMUX2",
                    "DCM_PTE2OMUX3",
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
                    "DCM_PTE2OMUX0_STUB",
                    "DCM_PTE2OMUX1_STUB",
                    "DCM_PTE2OMUX2_STUB",
                    "DCM_PTE2OMUX3_STUB",
                ][i],
                &format!("CNR_D_OUT_B{i}")[..],
            ],
        );
    }
    builder.stub_out("STUB_IOIS_X3");
    builder.stub_out("STUB_IOIS_Y3");
    builder.stub_out("STUB_IOIS_XQ3");
    builder.stub_out("STUB_IOIS_YQ3");

    for i in 0..4 {
        for j in 0..2 {
            builder.logic_out(
                format!("OUT.HALF{i}.{j}"),
                &[
                    [
                        "BRAM_DOA16",
                        "BRAM_DOA17",
                        "BRAM_DOA19",
                        "BRAM_DOA18",
                        "BRAM_DOB16",
                        "BRAM_DOB17",
                        "BRAM_DOB19",
                        "BRAM_DOB18",
                    ][i + j * 4],
                    [
                        "MACC_DOA16",
                        "MACC_DOA17",
                        "MACC_DOA19",
                        "MACC_DOA18",
                        "MACC_DOB16",
                        "MACC_DOB17",
                        "MACC_DOB19",
                        "MACC_DOB18",
                    ][i + j * 4],
                ],
            );
        }
    }

    for i in 0..4 {
        builder.mux_out(
            format!("CLK.IMUX.SEL{i}"),
            &[format!("CLKB_SELDUB{i}"), format!("CLKT_SELDUB{i}")],
        );
    }
    for i in 0..4 {
        builder.mux_out(
            format!("CLK.IMUX.CLK{i}"),
            &[format!("CLKB_CLKDUB{i}"), format!("CLKT_CLKDUB{i}")],
        );
    }
    for i in 0..4 {
        builder.logic_out(
            format!("CLK.OUT.{i}"),
            &[format!("CLKB_GCLK_MAIN{i}"), format!("CLKT_GCLK_MAIN{i}")],
        );
    }

    builder.extract_main_passes();

    let bels_int = [builder.bel_xy(bels::RLL, "RLL", 0, 0)];
    let bels_int_dcm = [
        builder.bel_xy(bels::RLL, "RLL", 0, 0),
        builder
            .bel_virtual(bels::PTE2OMUX0)
            .extra_int_in("CLKFB", &["BRAM_IOIS_CLKFB", "DCM_CLKFB", "DCM_CLKFB_STUB"])
            .extra_int_in(
                "CTLSEL1",
                &["BRAM_IOIS_F1_B3", "DCM_F1_B3", "DCM_CTLSEL1_STUB"],
            )
            .extra_int_in("RST", &["BRAM_IOIS_G1_B3", "DCM_G1_B3", "DCM_RST_STUB"])
            .extra_int_in(
                "STSADRS4",
                &["BRAM_IOIS_G2_B3", "DCM_G2_B3", "DCM_STSADRS4_STUB"],
            )
            .extra_int_in(
                "STSADRS0",
                &["BRAM_IOIS_G3_B3", "DCM_G3_B3", "DCM_STSADRS0_STUB"],
            )
            .extra_int_in("CTLGO", &["BRAM_IOIS_G4_B3", "DCM_G4_B3", "DCM_CTLG0_STUB"])
            .extra_int_out(
                "OUT",
                &["BRAM_IOIS_PTE2OMUX0", "DCM_PTE2OMUX0", "DCM_PTE2OMUX0_STUB"],
            ),
        builder
            .bel_virtual(bels::PTE2OMUX1)
            .extra_int_in("CLKIN", &["BRAM_IOIS_CLKIN", "DCM_CLKIN", "DCM_CLKIN_STUB"])
            .extra_int_in(
                "PSINCDEC",
                &["BRAM_IOIS_G1_B2", "DCM_G1_B2", "DCM_PSINCDEC_STUB"],
            )
            .extra_int_in(
                "STSADRS3",
                &["BRAM_IOIS_G2_B2", "DCM_G2_B2", "DCM_STSADRS3_STUB"],
            )
            .extra_int_in(
                "FREEZEDFS",
                &["BRAM_IOIS_G3_B2", "DCM_G3_B2", "DCM_FREEZEDFS_STUB"],
            )
            .extra_int_in(
                "CTLOSC1",
                &["BRAM_IOIS_G4_B2", "DCM_G4_B2", "DCM_CTLOSC1_STUB"],
            )
            .extra_int_out(
                "OUT",
                &["BRAM_IOIS_PTE2OMUX1", "DCM_PTE2OMUX1", "DCM_PTE2OMUX1_STUB"],
            ),
        builder
            .bel_virtual(bels::PTE2OMUX2)
            .extra_int_in("PSCLK", &["BRAM_IOIS_PSCLK", "DCM_PSCLK", "DCM_PSCLK_STUB"])
            .extra_int_in(
                "CTLSEL0",
                &["BRAM_IOIS_F1_B2", "DCM_F1_B2", "DCM_CTLSEL0_STUB"],
            )
            .extra_int_in("PSEN", &["BRAM_IOIS_G1_B1", "DCM_G1_B1", "DCM_PSEN_STUB"])
            .extra_int_in(
                "STSADRS2",
                &["BRAM_IOIS_G2_B1", "DCM_G2_B1", "DCM_STSADRS2_STUB"],
            )
            .extra_int_in(
                "FREEZEDLL",
                &["BRAM_IOIS_G3_B1", "DCM_G3_B1", "DCM_FREEZEDLL_STUB"],
            )
            .extra_int_in(
                "CTLOSC2",
                &["BRAM_IOIS_G4_B1", "DCM_G4_B1", "DCM_CTLOSC2_STUB"],
            )
            .extra_int_out(
                "OUT",
                &["BRAM_IOIS_PTE2OMUX2", "DCM_PTE2OMUX2", "DCM_PTE2OMUX2_STUB"],
            ),
        builder
            .bel_virtual(bels::PTE2OMUX3)
            .extra_int_in("DSSEN", &["BRAM_IOIS_G1_B0", "DCM_G1_B0", "DCM_DSSEN_STUB"])
            .extra_int_in(
                "STSADRS1",
                &["BRAM_IOIS_G2_B0", "DCM_G2_B0", "DCM_STSADRS1_STUB"],
            )
            .extra_int_in(
                "CTLMODE",
                &["BRAM_IOIS_G3_B0", "DCM_G3_B0", "DCM_CTLMODE_STUB"],
            )
            .extra_int_in(
                "CTLSEL2",
                &["BRAM_IOIS_G4_B0", "DCM_G4_B0", "DCM_CTLSEL2_STUB"],
            )
            .extra_int_out(
                "OUT",
                &["BRAM_IOIS_PTE2OMUX3", "DCM_PTE2OMUX3", "DCM_PTE2OMUX3_STUB"],
            ),
    ];

    builder.extract_node(tslots::INT, "CENTER", "INT.CLB", "INT.CLB", &bels_int);
    builder.extract_node(tslots::INT, "CENTER_SMALL", "INT.CLB", "INT.CLB", &bels_int);
    builder.extract_node(
        tslots::INT,
        "CENTER_SMALL_BRK",
        "INT.CLB",
        "INT.CLB.BRK",
        &bels_int,
    );
    if rd.family.starts_with("spartan3a") {
        builder.extract_node(
            tslots::INT,
            "LIOIS",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIOIS_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIOIS_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIOIS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIOIS_CLK_PCI_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIBUFS",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIBUFS_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIBUFS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIOIS",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIOIS_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIOIS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS_CLK_PCI_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BIOIS",
            "INT.IOI.S3A.TB",
            "INT.IOI.S3A.TB",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BIOIB",
            "INT.IOI.S3A.TB",
            "INT.IOI.S3A.TB",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "TIOIS",
            "INT.IOI.S3A.TB",
            "INT.IOI.S3A.TB",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "TIOIB",
            "INT.IOI.S3A.TB",
            "INT.IOI.S3A.TB",
            &bels_int,
        );
    } else if rd.family == "spartan3e" {
        builder.extract_node(tslots::INT, "LIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(
            tslots::INT,
            "LIOIS_BRK",
            "INT.IOI.S3E",
            "INT.IOI.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIOIS_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIOIS_CLK_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(tslots::INT, "LIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(
            tslots::INT,
            "LIBUFS_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "LIBUFS_CLK_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(tslots::INT, "RIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(
            tslots::INT,
            "RIOIS_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIOIS_CLK_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(tslots::INT, "RIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(
            tslots::INT,
            "RIBUFS_BRK",
            "INT.IOI.S3E",
            "INT.IOI.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "RIBUFS_CLK_PCI",
            "INT.IOI.S3E",
            "INT.IOI",
            &bels_int,
        );
        builder.extract_node(tslots::INT, "BIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(tslots::INT, "BIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(tslots::INT, "TIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node(tslots::INT, "TIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
    } else if rd.family == "fpgacore" {
        builder.extract_node(tslots::INT, "LIOIS", "INT.IOI.FC", "INT.IOI.FC", &bels_int);
        builder.extract_node(tslots::INT, "RIOIS", "INT.IOI.FC", "INT.IOI.FC", &bels_int);
        builder.extract_node(tslots::INT, "BIOIS", "INT.IOI.FC", "INT.IOI.FC", &bels_int);
        builder.extract_node(tslots::INT, "TIOIS", "INT.IOI.FC", "INT.IOI.FC", &bels_int);
    } else {
        // NOTE: could be unified by pulling extra muxes from CLB
        builder.extract_node(tslots::INT, "LIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
        builder.extract_node(tslots::INT, "RIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
        builder.extract_node(tslots::INT, "BIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
        builder.extract_node(tslots::INT, "TIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
    }
    // NOTE:
    // - S3/S3E/S3A could be unified by pulling some extra muxes from CLB
    // - S3A/S3ADSP adds VCC input to B[XY] and splits B[XY] to two nodes
    if rd.family == "spartan3adsp" {
        builder.extract_node(
            tslots::INT,
            "BRAM0_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM0_SMALL_BOT",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM1_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM2_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL_TOP",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL_BRK",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP.BRK",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC0_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.MACC",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC0_SMALL_BOT",
            "INT.BRAM.S3ADSP",
            "INT.MACC",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC1_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.MACC",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC2_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.MACC",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC3_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.MACC",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC3_SMALL_TOP",
            "INT.BRAM.S3ADSP",
            "INT.MACC",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "MACC3_SMALL_BRK",
            "INT.BRAM.S3ADSP",
            "INT.MACC.BRK",
            &bels_int,
        );
    } else if rd.family == "spartan3a" {
        builder.extract_node(
            tslots::INT,
            "BRAM0_SMALL",
            "INT.BRAM.S3A.03",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM0_SMALL_BOT",
            "INT.BRAM.S3A.03",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM1_SMALL",
            "INT.BRAM.S3A.12",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM2_SMALL",
            "INT.BRAM.S3A.12",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL",
            "INT.BRAM.S3A.03",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL_TOP",
            "INT.BRAM.S3A.03",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL_BRK",
            "INT.BRAM.S3A.03",
            "INT.BRAM.BRK",
            &bels_int,
        );
    } else if rd.family == "spartan3e" {
        builder.extract_node(
            tslots::INT,
            "BRAM0_SMALL",
            "INT.BRAM.S3E",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM1_SMALL",
            "INT.BRAM.S3E",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM2_SMALL",
            "INT.BRAM.S3E",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL",
            "INT.BRAM.S3E",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL_BRK",
            "INT.BRAM.S3E",
            "INT.BRAM.BRK",
            &bels_int,
        );
    } else {
        builder.extract_node(tslots::INT, "BRAM0", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node(tslots::INT, "BRAM1", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node(tslots::INT, "BRAM2", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node(tslots::INT, "BRAM3", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node(
            tslots::INT,
            "BRAM0_SMALL",
            "INT.BRAM.S3",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM1_SMALL",
            "INT.BRAM.S3",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM2_SMALL",
            "INT.BRAM.S3",
            "INT.BRAM",
            &bels_int,
        );
        builder.extract_node(
            tslots::INT,
            "BRAM3_SMALL",
            "INT.BRAM.S3",
            "INT.BRAM",
            &bels_int,
        );
    }
    builder.extract_node(
        tslots::INT,
        "BRAM_IOIS",
        "INT.DCM",
        "INT.DCM.S3",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "BRAM_IOIS_NODCM",
        "INT.DCM.S3.DUMMY",
        "INT.DCM.S3.DUMMY",
        &bels_int,
    );
    builder.extract_node(
        tslots::INT,
        "DCMAUX_BL_CENTER",
        "INT.DCM.S3E.DUMMY",
        "INT.DCM.S3E.DUMMY",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCMAUX_TL_CENTER",
        "INT.DCM.S3E.DUMMY",
        "INT.DCM.S3E.DUMMY",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_BL_CENTER",
        "INT.DCM",
        "INT.DCM.S3E",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_TL_CENTER",
        "INT.DCM",
        "INT.DCM.S3E",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_BR_CENTER",
        "INT.DCM",
        "INT.DCM.S3E",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_TR_CENTER",
        "INT.DCM",
        "INT.DCM.S3E",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_H_BL_CENTER",
        "INT.DCM",
        "INT.DCM.S3E.H",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_H_TL_CENTER",
        "INT.DCM",
        "INT.DCM.S3E.H",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_H_BR_CENTER",
        "INT.DCM",
        "INT.DCM.S3E.H",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_H_TR_CENTER",
        "INT.DCM",
        "INT.DCM.S3E.H",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_BGAP",
        "INT.DCM",
        "INT.DCM.S3E.H",
        &bels_int_dcm,
    );
    builder.extract_node(
        tslots::INT,
        "DCM_SPLY",
        "INT.DCM",
        "INT.DCM.S3E.H",
        &bels_int_dcm,
    );
    builder.extract_node(tslots::INT, "LL", "INT.CLB", "INT.CNR", &bels_int);
    builder.extract_node(tslots::INT, "LR", "INT.CLB", "INT.CNR", &bels_int);
    builder.extract_node(tslots::INT, "UL", "INT.CLB", "INT.CNR", &bels_int);
    builder.extract_node(tslots::INT, "UR", "INT.CLB", "INT.CNR", &bels_int);

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    let bels_clb = [
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
    ];
    builder.extract_node_bels(tslots::BEL, "CENTER", "CLB", "CLB", &bels_clb);
    builder.extract_node_bels(tslots::BEL, "CENTER_SMALL", "CLB", "CLB", &bels_clb);
    builder.extract_node_bels(tslots::BEL, "CENTER_SMALL_BRK", "CLB", "CLB", &bels_clb);

    let ioi_name_only = [
        "DIFFI_IN",
        "PADOUT",
        "DIFFO_IN",
        "DIFFO_OUT",
        "IDDRIN1",
        "IDDRIN2",
        "ODDRIN1",
        "ODDRIN2",
        "ODDROUT1",
        "ODDROUT2",
        "PCI_RDY",
        "TAUX",
        "OAUX",
    ];
    if rd.family == "spartan3" {
        let bels_ioi = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bels::IO2, "IOB", 2)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_node_bels(tslots::BEL, "LIOIS", "IOI.S3", "IOI.S3.L", &bels_ioi);
        builder.extract_node_bels(tslots::BEL, "RIOIS", "IOI.S3", "IOI.S3.R", &bels_ioi);
        builder.extract_node_bels(tslots::BEL, "BIOIS", "IOI.S3", "IOI.S3.B", &bels_ioi);
        builder.extract_node_bels(tslots::BEL, "TIOIS", "IOI.S3", "IOI.S3.T", &bels_ioi);
        for (kind, num) in [
            ("IOBS.S3.B2", 2),
            ("IOBS.S3.T2", 2),
            ("IOBS.S3.L1", 1),
            ("IOBS.S3.R1", 1),
        ] {
            builder.make_marker_node(tslots::IOB, kind, num);
        }
    } else if rd.family == "fpgacore" {
        let bels_ioi = [
            builder
                .bel_indexed(bels::IBUF0, "IBUF", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0"),
            builder
                .bel_indexed(bels::IBUF1, "IBUF", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1"),
            builder.bel_indexed(bels::IBUF2, "IBUF", 2),
            builder.bel_indexed(bels::IBUF3, "IBUF", 3),
            builder.bel_indexed(bels::OBUF0, "OBUF", 0),
            builder.bel_indexed(bels::OBUF1, "OBUF", 1),
            builder.bel_indexed(bels::OBUF2, "OBUF", 2),
            builder.bel_indexed(bels::OBUF3, "OBUF", 3),
        ];
        builder.extract_node_bels(tslots::BEL, "LIOIS", "IOI.FC", "IOI.FC.L", &bels_ioi);
        builder.extract_node_bels(tslots::BEL, "RIOIS", "IOI.FC", "IOI.FC.R", &bels_ioi);
        builder.extract_node_bels(tslots::BEL, "BIOIS", "IOI.FC", "IOI.FC.B", &bels_ioi);
        builder.extract_node_bels(tslots::BEL, "TIOIS", "IOI.FC", "IOI.FC.T", &bels_ioi);
        for (kind, num) in [
            ("IOBS.FC.B", 1),
            ("IOBS.FC.T", 1),
            ("IOBS.FC.L", 1),
            ("IOBS.FC.R", 1),
        ] {
            builder.make_marker_node(tslots::IOB, kind, num);
        }
    } else if rd.family == "spartan3e" {
        let bels_ioi_tb = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bels::IO2, "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed(bels::IO2, "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_wire_force("IBUF", "RIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_wire_force("IBUF", "RIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed(bels::IO2, "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_node_bels(tslots::BEL, "LIOIS", "IOI.S3E", "IOI.S3E.L", &bels_ioi_l);
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_BRK",
            "IOI.S3E",
            "IOI.S3E.L",
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_PCI",
            "IOI.S3E",
            "IOI.S3E.L.PCI.PCI",
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_CLK_PCI",
            "IOI.S3E",
            "IOI.S3E.L.PCI.PCI",
            &bels_ioi_l,
        );
        builder.extract_node_bels(tslots::BEL, "LIBUFS", "IOI.S3E", "IOI.S3E.L", &bels_ioi_l);
        builder.extract_node_bels(
            tslots::BEL,
            "LIBUFS_PCI",
            "IOI.S3E",
            "IOI.S3E.L.PCI",
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIBUFS_CLK_PCI",
            "IOI.S3E",
            "IOI.S3E.L.PCI",
            &bels_ioi_l,
        );
        builder.extract_node_bels(tslots::BEL, "RIOIS", "IOI.S3E", "IOI.S3E.R", &bels_ioi_r);
        builder.extract_node_bels(
            tslots::BEL,
            "RIOIS_PCI",
            "IOI.S3E",
            "IOI.S3E.R.PCI.PCI",
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIOIS_CLK_PCI",
            "IOI.S3E",
            "IOI.S3E.R.PCI.PCI",
            &bels_ioi_r,
        );
        builder.extract_node_bels(tslots::BEL, "RIBUFS", "IOI.S3E", "IOI.S3E.R", &bels_ioi_r);
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_BRK",
            "IOI.S3E",
            "IOI.S3E.R",
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_PCI",
            "IOI.S3E",
            "IOI.S3E.R.PCI",
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_CLK_PCI",
            "IOI.S3E",
            "IOI.S3E.R.PCI",
            &bels_ioi_r,
        );
        builder.extract_node_bels(tslots::BEL, "BIOIS", "IOI.S3E", "IOI.S3E.B", &bels_ioi_tb);
        builder.extract_node_bels(tslots::BEL, "BIBUFS", "IOI.S3E", "IOI.S3E.B", &bels_ioi_tb);
        builder.extract_node_bels(tslots::BEL, "TIOIS", "IOI.S3E", "IOI.S3E.T", &bels_ioi_tb);
        builder.extract_node_bels(tslots::BEL, "TIBUFS", "IOI.S3E", "IOI.S3E.T", &bels_ioi_tb);
        for (kind, num) in [
            ("IOBS.S3E.B1", 1),
            ("IOBS.S3E.B2", 2),
            ("IOBS.S3E.B3", 3),
            ("IOBS.S3E.B4", 4),
            ("IOBS.S3E.T1", 1),
            ("IOBS.S3E.T2", 2),
            ("IOBS.S3E.T3", 3),
            ("IOBS.S3E.T4", 4),
            ("IOBS.S3E.L1", 1),
            ("IOBS.S3E.L2", 2),
            ("IOBS.S3E.L3", 3),
            ("IOBS.S3E.L4", 4),
            ("IOBS.S3E.R1", 1),
            ("IOBS.S3E.R2", 2),
            ("IOBS.S3E.R3", 3),
            ("IOBS.S3E.R4", 4),
        ] {
            builder.make_marker_node(tslots::IOB, kind, num);
        }
    } else {
        let bels_ioi_tb = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bels::IO2, "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed(bels::IO0, "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "RIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bels::IO1, "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "RIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
        ];
        let naming_l;
        let naming_r;
        let naming_l_pci;
        let naming_r_pci;
        let naming_b;
        let naming_t;
        if rd.family == "spartan3adsp" {
            naming_l = "IOI.S3ADSP.L";
            naming_r = "IOI.S3ADSP.R";
            naming_l_pci = "IOI.S3ADSP.L.PCI";
            naming_r_pci = "IOI.S3ADSP.R.PCI";
            naming_b = "IOI.S3ADSP.B";
            naming_t = "IOI.S3ADSP.T";
        } else {
            naming_l = "IOI.S3A.L";
            naming_r = "IOI.S3A.R";
            naming_l_pci = "IOI.S3A.L.PCI";
            naming_r_pci = "IOI.S3A.R.PCI";
            naming_b = "IOI.S3A.B";
            naming_t = "IOI.S3A.T";
        }
        builder.extract_node_bels(tslots::BEL, "LIOIS", "IOI.S3A.LR", naming_l, &bels_ioi_l);
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_BRK",
            "IOI.S3A.LR",
            naming_l,
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_PCI",
            "IOI.S3A.LR",
            naming_l_pci,
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_CLK_PCI",
            "IOI.S3A.LR",
            naming_l_pci,
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIOIS_CLK_PCI_BRK",
            "IOI.S3A.LR",
            naming_l_pci,
            &bels_ioi_l,
        );
        builder.extract_node_bels(tslots::BEL, "LIBUFS", "IOI.S3A.LR", naming_l, &bels_ioi_l);
        builder.extract_node_bels(
            tslots::BEL,
            "LIBUFS_PCI",
            "IOI.S3A.LR",
            naming_l,
            &bels_ioi_l,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LIBUFS_CLK_PCI",
            "IOI.S3A.LR",
            naming_l,
            &bels_ioi_l,
        );
        builder.extract_node_bels(tslots::BEL, "RIOIS", "IOI.S3A.LR", naming_r, &bels_ioi_r);
        builder.extract_node_bels(
            tslots::BEL,
            "RIOIS_PCI",
            "IOI.S3A.LR",
            naming_r_pci,
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIOIS_CLK_PCI",
            "IOI.S3A.LR",
            naming_r_pci,
            &bels_ioi_r,
        );
        builder.extract_node_bels(tslots::BEL, "RIBUFS", "IOI.S3A.LR", naming_r, &bels_ioi_r);
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_BRK",
            "IOI.S3A.LR",
            naming_r,
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_PCI",
            "IOI.S3A.LR",
            naming_r,
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_CLK_PCI",
            "IOI.S3A.LR",
            naming_r,
            &bels_ioi_r,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "RIBUFS_CLK_PCI_BRK",
            "IOI.S3A.LR",
            naming_r,
            &bels_ioi_r,
        );
        builder.extract_node_bels(tslots::BEL, "BIOIS", "IOI.S3A.B", naming_b, &bels_ioi_tb);
        builder.extract_node_bels(tslots::BEL, "BIOIB", "IOI.S3A.B", naming_b, &bels_ioi_tb);
        builder.extract_node_bels(tslots::BEL, "TIOIS", "IOI.S3A.T", naming_t, &bels_ioi_tb);
        builder.extract_node_bels(tslots::BEL, "TIOIB", "IOI.S3A.T", naming_t, &bels_ioi_tb);
        for (kind, num) in [
            ("IOBS.S3A.B2", 2),
            ("IOBS.S3A.T2", 2),
            ("IOBS.S3A.L4", 4),
            ("IOBS.S3A.R4", 4),
        ] {
            builder.make_marker_node(tslots::IOB, kind, num);
        }
    }
    if rd.family != "fpgacore" {
        let bels_randor_b = [builder
            .bel_xy(bels::RANDOR, "RANDOR", 0, 0)
            .pins_name_only(&["CIN0", "CIN1", "CPREV", "O"])];
        builder.extract_node_bels(
            tslots::RANDOR,
            "BIOIS",
            "RANDOR",
            "RANDOR.B",
            &bels_randor_b,
        );
        builder.extract_node_bels(
            tslots::RANDOR,
            "BIOIB",
            "RANDOR",
            "RANDOR.B",
            &bels_randor_b,
        );
        builder.extract_node_bels(
            tslots::RANDOR,
            "BIBUFS",
            "RANDOR",
            "RANDOR.B",
            &bels_randor_b,
        );
    }
    let bels_randor_t = [builder
        .bel_xy(bels::RANDOR, "RANDOR", 0, 0)
        .pins_name_only(&["CIN0", "CIN1"])
        .pin_name_only("CPREV", 1)
        .pin_name_only("O", 1)];
    builder.extract_node_bels(
        tslots::RANDOR,
        "TIOIS",
        "RANDOR",
        "RANDOR.T",
        &bels_randor_t,
    );
    builder.extract_node_bels(
        tslots::RANDOR,
        "TIOIB",
        "RANDOR",
        "RANDOR.T",
        &bels_randor_t,
    );
    builder.extract_node_bels(
        tslots::RANDOR,
        "TIBUFS",
        "RANDOR",
        "RANDOR.T",
        &bels_randor_t,
    );
    builder.make_marker_node(tslots::RANDOR, "RANDOR_INIT", 0);
    if rd.family == "spartan3" {
        let bels_dcm = [builder.bel_xy(bels::DCM, "DCM", 0, 0)];
        builder.extract_node_bels(tslots::BEL, "BRAM_IOIS", "DCM.S3", "DCM.S3", &bels_dcm);
    } else if rd.family != "fpgacore" {
        let bels_dcm = [
            builder.bel_xy(bels::DCM, "DCM", 0, 0),
            builder
                .bel_virtual(bels::DCMCONN_S3E)
                .extra_int_out("CLKPAD0", &["DCM_DLL_CLKPAD0", "DCM_H_DLL_CLKPAD0"])
                .extra_int_out("CLKPAD1", &["DCM_DLL_CLKPAD1", "DCM_H_DLL_CLKPAD1"])
                .extra_int_out("CLKPAD2", &["DCM_DLL_CLKPAD2", "DCM_H_DLL_CLKPAD2"])
                .extra_int_out("CLKPAD3", &["DCM_DLL_CLKPAD3", "DCM_H_DLL_CLKPAD3"])
                .extra_int_in(
                    "OUT0",
                    &[
                        "DCM_OMUX10_CLKOUT0",
                        "DCM_OMUX10_CLKOUTL0",
                        "DCM_OMUX10_CLKOUTR0",
                    ],
                )
                .extra_int_in(
                    "OUT1",
                    &[
                        "DCM_OMUX11_CLKOUT1",
                        "DCM_OMUX11_CLKOUTL1",
                        "DCM_OMUX11_CLKOUTR1",
                    ],
                )
                .extra_int_in(
                    "OUT2",
                    &[
                        "DCM_OMUX12_CLKOUT2",
                        "DCM_OMUX12_CLKOUTL2",
                        "DCM_OMUX12_CLKOUTR2",
                    ],
                )
                .extra_int_in(
                    "OUT3",
                    &[
                        "DCM_OMUX15_CLKOUT3",
                        "DCM_OMUX15_CLKOUTL3",
                        "DCM_OMUX15_CLKOUTR3",
                    ],
                ),
        ];
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_BL_CENTER",
            "DCM.S3E.BL",
            "DCM.S3E.L",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_TL_CENTER",
            "DCM.S3E.TL",
            "DCM.S3E.L",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_BR_CENTER",
            "DCM.S3E.BR",
            "DCM.S3E.R",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_TR_CENTER",
            "DCM.S3E.TR",
            "DCM.S3E.R",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_H_BL_CENTER",
            "DCM.S3E.LB",
            "DCM.S3E.H",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_H_TL_CENTER",
            "DCM.S3E.LT",
            "DCM.S3E.H",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_H_BR_CENTER",
            "DCM.S3E.RB",
            "DCM.S3E.H",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_H_TR_CENTER",
            "DCM.S3E.RT",
            "DCM.S3E.H",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_BGAP",
            "DCM.S3E.LB",
            "DCM.S3E.H",
            &bels_dcm,
        );
        builder.extract_node_bels(
            tslots::BEL,
            "DCM_SPLY",
            "DCM.S3E.LT",
            "DCM.S3E.H",
            &bels_dcm,
        );
    }

    if rd.family == "spartan3" {
        builder.extract_node_bels(
            tslots::BEL,
            "LL",
            "LL.S3",
            "LL.S3",
            &[
                builder.bel_indexed(bels::DCI0, "DCI", 6),
                builder.bel_indexed(bels::DCI1, "DCI", 5),
                builder.bel_indexed(bels::DCIRESET0, "DCIRESET", 6),
                builder.bel_indexed(bels::DCIRESET1, "DCIRESET", 5),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "LR",
            "LR.S3",
            "LR.S3",
            &[
                builder.bel_indexed(bels::DCI0, "DCI", 3),
                builder.bel_indexed(bels::DCI1, "DCI", 4),
                builder.bel_indexed(bels::DCIRESET0, "DCIRESET", 3),
                builder.bel_indexed(bels::DCIRESET1, "DCIRESET", 4),
                builder.bel_single(bels::STARTUP, "STARTUP"),
                builder.bel_single(bels::CAPTURE, "CAPTURE"),
                builder.bel_single(bels::ICAP, "ICAP"),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UL",
            "UL.S3",
            "UL.S3",
            &[
                builder.bel_indexed(bels::DCI0, "DCI", 7),
                builder.bel_indexed(bels::DCI1, "DCI", 0),
                builder.bel_indexed(bels::DCIRESET0, "DCIRESET", 7),
                builder.bel_indexed(bels::DCIRESET1, "DCIRESET", 0),
                builder.bel_single(bels::PMV, "PMV"),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UR",
            "UR.S3",
            "UR.S3",
            &[
                builder.bel_indexed(bels::DCI0, "DCI", 2),
                builder.bel_indexed(bels::DCI1, "DCI", 1),
                builder.bel_indexed(bels::DCIRESET0, "DCIRESET", 2),
                builder.bel_indexed(bels::DCIRESET1, "DCIRESET", 1),
                builder.bel_single(bels::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bels::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
    } else if rd.family == "fpgacore" {
        builder.extract_node_bels(tslots::BEL, "LL", "LL.FC", "LL.FC", &[]);
        builder.extract_node_bels(
            tslots::BEL,
            "LR",
            "LR.FC",
            "LR.FC",
            &[
                builder.bel_single(bels::STARTUP, "STARTUP"),
                builder.bel_single(bels::CAPTURE, "CAPTURE"),
                builder.bel_single(bels::ICAP, "ICAP"),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UL",
            "UL.FC",
            "UL.FC",
            &[builder.bel_single(bels::PMV, "PMV")],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UR",
            "UR.FC",
            "UR.FC",
            &[
                builder.bel_single(bels::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bels::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
        for tile in ["LL.FC", "UL.FC", "LR.FC", "UR.FC"] {
            let mut bel = BelInfo::default();
            bel.pins.insert(
                "CLK".into(),
                BelPin {
                    wires: BTreeSet::from_iter([(
                        TileCellId::from_idx(0),
                        builder.db.get_wire("IMUX.CLK3"),
                    )]),
                    dir: PinDir::Input,
                    is_intf_in: false,
                },
            );
            let node = builder.db.tile_classes.get_mut(tile).unwrap().1;
            node.bels.insert(bels::MISR, bel);
            let pin_naming = BelPinNaming {
                name: "CNR_CLK3".into(),
                name_far: "CNR_CLK3".into(),
                pips: vec![],
                int_pips: BTreeMap::new(),
                is_intf_out: false,
            };
            let mut bel_naming = BelNaming {
                tile: RawTileId::from_idx(0),
                pins: BTreeMap::new(),
            };
            bel_naming.pins.insert("CLK".into(), pin_naming);
            let naming = builder.ndb.tile_class_namings.get_mut(tile).unwrap().1;
            naming.bels.insert(bels::MISR, bel_naming);
        }
    } else if rd.family == "spartan3e" {
        builder.extract_node_bels(tslots::BEL, "LL", "LL.S3E", "LL.S3E", &[]);
        builder.extract_node_bels(
            tslots::BEL,
            "LR",
            "LR.S3E",
            "LR.S3E",
            &[
                builder.bel_single(bels::STARTUP, "STARTUP"),
                builder.bel_single(bels::CAPTURE, "CAPTURE"),
                builder.bel_single(bels::ICAP, "ICAP").pin_force_int(
                    "I2",
                    (TileCellId::from_idx(0), lr_di2.unwrap()),
                    "CNR_DATA_IN2",
                ),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UL",
            "UL.S3E",
            "UL.S3E",
            &[builder.bel_single(bels::PMV, "PMV")],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UR",
            "UR.S3E",
            "UR.S3E",
            &[
                builder.bel_single(bels::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bels::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
    } else {
        builder.extract_node_bels(tslots::BEL, "LL", "LL.S3A", "LL.S3A", &[]);
        builder.extract_node_bels(
            tslots::BEL,
            "LR",
            "LR.S3A",
            "LR.S3A",
            &[
                builder.bel_single(bels::STARTUP, "STARTUP"),
                builder.bel_single(bels::CAPTURE, "CAPTURE"),
                builder.bel_single(bels::ICAP, "ICAP"),
                builder.bel_single(bels::SPI_ACCESS, "SPI_ACCESS"),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UL",
            "UL.S3A",
            "UL.S3A",
            &[
                builder.bel_single(bels::PMV, "PMV"),
                builder.bel_single(bels::DNA_PORT, "DNA_PORT"),
            ],
        );
        builder.extract_node_bels(
            tslots::BEL,
            "UR",
            "UR.S3A",
            "UR.S3A",
            &[
                builder.bel_single(bels::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bels::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
    }

    for tkn in [
        "LTERM",
        "LTERM1",
        "LTERM2",
        "LTERM3",
        "LTERM4",
        "LTERM4B",
        "LTERM4CLK",
        "LTERMCLK",
        "LTERMCLKA",
        "CNR_LBTERM",
        "CNR_LTTERM",
    ] {
        builder.extract_term("TERM.W", None, Dir::W, tkn, "TERM.W");
    }
    for tkn in [
        "RTERM",
        "RTERM1",
        "RTERM2",
        "RTERM3",
        "RTERM4",
        "RTERM4B",
        "RTERM4CLK",
        "RTERM4CLKB",
        "RTERMCLKA",
        "RTERMCLKB",
        "CNR_RBTERM",
        "CNR_RTTERM",
    ] {
        builder.extract_term("TERM.E", None, Dir::E, tkn, "TERM.E");
    }
    for tkn in [
        "BTERM",
        "BTERM1",
        "BTERM1_MACC",
        "BTERM2",
        "BTERM2CLK",
        "BTERM3",
        "BTERM4",
        "BTERM4CLK",
        "BTERM4_BRAM2",
        "BTERMCLK",
        "BTERMCLKA",
        "BTERMCLKB",
        "BCLKTERM2",
        "BCLKTERM3",
        "BBTERM",
    ] {
        builder.extract_term("TERM.S", None, Dir::S, tkn, "TERM.S");
    }
    for tkn in [
        "TTERM",
        "TTERM1",
        "TTERM1_MACC",
        "TTERM2",
        "TTERM2CLK",
        "TTERM3",
        "TTERM4",
        "TTERM4CLK",
        "TTERM4_BRAM2",
        "TTERMCLK",
        "TTERMCLKA",
        "TCLKTERM2",
        "TCLKTERM3",
        "BTTERM",
    ] {
        builder.extract_term("TERM.N", None, Dir::N, tkn, "TERM.N");
    }
    builder.extract_term("TERM.S", None, Dir::S, "CNR_BTERM", "TERM.S.CNR");
    builder.extract_term("TERM.N", None, Dir::N, "CNR_TTERM", "TERM.N.CNR");

    if rd.family == "spartan3e" {
        let cob_term_t_y = rd.tile_kinds.get("COB_TERM_T").unwrap().1.tiles[0].y;
        for &xy_b in &rd.tile_kinds.get("COB_TERM_B").unwrap().1.tiles {
            let xy_t = Coord {
                x: xy_b.x,
                y: cob_term_t_y,
            };
            let int_s_xy = builder.walk_to_int(xy_b, Dir::S, false).unwrap();
            let int_n_xy = builder.walk_to_int(xy_t, Dir::N, false).unwrap();
            builder.extract_pass_tile(
                "TERM.BRAM.S",
                Dir::S,
                int_n_xy,
                Some(xy_t),
                None,
                Some("TERM.BRAM.S"),
                None,
                None,
                int_s_xy,
                &lv,
            );
            builder.extract_pass_tile(
                "TERM.BRAM.N",
                Dir::N,
                int_s_xy,
                Some(xy_b),
                None,
                Some("TERM.BRAM.N"),
                None,
                None,
                int_n_xy,
                &lv,
            );
        }
        for tkn in ["CLKL_IOIS", "CLKR_IOIS"] {
            builder.extract_pass_simple("CLKLR.S3E", Dir::S, tkn, &[]);
        }
    }
    if rd.family == "spartan3" {
        builder.extract_pass_simple("BRKH.S3", Dir::S, "BRKH", &[]);
    }
    for (tkn, naming) in [
        ("CLKH_LL", "LLV"),
        ("CLKH_DCM_LL", "LLV"),
        ("CLKLH_DCM_LL", "LLV"),
        ("CLKRH_DCM_LL", "LLV"),
        ("CLKL_IOIS_LL", "LLV.CLKL"),
        ("CLKR_IOIS_LL", "LLV.CLKR"),
    ] {
        let mut llv_s = "LLV.S";
        let mut llv_n = "LLV.N";
        if rd.family != "spartan3a" && naming != "LLV" {
            llv_s = "LLV.CLKLR.S3E.S";
            llv_n = "LLV.CLKLR.S3E.N";
        }
        let node = if rd.family == "spartan3e" {
            "LLV.S3E"
        } else {
            "LLV.S3A"
        };
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::N, false).unwrap();
            builder.extract_pass_tile(
                llv_s,
                Dir::S,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                None,
                Some((tslots::VTERM, node, naming)),
                int_fwd_xy,
                &[],
            );
            builder.extract_pass_tile(
                llv_n,
                Dir::N,
                int_fwd_xy,
                Some(xy),
                None,
                None,
                None,
                None,
                int_bwd_xy,
                &[],
            );
        }
    }
    for (node, tkn) in [
        ("LLH", "CLKV_DCM_LL"),
        ("LLH", "CLKV_LL"),
        (
            if rd.family == "spartan3e" {
                "LLH"
            } else {
                "LLH.CLKT.S3A"
            },
            "CLKT_LL",
        ),
        (
            if rd.family == "spartan3e" {
                "LLH"
            } else {
                "LLH.CLKB.S3A"
            },
            "CLKB_LL",
        ),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let fix_xy = if tkn == "CLKB_LL" { xy.delta(0, 1) } else { xy };
            let int_fwd_xy = builder.walk_to_int(fix_xy, Dir::W, false).unwrap();
            let int_bwd_xy = builder.walk_to_int(fix_xy, Dir::E, false).unwrap();
            let mut llh_w = "LLH.W";
            let mut llh_e = "LLH.E";
            if rd.family == "spartan3adsp" && tkn == "CLKV_DCM_LL" {
                llh_w = "LLH.DCM.S3ADSP.W";
                llh_e = "LLH.DCM.S3ADSP.E";
            }
            builder.extract_pass_tile(
                llh_w,
                Dir::W,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                None,
                Some((tslots::HTERM, node, "LLH")),
                int_fwd_xy,
                &[],
            );
            builder.extract_pass_tile(
                llh_e,
                Dir::E,
                int_fwd_xy,
                Some(xy),
                None,
                None,
                None,
                None,
                int_bwd_xy,
                &[],
            );
        }
    }
    if rd.family == "spartan3adsp" {
        for tkn in ["EMPTY_TIOI", "EMPTY_BIOI"] {
            builder.extract_pass_simple("DSPHOLE", Dir::W, tkn, &lh);
        }
        for &xy in rd.tiles_by_kind_name("DCM_BGAP") {
            let mut int_w_xy = xy;
            let mut int_e_xy = xy;
            int_e_xy.x += 5;
            builder.extract_pass_tile(
                "DSPHOLE.W",
                Dir::W,
                int_e_xy,
                None,
                None,
                None,
                None,
                None,
                int_w_xy,
                &lh,
            );
            builder.extract_pass_tile(
                "DSPHOLE.E",
                Dir::E,
                int_w_xy,
                None,
                None,
                None,
                None,
                None,
                int_e_xy,
                &lh,
            );
            int_w_xy.x -= 1;
            for _ in 0..3 {
                int_w_xy.y -= 1;
                int_e_xy.y -= 1;
                builder.extract_pass_tile(
                    "HDCM.W",
                    Dir::W,
                    int_e_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_w_xy,
                    &lh,
                );
                builder.extract_pass_tile(
                    "HDCM.E",
                    Dir::E,
                    int_w_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_e_xy,
                    &lh,
                );
            }
        }
        for &xy in rd.tiles_by_kind_name("DCM_SPLY") {
            let mut int_w_xy = xy;
            let mut int_e_xy = xy;
            int_e_xy.x += 5;
            builder.extract_pass_tile(
                "DSPHOLE.W",
                Dir::W,
                int_e_xy,
                None,
                None,
                None,
                None,
                None,
                int_w_xy,
                &lh,
            );
            builder.extract_pass_tile(
                "DSPHOLE.E",
                Dir::E,
                int_w_xy,
                None,
                None,
                None,
                None,
                None,
                int_e_xy,
                &lh,
            );
            int_w_xy.x -= 1;
            for _ in 0..3 {
                int_w_xy.y += 1;
                int_e_xy.y += 1;
                builder.extract_pass_tile(
                    "HDCM.W",
                    Dir::W,
                    int_e_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_w_xy,
                    &lh,
                );
                builder.extract_pass_tile(
                    "HDCM.E",
                    Dir::E,
                    int_w_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_e_xy,
                    &lh,
                );
            }
        }
    }

    for tkn in ["CLKB", "CLKB_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(
                -1,
                if rd.family == "spartan3" || rd.family == "fpgacore" {
                    0
                } else {
                    1
                },
            );
            if rd.family == "spartan3" {
                builder.extract_xnode(
                    tslots::CLK,
                    "CLKB.S3",
                    xy,
                    &[],
                    &[xy_l],
                    "CLKB.S3",
                    &[
                        builder
                            .bel_indexed(bels::BUFGMUX0, "BUFGMUX", 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI0"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD0"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL0"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR0"])
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_indexed(bels::BUFGMUX1, "BUFGMUX", 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI1"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD1"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL1"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR1"])
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_indexed(bels::BUFGMUX2, "BUFGMUX", 2)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI2"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD2"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL2"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR2"])
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_indexed(bels::BUFGMUX3, "BUFGMUX", 3)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI3"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD3"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL3"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR3"])
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual(bels::GLOBALSIG_S),
                    ],
                    &lh,
                );
            } else if rd.family == "fpgacore" {
                builder.extract_xnode(
                    tslots::CLK,
                    "CLKB.FC",
                    xy,
                    &[],
                    &[xy_l],
                    "CLKB.FC",
                    &[
                        builder
                            .bel_indexed(bels::BUFGMUX0, "BUFG", 0)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI0"])
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_indexed(bels::BUFGMUX1, "BUFG", 1)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI1"])
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_indexed(bels::BUFGMUX2, "BUFG", 2)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI2"])
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_indexed(bels::BUFGMUX3, "BUFG", 3)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI3"])
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual(bels::GLOBALSIG_S),
                    ],
                    &lh,
                );
            } else {
                let kind = if rd.family == "spartan3e" {
                    "CLKB.S3E"
                } else {
                    "CLKB.S3A"
                };
                builder.extract_xnode(
                    tslots::CLK,
                    kind,
                    xy,
                    &[],
                    &[xy_l],
                    kind,
                    &[
                        builder
                            .bel_xy(bels::BUFGMUX0, "BUFGMUX", 1, 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI0"])
                            .extra_wire("CKIL", &["CLKB_CKI4"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD7")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD0")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL0",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTL0".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR0",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTR0".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_xy(bels::BUFGMUX1, "BUFGMUX", 1, 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI1"])
                            .extra_wire("CKIL", &["CLKB_CKI5"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD6")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD1")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL1",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTL1".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR1",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTR1".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_xy(bels::BUFGMUX2, "BUFGMUX", 0, 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI2"])
                            .extra_wire("CKIL", &["CLKB_CKI6"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD5")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD2")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL2",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTL2".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR2",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTR2".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_xy(bels::BUFGMUX3, "BUFGMUX", 0, 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI3"])
                            .extra_wire("CKIL", &["CLKB_CKI7"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD4")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD3")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL3",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTL3".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR3",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTR3".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual(bels::GLOBALSIG_S),
                    ],
                    &lh,
                );
            }
        }
    }
    for tkn in ["CLKT", "CLKT_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            if rd.family == "spartan3" {
                builder.extract_xnode(
                    tslots::CLK,
                    "CLKT.S3",
                    xy,
                    &[],
                    &[xy_l],
                    "CLKT.S3",
                    &[
                        builder
                            .bel_indexed(bels::BUFGMUX0, "BUFGMUX", 4)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI0"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD0"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL0"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR0"])
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_indexed(bels::BUFGMUX1, "BUFGMUX", 5)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI1"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD1"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL1"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR1"])
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_indexed(bels::BUFGMUX2, "BUFGMUX", 6)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI2"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD2"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL2"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR2"])
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_indexed(bels::BUFGMUX3, "BUFGMUX", 7)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI3"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD3"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL3"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR3"])
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual(bels::GLOBALSIG_N),
                    ],
                    &lh,
                );
            } else if rd.family == "fpgacore" {
                builder.extract_xnode(
                    tslots::CLK,
                    "CLKT.FC",
                    xy,
                    &[],
                    &[xy_l],
                    "CLKT.FC",
                    &[
                        builder
                            .bel_indexed(bels::BUFGMUX0, "BUFG", 4)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI0"])
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_indexed(bels::BUFGMUX1, "BUFG", 5)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI1"])
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_indexed(bels::BUFGMUX2, "BUFG", 6)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI2"])
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_indexed(bels::BUFGMUX3, "BUFG", 7)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI3"])
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual(bels::GLOBALSIG_N),
                    ],
                    &lh,
                );
            } else {
                let kind = if rd.family == "spartan3e" {
                    "CLKT.S3E"
                } else {
                    "CLKT.S3A"
                };
                builder.extract_xnode(
                    tslots::CLK,
                    kind,
                    xy,
                    &[],
                    &[xy_l],
                    kind,
                    &[
                        builder
                            .bel_xy(bels::BUFGMUX0, "BUFGMUX", 1, 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKT_CKI0"])
                            .extra_wire("CKIL", &["CLKT_CKI4"])
                            .extra_wire_force("DCM_PAD_L", "CLKT_DLL_CLKPAD4")
                            .extra_wire_force(
                                "DCM_PAD_R",
                                if rd.family == "spartan3e" {
                                    "CLKT_DLL_CLKPAD0"
                                } else {
                                    "CLKT_DLL_CLKPAD2"
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKT_DLL_OUTL0",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTL0".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR0",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTR0".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_xy(bels::BUFGMUX1, "BUFGMUX", 1, 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKT_CKI1"])
                            .extra_wire("CKIL", &["CLKT_CKI5"])
                            .extra_wire_force("DCM_PAD_L", "CLKT_DLL_CLKPAD5")
                            .extra_wire_force(
                                "DCM_PAD_R",
                                if rd.family == "spartan3e" {
                                    "CLKT_DLL_CLKPAD1"
                                } else {
                                    "CLKT_DLL_CLKPAD3"
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKT_DLL_OUTL1",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTL1".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR1",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTR1".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_xy(bels::BUFGMUX2, "BUFGMUX", 0, 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKT_CKI2"])
                            .extra_wire("CKIL", &["CLKT_CKI6"])
                            .extra_wire_force("DCM_PAD_L", "CLKT_DLL_CLKPAD6")
                            .extra_wire_force(
                                "DCM_PAD_R",
                                if rd.family == "spartan3e" {
                                    "CLKT_DLL_CLKPAD2"
                                } else {
                                    "CLKT_DLL_CLKPAD0"
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKT_DLL_OUTL2",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTL2".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR2",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTR2".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_xy(bels::BUFGMUX3, "BUFGMUX", 0, 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKT_CKI3"])
                            .extra_wire("CKIL", &["CLKT_CKI7"])
                            .extra_wire_force("DCM_PAD_L", "CLKT_DLL_CLKPAD7")
                            .extra_wire_force(
                                "DCM_PAD_R",
                                if rd.family == "spartan3e" {
                                    "CLKT_DLL_CLKPAD3"
                                } else {
                                    "CLKT_DLL_CLKPAD1"
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKT_DLL_OUTL3",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTL3".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR3",
                                PipNaming {
                                    tile: RawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTR3".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual(bels::GLOBALSIG_N),
                    ],
                    &lh,
                );
            }
        }
    }

    for (tkn, kind) in [("BBTERM", "DCMCONN.BOT"), ("BTTERM", "DCMCONN.TOP")] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = [xy.delta(0, if kind == "DCMCONN.BOT" { 1 } else { -1 })];
            if rd.tile_kinds.key(rd.tiles[&int_xy[0]].kind) == "BRAM_IOIS_NODCM" {
                continue;
            }
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
                    .extra_wire("CLKPADBUS0", &["BBTERM_DLL_CLKPAD0", "BTTERM_DLL_CLKPAD0"])
                    .extra_wire("CLKPADBUS1", &["BBTERM_DLL_CLKPAD1", "BTTERM_DLL_CLKPAD1"])
                    .extra_wire("CLKPADBUS2", &["BBTERM_DLL_CLKPAD2", "BTTERM_DLL_CLKPAD2"])
                    .extra_wire("CLKPADBUS3", &["BBTERM_DLL_CLKPAD3", "BTTERM_DLL_CLKPAD3"])
                    .extra_int_out("CLKPAD0", &["BBTERM_CLKPAD0", "BTTERM_CLKPAD0"])
                    .extra_int_out("CLKPAD1", &["BBTERM_CLKPAD1", "BTTERM_CLKPAD1"])
                    .extra_int_out("CLKPAD2", &["BBTERM_CLKPAD2", "BTTERM_CLKPAD2"])
                    .extra_int_out("CLKPAD3", &["BBTERM_CLKPAD3", "BTTERM_CLKPAD3"])
                    .extra_int_in("OUT0", &["BTERM_OMUX0", "BTTERM_OMUX10"])
                    .extra_int_in("OUT1", &["BTERM_OMUX3", "BTTERM_OMUX11"])
                    .extra_int_in("OUT2", &["BTERM_OMUX4", "BTTERM_OMUX12"])
                    .extra_int_in("OUT3", &["BTERM_OMUX5", "BTTERM_OMUX15"])],
            );
        }
    }

    if rd.family != "spartan3" && rd.family != "fpgacore" {
        for tkn in ["CLKL", "CLKR"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                let xy_o = xy.delta(if xy.x == 0 { 1 } else { -1 }, 0);
                let int_s_xy = builder.walk_to_int(xy_o, Dir::S, false).unwrap();
                let int_n_xy = builder.walk_to_int(xy_o, Dir::N, false).unwrap();
                let int_xy = [int_s_xy, int_n_xy];
                let kind;
                let buf_xy;
                let mut bels = [
                    builder
                        .bel_xy(bels::BUFGMUX0, "BUFGMUX", 0, 0)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI0", "CLKR_CKI0"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT0", "CLKR_OUTT0"])
                        .extra_int_in("CLK", &["CLKL_GCLK0", "CLKR_GCLK0"]),
                    builder
                        .bel_xy(bels::BUFGMUX1, "BUFGMUX", 0, 1)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI1", "CLKR_CKI1"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT1", "CLKR_OUTT1"])
                        .extra_int_in("CLK", &["CLKL_GCLK1", "CLKR_GCLK1"]),
                    builder
                        .bel_xy(bels::BUFGMUX2, "BUFGMUX", 0, 2)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI2", "CLKR_CKI2"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT2", "CLKR_OUTT2"])
                        .extra_int_in("CLK", &["CLKL_GCLK2", "CLKR_GCLK2"]),
                    builder
                        .bel_xy(bels::BUFGMUX3, "BUFGMUX", 0, 3)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI3", "CLKR_CKI3"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT3", "CLKR_OUTT3"])
                        .extra_int_in("CLK", &["CLKL_GCLK3", "CLKR_GCLK3"]),
                    builder
                        .bel_xy(bels::BUFGMUX4, "BUFGMUX", 0, 4)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI4", "CLKR_CKI4"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB0", "CLKR_OUTB0"])
                        .extra_int_in("CLK", &["CLKL_GCLK4", "CLKR_GCLK4"]),
                    builder
                        .bel_xy(bels::BUFGMUX5, "BUFGMUX", 0, 5)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI5", "CLKR_CKI5"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB1", "CLKR_OUTB1"])
                        .extra_int_in("CLK", &["CLKL_GCLK5", "CLKR_GCLK5"]),
                    builder
                        .bel_xy(bels::BUFGMUX6, "BUFGMUX", 0, 6)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI6", "CLKR_CKI6"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB2", "CLKR_OUTB2"])
                        .extra_int_in("CLK", &["CLKL_GCLK6", "CLKR_GCLK6"]),
                    builder
                        .bel_xy(bels::BUFGMUX7, "BUFGMUX", 0, 7)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI7", "CLKR_CKI7"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB3", "CLKR_OUTB3"])
                        .extra_int_in("CLK", &["CLKL_GCLK7", "CLKR_GCLK7"]),
                    builder
                        .bel_xy(bels::PCILOGICSE, "PCILOGIC", 0, 0)
                        .pin_name_only("PCI_CE", 1)
                        .pin_name_only("IRDY", 1)
                        .pin_name_only("TRDY", 1),
                    builder
                        .bel_xy(bels::VCC, "VCC", 0, 0)
                        .pin_name_only("VCCOUT", 0),
                    builder.bel_virtual(bels::GLOBALSIG_WE),
                ];
                if rd.family == "spartan3e" {
                    kind = format!("{tkn}.S3E");
                    buf_xy = vec![];
                } else {
                    kind = format!("{tkn}.S3A");
                    buf_xy = vec![xy_o];
                    let mut i = 0;
                    bels = bels.map(|x| {
                        if builder.db.bel_slots.key(x.bel).starts_with("BUFGMUX") {
                            let res = x.extra_wire_force("DCM_PAD", format!("{tkn}_CKI{i}_END"));
                            i += 1;
                            res
                        } else {
                            x
                        }
                    });
                }
                builder.extract_xnode_bels(tslots::CLK, &kind, xy, &buf_xy, &int_xy, &kind, &bels);
            }
        }

        for tkn in ["GCLKH_PCI_CE_N"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                builder.extract_xnode_bels(
                    tslots::PCI_CE,
                    "PCI_CE_N",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_N",
                    &[builder
                        .bel_virtual(bels::PCI_CE_N)
                        .extra_wire("I", &["GCLKH_PCI_CE_IN"])
                        .extra_wire("O", &["GCLKH_PCI_CE_OUT"])],
                );
            }
        }
        for tkn in ["GCLKH_PCI_CE_S", "GCLKH_PCI_CE_S_50A"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                builder.extract_xnode_bels(
                    tslots::PCI_CE,
                    "PCI_CE_S",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_S",
                    &[builder
                        .bel_virtual(bels::PCI_CE_S)
                        .extra_wire("I", &["GCLKH_PCI_CE_OUT"])
                        .extra_wire("O", &["GCLKH_PCI_CE_IN"])],
                );
            }
        }
        for tkn in ["LL", "LR", "UL", "UR"] {
            builder.extract_node_bels(
                tslots::PCI_CE,
                tkn,
                "PCI_CE_CNR",
                "PCI_CE_CNR",
                &[builder
                    .bel_virtual(bels::PCI_CE_CNR)
                    .extra_wire("I", &["PCI_CE_NS"])
                    .extra_wire("O", &["PCI_CE_EW"])],
            );
        }
        if rd.family == "spartan3a" {
            for &xy in rd.tiles_by_kind_name("GCLKV_IOISL") {
                builder.extract_xnode_bels(
                    tslots::PCI_CE,
                    "PCI_CE_E",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_E",
                    &[builder
                        .bel_virtual(bels::PCI_CE_E)
                        .extra_wire("I", &["CLKV_PCI_CE_W"])
                        .extra_wire("O", &["CLKV_PCI_CE_E"])],
                );
            }
            for &xy in rd.tiles_by_kind_name("GCLKV_IOISR") {
                builder.extract_xnode_bels(
                    tslots::PCI_CE,
                    "PCI_CE_W",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_W",
                    &[builder
                        .bel_virtual(bels::PCI_CE_W)
                        .extra_wire("I", &["CLKV_PCI_CE_E"])
                        .extra_wire("O", &["CLKV_PCI_CE_W"])],
                );
            }
        }
    }

    if rd.family == "spartan3adsp" {
        for tkn in [
            "BRAMSITE2_3M",
            "BRAMSITE2_3M_BRK",
            "BRAMSITE2_3M_BOT",
            "BRAMSITE2_3M_TOP",
        ] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                builder.extract_xnode_bels(
                    tslots::BEL,
                    "BRAM.S3ADSP",
                    xy,
                    &[],
                    &int_xy,
                    "BRAM.S3ADSP",
                    &[builder.bel_xy(bels::BRAM, "RAMB16", 0, 0)],
                );
            }
        }
        for (tkn, naming) in [
            ("MACCSITE2", "DSP"),
            ("MACCSITE2_BRK", "DSP"),
            ("MACCSITE2_BOT", "DSP"),
            ("MACCSITE2_TOP", "DSP.TOP"),
        ] {
            let buf_cnt = if naming == "DSP.TOP" { 0 } else { 1 };
            let mut bel_dsp = builder
                .bel_xy(bels::DSP, "DSP48A", 0, 0)
                .pin_name_only("CARRYIN", 0)
                .pin_name_only("CARRYOUT", buf_cnt);
            for i in 0..18 {
                bel_dsp = bel_dsp.pin_name_only(&format!("BCIN{i}"), 0);
                bel_dsp = bel_dsp.pin_name_only(&format!("BCOUT{i}"), buf_cnt);
            }
            for i in 0..48 {
                bel_dsp = bel_dsp.pin_name_only(&format!("PCIN{i}"), 0);
                bel_dsp = bel_dsp.pin_name_only(&format!("PCOUT{i}"), buf_cnt);
            }
            let bels_dsp = [bel_dsp];
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                builder.extract_xnode_bels(tslots::BEL, "DSP", xy, &[], &int_xy, naming, &bels_dsp);
                builder.extract_intf_tile_multi(
                    tslots::INTF,
                    "INTF.DSP",
                    xy,
                    &int_xy,
                    "INTF.DSP",
                    false,
                );
            }
        }
    } else if rd.family != "fpgacore" {
        let kind = match &*rd.family {
            "spartan3" => "BRAM.S3",
            "spartan3e" => "BRAM.S3E",
            "spartan3a" => "BRAM.S3A",
            _ => unreachable!(),
        };
        for (tkn, naming) in [
            ("BRAMSITE", kind),
            ("BRAMSITE2", kind),
            ("BRAMSITE2_BRK", kind),
            ("BRAMSITE2_BOT", "BRAM.S3A.BOT"),
            ("BRAMSITE2_TOP", "BRAM.S3A.TOP"),
        ] {
            let mut bel_mult = builder.bel_xy(bels::MULT, "MULT18X18", 0, 0);
            let buf_cnt = if naming == "BRAM.S3A.TOP" { 0 } else { 1 };
            for i in 0..18 {
                bel_mult = bel_mult.pin_name_only(&format!("BCIN{i}"), 0);
                bel_mult = bel_mult.pin_name_only(&format!("BCOUT{i}"), buf_cnt);
            }
            let bels_bram = [builder.bel_xy(bels::BRAM, "RAMB16", 0, 0), bel_mult];
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                builder.extract_xnode_bels(tslots::BEL, kind, xy, &[], &int_xy, naming, &bels_bram);
            }
        }
    }

    for tkn in ["CLKC", "CLKC_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual(bels::CLKC);
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("IN_B{i}"), &[format!("CLKC_GCLK_MAIN_B{i}")])
                    .extra_wire(format!("IN_T{i}"), &[format!("CLKC_GCLK_MAIN_T{i}")])
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("OUT{i}"), &[format!("CLKC_GCLK{i}")]);
            }
            builder.extract_xnode_bels(tslots::CLK, "CLKC", xy, &[], &[xy], "CLKC", &[bel]);
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC_50A") {
        let mut bel = builder.bel_virtual(bels::CLKC_50A);
        for i in 0..4 {
            bel = bel
                .extra_wire(format!("IN_B{i}"), &[format!("CLKC_50A_GCLKB{i}")])
                .extra_wire(format!("IN_T{i}"), &[format!("CLKC_50A_GCLKT{i}")])
        }
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_L{i}"), &[format!("CLKC_50A_GCLK_IN_LH{i}")])
                .extra_wire(format!("IN_R{i}"), &[format!("CLKC_50A_GCLK_IN_RH{i}")])
                .extra_wire(format!("OUT_L{i}"), &[format!("CLKC_50A_GCLK_OUT_LH{i}")])
                .extra_wire(format!("OUT_R{i}"), &[format!("CLKC_50A_GCLK_OUT_RH{i}")]);
        }
        builder.extract_xnode_bels(tslots::CLK, "CLKC_50A", xy, &[], &[xy], "CLKC_50A", &[bel]);
    }

    for &xy in rd.tiles_by_kind_name("GCLKVM") {
        let mut bel = builder.bel_virtual(bels::GCLKVM);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_CORE{i}"), &[format!("GCLKVM_GCLK{i}")])
                .extra_wire(format!("OUT_B{i}"), &[format!("GCLKVM_GCLK_DN{i}")])
                .extra_wire(format!("OUT_T{i}"), &[format!("GCLKVM_GCLK_UP{i}")]);
        }
        builder.extract_xnode_bels(
            tslots::CLK,
            "GCLKVM.S3",
            xy,
            &[],
            &[xy],
            "GCLKVM.S3",
            &[bel],
        );
    }

    for tkn in ["GCLKVML", "GCLKVMR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual(bels::GCLKVM);
            for i in 0..8 {
                bel = bel
                    .extra_wire(
                        format!("IN_CORE{i}"),
                        &[
                            format!("GCLKVML_GCLKCORE{i}"),
                            format!("GCLKVMR_GCLKCORE{i}"),
                        ],
                    )
                    .extra_wire(
                        format!("IN_LR{i}"),
                        &[format!("GCLKVML_GCLKLR{i}"), format!("GCLKVMR_GCLKLR{i}")],
                    )
                    .extra_wire(format!("OUT_B{i}"), &[format!("GCLKVMLR_GCLK_DN{i}")])
                    .extra_wire(format!("OUT_T{i}"), &[format!("GCLKVMLR_GCLK_UP{i}")]);
            }
            builder.extract_xnode_bels(tslots::CLK, "GCLKVM.S3E", xy, &[], &[xy], tkn, &[bel]);
        }
    }

    for &xy in rd.tiles_by_kind_name("GCLKVC") {
        let mut bel = builder.bel_virtual(bels::GCLKVC);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN{i}"), &[format!("GCLKC_GCLK{i}")])
                .extra_wire(format!("OUT_L{i}"), &[format!("GCLKC_GCLK_OUT_L{i}")])
                .extra_wire(format!("OUT_R{i}"), &[format!("GCLKC_GCLK_OUT_R{i}")]);
        }
        builder.extract_xnode_bels(tslots::HROW, "GCLKVC", xy, &[], &[xy], "GCLKVC", &[bel]);
    }

    for tkn in [
        "GCLKH",
        "GCLKH_PCI_CE_S",
        "GCLKH_PCI_CE_N",
        "GCLKH_PCI_CE_S_50A",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_s_xy = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let int_n_xy = builder.walk_to_int(xy, Dir::N, false).unwrap();
            let mut bel = builder.bel_virtual(bels::GCLKH);
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN{i}"), &[format!("GCLKH_GCLK{i}")])
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

    if rd.family != "spartan3" && rd.family != "fpgacore" {
        let dummy_xy = Coord { x: 0, y: 0 };
        let bel_globalsig = builder.bel_virtual(bels::GLOBALSIG);
        let mut bel = builder.bel_virtual(bels::GCLKH);
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_T{i}"),
                    (TileCellId::from_idx(1), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_UP{i}"),
                )
                .extra_int_out_force(
                    format!("OUT_B{i}"),
                    (TileCellId::from_idx(0), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_DN{i}"),
                );
        }
        builder.extract_xnode_bels(
            tslots::HCLK,
            if rd.family == "spartan3e" {
                "GCLKH"
            } else {
                "GCLKH.UNI"
            },
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.BRAM",
            &[bel_globalsig.clone(), bel],
        );
        let mut bel = builder.bel_virtual(bels::GCLKH);
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_B{i}"),
                    (TileCellId::from_idx(0), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_DN{i}"),
                );
        }
        builder.extract_xnode_bels(
            tslots::HCLK,
            if rd.family == "spartan3e" {
                "GCLKH.S"
            } else {
                "GCLKH.UNI.S"
            },
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.BRAM.S",
            &[bel_globalsig.clone(), bel],
        );
        let mut bel = builder.bel_virtual(bels::GCLKH);
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_T{i}"),
                    (TileCellId::from_idx(1), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_UP{i}"),
                )
        }
        builder.extract_xnode_bels(
            tslots::HCLK,
            if rd.family == "spartan3e" {
                "GCLKH.N"
            } else {
                "GCLKH.UNI.N"
            },
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.BRAM.N",
            &[bel_globalsig.clone(), bel],
        );
        builder.extract_xnode_bels(
            tslots::HCLK,
            "GCLKH.0",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.0",
            &[bel_globalsig],
        );
        builder.extract_xnode_bels(
            tslots::CLK,
            "GCLKH.DSP",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.DSP",
            &[builder.bel_virtual(bels::GLOBALSIG_DSP)],
        );
    }

    builder.build()
}
