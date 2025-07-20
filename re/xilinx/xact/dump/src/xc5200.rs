use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, CellSlotId, ConnectorClass, ConnectorSlot, ConnectorSlotId,
        ConnectorWire, IntDb, PinDir, TileClass, TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
    grid::{CellCoord, DieId, EdgeIoCoord},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, NodeNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bels::xc5200 as bels,
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    tslots,
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
                dir: if wire.starts_with("IMUX") || wire.starts_with("OMUX") {
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
    let mut db = IntDb::default();

    db.init_slots(tslots::SLOTS, bels::SLOTS);

    let slot_w = db
        .conn_slots
        .insert(
            "W".into(),
            ConnectorSlot {
                opposite: ConnectorSlotId::from_idx(0),
            },
        )
        .0;
    let slot_e = db
        .conn_slots
        .insert("E".into(), ConnectorSlot { opposite: slot_w })
        .0;
    let slot_s = db
        .conn_slots
        .insert(
            "S".into(),
            ConnectorSlot {
                opposite: ConnectorSlotId::from_idx(0),
            },
        )
        .0;
    let slot_n = db
        .conn_slots
        .insert("N".into(), ConnectorSlot { opposite: slot_s })
        .0;
    db.conn_slots[slot_w].opposite = slot_e;
    db.conn_slots[slot_s].opposite = slot_n;

    let term_slots = DirMap::from_fn(|dir| match dir {
        Dir::W => slot_w,
        Dir::E => slot_e,
        Dir::S => slot_s,
        Dir::N => slot_n,
    });

    let mut main_terms = DirMap::from_fn(|dir| ConnectorClass {
        slot: term_slots[dir],
        wires: Default::default(),
    });
    let mut cnr_ll_w = ConnectorClass {
        slot: slot_w,
        wires: Default::default(),
    };
    let mut cnr_lr_s = ConnectorClass {
        slot: slot_s,
        wires: Default::default(),
    };
    let mut cnr_ul_n = ConnectorClass {
        slot: slot_n,
        wires: Default::default(),
    };
    let mut cnr_ur_e = ConnectorClass {
        slot: slot_e,
        wires: Default::default(),
    };

    db.wires.insert("GND".into(), WireKind::Tie0);

    for i in 0..24 {
        db.wires.insert(format!("CLB.M{i}"), WireKind::MultiOut);
        db.wires.insert(format!("CLB.M{i}.BUF"), WireKind::MuxOut);
    }
    for i in 0..16 {
        db.wires.insert(format!("IO.M{i}"), WireKind::MultiOut);
        db.wires.insert(format!("IO.M{i}.BUF"), WireKind::MuxOut);
    }

    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w0 = db
            .wires
            .insert(format!("SINGLE.E{i}"), WireKind::MultiOut)
            .0;
        let w1 = db
            .wires
            .insert(format!("SINGLE.W{i}"), WireKind::MultiBranch(slot_w))
            .0;
        main_terms[Dir::W].wires.insert(w1, ConnectorWire::Pass(w0));
    }
    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w0 = db
            .wires
            .insert(format!("SINGLE.S{i}"), WireKind::MultiOut)
            .0;
        let w1 = db
            .wires
            .insert(format!("SINGLE.N{i}"), WireKind::MultiBranch(slot_n))
            .0;
        main_terms[Dir::N].wires.insert(w1, ConnectorWire::Pass(w0));
    }

    for i in 0..8 {
        let w_be = db
            .wires
            .insert(format!("IO.SINGLE.B.E{i}"), WireKind::MultiBranch(slot_w))
            .0;
        let w_bw = db
            .wires
            .insert(format!("IO.SINGLE.B.W{i}"), WireKind::MultiBranch(slot_w))
            .0;
        main_terms[Dir::W]
            .wires
            .insert(w_bw, ConnectorWire::Pass(w_be));
        let w_rn = db
            .wires
            .insert(format!("IO.SINGLE.R.N{i}"), WireKind::MultiBranch(slot_s))
            .0;
        let w_rs = db
            .wires
            .insert(format!("IO.SINGLE.R.S{i}"), WireKind::MultiBranch(slot_s))
            .0;
        main_terms[Dir::S]
            .wires
            .insert(w_rs, ConnectorWire::Pass(w_rn));
        let w_tw = db
            .wires
            .insert(format!("IO.SINGLE.T.W{i}"), WireKind::MultiBranch(slot_e))
            .0;
        let w_te = db
            .wires
            .insert(format!("IO.SINGLE.T.E{i}"), WireKind::MultiBranch(slot_e))
            .0;
        main_terms[Dir::E]
            .wires
            .insert(w_te, ConnectorWire::Pass(w_tw));
        let w_ls = db
            .wires
            .insert(format!("IO.SINGLE.L.S{i}"), WireKind::MultiBranch(slot_n))
            .0;
        let w_ln = db
            .wires
            .insert(format!("IO.SINGLE.L.N{i}"), WireKind::MultiBranch(slot_n))
            .0;
        main_terms[Dir::N]
            .wires
            .insert(w_ln, ConnectorWire::Pass(w_ls));
        cnr_ll_w.wires.insert(w_be, ConnectorWire::Reflect(w_ln));
        cnr_lr_s.wires.insert(w_rn, ConnectorWire::Reflect(w_bw));
        cnr_ul_n.wires.insert(w_ls, ConnectorWire::Reflect(w_te));
        cnr_ur_e.wires.insert(w_tw, ConnectorWire::Reflect(w_rs));
    }

    for i in [0, 6] {
        let w = db.wires.insert(format!("DBL.H{i}.M"), WireKind::MultiOut).0;
        let ww = db
            .wires
            .insert(format!("DBL.H{i}.W"), WireKind::MultiBranch(slot_e))
            .0;
        main_terms[Dir::E].wires.insert(ww, ConnectorWire::Pass(w));
        let we = db
            .wires
            .insert(format!("DBL.H{i}.E"), WireKind::MultiBranch(slot_w))
            .0;
        main_terms[Dir::W].wires.insert(we, ConnectorWire::Pass(w));
    }
    for i in [0, 6] {
        let w = db.wires.insert(format!("DBL.V{i}.M"), WireKind::MultiOut).0;
        let ws = db
            .wires
            .insert(format!("DBL.V{i}.S"), WireKind::MultiBranch(slot_n))
            .0;
        main_terms[Dir::N].wires.insert(ws, ConnectorWire::Pass(w));
        let wn = db
            .wires
            .insert(format!("DBL.V{i}.N"), WireKind::MultiBranch(slot_s))
            .0;
        main_terms[Dir::S].wires.insert(wn, ConnectorWire::Pass(w));
    }

    for i in 0..8 {
        let w = db
            .wires
            .insert(format!("LONG.H{i}"), WireKind::MultiBranch(slot_w))
            .0;
        main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    }
    for i in 0..8 {
        let w = db
            .wires
            .insert(format!("LONG.V{i}"), WireKind::MultiBranch(slot_s))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }

    let w = db
        .wires
        .insert("GLOBAL.L".into(), WireKind::Branch(slot_w))
        .0;
    main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    let w = db
        .wires
        .insert("GLOBAL.R".into(), WireKind::Branch(slot_e))
        .0;
    main_terms[Dir::E].wires.insert(w, ConnectorWire::Pass(w));
    let w = db
        .wires
        .insert("GLOBAL.B".into(), WireKind::Branch(slot_s))
        .0;
    main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    let w = db
        .wires
        .insert("GLOBAL.T".into(), WireKind::Branch(slot_n))
        .0;
    main_terms[Dir::N].wires.insert(w, ConnectorWire::Pass(w));

    let w = db
        .wires
        .insert("GLOBAL.TL".into(), WireKind::Branch(slot_w))
        .0;
    main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    let w = db
        .wires
        .insert("GLOBAL.BR".into(), WireKind::Branch(slot_e))
        .0;
    main_terms[Dir::E].wires.insert(w, ConnectorWire::Pass(w));
    let w = db
        .wires
        .insert("GLOBAL.BL".into(), WireKind::Branch(slot_s))
        .0;
    main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    let w = db
        .wires
        .insert("GLOBAL.TR".into(), WireKind::Branch(slot_n))
        .0;
    main_terms[Dir::N].wires.insert(w, ConnectorWire::Pass(w));

    for i in 0..8 {
        // only 4 of these outside CLB
        db.wires.insert(format!("OMUX{i}"), WireKind::MuxOut);
        let w = db.wires.insert(format!("OMUX{i}.BUF"), WireKind::MuxOut).0;
        if i < 4 {
            let ww = db
                .wires
                .insert(format!("OMUX{i}.BUF.W"), WireKind::Branch(slot_e))
                .0;
            main_terms[Dir::E].wires.insert(ww, ConnectorWire::Pass(w));
            let we = db
                .wires
                .insert(format!("OMUX{i}.BUF.E"), WireKind::Branch(slot_w))
                .0;
            main_terms[Dir::W].wires.insert(we, ConnectorWire::Pass(w));
            let ws = db
                .wires
                .insert(format!("OMUX{i}.BUF.S"), WireKind::Branch(slot_n))
                .0;
            main_terms[Dir::N].wires.insert(ws, ConnectorWire::Pass(w));
            let wn = db
                .wires
                .insert(format!("OMUX{i}.BUF.N"), WireKind::Branch(slot_s))
                .0;
            main_terms[Dir::S].wires.insert(wn, ConnectorWire::Pass(w));
        }
    }

    for i in 0..4 {
        for pin in ["X", "Q", "DO"] {
            db.wires
                .insert(format!("OUT.LC{i}.{pin}"), WireKind::LogicOut);
        }
    }
    for i in 0..4 {
        db.wires.insert(format!("OUT.TBUF{i}"), WireKind::LogicOut);
    }
    db.wires.insert("OUT.PWRGND".into(), WireKind::LogicOut);
    for i in 0..4 {
        db.wires.insert(format!("OUT.IO{i}.I"), WireKind::LogicOut);
    }
    db.wires.insert("OUT.CLKIOB".into(), WireKind::LogicOut);
    db.wires.insert("OUT.RDBK.RIP".into(), WireKind::LogicOut);
    db.wires.insert("OUT.RDBK.DATA".into(), WireKind::LogicOut);
    db.wires
        .insert("OUT.STARTUP.DONEIN".into(), WireKind::LogicOut);
    db.wires
        .insert("OUT.STARTUP.Q1Q4".into(), WireKind::LogicOut);
    db.wires.insert("OUT.STARTUP.Q2".into(), WireKind::LogicOut);
    db.wires.insert("OUT.STARTUP.Q3".into(), WireKind::LogicOut);
    db.wires.insert("OUT.BSCAN.DRCK".into(), WireKind::LogicOut);
    db.wires.insert("OUT.BSCAN.IDLE".into(), WireKind::LogicOut);
    db.wires
        .insert("OUT.BSCAN.RESET".into(), WireKind::LogicOut);
    db.wires.insert("OUT.BSCAN.SEL1".into(), WireKind::LogicOut);
    db.wires.insert("OUT.BSCAN.SEL2".into(), WireKind::LogicOut);
    db.wires
        .insert("OUT.BSCAN.SHIFT".into(), WireKind::LogicOut);
    db.wires
        .insert("OUT.BSCAN.UPDATE".into(), WireKind::LogicOut);
    db.wires.insert("OUT.BSUPD".into(), WireKind::LogicOut);
    db.wires.insert("OUT.OSC.OSC1".into(), WireKind::LogicOut);
    db.wires.insert("OUT.OSC.OSC2".into(), WireKind::LogicOut);
    db.wires.insert("OUT.TOP.COUT".into(), WireKind::LogicOut);

    for i in 0..4 {
        for pin in ["F1", "F2", "F3", "F4", "DI"] {
            db.wires
                .insert(format!("IMUX.LC{i}.{pin}"), WireKind::MuxOut);
        }
    }
    for pin in ["CE", "CLK", "RST"] {
        db.wires.insert(format!("IMUX.CLB.{pin}"), WireKind::MuxOut);
    }
    db.wires.insert("IMUX.TS".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.GIN".into(), WireKind::MuxOut);
    for i in 0..4 {
        for pin in ["T", "O"] {
            db.wires
                .insert(format!("IMUX.IO{i}.{pin}"), WireKind::MuxOut);
        }
    }
    db.wires.insert("IMUX.RDBK.RCLK".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.RDBK.TRIG".into(), WireKind::MuxOut);
    db.wires
        .insert("IMUX.STARTUP.SCLK".into(), WireKind::MuxOut);
    db.wires
        .insert("IMUX.STARTUP.GRST".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.STARTUP.GTS".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.BSCAN.TDO1".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.BSCAN.TDO2".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.OSC.OCLK".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.BYPOSC.PUMP".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.BUFG".into(), WireKind::MuxOut);
    db.wires.insert("IMUX.BOT.CIN".into(), WireKind::MuxOut);

    let mut ll_terms = main_terms.clone();
    for term in ll_terms.values_mut() {
        for (w, name, _) in &db.wires {
            if name.starts_with("LONG") {
                term.wires.remove(w);
            }
        }
    }

    db.conn_classes.insert("CNR.LL".into(), cnr_ll_w);
    db.conn_classes.insert("CNR.LR".into(), cnr_lr_s);
    db.conn_classes.insert("CNR.UL".into(), cnr_ul_n);
    db.conn_classes.insert("CNR.UR".into(), cnr_ur_e);
    for (dir, term) in main_terms {
        db.conn_classes.insert(format!("MAIN.{dir}"), term);
    }
    for (dir, term) in ll_terms {
        let hv = match dir {
            Dir::W | Dir::E => 'H',
            Dir::S | Dir::N => 'V',
        };
        db.conn_classes.insert(format!("LL{hv}.{dir}"), term);
    }

    {
        let mut tcls = TileClass::new(tslots::MAIN, 1);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));

        for i in 0..4 {
            tcls.bels.insert(
                bels::LC[i],
                bel_from_pins(
                    &db,
                    &[
                        ("F1", format!("IMUX.LC{i}.F1")),
                        ("F2", format!("IMUX.LC{i}.F2")),
                        ("F3", format!("IMUX.LC{i}.F3")),
                        ("F4", format!("IMUX.LC{i}.F4")),
                        ("DI", format!("IMUX.LC{i}.DI")),
                        ("CE", "IMUX.CLB.CE".to_string()),
                        ("CK", "IMUX.CLB.CLK".to_string()),
                        ("CLR", "IMUX.CLB.RST".to_string()),
                        ("X", format!("OUT.LC{i}.X")),
                        ("Q", format!("OUT.LC{i}.Q")),
                        ("DO", format!("OUT.LC{i}.DO")),
                    ],
                ),
            );
        }
        for i in 0..4 {
            tcls.bels.insert(
                bels::TBUF[i],
                bel_from_pins(
                    &db,
                    &[
                        ("I", format!("OMUX{ii}.BUF", ii = i + 4)),
                        ("O", format!("OUT.TBUF{i}")),
                        ("T", "IMUX.TS".to_string()),
                    ],
                ),
            );
        }
        tcls.bels
            .insert(bels::VCC_GND, bel_from_pins(&db, &[("O", "OUT.PWRGND")]));
        db.tile_classes.insert("CLB".into(), tcls);
    }

    for (name, gout) in [
        ("IO.L", "GLOBAL.L"),
        ("IO.R", "GLOBAL.R"),
        ("IO.B", "GLOBAL.B"),
        ("IO.T", "GLOBAL.T"),
    ] {
        let mut tcls = TileClass::new(tslots::MAIN, 1);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));

        for i in 0..4 {
            tcls.bels.insert(
                bels::IO[i],
                bel_from_pins(
                    &db,
                    &[
                        ("O", format!("IMUX.IO{i}.O")),
                        ("T", format!("IMUX.IO{i}.T")),
                        ("I", format!("OUT.IO{i}.I")),
                    ],
                ),
            );
        }
        for i in 0..4 {
            tcls.bels.insert(
                bels::TBUF[i],
                bel_from_pins(
                    &db,
                    &[
                        ("I", format!("OMUX{i}.BUF")),
                        ("O", format!("OUT.TBUF{i}")),
                        ("T", "IMUX.TS".to_string()),
                    ],
                ),
            );
        }
        tcls.bels.insert(
            bels::BUFR,
            bel_from_pins(&db, &[("IN", "IMUX.GIN"), ("OUT", gout)]),
        );
        if name == "IO.B" {
            tcls.bels
                .insert(bels::CIN, bel_from_pins(&db, &[("IN", "IMUX.BOT.CIN")]));
            tcls.bels
                .insert(bels::SCANTEST, BelInfo::Bel(Bel::default()));
        }
        if name == "IO.T" {
            tcls.bels
                .insert(bels::COUT, bel_from_pins(&db, &[("OUT", "OUT.TOP.COUT")]));
        }
        db.tile_classes.insert(name.into(), tcls);
    }
    for (name, gout) in [
        ("CNR.BL", "GLOBAL.BL"),
        ("CNR.BR", "GLOBAL.BR"),
        ("CNR.TL", "GLOBAL.TL"),
        ("CNR.TR", "GLOBAL.TR"),
    ] {
        let mut tcls = TileClass::new(tslots::MAIN, 1);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));

        tcls.bels.insert(
            bels::BUFG,
            bel_from_pins(&db, &[("I", "IMUX.BUFG"), ("O", gout)]),
        );
        tcls.bels
            .insert(bels::CLKIOB, bel_from_pins(&db, &[("OUT", "OUT.CLKIOB")]));
        match name {
            "CNR.BL" => {
                tcls.bels.insert(
                    bels::RDBK,
                    bel_from_pins(
                        &db,
                        &[
                            ("CK", "IMUX.RDBK.RCLK"),
                            ("TRIG", "IMUX.RDBK.TRIG"),
                            ("DATA", "OUT.RDBK.DATA"),
                            ("RIP", "OUT.RDBK.RIP"),
                        ],
                    ),
                );
            }
            "CNR.BR" => {
                tcls.bels.insert(
                    bels::STARTUP,
                    bel_from_pins(
                        &db,
                        &[
                            ("CLK", "IMUX.STARTUP.SCLK"),
                            ("GR", "IMUX.STARTUP.GRST"),
                            ("GTS", "IMUX.STARTUP.GTS"),
                            ("DONEIN", "OUT.STARTUP.DONEIN"),
                            ("Q1Q4", "OUT.STARTUP.Q1Q4"),
                            ("Q2", "OUT.STARTUP.Q2"),
                            ("Q3", "OUT.STARTUP.Q3"),
                        ],
                    ),
                );
            }
            "CNR.TL" => {
                tcls.bels.insert(
                    bels::BSCAN,
                    bel_from_pins(
                        &db,
                        &[
                            ("TDO1", "IMUX.BSCAN.TDO1"),
                            ("TDO2", "IMUX.BSCAN.TDO2"),
                            ("DRCK", "OUT.BSCAN.DRCK"),
                            ("IDLE", "OUT.BSCAN.IDLE"),
                            ("RESET", "OUT.BSCAN.RESET"),
                            ("SEL1", "OUT.BSCAN.SEL1"),
                            ("SEL2", "OUT.BSCAN.SEL2"),
                            ("SHIFT", "OUT.BSCAN.SHIFT"),
                            ("UPDATE", "OUT.BSCAN.UPDATE"),
                        ],
                    ),
                );
            }
            "CNR.TR" => {
                tcls.bels.insert(
                    bels::OSC,
                    bel_from_pins(
                        &db,
                        &[
                            ("C", "IMUX.OSC.OCLK"),
                            ("OSC1", "OUT.OSC.OSC1"),
                            ("OSC2", "OUT.OSC.OSC2"),
                        ],
                    ),
                );
                tcls.bels.insert(
                    bels::BYPOSC,
                    bel_from_pins(&db, &[("I", "IMUX.BYPOSC.PUMP")]),
                );
                tcls.bels
                    .insert(bels::BSUPD, bel_from_pins(&db, &[("O", "OUT.BSUPD")]));
            }
            _ => unreachable!(),
        }
        db.tile_classes.insert(name.into(), tcls);
    }
    for (name, slot, sbslot) in [
        ("CLKV", tslots::EXTRA_COL, bels::LLH),
        ("CLKB", tslots::EXTRA_COL, bels::LLH),
        ("CLKT", tslots::EXTRA_COL, bels::LLH),
        ("CLKH", tslots::EXTRA_ROW, bels::LLV),
        ("CLKL", tslots::EXTRA_ROW, bels::LLV),
        ("CLKR", tslots::EXTRA_ROW, bels::LLV),
    ] {
        let mut tcls = TileClass::new(slot, 2);
        tcls.bels
            .insert(sbslot, BelInfo::SwitchBox(Default::default()));

        db.tile_classes.insert(name.into(), tcls);
    }

    db
}

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
    let mut intdb = make_intdb();
    let mut ndb = NamingDb::default();
    for name in intdb.tile_classes.keys() {
        ndb.node_namings.insert(name.clone(), NodeNaming::default());
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

    let mut extractor = Extractor::new(die, &edev.egrid, &endev.ngrid);

    let die = DieId::from_idx(0);
    for (tcrd, tile) in edev.egrid.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let node_kind = &intdb.tile_classes[tile.class];
        let nnode = &endev.ngrid.tiles[&tcrd];
        if !nnode.tie_names.is_empty() {
            let mut tie = extractor.grab_prim_a(&nnode.tie_names[0]);
            let o = tie.get_pin("O");
            extractor.net_int(o, cell.wire(intdb.get_wire("GND")));
            let mut dummy = extractor.grab_prim_a(&nnode.tie_names[1]);
            let i = dummy.get_pin("I");
            let wire = if col == chip.col_w() {
                if row == chip.row_s() {
                    "GLOBAL.BR"
                } else if row == chip.row_n() {
                    "GLOBAL.BL"
                } else {
                    "GLOBAL.R"
                }
            } else if col == chip.col_e() {
                if row == chip.row_s() {
                    "GLOBAL.TR"
                } else if row == chip.row_n() {
                    "GLOBAL.TL"
                } else {
                    "GLOBAL.L"
                }
            } else {
                if row == chip.row_s() {
                    "GLOBAL.T"
                } else if row == chip.row_n() {
                    "GLOBAL.B"
                } else {
                    unreachable!()
                }
            };
            let wire = cell.wire(intdb.get_wire(wire));
            extractor.net_dummy(i);
            let (line, _) = extractor.consume_one_bwd(i, tcrd);
            extractor.net_int(line, wire);
            if nnode.tie_names.len() > 2 {
                // SCANTEST
                extractor.grab_prim_ab(&nnode.tie_names[2], &nnode.tie_names[3]);
            }
        }
        for (slot, bel_info) in &node_kind.bels {
            let BelInfo::Bel(bel_info) = bel_info else {
                continue;
            };
            let bel = cell.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);
            match slot {
                bels::BUFG | bels::RDBK | bels::BSCAN => {
                    let mut prim = extractor.grab_prim_a(&nnode.bels[slot][0]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                }
                bels::STARTUP => {
                    let mut prim = extractor.grab_prim_a(&nnode.bels[slot][0]);
                    for pin in ["DONEIN", "Q1Q4", "Q2", "Q3", "GTS"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    extractor.net_bel_int(prim.get_pin("CK"), bel, "CLK");
                    extractor.net_bel_int(prim.get_pin("GCLR"), bel, "GR");
                }
                bels::OSC => {
                    let mut prim = extractor.grab_prim_a(&nnode.bels[slot][0]);
                    for pin in ["OSC1", "OSC2"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    extractor.net_bel_int(prim.get_pin("CK"), bel, "C");
                    extractor.net_bel_int(prim.get_pin("BSUPD"), cell.bel(bels::BSUPD), "O");
                }
                bels::BYPOSC => {
                    // ???
                }
                bels::BSUPD => {
                    // handled with OSC
                }
                bels::CLKIOB => {
                    let mut prim = extractor.grab_prim_a(&nnode.bels[slot][0]);
                    extractor.net_bel_int(prim.get_pin("I"), bel, "OUT");
                }
                bels::IO0 | bels::IO1 | bels::IO2 | bels::IO3 => {
                    let mut prim = extractor.grab_prim_i(&nnode.bels[slot][0]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                }
                bels::TBUF0 | bels::TBUF1 | bels::TBUF2 | bels::TBUF3 => {
                    let mut prim =
                        extractor.grab_prim_ab(&nnode.bels[slot][0], &nnode.bels[slot][1]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (net_o, pip) = extractor.consume_one_fwd(o, tcrd);
                    extractor.net_bel_int(net_o, bel, "O");
                    extractor.bel_pip(nnode.naming, slot, "O", pip);
                    let i = prim.get_pin("I");
                    extractor.net_bel(i, bel, "I");
                    let (net_i, pip) = extractor.consume_one_bwd(i, tcrd);
                    extractor.net_bel_int(net_i, bel, "I");
                    extractor.bel_pip(nnode.naming, slot, "I", pip);
                    let t = prim.get_pin("T");
                    extractor.net_bel(t, bel, "T");
                    let (net_t, pip) = extractor.consume_one_bwd(t, tcrd);
                    extractor.net_bel_int(net_t, bel, "T");
                    extractor.bel_pip(nnode.naming, slot, "T", pip);
                    extractor.mark_tbuf_pseudo(net_o, net_i);

                    let wib = bel_info.pins["I"].wires.iter().next().unwrap().wire;
                    let wi = intdb.get_wire(intdb.wires.key(wib).strip_suffix(".BUF").unwrap());
                    assert_eq!(extractor.nets[net_i].pips_bwd.len(), 1);
                    let net_omux = *extractor.nets[net_i].pips_bwd.iter().next().unwrap().0;
                    extractor.net_int(net_omux, cell.wire(wi));
                }
                bels::BUFR | bels::CIN | bels::COUT => {
                    // handled later
                }
                bels::LC0 => {
                    let mut prim =
                        extractor.grab_prim_ab(&nnode.bels[slot][0], &nnode.bels[slot][1]);
                    for pin in ["CE", "CK", "CLR"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    let cv = prim.get_pin("CV");
                    extractor.net_bel_int(cv, cell.bel(bels::VCC_GND), "O");
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
                        extractor.net_int(net, cell.wire(intdb.get_wire(&format!("OMUX{i}"))));
                        assert_eq!(extractor.nets[net].pips_fwd.len(), 1);
                        let (&net_buf, _) = extractor.nets[net].pips_fwd.iter().next().unwrap();
                        extractor
                            .net_int(net_buf, cell.wire(intdb.get_wire(&format!("OMUX{i}.BUF"))));
                    }
                    for (i, pin, spin) in [
                        (0, "F1", "LC0.F1"),
                        (0, "F2", "LC0.F2"),
                        (0, "F3", "LC0.F3"),
                        (0, "F4", "LC0.F4"),
                        (0, "DI", "LC0.DI"),
                        (0, "DO", "LC0.DO"),
                        (0, "X", "LC0.X"),
                        (0, "Q", "LC0.Q"),
                        (1, "F1", "LC1.F1"),
                        (1, "F2", "LC1.F2"),
                        (1, "F3", "LC1.F3"),
                        (1, "F4", "LC1.F4"),
                        (1, "DI", "LC1.DI"),
                        (1, "DO", "LC1.DO"),
                        (1, "X", "LC1.X"),
                        (1, "Q", "LC1.Q"),
                        (2, "F1", "LC2.F1"),
                        (2, "F2", "LC2.F2"),
                        (2, "F3", "LC2.F3"),
                        (2, "F4", "LC2.F4"),
                        (2, "DI", "LC2.DI"),
                        (2, "DO", "LC2.DO"),
                        (2, "X", "LC2.X"),
                        (2, "Q", "LC2.Q"),
                        (3, "F1", "LC3.F1"),
                        (3, "F2", "LC3.F2"),
                        (3, "F3", "LC3.F3"),
                        (3, "F4", "LC3.F4"),
                        (3, "DI", "LC3.DI"),
                        (3, "DO", "LC3.DO"),
                        (3, "X", "LC3.X"),
                        (3, "Q", "LC3.Q"),
                    ] {
                        extractor.net_bel_int(prim.get_pin(spin), cell.bel(bels::LC[i]), pin);
                    }
                    let ci = prim.get_pin("CI");
                    extractor.net_bel(ci, bel, "CI");
                    let co = prim.get_pin("CO");
                    if row == chip.row_n() - 1 {
                        extractor.net_bel_int(co, cell.delta(0, 1).bel(bels::COUT), "OUT");
                    } else {
                        extractor.net_bel(co, bel, "CO");
                    }
                    let (co_b, pip) = extractor.consume_one_bwd(ci, tcrd);
                    extractor.bel_pip(nnode.naming, slot, "CI", pip);
                    if row == chip.row_s() + 1 {
                        extractor.net_bel_int(co_b, cell.delta(0, -1).bel(bels::CIN), "IN");
                    } else {
                        extractor.net_bel(co_b, cell.delta(0, -1).bel(bels::LC0), "CO");
                    }
                }
                bels::LC1 | bels::LC2 | bels::LC3 | bels::VCC_GND => {
                    // handled with LC0
                }
                bels::SCANTEST => {
                    extractor.grab_prim_ab(&nnode.bels[slot][0], &nnode.bels[slot][1]);
                }

                _ => panic!("umm bel {slot_name}?"),
            }
        }
    }
    extractor.grab_prim_a("_cfg5200_");

    for (tcrd, tile) in edev.egrid.tiles() {
        let nnode = &endev.ngrid.tiles[&tcrd];
        let node_kind = &intdb.tile_classes[tile.class];
        for (slot, _) in &node_kind.bels {
            if slot == bels::BUFR {
                let bel = tcrd.bel(slot);
                let net = extractor.get_bel_int_net(bel, "OUT");
                let (imux, pip) = extractor.consume_one_bwd(net, tcrd);
                extractor.net_bel_int(imux, bel, "IN");
                extractor.bel_pip(nnode.naming, slot, "BUF", pip);
            }
        }
    }

    // long verticals + GCLK
    for col in edev.egrid.cols(die) {
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
                let wire = intdb.get_wire(&format!("LONG.V{i}"));
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
                let wire = intdb.get_wire(&format!("LONG.H{i}"));
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // horizontal single and double
    let mut queue = vec![];
    for col in edev.egrid.cols(die) {
        if col == chip.col_w() {
            continue;
        }
        let x = endev.col_x[col].start;
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
                    "IO.SINGLE.B.W0",
                    "IO.SINGLE.B.W1",
                    "IO.SINGLE.B.W2",
                    "IO.SINGLE.B.W3",
                    "IO.SINGLE.B.W4",
                    "IO.SINGLE.B.W5",
                    "IO.SINGLE.B.W6",
                    "IO.SINGLE.B.W7",
                ][..]
            } else if row == chip.row_n() {
                &[
                    "IO.SINGLE.T.W0",
                    "IO.SINGLE.T.W1",
                    "IO.SINGLE.T.W2",
                    "IO.SINGLE.T.W3",
                    "IO.SINGLE.T.W4",
                    "IO.SINGLE.T.W5",
                    "IO.SINGLE.T.W6",
                    "IO.SINGLE.T.W7",
                ][..]
            } else {
                &[
                    "SINGLE.W11",
                    "SINGLE.W10",
                    "SINGLE.W9",
                    "SINGLE.W8",
                    "SINGLE.W7",
                    "SINGLE.W5",
                    "SINGLE.W4",
                    "SINGLE.W3",
                    "SINGLE.W2",
                    "SINGLE.W1",
                    "DBL.H0.M",
                    "DBL.H6.M",
                    "DBL.H0.E",
                    "DBL.H6.E",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
            }
        }
    }
    // vertical single and double
    for row in edev.egrid.rows(die) {
        if row == chip.row_s() {
            continue;
        }
        let y = endev.row_y[row].start;
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
                    "IO.SINGLE.L.S7",
                    "IO.SINGLE.L.S6",
                    "IO.SINGLE.L.S5",
                    "IO.SINGLE.L.S4",
                    "IO.SINGLE.L.S3",
                    "IO.SINGLE.L.S2",
                    "IO.SINGLE.L.S1",
                    "IO.SINGLE.L.S0",
                ][..]
            } else if col == chip.col_e() {
                &[
                    "IO.SINGLE.R.S7",
                    "IO.SINGLE.R.S6",
                    "IO.SINGLE.R.S5",
                    "IO.SINGLE.R.S4",
                    "IO.SINGLE.R.S3",
                    "IO.SINGLE.R.S2",
                    "IO.SINGLE.R.S1",
                    "IO.SINGLE.R.S0",
                ][..]
            } else {
                &[
                    "DBL.V6.M",
                    "DBL.V6.N",
                    "DBL.V0.N",
                    "DBL.V0.M",
                    "SINGLE.S11",
                    "SINGLE.S10",
                    "SINGLE.S9",
                    "SINGLE.S8",
                    "SINGLE.S7",
                    "SINGLE.S5",
                    "SINGLE.S4",
                    "SINGLE.S3",
                    "SINGLE.S2",
                    "SINGLE.S1",
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

    for (cell, _) in edev.egrid.cells() {
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
                extractor.net_int(net, cell.wire(intdb.get_wire(&format!("IO.M{i}"))));
                extractor.net_int(net_b, cell.wire(intdb.get_wire(&format!("IO.M{i}.BUF"))))
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
                extractor.net_int(net, cell.wire(intdb.get_wire(&format!("IO.M{i}"))));
                extractor.net_int(net_b, cell.wire(intdb.get_wire(&format!("IO.M{i}.BUF"))))
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
                extractor.net_int(net, cell.wire(intdb.get_wire(&format!("CLB.M{i}"))));
                extractor.net_int(net_b, cell.wire(intdb.get_wire(&format!("CLB.M{i}.BUF"))))
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
            if w_f == "IMUX.BOT.CIN" {
                queue.push((net_t, net_f));
            }
        }
    }
    for (net_t, net_f) in queue {
        extractor.mark_tbuf_pseudo(net_t, net_f);
    }

    for i in 0..8 {
        let wire = intdb.get_wire(&format!("IO.SINGLE.L.N{i}"));
        let rw = edev
            .egrid
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
    finisher.finish(&mut intdb, &mut ndb, |db, _, wt, _| {
        let wtn = db.wires.key(wt.wire);
        if wtn.ends_with(".BUF") {
            PipMode::PermaBuf
        } else if wtn.starts_with("IMUX") || wtn.starts_with("OMUX") {
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
