use prjcombine_interconnect::{
    db::{BelKind, TileWireCoord},
    dir::DirHV,
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_enum_attr, xlat_enum_raw};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::PolTileBit;
use prjcombine_xc2000::xc3000::{bcls, bslots, enums, tslots, wires};

use crate::{
    backend::{Key, Value, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
    props::BaseBelNoConfig,
    specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for slot in tcls.bels.ids() {
            if backend.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
                continue;
            }
            let mut bctx = ctx.bel(slot);
            bctx.build()
                .null_bits()
                .mode("IO")
                .cfg("IN", "I")
                .test_bel_special(specials::IO_IQ)
                .cfg("IN", "IQ")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .test_bel_attr_as("IN", bcls::IO::IFF_MODE);
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "GND")
                .test_bel_attr_val(bcls::IO::MUX_O, enums::IO_MUX_O::OQ)
                .cfg_diff("OUT", "O", "OQ")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "GND")
                .cfg("OUT", "O")
                .test_bel_input_inv(bcls::IO::O, true)
                .cfg("OUT", "NOT")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "GND")
                .cfg("OUT", "O")
                .test_bel_attr_val(bcls::IO::SLEW, enums::IO_SLEW::FAST)
                .cfg("OUT", "FAST")
                .commit();
            bctx.mode("IO")
                .cfg("IN", "I")
                .mutex("TRI", "T")
                .cfg("OUT", "O")
                .cfg("TRI", "T")
                .test_bel_input_inv(bcls::IO::T, true)
                .cfg("TRI", "NOT")
                .commit();
        }
        if tcls.bels.contains_id(bslots::MISC_SE) {
            let mut bctx = ctx.bel(bslots::MISC_SE);
            bctx.test_attr_global_enum_bool_as(
                "DONEPAD",
                bcls::MISC_SE::DONE_PULLUP,
                "NOPULLUP",
                "PULLUP",
            );
            bctx.test_attr_global_enum_bool_as(
                "REPROGRAM",
                bcls::MISC_SE::REPROGRAM_ENABLE,
                "DISABLE",
                "ENABLE",
            );
            bctx.test_attr_global(bcls::MISC_SE::DONETIME);
            bctx.test_attr_global(bcls::MISC_SE::RESETTIME);

            let mut bctx = ctx.bel(bslots::OSC);
            let tcrd = backend.edev.chip.corner(DirHV::SE);
            let wt = TileWireCoord::new_idx(0, wires::IMUX_BUFG);
            let wf = TileWireCoord::new_idx(0, wires::OUT_OSC);
            let crd = backend.ngrid.int_pip(tcrd, wt, wf);
            let rwt = backend.edev.resolve_tile_wire(tcrd, wt).unwrap();
            let rwf = backend.edev.resolve_tile_wire(tcrd, wf).unwrap();
            for (vid, val) in [
                (enums::OSC_MODE::ENABLE, "ENABLE"),
                (enums::OSC_MODE::DIV2, "DIV2"),
            ] {
                bctx.build()
                    .raw(Key::WireMutex(rwt), "OSC_SPECIAL")
                    .raw(Key::WireMutex(rwf), "OSC_SPECIAL")
                    .test_bel_attr_val(bcls::OSC::MODE, vid)
                    .global_diff("XTALOSC", "DISABLE", val)
                    .raw_diff(Key::Pip(crd), None, Value::FromPin("OSC", "O".into()))
                    .raw_diff(
                        Key::BlockPin("ACLK", "I".into()),
                        None,
                        Value::FromPin("OSC", "O".into()),
                    )
                    .prop(BaseBelNoConfig::new(
                        bslots::IO_S[1],
                        "IN".into(),
                        "I".into(),
                    ))
                    .prop(BaseBelNoConfig::new(
                        bslots::IO_E[0],
                        "IN".into(),
                        "I".into(),
                    ))
                    .commit();
            }
        }
        if tcls.bels.contains_id(bslots::MISC_SW) {
            let mut bctx = ctx.bel(bslots::MISC_SW);
            bctx.test_attr_global_as("READ", bcls::MISC_SW::READBACK_MODE);
        }
        if tcls.bels.contains_id(bslots::MISC_NW) {
            let mut bctx = ctx.bel(bslots::MISC_NW);
            bctx.test_attr_global_as("INPUT", bcls::MISC_NW::IO_INPUT_MODE);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if tcls.slot != tslots::MAIN {
            continue;
        }
        if !ctx.has_tile(tcid) {
            continue;
        }
        for slot in tcls.bels.ids() {
            if ctx.edev.db.bel_slots[slot].kind != BelKind::Class(bcls::IO) {
                continue;
            }
            ctx.collect_bel_input_inv(tcid, slot, bcls::IO::T);
            ctx.collect_bel_input_inv(tcid, slot, bcls::IO::O);
            ctx.collect_bel_attr_default(tcid, slot, bcls::IO::SLEW, enums::IO_SLEW::SLOW);
            ctx.collect_bel_attr_default(tcid, slot, bcls::IO::MUX_O, enums::IO_MUX_O::O);
            ctx.collect_bel_attr(tcid, slot, bcls::IO::IFF_MODE);
        }
        if tcls.bels.contains_id(bslots::MISC_SE) {
            let diff0 = ctx.get_diff_raw(&DiffKey::BelAttrBit(
                tcid,
                bslots::MISC_SE,
                bcls::MISC_SE::REPROGRAM_ENABLE,
                0,
                false,
            ));
            let diff1 = ctx.get_diff_raw(&DiffKey::BelAttrBit(
                tcid,
                bslots::MISC_SE,
                bcls::MISC_SE::REPROGRAM_ENABLE,
                0,
                true,
            ));
            let mut item = xlat_enum_raw(vec![(false, diff0), (true, diff1)], OcdMode::BitOrder);
            let bits = Vec::from_iter(
                item.bits
                    .into_iter()
                    .zip(item.values.remove(&false).unwrap())
                    .map(|(bit, inv)| PolTileBit { bit, inv }),
            );
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslots::MISC_SE,
                bcls::MISC_SE::REPROGRAM_ENABLE,
                bits,
            );
            ctx.collect_bel_attr_bi(tcid, bslots::MISC_SE, bcls::MISC_SE::DONE_PULLUP);
            ctx.collect_bel_attr(tcid, bslots::MISC_SE, bcls::MISC_SE::DONETIME);
            ctx.collect_bel_attr(tcid, bslots::MISC_SE, bcls::MISC_SE::RESETTIME);
            let mut diffs = vec![(enums::OSC_MODE::DISABLE, Diff::default())];
            for val in [enums::OSC_MODE::ENABLE, enums::OSC_MODE::DIV2] {
                let mut diff = ctx.get_diff_raw(&DiffKey::BelAttrValue(
                    tcid,
                    bslots::OSC,
                    bcls::OSC::MODE,
                    val,
                ));
                let item = &ctx.data.sb_mux[&(tcid, TileWireCoord::new_idx(0, wires::IMUX_BUFG))];
                diff.discard_bits(&item.bits);
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslots::IO_S[1], bcls::IO::OSC_PULLUP),
                    false,
                    true,
                );
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslots::IO_E[0], bcls::IO::OSC_PULLUP),
                    false,
                    true,
                );
                diffs.push((val, diff));
            }
            ctx.insert_bel_attr_enum(tcid, bslots::OSC, bcls::OSC::MODE, xlat_enum_attr(diffs));
        }
        if tcls.bels.contains_id(bslots::MISC_SW) {
            ctx.collect_bel_attr(tcid, bslots::MISC_SW, bcls::MISC_SW::READBACK_MODE);
        }
        if tcls.bels.contains_id(bslots::MISC_NW) {
            ctx.collect_bel_attr(tcid, bslots::MISC_NW, bcls::MISC_NW::IO_INPUT_MODE);
        }
    }
}
