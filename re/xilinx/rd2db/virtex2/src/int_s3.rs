use std::collections::{BTreeMap, BTreeSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{Bel, BelInfo, BelInput, IntDb, Mux, SwitchBox, SwitchBoxItem, TileWireCoord},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{
    BelNaming, BelPinNaming, NamingDb, RawTileId, TileClassNaming, WireNaming,
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_virtex2::{
    chip::ChipKind,
    defs::{
        self, bcls, bslots,
        spartan3::{ccls, tcls, wires},
    },
    iob::get_iob_tiles,
};

use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};

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
        builder.mark_permabuf(wires::DCM_CLKPAD[i]);
        builder.mark_permabuf(wires::DCM_BUS[i]);
        builder.wire_names(
            wires::DCM_CLKPAD[i],
            &[
                format!("BRAM_IOIS_DLL_CLKPAD{i}"),
                format!("DCM_DLL_CLKPAD{i}"),
                format!("DCM_H_DLL_CLKPAD{i}"),
            ],
        );
    }
    if matches!(rd.family.as_str(), "spartan3" | "fpgacore") {
        for i in 0..4 {
            for tkn in ["CLKB", "CLKT"] {
                builder.extra_name_sub(format!("{tkn}_DLL_OUTL{i}"), 0, wires::DCM_BUS[i]);
                builder.extra_name_sub(format!("{tkn}_DLL_OUTR{i}"), 1, wires::DCM_BUS[i]);
                builder.extra_name_sub(format!("{tkn}_DLL_CLKPAD{i}"), 1, wires::DCM_CLKPAD[i]);
            }
            builder.extra_name(format!("BBTERM_CKMUX{i}"), wires::DCM_BUS[i]);
            builder.extra_name(format!("TTERM_CKMUX{i}"), wires::DCM_BUS[i]);
        }
        for i in 0..2 {
            for tkn in ["CLKB", "CLKT"] {
                builder.extra_name_sub(
                    format!("{tkn}_CKI{ii}", ii = i + 2),
                    0,
                    wires::OUT_CLKPAD[i],
                );
                builder.extra_name_sub(format!("{tkn}_CKI{i}"), 1, wires::OUT_CLKPAD[i]);
            }
        }
    } else {
        for i in 0..4 {
            for tkn in ["CLKB", "CLKT"] {
                builder.extra_name_sub(format!("{tkn}_DLL_OUTL{i}"), 3, wires::DCM_BUS[i]);
                builder.extra_name_sub(format!("{tkn}_DLL_OUTR{i}"), 4, wires::DCM_BUS[i]);
            }
            builder.extra_name_sub(
                format!("CLKB_DLL_CLKPAD{ii}", ii = 4 + i),
                3,
                wires::DCM_CLKPAD[i],
            );
            builder.extra_name_sub(format!("CLKB_DLL_CLKPAD{i}"), 4, wires::DCM_CLKPAD[i]);
            builder.extra_name_sub(
                format!("CLKT_DLL_CLKPAD{ii}", ii = 4 + i),
                3,
                wires::DCM_CLKPAD[i],
            );
            let x = if rd.family == "spartan3e" { 2 } else { 0 };
            builder.extra_name_sub(
                format!("CLKT_DLL_CLKPAD{ii}", ii = i ^ x),
                4,
                wires::DCM_CLKPAD[i],
            );
        }
        let (clk_s_pads, clk_n_pads, clk_w_pads, clk_e_pads) = if rd.family == "spartan3e" {
            ([4, 5, 1, 3], [6, 4, 3, 2], [7, 5, 3, 1], [4, 6, 0, 2])
        } else {
            ([4, 5, 2, 3], [5, 4, 3, 2], [6, 5, 3, 2], [4, 5, 1, 2])
        };
        for i in 0..8 {
            builder.extra_name_sub(
                format!("CLKB_CKI{i}"),
                clk_s_pads[i / 2],
                wires::OUT_CLKPAD[i % 2],
            );
            builder.extra_name_sub(
                format!("CLKT_CKI{i}"),
                clk_n_pads[i / 2],
                wires::OUT_CLKPAD[i % 2],
            );
            builder.extra_name_sub(
                format!("CLKL_CKI{i}"),
                clk_w_pads[i / 2],
                if rd.family == "spartan3e" {
                    wires::OUT_CLKPAD[i % 2]
                } else {
                    wires::OUT_CLKPAD[1 - i % 2]
                },
            );
            builder.extra_name_sub(
                format!("CLKR_CKI{i}"),
                clk_e_pads[i / 2],
                wires::OUT_CLKPAD[i % 2],
            );
        }
        builder.wire_names(
            wires::DCM_BUS[0],
            &[
                "DCM_OMUX10_CLKOUTL0",
                "DCM_OMUX10_CLKOUTR0",
                "DCM_OMUX10_CLKOUT0",
            ],
        );
        builder.wire_names(
            wires::DCM_BUS[1],
            &[
                "DCM_OMUX11_CLKOUTL1",
                "DCM_OMUX11_CLKOUTR1",
                "DCM_OMUX11_CLKOUT1",
            ],
        );
        builder.wire_names(
            wires::DCM_BUS[2],
            &[
                "DCM_OMUX12_CLKOUTL2",
                "DCM_OMUX12_CLKOUTR2",
                "DCM_OMUX12_CLKOUT2",
            ],
        );
        builder.wire_names(
            wires::DCM_BUS[3],
            &[
                "DCM_OMUX15_CLKOUTL3",
                "DCM_OMUX15_CLKOUTR3",
                "DCM_OMUX15_CLKOUT3",
            ],
        );
    }

    let clk_sn_sub = if matches!(rd.family.as_str(), "spartan3" | "fpgacore") {
        0
    } else {
        3
    };
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
                builder.extra_name_sub(format!("CLKB_TO_OMUX{i}"), clk_sn_sub, omux_da1);
            }
            (Some(i), Dir::S) => {
                builder.extra_name_sub(format!("CLKT_TO_OMUX{i}"), clk_sn_sub, omux_da1);
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
        builder.mark_optinv(wires::IMUX_SR[i], wires::IMUX_SR_OPTINV[i]);
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
        builder.mark_optinv(wires::IMUX_CLK[i], wires::IMUX_CLK_OPTINV[i]);
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
        builder.mark_optinv(wires::IMUX_CE[i], wires::IMUX_CE_OPTINV[i]);
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
        builder.mark_test_mux_in(wires::OUT_FAN_BEL[i], wires::OUT_FAN[i]);
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
        builder.mark_test_mux_in(wires::OUT_SEC_BEL[i], wires::OUT_SEC[i]);
    }
    builder.stub_out("STUB_IOIS_X3");
    builder.stub_out("STUB_IOIS_Y3");
    builder.stub_out("STUB_IOIS_XQ3");
    builder.stub_out("STUB_IOIS_YQ3");

    for (j, (ws, tmin)) in [
        (wires::OUT_HALF0, wires::OUT_HALF0_BEL),
        (wires::OUT_HALF1, wires::OUT_HALF1_BEL),
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

    if matches!(rd.family.as_str(), "spartan3" | "fpgacore") {
        for i in 0..4 {
            for tkn in ["CLKB", "CLKT"] {
                builder.extra_name_sub(format!("{tkn}_SELDUB{i}"), 1, wires::IMUX_BUFG_SEL[i]);
                builder.extra_name_sub(format!("{tkn}_CLKDUB{i}"), 1, wires::IMUX_BUFG_CLK_INT[i]);
                builder.extra_name_sub(format!("{tkn}_GCLK{i}"), 1, wires::IMUX_BUFG_CLK[i]);
            }
            builder.extra_name_sub(format!("CLKB_GCLK_MAIN{i}"), 1, wires::GCLK_S[i]);
            builder.extra_name_sub(format!("CLKT_GCLK_MAIN{i}"), 1, wires::GCLK_N[i]);
        }
    } else {
        for i in 0..4 {
            for tkn in ["CLKB", "CLKT"] {
                builder.extra_name_sub(format!("{tkn}_SELDUB{i}"), 4, wires::IMUX_BUFG_SEL[i]);
                builder.extra_name_sub(format!("{tkn}_CLKDUB{i}"), 4, wires::IMUX_BUFG_CLK_INT[i]);
                builder.extra_name_sub(format!("{tkn}_GCLK{i}"), 4, wires::IMUX_BUFG_CLK[i]);
            }
            builder.extra_name_sub(format!("CLKB_GCLK_MAIN{i}"), 4, wires::GCLK_S[i]);
            builder.extra_name_sub(format!("CLKT_GCLK_MAIN{i}"), 4, wires::GCLK_N[i]);
            for tkn in ["CLKL", "CLKR"] {
                builder.extra_name_sub(format!("{tkn}_GCLK{i}"), 4, wires::IMUX_BUFG_CLK[i]);
                builder.extra_name_sub(
                    format!("{tkn}_GCLK{ii}", ii = i + 4),
                    3,
                    wires::IMUX_BUFG_CLK[i],
                );
                builder.extra_name_sub(format!("{tkn}_OUTB{i}"), 3, wires::DCM_BUS[i]);
                builder.extra_name_sub(format!("{tkn}_OUTT{i}"), 4, wires::DCM_BUS[i]);
            }
        }
    }
    for i in 0..4 {
        builder.extra_name_sub(format!("CLKC_50A_GCLKB{i}"), 1, wires::GCLK_S[i]);
        builder.extra_name_sub(format!("CLKC_50A_GCLKT{i}"), 1, wires::GCLK_N[i]);
        builder.extra_name_sub(format!("GCLKVML_GCLKCORE{i}"), 1, wires::GCLK_S[i]);
        builder.extra_name_sub(
            format!("GCLKVML_GCLKCORE{ii}", ii = i + 4),
            1,
            wires::GCLK_N[i],
        );
        builder.extra_name_sub(format!("GCLKVMR_GCLKCORE{i}"), 1, wires::GCLK_S[i]);
        builder.extra_name_sub(
            format!("GCLKVMR_GCLKCORE{ii}", ii = i + 4),
            1,
            wires::GCLK_N[i],
        );
        builder.extra_name_sub(format!("GCLKVM_GCLK{i}"), 1, wires::GCLK_S[i]);
        builder.extra_name_sub(format!("GCLKVM_GCLK{ii}", ii = i + 4), 1, wires::GCLK_N[i]);
    }
    for i in 0..8 {
        builder.extra_name_sub(format!("CLKC_50A_GCLK_OUT_LH{i}"), 0, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("CLKC_50A_GCLK_OUT_RH{i}"), 1, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("GCLKVM_GCLK_DN{i}"), 0, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("GCLKVM_GCLK_UP{i}"), 1, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("GCLKVMLR_GCLK_DN{i}"), 0, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("GCLKVMLR_GCLK_UP{i}"), 1, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("GCLKH_GCLK{i}"), 1, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("BRAMSITE2_GCLKH_GCLK{i}"), 1, wires::GCLK_QUAD[i]);
        builder.extra_name_sub(format!("GCLKVML_GCLKLR{i}"), 1, wires::GCLK_WE[i]);
        builder.extra_name_sub(format!("GCLKVMR_GCLKLR{i}"), 1, wires::GCLK_WE[i]);
        builder.extra_name_sub(format!("CLKC_50A_GCLK_IN_LH{i}"), 0, wires::GCLK_WE[i]);
        builder.extra_name_sub(format!("CLKC_50A_GCLK_IN_RH{i}"), 1, wires::GCLK_WE[i]);
    }

    if rd.family == "fpgacore" {
        builder.extract_int_id(tcls::INT_CLB_FC, bslots::INT, "CENTER", "INT_CLB_FC", &[]);
        builder.extract_int_id(
            tcls::INT_CLB_FC,
            bslots::INT,
            "CENTER_SMALL",
            "INT_CLB_FC",
            &[],
        );
    } else {
        builder.extract_int_id(tcls::INT_CLB, bslots::INT, "CENTER", "INT_CLB", &[]);
        builder.extract_int_id(tcls::INT_CLB, bslots::INT, "CENTER_SMALL", "INT_CLB", &[]);
        builder.extract_int_id(
            tcls::INT_CLB,
            bslots::INT,
            "CENTER_SMALL_BRK",
            "INT_CLB_BRK",
            &[],
        );
    }
    if rd.family.starts_with("spartan3a") {
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIOIS",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIOIS_BRK",
            "INT_IOI_S3A_WE_BRK",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIOIS_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIOIS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIOIS_CLK_PCI_BRK",
            "INT_IOI_S3A_WE_BRK",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIBUFS",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIBUFS_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "LIBUFS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIOIS",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIOIS_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIOIS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIBUFS",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIBUFS_BRK",
            "INT_IOI_S3A_WE_BRK",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIBUFS_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIBUFS_CLK_PCI",
            "INT_IOI_S3A_WE",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_WE,
            bslots::INT,
            "RIBUFS_CLK_PCI_BRK",
            "INT_IOI_S3A_WE_BRK",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            bslots::INT,
            "BIOIS",
            "INT_IOI_S3A_SN",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            bslots::INT,
            "BIOIB",
            "INT_IOI_S3A_SN",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            bslots::INT,
            "TIOIS",
            "INT_IOI_S3A_SN",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_IOI_S3A_SN,
            bslots::INT,
            "TIOIB",
            "INT_IOI_S3A_SN",
            &[],
        );
    } else if rd.family == "spartan3e" {
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "LIOIS", "INT_IOI", &[]);
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            bslots::INT,
            "LIOIS_BRK",
            "INT_IOI_BRK",
            &[],
        );
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "LIOIS_PCI", "INT_IOI", &[]);
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            bslots::INT,
            "LIOIS_CLK_PCI",
            "INT_IOI",
            &[],
        );
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "LIBUFS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "LIBUFS_PCI", "INT_IOI", &[]);
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            bslots::INT,
            "LIBUFS_CLK_PCI",
            "INT_IOI",
            &[],
        );
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "RIOIS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "RIOIS_PCI", "INT_IOI", &[]);
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            bslots::INT,
            "RIOIS_CLK_PCI",
            "INT_IOI",
            &[],
        );
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "RIBUFS", "INT_IOI", &[]);
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            bslots::INT,
            "RIBUFS_BRK",
            "INT_IOI_BRK",
            &[],
        );
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "RIBUFS_PCI", "INT_IOI", &[]);
        builder.extract_int_id(
            tcls::INT_IOI_S3E,
            bslots::INT,
            "RIBUFS_CLK_PCI",
            "INT_IOI",
            &[],
        );
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "BIOIS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "BIBUFS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "TIOIS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3E, bslots::INT, "TIBUFS", "INT_IOI", &[]);
    } else if rd.family == "fpgacore" {
        builder.extract_int_id(tcls::INT_IOI_FC, bslots::INT, "LIOIS", "INT_IOI_FC", &[]);
        builder.extract_int_id(tcls::INT_IOI_FC, bslots::INT, "RIOIS", "INT_IOI_FC", &[]);
        builder.extract_int_id(tcls::INT_IOI_FC, bslots::INT, "BIOIS", "INT_IOI_FC", &[]);
        builder.extract_int_id(tcls::INT_IOI_FC, bslots::INT, "TIOIS", "INT_IOI_FC", &[]);
    } else {
        // NOTE: could be unified by pulling extra muxes from CLB
        builder.extract_int_id(tcls::INT_IOI_S3, bslots::INT, "LIOIS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3, bslots::INT, "RIOIS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3, bslots::INT, "BIOIS", "INT_IOI", &[]);
        builder.extract_int_id(tcls::INT_IOI_S3, bslots::INT, "TIOIS", "INT_IOI", &[]);
    }
    // NOTE:
    // - S3/S3E/S3A could be unified by pulling some extra muxes from CLB
    // - S3A/S3ADSP adds VCC input to B[XY] and splits B[XY] to two wires
    if rd.family == "spartan3adsp" {
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM_S3ADSP",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM0_SMALL_BOT",
            "INT_BRAM_S3ADSP",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM_S3ADSP",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM_S3ADSP",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM_S3ADSP",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM3_SMALL_TOP",
            "INT_BRAM_S3ADSP",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "BRAM3_SMALL_BRK",
            "INT_BRAM_S3ADSP_BRK",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC0_SMALL",
            "INT_MACC",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC0_SMALL_BOT",
            "INT_MACC",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC1_SMALL",
            "INT_MACC",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC2_SMALL",
            "INT_MACC",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC3_SMALL",
            "INT_MACC",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC3_SMALL_TOP",
            "INT_MACC",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3ADSP,
            bslots::INT,
            "MACC3_SMALL_BRK",
            "INT_MACC_BRK",
            &[],
        );
    } else if rd.family == "spartan3a" {
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            bslots::INT,
            "BRAM0_SMALL_BOT",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_12,
            bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_12,
            bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            bslots::INT,
            "BRAM3_SMALL_TOP",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3A_03,
            bslots::INT,
            "BRAM3_SMALL_BRK",
            "INT_BRAM_BRK",
            &[],
        );

        if let Some(pips) = builder.pips.get_mut(&(tcls::INT_BRAM_S3A_03, bslots::INT)) {
            pips.pips.retain(|&(wt, _), _| {
                !wires::IMUX_CE.contains(wt.wire) && !wires::IMUX_CLK.contains(wt.wire)
            });
        }
    } else if rd.family == "spartan3e" {
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3E,
            bslots::INT,
            "BRAM3_SMALL_BRK",
            "INT_BRAM_BRK",
            &[],
        );
    } else {
        builder.extract_int_id(tcls::INT_BRAM_S3, bslots::INT, "BRAM0", "INT_BRAM", &[]);
        builder.extract_int_id(tcls::INT_BRAM_S3, bslots::INT, "BRAM1", "INT_BRAM", &[]);
        builder.extract_int_id(tcls::INT_BRAM_S3, bslots::INT, "BRAM2", "INT_BRAM", &[]);
        builder.extract_int_id(tcls::INT_BRAM_S3, bslots::INT, "BRAM3", "INT_BRAM", &[]);
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            bslots::INT,
            "BRAM0_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            bslots::INT,
            "BRAM1_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            bslots::INT,
            "BRAM2_SMALL",
            "INT_BRAM",
            &[],
        );
        builder.extract_int_id(
            tcls::INT_BRAM_S3,
            bslots::INT,
            "BRAM3_SMALL",
            "INT_BRAM",
            &[],
        );
    }
    builder.extract_int_id(tcls::INT_DCM, bslots::INT, "BRAM_IOIS", "INT_DCM_S3", &[]);
    for &xy in rd.tiles_by_kind_name("BRAM_IOIS") {
        builder
            .xtile_id(tcls::INT_DCM, "INT_DCM_S3", xy)
            .switchbox(bslots::PTE2OMUX)
            .optin_muxes(&wires::OUT_SEC[..])
            .extract();
    }
    builder.extract_int_id(
        tcls::INT_DCM_S3_DUMMY,
        bslots::INT,
        "BRAM_IOIS_NODCM",
        "INT_DCM_S3_DUMMY",
        &[],
    );
    for tkn in ["DCMAUX_BL_CENTER", "DCMAUX_TL_CENTER"] {
        builder.extract_int_id(
            tcls::INT_DCM_S3E_DUMMY,
            bslots::INT,
            tkn,
            "INT_DCM_S3E_DUMMY",
            &[],
        );
        for &xy in rd.tiles_by_kind_name(tkn) {
            builder
                .xtile_id(tcls::INT_DCM_S3E_DUMMY, "INT_DCM_S3E_DUMMY", xy)
                .switchbox(bslots::PTE2OMUX)
                .optin_muxes(&wires::OUT_SEC[..])
                .extract();
        }
    }
    for (tkn, naming) in [
        ("DCM_BL_CENTER", "INT_DCM_S3E"),
        ("DCM_TL_CENTER", "INT_DCM_S3E"),
        ("DCM_BR_CENTER", "INT_DCM_S3E"),
        ("DCM_TR_CENTER", "INT_DCM_S3E"),
        ("DCM_H_BL_CENTER", "INT_DCM_S3E_H"),
        ("DCM_H_TL_CENTER", "INT_DCM_S3E_H"),
        ("DCM_H_BR_CENTER", "INT_DCM_S3E_H"),
        ("DCM_H_TR_CENTER", "INT_DCM_S3E_H"),
        ("DCM_BGAP", "INT_DCM_S3E_H"),
        ("DCM_SPLY", "INT_DCM_S3E_H"),
    ] {
        builder.extract_int_id(tcls::INT_DCM, bslots::INT, tkn, naming, &[]);
        for &xy in rd.tiles_by_kind_name(tkn) {
            builder
                .xtile_id(tcls::INT_DCM, naming, xy)
                .switchbox(bslots::PTE2OMUX)
                .optin_muxes(&wires::OUT_SEC[..])
                .extract();
        }
        if let Some((_, ntcls)) = builder.ndb.tile_class_namings.get_mut(naming) {
            for i in 0..4 {
                ntcls
                    .wires
                    .remove(&TileWireCoord::new_idx(0, wires::DCM_BUS[i]));
            }
        }
    }
    for tcid in [tcls::INT_DCM, tcls::INT_DCM_S3E_DUMMY] {
        if let Some(pips) = builder.pips.get_mut(&(tcid, bslots::PTE2OMUX)) {
            let mut new_pips = vec![];
            pips.pips.retain(|&(wt, wf), &mut mode| {
                if let Some(idx) = wires::IMUX_CLK.index_of(wf.wire) {
                    new_pips.push((
                        (
                            wt,
                            TileWireCoord::new_idx(0, wires::IMUX_CLK_OPTINV[idx]).neg(),
                        ),
                        mode,
                    ));
                    false
                } else {
                    true
                }
            });
            pips.pips.extend(new_pips);
        }
    }
    for naming in [
        "INT_DCM_S3",
        "INT_DCM_S3E",
        "INT_DCM_S3E_H",
        "INT_DCM_S3E_DUMMY",
    ] {
        if let Some((_, ntcls)) = builder.ndb.tile_class_namings.get_mut(naming) {
            for i in 1..4 {
                let wn = ntcls.wires[&TileWireCoord::new_idx(0, wires::IMUX_CLK[i])].clone();
                ntcls
                    .wires
                    .insert(TileWireCoord::new_idx(0, wires::IMUX_CLK_OPTINV[i]), wn);
            }
        }
    }
    let (int_clb, int_cnr) = if rd.family == "fpgacore" {
        (tcls::INT_CLB_FC, "INT_CNR_FC")
    } else {
        (tcls::INT_CLB, "INT_CNR")
    };
    builder.extract_int_id(int_clb, bslots::INT, "LL", int_cnr, &[]);
    builder.extract_int_id(int_clb, bslots::INT, "LR", int_cnr, &[]);
    builder.extract_int_id(int_clb, bslots::INT, "UL", int_cnr, &[]);
    builder.extract_int_id(int_clb, bslots::INT, "UR", int_cnr, &[]);

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    let bels_clb = [
        builder
            .bel_xy(bslots::SLICE[0], "SLICE", 0, 0)
            .pins_name_only(&slicem_name_only),
        builder
            .bel_xy(bslots::SLICE[1], "SLICE", 1, 0)
            .pins_name_only(&slicel_name_only),
        builder
            .bel_xy(bslots::SLICE[2], "SLICE", 0, 1)
            .pins_name_only(&slicem_name_only)
            .extra_wire("COUT_N", &["COUT_N1"])
            .extra_wire("FX_S", &["FX_S2"]),
        builder
            .bel_xy(bslots::SLICE[3], "SLICE", 1, 1)
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
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "IOIS_IBUF0",
                )
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "IOIS_IBUF1",
                )
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bslots::IOI[2], "IOB", 2)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_int_bels_id(tcls::IOI_S3, "LIOIS", "IOI_S3_W", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_S3, "RIOIS", "IOI_S3_E", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_S3, "BIOIS", "IOI_S3_S", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_S3, "TIOIS", "IOI_S3_N", &bels_ioi);
    } else if rd.family == "fpgacore" {
        let bels_ioi = [
            builder
                .bel_indexed(bslots::IREG[0], "IBUF", 0)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "IOIS_IBUF0",
                ),
            builder
                .bel_indexed(bslots::IREG[1], "IBUF", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "IOIS_IBUF1",
                ),
            builder.bel_indexed(bslots::IREG[2], "IBUF", 2),
            builder.bel_indexed(bslots::IREG[3], "IBUF", 3),
            builder.bel_indexed(bslots::OREG[0], "OBUF", 0),
            builder.bel_indexed(bslots::OREG[1], "OBUF", 1),
            builder.bel_indexed(bslots::OREG[2], "OBUF", 2),
            builder.bel_indexed(bslots::OREG[3], "OBUF", 3),
        ];
        builder.extract_int_bels_id(tcls::IOI_FC, "LIOIS", "IOI_FC_W", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_FC, "RIOIS", "IOI_FC_E", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_FC, "BIOIS", "IOI_FC_S", &bels_ioi);
        builder.extract_int_bels_id(tcls::IOI_FC, "TIOIS", "IOI_FC_N", &bels_ioi);
    } else if rd.family == "spartan3e" {
        let bels_ioi_tb = [
            builder
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "IOIS_IBUF0",
                )
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "IOIS_IBUF1",
                )
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "LIOIS_IBUF1",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "LIOIS_IBUF0",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed(bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "RIOIS_IBUF1",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "RIOIS_IBUF0",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed(bslots::IOI[2], "IOB", 2)
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
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "IOIS_IBUF0",
                )
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "IOIS_IBUF1",
                )
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed(bslots::IOI[2], "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "LIOIS_IBUF0",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "LIOIS_IBUF1",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed(bslots::IOI[0], "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[0]),
                    "RIOIS_IBUF1",
                )
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed(bslots::IOI[1], "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_int_out_force(
                    "CLKPAD",
                    TileWireCoord::new_idx(0, wires::OUT_CLKPAD[1]),
                    "RIOIS_IBUF0",
                )
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
            .bel_xy(bslots::RANDOR, "RANDOR", 0, 0)
            .pins_name_only(&["CIN0", "CIN1", "CPREV", "O"])];
        builder.extract_int_bels_id(tcid_randor, "BIOIS", "RANDOR_S", &bels_randor_b);
        builder.extract_int_bels_id(tcid_randor, "BIOIB", "RANDOR_S", &bels_randor_b);
        builder.extract_int_bels_id(tcid_randor, "BIBUFS", "RANDOR_S", &bels_randor_b);
    }
    let bels_randor_t = [builder
        .bel_xy(bslots::RANDOR, "RANDOR", 0, 0)
        .pins_name_only(&["CIN0", "CIN1"])
        .pin_name_only("CPREV", 1)
        .pin_name_only("O", 1)];
    builder.extract_int_bels_id(tcid_randor, "TIOIS", "RANDOR_N", &bels_randor_t);
    builder.extract_int_bels_id(tcid_randor, "TIOIB", "RANDOR_N", &bels_randor_t);
    builder.extract_int_bels_id(tcid_randor, "TIBUFS", "RANDOR_N", &bels_randor_t);
    builder.db.tile_classes[tcid_randor].cells.clear();
    if rd.family == "spartan3" {
        let bels_dcm = [builder.bel_xy(bslots::DCM, "DCM", 0, 0)];
        builder.extract_int_bels_id(tcls::DCM_S3, "BRAM_IOIS", "DCM_S3", &bels_dcm);
    } else if rd.family != "fpgacore" {
        for (tcid, tkn, naming) in [
            (tcls::DCM_S3E_SW, "DCM_BL_CENTER", "DCM_S3E_W"),
            (tcls::DCM_S3E_NW, "DCM_TL_CENTER", "DCM_S3E_W"),
            (tcls::DCM_S3E_SE, "DCM_BR_CENTER", "DCM_S3E_E"),
            (tcls::DCM_S3E_NE, "DCM_TR_CENTER", "DCM_S3E_E"),
            (tcls::DCM_S3E_WS, "DCM_H_BL_CENTER", "DCM_S3E_H"),
            (tcls::DCM_S3E_WN, "DCM_H_TL_CENTER", "DCM_S3E_H"),
            (tcls::DCM_S3E_ES, "DCM_H_BR_CENTER", "DCM_S3E_H"),
            (tcls::DCM_S3E_EN, "DCM_H_TR_CENTER", "DCM_S3E_H"),
            (tcls::DCM_S3E_WS, "DCM_BGAP", "DCM_S3E_H"),
            (tcls::DCM_S3E_WN, "DCM_SPLY", "DCM_S3E_H"),
        ] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                let bel = builder.bel_xy(bslots::DCM, "DCM", 0, 0);
                builder
                    .xtile_id(tcid, naming, xy)
                    .bel(bel)
                    .switchbox(bslots::DCM_INT)
                    .optin_muxes(&wires::DCM_BUS[..])
                    .ref_int(xy, 0)
                    .extract();
            }
        }
    }

    if rd.family == "spartan3" {
        builder.extract_int_bels_id(
            tcls::CNR_SW_S3,
            "LL",
            "CNR_SW_S3",
            &[
                builder.bel_indexed(bslots::DCI[0], "DCI", 6),
                builder.bel_indexed(bslots::DCI[1], "DCI", 5),
                builder.bel_indexed(bslots::DCIRESET[0], "DCIRESET", 6),
                builder.bel_indexed(bslots::DCIRESET[1], "DCIRESET", 5),
                builder.bel_virtual(bslots::MISC_CNR_S3),
                builder.bel_virtual(bslots::MISC_SW),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_SE_S3,
            "LR",
            "CNR_SE_S3",
            &[
                builder.bel_indexed(bslots::DCI[0], "DCI", 3),
                builder.bel_indexed(bslots::DCI[1], "DCI", 4),
                builder.bel_indexed(bslots::DCIRESET[0], "DCIRESET", 3),
                builder.bel_indexed(bslots::DCIRESET[1], "DCIRESET", 4),
                builder.bel_single(bslots::STARTUP, "STARTUP"),
                builder.bel_single(bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(bslots::ICAP, "ICAP"),
                builder.bel_virtual(bslots::MISC_CNR_S3),
                builder.bel_virtual(bslots::MISC_SE),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_S3,
            "UL",
            "CNR_NW_S3",
            &[
                builder.bel_indexed(bslots::DCI[0], "DCI", 7),
                builder.bel_indexed(bslots::DCI[1], "DCI", 0),
                builder.bel_indexed(bslots::DCIRESET[0], "DCIRESET", 7),
                builder.bel_indexed(bslots::DCIRESET[1], "DCIRESET", 0),
                builder.bel_single(bslots::PMV, "PMV"),
                builder.bel_virtual(bslots::MISC_CNR_S3),
                builder.bel_virtual(bslots::MISC_NW),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_S3,
            "UR",
            "CNR_NE_S3",
            &[
                builder.bel_indexed(bslots::DCI[0], "DCI", 2),
                builder.bel_indexed(bslots::DCI[1], "DCI", 1),
                builder.bel_indexed(bslots::DCIRESET[0], "DCIRESET", 2),
                builder.bel_indexed(bslots::DCIRESET[1], "DCIRESET", 1),
                builder.bel_single(bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
                builder.bel_virtual(bslots::MISC_CNR_S3),
                builder.bel_virtual(bslots::MISC_NE),
            ],
        );
    } else if rd.family == "fpgacore" {
        builder.extract_int_bels_id(
            tcls::CNR_SW_FC,
            "LL",
            "CNR_SW_FC",
            &[builder.bel_virtual(bslots::MISC_SW)],
        );
        builder.extract_int_bels_id(
            tcls::CNR_SE_FC,
            "LR",
            "CNR_SE_FC",
            &[
                builder.bel_single(bslots::STARTUP, "STARTUP"),
                builder.bel_single(bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(bslots::ICAP, "ICAP"),
                builder.bel_virtual(bslots::MISC_SE),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_FC,
            "UL",
            "CNR_NW_FC",
            &[
                builder.bel_single(bslots::PMV, "PMV"),
                builder.bel_virtual(bslots::MISC_NW),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_FC,
            "UR",
            "CNR_NE_FC",
            &[
                builder.bel_single(bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
                builder.bel_virtual(bslots::MISC_NE),
            ],
        );
        for (tcid, nn) in [
            (tcls::CNR_SW_FC, "CNR_SW_FC"),
            (tcls::CNR_SE_FC, "CNR_SE_FC"),
            (tcls::CNR_NW_FC, "CNR_NW_FC"),
            (tcls::CNR_NE_FC, "CNR_NE_FC"),
        ] {
            let mut bel = Bel::default();
            bel.inputs.insert(
                bcls::MISR_FC::CLK,
                BelInput::Fixed(TileWireCoord::new_idx(0, wires::IMUX_CLK_OPTINV[3]).pos()),
            );
            let tcls = &mut builder.db.tile_classes[tcid];
            tcls.bels.insert(bslots::MISR_FC, BelInfo::Bel(bel));
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
            naming.bels.insert(bslots::MISR_FC, bel_naming);
        }
    } else if rd.family == "spartan3e" {
        builder.extract_int_bels_id(
            tcls::CNR_SW_S3E,
            "LL",
            "CNR_SW_S3E",
            &[
                builder.bel_virtual(bslots::MISC_SW),
                builder.bel_virtual(bslots::BANK),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_SE_S3E,
            "LR",
            "CNR_SE_S3E",
            &[
                builder.bel_single(bslots::STARTUP, "STARTUP"),
                builder.bel_single(bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(bslots::ICAP, "ICAP").pin_force_int(
                    "I2",
                    TileWireCoord::new_idx(0, wires::IMUX_DATA[2]),
                    "CNR_DATA_IN2",
                ),
                builder.bel_virtual(bslots::MISC_SE),
                builder.bel_virtual(bslots::BANK),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_S3E,
            "UL",
            "CNR_NW_S3E",
            &[
                builder.bel_single(bslots::PMV, "PMV"),
                builder.bel_virtual(bslots::MISC_NW),
                builder.bel_virtual(bslots::BANK),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_S3E,
            "UR",
            "CNR_NE_S3E",
            &[
                builder.bel_single(bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
                builder.bel_virtual(bslots::MISC_NE),
                builder.bel_virtual(bslots::BANK),
            ],
        );
    } else {
        builder.extract_int_bels_id(
            tcls::CNR_SW_S3A,
            "LL",
            "CNR_SW_S3A",
            &[
                builder.bel_virtual(bslots::MISC_SW),
                builder.bel_virtual(bslots::BANK),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_SE_S3A,
            "LR",
            "CNR_SE_S3A",
            &[
                builder.bel_single(bslots::STARTUP, "STARTUP"),
                builder.bel_single(bslots::CAPTURE, "CAPTURE"),
                builder.bel_single(bslots::ICAP, "ICAP"),
                builder.bel_single(bslots::SPI_ACCESS, "SPI_ACCESS"),
                builder.bel_virtual(bslots::MISC_SE),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NW_S3A,
            "UL",
            "CNR_NW_S3A",
            &[
                builder.bel_single(bslots::PMV, "PMV"),
                builder.bel_single(bslots::DNA_PORT, "DNA_PORT"),
                builder.bel_virtual(bslots::MISC_NW),
                builder.bel_virtual(bslots::BANK),
            ],
        );
        builder.extract_int_bels_id(
            tcls::CNR_NE_S3A,
            "UR",
            "CNR_NE_S3A",
            &[
                builder.bel_single(bslots::BSCAN, "BSCAN"),
                builder
                    .bel_virtual(bslots::RANDOR_OUT)
                    .extra_int_out("O", &["UR_CARRY_IN"]),
                builder.bel_virtual(bslots::MISC_NE),
            ],
        );
    }
    for tcid in [
        tcls::CNR_SE_S3,
        tcls::CNR_SE_FC,
        tcls::CNR_SE_S3E,
        tcls::CNR_SE_S3A,
    ] {
        if let Some(bel) = builder.db.tile_classes[tcid].bels.get_mut(bslots::ICAP) {
            let BelInfo::Bel(bel) = bel else {
                unreachable!()
            };
            for pin in [bcls::ICAP::CE, bcls::ICAP::WRITE] {
                let BelInput::Fixed(wire) = bel.inputs[pin] else {
                    unreachable!()
                };
                bel.inputs[pin] = BelInput::Fixed(!wire);
            }
        }
        if let Some(bel) = builder.db.tile_classes[tcid]
            .bels
            .get_mut(bslots::SPI_ACCESS)
        {
            let BelInfo::Bel(bel) = bel else {
                unreachable!()
            };
            let pin = bcls::SPI_ACCESS::CSB;
            let BelInput::Fixed(wire) = bel.inputs[pin] else {
                unreachable!()
            };
            bel.inputs[pin] = BelInput::Fixed(!wire);
        }
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
                Some((tcid, bslots::LLV, naming)),
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
                Some((tcid, bslots::LLH, "LLH")),
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
                let bels = [
                    builder.bel_indexed(bslots::BUFGMUX[0], "BUFGMUX", 0),
                    builder.bel_indexed(bslots::BUFGMUX[1], "BUFGMUX", 1),
                    builder.bel_indexed(bslots::BUFGMUX[2], "BUFGMUX", 2),
                    builder.bel_indexed(bslots::BUFGMUX[3], "BUFGMUX", 3),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let mut xt = builder
                    .xtile_id(tcls::CLK_S_S3, "CLK_S_S3", xy)
                    .num_cells(2)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::GCLK_S[..])
                    .skip_muxes(&wires::LH[..]);
                for i in 0..2 {
                    xt = xt.ref_int(xy_l.delta(i, 0), i as usize);
                }
                xt.bels(bels).extract();
            } else if rd.family == "fpgacore" {
                let bels = [
                    builder
                        .bel_indexed(bslots::BUFGMUX[0], "BUFG", 0)
                        .pin_rename("I", "I0"),
                    builder
                        .bel_indexed(bslots::BUFGMUX[1], "BUFG", 1)
                        .pin_rename("I", "I0"),
                    builder
                        .bel_indexed(bslots::BUFGMUX[2], "BUFG", 2)
                        .pin_rename("I", "I0"),
                    builder
                        .bel_indexed(bslots::BUFGMUX[3], "BUFG", 3)
                        .pin_rename("I", "I0"),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let mut xt = builder
                    .xtile_id(tcls::CLK_S_FC, "CLK_S_FC", xy)
                    .num_cells(2)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::GCLK_S[..])
                    .skip_muxes(&wires::LH[..]);
                for i in 0..2 {
                    xt = xt.ref_int(xy_l.delta(i, 0), i as usize);
                }
                xt.bels(bels).extract();
            } else {
                let (tcid, naming) = if rd.family == "spartan3e" {
                    (tcls::CLK_S_S3E, "CLK_S_S3E")
                } else {
                    (tcls::CLK_S_S3A, "CLK_S_S3A")
                };
                let bels = [
                    builder.bel_xy(bslots::BUFGMUX[0], "BUFGMUX", 1, 1),
                    builder.bel_xy(bslots::BUFGMUX[1], "BUFGMUX", 1, 0),
                    builder.bel_xy(bslots::BUFGMUX[2], "BUFGMUX", 0, 1),
                    builder.bel_xy(bslots::BUFGMUX[3], "BUFGMUX", 0, 0),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let mut xt = builder
                    .xtile_id(tcid, naming, xy)
                    .num_cells(8)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::GCLK_S[..])
                    .skip_muxes(&wires::LH[..])
                    .bels(bels);
                for i in 0..8 {
                    if i < 4 {
                        xt = xt.ref_int(xy_l.delta(-3 + i, 0), i as usize);
                    } else {
                        xt = xt.ref_int(xy_l.delta(-3 + i + 1, 0), i as usize);
                    }
                }
                xt.extract();

                let pips = builder.pips.get_mut(&(tcid, bslots::CLK_INT)).unwrap();
                let mut new_pips = vec![];
                for &(wt, wf) in pips.pips.keys() {
                    if !wires::OUT_CLKPAD.contains(wf.wire) {
                        continue;
                    }
                    let Some(idx) = wires::IMUX_BUFG_CLK.index_of(wt.wire) else {
                        continue;
                    };
                    new_pips.push((
                        TileWireCoord::new_idx(
                            if wf.cell.to_idx() < 4 { 3 } else { 4 },
                            wires::DCM_CLKPAD[if wf.cell.to_idx() < 4 { idx ^ 3 } else { idx }],
                        ),
                        wf,
                    ));
                }
                for pip in new_pips {
                    pips.pips.insert(pip, PipMode::PermaBuf);
                }
            }
        }
    }
    for tkn in ["CLKT", "CLKT_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, 0);
            if rd.family == "spartan3" {
                let bels = [
                    builder.bel_indexed(bslots::BUFGMUX[0], "BUFGMUX", 4),
                    builder.bel_indexed(bslots::BUFGMUX[1], "BUFGMUX", 5),
                    builder.bel_indexed(bslots::BUFGMUX[2], "BUFGMUX", 6),
                    builder.bel_indexed(bslots::BUFGMUX[3], "BUFGMUX", 7),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let mut xt = builder
                    .xtile_id(tcls::CLK_N_S3, "CLK_N_S3", xy)
                    .num_cells(2)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::GCLK_N[..])
                    .skip_muxes(&wires::LH[..]);
                for i in 0..2 {
                    xt = xt.ref_int(xy_l.delta(i, 0), i as usize);
                }
                xt.bels(bels).extract();
            } else if rd.family == "fpgacore" {
                let bels = [
                    builder
                        .bel_indexed(bslots::BUFGMUX[0], "BUFG", 4)
                        .pin_rename("I", "I0"),
                    builder
                        .bel_indexed(bslots::BUFGMUX[1], "BUFG", 5)
                        .pin_rename("I", "I0"),
                    builder
                        .bel_indexed(bslots::BUFGMUX[2], "BUFG", 6)
                        .pin_rename("I", "I0"),
                    builder
                        .bel_indexed(bslots::BUFGMUX[3], "BUFG", 7)
                        .pin_rename("I", "I0"),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let mut xt = builder
                    .xtile_id(tcls::CLK_N_FC, "CLK_N_FC", xy)
                    .num_cells(2)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::GCLK_N[..])
                    .skip_muxes(&wires::LH[..]);
                for i in 0..2 {
                    xt = xt.ref_int(xy_l.delta(i, 0), i as usize);
                }
                xt.bels(bels).extract();
            } else {
                let (tcid, naming) = if rd.family == "spartan3e" {
                    (tcls::CLK_N_S3E, "CLK_N_S3E")
                } else {
                    (tcls::CLK_N_S3A, "CLK_N_S3A")
                };
                let bels = [
                    builder.bel_xy(bslots::BUFGMUX[0], "BUFGMUX", 1, 1),
                    builder.bel_xy(bslots::BUFGMUX[1], "BUFGMUX", 1, 0),
                    builder.bel_xy(bslots::BUFGMUX[2], "BUFGMUX", 0, 1),
                    builder.bel_xy(bslots::BUFGMUX[3], "BUFGMUX", 0, 0),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let mut xt = builder
                    .xtile_id(tcid, naming, xy)
                    .num_cells(8)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::GCLK_N[..])
                    .skip_muxes(&wires::LH[..])
                    .bels(bels);
                for i in 0..8 {
                    if i < 4 {
                        xt = xt.ref_int(xy_l.delta(-3 + i, 0), i as usize);
                    } else {
                        xt = xt.ref_int(xy_l.delta(-3 + i + 1, 0), i as usize);
                    }
                }
                xt.extract();

                if tcid == tcls::CLK_N_S3E {
                    let pips = builder.pips.get_mut(&(tcid, bslots::CLK_INT)).unwrap();
                    let mut new_pips = vec![];
                    for &(wt, wf) in pips.pips.keys() {
                        if !wires::OUT_CLKPAD.contains(wf.wire) {
                            continue;
                        }
                        let Some(idx) = wires::IMUX_BUFG_CLK.index_of(wt.wire) else {
                            continue;
                        };
                        new_pips.push((
                            TileWireCoord::new_idx(
                                if wf.cell.to_idx() < 4 { 3 } else { 4 },
                                wires::DCM_CLKPAD[if wf.cell.to_idx() < 4 { idx } else { idx ^ 2 }],
                            ),
                            wf,
                        ));
                    }
                    for pip in new_pips {
                        pips.pips.insert(pip, PipMode::PermaBuf);
                    }
                }
            }
        }
    }

    for (tkn, tcid, kind) in [
        ("BBTERM", tcls::DCMCONN_S, "DCMCONN_S"),
        ("BTTERM", tcls::DCMCONN_N, "DCMCONN_N"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = xy.delta(0, if kind == "DCMCONN_S" { 1 } else { -1 });
            if rd.tile_kinds.key(rd.tiles[&int_xy].kind) == "BRAM_IOIS_NODCM" {
                continue;
            }
            builder
                .xtile_id(tcid, kind, xy)
                .ref_int(int_xy, 0)
                .switchbox(bslots::DCMCONN)
                .optin_muxes(wires::DCM_BUS.as_slice())
                .extract();
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
                let bels = [
                    builder.bel_xy(bslots::BUFGMUX[0], "BUFGMUX", 0, 0),
                    builder.bel_xy(bslots::BUFGMUX[1], "BUFGMUX", 0, 1),
                    builder.bel_xy(bslots::BUFGMUX[2], "BUFGMUX", 0, 2),
                    builder.bel_xy(bslots::BUFGMUX[3], "BUFGMUX", 0, 3),
                    builder.bel_xy(bslots::BUFGMUX[4], "BUFGMUX", 0, 4),
                    builder.bel_xy(bslots::BUFGMUX[5], "BUFGMUX", 0, 5),
                    builder.bel_xy(bslots::BUFGMUX[6], "BUFGMUX", 0, 6),
                    builder.bel_xy(bslots::BUFGMUX[7], "BUFGMUX", 0, 7),
                    builder
                        .bel_xy(bslots::PCILOGICSE, "PCILOGIC", 0, 0)
                        .pin_name_only("PCI_CE", 1)
                        .pin_name_only("IRDY", 1)
                        .pin_name_only("TRDY", 1),
                    builder.bel_virtual(bslots::GLOBALSIG_BUFG[0]),
                ];
                let tcid;
                let naming;
                let buf_xy;
                if rd.family == "spartan3e" {
                    naming = format!("{nn}_S3E");
                    tcid = tcid_s3e;
                    buf_xy = vec![];
                } else {
                    naming = format!("{nn}_S3A");
                    tcid = tcid_s3a;
                    buf_xy = vec![xy_o];
                }
                let mut xt = builder
                    .xtile_id(tcid, &naming, xy)
                    .num_cells(8)
                    .bels(bels)
                    .extract_muxes(bslots::CLK_INT)
                    .skip_muxes(&wires::LV[..])
                    .skip_muxes(&wires::IMUX_DATA[..]);
                for xy in buf_xy {
                    xt = xt.raw_tile(xy);
                }
                for i in 0..4 {
                    xt = xt.ref_int(int_s_xy.delta(0, -3 + i), i as usize);
                }
                for i in 0..4 {
                    xt = xt.ref_int(int_n_xy.delta(0, i), i as usize + 4);
                }
                xt.extract();

                let tcls = &mut builder.db.tile_classes[tcid];
                for i in 0..8 {
                    let bel = &mut tcls.bels[bslots::BUFGMUX[i]];
                    let BelInfo::Bel(bel) = bel else {
                        unreachable!()
                    };
                    bel.outputs[bcls::BUFGMUX::O]
                        .insert(TileWireCoord::new_idx(4, wires::GCLK_WE[i]));
                }

                let pips = builder.pips.get_mut(&(tcid, bslots::CLK_INT)).unwrap();
                let mut new_pips = vec![];
                for &(wt, wf) in pips.pips.keys() {
                    if !wires::OUT_CLKPAD.contains(wf.wire) {
                        continue;
                    }
                    let Some(idx) = wires::IMUX_BUFG_CLK.index_of(wt.wire) else {
                        continue;
                    };
                    new_pips.push((
                        TileWireCoord {
                            cell: wt.cell,
                            wire: wires::DCM_CLKPAD[idx],
                        },
                        wf,
                    ));
                }
                for pip in new_pips {
                    pips.pips.insert(pip, PipMode::PermaBuf);
                }

                let ntcls = builder.ndb.tile_class_namings.get_mut(&naming).unwrap().1;
                ntcls.wires.insert(
                    TileWireCoord::new_idx(4, wires::PULLUP),
                    WireNaming {
                        name: format!("{tkn}_VCC_WIRE"),
                        alt_name: None,
                        alt_pips_to: Default::default(),
                        alt_pips_from: Default::default(),
                    },
                );
                for i in 0..8 {
                    let bel = &mut ntcls.bels[bslots::BUFGMUX[i]];
                    let pin = bel.pins.get_mut("O").unwrap();
                    if pin.pips.len() == 2 {
                        pin.pips.pop();
                    }
                    ntcls.wires.insert(
                        TileWireCoord::new_idx(4, wires::GCLK_WE[i]),
                        WireNaming {
                            name: pin.name_far.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                }
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
                    &[builder.bel_xy(bslots::BRAM, "RAMB16", 0, 0)],
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
                .bel_xy(bslots::DSP, "DSP48A", 0, 0)
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
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                let mut x = builder
                    .xtile_id(tcls::DSP, naming, xy)
                    .num_cells(int_xy.len())
                    .extract_intfs(bslots::DSP_TESTMUX, None, false)
                    .bel(bel_dsp.clone())
                    .force_test_mux_in();
                for (i, &xy) in int_xy.iter().enumerate() {
                    x = x.ref_int(xy, i);
                }
                x.extract();
            }
        }
        let mut wires_c = BTreeSet::new();
        let tcls = &builder.db.tile_classes[tcls::DSP];
        let BelInfo::Bel(ref bel) = tcls.bels[bslots::DSP] else {
            unreachable!()
        };
        for pin in bcls::DSP::C {
            let BelInput::Fixed(w) = bel.inputs[pin] else {
                unreachable!()
            };
            wires_c.insert(w.tw);
        }

        let tcls = &mut builder.db.tile_classes[tcls::DSP];
        let BelInfo::TestMux(ref mut tm) = tcls.bels[bslots::DSP_TESTMUX] else {
            unreachable!()
        };
        for tout in tm.wires.values_mut() {
            let mut test_src = vec![None, None];
            let num = tout.test_src.iter().flatten().count();
            for &src in tout.test_src.iter().flatten() {
                let group = if num == 2 && !wires_c.contains(&src.tw) {
                    1
                } else {
                    0
                };
                assert_eq!(test_src[group], None);
                test_src[group] = Some(src);
            }
            tout.test_src = test_src;
        }
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
            let mut bel_mult = builder.bel_xy(bslots::MULT, "MULT18X18", 0, 0);
            if rd.family == "spartan3" {
                bel_mult = bel_mult.pin_rename("CE", "CEP").pin_rename("RST", "RSTP");
            }
            let buf_cnt = if naming == "BRAM_S3A_TOP" { 0 } else { 1 };
            for i in 0..18 {
                bel_mult = bel_mult.pin_name_only(&format!("BCIN{i}"), 0);
                bel_mult = bel_mult.pin_name_only(&format!("BCOUT{i}"), buf_cnt);
            }
            let mut bel_bram = builder
                .bel_xy(bslots::BRAM, "RAMB16", 0, 0)
                .pin_rename("SSRA", "RSTA")
                .pin_rename("SSRB", "RSTB");
            if rd.family != "spartan3a" {
                bel_bram = bel_bram.pin_rename("WEA", "WEA0").pin_rename("WEB", "WEB0");
            }
            let bels_bram = [bel_bram, bel_mult];
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                builder.extract_xtile_bels_id(tcid, xy, &[], &int_xy, naming, &bels_bram, false);
            }
        }

        if rd.family == "spartan3a" {
            let tcls = &mut builder.db.tile_classes[tcls::BRAM_S3A];
            let BelInfo::Bel(bel_bram) = &tcls.bels[bslots::BRAM] else {
                unreachable!()
            };
            let wires_doa = Vec::from_iter(
                bcls::BRAM::DOA[0..16]
                    .iter()
                    .chain(&bcls::BRAM::DOPA[0..2])
                    .map(|&pin| {
                        let wires = &bel_bram.outputs[pin];
                        assert_eq!(wires.len(), 1);
                        wires.iter().next().copied().unwrap()
                    }),
            );
            let wires_dob = Vec::from_iter(
                bcls::BRAM::DOB[0..16]
                    .iter()
                    .chain(&bcls::BRAM::DOPB[0..2])
                    .map(|&pin| {
                        let wires = &bel_bram.outputs[pin];
                        assert_eq!(wires.len(), 1);
                        wires.iter().next().copied().unwrap()
                    }),
            );
            let BelInfo::Bel(bel_mult) = &mut tcls.bels[bslots::MULT] else {
                unreachable!()
            };
            let wires_a = Vec::from_iter(bcls::MULT::A.into_iter().map(|pin| {
                let BelInput::Fixed(wire) = bel_mult.inputs[pin] else {
                    unreachable!()
                };
                wire.tw
            }));
            let wires_b = Vec::from_iter(bcls::MULT::B.into_iter().map(|pin| {
                let BelInput::Fixed(wire) = bel_mult.inputs[pin] else {
                    unreachable!()
                };
                wire.tw
            }));
            for i in 0..18 {
                bel_mult.inputs[bcls::MULT::A[i]] =
                    BelInput::Fixed(TileWireCoord::new_idx(0, wires::IMUX_MULT_A[i]).pos());
                bel_mult.inputs[bcls::MULT::B[i]] =
                    BelInput::Fixed(TileWireCoord::new_idx(0, wires::IMUX_MULT_B[i]).pos());
            }
            let mut sb = SwitchBox::default();
            for i in 0..18 {
                sb.items.push(SwitchBoxItem::Mux(Mux {
                    dst: TileWireCoord::new_idx(0, wires::IMUX_MULT_A[i]),
                    bits: Default::default(),
                    src: [wires_a[i], wires_doa[i]]
                        .into_iter()
                        .map(|v| (v.pos(), Default::default()))
                        .collect(),
                    bits_off: None,
                }));
                sb.items.push(SwitchBoxItem::Mux(Mux {
                    dst: TileWireCoord::new_idx(0, wires::IMUX_MULT_B[i]),
                    bits: Default::default(),
                    src: [wires_b[i], wires_dob[i]]
                        .into_iter()
                        .map(|v| (v.pos(), Default::default()))
                        .collect(),
                    bits_off: None,
                }));
            }
            sb.items.sort();
            tcls.bels.insert(bslots::MULT_INT, BelInfo::SwitchBox(sb));

            for naming in ["BRAM_S3A", "BRAM_S3A_BOT", "BRAM_S3A_TOP"] {
                let naming = builder.ndb.tile_class_namings.get_mut(naming).unwrap().1;
                let bel_mult = &mut naming.bels[bslots::MULT];
                for i in 0..18 {
                    let pin = bel_mult.pins.get_mut(&format!("A{i}")).unwrap();
                    pin.pips.clear();
                    naming.wires.insert(
                        TileWireCoord::new_idx(0, wires::IMUX_MULT_A[i]),
                        WireNaming {
                            name: pin.name.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    naming.wires.insert(
                        wires_a[i],
                        WireNaming {
                            name: pin.name_far.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    pin.name_far = pin.name.clone();

                    let pin = bel_mult.pins.get_mut(&format!("B{i}")).unwrap();
                    pin.pips.clear();
                    naming.wires.insert(
                        TileWireCoord::new_idx(0, wires::IMUX_MULT_B[i]),
                        WireNaming {
                            name: pin.name.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    naming.wires.insert(
                        wires_b[i],
                        WireNaming {
                            name: pin.name_far.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    pin.name_far = pin.name.clone();
                }
                let bel_bram = &naming.bels[bslots::BRAM];
                for i in 0..18 {
                    let pin = if i < 16 {
                        format!("DOA{i}")
                    } else {
                        format!("DOPA{ii}", ii = i - 16)
                    };
                    let pin = &bel_bram.pins[&pin];
                    naming.wires.insert(
                        wires_doa[i],
                        WireNaming {
                            name: pin.name.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                    let pin = if i < 16 {
                        format!("DOB{i}")
                    } else {
                        format!("DOPB{ii}", ii = i - 16)
                    };
                    let pin = &bel_bram.pins[&pin];
                    naming.wires.insert(
                        wires_dob[i],
                        WireNaming {
                            name: pin.name.clone(),
                            alt_name: None,
                            alt_pips_to: Default::default(),
                            alt_pips_from: Default::default(),
                        },
                    );
                }
            }
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC_50A") {
        builder
            .xtile_id(tcls::CLKC_50A, "CLKC_50A", xy)
            .num_cells(2)
            .switchbox(bslots::HROW)
            .optin_muxes(&wires::GCLK_QUAD[..])
            .extract();
    }

    for &xy in rd.tiles_by_kind_name("GCLKVM") {
        builder
            .xtile_id(tcls::CLKQC_S3, "GCLKVM_S3", xy)
            .switchbox(bslots::HROW)
            .optin_muxes(&wires::GCLK_QUAD[..])
            .num_cells(2)
            .extract();
    }
    if let Some(pips) = builder.pips.get_mut(&(tcls::CLKQC_S3, bslots::HROW)) {
        for mode in pips.pips.values_mut() {
            *mode = PipMode::Buf;
        }
    }

    for tkn in ["GCLKVML", "GCLKVMR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            builder
                .xtile_id(tcls::CLKQC_S3E, tkn, xy)
                .num_cells(2)
                .switchbox(bslots::HROW)
                .optin_muxes(&wires::GCLK_QUAD[..])
                .extract();
        }
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
            {
                let bel = builder.bel_virtual(bslots::GLOBALSIG_HCLK);
                builder
                    .xtile_id(tcls::HCLK, "HCLK", xy)
                    .num_cells(2)
                    .ref_int(int_s_xy, 0)
                    .ref_int(int_n_xy, 1)
                    .switchbox(bslots::HCLK)
                    .optin_muxes(&wires::GCLK[..])
                    .bel(bel)
                    .extract();
            };
        }
    }
    let pips = builder.pips.get_mut(&(tcls::HCLK, bslots::HCLK)).unwrap();
    for mode in pips.pips.values_mut() {
        *mode = PipMode::Buf;
    }

    let naming = builder
        .ndb
        .tile_class_namings
        .get("HCLK")
        .unwrap()
        .1
        .clone();
    let mut naming_dsp = naming.clone();
    naming_dsp.bels[bslots::GLOBALSIG_HCLK]
        .tiles
        .push(RawTileId::from_idx(1));
    builder
        .ndb
        .tile_class_namings
        .insert("HCLK_DSP".into(), naming_dsp);

    let mut naming_bram = naming.clone();
    for i in 0..8 {
        naming_bram
            .wires
            .get_mut(&TileWireCoord::new_idx(1, wires::GCLK_QUAD[i]))
            .unwrap()
            .name = format!("BRAMSITE2_GCLKH_GCLK{i}");
        naming_bram
            .wires
            .get_mut(&TileWireCoord::new_idx(0, wires::GCLK[i]))
            .unwrap()
            .name = format!("BRAMSITE2_GCLKH_GCLK_DN{i}");
        naming_bram
            .wires
            .get_mut(&TileWireCoord::new_idx(1, wires::GCLK[i]))
            .unwrap()
            .name = format!("BRAMSITE2_GCLKH_GCLK_UP{i}");
    }
    builder
        .ndb
        .tile_class_namings
        .insert("HCLK_BRAM".into(), naming_bram.clone());
    let mut naming_bram_s = naming_bram.clone();
    let mut naming_bram_uni_s = naming_bram.clone();
    let mut naming_bram_n = naming_bram;
    for i in 0..8 {
        naming_bram_n
            .wires
            .remove(&TileWireCoord::new_idx(0, wires::GCLK[i]));
        naming_bram_s
            .wires
            .remove(&TileWireCoord::new_idx(1, wires::GCLK[i]));
        let wn = naming_bram_uni_s
            .wires
            .remove(&TileWireCoord::new_idx(0, wires::GCLK[i]))
            .unwrap();
        naming_bram_uni_s
            .wires
            .insert(TileWireCoord::new_idx(1, wires::GCLK[i]), wn);
    }
    builder
        .ndb
        .tile_class_namings
        .insert("HCLK_BRAM_S".into(), naming_bram_s);
    builder
        .ndb
        .tile_class_namings
        .insert("HCLK_BRAM_UNI_S".into(), naming_bram_uni_s);
    builder
        .ndb
        .tile_class_namings
        .insert("HCLK_BRAM_N".into(), naming_bram_n);

    let mut naming_0 = naming;
    naming_0.wires.clear();
    builder
        .ndb
        .tile_class_namings
        .insert("HCLK_0".into(), naming_0);

    let mut pips = builder.pips[&(tcls::HCLK, bslots::HCLK)].clone();
    pips.pips.retain(|&(wt, _), _| wt.cell.to_idx() == 1);
    builder.pips.insert((tcls::HCLK_UNI, bslots::HCLK), pips);

    builder.insert_tcls_bel(
        tcls::HCLK_UNI,
        bslots::GLOBALSIG_HCLK,
        BelInfo::Bel(Default::default()),
    );

    let kind = if rd.family == "spartan3" {
        ChipKind::Spartan3
    } else if rd.family == "fpgacore" {
        ChipKind::FpgaCore
    } else if rd.family == "spartan3e" {
        ChipKind::Spartan3E
    } else {
        ChipKind::Spartan3A
    };
    for itd in get_iob_tiles(kind) {
        let tcid = itd.tcid;
        let mut naming = TileClassNaming::default();
        if kind == ChipKind::FpgaCore {
            for bslot in bslots::IBUF.into_iter().chain(bslots::OBUF) {
                builder.insert_tcls_bel(tcid, bslot, BelInfo::Bel(Default::default()));
                naming.bels.insert(bslot, Default::default());
            }
        } else {
            for i in 0..itd.iobs.len() {
                builder.insert_tcls_bel(tcid, bslots::IOB[i], BelInfo::Bel(Default::default()));
                naming.bels.insert(bslots::IOB[i], Default::default());
            }
        }
        builder
            .ndb
            .tile_class_namings
            .insert(builder.db.tile_classes.key(tcid).into(), naming);
    }

    builder.build()
}
