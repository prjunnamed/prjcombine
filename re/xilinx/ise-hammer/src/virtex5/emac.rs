use prjcombine_interconnect::db::{BelAttributeType, BelInputId};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{bcls, bslots, virtex5::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

const EMAC_INVPINS: &[BelInputId] = &[
    bcls::EMAC_V4::CLIENTEMAC0RXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC0TXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC1RXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC1TXCLIENTCLKIN,
    bcls::EMAC_V4::DCREMACCLK,
    bcls::EMAC_V4::HOSTCLK,
    bcls::EMAC_V4::PHYEMAC0GTXCLK,
    bcls::EMAC_V4::PHYEMAC0MCLKIN,
    bcls::EMAC_V4::PHYEMAC0MIITXCLK,
    bcls::EMAC_V4::PHYEMAC0RXCLK,
    bcls::EMAC_V4::PHYEMAC0TXGMIIMIICLKIN,
    bcls::EMAC_V4::PHYEMAC1GTXCLK,
    bcls::EMAC_V4::PHYEMAC1MCLKIN,
    bcls::EMAC_V4::PHYEMAC1MIITXCLK,
    bcls::EMAC_V4::PHYEMAC1RXCLK,
    bcls::EMAC_V4::PHYEMAC1TXGMIIMIICLKIN,
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::EMAC) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::EMAC);
    let mode = "TEMAC";
    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();

    for &pin in EMAC_INVPINS {
        bctx.mode(mode).test_bel_input_inv_auto(pin);
    }
    for (aid, _, attr) in &backend.edev.db[bcls::EMAC_V4].attributes {
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
    let tcid = tcls::EMAC;
    let bslot = bslots::EMAC;
    if !ctx.has_tcls(tcid) {
        return;
    }
    for &pin in EMAC_INVPINS {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    for (aid, _, attr) in &ctx.edev.db[bcls::EMAC_V4].attributes {
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
