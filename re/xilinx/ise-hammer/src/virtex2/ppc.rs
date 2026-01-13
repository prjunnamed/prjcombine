use prjcombine_interconnect::db::{BelInfo, PinDir};
use prjcombine_re_hammer::Session;
use prjcombine_virtex2::{defs, defs::virtex2::tcls};

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for tcid in [tcls::PPC_W, tcls::PPC_E] {
        let Some(mut ctx) = FuzzCtx::try_new_id(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(defs::bslots::PPC405);
        let mode = "PPC405";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
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
            bctx.mode(mode).test_inv(pin);
        }
        bctx.mode(mode)
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
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
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
            let flip = ctx.edev.db.wires.key(wire.wire).starts_with("IMUX_SR");
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
