use prjcombine_entity::EntityId;
use prjcombine_int::db::{Dir, IntDb, NodeExtPipNaming, NodeRawTileId, NodeTileId, WireKind};
use prjcombine_rawdump::{Coord, Part};

use prjcombine_rdintb::IntBuilder;

pub fn make_int_db(rd: &Part) -> IntDb {
    let mut builder = IntBuilder::new("spartan3", rd);

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
            WireKind::ClkOut(i),
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

    let lh: Vec<_> = (0..24)
        .map(|i| {
            builder.wire(
                format!("LH.{i}"),
                WireKind::MultiBranch(Dir::W),
                &[format!("LH{i}")],
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

    // The set/reset inputs.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.SR{i}"),
            &[
                format!("SR{i}"),
                format!("IOIS_SR{i}"),
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
        builder.mux_out(format!("IMUX.IOCLK{i}"), &[format!("IOIS_CLK{i}")]);
    }

    // The clock enables.
    for i in 0..4 {
        builder.mux_out(
            format!("IMUX.CE{i}"),
            &[
                format!("CE_B{i}"),
                format!("IOIS_CE_B{i}"),
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

    let bels_int = [builder.bel_xy("RLL", "RLL", 0, 0)];
    let bels_int_dcm = [
        builder.bel_xy("RLL", "RLL", 0, 0),
        builder
            .bel_virtual("PTE2OMUX0")
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
            .bel_virtual("PTE2OMUX1")
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
            .bel_virtual("PTE2OMUX2")
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
            .bel_virtual("PTE2OMUX3")
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

    builder.extract_node("CENTER", "INT.CLB", "INT.CLB", &bels_int);
    builder.extract_node("CENTER_SMALL", "INT.CLB", "INT.CLB", &bels_int);
    builder.extract_node("CENTER_SMALL_BRK", "INT.CLB", "INT.CLB.BRK", &bels_int);
    if rd.family.starts_with("spartan3a") {
        builder.extract_node("LIOIS", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node(
            "LIOIS_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node("LIOIS_PCI", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node(
            "LIOIS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            "LIOIS_CLK_PCI_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node("LIBUFS", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node("LIBUFS_PCI", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node(
            "LIBUFS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node("RIOIS", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node("RIOIS_PCI", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node(
            "RIOIS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node("RIBUFS", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node(
            "RIBUFS_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node("RIBUFS_PCI", "INT.IOI.S3A.LR", "INT.IOI.S3A.LR", &bels_int);
        builder.extract_node(
            "RIBUFS_CLK_PCI",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR",
            &bels_int,
        );
        builder.extract_node(
            "RIBUFS_CLK_PCI_BRK",
            "INT.IOI.S3A.LR",
            "INT.IOI.S3A.LR.BRK",
            &bels_int,
        );
        builder.extract_node("BIOIS", "INT.IOI.S3A.TB", "INT.IOI.S3A.TB", &bels_int);
        builder.extract_node("BIOIB", "INT.IOI.S3A.TB", "INT.IOI.S3A.TB", &bels_int);
        builder.extract_node("TIOIS", "INT.IOI.S3A.TB", "INT.IOI.S3A.TB", &bels_int);
        builder.extract_node("TIOIB", "INT.IOI.S3A.TB", "INT.IOI.S3A.TB", &bels_int);
    } else if rd.family == "spartan3e" {
        builder.extract_node("LIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("LIOIS_BRK", "INT.IOI.S3E", "INT.IOI.BRK", &bels_int);
        builder.extract_node("LIOIS_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("LIOIS_CLK_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("LIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("LIBUFS_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("LIBUFS_CLK_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("RIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("RIOIS_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("RIOIS_CLK_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("RIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("RIBUFS_BRK", "INT.IOI.S3E", "INT.IOI.BRK", &bels_int);
        builder.extract_node("RIBUFS_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("RIBUFS_CLK_PCI", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("BIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("BIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("TIOIS", "INT.IOI.S3E", "INT.IOI", &bels_int);
        builder.extract_node("TIBUFS", "INT.IOI.S3E", "INT.IOI", &bels_int);
    } else {
        // NOTE: could be unified by pulling extra muxes from CLB
        builder.extract_node("LIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
        builder.extract_node("RIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
        builder.extract_node("BIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
        builder.extract_node("TIOIS", "INT.IOI.S3", "INT.IOI", &bels_int);
    }
    // NOTE:
    // - S3/S3E/S3A could be unified by pulling some extra muxes from CLB
    // - S3A/S3ADSP adds VCC input to B[XY] and splits B[XY] to two nodes
    if rd.family == "spartan3adsp" {
        builder.extract_node(
            "BRAM0_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            "BRAM0_SMALL_BOT",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            "BRAM1_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            "BRAM2_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            "BRAM3_SMALL",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            "BRAM3_SMALL_TOP",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP",
            &bels_int,
        );
        builder.extract_node(
            "BRAM3_SMALL_BRK",
            "INT.BRAM.S3ADSP",
            "INT.BRAM.S3ADSP.BRK",
            &bels_int,
        );
        builder.extract_node("MACC0_SMALL", "INT.BRAM.S3ADSP", "INT.MACC", &bels_int);
        builder.extract_node("MACC0_SMALL_BOT", "INT.BRAM.S3ADSP", "INT.MACC", &bels_int);
        builder.extract_node("MACC1_SMALL", "INT.BRAM.S3ADSP", "INT.MACC", &bels_int);
        builder.extract_node("MACC2_SMALL", "INT.BRAM.S3ADSP", "INT.MACC", &bels_int);
        builder.extract_node("MACC3_SMALL", "INT.BRAM.S3ADSP", "INT.MACC", &bels_int);
        builder.extract_node("MACC3_SMALL_TOP", "INT.BRAM.S3ADSP", "INT.MACC", &bels_int);
        builder.extract_node(
            "MACC3_SMALL_BRK",
            "INT.BRAM.S3ADSP",
            "INT.MACC.BRK",
            &bels_int,
        );
    } else if rd.family == "spartan3a" {
        builder.extract_node("BRAM0_SMALL", "INT.BRAM.S3A", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM0_SMALL_BOT", "INT.BRAM.S3A", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM1_SMALL", "INT.BRAM.S3A", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM2_SMALL", "INT.BRAM.S3A", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3_SMALL", "INT.BRAM.S3A", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3_SMALL_TOP", "INT.BRAM.S3A", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3_SMALL_BRK", "INT.BRAM.S3A", "INT.BRAM.BRK", &bels_int);
    } else if rd.family == "spartan3e" {
        builder.extract_node("BRAM0_SMALL", "INT.BRAM.S3E", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM1_SMALL", "INT.BRAM.S3E", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM2_SMALL", "INT.BRAM.S3E", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3_SMALL", "INT.BRAM.S3E", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3_SMALL_BRK", "INT.BRAM.S3E", "INT.BRAM.BRK", &bels_int);
    } else {
        builder.extract_node("BRAM0", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM1", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM2", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM0_SMALL", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM1_SMALL", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM2_SMALL", "INT.BRAM.S3", "INT.BRAM", &bels_int);
        builder.extract_node("BRAM3_SMALL", "INT.BRAM.S3", "INT.BRAM", &bels_int);
    }
    builder.extract_node("BRAM_IOIS", "INT.DCM", "INT.DCM.S3", &bels_int_dcm);
    builder.extract_node(
        "BRAM_IOIS_NODCM",
        "INT.DCM.S3.DUMMY",
        "INT.DCM.S3.DUMMY",
        &bels_int,
    );
    builder.extract_node(
        "DCMAUX_BL_CENTER",
        "INT.DCM.S3E.DUMMY",
        "INT.DCM.S3E.DUMMY",
        &bels_int_dcm,
    );
    builder.extract_node(
        "DCMAUX_TL_CENTER",
        "INT.DCM.S3E.DUMMY",
        "INT.DCM.S3E.DUMMY",
        &bels_int_dcm,
    );
    builder.extract_node("DCM_BL_CENTER", "INT.DCM", "INT.DCM.S3E", &bels_int_dcm);
    builder.extract_node("DCM_TL_CENTER", "INT.DCM", "INT.DCM.S3E", &bels_int_dcm);
    builder.extract_node("DCM_BR_CENTER", "INT.DCM", "INT.DCM.S3E", &bels_int_dcm);
    builder.extract_node("DCM_TR_CENTER", "INT.DCM", "INT.DCM.S3E", &bels_int_dcm);
    builder.extract_node("DCM_H_BL_CENTER", "INT.DCM", "INT.DCM.S3E.H", &bels_int_dcm);
    builder.extract_node("DCM_H_TL_CENTER", "INT.DCM", "INT.DCM.S3E.H", &bels_int_dcm);
    builder.extract_node("DCM_H_BR_CENTER", "INT.DCM", "INT.DCM.S3E.H", &bels_int_dcm);
    builder.extract_node("DCM_H_TR_CENTER", "INT.DCM", "INT.DCM.S3E.H", &bels_int_dcm);
    builder.extract_node("DCM_BGAP", "INT.DCM", "INT.DCM.S3E.H", &bels_int_dcm);
    builder.extract_node("DCM_SPLY", "INT.DCM", "INT.DCM.S3E.H", &bels_int_dcm);
    builder.extract_node("LL", "INT.CLB", "INT.CNR", &bels_int);
    builder.extract_node("LR", "INT.CLB", "INT.CNR", &bels_int);
    builder.extract_node("UL", "INT.CLB", "INT.CNR", &bels_int);
    builder.extract_node("UR", "INT.CLB", "INT.CNR", &bels_int);

    let slicem_name_only = [
        "FXINA", "FXINB", "F5", "FX", "CIN", "COUT", "SHIFTIN", "SHIFTOUT", "ALTDIG", "DIG",
        "SLICEWE1", "BYOUT", "BYINVOUT",
    ];
    let slicel_name_only = ["FXINA", "FXINB", "F5", "FX", "CIN", "COUT"];
    let bels_clb = [
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
    ];
    builder.extract_node_bels("CENTER", "CLB", "CLB", &bels_clb);
    builder.extract_node_bels("CENTER_SMALL", "CLB", "CLB", &bels_clb);
    builder.extract_node_bels("CENTER_SMALL_BRK", "CLB", "CLB", &bels_clb);

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
                .bel_indexed("IOI0", "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed("IOI2", "IOB", 2)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_node_bels("LIOIS", "IOI.S3", "IOI.S3.L", &bels_ioi);
        builder.extract_node_bels("RIOIS", "IOI.S3", "IOI.S3.R", &bels_ioi);
        builder.extract_node_bels("BIOIS", "IOI.S3", "IOI.S3.B", &bels_ioi);
        builder.extract_node_bels("TIOIS", "IOI.S3", "IOI.S3.T", &bels_ioi);
        for (kind, num) in [
            ("IOBS.S3.B2", 2),
            ("IOBS.S3.T2", 2),
            ("IOBS.S3.L1", 1),
            ("IOBS.S3.R1", 1),
        ] {
            builder.make_marker_bel(kind, kind, kind, num);
        }
    } else if rd.family == "spartan3e" {
        let bels_ioi_tb = [
            builder
                .bel_indexed("IOI0", "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed("IOI2", "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed("IOI0", "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed("IOI2", "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed("IOI0", "IOB", 0)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_wire_force("IBUF", "RIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only)
                .extra_wire_force("IBUF", "RIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
            builder
                .bel_indexed("IOI2", "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        builder.extract_node_bels("LIOIS", "IOI.S3E", "IOI.S3E.L", &bels_ioi_l);
        builder.extract_node_bels("LIOIS_BRK", "IOI.S3E", "IOI.S3E.L", &bels_ioi_l);
        builder.extract_node_bels("LIOIS_PCI", "IOI.S3E", "IOI.S3E.L.PCI.PCI", &bels_ioi_l);
        builder.extract_node_bels("LIOIS_CLK_PCI", "IOI.S3E", "IOI.S3E.L.PCI.PCI", &bels_ioi_l);
        builder.extract_node_bels("LIBUFS", "IOI.S3E", "IOI.S3E.L", &bels_ioi_l);
        builder.extract_node_bels("LIBUFS_PCI", "IOI.S3E", "IOI.S3E.L.PCI", &bels_ioi_l);
        builder.extract_node_bels("LIBUFS_CLK_PCI", "IOI.S3E", "IOI.S3E.L.PCI", &bels_ioi_l);
        builder.extract_node_bels("RIOIS", "IOI.S3E", "IOI.S3E.R", &bels_ioi_r);
        builder.extract_node_bels("RIOIS_PCI", "IOI.S3E", "IOI.S3E.R.PCI.PCI", &bels_ioi_r);
        builder.extract_node_bels("RIOIS_CLK_PCI", "IOI.S3E", "IOI.S3E.R.PCI.PCI", &bels_ioi_r);
        builder.extract_node_bels("RIBUFS", "IOI.S3E", "IOI.S3E.R", &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_BRK", "IOI.S3E", "IOI.S3E.R", &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_PCI", "IOI.S3E", "IOI.S3E.R.PCI", &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_CLK_PCI", "IOI.S3E", "IOI.S3E.R.PCI", &bels_ioi_r);
        builder.extract_node_bels("BIOIS", "IOI.S3E", "IOI.S3E.B", &bels_ioi_tb);
        builder.extract_node_bels("BIBUFS", "IOI.S3E", "IOI.S3E.B", &bels_ioi_tb);
        builder.extract_node_bels("TIOIS", "IOI.S3E", "IOI.S3E.T", &bels_ioi_tb);
        builder.extract_node_bels("TIBUFS", "IOI.S3E", "IOI.S3E.T", &bels_ioi_tb);
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
            builder.make_marker_bel(kind, kind, kind, num);
        }
    } else {
        let bels_ioi_tb = [
            builder
                .bel_indexed("IOI0", "IOB", 0)
                .extra_wire_force("IBUF", "IOIS_IBUF0")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .extra_wire_force("IBUF", "IOIS_IBUF1")
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
            builder
                .bel_indexed("IOI2", "IOB", 2)
                .pin_name_only("PCI_CE", 1)
                .pins_name_only(&ioi_name_only),
        ];
        let bels_ioi_l = [
            builder
                .bel_indexed("IOI0", "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "LIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
        ];
        let bels_ioi_r = [
            builder
                .bel_indexed("IOI0", "IOB", 0)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "RIOIS_IBUF1")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_TRDY_IN"),
            builder
                .bel_indexed("IOI1", "IOB", 1)
                .pins_name_only(&ioi_name_only)
                .pin_name_only("PCI_CE", 1)
                .extra_wire_force("IBUF", "RIOIS_IBUF0")
                .extra_wire_force("PCI_RDY_IN", "IOIS_PCI_IRDY_IN"),
        ];
        let kind_l;
        let kind_r;
        let kind_b;
        let kind_t;
        let naming_l;
        let naming_r;
        let naming_l_pci;
        let naming_r_pci;
        let naming_b;
        let naming_t;
        if rd.family == "spartan3adsp" {
            kind_l = "IOI.S3ADSP.L";
            kind_r = "IOI.S3ADSP.R";
            kind_b = "IOI.S3ADSP.B";
            kind_t = "IOI.S3ADSP.T";
            naming_l = "IOI.S3ADSP.L";
            naming_r = "IOI.S3ADSP.R";
            naming_l_pci = "IOI.S3ADSP.L.PCI";
            naming_r_pci = "IOI.S3ADSP.R.PCI";
            naming_b = "IOI.S3ADSP.B";
            naming_t = "IOI.S3ADSP.T";
        } else {
            kind_l = "IOI.S3A.L";
            kind_r = "IOI.S3A.R";
            kind_b = "IOI.S3A.B";
            kind_t = "IOI.S3A.T";
            naming_l = "IOI.S3A.L";
            naming_r = "IOI.S3A.R";
            naming_l_pci = "IOI.S3A.L.PCI";
            naming_r_pci = "IOI.S3A.R.PCI";
            naming_b = "IOI.S3A.B";
            naming_t = "IOI.S3A.T";
        }
        builder.extract_node_bels("LIOIS", kind_l, naming_l, &bels_ioi_l);
        builder.extract_node_bels("LIOIS_BRK", kind_l, naming_l, &bels_ioi_l);
        builder.extract_node_bels("LIOIS_PCI", kind_l, naming_l_pci, &bels_ioi_l);
        builder.extract_node_bels("LIOIS_CLK_PCI", kind_l, naming_l_pci, &bels_ioi_l);
        builder.extract_node_bels("LIOIS_CLK_PCI_BRK", kind_l, naming_l_pci, &bels_ioi_l);
        builder.extract_node_bels("LIBUFS", kind_l, naming_l, &bels_ioi_l);
        builder.extract_node_bels("LIBUFS_PCI", kind_l, naming_l, &bels_ioi_l);
        builder.extract_node_bels("LIBUFS_CLK_PCI", kind_l, naming_l, &bels_ioi_l);
        builder.extract_node_bels("RIOIS", kind_r, naming_r, &bels_ioi_r);
        builder.extract_node_bels("RIOIS_PCI", kind_r, naming_r_pci, &bels_ioi_r);
        builder.extract_node_bels("RIOIS_CLK_PCI", kind_r, naming_r_pci, &bels_ioi_r);
        builder.extract_node_bels("RIBUFS", kind_r, naming_r, &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_BRK", kind_r, naming_r, &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_PCI", kind_r, naming_r, &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_CLK_PCI", kind_r, naming_r, &bels_ioi_r);
        builder.extract_node_bels("RIBUFS_CLK_PCI_BRK", kind_r, naming_r, &bels_ioi_r);
        builder.extract_node_bels("BIOIS", kind_b, naming_b, &bels_ioi_tb);
        builder.extract_node_bels("BIOIB", kind_b, naming_b, &bels_ioi_tb);
        builder.extract_node_bels("TIOIS", kind_t, naming_t, &bels_ioi_tb);
        builder.extract_node_bels("TIOIB", kind_t, naming_t, &bels_ioi_tb);
        for (kind, num) in [
            ("IOBS.S3A.B2", 2),
            ("IOBS.S3A.T2", 2),
            ("IOBS.S3A.L4", 4),
            ("IOBS.S3A.R4", 4),
        ] {
            builder.make_marker_bel(kind, kind, kind, num);
        }
    }
    let bels_randor_b = [builder
        .bel_xy("RANDOR", "RANDOR", 0, 0)
        .pins_name_only(&["CIN0", "CIN1", "CPREV", "O"])];
    let bels_randor_t = [builder
        .bel_xy("RANDOR", "RANDOR", 0, 0)
        .pins_name_only(&["CIN0", "CIN1"])
        .pin_name_only("CPREV", 1)
        .pin_name_only("O", 1)];
    builder.extract_node_bels("BIOIS", "RANDOR", "RANDOR.B", &bels_randor_b);
    builder.extract_node_bels("BIOIB", "RANDOR", "RANDOR.B", &bels_randor_b);
    builder.extract_node_bels("BIBUFS", "RANDOR", "RANDOR.B", &bels_randor_b);
    builder.extract_node_bels("TIOIS", "RANDOR", "RANDOR.T", &bels_randor_t);
    builder.extract_node_bels("TIOIB", "RANDOR", "RANDOR.T", &bels_randor_t);
    builder.extract_node_bels("TIBUFS", "RANDOR", "RANDOR.T", &bels_randor_t);

    if rd.family == "spartan3" {
        let bels_dcm = [builder.bel_xy("DCM", "DCM", 0, 0)];
        builder.extract_node_bels("BRAM_IOIS", "DCM.S3", "DCM.S3", &bels_dcm);
    } else {
        let bels_dcm = [
            builder.bel_xy("DCM", "DCM", 0, 0),
            builder
                .bel_virtual("DCMCONN.S3E")
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
        builder.extract_node_bels("DCM_BL_CENTER", "DCM.S3E", "DCM.S3E.L", &bels_dcm);
        builder.extract_node_bels("DCM_TL_CENTER", "DCM.S3E", "DCM.S3E.L", &bels_dcm);
        builder.extract_node_bels("DCM_BR_CENTER", "DCM.S3E", "DCM.S3E.R", &bels_dcm);
        builder.extract_node_bels("DCM_TR_CENTER", "DCM.S3E", "DCM.S3E.R", &bels_dcm);
        builder.extract_node_bels("DCM_H_BL_CENTER", "DCM.S3E", "DCM.S3E.H", &bels_dcm);
        builder.extract_node_bels("DCM_H_TL_CENTER", "DCM.S3E", "DCM.S3E.H", &bels_dcm);
        builder.extract_node_bels("DCM_H_BR_CENTER", "DCM.S3E", "DCM.S3E.H", &bels_dcm);
        builder.extract_node_bels("DCM_H_TR_CENTER", "DCM.S3E", "DCM.S3E.H", &bels_dcm);
        builder.extract_node_bels("DCM_BGAP", "DCM.S3E", "DCM.S3E.H", &bels_dcm);
        builder.extract_node_bels("DCM_SPLY", "DCM.S3E", "DCM.S3E.H", &bels_dcm);
    }

    if rd.family == "spartan3" {
        builder.extract_node_bels(
            "LL",
            "DCI",
            "DCI",
            &[
                builder.bel_indexed("DCI0", "DCI", 6),
                builder.bel_indexed("DCI1", "DCI", 5),
                builder.bel_indexed("DCIRESET0", "DCIRESET", 6),
                builder.bel_indexed("DCIRESET1", "DCIRESET", 5),
            ],
        );
        builder.extract_node_bels(
            "LR",
            "DCI",
            "DCI",
            &[
                builder.bel_indexed("DCI0", "DCI", 3),
                builder.bel_indexed("DCI1", "DCI", 4),
                builder.bel_indexed("DCIRESET0", "DCIRESET", 3),
                builder.bel_indexed("DCIRESET1", "DCIRESET", 4),
            ],
        );
        builder.extract_node_bels(
            "UL",
            "DCI",
            "DCI",
            &[
                builder.bel_indexed("DCI0", "DCI", 7),
                builder.bel_indexed("DCI1", "DCI", 0),
                builder.bel_indexed("DCIRESET0", "DCIRESET", 7),
                builder.bel_indexed("DCIRESET1", "DCIRESET", 0),
            ],
        );
        builder.extract_node_bels(
            "UR",
            "DCI.UR",
            "DCI.UR",
            &[
                builder.bel_indexed("DCI0", "DCI", 2),
                builder.bel_indexed("DCI1", "DCI", 1),
                builder.bel_indexed("DCIRESET0", "DCIRESET", 2),
                builder.bel_indexed("DCIRESET1", "DCIRESET", 1),
            ],
        );
    }

    if rd.family == "spartan3" {
        builder.extract_node_bels(
            "LR",
            "LR.S3",
            "LR.S3",
            &[
                builder.bel_single("STARTUP", "STARTUP"),
                builder.bel_single("CAPTURE", "CAPTURE"),
                builder.bel_single("ICAP", "ICAP"),
            ],
        );
    } else if rd.family == "spartan3e" {
        builder.extract_node_bels(
            "LR",
            "LR.S3E",
            "LR.S3E",
            &[
                builder.bel_single("STARTUP", "STARTUP"),
                builder.bel_single("CAPTURE", "CAPTURE"),
                builder.bel_single("ICAP", "ICAP").pin_force_int(
                    "I2",
                    (NodeTileId::from_idx(0), lr_di2.unwrap()),
                    "CNR_DATA_IN2",
                ),
            ],
        );
    } else {
        builder.extract_node_bels(
            "LR",
            "LR.S3A",
            "LR.S3A",
            &[
                builder.bel_single("STARTUP", "STARTUP"),
                builder.bel_single("CAPTURE", "CAPTURE"),
                builder.bel_single("ICAP", "ICAP"),
                builder.bel_single("SPI_ACCESS", "SPI_ACCESS"),
            ],
        );
    }
    builder.extract_node_bels("UL", "PMV", "PMV", &[builder.bel_single("PMV", "PMV")]);
    if rd.family.starts_with("spartan3a") {
        builder.extract_node_bels(
            "UL",
            "DNA_PORT",
            "DNA_PORT",
            &[builder.bel_single("DNA_PORT", "DNA_PORT")],
        );
        builder.extract_node_bels(
            "UR",
            "UR.S3A",
            "UR.S3A",
            &[
                builder.bel_single("BSCAN", "BSCAN"),
                builder
                    .bel_virtual("RANDOR_OUT")
                    .extra_int_out("O", &["UR_CARRY_IN"]),
            ],
        );
    } else {
        builder.extract_node_bels(
            "UR",
            "UR.S3",
            "UR.S3",
            &[
                builder.bel_single("BSCAN", "BSCAN"),
                builder
                    .bel_virtual("RANDOR_OUT")
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
            let int_s_xy = builder.walk_to_int(xy_b, Dir::S).unwrap();
            let int_n_xy = builder.walk_to_int(xy_t, Dir::N).unwrap();
            builder.extract_pass_tile(
                "TERM.BRAM.S",
                Dir::S,
                int_n_xy,
                Some(xy_t),
                None,
                Some("TERM.BRAM.S"),
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
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_fwd_xy = builder.walk_to_int(xy, Dir::S).unwrap();
            let int_bwd_xy = builder.walk_to_int(xy, Dir::N).unwrap();
            builder.extract_pass_tile(
                llv_s,
                Dir::S,
                int_bwd_xy,
                Some(xy),
                None,
                None,
                Some(("LLV", naming)),
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
                int_bwd_xy,
                &[],
            );
        }
    }
    for tkn in ["CLKV_DCM_LL", "CLKV_LL", "CLKT_LL", "CLKB_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let fix_xy = if tkn == "CLKB_LL" { xy.delta(0, 1) } else { xy };
            let int_fwd_xy = builder.walk_to_int(fix_xy, Dir::W).unwrap();
            let int_bwd_xy = builder.walk_to_int(fix_xy, Dir::E).unwrap();
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
                Some(("LLH", "LLH")),
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
                    int_e_xy,
                    &lh,
                );
            }
        }
    }

    for tkn in ["CLKB", "CLKB_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let xy_l = xy.delta(-1, if rd.family == "spartan3" { 0 } else { 1 });
            if rd.family == "spartan3" {
                builder.extract_xnode(
                    "CLKB.S3",
                    xy,
                    &[],
                    &[xy_l],
                    "CLKB.S3",
                    &[
                        builder
                            .bel_indexed("BUFGMUX0", "BUFGMUX", 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI0"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD0"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL0"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR0"])
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_indexed("BUFGMUX1", "BUFGMUX", 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI1"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD1"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL1"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR1"])
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_indexed("BUFGMUX2", "BUFGMUX", 2)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI2"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD2"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL2"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR2"])
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_indexed("BUFGMUX3", "BUFGMUX", 3)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKB_CKI3"])
                            .extra_wire("DCM_PAD", &["CLKB_DLL_CLKPAD3"])
                            .extra_wire("DCM_OUT_L", &["CLKB_DLL_OUTL3"])
                            .extra_wire("DCM_OUT_R", &["CLKB_DLL_OUTR3"])
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual("GLOBALSIG.B"),
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
                    kind,
                    xy,
                    &[],
                    &[xy_l],
                    kind,
                    &[
                        builder
                            .bel_xy("BUFGMUX0", "BUFGMUX", 1, 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI0"])
                            .extra_wire("CKIL", &["CLKB_CKI4"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD7")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD0")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL0",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTL0".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR0",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTR0".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK0"]),
                        builder
                            .bel_xy("BUFGMUX1", "BUFGMUX", 1, 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI1"])
                            .extra_wire("CKIL", &["CLKB_CKI5"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD6")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD1")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL1",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTL1".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR1",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTR1".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK1"]),
                        builder
                            .bel_xy("BUFGMUX2", "BUFGMUX", 0, 1)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI2"])
                            .extra_wire("CKIL", &["CLKB_CKI6"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD5")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD2")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL2",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTL2".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR2",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTR2".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK2"]),
                        builder
                            .bel_xy("BUFGMUX3", "BUFGMUX", 0, 0)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKIR", &["CLKB_CKI3"])
                            .extra_wire("CKIL", &["CLKB_CKI7"])
                            .extra_wire_force("DCM_PAD_L", "CLKB_DLL_CLKPAD4")
                            .extra_wire_force("DCM_PAD_R", "CLKB_DLL_CLKPAD3")
                            .extra_wire_force_pip(
                                "DCM_OUT_L",
                                "CLKB_DLL_OUTL3",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTL3".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKB_DLL_OUTR3",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTR3".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKB_GCLK3"]),
                        builder.bel_virtual("GLOBALSIG.B"),
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
                    "CLKT.S3",
                    xy,
                    &[],
                    &[xy_l],
                    "CLKT.S3",
                    &[
                        builder
                            .bel_indexed("BUFGMUX0", "BUFGMUX", 4)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI0"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD0"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL0"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR0"])
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_indexed("BUFGMUX1", "BUFGMUX", 5)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI1"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD1"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL1"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR1"])
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_indexed("BUFGMUX2", "BUFGMUX", 6)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI2"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD2"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL2"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR2"])
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_indexed("BUFGMUX3", "BUFGMUX", 7)
                            .pins_name_only(&["I0", "I1"])
                            .extra_wire("CKI", &["CLKT_CKI3"])
                            .extra_wire("DCM_PAD", &["CLKT_DLL_CLKPAD3"])
                            .extra_wire("DCM_OUT_L", &["CLKT_DLL_OUTL3"])
                            .extra_wire("DCM_OUT_R", &["CLKT_DLL_OUTR3"])
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual("GLOBALSIG.T"),
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
                    kind,
                    xy,
                    &[],
                    &[xy_l],
                    kind,
                    &[
                        builder
                            .bel_xy("BUFGMUX0", "BUFGMUX", 1, 1)
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
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTL0".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR0",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR0".to_string(),
                                    wire_from: "CLKV_OMUX10_OUTR0".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK0"]),
                        builder
                            .bel_xy("BUFGMUX1", "BUFGMUX", 1, 0)
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
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTL1".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR1",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR1".to_string(),
                                    wire_from: "CLKV_OMUX11_OUTR1".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK1"]),
                        builder
                            .bel_xy("BUFGMUX2", "BUFGMUX", 0, 1)
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
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTL2".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR2",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR2".to_string(),
                                    wire_from: "CLKV_OMUX12_OUTR2".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK2"]),
                        builder
                            .bel_xy("BUFGMUX3", "BUFGMUX", 0, 0)
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
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTL3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTL3".to_string(),
                                },
                            )
                            .extra_wire_force_pip(
                                "DCM_OUT_R",
                                "CLKT_DLL_OUTR3",
                                NodeExtPipNaming {
                                    tile: NodeRawTileId::from_idx(1),
                                    wire_to: "CLKV_OUTR3".to_string(),
                                    wire_from: "CLKV_OMUX15_OUTR3".to_string(),
                                },
                            )
                            .extra_int_in("CLK", &["CLKT_GCLK3"]),
                        builder.bel_virtual("GLOBALSIG.T"),
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

    if rd.family != "spartan3" {
        for tkn in ["CLKL", "CLKR"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                let xy_o = xy.delta(if xy.x == 0 { 1 } else { -1 }, 0);
                let int_s_xy = builder.walk_to_int(xy_o, Dir::S).unwrap();
                let int_n_xy = builder.walk_to_int(xy_o, Dir::N).unwrap();
                let int_xy = [int_s_xy, int_n_xy];
                let kind;
                let buf_xy;
                let mut bels = [
                    builder
                        .bel_xy("BUFGMUX0", "BUFGMUX", 0, 0)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI0", "CLKR_CKI0"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT0", "CLKR_OUTT0"])
                        .extra_int_in("CLK", &["CLKL_GCLK0", "CLKR_GCLK0"]),
                    builder
                        .bel_xy("BUFGMUX1", "BUFGMUX", 0, 1)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI1", "CLKR_CKI1"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT1", "CLKR_OUTT1"])
                        .extra_int_in("CLK", &["CLKL_GCLK1", "CLKR_GCLK1"]),
                    builder
                        .bel_xy("BUFGMUX2", "BUFGMUX", 0, 2)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI2", "CLKR_CKI2"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT2", "CLKR_OUTT2"])
                        .extra_int_in("CLK", &["CLKL_GCLK2", "CLKR_GCLK2"]),
                    builder
                        .bel_xy("BUFGMUX3", "BUFGMUX", 0, 3)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI3", "CLKR_CKI3"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTT3", "CLKR_OUTT3"])
                        .extra_int_in("CLK", &["CLKL_GCLK3", "CLKR_GCLK3"]),
                    builder
                        .bel_xy("BUFGMUX4", "BUFGMUX", 0, 4)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI4", "CLKR_CKI4"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB0", "CLKR_OUTB0"])
                        .extra_int_in("CLK", &["CLKL_GCLK4", "CLKR_GCLK4"]),
                    builder
                        .bel_xy("BUFGMUX5", "BUFGMUX", 0, 5)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI5", "CLKR_CKI5"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB1", "CLKR_OUTB1"])
                        .extra_int_in("CLK", &["CLKL_GCLK5", "CLKR_GCLK5"]),
                    builder
                        .bel_xy("BUFGMUX6", "BUFGMUX", 0, 6)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI6", "CLKR_CKI6"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB2", "CLKR_OUTB2"])
                        .extra_int_in("CLK", &["CLKL_GCLK6", "CLKR_GCLK6"]),
                    builder
                        .bel_xy("BUFGMUX7", "BUFGMUX", 0, 7)
                        .pins_name_only(&["I0", "I1"])
                        .extra_wire("CKI", &["CLKL_CKI7", "CLKR_CKI7"])
                        .extra_wire("DCM_OUT", &["CLKL_OUTB3", "CLKR_OUTB3"])
                        .extra_int_in("CLK", &["CLKL_GCLK7", "CLKR_GCLK7"]),
                    builder
                        .bel_xy("PCILOGICSE", "PCILOGIC", 0, 0)
                        .pin_name_only("PCI_CE", 1)
                        .pin_name_only("IRDY", 1)
                        .pin_name_only("TRDY", 1),
                    builder
                        .bel_xy("VCC", "VCC", 0, 0)
                        .pin_name_only("VCCOUT", 0),
                    builder.bel_virtual("GLOBALSIG.LR"),
                ];
                if rd.family == "spartan3e" {
                    kind = format!("{tkn}.S3E");
                    buf_xy = vec![];
                } else {
                    kind = format!("{tkn}.S3A");
                    buf_xy = vec![xy_o];
                    let mut i = 0;
                    bels = bels.map(|x| {
                        if x.name.starts_with("BUFGMUX") {
                            let res = x.extra_wire_force("DCM_PAD", format!("{tkn}_CKI{i}_END"));
                            i += 1;
                            res
                        } else {
                            x
                        }
                    });
                }
                builder.extract_xnode_bels(&kind, xy, &buf_xy, &int_xy, &kind, &bels);
            }
        }

        for tkn in ["GCLKH_PCI_CE_N"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                builder.extract_xnode_bels(
                    "PCI_CE_N",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_N",
                    &[builder
                        .bel_virtual("PCI_CE_N")
                        .extra_wire("I", &["GCLKH_PCI_CE_IN"])
                        .extra_wire("O", &["GCLKH_PCI_CE_OUT"])],
                );
            }
        }
        for tkn in ["GCLKH_PCI_CE_S", "GCLKH_PCI_CE_S_50A"] {
            for &xy in rd.tiles_by_kind_name(tkn) {
                builder.extract_xnode_bels(
                    "PCI_CE_S",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_S",
                    &[builder
                        .bel_virtual("PCI_CE_S")
                        .extra_wire("I", &["GCLKH_PCI_CE_OUT"])
                        .extra_wire("O", &["GCLKH_PCI_CE_IN"])],
                );
            }
        }
        for tkn in ["LL", "LR", "UL", "UR"] {
            builder.extract_node_bels(
                tkn,
                "PCI_CE_CNR",
                "PCI_CE_CNR",
                &[builder
                    .bel_virtual("PCI_CE_CNR")
                    .extra_wire("I", &["PCI_CE_NS"])
                    .extra_wire("O", &["PCI_CE_EW"])],
            );
        }
        if rd.family == "spartan3a" {
            for &xy in rd.tiles_by_kind_name("GCLKV_IOISL") {
                builder.extract_xnode_bels(
                    "PCI_CE_E",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_E",
                    &[builder
                        .bel_virtual("PCI_CE_E")
                        .extra_wire("I", &["CLKV_PCI_CE_W"])
                        .extra_wire("O", &["CLKV_PCI_CE_E"])],
                );
            }
            for &xy in rd.tiles_by_kind_name("GCLKV_IOISR") {
                builder.extract_xnode_bels(
                    "PCI_CE_W",
                    xy,
                    &[],
                    &[xy],
                    "PCI_CE_W",
                    &[builder
                        .bel_virtual("PCI_CE_W")
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
                    "BRAM.S3ADSP",
                    xy,
                    &[],
                    &int_xy,
                    "BRAM.S3ADSP",
                    &[builder.bel_xy("BRAM", "RAMB16", 0, 0)],
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
                .bel_xy("DSP", "DSP48A", 0, 0)
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
                builder.extract_xnode_bels("DSP", xy, &[], &int_xy, naming, &bels_dsp);
                builder.extract_intf_tile_multi("INTF.DSP", xy, &int_xy, "INTF.DSP", false);
            }
        }
    } else {
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
            let mut bel_mult = builder.bel_xy("MULT", "MULT18X18", 0, 0);
            let buf_cnt = if naming == "BRAM.S3A.TOP" { 0 } else { 1 };
            for i in 0..18 {
                bel_mult = bel_mult.pin_name_only(&format!("BCIN{i}"), 0);
                bel_mult = bel_mult.pin_name_only(&format!("BCOUT{i}"), buf_cnt);
            }
            let bels_bram = [builder.bel_xy("BRAM", "RAMB16", 0, 0), bel_mult];
            for &xy in rd.tiles_by_kind_name(tkn) {
                let mut int_xy = Vec::new();
                for dy in 0..4 {
                    int_xy.push(xy.delta(-1, dy));
                }
                builder.extract_xnode_bels(kind, xy, &[], &int_xy, naming, &bels_bram);
            }
        }
    }

    for tkn in ["CLKC", "CLKC_LL"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual("CLKC");
            for i in 0..4 {
                bel = bel
                    .extra_wire(format!("IN_B{i}"), &[format!("CLKC_GCLK_MAIN_B{i}")])
                    .extra_wire(format!("IN_T{i}"), &[format!("CLKC_GCLK_MAIN_T{i}")])
            }
            for i in 0..8 {
                bel = bel.extra_wire(format!("OUT{i}"), &[format!("CLKC_GCLK{i}")]);
            }
            builder.extract_xnode_bels("CLKC", xy, &[], &[xy], "CLKC", &[bel]);
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC_50A") {
        let mut bel = builder.bel_virtual("CLKC_50A");
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
        builder.extract_xnode_bels("CLKC_50A", xy, &[], &[xy], "CLKC_50A", &[bel]);
    }

    for &xy in rd.tiles_by_kind_name("GCLKVM") {
        let mut bel = builder.bel_virtual("GCLKVM");
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN_CORE{i}"), &[format!("GCLKVM_GCLK{i}")])
                .extra_wire(format!("OUT_DN{i}"), &[format!("GCLKVM_GCLK_DN{i}")])
                .extra_wire(format!("OUT_UP{i}"), &[format!("GCLKVM_GCLK_UP{i}")]);
        }
        builder.extract_xnode_bels("GCLKVM.S3", xy, &[], &[xy], "GCLKVM.S3", &[bel]);
    }

    for tkn in ["GCLKVML", "GCLKVMR"] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut bel = builder.bel_virtual("GCLKVM");
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
                    .extra_wire(format!("OUT_DN{i}"), &[format!("GCLKVMLR_GCLK_DN{i}")])
                    .extra_wire(format!("OUT_UP{i}"), &[format!("GCLKVMLR_GCLK_UP{i}")]);
            }
            builder.extract_xnode_bels("GCLKVM.S3E", xy, &[], &[xy], tkn, &[bel]);
        }
    }

    for &xy in rd.tiles_by_kind_name("GCLKVC") {
        let mut bel = builder.bel_virtual("GCLKVC");
        for i in 0..8 {
            bel = bel
                .extra_wire(format!("IN{i}"), &[format!("GCLKC_GCLK{i}")])
                .extra_wire(format!("OUT_L{i}"), &[format!("GCLKC_GCLK_OUT_L{i}")])
                .extra_wire(format!("OUT_R{i}"), &[format!("GCLKC_GCLK_OUT_R{i}")]);
        }
        builder.extract_xnode_bels("GCLKVC", xy, &[], &[xy], "GCLKVC", &[bel]);
    }

    for tkn in [
        "GCLKH",
        "GCLKH_PCI_CE_S",
        "GCLKH_PCI_CE_N",
        "GCLKH_PCI_CE_S_50A",
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_s_xy = builder.walk_to_int(xy, Dir::S).unwrap();
            let int_n_xy = builder.walk_to_int(xy, Dir::N).unwrap();
            let mut bel = builder.bel_virtual("GCLKH");
            for i in 0..8 {
                bel = bel
                    .extra_wire(format!("IN{i}"), &[format!("GCLKH_GCLK{i}")])
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

    if rd.family != "spartan3" {
        let dummy_xy = Coord { x: 0, y: 0 };
        let bel_globalsig = builder.bel_virtual("GLOBALSIG");
        let mut bel = builder.bel_virtual("GCLKH");
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_UP{i}"),
                    (NodeTileId::from_idx(1), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_UP{i}"),
                )
                .extra_int_out_force(
                    format!("OUT_DN{i}"),
                    (NodeTileId::from_idx(0), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_DN{i}"),
                );
        }
        builder.extract_xnode_bels(
            "GCLKH",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.BRAM",
            &[bel_globalsig.clone(), bel],
        );
        let mut bel = builder.bel_virtual("GCLKH.S");
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_DN{i}"),
                    (NodeTileId::from_idx(0), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_DN{i}"),
                );
        }
        builder.extract_xnode_bels(
            "GCLKH.S",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.BRAM.S",
            &[bel_globalsig.clone(), bel],
        );
        let mut bel = builder.bel_virtual("GCLKH.N");
        for i in 0..8 {
            bel = bel
                .extra_wire_force(format!("IN{i}"), format!("BRAMSITE2_GCLKH_GCLK{i}"))
                .extra_int_out_force(
                    format!("OUT_UP{i}"),
                    (NodeTileId::from_idx(1), gclk[i]),
                    format!("BRAMSITE2_GCLKH_GCLK_UP{i}"),
                )
        }
        builder.extract_xnode_bels(
            "GCLKH.N",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.BRAM.N",
            &[bel_globalsig.clone(), bel],
        );
        builder.extract_xnode_bels(
            "GCLKH.0",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.0",
            &[bel_globalsig],
        );
        builder.extract_xnode_bels(
            "GCLKH.DSP",
            dummy_xy,
            &[],
            &[dummy_xy, dummy_xy],
            "GCLKH.DSP",
            &[builder.bel_virtual("GLOBALSIG.DSP")],
        );
    }

    builder.build()
}
