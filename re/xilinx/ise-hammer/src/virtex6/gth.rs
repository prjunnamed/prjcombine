use prjcombine_interconnect::db::{BelAttributeType, BelInputId};
use prjcombine_re_collector::diff::{OcdMode, xlat_bitvec};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls::{self, GTH_QUAD},
    bslots, enums,
    virtex6::tcls,
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{pip::PinFar, relation::Delta},
    },
    virtex4::specials,
};

const GTH_INVPINS: &[BelInputId] = &[
    GTH_QUAD::DCLK,
    GTH_QUAD::SCANCLK,
    GTH_QUAD::SDSSCANCLK,
    GTH_QUAD::TPCLK,
    GTH_QUAD::TSTNOISECLK,
    GTH_QUAD::RXUSERCLKIN0,
    GTH_QUAD::RXUSERCLKIN1,
    GTH_QUAD::RXUSERCLKIN2,
    GTH_QUAD::RXUSERCLKIN3,
    GTH_QUAD::TXUSERCLKIN0,
    GTH_QUAD::TXUSERCLKIN1,
    GTH_QUAD::TXUSERCLKIN2,
    GTH_QUAD::TXUSERCLKIN3,
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::GTH) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::GTH_QUAD);
    let mode = "GTHE1_QUAD";
    bctx.build()
        .extra_tile_bel_special(
            Delta::new(0, 0, tcls::HCLK),
            bslots::HCLK_DRP[0],
            specials::DRP_MASK_GTH,
        )
        .test_bel_attr_bits(GTH_QUAD::ENABLE)
        .mode(mode)
        .commit();

    for &pin in GTH_INVPINS {
        bctx.mode(mode).test_bel_input_inv_auto(pin);
    }

    for (aid, aname, attr) in &backend.edev.db[GTH_QUAD].attributes {
        match aid {
            GTH_QUAD::DRP | GTH_QUAD::ENABLE | GTH_QUAD::MUX_REFCLK => (),
            _ => match attr.typ {
                BelAttributeType::Bool => {
                    bctx.mode(mode)
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }
                BelAttributeType::Enum(enums::GTH_QUAD_FABRIC_WIDTH) => {
                    for (val, vname) in [
                        (enums::GTH_QUAD_FABRIC_WIDTH::_16_20, "8"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_16_20, "10"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_16_20, "16"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_16_20, "20"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_32, "32"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_40, "40"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_64, "64"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_80, "80"),
                        (enums::GTH_QUAD_FABRIC_WIDTH::_64_66, "6466"),
                    ] {
                        bctx.mode(mode)
                            .test_bel_attr_val(aid, val)
                            .attr(aname, vname)
                            .commit();
                    }
                }
                BelAttributeType::Enum(_) => {
                    bctx.mode(mode).test_bel_attr_auto(aid);
                }
                BelAttributeType::BitVec(_width) => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }
                _ => unreachable!(),
            },
        }
    }

    for (val, pin) in [
        (enums::GTH_QUAD_MUX_REFCLK::GREFCLK, "GREFCLK"),
        (enums::GTH_QUAD_MUX_REFCLK::REFCLK_IN, "REFCLK_IN"),
        (enums::GTH_QUAD_MUX_REFCLK::REFCLK_SOUTH, "REFCLK_SOUTH"),
        (enums::GTH_QUAD_MUX_REFCLK::REFCLK_NORTH, "REFCLK_NORTH"),
    ] {
        bctx.mode(mode)
            .mutex("MUX.REFCLK", pin)
            .attr("PLL_CFG2", "")
            .test_bel_attr_val(GTH_QUAD::MUX_REFCLK, val)
            .pip((PinFar, "REFCLK"), pin)
            .commit();
    }

    let mut bctx = ctx.bel(bslots::GTH_QUAD).sub(1);
    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode("IBUFDS_GTHE1")
        .commit();
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::GTH;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let bslot = bslots::GTH_QUAD;
    fn drp_bit(idx: usize, bit: usize) -> TileBit {
        let tile = idx >> 3;
        let frame = 28 + (bit & 1);
        let bit = (bit >> 1) | (idx & 7) << 3;
        TileBit::new(tile, frame, bit)
    }
    let mut drp = vec![];
    for addr in 0..0x140 {
        for bit in 0..16 {
            drp.push(drp_bit(addr, bit).pos());
        }
    }
    ctx.insert_bel_attr_bitvec(tcid, bslot, GTH_QUAD::DRP, drp);

    for &pin in GTH_INVPINS {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    ctx.collect_bel_attr(tcid, bslot, GTH_QUAD::ENABLE);

    for (aid, _, attr) in &ctx.edev.db[GTH_QUAD].attributes {
        if matches!(aid, GTH_QUAD::DRP | GTH_QUAD::ENABLE) {
            continue;
        }
        if aid == GTH_QUAD::MUX_REFCLK {
            ctx.collect_bel_attr_default(tcid, bslot, aid, enums::GTH_QUAD_MUX_REFCLK::NONE);
            continue;
        }
        if aid == GTH_QUAD::SLICE_NOISE_CTRL_1_LANE01 {
            // AAAAAAAAAAAAAAARGH
            let mut diffs = ctx.get_diffs_attr_bits(tcid, bslot, aid, 16);
            let bit = TileBit::new(12, 29, 32);
            assert_eq!(diffs[1].bits.len(), 0);
            assert_eq!(diffs[2].bits.len(), 2);
            diffs[1].bits.insert(bit, true);
            assert_eq!(diffs[2].bits.remove(&bit), Some(true));
            ctx.insert_bel_attr_bitvec(tcid, bslot, aid, xlat_bitvec(diffs));
            continue;
        }
        match attr.typ {
            BelAttributeType::Bool => {
                ctx.collect_bel_attr_bi(tcid, bslot, aid);
            }
            BelAttributeType::BitVec(_) => {
                ctx.collect_bel_attr(tcid, bslot, aid);
            }
            BelAttributeType::Enum(_) => {
                ctx.collect_bel_attr_ocd(tcid, bslot, aid, OcdMode::BitOrderDrpV6);
            }
            _ => unreachable!(),
        }
    }

    let tcid = tcls::HCLK;
    let bslot = bslots::HCLK_DRP[0];
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DRP_MASK_GTH);
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::HCLK_DRP::DRP_MASK_S),
        true,
        false,
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::HCLK_DRP::DRP_MASK_N),
        true,
        false,
    );
    diff.assert_empty();
}
