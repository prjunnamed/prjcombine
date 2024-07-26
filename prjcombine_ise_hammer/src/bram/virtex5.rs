use bitvec::prelude::*;
use prjcombine_hammer::Session;

use crate::{
    backend::IseBackend,
    diff::{xlat_bitvec, xlat_enum, xlat_enum_int, CollectorCtx, Diff},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv_suffix, fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ctx = FuzzCtx::new(session, backend, "BRAM", "BRAM", TileBits::Bram);

    fuzz_one!(ctx, "PRESENT", "RAMBFIFO36", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMBFIFO36")
    ]);
    fuzz_one!(ctx, "PRESENT", "RAMB18X2", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMB18X2")
    ]);
    fuzz_one!(ctx, "PRESENT", "RAMB18X2SDP", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMB18X2SDP")
    ]);
    fuzz_one!(ctx, "PRESENT", "RAMBFIFO18", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMBFIFO18")
    ]);
    fuzz_one!(ctx, "PRESENT", "RAMBFIFO18_36", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMBFIFO18_36")
    ]);
    fuzz_one!(ctx, "PRESENT", "RAMB36_EXP", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMB36_EXP")
    ]);
    fuzz_one!(ctx, "PRESENT", "RAMB36SDP_EXP", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "RAMB36SDP_EXP")
    ]);
    fuzz_one!(ctx, "PRESENT", "FIFO36_EXP", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "FIFO36_EXP")
    ]);
    fuzz_one!(ctx, "PRESENT", "FIFO36_72_EXP", [
        (global_mutex "BRAM_OPT", "NONE")
    ], [
        (mode "FIFO36_72_EXP")
    ]);

    for opt in [
        "TEST_FIFO_FLAG",
        "TEST_FIFO_OFFSET",
        "TEST_FIFO_CNT",
        "SWAP_CFGPORT",
        "BYPASS_RSR",
    ] {
        fuzz_one!(ctx, "PRESENT", format!("FIFO36_EXP.{opt}"), [
            (global_mutex "BRAM_OPT", opt),
            (global_opt opt, "ENABLED")
        ], [
            (mode "FIFO36_EXP")
        ]);
    }
    for val in ["WW0", "WW1"] {
        fuzz_one!(ctx, "PRESENT", format!("FIFO36_EXP.WEAK_WRITE.{val}"), [
            (global_mutex "BRAM_OPT", "WEAK_WRITE"),
            (global_opt "WEAK_WRITE", val)
        ], [
            (mode "FIFO36_EXP")
        ]);
    }
    for val in ["0", "1", "10", "11", "100", "101", "110", "111"] {
        fuzz_one!(ctx, "PRESENT", format!("FIFO36_EXP.TRD_DLY.{val}"), [
            (global_mutex "BRAM_OPT", "TRD_DLY"),
            (global_opt "TRD_DLY", val)
        ], [
            (mode "FIFO36_EXP")
        ]);
    }
    for val in ["0", "11", "101", "1000"] {
        fuzz_one!(ctx, "PRESENT", format!("FIFO36_EXP.TWR_DLY.{val}"), [
            (global_mutex "BRAM_OPT", "TWR_DLY"),
            (global_opt "TWR_DLY", val)
        ], [
            (mode "FIFO36_EXP")
        ]);
    }
    for val in ["0", "101", "1010", "1111"] {
        fuzz_one!(ctx, "PRESENT", format!("FIFO36_EXP.TSCRUB_DLY.{val}"), [
            (global_mutex "BRAM_OPT", "TSCRUB_DLY"),
            (global_opt "TSCRUB_DLY", val)
        ], [
            (mode "FIFO36_EXP")
        ]);
    }

    for pin in [
        "CLKAL", "CLKAU", "CLKBL", "CLKBU", "REGCLKAL", "REGCLKAU", "REGCLKBL", "REGCLKBU",
        "SSRAL", "SSRAU", "SSRBL", "SSRBU", "ENAL", "ENAU", "ENBL", "ENBU",
    ] {
        fuzz_inv_suffix!(ctx, pin, "RAMB18X2", [
            (mode "RAMB18X2"),
            (attr "DOA_REG_L", "1"),
            (attr "DOA_REG_U", "1")
        ]);
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "SSRL", "SSRU", "RDENL",
        "RDENU", "WRENL", "WRENU",
    ] {
        fuzz_inv_suffix!(ctx, pin, "RAMB18X2SDP", [
            (mode "RAMB18X2SDP"),
            (attr "DO_REG_L", "1"),
            (attr "DO_REG_U", "1")
        ]);
    }
    for pin in [
        "CLKAL", "CLKAU", "CLKBL", "CLKBU", "REGCLKAL", "REGCLKAU", "REGCLKBL", "REGCLKBU",
        "SSRAL", "SSRAU", "SSRBL", "SSRBU", "ENAL", "ENAU", "ENBL", "ENBU",
    ] {
        fuzz_inv_suffix!(ctx, pin, "RAMB36_EXP", [
            (mode "RAMB36_EXP")
        ]);
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "SSRL", "SSRU", "RDENL",
        "RDENU", "WRENL", "WRENU",
    ] {
        fuzz_inv_suffix!(ctx, pin, "RAMB36SDP_EXP", [
            (mode "RAMB36SDP_EXP")
        ]);
    }
    for pin in [
        "RDCLK", "CLKA", "WRCLK", "CLKB", "RDRCLK", "REGCLKA", "REGCLKB", "RST", "SSRA", "SSRB",
        "RDEN", "ENA", "WREN", "ENB",
    ] {
        fuzz_inv_suffix!(ctx, pin, "RAMBFIFO18", [
            (mode "RAMBFIFO18"),
            (attr "DO_REG", "1"),
            (attr "DOA_REG", "1"),
            (attr "EN_SYN", "FALSE")
        ]);
    }
    for pin in [
        "RDCLK", "RDCLKU", "WRCLK", "WRCLKU", "RDRCLK", "RDRCLKU", "RST", "SSRU", "RDEN", "RDENU",
        "WREN", "WRENU",
    ] {
        fuzz_inv_suffix!(ctx, pin, "RAMBFIFO18_36", [
            (mode "RAMBFIFO18_36"),
            (attr "DO_REG_L", "1"),
            (attr "DO_REG_U", "1"),
            (attr "EN_SYN", "FALSE")
        ]);
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "RST", "RDEN", "WREN",
    ] {
        fuzz_inv_suffix!(ctx, pin, "FIFO36_EXP", [
            (mode "FIFO36_EXP")
        ]);
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "RST", "RDEN", "WREN",
    ] {
        fuzz_inv_suffix!(ctx, pin, "FIFO36_72_EXP", [
            (mode "FIFO36_72_EXP")
        ]);
    }

    for mode in ["RAMBFIFO36", "RAMB36SDP_EXP", "FIFO36_72_EXP"] {
        fuzz_enum_suffix!(ctx, "EN_ECC_READ", mode, ["FALSE", "TRUE"], [
            (mode mode),
            (attr "EN_ECC_WRITE", "FALSE")
        ]);
        fuzz_enum_suffix!(ctx, "EN_ECC_WRITE", mode, ["FALSE", "TRUE"], [
            (mode mode),
            (attr "EN_ECC_READ", "FALSE")
        ]);
        fuzz_enum_suffix!(ctx, "EN_ECC_WRITE", format!("{mode}.READ"), ["FALSE", "TRUE"], [
            (mode mode),
            (attr "EN_ECC_READ", "TRUE")
        ]);
        if mode != "FIFO36_72_EXP" {
            fuzz_enum_suffix!(ctx, "EN_ECC_SCRUB", mode, ["FALSE", "TRUE"], [
                (mode mode),
                (global_mutex "BRAM_OPT", "NONE")
            ]);
        }
    }
    for mode in [
        "RAMBFIFO36",
        "RAMBFIFO18",
        "RAMBFIFO18_36",
        "FIFO36_EXP",
        "FIFO36_72_EXP",
    ] {
        fuzz_enum_suffix!(ctx, "EN_SYN", mode, ["FALSE", "TRUE"], [
            (mode mode)
        ]);
        fuzz_enum_suffix!(ctx, "FIRST_WORD_FALL_THROUGH", mode, ["FALSE", "TRUE"], [
            (mode mode)
        ]);
        if mode != "RAMBFIFO36" {
            fuzz_multi!(ctx, format!("ALMOST_FULL_OFFSET.{mode}"), "", 13, [
                (mode mode),
                (attr "EN_SYN", "TRUE")
            ], (attr_hex "ALMOST_FULL_OFFSET"));
            fuzz_multi!(ctx, format!("ALMOST_EMPTY_OFFSET.{mode}"), "", 13, [
                (mode mode),
                (attr "EN_SYN", "TRUE")
            ], (attr_hex "ALMOST_EMPTY_OFFSET"));
        }
    }
    fuzz_enum!(ctx, "IS_FIFO", ["FALSE", "TRUE"], [(mode "RAMBFIFO36")]);

    for (mode, attr, init, srval) in [
        ("RAMBFIFO36", "DOA_REG_L", "INIT_A_L", "SRVAL_A_L"),
        ("RAMBFIFO36", "DOA_REG_U", "INIT_A_U", "SRVAL_A_U"),
        ("RAMBFIFO36", "DOB_REG_L", "INIT_B_L", "SRVAL_B_L"),
        ("RAMBFIFO36", "DOB_REG_U", "INIT_B_U", "SRVAL_B_U"),
        ("RAMB18X2", "DOA_REG_L", "INIT_A_L", "SRVAL_A_L"),
        ("RAMB18X2", "DOA_REG_U", "INIT_A_U", "SRVAL_A_U"),
        ("RAMB18X2", "DOB_REG_L", "INIT_B_L", "SRVAL_B_L"),
        ("RAMB18X2", "DOB_REG_U", "INIT_B_U", "SRVAL_B_U"),
        ("RAMB18X2SDP", "DO_REG_L", "INIT_L", "SRVAL_L"),
        ("RAMB18X2SDP", "DO_REG_U", "INIT_U", "SRVAL_U"),
        ("RAMB36_EXP", "DOA_REG", "INIT_A", "SRVAL_A"),
        ("RAMB36_EXP", "DOB_REG", "INIT_B", "SRVAL_B"),
        ("RAMB36SDP_EXP", "DO_REG", "INIT", "SRVAL"),
        ("RAMBFIFO18", "DOA_REG", "DOB_REG", "SRVAL_A"),
        ("RAMBFIFO18", "DOB_REG", "DOA_REG", "SRVAL_B"),
    ] {
        fuzz_enum_suffix!(ctx, attr, mode, ["0", "1"], [
            (mode mode),
            (attr init, "0"),
            (attr srval, "0")
        ]);
    }
    fuzz_enum_suffix!(ctx, "DO_REG_U", "RAMBFIFO18_36", ["0", "1"], [
        (mode "RAMBFIFO18_36"),
        (attr "INIT", "fffffffff"),
        (attr "SRVAL", "fffffffff")
    ]);

    for (mode, attr) in [
        ("RAMBFIFO18", "DO_REG"),
        ("RAMBFIFO18_36", "DO_REG_L"),
        ("FIFO36_EXP", "DO_REG"),
        ("FIFO36_72_EXP", "DO_REG"),
    ] {
        fuzz_enum_suffix!(ctx, attr, mode, ["0", "1"], [
            (mode mode),
            (attr "EN_SYN", "TRUE")
        ]);
    }

    fuzz_enum_suffix!(ctx, "DATA_WIDTH", "RAMBFIFO18", ["4", "9", "18"], [
        (mode "RAMBFIFO18")
    ]);
    fuzz_enum_suffix!(ctx, "DATA_WIDTH", "FIFO36_EXP", ["4", "9", "18", "36"], [
        (mode "FIFO36_EXP")
    ]);
    for ab in ['A', 'B'] {
        for ul in ['U', 'L'] {
            fuzz_one!(ctx, format!("READ_WIDTH_{ab}_{ul}.RAMBFIFO36"), "36", [
                (mode "RAMBFIFO36")
            ], [
                (attr format!("READ_WIDTH_{ab}_{ul}"), "36"),
                (attr format!("DO{ab}_REG_{ul}"), "0"),
                (attr format!("INIT_{ab}_{ul}"), "0"),
                (attr format!("SRVAL_{ab}_{ul}"), "0")
            ]);
            fuzz_one!(ctx, format!("WRITE_WIDTH_{ab}_{ul}.RAMBFIFO36"), "36", [
                (mode "RAMBFIFO36")
            ], [
                (attr format!("WRITE_WIDTH_{ab}_{ul}"), "36")
            ]);
            fuzz_enum_suffix!(ctx, format!("READ_WIDTH_{ab}_{ul}"), "RAMB18X2", ["0", "1", "2", "4", "9", "18"], [
                (mode "RAMB18X2"),
                (attr format!("INIT_{ab}_{ul}"), "0"),
                (attr format!("SRVAL_{ab}_{ul}"), "0")
            ]);
            fuzz_enum_suffix!(ctx, format!("WRITE_WIDTH_{ab}_{ul}"), "RAMB18X2", ["0", "1", "2", "4", "9", "18"], [
                (mode "RAMB18X2"),
                (pin format!("WE{ab}{ul}0")),
                (pin format!("WE{ab}{ul}1")),
                (pin format!("WE{ab}{ul}2")),
                (pin format!("WE{ab}{ul}3"))
            ]);
        }
        fuzz_enum_suffix!(ctx, format!("READ_WIDTH_{ab}"), "RAMBFIFO18", ["0", "1", "2", "4", "9", "18"], [
            (mode "RAMBFIFO18"),
            (attr format!("DO{ab}_REG"), "0"),
            (attr format!("INIT_{ab}"), "0"),
            (attr format!("SRVAL_{ab}"), "0")
        ]);
        fuzz_enum_suffix!(ctx, format!("WRITE_WIDTH_{ab}"), "RAMBFIFO18", ["0", "1", "2", "4", "9", "18"], [
            (mode "RAMBFIFO18"),
            (attr format!("DO{ab}_REG"), "0"),
            (pin format!("WE{ab}0")),
            (pin format!("WE{ab}1")),
            (pin format!("WE{ab}2")),
            (pin format!("WE{ab}3"))
        ]);
        fuzz_enum_suffix!(ctx, format!("READ_WIDTH_{ab}"), "RAMB36_EXP", ["0", "1", "2", "4", "9", "18", "36"], [
            (mode "RAMB36_EXP"),
            (attr format!("INIT_{ab}"), "0"),
            (attr format!("SRVAL_{ab}"), "0")
        ]);
        fuzz_enum_suffix!(ctx, format!("WRITE_WIDTH_{ab}"), "RAMB36_EXP", ["0", "1", "2", "4", "9", "18", "36"], [
            (mode "RAMB36_EXP")
        ]);
    }

    for (mode, attr) in [
        ("RAMBFIFO36", "WRITE_MODE_A_L"),
        ("RAMBFIFO36", "WRITE_MODE_A_U"),
        ("RAMBFIFO36", "WRITE_MODE_B_L"),
        ("RAMBFIFO36", "WRITE_MODE_B_U"),
        ("RAMB18X2", "WRITE_MODE_A_L"),
        ("RAMB18X2", "WRITE_MODE_A_U"),
        ("RAMB18X2", "WRITE_MODE_B_L"),
        ("RAMB18X2", "WRITE_MODE_B_U"),
        ("RAMBFIFO18", "WRITE_MODE_A"),
        ("RAMBFIFO18", "WRITE_MODE_B"),
        ("RAMB36_EXP", "WRITE_MODE_A"),
        ("RAMB36_EXP", "WRITE_MODE_B"),
    ] {
        fuzz_enum_suffix!(ctx, attr, mode, ["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"], [
            (mode mode)
        ]);
    }
    for (mode, attr) in [
        ("RAMBFIFO36", "RAM_EXTENSION_A"),
        ("RAMBFIFO36", "RAM_EXTENSION_B"),
        ("RAMB36_EXP", "RAM_EXTENSION_A"),
        ("RAMB36_EXP", "RAM_EXTENSION_B"),
    ] {
        fuzz_enum_suffix!(ctx, attr, mode, ["NONE", "UPPER", "LOWER"], [
            (mode mode)
        ]);
    }

    for attr in ["INIT", "SRVAL"] {
        for ab in ['A', 'B'] {
            for ul in ['U', 'L'] {
                fuzz_multi!(ctx, format!("{attr}_{ab}_{ul}.RAMBFIFO36"), "", 18, [
                    (mode "RAMBFIFO36"),
                    (attr "IS_FIFO", "FALSE"),
                    (attr format!("READ_WIDTH_{ab}_{ul}"), "18")
                ], (attr_hex format!("{attr}_{ab}_{ul}")));
                fuzz_multi!(ctx, format!("{attr}_{ab}_{ul}.RAMB18X2"), "", 18, [
                    (mode "RAMB18X2"),
                    (attr format!("READ_WIDTH_{ab}_{ul}"), "18")
                ], (attr_hex format!("{attr}_{ab}_{ul}")));
            }
            fuzz_multi!(ctx, format!("{attr}_{ab}.RAMB36_EXP"), "", 36, [
                (mode "RAMB36_EXP"),
                (attr format!("READ_WIDTH_{ab}"), "36")
            ], (attr_hex format!("{attr}_{ab}")));
        }
        for ul in ['U', 'L'] {
            fuzz_multi!(ctx, format!("{attr}_{ul}.RAMB18X2SDP"), "", 36, [
                (mode "RAMB18X2SDP"),
                (attr format!("DO_REG_{ul}"), "0")
            ], (attr_hex format!("{attr}_{ul}")));
        }
        fuzz_multi!(ctx, format!("{attr}.RAMB36SDP_EXP"), "", 72, [
            (mode "RAMB36SDP_EXP")
        ], (attr_hex attr));
    }

    for mode in ["RAMB18X2", "RAMB18X2SDP"] {
        for ul in ['U', 'L'] {
            for i in 0..0x40 {
                fuzz_multi!(ctx, format!("INIT_{i:02X}_{ul}.{mode}"), "", 256, [
                    (mode mode),
                    (attr format!("READ_WIDTH_A_{ul}"), if mode == "RAMB18X2SDP" {""} else {"18"}),
                    (attr format!("DO_REG_{ul}"), if mode == "RAMB18X2SDP" {"1"} else {""}),
                    (attr "IS_FIFO", if mode == "RAMBFIFO36" {"FALSE"} else {""})
                ], (attr_hex &format!("INIT_{i:02X}_{ul}")));
            }
            for i in 0..8 {
                fuzz_multi!(ctx, format!("INITP_{i:02X}_{ul}.{mode}"), "", 256, [
                    (mode mode),
                    (attr format!("READ_WIDTH_A_{ul}"), if mode == "RAMB18X2SDP" {""} else {"18"}),
                    (attr format!("DO_REG_{ul}"), if mode == "RAMB18X2SDP" {"1"} else {""}),
                    (attr "IS_FIFO", if mode == "RAMBFIFO36" {"FALSE"} else {""})
                ], (attr_hex &format!("INITP_{i:02X}_{ul}")));
            }
        }
    }
    for mode in ["RAMBFIFO18", "RAMBFIFO18_36"] {
        for i in 0..0x40 {
            fuzz_multi!(ctx, format!("INIT_{i:02X}.{mode}"), "", 256, [
                (mode mode),
                (attr "DOA_REG", if mode == "RAMBFIFO18_36" {""} else {"1"}),
                (attr "DO_REG_U", if mode == "RAMBFIFO18_36" {"1"} else {""})
            ], (attr_hex &format!("INIT_{i:02X}")));
        }
        for i in 0..8 {
            fuzz_multi!(ctx, format!("INITP_{i:02X}.{mode}"), "", 256, [
                (mode mode),
                (attr "DOA_REG", if mode == "RAMBFIFO18_36" {""} else {"1"}),
                (attr "DO_REG_U", if mode == "RAMBFIFO18_36" {"1"} else {""})
            ], (attr_hex &format!("INITP_{i:02X}")));
        }
    }
    for mode in ["RAMB36_EXP", "RAMB36SDP_EXP"] {
        for i in 0..0x80 {
            fuzz_multi!(ctx, format!("INIT_{i:02X}.{mode}"), "", 256, [
                (mode mode),
                (attr "READ_WIDTH_A", if mode == "RAMB36SDP_EXP" {""} else {"36"})
            ], (attr_hex &format!("INIT_{i:02X}")));
        }
        for i in 0..0x10 {
            fuzz_multi!(ctx, format!("INITP_{i:02X}.{mode}"), "", 256, [
                (mode mode),
                (attr "READ_WIDTH_A", if mode == "RAMB36SDP_EXP" {""} else {"36"})
            ], (attr_hex &format!("INITP_{i:02X}")));
        }
    }

    fuzz_enum!(ctx, "SAVEDATA", ["FALSE", "TRUE"], [(mode "RAMB36_EXP")]);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "BRAM";
    let bel = "BRAM";

    let mut present_rambfifo36 = ctx.state.get_diff(tile, bel, "PRESENT", "RAMBFIFO36");
    let mut present_ramb18x2 = ctx.state.get_diff(tile, bel, "PRESENT", "RAMB18X2");
    let mut present_ramb18x2sdp = ctx.state.get_diff(tile, bel, "PRESENT", "RAMB18X2SDP");
    let mut present_ramb36 = ctx.state.get_diff(tile, bel, "PRESENT", "RAMB36_EXP");
    let mut present_ramb36sdp = ctx.state.get_diff(tile, bel, "PRESENT", "RAMB36SDP_EXP");
    let mut present_rambfifo18 = ctx.state.get_diff(tile, bel, "PRESENT", "RAMBFIFO18");
    let mut present_rambfifo18_36 = ctx.state.get_diff(tile, bel, "PRESENT", "RAMBFIFO18_36");
    let mut present_fifo36 = ctx.state.get_diff(tile, bel, "PRESENT", "FIFO36_EXP");
    let mut present_fifo36_72 = ctx.state.get_diff(tile, bel, "PRESENT", "FIFO36_72_EXP");

    for opt in [
        "TEST_FIFO_FLAG",
        "TEST_FIFO_OFFSET",
        "TEST_FIFO_CNT",
        "SWAP_CFGPORT",
        "BYPASS_RSR",
    ] {
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "PRESENT", format!("FIFO36_EXP.{opt}"));
        diff = diff.combine(&!&present_fifo36);
        ctx.tiledb.insert(tile, bel, opt, xlat_bitvec(vec![diff]));
    }
    let mut diffs = vec![("NONE", Diff::default())];
    for val in ["WW0", "WW1"] {
        let mut diff =
            ctx.state
                .get_diff(tile, bel, "PRESENT", format!("FIFO36_EXP.WEAK_WRITE.{val}"));
        diff = diff.combine(&!&present_fifo36);
        diffs.push((val, diff));
    }
    ctx.tiledb.insert(tile, bel, "WEAK_WRITE", xlat_enum(diffs));

    fn split_diff_ul(diff: Diff) -> (Diff, Diff) {
        let mut diff_l = Diff::default();
        let mut diff_u = Diff::default();
        for (k, v) in diff.bits {
            if k.tile < 2 {
                diff_l.bits.insert(k, v);
            } else {
                diff_u.bits.insert(k, v);
            }
        }
        (diff_l, diff_u)
    }

    let mut diffs_l = vec![];
    let mut diffs_u = vec![];
    for (i, val) in ["0", "1", "10", "11", "100", "101", "110", "111"]
        .into_iter()
        .enumerate()
    {
        let mut diff =
            ctx.state
                .get_diff(tile, bel, "PRESENT", format!("FIFO36_EXP.TRD_DLY.{val}"));
        diff = diff.combine(&!&present_fifo36);
        if i == 0 {
            diff.assert_empty();
        }
        let i: u32 = i.try_into().unwrap();
        let (diff_l, diff_u) = split_diff_ul(diff);
        diffs_l.push((i, diff_l));
        diffs_u.push((i, diff_u));
    }
    ctx.tiledb
        .insert(tile, bel, "TRD_DLY_L", xlat_enum_int(diffs_l));
    ctx.tiledb
        .insert(tile, bel, "TRD_DLY_U", xlat_enum_int(diffs_u));

    let diff_3 = ctx
        .state
        .peek_diff(tile, bel, "PRESENT", "FIFO36_EXP.TWR_DLY.11")
        .clone();
    let diff_5 = ctx
        .state
        .peek_diff(tile, bel, "PRESENT", "FIFO36_EXP.TWR_DLY.101")
        .clone();
    let (_, _, mut diff_1) = Diff::split(diff_3, diff_5);
    diff_1 = diff_1.combine(&!&present_fifo36);
    let (diff_l, diff_u) = split_diff_ul(diff_1);
    let mut diffs_l = vec![(1, diff_l)];
    let mut diffs_u = vec![(1, diff_u)];
    for (i, val) in [(0, "0"), (3, "11"), (5, "101"), (8, "1000")] {
        let mut diff =
            ctx.state
                .get_diff(tile, bel, "PRESENT", format!("FIFO36_EXP.TWR_DLY.{val}"));
        diff = diff.combine(&!&present_fifo36);
        if i == 0 {
            diff.assert_empty();
        }
        let (diff_l, diff_u) = split_diff_ul(diff);
        diffs_l.push((i, diff_l));
        diffs_u.push((i, diff_u));
    }
    ctx.tiledb
        .insert(tile, bel, "TWR_DLY_L", xlat_enum_int(diffs_l));
    ctx.tiledb
        .insert(tile, bel, "TWR_DLY_U", xlat_enum_int(diffs_u));

    for val in ["0", "101", "1010", "1111"] {
        let mut diff =
            ctx.state
                .get_diff(tile, bel, "PRESENT", format!("FIFO36_EXP.TSCRUB_DLY.{val}"));
        diff = diff.combine(&!&present_fifo36);
        if matches!(val, "101" | "1111") {
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TWR_DLY_L"), 8, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TWR_DLY_U"), 8, 0);
        }
        if matches!(val, "1010" | "1111") {
            let (diff_l, diff_u) = split_diff_ul(diff);
            ctx.tiledb
                .insert(tile, bel, "TSCRUB_DLY_L", xlat_bitvec(vec![diff_l]));
            ctx.tiledb
                .insert(tile, bel, "TSCRUB_DLY_U", xlat_bitvec(vec![diff_u]));
        } else {
            diff.assert_empty();
        }
    }

    for (hwpin, mode, pin) in [
        ("CLKARDCLKL", "RAMB18X2", "CLKAL"),
        ("CLKARDCLKU", "RAMB18X2", "CLKAU"),
        ("CLKBWRCLKL", "RAMB18X2", "CLKBL"),
        ("CLKBWRCLKU", "RAMB18X2", "CLKBU"),
        ("CLKARDCLKL", "RAMB18X2SDP", "RDCLKL"),
        ("CLKARDCLKU", "RAMB18X2SDP", "RDCLKU"),
        ("CLKBWRCLKL", "RAMB18X2SDP", "WRCLKL"),
        ("CLKBWRCLKU", "RAMB18X2SDP", "WRCLKU"),
        ("CLKARDCLKL", "RAMB36_EXP", "CLKAL"),
        ("CLKARDCLKU", "RAMB36_EXP", "CLKAU"),
        ("CLKBWRCLKL", "RAMB36_EXP", "CLKBL"),
        ("CLKBWRCLKU", "RAMB36_EXP", "CLKBU"),
        ("CLKARDCLKL", "RAMB36SDP_EXP", "RDCLKL"),
        ("CLKARDCLKU", "RAMB36SDP_EXP", "RDCLKU"),
        ("CLKBWRCLKL", "RAMB36SDP_EXP", "WRCLKL"),
        ("CLKBWRCLKU", "RAMB36SDP_EXP", "WRCLKU"),
        ("CLKARDCLKL", "RAMBFIFO18", "RDCLK"),
        ("CLKARDCLKU", "RAMBFIFO18", "CLKA"),
        ("CLKBWRCLKL", "RAMBFIFO18", "WRCLK"),
        ("CLKBWRCLKU", "RAMBFIFO18", "CLKB"),
        ("CLKARDCLKL", "RAMBFIFO18_36", "RDCLK"),
        ("CLKARDCLKU", "RAMBFIFO18_36", "RDCLKU"),
        ("CLKBWRCLKL", "RAMBFIFO18_36", "WRCLK"),
        ("CLKBWRCLKU", "RAMBFIFO18_36", "WRCLKU"),
        ("CLKARDCLKL", "FIFO36_EXP", "RDCLKL"),
        ("CLKARDCLKU", "FIFO36_EXP", "RDCLKU"),
        ("CLKBWRCLKL", "FIFO36_EXP", "WRCLKL"),
        ("CLKBWRCLKU", "FIFO36_EXP", "WRCLKU"),
        ("CLKARDCLKL", "FIFO36_72_EXP", "RDCLKL"),
        ("CLKARDCLKU", "FIFO36_72_EXP", "RDCLKU"),
        ("CLKBWRCLKL", "FIFO36_72_EXP", "WRCLKL"),
        ("CLKBWRCLKU", "FIFO36_72_EXP", "WRCLKU"),
        ("REGCLKARDRCLKL", "RAMB18X2", "REGCLKAL"),
        ("REGCLKARDRCLKU", "RAMB18X2", "REGCLKAU"),
        ("REGCLKBWRRCLKL", "RAMB18X2", "REGCLKBL"),
        ("REGCLKBWRRCLKU", "RAMB18X2", "REGCLKBU"),
        ("REGCLKARDRCLKL", "RAMB18X2SDP", "RDRCLKL"),
        ("REGCLKARDRCLKU", "RAMB18X2SDP", "RDRCLKU"),
        ("REGCLKARDRCLKL", "RAMB36_EXP", "REGCLKAL"),
        ("REGCLKARDRCLKU", "RAMB36_EXP", "REGCLKAU"),
        ("REGCLKBWRRCLKL", "RAMB36_EXP", "REGCLKBL"),
        ("REGCLKBWRRCLKU", "RAMB36_EXP", "REGCLKBU"),
        ("REGCLKARDRCLKL", "RAMB36SDP_EXP", "RDRCLKL"),
        ("REGCLKARDRCLKU", "RAMB36SDP_EXP", "RDRCLKU"),
        ("REGCLKARDRCLKL", "RAMBFIFO18", "RDRCLK"),
        ("REGCLKARDRCLKU", "RAMBFIFO18", "REGCLKA"),
        ("REGCLKBWRRCLKU", "RAMBFIFO18", "REGCLKB"),
        ("REGCLKARDRCLKL", "RAMBFIFO18_36", "RDRCLK"),
        ("REGCLKARDRCLKU", "RAMBFIFO18_36", "RDRCLKU"),
        ("REGCLKARDRCLKL", "FIFO36_EXP", "RDRCLKL"),
        ("REGCLKARDRCLKU", "FIFO36_EXP", "RDRCLKU"),
        ("REGCLKARDRCLKL", "FIFO36_72_EXP", "RDRCLKL"),
        ("REGCLKARDRCLKU", "FIFO36_72_EXP", "RDRCLKU"),
        ("SSRARSTL", "RAMB18X2", "SSRAL"),
        ("SSRAU", "RAMB18X2", "SSRAU"),
        ("SSRBL", "RAMB18X2", "SSRBL"),
        ("SSRBU", "RAMB18X2", "SSRBU"),
        ("SSRARSTL", "RAMB18X2SDP", "SSRL"),
        ("SSRAU", "RAMB18X2SDP", "SSRU"),
        ("SSRARSTL", "RAMB36_EXP", "SSRAL"),
        ("SSRAU", "RAMB36_EXP", "SSRAU"),
        ("SSRBL", "RAMB36_EXP", "SSRBL"),
        ("SSRBU", "RAMB36_EXP", "SSRBU"),
        ("SSRARSTL", "RAMB36SDP_EXP", "SSRL"),
        ("SSRAU", "RAMB36SDP_EXP", "SSRU"),
        ("SSRARSTL", "RAMBFIFO18", "RST"),
        ("SSRAU", "RAMBFIFO18", "SSRA"),
        ("SSRBU", "RAMBFIFO18", "SSRB"),
        ("SSRARSTL", "RAMBFIFO18_36", "RST"),
        ("SSRAU", "RAMBFIFO18_36", "SSRU"),
        ("SSRARSTL", "FIFO36_EXP", "RST"),
        ("SSRARSTL", "FIFO36_72_EXP", "RST"),
        ("ENARDENL", "RAMB18X2", "ENAL"),
        ("ENAU", "RAMB18X2", "ENAU"),
        ("ENBWRENL", "RAMB18X2", "ENBL"),
        ("ENBU", "RAMB18X2", "ENBU"),
        ("ENARDENL", "RAMB18X2SDP", "RDENL"),
        ("ENAU", "RAMB18X2SDP", "RDENU"),
        ("ENBWRENL", "RAMB18X2SDP", "WRENL"),
        ("ENBU", "RAMB18X2SDP", "WRENU"),
        ("ENARDENL", "RAMB36_EXP", "ENAL"),
        ("ENAU", "RAMB36_EXP", "ENAU"),
        ("ENBWRENL", "RAMB36_EXP", "ENBL"),
        ("ENBU", "RAMB36_EXP", "ENBU"),
        ("ENARDENL", "RAMB36SDP_EXP", "RDENL"),
        ("ENAU", "RAMB36SDP_EXP", "RDENU"),
        ("ENBWRENL", "RAMB36SDP_EXP", "WRENL"),
        ("ENBU", "RAMB36SDP_EXP", "WRENU"),
        ("ENARDENL", "RAMBFIFO18", "RDEN"),
        ("ENAU", "RAMBFIFO18", "ENA"),
        ("ENBWRENL", "RAMBFIFO18", "WREN"),
        ("ENBU", "RAMBFIFO18", "ENB"),
        ("ENARDENL", "RAMBFIFO18_36", "RDEN"),
        ("ENAU", "RAMBFIFO18_36", "RDENU"),
        ("ENBWRENL", "RAMBFIFO18_36", "WREN"),
        ("ENBU", "RAMBFIFO18_36", "WRENU"),
        ("ENARDENL", "FIFO36_EXP", "RDEN"),
        ("ENBWRENL", "FIFO36_EXP", "WREN"),
        ("ENARDENL", "FIFO36_72_EXP", "RDEN"),
        ("ENBWRENL", "FIFO36_72_EXP", "WREN"),
    ] {
        let item = ctx.extract_enum_bool(
            tile,
            bel,
            &format!("{pin}INV.{mode}"),
            pin,
            &format!("{pin}_B"),
        );
        ctx.tiledb.insert(tile, bel, format!("INV.{hwpin}"), item);
    }
    for hwpin in [
        "CLKARDCLKL",
        "CLKARDCLKU",
        "CLKBWRCLKL",
        "CLKBWRCLKU",
        "REGCLKARDRCLKL",
        "REGCLKARDRCLKU",
        "REGCLKBWRRCLKL",
        "REGCLKBWRRCLKU",
        "SSRARSTL",
        "SSRAU",
        "SSRBL",
        "SSRBU",
        "ENARDENL",
        "ENAU",
        "ENBWRENL",
        "ENBU",
    ] {
        let item = ctx.tiledb.item(tile, bel, &format!("INV.{hwpin}"));
        present_rambfifo36.apply_bit_diff(item, false, true);
        present_ramb18x2.apply_bit_diff(item, false, true);
        present_ramb18x2sdp.apply_bit_diff(item, false, true);
        present_ramb36.apply_bit_diff(item, false, true);
        present_ramb36sdp.apply_bit_diff(item, false, true);
        present_rambfifo18.apply_bit_diff(item, false, true);
        present_rambfifo18_36.apply_bit_diff(item, false, true);
        present_fifo36.apply_bit_diff(item, false, true);
        present_fifo36_72.apply_bit_diff(item, false, true);
    }

    for mode in ["RAMBFIFO36", "RAMB36SDP_EXP", "FIFO36_72_EXP"] {
        let item =
            ctx.extract_enum_bool(tile, bel, &format!("EN_ECC_READ.{mode}"), "FALSE", "TRUE");
        ctx.tiledb.insert(tile, bel, "EN_ECC_READ", item);
        let item = ctx.extract_enum_bool(
            tile,
            bel,
            &format!("EN_ECC_WRITE.{mode}.READ"),
            "FALSE",
            "TRUE",
        );
        if mode == "RAMBFIFO36" {
            let item =
                ctx.extract_enum_bool(tile, bel, &format!("EN_ECC_WRITE.{mode}"), "FALSE", "TRUE");
            ctx.tiledb.insert(tile, bel, "EN_ECC_WRITE", item);
        } else {
            ctx.state
                .get_diff(tile, bel, format!("EN_ECC_WRITE.{mode}"), "FALSE")
                .assert_empty();
            let mut diff = ctx
                .state
                .get_diff(tile, bel, format!("EN_ECC_WRITE.{mode}"), "TRUE");
            diff.apply_bit_diff(&item, true, false);
            ctx.tiledb
                .insert(tile, bel, "EN_ECC_WRITE_NO_READ", xlat_bitvec(vec![diff]));
        }
        ctx.tiledb.insert(tile, bel, "EN_ECC_WRITE", item);
    }
    let item = ctx.extract_enum_bool(tile, bel, "EN_ECC_SCRUB.RAMBFIFO36", "FALSE", "TRUE");
    ctx.tiledb.insert(tile, bel, "EN_ECC_SCRUB", item);
    ctx.state
        .get_diff(tile, bel, "EN_ECC_SCRUB.RAMB36SDP_EXP", "FALSE")
        .assert_empty();
    let mut diff = ctx
        .state
        .get_diff(tile, bel, "EN_ECC_SCRUB.RAMB36SDP_EXP", "TRUE");
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_ECC_SCRUB"), true, false);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TWR_DLY_L"), 8, 0);
    diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TWR_DLY_U"), 8, 0);
    diff.assert_empty();

    let item = ctx.extract_enum_bool(tile, bel, "IS_FIFO", "FALSE", "TRUE");
    present_rambfifo18.apply_bit_diff(&item, true, false);
    present_rambfifo18_36.apply_bit_diff(&item, true, false);
    present_fifo36.apply_bit_diff(&item, true, false);
    present_fifo36_72.apply_bit_diff(&item, true, false);
    ctx.tiledb.insert(tile, bel, "IS_FIFO", item);
    for mode in [
        "RAMBFIFO36",
        "RAMBFIFO18",
        "RAMBFIFO18_36",
        "FIFO36_EXP",
        "FIFO36_72_EXP",
    ] {
        let item = ctx.extract_enum_bool(
            tile,
            bel,
            &format!("FIRST_WORD_FALL_THROUGH.{mode}"),
            "FALSE",
            "TRUE",
        );
        ctx.tiledb
            .insert(tile, bel, "FIRST_WORD_FALL_THROUGH", item);
        if mode != "RAMBFIFO36" {
            for attr in ["ALMOST_FULL_OFFSET", "ALMOST_EMPTY_OFFSET"] {
                let item = ctx.extract_bitvec(tile, bel, &format!("{attr}.{mode}"), "");
                ctx.tiledb.insert(tile, bel, attr, item);
            }
        }
    }
    for attr in ["ALMOST_FULL_OFFSET", "ALMOST_EMPTY_OFFSET"] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_rambfifo36.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_ramb18x2.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_ramb18x2sdp.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_ramb36.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_ramb36sdp.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_rambfifo18.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_rambfifo18_36.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_fifo36.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
        present_fifo36_72.apply_bitvec_diff(item, &bitvec![0; 13], &bitvec![1; 13]);
    }

    for (attr, attr_l, attr_u) in [
        ("WRITE_MODE_A", "WRITE_MODE_A_L", "WRITE_MODE_A_U"),
        ("WRITE_MODE_B", "WRITE_MODE_B_L", "WRITE_MODE_B_U"),
    ] {
        for val in ["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"] {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("{attr}.RAMB36_EXP"), val);
            let diff_l = ctx
                .state
                .peek_diff(tile, bel, format!("{attr_l}.RAMB18X2"), val);
            let diff_u = ctx
                .state
                .peek_diff(tile, bel, format!("{attr_u}.RAMB18X2"), val);
            assert_eq!(diff, diff_l.combine(diff_u));
        }
    }
    for (hwattr, mode, attr) in [
        ("WRITE_MODE_A_L", "RAMBFIFO36", "WRITE_MODE_A_L"),
        ("WRITE_MODE_A_U", "RAMBFIFO36", "WRITE_MODE_A_U"),
        ("WRITE_MODE_B_L", "RAMBFIFO36", "WRITE_MODE_B_L"),
        ("WRITE_MODE_B_U", "RAMBFIFO36", "WRITE_MODE_B_U"),
        ("WRITE_MODE_A_L", "RAMB18X2", "WRITE_MODE_A_L"),
        ("WRITE_MODE_A_U", "RAMB18X2", "WRITE_MODE_A_U"),
        ("WRITE_MODE_B_L", "RAMB18X2", "WRITE_MODE_B_L"),
        ("WRITE_MODE_B_U", "RAMB18X2", "WRITE_MODE_B_U"),
        ("WRITE_MODE_A_U", "RAMBFIFO18", "WRITE_MODE_A"),
        ("WRITE_MODE_B_U", "RAMBFIFO18", "WRITE_MODE_B"),
    ] {
        let item = ctx.extract_enum(
            tile,
            bel,
            &format!("{attr}.{mode}"),
            &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"],
        );
        ctx.tiledb.insert(tile, bel, hwattr, item);
    }
    for attr in [
        "WRITE_MODE_A_L",
        "WRITE_MODE_A_U",
        "WRITE_MODE_B_L",
        "WRITE_MODE_B_U",
    ] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_ramb18x2sdp.apply_enum_diff(item, "READ_FIRST", "WRITE_FIRST");
        present_ramb36sdp.apply_enum_diff(item, "READ_FIRST", "WRITE_FIRST");
    }
    for attr in ["WRITE_MODE_A_L", "WRITE_MODE_B_L"] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_rambfifo18.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
        present_rambfifo18_36.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
        present_fifo36.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
        present_fifo36_72.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
    }
    for attr in ["WRITE_MODE_A_U", "WRITE_MODE_B_U"] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_rambfifo18_36.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
        present_fifo36.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
        present_fifo36_72.apply_enum_diff(item, "NO_CHANGE", "WRITE_FIRST");
    }

    for mode in ["RAMBFIFO36", "RAMB36_EXP"] {
        for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
            ctx.tiledb.insert(
                tile,
                bel,
                attr,
                xlat_enum(vec![
                    (
                        "NONE_UPPER",
                        ctx.state
                            .get_diff(tile, bel, format!("{attr}.{mode}"), "NONE"),
                    ),
                    (
                        "NONE_UPPER",
                        ctx.state
                            .get_diff(tile, bel, format!("{attr}.{mode}"), "UPPER"),
                    ),
                    (
                        "LOWER",
                        ctx.state
                            .get_diff(tile, bel, format!("{attr}.{mode}"), "LOWER"),
                    ),
                ]),
            )
        }
    }

    for attr in ["INIT", "SRVAL"] {
        for ab in ['A', 'B'] {
            for ul in ['U', 'L'] {
                for mode in ["RAMBFIFO36", "RAMB18X2"] {
                    let item =
                        ctx.extract_bitvec(tile, bel, &format!("{attr}_{ab}_{ul}.{mode}"), "");
                    ctx.tiledb
                        .insert(tile, bel, format!("{attr}_{ab}_{ul}"), item);
                }
                let item = ctx.tiledb.item(tile, bel, &format!("{attr}_{ab}_{ul}"));
                present_rambfifo36.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_ramb18x2.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_ramb18x2sdp.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_ramb36.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_rambfifo18.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_rambfifo18_36.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_fifo36.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
                present_fifo36_72.apply_bitvec_diff(item, &bitvec![0; 18], &bitvec![1; 18]);
            }
            let diffs = ctx
                .state
                .get_diffs(tile, bel, format!("{attr}_{ab}.RAMB36_EXP"), "");
            let mut diffs_l = vec![];
            let mut diffs_u = vec![];
            for (i, diff) in diffs.into_iter().enumerate() {
                if i % 2 == 0 {
                    diffs_l.push(diff);
                } else {
                    diffs_u.push(diff);
                }
            }
            ctx.tiledb
                .insert(tile, bel, format!("{attr}_{ab}_L"), xlat_bitvec(diffs_l));
            ctx.tiledb
                .insert(tile, bel, format!("{attr}_{ab}_U"), xlat_bitvec(diffs_u));
        }
        for ul in ['U', 'L'] {
            let mut diffs = ctx
                .state
                .get_diffs(tile, bel, format!("{attr}_{ul}.RAMB18X2SDP"), "");
            let diffs_b_hi = diffs.split_off(34);
            let diffs_a_hi = diffs.split_off(32);
            let mut diffs_b = diffs.split_off(16);
            diffs.extend(diffs_a_hi);
            diffs_b.extend(diffs_b_hi);
            ctx.tiledb
                .insert(tile, bel, format!("{attr}_A_{ul}"), xlat_bitvec(diffs));
            ctx.tiledb
                .insert(tile, bel, format!("{attr}_B_{ul}"), xlat_bitvec(diffs_b));
        }
        let mut diffs_a = ctx
            .state
            .get_diffs(tile, bel, format!("{attr}.RAMB36SDP_EXP"), "");
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
            if i % 2 == 0 {
                diffs_a_l.push(diff);
            } else {
                diffs_a_u.push(diff);
            }
        }
        for (i, diff) in diffs_b.into_iter().enumerate() {
            if i % 2 == 0 {
                diffs_b_l.push(diff);
            } else {
                diffs_b_u.push(diff);
            }
        }
        ctx.tiledb
            .insert(tile, bel, format!("{attr}_A_L"), xlat_bitvec(diffs_a_l));
        ctx.tiledb
            .insert(tile, bel, format!("{attr}_A_U"), xlat_bitvec(diffs_a_u));
        ctx.tiledb
            .insert(tile, bel, format!("{attr}_B_L"), xlat_bitvec(diffs_b_l));
        ctx.tiledb
            .insert(tile, bel, format!("{attr}_B_U"), xlat_bitvec(diffs_b_u));
    }

    for val in ["0", "1"] {
        let diff = ctx.state.get_diff(tile, bel, "DO_REG.RAMB36SDP_EXP", val);
        assert_eq!(
            diff,
            ctx.state.get_diff(tile, bel, "DO_REG.FIFO36_EXP", val)
        );
        assert_eq!(
            diff,
            ctx.state.get_diff(tile, bel, "DO_REG.FIFO36_72_EXP", val)
        );

        let diff_a = ctx.state.peek_diff(tile, bel, "DOA_REG.RAMB36_EXP", val);
        let diff_b = ctx.state.peek_diff(tile, bel, "DOB_REG.RAMB36_EXP", val);
        assert_eq!(diff, diff_a.combine(diff_b));

        let diff = ctx.state.get_diff(tile, bel, "DO_REG_L.RAMBFIFO18_36", val);
        assert_eq!(
            &diff,
            ctx.state.peek_diff(tile, bel, "DO_REG_L.RAMB18X2SDP", val)
        );
    }
    for (attr, attr_l, attr_u) in [
        ("DOA_REG", "DOA_REG_L", "DOA_REG_U"),
        ("DOB_REG", "DOB_REG_L", "DOB_REG_U"),
    ] {
        for val in ["0", "1"] {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("{attr}.RAMB36_EXP"), val);
            let diff_l = ctx
                .state
                .peek_diff(tile, bel, format!("{attr_l}.RAMB18X2"), val);
            let diff_u = ctx
                .state
                .peek_diff(tile, bel, format!("{attr_u}.RAMB18X2"), val);
            assert_eq!(diff, diff_l.combine(diff_u));
        }
    }
    for (attr, attr_a, attr_b) in [
        ("DO_REG_L", "DOA_REG_L", "DOB_REG_L"),
        ("DO_REG_U", "DOA_REG_U", "DOB_REG_U"),
    ] {
        for val in ["0", "1"] {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("{attr}.RAMB18X2SDP"), val);
            let diff_a = ctx
                .state
                .peek_diff(tile, bel, format!("{attr_a}.RAMB18X2"), val);
            let diff_b = ctx
                .state
                .peek_diff(tile, bel, format!("{attr_b}.RAMB18X2"), val);
            assert_eq!(diff, diff_a.combine(diff_b));
        }
    }

    for (hwattr, mode, attr) in [
        ("DOA_REG_L", "RAMBFIFO36", "DOA_REG_L"),
        ("DOA_REG_U", "RAMBFIFO36", "DOA_REG_U"),
        ("DOB_REG_L", "RAMBFIFO36", "DOB_REG_L"),
        ("DOB_REG_U", "RAMBFIFO36", "DOB_REG_U"),
        ("DOA_REG_L", "RAMB18X2", "DOA_REG_L"),
        ("DOA_REG_U", "RAMB18X2", "DOA_REG_U"),
        ("DOB_REG_L", "RAMB18X2", "DOB_REG_L"),
        ("DOB_REG_U", "RAMB18X2", "DOB_REG_U"),
        ("DOA_REG_L", "RAMBFIFO18", "DO_REG"),
        ("DOA_REG_U", "RAMBFIFO18", "DOA_REG"),
        ("DOB_REG_U", "RAMBFIFO18", "DOB_REG"),
    ] {
        let item = ctx.extract_enum(tile, bel, &format!("{attr}.{mode}"), &["0", "1"]);
        ctx.tiledb.insert(tile, bel, hwattr, item);
    }
    for attr in ["DOA_REG_L", "DOB_REG_L"] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_rambfifo18.apply_enum_diff(item, "1", "0");
        present_rambfifo18_36.apply_enum_diff(item, "1", "0");
        present_fifo36.apply_enum_diff(item, "1", "0");
        present_fifo36_72.apply_enum_diff(item, "1", "0");
    }
    for attr in ["DOA_REG_U", "DOB_REG_U"] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_fifo36.apply_enum_diff(item, "1", "0");
        present_fifo36_72.apply_enum_diff(item, "1", "0");
    }

    let item = ctx.extract_enum_bool(tile, bel, "EN_SYN.RAMBFIFO36", "FALSE", "TRUE");
    ctx.tiledb.insert(tile, bel, "EN_SYN", item);
    ctx.state
        .get_diff(tile, bel, "EN_SYN.RAMBFIFO18", "FALSE")
        .assert_empty();
    ctx.state
        .get_diff(tile, bel, "EN_SYN.RAMBFIFO18_36", "FALSE")
        .assert_empty();
    ctx.state
        .get_diff(tile, bel, "EN_SYN.FIFO36_EXP", "FALSE")
        .assert_empty();
    ctx.state
        .get_diff(tile, bel, "EN_SYN.FIFO36_72_EXP", "FALSE")
        .assert_empty();
    for mode in ["RAMBFIFO18", "RAMBFIFO18_36"] {
        let mut diff = ctx
            .state
            .get_diff(tile, bel, format!("EN_SYN.{mode}"), "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOA_REG_L"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOB_REG_L"), "0", "1");
        ctx.tiledb
            .insert(tile, bel, "EN_SYN", xlat_bitvec(vec![diff]));
    }
    for mode in ["FIFO36_EXP", "FIFO36_72_EXP"] {
        let mut diff = ctx
            .state
            .get_diff(tile, bel, format!("EN_SYN.{mode}"), "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOA_REG_L"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOA_REG_U"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOB_REG_L"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOB_REG_U"), "0", "1");
        ctx.tiledb
            .insert(tile, bel, "EN_SYN", xlat_bitvec(vec![diff]));
    }
    let mut d0 = ctx.state.get_diff(tile, bel, "DO_REG_U.RAMBFIFO18_36", "0");
    let mut d1 = ctx.state.get_diff(tile, bel, "DO_REG_U.RAMBFIFO18_36", "1");
    d1 = d1.combine(&!&d0);
    for attr in [
        "SRVAL_A_L",
        "SRVAL_A_U",
        "SRVAL_B_L",
        "SRVAL_B_U",
        "INIT_A_L",
        "INIT_A_U",
        "INIT_B_L",
        "INIT_B_U",
    ] {
        d0.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, attr),
            &bitvec![1; 18],
            &bitvec![0; 18],
        );
    }
    d0.assert_empty();
    d1.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOA_REG_U"), "1", "0");
    d1.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOB_REG_U"), "1", "0");
    d1.assert_empty();

    for rw in ["READ", "WRITE"] {
        for ab in ['A', 'B'] {
            ctx.state
                .get_diff(tile, bel, format!("{rw}_WIDTH_{ab}.RAMB36_EXP"), "0")
                .assert_empty();
            let item = ctx.extract_bit(tile, bel, &format!("{rw}_WIDTH_{ab}.RAMB36_EXP"), "1");
            for (val, val2) in [
                ("1", "2"),
                ("2", "4"),
                ("4", "9"),
                ("9", "18"),
                ("18", "36"),
            ] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, format!("{rw}_WIDTH_{ab}.RAMB36_EXP"), val2);
                if val2 == "9" {
                    diff.apply_bit_diff(&item, true, false);
                }
                let diff_l =
                    ctx.state
                        .peek_diff(tile, bel, format!("{rw}_WIDTH_{ab}_L.RAMB18X2"), val);
                let diff_u =
                    ctx.state
                        .peek_diff(tile, bel, format!("{rw}_WIDTH_{ab}_U.RAMB18X2"), val);
                assert_eq!(diff, diff_l.combine(diff_u));
            }
            ctx.tiledb
                .insert(tile, bel, format!("{rw}_MUX_UL_{ab}"), item);
        }
    }
    for ab in ['A', 'B'] {
        for (val, isr) in [
            // ????????? the fuck were they smoking
            ("0", bitvec![1; 18]),
            (
                "1",
                bitvec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                "2",
                bitvec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (
                "4",
                bitvec![1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (
                "9",
                bitvec![1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            ("18", bitvec![1; 18]),
        ] {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("WRITE_WIDTH_{ab}.RAMBFIFO18"), val);
            assert_eq!(
                &diff,
                ctx.state
                    .peek_diff(tile, bel, format!("WRITE_WIDTH_{ab}_U.RAMB18X2"), val)
            );
            let mut diff =
                ctx.state
                    .get_diff(tile, bel, format!("READ_WIDTH_{ab}.RAMBFIFO18"), val);
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, &format!("INIT_{ab}_L")),
                &isr,
                &bitvec![1; 18],
            );
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, &format!("SRVAL_{ab}_L")),
                &isr,
                &bitvec![1; 18],
            );
            assert_eq!(
                &diff,
                ctx.state
                    .peek_diff(tile, bel, format!("READ_WIDTH_{ab}_U.RAMB18X2"), val)
            );
        }
    }
    for attr in [
        "READ_WIDTH_A_L",
        "READ_WIDTH_A_U",
        "READ_WIDTH_B_L",
        "READ_WIDTH_B_U",
        "WRITE_WIDTH_A_L",
        "WRITE_WIDTH_A_U",
        "WRITE_WIDTH_B_L",
        "WRITE_WIDTH_B_U",
    ] {
        ctx.state
            .get_diff(tile, bel, format!("{attr}.RAMB18X2"), "0")
            .assert_empty();
        let item = ctx.extract_enum(
            tile,
            bel,
            &format!("{attr}.RAMB18X2"),
            &["1", "2", "4", "9", "18"],
        );
        present_ramb18x2sdp.apply_enum_diff(&item, "18", "1");
        present_ramb36sdp.apply_enum_diff(&item, "18", "1");
        present_rambfifo18_36.apply_enum_diff(&item, "18", "1");
        present_fifo36_72.apply_enum_diff(&item, "18", "1");
        ctx.tiledb.insert(tile, bel, attr, item);
    }
    for ab in ['A', 'B'] {
        for ul in ['U', 'L'] {
            for rw in ["WRITE", "READ"] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, format!("{rw}_WIDTH_{ab}_{ul}.RAMBFIFO36"), "36");
                diff.apply_enum_diff(
                    ctx.tiledb.item(tile, bel, &format!("{rw}_WIDTH_{ab}_{ul}")),
                    "18",
                    "1",
                );
                if ab == 'A' {
                    ctx.tiledb
                        .insert(tile, bel, format!("{rw}_SDP_{ul}"), xlat_bitvec(vec![diff]));
                } else {
                    diff.assert_empty();
                }
            }
        }
    }
    for attr in ["READ_SDP_L", "READ_SDP_U", "WRITE_SDP_L", "WRITE_SDP_U"] {
        let item = ctx.tiledb.item(tile, bel, attr);
        present_ramb18x2sdp.apply_bit_diff(item, true, false);
        present_ramb36sdp.apply_bit_diff(item, true, false);
        present_rambfifo18_36.apply_bit_diff(item, true, false);
        present_fifo36_72.apply_bit_diff(item, true, false);
    }

    for mode in ["RAMB18X2", "RAMB18X2SDP"] {
        for ul in ['U', 'L'] {
            let mut data = vec![];
            let mut datap = vec![];
            for i in 0..0x40 {
                data.extend(ctx.state.get_diffs(
                    tile,
                    bel,
                    format!("INIT_{i:02X}_{ul}.{mode}"),
                    "",
                ));
            }
            for i in 0..8 {
                datap.extend(ctx.state.get_diffs(
                    tile,
                    bel,
                    format!("INITP_{i:02X}_{ul}.{mode}"),
                    "",
                ));
            }
            ctx.tiledb
                .insert(tile, bel, format!("DATA_{ul}"), xlat_bitvec(data));
            ctx.tiledb
                .insert(tile, bel, format!("DATAP_{ul}"), xlat_bitvec(datap));
        }
    }
    for mode in ["RAMBFIFO18", "RAMBFIFO18_36"] {
        let mut data = vec![];
        let mut datap = vec![];
        for i in 0..0x40 {
            data.extend(
                ctx.state
                    .get_diffs(tile, bel, format!("INIT_{i:02X}.{mode}"), ""),
            );
        }
        for i in 0..8 {
            datap.extend(
                ctx.state
                    .get_diffs(tile, bel, format!("INITP_{i:02X}.{mode}"), ""),
            );
        }
        ctx.tiledb.insert(tile, bel, "DATA_U", xlat_bitvec(data));
        ctx.tiledb.insert(tile, bel, "DATAP_U", xlat_bitvec(datap));
    }
    for mode in ["RAMB36_EXP", "RAMB36SDP_EXP"] {
        let mut data = vec![];
        let mut datap = vec![];
        for i in 0..0x80 {
            data.extend(
                ctx.state
                    .get_diffs(tile, bel, format!("INIT_{i:02X}.{mode}"), ""),
            );
        }
        for i in 0..0x10 {
            datap.extend(
                ctx.state
                    .get_diffs(tile, bel, format!("INITP_{i:02X}.{mode}"), ""),
            );
        }
        let mut data_l = vec![];
        let mut data_u = vec![];
        for (i, diff) in data.into_iter().enumerate() {
            if i % 2 == 0 {
                data_l.push(diff);
            } else {
                data_u.push(diff);
            }
        }
        let mut datap_l = vec![];
        let mut datap_u = vec![];
        for (i, diff) in datap.into_iter().enumerate() {
            if i % 2 == 0 {
                datap_l.push(diff);
            } else {
                datap_u.push(diff);
            }
        }
        ctx.tiledb.insert(tile, bel, "DATA_L", xlat_bitvec(data_l));
        ctx.tiledb.insert(tile, bel, "DATA_U", xlat_bitvec(data_u));
        ctx.tiledb
            .insert(tile, bel, "DATAP_L", xlat_bitvec(datap_l));
        ctx.tiledb
            .insert(tile, bel, "DATAP_U", xlat_bitvec(datap_u));
    }

    ctx.collect_enum_bool_wide(tile, bel, "SAVEDATA", "FALSE", "TRUE");

    present_rambfifo36.assert_empty();
    present_ramb18x2.assert_empty();
    present_ramb18x2sdp.assert_empty();
    present_ramb36.assert_empty();
    present_rambfifo18.assert_empty();

    let mut diffs = vec![];
    for val in ["4", "9", "18"] {
        let mut diff = ctx.state.get_diff(tile, bel, "DATA_WIDTH.RAMBFIFO18", val);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "READ_WIDTH_A_L"), val, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "READ_WIDTH_B_L"), val, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "WRITE_WIDTH_A_L"), val, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "WRITE_WIDTH_B_L"), val, "1");
        diffs.push((val, diff));
    }
    for (val, hwval) in [("4", "2"), ("9", "4"), ("18", "9"), ("36", "18")] {
        let mut diff = ctx.state.get_diff(tile, bel, "DATA_WIDTH.FIFO36_EXP", val);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "READ_WIDTH_A_L"), hwval, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "WRITE_WIDTH_B_L"), hwval, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "READ_WIDTH_A_U"), hwval, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "WRITE_WIDTH_B_U"), hwval, "1");
        if val == "9" {
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "READ_MUX_UL_A"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "READ_MUX_UL_B"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "WRITE_MUX_UL_A"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "WRITE_MUX_UL_B"), true, false);
        }
        if val == "36" {
            // what the fuck.
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "READ_WIDTH_B_L"), hwval, "1");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "WRITE_WIDTH_A_L"), hwval, "1");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "READ_WIDTH_B_U"), hwval, "1");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "WRITE_WIDTH_A_U"), hwval, "1");
        }
        diffs.push((hwval, diff));
    }
    diffs.push(("36", present_rambfifo18_36));
    ctx.tiledb.insert(tile, bel, "FIFO_WIDTH", xlat_enum(diffs));

    present_ramb36sdp.apply_enum_diff(ctx.tiledb.item(tile, bel, "FIFO_WIDTH"), "36", "2");
    present_fifo36_72.apply_enum_diff(ctx.tiledb.item(tile, bel, "FIFO_WIDTH"), "36", "2");
    present_ramb36sdp.assert_empty();

    assert_eq!(present_fifo36, present_fifo36_72);
    ctx.tiledb
        .insert(tile, bel, "IS_FIFO_U", xlat_bitvec(vec![present_fifo36]));
}
