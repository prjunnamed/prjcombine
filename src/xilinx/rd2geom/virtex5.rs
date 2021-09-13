use std::collections::HashMap;
use crate::xilinx::rawdump;
use crate::xilinx::geomdb::{GeomDb, TieState, Dir};
use crate::xilinx::geomdb::builder::{GeomDbBuilder};
use crate::xilinx::geomraw::GeomRaw;
use super::RdGeomMakerImpl;
use super::cfg::{GeomBuilderConfig, IntTileInfo, IntTermInfo, IntDoubleBufInfo, TieSiteInfo};
use super::builder::GeomBuilder;
use super::part::PartBuilder;

pub struct Virtex5GeomMaker {
    builder: GeomBuilder,
}

impl Virtex5GeomMaker {
    pub fn new(family: &str) -> Self {
        let int_tiles = vec![
            IntTileInfo::int("INT", "INT"),
        ];

        let int_terms = vec![
            IntTermInfo::b("PPC_T_TERM", "INT_BTERM"),
            IntTermInfo::t("PPC_B_TERM", "INT_TTERM"),
            IntTermInfo::l("L_TERM_INT", "INT_LTERM"),
            IntTermInfo::r("R_TERM_INT", "INT_RTERM"),
            IntTermInfo::l("GTX_L_TERM_INT", "INT_LTERM"),
        ];

        let int_dbufs = vec![
            IntDoubleBufInfo::h("INT_BUFS_L", "INT_BUFS_R", "INT_HBUF"),
            IntDoubleBufInfo::h("INT_BUFS_L", "INT_BUFS_R_MON", "INT_HBUF"),
            IntDoubleBufInfo::h("L_TERM_PPC", "R_TERM_PPC", "INT_HPPC"),
        ];

        let tie_sites = vec![
            TieSiteInfo {
                kind: "TIEOFF",
                pins: &[
                    ("HARD0", TieState::S0),
                    ("HARD1", TieState::S1),
                    ("KEEP1", TieState::S1),
                ],
            },
        ];

        let tiles = vec![
            // XXX
        ];

        let cfg = GeomBuilderConfig {
            int_tiles,
            extra_cell_col_injectors: HashMap::new(),
            int_terms,
            int_bufs: Vec::new(),
            int_dbufs,
            int_passes: Vec::new(),
            int_pass_combine: HashMap::new(),
            tie_sites,
            tiles,
        };
        let mut res = Virtex5GeomMaker {
            builder: GeomBuilder::new(family.to_string(), cfg),
        };
        res.setup();
        res
    }

    fn setup_int_ll(&mut self) {
        // The long wires.
        let lh: Vec<_> = (0..19).map(|i| {
            let w = self.builder.geomdb.make_wire(&format!("INT.LH{}", i), "INT.LH", true);
            self.builder.register_int_wire(w, &[&format!("LH{}", i)]);
            w
        }).collect();
        for i in 0..9 {
            self.builder.connect_int_wire(lh[i], Dir::W, lh[i+1]);
            self.builder.connect_int_wire(lh[18-i], Dir::E, lh[18-i-1]);
        }

        let lv: Vec<_> = (0..19).map(|i| {
            let w = self.builder.geomdb.make_wire(&format!("INT.LV{}", i), "INT.LV", true);
            self.builder.register_int_wire(w, &[&format!("LV{}", i)]);
            w
        }).collect();
        for i in 0..9 {
            self.builder.connect_int_wire(lv[i], Dir::N, lv[i+1]);
            self.builder.connect_int_wire(lv[18-i], Dir::S, lv[18-i-1]);
        }
    }

    fn setup_int_pent(&mut self) {
        for (name, da, db, dbeg, dend, dmid) in [
            ("EL5", Dir::E, Dir::E, None, None, None),
            ("ER5", Dir::E, Dir::E, None, None, None),
            ("EN5", Dir::E, Dir::N, None, None, None),
            ("ES5", Dir::E, Dir::S, None, None, None),

            ("WL5", Dir::W, Dir::W, Some(Dir::S), None, None),
            ("WR5", Dir::W, Dir::W, None, None, None),
            ("WN5", Dir::W, Dir::N, None, Some(Dir::S), None),
            ("WS5", Dir::W, Dir::S, None, None, Some(Dir::S)),

            ("NL5", Dir::N, Dir::N, None, None, None),
            ("NR5", Dir::N, Dir::N, Some(Dir::N), None, None),
            ("NE5", Dir::N, Dir::E, None, None, Some(Dir::N)),
            ("NW5", Dir::N, Dir::W, None, Some(Dir::N), None),

            ("SL5", Dir::S, Dir::S, None, None, None),
            ("SR5", Dir::S, Dir::S, None, None, None),
            ("SE5", Dir::S, Dir::E, None, None, None),
            ("SW5", Dir::S, Dir::W, None, None, None),
        ].iter().copied() {
            for i in 0..3 {
                let beg = self.builder.make_int_wire(&format!("INT.{}BEG{}", name, i), "INT.PENT", &[&format!("{}BEG{}", name, i)]);
                let a = self.builder.make_int_wire(&format!("INT.{}A{}", name, i), "INT.PENT", &[&format!("{}A{}", name, i)]);
                let b = self.builder.make_int_wire(&format!("INT.{}B{}", name, i), "INT.PENT", &[&format!("{}B{}", name, i)]);
                let mid = self.builder.make_int_wire(&format!("INT.{}MID{}", name, i), "INT.PENT", &[&format!("{}MID{}", name, i)]);
                let c = self.builder.make_int_wire(&format!("INT.{}C{}", name, i), "INT.PENT", &[&format!("{}C{}", name, i)]);
                let end = self.builder.make_int_wire(&format!("INT.{}END{}", name, i), "INT.PENT", &[&format!("{}END{}", name, i)]);
                self.builder.connect_int_wire(a, da, beg);
                self.builder.connect_int_wire(b, da, a);
                self.builder.connect_int_wire(mid, da, b);
                self.builder.connect_int_wire(c, db, mid);
                self.builder.connect_int_wire(end, db, c);
                if dbeg == Some(Dir::S) && i == 0 {
                    let beg_s = self.builder.make_int_wire(&format!("INT.{}BEG_S{}", name, i), "INT.PENT", &[&format!("{}BEG_S{}", name, i)]);
                    self.builder.connect_int_wire(beg, Dir::N, beg_s);
                }
                if dbeg == Some(Dir::N) && i == 2 {
                    let beg_n = self.builder.make_int_wire(&format!("INT.{}BEG_N{}", name, i), "INT.PENT", &[&format!("{}BEG_N{}", name, i)]);
                    self.builder.connect_int_wire(beg, Dir::S, beg_n);
                }
                if dend == Some(Dir::S) && i == 0 {
                    let end_s = self.builder.make_int_wire(&format!("INT.{}END_S{}", name, i), "INT.PENT", &[&format!("{}END_S{}", name, i)]);
                    self.builder.connect_int_wire(end_s, Dir::S, end);
                }
                if dend == Some(Dir::N) && i == 2 {
                    let end_n = self.builder.make_int_wire(&format!("INT.{}END_N{}", name, i), "INT.PENT", &[&format!("{}END_N{}", name, i)]);
                    self.builder.connect_int_wire(end_n, Dir::N, end);
                }
                if dmid == Some(Dir::S) && i == 0 {
                    let mid_fake = self.builder.make_int_wire(&format!("INT.{}MID_FAKE{}", name, i), "INT.PENT", &[&format!("{}MID_FAKE{}", name, i)]);
                    let mid_s = self.builder.make_int_wire(&format!("INT.{}MID_S{}", name, i), "INT.PENT", &[&format!("{}MID_S{}", name, i)]);
                    self.builder.connect_int_wire(mid_s, Dir::S, mid_fake);
                }
                if dmid == Some(Dir::N) && i == 2 {
                    let mid_fake = self.builder.make_int_wire(&format!("INT.{}MID_FAKE{}", name, i), "INT.PENT", &[&format!("{}MID_FAKE{}", name, i)]);
                    let mid_n = self.builder.make_int_wire(&format!("INT.{}MID_N{}", name, i), "INT.PENT", &[&format!("{}MID_N{}", name, i)]);
                    self.builder.connect_int_wire(mid_n, Dir::N, mid_fake);
                }
            }
        }
    }

    fn setup_int_dbl(&mut self) {
        for (name, da, db, dbeg, dend, dmid) in [
            ("EL2", Dir::E, Dir::E, None, None, None),
            ("ER2", Dir::E, Dir::E, Some(Dir::S), None, None),
            ("EN2", Dir::E, Dir::N, None, None, None),
            ("ES2", Dir::E, Dir::S, None, None, None),

            ("WL2", Dir::W, Dir::W, Some(Dir::S), None, None),
            ("WR2", Dir::W, Dir::W, Some(Dir::N), None, None),
            ("WN2", Dir::W, Dir::N, None, Some(Dir::S), None),
            ("WS2", Dir::W, Dir::S, None, None, Some(Dir::S)),

            ("NL2", Dir::N, Dir::N, Some(Dir::S), None, None),
            ("NR2", Dir::N, Dir::N, Some(Dir::N), None, None),
            ("NE2", Dir::N, Dir::E, None, None, Some(Dir::N)),
            ("NW2", Dir::N, Dir::W, None, Some(Dir::N), None),

            ("SL2", Dir::S, Dir::S, Some(Dir::N), None, None),
            ("SR2", Dir::S, Dir::S, None, None, None),
            ("SE2", Dir::S, Dir::E, None, None, None),
            ("SW2", Dir::S, Dir::W, None, None, None),
        ].iter().copied() {
            for i in 0..3 {
                let beg = self.builder.make_int_wire(&format!("INT.{}BEG{}", name, i), "INT.DBL", &[&format!("{}BEG{}", name, i)]);
                let mid = self.builder.make_int_wire(&format!("INT.{}MID{}", name, i), "INT.DBL", &[&format!("{}MID{}", name, i)]);
                let end = self.builder.make_int_wire(&format!("INT.{}END{}", name, i), "INT.DBL", &[&format!("{}END{}", name, i)]);
                self.builder.connect_int_wire(mid, da, beg);
                self.builder.connect_int_wire(end, db, mid);
                if dbeg == Some(Dir::S) && i == 0 {
                    let beg_s = self.builder.make_int_wire(&format!("INT.{}BEG_S{}", name, i), "INT.DBL", &[&format!("{}BEG_S{}", name, i)]);
                    self.builder.connect_int_wire(beg, Dir::N, beg_s);
                }
                if dbeg == Some(Dir::N) && i == 2 {
                    let beg_n = self.builder.make_int_wire(&format!("INT.{}BEG_N{}", name, i), "INT.DBL", &[&format!("{}BEG_N{}", name, i)]);
                    self.builder.connect_int_wire(beg, Dir::S, beg_n);
                }
                if dend == Some(Dir::S) && i == 0 {
                    let end_s = self.builder.make_int_wire(&format!("INT.{}END_S{}", name, i), "INT.DBL", &[&format!("{}END_S{}", name, i)]);
                    self.builder.connect_int_wire(end_s, Dir::S, end);
                }
                if dend == Some(Dir::N) && i == 2 {
                    let end_n = self.builder.make_int_wire(&format!("INT.{}END_N{}", name, i), "INT.DBL", &[&format!("{}END_N{}", name, i)]);
                    self.builder.connect_int_wire(end_n, Dir::N, end);
                }
                if dmid == Some(Dir::S) && i == 0 {
                    let mid_fake = self.builder.make_int_wire(&format!("INT.{}MID_FAKE{}", name, i), "INT.DBL", &[&format!("{}MID_FAKE{}", name, i)]);
                    let mid_s = self.builder.make_int_wire(&format!("INT.{}MID_S{}", name, i), "INT.DBL", &[&format!("{}MID_S{}", name, i)]);
                    self.builder.connect_int_wire(mid_s, Dir::S, mid_fake);
                }
                if dmid == Some(Dir::N) && i == 2 {
                    let mid_fake = self.builder.make_int_wire(&format!("INT.{}MID_FAKE{}", name, i), "INT.DBL", &[&format!("{}MID_FAKE{}", name, i)]);
                    let mid_n = self.builder.make_int_wire(&format!("INT.{}MID_N{}", name, i), "INT.DBL", &[&format!("{}MID_N{}", name, i)]);
                    self.builder.connect_int_wire(mid_n, Dir::N, mid_fake);
                }
            }
        }
    }

    fn setup_int_tie(&mut self) {
        let vcc = self.builder.geomdb.make_tie_wire("TIE.VCC", "TIE", TieState::S1);
        let gnd = self.builder.geomdb.make_tie_wire("TIE.GND", "TIE", TieState::S0);
        let pullup = self.builder.geomdb.make_tie_wire("TIE.PULLUP", "TIE", TieState::S1);
        self.builder.register_int_wire(vcc, &["VCC_WIRE"]);
        self.builder.register_int_wire(gnd, &["GND_WIRE"]);
        self.builder.register_int_wire(pullup, &["KEEP1_WIRE"]);
    }

    fn setup_int_imux(&mut self) {
        // The control inputs.
        for i in 0..2 {
            self.builder.make_int_wire(&format!("INT.IMUX.GFAN{}", i), "INT.IMUX.GFAN", &[&format!("GFAN{}", i)]);
        }
        for i in 0..2 {
            self.builder.make_int_wire(&format!("INT.IMUX.CLK{}", i), "INT.IMUX.CLK", &[&format!("CLK_B{}", i)]);
        }
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.CTRL{}", i), "INT.IMUX.CTRL", &[&format!("CTRL{}", i)]);
            self.builder.make_int_wire(&format!("INT.IMUX.CTRL{}.SITE", i), "INT.IMUX.CTRL_SITE", &[&format!("CTRL_B{}", i)]);
            let w = self.builder.make_int_wire(&format!("INT.IMUX.CTRL{}.BOUNCE", i), "INT.IMUX.CTRL_BOUNCE", &[&format!("CTRL_BOUNCE{}", i)]);
            if i == 0 {
                let w_s = self.builder.make_int_wire(&format!("INT.IMUX.CTRL{}.BOUNCE.S", i), "INT.IMUX.CTRL_BOUNCE", &[&format!("CTRL_BOUNCE_S{}", i)]);
                self.builder.connect_int_wire(w_s, Dir::S, w);
            }
            if i == 3 {
                let w_n = self.builder.make_int_wire(&format!("INT.IMUX.CTRL{}.BOUNCE.N", i), "INT.IMUX.CTRL_BOUNCE", &[&format!("CTRL_BOUNCE_N{}", i)]);
                self.builder.connect_int_wire(w_n, Dir::N, w);
            }
        }
        for i in 0..8 {
            self.builder.make_int_wire(&format!("INT.IMUX.BYP{}", i), "INT.IMUX.BYP", &[&format!("BYP{}", i)]);
            self.builder.make_int_wire(&format!("INT.IMUX.BYP{}.SITE", i), "INT.IMUX.BYP_SITE", &[&format!("BYP_B{}", i)]);
            let w = self.builder.make_int_wire(&format!("INT.IMUX.BYP{}.BOUNCE", i), "INT.IMUX.BYP_BOUNCE", &[&format!("BYP_BOUNCE{}", i)]);
            if i == 0 || i == 4 {
                let w_s = self.builder.make_int_wire(&format!("INT.IMUX.BYP{}.BOUNCE.S", i), "INT.IMUX.BYP_BOUNCE", &[&format!("BYP_BOUNCE_S{}", i)]);
                self.builder.connect_int_wire(w_s, Dir::S, w);
            }
            if i == 3 || i == 7 {
                let w_n = self.builder.make_int_wire(&format!("INT.IMUX.BYP{}.BOUNCE.N", i), "INT.IMUX.BYP_BOUNCE", &[&format!("BYP_BOUNCE_N{}", i)]);
                self.builder.connect_int_wire(w_n, Dir::N, w);
            }
        }
        for i in 0..8 {
            self.builder.make_int_wire(&format!("INT.IMUX.FAN{}", i), "INT.IMUX.FAN", &[&format!("FAN{}", i)]);
            self.builder.make_int_wire(&format!("INT.IMUX.FAN{}.SITE", i), "INT.IMUX.FAN_SITE", &[&format!("FAN_B{}", i)]);
            let w = self.builder.make_int_wire(&format!("INT.IMUX.FAN{}.BOUNCE", i), "INT.IMUX.FAN_BOUNCE", &[&format!("FAN_BOUNCE{}", i)]);
            if i == 0 {
                let w_s = self.builder.make_int_wire(&format!("INT.IMUX.FAN{}.BOUNCE.S", i), "INT.IMUX.FAN_BOUNCE", &[&format!("FAN_BOUNCE_S{}", i)]);
                self.builder.connect_int_wire(w_s, Dir::S, w);
            }
            if i == 7 {
                let w_n = self.builder.make_int_wire(&format!("INT.IMUX.FAN{}.BOUNCE.N", i), "INT.IMUX.FAN_BOUNCE", &[&format!("FAN_BOUNCE_N{}", i)]);
                self.builder.connect_int_wire(w_n, Dir::N, w);
            }
        }
        for i in 0..48 {
            self.builder.make_int_wire(&format!("INT.IMUX.IMUX{}", i), "INT.IMUX.IMUX", &[&format!("IMUX_B{}", i)]);
        }
    }

    fn setup_int_out(&mut self) {
        for i in 0..24 {
            let w = self.builder.make_int_out_wire(&format!("INT.OUT{}", i), "INT.OUT", &[
                &format!("LOGIC_OUTS{}", i),
            ]);
            if i == 15 || i == 17 {
                let w_n2 = self.builder.make_int_wire(&format!("INT.OUT{}.N2", i), "INT.OUT", &[
                    &format!("LOGIC_OUTS_N{}", i),
                ]);
                let w_n5 = self.builder.make_int_wire(&format!("INT.OUT{}.N5", i), "INT.OUT", &[
                    &format!("LOGIC_OUTS_N1_{}", i),
                ]);
                self.builder.connect_int_wire(w_n2, Dir::N, w);
                self.builder.connect_int_wire(w_n5, Dir::N, w);
            }
            if i == 12 || i == 18 {
                let w_s2 = self.builder.make_int_wire(&format!("INT.OUT{}.S2", i), "INT.OUT", &[
                    &format!("LOGIC_OUTS_S{}", i),
                ]);
                let w_s5 = self.builder.make_int_wire(&format!("INT.OUT{}.S5", i), "INT.OUT", &[
                    &format!("LOGIC_OUTS_S1_{}", i),
                ]);
                self.builder.connect_int_wire(w_s2, Dir::S, w);
                self.builder.connect_int_wire(w_s5, Dir::S, w);
            }
        }
    }

    fn setup_int(&mut self) {
        self.builder.setup_int();

        self.setup_int_ll();
        self.setup_int_pent();
        self.setup_int_dbl();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();
    }

    fn setup_buses_clk(&mut self) {
        // XXX spine
        // XXX hrow

        // Final clock distribution tile: from HROW bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..10 {
            let w = self.builder.geomdb.make_vbus_wire(&format!("HCLK.GCLK{}", i), "HCLK.CLK", vbus_hclk, false);
            self.builder.register_int_wire(w, &[&format!("GCLK{}", i)]);
        }
        for i in 0..4 {
            let w = self.builder.geomdb.make_vbus_wire(&format!("HCLK.RCLK{}", i), "HCLK.CLK", vbus_hclk, false);
            self.builder.register_int_wire(w, &[&format!("RCLK{}", i)]);
        }
    }

    fn setup_site(&mut self) {
        // XXX
    }

    fn setup(&mut self) {
        self.setup_int();
        self.builder.setup_tiles();
        self.setup_buses_clk();
        self.setup_site();
    }

    fn make_grid_name(part: &str) -> String {
        if part.starts_with("xc5v") || part.starts_with("xq5v") {
            part[2..].to_string()
        } else {
            panic!("unregognized part name {}", part);
        }
    }

    fn fill_grid_bus(&mut self, _part: &mut PartBuilder) {
        // XXX
    }

    fn fill_grid_site_conns(&mut self, _part: &mut PartBuilder) {
        // XXX
    }

    fn fill_grid_cols(&mut self, _part: &mut PartBuilder) {
        // XXX
    }

    fn fill_grid(&mut self, part: &mut PartBuilder) {
        part.fill_int(&mut self.builder);
        self.fill_grid_bus(part);
        self.fill_grid_site_conns(part);
        part.fill_tiles(&self.builder);
        self.fill_grid_cols(part);
    }

    fn verify(&self, _part: &PartBuilder) {
        // XXX
    }
}

impl RdGeomMakerImpl for Virtex5GeomMaker {
    fn get_family(&self) -> &str {
        &self.builder.geomdb.name
    }
    fn ingest(&mut self, rd: &rawdump::Part) {
        let grid_name = Self::make_grid_name(&rd.part);
        let mut part = PartBuilder::new(grid_name, rd, &self.builder);
        self.fill_grid(&mut part);
        self.verify(&part);
        self.builder.ingest(part);
    }
    fn finish(self:Box<Self>) -> (GeomDb, GeomRaw) {
        self.builder.finish()
    }
}
