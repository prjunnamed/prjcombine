use prjcombine_ecp::{
    bels,
    chip::{ChipKind, SpecialLocKey},
};
use prjcombine_interconnect::dir::{DirH, DirHV, DirV};

use crate::ChipContext;

impl ChipContext<'_> {
    fn process_serdes_ecp2(&mut self) {
        for tcname in ["SERDES_S", "SERDES_N"] {
            let tcid = self.intdb.get_tile_class(tcname);
            for &tcrd in &self.edev.egrid.tile_index[tcid] {
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
        for &tcrd in &self.edev.egrid.tile_index[tcid] {
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
            if let Some(cell_prev) = self.edev.egrid.cell_delta(bcrd.cell, -36, 0)
                && self.edev.egrid.has_bel(cell_prev.bel(bels::SERDES))
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

    pub fn process_serdes(&mut self) {
        match self.chip.kind {
            ChipKind::Ecp2M => {
                self.process_serdes_ecp2();
            }
            ChipKind::Ecp3 | ChipKind::Ecp3A => {
                self.process_serdes_ecp3();
            }
            ChipKind::Ecp4 => {
                self.process_serdes_ecp4();
            }
            _ => (),
        }
    }
}
