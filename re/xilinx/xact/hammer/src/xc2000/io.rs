use prjcombine_interconnect::db::BelKind;
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc2000::{bcls, bslots, tcls, tslots};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for slot in tcls.bels.ids() {
            if backend.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
                continue;
            }
            let mut bctx = ctx.bel(slot);
            bctx.mode("IO").test_bel_attr_as("I", bcls::IO::MUX_I);
        }
        if tcid == tcls::CLB_SE {
            let mut bctx = ctx.bel(bslots::MISC_SE);
            bctx.test_attr_global_enum_bool_as(
                "DONEPAD",
                bcls::MISC_SE::DONE_PULLUP,
                "NOPULLUP",
                "PULLUP",
            );
            bctx.test_attr_global_enum_bool_as(
                "REPROGRAM",
                bcls::MISC_SE::REPROGRAM_ENABLE,
                "DISABLE",
                "ENABLE",
            );
        }
        if tcid == tcls::CLB_SW {
            let mut bctx = ctx.bel(bslots::MISC_SW);
            bctx.test_attr_global_as("READ", bcls::MISC_SW::READBACK_MODE);
        }
        if tcid == tcls::CLB_NW {
            let mut bctx = ctx.bel(bslots::MISC_NW);
            bctx.test_attr_global_as("INPUT", bcls::MISC_NW::IO_INPUT_MODE);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        for slot in tcls.bels.ids() {
            if ctx.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
                continue;
            }
            ctx.collect_bel_attr(tcid, slot, bcls::IO::MUX_I);
        }
        if tcid == tcls::CLB_SE {
            ctx.collect_bel_attr_bi(tcid, bslots::MISC_SE, bcls::MISC_SE::REPROGRAM_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslots::MISC_SE, bcls::MISC_SE::DONE_PULLUP);
        }
        if tcid == tcls::CLB_SW {
            ctx.collect_bel_attr(tcid, bslots::MISC_SW, bcls::MISC_SW::READBACK_MODE);
        }
        if tcid == tcls::CLB_NW {
            ctx.collect_bel_attr(tcid, bslots::MISC_NW, bcls::MISC_NW::IO_INPUT_MODE);
        }
    }
}
