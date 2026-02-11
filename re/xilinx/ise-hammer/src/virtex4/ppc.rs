use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelInfo, BelInputId},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::defs::{
    bcls, bslots,
    virtex4::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    virtex4::specials,
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

const EMAC_INVPINS: &[BelInputId] = &[
    bcls::EMAC_V4::CLIENTEMAC0RXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC0TXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC0TXGMIIMIICLKIN,
    bcls::EMAC_V4::CLIENTEMAC1RXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC1TXCLIENTCLKIN,
    bcls::EMAC_V4::CLIENTEMAC1TXGMIIMIICLKIN,
    bcls::EMAC_V4::HOSTCLK,
    bcls::EMAC_V4::PHYEMAC0GTXCLK,
    bcls::EMAC_V4::PHYEMAC0MCLKIN,
    bcls::EMAC_V4::PHYEMAC0MIITXCLK,
    bcls::EMAC_V4::PHYEMAC0RXCLK,
    bcls::EMAC_V4::PHYEMAC1GTXCLK,
    bcls::EMAC_V4::PHYEMAC1MCLKIN,
    bcls::EMAC_V4::PHYEMAC1MIITXCLK,
    bcls::EMAC_V4::PHYEMAC1RXCLK,
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    let tcid = tcls::PPC;
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
        return;
    };
    let tcls = &intdb[tcid];
    for (slot, bel_data) in &tcls.bels {
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let mut bctx = ctx.bel(slot);
        let mode = if slot == bslots::PPC {
            "PPC405_ADV"
        } else {
            "EMAC"
        };
        bctx.build()
            .test_bel_special(specials::PRESENT)
            .prop(ForceBitRects)
            .mode(mode)
            .commit();
        for (pin, wire) in &bel_data.inputs {
            if slot == bslots::EMAC
                && !EMAC_INVPINS.contains(&pin)
                && !bcls::EMAC_V4::TIEEMAC1UNICASTADDR.contains(pin)
            {
                continue;
            }
            if wires::IMUX_IMUX.contains(wire.wire().wire) {
                continue;
            }
            bctx.mode(mode)
                .prop(ForceBitRects)
                .test_bel_input_inv_auto(pin);
        }
        if slot == bslots::PPC {
            for (spec, val) in [
                (specials::PPC_SYNCBYPASS, "SYNCBYPASS"),
                (specials::PPC_SYNCACTIVE, "SYNCACTIVE"),
            ] {
                bctx.mode(mode)
                    .null_bits()
                    .test_bel_special(spec)
                    .attr("PLB_SYNC_MODE", val)
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::PPC;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let tcls = &ctx.edev.db[tcid];
    for (bslot, bel_data) in &tcls.bels {
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        if bslot == bslots::PPC {
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            for pin in bcls::PPC405::LSSDSCANIN {
                let bit = ctx.item_int_inv(&[tcls::INT; 62], tcid, bslot, pin);
                diff.discard_polbits(&[bit]);
            }
            diff.assert_empty();
        } else {
            ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT)
                .assert_empty();
        }
        for (pin, wire) in &bel_data.inputs {
            if bslot == bslots::EMAC
                && !EMAC_INVPINS.contains(&pin)
                && !bcls::EMAC_V4::TIEEMAC1UNICASTADDR.contains(pin)
            {
                continue;
            }
            if wires::IMUX_IMUX.contains(wire.wire().wire) {
                continue;
            }
            let int_tiles = &[tcls::INT; 62];
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, pin);
        }
    }
}
