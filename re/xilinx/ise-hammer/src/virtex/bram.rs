use prjcombine_re_collector::legacy::{xlat_bit_bi_legacy, xlat_bitvec_legacy};
use prjcombine_re_hammer::Session;
use prjcombine_virtex::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tile_name in ["BRAM_W", "BRAM_E", "BRAM_M"] {
        let tcls = backend.edev.db.get_tile_class(tile_name);
        if backend.edev.tile_index[tcls].is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new_legacy(session, backend, tile_name);
        let mut bctx = ctx.bel(defs::bslots::BRAM);
        let mode = "BLOCKRAM";

        bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();
        for (pinmux, pin) in [("CLKAMUX", "CLKA"), ("CLKBMUX", "CLKB")] {
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .pin(pin)
                .test_enum_legacy(pinmux, &["0", "1"]);
        }
        for (pinmux, pin, pin_b) in [
            ("ENAMUX", "ENA", "ENA_B"),
            ("ENBMUX", "ENB", "ENB_B"),
            ("WEAMUX", "WEA", "WEA_B"),
            ("WEBMUX", "WEB", "WEB_B"),
            ("RSTAMUX", "RSTA", "RSTA_B"),
            ("RSTBMUX", "RSTB", "RSTB_B"),
        ] {
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .pin(pin)
                .test_enum_legacy(pinmux, &["0", "1", pin, pin_b]);
        }
        for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
            bctx.mode(mode)
                .test_enum_legacy(attr, &["4096X1", "2048X2", "1024X4", "512X8", "256X16"]);
        }
        for i in 0..0x10 {
            let attr = format!("INIT_{i:02x}");
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .test_multi_attr_hex_legacy(attr, 256);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["BRAM_W", "BRAM_E", "BRAM_M"] {
        let tcls = ctx.edev.db.get_tile_class(tile);
        if ctx.edev.tile_index[tcls].is_empty() {
            continue;
        }
        let bel = "BRAM";
        let ti = ctx.extract_bit_bi_legacy(tile, bel, "CLKAMUX", "1", "0");
        ctx.insert_legacy(tile, "INT", "INV.0.IMUX.BRAM.CLKA", ti);
        let ti = ctx.extract_bit_bi_legacy(tile, bel, "CLKBMUX", "1", "0");
        ctx.insert_legacy(tile, "INT", "INV.0.IMUX.BRAM.CLKB", ti);
        for (wire, pinmux, pin, pin_b) in [
            ("SELA", "ENAMUX", "ENA", "ENA_B"),
            ("SELB", "ENBMUX", "ENB", "ENB_B"),
            ("WEA", "WEAMUX", "WEA", "WEA_B"),
            ("WEB", "WEBMUX", "WEB", "WEB_B"),
            ("RSTA", "RSTAMUX", "RSTA", "RSTA_B"),
            ("RSTB", "RSTBMUX", "RSTB", "RSTB_B"),
        ] {
            let d0 = ctx.get_diff_legacy(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.get_diff_legacy(tile, bel, pinmux, "1"));
            let d1 = ctx.get_diff_legacy(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.get_diff_legacy(tile, bel, pinmux, "0"));
            ctx.insert_legacy(
                tile,
                "INT",
                format!("INV.0.IMUX.BRAM.{wire}"),
                xlat_bit_bi_legacy(d0, d1),
            );
        }
        let mut diffs_data = vec![];
        for i in 0..0x10 {
            diffs_data.extend(ctx.get_diffs_legacy(tile, bel, format!("INIT_{i:02x}"), ""));
        }
        for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
            ctx.collect_enum_legacy(
                tile,
                bel,
                attr,
                &["4096X1", "2048X2", "1024X4", "512X8", "256X16"],
            );
        }
        ctx.insert_legacy(tile, bel, "DATA", xlat_bitvec_legacy(diffs_data));
        let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
        present.discard_bits_legacy(ctx.item_legacy(tile, "INT", "INV.0.IMUX.BRAM.SELA"));
        present.discard_bits_legacy(ctx.item_legacy(tile, "INT", "INV.0.IMUX.BRAM.SELB"));
        present.assert_empty();
    }
}
