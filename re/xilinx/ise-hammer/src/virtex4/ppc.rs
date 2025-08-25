use prjcombine_interconnect::db::{BelInfo, PinDir};
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
    let intdb = backend.edev.db;
    let tile = "PPC";
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
        return;
    };
    let tcid = intdb.get_tile_class(tile);
    let tcls = &intdb[tcid];
    for (slot, bel_data) in &tcls.bels {
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
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
            if intdb.wires.key(wire.wire).starts_with("IMUX.IMUX") {
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
    let tile = "PPC";
    let tcid = ctx.edev.db.get_tile_class(tile);
    if !ctx.has_tile(tile) {
        return;
    }
    let tcls = &ctx.edev.db[tcid];
    for (slot, bel_data) in &tcls.bels {
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let bel = ctx.edev.db.bel_slots.key(slot);
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
            if ctx.edev.db.wires.key(wire.wire).starts_with("IMUX.IMUX") {
                continue;
            }
            let int_tiles = &["INT"; 62];
            ctx.collect_int_inv(int_tiles, tile, bel, pin, true);
        }
    }
}
