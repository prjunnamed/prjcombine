use prjcombine_re_collector::{xlat_bit, xlat_enum};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::tiledb::{TileBit, TileItem};
use prjcombine_xc2000::chip::ChipKind;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelKV, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    for (kind, tile, _) in &backend.egrid.db.nodes {
        if !tile.starts_with("IO") {
            continue;
        }
        if backend.egrid.node_index[kind].is_empty() {
            continue;
        }
        for bel in ["IOB0", "IOB1"] {
            let ctx = FuzzCtx::new(session, backend, tile, bel, TileBits::MainXc4000);
            fuzz_enum!(ctx, "SLEW", ["SLOW", "FAST"], [(mode "IOB")]);
            fuzz_enum!(ctx, "PULL", ["PULLDOWN", "PULLUP"], [(mode "IOB")]);
            fuzz_enum!(ctx, "ISR", ["RESET", "SET"], [(mode "IOB")]);
            fuzz_enum!(ctx, "OSR", ["RESET", "SET"], [(mode "IOB")]);
            fuzz_enum!(ctx, "IKMUX", ["IK", "IKNOT"], [(mode "IOB")]);
            fuzz_enum!(ctx, "OKMUX", ["OK", "OKNOT"], [(mode "IOB")]);
            fuzz_enum!(ctx, "OCE", ["CE"], [(mode "IOB")]);
            fuzz_enum!(ctx, "I1MUX", ["I", "IQ", "IQL"], [(mode "IOB"), (attr "ICE", ""), (attr "I2MUX", "IQ"), (attr "IKMUX", "IK")]);
            fuzz_enum!(ctx, "I2MUX", ["I", "IQ", "IQL"], [(mode "IOB"), (attr "ICE", ""), (attr "I1MUX", "IQ"), (attr "IKMUX", "IK")]);
            fuzz_enum!(ctx, "ICE", ["CE"], [(mode "IOB"), (attr "I1MUX", "IQ"), (attr "I2MUX", "IQ"), (attr "IKMUX", "IK")]);
            fuzz_enum_suffix!(ctx, "ICE", "IQL", ["CE"], [(mode "IOB"), (attr "I1MUX", "IQL"), (attr "I2MUX", "IQL")]);
            fuzz_one!(ctx, "INV.T", "1", [
                (mode "IOB"),
                (attr "OUTMUX", "O")
            ], [
                (attr_diff "TRI", "T", "TNOT")
            ]);
            if edev.chip.kind == ChipKind::Xc4000E {
                fuzz_enum!(ctx, "IMUX", ["DELAY", "I"], [(mode "IOB")]);
                for outmux in ["OQ", "O"] {
                    for omux in ["O", "ONOT"] {
                        fuzz_one!(ctx, "OUTMUX", format!("{outmux}.{omux}.O"), [
                            (mode "IOB"),
                            (attr "OKMUX", "OK"),
                            (attr "TRI", "TNOT"),
                            (bel_special BelKV::Xc4000DriveImux("O", true)),
                            (bel_special BelKV::Xc4000DriveImux("EC", false))
                        ], [
                            (attr "OUTMUX", outmux),
                            (attr "OMUX", omux)
                        ]);
                        fuzz_one!(ctx, "OUTMUX", format!("{outmux}.{omux}.CE"), [
                            (mode "IOB"),
                            (attr "OKMUX", "OK"),
                            (attr "TRI", "TNOT"),
                            (bel_special BelKV::Xc4000DriveImux("O", false)),
                            (bel_special BelKV::Xc4000DriveImux("EC", true)),
                            (pip (pin_far "EC"), (pin "O"))
                        ], [
                            (attr "OUTMUX", outmux),
                            (attr "OMUX", omux)
                        ]);
                    }
                }
            } else {
                fuzz_enum!(ctx, "DELAYMUX", ["DELAY", "I"], [(mode "IOB")]);
                fuzz_enum!(ctx, "IMUX", ["SYNC", "MEDDELAY", "DELAY", "I"], [(mode "IOB")]);
                for val in ["O", "ONOT", "CE", "CENOT", "ACTIVE", "OQ"] {
                    fuzz_one!(ctx, "OUTMUX", val, [
                        (mode "IOB"),
                        (attr "OINVMUX", ""),
                        (attr "OCEMUX", ""),
                        (attr "OKMUX", "OK")
                    ], [
                        (attr_diff "OUTMUX", "ACTIVE", val)
                    ]);
                }
                fuzz_enum!(ctx, "OINVMUX", ["O", "ONOT"], [(mode "IOB"), (attr "OUTMUX", "OQ"), (attr "OKMUX", "OK")]);
                fuzz_enum!(ctx, "OCEMUX", ["O", "CE"], [(mode "IOB"), (attr "OUTMUX", "OQ"), (attr "OKMUX", "OK")]);
            }
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                fuzz_enum!(ctx, "DRIVE", ["12", "24"], [(mode "IOB")]);
                fuzz_enum!(ctx, "TRIFFMUX", ["TRI", "TRIQ"], [(mode "IOB"), (attr "TRI", "T"), (attr "OKMUX", "OK")]);
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in edev.egrid.db.nodes.keys() {
        if !tile.starts_with("IO") {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        for bel in ["IOB0", "IOB1"] {
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
                ("IO.L", ChipKind::Xc4000E | ChipKind::SpartanXl, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 25, 8)),
                    ("READBACK_I2", TileBit::new(0, 23, 8)),
                    ("READBACK_OFF", TileBit::new(0, 22, 8)),
                ],
                ("IO.L", ChipKind::Xc4000E | ChipKind::SpartanXl, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 21, 3)),
                    ("READBACK_I2", TileBit::new(0, 22, 3)),
                    ("READBACK_OFF", TileBit::new(0, 23, 2)),
                ],
                ("IO.L", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IOB0") => {
                    [
                        ("READBACK_I1", TileBit::new(0, 26, 8)),
                        ("READBACK_I2", TileBit::new(0, 24, 8)),
                        ("READBACK_OFF", TileBit::new(0, 23, 8)),
                    ]
                }
                ("IO.L", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IOB1") => {
                    [
                        ("READBACK_I1", TileBit::new(0, 22, 3)),
                        ("READBACK_I2", TileBit::new(0, 23, 3)),
                        ("READBACK_OFF", TileBit::new(0, 24, 2)),
                    ]
                }

                ("IO.R", _, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 0, 8)),
                    ("READBACK_I2", TileBit::new(0, 2, 8)),
                    ("READBACK_OFF", TileBit::new(0, 3, 8)),
                ],
                ("IO.R", _, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 4, 3)),
                    ("READBACK_I2", TileBit::new(0, 3, 3)),
                    ("READBACK_OFF", TileBit::new(0, 2, 2)),
                ],

                ("IO.B", ChipKind::Xc4000E, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 2)),
                    ("READBACK_OFF", TileBit::new(0, 14, 2)),
                ],
                ("IO.B", ChipKind::Xc4000E, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 16, 2)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 15, 2)),
                ],
                ("IO.B", ChipKind::SpartanXl, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 2)),
                    ("READBACK_OFF", TileBit::new(0, 16, 3)),
                ],
                ("IO.B", ChipKind::SpartanXl, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 17, 2)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 16, 2)),
                ],
                ("IO.B", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IOB0") => {
                    [
                        ("READBACK_I1", TileBit::new(0, 19, 3)),
                        ("READBACK_I2", TileBit::new(0, 19, 2)),
                        ("READBACK_OFF", TileBit::new(0, 17, 3)),
                    ]
                }
                ("IO.B", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv, "IOB1") => {
                    [
                        ("READBACK_I1", TileBit::new(0, 18, 2)),
                        ("READBACK_I2", TileBit::new(0, 18, 3)),
                        ("READBACK_OFF", TileBit::new(0, 17, 2)),
                    ]
                }

                ("IO.T", ChipKind::Xc4000E, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 4)),
                    ("READBACK_OFF", TileBit::new(0, 14, 4)),
                ],
                ("IO.T", ChipKind::Xc4000E, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 16, 4)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 15, 4)),
                ],
                ("IO.T", ChipKind::SpartanXl, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 18, 3)),
                    ("READBACK_I2", TileBit::new(0, 18, 4)),
                    ("READBACK_OFF", TileBit::new(0, 16, 3)),
                ],
                ("IO.T", ChipKind::SpartanXl, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 17, 4)),
                    ("READBACK_I2", TileBit::new(0, 17, 3)),
                    ("READBACK_OFF", TileBit::new(0, 16, 4)),
                ],
                ("IO.T", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 19, 4)),
                    ("READBACK_I2", TileBit::new(0, 19, 5)),
                    ("READBACK_OFF", TileBit::new(0, 17, 4)),
                ],
                ("IO.T", ChipKind::Xc4000Ex | ChipKind::Xc4000Xla, "IOB1") => [
                    ("READBACK_I1", TileBit::new(0, 18, 5)),
                    ("READBACK_I2", TileBit::new(0, 18, 4)),
                    ("READBACK_OFF", TileBit::new(0, 17, 5)),
                ],
                ("IO.T", ChipKind::Xc4000Xv, "IOB0") => [
                    ("READBACK_I1", TileBit::new(0, 19, 5)),
                    ("READBACK_I2", TileBit::new(0, 19, 6)),
                    ("READBACK_OFF", TileBit::new(0, 17, 5)),
                ],
                ("IO.T", ChipKind::Xc4000Xv, "IOB1") => [
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
