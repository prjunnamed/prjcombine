use prjcombine_entity::EntityId;
use prjcombine_re_collector::{
    diff::Diff,
    legacy::{
        extract_bitvec_val_legacy, extract_bitvec_val_part_legacy, xlat_bit_legacy,
        xlat_bitvec_legacy, xlat_enum_legacy,
    },
};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bits;
use prjcombine_virtex2::{
    chip::ChipKind, defs, defs::spartan3::tcls as tcls_s3, defs::virtex2::tcls as tcls_v2,
};

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
    let tcid = match grid_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => tcls_v2::BRAM,
        ChipKind::Spartan3 => tcls_s3::BRAM_S3,
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => tcls_s3::BRAM_S3E,
        ChipKind::Spartan3A => tcls_s3::BRAM_S3A,
        ChipKind::Spartan3ADsp => tcls_s3::BRAM_S3ADSP,
    };
    let mut ctx = FuzzCtx::new_id(session, backend, tcid);
    let mut bctx = ctx.bel(defs::bslots::BRAM);
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
        let mut bctx = ctx.bel(defs::bslots::MULT);
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
                            .pip(mult_pin, (defs::bslots::BRAM, bram_pin))
                            .commit();
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let chip_kind = match ctx.edev {
        ExpandedDevice::Virtex2(edev) => edev.chip.kind,
        _ => unreachable!(),
    };
    let int_tiles = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => &["INT_BRAM"; 4],
        ChipKind::Spartan3 => &["INT_BRAM_S3"; 4],
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => &["INT_BRAM_S3E"; 4],
        ChipKind::Spartan3A => &[
            "INT_BRAM_S3A_03",
            "INT_BRAM_S3A_12",
            "INT_BRAM_S3A_12",
            "INT_BRAM_S3A_03",
        ],
        ChipKind::Spartan3ADsp => &["INT_BRAM_S3ADSP"; 4],
    };
    let tile = match chip_kind {
        ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => "BRAM",
        ChipKind::Spartan3 => "BRAM_S3",
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => "BRAM_S3E",
        ChipKind::Spartan3A => "BRAM_S3A",
        ChipKind::Spartan3ADsp => "BRAM_S3ADSP",
    };
    fn filter_ab(diff: Diff) -> (Diff, Diff) {
        (
            Diff {
                bits: diff
                    .bits
                    .iter()
                    .filter(|&(&a, _)| a.rect.to_idx() < 2)
                    .map(|(&a, &b)| (a, b))
                    .collect(),
            },
            Diff {
                bits: diff
                    .bits
                    .iter()
                    .filter(|&(&a, _)| a.rect.to_idx() >= 2)
                    .map(|(&a, &b)| (a, b))
                    .collect(),
            },
        )
    }
    if devdata_only {
        if !chip_kind.is_virtex2() {
            let present_base = ctx.get_diff_legacy(tile, "BRAM", "PRESENT", "BASE");
            let all_0 = ctx.get_diff_legacy(tile, "BRAM", "PRESENT", "ALL_0");
            let mut diff = present_base.combine(&!all_0);
            let adef = extract_bitvec_val_part_legacy(
                ctx.item(tile, "BRAM", "DDEL_A"),
                &bits![0, 0],
                &mut diff,
            );
            ctx.insert_device_data("BRAM:DDEL_A_DEFAULT", adef);
            if chip_kind != ChipKind::Spartan3 {
                let bdef = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "BRAM", "DDEL_B"),
                    &bits![0, 0],
                    &mut diff,
                );
                ctx.insert_device_data("BRAM:DDEL_B_DEFAULT", bdef);
            }

            let adef = extract_bitvec_val_part_legacy(
                ctx.item(tile, "BRAM", "WDEL_A"),
                &bits![0, 0, 0],
                &mut diff,
            );
            ctx.insert_device_data("BRAM:WDEL_A_DEFAULT", adef);
            if chip_kind != ChipKind::Spartan3 {
                let bdef = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "BRAM", "WDEL_B"),
                    &bits![0, 0, 0],
                    &mut diff,
                );
                ctx.insert_device_data("BRAM:WDEL_B_DEFAULT", bdef);
            }
            diff.assert_empty();
        }
        return;
    }
    let present_base = ctx.get_diff_legacy(tile, "BRAM", "PRESENT", "BASE");
    let mut present = present_base.clone();
    if !chip_kind.is_virtex2() {
        let diff_base = ctx.get_diff_legacy(tile, "BRAM", "PRESENT", "DDEL_00");
        let diff0 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "DDEL_01")
            .combine(&!&diff_base);
        let diff1 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "DDEL_10")
            .combine(&!&diff_base);
        let diff_def = present_base.combine(&!diff_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        let (adef, bdef) = filter_ab(diff_def);
        let ddel_a = xlat_bitvec_legacy(vec![a0, a1]);
        let adef = extract_bitvec_val_legacy(&ddel_a, &bits![0, 0], adef);
        ctx.insert(tile, "BRAM", "DDEL_A", ddel_a);
        ctx.insert_device_data("BRAM:DDEL_A_DEFAULT", adef);
        present.discard_bits_legacy(ctx.item(tile, "BRAM", "DDEL_A"));
        if chip_kind == ChipKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            bdef.assert_empty();
        } else {
            let ddel_b = xlat_bitvec_legacy(vec![b0, b1]);
            let bdef = extract_bitvec_val_legacy(&ddel_b, &bits![0, 0], bdef);
            ctx.insert(tile, "BRAM", "DDEL_B", ddel_b);
            ctx.insert_device_data("BRAM:DDEL_B_DEFAULT", bdef);
            present.discard_bits_legacy(ctx.item(tile, "BRAM", "DDEL_B"));
        }

        let diff_base = ctx.get_diff_legacy(tile, "BRAM", "PRESENT", "WDEL_000");
        let diff0 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "WDEL_001")
            .combine(&!&diff_base);
        let diff1 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "WDEL_010")
            .combine(&!&diff_base);
        let diff2 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "WDEL_100")
            .combine(&!&diff_base);
        let diff_def = present_base.combine(&!diff_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        let (a2, b2) = filter_ab(diff2);
        let (adef, bdef) = filter_ab(diff_def);
        let wdel_a = xlat_bitvec_legacy(vec![a0, a1, a2]);
        let adef = extract_bitvec_val_legacy(&wdel_a, &bits![0, 0, 0], adef);
        ctx.insert_device_data("BRAM:WDEL_A_DEFAULT", adef);
        ctx.insert(tile, "BRAM", "WDEL_A", wdel_a);
        present.discard_bits_legacy(ctx.item(tile, "BRAM", "WDEL_A"));
        if chip_kind == ChipKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            b2.assert_empty();
            bdef.assert_empty();
        } else {
            let wdel_b = xlat_bitvec_legacy(vec![b0, b1, b2]);
            let bdef = extract_bitvec_val_legacy(&wdel_b, &bits![0, 0, 0], bdef);
            ctx.insert(tile, "BRAM", "WDEL_B", wdel_b);
            ctx.insert_device_data("BRAM:WDEL_B_DEFAULT", bdef);
            present.discard_bits_legacy(ctx.item(tile, "BRAM", "WDEL_B"));
        }

        let diff0 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "WW_VALUE_0")
            .combine(&!&present_base);
        let diff1 = ctx
            .get_diff_legacy(tile, "BRAM", "PRESENT", "WW_VALUE_1")
            .combine(&!&present_base);
        let (a0, b0) = filter_ab(diff0);
        let (a1, b1) = filter_ab(diff1);
        ctx.insert(
            tile,
            "BRAM",
            "WW_VALUE_A",
            xlat_enum_legacy(vec![("NONE", Diff::default()), ("0", a0), ("1", a1)]),
        );
        ctx.insert(
            tile,
            "BRAM",
            "WW_VALUE_B",
            xlat_enum_legacy(vec![("NONE", Diff::default()), ("0", b0), ("1", b1)]),
        );
    }

    let mut diffs_data = vec![];
    let mut diffs_datap = vec![];
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "CLKA", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "CLKB", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "ENA", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "ENB", false);
    present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "BRAM", "ENA"));
    present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "BRAM", "ENB"));
    match chip_kind {
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
            for pin in [
                "WEA0", "WEB0", "WEA1", "WEB1", "WEA2", "WEB2", "WEA3", "WEB3",
            ] {
                ctx.collect_int_inv(int_tiles, tile, "BRAM", pin, false);
                present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "BRAM", pin));
            }
            for i in 0..0x40 {
                diffs_data.extend(ctx.get_diffs_legacy(tile, "BRAM", format!("INIT_{i:02X}"), ""));
            }
            for i in 0..0x08 {
                diffs_datap.extend(ctx.get_diffs_legacy(
                    tile,
                    "BRAM",
                    format!("INITP_{i:02X}"),
                    "",
                ));
            }
            for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
                ctx.collect_enum_legacy(
                    tile,
                    "BRAM",
                    attr,
                    &["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"],
                );
            }
            for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
                ctx.get_diff_legacy(tile, "BRAM", attr, "0").assert_empty();
                ctx.collect_enum_legacy(tile, "BRAM", attr, &["1", "2", "4", "9", "18", "36"]);
            }
            if chip_kind == ChipKind::Spartan3ADsp {
                for pin in ["RSTA", "RSTB", "REGCEA", "REGCEB"] {
                    ctx.collect_int_inv(int_tiles, tile, "BRAM", pin, false);
                    present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "BRAM", pin));
                }

                ctx.collect_enum_legacy(tile, "BRAM", "DOA_REG", &["0", "1"]);
                ctx.collect_enum_legacy(tile, "BRAM", "DOB_REG", &["0", "1"]);
                ctx.collect_enum_legacy(tile, "BRAM", "RSTTYPE", &["ASYNC", "SYNC"]);
            } else {
                for pin in ["SSRA", "SSRB"] {
                    ctx.collect_int_inv(int_tiles, tile, "BRAM", pin, false);
                }
            }
        }
        _ => {
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "WEA", false);
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "WEB", false);
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "SSRA", false);
            ctx.collect_int_inv(int_tiles, tile, "BRAM", "SSRB", false);
            for pin in ["WEA", "WEB", "SSRA", "SSRB"] {
                present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "BRAM", pin));
            }
            for i in 0..0x40 {
                diffs_data.extend(ctx.get_diffs_legacy(tile, "BRAM", format!("INIT_{i:02x}"), ""));
            }
            for i in 0..0x08 {
                diffs_datap.extend(ctx.get_diffs_legacy(
                    tile,
                    "BRAM",
                    format!("INITP_{i:02x}"),
                    "",
                ));
            }
            for (dattr, sattr) in [
                ("WRITE_MODE_A", "WRITEMODEA"),
                ("WRITE_MODE_B", "WRITEMODEB"),
            ] {
                let diffs = ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"]
                    .into_iter()
                    .map(|val| (val, ctx.get_diff_legacy(tile, "BRAM", sattr, val)))
                    .collect();
                ctx.insert(tile, "BRAM", dattr, xlat_enum_legacy(diffs));
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
                .map(|(dval, sval)| (dval, ctx.get_diff_legacy(tile, "BRAM", sattr, sval)))
                .collect();
                ctx.insert(tile, "BRAM", dattr, xlat_enum_legacy(diffs));
            }
            if chip_kind.is_virtex2() {
                ctx.get_diff_legacy(tile, "BRAM", "SAVEDATA", "FALSE")
                    .assert_empty();
                let diff = ctx.get_diff_legacy(tile, "BRAM", "SAVEDATA", "TRUE");
                let mut bits: Vec<_> = diff.bits.into_iter().collect();
                bits.sort();
                ctx.insert(
                    tile,
                    "BRAM",
                    "SAVEDATA",
                    xlat_bitvec_legacy(
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
    ctx.insert(tile, "BRAM", "DATA", xlat_bitvec_legacy(diffs_data));
    ctx.insert(tile, "BRAM", "DATAP", xlat_bitvec_legacy(diffs_datap));
    ctx.collect_bitvec_legacy(tile, "BRAM", "INIT_A", "");
    ctx.collect_bitvec_legacy(tile, "BRAM", "INIT_B", "");
    ctx.collect_bitvec_legacy(tile, "BRAM", "SRVAL_A", "");
    ctx.collect_bitvec_legacy(tile, "BRAM", "SRVAL_B", "");
    present.discard_bits_legacy(ctx.item(tile, "BRAM", "DATA_WIDTH_A"));
    present.discard_bits_legacy(ctx.item(tile, "BRAM", "DATA_WIDTH_B"));
    if chip_kind.is_spartan3a() {
        ctx.insert(
            tile,
            "BRAM",
            "UNK_PRESENT",
            xlat_enum_legacy(vec![("0", Diff::default()), ("1", present)]),
        );
    } else {
        present.assert_empty();
    }

    if chip_kind != ChipKind::Spartan3ADsp {
        let mut present = ctx.get_diff_legacy(tile, "MULT", "PRESENT", "1");
        if chip_kind.is_virtex2() || chip_kind == ChipKind::Spartan3 {
            let f_clk = ctx.get_diff_legacy(tile, "MULT", "CLKINV", "CLK");
            let f_clk_b = ctx.get_diff_legacy(tile, "MULT", "CLKINV", "CLK_B");
            let (f_clk, f_clk_b, f_reg) = Diff::split(f_clk, f_clk_b);
            f_clk.assert_empty();
            ctx.insert(tile, "MULT", "REG", xlat_bit_legacy(f_reg));
            ctx.insert_int_inv(int_tiles, tile, "MULT", "CLK", xlat_bit_legacy(f_clk_b));
            ctx.collect_int_inv(int_tiles, tile, "MULT", "CE", false);
            ctx.collect_int_inv(int_tiles, tile, "MULT", "RST", false);
            present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "MULT", "CE"));
        } else {
            for pin in ["CLK", "CEA", "CEB", "CEP", "RSTA", "RSTB", "RSTP"] {
                ctx.collect_int_inv(int_tiles, tile, "MULT", pin, false);
            }
            ctx.collect_enum_legacy(tile, "MULT", "AREG", &["0", "1"]);
            ctx.collect_enum_legacy(tile, "MULT", "BREG", &["0", "1"]);
            ctx.collect_enum_legacy(tile, "MULT", "PREG", &["0", "1"]);
            ctx.collect_enum_legacy(tile, "MULT", "B_INPUT", &["DIRECT", "CASCADE"]);
            ctx.get_diff_legacy(tile, "MULT", "PREG_CLKINVERSION", "0")
                .assert_empty();
            let item = xlat_bitvec_legacy(vec![ctx.get_diff_legacy(
                tile,
                "MULT",
                "PREG_CLKINVERSION",
                "1",
            )]);
            ctx.insert(tile, "MULT", "PREG_CLKINVERSION", item);
            present.discard_bits_legacy(ctx.item(tile, "MULT", "PREG_CLKINVERSION"));
            present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "MULT", "CEA"));
            present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "MULT", "CEB"));
            present.discard_bits_legacy(&ctx.item_int_inv(int_tiles, tile, "MULT", "CEP"));
            if chip_kind == ChipKind::Spartan3A {
                for ab in ['A', 'B'] {
                    for i in 0..18 {
                        let name = &*format!("MUX.{ab}{i}");
                        let item = xlat_enum_legacy(vec![
                            ("INT", Diff::default()),
                            ("BRAM", ctx.get_diff_legacy(tile, "MULT", name, "BRAM")),
                        ]);
                        ctx.insert(tile, "MULT", name, item);
                    }
                }
            }
        }
        present.assert_empty();
    }
}
