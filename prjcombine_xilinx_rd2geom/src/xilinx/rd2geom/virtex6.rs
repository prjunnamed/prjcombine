use super::builder::GeomBuilder;
use super::cfg::{GeomBuilderConfig, IntTermInfo, IntTileInfo, TieSiteInfo};
use super::part::PartBuilder;
use super::RdGeomMakerImpl;
use crate::xilinx::geomdb::builder::GeomDbBuilder;
use crate::xilinx::geomdb::{Dir, GeomDb, TieState};
use crate::xilinx::geomraw::GeomRaw;
use prjcombine_xilinx_rawdump as rawdump;
use std::collections::HashMap;

pub struct Virtex6GeomMaker {
    builder: GeomBuilder,
}

impl Virtex6GeomMaker {
    pub fn new(family: &str) -> Self {
        let int_tiles = vec![IntTileInfo::int("INT", "INT")];

        let int_terms = vec![
            IntTermInfo::b("BRKH_T_TERM_INT", "INT_BTERM"),
            IntTermInfo::t("BRKH_B_TERM_INT", "INT_TTERM"),
            IntTermInfo::l("L_TERM_INT", "INT_LTERM"),
            IntTermInfo::r("R_TERM_INT", "INT_RTERM"),
            IntTermInfo::b_multi("PCIE", (2, 0), "INT_BTERM"),
        ];

        let tie_sites = vec![TieSiteInfo {
            kind: "TIEOFF",
            pins: &[("HARD0", TieState::S0), ("HARD1", TieState::S1)],
        }];

        let tiles = vec![
            // XXX
        ];

        let cfg = GeomBuilderConfig {
            int_tiles,
            extra_cell_col_injectors: HashMap::new(),
            int_terms,
            int_bufs: Vec::new(),
            int_dbufs: Vec::new(),
            int_passes: Vec::new(),
            int_pass_combine: HashMap::new(),
            tie_sites,
            tiles,
        };
        let mut res = Virtex6GeomMaker {
            builder: GeomBuilder::new(family.to_string(), cfg),
        };
        res.setup();
        res
    }

    fn setup_int_ll(&mut self) {
        // The long wires.
        let lh: Vec<_> = (0..17)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LH{}", i), "INT.LH", true);
                self.builder.register_int_wire(w, &[&format!("LH{}", i)]);
                w
            })
            .collect();
        for i in 0..8 {
            self.builder.connect_int_wire(lh[i], Dir::W, lh[i + 1]);
            self.builder
                .connect_int_wire(lh[16 - i], Dir::E, lh[16 - i - 1]);
        }

        let lv: Vec<_> = (0..17)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LV{}", i), "INT.LV", true);
                self.builder.register_int_wire(w, &[&format!("LV{}", i)]);
                w
            })
            .collect();
        for i in 0..8 {
            self.builder.connect_int_wire(lv[i], Dir::N, lv[i + 1]);
            self.builder
                .connect_int_wire(lv[16 - i], Dir::S, lv[16 - i - 1]);
        }
    }

    fn setup_int_quad(&mut self) {
        for (name, da, db, dend) in [
            ("EE4", Dir::E, Dir::E, None),
            ("WW4", Dir::W, Dir::W, Some((Dir::S, 0))),
            ("NN4", Dir::N, Dir::N, Some((Dir::S, 1))),
            ("NE4", Dir::N, Dir::E, None),
            ("NW4", Dir::N, Dir::W, Some((Dir::S, 0))),
            ("SS4", Dir::S, Dir::S, Some((Dir::N, 0))),
            ("SE4", Dir::S, Dir::E, None),
            ("SW4", Dir::S, Dir::W, Some((Dir::N, 0))),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                let beg = self.builder.make_int_wire(
                    &format!("INT.{}BEG{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}BEG{}", name, i)],
                );
                let a = self.builder.make_int_wire(
                    &format!("INT.{}A{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}A{}", name, i)],
                );
                let b = self.builder.make_int_wire(
                    &format!("INT.{}B{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}B{}", name, i)],
                );
                let c = self.builder.make_int_wire(
                    &format!("INT.{}C{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}C{}", name, i)],
                );
                let end = self.builder.make_int_wire(
                    &format!("INT.{}END{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}END{}", name, i)],
                );
                self.builder.connect_int_wire(a, db, beg);
                self.builder.connect_int_wire(b, da, a);
                self.builder.connect_int_wire(c, da, b);
                self.builder.connect_int_wire(end, db, c);
                if i == 0 {
                    if let Some((Dir::S, x)) = dend {
                        let end_s = self.builder.make_int_wire(
                            &format!("INT.{}END_S{}", name, i),
                            "INT.QUAD",
                            &[&format!("{}END_S{}_{}", name, x, i)],
                        );
                        self.builder.connect_int_wire(end_s, Dir::S, end);
                    }
                }
                if i == 3 {
                    if let Some((Dir::N, x)) = dend {
                        let end_n = self.builder.make_int_wire(
                            &format!("INT.{}END_N{}", name, i),
                            "INT.QUAD",
                            &[&format!("{}END_N{}_{}", name, x, i)],
                        );
                        self.builder.connect_int_wire(end_n, Dir::N, end);
                    }
                }
            }
        }
    }

    fn setup_int_dbl(&mut self) {
        for (name, da, db, dend) in [
            ("EE2", Dir::E, Dir::E, None),
            ("WW2", Dir::W, Dir::W, Some((Dir::N, 0))),
            ("NN2", Dir::N, Dir::N, Some((Dir::S, 2))),
            ("NE2", Dir::N, Dir::E, Some((Dir::S, 3))),
            ("NW2", Dir::N, Dir::W, Some((Dir::S, 0))),
            ("SS2", Dir::S, Dir::S, Some((Dir::N, 0))),
            ("SE2", Dir::S, Dir::E, None),
            ("SW2", Dir::S, Dir::W, Some((Dir::N, 0))),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                let beg = self.builder.make_int_wire(
                    &format!("INT.{}BEG{}", name, i),
                    "INT.DBL",
                    &[&format!("{}BEG{}", name, i)],
                );
                let a = self.builder.make_int_wire(
                    &format!("INT.{}A{}", name, i),
                    "INT.DBL",
                    &[&format!("{}A{}", name, i)],
                );
                let end = self.builder.make_int_wire(
                    &format!("INT.{}END{}", name, i),
                    "INT.DBL",
                    &[&format!("{}END{}", name, i)],
                );
                self.builder.connect_int_wire(a, da, beg);
                self.builder.connect_int_wire(end, db, a);
                if i == 0 {
                    if let Some((Dir::S, x)) = dend {
                        let end_s = self.builder.make_int_wire(
                            &format!("INT.{}END_S{}", name, i),
                            "INT.DBL",
                            &[&format!("{}END_S{}_{}", name, x, i)],
                        );
                        self.builder.connect_int_wire(end_s, Dir::S, end);
                    }
                }
                if i == 3 {
                    if let Some((Dir::N, x)) = dend {
                        let end_n = self.builder.make_int_wire(
                            &format!("INT.{}END_N{}", name, i),
                            "INT.DBL",
                            &[&format!("{}END_N{}_{}", name, x, i)],
                        );
                        self.builder.connect_int_wire(end_n, Dir::N, end);
                    }
                }
            }
        }
    }

    fn setup_int_single(&mut self) {
        for (name, dir, dbeg, dend) in [
            ("EL1", Dir::E, Some(Dir::N), Some((Dir::S, 3))),
            ("ER1", Dir::E, Some(Dir::S), Some((Dir::N, 3))),
            ("WL1", Dir::W, Some(Dir::N), Some((Dir::N, 1))),
            ("WR1", Dir::W, Some(Dir::S), Some((Dir::S, 1))),
            ("NL1", Dir::N, Some(Dir::N), Some((Dir::S, 3))),
            ("NR1", Dir::N, None, None),
            ("SL1", Dir::S, None, None),
            ("SR1", Dir::S, Some(Dir::S), Some((Dir::N, 3))),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                if dir == Dir::N && dbeg == Some(Dir::N) && i == 3 {
                    self.builder.make_int_wire(
                        &format!("INT.SNG.{}BEG_N{}", name, i),
                        "INT.SNG",
                        &[&format!("{}BEG_N{}", name, i)],
                    );
                } else if dir == Dir::S && dbeg == Some(Dir::S) && i == 0 {
                    self.builder.make_int_wire(
                        &format!("INT.SNG.{}BEG_S{}", name, i),
                        "INT.SNG",
                        &[&format!("{}BEG_S{}", name, i)],
                    );
                } else {
                    let beg = self.builder.make_int_wire(
                        &format!("INT.SNG.{}BEG{}", name, i),
                        "INT.SNG",
                        &[&format!("{}BEG{}", name, i)],
                    );
                    let end = self.builder.make_int_wire(
                        &format!("INT.SNG.{}END{}", name, i),
                        "INT.SNG",
                        &[&format!("{}END{}", name, i)],
                    );
                    self.builder.connect_int_wire(end, dir, beg);

                    if i == 0 {
                        if dbeg == Some(Dir::S) {
                            let beg_s = self.builder.make_int_wire(
                                &format!("INT.SNG.{}BEG_S{}", name, i),
                                "INT.SNG",
                                &[&format!("{}BEG_S{}", name, i)],
                            );
                            self.builder.connect_int_wire(beg, Dir::N, beg_s);
                        }
                        if let Some((Dir::S, x)) = dend {
                            let end_s = self.builder.make_int_wire(
                                &format!("INT.SNG.{}END_S{}", name, i),
                                "INT.SNG",
                                &[&format!("{}END_S{}_{}", name, x, i)],
                            );
                            self.builder.connect_int_wire(end_s, Dir::S, end);
                        }
                    }
                    if i == 3 {
                        if dbeg == Some(Dir::N) {
                            let beg_n = self.builder.make_int_wire(
                                &format!("INT.SNG.{}BEG_N{}", name, i),
                                "INT.SNG",
                                &[&format!("{}BEG_N{}", name, i)],
                            );
                            self.builder.connect_int_wire(beg, Dir::S, beg_n);
                        }
                        if let Some((Dir::N, x)) = dend {
                            let end_n = self.builder.make_int_wire(
                                &format!("INT.SNG.{}END_N{}", name, i),
                                "INT.SNG",
                                &[&format!("{}END_N{}_{}", name, x, i)],
                            );
                            self.builder.connect_int_wire(end_n, Dir::N, end);
                        }
                    }
                }
            }
        }
    }

    fn setup_int_tie(&mut self) {
        let vcc = self
            .builder
            .geomdb
            .make_tie_wire("TIE.VCC", "TIE", TieState::S1);
        let gnd = self
            .builder
            .geomdb
            .make_tie_wire("TIE.GND", "TIE", TieState::S0);
        self.builder.register_int_wire(vcc, &["VCC_WIRE"]);
        self.builder.register_int_wire(gnd, &["GND_WIRE"]);
    }

    fn setup_int_imux(&mut self) {
        // The control inputs.
        for i in 0..2 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.GFAN{}", i),
                "INT.IMUX.GFAN",
                &[&format!("GFAN{}", i)],
            );
        }
        for i in 0..2 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.CLK{}", i),
                "INT.IMUX.CLK",
                &[&format!("CLK_B{}", i)],
            );
        }
        for i in 0..2 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.CTRL{}", i),
                "INT.IMUX.CTRL",
                &[&format!("CTRL_B{}", i)],
            );
        }
        for i in 0..8 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.BYP{}", i),
                "INT.IMUX.BYP",
                &[&format!("BYP{}", i)],
            );
            self.builder.make_int_wire(
                &format!("INT.IMUX.BYP{}.SITE", i),
                "INT.IMUX.BYP_SITE",
                &[&format!("BYP_B{}", i)],
            );
            let w = self.builder.make_int_wire(
                &format!("INT.IMUX.BYP{}.BOUNCE", i),
                "INT.IMUX.BYP_BOUNCE",
                &[&format!("BYP_BOUNCE{}", i)],
            );
            if i == 2 || i == 3 || i == 6 || i == 7 {
                let w_n = self.builder.make_int_wire(
                    &format!("INT.IMUX.BYP{}.BOUNCE.N", i),
                    "INT.IMUX.BYP_BOUNCE",
                    &[&format!("BYP_BOUNCE_N3_{}", i)],
                );
                self.builder.connect_int_wire(w_n, Dir::N, w);
            }
        }
        for i in 0..8 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.FAN{}", i),
                "INT.IMUX.FAN",
                &[&format!("FAN{}", i)],
            );
            self.builder.make_int_wire(
                &format!("INT.IMUX.FAN{}.SITE", i),
                "INT.IMUX.FAN_SITE",
                &[&format!("FAN_B{}", i)],
            );
            let w = self.builder.make_int_wire(
                &format!("INT.IMUX.FAN{}.BOUNCE", i),
                "INT.IMUX.FAN_BOUNCE",
                &[&format!("FAN_BOUNCE{}", i)],
            );
            if i == 0 || i == 2 || i == 4 || i == 6 {
                let w_s = self.builder.make_int_wire(
                    &format!("INT.IMUX.FAN{}.BOUNCE.S", i),
                    "INT.IMUX.FAN_BOUNCE",
                    &[&format!("FAN_BOUNCE_S3_{}", i)],
                );
                self.builder.connect_int_wire(w_s, Dir::S, w);
            }
        }
        for i in 0..48 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.IMUX{}", i),
                "INT.IMUX.IMUX",
                &[&format!("IMUX_B{}", i)],
            );
        }
    }

    fn setup_int_out(&mut self) {
        for i in 0..24 {
            self.builder.make_int_out_wire(
                &format!("INT.OUT{}", i),
                "INT.OUT",
                &[&format!("LOGIC_OUTS{}", i)],
            );
        }
    }

    fn setup_int(&mut self) {
        self.builder.setup_int();

        self.setup_int_ll();
        self.setup_int_quad();
        self.setup_int_dbl();
        self.setup_int_single();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();
    }

    fn setup_buses_clk(&mut self) {
        // XXX spine
        // XXX hrow
        // XXX qbuf

        // Final clock distribution tile: from QBUF and BUFR bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..8 {
            let w = self.builder.geomdb.make_vbus_wire(
                &format!("HCLK.CLK{}", i),
                "HCLK.CLK",
                vbus_hclk,
                false,
            );
            self.builder
                .register_int_wire(w, &[&format!("GCLK_B{}", i)]);
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
        if part.starts_with("xc6v") || part.starts_with("xq6v") {
            if part.ends_with('l') {
                part[2..part.len() - 1].to_string()
            } else {
                part[2..].to_string()
            }
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

impl RdGeomMakerImpl for Virtex6GeomMaker {
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
    fn finish(self: Box<Self>) -> (GeomDb, GeomRaw) {
        self.builder.finish()
    }
}
