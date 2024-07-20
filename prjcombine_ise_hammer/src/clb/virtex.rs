use prjcombine_hammer::Session;
use prjcombine_types::TileItem;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{xlat_bitvec, xlat_bool, xlat_enum, CollectorCtx},
    fgen::TileBits,
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "CLB",
            format!("SLICE{i}"),
            TileBits::MainAuto,
        );

        // inverters
        fuzz_enum!(ctx, "CKINV", ["0", "1"], [
            (mode "SLICE"),
            (attr "FFX", "#FF"),
            (pin "CLK")
        ]);
        fuzz_enum!(ctx, "CEMUX", ["0", "1", "CE", "CE_B"], [
            (mode "SLICE"),
            (attr "FFX", "#FF"),
            (attr "CKINV", "1"),
            (pin "CE"),
            (pin "CLK"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "SRMUX", ["0", "1", "SR", "SR_B"], [
            (mode "SLICE"),
            (attr "F", "#LUT:0"),
            (attr "DXMUX", "1"),
            (attr "SRFFMUX", "0"),
            (attr "CEMUX", "0"),
            (attr "FFX", "#FF"),
            (attr "FFY", "#FF"),
            (attr "CKINV", "1"),
            (pin "SR"),
            (pin "CLK"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "BXMUX", ["0", "1", "BX", "BX_B"], [
            (mode "SLICE"),
            (attr "FFX", "#FF"),
            (attr "DXMUX", "0"),
            (pin "BX"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "BYMUX", ["0", "1", "BY", "BY_B"], [
            (mode "SLICE"),
            (attr "FFY", "#FF"),
            (attr "DYMUX", "0"),
            (pin "BY"),
            (pin "YQ")
        ]);

        // LUT
        for attr in ["F", "G"] {
            fuzz_multi!(ctx, attr, "#LUT", 16, [(mode "SLICE")], (attr_lut attr));
        }
        fuzz_enum!(ctx, "RAMCONFIG", ["16X1", "16X1DP", "16X2", "32X1", "1SHIFT", "2SHIFTS"], [
            (mode "SLICE")
        ]);

        // carry chain
        fuzz_enum!(ctx, "CYINIT", ["CIN", "BX"], [
            (mode "SLICE"),
            (attr "BXMUX", "BX"),
            (attr "CYSELF", "1"),
            (attr "CYSELG", "1"),
            (attr "COUTUSED", "0"),
            (pin "CIN"),
            (pin "BX"),
            (pin "COUT")
        ]);
        fuzz_enum!(ctx, "CYSELF", ["F", "1"], [
            (mode "SLICE"),
            (attr "F", "#LUT:0"),
            (attr "CY0F", "0"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "CYSELG", "1"),
            (attr "COUTUSED", "0"),
            (pin "BX"),
            (pin "COUT")
        ]);
        fuzz_enum!(ctx, "CYSELG", ["G", "1"], [
            (mode "SLICE"),
            (attr "G", "#LUT:0"),
            (attr "CY0G", "0"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "CYSELF", "1"),
            (attr "COUTUSED", "0"),
            (pin "BX"),
            (pin "COUT")
        ]);
        fuzz_enum!(ctx, "CY0F", ["0", "1", "F1", "PROD"], [
            (mode "SLICE"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "FXMUX", "FXOR"),
            (attr "F", "#LUT:0"),
            (attr "XUSED", "0"),
            (attr "CYSELF", "F"),
            (attr "CYSELG", "1"),
            (attr "COUTUSED", "0"),
            (pin "F1"),
            (pin "F2"),
            (pin "BX"),
            (pin "X"),
            (pin "COUT")
        ]);
        fuzz_enum!(ctx, "CY0G", ["0", "1", "G1", "PROD"], [
            (mode "SLICE"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "BYMUX", "BY"),
            (attr "GYMUX", "GXOR"),
            (attr "G", "#LUT:0"),
            (attr "YUSED", "0"),
            (attr "CYSELF", "1"),
            (attr "CYSELG", "G"),
            (attr "COUTUSED", "0"),
            (pin "G1"),
            (pin "G2"),
            (pin "BX"),
            (pin "BY"),
            (pin "Y"),
            (pin "COUT")
        ]);

        // muxes
        fuzz_enum!(ctx, "YBMUX", ["0", "1"], [
            (mode "SLICE"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "BYMUX", "BY"),
            (attr "GYMUX", "GXOR"),
            (attr "G", "#LUT:0"),
            (attr "YUSED", "0"),
            (attr "CYSELF", "1"),
            (attr "CYSELG", "1"),
            (attr "COUTUSED", "0"),
            (pin "BX"),
            (pin "BY"),
            (pin "Y"),
            (pin "YB"),
            (pin "COUT")
        ]);
        fuzz_enum!(ctx, "DXMUX", ["0", "1"], [
            (mode "SLICE"),
            (attr "F", "#LUT:0"),
            (attr "XUSED", "0"),
            (attr "FXMUX", "F"),
            (attr "FFX", "#FF"),
            (attr "BXMUX", "BX"),
            (pin "X"),
            (pin "XQ"),
            (pin "BX")
        ]);
        fuzz_enum!(ctx, "DYMUX", ["0", "1"], [
            (mode "SLICE"),
            (attr "G", "#LUT:0"),
            (attr "YUSED", "0"),
            (attr "GYMUX", "G"),
            (attr "FFY", "#FF"),
            (attr "BYMUX", "BY"),
            (pin "Y"),
            (pin "YQ"),
            (pin "BY")
        ]);
        fuzz_enum!(ctx, "FXMUX", ["F", "F5", "FXOR"], [
            (mode "SLICE"),
            (attr "F", "#LUT:0"),
            (attr "CYSELF", "1"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "XUSED", "0"),
            (attr "COUTUSED", "0"),
            (pin "X"),
            (pin "BX"),
            (pin "COUT")
        ]);
        fuzz_enum!(ctx, "GYMUX", ["G", "F6", "GXOR"], [
            (mode "SLICE"),
            (attr "G", "#LUT:0"),
            (attr "CYSELF", "1"),
            (attr "CYSELG", "1"),
            (attr "CYINIT", "BX"),
            (attr "BXMUX", "BX"),
            (attr "YUSED", "0"),
            (attr "COUTUSED", "0"),
            (pin "Y"),
            (pin "BX"),
            (pin "COUT")
        ]);

        // FFs
        fuzz_enum!(ctx, "SYNC_ATTR", ["SYNC", "ASYNC"], [
            (mode "SLICE"),
            (pin "XQ"),
            (attr "FFX", "#FF")
        ]);
        fuzz_enum!(ctx, "FFX", ["#FF", "#LATCH"], [
            (mode "SLICE"),
            (attr "FFY", ""),
            (attr "CEMUX", "CE_B"),
            (attr "INITX", "LOW"),
            (pin "XQ"),
            (pin "CE")
        ]);
        fuzz_enum!(ctx, "FFY", ["#FF", "#LATCH"], [
            (mode "SLICE"),
            (attr "FFX", ""),
            (attr "CEMUX", "CE_B"),
            (attr "INITY", "LOW"),
            (pin "YQ"),
            (pin "CE")
        ]);
        fuzz_enum!(ctx, "INITX", ["LOW", "HIGH"], [
            (mode "SLICE"),
            (attr "FFX", "#FF"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "INITY", ["LOW", "HIGH"], [
            (mode "SLICE"),
            (attr "FFY", "#FF"),
            (pin "YQ")
        ]);
        fuzz_enum!(ctx, "REVUSED", ["0"], [
            (mode "SLICE"),
            (attr "FFX", "#FF"),
            (attr "BYMUX", "BY"),
            (pin "XQ"),
            (pin "BY")
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CLB";
    for bel in ["SLICE0", "SLICE1"] {
        let item = ctx.extract_enum_bool(tile, bel, "CKINV", "1", "0");
        ctx.insert_int_inv(&[tile], tile, bel, "CLK", item);
        for (pinmux, pin, pin_b) in [
            ("BXMUX", "BX", "BX_B"),
            ("BYMUX", "BY", "BY_B"),
            ("CEMUX", "CE", "CE_B"),
            ("SRMUX", "SR", "SR_B"),
        ] {
            let d0 = ctx.state.get_diff(tile, bel, pinmux, pin);
            assert_eq!(d0, ctx.state.get_diff(tile, bel, pinmux, "1"));
            let d1 = ctx.state.get_diff(tile, bel, pinmux, pin_b);
            assert_eq!(d1, ctx.state.get_diff(tile, bel, pinmux, "0"));
            ctx.insert_int_inv(&[tile], tile, bel, pin, xlat_bool(d0, d1));
        }

        ctx.collect_bitvec(tile, bel, "F", "#LUT");
        ctx.collect_bitvec(tile, bel, "G", "#LUT");
        ctx.collect_enum_default(
            tile,
            bel,
            "RAMCONFIG",
            &["16X1", "16X1DP", "16X2", "32X1", "1SHIFT", "2SHIFTS"],
            "ROM",
        );

        // carry chain
        ctx.collect_enum(tile, bel, "CYINIT", &["BX", "CIN"]);
        ctx.collect_enum(tile, bel, "CYSELF", &["F", "1"]);
        ctx.collect_enum(tile, bel, "CYSELG", &["G", "1"]);
        let d_0 = ctx.state.get_diff(tile, bel, "CY0F", "0");
        let d_1 = ctx.state.get_diff(tile, bel, "CY0F", "1");
        let d_f1_g1 = ctx.state.get_diff(tile, bel, "CY0F", "F1");
        let d_prod = ctx.state.get_diff(tile, bel, "CY0F", "PROD");
        assert_eq!(d_0, ctx.state.get_diff(tile, bel, "CY0G", "0"));
        assert_eq!(d_1, ctx.state.get_diff(tile, bel, "CY0G", "1"));
        assert_eq!(d_f1_g1, ctx.state.get_diff(tile, bel, "CY0G", "G1"));
        assert_eq!(d_prod, ctx.state.get_diff(tile, bel, "CY0G", "PROD"));
        ctx.tiledb.insert(
            tile,
            bel,
            "CY0",
            xlat_enum(vec![
                ("0", d_0),
                ("1", d_1),
                ("F1_G1", d_f1_g1),
                ("PROD", d_prod),
            ]),
        );

        // muxes
        let yb_by = ctx.state.get_diff(tile, bel, "YBMUX", "0");
        let yb_cy = ctx.state.get_diff(tile, bel, "YBMUX", "1");
        ctx.tiledb.insert(
            tile,
            bel,
            "YBMUX",
            xlat_enum(vec![("BY", yb_by), ("CY", yb_cy)]),
        );
        let dx_bx = ctx.state.get_diff(tile, bel, "DXMUX", "0");
        let dx_x = ctx.state.get_diff(tile, bel, "DXMUX", "1");
        ctx.tiledb.insert(
            tile,
            bel,
            "DXMUX",
            xlat_enum(vec![("BX", dx_bx), ("X", dx_x)]),
        );
        let dy_by = ctx.state.get_diff(tile, bel, "DYMUX", "0");
        let dy_y = ctx.state.get_diff(tile, bel, "DYMUX", "1");
        ctx.tiledb.insert(
            tile,
            bel,
            "DYMUX",
            xlat_enum(vec![("BY", dy_by), ("Y", dy_y)]),
        );
        ctx.collect_enum(tile, bel, "FXMUX", &["F", "F5", "FXOR"]);
        ctx.collect_enum(tile, bel, "GYMUX", &["G", "F6", "GXOR"]);

        // FFs
        let ff_sync = ctx.state.get_diff(tile, bel, "SYNC_ATTR", "SYNC");
        ctx.state
            .get_diff(tile, bel, "SYNC_ATTR", "ASYNC")
            .assert_empty();
        ctx.tiledb
            .insert(tile, bel, "FF_SYNC", xlat_bitvec(vec![ff_sync]));

        let revused = ctx.state.get_diff(tile, bel, "REVUSED", "0");
        ctx.tiledb
            .insert(tile, bel, "FF_REV_ENABLE", xlat_bitvec(vec![revused]));

        let ff_latch = ctx.state.get_diff(tile, bel, "FFX", "#LATCH");
        assert_eq!(ff_latch, ctx.state.get_diff(tile, bel, "FFY", "#LATCH"));
        ctx.state.get_diff(tile, bel, "FFX", "#FF").assert_empty();
        ctx.state.get_diff(tile, bel, "FFY", "#FF").assert_empty();
        ctx.tiledb
            .insert(tile, bel, "FF_LATCH", xlat_bitvec(vec![ff_latch]));

        ctx.collect_enum_bool(tile, bel, "INITX", "LOW", "HIGH");
        ctx.collect_enum_bool(tile, bel, "INITY", "LOW", "HIGH");
    }
    // extracted manually from .ll
    for (bel, attr, frame, bit) in [
        ("SLICE0", "READBACK_XQ", 45, 16),
        ("SLICE0", "READBACK_YQ", 39, 16),
        ("SLICE1", "READBACK_XQ", 2, 16),
        ("SLICE1", "READBACK_YQ", 8, 16),
    ] {
        ctx.tiledb.insert(
            tile,
            bel,
            attr,
            TileItem::from_bit(FeatureBit::new(0, frame, bit), false),
        );
    }
}
