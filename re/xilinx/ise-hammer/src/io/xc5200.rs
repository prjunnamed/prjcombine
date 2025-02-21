use prjcombine_re_hammer::Session;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for tile in ["IO.L", "IO.R", "IO.B", "IO.T"] {
        for i in 0..4 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
                format!("IOB{i}"),
                TileBits::MainAuto,
            );
            fuzz_enum!(ctx, "SLEW", ["SLOW", "FAST"], [(mode "IOB")]);
            fuzz_enum!(ctx, "PULL", ["PULLUP", "PULLDOWN"], [(mode "IOB")]);
            fuzz_enum!(ctx, "IMUX", ["I", "INOT"], [(mode "IOB"), (attr "DELAYMUX", "NODELAY"), (pin "I")]);
            fuzz_enum!(ctx, "DELAYMUX", ["DELAY", "NODELAY"], [(mode "IOB"), (attr "IMUX", "I"), (pin "I")]);
            fuzz_enum!(ctx, "TMUX", ["T", "TNOT"], [(mode "IOB"), (pin "T")]);
            if tile == "IO.L" || tile == "IO.R" {
                fuzz_enum!(ctx, "OMUX", ["O", "ONOT"], [(mode "IOB"), (pin "O"), (attr "TMUX", "T"), (pin "T")]);
            } else {
                let sn = if tile == "IO.B" { 'S' } else { 'N' };
                fuzz_one!(ctx, "OMUX", "INT", [
                    (mode "IOB"),
                    (attr "TMUX", "T"),
                    (pin "O"),
                    (pin "T")
                ], [
                    (pip (int 0, "GND"), (pin "O")),
                    (attr "OMUX", "O")
                ]);
                fuzz_one!(ctx, "OMUX", "INT.INV", [
                    (mode "IOB"),
                    (attr "TMUX", "T"),
                    (pin "O"),
                    (pin "T")
                ], [
                    (pip (int 0, "GND"), (pin "O")),
                    (attr "OMUX", "ONOT")
                ]);
                fuzz_one!(ctx, "OMUX", "OMUX", [
                    (mode "IOB"),
                    (attr "TMUX", "T"),
                    (pin "O"),
                    (pin "T")
                ], [
                    (pip (int 0, format!("OMUX{i}.BUF.{sn}")), (pin "O")),
                    (attr "OMUX", "O")
                ]);
                fuzz_one!(ctx, "OMUX", "OMUX.INV", [
                    (mode "IOB"),
                    (attr "TMUX", "T"),
                    (pin "O"),
                    (pin "T")
                ], [
                    (pip (int 0, format!("OMUX{i}.BUF.{sn}")), (pin "O")),
                    (attr "OMUX", "ONOT")
                ]);
            }
        }
        for i in 0..4 {
            let ctx = FuzzCtx::new(
                session,
                backend,
                tile,
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
        let ctx = FuzzCtx::new(session, backend, tile, "BUFR", TileBits::Null);
        fuzz_one!(ctx, "ENABLE", "1", [], [
            (pip (pin "IN"), (pin "OUT"))
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["IO.L", "IO.R", "IO.B", "IO.T"] {
        for i in 0..4 {
            let bel = &format!("IOB{i}");
            ctx.collect_enum(tile, bel, "SLEW", &["FAST", "SLOW"]);
            ctx.collect_enum_default(tile, bel, "PULL", &["PULLUP", "PULLDOWN"], "NONE");
            ctx.collect_enum(tile, bel, "DELAYMUX", &["DELAY", "NODELAY"]);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "I", "INOT");
            ctx.tiledb.insert(tile, bel, "INV.I", item);
            let item = ctx.extract_enum_bool(tile, bel, "TMUX", "T", "TNOT");
            ctx.tiledb.insert(tile, bel, "INV.T", item);
            if tile == "IO.L" || tile == "IO.R" {
                let item = ctx.extract_enum_bool(tile, bel, "OMUX", "O", "ONOT");
                ctx.tiledb.insert(tile, bel, "INV.O", item);
            } else {
                ctx.collect_enum(tile, bel, "OMUX", &["INT", "INT.INV", "OMUX", "OMUX.INV"]);
            }
        }
        for i in 0..4 {
            let bel = &format!("TBUF{i}");
            ctx.collect_enum_default(tile, bel, "TMUX", &["T"], "NONE");
        }
    }
}
