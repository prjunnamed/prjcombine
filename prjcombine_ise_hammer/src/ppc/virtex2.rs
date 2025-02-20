use prjcombine_hammer::Session;
use prjcombine_interconnect::db::{BelId, PinDir};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_inv,
    fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let intdb = backend.egrid.db;
    for tile in ["RBPPC", "LBPPC"] {
        let node_kind = intdb.get_node(tile);
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bel = BelId::from_idx(0);
        let ctx = FuzzCtx::new(session, backend, tile, "PPC405", TileBits::MainAuto);
        fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PPC405")]);
        let bel_data = &intdb.nodes[node_kind].bels[bel];
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if intdb.wires.key(wire.1).starts_with("IMUX.G") {
                continue;
            }
            fuzz_inv!(ctx, pin, [(mode "PPC405")]);
        }
        fuzz_enum!(ctx, "PPC405_TEST_MODE", ["CORE_TEST", "GASKET_TEST"], [(mode "PPC405")]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    for tile in ["RBPPC", "LBPPC"] {
        let node_kind = egrid.db.get_node(tile);
        if egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let bel = "PPC405";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        let bel_data = &egrid.db.nodes[node_kind].bels[BelId::from_idx(0)];
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if egrid.db.wires.key(wire.1).starts_with("IMUX.G") {
                continue;
            }
            let int_tiles = &["INT.PPC"; 48];
            let flip = egrid.db.wires.key(wire.1).starts_with("IMUX.SR");
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
