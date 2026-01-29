use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_collector::legacy::xlat_bit_bi_legacy;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex::defs::{bslots, tcls, tslots};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{fbuild::FuzzCtx, props::relation::TileRelation},
};

#[derive(Copy, Clone, Debug)]
struct ClbTbusRight;

impl TileRelation for ClbTbusRight {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let mut cell = tcrd.cell;
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        loop {
            if cell.col == edev.chip.col_e() {
                return None;
            }
            cell.col += 1;
            if edev.has_bel(cell.bel(bslots::SLICE[0])) {
                return Some(cell.tile(tslots::MAIN));
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::CLB, tcls::IO_W, tcls::IO_E] {
        let mut ctx = FuzzCtx::new_id(session, backend, tcid);
        for (i, out_a, out_b) in [(0, "BUS0", "BUS2"), (1, "BUS1", "BUS3")] {
            let mut bctx = ctx.bel(bslots::TBUF[i]);
            bctx.mode("TBUF")
                .pin("T")
                .pin("O")
                .test_enum("TMUX", &["0", "1", "T", "T_B"]);
            bctx.mode("TBUF")
                .pin("I")
                .pin("O")
                .test_enum("IMUX", &["0", "1", "I", "I_B"]);
            bctx.build()
                .row_mutex_here("TBUF")
                .test_manual_legacy("OUT_A", "1")
                .pip((bslots::TBUS, out_a), "O")
                .commit();
            bctx.build()
                .row_mutex_here("TBUF")
                .test_manual_legacy("OUT_B", "1")
                .pip((bslots::TBUS, out_b), "O")
                .commit();
        }
        let mut bctx = ctx.bel(bslots::TBUS);
        if tcid == tcls::IO_W {
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual_legacy("JOINER", "1")
                .pip("BUS3_E", "BUS3")
                .commit();
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual_legacy("JOINER_E", "1")
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
        } else if tcid == tcls::CLB {
            bctx.build()
                .row_mutex_here("TBUS")
                .test_manual_legacy("JOINER_E", "1")
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::CLB, tcls::IO_W, tcls::IO_E] {
        let tile = ctx.edev.db.tile_classes.key(tcid);
        for bel in ["TBUF[0]", "TBUF[1]"] {
            for (pinmux, pin, pin_b) in [("TMUX", "T", "T_B"), ("IMUX", "I", "I_B")] {
                let d0 = ctx.get_diff_legacy(tile, bel, pinmux, pin);
                assert_eq!(d0, ctx.get_diff_legacy(tile, bel, pinmux, "1"));
                let d1 = ctx.get_diff_legacy(tile, bel, pinmux, pin_b);
                assert_eq!(d1, ctx.get_diff_legacy(tile, bel, pinmux, "0"));
                let item = xlat_bit_bi_legacy(d0, d1);
                ctx.insert(tile, bel, format!("INV.{pin}"), item);
            }
            for attr in ["OUT_A", "OUT_B"] {
                ctx.collect_bit_legacy(tile, bel, attr, "1");
            }
        }
        let bel = "TBUS";
        if tile == "IO_W" {
            ctx.collect_bit_legacy(tile, bel, "JOINER", "1");
        }
        if tile != "IO_E" {
            ctx.collect_bit_legacy(tile, bel, "JOINER_E", "1");
        }
    }
}
