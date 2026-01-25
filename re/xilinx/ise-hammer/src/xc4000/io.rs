use prjcombine_interconnect::{
    db::{BelInputId, BelSlotId, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{xlat_bit, xlat_enum_attr};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::{
    chip::ChipKind,
    xc4000::{bslots, enums, tslots, wires, xc4000::bcls},
};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::resolve_int_pip,
        props::{DynProp, pip::PinFar},
    },
    xc4000::specials,
};

#[derive(Clone, Debug)]
pub struct Xc4000DriveImux {
    pub slot: BelSlotId,
    pub pin: BelInputId,
    pub drive: bool,
}

impl Xc4000DriveImux {
    pub fn new(slot: BelSlotId, pin: BelInputId, drive: bool) -> Self {
        Self { slot, pin, drive }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Xc4000DriveImux {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let res_wire = backend
            .edev
            .resolve_wire(
                backend
                    .edev
                    .get_bel_input(tcrd.bel(self.slot), self.pin)
                    .wire,
            )
            .unwrap();
        fuzzer = fuzzer.fuzz(Key::WireMutex(res_wire), None, "EXCLUSIVE");
        if self.drive {
            let otcrd = res_wire.cell.tile(tslots::MAIN);
            let otile = &backend.edev[otcrd];
            let otcls = &backend.edev.db_index[otile.class];
            let wt = TileWireCoord::new_idx(0, res_wire.slot);
            let ins = &otcls.pips_bwd[&wt];
            let wf = ins.iter().find(|w| w.wire != wires::TIE_0).unwrap().tw;
            let res_wf = backend
                .edev
                .resolve_wire(backend.edev.tile_wire(otcrd, wf))
                .unwrap();
            let (tile, wt, wf) = resolve_int_pip(backend, otcrd, wt, wf).unwrap();
            fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true).fuzz(
                Key::WireMutex(res_wf),
                None,
                "EXCLUSIVE",
            );
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new_id(session, backend, tcid) else {
            continue;
        };
        for i in 0..2 {
            if !tcls.bels.contains_id(bslots::IO[i]) {
                continue;
            }
            let mut bctx = ctx.bel(bslots::IO[i]);
            let mode = "IOB";
            for (val, vname) in [
                (enums::IO_SLEW::SLOW, "SLOW"),
                (enums::IO_SLEW::FAST, "FAST"),
            ] {
                bctx.mode(mode)
                    .test_bel_attr_val(bcls::IO::SLEW, val)
                    .attr("SLEW", vname)
                    .commit();
            }
            bctx.mode(mode)
                .test_bel_attr_default(bcls::IO::PULL, enums::IO_PULL::NONE);
            bctx.mode(mode)
                .test_bel_attr_bool_rename("ISR", bcls::IO::IFF_SRVAL, "RESET", "SET");
            bctx.mode(mode)
                .test_bel_attr_bool_rename("OSR", bcls::IO::OFF_SRVAL, "RESET", "SET");
            bctx.mode(mode)
                .test_bel_input_inv_enum("IKMUX", bcls::IO::IK, "IK", "IKNOT");
            bctx.mode(mode)
                .test_bel_input_inv_enum("OKMUX", bcls::IO::OK, "OK", "OKNOT");
            bctx.mode(mode)
                .test_bel_attr_bits(bcls::IO::OFF_CE_ENABLE)
                .attr("OCE", "CE")
                .commit();
            bctx.mode(mode)
                .attr("ICE", "")
                .attr("I2MUX", "IQ")
                .attr("IKMUX", "IK")
                .test_bel_attr_rename("I1MUX", bcls::IO::MUX_I1);
            bctx.mode(mode)
                .attr("ICE", "")
                .attr("I1MUX", "IQ")
                .attr("IKMUX", "IK")
                .test_bel_attr_rename("I2MUX", bcls::IO::MUX_I2);
            bctx.mode(mode)
                .attr("I1MUX", "IQ")
                .attr("I2MUX", "IQ")
                .attr("IKMUX", "IK")
                .test_bel_attr_bits(bcls::IO::IFF_CE_ENABLE)
                .attr("ICE", "CE")
                .commit();
            bctx.mode(mode)
                .attr("I1MUX", "IQL")
                .attr("I2MUX", "IQL")
                .test_bel_special(specials::IO_ICE_IQL_CE)
                .attr("ICE", "CE")
                .commit();
            bctx.mode(mode)
                .attr("OUTMUX", "O")
                .test_bel_input_inv(bcls::IO::T, true)
                .attr_diff("TRI", "T", "TNOT")
                .commit();
            if edev.chip.kind == ChipKind::Xc4000E {
                for (val, vname) in [(enums::IO_IFF_D::I, "I"), (enums::IO_IFF_D::DELAY, "DELAY")] {
                    bctx.mode(mode)
                        .test_bel_attr_val(bcls::IO::IFF_D, val)
                        .attr("IMUX", vname)
                        .commit();
                }
                for (outmux, omux, spec_o1, spec_o2) in [
                    ("O", "O", specials::IO_OUTMUX_O_O1, specials::IO_OUTMUX_O_O2),
                    (
                        "O",
                        "ONOT",
                        specials::IO_OUTMUX_OI_O1,
                        specials::IO_OUTMUX_OI_O2,
                    ),
                    (
                        "OQ",
                        "O",
                        specials::IO_OUTMUX_OQ_O1,
                        specials::IO_OUTMUX_OQ_O2,
                    ),
                    (
                        "OQ",
                        "ONOT",
                        specials::IO_OUTMUX_OQI_O1,
                        specials::IO_OUTMUX_OQI_O2,
                    ),
                ] {
                    bctx.mode(mode)
                        .attr("OKMUX", "OK")
                        .attr("TRI", "TNOT")
                        .prop(Xc4000DriveImux::new(bslots::IO[i], bcls::IO::O2, false))
                        .prop(Xc4000DriveImux::new(bslots::IO[i], bcls::IO::O1, true))
                        .pip("O2", (PinFar, "O1"))
                        .test_bel_special(spec_o1)
                        .attr("OUTMUX", outmux)
                        .attr("OMUX", omux)
                        .commit();
                    bctx.mode(mode)
                        .attr("OKMUX", "OK")
                        .attr("TRI", "TNOT")
                        .prop(Xc4000DriveImux::new(bslots::IO[i], bcls::IO::O2, true))
                        .prop(Xc4000DriveImux::new(bslots::IO[i], bcls::IO::O1, false))
                        .test_bel_special(spec_o2)
                        .attr("OUTMUX", outmux)
                        .attr("OMUX", omux)
                        .commit();
                }
            } else {
                bctx.mode(mode)
                    .test_bel_attr_rename("DELAYMUX", bcls::IO::SYNC_D);
                bctx.mode(mode)
                    .test_bel_attr_rename("IMUX", bcls::IO::IFF_D);
                for (val, vname) in [
                    (enums::IO_MUX_O::O1, "CE"),
                    (enums::IO_MUX_O::O1_INV, "CENOT"),
                    (enums::IO_MUX_O::O2, "O"),
                    (enums::IO_MUX_O::O2_INV, "ONOT"),
                    (enums::IO_MUX_O::OQ, "OQ"),
                    (enums::IO_MUX_O::MUX, "ACTIVE"),
                ] {
                    bctx.mode(mode)
                        .attr("OINVMUX", "")
                        .attr("OCEMUX", "")
                        .attr("OKMUX", "OK")
                        .test_bel_attr_val(bcls::IO::MUX_O, val)
                        .attr_diff("OUTMUX", "ACTIVE", vname)
                        .commit();
                }
                bctx.mode(mode)
                    .attr("OUTMUX", "OQ")
                    .attr("OKMUX", "OK")
                    .test_bel_attr_bool_rename("OINVMUX", bcls::IO::OFF_D_INV, "O", "ONOT");
                for (val, vname) in [
                    (enums::IO_MUX_OFF_D::O1, "CE"),
                    (enums::IO_MUX_OFF_D::O2, "O"),
                ] {
                    bctx.mode(mode)
                        .attr("OUTMUX", "OQ")
                        .attr("OKMUX", "OK")
                        .test_bel_attr_val(bcls::IO::MUX_OFF_D, val)
                        .attr("OCEMUX", vname)
                        .commit();
                }
            }
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                for (val, vname) in [(enums::IO_DRIVE::_12, "12"), (enums::IO_DRIVE::_24, "24")] {
                    bctx.mode(mode)
                        .test_bel_attr_val(bcls::IO::DRIVE, val)
                        .attr("DRIVE", vname)
                        .commit();
                }
                for (val, vname) in [(enums::IO_MUX_T::T, "TRI"), (enums::IO_MUX_T::TQ, "TRIQ")] {
                    bctx.mode(mode)
                        .attr("TRI", "T")
                        .attr("OKMUX", "OK")
                        .test_bel_attr_val(bcls::IO::MUX_T, val)
                        .attr("TRIFFMUX", vname)
                        .commit();
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    for (tcid, tile, tcls) in &edev.db.tile_classes {
        if !ctx.has_tile_id(tcid) {
            continue;
        }
        for i in 0..2 {
            let bslot = bslots::IO[i];
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                bcls::IO::SLEW,
                &[enums::IO_SLEW::FAST, enums::IO_SLEW::SLOW],
            );
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IO::PULL, enums::IO_PULL::NONE);
            ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::IO::IFF_SRVAL);
            ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::IO::OFF_SRVAL);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IO::IK);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IO::OK);
            ctx.collect_bel_input_inv(tcid, bslot, bcls::IO::T);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::IFF_CE_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::OFF_CE_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::MUX_I1);
            ctx.collect_bel_attr(tcid, bslot, bcls::IO::MUX_I2);

            if edev.chip.kind == ChipKind::Xc4000E {
                ctx.collect_bel_attr_subset(
                    tcid,
                    bslot,
                    bcls::IO::IFF_D,
                    &[enums::IO_IFF_D::I, enums::IO_IFF_D::DELAY],
                );
                let item = xlat_bit(ctx.get_diff_bel_special(tcid, bslot, specials::IO_ICE_IQL_CE));
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IO::IFF_CE_ENABLE, item);

                let diff_oq = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_OQ_O1);
                assert_eq!(
                    diff_oq,
                    ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_OQ_O2)
                );
                let diff_oq_not = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_OQI_O1);
                assert_eq!(
                    diff_oq_not,
                    ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_OQI_O2)
                );
                let diff_inv_off_d = diff_oq_not.combine(&!&diff_oq);
                let diff_o2 = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_O_O2);
                let diff_o2i = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_OI_O2);
                let diff_o1 = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_O_O1);
                let diff_o1i = ctx.get_diff_bel_special(tcid, bslot, specials::IO_OUTMUX_OI_O1);
                let diff_o2not = diff_o2i.combine(&!&diff_inv_off_d);
                let diff_o1not = diff_o1i.combine(&!&diff_inv_off_d);
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslot,
                    bcls::IO::OFF_D_INV,
                    xlat_bit(diff_inv_off_d),
                );
                let mut diff_off_used = diff_oq.clone();
                diff_off_used
                    .bits
                    .retain(|bit, _| !diff_o1.bits.contains_key(bit));
                diff_off_used
                    .bits
                    .retain(|bit, _| !diff_o1not.bits.contains_key(bit));
                let diff_oq = diff_oq.combine(&!&diff_off_used);
                ctx.insert_bel_attr_raw(
                    tcid,
                    bslot,
                    bcls::IO::MUX_O,
                    xlat_enum_attr(vec![
                        (enums::IO_MUX_O::O1, diff_o1),
                        (enums::IO_MUX_O::O1_INV, diff_o1not),
                        (enums::IO_MUX_O::O2, diff_o2),
                        (enums::IO_MUX_O::O2_INV, diff_o2not),
                        (enums::IO_MUX_O::OQ, diff_oq),
                    ]),
                );
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IO::OFF_USED, xlat_bit(diff_off_used));
            } else {
                ctx.collect_bel_attr(tcid, bslot, bcls::IO::IFF_D);
                ctx.collect_bel_attr(tcid, bslot, bcls::IO::SYNC_D);

                // ?!?
                let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IO_ICE_IQL_CE);
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, bcls::IO::IFF_CE_ENABLE),
                    true,
                    false,
                );
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslot,
                    bcls::IO::IFF_CE_ENABLE_NO_IQ,
                    xlat_bit(diff),
                );

                ctx.collect_bel_attr(tcid, bslot, bcls::IO::MUX_OFF_D);
                ctx.collect_bel_attr_bool_bi(tcid, bslot, bcls::IO::OFF_D_INV);

                let mut diff_oq =
                    ctx.get_diff_attr_val(tcid, bslot, bcls::IO::MUX_O, enums::IO_MUX_O::OQ);
                let diff_o1 =
                    ctx.get_diff_attr_val(tcid, bslot, bcls::IO::MUX_O, enums::IO_MUX_O::O1);
                let mut diff_o1not =
                    ctx.get_diff_attr_val(tcid, bslot, bcls::IO::MUX_O, enums::IO_MUX_O::O1_INV);
                let mut diff_o2 =
                    ctx.get_diff_attr_val(tcid, bslot, bcls::IO::MUX_O, enums::IO_MUX_O::O2);
                let mut diff_o2not =
                    ctx.get_diff_attr_val(tcid, bslot, bcls::IO::MUX_O, enums::IO_MUX_O::O2_INV);
                let diff_mux =
                    ctx.get_diff_attr_val(tcid, bslot, bcls::IO::MUX_O, enums::IO_MUX_O::MUX);
                diff_o1not.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, bcls::IO::OFF_D_INV),
                    true,
                    false,
                );
                diff_o2not.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, bcls::IO::OFF_D_INV),
                    true,
                    false,
                );
                diff_o2.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, bcls::IO::MUX_OFF_D),
                    enums::IO_MUX_OFF_D::O2,
                    enums::IO_MUX_OFF_D::O1,
                );
                diff_o2not.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, bcls::IO::MUX_OFF_D),
                    enums::IO_MUX_OFF_D::O2,
                    enums::IO_MUX_OFF_D::O1,
                );
                let mut diff_off_used = diff_oq.clone();
                diff_off_used
                    .bits
                    .retain(|bit, _| !diff_o1.bits.contains_key(bit));
                diff_oq = diff_oq.combine(&!&diff_off_used);
                ctx.insert_bel_attr_raw(
                    tcid,
                    bslot,
                    bcls::IO::MUX_O,
                    xlat_enum_attr(vec![
                        (enums::IO_MUX_O::O1, diff_o1),
                        (enums::IO_MUX_O::O1_INV, diff_o1not),
                        (enums::IO_MUX_O::O2, diff_o2),
                        (enums::IO_MUX_O::O2_INV, diff_o2not),
                        (enums::IO_MUX_O::OQ, diff_oq),
                        (enums::IO_MUX_O::MUX, diff_mux),
                    ]),
                );
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IO::OFF_USED, xlat_bit(diff_off_used));
            }
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                ctx.collect_bel_attr(tcid, bslot, bcls::IO::DRIVE);
                ctx.collect_bel_attr(tcid, bslot, bcls::IO::MUX_T);
            }
            let rb_bits = match (&tile[..4], edev.chip.kind, i) {
                ("IO_W", ChipKind::Xc4000E | ChipKind::SpartanXl, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 25, 8)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 23, 8)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 22, 8)),
                ],
                ("IO_W", ChipKind::Xc4000E | ChipKind::SpartanXl, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 21, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 22, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 23, 2)),
                ],
                ("IO_W", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 26, 8)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 24, 8)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 23, 8)),
                ],
                ("IO_W", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 22, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 23, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 24, 2)),
                ],

                ("IO_E", _, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 0, 8)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 2, 8)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 3, 8)),
                ],
                ("IO_E", _, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 4, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 3, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 2, 2)),
                ],

                ("IO_S", ChipKind::Xc4000E, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 2)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 14, 2)),
                ],
                ("IO_S", ChipKind::Xc4000E, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 16, 2)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 17, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 15, 2)),
                ],
                ("IO_S", ChipKind::SpartanXl, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 2)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 16, 3)),
                ],
                ("IO_S", ChipKind::SpartanXl, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 17, 2)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 17, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 16, 2)),
                ],
                ("IO_S", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 19, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 19, 2)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 17, 3)),
                ],
                ("IO_S", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 2)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 17, 2)),
                ],

                ("IO_N", ChipKind::Xc4000E, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 4)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 14, 4)),
                ],
                ("IO_N", ChipKind::Xc4000E, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 16, 4)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 17, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 15, 4)),
                ],
                ("IO_N", ChipKind::SpartanXl, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 3)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 4)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 16, 3)),
                ],
                ("IO_N", ChipKind::SpartanXl, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 17, 4)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 17, 3)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 16, 4)),
                ],
                ("IO_N", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 19, 4)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 19, 5)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 17, 4)),
                ],
                ("IO_N", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 5)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 4)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 17, 5)),
                ],
                ("IO_N", ChipKind::Xc4000Xv, 0) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 19, 5)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 19, 6)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 17, 5)),
                ],
                ("IO_N", ChipKind::Xc4000Xv, 1) => [
                    (bcls::IO::READBACK_I1, TileBit::new(0, 18, 6)),
                    (bcls::IO::READBACK_I2, TileBit::new(0, 18, 5)),
                    (bcls::IO::READBACK_OQ, TileBit::new(0, 17, 6)),
                ],

                _ => unreachable!(),
            };
            for (attr, bit) in rb_bits {
                ctx.insert_bel_attr_bool(tcid, bslot, attr, bit.neg());
            }
        }
    }
}
