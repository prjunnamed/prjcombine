use prjcombine_interconnect::grid::{DieId, LayerId};
use prjcombine_re_fpga_hammer::{Diff, xlat_bit, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind};
use unnamed_entity::EntityId;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let grid = backend.edev.chip;
    let num_dec = if grid.kind == ChipKind::Xc4000A { 2 } else { 4 };
    for tile in ["CNR.BL", "CNR.TL", "CNR.BR", "CNR.TR"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for slots in [bels::PULLUP_DEC_H, bels::PULLUP_DEC_V] {
            for i in 0..num_dec {
                let mut bctx = ctx.bel(slots[i]);
                bctx.build()
                    .pin_mutex_exclusive("O")
                    .test_manual("ENABLE", "1")
                    .pip_pin("O", "O")
                    .commit();
            }
        }
        if tile == "CNR.BL" {
            ctx.test_global("MISC", "READABORT", &["DISABLE", "ENABLE"]);
            ctx.test_global("MISC", "READCAPTURE", &["DISABLE", "ENABLE"]);
            ctx.test_cfg4000("MISC", "TMBOT", &["OFF", "ON"]);
            ctx.test_global("MD1", "M1PIN", &["PULLDOWN", "PULLUP"]);
        }
        if tile == "CNR.TL" {
            ctx.test_cfg4000("MISC", "TMLEFT", &["OFF", "ON"]);
            ctx.test_cfg4000("MISC", "TMTOP", &["OFF", "ON"]);
            ctx.test_cfg4000("MISC", "TTLBAR", &["OFF", "ON"]);
            let mut bctx = ctx.bel(bels::BSCAN);
            bctx.mode("BSCAN")
                .extra_tile(
                    (
                        DieId::from_idx(0),
                        grid.col_e(),
                        grid.row_n(),
                        LayerId::from_idx(0),
                    ),
                    "BSCAN",
                    "BSCAN",
                    "USED",
                )
                .test_cfg("BSCAN", "USED");
        }
        if tile == "CNR.BR" {
            ctx.test_cfg4000("MISC", "TCTEST", &["OFF", "ON"]);
            ctx.test_global("STARTUP", "CRC", &["DISABLE", "ENABLE"]);
            ctx.test_global("STARTUP", "CONFIGRATE", &["SLOW", "FAST"]);
            ctx.test_global("DONE", "DONEPIN", &["NOPULLUP", "PULLUP"]);
            ctx.build()
                .global("SYNCTODONE", "NO")
                .global("DONEACTIVE", "C1")
                .test_manual("STARTUP", "STARTUP_CLK", "USERCLK")
                .global_diff("GSRINACTIVE", "C4", "U3")
                .global_diff("OUTPUTSACTIVE", "C4", "U3")
                .global_diff("STARTUPCLK", "CCLK", "USERCLK")
                .bel_out("STARTUP", "CLK")
                .commit();
            ctx.build()
                .global("STARTUPCLK", "CCLK")
                .global("DONEACTIVE", "C1")
                .test_manual("STARTUP", "SYNC_TO_DONE", "YES")
                .global_diff("GSRINACTIVE", "C4", "DI_PLUS_1")
                .global_diff("OUTPUTSACTIVE", "C4", "DI_PLUS_1")
                .global_diff("SYNCTODONE", "NO", "YES")
                .commit();

            for val in ["C1", "C2", "C3", "C4"] {
                ctx.build()
                    .global("STARTUPCLK", "CCLK")
                    .global("SYNCTODONE", "NO")
                    .global("GSRINACTIVE", "C4")
                    .global("OUTPUTSACTIVE", "C4")
                    .test_manual("STARTUP", "DONE_ACTIVE", val)
                    .global_diff("DONEACTIVE", "C1", val)
                    .commit();
            }
            for val in ["U2", "U3", "U4"] {
                ctx.build()
                    .global("STARTUPCLK", "USERCLK")
                    .global("SYNCTODONE", "NO")
                    .global("GSRINACTIVE", "U3")
                    .global("OUTPUTSACTIVE", "U3")
                    .bel_out("STARTUP", "CLK")
                    .test_manual("STARTUP", "DONE_ACTIVE", val)
                    .global_diff("DONEACTIVE", "C1", val)
                    .commit();
            }
            for (attr, opt, oopt) in [
                ("OUTPUTS_ACTIVE", "OUTPUTSACTIVE", "GSRINACTIVE"),
                ("GSR_INACTIVE", "GSRINACTIVE", "OUTPUTSACTIVE"),
            ] {
                for val in ["C2", "C3", "C4"] {
                    ctx.build()
                        .global("STARTUPCLK", "CCLK")
                        .global("SYNCTODONE", "NO")
                        .global("DONEACTIVE", "C1")
                        .global(oopt, "C4")
                        .test_manual("STARTUP", attr, val)
                        .global_diff(opt, "C4", val)
                        .commit();
                }
                for val in ["U2", "U3", "U4"] {
                    ctx.build()
                        .global("STARTUPCLK", "USERCLK")
                        .global("SYNCTODONE", "NO")
                        .global("DONEACTIVE", "C1")
                        .global(oopt, "U3")
                        .bel_out("STARTUP", "CLK")
                        .test_manual("STARTUP", attr, val)
                        .global_diff(opt, "U3", val)
                        .commit();
                }
                for val in ["DI", "DI_PLUS_1", "DI_PLUS_2"] {
                    ctx.build()
                        .global("STARTUPCLK", "USERCLK")
                        .global("SYNCTODONE", "YES")
                        .global("DONEACTIVE", "C1")
                        .global(oopt, "DI_PLUS_1")
                        .bel_out("STARTUP", "CLK")
                        .test_manual("STARTUP", attr, val)
                        .global_diff(opt, "DI_PLUS_1", val)
                        .commit();
                }
            }

            let mut bctx = ctx.bel(bels::STARTUP);
            bctx.mode("STARTUP").test_cfg("GTS", "NOT");
            bctx.mode("STARTUP").test_cfg("GSR", "NOT");
        }
        if tile == "CNR.TR" {
            ctx.test_cfg4000("MISC", "TMRIGHT", &["OFF", "ON"]);
            ctx.test_cfg4000("MISC", "TAC", &["OFF", "ON"]);
            ctx.test_global("TDO", "TDOPIN", &["PULLDOWN", "PULLUP"]);
            ctx.test_global("READCLK", "READCLK", &["CCLK", "RDBK"]);
            let mut bctx = ctx.bel(bels::OSC);
            for out in ["OUT0", "OUT1"] {
                for pin in ["F500K", "F16K", "F490", "F15"] {
                    bctx.build()
                        .extra_tile(
                            (
                                DieId::from_idx(0),
                                grid.col_e(),
                                grid.row_s(),
                                LayerId::from_idx(0),
                            ),
                            "OSC",
                            format!("MUX.{out}"),
                            pin,
                        )
                        .mutex("MODE", "TEST")
                        .mutex("MUXOUT", out)
                        .mutex("MUXIN", pin)
                        .test_manual(format!("MUX.{out}"), pin)
                        .pip_pin(format!("{out}.{pin}"), pin)
                        .commit();
                }
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "LLV.IO.R");
        ctx.test_cfg4000("MISC", "TLC", &["OFF", "ON"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let grid = ctx.edev.chip;
    let num_dec = if grid.kind == ChipKind::Xc4000A { 2 } else { 4 };
    for tile in ["CNR.BL", "CNR.TL", "CNR.BR", "CNR.TR"] {
        for hv in ['H', 'V'] {
            for i in 0..num_dec {
                let bel = &format!("PULLUP_DEC{i}_{hv}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
            }
        }
        if tile == "CNR.BL" {
            let bel = "MISC";
            let item = ctx.extract_enum_bool(tile, bel, "TMBOT", "OFF", "ON");
            ctx.tiledb.insert(tile, bel, "TM_BOT", item);
            let item = ctx.extract_enum_bool(tile, bel, "READABORT", "DISABLE", "ENABLE");
            ctx.tiledb.insert(tile, bel, "READ_ABORT", item);
            let item = ctx.extract_enum_bool(tile, bel, "READCAPTURE", "DISABLE", "ENABLE");
            ctx.tiledb.insert(tile, bel, "READ_CAPTURE", item);
            let bel = "MD1";
            let item =
                ctx.extract_enum_default(tile, bel, "M1PIN", &["PULLUP", "PULLDOWN"], "PULLNONE");
            ctx.tiledb.insert(tile, bel, "PULL", item);
        }
        if tile == "CNR.TL" {
            let bel = "MISC";
            let item = ctx.extract_enum_bool(tile, bel, "TMTOP", "OFF", "ON");
            ctx.tiledb.insert(tile, bel, "TM_TOP", item);
            let item = ctx.extract_enum_bool(tile, bel, "TMLEFT", "OFF", "ON");
            ctx.tiledb.insert(tile, bel, "TM_LEFT", item);
            let item = xlat_enum(vec![
                ("CMOS", ctx.state.get_diff(tile, bel, "TTLBAR", "ON")),
                ("TTL", ctx.state.get_diff(tile, bel, "TTLBAR", "OFF")),
            ]);
            ctx.tiledb.insert(tile, bel, "INPUT", item);
            let bel = "BSCAN";
            let item = ctx.extract_bit(tile, bel, "BSCAN", "USED");
            ctx.tiledb.insert(tile, bel, "ENABLE", item);
        }
        if tile == "CNR.BR" {
            let bel = "MISC";
            ctx.collect_enum_bool(tile, bel, "TCTEST", "OFF", "ON");

            let bel = "STARTUP";
            ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
            let item = ctx.extract_enum(tile, bel, "CONFIGRATE", &["SLOW", "FAST"]);
            ctx.tiledb.insert(tile, bel, "CONFIG_RATE", item);
            let item = ctx.extract_bit(tile, bel, "GTS", "NOT");
            ctx.tiledb.insert(tile, bel, "INV.GTS", item);
            let item = ctx.extract_bit(tile, bel, "GSR", "NOT");
            ctx.tiledb.insert(tile, bel, "INV.GSR", item);
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
            ctx.collect_enum_default(tile, bel, "STARTUP_CLK", &["USERCLK"], "CCLK");
            ctx.collect_bit(tile, bel, "SYNC_TO_DONE", "YES");

            let bel = "DONE";
            let item = xlat_enum(vec![
                ("PULLUP", ctx.state.get_diff(tile, bel, "DONEPIN", "PULLUP")),
                (
                    "PULLNONE",
                    ctx.state.get_diff(tile, bel, "DONEPIN", "NOPULLUP"),
                ),
            ]);
            ctx.tiledb.insert(tile, bel, "PULL", item);

            let bel = "OSC";
            let mut diffs0 = vec![];
            let mut diffs1 = vec![];
            for val in ["F500K", "F16K", "F490", "F15"] {
                let diff0 = ctx.state.get_diff(tile, bel, "MUX.OUT0", val);
                let diff1 = ctx.state.get_diff(tile, bel, "MUX.OUT1", val);
                let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
                diffs0.push((val, diff0));
                diffs1.push((val, diff1));
                ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff_en));
            }
            ctx.tiledb.insert(tile, bel, "MUX.OUT0", xlat_enum(diffs0));
            ctx.tiledb.insert(tile, bel, "MUX.OUT1", xlat_enum(diffs1));
        }
        if tile == "CNR.TR" {
            let bel = "MISC";
            let item = ctx.extract_enum_bool(tile, bel, "TMRIGHT", "OFF", "ON");
            ctx.tiledb.insert(tile, bel, "TM_RIGHT", item);
            ctx.collect_enum_bool(tile, bel, "TAC", "OFF", "ON");
            let bel = "BSCAN";
            let item = ctx.extract_bit(tile, bel, "BSCAN", "USED");
            ctx.tiledb.insert(tile, bel, "ENABLE", item);
            let bel = "TDO";
            let item =
                ctx.extract_enum_default(tile, bel, "TDOPIN", &["PULLUP", "PULLDOWN"], "PULLNONE");
            ctx.tiledb.insert(tile, bel, "PULL", item);
            let bel = "READCLK";
            let item = ctx.extract_enum(tile, bel, "READCLK", &["RDBK", "CCLK"]);
            ctx.tiledb.insert(tile, bel, "READ_CLK", item);

            let bel = "OSC";
            for attr in ["MUX.OUT0", "MUX.OUT1"] {
                for val in ["F500K", "F16K", "F490", "F15"] {
                    ctx.state.get_diff(tile, bel, attr, val).assert_empty();
                }
            }
        }
    }
    {
        let tile = "LLV.IO.R";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TLC", "OFF", "ON");
    }
}
