use prjcombine_ecp::{
    bels,
    chip::{ChipKind, IoGroupKind, PllLoc, SpecialIoKey, SpecialLocKey},
};
use prjcombine_interconnect::{
    db::{Bel, BelPin, TileWireCoord},
    dir::{Dir, DirH, DirHV, DirV},
};

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_serdes_scm(&mut self) {
        for tcname in ["SERDES_W", "SERDES_E"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::SERDES);
                let bank = self.chip.columns[bcrd.col].bank_n.unwrap();
                let (edge, quad) = match bank {
                    9 => (DirH::W, 0),
                    11 => (DirH::W, 1),
                    13 => (DirH::W, 2),
                    15 => (DirH::W, 3),
                    10 => (DirH::E, 0),
                    12 => (DirH::E, 1),
                    14 => (DirH::E, 2),
                    16 => (DirH::E, 3),
                    _ => unreachable!(),
                };
                let name_pcs = match edge {
                    DirH::W => format!("PCS36{quad}00"),
                    DirH::E => format!("PCS3E{quad}00"),
                };
                let abcd = ['A', 'B', 'C', 'D'][quad];
                let lr = match edge {
                    DirH::W => 'L',
                    DirH::E => 'R',
                };
                let cell = match edge {
                    DirH::W => bcrd.delta(6, 1),
                    DirH::E => bcrd.delta(0, 1),
                };
                let cell_next = self
                    .chip
                    .columns
                    .ids()
                    .find(|&col| self.chip.columns[col].bank_n == Some(bank + 2))
                    .map(|col| match edge {
                        DirH::W => bcrd.with_col(col).delta(6, 1),
                        DirH::E => bcrd.with_col(col).delta(0, 1),
                    });
                let cell_apio = match edge {
                    DirH::W => [cell.delta(0, 11), cell.delta(-1, 11)],
                    DirH::E => [cell.delta(0, 11), cell.delta(1, 11)],
                };
                let (_, c_apio1) = self.rc(cell_apio[1]);
                self.name_bel(
                    bcrd,
                    [
                        name_pcs,
                        format!("{abcd}_REFCLKP_{lr}"),
                        format!("{abcd}_REFCLKN_{lr}"),
                        format!("{abcd}_HDINP0_{lr}"),
                        format!("{abcd}_HDINN0_{lr}"),
                        format!("{abcd}_HDINP1_{lr}"),
                        format!("{abcd}_HDINN1_{lr}"),
                        format!("{abcd}_HDINP2_{lr}"),
                        format!("{abcd}_HDINN2_{lr}"),
                        format!("{abcd}_HDINP3_{lr}"),
                        format!("{abcd}_HDINN3_{lr}"),
                        format!("{abcd}_HDOUTP0_{lr}"),
                        format!("{abcd}_HDOUTN0_{lr}"),
                        format!("{abcd}_HDOUTP1_{lr}"),
                        format!("{abcd}_HDOUTN1_{lr}"),
                        format!("{abcd}_HDOUTP2_{lr}"),
                        format!("{abcd}_HDOUTN2_{lr}"),
                        format!("{abcd}_HDOUTP3_{lr}"),
                        format!("{abcd}_HDOUTN3_{lr}"),
                        format!("PT{c_apio1}I"),
                        format!("PT{c_apio1}J"),
                    ],
                );
                let mut bel = Bel::default();

                let wires = self.sorted_wires[&(cell, "PCS")].clone();
                for (pin, wire) in wires {
                    if pin.starts_with("HDOUT")
                        || pin.starts_with("HDIN")
                        || pin.starts_with("REFCLK")
                        || pin.starts_with("RXREFCLK")
                        || pin.starts_with("COUT")
                        || pin.starts_with("CIN")
                        || pin.starts_with("BIST_COUTP")
                        || pin.starts_with("SCAN_COUTP")
                        || pin.starts_with("SERDES_COUT")
                        || pin.starts_with("TCK_FMAC")
                        || pin.starts_with("TESTCLK")
                        || pin.starts_with("FF_RXCLK_P")
                        || pin.starts_with("FF_SYSCLK_P")
                        || pin == "FFC_CK_CORE_TX"
                        || pin == "CS_QIF"
                    {
                        continue;
                    }
                    self.add_bel_wire(bcrd, &pin, wire);
                    bel.pins.insert(pin, self.xlat_int_wire(bcrd, wire));
                }

                for (cell_pio, abcd, pin) in [
                    (cell_apio[0], 'A', "HDOUTP3"),
                    (cell_apio[0], 'B', "HDOUTN3"),
                    (cell_apio[0], 'C', "HDINP3"),
                    (cell_apio[0], 'D', "HDINN3"),
                    (cell_apio[0], 'E', "HDOUTP2"),
                    (cell_apio[0], 'F', "HDOUTN2"),
                    (cell_apio[0], 'G', "HDINP2"),
                    (cell_apio[0], 'H', "HDINN2"),
                    (cell_apio[0], 'I', "REFCLKP"),
                    (cell_apio[0], 'J', "REFCLKN"),
                    (cell_apio[1], 'A', "HDOUTP1"),
                    (cell_apio[1], 'B', "HDOUTN1"),
                    (cell_apio[1], 'C', "HDINP1"),
                    (cell_apio[1], 'D', "HDINN1"),
                    (cell_apio[1], 'E', "HDOUTP0"),
                    (cell_apio[1], 'F', "HDOUTN0"),
                    (cell_apio[1], 'G', "HDINP0"),
                    (cell_apio[1], 'H', "HDINN0"),
                    (cell_apio[1], 'I', "RXREFCLKP"),
                    (cell_apio[1], 'J', "RXREFCLKN"),
                ] {
                    for pin_pio in ["INPUT", "OUTPUT", "CLOCK"] {
                        let wire_apio =
                            self.rc_io_wire(cell_pio, &format!("J{pin_pio}{abcd}_APIO"));
                        self.add_bel_wire(bcrd, format!("{pin}_APIO_{pin_pio}"), wire_apio);
                        if pin_pio == "INPUT"
                            && (pin.starts_with("HDIN") || pin.starts_with("REFCLK"))
                        {
                            let wire = self.rc_wire(cell, &format!("J{pin}_PCS"));
                            self.add_bel_wire(bcrd, pin, wire);
                            self.claim_pip(wire, wire_apio);
                        }
                        if pin_pio == "OUTPUT" && pin.starts_with("HDOUT") {
                            let wire = self.rc_wire(cell, &format!("J{pin}_PCS"));
                            self.add_bel_wire(bcrd, pin, wire);
                            self.claim_pip(wire_apio, wire);
                        }
                    }
                }

                {
                    let wire = self.rc_wire(cell, "JCS_QIF_PCS");
                    self.add_bel_wire(bcrd, "CS_QIF", wire);
                    let cell_sysbus = self.chip.special_loc[&SpecialLocKey::Config].delta(2, 0);
                    let wire_sysbus = self.rc_wire(cell_sysbus, &format!("J{lr}PCSQ{quad}_SYSBUS"));
                    self.claim_pip(wire, wire_sysbus);
                }

                for i in 0..22 {
                    let cout = self.rc_wire(cell, &format!("JCOUT_{i}_PCS"));
                    self.add_bel_wire(bcrd, format!("COUT_{i}"), cout);
                    let bist_coutp = self.rc_wire(cell, &format!("JBIST_COUTP_{i}_PCS"));
                    self.add_bel_wire(bcrd, format!("BIST_COUTP_{i}"), bist_coutp);
                    let scan_coutp = self.rc_wire(cell, &format!("JSCAN_COUTP_{i}_PCS"));
                    self.add_bel_wire(bcrd, format!("SCAN_COUTP_{i}"), scan_coutp);
                    self.claim_pip(cout, scan_coutp);
                    self.claim_pip(cout, bist_coutp);
                    if let Some(cell_next) = cell_next {
                        let cout_next = self.rc_wire(cell_next, &format!("JCOUT_{i}_PCS"));
                        self.claim_pip(scan_coutp, cout_next);
                        self.claim_pip(bist_coutp, cout_next);
                    }
                }
                for i in 0..22 {
                    let cout = self.rc_wire(cell, &format!("JSERDES_COUT_{i}_PCS"));
                    self.add_bel_wire(bcrd, format!("SERDES_COUT_{i}"), cout);
                    let coutp = self.rc_wire(cell, &format!("JSERDES_COUTP_{i}_PCS"));
                    self.add_bel_wire(bcrd, format!("SERDES_COUTP_{i}"), coutp);
                    self.claim_pip(cout, coutp);
                    if let Some(cell_next) = cell_next {
                        let cout_next = self.rc_wire(cell_next, &format!("JSERDES_COUT_{i}_PCS"));
                        self.claim_pip(coutp, cout_next);
                    }
                }
                {
                    let testclk = self.rc_wire(cell, "JTESTCLK_PCS");
                    self.add_bel_wire(bcrd, "TESTCLK", testclk);
                    let testclk_maco = self.rc_wire(cell, "JTESTCLK_MACO_PCS");
                    self.add_bel_wire(bcrd, "TESTCLK_MACO", testclk_maco);
                    self.claim_pip(testclk, testclk_maco);
                    if let Some(cell_next) = cell_next {
                        let testclk_maco_next = self.rc_wire(cell_next, "JTESTCLK_MACO_PCS");
                        self.claim_pip(testclk_maco_next, testclk);
                    }
                }
                {
                    let tck_fmac = self.rc_wire(cell, "JTCK_FMAC_PCS");
                    self.add_bel_wire(bcrd, "TCK_FMAC", tck_fmac);
                    let tck_fmacp = self.rc_wire(cell, "JTCK_FMACP_PCS");
                    self.add_bel_wire(bcrd, "TCK_FMACP", tck_fmacp);
                    self.claim_pip(tck_fmac, tck_fmacp);
                    if let Some(cell_next) = cell_next {
                        let tck_fmac_next = self.rc_wire(cell_next, "JTCK_FMAC_PCS");
                        self.claim_pip(tck_fmacp, tck_fmac_next);
                    }
                }
                for i in 0..13 {
                    let cin = self.rc_wire(cell, &format!("JCIN_{i}_PCS"));
                    self.add_bel_wire(bcrd, format!("CIN_{i}"), cin);
                }
                for pin in [
                    "FF_RXCLK_P1",
                    "FF_RXCLK_P2",
                    "FF_SYSCLK_P1",
                    "RXREFCLKP",
                    "RXREFCLKN",
                ] {
                    let wire = self.rc_wire(cell, &format!("J{pin}_PCS"));
                    self.add_bel_wire(bcrd, pin, wire);
                }

                self.insert_bel(bcrd, bel);

                {
                    let bcrd_center =
                        self.chip.special_loc[&SpecialLocKey::Config].bel(bels::SERDES_CENTER);
                    let mut bel_center = Bel::default();
                    let wire = self.rc_wire(cell, "JFFC_CK_CORE_TX_PCS");
                    self.add_bel_wire(bcrd, "FFC_CK_CORE_TX", wire);
                    bel_center.pins.insert(
                        "FFC_CK_CORE_TX".into(),
                        self.xlat_int_wire(bcrd_center, wire),
                    );
                    self.insert_bel(bcrd_center, bel_center);
                }
            }
        }
        for hv in [DirHV::NW, DirHV::NE] {
            let bcrd = self.chip.special_loc[&SpecialLocKey::Pll(PllLoc::new(hv, 0))]
                .bel(bels::SERDES_CORNER);
            let tcrd = self.edev.get_tile_by_bel(bcrd);
            let tcls = &self.intdb.tile_classes[self.edev[tcrd].class];
            let cell = bcrd.with_row(self.chip.row_n());
            let dir_s = !hv.h;
            let dir_n = hv.h;
            self.name_bel_null(bcrd);
            let mut bel = Bel::default();

            let mut cells_serdes = vec![];
            match hv.h {
                DirH::W => {
                    for (col, cd) in &self.chip.columns {
                        if cd.io_n == IoGroupKind::Serdes && col < self.chip.col_clk {
                            cells_serdes.push(bcrd.with_cr(col + 6, self.chip.row_n() - 11));
                        }
                    }
                }
                DirH::E => {
                    for (col, cd) in self.chip.columns.iter().rev() {
                        if cd.io_n == IoGroupKind::Serdes && col >= self.chip.col_clk {
                            cells_serdes.push(bcrd.with_cr(col, self.chip.row_n() - 11));
                        }
                    }
                }
            };

            for i in 0..13 {
                let twire =
                    TileWireCoord::new_idx(2, self.intdb.get_wire(&format!("IO_{dir_n}{i}_1")));
                bel.pins.insert(format!("CIN_{i}"), BelPin::new_in(twire));
                let wire_int = self.io_int_names[&self.edev.tile_wire(tcrd, twire)];
                let wire = self.rc_corner_wire(cell, &format!("JVMAN01{i:02}"));
                self.claim_pip_bi(wire, wire_int);
                self.add_bel_wire(bcrd, twire.to_string(self.intdb, tcls), wire);
                for &cell_serdes in &cells_serdes {
                    let wire_serdes = self.rc_wire(cell_serdes, &format!("JCIN_{i}_PCS"));
                    self.claim_pip(wire_serdes, wire);
                }
            }
            {
                let twire =
                    TileWireCoord::new_idx(2, self.intdb.get_wire(&format!("IO_{dir_n}14_1")));
                bel.pins
                    .insert("TESTCLK_MACO".into(), BelPin::new_in(twire));
                let wire_int = self.io_int_names[&self.edev.tile_wire(tcrd, twire)];
                let wire = self.rc_corner_wire(cell, "JVMAN0114");
                self.claim_pip_bi(wire, wire_int);
                self.add_bel_wire(bcrd, twire.to_string(self.intdb, tcls), wire);
                for &cell_serdes in &cells_serdes {
                    let wire_serdes = self.rc_wire(cell_serdes, "JTESTCLK_MACO_PCS");
                    self.claim_pip(wire_serdes, wire);
                }
            }
            let mut outps = vec![];
            for i in 0..6 {
                outps.push((
                    1,
                    i,
                    format!("COUT_{i}"),
                    Some(cells_serdes[0]),
                    format!("JCOUT_{i}_PCS"),
                ));
            }
            outps.push((
                1,
                7,
                "TCK_FMAC_PCS".to_string(),
                Some(cells_serdes[0]),
                "JTCK_FMAC_PCS".to_string(),
            ));
            for i in 6..22 {
                outps.push((
                    1,
                    i + 3,
                    format!("COUT_{i}"),
                    Some(cells_serdes[0]),
                    format!("JCOUT_{i}_PCS"),
                ));
            }
            for i in 0..4 {
                for j in 0..4 {
                    let idx = 25 + i * 4 + j;
                    let idx = if idx < 38 { idx } else { idx + 3 };
                    outps.push((
                        1 + idx / 32,
                        idx % 32,
                        format!("SERDES{i}_BS4PAD_{j}"),
                        cells_serdes.get(i).copied(),
                        format!("JBS4PAD_{j}_PCS"),
                    ));
                }
            }

            for (seg, idx, pin, cell_serdes, wn_serdes) in outps {
                let twire = TileWireCoord::new_idx(
                    2,
                    self.intdb
                        .get_wire(&format!("IO_{dir_s}{idx}_{seg}", seg = seg + 1)),
                );
                bel.pins.insert(pin, BelPin::new_out(twire));
                if let Some(cell_serdes) = cell_serdes {
                    let wire_int = self.io_int_names[&self.edev.tile_wire(tcrd, twire)];
                    let wire = self.rc_corner_wire(cell, &format!("JVMAS{seg:02}{idx:02}"));
                    self.claim_pip_bi(wire, wire_int);
                    self.add_bel_wire(bcrd, twire.to_string(self.intdb, tcls), wire);
                    let wire_serdes = self.rc_wire(cell_serdes, &wn_serdes);
                    self.claim_pip(wire, wire_serdes);
                }
            }

            self.insert_bel(bcrd, bel);
        }
    }

    fn process_serdes_ecp2(&mut self) {
        for tcname in ["SERDES_S", "SERDES_N"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::SERDES);
                let hv = DirHV {
                    h: if bcrd.col < self.chip.col_clk {
                        DirH::W
                    } else {
                        DirH::E
                    },
                    v: if bcrd.row < self.chip.row_clk {
                        DirV::S
                    } else {
                        DirV::N
                    },
                };
                let cell = bcrd.cell.delta(
                    match hv.h {
                        DirH::W => 12,
                        DirH::E => 13,
                    },
                    match hv.v {
                        DirV::S => -1,
                        DirV::N => 1,
                    },
                );
                let cell_apio = bcrd.cell.delta(
                    match hv.h {
                        DirH::W => 12,
                        DirH::E => 13,
                    },
                    match hv.v {
                        DirV::S => -7,
                        DirV::N => 7,
                    },
                );
                let corner = match hv {
                    DirHV::SW => "LL",
                    DirHV::SE => "LR",
                    DirHV::NW => "UL",
                    DirHV::NE => "UR",
                };
                self.name_bel(
                    bcrd,
                    [
                        format!("{corner}PCS"),
                        format!("{corner}C_SQ_REFCLKP"),
                        format!("{corner}C_SQ_REFCLKN"),
                        format!("{corner}C_SQ_HDINP0"),
                        format!("{corner}C_SQ_HDINN0"),
                        format!("{corner}C_SQ_HDINP1"),
                        format!("{corner}C_SQ_HDINN1"),
                        format!("{corner}C_SQ_HDINP2"),
                        format!("{corner}C_SQ_HDINN2"),
                        format!("{corner}C_SQ_HDINP3"),
                        format!("{corner}C_SQ_HDINN3"),
                        format!("{corner}C_SQ_HDOUTP0"),
                        format!("{corner}C_SQ_HDOUTN0"),
                        format!("{corner}C_SQ_HDOUTP1"),
                        format!("{corner}C_SQ_HDOUTN1"),
                        format!("{corner}C_SQ_HDOUTP2"),
                        format!("{corner}C_SQ_HDOUTN2"),
                        format!("{corner}C_SQ_HDOUTP3"),
                        format!("{corner}C_SQ_HDOUTN3"),
                    ],
                );
                for (dx, name) in [
                    (-11, "IN3"),
                    (-10, "OUT3"),
                    (-6, "IN2"),
                    (-5, "OUT2"),
                    (0, "REFCLK"),
                    (5, "IN1"),
                    (6, "OUT1"),
                    (10, "IN0"),
                    (11, "OUT0"),
                ] {
                    for pin in [
                        "JINPUTA_APIO",
                        "JOUTPUTA_APIO",
                        "JCLOCKA_APIO",
                        "JINPUTB_APIO",
                        "JOUTPUTB_APIO",
                        "JCLOCKB_APIO",
                    ] {
                        let wire = self.rc_io_wire(cell_apio.delta(dx, 0), pin);
                        self.add_bel_wire(bcrd, format!("{name}_{pin}"), wire);
                    }
                }
                for (dx, wp, wn) in [
                    (-11, "HDINP3", "HDINN3"),
                    (-6, "HDINP2", "HDINN2"),
                    (0, "REFCLKP", "REFCLKN"),
                    (5, "HDINP1", "HDINN1"),
                    (10, "HDINP0", "HDINN0"),
                ] {
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JINPUTA_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                    self.claim_pip(wire_pcs, wire_apio);
                    self.add_bel_wire(bcrd, wp, wire_pcs);
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JINPUTB_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                    self.claim_pip(wire_pcs, wire_apio);
                    self.add_bel_wire(bcrd, wn, wire_pcs);
                }
                for (dx, wp, wn) in [
                    (-10, "HDOUTP3", "HDOUTN3"),
                    (-5, "HDOUTP2", "HDOUTN2"),
                    (6, "HDOUTP1", "HDOUTN1"),
                    (11, "HDOUTP0", "HDOUTN0"),
                ] {
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JOUTPUTA_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                    self.claim_pip(wire_apio, wire_pcs);
                    self.add_bel_wire(bcrd, wp, wire_pcs);
                    let wire_apio = self.rc_io_wire(cell_apio.delta(dx, 0), "JOUTPUTB_APIO");
                    let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                    self.claim_pip(wire_apio, wire_pcs);
                    self.add_bel_wire(bcrd, wn, wire_pcs);
                }
                self.insert_simple_bel(bcrd, cell, "PCS");
            }
        }
    }

    fn process_serdes_ecp3(&mut self) {
        let tcid = self.intdb.get_tile_class("SERDES");
        for &tcrd in &self.edev.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::SERDES);
            let bank = self.chip.columns[bcrd.cell.col].bank_s.unwrap();
            let name = match bank {
                50 => "PCSA",
                51 => "PCSB",
                52 => "PCSC",
                53 => "PCSD",
                _ => unreachable!(),
            };
            let cell = bcrd.cell.delta(0, -1);
            self.name_bel(
                bcrd,
                [
                    name.to_string(),
                    format!("{name}_REFCLKP"),
                    format!("{name}_REFCLKN"),
                    format!("{name}_HDINP0"),
                    format!("{name}_HDINN0"),
                    format!("{name}_HDINP1"),
                    format!("{name}_HDINN1"),
                    format!("{name}_HDINP2"),
                    format!("{name}_HDINN2"),
                    format!("{name}_HDINP3"),
                    format!("{name}_HDINN3"),
                    format!("{name}_HDOUTP0"),
                    format!("{name}_HDOUTN0"),
                    format!("{name}_HDOUTP1"),
                    format!("{name}_HDOUTN1"),
                    format!("{name}_HDOUTP2"),
                    format!("{name}_HDOUTN2"),
                    format!("{name}_HDOUTP3"),
                    format!("{name}_HDOUTN3"),
                ],
            );
            for (dx, name) in [
                (0, "IN3"),
                (1, "OUT3"),
                (2, "OUT2"),
                (3, "IN2"),
                (4, "REFCLK"),
                (5, "IN1"),
                (6, "OUT1"),
                (7, "OUT0"),
                (8, "IN0"),
            ] {
                for pin in [
                    "JINPUTA_APIO",
                    "JOUTPUTA_APIO",
                    "JCLOCKA_APIO",
                    "JINPUTB_APIO",
                    "JOUTPUTB_APIO",
                    "JCLOCKB_APIO",
                ] {
                    let wire = self.rc_io_wire(cell.delta(dx, -8), pin);
                    self.add_bel_wire(bcrd, format!("{name}_{pin}"), wire);
                }
            }
            for (dx, wp, wn) in [
                (0, "HDINP3", "HDINN3"),
                (3, "HDINN2", "HDINP2"),
                (4, "REFCLKP", "REFCLKN"),
                (5, "HDINP1", "HDINN1"),
                (8, "HDINN0", "HDINP0"),
            ] {
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JINPUTA_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                self.claim_pip(wire_pcs, wire_apio);
                self.add_bel_wire(bcrd, wp, wire_pcs);
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JINPUTB_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                self.claim_pip(wire_pcs, wire_apio);
                self.add_bel_wire(bcrd, wn, wire_pcs);
            }
            for (dx, wp, wn) in [
                (1, "HDOUTP3", "HDOUTN3"),
                (2, "HDOUTN2", "HDOUTP2"),
                (6, "HDOUTP1", "HDOUTN1"),
                (7, "HDOUTN0", "HDOUTP0"),
            ] {
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JOUTPUTA_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wp}_PCS"));
                self.claim_pip(wire_apio, wire_pcs);
                self.add_bel_wire(bcrd, wp, wire_pcs);
                let wire_apio = self.rc_io_wire(cell.delta(dx, -8), "JOUTPUTB_APIO");
                let wire_pcs = self.rc_wire(cell, &format!("J{wn}_PCS"));
                self.claim_pip(wire_apio, wire_pcs);
                self.add_bel_wire(bcrd, wn, wire_pcs);
            }
            let wire = self.rc_wire(cell, "JREFCLK_TO_NQ_PCS");
            self.add_bel_wire(bcrd, "REFCLK_TO_NQ_PCS", wire);
            let wire = self.rc_wire(cell, "JREFCLK_FROM_NQ_PCS");
            self.add_bel_wire(bcrd, "REFCLK_FROM_NQ_PCS", wire);
            if let Some(cell_prev) = self.edev.cell_delta(bcrd.cell, -36, 0)
                && self.edev.has_bel(cell_prev.bel(bels::SERDES))
            {
                let wire_prev = self.rc_wire(cell.delta(-36, 0), "JREFCLK_TO_NQ_PCS");
                self.claim_pip(wire, wire_prev);
            }
            self.insert_simple_bel(bcrd, cell, "PCS");
        }
    }

    fn process_serdes_ecp4(&mut self) {
        let (cell, num_quads) =
            if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::SerdesSingle) {
                (cell, 1)
            } else if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::SerdesDouble) {
                (cell, 2)
            } else if let Some(&cell) = self.chip.special_loc.get(&SpecialLocKey::SerdesTriple) {
                (cell, 3)
            } else {
                unreachable!()
            };
        let bcrd = cell.bel(bels::SERDES);

        let mut names = vec![
            "ASB".to_string(),
            "REFCLKP_CORNER".to_string(),
            "REFCLKN_CORNER".to_string(),
        ];
        for quad in 0..num_quads {
            names.extend([format!("REFCLKP_Q{quad}"), format!("REFCLKN_Q{quad}")]);
            for ch in 0..4 {
                names.extend([
                    format!("HDRXP_Q{quad}CH{ch}"),
                    format!("HDRXN_Q{quad}CH{ch}"),
                    format!("HDTXP_Q{quad}CH{ch}"),
                    format!("HDTXN_Q{quad}CH{ch}"),
                ]);
            }
        }
        self.name_bel(bcrd, names);
        let cell = cell.with_col(self.chip.col_w());
        self.insert_simple_bel(bcrd, cell, "ASB");

        for quad in 0..num_quads {
            let cell_apio = cell.delta(1 + quad, 0);
            if quad == 0 {
                for pn in ['P', 'N'] {
                    let wire_apio = self.rc_wire(cell_apio, &format!("JINPUT_CORNREF{pn}_APIO"));
                    self.add_bel_wire(bcrd, format!("Q{quad}_INPUT_CORNREF{pn}_APIO"), wire_apio);
                    let wire = self.rc_io_sn_wire(cell, &format!("JCORNER_REFCLK{pn}_ASB"));
                    self.add_bel_wire(bcrd, format!("CORNER_REFCLK{pn}"), wire);
                    self.claim_pip(wire, wire_apio);
                    for pin in ["OUTPUT", "CLK"] {
                        let wire = self.rc_wire(cell_apio, &format!("{pin}_CORNREF{pn}_APIO"));
                        self.add_bel_wire(bcrd, format!("Q{quad}_{pin}_CORNREF{pn}_APIO"), wire);
                    }
                }
            }
            for pn in ['P', 'N'] {
                let wire_apio = self.rc_wire(cell_apio, &format!("JINPUT_REF{pn}_APIO"));
                self.add_bel_wire(bcrd, format!("Q{quad}_INPUT_REF{pn}_APIO"), wire_apio);
                let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}_REFCLK{pn}_ASB"));
                self.add_bel_wire(bcrd, format!("Q{quad}_REFCLK{pn}"), wire);
                self.claim_pip(wire, wire_apio);
                for pin in ["OUTPUT", "CLK"] {
                    let wire = self.rc_wire(cell_apio, &format!("{pin}_REF{pn}_APIO"));
                    self.add_bel_wire(bcrd, format!("Q{quad}_{pin}_REF{pn}_APIO"), wire);
                }
            }
            for ch in 0..4 {
                for pn in ['P', 'N'] {
                    let wire_apio = self.rc_wire(cell_apio, &format!("JOUTPUT_O{pn}{ch}_APIO"));
                    self.add_bel_wire(bcrd, format!("Q{quad}_OUTPUT_O{pn}{ch}_APIO"), wire_apio);
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_HDOUT{pn}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_HDOUT{pn}"), wire);
                    self.claim_pip(wire_apio, wire);
                    for pin in ["INPUT", "CLK"] {
                        let wire = self.rc_wire(cell_apio, &format!("{pin}_O{pn}{ch}_APIO"));
                        self.add_bel_wire(bcrd, format!("Q{quad}_{pin}_O{pn}{ch}_APIO"), wire);
                    }
                }
                for pn in ['P', 'N'] {
                    let wire_apio = self.rc_wire(cell_apio, &format!("JINPUT_I{pn}{ch}_APIO"));
                    self.add_bel_wire(bcrd, format!("Q{quad}_INPUT_I{pn}{ch}_APIO"), wire_apio);
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_HDIN{pn}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_HDIN{pn}"), wire);
                    self.claim_pip(wire, wire_apio);
                    for pin in ["OUTPUT", "CLK"] {
                        let wire = self.rc_wire(cell_apio, &format!("{pin}_I{pn}{ch}_APIO"));
                        self.add_bel_wire(bcrd, format!("Q{quad}_{pin}_I{pn}{ch}_APIO"), wire);
                    }
                }
            }
        }

        for quad in num_quads..4 {
            for i in 0..6 {
                for j in 0..8 {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}_FCDFECOEFF{i}_{j}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}_FCDFECOEFF{i}_{j}"), wire);
                }
            }
            for pin in [
                "FCDFESIGN1",
                "FCDFESIGN2",
                "FCDFESIGN3",
                "FCDFESIGN4",
                "FCDFESIGN5",
                "FCMPWRUP",
                "FCMRST",
                "FCSCANMODE",
                "FIRXTESTCLK",
                "FISYNCCLK",
                "FITMRCLK",
                "FITXTESTCLK",
                "FOREFCLK2FPGA",
                "HSPLLLOL",
                "HSPLLPWRUP",
                "HSPLLREFCLKI",
                "HSPLLRST",
                "LSPLLLOL",
                "LSPLLPWRUP",
                "LSPLLREFCLKI",
                "LSPLLRST",
            ] {
                let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}_{pin}_ASB"));
                self.add_bel_wire(bcrd, format!("Q{quad}_{pin}"), wire);
            }
            for (pin, width) in [
                ("FDDFECHSEL", 2),
                ("FDDFEDATA", 10),
                ("FDDFEERR", 10),
                ("FIGRPFBRRCLK", 2),
                ("FIGRPFBTWCLK", 2),
            ] {
                for j in 0..width {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}_{pin}{j}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}_{pin}{j}"), wire);
                }
            }
            for ch in 0..4 {
                for pin in [
                    "FCALIGNEN",
                    "FCCDRFORCEDLOCK",
                    "FCDFERDEN",
                    "FCDFEUPD",
                    "FCLDRTXEN",
                    "FCLSMEN",
                    "FCPCIEDETEN",
                    "FCPCSRXRST",
                    "FCPCSTXRST",
                    "FCPIPEPHYRESETN",
                    "FCPLLLOL",
                    "FCRATE0",
                    "FCRATE1",
                    "FCRATE2",
                    "FCRRST",
                    "FCRXPOLARITY",
                    "FCRXPWRUP",
                    "FCTMRSTART",
                    "FCTMRSTOP",
                    "FCTRST",
                    "FCTXMARGIN0",
                    "FCTXMARGIN1",
                    "FCTXMARGIN2",
                    "FCTXPWRUP",
                    "FCWORDALGNEN",
                    "FDLDRRX",
                    "FDLDRTX",
                    "FIRCLK",
                    "FIREFRXCLK",
                    "FITCLK",
                    "FITMRSTARTCLK",
                    "FITMRSTOPCLK",
                    "FSCCOVERRUN",
                    "FSCCUNDERRUN",
                    "FSDFEVLD",
                    "FSLSM",
                    "FSPCIECON",
                    "FSPCIEDONE",
                    "FSRCDONE",
                    "FSRLOL",
                    "FSRLOS",
                    "FSSKPADDED",
                    "FSSKPDELETED",
                ] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_{pin}"), wire);
                }
                for i in 0..48 {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_FDRX{i}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_FDRX{i}"), wire);
                }
                for i in 0..50 {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_FDTX{i}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_FDTX{i}"), wire);
                }
            }
            for i in 0..2 {
                for pin in ["FCDERST", "FSDE", "FSDM"] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}D{i}_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}D{i}_{pin}"), wire);
                }
            }
            for ch in 0..4 {
                for pin in [
                    "CIRXFULL",
                    "CIRXIGNOREPKT",
                    "CITXDATAAVAIL",
                    "CITXEMPTY",
                    "CITXEOF",
                    "CITXFIFOCTRL",
                    "CITXFORCEERR",
                    "CITXLASTBYTEVLD",
                    "CITXPAUSREQ",
                    "CORXEOF",
                    "CORXERROR",
                    "CORXFIFOFULLERROR",
                    "CORXLASTBYTEVLD",
                    "CORXSTATEN",
                    "CORXWRITE",
                    "COTXDISCFRM",
                    "COTXDONE",
                    "COTXREAD",
                    "COTXSTATEN",
                    "GIIPGSHRINK",
                    "GINONPADRXDV",
                    "GISYNCCOL",
                    "GISYNCCRS",
                    "GISYNCNIBDRIB",
                    "GISYNCRXDV",
                    "GISYNCRXER",
                    "GODISCARDFCS",
                    "GOTXMACERR",
                    "GOTXMACWR",
                    "KIRSTN",
                    "KIRXMACCLK",
                    "KIRXMACCLKENEXT",
                    "KIRXTXFECLK",
                    "KITXGMIILPBK",
                    "KITXMACCLK",
                    "KITXMACCLKENEXT",
                    "KOGBITEN",
                    "KORXMACCLKEN",
                ] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}EA{ch}_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}EA{ch}_{pin}"), wire);
                }
                for (pin, width) in [
                    ("CITXDATA", 16),
                    ("CITXPAUSTIM", 16),
                    ("CORXDATA", 16),
                    ("CORXSTATVEC", 8),
                    ("COTXSTATVEC", 8),
                    ("GISYNCRXD", 8),
                    ("GOTXMACDATA", 8),
                ] {
                    for j in 0..width {
                        let wire =
                            self.rc_io_sn_wire(cell, &format!("JQ{quad}EA{ch}_{pin}{j}_ASB"));
                        self.add_bel_wire(bcrd, format!("Q{quad}EA{ch}_{pin}{j}"), wire);
                    }
                }
            }
            if quad == 1 {
                for pin in [
                    "BUFFDEQACKADVPTRN",
                    "ENABLETXFLOWCONTROLN",
                    "GEAR",
                    "LNKCLK",
                    "LNKCLKDIV2",
                    "LNKCLKDIV2RSTN",
                    "LNKCLKRSTN",
                    "LNKDIV2RSTN",
                    "LNKMCERXACKN",
                    "LNKMCERXREQN",
                    "LNKMCETXACKN",
                    "LNKMCETXREQN",
                    "LNKRSTN",
                    "LNKTOUTPUTPORTENABLE",
                    "MASTERENABLE",
                    "MGTCLK",
                    "MGTCLKRSTN",
                    "MGTRDN",
                    "MGTRSTN",
                    "OLLMMGTINTN",
                    "OLLMMGTRDYN",
                    "OUTPUTUNRECOVERREVENTACKN",
                    "OUTPUTUNRECOVERREVENTREQN",
                    "PHYCLK",
                    "PHYEMEVENTACKN",
                    "PHYEMEVENTREQN",
                    "PHYMSTB",
                    "PHYRINITN",
                    "PHYRSTN",
                    "PHYTINITN",
                    "PHYUSTB",
                    "PORTDISABLE",
                    "RECOVERRSTN",
                    "RIOCLK",
                    "RIORSTN",
                    "RLNKDSTDSCN",
                    "RLNKDSTRDYN",
                    "RLNKEOFN",
                    "RLNKSOFN",
                    "RLNKSRCRDYN",
                    "RXINITN",
                    "RXPFORCERETRYN",
                    "SOFTRSTN",
                    "SYSRSTIN",
                    "SYSRSTON",
                    "TIMBISTMODE",
                    "TIMBISTRE",
                    "TIMBISTWE",
                    "TISCANCLK",
                    "TISCANENA",
                    "TISCANMODE",
                    "TISCANRSTN",
                    "TLNKDSTRDYN",
                    "TLNKEOFN",
                    "TLNKREM0",
                    "TLNKREM1",
                    "TLNKREM2",
                    "TLNKSOFN",
                    "TLNKSRCDSCN",
                    "TLNKSRCRDYN",
                    "TPORTENABLEN",
                    "TXINITN",
                ] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}S_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}S_{pin}"), wire);
                }
                for (pin, width) in [
                    ("BAUDSUPPORT", 5),
                    ("DECRBUFCNTVECTOR", 10),
                    ("FLOWFIFOACKON", 4),
                    ("LOOPBACK", 3),
                    ("MGTA", 22),
                    ("MGTDI", 32),
                    ("MGTWRN", 4),
                    ("OLLMEFPTR", 16),
                    ("OLLMMGTDI", 32),
                    ("PORTNERRORDETECT", 32),
                    ("RCVCLKSEL", 4),
                    ("RESPTIMEOUT", 24),
                    ("RLNKBEATS", 8),
                    ("RLNKD", 64),
                    ("RLNKREM", 3),
                    ("RXSYNCCTRL", 2),
                    ("STATUS", 22),
                    ("TIMBISTBANKSEL", 4),
                    ("TIMBISTRA", 8),
                    ("TIMBISTSDI", 36),
                    ("TIMBISTWA", 8),
                    ("TISCANI", 30),
                    ("TISCANO", 30),
                    ("TLNKD", 64),
                    ("TOMBISTDO", 36),
                    ("TPORTCHARISK", 8),
                    ("TPORTTDI", 64),
                    ("TXENQUEUEFLOWN", 4),
                    ("TXFLOWCTRLSTATE", 5),
                    ("TXLANESILENCECH", 4),
                    ("TXSYNCCTRL", 2),
                    ("WM0", 4),
                    ("WM1", 4),
                    ("WM2", 4),
                ] {
                    for j in 0..width {
                        let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}S_{pin}{j}_ASB"));
                        self.add_bel_wire(bcrd, format!("Q{quad}S_{pin}{j}"), wire);
                    }
                }
            }
            if quad >= 2 {
                for ch in 0..4 {
                    for pin in [
                        "CIRXFULL",
                        "CIRXIGNOREPKT",
                        "CITXDATAAVAIL",
                        "CITXEMPTY",
                        "CITXEOF",
                        "CITXFIFOCTRL",
                        "CITXFORCEERR",
                        "CITXPAUSEREQ",
                        "GIIPGSHRINK",
                        "GINONPADRXDV",
                        "GISYNCCOL",
                        "GISYNCCRS",
                        "GISYNCNIBDRIB",
                        "GISYNCRXDV",
                        "GISYNCRXER",
                        "KIRSTN",
                        "KIRXMACCLK",
                        "KIRXMACCLKENEXT",
                        "KIRXTXFECLK",
                        "KITXGMIILPBK",
                        "KITXMACCLK",
                        "KITXMACCLKENEXT",
                    ] {
                        let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}EB{ch}_{pin}_ASB"));
                        self.add_bel_wire(bcrd, format!("Q{quad}EB{ch}_{pin}"), wire);
                    }
                    for (pin, width) in [("CITXDATA", 8), ("CITXPAUSTIM", 16), ("GISYNCRXD", 8)] {
                        for j in 0..width {
                            let wire =
                                self.rc_io_sn_wire(cell, &format!("JQ{quad}EB{ch}_{pin}{j}_ASB"));
                            self.add_bel_wire(bcrd, format!("Q{quad}EB{ch}_{pin}{j}"), wire);
                        }
                    }
                }
                for pin in [
                    "IIGNOREPKT",
                    "IRESETN",
                    "IRXMACCLK",
                    "ITSMSGAVAIL",
                    "ITXDATAAVAIL",
                    "ITXEMPTY",
                    "ITXEOF",
                    "ITXFORCEERR",
                    "ITXMACCLK",
                    "ITXPAUSREQ",
                    "TIBISTREN",
                    "TIBISTTESTMODE",
                    "TIBISTWEN",
                    "TISCANEN",
                    "TISCANMODE",
                    "TISCANRSTN",
                ] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}X_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}X_{pin}"), wire);
                }
                for (pin, width) in [
                    ("CSO", 141),
                    ("GSO", 44),
                    ("ITXBYTEN", 3),
                    ("ITXDATA", 64),
                    ("ITXMSG", 64),
                    ("ITXPAUSTIM", 16),
                    ("KSO", 20),
                    ("TIBISTBANKSEL", 4),
                    ("TIBISTRA", 8),
                    ("TIBISTTDI", 36),
                    ("TIBISTWA", 8),
                    ("TISCANIN", 30),
                    ("TOSCANOUT", 29),
                ] {
                    for j in 0..width {
                        let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}X_{pin}{j}_ASB"));
                        self.add_bel_wire(bcrd, format!("Q{quad}X_{pin}{j}"), wire);
                    }
                }
            }

            for pn in ['P', 'N'] {
                let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}_REFCLK{pn}_ASB"));
                self.add_bel_wire(bcrd, format!("Q{quad}_REFCLK{pn}"), wire);
            }
            for ch in 0..4 {
                for pn in ['P', 'N'] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_HDOUT{pn}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_HDOUT{pn}"), wire);
                }
                for pn in ['P', 'N'] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_HDIN{pn}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_HDIN{pn}"), wire);
                }
            }
        }
        for quad in 0..4 {
            for ch in 0..4 {
                for pin in ["FOPCLKA", "FOPCLKB"] {
                    let wire = self.rc_io_sn_wire(cell, &format!("JQ{quad}CH{ch}_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_{pin}"), wire);
                }
                for pin in ["TXBITCLK", "FORECCLK"] {
                    let wire = self.rc_io_sn_wire(cell, &format!("Q{quad}CH{ch}_{pin}_ASB"));
                    self.add_bel_wire(bcrd, format!("Q{quad}CH{ch}_{pin}"), wire);
                }
            }
            for pin in ["REFCLKO", "HSPLLCLKO", "LSPLLCLKO", "FCDFESIGN0"] {
                let wire = self.rc_io_sn_wire(cell, &format!("Q{quad}_{pin}_ASB"));
                self.add_bel_wire(bcrd, format!("Q{quad}_{pin}"), wire);
            }
        }
        for pin in [
            "Q0P_CORECLK",
            "Q0P_SCANO25",
            "Q0P_SCANO26",
            "Q2X_TOSCANOUT29",
            "Q3X_TOSCANOUT29",
            "SCIINT",
            "SCIRTYO",
            "SCIACKO",
        ] {
            let wire = self.rc_io_sn_wire(cell, &format!("J{pin}_ASB"));
            self.add_bel_wire(bcrd, pin, wire);
        }
        for pin in ["CORNER_REFCLKO", "CORNER_LSPLLCLKO"] {
            let wire = self.rc_io_sn_wire(cell, &format!("{pin}_ASB"));
            self.add_bel_wire(bcrd, pin, wire);
        }
        for i in 0..8 {
            let wire = self.rc_io_sn_wire(cell, &format!("JSCIDATO{i}_ASB"));
            self.add_bel_wire(bcrd, format!("SCIDATO{i}"), wire);
        }

        let cell_efb = self.chip.special_loc[&SpecialLocKey::Config];
        for (pin, pin_efb, range) in [
            ("SCISTBI", "SCISTBO", 0..64),
            ("SCIDATI", "SCIDATO", 0..8),
            ("SCIADRI", "SCIADRO", 0..12),
        ] {
            for i in range {
                let wire = self.rc_io_sn_wire(cell, &format!("J{pin}{i}_ASB"));
                self.add_bel_wire(bcrd, format!("{pin}{i}"), wire);
                let wire_efb = self.rc_io_wire(cell_efb, &format!("J{pin_efb}{i}_EFB"));
                self.claim_pip(wire, wire_efb);
            }
        }
        for (pin, pin_efb) in [
            ("SCIWEI", "SCIWEO"),
            ("SCICYCI", "SCICYCO"),
            ("SCICLKI", "SCICLKO"),
            ("SCISLEEP", "SCISLEEP"),
            ("SCIINITEN0", "SCIINITEN0"),
            ("SCIINITEN1", "SCIINITEN1"),
            ("SCIRSTN", "SCIRSTN"),
            ("SCIRSTI", "SCIRSTO"),
        ] {
            let wire = self.rc_io_sn_wire(cell, &format!("J{pin}_ASB"));
            self.add_bel_wire(bcrd, pin, wire);
            let wire_efb = self.rc_io_wire(cell_efb, &format!("J{pin_efb}_EFB"));
            self.claim_pip(wire, wire_efb);
        }
    }

    fn process_serdes_ecp5(&mut self) {
        if self.skip_serdes {
            return;
        }
        let tcid = self.intdb.get_tile_class("SERDES");
        for &tcrd in &self.edev.tile_index[tcid] {
            let bcrd = tcrd.bel(bels::SERDES);
            let cell = bcrd.cell;
            let bank = self.chip.columns[bcrd.cell.col].bank_s.unwrap();
            let dual = match bank {
                50 => 0,
                51 => 1,
                _ => unreachable!(),
            };
            let cell_prev = if dual != 0 {
                let mut cell_prev = cell.delta(-1, 0);
                while self.chip.columns[cell_prev.col].io_s != IoGroupKind::Serdes {
                    cell_prev.col -= 1;
                }
                Some(cell_prev)
            } else {
                None
            };
            self.name_bel(
                bcrd,
                [
                    format!("DCU{dual}"),
                    format!("EXTREF{dual}"),
                    format!("REFCLKP_D{dual}"),
                    format!("REFCLKN_D{dual}"),
                    format!("HDRXP0_D{dual}CH0"),
                    format!("HDRXN0_D{dual}CH0"),
                    format!("HDRXP0_D{dual}CH1"),
                    format!("HDRXN0_D{dual}CH1"),
                    format!("HDTXP0_D{dual}CH0"),
                    format!("HDTXN0_D{dual}CH0"),
                    format!("HDTXP0_D{dual}CH1"),
                    format!("HDTXN0_D{dual}CH1"),
                ],
            );
            let mut bel = self.extract_simple_bel(bcrd, cell, "DCU");

            for (name, pin) in [
                ("IP0", "CH0_HDINP"),
                ("IN0", "CH0_HDINN"),
                ("IP1", "CH1_HDINP"),
                ("IN1", "CH1_HDINN"),
            ] {
                let wire_apio = self.rc_io_wire(cell, &format!("JINPUT_{name}_APIO"));
                self.add_bel_wire(bcrd, format!("INPUT_{name}_APIO"), wire_apio);
                let wire = self.rc_io_wire(cell, &format!("J{pin}_DCU"));
                self.add_bel_wire(bcrd, pin, wire);
                self.claim_pip(wire, wire_apio);
                for apin in ["CLK", "OUTPUT"] {
                    let wire_apio = self.rc_io_wire(cell, &format!("{apin}_{name}_APIO"));
                    self.add_bel_wire(bcrd, format!("{apin}_{name}_APIO"), wire_apio);
                }
            }
            for (name, pin) in [
                ("OP0", "CH0_HDOUTP"),
                ("ON0", "CH0_HDOUTN"),
                ("OP1", "CH1_HDOUTP"),
                ("ON1", "CH1_HDOUTN"),
            ] {
                let wire_apio = self.rc_io_wire(cell, &format!("JOUTPUT_{name}_APIO"));
                self.add_bel_wire(bcrd, format!("OUTPUT_{name}_APIO"), wire_apio);
                let wire = self.rc_io_wire(cell, &format!("J{pin}_DCU"));
                self.add_bel_wire(bcrd, pin, wire);
                self.claim_pip(wire_apio, wire);
                for apin in ["CLK", "INPUT"] {
                    let wire_apio = self.rc_io_wire(cell, &format!("{apin}_{name}_APIO"));
                    self.add_bel_wire(bcrd, format!("{apin}_{name}_APIO"), wire_apio);
                }
            }

            for (name, pin) in [("REFP", "REFCLKP"), ("REFN", "REFCLKN")] {
                let wire_apio = self.rc_io_wire(cell, &format!("INPUT_{name}_APIO"));
                self.add_bel_wire(bcrd, format!("INPUT_{name}_APIO"), wire_apio);
                let wire = self.rc_io_wire(cell, &format!("{pin}_EXTREF"));
                self.add_bel_wire(bcrd, pin, wire);
                self.claim_pip(wire, wire_apio);
                for apin in ["CLK", "OUTPUT"] {
                    let wire_apio = self.rc_io_wire(cell, &format!("{apin}_{name}_APIO"));
                    self.add_bel_wire(bcrd, format!("{apin}_{name}_APIO"), wire_apio);
                }
            }

            let refclko = self.rc_io_wire(cell, "JREFCLKO_EXTREF");
            self.add_bel_wire(bcrd, "REFCLKO", refclko);

            let refclko_out = self.rc_io_wire(cell, "EXTREFCLK");
            self.add_bel_wire(bcrd, "REFCLKO_OUT", refclko_out);
            self.claim_pip(refclko_out, refclko);

            let keepwire = self.rc_io_wire(cell, "KEEPWIRE");
            self.add_bel_wire(bcrd, "KEEPWIRE", keepwire);

            let refclk_prev = if let Some(cell_prev) = cell_prev {
                let wire_prev = self.rc_io_wire(cell_prev, "JTXREFCLK");
                let refclk_prev = self.rc_io_wire(cell, "JREFCLKFROMND");
                self.add_bel_wire(bcrd, "REFCLK_PREV", refclk_prev);
                self.claim_pip(refclk_prev, wire_prev);
                Some(refclk_prev)
            } else {
                None
            };

            for ch in 0..2 {
                let rx_refclk = self.rc_io_wire(cell, &format!("CH{ch}_RX_REFCLK_DCU"));
                self.add_bel_wire(bcrd, format!("CH{ch}_RX_REFCLK"), rx_refclk);

                let rx_refclk_in = self.rc_io_wire(cell, &format!("CH{ch}_RX_REFCLK"));
                self.add_bel_wire(bcrd, format!("CH{ch}_RX_REFCLK_IN"), rx_refclk_in);
                self.claim_pip(rx_refclk, rx_refclk_in);

                let rx_refclk_int = self.rc_io_wire(cell, &format!("JCH{ch}RXREFCLKCIB"));
                self.add_bel_wire(bcrd, format!("CH{ch}_RX_REFCLK_INT"), rx_refclk_int);
                self.claim_pip(rx_refclk_in, rx_refclk_int);
                bel.pins.insert(
                    format!("CH{ch}_RX_REFCLK"),
                    self.xlat_int_wire(bcrd, rx_refclk_int),
                );

                let rx_refclk_mux = self.rc_io_wire(cell, &format!("RXREFCLK{ch}"));
                self.add_bel_wire(bcrd, format!("CH{ch}_RX_REFCLK_MUX"), rx_refclk_mux);
                self.claim_pip(rx_refclk_in, rx_refclk_mux);

                self.claim_pip(rx_refclk_mux, keepwire);
                self.claim_pip(rx_refclk_mux, refclko_out);
                if let Some(refclk_prev) = refclk_prev {
                    self.claim_pip(rx_refclk_mux, refclk_prev);
                }
            }

            {
                let d_refclki = self.rc_io_wire(cell, "D_REFCLKI_DCU");
                self.add_bel_wire(bcrd, "D_REFCLKI", d_refclki);

                let d_refclki_in = self.rc_io_wire(cell, "D_REFCLKI");
                self.add_bel_wire(bcrd, "D_REFCLKI_IN", d_refclki_in);
                self.claim_pip(d_refclki, d_refclki_in);

                let d_refclki_int = self.rc_io_wire(cell, "JTXREFCLKCIB");
                self.add_bel_wire(bcrd, "D_REFCLKI_INT", d_refclki_int);
                self.claim_pip(d_refclki_in, d_refclki_int);
                bel.pins
                    .insert("D_REFCLKI".into(), self.xlat_int_wire(bcrd, d_refclki_int));

                let d_refclki_mux = self.rc_io_wire(cell, "JTXREFCLK");
                self.add_bel_wire(bcrd, "D_REFCLKI_MUX", d_refclki_mux);
                self.claim_pip(d_refclki_in, d_refclki_mux);

                self.claim_pip(d_refclki_mux, keepwire);
                self.claim_pip(d_refclki_mux, refclko_out);
                if let Some(refclk_prev) = refclk_prev {
                    self.claim_pip(d_refclki_mux, refclk_prev);
                }
            }

            for (pin_to, pin_from) in [
                ("D_SYNC_ND", "D_SYNC_PULSE2ND"),
                ("D_TXPLL_LOL_FROM_ND", "D_TXPLL_LOL_TO_ND"),
                ("D_TXBIT_CLKP_FROM_ND", "D_TXBIT_CLKP_TO_ND"),
                ("D_TXBIT_CLKN_FROM_ND", "D_TXBIT_CLKN_TO_ND"),
            ] {
                let wire_from = self.rc_io_wire(cell, &format!("J{pin_from}_DCU"));
                self.add_bel_wire(bcrd, pin_from, wire_from);
                let wire_to = self.rc_io_wire(cell, &format!("J{pin_to}_DCU"));
                self.add_bel_wire(bcrd, pin_to, wire_to);
                if let Some(cell_prev) = cell_prev {
                    let wire_prev = self.rc_io_wire(cell_prev, &format!("J{pin_from}_DCU"));
                    self.claim_pip(wire_to, wire_prev);
                }
            }

            for ch in 0..2 {
                for pin in ["FF_TX_PCLK", "FF_RX_PCLK"] {
                    let wire = self.rc_io_wire(cell, &format!("JCH{ch}_{pin}_DCU"));
                    self.add_bel_wire(bcrd, format!("CH{ch}_{pin}"), wire);
                }
            }

            self.insert_bel(bcrd, bel);
        }
    }

    fn process_mipi_crosslink(&mut self) {
        for tcname in ["MIPI_W", "MIPI_E"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.tile_index[tcid] {
                let bcrd = tcrd.bel(bels::MIPI);
                let cell = bcrd.cell;
                let bank = self.chip.columns[bcrd.cell.col].bank_n.unwrap();
                let idx = match bank {
                    60 => 0,
                    61 => 1,
                    _ => unreachable!(),
                };

                self.name_bel(
                    bcrd,
                    [
                        format!("MIPIDPHY{idx}"),
                        format!("DPHY{idx}_CKP"),
                        format!("DPHY{idx}_CKN"),
                        format!("DPHY{idx}_DP0"),
                        format!("DPHY{idx}_DN0"),
                        format!("DPHY{idx}_DP1"),
                        format!("DPHY{idx}_DN1"),
                        format!("DPHY{idx}_DP2"),
                        format!("DPHY{idx}_DN2"),
                        format!("DPHY{idx}_DP3"),
                        format!("DPHY{idx}_DN3"),
                    ],
                );
                let mut bel = self.extract_simple_bel(bcrd, cell, "MIPIDPHY");

                for pin in [
                    "DP0", "DN0", "DP1", "DN1", "DP2", "DN2", "DP3", "DN3", "CKP", "CKN",
                ] {
                    let wire_abpio = self.rc_io_wire(cell, &format!("PAD{pin}_ABPIO"));
                    self.add_bel_wire(bcrd, format!("PAD{pin}"), wire_abpio);
                    let wire = self.rc_io_wire(cell, &format!("{pin}_MIPIDPHY"));
                    self.add_bel_wire(bcrd, format!("PAD{pin}"), wire);
                    self.claim_pip(wire_abpio, wire);
                }

                for pin in ["HSBYTECLKD", "HSBYTECLKS"] {
                    let wire = self.rc_io_wire(cell, &format!("J{pin}_MIPIDPHY"));
                    self.add_bel_wire(bcrd, format!("PAD{pin}"), wire);
                }

                let clkref = self.rc_io_wire(cell, "CLKREF_MIPIDPHY");
                self.add_bel_wire(bcrd, "CLKREF", clkref);
                let clkref_in = self.rc_io_wire(cell, "CLKREF");
                self.add_bel_wire(bcrd, "CLKREF_IN", clkref_in);
                self.claim_pip(clkref, clkref_in);
                let clkref_int = self.rc_io_wire(cell, "JREFCLK1");
                self.add_bel_wire(bcrd, "CLKREF_INT", clkref_int);
                self.claim_pip(clkref_in, clkref_int);
                bel.pins
                    .insert("CLKREF".into(), self.xlat_int_wire(bcrd, clkref_int));
                for (i, key) in [
                    (2, SpecialIoKey::MipiClk([DirH::W, DirH::E][idx])),
                    (3, SpecialIoKey::Clock(Dir::S, 4 + (idx as u8))),
                ] {
                    let clkref_io = self.rc_io_wire(cell, &format!("JREFCLK{i}"));
                    self.add_bel_wire(bcrd, format!("REFCLK{i}"), clkref_io);
                    let io = self.chip.special_io[&key];
                    let (cell_io, abcd) = self.xlat_io_loc_crosslink(io);
                    let paddi = self.rc_io_wire(cell_io, &format!("JPADDI{abcd}_PIO"));
                    self.claim_pip(clkref_io, paddi);
                    self.claim_pip(clkref_in, clkref_io);
                }

                self.insert_bel(bcrd, bel);

                let bcrd = bcrd.bel(bels::CLKTEST_MIPI);
                self.name_bel(bcrd, [format!("CLKTEST_MIPIDPHY{idx}")]);
                let mut bel = Bel::default();

                for i in 0..2 {
                    let wire = self.rc_io_wire(cell, &format!("JTESTIN{i}_CLKTEST"));
                    self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                    bel.pins
                        .insert(format!("TESTIN{i}"), self.xlat_int_wire(bcrd, wire));
                }
                for i in 2..6 {
                    let wire = self.rc_io_wire(cell, &format!("TESTIN{i}_CLKTEST"));
                    self.add_bel_wire(bcrd, format!("TESTIN{i}"), wire);
                }

                self.insert_bel(bcrd, bel);
            }
        }
    }

    pub fn process_serdes(&mut self) {
        match self.chip.kind {
            ChipKind::Scm => self.process_serdes_scm(),
            ChipKind::Ecp2M => self.process_serdes_ecp2(),
            ChipKind::Ecp3 | ChipKind::Ecp3A => self.process_serdes_ecp3(),
            ChipKind::Ecp4 => self.process_serdes_ecp4(),
            ChipKind::Ecp5 => self.process_serdes_ecp5(),
            ChipKind::Crosslink => self.process_mipi_crosslink(),
            _ => (),
        }
    }
}
