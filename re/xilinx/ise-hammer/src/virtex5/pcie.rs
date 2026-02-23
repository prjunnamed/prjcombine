use prjcombine_interconnect::db::{BelAttributeType, BelInputId};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{bcls, bslots, virtex5::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

const PCIE_INVPINS: &[BelInputId] = &[
    bcls::PCIE_V5::CRMCORECLK,
    bcls::PCIE_V5::CRMCORECLKDLO,
    bcls::PCIE_V5::CRMCORECLKRXO,
    bcls::PCIE_V5::CRMCORECLKTXO,
    bcls::PCIE_V5::CRMUSERCLK,
    bcls::PCIE_V5::CRMUSERCLKRXO,
    bcls::PCIE_V5::CRMUSERCLKTXO,
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::PCIE) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::PCIE);
    let mode = "PCIE";

    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();

    for &pin in PCIE_INVPINS {
        bctx.mode(mode).test_bel_input_inv_auto(pin);
    }
    for (aid, _, attr) in &backend.edev.db[bcls::PCIE_V5].attributes {
        match attr.typ {
            BelAttributeType::Bool => {
                bctx.mode(mode)
                    .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
            }
            BelAttributeType::BitVec(_width) => match aid {
                bcls::PCIE_V5::TXTSNFTS | bcls::PCIE_V5::TXTSNFTSCOMCLK => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }
                _ => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }
            },
            _ => unreachable!(),
        }
    }

    for val in ["FALSE", "TRUE"] {
        bctx.mode(mode)
            .null_bits()
            .test_bel_special(specials::PCIE_CLKDIVIDED)
            .attr("CLKDIVIDED", val)
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::PCIE;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let bslot = bslots::PCIE;
    for &pin in PCIE_INVPINS {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    for (aid, _, attr) in &ctx.edev.db[bcls::PCIE_V5].attributes {
        match attr.typ {
            BelAttributeType::Bool => {
                ctx.collect_bel_attr_bi(tcid, bslot, aid);
            }
            BelAttributeType::BitVec(_width) => {
                ctx.collect_bel_attr(tcid, bslot, aid);
            }
            _ => unreachable!(),
        }
    }
}
