use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, DiffKey, xlat_bit};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex2::{
    chip::ChipKind,
    defs::{bcls, bslots, enums, spartan3::tcls as tcls_s3, tslots, virtex2::tcls as tcls_v2},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    virtex2::specials,
};

#[derive(Clone, Debug)]
struct RandorInit;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for RandorInit {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        if tcrd.row != edev.chip.row_n() {
            return None;
        }
        if tcrd.col == edev.chip.col_w() + 1 {
            let tcrd = tcrd.delta(-1, 0).tile(tslots::RANDOR);
            let tile = &backend.edev[tcrd];
            let DiffKey::BelAttrValue(_, bslots::RANDOR, bcls::RANDOR::MODE, val) =
                fuzzer.info.features.first().unwrap().key
            else {
                unreachable!()
            };
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelAttrValue(
                    tile.class,
                    bslots::RANDOR_INIT,
                    bcls::RANDOR_INIT::MODE,
                    val,
                ),
                rects: backend.edev.tile_bits(tcrd),
            });
            Some((fuzzer, false))
        } else {
            Some((fuzzer, true))
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };

    let (bk_l, bk_m) = if edev.chip.kind.is_virtex2() {
        ("SLICE", "SLICE")
    } else {
        ("SLICEL", "SLICEM")
    };
    let mut ctx = FuzzCtx::new_id(
        session,
        backend,
        if edev.chip.kind.is_virtex2() {
            tcls_v2::CLB
        } else {
            tcls_s3::CLB
        },
    );
    for i in 0..4 {
        let mut bctx = ctx.bel(bslots::SLICE[i]);
        let is_m = edev.chip.kind.is_virtex2() || matches!(i, 0 | 2);

        // inverters
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .pin("XQ")
            .test_bel_input_inv_auto(bcls::SLICE::CE);
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .pin("XQ")
            .test_bel_input_inv_auto(bcls::SLICE::CLK);
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .attr("FFY", "#FF")
            .attr(
                "SRFFMUX",
                if edev.chip.kind.is_virtex2() { "0" } else { "" },
            )
            .pin("XQ")
            .pin("YQ")
            .test_bel_input_inv_auto(bcls::SLICE::SR);
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .attr("XUSED", "0")
            .attr("DXMUX", "0")
            .pin("X")
            .pin("XQ")
            .test_bel_input_inv_auto(bcls::SLICE::BX);
        bctx.mode(bk_l)
            .attr("FFY", "#FF")
            .attr("YUSED", "0")
            .attr("DYMUX", "0")
            .pin("Y")
            .pin("YQ")
            .test_bel_input_inv_auto(bcls::SLICE::BY);

        // LUT
        for attr in [bcls::SLICE::F, bcls::SLICE::G] {
            bctx.mode(bk_l).test_bel_attr_multi(attr, MultiValue::Lut);
        }

        // carry chain
        bctx.mode(bk_l)
            .attr("BXINV", "BX")
            .attr("CYSELF", "1")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("CIN")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr(bcls::SLICE::CYINIT);
        bctx.mode(bk_l)
            .attr("F", "#LUT:0")
            .attr("CY0F", "0")
            .attr("CYINIT", "BX")
            .attr("BXINV", "BX")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr(bcls::SLICE::CYSELF);
        bctx.mode(bk_l)
            .attr("G", "#LUT:0")
            .attr("CY0G", "0")
            .attr("CYINIT", "BX")
            .attr("BXINV", "BX")
            .attr("CYSELF", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("COUT")
            .test_bel_attr(bcls::SLICE::CYSELG);
        bctx.mode(bk_l)
            .attr("CYINIT", "BX")
            .attr("BXINV", "BX")
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
            .test_bel_attr(bcls::SLICE::CY0F);
        bctx.mode(bk_l)
            .attr("CYINIT", "BX")
            .attr("BXINV", "BX")
            .attr("BYINV", "BY")
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
            .test_bel_attr(bcls::SLICE::CY0G);

        // various muxes
        bctx.mode(bk_l)
            .attr("F", "#LUT:0")
            .attr("CYSELF", "1")
            .attr("CYINIT", "BX")
            .attr("BXINV", "BX")
            .attr("XUSED", "0")
            .pin("X")
            .pin("BX")
            .test_bel_attr(bcls::SLICE::FXMUX);
        if edev.chip.kind.is_virtex2() {
            for (val, vname) in [
                (enums::SLICE_GYMUX::G, "G"),
                (enums::SLICE_GYMUX::FX, "FX"),
                (enums::SLICE_GYMUX::GXOR, "GXOR"),
                (enums::SLICE_GYMUX::SOPOUT, "SOPEXT"),
            ] {
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("CYSELF", "1")
                    .attr("CYSELG", "1")
                    .attr("CYINIT", "BX")
                    .attr("BXINV", "BX")
                    .attr("YUSED", "0")
                    .attr("SOPEXTSEL", "SOPIN")
                    .attr("SOPOUTUSED", "0")
                    .pin("Y")
                    .pin("BX")
                    .test_bel_attr_val(bcls::SLICE::GYMUX, val)
                    .attr("GYMUX", vname)
                    .commit();
            }
            for (val, vname) in [(enums::SLICE_DXMUX::BX, "0"), (enums::SLICE_DXMUX::X, "1")] {
                bctx.mode(bk_l)
                    .attr("FFX", "#FF")
                    .attr("BXINV", "BX")
                    .pin("DX")
                    .pin("XQ")
                    .pin("BX")
                    .test_bel_attr_val(bcls::SLICE::DXMUX, val)
                    .attr("DXMUX", vname)
                    .commit();
            }
            for (val, vname) in [(enums::SLICE_DYMUX::BY, "0"), (enums::SLICE_DYMUX::Y, "1")] {
                bctx.mode(bk_l)
                    .attr("FFY", "#FF")
                    .attr("BYINV", "BY")
                    .pin("DY")
                    .pin("YQ")
                    .pin("BY")
                    .test_bel_attr_val(bcls::SLICE::DYMUX, val)
                    .attr("DYMUX", vname)
                    .commit();
            }
            bctx.mode(bk_l)
                .attr("SOPOUTUSED", "0")
                .pin("SOPIN")
                .pin("SOPOUT")
                .test_bel_attr(bcls::SLICE::SOPEXTSEL);
        } else {
            for (val, vname) in [
                (enums::SLICE_GYMUX::G, "G"),
                (enums::SLICE_GYMUX::FX, "FX"),
                (enums::SLICE_GYMUX::GXOR, "GXOR"),
            ] {
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("CYSELF", "1")
                    .attr("CYSELG", "1")
                    .attr("CYINIT", "BX")
                    .attr("BXINV", "BX")
                    .attr("YUSED", "0")
                    .pin("Y")
                    .pin("BX")
                    .test_bel_attr_val(bcls::SLICE::GYMUX, val)
                    .attr("GYMUX", vname)
                    .commit();
            }
            for (val, vname) in [(enums::SLICE_DXMUX::BX, "0"), (enums::SLICE_DXMUX::X, "1")] {
                bctx.mode(bk_l)
                    .attr("F", "#LUT:0")
                    .attr("XUSED", "0")
                    .attr("FXMUX", "F")
                    .attr("FFX", "#FF")
                    .attr("BXINV", "BX")
                    .pin("X")
                    .pin("XQ")
                    .pin("BX")
                    .test_bel_attr_val(bcls::SLICE::DXMUX, val)
                    .attr("DXMUX", vname)
                    .commit();
            }
            for (val, vname) in [(enums::SLICE_DYMUX::BY, "0"), (enums::SLICE_DYMUX::Y, "1")] {
                bctx.mode(bk_l)
                    .attr("G", "#LUT:0")
                    .attr("YUSED", "0")
                    .attr("GYMUX", "G")
                    .attr("FFY", "#FF")
                    .attr("BYINV", "BY")
                    .pin("Y")
                    .pin("YQ")
                    .pin("BY")
                    .test_bel_attr_val(bcls::SLICE::DYMUX, val)
                    .attr("DYMUX", vname)
                    .commit();
            }
        }

        // LUT: memory mode
        if is_m {
            for (val, vname) in [
                (enums::SLICE_DIF_MUX::BX, "BX"),
                (enums::SLICE_DIF_MUX::ALT, "ALTDIF"),
                (enums::SLICE_DIF_MUX::ALT, "SHIFTIN"),
            ] {
                bctx.mode(bk_m)
                    .attr("F", "#RAM:0")
                    .attr("FXMUX", "F")
                    .attr("XUSED", "0")
                    .attr("BXINV", "BX")
                    .pin("X")
                    .pin("BX")
                    .pin("SHIFTIN")
                    .test_bel_attr_val(bcls::SLICE::DIF_MUX, val)
                    .attr("DIF_MUX", vname)
                    .commit();
            }
            for (val, vname) in [
                (enums::SLICE_DIG_MUX::BY, "BY"),
                (enums::SLICE_DIG_MUX::ALT, "ALTDIG"),
                (enums::SLICE_DIG_MUX::ALT, "SHIFTIN"),
            ] {
                bctx.mode(bk_m)
                    .attr("G", "#RAM:0")
                    .attr("GYMUX", "G")
                    .attr("YUSED", "0")
                    .attr("BYINV", "BY")
                    .pin("Y")
                    .pin("BY")
                    .pin("SHIFTIN")
                    .test_bel_attr_val(bcls::SLICE::DIG_MUX, val)
                    .attr("DIG_MUX", vname)
                    .commit();
            }
            for (val, vname) in [
                (enums::SLICE_XBMUX::FMC15, "0"),
                (enums::SLICE_XBMUX::FCY, "1"),
            ] {
                bctx.mode(bk_m)
                    .attr("F", "#RAM:0")
                    .pin("XB")
                    .test_bel_attr_val(bcls::SLICE::XBMUX, val)
                    .attr("XBMUX", vname)
                    .commit();
            }
            for (val, vname) in [
                (enums::SLICE_YBMUX::GMC15, "0"),
                (enums::SLICE_YBMUX::GCY, "1"),
            ] {
                bctx.mode(bk_m)
                    .attr("G", "#RAM:0")
                    .attr("YBUSED", "0")
                    .pin("YB")
                    .test_bel_attr_val(bcls::SLICE::YBMUX, val)
                    .attr("YBMUX", vname)
                    .commit();
            }
            bctx.mode(bk_m)
                .attr("XUSED", "0")
                .attr("FXMUX", "F")
                .attr("G", "#LUT:0")
                .attr("F_ATTR", "DUAL_PORT")
                .pin("X")
                .test_bel_attr_bits(bcls::SLICE::F_RAM_ENABLE)
                .attr_diff("F", "#LUT:0", "#RAM:0")
                .commit();
            bctx.mode(bk_m)
                .attr("YUSED", "0")
                .attr("GYMUX", "G")
                .attr("F", "#LUT:0")
                .attr("G_ATTR", "DUAL_PORT")
                .pin("Y")
                .test_bel_attr_bits(bcls::SLICE::G_RAM_ENABLE)
                .attr_diff("G", "#LUT:0", "#RAM:0")
                .commit();
            bctx.mode(bk_m)
                .attr("F", "#RAM:0")
                .attr("XUSED", "0")
                .attr("FXMUX", "F")
                .pin("X")
                .test_bel_attr_bool_rename(
                    "F_ATTR",
                    bcls::SLICE::F_SHIFT_ENABLE,
                    "DUAL_PORT",
                    "SHIFT_REG",
                );
            bctx.mode(bk_m)
                .attr("G", "#RAM:0")
                .attr("YUSED", "0")
                .attr("GYMUX", "G")
                .pin("Y")
                .test_bel_attr_bool_rename(
                    "G_ATTR",
                    bcls::SLICE::G_SHIFT_ENABLE,
                    "DUAL_PORT",
                    "SHIFT_REG",
                );
            if edev.chip.kind.is_virtex2() {
                bctx.mode(bk_m)
                    .attr("F", "#RAM:0")
                    .attr("FXMUX", "F")
                    .attr("XUSED", "0")
                    .attr("BXINV", "BX")
                    .pin("X")
                    .pin("BX")
                    .pin("SLICEWE0")
                    .test_bel_attr_bits(bcls::SLICE::SLICEWE0USED)
                    .attr("SLICEWE0USED", "0")
                    .commit();
                for (spec, pin, pinused) in [
                    (specials::SLICE_SLICEWE1USED, "SLICEWE1", "SLICEWE1USED"),
                    (specials::SLICE_SLICEWE2USED, "SLICEWE2", "SLICEWE2USED"),
                ] {
                    bctx.mode(bk_m)
                        .null_bits()
                        .attr("F", "#RAM:0")
                        .attr("FXMUX", "F")
                        .attr("XUSED", "0")
                        .attr("BXINV", "BX")
                        .pin("X")
                        .pin("BX")
                        .pin(pin)
                        .test_bel_special(spec)
                        .attr(pinused, "0")
                        .commit();
                }
                bctx.mode(bk_m)
                    .null_bits()
                    .attr("BXINV", "BX")
                    .pin("BX")
                    .pin("BXOUT")
                    .test_bel_special(specials::SLICE_BXOUTUSED)
                    .attr("BXOUTUSED", "0")
                    .commit();
                bctx.mode(bk_m)
                    .attr("BYINV", "BY")
                    .attr("BYINVOUTUSED", "")
                    .pin("BY")
                    .pin("BYOUT")
                    .test_bel_attr_bits(bcls::SLICE::BYOUTUSED)
                    .attr("BYOUTUSED", "0")
                    .commit();
                bctx.mode(bk_m)
                    .attr("BYINV", "BY")
                    .attr("BYOUTUSED", "")
                    .pin("BY")
                    .pin("BYOUT")
                    .test_bel_attr_bits(bcls::SLICE::BYOUTUSED)
                    .attr("BYINVOUTUSED", "0")
                    .commit();
            } else {
                for (attr, aname) in [
                    (bcls::SLICE::SLICEWE0USED, "SLICEWE0USED"),
                    (bcls::SLICE::SLICEWE1USED, "SLICEWE1USED"),
                ] {
                    bctx.mode(bk_m)
                        .attr("F", "#RAM:0")
                        .attr("FXMUX", "F")
                        .attr("XUSED", "0")
                        .attr("BXINV", "BX")
                        .pin("X")
                        .pin("BX")
                        .pin("SLICEWE1")
                        .test_bel_attr_bits(attr)
                        .attr(aname, "0")
                        .commit();
                }
                bctx.mode(bk_m)
                    .null_bits()
                    .attr("BYINV", "BY")
                    .attr("BYINVOUTUSED", "")
                    .pin("BY")
                    .pin("BYOUT")
                    .test_bel_special(specials::SLICE_BYOUTUSED)
                    .attr("BYOUTUSED", "0")
                    .commit();
                bctx.mode(bk_m)
                    .null_bits()
                    .attr("BYINV", "BY")
                    .attr("BYOUTUSED", "")
                    .pin("BY")
                    .pin("BYOUT")
                    .test_bel_special(specials::SLICE_BYINVOUTUSED)
                    .attr("BYINVOUTUSED", "0")
                    .commit();
            }
        }

        // FF
        bctx.mode(bk_l)
            .pin("BX")
            .pin("XQ")
            .pin("CE")
            .attr("FFY", "")
            .attr("CEINV", "CE_B")
            .attr("FFX_INIT_ATTR", "INIT1")
            .test_bel_attr_bool_rename("FFX", bcls::SLICE::FF_LATCH, "#FF", "#LATCH");
        bctx.mode(bk_l)
            .pin("BY")
            .pin("YQ")
            .pin("CE")
            .attr("FFX", "")
            .attr("CEINV", "CE_B")
            .attr("FFY_INIT_ATTR", "INIT1")
            .test_bel_attr_bool_rename("FFY", bcls::SLICE::FF_LATCH, "#FF", "#LATCH");
        bctx.mode(bk_l)
            .pin("XQ")
            .attr("FFX", "#FF")
            .test_bel_attr_bool_rename("SYNC_ATTR", bcls::SLICE::FF_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode(bk_l)
            .pin("XQ")
            .attr("FFX_INIT_ATTR", "INIT1")
            .attr("FFX", "#FF")
            .test_bel_attr_bool_rename("FFX_SR_ATTR", bcls::SLICE::FFX_SRVAL, "SRLOW", "SRHIGH");
        bctx.mode(bk_l)
            .pin("YQ")
            .attr("FFY_INIT_ATTR", "INIT1")
            .attr("FFY", "#FF")
            .test_bel_attr_bool_rename("FFY_SR_ATTR", bcls::SLICE::FFY_SRVAL, "SRLOW", "SRHIGH");
        bctx.mode(bk_l)
            .pin("XQ")
            .attr("FFX", "#FF")
            .test_bel_attr_bool_rename("FFX_INIT_ATTR", bcls::SLICE::FFX_INIT, "INIT0", "INIT1");
        bctx.mode(bk_l)
            .pin("YQ")
            .attr("FFY", "#FF")
            .test_bel_attr_bool_rename("FFY_INIT_ATTR", bcls::SLICE::FFY_INIT, "INIT0", "INIT1");
        bctx.mode(bk_l)
            .attr("FFX", "#FF")
            .attr("BYINV", "BY")
            .pin("XQ")
            .pin("BY")
            .test_bel_attr_bits(bcls::SLICE::FF_REV_ENABLE)
            .attr("REVUSED", "0")
            .commit();
    }
    if !edev.chip.kind.is_virtex2() {
        let mut ctx = FuzzCtx::new_id(
            session,
            backend,
            if edev.chip.kind == ChipKind::FpgaCore {
                tcls_s3::RANDOR_FC
            } else {
                tcls_s3::RANDOR
            },
        );
        let mut bctx = ctx.bel(bslots::RANDOR);
        for (val, vname) in [
            (enums::RANDOR_MODE::AND, "1"),
            (enums::RANDOR_MODE::OR, "0"),
        ] {
            bctx.mode("RESERVED_ANDOR")
                .pin("O")
                .prop(RandorInit)
                .test_bel_attr_val(bcls::RANDOR::MODE, val)
                .attr("ANDORMUX", vname)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = if edev.chip.kind.is_virtex2() {
        tcls_v2::CLB
    } else {
        tcls_s3::CLB
    };
    for (idx, bslot) in bslots::SLICE.into_iter().enumerate() {
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::F);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::G);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::CYINIT);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::CYSELF);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::CYSELG);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::CY0F);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::CY0G);

        // LUT RAM
        let is_m = edev.chip.kind.is_virtex2() || matches!(idx, 0 | 2);
        if is_m {
            ctx.get_diff_attr_bool(tcid, bslot, bcls::SLICE::F_SHIFT_ENABLE, false)
                .assert_empty();
            ctx.get_diff_attr_bool(tcid, bslot, bcls::SLICE::G_SHIFT_ENABLE, false)
                .assert_empty();
            let f_ram = ctx.get_diff_attr_bit(tcid, bslot, bcls::SLICE::F_RAM_ENABLE, 0);
            let g_ram = ctx.get_diff_attr_bit(tcid, bslot, bcls::SLICE::G_RAM_ENABLE, 0);
            let (f_ram, g_ram, ram) = Diff::split(f_ram, g_ram);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::SLICE::FF_SR_ENABLE, xlat_bit(!ram));
            let f_shift_d = ctx.get_diff_attr_bool(tcid, bslot, bcls::SLICE::F_SHIFT_ENABLE, true);
            let g_shift_d = ctx.get_diff_attr_bool(tcid, bslot, bcls::SLICE::G_SHIFT_ENABLE, true);
            let f_shift = f_ram.combine(&f_shift_d);
            let g_shift = g_ram.combine(&g_shift_d);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::SLICE::F_RAM_ENABLE, xlat_bit(f_ram));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::SLICE::G_RAM_ENABLE, xlat_bit(g_ram));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::SLICE::F_SHIFT_ENABLE, xlat_bit(f_shift));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::SLICE::G_SHIFT_ENABLE, xlat_bit(g_shift));

            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::DIF_MUX);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::DIG_MUX);

            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::SLICEWE0USED);
            if edev.chip.kind.is_virtex2() {
                ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::BYOUTUSED);
            } else {
                if idx == 0 {
                    ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::SLICEWE1USED);
                } else {
                    let slicewe1used =
                        ctx.get_diff_attr_bit(tcid, bslot, bcls::SLICE::SLICEWE1USED, 0);
                    slicewe1used.assert_empty();
                }
            }
        }

        // muxes
        if edev.chip.kind.is_virtex2() {
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::FXMUX);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::GYMUX);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::SOPEXTSEL);
        } else {
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::FXMUX);
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                bcls::SLICE::GYMUX,
                &[
                    enums::SLICE_GYMUX::G,
                    enums::SLICE_GYMUX::FX,
                    enums::SLICE_GYMUX::GXOR,
                ],
            );
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::DXMUX);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::DYMUX);
        if is_m {
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::XBMUX);
            ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::YBMUX);
        }

        // FFs
        ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::SLICE::FF_SR_SYNC);
        ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::SLICE::FF_LATCH);
        ctx.collect_bel_attr(tcid, bslot, bcls::SLICE::FF_REV_ENABLE);
        ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::SLICE::FFX_SRVAL);
        ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::SLICE::FFY_SRVAL);
        ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::SLICE::FFX_INIT);
        ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::SLICE::FFY_INIT);

        // inverts
        let int = if edev.chip.kind.is_virtex2() {
            tcls_v2::INT_CLB
        } else if edev.chip.kind == ChipKind::FpgaCore {
            tcls_s3::INT_CLB_FC
        } else {
            tcls_s3::INT_CLB
        };
        ctx.collect_bel_input_inv_int_bi(&[int], tcid, bslot, bcls::SLICE::CLK);
        ctx.collect_bel_input_inv_int_bi(&[int], tcid, bslot, bcls::SLICE::SR);
        ctx.collect_bel_input_inv_int_bi(&[int], tcid, bslot, bcls::SLICE::CE);
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SLICE::BX);
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SLICE::BY);
    }
    if !edev.chip.kind.is_virtex2() {
        let tcid = if edev.chip.kind == ChipKind::FpgaCore {
            tcls_s3::RANDOR_FC
        } else {
            tcls_s3::RANDOR
        };
        let bslot = bslots::RANDOR;
        if edev.chip.kind.is_spartan3a() || edev.chip.kind == ChipKind::FpgaCore {
            ctx.get_diff_attr_val(tcid, bslot, bcls::RANDOR::MODE, enums::RANDOR_MODE::AND)
                .assert_empty();
            ctx.get_diff_attr_val(tcid, bslot, bcls::RANDOR::MODE, enums::RANDOR_MODE::OR)
                .assert_empty();
        } else {
            ctx.collect_bel_attr(tcid, bslot, bcls::RANDOR::MODE);
        }
        let tcid = if edev.chip.kind == ChipKind::FpgaCore {
            tcls_s3::RANDOR_INIT_FC
        } else {
            tcls_s3::RANDOR_INIT
        };
        let bslot = bslots::RANDOR_INIT;
        ctx.collect_bel_attr(tcid, bslot, bcls::RANDOR_INIT::MODE);
    }
    let int_clb = if edev.chip.kind.is_virtex2() {
        tcls_v2::INT_CLB
    } else if edev.chip.kind == ChipKind::FpgaCore {
        tcls_s3::INT_CLB_FC
    } else {
        tcls_s3::INT_CLB
    };
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if tcid == int_clb {
            continue;
        }
        if !ctx.has_tile_id(tcid) {
            continue;
        }
        if !tcls.bels.contains_id(bslots::INT) {
            continue;
        }
        let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::INT] else {
            continue;
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgInv(inv) = item else {
                continue;
            };
            let Some(&bit) = ctx.data.sb_inv.get(&(int_clb, inv.dst)) else {
                continue;
            };
            ctx.insert_inv(tcid, inv.dst, bit);
        }
    }
}
