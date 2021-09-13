use super::builder::GeomBuilder;
use super::cfg::{GeomBuilderConfig, IntTermInfo, IntTileInfo, TieSiteInfo};
use super::part::PartBuilder;
use super::RdGeomMakerImpl;
use crate::xilinx::geomdb::builder::GeomDbBuilder;
use crate::xilinx::geomdb::{Dir, GeomDb, TieState};
use crate::xilinx::geomraw::GeomRaw;
use prjcombine_xilinx_rawdump as rawdump;
use std::collections::HashMap;

pub struct Spartan6GeomMaker {
    builder: GeomBuilder,
}

impl Spartan6GeomMaker {
    pub fn new(family: &str) -> Self {
        let int_tiles = vec![
            IntTileInfo::int("INT", "INT"),
            IntTileInfo::int("INT_BRK", "INT"),
            IntTileInfo::int("INT_GCLK", "INT"),
            IntTileInfo::int("INT_BRAM", "INT"),
            IntTileInfo::int("INT_BRAM_BRK", "INT"),
            IntTileInfo::int("INT_TERM", "INT"),
            IntTileInfo::int("INT_TERM_BRK", "INT"),
            IntTileInfo::int("IOI_INT", "INT_IOI"),
            IntTileInfo::int("LIOI_INT", "INT_IOI"),
            IntTileInfo::int("LIOI_INT_BRK", "INT_IOI"),
        ];

        let mut int_terms = Vec::new();
        for t in &[
            "CNR_TL_LTERM",
            "IOI_LTERM",
            "IOI_LTERM_LOWER_BOT",
            "IOI_LTERM_LOWER_TOP",
            "IOI_LTERM_UPPER_BOT",
            "IOI_LTERM_UPPER_TOP",
            "INT_LTERM",
            "INT_INTERFACE_LTERM",
        ] {
            int_terms.push(IntTermInfo::l(t, "INT_LTERM"));
        }
        for t in &[
            "CNR_TL_RTERM",
            "IOI_RTERM",
            "IOI_RTERM_LOWER_BOT",
            "IOI_RTERM_LOWER_TOP",
            "IOI_RTERM_UPPER_BOT",
            "IOI_RTERM_UPPER_TOP",
            "INT_RTERM",
            "INT_INTERFACE_RTERM",
        ] {
            int_terms.push(IntTermInfo::r(t, "INT_RTERM"));
        }
        for t in &[
            "CNR_TR_TTERM",
            "IOI_TTERM",
            "IOI_TTERM_BUFPLL",
            "DSP_INT_TTERM",
            "RAMB_TOP_TTERM",
        ] {
            int_terms.push(IntTermInfo::t(t, "INT_TTERM"));
        }
        for t in &[
            "CNR_BR_BTERM",
            "IOI_BTERM",
            "IOI_BTERM_BUFPLL",
            "CLB_INT_BTERM",
            "DSP_INT_BTERM",
            "RAMB_BOT_BTERM",
        ] {
            int_terms.push(IntTermInfo::b(t, "INT_BTERM"));
        }

        let tie_sites = vec![TieSiteInfo {
            kind: "TIEOFF",
            pins: &[
                ("HARD0", TieState::S0),
                ("HARD1", TieState::S1),
                ("KEEP1", TieState::S1),
            ],
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
        let mut res = Spartan6GeomMaker {
            builder: GeomBuilder::new(family.to_string(), cfg),
        };
        res.setup();
        res
    }

    fn setup_int_quad(&mut self) {
        for (name, da, db, dend) in [
            ("EE4", Dir::E, Dir::E, None),
            ("WW4", Dir::W, Dir::W, Some(Dir::S)),
            ("NN4", Dir::N, Dir::N, None),
            ("NE4", Dir::N, Dir::E, None),
            ("NW4", Dir::N, Dir::W, Some(Dir::S)),
            ("SS4", Dir::S, Dir::S, Some(Dir::N)),
            ("SE4", Dir::S, Dir::E, None),
            ("SW4", Dir::S, Dir::W, Some(Dir::N)),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                let b = self.builder.make_int_wire(
                    &format!("INT.{}B{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}B{}", name, i)],
                );
                let a = self.builder.make_int_wire(
                    &format!("INT.{}A{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}A{}", name, i)],
                );
                let m = self.builder.make_int_wire(
                    &format!("INT.{}M{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}M{}", name, i)],
                );
                let c = self.builder.make_int_wire(
                    &format!("INT.{}C{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}C{}", name, i)],
                );
                let e = self.builder.make_int_wire(
                    &format!("INT.{}E{}", name, i),
                    "INT.QUAD",
                    &[&format!("{}E{}", name, i)],
                );
                self.builder.connect_int_wire(a, da, b);
                self.builder.connect_int_wire(m, da, a);
                self.builder.connect_int_wire(c, db, m);
                self.builder.connect_int_wire(e, db, c);
                if i == 0 && dend == Some(Dir::S) {
                    let e_s = self.builder.make_int_wire(
                        &format!("INT.{}E_S{}", name, i),
                        "INT.QUAD",
                        &[&format!("{}E_S{}", name, i)],
                    );
                    self.builder.connect_int_wire(e_s, Dir::S, e);
                }
                if i == 3 && dend == Some(Dir::N) {
                    let e_n = self.builder.make_int_wire(
                        &format!("INT.{}E_N{}", name, i),
                        "INT.QUAD",
                        &[&format!("{}E_N{}", name, i)],
                    );
                    self.builder.connect_int_wire(e_n, Dir::N, e);
                }
            }
        }
    }

    fn setup_int_dbl(&mut self) {
        for (name, da, db, dend) in [
            ("EE2", Dir::E, Dir::E, None),
            ("WW2", Dir::W, Dir::W, Some(Dir::N)),
            ("NN2", Dir::N, Dir::N, Some(Dir::S)),
            ("NE2", Dir::N, Dir::E, Some(Dir::S)),
            ("NW2", Dir::N, Dir::W, Some(Dir::S)),
            ("SS2", Dir::S, Dir::S, Some(Dir::N)),
            ("SE2", Dir::S, Dir::E, None),
            ("SW2", Dir::S, Dir::W, Some(Dir::N)),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                let b = self.builder.make_int_wire(
                    &format!("INT.{}B{}", name, i),
                    "INT.DBL",
                    &[&format!("{}B{}", name, i)],
                );
                let m = self.builder.make_int_wire(
                    &format!("INT.{}M{}", name, i),
                    "INT.DBL",
                    &[&format!("{}M{}", name, i)],
                );
                let e = self.builder.make_int_wire(
                    &format!("INT.{}E{}", name, i),
                    "INT.DBL",
                    &[&format!("{}E{}", name, i)],
                );
                self.builder.connect_int_wire(m, da, b);
                self.builder.connect_int_wire(e, db, m);
                if i == 0 && dend == Some(Dir::S) {
                    let e_s = self.builder.make_int_wire(
                        &format!("INT.{}E_S{}", name, i),
                        "INT.DBL",
                        &[&format!("{}E_S{}", name, i)],
                    );
                    self.builder.connect_int_wire(e_s, Dir::S, e);
                }
                if i == 3 && dend == Some(Dir::N) {
                    let e_n = self.builder.make_int_wire(
                        &format!("INT.{}E_N{}", name, i),
                        "INT.DBL",
                        &[&format!("{}E_N{}", name, i)],
                    );
                    self.builder.connect_int_wire(e_n, Dir::N, e);
                }
            }
        }
    }

    fn setup_int_single(&mut self) {
        for (name, dir, dend) in [
            ("EL1", Dir::E, Some(Dir::S)),
            ("ER1", Dir::E, Some(Dir::N)),
            ("WL1", Dir::W, Some(Dir::N)),
            ("WR1", Dir::W, Some(Dir::S)),
            ("NL1", Dir::N, Some(Dir::S)),
            ("NR1", Dir::N, None),
            ("SL1", Dir::S, None),
            ("SR1", Dir::S, Some(Dir::N)),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                let b = self.builder.make_int_wire(
                    &format!("INT.SNG.{}B{}", name, i),
                    "INT.SNG",
                    &[&format!("{}B{}", name, i)],
                );
                let e = self.builder.make_int_wire(
                    &format!("INT.SNG.{}E{}", name, i),
                    "INT.SNG",
                    &[&format!("{}E{}", name, i)],
                );
                self.builder.connect_int_wire(e, dir, b);

                if i == 0 && dend == Some(Dir::S) {
                    let e_s = self.builder.make_int_wire(
                        &format!("INT.SNG.{}E_S{}", name, i),
                        "INT.SNG",
                        &[&format!("{}E_S{}", name, i)],
                    );
                    self.builder.connect_int_wire(e_s, Dir::S, e);
                }
                if i == 3 && dend == Some(Dir::N) {
                    let e_n = self.builder.make_int_wire(
                        &format!("INT.SNG.{}E_N{}", name, i),
                        "INT.SNG",
                        &[&format!("{}E_N{}", name, i)],
                    );
                    self.builder.connect_int_wire(e_n, Dir::N, e);
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
        let pullup = self
            .builder
            .geomdb
            .make_tie_wire("TIE.PULLUP", "TIE", TieState::S1);
        self.builder.register_int_wire(vcc, &["VCC_WIRE"]);
        self.builder.register_int_wire(gnd, &["GND_WIRE"]);
        self.builder.register_int_wire(pullup, &["KEEP1_WIRE"]);
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
                &[&format!("CLK{}", i)],
            );
        }
        for i in 0..2 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.SR{}", i),
                "INT.IMUX.SR",
                &[&format!("SR{}", i)],
            );
        }
        for i in 0..63 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.LOGICIN{}", i),
                "INT.IMUX.LOGICIN",
                &[
                    &format!("LOGICIN_B{}", i),
                    &format!("INT_TERM_LOGICIN_B{}", i),
                ],
            );
            let dir = match i {
                20 | 36 | 44 | 62 => Dir::S,
                21 | 28 | 52 | 60 => Dir::N,
                _ => continue,
            };
            let w = self.builder.make_int_wire(
                &format!("INT.IMUX.LOGICIN{}.BOUNCE", i),
                "INT.IMUX.LOGICIN.BOUNCE",
                &[&format!("LOGICIN{}", i)],
            );
            if dir == Dir::S {
                let w_s = self.builder.make_int_wire(
                    &format!("INT.IMUX.LOGICIN{}.BOUNCE.S", i),
                    "INT.IMUX.LOGICIN.BOUNCE",
                    &[&format!("LOGICIN_S{}", i)],
                );
                self.builder.connect_int_wire(w_s, dir, w);
            } else {
                let w_n = self.builder.make_int_wire(
                    &format!("INT.IMUX.LOGICIN{}.BOUNCE.N", i),
                    "INT.IMUX.LOGICIN.BOUNCE",
                    &[&format!("LOGICIN_N{}", i)],
                );
                self.builder.connect_int_wire(w_n, dir, w);
            }
        }
        self.builder.make_int_wire(
            &format!("INT.IMUX.LOGICIN{}", 63),
            "INT.IMUX.LOGICIN",
            &["FAN_B"],
        );
    }

    fn setup_int_out(&mut self) {
        for i in 0..24 {
            self.builder.make_int_out_wire(
                &format!("INT.OUT{}", i),
                "INT.OUT",
                &[
                    &format!("LOGICOUT{}", i),
                    &format!("INT_TERM_LOGICOUT{}", i),
                ],
            );
        }
    }

    fn setup_int(&mut self) {
        self.builder.setup_int();

        self.setup_int_quad();
        self.setup_int_dbl();
        self.setup_int_single();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();
    }

    fn setup_buses_clk(&mut self) {
        // XXX

        // Final clock distribution tile: from HROW/FOLD bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..16 {
            let w = self.builder.geomdb.make_vbus_wire(
                &format!("HCLK.CLK{}", i),
                "HCLK.CLK",
                vbus_hclk,
                false,
            );
            self.builder
                .register_int_wire(w, &[&format!("GCLK{}", i), &format!("GCLK{}_BRK", i)]);
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
        if part.starts_with("xc6s") || part.starts_with("xa6s") || part.starts_with("xq6s") {
            if part.ends_with('l') {
                part[2..part.len() - 1].to_string()
            } else {
                part[2..].to_string()
            }
        } else {
            panic!("unregognized part name {}", part);
        }
    }

    fn curse_tiles(&mut self, part: &mut PartBuilder) {
        for x in 0..part.rd.width - 2 {
            for y in 0..part.rd.height {
                let crd_a = rawdump::Coord { x, y };
                let crd_b = rawdump::Coord { x: x + 2, y };
                let tile_a = &part.rd.tiles[&crd_a];
                let tile_b = &part.rd.tiles[&crd_b];
                if tile_a.kind == "INT_LTERM" && tile_b.kind == "INT_LTERM" {
                    part.curse_tile(crd_a);
                }
            }
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

impl RdGeomMakerImpl for Spartan6GeomMaker {
    fn get_family(&self) -> &str {
        &self.builder.geomdb.name
    }
    fn ingest(&mut self, rd: &rawdump::Part) {
        let grid_name = Self::make_grid_name(&rd.part);
        let mut part = PartBuilder::new(grid_name, rd, &self.builder);
        self.curse_tiles(&mut part);
        self.fill_grid(&mut part);
        self.verify(&part);
        self.builder.ingest(part);
    }
    fn finish(self: Box<Self>) -> (GeomDb, GeomRaw) {
        self.builder.finish()
    }
}
