use std::collections::{BTreeMap, BTreeSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, IntDb, SwitchBoxItem, TileWireCoord, WireKind},
    grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    xc2000 as defs,
    xc2000::bcls,
    xc2000::wires,
};

use crate::extractor::{Extractor, NetBinding, PipMode};

pub fn make_chip(die: &Die) -> Chip {
    let pd_clb = die
        .primdefs
        .iter()
        .find(|(_, pd)| pd.name == "cle")
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
    let clb_x = Vec::from_iter(clb_x);
    let clb_y = Vec::from_iter(clb_y);
    let mut cols_bidi_raw = BTreeSet::new();
    let mut rows_bidi_raw = BTreeSet::new();
    let matrix = die.matrix.as_ref().unwrap();
    for x in 0..matrix.dim().0 {
        for y in 0..matrix.dim().1 {
            let cv = matrix[(x, y)] & 0xff;
            if cv == 0x11 {
                cols_bidi_raw.insert(x);
            }
            if cv == 0x15 {
                rows_bidi_raw.insert(y);
            }
        }
    }
    let cols_bidi = cols_bidi_raw
        .into_iter()
        .map(|x| ColId::from_idx(clb_x.binary_search(&x).unwrap_err()))
        .collect();
    let rows_bidi = rows_bidi_raw
        .into_iter()
        .map(|y| RowId::from_idx(clb_y.binary_search(&y).unwrap_err()))
        .collect();
    Chip {
        kind: ChipKind::Xc2000,
        columns: clb_x.len(),
        rows: clb_y.len(),
        cols_bidi,
        rows_bidi,
        is_small: false,
        is_buff_large: false,
        cfg_io: Default::default(),
        unbonded_io: BTreeSet::new(),
    }
}

pub fn dump_chip(die: &Die) -> (Chip, IntDb, NamingDb) {
    let chip = make_chip(die);
    let mut intdb: IntDb = bincode::decode_from_slice(defs::INIT, bincode::config::standard())
        .unwrap()
        .0;
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.tile_namings.insert(name.clone(), TileNaming::default());
    }
    for name in ["CLB_S1W", "CLB_S1E"] {
        ndb.tile_namings.insert(name.into(), TileNaming::default());
    }
    let bd_c8 = die
        .boxdefs
        .iter()
        .find(|(_, bd)| bd.name == "cross8")
        .unwrap()
        .0;
    let mut c8_x = BTreeSet::new();
    let mut c8_y = BTreeSet::new();
    for boxx in die.boxes.values() {
        if boxx.boxdef != bd_c8 {
            continue;
        }
        c8_x.insert(usize::from(boxx.bx));
        c8_y.insert(usize::from(boxx.by));
    }
    let c8_x = Vec::from_iter(c8_x);
    let c8_y = Vec::from_iter(c8_y);
    assert_eq!(c8_x.len(), chip.columns * 2 - 2);
    assert_eq!(c8_y.len(), chip.rows * 2 - 2);
    ndb.tile_widths.insert("L".into(), c8_x[0] - 2);
    ndb.tile_widths.insert("C".into(), c8_x[2] - c8_x[0]);
    ndb.tile_widths.insert(
        "R".into(),
        die.matrix.as_ref().unwrap().dim().0 - (c8_x[chip.columns * 2 - 4] - 2),
    );
    ndb.tile_heights.insert("B".into(), c8_y[1] + 2);
    ndb.tile_heights.insert("C".into(), c8_y[3] - c8_y[1]);
    ndb.tile_heights.insert(
        "T".into(),
        die.matrix.as_ref().unwrap().dim().1 - (c8_y[chip.rows * 2 - 3] + 2),
    );
    let edev = chip.expand_grid(&intdb);
    let endev = name_device(&edev, &ndb);

    let mut extractor = Extractor::new(die, &edev, &endev.ngrid);

    let die = DieId::from_idx(0);
    for (tcrd, tile) in edev.tiles() {
        let tcls = &intdb[tile.class];
        let ntile = &endev.ngrid.tiles[&tcrd];
        for (slot, bel_info) in &tcls.bels {
            let bel = tcrd.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);
            match slot {
                defs::bslots::INT
                | defs::bslots::BIDIH
                | defs::bslots::BIDIV
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
                defs::bslots::CLB | defs::bslots::OSC => {
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
                    for pin in [bcls::IO::O, bcls::IO::T] {
                        extractor.pin_bel_input(&mut prim, bel, pin);
                    }
                    extractor.pin_bel_output(&mut prim, bel, bcls::IO::I);
                    let k = prim.get_pin("K");
                    extractor.net_bel(k, bel, "K");
                    let (line, pip) = extractor.consume_one_bwd(k, tcrd);
                    let wire = edev.get_bel_input(bel, bcls::IO::K);
                    extractor.net_int(line, wire.wire);
                    extractor.bel_pip(ntile.naming, slot, "K", pip);
                }
                _ => panic!("umm bel {slot_name}?"),
            }
        }
    }
    extractor.junk_prim_names.extend(
        ["VCC", "GND", "M0RT", "M1RD", "DP", "RST", "PWRDN", "CCLK"]
            .into_iter()
            .map(|x| x.to_string()),
    );

    // long verticals
    for col in edev.cols(die) {
        let row = chip.row_s() + 1;
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
            &[wires::LONG_IO_W, wires::LONG_V[0], wires::LONG_V[1]][..]
        } else if col == chip.col_e() {
            &[
                wires::LONG_V[0],
                wires::LONG_V[1],
                wires::LONG_VE[0],
                wires::LONG_VE[1],
                wires::LONG_IO_E,
            ][..]
        } else {
            &[wires::LONG_V[0], wires::LONG_V[1]][..]
        };
        assert_eq!(nets.len(), wires.len());
        for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
            extractor.net_int(net, CellCoord::new(die, col, row).wire(wire));
        }
    }
    // long horizontals
    for row in edev.rows(die) {
        let col = chip.col_w() + 1;
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
            &[wires::LONG_IO_S, wires::LONG_HS, wires::LONG_H][..]
        } else if row == chip.row_n() {
            &[wires::LONG_H, wires::LONG_IO_N][..]
        } else {
            &[wires::LONG_H][..]
        };
        assert_eq!(nets.len(), wires.len());
        for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
            extractor.net_int(net, CellCoord::new(die, col, row).wire(wire));
        }
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
                    wires::SINGLE_HS[3],
                    wires::SINGLE_HS[2],
                    wires::SINGLE_HS[1],
                    wires::SINGLE_HS[0],
                    wires::SINGLE_H[3],
                    wires::SINGLE_H[2],
                    wires::SINGLE_H[1],
                    wires::SINGLE_H[0],
                ][..]
            } else if row == chip.row_n() {
                &[
                    wires::SINGLE_HN[3],
                    wires::SINGLE_HN[2],
                    wires::SINGLE_HN[1],
                    wires::SINGLE_HN[0],
                ][..]
            } else {
                &[
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
        if tcls.bels.contains_id(defs::bslots::CLB) {
            for w in [
                TileWireCoord::new_idx(0, wires::SPECIAL_CLB_C).pos(),
                TileWireCoord::new_idx(0, wires::SPECIAL_CLB_G).neg(),
            ] {
                extractor.inject_pip(tcid, TileWireCoord::new_idx(0, wires::IMUX_CLB_K), w);
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
                }
            }
        }
    }

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb, |db, _, wt, _| {
        let wtn = db.wires.key(wt.wire);
        if wtn.starts_with("IMUX") || wtn.starts_with("IOCLK") {
            PipMode::Mux
        } else {
            PipMode::Pass
        }
    });
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
        "pd48" => (
            "P7",
            "P17",
            "P18",
            "P31",
            "P32",
            "P42",
            &["P24"][..],
            &["P12"][..],
        ),
        "pc44" => (
            "P8",
            "P16",
            "P17",
            "P27",
            "P28",
            "P38",
            &["P1", "P23"][..],
            &["P12", "P33"][..],
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
        "pc84" => (
            "P12",
            "P31",
            "P32",
            "P54",
            "P55",
            "P74",
            &["P1", "P43"][..],
            &["P22", "P64"][..],
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
            &["P13", "P63"][..],
            &["P38", "P88"][..],
        ),
        "pg68" => (
            "B2",
            "J1",
            "K1",
            "K10",
            "K11",
            "B11",
            &["B6", "K6"][..],
            &["F2", "F10"][..],
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
    match name {
        "pd48" => assert_eq!(bond.pins.len(), 48),
        "pc44" => assert_eq!(bond.pins.len(), 44),
        "pc68" => assert_eq!(bond.pins.len(), 68),
        "pc84" => assert_eq!(bond.pins.len(), 84),
        "vq64" => assert_eq!(bond.pins.len(), 64),
        "vq100" | "tq100" => {
            for i in 1..=100 {
                bond.pins.entry(format!("P{i}")).or_insert(BondPad::Nc);
            }
        }
        "pg68" => assert_eq!(bond.pins.len(), 68),
        "pg84" => assert_eq!(bond.pins.len(), 84),
        _ => panic!("ummm {name}?"),
    };

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
            ("P41", SharedCfgPad::Data(7)),
            ("P42", SharedCfgPad::Data(6)),
            ("P48", SharedCfgPad::Data(5)),
            ("P50", SharedCfgPad::Data(4)),
            ("P51", SharedCfgPad::Data(3)),
            ("P54", SharedCfgPad::Data(2)),
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
    }

    (bond, cfg_io)
}
