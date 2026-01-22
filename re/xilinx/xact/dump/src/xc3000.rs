use std::collections::{BTreeMap, BTreeSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, IntDb, ProgInv, SwitchBoxItem, TileWireCoord, WireKind, WireSlotId},
    dir::Dir,
    grid::{CellCoord, DieId, EdgeIoCoord},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_types::bsdata::PolTileBit;
use prjcombine_xc2000::{
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    xc3000 as defs,
    xc3000::{bcls, wires},
};

use crate::extractor::{Extractor, NetBinding, PipMode};

pub fn make_chip(die: &Die, kind: ChipKind) -> Chip {
    let pd_clb = die
        .primdefs
        .iter()
        .find(|(_, pd)| pd.name == "xcle")
        .unwrap()
        .0;
    let mut clb_x = BTreeSet::new();
    let mut clb_y = BTreeSet::new();
    for prim in die.prims.values() {
        if prim.primdef != pd_clb {
            continue;
        }
        clb_x.insert(prim.pins.first().unwrap().x);
        clb_y.insert(prim.pins.first().unwrap().y);
    }
    Chip {
        kind,
        columns: clb_x.len(),
        rows: clb_y.len(),
        cols_bidi: Default::default(),
        rows_bidi: Default::default(),
        is_small: clb_x.len() == 8,
        is_buff_large: false,
        cfg_io: Default::default(),
        unbonded_io: BTreeSet::new(),
    }
}

fn is_single_stub(wire: WireSlotId, stub: WireSlotId) -> bool {
    if let Some(idx) = wires::SINGLE_H.index_of(wire) {
        stub == wires::SINGLE_H_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_HS.index_of(wire) {
        stub == wires::SINGLE_HS_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_HN.index_of(wire) {
        stub == wires::SINGLE_HN_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_V.index_of(wire) {
        stub == wires::SINGLE_V_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_V_S.index_of(wire) {
        stub == wires::SINGLE_V_S_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_VW.index_of(wire) {
        stub == wires::SINGLE_VW_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_VW_S.index_of(wire) {
        stub == wires::SINGLE_VW_S_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_VE.index_of(wire) {
        stub == wires::SINGLE_VE_STUB[idx]
    } else if let Some(idx) = wires::SINGLE_VE_S.index_of(wire) {
        stub == wires::SINGLE_VE_S_STUB[idx]
    } else {
        false
    }
}

fn wire_as_ioclk(wire: WireSlotId) -> Option<(Dir, usize)> {
    if let Some(idx) = wires::IOCLK_W.index_of(wire) {
        Some((Dir::W, idx))
    } else if let Some(idx) = wires::IOCLK_E.index_of(wire) {
        Some((Dir::E, idx))
    } else if let Some(idx) = wires::IOCLK_S.index_of(wire) {
        Some((Dir::S, idx))
    } else if let Some(idx) = wires::IOCLK_N.index_of(wire) {
        Some((Dir::N, idx))
    } else {
        None
    }
}

pub fn dump_chip(die: &Die, kind: ChipKind) -> (Chip, IntDb, NamingDb) {
    let chip = make_chip(die, kind);
    let mut intdb: IntDb = bincode::decode_from_slice(defs::INIT, bincode::config::standard())
        .unwrap()
        .0;
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.tile_namings.insert(name.clone(), TileNaming::default());
        if name.starts_with("CLB") && !name.contains("W_") && !name.contains("E_") {
            ndb.tile_namings
                .insert(format!("{name}_W1"), TileNaming::default());
            if !name.starts_with("CLB_S") && !name.starts_with("CLB_N") {
                ndb.tile_namings
                    .insert(format!("{name}_W1_S1"), TileNaming::default());
            }
        }
        if name.starts_with("CLB") && !name.starts_with("CLB_S") && !name.starts_with("CLB_N") {
            ndb.tile_namings
                .insert(format!("{name}_S1"), TileNaming::default());
        }
    }
    let bd_c20 = die
        .boxdefs
        .iter()
        .find(|(_, bd)| bd.name == "cross20")
        .unwrap()
        .0;
    let mut c20_x = BTreeSet::new();
    let mut c20_y = BTreeSet::new();
    for boxx in die.boxes.values() {
        if boxx.boxdef != bd_c20 {
            continue;
        }
        c20_x.insert(usize::from(boxx.bx));
        c20_y.insert(usize::from(boxx.by));
    }
    let c20_x = Vec::from_iter(c20_x);
    let c20_y = Vec::from_iter(c20_y);
    assert_eq!(c20_x.len(), chip.columns - 1);
    assert_eq!(c20_y.len(), chip.rows - 1);
    ndb.tile_widths.insert("L".into(), c20_x[0] - 1);
    ndb.tile_widths.insert("C".into(), c20_x[1] - c20_x[0]);
    ndb.tile_widths.insert(
        "R".into(),
        die.matrix.as_ref().unwrap().dim().0 - (c20_x[chip.columns - 2] - 1),
    );
    ndb.tile_heights.insert("B".into(), c20_y[0] + 2);
    ndb.tile_heights.insert("C".into(), c20_y[1] - c20_y[0]);
    ndb.tile_heights.insert(
        "T".into(),
        die.matrix.as_ref().unwrap().dim().1 - (c20_y[chip.rows - 2] + 2),
    );
    let edev = chip.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);

    let mut extractor = Extractor::new(die, &edev, &endev.ngrid);

    let die = DieId::from_idx(0);
    for (tcrd, tile) in edev.tiles() {
        let tcld = &intdb[tile.class];
        let ntile = &endev.ngrid.tiles[&tcrd];
        for (slot, bel_info) in &tcld.bels {
            let bel = tcrd.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);
            match slot {
                defs::bslots::INT
                | defs::bslots::LLH
                | defs::bslots::LLV
                | defs::bslots::MISC_SW
                | defs::bslots::MISC_SE
                | defs::bslots::MISC_NW
                | defs::bslots::MISC_NE
                | defs::bslots::MISC_E => (),
                defs::bslots::BUFG => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let BelInfo::SwitchBox(sb) = bel_info else {
                        unreachable!();
                    };
                    assert_eq!(sb.items.len(), 1);
                    let SwitchBoxItem::PermaBuf(buf) = sb.items[0] else {
                        unreachable!();
                    };
                    extractor.net_int(prim.get_pin("O"), edev.tile_wire(tcrd, buf.dst));
                    extractor.net_int(prim.get_pin("I"), edev.tile_wire(tcrd, buf.src.tw));
                }
                defs::bslots::CLB | defs::bslots::OSC | defs::bslots::CLKIOB => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let BelInfo::Bel(bel_info) = bel_info else {
                        unreachable!();
                    };
                    for pid in bel_info.inputs.ids() {
                        extractor.pin_bel_input(&mut prim, bel, pid);
                    }
                    for pid in bel_info.outputs.ids() {
                        extractor.pin_bel_output(&mut prim, bel, pid);
                    }
                }
                _ if defs::bslots::IO_W.contains(slot)
                    || defs::bslots::IO_E.contains(slot)
                    || defs::bslots::IO_S.contains(slot)
                    || defs::bslots::IO_N.contains(slot) =>
                {
                    let mut prim = extractor.grab_prim_i(&ntile.bels[slot][0]);
                    let BelInfo::Bel(bel_info) = bel_info else {
                        unreachable!();
                    };
                    for pid in bel_info.inputs.ids() {
                        extractor.pin_bel_input(&mut prim, bel, pid);
                    }
                    for pid in bel_info.outputs.ids() {
                        extractor.pin_bel_output(&mut prim, bel, pid);
                    }
                }
                _ if defs::bslots::TBUF.contains(slot) || defs::bslots::TBUF_E.contains(slot) => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let BelInfo::Bel(bel_info) = bel_info else {
                        unreachable!();
                    };
                    for pid in bel_info.inputs.ids() {
                        extractor.pin_bel_input(&mut prim, bel, pid);
                    }
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    extractor.bel_pip(ntile.naming, slot, "O", pip);

                    let wire_i = edev.get_bel_input(bel, bcls::TBUF::I);
                    let net_i = extractor.get_wire_net(wire_i.wire);
                    let wire_o = edev.get_bel_bidir(bel, bcls::TBUF::O);
                    extractor.net_int(line, wire_o);
                    let net_o = extractor.get_wire_net(wire_o);
                    let src_nets = Vec::from_iter(extractor.nets[net_i].pips_bwd.keys().copied());
                    for net in src_nets {
                        extractor.mark_tbuf_pseudo(net_o, net);
                    }
                }
                _ if defs::bslots::PULLUP_TBUF.contains(slot) => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    let wire_o = edev.get_bel_bidir(bel, bcls::PULLUP::O);
                    extractor.net_int(line, wire_o);
                    extractor.bel_pip(ntile.naming, slot, "O", pip);
                }
                _ => panic!("umm bel {slot_name}?"),
            }
        }
    }
    extractor.junk_prim_names.extend(
        ["VCC", "GND", "M0RT", "M1RD", "DPGM", "RST", "PWRDN", "CCLK"]
            .into_iter()
            .map(|x| x.to_string()),
    );

    // long verticals + GCLK
    for col in edev.cols(die) {
        let mut queue = vec![];
        for row in [chip.row_s() + 1, chip.row_n() - 1] {
            let by = endev.row_y[row].start;
            let ty = endev.row_y[row].end;
            let mut nets = vec![];
            for x in endev.col_x[col].clone() {
                let net_b = extractor.matrix_nets[(x, by)].net_b;
                let net_t = extractor.matrix_nets[(x, ty)].net_b;
                if net_b != net_t {
                    continue;
                }
                let Some(net) = net_b else { continue };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if col == chip.col_w() {
                &[
                    wires::IOCLK_W[0],
                    wires::IOCLK_W[1],
                    wires::LONG_IO_W[0],
                    wires::LONG_IO_W[1],
                    wires::LONG_V[0],
                    wires::LONG_V[1],
                    wires::ACLK_V,
                    wires::GCLK_V,
                ][..]
            } else if col == chip.col_e() {
                &[
                    wires::GCLK_V,
                    wires::LONG_V[0],
                    wires::LONG_V[1],
                    wires::ACLK_V,
                    wires::LONG_IO_E[1],
                    wires::LONG_IO_E[0],
                    wires::IOCLK_E[1],
                    wires::IOCLK_E[0],
                ][..]
            } else {
                &[
                    wires::GCLK_V,
                    wires::LONG_V[0],
                    wires::LONG_V[1],
                    wires::ACLK_V,
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }
    // long horizontals
    for row in edev.rows(die) {
        let mut queue = vec![];
        for col in [chip.col_w() + 1, chip.col_e() - 1] {
            let lx = endev.col_x[col].start;
            let rx = endev.col_x[col].end;
            let mut nets = vec![];
            for y in endev.row_y[row].clone() {
                let net_l = extractor.matrix_nets[(lx, y)].net_l;
                let net_r = extractor.matrix_nets[(rx, y)].net_l;
                if net_l != net_r {
                    continue;
                }
                let Some(net) = net_l else { continue };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if row == chip.row_s() {
                &[
                    wires::IOCLK_S[0],
                    wires::IOCLK_S[1],
                    wires::LONG_IO_S[0],
                    wires::LONG_IO_S[1],
                ][..]
            } else if row == chip.row_n() {
                &[
                    wires::LONG_IO_N[1],
                    wires::LONG_IO_N[0],
                    wires::IOCLK_N[1],
                    wires::IOCLK_N[0],
                ][..]
            } else {
                &[][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // assign LL splitters to their proper tiles
    let mut queue = vec![];
    for (net_t, net_info) in &extractor.nets {
        let NetBinding::Int(rwt) = net_info.binding else {
            continue;
        };
        for &net_f in net_info.pips_bwd.keys() {
            let NetBinding::Int(rwf) = extractor.nets[net_f].binding else {
                continue;
            };
            if rwt.slot != rwf.slot {
                continue;
            }
            if rwt.cell.col == rwf.cell.col {
                assert_ne!(rwt.cell.row, rwf.cell.row);
                // LLV
                let col = rwt.cell.col;
                let row = chip.row_mid();
                queue.push((
                    net_t,
                    net_f,
                    CellCoord::new(die, col, row).tile(defs::tslots::LLV),
                ))
            } else {
                assert_eq!(rwt.cell.row, rwf.cell.row);
                // LLH
                let col = chip.col_mid();
                let row = rwt.cell.row;
                queue.push((
                    net_t,
                    net_f,
                    CellCoord::new(die, col, row).tile(defs::tslots::LLH),
                ))
            }
        }
    }
    for (net_t, net_f, tcrd) in queue {
        extractor.own_pip(net_t, net_f, tcrd);
    }

    // horizontal singles
    let mut queue = vec![];
    for col in edev.cols(die) {
        let mut x = endev.col_x[col].end;
        if col == chip.col_e() {
            x = endev.col_x[col].start + 8;
        }
        for row in edev.rows(die) {
            let mut nets = vec![];
            for y in endev.row_y[row].clone() {
                let Some(net) = extractor.matrix_nets[(x, y)].net_l else {
                    continue;
                };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if row == chip.row_s() {
                &[
                    wires::SINGLE_HS[4],
                    wires::SINGLE_HS[3],
                    wires::SINGLE_HS[2],
                    wires::SINGLE_HS[1],
                    wires::SINGLE_HS[0],
                    wires::SINGLE_H[4],
                    wires::SINGLE_H[3],
                    wires::SINGLE_H[2],
                    wires::SINGLE_H[1],
                    wires::SINGLE_H[0],
                ][..]
            } else if row == chip.row_n() {
                &[
                    wires::SINGLE_HN[4],
                    wires::SINGLE_HN[3],
                    wires::SINGLE_HN[2],
                    wires::SINGLE_HN[1],
                    wires::SINGLE_HN[0],
                ][..]
            } else {
                &[
                    wires::SINGLE_H[4],
                    wires::SINGLE_H[3],
                    wires::SINGLE_H[2],
                    wires::SINGLE_H[1],
                    wires::SINGLE_H[0],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
    }
    // vertical singles
    for row in edev.rows(die) {
        let mut y = endev.row_y[row].start;
        if row == chip.row_s() {
            y = endev.row_y[row + 1].start - 8;
        }
        for col in edev.cols(die) {
            let mut nets = vec![];
            for x in endev.col_x[col].clone() {
                let Some(net) = extractor.matrix_nets[(x, y)].net_b else {
                    continue;
                };
                if extractor.nets[net].binding != NetBinding::None {
                    continue;
                }
                nets.push(net);
            }
            let wires = if col == chip.col_w() {
                &[
                    wires::SINGLE_VW[0],
                    wires::SINGLE_VW[1],
                    wires::SINGLE_VW[2],
                    wires::SINGLE_VW[3],
                    wires::SINGLE_VW[4],
                ][..]
            } else if col == chip.col_e() {
                &[
                    wires::SINGLE_V[0],
                    wires::SINGLE_V[1],
                    wires::SINGLE_V[2],
                    wires::SINGLE_V[3],
                    wires::SINGLE_V[4],
                    wires::SINGLE_VE[0],
                    wires::SINGLE_VE[1],
                    wires::SINGLE_VE[2],
                    wires::SINGLE_VE[3],
                    wires::SINGLE_VE[4],
                ][..]
            } else {
                &[
                    wires::SINGLE_V[0],
                    wires::SINGLE_V[1],
                    wires::SINGLE_V[2],
                    wires::SINGLE_V[3],
                    wires::SINGLE_V[4],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    // find single stubs
    for x in 0..extractor.matrix.dim().0 {
        for y in 0..extractor.matrix.dim().1 {
            let cv = extractor.matrix[(x, y)] & 0xff;
            if cv == 0x28 {
                // vertical joiner
                let net_u = extractor.matrix_nets[(x, y + 1)].net_b.unwrap();
                let net_d = extractor.matrix_nets[(x, y)].net_b.unwrap();
                if let NetBinding::Int(rw) = extractor.nets[net_u].binding {
                    if rw.cell.row == chip.row_s() {
                        if extractor.nets[net_d].binding == NetBinding::None {
                            let sw = if let Some(idx) = wires::SINGLE_V.index_of(rw.slot) {
                                wires::SINGLE_V_STUB[idx]
                            } else if let Some(idx) = wires::SINGLE_VW.index_of(rw.slot) {
                                wires::SINGLE_VW_STUB[idx]
                            } else if let Some(idx) = wires::SINGLE_VE.index_of(rw.slot) {
                                wires::SINGLE_VE_STUB[idx]
                            } else {
                                unreachable!()
                            };
                            extractor.net_int(net_d, rw.cell.wire(sw));
                        }
                    } else {
                        if extractor.nets[net_d].binding == NetBinding::None {
                            let sw = if let Some(idx) = wires::SINGLE_V.index_of(rw.slot) {
                                wires::SINGLE_V_S_STUB[idx]
                            } else if let Some(idx) = wires::SINGLE_VW.index_of(rw.slot) {
                                wires::SINGLE_VW_S_STUB[idx]
                            } else if let Some(idx) = wires::SINGLE_VE.index_of(rw.slot) {
                                wires::SINGLE_VE_S_STUB[idx]
                            } else {
                                unreachable!()
                            };
                            extractor.net_int(net_d, rw.cell.delta(0, -1).wire(sw));
                        }
                    }
                }
            } else if cv == 0x68 {
                // horizontal joiner
                let net_r = extractor.matrix_nets[(x + 1, y)].net_l.unwrap();
                let net_l = extractor.matrix_nets[(x, y)].net_l.unwrap();
                if let NetBinding::Int(rw) = extractor.nets[net_r].binding
                    && extractor.nets[net_l].binding == NetBinding::None
                {
                    let sw = if let Some(idx) = wires::SINGLE_H.index_of(rw.slot) {
                        wires::SINGLE_H_STUB[idx]
                    } else if let Some(idx) = wires::SINGLE_HS.index_of(rw.slot) {
                        wires::SINGLE_HS_STUB[idx]
                    } else if let Some(idx) = wires::SINGLE_HN.index_of(rw.slot) {
                        wires::SINGLE_HN_STUB[idx]
                    } else {
                        unreachable!()
                    };
                    extractor.net_int(net_l, rw.cell.wire(sw));
                }
            }
        }
    }

    let xlut = endev.col_x.map_values(|x| x.end);
    let ylut = endev.row_y.map_values(|y| y.end);
    for (box_id, boxx) in &extractor.die.boxes {
        let col = xlut.binary_search(&usize::from(boxx.bx)).unwrap_err();
        let row = ylut.binary_search(&usize::from(boxx.by)).unwrap_err();
        extractor.own_box(
            box_id,
            CellCoord::new(die, col, row).tile(defs::tslots::MAIN),
        );
    }

    // find IMUX.IOCLK
    let mut queue = vec![];
    for (net, net_info) in &extractor.nets {
        if net_info.binding != NetBinding::None {
            continue;
        }
        assert_eq!(net_info.pips_fwd.len(), 1);
        let (&net_t, &pip) = net_info.pips_fwd.iter().next().unwrap();
        let NetBinding::Int(rw) = extractor.nets[net_t].binding else {
            unreachable!()
        };
        let (_, idx) = wire_as_ioclk(rw.slot).unwrap();
        let nw = wires::IMUX_IOCLK[idx];
        let col = xlut.binary_search(&pip.0).unwrap_err();
        let row = ylut.binary_search(&pip.1).unwrap_err();
        assert!(col == chip.col_w() || col == chip.col_e());
        assert!(row == chip.row_s() || row == chip.row_n());
        queue.push((net, CellCoord::new(die, col, row).wire(nw)));
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    // fix coords for [AG]CLK -> IOCLK
    let mut queue = vec![];
    for (net, net_info) in &extractor.nets {
        let NetBinding::Int(wire) = net_info.binding else {
            continue;
        };
        let Some((side, idx)) = wire_as_ioclk(wire.slot) else {
            continue;
        };
        let (col, row) = match (side, idx) {
            (Dir::W, 1) | (Dir::S, 0) => (chip.col_w(), chip.row_s()),
            (Dir::E, 0) | (Dir::S, 1) => (chip.col_e(), chip.row_s()),
            (Dir::W, 0) | (Dir::N, 1) => (chip.col_w(), chip.row_n()),
            (Dir::E, 1) | (Dir::N, 0) => (chip.col_e(), chip.row_n()),
            _ => unreachable!(),
        };
        let tcrd = CellCoord::new(die, col, row).tile(defs::tslots::MAIN);
        for &net_f in net_info.pips_bwd.keys() {
            queue.push((net, net_f, tcrd));
        }
    }
    for (net_t, net_f, tcrd) in queue {
        extractor.own_pip(net_t, net_f, tcrd);
    }

    for (wire, name, &kind) in &intdb.wires {
        if !name.starts_with("IMUX") {
            continue;
        }
        if kind != WireKind::MuxOut {
            continue;
        }
        for (cell, _) in edev.cells() {
            let rw = edev.resolve_wire(cell.wire(wire)).unwrap();
            if extractor.int_nets.contains_key(&rw) {
                extractor.own_mux(rw, cell.tile(defs::tslots::MAIN));
            }
        }
    }

    for (tcid, _, tcls) in &intdb.tile_classes {
        for i in 0..2 {
            if tcls.bels.contains_id(defs::bslots::TBUF[i]) {
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[i]),
                    TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                );
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[i]),
                    TileWireCoord::new_idx(0, wires::TIE_1).pos(),
                );
            }
            if tcls.bels.contains_id(defs::bslots::TBUF_E[i]) {
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[i + 2]),
                    TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                );
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[i + 2]),
                    TileWireCoord::new_idx(0, wires::TIE_1).pos(),
                );
            }
        }
        for (slots, imux) in [
            (defs::bslots::IO_W, wires::IMUX_IO_W_T),
            (defs::bslots::IO_E, wires::IMUX_IO_E_T),
            (defs::bslots::IO_S, wires::IMUX_IO_S_T),
            (defs::bslots::IO_N, wires::IMUX_IO_N_T),
        ] {
            for i in 0..2 {
                if tcls.bels.contains_id(slots[i]) {
                    extractor.inject_pip(
                        tcid,
                        TileWireCoord::new_idx(0, imux[i]),
                        TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                    );
                    extractor.inject_pip(
                        tcid,
                        TileWireCoord::new_idx(0, imux[i]),
                        TileWireCoord::new_idx(0, wires::TIE_1).pos(),
                    );
                    extractor.inject_pip(
                        tcid,
                        TileWireCoord::new_idx(0, imux[i]),
                        TileWireCoord::new_idx(0, wires::SPECIAL_IO_PULLUP).pos(),
                    );
                }
            }
        }
    }

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb, |db, tslot, wt, wf| {
        if tslot != defs::tslots::MAIN {
            PipMode::Pass
        } else {
            let wtn = db.wires.key(wt.wire);
            if is_single_stub(wt.wire, wf.wire) || is_single_stub(wf.wire, wt.wire) {
                PipMode::Buf
            } else if wtn.starts_with("IMUX") {
                PipMode::Mux
            } else if wt.wire == wires::GCLK_V && chip.is_small {
                PipMode::PermaBuf
            } else if wtn.starts_with("LONG")
                || wt.wire == wires::ACLK_V
                || wt.wire == wires::GCLK_V
                || wire_as_ioclk(wt.wire).is_some()
            {
                PipMode::Buf
            } else {
                PipMode::Pass
            }
        }
    });

    // fix up IOCLK
    for tcls in intdb.tile_classes.values_mut() {
        for bel in tcls.bels.values_mut() {
            let BelInfo::SwitchBox(sb) = bel else {
                continue;
            };
            let mut imux_to_ioclk = BTreeMap::new();
            let mut ioclk_to_gclk = BTreeMap::new();
            sb.items.retain_mut(|item| {
                if let SwitchBoxItem::ProgBuf(buf) = item
                    && wire_as_ioclk(buf.dst.wire).is_some()
                {
                    if matches!(buf.src.wire, wires::ACLK | wires::GCLK) {
                        ioclk_to_gclk.insert(buf.dst, buf.src);
                        false
                    } else {
                        imux_to_ioclk.insert(buf.src.tw, buf.dst);
                        let dst = buf.dst;
                        let src = buf.src.tw;
                        *item = SwitchBoxItem::ProgInv(ProgInv {
                            dst,
                            src,
                            bit: PolTileBit::DUMMY,
                        });
                        true
                    }
                } else {
                    true
                }
            });
            for item in &mut sb.items {
                let SwitchBoxItem::Mux(mux) = item else {
                    continue;
                };
                if let Some(ioclk) = imux_to_ioclk.remove(&mux.dst) {
                    let gclk = ioclk_to_gclk.remove(&ioclk).unwrap();
                    mux.src.insert(gclk, Default::default());
                }
            }
        }
    }

    (chip, intdb, ndb)
}

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    name: &str,
    pkg: &BTreeMap<String, String>,
) -> (Bond, BTreeMap<SharedCfgPad, EdgeIoCoord>) {
    let io_lookup: BTreeMap<_, _> = endev
        .chip
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io), io))
        .collect();
    let mut bond = Bond {
        pins: Default::default(),
    };
    for (pin, pad) in pkg {
        let io = io_lookup[&**pad];
        bond.pins.insert(pin.to_ascii_uppercase(), BondPad::Io(io));
    }

    let (pwrdwn, m1, m0, prog, done, cclk, gnd, vcc) = match name {
        "pc44" => (
            "P7",
            "P16",
            "P17",
            "P27",
            "P28",
            "P40",
            &["P1", "P23"][..],
            &["P12", "P34"][..],
        ),
        "pc68" => (
            "P10",
            "P25",
            "P26",
            "P44",
            "P45",
            "P60",
            &["P1", "P35"][..],
            &["P18", "P52"][..],
        ),
        "pc84" if endev.chip.columns < 14 => (
            "P12",
            "P31",
            "P32",
            "P54",
            "P55",
            "P74",
            &["P1", "P43"][..],
            &["P22", "P64"][..],
        ),
        "pc84" if endev.chip.columns >= 14 => (
            "P12",
            "P31",
            "P32",
            "P54",
            "P55",
            "P74",
            &["P1", "P21", "P43", "P65"][..],
            &["P2", "P22", "P42", "P64"][..],
        ),

        "pq100" => (
            "P29",
            "P52",
            "P54",
            "P78",
            "P80",
            "P2",
            &["P4", "P16", "P28", "P53", "P66", "P77"][..],
            &["P3", "P27", "P41", "P55", "P79", "P91"][..],
        ),
        "pq160" => (
            "P159",
            "P40",
            "P42",
            "P78",
            "P80",
            "P121",
            &["P19", "P41", "P61", "P77", "P101", "P123", "P139", "P158"][..],
            &["P20", "P43", "P60", "P79", "P100", "P122", "P140", "P157"][..],
        ),
        "pq208" if endev.chip.columns == 16 => (
            "P3",
            "P48",
            "P50",
            "P102",
            "P107",
            "P153",
            &["P2", "P25", "P49", "P79", "P101", "P131", "P160", "P182"][..],
            &["P26", "P55", "P78", "P106", "P130", "P154", "P183", "P205"][..],
        ),
        "pq208" if endev.chip.columns == 22 => (
            "P1",
            "P50",
            "P52",
            "P105",
            "P107",
            "P156",
            &["P26", "P51", "P79", "P104", "P131", "P158", "P182", "P208"][..],
            &["P27", "P53", "P78", "P106", "P130", "P157", "P183", "P207"][..],
        ),

        "vq64" => (
            "P17",
            "P31",
            "P32",
            "P48",
            "P49",
            "P64",
            &["P8", "P41"][..],
            &["P24", "P56"][..],
        ),
        "tq100" | "vq100" => (
            "P26",
            "P49",
            "P51",
            "P75",
            "P77",
            "P99",
            &["P1", "P13", "P25", "P50", "P63", "P74"][..],
            &["P24", "P38", "P52", "P76", "P88", "P100"][..],
        ),
        "tq144" => (
            "P1",
            "P36",
            "P38",
            "P71",
            "P73",
            "P108",
            &["P18", "P37", "P55", "P70", "P91", "P110", "P126", "P144"][..],
            &["P19", "P39", "P54", "P72", "P90", "P109", "P127", "P143"][..],
        ),
        "tq176" => (
            "P1",
            "P45",
            "P47",
            "P87",
            "P89",
            "P132",
            &["P22", "P46", "P67", "P86", "P111", "P134", "P154", "P176"][..],
            &["P23", "P48", "P66", "P88", "P110", "P133", "P155", "P175"][..],
        ),

        "cb100" | "cq100" => (
            "P14",
            "P37",
            "P39",
            "P63",
            "P65",
            "P87",
            &["P1", "P13", "P38", "P51", "P62", "P89"][..],
            &["P12", "P26", "P40", "P64", "P76", "P88"][..],
        ),
        "cb164" | "cq164" => (
            "P20",
            "P62",
            "P64",
            "P101",
            "P103",
            "P145",
            &["P19", "P41", "P63", "P83", "P100", "P124", "P147", "P164"][..],
            &["P1", "P18", "P42", "P65", "P82", "P102", "P123", "P146"][..],
        ),

        "pg84" => (
            "B2",
            "J2",
            "L1",
            "K10",
            "J10",
            "A11",
            &["C6", "J6"][..],
            &["F3", "F9"][..],
        ),
        "pg132" | "pp132" => (
            "A1",
            "B13",
            "A14",
            "P14",
            "N13",
            "P1",
            &["C4", "C7", "C11", "H12", "L12", "M7", "L3", "H3"][..],
            &["C8", "D12", "G12", "M11", "M8", "M4", "G3", "D3"][..],
        ),
        "pp175" | "pg175" => (
            "B2",
            "B14",
            "B15",
            "R15",
            "R14",
            "R2",
            &["D8", "C14", "J14", "N14", "N8", "N3", "J3", "C3"][..],
            &["D9", "D14", "H14", "P14", "N9", "P3", "H3", "D3"][..],
        ),
        "pg223" => (
            "B2",
            "C16",
            "B17",
            "U17",
            "V17",
            "U2",
            &["D9", "D15", "K15", "R16", "R9", "R3", "K4", "D4"][..],
            &["D10", "D16", "J15", "R15", "R10", "R4", "J4", "D3"][..],
        ),

        _ => panic!("ummm {name}?"),
    };
    assert_eq!(
        bond.pins
            .insert(pwrdwn.into(), BondPad::Cfg(CfgPad::PwrdwnB)),
        None
    );
    assert_eq!(bond.pins.insert(m0.into(), BondPad::Cfg(CfgPad::M0)), None);
    assert_eq!(bond.pins.insert(m1.into(), BondPad::Cfg(CfgPad::M1)), None);
    assert_eq!(
        bond.pins.insert(prog.into(), BondPad::Cfg(CfgPad::ProgB)),
        None
    );
    assert_eq!(
        bond.pins.insert(done.into(), BondPad::Cfg(CfgPad::Done)),
        None
    );
    assert_eq!(
        bond.pins.insert(cclk.into(), BondPad::Cfg(CfgPad::Cclk)),
        None
    );
    for &pin in gnd {
        assert_eq!(bond.pins.insert(pin.into(), BondPad::Gnd), None);
    }
    for &pin in vcc {
        assert_eq!(bond.pins.insert(pin.into(), BondPad::Vcc), None);
    }

    let len1d = match name {
        "pc44" => Some(44),
        "pc68" => Some(68),
        "pc84" => Some(84),
        "vq64" => Some(64),
        "vq100" | "tq100" => Some(100),
        "tq144" => Some(144),
        "tq176" => Some(176),
        "pq100" => Some(100),
        "pq160" => Some(160),
        "pq208" => Some(208),
        "cb100" | "cq100" => Some(100),
        "cb164" | "cq164" => Some(164),
        _ => None,
    };
    if let Some(len1d) = len1d {
        for i in 1..=len1d {
            bond.pins.entry(format!("P{i}")).or_insert(BondPad::Nc);
        }
        assert_eq!(bond.pins.len(), len1d);
    }

    match name {
        "pg84" => {
            for a in ["A", "B", "K", "L"] {
                for i in 1..=11 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["C", "D", "E", "F", "G", "H", "J"] {
                for i in (1..=2).chain(10..=11) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["C", "J"] {
                for i in 5..=7 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["E", "F", "G"] {
                for i in [3, 9] {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 84);
        }
        "pg132" | "pp132" => {
            for a in ["A", "B", "C", "M", "N", "P"] {
                for i in 1..=14 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L"] {
                for i in (1..=3).chain(12..=14) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }

            assert_eq!(bond.pins.len(), 132);
        }
        "pp175" | "pg175" => {
            for i in 2..=16 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPad::Nc);
            }
            for a in ["B", "C", "D", "N", "P", "R", "T"] {
                for i in 1..=16 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["E", "F", "G", "H", "J", "K", "L", "M"] {
                for i in (1..=3).chain(14..=16) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 175);
        }
        "pg223" => {
            for i in 2..=18 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPad::Nc);
            }
            for a in ["B", "C", "D", "R", "T", "U", "V"] {
                for i in 1..=18 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["E", "F", "G", "H", "J", "K", "L", "M", "N", "P"] {
                for i in (1..=4).chain(15..=18) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 223);
        }
        _ => (),
    }

    let mut cfg_io = BTreeMap::new();
    if name == "pc68" {
        for (pin, fun) in [
            ("P2", SharedCfgPad::Addr(13)),
            ("P3", SharedCfgPad::Addr(6)),
            ("P4", SharedCfgPad::Addr(12)),
            ("P5", SharedCfgPad::Addr(7)),
            ("P6", SharedCfgPad::Addr(11)),
            ("P7", SharedCfgPad::Addr(8)),
            ("P8", SharedCfgPad::Addr(10)),
            ("P9", SharedCfgPad::Addr(9)),
            ("P27", SharedCfgPad::M2),
            ("P28", SharedCfgPad::Hdc),
            ("P30", SharedCfgPad::Ldc),
            ("P34", SharedCfgPad::InitB),
            ("P46", SharedCfgPad::Data(7)),
            ("P48", SharedCfgPad::Data(6)),
            ("P49", SharedCfgPad::Data(5)),
            ("P50", SharedCfgPad::Cs0B),
            ("P51", SharedCfgPad::Data(4)),
            ("P53", SharedCfgPad::Data(3)),
            ("P54", SharedCfgPad::Cs1B),
            ("P55", SharedCfgPad::Data(2)),
            ("P56", SharedCfgPad::Data(1)),
            ("P57", SharedCfgPad::RclkB),
            ("P58", SharedCfgPad::Data(0)),
            ("P59", SharedCfgPad::Dout),
            ("P61", SharedCfgPad::Addr(0)),
            ("P62", SharedCfgPad::Addr(1)),
            ("P63", SharedCfgPad::Addr(2)),
            ("P64", SharedCfgPad::Addr(3)),
            ("P65", SharedCfgPad::Addr(15)),
            ("P66", SharedCfgPad::Addr(4)),
            ("P67", SharedCfgPad::Addr(14)),
            ("P68", SharedCfgPad::Addr(5)),
        ] {
            let BondPad::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    } else if name == "pc84" && endev.chip.columns < 14 {
        for (pin, fun) in [
            ("P2", SharedCfgPad::Addr(13)),
            ("P3", SharedCfgPad::Addr(6)),
            ("P4", SharedCfgPad::Addr(12)),
            ("P5", SharedCfgPad::Addr(7)),
            ("P8", SharedCfgPad::Addr(11)),
            ("P9", SharedCfgPad::Addr(8)),
            ("P10", SharedCfgPad::Addr(10)),
            ("P11", SharedCfgPad::Addr(9)),
            ("P33", SharedCfgPad::M2),
            ("P34", SharedCfgPad::Hdc),
            ("P36", SharedCfgPad::Ldc),
            ("P42", SharedCfgPad::InitB),
            ("P56", SharedCfgPad::Data(7)),
            ("P58", SharedCfgPad::Data(6)),
            ("P60", SharedCfgPad::Data(5)),
            ("P61", SharedCfgPad::Cs0B),
            ("P62", SharedCfgPad::Data(4)),
            ("P65", SharedCfgPad::Data(3)),
            ("P66", SharedCfgPad::Cs1B),
            ("P67", SharedCfgPad::Data(2)),
            ("P70", SharedCfgPad::Data(1)),
            ("P71", SharedCfgPad::RclkB),
            ("P72", SharedCfgPad::Data(0)),
            ("P73", SharedCfgPad::Dout),
            ("P75", SharedCfgPad::Addr(0)),
            ("P76", SharedCfgPad::Addr(1)),
            ("P77", SharedCfgPad::Addr(2)),
            ("P78", SharedCfgPad::Addr(3)),
            ("P81", SharedCfgPad::Addr(15)),
            ("P82", SharedCfgPad::Addr(4)),
            ("P83", SharedCfgPad::Addr(14)),
            ("P84", SharedCfgPad::Addr(5)),
        ] {
            let BondPad::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    } else if name == "pg132" {
        for (pin, fun) in [
            ("G2", SharedCfgPad::Addr(13)),
            ("G1", SharedCfgPad::Addr(6)),
            ("F2", SharedCfgPad::Addr(12)),
            ("E1", SharedCfgPad::Addr(7)),
            ("D1", SharedCfgPad::Addr(11)),
            ("D2", SharedCfgPad::Addr(8)),
            ("B1", SharedCfgPad::Addr(10)),
            ("C2", SharedCfgPad::Addr(9)),
            ("C13", SharedCfgPad::M2),
            ("B14", SharedCfgPad::Hdc),
            ("D14", SharedCfgPad::Ldc),
            ("G14", SharedCfgPad::InitB),
            ("M12", SharedCfgPad::Data(7)),
            ("N11", SharedCfgPad::Data(6)),
            ("M9", SharedCfgPad::Data(5)),
            ("N9", SharedCfgPad::Cs0B),
            ("N8", SharedCfgPad::Data(4)),
            ("N7", SharedCfgPad::Data(3)),
            ("P6", SharedCfgPad::Cs1B),
            ("M6", SharedCfgPad::Data(2)),
            ("M5", SharedCfgPad::Data(1)),
            ("N4", SharedCfgPad::RclkB),
            ("N2", SharedCfgPad::Data(0)),
            ("M3", SharedCfgPad::Dout),
            ("M2", SharedCfgPad::Addr(0)),
            ("N1", SharedCfgPad::Addr(1)),
            ("L2", SharedCfgPad::Addr(2)),
            ("L1", SharedCfgPad::Addr(3)),
            ("K1", SharedCfgPad::Addr(15)),
            ("J2", SharedCfgPad::Addr(4)),
            ("H1", SharedCfgPad::Addr(14)),
            ("H2", SharedCfgPad::Addr(5)),
        ] {
            let BondPad::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    } else if name == "pq160" {
        for (pin, fun) in [
            ("P141", SharedCfgPad::Addr(13)),
            ("P142", SharedCfgPad::Addr(6)),
            ("P147", SharedCfgPad::Addr(12)),
            ("P148", SharedCfgPad::Addr(7)),
            ("P151", SharedCfgPad::Addr(11)),
            ("P152", SharedCfgPad::Addr(8)),
            ("P155", SharedCfgPad::Addr(10)),
            ("P156", SharedCfgPad::Addr(9)),
            ("P44", SharedCfgPad::M2),
            ("P45", SharedCfgPad::Hdc),
            ("P49", SharedCfgPad::Ldc),
            ("P59", SharedCfgPad::InitB),
            ("P81", SharedCfgPad::Data(7)),
            ("P86", SharedCfgPad::Data(6)),
            ("P92", SharedCfgPad::Data(5)),
            ("P93", SharedCfgPad::Cs0B),
            ("P98", SharedCfgPad::Data(4)),
            ("P102", SharedCfgPad::Data(3)),
            ("P103", SharedCfgPad::Cs1B),
            ("P108", SharedCfgPad::Data(2)),
            ("P114", SharedCfgPad::Data(1)),
            ("P115", SharedCfgPad::RclkB),
            ("P119", SharedCfgPad::Data(0)),
            ("P120", SharedCfgPad::Dout),
            ("P124", SharedCfgPad::Addr(0)),
            ("P125", SharedCfgPad::Addr(1)),
            ("P128", SharedCfgPad::Addr(2)),
            ("P129", SharedCfgPad::Addr(3)),
            ("P132", SharedCfgPad::Addr(15)),
            ("P133", SharedCfgPad::Addr(4)),
            ("P136", SharedCfgPad::Addr(14)),
            ("P137", SharedCfgPad::Addr(5)),
        ] {
            let BondPad::Io(io) = bond.pins[pin] else {
                unreachable!()
            };
            cfg_io.insert(fun, io);
        }
    }

    (bond, cfg_io)
}
