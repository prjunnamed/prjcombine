use prjcombine_interconnect::db::BelInputId;
use prjcombine_re_collector::diff::{Diff, xlat_bit, xlat_bit_bi};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{self, bcls, bslots, virtex4::tcls};

use crate::{
    backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx, virtex4::specials,
};

fn dsp48_invpins_bel() -> Vec<BelInputId> {
    vec![
        bcls::DSP_V4::CARRYINSEL[0],
        bcls::DSP_V4::CARRYINSEL[1],
        bcls::DSP_V4::CARRYIN,
        bcls::DSP_V4::SUBTRACT,
        bcls::DSP_V4::OPMODE[0],
        bcls::DSP_V4::OPMODE[1],
        bcls::DSP_V4::OPMODE[2],
        bcls::DSP_V4::OPMODE[3],
        bcls::DSP_V4::OPMODE[4],
        bcls::DSP_V4::OPMODE[5],
        bcls::DSP_V4::OPMODE[6],
    ]
}

fn dsp48_invpins_int() -> Vec<BelInputId> {
    vec![
        bcls::DSP_V4::CLK,
        bcls::DSP_V4::CEA,
        bcls::DSP_V4::CEB,
        bcls::DSP_V4::CEM,
        bcls::DSP_V4::CEP,
        bcls::DSP_V4::CECTRL,
        bcls::DSP_V4::CECARRYIN,
        bcls::DSP_V4::CECINSUB,
        bcls::DSP_V4::RSTA,
        bcls::DSP_V4::RSTB,
        bcls::DSP_V4::RSTM,
        bcls::DSP_V4::RSTP,
        bcls::DSP_V4::RSTCTRL,
        bcls::DSP_V4::RSTCARRYIN,
    ]
}

fn dsp48_invpins() -> Vec<BelInputId> {
    Vec::from_iter(dsp48_invpins_bel().into_iter().chain(dsp48_invpins_int()))
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::DSP);
    let mode = "DSP48";
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::DSP[i]);
        let obslot = defs::bslots::DSP[i ^ 1];
        bctx.build()
            .bel_unused(obslot)
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for pin in dsp48_invpins() {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }
        for attr in [bcls::DSP_V4::AREG, bcls::DSP_V4::BREG] {
            bctx.mode(mode).test_bel_attr(attr);
        }
        for attr in [
            bcls::DSP_V4::MREG,
            bcls::DSP_V4::PREG,
            bcls::DSP_V4::OPMODEREG,
            bcls::DSP_V4::CARRYINREG,
            bcls::DSP_V4::CARRYINSELREG,
            bcls::DSP_V4::SUBTRACTREG,
        ] {
            bctx.mode(mode).test_bel_attr_bool_auto(attr, "0", "1");
        }
        bctx.mode(mode).test_bel_attr(bcls::DSP_V4::B_INPUT);
        for (spec, vname) in [(specials::DSP_CREG_0, "0"), (specials::DSP_CREG_1, "1")] {
            bctx.build()
                .mode(mode)
                .bel_mode(obslot, mode)
                .bel_attr(obslot, "CREG", "")
                .test_bel_special(spec)
                .attr("CREG", vname)
                .commit();
        }
    }
    let mut bctx = ctx.bel(defs::bslots::DSP_C);
    for i in 0..2 {
        let bslot = bslots::DSP[i];
        let obslot = bslots::DSP[i ^ 1];
        for (pin, pname) in [(bcls::DSP_C::CEC, "CEC"), (bcls::DSP_C::RSTC, "RSTC")] {
            for (val, vname) in [(false, pname.to_string()), (true, format!("{pname}_B"))] {
                bctx.build()
                    .bel_mode(bslot, mode)
                    .bel_unused(obslot)
                    .bel_pin(bslot, pname)
                    .test_bel_input_inv(pin, val)
                    .bel_attr(bslot, format!("{pname}INV"), vname)
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::DSP;
    let bslot = bslots::DSP_C;
    for pin in [bcls::DSP_C::RSTC, bcls::DSP_C::CEC] {
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 4], tcid, bslot, pin);
    }
    let d0_0 = ctx.get_diff_bel_special(tcid, bslots::DSP[0], specials::DSP_CREG_0);
    let d0_1 = ctx.get_diff_bel_special(tcid, bslots::DSP[0], specials::DSP_CREG_1);
    let d1_0 = ctx.get_diff_bel_special(tcid, bslots::DSP[1], specials::DSP_CREG_0);
    let d1_1 = ctx.get_diff_bel_special(tcid, bslots::DSP[1], specials::DSP_CREG_1);
    let (d0_0, d1_0, dc_0) = Diff::split(d0_0, d1_0);
    let (d0_1, d1_1, dc_1) = Diff::split(d0_1, d1_1);
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DSP_C::CREG, xlat_bit_bi(dc_0, dc_1));
    d0_0.assert_empty();
    d1_0.assert_empty();
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DSP_C::MUX_CLK, xlat_bit_bi(d0_1, d1_1));
    for bslot in bslots::DSP {
        for pin in dsp48_invpins_int() {
            ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 4], tcid, bslot, pin);
        }
        for pin in dsp48_invpins_bel() {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        for attr in [bcls::DSP_V4::AREG, bcls::DSP_V4::BREG] {
            ctx.collect_bel_attr(tcid, bslot, attr);
            present.discard_bits(&ctx.bel_attr_enum(tcid, bslot, attr).bits);
        }
        for attr in [
            bcls::DSP_V4::MREG,
            bcls::DSP_V4::PREG,
            bcls::DSP_V4::OPMODEREG,
            bcls::DSP_V4::CARRYINREG,
            bcls::DSP_V4::CARRYINSELREG,
            bcls::DSP_V4::SUBTRACTREG,
        ] {
            ctx.collect_bel_attr_bi(tcid, bslot, attr);
            present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslot, attr)]);
        }
        ctx.collect_bel_attr(tcid, bslot, bcls::DSP_V4::B_INPUT);
        present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslots::DSP_C, bcls::DSP_C::CREG)]);
        present.discard_polbits(&[ctx.bel_attr_bit(tcid, bslots::DSP_C, bcls::DSP_C::MUX_CLK)]);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DSP_V4::UNK_ENABLE, xlat_bit(present));
    }
}
