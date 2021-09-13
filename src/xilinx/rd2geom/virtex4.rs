use std::collections::HashMap;
use crate::xilinx::rawdump;
use crate::xilinx::geomdb::{GeomDb, TieState, Dir};
use crate::xilinx::geomdb::builder::{GeomDbBuilder};
use crate::xilinx::geomraw::GeomRaw;
use super::RdGeomMakerImpl;
use super::cfg::{GeomBuilderConfig, IntTileInfo, IntTermInfo, IntPassInfo, IntBufInfo, IntDoubleBufInfo, TieSiteInfo};
use super::builder::GeomBuilder;
use super::part::PartBuilder;

pub struct Virtex4GeomMaker {
    builder: GeomBuilder,
}

impl Virtex4GeomMaker {
    pub fn new(family: &str) -> Self {
        let int_tiles = vec![
            IntTileInfo::int("INT", "INT"),
            IntTileInfo::int("INT_SO", "INT"),
            IntTileInfo::int("INT_SO_DCM0", "INT"),
        ];

        let int_terms = vec![
            IntTermInfo::b("B_TERM_INT", "INT_BTERM"),
            IntTermInfo::t("T_TERM_INT", "INT_TTERM"),
            IntTermInfo::l("L_TERM_INT", "INT_LTERM"),
            IntTermInfo::r("R_TERM_INT", "INT_RTERM"),
            IntTermInfo::l_multi("MGT_AL_BOT", (16, 8), "INT_LTERM"),
            IntTermInfo::l_multi("MGT_AL_MID", (16, 8), "INT_LTERM"),
            IntTermInfo::l_multi("MGT_AL", (16, 8), "INT_LTERM"),
            IntTermInfo::l_multi("MGT_BL", (16, 8), "INT_LTERM"),
            IntTermInfo::r_multi("MGT_AR_BOT", (16, 8), "INT_RTERM"),
            IntTermInfo::r_multi("MGT_AR_MID", (16, 8), "INT_RTERM"),
            IntTermInfo::r_multi("MGT_AR", (16, 8), "INT_RTERM"),
            IntTermInfo::r_multi("MGT_BR", (16, 8), "INT_RTERM"),
        ];

        let int_passes = vec![
            IntPassInfo::v("BRKH", "INT_BRKH"),
        ];

        let int_bufs = vec![
            IntBufInfo::h("CLB_BUFFER", "INT_HBUF"),
            IntBufInfo::h_multi("PT", (12, 8), "INT_HPPC"),
            IntBufInfo::h_multi("PB", (12, 4), "INT_HPPC"),
        ];

        let int_dbufs = vec![
            IntDoubleBufInfo::v_multi("PB", "PT", (7, 0), "INT_VPPC"),
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
            int_bufs,
            int_dbufs,
            int_passes,
            int_pass_combine: HashMap::new(),
            tie_sites,
            tiles,
        };
        let mut res = Virtex4GeomMaker {
            builder: GeomBuilder::new(family.to_string(), cfg),
        };
        res.setup();
        res
    }

    fn setup_int_ll(&mut self) {
        // The long wires.
        let lh: Vec<_> = (0..25).map(|i| {
            let w = self.builder.geomdb.make_wire(&format!("INT.LH{}", i), "INT.LH", true);
            self.builder.register_int_wire(w, &[&format!("LH{}", i)]);
            w
        }).collect();
        for i in 0..12 {
            self.builder.connect_int_wire(lh[i], Dir::W, lh[i+1]);
            self.builder.connect_int_wire(lh[24-i], Dir::E, lh[24-i-1]);
        }

        let lv: Vec<_> = (0..25).map(|i| {
            let w = self.builder.geomdb.make_wire(&format!("INT.LV{}", i), "INT.LV", true);
            self.builder.register_int_wire(w, &[&format!("LV{}", i)]);
            w
        }).collect();
        for i in 0..12 {
            self.builder.connect_int_wire(lv[i], Dir::S, lv[i+1]);
            self.builder.connect_int_wire(lv[24-i], Dir::N, lv[24-i-1]);
        }
    }

    fn setup_int_hex(&mut self) {
        for (dir, d, cls) in [
            (Dir::W, "W", "INT.HHEX"),
            (Dir::E, "E", "INT.HHEX"),
            (Dir::S, "S", "INT.VHEX"),
            (Dir::N, "N", "INT.VHEX"),
        ].iter().copied() {
            for i in 0..10 {
                let mut last = self.builder.make_int_wire(
                    &format!("INT.{}6BEG{}", d, i),
                    cls,
                    &[&format!("{}6BEG{}", d, i)],
                );
                for seg in ["A", "B", "MID", "C", "D", "END"].iter().copied() {
                    let cur = self.builder.make_int_wire_cont(
                        &format!("INT.{}6{}{}", d, seg, i),
                        cls,
                        &[&format!("{}6{}{}", d, seg, i)],
                        dir,
                        last
                    );
                    last = cur;
                }
                match dir {
                    Dir::E | Dir::S => {
                        if i < 2 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}6END_S{}", d, i),
                                cls,
                                &[&format!("{}6END_S{}", d, i)],
                                Dir::S,
                                last
                            );
                        }
                    },
                    Dir::W | Dir::N => {
                        if i >= 8 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}6END_N{}", d, i),
                                cls,
                                &[&format!("{}6END_N{}", d, i)],
                                Dir::N,
                                last
                            );
                        }
                    },
                }
            }
        }
    }

    fn setup_int_dbl(&mut self) {
        for (dir, d, cls) in [
            (Dir::W, "W", "INT.HDBL"),
            (Dir::E, "E", "INT.HDBL"),
            (Dir::S, "S", "INT.VDBL"),
            (Dir::N, "N", "INT.VDBL"),
        ].iter().copied() {
            for i in 0..10 {
                let mut last = self.builder.make_int_wire(
                    &format!("INT.{}2BEG{}", d, i),
                    cls,
                    &[&format!("{}2BEG{}", d, i)],
                );
                for seg in ["MID", "END"].iter().copied() {
                    last = self.builder.make_int_wire_cont(
                        &format!("INT.{}2{}{}", d, seg, i),
                        cls,
                        &[&format!("{}2{}{}", d, seg, i)],
                        dir,
                        last
                    );
                }
                match dir {
                    Dir::E | Dir::S => {
                        if i < 2 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}2END_S{}", d, i),
                                cls,
                                &[&format!("{}2END_S{}", d, i)],
                                Dir::S,
                                last
                            );
                        }
                    },
                    Dir::W | Dir::N => {
                        if i >= 8 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}2END_N{}", d, i),
                                cls,
                                &[&format!("{}2END_N{}", d, i)],
                                Dir::N,
                                last
                            );
                        }
                    },
                }
            }
        }
    }

    fn setup_int_omux(&mut self) {
        for (i, dirs) in [
            (0, "S"),
            (1, "WS"),
            (2, "E:S"),
            (3, "SE"),
            (4, "S"),
            (5, "SW"),
            (6, "W"),
            (7, "ES"),
            (8, "EN"),
            (9, "W"),
            (10, "NW"),
            (11, "N"),
            (12, "NE"),
            (13, "E:N"),
            (14, "WN"),
            (15, "N"),
        ].iter().copied() {
            let base = self.builder.make_int_wire(&format!("INT.OMUX{}", i), "INT.OMUX", &[&format!("OMUX{}", i)]);
            let mut last = base;
            let mut suf = "";
            for c in dirs.chars() {
                let (dir, nsuf) = match c {
                    ':' => { last = base; suf = ""; continue; }
                    'W' => (Dir::W, "W"),
                    'E' => (Dir::E, "E"),
                    'S' => (Dir::S, "S"),
                    'N' => (Dir::N, "N"),
                    _ => unreachable!(),
                };
                last = self.builder.make_int_wire_cont(&format!("INT.OMUX{}.{}{}", i, suf, c), "INT.OMUX", &[&format!("OMUX_{}{}{}", suf, c, i)], dir, last);
                suf = nsuf;
            }
            if i == 0 {
                self.builder.make_int_wire_cont("INT.OMUX0.S.ALT", "INT.OMUX", &["OUT_S"], Dir::S, base);
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
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.SR{}", i), "INT.IMUX.SR", &[&format!("SR_B{}", i)]);
        }
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.BOUNCE{}", i), "INT.IMUX.BOUNCE", &[&format!("BOUNCE{}", i)]);
        }
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.CLK{}", i), "INT.IMUX.CLK", &[
                &format!("CLK_B{}", i),
                &format!("CLK_B{}_DCM", i),
            ]);
        }
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.CE{}", i), "INT.IMUX.CE", &[&format!("CE_B{}", i)]);
        }

        // The data inputs.
        for i in 0..8 {
            self.builder.make_int_wire(&format!("INT.IMUX.BYP{}", i), "INT.IMUX.BYP", &[&format!("BYP_INT_B{}", i)]);
        }
        for i in 0..8 {
            self.builder.make_int_wire(&format!("INT.IMUX.BYP{}.BOUNCE", i), "INT.IMUX.BYP_BOUNCE", &[&format!("BYP_BOUNCE{}", i)]);
        }

        for i in 0..32 {
            self.builder.make_int_wire(&format!("INT.IMUX.IMUX{}", i), "INT.IMUX.IMUX", &[&format!("IMUX_B{}", i)]);
        }
    }

    fn setup_int_out(&mut self) {
        for i in 0..8 {
            self.builder.make_int_out_wire(&format!("INT.OUT.BEST{}", i), "INT.OUT.BEST", &[
                &format!("BEST_LOGIC_OUTS{}", i),
            ]);
        }
        for i in 0..8 {
            self.builder.make_int_out_wire(&format!("INT.OUT.SEC{}", i), "INT.OUT.SEC", &[
                &format!("SECONDARY_LOGIC_OUTS{}", i),
            ]);
        }
        for i in 0..8 {
            self.builder.make_int_out_wire(&format!("INT.OUT.HALF_BOT{}", i), "INT.OUT.HALF", &[
                &format!("HALF_OMUX_BOT{}", i),
            ]);
        }
        for i in 0..8 {
            self.builder.make_int_out_wire(&format!("INT.OUT.HALF_TOP{}", i), "INT.OUT.HALF", &[
                &format!("HALF_OMUX_TOP{}", i),
            ]);
        }
    }

    fn setup_int(&mut self) {
        self.builder.setup_int();

        self.setup_int_ll();
        self.setup_int_hex();
        self.setup_int_dbl();
        self.setup_int_omux();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();
    }

    fn setup_buses_clk(&mut self) {
        // XXX spine
        // XXX hrow

        // Final clock distribution tile: from HROW bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..8 {
            let w = self.builder.geomdb.make_vbus_wire(&format!("HCLK.GCLK{}", i), "HCLK.CLK", vbus_hclk, false);
            self.builder.register_int_wire(w, &[&format!("GCLK{}", i)]);
        }
        for i in 0..2 {
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
        if part.starts_with("xc4v") || part.starts_with("xq4v") {
            part[2..].to_string()
        } else if part.starts_with("xqr4v") {
            part[3..].to_string()
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

impl RdGeomMakerImpl for Virtex4GeomMaker {
    fn get_family(&self) -> &str {
        &self.builder.geomdb.name
    }
    fn ingest(&mut self, rd: &rawdump::Part) {
        let grid_name = Self::make_grid_name(&rd.part);
        let mut grid = PartBuilder::new(grid_name, rd, &self.builder);
        self.fill_grid(&mut grid);
        self.verify(&grid);
        self.builder.ingest(grid);
    }
    fn finish(self:Box<Self>) -> (GeomDb, GeomRaw) {
        self.builder.finish()
    }
}

