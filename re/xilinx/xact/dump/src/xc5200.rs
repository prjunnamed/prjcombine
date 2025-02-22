use std::collections::{BTreeMap, BTreeSet};

use enum_map::EnumMap;
use prjcombine_interconnect::{
    db::{
        BelId, BelInfo, BelPin, Dir, IntDb, NodeKind, NodeTileId, PinDir, TermInfo, TermKind,
        TermSlotId, TermSlotInfo, WireKind,
    },
    grid::{DieId, EdgeIoCoord, LayerId},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, NodeNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bond::{Bond, BondPin, CfgPin},
    chip::{Chip, ChipKind, SharedCfgPin},
};
use unnamed_entity::{EntityId, EntityVec};

use crate::extractor::{Extractor, NetBinding};

fn bel_from_pins(db: &IntDb, pins: &[(&str, impl AsRef<str>)]) -> BelInfo {
    let mut bel = BelInfo::default();
    for &(name, ref wire) in pins {
        let wire = wire.as_ref();
        bel.pins.insert(
            name.into(),
            BelPin {
                wires: BTreeSet::from_iter([(NodeTileId::from_idx(0), db.get_wire(wire))]),
                dir: if wire.starts_with("IMUX") || wire.starts_with("OMUX") {
                    PinDir::Input
                } else {
                    PinDir::Output
                },
                is_intf_in: false,
            },
        );
    }
    bel
}

pub fn make_intdb() -> IntDb {
    let mut db = IntDb::default();

    let slot_w = db
        .term_slots
        .insert(
            "W".into(),
            TermSlotInfo {
                opposite: TermSlotId::from_idx(0),
            },
        )
        .0;
    let slot_e = db
        .term_slots
        .insert("E".into(), TermSlotInfo { opposite: slot_w })
        .0;
    let slot_s = db
        .term_slots
        .insert(
            "S".into(),
            TermSlotInfo {
                opposite: TermSlotId::from_idx(0),
            },
        )
        .0;
    let slot_n = db
        .term_slots
        .insert("N".into(), TermSlotInfo { opposite: slot_s })
        .0;
    db.term_slots[slot_w].opposite = slot_e;
    db.term_slots[slot_s].opposite = slot_n;

    let term_slots = EnumMap::from_fn(|dir| match dir {
        Dir::W => slot_w,
        Dir::E => slot_e,
        Dir::S => slot_s,
        Dir::N => slot_n,
    });

    let mut main_terms = EnumMap::from_fn(|dir| TermKind {
        slot: term_slots[dir],
        wires: Default::default(),
    });
    let mut cnr_ll_w = TermKind {
        slot: slot_w,
        wires: Default::default(),
    };
    let mut cnr_lr_s = TermKind {
        slot: slot_s,
        wires: Default::default(),
    };
    let mut cnr_ul_n = TermKind {
        slot: slot_n,
        wires: Default::default(),
    };
    let mut cnr_ur_e = TermKind {
        slot: slot_e,
        wires: Default::default(),
    };

    db.wires.insert("GND".into(), WireKind::Tie0);

    for i in 0..24 {
        let w = db.wires.insert(format!("CLB.M{i}"), WireKind::PipOut).0;
        db.wires.insert(format!("CLB.M{i}.BUF"), WireKind::Buf(w));
    }
    for i in 0..16 {
        let w = db.wires.insert(format!("IO.M{i}"), WireKind::PipOut).0;
        db.wires.insert(format!("IO.M{i}.BUF"), WireKind::Buf(w));
    }

    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w0 = db.wires.insert(format!("SINGLE.E{i}"), WireKind::PipOut).0;
        let w1 = db
            .wires
            .insert(format!("SINGLE.W{i}"), WireKind::PipBranch(slot_w))
            .0;
        main_terms[Dir::W].wires.insert(w1, TermInfo::PassFar(w0));
    }
    for i in 0..12 {
        if matches!(i, 0 | 6) {
            continue;
        }
        let w0 = db.wires.insert(format!("SINGLE.S{i}"), WireKind::PipOut).0;
        let w1 = db
            .wires
            .insert(format!("SINGLE.N{i}"), WireKind::PipBranch(slot_n))
            .0;
        main_terms[Dir::N].wires.insert(w1, TermInfo::PassFar(w0));
    }

    for i in 0..8 {
        let w_be = db
            .wires
            .insert(format!("IO.SINGLE.B.E{i}"), WireKind::PipBranch(slot_w))
            .0;
        let w_bw = db
            .wires
            .insert(format!("IO.SINGLE.B.W{i}"), WireKind::PipBranch(slot_w))
            .0;
        main_terms[Dir::W]
            .wires
            .insert(w_bw, TermInfo::PassFar(w_be));
        let w_rn = db
            .wires
            .insert(format!("IO.SINGLE.R.N{i}"), WireKind::PipBranch(slot_s))
            .0;
        let w_rs = db
            .wires
            .insert(format!("IO.SINGLE.R.S{i}"), WireKind::PipBranch(slot_s))
            .0;
        main_terms[Dir::S]
            .wires
            .insert(w_rs, TermInfo::PassFar(w_rn));
        let w_tw = db
            .wires
            .insert(format!("IO.SINGLE.T.W{i}"), WireKind::PipBranch(slot_e))
            .0;
        let w_te = db
            .wires
            .insert(format!("IO.SINGLE.T.E{i}"), WireKind::PipBranch(slot_e))
            .0;
        main_terms[Dir::E]
            .wires
            .insert(w_te, TermInfo::PassFar(w_tw));
        let w_ls = db
            .wires
            .insert(format!("IO.SINGLE.L.S{i}"), WireKind::PipBranch(slot_n))
            .0;
        let w_ln = db
            .wires
            .insert(format!("IO.SINGLE.L.N{i}"), WireKind::PipBranch(slot_n))
            .0;
        main_terms[Dir::N]
            .wires
            .insert(w_ln, TermInfo::PassFar(w_ls));
        cnr_ll_w.wires.insert(w_be, TermInfo::PassNear(w_ln));
        cnr_lr_s.wires.insert(w_rn, TermInfo::PassNear(w_bw));
        cnr_ul_n.wires.insert(w_ls, TermInfo::PassNear(w_te));
        cnr_ur_e.wires.insert(w_tw, TermInfo::PassNear(w_rs));
    }

    for i in [0, 6] {
        let w = db.wires.insert(format!("DBL.H{i}.M"), WireKind::PipOut).0;
        let ww = db
            .wires
            .insert(format!("DBL.H{i}.W"), WireKind::PipBranch(slot_e))
            .0;
        main_terms[Dir::E].wires.insert(ww, TermInfo::PassFar(w));
        let we = db
            .wires
            .insert(format!("DBL.H{i}.E"), WireKind::PipBranch(slot_w))
            .0;
        main_terms[Dir::W].wires.insert(we, TermInfo::PassFar(w));
    }
    for i in [0, 6] {
        let w = db.wires.insert(format!("DBL.V{i}.M"), WireKind::PipOut).0;
        let ws = db
            .wires
            .insert(format!("DBL.V{i}.S"), WireKind::PipBranch(slot_n))
            .0;
        main_terms[Dir::N].wires.insert(ws, TermInfo::PassFar(w));
        let wn = db
            .wires
            .insert(format!("DBL.V{i}.N"), WireKind::PipBranch(slot_s))
            .0;
        main_terms[Dir::S].wires.insert(wn, TermInfo::PassFar(w));
    }

    for i in 0..8 {
        let w = db
            .wires
            .insert(format!("LONG.H{i}"), WireKind::MultiBranch(slot_w))
            .0;
        main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    }
    for i in 0..8 {
        let w = db
            .wires
            .insert(format!("LONG.V{i}"), WireKind::MultiBranch(slot_s))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    }

    let w = db
        .wires
        .insert("GLOBAL.L".into(), WireKind::Branch(slot_w))
        .0;
    main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    let w = db
        .wires
        .insert("GLOBAL.R".into(), WireKind::Branch(slot_e))
        .0;
    main_terms[Dir::E].wires.insert(w, TermInfo::PassFar(w));
    let w = db
        .wires
        .insert("GLOBAL.B".into(), WireKind::Branch(slot_s))
        .0;
    main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    let w = db
        .wires
        .insert("GLOBAL.T".into(), WireKind::Branch(slot_n))
        .0;
    main_terms[Dir::N].wires.insert(w, TermInfo::PassFar(w));

    let w = db
        .wires
        .insert("GLOBAL.TL".into(), WireKind::Branch(slot_w))
        .0;
    main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    let w = db
        .wires
        .insert("GLOBAL.BR".into(), WireKind::Branch(slot_e))
        .0;
    main_terms[Dir::E].wires.insert(w, TermInfo::PassFar(w));
    let w = db
        .wires
        .insert("GLOBAL.BL".into(), WireKind::Branch(slot_s))
        .0;
    main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    let w = db
        .wires
        .insert("GLOBAL.TR".into(), WireKind::Branch(slot_n))
        .0;
    main_terms[Dir::N].wires.insert(w, TermInfo::PassFar(w));

    for i in 0..8 {
        // only 4 of these outside CLB
        let w0 = db.wires.insert(format!("OMUX{i}"), WireKind::MuxOut).0;
        let w = db.wires.insert(format!("OMUX{i}.BUF"), WireKind::Buf(w0)).0;
        if i < 4 {
            let ww = db
                .wires
                .insert(format!("OMUX{i}.BUF.W"), WireKind::Branch(slot_e))
                .0;
            main_terms[Dir::E].wires.insert(ww, TermInfo::PassFar(w));
            let we = db
                .wires
                .insert(format!("OMUX{i}.BUF.E"), WireKind::Branch(slot_w))
                .0;
            main_terms[Dir::W].wires.insert(we, TermInfo::PassFar(w));
            let ws = db
                .wires
                .insert(format!("OMUX{i}.BUF.S"), WireKind::Branch(slot_n))
                .0;
            main_terms[Dir::N].wires.insert(ws, TermInfo::PassFar(w));
            let wn = db
                .wires
                .insert(format!("OMUX{i}.BUF.N"), WireKind::Branch(slot_s))
                .0;
            main_terms[Dir::S].wires.insert(wn, TermInfo::PassFar(w));
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

    db.terms.insert("CNR.LL".into(), cnr_ll_w);
    db.terms.insert("CNR.LR".into(), cnr_lr_s);
    db.terms.insert("CNR.UL".into(), cnr_ul_n);
    db.terms.insert("CNR.UR".into(), cnr_ur_e);
    for (dir, term) in main_terms {
        db.terms.insert(format!("MAIN.{dir}"), term);
    }
    for (dir, term) in ll_terms {
        let hv = match dir {
            Dir::W | Dir::E => 'H',
            Dir::S | Dir::N => 'V',
        };
        db.terms.insert(format!("LL{hv}.{dir}"), term);
    }

    {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        for i in 0..4 {
            node.bels.insert(
                format!("LC{i}"),
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
            node.bels.insert(
                format!("TBUF{i}"),
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
        node.bels
            .insert("VCC_GND".into(), bel_from_pins(&db, &[("O", "OUT.PWRGND")]));
        db.nodes.insert("CLB".into(), node);
    }

    for (name, gout) in [
        ("IO.L", "GLOBAL.L"),
        ("IO.R", "GLOBAL.R"),
        ("IO.B", "GLOBAL.B"),
        ("IO.T", "GLOBAL.T"),
    ] {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        for i in 0..4 {
            node.bels.insert(
                format!("IOB{i}"),
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
            node.bels.insert(
                format!("TBUF{i}"),
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
        node.bels.insert(
            "BUFR".into(),
            bel_from_pins(&db, &[("IN", "IMUX.GIN"), ("OUT", gout)]),
        );
        if name == "IO.B" {
            node.bels.insert(
                "BOT_CIN".into(),
                bel_from_pins(&db, &[("IN", "IMUX.BOT.CIN")]),
            );
            node.bels.insert("SCANTEST".into(), BelInfo::default());
        }
        if name == "IO.T" {
            node.bels.insert(
                "TOP_COUT".into(),
                bel_from_pins(&db, &[("OUT", "OUT.TOP.COUT")]),
            );
        }
        db.nodes.insert(name.into(), node);
    }
    for (name, gout) in [
        ("CNR.BL", "GLOBAL.BL"),
        ("CNR.BR", "GLOBAL.BR"),
        ("CNR.TL", "GLOBAL.TL"),
        ("CNR.TR", "GLOBAL.TR"),
    ] {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        node.bels.insert(
            "BUFG".into(),
            bel_from_pins(&db, &[("I", "IMUX.BUFG"), ("O", gout)]),
        );
        node.bels.insert(
            "CLKIOB".into(),
            bel_from_pins(&db, &[("OUT", "OUT.CLKIOB")]),
        );
        match name {
            "CNR.BL" => {
                node.bels.insert(
                    "RDBK".into(),
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
                node.bels.insert(
                    "STARTUP".into(),
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
                node.bels.insert(
                    "BSCAN".into(),
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
                node.bels.insert(
                    "OSC".into(),
                    bel_from_pins(
                        &db,
                        &[
                            ("C", "IMUX.OSC.OCLK"),
                            ("OSC1", "OUT.OSC.OSC1"),
                            ("OSC2", "OUT.OSC.OSC2"),
                        ],
                    ),
                );
                node.bels.insert(
                    "BYPOSC".into(),
                    bel_from_pins(&db, &[("I", "IMUX.BYPOSC.PUMP")]),
                );
                node.bels
                    .insert("BSUPD".into(), bel_from_pins(&db, &[("O", "OUT.BSUPD")]));
            }
            _ => unreachable!(),
        }
        db.nodes.insert(name.into(), node);
    }
    for name in ["CLKV", "CLKB", "CLKT", "CLKH", "CLKL", "CLKR"] {
        let node = NodeKind {
            tiles: EntityVec::from_iter([(), ()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        db.nodes.insert(name.into(), node);
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
    for name in intdb.nodes.keys() {
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

    let die = edev.egrid.die(DieId::from_idx(0));
    for col in die.cols() {
        for row in die.rows() {
            for (layer, node) in &die[(col, row)].nodes {
                let nloc = (die.die, col, row, layer);
                let node_kind = &intdb.nodes[node.kind];
                let nnode = &endev.ngrid.nodes[&nloc];
                if !nnode.tie_names.is_empty() {
                    let mut tie = extractor.grab_prim_a(&nnode.tie_names[0]);
                    let o = tie.get_pin("O");
                    extractor.net_int(o, (DieId::from_idx(0), (col, row), intdb.get_wire("GND")));
                    let mut dummy = extractor.grab_prim_a(&nnode.tie_names[1]);
                    let i = dummy.get_pin("I");
                    let wire = if col == chip.col_lio() {
                        if row == chip.row_bio() {
                            "GLOBAL.BR"
                        } else if row == chip.row_tio() {
                            "GLOBAL.BL"
                        } else {
                            "GLOBAL.R"
                        }
                    } else if col == chip.col_rio() {
                        if row == chip.row_bio() {
                            "GLOBAL.TR"
                        } else if row == chip.row_tio() {
                            "GLOBAL.TL"
                        } else {
                            "GLOBAL.L"
                        }
                    } else {
                        if row == chip.row_bio() {
                            "GLOBAL.T"
                        } else if row == chip.row_tio() {
                            "GLOBAL.B"
                        } else {
                            unreachable!()
                        }
                    };
                    let wire = (die.die, (col, row), intdb.get_wire(wire));
                    extractor.net_dummy(i);
                    let (line, _) = extractor.consume_one_bwd(i, nloc);
                    extractor.net_int(line, wire);
                    if nnode.tie_names.len() > 2 {
                        // SCANTEST
                        extractor.grab_prim_ab(&nnode.tie_names[2], &nnode.tie_names[3]);
                    }
                }
                for (bel, key, bel_info) in &node_kind.bels {
                    match &key[..] {
                        "BUFG" | "RDBK" | "BSCAN" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                        }
                        "STARTUP" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            for pin in ["DONEIN", "Q1Q4", "Q2", "Q3", "GTS"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            extractor.net_bel_int(prim.get_pin("CK"), nloc, bel, "CLK");
                            extractor.net_bel_int(prim.get_pin("GCLR"), nloc, bel, "GR");
                        }
                        "OSC" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            for pin in ["OSC1", "OSC2"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            extractor.net_bel_int(prim.get_pin("CK"), nloc, bel, "C");
                            extractor.net_bel_int(
                                prim.get_pin("BSUPD"),
                                nloc,
                                BelId::from_idx(4),
                                "O",
                            );
                        }
                        "BYPOSC" => {
                            // ???
                        }
                        "BSUPD" => {
                            // handled with OSC
                        }
                        "CLKIOB" => {
                            let mut prim = extractor.grab_prim_a(&nnode.bels[bel][0]);
                            extractor.net_bel_int(prim.get_pin("I"), nloc, bel, "OUT");
                        }
                        "IOB0" | "IOB1" | "IOB2" | "IOB3" => {
                            let mut prim = extractor.grab_prim_i(&nnode.bels[bel][0]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                        }
                        "TBUF0" | "TBUF1" | "TBUF2" | "TBUF3" => {
                            let mut prim =
                                extractor.grab_prim_ab(&nnode.bels[bel][0], &nnode.bels[bel][1]);
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let (net_o, pip) = extractor.consume_one_fwd(o, nloc);
                            extractor.net_bel_int(net_o, nloc, bel, "O");
                            extractor.bel_pip(nnode.naming, bel, "O", pip);
                            let i = prim.get_pin("I");
                            extractor.net_bel(i, nloc, bel, "I");
                            let (net_i, pip) = extractor.consume_one_bwd(i, nloc);
                            extractor.net_bel_int(net_i, nloc, bel, "I");
                            extractor.bel_pip(nnode.naming, bel, "I", pip);
                            let t = prim.get_pin("T");
                            extractor.net_bel(t, nloc, bel, "T");
                            let (net_t, pip) = extractor.consume_one_bwd(t, nloc);
                            extractor.net_bel_int(net_t, nloc, bel, "T");
                            extractor.bel_pip(nnode.naming, bel, "T", pip);
                            extractor.mark_tbuf_pseudo(net_o, net_i);

                            let wib = bel_info.pins["I"].wires.iter().next().unwrap().1;
                            let WireKind::Buf(wi) = intdb.wires[wib] else {
                                unreachable!()
                            };
                            assert_eq!(extractor.nets[net_i].pips_bwd.len(), 1);
                            let net_omux = *extractor.nets[net_i].pips_bwd.iter().next().unwrap().0;
                            extractor.net_int(net_omux, (die.die, (col, row), wi));
                        }
                        "BUFR" | "BOT_CIN" | "TOP_COUT" => {
                            // handled later
                        }
                        "LC0" => {
                            let mut prim =
                                extractor.grab_prim_ab(&nnode.bels[bel][0], &nnode.bels[bel][1]);
                            for pin in ["CE", "CK", "CLR"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            let cv = prim.get_pin("CV");
                            extractor.net_bel_int(cv, nloc, BelId::from_idx(8), "O");
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
                                extractor.net_int(
                                    net,
                                    (die.die, (col, row), intdb.get_wire(&format!("OMUX{i}"))),
                                );
                                assert_eq!(extractor.nets[net].pips_fwd.len(), 1);
                                let (&net_buf, _) =
                                    extractor.nets[net].pips_fwd.iter().next().unwrap();
                                extractor.net_int(
                                    net_buf,
                                    (die.die, (col, row), intdb.get_wire(&format!("OMUX{i}.BUF"))),
                                );
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
                                extractor.net_bel_int(
                                    prim.get_pin(spin),
                                    nloc,
                                    BelId::from_idx(i),
                                    pin,
                                );
                            }
                            let ci = prim.get_pin("CI");
                            extractor.net_bel(ci, nloc, bel, "CI");
                            let co = prim.get_pin("CO");
                            if row == chip.row_tio() - 1 {
                                extractor.net_bel_int(
                                    co,
                                    (die.die, col, row + 1, LayerId::from_idx(0)),
                                    BelId::from_idx(9),
                                    "OUT",
                                );
                            } else {
                                extractor.net_bel(co, nloc, bel, "CO");
                            }
                            let (co_b, pip) = extractor.consume_one_bwd(ci, nloc);
                            extractor.bel_pip(nnode.naming, bel, "CI", pip);
                            if row == chip.row_bio() + 1 {
                                extractor.net_bel_int(
                                    co_b,
                                    (die.die, col, row - 1, LayerId::from_idx(0)),
                                    BelId::from_idx(9),
                                    "IN",
                                );
                            } else {
                                extractor.net_bel(
                                    co_b,
                                    (die.die, col, row - 1, LayerId::from_idx(0)),
                                    bel,
                                    "CO",
                                );
                            }
                        }
                        "LC1" | "LC2" | "LC3" | "VCC_GND" => {
                            // handled with LC0
                        }
                        "SCANTEST" => {
                            extractor.grab_prim_ab(&nnode.bels[bel][0], &nnode.bels[bel][1]);
                        }

                        _ => panic!("umm bel {key}?"),
                    }
                }
            }
        }
    }
    extractor.grab_prim_a("_cfg5200_");

    for col in die.cols() {
        for row in die.rows() {
            for (layer, node) in &die[(col, row)].nodes {
                let nloc = (die.die, col, row, layer);
                let nnode = &endev.ngrid.nodes[&nloc];
                let node_kind = &intdb.nodes[node.kind];
                for (bel, key, _) in &node_kind.bels {
                    if key == "BUFR" {
                        let net = extractor.get_bel_int_net(nloc, bel, "OUT");
                        let (imux, pip) = extractor.consume_one_bwd(net, nloc);
                        extractor.net_bel_int(imux, nloc, bel, "IN");
                        extractor.bel_pip(nnode.naming, bel, "BUF", pip);
                    }
                }
            }
        }
    }

    // long verticals + GCLK
    for col in die.cols() {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }
    // long horizontals
    for row in die.rows() {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // horizontal single and double
    let mut queue = vec![];
    for col in die.cols() {
        if col == chip.col_lio() {
            continue;
        }
        let x = endev.col_x[col].start;
        for row in die.rows() {
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
            let wires = if row == chip.row_bio() {
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
            } else if row == chip.row_tio() {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
    }
    // vertical single and double
    for row in die.rows() {
        if row == chip.row_bio() {
            continue;
        }
        let y = endev.row_y[row].start;
        for col in die.cols() {
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
            let wires = if col == chip.col_lio() {
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
            } else if col == chip.col_rio() {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    for col in die.cols() {
        for row in die.rows() {
            if row == chip.row_bio() || row == chip.row_tio() {
                if col == chip.col_lio() || col == chip.col_rio() {
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
                    extractor.net_int(
                        net,
                        (die.die, (col, row), intdb.get_wire(&format!("IO.M{i}"))),
                    );
                    extractor.net_int(
                        net_b,
                        (die.die, (col, row), intdb.get_wire(&format!("IO.M{i}.BUF"))),
                    )
                }
            } else if col == chip.col_lio() || col == chip.col_rio() {
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
                    extractor.net_int(
                        net,
                        (die.die, (col, row), intdb.get_wire(&format!("IO.M{i}"))),
                    );
                    extractor.net_int(
                        net_b,
                        (die.die, (col, row), intdb.get_wire(&format!("IO.M{i}.BUF"))),
                    )
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
                    extractor.net_int(
                        net,
                        (die.die, (col, row), intdb.get_wire(&format!("CLB.M{i}"))),
                    );
                    extractor.net_int(
                        net_b,
                        (
                            die.die,
                            (col, row),
                            intdb.get_wire(&format!("CLB.M{i}.BUF")),
                        ),
                    )
                }
            }
        }
    }

    let mut queue = vec![];
    for (net_t, net_info) in &extractor.nets {
        let NetBinding::Int(rw_t) = net_info.binding else {
            continue;
        };
        let w_t = intdb.wires.key(rw_t.2);
        for &net_f in net_info.pips_bwd.keys() {
            let NetBinding::Int(rw_f) = extractor.nets[net_f].binding else {
                continue;
            };
            let w_f = intdb.wires.key(rw_f.2);
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
            .resolve_wire((die.die, (chip.col_lio(), chip.row_bio()), wire))
            .unwrap();
        let net = extractor.int_nets[&rw];
        let nbto = extractor
            .net_by_tile_override
            .entry((chip.col_lio(), chip.row_bio()))
            .or_default();
        nbto.insert(net, wire);
    }

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb);
    (chip, intdb, ndb)
}

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    name: &str,
    pkg: &BTreeMap<String, String>,
) -> (Bond, BTreeMap<SharedCfgPin, EdgeIoCoord>) {
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
        bond.pins.insert(pin.into(), BondPin::Io(io));
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
        assert_eq!(bond.pins.insert(pin.to_string(), BondPin::Gnd), None);
    }
    for &pin in vcc {
        assert_eq!(bond.pins.insert(pin.to_string(), BondPin::Vcc), None);
    }
    assert_eq!(
        bond.pins
            .insert(done.to_string(), BondPin::Cfg(CfgPin::Done)),
        None
    );
    assert_eq!(
        bond.pins
            .insert(prog.to_string(), BondPin::Cfg(CfgPin::ProgB)),
        None
    );
    assert_eq!(
        bond.pins
            .insert(cclk.to_string(), BondPin::Cfg(CfgPin::Cclk)),
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
            bond.pins.entry(format!("P{i}")).or_insert(BondPin::Nc);
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
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in [
                "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R", "T", "U", "V", "W", "Y",
                "AA", "AB",
            ] {
                for i in (1..=4).chain(23..=26) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 352);
        }
        "pg156" => {
            for a in ["A", "B", "C", "P", "R", "T"] {
                for i in 1..=16 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L", "M", "N"] {
                for i in (1..=3).chain(14..=16) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 156);
        }
        "pg191" => {
            for i in 2..=18 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "T", "U", "V"] {
                for i in 1..=18 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R"] {
                for i in (1..=3).chain(16..=18) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "R"] {
                for i in [4, 9, 10, 15] {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["J", "K"] {
                for i in [4, 15] {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 191);
        }
        "pg223" => {
            for i in 2..=18 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "D", "R", "T", "U", "V"] {
                for i in 1..=18 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["E", "F", "G", "H", "J", "K", "L", "M", "N", "P"] {
                for i in (1..=4).chain(15..=18) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 223);
        }
        "pg299" => {
            for i in 2..=20 {
                bond.pins.entry(format!("A{i}")).or_insert(BondPin::Nc);
            }
            for a in ["B", "C", "D", "E", "T", "U", "V", "W", "X"] {
                for i in 1..=20 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["F", "G", "H", "J", "K", "L", "M", "N", "P", "R"] {
                for i in (1..=5).chain(16..=20) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 299);
        }
        _ => (),
    }

    let pkg_cfg_io = match name {
        "pc84" => &[
            ("P3", SharedCfgPin::Addr(8)),
            ("P4", SharedCfgPin::Addr(9)),
            ("P5", SharedCfgPin::Addr(10)),
            ("P6", SharedCfgPin::Addr(11)),
            ("P7", SharedCfgPin::Addr(12)),
            ("P8", SharedCfgPin::Addr(13)),
            ("P9", SharedCfgPin::Addr(14)),
            ("P10", SharedCfgPin::Addr(15)),
            ("P13", SharedCfgPin::Addr(16)),
            ("P14", SharedCfgPin::Addr(17)),
            ("P15", SharedCfgPin::Tdi),
            ("P16", SharedCfgPin::Tck),
            ("P17", SharedCfgPin::Tms),
            ("P30", SharedCfgPin::M1),
            ("P32", SharedCfgPin::M0),
            ("P34", SharedCfgPin::M2),
            ("P36", SharedCfgPin::Hdc),
            ("P37", SharedCfgPin::Ldc),
            ("P41", SharedCfgPin::InitB),
            ("P56", SharedCfgPin::Data(7)),
            ("P58", SharedCfgPin::Data(6)),
            ("P59", SharedCfgPin::Data(5)),
            ("P60", SharedCfgPin::Cs0B),
            ("P61", SharedCfgPin::Data(4)),
            ("P65", SharedCfgPin::Data(3)),
            ("P66", SharedCfgPin::Cs1B),
            ("P67", SharedCfgPin::Data(2)),
            ("P69", SharedCfgPin::Data(1)),
            ("P70", SharedCfgPin::RclkB),
            ("P71", SharedCfgPin::Data(0)),
            ("P72", SharedCfgPin::Dout),
            ("P75", SharedCfgPin::Tdo),
            ("P77", SharedCfgPin::Addr(0)),
            ("P78", SharedCfgPin::Addr(1)),
            ("P79", SharedCfgPin::Addr(2)),
            ("P80", SharedCfgPin::Addr(3)),
            ("P81", SharedCfgPin::Addr(4)),
            ("P82", SharedCfgPin::Addr(5)),
            ("P83", SharedCfgPin::Addr(6)),
            ("P84", SharedCfgPin::Addr(7)),
        ][..],
        "pq160" => &[
            ("P143", SharedCfgPin::Addr(8)),
            ("P144", SharedCfgPin::Addr(9)),
            ("P147", SharedCfgPin::Addr(10)),
            ("P148", SharedCfgPin::Addr(11)),
            ("P154", SharedCfgPin::Addr(12)),
            ("P155", SharedCfgPin::Addr(13)),
            ("P158", SharedCfgPin::Addr(14)),
            ("P159", SharedCfgPin::Addr(15)),
            ("P2", SharedCfgPin::Addr(16)),
            ("P3", SharedCfgPin::Addr(17)),
            ("P6", SharedCfgPin::Tdi),
            ("P7", SharedCfgPin::Tck),
            ("P13", SharedCfgPin::Tms),
            ("P38", SharedCfgPin::M1),
            ("P40", SharedCfgPin::M0),
            ("P42", SharedCfgPin::M2),
            ("P44", SharedCfgPin::Hdc),
            ("P48", SharedCfgPin::Ldc),
            ("P59", SharedCfgPin::InitB),
            ("P83", SharedCfgPin::Data(7)),
            ("P87", SharedCfgPin::Data(6)),
            ("P94", SharedCfgPin::Data(5)),
            ("P95", SharedCfgPin::Cs0B),
            ("P98", SharedCfgPin::Data(4)),
            ("P102", SharedCfgPin::Data(3)),
            ("P103", SharedCfgPin::Cs1B),
            ("P106", SharedCfgPin::Data(2)),
            ("P113", SharedCfgPin::Data(1)),
            ("P114", SharedCfgPin::RclkB),
            ("P117", SharedCfgPin::Data(0)),
            ("P118", SharedCfgPin::Dout),
            ("P121", SharedCfgPin::Tdo),
            ("P123", SharedCfgPin::Addr(0)),
            ("P124", SharedCfgPin::Addr(1)),
            ("P127", SharedCfgPin::Addr(2)),
            ("P128", SharedCfgPin::Addr(3)),
            ("P134", SharedCfgPin::Addr(4)),
            ("P135", SharedCfgPin::Addr(5)),
            ("P139", SharedCfgPin::Addr(6)),
            ("P140", SharedCfgPin::Addr(7)),
        ][..],
        "pq208" => &[
            ("P184", SharedCfgPin::Addr(8)),
            ("P185", SharedCfgPin::Addr(9)),
            ("P190", SharedCfgPin::Addr(10)),
            ("P191", SharedCfgPin::Addr(11)),
            ("P199", SharedCfgPin::Addr(12)),
            ("P200", SharedCfgPin::Addr(13)),
            ("P203", SharedCfgPin::Addr(14)),
            ("P204", SharedCfgPin::Addr(15)),
            ("P4", SharedCfgPin::Addr(16)),
            ("P5", SharedCfgPin::Addr(17)),
            ("P8", SharedCfgPin::Tdi),
            ("P9", SharedCfgPin::Tck),
            ("P17", SharedCfgPin::Tms),
            ("P48", SharedCfgPin::M1),
            ("P50", SharedCfgPin::M0),
            ("P56", SharedCfgPin::M2),
            ("P58", SharedCfgPin::Hdc),
            ("P62", SharedCfgPin::Ldc),
            ("P77", SharedCfgPin::InitB),
            ("P109", SharedCfgPin::Data(7)),
            ("P113", SharedCfgPin::Data(6)),
            ("P122", SharedCfgPin::Data(5)),
            ("P123", SharedCfgPin::Cs0B),
            ("P128", SharedCfgPin::Data(4)),
            ("P132", SharedCfgPin::Data(3)),
            ("P133", SharedCfgPin::Cs1B),
            ("P138", SharedCfgPin::Data(2)),
            ("P147", SharedCfgPin::Data(1)),
            ("P148", SharedCfgPin::RclkB),
            ("P151", SharedCfgPin::Data(0)),
            ("P152", SharedCfgPin::Dout),
            ("P159", SharedCfgPin::Tdo),
            ("P161", SharedCfgPin::Addr(0)),
            ("P162", SharedCfgPin::Addr(1)),
            ("P165", SharedCfgPin::Addr(2)),
            ("P166", SharedCfgPin::Addr(3)),
            ("P174", SharedCfgPin::Addr(4)),
            ("P175", SharedCfgPin::Addr(5)),
            ("P180", SharedCfgPin::Addr(6)),
            ("P181", SharedCfgPin::Addr(7)),
        ][..],
        "pq240" => &[
            ("P213", SharedCfgPin::Addr(8)),
            ("P214", SharedCfgPin::Addr(9)),
            ("P220", SharedCfgPin::Addr(10)),
            ("P221", SharedCfgPin::Addr(11)),
            ("P232", SharedCfgPin::Addr(12)),
            ("P233", SharedCfgPin::Addr(13)),
            ("P238", SharedCfgPin::Addr(14)),
            ("P239", SharedCfgPin::Addr(15)),
            ("P2", SharedCfgPin::Addr(16)),
            ("P3", SharedCfgPin::Addr(17)),
            ("P6", SharedCfgPin::Tdi),
            ("P7", SharedCfgPin::Tck),
            ("P17", SharedCfgPin::Tms),
            ("P58", SharedCfgPin::M1),
            ("P60", SharedCfgPin::M0),
            ("P62", SharedCfgPin::M2),
            ("P64", SharedCfgPin::Hdc),
            ("P68", SharedCfgPin::Ldc),
            ("P89", SharedCfgPin::InitB),
            ("P123", SharedCfgPin::Data(7)),
            ("P129", SharedCfgPin::Data(6)),
            ("P141", SharedCfgPin::Data(5)),
            ("P142", SharedCfgPin::Cs0B),
            ("P148", SharedCfgPin::Data(4)),
            ("P152", SharedCfgPin::Data(3)),
            ("P153", SharedCfgPin::Cs1B),
            ("P159", SharedCfgPin::Data(2)),
            ("P173", SharedCfgPin::Data(1)),
            ("P174", SharedCfgPin::RclkB),
            ("P177", SharedCfgPin::Data(0)),
            ("P178", SharedCfgPin::Dout),
            ("P181", SharedCfgPin::Tdo),
            ("P183", SharedCfgPin::Addr(0)),
            ("P184", SharedCfgPin::Addr(1)),
            ("P187", SharedCfgPin::Addr(2)),
            ("P188", SharedCfgPin::Addr(3)),
            ("P202", SharedCfgPin::Addr(4)),
            ("P203", SharedCfgPin::Addr(5)),
            ("P209", SharedCfgPin::Addr(6)),
            ("P210", SharedCfgPin::Addr(7)),
        ][..],
        _ => &[][..],
    };
    let mut cfg_io = BTreeMap::new();
    for &(pin, fun) in pkg_cfg_io {
        let BondPin::Io(io) = bond.pins[pin] else {
            unreachable!()
        };
        cfg_io.insert(fun, io);
    }

    (bond, cfg_io)
}
