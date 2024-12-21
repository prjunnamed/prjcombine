use std::collections::{BTreeMap, BTreeSet};

use enum_map::{enum_map, EnumMap};
use prjcombine_int::{
    db::{
        BelId, BelInfo, BelPin, Dir, IntDb, NodeKind, NodeTileId, PinDir, TermInfo, TermKind,
        WireKind,
    },
    grid::{DieId, EdgeIoCoord, LayerId},
};
use prjcombine_xact_data::die::Die;
use prjcombine_xact_naming::db::{NamingDb, NodeNaming};
use prjcombine_xc2000::{
    bond::{Bond, BondPin, CfgPin},
    grid::{Grid, GridKind, SharedCfgPin},
};
use prjcombine_xc2000_xact::{name_device, ExpandedNamedDevice};
use unnamed_entity::{EntityId, EntityVec};

use crate::extractor::{Extractor, NetBinding};

fn bel_from_pins(db: &IntDb, pins: &[(&str, impl AsRef<str>)]) -> BelInfo {
    let mut bel = BelInfo::default();
    let mut has_dec = false;
    for &(name, ref wire) in pins {
        let wire = wire.as_ref();
        if wire.starts_with("DEC") {
            has_dec = true;
        }
        bel.pins.insert(
            name.into(),
            BelPin {
                wires: BTreeSet::from_iter([(NodeTileId::from_idx(0), db.get_wire(wire))]),
                dir: if wire.starts_with("IMUX") {
                    PinDir::Input
                } else {
                    PinDir::Output
                },
                is_intf_in: false,
            },
        );
    }
    if has_dec {
        if let Some(pin) = bel.pins.get_mut("I") {
            pin.dir = PinDir::Input;
        }
    }
    bel
}

pub fn make_intdb(kind: GridKind) -> IntDb {
    let mut db = IntDb::default();
    let mut main_terms = EnumMap::from_fn(|dir| TermKind {
        dir,
        wires: Default::default(),
    });
    let mut cnr_ll_w = TermKind {
        dir: Dir::W,
        wires: Default::default(),
    };
    let cnr_lr_s = TermKind {
        dir: Dir::S,
        wires: Default::default(),
    };
    let mut cnr_ul_n = TermKind {
        dir: Dir::N,
        wires: Default::default(),
    };
    let mut cnr_ur_e = TermKind {
        dir: Dir::E,
        wires: Default::default(),
    };

    db.wires.insert("GND".into(), WireKind::Tie0);

    let single_num = if kind == GridKind::Xc4000A { 4 } else { 8 };
    for (dir, hv) in [(Dir::E, 'H'), (Dir::S, 'V')] {
        for i in 0..single_num {
            let w0 = db
                .wires
                .insert(format!("SINGLE.{hv}{i}"), WireKind::PipOut)
                .0;
            let w1 = db
                .wires
                .insert(format!("SINGLE.{hv}{i}.{dir}"), WireKind::PipBranch(!dir))
                .0;
            main_terms[!dir].wires.insert(w1, TermInfo::PassFar(w0));
        }
    }

    for (dir, hv) in [(Dir::E, 'H'), (Dir::S, 'V')] {
        for i in 0..2 {
            let w0 = db
                .wires
                .insert(format!("DOUBLE.{hv}{i}.0"), WireKind::PipOut)
                .0;
            let w1 = db
                .wires
                .insert(format!("DOUBLE.{hv}{i}.1"), WireKind::PipBranch(!dir))
                .0;
            let w2 = db
                .wires
                .insert(format!("DOUBLE.{hv}{i}.2"), WireKind::PipBranch(!dir))
                .0;
            main_terms[!dir].wires.insert(w1, TermInfo::PassFar(w0));
            main_terms[!dir].wires.insert(w2, TermInfo::PassFar(w1));
        }
    }

    let io_double_num = if kind == GridKind::Xc4000A { 2 } else { 4 };
    let bdir = enum_map!(
        Dir::S => Dir::W,
        Dir::E => Dir::S,
        Dir::N => Dir::E,
        Dir::W => Dir::N,
    );
    for i in 0..io_double_num {
        let mut wires = EnumMap::from_fn(|_| vec![]);

        for j in 0..3 {
            for dir in Dir::DIRS {
                wires[dir].push(
                    db.wires
                        .insert(
                            format!("IO.DOUBLE.{i}.{dir}.{j}"),
                            WireKind::PipBranch(bdir[dir]),
                        )
                        .0,
                );
            }
        }

        for j in 0..2 {
            for dir in Dir::DIRS {
                main_terms[bdir[dir]]
                    .wires
                    .insert(wires[dir][j + 1], TermInfo::PassFar(wires[dir][j]));
            }
            cnr_ul_n
                .wires
                .insert(wires[Dir::W][j], TermInfo::PassNear(wires[Dir::N][j + 1]));
        }
        cnr_ll_w
            .wires
            .insert(wires[Dir::S][1], TermInfo::PassNear(wires[Dir::W][1]));
        cnr_ur_e
            .wires
            .insert(wires[Dir::N][1], TermInfo::PassNear(wires[Dir::E][1]));
    }

    for i in 0..2 {
        db.wires.insert(format!("IO.DBUF.H{i}"), WireKind::MuxOut);
    }
    for i in 0..2 {
        db.wires.insert(format!("IO.DBUF.V{i}"), WireKind::MuxOut);
    }

    let long_num = if kind == GridKind::Xc4000A { 4 } else { 6 };
    for i in 0..long_num {
        let w = db
            .wires
            .insert(format!("LONG.H{i}"), WireKind::MultiBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    }
    for i in 0..long_num {
        let w = db
            .wires
            .insert(format!("LONG.V{i}"), WireKind::MultiBranch(Dir::S))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    }
    let io_long_num = if kind == GridKind::Xc4000A { 2 } else { 4 };
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("LONG.IO.H{i}"), WireKind::MultiBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    }
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("LONG.IO.V{i}"), WireKind::MultiBranch(Dir::S))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    }
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("DEC.H{i}"), WireKind::MultiBranch(Dir::W))
            .0;
        main_terms[Dir::W].wires.insert(w, TermInfo::PassFar(w));
    }
    for i in 0..io_long_num {
        let w = db
            .wires
            .insert(format!("DEC.V{i}"), WireKind::MultiBranch(Dir::S))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
    }

    for i in 0..4 {
        let w = db
            .wires
            .insert(format!("GCLK{i}"), WireKind::MultiBranch(Dir::S))
            .0;
        main_terms[Dir::S].wires.insert(w, TermInfo::PassFar(w));
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
                    .insert(format!("IMUX.CLB.{p}{i}.N"), WireKind::Branch(Dir::S))
                    .0;
                main_terms[Dir::S].wires.insert(wn, TermInfo::PassFar(w));
            }
            if i == 3 {
                let ww = db
                    .wires
                    .insert(format!("IMUX.CLB.{p}{i}.W"), WireKind::Branch(Dir::E))
                    .0;
                main_terms[Dir::E].wires.insert(ww, TermInfo::PassFar(w));
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
        if name.starts_with("OUT.HIOB") && kind != GridKind::Xc4000H {
            continue;
        }
        let w = db.wires.insert(name.into(), WireKind::LogicOut).0;
        for &dir in dirs {
            let wo = db
                .wires
                .insert(format!("{name}.{dir}"), WireKind::Branch(!dir))
                .0;
            main_terms[!dir].wires.insert(wo, TermInfo::PassFar(w));
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
        tclb_n.wires.insert(wt, TermInfo::PassFar(wf));
    }

    let mut lclb_w = main_terms[Dir::W].clone();
    for (wt, wf) in [
        ("OUT.CLB.GY.E", "OUT.LR.IOB1.I2"),
        ("OUT.CLB.GYQ.E", "OUT.LR.IOB0.I2"),
    ] {
        let wt = db.get_wire(wt);
        let wf = db.get_wire(wf);
        lclb_w.wires.insert(wt, TermInfo::PassFar(wf));
    }

    for (dir, term) in main_terms {
        db.terms.insert(format!("MAIN.{dir}"), term);
    }
    for (dir, term) in ll_terms {
        let hv = match dir {
            Dir::W | Dir::E => 'H',
            Dir::S | Dir::N => 'V',
        };
        db.terms.insert(format!("LL{hv}C.{dir}"), term);
    }
    db.terms.insert("TCLB.N".into(), tclb_n);
    db.terms.insert("LCLB.W".into(), lclb_w);
    db.terms.insert("CNR.LL.W".into(), cnr_ll_w);
    db.terms.insert("CNR.LR.S".into(), cnr_lr_s);
    db.terms.insert("CNR.UL.N".into(), cnr_ul_n);
    db.terms.insert("CNR.UR.E".into(), cnr_ur_e);

    for name in [
        "CLB.LT", "CLB.T", "CLB.RT", "CLB.L", "CLB", "CLB.R", "CLB.LB", "CLB.B", "CLB.RB",
    ] {
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([(), (), ()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        node.bels.insert(
            "CLB".into(),
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
            node.bels.insert(
                format!("TBUF{i}"),
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
        db.nodes.insert(name.into(), node);
    }
    for name in [
        "IO.B", "IO.B.R", "IO.BS", "IO.BS.L", "IO.T", "IO.T.R", "IO.TS", "IO.TS.L",
    ] {
        let is_bot = name.starts_with("IO.B");
        let mut node = NodeKind {
            tiles: Default::default(),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let num_tiles = if is_bot { 4 } else { 3 };
        for _ in 0..num_tiles {
            node.tiles.push(());
        }
        if kind != GridKind::Xc4000H {
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
                node.bels
                    .insert(format!("IOB{i}"), bel_from_pins(&db, &pins));
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
                node.bels
                    .insert(format!("HIOB{i}"), bel_from_pins(&db, &pins));
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
            node.bels
                .insert(format!("DEC{i}"), bel_from_pins(&db, &pins));
        }
        db.nodes.insert(name.into(), node);
    }
    for name in [
        "IO.R", "IO.R.T", "IO.RS", "IO.RS.B", "IO.L", "IO.L.T", "IO.LS", "IO.LS.B",
    ] {
        let is_left = name.starts_with("IO.L");
        let mut node = NodeKind {
            tiles: Default::default(),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let num_tiles = if is_left { 4 } else { 3 };
        for _ in 0..num_tiles {
            node.tiles.push(());
        }
        if kind != GridKind::Xc4000H {
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
                node.bels
                    .insert(format!("IOB{i}"), bel_from_pins(&db, &pins));
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
                node.bels
                    .insert(format!("HIOB{i}"), bel_from_pins(&db, &pins));
            }
        }
        for i in 0..2 {
            node.bels.insert(
                format!("TBUF{i}"),
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
            node.bels.insert(
                format!("PULLUP.TBUF{i}"),
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
            node.bels
                .insert(format!("DEC{i}"), bel_from_pins(&db, &pins));
        }
        db.nodes.insert(name.into(), node);
    }
    for (name, num_tiles) in [("CNR.BR", 1), ("CNR.TR", 2), ("CNR.BL", 2), ("CNR.TL", 4)] {
        let mut node = NodeKind {
            tiles: Default::default(),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        for _ in 0..num_tiles {
            node.tiles.push(());
        }
        for hv in ['H', 'V'] {
            for i in 0..io_long_num {
                node.bels.insert(
                    format!("PULLUP.DEC.{hv}{i}"),
                    bel_from_pins(&db, &[("O", format!("DEC.{hv}{i}"))]),
                );
            }
        }
        for hv in ['H', 'V'] {
            node.bels.insert(
                format!("BUFGLS.{hv}"),
                bel_from_pins(&db, &[("I", format!("IMUX.BUFG.{hv}"))]),
            );
        }
        match name {
            "CNR.BR" => {
                node.bels.insert(
                    "COUT.LR".into(),
                    BelInfo {
                        pins: Default::default(),
                    },
                );
                node.bels.insert(
                    "STARTUP".into(),
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
                node.bels.insert(
                    "READCLK".into(),
                    bel_from_pins(&db, &[("I", "IMUX.READCLK.I")]),
                );
            }
            "CNR.TR" => {
                node.bels.insert(
                    "COUT.UR".into(),
                    BelInfo {
                        pins: Default::default(),
                    },
                );
                node.bels.insert(
                    "UPDATE".into(),
                    bel_from_pins(&db, &[("O", "OUT.UPDATE.O")]),
                );
                node.bels.insert(
                    "OSC".into(),
                    bel_from_pins(
                        &db,
                        &[
                            ("F8M", "OUT.LR.IOB1.I1"),
                            ("OUT0", "OUT.LR.IOB1.I2"),
                            ("OUT1", "OUT.OSC.MUX1"),
                        ],
                    ),
                );
                node.bels.insert(
                    "TDO".into(),
                    bel_from_pins(&db, &[("O", "IMUX.TDO.O"), ("T", "IMUX.TDO.T")]),
                );
            }
            "CNR.BL" => {
                node.bels.insert(
                    "CIN.LL".into(),
                    BelInfo {
                        pins: Default::default(),
                    },
                );
                node.bels
                    .insert("MD0".into(), bel_from_pins(&db, &[("I", "OUT.MD0.I")]));
                node.bels.insert(
                    "MD1".into(),
                    bel_from_pins(&db, &[("O", "IMUX.IOB1.O1"), ("T", "IMUX.IOB1.IK")]),
                );
                node.bels
                    .insert("MD2".into(), bel_from_pins(&db, &[("I", "OUT.BT.IOB1.I1")]));
                node.bels.insert(
                    "RDBK".into(),
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
                node.bels.insert(
                    "CIN.UL".into(),
                    BelInfo {
                        pins: Default::default(),
                    },
                );
                node.bels.insert(
                    "BSCAN".into(),
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
        db.nodes.insert(name.into(), node);
    }
    for name in ["LLH.IO.B", "LLH.IO.T", "LLH.CLB", "LLH.CLB.B"] {
        let mut node = NodeKind {
            tiles: Default::default(),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        for _ in 0..2 {
            node.tiles.push(());
        }
        db.nodes.insert(name.into(), node);
    }
    for name in ["LLV.IO.L", "LLV.IO.R", "LLV.CLB"] {
        let mut node = NodeKind {
            tiles: Default::default(),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        for _ in 0..2 {
            node.tiles.push(());
        }
        node.bels.insert(
            "CLKH".into(),
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
        db.nodes.insert(name.into(), node);
    }

    db
}

pub fn make_grid(die: &Die) -> Grid {
    let mut kind = GridKind::Xc4000;
    for pd in die.primdefs.values() {
        if pd.name == "iobh_bl" {
            kind = GridKind::Xc4000H;
        } else if pd.name == "xdecoder2_b" {
            kind = GridKind::Xc4000A;
        }
    }
    Grid {
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

pub fn dump_grid(die: &Die, noblock: &[String]) -> (Grid, IntDb, NamingDb) {
    let mut grid = make_grid(die);
    let mut intdb = make_intdb(grid.kind);
    let mut ndb = NamingDb::default();
    for name in intdb.nodes.keys() {
        ndb.node_namings.insert(name.clone(), NodeNaming::default());
    }
    for (key, kind) in [("L", "left"), ("C", "center"), ("R", "rt"), ("CLK", "clkc")] {
        ndb.tile_widths
            .insert(key.into(), die.tiledefs[kind].matrix.dim().0);
    }
    for (key, kind) in [("B", "bot"), ("C", "center"), ("T", "top"), ("CLK", "clkc")] {
        ndb.tile_heights
            .insert(key.into(), die.tiledefs[kind].matrix.dim().1);
    }
    let edev = grid.expand_grid(&intdb);
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
                    let mut tie = extractor.grab_prim_ab(&nnode.tie_names[0], &nnode.tie_names[1]);
                    let o = tie.get_pin("O");
                    extractor.net_int(o, (DieId::from_idx(0), (col, row), intdb.get_wire("GND")));
                }
                let tile =
                    &extractor.die.newtiles[&(endev.col_x[col].start, endev.row_y[row].start)];
                if nloc.3 == LayerId::from_idx(0) {
                    for &box_id in &tile.boxes {
                        extractor.own_box(box_id, nloc);
                    }
                }
                for (bel, key, bel_info) in &node_kind.bels {
                    if key == "CLKH" && grid.kind != GridKind::Xc4000H {
                        continue;
                    }
                    let bel_names = &nnode.bels[bel];
                    match &key[..] {
                        _ if key.starts_with("PULLUP") => {
                            let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let (line, pip) = extractor.consume_one_fwd(o, nloc);
                            extractor.net_bel_int(line, nloc, bel, "O");
                            extractor.bel_pip(nnode.naming, bel, "O", pip);
                        }
                        "BUFGLS.H" | "BUFGLS.V" => {
                            let mut prim = extractor.grab_prim_a(&bel_names[0]);
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let i = prim.get_pin("I");
                            extractor.net_bel_int(i, nloc, bel, "I");
                        }
                        "CIN.LL" | "CIN.UL" => {
                            let mut prim = extractor.grab_prim_a(&bel_names[0]);
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                        }
                        "COUT.LR" | "COUT.UR" => {
                            let mut prim = extractor.grab_prim_a(&bel_names[0]);
                            let i = prim.get_pin("I");
                            extractor.net_bel(i, nloc, bel, "I");
                        }
                        "MD0" | "MD1" | "MD2" | "RDBK" | "BSCAN" | "STARTUP" | "READCLK"
                        | "UPDATE" | "TDO" => {
                            let mut prim = extractor.grab_prim_a(&bel_names[0]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                        }
                        "OSC" => {
                            let mut prim = extractor.grab_prim_a(&bel_names[0]);
                            extractor.net_bel_int(prim.get_pin("F8M"), nloc, bel, "F8M");
                            for pin in ["F500K", "F16K", "F490", "F15"] {
                                let o = prim.get_pin(pin);
                                extractor.net_bel(o, nloc, bel, pin);
                                let mut o = extractor.consume_all_fwd(o, nloc);
                                o.sort_by_key(|(_, pip)| pip.y);
                                extractor.net_bel_int(o[0].0, nloc, bel, "OUT0");
                                extractor.net_bel_int(o[1].0, nloc, bel, "OUT1");
                                extractor.bel_pip(nnode.naming, bel, format!("OUT0.{pin}"), o[0].1);
                                extractor.bel_pip(nnode.naming, bel, format!("OUT1.{pin}"), o[1].1);
                            }
                        }
                        "IOB0" | "IOB1" => {
                            let mut prim = extractor.grab_prim_i(&bel_names[0]);
                            for pin in ["I1", "I2", "IK", "OK", "T"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            // not quite true, but we'll fix it up.
                            extractor.net_bel_int(prim.get_pin("O"), nloc, bel, "O2");
                            if bel_names.len() > 1 {
                                let mut prim = extractor.grab_prim_a(&bel_names[1]);
                                extractor.net_bel_int(prim.get_pin("I"), nloc, bel, "CLKIN");
                            }
                        }
                        "HIOB0" | "HIOB1" | "HIOB2" | "HIOB3" => {
                            let mut prim = extractor.grab_prim_i(&bel_names[0]);
                            extractor.net_bel_int(prim.get_pin("TS"), nloc, bel, "T2");
                            let tp = prim.get_pin("TP");
                            extractor.net_bel(tp, nloc, bel, "T1");
                            let (line, pip) = extractor.consume_one_bwd(tp, nloc);
                            extractor.net_bel_int(line, nloc, bel, "T1");
                            extractor.bel_pip(nnode.naming, bel, "T1", pip);

                            // O1/O2
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let mut o = extractor.consume_all_bwd(o, nloc);
                            assert_eq!(o.len(), 2);
                            if col == grid.col_lio() {
                                o.sort_by_key(|(_, pip)| pip.x);
                            } else if col == grid.col_rio() {
                                o.sort_by_key(|(_, pip)| !pip.x);
                            } else if row == grid.row_bio() {
                                o.sort_by_key(|(_, pip)| pip.y);
                            } else if row == grid.row_tio() {
                                o.sort_by_key(|(_, pip)| !pip.y);
                            }
                            extractor.net_bel_int(o[0].0, nloc, bel, "O1");
                            extractor.net_bel_int(o[1].0, nloc, bel, "O2");
                            extractor.bel_pip(nnode.naming, bel, "O1", o[0].1);
                            extractor.bel_pip(nnode.naming, bel, "O2", o[1].1);

                            // I1/I2
                            let net_i = prim.get_pin("I");
                            extractor.net_bel_int(net_i, nloc, bel, "I");
                            let mut i = vec![];
                            for (&net, &pip) in &extractor.nets[net_i].pips_fwd {
                                i.push((net, pip));
                            }
                            assert_eq!(i.len(), 2);
                            if col == grid.col_lio() {
                                i.sort_by_key(|(_, pip)| pip.0);
                            } else if col == grid.col_rio() {
                                i.sort_by_key(|(_, pip)| !pip.0);
                            } else if row == grid.row_bio() {
                                i.sort_by_key(|(_, pip)| pip.1);
                            } else if row == grid.row_tio() {
                                i.sort_by_key(|(_, pip)| !pip.1);
                            }
                            let lrbt = if col == grid.col_lio() || col == grid.col_rio() {
                                "LR"
                            } else {
                                "BT"
                            };
                            let ii = match &key[..] {
                                "HIOB0" | "HIOB1" => 0,
                                "HIOB2" | "HIOB3" => 1,
                                _ => unreachable!(),
                            };
                            extractor.net_int(
                                i[0].0,
                                (
                                    die.die,
                                    (col, row),
                                    intdb.get_wire(&format!("OUT.{lrbt}.IOB{ii}.I1")),
                                ),
                            );
                            extractor.net_int(
                                i[1].0,
                                (
                                    die.die,
                                    (col, row),
                                    intdb.get_wire(&format!("OUT.{lrbt}.IOB{ii}.I2")),
                                ),
                            );

                            if bel_names.len() > 1 {
                                let mut prim = extractor.grab_prim_a(&bel_names[1]);
                                extractor.net_bel_int(prim.get_pin("I"), nloc, bel, "CLKIN");
                            }
                        }
                        "TBUF0" | "TBUF1" => {
                            let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                            for pin in ["I", "T"] {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "O");
                            let (line, pip) = extractor.consume_one_fwd(o, nloc);
                            extractor.net_bel_int(line, nloc, bel, "O");
                            extractor.bel_pip(nnode.naming, bel, "O", pip);
                        }
                        "DEC0" | "DEC1" | "DEC2" => {
                            let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                            for pin in bel_info.pins.keys() {
                                if pin.starts_with('O') {
                                    let o = prim.get_pin(pin);
                                    extractor.net_bel(o, nloc, bel, pin);
                                    let (line, pip) = extractor.consume_one_fwd(o, nloc);
                                    extractor.bel_pip(nnode.naming, bel, pin, pip);
                                    extractor.net_bel_int(line, nloc, bel, pin);
                                }
                            }
                            let i = prim.get_pin("I");
                            if key == "DEC1" {
                                extractor.net_bel_int(i, nloc, bel, "I");
                            } else {
                                extractor.net_bel(i, nloc, bel, "I");
                                let (line, pip) = extractor.consume_one_bwd(i, nloc);
                                extractor.net_bel_int(line, nloc, bel, "I");
                                extractor.bel_pip(nnode.naming, bel, "I", pip);
                            }
                        }
                        "CLB" => {
                            let mut prim = extractor.grab_prim_ab(&bel_names[0], &bel_names[1]);
                            for pin in bel_info.pins.keys() {
                                extractor.net_bel_int(prim.get_pin(pin), nloc, bel, pin);
                            }
                            let cin = prim.get_pin("CIN");
                            extractor.net_bel(cin, nloc, bel, "CIN");
                            let cout = prim.get_pin("COUT");
                            extractor.net_bel(cout, nloc, bel, "COUT");
                        }
                        "CLKH" => {
                            let mut prim = extractor.grab_prim_a(&bel_names[0]);
                            let o = prim.get_pin("O");
                            extractor.net_bel(o, nloc, bel, "GND");
                        }
                        _ => panic!("umm bel {key}?"),
                    }
                }
            }
        }
    }
    extractor.grab_prim_a("_cfg4000_");

    // long verticals + GCLK
    for col in die.cols() {
        let mut queue = vec![];
        for row in [grid.row_mid() - 1, grid.row_mid()] {
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
            let wires = if col == grid.col_lio() {
                if grid.kind == GridKind::Xc4000A {
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
            } else if col == grid.col_rio() {
                if grid.kind == GridKind::Xc4000A {
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
                if grid.kind == GridKind::Xc4000A {
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
        for col in [grid.col_mid() - 1, grid.col_mid()] {
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
            let wires = if row == grid.row_bio() {
                if grid.kind == GridKind::Xc4000A {
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
            } else if row == grid.row_tio() {
                if grid.kind == GridKind::Xc4000A {
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
                if grid.kind == GridKind::Xc4000A {
                    &["LONG.H0", "LONG.H3"][..]
                } else {
                    &["LONG.H0", "LONG.H1", "LONG.H4", "LONG.H5"][..]
                }
            };
            assert_eq!(nets.len(), wires.len());
            for (net, wire) in nets.into_iter().zip(wires.iter().copied()) {
                let wire = intdb.get_wire(wire);
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        for (net, wire) in queue {
            extractor.net_int(net, wire);
        }
    }

    // boxes — pin single and double wires
    for col in die.cols() {
        if col == grid.col_lio() {
            continue;
        }
        for row in die.rows() {
            if row == grid.row_tio() {
                continue;
            }
            let tile = &extractor.die.newtiles[&(endev.col_x[col].start, endev.row_y[row].start)];
            assert_eq!(tile.boxes.len(), 1);
            let num_singles = if grid.kind == GridKind::Xc4000A { 4 } else { 8 };
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
                    extractor.net_int(net, (die.die, (col, row), intdb.get_wire(&wire)));
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
                extractor.net_int(net, (die.die, (col, row), intdb.get_wire(wire)));
            }
        }
    }

    // io doubles
    let mut queue = vec![];
    for col in die.cols() {
        if col == grid.col_lio() {
            continue;
        }
        let x = endev.col_x[col].start;
        {
            let row = grid.row_bio();
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
            let wires = if grid.kind == GridKind::Xc4000A {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        {
            let row = grid.row_tio();
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
            let wires = if grid.kind == GridKind::Xc4000A {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
    }
    for row in die.rows() {
        if row == grid.row_bio() {
            continue;
        }
        let y = endev.row_y[row].start;
        {
            let col = grid.col_lio();
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
            let wires = if grid.kind == GridKind::Xc4000A {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
        {
            let col = grid.col_rio();
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
            let wires = if grid.kind == GridKind::Xc4000A {
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
                queue.push((net, (die.die, (col, row), wire)));
            }
        }
    }
    for (net, wire) in queue {
        extractor.net_int(net, wire);
    }

    // DBUF
    for col in die.cols() {
        if col == grid.col_lio() {
            continue;
        }
        for (row, w_h0, w_h1) in [
            (
                grid.row_bio(),
                if col == grid.col_rio() {
                    "IO.DOUBLE.0.E.1"
                } else {
                    "IO.DOUBLE.0.S.0"
                },
                "IO.DOUBLE.0.S.2",
            ),
            (
                grid.row_tio(),
                if col == grid.col_rio() {
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
                    .egrid
                    .resolve_wire((die.die, (col, row), w_anchor))
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
                extractor.net_int(net, (die.die, (col, row), w_dbuf));
            }
        }
    }
    for row in die.rows() {
        if row == grid.row_tio() {
            continue;
        }
        for (col, w_v0, w_v1) in [
            (
                grid.col_lio(),
                if row == grid.row_bio() {
                    "IO.DOUBLE.0.S.0"
                } else {
                    "IO.DOUBLE.0.W.0"
                },
                "IO.DOUBLE.0.W.2",
            ),
            (
                grid.col_rio(),
                if row == grid.row_bio() {
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
                    .egrid
                    .resolve_wire((die.die, (col, row), w_anchor))
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
                extractor.net_int(net, (die.die, (col, row), w_dbuf));
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
        for col in die.cols() {
            for row in die.rows() {
                let rw = edev
                    .egrid
                    .resolve_wire((die.die, (col, row), wire))
                    .unwrap();
                if extractor.int_nets.contains_key(&rw) {
                    extractor.own_mux(rw, (die.die, col, row, LayerId::from_idx(0)));
                }
            }
        }
    }

    let nloc_ll = (
        die.die,
        grid.col_lio(),
        grid.row_bio(),
        LayerId::from_idx(0),
    );
    let nloc_ul = (
        die.die,
        grid.col_lio(),
        grid.row_tio(),
        LayerId::from_idx(0),
    );
    let nloc_lr = (
        die.die,
        grid.col_rio(),
        grid.row_bio(),
        LayerId::from_idx(0),
    );
    let nloc_ur = (
        die.die,
        grid.col_rio(),
        grid.row_tio(),
        LayerId::from_idx(0),
    );
    let bidx = if grid.kind == GridKind::Xc4000A { 4 } else { 8 };
    let i_ll_h = extractor.get_bel_net(nloc_ll, BelId::from_idx(bidx), "O");
    let i_ll_v = extractor.get_bel_net(nloc_ll, BelId::from_idx(bidx + 1), "O");
    let i_ul_h = extractor.get_bel_net(nloc_ul, BelId::from_idx(bidx), "O");
    let i_ul_v = extractor.get_bel_net(nloc_ul, BelId::from_idx(bidx + 1), "O");
    let i_lr_h = extractor.get_bel_net(nloc_lr, BelId::from_idx(bidx), "O");
    let i_lr_v = extractor.get_bel_net(nloc_lr, BelId::from_idx(bidx + 1), "O");
    let i_ur_h = extractor.get_bel_net(nloc_ur, BelId::from_idx(bidx), "O");
    let i_ur_v = extractor.get_bel_net(nloc_ur, BelId::from_idx(bidx + 1), "O");
    for col in die.cols() {
        for row in die.rows() {
            for (layer, node) in &die[(col, row)].nodes {
                let nloc = (die.die, col, row, layer);
                let nnode = &endev.ngrid.nodes[&nloc];
                let node_kind = &intdb.nodes[node.kind];
                for (bel, key, bel_info) in &node_kind.bels {
                    match &key[..] {
                        "TBUF0" | "TBUF1" => {
                            let net_i = extractor.get_bel_int_net(nloc, bel, "I");
                            let net_o = extractor.get_bel_int_net(nloc, bel, "O");
                            let src_nets =
                                Vec::from_iter(extractor.nets[net_i].pips_bwd.keys().copied());
                            for net in src_nets {
                                extractor.mark_tbuf_pseudo(net_o, net);
                            }
                        }
                        "IOB0" | "IOB1" => {
                            let net_o2 = extractor.get_bel_int_net(nloc, bel, "O2");
                            let o1 = bel_info.pins["O1"].wires.iter().copied().next().unwrap();
                            let mut nets = vec![];
                            for net in extractor.nets[net_o2].pips_bwd.keys().copied() {
                                let NetBinding::Int(rw) = extractor.nets[net].binding else {
                                    continue;
                                };
                                let wkey = intdb.wires.key(rw.2);
                                let is_o2 = if col == grid.col_lio() || col == grid.col_rio() {
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
                                extractor.force_int_pip_dst(net_o2, net, nloc, o1);
                            }
                        }
                        "CLKH" => {
                            let net_o0 = extractor.get_bel_int_net(nloc, bel, "O0");
                            let net_o1 = extractor.get_bel_int_net(nloc, bel, "O1");
                            let net_o2 = extractor.get_bel_int_net(nloc, bel, "O2");
                            let net_o3 = extractor.get_bel_int_net(nloc, bel, "O3");
                            for (opin, ipin, onet, inet) in [
                                ("O0", "I.UL.V", net_o0, i_ul_v),
                                ("O1", "I.LL.H", net_o1, i_ll_h),
                                ("O2", "I.LR.V", net_o2, i_lr_v),
                                ("O3", "I.UR.H", net_o3, i_ur_h),
                            ] {
                                let crd = extractor.use_pip(onet, inet);
                                let pip = extractor.xlat_pip_loc(nloc, crd);
                                extractor.bel_pip(nnode.naming, bel, format!("{opin}.{ipin}"), pip);
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
                                    let pip = extractor.xlat_pip_loc(nloc, crd);
                                    extractor.bel_pip(
                                        nnode.naming,
                                        bel,
                                        format!("{opin}.{ipin}"),
                                        pip,
                                    );
                                }
                            }
                            if grid.kind == GridKind::Xc4000H {
                                let net_gnd = extractor.get_bel_net(nloc, bel, "GND");
                                for (opin, onet) in [
                                    ("O0", net_o0),
                                    ("O1", net_o1),
                                    ("O2", net_o2),
                                    ("O3", net_o3),
                                ] {
                                    let crd = extractor.use_pip(onet, net_gnd);
                                    let pip = extractor.xlat_pip_loc(nloc, crd);
                                    extractor.bel_pip(
                                        nnode.naming,
                                        bel,
                                        format!("{opin}.GND"),
                                        pip,
                                    );
                                }
                            }
                        }
                        "CLB" => {
                            let net_cin = extractor.get_bel_net(nloc, bel, "CIN");
                            let net_cout_b = if row != grid.row_bio() + 1 {
                                extractor.get_bel_net(
                                    (die.die, col, row - 1, LayerId::from_idx(0)),
                                    bel,
                                    "COUT",
                                )
                            } else if col != grid.col_lio() + 1 {
                                extractor.get_bel_net(
                                    (die.die, col - 1, row, LayerId::from_idx(0)),
                                    bel,
                                    "COUT",
                                )
                            } else {
                                extractor.get_bel_net(
                                    (die.die, col - 1, row - 1, LayerId::from_idx(0)),
                                    BelId::from_idx(bidx + 2),
                                    "O",
                                )
                            };
                            let crd = extractor.use_pip(net_cin, net_cout_b);
                            let pip = extractor.xlat_pip_loc(nloc, crd);
                            extractor.bel_pip(nnode.naming, bel, "CIN.B", pip);
                            let net_cout_t = if row != grid.row_tio() - 1 {
                                extractor.get_bel_net(
                                    (die.die, col, row + 1, LayerId::from_idx(0)),
                                    bel,
                                    "COUT",
                                )
                            } else if col != grid.col_lio() + 1 {
                                extractor.get_bel_net(
                                    (die.die, col - 1, row, LayerId::from_idx(0)),
                                    bel,
                                    "COUT",
                                )
                            } else {
                                extractor.get_bel_net(
                                    (die.die, col - 1, row + 1, LayerId::from_idx(0)),
                                    BelId::from_idx(bidx + 2),
                                    "O",
                                )
                            };
                            let crd = extractor.use_pip(net_cin, net_cout_t);
                            let pip = extractor.xlat_pip_loc(nloc, crd);
                            extractor.bel_pip(nnode.naming, bel, "CIN.T", pip);
                        }
                        "COUT.LR" => {
                            let net_cin = extractor.get_bel_net(nloc, bel, "I");
                            let net_cout = extractor.get_bel_net(
                                (die.die, col - 1, row + 1, LayerId::from_idx(0)),
                                BelId::from_idx(0),
                                "COUT",
                            );
                            let crd = extractor.use_pip(net_cin, net_cout);
                            let pip = extractor.xlat_pip_loc(nloc, crd);
                            extractor.bel_pip(nnode.naming, bel, "I", pip);
                        }
                        "COUT.UR" => {
                            let net_cin = extractor.get_bel_net(nloc, bel, "I");
                            let net_cout = extractor.get_bel_net(
                                (die.die, col - 1, row - 1, LayerId::from_idx(0)),
                                BelId::from_idx(0),
                                "COUT",
                            );
                            let crd = extractor.use_pip(net_cin, net_cout);
                            let pip = extractor.xlat_pip_loc(nloc, crd);
                            extractor.bel_pip(nnode.naming, bel, "I", pip);
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    let io_lookup: BTreeMap<_, _> = endev
        .grid
        .get_bonded_ios()
        .into_iter()
        .map(|io| (endev.get_io_name(io).to_string(), io))
        .collect();

    let finisher = extractor.finish();
    finisher.finish(&mut intdb, &mut ndb);

    for pad in noblock {
        let io = io_lookup[pad];
        grid.unbonded_io.insert(io);
    }

    (grid, intdb, ndb)
}

pub fn make_bond(
    endev: &ExpandedNamedDevice,
    name: &str,
    pkg: &BTreeMap<String, String>,
) -> (Bond, BTreeMap<SharedCfgPin, EdgeIoCoord>) {
    let io_lookup: BTreeMap<_, _> = endev
        .grid
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
    assert_eq!(bond.pins.insert(m0.into(), BondPin::Cfg(CfgPin::M0)), None);
    assert_eq!(bond.pins.insert(m1.into(), BondPin::Cfg(CfgPin::M1)), None);
    assert_eq!(bond.pins.insert(m2.into(), BondPin::Cfg(CfgPin::M2)), None);
    assert_eq!(
        bond.pins.insert(prog.into(), BondPin::Cfg(CfgPin::ProgB)),
        None
    );
    assert_eq!(
        bond.pins.insert(done.into(), BondPin::Cfg(CfgPin::Done)),
        None
    );
    assert_eq!(
        bond.pins.insert(cclk.into(), BondPin::Cfg(CfgPin::Cclk)),
        None
    );
    assert_eq!(
        bond.pins.insert(tdo.into(), BondPin::Cfg(CfgPin::Tdo)),
        None
    );
    for &pin in gnd {
        assert_eq!(bond.pins.insert(pin.into(), BondPin::Gnd), None);
    }
    for &pin in vcc {
        assert_eq!(bond.pins.insert(pin.into(), BondPin::Vcc), None);
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
            bond.pins.entry(format!("P{i}")).or_insert(BondPin::Nc);
        }
        assert_eq!(bond.pins.len(), len1d);
    }
    match name {
        "bg225" => {
            for a in [
                "A", "B", "C", "D", "E", "F", "G", "H", "J", "K", "L", "M", "N", "P", "R",
            ] {
                for i in 1..=15 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 225);
        }
        "pg120" => {
            for a in ["A", "B", "C", "L", "M", "N"] {
                for i in 1..=13 {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            for a in ["D", "E", "F", "G", "H", "J", "K"] {
                for i in (1..=3).chain(11..=13) {
                    bond.pins.entry(format!("{a}{i}")).or_insert(BondPin::Nc);
                }
            }
            assert_eq!(bond.pins.len(), 120);
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
        _ => (),
    }

    let mut pkg_cfg_io = vec![];
    match name {
        "pc84" => {
            pkg_cfg_io.extend([
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
                ("P77", SharedCfgPin::Addr(0)),
                ("P78", SharedCfgPin::Addr(1)),
                ("P79", SharedCfgPin::Addr(2)),
                ("P80", SharedCfgPin::Addr(3)),
                ("P81", SharedCfgPin::Addr(4)),
                ("P82", SharedCfgPin::Addr(5)),
                ("P83", SharedCfgPin::Addr(6)),
                ("P84", SharedCfgPin::Addr(7)),
            ]);
        }
        "pq160" | "mq160" => {
            pkg_cfg_io.extend([
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
                ("P123", SharedCfgPin::Addr(0)),
                ("P124", SharedCfgPin::Addr(1)),
                ("P127", SharedCfgPin::Addr(2)),
                ("P128", SharedCfgPin::Addr(3)),
                ("P134", SharedCfgPin::Addr(4)),
                ("P135", SharedCfgPin::Addr(5)),
                ("P139", SharedCfgPin::Addr(6)),
                ("P140", SharedCfgPin::Addr(7)),
            ]);
        }
        "pq208" | "mq208" => {
            pkg_cfg_io.extend([
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
                ("P161", SharedCfgPin::Addr(0)),
                ("P162", SharedCfgPin::Addr(1)),
                ("P165", SharedCfgPin::Addr(2)),
                ("P166", SharedCfgPin::Addr(3)),
                ("P174", SharedCfgPin::Addr(4)),
                ("P175", SharedCfgPin::Addr(5)),
                ("P180", SharedCfgPin::Addr(6)),
                ("P181", SharedCfgPin::Addr(7)),
            ]);
        }
        "pq240" | "mq240" => {
            pkg_cfg_io.extend([
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
                ("P183", SharedCfgPin::Addr(0)),
                ("P184", SharedCfgPin::Addr(1)),
                ("P187", SharedCfgPin::Addr(2)),
                ("P188", SharedCfgPin::Addr(3)),
                ("P202", SharedCfgPin::Addr(4)),
                ("P203", SharedCfgPin::Addr(5)),
                ("P209", SharedCfgPin::Addr(6)),
                ("P210", SharedCfgPin::Addr(7)),
            ]);
        }
        _ => (),
    }
    let mut cfg_io = BTreeMap::new();
    for (pin, fun) in pkg_cfg_io {
        let BondPin::Io(io) = bond.pins[pin] else {
            unreachable!()
        };
        cfg_io.insert(fun, io);
    }

    (bond, cfg_io)
}
