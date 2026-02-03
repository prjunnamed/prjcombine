use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc5200::{bcls, bslots, enums, tcls};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    xc5200::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::IO[i]);
            let mode = "IOB";
            bctx.mode(mode).test_bel_attr(bcls::IO::SLEW);
            bctx.mode(mode)
                .test_bel_attr_default(bcls::IO::PULL, enums::IO_PULL::NONE);
            bctx.mode(mode)
                .attr("DELAYMUX", "NODELAY")
                .pin("I")
                .test_bel_attr_bool_rename("IMUX", bcls::IO::INV_I, "I", "INOT");
            bctx.mode(mode)
                .attr("IMUX", "I")
                .pin("I")
                .test_bel_attr_bool_rename("DELAYMUX", bcls::IO::DELAY_ENABLE, "NODELAY", "DELAY");
            bctx.mode(mode)
                .pin("T")
                .test_bel_input_inv_enum("TMUX", bcls::IO::T, "T", "TNOT");
            if matches!(tcid, tcls::IO_W | tcls::IO_E) {
                bctx.mode(mode)
                    .pin("O")
                    .attr("TMUX", "T")
                    .pin("T")
                    .test_bel_input_inv_enum("OMUX", bcls::IO::O, "O", "ONOT");
            }
        }
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::TBUF[i]);
            bctx.mode("TBUF")
                .test_bel_attr_bits(bcls::TBUF::T_ENABLE)
                .pin("T")
                .pin_pips("T")
                .commit();
        }
        let mut bctx = ctx.bel(bslots::BUFR);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .pip("OUT", "IN")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        for i in 0..4 {
            let bslot = bslots::IO[i];
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::SLEW);
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IO::PULL, enums::IO_PULL::NONE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IO::DELAY_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IO::INV_I);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IO::T);
            if matches!(tcid, tcls::IO_W | tcls::IO_E) {
                ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IO::O);
            }
        }
        for i in 0..4 {
            ctx.collect_bel_attr(tcid, bslots::TBUF[i], bcls::TBUF::T_ENABLE);
        }
    }
}
