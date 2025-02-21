use prjcombine_re_collector::{xlat_bit, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_xc2000::chip::ChipKind;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BL", "MISC", TileBits::Main(0, 1));
    for val in ["ENABLE", "DISABLE"] {
        fuzz_one!(ctx, "READ_ABORT", val, [], [(global_opt "READABORT", val)]);
        fuzz_one!(ctx, "READ_CAPTURE", val, [], [(global_opt "READCAPTURE", val)]);
    }
    for val in ["ON", "OFF"] {
        fuzz_one!(ctx, "TM_BOT", val, [], [(global_opt "TMBOT", val)]);
    }
    if matches!(edev.chip.kind, ChipKind::Xc4000Xla | ChipKind::Xc4000Xv) {
        for val in ["ON", "OFF"] {
            fuzz_one!(ctx, "5V_TOLERANT_IO", val, [], [(global_opt "5V_TOLERANT_IO", val)]);
        }
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BL", "MD0", TileBits::Main(0, 1));
    for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
        fuzz_one!(ctx, "PULL", val, [], [(global_opt "M0PIN", val)]);
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BL", "MD1", TileBits::Main(0, 1));
    for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
        fuzz_one!(ctx, "PULL", val, [], [(global_opt "M1PIN", val)]);
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BL", "MD2", TileBits::Main(0, 1));
    for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
        let opt = if edev.chip.kind == ChipKind::SpartanXl {
            "POWERDOWN"
        } else {
            "M2PIN"
        };
        fuzz_one!(ctx, "PULL", val, [], [(global_opt opt, val)]);
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TL", "MISC", TileBits::Main(0, 1));
    for val in ["ON", "OFF"] {
        fuzz_one!(ctx, "TM_LEFT", val, [], [(global_opt "TMLEFT", val)]);
    }
    for val in ["ON", "OFF"] {
        fuzz_one!(ctx, "TM_TOP", val, [], [(global_opt "TMTOP", val)]);
    }
    for val in ["TTL", "CMOS"] {
        fuzz_one!(ctx, "INPUT", val, [], [(global_opt "INPUT", val)]);
        fuzz_one!(ctx, "OUTPUT", val, [], [(global_opt "OUTPUT", val)]);
    }
    if edev.chip.kind != ChipKind::Xc4000E {
        for val in ["ON", "OFF"] {
            fuzz_one!(ctx, "3V", val, [], [(global_opt "3V", val)]);
        }
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.TL", "BSCAN", TileBits::Main(0, 1));
    let extras = vec![ExtraFeature::new(
        ExtraFeatureKind::MainFixed(edev.chip.col_rio(), edev.chip.row_tio()),
        "CNR.TR",
        "BSCAN",
        "ENABLE",
        "1",
    )];
    fuzz_one_extras!(ctx, "ENABLE", "1", [], [(mode "BSCAN"), (attr "BSCAN", "USED")], extras);
    if matches!(
        edev.chip.kind,
        ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
    ) {
        for val in ["ENABLE", "DISABLE"] {
            fuzz_one!(ctx, "CONFIG", val, [], [(global_opt "BSCAN_CONFIG", val)]);
        }
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BR", "MISC", TileBits::Main(0, 1));
    for val in ["ON", "OFF"] {
        fuzz_one!(ctx, "TCTEST", val, [], [(global_opt "TCTEST", val)]);
    }
    if matches!(
        edev.chip.kind,
        ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
    ) {
        for val in ["ON", "OFF"] {
            fuzz_one!(ctx, "FIX_DISCHARGE", val, [], [(global_opt "FIXDISCHARGE", val)]);
        }
    }
    if matches!(
        edev.chip.kind,
        ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
    ) {
        let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BR", "OSC", TileBits::Main(0, 1));
        for val in ["ON", "OFF"] {
            fuzz_one!(ctx, "TM_OSC", val, [], [(global_opt "TMOSC", val)]);
        }
        for val in ["CCLK", "EXTCLK"] {
            fuzz_one!(ctx, "OSC_CLK", val, [], [(global_opt "OSCCLK", val)]);
        }
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.BR", "STARTUP", TileBits::Main(0, 1));
    fuzz_one!(ctx, "INV.GSR", "1", [(mode "STARTUP"), (pin "GSR")], [(attr "GSRATTR", "NOT")]);
    fuzz_one!(ctx, "INV.GTS", "1", [(mode "STARTUP"), (pin "GTS")], [(attr "GTSATTR", "NOT")]);
    for val in ["ENABLE", "DISABLE"] {
        fuzz_one!(ctx, "CRC", val, [], [(global_opt "CRC", val)]);
    }
    for val in ["SLOW", "FAST"] {
        fuzz_one!(ctx, "CONFIG_RATE", val, [], [(global_opt "CONFIGRATE", val)]);
    }
    if matches!(
        edev.chip.kind,
        ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
    ) {
        for val in ["ENABLE", "DISABLE"] {
            fuzz_one!(ctx, "EXPRESS_MODE", val, [
                (global_opt "CRC", "DISABLE")
            ], [(global_opt "EXPRESSMODE", val)]);
        }
    }
    for (val, phase) in [("CCLK", "C4"), ("USERCLK", "U3")] {
        fuzz_one!(ctx, "STARTUP_CLK", val, [
            (mode "STARTUP"), (pin "CLK"),
            (global_opt "SYNCTODONE", "NO"),
            (global_opt "DONEACTIVE", "C1")
        ], [
            (global_opt_diff "GSRINACTIVE", "C4", phase),
            (global_opt_diff "OUTPUTSACTIVE", "C4", phase),
            (global_opt_diff "STARTUPCLK", "CCLK", val)
        ]);
    }
    for (val, phase) in [("NO", "C4"), ("YES", "DI_PLUS_1")] {
        fuzz_one!(ctx, "SYNC_TO_DONE", val, [
            (global_opt "STARTUPCLK", "CCLK"),
            (global_opt "DONEACTIVE", "C1")
        ], [
            (global_opt_diff "GSRINACTIVE", "C4", phase),
            (global_opt_diff "OUTPUTSACTIVE", "C4", phase),
            (global_opt_diff "SYNCTODONE", "NO", val)
        ]);
    }
    for val in ["C1", "C2", "C3", "C4"] {
        fuzz_one!(ctx, "DONE_ACTIVE", val, [
            (global_opt "SYNCTODONE", "NO"),
            (global_opt "STARTUPCLK", "CCLK")
        ], [(global_opt_diff "DONEACTIVE", "C1", val)]);
    }
    for val in ["U2", "U3", "U4"] {
        fuzz_one!(ctx, "DONE_ACTIVE", val, [
            (mode "STARTUP"),
            (pin "CLK"),
            (global_opt "SYNCTODONE", "NO"),
            (global_opt "STARTUPCLK", "USERCLK")
        ], [(global_opt_diff "DONEACTIVE", "C1", val)]);
    }
    for (attr, opt) in [
        ("OUTPUTS_ACTIVE", "OUTPUTSACTIVE"),
        ("GSR_INACTIVE", "GSRINACTIVE"),
    ] {
        for val in ["C2", "C3", "C4"] {
            fuzz_one!(ctx, attr, val, [
                (global_opt "SYNCTODONE", "NO"),
                (global_opt "STARTUPCLK", "CCLK")
            ], [(global_opt_diff opt, "C4", val)]);
        }
        for val in ["U2", "U3", "U4"] {
            fuzz_one!(ctx, attr, val, [
                (mode "STARTUP"),
                (pin "CLK"),
                (global_opt "SYNCTODONE", "NO"),
                (global_opt "STARTUPCLK", "USERCLK")
            ], [(global_opt_diff opt, "U3", val)]);
        }
        for val in ["DI", "DI_PLUS_1", "DI_PLUS_2"] {
            fuzz_one!(ctx, attr, val, [
                (mode "STARTUP"),
                (pin "CLK"),
                (global_opt "SYNCTODONE", "YES"),
                (global_opt "STARTUPCLK", "USERCLK")
            ], [(global_opt_diff opt, "DI_PLUS_1", val)]);
        }
    }
    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.BR", "DONE", TileBits::Main(0, 1));
    for val in ["PULLUP", "PULLNONE"] {
        fuzz_one!(ctx, "PULL", val, [], [(global_opt "DONEPIN", val)]);
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.TR", "OSC", TileBits::Null);
    for (pin, opin) in [("OUT0", "OUT1"), ("OUT1", "OUT0")] {
        for spin in ["F15", "F490", "F16K", "F500K"] {
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::MainFixed(edev.chip.col_rio(), edev.chip.row_bio()),
                "CNR.BR",
                "OSC",
                format!("MUX.{pin}"),
                spin,
            )];
            fuzz_one_extras!(ctx, format!("MUX.{pin}"), spin, [
                (mutex "MODE", "USE"),
                (mutex format!("MUX.{pin}"), spin),
                (mutex format!("MUX.{opin}"), spin),
                (pip (pin spin), (pin opin))
            ], [
                (pip (pin spin), (pin pin))
            ], extras);
        }
    }
    let extras = vec![ExtraFeature::new(
        ExtraFeatureKind::MainFixed(edev.chip.col_rio(), edev.chip.row_bio()),
        "CNR.BR",
        "OSC",
        "ENABLE",
        "1",
    )];
    fuzz_one_extras!(ctx, "ENABLE", "1", [
        (mutex "MODE", "TEST")
    ], [
        (pip (pin "F15"), (pin "OUT0"))
    ], extras);

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TR", "MISC", TileBits::Main(0, 1));
    for val in ["ON", "OFF"] {
        fuzz_one!(ctx, "TM_RIGHT", val, [], [(global_opt "TMRIGHT", val)]);
    }
    if edev.chip.kind != ChipKind::Xc4000E {
        for val in ["ON", "OFF"] {
            fuzz_one!(ctx, "TAC", val, [], [(global_opt "TAC", val)]);
        }
        for val in ["18", "22"] {
            fuzz_one!(ctx, "ADDRESS_LINES", val, [], [(global_opt "ADDRESSLINES", val)]);
        }
    }
    let ctx = FuzzCtx::new(session, backend, "CNR.BR", "READCLK", TileBits::Null);
    for val in ["CCLK", "RDBK"] {
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::MainFixed(edev.chip.col_rio(), edev.chip.row_tio()),
            "CNR.TR",
            "READCLK",
            "READ_CLK",
            val,
        )];
        fuzz_one_extras!(ctx, "READ_CLK", val, [(mode "READCLK"), (pin "I")], [(global_opt "READCLK", val)], extras);
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TR", "BSCAN", TileBits::Main(0, 1));
    if matches!(
        edev.chip.kind,
        ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
    ) {
        for val in ["ENABLE", "DISABLE"] {
            fuzz_one!(ctx, "STATUS", val, [], [(global_opt "BSCAN_STATUS", val)]);
        }
    }

    let ctx = FuzzCtx::new_fake_bel(session, backend, "CNR.TR", "TDO", TileBits::Main(0, 1));
    for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
        fuzz_one!(ctx, "PULL", val, [], [(global_opt "TDOPIN", val)]);
    }

    let tile = if edev.chip.kind.is_xl() {
        "LLVC.IO.R"
    } else {
        "LLV.IO.R"
    };
    let ctx = FuzzCtx::new_fake_bel(session, backend, tile, "MISC", TileBits::Llv);
    for val in ["OFF", "ON"] {
        fuzz_one!(ctx, "TLC", val, [], [(global_opt "TLC", val)]);
    }

    if edev.chip.kind == ChipKind::SpartanXl {
        let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
        let mut extras = vec![
            ExtraFeature::new(
                ExtraFeatureKind::MainFixed(edev.chip.col_lio(), edev.chip.row_bio()),
                "CNR.BL",
                "MISC",
                "5V_TOLERANT_IO",
                "OFF",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::MainFixed(edev.chip.col_rio(), edev.chip.row_bio()),
                "CNR.BR",
                "MISC",
                "5V_TOLERANT_IO",
                "OFF",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::MainFixed(edev.chip.col_rio(), edev.chip.row_tio()),
                "CNR.TR",
                "MISC",
                "5V_TOLERANT_IO",
                "OFF",
            ),
        ];
        for (nid, name, node) in &backend.egrid.db.nodes {
            if node.bels.get("IOB0").is_none() {
                continue;
            }
            if backend.egrid.node_index[nid].is_empty() {
                continue;
            }
            extras.push(ExtraFeature::new(
                ExtraFeatureKind::AllIobs,
                name,
                "MISC",
                "5V_TOLERANT_IO",
                "OFF",
            ));
        }
        fuzz_one_extras!(ctx, "5V_TOLERANT_IO", "OFF", [], [
            (global_opt "5V_TOLERANT_IO", "OFF")
        ], extras);
    }
    if edev.chip.kind == ChipKind::Xc4000Ex {
        let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
        for val in ["EXTERNAL", "INTERNAL"] {
            let mut extras = vec![];
            for (nid, name, _) in &backend.egrid.db.nodes {
                if !name.starts_with("IO.L") {
                    continue;
                }
                if backend.egrid.node_index[nid].is_empty() {
                    continue;
                }
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::AllIobs,
                    name,
                    "MISC",
                    "PUMP",
                    val,
                ));
            }
            fuzz_one_extras!(ctx, "PUMP", val, [], [
                (global_opt "PUMP", val)
            ], extras);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };

    {
        let tile = "CNR.BL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "READ_ABORT", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "READ_CAPTURE", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "TM_BOT", "OFF", "ON");
        if matches!(edev.chip.kind, ChipKind::Xc4000Xla | ChipKind::Xc4000Xv) {
            ctx.collect_enum_bool(tile, bel, "5V_TOLERANT_IO", "OFF", "ON");
        }
        ctx.collect_enum(tile, "MD0", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        ctx.collect_enum(tile, "MD1", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        ctx.collect_enum(tile, "MD2", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff = ctx.state.get_diff(tile, bel, "5V_TOLERANT_IO", "OFF");
            let diff_m0 = diff.split_bits_by(|bit| bit.frame == 21);
            let diff_m1 = diff.split_bits_by(|bit| bit.frame == 22);
            let diff_m2 = diff.split_bits_by(|bit| bit.frame == 20);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "MD0", "5V_TOLERANT_IO", xlat_bit(!diff_m0));
            ctx.tiledb
                .insert(tile, "MD1", "5V_TOLERANT_IO", xlat_bit(!diff_m1));
            ctx.tiledb
                .insert(tile, "MD2", "5V_TOLERANT_IO", xlat_bit(!diff_m2));
        }
    }

    {
        let tile = "CNR.TL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TM_LEFT", "OFF", "ON");
        ctx.collect_enum_bool(tile, bel, "TM_TOP", "OFF", "ON");
        if edev.chip.kind != ChipKind::Xc4000E {
            ctx.collect_enum_bool(tile, bel, "3V", "OFF", "ON");
        }
        ctx.collect_enum(tile, bel, "INPUT", &["CMOS", "TTL"]);
        ctx.collect_enum(tile, bel, "OUTPUT", &["CMOS", "TTL"]);
        let bel = "BSCAN";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "CONFIG", "DISABLE", "ENABLE");
        }
    }

    {
        let tile = "CNR.BR";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TCTEST", "OFF", "ON");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
        ) {
            ctx.collect_enum_bool(tile, bel, "FIX_DISCHARGE", "OFF", "ON");
        }
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff = ctx.state.get_diff(tile, bel, "5V_TOLERANT_IO", "OFF");
            let diff_prog = diff.split_bits_by(|bit| bit.frame == 8);
            let diff_done = diff.split_bits_by(|bit| bit.frame == 3);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "PROG", "5V_TOLERANT_IO", xlat_bit(!diff_prog));
            ctx.tiledb
                .insert(tile, "DONE", "5V_TOLERANT_IO", xlat_bit(!diff_done));
        }

        let bel = "STARTUP";
        ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
        ctx.collect_enum(tile, bel, "CONFIG_RATE", &["SLOW", "FAST"]);
        ctx.collect_bit(tile, bel, "INV.GSR", "1");
        ctx.collect_bit(tile, bel, "INV.GTS", "1");
        let item = xlat_enum(vec![
            ("Q0", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C1")),
            ("Q2", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C3")),
            ("Q3", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C4")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C2")),
            ("Q2", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U2")),
            ("Q3", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U3")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U4")),
        ]);
        ctx.tiledb.insert(tile, bel, "DONE_ACTIVE", item);
        for attr in ["OUTPUTS_ACTIVE", "GSR_INACTIVE"] {
            let item = xlat_enum(vec![
                ("DONE_IN", ctx.state.get_diff(tile, bel, attr, "DI")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_1")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_2")),
                ("Q2", ctx.state.get_diff(tile, bel, attr, "C3")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "C4")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "C2")),
                ("Q2", ctx.state.get_diff(tile, bel, attr, "U2")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "U3")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "U4")),
            ]);
            ctx.tiledb.insert(tile, bel, attr, item);
        }
        ctx.collect_enum(tile, bel, "STARTUP_CLK", &["CCLK", "USERCLK"]);
        ctx.collect_enum_bool(tile, bel, "SYNC_TO_DONE", "NO", "YES");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "EXPRESS_MODE", "DISABLE", "ENABLE");
        }
        let bel = "OSC";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        ctx.collect_enum(tile, bel, "MUX.OUT0", &["F500K", "F16K", "F490", "F15"]);
        ctx.collect_enum(tile, bel, "MUX.OUT1", &["F500K", "F16K", "F490", "F15"]);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "TM_OSC", "OFF", "ON");
            ctx.collect_enum(tile, bel, "OSC_CLK", &["CCLK", "EXTCLK"]);
        }
        ctx.collect_enum(tile, "DONE", "PULL", &["PULLUP", "PULLNONE"]);
    }
    {
        let tile = "CNR.TR";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TM_RIGHT", "OFF", "ON");
        ctx.collect_enum(tile, "TDO", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        if edev.chip.kind != ChipKind::Xc4000E {
            ctx.collect_enum_bool(tile, bel, "TAC", "OFF", "ON");
            ctx.collect_enum(tile, bel, "ADDRESS_LINES", &["18", "22"]);
        }
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff = ctx.state.get_diff(tile, bel, "5V_TOLERANT_IO", "OFF");
            let diff_tdo = diff.split_bits_by(|bit| bit.frame == 12);
            let diff_cclk = diff.split_bits_by(|bit| bit.frame == 13);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "TDO", "5V_TOLERANT_IO", xlat_bit(!diff_tdo));
            ctx.tiledb
                .insert(tile, "CCLK", "5V_TOLERANT_IO", xlat_bit(!diff_cclk));
        }
        let bel = "BSCAN";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "STATUS", "DISABLE", "ENABLE");
        }
        let bel = "READCLK";
        ctx.collect_enum(tile, bel, "READ_CLK", &["CCLK", "RDBK"]);
    }
    {
        let tile = if edev.chip.kind.is_xl() {
            "LLVC.IO.R"
        } else {
            "LLV.IO.R"
        };
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TLC", "OFF", "ON");
    }
    if edev.chip.kind == ChipKind::SpartanXl {
        for tile in edev.egrid.db.nodes.keys() {
            if !tile.starts_with("IO") {
                continue;
            }
            if !ctx.has_tile(tile) {
                continue;
            }
            let mut diff = ctx.state.get_diff(tile, "MISC", "5V_TOLERANT_IO", "OFF");
            let (f0, f1) = if tile.starts_with("IO.L") {
                (19, 20)
            } else if tile.starts_with("IO.R") {
                (6, 5)
            } else if tile.starts_with("IO.B") || tile.starts_with("IO.T") {
                (13, 12)
            } else {
                unreachable!()
            };
            let diff_iob0 = diff.split_bits_by(|bit| bit.frame == f0);
            let diff_iob1 = diff.split_bits_by(|bit| bit.frame == f1);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "IOB0", "5V_TOLERANT_IO", xlat_bit(!diff_iob0));
            ctx.tiledb
                .insert(tile, "IOB1", "5V_TOLERANT_IO", xlat_bit(!diff_iob1));
        }
    }
    if edev.chip.kind == ChipKind::Xc4000Ex {
        for tile in edev.egrid.db.nodes.keys() {
            if !tile.starts_with("IO.L") {
                continue;
            }
            if !ctx.has_tile(tile) {
                continue;
            }
            ctx.collect_enum(tile, "MISC", "PUMP", &["EXTERNAL", "INTERNAL"]);
        }
    }
}
