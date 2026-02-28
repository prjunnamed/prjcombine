use prjcombine_interconnect::db::BelAttributeType;
use prjcombine_re_collector::diff::xlat_bit;
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{bcls, bcls::PCIE_V6 as PCIE, bslots, virtex6::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::Delta,
    },
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::PCIE) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::PCIE);
    let mode = "PCIE_2_0";

    bctx.build()
        .null_bits()
        .extra_tile_bel_special(
            Delta::new(3, 20, tcls::HCLK),
            bslots::HCLK_DRP[0],
            specials::DRP_MASK_PCIE,
        )
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();

    for (aid, _, attr) in &backend.edev.db[PCIE].attributes {
        if aid == PCIE::DRP {
            continue;
        }
        match attr.typ {
            BelAttributeType::Bool => {
                bctx.mode(mode)
                    .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
            }
            BelAttributeType::BitVec(_width) => {
                let multi = if matches!(
                    aid,
                    PCIE::N_FTS_COMCLK_GEN1
                        | PCIE::N_FTS_COMCLK_GEN2
                        | PCIE::N_FTS_GEN1
                        | PCIE::N_FTS_GEN2
                        | PCIE::PCIE_REVISION
                        | PCIE::VC0_TOTAL_CREDITS_CD
                        | PCIE::VC0_TOTAL_CREDITS_CH
                        | PCIE::VC0_TOTAL_CREDITS_NPH
                        | PCIE::VC0_TOTAL_CREDITS_PD
                        | PCIE::VC0_TOTAL_CREDITS_PH
                        | PCIE::VC0_TX_LASTPACKET
                ) {
                    MultiValue::Dec(0)
                } else {
                    MultiValue::Hex(0)
                };
                bctx.mode(mode).test_bel_attr_multi(aid, multi);
            }
            _ => unreachable!(),
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::PCIE;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let bslot = bslots::PCIE;

    fn pcie_drp_bit(reg: usize, bit: usize) -> TileBit {
        let tile = reg / 6;
        let frame = 26 + (bit & 1);
        let bit = (bit >> 1) | (reg % 6) << 3;
        TileBit::new(tile, frame, bit)
    }
    let mut drp = vec![];
    for reg in 0..0x78 {
        for bit in 0..16 {
            drp.push(pcie_drp_bit(reg, bit).pos());
        }
    }
    ctx.insert_bel_attr_bitvec(tcid, bslot, PCIE::DRP, drp);

    for (aid, _, attr) in &ctx.edev.db[PCIE].attributes {
        if aid == PCIE::DRP {
            continue;
        }
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

    let tcid = tcls::HCLK;
    let bslot = bslots::HCLK_DRP[0];
    let bit = xlat_bit(ctx.get_diff_bel_special(tcid, bslot, specials::DRP_MASK_PCIE));
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::HCLK_DRP::DRP_MASK_S, bit);
}
