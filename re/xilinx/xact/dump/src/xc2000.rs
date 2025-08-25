use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, ConnectorClass, ConnectorWire, IntDb, PinDir, TileClass,
        TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
    grid::{CellCoord, ColId, DieId, EdgeIoCoord, RowId},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bels::xc2000 as bels,
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    cslots, regions, tslots,
};
use unnamed_entity::EntityId;

use crate::extractor::{Extractor, NetBinding, PipMode};

fn bel_from_pins(db: &IntDb, pins: &[(&str, impl AsRef<str>)]) -> BelInfo {
    let mut bel = Bel::default();
    for &(name, ref wire) in pins {
        let wire = wire.as_ref();
        bel.pins.insert(
            name.into(),
            BelPin {
                wires: BTreeSet::from_iter([TileWireCoord::new_idx(0, db.get_wire(wire))]),
                dir: if wire.starts_with("IMUX") || wire.starts_with("IOCLK") {
                    PinDir::Input
                } else {
                    PinDir::Output
                },
                is_intf_in: false,
            },
        );
    }
    BelInfo::Bel(bel)
}

pub fn make_intdb() -> IntDb {
    let mut db = IntDb::new(tslots::SLOTS, bels::SLOTS, regions::SLOTS, cslots::SLOTS);

    let term_slots = DirMap::from_fn(|dir| match dir {
        Dir::W => cslots::W,
        Dir::E => cslots::E,
        Dir::S => cslots::S,
        Dir::N => cslots::N,
    });

    let mut main_terms = DirMap::from_fn(|dir| ConnectorClass::new(term_slots[dir]));

    for name in [
        "SINGLE.H0",
        "SINGLE.H1",
        "SINGLE.H2",
        "SINGLE.H3",
        "SINGLE.H.B0",
        "SINGLE.H.B1",
        "SINGLE.H.B2",
        "SINGLE.H.B3",
        "SINGLE.H.T0",
        "SINGLE.H.T1",
        "SINGLE.H.T2",
        "SINGLE.H.T3",
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::MultiOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.E"), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w1, ConnectorWire::Pass(w0));
    }

    for name in [
        "SINGLE.V0",
        "SINGLE.V1",
        "SINGLE.V2",
        "SINGLE.V3",
        "SINGLE.V4",
        "SINGLE.V.L0",
        "SINGLE.V.L1",
        "SINGLE.V.L2",
        "SINGLE.V.L3",
        "SINGLE.V.R0",
        "SINGLE.V.R1",
        "SINGLE.V.R2",
        "SINGLE.V.R3",
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::MultiOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.S"), WireKind::MultiBranch(cslots::N))
            .0;
        main_terms[Dir::N].wires.insert(w1, ConnectorWire::Pass(w0));
    }

    for name in ["LONG.H", "LONG.BH", "LONG.IO.B", "LONG.IO.T"] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    }
    for name in [
        "LONG.V0",
        "LONG.V1",
        "LONG.RV0",
        "LONG.RV1",
        "LONG.IO.L",
        "LONG.IO.R",
    ] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(cslots::S))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }

    for name in ["GCLK", "ACLK", "IOCLK.B", "IOCLK.T", "IOCLK.L", "IOCLK.R"] {
        db.wires
            .insert(name.into(), WireKind::Regional(regions::GLOBAL));
    }

    for name in [
        "IMUX.CLB.A",
        "IMUX.CLB.B",
        "IMUX.CLB.C",
        "IMUX.CLB.D",
        "IMUX.CLB.K",
        "IMUX.BIOB0.O",
        "IMUX.BIOB0.T",
        "IMUX.BIOB1.O",
        "IMUX.BIOB1.T",
        "IMUX.TIOB0.O",
        "IMUX.TIOB0.T",
        "IMUX.TIOB1.O",
        "IMUX.TIOB1.T",
        "IMUX.LIOB0.O",
        "IMUX.LIOB0.T",
        "IMUX.LIOB1.O",
        "IMUX.LIOB1.T",
        "IMUX.RIOB0.O",
        "IMUX.RIOB0.T",
        "IMUX.RIOB1.O",
        "IMUX.RIOB1.T",
        "IMUX.BUFG",
    ] {
        let w = db.wires.insert(name.into(), WireKind::MuxOut).0;
        if name == "IMUX.CLB.D" {
            let wn = db
                .wires
                .insert(format!("{name}.N"), WireKind::Branch(cslots::S))
                .0;
            main_terms[Dir::S].wires.insert(wn, ConnectorWire::Pass(w));
        }
    }

    for (name, dirs) in [
        ("OUT.CLB.X", &[Dir::E, Dir::S, Dir::N][..]),
        ("OUT.CLB.Y", &[Dir::E][..]),
        ("OUT.BIOB0.I", &[][..]),
        ("OUT.BIOB1.I", &[Dir::E][..]),
        ("OUT.TIOB0.I", &[][..]),
        ("OUT.TIOB1.I", &[Dir::E][..]),
        ("OUT.LIOB0.I", &[][..]),
        ("OUT.LIOB1.I", &[Dir::S][..]),
        ("OUT.RIOB0.I", &[][..]),
        ("OUT.RIOB1.I", &[Dir::S][..]),
        ("OUT.OSC", &[][..]),
    ] {
        let w = db.wires.insert(name.into(), WireKind::LogicOut).0;
        for &dir in dirs {
            let wo = db
                .wires
                .insert(format!("{name}.{dir}"), WireKind::Branch(term_slots[!dir]))
                .0;
            main_terms[!dir].wires.insert(wo, ConnectorWire::Pass(w));
        }
    }

    for (dir, term) in main_terms {
        db.conn_classes.insert(format!("MAIN.{dir}"), term);
    }

    for (name, num_cells) in [
        ("CLB", 1),
        ("CLB.L", 2),
        ("CLB.R", 2),
        ("CLB.B", 2),
        ("CLB.BL", 2),
        ("CLB.BR", 1),
        ("CLB.T", 2),
        ("CLB.TL", 3),
        ("CLB.TR", 2),
        ("CLB.ML", 2),
        ("CLB.MR", 2),
        ("CLB.BR1", 2),
        ("CLB.TR1", 2),
    ] {
        let mut tcls = TileClass::new(tslots::MAIN, num_cells);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));

        tcls.bels.insert(
            bels::CLB,
            bel_from_pins(
                &db,
                &[
                    ("A", "IMUX.CLB.A"),
                    ("B", "IMUX.CLB.B"),
                    ("C", "IMUX.CLB.C"),
                    ("D", "IMUX.CLB.D.N"),
                    ("K", "IMUX.CLB.K"),
                    ("X", "OUT.CLB.X"),
                    ("Y", "OUT.CLB.Y"),
                ],
            ),
        );

        if name.starts_with("CLB.B") || name.starts_with("CLB.T") {
            let bt = if name.starts_with("CLB.B") { 'B' } else { 'T' };
            let io = if name.starts_with("CLB.B") {
                bels::IO_S
            } else {
                bels::IO_N
            };
            for i in 0..2 {
                tcls.bels.insert(
                    io[i],
                    bel_from_pins(
                        &db,
                        &[
                            ("O", format!("IMUX.{bt}IOB{i}.O")),
                            ("T", format!("IMUX.{bt}IOB{i}.T")),
                            ("I", format!("OUT.{bt}IOB{i}.I")),
                            ("K", format!("IOCLK.{bt}")),
                        ],
                    ),
                );
            }
        }

        if name.ends_with('L') || name.ends_with('R') {
            let lr = if name.ends_with('L') { 'L' } else { 'R' };
            let io = if name.ends_with('L') {
                bels::IO_W
            } else {
                bels::IO_E
            };
            for i in 0..2 {
                if i == 1 && name.starts_with("CLB.B") {
                    continue;
                }
                if i == 0 && (name.starts_with("CLB.T") || name.starts_with("CLB.M")) {
                    continue;
                }
                tcls.bels.insert(
                    io[i],
                    bel_from_pins(
                        &db,
                        &[
                            ("O", format!("IMUX.{lr}IOB{i}.O")),
                            ("T", format!("IMUX.{lr}IOB{i}.T")),
                            ("I", format!("OUT.{lr}IOB{i}.I")),
                            ("K", format!("IOCLK.{lr}")),
                        ],
                    ),
                );
            }
        }

        if name == "CLB.TL" {
            tcls.bels.insert(
                bels::BUFG,
                bel_from_pins(&db, &[("O", "GCLK"), ("I", "IMUX.BUFG")]),
            );
        }
        if name == "CLB.BR" {
            tcls.bels.insert(
                bels::BUFG,
                bel_from_pins(&db, &[("O", "ACLK"), ("I", "IMUX.BUFG")]),
            );
            tcls.bels
                .insert(bels::OSC, bel_from_pins(&db, &[("O", "OUT.OSC")]));
        }

        db.tile_classes.insert(name.into(), tcls);
    }

    for (name, slot, sbslot) in [
        ("BIDIH", tslots::EXTRA_COL, bels::LLH),
        ("BIDIV", tslots::EXTRA_ROW, bels::LLV),
    ] {
        let mut tcls = TileClass::new(slot, 0);
        tcls.bels
            .insert(sbslot, BelInfo::SwitchBox(Default::default()));
        db.tile_classes.insert(name.into(), tcls);
    }

    db
}

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
    let mut intdb = make_intdb();
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.tile_namings.insert(name.clone(), TileNaming::default());
    }
    for name in ["CLB.B1L", "CLB.B1R"] {
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
            let BelInfo::Bel(bel_info) = bel_info else {
                continue;
            };
            let bel = tcrd.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);
            match slot {
                bels::CLB | bels::BUFG | bels::OSC => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                }
                _ if slot_name.starts_with("IO") => {
                    let mut prim = extractor.grab_prim_i(&ntile.bels[slot][0]);
                    for pin in ["I", "O", "T"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    let k = prim.get_pin("K");
                    extractor.net_bel(k, bel, "K");
                    let (line, pip) = extractor.consume_one_bwd(k, tcrd);
                    extractor.net_bel_int(line, bel, "K");
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
            &["LONG.IO.L", "LONG.V0", "LONG.V1"][..]
        } else if col == chip.col_e() {
            &["LONG.V0", "LONG.V1", "LONG.RV0", "LONG.RV1", "LONG.IO.R"][..]
        } else {
            &["LONG.V0", "LONG.V1"][..]
        };
        assert_eq!(nets.len(), wires.len());
        for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
            let wire = intdb.get_wire(wire);
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
            &["LONG.IO.B", "LONG.BH", "LONG.H"][..]
        } else if row == chip.row_n() {
            &["LONG.H", "LONG.IO.T"][..]
        } else {
            &["LONG.H"][..]
        };
        assert_eq!(nets.len(), wires.len());
        for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
            let wire = intdb.get_wire(wire);
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
                    "SINGLE.H.B3",
                    "SINGLE.H.B2",
                    "SINGLE.H.B1",
                    "SINGLE.H.B0",
                    "SINGLE.H3",
                    "SINGLE.H2",
                    "SINGLE.H1",
                    "SINGLE.H0",
                ][..]
            } else if row == chip.row_n() {
                &["SINGLE.H.T3", "SINGLE.H.T2", "SINGLE.H.T1", "SINGLE.H.T0"][..]
            } else {
                &["SINGLE.H3", "SINGLE.H2", "SINGLE.H1", "SINGLE.H0"][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
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
                &["SINGLE.V.L0", "SINGLE.V.L1", "SINGLE.V.L2", "SINGLE.V.L3"][..]
            } else if col == chip.col_e() {
                &[
                    "SINGLE.V0",
                    "SINGLE.V1",
                    "SINGLE.V2",
                    "SINGLE.V3",
                    "SINGLE.V4",
                    "SINGLE.V.R0",
                    "SINGLE.V.R1",
                    "SINGLE.V.R2",
                    "SINGLE.V.R3",
                ][..]
            } else {
                &[
                    "SINGLE.V0",
                    "SINGLE.V1",
                    "SINGLE.V2",
                    "SINGLE.V3",
                    "SINGLE.V4",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
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
        extractor.own_box(box_id, CellCoord::new(die, col, row).tile(tslots::MAIN));
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
