use prjcombine_re_fpga_hammer::diff::xlat_enum_attr;
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc5200::{bcls, bslots, enums, tcls};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx, specials};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let tcid = backend.edev.db.get_tile_class("CLB");
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    let mut bctx = ctx.bel(bslots::LC[0]);
    bctx.mode("CLB")
        .null_bits()
        .test_bel_special(specials::CLB_CO_USED)
        .cfg("CO", "USED")
        .commit();
    for (val, rval) in [(false, "GND"), (true, "VCC")] {
        bctx.mode("CLB")
            .mutex("CV", rval)
            .test_other_bel_attr_enum_bool(bslots::PROGTIE, bcls::PROGTIE::VAL, val)
            .cfg("CV", rval)
            .commit();
    }
    for i in 0..4 {
        let bslot = bslots::LC[i];
        bctx.mode("CLB").test_other_bel_attr_equate(
            bslot,
            bcls::LC::LUT,
            format!("LC{i}.F"),
            match i {
                0 => &["LC0.F1", "LC0.F2", "LC0.F3", "LC0.F4"],
                1 => &["LC1.F1", "LC1.F2", "LC1.F3", "LC1.F4"],
                2 => &["LC2.F1", "LC2.F2", "LC2.F3", "LC2.F4"],
                3 => &["LC3.F1", "LC3.F2", "LC3.F3", "LC3.F4"],
                _ => unreachable!(),
            },
        );
        bctx.mode("CLB")
            .mutex("RDBK", format!("LC{i}.QX"))
            .test_other_bel_attr_bits(bslot, bcls::LC::READBACK)
            .cfg("RDBK", format!("LC{i}.QX"))
            .commit();
        bctx.mode("CLB")
            .null_bits()
            .test_other_bel_special(bslot, specials::LC_DO_XBI)
            .cfg(format!("LC{i}.DO"), format!("LC{i}.XBI"))
            .commit();
        bctx.mode("CLB")
            .null_bits()
            .test_other_bel_special(bslot, specials::LC_X_F)
            .cfg(format!("LC{i}.X"), format!("LC{i}.F"))
            .commit();
        bctx.mode("CLB")
            .mutex(format!("LC{i}.DX"), format!("LC{i}.F"))
            .test_other_bel_attr_val(bslot, bcls::LC::MUX_D, enums::LC_MUX_D::F)
            .cfg(format!("LC{i}.DX"), format!("LC{i}.F"))
            .commit();
        bctx.mode("CLB")
            .mutex(format!("LC{i}.DX"), format!("LC{i}.XBI"))
            .test_other_bel_attr_val(bslot, bcls::LC::MUX_D, enums::LC_MUX_D::DO)
            .cfg(format!("LC{i}.DX"), format!("LC{i}.XBI"))
            .commit();
        bctx.mode("CLB")
            .test_other_bel_attr_bits(bslot, bcls::LC::CLR_ENABLE)
            .cfg(format!("LC{i}.FFX"), "CLR")
            .commit();
        bctx.mode("CLB")
            .test_other_bel_attr_bits(bslot, bcls::LC::CE_ENABLE)
            .cfg(format!("LC{i}.FFX"), "CE")
            .commit();
        bctx.mode("CLB")
            .mutex(format!("LC{i}.FFX"), "FF")
            .test_other_bel_input_inv(bslot, bcls::LC::CK, true)
            .cfg(format!("LC{i}.FFX"), "NOTK")
            .commit();
        bctx.mode("CLB")
            .test_other_bel_attr_as(bslot, format!("LC{i}.FFX"), bcls::LC::FF_MODE);
        bctx.mode("CLB")
            .mutex(format!("LC{i}.XBI"), format!("LC{i}.DI"))
            .test_other_bel_attr_val(bslot, bcls::LC::MUX_DO, enums::LC_MUX_DO::DI)
            .cfg(format!("LC{i}.XBI"), format!("LC{i}.DI"))
            .commit();
        bctx.mode("CLB")
            .mutex(format!("LC{i}.XBI"), format!("LC{i}.CARRY"))
            .test_other_bel_attr_val(bslot, bcls::LC::MUX_DO, enums::LC_MUX_DO::CO)
            .cfg(format!("LC{i}.XBI"), format!("LC{i}.CARRY"))
            .commit();
        if matches!(i, 0 | 2) {
            bctx.mode("CLB")
                .mutex(format!("LC{i}.XBI"), format!("LC{i}{ii}.F5", ii = i + 1))
                .test_other_bel_attr_val(bslot, bcls::LC::MUX_DO, enums::LC_MUX_DO::F5O)
                .cfg(format!("LC{i}.XBI"), format!("LC{i}{ii}.F5", ii = i + 1))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CLB;
    for i in 0..4 {
        let bslot = bslots::LC[i];
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::LUT);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::READBACK);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::CE_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::CLR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::LC::MUX_D);
        ctx.collect_bel_input_inv(tcid, bslot, bcls::LC::CK);
        let diff_ff = ctx.get_diff_attr_val(tcid, bslot, bcls::LC::FF_MODE, enums::FF_MODE::FF);
        let mut diff_latch =
            ctx.get_diff_attr_val(tcid, bslot, bcls::LC::FF_MODE, enums::FF_MODE::LATCH);
        diff_latch.apply_bit_diff_raw(ctx.bel_input_inv(tcid, bslot, bcls::LC::CK), true, false);
        ctx.insert_bel_attr_raw(
            tcid,
            bslot,
            bcls::LC::FF_MODE,
            xlat_enum_attr(vec![
                (enums::FF_MODE::FF, diff_ff),
                (enums::FF_MODE::LATCH, diff_latch),
            ]),
        );
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
    }
    ctx.collect_bel_attr_enum_bool(tcid, bslots::PROGTIE, bcls::PROGTIE::VAL)
}
