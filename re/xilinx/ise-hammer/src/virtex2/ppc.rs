use prjcombine_interconnect::db::{BelInfo, PinDir};
use prjcombine_re_hammer::Session;
use prjcombine_virtex2::bels;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for tile in ["RBPPC", "LBPPC"] {
        let tcid = intdb.get_tile_class(tile);
        if backend.egrid.tile_index[tcid].is_empty() {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::PPC405);
        let mode = "PPC405";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        let bel_data = &intdb.tile_classes[tcid].bels[bels::PPC405];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if intdb.wires.key(wire.wire).starts_with("IMUX.G") {
                continue;
            }
            bctx.mode(mode).test_inv(pin);
        }
        bctx.mode(mode)
            .test_enum("PPC405_TEST_MODE", &["CORE_TEST", "GASKET_TEST"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    for tile in ["RBPPC", "LBPPC"] {
        let tcid = egrid.db.get_tile_class(tile);
        if egrid.tile_index[tcid].is_empty() {
            continue;
        }
        let bel = "PPC405";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        let bel_data = &egrid.db.tile_classes[tcid].bels[bels::PPC405];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if egrid.db.wires.key(wire.wire).starts_with("IMUX.G") {
                continue;
            }
            let int_tiles = &["INT.PPC"; 48];
            let flip = egrid.db.wires.key(wire.wire).starts_with("IMUX.SR");
            ctx.collect_int_inv(int_tiles, tile, bel, pin, flip);
        }
        ctx.state
            .get_diff(tile, bel, "PPC405_TEST_MODE", "CORE_TEST")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "PPC405_TEST_MODE", "GASKET_TEST")
            .assert_empty();
    }
}
