use prjcombine_interconnect::{
    db::{BelInfo, BelSlotId, CellSlotId, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{FuzzerProp, xlat_bit, xlat_enum};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind, tslots};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::resolve_int_pip,
        props::{DynProp, pip::PinFar},
    },
};

#[derive(Clone, Debug)]
pub struct Xc4000DriveImux {
    pub slot: BelSlotId,
    pub pin: &'static str,
    pub drive: bool,
}

impl Xc4000DriveImux {
    pub fn new(slot: BelSlotId, pin: &'static str, drive: bool) -> Self {
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
        let tile = backend.egrid.tile(tcrd);
        let tcls = &backend.egrid.db.tile_classes[tile.class];
        let bel_data = &tcls.bels[self.slot];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let wire = *bel_data.pins[self.pin].wires.iter().next().unwrap();
        let res_wire = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, wire))
            .unwrap();
        fuzzer = fuzzer.fuzz(Key::NodeMutex(res_wire), None, "EXCLUSIVE");
        if self.drive {
            let otcrd = res_wire.cell.tile(tslots::MAIN);
            let otile = backend.egrid.tile(otcrd);
            let otcls = &backend.egrid.db_index.tile_classes[otile.class];
            let wt = TileWireCoord {
                cell: CellSlotId::from_idx(0),
                wire: res_wire.slot,
            };
            let ins = &otcls.pips_bwd[&wt];
            let wf = ins.iter().next().unwrap().tw;
            let res_wf = backend
                .egrid
                .resolve_wire(backend.egrid.tile_wire(otcrd, wf))
                .unwrap();
            let (tile, wt, wf) = resolve_int_pip(backend, otcrd, wt, wf).unwrap();
            fuzzer = fuzzer.base(Key::Pip(tile, wf, wt), true).fuzz(
                Key::NodeMutex(res_wf),
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
    for tile in backend.egrid.db.tile_classes.keys() {
        if !tile.starts_with("IO") {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::IO[i]);
            let mode = "IOB";
            bctx.mode(mode).test_enum("SLEW", &["SLOW", "FAST"]);
            bctx.mode(mode).test_enum("PULL", &["PULLDOWN", "PULLUP"]);
            bctx.mode(mode).test_enum("ISR", &["RESET", "SET"]);
            bctx.mode(mode).test_enum("OSR", &["RESET", "SET"]);
            bctx.mode(mode).test_enum("IKMUX", &["IK", "IKNOT"]);
            bctx.mode(mode).test_enum("OKMUX", &["OK", "OKNOT"]);
            bctx.mode(mode).test_enum("OCE", &["CE"]);
            bctx.mode(mode)
                .attr("ICE", "")
                .attr("I2MUX", "IQ")
                .attr("IKMUX", "IK")
                .test_enum("I1MUX", &["I", "IQ", "IQL"]);
            bctx.mode(mode)
                .attr("ICE", "")
                .attr("I1MUX", "IQ")
                .attr("IKMUX", "IK")
                .test_enum("I2MUX", &["I", "IQ", "IQL"]);
            bctx.mode(mode)
                .attr("I1MUX", "IQ")
                .attr("I2MUX", "IQ")
                .attr("IKMUX", "IK")
                .test_enum("ICE", &["CE"]);
            bctx.mode(mode)
                .attr("I1MUX", "IQL")
                .attr("I2MUX", "IQL")
                .test_enum_suffix("ICE", "IQL", &["CE"]);
            bctx.mode(mode)
                .attr("OUTMUX", "O")
                .test_manual("INV.T", "1")
                .attr_diff("TRI", "T", "TNOT")
                .commit();
            if edev.chip.kind == ChipKind::Xc4000E {
                bctx.mode(mode).test_enum("IMUX", &["DELAY", "I"]);
                for outmux in ["OQ", "O"] {
                    for omux in ["O", "ONOT"] {
                        bctx.mode(mode)
                            .attr("OKMUX", "OK")
                            .attr("TRI", "TNOT")
                            .prop(Xc4000DriveImux::new(bels::IO[i], "O", true))
                            .prop(Xc4000DriveImux::new(bels::IO[i], "EC", false))
                            .test_manual("OUTMUX", format!("{outmux}.{omux}.O"))
                            .attr("OUTMUX", outmux)
                            .attr("OMUX", omux)
                            .commit();
                        bctx.mode(mode)
                            .attr("OKMUX", "OK")
                            .attr("TRI", "TNOT")
                            .prop(Xc4000DriveImux::new(bels::IO[i], "O", false))
                            .prop(Xc4000DriveImux::new(bels::IO[i], "EC", true))
                            .pip("O", (PinFar, "EC"))
                            .test_manual("OUTMUX", format!("{outmux}.{omux}.CE"))
                            .attr("OUTMUX", outmux)
                            .attr("OMUX", omux)
                            .commit();
                    }
                }
            } else {
                bctx.mode(mode).test_enum("DELAYMUX", &["DELAY", "I"]);
                bctx.mode(mode)
                    .test_enum("IMUX", &["SYNC", "MEDDELAY", "DELAY", "I"]);
                for val in ["O", "ONOT", "CE", "CENOT", "ACTIVE", "OQ"] {
                    bctx.mode(mode)
                        .attr("OINVMUX", "")
                        .attr("OCEMUX", "")
                        .attr("OKMUX", "OK")
                        .test_manual("OUTMUX", val)
                        .attr_diff("OUTMUX", "ACTIVE", val)
                        .commit();
                }
                bctx.mode(mode)
                    .attr("OUTMUX", "OQ")
                    .attr("OKMUX", "OK")
                    .test_enum("OINVMUX", &["O", "ONOT"]);
                bctx.mode(mode)
                    .attr("OUTMUX", "OQ")
                    .attr("OKMUX", "OK")
                    .test_enum("OCEMUX", &["O", "CE"]);
            }
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                bctx.mode(mode).test_enum("DRIVE", &["12", "24"]);
                bctx.mode(mode)
                    .attr("TRI", "T")
                    .attr("OKMUX", "OK")
                    .test_enum("TRIFFMUX", &["TRI", "TRIQ"]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in edev.egrid.db.tile_classes.keys() {
        if !tile.starts_with("IO") {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        for bel in ["IO0", "IO1"] {
            ctx.collect_enum(tile, bel, "SLEW", &["SLOW", "FAST"]);
            ctx.collect_enum_default(tile, bel, "PULL", &["PULLUP", "PULLDOWN"], "NONE");
            let item = ctx.extract_enum_bool(tile, bel, "ISR", "RESET", "SET");
            ctx.tiledb.insert(tile, bel, "IFF_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OSR", "RESET", "SET");
            ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IKMUX", "IK", "IKNOT");
            ctx.tiledb.insert(tile, bel, "INV.IFF_CLK", item);
            let item = ctx.extract_enum_bool(tile, bel, "OKMUX", "OK", "OKNOT");
            ctx.tiledb.insert(tile, bel, "INV.OFF_CLK", item);
            let item = ctx.extract_bit(tile, bel, "OCE", "CE");
            ctx.tiledb.insert(tile, bel, "OFF_CE_ENABLE", item);
            ctx.collect_enum(tile, bel, "I1MUX", &["I", "IQ", "IQL"]);
            ctx.collect_enum(tile, bel, "I2MUX", &["I", "IQ", "IQL"]);
            let item = ctx.extract_bit(tile, bel, "ICE", "CE");
            ctx.tiledb.insert(tile, bel, "IFF_CE_ENABLE", item);
            ctx.collect_bit(tile, bel, "INV.T", "1");
            if edev.chip.kind == ChipKind::Xc4000E {
                let item = ctx.extract_enum(tile, bel, "IMUX", &["I", "DELAY"]);
                ctx.tiledb.insert(tile, bel, "IFF_D", item);
                let item = ctx.extract_bit(tile, bel, "ICE.IQL", "CE");
                ctx.tiledb.insert(tile, bel, "IFF_CE_ENABLE", item);

                let diff_oq = ctx.state.get_diff(tile, bel, "OUTMUX", "OQ.O.CE");
                assert_eq!(diff_oq, ctx.state.get_diff(tile, bel, "OUTMUX", "OQ.O.O"));
                let diff_oq_not = ctx.state.get_diff(tile, bel, "OUTMUX", "OQ.ONOT.CE");
                assert_eq!(
                    diff_oq_not,
                    ctx.state.get_diff(tile, bel, "OUTMUX", "OQ.ONOT.O")
                );
                let diff_inv_off_d = diff_oq_not.combine(&!&diff_oq);
                let diff_o = ctx.state.get_diff(tile, bel, "OUTMUX", "O.O.O");
                let diff_onot = ctx.state.get_diff(tile, bel, "OUTMUX", "O.ONOT.O");
                let diff_ce = ctx.state.get_diff(tile, bel, "OUTMUX", "O.O.CE");
                let diff_cenot = ctx.state.get_diff(tile, bel, "OUTMUX", "O.ONOT.CE");
                let diff_onot = diff_onot.combine(&!&diff_inv_off_d);
                let diff_cenot = diff_cenot.combine(&!&diff_inv_off_d);
                ctx.tiledb
                    .insert(tile, bel, "INV.OFF_D", xlat_bit(diff_inv_off_d));
                let mut diff_off_used = diff_oq.clone();
                diff_off_used
                    .bits
                    .retain(|bit, _| !diff_ce.bits.contains_key(bit));
                diff_off_used
                    .bits
                    .retain(|bit, _| !diff_cenot.bits.contains_key(bit));
                let diff_oq = diff_oq.combine(&!&diff_off_used);
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "OMUX",
                    xlat_enum(vec![
                        ("CE", diff_ce),
                        ("CE.INV", diff_cenot),
                        ("O", diff_o),
                        ("O.INV", diff_onot),
                        ("OFF", diff_oq),
                    ]),
                );
                ctx.tiledb
                    .insert(tile, bel, "OFF_USED", xlat_bit(diff_off_used));
            } else {
                let item = ctx.extract_enum(tile, bel, "IMUX", &["I", "DELAY", "MEDDELAY", "SYNC"]);
                ctx.tiledb.insert(tile, bel, "IFF_D", item);
                let item = ctx.extract_enum(tile, bel, "DELAYMUX", &["I", "DELAY"]);
                ctx.tiledb.insert(tile, bel, "SYNC_D", item);
                // ?!?
                let mut diff = ctx.state.get_diff(tile, bel, "ICE.IQL", "CE");
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_CE_ENABLE"), true, false);
                ctx.tiledb
                    .insert(tile, bel, "IFF_CE_ENABLE_NO_IQ", xlat_bit(diff));

                let item = ctx.extract_enum(tile, bel, "OCEMUX", &["O", "CE"]);
                ctx.tiledb.insert(tile, bel, "MUX.OFF_D", item);
                let item = ctx.extract_enum_bool(tile, bel, "OINVMUX", "O", "ONOT");
                ctx.tiledb.insert(tile, bel, "INV.OFF_D", item);

                let mut diff_oq = ctx.state.get_diff(tile, bel, "OUTMUX", "OQ");
                let diff_ce = ctx.state.get_diff(tile, bel, "OUTMUX", "CE");
                let mut diff_cenot = ctx.state.get_diff(tile, bel, "OUTMUX", "CENOT");
                let mut diff_o = ctx.state.get_diff(tile, bel, "OUTMUX", "O");
                let mut diff_onot = ctx.state.get_diff(tile, bel, "OUTMUX", "ONOT");
                let diff_mux = ctx.state.get_diff(tile, bel, "OUTMUX", "ACTIVE");
                diff_cenot.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.OFF_D"), true, false);
                diff_onot.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.OFF_D"), true, false);
                diff_o.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.OFF_D"), "O", "CE");
                diff_onot.apply_enum_diff(ctx.tiledb.item(tile, bel, "MUX.OFF_D"), "O", "CE");
                let mut diff_off_used = diff_oq.clone();
                diff_off_used
                    .bits
                    .retain(|bit, _| !diff_ce.bits.contains_key(bit));
                diff_oq = diff_oq.combine(&!&diff_off_used);
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "OMUX",
                    xlat_enum(vec![
                        ("CE", diff_ce),
                        ("CE.INV", diff_cenot),
                        ("O", diff_o),
                        ("O.INV", diff_onot),
                        ("OFF", diff_oq),
                        ("MUX", diff_mux),
                    ]),
                );
                ctx.tiledb
                    .insert(tile, bel, "OFF_USED", xlat_bit(diff_off_used));
            }
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                ctx.collect_enum(tile, bel, "DRIVE", &["12", "24"]);
                let item = xlat_enum(vec![
                    ("T", ctx.state.get_diff(tile, bel, "TRIFFMUX", "TRI")),
                    ("TFF", ctx.state.get_diff(tile, bel, "TRIFFMUX", "TRIQ")),
                ]);
                ctx.tiledb.insert(tile, bel, "TMUX", item);
            }
            let rb_bits = match (&tile[..4], edev.chip.kind, bel) {
                ("IO.L", ChipKind::Xc4000E | ChipKind::SpartanXl, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 25, 8)),
                    ("READBACK_I2", TileBit::new(0, 23, 8)),
                    ("READBACK_OFF", TileBit::new(0, 22, 8)),
                ],
                ("IO.L", ChipKind::Xc4000E | ChipKind::SpartanXl, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 21, 3)),
                    ("READBACK_I2", TileBit::new(0, 22, 3)),
                    ("READBACK_OFF", TileBit::new(0, 23, 2)),
                ],
                ("IO.L", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 26, 8)),
                    ("READBACK_I2", TileBit::new(0, 24, 8)),
                    ("READBACK_OFF", TileBit::new(0, 23, 8)),
                ],
                ("IO.L", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 22, 3)),
                    ("READBACK_I2", TileBit::new(0, 23, 3)),
                    ("READBACK_OFF", TileBit::new(0, 24, 2)),
                ],

                ("IO.R", _, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 0, 8)),
                    ("READBACK_I2", TileBit::new(0, 2, 8)),
                    ("READBACK_OFF", TileBit::new(0, 3, 8)),
                ],
                ("IO.R", _, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 4, 3)),
                    ("READBACK_I2", TileBit::new(0, 3, 3)),
                    ("READBACK_OFF", TileBit::new(0, 2, 2)),
                ],

                ("IO.B", ChipKind::Xc4000E, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 2)),
                    ("READBACK_OFF", TileBit::new(0, 14, 2)),
                ],
                ("IO.B", ChipKind::Xc4000E, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 16, 2)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 15, 2)),
                ],
                ("IO.B", ChipKind::SpartanXl, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 2)),
                    ("READBACK_OFF", TileBit::new(0, 16, 3)),
                ],
                ("IO.B", ChipKind::SpartanXl, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 17, 2)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 16, 2)),
                ],
                ("IO.B", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 19, 3)),
                    ("READBACK_I2", TileBit::new(0, 19, 2)),
                    ("READBACK_OFF", TileBit::new(0, 17, 3)),
                ],
                ("IO.B", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 18, 2)),
                    ("READBACK_I2", TileBit::new(0, 18, 3)),
                    ("READBACK_OFF", TileBit::new(0, 17, 2)),
                ],

                ("IO.T", ChipKind::Xc4000E, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 4)),
                    ("READBACK_OFF", TileBit::new(0, 14, 4)),
                ],
                ("IO.T", ChipKind::Xc4000E, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 16, 4)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 15, 4)),
                ],
                ("IO.T", ChipKind::SpartanXl, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 4)),
                    ("READBACK_OFF", TileBit::new(0, 16, 3)),
                ],
                ("IO.T", ChipKind::SpartanXl, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 17, 4)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 16, 4)),
                ],
                ("IO.T", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 19, 4)),
                    ("READBACK_I2", TileBit::new(0, 19, 5)),
                    ("READBACK_OFF", TileBit::new(0, 17, 4)),
                ],
                ("IO.T", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 18, 5)),
                    ("READBACK_I2", TileBit::new(0, 18, 4)),
                    ("READBACK_OFF", TileBit::new(0, 17, 5)),
                ],
                ("IO.T", ChipKind::Xc4000Xv, "IO0") => [
                    ("READBACK_I1", TileBit::new(0, 19, 5)),
                    ("READBACK_I2", TileBit::new(0, 19, 6)),
                    ("READBACK_OFF", TileBit::new(0, 17, 5)),
                ],
                ("IO.T", ChipKind::Xc4000Xv, "IO1") => [
                    ("READBACK_I1", TileBit::new(0, 18, 6)),
                    ("READBACK_I2", TileBit::new(0, 18, 5)),
                    ("READBACK_OFF", TileBit::new(0, 17, 6)),
                ],

                _ => unreachable!(),
            };
            for (attr, bit) in rb_bits {
                ctx.tiledb
                    .insert(tile, bel, attr, TileItem::from_bit(bit, true));
            }
        }
    }
}
