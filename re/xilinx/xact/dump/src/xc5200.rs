use std::collections::{BTreeMap, BTreeSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, IntDb, SwitchBoxItem},
    dir::Dir,
    grid::{CellCoord, DieId, EdgeIoCoord},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    xc5200::{INIT, bcls, bslots, wires},
};

use crate::extractor::{Extractor, NetBinding, PipMode};

pub fn make_chip(die: &Die) -> Chip {
    Chip {
        kind: ChipKind::Xc5200,
        columns: die.newcols.len() - 1,
        rows: die.newrows.len() - 1,
        cfg_io: Default::default(),
        is_small: false,
        is_buff_large: false,
        cols_bidi: Default::default(),
        rows_bidi: Default::default(),
        unbonded_io: BTreeSet::new(),
    }
}

pub fn dump_chip(die: &Die) -> (Chip, IntDb, NamingDb) {
    let chip = make_chip(die);
    let mut intdb: IntDb = bincode::decode_from_slice(INIT, bincode::config::standard())
        .unwrap()
        .0;
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.tile_namings.insert(name.clone(), TileNaming::default());
    }
    for (key, kind) in [
        ("L", "left"),
        ("C", "center"),
        ("R", "right"),
        ("CLK", "clkc"),
    ] {
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
            let mut tie = extractor.grab_prim_a(&ntile.tie_names[0]);
            let o = tie.get_pin("O");
            extractor.net_int(o, cell.wire(wires::TIE_0));
            let mut dummy = extractor.grab_prim_a(&ntile.tie_names[1]);
            let i = dummy.get_pin("I");
            let wire = if col == chip.col_w() {
                if row == chip.row_s() {
                    wires::GCLK_SE
                } else if row == chip.row_n() {
                    wires::GCLK_SW
                } else {
                    wires::GCLK_E
                }
            } else if col == chip.col_e() {
                if row == chip.row_s() {
                    wires::GCLK_NE
                } else if row == chip.row_n() {
                    wires::GCLK_NW
                } else {
                    wires::GCLK_W
                }
            } else {
                if row == chip.row_s() {
                    wires::GCLK_N
                } else if row == chip.row_n() {
                    wires::GCLK_S
                } else {
                    unreachable!()
                }
            };
            let wire = cell.wire(wire);
            extractor.net_dummy(i);
            let (line, _) = extractor.consume_one_bwd(i, tcrd);
            extractor.net_int(line, wire);
            if ntile.tie_names.len() > 2 {
                // SCANTEST
                extractor.grab_prim_ab(&ntile.tie_names[2], &ntile.tie_names[3]);
            }
        }
        for (slot, bel_info) in &tcls.bels {
            let bel = cell.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);
            match slot {
                bslots::INT | bslots::LLH | bslots::LLV => (),
                bslots::BUFG => {
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
                bslots::RDBK | bslots::BSCAN => {
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
                bslots::STARTUP => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let BelInfo::Bel(bel_info) = bel_info else {
                        unreachable!();
                    };
                    for pid in bel_info.outputs.ids() {
                        extractor.pin_bel_output(&mut prim, bel, pid);
                    }
                    extractor.pin_bel_input(&mut prim, bel, bcls::STARTUP::GTS);
                    extractor.net_int(
                        prim.get_pin("CK"),
                        edev.get_bel_input(bel, bcls::STARTUP::CLK).wire,
                    );
                    extractor.net_int(
                        prim.get_pin("GCLR"),
                        edev.get_bel_input(bel, bcls::STARTUP::GR).wire,
                    );
                }
                bslots::OSC => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    for pid in [bcls::OSC::OSC1, bcls::OSC::OSC2] {
                        extractor.pin_bel_output(&mut prim, bel, pid);
                    }
                    extractor.net_int(
                        prim.get_pin("CK"),
                        edev.get_bel_input(bel, bcls::OSC::C).wire,
                    );
                    extractor.net_int(
                        prim.get_pin("BSUPD"),
                        edev.get_bel_output(cell.bel(bslots::BSUPD), bcls::BSUPD::O)[0],
                    );
                }
                bslots::BYPOSC => {
                    // ???
                }
                bslots::BSUPD => {
                    // handled with OSC
                }
                bslots::CLKIOB => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let net = prim.get_pin("I");
                    let wire = edev.get_bel_output(bel, bcls::CLKIOB::OUT)[0];
                    extractor.net_int(net, wire);
                }
                _ if bslots::IO.contains(slot) => {
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
                _ if bslots::TBUF.contains(slot) => {
                    let mut prim =
                        extractor.grab_prim_ab(&ntile.bels[slot][0], &ntile.bels[slot][1]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (net_o, pip) = extractor.consume_one_fwd(o, tcrd);
                    let wire_o = edev.get_bel_output(bel, bcls::TBUF::O)[0];
                    extractor.net_int(net_o, wire_o);
                    extractor.bel_pip(ntile.naming, slot, "O", pip);
                    let i = prim.get_pin("I");
                    extractor.net_bel(i, bel, "I");
                    let (net_i, pip) = extractor.consume_one_bwd(i, tcrd);
                    let wire_i = edev.get_bel_input(bel, bcls::TBUF::I);
                    extractor.net_int(net_i, wire_i.wire);
                    extractor.bel_pip(ntile.naming, slot, "I", pip);
                    let t = prim.get_pin("T");
                    extractor.net_bel(t, bel, "T");
                    let (net_t, pip) = extractor.consume_one_bwd(t, tcrd);
                    let wire_t = edev.get_bel_input(bel, bcls::TBUF::T);
                    extractor.net_int(net_t, wire_t.wire);
                    extractor.bel_pip(ntile.naming, slot, "T", pip);
                    extractor.mark_tbuf_pseudo(net_o, net_i);

                    let wi = wires::OMUX[wires::OMUX_BUF.index_of(wire_i.slot).unwrap()];
                    assert_eq!(extractor.nets[net_i].pips_bwd.len(), 1);
                    let net_omux = *extractor.nets[net_i].pips_bwd.iter().next().unwrap().0;
                    extractor.net_int(net_omux, cell.wire(wi));
                }
                bslots::BUFR | bslots::CIN | bslots::COUT => {
                    // handled later
                }
                _ if slot == bslots::LC[0] => {
                    let mut prim =
                        extractor.grab_prim_ab(&ntile.bels[slot][0], &ntile.bels[slot][1]);
                    for (pin, pid) in [
                        ("CE", bcls::LC::CE),
                        ("CK", bcls::LC::CK),
                        ("CLR", bcls::LC::CLR),
                    ] {
                        let net = prim.get_pin(pin);
                        let wire = edev.get_bel_input(bel, pid);
                        extractor.net_int(net, wire.wire);
                    }
                    let cv = prim.get_pin("CV");
                    let wire_cv =
                        edev.get_bel_output(cell.bel(bslots::VCC_GND), bcls::VCC_GND::O)[0];
                    extractor.net_int(cv, wire_cv);
                    let mut omux = Vec::from_iter(
                        extractor.nets[cv]
                            .pips_fwd
                            .iter()
                            .map(|(&net, &pip)| (net, pip)),
                    );
                    omux.sort_by_key(|(_, pip)| pip.0);
                    assert_eq!(omux.len(), 8);
                    for (i, (net, _)) in omux.into_iter().enumerate() {
                        let i = 7 - i;
                        extractor.net_int(net, cell.wire(wires::OMUX[i]));
                        assert_eq!(extractor.nets[net].pips_fwd.len(), 1);
                        let (&net_buf, _) = extractor.nets[net].pips_fwd.iter().next().unwrap();
                        extractor.net_int(net_buf, cell.wire(wires::OMUX_BUF[i]));
                    }
                    for (i, pin, spin) in [
                        (0, bcls::LC::F1, "LC0.F1"),
                        (0, bcls::LC::F2, "LC0.F2"),
                        (0, bcls::LC::F3, "LC0.F3"),
                        (0, bcls::LC::F4, "LC0.F4"),
                        (0, bcls::LC::DI, "LC0.DI"),
                        (1, bcls::LC::F1, "LC1.F1"),
                        (1, bcls::LC::F2, "LC1.F2"),
                        (1, bcls::LC::F3, "LC1.F3"),
                        (1, bcls::LC::F4, "LC1.F4"),
                        (1, bcls::LC::DI, "LC1.DI"),
                        (2, bcls::LC::F1, "LC2.F1"),
                        (2, bcls::LC::F2, "LC2.F2"),
                        (2, bcls::LC::F3, "LC2.F3"),
                        (2, bcls::LC::F4, "LC2.F4"),
                        (2, bcls::LC::DI, "LC2.DI"),
                        (3, bcls::LC::F1, "LC3.F1"),
                        (3, bcls::LC::F2, "LC3.F2"),
                        (3, bcls::LC::F3, "LC3.F3"),
                        (3, bcls::LC::F4, "LC3.F4"),
                        (3, bcls::LC::DI, "LC3.DI"),
                    ] {
                        let wire = edev.get_bel_input(cell.bel(bslots::LC[i]), pin);
                        extractor.net_int(prim.get_pin(spin), wire.wire);
                    }
                    for (i, pin, spin) in [
                        (0, bcls::LC::DO, "LC0.DO"),
                        (0, bcls::LC::X, "LC0.X"),
                        (0, bcls::LC::Q, "LC0.Q"),
                        (1, bcls::LC::DO, "LC1.DO"),
                        (1, bcls::LC::X, "LC1.X"),
                        (1, bcls::LC::Q, "LC1.Q"),
                        (2, bcls::LC::DO, "LC2.DO"),
                        (2, bcls::LC::X, "LC2.X"),
                        (2, bcls::LC::Q, "LC2.Q"),
                        (3, bcls::LC::DO, "LC3.DO"),
                        (3, bcls::LC::X, "LC3.X"),
                        (3, bcls::LC::Q, "LC3.Q"),
                    ] {
                        let wire = edev.get_bel_output(cell.bel(bslots::LC[i]), pin)[0];
                        extractor.net_int(prim.get_pin(spin), wire);
                    }
                    let ci = prim.get_pin("CI");
                    extractor.net_bel(ci, bel, "CI");
                    let co = prim.get_pin("CO");
                    if row == chip.row_n() - 1 {
                        let wire = edev
                            .get_bel_output(cell.delta(0, 1).bel(bslots::COUT), bcls::COUT::OUT)[0];
                        extractor.net_int(co, wire);
                    } else {
                        extractor.net_bel(co, bel, "CO");
                    }
                    let (co_b, pip) = extractor.consume_one_bwd(ci, tcrd);
                    extractor.bel_pip(ntile.naming, slot, "CI", pip);
                    if row == chip.row_s() + 1 {
                        let wire =
                            edev.get_bel_input(cell.delta(0, -1).bel(bslots::CIN), bcls::CIN::IN);
                        extractor.net_int(co_b, wire.wire);
                    } else {
                        extractor.net_bel(co_b, cell.delta(0, -1).bel(bslots::LC[0]), "CO");
                    }
                }
                _ if bslots::LC.contains(slot) => {
                    // handled with LC0
                }
                bslots::VCC_GND => {
                    // handled with LC0
                }
                bslots::SCANTEST => {
                    extractor.grab_prim_ab(&ntile.bels[slot][0], &ntile.bels[slot][1]);
                }

                _ => panic!("umm bel {slot_name}?"),
            }
        }
    }
    extractor.grab_prim_a("_cfg5200_");

    for (tcrd, tile) in edev.tiles() {
        let ntile = &endev.ngrid.tiles[&tcrd];
        let tcls = &intdb[tile.class];
        for (slot, bel_info) in &tcls.bels {
            if slot == bslots::BUFR {
                let BelInfo::SwitchBox(sb) = bel_info else {
                    unreachable!();
                };
                assert_eq!(sb.items.len(), 1);
                let SwitchBoxItem::PermaBuf(buf) = sb.items[0] else {
                    unreachable!();
                };
                let net = extractor.get_wire_net(edev.tile_wire(tcrd, buf.dst));
                let (imux, pip) = extractor.consume_one_bwd(net, tcrd);
                extractor.net_int(imux, edev.tile_wire(tcrd, buf.src.tw));
                extractor.bel_pip(ntile.naming, slot, "BUF", pip);
            }
        }
    }

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
            assert_eq!(nets.len(), 8);
            for (i, net) in nets.into_iter().enumerate() {
                let i = 7 - i;
                let wire = wires::LONG_V[i];
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
            assert_eq!(nets.len(), 8);
            for (i, net) in nets.into_iter().enumerate() {
                let wire = wires::LONG_H[i];
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // horizontal single and double
    let mut queue = vec![];
    for col in edev.cols(die) {
        if col == chip.col_w() {
            continue;
        }
        let x = endev.col_x[col].start;
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
                &wires::SINGLE_IO_S_W[..]
            } else if row == chip.row_n() {
                &wires::SINGLE_IO_N_W[..]
            } else {
                &[
                    wires::SINGLE_W[11],
                    wires::SINGLE_W[10],
                    wires::SINGLE_W[9],
                    wires::SINGLE_W[8],
                    wires::SINGLE_W[7],
                    wires::SINGLE_W[5],
                    wires::SINGLE_W[4],
                    wires::SINGLE_W[3],
                    wires::SINGLE_W[2],
                    wires::SINGLE_W[1],
                    wires::DBL_H_M[0],
                    wires::DBL_H_M[1],
                    wires::DBL_H_E[0],
                    wires::DBL_H_E[1],
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
    }
    // vertical single and double
    for row in edev.rows(die) {
        if row == chip.row_s() {
            continue;
        }
        let y = endev.row_y[row].start;
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
                    wires::SINGLE_IO_W_S[7],
                    wires::SINGLE_IO_W_S[6],
                    wires::SINGLE_IO_W_S[5],
                    wires::SINGLE_IO_W_S[4],
                    wires::SINGLE_IO_W_S[3],
                    wires::SINGLE_IO_W_S[2],
                    wires::SINGLE_IO_W_S[1],
                    wires::SINGLE_IO_W_S[0],
                ][..]
            } else if col == chip.col_e() {
                &[
                    wires::SINGLE_IO_E_S[7],
                    wires::SINGLE_IO_E_S[6],
                    wires::SINGLE_IO_E_S[5],
                    wires::SINGLE_IO_E_S[4],
                    wires::SINGLE_IO_E_S[3],
                    wires::SINGLE_IO_E_S[2],
                    wires::SINGLE_IO_E_S[1],
                    wires::SINGLE_IO_E_S[0],
                ][..]
            } else {
                &[
                    wires::DBL_V_M[1],
                    wires::DBL_V_N[1],
                    wires::DBL_V_N[0],
                    wires::DBL_V_M[0],
                    wires::SINGLE_S[11],
                    wires::SINGLE_S[10],
                    wires::SINGLE_S[9],
                    wires::SINGLE_S[8],
                    wires::SINGLE_S[7],
                    wires::SINGLE_S[5],
                    wires::SINGLE_S[4],
                    wires::SINGLE_S[3],
                    wires::SINGLE_S[2],
                    wires::SINGLE_S[1],
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

    for (cell, _) in edev.cells() {
        let CellCoord { col, row, .. } = cell;
        if row == chip.row_s() || row == chip.row_n() {
            if col == chip.col_w() || col == chip.col_e() {
                continue;
            }
            let mut crd = None;
            'a: for x in endev.col_x[col].clone() {
                for y in endev.row_y[row].clone() {
                    if (0..16).all(|dy| extractor.matrix[(x, y + dy)] & 0xff == 0x6b) {
                        crd = Some((x, y));
                        break 'a;
                    }
                }
            }
            let crd = crd.unwrap();
            for i in 0..16 {
                let net = extractor.get_net(crd.0, crd.1 + i, Dir::E).unwrap();
                let net_b = extractor.get_net(crd.0, crd.1 + i, Dir::W).unwrap();
                extractor.net_int(net, cell.wire(wires::IO_M[i]));
                extractor.net_int(net_b, cell.wire(wires::IO_M_BUF[i]))
            }
        } else if col == chip.col_w() || col == chip.col_e() {
            let mut crd = None;
            'a: for x in endev.col_x[col].clone() {
                for y in endev.row_y[row].clone() {
                    if (0..16).all(|dx| extractor.matrix[(x + dx, y)] & 0xff == 0x2a) {
                        crd = Some((x, y));
                        break 'a;
                    }
                }
            }
            let crd = crd.unwrap();
            for i in 0..16 {
                let net = extractor.get_net(crd.0 + (15 - i), crd.1, Dir::S).unwrap();
                let net_b = extractor.get_net(crd.0 + (15 - i), crd.1, Dir::N).unwrap();
                extractor.net_int(net, cell.wire(wires::IO_M[i]));
                extractor.net_int(net_b, cell.wire(wires::IO_M_BUF[i]))
            }
        } else {
            let mut crd = None;
            'a: for x in endev.col_x[col].clone() {
                for y in endev.row_y[row].clone() {
                    if (0..24).all(|dx| extractor.matrix[(x + dx, y)] & 0xff == 0x2a) {
                        crd = Some((x, y));
                        break 'a;
                    }
                }
            }
            let crd = crd.unwrap();
            for i in 0..24 {
                let net = extractor.get_net(crd.0 + (23 - i), crd.1, Dir::S).unwrap();
                let net_b = extractor.get_net(crd.0 + (23 - i), crd.1, Dir::N).unwrap();
                extractor.net_int(net, cell.wire(wires::CLB_M[i]));
                extractor.net_int(net_b, cell.wire(wires::CLB_M_BUF[i]))
            }
        }
    }

    let mut queue = vec![];
    for (net_t, net_info) in &extractor.nets {
        let NetBinding::Int(rw_t) = net_info.binding else {
            continue;
        };
        let w_t = intdb.wires.key(rw_t.slot);
        for &net_f in net_info.pips_bwd.keys() {
            let NetBinding::Int(rw_f) = extractor.nets[net_f].binding else {
                continue;
            };
            let w_f = intdb.wires.key(rw_f.slot);
            if w_t.starts_with("LONG") && !(w_f.starts_with("LONG") || w_f.starts_with("OUT")) {
                queue.push((net_t, net_f));
            }
            if rw_f.slot == wires::IMUX_BOT_CIN {
                queue.push((net_t, net_f));
            }
        }
    }
    for (net_t, net_f) in queue {
        extractor.mark_tbuf_pseudo(net_t, net_f);
    }

    for i in 0..8 {
        let wire = wires::SINGLE_IO_W_N[i];
        let rw = edev
            .resolve_wire(CellCoord::new(die, chip.col_w(), chip.row_s()).wire(wire))
            .unwrap();
        let net = extractor.int_nets[&rw];
        let nbto = extractor
            .net_by_cell_override
            .entry(CellCoord::new(die, chip.col_w(), chip.row_s()))
            .or_default();
        nbto.insert(net, wire);
    }

    let finisher = extractor.finish();
    finisher.finish(
        &mut intdb,
        &mut ndb,
        |db, _, wt, _| {
            let wtn = db.wires.key(wt.wire);
            if wires::IO_M_BUF.contains(wt.wire)
                || wires::CLB_M_BUF.contains(wt.wire)
                || wires::OMUX_BUF.contains(wt.wire)
            {
                PipMode::PermaBuf
            } else if wtn.starts_with("IMUX") || wtn.starts_with("OMUX") {
                PipMode::Mux
            } else {
                PipMode::Pass
            }
        },
        false,
    );
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

    let (gnd, vcc, done, prog, cclk) = match name {
        "pc84" => (
            &["P1", "P12", "P21", "P31", "P43", "P52", "P64", "P76"][..],
            &["P2", "P11", "P22", "P33", "P42", "P54", "P63", "P74"][..],
            "P53",
            "P55",
            "P73",
        ),
        "pq100" => (
            &["P4", "P14", "P26", "P41", "P52", "P67", "P80", "P91"][..],
            &["P3", "P15", "P28", "P40", "P54", "P66", "P78", "P92"][..],
            "P53",
            "P55",
            "P77",
        ),
        "pq160" => (
            &[
                "P1", "P10", "P19", "P29", "P39", "P51", "P61", "P70", "P79", "P91", "P101",
                "P110", "P122", "P131", "P141", "P151",
            ][..],
            &["P20", "P41", "P60", "P81", "P100", "P120", "P142", "P160"][..],
            "P80",
            "P82",
            "P119",
        ),
        "pq208" | "hq208" => (
            &[
                "P2", "P14", "P25", "P37", "P49", "P67", "P79", "P90", "P101", "P119", "P131",
                "P142", "P160", "P171", "P182", "P194",
            ][..],
            &["P26", "P55", "P78", "P106", "P130", "P154", "P183", "P205"][..],
            "P103",
            "P108",
            "P153",
        ),
        "pq240" | "hq240" => (
            &[
                "P1", "P14", "P29", "P45", "P59", "P75", "P91", "P106", "P119", "P135", "P151",
                "P166", "P182", "P196", "P211", "P227",
            ][..],
            &[
                "P19", "P30", "P40", "P61", "P80", "P90", "P101", "P121", "P140", "P150", "P161",
                "P180", "P201", "P212", "P222", "P240",
            ][..],
            "P120",
            "P122",
            "P179",
        ),
        "hq304" => (
            &[
                "P19", "P39", "P58", "P75", "P95", "P114", "P134", "P154", "P171", "P190", "P210",
                "P230", "P248", "P268", "P287", "P304",
            ][..],
            &[
                "P1", "P25", "P38", "P52", "P77", "P101", "P115", "P129", "P152", "P177", "P191",
                "P204", "P228", "P253", "P267", "P282",
            ][..],
            "P153",
            "P151",
            "P78",
        ),
        "tq144" => (
            &[
                "P1", "P8", "P17", "P27", "P35", "P45", "P55", "P64", "P71", "P73", "P81", "P91",
                "P100", "P110", "P118", "P127", "P137",
            ][..],
            &["P18", "P37", "P54", "P90", "P108", "P128", "P144"][..],
            "P72",
            "P74",
            "P107",
        ),
        "tq176" => (
            &[
                "P1", "P10", "P22", "P33", "P43", "P55", "P67", "P78", "P87", "P99", "P111",
                "P122", "P134", "P143", "P154", "P166",
            ][..],
            &["P21", "P45", "P66", "P89", "P110", "P132", "P155", "P176"][..],
            "P88",
            "P90",
            "P131",
        ),
        "vq64" => (
            &["P8", "P25", "P41", "P56"][..],
            &["P9", "P24", "P33", "P40", "P64"][..],
            "P32",
            "P34",
            "P48",
        ),
        "vq100" => (
            &["P1", "P11", "P23", "P38", "P49", "P64", "P77", "P88"][..],
            &["P12", "P25", "P37", "P51", "P63", "P75", "P89", "P100"][..],
            "P50",
            "P52",
            "P74",
        ),
        "pg156" => (
            &[
                "F3", "C4", "C6", "C8", "C11", "C13", "F14", "J14", "L14", "P14", "P11", "P8",
                "P6", "N3", "L3", "H2",
            ][..],
            &["H3", "C3", "B8", "C14", "H14", "P13", "R8", "P3"][..],
            "R15",
            "R14",
            "R2",
        ),
        "pg191" => (
            &[
                "G3", "D4", "C7", "D9", "C12", "D15", "G16", "K15", "M16", "R16", "T12", "R9",
                "T7", "R3", "M3", "K4",
            ][..],
            &["J4", "D3", "D10", "D16", "J15", "R15", "R10", "R4"][..],
            "U17",
            "V18",
            "V1",
        ),
        "pg223" => (
            &[
                "G3", "D4", "C7", "D9", "C12", "D15", "G16", "K15", "M16", "R16", "T12", "R9",
                "T7", "R3", "M3", "K4",
            ][..],
            &["J4", "D3", "D10", "D16", "J15", "R15", "R10", "R4"][..],
            "U17",
            "V18",
            "V1",
        ),
        "pg299" => (
            &[
                "F1", "B1", "A5", "A10", "A15", "A19", "E20", "K20", "R20", "W20", "X16", "X11",
                "X6", "X2", "T1", "L1",
            ][..],
            &[
                "K1", "E1", "A2", "A6", "A11", "A16", "B20", "F20", "L20", "T20", "X19", "X15",
                "X10", "X5", "W1", "R1",
            ][..],
            "V18",
            "U17",
            "V3",
        ),
        "bg225" => (
            &[
                "A1", "D12", "G7", "G9", "H6", "H8", "H10", "J8", "K8", "A8", "F8", "G8", "H2",
                "H7", "H9", "J7", "J9", "M8",
            ][..],
            &["B2", "D8", "H15", "R8", "B14", "R1", "H1", "R15"][..],
            "P14",
            "M12",
            "C13",
        ),
        "bg352" => (
            &[
                "A1", "A2", "A5", "A8", "A14", "A19", "A22", "A25", "A26", "B1", "B26", "E1",
                "E26", "H1", "H26", "N1", "P26", "W1", "W26", "AB1", "AB26", "AE1", "AE26", "AF1",
                "AF13", "AF19", "AF2", "AF22", "AF25", "AF26", "AF5", "AF8",
            ][..],
            &[
                "A10", "A17", "B2", "B25", "D13", "D19", "D7", "G23", "H4", "K1", "K26", "N23",
                "P4", "U1", "U26", "W23", "Y4", "AC14", "AC20", "AC8", "AE2", "AE25", "AF10",
                "AF17",
            ][..],
            "AD3",
            "AC4",
            "C3",
        ),
        _ => panic!("ummm {name}?"),
    };
    for &pin in gnd {
        assert_eq!(bond.pins.insert(pin.to_string(), BondPad::Gnd), None);
    }
    for &pin in vcc {
        assert_eq!(bond.pins.insert(pin.to_string(), BondPad::Vcc), None);
    }
    assert_eq!(
        bond.pins
            .insert(done.to_string(), BondPad::Cfg(CfgPad::Done)),
        None
    );
    assert_eq!(
        bond.pins
            .insert(prog.to_string(), BondPad::Cfg(CfgPad::ProgB)),
        None
    );
    assert_eq!(
        bond.pins
            .insert(cclk.to_string(), BondPad::Cfg(CfgPad::Cclk)),
        None
    );

    let len1d = match name {
        "pc84" => Some(84),
        "pq100" => Some(100),
        "pq160" => Some(160),
        "pq208" | "hq208" => Some(208),
        "pq240" | "hq240" => Some(240),
        "hq304" => Some(304),
        "tq144" => Some(144),
        "tq176" => Some(176),
        "vq100" => Some(100),
        "vq64" => Some(64),
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
            assert_eq!(bond.pins.len(), 225);
        }
        "bg352" => {
            for a in ["A", "B", "C", "D", "AC", "AD", "AE", "AF"] {
                for i in 1..=26 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in [
                "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R", "T", "U", "V", "W", "Y",
                "AA", "AB",
            ] {
                for i in (1..=4).chain(23..=26) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 352);
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
        "pg299" => {
            for i in 2..=20 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPad::Nc);
            }
            for a in ["B", "C", "D", "E", "T", "U", "V", "W", "X"] {
                for i in 1..=20 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            for a in ["F", "G", "H", "J", "K", "L", "M", "N", "P", "R"] {
                for i in (1..=5).chain(16..=20) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPad::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 299);
        }
        _ => (),
    }

    let pkg_cfg_io = match name {
        "pc84" => &[
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
            ("P30", SharedCfgPad::M1),
            ("P32", SharedCfgPad::M0),
            ("P34", SharedCfgPad::M2),
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
            ("P75", SharedCfgPad::Tdo),
            ("P77", SharedCfgPad::Addr(0)),
            ("P78", SharedCfgPad::Addr(1)),
            ("P79", SharedCfgPad::Addr(2)),
            ("P80", SharedCfgPad::Addr(3)),
            ("P81", SharedCfgPad::Addr(4)),
            ("P82", SharedCfgPad::Addr(5)),
            ("P83", SharedCfgPad::Addr(6)),
            ("P84", SharedCfgPad::Addr(7)),
        ][..],
        "pq160" => &[
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
            ("P38", SharedCfgPad::M1),
            ("P40", SharedCfgPad::M0),
            ("P42", SharedCfgPad::M2),
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
            ("P121", SharedCfgPad::Tdo),
            ("P123", SharedCfgPad::Addr(0)),
            ("P124", SharedCfgPad::Addr(1)),
            ("P127", SharedCfgPad::Addr(2)),
            ("P128", SharedCfgPad::Addr(3)),
            ("P134", SharedCfgPad::Addr(4)),
            ("P135", SharedCfgPad::Addr(5)),
            ("P139", SharedCfgPad::Addr(6)),
            ("P140", SharedCfgPad::Addr(7)),
        ][..],
        "pq208" => &[
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
            ("P48", SharedCfgPad::M1),
            ("P50", SharedCfgPad::M0),
            ("P56", SharedCfgPad::M2),
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
            ("P159", SharedCfgPad::Tdo),
            ("P161", SharedCfgPad::Addr(0)),
            ("P162", SharedCfgPad::Addr(1)),
            ("P165", SharedCfgPad::Addr(2)),
            ("P166", SharedCfgPad::Addr(3)),
            ("P174", SharedCfgPad::Addr(4)),
            ("P175", SharedCfgPad::Addr(5)),
            ("P180", SharedCfgPad::Addr(6)),
            ("P181", SharedCfgPad::Addr(7)),
        ][..],
        "pq240" => &[
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
            ("P58", SharedCfgPad::M1),
            ("P60", SharedCfgPad::M0),
            ("P62", SharedCfgPad::M2),
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
            ("P181", SharedCfgPad::Tdo),
            ("P183", SharedCfgPad::Addr(0)),
            ("P184", SharedCfgPad::Addr(1)),
            ("P187", SharedCfgPad::Addr(2)),
            ("P188", SharedCfgPad::Addr(3)),
            ("P202", SharedCfgPad::Addr(4)),
            ("P203", SharedCfgPad::Addr(5)),
            ("P209", SharedCfgPad::Addr(6)),
            ("P210", SharedCfgPad::Addr(7)),
        ][..],
        _ => &[][..],
    };
    let mut cfg_io = BTreeMap::new();
    for &(pin, fun) in pkg_cfg_io {
        let BondPad::Io(io) = bond.pins[pin] else {
            unreachable!()
        };
        cfg_io.insert(fun, io);
    }

    (bond, cfg_io)
}
