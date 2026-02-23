use prjcombine_interconnect::db::BelAttributeType;
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{bcls, bslots, virtex6::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::EMAC) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::EMAC);
    let mode = "TEMAC_SINGLE";

    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();
    for val in ["FALSE", "TRUE"] {
        // ???
        bctx.mode(mode)
            .null_bits()
            .test_bel_special(specials::EMAC_MDIO_IGNORE_PHYADZERO)
            .attr("EMAC_MDIO_IGNORE_PHYADZERO", val)
            .commit();
    }

    for (aid, _, attr) in &backend.edev.db[bcls::EMAC_V6].attributes {
        match attr.typ {
            BelAttributeType::Bool => {
                bctx.mode(mode)
                    .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
            }
            BelAttributeType::BitVec(_) => {
                bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
            }
            _ => unreachable!(),
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    if !ctx.has_tcls(tcls::EMAC) {
        return;
    }
    let tcid = tcls::EMAC;
    let bslot = bslots::EMAC;
    for (aid, _, attr) in &ctx.edev.db[bcls::EMAC_V6].attributes {
        match attr.typ {
            BelAttributeType::Bool => {
                ctx.collect_bel_attr_bi(tcid, bslot, aid);
            }
            BelAttributeType::BitVec(_) => {
                ctx.collect_bel_attr(tcid, bslot, aid);
            }
            _ => unreachable!(),
        }
    }
}
