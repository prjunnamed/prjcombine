use std::collections::BTreeSet;

use prjcombine_ecp::{
    bels,
    chip::ChipKind,
    expanded::{REGION_HSDCLK, REGION_PCLK, REGION_SCLK, REGION_VSDCLK},
    tslots,
};
use prjcombine_interconnect::{
    db::{
        Bel, BelInfo, BelPin, Buf, CellSlotId, ConnectorClass, ConnectorSlot, ConnectorSlotId,
        ConnectorWire, IntDb, PinDir, SwitchBox, SwitchBoxItem, TileClass, TileWireCoord, WireKind,
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

pub fn init_intdb(kind: ChipKind) -> IntDb {
    let mut db = IntDb::default();

    assert_eq!(db.region_slots.insert("PCLK".into()).0, REGION_PCLK);
    assert_eq!(db.region_slots.insert("SCLK0".into()).0, REGION_SCLK[0]);
    assert_eq!(db.region_slots.insert("SCLK1".into()).0, REGION_SCLK[1]);
    assert_eq!(db.region_slots.insert("SCLK2".into()).0, REGION_SCLK[2]);
    assert_eq!(db.region_slots.insert("SCLK3".into()).0, REGION_SCLK[3]);
    assert_eq!(db.region_slots.insert("HSDCLK".into()).0, REGION_HSDCLK);
    assert_eq!(db.region_slots.insert("VSDCLK".into()).0, REGION_VSDCLK);
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

    let num_x0 = if kind.has_x0_branch() { 4 } else { 2 };
    for dir in Dir::DIRS {
        for i in 0..num_x0 {
            if kind.has_x0_branch() {
                let w0 = db
                    .wires
                    .insert(format!("X0_{dir}{i}_0"), WireKind::MuxOut)
                    .0;
                let w1 = db
                    .wires
                    .insert(format!("X0_{dir}{i}_1"), WireKind::Branch(conn_slots[!dir]))
                    .0;
                passes[!dir].wires.insert(w1, ConnectorWire::Pass(w0));
            } else {
                db.wires.insert(format!("X0_{dir}{i}"), WireKind::MuxOut);
            }
        }
    }

    if kind.has_x1_bi() {
        for i in [1, 4] {
            db.wires.insert(format!("X1_H{i}"), WireKind::MuxOut);
        }
        for i in [1, 4] {
            db.wires.insert(format!("X1_V{i}"), WireKind::MuxOut);
        }
    }

    let nums_x1 = if kind.has_x1_bi() {
        [0, 1, 4, 5].as_slice()
    } else {
        [0, 1].as_slice()
    };

    for dir in Dir::DIRS {
        for &i in nums_x1 {
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

    let num_clk = match kind {
        ChipKind::Ecp | ChipKind::Xp => 4,
        ChipKind::MachXo => 4,
        ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => 8,
    };

    for i in 0..num_clk {
        db.wires
            .insert(format!("PCLK{i}"), WireKind::Regional(REGION_PCLK));
    }
    for i in 0..num_clk {
        let region = if kind.has_distributed_sclk() {
            REGION_SCLK[i % 4]
        } else {
            REGION_PCLK
        };
        db.wires
            .insert(format!("SCLK{i}"), WireKind::Regional(region));
    }

    if kind.has_distributed_sclk() {
        for i in 0..2 {
            db.wires
                .insert(format!("HSDCLK{i}"), WireKind::Regional(REGION_HSDCLK));
        }
        for i in 0..2 {
            let w = db
                .wires
                .insert(format!("VSDCLK{i}"), WireKind::Regional(REGION_VSDCLK))
                .0;
            let w_n = db
                .wires
                .insert(format!("VSDCLK{i}_N"), WireKind::Branch(slot_s))
                .0;
            passes[Dir::S].wires.insert(w_n, ConnectorWire::Pass(w));
        }
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
            if kind.has_x0_branch() {
                if (l == "OFX" && i == 3) || (l == "F" && matches!(i, 0..3)) {
                    let w_w = db
                        .wires
                        .insert(format!("OUT_{l}{i}_W"), WireKind::Branch(slot_e))
                        .0;
                    passes[Dir::E].wires.insert(w_w, ConnectorWire::Pass(w));
                }
                if l == "F" && matches!(i, 4..8) {
                    let w_e = db
                        .wires
                        .insert(format!("OUT_{l}{i}_E"), WireKind::Branch(slot_w))
                        .0;
                    passes[Dir::W].wires.insert(w_e, ConnectorWire::Pass(w));
                }
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

    let int_tiles = match kind {
        ChipKind::MachXo => [
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
        .as_slice(),
        ChipKind::Ecp | ChipKind::Xp => {
            ["INT_PLC", "INT_IO_WE", "INT_IO_SN", "INT_EBR", "INT_PLL"].as_slice()
        }
        ChipKind::Ecp2 | ChipKind::Xp2 => [
            "INT_PLC",
            "INT_IO_WE",
            "INT_IO_S",
            "INT_IO_N",
            "INT_EBR",
            "INT_EBR_IO",
        ]
        .as_slice(),
        ChipKind::Ecp2M => [
            "INT_PLC",
            "INT_IO_WE",
            "INT_IO_S",
            "INT_IO_N",
            "INT_EBR",
            "INT_EBR_IO",
            "INT_SERDES_N",
        ]
        .as_slice(),
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
            if i < 3 || !kind.has_ecp2_plc() {
                add_input(&db, &mut bel, "CLK", 0, &format!("IMUX_CLK{i}"));
                add_input(&db, &mut bel, "LSR", 0, &format!("IMUX_LSR{i}"));
                add_input(&db, &mut bel, "CE", 0, &format!("IMUX_CE{i}"));
                add_output(&db, &mut bel, "Q0", 0, &format!("OUT_Q{i0}"));
                add_output(&db, &mut bel, "Q1", 0, &format!("OUT_Q{i1}"));
            }
            for l in ["F", "OFX"] {
                add_output(&db, &mut bel, &format!("{l}0"), 0, &format!("OUT_{l}{i0}"));
                add_output(&db, &mut bel, &format!("{l}1"), 0, &format!("OUT_{l}{i1}"));
            }
            if i == 3 && kind.has_x0_branch() {
                add_input(&db, &mut bel, "FXB", 0, "OUT_OFX3_W");
            }
            tcls.bels.insert(bels::SLICE[i], BelInfo::Bel(bel));
        }
        db.tile_classes.insert(name.to_string(), tcls);
    }

    if kind == ChipKind::MachXo {
        let mut tcls = TileClass::new(tslots::BEL, 1);
        tcls.bels
            .insert(bels::CIBTEST_SEL, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("CIBTEST_SEL".to_string(), tcls);
    }

    {
        let num_cells = match kind {
            ChipKind::Ecp | ChipKind::Xp => 2,
            ChipKind::MachXo => 4,
            ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => 3,
        };
        let mut tcls = TileClass::new(tslots::BEL, num_cells);
        tcls.bels
            .insert(bels::EBR0, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("EBR".to_string(), tcls);
    }

    if let Some(num_cells) = match kind {
        ChipKind::Ecp => Some(8),
        ChipKind::Xp | ChipKind::MachXo => None,
        ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => Some(9),
    } {
        let mut tcls = TileClass::new(tslots::BEL, num_cells);
        tcls.bels
            .insert(bels::DSP0, BelInfo::Bel(Default::default()));
        db.tile_classes.insert("DSP".to_string(), tcls);
    }

    match kind {
        ChipKind::Ecp => {
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
        }
        ChipKind::Xp => {
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::START, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::JTAG, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::GSR, BelInfo::Bel(Default::default()));
            // OSC and RDBK exist as stubs?
            db.tile_classes.insert("CONFIG".to_string(), tcls);
        }
        ChipKind::MachXo => {
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
        ChipKind::Ecp2 | ChipKind::Ecp2M => {
            let mut tcls = TileClass::new(tslots::BEL, 3);
            tcls.bels
                .insert(bels::START, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::OSC, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::JTAG, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::GSR, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::SED, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::SPIM, BelInfo::Bel(Default::default()));
            db.tile_classes.insert("CONFIG".to_string(), tcls);
        }
        ChipKind::Xp2 => {
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::JTAG, BelInfo::Bel(Default::default()));
            tcls.bels
                .insert(bels::SED, BelInfo::Bel(Default::default()));
            db.tile_classes.insert("CONFIG".to_string(), tcls);

            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::OSC, BelInfo::Bel(Default::default()));
            db.tile_classes.insert("OSC".to_string(), tcls);
        }
    }

    if kind == ChipKind::MachXo {
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
            if matches!(kind, ChipKind::Ecp2 | ChipKind::Ecp2M) && name == "DQS_N" {
                continue;
            }
            let mut tcls = TileClass::new(tslots::BEL, 1);
            tcls.bels
                .insert(bels::DQS, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
        if matches!(kind, ChipKind::Ecp | ChipKind::Xp) {
            for name in ["DQSDLL_S", "DQSDLL_N"] {
                let mut tcls = TileClass::new(tslots::BEL, 1);
                tcls.bels
                    .insert(bels::DQSDLL, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
        if kind == ChipKind::Xp2 {
            for name in ["DQSDLL_W", "DQSDLL_E"] {
                let mut tcls = TileClass::new(tslots::BEL, if name == "DQSDLL_E" { 2 } else { 1 });
                tcls.bels
                    .insert(bels::DQSDLL, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::CLKDIV, BelInfo::Bel(Default::default()));
                if name == "DQSDLL_E" {
                    tcls.bels
                        .insert(bels::SSPI, BelInfo::Bel(Default::default()));
                    tcls.bels
                        .insert(bels::STF, BelInfo::Bel(Default::default()));
                    tcls.bels
                        .insert(bels::WAKEUP, BelInfo::Bel(Default::default()));
                    tcls.bels
                        .insert(bels::START, BelInfo::Bel(Default::default()));
                    tcls.bels
                        .insert(bels::GSR, BelInfo::Bel(Default::default()));
                }
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
    }

    if kind == ChipKind::Ecp2M {
        for name in ["SERDES_S", "SERDES_N"] {
            let mut tcls = TileClass::new(tslots::BEL, 27);
            tcls.bels
                .insert(bels::SERDES, BelInfo::Bel(Default::default()));
            db.tile_classes.insert(name.to_string(), tcls);
        }
    }

    match kind {
        ChipKind::MachXo => {
            for name in ["PLL_S", "PLL_N"] {
                let mut tcls = TileClass::new(tslots::BEL, 1);
                tcls.bels
                    .insert(bels::PLL, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
        ChipKind::Ecp | ChipKind::Xp => {
            for name in ["PLL_W", "PLL_E"] {
                let mut tcls = TileClass::new(tslots::BEL, 1);
                tcls.bels
                    .insert(bels::PLL, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
        ChipKind::Ecp2 | ChipKind::Ecp2M => {
            for name in ["SPLL_W", "SPLL_E"] {
                let mut tcls = TileClass::new(tslots::BEL, 2);
                tcls.bels
                    .insert(bels::SPLL, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
            for name in ["PLL_W", "PLL_E"] {
                let mut tcls = TileClass::new(tslots::BEL, 4);
                tcls.bels
                    .insert(bels::PLL, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::DLL, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::DLLDEL, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::CLKDIV, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::ECLK_ALT_ROOT, BelInfo::Bel(Default::default()));
                tcls.bels
                    .insert(bels::DQSDLL, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
        ChipKind::Xp2 => {
            for name in ["PLL_S", "PLL_N"] {
                let mut tcls = TileClass::new(tslots::BEL, 2);
                tcls.bels
                    .insert(bels::PLL, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
    }

    if kind.has_distributed_sclk() {
        for i in 0..4 {
            let mut tcls = TileClass::new(tslots::SCLK_ROOT, 1);
            let mut bel = Bel::default();
            add_output(&db, &mut bel, "OUT0", 0, &format!("SCLK{i}"));
            add_output(&db, &mut bel, "OUT1", 0, &format!("SCLK{ii}", ii = i + 4));
            add_output(&db, &mut bel, "IN0", 0, "VSDCLK0");
            add_output(&db, &mut bel, "IN1", 0, "VSDCLK1");
            tcls.bels.insert(bels::SCLK_ROOT, BelInfo::Bel(bel));
            db.tile_classes.insert(format!("SCLK{i}_ROOT"), tcls);
        }

        for name in ["HSDCLK_SPLITTER", "HSDCLK_ROOT"] {
            let mut tcls = TileClass::new(tslots::HSDCLK_SPLITTER, 8);
            let mut sb = SwitchBox::default();
            for i in 0..2 {
                let wire = db.get_wire(&format!("HSDCLK{i}"));
                for j in 0..4 {
                    let cell_w = CellSlotId::from_idx(j);
                    let cell_e = CellSlotId::from_idx(4 + j);
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: TileWireCoord { cell: cell_w, wire },
                        src: TileWireCoord { cell: cell_e, wire }.pos(),
                    }));
                    sb.items.push(SwitchBoxItem::ProgBuf(Buf {
                        dst: TileWireCoord { cell: cell_e, wire },
                        src: TileWireCoord { cell: cell_w, wire }.pos(),
                    }));
                }
            }
            tcls.bels
                .insert(bels::HSDCLK_SPLITTER, BelInfo::SwitchBox(sb));
            if name == "HSDCLK_ROOT" {
                let mut bel = Bel::default();
                for i in 0..8 {
                    add_output(
                        &db,
                        &mut bel,
                        &format!("OUT_W{i}"),
                        i % 4,
                        &format!("HSDCLK{ii}", ii = i / 4),
                    );
                    add_output(
                        &db,
                        &mut bel,
                        &format!("OUT_E{i}"),
                        4 + i % 4,
                        &format!("HSDCLK{ii}", ii = i / 4),
                    );
                }
                tcls.bels.insert(bels::HSDCLK_ROOT, BelInfo::Bel(bel));
            }
            db.tile_classes.insert(name.into(), tcls);
        }
    }

    match kind {
        ChipKind::MachXo => {
            for name in ["CLK_ROOT_0PLL", "CLK_ROOT_1PLL", "CLK_ROOT_2PLL"] {
                let mut tcls = TileClass::new(tslots::CLK, 6);
                tcls.bels
                    .insert(bels::CLK_ROOT, BelInfo::Bel(Default::default()));
                db.tile_classes.insert(name.to_string(), tcls);
            }
        }
        ChipKind::Ecp | ChipKind::Xp | ChipKind::Ecp2 | ChipKind::Ecp2M | ChipKind::Xp2 => {
            let tile_classes = match kind {
                ChipKind::Ecp => [("CLK_ROOT_2PLL", 22), ("CLK_ROOT_4PLL", 32)].as_slice(),
                ChipKind::Xp => [
                    ("CLK_ROOT_2PLL_A", 28),
                    ("CLK_ROOT_2PLL_B", 28),
                    ("CLK_ROOT_4PLL", 32),
                ]
                .as_slice(),
                ChipKind::Ecp2 => [
                    ("CLK_ROOT_2PLL", 30),
                    ("CLK_ROOT_4PLL", 30),
                    ("CLK_ROOT_6PLL", 30),
                ]
                .as_slice(),
                ChipKind::Ecp2M => [("CLK_ROOT_8PLL", 30)].as_slice(),
                ChipKind::Xp2 => [("CLK_ROOT_2PLL", 30), ("CLK_ROOT_4PLL", 30)].as_slice(),
                _ => unreachable!(),
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
            if kind.has_distributed_sclk() {
                for (name, num_cells) in [
                    ("ECLK_ROOT_W", 1),
                    ("ECLK_ROOT_E", 1),
                    ("ECLK_ROOT_S", 2),
                    ("ECLK_ROOT_N", 2),
                ] {
                    let mut tcls = TileClass::new(tslots::CLK, num_cells);
                    tcls.bels
                        .insert(bels::ECLK_ROOT, BelInfo::Bel(Default::default()));
                    db.tile_classes.insert(name.to_string(), tcls);
                }
                {
                    let mut tcls = TileClass::new(tslots::ECLK_TAP, 1);
                    let mut bel = Bel::default();
                    add_output(&db, &mut bel, "ECLK0", 0, "OUT_F6");
                    add_output(&db, &mut bel, "ECLK1", 0, "OUT_F7");
                    tcls.bels.insert(bels::ECLK_TAP, BelInfo::Bel(bel));
                    db.tile_classes.insert("ECLK_TAP".to_string(), tcls);
                }
            }
        }
    }

    db
}
