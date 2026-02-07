use prjcombine_entity::EntityBundleIndex;
use prjcombine_interconnect::db::BelAttributeType;
use prjcombine_re_hammer::Session;
use prjcombine_spartan6::defs::{bcls, bslots, tcls};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    spartan6::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::DSP);
    let mode = "DSP48A1";
    let mut bctx = ctx.bel(bslots::DSP);
    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();
    for (pids, _, _) in backend.edev.db.bel_classes[bcls::DSP].inputs.bundles() {
        if let EntityBundleIndex::Single(pid) = pids {
            bctx.mode(mode).test_bel_input_inv_auto(pid);
        }
    }
    for (aid, aname, attr) in &backend.edev.db.bel_classes[bcls::DSP].attributes {
        if attr.typ == BelAttributeType::Bool {
            bctx.mode(mode)
                .test_bel_attr_bool_rename(aname, aid, "0", "1");
        } else {
            bctx.mode(mode).test_bel_attr_auto(aid);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::DSP;
    let bslot = bslots::DSP;
    for (pids, _, _) in ctx.edev.db.bel_classes[bcls::DSP].inputs.bundles() {
        if let EntityBundleIndex::Single(pid) = pids {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pid);
        }
    }
    for (aid, _, attr) in &ctx.edev.db.bel_classes[bcls::DSP].attributes {
        if attr.typ == BelAttributeType::Bool {
            ctx.collect_bel_attr_bi(tcid, bslot, aid);
        } else {
            ctx.collect_bel_attr(tcid, bslot, aid);
        }
    }
}
