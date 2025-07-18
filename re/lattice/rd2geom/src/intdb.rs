use std::collections::BTreeSet;

use prjcombine_ecp::{bels, expanded::REGION_PCLK, tslots};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, CellSlotId, ConnectorClass, ConnectorSlot, ConnectorSlotId,
        ConnectorWire, IntDb, PinDir, SwitchBox, TileClass, TileWireCoord, WireKind,
    },
    dir::{Dir, DirMap},
};
use unnamed_entity::EntityId;

fn add_input(db: &IntDb, bel: &mut Bel, name: &str, cell: usize, wire: &str) {
    bel.pins.insert(
        name.into(),
        BelPin {
            wires: BTreeSet::from_iter([TileWireCoord {
                cell: CellSlotId::from_idx(cell),
                wire: db.get_wire(wire),
            }]),
            dir: PinDir::Input,
            is_intf_in: false,
        },
    );
}

fn add_output(db: &IntDb, bel: &mut Bel, name: &str, cell: usize, wire: &str) {
    bel.pins.insert(
        name.into(),
        BelPin {
            wires: BTreeSet::from_iter([TileWireCoord {
                cell: CellSlotId::from_idx(cell),
                wire: db.get_wire(wire),
            }]),
            dir: PinDir::Output,
            is_intf_in: false,
        },
    );
}

pub fn init_intdb(family: &str) -> IntDb {
    let mut db = IntDb::default();

    assert_eq!(db.region_slots.insert("PCLK".into()).0, REGION_PCLK);
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

    let conn_slots = DirMap::from_fn(|dir| match dir {
        Dir::W => slot_w,
        Dir::E => slot_e,
        Dir::S => slot_s,
        Dir::N => slot_n,
    });

    let mut passes = DirMap::from_fn(|dir| ConnectorClass {
        slot: conn_slots[dir],
        wires: Default::default(),
    });

    let mut terms = DirMap::from_fn(|dir| ConnectorClass {
        slot: conn_slots[dir],
        wires: Default::default(),
    });

    for n in [
        "KEEP_W", "KEEP_E", "KEEP_S0", "KEEP_S1", "KEEP_N0", "KEEP_N1",
    ] {
        db.wires.insert(n.into(), WireKind::MuxOut);
    }

    for dir in Dir::DIRS {
        for i in 0..4 {
            let w0 = db
                .wires
                .insert(format!("X0_{dir}{i}_0"), WireKind::MuxOut)
                .0;
            let w1 = db
                .wires
                .insert(format!("X0_{dir}{i}_1"), WireKind::Branch(conn_slots[!dir]))
                .0;
            passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
        }
    }

    for dir in Dir::DIRS {
        for i in 0..2 {
            let w0 = db
                .wires
                .insert(format!("X1_{dir}{i}_0"), WireKind::MuxOut)
                .0;
            let w1 = db
                .wires
                .insert(format!("X1_{dir}{i}_1"), WireKind::Branch(conn_slots[!dir]))
                .0;
            passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
            let w0 = db
                .wires
                .insert(format!("X2_{dir}{i}_0"), WireKind::MuxOut)
                .0;
            let w1 = db
                .wires
                .insert(format!("X2_{dir}{i}_1"), WireKind::Branch(conn_slots[!dir]))
                .0;
            let w2 = db
                .wires
                .insert(format!("X2_{dir}{i}_2"), WireKind::Branch(conn_slots[!dir]))
                .0;
            passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
            passes[!dir].wires.insert(w2, ConnectorWire::Pass(w1));
        }
    }

    for dir in Dir::DIRS {
        for i in 0..8 {
            for seg in 0..2 {
                let w0 = db.get_wire(&format!("X2_{dir}{i}_{seg}"));
                let w1 = db.get_wire(&format!("X2_{ndir}{i}_{nseg}", ndir = !dir, nseg = seg + 1));
                terms[dir].wires.insert(w1, ConnectorWire::Reflect(w0));
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..4 {
            let mut w = db
                .wires
                .insert(format!("X6_{dir}{i}_0"), WireKind::MuxOut)
                .0;
            for j in 1..=6 {
                let nw = db
                    .wires
                    .insert(
                        format!("X6_{dir}{i}_{j}"),
                        WireKind::Branch(conn_slots[!dir]),
                    )
                    .0;
                passes[!dir].wires.insert(nw, ConnectorWire::Pass(w));
                w = nw;
            }
        }
    }

    for dir in Dir::DIRS {
        for i in 0..4 {
            for seg in 0..6 {
                let w0 = db.get_wire(&format!("X6_{dir}{i}_{seg}"));
                let w1 = db.get_wire(&format!("X6_{ndir}{i}_{nseg}", ndir = !dir, nseg = seg + 1));
                terms[dir].wires.insert(w1, ConnectorWire::Reflect(w0));
            }
        }
    }

    for i in 0..4 {
        db.wires
            .insert(format!("PCLK{i}"), WireKind::Regional(REGION_PCLK));
    }
    for i in 0..4 {
        db.wires
            .insert(format!("SCLK{i}"), WireKind::Regional(REGION_PCLK));
    }

    for l in ['A', 'B', 'C', 'D', 'M'] {
        for i in 0..8 {
            db.wires.insert(format!("IMUX_{l}{i}"), WireKind::MuxOut);
        }
    }
    for l in ["CLK", "LSR", "CE"] {
        for i in 0..4 {
            db.wires.insert(format!("IMUX_{l}{i}"), WireKind::MuxOut);
        }
    }

    for l in ["F", "Q", "OFX"] {
        for i in 0..8 {
            let w = db.wires.insert(format!("OUT_{l}{i}"), WireKind::LogicOut).0;
            #[allow(non_contiguous_range_endpoints)]
            match (l, i) {
                ("OFX", 3) | ("F", 0..3) => {
                    let w_w = db
                        .wires
                        .insert(format!("OUT_{l}{i}_W"), WireKind::Branch(slot_e))
                        .0;
                    passes[Dir::E].wires.insert(w_w, ConnectorWire::Pass(w));
                }
                ("F", 4..8) => {
                    let w_e = db
                        .wires
                        .insert(format!("OUT_{l}{i}_E"), WireKind::Branch(slot_w))
                        .0;
                    passes[Dir::W].wires.insert(w_e, ConnectorWire::Pass(w));
                }
                _ => (),
            }
        }
    }
    for i in 0..12 {
        db.wires.insert(format!("OUT_TI{i}"), WireKind::LogicOut);
    }

    for (dir, pass) in passes {
        db.conn_classes.insert(format!("PASS_{dir}"), pass);
    }

    for (dir, pass) in terms {
        db.conn_classes.insert(format!("TERM_{dir}"), pass);
    }

    let int_tiles = if family == "machxo" {
        [
            "INT_PLC",
            "INT_SIO_W",
            "INT_SIO_W_CLK",
            "INT_SIO_E",
            "INT_SIO_E_CFG",
            "INT_SIO_S4",
            "INT_SIO_S6",
            "INT_SIO_N4",
            "INT_SIO_N6",
            "INT_SIO_XW",
        ]
        .as_slice()
    } else {
        ["INT_PLC", "INT_IO_WE", "INT_IO_SN", "INT_EBR", "INT_PLL"].as_slice()
    };
    for &name in int_tiles {
        let mut tcls = TileClass::new(tslots::INT, 1);
        tcls.bels
            .insert(bels::INT, BelInfo::SwitchBox(SwitchBox::default()));
        db.tile_classes.insert(name.to_string(), tcls);
    }

    for name in ["PLC", "FPLC"] {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        for i in 0..4 {
            let mut bel = Bel::default();
            let i0 = 2 * i;
            let i1 = 2 * i + 1;
            for l in ['A', 'B', 'C', 'D', 'M'] {
                add_input(&db, &mut bel, &format!("{l}0"), 0, &format!("IMUX_{l}{i0}"));
                add_input(&db, &mut bel, &format!("{l}1"), 0, &format!("IMUX_{l}{i1}"));
            }
            add_input(&db, &mut bel, "CLK", 0, &format!("IMUX_CLK{i}"));
            add_input(&db, &mut bel, "LSR", 0, &format!("IMUX_LSR{i}"));
            add_input(&db, &mut bel, "CE", 0, &format!("IMUX_CE{i}"));
            for l in ["F", "Q", "OFX"] {
                add_output(&db, &mut bel, &format!("{l}0"), 0, &format!("OUT_{l}{i0}"));
                add_output(&db, &mut bel, &format!("{l}1"), 0, &format!("OUT_{l}{i1}"));
            }
            tcls.bels.insert(bels::SLICE[i], BelInfo::Bel(bel));
        }
        db.tile_classes.insert(name.to_string(), tcls);
    }

    if family == "machxo" {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        tcls.bels
            .insert(bels::CIBTEST_SEL, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("CIBTEST_SEL".to_string(), tcls);
    }

    if family == "machxo" {
        let mut tcls = TileClass::new(tslots::BEL, 4);
        tcls.bels
            .insert(bels::EBR0, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("EBR".to_string(), tcls);
    } else {
        let mut tcls = TileClass::new(tslots::BEL, 2);
        tcls.bels
            .insert(bels::EBR0, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("EBR".to_string(), tcls);
    }

    if family == "ecp" {
        let mut tcls = TileClass::new(tslots::BEL, 8);
        tcls.bels
            .insert(bels::DSP0, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("DSP".to_string(), tcls);

        for (name, num_cells) in [("CONFIG_S", 4), ("CONFIG_L", 5)] {
            let mut tcls = TileClass::new(tslots::BEL, num_cells);
            tcls.bels
                .insert(bels::START, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::OSC, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::JTAG, BelInfo::Bel(Default::default()));
            // RDBK exists as stub?
            tcls.bels
                .insert(bels::GSR, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
    } else if family == "xp" {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        tcls.bels
            .insert(bels::START, BelInfo::Bel(Default::default()));
        tcls.bels
            .insert(bels::JTAG, BelInfo::Bel(Default::default()));
        tcls.bels
            .insert(bels::GSR, BelInfo::Bel(Default::default()));
        // OSC and RDBK exist as stubs?
        db.tile_classes.insert("CONFIG".to_string(), tcls);
    } else {
        for name in ["OSC", "OSC_X"] {
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::OSC, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }

        let mut tcls = TileClass::new(tslots::BEL, 5);
        tcls.bels
            .insert(bels::GSR, BelInfo::Bel(Default::default()));
        tcls.bels
            .insert(bels::JTAG, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("CONFIG".to_string(), tcls);
    }

    if family == "machxo" {
        for (name, num) in [
            ("SIO_W2", 4),
            ("SIO_W4", 4),
            ("SIO_XW2", 4),
            ("SIO_XW4", 4),
            ("SIO_E2", 4),
            ("SIO_E4", 4),
            ("SIO_S4", 4),
            ("SIO_S6", 6),
            ("SIO_N4", 4),
            ("SIO_N6", 6),
        ] {
            let mut tcls = TileClass::new(tslots::IO, 1);
            for i in 0..num {
                tcls.bels
                    .insert(bels::IO[i], BelInfo::Bel(Default::default()));
            }
            db.tile_classes.insert(name.to_string(), tcls);
        }
    } else {
        for name in ["IO_W", "IO_E", "IO_S", "IO_N"] {
            let mut tcls = TileClass::new(tslots::IO, 1);
            tcls.bels
                .insert(bels::IO0, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::IO1, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
        for name in ["DQS_W", "DQS_E", "DQS_S", "DQS_N"] {
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::DQS, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
        for name in ["DQSDLL_S", "DQSDLL_N"] {
            let mut tcls = TileClass::new(tslots::CLK, 1);
            tcls.bels
                .insert(bels::DQSDLL, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
    }

    if family == "machxo" {
        for name in ["PLL_S", "PLL_N"] {
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::PLL, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
    } else {
        for name in ["PLL_W", "PLL_E"] {
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::PLL, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
    }

    if family == "machxo" {
        for name in ["CLK_ROOT_0PLL", "CLK_ROOT_1PLL", "CLK_ROOT_2PLL"] {
            let mut tcls = TileClass::new(tslots::CLK, 6);
            tcls.bels
                .insert(bels::CLK_ROOT, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
    } else {
        let tile_classes = if family == "ecp" {
            [("CLK_ROOT_2PLL", 22), ("CLK_ROOT_4PLL", 32)].as_slice()
        } else {
            [
                ("CLK_ROOT_2PLL_A", 28),
                ("CLK_ROOT_2PLL_B", 28),
                ("CLK_ROOT_4PLL", 32),
            ]
            .as_slice()
        };
        for &(name, num_cells) in tile_classes {
            let mut tcls = TileClass::new(tslots::CLK, num_cells);
            tcls.bels
                .insert(bels::CLK_ROOT, BelInfo::Bel(Default::default()));
            for i in 0..8 {
                tcls.bels
                    .insert(bels::DCS[i], BelInfo::Bel(Default::default()));
            }
            db.tile_classes.insert(name.to_string(), tcls);
        }
    }

    db
}
