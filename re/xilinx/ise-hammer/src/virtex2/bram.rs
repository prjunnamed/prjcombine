use prjcombine_re_fpga_hammer::{
    Diff, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bitvec, xlat_enum,
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bits;
use prjcombine_virtex2::{bels, chip::ChipKind};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzBuilderBel, FuzzCtx},
};

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let grid_kind = match backend.edev {
        ExpandedDevice::Virtex2(edev) => edev.chip.kind,
        _ => unreachable!(),
    };
    let tile_name = match grid_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => "BRAM",
        ChipKind::Spartan3 => "BRAM.S3",
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => "BRAM.S3E",
        ChipKind::Spartan3A => "BRAM.S3A",
        ChipKind::Spartan3ADsp => "BRAM.S3ADSP",
    };
    let mut ctx = FuzzCtx::new(session, backend, tile_name);
    let mut bctx = ctx.bel(bels::BRAM);
    let mode = match grid_kind {
        ChipKind::Spartan3ADsp => "RAMB16BWER",
        ChipKind::Spartan3A => "RAMB16BWE",
        _ => "RAMB16",
    };
    let test_present = |builder: FuzzBuilderBel, val: &str| match grid_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            builder
                .global_mutex("BRAM_OPTS", val)
                .test_manual("PRESENT", val)
                .mode(mode)
                .attr("DATA_WIDTH_A", "36")
                .attr("DATA_WIDTH_B", "36")
                .attr("SRVAL_A", "fffffffff")
                .attr("SRVAL_B", "fffffffff")
                .attr("INIT_A", "fffffffff")
                .attr("INIT_B", "fffffffff")
                .commit();
        }
        _ => {
            builder
                .global_mutex("BRAM_OPTS", val)
                .test_manual("PRESENT", val)
                .mode(mode)
                .attr("PORTA_ATTR", "512X36")
                .attr("PORTB_ATTR", "512X36")
                .attr("SRVAL_A", "fffffffff")
                .attr("SRVAL_B", "fffffffff")
                .attr("INIT_A", "fffffffff")
                .attr("INIT_B", "fffffffff")
                .commit();
        }
    };
    if devdata_only {
        if !grid_kind.is_virtex2() {
            test_present(bctx.build(), "BASE");
            test_present(
                bctx.build()
                    .global("Ibram_ddel0", "0")
                    .global("Ibram_ddel1", "0")
                    .global("Ibram_wdel0", "0")
                    .global("Ibram_wdel1", "0")
                    .global("Ibram_wdel2", "0"),
                "ALL_0",
            );
        }
        return;
    }
    match grid_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            for pin in [
                "CLKA", "CLKB", "ENA", "ENB", "WEA0", "WEA1", "WEA2", "WEA3", "WEB0", "WEB1",
                "WEB2", "WEB3",
            ] {
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_inv(pin);
            }
            if grid_kind == ChipKind::Spartan3ADsp {
                for pin in ["RSTA", "RSTB", "REGCEA", "REGCEB"] {
                    bctx.mode(mode)
                        .attr("DATA_WIDTH_A", "36")
                        .attr("DATA_WIDTH_B", "36")
                        .test_inv(pin);
                }
            } else {
                for pin in ["SSRA", "SSRB"] {
                    bctx.mode(mode)
                        .attr("DATA_WIDTH_A", "36")
                        .attr("DATA_WIDTH_B", "36")
                        .test_inv(pin);
                }
            }
            for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
                bctx.mode(mode)
                    .attr("INIT_A", "0")
                    .attr("INIT_B", "0")
                    .attr("SRVAL_A", "0")
                    .attr("SRVAL_B", "0")
                    .test_enum(attr, &["0", "1", "2", "4", "9", "18", "36"]);
            }
            for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_enum(attr, &["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"]);
            }
            if grid_kind == ChipKind::Spartan3ADsp {
                bctx.mode(mode).test_enum("RSTTYPE", &["SYNC", "ASYNC"]);
                bctx.mode(mode).test_enum("DOA_REG", &["0", "1"]);
                bctx.mode(mode).test_enum("DOB_REG", &["0", "1"]);
            }
            for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_multi_attr_hex(attr, 36);
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02X}");
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_multi_attr_hex(attr, 256);
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02X}");
                bctx.mode(mode)
                    .attr("DATA_WIDTH_A", "36")
                    .attr("DATA_WIDTH_B", "36")
                    .test_multi_attr_hex(attr, 256);
            }
        }
        _ => {
            for pin in ["CLKA", "CLKB", "SSRA", "SSRB", "WEA", "WEB", "ENA", "ENB"] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_inv(pin);
            }
            for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
                bctx.mode(mode)
                    .attr("INIT_A", "0")
                    .attr("INIT_B", "0")
                    .attr("SRVAL_A", "0")
                    .attr("SRVAL_B", "0")
                    .test_enum(
                        attr,
                        &["16384X1", "8192X2", "4096X4", "2048X9", "1024X18", "512X36"],
                    );
            }
            for attr in ["WRITEMODEA", "WRITEMODEB"] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_enum(attr, &["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"]);
            }
            if grid_kind.is_virtex2() {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_enum("SAVEDATA", &["FALSE", "TRUE"]);
            }
            for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_multi_attr_hex(attr, 36);
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02x}");
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_multi_attr_hex(attr, 256);
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02x}");
                bctx.mode(mode)
                    .attr("PORTA_ATTR", "512X36")
                    .attr("PORTB_ATTR", "512X36")
                    .test_multi_attr_hex(attr, 256);
            }
        }
    }
    test_present(bctx.build(), "BASE");
    if !grid_kind.is_virtex2() {
        test_present(
            bctx.build()
                .global("Ibram_ddel0", "0")
                .global("Ibram_ddel1", "0"),
            "DDEL_00",
        );
        test_present(
            bctx.build()
                .global("Ibram_ddel0", "1")
                .global("Ibram_ddel1", "0"),
            "DDEL_01",
        );
        test_present(
            bctx.build()
                .global("Ibram_ddel0", "0")
                .global("Ibram_ddel1", "1"),
            "DDEL_10",
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "0")
                .global("Ibram_wdel1", "0")
                .global("Ibram_wdel2", "0"),
            "WDEL_000",
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "1")
                .global("Ibram_wdel1", "0")
                .global("Ibram_wdel2", "0"),
            "WDEL_001",
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "0")
                .global("Ibram_wdel1", "1")
                .global("Ibram_wdel2", "0"),
            "WDEL_010",
        );
        test_present(
            bctx.build()
                .global("Ibram_wdel0", "0")
                .global("Ibram_wdel1", "0")
                .global("Ibram_wdel2", "1"),
            "WDEL_100",
        );
        test_present(bctx.build().global("Ibram_ww_value", "0"), "WW_VALUE_0");
        test_present(bctx.build().global("Ibram_ww_value", "1"), "WW_VALUE_1");
    }
    if grid_kind != ChipKind::Spartan3ADsp {
        // mult
        let mut bctx = ctx.bel(bels::MULT);
        let mode = if matches!(grid_kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
            "MULT18X18SIO"
        } else {
            "MULT18X18"
        };
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        if !matches!(grid_kind, ChipKind::Spartan3E | ChipKind::Spartan3A) {
            for pin in ["CLK", "RST", "CE"] {
                bctx.mode(mode).test_inv(pin);
            }
        } else {
            for pin in ["CLK", "RSTA", "RSTB", "RSTP", "CEA", "CEB", "CEP"] {
                bctx.mode(mode).test_inv(pin);
            }
            bctx.mode(mode).test_enum("AREG", &["0", "1"]);
            bctx.mode(mode).test_enum("BREG", &["0", "1"]);
            bctx.mode(mode).test_enum("PREG", &["0", "1"]);
            bctx.mode(mode).test_enum("PREG_CLKINVERSION", &["0", "1"]);
            bctx.mode(mode).test_enum("B_INPUT", &["DIRECT", "CASCADE"]);
            if grid_kind == ChipKind::Spartan3A {
                for ab in ['A', 'B'] {
                    for i in 0..18 {
                        let name = format!("MUX.{ab}{i}");
                        let bram_pin = if i < 16 {
                            format!("DO{ab}{i}")
                        } else {
                            format!("DOP{ab}{ii}", ii = i - 16)
                        };
                        let mult_pin = format!("{ab}{i}");
                        bctx.test_manual(name, "BRAM")
                            .pip(mult_pin, (bels::BRAM, bram_pin))
                            .commit();
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let grid_kind = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => edev.chip.kind,
        _ => unreachable!(),
    };
    let int_tiles = match grid_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &["INT.BRAM"; 4],
        ChipKind::Spartan3 => &["INT.BRAM.S3"; 4],
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => &["INT.BRAM.S3E"; 4],
        ChipKind::Spartan3A => &[
            "INT.BRAM.S3A.03",
            "INT.BRAM.S3A.12",
            "INT.BRAM.S3A.12",
            "INT.BRAM.S3A.03",
        ],
        ChipKind::Spartan3ADsp => &["INT.BRAM.S3ADSP"; 4],
    };
    let tile = match grid_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => "BRAM",
        ChipKind::Spartan3 => "BRAM.S3",
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => "BRAM.S3E",
        ChipKind::Spartan3A => "BRAM.S3A",
        ChipKind::Spartan3ADsp => "BRAM.S3ADSP",
    };
    fn filter_ab(diff: Diff) -> (Diff, Diff) {
        (
            Diff {
                bits: diff
                    .bits
                    .iter()
                    .filter(|&(&a, _)| a.tile < 2)
                    .map(|(&a, &b)| (a, b))
                    .collect(),
            },
            Diff {
                bits: diff
                    .bits
                    .iter()
                    .filter(|&(&a, _)| a.tile >= 2)
                    .map(|(&a, &b)| (a, b))
                    .collect(),
            },
        )
    }
    if devdata_only {
        if !grid_kind.is_virtex2() {
            let present_base = ctx.state.get_diff(tile, "BRAM", "PRESENT", "BASE");
            let all_0 = ctx.state.get_diff(tile, "BRAM", "PRESENT", "ALL_0");
            let mut diff = present_base.combine(&!all_0);
            let adef = extract_bitvec_val_part(
                ctx.tiledb.item(tile, "BRAM", "DDEL_A"),
                &bits![0, 0],
                &mut diff,
            );
            ctx.insert_device_data("BRAM:DDEL_A_DEFAULT", adef);
            if grid_kind != ChipKind::Spartan3 {
                let bdef = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "BRAM", "DDEL_B"),
                    &bits![0, 0],
                    &mut diff,
                );
                ctx.insert_device_data("BRAM:DDEL_B_DEFAULT", bdef);
            }

            let adef = extract_bitvec_val_part(
                ctx.tiledb.item(tile, "BRAM", "WDEL_A"),
                &bits![0, 0, 0],
                &mut diff,
            );
            ctx.insert_device_data("BRAM:WDEL_A_DEFAULT", adef);
            if grid_kind != ChipKind::Spartan3 {
                let bdef = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "BRAM", "WDEL_B"),
                    &bits![0, 0, 0],
                    &mut diff,
                );
                ctx.insert_device_data("BRAM:WDEL_B_DEFAULT", bdef);
            }
            diff.assert_empty();
        }
        return;
    }
    let present_base = ctx.state.get_diff(tile, "BRAM", "PRESENT", "BASE");
    let mut present = present_base.clone();
    if !grid_kind.is_virtex2() {
        let diff_base = ctx.state.get_diff(tile, "BRAM", "PRESENT", "DDEL_00");
        let diff0 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "DDEL_01")
            .combine(&!&diff_base);
        let diff1 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "DDEL_10")
            .combine(&!&diff_base);
        let diff_def = present_base.combine(&!diff_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        let (adef, bdef) = filter_ab(diff_def);
        let ddel_a = xlat_bitvec(vec![a0, a1]);
        let adef = extract_bitvec_val(&ddel_a, &bits![0, 0], adef);
        ctx.tiledb.insert(tile, "BRAM", "DDEL_A", ddel_a);
        ctx.insert_device_data("BRAM:DDEL_A_DEFAULT", adef);
        present.discard_bits(ctx.tiledb.item(tile, "BRAM", "DDEL_A"));
        if grid_kind == ChipKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            bdef.assert_empty();
        } else {
            let ddel_b = xlat_bitvec(vec![b0, b1]);
            let bdef = extract_bitvec_val(&ddel_b, &bits![0, 0], bdef);
            ctx.tiledb.insert(tile, "BRAM", "DDEL_B", ddel_b);
            ctx.insert_device_data("BRAM:DDEL_B_DEFAULT", bdef);
            present.discard_bits(ctx.tiledb.item(tile, "BRAM", "DDEL_B"));
        }

        let diff_base = ctx.state.get_diff(tile, "BRAM", "PRESENT", "WDEL_000");
        let diff0 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "WDEL_001")
            .combine(&!&diff_base);
        let diff1 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "WDEL_010")
            .combine(&!&diff_base);
        let diff2 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "WDEL_100")
            .combine(&!&diff_base);
        let diff_def = present_base.combine(&!diff_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        let (a2, b2) = filter_ab(diff2);
        let (adef, bdef) = filter_ab(diff_def);
        let wdel_a = xlat_bitvec(vec![a0, a1, a2]);
        let adef = extract_bitvec_val(&wdel_a, &bits![0, 0, 0], adef);
        ctx.insert_device_data("BRAM:WDEL_A_DEFAULT", adef);
        ctx.tiledb.insert(tile, "BRAM", "WDEL_A", wdel_a);
        present.discard_bits(ctx.tiledb.item(tile, "BRAM", "WDEL_A"));
        if grid_kind == ChipKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            b2.assert_empty();
            bdef.assert_empty();
        } else {
            let wdel_b = xlat_bitvec(vec![b0, b1, b2]);
            let bdef = extract_bitvec_val(&wdel_b, &bits![0, 0, 0], bdef);
            ctx.tiledb.insert(tile, "BRAM", "WDEL_B", wdel_b);
            ctx.insert_device_data("BRAM:WDEL_B_DEFAULT", bdef);
            present.discard_bits(ctx.tiledb.item(tile, "BRAM", "WDEL_B"));
        }

        let diff0 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "WW_VALUE_0")
            .combine(&!&present_base);
        let diff1 = ctx
            .state
            .get_diff(tile, "BRAM", "PRESENT", "WW_VALUE_1")
            .combine(&!&present_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        ctx.tiledb.insert(
            tile,
            "BRAM",
            "WW_VALUE_A",
            xlat_enum(vec![("NONE", Diff::default()), ("0", a0), ("1", a1)]),
        );
        ctx.tiledb.insert(
            tile,
            "BRAM",
            "WW_VALUE_B",
            xlat_enum(vec![("NONE", Diff::default()), ("0", b0), ("1", b1)]),
        );
    }

    let mut diffs_data = vec![];
    let mut diffs_datap = vec![];
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "CLKA", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "CLKB", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "ENA", grid_kind.is_virtex2());
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "ENB", grid_kind.is_virtex2());
    present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", "ENA"));
    present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", "ENB"));
    match grid_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            for pin in [
                "WEA0", "WEB0", "WEA1", "WEB1", "WEA2", "WEB2", "WEA3", "WEB3",
            ] {
                ctx.collect_int_inv(int_tiles, tile, "BRAM", pin, false);
                present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", pin));
            }
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
            for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
                ctx.collect_enum(
                    tile,
                    "BRAM",
                    attr,
                    &["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"],
                );
            }
            for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
                ctx.state.get_diff(tile, "BRAM", attr, "0").assert_empty();
                ctx.collect_enum(tile, "BRAM", attr, &["1", "2", "4", "9", "18", "36"]);
            }
            if grid_kind == ChipKind::Spartan3ADsp {
                for pin in ["RSTA", "RSTB", "REGCEA", "REGCEB"] {
                    ctx.collect_int_inv(int_tiles, tile, "BRAM", pin, false);
                    present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", pin));
                }

                ctx.collect_enum(tile, "BRAM", "DOA_REG", &["0", "1"]);
                ctx.collect_enum(tile, "BRAM", "DOB_REG", &["0", "1"]);
                ctx.collect_enum(tile, "BRAM", "RSTTYPE", &["ASYNC", "SYNC"]);
            } else {
                for pin in ["SSRA", "SSRB"] {
                    ctx.collect_int_inv(int_tiles, tile, "BRAM", pin, false);
                }
            }
        }
        _ => {
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "WEA", false);
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "WEB", false);
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "SSRA", grid_kind.is_virtex2());
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "SSRB", grid_kind.is_virtex2());
            for pin in ["WEA", "WEB", "SSRA", "SSRB"] {
                present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", pin));
            }
            for i in 0..0x40 {
                diffs_data.extend(
                    ctx.state
                        .get_diffs(tile, "BRAM", format!("INIT_{i:02x}"), ""),
                );
            }
            for i in 0..0x08 {
                diffs_datap.extend(
                    ctx.state
                        .get_diffs(tile, "BRAM", format!("INITP_{i:02x}"), ""),
                );
            }
            for (dattr, sattr) in [
                ("WRITE_MODE_A", "WRITEMODEA"),
                ("WRITE_MODE_B", "WRITEMODEB"),
            ] {
                let diffs = ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"]
                    .into_iter()
                    .map(|val| (val, ctx.state.get_diff(tile, "BRAM", sattr, val)))
                    .collect();
                ctx.tiledb.insert(tile, "BRAM", dattr, xlat_enum(diffs));
            }
            for (dattr, sattr) in [
                ("DATA_WIDTH_A", "PORTA_ATTR"),
                ("DATA_WIDTH_B", "PORTB_ATTR"),
            ] {
                let diffs = [
                    ("1", "16384X1"),
                    ("2", "8192X2"),
                    ("4", "4096X4"),
                    ("9", "2048X9"),
                    ("18", "1024X18"),
                    ("36", "512X36"),
                ]
                .into_iter()
                .map(|(dval, sval)| (dval, ctx.state.get_diff(tile, "BRAM", sattr, sval)))
                .collect();
                ctx.tiledb.insert(tile, "BRAM", dattr, xlat_enum(diffs));
            }
            if grid_kind.is_virtex2() {
                ctx.state
                    .get_diff(tile, "BRAM", "SAVEDATA", "FALSE")
                    .assert_empty();
                let diff = ctx.state.get_diff(tile, "BRAM", "SAVEDATA", "TRUE");
                let mut bits: Vec<_> = diff.bits.into_iter().collect();
                bits.sort();
                ctx.tiledb.insert(
                    tile,
                    "BRAM",
                    "SAVEDATA",
                    xlat_bitvec(
                        bits.into_iter()
                            .map(|(k, v)| Diff {
                                bits: [(k, v)].into_iter().collect(),
                            })
                            .collect(),
                    ),
                )
            }
        }
    }
    ctx.tiledb
        .insert(tile, "BRAM", "DATA", xlat_bitvec(diffs_data));
    ctx.tiledb
        .insert(tile, "BRAM", "DATAP", xlat_bitvec(diffs_datap));
    ctx.collect_bitvec(tile, "BRAM", "INIT_A", "");
    ctx.collect_bitvec(tile, "BRAM", "INIT_B", "");
    ctx.collect_bitvec(tile, "BRAM", "SRVAL_A", "");
    ctx.collect_bitvec(tile, "BRAM", "SRVAL_B", "");
    present.discard_bits(ctx.tiledb.item(tile, "BRAM", "DATA_WIDTH_A"));
    present.discard_bits(ctx.tiledb.item(tile, "BRAM", "DATA_WIDTH_B"));
    if grid_kind.is_spartan3a() {
        ctx.tiledb.insert(
            tile,
            "BRAM",
            "UNK_PRESENT",
            xlat_enum(vec![("0", Diff::default()), ("1", present)]),
        );
    } else {
        present.assert_empty();
    }

    if grid_kind != ChipKind::Spartan3ADsp {
        let mut present = ctx.state.get_diff(tile, "MULT", "PRESENT", "1");
        if grid_kind.is_virtex2() || grid_kind == ChipKind::Spartan3 {
            let f_clk = ctx.state.get_diff(tile, "MULT", "CLKINV", "CLK");
            let f_clk_b = ctx.state.get_diff(tile, "MULT", "CLKINV", "CLK_B");
            let (f_clk, f_clk_b, f_reg) = Diff::split(f_clk, f_clk_b);
            f_clk.assert_empty();
            ctx.tiledb.insert(tile, "MULT", "REG", xlat_bit(f_reg));
            ctx.insert_int_inv(int_tiles, tile, "MULT", "CLK", xlat_bit(f_clk_b));
            ctx.collect_int_inv(int_tiles, tile, "MULT", "CE", grid_kind.is_virtex2());
            ctx.collect_int_inv(int_tiles, tile, "MULT", "RST", grid_kind.is_virtex2());
            present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "MULT", "CE"));
        } else {
            for pin in ["CLK", "CEA", "CEB", "CEP", "RSTA", "RSTB", "RSTP"] {
                ctx.collect_int_inv(int_tiles, tile, "MULT", pin, false);
            }
            ctx.collect_enum(tile, "MULT", "AREG", &["0", "1"]);
            ctx.collect_enum(tile, "MULT", "BREG", &["0", "1"]);
            ctx.collect_enum(tile, "MULT", "PREG", &["0", "1"]);
            ctx.collect_enum(tile, "MULT", "B_INPUT", &["DIRECT", "CASCADE"]);
            ctx.state
                .get_diff(tile, "MULT", "PREG_CLKINVERSION", "0")
                .assert_empty();
            let item = xlat_bitvec(vec![ctx.state.get_diff(
                tile,
                "MULT",
                "PREG_CLKINVERSION",
                "1",
            )]);
            ctx.tiledb.insert(tile, "MULT", "PREG_CLKINVERSION", item);
            present.discard_bits(ctx.tiledb.item(tile, "MULT", "PREG_CLKINVERSION"));
            present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "MULT", "CEA"));
            present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "MULT", "CEB"));
            present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "MULT", "CEP"));
            if grid_kind == ChipKind::Spartan3A {
                for ab in ['A', 'B'] {
                    for i in 0..18 {
                        let name = &*format!("MUX.{ab}{i}");
                        let item = xlat_enum(vec![
                            ("INT", Diff::default()),
                            ("BRAM", ctx.state.get_diff(tile, "MULT", name, "BRAM")),
                        ]);
                        ctx.tiledb.insert(tile, "MULT", name, item);
                    }
                }
            }
        }
        present.assert_empty();
    }
}
