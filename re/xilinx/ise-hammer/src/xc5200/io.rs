use prjcombine_re_hammer::Session;
use prjcombine_xc2000::bels::xc5200 as bels;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::pip::PipInt,
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tile in ["IO.L", "IO.R", "IO.B", "IO.T"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for i in 0..4 {
            let mut bctx = ctx.bel(bels::IO[i]);
            let mode = "IOB";
            bctx.mode(mode).test_enum("SLEW", &["SLOW", "FAST"]);
            bctx.mode(mode).test_enum("PULL", &["PULLUP", "PULLDOWN"]);
            bctx.mode(mode)
                .attr("DELAYMUX", "NODELAY")
                .pin("I")
                .test_enum("IMUX", &["I", "INOT"]);
            bctx.mode(mode)
                .attr("IMUX", "I")
                .pin("I")
                .test_enum("DELAYMUX", &["DELAY", "NODELAY"]);
            bctx.mode(mode).pin("T").test_enum("TMUX", &["T", "TNOT"]);
            if tile == "IO.L" || tile == "IO.R" {
                bctx.mode(mode)
                    .pin("O")
                    .attr("TMUX", "T")
                    .pin("T")
                    .test_enum("OMUX", &["O", "ONOT"]);
            } else {
                let sn = if tile == "IO.B" { 'S' } else { 'N' };
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "INT")
                    .pip("O", (PipInt, 0, "GND"))
                    .attr("OMUX", "O")
                    .commit();
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "INT.INV")
                    .pip("O", (PipInt, 0, "GND"))
                    .attr("OMUX", "ONOT")
                    .commit();
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "OMUX")
                    .pip("O", (PipInt, 0, format!("OMUX{i}.BUF.{sn}")))
                    .attr("OMUX", "O")
                    .commit();
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "OMUX.INV")
                    .pip("O", (PipInt, 0, format!("OMUX{i}.BUF.{sn}")))
                    .attr("OMUX", "ONOT")
                    .commit();
            }
        }
        for i in 0..4 {
            let mut bctx = ctx.bel(bels::TBUF[i]);
            bctx.mode("TBUF")
                .test_manual("TMUX", "T")
                .pin("T")
                .pin_pips("T")
                .commit();
        }
        let mut bctx = ctx.bel(bels::BUFR);
        bctx.build()
            .null_bits()
            .test_manual("ENABLE", "1")
            .pip("OUT", "IN")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["IO.L", "IO.R", "IO.B", "IO.T"] {
        for i in 0..4 {
            let bel = &format!("IO{i}");
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
