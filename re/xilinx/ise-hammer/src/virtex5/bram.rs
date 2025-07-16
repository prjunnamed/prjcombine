use prjcombine_re_fpga_hammer::{Diff, xlat_bit, xlat_bitvec, xlat_enum, xlat_enum_int};
use prjcombine_re_hammer::Session;
use prjcombine_types::bits;
use prjcombine_virtex4::bels;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "BRAM");
    let mut bctx = ctx.bel(bels::BRAM);

    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMBFIFO36")
        .mode("RAMBFIFO36")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMB18X2")
        .mode("RAMB18X2")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMB18X2SDP")
        .mode("RAMB18X2SDP")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMBFIFO18")
        .mode("RAMBFIFO18")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMBFIFO18_36")
        .mode("RAMBFIFO18_36")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMB36_EXP")
        .mode("RAMB36_EXP")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "RAMB36SDP_EXP")
        .mode("RAMB36SDP_EXP")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "FIFO36_EXP")
        .mode("FIFO36_EXP")
        .commit();
    bctx.build()
        .global_mutex("BRAM_OPT", "NONE")
        .test_manual("PRESENT", "FIFO36_72_EXP")
        .mode("FIFO36_72_EXP")
        .commit();

    for opt in [
        "TEST_FIFO_FLAG",
        "TEST_FIFO_OFFSET",
        "TEST_FIFO_CNT",
        "SWAP_CFGPORT",
        "BYPASS_RSR",
    ] {
        bctx.build()
            .global_mutex("BRAM_OPT", opt)
            .global(opt, "ENABLED")
            .test_manual("PRESENT", format!("FIFO36_EXP.{opt}"))
            .mode("FIFO36_EXP")
            .commit();
    }
    for val in ["WW0", "WW1"] {
        bctx.build()
            .global_mutex("BRAM_OPT", "WEAK_WRITE")
            .global("WEAK_WRITE", val)
            .test_manual("PRESENT", format!("FIFO36_EXP.WEAK_WRITE.{val}"))
            .mode("FIFO36_EXP")
            .commit();
    }
    for val in ["0", "1", "10", "11", "100", "101", "110", "111"] {
        bctx.build()
            .global_mutex("BRAM_OPT", "TRD_DLY")
            .global("TRD_DLY", val)
            .test_manual("PRESENT", format!("FIFO36_EXP.TRD_DLY.{val}"))
            .mode("FIFO36_EXP")
            .commit();
    }
    for val in ["0", "11", "101", "1000"] {
        bctx.build()
            .global_mutex("BRAM_OPT", "TWR_DLY")
            .global("TWR_DLY", val)
            .test_manual("PRESENT", format!("FIFO36_EXP.TWR_DLY.{val}"))
            .mode("FIFO36_EXP")
            .commit();
    }
    for val in ["0", "101", "1010", "1111"] {
        bctx.build()
            .global_mutex("BRAM_OPT", "TSCRUB_DLY")
            .global("TSCRUB_DLY", val)
            .test_manual("PRESENT", format!("FIFO36_EXP.TSCRUB_DLY.{val}"))
            .mode("FIFO36_EXP")
            .commit();
    }

    for pin in [
        "CLKAL", "CLKAU", "CLKBL", "CLKBU", "REGCLKAL", "REGCLKAU", "REGCLKBL", "REGCLKBU",
        "SSRAL", "SSRAU", "SSRBL", "SSRBU", "ENAL", "ENAU", "ENBL", "ENBU",
    ] {
        bctx.mode("RAMB18X2")
            .attr("DOA_REG_L", "1")
            .attr("DOA_REG_U", "1")
            .test_inv_suffix(pin, "RAMB18X2");
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "SSRL", "SSRU", "RDENL",
        "RDENU", "WRENL", "WRENU",
    ] {
        bctx.mode("RAMB18X2SDP")
            .attr("DO_REG_L", "1")
            .attr("DO_REG_U", "1")
            .test_inv_suffix(pin, "RAMB18X2SDP");
    }
    for pin in [
        "CLKAL", "CLKAU", "CLKBL", "CLKBU", "REGCLKAL", "REGCLKAU", "REGCLKBL", "REGCLKBU",
        "SSRAL", "SSRAU", "SSRBL", "SSRBU", "ENAL", "ENAU", "ENBL", "ENBU",
    ] {
        bctx.mode("RAMB36_EXP").test_inv_suffix(pin, "RAMB36_EXP");
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "SSRL", "SSRU", "RDENL",
        "RDENU", "WRENL", "WRENU",
    ] {
        bctx.mode("RAMB36SDP_EXP")
            .test_inv_suffix(pin, "RAMB36SDP_EXP");
    }
    for pin in [
        "RDCLK", "CLKA", "WRCLK", "CLKB", "RDRCLK", "REGCLKA", "REGCLKB", "RST", "SSRA", "SSRB",
        "RDEN", "ENA", "WREN", "ENB",
    ] {
        bctx.mode("RAMBFIFO18")
            .attr("DO_REG", "1")
            .attr("DOA_REG", "1")
            .attr("EN_SYN", "FALSE")
            .test_inv_suffix(pin, "RAMBFIFO18");
    }
    for pin in [
        "RDCLK", "RDCLKU", "WRCLK", "WRCLKU", "RDRCLK", "RDRCLKU", "RST", "SSRU", "RDEN", "RDENU",
        "WREN", "WRENU",
    ] {
        bctx.mode("RAMBFIFO18_36")
            .attr("DO_REG_L", "1")
            .attr("DO_REG_U", "1")
            .attr("EN_SYN", "FALSE")
            .test_inv_suffix(pin, "RAMBFIFO18_36");
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "RST", "RDEN", "WREN",
    ] {
        bctx.mode("FIFO36_EXP").test_inv_suffix(pin, "FIFO36_EXP");
    }
    for pin in [
        "RDCLKL", "RDCLKU", "WRCLKL", "WRCLKU", "RDRCLKL", "RDRCLKU", "RST", "RDEN", "WREN",
    ] {
        bctx.mode("FIFO36_72_EXP")
            .test_inv_suffix(pin, "FIFO36_72_EXP");
    }

    for mode in ["RAMBFIFO36", "RAMB36SDP_EXP", "FIFO36_72_EXP"] {
        bctx.mode(mode)
            .attr("EN_ECC_WRITE", "FALSE")
            .test_enum_suffix("EN_ECC_READ", mode, &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .attr("EN_ECC_READ", "FALSE")
            .test_enum_suffix("EN_ECC_WRITE", mode, &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .attr("EN_ECC_READ", "TRUE")
            .test_enum_suffix("EN_ECC_WRITE", format!("{mode}.READ"), &["FALSE", "TRUE"]);
        if mode != "FIFO36_72_EXP" {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .test_enum_suffix("EN_ECC_SCRUB", mode, &["FALSE", "TRUE"]);
        }
    }
    for mode in [
        "RAMBFIFO36",
        "RAMBFIFO18",
        "RAMBFIFO18_36",
        "FIFO36_EXP",
        "FIFO36_72_EXP",
    ] {
        bctx.mode(mode)
            .test_enum_suffix("EN_SYN", mode, &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum_suffix("FIRST_WORD_FALL_THROUGH", mode, &["FALSE", "TRUE"]);
        if mode != "RAMBFIFO36" {
            bctx.mode(mode)
                .attr("EN_SYN", "TRUE")
                .test_manual(format!("ALMOST_FULL_OFFSET.{mode}"), "")
                .multi_attr("ALMOST_FULL_OFFSET", MultiValue::Hex(0), 13);
            bctx.mode(mode)
                .attr("EN_SYN", "TRUE")
                .test_manual(format!("ALMOST_EMPTY_OFFSET.{mode}"), "")
                .multi_attr("ALMOST_EMPTY_OFFSET", MultiValue::Hex(0), 13);
        }
    }
    bctx.mode("RAMBFIFO36")
        .test_enum("IS_FIFO", &["FALSE", "TRUE"]);

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
        bctx.mode(mode)
            .attr(init, "0")
            .attr(srval, "0")
            .test_enum_suffix(attr, mode, &["0", "1"]);
    }
    bctx.mode("RAMBFIFO18_36")
        .attr("INIT", "fffffffff")
        .attr("SRVAL", "fffffffff")
        .test_enum_suffix("DO_REG_U", "RAMBFIFO18_36", &["0", "1"]);

    for (mode, attr) in [
        ("RAMBFIFO18", "DO_REG"),
        ("RAMBFIFO18_36", "DO_REG_L"),
        ("FIFO36_EXP", "DO_REG"),
        ("FIFO36_72_EXP", "DO_REG"),
    ] {
        bctx.mode(mode)
            .attr("EN_SYN", "TRUE")
            .test_enum_suffix(attr, mode, &["0", "1"]);
    }

    bctx.mode("RAMBFIFO18")
        .test_enum_suffix("DATA_WIDTH", "RAMBFIFO18", &["4", "9", "18"]);
    bctx.mode("FIFO36_EXP")
        .test_enum_suffix("DATA_WIDTH", "FIFO36_EXP", &["4", "9", "18", "36"]);
    for ab in ['A', 'B'] {
        for ul in ['U', 'L'] {
            bctx.mode("RAMBFIFO36")
                .test_manual(format!("READ_WIDTH_{ab}_{ul}.RAMBFIFO36"), "36")
                .attr(format!("READ_WIDTH_{ab}_{ul}"), "36")
                .attr(format!("DO{ab}_REG_{ul}"), "0")
                .attr(format!("INIT_{ab}_{ul}"), "0")
                .attr(format!("SRVAL_{ab}_{ul}"), "0")
                .commit();
            bctx.mode("RAMBFIFO36")
                .test_manual(format!("WRITE_WIDTH_{ab}_{ul}.RAMBFIFO36"), "36")
                .attr(format!("WRITE_WIDTH_{ab}_{ul}"), "36")
                .commit();
            bctx.mode("RAMB18X2")
                .attr(format!("INIT_{ab}_{ul}"), "0")
                .attr(format!("SRVAL_{ab}_{ul}"), "0")
                .test_enum_suffix(
                    format!("READ_WIDTH_{ab}_{ul}"),
                    "RAMB18X2",
                    &["0", "1", "2", "4", "9", "18"],
                );
            bctx.mode("RAMB18X2")
                .pin(format!("WE{ab}{ul}0"))
                .pin(format!("WE{ab}{ul}1"))
                .pin(format!("WE{ab}{ul}2"))
                .pin(format!("WE{ab}{ul}3"))
                .test_enum_suffix(
                    format!("WRITE_WIDTH_{ab}_{ul}"),
                    "RAMB18X2",
                    &["0", "1", "2", "4", "9", "18"],
                );
        }
        bctx.mode("RAMBFIFO18")
            .attr(format!("DO{ab}_REG"), "0")
            .attr(format!("INIT_{ab}"), "0")
            .attr(format!("SRVAL_{ab}"), "0")
            .test_enum_suffix(
                format!("READ_WIDTH_{ab}"),
                "RAMBFIFO18",
                &["0", "1", "2", "4", "9", "18"],
            );
        bctx.mode("RAMBFIFO18")
            .attr(format!("DO{ab}_REG"), "0")
            .pin(format!("WE{ab}0"))
            .pin(format!("WE{ab}1"))
            .pin(format!("WE{ab}2"))
            .pin(format!("WE{ab}3"))
            .test_enum_suffix(
                format!("WRITE_WIDTH_{ab}"),
                "RAMBFIFO18",
                &["0", "1", "2", "4", "9", "18"],
            );
        bctx.mode("RAMB36_EXP")
            .attr(format!("INIT_{ab}"), "0")
            .attr(format!("SRVAL_{ab}"), "0")
            .test_enum_suffix(
                format!("READ_WIDTH_{ab}"),
                "RAMB36_EXP",
                &["0", "1", "2", "4", "9", "18", "36"],
            );
        bctx.mode("RAMB36_EXP").test_enum_suffix(
            format!("WRITE_WIDTH_{ab}"),
            "RAMB36_EXP",
            &["0", "1", "2", "4", "9", "18", "36"],
        );
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
        bctx.mode(mode)
            .test_enum_suffix(attr, mode, &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"]);
    }
    for (mode, attr) in [
        ("RAMBFIFO36", "RAM_EXTENSION_A"),
        ("RAMBFIFO36", "RAM_EXTENSION_B"),
        ("RAMB36_EXP", "RAM_EXTENSION_A"),
        ("RAMB36_EXP", "RAM_EXTENSION_B"),
    ] {
        bctx.mode(mode)
            .test_enum_suffix(attr, mode, &["NONE", "UPPER", "LOWER"]);
    }

    for attr in ["INIT", "SRVAL"] {
        for ab in ['A', 'B'] {
            for ul in ['U', 'L'] {
                bctx.mode("RAMBFIFO36")
                    .attr("IS_FIFO", "FALSE")
                    .attr(format!("READ_WIDTH_{ab}_{ul}"), "18")
                    .test_manual(format!("{attr}_{ab}_{ul}.RAMBFIFO36"), "")
                    .multi_attr(format!("{attr}_{ab}_{ul}"), MultiValue::Hex(0), 18);
                bctx.mode("RAMB18X2")
                    .attr(format!("READ_WIDTH_{ab}_{ul}"), "18")
                    .test_manual(format!("{attr}_{ab}_{ul}.RAMB18X2"), "")
                    .multi_attr(format!("{attr}_{ab}_{ul}"), MultiValue::Hex(0), 18);
            }
            bctx.mode("RAMB36_EXP")
                .attr(format!("READ_WIDTH_{ab}"), "36")
                .test_manual(format!("{attr}_{ab}.RAMB36_EXP"), "")
                .multi_attr(format!("{attr}_{ab}"), MultiValue::Hex(0), 36);
        }
        for ul in ['U', 'L'] {
            bctx.mode("RAMB18X2SDP")
                .attr(format!("DO_REG_{ul}"), "0")
                .test_manual(format!("{attr}_{ul}.RAMB18X2SDP"), "")
                .multi_attr(format!("{attr}_{ul}"), MultiValue::Hex(0), 36);
        }
        bctx.mode("RAMB36SDP_EXP")
            .test_manual(format!("{attr}.RAMB36SDP_EXP"), "")
            .multi_attr(attr, MultiValue::Hex(0), 72);
    }

    for mode in ["RAMB18X2", "RAMB18X2SDP"] {
        for ul in ['U', 'L'] {
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
                    .test_manual(format!("INIT_{i:02X}_{ul}.{mode}"), "")
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
                    .test_manual(format!("INITP_{i:02X}_{ul}.{mode}"), "")
                    .multi_attr(format!("INITP_{i:02X}_{ul}"), MultiValue::Hex(0), 256);
            }
        }
    }
    for mode in ["RAMBFIFO18", "RAMBFIFO18_36"] {
        for i in 0..0x40 {
            bctx.mode(mode)
                .attr("DOA_REG", if mode == "RAMBFIFO18_36" { "" } else { "1" })
                .attr("DO_REG_U", if mode == "RAMBFIFO18_36" { "1" } else { "" })
                .test_manual(format!("INIT_{i:02X}.{mode}"), "")
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..8 {
            bctx.mode(mode)
                .attr("DOA_REG", if mode == "RAMBFIFO18_36" { "" } else { "1" })
                .attr("DO_REG_U", if mode == "RAMBFIFO18_36" { "1" } else { "" })
                .test_manual(format!("INITP_{i:02X}.{mode}"), "")
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
                .test_manual(format!("INIT_{i:02X}.{mode}"), "")
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 256);
        }
        for i in 0..0x10 {
            bctx.mode(mode)
                .attr(
                    "READ_WIDTH_A",
                    if mode == "RAMB36SDP_EXP" { "" } else { "36" },
                )
                .test_manual(format!("INITP_{i:02X}.{mode}"), "")
                .multi_attr(format!("INITP_{i:02X}"), MultiValue::Hex(0), 256);
        }
    }

    bctx.mode("RAMB36_EXP")
        .test_enum("SAVEDATA", &["FALSE", "TRUE"]);
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
        ctx.tiledb.insert(tile, bel, opt, xlat_bit(diff));
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
                .insert(tile, bel, "TSCRUB_DLY_L", xlat_bit(diff_l));
            ctx.tiledb
                .insert(tile, bel, "TSCRUB_DLY_U", xlat_bit(diff_u));
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
                .insert(tile, bel, "EN_ECC_WRITE_NO_READ", xlat_bit(diff));
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
            let item = xlat_enum(vec![
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
            ]);
            ctx.tiledb.insert(tile, bel, attr, item);
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
                present_rambfifo36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_ramb18x2.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_ramb18x2sdp.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_ramb36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_rambfifo18.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_rambfifo18_36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_fifo36.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
                present_fifo36_72.apply_bitvec_diff(item, &bits![0; 18], &bits![1; 18]);
            }
            let diffs = ctx
                .state
                .get_diffs(tile, bel, format!("{attr}_{ab}.RAMB36_EXP"), "");
            let mut diffs_l = vec![];
            let mut diffs_u = vec![];
            for (i, diff) in diffs.into_iter().enumerate() {
                if i.is_multiple_of(2) {
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
        ctx.tiledb.insert(tile, bel, "EN_SYN", xlat_bit(diff));
    }
    for mode in ["FIFO36_EXP", "FIFO36_72_EXP"] {
        let mut diff = ctx
            .state
            .get_diff(tile, bel, format!("EN_SYN.{mode}"), "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOA_REG_L"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOA_REG_U"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOB_REG_L"), "0", "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DOB_REG_U"), "0", "1");
        ctx.tiledb.insert(tile, bel, "EN_SYN", xlat_bit(diff));
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
            &bits![1; 18],
            &bits![0; 18],
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
            ("0", bits![1; 18]),
            (
                "1",
                bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (
                "2",
                bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (
                "4",
                bits![1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            (
                "9",
                bits![1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
            ),
            ("18", bits![1; 18]),
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
                &bits![1; 18],
            );
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, &format!("SRVAL_{ab}_L")),
                &isr,
                &bits![1; 18],
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
                        .insert(tile, bel, format!("{rw}_SDP_{ul}"), xlat_bit(diff));
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
        .insert(tile, bel, "IS_FIFO_U", xlat_bit(present_fifo36));
}
