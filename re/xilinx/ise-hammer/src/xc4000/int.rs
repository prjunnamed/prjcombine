use std::collections::{BTreeMap, HashSet, btree_map};

use prjcombine_interconnect::{
    db::{BelSlotId, CellSlotId, TileWireCoord},
    dir::DirH,
    grid::{TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_bit, xlat_enum, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind, tslots};
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{
            BaseIntPip, FuzzIntPip, NodeIntDistinct, NodeIntDstFilter, NodeIntSrcFilter,
            resolve_int_pip,
        },
        props::{
            DynProp,
            bel::{BaseBelAttr, BaseBelMode, BaseBelPin, BelMutex, FuzzBelAttr, FuzzBelMode},
            mutex::{IntMutex, NodeMutexExclusive},
            pip::{BasePip, PinFar, PipWire},
            relation::{Delta, NoopRelation, Related},
        },
    },
};

fn drive_xc4000_wire<'a>(
    backend: &IseBackend<'a>,
    fuzzer: Fuzzer<IseBackend<'a>>,
    wire_target: WireCoord,
    orig_target: Option<(TileCoord, TileWireCoord)>,
    wire_avoid: WireCoord,
) -> (Fuzzer<IseBackend<'a>>, &'a str, &'a str) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    let wname = backend.egrid.db.wires.key(wire_target.slot);
    let aname = backend.egrid.db.wires.key(wire_avoid.slot);
    let mut cell = wire_target.cell;
    let mut wt = wire_target.slot;
    let fuzzer = fuzzer.fuzz(Key::NodeMutex(wire_target), None, "EXCLUSIVE");
    // println!("DRIVING {wire_target:?} {wname}");
    if cell.row != edev.chip.row_s()
        && cell.row != edev.chip.row_n()
        && (wname == "LONG.H2" || wname == "LONG.H3")
    {
        let bel = if wname == "LONG.H3" {
            bels::TBUF1
        } else {
            bels::TBUF0
        };
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let node_naming = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let bel_naming = &node_naming.bels[bel];
        let pin_naming = &bel_naming.pins["O"];
        let site_name = &ntile.bels[bel];
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "TBUF")
            .base(Key::SitePin(site_name, "O".into()), true)
            .base(
                Key::Pip(
                    &ntile.names[bel_naming.tile],
                    &pin_naming.name,
                    &pin_naming.name_far,
                ),
                Value::FromPin(site_name, "O".into()),
            );
        (fuzzer, site_name, "O")
    } else if wname == "GND" {
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let site_name = ntile.tie_name.as_ref().unwrap();
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "TIE")
            .base(Key::SitePin(site_name, "O".into()), true);
        (fuzzer, site_name, "O")
    } else if wname.starts_with("OUT.CLB") && (wname.ends_with(".V") || wname.ends_with(".H")) {
        let owname = &wname[..(wname.len() - 2)];
        let nwt = cell.wire(backend.egrid.db.get_wire(owname));
        let (fuzzer, site_name, pin_name) =
            drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
        let (tile, wt, wf) = resolve_int_pip(
            backend,
            cell.tile(tslots::MAIN),
            TileWireCoord {
                cell: CellSlotId::from_idx(0),
                wire: wt,
            },
            TileWireCoord {
                cell: CellSlotId::from_idx(0),
                wire: nwt.slot,
            },
        )
        .unwrap();
        let fuzzer = fuzzer.base(
            Key::Pip(tile, wf, wt),
            Value::FromPin(site_name, pin_name.into()),
        );
        (fuzzer, site_name, pin_name)
    } else if wname.starts_with("OUT.CLB") {
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let site_name = &ntile.bels[bels::CLB];
        let (pin, fuzzer) = match &wname[..] {
            "OUT.CLB.FX" => (
                "X",
                fuzzer
                    .base(Key::SiteAttr(site_name, "F".into()), "#LUT:F=0x0000")
                    .base(Key::SiteAttr(site_name, "XMUX".into()), "F"),
            ),
            "OUT.CLB.GY" => (
                "Y",
                fuzzer
                    .base(Key::SiteAttr(site_name, "G".into()), "#LUT:G=0x0000")
                    .base(Key::SiteAttr(site_name, "YMUX".into()), "G"),
            ),
            "OUT.CLB.FXQ" => (
                "XQ",
                if edev.chip.kind.is_clb_xl() {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKX".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "XQMUX".into()), "QX")
                        .base(Key::SiteAttr(site_name, "FFX".into()), "#LATCH")
                        .base(Key::SiteAttr(site_name, "DX".into()), "DIN")
                        .base(Key::SiteAttr(site_name, "DIN".into()), "C1")
                } else {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKX".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "XQMUX".into()), "QX")
                },
            ),
            "OUT.CLB.GYQ" => (
                "YQ",
                if edev.chip.kind.is_clb_xl() {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKY".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "YQMUX".into()), "QY")
                        .base(Key::SiteAttr(site_name, "FFY".into()), "#LATCH")
                        .base(Key::SiteAttr(site_name, "DY".into()), "DIN")
                        .base(Key::SiteAttr(site_name, "DIN".into()), "C1")
                } else {
                    fuzzer
                        .base(Key::SiteAttr(site_name, "CLKY".into()), "CLK")
                        .base(Key::SiteAttr(site_name, "YQMUX".into()), "QY")
                },
            ),
            _ => unreachable!(),
        };
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "CLB")
            .base(Key::SitePin(site_name, pin.into()), true);
        (fuzzer, site_name, pin)
    } else if let Some(idx) = wname.strip_prefix("SINGLE.H") {
        let idx: u8 = idx.parse().unwrap();
        assert_ne!(cell.row, edev.chip.row_n());
        if cell.col == edev.chip.col_w()
            || (cell.col == edev.chip.col_w() + 1
                && (cell.row == edev.chip.row_s() || cell.row == edev.chip.row_n() - 1))
        {
            let nwt = cell.delta(1, 0).wire(wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.delta(1, 0).tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(&format!("SINGLE.H{idx}.E")),
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.col == edev.chip.col_e() {
            let nwt = cell.delta(-1, 0).wire(wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(&format!("SINGLE.H{idx}.E")),
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.row == edev.chip.row_s() {
            let nwt = cell
                .delta(0, 1)
                .wire(backend.egrid.db.get_wire(&format!("SINGLE.V{idx}")));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(&format!("SINGLE.V{idx}.S")),
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.row == edev.chip.row_n() - 1 {
            let nwt = cell.wire(backend.egrid.db.get_wire(&format!("SINGLE.V{idx}")));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: nwt.slot,
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else {
            let (out, sout, dy) = match (
                idx,
                edev.chip.kind == prjcombine_xc2000::chip::ChipKind::Xc4000E,
            ) {
                (0 | 4, true) => ("OUT.CLB.GY", "OUT.CLB.GY", 0),
                (1 | 5, true) => ("OUT.CLB.GYQ", "OUT.CLB.GYQ", 0),
                (2 | 6, true) => ("OUT.CLB.FXQ.S", "OUT.CLB.FXQ", 1),
                (3 | 7, true) => ("OUT.CLB.FX.S", "OUT.CLB.FX", 1),
                (0 | 4, false) => ("OUT.CLB.GY.V", "OUT.CLB.GY.V", 0),
                (1 | 5, false) => ("OUT.CLB.GYQ.V", "OUT.CLB.GYQ.V", 0),
                (2 | 6, false) => ("OUT.CLB.FXQ.S", "OUT.CLB.FXQ.V", 1),
                (3 | 7, false) => ("OUT.CLB.FX.S", "OUT.CLB.FX.V", 1),
                _ => unreachable!(),
            };
            let nwt = cell.delta(0, dy).wire(backend.egrid.db.get_wire(sout));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(out),
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        }
    } else if let Some(idx) = wname.strip_prefix("SINGLE.V") {
        let idx: u8 = idx.parse().unwrap();
        assert_ne!(cell.col, edev.chip.col_w());
        if cell.row == edev.chip.row_s() {
            let nwt = cell.delta(0, 1).wire(wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(&format!("SINGLE.V{idx}.S")),
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.row == edev.chip.row_n()
            || (cell.row == edev.chip.row_n() - 1
                && (cell.col == edev.chip.col_w() + 1 || cell.col == edev.chip.col_e()))
        {
            let nwt = cell.delta(0, -1).wire(wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.delta(0, -1).tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(&format!("SINGLE.V{idx}.S")),
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.col == edev.chip.col_w() + 1 {
            let nwt = cell.wire(backend.egrid.db.get_wire(&format!("SINGLE.H{idx}")));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: nwt.slot,
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.col == edev.chip.col_e() {
            let nwt = cell
                .delta(-1, 0)
                .wire(backend.egrid.db.get_wire(&format!("SINGLE.H{idx}")));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(&format!("SINGLE.H{idx}.E")),
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else {
            let (out, sout, dx) = match (
                idx,
                edev.chip.kind == prjcombine_xc2000::chip::ChipKind::Xc4000E,
            ) {
                (0 | 4, true) => ("OUT.CLB.FXQ", "OUT.CLB.FXQ", 0),
                (1 | 5, true) => ("OUT.CLB.FX", "OUT.CLB.FX", 0),
                (2 | 6, true) => ("OUT.CLB.GY.E", "OUT.CLB.GY", -1),
                (3 | 7, true) => ("OUT.CLB.GYQ.E", "OUT.CLB.GYQ", -1),
                (0 | 4, false) => ("OUT.CLB.FXQ.H", "OUT.CLB.FXQ.H", 0),
                (1 | 5, false) => ("OUT.CLB.FX.H", "OUT.CLB.FX.H", 0),
                (2 | 6, false) => ("OUT.CLB.GY.E", "OUT.CLB.GY.H", -1),
                (3 | 7, false) => ("OUT.CLB.GYQ.E", "OUT.CLB.GYQ.H", -1),
                _ => unreachable!(),
            };
            let nwt = cell.delta(dx, 0).wire(backend.egrid.db.get_wire(sout));
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: wt,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire(out),
                },
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        }
    } else if wname.starts_with("LONG")
        || wname.starts_with("IO.OCTAL")
        || wname.starts_with("QUAD")
        || wname.starts_with("OCTAL")
        || wname.starts_with("GCLK")
        || wname.starts_with("VCLK")
    {
        let mut filter = None;
        let mut twt = CellSlotId::from_idx(0);
        let mut tslot = tslots::MAIN;
        if wname.starts_with("LONG") {
            if wname.contains(".H") {
                if cell.col == edev.chip.col_w() {
                    cell.col += 1;
                }
                if cell.col == wire_avoid.cell.col {
                    cell.col += 1;
                }
            } else if wname.contains(".V") {
                if cell.row == wire_avoid.cell.row {
                    cell.row += 1;
                }
            } else {
                unreachable!()
            }
        } else if wname.starts_with("IO.OCTAL") {
            match &wname[..] {
                "IO.OCTAL.W.0" => (),
                "IO.OCTAL.E.0" => {
                    assert_ne!(cell.row, edev.chip.row_n());
                    cell.row += 1;
                    wt = backend.egrid.db.get_wire("IO.OCTAL.E.1");
                    if cell.row == edev.chip.row_n() {
                        wt = backend.egrid.db.get_wire("IO.OCTAL.N.1");
                        cell.col -= 1;
                    }
                }
                "IO.OCTAL.S.0" => (),
                "IO.OCTAL.N.0" => {
                    assert_ne!(cell.col, edev.chip.col_w());
                    cell.col -= 1;
                    wt = backend.egrid.db.get_wire("IO.OCTAL.N.1");
                    if cell.col == edev.chip.col_w() {
                        wt = backend.egrid.db.get_wire("IO.OCTAL.W.1");
                        cell.row -= 1;
                    }
                }
                _ => unreachable!(),
            }
        } else if wname.starts_with("QUAD.H") {
            if cell.col == edev.chip.col_w() {
                if wname.ends_with(".3") {
                    if aname.starts_with("LONG.IO") {
                        cell.col += 1;
                        match &wname[..] {
                            "QUAD.H0.3" => {
                                filter = Some("QUAD.H0.0");
                                wt = backend.egrid.db.get_wire("QUAD.H0.4");
                            }
                            "QUAD.H1.3" => {
                                filter = Some("QUAD.H1.0");
                                wt = backend.egrid.db.get_wire("QUAD.H1.4");
                            }
                            "QUAD.H2.3" => {
                                filter = Some("QUAD.H2.0");
                                wt = backend.egrid.db.get_wire("QUAD.H2.4");
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else if wname == "QUAD.H1.0" {
                    cell.col += 1;
                    wt = backend.egrid.db.get_wire("QUAD.H1.1");
                }
            } else if wname == "QUAD.H2.0" {
                if cell.col == edev.chip.col_e() {
                    if aname.starts_with("LONG.IO") {
                        filter = Some("QUAD.H2.4");
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else {
                    cell.col += 1;
                    wt = backend.egrid.db.get_wire("QUAD.H2.1");
                }
            }
        } else if wname.starts_with("QUAD.V") {
            if cell.row == edev.chip.row_n() {
                if wname.ends_with(".3") {
                    if aname.starts_with("LONG.IO") {
                        cell.row -= 1;
                        match &wname[..] {
                            "QUAD.V0.3" => {
                                filter = Some("QUAD.V0.0");
                                wt = backend.egrid.db.get_wire("QUAD.V0.4");
                            }
                            "QUAD.V1.3" => {
                                filter = Some("QUAD.V1.0");
                                wt = backend.egrid.db.get_wire("QUAD.V1.4");
                            }
                            "QUAD.V2.3" => {
                                filter = Some("QUAD.V2.0");
                                wt = backend.egrid.db.get_wire("QUAD.V2.4");
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else if wname == "QUAD.V2.2" {
                    cell.row -= 1;
                    wt = backend.egrid.db.get_wire("QUAD.V2.3");
                }
            } else if wname == "QUAD.V0.0" {
                if cell.row == edev.chip.row_s() {
                    if aname.starts_with("LONG.IO") {
                        filter = Some("QUAD.V0.4");
                    } else {
                        filter = Some("LONG.IO");
                    }
                } else {
                    cell.row -= 1;
                    wt = backend.egrid.db.get_wire("QUAD.V0.1");
                }
            }
        } else if let Some(idx) = wname.strip_prefix("OCTAL.H.") {
            if cell.col == edev.chip.col_w() {
                let idx: usize = idx.parse().unwrap();
                cell.col += 7 - idx;
                wt = backend.egrid.db.get_wire("OCTAL.H.7");
            }
        } else if let Some(idx) = wname.strip_prefix("OCTAL.V.") {
            if cell.row == edev.chip.row_n() {
                let idx: usize = idx.parse().unwrap();
                cell.row -= 7 - idx;
                wt = backend.egrid.db.get_wire("OCTAL.V.7");
            }
        } else if wname.starts_with("GCLK") {
            if cell.row == edev.chip.row_s() {
                cell.row = edev.chip.row_qb();
            } else {
                cell.row = edev.chip.row_qt();
            }
            tslot = tslots::EXTRA_ROW;
        } else if wname == "VCLK" {
            if cell.row == edev.chip.row_s() {
                // OK
            } else if cell.row == edev.chip.row_qb() {
                cell.row = edev.chip.row_mid();
                tslot = tslots::EXTRA_ROW;
            } else if cell.row == edev.chip.row_mid() {
                twt = CellSlotId::from_idx(1);
                tslot = tslots::EXTRA_ROW;
            } else if cell.row == edev.chip.row_qt() {
                cell.row = edev.chip.row_n();
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
        let tcrd = cell.tile(tslot);
        let tile = backend.egrid.tile(tcrd);
        let mwt = TileWireCoord {
            cell: twt,
            wire: wt,
        };
        let res = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, mwt))
            .unwrap();
        assert_eq!(res, wire_target);
        let mux = &backend.egrid.db.tile_classes[tile.class].muxes[&mwt];
        for &mwf in &mux.ins {
            let wfname = backend.egrid.db.wires.key(mwf.wire);
            if let Some(filter) = filter {
                if !wfname.starts_with(filter) {
                    continue;
                }
            } else {
                if !(wfname.starts_with("SINGLE")
                    || wfname == "GND"
                    || (wfname.starts_with("IO.DOUBLE")
                        && (wname.starts_with("OCTAL")
                            || wname.starts_with("QUAD")
                            || wname == "VCLK")))
                {
                    continue;
                }
            }
            let nwt = backend
                .egrid
                .resolve_wire(backend.egrid.tile_wire(tcrd, mwf))
                .unwrap();
            if nwt == wire_avoid {
                continue;
            }
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, Some((tcrd, mwf)), wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(backend, tcrd, mwt, mwf).unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            return (fuzzer, site_name, pin_name);
        }
        panic!("umm failed at {wire_target:?} {wname}");
    } else if wname.starts_with("IO.DOUBLE") {
        let (tcrd, mwt) = orig_target.unwrap();
        let node = backend.egrid.tile(tcrd);
        let mux = &backend.egrid.db.tile_classes[node.class].muxes[&mwt];
        for &mwf in &mux.ins {
            let wfname = backend.egrid.db.wires.key(mwf.wire);
            if !wfname.starts_with("SINGLE") {
                continue;
            }
            let nwt = backend
                .egrid
                .resolve_wire(backend.egrid.tile_wire(tcrd, mwf))
                .unwrap();
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(backend, tcrd, mwt, mwf).unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            return (fuzzer, site_name, pin_name);
        }
        panic!("umm failed at {wire_target:?} {wname}");
    } else {
        panic!("how to drive {wname}");
    }
}

#[derive(Clone, Debug)]
struct Xc4000DoublePip {
    wire_to: TileWireCoord,
    wire_mid: TileWireCoord,
    wire_from: TileWireCoord,
}

impl Xc4000DoublePip {
    fn new(wire_to: TileWireCoord, wire_mid: TileWireCoord, wire_from: TileWireCoord) -> Self {
        Self {
            wire_to,
            wire_mid,
            wire_from,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Xc4000DoublePip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let res_from = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire_from))
            .unwrap();
        let res_mid = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire_mid))
            .unwrap();
        let res_to = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire_to))
            .unwrap();
        let fuzzer = fuzzer
            .fuzz(Key::NodeMutex(res_to), None, "EXCLUSIVE-TGT")
            .fuzz(Key::NodeMutex(res_mid), None, "EXCLUSIVE-MID");
        let (fuzzer, src_site, src_pin) = drive_xc4000_wire(
            backend,
            fuzzer,
            res_from,
            Some((tcrd, self.wire_from)),
            res_to,
        );
        let (tile0, wt0, wf0) = resolve_int_pip(backend, tcrd, self.wire_mid, self.wire_from)?;
        let (tile1, wt1, wf1) = resolve_int_pip(backend, tcrd, self.wire_to, self.wire_mid)?;
        Some((
            fuzzer
                .fuzz(
                    Key::Pip(tile0, wf0, wt0),
                    None,
                    Value::FromPin(src_site, src_pin.into()),
                )
                .fuzz(
                    Key::Pip(tile1, wf1, wt1),
                    None,
                    Value::FromPin(src_site, src_pin.into()),
                ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
struct Xc4000BiPip {
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
}

impl Xc4000BiPip {
    fn new(wire_to: TileWireCoord, wire_from: TileWireCoord) -> Self {
        Self { wire_to, wire_from }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Xc4000BiPip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let res_from = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire_from))
            .unwrap();
        let res_to = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, self.wire_to))
            .unwrap();
        let fuzzer = fuzzer.fuzz(Key::NodeMutex(res_to), None, "EXCLUSIVE-TGT");
        let (fuzzer, src_site, src_pin) = drive_xc4000_wire(
            backend,
            fuzzer,
            res_from,
            Some((tcrd, self.wire_from)),
            res_to,
        );
        let (tile, wt, wf) = resolve_int_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        Some((
            fuzzer.fuzz(
                Key::Pip(tile, wf, wt),
                None,
                Value::FromPin(src_site, src_pin.into()),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
struct Xc4000TbufSplitter {
    pub slot: BelSlotId,
    pub dir: DirH,
    pub buf: bool,
}

impl Xc4000TbufSplitter {
    fn new(slot: BelSlotId, dir: DirH, buf: bool) -> Self {
        Self { slot, dir, buf }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Xc4000TbufSplitter {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let node = backend.egrid.tile(tcrd);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let node_data = &backend.egrid.db.tile_classes[node.class];
        let bel_data = &node_data.bels[self.slot];
        let node_naming = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let bel_naming = &node_naming.bels[self.slot];

        let (wire_from, wire_to, pin_from, pin_to, ex_from, ex_to) = match self.dir {
            DirH::E => (
                bel_data.pins["L"].wires.iter().copied().next().unwrap(),
                bel_data.pins["R"].wires.iter().copied().next().unwrap(),
                &bel_naming.pins["L"].name,
                &bel_naming.pins["R"].name,
                &bel_naming.pins["L.EXCL"].name,
                &bel_naming.pins["R.EXCL"].name,
            ),
            DirH::W => (
                bel_data.pins["R"].wires.iter().copied().next().unwrap(),
                bel_data.pins["L"].wires.iter().copied().next().unwrap(),
                &bel_naming.pins["R"].name,
                &bel_naming.pins["L"].name,
                &bel_naming.pins["R.EXCL"].name,
                &bel_naming.pins["L.EXCL"].name,
            ),
        };
        let res_from = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, wire_from))
            .unwrap();
        let res_to = backend
            .egrid
            .resolve_wire(backend.egrid.tile_wire(tcrd, wire_to))
            .unwrap();
        let fuzzer = fuzzer.fuzz(Key::NodeMutex(res_to), None, "EXCLUSIVE-TGT");
        let (fuzzer, src_site, src_pin) =
            drive_xc4000_wire(backend, fuzzer, res_from, Some((tcrd, wire_from)), res_to);
        let tile = &ntile.names[bel_naming.tile];
        let fuzzer = if self.buf {
            fuzzer
                .fuzz(
                    Key::Pip(tile, pin_from, ex_from),
                    None,
                    Value::FromPin(src_site, src_pin.into()),
                )
                .fuzz(
                    Key::Pip(tile, ex_from, ex_to),
                    None,
                    Value::FromPin(src_site, src_pin.into()),
                )
                .fuzz(
                    Key::Pip(tile, ex_to, pin_to),
                    None,
                    Value::FromPin(src_site, src_pin.into()),
                )
        } else {
            fuzzer.fuzz(
                Key::Pip(tile, pin_from, pin_to),
                None,
                Value::FromPin(src_site, src_pin.into()),
            )
        };
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };
    let kind = edev.chip.kind;
    let intdb = backend.egrid.db;
    for (_, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.wire);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt:#}.{out_name}", wtt = wire_to.cell)
            } else {
                assert_eq!(wire_to.cell.to_idx(), 0);
                format!("MUX.{out_name}")
            };
            if kind == ChipKind::SpartanXl {
                if out_name == "IMUX.CLB.C2" && matches!(&tile[..], "CLB.T" | "CLB.LT" | "CLB.RT") {
                    continue;
                }
                if out_name == "IMUX.CLB.C3" && matches!(&tile[..], "CLB.L" | "CLB.LB" | "CLB.LT") {
                    continue;
                }
            }
            if out_name.starts_with("QBUF") || out_name.ends_with("EXCL") {
                let wire_mid = wire_to;
                for &wire_to in &mux.ins {
                    let wtname = format!("{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire));
                    if wtname.contains("CLK") {
                        continue;
                    }
                    for &wire_from in &mux.ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        let wfname =
                            format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                        ctx.build()
                            .prop(IntMutex::new("MAIN".to_string()))
                            .test_manual(
                                "INT",
                                format!("DMUX.{out_name}"),
                                format!("{wtname}.{wfname}"),
                            )
                            .prop(Xc4000DoublePip::new(wire_to, wire_mid, wire_from))
                            .commit();
                    }
                }
                continue;
            }
            for &wire_from in &mux.ins {
                let wire_from_name = intdb.wires.key(wire_from.wire);
                let in_name = format!("{:#}.{}", wire_from.cell, wire_from_name);

                let mut is_bidi = false;
                if let Some(mux) = node.muxes.get(&wire_from) {
                    if mux.ins.contains(&wire_to) {
                        is_bidi = true;
                    }
                }
                let tbuf_i_wire = if wire_from_name == "LONG.H2" {
                    Some("IMUX.TBUF0.I")
                } else if wire_from_name == "LONG.H3" {
                    Some("IMUX.TBUF1.I")
                } else {
                    None
                };
                if let Some(tbuf_i_wire) = tbuf_i_wire {
                    let tbuf_i_wire = backend.egrid.db.get_wire(tbuf_i_wire);
                    if let Some(mux) = node.muxes.get(&TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: tbuf_i_wire,
                    }) {
                        if mux.ins.contains(&wire_to) {
                            is_bidi = true;
                        }
                    }
                }

                let mut is_bipass = false;
                let is_wt_sd = out_name.starts_with("SINGLE")
                    || out_name.starts_with("DOUBLE")
                    || out_name.starts_with("QUAD")
                    || out_name.starts_with("IO.DOUBLE");
                let is_wf_sd = wire_from_name.starts_with("SINGLE")
                    || wire_from_name.starts_with("DOUBLE")
                    || wire_from_name.starts_with("QUAD")
                    || wire_from_name.starts_with("IO.DOUBLE");
                if is_wt_sd && is_wf_sd {
                    is_bipass = true;
                }
                if out_name.starts_with("IO.OCTAL") && wire_from_name.starts_with("SINGLE") {
                    is_bipass = true;
                }
                if out_name.starts_with("SINGLE") && wire_from_name.starts_with("IO.OCTAL") {
                    is_bipass = true;
                }
                if out_name.starts_with("DEC") && wire_from_name.starts_with("DEC") {
                    is_bipass = true;
                }

                if wire_from_name.starts_with("QBUF") || wire_from_name.ends_with("EXCL") {
                    continue;
                }

                if is_bidi && !is_bipass {
                    ctx.build()
                        .prop(IntMutex::new("MAIN".to_string()))
                        .test_manual("INT", &mux_name, &in_name)
                        .prop(Xc4000BiPip::new(wire_to, wire_from))
                        .commit();
                } else {
                    let mut builder = ctx
                        .build()
                        .prop(NodeIntDistinct::new(wire_to, wire_from))
                        .prop(NodeIntDstFilter::new(wire_to))
                        .prop(NodeIntSrcFilter::new(wire_from))
                        .prop(IntMutex::new("MAIN".to_string()))
                        .test_manual("INT", &mux_name, &in_name)
                        .prop(NodeMutexExclusive::new(wire_to))
                        .prop(NodeMutexExclusive::new(wire_from))
                        .prop(FuzzIntPip::new(wire_to, wire_from));
                    if tile == "CNR.TR"
                        && (in_name.contains("OUT.LR.IOB1.I") || in_name.contains("OUT.OSC"))
                    {
                        // sigh.
                        builder = builder
                            .prop(BelMutex::new(bels::OSC, "MODE".into(), "INT".into()))
                            .prop(BasePip::new(
                                NoopRelation,
                                PipWire::BelPinNear(bels::OSC, "OUT0".into()),
                                PipWire::BelPinNear(bels::OSC, "F15".into()),
                            ));
                    }
                    if tile == "IO.R.T"
                        && (in_name.contains("OUT.LR.IOB1.I") && in_name.ends_with(".S"))
                    {
                        // sigh.
                        let bel = bels::OSC;
                        builder = builder
                            .prop(Related::new(
                                Delta::new(0, 1, "CNR.TR"),
                                BelMutex::new(bel, "MODE".into(), "INT".into()),
                            ))
                            .prop(BasePip::new(
                                Delta::new(0, 1, "CNR.TR"),
                                PipWire::BelPinNear(bel, "OUT0".into()),
                                PipWire::BelPinNear(bel, "F15".into()),
                            ));
                    }

                    if out_name == "IMUX.IOB0.TS" || out_name == "IMUX.IOB1.TS" {
                        let idx = if out_name == "IMUX.IOB0.TS" { 0 } else { 1 };
                        let bel = bels::IO[idx];
                        builder = builder
                            .prop(BaseBelMode::new(bel, "IOB".into()))
                            .prop(BaseBelAttr::new(bel, "TRI".into(), "T".into()))
                            .prop(BaseBelPin::new(bel, "T".into()));
                        if edev.chip.kind != ChipKind::Xc4000E {
                            builder =
                                builder.prop(BaseBelAttr::new(bel, "OUTMUX".into(), "O".into()));
                        }
                    }

                    if out_name.starts_with("IMUX.TBUF") {
                        let idx = if out_name.starts_with("IMUX.TBUF0") {
                            0
                        } else {
                            1
                        };
                        let bel = bels::TBUF[idx];
                        if out_name.ends_with("I") {
                            builder = builder.prop(BaseBelMode::new(bel, "TBUF".into())).prop(
                                FuzzBelAttr::new(
                                    bel,
                                    "TBUFATTR".into(),
                                    "WANDT".into(),
                                    "WORAND".into(),
                                ),
                            );
                        } else {
                            builder = builder
                                .prop(FuzzBelMode::new(bel, "".into(), "TBUF".into()))
                                .prop(FuzzBelAttr::new(
                                    bel,
                                    "TBUFATTR".into(),
                                    "".into(),
                                    "WANDT".into(),
                                ));
                        }
                    }
                    builder.commit();
                }
            }
        }
        if tile.starts_with("CLB") {
            ctx.build()
                .prop(BaseBelMode::new(bels::CLB, "CLB".into()))
                .test_manual("INT", "MUX.IMUX.CLB.F4", "CIN")
                .prop(FuzzBelAttr::new(
                    bels::CLB,
                    "F4MUX".into(),
                    "".into(),
                    "CIN".into(),
                ))
                .prop(NodeMutexExclusive::new(TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire("IMUX.CLB.F4"),
                }))
                .commit();
        }
        if tile.starts_with("IO.R")
            || matches!(
                &tile[..],
                "CLB" | "CLB.B" | "CLB.T" | "CLB.R" | "CLB.RB" | "CLB.RT"
            )
        {
            let tgt_node = if tile == "CLB.R" {
                "CLB"
            } else if tile == "CLB.RB" {
                "CLB.B"
            } else if tile == "CLB.RT" {
                "CLB.T"
            } else if tile.starts_with("CLB") {
                tile
            } else if tile == "IO.R.T" {
                "CLB.RT"
            } else if tile == "IO.RS.B" {
                "CLB.RB"
            } else {
                "CLB.R"
            };
            ctx.build()
                .prop(Related::new(
                    Delta::new(-1, 0, tgt_node),
                    BaseBelMode::new(bels::CLB, "CLB".into()),
                ))
                .test_manual("INT", "MUX.IMUX.CLB.G3", "CIN")
                .prop(Related::new(
                    Delta::new(-1, 0, tgt_node),
                    FuzzBelAttr::new(bels::CLB, "G3MUX".into(), "".into(), "CIN".into()),
                ))
                .prop(NodeMutexExclusive::new(TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire("IMUX.CLB.G3"),
                }))
                .commit();
        }
        if tile.starts_with("IO.B")
            || matches!(
                &tile[..],
                "CLB" | "CLB.B" | "CLB.L" | "CLB.LB" | "CLB.R" | "CLB.RB"
            )
        {
            let tgt_node = if tile == "CLB" || tile == "CLB.B" {
                "CLB"
            } else if tile == "CLB.R" || tile == "CLB.RB" {
                "CLB.R"
            } else if tile == "CLB.L" || tile == "CLB.LB" {
                "CLB.L"
            } else if tile == "IO.BS.L" {
                "CLB.LB"
            } else if tile == "IO.B.R" {
                "CLB.RB"
            } else {
                "CLB.B"
            };
            ctx.build()
                .prop(Related::new(
                    Delta::new(0, 1, tgt_node),
                    BaseBelMode::new(bels::CLB, "CLB".into()),
                ))
                .test_manual("INT", "MUX.IMUX.CLB.G2", "COUT0")
                .prop(Related::new(
                    Delta::new(0, 1, tgt_node),
                    FuzzBelAttr::new(bels::CLB, "G2MUX".into(), "".into(), "COUT0".into()),
                ))
                .prop(NodeMutexExclusive::new(TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: backend.egrid.db.get_wire("IMUX.CLB.G2"),
                }))
                .commit();
        }
        if tile.starts_with("CLB") || tile.starts_with("IO.R") || tile.starts_with("IO.L") {
            for idx in 0..2 {
                let bel = bels::TBUF[idx];
                ctx.build()
                    .test_manual("INT", format!("MUX.IMUX.TBUF{idx}.TS"), "GND")
                    .prop(FuzzBelMode::new(bel, "".into(), "TBUF".into()))
                    .prop(FuzzBelAttr::new(
                        bel,
                        "TBUFATTR".into(),
                        "".into(),
                        "WAND".into(),
                    ))
                    .prop(NodeMutexExclusive::new(TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire(&format!("IMUX.TBUF{idx}.TS")),
                    }))
                    .commit();
            }
            for idx in 0..2 {
                let bel = bels::TBUF[idx];
                let mut bctx = ctx.bel(bel);
                if kind.is_clb_xl() && tile.starts_with("CLB") {
                    let wt = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire(&format!("IMUX.TBUF{idx}.TS")),
                    };
                    let wf = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire("LONG.V0"),
                    };
                    bctx.mode("TBUF")
                        .prop(BaseIntPip::new(wt, wf))
                        .test_manual("DRIVE1", "1")
                        .attr_diff("TBUFATTR", "WORAND", "TBUF")
                        .prop(NodeMutexExclusive::new(wt))
                        .prop(NodeMutexExclusive::new(wf))
                        .commit();
                } else {
                    bctx.mode("TBUF")
                        .test_manual("DRIVE1", "1")
                        .attr_diff("TBUFATTR", "WORAND", "TBUF")
                        .commit();
                }
            }
        }
        if tile.starts_with("IO") {
            for idx in 0..2 {
                let bel = bels::IO[idx];
                ctx.build()
                    .prop(BaseBelMode::new(bel, "IOB".into()))
                    .prop(BaseBelAttr::new(bel, "OUTMUX".into(), "O".into()))
                    .test_manual("INT", format!("MUX.IMUX.IOB{idx}.TS"), "GND")
                    .prop(FuzzBelAttr::new(bel, "TRI".into(), "T".into(), "".into()))
                    .commit();
            }
        }
        if tile.starts_with("LLV.") {
            let mut bctx = ctx.bel(bels::CLKH);
            if edev.chip.kind == ChipKind::SpartanXl {
                for opin in ["O0", "O1", "O2", "O3"] {
                    for ipin in [
                        "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H",
                        "I.UR.V",
                    ] {
                        bctx.build()
                            .mutex(format!("MUX.{opin}"), ipin)
                            .mutex(format!("OUT.{ipin}"), opin)
                            .test_manual(format!("MUX.{opin}"), ipin)
                            .pip(opin, ipin)
                            .commit();
                    }
                }
            } else {
                for (opin, ipin_p) in [
                    ("O0", "I.UL.V"),
                    ("O1", "I.LL.H"),
                    ("O2", "I.LR.V"),
                    ("O3", "I.UR.H"),
                ] {
                    for ipin in [ipin_p, "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"] {
                        bctx.build()
                            .mutex(format!("MUX.{opin}"), ipin)
                            .mutex(format!("OUT.{ipin}"), opin)
                            .test_manual(format!("MUX.{opin}"), ipin)
                            .pip(opin, ipin)
                            .commit();
                    }
                }
            }
        }
        if tile.starts_with("CNR") {
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                for (rtile, opt, bel, out, inp) in [
                    (
                        "CNR.TL",
                        "GCLK1",
                        bels::BUFGLS_V,
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.N",
                    ),
                    (
                        "CNR.BL",
                        "GCLK2",
                        bels::BUFGLS_V,
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.S",
                    ),
                    (
                        "CNR.BL",
                        "GCLK3",
                        bels::BUFGLS_H,
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.W",
                    ),
                    (
                        "CNR.BR",
                        "GCLK4",
                        bels::BUFGLS_H,
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.E",
                    ),
                    (
                        "CNR.BR",
                        "GCLK5",
                        bels::BUFGLS_V,
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.S",
                    ),
                    (
                        "CNR.TR",
                        "GCLK6",
                        bels::BUFGLS_V,
                        "IMUX.BUFG.V",
                        "OUT.IOB.CLKIN.N",
                    ),
                    (
                        "CNR.TR",
                        "GCLK7",
                        bels::BUFGLS_H,
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.E",
                    ),
                    (
                        "CNR.TL",
                        "GCLK8",
                        bels::BUFGLS_H,
                        "IMUX.BUFG.H",
                        "OUT.IOB.CLKIN.W",
                    ),
                ] {
                    if rtile != tile {
                        continue;
                    }
                    let mut bctx = ctx.bel(bel);
                    let wt = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire(out),
                    };
                    let wf = TileWireCoord {
                        cell: CellSlotId::from_idx(0),
                        wire: backend.egrid.db.get_wire(inp),
                    };
                    bctx.build()
                        .prop(BaseIntPip::new(wt, wf))
                        .test_manual("ALT_PAD", "1")
                        .global(opt, "ALTPAD")
                        .prop(NodeMutexExclusive::new(wt))
                        .prop(NodeMutexExclusive::new(wf))
                        .commit();
                    bctx.build()
                        .prop(BaseIntPip::new(wt, wf))
                        .test_manual("CLK_EN", "1")
                        .global(opt, "CLKEN")
                        .prop(NodeMutexExclusive::new(wt))
                        .prop(NodeMutexExclusive::new(wf))
                        .commit();
                }
            }
            if edev.chip.kind != ChipKind::SpartanXl {
                for slots in [bels::PULLUP_DEC_H, bels::PULLUP_DEC_V] {
                    for slot in slots {
                        let mut bctx = ctx.bel(slot);
                        bctx.build()
                            .test_manual("ENABLE", "1")
                            .pip((PinFar, "O"), "O")
                            .commit();
                    }
                }
            }
        }
        if tile.starts_with("IO.L") || tile.starts_with("IO.R") {
            for i in 0..2 {
                let mut bctx = ctx.bel(bels::PULLUP_TBUF[i]);
                bctx.build()
                    .test_manual("ENABLE", "1")
                    .pip((PinFar, "O"), "O")
                    .commit();
            }
        }
        if matches!(
            &tile[..],
            "LLHC.CLB" | "LLHC.CLB.B" | "LLHQ.CLB" | "LLHQ.CLB.B" | "LLHQ.CLB.T"
        ) {
            for slots in [bels::PULLUP_TBUF_E, bels::PULLUP_TBUF_W] {
                for slot in slots {
                    let mut bctx = ctx.bel(slot);
                    bctx.build()
                        .test_manual("ENABLE", "1")
                        .pip((PinFar, "O"), "O")
                        .commit();
                }
            }
        }
        if edev.chip.kind != ChipKind::Xc4000E
            && matches!(
                &tile[..],
                "LLHC.CLB" | "LLHC.CLB.B" | "LLH.CLB" | "LLH.CLB.B"
            )
        {
            for bel in [bels::TBUF_SPLITTER0, bels::TBUF_SPLITTER1] {
                let mut bctx = ctx.bel(bel);
                for (val, dir, buf) in [
                    ("W", DirH::W, false),
                    ("E", DirH::E, false),
                    ("W.BUF", DirH::W, true),
                    ("E.BUF", DirH::E, true),
                ] {
                    bctx.test_manual("BUF", val)
                        .prop(Xc4000TbufSplitter::new(bel, dir, buf))
                        .commit();
                }
            }
        }
        if edev.chip.kind != ChipKind::SpartanXl {
            if matches!(&tile[..], "LLVC.IO.L" | "LLVC.IO.R") {
                for slots in [bels::PULLUP_DEC_S, bels::PULLUP_DEC_N] {
                    for slot in slots {
                        let mut bctx = ctx.bel(slot);
                        bctx.build()
                            .test_manual("ENABLE", "1")
                            .pip((PinFar, "O"), "O")
                            .commit();
                    }
                }
            }
            if matches!(&tile[..], "LLHC.IO.B" | "LLHC.IO.T") {
                for slots in [bels::PULLUP_DEC_W, bels::PULLUP_DEC_E] {
                    for slot in slots {
                        let mut bctx = ctx.bel(slot);
                        bctx.build()
                            .test_manual("ENABLE", "1")
                            .pip((PinFar, "O"), "O")
                            .commit();
                    }
                }
            }
            if tile.starts_with("IO") {
                for i in 0..3 {
                    let mut bctx = ctx.bel(bels::DEC[i]);
                    for j in 1..=4 {
                        for val in ["I", "NOT"] {
                            bctx.mode("DECODER")
                                .pin(format!("O{j}"))
                                .pin("I")
                                .test_manual(format!("O{j}MUX"), val)
                                .attr(format!("O{j}MUX"), val)
                                .pip((PinFar, format!("O{j}")), format!("O{j}"))
                                .commit();
                        }
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = edev.chip.kind;
    let egrid = ctx.edev.egrid();
    let intdb = egrid.db;
    for (node_kind, tile, node) in &intdb.tile_classes {
        if node.muxes.is_empty() {
            continue;
        }
        if egrid.tile_index[node_kind].is_empty() {
            continue;
        }
        let mut mux_diffs: BTreeMap<TileWireCoord, BTreeMap<TileWireCoord, Diff>> = BTreeMap::new();
        let mut obuf_diffs: BTreeMap<TileWireCoord, BTreeMap<TileWireCoord, Diff>> =
            BTreeMap::new();
        for (&wire_to, mux) in &node.muxes {
            let out_name = intdb.wires.key(wire_to.wire);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt:#}.{out_name}", wtt = wire_to.cell)
            } else {
                format!("MUX.{out_name}")
            };

            if out_name.starts_with("QBUF") {
                let wire_mid = wire_to;
                for &wire_to in &mux.ins {
                    let wtname = format!("{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire));
                    let mut diffs = vec![];
                    for &wire_from in &mux.ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        let wfname =
                            format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                        let diff = ctx.state.get_diff(
                            tile,
                            "INT",
                            format!("DMUX.{out_name}"),
                            format!("{wtname}.{wfname}"),
                        );
                        diffs.push((wire_from, diff.clone()));
                    }
                    let mut odiff = diffs[0].1.clone();
                    for (_, diff) in &diffs {
                        odiff.bits.retain(|bit, _| diff.bits.contains_key(bit));
                    }
                    for (_, diff) in &mut diffs {
                        *diff = diff.combine(&!&odiff);
                    }
                    mux_diffs
                        .entry(wire_to)
                        .or_default()
                        .insert(wire_mid, odiff);
                    for (wire_from, diff) in diffs {
                        match mux_diffs.entry(wire_mid).or_default().entry(wire_from) {
                            btree_map::Entry::Vacant(entry) => {
                                entry.insert(diff);
                            }
                            btree_map::Entry::Occupied(entry) => {
                                assert_eq!(*entry.get(), diff);
                            }
                        }
                    }
                }
                continue;
            }
            if out_name.ends_with("EXCL") {
                for &wire_to in &mux.ins {
                    let wtname = format!("{:#}.{}", wire_to.cell, intdb.wires.key(wire_to.wire));
                    if wtname.contains("CLK") {
                        continue;
                    }
                    for &wire_from in &mux.ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        let wfname =
                            format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                        let diff = ctx.state.get_diff(
                            tile,
                            "INT",
                            format!("DMUX.{out_name}"),
                            format!("{wtname}.{wfname}"),
                        );
                        if diff.bits.is_empty() {
                            assert!(wfname.contains("CLK"));
                            continue;
                        }
                        mux_diffs
                            .entry(wire_to)
                            .or_default()
                            .insert(wire_from, diff);
                    }
                }
                continue;
            }
            if !out_name.starts_with("IMUX")
                && !out_name.starts_with("VCLK")
                && !out_name.starts_with("ECLK")
                && !out_name.starts_with("GCLK")
                && !out_name.starts_with("IO.DBUF")
            {
                for &wire_from in &mux.ins {
                    let wfname = intdb.wires.key(wire_from.wire);
                    if wfname.starts_with("QBUF") || wfname.ends_with("EXCL") {
                        continue;
                    }
                    let in_name = format!("{:#}.{}", wire_from.cell, wfname);
                    let diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                    if out_name.contains("OCTAL")
                        && wfname.contains("OCTAL")
                        && tile.starts_with("IO")
                        && edev.chip.kind == ChipKind::Xc4000Xv
                    {
                        obuf_diffs
                            .entry(wire_to)
                            .or_default()
                            .insert(wire_from, diff);
                    } else {
                        if diff.bits.is_empty() {
                            if wfname == "GND" {
                                continue;
                            }
                            if wfname.starts_with("OUT.BUFGE") && out_name.starts_with("BUFGE") {
                                continue;
                            }
                            panic!("weird lack of bits: {tile} {out_name} {wfname}");
                        }
                        mux_diffs
                            .entry(wire_to)
                            .or_default()
                            .insert(wire_from, diff);
                    }
                }
                continue;
            }
            if kind == ChipKind::SpartanXl {
                if out_name == "IMUX.CLB.C2" && matches!(&tile[..], "CLB.T" | "CLB.LT" | "CLB.RT") {
                    continue;
                }
                if out_name == "IMUX.CLB.C3" && matches!(&tile[..], "CLB.L" | "CLB.LB" | "CLB.LT") {
                    continue;
                }
            }
            let mut inps = vec![];
            let mut got_empty = false;
            for &wire_from in &mux.ins {
                let in_name = format!("{:#}.{}", wire_from.cell, intdb.wires.key(wire_from.wire));
                let mut diff = ctx.state.get_diff(tile, "INT", &mux_name, &in_name);
                if edev.chip.kind == ChipKind::Xc4000E
                    && tile.starts_with("IO.L")
                    && out_name == "IMUX.TBUF1.I"
                    && in_name == "0.DEC.V1"
                {
                    // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                    // found by diffing XC4000E with xact
                    assert!(!diff.bits.contains_key(&TileBit::new(0, 11, 1)));
                    diff.bits.insert(TileBit::new(0, 11, 1), false);
                }
                if diff.bits.is_empty() {
                    got_empty = true;
                }
                inps.push((in_name.to_string(), diff));
            }
            if tile.starts_with("CLB") && out_name == "IMUX.CLB.F4" {
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "CIN");
                inps.push(("CIN".to_string(), diff));
            }
            if (tile.starts_with("IO.B")
                || matches!(
                    &tile[..],
                    "CLB" | "CLB.L" | "CLB.R" | "CLB.B" | "CLB.LB" | "CLB.RB"
                ))
                && out_name == "IMUX.CLB.G2"
            {
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "COUT0");
                inps.push(("COUT0".to_string(), diff));
            }
            if (tile.starts_with("IO.R")
                || matches!(
                    &tile[..],
                    "CLB" | "CLB.B" | "CLB.T" | "CLB.R" | "CLB.RB" | "CLB.RT"
                ))
                && out_name == "IMUX.CLB.G3"
            {
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "CIN");
                inps.push(("CIN".to_string(), diff));
            }
            if out_name == "IMUX.IOB0.TS" || out_name == "IMUX.IOB1.TS" {
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "GND");
                inps.push(("GND".to_string(), diff));
                // ... I fucking can't with this fpga; look, let's just... not think about it
                got_empty = true;
            }
            if out_name == "IMUX.TBUF0.TS" || out_name == "IMUX.TBUF1.TS" {
                let diff = ctx.state.get_diff(tile, "INT", &mux_name, "GND");
                inps.push(("GND".to_string(), diff));

                let bel = if out_name == "IMUX.TBUF0.TS" {
                    "TBUF0"
                } else {
                    "TBUF1"
                };
                let drive1 = ctx.extract_bit_wide(tile, bel, "DRIVE1", "1");
                if drive1.bits.len() == 2 {
                    for (_, diff) in &mut inps {
                        diff.apply_bitvec_diff_int(&drive1, 0, 3);
                    }
                } else {
                    assert_eq!(drive1.bits.len(), 1);
                    for (_, diff) in &mut inps {
                        diff.apply_bit_diff(&drive1, false, true);
                    }
                }
                ctx.tiledb.insert(tile, bel, "DRIVE1", drive1);

                inps.push(("VCC".to_string(), Diff::default()));
                assert!(!got_empty);
                got_empty = true;
            }
            if out_name == "IMUX.TBUF0.I"
                || out_name == "IMUX.TBUF1.I"
                || ((out_name == "IMUX.IOB0.O1" || out_name == "IMUX.IOB1.O1")
                    && tile.starts_with("IO"))
            {
                assert!(!got_empty);
                inps.push(("GND".to_string(), Diff::default()));
                got_empty = true;
            }

            for (rtile, rwire, rbel, rattr) in [
                ("CNR.BL", "IMUX.IOB1.IK", "MD1", "ENABLE.T"),
                ("CNR.BL", "IMUX.IOB1.O1", "MD1", "ENABLE.O"),
                ("CNR.BL", "IMUX.RDBK.TRIG", "RDBK", "ENABLE"),
                ("CNR.BR", "IMUX.STARTUP.GTS", "STARTUP", "ENABLE.GTS"),
                ("CNR.BR", "IMUX.STARTUP.GSR", "STARTUP", "ENABLE.GSR"),
                ("CNR.TR", "IMUX.TDO.T", "TDO", "ENABLE.T"),
                ("CNR.TR", "IMUX.TDO.O", "TDO", "ENABLE.O"),
            ] {
                if tile == rtile && out_name == rwire {
                    let mut common = inps[0].1.clone();
                    for (_, diff) in &inps {
                        common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                    }
                    assert_eq!(common.bits.len(), 1);
                    for (_, diff) in &mut inps {
                        *diff = diff.combine(&!&common);
                        if diff.bits.is_empty() {
                            got_empty = true;
                        }
                    }
                    assert!(got_empty);
                    ctx.tiledb.insert(tile, rbel, rattr, xlat_bit(common));
                }
            }

            if edev.chip.kind == ChipKind::Xc4000E {
                let iob_mux_off_d = if tile.starts_with("IO.R") && out_name == "IMUX.CLB.G1" {
                    Some(("IO.R", "IO0"))
                } else if tile.starts_with("IO.R") && out_name == "IMUX.CLB.F1" {
                    Some(("IO.R", "IO1"))
                } else if tile.starts_with("IO.B") && out_name == "IMUX.CLB.F4" {
                    Some(("IO.B", "IO0"))
                } else if tile.starts_with("IO.B") && out_name == "IMUX.CLB.G4" {
                    Some(("IO.B", "IO1"))
                } else if tile.starts_with("CLB.L") && out_name == "IMUX.CLB.G3" {
                    Some(("IO.L", "IO0"))
                } else if tile.starts_with("CLB.L") && out_name == "IMUX.CLB.F3" {
                    Some(("IO.L", "IO1"))
                } else if matches!(&tile[..], "CLB.LT" | "CLB.T" | "CLB.RT")
                    && out_name == "IMUX.CLB.F2"
                {
                    Some(("IO.T", "IO0"))
                } else if matches!(&tile[..], "CLB.LT" | "CLB.T" | "CLB.RT")
                    && out_name == "IMUX.CLB.G2"
                {
                    Some(("IO.T", "IO1"))
                } else {
                    None
                };
                if let Some((filter, bel)) = iob_mux_off_d {
                    let mut common = inps[0].1.clone();
                    for (_, diff) in &inps {
                        common.bits.retain(|bit, _| diff.bits.contains_key(bit));
                    }
                    assert_eq!(common.bits.len(), 1);
                    for (_, diff) in &mut inps {
                        *diff = diff.combine(&!&common);
                        if diff.bits.is_empty() {
                            got_empty = true;
                        }
                    }
                    assert!(got_empty);
                    if tile.starts_with("CLB") {
                        let (mut bit, val) = common.bits.into_iter().next().unwrap();
                        assert_ne!(bit.tile, 0);
                        bit.tile = 0;
                        let common = Diff {
                            bits: [(bit, val)].into_iter().collect(),
                        };
                        for iotile in intdb.tile_classes.keys() {
                            if iotile.starts_with(filter) {
                                ctx.tiledb.insert(
                                    iotile,
                                    bel,
                                    "MUX.OFF_D",
                                    xlat_enum(vec![("CE", Diff::default()), ("O", common.clone())]),
                                );
                            }
                        }
                    } else {
                        assert!(tile.starts_with(filter));
                        ctx.tiledb.insert(
                            tile,
                            bel,
                            "MUX.OFF_D",
                            xlat_enum(vec![("CE", Diff::default()), ("O", common)]),
                        );
                    }
                }
            }

            if !got_empty {
                inps.push(("NONE".to_string(), Diff::default()));
            }
            let item = xlat_enum_ocd(inps, OcdMode::Mux);
            if kind == ChipKind::SpartanXl && out_name == "IMUX.BOT.COUT" {
                assert_eq!(mux.ins.len(), 1);
                assert!(item.bits.is_empty());
                continue;
            }
            if item.bits.is_empty() {
                println!("UMMM MUX {tile} {mux_name} is empty");
            }
            ctx.tiledb.insert(tile, "INT", mux_name, item);
        }

        for (wire_to, ins) in obuf_diffs {
            let out_name = edev.egrid.db.wires.key(wire_to.wire);
            let mut odiff = ins.iter().next().unwrap().1.clone();
            for diff in ins.values() {
                odiff.bits.retain(|bit, _| diff.bits.contains_key(bit));
            }
            for (wire_from, diff) in ins {
                let wfname = edev.egrid.db.wires.key(wire_from.wire);
                let in_name = format!("{:#}.{}", wire_from.cell, wfname);
                let diff = diff.combine(&!&odiff);
                ctx.tiledb
                    .insert(tile, "INT", format!("BUF.OBUF.{in_name}"), xlat_bit(diff));
            }
            ctx.tiledb.insert(
                tile,
                "INT",
                format!("BUF.{out_name}.0.OBUF"),
                xlat_bit(odiff),
            );
        }

        let mut handled = HashSet::new();
        for (&wire_to, ins) in &mux_diffs {
            let wtname = edev.egrid.db.wires.key(wire_to.wire);
            for (&wire_from, diff) in ins {
                if handled.contains(&(wire_to, wire_from)) {
                    continue;
                }
                let wfname = edev.egrid.db.wires.key(wire_from.wire);
                if let Some(oins) = mux_diffs.get(&wire_from) {
                    if let Some(odiff) = oins.get(&wire_to) {
                        if odiff == diff {
                            assert_eq!(diff.bits.len(), 1);
                            handled.insert((wire_to, wire_from));
                            handled.insert((wire_from, wire_to));
                            let diff = diff.clone();
                            let name = if tile.starts_with("LL") {
                                format!(
                                    "BIPASS.{:#}.{}.{:#}.{}",
                                    wire_to.cell, wtname, wire_from.cell, wfname
                                )
                            } else {
                                assert_eq!(wire_to.cell.to_idx(), 0);
                                assert_eq!(wire_from.cell.to_idx(), 0);
                                format!("BIPASS.{wtname}.{wfname}")
                            };
                            ctx.tiledb.insert(tile, "INT", name, xlat_bit(diff));
                            continue;
                        }
                    }
                }
                if diff.bits.len() != 1 {
                    continue;
                }
                let bit = *diff.bits.iter().next().unwrap().0;
                let mut unique = true;
                for (&owf, odiff) in ins {
                    if owf != wire_from && odiff.bits.contains_key(&bit) {
                        unique = false;
                    }
                }
                if !unique {
                    continue;
                }
                handled.insert((wire_to, wire_from));
                let diff = diff.clone();
                let oname = if tile.starts_with("LL") {
                    format!("{:#}.{}", wire_to.cell, wtname)
                } else {
                    wtname.to_string()
                };
                let iname = format!("{:#}.{}", wire_from.cell, wfname);
                if wtname.starts_with("SINGLE")
                    || wtname.starts_with("DOUBLE")
                    || wtname.starts_with("QUAD")
                    || wtname.starts_with("IO.DOUBLE")
                {
                    ctx.tiledb
                        .insert(tile, "INT", format!("PASS.{oname}.{iname}"), xlat_bit(diff));
                } else if wtname.starts_with("LONG")
                    || wtname.starts_with("OCTAL")
                    || wtname.starts_with("IO.OCTAL")
                {
                    ctx.tiledb
                        .insert(tile, "INT", format!("BUF.{oname}.{iname}"), xlat_bit(diff));
                } else {
                    println!("MEOW {tile} {oname} {iname}");
                }
            }
        }

        for (wire_to, ins) in mux_diffs {
            let out_name = edev.egrid.db.wires.key(wire_to.wire);
            let mux_name = if tile.starts_with("LL") {
                format!("MUX.{wtt:#}.{out_name}", wtt = wire_to.cell)
            } else {
                format!("MUX.{out_name}")
            };
            let mut in_diffs = vec![];
            let mut got_empty = false;
            for (wire_from, diff) in ins {
                if handled.contains(&(wire_to, wire_from)) {
                    continue;
                }
                let wfname = edev.egrid.db.wires.key(wire_from.wire);
                let in_name = format!("{:#}.{}", wire_from.cell, wfname);
                if diff.bits.is_empty() {
                    got_empty = true;
                }
                in_diffs.push((in_name, diff));
            }
            if in_diffs.is_empty() {
                continue;
            }
            if !got_empty {
                in_diffs.push(("NONE".to_string(), Diff::default()));
            }
            ctx.tiledb
                .insert(tile, "INT", mux_name, xlat_enum_ocd(in_diffs, OcdMode::Mux));
        }
        if tile.starts_with("IO.L") || tile.starts_with("IO.R") {
            for i in 0..2 {
                let bel = &format!("PULLUP_TBUF{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
            }
        }
        if edev.chip.kind != ChipKind::Xc4000E
            && matches!(
                &tile[..],
                "LLHC.CLB" | "LLHC.CLB.B" | "LLH.CLB" | "LLH.CLB.B"
            )
        {
            for bel in ["TBUF_SPLITTER0", "TBUF_SPLITTER1"] {
                let item = ctx.extract_bit(tile, bel, "BUF", "W");
                ctx.tiledb.insert(tile, bel, "PASS", item);
                let item = ctx.extract_bit(tile, bel, "BUF", "E");
                ctx.tiledb.insert(tile, bel, "PASS", item);
                let item = ctx.extract_bit(tile, bel, "BUF", "W.BUF");
                ctx.tiledb.insert(tile, bel, "BUF_W", item);
                let item = ctx.extract_bit(tile, bel, "BUF", "E.BUF");
                ctx.tiledb.insert(tile, bel, "BUF_E", item);
            }
        }
        if matches!(
            &tile[..],
            "LLHC.CLB" | "LLHC.CLB.B" | "LLHQ.CLB" | "LLHQ.CLB.B" | "LLHQ.CLB.T"
        ) {
            for we in ['W', 'E'] {
                for i in 0..2 {
                    let bel = &format!("PULLUP_TBUF{i}_{we}");
                    ctx.collect_bit(tile, bel, "ENABLE", "1");
                }
            }
        }
        if tile.starts_with("LLV.") {
            let bel = "CLKH";
            if edev.chip.kind == ChipKind::SpartanXl {
                for ipin in [
                    "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H", "I.UR.V",
                ] {
                    let (_, _, diff) = Diff::split(
                        ctx.state.peek_diff(tile, bel, "MUX.O0", ipin).clone(),
                        ctx.state.peek_diff(tile, bel, "MUX.O1", ipin).clone(),
                    );
                    ctx.tiledb
                        .insert(tile, bel, format!("ENABLE.{ipin}"), xlat_bit(diff));
                }
                for opin in ["O0", "O1", "O2", "O3"] {
                    let mut diffs = vec![("NONE", Diff::default())];
                    for ipin in [
                        "I.LL.H", "I.LL.V", "I.UL.H", "I.UL.V", "I.LR.H", "I.LR.V", "I.UR.H",
                        "I.UR.V",
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, format!("MUX.{opin}"), ipin);
                        diff.apply_bit_diff(
                            ctx.tiledb.item(tile, bel, &format!("ENABLE.{ipin}")),
                            true,
                            false,
                        );
                        diffs.push((ipin, diff));
                    }
                    ctx.tiledb
                        .insert(tile, bel, format!("MUX.{opin}"), xlat_enum(diffs));
                }
            } else {
                for (opin, ipin_p) in [
                    ("O0", "I.UL.V"),
                    ("O1", "I.LL.H"),
                    ("O2", "I.LR.V"),
                    ("O3", "I.UR.H"),
                ] {
                    ctx.collect_enum_default(
                        tile,
                        bel,
                        &format!("MUX.{opin}"),
                        &[ipin_p, "I.LL.V", "I.UL.H", "I.LR.H", "I.UR.V"],
                        "NONE",
                    );
                }
            }
        }
        if tile.starts_with("CNR") {
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                for hv in ['H', 'V'] {
                    for attr in ["CLK_EN", "ALT_PAD"] {
                        let item = ctx.extract_bit(tile, &format!("BUFGLS_{hv}"), attr, "1");
                        let bel = if edev.chip.kind == ChipKind::SpartanXl {
                            format!("BUFGLS.{hv}")
                        } else {
                            format!("BUFG.{hv}")
                        };
                        ctx.tiledb.insert(tile, bel, attr, item);
                    }
                }
            }
            if edev.chip.kind != ChipKind::SpartanXl {
                for hv in ['H', 'V'] {
                    for i in 0..4 {
                        let bel = &format!("PULLUP_DEC{i}_{hv}");
                        ctx.collect_bit(tile, bel, "ENABLE", "1");
                    }
                }
            }
        }
        if edev.chip.kind != ChipKind::SpartanXl {
            if matches!(&tile[..], "LLVC.IO.L" | "LLVC.IO.R") {
                for sn in ['S', 'N'] {
                    for i in 0..4 {
                        let bel = &format!("PULLUP_DEC{i}_{sn}");
                        ctx.collect_bit(tile, bel, "ENABLE", "1");
                    }
                }
            }
            if matches!(&tile[..], "LLHC.IO.B" | "LLHC.IO.T") {
                for we in ['W', 'E'] {
                    for i in 0..4 {
                        let bel = &format!("PULLUP_DEC{i}_{we}");
                        ctx.collect_bit(tile, bel, "ENABLE", "1");
                    }
                }
            }
            if tile.starts_with("IO") {
                for i in 0..3 {
                    let bel = &format!("DEC{i}");
                    for j in 1..=4 {
                        let item = ctx.extract_bit(tile, bel, &format!("O{j}MUX"), "I");
                        ctx.tiledb.insert(tile, bel, format!("O{j}_P"), item);
                        let item = ctx.extract_bit(tile, bel, &format!("O{j}MUX"), "NOT");
                        ctx.tiledb.insert(tile, bel, format!("O{j}_N"), item);
                    }
                }
            }
        }
    }
}
