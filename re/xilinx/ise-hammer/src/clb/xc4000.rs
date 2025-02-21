use prjcombine_re_collector::{xlat_bit, xlat_enum, Diff};
use prjcombine_re_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem};
use prjcombine_xc2000::grid::GridKind;
use prjcombine_re_xilinx_geom::ExpandedDevice;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    let kind = edev.grid.kind;
    let ff_maybe = if kind.is_clb_xl() { "#FF" } else { "" };
    for tile in [
        "CLB", "CLB.B", "CLB.T", "CLB.L", "CLB.LB", "CLB.LT", "CLB.R", "CLB.RB", "CLB.RT",
    ] {
        let ctx = FuzzCtx::new(session, backend, tile, "CLB", TileBits::Main(0, 1));
        fuzz_multi!(ctx, "F", "", 16, [
            (mode "CLB"),
            (attr "XMUX", "F"),
            (pin "X")
        ], (attr_oldlut "F", 'F'));
        fuzz_multi!(ctx, "G", "", 16, [
            (mode "CLB"),
            (attr "YMUX", "G"),
            (pin "Y")
        ], (attr_oldlut "G", 'G'));
        fuzz_multi!(ctx, "H", "", 8, [
            (mode "CLB"),
            (attr "YMUX", "H"),
            (pin "Y")
        ], (attr_oldlut "H", 'H'));
        fuzz_one!(ctx, "F_RAM", "1", [
            (mode "CLB"),
            (attr "XMUX", "F"),
            (pin "X"),
            (pin "C1"),
            (attr "DIN", "C1"),
            (attr "H1", "C1"),
            (attr "SR", "C1")
        ], [
            (attr_diff "F", "#LUT:F=0x0", "#RAM:F=0x0")
        ]);
        fuzz_one!(ctx, "G_RAM", "1", [
            (mode "CLB"),
            (attr "YMUX", "G"),
            (pin "Y"),
            (pin "C1"),
            (attr "DIN", "C1"),
            (attr "H1", "C1"),
            (attr "SR", "C1")
        ], [
            (attr_diff "G", "#LUT:G=0x0", "#RAM:G=0x0")
        ]);
        fuzz_enum!(ctx, "RAMCLK", ["CLK", "CLKNOT"], [
            (mode "CLB"),
            (attr "YMUX", "G"),
            (pin "Y"),
            (attr "G", "#RAM:G=0x0"),
            (pin "C1"),
            (attr "DIN", "C1"),
            (attr "H1", "C1"),
            (attr "SR", "C1")
        ]);
        fuzz_enum!(ctx, "RAM", ["DP", "32X1"], [
            (mode "CLB"),
            (attr "F", "#RAM:F=0x0"),
            (attr "G", "#RAM:G=0x0"),
            (pin "X"),
            (pin "Y"),
            (attr "XMUX", "F"),
            (attr "YMUX", "G"),
            (pin "C1"),
            (attr "DIN", "C1"),
            (attr "H1", "C1"),
            (attr "SR", "C1")
        ]);
        fuzz_enum!(ctx, "H0", ["G", "SR"], [
            (mode "CLB"),
            (attr "XMUX", "H"),
            (attr "YMUX", "G"),
            (pin "X"),
            (pin "Y"),
            (pin "C1"),
            (attr "SR", "C1"),
            (attr "G", "#LUT:G=0x0"),
            (attr "H", "#LUT:H=0x0")
        ]);
        fuzz_enum!(ctx, "H1", ["C1", "C2", "C3", "C4"], [
            (mode "CLB"),
            (attr "XMUX", "H"),
            (pin "X"),
            (pin "C1"),
            (pin "C2"),
            (pin "C3"),
            (pin "C4"),
            (attr "H", "#LUT:H=0x0")
        ]);
        fuzz_enum!(ctx, "H2", ["F", "DIN"], [
            (mode "CLB"),
            (attr "XMUX", "F"),
            (attr "YMUX", "H"),
            (pin "X"),
            (pin "Y"),
            (pin "C1"),
            (attr "DIN", "C1"),
            (attr "F", "#LUT:F=0x0"),
            (attr "H", "#LUT:H=0x0")
        ]);
        fuzz_enum!(ctx, "DIN", ["C1", "C2", "C3", "C4"], [
            (mode "CLB"),
            (attr "XQMUX", "DIN"),
            (pin "XQ"),
            (pin "C1"),
            (pin "C2"),
            (pin "C3"),
            (pin "C4")
        ]);
        fuzz_enum!(ctx, "EC", ["C1", "C2", "C3", "C4"], [
            (mode "CLB"),
            (attr "YQMUX", "EC"),
            (pin "YQ"),
            (pin "C1"),
            (pin "C2"),
            (pin "C3"),
            (pin "C4")
        ]);
        fuzz_enum!(ctx, "SR", ["C1", "C2", "C3", "C4"], [
            (mode "CLB"),
            (attr "XMUX", "H"),
            (attr "YMUX", "G"),
            (pin "X"),
            (pin "Y"),
            (attr "G", "#LUT:G=0x0"),
            (attr "H", "#LUT:H=0x0"),
            (attr "H0", "SR"),
            (attr "YQMUX", "EC"),
            (pin "YQ"),
            (pin "C1"),
            (pin "C2"),
            (pin "C3"),
            (pin "C4")
        ]);
        fuzz_enum!(ctx, "DX", ["H", "G", "F", "DIN"], [
            (mode "CLB"),
            (attr "XMUX", "H"),
            (pin "X"),
            (attr "F", "#LUT:F=0x0"),
            (attr "G", "#LUT:G=0x0"),
            (attr "H", "#LUT:H=0x0"),
            (attr "H0", "G"),
            (attr "H2", "F"),
            (attr "DIN", "C1"),
            (pin "C1"),
            (attr "XQMUX", "DIN")
        ]);
        fuzz_enum!(ctx, "DY", ["H", "G", "F", "DIN"], [
            (mode "CLB"),
            (attr "XMUX", "H"),
            (pin "X"),
            (attr "F", "#LUT:F=0x0"),
            (attr "G", "#LUT:G=0x0"),
            (attr "H", "#LUT:H=0x0"),
            (attr "H0", "G"),
            (attr "H2", "F"),
            (attr "DIN", "C1"),
            (pin "C1"),
            (attr "XQMUX", "DIN")
        ]);
        fuzz_enum!(ctx, "SRX", ["SET", "RESET"], [
            (mode "CLB")
        ]);
        fuzz_enum!(ctx, "SRY", ["SET", "RESET"], [
            (mode "CLB")
        ]);
        fuzz_enum!(ctx, "ECX", ["EC"], [
            (mode "CLB"),
            (attr "EC", "C1"),
            (attr "DIN", "C1"),
            (attr "DX", "DIN"),
            (attr "XQMUX", "QX"),
            (attr "CLKX", "CLK"),
            (pin "C1"),
            (pin "XQ"),
            (pin "K")
        ]);
        fuzz_enum!(ctx, "ECY", ["EC"], [
            (mode "CLB"),
            (attr "EC", "C1"),
            (attr "DIN", "C1"),
            (attr "DY", "DIN"),
            (attr "YQMUX", "QY"),
            (attr "CLKY", "CLK"),
            (pin "C1"),
            (pin "YQ"),
            (pin "K")
        ]);
        fuzz_enum!(ctx, "SETX", ["SR"], [
            (mode "CLB"),
            (attr "SR", "C1"),
            (attr "DIN", "C1"),
            (attr "DX", "DIN"),
            (attr "XQMUX", "QX"),
            (attr "CLKX", "CLK"),
            (pin "C1"),
            (pin "XQ"),
            (pin "K")
        ]);
        fuzz_enum!(ctx, "SETY", ["SR"], [
            (mode "CLB"),
            (attr "SR", "C1"),
            (attr "DIN", "C1"),
            (attr "DY", "DIN"),
            (attr "YQMUX", "QY"),
            (attr "CLKY", "CLK"),
            (pin "C1"),
            (pin "YQ"),
            (pin "K")
        ]);
        fuzz_enum!(ctx, "XMUX", ["F", "H"], [
            (mode "CLB"),
            (attr "F", "#LUT:F=0x0"),
            (attr "H", "#LUT:H=0x0"),
            (attr "H2", "F"),
            (attr "YMUX", "H"),
            (pin "X"),
            (pin "Y")
        ]);
        fuzz_enum!(ctx, "YMUX", ["G", "H"], [
            (mode "CLB"),
            (attr "G", "#LUT:G=0x0"),
            (attr "H", "#LUT:H=0x0"),
            (attr "H0", "G"),
            (attr "XMUX", "H"),
            (pin "X"),
            (pin "Y")
        ]);
        fuzz_one!(ctx, "INV.FFX_CLK", "1", [
            (mode "CLB"),
            (attr "DIN", "C1"),
            (attr "DX", "DIN"),
            (attr "XQMUX", "QX"),
            (attr "FFX", ff_maybe),
            (pin "C1"),
            (pin "XQ"),
            (pin "K")
        ], [
            (attr_diff "CLKX", "CLK", "CLKNOT")
        ]);
        fuzz_one!(ctx, "INV.FFY_CLK", "1", [
            (mode "CLB"),
            (attr "DIN", "C1"),
            (attr "DY", "DIN"),
            (attr "YQMUX", "QY"),
            (attr "FFY", ff_maybe),
            (pin "C1"),
            (pin "YQ"),
            (pin "K")
        ], [
            (attr_diff "CLKY", "CLK", "CLKNOT")
        ]);

        if kind.is_clb_xl() {
            fuzz_one!(ctx, "FFX_LATCH", "1", [
                (mode "CLB"),
                (attr "DIN", "C1"),
                (attr "DX", "DIN"),
                (attr "XQMUX", "QX"),
                (pin "C1"),
                (pin "XQ"),
                (pin "K")
            ], [
                (attr_diff "CLKX", "CLK", "CLKNOT"),
                (attr_diff "FFX", "#FF", "#LATCH")
            ]);
            fuzz_one!(ctx, "FFY_LATCH", "1", [
                (mode "CLB"),
                (attr "DIN", "C1"),
                (attr "DY", "DIN"),
                (attr "YQMUX", "QY"),
                (pin "C1"),
                (pin "YQ"),
                (pin "K")
            ], [
                (attr_diff "CLKY", "CLK", "CLKNOT"),
                (attr_diff "FFY", "#FF", "#LATCH")
            ]);
        }

        fuzz_enum!(ctx, "XQMUX", ["DIN"], [
            (mode "CLB"),
            (attr "DIN", "C1"),
            (pin "C1"),
            (pin "XQ")
        ]);
        fuzz_enum!(ctx, "YQMUX", ["EC"], [
            (mode "CLB"),
            (attr "EC", "C1"),
            (pin "C1"),
            (pin "XQ")
        ]);

        for val in ["ADDSUB", "SUB"] {
            fuzz_one!(ctx, "CARRY_ADDSUB", val, [
                (mode "CLB"),
                (attr "FCARRY", "CARRY"),
                (attr "GCARRY", "CARRY"),
                (attr "CINMUX", "CIN")
            ], [
                (attr_diff "CARRY", "ADD", val)
            ]);
        }
        for val in ["F1", "F3_INV"] {
            let rval = if val == "F3_INV" { "F3" } else { val };
            fuzz_one!(ctx, "CARRY_FGEN", val, [
                (mode "CLB"),
                (attr "FCARRY", ""),
                (attr "GCARRY", "CARRY"),
                (attr "CARRY", "ADD"),
                (pin "CIN")
            ], [
                (attr_diff "CINMUX", "0", rval)
            ]);
        }
        fuzz_one!(ctx, "CARRY_OP2_ENABLE", "1", [
            (mode "CLB"),
            (attr "FCARRY", "CARRY"),
            (attr "GCARRY", "CARRY"),
            (attr "CINMUX", "CIN")
        ], [
            (attr_diff "CARRY", "INCDEC", "ADDSUB")
        ]);
        fuzz_one!(ctx, "CARRY_FPROP", "CONST_0", [
            (mode "CLB"),
            (attr "CARRY", "ADD"),
            (attr "GCARRY", "CARRY")
        ], [
            (attr_diff "FCARRY", "CARRY", ""),
            (attr_diff "CINMUX", "CIN", "F1")
        ]);
        fuzz_one!(ctx, "CARRY_FPROP", "CONST_1", [
            (mode "CLB"),
            (attr "CARRY", "ADD"),
            (attr "GCARRY", "CARRY"),
            (attr "CINMUX", "CIN")
        ], [
            (attr_diff "FCARRY", "CARRY", "")
        ]);

        fuzz_one!(ctx, "CARRY_GPROP", "CONST_1", [
            (mode "CLB"),
            (attr "CARRY", "ADD"),
            (attr "FCARRY", "CARRY"),
            (attr "CINMUX", "CIN")
        ], [
            (attr_diff "GCARRY", "CARRY", "")
        ]);
        if kind.is_clb_xl() {
            fuzz_one!(ctx, "CARRY_GPROP", "CONST_0", [
                (mode "CLB"),
                (attr "CARRY", "ADD"),
                (attr "FCARRY", "CARRY")
            ], [
                (attr_diff "GCARRY", "CARRY", ""),
                (attr_diff "CINMUX", "CIN", "G4")
            ]);
        } else if tile == "CLB" {
            let node_clb = backend.egrid.db.get_node("CLB");
            fuzz_one!(ctx, "MUX.CIN", "COUT_B", [
                (mode "CLB"),
                (pin "CIN"),
                (related TileRelation::Delta(0, -1, node_clb), (unused)),
                (related TileRelation::Delta(0, 1, node_clb), (unused))
            ], [
                (related TileRelation::Delta(0, -1, node_clb),
                    (pip (pin "COUT"), (pin "CIN.T")))
            ]);
            fuzz_one!(ctx, "MUX.CIN", "COUT_T", [
                (mode "CLB"),
                (pin "CIN"),
                (related TileRelation::Delta(0, -1, node_clb), (unused)),
                (related TileRelation::Delta(0, 1, node_clb), (unused))
            ], [
                (related TileRelation::Delta(0, 1, node_clb),
                    (pip (pin "COUT"), (pin "CIN.B")))
            ]);
        }
        // F4MUX, G2MUX, G3MUX handled as part of interconnect
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = edev.grid.kind;
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
        ctx.tiledb.insert(tile, bel, "RAM_DP", item);
        let item = ctx.extract_bit(tile, bel, "RAM", "32X1");
        ctx.tiledb.insert(tile, bel, "RAM_32X1", item);

        let diff_s = ctx.state.get_diff(tile, bel, "RAMCLK", "CLK");
        let diff_inv = ctx
            .state
            .get_diff(tile, bel, "RAMCLK", "CLKNOT")
            .combine(&!&diff_s);
        ctx.tiledb.insert(tile, bel, "RAM_SYNC", xlat_bit(diff_s));
        ctx.tiledb
            .insert(tile, bel, "INV.RAM_CLK", xlat_bit(diff_inv));

        for pin in ["H1", "EC", "SR", "DIN"] {
            let item = ctx.extract_enum(tile, bel, pin, &["C1", "C2", "C3", "C4"]);
            ctx.tiledb.insert(tile, bel, format!("MUX.{pin}"), item);
        }
        let item = ctx.extract_enum(tile, bel, "H0", &["G", "SR"]);
        ctx.tiledb.insert(tile, bel, "MUX.H0", item);
        let item = ctx.extract_enum(tile, bel, "H2", &["F", "DIN"]);
        ctx.tiledb.insert(tile, bel, "MUX.H2", item);

        let item = ctx.extract_enum(tile, bel, "DX", &["F", "G", "H", "DIN"]);
        ctx.tiledb.insert(tile, bel, "MUX.DX", item);
        let item = ctx.extract_enum(tile, bel, "DY", &["F", "G", "H", "DIN"]);
        ctx.tiledb.insert(tile, bel, "MUX.DY", item);

        let item = ctx.extract_enum_bool(tile, bel, "SRX", "RESET", "SET");
        ctx.tiledb.insert(tile, bel, "FFX_SRVAL", item);
        let item = ctx.extract_enum_bool(tile, bel, "SRY", "RESET", "SET");
        ctx.tiledb.insert(tile, bel, "FFY_SRVAL", item);

        let item = ctx.extract_bit(tile, bel, "ECX", "EC");
        ctx.tiledb.insert(tile, bel, "FFX_EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "ECY", "EC");
        ctx.tiledb.insert(tile, bel, "FFY_EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "SETX", "SR");
        ctx.tiledb.insert(tile, bel, "FFX_SR_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "SETY", "SR");
        ctx.tiledb.insert(tile, bel, "FFY_SR_ENABLE", item);

        let item = ctx.extract_enum(tile, bel, "XMUX", &["F", "H"]);
        ctx.tiledb.insert(tile, bel, "MUX.X", item);
        let item = ctx.extract_enum(tile, bel, "YMUX", &["G", "H"]);
        ctx.tiledb.insert(tile, bel, "MUX.Y", item);

        ctx.collect_bit(tile, bel, "INV.FFX_CLK", "1");
        ctx.collect_bit(tile, bel, "INV.FFY_CLK", "1");

        if kind.is_clb_xl() {
            ctx.collect_bit(tile, bel, "FFX_LATCH", "1");
            ctx.collect_bit(tile, bel, "FFY_LATCH", "1");
        }

        let item = ctx.extract_enum_default(tile, bel, "XQMUX", &["DIN"], "FFX");
        ctx.tiledb.insert(tile, bel, "MUX.XQ", item);
        let item = ctx.extract_enum_default(tile, bel, "YQMUX", &["EC"], "FFY");
        ctx.tiledb.insert(tile, bel, "MUX.YQ", item);

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
        diff1.discard_bits(ctx.tiledb.item(tile, bel, "CARRY_FGEN"));
        ctx.tiledb.insert(
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
            ctx.tiledb.insert(
                tile,
                bel,
                "CARRY_GPROP",
                xlat_enum(vec![("XOR", Diff::default()), ("CONST_1", diff1)]),
            );
        } else {
            let mut diff0 = ctx.state.get_diff(tile, bel, "CARRY_GPROP", "CONST_0");
            diff0.discard_bits(ctx.tiledb.item(tile, bel, "CARRY_FGEN"));
            diff0.discard_bits(ctx.tiledb.item(tile, bel, "CARRY_FPROP"));
            ctx.tiledb.insert(
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
                let item = ctx.tiledb.item("CLB", bel, "MUX.CIN").clone();
                ctx.tiledb.insert(tile, bel, "MUX.CIN", item);
            }
        }

        let rb = if kind.is_xl() {
            [
                ("READBACK_X", 0, 3),
                ("READBACK_Y", 0, 5),
                ("READBACK_XQ", 0, 7),
                ("READBACK_YQ", 0, 4),
            ]
        } else if kind == GridKind::SpartanXl {
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
            ctx.tiledb.insert(
                tile,
                bel,
                name,
                TileItem::from_bit(TileBit::new(0, frame, bit), true),
            );
        }
    }
}
