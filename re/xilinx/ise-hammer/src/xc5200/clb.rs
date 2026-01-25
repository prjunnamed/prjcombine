use prjcombine_re_collector::diff::{Diff, DiffKey, xlat_bit_bi, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::xc5200::{bcls, bslots, enums, tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    xc5200::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new_id(session, backend, tcls::CLB);
    for i in 0..4 {
        let mut bctx = ctx.bel(bslots::LC[i]);
        let mode = if i.is_multiple_of(2) { "LC5A" } else { "LC5B" };
        bctx.mode(mode)
            .test_bel_attr_multi(bcls::LC::LUT, MultiValue::OldLut('F'));
        bctx.mode(mode)
            .attr("DMUX", "F")
            .pin("DI")
            .test_bel_attr_val(bcls::LC::FF_MODE, enums::FF_MODE::LATCH)
            .attr("FFLATCH", "#LATCH")
            .pin("Q")
            .commit();
        for (val, vname) in &backend.edev.db[enums::LC_MUX_DO].values {
            if val == enums::LC_MUX_DO::F5O && !i.is_multiple_of(2) {
                continue;
            }
            bctx.mode(mode)
                .pin("DO")
                .pin("DI")
                .test_bel_attr_val(bcls::LC::MUX_DO, val)
                .attr("DOMUX", vname)
                .commit();
        }
        bctx.mode(mode)
            .pin("DI")
            .attr("DOMUX", "DI")
            .test_bel_attr_rename("DMUX", bcls::LC::MUX_D);
        bctx.mode(mode)
            .pin("CE")
            .test_bel_attr_bits(bcls::LC::CE_ENABLE)
            .attr("CEMUX", "CE")
            .commit();
        bctx.mode(mode)
            .pin("CLR")
            .test_bel_attr_bits(bcls::LC::CLR_ENABLE)
            .attr("CLRMUX", "CLR")
            .commit();
        for (val, rval) in [(false, "CK"), (true, "CKNOT")] {
            bctx.mode(mode)
                .pin("CK")
                .pin("DI")
                .pin("Q")
                .attr("DMUX", "F")
                .attr("FFLATCH", "")
                .test_bel_input_inv(bcls::LC::CK, val)
                .attr("CKMUX", rval)
                .commit();
        }
        for (spec, rval) in [(specials::CK_LATCH, "CK"), (specials::CKNOT_LATCH, "CKNOT")] {
            bctx.mode(mode)
                .pin("CK")
                .pin("DI")
                .pin("Q")
                .attr("DMUX", "F")
                .attr("FFLATCH", "#LATCH")
                .test_bel_special(spec)
                .attr("CKMUX", rval)
                .commit();
        }
        bctx.mode(mode)
            .null_bits()
            .pin("CO")
            .test_bel_special(specials::COMUX)
            .attr("COMUX", "CY")
            .commit();
    }
    for i in 0..4 {
        let mut bctx = ctx.bel(bslots::TBUF[i]);
        bctx.mode("TBUF")
            .test_bel_attr_bits(bcls::TBUF::T_ENABLE)
            .pin("T")
            .pin_pips("T")
            .commit();
    }
    let mut bctx = ctx.bel(bslots::PROGTIE);
    bctx.mode("VCC_GND")
        .test_bel_attr_bool_rename("MUX", bcls::PROGTIE::VAL, "0", "1");
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CLB;
    for i in 0..4 {
        let bslot = bslots::LC[i];
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::LUT);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::CE_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::CLR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::MUX_D);
        let item = xlat_enum_attr(vec![
            (enums::FF_MODE::FF, Diff::default()),
            (
                enums::FF_MODE::LATCH,
                ctx.get_diff_attr_val(tcid, bslot, bcls::LC::FF_MODE, enums::FF_MODE::LATCH)
                    .combine(&!ctx.peek_diff_raw(&DiffKey::BelInputInv(
                        tcid,
                        bslot,
                        bcls::LC::CK,
                        true,
                    ))),
            ),
        ]);
        ctx.insert_bel_attr_raw(tcid, bslot, bcls::LC::FF_MODE, item);
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::LC::CK);
        let diff0 = ctx.get_diff_bel_special(tcid, bslot, specials::CKNOT_LATCH);
        let diff1 = ctx.get_diff_bel_special(tcid, bslot, specials::CK_LATCH);
        ctx.insert_bel_input_inv(tcid, bslot, bcls::LC::CK, xlat_bit_bi(diff0, diff1));
        let mut diffs = vec![
            (
                enums::LC_MUX_DO::DI,
                ctx.get_diff_attr_val(tcid, bslot, bcls::LC::MUX_DO, enums::LC_MUX_DO::DI),
            ),
            (
                enums::LC_MUX_DO::CO,
                ctx.get_diff_attr_val(tcid, bslot, bcls::LC::MUX_DO, enums::LC_MUX_DO::CO),
            ),
        ];
        if i.is_multiple_of(2) {
            diffs.push((
                enums::LC_MUX_DO::F5O,
                ctx.get_diff_attr_val(tcid, bslot, bcls::LC::MUX_DO, enums::LC_MUX_DO::F5O),
            ));
        }
        ctx.insert_bel_attr_raw(tcid, bslot, bcls::LC::MUX_DO, xlat_enum_attr(diffs));
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::LC::READBACK,
            TileBit::new(0, 1, [5, 10, 23, 28][i]).neg(),
        );
    }
    for i in 0..4 {
        ctx.collect_bel_attr(tcid, bslots::TBUF[i], bcls::TBUF::T_ENABLE);
    }
    ctx.collect_bel_attr_bool_bi(tcid, bslots::PROGTIE, bcls::PROGTIE::VAL)
}
