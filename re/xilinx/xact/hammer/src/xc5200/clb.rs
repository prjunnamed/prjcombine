use prjcombine_re_collector::xlat_enum;
use prjcombine_re_hammer::Session;

use crate::{backend::XactBackend, collector::CollectorCtx, fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "CLB");
    let mut bctx = ctx.bel("LC0");
    bctx.mode("CLB").test_cfg("CO", "USED");
    bctx.mode("CLB").test_enum("CV", &["GND", "VCC"]);
    bctx.mode("CLB")
        .test_enum("RDBK", &["LC0.QX", "LC1.QX", "LC2.QX", "LC3.QX"]);
    for i in 0..4 {
        bctx.mode("CLB").test_equate(
            format!("LC{i}.F"),
            match i {
                0 => &["LC0.F1", "LC0.F2", "LC0.F3", "LC0.F4"],
                1 => &["LC1.F1", "LC1.F2", "LC1.F3", "LC1.F4"],
                2 => &["LC2.F1", "LC2.F2", "LC2.F3", "LC2.F4"],
                3 => &["LC3.F1", "LC3.F2", "LC3.F3", "LC3.F4"],
                _ => unreachable!(),
            },
        );
        bctx.mode("CLB")
            .test_cfg(format!("LC{i}.DO"), format!("LC{i}.XBI"));
        bctx.mode("CLB")
            .test_cfg(format!("LC{i}.X"), format!("LC{i}.F"));
        bctx.mode("CLB").test_enum(
            format!("LC{i}.DX"),
            &[format!("LC{i}.F"), format!("LC{i}.XBI")],
        );
        bctx.mode("CLB").test_cfg(format!("LC{i}.FFX"), "CLR");
        bctx.mode("CLB").test_cfg(format!("LC{i}.FFX"), "CE");
        bctx.mode("CLB")
            .mutex(format!("LC{i}.FFX"), "FF")
            .test_cfg(format!("LC{i}.FFX"), "NOTK");
        bctx.mode("CLB")
            .test_enum(format!("LC{i}.FFX"), &["FF", "LATCH"]);
        if matches!(i, 0 | 2) {
            bctx.mode("CLB").test_enum(
                format!("LC{i}.XBI"),
                &[
                    format!("LC{i}.DI"),
                    format!("LC{i}.CARRY"),
                    format!("LC{i}{ii}.F5", ii = i + 1),
                ],
            );
        } else {
            bctx.mode("CLB").test_enum(
                format!("LC{i}.XBI"),
                &[format!("LC{i}.DI"), format!("LC{i}.CARRY")],
            );
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CLB";
    let sbel = "LC0";
    for i in 0..4 {
        let bel = &format!("LC{i}");
        let item = ctx.extract_bitvec(tile, sbel, &format!("LC{i}.F"), "");
        ctx.tiledb.insert(tile, bel, "LUT", item);
        let item = ctx.extract_bit(tile, sbel, &format!("LC{i}.FFX"), "NOTK");
        ctx.tiledb.insert(tile, bel, "INV.CK", item);
        let diff_ff = ctx.state.get_diff(tile, sbel, format!("LC{i}.FFX"), "FF");
        let mut diff_latch = ctx
            .state
            .get_diff(tile, sbel, format!("LC{i}.FFX"), "LATCH");
        diff_latch.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.CK"), true, false);
        ctx.tiledb.insert(
            tile,
            bel,
            "FFLATCH",
            xlat_enum(vec![("FF", diff_ff), ("LATCH", diff_latch)]),
        );
        let item = ctx.extract_enum_default(tile, sbel, &format!("LC{i}.FFX"), &["CE"], "NONE");
        ctx.tiledb.insert(tile, bel, "CEMUX", item);
        let item = ctx.extract_enum_default(tile, sbel, &format!("LC{i}.FFX"), &["CLR"], "NONE");
        ctx.tiledb.insert(tile, bel, "CLRMUX", item);
        ctx.state
            .get_diff(tile, sbel, format!("LC{i}.DO"), format!("LC{i}.XBI"))
            .assert_empty();
        ctx.state
            .get_diff(tile, sbel, format!("LC{i}.X"), format!("LC{i}.F"))
            .assert_empty();
        let item = xlat_enum(vec![
            (
                "F",
                ctx.state
                    .get_diff(tile, sbel, format!("LC{i}.DX"), format!("LC{i}.F")),
            ),
            (
                "DO",
                ctx.state
                    .get_diff(tile, sbel, format!("LC{i}.DX"), format!("LC{i}.XBI")),
            ),
        ]);
        ctx.tiledb.insert(tile, bel, "DMUX", item);
        let mut diffs = vec![
            (
                "DI",
                ctx.state
                    .get_diff(tile, sbel, format!("LC{i}.XBI"), format!("LC{i}.DI")),
            ),
            (
                "CO",
                ctx.state
                    .get_diff(tile, sbel, format!("LC{i}.XBI"), format!("LC{i}.CARRY")),
            ),
        ];
        if matches!(i, 0 | 2) {
            diffs.push((
                "F5O",
                ctx.state.get_diff(
                    tile,
                    sbel,
                    format!("LC{i}.XBI"),
                    format!("LC{i}{ii}.F5", ii = i + 1),
                ),
            ));
        }
        ctx.tiledb.insert(tile, bel, "DOMUX", xlat_enum(diffs));
        let item = ctx.extract_bit(tile, sbel, "RDBK", &format!("LC{i}.QX"));
        ctx.tiledb.insert(tile, bel, "READBACK", item);
    }
    ctx.state.get_diff(tile, sbel, "CO", "USED").assert_empty();
    let item = ctx.extract_enum_bool(tile, sbel, "CV", "GND", "VCC");
    ctx.tiledb.insert(tile, "VCC_GND", "MUX", item);
}
