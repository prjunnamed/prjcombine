use std::collections::{BTreeMap, BTreeSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelInput, IntDb, TileWireCoord, WireKind},
    grid::{CellCoord, DieId, DieIdExt, EdgeIoCoord},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    xc4000::{
        self as defs, bslots, tslots, wires,
        xc4000::{bcls, tcls},
    },
};

use crate::extractor::{Extractor, NetBinding, PipMode};

pub fn make_chip(die: &Die) -> Chip {
    let mut kind = ChipKind::Xc4000;
    for pd in die.primdefs.values() {
        if pd.name == "iobh_bl" {
            kind = ChipKind::Xc4000H;
        } else if pd.name == "xdecoder2_b" {
            kind = ChipKind::Xc4000A;
        }
    }
    Chip {
        kind,
        columns: die.newcols.len() - 1,
        rows: die.newrows.len() - 1,
        cfg_io: Default::default(),
        is_buff_large: false,
        is_small: false,
        cols_bidi: Default::default(),
        rows_bidi: Default::default(),
        unbonded_io: BTreeSet::new(),
    }
}

pub fn dump_chip(die: &Die, noblock: &[String]) -> (Chip, IntDb, NamingDb) {
    let mut chip = make_chip(die);
    let mut intdb: IntDb = {
        let kind = chip.kind;
        bincode::decode_from_slice(
            match kind {
                ChipKind::Xc4000 => defs::xc4000::INIT,
                ChipKind::Xc4000A => defs::xc4000a::INIT,
                ChipKind::Xc4000H => defs::xc4000h::INIT,
                _ => unreachable!(),
            },
            bincode::config::standard(),
        )
        .unwrap()
        .0
    };
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.tile_namings.insert(name.clone(), TileNaming::default());
    }
    for (key, kind) in [("L", "left"), ("C", "center"), ("R", "rt"), ("CLK", "clkc")] {
        ndb.tile_widths
            .insert(key.into(), die.tiledefs[kind].matrix.dim().0);
    }
    for (key, kind) in [("B", "bot"), ("C", "center"), ("T", "top"), ("CLK", "clkc")] {
        ndb.tile_heights
            .insert(key.into(), die.tiledefs[kind].matrix.dim().1);
    }
    let edev = chip.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);

    let mut extractor = Extractor::new(die, &edev, &endev.ngrid);

    let die = DieId::from_idx(0);
    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let tcls = &intdb[tile.class];
        let ntile = &endev.ngrid.tiles[&tcrd];
        if !ntile.tie_names.is_empty() {
            if tcrd.slot == tslots::MAIN {
                let mut tie = extractor.grab_prim_ab(&ntile.tie_names[0], &ntile.tie_names[1]);
                let o = tie.get_pin("O");
                extractor.net_int(o, cell.wire(wires::TIE_0));
            } else {
                let mut prim = extractor.grab_prim_a(&ntile.tie_names[0]);
                let o = prim.get_pin("O");
                extractor.net_int_alt_tie(o, cell.delta(0, -1).wire(wires::TIE_0));
                let nbto = extractor
                    .net_by_cell_override
                    .entry(cell.delta(0, -1))
                    .or_default();
                nbto.insert(o, wires::TIE_0);
            }
        }
        let tile = &extractor.die.newtiles[&(endev.col_x[col].start, endev.row_y[row].start)];
        if tcrd.slot == tslots::MAIN {
            for &box_id in &tile.boxes {
                extractor.own_box(box_id, tcrd);
            }
        }
        for (slot, bel_info) in &tcls.bels {
            let bel = cell.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);

            match slot {
                bslots::INT
                | bslots::LLH
                | bslots::LLV
                | bslots::MISC_SW
                | bslots::MISC_SE
                | bslots::MISC_NW
                | bslots::MISC_NE
                | bslots::MISC_E => (),
                _ if slot_name.starts_with("PULLUP") => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    let wire_o = edev.get_bel_bidir(bel, bcls::PULLUP::O);
                    extractor.net_int(line, wire_o);
                    extractor.bel_pip(ntile.naming, slot, "O", pip);
                }
                bslots::BUFG_H | bslots::BUFG_V => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    extractor.pin_bel_input(&mut prim, bel, bcls::BUFG::I);
                    let o = prim.get_pin("O");
                    let wire_o = edev.get_bel_output(bel, bcls::BUFG::O)[0];
                    extractor.net_int(o, wire_o);
                }
                bslots::MD0
                | bslots::MD1
                | bslots::MD2
                | bslots::RDBK
                | bslots::BSCAN
                | bslots::STARTUP
                | bslots::READCLK
                | bslots::UPDATE
                | bslots::TDO => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
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
                bslots::OSC => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    extractor.pin_bel_output(&mut prim, bel, bcls::OSC::F8M);
                    for pin in ["F500K", "F16K", "F490", "F15"] {
                        let o = prim.get_pin(pin);
                        extractor.net_bel(o, bel, pin);
                        let mut o = extractor.consume_all_fwd(o, tcrd);
                        o.sort_by_key(|(_, pip)| pip.y);
                        extractor.net_int(o[0].0, edev.get_bel_output(bel, bcls::OSC::OUT0)[0]);
                        extractor.net_int(o[1].0, edev.get_bel_output(bel, bcls::OSC::OUT1)[0]);
                        extractor.bel_pip(ntile.naming, slot, format!("OUT0_{pin}"), o[0].1);
                        extractor.bel_pip(ntile.naming, slot, format!("OUT1_{pin}"), o[1].1);
                    }
                }
                _ if bslots::IO.contains(slot) => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_i(&bel_names[0]);
                    extractor.pin_bel_input(&mut prim, bel, bcls::IO::IK);
                    extractor.pin_bel_input(&mut prim, bel, bcls::IO::OK);
                    extractor.pin_bel_input(&mut prim, bel, bcls::IO::T);
                    extractor.pin_bel_output(&mut prim, bel, bcls::IO::I1);
                    extractor.pin_bel_output(&mut prim, bel, bcls::IO::I2);
                    // not quite true, but we'll fix it up.
                    let wire_o2 = edev.get_bel_input(bel, bcls::IO::O2);
                    extractor.net_int(prim.get_pin("O"), wire_o2.wire);
                    if bel_names.len() > 1 {
                        let mut prim = extractor.grab_prim_a(&bel_names[1]);
                        let wire_clkin = edev.get_bel_output(bel, bcls::IO::CLKIN)[0];
                        extractor.net_int(prim.get_pin("I"), wire_clkin);
                    }
                }
                _ if bslots::HIO.contains(slot) => {
                    let bel_names = &ntile.bels[slot];
                    let idx = bslots::HIO.index_of(slot).unwrap();
                    let mut prim = extractor.grab_prim_i(&bel_names[0]);
                    let ts = prim.get_pin("TS");
                    extractor.net_int(
                        ts,
                        tcrd.wire(
                            [
                                wires::IMUX_IO_OK[0],
                                wires::IMUX_IO_IK[0],
                                wires::IMUX_IO_IK[1],
                                wires::IMUX_IO_OK[1],
                            ][idx],
                        ),
                    );
                    let tp = prim.get_pin("TP");
                    extractor.net_bel(tp, bel, "TP");
                    let (line, pip) = extractor.consume_one_bwd(tp, tcrd);
                    extractor.net_int(line, tcrd.wire(wires::IMUX_IO_T[idx / 2]));
                    extractor.bel_pip(ntile.naming, slot, "TP", pip);

                    // O1/O2
                    let o = prim.get_pin("O");
                    let wire_o = edev.get_bel_input(bel, bcls::HIO::O).wire;
                    extractor.net_int(o, wire_o);
                    let mut o = Vec::from_iter(extractor.nets[o].pips_bwd.clone().into_iter());
                    assert_eq!(o.len(), 2);
                    if col == chip.col_w() {
                        o.sort_by_key(|(_, pip)| pip.0);
                    } else if col == chip.col_e() {
                        o.sort_by_key(|(_, pip)| !pip.0);
                    } else if row == chip.row_s() {
                        o.sort_by_key(|(_, pip)| pip.1);
                    } else if row == chip.row_n() {
                        o.sort_by_key(|(_, pip)| !pip.1);
                    }
                    extractor.net_int(o[0].0, tcrd.wire(wires::IMUX_IO_O1[idx / 2]));
                    extractor.net_int(
                        o[1].0,
                        tcrd.wire(if bel.col == chip.col_w() {
                            [wires::IMUX_CLB_G3_W, wires::IMUX_CLB_F3_W][idx / 2]
                        } else if bel.col == chip.col_e() {
                            [wires::IMUX_CLB_G1, wires::IMUX_CLB_F1][idx / 2]
                        } else if bel.row == chip.row_s() {
                            [wires::IMUX_CLB_F4, wires::IMUX_CLB_G4][idx / 2]
                        } else if bel.row == chip.row_n() {
                            [wires::IMUX_CLB_F2_N, wires::IMUX_CLB_G2_N][idx / 2]
                        } else {
                            unreachable!()
                        }),
                    );

                    // I1/I2
                    let net_i = prim.get_pin("I");
                    let wire_i = edev.get_bel_output(bel, bcls::HIO::I)[0];
                    extractor.net_int(net_i, wire_i);
                    let mut i = vec![];
                    for (&net, &pip) in &extractor.nets[net_i].pips_fwd {
                        i.push((net, pip));
                    }
                    assert_eq!(i.len(), 2);
                    if col == chip.col_w() {
                        i.sort_by_key(|(_, pip)| pip.0);
                    } else if col == chip.col_e() {
                        i.sort_by_key(|(_, pip)| !pip.0);
                    } else if row == chip.row_s() {
                        i.sort_by_key(|(_, pip)| pip.1);
                    } else if row == chip.row_n() {
                        i.sort_by_key(|(_, pip)| !pip.1);
                    }
                    let (out_i1, out_i2) = if col == chip.col_w() || col == chip.col_e() {
                        (wires::OUT_IO_WE_I1, wires::OUT_IO_WE_I2)
                    } else {
                        (wires::OUT_IO_SN_I1, wires::OUT_IO_SN_I2)
                    };
                    let ii = idx / 2;
                    extractor.net_int(i[0].0, cell.wire(out_i1[ii]));
                    extractor.net_int(i[1].0, cell.wire(out_i2[ii]));

                    if bel_names.len() > 1 {
                        let mut prim = extractor.grab_prim_a(&bel_names[1]);
                        let wire_clkin = edev.get_bel_output(bel, bcls::HIO::CLKIN)[0];
                        extractor.net_int(prim.get_pin("I"), wire_clkin);
                    }
                }
                _ if bslots::TBUF.contains(slot) => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    extractor.pin_bel_input(&mut prim, bel, bcls::TBUF::I);
                    extractor.pin_bel_input(&mut prim, bel, bcls::TBUF::T);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    let wire_o = edev.get_bel_bidir(bel, bcls::TBUF::O);
                    extractor.net_int(line, wire_o);
                    extractor.bel_pip(ntile.naming, slot, "O", pip);
                }
                _ if bslots::DEC.contains(slot) => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    let pins = if chip.kind == ChipKind::Xc4000A {
                        [bcls::DEC::O1, bcls::DEC::O2].as_slice()
                    } else {
                        [bcls::DEC::O1, bcls::DEC::O2, bcls::DEC::O3, bcls::DEC::O4].as_slice()
                    };
                    for &pid in pins {
                        let pin = intdb[bcls::DEC].bidirs.key(pid).0;
                        let o = prim.get_pin(pin);
                        extractor.net_bel(o, bel, pin);
                        let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                        extractor.bel_pip(ntile.naming, slot, pin, pip);
                        let wire = edev.get_bel_bidir(bel, pid);
                        extractor.net_int(line, wire);
                    }
                    let i = prim.get_pin("I");
                    let wire_i = edev.get_bel_input(bel, bcls::DEC::I).wire;
                    if slot == bslots::DEC[1] {
                        extractor.net_int(i, wire_i);
                    } else {
                        extractor.net_bel(i, bel, "I");
                        let (line, pip) = extractor.consume_one_bwd(i, tcrd);
                        extractor.net_int(line, wire_i);
                        extractor.bel_pip(ntile.naming, slot, "I", pip);
                    }
                }
                bslots::CLB => {
                    let bel_names = &ntile.bels[slot];
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    let BelInfo::Bel(bel_info) = bel_info else {
                        unreachable!();
                    };
                    for pid in bel_info.inputs.ids() {
                        extractor.pin_bel_input(&mut prim, bel, pid);
                    }
                    for pid in bel_info.outputs.ids() {
                        extractor.pin_bel_output(&mut prim, bel, pid);
                    }
                    let cin = prim.get_pin("CIN");
                    extractor.net_bel(cin, bel, "CIN");
                    let cout = prim.get_pin("COUT");
                    extractor.net_bel(cout, bel, "COUT");

                    if bel.col == chip.col_w() + 1
                        && (bel.row == chip.row_s() + 1 || bel.row == chip.row_n() - 1)
                    {
                        // CIN
                        let mut prim = extractor.grab_prim_a(&bel_names[2]);
                        let o = prim.get_pin("O");
                        extractor.net_bel(o, bel, "CIN_O");
                    }
                    if bel.col == chip.col_e() - 1
                        && (bel.row == chip.row_s() + 1 || bel.row == chip.row_n() - 1)
                    {
                        // COUT
                        let mut prim = extractor.grab_prim_a(&bel_names[2]);
                        let i = prim.get_pin("I");
                        extractor.net_bel(i, bel, "COUT_I");
                    }
                }
                _ => panic!("umm bel {slot_name}?"),
            }
        }
    }
    extractor.grab_prim_a("_cfg4000_");

    // long verticals + GCLK
    for col in edev.cols(die) {
        let mut queue = vec![];
        for row in [chip.row_mid() - 1, chip.row_mid()] {
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
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        wires::LONG_IO_V[0],
                        wires::LONG_IO_V[1],
                        wires::GCLK[0],
                        wires::GCLK[1],
                        wires::GCLK[2],
                        wires::GCLK[3],
                    ][..]
                } else {
                    &[
                        wires::LONG_IO_V[0],
                        wires::LONG_IO_V[1],
                        wires::LONG_IO_V[2],
                        wires::LONG_IO_V[3],
                        wires::GCLK[0],
                        wires::GCLK[1],
                        wires::GCLK[2],
                        wires::GCLK[3],
                    ][..]
                }
            } else if col == chip.col_e() {
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        wires::LONG_V[0],
                        wires::LONG_V[1],
                        wires::LONG_V[2],
                        wires::LONG_V[3],
                        wires::GCLK[0],
                        wires::GCLK[1],
                        wires::GCLK[2],
                        wires::GCLK[3],
                        wires::LONG_IO_V[0],
                        wires::LONG_IO_V[1],
                    ][..]
                } else {
                    &[
                        wires::LONG_V[0],
                        wires::LONG_V[1],
                        wires::LONG_V[2],
                        wires::LONG_V[3],
                        wires::LONG_V[4],
                        wires::LONG_V[5],
                        wires::GCLK[0],
                        wires::GCLK[1],
                        wires::GCLK[2],
                        wires::GCLK[3],
                        wires::LONG_IO_V[0],
                        wires::LONG_IO_V[1],
                        wires::LONG_IO_V[2],
                        wires::LONG_IO_V[3],
                    ][..]
                }
            } else {
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        wires::LONG_V[0],
                        wires::LONG_V[1],
                        wires::LONG_V[2],
                        wires::LONG_V[3],
                        wires::GCLK[0],
                        wires::GCLK[1],
                        wires::GCLK[2],
                        wires::GCLK[3],
                    ][..]
                } else {
                    &[
                        wires::LONG_V[0],
                        wires::LONG_V[1],
                        wires::LONG_V[2],
                        wires::LONG_V[3],
                        wires::LONG_V[4],
                        wires::LONG_V[5],
                        wires::GCLK[0],
                        wires::GCLK[1],
                        wires::GCLK[2],
                        wires::GCLK[3],
                    ][..]
                }
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, die.cell(col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }
    // long horizontals
    for row in edev.rows(die) {
        let mut queue = vec![];
        for col in [chip.col_mid() - 1, chip.col_mid()] {
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
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        wires::LONG_IO_H[0],
                        wires::LONG_IO_H[1],
                        wires::LONG_H[3],
                        wires::LONG_H[4],
                    ][..]
                } else {
                    &[
                        wires::LONG_IO_H[0],
                        wires::LONG_IO_H[1],
                        wires::LONG_IO_H[2],
                        wires::LONG_IO_H[3],
                        wires::LONG_H[3],
                        wires::LONG_H[4],
                        wires::LONG_H[5],
                    ][..]
                }
            } else if row == chip.row_n() {
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        wires::LONG_H[0],
                        wires::LONG_H[2],
                        wires::LONG_IO_H[0],
                        wires::LONG_IO_H[1],
                    ][..]
                } else {
                    &[
                        wires::LONG_H[0],
                        wires::LONG_H[1],
                        wires::LONG_H[2],
                        wires::LONG_IO_H[0],
                        wires::LONG_IO_H[1],
                        wires::LONG_IO_H[2],
                        wires::LONG_IO_H[3],
                    ][..]
                }
            } else {
                if chip.kind == ChipKind::Xc4000A {
                    &[wires::LONG_H[0], wires::LONG_H[4]][..]
                } else {
                    &[
                        wires::LONG_H[0],
                        wires::LONG_H[1],
                        wires::LONG_H[4],
                        wires::LONG_H[5],
                    ][..]
                }
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, die.cell(col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // boxes — pin single and double wires
    for col in edev.cols(die) {
        if col == chip.col_w() {
            continue;
        }
        for row in edev.rows(die) {
            if row == chip.row_n() {
                continue;
            }
            let tile = &extractor.die.newtiles[&(endev.col_x[col].start, endev.row_y[row].start)];
            assert_eq!(tile.boxes.len(), 1);
            let num_singles = if chip.kind == ChipKind::Xc4000A { 4 } else { 8 };
            let num_sd = num_singles + 2;
            for i in 0..num_singles {
                let net_n = extractor.box_net(tile.boxes[0], 1 + i);
                let net_e = extractor.box_net(tile.boxes[0], num_sd * 2 - 2 - i);
                let net_s = extractor.box_net(tile.boxes[0], num_sd * 3 - 2 - i);
                let net_w = extractor.box_net(tile.boxes[0], num_sd * 3 + 1 + i);
                for (net, wire) in [
                    (net_n, wires::SINGLE_V_S[i]),
                    (net_e, wires::SINGLE_H[i]),
                    (net_s, wires::SINGLE_V[i]),
                    (net_w, wires::SINGLE_H_E[i]),
                ] {
                    extractor.net_int(net, die.cell(col, row).wire(wire));
                }
            }
            for (idx, wire) in [
                (0, wires::DOUBLE_V2[0]),
                (num_sd - 1, wires::DOUBLE_V2[1]),
                (num_sd, wires::DOUBLE_H0[1]),
                (2 * num_sd - 1, wires::DOUBLE_H0[0]),
                (2 * num_sd, wires::DOUBLE_V0[1]),
                (3 * num_sd - 1, wires::DOUBLE_V0[0]),
                (3 * num_sd, wires::DOUBLE_H2[0]),
                (4 * num_sd - 1, wires::DOUBLE_H2[1]),
            ] {
                let net = extractor.box_net(tile.boxes[0], idx);
                extractor.net_int(net, die.cell(col, row).wire(wire));
            }
        }
    }

    // io doubles
    let mut queue = vec![];
    for col in edev.cols(die) {
        if col == chip.col_w() {
            continue;
        }
        let x = endev.col_x[col].start;
        {
            let row = chip.row_s();
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
            let wires = if chip.kind == ChipKind::Xc4000A {
                &[
                    wires::DOUBLE_IO_S2[0],
                    wires::DOUBLE_IO_S1[0],
                    wires::DOUBLE_IO_S2[1],
                    wires::DOUBLE_IO_S1[1],
                ][..]
            } else {
                &[
                    wires::DOUBLE_IO_S2[0],
                    wires::DOUBLE_IO_S1[0],
                    wires::DOUBLE_IO_S2[1],
                    wires::DOUBLE_IO_S1[1],
                    wires::DOUBLE_IO_S2[2],
                    wires::DOUBLE_IO_S1[2],
                    wires::DOUBLE_IO_S2[3],
                    wires::DOUBLE_IO_S1[3],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, die.cell(col, row).wire(wire)));
            }
        }
        {
            let row = chip.row_n();
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
            let wires = if chip.kind == ChipKind::Xc4000A {
                &[
                    wires::DOUBLE_IO_N0[0],
                    wires::DOUBLE_IO_N1[0],
                    wires::DOUBLE_IO_N0[1],
                    wires::DOUBLE_IO_N1[1],
                ][..]
            } else {
                &[
                    wires::DOUBLE_IO_N0[0],
                    wires::DOUBLE_IO_N1[0],
                    wires::DOUBLE_IO_N0[1],
                    wires::DOUBLE_IO_N1[1],
                    wires::DOUBLE_IO_N0[2],
                    wires::DOUBLE_IO_N1[2],
                    wires::DOUBLE_IO_N0[3],
                    wires::DOUBLE_IO_N1[3],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, die.cell(col, row).wire(wire)));
            }
        }
    }
    for row in edev.rows(die) {
        if row == chip.row_s() {
            continue;
        }
        let y = endev.row_y[row].start;
        {
            let col = chip.col_w();
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
            let wires = if chip.kind == ChipKind::Xc4000A {
                &[
                    wires::DOUBLE_IO_W0[0],
                    wires::DOUBLE_IO_W1[0],
                    wires::DOUBLE_IO_W0[1],
                    wires::DOUBLE_IO_W1[1],
                ][..]
            } else {
                &[
                    wires::DOUBLE_IO_W0[0],
                    wires::DOUBLE_IO_W1[0],
                    wires::DOUBLE_IO_W0[1],
                    wires::DOUBLE_IO_W1[1],
                    wires::DOUBLE_IO_W0[2],
                    wires::DOUBLE_IO_W1[2],
                    wires::DOUBLE_IO_W0[3],
                    wires::DOUBLE_IO_W1[3],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, die.cell(col, row).wire(wire)));
            }
        }
        {
            let col = chip.col_e();
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
            let wires = if chip.kind == ChipKind::Xc4000A {
                &[
                    wires::DOUBLE_IO_E2[0],
                    wires::DOUBLE_IO_E1[0],
                    wires::DOUBLE_IO_E2[1],
                    wires::DOUBLE_IO_E1[1],
                ][..]
            } else {
                &[
                    wires::DOUBLE_IO_E2[0],
                    wires::DOUBLE_IO_E1[0],
                    wires::DOUBLE_IO_E2[1],
                    wires::DOUBLE_IO_E1[1],
                    wires::DOUBLE_IO_E2[2],
                    wires::DOUBLE_IO_E1[2],
                    wires::DOUBLE_IO_E2[3],
                    wires::DOUBLE_IO_E1[3],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, die.cell(col, row).wire(wire)));
            }
        }
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    // DBUF
    for col in edev.cols(die) {
        if col == chip.col_w() {
            continue;
        }
        for (row, w_h0, w_h1) in [
            (
                chip.row_s(),
                if col == chip.col_e() {
                    wires::DOUBLE_IO_E1[0]
                } else {
                    wires::DOUBLE_IO_S0[0]
                },
                wires::DOUBLE_IO_S2[0],
            ),
            (
                chip.row_n(),
                if col == chip.col_e() {
                    wires::DOUBLE_IO_E2[0]
                } else {
                    wires::DOUBLE_IO_N2[0]
                },
                wires::DOUBLE_IO_N0[0],
            ),
        ] {
            for (w_anchor, w_dbuf) in [(w_h0, wires::DBUF_IO_H[0]), (w_h1, wires::DBUF_IO_H[1])] {
                let rw_anchor = edev
                    .resolve_wire(die.cell(col, row).wire(w_anchor))
                    .unwrap();
                let net = extractor.int_nets[&rw_anchor];
                let mut nets = vec![];
                for (&net_t, &crd) in &extractor.nets[net].pips_fwd {
                    if !endev.col_x[col].contains(&crd.0) {
                        continue;
                    }
                    if !endev.row_y[row].contains(&crd.1) {
                        continue;
                    }
                    if extractor.nets[net_t].binding != NetBinding::None {
                        continue;
                    }
                    nets.push(net_t);
                }
                assert_eq!(nets.len(), 1);
                let net = nets[0];
                extractor.net_int(net, die.cell(col, row).wire(w_dbuf));
            }
        }
    }
    for row in edev.rows(die) {
        if row == chip.row_n() {
            continue;
        }
        for (col, w_v0, w_v1) in [
            (
                chip.col_w(),
                if row == chip.row_s() {
                    wires::DOUBLE_IO_S0[0]
                } else {
                    wires::DOUBLE_IO_W0[0]
                },
                wires::DOUBLE_IO_W2[0],
            ),
            (
                chip.col_e(),
                if row == chip.row_s() {
                    wires::DOUBLE_IO_S1[0]
                } else {
                    wires::DOUBLE_IO_E2[0]
                },
                wires::DOUBLE_IO_E0[0],
            ),
        ] {
            for (w_anchor, w_dbuf) in [(w_v0, wires::DBUF_IO_V[0]), (w_v1, wires::DBUF_IO_V[1])] {
                let rw_anchor = edev
                    .resolve_wire(die.cell(col, row).wire(w_anchor))
                    .unwrap();
                let net = extractor.int_nets[&rw_anchor];
                let mut nets = vec![];
                for (&net_t, &crd) in &extractor.nets[net].pips_fwd {
                    if !endev.col_x[col].contains(&crd.0) {
                        continue;
                    }
                    if !endev.row_y[row].contains(&crd.1) {
                        continue;
                    }
                    if extractor.nets[net_t].binding != NetBinding::None {
                        continue;
                    }
                    nets.push(net_t);
                }
                assert_eq!(nets.len(), 1);
                let net = nets[0];
                extractor.net_int(net, die.cell(col, row).wire(w_dbuf));
            }
        }
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
                extractor.own_mux(rw, cell.tile(tslots::MAIN));
            }
        }
    }

    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let ntile = &endev.ngrid.tiles[&tcrd];
        let tcls = &intdb[tile.class];
        for (slot, bel_info) in &tcls.bels {
            let bel = cell.bel(slot);
            match slot {
                _ if bslots::TBUF.contains(slot) => {
                    let net_i = extractor.get_bel_input_net(bel, bcls::TBUF::I);
                    let net_o = extractor.get_bel_bidir_net(bel, bcls::TBUF::O);
                    let src_nets = Vec::from_iter(extractor.nets[net_i].pips_bwd.keys().copied());
                    for net in src_nets {
                        extractor.mark_tbuf_pseudo(net_o, net);
                    }
                }
                _ if bslots::IO.contains(slot) => {
                    let BelInfo::Bel(bel_info) = bel_info else {
                        unreachable!()
                    };
                    let net_o2 = extractor.get_bel_input_net(bel, bcls::IO::O2);
                    let BelInput::Fixed(o1) = bel_info.inputs[bcls::IO::O1] else {
                        unreachable!()
                    };
                    let mut nets = vec![];
                    for net in extractor.nets[net_o2].pips_bwd.keys().copied() {
                        let NetBinding::Int(rw) = extractor.nets[net].binding else {
                            continue;
                        };
                        let wkey = intdb.wires.key(rw.slot);
                        let is_o2 = if col == chip.col_w() || col == chip.col_e() {
                            wkey.starts_with("SINGLE_V")
                                || wkey.starts_with("DOUBLE_V")
                                || wkey.starts_with("LONG_V")
                                || wkey.starts_with("GCLK")
                        } else {
                            wkey.starts_with("SINGLE_H")
                                || wkey.starts_with("DOUBLE_H")
                                || wkey.starts_with("LONG_H")
                        };
                        if !is_o2 {
                            nets.push(net);
                        }
                    }
                    for net in nets {
                        extractor.force_int_pip_dst(net_o2, net, tcrd, o1.tw);
                    }
                }
                bslots::CLB => {
                    let net_cin = extractor.get_bel_net(bel, "CIN");
                    let net_cout_b = if row != chip.row_s() + 1 {
                        extractor.get_bel_net(cell.delta(0, -1).bel(bslots::CLB), "COUT")
                    } else if col != chip.col_w() + 1 {
                        extractor.get_bel_net(cell.delta(-1, 0).bel(bslots::CLB), "COUT")
                    } else {
                        extractor.get_bel_net(bel, "CIN_O")
                    };
                    let crd = extractor.use_pip(net_cin, net_cout_b);
                    let pip = extractor.xlat_pip_loc(tcrd, crd);
                    extractor.bel_pip(ntile.naming, slot, "CIN_S", pip);
                    let net_cout_t = if row != chip.row_n() - 1 {
                        extractor.get_bel_net(cell.delta(0, 1).bel(bslots::CLB), "COUT")
                    } else if col != chip.col_w() + 1 {
                        extractor.get_bel_net(cell.delta(-1, 0).bel(bslots::CLB), "COUT")
                    } else {
                        extractor.get_bel_net(bel, "CIN_O")
                    };
                    let crd = extractor.use_pip(net_cin, net_cout_t);
                    let pip = extractor.xlat_pip_loc(tcrd, crd);
                    extractor.bel_pip(ntile.naming, slot, "CIN_N", pip);
                    if bel.col == chip.col_e() - 1
                        && (bel.row == chip.row_s() + 1 || bel.row == chip.row_n() - 1)
                    {
                        let net_cin = extractor.get_bel_net(bel, "COUT_I");
                        let net_cout = extractor.get_bel_net(bel, "COUT");
                        let crd = extractor.use_pip(net_cin, net_cout);
                        let pip = extractor.xlat_pip_loc(tcrd, crd);
                        extractor.bel_pip(ntile.naming, slot, "COUT_I", pip);
                    }
                }
                _ => (),
            }
        }
    }

    if chip.kind == ChipKind::Xc4000H {
        for tcid in [
            tcls::IO_W0,
            tcls::IO_W1,
            tcls::IO_W0_N,
            tcls::IO_W1_S,
            tcls::IO_E0,
            tcls::IO_E1,
            tcls::IO_E0_N,
            tcls::IO_E1_S,
            tcls::IO_S0,
            tcls::IO_S1,
            tcls::IO_S0_E,
            tcls::IO_S1_W,
            tcls::IO_N0,
            tcls::IO_N1,
            tcls::IO_N0_E,
            tcls::IO_N1_W,
        ] {
            for idx in 0..4 {
                let t = TileWireCoord::new_idx(0, wires::IMUX_HIO_T[idx]);
                let tp = TileWireCoord::new_idx(0, wires::IMUX_IO_T[idx / 2]);
                let ts = TileWireCoord::new_idx(
                    0,
                    [
                        wires::IMUX_IO_OK[0],
                        wires::IMUX_IO_IK[0],
                        wires::IMUX_IO_IK[1],
                        wires::IMUX_IO_OK[1],
                    ][idx],
                );
                extractor.inject_pip(tcid, t, tp.pos());
                extractor.inject_pip(tcid, t, ts.pos());
            }
        }
    }
    for tcid in [
        tcls::CLB,
        tcls::CLB_W,
        tcls::CLB_E,
        tcls::CLB_S,
        tcls::CLB_SW,
        tcls::CLB_SE,
        tcls::CLB_N,
        tcls::CLB_NW,
        tcls::CLB_NE,
    ] {
        extractor.inject_pip(
            tcid,
            TileWireCoord::new_idx(0, wires::IMUX_CLB_F4),
            TileWireCoord::new_idx(0, wires::SPECIAL_CLB_CIN).pos(),
        );
    }
    for tcid in [
        tcls::CLB,
        tcls::CLB_E,
        tcls::CLB_S,
        tcls::CLB_SE,
        tcls::CLB_N,
        tcls::CLB_NE,
        tcls::IO_E0,
        tcls::IO_E0_N,
        tcls::IO_E1,
        tcls::IO_E1_S,
    ] {
        extractor.inject_pip(
            tcid,
            TileWireCoord::new_idx(0, wires::IMUX_CLB_G3),
            TileWireCoord::new_idx(0, wires::SPECIAL_CLB_CIN).pos(),
        );
    }
    for tcid in [
        tcls::CLB,
        tcls::CLB_W,
        tcls::CLB_E,
        tcls::CLB_S,
        tcls::CLB_SW,
        tcls::CLB_SE,
        tcls::IO_S0,
        tcls::IO_S0_E,
        tcls::IO_S1,
        tcls::IO_S1_W,
    ] {
        extractor.inject_pip(
            tcid,
            TileWireCoord::new_idx(0, wires::IMUX_CLB_G2),
            TileWireCoord::new_idx(0, wires::SPECIAL_CLB_COUT0).pos(),
        );
    }
    for (tcid, _, tcls) in &intdb.tile_classes {
        for idx in 0..2 {
            if tcls.bels.contains_id(bslots::TBUF[idx]) {
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_I[idx]),
                    TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                );
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[idx]),
                    TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                );
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_TBUF_T[idx]),
                    TileWireCoord::new_idx(0, wires::TIE_1).pos(),
                );
            }
        }
        if tcls.bels.contains_id(bslots::IO[0]) || tcls.bels.contains_id(bslots::HIO[0]) {
            for idx in 0..2 {
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_IO_O1[idx]),
                    TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                );
                extractor.inject_pip(
                    tcid,
                    TileWireCoord::new_idx(0, wires::IMUX_IO_T[idx]),
                    TileWireCoord::new_idx(0, wires::TIE_0).pos(),
                );
            }
        }
    }

    for i in 0..4 {
        if i >= 2 && chip.kind == ChipKind::Xc4000A {
            continue;
        }
        let wire = wires::DOUBLE_IO_W1[i];
        let rw = edev
            .resolve_wire(die.cell(chip.col_w(), chip.row_s()).wire(wire))
            .unwrap();
        let net = extractor.int_nets[&rw];
        let nbto = extractor
            .net_by_cell_override
            .entry(die.cell(chip.col_w(), chip.row_s()))
            .or_default();
        nbto.insert(net, wire);
    }

    let io_lookup: BTreeMap<_, _> = endev
        .chip
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io).to_string(), io))
        .collect();

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb, |db, tslot, wt, wf| {
        if tslot != tslots::MAIN {
            if wires::GCLK.contains(wt.wire) {
                PipMode::Mux
            } else {
                PipMode::Pass
            }
        } else {
            let wtn = db.wires.key(wt.wire);
            let wfn = db.wires.key(wf.wire);
            if wtn.starts_with("IMUX")
                || wtn.starts_with("LONG_IO")
                || wtn.starts_with("DBUF_IO")
                || wtn.starts_with("OUT")
            {
                PipMode::Mux
            } else if wtn.starts_with("LONG") && wfn.starts_with("SINGLE") {
                PipMode::Buf
            } else if wtn.starts_with("LONG") {
                PipMode::Mux
            } else {
                PipMode::Pass
            }
        }
    });

    for pad in noblock {
        let io = io_lookup[pad];
        chip.unbonded_io.insert(io);
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
        bond.pins.insert(pin.into(), BondPad::Io(io));
    }
    let (m1, m0, m2, done, prog, cclk, tdo, gnd, vcc) = match name {
        "pc84" => (
            "P30",
            "P32",
            "P34",
            "P53",
            "P55",
            "P73",
            "P75",
            &["P1", "P12", "P21", "P31", "P43", "P52", "P64", "P76"][..],
            &["P2", "P11", "P22", "P33", "P42", "P54", "P63", "P74"][..],
        ),

        "vq100" => (
            "P22",
            "P24",
            "P26",
            "P50",
            "P52",
            "P74",
            "P76",
            &["P1", "P11", "P23", "P38", "P49", "P64", "P77", "P88"][..],
            &["P12", "P25", "P37", "P51", "P63", "P75", "P89", "P100"][..],
        ),
        "tq144" => (
            "P34",
            "P36",
            "P38",
            "P72",
            "P74",
            "P107",
            "P109",
            &[
                "P1", "P8", "P17", "P27", "P35", "P45", "P55", "P64", "P71", "P73", "P81", "P91",
                "P100", "P110", "P118", "P127", "P137",
            ][..],
            &["P18", "P37", "P54", "P90", "P108", "P128", "P144"][..],
        ),

        "pq100" => (
            "P25",
            "P27",
            "P29",
            "P53",
            "P55",
            "P77",
            "P79",
            &["P4", "P14", "P26", "P41", "P52", "P67", "P80", "P91"][..],
            &["P3", "P15", "P28", "P40", "P54", "P66", "P78", "P92"][..],
        ),
        "pq160" => (
            "P38",
            "P40",
            "P42",
            "P80",
            "P82",
            "P119",
            "P121",
            &[
                "P1", "P10", "P19", "P29", "P39", "P51", "P61", "P70", "P79", "P91", "P101",
                "P110", "P122", "P131", "P141", "P151",
            ][..],
            &["P20", "P41", "P60", "P81", "P100", "P120", "P142", "P160"][..],
        ),
        "pq208" | "mq208" => (
            "P48",
            "P50",
            "P56",
            "P103",
            "P108",
            "P153",
            "P159",
            &[
                "P2", "P14", "P25", "P37", "P49", "P67", "P79", "P90", "P101", "P119", "P131",
                "P142", "P160", "P171", "P182", "P194",
            ][..],
            &["P26", "P55", "P78", "P106", "P130", "P154", "P183", "P205"][..],
        ),
        "pq240" | "mq240" => (
            "P58",
            "P60",
            "P62",
            "P120",
            "P122",
            "P179",
            "P181",
            &[
                "P1", "P14", "P29", "P45", "P59", "P75", "P91", "P106", "P119", "P135", "P151",
                "P166", "P182", "P196", "P211", "P227",
            ][..],
            &[
                "P19", "P30", "P40", "P61", "P80", "P90", "P101", "P121", "P140", "P150", "P161",
                "P180", "P201", "P212", "P222", "P240",
            ][..],
        ),

        "cb100" => (
            "P22",
            "P24",
            "P26",
            "P50",
            "P52",
            "P74",
            "P76",
            &["P1", "P11", "P23", "P38", "P49", "P64", "P77", "P88"][..],
            &["P12", "P25", "P37", "P51", "P63", "P75", "P89", "P100"][..],
        ),
        "cb164" => (
            "P39",
            "P41",
            "P43",
            "P82",
            "P84",
            "P122",
            "P124",
            &[
                "P1", "P10", "P19", "P30", "P40", "P53", "P63", "P72", "P81", "P94", "P104",
                "P113", "P125", "P135", "P144", "P154",
            ][..],
            &["P20", "P42", "P62", "P83", "P103", "P123", "P145", "P164"][..],
        ),
        "cb196" => (
            "P47",
            "P49",
            "P51",
            "P98",
            "P100",
            "P146",
            "P148",
            &[
                "P1", "P13", "P24", "P36", "P48", "P63", "P75", "P86", "P97", "P112", "P124",
                "P135", "P149", "P161", "P172", "P184",
            ][..],
            &["P25", "P50", "P74", "P99", "P123", "P147", "P173", "P196"][..],
        ),
        "cb228" => (
            "P55",
            "P57",
            "P59",
            "P114",
            "P116",
            "P170",
            "P172",
            &[
                "P1", "P14", "P27", "P42", "P56", "P72", "P86", "P100", "P113", "P129", "P143",
                "P157", "P173", "P186", "P200", "P215",
            ][..],
            &[
                "P28", "P37", "P58", "P85", "P95", "P115", "P142", "P152", "P171", "P191", "P201",
                "P210", "P228",
            ][..],
        ),

        "pg120" => (
            "B11",
            "C11",
            "B12",
            "L11",
            "M12",
            "L4",
            "M2",
            &["C4", "B7", "C10", "G11", "K11", "L7", "K3", "G2"][..],
            &["G3", "C3", "C7", "D11", "G12", "L10", "M7", "L3"][..],
        ),
        "pg156" => (
            "A15",
            "A16",
            "B15",
            "R15",
            "R14",
            "R2",
            "T1",
            &[
                "F3", "C4", "C6", "C8", "C11", "C13", "F14", "J14", "L14", "P14", "P11", "P8",
                "P6", "N3", "L3", "H2",
            ][..],
            &["H3", "C3", "B8", "C14", "H14", "P13", "R8", "P3"][..],
        ),
        "pg191" => (
            "C15",
            "A18",
            "C16",
            "U17",
            "V18",
            "V1",
            "U2",
            &[
                "G3", "D4", "C7", "D9", "C12", "D15", "G16", "K15", "M16", "R16", "T12", "R9",
                "T7", "R3", "M3", "K4",
            ][..],
            &["J4", "D3", "D10", "D16", "J15", "R15", "R10", "R4"][..],
        ),
        "pg223" => (
            "C15",
            "A18",
            "C16",
            "U17",
            "V18",
            "V1",
            "U2",
            &[
                "G3", "D4", "C7", "D9", "C12", "D15", "G16", "K15", "M16", "R16", "T12", "R9",
                "T7", "R3", "M3", "K4",
            ][..],
            &["J4", "D3", "D10", "D16", "J15", "R15", "R10", "R4"][..],
        ),

        "bg225" => (
            "N3",
            "P2",
            "M4",
            "P14",
            "M12",
            "C13",
            "A15",
            &[
                "A1", "D12", "G7", "G9", "H6", "H8", "H10", "J8", "K8", "A8", "F8", "G8", "H2",
                "H7", "H9", "J7", "J9", "M8",
            ][..],
            &["B2", "D8", "H15", "R8", "B14", "R1", "H1", "R15"][..],
        ),

        _ => panic!("ummm {name}?"),
    };
    assert_eq!(bond.pins.insert(m0.into(), BondPad::Cfg(CfgPad::M0)), None);
    assert_eq!(bond.pins.insert(m1.into(), BondPad::Cfg(CfgPad::M1)), None);
    assert_eq!(bond.pins.insert(m2.into(), BondPad::Cfg(CfgPad::M2)), None);
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
    assert_eq!(
        bond.pins.insert(tdo.into(), BondPad::Cfg(CfgPad::Tdo)),
        None
    );
    for &pin in gnd {
        assert_eq!(bond.pins.insert(pin.into(), BondPad::Gnd), None);
    }
    for &pin in vcc {
        assert_eq!(bond.pins.insert(pin.into(), BondPad::Vcc), None);
    }

    let len1d = match name {
        "pc84" => Some(84),
        "vq100" => Some(100),
        "tq144" => Some(144),
        "pq100" => Some(100),
        "pq160" => Some(160),
        "pq208" | "mq208" => Some(208),
        "pq240" | "mq240" => Some(240),
        "cb100" => Some(100),
        "cb164" => Some(164),
        "cb196" => Some(196),
        "cb228" => Some(228),
        _ => None,
    };
    if let Some(len1d) = len1d {
        for i in 1..=len1d {
            bond.pins.entry(format!("P{i}")).or_insert(BondPad::Nc);
        }
        assert_eq!(bond.pins.len(), len1d);
    }
    match name {
        "bg225" => {
            for a in [
                "A", "B", "C", "D", "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R",
            ] {
                for i in 1..=15 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 225);
        }
        "pg120" => {
            for a in ["A", "B", "C", "L", "M", "N"] {
                for i in 1..=13 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K"] {
                for i in (1..=3).chain(11..=13) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 120);
        }
        "pg156" => {
            for a in ["A", "B", "C", "P", "R", "T"] {
                for i in 1..=16 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L", "M", "N"] {
                for i in (1..=3).chain(14..=16) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 156);
        }
        "pg191" => {
            for i in 2..=18 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPad::Nc);
            }
            for a in ["B", "C", "T", "U", "V"] {
                for i in 1..=18 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R"] {
                for i in (1..=3).chain(16..=18) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["D", "R"] {
                for i in [4, 9, 10, 15] {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["J", "K"] {
                for i in [4, 15] {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 191);
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

    let mut pkg_cfg_io = vec![];
    match name {
        "pc84" => {
            pkg_cfg_io.extend([
                ("P3", SharedCfgPad::Addr(8)),
                ("P4", SharedCfgPad::Addr(9)),
                ("P5", SharedCfgPad::Addr(10)),
                ("P6", SharedCfgPad::Addr(11)),
                ("P7", SharedCfgPad::Addr(12)),
                ("P8", SharedCfgPad::Addr(13)),
                ("P9", SharedCfgPad::Addr(14)),
                ("P10", SharedCfgPad::Addr(15)),
                ("P13", SharedCfgPad::Addr(16)),
                ("P14", SharedCfgPad::Addr(17)),
                ("P15", SharedCfgPad::Tdi),
                ("P16", SharedCfgPad::Tck),
                ("P17", SharedCfgPad::Tms),
                ("P36", SharedCfgPad::Hdc),
                ("P37", SharedCfgPad::Ldc),
                ("P41", SharedCfgPad::InitB),
                ("P56", SharedCfgPad::Data(7)),
                ("P58", SharedCfgPad::Data(6)),
                ("P59", SharedCfgPad::Data(5)),
                ("P60", SharedCfgPad::Cs0B),
                ("P61", SharedCfgPad::Data(4)),
                ("P65", SharedCfgPad::Data(3)),
                ("P66", SharedCfgPad::Cs1B),
                ("P67", SharedCfgPad::Data(2)),
                ("P69", SharedCfgPad::Data(1)),
                ("P70", SharedCfgPad::RclkB),
                ("P71", SharedCfgPad::Data(0)),
                ("P72", SharedCfgPad::Dout),
                ("P77", SharedCfgPad::Addr(0)),
                ("P78", SharedCfgPad::Addr(1)),
                ("P79", SharedCfgPad::Addr(2)),
                ("P80", SharedCfgPad::Addr(3)),
                ("P81", SharedCfgPad::Addr(4)),
                ("P82", SharedCfgPad::Addr(5)),
                ("P83", SharedCfgPad::Addr(6)),
                ("P84", SharedCfgPad::Addr(7)),
            ]);
        }
        "pq160" | "mq160" => {
            pkg_cfg_io.extend([
                ("P143", SharedCfgPad::Addr(8)),
                ("P144", SharedCfgPad::Addr(9)),
                ("P147", SharedCfgPad::Addr(10)),
                ("P148", SharedCfgPad::Addr(11)),
                ("P154", SharedCfgPad::Addr(12)),
                ("P155", SharedCfgPad::Addr(13)),
                ("P158", SharedCfgPad::Addr(14)),
                ("P159", SharedCfgPad::Addr(15)),
                ("P2", SharedCfgPad::Addr(16)),
                ("P3", SharedCfgPad::Addr(17)),
                ("P6", SharedCfgPad::Tdi),
                ("P7", SharedCfgPad::Tck),
                ("P13", SharedCfgPad::Tms),
                ("P44", SharedCfgPad::Hdc),
                ("P48", SharedCfgPad::Ldc),
                ("P59", SharedCfgPad::InitB),
                ("P83", SharedCfgPad::Data(7)),
                ("P87", SharedCfgPad::Data(6)),
                ("P94", SharedCfgPad::Data(5)),
                ("P95", SharedCfgPad::Cs0B),
                ("P98", SharedCfgPad::Data(4)),
                ("P102", SharedCfgPad::Data(3)),
                ("P103", SharedCfgPad::Cs1B),
                ("P106", SharedCfgPad::Data(2)),
                ("P113", SharedCfgPad::Data(1)),
                ("P114", SharedCfgPad::RclkB),
                ("P117", SharedCfgPad::Data(0)),
                ("P118", SharedCfgPad::Dout),
                ("P123", SharedCfgPad::Addr(0)),
                ("P124", SharedCfgPad::Addr(1)),
                ("P127", SharedCfgPad::Addr(2)),
                ("P128", SharedCfgPad::Addr(3)),
                ("P134", SharedCfgPad::Addr(4)),
                ("P135", SharedCfgPad::Addr(5)),
                ("P139", SharedCfgPad::Addr(6)),
                ("P140", SharedCfgPad::Addr(7)),
            ]);
        }
        "pq208" | "mq208" => {
            pkg_cfg_io.extend([
                ("P184", SharedCfgPad::Addr(8)),
                ("P185", SharedCfgPad::Addr(9)),
                ("P190", SharedCfgPad::Addr(10)),
                ("P191", SharedCfgPad::Addr(11)),
                ("P199", SharedCfgPad::Addr(12)),
                ("P200", SharedCfgPad::Addr(13)),
                ("P203", SharedCfgPad::Addr(14)),
                ("P204", SharedCfgPad::Addr(15)),
                ("P4", SharedCfgPad::Addr(16)),
                ("P5", SharedCfgPad::Addr(17)),
                ("P8", SharedCfgPad::Tdi),
                ("P9", SharedCfgPad::Tck),
                ("P17", SharedCfgPad::Tms),
                ("P58", SharedCfgPad::Hdc),
                ("P62", SharedCfgPad::Ldc),
                ("P77", SharedCfgPad::InitB),
                ("P109", SharedCfgPad::Data(7)),
                ("P113", SharedCfgPad::Data(6)),
                ("P122", SharedCfgPad::Data(5)),
                ("P123", SharedCfgPad::Cs0B),
                ("P128", SharedCfgPad::Data(4)),
                ("P132", SharedCfgPad::Data(3)),
                ("P133", SharedCfgPad::Cs1B),
                ("P138", SharedCfgPad::Data(2)),
                ("P147", SharedCfgPad::Data(1)),
                ("P148", SharedCfgPad::RclkB),
                ("P151", SharedCfgPad::Data(0)),
                ("P152", SharedCfgPad::Dout),
                ("P161", SharedCfgPad::Addr(0)),
                ("P162", SharedCfgPad::Addr(1)),
                ("P165", SharedCfgPad::Addr(2)),
                ("P166", SharedCfgPad::Addr(3)),
                ("P174", SharedCfgPad::Addr(4)),
                ("P175", SharedCfgPad::Addr(5)),
                ("P180", SharedCfgPad::Addr(6)),
                ("P181", SharedCfgPad::Addr(7)),
            ]);
        }
        "pq240" | "mq240" => {
            pkg_cfg_io.extend([
                ("P213", SharedCfgPad::Addr(8)),
                ("P214", SharedCfgPad::Addr(9)),
                ("P220", SharedCfgPad::Addr(10)),
                ("P221", SharedCfgPad::Addr(11)),
                ("P232", SharedCfgPad::Addr(12)),
                ("P233", SharedCfgPad::Addr(13)),
                ("P238", SharedCfgPad::Addr(14)),
                ("P239", SharedCfgPad::Addr(15)),
                ("P2", SharedCfgPad::Addr(16)),
                ("P3", SharedCfgPad::Addr(17)),
                ("P6", SharedCfgPad::Tdi),
                ("P7", SharedCfgPad::Tck),
                ("P17", SharedCfgPad::Tms),
                ("P64", SharedCfgPad::Hdc),
                ("P68", SharedCfgPad::Ldc),
                ("P89", SharedCfgPad::InitB),
                ("P123", SharedCfgPad::Data(7)),
                ("P129", SharedCfgPad::Data(6)),
                ("P141", SharedCfgPad::Data(5)),
                ("P142", SharedCfgPad::Cs0B),
                ("P148", SharedCfgPad::Data(4)),
                ("P152", SharedCfgPad::Data(3)),
                ("P153", SharedCfgPad::Cs1B),
                ("P159", SharedCfgPad::Data(2)),
                ("P173", SharedCfgPad::Data(1)),
                ("P174", SharedCfgPad::RclkB),
                ("P177", SharedCfgPad::Data(0)),
                ("P178", SharedCfgPad::Dout),
                ("P183", SharedCfgPad::Addr(0)),
                ("P184", SharedCfgPad::Addr(1)),
                ("P187", SharedCfgPad::Addr(2)),
                ("P188", SharedCfgPad::Addr(3)),
                ("P202", SharedCfgPad::Addr(4)),
                ("P203", SharedCfgPad::Addr(5)),
                ("P209", SharedCfgPad::Addr(6)),
                ("P210", SharedCfgPad::Addr(7)),
            ]);
        }
        _ => (),
    }
    let mut cfg_io = BTreeMap::new();
    for (pin, fun) in pkg_cfg_io {
        let BondPad::Io(io) = bond.pins[pin] else {
            unreachable!()
        };
        cfg_io.insert(fun, io);
    }

    (bond, cfg_io)
}
