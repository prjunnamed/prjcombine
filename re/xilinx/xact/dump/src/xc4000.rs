use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, ConnectorClass, ConnectorWire, IntDb, PinDir, TileClass,
        TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
    grid::{CellCoord, DieId, EdgeIoCoord},
};
use prjcombine_re_xilinx_xact_data::die::Die;
use prjcombine_re_xilinx_xact_naming::db::{NamingDb, TileNaming};
use prjcombine_re_xilinx_xact_xc2000::{ExpandedNamedDevice, name_device};
use prjcombine_xc2000::{
    bels::xc4000 as bels,
    bond::{Bond, BondPad, CfgPad},
    chip::{Chip, ChipKind, SharedCfgPad},
    cslots, regions, tslots,
};
use prjcombine_entity::EntityId;

use crate::extractor::{Extractor, NetBinding, PipMode};

fn bel_from_pins(db: &IntDb, pins: &[(&str, impl AsRef<str>)]) -> BelInfo {
    let mut bel = Bel::default();
    let mut has_dec = false;
    for &(name, ref wire) in pins {
        let wire = wire.as_ref();
        if wire.starts_with("DEC") {
            has_dec = true;
        }
        bel.pins.insert(
            name.into(),
            BelPin {
                wires: BTreeSet::from_iter([TileWireCoord::new_idx(0, db.get_wire(wire))]),
                dir: if wire.starts_with("IMUX") {
                    PinDir::Input
                } else {
                    PinDir::Output
                },
            },
        );
    }
    if has_dec && let Some(pin) = bel.pins.get_mut("I") {
        pin.dir = PinDir::Input;
    }
    BelInfo::Bel(bel)
}

pub fn make_intdb(kind: ChipKind) -> IntDb {
    let mut db = IntDb::new(tslots::SLOTS, bels::SLOTS, regions::SLOTS, cslots::SLOTS);

    let term_slots = DirMap::from_fn(|dir| match dir {
        Dir::W => cslots::W,
        Dir::E => cslots::E,
        Dir::S => cslots::S,
        Dir::N => cslots::N,
    });

    let mut main_terms = DirMap::from_fn(|dir| ConnectorClass::new(term_slots[dir]));
    let mut cnr_ll_w = ConnectorClass::new(cslots::W);
    let cnr_lr_s = ConnectorClass::new(cslots::S);
    let mut cnr_ul_n = ConnectorClass::new(cslots::N);
    let mut cnr_ur_e = ConnectorClass::new(cslots::E);

    db.wires.insert("GND".into(), WireKind::Tie0);

    let single_num = if kind == ChipKind::Xc4000A { 4 } else { 8 };
    for (dir, hv) in [(Dir::E, 'H'), (Dir::S, 'V')] {
        for i in 0..single_num {
            let w0 = db
                .wires
                .insert(format!("SINGLE.{hv}{i}"), WireKind::MultiOut)
                .0;
            let w1 = db
                .wires
                .insert(
                    format!("SINGLE.{hv}{i}.{dir}"),
                    WireKind::MultiBranch(term_slots[!dir]),
                )
                .0;
            main_terms[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
        }
    }

    for (dir, hv) in [(Dir::E, 'H'), (Dir::S, 'V')] {
        for i in 0..2 {
            let w0 = db
                .wires
                .insert(format!("DOUBLE.{hv}{i}.0"), WireKind::MultiOut)
                .0;
            let w1 = db
                .wires
                .insert(
                    format!("DOUBLE.{hv}{i}.1"),
                    WireKind::MultiBranch(term_slots[!dir]),
                )
                .0;
            let w2 = db
                .wires
                .insert(
                    format!("DOUBLE.{hv}{i}.2"),
                    WireKind::MultiBranch(term_slots[!dir]),
                )
                .0;
            main_terms[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
            main_terms[!dir].wires.insert(w2, ConnectorWire::Pass(w1));
        }
    }

    let io_double_num = if kind == ChipKind::Xc4000A { 2 } else { 4 };
    let bdir = DirMap::from_fn(|dir| match dir {
        Dir::S => Dir::W,
        Dir::E => Dir::S,
        Dir::N => Dir::E,
        Dir::W => Dir::N,
    });
    for i in 0..io_double_num {
        let mut wires = DirMap::from_fn(|_| vec![]);

        for j in 0..3 {
            for dir in Dir::DIRS {
                wires[dir].push(
                    db.wires
                        .insert(
                            format!("IO.DOUBLE.{i}.{dir}.{j}"),
                            WireKind::MultiBranch(term_slots[bdir[dir]]),
                        )
                        .0,
                );
            }
        }

        for j in 0..2 {
            for dir in Dir::DIRS {
                main_terms[bdir[dir]]
                    .wires
                    .insert(wires[dir][j + 1], ConnectorWire::Pass(wires[dir][j]));
            }
            cnr_ul_n.wires.insert(
                wires[Dir::W][j],
                ConnectorWire::Reflect(wires[Dir::N][j + 1]),
            );
        }
        cnr_ll_w
            .wires
            .insert(wires[Dir::S][1], ConnectorWire::Reflect(wires[Dir::W][1]));
        cnr_ur_e
            .wires
            .insert(wires[Dir::N][1], ConnectorWire::Reflect(wires[Dir::E][1]));
    }

    for i in 0..2 {
        db.wires.insert(format!("IO.DBUF.H{i}"), WireKind::MuxOut);
    }
    for i in 0..2 {
        db.wires.insert(format!("IO.DBUF.V{i}"), WireKind::MuxOut);
    }

    let long_num = if kind == ChipKind::Xc4000A { 4 } else { 6 };
    for i in 0..long_num {
        let w = db
            .wires
            .insert(format!("LONG.H{i}"), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    }
    for i in 0..long_num {
        let w = db
            .wires
            .insert(format!("LONG.V{i}"), WireKind::MultiBranch(cslots::S))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }
    let io_long_num = if kind == ChipKind::Xc4000A { 2 } else { 4 };
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("LONG.IO.H{i}"), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    }
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("LONG.IO.V{i}"), WireKind::MultiBranch(cslots::S))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("DEC.H{i}"), WireKind::MultiBranch(cslots::W))
            .0;
        main_terms[Dir::W].wires.insert(w, ConnectorWire::Pass(w));
    }
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("DEC.V{i}"), WireKind::MultiBranch(cslots::S))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }

    for i in 0..4 {
        let w = db
            .wires
            .insert(format!("GCLK{i}"), WireKind::MultiBranch(cslots::S))
            .0;
        main_terms[Dir::S].wires.insert(w, ConnectorWire::Pass(w));
    }

    for i in 1..=4 {
        for p in ['F', 'G', 'C'] {
            let w = db
                .wires
                .insert(format!("IMUX.CLB.{p}{i}"), WireKind::MuxOut)
                .0;
            if i == 2 {
                let wn = db
                    .wires
                    .insert(format!("IMUX.CLB.{p}{i}.N"), WireKind::Branch(cslots::S))
                    .0;
                main_terms[Dir::S].wires.insert(wn, ConnectorWire::Pass(w));
            }
            if i == 3 {
                let ww = db
                    .wires
                    .insert(format!("IMUX.CLB.{p}{i}.W"), WireKind::Branch(cslots::E))
                    .0;
                main_terms[Dir::E].wires.insert(ww, ConnectorWire::Pass(w));
            }
        }
    }
    for name in [
        "IMUX.CLB.K",
        "IMUX.TBUF0.I",
        "IMUX.TBUF0.TS",
        "IMUX.TBUF1.I",
        "IMUX.TBUF1.TS",
        "IMUX.IOB0.O1",
        "IMUX.IOB0.OK",
        "IMUX.IOB0.IK",
        "IMUX.IOB0.TS",
        "IMUX.IOB1.O1",
        "IMUX.IOB1.OK",
        "IMUX.IOB1.IK",
        "IMUX.IOB1.TS",
        "IMUX.BOT.COUT",
        "IMUX.STARTUP.CLK",
        "IMUX.STARTUP.GSR",
        "IMUX.STARTUP.GTS",
        "IMUX.READCLK.I",
        "IMUX.BUFG.H",
        "IMUX.BUFG.V",
        "IMUX.TDO.O",
        "IMUX.TDO.T",
        "IMUX.RDBK.TRIG",
        "IMUX.BSCAN.TDO1",
        "IMUX.BSCAN.TDO2",
    ] {
        db.wires.insert(name.into(), WireKind::MuxOut);
    }
    for (name, dirs) in [
        ("OUT.CLB.FX", &[Dir::S][..]),
        ("OUT.CLB.FXQ", &[Dir::S][..]),
        ("OUT.CLB.GY", &[Dir::E][..]),
        ("OUT.CLB.GYQ", &[Dir::E][..]),
        ("OUT.BT.IOB0.I1", &[][..]),
        ("OUT.BT.IOB0.I2", &[][..]),
        ("OUT.BT.IOB1.I1", &[Dir::E][..]),
        ("OUT.BT.IOB1.I2", &[Dir::E][..]),
        ("OUT.LR.IOB0.I1", &[][..]),
        ("OUT.LR.IOB0.I2", &[][..]),
        ("OUT.LR.IOB1.I1", &[Dir::S][..]),
        ("OUT.LR.IOB1.I2", &[Dir::S][..]),
        ("OUT.HIOB0.I", &[][..]),
        ("OUT.HIOB1.I", &[][..]),
        ("OUT.HIOB2.I", &[][..]),
        ("OUT.HIOB3.I", &[][..]),
        ("OUT.IOB.CLKIN", &[Dir::W, Dir::E, Dir::S, Dir::N]),
        ("OUT.OSC.MUX1", &[][..]),
        ("OUT.STARTUP.DONEIN", &[][..]),
        ("OUT.STARTUP.Q1Q4", &[][..]),
        ("OUT.STARTUP.Q2", &[][..]),
        ("OUT.STARTUP.Q3", &[][..]),
        ("OUT.UPDATE.O", &[][..]),
        ("OUT.MD0.I", &[][..]),
        ("OUT.RDBK.DATA", &[][..]),
    ] {
        if name.starts_with("OUT.HIOB") && kind != ChipKind::Xc4000H {
            continue;
        }
        let w = db.wires.insert(name.into(), WireKind::LogicOut).0;
        for &dir in dirs {
            let wo = db
                .wires
                .insert(format!("{name}.{dir}"), WireKind::Branch(term_slots[!dir]))
                .0;
            main_terms[!dir].wires.insert(wo, ConnectorWire::Pass(w));
        }
    }

    let mut ll_terms = main_terms.clone();
    for term in ll_terms.values_mut() {
        for (w, name, _) in &db.wires {
            if name.starts_with("LONG") || name.starts_with("DEC") {
                term.wires.remove(w);
            }
        }
    }

    let mut tclb_n = main_terms[Dir::N].clone();
    for (wt, wf) in [
        ("OUT.CLB.FX.S", "OUT.BT.IOB0.I2"),
        ("OUT.CLB.FXQ.S", "OUT.BT.IOB1.I2"),
    ] {
        let wt = db.get_wire(wt);
        let wf = db.get_wire(wf);
        tclb_n.wires.insert(wt, ConnectorWire::Pass(wf));
    }

    let mut lclb_w = main_terms[Dir::W].clone();
    for (wt, wf) in [
        ("OUT.CLB.GY.E", "OUT.LR.IOB1.I2"),
        ("OUT.CLB.GYQ.E", "OUT.LR.IOB0.I2"),
    ] {
        let wt = db.get_wire(wt);
        let wf = db.get_wire(wf);
        lclb_w.wires.insert(wt, ConnectorWire::Pass(wf));
    }

    for (dir, term) in main_terms {
        db.conn_classes.insert(format!("MAIN.{dir}"), term);
    }
    for (dir, term) in ll_terms {
        let hv = match dir {
            Dir::W | Dir::E => 'H',
            Dir::S | Dir::N => 'V',
        };
        db.conn_classes.insert(format!("LL{hv}C.{dir}"), term);
    }
    db.conn_classes.insert("TCLB.N".into(), tclb_n);
    db.conn_classes.insert("LCLB.W".into(), lclb_w);
    db.conn_classes.insert("CNR.LL.W".into(), cnr_ll_w);
    db.conn_classes.insert("CNR.LR.S".into(), cnr_lr_s);
    db.conn_classes.insert("CNR.UL.N".into(), cnr_ul_n);
    db.conn_classes.insert("CNR.UR.E".into(), cnr_ur_e);

    for name in [
        "CLB.LT", "CLB.T", "CLB.RT", "CLB.L", "CLB", "CLB.R", "CLB.LB", "CLB.B", "CLB.RB",
    ] {
        let mut tcls = TileClass::new(tslots::MAIN, 3);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));

        tcls.bels.insert(
            bels::CLB,
            bel_from_pins(
                &db,
                &[
                    ("F1", "IMUX.CLB.F1"),
                    ("G1", "IMUX.CLB.G1"),
                    ("C1", "IMUX.CLB.C1"),
                    ("F2", "IMUX.CLB.F2.N"),
                    ("G2", "IMUX.CLB.G2.N"),
                    ("C2", "IMUX.CLB.C2.N"),
                    ("F3", "IMUX.CLB.F3.W"),
                    ("G3", "IMUX.CLB.G3.W"),
                    ("C3", "IMUX.CLB.C3.W"),
                    ("F4", "IMUX.CLB.F4"),
                    ("G4", "IMUX.CLB.G4"),
                    ("C4", "IMUX.CLB.C4"),
                    ("K", "IMUX.CLB.K"),
                    ("X", "OUT.CLB.FX"),
                    ("XQ", "OUT.CLB.FXQ"),
                    ("Y", "OUT.CLB.GY"),
                    ("YQ", "OUT.CLB.GYQ"),
                ],
            ),
        );
        for i in 0..2 {
            tcls.bels.insert(
                bels::TBUF[i],
                bel_from_pins(
                    &db,
                    &[
                        ("I", format!("IMUX.TBUF{i}.I")),
                        ("O", format!("LONG.H{}", long_num / 2 - 1 + i)),
                        ("T", format!("IMUX.TBUF{i}.TS")),
                    ],
                ),
            );
        }
        db.tile_classes.insert(name.into(), tcls);
    }
    for name in [
        "IO.B", "IO.B.R", "IO.BS", "IO.BS.L", "IO.T", "IO.T.R", "IO.TS", "IO.TS.L",
    ] {
        let is_bot = name.starts_with("IO.B");
        let num_cells = if is_bot { 4 } else { 3 };
        let mut tcls = TileClass::new(tslots::MAIN, num_cells);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));
        if kind != ChipKind::Xc4000H {
            for i in 0..2 {
                let mut pins = vec![
                    ("O1", format!("IMUX.IOB{i}.O1")),
                    (
                        "O2",
                        (if is_bot {
                            ["IMUX.CLB.F4", "IMUX.CLB.G4"][i]
                        } else {
                            ["IMUX.CLB.F2.N", "IMUX.CLB.G2.N"][i]
                        })
                        .to_string(),
                    ),
                    ("I1", format!("OUT.BT.IOB{i}.I1")),
                    ("I2", format!("OUT.BT.IOB{i}.I2")),
                    ("IK", format!("IMUX.IOB{i}.IK")),
                    ("OK", format!("IMUX.IOB{i}.OK")),
                    ("T", format!("IMUX.IOB{i}.TS")),
                ];
                if matches!(
                    (name, i),
                    ("IO.B.R", 1) | ("IO.BS.L", 0) | ("IO.T.R", 0) | ("IO.TS.L", 0)
                ) {
                    pins.push(("CLKIN", "OUT.IOB.CLKIN".to_string()));
                }
                tcls.bels.insert(bels::IO[i], bel_from_pins(&db, &pins));
            }
        } else {
            for i in 0..4 {
                let ii = i / 2;
                let mut pins = vec![
                    ("O1", format!("IMUX.IOB{ii}.O1")),
                    (
                        "O2",
                        (if is_bot {
                            ["IMUX.CLB.F4", "IMUX.CLB.G4"][ii]
                        } else {
                            ["IMUX.CLB.F2.N", "IMUX.CLB.G2.N"][ii]
                        })
                        .to_string(),
                    ),
                    ("I", format!("OUT.HIOB{i}.I")),
                    ("T1", format!("IMUX.IOB{ii}.TS")),
                    (
                        "T2",
                        if matches!(i, 0 | 3) {
                            format!("IMUX.IOB{ii}.OK")
                        } else {
                            format!("IMUX.IOB{ii}.IK")
                        },
                    ),
                ];
                if matches!(
                    (name, i),
                    ("IO.B.R", 3) | ("IO.BS.L", 0) | ("IO.T.R", 2) | ("IO.TS.L", 0)
                ) {
                    pins.push(("CLKIN", "OUT.IOB.CLKIN".to_string()));
                }
                tcls.bels.insert(bels::HIO[i], bel_from_pins(&db, &pins));
            }
        }
        for (i, iwire) in [
            "OUT.BT.IOB0.I1",
            if is_bot {
                "IMUX.CLB.C4"
            } else {
                "IMUX.CLB.C2.N"
            },
            "OUT.BT.IOB1.I1",
        ]
        .into_iter()
        .enumerate()
        {
            let mut pins = vec![("I", iwire.to_string())];
            for j in 0..io_long_num {
                pins.push((["O1", "O2", "O3", "O4"][j], format!("DEC.H{j}")));
            }
            tcls.bels.insert(bels::DEC[i], bel_from_pins(&db, &pins));
        }
        db.tile_classes.insert(name.into(), tcls);
    }
    for name in [
        "IO.R", "IO.R.T", "IO.RS", "IO.RS.B", "IO.L", "IO.L.T", "IO.LS", "IO.LS.B",
    ] {
        let is_left = name.starts_with("IO.L");
        let num_cells = if is_left { 4 } else { 3 };
        let mut tcls = TileClass::new(tslots::MAIN, num_cells);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));
        if kind != ChipKind::Xc4000H {
            for i in 0..2 {
                let mut pins = vec![
                    ("O1", format!("IMUX.IOB{i}.O1")),
                    (
                        "O2",
                        (if is_left {
                            ["IMUX.CLB.G3.W", "IMUX.CLB.F3.W"][i]
                        } else {
                            ["IMUX.CLB.G1", "IMUX.CLB.F1"][i]
                        })
                        .to_string(),
                    ),
                    ("I1", format!("OUT.LR.IOB{i}.I1")),
                    ("I2", format!("OUT.LR.IOB{i}.I2")),
                    ("IK", format!("IMUX.IOB{i}.IK")),
                    ("OK", format!("IMUX.IOB{i}.OK")),
                    ("T", format!("IMUX.IOB{i}.TS")),
                ];
                if matches!(
                    (name, i),
                    ("IO.L.T", 0) | ("IO.LS.B", 1) | ("IO.R.T", 0) | ("IO.RS.B", 0)
                ) {
                    pins.push(("CLKIN", "OUT.IOB.CLKIN".to_string()));
                }
                tcls.bels.insert(bels::IO[i], bel_from_pins(&db, &pins));
            }
        } else {
            for i in 0..4 {
                let ii = i / 2;
                let mut pins = vec![
                    ("O1", format!("IMUX.IOB{ii}.O1")),
                    (
                        "O2",
                        (if is_left {
                            ["IMUX.CLB.G3.W", "IMUX.CLB.F3.W"][ii]
                        } else {
                            ["IMUX.CLB.G1", "IMUX.CLB.F1"][ii]
                        })
                        .to_string(),
                    ),
                    ("I", format!("OUT.HIOB{i}.I")),
                    ("T1", format!("IMUX.IOB{ii}.TS")),
                    (
                        "T2",
                        if matches!(i, 0 | 3) {
                            format!("IMUX.IOB{ii}.OK")
                        } else {
                            format!("IMUX.IOB{ii}.IK")
                        },
                    ),
                ];
                if matches!(
                    (name, i),
                    ("IO.L.T", 0) | ("IO.LS.B", 3) | ("IO.R.T", 0) | ("IO.RS.B", 2)
                ) {
                    pins.push(("CLKIN", "OUT.IOB.CLKIN".to_string()));
                }
                tcls.bels.insert(bels::HIO[i], bel_from_pins(&db, &pins));
            }
        }
        for i in 0..2 {
            tcls.bels.insert(
                bels::TBUF[i],
                bel_from_pins(
                    &db,
                    &[
                        ("I", format!("IMUX.TBUF{i}.I")),
                        ("O", format!("LONG.H{}", long_num / 2 - 1 + i)),
                        ("T", format!("IMUX.TBUF{i}.TS")),
                    ],
                ),
            );
        }
        for i in 0..2 {
            tcls.bels.insert(
                bels::PULLUP_TBUF[i],
                bel_from_pins(&db, &[("O", format!("LONG.H{}", long_num / 2 - 1 + i))]),
            );
        }
        for (i, iwire) in [
            "OUT.LR.IOB0.I1",
            if is_left {
                "IMUX.CLB.C3.W"
            } else {
                "IMUX.CLB.C1"
            },
            "OUT.LR.IOB1.I1",
        ]
        .into_iter()
        .enumerate()
        {
            let mut pins = vec![("I", iwire.to_string())];
            for j in 0..io_long_num {
                pins.push((["O1", "O2", "O3", "O4"][j], format!("DEC.V{j}")));
            }
            tcls.bels.insert(bels::DEC[i], bel_from_pins(&db, &pins));
        }
        db.tile_classes.insert(name.into(), tcls);
    }
    for (name, num_cells) in [("CNR.BR", 1), ("CNR.TR", 2), ("CNR.BL", 2), ("CNR.TL", 4)] {
        let mut tcls = TileClass::new(tslots::MAIN, num_cells);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(Default::default()));

        for i in 0..io_long_num {
            tcls.bels.insert(
                bels::PULLUP_DEC_H[i],
                bel_from_pins(&db, &[("O", format!("DEC.H{i}"))]),
            );
        }
        for i in 0..io_long_num {
            tcls.bels.insert(
                bels::PULLUP_DEC_V[i],
                bel_from_pins(&db, &[("O", format!("DEC.V{i}"))]),
            );
        }
        for (hv, slot) in [('H', bels::BUFGLS_H), ('V', bels::BUFGLS_V)] {
            tcls.bels.insert(
                slot,
                bel_from_pins(&db, &[("I", format!("IMUX.BUFG.{hv}"))]),
            );
        }
        match name {
            "CNR.BR" => {
                tcls.bels
                    .insert(bels::COUT, BelInfo::Bel(Default::default()));
                tcls.bels.insert(
                    bels::STARTUP,
                    bel_from_pins(
                        &db,
                        &[
                            ("CLK", "IMUX.STARTUP.CLK"),
                            ("GSR", "IMUX.STARTUP.GSR"),
                            ("GTS", "IMUX.STARTUP.GTS"),
                            ("DONEIN", "OUT.STARTUP.DONEIN"),
                            ("Q1Q4", "OUT.STARTUP.Q1Q4"),
                            ("Q2", "OUT.STARTUP.Q2"),
                            ("Q3", "OUT.STARTUP.Q3"),
                        ],
                    ),
                );
                tcls.bels.insert(
                    bels::READCLK,
                    bel_from_pins(&db, &[("I", "IMUX.READCLK.I")]),
                );
            }
            "CNR.TR" => {
                tcls.bels
                    .insert(bels::COUT, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::UPDATE, bel_from_pins(&db, &[("O", "OUT.UPDATE.O")]));
                tcls.bels.insert(
                    bels::OSC,
                    bel_from_pins(
                        &db,
                        &[
                            ("F8M", "OUT.LR.IOB1.I1"),
                            ("OUT0", "OUT.LR.IOB1.I2"),
                            ("OUT1", "OUT.OSC.MUX1"),
                        ],
                    ),
                );
                tcls.bels.insert(
                    bels::TDO,
                    bel_from_pins(&db, &[("O", "IMUX.TDO.O"), ("T", "IMUX.TDO.T")]),
                );
            }
            "CNR.BL" => {
                tcls.bels
                    .insert(bels::CIN, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::MD0, bel_from_pins(&db, &[("I", "OUT.MD0.I")]));
                tcls.bels.insert(
                    bels::MD1,
                    bel_from_pins(&db, &[("O", "IMUX.IOB1.O1"), ("T", "IMUX.IOB1.IK")]),
                );
                tcls.bels
                    .insert(bels::MD2, bel_from_pins(&db, &[("I", "OUT.BT.IOB1.I1")]));
                tcls.bels.insert(
                    bels::RDBK,
                    bel_from_pins(
                        &db,
                        &[
                            ("DATA", "OUT.RDBK.DATA"),
                            ("RIP", "OUT.BT.IOB1.I2"),
                            ("TRIG", "IMUX.RDBK.TRIG"),
                        ],
                    ),
                );
            }
            "CNR.TL" => {
                tcls.bels
                    .insert(bels::CIN, BelInfo::Bel(Default::default()));
                tcls.bels.insert(
                    bels::BSCAN,
                    bel_from_pins(
                        &db,
                        &[
                            ("TDO1", "IMUX.BSCAN.TDO1"),
                            ("TDO2", "IMUX.BSCAN.TDO2"),
                            ("DRCK", "OUT.BT.IOB1.I2"),
                            ("IDLE", "OUT.LR.IOB1.I2"),
                            ("SEL1", "OUT.LR.IOB1.I1"),
                            ("SEL2", "OUT.BT.IOB1.I1"),
                        ],
                    ),
                );
            }
            _ => unreachable!(),
        }
        db.tile_classes.insert(name.into(), tcls);
    }
    for name in ["LLH.IO.B", "LLH.IO.T", "LLH.CLB", "LLH.CLB.B"] {
        let mut tcls = TileClass::new(tslots::EXTRA_COL, 2);
        tcls.bels
            .insert(bels::LLH, BelInfo::SwitchBox(Default::default()));
        db.tile_classes.insert(name.into(), tcls);
    }
    for name in ["LLV.IO.L", "LLV.IO.R", "LLV.CLB"] {
        let mut tcls = TileClass::new(tslots::EXTRA_ROW, 2);
        tcls.bels
            .insert(bels::LLV, BelInfo::SwitchBox(Default::default()));
        tcls.bels.insert(
            bels::CLKH,
            bel_from_pins(
                &db,
                &[
                    ("O0", "GCLK0"),
                    ("O1", "GCLK1"),
                    ("O2", "GCLK2"),
                    ("O3", "GCLK3"),
                ],
            ),
        );
        db.tile_classes.insert(name.into(), tcls);
    }

    db
}

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
    let mut intdb = make_intdb(chip.kind);
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
            let mut tie = extractor.grab_prim_ab(&ntile.tie_names[0], &ntile.tie_names[1]);
            let o = tie.get_pin("O");
            extractor.net_int(o, cell.wire(intdb.get_wire("GND")));
        }
        let tile = &extractor.die.newtiles[&(endev.col_x[col].start, endev.row_y[row].start)];
        if tcrd.slot == tslots::MAIN {
            for &box_id in &tile.boxes {
                extractor.own_box(box_id, tcrd);
            }
        }
        for (slot, bel_info) in &tcls.bels {
            let BelInfo::Bel(bel_info) = bel_info else {
                continue;
            };
            let bel = cell.bel(slot);
            let slot_name = intdb.bel_slots.key(slot);

            if slot == bels::CLKH && chip.kind != ChipKind::Xc4000H {
                continue;
            }
            let bel_names = &ntile.bels[slot];
            match slot {
                _ if slot_name.starts_with("PULLUP") => {
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    extractor.net_bel_int(line, bel, "O");
                    extractor.bel_pip(ntile.naming, slot, "O", pip);
                }
                bels::BUFGLS_H | bels::BUFGLS_V => {
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let i = prim.get_pin("I");
                    extractor.net_bel_int(i, bel, "I");
                }
                bels::CIN => {
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                }
                bels::COUT => {
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    let i = prim.get_pin("I");
                    extractor.net_bel(i, bel, "I");
                }
                bels::MD0
                | bels::MD1
                | bels::MD2
                | bels::RDBK
                | bels::BSCAN
                | bels::STARTUP
                | bels::READCLK
                | bels::UPDATE
                | bels::TDO => {
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                }
                bels::OSC => {
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    extractor.net_bel_int(prim.get_pin("F8M"), bel, "F8M");
                    for pin in ["F500K", "F16K", "F490", "F15"] {
                        let o = prim.get_pin(pin);
                        extractor.net_bel(o, bel, pin);
                        let mut o = extractor.consume_all_fwd(o, tcrd);
                        o.sort_by_key(|(_, pip)| pip.y);
                        extractor.net_bel_int(o[0].0, bel, "OUT0");
                        extractor.net_bel_int(o[1].0, bel, "OUT1");
                        extractor.bel_pip(ntile.naming, slot, format!("OUT0.{pin}"), o[0].1);
                        extractor.bel_pip(ntile.naming, slot, format!("OUT1.{pin}"), o[1].1);
                    }
                }
                bels::IO0 | bels::IO1 => {
                    let mut prim = extractor.grab_prim_i(&bel_names[0]);
                    for pin in ["I1", "I2", "IK", "OK", "T"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    // not quite true, but we'll fix it up.
                    extractor.net_bel_int(prim.get_pin("O"), bel, "O2");
                    if bel_names.len() > 1 {
                        let mut prim = extractor.grab_prim_a(&bel_names[1]);
                        extractor.net_bel_int(prim.get_pin("I"), bel, "CLKIN");
                    }
                }
                bels::HIO0 | bels::HIO1 | bels::HIO2 | bels::HIO3 => {
                    let mut prim = extractor.grab_prim_i(&bel_names[0]);
                    extractor.net_bel_int(prim.get_pin("TS"), bel, "T2");
                    let tp = prim.get_pin("TP");
                    extractor.net_bel(tp, bel, "T1");
                    let (line, pip) = extractor.consume_one_bwd(tp, tcrd);
                    extractor.net_bel_int(line, bel, "T1");
                    extractor.bel_pip(ntile.naming, slot, "T1", pip);

                    // O1/O2
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let mut o = extractor.consume_all_bwd(o, tcrd);
                    assert_eq!(o.len(), 2);
                    if col == chip.col_w() {
                        o.sort_by_key(|(_, pip)| pip.x);
                    } else if col == chip.col_e() {
                        o.sort_by_key(|(_, pip)| !pip.x);
                    } else if row == chip.row_s() {
                        o.sort_by_key(|(_, pip)| pip.y);
                    } else if row == chip.row_n() {
                        o.sort_by_key(|(_, pip)| !pip.y);
                    }
                    extractor.net_bel_int(o[0].0, bel, "O1");
                    extractor.net_bel_int(o[1].0, bel, "O2");
                    extractor.bel_pip(ntile.naming, slot, "O1", o[0].1);
                    extractor.bel_pip(ntile.naming, slot, "O2", o[1].1);

                    // I1/I2
                    let net_i = prim.get_pin("I");
                    extractor.net_bel_int(net_i, bel, "I");
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
                    let lrbt = if col == chip.col_w() || col == chip.col_e() {
                        "LR"
                    } else {
                        "BT"
                    };
                    let ii = match slot {
                        bels::HIO0 | bels::HIO1 => 0,
                        bels::HIO2 | bels::HIO3 => 1,
                        _ => unreachable!(),
                    };
                    extractor.net_int(
                        i[0].0,
                        cell.wire(intdb.get_wire(&format!("OUT.{lrbt}.IOB{ii}.I1"))),
                    );
                    extractor.net_int(
                        i[1].0,
                        cell.wire(intdb.get_wire(&format!("OUT.{lrbt}.IOB{ii}.I2"))),
                    );

                    if bel_names.len() > 1 {
                        let mut prim = extractor.grab_prim_a(&bel_names[1]);
                        extractor.net_bel_int(prim.get_pin("I"), bel, "CLKIN");
                    }
                }
                bels::TBUF0 | bels::TBUF1 => {
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    for pin in ["I", "T"] {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "O");
                    let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                    extractor.net_bel_int(line, bel, "O");
                    extractor.bel_pip(ntile.naming, slot, "O", pip);
                }
                bels::DEC0 | bels::DEC1 | bels::DEC2 => {
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    for pin in bel_info.pins.keys() {
                        if pin.starts_with('O') {
                            let o = prim.get_pin(pin);
                            extractor.net_bel(o, bel, pin);
                            let (line, pip) = extractor.consume_one_fwd(o, tcrd);
                            extractor.bel_pip(ntile.naming, slot, pin, pip);
                            extractor.net_bel_int(line, bel, pin);
                        }
                    }
                    let i = prim.get_pin("I");
                    if slot == bels::DEC1 {
                        extractor.net_bel_int(i, bel, "I");
                    } else {
                        extractor.net_bel(i, bel, "I");
                        let (line, pip) = extractor.consume_one_bwd(i, tcrd);
                        extractor.net_bel_int(line, bel, "I");
                        extractor.bel_pip(ntile.naming, slot, "I", pip);
                    }
                }
                bels::CLB => {
                    let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                    for pin in bel_info.pins.keys() {
                        extractor.net_bel_int(prim.get_pin(pin), bel, pin);
                    }
                    let cin = prim.get_pin("CIN");
                    extractor.net_bel(cin, bel, "CIN");
                    let cout = prim.get_pin("COUT");
                    extractor.net_bel(cout, bel, "COUT");
                }
                bels::CLKH => {
                    let mut prim = extractor.grab_prim_a(&bel_names[0]);
                    let o = prim.get_pin("O");
                    extractor.net_bel(o, bel, "GND");
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
                        "LONG.IO.V0",
                        "LONG.IO.V1",
                        "GCLK0",
                        "GCLK1",
                        "GCLK2",
                        "GCLK3",
                    ][..]
                } else {
                    &[
                        "LONG.IO.V0",
                        "LONG.IO.V1",
                        "LONG.IO.V2",
                        "LONG.IO.V3",
                        "GCLK0",
                        "GCLK1",
                        "GCLK2",
                        "GCLK3",
                    ][..]
                }
            } else if col == chip.col_e() {
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        "LONG.V0",
                        "LONG.V1",
                        "LONG.V2",
                        "LONG.V3",
                        "GCLK0",
                        "GCLK1",
                        "GCLK2",
                        "GCLK3",
                        "LONG.IO.V0",
                        "LONG.IO.V1",
                    ][..]
                } else {
                    &[
                        "LONG.V0",
                        "LONG.V1",
                        "LONG.V2",
                        "LONG.V3",
                        "LONG.V4",
                        "LONG.V5",
                        "GCLK0",
                        "GCLK1",
                        "GCLK2",
                        "GCLK3",
                        "LONG.IO.V0",
                        "LONG.IO.V1",
                        "LONG.IO.V2",
                        "LONG.IO.V3",
                    ][..]
                }
            } else {
                if chip.kind == ChipKind::Xc4000A {
                    &[
                        "LONG.V0", "LONG.V1", "LONG.V2", "LONG.V3", "GCLK0", "GCLK1", "GCLK2",
                        "GCLK3",
                    ][..]
                } else {
                    &[
                        "LONG.V0", "LONG.V1", "LONG.V2", "LONG.V3", "LONG.V4", "LONG.V5", "GCLK0",
                        "GCLK1", "GCLK2", "GCLK3",
                    ][..]
                }
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
                    &["LONG.IO.H0", "LONG.IO.H1", "LONG.H2", "LONG.H3"][..]
                } else {
                    &[
                        "LONG.IO.H0",
                        "LONG.IO.H1",
                        "LONG.IO.H2",
                        "LONG.IO.H3",
                        "LONG.H3",
                        "LONG.H4",
                        "LONG.H5",
                    ][..]
                }
            } else if row == chip.row_n() {
                if chip.kind == ChipKind::Xc4000A {
                    &["LONG.H0", "LONG.H1", "LONG.IO.H0", "LONG.IO.H1"][..]
                } else {
                    &[
                        "LONG.H0",
                        "LONG.H1",
                        "LONG.H2",
                        "LONG.IO.H0",
                        "LONG.IO.H1",
                        "LONG.IO.H2",
                        "LONG.IO.H3",
                    ][..]
                }
            } else {
                if chip.kind == ChipKind::Xc4000A {
                    &["LONG.H0", "LONG.H3"][..]
                } else {
                    &["LONG.H0", "LONG.H1", "LONG.H4", "LONG.H5"][..]
                }
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
                    (net_n, format!("SINGLE.V{i}.S")),
                    (net_e, format!("SINGLE.H{i}")),
                    (net_s, format!("SINGLE.V{i}")),
                    (net_w, format!("SINGLE.H{i}.E")),
                ] {
                    extractor.net_int(
                        net,
                        CellCoord::new(die, col, row).wire(intdb.get_wire(&wire)),
                    );
                }
            }
            for (idx, wire) in [
                (0, "DOUBLE.V0.2"),
                (num_sd - 1, "DOUBLE.V1.2"),
                (num_sd, "DOUBLE.H1.0"),
                (2 * num_sd - 1, "DOUBLE.H0.0"),
                (2 * num_sd, "DOUBLE.V1.0"),
                (3 * num_sd - 1, "DOUBLE.V0.0"),
                (3 * num_sd, "DOUBLE.H0.2"),
                (4 * num_sd - 1, "DOUBLE.H1.2"),
            ] {
                let net = extractor.box_net(tile.boxes[0], idx);
                extractor.net_int(
                    net,
                    CellCoord::new(die, col, row).wire(intdb.get_wire(wire)),
                );
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
                    "IO.DOUBLE.0.S.2",
                    "IO.DOUBLE.0.S.1",
                    "IO.DOUBLE.1.S.2",
                    "IO.DOUBLE.1.S.1",
                ][..]
            } else {
                &[
                    "IO.DOUBLE.0.S.2",
                    "IO.DOUBLE.0.S.1",
                    "IO.DOUBLE.1.S.2",
                    "IO.DOUBLE.1.S.1",
                    "IO.DOUBLE.2.S.2",
                    "IO.DOUBLE.2.S.1",
                    "IO.DOUBLE.3.S.2",
                    "IO.DOUBLE.3.S.1",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
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
                    "IO.DOUBLE.0.N.0",
                    "IO.DOUBLE.0.N.1",
                    "IO.DOUBLE.1.N.0",
                    "IO.DOUBLE.1.N.1",
                ][..]
            } else {
                &[
                    "IO.DOUBLE.0.N.0",
                    "IO.DOUBLE.0.N.1",
                    "IO.DOUBLE.1.N.0",
                    "IO.DOUBLE.1.N.1",
                    "IO.DOUBLE.2.N.0",
                    "IO.DOUBLE.2.N.1",
                    "IO.DOUBLE.3.N.0",
                    "IO.DOUBLE.3.N.1",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
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
                    "IO.DOUBLE.0.W.0",
                    "IO.DOUBLE.0.W.1",
                    "IO.DOUBLE.1.W.0",
                    "IO.DOUBLE.1.W.1",
                ][..]
            } else {
                &[
                    "IO.DOUBLE.0.W.0",
                    "IO.DOUBLE.0.W.1",
                    "IO.DOUBLE.1.W.0",
                    "IO.DOUBLE.1.W.1",
                    "IO.DOUBLE.2.W.0",
                    "IO.DOUBLE.2.W.1",
                    "IO.DOUBLE.3.W.0",
                    "IO.DOUBLE.3.W.1",
                ][..]
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, CellCoord::new(die, col, row).wire(wire)));
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
                    "IO.DOUBLE.0.E.2",
                    "IO.DOUBLE.0.E.1",
                    "IO.DOUBLE.1.E.2",
                    "IO.DOUBLE.1.E.1",
                ][..]
            } else {
                &[
                    "IO.DOUBLE.0.E.2",
                    "IO.DOUBLE.0.E.1",
                    "IO.DOUBLE.1.E.2",
                    "IO.DOUBLE.1.E.1",
                    "IO.DOUBLE.2.E.2",
                    "IO.DOUBLE.2.E.1",
                    "IO.DOUBLE.3.E.2",
                    "IO.DOUBLE.3.E.1",
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

    // DBUF
    for col in edev.cols(die) {
        if col == chip.col_w() {
            continue;
        }
        for (row, w_h0, w_h1) in [
            (
                chip.row_s(),
                if col == chip.col_e() {
                    "IO.DOUBLE.0.E.1"
                } else {
                    "IO.DOUBLE.0.S.0"
                },
                "IO.DOUBLE.0.S.2",
            ),
            (
                chip.row_n(),
                if col == chip.col_e() {
                    "IO.DOUBLE.0.E.2"
                } else {
                    "IO.DOUBLE.0.N.2"
                },
                "IO.DOUBLE.0.N.0",
            ),
        ] {
            for (w_anchor, w_dbuf) in [(w_h0, "IO.DBUF.H0"), (w_h1, "IO.DBUF.H1")] {
                let w_anchor = intdb.get_wire(w_anchor);
                let w_dbuf = intdb.get_wire(w_dbuf);
                let rw_anchor = edev
                    .resolve_wire(CellCoord::new(die, col, row).wire(w_anchor))
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
                extractor.net_int(net, CellCoord::new(die, col, row).wire(w_dbuf));
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
                    "IO.DOUBLE.0.S.0"
                } else {
                    "IO.DOUBLE.0.W.0"
                },
                "IO.DOUBLE.0.W.2",
            ),
            (
                chip.col_e(),
                if row == chip.row_s() {
                    "IO.DOUBLE.0.S.1"
                } else {
                    "IO.DOUBLE.0.E.2"
                },
                "IO.DOUBLE.0.E.0",
            ),
        ] {
            for (w_anchor, w_dbuf) in [(w_v0, "IO.DBUF.V0"), (w_v1, "IO.DBUF.V1")] {
                let w_anchor = intdb.get_wire(w_anchor);
                let w_dbuf = intdb.get_wire(w_dbuf);
                let rw_anchor = edev
                    .resolve_wire(CellCoord::new(die, col, row).wire(w_anchor))
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
                extractor.net_int(net, CellCoord::new(die, col, row).wire(w_dbuf));
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

    let crd_ll = CellCoord::new(die, chip.col_w(), chip.row_s());
    let crd_ul = CellCoord::new(die, chip.col_w(), chip.row_n());
    let crd_lr = CellCoord::new(die, chip.col_e(), chip.row_s());
    let crd_ur = CellCoord::new(die, chip.col_e(), chip.row_n());
    let i_ll_h = extractor.get_bel_net(crd_ll.bel(bels::BUFGLS_H), "O");
    let i_ll_v = extractor.get_bel_net(crd_ll.bel(bels::BUFGLS_V), "O");
    let i_ul_h = extractor.get_bel_net(crd_ul.bel(bels::BUFGLS_H), "O");
    let i_ul_v = extractor.get_bel_net(crd_ul.bel(bels::BUFGLS_V), "O");
    let i_lr_h = extractor.get_bel_net(crd_lr.bel(bels::BUFGLS_H), "O");
    let i_lr_v = extractor.get_bel_net(crd_lr.bel(bels::BUFGLS_V), "O");
    let i_ur_h = extractor.get_bel_net(crd_ur.bel(bels::BUFGLS_H), "O");
    let i_ur_v = extractor.get_bel_net(crd_ur.bel(bels::BUFGLS_V), "O");
    for (tcrd, tile) in edev.tiles() {
        let cell = tcrd.cell;
        let CellCoord { col, row, .. } = cell;
        let ntile = &endev.ngrid.tiles[&tcrd];
        let tcls = &intdb[tile.class];
        for (slot, bel_info) in &tcls.bels {
            let BelInfo::Bel(bel_info) = bel_info else {
                continue;
            };
            let bel = cell.bel(slot);
            match slot {
                bels::TBUF0 | bels::TBUF1 => {
                    let net_i = extractor.get_bel_int_net(bel, "I");
                    let net_o = extractor.get_bel_int_net(bel, "O");
                    let src_nets = Vec::from_iter(extractor.nets[net_i].pips_bwd.keys().copied());
                    for net in src_nets {
                        extractor.mark_tbuf_pseudo(net_o, net);
                    }
                }
                bels::IO0 | bels::IO1 => {
                    let net_o2 = extractor.get_bel_int_net(bel, "O2");
                    let o1 = bel_info.pins["O1"].wires.iter().copied().next().unwrap();
                    let mut nets = vec![];
                    for net in extractor.nets[net_o2].pips_bwd.keys().copied() {
                        let NetBinding::Int(rw) = extractor.nets[net].binding else {
                            continue;
                        };
                        let wkey = intdb.wires.key(rw.slot);
                        let is_o2 = if col == chip.col_w() || col == chip.col_e() {
                            wkey.starts_with("SINGLE.V")
                                || wkey.starts_with("DOUBLE.V")
                                || wkey.starts_with("LONG.V")
                                || wkey.starts_with("GCLK")
                        } else {
                            wkey.starts_with("SINGLE.H")
                                || wkey.starts_with("DOUBLE.H")
                                || wkey.starts_with("LONG.H")
                        };
                        if !is_o2 {
                            nets.push(net);
                        }
                    }
                    for net in nets {
                        extractor.force_int_pip_dst(net_o2, net, tcrd, o1);
                    }
                }
                bels::CLKH => {
                    let net_o0 = extractor.get_bel_int_net(bel, "O0");
                    let net_o1 = extractor.get_bel_int_net(bel, "O1");
                    let net_o2 = extractor.get_bel_int_net(bel, "O2");
                    let net_o3 = extractor.get_bel_int_net(bel, "O3");
                    for (opin, ipin, onet, inet) in [
                        ("O0", "I.UL.V", net_o0, i_ul_v),
                        ("O1", "I.LL.H", net_o1, i_ll_h),
                        ("O2", "I.LR.V", net_o2, i_lr_v),
                        ("O3", "I.UR.H", net_o3, i_ur_h),
                    ] {
                        let crd = extractor.use_pip(onet, inet);
                        let pip = extractor.xlat_pip_loc(tcrd, crd);
                        extractor.bel_pip(ntile.naming, slot, format!("{opin}.{ipin}"), pip);
                    }
                    for (opin, onet) in [
                        ("O0", net_o0),
                        ("O1", net_o1),
                        ("O2", net_o2),
                        ("O3", net_o3),
                    ] {
                        for (ipin, inet) in [
                            ("I.UL.H", i_ul_h),
                            ("I.LL.V", i_ll_v),
                            ("I.LR.H", i_lr_h),
                            ("I.UR.V", i_ur_v),
                        ] {
                            let crd = extractor.use_pip(onet, inet);
                            let pip = extractor.xlat_pip_loc(tcrd, crd);
                            extractor.bel_pip(ntile.naming, slot, format!("{opin}.{ipin}"), pip);
                        }
                    }
                    if chip.kind == ChipKind::Xc4000H {
                        let net_gnd = extractor.get_bel_net(bel, "GND");
                        for (opin, onet) in [
                            ("O0", net_o0),
                            ("O1", net_o1),
                            ("O2", net_o2),
                            ("O3", net_o3),
                        ] {
                            let crd = extractor.use_pip(onet, net_gnd);
                            let pip = extractor.xlat_pip_loc(tcrd, crd);
                            extractor.bel_pip(ntile.naming, slot, format!("{opin}.GND"), pip);
                        }
                    }
                }
                bels::CLB => {
                    let net_cin = extractor.get_bel_net(bel, "CIN");
                    let net_cout_b = if row != chip.row_s() + 1 {
                        extractor.get_bel_net(cell.delta(0, -1).bel(bels::CLB), "COUT")
                    } else if col != chip.col_w() + 1 {
                        extractor.get_bel_net(cell.delta(-1, 0).bel(bels::CLB), "COUT")
                    } else {
                        extractor.get_bel_net(cell.delta(-1, -1).bel(bels::CIN), "O")
                    };
                    let crd = extractor.use_pip(net_cin, net_cout_b);
                    let pip = extractor.xlat_pip_loc(tcrd, crd);
                    extractor.bel_pip(ntile.naming, slot, "CIN.B", pip);
                    let net_cout_t = if row != chip.row_n() - 1 {
                        extractor.get_bel_net(cell.delta(0, 1).bel(bels::CLB), "COUT")
                    } else if col != chip.col_w() + 1 {
                        extractor.get_bel_net(cell.delta(-1, 0).bel(bels::CLB), "COUT")
                    } else {
                        extractor.get_bel_net(cell.delta(-1, 1).bel(bels::CIN), "O")
                    };
                    let crd = extractor.use_pip(net_cin, net_cout_t);
                    let pip = extractor.xlat_pip_loc(tcrd, crd);
                    extractor.bel_pip(ntile.naming, slot, "CIN.T", pip);
                }
                bels::COUT => {
                    let net_cin = extractor.get_bel_net(bel, "I");
                    let net_cout = extractor.get_bel_net(
                        cell.delta(-1, if row == chip.row_s() { 1 } else { -1 })
                            .bel(bels::CLB),
                        "COUT",
                    );
                    let crd = extractor.use_pip(net_cin, net_cout);
                    let pip = extractor.xlat_pip_loc(tcrd, crd);
                    extractor.bel_pip(ntile.naming, slot, "I", pip);
                }
                _ => (),
            }
        }
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
            PipMode::Pass
        } else {
            let wtn = db.wires.key(wt.wire);
            let wfn = db.wires.key(wf.wire);
            if wtn.starts_with("IMUX")
                || wtn.starts_with("LONG.IO")
                || wtn.starts_with("IO.DBUF")
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
