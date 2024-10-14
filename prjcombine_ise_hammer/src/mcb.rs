use prjcombine_hammer::Session;
use prjcombine_types::TileItem;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{xlat_bool, xlat_enum, CollectorCtx, Diff},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec,
    fuzz_multi_attr_hex, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let Some(ctx) = FuzzCtx::try_new(session, backend, "MCB", "MCB", TileBits::MainAuto) else {
        return;
    };
    fuzz_one!(ctx, "PRESENT", "1", [
        (global_mutex "MCB", "TEST")
    ], [
        (mode "MCB")
    ]);
    for pin in [
        "P0CMDCLK", "P1CMDCLK", "P2CMDCLK", "P3CMDCLK", "P4CMDCLK", "P5CMDCLK", "P0CMDEN",
        "P1CMDEN", "P2CMDEN", "P3CMDEN", "P4CMDEN", "P5CMDEN", "P0RDCLK", "P1RDCLK", "P0RDEN",
        "P1RDEN", "P0WRCLK", "P1WRCLK", "P0WREN", "P1WREN", "P2CLK", "P3CLK", "P4CLK", "P5CLK",
        "P2EN", "P3EN", "P4EN", "P5EN",
    ] {
        fuzz_inv!(ctx, pin, [
            (global_mutex "MCB", "TEST"),
            (mode "MCB")
        ]);
    }
    fuzz_enum!(ctx, "ARB_NUM_TIME_SLOTS", ["10", "12"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "CAL_BYPASS", ["YES", "NO"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "CAL_CALIBRATION_MODE", ["CALIBRATION", "NOCALIBRATION"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "CAL_CLK_DIV", ["1", "2", "4", "8"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "CAL_DELAY", ["QUARTER", "HALF", "THREEQUARTER", "FULL"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "MEM_ADDR_ORDER", ["BANK_ROW_COLUMN", "ROW_BANK_COLUMN"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "MEM_BA_SIZE", ["2", "3"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "MEM_CA_SIZE", ["9", "10", "11", "12"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "MEM_RA_SIZE", ["12", "13", "14", "15"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "MEM_TYPE", ["DDR", "DDR2", "DDR3", "MDDR"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "MEM_WIDTH", ["4", "8", "16"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_enum!(ctx, "PORT_CONFIG", [
            "B32_B32_B32_B32",
            "B32_B32_R32_R32_R32_R32",
            "B32_B32_R32_R32_R32_W32",
            "B32_B32_R32_R32_W32_R32",
            "B32_B32_R32_R32_W32_W32",
            "B32_B32_R32_W32_R32_R32",
            "B32_B32_R32_W32_R32_W32",
            "B32_B32_R32_W32_W32_R32",
            "B32_B32_R32_W32_W32_W32",
            "B32_B32_W32_R32_R32_R32",
            "B32_B32_W32_R32_R32_W32",
            "B32_B32_W32_R32_W32_R32",
            "B32_B32_W32_R32_W32_W32",
            "B32_B32_W32_W32_R32_R32",
            "B32_B32_W32_W32_R32_W32",
            "B32_B32_W32_W32_W32_R32",
            "B32_B32_W32_W32_W32_W32",
            "B64_B32_B32",
            "B64_B64",
            "B128",
        ], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_RCD_VAL", 3, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_RAS_VAL", 5, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_RTP_VAL", 3, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_WR_VAL", 3, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_WTR_VAL", 3, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_RFC_VAL", 8, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_RP_VAL", 4, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_dec!(ctx, "MEM_REFI_VAL", 12, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_hex!(ctx, "CAL_BA", 3, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_hex!(ctx, "CAL_CA", 12, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    fuzz_multi_attr_hex!(ctx, "CAL_RA", 15, [
        (global_mutex "MCB", "TEST"),
        (mode "MCB")
    ]);
    for i in 0..12 {
        fuzz_multi_attr_bin!(ctx, format!("ARB_TIME_SLOT_{i}"), 18, [
            (global_mutex "MCB", "TEST"),
            (mode "MCB")
        ]);
    }

    for mem_type in ["MDDR", "DDR", "DDR2", "DDR3"] {
        fuzz_enum_suffix!(ctx, "MEM_BURST_LEN",  mem_type, ["4", "8"], [
            (global_mutex "MCB", "TEST"),
            (mode "MCB"),
            (attr "MEM_TYPE", mem_type)
        ]);
    }

    fuzz_enum!(ctx, "MEM_CAS_LATENCY", ["1", "2", "3", "4", "5", "6"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr_any "MEM_TYPE", ["DDR", "DDR2", "MDDR"])
    ]);
    fuzz_enum!(ctx, "MEM_DDR1_2_ODS", ["REDUCED", "FULL"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        // sigh. doesn't actually work for plain DDR.
        (attr "MEM_TYPE", "DDR2")
    ]);
    fuzz_enum!(ctx, "MEM_DDR2_ADD_LATENCY", ["0", "1", "2", "3", "4", "5"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR2")
    ]);
    fuzz_enum!(ctx, "MEM_DDR2_DIFF_DQS_EN", ["YES", "NO"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR2")
    ]);
    fuzz_enum!(ctx, "MEM_DDR2_RTT", ["50OHMS", "75OHMS", "150OHMS"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR2")
    ]);
    fuzz_enum!(ctx, "MEM_DDR2_WRT_RECOVERY", ["2", "3", "4", "5", "6"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR2")
    ]);
    fuzz_enum!(ctx, "MEM_DDR2_3_HIGH_TEMP_SR", ["NORMAL", "EXTENDED"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr_any "MEM_TYPE", ["DDR2", "DDR3"])
    ]);
    fuzz_enum!(ctx, "MEM_DDR2_3_PA_SR", [
        "FULL", "EIGHTH1", "EIGHTH2", "HALF1", "HALF2", "QUARTER1", "QUARTER2", "THREEQUARTER"
    ], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr_any "MEM_TYPE", ["DDR2", "DDR3"])
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_ADD_LATENCY", ["CL1", "CL2"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_AUTO_SR", ["ENABLED", "MANUAL"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_CAS_LATENCY", ["5", "6", "7", "8", "9", "10"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_CAS_WR_LATENCY", ["5", "6", "7", "8"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_DYN_WRT_ODT", ["DIV2", "DIV4"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_ODS", ["DIV6", "DIV7"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_RTT", ["DIV2", "DIV4", "DIV6", "DIV8", "DIV12"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_DDR3_WRT_RECOVERY", ["5", "6", "7", "8", "10", "12"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "DDR3")
    ]);
    fuzz_enum!(ctx, "MEM_MDDR_ODS", ["QUARTER", "HALF", "THREEQUARTERS", "FULL"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "MDDR")
    ]);
    fuzz_enum!(ctx, "MEM_MOBILE_PA_SR", ["HALF", "FULL"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "MDDR")
    ]);
    fuzz_enum!(ctx, "MEM_MOBILE_TC_SR", ["0", "1", "2", "3"], [
        (global_mutex "MCB", "TEST"),
        (mode "MCB"),
        (attr "MEM_TYPE", "MDDR")
    ]);

    for val in ["DISABLED", "ENABLED"] {
        fuzz_one!(ctx, "MEM_PLL_DIV_EN", val, [
            (global_mutex_site "MCB"),
            (global_mutex "DRPSDO", "NOPE"),
            (mode "MCB")
        ], [
            (global_opt "MEM_PLL_DIV_EN", val)
        ]);
    }
    for val in ["INVERTED", "NOTINVERTED"] {
        fuzz_one!(ctx, "MEM_PLL_POL_SEL", val, [
            (global_mutex_site "MCB"),
            (global_mutex "DRPSDO", "NOPE"),
            (mode "MCB")
        ], [
            (global_opt "MEM_PLL_POL_SEL", val)
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    if !ctx.has_tile("MCB") {
        return;
    }

    let tile = "MCB";
    let bel = "MCB";

    let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
    present = present.combine(ctx.state.peek_diff(tile, bel, "MEM_PLL_DIV_EN", "DISABLED"));
    present = present.combine(
        ctx.state
            .peek_diff(tile, bel, "MEM_PLL_POL_SEL", "INVERTED"),
    );

    for pin in [
        "P0CMDCLK", "P1CMDCLK", "P2CMDCLK", "P3CMDCLK", "P4CMDCLK", "P5CMDCLK", "P0CMDEN",
        "P1CMDEN", "P2CMDEN", "P3CMDEN", "P4CMDEN", "P5CMDEN", "P0RDCLK", "P1RDCLK", "P0RDEN",
        "P1RDEN", "P0WRCLK", "P1WRCLK", "P0WREN", "P1WREN", "P2CLK", "P3CLK", "P4CLK", "P5CLK",
        "P2EN", "P3EN", "P4EN", "P5EN",
    ] {
        ctx.collect_inv(tile, bel, pin);
    }
    ctx.collect_enum(tile, bel, "ARB_NUM_TIME_SLOTS", &["10", "12"]);
    ctx.collect_enum_bool(tile, bel, "CAL_BYPASS", "NO", "YES");
    ctx.collect_enum(
        tile,
        bel,
        "CAL_CALIBRATION_MODE",
        &["CALIBRATION", "NOCALIBRATION"],
    );
    ctx.collect_enum(tile, bel, "CAL_CLK_DIV", &["1", "2", "4", "8"]);
    ctx.collect_enum(
        tile,
        bel,
        "CAL_DELAY",
        &["QUARTER", "HALF", "THREEQUARTER", "FULL"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "MEM_ADDR_ORDER",
        &["BANK_ROW_COLUMN", "ROW_BANK_COLUMN"],
    );
    ctx.collect_enum(tile, bel, "MEM_BA_SIZE", &["2", "3"]);
    ctx.collect_enum(tile, bel, "MEM_CA_SIZE", &["9", "10", "11", "12"]);
    ctx.collect_enum(tile, bel, "MEM_RA_SIZE", &["12", "13", "14", "15"]);
    ctx.collect_enum(tile, bel, "MEM_TYPE", &["DDR", "DDR2", "DDR3", "MDDR"]);
    for (attr, vals) in [
        ("MEM_WIDTH", &["4", "8", "16"][..]),
        ("MEM_PLL_POL_SEL", &["INVERTED", "NOTINVERTED"]),
        ("MEM_PLL_DIV_EN", &["DISABLED", "ENABLED"]),
    ] {
        let mut diffs: [Vec<_>; 9] = Default::default();
        for &val in vals {
            let mut diff = ctx.state.get_diff(tile, bel, attr, val);
            for i in 0..8 {
                diffs[i + 1].push((val, diff.split_bits_by(|bit| bit.tile == 13 + i * 2)));
            }
            diffs[0].push((val, diff));
        }
        let items = diffs.map(|mut diffs| {
            if attr == "MEM_PLL_DIV_EN" {
                xlat_bool(
                    core::mem::take(&mut diffs[0].1),
                    core::mem::take(&mut diffs[1].1),
                )
            } else {
                xlat_enum(diffs)
            }
        });
        for (i, item) in items.into_iter().enumerate() {
            let name = match i {
                0 => attr.to_string(),
                1 => format!("MUI0R.{attr}"),
                2 => format!("MUI0W.{attr}"),
                3 => format!("MUI1R.{attr}"),
                4 => format!("MUI1W.{attr}"),
                _ => format!("MUI{ii}.{attr}", ii = i - 5),
            };
            ctx.tiledb.insert(tile, bel, name, item);
        }
    }
    ctx.state
        .peek_diff(tile, bel, "PORT_CONFIG", "B32_B32_W32_W32_W32_W32")
        .assert_empty();
    for (attr, val) in [
        ("MUI2_PORT_CONFIG", "B32_B32_R32_W32_W32_W32"),
        ("MUI3_PORT_CONFIG", "B32_B32_W32_R32_W32_W32"),
        ("MUI4_PORT_CONFIG", "B32_B32_W32_W32_R32_W32"),
        ("MUI5_PORT_CONFIG", "B32_B32_W32_W32_W32_R32"),
    ] {
        let diff = ctx.state.peek_diff(tile, bel, "PORT_CONFIG", val).clone();
        ctx.tiledb.insert(
            tile,
            bel,
            attr,
            xlat_enum(vec![("WRITE", Diff::default()), ("READ", diff)]),
        );
    }
    let mut diffs = vec![("B32_B32_X32_X32_X32_X32", Diff::default())];
    for val in ["B32_B32_B32_B32", "B64_B32_B32", "B64_B64", "B128"] {
        let mut diff = ctx.state.get_diff(tile, bel, "PORT_CONFIG", val);
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "MUI2_PORT_CONFIG"),
            "READ",
            "WRITE",
        );
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "MUI4_PORT_CONFIG"),
            "READ",
            "WRITE",
        );
        diffs.push((val, diff));
    }
    ctx.tiledb
        .insert(tile, bel, "PORT_CONFIG", xlat_enum(diffs));
    for mask in 0..16 {
        let val = format!(
            "B32_B32_{p2}32_{p3}32_{p4}32_{p5}32",
            p2 = if (mask & 1) != 0 { 'R' } else { 'W' },
            p3 = if (mask & 2) != 0 { 'R' } else { 'W' },
            p4 = if (mask & 4) != 0 { 'R' } else { 'W' },
            p5 = if (mask & 8) != 0 { 'R' } else { 'W' },
        );
        let mut diff = ctx.state.get_diff(tile, bel, "PORT_CONFIG", val);
        for i in 0..4 {
            if (mask & (1 << i)) != 0 {
                diff.apply_enum_diff(
                    ctx.tiledb
                        .item(tile, bel, &format!("MUI{ii}_PORT_CONFIG", ii = i + 2)),
                    "READ",
                    "WRITE",
                );
            }
        }
        diff.assert_empty();
    }
    for (i, mui) in ["MUI0R", "MUI0W", "MUI1R", "MUI1W"].into_iter().enumerate() {
        let mut item = ctx.tiledb.item(tile, bel, "MUI2_PORT_CONFIG").clone();
        for bit in &mut item.bits {
            bit.tile -= 4 * 2;
            bit.tile += i * 2;
        }
        ctx.tiledb
            .insert(tile, bel, format!("{mui}_PORT_CONFIG"), item);
    }
    present.apply_enum_diff(
        ctx.tiledb.item(tile, bel, "MUI0R_PORT_CONFIG"),
        "READ",
        "WRITE",
    );
    present.apply_enum_diff(
        ctx.tiledb.item(tile, bel, "MUI1R_PORT_CONFIG"),
        "READ",
        "WRITE",
    );

    present.assert_empty();

    ctx.collect_bitvec(tile, bel, "MEM_RCD_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_RAS_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_RTP_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_WR_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_WTR_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_RFC_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_RP_VAL", "");
    ctx.collect_bitvec(tile, bel, "MEM_REFI_VAL", "");
    ctx.collect_bitvec(tile, bel, "CAL_BA", "");
    ctx.collect_bitvec(tile, bel, "CAL_CA", "");
    ctx.collect_bitvec(tile, bel, "CAL_RA", "");
    for i in 0..12 {
        ctx.collect_bitvec(tile, bel, &format!("ARB_TIME_SLOT_{i}"), "");
    }

    for mem_type in ["MDDR", "DDR", "DDR2"] {
        let mut diffs = vec![];
        for val in ["4", "8"] {
            let mut diff = ctx
                .state
                .get_diff(tile, bel, format!("MEM_BURST_LEN.{mem_type}"), val);
            diff = diff.combine(&!ctx.state.peek_diff(tile, bel, "MEM_BURST_LEN.DDR3", val));
            diffs.push((val, diff));
        }
        ctx.tiledb
            .insert(tile, bel, "MEM_DDR_DDR2_MDDR_BURST_LEN", xlat_enum(diffs));
    }
    let item = ctx.extract_enum(tile, bel, "MEM_BURST_LEN.DDR3", &["4", "8"]);
    ctx.tiledb.insert(tile, bel, "MEM_BURST_LEN", item);

    ctx.collect_enum(
        tile,
        bel,
        "MEM_CAS_LATENCY",
        &["1", "2", "3", "4", "5", "6"],
    );
    ctx.collect_enum(tile, bel, "MEM_DDR1_2_ODS", &["REDUCED", "FULL"]);
    ctx.collect_enum(
        tile,
        bel,
        "MEM_DDR2_ADD_LATENCY",
        &["0", "1", "2", "3", "4", "5"],
    );
    ctx.collect_enum(tile, bel, "MEM_DDR2_DIFF_DQS_EN", &["YES", "NO"]);
    ctx.collect_enum_default(
        tile,
        bel,
        "MEM_DDR2_RTT",
        &["50OHMS", "75OHMS", "150OHMS"],
        "NONE",
    );
    ctx.collect_enum(
        tile,
        bel,
        "MEM_DDR2_WRT_RECOVERY",
        &["2", "3", "4", "5", "6"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "MEM_DDR2_3_HIGH_TEMP_SR",
        &["NORMAL", "EXTENDED"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "MEM_DDR2_3_PA_SR",
        &[
            "FULL",
            "EIGHTH1",
            "EIGHTH2",
            "HALF1",
            "HALF2",
            "QUARTER1",
            "QUARTER2",
            "THREEQUARTER",
        ],
    );
    ctx.collect_enum_default(tile, bel, "MEM_DDR3_ADD_LATENCY", &["CL1", "CL2"], "NONE");
    ctx.collect_enum(tile, bel, "MEM_DDR3_AUTO_SR", &["ENABLED", "MANUAL"]);
    ctx.collect_enum(
        tile,
        bel,
        "MEM_DDR3_CAS_LATENCY",
        &["5", "6", "7", "8", "9", "10"],
    );
    ctx.collect_enum(tile, bel, "MEM_DDR3_CAS_WR_LATENCY", &["5", "6", "7", "8"]);
    ctx.collect_enum_default(tile, bel, "MEM_DDR3_DYN_WRT_ODT", &["DIV2", "DIV4"], "NONE");
    ctx.collect_enum(tile, bel, "MEM_DDR3_ODS", &["DIV6", "DIV7"]);
    ctx.collect_enum_default(
        tile,
        bel,
        "MEM_DDR3_RTT",
        &["DIV2", "DIV4", "DIV6", "DIV8", "DIV12"],
        "NONE",
    );
    ctx.collect_enum(
        tile,
        bel,
        "MEM_DDR3_WRT_RECOVERY",
        &["5", "6", "7", "8", "10", "12"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "MEM_MDDR_ODS",
        &["QUARTER", "HALF", "THREEQUARTERS", "FULL"],
    );
    ctx.collect_enum(tile, bel, "MEM_MOBILE_PA_SR", &["HALF", "FULL"]);
    ctx.collect_enum(tile, bel, "MEM_MOBILE_TC_SR", &["0", "1", "2", "3"]);

    for (reg, bittile) in [("MR", 7), ("EMR1", 6), ("EMR2", 5), ("EMR3", 4)] {
        ctx.tiledb.insert(
            tile,
            bel,
            reg,
            TileItem::from_bitvec(
                (0..14).map(|i| FeatureBit::new(bittile, 22, 18 + i)).collect(),
                false,
            ),
        );
    }
}
