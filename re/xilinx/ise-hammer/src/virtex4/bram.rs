use prjcombine_entity::EntityId;
use prjcombine_re_collector::diff::{Diff, xlat_bit, xlat_bit_wide, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{bcls, bslots, enums, virtex4::tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::BRAM);
    {
        let mut bctx = ctx.bel(bslots::BRAM);
        let mode = "RAMB16";
        bctx.build()
            .global_mutex("BRAM", "NOPE")
            .sub_unused(1)
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for pin in [
            bcls::BRAM_V4::CLKA,
            bcls::BRAM_V4::CLKB,
            bcls::BRAM_V4::ENA,
            bcls::BRAM_V4::ENB,
            bcls::BRAM_V4::SSRA,
            bcls::BRAM_V4::SSRB,
            bcls::BRAM_V4::REGCEA,
            bcls::BRAM_V4::REGCEB,
            bcls::BRAM_V4::WEA[0],
            bcls::BRAM_V4::WEA[1],
            bcls::BRAM_V4::WEA[2],
            bcls::BRAM_V4::WEA[3],
            bcls::BRAM_V4::WEB[0],
            bcls::BRAM_V4::WEB[1],
            bcls::BRAM_V4::WEB[2],
            bcls::BRAM_V4::WEB[3],
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .sub_unused(1)
                .test_bel_input_inv_auto(pin);
        }
        for attr in [
            bcls::BRAM_V4::INVERT_CLK_DOA_REG,
            bcls::BRAM_V4::INVERT_CLK_DOB_REG,
            bcls::BRAM_V4::EN_ECC_READ,
            bcls::BRAM_V4::EN_ECC_WRITE,
            bcls::BRAM_V4::SAVEDATA,
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .sub_unused(1)
                .test_bel_attr_bool_auto(attr, "FALSE", "TRUE");
        }
        for attr in [bcls::BRAM_V4::DOA_REG, bcls::BRAM_V4::DOB_REG] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .sub_unused(1)
                .test_bel_attr_bool_auto(attr, "0", "1");
        }
        for attr in [
            bcls::BRAM_V4::READ_WIDTH_A,
            bcls::BRAM_V4::READ_WIDTH_B,
            bcls::BRAM_V4::WRITE_WIDTH_A,
            bcls::BRAM_V4::WRITE_WIDTH_B,
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .sub_unused(1)
                .attr("INIT_A", "0")
                .attr("INIT_B", "0")
                .attr("SRVAL_A", "0")
                .attr("SRVAL_B", "0")
                .test_bel_attr(attr);
            bctx.mode(mode)
                .null_bits()
                .global_mutex("BRAM", "NOPE")
                .sub_unused(1)
                .attr("INIT_A", "0")
                .attr("INIT_B", "0")
                .attr("SRVAL_A", "0")
                .attr("SRVAL_B", "0")
                .test_bel_attr_special(attr, specials::BRAM_DATA_WIDTH_0)
                .attr(backend.edev.db[bcls::BRAM_V4].attributes.key(attr), "0")
                .commit();
        }
        for (attr, aname) in [
            (bcls::BRAM_V4::RAM_EXTENSION_A_LOWER, "RAM_EXTENSION_A"),
            (bcls::BRAM_V4::RAM_EXTENSION_B_LOWER, "RAM_EXTENSION_B"),
        ] {
            for (val, vname) in [(false, "NONE"), (false, "UPPER"), (true, "LOWER")] {
                bctx.mode(mode)
                    .global_mutex("BRAM", "NOPE")
                    .sub_unused(1)
                    .test_bel_attr_bits_bi(attr, val)
                    .attr(aname, vname)
                    .commit();
            }
        }
        for attr in [bcls::BRAM_V4::WRITE_MODE_A, bcls::BRAM_V4::WRITE_MODE_B] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .sub_unused(1)
                .test_bel_attr(attr);
        }
        for attr in [
            bcls::BRAM_V4::INIT_A,
            bcls::BRAM_V4::INIT_B,
            bcls::BRAM_V4::SRVAL_A,
            bcls::BRAM_V4::SRVAL_B,
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .attr("READ_WIDTH_A", "36")
                .attr("READ_WIDTH_B", "36")
                .test_bel_attr_multi(attr, MultiValue::Hex(0));
        }
        for i in 0..0x40 {
            let attr = format!("INIT_{i:02X}");
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .test_bel_attr_bits_base(bcls::BRAM_V4::DATA, i * 0x100)
                .multi_attr(attr, MultiValue::Hex(0), 0x100);
        }
        for i in 0..0x8 {
            let attr = format!("INITP_{i:02X}");
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .test_bel_attr_bits_base(bcls::BRAM_V4::DATAP, i * 0x100)
                .multi_attr(attr, MultiValue::Hex(0), 0x100);
        }
        for (val, vname) in [
            (enums::BRAM_WW_VALUE::_0, "0"),
            (enums::BRAM_WW_VALUE::_1, "1"),
        ] {
            bctx.mode(mode)
                .global_mutex_here("BRAM")
                .test_bel_attr_val(bcls::BRAM_V4::WW_VALUE_A, val)
                .global("Ibram_ww_value", vname)
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(bslots::BRAM).sub(1);
        let mode = "FIFO16";
        bctx.build()
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .test_bel_special(specials::BRAM_FIFO)
            .mode(mode)
            .commit();
        for (pin, pname) in [
            (bcls::BRAM_V4::CLKA, "RDCLK"),
            (bcls::BRAM_V4::CLKB, "WRCLK"),
            (bcls::BRAM_V4::ENA, "RDEN"),
            (bcls::BRAM_V4::ENB, "WREN"),
            (bcls::BRAM_V4::SSRA, "RST"),
        ] {
            for val in [false, true] {
                bctx.mode(mode)
                    .global_mutex("BRAM", "NOPE")
                    .sub_unused(0)
                    .attr("DATA_WIDTH", "36")
                    .pin(pname)
                    .test_bel_input_inv(pin, val)
                    .attr(
                        format!("{pname}INV"),
                        if val {
                            format!("{pname}_B")
                        } else {
                            pname.to_string()
                        },
                    )
                    .commit();
            }
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .test_bel_attr_rename("DATA_WIDTH", bcls::BRAM_V4::FIFO_WIDTH);
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .attr("DATA_WIDTH", "36")
            .test_bel_attr_bool_auto(bcls::BRAM_V4::FIRST_WORD_FALL_THROUGH, "FALSE", "TRUE");
        for attr in [bcls::BRAM_V4::EN_ECC_READ, bcls::BRAM_V4::EN_ECC_WRITE] {
            for (val, spec) in [
                (false, specials::FIFO_EN_ECC_FALSE),
                (true, specials::FIFO_EN_ECC_TRUE),
            ] {
                bctx.mode(mode)
                    .null_bits()
                    .global_mutex("BRAM", "NOPE")
                    .sub_unused(0)
                    .attr("DATA_WIDTH", "36")
                    .test_bel_attr_special(attr, spec)
                    .attr(
                        backend.edev.db[bcls::BRAM_V4].attributes.key(attr),
                        if val { "TRUE" } else { "FALSE" },
                    )
                    .commit();
            }
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .attr("FIRST_WORD_FALL_THROUGH", "FALSE")
            .test_bel_attr_bits(bcls::BRAM_V4::ALMOST_FULL_OFFSET)
            .multi_attr("ALMOST_FULL_OFFSET", MultiValue::Hex(0), 12);
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .attr("FIRST_WORD_FALL_THROUGH", "FALSE")
            .test_bel_attr_bits(bcls::BRAM_V4::ALMOST_EMPTY_OFFSET)
            .multi_attr("ALMOST_EMPTY_OFFSET", MultiValue::Hex(1), 12);

        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .attr("FIRST_WORD_FALL_THROUGH", "TRUE")
            .test_bel_attr_bits(bcls::BRAM_V4::ALMOST_FULL_OFFSET)
            .multi_attr("ALMOST_FULL_OFFSET", MultiValue::Hex(0), 12);
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .sub_unused(0)
            .attr("FIRST_WORD_FALL_THROUGH", "TRUE")
            .test_bel_attr_bits(bcls::BRAM_V4::ALMOST_EMPTY_OFFSET)
            .multi_attr("ALMOST_EMPTY_OFFSET", MultiValue::Hex(2), 12);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::BRAM;
    let bslot = bslots::BRAM;
    for pin in [
        bcls::BRAM_V4::CLKA,
        bcls::BRAM_V4::CLKB,
        bcls::BRAM_V4::ENA,
        bcls::BRAM_V4::ENB,
        bcls::BRAM_V4::SSRA,
        bcls::BRAM_V4::SSRB,
        bcls::BRAM_V4::REGCEA,
        bcls::BRAM_V4::REGCEB,
    ] {
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 4], tcid, bslot, pin);
    }
    for pin in [
        bcls::BRAM_V4::WEA[0],
        bcls::BRAM_V4::WEA[1],
        bcls::BRAM_V4::WEA[2],
        bcls::BRAM_V4::WEA[3],
        bcls::BRAM_V4::WEB[0],
        bcls::BRAM_V4::WEB[1],
        bcls::BRAM_V4::WEB[2],
        bcls::BRAM_V4::WEB[3],
    ] {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::INVERT_CLK_DOA_REG);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::INVERT_CLK_DOB_REG);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::DOA_REG);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::DOB_REG);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::READ_WIDTH_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::READ_WIDTH_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::WRITE_WIDTH_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::WRITE_WIDTH_B);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::RAM_EXTENSION_A_LOWER);
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::RAM_EXTENSION_B_LOWER);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::WRITE_MODE_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::WRITE_MODE_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::INIT_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::INIT_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::SRVAL_A);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::SRVAL_B);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::DATA);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::DATAP);

    for attr in [
        bcls::BRAM_V4::EN_ECC_READ,
        bcls::BRAM_V4::EN_ECC_WRITE,
        bcls::BRAM_V4::SAVEDATA,
    ] {
        ctx.get_diff_attr_bool_bi(tcid, bslot, attr, false)
            .assert_empty();
        let diff = ctx.get_diff_attr_bool_bi(tcid, bslot, attr, true);
        ctx.insert_bel_attr_bitvec(tcid, bslot, attr, xlat_bit_wide(diff));
    }

    ctx.collect_bel_attr_bi(tcid, bslot, bcls::BRAM_V4::FIRST_WORD_FALL_THROUGH);
    let mut diffs = vec![];
    let item_ra = ctx
        .bel_attr_enum(tcid, bslot, bcls::BRAM_V4::READ_WIDTH_A)
        .clone();
    let item_wb = ctx
        .bel_attr_enum(tcid, bslot, bcls::BRAM_V4::WRITE_WIDTH_B)
        .clone();
    for (val_fifo, val_port) in [
        (enums::BRAM_V4_FIFO_WIDTH::_4, enums::BRAM_V4_DATA_WIDTH::_4),
        (enums::BRAM_V4_FIFO_WIDTH::_9, enums::BRAM_V4_DATA_WIDTH::_9),
        (
            enums::BRAM_V4_FIFO_WIDTH::_18,
            enums::BRAM_V4_DATA_WIDTH::_18,
        ),
        (
            enums::BRAM_V4_FIFO_WIDTH::_36,
            enums::BRAM_V4_DATA_WIDTH::_36,
        ),
    ] {
        let mut diff = ctx.get_diff_attr_val(tcid, bslot, bcls::BRAM_V4::FIFO_WIDTH, val_fifo);
        diff.apply_enum_diff(&item_ra, val_port, enums::BRAM_V4_DATA_WIDTH::_1);
        diff.apply_enum_diff(&item_wb, val_port, enums::BRAM_V4_DATA_WIDTH::_1);
        diffs.push((val_fifo, diff));
    }
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        bcls::BRAM_V4::FIFO_WIDTH,
        xlat_enum_attr(diffs),
    );

    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::ALMOST_EMPTY_OFFSET);
    ctx.collect_bel_attr(tcid, bslot, bcls::BRAM_V4::ALMOST_FULL_OFFSET);

    let mut present_bram = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
    let mut present_fifo = ctx.get_diff_bel_special(tcid, bslot, specials::BRAM_FIFO);
    for attr in [
        bcls::BRAM_V4::INIT_A,
        bcls::BRAM_V4::INIT_B,
        bcls::BRAM_V4::SRVAL_A,
        bcls::BRAM_V4::SRVAL_B,
    ] {
        present_bram.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, attr));
        present_fifo.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, attr));
    }
    present_bram.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, bcls::BRAM_V4::WRITE_MODE_A),
        enums::BRAM_WRITE_MODE::WRITE_FIRST,
        enums::BRAM_WRITE_MODE::READ_FIRST,
    );
    present_bram.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, bcls::BRAM_V4::WRITE_MODE_B),
        enums::BRAM_WRITE_MODE::WRITE_FIRST,
        enums::BRAM_WRITE_MODE::READ_FIRST,
    );
    present_fifo.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, bcls::BRAM_V4::WRITE_MODE_A),
        enums::BRAM_WRITE_MODE::WRITE_FIRST,
        enums::BRAM_WRITE_MODE::READ_FIRST,
    );
    present_fifo.apply_enum_diff(
        ctx.bel_attr_enum(tcid, bslot, bcls::BRAM_V4::WRITE_MODE_B),
        enums::BRAM_WRITE_MODE::WRITE_FIRST,
        enums::BRAM_WRITE_MODE::READ_FIRST,
    );
    present_fifo.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::BRAM_V4::DOA_REG),
        true,
        false,
    );
    present_fifo.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::BRAM_V4::DOB_REG),
        true,
        false,
    );
    present_bram.assert_empty();
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        bcls::BRAM_V4::FIFO_ENABLE,
        xlat_bit(present_fifo),
    );

    let mut diffs_a = vec![(enums::BRAM_WW_VALUE::NONE, Diff::default())];
    let mut diffs_b = vec![(enums::BRAM_WW_VALUE::NONE, Diff::default())];
    for val in [enums::BRAM_WW_VALUE::_0, enums::BRAM_WW_VALUE::_1] {
        let mut diff = ctx.get_diff_attr_val(tcid, bslot, bcls::BRAM_V4::WW_VALUE_A, val);
        let diff_b = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 29 | 30));
        diffs_a.push((val, diff));
        diffs_b.push((val, diff_b));
    }

    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        bcls::BRAM_V4::WW_VALUE_A,
        xlat_enum_attr(diffs_a),
    );
    ctx.insert_bel_attr_enum(
        tcid,
        bslot,
        bcls::BRAM_V4::WW_VALUE_B,
        xlat_enum_attr(diffs_b),
    );
}
