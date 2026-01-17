use prjcombine_re_fpga_hammer::{xlat_bitvec, xlat_bool};
use prjcombine_re_hammer::Session;
use prjcombine_virtex::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tile_name in ["BRAM_W", "BRAM_E", "BRAM_M"] {
        let tcls = backend.edev.db.get_tile_class(tile_name);
        if backend.edev.tile_index[tcls].is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile_name);
        let mut bctx = ctx.bel(defs::bslots::BRAM);
        let mode = "BLOCKRAM";

        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for (pinmux, pin) in [("CLKAMUX", "CLKA"), ("CLKBMUX", "CLKB")] {
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .pin(pin)
                .test_enum(pinmux, &["0", "1"]);
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
                .test_enum(pinmux, &["0", "1", pin, pin_b]);
        }
        for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
            bctx.mode(mode)
                .test_enum(attr, &["4096X1", "2048X2", "1024X4", "512X8", "256X16"]);
        }
        for i in 0..0x10 {
            let attr = format!("INIT_{i:02x}");
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .test_multi_attr_hex(attr, 256);
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
        let ti = ctx.extract_enum_bool(tile, bel, "CLKAMUX", "1", "0");
        ctx.insert(tile, "INT", "INV.0.IMUX.BRAM.CLKA", ti);
        let ti = ctx.extract_enum_bool(tile, bel, "CLKBMUX", "1", "0");
        ctx.insert(tile, "INT", "INV.0.IMUX.BRAM.CLKB", ti);
        for (wire, pinmux, pin, pin_b) in [
            ("SELA", "ENAMUX", "ENA", "ENA_B"),
            ("SELB", "ENBMUX", "ENB", "ENB_B"),
            ("WEA", "WEAMUX", "WEA", "WEA_B"),
            ("WEB", "WEBMUX", "WEB", "WEB_B"),
            ("RSTA", "RSTAMUX", "RSTA", "RSTA_B"),
            ("RSTB", "RSTBMUX", "RSTB", "RSTB_B"),
        ] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            ctx.insert(
                tile,
                "INT",
                format!("INV.0.IMUX.BRAM.{wire}"),
                xlat_bool(d0, d1),
            );
        }
        let mut diffs_data = vec![];
        for i in 0..0x10 {
            diffs_data.extend(ctx.state.get_diffs(tile, bel, format!("INIT_{i:02x}"), ""));
        }
        for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
            ctx.collect_enum(
                tile,
                bel,
                attr,
                &["4096X1", "2048X2", "1024X4", "512X8", "256X16"],
            );
        }
        ctx.insert(tile, bel, "DATA", xlat_bitvec(diffs_data));
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        present.discard_bits(ctx.item(tile, "INT", "INV.0.IMUX.BRAM.SELA"));
        present.discard_bits(ctx.item(tile, "INT", "INV.0.IMUX.BRAM.SELB"));
        present.assert_empty();
    }
}
