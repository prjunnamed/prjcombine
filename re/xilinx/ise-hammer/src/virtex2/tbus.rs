use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex2::defs::{bcls, bslots, tslots, virtex2::tcls};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{fbuild::FuzzCtx, props::relation::TileRelation},
};

#[derive(Copy, Clone, Debug)]
struct ClbTbusRight;

impl TileRelation for ClbTbusRight {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        let mut cell = tcrd.cell;
        loop {
            if cell.col == edev.chip.col_e() {
                return None;
            }
            cell.col += 1;
            if cell.col == edev.chip.col_e() - 1 {
                return None;
            }
            if backend.edev.has_bel(cell.bel(bslots::SLICE[0])) {
                return Some(cell.tile(tslots::BEL));
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CLB);
    for (i, out_a, out_b) in [(0, "BUS0", "BUS2"), (1, "BUS1", "BUS3")] {
        let mut bctx = ctx.bel(bslots::TBUF[i]);
        bctx.mode("TBUF")
            .pin("O")
            .test_bel_input_inv_auto(bcls::TBUF::T);
        bctx.mode("TBUF")
            .pin("O")
            .test_bel_input_inv_auto(bcls::TBUF::I);
        bctx.build()
            .row_mutex_here("TBUF")
            .test_bel_attr_bits(bcls::TBUF::OUT_A)
            .pip((bslots::TBUS, out_a), "O")
            .commit();
        bctx.build()
            .row_mutex_here("TBUF")
            .test_bel_attr_bits(bcls::TBUF::OUT_B)
            .pip((bslots::TBUS, out_b), "O")
            .commit();
    }
    let mut bctx = ctx.bel(bslots::TBUS);
    bctx.build()
        .row_mutex_here("TBUS")
        .test_bel_attr_bits(bcls::TBUS::JOINER_E)
        .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
        .commit();
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CLB;
    for bslot in bslots::TBUF {
        let int_tcid = &[tcls::INT_CLB];
        ctx.collect_bel_input_inv_int_bi(int_tcid, tcid, bslot, bcls::TBUF::T);
        ctx.collect_bel_input_inv_int_bi(int_tcid, tcid, bslot, bcls::TBUF::I);
        for attr in [bcls::TBUF::OUT_A, bcls::TBUF::OUT_B] {
            ctx.collect_bel_attr(tcid, bslot, attr);
        }
    }
    let bslot = bslots::TBUS;
    ctx.collect_bel_attr(tcid, bslot, bcls::TBUS::JOINER_E);
}
