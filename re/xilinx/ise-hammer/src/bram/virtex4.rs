use prjcombine_interconnect::db::BelId;
use prjcombine_re_collector::{Diff, xlat_bitvec, xlat_enum};
use prjcombine_re_hammer::Session;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_inv,
    fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let bel_bram = BelId::from_idx(0);
    let bel_fifo = BelId::from_idx(1);
    {
        // RAMB16
        let ctx = FuzzCtx::new(session, backend, "BRAM", "BRAM", TileBits::Bram);
        fuzz_one!(ctx, "PRESENT", "1", [
            (global_mutex_none "BRAM"),
            (bel_unused bel_fifo)
        ], [
            (mode "RAMB16")
        ]);
        for pin in [
            "CLKA", "CLKB", "ENA", "ENB", "SSRA", "SSRB", "REGCEA", "REGCEB", "WEA0", "WEA1",
            "WEA2", "WEA3", "WEB0", "WEB1", "WEB2", "WEB3",
        ] {
            fuzz_inv!(ctx, pin, [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (bel_unused bel_fifo)
            ]);
        }
        for attr in [
            "INVERT_CLK_DOA_REG",
            "INVERT_CLK_DOB_REG",
            "EN_ECC_READ",
            "EN_ECC_WRITE",
            "SAVEDATA",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (bel_unused bel_fifo)
            ])
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (bel_unused bel_fifo)
            ])
        }
        for attr in [
            "READ_WIDTH_A",
            "READ_WIDTH_B",
            "WRITE_WIDTH_A",
            "WRITE_WIDTH_B",
        ] {
            fuzz_enum!(ctx, attr, ["0", "1", "2", "4", "9", "18", "36"], [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (bel_unused bel_fifo),
                (attr "INIT_A", "0"),
                (attr "INIT_B", "0"),
                (attr "SRVAL_A", "0"),
                (attr "SRVAL_B", "0")
            ]);
        }
        for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
            fuzz_enum!(ctx, attr, ["NONE", "LOWER", "UPPER"], [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (bel_unused bel_fifo)
            ]);
        }
        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            fuzz_enum!(ctx, attr, ["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"], [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (bel_unused bel_fifo)
            ]);
        }
        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            fuzz_multi!(ctx, attr, "", 36, [
                (global_mutex_none "BRAM"),
                (mode "RAMB16"),
                (attr "READ_WIDTH_A", "36"),
                (attr "READ_WIDTH_B", "36")
            ], (attr_hex attr));
        }
        for i in 0..0x40 {
            let attr = format!("INIT_{i:02X}");
            fuzz_multi!(ctx, attr, "", 256, [
                (global_mutex_none "BRAM"),
                (mode "RAMB16")
            ], (attr_hex attr));
        }
        for i in 0..0x8 {
            let attr = format!("INITP_{i:02X}");
            fuzz_multi!(ctx, attr, "", 256, [
                (global_mutex_none "BRAM"),
                (mode "RAMB16")
            ], (attr_hex attr));
        }
        for val in ["0", "1"] {
            fuzz_one!(ctx, "Ibram_ww_value", val, [
                (global_mutex_site "BRAM"),
                (mode "RAMB16")
            ], [(global_opt "Ibram_ww_value", val)]);
        }
    }
    {
        // FIFO16
        let ctx = FuzzCtx::new(session, backend, "BRAM", "FIFO", TileBits::Bram);
        fuzz_one!(ctx, "PRESENT", "1", [
            (global_mutex_none "BRAM"),
            (bel_unused bel_bram)
        ], [
            (mode "FIFO16")
        ]);
        for pin in ["RDCLK", "WRCLK", "RDEN", "WREN", "RST"] {
            fuzz_inv!(ctx, pin, [
                (global_mutex_none "BRAM"),
                (mode "FIFO16"),
                (bel_unused bel_bram),
                (attr "DATA_WIDTH", "36")
            ]);
        }
        fuzz_enum!(ctx, "DATA_WIDTH", ["4", "9", "18", "36"], [
            (global_mutex_none "BRAM"),
            (mode "FIFO16"),
            (bel_unused bel_bram)
        ]);
        for attr in ["FIRST_WORD_FALL_THROUGH", "EN_ECC_READ", "EN_ECC_WRITE"] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (global_mutex_none "BRAM"),
                (mode "FIFO16"),
                (bel_unused bel_bram),
                (attr "DATA_WIDTH", "36")
            ]);
        }
        fuzz_multi!(ctx, "ALMOST_FULL_OFFSET:NFWFT", "", 12, [
            (global_mutex_none "BRAM"),
            (mode "FIFO16"),
            (bel_unused bel_bram),
            (attr "FIRST_WORD_FALL_THROUGH", "FALSE")
        ], (attr_hex "ALMOST_FULL_OFFSET"));
        fuzz_multi!(ctx, "ALMOST_EMPTY_OFFSET:NFWFT", "", 12, [
            (global_mutex_none "BRAM"),
            (mode "FIFO16"),
            (bel_unused bel_bram),
            (attr "FIRST_WORD_FALL_THROUGH", "FALSE")
        ], (attr_hex_delta "ALMOST_EMPTY_OFFSET", 1));

        fuzz_multi!(ctx, "ALMOST_FULL_OFFSET:FWFT", "", 12, [
            (global_mutex_none "BRAM"),
            (mode "FIFO16"),
            (bel_unused bel_bram),
            (attr "FIRST_WORD_FALL_THROUGH", "TRUE")
        ], (attr_hex "ALMOST_FULL_OFFSET"));
        fuzz_multi!(ctx, "ALMOST_EMPTY_OFFSET:FWFT", "", 12, [
            (global_mutex_none "BRAM"),
            (mode "FIFO16"),
            (bel_unused bel_bram),
            (attr "FIRST_WORD_FALL_THROUGH", "TRUE")
        ], (attr_hex_delta "ALMOST_EMPTY_OFFSET", 2));
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
        ctx.state.get_diff(tile, "BRAM", attr, "0").assert_empty();
        ctx.collect_enum(tile, "BRAM", attr, &["1", "2", "4", "9", "18", "36"]);
    }
    for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
        let d_none = ctx.state.get_diff(tile, "BRAM", attr, "NONE");
        assert_eq!(d_none, ctx.state.get_diff(tile, "BRAM", attr, "UPPER"));
        let d_lower = ctx.state.get_diff(tile, "BRAM", attr, "LOWER");
        ctx.tiledb.insert(
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
        diffs_data.extend(
            ctx.state
                .get_diffs(tile, "BRAM", format!("INIT_{i:02X}"), ""),
        );
    }
    for i in 0..0x08 {
        diffs_datap.extend(
            ctx.state
                .get_diffs(tile, "BRAM", format!("INITP_{i:02X}"), ""),
        );
    }
    ctx.tiledb
        .insert(tile, "BRAM", "DATA", xlat_bitvec(diffs_data));
    ctx.tiledb
        .insert(tile, "BRAM", "DATAP", xlat_bitvec(diffs_datap));

    for attr in ["EN_ECC_READ", "EN_ECC_WRITE", "SAVEDATA"] {
        ctx.state
            .get_diff(tile, "BRAM", attr, "FALSE")
            .assert_empty();
        let diff = ctx.state.get_diff(tile, "BRAM", attr, "TRUE");
        let mut bits: Vec<_> = diff.bits.into_iter().collect();
        bits.sort();
        ctx.tiledb.insert(
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
    ctx.tiledb
        .insert(tile, "BRAM", "FIRST_WORD_FALL_THROUGH", ti);
    let mut diffs = vec![];
    let item_ra = ctx.collector.tiledb.item(tile, "BRAM", "READ_WIDTH_A");
    let item_wb = ctx.collector.tiledb.item(tile, "BRAM", "WRITE_WIDTH_B");
    for val in ["4", "9", "18", "36"] {
        let mut diff = ctx
            .collector
            .state
            .get_diff(tile, "FIFO", "DATA_WIDTH", val);
        diff.apply_enum_diff(item_ra, val, "1");
        diff.apply_enum_diff(item_wb, val, "1");
        diffs.push((val, diff));
    }
    ctx.tiledb
        .insert(tile, "BRAM", "FIFO_WIDTH", xlat_enum(diffs));
    ctx.state
        .get_diff(tile, "FIFO", "EN_ECC_READ", "FALSE")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FIFO", "EN_ECC_READ", "TRUE")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FIFO", "EN_ECC_WRITE", "FALSE")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FIFO", "EN_ECC_WRITE", "TRUE")
        .assert_empty();

    let diffs = ctx
        .state
        .get_diffs(tile, "FIFO", "ALMOST_FULL_OFFSET:NFWFT", "");
    assert_eq!(
        diffs,
        ctx.state
            .get_diffs(tile, "FIFO", "ALMOST_FULL_OFFSET:FWFT", "")
    );
    ctx.tiledb
        .insert(tile, "BRAM", "ALMOST_FULL_OFFSET", xlat_bitvec(diffs));
    let diffs = ctx
        .state
        .get_diffs(tile, "FIFO", "ALMOST_EMPTY_OFFSET:NFWFT", "");
    assert_eq!(
        diffs,
        ctx.state
            .get_diffs(tile, "FIFO", "ALMOST_EMPTY_OFFSET:FWFT", "")
    );
    ctx.tiledb
        .insert(tile, "BRAM", "ALMOST_EMPTY_OFFSET", xlat_bitvec(diffs));

    let mut present_bram = ctx.state.get_diff(tile, "BRAM", "PRESENT", "1");
    let mut present_fifo = ctx.state.get_diff(tile, "FIFO", "PRESENT", "1");
    for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
        present_bram.discard_bits(ctx.tiledb.item(tile, "BRAM", attr));
        present_fifo.discard_bits(ctx.tiledb.item(tile, "BRAM", attr));
    }
    present_bram.apply_enum_diff(
        ctx.tiledb.item(tile, "BRAM", "WRITE_MODE_A"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_bram.apply_enum_diff(
        ctx.tiledb.item(tile, "BRAM", "WRITE_MODE_B"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_fifo.apply_enum_diff(
        ctx.tiledb.item(tile, "BRAM", "WRITE_MODE_A"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_fifo.apply_enum_diff(
        ctx.tiledb.item(tile, "BRAM", "WRITE_MODE_B"),
        "WRITE_FIRST",
        "READ_FIRST",
    );
    present_fifo.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "DOA_REG"), "1", "0");
    present_fifo.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "DOB_REG"), "1", "0");
    ctx.tiledb.insert(
        tile,
        "BRAM",
        "MODE",
        xlat_enum(vec![("RAM", present_bram), ("FIFO", present_fifo)]),
    );

    let item = xlat_enum(vec![
        ("NONE", Diff::default()),
        ("0", ctx.state.get_diff(tile, "BRAM", "Ibram_ww_value", "0")),
        ("1", ctx.state.get_diff(tile, "BRAM", "Ibram_ww_value", "1")),
    ]);

    ctx.tiledb.insert(tile, "BRAM", "WW_VALUE", item);
}
