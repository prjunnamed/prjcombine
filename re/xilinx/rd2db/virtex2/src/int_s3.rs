use std::collections::{BTreeMap, BTreeSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelPin, GroupTestMux, GroupTestMuxWire, IntDb, LegacyBel, TileWireCoord},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{BelNaming, BelPinNaming, NamingDb, PipNaming, RawTileId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_types::bitvec::BitVec;
use prjcombine_virtex2::{defs, defs::spartan3::ccls, defs::spartan3::tcls, defs::spartan3::wires};

use prjcombine_re_xilinx_rd2db_interconnect::IntBuilder;

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::spartan3::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => {
            if rd.family == "fpgacore" {
                ccls::PASS_W_FC
            } else {
                ccls::PASS_W
            }
        }
        Dir::E => ccls::PASS_E,
        Dir::S => {
            if rd.family == "fpgacore" {
                ccls::PASS_S_FC
            } else {
                ccls::PASS_S
            }
        }
        Dir::N => ccls::PASS_N,
    }));

    builder.wire_names(
        wires::PULLUP,
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

    for i in 0..8 {
        builder.wire_names(
            wires::GCLK[i],
            &[format!("GCLK{i}"), format!("GCLK{i}_BRK")],
        );
    }
    for i in 0..4 {
        builder.wire_names(
            wires::DCM_CLKPAD[i],
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
        builder.wire_names(wires::OMUX[i], &[format!("OMUX{i}")]);
        let omux_da1 = builder.db.get_wire(&format!("OMUX_{da1}{i}"));
        builder.wire_names(omux_da1, &[format!("OMUX_{da1}{i}")]);
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
            let omux_da2 = builder.db.get_wire(&format!("OMUX_{da1}{da2}{i}"));
            builder.wire_names(omux_da2, &[format!("OMUX_{da1}{da2}{i}")]);
        }
        if let Some(db) = db {
            let omux_db = builder.db.get_wire(&format!("OMUX_{db}{i}"));
            builder.wire_names(omux_db, &[format!("{db}{da1}_{db}")]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
            let beg = builder.db.get_wire(&format!("DBL_{dir}0[{i}]"));
            let mid = builder.db.get_wire(&format!("DBL_{dir}1[{i}]"));
            let end = builder.db.get_wire(&format!("DBL_{dir}2[{i}]"));
            let (end2, e2d) = match dir {
                Dir::W => (wires::DBL_W2_N[i], Dir::N),
                Dir::E => (wires::DBL_E2_S[i], Dir::S),
                Dir::S => (wires::DBL_S3[i], Dir::S),
                Dir::N => (wires::DBL_N3[i], Dir::N),
            };
            builder.wire_names(beg, &[format!("{dir}2BEG{i}")]);
            builder.wire_names(mid, &[format!("{dir}2MID{i}")]);
            builder.wire_names(end, &[format!("{dir}2END{i}")]);
            builder.wire_names(end2, &[format!("{dir}2END_{e2d}{i}")]);
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
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
                builder.wire_names(w, &[format!("{dir}6{seg}{i}")]);
            }
            let (end2, e2d) = match dir {
                Dir::W => (wires::HEX_W6_N[i], Dir::N),
                Dir::E => (wires::HEX_E6_S[i], Dir::S),
                Dir::S => (wires::HEX_S7[i], Dir::S),
                Dir::N => (wires::HEX_N7[i], Dir::N),
            };
            builder.wire_names(end2, &[format!("{dir}6END_{e2d}{i}")]);
        }
    }

    for i in 0..24 {
        builder.wire_names(wires::LH[i], &[format!("LH{i}")]);
        builder.wire_names(wires::LV[i], &[format!("LV{i}")]);
    }

    // The set/reset inputs.
    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_SR[i],
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
        builder.wire_names(
            wires::IMUX_CLK[i],
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
        builder.wire_names(
            wires::IMUX_IOCLK[i],
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
        builder.wire_names(
            wires::IMUX_CE[i],
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

    for (xy, ws, bounce) in [
        ('X', wires::IMUX_FAN_BX, wires::IMUX_FAN_BX_BOUNCE),
        ('Y', wires::IMUX_FAN_BY, wires::IMUX_FAN_BY_BOUNCE),
    ] {
        for i in 0..4 {
            builder.wire_names(
                ws[i],
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
            if rd.family == "spartan3adsp" {
                builder.wire_names(
                    bounce[i],
                    &[format!("BRAM_FAN_B{xy}{i}"), format!("MACC_FAN_B{xy}{i}")],
                );
            }
            builder.mark_permabuf(bounce[i]);
        }
    }

    for i in 0..32 {
        builder.wire_names(
            wires::IMUX_DATA[i],
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
    }

    for i in 0..8 {
        builder.wire_names(
            wires::OUT_FAN[i],
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
        builder.mark_test_mux_in(wires::OUT_FAN_TMIN[i], wires::OUT_FAN[i]);
    }

    for i in 0..16 {
        builder.wire_names(
            wires::OUT_SEC[i],
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
        builder.mark_test_mux_in(wires::OUT_SEC_TMIN[i], wires::OUT_SEC[i]);
    }
    builder.stub_out("STUB_IOIS_X3");
    builder.stub_out("STUB_IOIS_Y3");
    builder.stub_out("STUB_IOIS_XQ3");
    builder.stub_out("STUB_IOIS_YQ3");

    for (j, (ws, tmin)) in [
        (wires::OUT_HALF0, wires::OUT_HALF0_TMIN),
        (wires::OUT_HALF1, wires::OUT_HALF1_TMIN),
    ]
    .into_iter()
    .enumerate()
    {
        for i in 0..4 {
            builder.wire_names(
                ws[i],
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
            builder.mark_test_mux_in(tmin[i], ws[i]);
        }
    }

    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_BUFG_SEL[i],
            &[format!("CLKB_SELDUB{i}"), format!("CLKT_SELDUB{i}")],
        );
    }
    for i in 0..4 {
        builder.wire_names(
            wires::IMUX_BUFG_CLK[i],
            &[format!("CLKB_CLKDUB{i}"), format!("CLKT_CLKDUB{i}")],
        );
    }
    for i in 0..4 {
        builder.wire_names(
            wires::OUT_BUFG[i],
            &[format!("CLKB_GCLK_MAIN{i}"), format!("CLKT_GCLK_MAIN{i}")],
        );
    }

    let bels_int = [builder.bel_xy(defs::bslots::RLL, "RLL", 0, 0)];
    let bels_int_dcm = [
        builder.bel_xy(defs::bslots::RLL, "RLL", 0, 0),
        builder
            .bel_virtual(defs::bslots::PTE2OMUX[0])
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
            .bel_virtual(defs::bslots::PTE2OMUX[1])
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
            .bel_virtual(defs::bslots::PTE2OMUX[2])
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
            .bel_virtual(defs::bslots::PTE2OMUX[3])
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

    if rd.family == "fpgacore" {
        builder.extract_int_id(
            tcls::INT_CLB_FC,
            defs::bslots::INT,
            "CENTER",
            "INT_CLB_FC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_CLB_FC,
            defs::bslots::INT,
            "CENTER_SMALL",
            "INT_CLB_FC",
            &bels_int,
        );
    } else {
        builder.extract_int_id(
            tcls::INT_CLB,
            defs::bslots::INT,
            "CENTER",
            "INT_CLB",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_CLB,
            defs::bslots::INT,
            "CENTER_SMALL",
            "INT_CLB",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_CLB,
            defs::bslots::INT,
            "CENTER_SMALL_BRK",
            "INT_CLB_BRK",
            &bels_int,
        );
    }
    if rd.family.starts_with("spartan3a") {
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIOIS",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIOIS_BRK",
            "INT_IOI_S3A_WE_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIOIS_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIOIS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIOIS_CLK_PCI_BRK",
            "INT_IOI_S3A_WE_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIBUFS",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIBUFS_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "LIBUFS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIOIS",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIOIS_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIOIS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIBUFS",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIBUFS_BRK",
            "INT_IOI_S3A_WE_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIBUFS_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIBUFS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            defs::bslots::INT,
            "RIBUFS_CLK_PCI_BRK",
            "INT_IOI_S3A_WE_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            defs::bslots::INT,
            "BIOIS",
            "INT_IOI_S3A_SN",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            defs::bslots::INT,
            "BIOIB",
            "INT_IOI_S3A_SN",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            defs::bslots::INT,
            "TIOIS",
            "INT_IOI_S3A_SN",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            defs::bslots::INT,
            "TIOIB",
            "INT_IOI_S3A_SN",
            &bels_int,
        );
    } else if rd.family == "spartan3e" {
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIOIS_BRK",
            "INT_IOI_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIOIS_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIOIS_CLK_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIBUFS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIBUFS_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "LIBUFS_CLK_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIOIS_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIOIS_CLK_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIBUFS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIBUFS_BRK",
            "INT_IOI_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIBUFS_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "RIBUFS_CLK_PCI",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "BIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "BIBUFS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "TIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            defs::bslots::INT,
            "TIBUFS",
            "INT_IOI",
            &bels_int,
        );
    } else if rd.family == "fpgacore" {
        builder.extract_int_id(
            tcls::INT_IOI_FC,
            defs::bslots::INT,
            "LIOIS",
            "INT_IOI_FC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_FC,
            defs::bslots::INT,
            "RIOIS",
            "INT_IOI_FC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_FC,
            defs::bslots::INT,
            "BIOIS",
            "INT_IOI_FC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_FC,
            defs::bslots::INT,
            "TIOIS",
            "INT_IOI_FC",
            &bels_int,
        );
    } else {
        // NOTE: could be unified by pulling extra muxes from CLB
        builder.extract_int_id(
            tcls::INT_IOI_S3,
            defs::bslots::INT,
            "LIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3,
            defs::bslots::INT,
            "RIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3,
            defs::bslots::INT,
            "BIOIS",
            "INT_IOI",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3,
            defs::bslots::INT,
            "TIOIS",
            "INT_IOI",
            &bels_int,
        );
    }
    // NOTE:
    // - S3/S3E/S3A could be unified by pulling some extra muxes from CLB
    // - S3A/S3ADSP adds VCC input to B[XY] and splits B[XY] to two wires
    if rd.family == "spartan3adsp" {
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM_S3ADSP",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM0_SMALL_BOT",
            "INT_BRAM_S3ADSP",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM_S3ADSP",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM_S3ADSP",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM_S3ADSP",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM3_SMALL_TOP",
            "INT_BRAM_S3ADSP",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "BRAM3_SMALL_BRK",
            "INT_BRAM_S3ADSP_BRK",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC0_SMALL",
            "INT_MACC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC0_SMALL_BOT",
            "INT_MACC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC1_SMALL",
            "INT_MACC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC2_SMALL",
            "INT_MACC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC3_SMALL",
            "INT_MACC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC3_SMALL_TOP",
            "INT_MACC",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            defs::bslots::INT,
            "MACC3_SMALL_BRK",
            "INT_MACC_BRK",
            &bels_int,
        );
    } else if rd.family == "spartan3a" {
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            defs::bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            defs::bslots::INT,
            "BRAM0_SMALL_BOT",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_12,
            defs::bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_12,
            defs::bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            defs::bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            defs::bslots::INT,
            "BRAM3_SMALL_TOP",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            defs::bslots::INT,
            "BRAM3_SMALL_BRK",
            "INT_BRAM_BRK",
            &bels_int,
        );
    } else if rd.family == "spartan3e" {
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            defs::bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            defs::bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            defs::bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            defs::bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            defs::bslots::INT,
            "BRAM3_SMALL_BRK",
            "INT_BRAM_BRK",
            &bels_int,
        );
    } else {
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM0",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM1",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM2",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM3",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM",
            &bels_int,
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            defs::bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM",
            &bels_int,
        );
    }
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "BRAM_IOIS",
        "INT_DCM_S3",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM_S3_DUMMY,
        defs::bslots::INT,
        "BRAM_IOIS_NODCM",
        "INT_DCM_S3_DUMMY",
        &bels_int,
    );
    builder.extract_int_id(
        tcls::INT_DCM_S3E_DUMMY,
        defs::bslots::INT,
        "DCMAUX_BL_CENTER",
        "INT_DCM_S3E_DUMMY",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM_S3E_DUMMY,
        defs::bslots::INT,
        "DCMAUX_TL_CENTER",
        "INT_DCM_S3E_DUMMY",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_BL_CENTER",
        "INT_DCM_S3E",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_TL_CENTER",
        "INT_DCM_S3E",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_BR_CENTER",
        "INT_DCM_S3E",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_TR_CENTER",
        "INT_DCM_S3E",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_H_BL_CENTER",
        "INT_DCM_S3E_H",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_H_TL_CENTER",
        "INT_DCM_S3E_H",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_H_BR_CENTER",
        "INT_DCM_S3E_H",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_H_TR_CENTER",
        "INT_DCM_S3E_H",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_BGAP",
        "INT_DCM_S3E_H",
        &bels_int_dcm,
    );
    builder.extract_int_id(
        tcls::INT_DCM,
        defs::bslots::INT,
        "DCM_SPLY",
        "INT_DCM_S3E_H",
        &bels_int_dcm,
    );
    let (int_clb, int_cnr) = if rd.family == "fpgacore" {
        (tcls::INT_CLB_FC, "INT_CNR_FC")
    } else {
        (tcls::INT_CLB, "INT_CNR")
    };
    builder.extract_int_id(int_clb, defs::bslots::INT, "LL", int_cnr, &bels_int);
    builder.extract_int_id(int_clb, defs::bslots::INT, "LR", int_cnr, &bels_int);
    builder.extract_int_id(int_clb, defs::bslots::INT, "UL", int_cnr, &bels_int);
    builder.extract_int_id(int_clb, defs::bslots::INT, "UR", int_cnr, &bels_int);

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    let bels_clb = [
        builder
            .bel_xy(defs::bslots::SLICE[0], "SLICE", 0, 0)
            .pins_name_only(&slicem_name_only),
        builder
            .bel_xy(defs::bslots::SLICE[1], "SLICE", 1, 0)
            .pins_name_only(&slicel_name_only),
        builder
            .bel_xy(defs::bslots::SLICE[2], "SLICE", 0, 1)
            .pins_name_only(&slicem_name_only)
            .extra_wire("COUT_N", &["COUT_N1"])
            .extra_wire("FX_S", &["FX_S2"]),
        builder
            .bel_xy(defs::bslots::SLICE[3], "SLICE", 1, 1)
            .pins_name_only(&slicel_name_only)
            .extra_wire("COUT_N", &["COUT_N3"]),
    ];
    builder.extract_int_bels_id(tcls::CLB, "CENTER", "CLB", &bels_clb);
    builder.extract_int_bels_id(tcls::CLB, "CENTER_SMALL", "CLB", &bels_clb);
    builder.extract_int_bels_id(tcls::CLB, "CENTER_SMALL_BRK", "CLB", &bels_clb);

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
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(defs::bslots::IOI[2], "IOB", 2)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_int_bels_id(tcls::IOI_S3, "LIOIS", "IOI_S3_W", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_S3, "RIOIS", "IOI_S3_E", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_S3, "BIOIS", "IOI_S3_S", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_S3, "TIOIS", "IOI_S3_N", &bels_ioi);
    } else if rd.family == "fpgacore" {
        let bels_ioi = [
            builder
                .bel_indexed(defs::bslots::IBUF[0], "IBUF", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0"),
            builder
                .bel_indexed(defs::bslots::IBUF[1], "IBUF", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1"),
            builder.bel_indexed(defs::bslots::IBUF[2], "IBUF", 2),
            builder.bel_indexed(defs::bslots::IBUF[3], "IBUF", 3),
            builder.bel_indexed(defs::bslots::OBUF[0], "OBUF", 0),
            builder.bel_indexed(defs::bslots::OBUF[1], "OBUF", 1),
            builder.bel_indexed(defs::bslots::OBUF[2], "OBUF", 2),
            builder.bel_indexed(defs::bslots::OBUF[3], "OBUF", 3),
        ];
        builder.extract_int_bels_id(tcls::IOI_FC, "LIOIS", "IOI_FC_W", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_FC, "RIOIS", "IOI_FC_E", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_FC, "BIOIS", "IOI_FC_S", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_FC, "TIOIS", "IOI_FC_N", &bels_ioi);
    } else if rd.family == "spartan3e" {
        let bels_ioi_tb = [
            builder
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(defs::bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed(defs::bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_wire_force("IBUF", "RIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_wire_force("IBUF", "RIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed(defs::bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_int_bels_id(tcls::IOI_S3E, "LIOIS", "IOI_S3E_W", &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3E, "LIOIS_BRK", "IOI_S3E_W", &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3E, "LIOIS_PCI", "IOI_S3E_W_PCI_PCI", &bels_ioi_l);
        builder.extract_int_bels_id(
            tcls::IOI_S3E,
            "LIOIS_CLK_PCI",
            "IOI_S3E_W_PCI_PCI",
            &bels_ioi_l,
        );
        builder.extract_int_bels_id(tcls::IOI_S3E, "LIBUFS", "IOI_S3E_W", &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3E, "LIBUFS_PCI", "IOI_S3E_W_PCI", &bels_ioi_l);
        builder.extract_int_bels_id(
            tcls::IOI_S3E,
            "LIBUFS_CLK_PCI",
            "IOI_S3E_W_PCI",
            &bels_ioi_l,
        );
        builder.extract_int_bels_id(tcls::IOI_S3E, "RIOIS", "IOI_S3E_E", &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3E, "RIOIS_PCI", "IOI_S3E_E_PCI_PCI", &bels_ioi_r);
        builder.extract_int_bels_id(
            tcls::IOI_S3E,
            "RIOIS_CLK_PCI",
            "IOI_S3E_E_PCI_PCI",
            &bels_ioi_r,
        );
        builder.extract_int_bels_id(tcls::IOI_S3E, "RIBUFS", "IOI_S3E_E", &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3E, "RIBUFS_BRK", "IOI_S3E_E", &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3E, "RIBUFS_PCI", "IOI_S3E_E_PCI", &bels_ioi_r);
        builder.extract_int_bels_id(
            tcls::IOI_S3E,
            "RIBUFS_CLK_PCI",
            "IOI_S3E_E_PCI",
            &bels_ioi_r,
        );
        builder.extract_int_bels_id(tcls::IOI_S3E, "BIOIS", "IOI_S3E_S", &bels_ioi_tb);
        builder.extract_int_bels_id(tcls::IOI_S3E, "BIBUFS", "IOI_S3E_S", &bels_ioi_tb);
        builder.extract_int_bels_id(tcls::IOI_S3E, "TIOIS", "IOI_S3E_N", &bels_ioi_tb);
        builder.extract_int_bels_id(tcls::IOI_S3E, "TIBUFS", "IOI_S3E_N", &bels_ioi_tb);
    } else {
        let bels_ioi_tb = [
            builder
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(defs::bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed(defs::bslots::IOI[0], "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "RIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(defs::bslots::IOI[1], "IOB", 1)
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
            naming_l = "IOI_S3ADSP_W";
            naming_r = "IOI_S3ADSP_E";
            naming_l_pci = "IOI_S3ADSP_W_PCI";
            naming_r_pci = "IOI_S3ADSP_E_PCI";
            naming_b = "IOI_S3ADSP_S";
            naming_t = "IOI_S3ADSP_N";
        } else {
            naming_l = "IOI_S3A_W";
            naming_r = "IOI_S3A_E";
            naming_l_pci = "IOI_S3A_W_PCI";
            naming_r_pci = "IOI_S3A_E_PCI";
            naming_b = "IOI_S3A_S";
            naming_t = "IOI_S3A_N";
        }
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIOIS", naming_l, &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIOIS_BRK", naming_l, &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIOIS_PCI", naming_l_pci, &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIOIS_CLK_PCI", naming_l_pci, &bels_ioi_l);
        builder.extract_int_bels_id(
            tcls::IOI_S3A_WE,
            "LIOIS_CLK_PCI_BRK",
            naming_l_pci,
            &bels_ioi_l,
        );
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIBUFS", naming_l, &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIBUFS_PCI", naming_l, &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "LIBUFS_CLK_PCI", naming_l, &bels_ioi_l);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIOIS", naming_r, &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIOIS_PCI", naming_r_pci, &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIOIS_CLK_PCI", naming_r_pci, &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIBUFS", naming_r, &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIBUFS_BRK", naming_r, &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIBUFS_PCI", naming_r, &bels_ioi_r);
        builder.extract_int_bels_id(tcls::IOI_S3A_WE, "RIBUFS_CLK_PCI", naming_r, &bels_ioi_r);
        builder.extract_int_bels_id(
            tcls::IOI_S3A_WE,
            "RIBUFS_CLK_PCI_BRK",
            naming_r,
            &bels_ioi_r,
        );
        builder.extract_int_bels_id(tcls::IOI_S3A_S, "BIOIS", naming_b, &bels_ioi_tb);
        builder.extract_int_bels_id(tcls::IOI_S3A_S, "BIOIB", naming_b, &bels_ioi_tb);
        builder.extract_int_bels_id(tcls::IOI_S3A_N, "TIOIS", naming_t, &bels_ioi_tb);
        builder.extract_int_bels_id(tcls::IOI_S3A_N, "TIOIB", naming_t, &bels_ioi_tb);
    }
    let tcid_randor = if rd.family == "fpgacore" {
        tcls::RANDOR_FC
    } else {
        tcls::RANDOR
    };
    if rd.family != "fpgacore" {
        let bels_randor_b = [builder
            .bel_xy(defs::bslots::RANDOR, "RANDOR", 0, 0)
            .pins_name_only(&["CIN0", "CIN1", "CPREV", "O"])];
        builder.extract_int_bels_id(tcid_randor, "BIOIS", "RANDOR_S", &bels_randor_b);
        builder.extract_int_bels_id(tcid_randor, "BIOIB", "RANDOR_S", &bels_randor_b);
        builder.extract_int_bels_id(tcid_randor, "BIBUFS", "RANDOR_S", &bels_randor_b);
    }
    let bels_randor_t = [builder
        .bel_xy(defs::bslots::RANDOR, "RANDOR", 0, 0)
        .pins_name_only(&["CIN0", "CIN1"])
        .pin_name_only("CPREV", 1)
        .pin_name_only("O", 1)];
    builder.extract_int_bels_id(tcid_randor, "TIOIS", "RANDOR_N", &bels_randor_t);
    builder.extract_int_bels_id(tcid_randor, "TIOIB", "RANDOR_N", &bels_randor_t);
    builder.extract_int_bels_id(tcid_randor, "TIBUFS", "RANDOR_N", &bels_randor_t);
    builder.db.tile_classes[tcid_randor].cells.clear();
    if rd.family == "spartan3" {
        let bels_dcm = [builder.bel_xy(defs::bslots::DCM, "DCM", 0, 0)];
        builder.extract_int_bels_id(tcls::DCM_S3, "BRAM_IOIS", "DCM_S3", &bels_dcm);
    } else if rd.family != "fpgacore" {
        let bels_dcm = [
            builder.bel_xy(defs::bslots::DCM, "DCM", 0, 0),
            builder
                .bel_virtual(defs::bslots::DCMCONN_S3E)
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
        builder.extract_int_bels_id(tcls::DCM_S3E_SW, "DCM_BL_CENTER", "DCM_S3E_W", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_NW, "DCM_TL_CENTER", "DCM_S3E_W", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_SE, "DCM_BR_CENTER", "DCM_S3E_E", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_NE, "DCM_TR_CENTER", "DCM_S3E_E", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_WS, "DCM_H_BL_CENTER", "DCM_S3E_H", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_WN, "DCM_H_TL_CENTER", "DCM_S3E_H", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_ES, "DCM_H_BR_CENTER", "DCM_S3E_H", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_EN, "DCM_H_TR_CENTER", "DCM_S3E_H", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_WS, "DCM_BGAP", "DCM_S3E_H", &bels_dcm);
        builder.extract_int_bels_id(tcls::DCM_S3E_WN, "DCM_SPLY", "DCM_S3E_H", &bels_dcm);
    }

    if rd.family == "spartan3" {
        builder.extract_int_bels_id(
            tcls::CNR_SW_S3,
            "LL",
            "CNR_SW_S3",
            &[
                builder.bel_indexed(defs::bslots::DCI[0], "DCI", 6),
                builder.bel_indexed(defs::bslots::DCI[1], "DCI", 5),
                builder.bel_indexed(defs::bslots::DCIRESET[0], "DCIRESET", 6),
                builder.bel_indexed(defs::bslots::DCIRESET[1], "DCIRESET", 5),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_SE_S3,
            "LR",
            "CNR_SE_S3",
            &[
                builder.bel_indexed(defs::bslots::DCI[0], "DCI", 3),
                builder.bel_indexed(defs::bslots::DCI[1], "DCI", 4),
                builder.bel_indexed(defs::bslots::DCIRESET[0], "DCIRESET", 3),
                builder.bel_indexed(defs::bslots::DCIRESET[1], "DCIRESET", 4),
                builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
                builder.bel_single(defs::bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(defs::bslots::ICAP, "ICAP"),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_S3,
            "UL",
            "CNR_NW_S3",
            &[
                builder.bel_indexed(defs::bslots::DCI[0], "DCI", 7),
                builder.bel_indexed(defs::bslots::DCI[1], "DCI", 0),
                builder.bel_indexed(defs::bslots::DCIRESET[0], "DCIRESET", 7),
                builder.bel_indexed(defs::bslots::DCIRESET[1], "DCIRESET", 0),
                builder.bel_single(defs::bslots::PMV, "PMV"),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_S3,
            "UR",
            "CNR_NE_S3",
            &[
                builder.bel_indexed(defs::bslots::DCI[0], "DCI", 2),
                builder.bel_indexed(defs::bslots::DCI[1], "DCI", 1),
                builder.bel_indexed(defs::bslots::DCIRESET[0], "DCIRESET", 2),
                builder.bel_indexed(defs::bslots::DCIRESET[1], "DCIRESET", 1),
                builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(defs::bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
    } else if rd.family == "fpgacore" {
        builder.extract_int_bels_id(tcls::CNR_SW_FC, "LL", "CNR_SW_FC", &[]);
        builder.extract_int_bels_id(
            tcls::CNR_SE_FC,
            "LR",
            "CNR_SE_FC",
            &[
                builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
                builder.bel_single(defs::bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(defs::bslots::ICAP, "ICAP"),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_FC,
            "UL",
            "CNR_NW_FC",
            &[builder.bel_single(defs::bslots::PMV, "PMV")],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_FC,
            "UR",
            "CNR_NE_FC",
            &[
                builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(defs::bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
        for (tcid, nn) in [
            (tcls::CNR_SW_FC, "CNR_SW_FC"),
            (tcls::CNR_SE_FC, "CNR_SE_FC"),
            (tcls::CNR_NW_FC, "CNR_NW_FC"),
            (tcls::CNR_NE_FC, "CNR_NE_FC"),
        ] {
            let mut bel = LegacyBel::default();
            bel.pins.insert(
                "CLK".into(),
                BelPin::new_in(TileWireCoord::new_idx(0, wires::IMUX_CLK[3])),
            );
            let tcls = &mut builder.db.tile_classes[tcid];
            tcls.bels.insert(defs::bslots::MISR, BelInfo::Legacy(bel));
            let pin_naming = BelPinNaming {
                tile: RawTileId::from_idx(0),
                name: "CNR_CLK3".into(),
                name_far: "CNR_CLK3".into(),
                pips: vec![],
                int_pips: BTreeMap::new(),
                is_intf: false,
            };
            let mut bel_naming = BelNaming {
                tiles: vec![RawTileId::from_idx(0)],
                pins: BTreeMap::new(),
            };
            bel_naming.pins.insert("CLK".into(), pin_naming);
            let naming = builder.ndb.tile_class_namings.get_mut(nn).unwrap().1;
            naming.bels.insert(defs::bslots::MISR, bel_naming);
        }
    } else if rd.family == "spartan3e" {
        builder.extract_int_bels_id(tcls::CNR_SW_S3E, "LL", "CNR_SW_S3E", &[]);
        builder.extract_int_bels_id(
            tcls::CNR_SE_S3E,
            "LR",
            "CNR_SE_S3E",
            &[
                builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
                builder.bel_single(defs::bslots::CAPTURE, "CAPTURE"),
                builder
                    .bel_single(defs::bslots::ICAP, "ICAP")
                    .pin_force_int(
                        "I2",
                        TileWireCoord::new_idx(0, wires::IMUX_DATA[2]),
                        "CNR_DATA_IN2",
                    ),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_S3E,
            "UL",
            "CNR_NW_S3E",
            &[builder.bel_single(defs::bslots::PMV, "PMV")],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_S3E,
            "UR",
            "CNR_NE_S3E",
            &[
                builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(defs::bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
    } else {
        builder.extract_int_bels_id(tcls::CNR_SW_S3A, "LL", "CNR_SW_S3A", &[]);
        builder.extract_int_bels_id(
            tcls::CNR_SE_S3A,
            "LR",
            "CNR_SE_S3A",
            &[
                builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
                builder.bel_single(defs::bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(defs::bslots::ICAP, "ICAP"),
                builder.bel_single(defs::bslots::SPI_ACCESS, "SPI_ACCESS"),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_S3A,
            "UL",
            "CNR_NW_S3A",
            &[
                builder.bel_single(defs::bslots::PMV, "PMV"),
                builder.bel_single(defs::bslots::DNA_PORT, "DNA_PORT"),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_S3A,
            "UR",
            "CNR_NE_S3A",
            &[
                builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(defs::bslots::RANDOR_OUT)
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
        builder.extract_term_id(ccls::TERM_W, None, Dir::W, tkn, "TERM_W");
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
        builder.extract_term_id(ccls::TERM_E, None, Dir::E, tkn, "TERM_E");
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
        builder.extract_term_id(ccls::TERM_S, None, Dir::S, tkn, "TERM_S");
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
        builder.extract_term_id(ccls::TERM_N, None, Dir::N, tkn, "TERM_N");
    }
    if rd.family == "fpgacore" {
        builder.extract_term_id(ccls::TERM_S, None, Dir::S, "CNR_BTERM", "TERM_S_CNR_FC");
        builder.extract_term_id(ccls::TERM_N, None, Dir::N, "CNR_TTERM", "TERM_N_CNR_FC");
    } else {
        builder.extract_term_id(ccls::TERM_S, None, Dir::S, "CNR_BTERM", "TERM_S_CNR");
        builder.extract_term_id(ccls::TERM_N, None, Dir::N, "CNR_TTERM", "TERM_N_CNR");
    }

    if rd.family == "spartan3e" {
        let cob_term_t_y = rd.tile_kinds.get("COB_TERM_T").unwrap().1.tiles[0].y;
        for &xy_b in &rd.tile_kinds.get("COB_TERM_B").unwrap().1.tiles {
            let xy_t = Coord {
                x: xy_b.x,
                y: cob_term_t_y,
            };
            let int_s_xy = builder.walk_to_int(xy_b, Dir::S, false).unwrap();
            let int_n_xy = builder.walk_to_int(xy_t, Dir::N, false).unwrap();
            builder.extract_pass_tile_id(
                ccls::TERM_BRAM_S,
                Dir::S,
                int_n_xy,
                Some(xy_t),
                None,
                Some("TERM_BRAM_S"),
                None,
                None,
                int_s_xy,
                &wires::LV[..],
            );
            builder.extract_pass_tile_id(
                ccls::TERM_BRAM_N,
                Dir::N,
                int_s_xy,
                Some(xy_b),
                None,
                Some("TERM_BRAM_N"),
                None,
                None,
                int_n_xy,
                &wires::LV[..],
            );
        }
        for tkn in ["CLKL_IOIS", "CLKR_IOIS"] {
            builder.extract_pass_simple_id(
                ccls::CLK_WE_S3E_S,
                ccls::CLK_WE_S3E_N,
                Dir::S,
                tkn,
                &[],
            );
        }
    }
    if rd.family == "spartan3" {
        builder.extract_pass_simple_id(ccls::BRKH_S3_S, ccls::BRKH_S3_N, Dir::S, "BRKH", &[]);
    }
    for (tkn, naming) in [
        ("CLKH_LL", "LLV"),
        ("CLKH_DCM_LL", "LLV"),
        ("CLKLH_DCM_LL", "LLV"),
        ("CLKRH_DCM_LL", "LLV"),
        ("CLKL_IOIS_LL", "LLV_CLKL"),
        ("CLKR_IOIS_LL", "LLV_CLKR"),
    ] {
        let mut llv_s = ccls::LLV_S;
        let mut llv_n = ccls::LLV_N;
        if rd.family != "spartan3a" && naming != "LLV" {
            llv_s = ccls::LLV_CLK_WE_S3E_S;
            llv_n = ccls::LLV_CLK_WE_S3E_N;
        }
        let tcid = if rd.family == "spartan3e" {
            tcls::LLV_S3E
        } else {
            tcls::LLV_S3A
        };
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::S, false).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::N, false).unwrap();
            builder.extract_pass_tile_id(
                llv_s,
                Dir::S,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                None,
                Some((tcid, defs::bslots::LLV, naming)),
                int_fwd_xy,
                &[],
            );
            builder.extract_pass_tile_id(
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
    for (tcid, tkn) in [
        (tcls::LLH, "CLKV_DCM_LL"),
        (tcls::LLH, "CLKV_LL"),
        (
            if rd.family == "spartan3e" {
                tcls::LLH
            } else {
                tcls::LLH_N_S3A
            },
            "CLKT_LL",
        ),
        (
            if rd.family == "spartan3e" {
                tcls::LLH
            } else {
                tcls::LLH_S_S3A
            },
            "CLKB_LL",
        ),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let fix_xy = if tkn == "CLKB_LL" { xy.delta(0, 1) } else { xy };
            let int_fwd_xy = builder.walk_to_int(fix_xy, Dir::W, false).unwrap();
            let int_bwd_xy = builder.walk_to_int(fix_xy, Dir::E, false).unwrap();
            let mut llh_w = ccls::LLH_W;
            let mut llh_e = ccls::LLH_E;
            if rd.family == "spartan3adsp" && tkn == "CLKV_DCM_LL" {
                llh_w = ccls::LLH_DCM_S3ADSP_W;
                llh_e = ccls::LLH_DCM_S3ADSP_E;
            }
            builder.extract_pass_tile_id(
                llh_w,
                Dir::W,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                None,
                Some((tcid, defs::bslots::LLH, "LLH")),
                int_fwd_xy,
                &[],
            );
            builder.extract_pass_tile_id(
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
            builder.extract_pass_simple_id(
                ccls::DSPHOLE_W,
                ccls::DSPHOLE_E,
                Dir::W,
                tkn,
                &wires::LH[..],
            );
        }
        for &xy in rd.tiles_by_kind_name("DCM_BGAP") {
            let mut int_w_xy = xy;
            let mut int_e_xy = xy;
            int_e_xy.x += 5;
            builder.extract_pass_tile_id(
                ccls::DSPHOLE_W,
                Dir::W,
                int_e_xy,
                None,
                None,
                None,
                None,
                None,
                int_w_xy,
                &wires::LH[..],
            );
            builder.extract_pass_tile_id(
                ccls::DSPHOLE_E,
                Dir::E,
                int_w_xy,
                None,
                None,
                None,
                None,
                None,
                int_e_xy,
                &wires::LH[..],
            );
            int_w_xy.x -= 1;
            for _ in 0..3 {
                int_w_xy.y -= 1;
                int_e_xy.y -= 1;
                builder.extract_pass_tile_id(
                    ccls::HDCM_W,
                    Dir::W,
                    int_e_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_w_xy,
                    &wires::LH[..],
                );
                builder.extract_pass_tile_id(
                    ccls::HDCM_E,
                    Dir::E,
                    int_w_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_e_xy,
                    &wires::LH[..],
                );
            }
        }
        for &xy in rd.tiles_by_kind_name("DCM_SPLY") {
            let mut int_w_xy = xy;
            let mut int_e_xy = xy;
            int_e_xy.x += 5;
            builder.extract_pass_tile_id(
                ccls::DSPHOLE_W,
                Dir::W,
                int_e_xy,
                None,
                None,
                None,
                None,
                None,
                int_w_xy,
                &wires::LH[..],
            );
            builder.extract_pass_tile_id(
                ccls::DSPHOLE_E,
                Dir::E,
                int_w_xy,
                None,
                None,
                None,
                None,
                None,
                int_e_xy,
                &wires::LH[..],
            );
            int_w_xy.x -= 1;
            for _ in 0..3 {
                int_w_xy.y += 1;
                int_e_xy.y += 1;
                builder.extract_pass_tile_id(
                    ccls::HDCM_W,
                    Dir::W,
                    int_e_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_w_xy,
                    &wires::LH[..],
                );
                builder.extract_pass_tile_id(
                    ccls::HDCM_E,
                    Dir::E,
                    int_w_xy,
                    None,
                    None,
                    None,
                    None,
                    None,
                    int_e_xy,
                    &wires::LH[..],
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
                builder.extract_xtile_id(
                    tcls::CLK_S_S3,
                    defs::bslots::CLK_INT,
                    xy,
                    &[],
                    &[xy_l],
                    "CLK_S_S3",
                    &[
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[0], "BUFGMUX", 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI0"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD0"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL0"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR0"])
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[1], "BUFGMUX", 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI1"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD1"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL1"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR1"])
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[2], "BUFGMUX", 2)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI2"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD2"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL2"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR2"])
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[3], "BUFGMUX", 3)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI3"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD3"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL3"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR3"])
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual(defs::bslots::GLOBALSIG_S[0]),
                    ],
                    &wires::LH[..],
                );
            } else if rd.family == "fpgacore" {
                builder.extract_xtile_id(
                    tcls::CLK_S_FC,
                    defs::bslots::CLK_INT,
                    xy,
                    &[],
                    &[xy_l],
                    "CLK_S_FC",
                    &[
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[0], "BUFG", 0)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI0"])
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[1], "BUFG", 1)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI1"])
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[2], "BUFG", 2)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI2"])
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[3], "BUFG", 3)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKB_CKI3"])
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual(defs::bslots::GLOBALSIG_S[0]),
                    ],
                    &wires::LH[..],
                );
            } else {
                let (tcid, naming) = if rd.family == "spartan3e" {
                    (tcls::CLK_S_S3E, "CLK_S_S3E")
                } else {
                    (tcls::CLK_S_S3A, "CLK_S_S3A")
                };
                builder.extract_xtile_id(
                    tcid,
                    defs::bslots::CLK_INT,
                    xy,
                    &[],
                    &[xy_l],
                    naming,
                    &[
                        builder
                            .bel_xy(defs::bslots::BUFGMUX[0], "BUFGMUX", 1, 1)
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
                            .bel_xy(defs::bslots::BUFGMUX[1], "BUFGMUX", 1, 0)
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
                            .bel_xy(defs::bslots::BUFGMUX[2], "BUFGMUX", 0, 1)
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
                            .bel_xy(defs::bslots::BUFGMUX[3], "BUFGMUX", 0, 0)
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
                        builder.bel_virtual(defs::bslots::GLOBALSIG_S[0]),
                    ],
                    &wires::LH[..],
                );
            }
        }
    }
    for tkn in ["CLKT", "CLKT_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            if rd.family == "spartan3" {
                builder.extract_xtile_id(
                    tcls::CLK_N_S3,
                    defs::bslots::CLK_INT,
                    xy,
                    &[],
                    &[xy_l],
                    "CLK_N_S3",
                    &[
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[0], "BUFGMUX", 4)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI0"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD0"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL0"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR0"])
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[1], "BUFGMUX", 5)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI1"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD1"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL1"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR1"])
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[2], "BUFGMUX", 6)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI2"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD2"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL2"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR2"])
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[3], "BUFGMUX", 7)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI3"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD3"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL3"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR3"])
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual(defs::bslots::GLOBALSIG_N[0]),
                    ],
                    &wires::LH[..],
                );
            } else if rd.family == "fpgacore" {
                builder.extract_xtile_id(
                    tcls::CLK_N_FC,
                    defs::bslots::CLK_INT,
                    xy,
                    &[],
                    &[xy_l],
                    "CLK_N_FC",
                    &[
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[0], "BUFG", 4)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI0"])
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[1], "BUFG", 5)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI1"])
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[2], "BUFG", 6)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI2"])
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_indexed(defs::bslots::BUFGMUX[3], "BUFG", 7)
                            .pin_name_only("I", 0)
                            .extra_wire("CKI", &["CLKT_CKI3"])
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual(defs::bslots::GLOBALSIG_N[0]),
                    ],
                    &wires::LH[..],
                );
            } else {
                let (tcid, naming) = if rd.family == "spartan3e" {
                    (tcls::CLK_N_S3E, "CLK_N_S3E")
                } else {
                    (tcls::CLK_N_S3A, "CLK_N_S3A")
                };
                builder.extract_xtile_id(
                    tcid,
                    defs::bslots::CLK_INT,
                    xy,
                    &[],
                    &[xy_l],
                    naming,
                    &[
                        builder
                            .bel_xy(defs::bslots::BUFGMUX[0], "BUFGMUX", 1, 1)
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
                            .bel_xy(defs::bslots::BUFGMUX[1], "BUFGMUX", 1, 0)
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
                            .bel_xy(defs::bslots::BUFGMUX[2], "BUFGMUX", 0, 1)
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
                            .bel_xy(defs::bslots::BUFGMUX[3], "BUFGMUX", 0, 0)
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
                        builder.bel_virtual(defs::bslots::GLOBALSIG_N[0]),
                    ],
                    &wires::LH[..],
                );
            }
        }
    }

    for (tkn, tcid, kind) in [
        ("BBTERM", tcls::DCMCONN_S, "DCMCONN_S"),
        ("BTTERM", tcls::DCMCONN_N, "DCMCONN_N"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = [xy.delta(0, if kind == "DCMCONN_S" { 1 } else { -1 })];
            if rd.tile_kinds.key(rd.tiles[&int_xy[0]].kind) == "BRAM_IOIS_NODCM" {
                continue;
            }
            builder.extract_xtile_bels_id(
                tcid,
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
                false,
            );
        }
    }

    if rd.family != "spartan3" && rd.family != "fpgacore" {
        for (tkn, nn, tcid_s3e, tcid_s3a) in [
            ("CLKL", "CLK_W", tcls::CLK_W_S3E, tcls::CLK_W_S3A),
            ("CLKR", "CLK_E", tcls::CLK_E_S3E, tcls::CLK_E_S3A),
        ] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                let xy_o = xy.delta(if xy.x == 0 { 1 } else { -1 }, 0);
                let int_s_xy = builder.walk_to_int(xy_o, Dir::S, false).unwrap();
                let int_n_xy = builder.walk_to_int(xy_o, Dir::N, false).unwrap();
                let int_xy = [int_s_xy, int_n_xy];
                let kind;
                let buf_xy;
                let mut bels = [
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[0], "BUFGMUX", 0, 0)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI0", "CLKR_CKI0"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT0", "CLKR_OUTT0"])
                        .extra_int_in("CLK", &["CLKL_GCLK0", "CLKR_GCLK0"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[1], "BUFGMUX", 0, 1)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI1", "CLKR_CKI1"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT1", "CLKR_OUTT1"])
                        .extra_int_in("CLK", &["CLKL_GCLK1", "CLKR_GCLK1"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[2], "BUFGMUX", 0, 2)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI2", "CLKR_CKI2"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT2", "CLKR_OUTT2"])
                        .extra_int_in("CLK", &["CLKL_GCLK2", "CLKR_GCLK2"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[3], "BUFGMUX", 0, 3)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI3", "CLKR_CKI3"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT3", "CLKR_OUTT3"])
                        .extra_int_in("CLK", &["CLKL_GCLK3", "CLKR_GCLK3"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[4], "BUFGMUX", 0, 4)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI4", "CLKR_CKI4"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB0", "CLKR_OUTB0"])
                        .extra_int_in("CLK", &["CLKL_GCLK4", "CLKR_GCLK4"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[5], "BUFGMUX", 0, 5)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI5", "CLKR_CKI5"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB1", "CLKR_OUTB1"])
                        .extra_int_in("CLK", &["CLKL_GCLK5", "CLKR_GCLK5"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[6], "BUFGMUX", 0, 6)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI6", "CLKR_CKI6"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB2", "CLKR_OUTB2"])
                        .extra_int_in("CLK", &["CLKL_GCLK6", "CLKR_GCLK6"]),
                    builder
                        .bel_xy(defs::bslots::BUFGMUX[7], "BUFGMUX", 0, 7)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI7", "CLKR_CKI7"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB3", "CLKR_OUTB3"])
                        .extra_int_in("CLK", &["CLKL_GCLK7", "CLKR_GCLK7"]),
                    builder
                        .bel_xy(defs::bslots::PCILOGICSE, "PCILOGIC", 0, 0)
                        .pin_name_only("PCI_CE", 1)
                        .pin_name_only("IRDY", 1)
                        .pin_name_only("TRDY", 1),
                    builder
                        .bel_xy(defs::bslots::VCC, "VCC", 0, 0)
                        .pin_name_only("VCCOUT", 0),
                    builder.bel_virtual(defs::bslots::GLOBALSIG_WE),
                ];
                let tcid;
                if rd.family == "spartan3e" {
                    kind = format!("{nn}_S3E");
                    tcid = tcid_s3e;
                    buf_xy = vec![];
                } else {
                    kind = format!("{nn}_S3A");
                    tcid = tcid_s3a;
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
                builder.extract_xtile_bels_id(tcid, xy, &buf_xy, &int_xy, &kind, &bels, false);
            }
        }

        for tkn in ["GCLKH_PCI_CE_N"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                builder.extract_xtile_bels_id(
                    tcls::PCI_CE_N,
                    xy,
                    &[],
                    &[],
                    "PCI_CE_N",
                    &[builder
                        .bel_virtual(defs::bslots::PCI_CE_N)
                        .extra_wire("I", &["GCLKH_PCI_CE_IN"])
                        .extra_wire("O", &["GCLKH_PCI_CE_OUT"])],
                    false,
                );
            }
        }
        for tkn in ["GCLKH_PCI_CE_S", "GCLKH_PCI_CE_S_50A"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                builder.extract_xtile_bels_id(
                    tcls::PCI_CE_S,
                    xy,
                    &[],
                    &[],
                    "PCI_CE_S",
                    &[builder
                        .bel_virtual(defs::bslots::PCI_CE_S)
                        .extra_wire("I", &["GCLKH_PCI_CE_OUT"])
                        .extra_wire("O", &["GCLKH_PCI_CE_IN"])],
                    false,
                );
            }
        }
        for tkn in ["LL", "LR", "UL", "UR"] {
            builder.extract_int_bels_id(
                tcls::PCI_CE_CNR,
                tkn,
                "PCI_CE_CNR",
                &[builder
                    .bel_virtual(defs::bslots::PCI_CE_CNR)
                    .extra_wire("I", &["PCI_CE_NS"])
                    .extra_wire("O", &["PCI_CE_EW"])],
            );
        }
        builder.db.tile_classes[tcls::PCI_CE_CNR].cells.clear();
        if rd.family == "spartan3a" {
            for &xy in rd.tiles_by_kind_name("GCLKV_IOISL") {
                builder.extract_xtile_bels_id(
                    tcls::PCI_CE_E,
                    xy,
                    &[],
                    &[],
                    "PCI_CE_E",
                    &[builder
                        .bel_virtual(defs::bslots::PCI_CE_E)
                        .extra_wire("I", &["CLKV_PCI_CE_W"])
                        .extra_wire("O", &["CLKV_PCI_CE_E"])],
                    false,
                );
            }
            for &xy in rd.tiles_by_kind_name("GCLKV_IOISR") {
                builder.extract_xtile_bels_id(
                    tcls::PCI_CE_W,
                    xy,
                    &[],
                    &[],
                    "PCI_CE_W",
                    &[builder
                        .bel_virtual(defs::bslots::PCI_CE_W)
                        .extra_wire("I", &["CLKV_PCI_CE_E"])
                        .extra_wire("O", &["CLKV_PCI_CE_W"])],
                    false,
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
                builder.extract_xtile_bels_id(
                    tcls::BRAM_S3ADSP,
                    xy,
                    &[],
                    &int_xy,
                    "BRAM_S3ADSP",
                    &[builder.bel_xy(defs::bslots::BRAM, "RAMB16", 0, 0)],
                    false,
                );
            }
        }
        for (tkn, naming) in [
            ("MACCSITE2", "DSP"),
            ("MACCSITE2_BRK", "DSP"),
            ("MACCSITE2_BOT", "DSP"),
            ("MACCSITE2_TOP", "DSP_TOP"),
        ] {
            let buf_cnt = if naming == "DSP_TOP" { 0 } else { 1 };
            let mut bel_dsp = builder
                .bel_xy(defs::bslots::DSP, "DSP48A", 0, 0)
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
                builder.extract_xtile_bels_id(tcls::DSP, xy, &[], &int_xy, naming, &bels_dsp, true);
                builder.extract_intf_tile_multi_id(
                    tcls::INTF_DSP,
                    xy,
                    &int_xy,
                    "INTF_DSP",
                    defs::bslots::INTF_TESTMUX,
                    false,
                    None,
                );
            }
        }
        let mut wires_c = BTreeSet::new();
        let tcls = builder.db.tile_classes.get("DSP").unwrap().1;
        let BelInfo::Legacy(ref bel) = tcls.bels[defs::bslots::DSP] else {
            unreachable!()
        };
        for i in 0..48 {
            for &w in &bel.pins[&format!("C{i}")].wires {
                wires_c.insert(w);
            }
        }

        let tcls = &mut builder.db.tile_classes[tcls::INTF_DSP];
        let BelInfo::TestMux(tm) = tcls.bels.remove(defs::bslots::INTF_TESTMUX).unwrap() else {
            unreachable!()
        };
        let mut gtm = GroupTestMux {
            bits: vec![],
            groups: vec![BitVec::new(), BitVec::new()],
            bits_primary: BitVec::new(),
            wires: Default::default(),
        };
        for (dst, tmux) in tm.wires {
            let mut gtmux = GroupTestMuxWire {
                primary_src: tmux.primary_src,
                test_src: vec![None, None],
            };
            let num = tmux.test_src.len();
            for src in tmux.test_src.into_keys() {
                let group = if num == 2 && !wires_c.contains(&src.tw) {
                    1
                } else {
                    0
                };
                assert_eq!(gtmux.test_src[group], None);
                gtmux.test_src[group] = Some(src);
            }
            gtm.wires.insert(dst, gtmux);
        }
        tcls.bels
            .insert(defs::bslots::INTF_TESTMUX, BelInfo::GroupTestMux(gtm));
    } else if rd.family != "fpgacore" {
        let (tcid, kind) = match &*rd.family {
            "spartan3" => (tcls::BRAM_S3, "BRAM_S3"),
            "spartan3e" => (tcls::BRAM_S3E, "BRAM_S3E"),
            "spartan3a" => (tcls::BRAM_S3A, "BRAM_S3A"),
            _ => unreachable!(),
        };
        for (tkn, naming) in [
            ("BRAMSITE", kind),
            ("BRAMSITE2", kind),
            ("BRAMSITE2_BRK", kind),
            ("BRAMSITE2_BOT", "BRAM_S3A_BOT"),
            ("BRAMSITE2_TOP", "BRAM_S3A_TOP"),
        ] {
            let mut bel_mult = builder.bel_xy(defs::bslots::MULT, "MULT18X18", 0, 0);
            let buf_cnt = if naming == "BRAM_S3A_TOP" { 0 } else { 1 };
            for i in 0..18 {
                bel_mult = bel_mult.pin_name_only(&format!("BCIN{i}"), 0);
                bel_mult = bel_mult.pin_name_only(&format!("BCOUT{i}"), buf_cnt);
            }
            let bels_bram = [builder.bel_xy(defs::bslots::BRAM, "RAMB16", 0, 0), bel_mult];
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                builder.extract_xtile_bels_id(tcid, xy, &[], &int_xy, naming, &bels_bram, false);
            }
        }
    }

    for tkn in ["CLKC", "CLKC_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual(defs::bslots::CLKC);
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("IN_B{i}"), &[format!("CLKC_GCLK_MAIN_B{i}")])
                    .extra_wire(format!("IN_T{i}"), &[format!("CLKC_GCLK_MAIN_T{i}")])
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("OUT{i}"), &[format!("CLKC_GCLK{i}")]);
            }
            builder.extract_xtile_bels_id(tcls::CLKC, xy, &[], &[], "CLKC", &[bel], false);
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC_50A") {
        let mut bel = builder.bel_virtual(defs::bslots::CLKC_50A);
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
        builder.extract_xtile_bels_id(tcls::CLKC_50A, xy, &[], &[], "CLKC_50A", &[bel], false);
    }

    for &xy in rd.tiles_by_kind_name("GCLKVM") {
        let mut bel = builder.bel_virtual(defs::bslots::CLKQC);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_CORE{i}"), &[format!("GCLKVM_GCLK{i}")])
                .extra_wire(format!("OUT_B{i}"), &[format!("GCLKVM_GCLK_DN{i}")])
                .extra_wire(format!("OUT_T{i}"), &[format!("GCLKVM_GCLK_UP{i}")]);
        }
        builder.extract_xtile_bels_id(tcls::CLKQC_S3, xy, &[], &[], "GCLKVM_S3", &[bel], false);
    }

    for tkn in ["GCLKVML", "GCLKVMR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual(defs::bslots::CLKQC);
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
            builder.extract_xtile_bels_id(tcls::CLKQC_S3E, xy, &[], &[], tkn, &[bel], false);
        }
    }

    for &xy in rd.tiles_by_kind_name("GCLKVC") {
        let mut bel = builder.bel_virtual(defs::bslots::HROW);
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN{i}"), &[format!("GCLKC_GCLK{i}")])
                .extra_wire(format!("OUT_L{i}"), &[format!("GCLKC_GCLK_OUT_L{i}")])
                .extra_wire(format!("OUT_R{i}"), &[format!("GCLKC_GCLK_OUT_R{i}")]);
        }
        builder.extract_xtile_bels_id(tcls::HROW, xy, &[], &[], "GCLKVC", &[bel], false);
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
            let mut bel = builder.bel_virtual(defs::bslots::HCLK);
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN{i}"), &[format!("GCLKH_GCLK{i}")])
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

    if rd.family != "spartan3" && rd.family != "fpgacore" {
        let dummy_xy = Coord { x: 0, y: 0 };
        let bel_globalsig = builder.bel_virtual(defs::bslots::GLOBALSIG);
        let mut bel = builder.bel_virtual(defs::bslots::HCLK);
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_T{i}"),
                    TileWireCoord::new_idx(1, wires::GCLK[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_UP{i}"),
                )
                .extra_int_out_force(
                    format!("OUT_B{i}"),
                    TileWireCoord::new_idx(0, wires::GCLK[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_DN{i}"),
                );
        }
        builder.extract_xtile_bels_id(
            if rd.family == "spartan3e" {
                tcls::HCLK
            } else {
                tcls::HCLK_UNI
            },
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "HCLK_BRAM",
            &[bel_globalsig.clone(), bel],
            false,
        );
        let mut bel = builder.bel_virtual(defs::bslots::HCLK);
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_B{i}"),
                    TileWireCoord::new_idx(0, wires::GCLK[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_DN{i}"),
                );
        }
        builder.extract_xtile_bels_id(
            if rd.family == "spartan3e" {
                tcls::HCLK_S
            } else {
                tcls::HCLK_UNI_S
            },
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "HCLK_BRAM_S",
            &[bel_globalsig.clone(), bel],
            false,
        );
        let mut bel = builder.bel_virtual(defs::bslots::HCLK);
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_T{i}"),
                    TileWireCoord::new_idx(1, wires::GCLK[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_UP{i}"),
                )
        }
        builder.extract_xtile_bels_id(
            if rd.family == "spartan3e" {
                tcls::HCLK_N
            } else {
                tcls::HCLK_UNI_N
            },
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "HCLK_BRAM_N",
            &[bel_globalsig.clone(), bel],
            false,
        );
        builder.extract_xtile_bels_id(
            tcls::HCLK_0,
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "HCLK_0",
            &[bel_globalsig],
            false,
        );
        builder.extract_xtile_bels_id(
            tcls::HCLK_DSP,
            dummy_xy,
            &[],
            &[],
            "HCLK_DSP",
            &[builder.bel_virtual(defs::bslots::GLOBALSIG_DSP)],
            false,
        );
    }

    builder.build()
}
