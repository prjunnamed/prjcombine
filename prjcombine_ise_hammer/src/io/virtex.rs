use std::collections::{HashMap, HashSet};

use bitvec::vec::BitVec;
use prjcombine_collector::{xlat_bit, xlat_bitvec, xlat_bool, xlat_enum, Diff};
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_virtex::grid::{GridKind, IoCoord, TileIobId};
use prjcombine_xilinx_geom::{Bond, Device, ExpandedDevice, GeomDb};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelKV, ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_one, fuzz_one_extras,
};

fn has_any_vref<'a>(
    edev: &prjcombine_virtex::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tile: &str,
    bel: BelId,
) -> Option<&'a str> {
    let node_kind = edev.egrid.db.get_node(tile);
    let mut bonded_ios = HashMap::new();
    for devbond in device.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        let Bond::Virtex(bond) = bond else {
            unreachable!()
        };
        for &io in &bond.vref {
            bonded_ios.insert(io, &devbond.name[..]);
        }
    }
    for &(_, col, row, _) in &edev.egrid.node_index[node_kind] {
        let crd = IoCoord {
            col,
            row,
            iob: TileIobId::from_idx(bel.to_idx()),
        };
        if let Some(&pkg) = bonded_ios.get(&crd) {
            return Some(pkg);
        }
    }
    None
}

const IOSTDS_CMOS_V: &[&str] = &["LVTTL", "LVCMOS2", "PCI33_3", "PCI33_5", "PCI66_3"];
const IOSTDS_CMOS_VE: &[&str] = &[
    "LVTTL", "LVCMOS2", "LVCMOS18", "PCI33_3", "PCI66_3", "PCIX66_3",
];
const IOSTDS_VREF_LV: &[&str] = &["GTL", "HSTL_I", "HSTL_III", "HSTL_IV"];
const IOSTDS_VREF_HV: &[&str] = &[
    "GTLP", "SSTL3_I", "SSTL3_II", "SSTL2_I", "SSTL2_II", "AGP", "CTT",
];
const IOSTDS_DIFF: &[&str] = &["LVDS", "LVPECL"];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let package = backend.ebonds.keys().next().unwrap();
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for side in ['L', 'R', 'B', 'T'] {
        let tile = format!("IO.{side}");
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'B' | 'T')) {
                continue;
            }
            let ctx = FuzzCtx::new(
                session,
                backend,
                &tile,
                format!("IOB{i}"),
                TileBits::MainAuto,
            );
            fuzz_one!(ctx, "PRESENT", "1", [
                (global_mutex "VREF", "NO"),
                (global_opt "SHORTENJTAGCHAIN", "NO"),
                (global_opt "UNUSEDPIN", "PULLNONE"),
                (bel_special BelKV::VirtexIsDllIob(false))
            ], [
                (mode "IOB"),
                (attr "TFFATTRBOX", "HIGH"),
                (attr "OFFATTRBOX", "HIGH")
            ]);
            if let Some(pkg) = has_any_vref(edev, backend.device, backend.db, &tile, ctx.bel) {
                fuzz_one!(ctx, "PRESENT", "NOT_VREF", [
                    (package pkg),
                    (global_mutex "VREF", "YES"),
                    (bel_special BelKV::OtherIobInput("GTL".to_string())),
                    (global_opt "SHORTENJTAGCHAIN", "NO"),
                    (global_opt "UNUSEDPIN", "PULLNONE"),
                    (bel_special BelKV::VirtexIsDllIob(false)),
                    (bel_special BelKV::IsVref)
                ], [
                    (mode "IOB"),
                    (attr "TFFATTRBOX", "HIGH"),
                    (attr "OFFATTRBOX", "HIGH")
                ]);
            }
            fuzz_one!(ctx, "SHORTEN_JTAG_CHAIN", "0", [
                (global_mutex "VREF", "NO"),
                (global_opt "SHORTENJTAGCHAIN", "YES"),
                (global_opt "UNUSEDPIN", "PULLNONE"),
                (bel_special BelKV::VirtexIsDllIob(false))
            ], [
                (mode "IOB"),
                (attr "TFFATTRBOX", "HIGH"),
                (attr "OFFATTRBOX", "HIGH")
            ]);
            fuzz_enum!(ctx, "SRMUX", ["0", "1", "SR", "SR_B"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "IINITMUX", "0"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "ICEMUX", ["0", "1", "ICE", "ICE_B"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "OCEMUX", ["0", "1", "OCE", "OCE_B"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (pin "OCE")
            ]);
            fuzz_enum!(ctx, "TCEMUX", ["0", "1", "TCE", "TCE_B"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (pin "TCE")
            ]);
            fuzz_enum!(ctx, "TRIMUX", ["0", "1", "T", "T_TB"], [
                (global_mutex "DRIVE", "IOB"),
                (mode "IOB"),
                (attr "TSEL", "1"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "OMUX", ["0", "1", "O", "O_B"], [
                (global_mutex "DRIVE", "IOB"),
                (mode "IOB"),
                (attr "OUTMUX", "1"),
                (pin "O")
            ]);
            fuzz_enum!(ctx, "ICKINV", ["0", "1"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OCKINV", ["0", "1"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TCKINV", ["0", "1"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "IFF", ["#FF", "#LATCH"], [
                (mode "IOB"),
                (attr "ICEMUX", "0"),
                (attr "ICKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OFF", ["#FF", "#LATCH"], [
                (mode "IOB"),
                (attr "OCEMUX", "0"),
                (attr "OCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TFF", ["#FF", "#LATCH"], [
                (mode "IOB"),
                (attr "TCEMUX", "0"),
                (attr "TCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "IINITMUX", ["0"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "ICKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OINITMUX", ["0"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (attr "OCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TINITMUX", ["0"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (attr "TCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "IFFINITATTR", ["LOW", "HIGH"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "ICKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "OFFATTRBOX", ["LOW", "HIGH"], [
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (attr "OCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "TFFATTRBOX", ["LOW", "HIGH"], [
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (attr "TCKINV", "1"),
                (pin "CLK")
            ]);
            fuzz_enum!(ctx, "FFATTRBOX", ["SYNC", "ASYNC"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (pin "IQ")
            ]);
            fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "IFFMUX", "1"),
                (pin "IQ"),
                (pin "I")
            ]);
            fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
                (mode "IOB"),
                (attr "IFF", "#FF"),
                (attr "IMUX", "1"),
                (pin "IQ"),
                (pin "I")
            ]);
            fuzz_enum!(ctx, "TSEL", ["0", "1"], [
                (global_mutex "DRIVE", "IOB"),
                (mode "IOB"),
                (attr "TFF", "#FF"),
                (attr "TRIMUX", "T"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "OUTMUX", ["0", "1"], [
                (global_mutex "DRIVE", "IOB"),
                (mode "IOB"),
                (attr "OFF", "#FF"),
                (attr "OMUX", "O"),
                (attr "TRIMUX", "T"),
                (attr "TSEL", "1"),
                (pin "O"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "PULL", ["PULLDOWN", "PULLUP", "KEEPER"], [
                (mode "IOB"),
                (attr "IMUX", "0"),
                (pin "I")
            ]);
            let iostds_cmos = if edev.grid.kind == GridKind::Virtex {
                IOSTDS_CMOS_V
            } else {
                IOSTDS_CMOS_VE
            };
            for &iostd in iostds_cmos {
                fuzz_one!(ctx, "ISTD", iostd, [
                    (mode "IOB"),
                    (attr "OUTMUX", ""),
                    (pin "I"),
                    (bel_special BelKV::VirtexIsDllIob(false))
                ], [
                    (attr "IOATTRBOX", iostd),
                    (attr "IMUX", "1")
                ]);
                for slew in ["FAST", "SLOW"] {
                    if iostd == "LVTTL" {
                        for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                            fuzz_one!(ctx, "OSTD", format!("{iostd}.{drive}.{slew}"), [
                                (global_mutex "DRIVE", "IOB"),
                                (mode "IOB"),
                                (attr "IMUX", ""),
                                (attr "IFFMUX", ""),
                                (pin "O"),
                                (pin "T"),
                                (bel_special BelKV::VirtexIsDllIob(false))
                            ], [
                                (attr "IOATTRBOX", iostd),
                                (attr "DRIVEATTRBOX", drive),
                                (attr "SLEW", slew),
                                (attr "OMUX", "O_B"),
                                (attr "OUTMUX", "1"),
                                (attr "TRIMUX", "T"),
                                (attr "TSEL", "1")
                            ]);
                        }
                    } else {
                        fuzz_one!(ctx, "OSTD", format!("{iostd}.{slew}"), [
                            (global_mutex "DRIVE", "IOB"),
                            (mode "IOB"),
                            (attr "IMUX", ""),
                            (attr "IFFMUX", ""),
                            (pin "O"),
                            (pin "T"),
                            (bel_special BelKV::VirtexIsDllIob(false))
                        ], [
                            (attr "IOATTRBOX", iostd),
                            (attr "SLEW", slew),
                            (attr "OMUX", "O_B"),
                            (attr "OUTMUX", "1"),
                            (attr "TRIMUX", "T"),
                            (attr "TSEL", "1")
                        ]);
                    }
                }
            }
            for &iostd in IOSTDS_VREF_LV.iter().chain(IOSTDS_VREF_HV) {
                fuzz_one!(ctx, "ISTD", iostd, [
                    (global_mutex "VREF", "YES"),
                    (package package),
                    (mode "IOB"),
                    (bel_special BelKV::OtherIobInput(iostd.to_string())),
                    (attr "OUTMUX", ""),
                    (pin "I"),
                    (bel_special BelKV::VirtexIsDllIob(false))
                ], [
                    (attr "IOATTRBOX", iostd),
                    (attr "IMUX", "1")
                ]);
                for slew in ["FAST", "SLOW"] {
                    fuzz_one!(ctx, "OSTD", format!("{iostd}.{slew}"), [
                        (global_mutex "DRIVE", "IOB"),
                        (mode "IOB"),
                        (attr "IMUX", ""),
                        (attr "IFFMUX", ""),
                        (pin "O"),
                        (pin "T"),
                        (bel_special BelKV::VirtexIsDllIob(false))
                    ], [
                        (attr "IOATTRBOX", iostd),
                        (attr "SLEW", slew),
                        (attr "OMUX", "O_B"),
                        (attr "OUTMUX", "1"),
                        (attr "TRIMUX", "T"),
                        (attr "TSEL", "1")
                    ]);
                }
            }
            if edev.grid.kind != GridKind::Virtex {
                for &iostd in IOSTDS_DIFF {
                    fuzz_one!(ctx, "ISTD", iostd, [
                        (package package),
                        (global_opt "UNUSEDPIN", "PULLNONE"),
                        (mode "IOB"),
                        (attr "OUTMUX", ""),
                        (pin "I"),
                        (bel_special BelKV::VirtexIsDllIob(false)),
                        (bel_special BelKV::IsDiff)
                    ], [
                        (attr "IOATTRBOX", iostd),
                        (attr "IMUX", "1")
                    ]);
                    for slew in ["FAST", "SLOW"] {
                        fuzz_one!(ctx, "OSTD", format!("{iostd}.{slew}"), [
                            (global_mutex "DRIVE", "IOB"),
                            (package package),
                            (global_opt "UNUSEDPIN", "PULLNONE"),
                            (mode "IOB"),
                            (attr "IMUX", ""),
                            (attr "IFFMUX", ""),
                            (pin "O"),
                            (pin "T"),
                            (bel_special BelKV::VirtexIsDllIob(false)),
                            (bel_special BelKV::IsDiff)
                        ], [
                            (attr "IOATTRBOX", iostd),
                            (attr "SLEW", slew),
                            (attr "OMUX", "O_B"),
                            (attr "OUTMUX", "1"),
                            (attr "TRIMUX", "T"),
                            (attr "TSEL", "1")
                        ]);
                    }
                }
                if tile == "IO.B" || tile == "IO.T" {
                    let tile_clk = if backend.device.name.contains("2s") {
                        if tile == "IO.B" {
                            "CLKB_2DLL"
                        } else {
                            "CLKT_2DLL"
                        }
                    } else {
                        if tile == "IO.B" {
                            "CLKB_4DLL"
                        } else {
                            "CLKT_4DLL"
                        }
                    };
                    let bel_clk = if i == 1 { "IOFB1" } else { "IOFB0" };
                    for &iostd in IOSTDS_CMOS_VE {
                        let extras = vec![ExtraFeature::new(
                            ExtraFeatureKind::VirtexClkBt,
                            tile_clk,
                            bel_clk,
                            "IBUF",
                            "CMOS",
                        )];
                        fuzz_one_extras!(ctx, "ISTD", iostd, [
                            (global_mutex "GCLKIOB", "NO"),
                            (mode "DLLIOB"),
                            (attr "OUTMUX", ""),
                            (pin "DLLFB"),
                            (pin "I"),
                            (bel_special BelKV::VirtexIsDllIob(true))
                        ], [
                            (attr "IOATTRBOX", iostd),
                            (attr "DLLFBUSED", "0"),
                            (attr "IMUX", "1")
                        ], extras);
                    }
                    for &iostd in IOSTDS_VREF_LV.iter().chain(IOSTDS_VREF_HV) {
                        let extras = vec![ExtraFeature::new(
                            ExtraFeatureKind::VirtexClkBt,
                            tile_clk,
                            bel_clk,
                            "IBUF",
                            "VREF",
                        )];
                        fuzz_one_extras!(ctx, "ISTD", iostd, [
                            (global_mutex "GCLKIOB", "NO"),
                            (global_mutex "VREF", "YES"),
                            (package package),
                            (mode "DLLIOB"),
                            (bel_special BelKV::OtherIobInput(iostd.to_string())),
                            (attr "OUTMUX", ""),
                            (pin "DLLFB"),
                            (pin "I"),
                            (bel_special BelKV::VirtexIsDllIob(true))
                        ], [
                            (attr "IOATTRBOX", iostd),
                            (attr "DLLFBUSED", "0"),
                            (attr "IMUX", "1")
                        ], extras);
                    }
                }
            }
        }
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    for attr in [
        "IDNX", "IDNA", "IDNB", "IDNC", "IDND", "IDPA", "IDPB", "IDPC", "IDPD",
    ] {
        for val in ["0", "1"] {
            let extras = vec![
                ExtraFeature::new(ExtraFeatureKind::AllIobs, "IO.L", "IOB_ALL", attr, val),
                ExtraFeature::new(ExtraFeatureKind::AllIobs, "IO.R", "IOB_ALL", attr, val),
                ExtraFeature::new(ExtraFeatureKind::AllIobs, "IO.B", "IOB_ALL", attr, val),
                ExtraFeature::new(ExtraFeatureKind::AllIobs, "IO.T", "IOB_ALL", attr, val),
            ];
            fuzz_one_extras!(ctx, attr, val, [
                (global_mutex "DRIVE", "GLOBAL")
            ], [
                (global_opt attr, val)
            ], extras);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = match edev.grid.kind {
        GridKind::Virtex => "V",
        GridKind::VirtexE | GridKind::VirtexEM => "VE",
    };
    for side in ['L', 'R', 'B', 'T'] {
        let tile = &format!("IO.{side}");
        let tile_iob = &format!("IOB.{side}.{kind}");
        let mut pdrive_all = vec![];
        let mut ndrive_all = vec![];
        for attr in ["IDPD", "IDPC", "IDPB", "IDPA"] {
            pdrive_all.push(
                ctx.extract_enum_bool_wide(tile, "IOB_ALL", attr, "0", "1")
                    .bits,
            );
        }
        for attr in ["IDND", "IDNC", "IDNB", "IDNA", "IDNX"] {
            ndrive_all.push(
                ctx.extract_enum_bool_wide(tile, "IOB_ALL", attr, "0", "1")
                    .bits,
            );
        }
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'B' | 'T')) {
                continue;
            }
            let bel = &format!("IOB{i}");

            // IOI

            let present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            let diff = ctx
                .state
                .get_diff(tile, bel, "SHORTEN_JTAG_CHAIN", "0")
                .combine(&!&present);
            let item = xlat_bit(!diff);
            ctx.tiledb.insert(tile, bel, "SHORTEN_JTAG_CHAIN", item);
            for (pin, pin_b, pinmux) in [
                ("SR", "SR_B", "SRMUX"),
                ("ICE", "ICE_B", "ICEMUX"),
                ("OCE", "OCE_B", "OCEMUX"),
                ("TCE", "TCE_B", "TCEMUX"),
                ("T", "T_TB", "TRIMUX"),
                ("O", "O_B", "OMUX"),
            ] {
                let diff0 = ctx.state.get_diff(tile, bel, pinmux, "1");
                assert_eq!(diff0, ctx.state.get_diff(tile, bel, pinmux, pin));
                let diff1 = ctx.state.get_diff(tile, bel, pinmux, "0");
                assert_eq!(diff1, ctx.state.get_diff(tile, bel, pinmux, pin_b));
                let item = xlat_bool(diff0, diff1);
                ctx.insert_int_inv(&[tile], tile, bel, pin, item);
            }
            for iot in ['I', 'O', 'T'] {
                let item = ctx.extract_enum_bool(tile, bel, &format!("{iot}CKINV"), "1", "0");
                ctx.tiledb
                    .insert(tile, bel, format!("INV.{iot}FF.CLK"), item);
                let item = ctx.extract_bit(tile, bel, &format!("{iot}INITMUX"), "0");
                ctx.tiledb
                    .insert(tile, bel, format!("{iot}FF_SR_ENABLE"), item);
            }
            let item = ctx.extract_enum_bool(tile, bel, "IFFINITATTR", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "IFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFFATTRBOX", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            ctx.state
                .get_diff(tile, bel, "FFATTRBOX", "ASYNC")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "FFATTRBOX", "SYNC");
            for iot in ['I', 'O', 'T'] {
                let init = ctx.tiledb.item(tile, bel, &format!("{iot}FF_INIT"));
                let init_bit = init.bits[0];
                let item = xlat_bitvec(vec![diff.split_bits_by(|bit| {
                    bit.tile == init_bit.tile
                        && bit.frame.abs_diff(init_bit.frame) == 1
                        && bit.bit == init_bit.bit
                })]);
                ctx.tiledb
                    .insert(tile, bel, format!("{iot}FF_SR_SYNC"), item);
            }
            diff.assert_empty();
            let item = ctx.extract_enum_bool(tile, bel, "IFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_IFF",
                TileItem::from_bit(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 2,
                            ('R', 2) => 27,
                            ('R', 3) => 32,
                            (_, 1) => 45,
                            (_, 2) => 20,
                            (_, 3) => 15,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_OFF",
                TileItem::from_bit(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 8,
                            ('R', 2) => 21,
                            ('R', 3) => 38,
                            (_, 1) => 39,
                            (_, 2) => 26,
                            (_, 3) => 9,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_TFF",
                TileItem::from_bit(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 12,
                            ('R', 2) => 17,
                            ('R', 3) => 42,
                            (_, 1) => 35,
                            (_, 2) => 30,
                            (_, 3) => 5,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );

            // IOI + IOB

            ctx.state.get_diff(tile, bel, "TSEL", "1").assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "TSEL", "0");
            let diff_ioi = diff.split_bits_by(|bit| bit.frame < 48 && bit.bit == 16);
            ctx.tiledb.insert(
                tile,
                bel,
                "TMUX",
                xlat_enum(vec![("T", Diff::default()), ("TFF", diff_ioi)]),
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "TMUX",
                xlat_enum(vec![("T", Diff::default()), ("TFF", diff)]),
            );
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "OUTMUX", "0")
                .combine(&!ctx.state.get_diff(tile, bel, "OUTMUX", "1"));
            let diff_ioi = diff.split_bits_by(|bit| bit.frame < 48 && bit.bit == 16);
            ctx.tiledb.insert(
                tile,
                bel,
                "OMUX",
                xlat_enum(vec![("O", Diff::default()), ("OFF", diff_ioi)]),
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "OMUX",
                xlat_enum(vec![("O", Diff::default()), ("OFF", diff)]),
            );

            // IOB

            ctx.tiledb.insert(
                tile_iob,
                bel,
                "READBACK_I",
                TileItem::from_bit(
                    match (side, i) {
                        ('L' | 'R', 1) => TileBit::new(0, 50, 13),
                        ('L' | 'R', 2) => TileBit::new(0, 50, 12),
                        ('L' | 'R', 3) => TileBit::new(0, 50, 2),
                        ('B' | 'T', 1) => TileBit::new(0, 25, 17),
                        ('B' | 'T', 2) => TileBit::new(0, 21, 17),
                        _ => unreachable!(),
                    },
                    false,
                ),
            );
            let item = ctx.extract_enum_default(
                tile,
                bel,
                "PULL",
                &["PULLDOWN", "PULLUP", "KEEPER"],
                "NONE",
            );
            ctx.tiledb.insert(tile_iob, bel, "PULL", item);

            if has_any_vref(edev, ctx.device, ctx.db, tile, BelId::from_idx(i)).is_some() {
                let diff = present.combine(&!&ctx.state.get_diff(tile, bel, "PRESENT", "NOT_VREF"));
                ctx.tiledb.insert(tile_iob, bel, "VREF", xlat_bit(diff));
            }

            let mut diffs_istd = vec![];
            let mut diffs_iostd_misc = HashMap::new();
            let mut diffs_iostd_misc_vec = vec![("NONE", !&present)];
            let iostds: Vec<_> = if edev.grid.kind == GridKind::Virtex {
                IOSTDS_CMOS_V
                    .iter()
                    .map(|&x| ("CMOS", x))
                    .chain(IOSTDS_VREF_LV.iter().map(|&x| ("VREF_LV", x)))
                    .chain(IOSTDS_VREF_HV.iter().map(|&x| ("VREF_HV", x)))
                    .collect()
            } else {
                IOSTDS_CMOS_VE
                    .iter()
                    .map(|&x| ("CMOS", x))
                    .chain(IOSTDS_VREF_LV.iter().map(|&x| ("VREF", x)))
                    .chain(IOSTDS_VREF_HV.iter().map(|&x| ("VREF", x)))
                    .chain(IOSTDS_DIFF.iter().map(|&x| ("DIFF", x)))
                    .collect()
            };
            for &(kind, iostd) in &iostds {
                let diff_i = ctx.state.get_diff(tile, bel, "ISTD", iostd);
                let diff_o = if iostd == "LVTTL" {
                    ctx.state
                        .peek_diff(tile, bel, "OSTD", format!("{iostd}.12.SLOW"))
                } else {
                    ctx.state
                        .peek_diff(tile, bel, "OSTD", format!("{iostd}.SLOW"))
                }
                .clone();
                let (diff_i, _, diff_c) = Diff::split(diff_i, diff_o);
                diffs_istd.push((kind, diff_i));
                diffs_iostd_misc.insert(iostd, diff_c.clone());
                diffs_iostd_misc_vec.push((iostd, diff_c));
            }
            diffs_istd.push(("NONE", Diff::default()));
            ctx.tiledb
                .insert(tile_iob, bel, "IBUF", xlat_enum(diffs_istd));

            let mut pdrive = vec![None; 4];
            let mut ndrive = vec![None; 5];
            for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                let diff = ctx
                    .state
                    .peek_diff(tile, bel, "OSTD", format!("LVTTL.{drive}.SLOW"));
                for (i, bits) in pdrive_all.iter().enumerate() {
                    for &bit in bits {
                        if let Some(&pol) = diff.bits.get(&bit) {
                            if pdrive[i].is_none() {
                                pdrive[i] = Some((bit, !pol));
                            }
                            assert_eq!(pdrive[i], Some((bit, !pol)));
                        }
                    }
                }
                for (i, bits) in ndrive_all.iter().enumerate() {
                    for &bit in bits {
                        if let Some(&pol) = diff.bits.get(&bit) {
                            if ndrive[i].is_none() {
                                ndrive[i] = Some((bit, !pol));
                            }
                            assert_eq!(ndrive[i], Some((bit, !pol)));
                        }
                    }
                }
            }
            let pdrive: Vec<_> = pdrive.into_iter().map(|x| x.unwrap()).collect();
            let ndrive: Vec<_> = ndrive.into_iter().map(|x| x.unwrap()).collect();

            let slew_bits: HashSet<_> = ctx
                .state
                .peek_diff(tile, bel, "OSTD", "LVTTL.24.FAST")
                .combine(&!ctx.state.peek_diff(tile, bel, "OSTD", "LVTTL.24.SLOW"))
                .bits
                .into_keys()
                .collect();

            let tag = if edev.grid.kind == GridKind::Virtex {
                "V"
            } else {
                "VE"
            };

            let mut slews = vec![("NONE".to_string(), Diff::default())];
            let mut ostd_misc = vec![("NONE", Diff::default())];
            for (_, iostd) in iostds {
                if iostd == "LVTTL" {
                    for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                        for slew in ["SLOW", "FAST"] {
                            let mut diff = ctx.state.get_diff(
                                tile,
                                bel,
                                "OSTD",
                                format!("{iostd}.{drive}.{slew}"),
                            );
                            let pdrive_val: BitVec = pdrive
                                .iter()
                                .map(|&(bit, inv)| {
                                    if let Some(val) = diff.bits.remove(&bit) {
                                        assert_eq!(inv, !val);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            ctx.tiledb.insert_misc_data(
                                format!("IOSTD:{tag}:PDRIVE:{iostd}.{drive}"),
                                pdrive_val,
                            );
                            let ndrive_val: BitVec = ndrive
                                .iter()
                                .map(|&(bit, inv)| {
                                    if let Some(val) = diff.bits.remove(&bit) {
                                        assert_eq!(inv, !val);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            ctx.tiledb.insert_misc_data(
                                format!("IOSTD:{tag}:NDRIVE:{iostd}.{drive}"),
                                ndrive_val,
                            );
                            slews.push((
                                format!("{iostd}.{drive}.{slew}"),
                                diff.split_bits(&slew_bits),
                            ));
                            ostd_misc.push((iostd, diff))
                        }
                    }
                } else {
                    for slew in ["SLOW", "FAST"] {
                        let mut diff =
                            ctx.state
                                .get_diff(tile, bel, "OSTD", format!("{iostd}.{slew}"));
                        let pdrive_val: BitVec = pdrive
                            .iter()
                            .map(|&(bit, inv)| {
                                if let Some(val) = diff.bits.remove(&bit) {
                                    assert_eq!(inv, !val);
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{tag}:PDRIVE:{iostd}"), pdrive_val);
                        let ndrive_val: BitVec = ndrive
                            .iter()
                            .map(|&(bit, inv)| {
                                if let Some(val) = diff.bits.remove(&bit) {
                                    assert_eq!(inv, !val);
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();
                        diff = diff.combine(&!&diffs_iostd_misc[iostd]);
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{tag}:NDRIVE:{iostd}"), ndrive_val);
                        slews.push((format!("{iostd}.{slew}"), diff.split_bits(&slew_bits)));
                        ostd_misc.push((iostd, diff))
                    }
                }
            }

            ctx.tiledb.insert(
                tile_iob,
                bel,
                "PDRIVE",
                TileItem {
                    bits: pdrive.iter().map(|&(bit, _)| bit).collect(),
                    kind: TileItemKind::BitVec {
                        invert: pdrive.iter().map(|&(_, pol)| pol).collect(),
                    },
                },
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "NDRIVE",
                TileItem {
                    bits: ndrive.iter().map(|&(bit, _)| bit).collect(),
                    kind: TileItemKind::BitVec {
                        invert: ndrive.iter().map(|&(_, pol)| pol).collect(),
                    },
                },
            );

            for (attr, item) in [
                ("IOSTD_MISC", xlat_enum(diffs_iostd_misc_vec)),
                ("OUTPUT_MISC", xlat_enum(ostd_misc)),
                ("SLEW", xlat_enum(slews)),
            ] {
                let TileItemKind::Enum { values } = item.kind else {
                    unreachable!()
                };
                for (name, val) in values {
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:{tag}:{attr}:{name}"), val);
                }
                let item = TileItem::from_bitvec(item.bits, false);
                ctx.tiledb.insert(tile_iob, bel, attr, item);
            }
        }
    }
    if edev.grid.kind != GridKind::Virtex {
        for tile in if ctx.device.name.contains("2s") {
            ["CLKB_2DLL", "CLKT_2DLL"]
        } else {
            ["CLKB_4DLL", "CLKT_4DLL"]
        } {
            for bel in ["IOFB0", "IOFB1"] {
                ctx.collect_enum_default(tile, bel, "IBUF", &["CMOS", "VREF"], "NONE");
            }
        }
    }
}
