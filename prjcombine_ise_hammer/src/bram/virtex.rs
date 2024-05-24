use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bitvec, xlat_bool, CollectorCtx},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for tile_name in ["LBRAM", "RBRAM", "MBRAM"] {
        let node_kind = backend.egrid.db.get_node(tile_name);
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bel = BelId::from_idx(0);
        let ctx = FuzzCtx {
            session,
            node_kind,
            bits: TileBits::Bram,
            tile_name,
            bel,
            bel_name: "BRAM",
        };

        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "BLOCKRAM")]);
        for (pinmux, pin) in [("CLKAMUX", "CLKA"), ("CLKBMUX", "CLKB")] {
            fuzz_enum!(ctx, pinmux, ["0", "1"], [
                (mode "BLOCKRAM"),
                (attr "PORTA_ATTR", "256X16"),
                (attr "PORTB_ATTR", "256X16"),
                (pin pin)
            ]);
        }
        for (pinmux, pin, pin_b) in [
            ("ENAMUX", "ENA", "ENA_B"),
            ("ENBMUX", "ENB", "ENB_B"),
            ("WEAMUX", "WEA", "WEA_B"),
            ("WEBMUX", "WEB", "WEB_B"),
            ("RSTAMUX", "RSTA", "RSTA_B"),
            ("RSTBMUX", "RSTB", "RSTB_B"),
        ] {
            fuzz_enum!(ctx, pinmux, ["0", "1", pin, pin_b], [
                (mode "BLOCKRAM"),
                (attr "PORTA_ATTR", "256X16"),
                (attr "PORTB_ATTR", "256X16"),
                (pin pin)
            ]);
        }
        for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
            fuzz_enum!(ctx, attr, ["4096X1", "2048X2", "1024X4", "512X8", "256X16"], [
                (mode "BLOCKRAM")
            ]);
        }
        for i in 0..0x10 {
            let attr = format!("INIT_{i:02x}").leak();
            fuzz_multi!(ctx, attr, "", 256, [
                (mode "BLOCKRAM"),
                (attr "PORTA_ATTR", "256X16"),
                (attr "PORTB_ATTR", "256X16")
            ], (attr_hex attr));
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    for tile in ["LBRAM", "RBRAM", "MBRAM"] {
        let node_kind = egrid.db.get_node(tile);
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bel = "BRAM";
        let ti = ctx.extract_enum_bool(tile, bel, "CLKAMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "CLKAINV", ti);
        let ti = ctx.extract_enum_bool(tile, bel, "CLKBMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "CLKBINV", ti);
        for (pininv, pinmux, pin, pin_b) in [
            ("ENAINV", "ENAMUX", "ENA", "ENA_B"),
            ("ENBINV", "ENBMUX", "ENB", "ENB_B"),
            ("WEAINV", "WEAMUX", "WEA", "WEA_B"),
            ("WEBINV", "WEBMUX", "WEB", "WEB_B"),
            ("RSTAINV", "RSTAMUX", "RSTA", "RSTA_B"),
            ("RSTBINV", "RSTBMUX", "RSTB", "RSTB_B"),
        ] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            ctx.tiledb.insert(tile, bel, pininv, xlat_bool(d0, d1));
        }
        let mut diffs_data = vec![];
        for i in 0..0x10 {
            diffs_data.extend(
                ctx.state
                    .get_diffs(tile, bel, format!("INIT_{i:02x}").leak(), ""),
            );
        }
        for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
            ctx.collect_enum(
                tile,
                bel,
                attr,
                &["4096X1", "2048X2", "1024X4", "512X8", "256X16"],
            );
        }
        ctx.tiledb
            .insert(tile, bel, "DATA", xlat_bitvec(diffs_data));
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        present.discard_bits(ctx.tiledb.item(tile, bel, "ENAINV"));
        present.discard_bits(ctx.tiledb.item(tile, bel, "ENBINV"));
        present.assert_empty();
    }
}
