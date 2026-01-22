use std::collections::{BTreeMap, btree_map};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelSlotId, CellSlotId, SwitchBoxItem, TileWireCoord},
    dir::{DirH, DirV},
    grid::{TileCoord, WireCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, DiffKey, FuzzerProp, OcdMode, xlat_bit_raw, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{BitRectId, TileBit};
use prjcombine_xc2000::{
    chip::ChipKind,
    xc4000::{bslots, enums, tslots, wires, xc4000::bcls, xc4000::tcls},
};

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, WireIntDistinct, WireIntDstFilter, WireIntSrcFilter, resolve_int_pip},
        props::{
            DynProp,
            bel::{BaseBelAttr, BaseBelMode, BaseBelPin, BelMutex, FuzzBelAttr, FuzzBelMode},
            mutex::{IntMutex, WireMutexExclusive},
            pip::{BasePip, PinFar, PipWire},
            relation::{DeltaSlot, NoopRelation, Related},
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
    let wname = backend.edev.db.wires.key(wire_target.slot);
    let aname = backend.edev.db.wires.key(wire_avoid.slot);
    let mut cell = wire_target.cell;
    let mut wt = wire_target.slot;
    let fuzzer = fuzzer.fuzz(Key::WireMutex(wire_target), None, "EXCLUSIVE");
    // println!("DRIVING {wire_target:?} {wname}");
    if cell.row != edev.chip.row_s()
        && cell.row != edev.chip.row_n()
        && (wire_target.slot == wires::LONG_H[2] || wire_target.slot == wires::LONG_H[3])
    {
        let bel = if wire_target.slot == wires::LONG_H[3] {
            bslots::TBUF[1]
        } else {
            bslots::TBUF[0]
        };
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let bel_naming = backend.ngrid.get_bel_naming(tcrd.bel(bel));
        let pin_naming = &bel_naming.pins["O"];
        let site_name = backend.ngrid.get_bel_name(tcrd.bel(bel)).unwrap();
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "TBUF")
            .base(Key::SitePin(site_name, "O".into()), true)
            .base(
                Key::Pip(
                    &ntile.names[pin_naming.tile],
                    &pin_naming.name,
                    &pin_naming.name_far,
                ),
                Value::FromPin(site_name, "O".into()),
            );
        (fuzzer, site_name, "O")
    } else if wire_target.slot == wires::TIE_0 {
        let tcrd = cell.tile(tslots::MAIN);
        let ntile = &backend.ngrid.tiles[&tcrd];
        let site_name = ntile.tie_name.as_ref().unwrap();
        let fuzzer = fuzzer
            .base(Key::SiteMode(site_name), "TIE")
            .base(Key::SitePin(site_name, "O".into()), true);
        (fuzzer, site_name, "O")
    } else if wname.starts_with("OUT_CLB") && (wname.ends_with("_V") || wname.ends_with("_H")) {
        let owname = &wname[..(wname.len() - 2)];
        let nwt = cell.wire(backend.edev.db.get_wire(owname));
        let (fuzzer, site_name, pin_name) =
            drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
        let (tile, wt, wf) = resolve_int_pip(
            backend,
            cell.tile(tslots::MAIN),
            TileWireCoord::new_idx(0, wt),
            TileWireCoord::new_idx(0, nwt.slot),
        )
        .unwrap();
        let fuzzer = fuzzer.base(
            Key::Pip(tile, wf, wt),
            Value::FromPin(site_name, pin_name.into()),
        );
        (fuzzer, site_name, pin_name)
    } else if wname.starts_with("OUT_CLB") {
        let site_name = backend.ngrid.get_bel_name(cell.bel(bslots::CLB)).unwrap();
        let (pin, fuzzer) = match wire_target.slot {
            wires::OUT_CLB_X => (
                "X",
                fuzzer
                    .base(Key::SiteAttr(site_name, "F".into()), "#LUT:F=0x0000")
                    .base(Key::SiteAttr(site_name, "XMUX".into()), "F"),
            ),
            wires::OUT_CLB_Y => (
                "Y",
                fuzzer
                    .base(Key::SiteAttr(site_name, "G".into()), "#LUT:G=0x0000")
                    .base(Key::SiteAttr(site_name, "YMUX".into()), "G"),
            ),
            wires::OUT_CLB_XQ => (
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
            wires::OUT_CLB_YQ => (
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
    } else if let Some(idx) = wires::SINGLE_H.index_of(wire_target.slot) {
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
                TileWireCoord::new_idx(0, wires::SINGLE_H_E[idx]),
                TileWireCoord::new_idx(0, wt),
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
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, wires::SINGLE_H_E[idx]),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.row == edev.chip.row_s() {
            let nwt = cell.delta(0, 1).wire(wires::SINGLE_V[idx]);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, wires::SINGLE_V_S[idx]),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.row == edev.chip.row_n() - 1 {
            let nwt = cell.wire(wires::SINGLE_V[idx]);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, nwt.slot),
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
                (0 | 4, true) => (wires::OUT_CLB_Y, wires::OUT_CLB_Y, 0),
                (1 | 5, true) => (wires::OUT_CLB_YQ, wires::OUT_CLB_YQ, 0),
                (2 | 6, true) => (wires::OUT_CLB_XQ_S, wires::OUT_CLB_XQ, 1),
                (3 | 7, true) => (wires::OUT_CLB_X_S, wires::OUT_CLB_X, 1),
                (0 | 4, false) => (wires::OUT_CLB_Y_V, wires::OUT_CLB_Y_V, 0),
                (1 | 5, false) => (wires::OUT_CLB_YQ_V, wires::OUT_CLB_YQ_V, 0),
                (2 | 6, false) => (wires::OUT_CLB_XQ_S, wires::OUT_CLB_XQ_V, 1),
                (3 | 7, false) => (wires::OUT_CLB_X_S, wires::OUT_CLB_X_V, 1),
                _ => unreachable!(),
            };
            let nwt = cell.delta(0, dy).wire(sout);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, out),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        }
    } else if let Some(idx) = wires::SINGLE_V.index_of(wire_target.slot) {
        assert_ne!(cell.col, edev.chip.col_w());
        if cell.row == edev.chip.row_s() {
            let nwt = cell.delta(0, 1).wire(wt);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, wires::SINGLE_V_S[idx]),
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
                TileWireCoord::new_idx(0, wires::SINGLE_V_S[idx]),
                TileWireCoord::new_idx(0, wt),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.col == edev.chip.col_w() + 1 {
            let nwt = cell.wire(wires::SINGLE_H[idx]);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, nwt.slot),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        } else if cell.col == edev.chip.col_e() {
            let nwt = cell.delta(-1, 0).wire(wires::SINGLE_H[idx]);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, wires::SINGLE_H_E[idx]),
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
                (0 | 4, true) => (wires::OUT_CLB_XQ, wires::OUT_CLB_XQ, 0),
                (1 | 5, true) => (wires::OUT_CLB_X, wires::OUT_CLB_X, 0),
                (2 | 6, true) => (wires::OUT_CLB_Y_E, wires::OUT_CLB_Y, -1),
                (3 | 7, true) => (wires::OUT_CLB_YQ_E, wires::OUT_CLB_YQ, -1),
                (0 | 4, false) => (wires::OUT_CLB_XQ_H, wires::OUT_CLB_XQ_H, 0),
                (1 | 5, false) => (wires::OUT_CLB_X_H, wires::OUT_CLB_X_H, 0),
                (2 | 6, false) => (wires::OUT_CLB_Y_E, wires::OUT_CLB_Y_H, -1),
                (3 | 7, false) => (wires::OUT_CLB_YQ_E, wires::OUT_CLB_YQ_H, -1),
                _ => unreachable!(),
            };
            let nwt = cell.delta(dx, 0).wire(sout);
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(
                backend,
                cell.tile(tslots::MAIN),
                TileWireCoord::new_idx(0, wt),
                TileWireCoord::new_idx(0, out),
            )
            .unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            (fuzzer, site_name, pin_name)
        }
    } else if wname.starts_with("LONG")
        || wname.starts_with("QUAD")
        || wname.starts_with("OCTAL")
        || wname.starts_with("GCLK")
        || wname.starts_with("VCLK")
    {
        let mut filter = None;
        let mut twt = CellSlotId::from_idx(0);
        let mut tslot = tslots::MAIN;
        if wname.starts_with("LONG") {
            if wname.contains("_H") {
                if cell.col == edev.chip.col_w() {
                    cell.col += 1;
                }
                if cell.col == edev.chip.col_e() {
                    cell.col -= 2;
                }
                if cell.col == wire_avoid.cell.col {
                    if (edev.chip.kind.is_xl()
                        && (cell.col == edev.chip.col_q(DirH::W) - 1
                            || cell.col == edev.chip.col_q(DirH::E) - 1))
                        || cell.col == edev.chip.col_mid() - 1
                    {
                        cell.col -= 1;
                    } else {
                        cell.col += 1;
                    }
                }
            } else if wname.contains("_V") {
                if cell.row == edev.chip.row_n() {
                    cell.row -= 2;
                }
                if cell.row == wire_avoid.cell.row {
                    if (edev.chip.kind.is_xl()
                        && (cell.row == edev.chip.row_q(DirV::S) - 1
                            || cell.row == edev.chip.row_q(DirV::N) - 1))
                        || cell.row == edev.chip.row_mid() - 1
                    {
                        cell.row -= 1;
                    } else {
                        cell.row += 1;
                    }
                }
            } else {
                unreachable!()
            }
        } else if wname.starts_with("OCTAL_IO") {
            if wire_target.slot == wires::OCTAL_IO_W[0] {
                // ok
            } else if wire_target.slot == wires::OCTAL_IO_E[0] {
                assert_ne!(cell.row, edev.chip.row_n());
                cell.row += 1;
                wt = wires::OCTAL_IO_E[1];
                if cell.row == edev.chip.row_n() {
                    wt = wires::OCTAL_IO_N[1];
                    cell.col -= 1;
                }
            } else if wire_target.slot == wires::OCTAL_IO_S[0] {
                // ok
            } else if wire_target.slot == wires::OCTAL_IO_N[0] {
                assert_ne!(cell.col, edev.chip.col_w());
                cell.col -= 1;
                wt = wires::OCTAL_IO_N[1];
                if cell.col == edev.chip.col_w() {
                    wt = wires::OCTAL_IO_W[1];
                    cell.row -= 1;
                }
            } else {
                unreachable!()
            }
        } else if wname.starts_with("QUAD_H") {
            if cell.col == edev.chip.col_w() {
                if let Some(idx) = wires::QUAD_H3.index_of(wire_target.slot) {
                    if aname.starts_with("LONG_IO") {
                        cell.col += 1;
                        match idx {
                            0 => {
                                filter = Some("QUAD_H0[0]");
                                wt = wires::QUAD_H4[0];
                            }
                            1 => {
                                filter = Some("QUAD_H0[1]");
                                wt = wires::QUAD_H4[1];
                            }
                            2 => {
                                filter = Some("QUAD_H0[2]");
                                wt = wires::QUAD_H4[2];
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        filter = Some("LONG_IO");
                    }
                } else if wire_target.slot == wires::QUAD_H0[1] {
                    cell.col += 1;
                    wt = wires::QUAD_H1[1];
                }
            } else if wire_target.slot == wires::QUAD_H0[2] {
                if cell.col == edev.chip.col_e() {
                    if aname.starts_with("LONG_IO") {
                        filter = Some("QUAD_H4[2]");
                    } else {
                        filter = Some("LONG_IO");
                    }
                } else {
                    cell.col += 1;
                    wt = wires::QUAD_H1[2];
                }
            }
        } else if wname.starts_with("QUAD_V") {
            if cell.row == edev.chip.row_n() {
                if let Some(idx) = wires::QUAD_V3.index_of(wire_target.slot) {
                    if aname.starts_with("LONG_IO") {
                        cell.row -= 1;
                        match idx {
                            0 => {
                                filter = Some("QUAD_V0[0]");
                                wt = wires::QUAD_V4[0];
                            }
                            1 => {
                                filter = Some("QUAD_V0[1]");
                                wt = wires::QUAD_V4[1];
                            }
                            2 => {
                                filter = Some("QUAD_V0[2]");
                                wt = wires::QUAD_V4[2];
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        filter = Some("LONG_IO");
                    }
                } else if wire_target.slot == wires::QUAD_V2[2] {
                    cell.row -= 1;
                    wt = wires::QUAD_V3[2];
                }
            } else if wire_target.slot == wires::QUAD_V0[0] {
                if cell.row == edev.chip.row_s() {
                    if aname.starts_with("LONG_IO") {
                        filter = Some("QUAD_V4[0]");
                    } else {
                        filter = Some("LONG_IO");
                    }
                } else {
                    cell.row -= 1;
                    wt = wires::QUAD_V1[0];
                }
            }
        } else if let Some(idx) = wires::OCTAL_H.index_of(wire_target.slot) {
            if cell.col == edev.chip.col_w() {
                cell.col += 7 - idx;
                wt = wires::OCTAL_H[7];
            }
        } else if let Some(idx) = wires::OCTAL_V.index_of(wire_target.slot) {
            if cell.row == edev.chip.row_n() {
                cell.row -= 7 - idx;
                wt = wires::OCTAL_V[7];
            }
        } else if wires::GCLK.contains(wire_target.slot) {
            if edev.chip.kind.is_xl() {
                if cell.row == edev.chip.row_s() {
                    cell.row = edev.chip.row_q(DirV::S);
                } else {
                    cell.row = edev.chip.row_q(DirV::N);
                }
            } else {
                cell.row = edev.chip.row_mid();
            }
            tslot = tslots::LLV;
        } else if wire_target.slot == wires::VCLK {
            if cell.row == edev.chip.row_s() {
                // OK
            } else if cell.row == edev.chip.row_q(DirV::S) {
                cell.row = edev.chip.row_mid();
                tslot = tslots::LLV;
            } else if cell.row == edev.chip.row_mid() {
                twt = CellSlotId::from_idx(1);
                tslot = tslots::LLV;
            } else if cell.row == edev.chip.row_q(DirV::N) {
                cell.row = edev.chip.row_n();
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
        let tcrd = cell.tile(tslot);
        let tile = &backend.edev[tcrd];
        let mwt = TileWireCoord {
            cell: twt,
            wire: wt,
        };
        let res = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, mwt))
            .unwrap();
        assert_eq!(res, wire_target);
        let mut ins = backend.edev.db_index[tile.class].pips_bwd[&mwt].clone();
        if backend.edev.db[tile.class].bels.contains_id(bslots::IO[0])
            && ((wire_target.slot == wires::OCTAL_V[7] && cell.row == edev.chip.row_n())
                || (wire_target.slot == wires::OCTAL_H[7] && cell.col == edev.chip.col_w()))
        {
            ins.insert(TileWireCoord::new_idx(0, wires::TIE_0).pos());
        }
        for mwf in ins {
            let wfname = backend.edev.db.wires.key(mwf.wire);
            if let Some(filter) = filter {
                if !wfname.starts_with(filter) {
                    continue;
                }
            } else {
                if !(wfname.starts_with("SINGLE")
                    || mwf.wire == wires::TIE_0
                    || (wfname.starts_with("DOUBLE_IO")
                        && ((wname.starts_with("OCTAL") && !wname.starts_with("OCTAL_IO"))
                            || wname.starts_with("QUAD")
                            || mwf.wire == wires::VCLK)))
                {
                    continue;
                }
            }
            let nwt = backend
                .edev
                .resolve_wire(backend.edev.tile_wire(tcrd, mwf.tw))
                .unwrap();
            if nwt == wire_avoid {
                continue;
            }
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, Some((tcrd, mwf.tw)), wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(backend, tcrd, mwt, mwf.tw).unwrap();
            let fuzzer = fuzzer.base(
                Key::Pip(tile, wf, wt),
                Value::FromPin(site_name, pin_name.into()),
            );
            return (fuzzer, site_name, pin_name);
        }
        panic!(
            "umm failed at {wire_target} {wname}",
            wire_target = wire_target.to_string(backend.edev.db)
        );
    } else if wname.starts_with("DOUBLE_IO") {
        let (tcrd, mwt) = orig_target.unwrap();
        let tile = &backend.edev[tcrd];
        let ins = &backend.edev.db_index[tile.class].pips_bwd[&mwt];
        for &mwf in ins {
            let wfname = backend.edev.db.wires.key(mwf.wire);
            if !wfname.starts_with("SINGLE") {
                continue;
            }
            let nwt = backend
                .edev
                .resolve_wire(backend.edev.tile_wire(tcrd, mwf.tw))
                .unwrap();
            let (fuzzer, site_name, pin_name) =
                drive_xc4000_wire(backend, fuzzer, nwt, None, wire_avoid);
            let (tile, wt, wf) = resolve_int_pip(backend, tcrd, mwt, mwf.tw).unwrap();
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
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_from))
            .unwrap();
        let res_mid = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_mid))
            .unwrap();
        let res_to = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_to))
            .unwrap();
        let fuzzer = fuzzer
            .fuzz(Key::WireMutex(res_to), None, "EXCLUSIVE-TGT")
            .fuzz(Key::WireMutex(res_mid), None, "EXCLUSIVE-MID");
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
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_from))
            .unwrap();
        let res_to = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_to))
            .unwrap();
        let fuzzer = fuzzer.fuzz(Key::WireMutex(res_to), None, "EXCLUSIVE-TGT");
        let (fuzzer, src_site, src_pin) = drive_xc4000_wire(
            backend,
            fuzzer,
            res_from,
            Some((tcrd, self.wire_from)),
            res_to,
        );
        let ntile = &backend.ngrid.tiles[&tcrd];
        let ntcls = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let (tile, wt, wf) = resolve_int_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        if let Some(wn) = ntcls.wires.get(&self.wire_to)
            && wn.alt_pips_to.contains(&self.wire_from)
        {
            let alt = wn.alt_name.as_ref().unwrap();
            Some((
                fuzzer
                    .fuzz(
                        Key::Pip(tile, alt, wt),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    )
                    .fuzz(
                        Key::Pip(tile, wf, alt),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    ),
                false,
            ))
        } else if let Some(wn) = ntcls.wires.get(&self.wire_from)
            && wn.alt_pips_from.contains(&self.wire_to)
        {
            let alt = wn.alt_name.as_ref().unwrap();
            Some((
                fuzzer
                    .fuzz(
                        Key::Pip(tile, alt, wt),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    )
                    .fuzz(
                        Key::Pip(tile, wf, alt),
                        None,
                        Value::FromPin(src_site, src_pin.into()),
                    ),
                false,
            ))
        } else {
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
}

#[derive(Clone, Debug)]
struct Xc4000SimplePip {
    wire_to: TileWireCoord,
    wire_from: TileWireCoord,
}

impl Xc4000SimplePip {
    fn new(wire_to: TileWireCoord, wire_from: TileWireCoord) -> Self {
        Self { wire_to, wire_from }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Xc4000SimplePip {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_from))?;
        backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, self.wire_to))?;
        let ntile = &backend.ngrid.tiles[&tcrd];
        let ntcls = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let (tile, wt, wf) = resolve_int_pip(backend, tcrd, self.wire_to, self.wire_from)?;
        if let Some(wn) = ntcls.wires.get(&self.wire_to)
            && wn.alt_pips_to.contains(&self.wire_from)
        {
            let alt = wn.alt_name.as_ref().unwrap();
            Some((
                fuzzer.fuzz(Key::Pip(tile, alt, wt), None, true).fuzz(
                    Key::Pip(tile, wf, alt),
                    None,
                    true,
                ),
                false,
            ))
        } else if let Some(wn) = ntcls.wires.get(&self.wire_from)
            && wn.alt_pips_from.contains(&self.wire_to)
        {
            let alt = wn.alt_name.as_ref().unwrap();
            Some((
                fuzzer.fuzz(Key::Pip(tile, alt, wt), None, true).fuzz(
                    Key::Pip(tile, wf, alt),
                    None,
                    true,
                ),
                false,
            ))
        } else {
            Some((fuzzer.fuzz(Key::Pip(tile, wf, wt), None, true), false))
        }
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
        let tile = &backend.edev[tcrd];
        let ntile = &backend.ngrid.tiles[&tcrd];
        let tcls = &backend.edev.db[tile.class];
        let bel_data = &tcls.bels[self.slot];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let bcrd = tcrd.bel(self.slot);
        let bel_naming = backend.ngrid.get_bel_naming(bcrd);

        let (pid_from, pid_to, pin_from, pin_to, ex_from, ex_to) = match self.dir {
            DirH::E => (
                bcls::TBUF_SPLITTER::W,
                bcls::TBUF_SPLITTER::E,
                &bel_naming.pins["W"].name,
                &bel_naming.pins["E"].name,
                &bel_naming.pins["W_EXCL"].name,
                &bel_naming.pins["E_EXCL"].name,
            ),
            DirH::W => (
                bcls::TBUF_SPLITTER::E,
                bcls::TBUF_SPLITTER::W,
                &bel_naming.pins["E"].name,
                &bel_naming.pins["W"].name,
                &bel_naming.pins["E_EXCL"].name,
                &bel_naming.pins["W_EXCL"].name,
            ),
        };
        let res_from = backend
            .edev
            .resolve_wire(backend.edev.get_bel_bidir(bcrd, pid_from))
            .unwrap();
        let res_to = backend
            .edev
            .resolve_wire(backend.edev.get_bel_bidir(bcrd, pid_to))
            .unwrap();
        let wire_from = bel_data.bidirs[pid_from];
        let fuzzer = fuzzer.fuzz(Key::WireMutex(res_to), None, "EXCLUSIVE-TGT");
        let (fuzzer, src_site, src_pin) =
            drive_xc4000_wire(backend, fuzzer, res_from, Some((tcrd, wire_from)), res_to);
        let tile = &ntile.names[bel_naming.tiles[0]];
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
    let intdb = backend.edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        let tcls_index = &backend.edev.db_index[tcid];
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcname) else {
            continue;
        };
        for (&wire_to, ins) in &tcls_index.pips_bwd {
            let out_name = intdb.wires.key(wire_to.wire);
            if wires::BUFGLS_H.contains(wire_to.wire) && kind == ChipKind::SpartanXl {
                continue;
            }
            if wires::GCLK.contains(wire_to.wire) && kind == ChipKind::SpartanXl {
                for &wire_from in ins {
                    let idx = wires::BUFGLS_H.index_of(wire_from.wire).unwrap();
                    let wire_mid = wire_from;
                    let wire_from = TileWireCoord {
                        wire: wires::BUFGLS[idx],
                        ..wire_from.tw
                    };
                    ctx.build()
                        .prop(IntMutex::new("MAIN".to_string()))
                        .test_raw(DiffKey::RoutingVia(
                            tcid,
                            wire_to,
                            wire_mid.pos(),
                            wire_from.pos(),
                        ))
                        .prop(WireMutexExclusive::new(wire_to))
                        .prop(WireMutexExclusive::new(wire_mid.tw))
                        .prop(Xc4000SimplePip::new(wire_to, wire_from))
                        .commit();
                }
                continue;
            }
            if wires::QBUF.contains(wire_to.wire) {
                let wire_mid = wire_to;
                for &wire_to in ins {
                    for &wire_from in ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        ctx.build()
                            .prop(IntMutex::new("MAIN".to_string()))
                            .test_raw(DiffKey::RoutingVia(
                                tcid,
                                wire_to.tw,
                                wire_mid.pos(),
                                wire_from,
                            ))
                            .prop(Xc4000DoublePip::new(wire_to.tw, wire_mid, wire_from.tw))
                            .commit();
                    }
                }
                continue;
            }
            if wire_to.wire == wires::OBUF {
                let wire_mid = wire_to;
                for &wire_to in &tcls_index.pips_fwd[&wire_mid] {
                    for &wire_from in ins {
                        if wire_to == wire_from {
                            continue;
                        }
                        ctx.build()
                            .prop(IntMutex::new("MAIN".to_string()))
                            .test_raw(DiffKey::RoutingVia(
                                tcid,
                                wire_to.tw,
                                wire_mid.pos(),
                                wire_from,
                            ))
                            .prop(Xc4000BiPip::new(wire_to.tw, wire_from.tw))
                            .commit();
                    }
                }
                continue;
            }
            for &wire_from in ins {
                let wire_from = wire_from.tw;
                let wire_from_name = intdb.wires.key(wire_from.wire);

                let mut is_bidi = false;
                if let Some(mux) = tcls_index.pips_bwd.get(&wire_from)
                    && mux.contains(&wire_to.pos())
                {
                    is_bidi = true;
                }
                let tbuf_i_wire = if wire_from.wire == wires::LONG_H[2] {
                    Some(wires::IMUX_TBUF_I[0])
                } else if wire_from.wire == wires::LONG_H[3] {
                    Some(wires::IMUX_TBUF_I[1])
                } else {
                    None
                };
                if let Some(tbuf_i_wire) = tbuf_i_wire
                    && let Some(mux) = tcls_index
                        .pips_bwd
                        .get(&TileWireCoord::new_idx(0, tbuf_i_wire))
                    && mux.contains(&wire_to.pos())
                {
                    is_bidi = true;
                }

                let mut is_bipass = false;
                let is_wt_sd = out_name.starts_with("SINGLE")
                    || out_name.starts_with("DOUBLE")
                    || out_name.starts_with("QUAD");
                let is_wf_sd = wire_from_name.starts_with("SINGLE")
                    || wire_from_name.starts_with("DOUBLE")
                    || wire_from_name.starts_with("QUAD");
                if is_wt_sd && is_wf_sd {
                    is_bipass = true;
                }
                if out_name.starts_with("OCTAL_IO") && wire_from_name.starts_with("SINGLE") {
                    is_bipass = true;
                }
                if out_name.starts_with("SINGLE") && wire_from_name.starts_with("OCTAL_IO") {
                    is_bipass = true;
                }
                if out_name.starts_with("DEC") && wire_from_name.starts_with("DEC") {
                    is_bipass = true;
                }

                if wire_from.wire == wires::OBUF {
                    continue;
                }
                if wires::QBUF.contains(wire_from.wire) {
                    continue;
                }
                if wire_to.wire == wires::IMUX_CLB_F4 && wire_from.wire == wires::SPECIAL_CLB_CIN {
                    ctx.build()
                        .prop(BaseBelMode::new(bslots::CLB, "CLB".into()))
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(FuzzBelAttr::new(
                            bslots::CLB,
                            "F4MUX".into(),
                            "".into(),
                            "CIN".into(),
                        ))
                        .prop(WireMutexExclusive::new(wire_to))
                        .commit();
                    continue;
                }
                if wire_to.wire == wires::IMUX_CLB_G3 && wire_from.wire == wires::SPECIAL_CLB_CIN {
                    ctx.build()
                        .prop(Related::new(
                            DeltaSlot::new(-1, 0, tslots::MAIN),
                            BaseBelMode::new(bslots::CLB, "CLB".into()),
                        ))
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(Related::new(
                            DeltaSlot::new(-1, 0, tslots::MAIN),
                            FuzzBelAttr::new(bslots::CLB, "G3MUX".into(), "".into(), "CIN".into()),
                        ))
                        .prop(WireMutexExclusive::new(wire_to))
                        .commit();
                    continue;
                }
                if wire_to.wire == wires::IMUX_CLB_G2 && wire_from.wire == wires::SPECIAL_CLB_COUT0
                {
                    ctx.build()
                        .prop(Related::new(
                            DeltaSlot::new(0, 1, tslots::MAIN),
                            BaseBelMode::new(bslots::CLB, "CLB".into()),
                        ))
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(Related::new(
                            DeltaSlot::new(0, 1, tslots::MAIN),
                            FuzzBelAttr::new(
                                bslots::CLB,
                                "G2MUX".into(),
                                "".into(),
                                "COUT0".into(),
                            ),
                        ))
                        .prop(WireMutexExclusive::new(wire_to))
                        .commit();
                    continue;
                }
                if wires::IMUX_TBUF_I.contains(wire_to.wire) && wire_from.wire == wires::TIE_0 {
                    continue;
                }
                if let Some(idx) = wires::IMUX_TBUF_T.index_of(wire_to.wire)
                    && wire_from.wire == wires::TIE_0
                {
                    let bel = bslots::TBUF[idx];
                    ctx.build()
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(FuzzBelMode::new(bel, "".into(), "TBUF".into()))
                        .prop(FuzzBelAttr::new(
                            bel,
                            "TBUFATTR".into(),
                            "".into(),
                            "WAND".into(),
                        ))
                        .prop(WireMutexExclusive::new(wire_to))
                        .commit();
                    continue;
                }
                if wires::IMUX_TBUF_T.contains(wire_to.wire) && wire_from.wire == wires::TIE_1 {
                    continue;
                }
                if let Some(idx) = wires::IMUX_IO_T.index_of(wire_to.wire)
                    && wire_from.wire == wires::TIE_0
                {
                    let bel = bslots::IO[idx];
                    ctx.build()
                        .prop(BaseBelMode::new(bel, "IOB".into()))
                        .prop(BaseBelAttr::new(bel, "OUTMUX".into(), "O".into()))
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(FuzzBelAttr::new(bel, "TRI".into(), "T".into(), "".into()))
                        .prop(WireMutexExclusive::new(wire_to))
                        .commit();
                    continue;
                }
                if wires::IMUX_IO_O1.contains(wire_to.wire) && wire_from.wire == wires::TIE_0 {
                    continue;
                }

                if is_bidi && !is_bipass {
                    ctx.build()
                        .prop(IntMutex::new("MAIN".to_string()))
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(Xc4000BiPip::new(wire_to, wire_from))
                        .commit();
                } else {
                    let mut builder = ctx
                        .build()
                        .prop(WireIntDistinct::new(wire_to, wire_from))
                        .prop(WireIntDstFilter::new(wire_to))
                        .prop(WireIntSrcFilter::new(wire_from))
                        .prop(IntMutex::new("MAIN".to_string()))
                        .test_raw(DiffKey::Routing(tcid, wire_to, wire_from.pos()))
                        .prop(WireMutexExclusive::new(wire_to))
                        .prop(WireMutexExclusive::new(wire_from))
                        .prop(Xc4000SimplePip::new(wire_to, wire_from));
                    if tcid == tcls::CNR_NE
                        && (wire_from.wire == wires::OUT_IO_WE_I1[1]
                            || wire_from.wire == wires::OUT_IO_WE_I2[1]
                            || wire_from.wire == wires::OUT_OSC_MUX1)
                    {
                        // sigh.
                        builder = builder
                            .prop(BelMutex::new(bslots::OSC, "MODE".into(), "INT".into()))
                            .prop(BasePip::new(
                                NoopRelation,
                                PipWire::BelPinNear(bslots::OSC, "OUT0".into()),
                                PipWire::BelPinNear(bslots::OSC, "F15".into()),
                            ));
                    }
                    if tcid == tcls::IO_E0_N
                        && matches!(
                            wire_from.wire,
                            wires::OUT_IO_WE_I1_S1 | wires::OUT_IO_WE_I2_S1
                        )
                    {
                        // sigh.
                        let bel = bslots::OSC;
                        builder = builder
                            .prop(Related::new(
                                DeltaSlot::new(0, 1, tslots::MAIN),
                                BelMutex::new(bel, "MODE".into(), "INT".into()),
                            ))
                            .prop(BasePip::new(
                                DeltaSlot::new(0, 1, tslots::MAIN),
                                PipWire::BelPinNear(bel, "OUT0".into()),
                                PipWire::BelPinNear(bel, "F15".into()),
                            ));
                    }

                    if let Some(idx) = wires::IMUX_IO_T.index_of(wire_to.wire) {
                        let bel = bslots::IO[idx];
                        builder = builder
                            .prop(BaseBelMode::new(bel, "IOB".into()))
                            .prop(BaseBelAttr::new(bel, "TRI".into(), "T".into()))
                            .prop(BaseBelPin::new(bel, "T".into()));
                        if edev.chip.kind != ChipKind::Xc4000E {
                            builder =
                                builder.prop(BaseBelAttr::new(bel, "OUTMUX".into(), "O".into()));
                        }
                    }

                    if let Some(idx) = wires::IMUX_TBUF_I.index_of(wire_to.wire) {
                        let bel = bslots::TBUF[idx];
                        builder = builder.prop(BaseBelMode::new(bel, "TBUF".into())).prop(
                            FuzzBelAttr::new(
                                bel,
                                "TBUFATTR".into(),
                                "WANDT".into(),
                                "WORAND".into(),
                            ),
                        );
                    }
                    if let Some(idx) = wires::IMUX_TBUF_T.index_of(wire_to.wire) {
                        let bel = bslots::TBUF[idx];
                        builder = builder
                            .prop(FuzzBelMode::new(bel, "".into(), "TBUF".into()))
                            .prop(FuzzBelAttr::new(
                                bel,
                                "TBUFATTR".into(),
                                "".into(),
                                "WANDT".into(),
                            ));
                    }
                    builder.commit();
                }
            }
        }
        for (idx, bslot) in bslots::TBUF.into_iter().enumerate() {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            if kind.is_clb_xl() && tcname.starts_with("CLB") {
                let wt = TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[idx]);
                let wf = TileWireCoord::new_idx(0, wires::LONG_V[0]);
                bctx.mode("TBUF")
                    .prop(BaseIntPip::new(wt, wf))
                    .test_bel_attr_bits(bcls::TBUF::DRIVE1)
                    .attr_diff("TBUFATTR", "WORAND", "TBUF")
                    .prop(WireMutexExclusive::new(wt))
                    .prop(WireMutexExclusive::new(wf))
                    .commit();
            } else {
                bctx.mode("TBUF")
                    .test_bel_attr_bits(bcls::TBUF::DRIVE1)
                    .attr_diff("TBUFATTR", "WORAND", "TBUF")
                    .commit();
            }
        }
        for bslot in [bslots::BUFG_H, bslots::BUFG_V] {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                let (opt, out, inp) = match (tcid, bslot) {
                    (tcls::CNR_NW, bslots::BUFG_V) => {
                        ("GCLK1", wires::IMUX_BUFG_V, wires::OUT_IO_CLKIN_N)
                    }
                    (tcls::CNR_SW, bslots::BUFG_V) => {
                        ("GCLK2", wires::IMUX_BUFG_V, wires::OUT_IO_CLKIN_S)
                    }
                    (tcls::CNR_SW, bslots::BUFG_H) => {
                        ("GCLK3", wires::IMUX_BUFG_H, wires::OUT_IO_CLKIN_W)
                    }
                    (tcls::CNR_SE, bslots::BUFG_H) => {
                        ("GCLK4", wires::IMUX_BUFG_H, wires::OUT_IO_CLKIN_E)
                    }
                    (tcls::CNR_SE, bslots::BUFG_V) => {
                        ("GCLK5", wires::IMUX_BUFG_V, wires::OUT_IO_CLKIN_S)
                    }
                    (tcls::CNR_NE, bslots::BUFG_V) => {
                        ("GCLK6", wires::IMUX_BUFG_V, wires::OUT_IO_CLKIN_N)
                    }
                    (tcls::CNR_NE, bslots::BUFG_H) => {
                        ("GCLK7", wires::IMUX_BUFG_H, wires::OUT_IO_CLKIN_E)
                    }
                    (tcls::CNR_NW, bslots::BUFG_H) => {
                        ("GCLK8", wires::IMUX_BUFG_H, wires::OUT_IO_CLKIN_W)
                    }
                    _ => unreachable!(),
                };
                let wt = TileWireCoord::new_idx(0, out);
                let wf = TileWireCoord::new_idx(0, inp);
                bctx.build()
                    .prop(BaseIntPip::new(wt, wf))
                    .test_bel_attr_bits(bcls::BUFG::ALT_PAD)
                    .global(opt, "ALTPAD")
                    .prop(WireMutexExclusive::new(wt))
                    .prop(WireMutexExclusive::new(wf))
                    .commit();
                bctx.build()
                    .prop(BaseIntPip::new(wt, wf))
                    .test_bel_attr_bits(bcls::BUFG::CLK_EN)
                    .global(opt, "CLKEN")
                    .prop(WireMutexExclusive::new(wt))
                    .prop(WireMutexExclusive::new(wf))
                    .commit();
            }
        }
        for slots in [
            bslots::PULLUP_TBUF.as_slice(),
            bslots::PULLUP_TBUF_W.as_slice(),
            bslots::PULLUP_TBUF_E.as_slice(),
            bslots::PULLUP_DEC_H.as_slice(),
            bslots::PULLUP_DEC_V.as_slice(),
            bslots::PULLUP_DEC_W.as_slice(),
            bslots::PULLUP_DEC_E.as_slice(),
            bslots::PULLUP_DEC_S.as_slice(),
            bslots::PULLUP_DEC_N.as_slice(),
        ] {
            for &slot in slots {
                if !tcls.bels.contains_id(slot) {
                    continue;
                }
                let mut bctx = ctx.bel(slot);
                bctx.build()
                    .test_bel_attr_bits(bcls::PULLUP::ENABLE)
                    .pip((PinFar, "O"), "O")
                    .commit();
            }
        }
        for bslot in bslots::TBUF_SPLITTER {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let mut bctx = ctx.bel(bslot);
            for (attr, dir, buf) in [
                (bcls::TBUF_SPLITTER::PASS, DirH::W, false),
                (bcls::TBUF_SPLITTER::PASS, DirH::E, false),
                (bcls::TBUF_SPLITTER::BUF_W, DirH::W, true),
                (bcls::TBUF_SPLITTER::BUF_E, DirH::E, true),
            ] {
                bctx.build()
                    .test_bel_attr_bits(attr)
                    .prop(Xc4000TbufSplitter::new(bslot, dir, buf))
                    .commit();
            }
        }
        for slot in bslots::DEC {
            if !tcls.bels.contains_id(slot) {
                continue;
            }
            let mut bctx = ctx.bel(slot);
            for (pin, attr_p, attr_n) in [
                ("O1", bcls::DEC::O1_P, bcls::DEC::O1_N),
                ("O2", bcls::DEC::O2_P, bcls::DEC::O2_N),
                ("O3", bcls::DEC::O3_P, bcls::DEC::O3_N),
                ("O4", bcls::DEC::O4_P, bcls::DEC::O4_N),
            ] {
                for (attr, val) in [(attr_p, "I"), (attr_n, "NOT")] {
                    bctx.mode("DECODER")
                        .pin(pin)
                        .pin("I")
                        .test_bel_attr_bits(attr)
                        .attr(format!("{pin}MUX"), val)
                        .pip((PinFar, pin), pin)
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
    let kind = edev.chip.kind;
    let intdb = edev.db;
    for (tcid, tcname, tcls) in &intdb.tile_classes {
        if !ctx.has_tile(tcname) {
            continue;
        }
        for bel in tcls.bels.values() {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let mut obuf_dsts = vec![];
            let mut obuf_srcs = vec![];
            let mut gclk_diffs = BTreeMap::new();
            let mut bufgls_diffs = BTreeMap::new();
            for item in &sb.items {
                match item {
                    SwitchBoxItem::Mux(mux) => {
                        if wires::GCLK.contains(mux.dst.wire)
                            && edev.chip.kind == ChipKind::SpartanXl
                        {
                            let mut diffs = vec![];
                            for &src in mux.src.keys() {
                                let idx = wires::BUFGLS_H.index_of(src.wire).unwrap();
                                let bufg = TileWireCoord {
                                    wire: wires::BUFGLS[idx],
                                    ..src.tw
                                };
                                let diff = ctx.state.get_diff_raw(&DiffKey::RoutingVia(
                                    tcid,
                                    mux.dst,
                                    src,
                                    bufg.pos(),
                                ));
                                match bufgls_diffs.entry(src) {
                                    btree_map::Entry::Vacant(e) => {
                                        e.insert(diff.clone());
                                    }
                                    btree_map::Entry::Occupied(mut e) => {
                                        e.get_mut()
                                            .bits
                                            .retain(|bit, _| diff.bits.contains_key(bit));
                                    }
                                }
                                diffs.push((Some(src), diff));
                            }
                            gclk_diffs.insert(mux.dst, diffs);
                            continue;
                        }

                        if wires::QBUF.contains(mux.dst.wire) {
                            let wire_mid = mux.dst;
                            let mut mux_diffs = vec![];
                            for &wire_to in mux.src.keys() {
                                let wire_to = wire_to.tw;
                                let mut diffs = vec![];
                                for &wire_from in mux.src.keys() {
                                    if wire_to == wire_from.tw {
                                        continue;
                                    }
                                    let diff = ctx.state.get_diff_raw(&DiffKey::RoutingVia(
                                        tcid,
                                        wire_to,
                                        wire_mid.pos(),
                                        wire_from,
                                    ));
                                    diffs.push((Some(wire_from), diff.clone()));
                                }
                                let mut odiff = diffs[0].1.clone();
                                for (_, diff) in &diffs {
                                    odiff.bits.retain(|bit, _| diff.bits.contains_key(bit));
                                }
                                for (_, diff) in &mut diffs {
                                    *diff = diff.combine(&!&odiff);
                                }
                                {
                                    ctx.insert_pass(tcid, wire_to, wire_mid, xlat_bit_raw(odiff));
                                }
                                mux_diffs.extend(diffs);
                            }
                            ctx.insert_mux(tcid, wire_mid, xlat_enum_raw(mux_diffs, OcdMode::Mux));
                            continue;
                        }

                        let mut inps = vec![];
                        let mut got_empty = false;
                        for &src in mux.src.keys() {
                            if wires::IMUX_TBUF_T.contains(mux.dst.wire) && src.wire == wires::TIE_1
                            {
                                // handled below
                                continue;
                            }
                            if (wires::IMUX_IO_O1.contains(mux.dst.wire)
                                || wires::IMUX_TBUF_I.contains(mux.dst.wire))
                                && src.wire == wires::TIE_0
                            {
                                assert!(!got_empty);
                                inps.push((Some(src), Diff::default()));
                                got_empty = true;
                                continue;
                            }
                            let mut diff = ctx
                                .state
                                .get_diff_raw(&DiffKey::Routing(tcid, mux.dst, src));
                            if edev.chip.kind == ChipKind::Xc4000E
                                && tcname.starts_with("IO_W")
                                && mux.dst.wire == wires::IMUX_TBUF_I[1]
                                && src.wire == wires::DEC_V[1]
                            {
                                // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                                // found by diffing XC4000E with xact
                                assert!(!diff.bits.contains_key(&TileBit::new(0, 11, 1)));
                                diff.bits.insert(TileBit::new(0, 11, 1), false);
                            }
                            if diff.bits.is_empty() {
                                got_empty = true;
                            }
                            inps.push((Some(src), diff));
                        }
                        if wires::IMUX_IO_T.contains(mux.dst.wire) {
                            // ... I fucking can't with this fpga; look, let's just... not think about it
                            got_empty = true;
                        }
                        if let Some(idx) = wires::IMUX_TBUF_T.index_of(mux.dst.wire) {
                            let bel = bslots::TBUF[idx];
                            let mut drive1 =
                                ctx.get_diff_attr_bit(tcid, bel, bcls::TBUF::DRIVE1, 0);
                            for (_, diff) in &mut inps {
                                *diff = diff.combine(&drive1);
                            }
                            if kind.is_xl() && !tcname.starts_with("IO_W") {
                                let dup = drive1.split_bits_by(|bit| {
                                    if tcname.starts_with("CLB") {
                                        bit.frame.to_idx() != 23
                                    } else {
                                        bit.bit.to_idx() != 3
                                    }
                                });
                                ctx.insert_bel_attr_bool(
                                    tcid,
                                    bel,
                                    bcls::TBUF::DRIVE1_DUP,
                                    xlat_bit_raw(dup),
                                );
                            }
                            ctx.insert_bel_attr_bool(
                                tcid,
                                bel,
                                bcls::TBUF::DRIVE1,
                                xlat_bit_raw(drive1),
                            );
                            inps.push((
                                Some(TileWireCoord::new_idx(0, wires::TIE_1).pos()),
                                Diff::default(),
                            ));
                            assert!(!got_empty);
                            got_empty = true;
                        }

                        for (rtile, rwire, rbel, rattr) in [
                            (
                                tcls::CNR_SW,
                                wires::IMUX_IO_IK[1],
                                bslots::MD1,
                                bcls::MD1::T_ENABLE,
                            ),
                            (
                                tcls::CNR_SW,
                                wires::IMUX_IO_O1[1],
                                bslots::MD1,
                                bcls::MD1::O_ENABLE,
                            ),
                            (
                                tcls::CNR_SW,
                                wires::IMUX_RDBK_TRIG,
                                bslots::RDBK,
                                bcls::RDBK::ENABLE,
                            ),
                            (
                                tcls::CNR_SE,
                                wires::IMUX_STARTUP_GTS,
                                bslots::STARTUP,
                                bcls::STARTUP::GTS_ENABLE,
                            ),
                            (
                                tcls::CNR_SE,
                                wires::IMUX_STARTUP_GSR,
                                bslots::STARTUP,
                                bcls::STARTUP::GSR_ENABLE,
                            ),
                            (
                                tcls::CNR_NE,
                                wires::IMUX_TDO_T,
                                bslots::TDO,
                                bcls::TDO::T_ENABLE,
                            ),
                            (
                                tcls::CNR_NE,
                                wires::IMUX_TDO_O,
                                bslots::TDO,
                                bcls::TDO::O_ENABLE,
                            ),
                        ] {
                            if tcid == rtile && mux.dst.wire == rwire {
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
                                ctx.insert_bel_attr_bool(tcid, rbel, rattr, xlat_bit_raw(common));
                            }
                        }

                        if edev.chip.kind == ChipKind::Xc4000E {
                            let iob_mux_off_d = if tcname.starts_with("IO_E")
                                && mux.dst.wire == wires::IMUX_CLB_G1
                            {
                                Some(("IO_E", 0))
                            } else if tcname.starts_with("IO_E")
                                && mux.dst.wire == wires::IMUX_CLB_F1
                            {
                                Some(("IO_E", 1))
                            } else if tcname.starts_with("IO_S")
                                && mux.dst.wire == wires::IMUX_CLB_F4
                            {
                                Some(("IO_S", 0))
                            } else if tcname.starts_with("IO_S")
                                && mux.dst.wire == wires::IMUX_CLB_G4
                            {
                                Some(("IO_S", 1))
                            } else if matches!(tcid, tcls::CLB_W | tcls::CLB_SW | tcls::CLB_NW)
                                && mux.dst.wire == wires::IMUX_CLB_G3
                            {
                                Some(("IO_W", 0))
                            } else if matches!(tcid, tcls::CLB_W | tcls::CLB_SW | tcls::CLB_NW)
                                && mux.dst.wire == wires::IMUX_CLB_F3
                            {
                                Some(("IO_W", 1))
                            } else if matches!(tcid, tcls::CLB_N | tcls::CLB_NW | tcls::CLB_NE)
                                && mux.dst.wire == wires::IMUX_CLB_F2
                            {
                                Some(("IO_N", 0))
                            } else if matches!(tcid, tcls::CLB_N | tcls::CLB_NW | tcls::CLB_NE)
                                && mux.dst.wire == wires::IMUX_CLB_G2
                            {
                                Some(("IO_N", 1))
                            } else {
                                None
                            };
                            if let Some((filter, idx)) = iob_mux_off_d {
                                let bel = bslots::IO[idx];
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
                                if tcls.bels.contains_id(bel) {
                                    assert!(tcname.starts_with(filter));
                                    ctx.insert_bel_attr_raw(
                                        tcid,
                                        bel,
                                        bcls::IO::MUX_OFF_D,
                                        xlat_enum_attr(vec![
                                            (enums::IO_MUX_OFF_D::O1, Diff::default()),
                                            (enums::IO_MUX_OFF_D::O2, common),
                                        ]),
                                    );
                                } else {
                                    let (mut bit, val) = common.bits.into_iter().next().unwrap();
                                    assert_ne!(bit.rect.to_idx(), 0);
                                    bit.rect = BitRectId::from_idx(0);
                                    let common = Diff {
                                        bits: [(bit, val)].into_iter().collect(),
                                    };
                                    for (io_tcid, io_tcname, _) in &intdb.tile_classes {
                                        if io_tcname.starts_with(filter) && ctx.has_tile_id(io_tcid)
                                        {
                                            ctx.insert_bel_attr_raw(
                                                io_tcid,
                                                bel,
                                                bcls::IO::MUX_OFF_D,
                                                xlat_enum_attr(vec![
                                                    (enums::IO_MUX_OFF_D::O1, Diff::default()),
                                                    (enums::IO_MUX_OFF_D::O2, common.clone()),
                                                ]),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        if !got_empty {
                            inps.push((None, Diff::default()));
                        }
                        inps.sort_by_key(|&(k, _)| k);
                        let item = xlat_enum_raw(inps, OcdMode::Mux);
                        if item.0.is_empty() {
                            println!(
                                "UMMM MUX {tcname} {mux_name} is empty",
                                mux_name = mux.dst.to_string(intdb, &intdb[tcid])
                            );
                        }
                        ctx.insert_mux(tcid, mux.dst, item);
                    }
                    SwitchBoxItem::PermaBuf(buf) => {
                        let diff = ctx
                            .state
                            .get_diff_raw(&DiffKey::Routing(tcid, buf.dst, buf.src));
                        diff.assert_empty();
                    }
                    SwitchBoxItem::ProgBuf(buf) => {
                        if wires::BUFGLS_H.contains(buf.dst.wire)
                            && edev.chip.kind == ChipKind::SpartanXl
                        {
                            continue;
                        }
                        if buf.src.wire == wires::OBUF {
                            obuf_dsts.push(buf.dst);
                            continue;
                        }
                        if buf.dst.wire == wires::OBUF {
                            obuf_srcs.push(buf.src);
                            continue;
                        }
                        ctx.collect_progbuf(tcid, buf.dst, buf.src);
                    }
                    SwitchBoxItem::Pass(pass) => {
                        if wires::QBUF.contains(pass.src.wire) {
                            continue;
                        }
                        ctx.collect_pass(tcid, pass.dst, pass.src);
                    }
                    SwitchBoxItem::BiPass(pass) => {
                        ctx.collect_bipass(tcid, pass.a, pass.b);
                    }
                    _ => unreachable!(),
                }
            }

            let obuf = TileWireCoord::new_idx(0, wires::OBUF);
            for dst in obuf_dsts {
                let mut diffs = vec![];
                for &src in &obuf_srcs {
                    if src.tw == dst {
                        continue;
                    }
                    diffs.push((
                        src,
                        ctx.state
                            .get_diff_raw(&DiffKey::RoutingVia(tcid, dst, obuf.pos(), src)),
                    ));
                }
                let mut odiff = diffs[0].1.clone();
                for (_, diff) in &diffs {
                    odiff.bits.retain(|bit, _| diff.bits.contains_key(bit));
                }
                for (src, diff) in diffs {
                    let diff = diff.combine(&!&odiff);
                    ctx.insert_progbuf(tcid, obuf, src, xlat_bit_raw(diff));
                }
                ctx.insert_progbuf(tcid, dst, obuf.pos(), xlat_bit_raw(odiff));
            }

            for (dst, mut diffs) in gclk_diffs {
                for (src, diff) in &mut diffs {
                    let src = src.unwrap();
                    *diff = diff.combine(&!&bufgls_diffs[&src]);
                }
                diffs.push((None, Diff::default()));
                ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
            }

            for (dst, diff) in bufgls_diffs {
                let idx = wires::BUFGLS_H.index_of(dst.wire).unwrap();
                let bufg = TileWireCoord {
                    wire: wires::BUFGLS[idx],
                    ..dst.tw
                };
                ctx.insert_progbuf(tcid, dst.tw, bufg.pos(), xlat_bit_raw(diff));
            }
        }
        for slots in [
            bslots::PULLUP_TBUF.as_slice(),
            bslots::PULLUP_TBUF_W.as_slice(),
            bslots::PULLUP_TBUF_E.as_slice(),
            bslots::PULLUP_DEC_H.as_slice(),
            bslots::PULLUP_DEC_V.as_slice(),
            bslots::PULLUP_DEC_W.as_slice(),
            bslots::PULLUP_DEC_E.as_slice(),
            bslots::PULLUP_DEC_S.as_slice(),
            bslots::PULLUP_DEC_N.as_slice(),
        ] {
            for &bslot in slots {
                if !tcls.bels.contains_id(bslot) {
                    continue;
                }
                ctx.collect_bel_attr(tcid, bslot, bcls::PULLUP::ENABLE);
            }
        }
        for bslot in bslots::DEC {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            for attr in [
                bcls::DEC::O1_P,
                bcls::DEC::O1_N,
                bcls::DEC::O2_P,
                bcls::DEC::O2_N,
                bcls::DEC::O3_P,
                bcls::DEC::O3_N,
                bcls::DEC::O4_P,
                bcls::DEC::O4_N,
            ] {
                ctx.collect_bel_attr(tcid, bslot, attr);
            }
        }
        for bslot in [bslots::BUFG_H, bslots::BUFG_V] {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            if matches!(
                edev.chip.kind,
                ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
            ) {
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFG::ALT_PAD);
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFG::CLK_EN);
            }
        }
        for bslot in bslots::TBUF_SPLITTER {
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            ctx.collect_bel_attr(tcid, bslot, bcls::TBUF_SPLITTER::PASS);
            ctx.collect_bel_attr(tcid, bslot, bcls::TBUF_SPLITTER::BUF_W);
            ctx.collect_bel_attr(tcid, bslot, bcls::TBUF_SPLITTER::BUF_E);
        }
    }
}
