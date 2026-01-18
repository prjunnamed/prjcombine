use prjcombine_interconnect::dir::DirV;
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc5200::{bslots, tcls, wires};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::pip::PipInt,
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        let mut ctx = FuzzCtx::new_id(session, backend, tcid);
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::IO[i]);
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
            if tcid == tcls::IO_W || tcid == tcls::IO_E {
                bctx.mode(mode)
                    .pin("O")
                    .attr("TMUX", "T")
                    .pin("T")
                    .test_enum("OMUX", &["O", "ONOT"]);
            } else {
                let sn = if tcid == tcls::IO_S { DirV::S } else { DirV::N };
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "INT")
                    .pip("O", (PipInt, 0, wires::TIE_0))
                    .attr("OMUX", "O")
                    .commit();
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "INT.INV")
                    .pip("O", (PipInt, 0, wires::TIE_0))
                    .attr("OMUX", "ONOT")
                    .commit();
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "OMUX")
                    .pip(
                        "O",
                        (
                            PipInt,
                            0,
                            match sn {
                                DirV::S => wires::OMUX_BUF_S[i],
                                DirV::N => wires::OMUX_BUF_N[i],
                            },
                        ),
                    )
                    .attr("OMUX", "O")
                    .commit();
                bctx.mode(mode)
                    .attr("TMUX", "T")
                    .pin("O")
                    .pin("T")
                    .test_manual("OMUX", "OMUX.INV")
                    .pip(
                        "O",
                        (
                            PipInt,
                            0,
                            match sn {
                                DirV::S => wires::OMUX_BUF_S[i],
                                DirV::N => wires::OMUX_BUF_N[i],
                            },
                        ),
                    )
                    .attr("OMUX", "ONOT")
                    .commit();
            }
        }
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::TBUF[i]);
            bctx.mode("TBUF")
                .test_manual("TMUX", "T")
                .pin("T")
                .pin_pips("T")
                .commit();
        }
        let mut bctx = ctx.bel(bslots::BUFR);
        bctx.build()
            .null_bits()
            .test_manual("ENABLE", "1")
            .pip("OUT", "IN")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["IO_W", "IO_E", "IO_S", "IO_N"] {
        for i in 0..4 {
            let bel = &format!("IO[{i}]");
            ctx.collect_enum(tile, bel, "SLEW", &["FAST", "SLOW"]);
            ctx.collect_enum_default(tile, bel, "PULL", &["PULLUP", "PULLDOWN"], "NONE");
            ctx.collect_enum(tile, bel, "DELAYMUX", &["DELAY", "NODELAY"]);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "I", "INOT");
            ctx.insert(tile, bel, "INV.I", item);
            let item = ctx.extract_enum_bool(tile, bel, "TMUX", "T", "TNOT");
            ctx.insert(tile, bel, "INV.T", item);
            if tile == "IO_W" || tile == "IO_E" {
                let item = ctx.extract_enum_bool(tile, bel, "OMUX", "O", "ONOT");
                ctx.insert(tile, bel, "INV.O", item);
            } else {
                ctx.collect_enum(tile, bel, "OMUX", &["INT", "INT.INV", "OMUX", "OMUX.INV"]);
            }
        }
        for i in 0..4 {
            let bel = &format!("TBUF[{i}]");
            ctx.collect_enum_default(tile, bel, "TMUX", &["T"], "NONE");
        }
    }
}
