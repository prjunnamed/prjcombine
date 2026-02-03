use prjcombine_re_collector::diff::{Diff, xlat_bit, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc4000::{bslots, enums, xc4000::bcls};

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx, specials};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        if !tcls.bels.contains_id(bslots::CLB) {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::CLB);
        bctx.mode("FG").mutex("RAM", "").test_bel_attr_equate(
            bcls::CLB::F,
            "F",
            &["F1", "F2", "F3", "F4"],
        );
        bctx.mode("FG").mutex("RAM", "").test_bel_attr_equate(
            bcls::CLB::G,
            "G",
            &["G1", "G2", "G3", "G4"],
        );
        bctx.mode("FG")
            .mutex("RAM", "")
            .test_bel_attr_equate(bcls::CLB::H, "H", &["F", "G", "H1"]);
        if !backend.device.name.ends_with('d') {
            bctx.mode("FG")
                .test_bel_attr_bits(bcls::CLB::F_RAM_ENABLE)
                .cfg_excl("RAM", "F")
                .commit();
            bctx.mode("FG")
                .test_bel_attr_bits(bcls::CLB::G_RAM_ENABLE)
                .cfg_excl("RAM", "G")
                .commit();
            bctx.mode("FG")
                .test_bel_special(specials::CLB_RAM_FG)
                .cfg_excl("RAM", "FG")
                .commit();
        }
        bctx.mode("FG").test_bel_attr_as("H1", bcls::CLB::MUX_H1);
        bctx.mode("FG").test_bel_attr_as("DIN", bcls::CLB::MUX_DIN);
        bctx.mode("FG").test_bel_attr_as("SR", bcls::CLB::MUX_SR);
        bctx.mode("FG").test_bel_attr_as("EC", bcls::CLB::MUX_EC);
        bctx.mode("FG").test_bel_attr_as("X", bcls::CLB::MUX_X);
        bctx.mode("FG").test_bel_attr_as("Y", bcls::CLB::MUX_Y);
        bctx.mode("FG")
            .test_bel_attr_val(bcls::CLB::MUX_XQ, enums::CLB_MUX_XQ::DIN)
            .cfg_excl("XQ", "DIN")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_val(bcls::CLB::MUX_XQ, enums::CLB_MUX_XQ::FFX)
            .cfg_excl("XQ", "QX")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_val(bcls::CLB::MUX_YQ, enums::CLB_MUX_YQ::EC)
            .cfg_excl("YQ", "EC")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_val(bcls::CLB::MUX_YQ, enums::CLB_MUX_YQ::FFY)
            .cfg_excl("YQ", "QY")
            .commit();
        bctx.mode("FG").test_bel_attr_as("DX", bcls::CLB::MUX_DX);
        bctx.mode("FG").test_bel_attr_as("DY", bcls::CLB::MUX_DY);
        bctx.mode("FG")
            .test_bel_attr_enum_bool_as("FFX", bcls::CLB::FFX_SRVAL, "RESET", "SET");
        bctx.mode("FG")
            .test_bel_attr_enum_bool_as("FFY", bcls::CLB::FFY_SRVAL, "RESET", "SET");
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::FFX_EC_ENABLE)
            .cfg("FFX", "EC")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::FFY_EC_ENABLE)
            .cfg("FFY", "EC")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::FFX_SR_ENABLE)
            .cfg("FFX", "SR")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::FFY_SR_ENABLE)
            .cfg("FFY", "SR")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::FFX_CLK_INV)
            .cfg("FFX", "NOT")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_bits(bcls::CLB::FFY_CLK_INV)
            .cfg("FFY", "NOT")
            .commit();
        bctx.mode("FG")
            .null_bits()
            .test_bel_special(specials::CLB_FFX_K)
            .cfg("FFX", "K")
            .commit();
        bctx.mode("FG")
            .null_bits()
            .test_bel_special(specials::CLB_FFY_K)
            .cfg("FFY", "K")
            .commit();
        for (attr, vname) in [
            (bcls::CLB::READBACK_X, "X"),
            (bcls::CLB::READBACK_XQ, "XQ"),
            (bcls::CLB::READBACK_Y, "Y"),
            (bcls::CLB::READBACK_YQ, "YQ"),
        ] {
            bctx.mode("FG")
                .test_bel_attr_bits(attr)
                .cfg_excl("RDBK", vname)
                .commit();
        }
        for (spec, val) in [
            (specials::CLB_CARRY_CB0, "CB0"),
            (specials::CLB_CARRY_CB1, "CB1"),
            (specials::CLB_CARRY_CB2, "CB2"),
            (specials::CLB_CARRY_CB3, "CB3"),
            (specials::CLB_CARRY_CB4, "CB4"),
            (specials::CLB_CARRY_CB5, "CB5"),
            (specials::CLB_CARRY_CB6, "CB6"),
            (specials::CLB_CARRY_CB7, "CB7"),
        ] {
            bctx.mode("FG")
                .test_bel_special(spec)
                .cfg("CARRY", val)
                .commit();
        }
        bctx.mode("FG")
            .test_bel_attr_val(bcls::CLB::MUX_CIN, enums::CLB_MUX_CIN::COUT_N)
            .cfg_excl("CDIR", "UP")
            .commit();
        bctx.mode("FG")
            .test_bel_attr_val(bcls::CLB::MUX_CIN, enums::CLB_MUX_CIN::COUT_S)
            .cfg_excl("CDIR", "DOWN")
            .commit();
        bctx.mode("FG")
            .null_bits()
            .test_bel_special(specials::CLB_CIN_CINI)
            .cfg("CIN", "CINI")
            .commit();
        bctx.mode("FG")
            .null_bits()
            .test_bel_special(specials::CLB_COUT_COUTI)
            .cfg("COUT", "COUTI")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        let bslot = bslots::CLB;
        if !tcls.bels.contains_id(bslot) {
            continue;
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::F);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::G);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::H);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_H1);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_DIN);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_SR);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_EC);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_X);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_Y);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_XQ);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_YQ);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_DX);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_DY);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::CLB::FFX_SRVAL);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::CLB::FFY_SRVAL);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFX_EC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFY_EC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFX_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFY_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFX_CLK_INV);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::FFY_CLK_INV);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::MUX_CIN);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::READBACK_X);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::READBACK_XQ);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::READBACK_Y);
        ctx.collect_bel_attr(tcid, bslot, bcls::CLB::READBACK_YQ);

        let bit0 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB0);
        let bit1 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB1);
        let bit2 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB2);
        let bit3 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB3);
        let bit4 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB4);
        let bit5 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB5);
        let bit6 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB6);
        let bit7 = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_CARRY_CB7);
        let item = xlat_enum_attr(vec![
            (enums::CLB_CARRY_ADDSUB::ADD, bit1),
            (enums::CLB_CARRY_ADDSUB::ADDSUB, bit0),
            (enums::CLB_CARRY_ADDSUB::SUB, Diff::default()),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_ADDSUB, item);
        let item = xlat_enum_attr(vec![
            (enums::CLB_CARRY_PROP::XOR, bit3),
            (enums::CLB_CARRY_PROP::CONST_1, bit2),
            (enums::CLB_CARRY_PROP::CONST_0, Diff::default()),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_FPROP, item);
        let item = xlat_enum_attr(vec![
            (enums::CLB_CARRY_FGEN::F1, bit4.combine(&bit5)),
            (enums::CLB_CARRY_FGEN::F3_INV, bit5),
            (enums::CLB_CARRY_FGEN::CONST_OP2_ENABLE, Diff::default()),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_FGEN, item);
        let item = xlat_enum_attr(vec![
            (enums::CLB_CARRY_PROP::XOR, bit6),
            (enums::CLB_CARRY_PROP::CONST_1, Diff::default()),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::CLB::CARRY_GPROP, item);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::CLB::CARRY_OP2_ENABLE, xlat_bit(bit7));

        if !ctx.device.name.ends_with('d') {
            ctx.collect_bel_attr(tcid, bslot, bcls::CLB::F_RAM_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::CLB::G_RAM_ENABLE);
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::CLB_RAM_FG);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::CLB::F_RAM_ENABLE),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::CLB::G_RAM_ENABLE),
                true,
                false,
            );
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, bcls::CLB::H), 0xca, 0x00);
            let item = xlat_enum_attr(vec![
                (enums::CLB_RAM_DIMS::_16X2, Diff::default()),
                (enums::CLB_RAM_DIMS::_32X1, diff),
            ]);
            ctx.insert_bel_attr_enum(tcid, bslot, bcls::CLB::RAM_DIMS, item);
        }
    }
}
