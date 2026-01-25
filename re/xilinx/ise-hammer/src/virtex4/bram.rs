use prjcombine_re_collector::diff::{Diff, xlat_bitvec, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "BRAM");
    {
        let mut bctx = ctx.bel(defs::bslots::BRAM);
        let mode = "RAMB16";
        bctx.build()
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::FIFO)
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();
        for pin in [
            "CLKA", "CLKB", "ENA", "ENB", "SSRA", "SSRB", "REGCEA", "REGCEB", "WEA0", "WEA1",
            "WEA2", "WEA3", "WEB0", "WEB1", "WEB2", "WEB3",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::FIFO)
                .test_inv(pin);
        }
        for attr in [
            "INVERT_CLK_DOA_REG",
            "INVERT_CLK_DOB_REG",
            "EN_ECC_READ",
            "EN_ECC_WRITE",
            "SAVEDATA",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::FIFO)
                .test_enum(attr, &["FALSE", "TRUE"]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::FIFO)
                .test_enum(attr, &["0", "1"]);
        }
        for attr in [
            "READ_WIDTH_A",
            "READ_WIDTH_B",
            "WRITE_WIDTH_A",
            "WRITE_WIDTH_B",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::FIFO)
                .attr("INIT_A", "0")
                .attr("INIT_B", "0")
                .attr("SRVAL_A", "0")
                .attr("SRVAL_B", "0")
                .test_enum(attr, &["0", "1", "2", "4", "9", "18", "36"]);
        }
        for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::FIFO)
                .test_enum(attr, &["NONE", "LOWER", "UPPER"]);
        }
        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::FIFO)
                .test_enum(attr, &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"]);
        }
        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .attr("READ_WIDTH_A", "36")
                .attr("READ_WIDTH_B", "36")
                .test_multi_attr_hex(attr, 36);
        }
        for i in 0..0x40 {
            let attr = format!("INIT_{i:02X}");
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .test_multi_attr_hex(attr, 256);
        }
        for i in 0..0x8 {
            let attr = format!("INITP_{i:02X}");
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .test_multi_attr_hex(attr, 256);
        }
        for val in ["0", "1"] {
            bctx.mode(mode)
                .global_mutex_here("BRAM")
                .test_manual("Ibram_ww_value", val)
                .global("Ibram_ww_value", val)
                .commit();
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::FIFO);
        let mode = "FIFO16";
        bctx.build()
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::BRAM)
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();
        for pin in ["RDCLK", "WRCLK", "RDEN", "WREN", "RST"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::BRAM)
                .attr("DATA_WIDTH", "36")
                .test_inv(pin);
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::BRAM)
            .test_enum("DATA_WIDTH", &["4", "9", "18", "36"]);
        for attr in ["FIRST_WORD_FALL_THROUGH", "EN_ECC_READ", "EN_ECC_WRITE"] {
            bctx.mode(mode)
                .global_mutex("BRAM", "NOPE")
                .bel_unused(defs::bslots::BRAM)
                .attr("DATA_WIDTH", "36")
                .test_enum(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::BRAM)
            .attr("FIRST_WORD_FALL_THROUGH", "FALSE")
            .test_manual("ALMOST_FULL_OFFSET:NFWFT", "")
            .multi_attr("ALMOST_FULL_OFFSET", MultiValue::Hex(0), 12);
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::BRAM)
            .attr("FIRST_WORD_FALL_THROUGH", "FALSE")
            .test_manual("ALMOST_EMPTY_OFFSET:NFWFT", "")
            .multi_attr("ALMOST_EMPTY_OFFSET", MultiValue::Hex(1), 12);

        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::BRAM)
            .attr("FIRST_WORD_FALL_THROUGH", "TRUE")
            .test_manual("ALMOST_FULL_OFFSET:FWFT", "")
            .multi_attr("ALMOST_FULL_OFFSET", MultiValue::Hex(0), 12);
        bctx.mode(mode)
            .global_mutex("BRAM", "NOPE")
            .bel_unused(defs::bslots::BRAM)
            .attr("FIRST_WORD_FALL_THROUGH", "TRUE")
            .test_manual("ALMOST_EMPTY_OFFSET:FWFT", "")
            .multi_attr("ALMOST_EMPTY_OFFSET", MultiValue::Hex(2), 12);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "BRAM";
    for pin in ["RDCLK", "WRCLK", "RDEN", "WREN", "RST"] {
        ctx.collect_int_inv(&["INT"; 4], tile, "FIFO", pin, false);
    }
    for pin in [
        "CLKA", "CLKB", "ENA", "ENB", "SSRA", "SSRB", "REGCEA", "REGCEB",
    ] {
        ctx.collect_int_inv(&["INT"; 4], tile, "BRAM", pin, false);
    }
    for pin in [
        "WEA0", "WEA1", "WEA2", "WEA3", "WEB0", "WEB1", "WEB2", "WEB3",
    ] {
        ctx.collect_inv(tile, "BRAM", pin);
    }
    ctx.collect_enum_bool(tile, "BRAM", "INVERT_CLK_DOA_REG", "FALSE", "TRUE");
    ctx.collect_enum_bool(tile, "BRAM", "INVERT_CLK_DOB_REG", "FALSE", "TRUE");
    ctx.collect_enum(tile, "BRAM", "DOA_REG", &["0", "1"]);
    ctx.collect_enum(tile, "BRAM", "DOB_REG", &["0", "1"]);
    for attr in [
        "READ_WIDTH_A",
        "READ_WIDTH_B",
        "WRITE_WIDTH_A",
        "WRITE_WIDTH_B",
    ] {
        ctx.get_diff(tile, "BRAM", attr, "0").assert_empty();
        ctx.collect_enum(tile, "BRAM", attr, &["1", "2", "4", "9", "18", "36"]);
    }
    for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
        let d_none = ctx.get_diff(tile, "BRAM", attr, "NONE");
        assert_eq!(d_none, ctx.get_diff(tile, "BRAM", attr, "UPPER"));
        let d_lower = ctx.get_diff(tile, "BRAM", attr, "LOWER");
        ctx.insert(
            tile,
            "BRAM",
            attr,
            xlat_enum(vec![("NONE_UPPER", d_none), ("LOWER", d_lower)]),
        );
    }
    ctx.collect_enum(
        tile,
        "BRAM",
        "WRITE_MODE_A",
        &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"],
    );
    ctx.collect_enum(
        tile,
        "BRAM",
        "WRITE_MODE_B",
        &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"],
    );
    ctx.collect_bitvec(tile, "BRAM", "INIT_A", "");
    ctx.collect_bitvec(tile, "BRAM", "INIT_B", "");
    ctx.collect_bitvec(tile, "BRAM", "SRVAL_A", "");
    ctx.collect_bitvec(tile, "BRAM", "SRVAL_B", "");
    let mut diffs_data = vec![];
    let mut diffs_datap = vec![];
    for i in 0..0x40 {
        diffs_data.extend(ctx.get_diffs(tile, "BRAM", format!("INIT_{i:02X}"), ""));
    }
    for i in 0..0x08 {
        diffs_datap.extend(ctx.get_diffs(tile, "BRAM", format!("INITP_{i:02X}"), ""));
    }
    ctx.insert(tile, "BRAM", "DATA", xlat_bitvec(diffs_data));
    ctx.insert(tile, "BRAM", "DATAP", xlat_bitvec(diffs_datap));

    for attr in ["EN_ECC_READ", "EN_ECC_WRITE", "SAVEDATA"] {
        ctx.get_diff(tile, "BRAM", attr, "FALSE").assert_empty();
        let diff = ctx.get_diff(tile, "BRAM", attr, "TRUE");
        let mut bits: Vec<_> = diff.bits.into_iter().collect();
        bits.sort();
        ctx.insert(
            tile,
            "BRAM",
            attr,
            xlat_bitvec(
                bits.into_iter()
                    .map(|(k, v)| Diff {
                        bits: [(k, v)].into_iter().collect(),
                    })
                    .collect(),
            ),
        );
    }

    let ti = ctx.extract_enum_bool(tile, "FIFO", "FIRST_WORD_FALL_THROUGH", "FALSE", "TRUE");
    ctx.insert(tile, "BRAM", "FIRST_WORD_FALL_THROUGH", ti);
    let mut diffs = vec![];
    let item_ra = ctx.item(tile, "BRAM", "READ_WIDTH_A").clone();
    let item_wb = ctx.item(tile, "BRAM", "WRITE_WIDTH_B").clone();
    for val in ["4", "9", "18", "36"] {
        let mut diff = ctx.get_diff(tile, "FIFO", "DATA_WIDTH", val);
        diff.apply_enum_diff(&item_ra, val, "1");
        diff.apply_enum_diff(&item_wb, val, "1");
        diffs.push((val, diff));
    }
    ctx.insert(tile, "BRAM", "FIFO_WIDTH", xlat_enum(diffs));
    ctx.get_diff(tile, "FIFO", "EN_ECC_READ", "FALSE")
        .assert_empty();
    ctx.get_diff(tile, "FIFO", "EN_ECC_READ", "TRUE")
        .assert_empty();
    ctx.get_diff(tile, "FIFO", "EN_ECC_WRITE", "FALSE")
        .assert_empty();
    ctx.get_diff(tile, "FIFO", "EN_ECC_WRITE", "TRUE")
        .assert_empty();

    let diffs = ctx.get_diffs(tile, "FIFO", "ALMOST_FULL_OFFSET:NFWFT", "");
    assert_eq!(
        diffs,
        ctx.get_diffs(tile, "FIFO", "ALMOST_FULL_OFFSET:FWFT", "")
    );
    ctx.insert(tile, "BRAM", "ALMOST_FULL_OFFSET", xlat_bitvec(diffs));
    let diffs = ctx.get_diffs(tile, "FIFO", "ALMOST_EMPTY_OFFSET:NFWFT", "");
    assert_eq!(
        diffs,
        ctx.get_diffs(tile, "FIFO", "ALMOST_EMPTY_OFFSET:FWFT", "")
    );
    ctx.insert(tile, "BRAM", "ALMOST_EMPTY_OFFSET", xlat_bitvec(diffs));

    let mut present_bram = ctx.get_diff(tile, "BRAM", "PRESENT", "1");
    let mut present_fifo = ctx.get_diff(tile, "FIFO", "PRESENT", "1");
    for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
        present_bram.discard_bits(ctx.item(tile, "BRAM", attr));
        present_fifo.discard_bits(ctx.item(tile, "BRAM", attr));
    }
    present_bram.apply_enum_diff(
        ctx.item(tile, "BRAM", "WRITE_MODE_A"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_bram.apply_enum_diff(
        ctx.item(tile, "BRAM", "WRITE_MODE_B"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_fifo.apply_enum_diff(
        ctx.item(tile, "BRAM", "WRITE_MODE_A"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_fifo.apply_enum_diff(
        ctx.item(tile, "BRAM", "WRITE_MODE_B"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_fifo.apply_enum_diff(ctx.item(tile, "BRAM", "DOA_REG"), "1", "0");
    present_fifo.apply_enum_diff(ctx.item(tile, "BRAM", "DOB_REG"), "1", "0");
    ctx.insert(
        tile,
        "BRAM",
        "MODE",
        xlat_enum(vec![("RAM", present_bram), ("FIFO", present_fifo)]),
    );

    let item = xlat_enum(vec![
        ("NONE", Diff::default()),
        ("0", ctx.get_diff(tile, "BRAM", "Ibram_ww_value", "0")),
        ("1", ctx.get_diff(tile, "BRAM", "Ibram_ww_value", "1")),
    ]);

    ctx.insert(tile, "BRAM", "WW_VALUE", item);
}
