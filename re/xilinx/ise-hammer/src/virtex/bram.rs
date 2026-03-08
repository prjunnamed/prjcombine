use prjcombine_re_hammer::Session;
use prjcombine_virtex::defs::{bcls::BRAM, bslots, enums, tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::FuzzCtx,
    virtex::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [
        tcls::BRAM_W,
        tcls::BRAM_E,
        tcls::BRAM_M,
        tcls::BRAM_W_S2,
        tcls::BRAM_E_S2,
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::BRAM);
        let mode = "BLOCKRAM";

        bctx.build()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for (p, pinmux, pin) in [
            (BRAM::CLKA, "CLKAMUX", "CLKA"),
            (BRAM::CLKB, "CLKBMUX", "CLKB"),
        ] {
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .pin(pin)
                .test_bel_input_inv_enum(pinmux, p, "1", "0");
        }
        for (p, pinmux, pin, pin_b) in [
            (BRAM::ENA, "ENAMUX", "ENA", "ENA_B"),
            (BRAM::ENB, "ENBMUX", "ENB", "ENB_B"),
            (BRAM::WEA, "WEAMUX", "WEA", "WEA_B"),
            (BRAM::WEB, "WEBMUX", "WEB", "WEB_B"),
            (BRAM::RSTA, "RSTAMUX", "RSTA", "RSTA_B"),
            (BRAM::RSTB, "RSTBMUX", "RSTB", "RSTB_B"),
        ] {
            for (val, vname) in [(false, "1"), (true, "0"), (false, pin), (true, pin_b)] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "256X16")
                    .attr("PORTB_ATTR", "256X16")
                    .pin(pin)
                    .test_bel_input_inv(p, val)
                    .attr(pinmux, vname)
                    .commit();
            }
        }
        for (attr, aname) in [
            (BRAM::DATA_WIDTH_A, "PORTA_ATTR"),
            (BRAM::DATA_WIDTH_B, "PORTB_ATTR"),
        ] {
            for (val, vname) in [
                (enums::BRAM_DATA_WIDTH::_1, "4096X1"),
                (enums::BRAM_DATA_WIDTH::_2, "2048X2"),
                (enums::BRAM_DATA_WIDTH::_4, "1024X4"),
                (enums::BRAM_DATA_WIDTH::_8, "512X8"),
                (enums::BRAM_DATA_WIDTH::_16, "256X16"),
            ] {
                bctx.mode(mode)
                    .test_bel_attr_val(attr, val)
                    .attr(aname, vname)
                    .commit();
            }
        }
        for i in 0..0x10 {
            let attr = format!("INIT_{i:02x}");
            bctx.mode(mode)
                .attr("PORTA_ATTR", "256X16")
                .attr("PORTB_ATTR", "256X16")
                .test_bel_attr_bits_base(BRAM::INIT, i * 0x100)
                .multi_attr(attr, MultiValue::Hex(0), 256);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [
        tcls::BRAM_W,
        tcls::BRAM_E,
        tcls::BRAM_M,
        tcls::BRAM_W_S2,
        tcls::BRAM_E_S2,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::BRAM;
        for pin in [
            BRAM::CLKA,
            BRAM::CLKB,
            BRAM::ENA,
            BRAM::ENB,
            BRAM::RSTA,
            BRAM::RSTB,
            BRAM::WEA,
            BRAM::WEB,
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        ctx.collect_bel_attr(tcid, bslot, BRAM::DATA_WIDTH_A);
        ctx.collect_bel_attr(tcid, bslot, BRAM::DATA_WIDTH_B);
        ctx.collect_bel_attr(tcid, bslot, BRAM::INIT);
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        present.discard_polbits(&[ctx.bel_input_inv(tcid, bslot, BRAM::ENA)]);
        present.discard_polbits(&[ctx.bel_input_inv(tcid, bslot, BRAM::ENB)]);
        present.assert_empty();
    }
}
