use prjcombine_interconnect::db::{BelAttributeType, BelInputId};
use prjcombine_re_collector::diff::extract_bitvec_val;
use prjcombine_re_hammer::Session;
use prjcombine_types::bits;
use prjcombine_virtex4::defs::{bcls, bslots, devdata, virtex5::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

const PPC_INVPINS: &[BelInputId] = &[
    bcls::PPC440::CPMC440CLK,
    bcls::PPC440::CPMC440TIMERCLOCK,
    bcls::PPC440::CPMDCRCLK,
    bcls::PPC440::CPMDMA0LLCLK,
    bcls::PPC440::CPMDMA1LLCLK,
    bcls::PPC440::CPMDMA2LLCLK,
    bcls::PPC440::CPMDMA3LLCLK,
    bcls::PPC440::CPMFCMCLK,
    bcls::PPC440::CPMINTERCONNECTCLK,
    bcls::PPC440::CPMMCCLK,
    bcls::PPC440::CPMPPCMPLBCLK,
    bcls::PPC440::CPMPPCS0PLBCLK,
    bcls::PPC440::CPMPPCS1PLBCLK,
    bcls::PPC440::JTGC440TCK,
];

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::PPC) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::PPC);
    let mode = "PPC440";

    if !devdata_only {
        bctx.build()
            .null_bits()
            .no_global("PPCCLKDLY")
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();

        for &pin in PPC_INVPINS {
            bctx.mode(mode)
                .no_global("PPCCLKDLY")
                .test_bel_input_inv_auto(pin);
        }

        for (aid, _, attr) in &backend.edev.db[bcls::PPC440].attributes {
            match attr.typ {
                BelAttributeType::Bool => {
                    bctx.mode(mode)
                        .no_global("PPCCLKDLY")
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }
                BelAttributeType::BitVec(_width) => {
                    if aid == bcls::PPC440::CLOCK_DELAY {
                        bctx.mode(mode)
                            .attr("CLOCK_DELAY", "TRUE")
                            .test_bel_attr_bits(bcls::PPC440::CLOCK_DELAY)
                            .multi_global("PPCCLKDLY", MultiValue::Bin, 5);
                    } else {
                        bctx.mode(mode)
                            .no_global("PPCCLKDLY")
                            .test_bel_attr_multi(aid, MultiValue::Hex(0));
                    }
                }
                _ => unreachable!(),
            }
        }

        for val in ["FALSE", "TRUE"] {
            bctx.mode(mode)
                .null_bits()
                .no_global("PPCCLKDLY")
                .test_bel_special(specials::PPC_MI_CONTROL_BIT6)
                .attr("MI_CONTROL_BIT6", val)
                .commit();
        }

        for (val, vname) in [(false, "FALSE"), (true, "TRUE")] {
            bctx.mode(mode)
                .no_global("PPCCLKDLY")
                .test_bel_attr_special_bits_bi(
                    bcls::PPC440::CLOCK_DELAY,
                    specials::PPC_CLOCK_DELAY,
                    0,
                    val,
                )
                .attr("CLOCK_DELAY", vname)
                .commit();
        }
    } else {
        bctx.mode(mode)
            .no_global("PPCCLKDLY")
            .test_bel_attr_special_bits_bi(
                bcls::PPC440::CLOCK_DELAY,
                specials::PPC_CLOCK_DELAY,
                0,
                false,
            )
            .attr("CLOCK_DELAY", "FALSE")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tcid = tcls::PPC;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let bslot = bslots::PPC;
    if !devdata_only {
        for &pin in PPC_INVPINS {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        for (aid, _, attr) in &ctx.edev.db[bcls::PPC440].attributes {
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

        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::PPC440::CLOCK_DELAY,
            specials::PPC_CLOCK_DELAY,
            0,
            true,
        )
        .assert_empty();
    }
    let diff = ctx.get_diff_attr_special_bit_bi(
        tcid,
        bslot,
        bcls::PPC440::CLOCK_DELAY,
        specials::PPC_CLOCK_DELAY,
        0,
        false,
    );
    let val = extract_bitvec_val(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PPC440::CLOCK_DELAY),
        &bits![0; 5],
        diff,
    );
    ctx.insert_devdata_bitvec(devdata::PPC440_CLOCK_DELAY, val);
}
