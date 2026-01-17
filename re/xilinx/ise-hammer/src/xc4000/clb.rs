use prjcombine_re_fpga_hammer::{Diff, xlat_bit, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            bel::BelUnused,
            relation::{Delta, Related},
        },
    },
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    let kind = edev.chip.kind;
    let ff_maybe = if kind.is_clb_xl() { "#FF" } else { "" };
    for tile in [
        "CLB", "CLB.B", "CLB.T", "CLB.L", "CLB.LB", "CLB.LT", "CLB.R", "CLB.RB", "CLB.RT",
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::CLB);
        let mode = "CLB";
        bctx.mode(mode).attr("XMUX", "F").pin("X").test_multi_attr(
            "F",
            MultiValue::OldLut('F'),
            16,
        );
        bctx.mode(mode).attr("YMUX", "G").pin("Y").test_multi_attr(
            "G",
            MultiValue::OldLut('G'),
            16,
        );
        bctx.mode(mode)
            .attr("YMUX", "H")
            .pin("Y")
            .test_multi_attr("H", MultiValue::OldLut('H'), 8);
        bctx.mode(mode)
            .attr("XMUX", "F")
            .pin("X")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_manual("F_RAM", "1")
            .attr_diff("F", "#LUT:F=0x0", "#RAM:F=0x0")
            .commit();
        bctx.mode(mode)
            .attr("YMUX", "G")
            .pin("Y")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_manual("G_RAM", "1")
            .attr_diff("G", "#LUT:G=0x0", "#RAM:G=0x0")
            .commit();
        bctx.mode(mode)
            .attr("YMUX", "G")
            .pin("Y")
            .attr("G", "#RAM:G=0x0")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_enum("RAMCLK", &["CLK", "CLKNOT"]);
        bctx.mode(mode)
            .attr("F", "#RAM:F=0x0")
            .attr("G", "#RAM:G=0x0")
            .pin("X")
            .pin("Y")
            .attr("XMUX", "F")
            .attr("YMUX", "G")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("H1", "C1")
            .attr("SR", "C1")
            .test_enum("RAM", &["DP", "32X1"]);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .attr("YMUX", "G")
            .pin("X")
            .pin("Y")
            .pin("C1")
            .attr("SR", "C1")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .test_enum("H0", &["G", "SR"]);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .pin("X")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .attr("H", "#LUT:H=0x0")
            .test_enum("H1", &["C1", "C2", "C3", "C4"]);
        bctx.mode(mode)
            .attr("XMUX", "F")
            .attr("YMUX", "H")
            .pin("X")
            .pin("Y")
            .pin("C1")
            .attr("DIN", "C1")
            .attr("F", "#LUT:F=0x0")
            .attr("H", "#LUT:H=0x0")
            .test_enum("H2", &["F", "DIN"]);
        bctx.mode(mode)
            .attr("XQMUX", "DIN")
            .pin("XQ")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .test_enum("DIN", &["C1", "C2", "C3", "C4"]);
        bctx.mode(mode)
            .attr("YQMUX", "EC")
            .pin("YQ")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .test_enum("EC", &["C1", "C2", "C3", "C4"]);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .attr("YMUX", "G")
            .pin("X")
            .pin("Y")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "SR")
            .attr("YQMUX", "EC")
            .pin("YQ")
            .pin("C1")
            .pin("C2")
            .pin("C3")
            .pin("C4")
            .test_enum("SR", &["C1", "C2", "C3", "C4"]);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .pin("X")
            .attr("F", "#LUT:F=0x0")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "G")
            .attr("H2", "F")
            .attr("DIN", "C1")
            .pin("C1")
            .attr("XQMUX", "DIN")
            .test_enum("DX", &["H", "G", "F", "DIN"]);
        bctx.mode(mode)
            .attr("XMUX", "H")
            .pin("X")
            .attr("F", "#LUT:F=0x0")
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "G")
            .attr("H2", "F")
            .attr("DIN", "C1")
            .pin("C1")
            .attr("XQMUX", "DIN")
            .test_enum("DY", &["H", "G", "F", "DIN"]);
        bctx.mode(mode).test_enum("SRX", &["SET", "RESET"]);
        bctx.mode(mode).test_enum("SRY", &["SET", "RESET"]);
        bctx.mode(mode)
            .attr("EC", "C1")
            .attr("DIN", "C1")
            .attr("DX", "DIN")
            .attr("XQMUX", "QX")
            .attr("CLKX", "CLK")
            .pin("C1")
            .pin("XQ")
            .pin("K")
            .test_enum("ECX", &["EC"]);
        bctx.mode(mode)
            .attr("EC", "C1")
            .attr("DIN", "C1")
            .attr("DY", "DIN")
            .attr("YQMUX", "QY")
            .attr("CLKY", "CLK")
            .pin("C1")
            .pin("YQ")
            .pin("K")
            .test_enum("ECY", &["EC"]);
        bctx.mode(mode)
            .attr("SR", "C1")
            .attr("DIN", "C1")
            .attr("DX", "DIN")
            .attr("XQMUX", "QX")
            .attr("CLKX", "CLK")
            .pin("C1")
            .pin("XQ")
            .pin("K")
            .test_enum("SETX", &["SR"]);
        bctx.mode(mode)
            .attr("SR", "C1")
            .attr("DIN", "C1")
            .attr("DY", "DIN")
            .attr("YQMUX", "QY")
            .attr("CLKY", "CLK")
            .pin("C1")
            .pin("YQ")
            .pin("K")
            .test_enum("SETY", &["SR"]);
        bctx.mode(mode)
            .attr("F", "#LUT:F=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H2", "F")
            .attr("YMUX", "H")
            .pin("X")
            .pin("Y")
            .test_enum("XMUX", &["F", "H"]);
        bctx.mode(mode)
            .attr("G", "#LUT:G=0x0")
            .attr("H", "#LUT:H=0x0")
            .attr("H0", "G")
            .attr("XMUX", "H")
            .pin("X")
            .pin("Y")
            .test_enum("YMUX", &["G", "H"]);
        bctx.mode(mode)
            .attr("DIN", "C1")
            .attr("DX", "DIN")
            .attr("XQMUX", "QX")
            .attr("FFX", ff_maybe)
            .pin("C1")
            .pin("XQ")
            .pin("K")
            .test_manual("INV.FFX_CLK", "1")
            .attr_diff("CLKX", "CLK", "CLKNOT")
            .commit();
        bctx.mode(mode)
            .attr("DIN", "C1")
            .attr("DY", "DIN")
            .attr("YQMUX", "QY")
            .attr("FFY", ff_maybe)
            .pin("C1")
            .pin("YQ")
            .pin("K")
            .test_manual("INV.FFY_CLK", "1")
            .attr_diff("CLKY", "CLK", "CLKNOT")
            .commit();

        if kind.is_clb_xl() {
            bctx.mode(mode)
                .attr("DIN", "C1")
                .attr("DX", "DIN")
                .attr("XQMUX", "QX")
                .pin("C1")
                .pin("XQ")
                .pin("K")
                .test_manual("FFX_LATCH", "1")
                .attr_diff("CLKX", "CLK", "CLKNOT")
                .attr_diff("FFX", "#FF", "#LATCH")
                .commit();
            bctx.mode(mode)
                .attr("DIN", "C1")
                .attr("DY", "DIN")
                .attr("YQMUX", "QY")
                .pin("C1")
                .pin("YQ")
                .pin("K")
                .test_manual("FFY_LATCH", "1")
                .attr_diff("CLKY", "CLK", "CLKNOT")
                .attr_diff("FFY", "#FF", "#LATCH")
                .commit();
        }

        bctx.mode(mode)
            .attr("DIN", "C1")
            .pin("C1")
            .pin("XQ")
            .test_enum("XQMUX", &["DIN"]);
        bctx.mode(mode)
            .attr("EC", "C1")
            .pin("C1")
            .pin("XQ")
            .test_enum("YQMUX", &["EC"]);

        for val in ["ADDSUB", "SUB"] {
            bctx.mode(mode)
                .attr("FCARRY", "CARRY")
                .attr("GCARRY", "CARRY")
                .attr("CINMUX", "CIN")
                .test_manual("CARRY_ADDSUB", val)
                .attr_diff("CARRY", "ADD", val)
                .commit();
        }
        for val in ["F1", "F3_INV"] {
            let rval = if val == "F3_INV" { "F3" } else { val };
            bctx.mode(mode)
                .attr("FCARRY", "")
                .attr("GCARRY", "CARRY")
                .attr("CARRY", "ADD")
                .pin("CIN")
                .test_manual("CARRY_FGEN", val)
                .attr_diff("CINMUX", "0", rval)
                .commit();
        }
        bctx.mode(mode)
            .attr("FCARRY", "CARRY")
            .attr("GCARRY", "CARRY")
            .attr("CINMUX", "CIN")
            .test_manual("CARRY_OP2_ENABLE", "1")
            .attr_diff("CARRY", "INCDEC", "ADDSUB")
            .commit();
        bctx.mode(mode)
            .attr("CARRY", "ADD")
            .attr("GCARRY", "CARRY")
            .test_manual("CARRY_FPROP", "CONST_0")
            .attr_diff("FCARRY", "CARRY", "")
            .attr_diff("CINMUX", "CIN", "F1")
            .commit();
        bctx.mode(mode)
            .attr("CARRY", "ADD")
            .attr("GCARRY", "CARRY")
            .attr("CINMUX", "CIN")
            .test_manual("CARRY_FPROP", "CONST_1")
            .attr_diff("FCARRY", "CARRY", "")
            .commit();

        bctx.mode(mode)
            .attr("CARRY", "ADD")
            .attr("FCARRY", "CARRY")
            .attr("CINMUX", "CIN")
            .test_manual("CARRY_GPROP", "CONST_1")
            .attr_diff("GCARRY", "CARRY", "")
            .commit();
        if kind.is_clb_xl() {
            bctx.mode(mode)
                .attr("CARRY", "ADD")
                .attr("FCARRY", "CARRY")
                .test_manual("CARRY_GPROP", "CONST_0")
                .attr_diff("GCARRY", "CARRY", "")
                .attr_diff("CINMUX", "CIN", "G4")
                .commit();
        } else if tile == "CLB" {
            bctx.mode(mode)
                .pin("CIN")
                .prop(Related::new(
                    Delta::new(0, -1, "CLB"),
                    BelUnused::new(bels::CLB),
                ))
                .prop(Related::new(
                    Delta::new(0, 1, "CLB"),
                    BelUnused::new(bels::CLB),
                ))
                .test_manual("MUX.CIN", "COUT_B")
                .related_pip(Delta::new(0, -1, "CLB"), "CIN.T", "COUT")
                .commit();
            bctx.mode(mode)
                .pin("CIN")
                .prop(Related::new(
                    Delta::new(0, -1, "CLB"),
                    BelUnused::new(bels::CLB),
                ))
                .prop(Related::new(
                    Delta::new(0, 1, "CLB"),
                    BelUnused::new(bels::CLB),
                ))
                .test_manual("MUX.CIN", "COUT_T")
                .related_pip(Delta::new(0, 1, "CLB"), "CIN.B", "COUT")
                .commit();
        }
        // F4MUX, G2MUX, G3MUX handled as part of interconnect
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = edev.chip.kind;
    for tile in [
        "CLB", "CLB.B", "CLB.T", "CLB.L", "CLB.LB", "CLB.LT", "CLB.R", "CLB.RB", "CLB.RT",
    ] {
        let bel = "CLB";
        ctx.collect_bitvec(tile, bel, "F", "");
        ctx.collect_bitvec(tile, bel, "G", "");
        ctx.collect_bitvec(tile, bel, "H", "");
        ctx.collect_bit(tile, bel, "F_RAM", "1");
        ctx.collect_bit(tile, bel, "G_RAM", "1");
        let item = ctx.extract_bit(tile, bel, "RAM", "DP");
        ctx.insert(tile, bel, "RAM_DP", item);
        let item = ctx.extract_bit(tile, bel, "RAM", "32X1");
        ctx.insert(tile, bel, "RAM_32X1", item);

        let diff_s = ctx.state.get_diff(tile, bel, "RAMCLK", "CLK");
        let diff_inv = ctx
            .state
            .get_diff(tile, bel, "RAMCLK", "CLKNOT")
            .combine(&!&diff_s);
        ctx.insert(tile, bel, "RAM_SYNC", xlat_bit(diff_s));
        ctx.insert(tile, bel, "INV.RAM_CLK", xlat_bit(diff_inv));

        for pin in ["H1", "EC", "SR", "DIN"] {
            let item = ctx.extract_enum(tile, bel, pin, &["C1", "C2", "C3", "C4"]);
            ctx.insert(tile, bel, format!("MUX.{pin}"), item);
        }
        let item = ctx.extract_enum(tile, bel, "H0", &["G", "SR"]);
        ctx.insert(tile, bel, "MUX.H0", item);
        let item = ctx.extract_enum(tile, bel, "H2", &["F", "DIN"]);
        ctx.insert(tile, bel, "MUX.H2", item);

        let item = ctx.extract_enum(tile, bel, "DX", &["F", "G", "H", "DIN"]);
        ctx.insert(tile, bel, "MUX.DX", item);
        let item = ctx.extract_enum(tile, bel, "DY", &["F", "G", "H", "DIN"]);
        ctx.insert(tile, bel, "MUX.DY", item);

        let item = ctx.extract_enum_bool(tile, bel, "SRX", "RESET", "SET");
        ctx.insert(tile, bel, "FFX_SRVAL", item);
        let item = ctx.extract_enum_bool(tile, bel, "SRY", "RESET", "SET");
        ctx.insert(tile, bel, "FFY_SRVAL", item);

        let item = ctx.extract_bit(tile, bel, "ECX", "EC");
        ctx.insert(tile, bel, "FFX_EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "ECY", "EC");
        ctx.insert(tile, bel, "FFY_EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "SETX", "SR");
        ctx.insert(tile, bel, "FFX_SR_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "SETY", "SR");
        ctx.insert(tile, bel, "FFY_SR_ENABLE", item);

        let item = ctx.extract_enum(tile, bel, "XMUX", &["F", "H"]);
        ctx.insert(tile, bel, "MUX.X", item);
        let item = ctx.extract_enum(tile, bel, "YMUX", &["G", "H"]);
        ctx.insert(tile, bel, "MUX.Y", item);

        ctx.collect_bit(tile, bel, "INV.FFX_CLK", "1");
        ctx.collect_bit(tile, bel, "INV.FFY_CLK", "1");

        if kind.is_clb_xl() {
            ctx.collect_bit(tile, bel, "FFX_LATCH", "1");
            ctx.collect_bit(tile, bel, "FFY_LATCH", "1");
        }

        let item = ctx.extract_enum_default(tile, bel, "XQMUX", &["DIN"], "FFX");
        ctx.insert(tile, bel, "MUX.XQ", item);
        let item = ctx.extract_enum_default(tile, bel, "YQMUX", &["EC"], "FFY");
        ctx.insert(tile, bel, "MUX.YQ", item);

        ctx.collect_enum_default(tile, bel, "CARRY_ADDSUB", &["ADDSUB", "SUB"], "ADD");
        ctx.collect_enum_default(
            tile,
            bel,
            "CARRY_FGEN",
            &["F1", "F3_INV"],
            "CONST_OP2_ENABLE",
        );
        ctx.collect_bit(tile, bel, "CARRY_OP2_ENABLE", "1");

        let diff0 = ctx.state.get_diff(tile, bel, "CARRY_FPROP", "CONST_0");
        let mut diff1 = ctx.state.get_diff(tile, bel, "CARRY_FPROP", "CONST_1");
        diff1.discard_bits(ctx.item(tile, bel, "CARRY_FGEN"));
        ctx.insert(
            tile,
            bel,
            "CARRY_FPROP",
            xlat_enum(vec![
                ("XOR", Diff::default()),
                ("CONST_0", diff0),
                ("CONST_1", diff1),
            ]),
        );

        let diff1 = ctx.state.get_diff(tile, bel, "CARRY_GPROP", "CONST_1");
        if !kind.is_clb_xl() {
            ctx.insert(
                tile,
                bel,
                "CARRY_GPROP",
                xlat_enum(vec![("XOR", Diff::default()), ("CONST_1", diff1)]),
            );
        } else {
            let mut diff0 = ctx.state.get_diff(tile, bel, "CARRY_GPROP", "CONST_0");
            diff0.discard_bits(ctx.item(tile, bel, "CARRY_FGEN"));
            diff0.discard_bits(ctx.item(tile, bel, "CARRY_FPROP"));
            ctx.insert(
                tile,
                bel,
                "CARRY_GPROP",
                xlat_enum(vec![
                    ("XOR", Diff::default()),
                    ("CONST_0", diff0),
                    ("CONST_1", diff1),
                ]),
            );
        }

        if !kind.is_clb_xl() {
            if tile == "CLB" {
                ctx.collect_enum(tile, bel, "MUX.CIN", &["COUT_B", "COUT_T"]);
            } else {
                let item = ctx.item("CLB", bel, "MUX.CIN").clone();
                ctx.insert(tile, bel, "MUX.CIN", item);
            }
        }

        let rb = if kind.is_xl() {
            [
                ("READBACK_X", 0, 3),
                ("READBACK_Y", 0, 5),
                ("READBACK_XQ", 0, 7),
                ("READBACK_YQ", 0, 4),
            ]
        } else if kind == ChipKind::SpartanXl {
            // ?!?! X/XQ swapped from XC4000?
            [
                ("READBACK_X", 12, 5),
                ("READBACK_Y", 3, 5),
                ("READBACK_XQ", 16, 4),
                ("READBACK_YQ", 8, 4),
            ]
        } else {
            [
                ("READBACK_X", 16, 4),
                ("READBACK_Y", 3, 5),
                ("READBACK_XQ", 12, 5),
                ("READBACK_YQ", 8, 4),
            ]
        };
        for (name, frame, bit) in rb {
            ctx.insert(
                tile,
                bel,
                name,
                TileItem::from_bit(TileBit::new(0, frame, bit), true),
            );
        }
    }
}
