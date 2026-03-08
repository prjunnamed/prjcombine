use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelPin, IntDb, LegacyBel, TileWireCoord, WireSlotIdExt},
    dir::{Dir, DirMap},
};
use prjcombine_re_xilinx_naming::db::{BelNaming, BelPinNaming, NamingDb, PipNaming, RawTileId};
use prjcombine_re_xilinx_rawdump::{Coord, Part};
use prjcombine_virtex::defs::{self, bcls::IOI, bslots, tcls, wire_to_mux, wires};
use std::collections::BTreeMap;

use prjcombine_re_xilinx_rd2db_grid::find_columns;
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, PipMode};

pub fn make_int_db(rd: &Part) -> (IntDb, NamingDb) {
    let is_s2 = rd.family == "virtex" && rd.part.contains("2s");
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
                format!("BRAM_GCLKIN{i}"),
                format!("CLKB_VGCLK{i}"),
                format!("CLKT_VGCLK{i}"),
                format!("BRAM_BOT_VGCLK{i}"),
                format!("BRAM_TOP_VGCLK{i}"),
                format!("GCLKV_GCLK_B{i}"),
                format!("CLKV_VGCLK{i}"),
                format!("GCLKB_VGCLK{i}"),
                format!("GCLKT_VGCLK{i}"),
                format!("CLKB_GCLK{i}"),
                format!("CLKT_GCLK{i}"),
            ],
        );
        builder.wire_names(
            wires::GCLK_LEAF[i],
            &[
                format!("GCLK{i}"),
                format!("LEFT_GCLK{i}"),
                format!("RIGHT_GCLK{i}"),
                format!("BOT_HGCLK{i}"),
                format!("TOP_HGCLK{i}"),
                format!("LL_GCLK{i}"),
                format!("UL_GCLK{i}"),
                format!("BRAM_BOT_GCLKE{i}"),
                format!("BRAM_TOP_GCLKE{i}"),
                format!("BRAM_BOTP_GCLK{i}"),
                format!("BRAM_TOPP_GCLK{i}"),
                format!("BRAM_BOTS_GCLK{i}"),
                format!("BRAM_TOPS_GCLK{i}"),
            ],
        );
        builder.extra_name_sub(format!("MBRAM_GCLKD{i}"), 0, wires::GCLK_LEAF[i]);
        builder.extra_name_sub(format!("MBRAM_GCLKA{i}"), 3, wires::GCLK_LEAF[i]);
        builder.wire_names(
            wires::GCLK_BUF[i],
            &[format!("BOT_GCLK{i}"), format!("TOP_GCLK{i}")],
        );
        builder.mark_permabuf(wires::GCLK_BUF[i]);
        builder.mark_permabuf(wires::GCLK[i]);
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
    builder.alt_name("TOP_FAKE_LH0", wires::LH[0]);
    builder.alt_name("TOP_FAKE_LH6", wires::LH[6]);
    builder.alt_name("BOT_FAKE_LH0", wires::LH[0]);
    builder.alt_name("BOT_FAKE_LH6", wires::LH[6]);

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
        let w = wires::IMUX_BUFGCE_CLK[i];
        builder.extra_name_sub(format!("CLKB_GCLKBUF{i}_IN"), 1, w);
        builder.extra_name_sub(format!("CLKT_GCLKBUF{ii}_IN", ii = i + 2), 1, w);
    }
    for i in 0..2 {
        let w = wires::IMUX_BUFGCE_CE[i];
        builder.extra_name_sub(format!("CLKB_CE{i}"), 1, w);
        builder.extra_name_sub(format!("CLKT_CE{i}"), 1, w);
    }
    for i in 0..2 {
        let w = wires::OUT_BUFGCE_O[i];
        builder.extra_name_sub(format!("CLKB_GCLK{i}_PW"), 1, w);
        builder.extra_name_sub(format!("CLKT_GCLK{ii}_PW", ii = i + 2), 1, w);
    }
    for i in 0..2 {
        let w = wires::OUT_CLKPAD[i];
        builder.extra_name_sub(format!("CLKB_CLKPAD{i}"), 1, w);
        builder.extra_name_sub(format!("CLKT_CLKPAD{i}"), 1, w);
    }
    for i in 0..2 {
        let w = wires::OUT_IOFB[i];
        builder.extra_name_sub(format!("CLKB_IOFB{i}"), 1, w);
        builder.extra_name_sub(format!("CLKT_IOFB{i}"), 1, w);
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
        builder.extra_name_sub(format!("CLKB_{name}L"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 4, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 5, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 2, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 3, w);
        builder.extra_name_sub(format!("CLKT_{name}L_1"), 4, w);
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 5, w);
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
        builder.extra_name_sub(format!("CLKB_{name}L"), 2, w);
        builder.extra_name_sub(format!("CLKB_{name}R"), 3, w);
        builder.extra_name_sub(format!("CLKB_{name}L_1"), 4, w);
        builder.extra_name_sub(format!("CLKB_{name}R_1"), 5, w);
        builder.extra_name_sub(format!("CLKT_{name}L"), 2, w);
        builder.extra_name_sub(format!("CLKT_{name}R"), 3, w);
        if name == "LOCKED" {
            builder.extra_name_sub("CLKT_LOCK_TL_1", 4, w);
        } else {
            builder.extra_name_sub(format!("CLKT_{name}L_1"), 4, w);
        }
        builder.extra_name_sub(format!("CLKT_{name}R_1"), 5, w);
        dll_pins.insert(
            name.to_string(),
            BelPin::new_out(TileWireCoord::new_idx(0, w)),
        );
    }

    let slice_name_only = ["F5IN", "F5", "CIN", "COUT"];

    builder.extract_int_id(
        tcls::CLB,
        bslots::INT,
        "CENTER",
        "CLB",
        &[
            builder
                .bel_indexed(bslots::SLICE[0], "SLICE", 0)
                .pins_name_only(&slice_name_only)
                .pin_name_only("COUT", 1),
            builder
                .bel_indexed(bslots::SLICE[1], "SLICE", 1)
                .pins_name_only(&slice_name_only)
                .pin_name_only("COUT", 1),
            builder
                .bel_indexed(bslots::TBUF[0], "TBUF", 0)
                .pins_name_only(&["O"]),
            builder
                .bel_indexed(bslots::TBUF[1], "TBUF", 1)
                .pins_name_only(&["O"]),
            builder
                .bel_virtual(bslots::TBUS)
                .extra_wire("BUS0", &["TBUF0"])
                .extra_wire("BUS1", &["TBUF1"])
                .extra_wire("BUS2", &["TBUF2"])
                .extra_wire("BUS3", &["TBUF3"])
                .extra_wire("BUS3_E", &["TBUF_STUB3"])
                .extra_int_out("OUT", &["TBUFO"]),
        ],
    );

    let pips = builder.pips.get_mut(&(tcls::CLB, bslots::INT)).unwrap();
    for w in [
        wires::IMUX_CLB_CLK,
        wires::IMUX_CLB_CE,
        wires::IMUX_CLB_SR,
        wires::IMUX_CLB_BX,
        wires::IMUX_CLB_BY,
        wires::IMUX_TBUF_T,
    ] {
        for w in w {
            pips.pips
                .insert((w.cell(0), wires::PULLUP.cell(0).pos()), PipMode::Mux);
        }
    }

    let bels_left = [
        builder
            .bel_indexed(bslots::IOI[0], "IOB", 0)
            .pin_rename("CLK", "ICLK"),
        builder
            .bel_indexed(bslots::IOI[1], "IOB", 1)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("PCI", "LEFT_PCI_BOT_PCI1"),
        builder
            .bel_indexed(bslots::IOI[2], "IOB", 2)
            .pin_rename("CLK", "ICLK"),
        builder
            .bel_indexed(bslots::IOI[3], "IOB", 3)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("PCI", "LEFT_PCI_TOP_PCI3"),
        builder
            .bel_indexed(bslots::TBUF[0], "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed(bslots::TBUF[1], "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual(bslots::TBUS_WE)
            .extra_int_out("BUS0", &["LEFT_TBUFO2"])
            .extra_int_out("BUS1", &["LEFT_TBUFO3"])
            .extra_int_out("BUS2", &["LEFT_TBUFO0"])
            .extra_int_out("BUS3", &["LEFT_TBUFO1"])
            .extra_wire("BUS3_E", &["LEFT_TBUF1_STUB"]),
    ];
    builder.extract_int_id(tcls::IO_W, bslots::INT, "LEFT", "IO_W", &bels_left);
    builder.extract_int_id(tcls::IO_W, bslots::INT, "LEFT_PCI_BOT", "IO_W", &bels_left);
    builder.extract_int_id(tcls::IO_W, bslots::INT, "LEFT_PCI_TOP", "IO_W", &bels_left);

    let bels_right = [
        builder
            .bel_indexed(bslots::IOI[0], "IOB", 0)
            .pin_rename("CLK", "ICLK"),
        builder
            .bel_indexed(bslots::IOI[1], "IOB", 1)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("PCI", "RIGHT_PCI_BOT_PCI1"),
        builder
            .bel_indexed(bslots::IOI[2], "IOB", 2)
            .pin_rename("CLK", "ICLK"),
        builder
            .bel_indexed(bslots::IOI[3], "IOB", 3)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("PCI", "RIGHT_PCI_TOP_PCI3"),
        builder
            .bel_indexed(bslots::TBUF[0], "TBUF", 0)
            .pins_name_only(&["O"]),
        builder
            .bel_indexed(bslots::TBUF[1], "TBUF", 1)
            .pins_name_only(&["O"]),
        builder
            .bel_virtual(bslots::TBUS_WE)
            .extra_int_out("BUS0", &["RIGHT_TBUFO2"])
            .extra_int_out("BUS1", &["RIGHT_TBUFO3"])
            .extra_int_out("BUS2", &["RIGHT_TBUFO0"])
            .extra_int_out("BUS3", &["RIGHT_TBUFO1"]),
    ];
    builder.extract_int_id(tcls::IO_E, bslots::INT, "RIGHT", "IO_E", &bels_right);
    builder.extract_int_id(
        tcls::IO_E,
        bslots::INT,
        "RIGHT_PCI_BOT",
        "IO_E",
        &bels_right,
    );
    builder.extract_int_id(
        tcls::IO_E,
        bslots::INT,
        "RIGHT_PCI_TOP",
        "IO_E",
        &bels_right,
    );

    let bels_bot = [
        builder
            .bel_indexed(bslots::IOI[0], "IOB", 0)
            .pin_rename("CLK", "ICLK"),
        builder
            .bel_indexed(bslots::IOI[1], "IOB", 1)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("DLLFB", "BL_DLLIOB_IOFB"),
        builder
            .bel_indexed(bslots::IOI[2], "IOB", 2)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("DLLFB", "BR_DLLIOB_IOFB"),
        builder
            .bel_indexed(bslots::IOI[3], "IOB", 3)
            .pin_rename("CLK", "ICLK"),
    ];
    builder.extract_int_id(tcls::IO_S, bslots::INT, "BOT", "IO_S", &bels_bot);
    builder.extract_int_id(tcls::IO_S, bslots::INT, "BL_DLLIOB", "IO_S", &bels_bot);
    builder.extract_int_id(tcls::IO_S, bslots::INT, "BR_DLLIOB", "IO_S", &bels_bot);

    let bels_top = [
        builder
            .bel_indexed(bslots::IOI[0], "IOB", 0)
            .pin_rename("CLK", "ICLK"),
        builder
            .bel_indexed(bslots::IOI[1], "IOB", 1)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("DLLFB", "TL_DLLIOB_IOFB"),
        builder
            .bel_indexed(bslots::IOI[2], "IOB", 2)
            .pin_rename("CLK", "ICLK")
            .extra_wire_force("DLLFB", "TR_DLLIOB_IOFB"),
        builder
            .bel_indexed(bslots::IOI[3], "IOB", 3)
            .pin_rename("CLK", "ICLK"),
    ];
    builder.extract_int_id(tcls::IO_N, bslots::INT, "TOP", "IO_N", &bels_top);
    builder.extract_int_id(tcls::IO_N, bslots::INT, "TL_DLLIOB", "IO_N", &bels_top);
    builder.extract_int_id(tcls::IO_N, bslots::INT, "TR_DLLIOB", "IO_N", &bels_top);

    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        let pips = builder.pips.get_mut(&(tcid, bslots::INT)).unwrap();
        for w in [
            wires::IMUX_IO_CLK,
            wires::IMUX_IO_ICE,
            wires::IMUX_IO_OCE,
            wires::IMUX_IO_TCE,
            wires::IMUX_IO_SR,
            wires::IMUX_IO_O,
            wires::IMUX_IO_T,
        ] {
            for w in w {
                pips.pips
                    .insert((w.cell(0), wires::PULLUP.cell(0).pos()), PipMode::Mux);
            }
        }
        let is_we = matches!(tcid, tcls::IO_W | tcls::IO_E);
        pips.pips.retain(|(_wt, wf), _| {
            if wf.wire == wires::OUT_IO_I[0] || wf.wire == wires::OUT_IO_IQ[0] {
                false
            } else if !is_we && (wf.wire == wires::OUT_IO_I[3] || wf.wire == wires::OUT_IO_IQ[3]) {
                // nope
                false
            } else {
                // okay
                true
            }
        });
        let tcls = &mut builder.db.tile_classes[tcid];
        for bslot in bslots::IOI {
            if let Some(BelInfo::Bel(bel)) = tcls.bels.get_mut(bslot) {
                let clk = bel.inputs[IOI::ICLK];
                bel.inputs.insert(IOI::OCLK, clk);
                bel.inputs.insert(IOI::TCLK, clk);
            }
        }
    }
    for naming in ["IO_W", "IO_E", "IO_S", "IO_N"] {
        let naming = builder.ndb.tile_class_namings.get_mut(naming).unwrap().1;
        for bslot in bslots::IOI {
            if let Some(bn) = naming.bels.get_mut(bslot) {
                let clk = bn.pins["ICLK"].clone();
                bn.pins.insert("OCLK".into(), clk.clone());
                bn.pins.insert("TCLK".into(), clk);
            }
        }
    }

    for tcid in [tcls::IO_W, tcls::IO_E] {
        let pips = builder.pips.get_mut(&(tcid, bslots::INT)).unwrap();
        for w in [wires::IMUX_TBUF_T, wires::IMUX_TBUF_I] {
            for w in w {
                pips.pips
                    .insert((w.cell(0), wires::PULLUP.cell(0).pos()), PipMode::Mux);
            }
        }
    }

    let (cnr_sw, cnr_nw, clkv_bram_s, clkv_bram_n, bram_w, bram_e) = if is_s2 {
        (
            tcls::CNR_SW_S2,
            tcls::CNR_NW_S2,
            tcls::CLKV_BRAM_S_S2,
            tcls::CLKV_BRAM_N_S2,
            tcls::BRAM_W_S2,
            tcls::BRAM_E_S2,
        )
    } else {
        (
            tcls::CNR_SW,
            tcls::CNR_NW,
            tcls::CLKV_BRAM_S,
            tcls::CLKV_BRAM_N,
            tcls::BRAM_W,
            tcls::BRAM_E,
        )
    };

    builder.extract_int_id(
        cnr_sw,
        bslots::INT,
        "LL",
        "CNR_SW",
        &[
            builder.bel_single(bslots::CAPTURE, "CAPTURE"),
            builder.bel_virtual(bslots::MISC_SW),
        ],
    );
    builder.extract_int_id(
        tcls::CNR_SE,
        bslots::INT,
        "LR",
        "CNR_SE",
        &[builder.bel_virtual(bslots::MISC_SE)],
    );
    builder.extract_int_id(
        cnr_nw,
        bslots::INT,
        "UL",
        "CNR_NW",
        &[
            builder.bel_single(bslots::STARTUP, "STARTUP"),
            builder.bel_single(bslots::BSCAN, "BSCAN"),
            builder.bel_virtual(bslots::MISC_NW),
        ],
    );
    builder.extract_int_id(
        tcls::CNR_NE,
        bslots::INT,
        "UR",
        "CNR_NE",
        &[builder.bel_virtual(bslots::MISC_NE)],
    );

    for (tcid, w) in [
        (cnr_sw, wires::IMUX_CAP_CLK),
        (cnr_sw, wires::IMUX_CAP_CAP),
        (cnr_nw, wires::IMUX_STARTUP_CLK),
        (cnr_nw, wires::IMUX_STARTUP_GWE),
        (cnr_nw, wires::IMUX_STARTUP_GTS),
        (cnr_nw, wires::IMUX_STARTUP_GSR),
        (cnr_nw, wires::IMUX_BSCAN_TDO1),
        (cnr_nw, wires::IMUX_BSCAN_TDO2),
    ] {
        let pips = builder.pips.get_mut(&(tcid, bslots::INT)).unwrap();
        pips.pips
            .insert((w.cell(0), wires::PULLUP.cell(0).pos()), PipMode::Mux);
    }

    for (tcid, naming, tkn) in [
        (bram_w, "BRAM_W", "LBRAM"),
        (bram_e, "BRAM_E", "RBRAM"),
        (tcls::BRAM_M, "BRAM_M", "MBRAM"),
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
            let bel = builder.bel_single(defs::bslots::BRAM, "BLOCKRAM");
            let mut x = builder
                .xtile_id(tcid, naming, xy)
                .num_cells(4)
                .extract_muxes(bslots::INT)
                .bel(bel);
            for (i, &xy) in coords.iter().enumerate() {
                x = x.ref_int(xy, i);
            }
            x.extract();
        }

        if let Some(pips) = builder.pips.get_mut(&(tcid, bslots::INT)) {
            for w in [
                wires::IMUX_BRAM_SELA,
                wires::IMUX_BRAM_SELB,
                wires::IMUX_BRAM_RSTA,
                wires::IMUX_BRAM_RSTB,
                wires::IMUX_BRAM_WEA,
                wires::IMUX_BRAM_WEB,
            ] {
                pips.pips
                    .insert((w.cell(0), wires::PULLUP.cell(0).pos()), PipMode::Mux);
            }
        }
    }

    let bram_bt_forbidden = Vec::from_iter(
        [
            wires::IMUX_DLL_CLKIN,
            wires::IMUX_DLL_CLKFB,
            wires::IMUX_DLL_RST,
        ]
        .into_iter()
        .chain(wires::GCLK_LEAF),
    );
    for (tkn, tcid, naming) in [
        ("BRAM_BOT", tcls::BRAM_S, "BRAM_S_BOT"),
        ("BRAM_BOT_GCLK", tcls::BRAM_S, "BRAM_S_BOT"),
        ("LBRAM_BOTS_GCLK", tcls::BRAM_S, "BRAM_S_BOT"),
        ("RBRAM_BOTS_GCLK", tcls::BRAM_S, "BRAM_S_BOT"),
        ("LBRAM_BOTS", tcls::BRAM_S, "BRAM_S_BOT"),
        ("RBRAM_BOTS", tcls::BRAM_S, "BRAM_S_BOT"),
        ("BRAM_BOT_NOGCLK", tcls::BRAM_S, "BRAM_S_BOTP"),
        ("BRAMS2E_BOT_NOGCLK", tcls::BRAM_S, "BRAM_S_BOTP"),
        ("LBRAM_BOTP", tcls::BRAM_S, "BRAM_S_BOTP"),
        ("RBRAM_BOTP", tcls::BRAM_S, "BRAM_S_BOTP"),
        ("BRAM_TOP", tcls::BRAM_N, "BRAM_N_TOP"),
        ("BRAM_TOP_GCLK", tcls::BRAM_N, "BRAM_N_TOP"),
        ("LBRAM_TOPS_GCLK", tcls::BRAM_N, "BRAM_N_TOP"),
        ("RBRAM_TOPS_GCLK", tcls::BRAM_N, "BRAM_N_TOP"),
        ("LBRAM_TOPS", tcls::BRAM_N, "BRAM_N_TOP"),
        ("RBRAM_TOPS", tcls::BRAM_N, "BRAM_N_TOP"),
        ("BRAM_TOP_NOGCLK", tcls::BRAM_N, "BRAM_N_TOPP"),
        ("BRAMS2E_TOP_NOGCLK", tcls::BRAM_N, "BRAM_N_TOPP"),
        ("LBRAM_TOPP", tcls::BRAM_N, "BRAM_N_TOPP"),
        ("RBRAM_TOPP", tcls::BRAM_N, "BRAM_N_TOPP"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let mut dx = -1;
            if find_columns(rd, &["GCLKV", "GBRKV"]).contains(&((xy.x - 1) as i32)) {
                dx -= 1;
            }
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(2)
                .extract_muxes(bslots::INT)
                .skip_muxes(&bram_bt_forbidden)
                .ref_int(xy, 0)
                .ref_int(xy.delta(dx, 0), 1)
                .extract();
        }
    }

    let dll_forbidden = Vec::from_iter(wires::GCLK_LEAF.into_iter().chain(wires::LV));
    for (tkn, tcid, mut naming, num_cells) in [
        ("BRAM_BOT", tcls::DLL_S, "", 3),
        ("LBRAM_BOTS_GCLK", tcls::DLLS_S, "DLLS_SW_GCLK", 3),
        ("RBRAM_BOTS_GCLK", tcls::DLLS_S, "DLLS_SE_GCLK", 3),
        ("LBRAM_BOTS", tcls::DLLS_S, "DLLS_SW", 3),
        ("RBRAM_BOTS", tcls::DLLS_S, "DLLS_SE", 3),
        ("LBRAM_BOTP", tcls::DLLP_S, "DLLP_SW", 4),
        ("RBRAM_BOTP", tcls::DLLP_S, "DLLP_SE", 4),
        ("BRAM_TOP", tcls::DLL_N, "", 3),
        ("LBRAM_TOPS_GCLK", tcls::DLLS_N, "DLLS_NW_GCLK", 3),
        ("RBRAM_TOPS_GCLK", tcls::DLLS_N, "DLLS_NE_GCLK", 3),
        ("LBRAM_TOPS", tcls::DLLS_N, "DLLS_NW", 3),
        ("RBRAM_TOPS", tcls::DLLS_N, "DLLS_NE", 3),
        ("LBRAM_TOPP", tcls::DLLP_N, "DLLP_NW", 4),
        ("RBRAM_TOPP", tcls::DLLP_N, "DLLP_NE", 4),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            if rd.family == "virtex" {
                naming = match tcid {
                    tcls::DLL_S => {
                        if xy.x == 1 {
                            "DLL_SW"
                        } else {
                            "DLL_SE"
                        }
                    }
                    tcls::DLL_N => {
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
                .extract_muxes(bslots::DLL_INT)
                .skip_muxes(&dll_forbidden)
                .ref_int(xy, 0)
                .ref_int(xy.delta(dx, 0), 1)
                .extract();
        }

        if let Some(pips) = builder.pips.get_mut(&(tcid, bslots::DLL_INT)) {
            pips.pips.insert(
                (wires::IMUX_DLL_RST.cell(0), wires::PULLUP.cell(0).pos()),
                PipMode::Mux,
            );
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
                bslots::DLL,
                BelNaming {
                    tiles: vec![RawTileId::from_idx(1)],
                    pins,
                },
            );
        }
    }
    for (tcid, mode) in [
        (tcls::DLL_S, '_'),
        (tcls::DLL_N, '_'),
        (tcls::DLLP_S, 'P'),
        (tcls::DLLP_N, 'P'),
        (tcls::DLLS_S, 'S'),
        (tcls::DLLS_N, 'S'),
    ] {
        let Some(pips) = builder.pips.get_mut(&(tcid, bslots::DLL_INT)) else {
            continue;
        };
        for i in 0..2 {
            pips.pips.insert(
                (
                    TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                    TileWireCoord::new_idx(2, wires::OUT_CLKPAD[i]).pos(),
                ),
                PipMode::Mux,
            );
            pips.pips.insert(
                (
                    TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                    TileWireCoord::new_idx(2, wires::OUT_CLKPAD[i]).pos(),
                ),
                PipMode::Mux,
            );
        }
        if mode != '_' {
            for i in 0..2 {
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                        TileWireCoord::new_idx(2, wires::OUT_IOFB[i]).pos(),
                    ),
                    PipMode::Mux,
                );
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                        TileWireCoord::new_idx(2, wires::OUT_IOFB[i]).pos(),
                    ),
                    PipMode::Mux,
                );
            }
            if mode == 'P' {
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKIN),
                        TileWireCoord::new_idx(3, wires::OUT_DLL_CLK2X).pos(),
                    ),
                    PipMode::Mux,
                );
            } else {
                pips.pips.insert(
                    (
                        TileWireCoord::new_idx(0, wires::IMUX_DLL_CLKFB),
                        TileWireCoord::new_idx(0, wires::OUT_DLL_CLK2X).pos(),
                    ),
                    PipMode::Mux,
                );
            }
        }
        builder.insert_tcls_bel(
            tcid,
            bslots::DLL,
            BelInfo::Legacy(LegacyBel {
                pins: dll_pins.clone(),
            }),
        );
    }

    let forbidden = [
        wires::IMUX_DLL_CLKIN,
        wires::IMUX_DLL_CLKFB,
        wires::IMUX_DLL_RST,
    ];
    for (tcid, naming, tkn) in [
        (tcls::CLK_S_V, "CLK_S_V", "CLKB"),
        (tcls::CLK_S_VE_4DLL, "CLK_S_VE_4DLL", "CLKB_4DLL"),
        (tcls::CLK_S_VE_2DLL, "CLK_S_VE_2DLL", "CLKB_2DLL"),
        (tcls::CLK_N_V, "CLK_N_V", "CLKT"),
        (tcls::CLK_N_VE_4DLL, "CLK_N_VE_4DLL", "CLKT_4DLL"),
        (tcls::CLK_N_VE_2DLL, "CLK_N_VE_2DLL", "CLKT_2DLL"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let coords = if rd.family == "virtex" {
                vec![
                    xy.delta(-1, 0),
                    xy.delta(1, 0),
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
                    xy.delta(-1, 0),
                    xy.delta(1, 0),
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
                builder
                    .bel_indexed(defs::bslots::GCLK_IOB[0], "GCLKIOB", 0)
                    .pin_rename("GCLKOUT", "I"),
                builder
                    .bel_indexed(defs::bslots::GCLK_IOB[1], "GCLKIOB", 1)
                    .pin_rename("GCLKOUT", "I"),
                builder
                    .bel_indexed(defs::bslots::BUFGCE[0], "GCLK", 0)
                    .pin_rename("IN", "I")
                    .pin_rename("OUT", "O"),
                builder
                    .bel_indexed(defs::bslots::BUFGCE[1], "GCLK", 1)
                    .pin_rename("IN", "I")
                    .pin_rename("OUT", "O"),
            ];
            if rd.family != "virtex" {
                bels.push(
                    builder
                        .bel_virtual(bslots::IOFB[0])
                        .extra_int_out("I", &["CLKB_IOFB0", "CLKT_IOFB0"]),
                );
                bels.push(
                    builder
                        .bel_virtual(bslots::IOFB[1])
                        .extra_int_out("I", &["CLKB_IOFB1", "CLKT_IOFB1"]),
                );
            }
            let mut x = builder
                .xtile_id(tcid, naming, xy)
                .num_cells(coords.len())
                .extract_muxes(bslots::CLK_INT)
                .skip_muxes(&forbidden)
                .force_ext_pips()
                .bels(bels);
            for (i, &xy) in coords.iter().enumerate() {
                x = x.ref_int(xy, i);
            }
            x.extract();
        }
        if let Some(pips) = builder.pips.get_mut(&(tcid, bslots::CLK_INT)) {
            for w in wires::IMUX_BUFGCE_CE {
                pips.pips
                    .insert((w.cell(1), wires::PULLUP.cell(1).pos()), PipMode::Mux);
            }
        }
    }

    let (pci_w, pci_e) = if rd.family == "virtex" {
        (tcls::PCI_W_V, tcls::PCI_E_V)
    } else {
        (tcls::PCI_W_VE, tcls::PCI_E_VE)
    };
    for (tcid, naming, tkn) in [(pci_w, "PCI_W", "CLKL"), (pci_e, "PCI_E", "CLKR")] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let bel = builder
                .bel_single(bslots::PCILOGIC, "PCILOGIC")
                .pin_name_only("IRDY", 1)
                .pin_name_only("TRDY", 1);
            builder
                .xtile_id(tcid, naming, xy)
                .extract_muxes(bslots::PCI_INT)
                .skip_muxes(&[wires::PCI_CE])
                .bel(bel)
                .ref_int(xy.delta(0, 1), 0)
                .extract();
        }
    }

    for (tkn, tcid, naming) in [
        ("CLKV", tcls::CLKV_CLKV, "CLKV_CLKV"),
        ("CLKB", tcls::CLKV_IO, "CLKV_CLKB"),
        ("CLKB_4DLL", tcls::CLKV_IO, "CLKV_CLKB"),
        ("CLKB_2DLL", tcls::CLKV_IO, "CLKV_CLKB"),
        ("CLKT", tcls::CLKV_IO, "CLKV_CLKT"),
        ("CLKT_4DLL", tcls::CLKV_IO, "CLKV_CLKT"),
        ("CLKT_2DLL", tcls::CLKV_IO, "CLKV_CLKT"),
        ("GCLKV", tcls::CLKV_GCLKV, "CLKV_GCLKV"),
        ("GCLKB", tcls::CLKV_IO, "CLKV_GCLKB"),
        ("GCLKT", tcls::CLKV_IO, "CLKV_GCLKT"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy_l = builder.walk_to_int(xy, Dir::W, false).unwrap();
            let int_xy_r = builder.walk_to_int(xy, Dir::E, false).unwrap();
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(2)
                .ref_int(int_xy_l, 0)
                .ref_int(int_xy_r, 1)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::GCLK_LEAF[..])
                .extract();
        }
    }

    for i in 0..4 {
        builder.extra_name_sub(format!("BRAM_BOT_GCLKE{i}"), 1, wires::GCLK_LEAF[i]);
        builder.extra_name_sub(format!("BRAM_TOP_GCLKE{i}"), 1, wires::GCLK_LEAF[i]);
    }

    for (tkn, tcid, naming) in [
        ("BRAM_BOT", clkv_bram_s, "CLKV_BRAM_S"),
        ("BRAM_BOT_GCLK", clkv_bram_s, "CLKV_BRAM_S"),
        ("LBRAM_BOTS_GCLK", clkv_bram_s, "CLKV_BRAM_S"),
        ("RBRAM_BOTS_GCLK", clkv_bram_s, "CLKV_BRAM_S"),
        ("BRAM_TOP", clkv_bram_n, "CLKV_BRAM_N"),
        ("BRAM_TOP_GCLK", clkv_bram_n, "CLKV_BRAM_N"),
        ("LBRAM_TOPS_GCLK", clkv_bram_n, "CLKV_BRAM_N"),
        ("RBRAM_TOPS_GCLK", clkv_bram_n, "CLKV_BRAM_N"),
    ] {
        for &xy in rd.tiles_by_kind_name(tkn) {
            let int_xy_l = builder.walk_to_int(xy, Dir::W, false).unwrap();
            builder
                .xtile_id(tcid, naming, xy)
                .num_cells(2)
                .ref_int(int_xy_l, 0)
                .ref_int(xy, 1)
                .switchbox(bslots::CLK_INT)
                .optin_muxes(&wires::GCLK_LEAF[..])
                .extract();
        }
    }

    for tcid in [
        cnr_sw,
        tcls::CNR_SE,
        cnr_nw,
        tcls::CNR_NE,
        tcls::BRAM_S,
        tcls::BRAM_N,
    ] {
        let pips = builder.pips.get_mut(&(tcid, bslots::INT)).unwrap();
        for (&(wt, wf), mode) in &mut pips.pips {
            if wires::LV.contains(wt.wire) || wf.wire == wires::PCI_CE {
                *mode = PipMode::Buf;
            }
        }
    }

    for (&(tcid, _bslot), pips) in builder.pips.iter_mut() {
        for (&(wt, wf), mode) in &mut pips.pips {
            if wires::GCLK_LEAF.contains(wt.wire) {
                if matches!(
                    tcid,
                    tcls::CLKV_IO
                        | tcls::CLKV_BRAM_S
                        | tcls::CLKV_BRAM_N
                        | tcls::CLK_S_V
                        | tcls::CLK_N_V
                        | tcls::CLK_S_VE_4DLL
                        | tcls::CLK_N_VE_4DLL
                        | tcls::CLK_S_VE_2DLL
                        | tcls::CLK_N_VE_2DLL
                ) || (tcid == tcls::BRAM_W && matches!(wt.cell.to_idx(), 4..8))
                    || (tcid == tcls::BRAM_E && matches!(wt.cell.to_idx(), 8..12))
                {
                    *mode = PipMode::PermaBuf;
                } else {
                    *mode = PipMode::Buf;
                }
            }
            if wires::BRAM_QUAD_DOUT.contains(wt.wire)
                || wires::BRAM_QUAD_DOUT_S.contains(wt.wire)
                || wires::BRAM_QUAD_DIN_S.contains(wt.wire)
                || wires::BRAM_QUAD_DIN_S.contains(wf.wire)
                || wires::BRAM_QUAD_ADDR_S.contains(wt.wire)
                || wires::BRAM_QUAD_ADDR_S.contains(wf.wire)
            {
                *mode = PipMode::Buf;
            }
            if wires::SINGLE_W.contains(wt.wire)
                || wires::SINGLE_E.contains(wt.wire)
                || wires::SINGLE_S.contains(wt.wire)
                || wires::SINGLE_N.contains(wt.wire)
            {
                *mode = PipMode::Pass;
            }
            if wires::HEX_W2.contains(wt.wire)
                || wires::HEX_W3.contains(wt.wire)
                || wires::HEX_W4.contains(wt.wire)
                || wires::HEX_W5.contains(wt.wire)
                || wires::HEX_E2.contains(wt.wire)
                || wires::HEX_E3.contains(wt.wire)
                || wires::HEX_E4.contains(wt.wire)
                || wires::HEX_E5.contains(wt.wire)
                || wires::HEX_S2.contains(wt.wire)
                || wires::HEX_S3.contains(wt.wire)
                || wires::HEX_S4.contains(wt.wire)
                || wires::HEX_S5.contains(wt.wire)
                || wires::HEX_N2.contains(wt.wire)
                || wires::HEX_N3.contains(wt.wire)
                || wires::HEX_N4.contains(wt.wire)
                || wires::HEX_N5.contains(wt.wire)
            {
                *mode = PipMode::PermaBuf;
            }
        }

        let mut new_pips = vec![];
        pips.pips.retain(|&(wt, wf), &mut mode| {
            if mode == PipMode::Mux
                && let Some(nwt) = wire_to_mux(wt.wire)
            {
                let nwt = TileWireCoord {
                    cell: wt.cell,
                    wire: nwt,
                };
                new_pips.push((nwt, wf, PipMode::Mux));
                new_pips.push((wt, nwt.pos(), PipMode::Buf));
                false
            } else {
                true
            }
        });
        for (wt, wf, mode) in new_pips {
            pips.pips.insert((wt, wf), mode);
        }
    }

    for naming in builder.ndb.tile_class_namings.values_mut() {
        let mut new_wires = vec![];
        for (&twc, wn) in &mut naming.wires {
            let mut new_alt = vec![];
            for aw in &wn.alt_pips_from {
                if let Some(naw) = wire_to_mux(aw.wire) {
                    new_alt.push(TileWireCoord {
                        wire: naw,
                        cell: aw.cell,
                    });
                }
            }
            wn.alt_pips_from.extend(new_alt);
            if let Some(nw) = wire_to_mux(twc.wire) {
                let nw = TileWireCoord {
                    wire: nw,
                    cell: twc.cell,
                };
                new_wires.push((nw, wn.clone()));
            }
        }
        for (w, wn) in new_wires {
            naming.wires.insert(w, wn);
        }
        let mut new_ext = vec![];
        for (&(wt, wf), ext) in &naming.ext_pips {
            if let Some(nwt) = wire_to_mux(wt.wire) {
                let nwt = TileWireCoord {
                    wire: nwt,
                    cell: wt.cell,
                };
                new_ext.push(((nwt, wf), ext.clone()));
            }
        }
        naming.ext_pips.extend(new_ext);
    }

    builder.build()
}
