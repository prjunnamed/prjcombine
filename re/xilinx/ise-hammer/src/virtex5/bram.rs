use prjcombine_entity::EntityId;
use prjcombine_re_collector::diff::{
    Diff, xlat_bit, xlat_bit_wide_bi, xlat_bitvec, xlat_bitvec_sparse_u32, xlat_enum_attr,
};
use prjcombine_re_hammer::Session;
use prjcombine_types::bits;
use prjcombine_virtex4::defs::{bcls::BRAM_V5 as BRAM, bslots, enums, virtex5::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::BRAM);
    let mut bctx = ctx.bel(bslots::BRAM);

    for (spec, mode) in [
        (specials::BRAM_RAMBFIFO36, "RAMBFIFO36"),
        (specials::BRAM_RAMB18X2, "RAMB18X2"),
        (specials::BRAM_RAMB18X2SDP, "RAMB18X2SDP"),
        (specials::BRAM_RAMBFIFO18, "RAMBFIFO18"),
        (specials::BRAM_RAMBFIFO18_36, "RAMBFIFO18_36"),
        (specials::BRAM_RAMB36, "RAMB36_EXP"),
        (specials::BRAM_RAMB36SDP, "RAMB36SDP_EXP"),
        (specials::BRAM_FIFO36, "FIFO36_EXP"),
        (specials::BRAM_FIFO36_72, "FIFO36_72_EXP"),
    ] {
        bctx.build()
            .global_mutex("BRAM_OPT", "NONE")
            .test_bel_special(spec)
            .mode(mode)
            .commit();
    }

    for (attr, opt) in [
        (BRAM::TEST_FIFO_FLAG, "TEST_FIFO_FLAG"),
        (BRAM::TEST_FIFO_OFFSET, "TEST_FIFO_OFFSET"),
        (BRAM::TEST_FIFO_CNT, "TEST_FIFO_CNT"),
        (BRAM::SWAP_CFGPORT, "SWAP_CFGPORT"),
        (BRAM::BYPASS_RSR, "BYPASS_RSR"),
    ] {
        bctx.build()
            .global_mutex("BRAM_OPT", opt)
            .global(opt, "ENABLED")
            .test_bel_attr_bits(attr)
            .mode("FIFO36_EXP")
            .commit();
    }
    for (val, vname) in [
        (enums::BRAM_WW_VALUE::_0, "WW0"),
        (enums::BRAM_WW_VALUE::_1, "WW1"),
    ] {
        bctx.build()
            .global_mutex("BRAM_OPT", "WEAK_WRITE")
            .global("WEAK_WRITE", vname)
            .test_bel_attr_val(BRAM::WW_VALUE, val)
            .mode("FIFO36_EXP")
            .commit();
    }
    for (val, vname) in ["0", "1", "10", "11", "100", "101", "110", "111"]
        .into_iter()
        .enumerate()
    {
        bctx.build()
            .global_mutex("BRAM_OPT", "TRD_DLY")
            .global("TRD_DLY", vname)
            .test_bel_attr_u32(BRAM::TRD_DLY_L, val as u32)
            .mode("FIFO36_EXP")
            .commit();
    }
    for (val, vname) in [(0, "0"), (3, "11"), (5, "101"), (8, "1000")] {
        bctx.build()
            .global_mutex("BRAM_OPT", "TWR_DLY")
            .global("TWR_DLY", vname)
            .test_bel_attr_u32(BRAM::TWR_DLY_L, val as u32)
            .mode("FIFO36_EXP")
            .commit();
    }
    for (val, vname) in [(0, "0"), (5, "101"), (10, "1010"), (15, "1111")] {
        bctx.build()
            .global_mutex("BRAM_OPT", "TSCRUB_DLY")
            .global("TSCRUB_DLY", vname)
            .test_bel_attr_u32(BRAM::TSCRUB_DLY_L, val as u32)
            .mode("FIFO36_EXP")
            .commit();
    }

    for (pin, name) in [
        (BRAM::CLKAL, "CLKAL"),
        (BRAM::CLKAU, "CLKAU"),
        (BRAM::CLKBL, "CLKBL"),
        (BRAM::CLKBU, "CLKBU"),
        (BRAM::ENAL, "ENAL"),
        (BRAM::ENAU, "ENAU"),
        (BRAM::ENBL, "ENBL"),
        (BRAM::ENBU, "ENBU"),
        (BRAM::REGCLKAL, "REGCLKAL"),
        (BRAM::REGCLKAU, "REGCLKAU"),
        (BRAM::REGCLKBL, "REGCLKBL"),
        (BRAM::REGCLKBU, "REGCLKBU"),
        (BRAM::SSRAL, "SSRAL"),
        (BRAM::SSRAU, "SSRAU"),
        (BRAM::SSRBL, "SSRBL"),
        (BRAM::SSRBU, "SSRBU"),
    ] {
        bctx.mode("RAMB18X2")
            .attr("DOA_REG_L", "1")
            .attr("DOA_REG_U", "1")
            .pin(name)
            .test_bel_input_inv_enum(format!("{name}INV"), pin, name, format!("{name}_B"));
    }
    for (pin, name) in [
        (BRAM::CLKAL, "RDCLKL"),
        (BRAM::CLKAU, "RDCLKU"),
        (BRAM::CLKBL, "WRCLKL"),
        (BRAM::CLKBU, "WRCLKU"),
        (BRAM::ENAL, "RDENL"),
        (BRAM::ENAU, "RDENU"),
        (BRAM::ENBL, "WRENL"),
        (BRAM::ENBU, "WRENU"),
        (BRAM::REGCLKAL, "RDRCLKL"),
        (BRAM::REGCLKAU, "RDRCLKU"),
        (BRAM::SSRAL, "SSRL"),
        (BRAM::SSRAU, "SSRU"),
    ] {
        bctx.mode("RAMB18X2SDP")
            .attr("DO_REG_L", "1")
            .attr("DO_REG_U", "1")
            .pin(name)
            .test_bel_input_inv_enum(format!("{name}INV"), pin, name, format!("{name}_B"));
    }

    for (pin, mode, name) in [
        (BRAM::CLKAL, "RAMB36_EXP", "CLKAL"),
        (BRAM::CLKAU, "RAMB36_EXP", "CLKAU"),
        (BRAM::CLKBL, "RAMB36_EXP", "CLKBL"),
        (BRAM::CLKBU, "RAMB36_EXP", "CLKBU"),
        (BRAM::ENAL, "RAMB36_EXP", "ENAL"),
        (BRAM::ENAU, "RAMB36_EXP", "ENAU"),
        (BRAM::ENBL, "RAMB36_EXP", "ENBL"),
        (BRAM::ENBU, "RAMB36_EXP", "ENBU"),
        (BRAM::REGCLKAL, "RAMB36_EXP", "REGCLKAL"),
        (BRAM::REGCLKAU, "RAMB36_EXP", "REGCLKAU"),
        (BRAM::REGCLKBL, "RAMB36_EXP", "REGCLKBL"),
        (BRAM::REGCLKBU, "RAMB36_EXP", "REGCLKBU"),
        (BRAM::SSRAL, "RAMB36_EXP", "SSRAL"),
        (BRAM::SSRAU, "RAMB36_EXP", "SSRAU"),
        (BRAM::SSRBL, "RAMB36_EXP", "SSRBL"),
        (BRAM::SSRBU, "RAMB36_EXP", "SSRBU"),
        (BRAM::CLKAL, "RAMB36SDP_EXP", "RDCLKL"),
        (BRAM::CLKAU, "RAMB36SDP_EXP", "RDCLKU"),
        (BRAM::CLKBL, "RAMB36SDP_EXP", "WRCLKL"),
        (BRAM::CLKBU, "RAMB36SDP_EXP", "WRCLKU"),
        (BRAM::ENAL, "RAMB36SDP_EXP", "RDENL"),
        (BRAM::ENAU, "RAMB36SDP_EXP", "RDENU"),
        (BRAM::ENBL, "RAMB36SDP_EXP", "WRENL"),
        (BRAM::ENBU, "RAMB36SDP_EXP", "WRENU"),
        (BRAM::REGCLKAL, "RAMB36SDP_EXP", "RDRCLKL"),
        (BRAM::REGCLKAU, "RAMB36SDP_EXP", "RDRCLKU"),
        (BRAM::SSRAL, "RAMB36SDP_EXP", "SSRL"),
        (BRAM::SSRAU, "RAMB36SDP_EXP", "SSRU"),
        (BRAM::CLKAL, "FIFO36_EXP", "RDCLKL"),
        (BRAM::CLKAU, "FIFO36_EXP", "RDCLKU"),
        (BRAM::CLKBL, "FIFO36_EXP", "WRCLKL"),
        (BRAM::CLKBU, "FIFO36_EXP", "WRCLKU"),
        (BRAM::ENAL, "FIFO36_EXP", "RDEN"),
        (BRAM::ENBL, "FIFO36_EXP", "WREN"),
        (BRAM::REGCLKAL, "FIFO36_EXP", "RDRCLKL"),
        (BRAM::REGCLKAU, "FIFO36_EXP", "RDRCLKU"),
        (BRAM::SSRAL, "FIFO36_EXP", "RST"),
        (BRAM::CLKAL, "FIFO36_72_EXP", "RDCLKL"),
        (BRAM::CLKAU, "FIFO36_72_EXP", "RDCLKU"),
        (BRAM::CLKBL, "FIFO36_72_EXP", "WRCLKL"),
        (BRAM::CLKBU, "FIFO36_72_EXP", "WRCLKU"),
        (BRAM::ENAL, "FIFO36_72_EXP", "RDEN"),
        (BRAM::ENBL, "FIFO36_72_EXP", "WREN"),
        (BRAM::REGCLKAL, "FIFO36_72_EXP", "RDRCLKL"),
        (BRAM::REGCLKAU, "FIFO36_72_EXP", "RDRCLKU"),
        (BRAM::SSRAL, "FIFO36_72_EXP", "RST"),
    ] {
        bctx.mode(mode).pin(name).test_bel_input_inv_enum(
            format!("{name}INV"),
            pin,
            name,
            format!("{name}_B"),
        );
    }

    for (pin, name) in [
        (BRAM::CLKAL, "RDCLK"),
        (BRAM::CLKAU, "CLKA"),
        (BRAM::CLKBL, "WRCLK"),
        (BRAM::CLKBU, "CLKB"),
        (BRAM::REGCLKAL, "RDRCLK"),
        (BRAM::REGCLKAU, "REGCLKA"),
        (BRAM::REGCLKBU, "REGCLKB"),
        (BRAM::SSRAL, "RST"),
        (BRAM::SSRAU, "SSRA"),
        (BRAM::SSRBU, "SSRB"),
        (BRAM::ENAL, "RDEN"),
        (BRAM::ENAU, "ENA"),
        (BRAM::ENBL, "WREN"),
        (BRAM::ENBU, "ENB"),
    ] {
        bctx.mode("RAMBFIFO18")
            .attr("DO_REG", "1")
            .attr("DOA_REG", "1")
            .attr("EN_SYN", "FALSE")
            .pin(name)
            .test_bel_input_inv_enum(format!("{name}INV"), pin, name, format!("{name}_B"));
    }
    for (pin, name) in [
        (BRAM::CLKAL, "RDCLK"),
        (BRAM::CLKAU, "RDCLKU"),
        (BRAM::CLKBL, "WRCLK"),
        (BRAM::CLKBU, "WRCLKU"),
        (BRAM::REGCLKAL, "RDRCLK"),
        (BRAM::REGCLKAU, "RDRCLKU"),
        (BRAM::SSRAL, "RST"),
        (BRAM::SSRAU, "SSRU"),
        (BRAM::ENAL, "RDEN"),
        (BRAM::ENAU, "RDENU"),
        (BRAM::ENBL, "WREN"),
        (BRAM::ENBU, "WRENU"),
    ] {
        bctx.mode("RAMBFIFO18_36")
            .attr("DO_REG_L", "1")
            .attr("DO_REG_U", "1")
            .attr("EN_SYN", "FALSE")
            .pin(name)
            .test_bel_input_inv_enum(format!("{name}INV"), pin, name, format!("{name}_B"));
    }

    for mode in ["RAMBFIFO36", "RAMB36SDP_EXP", "FIFO36_72_EXP"] {
        bctx.mode(mode)
            .attr("EN_ECC_WRITE", "FALSE")
            .test_bel_attr_bool_auto(BRAM::EN_ECC_READ, "FALSE", "TRUE");
        if mode == "RAMBFIFO36" {
            bctx.mode(mode)
                .test_bel_attr_bool_auto(BRAM::EN_ECC_WRITE, "FALSE", "TRUE");
        } else {
            bctx.mode(mode)
                .attr("EN_ECC_READ", "TRUE")
                .test_bel_attr_bool_auto(BRAM::EN_ECC_WRITE, "FALSE", "TRUE");
            bctx.mode(mode)
                .attr("EN_ECC_READ", "FALSE")
                .test_bel_attr_bool_rename(
                    "EN_ECC_WRITE",
                    BRAM::EN_ECC_WRITE_NO_READ,
                    "FALSE",
                    "TRUE",
                );
        }
    }
    bctx.mode("RAMBFIFO36")
        .global_mutex("BRAM_OPT", "NONE")
        .test_bel_attr_bool_auto(BRAM::EN_ECC_SCRUB, "FALSE", "TRUE");
    bctx.mode("RAMB36SDP_EXP")
        .global_mutex("BRAM_OPT", "NONE")
        .test_bel_attr_bool_special_auto(
            BRAM::EN_ECC_SCRUB,
            specials::BRAM_RAMB36SDP,
            "FALSE",
            "TRUE",
        );

    bctx.mode("RAMBFIFO36")
        .test_bel_attr_bool_auto(BRAM::EN_SYN, "FALSE", "TRUE");

    for (spec, mode) in [
        (specials::BRAM_RAMBFIFO18, "RAMBFIFO18"),
        (specials::BRAM_RAMBFIFO18, "RAMBFIFO18_36"),
        (specials::BRAM_FIFO36, "FIFO36_EXP"),
        (specials::BRAM_FIFO36, "FIFO36_72_EXP"),
    ] {
        bctx.mode(mode)
            .test_bel_attr_bool_special_auto(BRAM::EN_SYN, spec, "FALSE", "TRUE");
    }

    for mode in [
        "RAMBFIFO36",
        "RAMBFIFO18",
        "RAMBFIFO18_36",
        "FIFO36_EXP",
        "FIFO36_72_EXP",
    ] {
        bctx.mode(mode)
            .test_bel_attr_bool_auto(BRAM::FIRST_WORD_FALL_THROUGH, "FALSE", "TRUE");
        if mode != "RAMBFIFO36" {
            bctx.mode(mode)
                .attr("EN_SYN", "TRUE")
                .test_bel_attr_multi(BRAM::ALMOST_FULL_OFFSET, MultiValue::Hex(0));
            bctx.mode(mode)
                .attr("EN_SYN", "TRUE")
                .test_bel_attr_multi(BRAM::ALMOST_EMPTY_OFFSET, MultiValue::Hex(0));
        }
    }
    bctx.mode("RAMBFIFO36").test_bel_attr_bool_rename(
        "IS_FIFO",
        BRAM::FIFO_ENABLE_L,
        "FALSE",
        "TRUE",
    );

    for (attr, mode, aname, init, srval) in [
        (
            BRAM::DOA_REG_L,
            "RAMBFIFO36",
            "DOA_REG_L",
            "INIT_A_L",
            "SRVAL_A_L",
        ),
        (
            BRAM::DOA_REG_U,
            "RAMBFIFO36",
            "DOA_REG_U",
            "INIT_A_U",
            "SRVAL_A_U",
        ),
        (
            BRAM::DOB_REG_L,
            "RAMBFIFO36",
            "DOB_REG_L",
            "INIT_B_L",
            "SRVAL_B_L",
        ),
        (
            BRAM::DOB_REG_U,
            "RAMBFIFO36",
            "DOB_REG_U",
            "INIT_B_U",
            "SRVAL_B_U",
        ),
        (
            BRAM::DOA_REG_L,
            "RAMB18X2",
            "DOA_REG_L",
            "INIT_A_L",
            "SRVAL_A_L",
        ),
        (
            BRAM::DOA_REG_U,
            "RAMB18X2",
            "DOA_REG_U",
            "INIT_A_U",
            "SRVAL_A_U",
        ),
        (
            BRAM::DOB_REG_L,
            "RAMB18X2",
            "DOB_REG_L",
            "INIT_B_L",
            "SRVAL_B_L",
        ),
        (
            BRAM::DOB_REG_U,
            "RAMB18X2",
            "DOB_REG_U",
            "INIT_B_U",
            "SRVAL_B_U",
        ),
        (
            BRAM::DOA_REG_U,
            "RAMBFIFO18",
            "DOA_REG",
            "DOB_REG",
            "SRVAL_A",
        ),
        (
            BRAM::DOB_REG_U,
            "RAMBFIFO18",
            "DOB_REG",
            "DOA_REG",
            "SRVAL_B",
        ),
    ] {
        bctx.mode(mode)
            .attr(init, "0")
            .attr(srval, "0")
            .test_bel_attr_bool_rename(aname, attr, "0", "1");
    }

    for (attr, spec, mode, aname, init, srval) in [
        (
            BRAM::DOA_REG_L,
            specials::BRAM_RAMB18X2SDP,
            "RAMB18X2SDP",
            "DO_REG_L",
            "INIT_L",
            "SRVAL_L",
        ),
        (
            BRAM::DOA_REG_U,
            specials::BRAM_RAMB18X2SDP,
            "RAMB18X2SDP",
            "DO_REG_U",
            "INIT_U",
            "SRVAL_U",
        ),
        (
            BRAM::DOA_REG_L,
            specials::BRAM_RAMB36,
            "RAMB36_EXP",
            "DOA_REG",
            "INIT_A",
            "SRVAL_A",
        ),
        (
            BRAM::DOB_REG_L,
            specials::BRAM_RAMB36,
            "RAMB36_EXP",
            "DOB_REG",
            "INIT_B",
            "SRVAL_B",
        ),
        (
            BRAM::DOA_REG_L,
            specials::BRAM_RAMB36SDP,
            "RAMB36SDP_EXP",
            "DO_REG",
            "INIT",
            "SRVAL",
        ),
    ] {
        bctx.mode(mode)
            .attr(init, "0")
            .attr(srval, "0")
            .test_bel_attr_bool_special_rename(aname, attr, spec, "0", "1");
    }
    bctx.mode("RAMBFIFO18_36")
        .attr("INIT", "fffffffff")
        .attr("SRVAL", "fffffffff")
        .test_bel_attr_bool_special_rename(
            "DO_REG_U",
            BRAM::DOA_REG_U,
            specials::BRAM_RAMBFIFO18_36,
            "0",
            "1",
        );

    bctx.mode("RAMBFIFO18")
        .attr("EN_SYN", "TRUE")
        .test_bel_attr_bool_rename("DO_REG", BRAM::DOA_REG_L, "0", "1");
    for (spec, mode, attr) in [
        (specials::BRAM_RAMBFIFO18_36, "RAMBFIFO18_36", "DO_REG_L"),
        (specials::BRAM_FIFO36, "FIFO36_EXP", "DO_REG"),
        (specials::BRAM_FIFO36_72, "FIFO36_72_EXP", "DO_REG"),
    ] {
        bctx.mode(mode)
            .attr("EN_SYN", "TRUE")
            .test_bel_attr_bool_special_rename(attr, BRAM::DOA_REG_L, spec, "0", "1");
    }

    for (val, vname) in [
        (enums::BRAM_V5_FIFO_WIDTH::_4, "4"),
        (enums::BRAM_V5_FIFO_WIDTH::_9, "9"),
        (enums::BRAM_V5_FIFO_WIDTH::_18, "18"),
    ] {
        bctx.mode("RAMBFIFO18")
            .test_bel_attr_special_val(BRAM::FIFO_WIDTH, specials::BRAM_RAMBFIFO18, val)
            .attr("DATA_WIDTH", vname)
            .commit();
    }
    for (val, vname) in [
        (enums::BRAM_V5_FIFO_WIDTH::_2, "4"),
        (enums::BRAM_V5_FIFO_WIDTH::_4, "9"),
        (enums::BRAM_V5_FIFO_WIDTH::_9, "18"),
        (enums::BRAM_V5_FIFO_WIDTH::_18, "36"),
    ] {
        bctx.mode("FIFO36_EXP")
            .test_bel_attr_special_val(BRAM::FIFO_WIDTH, specials::BRAM_FIFO36, val)
            .attr("DATA_WIDTH", vname)
            .commit();
    }
    for (ab, ul, attr_read, attr_write) in [
        ('A', 'L', BRAM::READ_WIDTH_A_L, BRAM::WRITE_WIDTH_A_L),
        ('A', 'U', BRAM::READ_WIDTH_A_U, BRAM::WRITE_WIDTH_A_U),
        ('B', 'L', BRAM::READ_WIDTH_B_L, BRAM::WRITE_WIDTH_B_L),
        ('B', 'U', BRAM::READ_WIDTH_B_U, BRAM::WRITE_WIDTH_B_U),
    ] {
        bctx.mode("RAMBFIFO36")
            .test_bel_attr_special(attr_read, specials::BRAM_SDP)
            .attr(format!("READ_WIDTH_{ab}_{ul}"), "36")
            .attr(format!("DO{ab}_REG_{ul}"), "0")
            .attr(format!("INIT_{ab}_{ul}"), "0")
            .attr(format!("SRVAL_{ab}_{ul}"), "0")
            .commit();
        bctx.mode("RAMBFIFO36")
            .test_bel_attr_special(attr_write, specials::BRAM_SDP)
            .attr(format!("WRITE_WIDTH_{ab}_{ul}"), "36")
            .commit();

        bctx.mode("RAMB18X2")
            .null_bits()
            .attr(format!("INIT_{ab}_{ul}"), "0")
            .attr(format!("SRVAL_{ab}_{ul}"), "0")
            .test_bel_attr_special(attr_read, specials::BRAM_DATA_WIDTH_0)
            .attr(format!("READ_WIDTH_{ab}_{ul}"), "0")
            .commit();
        bctx.mode("RAMB18X2")
            .null_bits()
            .pin(format!("WE{ab}{ul}0"))
            .pin(format!("WE{ab}{ul}1"))
            .pin(format!("WE{ab}{ul}2"))
            .pin(format!("WE{ab}{ul}3"))
            .test_bel_attr_special(attr_write, specials::BRAM_DATA_WIDTH_0)
            .attr(format!("WRITE_WIDTH_{ab}_{ul}"), "0")
            .commit();

        bctx.mode("RAMB18X2")
            .attr(format!("INIT_{ab}_{ul}"), "0")
            .attr(format!("SRVAL_{ab}_{ul}"), "0")
            .test_bel_attr_auto(attr_read);
        bctx.mode("RAMB18X2")
            .pin(format!("WE{ab}{ul}0"))
            .pin(format!("WE{ab}{ul}1"))
            .pin(format!("WE{ab}{ul}2"))
            .pin(format!("WE{ab}{ul}3"))
            .test_bel_attr_auto(attr_write);
    }
    for (ab, attr_read, attr_write) in [
        ('A', BRAM::READ_WIDTH_A_U, BRAM::WRITE_WIDTH_A_U),
        ('B', BRAM::READ_WIDTH_B_U, BRAM::WRITE_WIDTH_B_U),
    ] {
        bctx.mode("RAMBFIFO18")
            .null_bits()
            .attr(format!("DO{ab}_REG"), "0")
            .pin(format!("WE{ab}0"))
            .pin(format!("WE{ab}1"))
            .pin(format!("WE{ab}2"))
            .pin(format!("WE{ab}3"))
            .test_bel_attr_special(attr_write, specials::BRAM_DATA_WIDTH_0)
            .attr(format!("WRITE_WIDTH_{ab}"), "0")
            .commit();

        bctx.mode("RAMBFIFO18")
            .attr(format!("DO{ab}_REG"), "0")
            .attr(format!("INIT_{ab}"), "0")
            .attr(format!("SRVAL_{ab}"), "0")
            .test_bel_attr_special_rename(
                format!("READ_WIDTH_{ab}"),
                attr_read,
                specials::BRAM_RAMBFIFO18,
            );
        bctx.mode("RAMBFIFO18")
            .attr(format!("DO{ab}_REG"), "0")
            .pin(format!("WE{ab}0"))
            .pin(format!("WE{ab}1"))
            .pin(format!("WE{ab}2"))
            .pin(format!("WE{ab}3"))
            .test_bel_attr_rename(format!("WRITE_WIDTH_{ab}"), attr_write);
    }
    for (ab, attr_read, attr_write, attr_read_mux, attr_write_mux) in [
        (
            'A',
            BRAM::READ_WIDTH_A_L,
            BRAM::WRITE_WIDTH_A_L,
            BRAM::READ_MUX_UL_A,
            BRAM::WRITE_MUX_UL_A,
        ),
        (
            'B',
            BRAM::READ_WIDTH_B_L,
            BRAM::WRITE_WIDTH_B_L,
            BRAM::READ_MUX_UL_B,
            BRAM::WRITE_MUX_UL_B,
        ),
    ] {
        bctx.mode("RAMB36_EXP")
            .null_bits()
            .attr(format!("INIT_{ab}"), "0")
            .attr(format!("SRVAL_{ab}"), "0")
            .test_bel_attr_special(attr_read, specials::BRAM_DATA_WIDTH_0)
            .attr(format!("READ_WIDTH_{ab}"), "0")
            .commit();
        bctx.mode("RAMB36_EXP")
            .null_bits()
            .test_bel_attr_special(attr_write, specials::BRAM_DATA_WIDTH_0)
            .attr(format!("WRITE_WIDTH_{ab}"), "0")
            .commit();

        bctx.mode("RAMB36_EXP")
            .attr(format!("INIT_{ab}"), "0")
            .attr(format!("SRVAL_{ab}"), "0")
            .test_bel_attr_bits(attr_read_mux)
            .attr(format!("READ_WIDTH_{ab}"), "1")
            .commit();
        bctx.mode("RAMB36_EXP")
            .test_bel_attr_bits(attr_write_mux)
            .attr(format!("WRITE_WIDTH_{ab}"), "1")
            .commit();

        for (val, vname) in [
            (enums::BRAM_V5_DATA_WIDTH::_1, "2"),
            (enums::BRAM_V5_DATA_WIDTH::_2, "4"),
            (enums::BRAM_V5_DATA_WIDTH::_4, "9"),
            (enums::BRAM_V5_DATA_WIDTH::_9, "18"),
            (enums::BRAM_V5_DATA_WIDTH::_18, "36"),
        ] {
            bctx.mode("RAMB36_EXP")
                .attr(format!("INIT_{ab}"), "0")
                .attr(format!("SRVAL_{ab}"), "0")
                .test_bel_attr_special_val(attr_read, specials::BRAM_RAMB36, val)
                .attr(format!("READ_WIDTH_{ab}"), vname)
                .commit();
            bctx.mode("RAMB36_EXP")
                .test_bel_attr_special_val(attr_write, specials::BRAM_RAMB36, val)
                .attr(format!("WRITE_WIDTH_{ab}"), vname)
                .commit();
        }
    }

    for (attr, mode, aname) in [
        (BRAM::WRITE_MODE_A_L, "RAMBFIFO36", "WRITE_MODE_A_L"),
        (BRAM::WRITE_MODE_A_U, "RAMBFIFO36", "WRITE_MODE_A_U"),
        (BRAM::WRITE_MODE_B_L, "RAMBFIFO36", "WRITE_MODE_B_L"),
        (BRAM::WRITE_MODE_B_U, "RAMBFIFO36", "WRITE_MODE_B_U"),
        (BRAM::WRITE_MODE_A_L, "RAMB18X2", "WRITE_MODE_A_L"),
        (BRAM::WRITE_MODE_A_U, "RAMB18X2", "WRITE_MODE_A_U"),
        (BRAM::WRITE_MODE_B_L, "RAMB18X2", "WRITE_MODE_B_L"),
        (BRAM::WRITE_MODE_B_U, "RAMB18X2", "WRITE_MODE_B_U"),
        (BRAM::WRITE_MODE_A_U, "RAMBFIFO18", "WRITE_MODE_A"),
        (BRAM::WRITE_MODE_B_U, "RAMBFIFO18", "WRITE_MODE_B"),
    ] {
        bctx.mode(mode).test_bel_attr_rename(aname, attr);
    }
    for (attr, aname) in [
        (BRAM::WRITE_MODE_A_L, "WRITE_MODE_A"),
        (BRAM::WRITE_MODE_B_L, "WRITE_MODE_B"),
    ] {
        bctx.mode("RAMB36_EXP")
            .test_bel_attr_special_rename(aname, attr, specials::BRAM_RAMB36);
    }

    for mode in ["RAMBFIFO36", "RAMB36_EXP"] {
        for (attr, aname) in [
            (BRAM::RAM_EXTENSION_A_LOWER, "RAM_EXTENSION_A"),
            (BRAM::RAM_EXTENSION_B_LOWER, "RAM_EXTENSION_B"),
        ] {
            for (val, vname) in [(false, "NONE"), (false, "UPPER"), (true, "LOWER")] {
                bctx.mode(mode)
                    .test_bel_attr_bits_bi(attr, val)
                    .attr(aname, vname)
                    .commit();
            }
        }
    }

    for (aname, attr_a_l, attr_a_u, attr_b_l, attr_b_u) in [
        (
            "INIT",
            BRAM::INIT_A_L,
            BRAM::INIT_A_U,
            BRAM::INIT_B_L,
            BRAM::INIT_B_U,
        ),
        (
            "SRVAL",
            BRAM::SRVAL_A_L,
            BRAM::SRVAL_A_U,
            BRAM::SRVAL_B_L,
            BRAM::SRVAL_B_U,
        ),
    ] {
        for (ab, ul, attr) in [
            ('A', 'L', attr_a_l),
            ('A', 'U', attr_a_u),
            ('B', 'L', attr_b_l),
            ('B', 'U', attr_b_u),
        ] {
            bctx.mode("RAMBFIFO36")
                .attr("IS_FIFO", "FALSE")
                .attr(format!("READ_WIDTH_{ab}_{ul}"), "18")
                .test_bel_attr_bits(attr)
                .multi_attr(format!("{aname}_{ab}_{ul}"), MultiValue::Hex(0), 18);
            bctx.mode("RAMB18X2")
                .attr(format!("READ_WIDTH_{ab}_{ul}"), "18")
                .test_bel_attr_bits(attr)
                .multi_attr(format!("{aname}_{ab}_{ul}"), MultiValue::Hex(0), 18);
        }
        for (ab, attr) in [('A', attr_a_l), ('B', attr_b_l)] {
            bctx.mode("RAMB36_EXP")
                .attr(format!("READ_WIDTH_{ab}"), "36")
                .test_bel_attr_special_bits(attr, specials::BRAM_RAMB36, 0)
                .multi_attr(format!("{aname}_{ab}"), MultiValue::Hex(0), 36);
        }
        for (ul, attr) in [('U', attr_a_u), ('L', attr_a_l)] {
            bctx.mode("RAMB18X2SDP")
                .attr(format!("DO_REG_{ul}"), "0")
                .test_bel_attr_special_bits(attr, specials::BRAM_RAMB18X2SDP, 0)
                .multi_attr(format!("{aname}_{ul}"), MultiValue::Hex(0), 36);
        }
        bctx.mode("RAMB36SDP_EXP")
            .test_bel_attr_special_bits(attr_a_l, specials::BRAM_RAMB36SDP, 0)
            .multi_attr(aname, MultiValue::Hex(0), 72);
    }

    for mode in ["RAMB18X2", "RAMB18X2SDP"] {
        for (ul, data, datap) in [
            ('L', BRAM::DATA_L, BRAM::DATAP_L),
            ('U', BRAM::DATA_U, BRAM::DATAP_U),
        ] {
            for i in 0..0x40 {
                bctx.mode(mode)
                    .attr(
                        format!("READ_WIDTH_A_{ul}"),
                        if mode == "RAMB18X2SDP" { "" } else { "18" },
                    )
                    .attr(
                        format!("DO_REG_{ul}"),
                        if mode == "RAMB18X2SDP" { "1" } else { "" },
                    )
                    .attr("IS_FIFO", if mode == "RAMBFIFO36" { "FALSE" } else { "" })
                    .test_bel_attr_bits_base(data, i * 0x100)
                    .multi_attr(format!("INIT_{i:02X}_{ul}"), MultiValue::Hex(0), 256);
            }
            for i in 0..8 {
                bctx.mode(mode)
                    .attr(
                        format!("READ_WIDTH_A_{ul}"),
                        if mode == "RAMB18X2SDP" { "" } else { "18" },
                    )
                    .attr(
                        format!("DO_REG_{ul}"),
                        if mode == "RAMB18X2SDP" { "1" } else { "" },
                    )
                    .attr("IS_FIFO", if mode == "RAMBFIFO36" { "FALSE" } else { "" })
                    .test_bel_attr_bits_base(datap, i * 0x100)
                    .multi_attr(format!("INITP_{i:02X}_{ul}"), MultiValue::Hex(0), 256);
            }
        }
    }
    for mode in ["RAMBFIFO18", "RAMBFIFO18_36"] {
        for i in 0..0x40 {
            bctx.mode(mode)
                .attr("DOA_REG", if mode == "RAMBFIFO18_36" { "" } else { "1" })
                .attr("DO_REG_U", if mode == "RAMBFIFO18_36" { "1" } else { "" })
                .test_bel_attr_bits_base(BRAM::DATA_U, i * 0x100)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..8 {
            bctx.mode(mode)
                .attr("DOA_REG", if mode == "RAMBFIFO18_36" { "" } else { "1" })
                .attr("DO_REG_U", if mode == "RAMBFIFO18_36" { "1" } else { "" })
                .test_bel_attr_bits_base(BRAM::DATAP_U, i * 0x100)
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }
    }
    for mode in ["RAMB36_EXP", "RAMB36SDP_EXP"] {
        for i in 0..0x80 {
            bctx.mode(mode)
                .attr(
                    "READ_WIDTH_A",
                    if mode == "RAMB36SDP_EXP" { "" } else { "36" },
                )
                .test_bel_attr_special_bits(BRAM::DATA_L, specials::BRAM_RAMB36, i * 0x100)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..0x10 {
            bctx.mode(mode)
                .attr(
                    "READ_WIDTH_A",
                    if mode == "RAMB36SDP_EXP" { "" } else { "36" },
                )
                .test_bel_attr_special_bits(BRAM::DATAP_L, specials::BRAM_RAMB36, i * 0x100)
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }
    }

    bctx.mode("RAMB36_EXP")
        .test_bel_attr_bool_auto(BRAM::SAVEDATA, "FALSE", "TRUE");
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::BRAM;
    let bslot = bslots::BRAM;

    let mut present_rambfifo36 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMBFIFO36);
    let mut present_ramb18x2 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMB18X2);
    let mut present_ramb18x2sdp = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMB18X2SDP);
    let mut present_ramb36 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMB36);
    let mut present_ramb36sdp = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMB36SDP);
    let mut present_rambfifo18 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMBFIFO18);
    let mut present_rambfifo18_36 =
        ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_RAMBFIFO18_36);
    let mut present_fifo36 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_FIFO36);
    let mut present_fifo36_72 = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_FIFO36_72);

    for attr in [
        BRAM::TEST_FIFO_FLAG,
        BRAM::TEST_FIFO_OFFSET,
        BRAM::TEST_FIFO_CNT,
        BRAM::SWAP_CFGPORT,
        BRAM::BYPASS_RSR,
    ] {
        let mut diff = ctx.get_diff_attr_bool(tcid, bslot, attr);
        diff = diff.combine(&!&present_fifo36);
        ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
    }
    let mut diffs = vec![(enums::BRAM_WW_VALUE::NONE, Diff::default())];
    for val in [enums::BRAM_WW_VALUE::_0, enums::BRAM_WW_VALUE::_1] {
        let mut diff = ctx.get_diff_attr_val(tcid, bslot, BRAM::WW_VALUE, val);
        diff = diff.combine(&!&present_fifo36);
        diffs.push((val, diff));
    }
    ctx.insert_bel_attr_enum(tcid, bslot, BRAM::WW_VALUE, xlat_enum_attr(diffs));

    fn split_diff_ul(diff: Diff) -> (Diff, Diff) {
        let mut diff_l = Diff::default();
        let mut diff_u = Diff::default();
        for (k, v) in diff.bits {
            if k.rect.to_idx() < 2 {
                diff_l.bits.insert(k, v);
            } else {
                diff_u.bits.insert(k, v);
            }
        }
        (diff_l, diff_u)
    }

    let mut diffs_l = vec![];
    let mut diffs_u = vec![];
    for val in 0..8 {
        let mut diff = ctx.get_diff_attr_u32(tcid, bslot, BRAM::TRD_DLY_L, val);
        diff = diff.combine(&!&present_fifo36);
        if val == 0 {
            diff.assert_empty();
        }
        let (diff_l, diff_u) = split_diff_ul(diff);
        diffs_l.push((val, diff_l));
        diffs_u.push((val, diff_u));
    }
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        BRAM::TRD_DLY_L,
        xlat_bitvec_sparse_u32(diffs_l),
    );
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        BRAM::TRD_DLY_U,
        xlat_bitvec_sparse_u32(diffs_u),
    );

    let diff_3 = ctx
        .peek_diff_attr_u32(tcid, bslot, BRAM::TWR_DLY_L, 3)
        .clone();
    let diff_5 = ctx
        .peek_diff_attr_u32(tcid, bslot, BRAM::TWR_DLY_L, 5)
        .clone();
    let (_, _, mut diff_1) = Diff::split(diff_3, diff_5);
    diff_1 = diff_1.combine(&!&present_fifo36);
    let (diff_l, diff_u) = split_diff_ul(diff_1);
    let mut diffs_l = vec![(1, diff_l)];
    let mut diffs_u = vec![(1, diff_u)];
    for val in [0, 3, 5, 8] {
        let mut diff = ctx.get_diff_attr_u32(tcid, bslot, BRAM::TWR_DLY_L, val);
        diff = diff.combine(&!&present_fifo36);
        if val == 0 {
            diff.assert_empty();
        }
        let (diff_l, diff_u) = split_diff_ul(diff);
        diffs_l.push((val, diff_l));
        diffs_u.push((val, diff_u));
    }
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        BRAM::TWR_DLY_L,
        xlat_bitvec_sparse_u32(diffs_l),
    );
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        BRAM::TWR_DLY_U,
        xlat_bitvec_sparse_u32(diffs_u),
    );

    for val in [0, 5, 10, 15] {
        let mut diff = ctx.get_diff_attr_u32(tcid, bslot, BRAM::TSCRUB_DLY_L, val);
        diff = diff.combine(&!&present_fifo36);
        if matches!(val, 5 | 15) {
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, BRAM::TWR_DLY_L), 8, 0);
            diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, BRAM::TWR_DLY_U), 8, 0);
        }
        if matches!(val, 10 | 15) {
            let (diff_l, diff_u) = split_diff_ul(diff);
            ctx.insert_bel_attr_bool(tcid, bslot, BRAM::TSCRUB_DLY_L, xlat_bit(diff_l));
            ctx.insert_bel_attr_bool(tcid, bslot, BRAM::TSCRUB_DLY_U, xlat_bit(diff_u));
        } else {
            diff.assert_empty();
        }
    }

    for pin in [
        BRAM::CLKAL,
        BRAM::CLKAU,
        BRAM::CLKBL,
        BRAM::CLKBU,
        BRAM::ENAL,
        BRAM::ENAU,
        BRAM::ENBL,
        BRAM::ENBU,
        BRAM::REGCLKAL,
        BRAM::REGCLKAU,
        BRAM::REGCLKBL,
        BRAM::REGCLKBU,
        BRAM::SSRAL,
        BRAM::SSRAU,
        BRAM::SSRBL,
        BRAM::SSRBU,
    ] {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        let bit = ctx.bel_input_inv(tcid, bslot, pin);
        present_rambfifo36.apply_bit_diff(bit, false, true);
        present_ramb18x2.apply_bit_diff(bit, false, true);
        present_ramb18x2sdp.apply_bit_diff(bit, false, true);
        present_ramb36.apply_bit_diff(bit, false, true);
        present_ramb36sdp.apply_bit_diff(bit, false, true);
        present_rambfifo18.apply_bit_diff(bit, false, true);
        present_rambfifo18_36.apply_bit_diff(bit, false, true);
        present_fifo36.apply_bit_diff(bit, false, true);
        present_fifo36_72.apply_bit_diff(bit, false, true);
    }

    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::EN_ECC_READ);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::EN_ECC_WRITE);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::EN_ECC_SCRUB);
    let eccwr = ctx.bel_attr_bit(tcid, bslot, BRAM::EN_ECC_WRITE);
    ctx.get_diff_attr_bool_bi(tcid, bslot, BRAM::EN_ECC_WRITE_NO_READ, false)
        .assert_empty();
    let mut diff = ctx.get_diff_attr_bool_bi(tcid, bslot, BRAM::EN_ECC_WRITE_NO_READ, true);
    diff.apply_bit_diff(eccwr, true, false);
    ctx.insert_bel_attr_bool(tcid, bslot, BRAM::EN_ECC_WRITE_NO_READ, xlat_bit(diff));

    ctx.get_diff_attr_special_bit_bi(
        tcid,
        bslot,
        BRAM::EN_ECC_SCRUB,
        specials::BRAM_RAMB36SDP,
        0,
        false,
    )
    .assert_empty();
    let mut diff = ctx.get_diff_attr_special_bit_bi(
        tcid,
        bslot,
        BRAM::EN_ECC_SCRUB,
        specials::BRAM_RAMB36SDP,
        0,
        true,
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, BRAM::EN_ECC_SCRUB),
        true,
        false,
    );
    diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, BRAM::TWR_DLY_L), 8, 0);
    diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, BRAM::TWR_DLY_U), 8, 0);
    diff.assert_empty();

    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::FIFO_ENABLE_L);
    let bit = ctx.bel_attr_bit(tcid, bslot, BRAM::FIFO_ENABLE_L);
    present_rambfifo18.apply_bit_diff(bit, true, false);
    present_rambfifo18_36.apply_bit_diff(bit, true, false);
    present_fifo36.apply_bit_diff(bit, true, false);
    present_fifo36_72.apply_bit_diff(bit, true, false);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::FIRST_WORD_FALL_THROUGH);
    for attr in [BRAM::ALMOST_FULL_OFFSET, BRAM::ALMOST_EMPTY_OFFSET] {
        ctx.collect_bel_attr(tcid, bslot, attr);
        let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
        present_rambfifo36.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_ramb18x2.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_ramb18x2sdp.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_ramb36.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_ramb36sdp.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_rambfifo18.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_rambfifo18_36.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_fifo36.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_fifo36_72.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
    }

    for (attr_l, attr_u) in [
        (BRAM::WRITE_MODE_A_L, BRAM::WRITE_MODE_A_U),
        (BRAM::WRITE_MODE_B_L, BRAM::WRITE_MODE_B_U),
    ] {
        for val in [
            enums::BRAM_WRITE_MODE::READ_FIRST,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
        ] {
            let diff =
                ctx.get_diff_attr_special_val(tcid, bslot, attr_l, specials::BRAM_RAMB36, val);
            let diff_l = ctx.peek_diff_attr_val(tcid, bslot, attr_l, val);
            let diff_u = ctx.peek_diff_attr_val(tcid, bslot, attr_u, val);
            assert_eq!(diff, diff_l.combine(diff_u));
        }
    }
    ctx.collect_bel_attr(tcid, bslot, BRAM::WRITE_MODE_A_L);
    ctx.collect_bel_attr(tcid, bslot, BRAM::WRITE_MODE_A_U);
    ctx.collect_bel_attr(tcid, bslot, BRAM::WRITE_MODE_B_L);
    ctx.collect_bel_attr(tcid, bslot, BRAM::WRITE_MODE_B_U);
    for attr in [
        BRAM::WRITE_MODE_A_L,
        BRAM::WRITE_MODE_A_U,
        BRAM::WRITE_MODE_B_L,
        BRAM::WRITE_MODE_B_U,
    ] {
        let item = ctx.bel_attr_enum(tcid, bslot, attr);
        present_ramb18x2sdp.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::READ_FIRST,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
        present_ramb36sdp.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::READ_FIRST,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
    }
    for attr in [BRAM::WRITE_MODE_A_L, BRAM::WRITE_MODE_B_L] {
        let item = ctx.bel_attr_enum(tcid, bslot, attr);
        present_rambfifo18.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
        present_rambfifo18_36.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
        present_fifo36.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
        present_fifo36_72.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
    }
    for attr in [BRAM::WRITE_MODE_A_U, BRAM::WRITE_MODE_B_U] {
        let item = ctx.bel_attr_enum(tcid, bslot, attr);
        present_rambfifo18_36.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
        present_fifo36.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
        present_fifo36_72.apply_enum_diff(
            item,
            enums::BRAM_WRITE_MODE::NO_CHANGE,
            enums::BRAM_WRITE_MODE::WRITE_FIRST,
        );
    }

    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::RAM_EXTENSION_A_LOWER);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::RAM_EXTENSION_B_LOWER);

    for (attr_a_l, attr_a_u, attr_b_l, attr_b_u) in [
        (
            BRAM::INIT_A_L,
            BRAM::INIT_A_U,
            BRAM::INIT_B_L,
            BRAM::INIT_B_U,
        ),
        (
            BRAM::SRVAL_A_L,
            BRAM::SRVAL_A_U,
            BRAM::SRVAL_B_L,
            BRAM::SRVAL_B_U,
        ),
    ] {
        for attr in [attr_a_l, attr_a_u, attr_b_l, attr_b_u] {
            ctx.collect_bel_attr(tcid, bslot, attr);
            let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
            present_rambfifo36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_ramb18x2.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_ramb18x2sdp.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_ramb36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_rambfifo18.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_rambfifo18_36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_fifo36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            present_fifo36_72.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
        }
        for (attr_l, attr_u) in [(attr_a_l, attr_a_u), (attr_b_l, attr_b_u)] {
            let diffs =
                ctx.get_diffs_attr_special_bits(tcid, bslot, attr_l, specials::BRAM_RAMB36, 36);
            let mut diffs_l = vec![];
            let mut diffs_u = vec![];
            for (i, diff) in diffs.into_iter().enumerate() {
                if i.is_multiple_of(2) {
                    diffs_l.push(diff);
                } else {
                    diffs_u.push(diff);
                }
            }
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr_l, xlat_bitvec(diffs_l));
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr_u, xlat_bitvec(diffs_u));
        }
        for (attr_a, attr_b) in [(attr_a_l, attr_b_l), (attr_a_u, attr_b_u)] {
            let mut diffs = ctx.get_diffs_attr_special_bits(
                tcid,
                bslot,
                attr_a,
                specials::BRAM_RAMB18X2SDP,
                36,
            );
            let diffs_b_hi = diffs.split_off(34);
            let diffs_a_hi = diffs.split_off(32);
            let mut diffs_b = diffs.split_off(16);
            diffs.extend(diffs_a_hi);
            diffs_b.extend(diffs_b_hi);
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr_a, xlat_bitvec(diffs));
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr_b, xlat_bitvec(diffs_b));
        }
        let mut diffs_a =
            ctx.get_diffs_attr_special_bits(tcid, bslot, attr_a_l, specials::BRAM_RAMB36SDP, 72);
        let diffs_b_hi = diffs_a.split_off(68);
        let diffs_a_hi = diffs_a.split_off(64);
        let mut diffs_b = diffs_a.split_off(32);
        diffs_a.extend(diffs_a_hi);
        diffs_b.extend(diffs_b_hi);
        let mut diffs_a_l = vec![];
        let mut diffs_a_u = vec![];
        let mut diffs_b_l = vec![];
        let mut diffs_b_u = vec![];
        for (i, diff) in diffs_a.into_iter().enumerate() {
            if i.is_multiple_of(2) {
                diffs_a_l.push(diff);
            } else {
                diffs_a_u.push(diff);
            }
        }
        for (i, diff) in diffs_b.into_iter().enumerate() {
            if i.is_multiple_of(2) {
                diffs_b_l.push(diff);
            } else {
                diffs_b_u.push(diff);
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr_a_l, xlat_bitvec(diffs_a_l));
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr_a_u, xlat_bitvec(diffs_a_u));
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr_b_l, xlat_bitvec(diffs_b_l));
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr_b_u, xlat_bitvec(diffs_b_u));
    }

    for val in [false, true] {
        let diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::DOA_REG_L,
            specials::BRAM_RAMB36SDP,
            0,
            val,
        );
        assert_eq!(
            diff,
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                BRAM::DOA_REG_L,
                specials::BRAM_FIFO36,
                0,
                val
            )
        );
        assert_eq!(
            diff,
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                BRAM::DOA_REG_L,
                specials::BRAM_FIFO36_72,
                0,
                val
            )
        );

        let diff_a = ctx.peek_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::DOA_REG_L,
            specials::BRAM_RAMB36,
            0,
            val,
        );
        let diff_b = ctx.peek_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::DOB_REG_L,
            specials::BRAM_RAMB36,
            0,
            val,
        );
        assert_eq!(diff, diff_a.combine(diff_b));

        let diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::DOA_REG_L,
            specials::BRAM_RAMBFIFO18_36,
            0,
            val,
        );
        assert_eq!(
            &diff,
            ctx.peek_diff_attr_special_bit_bi(
                tcid,
                bslot,
                BRAM::DOA_REG_L,
                specials::BRAM_RAMB18X2SDP,
                0,
                val
            )
        );
    }
    for (attr_l, attr_u) in [
        (BRAM::DOA_REG_L, BRAM::DOA_REG_U),
        (BRAM::DOB_REG_L, BRAM::DOB_REG_U),
    ] {
        for val in [false, true] {
            let diff = ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                attr_l,
                specials::BRAM_RAMB36,
                0,
                val,
            );
            let diff_l = ctx.peek_diff_attr_bool_bi(tcid, bslot, attr_l, val);
            let diff_u = ctx.peek_diff_attr_bool_bi(tcid, bslot, attr_u, val);
            assert_eq!(diff, diff_l.combine(diff_u));
        }
    }
    for (attr_a, attr_b) in [
        (BRAM::DOA_REG_L, BRAM::DOB_REG_L),
        (BRAM::DOA_REG_U, BRAM::DOB_REG_U),
    ] {
        for val in [false, true] {
            let diff = ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                attr_a,
                specials::BRAM_RAMB18X2SDP,
                0,
                val,
            );
            let diff_a = ctx.peek_diff_attr_bool_bi(tcid, bslot, attr_a, val);
            let diff_b = ctx.peek_diff_attr_bool_bi(tcid, bslot, attr_b, val);
            assert_eq!(diff, diff_a.combine(diff_b));
        }
    }

    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::DOA_REG_L);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::DOA_REG_U);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::DOB_REG_L);
    ctx.collect_bel_attr_bi(tcid, bslot, BRAM::DOB_REG_U);

    for attr in [BRAM::DOA_REG_L, BRAM::DOB_REG_L] {
        let item = ctx.bel_attr_bit(tcid, bslot, attr);
        present_rambfifo18.apply_bit_diff(item, true, false);
        present_rambfifo18_36.apply_bit_diff(item, true, false);
        present_fifo36.apply_bit_diff(item, true, false);
        present_fifo36_72.apply_bit_diff(item, true, false);
    }
    for attr in [BRAM::DOA_REG_U, BRAM::DOB_REG_U] {
        let item = ctx.bel_attr_bit(tcid, bslot, attr);
        present_fifo36.apply_bit_diff(item, true, false);
        present_fifo36_72.apply_bit_diff(item, true, false);
    }

    {
        ctx.collect_bel_attr_bi(tcid, bslot, BRAM::EN_SYN);
        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::EN_SYN,
            specials::BRAM_RAMBFIFO18,
            0,
            false,
        )
        .assert_empty();
        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::EN_SYN,
            specials::BRAM_FIFO36,
            0,
            false,
        )
        .assert_empty();

        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::EN_SYN,
            specials::BRAM_RAMBFIFO18,
            0,
            true,
        );
        diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOA_REG_L), false, true);
        diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOB_REG_L), false, true);
        ctx.insert_bel_attr_bool(tcid, bslot, BRAM::EN_SYN, xlat_bit(diff));

        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            BRAM::EN_SYN,
            specials::BRAM_FIFO36,
            0,
            true,
        );
        diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOA_REG_L), false, true);
        diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOA_REG_U), false, true);
        diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOB_REG_L), false, true);
        diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOB_REG_U), false, true);
        ctx.insert_bel_attr_bool(tcid, bslot, BRAM::EN_SYN, xlat_bit(diff));
    }

    let mut d0 = ctx.get_diff_attr_special_bit_bi(
        tcid,
        bslot,
        BRAM::DOA_REG_U,
        specials::BRAM_RAMBFIFO18_36,
        0,
        false,
    );
    let mut d1 = ctx.get_diff_attr_special_bit_bi(
        tcid,
        bslot,
        BRAM::DOA_REG_U,
        specials::BRAM_RAMBFIFO18_36,
        0,
        true,
    );
    d1 = d1.combine(&!&d0);
    for attr in [
        BRAM::SRVAL_A_L,
        BRAM::SRVAL_A_U,
        BRAM::SRVAL_B_L,
        BRAM::SRVAL_B_U,
        BRAM::INIT_A_L,
        BRAM::INIT_A_U,
        BRAM::INIT_B_L,
        BRAM::INIT_B_U,
    ] {
        d0.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, attr),
            &bits![1; 18],
            &bits![0; 18],
        );
    }
    d0.assert_empty();
    d1.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOA_REG_U), true, false);
    d1.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, BRAM::DOB_REG_U), true, false);
    d1.assert_empty();

    for (attr_l, attr_u, attr_mux) in [
        (
            BRAM::READ_WIDTH_A_L,
            BRAM::READ_WIDTH_A_U,
            BRAM::READ_MUX_UL_A,
        ),
        (
            BRAM::READ_WIDTH_B_L,
            BRAM::READ_WIDTH_B_U,
            BRAM::READ_MUX_UL_B,
        ),
        (
            BRAM::WRITE_WIDTH_A_L,
            BRAM::WRITE_WIDTH_A_U,
            BRAM::WRITE_MUX_UL_A,
        ),
        (
            BRAM::WRITE_WIDTH_B_L,
            BRAM::WRITE_WIDTH_B_U,
            BRAM::WRITE_MUX_UL_B,
        ),
    ] {
        ctx.collect_bel_attr(tcid, bslot, attr_mux);
        for val in [
            enums::BRAM_V5_DATA_WIDTH::_1,
            enums::BRAM_V5_DATA_WIDTH::_2,
            enums::BRAM_V5_DATA_WIDTH::_4,
            enums::BRAM_V5_DATA_WIDTH::_9,
            enums::BRAM_V5_DATA_WIDTH::_18,
        ] {
            let mut diff =
                ctx.get_diff_attr_special_val(tcid, bslot, attr_l, specials::BRAM_RAMB36, val);
            if val == enums::BRAM_V5_DATA_WIDTH::_4 {
                diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, attr_mux), true, false);
            }
            let diff_l = ctx.peek_diff_attr_val(tcid, bslot, attr_l, val);
            let diff_u = ctx.peek_diff_attr_val(tcid, bslot, attr_u, val);
            assert_eq!(diff, diff_l.combine(diff_u));
        }
    }
    for (attr_read, attr_init, attr_srval) in [
        (BRAM::READ_WIDTH_A_U, BRAM::INIT_A_L, BRAM::SRVAL_A_L),
        (BRAM::READ_WIDTH_B_U, BRAM::INIT_B_L, BRAM::SRVAL_B_L),
    ] {
        for (val, isr) in [
            // ????????? the fuck were they smoking
            (
                enums::BRAM_V5_DATA_WIDTH::_1,
                bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                enums::BRAM_V5_DATA_WIDTH::_2,
                bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (
                enums::BRAM_V5_DATA_WIDTH::_4,
                bits![1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (
                enums::BRAM_V5_DATA_WIDTH::_9,
                bits![1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (enums::BRAM_V5_DATA_WIDTH::_18, bits![1; 18]),
        ] {
            let mut diff = ctx.get_diff_attr_special_val(
                tcid,
                bslot,
                attr_read,
                specials::BRAM_RAMBFIFO18,
                val,
            );
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, attr_init),
                &isr,
                &bits![1; 18],
            );
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, attr_srval),
                &isr,
                &bits![1; 18],
            );
            assert_eq!(&diff, ctx.peek_diff_attr_val(tcid, bslot, attr_read, val));
        }
    }
    for attr in [
        BRAM::READ_WIDTH_A_L,
        BRAM::READ_WIDTH_A_U,
        BRAM::READ_WIDTH_B_L,
        BRAM::READ_WIDTH_B_U,
        BRAM::WRITE_WIDTH_A_L,
        BRAM::WRITE_WIDTH_A_U,
        BRAM::WRITE_WIDTH_B_L,
        BRAM::WRITE_WIDTH_B_U,
    ] {
        ctx.collect_bel_attr(tcid, bslot, attr);
        let item = ctx.bel_attr_enum(tcid, bslot, attr);
        present_ramb18x2sdp.apply_enum_diff(
            item,
            enums::BRAM_V5_DATA_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        present_ramb36sdp.apply_enum_diff(
            item,
            enums::BRAM_V5_DATA_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        present_rambfifo18_36.apply_enum_diff(
            item,
            enums::BRAM_V5_DATA_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        present_fifo36_72.apply_enum_diff(
            item,
            enums::BRAM_V5_DATA_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
    }
    for (attr, sdp) in [
        (BRAM::READ_WIDTH_A_L, BRAM::READ_SDP_L),
        (BRAM::READ_WIDTH_A_U, BRAM::READ_SDP_U),
        (BRAM::WRITE_WIDTH_A_L, BRAM::WRITE_SDP_L),
        (BRAM::WRITE_WIDTH_A_U, BRAM::WRITE_SDP_U),
    ] {
        let mut diff = ctx.get_diff_attr_special(tcid, bslot, attr, specials::BRAM_SDP);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, attr),
            enums::BRAM_V5_DATA_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        ctx.insert_bel_attr_bool(tcid, bslot, sdp, xlat_bit(diff));
    }
    for attr in [
        BRAM::READ_WIDTH_B_L,
        BRAM::READ_WIDTH_B_U,
        BRAM::WRITE_WIDTH_B_L,
        BRAM::WRITE_WIDTH_B_U,
    ] {
        let mut diff = ctx.get_diff_attr_special(tcid, bslot, attr, specials::BRAM_SDP);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, attr),
            enums::BRAM_V5_DATA_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.assert_empty();
    }

    for attr in [
        BRAM::READ_SDP_L,
        BRAM::READ_SDP_U,
        BRAM::WRITE_SDP_L,
        BRAM::WRITE_SDP_U,
    ] {
        let item = ctx.bel_attr_bit(tcid, bslot, attr);
        present_ramb18x2sdp.apply_bit_diff(item, true, false);
        present_ramb36sdp.apply_bit_diff(item, true, false);
        present_rambfifo18_36.apply_bit_diff(item, true, false);
        present_fifo36_72.apply_bit_diff(item, true, false);
    }

    ctx.collect_bel_attr(tcid, bslot, BRAM::DATA_L);
    ctx.collect_bel_attr(tcid, bslot, BRAM::DATA_U);
    ctx.collect_bel_attr(tcid, bslot, BRAM::DATAP_L);
    ctx.collect_bel_attr(tcid, bslot, BRAM::DATAP_U);

    {
        let data = ctx.get_diffs_attr_special_bits(
            tcid,
            bslot,
            BRAM::DATA_L,
            specials::BRAM_RAMB36,
            0x8000,
        );
        let datap = ctx.get_diffs_attr_special_bits(
            tcid,
            bslot,
            BRAM::DATAP_L,
            specials::BRAM_RAMB36,
            0x1000,
        );
        let mut data_l = vec![];
        let mut data_u = vec![];
        for (i, diff) in data.into_iter().enumerate() {
            if i.is_multiple_of(2) {
                data_l.push(diff);
            } else {
                data_u.push(diff);
            }
        }
        let mut datap_l = vec![];
        let mut datap_u = vec![];
        for (i, diff) in datap.into_iter().enumerate() {
            if i.is_multiple_of(2) {
                datap_l.push(diff);
            } else {
                datap_u.push(diff);
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, BRAM::DATA_L, xlat_bitvec(data_l));
        ctx.insert_bel_attr_bitvec(tcid, bslot, BRAM::DATA_U, xlat_bitvec(data_u));
        ctx.insert_bel_attr_bitvec(tcid, bslot, BRAM::DATAP_L, xlat_bitvec(datap_l));
        ctx.insert_bel_attr_bitvec(tcid, bslot, BRAM::DATAP_U, xlat_bitvec(datap_u));
    }

    let bits = xlat_bit_wide_bi(
        ctx.get_diff_attr_bool_bi(tcid, bslot, BRAM::SAVEDATA, false),
        ctx.get_diff_attr_bool_bi(tcid, bslot, BRAM::SAVEDATA, true),
    );
    ctx.insert_bel_attr_bitvec(tcid, bslot, BRAM::SAVEDATA, bits);

    present_rambfifo36.assert_empty();
    present_ramb18x2.assert_empty();
    present_ramb18x2sdp.assert_empty();
    present_ramb36.assert_empty();
    present_rambfifo18.assert_empty();

    let mut diffs = vec![];
    for (val, pval) in [
        (enums::BRAM_V5_FIFO_WIDTH::_4, enums::BRAM_V5_DATA_WIDTH::_4),
        (enums::BRAM_V5_FIFO_WIDTH::_9, enums::BRAM_V5_DATA_WIDTH::_9),
        (
            enums::BRAM_V5_FIFO_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_18,
        ),
    ] {
        let mut diff = ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            BRAM::FIFO_WIDTH,
            specials::BRAM_RAMBFIFO18,
            val,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::READ_WIDTH_A_L),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::READ_WIDTH_B_L),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::WRITE_WIDTH_A_L),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::WRITE_WIDTH_B_L),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diffs.push((val, diff));
    }
    for (val, pval) in [
        (enums::BRAM_V5_FIFO_WIDTH::_2, enums::BRAM_V5_DATA_WIDTH::_2),
        (enums::BRAM_V5_FIFO_WIDTH::_4, enums::BRAM_V5_DATA_WIDTH::_4),
        (enums::BRAM_V5_FIFO_WIDTH::_9, enums::BRAM_V5_DATA_WIDTH::_9),
        (
            enums::BRAM_V5_FIFO_WIDTH::_18,
            enums::BRAM_V5_DATA_WIDTH::_18,
        ),
    ] {
        let mut diff = ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            BRAM::FIFO_WIDTH,
            specials::BRAM_FIFO36,
            val,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::READ_WIDTH_A_L),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::WRITE_WIDTH_B_L),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::READ_WIDTH_A_U),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, BRAM::WRITE_WIDTH_B_U),
            pval,
            enums::BRAM_V5_DATA_WIDTH::_1,
        );
        if val == enums::BRAM_V5_FIFO_WIDTH::_4 {
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, BRAM::READ_MUX_UL_A),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, BRAM::READ_MUX_UL_B),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, BRAM::WRITE_MUX_UL_A),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, BRAM::WRITE_MUX_UL_B),
                true,
                false,
            );
        }
        if val == enums::BRAM_V5_FIFO_WIDTH::_18 {
            // what the fuck
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, BRAM::READ_WIDTH_B_L),
                pval,
                enums::BRAM_V5_DATA_WIDTH::_1,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, BRAM::WRITE_WIDTH_A_L),
                pval,
                enums::BRAM_V5_DATA_WIDTH::_1,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, BRAM::READ_WIDTH_B_U),
                pval,
                enums::BRAM_V5_DATA_WIDTH::_1,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, BRAM::WRITE_WIDTH_A_U),
                pval,
                enums::BRAM_V5_DATA_WIDTH::_1,
            );
        }
        diffs.push((val, diff));
    }
    diffs.push((enums::BRAM_V5_FIFO_WIDTH::_36, present_rambfifo18_36));
    ctx.insert_bel_attr_enum(tcid, bslot, BRAM::FIFO_WIDTH, xlat_enum_attr(diffs));

    present_ramb36sdp.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, BRAM::FIFO_WIDTH),
        enums::BRAM_V5_FIFO_WIDTH::_36,
        enums::BRAM_V5_FIFO_WIDTH::_2,
    );
    present_fifo36_72.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, BRAM::FIFO_WIDTH),
        enums::BRAM_V5_FIFO_WIDTH::_36,
        enums::BRAM_V5_FIFO_WIDTH::_2,
    );
    present_ramb36sdp.assert_empty();

    assert_eq!(present_fifo36, present_fifo36_72);
    ctx.insert_bel_attr_bool(tcid, bslot, BRAM::FIFO_ENABLE_U, xlat_bit(present_fifo36));
}
