use bitvec::prelude::*;
use prjcombine_re_collector::{extract_bitvec_val, xlat_bit, xlat_bitvec, xlat_enum, Diff};
use prjcombine_re_hammer::Session;
use prjcombine_interconnect::db::BelId;
use prjcombine_virtex2::grid::GridKind;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_multi,
    fuzz_one,
};

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    let grid_kind = match backend.edev {
        ExpandedDevice::Virtex2(ref edev) => edev.grid.kind,
        _ => unreachable!(),
    };
    let tile_name = match grid_kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
        GridKind::Spartan3 => "BRAM.S3",
        GridKind::FpgaCore => unreachable!(),
        GridKind::Spartan3E => "BRAM.S3E",
        GridKind::Spartan3A => "BRAM.S3A",
        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
    };
    let ctx = FuzzCtx::new(session, backend, tile_name, "BRAM", TileBits::Bram);
    let bel_kind = match grid_kind {
        GridKind::Spartan3ADsp => "RAMB16BWER",
        GridKind::Spartan3A => "RAMB16BWE",
        _ => "RAMB16",
    };
    if devdata_only {
        if !grid_kind.is_virtex2() {
            fuzz_one!(ctx, "Ibram_ddel", "!default", [
                (global_mutex_site "BRAM"),
                (mode bel_kind)
            ], [
                (global_opt "Ibram_ddel0", "0"),
                (global_opt "Ibram_ddel1", "0")
            ]);
            fuzz_one!(ctx, "Ibram_wdel", "!default", [
                (global_mutex_site "BRAM"),
                (mode bel_kind)
            ], [
                (global_opt "Ibram_wdel0", "0"),
                (global_opt "Ibram_wdel1", "0"),
                (global_opt "Ibram_wdel2", "0")
            ]);
        }
        return;
    }
    match grid_kind {
        GridKind::Spartan3A | GridKind::Spartan3ADsp => {
            fuzz_one!(ctx, "PRESENT", "1", [
                (global_mutex_none "BRAM")
            ], [
                (mode bel_kind),
                (attr "DATA_WIDTH_A", "36"),
                (attr "DATA_WIDTH_B", "36"),
                (attr "SRVAL_A", "fffffffff"),
                (attr "SRVAL_B", "fffffffff"),
                (attr "INIT_A", "fffffffff"),
                (attr "INIT_B", "fffffffff")
            ]);
            let mut invs = vec![
                ("CLKAINV", "CLKA", "CLKA_B"),
                ("CLKBINV", "CLKB", "CLKB_B"),
                ("ENAINV", "ENA", "ENA_B"),
                ("ENBINV", "ENB", "ENB_B"),
                ("WEA0INV", "WEA0", "WEA0_B"),
                ("WEA1INV", "WEA1", "WEA1_B"),
                ("WEA2INV", "WEA2", "WEA2_B"),
                ("WEA3INV", "WEA3", "WEA3_B"),
                ("WEB0INV", "WEB0", "WEB0_B"),
                ("WEB1INV", "WEB1", "WEB1_B"),
                ("WEB2INV", "WEB2", "WEB2_B"),
                ("WEB3INV", "WEB3", "WEB3_B"),
            ];
            if grid_kind == GridKind::Spartan3ADsp {
                invs.extend([
                    ("RSTAINV", "RSTA", "RSTA_B"),
                    ("RSTBINV", "RSTB", "RSTB_B"),
                    ("REGCEAINV", "REGCEA", "REGCEA_B"),
                    ("REGCEBINV", "REGCEB", "REGCEB_B"),
                ]);
            } else {
                invs.extend([("SSRAINV", "SSRA", "SSRA_B"), ("SSRBINV", "SSRB", "SSRB_B")]);
            }
            for (pininv, pin, pin_b) in invs {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36"),
                    (pin pin)
                ]);
            }
            for attr in ["DATA_WIDTH_A", "DATA_WIDTH_B"] {
                fuzz_enum!(ctx, attr, ["0", "1", "2", "4", "9", "18", "36"], [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind),
                    (attr "INIT_A", "0"),
                    (attr "INIT_B", "0"),
                    (attr "SRVAL_A", "0"),
                    (attr "SRVAL_B", "0")
                ]);
            }
            for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
                fuzz_enum!(ctx, attr, ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"], [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ]);
            }
            if grid_kind == GridKind::Spartan3ADsp {
                fuzz_enum!(ctx, "RSTTYPE", ["SYNC", "ASYNC"], [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind)
                ]);
                fuzz_enum!(ctx, "DOA_REG", ["0", "1"], [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind)
                ]);
                fuzz_enum!(ctx, "DOB_REG", ["0", "1"], [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind)
                ]);
            }
            for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
                fuzz_multi!(ctx, attr, "", 36, [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ], (attr_hex attr));
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02X}");
                fuzz_multi!(ctx, attr, "", 256, [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ], (attr_hex attr));
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02X}");
                fuzz_multi!(ctx, attr, "", 256, [
                    (global_mutex_none "BRAM"),
                    (mode bel_kind),
                    (attr "DATA_WIDTH_A", "36"),
                    (attr "DATA_WIDTH_B", "36")
                ], (attr_hex attr));
            }
        }
        _ => {
            fuzz_one!(ctx, "PRESENT", "1", [
                (global_mutex_none "BRAM")
            ], [
                (mode bel_kind),
                (attr "PORTA_ATTR", "512X36"),
                (attr "PORTB_ATTR", "512X36"),
                (attr "SRVAL_A", "fffffffff"),
                (attr "SRVAL_B", "fffffffff"),
                (attr "INIT_A", "fffffffff"),
                (attr "INIT_B", "fffffffff")
            ]);
            for (pininv, pin, pin_b) in [
                ("CLKAINV", "CLKA", "CLKA_B"),
                ("CLKBINV", "CLKB", "CLKB_B"),
                ("SSRAINV", "SSRA", "SSRA_B"),
                ("SSRBINV", "SSRB", "SSRB_B"),
                ("WEAINV", "WEA", "WEA_B"),
                ("WEBINV", "WEB", "WEB_B"),
                ("ENAINV", "ENA", "ENA_B"),
                ("ENBINV", "ENB", "ENB_B"),
            ] {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36"),
                    (pin pin)
                ]);
            }
            for attr in ["PORTA_ATTR", "PORTB_ATTR"] {
                fuzz_enum!(ctx, attr, ["16384X1", "8192X2", "4096X4", "2048X9", "1024X18", "512X36"], [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "INIT_A", "0"),
                    (attr "INIT_B", "0"),
                    (attr "SRVAL_A", "0"),
                    (attr "SRVAL_B", "0")
                ]);
            }
            for attr in ["WRITEMODEA", "WRITEMODEB"] {
                fuzz_enum!(ctx, attr, ["NO_CHANGE", "READ_FIRST", "WRITE_FIRST"], [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ]);
            }
            if grid_kind.is_virtex2() {
                fuzz_enum!(ctx, "SAVEDATA", ["FALSE", "TRUE"], [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ]);
            }
            for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
                fuzz_multi!(ctx, attr, "", 36, [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ], (attr_hex attr));
            }
            for i in 0..0x40 {
                let attr = format!("INIT_{i:02x}");
                fuzz_multi!(ctx, attr, "", 256, [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ], (attr_hex attr));
            }
            for i in 0..0x8 {
                let attr = format!("INITP_{i:02x}");
                fuzz_multi!(ctx, attr, "", 256, [
                    (global_mutex_none "BRAM"),
                    (mode "RAMB16"),
                    (attr "PORTA_ATTR", "512X36"),
                    (attr "PORTB_ATTR", "512X36")
                ], (attr_hex attr));
            }
        }
    }
    if !grid_kind.is_virtex2() {
        for opt in [
            "Ibram_ddel0",
            "Ibram_ddel1",
            "Ibram_wdel0",
            "Ibram_wdel1",
            "Ibram_wdel2",
        ] {
            fuzz_one!(ctx, opt, "1", [
                (global_mutex_site "BRAM"),
                (mode bel_kind)
            ], [(global_opt_diff opt, "0", "1")]);
        }
        fuzz_one!(ctx, "Ibram_ddel", "!default", [
            (global_mutex_site "BRAM"),
            (mode bel_kind)
        ], [
            (global_opt "Ibram_ddel0", "0"),
            (global_opt "Ibram_ddel1", "0")
        ]);
        fuzz_one!(ctx, "Ibram_wdel", "!default", [
            (global_mutex_site "BRAM"),
            (mode bel_kind)
        ], [
            (global_opt "Ibram_wdel0", "0"),
            (global_opt "Ibram_wdel1", "0"),
            (global_opt "Ibram_wdel2", "0")
        ]);
        for val in ["0", "1"] {
            fuzz_one!(ctx, "Ibram_ww_value", val, [
                (global_mutex_site "BRAM"),
                (mode bel_kind)
            ], [(global_opt "Ibram_ww_value", val)]);
        }
    }
    if grid_kind != GridKind::Spartan3ADsp {
        // mult
        let ctx = FuzzCtx::new(session, backend, tile_name, "MULT", TileBits::MainAuto);
        let bel_kind = if matches!(grid_kind, GridKind::Spartan3E | GridKind::Spartan3A) {
            "MULT18X18SIO"
        } else {
            "MULT18X18"
        };
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode bel_kind)]);
        if !matches!(grid_kind, GridKind::Spartan3E | GridKind::Spartan3A) {
            for (pininv, pin, pin_b) in [
                ("CLKINV", "CLK", "CLK_B"),
                ("RSTINV", "RST", "RST_B"),
                ("CEINV", "CE", "CE_B"),
            ] {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (mode "MULT18X18"),
                    (pin pin)
                ]);
            }
        } else {
            for (pininv, pin, pin_b) in [
                ("CLKINV", "CLK", "CLK_B"),
                ("RSTAINV", "RSTA", "RSTA_B"),
                ("RSTBINV", "RSTB", "RSTB_B"),
                ("RSTPINV", "RSTP", "RSTP_B"),
                ("CEAINV", "CEA", "CEA_B"),
                ("CEBINV", "CEB", "CEB_B"),
                ("CEPINV", "CEP", "CEP_B"),
            ] {
                fuzz_enum!(ctx, pininv, [pin, pin_b], [
                    (mode "MULT18X18SIO"),
                    (pin pin)
                ]);
            }
            fuzz_enum!(ctx, "AREG", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "BREG", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "PREG", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "PREG_CLKINVERSION", ["0", "1"], [
                (mode "MULT18X18SIO")
            ]);
            fuzz_enum!(ctx, "B_INPUT", ["DIRECT", "CASCADE"], [
                (mode "MULT18X18SIO")
            ]);
            if grid_kind == GridKind::Spartan3A {
                let bel_bram = BelId::from_idx(0);
                for ab in ['A', 'B'] {
                    for i in 0..18 {
                        let name = format!("MUX.{ab}{i}");
                        let bram_pin = if i < 16 {
                            format!("DO{ab}{i}")
                        } else {
                            format!("DOP{ab}{ii}", ii = i - 16)
                        };
                        let mult_pin = format!("{ab}{i}");
                        fuzz_one!(ctx, name, "BRAM", [], [
                            (pip (bel_pin bel_bram, bram_pin), (pin mult_pin))
                        ]);
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let grid_kind = match ctx.edev {
        ExpandedDevice::Virtex2(ref edev) => edev.grid.kind,
        _ => unreachable!(),
    };
    let int_tiles = match grid_kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => &["INT.BRAM"; 4],
        GridKind::Spartan3 => &["INT.BRAM.S3"; 4],
        GridKind::FpgaCore => unreachable!(),
        GridKind::Spartan3E => &["INT.BRAM.S3E"; 4],
        GridKind::Spartan3A => &[
            "INT.BRAM.S3A.03",
            "INT.BRAM.S3A.12",
            "INT.BRAM.S3A.12",
            "INT.BRAM.S3A.03",
        ],
        GridKind::Spartan3ADsp => &["INT.BRAM.S3ADSP"; 4],
    };
    let tile = match grid_kind {
        GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "BRAM",
        GridKind::Spartan3 => "BRAM.S3",
        GridKind::FpgaCore => unreachable!(),
        GridKind::Spartan3E => "BRAM.S3E",
        GridKind::Spartan3A => "BRAM.S3A",
        GridKind::Spartan3ADsp => "BRAM.S3ADSP",
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
            let (adef, bdef) =
                filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_ddel", "!default"));
            let adef = extract_bitvec_val(
                ctx.tiledb.item(tile, "BRAM", "DDEL_A"),
                &bitvec![0, 0],
                !adef,
            );
            ctx.insert_device_data("BRAM:DDEL_A_DEFAULT", adef);
            if grid_kind != GridKind::Spartan3 {
                let bdef = extract_bitvec_val(
                    ctx.tiledb.item(tile, "BRAM", "DDEL_B"),
                    &bitvec![0, 0],
                    !bdef,
                );
                ctx.insert_device_data("BRAM:DDEL_B_DEFAULT", bdef);
            }

            let (adef, bdef) =
                filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_wdel", "!default"));
            let adef = extract_bitvec_val(
                ctx.tiledb.item(tile, "BRAM", "WDEL_A"),
                &bitvec![0, 0, 0],
                !adef,
            );
            ctx.insert_device_data("BRAM:WDEL_A_DEFAULT", adef);
            if grid_kind != GridKind::Spartan3 {
                let bdef = extract_bitvec_val(
                    ctx.tiledb.item(tile, "BRAM", "WDEL_B"),
                    &bitvec![0, 0, 0],
                    !bdef,
                );
                ctx.insert_device_data("BRAM:WDEL_B_DEFAULT", bdef);
            }
        }
        return;
    }
    let mut present = ctx.state.get_diff(tile, "BRAM", "PRESENT", "1");
    let mut diffs_data = vec![];
    let mut diffs_datap = vec![];
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "CLKA", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "CLKB", false);
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "ENA", grid_kind.is_virtex2());
    ctx.collect_int_inv(int_tiles, tile, "BRAM", "ENB", grid_kind.is_virtex2());
    present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", "ENA"));
    present.discard_bits(&ctx.item_int_inv(int_tiles, tile, "BRAM", "ENB"));
    match grid_kind {
        GridKind::Spartan3A | GridKind::Spartan3ADsp => {
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
            if grid_kind == GridKind::Spartan3ADsp {
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
    if !grid_kind.is_virtex2() {
        let (a0, b0) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_ddel0", "1"));
        let (a1, b1) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_ddel1", "1"));
        let (adef, bdef) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_ddel", "!default"));
        let ddel_a = xlat_bitvec(vec![a0, a1]);
        let adef = extract_bitvec_val(&ddel_a, &bitvec![0, 0], !adef);
        ctx.tiledb.insert(tile, "BRAM", "DDEL_A", ddel_a);
        ctx.insert_device_data("BRAM:DDEL_A_DEFAULT", adef);
        present.discard_bits(ctx.tiledb.item(tile, "BRAM", "DDEL_A"));
        if grid_kind == GridKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
        } else {
            let ddel_b = xlat_bitvec(vec![b0, b1]);
            let bdef = extract_bitvec_val(&ddel_b, &bitvec![0, 0], !bdef);
            ctx.tiledb.insert(tile, "BRAM", "DDEL_B", ddel_b);
            ctx.insert_device_data("BRAM:DDEL_B_DEFAULT", bdef);
            present.discard_bits(ctx.tiledb.item(tile, "BRAM", "DDEL_B"));
        }

        let (a0, b0) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_wdel0", "1"));
        let (a1, b1) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_wdel1", "1"));
        let (a2, b2) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_wdel2", "1"));
        let (adef, bdef) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_wdel", "!default"));
        let wdel_a = xlat_bitvec(vec![a0, a1, a2]);
        let adef = extract_bitvec_val(&wdel_a, &bitvec![0, 0, 0], !adef);
        ctx.insert_device_data("BRAM:WDEL_A_DEFAULT", adef);
        ctx.tiledb.insert(tile, "BRAM", "WDEL_A", wdel_a);
        present.discard_bits(ctx.tiledb.item(tile, "BRAM", "WDEL_A"));
        if grid_kind == GridKind::Spartan3 {
            b0.assert_empty();
            b1.assert_empty();
            b2.assert_empty();
        } else {
            let wdel_b = xlat_bitvec(vec![b0, b1, b2]);
            let bdef = extract_bitvec_val(&wdel_b, &bitvec![0, 0, 0], !bdef);
            ctx.tiledb.insert(tile, "BRAM", "WDEL_B", wdel_b);
            ctx.insert_device_data("BRAM:WDEL_B_DEFAULT", bdef);
            present.discard_bits(ctx.tiledb.item(tile, "BRAM", "WDEL_B"));
        }

        let (a0, b0) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_ww_value", "0"));
        let (a1, b1) = filter_ab(ctx.state.get_diff(tile, "BRAM", "Ibram_ww_value", "1"));
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

    if grid_kind != GridKind::Spartan3ADsp {
        let mut present = ctx.state.get_diff(tile, "MULT", "PRESENT", "1");
        if grid_kind.is_virtex2() || grid_kind == GridKind::Spartan3 {
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
            if grid_kind == GridKind::Spartan3A {
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
