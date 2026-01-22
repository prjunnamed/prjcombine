use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelPin, IntDb, LegacyBel, TileWireCoord},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{BelNaming, BelPinNaming, NamingDb, PipNaming, RawTileId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_virtex::defs::{self, wires};
use std::collections::BTreeMap;

use prjcombine_re_xilinx_rd2db_grid::find_columns;
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let mut builder = IntBuilder::new(
        rd,
        bincode::decode_from_slice(defs::INIT, bincode::config::standard())
            .unwrap()
            .0,
    );

    builder.allow_mux_to_branch();

    builder.inject_main_passes(DirMap::from_fn(|dir| match dir {
        Dir::W => defs::ccls::PASS_W,
        Dir::E => defs::ccls::PASS_E,
        Dir::S => defs::ccls::PASS_S,
        Dir::N => defs::ccls::PASS_N,
    }));

    for i in 0..4 {
        builder.wire_names(
            wires::GCLK[i],
            &[
                format!("GCLK{i}"),
                format!("LEFT_GCLK{i}"),
                format!("RIGHT_GCLK{i}"),
                format!("BOT_HGCLK{i}"),
                format!("TOP_HGCLK{i}"),
                format!("LL_GCLK{i}"),
                format!("UL_GCLK{i}"),
                format!("BRAM_GCLKIN{i}"),
                format!("BRAM_BOT_GCLKE{i}"),
                format!("BRAM_TOP_GCLKE{i}"),
                format!("BRAM_BOTP_GCLK{i}"),
                format!("BRAM_TOPP_GCLK{i}"),
                format!("BRAM_BOTS_GCLK{i}"),
                format!("BRAM_TOPS_GCLK{i}"),
            ],
        );
        builder.extra_name_sub(format!("MBRAM_GCLKD{i}"), 0, wires::GCLK[i]);
        builder.extra_name_sub(format!("MBRAM_GCLKA{i}"), 3, wires::GCLK[i]);
        builder.extra_name_sub(format!("BRAM_BOT_VGCLK{i}"), 2, wires::GCLK[i]);
        builder.extra_name_sub(format!("BRAM_TOP_VGCLK{i}"), 2, wires::GCLK[i]);
        builder.wire_names(
            wires::GCLK_BUF[i],
            &[format!("BOT_GCLK{i}"), format!("TOP_GCLK{i}")],
        );
        builder.mark_permabuf(wires::GCLK_BUF[i]);
    }

    builder.wire_names(
        wires::PCI_CE,
        &[
            "LEFT_PCI_CE",
            "RIGHT_PCI_CE",
            "LL_PCI_CE",
            "LR_PCI_CE",
            "UL_PCI_CE",
            "UR_PCI_CE",
        ],
    );

    for i in 0..24 {
        builder.wire_names(wires::SINGLE_E[i], &[format!("E{i}"), format!("LEFT_E{i}")]);
        builder.mark_permabuf(wires::SINGLE_E_BUF[i]);
        builder.wire_names(
            wires::SINGLE_E_BUF[i],
            &[format!("E_P{i}"), format!("LEFT_E_BUF{i}")],
        );
        builder.wire_names(
            wires::SINGLE_W[i],
            &[format!("W{i}"), format!("RIGHT_W{i}")],
        );
        builder.mark_permabuf(wires::SINGLE_W_BUF[i]);
        builder.wire_names(
            wires::SINGLE_W_BUF[i],
            &[format!("W_P{i}"), format!("RIGHT_W_BUF{i}")],
        );
    }
    for i in 0..24 {
        builder.wire_names(wires::SINGLE_S[i], &[format!("S{i}"), format!("TOP_S{i}")]);
        builder.mark_permabuf(wires::SINGLE_S_BUF[i]);
        builder.wire_names(
            wires::SINGLE_S_BUF[i],
            &[format!("S_P{i}"), format!("TOP_S_BUF{i}")],
        );
        builder.wire_names(wires::SINGLE_N[i], &[format!("N{i}"), format!("BOT_N{i}")]);
        builder.mark_permabuf(wires::SINGLE_N_BUF[i]);
        builder.wire_names(
            wires::SINGLE_N_BUF[i],
            &[format!("N_P{i}"), format!("BOT_N_BUF{i}")],
        );
    }

    for (name, w, ws) in [
        ("ADDR", wires::BRAM_QUAD_ADDR, wires::BRAM_QUAD_ADDR_S),
        ("DIN", wires::BRAM_QUAD_DIN, wires::BRAM_QUAD_DIN_S),
        ("DOUT", wires::BRAM_QUAD_DOUT, wires::BRAM_QUAD_DOUT_S),
    ] {
        for i in 0..32 {
            builder.wire_names(w[i], &[format!("BRAM_R{name}S{i}")]);
            builder.wire_names(ws[i], &[format!("BRAM_R{name}N{i}")]);
        }
    }

    let hexnames = |pref, i| {
        [
            format!("{pref}{i}"),
            format!("LEFT_{pref}{i}"),
            format!("RIGHT_{pref}{i}"),
            format!("TOP_{pref}{i}"),
            format!("BOT_{pref}{i}"),
            format!("LL_{pref}{i}"),
            format!("LR_{pref}{i}"),
            format!("UL_{pref}{i}"),
            format!("UR_{pref}{i}"),
        ]
    };
    let hexnames_hc = |pref, i| {
        [
            format!("{pref}{i}"),
            format!("LEFT_{pref}{i}"),
            format!("RIGHT_{pref}{i}"),
        ]
    };
    let hexnames_hio = |pref, i| {
        [
            format!("TOP_{pref}{i}"),
            format!("BOT_{pref}{i}"),
            format!("LL_{pref}{i}"),
            format!("LR_{pref}{i}"),
            format!("UL_{pref}{i}"),
            format!("UR_{pref}{i}"),
        ]
    };
    for i in 0..4 {
        builder.wire_names(wires::HEX_H0[i], &hexnames("H6E", i));
        builder.wire_names(wires::HEX_H1[i], &hexnames("H6A", i));
        builder.wire_names(wires::HEX_H2[i], &hexnames("H6B", i));
        builder.wire_names(wires::HEX_H3[i], &hexnames("H6M", i));
        builder.wire_names(wires::HEX_H4[i], &hexnames("H6C", i));
        builder.wire_names(wires::HEX_H5[i], &hexnames("H6D", i));
        builder.wire_names(wires::HEX_H6[i], &hexnames("H6W", i));
        builder.mark_permabuf(wires::HEX_H0_BUF[i]);
        builder.mark_permabuf(wires::HEX_H1_BUF[i]);
        builder.mark_permabuf(wires::HEX_H2_BUF[i]);
        builder.mark_permabuf(wires::HEX_H3_BUF[i]);
        builder.mark_permabuf(wires::HEX_H4_BUF[i]);
        builder.mark_permabuf(wires::HEX_H5_BUF[i]);
        builder.mark_permabuf(wires::HEX_H6_BUF[i]);
        builder.wire_names(wires::HEX_H0_BUF[i], &hexnames("H6E_BUF", i));
        builder.wire_names(wires::HEX_H1_BUF[i], &hexnames("H6A_BUF", i));
        builder.wire_names(wires::HEX_H2_BUF[i], &hexnames("H6B_BUF", i));
        builder.wire_names(wires::HEX_H3_BUF[i], &hexnames("H6M_BUF", i));
        builder.wire_names(wires::HEX_H4_BUF[i], &hexnames("H6C_BUF", i));
        builder.wire_names(wires::HEX_H5_BUF[i], &hexnames("H6D_BUF", i));
        builder.wire_names(wires::HEX_H6_BUF[i], &hexnames("H6W_BUF", i));
    }
    for i in 4..6 {
        builder.wire_names(wires::HEX_H0[i], &hexnames_hio("H6E", i));
        builder.wire_names(wires::HEX_H1[i], &hexnames_hio("H6A", i));
        builder.wire_names(wires::HEX_H2[i], &hexnames_hio("H6B", i));
        builder.wire_names(wires::HEX_H3[i], &hexnames_hio("H6M", i));
        builder.wire_names(wires::HEX_H4[i], &hexnames_hio("H6C", i));
        builder.wire_names(wires::HEX_H5[i], &hexnames_hio("H6D", i));
        builder.wire_names(wires::HEX_H6[i], &hexnames_hio("H6W", i));
    }
    for i in 0..4 {
        let ii = 4 + i * 2;
        builder.wire_names(wires::HEX_W0[i], &hexnames_hc("H6W", ii));
        builder.wire_names(wires::HEX_W1[i], &hexnames_hc("H6D", ii));
        builder.wire_names(wires::HEX_W2[i], &hexnames_hc("H6C", ii));
        builder.wire_names(wires::HEX_W3[i], &hexnames_hc("H6M", ii));
        builder.wire_names(wires::HEX_W4[i], &hexnames_hc("H6B", ii));
        builder.wire_names(wires::HEX_W5[i], &hexnames_hc("H6A", ii));
        builder.wire_names(wires::HEX_W6[i], &hexnames_hc("H6E", ii));
    }
    for i in 0..4 {
        let ii = 5 + i * 2;
        builder.wire_names(wires::HEX_E0[i], &hexnames_hc("H6E", ii));
        builder.wire_names(wires::HEX_E1[i], &hexnames_hc("H6A", ii));
        builder.wire_names(wires::HEX_E2[i], &hexnames_hc("H6B", ii));
        builder.wire_names(wires::HEX_E3[i], &hexnames_hc("H6M", ii));
        builder.wire_names(wires::HEX_E4[i], &hexnames_hc("H6C", ii));
        builder.wire_names(wires::HEX_E5[i], &hexnames_hc("H6D", ii));
        builder.wire_names(wires::HEX_E6[i], &hexnames_hc("H6W", ii));
    }
    for i in 0..4 {
        builder.wire_names(wires::HEX_V0[i], &hexnames("V6N", i));
        builder.wire_names(wires::HEX_V1[i], &hexnames("V6A", i));
        builder.wire_names(wires::HEX_V2[i], &hexnames("V6B", i));
        builder.wire_names(wires::HEX_V3[i], &hexnames("V6M", i));
        builder.wire_names(wires::HEX_V4[i], &hexnames("V6C", i));
        builder.wire_names(wires::HEX_V5[i], &hexnames("V6D", i));
        builder.wire_names(wires::HEX_V6[i], &hexnames("V6S", i));
        builder.mark_permabuf(wires::HEX_V0_BUF[i]);
        builder.mark_permabuf(wires::HEX_V1_BUF[i]);
        builder.mark_permabuf(wires::HEX_V2_BUF[i]);
        builder.mark_permabuf(wires::HEX_V3_BUF[i]);
        builder.mark_permabuf(wires::HEX_V4_BUF[i]);
        builder.mark_permabuf(wires::HEX_V5_BUF[i]);
        builder.mark_permabuf(wires::HEX_V6_BUF[i]);
        builder.wire_names(wires::HEX_V0_BUF[i], &hexnames("V6N_BUF", i));
        builder.wire_names(wires::HEX_V1_BUF[i], &hexnames("V6A_BUF", i));
        builder.wire_names(wires::HEX_V2_BUF[i], &hexnames("V6B_BUF", i));
        builder.wire_names(wires::HEX_V3_BUF[i], &hexnames("V6M_BUF", i));
        builder.wire_names(wires::HEX_V4_BUF[i], &hexnames("V6C_BUF", i));
        builder.wire_names(wires::HEX_V5_BUF[i], &hexnames("V6D_BUF", i));
        builder.wire_names(wires::HEX_V6_BUF[i], &hexnames("V6S_BUF", i));
    }
    for i in 0..4 {
        let ii = 4 + i * 2;
        builder.wire_names(wires::HEX_S0[i], &hexnames("V6S", ii));
        builder.wire_names(wires::HEX_S1[i], &hexnames("V6D", ii));
        builder.wire_names(wires::HEX_S2[i], &hexnames("V6C", ii));
        builder.wire_names(wires::HEX_S3[i], &hexnames("V6M", ii));
        builder.wire_names(wires::HEX_S4[i], &hexnames("V6B", ii));
        builder.wire_names(wires::HEX_S5[i], &hexnames("V6A", ii));
        builder.wire_names(wires::HEX_S6[i], &hexnames("V6N", ii));
    }
    for i in 0..4 {
        let ii = 5 + i * 2;
        builder.wire_names(wires::HEX_N0[i], &hexnames("V6N", ii));
        builder.wire_names(wires::HEX_N1[i], &hexnames("V6A", ii));
        builder.wire_names(wires::HEX_N2[i], &hexnames("V6B", ii));
        builder.wire_names(wires::HEX_N3[i], &hexnames("V6M", ii));
        builder.wire_names(wires::HEX_N4[i], &hexnames("V6C", ii));
        builder.wire_names(wires::HEX_N5[i], &hexnames("V6D", ii));
        builder.wire_names(wires::HEX_N6[i], &hexnames("V6S", ii));
    }

    for i in 0..12 {
        builder.wire_names(
            wires::LH[i],
            &[
                format!("LH{i}"),
                format!("LEFT_LH{i}"),
                format!("RIGHT_LH{i}"),
                format!("BOT_LH{i}"),
                format!("TOP_LH{i}"),
                format!("LL_LH{i}"),
                format!("LR_LH{i}"),
                format!("UL_LH{i}"),
                format!("UR_LH{i}"),
            ],
        );
    }
    builder.mark_permabuf(wires::LH_FAKE0);
    builder.mark_permabuf(wires::LH_FAKE6);
    builder.wire_names(wires::LH_FAKE0, &["TOP_FAKE_LH0", "BOT_FAKE_LH0"]);
    builder.wire_names(wires::LH_FAKE6, &["TOP_FAKE_LH6", "BOT_FAKE_LH6"]);

    for i in 0..12 {
        builder.wire_names(
            wires::LV[i],
            &[
                format!("LV{i}"),
                format!("LEFT_LV{i}"),
                format!("RIGHT_LV{i}"),
                format!("BOT_LV{i}"),
                format!("TOP_LV{i}"),
                format!("LL_LV{i}"),
                format!("LR_LV{i}"),
                format!("UL_LV{i}"),
                format!("UR_LV{i}"),
                format!("BRAM_LV{i}"),
                format!("BRAM_BOT_RLV{ii}", ii = (i + 11) % 12),
                format!("BRAM_BOTP_RLV{ii}", ii = (i + 11) % 12),
                format!("BRAM_TOP_RLV{i}"),
                format!("BRAM_TOPP_RLV{i}"),
            ],
        );
    }

    for (pin, w) in [
        ("CLK_B", wires::IMUX_CLB_CLK),
        ("SR_B", wires::IMUX_CLB_SR),
        ("CE_B", wires::IMUX_CLB_CE),
        ("BX_B", wires::IMUX_CLB_BX),
        ("BY_B", wires::IMUX_CLB_BY),
        ("F_B1", wires::IMUX_CLB_F1),
        ("F_B2", wires::IMUX_CLB_F2),
        ("F_B3", wires::IMUX_CLB_F3),
        ("F_B4", wires::IMUX_CLB_F4),
        ("G_B1", wires::IMUX_CLB_G1),
        ("G_B2", wires::IMUX_CLB_G2),
        ("G_B3", wires::IMUX_CLB_G3),
        ("G_B4", wires::IMUX_CLB_G4),
    ] {
        for i in 0..2 {
            builder.wire_names(w[i], &[format!("S{i}_{pin}")]);
        }
    }
    for i in 0..2 {
        builder.wire_names(
            wires::IMUX_TBUF_T[i],
            &[
                format!("TS_B{i}"),
                format!("LEFT_TS{i}_B"),
                format!("RIGHT_TS{i}_B"),
            ],
        );
        builder.wire_names(
            wires::IMUX_TBUF_I[i],
            &[
                format!("T_IN{i}"),
                format!("LEFT_TI{i}_B"),
                format!("RIGHT_TI{i}_B"),
            ],
        );
    }
    for (pin, w) in [
        ("CLK", wires::IMUX_IO_CLK),
        ("SR_B", wires::IMUX_IO_SR),
        ("ICE", wires::IMUX_IO_ICE),
        ("OCE", wires::IMUX_IO_OCE),
        ("TCE", wires::IMUX_IO_TCE),
        ("O", wires::IMUX_IO_O),
        ("T", wires::IMUX_IO_T),
    ] {
        for i in 0..4 {
            builder.wire_names(
                w[i],
                &[
                    format!("LEFT_{pin}{i}"),
                    format!("RIGHT_{pin}{i}"),
                    format!("BOT_{pin}{i}"),
                    format!("TOP_{pin}{i}"),
                ],
            );
        }
    }
    builder.wire_names(wires::IMUX_CAP_CLK, &["LL_CAPTURE_CLK"]);
    builder.wire_names(wires::IMUX_CAP_CAP, &["LL_CAP"]);
    builder.wire_names(wires::IMUX_STARTUP_CLK, &["UL_STARTUP_CLK"]);
    builder.wire_names(wires::IMUX_STARTUP_GSR, &["UL_GSR"]);
    builder.wire_names(wires::IMUX_STARTUP_GTS, &["UL_GTS"]);
    builder.wire_names(wires::IMUX_STARTUP_GWE, &["UL_GWE"]);
    builder.wire_names(wires::IMUX_BSCAN_TDO1, &["UL_TDO1"]);
    builder.wire_names(wires::IMUX_BSCAN_TDO2, &["UL_TDO2"]);

    for (ab, w) in [('A', wires::IMUX_BRAM_DIA), ('B', wires::IMUX_BRAM_DIB)] {
        for i in 0..16 {
            builder.wire_names(w[i], &[format!("BRAM_DI{ab}{i}")]);
        }
    }
    for (ab, w) in [('A', wires::IMUX_BRAM_ADDRA), ('B', wires::IMUX_BRAM_ADDRB)] {
        for i in 0..12 {
            builder.wire_names(w[i], &[format!("BRAM_ADDR{ab}{i}")]);
        }
    }
    for (pin, w) in [
        ("CLKA", wires::IMUX_BRAM_CLKA),
        ("CLKB", wires::IMUX_BRAM_CLKB),
        ("RSTA", wires::IMUX_BRAM_RSTA),
        ("RSTB", wires::IMUX_BRAM_RSTB),
        ("SELA", wires::IMUX_BRAM_SELA),
        ("SELB", wires::IMUX_BRAM_SELB),
        ("WEA", wires::IMUX_BRAM_WEA),
        ("WEB", wires::IMUX_BRAM_WEB),
    ] {
        builder.wire_names(w, &[format!("BRAM_{pin}"), format!("MBRAM_{pin}")]);
    }

    for i in 0..8 {
        builder.wire_names(
            wires::OMUX[i],
            &[
                format!("OUT{i}"),
                format!("LEFT_OUT{i}"),
                format!("RIGHT_OUT{i}"),
            ],
        );
    }
    for (i, w) in [(0, wires::OMUX_E0), (1, wires::OMUX_E1)] {
        builder.wire_names(w, &[format!("OUT_W{i}"), format!("RIGHT_OUT_W{i}")]);
    }
    for (i, w) in [(6, wires::OMUX_W6), (7, wires::OMUX_W7)] {
        builder.wire_names(w, &[format!("OUT_E{i}"), format!("LEFT_OUT_E{i}")]);
    }

    for (pin, w) in [
        ("X", wires::OUT_CLB_X),
        ("Y", wires::OUT_CLB_Y),
        ("XQ", wires::OUT_CLB_XQ),
        ("YQ", wires::OUT_CLB_YQ),
        ("XB", wires::OUT_CLB_XB),
        ("YB", wires::OUT_CLB_YB),
    ] {
        for i in 0..2 {
            builder.wire_names(w[i], &[format!("S{i}_{pin}")]);
        }
    }
    builder.wire_names(wires::OUT_TBUF, &["TBUFO"]);
    for i in 0..4 {
        builder.wire_names(wires::OUT_TBUF_W[i], &[format!("LEFT_TBUFO{i}")]);
    }
    for i in 0..4 {
        builder.wire_names(wires::OUT_TBUF_E[i], &[format!("RIGHT_TBUFO{i}")]);
    }
    for (pin, w) in [("I", wires::OUT_IO_I), ("IQ", wires::OUT_IO_IQ)] {
        for i in 0..4 {
            builder.wire_names(
                w[i],
                &[
                    format!("LEFT_{pin}{i}"),
                    format!("RIGHT_{pin}{i}"),
                    format!("BOT_{pin}{i}"),
                    format!("TOP_{pin}{i}"),
                ],
            );
        }
    }
    for (pin, w) in [
        ("RESET", wires::OUT_BSCAN_RESET),
        ("DRCK1", wires::OUT_BSCAN_DRCK1),
        ("DRCK2", wires::OUT_BSCAN_DRCK2),
        ("SHIFT", wires::OUT_BSCAN_SHIFT),
        ("TDI", wires::OUT_BSCAN_TDI),
        ("UPDATE", wires::OUT_BSCAN_UPDATE),
        ("SEL1", wires::OUT_BSCAN_SEL1),
        ("SEL2", wires::OUT_BSCAN_SEL2),
    ] {
        builder.wire_names(w, &[format!("UL_{pin}")]);
    }

    for (pin, w) in [("DOA", wires::OUT_BRAM_DOA), ("DOB", wires::OUT_BRAM_DOB)] {
        for i in 0..16 {
            builder.wire_names(w[i], &[format!("BRAM_{pin}{i}")]);
        }
    }

    for i in 0..2 {
        builder.wire_names(
            wires::IMUX_BUFGCE_CLK[i],
            &[
                format!("CLKB_GCLKBUF{i}_IN"),
                format!("CLKT_GCLKBUF{ii}_IN", ii = i + 2),
            ],
        );
    }
    for i in 0..2 {
        builder.wire_names(
            wires::IMUX_BUFGCE_CE[i],
            &[format!("CLKB_CE{i}"), format!("CLKT_CE{i}")],
        );
    }
    for i in 0..2 {
        builder.wire_names(
            wires::OUT_BUFGCE_O[i],
            &[
                format!("CLKB_GCLK{i}_PW"),
                format!("CLKT_GCLK{ii}_PW", ii = i + 2),
            ],
        );
    }
    for i in 0..2 {
        builder.wire_names(
            wires::OUT_CLKPAD[i],
            &[format!("CLKB_CLKPAD{i}"), format!("CLKT_CLKPAD{i}")],
        );
    }
    for i in 0..2 {
        builder.wire_names(
            wires::OUT_IOFB[i],
            &[format!("CLKB_IOFB{i}"), format!("CLKT_IOFB{i}")],
        );
    }
    for (i, w) in [
        (1, wires::IMUX_PCI_I1),
        (2, wires::IMUX_PCI_I2),
        (3, wires::IMUX_PCI_I3),
    ] {
        builder.wire_names(w, &[format!("CLKL_I{i}"), format!("CLKR_I{i}")]);
    }
    let mut dll_pins = BTreeMap::new();
    for (name, w) in [
        ("CLKIN", wires::IMUX_DLL_CLKIN),
        ("CLKFB", wires::IMUX_DLL_CLKFB),
        ("RST", wires::IMUX_DLL_RST),
    ] {
        builder.wire_names(
            w,
            &[
                format!("BRAM_BOT_{name}"),
                format!("BRAM_BOTP_{name}"),
                format!("BRAM_BOT_{name}_1"),
                format!("BRAM_TOP_{name}"),
                format!("BRAM_TOPP_{name}"),
                format!("BRAM_TOPS_{name}"),
            ],
        );
        builder.extra_name_sub(format!("CLKB_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKT_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 4, w);
        dll_pins.insert(
            name.to_string(),
            BelPin::new_in(TileWireCoord::new_idx(0, w)),
        );
    }
    for (name, w) in [
        ("CLK0", wires::OUT_DLL_CLK0),
        ("CLK90", wires::OUT_DLL_CLK90),
        ("CLK180", wires::OUT_DLL_CLK180),
        ("CLK270", wires::OUT_DLL_CLK270),
        ("CLK2X", wires::OUT_DLL_CLK2X),
        ("CLK2X90", wires::OUT_DLL_CLK2X90),
        ("CLKDV", wires::OUT_DLL_CLKDV),
        ("LOCKED", wires::OUT_DLL_LOCKED),
    ] {
        builder.extra_name_sub(format!("CLKB_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 1, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 2, w);
        if name == "LOCKED" {
            builder.extra_name_sub("CLKT_LOCK_TL_1", 3, w);
        } else {
            builder.extra_name_sub(format!("CLKT_{name}L_1"), 3, w);
        }
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 4, w);
        dll_pins.insert(
            name.to_string(),
            BelPin::new_out(TileWireCoord::new_idx(0, w)),
        );
    }

    let slice_name_only = ["F5IN", "F5", "CIN", "COUT"];

    builder.extract_int_id(
        defs::tcls::CLB,
        defs::bslots::INT,
        "CENTER",
        "CLB",
        &[
            builder
                .bel_indexed(defs::bslots::SLICE[0], "SLICE", 0)
                .pins_name_only(&slice_name_only)
                .pin_name_only("COUT", 1),
            builder
                .bel_indexed(defs::bslots::SLICE[1], "SLICE", 1)
                .pins_name_only(&slice_name_only)
                .pin_name_only("COUT", 1),
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
                .extra_wire("BUS3_E", &["TBUF_STUB3"])
                .extra_int_out("OUT", &["TBUFO"]),
        ],
    );

    let bels_left = [
        builder.bel_indexed(defs::bslots::IO[0], "IOB", 0),
        builder
            .bel_indexed(defs::bslots::IO[1], "IOB", 1)
            .extra_wire_force("PCI", "LEFT_PCI_BOT_PCI1"),
        builder.bel_indexed(defs::bslots::IO[2], "IOB", 2),
        builder
            .bel_indexed(defs::bslots::IO[3], "IOB", 3)
            .extra_wire_force("PCI", "LEFT_PCI_TOP_PCI3"),
        builder
            .bel_indexed(defs::bslots::TBUF[0], "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed(defs::bslots::TBUF[1], "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual(defs::bslots::TBUS)
            .extra_int_out("BUS0", &["LEFT_TBUFO2"])
            .extra_int_out("BUS1", &["LEFT_TBUFO3"])
            .extra_int_out("BUS2", &["LEFT_TBUFO0"])
            .extra_int_out("BUS3", &["LEFT_TBUFO1"])
            .extra_wire("BUS3_E", &["LEFT_TBUF1_STUB"]),
    ];
    builder.extract_int_id(
        defs::tcls::IO_W,
        defs::bslots::INT,
        "LEFT",
        "IO_W",
        &bels_left,
    );
    builder.extract_int_id(
        defs::tcls::IO_W,
        defs::bslots::INT,
        "LEFT_PCI_BOT",
        "IO_W",
        &bels_left,
    );
    builder.extract_int_id(
        defs::tcls::IO_W,
        defs::bslots::INT,
        "LEFT_PCI_TOP",
        "IO_W",
        &bels_left,
    );

    let bels_right = [
        builder.bel_indexed(defs::bslots::IO[0], "IOB", 0),
        builder
            .bel_indexed(defs::bslots::IO[1], "IOB", 1)
            .extra_wire_force("PCI", "RIGHT_PCI_BOT_PCI1"),
        builder.bel_indexed(defs::bslots::IO[2], "IOB", 2),
        builder
            .bel_indexed(defs::bslots::IO[3], "IOB", 3)
            .extra_wire_force("PCI", "RIGHT_PCI_TOP_PCI3"),
        builder
            .bel_indexed(defs::bslots::TBUF[0], "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed(defs::bslots::TBUF[1], "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual(defs::bslots::TBUS)
            .extra_int_out("BUS0", &["RIGHT_TBUFO2"])
            .extra_int_out("BUS1", &["RIGHT_TBUFO3"])
            .extra_int_out("BUS2", &["RIGHT_TBUFO0"])
            .extra_int_out("BUS3", &["RIGHT_TBUFO1"]),
    ];
    builder.extract_int_id(
        defs::tcls::IO_E,
        defs::bslots::INT,
        "RIGHT",
        "IO_E",
        &bels_right,
    );
    builder.extract_int_id(
        defs::tcls::IO_E,
        defs::bslots::INT,
        "RIGHT_PCI_BOT",
        "IO_E",
        &bels_right,
    );
    builder.extract_int_id(
        defs::tcls::IO_E,
        defs::bslots::INT,
        "RIGHT_PCI_TOP",
        "IO_E",
        &bels_right,
    );

    let bels_bot = [
        builder.bel_indexed(defs::bslots::IO[0], "IOB", 0),
        builder
            .bel_indexed(defs::bslots::IO[1], "IOB", 1)
            .extra_wire_force("DLLFB", "BL_DLLIOB_IOFB"),
        builder
            .bel_indexed(defs::bslots::IO[2], "IOB", 2)
            .extra_wire_force("DLLFB", "BR_DLLIOB_IOFB"),
        builder.bel_indexed(defs::bslots::IO[3], "IOB", 3),
    ];
    builder.extract_int_id(
        defs::tcls::IO_S,
        defs::bslots::INT,
        "BOT",
        "IO_S",
        &bels_bot,
    );
    builder.extract_int_id(
        defs::tcls::IO_S,
        defs::bslots::INT,
        "BL_DLLIOB",
        "IO_S",
        &bels_bot,
    );
    builder.extract_int_id(
        defs::tcls::IO_S,
        defs::bslots::INT,
        "BR_DLLIOB",
        "IO_S",
        &bels_bot,
    );

    let bels_top = [
        builder.bel_indexed(defs::bslots::IO[0], "IOB", 0),
        builder
            .bel_indexed(defs::bslots::IO[1], "IOB", 1)
            .extra_wire_force("DLLFB", "TL_DLLIOB_IOFB"),
        builder
            .bel_indexed(defs::bslots::IO[2], "IOB", 2)
            .extra_wire_force("DLLFB", "TR_DLLIOB_IOFB"),
        builder.bel_indexed(defs::bslots::IO[3], "IOB", 3),
    ];
    builder.extract_int_id(
        defs::tcls::IO_N,
        defs::bslots::INT,
        "TOP",
        "IO_N",
        &bels_top,
    );
    builder.extract_int_id(
        defs::tcls::IO_N,
        defs::bslots::INT,
        "TL_DLLIOB",
        "IO_N",
        &bels_top,
    );
    builder.extract_int_id(
        defs::tcls::IO_N,
        defs::bslots::INT,
        "TR_DLLIOB",
        "IO_N",
        &bels_top,
    );

    builder.extract_int_id(
        defs::tcls::CNR_SW,
        defs::bslots::INT,
        "LL",
        "CNR_SW",
        &[builder.bel_single(defs::bslots::CAPTURE, "CAPTURE")],
    );
    builder.extract_int_id(defs::tcls::CNR_SE, defs::bslots::INT, "LR", "CNR_SE", &[]);
    builder.extract_int_id(
        defs::tcls::CNR_NW,
        defs::bslots::INT,
        "UL",
        "CNR_NW",
        &[
            builder.bel_single(defs::bslots::STARTUP, "STARTUP"),
            builder.bel_single(defs::bslots::BSCAN, "BSCAN"),
        ],
    );
    builder.extract_int_id(defs::tcls::CNR_NE, defs::bslots::INT, "UR", "CNR_NE", &[]);

    for (tcid, naming, tkn) in [
        (defs::tcls::BRAM_W, "BRAM_W", "LBRAM"),
        (defs::tcls::BRAM_E, "BRAM_E", "RBRAM"),
        (defs::tcls::BRAM_M, "BRAM_M", "MBRAM"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut dxl = -1;
            let mut dxr = 1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&((xy.x - 1) as i32)) {
                dxl -= 1;
            }
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&((xy.x + 1) as i32)) {
                dxr += 1;
            }
            let mut coords = Vec::new();
            for dy in 0..4 {
                coords.push(xy.delta(0, dy));
            }
            for dy in 0..4 {
                coords.push(xy.delta(dxl, dy));
            }
            for dy in 0..4 {
                coords.push(xy.delta(dxr, dy));
            }
            let mut bels = vec![builder.bel_single(defs::bslots::BRAM, "BLOCKRAM")];
            if tkn != "MBRAM" {
                let mut bel = builder.bel_virtual(defs::bslots::CLKV_BRAM);
                for i in 0..4 {
                    bel = bel.extra_int_in(format!("IN{i}"), &[format!("BRAM_GCLKIN{i}")]);
                }
                for (i, l) in ['D', 'C', 'B', 'A'].into_iter().enumerate() {
                    for j in 0..4 {
                        bel = bel.extra_int_out(
                            format!("OUT_L{i}_{j}"),
                            &[
                                format!("LBRAM_GCLK_IOB{l}{j}"),
                                format!("RBRAM_GCLK_CLB{l}{j}"),
                            ],
                        );
                        bel = bel.extra_int_out(
                            format!("OUT_R{i}_{j}"),
                            &[
                                format!("LBRAM_GCLK_CLB{l}{j}"),
                                format!("RBRAM_GCLK_IOB{l}{j}"),
                            ],
                        );
                    }
                }
                bels.push(bel);
            }
            builder.extract_xtile_id(
                tcid,
                defs::bslots::INT,
                xy,
                &[],
                &coords,
                naming,
                &bels,
                &wires::GCLK[..],
            );
        }
    }

    let bram_bt_forbidden = Vec::from_iter(
        [
            wires::IMUX_DLL_CLKIN,
            wires::IMUX_DLL_CLKFB,
            wires::IMUX_DLL_RST,
        ]
        .into_iter()
        .chain(wires::GCLK),
    );
    for (tkn, tcid, naming) in [
        ("BRAM_BOT", defs::tcls::BRAM_S, "BRAM_S_BOT"),
        ("BRAM_BOT_GCLK", defs::tcls::BRAM_S, "BRAM_S_BOT"),
        ("LBRAM_BOTS_GCLK", defs::tcls::BRAM_S, "BRAM_S_BOT"),
        ("RBRAM_BOTS_GCLK", defs::tcls::BRAM_S, "BRAM_S_BOT"),
        ("LBRAM_BOTS", defs::tcls::BRAM_S, "BRAM_S_BOT"),
        ("RBRAM_BOTS", defs::tcls::BRAM_S, "BRAM_S_BOT"),
        ("BRAM_BOT_NOGCLK", defs::tcls::BRAM_S, "BRAM_S_BOTP"),
        ("BRAMS2E_BOT_NOGCLK", defs::tcls::BRAM_S, "BRAM_S_BOTP"),
        ("LBRAM_BOTP", defs::tcls::BRAM_S, "BRAM_S_BOTP"),
        ("RBRAM_BOTP", defs::tcls::BRAM_S, "BRAM_S_BOTP"),
        ("BRAM_TOP", defs::tcls::BRAM_N, "BRAM_N_TOP"),
        ("BRAM_TOP_GCLK", defs::tcls::BRAM_N, "BRAM_N_TOP"),
        ("LBRAM_TOPS_GCLK", defs::tcls::BRAM_N, "BRAM_N_TOP"),
        ("RBRAM_TOPS_GCLK", defs::tcls::BRAM_N, "BRAM_N_TOP"),
        ("LBRAM_TOPS", defs::tcls::BRAM_N, "BRAM_N_TOP"),
        ("RBRAM_TOPS", defs::tcls::BRAM_N, "BRAM_N_TOP"),
        ("BRAM_TOP_NOGCLK", defs::tcls::BRAM_N, "BRAM_N_TOPP"),
        ("BRAMS2E_TOP_NOGCLK", defs::tcls::BRAM_N, "BRAM_N_TOPP"),
        ("LBRAM_TOPP", defs::tcls::BRAM_N, "BRAM_N_TOPP"),
        ("RBRAM_TOPP", defs::tcls::BRAM_N, "BRAM_N_TOPP"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut dx = -1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&((xy.x - 1) as i32)) {
                dx -= 1;
            }
            let coords = [xy, xy.delta(dx, 0)];
            builder.extract_xtile_id(
                tcid,
                defs::bslots::INT,
                xy,
                &[],
                &coords,
                naming,
                &[],
                &bram_bt_forbidden,
            );
        }
    }

    let dll_forbidden = Vec::from_iter(wires::GCLK.into_iter().chain(wires::LV));
    for (tkn, tcid, mut naming, num_cells) in [
        ("BRAM_BOT", defs::tcls::DLL_S, "", 3),
        ("LBRAM_BOTS_GCLK", defs::tcls::DLLS_S, "DLLS_SW_GCLK", 3),
        ("RBRAM_BOTS_GCLK", defs::tcls::DLLS_S, "DLLS_SE_GCLK", 3),
        ("LBRAM_BOTS", defs::tcls::DLLS_S, "DLLS_SW", 3),
        ("RBRAM_BOTS", defs::tcls::DLLS_S, "DLLS_SE", 3),
        ("LBRAM_BOTP", defs::tcls::DLLP_S, "DLLP_SW", 4),
        ("RBRAM_BOTP", defs::tcls::DLLP_S, "DLLP_SE", 4),
        ("BRAM_TOP", defs::tcls::DLL_N, "", 3),
        ("LBRAM_TOPS_GCLK", defs::tcls::DLLS_N, "DLLS_NW_GCLK", 3),
        ("RBRAM_TOPS_GCLK", defs::tcls::DLLS_N, "DLLS_NE_GCLK", 3),
        ("LBRAM_TOPS", defs::tcls::DLLS_N, "DLLS_NW", 3),
        ("RBRAM_TOPS", defs::tcls::DLLS_N, "DLLS_NE", 3),
        ("LBRAM_TOPP", defs::tcls::DLLP_N, "DLLP_NW", 4),
        ("RBRAM_TOPP", defs::tcls::DLLP_N, "DLLP_NE", 4),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            if rd.family == "virtex" {
                naming = match tcid {
                    defs::tcls::DLL_S => {
                        if xy.x == 1 {
                            "DLL_SW"
                        } else {
                            "DLL_SE"
                        }
                    }
                    defs::tcls::DLL_N => {
                        if xy.x == 1 {
                            "DLL_NW"
                        } else {
                            "DLL_NE"
                        }
                    }
                    _ => unreachable!(),
                };
            }
            let mut dx = -1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&((xy.x - 1) as i32)) {
                dx -= 1;
            }
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(num_cells)
                .extract_muxes(defs::bslots::DLL_INT)
                .skip_muxes(&dll_forbidden)
                .ref_int(xy, 0)
                .ref_int(xy.delta(dx, 0), 1)
                .extract();
        }
    }
    for (naming, mode, bt, lr) in [
        ("DLL_SW", '_', 'B', 'L'),
        ("DLL_SE", '_', 'B', 'R'),
        ("DLL_NW", '_', 'T', 'L'),
        ("DLL_NE", '_', 'T', 'R'),
        ("DLLP_SW", 'P', 'B', 'L'),
        ("DLLP_SE", 'P', 'B', 'R'),
        ("DLLP_NW", 'P', 'T', 'L'),
        ("DLLP_NE", 'P', 'T', 'R'),
        ("DLLS_SW", 'S', 'B', 'L'),
        ("DLLS_SE", 'S', 'B', 'R'),
        ("DLLS_NW", 'S', 'T', 'L'),
        ("DLLS_NE", 'S', 'T', 'R'),
        ("DLLS_SW_GCLK", 'S', 'B', 'L'),
        ("DLLS_SE_GCLK", 'S', 'B', 'R'),
        ("DLLS_NW_GCLK", 'S', 'T', 'L'),
        ("DLLS_NE_GCLK", 'S', 'T', 'R'),
    ] {
        if let Some((_, naming)) = builder.ndb.tile_class_namings.get_mut(naming) {
            let xt = if mode == 'S' { "_1" } else { "" };
            let tile = RawTileId::from_idx(1);
            let wt_clkin = format!("CLK{bt}_CLKIN{lr}{xt}");
            let wt_clkfb = format!("CLK{bt}_CLKFB{lr}{xt}");
            for i in 0..2 {
                naming.ext_pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                        TileWireCoord::new_idx(2, wires::OUT_CLKPAD[i]),
                    ),
                    PipNaming {
                        tile,
                        wire_to: wt_clkin.clone(),
                        wire_from: format!("CLK{bt}_CLKPAD{i}"),
                    },
                );
                naming.ext_pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                        TileWireCoord::new_idx(2, wires::OUT_CLKPAD[i]),
                    ),
                    PipNaming {
                        tile,
                        wire_to: wt_clkfb.clone(),
                        wire_from: format!("CLK{bt}_CLKPAD{i}"),
                    },
                );
            }
            if mode != '_' {
                for i in 0..2 {
                    naming.ext_pips.insert(
                        (
                            TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                            TileWireCoord::new_idx(2, wires::OUT_IOFB[i]),
                        ),
                        PipNaming {
                            tile,
                            wire_to: wt_clkin.clone(),
                            wire_from: format!("CLK{bt}_IOFB{i}"),
                        },
                    );
                    naming.ext_pips.insert(
                        (
                            TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                            TileWireCoord::new_idx(2, wires::OUT_IOFB[i]),
                        ),
                        PipNaming {
                            tile,
                            wire_to: wt_clkfb.clone(),
                            wire_from: format!("CLK{bt}_IOFB{i}"),
                        },
                    );
                }
                if mode == 'P' {
                    naming.ext_pips.insert(
                        (
                            TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                            TileWireCoord::new_idx(3, wires::OUT_DLL_CLK2X),
                        ),
                        PipNaming {
                            tile,
                            wire_to: wt_clkin,
                            wire_from: format!("CLK{bt}_CLK2X{lr}_1"),
                        },
                    );
                } else {
                    naming.ext_pips.insert(
                        (
                            TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                            TileWireCoord::new_idx(0, wires::OUT_DLL_CLK2X),
                        ),
                        PipNaming {
                            tile,
                            wire_to: wt_clkfb,
                            wire_from: format!("CLK{bt}_CLK2X{lr}_1"),
                        },
                    );
                }
            }
            let pins = dll_pins
                .keys()
                .map(|k| {
                    let mut name = format!("CLK{bt}_{k}{lr}{xt}");
                    if bt == 'T' && lr == 'L' && mode != '_' && k == "RST" {
                        if mode == 'S' {
                            name = "CLKT_RSTL".to_string();
                        } else {
                            name = "CLKT_RSTL_1".to_string();
                        }
                    }
                    if bt == 'T' && lr == 'L' && mode == 'S' && k == "LOCKED" {
                        name = "CLKT_LOCK_TL_1".to_string();
                    }
                    (
                        k.clone(),
                        BelPinNaming {
                            tile: RawTileId::from_idx(1),
                            name: name.clone(),
                            name_far: name,
                            pips: Vec::new(),
                            int_pips: BTreeMap::new(),
                            is_intf: false,
                        },
                    )
                })
                .collect();
            naming.bels.insert(
                defs::bslots::DLL,
                BelNaming {
                    tiles: vec![RawTileId::from_idx(1)],
                    pins,
                },
            );
        }
    }
    for (tcid, mode) in [
        (defs::tcls::DLL_S, '_'),
        (defs::tcls::DLL_N, '_'),
        (defs::tcls::DLLP_S, 'P'),
        (defs::tcls::DLLP_N, 'P'),
        (defs::tcls::DLLS_S, 'S'),
        (defs::tcls::DLLS_N, 'S'),
    ] {
        let tcls = &mut builder.db.tile_classes[tcid];
        let Some(pips) = builder.pips.get_mut(&(tcid, defs::bslots::DLL_INT)) else {
            continue;
        };
        for i in 0..2 {
            pips.pips.insert(
                (
                    TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                    TileWireCoord::new_idx(2, wires::OUT_CLKPAD[i]),
                ),
                PipMode::Mux,
            );
            pips.pips.insert(
                (
                    TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                    TileWireCoord::new_idx(2, wires::OUT_CLKPAD[i]),
                ),
                PipMode::Mux,
            );
        }
        if mode != '_' {
            for i in 0..2 {
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                        TileWireCoord::new_idx(2, wires::OUT_IOFB[i]),
                    ),
                    PipMode::Mux,
                );
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                        TileWireCoord::new_idx(2, wires::OUT_IOFB[i]),
                    ),
                    PipMode::Mux,
                );
            }
            if mode == 'P' {
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                        TileWireCoord::new_idx(3, wires::OUT_DLL_CLK2X),
                    ),
                    PipMode::Mux,
                );
            } else {
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                        TileWireCoord::new_idx(0, wires::OUT_DLL_CLK2X),
                    ),
                    PipMode::Mux,
                );
            }
        }
        tcls.bels.insert(
            defs::bslots::DLL,
            BelInfo::Legacy(LegacyBel {
                pins: dll_pins.clone(),
            }),
        );
    }

    let forbidden: Vec<_> = [
        wires::IMUX_DLL_CLKIN,
        wires::IMUX_DLL_CLKFB,
        wires::IMUX_DLL_RST,
    ]
    .into_iter()
    .chain(wires::GCLK)
    .collect();
    for (tcid, naming, tkn) in [
        (defs::tcls::CLK_S_V, "CLK_S_V", "CLKB"),
        (defs::tcls::CLK_S_VE_4DLL, "CLK_S_VE_4DLL", "CLKB_4DLL"),
        (defs::tcls::CLK_S_VE_2DLL, "CLK_S_VE_2DLL", "CLKB_2DLL"),
        (defs::tcls::CLK_N_V, "CLK_N_V", "CLKT"),
        (defs::tcls::CLK_N_VE_4DLL, "CLK_N_VE_4DLL", "CLKT_4DLL"),
        (defs::tcls::CLK_N_VE_2DLL, "CLK_N_VE_2DLL", "CLKT_2DLL"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy = xy.delta(1, 0);
            let coords = if rd.family == "virtex" {
                vec![
                    int_xy,
                    Coord { x: 1, y: xy.y },
                    Coord {
                        x: rd.width - 2,
                        y: xy.y,
                    },
                ]
            } else {
                let botp: Vec<_> = find_columns(
                    rd,
                    &[
                        "LBRAM_BOTP",
                        "LBRAMS2E_BOTP",
                        "RBRAM_BOTP",
                        "RBRAMS2E_BOTP",
                        "BRAMS2E_BOT_NOGCLK",
                    ],
                )
                .into_iter()
                .collect();
                let bots: Vec<_> = find_columns(
                    rd,
                    &[
                        "LBRAM_BOTS",
                        "LBRAM_BOTS_GCLK",
                        "RBRAM_BOTS",
                        "RBRAM_BOTS_GCLK",
                    ],
                )
                .into_iter()
                .collect();
                assert_eq!(botp.len(), 2);
                assert_eq!(bots.len(), 2);
                vec![
                    int_xy,
                    Coord {
                        x: botp[0] as u16,
                        y: xy.y,
                    },
                    Coord {
                        x: botp[1] as u16,
                        y: xy.y,
                    },
                    Coord {
                        x: bots[0] as u16,
                        y: xy.y,
                    },
                    Coord {
                        x: bots[1] as u16,
                        y: xy.y,
                    },
                ]
            };
            let mut bels = vec![
                builder.bel_indexed(defs::bslots::GCLK_IO[0], "GCLKIOB", 0),
                builder.bel_indexed(defs::bslots::GCLK_IO[1], "GCLKIOB", 1),
                builder
                    .bel_indexed(defs::bslots::BUFG[0], "GCLK", 0)
                    .extra_wire("OUT.GLOBAL", &["CLKB_GCLK0", "CLKT_GCLK2"]),
                builder
                    .bel_indexed(defs::bslots::BUFG[1], "GCLK", 1)
                    .extra_wire("OUT.GLOBAL", &["CLKB_GCLK1", "CLKT_GCLK3"]),
            ];
            if rd.family != "virtex" {
                bels.push(
                    builder
                        .bel_virtual(defs::bslots::IOFB[0])
                        .extra_int_out("O", &["CLKB_IOFB0", "CLKT_IOFB0"]),
                );
                bels.push(
                    builder
                        .bel_virtual(defs::bslots::IOFB[1])
                        .extra_int_out("O", &["CLKB_IOFB1", "CLKT_IOFB1"]),
                );
            }
            builder.extract_xtile_id(
                tcid,
                defs::bslots::GCLK_INT,
                xy,
                &[],
                &coords,
                naming,
                &bels,
                &forbidden,
            );
        }
    }

    for (tcid, naming, tkn) in [
        (defs::tcls::PCI_W, "PCI_W", "CLKL"),
        (defs::tcls::PCI_E, "PCI_E", "CLKR"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            builder.extract_xtile_id(
                tcid,
                defs::bslots::PCI_INT,
                xy,
                &[],
                &[xy.delta(0, 1)],
                naming,
                &[builder
                    .bel_single(defs::bslots::PCILOGIC, "PCILOGIC")
                    .pin_name_only("IRDY", 1)
                    .pin_name_only("TRDY", 1)],
                &[wires::PCI_CE],
            );
        }
    }

    for &xy in rd.tiles_by_kind_name("CLKC") {
        builder.extract_xtile_bels_id(
            defs::tcls::CLKC,
            xy,
            &[],
            &[],
            "CLKC",
            &[
                builder
                    .bel_virtual(defs::bslots::CLKC)
                    .extra_wire("IN0", &["CLKC_GCLK0"])
                    .extra_wire("IN1", &["CLKC_GCLK1"])
                    .extra_wire("IN2", &["CLKC_GCLK2"])
                    .extra_wire("IN3", &["CLKC_GCLK3"])
                    .extra_wire("OUT0", &["CLKC_HGCLK0"])
                    .extra_wire("OUT1", &["CLKC_HGCLK1"])
                    .extra_wire("OUT2", &["CLKC_HGCLK2"])
                    .extra_wire("OUT3", &["CLKC_HGCLK3"]),
                builder
                    .bel_virtual(defs::bslots::GCLKC)
                    .extra_wire("IN0", &["CLKC_HGCLK0"])
                    .extra_wire("IN1", &["CLKC_HGCLK1"])
                    .extra_wire("IN2", &["CLKC_HGCLK2"])
                    .extra_wire("IN3", &["CLKC_HGCLK3"])
                    .extra_wire("OUT0", &["CLKC_VGCLK0"])
                    .extra_wire("OUT1", &["CLKC_VGCLK1"])
                    .extra_wire("OUT2", &["CLKC_VGCLK2"])
                    .extra_wire("OUT3", &["CLKC_VGCLK3"]),
            ],
            false,
        );
    }

    for &xy in rd.tiles_by_kind_name("GCLKC") {
        builder.extract_xtile_bels_id(
            defs::tcls::GCLKC,
            xy,
            &[],
            &[],
            "GCLKC",
            &[builder
                .bel_virtual(defs::bslots::GCLKC)
                .extra_wire_force("IN0", "GCLKC_HGCLK0")
                .extra_wire_force("IN1", "GCLKC_HGCLK1")
                .extra_wire_force("IN2", "GCLKC_HGCLK2")
                .extra_wire_force("IN3", "GCLKC_HGCLK3")
                .extra_wire_force("OUT0", "GCLKC_VGCLK0")
                .extra_wire_force("OUT1", "GCLKC_VGCLK1")
                .extra_wire_force("OUT2", "GCLKC_VGCLK2")
                .extra_wire_force("OUT3", "GCLKC_VGCLK3")],
            false,
        );
    }

    for &xy in rd.tiles_by_kind_name("BRAM_CLKH") {
        builder.extract_xtile_bels_id(
            defs::tcls::BRAM_CLKH,
            xy,
            &[],
            &[xy],
            "BRAM_CLKH",
            &[builder
                .bel_virtual(defs::bslots::BRAM_CLKH)
                .extra_wire_force("IN0", "BRAM_CLKH_GCLK0")
                .extra_wire_force("IN1", "BRAM_CLKH_GCLK1")
                .extra_wire_force("IN2", "BRAM_CLKH_GCLK2")
                .extra_wire_force("IN3", "BRAM_CLKH_GCLK3")
                .extra_int_out_force(
                    "OUT0",
                    TileWireCoord::new_idx(0, wires::GCLK[0]),
                    "BRAM_CLKH_VGCLK0",
                )
                .extra_int_out_force(
                    "OUT1",
                    TileWireCoord::new_idx(0, wires::GCLK[1]),
                    "BRAM_CLKH_VGCLK1",
                )
                .extra_int_out_force(
                    "OUT2",
                    TileWireCoord::new_idx(0, wires::GCLK[2]),
                    "BRAM_CLKH_VGCLK2",
                )
                .extra_int_out_force(
                    "OUT3",
                    TileWireCoord::new_idx(0, wires::GCLK[3]),
                    "BRAM_CLKH_VGCLK3",
                )],
            false,
        );
    }

    for (tkn, tcid, naming) in [
        ("CLKV", defs::tcls::CLKV_CLKV, "CLKV_CLKV"),
        ("CLKB", defs::tcls::CLKV_NULL, "CLKV_CLKB"),
        ("CLKB_4DLL", defs::tcls::CLKV_NULL, "CLKV_CLKB"),
        ("CLKB_2DLL", defs::tcls::CLKV_NULL, "CLKV_CLKB"),
        ("CLKT", defs::tcls::CLKV_NULL, "CLKV_CLKT"),
        ("CLKT_4DLL", defs::tcls::CLKV_NULL, "CLKV_CLKT"),
        ("CLKT_2DLL", defs::tcls::CLKV_NULL, "CLKV_CLKT"),
        ("GCLKV", defs::tcls::CLKV_GCLKV, "CLKV_GCLKV"),
        ("GCLKB", defs::tcls::CLKV_NULL, "CLKV_GCLKB"),
        ("GCLKT", defs::tcls::CLKV_NULL, "CLKV_GCLKT"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy_l = builder.walk_to_int(xy, Dir::W, false).unwrap();
            let int_xy_r = builder.walk_to_int(xy, Dir::E, false).unwrap();
            let mut bel = builder.bel_virtual(defs::bslots::CLKV);
            for i in 0..4 {
                bel = bel.extra_int_out(
                    format!("OUT_L{i}"),
                    &[
                        format!("GCLKV_BUFL{i}"),
                        format!("CLKV_GCLK_BUFL{i}"),
                        format!("GCLKB_GCLKW{i}"),
                        format!("GCLKT_GCLKW{i}"),
                        format!("CLKB_HGCLK_W{i}"),
                        format!("CLKT_HGCLK_W{i}"),
                    ],
                );
                bel = bel.extra_int_out(
                    format!("OUT_R{i}"),
                    &[
                        format!("GCLKV_BUFR{i}"),
                        format!("CLKV_GCLK_BUFR{i}"),
                        format!("GCLKB_GCLKE{i}"),
                        format!("GCLKT_GCLKE{i}"),
                        format!("CLKB_HGCLK_E{i}"),
                        format!("CLKT_HGCLK_E{i}"),
                    ],
                );
                bel = bel.extra_wire(
                    format!("IN{i}"),
                    &[
                        format!("GCLKV_GCLK_B{i}"),
                        format!("CLKV_VGCLK{i}"),
                        format!("GCLKB_VGCLK{i}"),
                        format!("GCLKT_VGCLK{i}"),
                        format!("CLKB_VGCLK{i}"),
                        format!("CLKT_VGCLK{i}"),
                    ],
                );
            }
            builder.extract_xtile_bels_id(
                tcid,
                xy,
                &[],
                &[int_xy_l, int_xy_r],
                naming,
                &[bel],
                false,
            );
        }
    }

    for (tkn, tcid, naming, slot) in [
        (
            "BRAM_BOT",
            defs::tcls::CLKV_BRAM_S,
            "CLKV_BRAM_S",
            defs::bslots::CLKV_BRAM_S,
        ),
        (
            "BRAM_BOT_GCLK",
            defs::tcls::CLKV_BRAM_S,
            "CLKV_BRAM_S",
            defs::bslots::CLKV_BRAM_S,
        ),
        (
            "LBRAM_BOTS_GCLK",
            defs::tcls::CLKV_BRAM_S,
            "CLKV_BRAM_S",
            defs::bslots::CLKV_BRAM_S,
        ),
        (
            "RBRAM_BOTS_GCLK",
            defs::tcls::CLKV_BRAM_S,
            "CLKV_BRAM_S",
            defs::bslots::CLKV_BRAM_S,
        ),
        (
            "BRAM_TOP",
            defs::tcls::CLKV_BRAM_N,
            "CLKV_BRAM_N",
            defs::bslots::CLKV_BRAM_N,
        ),
        (
            "BRAM_TOP_GCLK",
            defs::tcls::CLKV_BRAM_N,
            "CLKV_BRAM_N",
            defs::bslots::CLKV_BRAM_N,
        ),
        (
            "LBRAM_TOPS_GCLK",
            defs::tcls::CLKV_BRAM_N,
            "CLKV_BRAM_N",
            defs::bslots::CLKV_BRAM_N,
        ),
        (
            "RBRAM_TOPS_GCLK",
            defs::tcls::CLKV_BRAM_N,
            "CLKV_BRAM_N",
            defs::bslots::CLKV_BRAM_N,
        ),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy_l = builder.walk_to_int(xy, Dir::W, false).unwrap();
            let mut bel = builder.bel_virtual(slot);
            for i in 0..4 {
                bel = bel.extra_int_out(
                    format!("OUT_L{i}"),
                    &[format!("BRAM_BOT_GCLKW{i}"), format!("BRAM_TOP_GCLKW{i}")],
                );
                bel = bel.extra_int_out(
                    format!("OUT_R{i}"),
                    &[format!("BRAM_BOT_GCLKE{i}"), format!("BRAM_TOP_GCLKE{i}")],
                );
                bel = bel.extra_int_in(
                    format!("IN{i}"),
                    &[format!("BRAM_BOT_VGCLK{i}"), format!("BRAM_TOP_VGCLK{i}")],
                );
            }
            let bram_xy = xy; // dummy position
            builder.extract_xtile_bels_id(
                tcid,
                xy,
                &[],
                &[xy, int_xy_l, bram_xy],
                naming,
                &[bel],
                false,
            );
        }
    }

    for pips in builder.pips.values_mut() {
        for (&(wt, _wf), mode) in &mut pips.pips {
            let wtn = builder.db.wires.key(wt.wire);
            if wtn.starts_with("SINGLE") && *mode != PipMode::PermaBuf {
                *mode = PipMode::Pass;
            }
        }
    }

    builder.build()
}
