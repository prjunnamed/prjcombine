use prjcombine_re_fpga_hammer::{xlat_bit, xlat_bool, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_virtex::bels;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "CLB");
    for i in 0..2 {
        let mut bctx = ctx.bel(bels::SLICE[i]);
        let mode = "SLICE";
        // inverters
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .pin("CLK")
            .test_enum("CKINV", &["0", "1"]);
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .attr("CKINV", "1")
            .pin("CE")
            .pin("CLK")
            .pin("XQ")
            .test_enum("CEMUX", &["0", "1", "CE", "CE_B"]);
        bctx.mode(mode)
            .attr("F", "#LUT:0")
            .attr("DXMUX", "1")
            .attr("SRFFMUX", "0")
            .attr("CEMUX", "0")
            .attr("FFX", "#FF")
            .attr("FFY", "#FF")
            .attr("CKINV", "1")
            .pin("SR")
            .pin("CLK")
            .pin("XQ")
            .test_enum("SRMUX", &["0", "1", "SR", "SR_B"]);
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .attr("DXMUX", "0")
            .pin("BX")
            .pin("XQ")
            .test_enum("BXMUX", &["0", "1", "BX", "BX_B"]);
        bctx.mode(mode)
            .attr("FFY", "#FF")
            .attr("DYMUX", "0")
            .pin("BY")
            .pin("YQ")
            .test_enum("BYMUX", &["0", "1", "BY", "BY_B"]);

        // LUT
        for attr in ["F", "G"] {
            bctx.mode(mode).test_multi_attr_lut(attr, 16);
        }
        bctx.mode(mode).test_enum(
            "RAMCONFIG",
            &["16X1", "16X1DP", "16X2", "32X1", "1SHIFT", "2SHIFTS"],
        );

        // carry chain
        bctx.mode(mode)
            .attr("BXMUX", "BX")
            .attr("CYSELF", "1")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("CIN")
            .pin("BX")
            .pin("COUT")
            .test_enum("CYINIT", &["CIN", "BX"]);
        bctx.mode(mode)
            .attr("F", "#LUT:0")
            .attr("CY0F", "0")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("COUT")
            .test_enum("CYSELF", &["F", "1"]);
        bctx.mode(mode)
            .attr("G", "#LUT:0")
            .attr("CY0G", "0")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("CYSELF", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("COUT")
            .test_enum("CYSELG", &["G", "1"]);
        bctx.mode(mode)
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("FXMUX", "FXOR")
            .attr("F", "#LUT:0")
            .attr("XUSED", "0")
            .attr("CYSELF", "F")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("F1")
            .pin("F2")
            .pin("BX")
            .pin("X")
            .pin("COUT")
            .test_enum("CY0F", &["0", "1", "F1", "PROD"]);
        bctx.mode(mode)
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("BYMUX", "BY")
            .attr("GYMUX", "GXOR")
            .attr("G", "#LUT:0")
            .attr("YUSED", "0")
            .attr("CYSELF", "1")
            .attr("CYSELG", "G")
            .attr("COUTUSED", "0")
            .pin("G1")
            .pin("G2")
            .pin("BX")
            .pin("BY")
            .pin("Y")
            .pin("COUT")
            .test_enum("CY0G", &["0", "1", "G1", "PROD"]);

        // muxes
        bctx.mode(mode)
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("BYMUX", "BY")
            .attr("GYMUX", "GXOR")
            .attr("G", "#LUT:0")
            .attr("YUSED", "0")
            .attr("CYSELF", "1")
            .attr("CYSELG", "1")
            .attr("COUTUSED", "0")
            .pin("BX")
            .pin("BY")
            .pin("Y")
            .pin("YB")
            .pin("COUT")
            .test_enum("YBMUX", &["0", "1"]);
        bctx.mode(mode)
            .attr("F", "#LUT:0")
            .attr("XUSED", "0")
            .attr("FXMUX", "F")
            .attr("FFX", "#FF")
            .attr("BXMUX", "BX")
            .pin("X")
            .pin("XQ")
            .pin("BX")
            .test_enum("DXMUX", &["0", "1"]);
        bctx.mode(mode)
            .attr("G", "#LUT:0")
            .attr("YUSED", "0")
            .attr("GYMUX", "G")
            .attr("FFY", "#FF")
            .attr("BYMUX", "BY")
            .pin("Y")
            .pin("YQ")
            .pin("BY")
            .test_enum("DYMUX", &["0", "1"]);
        bctx.mode(mode)
            .attr("F", "#LUT:0")
            .attr("CYSELF", "1")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("XUSED", "0")
            .attr("COUTUSED", "0")
            .pin("X")
            .pin("BX")
            .pin("COUT")
            .test_enum("FXMUX", &["F", "F5", "FXOR"]);
        bctx.mode(mode)
            .attr("G", "#LUT:0")
            .attr("CYSELF", "1")
            .attr("CYSELG", "1")
            .attr("CYINIT", "BX")
            .attr("BXMUX", "BX")
            .attr("YUSED", "0")
            .attr("COUTUSED", "0")
            .pin("Y")
            .pin("BX")
            .pin("COUT")
            .test_enum("GYMUX", &["G", "F6", "GXOR"]);

        // FFs
        bctx.mode(mode)
            .pin("XQ")
            .attr("FFX", "#FF")
            .test_enum("SYNC_ATTR", &["SYNC", "ASYNC"]);
        bctx.mode(mode)
            .attr("FFY", "")
            .attr("CEMUX", "CE_B")
            .attr("INITX", "LOW")
            .pin("XQ")
            .pin("CE")
            .test_enum("FFX", &["#FF", "#LATCH"]);
        bctx.mode(mode)
            .attr("FFX", "")
            .attr("CEMUX", "CE_B")
            .attr("INITY", "LOW")
            .pin("YQ")
            .pin("CE")
            .test_enum("FFY", &["#FF", "#LATCH"]);
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .pin("XQ")
            .test_enum("INITX", &["LOW", "HIGH"]);
        bctx.mode(mode)
            .attr("FFY", "#FF")
            .pin("YQ")
            .test_enum("INITY", &["LOW", "HIGH"]);
        bctx.mode(mode)
            .attr("FFX", "#FF")
            .attr("BYMUX", "BY")
            .pin("XQ")
            .pin("BY")
            .test_enum("REVUSED", &["0"]);
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
            .insert(tile, bel, "FF_SR_SYNC", xlat_bit(ff_sync));

        let revused = ctx.state.get_diff(tile, bel, "REVUSED", "0");
        ctx.tiledb
            .insert(tile, bel, "FF_REV_ENABLE", xlat_bit(revused));

        let ff_latch = ctx.state.get_diff(tile, bel, "FFX", "#LATCH");
        assert_eq!(ff_latch, ctx.state.get_diff(tile, bel, "FFY", "#LATCH"));
        ctx.state.get_diff(tile, bel, "FFX", "#FF").assert_empty();
        ctx.state.get_diff(tile, bel, "FFY", "#FF").assert_empty();
        ctx.tiledb.insert(tile, bel, "FF_LATCH", xlat_bit(ff_latch));

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
            TileItem::from_bit(TileBit::new(0, frame, bit), false),
        );
    }
}
