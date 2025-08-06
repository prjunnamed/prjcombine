use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, CellSlotId, ConnectorClass, ConnectorWire, IntDb, PinDir, TileClass,
        TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
    grid::{CellCoord, DieId, EdgeIoCoord},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bels::xc2000 as bels,
    bond::{Bond, BondPad, CfgPad},
    chip::SharedCfgPad,
    regions, tslots,
};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind},
    cslots,
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
                wires: BTreeSet::from_iter([TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: db.get_wire(wire),
                }]),
                dir: if wire.starts_with("IMUX") {
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

    let mut main_terms = DirMap::from_fn(|dir| ConnectorClass {
        slot: term_slots[dir],
        wires: Default::default(),
    });

    for (name, stub) in [
        ("SINGLE.H0", true),
        ("SINGLE.H1", false),
        ("SINGLE.H2", true),
        ("SINGLE.H3", false),
        ("SINGLE.H4", true),
        ("SINGLE.H.B0", true),
        ("SINGLE.H.B1", false),
        ("SINGLE.H.B2", true),
        ("SINGLE.H.B3", false),
        ("SINGLE.H.B4", true),
        ("SINGLE.H.T0", true),
        ("SINGLE.H.T1", false),
        ("SINGLE.H.T2", true),
        ("SINGLE.H.T3", false),
        ("SINGLE.H.T4", true),
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::MultiOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.E"), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w1, ConnectorWire::Pass(w0));
        if stub {
            db.wires.insert(format!("{name}.STUB"), WireKind::MultiOut);
        }
    }

    for (name, stub) in [
        ("SINGLE.V0", true),
        ("SINGLE.V1", false),
        ("SINGLE.V2", true),
        ("SINGLE.V3", false),
        ("SINGLE.V4", false),
        ("SINGLE.V.L0", false),
        ("SINGLE.V.L1", false),
        ("SINGLE.V.L2", true),
        ("SINGLE.V.L3", false),
        ("SINGLE.V.L4", true),
        ("SINGLE.V.R0", false),
        ("SINGLE.V.R1", false),
        ("SINGLE.V.R2", true),
        ("SINGLE.V.R3", false),
        ("SINGLE.V.R4", true),
    ] {
        let w0 = db.wires.insert(name.into(), WireKind::MultiOut).0;
        let w1 = db
            .wires
            .insert(format!("{name}.S"), WireKind::MultiBranch(cslots::N))
            .0;
        main_terms[Dir::N].wires.insert(w1, ConnectorWire::Pass(w0));
        if stub {
            db.wires.insert(format!("{name}.STUB"), WireKind::MultiOut);
            db.wires
                .insert(format!("{name}.S.STUB"), WireKind::MultiOut);
        }
    }

    for name in [
        "LONG.H0",
        "LONG.H1",
        "LONG.IO.B0",
        "LONG.IO.B1",
        "LONG.IO.T0",
        "LONG.IO.T1",
    ] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    }
    for name in [
        "LONG.V0",
        "LONG.V1",
        "LONG.IO.L0",
        "LONG.IO.L1",
        "LONG.IO.R0",
        "LONG.IO.R1",
        "GCLK.V",
        "ACLK.V",
    ] {
        let w = db
            .wires
            .insert(name.into(), WireKind::MultiBranch(cslots::S))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }

    for name in [
        "GCLK", "ACLK", "IOCLK.B0", "IOCLK.B1", "IOCLK.T0", "IOCLK.T1", "IOCLK.L0", "IOCLK.L1",
        "IOCLK.R0", "IOCLK.R1",
    ] {
        db.wires
            .insert(name.into(), WireKind::Regional(regions::GLOBAL));
    }

    for name in [
        "IMUX.CLB.A",
        "IMUX.CLB.B",
        "IMUX.CLB.C",
        "IMUX.CLB.D",
        "IMUX.CLB.E",
        "IMUX.CLB.DI",
        "IMUX.CLB.EC",
        "IMUX.CLB.RD",
        "IMUX.CLB.K",
        "IMUX.BIOB0.O",
        "IMUX.BIOB0.T",
        "IMUX.BIOB0.IK",
        "IMUX.BIOB0.OK",
        "IMUX.BIOB1.O",
        "IMUX.BIOB1.T",
        "IMUX.BIOB1.IK",
        "IMUX.BIOB1.OK",
        "IMUX.TIOB0.O",
        "IMUX.TIOB0.T",
        "IMUX.TIOB0.IK",
        "IMUX.TIOB0.OK",
        "IMUX.TIOB1.O",
        "IMUX.TIOB1.T",
        "IMUX.TIOB1.IK",
        "IMUX.TIOB1.OK",
        "IMUX.LIOB0.O",
        "IMUX.LIOB0.T",
        "IMUX.LIOB0.IK",
        "IMUX.LIOB0.OK",
        "IMUX.LIOB1.O",
        "IMUX.LIOB1.T",
        "IMUX.LIOB1.IK",
        "IMUX.LIOB1.OK",
        "IMUX.RIOB0.O",
        "IMUX.RIOB0.T",
        "IMUX.RIOB0.IK",
        "IMUX.RIOB0.OK",
        "IMUX.RIOB1.O",
        "IMUX.RIOB1.T",
        "IMUX.RIOB1.IK",
        "IMUX.RIOB1.OK",
        "IMUX.TBUF0.I",
        "IMUX.TBUF0.T",
        "IMUX.TBUF1.I",
        "IMUX.TBUF1.T",
        "IMUX.TBUF2.I",
        "IMUX.TBUF2.T",
        "IMUX.TBUF3.I",
        "IMUX.TBUF3.T",
        "IMUX.BUFG",
        "IMUX.IOCLK0",
        "IMUX.IOCLK1",
    ] {
        db.wires.insert(name.into(), WireKind::MuxOut);
    }

    for (name, dirs) in [
        ("OUT.CLB.X", &[Dir::W, Dir::E][..]),
        ("OUT.CLB.Y", &[Dir::E, Dir::S][..]),
        ("OUT.BIOB0.I", &[][..]),
        ("OUT.BIOB0.Q", &[][..]),
        ("OUT.BIOB1.I", &[Dir::E][..]),
        ("OUT.BIOB1.Q", &[Dir::E][..]),
        ("OUT.TIOB0.I", &[][..]),
        ("OUT.TIOB0.Q", &[][..]),
        ("OUT.TIOB1.I", &[Dir::E][..]),
        ("OUT.TIOB1.Q", &[Dir::E][..]),
        ("OUT.LIOB0.I", &[][..]),
        ("OUT.LIOB0.Q", &[][..]),
        ("OUT.LIOB1.I", &[Dir::S][..]),
        ("OUT.LIOB1.Q", &[Dir::S][..]),
        ("OUT.RIOB0.I", &[][..]),
        ("OUT.RIOB0.Q", &[][..]),
        ("OUT.RIOB1.I", &[Dir::S][..]),
        ("OUT.RIOB1.Q", &[Dir::S][..]),
        ("OUT.CLKIOB", &[][..]),
        ("OUT.OSC", &[][..]),
    ] {
        let w = db.wires.insert(name.into(), WireKind::LogicOut).0;
        for &dir in dirs {
            let wo = db
                .wires
                .insert(format!("{name}.{dir}"), WireKind::Branch(term_slots[!dir]))
                .0;
            main_terms[!dir].wires.insert(wo, ConnectorWire::Pass(w));

            if name == "OUT.CLB.X" && dir == Dir::E {
                let wos = db
                    .wires
                    .insert(format!("{name}.{dir}S"), WireKind::Branch(cslots::N))
                    .0;
                main_terms[Dir::N]
                    .wires
                    .insert(wos, ConnectorWire::Pass(wo));
            }
        }
    }

    let mut llh_w = main_terms[Dir::W].clone();
    let mut llh_e = main_terms[Dir::E].clone();
    let mut llv_s = main_terms[Dir::S].clone();
    let mut llv_n = main_terms[Dir::N].clone();
    let mut llvs_s = main_terms[Dir::S].clone();
    let mut llvs_n = main_terms[Dir::N].clone();

    for term in [&mut llh_w, &mut llh_e] {
        term.wires.remove(db.get_wire("LONG.IO.B0"));
        term.wires.remove(db.get_wire("LONG.IO.T0"));
    }
    for term in [&mut llv_s, &mut llv_n, &mut llvs_n, &mut llvs_s] {
        term.wires.remove(db.get_wire("LONG.IO.L0"));
        term.wires.remove(db.get_wire("LONG.IO.R0"));
    }
    for term in [&mut llv_s, &mut llv_n] {
        term.wires.remove(db.get_wire("LONG.V0"));
        term.wires.remove(db.get_wire("LONG.V1"));
    }

    for (dir, term) in main_terms {
        db.conn_classes.insert(format!("MAIN.{dir}"), term);
    }
    db.conn_classes.insert("LLH.W".into(), llh_w);
    db.conn_classes.insert("LLH.E".into(), llh_e);
    db.conn_classes.insert("LLV.S".into(), llv_s);
    db.conn_classes.insert("LLV.N".into(), llv_n);
    db.conn_classes.insert("LLV.S.S".into(), llvs_s);
    db.conn_classes.insert("LLV.S.N".into(), llvs_n);

    for (name, num_cells) in [
        ("CLB", 4),
        ("CLB.L", 4),
        ("CLB.R", 3),
        ("CLB.B", 3),
        ("CLB.BL", 3),
        ("CLB.BR", 2),
        ("CLB.T", 3),
        ("CLB.TL", 3),
        ("CLB.TR", 2),
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
                    ("D", "IMUX.CLB.D"),
                    ("E", "IMUX.CLB.E"),
                    ("DI", "IMUX.CLB.DI"),
                    ("EC", "IMUX.CLB.EC"),
                    ("RD", "IMUX.CLB.RD"),
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
                            ("IK", format!("IMUX.{bt}IOB{i}.IK")),
                            ("OK", format!("IMUX.{bt}IOB{i}.OK")),
                            ("I", format!("OUT.{bt}IOB{i}.I")),
                            ("Q", format!("OUT.{bt}IOB{i}.Q")),
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
                tcls.bels.insert(
                    io[i],
                    bel_from_pins(
                        &db,
                        &[
                            ("O", format!("IMUX.{lr}IOB{i}.O")),
                            ("T", format!("IMUX.{lr}IOB{i}.T")),
                            ("IK", format!("IMUX.{lr}IOB{i}.IK")),
                            ("OK", format!("IMUX.{lr}IOB{i}.OK")),
                            ("I", format!("OUT.{lr}IOB{i}.I")),
                            ("Q", format!("OUT.{lr}IOB{i}.Q")),
                        ],
                    ),
                );
            }
        }

        for (i, slot) in [bels::TBUF0, bels::TBUF1, bels::TBUF0_E, bels::TBUF1_E]
            .into_iter()
            .enumerate()
        {
            if i >= 2 && !name.ends_with('R') {
                continue;
            }
            tcls.bels.insert(
                slot,
                bel_from_pins(
                    &db,
                    &[
                        ("I", format!("IMUX.TBUF{i}.I")),
                        ("T", format!("IMUX.TBUF{i}.T")),
                        ("O", format!("LONG.H{}", i % 2)),
                    ],
                ),
            );
        }
        if name.ends_with('L') || name.ends_with('R') {
            for i in 0..2 {
                tcls.bels.insert(
                    [bels::PULLUP_TBUF0, bels::PULLUP_TBUF1][i],
                    bel_from_pins(&db, &[("O", format!("LONG.H{}", i % 2))]),
                );
            }
        }

        if name == "CLB.TL" || name == "CLB.BR" {
            tcls.bels
                .insert(bels::CLKIOB, bel_from_pins(&db, &[("I", "OUT.CLKIOB")]));
            tcls.bels.insert(
                bels::BUFG,
                bel_from_pins(
                    &db,
                    &[
                        ("I", "IMUX.BUFG"),
                        ("O", if name == "CLB.TL" { "GCLK" } else { "ACLK" }),
                    ],
                ),
            );
        }
        if name == "CLB.BR" {
            tcls.bels
                .insert(bels::OSC, bel_from_pins(&db, &[("O", "OUT.OSC")]));
        }

        for subkind in 0..4 {
            if subkind == 3 && name != "CLB.R" {
                continue;
            }
            db.tile_classes
                .insert(format!("{name}.{subkind}"), tcls.clone());
            if matches!(name, "CLB.BL" | "CLB.BR" | "CLB.TL" | "CLB.TR" | "CLB.T") {
                db.tile_classes
                    .insert(format!("{name}S.{subkind}"), tcls.clone());
            }
        }
    }
    for (name, slot, sbslot) in [
        ("LLH.B", tslots::EXTRA_COL, bels::LLH),
        ("LLH.T", tslots::EXTRA_COL, bels::LLH),
        ("LLV.LS", tslots::EXTRA_ROW, bels::LLV),
        ("LLV.RS", tslots::EXTRA_ROW, bels::LLV),
        ("LLV.L", tslots::EXTRA_ROW, bels::LLV),
        ("LLV.R", tslots::EXTRA_ROW, bels::LLV),
        ("LLV", tslots::EXTRA_ROW, bels::LLV),
    ] {
        let mut tcls = TileClass::new(slot, 2);
        tcls.bels
            .insert(sbslot, BelInfo::SwitchBox(Default::default()));
        db.tile_classes.insert(name.into(), tcls);
    }

    db
}

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

pub fn dump_chip(die: &Die, kind: ChipKind) -> (Chip, IntDb, NamingDb) {
    let chip = make_chip(die, kind);
    let mut intdb = make_intdb();
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.tile_namings.insert(name.clone(), TileNaming::default());
        if name.starts_with("CLB") && !name.contains("L.") && !name.contains("R.") {
            ndb.tile_namings
                .insert(format!("{name}.L1"), TileNaming::default());
            if !name.starts_with("CLB.B") && !name.starts_with("CLB.T") {
                ndb.tile_namings
                    .insert(format!("{name}.L1.B1"), TileNaming::default());
            }
        }
        if name.starts_with("CLB") && !name.starts_with("CLB.B") && !name.starts_with("CLB.T") {
            ndb.tile_namings
                .insert(format!("{name}.B1"), TileNaming::default());
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

    let mut extractor = Extractor::new(die, &edev.egrid, &endev.ngrid);

    let die = DieId::from_idx(0);
    for (tcrd, tile) in edev.egrid.tiles() {
        let tcld = &intdb.tile_classes[tile.class];
        let ntile = &endev.ngrid.tiles[&tcrd];
        for (slot, bel_info) in &tcld.bels {
            let BelInfo::Bel(bel_info) = bel_info else {
                continue;
            };
            let bel = tcrd.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);
            match slot {
                bels::CLB | bels::OSC | bels::CLKIOB | bels::BUFG => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                }
                _ if slot_name.starts_with("IO") => {
                    let mut prim = extractor.grab_prim_i(&ntile.bels[slot][0]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                }
                _ if slot_name.starts_with("TBUF") => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    for pin in ["I", "T"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    extractor.net_bel_int(line, bel, "O");
                    extractor.bel_pip(ntile.naming, slot, "O", pip);

                    let net_i = extractor.get_bel_int_net(bel, "I");
                    let net_o = extractor.get_bel_int_net(bel, "O");
                    let src_nets = Vec::from_iter(extractor.nets[net_i].pips_bwd.keys().copied());
                    for net in src_nets {
                        extractor.mark_tbuf_pseudo(net_o, net);
                    }
                }
                _ if slot_name.starts_with("PULLUP") => {
                    let mut prim = extractor.grab_prim_a(&ntile.bels[slot][0]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    extractor.net_bel_int(line, bel, "O");
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
    for col in edev.egrid.cols(die) {
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
                    "IOCLK.L0",
                    "IOCLK.L1",
                    "LONG.IO.L0",
                    "LONG.IO.L1",
                    "LONG.V0",
                    "LONG.V1",
                    "ACLK.V",
                    "GCLK.V",
                ][..]
            } else if col == chip.col_e() {
                &[
                    "GCLK.V",
                    "LONG.V0",
                    "LONG.V1",
                    "ACLK.V",
                    "LONG.IO.R1",
                    "LONG.IO.R0",
                    "IOCLK.R1",
                    "IOCLK.R0",
                ][..]
            } else {
                &["GCLK.V", "LONG.V0", "LONG.V1", "ACLK.V"][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }
    // long horizontals
    for row in edev.egrid.rows(die) {
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
                &["IOCLK.B0", "IOCLK.B1", "LONG.IO.B0", "LONG.IO.B1"][..]
            } else if row == chip.row_n() {
                &["LONG.IO.T1", "LONG.IO.T0", "IOCLK.T1", "IOCLK.T0"][..]
            } else {
                &[][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
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
                    CellCoord::new(die, col, row).tile(tslots::EXTRA_ROW),
                ))
            } else {
                assert_eq!(rwt.cell.row, rwf.cell.row);
                // LLH
                let col = chip.col_mid();
                let row = rwt.cell.row;
                queue.push((
                    net_t,
                    net_f,
                    CellCoord::new(die, col, row).tile(tslots::EXTRA_COL),
                ))
            }
        }
    }
    for (net_t, net_f, tcrd) in queue {
        extractor.own_pip(net_t, net_f, tcrd);
    }

    // horizontal singles
    let mut queue = vec![];
    for col in edev.egrid.cols(die) {
        let mut x = endev.col_x[col].end;
        if col == chip.col_e() {
            x = endev.col_x[col].start + 8;
        }
        for row in edev.egrid.rows(die) {
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
                    "SINGLE.H.B4",
                    "SINGLE.H.B3",
                    "SINGLE.H.B2",
                    "SINGLE.H.B1",
                    "SINGLE.H.B0",
                    "SINGLE.H4",
                    "SINGLE.H3",
                    "SINGLE.H2",
                    "SINGLE.H1",
                    "SINGLE.H0",
                ][..]
            } else if row == chip.row_n() {
                &[
                    "SINGLE.H.T4",
                    "SINGLE.H.T3",
                    "SINGLE.H.T2",
                    "SINGLE.H.T1",
                    "SINGLE.H.T0",
                ][..]
            } else {
                &[
                    "SINGLE.H4",
                    "SINGLE.H3",
                    "SINGLE.H2",
                    "SINGLE.H1",
                    "SINGLE.H0",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
    }
    // vertical singles
    for row in edev.egrid.rows(die) {
        let mut y = endev.row_y[row].start;
        if row == chip.row_s() {
            y = endev.row_y[row + 1].start - 8;
        }
        for col in edev.egrid.cols(die) {
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
                    "SINGLE.V.L0",
                    "SINGLE.V.L1",
                    "SINGLE.V.L2",
                    "SINGLE.V.L3",
                    "SINGLE.V.L4",
                ][..]
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
                    "SINGLE.V.R4",
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
                            let sw = intdb
                                .get_wire(&format!("{wn}.STUB", wn = intdb.wires.key(rw.slot)));
                            extractor.net_int(net_d, rw.cell.wire(sw));
                        }
                    } else {
                        if extractor.nets[net_d].binding == NetBinding::None {
                            let sw = intdb
                                .get_wire(&format!("{wn}.S.STUB", wn = intdb.wires.key(rw.slot)));
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
                    let sw = intdb.get_wire(&format!("{wn}.STUB", wn = intdb.wires.key(rw.slot)));
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
        extractor.own_box(box_id, CellCoord::new(die, col, row).tile(tslots::MAIN));
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
        let wn = intdb.wires.key(rw.slot);
        assert!(wn.starts_with("IOCLK"));
        let nwn = if wn.ends_with('1') {
            "IMUX.IOCLK1"
        } else {
            "IMUX.IOCLK0"
        };
        let col = xlut.binary_search(&pip.0).unwrap_err();
        let row = ylut.binary_search(&pip.1).unwrap_err();
        assert!(col == chip.col_w() || col == chip.col_e());
        assert!(row == chip.row_s() || row == chip.row_n());
        queue.push((net, CellCoord::new(die, col, row).wire(intdb.get_wire(nwn))));
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    for (wire, name, &kind) in &intdb.wires {
        if !name.starts_with("IMUX") {
            continue;
        }
        if kind != WireKind::MuxOut {
            continue;
        }
        for (cell, _) in edev.egrid.cells() {
            let rw = edev.egrid.resolve_wire(cell.wire(wire)).unwrap();
            if extractor.int_nets.contains_key(&rw) {
                extractor.own_mux(rw, cell.tile(tslots::MAIN));
            }
        }
    }

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb, |db, tslot, wt, wf| {
        if tslot != tslots::MAIN {
            PipMode::Pass
        } else {
            let wtn = db.wires.key(wt.wire);
            let wfn = db.wires.key(wf.wire);
            if wtn == &format!("{wfn}.STUB") || wfn == &format!("{wtn}.STUB") {
                PipMode::Buf
            } else if wtn.starts_with("IMUX") {
                PipMode::Mux
            } else if wtn == "GCLK.V" && chip.is_small {
                PipMode::PermaBuf
            } else if wtn.starts_with("LONG")
                || wtn == "ACLK.V"
                || wtn == "GCLK.V"
                || wtn.starts_with("IOCLK")
            {
                PipMode::Buf
            } else {
                PipMode::Pass
            }
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
