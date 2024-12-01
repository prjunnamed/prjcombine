use prjcombine_collector::{xlat_enum, Diff};
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem};

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum,
    fuzz_enum_suffix, fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for i in 0..4 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CLB",
            format!("LC{i}"),
            TileBits::MainAuto,
        );
        let mode = if i % 2 == 0 { "LC5A" } else { "LC5B" };
        fuzz_multi!(ctx, "LUT", "", 16, [(mode mode)], (attr_oldlut "LUT", 'F'));
        fuzz_one!(ctx, "FFLATCH",  "#LATCH", [
            (mode mode),
            (attr "DMUX", "F"),
            (pin "DI")
        ], [
            (attr "FFLATCH", "#LATCH"),
            (pin "Q")
        ]);
        if mode == "LC5A" {
            fuzz_enum!(ctx, "DOMUX", ["DI", "F5O", "CO"], [(mode mode), (pin "DO"), (pin "DI")]);
        } else {
            fuzz_enum!(ctx, "DOMUX", ["DI", "CO"], [(mode mode), (pin "DO"), (pin "DI")]);
        }
        fuzz_enum!(ctx, "DMUX", ["F", "DO"], [(mode mode), (pin "DI"), (attr "DOMUX", "DI")]);
        fuzz_enum!(ctx, "CEMUX", ["CE"], [(mode mode), (pin "CE")]);
        fuzz_enum!(ctx, "CLRMUX", ["CLR"], [(mode mode), (pin "CLR")]);
        fuzz_enum!(ctx, "CKMUX", ["CK", "CKNOT"], [
            (mode mode),
            (pin "CK"),
            (pin "DI"),
            (pin "Q"),
            (attr "DMUX", "F"),
            (attr "FFLATCH", "")
        ]);
        fuzz_enum_suffix!(ctx, "CKMUX", "LATCH", ["CK", "CKNOT"], [
            (mode mode),
            (pin "CK"),
            (pin "DI"),
            (pin "Q"),
            (attr "DMUX", "F"),
            (attr "FFLATCH", "#LATCH")
        ]);
        fuzz_enum!(ctx, "COMUX", ["CY"], [(mode mode), (pin "CO")]);
    }
    for i in 0..4 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CLB",
            format!("TBUF{i}"),
            TileBits::MainAuto,
        );
        fuzz_one!(ctx, "TMUX", "T", [
            (mode "TBUF")
        ], [
            (pin "T"),
            (pin_pips "T")
        ]);
    }
    let ctx = FuzzCtx::new(session, backend, "CLB", "VCC_GND", TileBits::MainAuto);
    fuzz_enum!(ctx, "MUX", ["0", "1"], [(mode "VCC_GND")]);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CLB";
    for i in 0..4 {
        let bel = &format!("LC{i}");
        ctx.collect_bitvec(tile, bel, "LUT", "");
        if i % 2 == 0 {
            ctx.collect_enum(tile, bel, "DOMUX", &["DI", "F5O", "CO"]);
        } else {
            ctx.collect_enum(tile, bel, "DOMUX", &["DI", "CO"]);
        }
        let item = xlat_enum(vec![
            ("FF", Diff::default()),
            (
                "LATCH",
                ctx.state
                    .get_diff(tile, bel, "FFLATCH", "#LATCH")
                    .combine(&!ctx.state.peek_diff(tile, bel, "CKMUX", "CKNOT")),
            ),
        ]);
        ctx.tiledb.insert(tile, bel, "FFLATCH", item);
        ctx.collect_enum(tile, bel, "DMUX", &["F", "DO"]);
        ctx.collect_enum_default(tile, bel, "CLRMUX", &["CLR"], "NONE");
        ctx.collect_enum_default(tile, bel, "CEMUX", &["CE"], "NONE");
        let item = ctx.extract_enum_bool(tile, bel, "CKMUX", "CK", "CKNOT");
        ctx.tiledb.insert(tile, bel, "INV.CK", item);
        let item = ctx.extract_enum_bool(tile, bel, "CKMUX.LATCH", "CKNOT", "CK");
        ctx.tiledb.insert(tile, bel, "INV.CK", item);
        ctx.state.get_diff(tile, bel, "COMUX", "CY").assert_empty();
        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK",
            TileItem::from_bit(TileBit::new(0, 1, [5, 10, 23, 28][i]), true),
        );
    }
    for i in 0..4 {
        let bel = &format!("TBUF{i}");
        ctx.collect_enum_default(tile, bel, "TMUX", &["T"], "NONE");
    }
    let bel = "VCC_GND";
    ctx.collect_enum_bool(tile, bel, "MUX", "0", "1");
}
