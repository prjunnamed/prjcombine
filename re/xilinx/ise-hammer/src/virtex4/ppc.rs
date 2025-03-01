use prjcombine_interconnect::db::PinDir;
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::bels;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const EMAC_INVPINS: &[&str] = &[
    "CLIENTEMAC0RXCLIENTCLKIN",
    "CLIENTEMAC0TXCLIENTCLKIN",
    "CLIENTEMAC0TXGMIIMIICLKIN",
    "CLIENTEMAC1RXCLIENTCLKIN",
    "CLIENTEMAC1TXCLIENTCLKIN",
    "CLIENTEMAC1TXGMIIMIICLKIN",
    "HOSTCLK",
    "PHYEMAC0GTXCLK",
    "PHYEMAC0MCLKIN",
    "PHYEMAC0MIITXCLK",
    "PHYEMAC0RXCLK",
    "PHYEMAC1GTXCLK",
    "PHYEMAC1MCLKIN",
    "PHYEMAC1MIITXCLK",
    "PHYEMAC1RXCLK",
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.egrid.db;
    let tile = "PPC";
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
        return;
    };
    let node_kind = intdb.get_node(tile);
    let node_data = &intdb.nodes[node_kind];
    for (slot, bel_data) in &node_data.bels {
        let mut bctx = ctx.bel(slot);
        let mode = if slot == bels::PPC {
            "PPC405_ADV"
        } else {
            "EMAC"
        };
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            if slot == bels::EMAC
                && !EMAC_INVPINS.contains(&&pin[..])
                && !pin.starts_with("TIEEMAC1UNICASTADDR")
            {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if intdb.wires.key(wire.1).starts_with("IMUX.IMUX") {
                continue;
            }
            bctx.mode(mode).test_inv(pin);
        }
        if slot == bels::PPC {
            bctx.mode(mode)
                .test_enum("PLB_SYNC_MODE", &["SYNCBYPASS", "SYNCACTIVE"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let tile = "PPC";
    let node_kind = egrid.db.get_node(tile);
    if egrid.node_index[node_kind].is_empty() {
        return;
    }
    let node_data = &egrid.db.nodes[node_kind];
    for (slot, bel_data) in &node_data.bels {
        let bel = &egrid.db.bel_slots[slot];
        if slot == bels::PPC {
            let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            for pin in bel_data.pins.keys() {
                if pin.starts_with("LSSDSCANIN") {
                    let item = ctx.item_int_inv(&["INT"; 62], tile, bel, pin);
                    diff.discard_bits(&item);
                }
            }
            diff.assert_empty();
            ctx.state
                .get_diff(tile, bel, "PLB_SYNC_MODE", "SYNCACTIVE")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "PLB_SYNC_MODE", "SYNCBYPASS")
                .assert_empty();
        } else {
            ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        }
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            if slot == bels::EMAC
                && !EMAC_INVPINS.contains(&&pin[..])
                && !pin.starts_with("TIEEMAC1UNICASTADDR")
            {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if egrid.db.wires.key(wire.1).starts_with("IMUX.IMUX") {
                continue;
            }
            let int_tiles = &["INT"; 62];
            ctx.collect_int_inv(int_tiles, tile, bel, pin, true);
        }
    }
}
