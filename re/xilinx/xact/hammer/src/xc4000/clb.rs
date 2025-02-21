use prjcombine_re_collector::{xlat_bit, xlat_enum, Diff};
use prjcombine_re_hammer::Session;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for tile in [
        "CLB", "CLB.L", "CLB.R", "CLB.B", "CLB.LB", "CLB.RB", "CLB.T", "CLB.LT", "CLB.RT",
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel("CLB");
        bctx.mode("FG")
            .mutex("RAM", "")
            .test_equate("F", &["F1", "F2", "F3", "F4"]);
        bctx.mode("FG")
            .mutex("RAM", "")
            .test_equate("G", &["G1", "G2", "G3", "G4"]);
        bctx.mode("FG")
            .mutex("RAM", "")
            .test_equate("H", &["F", "G", "H1"]);
        if !backend.device.name.ends_with('d') {
            bctx.mode("FG").test_enum("RAM", &["F", "G", "FG"]);
        }
        bctx.mode("FG").test_enum("H1", &["C1", "C2", "C3", "C4"]);
        bctx.mode("FG").test_enum("DIN", &["C1", "C2", "C3", "C4"]);
        bctx.mode("FG").test_enum("SR", &["C1", "C2", "C3", "C4"]);
        bctx.mode("FG").test_enum("EC", &["C1", "C2", "C3", "C4"]);
        bctx.mode("FG").test_enum("X", &["F", "H"]);
        bctx.mode("FG").test_enum("Y", &["G", "H"]);
        bctx.mode("FG").test_enum("XQ", &["QX", "DIN"]);
        bctx.mode("FG").test_enum("YQ", &["QY", "EC"]);
        bctx.mode("FG").test_enum("DX", &["DIN", "F", "G", "H"]);
        bctx.mode("FG").test_enum("DY", &["DIN", "F", "G", "H"]);
        bctx.mode("FG").test_enum("FFX", &["SET", "RESET"]);
        bctx.mode("FG").test_enum("FFY", &["SET", "RESET"]);
        bctx.mode("FG").test_cfg("FFX", "EC");
        bctx.mode("FG").test_cfg("FFY", "EC");
        bctx.mode("FG").test_cfg("FFX", "SR");
        bctx.mode("FG").test_cfg("FFY", "SR");
        bctx.mode("FG").test_cfg("FFX", "NOT");
        bctx.mode("FG").test_cfg("FFY", "NOT");
        bctx.mode("FG").test_cfg("FFX", "K");
        bctx.mode("FG").test_cfg("FFY", "K");
        bctx.mode("FG").test_enum("RDBK", &["X", "Y", "XQ", "YQ"]);
        for val in ["CB0", "CB1", "CB2", "CB3", "CB4", "CB5", "CB6", "CB7"] {
            bctx.mode("FG").test_cfg("CARRY", val);
        }
        bctx.mode("FG").test_enum("CDIR", &["UP", "DOWN"]);
        bctx.mode("FG").test_cfg("CIN", "CINI");
        bctx.mode("FG").test_cfg("COUT", "COUTI");
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in [
        "CLB", "CLB.L", "CLB.R", "CLB.B", "CLB.LB", "CLB.RB", "CLB.T", "CLB.LT", "CLB.RT",
    ] {
        let bel = "CLB";
        ctx.collect_bitvec(tile, bel, "F", "");
        ctx.collect_bitvec(tile, bel, "G", "");
        ctx.collect_bitvec(tile, bel, "H", "");

        for attr in ["H1", "DIN", "SR", "EC"] {
            let item = ctx.extract_enum(tile, bel, attr, &["C1", "C2", "C3", "C4"]);
            ctx.tiledb.insert(tile, bel, format!("MUX.{attr}"), item);
        }
        let item = ctx.extract_enum(tile, bel, "X", &["F", "H"]);
        ctx.tiledb.insert(tile, bel, "MUX.X", item);
        let item = ctx.extract_enum(tile, bel, "Y", &["G", "H"]);
        ctx.tiledb.insert(tile, bel, "MUX.Y", item);
        let item = xlat_enum(vec![
            ("DIN", ctx.state.get_diff(tile, bel, "XQ", "DIN")),
            ("FFX", ctx.state.get_diff(tile, bel, "XQ", "QX")),
        ]);
        ctx.tiledb.insert(tile, bel, "MUX.XQ", item);
        let item = xlat_enum(vec![
            ("EC", ctx.state.get_diff(tile, bel, "YQ", "EC")),
            ("FFY", ctx.state.get_diff(tile, bel, "YQ", "QY")),
        ]);
        ctx.tiledb.insert(tile, bel, "MUX.YQ", item);
        let item = ctx.extract_enum(tile, bel, "DX", &["F", "G", "H", "DIN"]);
        ctx.tiledb.insert(tile, bel, "MUX.DX", item);
        let item = ctx.extract_enum(tile, bel, "DY", &["F", "G", "H", "DIN"]);
        ctx.tiledb.insert(tile, bel, "MUX.DY", item);

        let item = ctx.extract_enum_bool(tile, bel, "FFX", "RESET", "SET");
        ctx.tiledb.insert(tile, bel, "FFX_SRVAL", item);
        let item = ctx.extract_enum_bool(tile, bel, "FFY", "RESET", "SET");
        ctx.tiledb.insert(tile, bel, "FFY_SRVAL", item);
        let item = ctx.extract_bit(tile, bel, "FFX", "EC");
        ctx.tiledb.insert(tile, bel, "FFX_EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "FFX", "SR");
        ctx.tiledb.insert(tile, bel, "FFX_SR_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "FFY", "EC");
        ctx.tiledb.insert(tile, bel, "FFY_EC_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "FFY", "SR");
        ctx.tiledb.insert(tile, bel, "FFY_SR_ENABLE", item);
        let item = ctx.extract_bit(tile, bel, "FFX", "NOT");
        ctx.tiledb.insert(tile, bel, "INV.FFX_CLK", item);
        let item = ctx.extract_bit(tile, bel, "FFY", "NOT");
        ctx.tiledb.insert(tile, bel, "INV.FFY_CLK", item);
        ctx.state.get_diff(tile, bel, "FFX", "K").assert_empty();
        ctx.state.get_diff(tile, bel, "FFY", "K").assert_empty();

        ctx.state.get_diff(tile, bel, "CIN", "CINI").assert_empty();
        ctx.state
            .get_diff(tile, bel, "COUT", "COUTI")
            .assert_empty();
        let item = xlat_enum(vec![
            ("COUT_B", ctx.state.get_diff(tile, bel, "CDIR", "UP")),
            ("COUT_T", ctx.state.get_diff(tile, bel, "CDIR", "DOWN")),
        ]);
        ctx.tiledb.insert(tile, bel, "MUX.CIN", item);
        let bit0 = ctx.state.get_diff(tile, bel, "CARRY", "CB0");
        let bit1 = ctx.state.get_diff(tile, bel, "CARRY", "CB1");
        let item = xlat_enum(vec![
            ("ADD", bit1),
            ("ADDSUB", bit0),
            ("SUB", Diff::default()),
        ]);
        ctx.tiledb.insert(tile, bel, "CARRY_ADDSUB", item);
        let bit2 = ctx.state.get_diff(tile, bel, "CARRY", "CB2");
        let bit3 = ctx.state.get_diff(tile, bel, "CARRY", "CB3");
        let item = xlat_enum(vec![
            ("XOR", bit3),
            ("CONST_1", bit2),
            ("CONST_0", Diff::default()),
        ]);
        ctx.tiledb.insert(tile, bel, "CARRY_FPROP", item);
        let bit4 = ctx.state.get_diff(tile, bel, "CARRY", "CB4");
        let bit5 = ctx.state.get_diff(tile, bel, "CARRY", "CB5");
        let item = xlat_enum(vec![
            ("F1", bit4.combine(&bit5)),
            ("F3_INV", bit5),
            ("CONST_OP2_ENABLE", Diff::default()),
        ]);
        ctx.tiledb.insert(tile, bel, "CARRY_FGEN", item);
        let bit6 = ctx.state.get_diff(tile, bel, "CARRY", "CB6");
        let item = xlat_enum(vec![("XOR", bit6), ("CONST_1", Diff::default())]);
        ctx.tiledb.insert(tile, bel, "CARRY_GPROP", item);
        let item = ctx.extract_bit(tile, bel, "CARRY", "CB7");
        ctx.tiledb.insert(tile, bel, "CARRY_OP2_ENABLE", item);

        let item = ctx.extract_bit(tile, bel, "RDBK", "X");
        ctx.tiledb.insert(tile, bel, "READBACK_X", item);
        let item = ctx.extract_bit(tile, bel, "RDBK", "XQ");
        ctx.tiledb.insert(tile, bel, "READBACK_XQ", item);
        let item = ctx.extract_bit(tile, bel, "RDBK", "Y");
        ctx.tiledb.insert(tile, bel, "READBACK_Y", item);
        let item = ctx.extract_bit(tile, bel, "RDBK", "YQ");
        ctx.tiledb.insert(tile, bel, "READBACK_YQ", item);

        if !ctx.device.name.ends_with('d') {
            let item = ctx.extract_bit(tile, bel, "RAM", "F");
            ctx.tiledb.insert(tile, bel, "F_RAM", item);
            let item = ctx.extract_bit(tile, bel, "RAM", "G");
            ctx.tiledb.insert(tile, bel, "G_RAM", item);
            let mut diff = ctx.state.get_diff(tile, bel, "RAM", "FG");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "F_RAM"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "G_RAM"), true, false);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "H"), 0xca, 0x00);
            let item = xlat_bit(diff);
            ctx.tiledb.insert(tile, bel, "RAM_32X1", item);
        }
    }
}
