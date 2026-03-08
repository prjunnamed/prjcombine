use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex::defs::{
    bcls::{TBUF, TBUS, TBUS_WE},
    bslots, tcls, tslots,
};

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
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let tbus = if tcid == tcls::CLB {
            bslots::TBUS
        } else {
            bslots::TBUS_WE
        };
        for (i, out_a, out_b) in [(0, "BUS0", "BUS2"), (1, "BUS1", "BUS3")] {
            let mut bctx = ctx.bel(bslots::TBUF[i]);
            for (val, vname) in [(false, "1"), (true, "0"), (false, "T"), (true, "T_B")] {
                bctx.mode("TBUF")
                    .pin("T")
                    .pin("O")
                    .test_bel_input_inv(TBUF::T, val)
                    .attr("TMUX", vname)
                    .commit();
            }
            for (val, vname) in [(false, "1"), (true, "0"), (false, "I"), (true, "I_B")] {
                bctx.mode("TBUF")
                    .pin("I")
                    .pin("O")
                    .test_bel_input_inv(TBUF::I, val)
                    .attr("IMUX", vname)
                    .commit();
            }
            bctx.build()
                .row_mutex_here("TBUF")
                .test_bel_attr_bits(TBUF::OUT_A)
                .pip((tbus, out_a), "O")
                .commit();
            bctx.build()
                .row_mutex_here("TBUF")
                .test_bel_attr_bits(TBUF::OUT_B)
                .pip((tbus, out_b), "O")
                .commit();
        }
    }
    {
        {
            let mut ctx = FuzzCtx::new(session, backend, tcls::IO_W);
            let mut bctx = ctx.bel(bslots::TBUS_WE);
            bctx.build()
                .row_mutex_here("TBUS")
                .test_bel_attr_bits(TBUS_WE::JOINER)
                .pip("BUS3_E", "BUS3")
                .commit();
            bctx.build()
                .row_mutex_here("TBUS")
                .test_bel_attr_bits(TBUS_WE::JOINER_E)
                .related_pip(
                    ClbTbusRight,
                    (bslots::TBUS, "BUS3_E"),
                    (bslots::TBUS, "BUS3"),
                )
                .commit();
        }
        {
            let mut ctx = FuzzCtx::new(session, backend, tcls::CLB);
            let mut bctx = ctx.bel(bslots::TBUS);
            bctx.build()
                .row_mutex_here("TBUS")
                .test_bel_attr_bits(TBUS::JOINER_E)
                .related_pip(ClbTbusRight, "BUS3_E", "BUS3")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::CLB, tcls::IO_W, tcls::IO_E] {
        for bslot in bslots::TBUF {
            ctx.collect_bel_input_inv_bi(tcid, bslot, TBUF::I);
            ctx.collect_bel_input_inv_bi(tcid, bslot, TBUF::T);
            ctx.collect_bel_attr(tcid, bslot, TBUF::OUT_A);
            ctx.collect_bel_attr(tcid, bslot, TBUF::OUT_B);
        }
    }
    {
        let tcid = tcls::IO_W;
        let bslot = bslots::TBUS_WE;
        ctx.collect_bel_attr(tcid, bslot, TBUS_WE::JOINER);
        ctx.collect_bel_attr(tcid, bslot, TBUS_WE::JOINER_E);
    }
    {
        let tcid = tcls::CLB;
        let bslot = bslots::TBUS;
        ctx.collect_bel_attr(tcid, bslot, TBUS::JOINER_E);
    }
}
