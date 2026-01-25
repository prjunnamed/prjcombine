use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelInfo, PinDir},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex2::{
    defs,
    defs::virtex2::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
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
        let Some(mut ctx) = FuzzCtx::try_new_id(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(defs::bslots::PPC405);
        let mode = "PPC405";
        bctx.build()
            .null_bits()
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();
        let bel_data = &intdb[tcid].bels[defs::bslots::PPC405];
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if intdb.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            bctx.mode(mode).prop(ForceBitRects).test_inv(pin);
        }
        bctx.mode(mode)
            .null_bits()
            .test_enum("PPC405_TEST_MODE", &["CORE_TEST", "GASKET_TEST"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["PPC_W", "PPC_E"] {
        let tcid = ctx.edev.db.get_tile_class(tile);
        if ctx.edev.tile_index[tcid].is_empty() {
            continue;
        }
        let bel = "PPC405";
        let bel_data = &ctx.edev.db[tcid].bels[defs::bslots::PPC405];
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if ctx.edev.db.wires.key(wire.wire).starts_with("IMUX_G") {
                continue;
            }
            let int_tiles = &["INT_PPC"; 48];
            let flip = wires::IMUX_CE.contains(wire.wire) || wires::IMUX_TI.contains(wire.wire);
            ctx.collect_int_inv(int_tiles, tile, bel, pin, flip);
        }
    }
}
