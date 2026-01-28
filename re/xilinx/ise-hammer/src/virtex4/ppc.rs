use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelInfo, PinDir},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::defs::{
    bslots,
    virtex4::{tcls, wires},
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
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        fuzzer.info.features[0].rects = EntityVec::from_iter(
            tile.cells
                .values()
                .map(|&cell| edev.btile_main(cell.die, cell.col, cell.row)),
        );
        Some((fuzzer, false))
    }
}

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
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        let mut bctx = ctx.bel(slot);
        let mode = if slot == bslots::PPC {
            "PPC405_ADV"
        } else {
            "EMAC"
        };
        bctx.test_manual("PRESENT", "1")
            .prop(ForceBitRects)
            .mode(mode)
            .commit();
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            if slot == bslots::EMAC
                && !EMAC_INVPINS.contains(&&pin[..])
                && !pin.starts_with("TIEEMAC1UNICASTADDR")
            {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if wires::IMUX_IMUX.contains(wire.wire) {
                continue;
            }
            bctx.mode(mode).prop(ForceBitRects).test_inv(pin);
        }
        if slot == bslots::PPC {
            bctx.mode(mode)
                .null_bits()
                .test_enum("PLB_SYNC_MODE", &["SYNCBYPASS", "SYNCACTIVE"]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "PPC";
    let tcid = tcls::PPC;
    if !ctx.has_tile_id(tcid) {
        return;
    }
    let tcls = &ctx.edev.db[tcid];
    for (bslot, bel_data) in &tcls.bels {
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        let bel = ctx.edev.db.bel_slots.key(bslot);
        if bslot == bslots::PPC {
            let mut diff = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
            for pin in bel_data.pins.keys() {
                if pin.starts_with("LSSDSCANIN") {
                    let item = ctx.item_int_inv(&[tcls::INT; 62], tcid, bslot, pin);
                    diff.discard_bits(&[item.bit]);
                }
            }
            diff.assert_empty();
        } else {
            ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
                .assert_empty();
        }
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            if bslot == bslots::EMAC
                && !EMAC_INVPINS.contains(&&pin[..])
                && !pin.starts_with("TIEEMAC1UNICASTADDR")
            {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if wires::IMUX_IMUX.contains(wire.wire) {
                continue;
            }
            let int_tiles = &[tcls::INT; 62];
            ctx.collect_int_inv_legacy(int_tiles, tcid, bslot, pin, true);
        }
    }
}
