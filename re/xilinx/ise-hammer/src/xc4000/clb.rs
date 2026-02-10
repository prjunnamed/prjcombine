use prjcombine_re_collector::diff::{Diff, xlat_bit, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::{
    chip::ChipKind,
    xc4000::{bslots, enums, xc4000::bcls, xc4000::tcls},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            bel::BelUnused,
            relation::{Delta, Related},
        },
    },
    xc4000::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    let kind = edev.chip.kind;
    let ff_maybe = if kind.is_clb_xl() { "#FF" } else { "" };
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        if !tcls.bels.contains_id(bslots::CLB) {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::CLB);
        let mode = "CLB";
        bctx.mode(mode)
            .attr("XMUX", "F")
            .pin("X")
            .test_bel_attr_multi(bcls::CLB::F, MultiValue::OldLut('F'));
        bctx.mode(mode)
            .attr("YMUX", "G")
            .pin("Y")
            .test_bel_attr_multi(bcls::CLB::G, MultiValue::OldLut('G'));
        bctx.mode(mode)
            .attr("YMUX", "H")
            .pin("Y")
            .test_bel_attr_multi(bcls::CLB::H, MultiValue::OldLut('H'));
        bctx.mode(mode)
            .attr("XMUX", "F")
            .pin("X")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_bel_attr_bits(bcls::CLB::F_RAM_ENABLE)
            .attr_diff("F", "#LUT:F=0x0", "#RAM:F=0x0")
            .commit();
        bctx.mode(mode)
            .attr("YMUX", "G")
            .pin("Y")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_bel_attr_bits(bcls::CLB::G_RAM_ENABLE)
            .attr_diff("G", "#LUT:G=0x0", "#RAM:G=0x0")
            .commit();
        for (spec, val) in [
            (specials::CLB_RAMCLK_CLK, "CLK"),
            (specials::CLB_RAMCLK_CLKNOT, "CLKNOT"),
        ] {
            bctx.mode(mode)
                .attr("YMUX", "G")
                .pin("Y")
                .attr("G", "#RAM:G=0x0")
                .pin("C1")
                .attr("DIN", "C1")
                .attr("H1", "C1")
                .attr("SR", "C1")
                .test_bel_special(spec)
                .attr("RAMCLK", val)
                .commit();
        }
        bctx.mode(mode)
            .attr("F", "#RAM:F=0x0")
            .attr("G", "#RAM:G=0x0")
            .pin("X")
            .pin("Y")
            .attr("XMUX", "F")
            .attr("YMUX", "G")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_bel_attr_val(bcls::CLB::RAM_DIMS, enums::CLB_RAM_DIMS::_32X1)
            .attr("RAM", "32X1")
            .commit();
        bctx.mode(mode)
            .attr("F", "#RAM:F=0x0")
            .attr("G", "#RAM:G=0x0")
            .pin("X")
            .pin("Y")
            .attr("XMUX", "F")
            .attr("YMUX", "G")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_bel_attr_bits(bcls::CLB::RAM_DP_ENABLE)
            .attr("RAM", "DP")
            .commit();
        bctx.mode(mode)
            .attr("XMUX", "H")
            .attr("YMUX", "G")
            .pin("X")
            .pin("Y")
            .pin("C1")
            .attr("SR", "C1")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .test_bel_attr_rename("H0", bcls::CLB::MUX_H0);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .pin("X")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .attr("H", "#LUT:H=0x0")
            .test_bel_attr_rename("H1", bcls::CLB::MUX_H1);
        bctx.mode(mode)
            .attr("XMUX", "F")
            .attr("YMUX", "H")
            .pin("X")
            .pin("Y")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("F", "#LUT:F=0x0")
            .attr("H", "#LUT:H=0x0")
            .test_bel_attr_rename("H2", bcls::CLB::MUX_H2);
        bctx.mode(mode)
            .attr("XQMUX", "DIN")
            .pin("XQ")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .test_bel_attr_rename("DIN", bcls::CLB::MUX_DIN);
        bctx.mode(mode)
            .attr("YQMUX", "EC")
            .pin("YQ")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .test_bel_attr_rename("EC", bcls::CLB::MUX_EC);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .attr("YMUX", "G")
            .pin("X")
            .pin("Y")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "SR")
            .attr("YQMUX", "EC")
            .pin("YQ")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .test_bel_attr_rename("SR", bcls::CLB::MUX_SR);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .pin("X")
            .attr("F", "#LUT:F=0x0")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "G")
            .attr("H2", "F")
            .attr("DIN", "C1")
            .pin("C1")
            .attr("XQMUX", "DIN")
            .test_bel_attr_rename("DX", bcls::CLB::MUX_DX);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .pin("X")
            .attr("F", "#LUT:F=0x0")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "G")
            .attr("H2", "F")
            .attr("DIN", "C1")
            .pin("C1")
            .attr("XQMUX", "DIN")
            .test_bel_attr_rename("DY", bcls::CLB::MUX_DY);
        bctx.mode(mode)
            .test_bel_attr_bool_rename("SRX", bcls::CLB::FFX_SRVAL, "RESET", "SET");
        bctx.mode(mode)
            .test_bel_attr_bool_rename("SRY", bcls::CLB::FFY_SRVAL, "RESET", "SET");
        bctx.mode(mode)
            .attr("EC", "C1")
            .attr("DIN", "C1")
            .attr("DX", "DIN")
            .attr("XQMUX", "QX")
            .attr("CLKX", "CLK")
            .pin("C1")
            .pin("XQ")
            .pin("K")
            .test_bel_attr_bits(bcls::CLB::FFX_EC_ENABLE)
            .attr("ECX", "EC")
            .commit();
        bctx.mode(mode)
            .attr("EC", "C1")
            .attr("DIN", "C1")
            .attr("DY", "DIN")
            .attr("YQMUX", "QY")
            .attr("CLKY", "CLK")
            .pin("C1")
            .pin("YQ")
            .pin("K")
            .test_bel_attr_bits(bcls::CLB::FFY_EC_ENABLE)
            .attr("ECY", "EC")
            .commit();
        bctx.mode(mode)
            .attr("SR", "C1")
            .attr("DIN", "C1")
            .attr("DX", "DIN")
            .attr("XQMUX", "QX")
            .attr("CLKX", "CLK")
            .pin("C1")
            .pin("XQ")
            .pin("K")
            .test_bel_attr_bits(bcls::CLB::FFX_SR_ENABLE)
            .attr("SETX", "SR")
            .commit();
        bctx.mode(mode)
            .attr("SR", "C1")
            .attr("DIN", "C1")
            .attr("DY", "DIN")
            .attr("YQMUX", "QY")
            .attr("CLKY", "CLK")
            .pin("C1")
            .pin("YQ")
            .pin("K")
            .test_bel_attr_bits(bcls::CLB::FFY_SR_ENABLE)
            .attr("SETY", "SR")
            .commit();
        bctx.mode(mode)
            .attr("F", "#LUT:F=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H2", "F")
            .attr("YMUX", "H")
            .pin("X")
            .pin("Y")
            .test_bel_attr_rename("XMUX", bcls::CLB::MUX_X);
        bctx.mode(mode)
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "G")
            .attr("XMUX", "H")
            .pin("X")
            .pin("Y")
            .test_bel_attr_rename("YMUX", bcls::CLB::MUX_Y);
        bctx.mode(mode)
            .attr("DIN", "C1")
            .attr("DX", "DIN")
            .attr("XQMUX", "QX")
            .attr("FFX", ff_maybe)
            .pin("C1")
            .pin("XQ")
            .pin("K")
            .test_bel_attr_bits(bcls::CLB::FFX_CLK_INV)
            .attr_diff("CLKX", "CLK", "CLKNOT")
            .commit();
        bctx.mode(mode)
            .attr("DIN", "C1")
            .attr("DY", "DIN")
            .attr("YQMUX", "QY")
            .attr("FFY", ff_maybe)
            .pin("C1")
            .pin("YQ")
            .pin("K")
            .test_bel_attr_bits(bcls::CLB::FFY_CLK_INV)
            .attr_diff("CLKY", "CLK", "CLKNOT")
            .commit();

        if kind.is_clb_xl() {
            bctx.mode(mode)
                .attr("DIN", "C1")
                .attr("DX", "DIN")
                .attr("XQMUX", "QX")
                .pin("C1")
                .pin("XQ")
                .pin("K")
                .test_bel_attr_val(bcls::CLB::FFX_MODE, enums::CLB_FF_MODE::LATCH)
                .attr_diff("CLKX", "CLK", "CLKNOT")
                .attr_diff("FFX", "#FF", "#LATCH")
                .commit();
            bctx.mode(mode)
                .attr("DIN", "C1")
                .attr("DY", "DIN")
                .attr("YQMUX", "QY")
                .pin("C1")
                .pin("YQ")
                .pin("K")
                .test_bel_attr_val(bcls::CLB::FFY_MODE, enums::CLB_FF_MODE::LATCH)
                .attr_diff("CLKY", "CLK", "CLKNOT")
                .attr_diff("FFY", "#FF", "#LATCH")
                .commit();
        }

        bctx.mode(mode)
            .attr("DIN", "C1")
            .pin("C1")
            .pin("XQ")
            .test_bel_attr_val(bcls::CLB::MUX_XQ, enums::CLB_MUX_XQ::DIN)
            .attr("XQMUX", "DIN")
            .commit();
        bctx.mode(mode)
            .attr("EC", "C1")
            .pin("C1")
            .pin("XQ")
            .test_bel_attr_val(bcls::CLB::MUX_YQ, enums::CLB_MUX_YQ::EC)
            .attr("YQMUX", "EC")
            .commit();

        for (val, vname) in [
            (enums::CLB_CARRY_ADDSUB::ADDSUB, "ADDSUB"),
            (enums::CLB_CARRY_ADDSUB::SUB, "SUB"),
        ] {
            bctx.mode(mode)
                .attr("FCARRY", "CARRY")
                .attr("GCARRY", "CARRY")
                .attr("CINMUX", "CIN")
                .test_bel_attr_val(bcls::CLB::CARRY_ADDSUB, val)
                .attr_diff("CARRY", "ADD", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::CLB_CARRY_FGEN::F1, "F1"),
            (enums::CLB_CARRY_FGEN::F3_INV, "F3"),
        ] {
            bctx.mode(mode)
                .attr("FCARRY", "")
                .attr("GCARRY", "CARRY")
                .attr("CARRY", "ADD")
                .pin("CIN")
                .test_bel_attr_val(bcls::CLB::CARRY_FGEN, val)
                .attr_diff("CINMUX", "0", vname)
                .commit();
        }
        bctx.mode(mode)
            .attr("FCARRY", "CARRY")
            .attr("GCARRY", "CARRY")
            .attr("CINMUX", "CIN")
            .test_bel_attr_bits(bcls::CLB::CARRY_OP2_ENABLE)
            .attr_diff("CARRY", "INCDEC", "ADDSUB")
            .commit();
        bctx.mode(mode)
            .attr("CARRY", "ADD")
            .attr("GCARRY", "CARRY")
            .test_bel_attr_val(bcls::CLB::CARRY_FPROP, enums::CLB_CARRY_PROP::CONST_0)
            .attr_diff("FCARRY", "CARRY", "")
            .attr_diff("CINMUX", "CIN", "F1")
            .commit();
        bctx.mode(mode)
            .attr("CARRY", "ADD")
            .attr("GCARRY", "CARRY")
            .attr("CINMUX", "CIN")
            .test_bel_attr_val(bcls::CLB::CARRY_FPROP, enums::CLB_CARRY_PROP::CONST_1)
            .attr_diff("FCARRY", "CARRY", "")
            .commit();

        bctx.mode(mode)
            .attr("CARRY", "ADD")
            .attr("FCARRY", "CARRY")
            .attr("CINMUX", "CIN")
            .test_bel_attr_val(bcls::CLB::CARRY_GPROP, enums::CLB_CARRY_PROP::CONST_1)
            .attr_diff("GCARRY", "CARRY", "")
            .commit();
        if kind.is_clb_xl() {
            bctx.mode(mode)
                .attr("CARRY", "ADD")
                .attr("FCARRY", "CARRY")
                .test_bel_attr_val(bcls::CLB::CARRY_GPROP, enums::CLB_CARRY_PROP::CONST_0)
                .attr_diff("GCARRY", "CARRY", "")
                .attr_diff("CINMUX", "CIN", "G4")
                .commit();
        } else if tcid == tcls::CLB {
            bctx.mode(mode)
                .pin("CIN")
                .prop(Related::new(
                    Delta::new(0, -1, tcls::CLB),
                    BelUnused::new(bslots::CLB, 0),
                ))
                .prop(Related::new(
                    Delta::new(0, 1, tcls::CLB),
                    BelUnused::new(bslots::CLB, 0),
                ))
                .test_bel_attr_val(bcls::CLB::MUX_CIN, enums::CLB_MUX_CIN::COUT_N)
                .related_pip(Delta::new(0, -1, tcls::CLB), "CIN_N", "COUT")
                .commit();
            bctx.mode(mode)
                .pin("CIN")
                .prop(Related::new(
                    Delta::new(0, -1, tcls::CLB),
                    BelUnused::new(bslots::CLB, 0),
                ))
                .prop(Related::new(
                    Delta::new(0, 1, tcls::CLB),
                    BelUnused::new(bslots::CLB, 0),
                ))
                .test_bel_attr_val(bcls::CLB::MUX_CIN, enums::CLB_MUX_CIN::COUT_S)
                .related_pip(Delta::new(0, 1, tcls::CLB), "CIN_S", "COUT")
                .commit();
        }
        // F4MUX, G2MUX, G3MUX handled as part of interconnect
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = edev.chip.kind;
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        let bslot = bslots::CLB;
        if !tcls.bels.contains_id(bslot) {
            continue;
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::F);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::G);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::H);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::F_RAM_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::G_RAM_ENABLE);
        ctx.collect_bel_attr_default(tcid, bslot, bcls::CLB::RAM_DIMS, enums::CLB_RAM_DIMS::_16X2);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::RAM_DP_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_H0);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_H1);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_H2);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_EC);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_DIN);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_SR);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_DX);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_DY);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFX_EC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFY_EC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFX_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFY_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_X);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_Y);
        ctx.collect_bel_attr_default(tcid, bslot, bcls::CLB::MUX_XQ, enums::CLB_MUX_XQ::FFX);
        ctx.collect_bel_attr_default(tcid, bslot, bcls::CLB::MUX_YQ, enums::CLB_MUX_YQ::FFY);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::CLB::FFX_SRVAL);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::CLB::FFY_SRVAL);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFX_CLK_INV);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFY_CLK_INV);
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            bcls::CLB::CARRY_ADDSUB,
            enums::CLB_CARRY_ADDSUB::ADD,
        );
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            bcls::CLB::CARRY_FGEN,
            enums::CLB_CARRY_FGEN::CONST_OP2_ENABLE,
        );
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::CARRY_OP2_ENABLE);
        if kind.is_clb_xl() {
            ctx.collect_bel_attr_default(tcid, bslot, bcls::CLB::FFX_MODE, enums::CLB_FF_MODE::FF);
            ctx.collect_bel_attr_default(tcid, bslot, bcls::CLB::FFY_MODE, enums::CLB_FF_MODE::FF);
        }

        if !kind.is_clb_xl() {
            if tcid == tcls::CLB {
                ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_CIN);
            } else {
                let item = ctx
                    .bel_attr_enum(tcls::CLB, bslot, bcls::CLB::MUX_CIN)
                    .clone();
                ctx.insert_bel_attr_enum(tcid, bslot, bcls::CLB::MUX_CIN, item);
            }
        }

        let diff_s = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_RAMCLK_CLK);
        let diff_inv = ctx
            .get_diff_bel_special(tcid, bslot, specials::CLB_RAMCLK_CLKNOT)
            .combine(&!&diff_s);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::CLB::RAM_SYNC_ENABLE, xlat_bit(diff_s));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::CLB::RAM_CLK_INV, xlat_bit(diff_inv));

        let diff0 = ctx.get_diff_attr_val(
            tcid,
            bslot,
            bcls::CLB::CARRY_FPROP,
            enums::CLB_CARRY_PROP::CONST_0,
        );
        let mut diff1 = ctx.get_diff_attr_val(
            tcid,
            bslot,
            bcls::CLB::CARRY_FPROP,
            enums::CLB_CARRY_PROP::CONST_1,
        );
        diff1.discard_bits_enum(ctx.bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_FGEN));
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::CLB::CARRY_FPROP,
            xlat_enum_attr(vec![
                (enums::CLB_CARRY_PROP::XOR, Diff::default()),
                (enums::CLB_CARRY_PROP::CONST_0, diff0),
                (enums::CLB_CARRY_PROP::CONST_1, diff1),
            ]),
        );

        let diff1 = ctx.get_diff_attr_val(
            tcid,
            bslot,
            bcls::CLB::CARRY_GPROP,
            enums::CLB_CARRY_PROP::CONST_1,
        );
        if !kind.is_clb_xl() {
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::CLB::CARRY_GPROP,
                xlat_enum_attr(vec![
                    (enums::CLB_CARRY_PROP::XOR, Diff::default()),
                    (enums::CLB_CARRY_PROP::CONST_1, diff1),
                ]),
            );
        } else {
            let mut diff0 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::CLB::CARRY_GPROP,
                enums::CLB_CARRY_PROP::CONST_0,
            );
            diff0.discard_bits_enum(ctx.bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_FGEN));
            diff0.discard_bits_enum(ctx.bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_FPROP));
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::CLB::CARRY_GPROP,
                xlat_enum_attr(vec![
                    (enums::CLB_CARRY_PROP::XOR, Diff::default()),
                    (enums::CLB_CARRY_PROP::CONST_0, diff0),
                    (enums::CLB_CARRY_PROP::CONST_1, diff1),
                ]),
            );
        }

        let rb = if kind.is_xl() {
            [
                (bcls::CLB::READBACK_X, 0, 3),
                (bcls::CLB::READBACK_Y, 0, 5),
                (bcls::CLB::READBACK_XQ, 0, 7),
                (bcls::CLB::READBACK_YQ, 0, 4),
            ]
        } else if kind == ChipKind::SpartanXl {
            // ?!?! X/XQ swapped from XC4000?
            [
                (bcls::CLB::READBACK_X, 12, 5),
                (bcls::CLB::READBACK_Y, 3, 5),
                (bcls::CLB::READBACK_XQ, 16, 4),
                (bcls::CLB::READBACK_YQ, 8, 4),
            ]
        } else {
            [
                (bcls::CLB::READBACK_X, 16, 4),
                (bcls::CLB::READBACK_Y, 3, 5),
                (bcls::CLB::READBACK_XQ, 12, 5),
                (bcls::CLB::READBACK_YQ, 8, 4),
            ]
        };
        for (attr, frame, bit) in rb {
            ctx.insert_bel_attr_bool(tcid, bslot, attr, TileBit::new(0, frame, bit).neg());
        }
    }
}
