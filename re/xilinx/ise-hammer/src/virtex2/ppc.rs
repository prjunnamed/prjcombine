use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelInfo, BelInput},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex2::defs::{bslots, virtex2::tcls};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    virtex2::specials,
};

#[derive(Clone, Debug)]
struct ForceBitRects;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ForceBitRects {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(ForceBitRects)
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let tile = &backend.edev[tcrd];
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        fuzzer.info.features[0].rects =
            EntityVec::from_iter(tile.cells.values().map(|&cell| edev.btile_main(cell)));
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for tcid in [tcls::PPC_W, tcls::PPC_E] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::PPC405);
        let mode = "PPC405";
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        let bel_data = &intdb[tcid].bels[bslots::PPC405];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        for (pid, &inp) in &bel_data.inputs {
            let BelInput::Fixed(wire) = inp else {
                unreachable!()
            };
            if intdb.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            bctx.mode(mode)
                .prop(ForceBitRects)
                .test_bel_input_inv_auto(pid);
        }
        for (spec, val) in [
            (specials::PPC_CORE_TEST, "CORE_TEST"),
            (specials::PPC_GASKET_TEST, "GASKET_TEST"),
        ] {
            bctx.mode(mode)
                .null_bits()
                .test_bel_special(spec)
                .attr("PPC405_TEST_MODE", val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tcid in [tcls::PPC_W, tcls::PPC_E] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::PPC405;
        let bel_data = &ctx.edev.db[tcid].bels[bslot];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        for (pid, &inp) in &bel_data.inputs {
            let BelInput::Fixed(wire) = inp else {
                unreachable!()
            };
            if ctx.edev.db.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            let int_tiles = &[tcls::INT_PPC; 48];
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pid);
        }
    }
}
