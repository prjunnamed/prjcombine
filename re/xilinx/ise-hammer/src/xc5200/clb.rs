use prjcombine_re_fpga_hammer::{Diff, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem};
use prjcombine_xc2000::bels::xc5200 as bels;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::FuzzCtx,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, "CLB");
    for i in 0..4 {
        let mut bctx = ctx.bel(bels::LC[i]);
        let mode = if i % 2 == 0 { "LC5A" } else { "LC5B" };
        bctx.mode(mode)
            .test_multi_attr("LUT", MultiValue::OldLut('F'), 16);
        bctx.mode(mode)
            .attr("DMUX", "F")
            .pin("DI")
            .test_manual("FFLATCH", "#LATCH")
            .attr("FFLATCH", "#LATCH")
            .pin("Q")
            .commit();
        if mode == "LC5A" {
            bctx.mode(mode)
                .pin("DO")
                .pin("DI")
                .test_enum("DOMUX", &["DI", "F5O", "CO"]);
        } else {
            bctx.mode(mode)
                .pin("DO")
                .pin("DI")
                .test_enum("DOMUX", &["DI", "CO"]);
        }
        bctx.mode(mode)
            .pin("DI")
            .attr("DOMUX", "DI")
            .test_enum("DMUX", &["F", "DO"]);
        bctx.mode(mode).pin("CE").test_enum("CEMUX", &["CE"]);
        bctx.mode(mode).pin("CLR").test_enum("CLRMUX", &["CLR"]);
        bctx.mode(mode)
            .pin("CK")
            .pin("DI")
            .pin("Q")
            .attr("DMUX", "F")
            .attr("FFLATCH", "")
            .test_enum("CKMUX", &["CK", "CKNOT"]);

        bctx.mode(mode)
            .pin("CK")
            .pin("DI")
            .pin("Q")
            .attr("DMUX", "F")
            .attr("FFLATCH", "#LATCH")
            .test_enum_suffix("CKMUX", "LATCH", &["CK", "CKNOT"]);
        bctx.mode(mode).pin("CO").test_enum("COMUX", &["CY"]);
    }
    for i in 0..4 {
        let mut bctx = ctx.bel(bels::TBUF[i]);
        bctx.mode("TBUF")
            .test_manual("TMUX", "T")
            .pin("T")
            .pin_pips("T")
            .commit();
    }
    let mut bctx = ctx.bel(bels::VCC_GND);
    bctx.mode("VCC_GND").test_enum("MUX", &["0", "1"]);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "CLB";
    for i in 0..4 {
        let bel = &format!("LC{i}");
        ctx.collect_bitvec(tile, bel, "LUT", "");
        if i % 2 == 0 {
            ctx.collect_enum(tile, bel, "DOMUX", &["DI", "F5O", "CO"]);
        } else {
            ctx.collect_enum(tile, bel, "DOMUX", &["DI", "CO"]);
        }
        let item = xlat_enum(vec![
            ("FF", Diff::default()),
            (
                "LATCH",
                ctx.state
                    .get_diff(tile, bel, "FFLATCH", "#LATCH")
                    .combine(&!ctx.state.peek_diff(tile, bel, "CKMUX", "CKNOT")),
            ),
        ]);
        ctx.tiledb.insert(tile, bel, "FFLATCH", item);
        ctx.collect_enum(tile, bel, "DMUX", &["F", "DO"]);
        ctx.collect_enum_default(tile, bel, "CLRMUX", &["CLR"], "NONE");
        ctx.collect_enum_default(tile, bel, "CEMUX", &["CE"], "NONE");
        let item = ctx.extract_enum_bool(tile, bel, "CKMUX", "CK", "CKNOT");
        ctx.tiledb.insert(tile, bel, "INV.CK", item);
        let item = ctx.extract_enum_bool(tile, bel, "CKMUX.LATCH", "CKNOT", "CK");
        ctx.tiledb.insert(tile, bel, "INV.CK", item);
        ctx.state.get_diff(tile, bel, "COMUX", "CY").assert_empty();
        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK",
            TileItem::from_bit(TileBit::new(0, 1, [5, 10, 23, 28][i]), true),
        );
    }
    for i in 0..4 {
        let bel = &format!("TBUF{i}");
        ctx.collect_enum_default(tile, bel, "TMUX", &["T"], "NONE");
    }
    let bel = "VCC_GND";
    ctx.collect_enum_bool(tile, bel, "MUX", "0", "1");
}
