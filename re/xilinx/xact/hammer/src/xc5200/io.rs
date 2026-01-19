use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc5200::{bcls, bslots, enums, tcls};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx, specials};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::IO[i]);
            bctx.mode("IO").cfg("IN", "I").test_bel_attr_default_as(
                "PAD",
                bcls::IO::PULL,
                enums::IO_PULL::NONE,
            );
            bctx.mode("IO").cfg("IN", "I").test_bel_attr_default_as(
                "PAD",
                bcls::IO::SLEW,
                enums::IO_SLEW::SLOW,
            );
            bctx.mode("IO")
                .test_bel_special(specials::IO_IN_I)
                .cfg("IN", "I")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .test_bel_attr_bits(bcls::IO::INV_I)
                .cfg("IN", "NOT")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .test_bel_attr_bits(bcls::IO::DELAY_ENABLE)
                .cfg("IN", "DELAY")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .test_bel_special(specials::IO_OUT_O)
                .cfg("OUT", "O")
                .commit();
            bctx.mode("IO")
                .null_bits()
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_bel_special(specials::IO_TRI_T)
                .cfg("TRI", "T")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .cfg("OUT", "O")
                .test_bel_input_inv(bcls::IO::T, true)
                .cfg("TRI", "NOT")
                .commit();
            if tcid == tcls::IO_W || tcid == tcls::IO_E {
                bctx.mode("IO")
                    .cfg("IN", "I")
                    .cfg("OUT", "O")
                    .test_bel_input_inv(bcls::IO::O, true)
                    .cfg("OUT", "NOT")
                    .commit();
            }
        }
        if tcid == tcls::IO_S {
            let mut bctx = ctx.bel(bslots::SCANTEST);
            bctx.mode("SCANTEST")
                .test_bel_attr_as("OUT", bcls::SCANTEST::OUT);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        for i in 0..4 {
            let bslot = bslots::IO[i];
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IO::PULL, enums::IO_PULL::NONE);
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IO::SLEW, enums::IO_SLEW::SLOW);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::DELAY_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::INV_I);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::IO::T);
            if tcid == tcls::IO_W || tcid == tcls::IO_E {
                ctx.collect_bel_input_inv(tcid, bslot, bcls::IO::O);
            }
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IO_IN_I);
            diff.apply_enum_diff_attr(
                ctx.bel_attr_enum(tcid, bslot, bcls::IO::PULL),
                enums::IO_PULL::NONE,
                enums::IO_PULL::PULLUP,
            );
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUT_O);
            diff.apply_bit_diff_raw(ctx.bel_input_inv(tcid, bslot, bcls::IO::T), false, true);
            diff.assert_empty();
        }
        if tcid == tcls::IO_S {
            ctx.collect_bel_attr(tcid, bslots::SCANTEST, bcls::SCANTEST::OUT);
        }
    }
}
