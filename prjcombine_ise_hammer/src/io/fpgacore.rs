use prjcombine_collector::{xlat_bit, xlat_bit_wide, xlat_bool, Diff};
use prjcombine_hammer::Session;
use prjcombine_interconnect::db::Dir;
use prjcombine_types::tiledb::{TileBit, TileItem};

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_one, fuzz_one_extras,
};

fn make_iob_extras(bel: &str, attr: &str, val: &str) -> Vec<ExtraFeature> {
    [
        ("IOBS.FC.L", Dir::W),
        ("IOBS.FC.R", Dir::E),
        ("IOBS.FC.B", Dir::S),
        ("IOBS.FC.T", Dir::N),
    ]
    .into_iter()
    .map(|(tile, dir)| ExtraFeature::new(ExtraFeatureKind::FpgaCoreIob(dir), tile, bel, attr, val))
    .collect()
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let tile = "IOI.FC";
    for i in 0..4 {
        let bel = &format!("IBUF{i}");
        let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::MainAuto);
        fuzz_one_extras!(ctx, "ENABLE", "1", [
        ], [
            (mode "IBUF")
        ], make_iob_extras(bel, "ENABLE", "1"));
        fuzz_one_extras!(ctx, "ENABLE_O2IPADPATH", "1", [
            (mode "IBUF")
        ], [
            (attr "ENABLE_O2IPADPATH", "ENABLE_O2IPADPATH")
        ], make_iob_extras(bel, "ENABLE_O2IPADPATH", "1"));
        fuzz_one!(ctx, "ENABLE_O2IPATH", "1", [
            (mode "IBUF"),
            (attr "ENABLE_O2IQPATH", "")
        ], [
            (attr "ENABLE_O2IPATH", "ENABLE_O2IPATH")
        ]);
        fuzz_one!(ctx, "ENABLE_O2IQPATH", "1", [
            (mode "IBUF"),
            (attr "ENABLE_O2IPATH", "")
        ], [
            (attr "ENABLE_O2IQPATH", "ENABLE_O2IQPATH")
        ]);
        fuzz_enum!(ctx, "IMUX", ["0", "1"], [
            (mode "IBUF"),
            (attr "IFFDMUX", "1"),
            (attr "IFF", "#FF"),
            (pin "I"),
            (pin "IQ")
        ]);
        fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
            (mode "IBUF"),
            (attr "IMUX", "1"),
            (attr "IFF", "#FF"),
            (pin "I"),
            (pin "IQ")
        ]);
        fuzz_enum!(ctx, "IFF", ["#FF", "#LATCH"], [
            (mode "IBUF"),
            (attr "IFFDMUX", "1"),
            (attr "IFF_INIT_ATTR", "INIT1"),
            (attr "CEINV", "CE_B"),
            (pin "IQ"),
            (pin "CE")
        ]);
        fuzz_enum!(ctx, "IFFATTRBOX", ["SYNC", "ASYNC"], [
            (mode "IBUF"),
            (attr "IFF", "#FF"),
            (attr "IFFDMUX", "1"),
            (pin "IQ")
        ]);
        fuzz_enum!(ctx, "IFF_INIT_ATTR", ["INIT0", "INIT1"], [
            (mode "IBUF"),
            (attr "IFF", "#FF"),
            (attr "IFFDMUX", "1"),
            (attr "IFF_SR_ATTR", "SRLOW"),
            (pin "IQ")
        ]);
        fuzz_enum!(ctx, "IFF_SR_ATTR", ["SRLOW", "SRHIGH"], [
            (mode "IBUF"),
            (attr "IFF", "#FF"),
            (attr "IFFDMUX", "1"),
            (attr "IFF_INIT_ATTR", "INIT0"),
            (pin "IQ")
        ]);

        for pin in ["CLK", "CE", "SR", "REV"] {
            fuzz_inv!(ctx, pin, [(mode "IBUF"), (pin "IQ"), (attr "IFF", "#FF")]);
        }
    }
    for i in 0..4 {
        let bel = &format!("OBUF{i}");
        let ctx = FuzzCtx::new(
            session,
            backend,
            tile,
            format!("OBUF{i}"),
            TileBits::MainAuto,
        );
        fuzz_one_extras!(ctx, "ENABLE", "1", [
        ], [
            (mode "OBUF"),
            (attr "ENABLE_MISR", "FALSE")
        ], make_iob_extras(bel, "ENABLE", "1"));
        fuzz_one_extras!(ctx, "ENABLE_MISR", "TRUE", [
            (mode "OBUF")
        ], [
            (attr_diff "ENABLE_MISR", "FALSE", "TRUE")
        ], make_iob_extras(bel, "ENABLE_MISR", "TRUE"));
        for pin in ["CLK", "CE", "SR", "REV", "O"] {
            fuzz_inv!(ctx, pin, [(mode "OBUF"), (attr "OMUX", "OFF"), (attr "OFF", "#FF")]);
        }
        fuzz_enum!(ctx, "OFF", ["#FF", "#LATCH"], [
            (mode "OBUF"),
            (attr "OINV", "O"),
            (attr "OFF_INIT_ATTR", "INIT1"),
            (attr "CEINV", "CE_B"),
            (pin "O"),
            (pin "CE")
        ]);
        fuzz_enum!(ctx, "OFFATTRBOX", ["SYNC", "ASYNC"], [
            (mode "OBUF"),
            (attr "OFF", "#FF"),
            (attr "OINV", "O"),
            (pin "O")
        ]);
        fuzz_enum!(ctx, "OFF_INIT_ATTR", ["INIT0", "INIT1"], [
            (mode "OBUF"),
            (attr "OFF", "#FF"),
            (attr "OINV", "O"),
            (attr "OFF_SR_ATTR", "SRLOW"),
            (pin "O")
        ]);
        fuzz_enum!(ctx, "OFF_SR_ATTR", ["SRLOW", "SRHIGH"], [
            (mode "OBUF"),
            (attr "OFF", "#FF"),
            (attr "OINV", "O"),
            (attr "OFF_INIT_ATTR", "INIT0"),
            (pin "O")
        ]);
        fuzz_enum!(ctx, "OMUX", ["O", "OFF"], [
            (mode "OBUF"),
            (attr "OFF", "#FF"),
            (attr "OINV", "O"),
            (pin "O")
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for i in 0..4 {
        let tile = "IOI.FC";
        let bel = &format!("IBUF{i}");
        ctx.state.get_diff(tile, bel, "ENABLE", "1").assert_empty();
        ctx.state
            .get_diff(tile, bel, "ENABLE_O2IPADPATH", "1")
            .assert_empty();
        let diff_i = ctx.state.get_diff(tile, bel, "ENABLE_O2IPATH", "1");
        let diff_iq = ctx.state.get_diff(tile, bel, "ENABLE_O2IQPATH", "1");
        let (diff_i, diff_iq, diff_common) = Diff::split(diff_i, diff_iq);
        ctx.tiledb
            .insert(tile, bel, "ENABLE_O2IPATH", xlat_bit(diff_i));
        ctx.tiledb
            .insert(tile, bel, "ENABLE_O2IQPATH", xlat_bit(diff_iq));
        ctx.tiledb
            .insert(tile, bel, "ENABLE_O2I_O2IQ_PATH", xlat_bit(diff_common));
        for pin in ["CLK", "CE"] {
            ctx.collect_inv(tile, bel, pin);
        }
        for pin in ["REV", "SR"] {
            let d0 = ctx.state.get_diff(tile, bel, format!("{pin}INV"), pin);
            let d1 = ctx
                .state
                .get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"));
            let (d0, d1, de) = Diff::split(d0, d1);
            ctx.tiledb
                .insert(tile, bel, format!("INV.{pin}"), xlat_bool(d0, d1));
            ctx.tiledb
                .insert(tile, bel, format!("FF_{pin}_ENABLE"), xlat_bit(de));
        }
        ctx.state.get_diff(tile, bel, "IMUX", "1").assert_empty();
        ctx.state.get_diff(tile, bel, "IFFDMUX", "1").assert_empty();
        let diff_i = ctx.state.get_diff(tile, bel, "IMUX", "0");
        let diff_iff = ctx.state.get_diff(tile, bel, "IFFDMUX", "0");
        let (diff_i, diff_iff, diff_common) = Diff::split(diff_i, diff_iff);
        ctx.tiledb
            .insert(tile, bel, "I_DELAY_ENABLE", xlat_bit(diff_i));
        ctx.tiledb
            .insert(tile, bel, "IFF_DELAY_ENABLE", xlat_bit(diff_iff));
        ctx.tiledb
            .insert(tile, bel, "DELAY_ENABLE", xlat_bit_wide(diff_common));
        let item = ctx.extract_enum_bool(tile, bel, "IFF", "#FF", "#LATCH");
        ctx.tiledb.insert(tile, bel, "FF_LATCH", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFATTRBOX", "ASYNC", "SYNC");
        ctx.tiledb.insert(tile, bel, "FF_SR_SYNC", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFF_INIT_ATTR", "INIT0", "INIT1");
        ctx.tiledb.insert(tile, bel, "FF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFF_SR_ATTR", "SRLOW", "SRHIGH");
        ctx.tiledb.insert(tile, bel, "FF_SRVAL", item);
        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit(TileBit::new(0, 3, [0, 31, 32, 63][i]), false),
        );
        for tile in ["IOBS.FC.B", "IOBS.FC.T", "IOBS.FC.L", "IOBS.FC.R"] {
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE_O2IPADPATH", "1");
        }
    }
    for i in 0..4 {
        let tile = "IOI.FC";
        let bel = &format!("OBUF{i}");
        ctx.state.get_diff(tile, bel, "ENABLE", "1").assert_empty();
        ctx.state
            .get_diff(tile, bel, "ENABLE_MISR", "TRUE")
            .assert_empty();
        for pin in ["CLK", "O"] {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_int_inv(&["INT.IOI.FC"], tile, bel, "CE", false);
        for pin in ["REV", "SR"] {
            let d0 = ctx.state.get_diff(tile, bel, format!("{pin}INV"), pin);
            let d1 = ctx
                .state
                .get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"));
            let (d0, d1, de) = Diff::split(d0, d1);
            if pin == "REV" {
                ctx.tiledb
                    .insert(tile, bel, format!("INV.{pin}"), xlat_bool(d0, d1));
            } else {
                ctx.insert_int_inv(&["INT.IOI.FC"], tile, bel, pin, xlat_bool(d0, d1));
            }
            ctx.tiledb
                .insert(tile, bel, format!("FF_{pin}_ENABLE"), xlat_bit(de));
        }
        let item = ctx.extract_enum_bool(tile, bel, "OFF", "#FF", "#LATCH");
        ctx.tiledb.insert(tile, bel, "FF_LATCH", item);
        let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "ASYNC", "SYNC");
        ctx.tiledb.insert(tile, bel, "FF_SR_SYNC", item);
        let item = ctx.extract_enum_bool(tile, bel, "OFF_INIT_ATTR", "INIT0", "INIT1");
        ctx.tiledb.insert(tile, bel, "FF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "OFF_SR_ATTR", "SRLOW", "SRHIGH");
        ctx.tiledb.insert(tile, bel, "FF_SRVAL", item);
        ctx.collect_enum_default(tile, bel, "OMUX", &["O", "OFF"], "NONE");
        for tile in ["IOBS.FC.B", "IOBS.FC.T", "IOBS.FC.L", "IOBS.FC.R"] {
            ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE_MISR", "TRUE");
        }
    }
}
