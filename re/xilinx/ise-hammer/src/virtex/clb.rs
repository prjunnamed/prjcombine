use prjcombine_re_collector::diff::{Diff, xlat_bit, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex::defs::{bcls::SLICE, bslots, enums, tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::FuzzCtx,
    virtex::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CLB);
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::SLICE[i]);
        let mode = "SLICE";
        // inverters
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .pin("CLK")
            .test_bel_input_inv_enum("CKINV", SLICE::CLK, "1", "0");
        for (val, vname) in [(false, "1"), (true, "0"), (false, "CE"), (true, "CE_B")] {
            bctx.mode(mode)
                .attr("FFX", "#FF")
                .attr("CKINV", "1")
                .pin("CE")
                .pin("CLK")
                .pin("XQ")
                .test_bel_input_inv(SLICE::CE, val)
                .attr("CEMUX", vname)
                .commit();
        }
        for (val, vname) in [(false, "1"), (true, "0"), (false, "SR"), (true, "SR_B")] {
            bctx.mode(mode)
                .attr("F", "#LUT:0")
                .attr("DXMUX", "1")
                .attr("SRFFMUX", "0")
                .attr("CEMUX", "0")
                .attr("FFX", "#FF")
                .attr("FFY", "#FF")
                .attr("CKINV", "1")
                .pin("SR")
                .pin("CLK")
                .pin("XQ")
                .test_bel_input_inv(SLICE::SR, val)
                .attr("SRMUX", vname)
                .commit();
        }
        for (val, vname) in [(false, "1"), (true, "0"), (false, "BX"), (true, "BX_B")] {
            bctx.mode(mode)
                .attr("FFX", "#FF")
                .attr("DXMUX", "0")
                .pin("BX")
                .pin("XQ")
                .test_bel_input_inv(SLICE::BX, val)
                .attr("BXMUX", vname)
                .commit();
        }
        for (val, vname) in [(false, "1"), (true, "0"), (false, "BY"), (true, "BY_B")] {
            bctx.mode(mode)
                .attr("FFY", "#FF")
                .attr("DYMUX", "0")
                .pin("BY")
                .pin("YQ")
                .test_bel_input_inv(SLICE::BY, val)
                .attr("BYMUX", vname)
                .commit();
        }

        // LUT
        bctx.mode(mode)
            .test_bel_attr_multi(SLICE::F, MultiValue::Lut);
        bctx.mode(mode)
            .test_bel_attr_multi(SLICE::G, MultiValue::Lut);
        for (spec, val) in [
            (specials::SLICE_RAMCONFIG_16X1, "16X1"),
            (specials::SLICE_RAMCONFIG_16X2, "16X2"),
            (specials::SLICE_RAMCONFIG_16X1DP, "16X1DP"),
            (specials::SLICE_RAMCONFIG_32X1, "32X1"),
            (specials::SLICE_RAMCONFIG_1SHIFT, "1SHIFT"),
            (specials::SLICE_RAMCONFIG_2SHIFTS, "2SHIFTS"),
        ] {
            bctx.mode(mode)
                .test_bel_special(spec)
                .attr("RAMCONFIG", val)
                .commit();
        }

        // carry chain
        bctx.mode(mode)
            .attr("BXMUX", "BX")
            .attr("CYSELF", "1")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("CIN")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr_auto(SLICE::CYINIT);
        bctx.mode(mode)
            .attr("F", "#LUT:0")
            .attr("CY0F", "0")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr_auto(SLICE::CYSELF);
        bctx.mode(mode)
            .attr("G", "#LUT:0")
            .attr("CY0G", "0")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("CYSELF", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr_auto(SLICE::CYSELG);
        for (val, vf, vg) in [
            (enums::SLICE_CY0::CONST_0, "0", "0"),
            (enums::SLICE_CY0::CONST_1, "1", "1"),
            (enums::SLICE_CY0::F1_G1, "F1", "G1"),
            (enums::SLICE_CY0::PROD, "PROD", "PROD"),
        ] {
            bctx.mode(mode)
                .attr("CYINIT", "BX")
                .attr("BXMUX", "BX")
                .attr("FXMUX", "FXOR")
                .attr("F", "#LUT:0")
                .attr("XUSED", "0")
                .attr("CYSELF", "F")
                .attr("CYSELG", "1")
                .attr("COUTUSED", "0")
                .pin("F1")
                .pin("F2")
                .pin("BX")
                .pin("X")
                .pin("COUT")
                .test_bel_attr_val(SLICE::CY0, val)
                .attr("CY0F", vf)
                .commit();
            bctx.mode(mode)
                .attr("CYINIT", "BX")
                .attr("BXMUX", "BX")
                .attr("BYMUX", "BY")
                .attr("GYMUX", "GXOR")
                .attr("G", "#LUT:0")
                .attr("YUSED", "0")
                .attr("CYSELF", "1")
                .attr("CYSELG", "G")
                .attr("COUTUSED", "0")
                .pin("G1")
                .pin("G2")
                .pin("BX")
                .pin("BY")
                .pin("Y")
                .pin("COUT")
                .test_bel_attr_val(SLICE::CY0, val)
                .attr("CY0G", vg)
                .commit();
        }

        // muxes
        for (val, vname) in [
            (enums::SLICE_YBMUX::GCY, "1"),
            (enums::SLICE_YBMUX::BY, "0"),
        ] {
            bctx.mode(mode)
                .attr("CYINIT", "BX")
                .attr("BXMUX", "BX")
                .attr("BYMUX", "BY")
                .attr("GYMUX", "GXOR")
                .attr("G", "#LUT:0")
                .attr("YUSED", "0")
                .attr("CYSELF", "1")
                .attr("CYSELG", "1")
                .attr("COUTUSED", "0")
                .pin("BX")
                .pin("BY")
                .pin("Y")
                .pin("YB")
                .pin("COUT")
                .test_bel_attr_val(SLICE::YBMUX, val)
                .attr("YBMUX", vname)
                .commit();
        }
        for (val, vname) in [(enums::SLICE_DXMUX::X, "1"), (enums::SLICE_DXMUX::BX, "0")] {
            bctx.mode(mode)
                .attr("F", "#LUT:0")
                .attr("XUSED", "0")
                .attr("FXMUX", "F")
                .attr("FFX", "#FF")
                .attr("BXMUX", "BX")
                .pin("X")
                .pin("XQ")
                .pin("BX")
                .test_bel_attr_val(SLICE::DXMUX, val)
                .attr("DXMUX", vname)
                .commit();
        }
        for (val, vname) in [(enums::SLICE_DYMUX::Y, "1"), (enums::SLICE_DYMUX::BY, "0")] {
            bctx.mode(mode)
                .attr("G", "#LUT:0")
                .attr("YUSED", "0")
                .attr("GYMUX", "G")
                .attr("FFY", "#FF")
                .attr("BYMUX", "BY")
                .pin("Y")
                .pin("YQ")
                .pin("BY")
                .test_bel_attr_val(SLICE::DYMUX, val)
                .attr("DYMUX", vname)
                .commit();
        }
        bctx.mode(mode)
            .attr("F", "#LUT:0")
            .attr("CYSELF", "1")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("XUSED", "0")
            .attr("COUTUSED", "0")
            .pin("X")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr_auto(SLICE::FXMUX);
        bctx.mode(mode)
            .attr("G", "#LUT:0")
            .attr("CYSELF", "1")
            .attr("CYSELG", "1")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("YUSED", "0")
            .attr("COUTUSED", "0")
            .pin("Y")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr_auto(SLICE::GYMUX);

        // FFs
        bctx.mode(mode)
            .pin("XQ")
            .attr("FFX", "#FF")
            .test_bel_attr_bool_rename("SYNC_ATTR", SLICE::FF_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode(mode)
            .attr("FFY", "")
            .attr("CEMUX", "CE_B")
            .attr("INITX", "LOW")
            .pin("XQ")
            .pin("CE")
            .test_bel_attr_bool_rename("FFX", SLICE::FF_LATCH, "#FF", "#LATCH");
        bctx.mode(mode)
            .attr("FFX", "")
            .attr("CEMUX", "CE_B")
            .attr("INITY", "LOW")
            .pin("YQ")
            .pin("CE")
            .test_bel_attr_bool_rename("FFY", SLICE::FF_LATCH, "#FF", "#LATCH");
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .pin("XQ")
            .test_bel_attr_bool_rename("INITX", SLICE::FFX_INIT, "LOW", "HIGH");
        bctx.mode(mode)
            .attr("FFY", "#FF")
            .pin("YQ")
            .test_bel_attr_bool_rename("INITY", SLICE::FFY_INIT, "LOW", "HIGH");
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .attr("BYMUX", "BY")
            .pin("XQ")
            .pin("BY")
            .test_bel_attr_bits(SLICE::FF_REV_ENABLE)
            .attr("REVUSED", "0")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CLB;
    for bslot in bslots::SLICE {
        ctx.collect_bel_input_inv_bi(tcid, bslot, SLICE::CLK);
        ctx.collect_bel_input_inv_bi(tcid, bslot, SLICE::SR);
        ctx.collect_bel_input_inv_bi(tcid, bslot, SLICE::CE);
        ctx.collect_bel_input_inv_bi(tcid, bslot, SLICE::BX);
        ctx.collect_bel_input_inv_bi(tcid, bslot, SLICE::BY);
        ctx.collect_bel_attr(tcid, bslot, SLICE::F);
        ctx.collect_bel_attr(tcid, bslot, SLICE::G);
        ctx.collect_bel_attr(tcid, bslot, SLICE::CYINIT);
        ctx.collect_bel_attr(tcid, bslot, SLICE::CYSELF);
        ctx.collect_bel_attr(tcid, bslot, SLICE::CYSELG);
        ctx.collect_bel_attr(tcid, bslot, SLICE::CY0);
        ctx.collect_bel_attr(tcid, bslot, SLICE::YBMUX);
        ctx.collect_bel_attr(tcid, bslot, SLICE::DXMUX);
        ctx.collect_bel_attr(tcid, bslot, SLICE::DYMUX);
        ctx.collect_bel_attr(tcid, bslot, SLICE::FXMUX);
        ctx.collect_bel_attr(tcid, bslot, SLICE::GYMUX);
        ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FF_SR_SYNC);
        ctx.collect_bel_attr(tcid, bslot, SLICE::FF_REV_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FF_LATCH);
        ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FFX_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, SLICE::FFY_INIT);

        let diff_16x1 = ctx.get_diff_bel_special(tcid, bslot, specials::SLICE_RAMCONFIG_16X1);
        let diff_16x2 = ctx.get_diff_bel_special(tcid, bslot, specials::SLICE_RAMCONFIG_16X2);
        let diff_16x1dp = ctx.get_diff_bel_special(tcid, bslot, specials::SLICE_RAMCONFIG_16X1DP);
        let diff_32x1 = ctx.get_diff_bel_special(tcid, bslot, specials::SLICE_RAMCONFIG_32X1);
        let diff_1shift = ctx.get_diff_bel_special(tcid, bslot, specials::SLICE_RAMCONFIG_1SHIFT);
        let diff_2shifts = ctx.get_diff_bel_special(tcid, bslot, specials::SLICE_RAMCONFIG_2SHIFTS);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            SLICE::WA4_ENABLE,
            xlat_bit(diff_32x1.combine(&!&diff_16x1dp)),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            SLICE::F_SHIFT_ENABLE,
            xlat_bit(diff_2shifts.combine(&!&diff_1shift)),
        );
        let f_ram_en = xlat_bit(diff_16x2.combine(&!&diff_16x1));
        ctx.insert_bel_attr_bool(tcid, bslot, SLICE::F_RAM_ENABLE, f_ram_en);
        let diff_dif_bx = diff_16x2.combine(&!&diff_16x1dp);
        let diff_1shift = diff_1shift.combine(&!&diff_dif_bx);
        let diff_16x1 = diff_16x1.combine(&!&diff_dif_bx);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            SLICE::DIF_MUX,
            xlat_enum_attr(vec![
                (enums::SLICE_DIF_MUX::BY, Diff::default()),
                (enums::SLICE_DIF_MUX::BX, diff_dif_bx),
            ]),
        );
        let (diff_g_shift, diff_g_ram, common) =
            Diff::split(diff_1shift.clone(), diff_16x1.clone());
        ctx.insert_bel_attr_bool(tcid, bslot, SLICE::FF_SR_ENABLE, !xlat_bit(common));
        ctx.insert_bel_attr_bool(tcid, bslot, SLICE::G_SHIFT_ENABLE, xlat_bit(diff_g_shift));
        ctx.insert_bel_attr_bool(tcid, bslot, SLICE::G_RAM_ENABLE, xlat_bit(diff_g_ram));
    }
    // extracted manually from .ll
    for (bslot, attr, frame, bit) in [
        (bslots::SLICE[0], SLICE::FFX_READBACK, 45, 16),
        (bslots::SLICE[0], SLICE::FFY_READBACK, 39, 16),
        (bslots::SLICE[1], SLICE::FFX_READBACK, 2, 16),
        (bslots::SLICE[1], SLICE::FFY_READBACK, 8, 16),
    ] {
        ctx.insert_bel_attr_bool(tcid, bslot, attr, TileBit::new(0, frame, bit).pos());
    }
}
