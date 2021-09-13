use super::builder::GeomBuilder;
use super::cfg::{GeomBuilderConfig, IntTermInfo, IntTileInfo, TieSiteInfo, TileAnchor};
use super::part::PartBuilder;
use super::RdGeomMakerImpl;
use crate::xilinx::geomdb::builder::{GeomDbBuilder, GridBuilder};
use crate::xilinx::geomdb::{Dir, GeomDb, TieState};
use crate::xilinx::geomraw::GeomRaw;
use prjcombine_xilinx_rawdump as rawdump;
use std::collections::{HashMap, HashSet};

pub struct Series7GeomMaker {
    builder: GeomBuilder,
    pcls_int_clk: Option<(usize, usize)>,
    pcls_int_slv: Option<(usize, usize)>,
}

impl Series7GeomMaker {
    pub fn new(family: &str) -> Self {
        let int_tiles = vec![
            IntTileInfo::int("INT_L", "INT_L"),
            IntTileInfo::int("INT_R", "INT_R"),
            IntTileInfo::int("INT_L_SLV", "INT_L"),
            IntTileInfo::int("INT_R_SLV", "INT_R"),
            IntTileInfo::int("INT_L_SLV_FLY", "INT_L"),
            IntTileInfo::int("INT_R_SLV_FLY", "INT_R"),
        ];

        let int_terms = vec![
            IntTermInfo::b("B_TERM_INT", "INT_BTERM"),
            IntTermInfo::b("B_TERM_INT_SLV", "INT_BTERM"),
            IntTermInfo::b("BRKH_B_TERM_INT", "INT_BTERM"),
            IntTermInfo::b("HCLK_L_BOT_UTURN", "INT_BTERM"),
            IntTermInfo::b("HCLK_R_BOT_UTURN", "INT_BTERM"),
            IntTermInfo::t("T_TERM_INT", "INT_TTERM"),
            IntTermInfo::t("T_TERM_INT_SLV", "INT_TTERM"),
            IntTermInfo::t("BRKH_TERM_INT", "INT_TTERM"),
            IntTermInfo::t("BRKH_INT_PSS", "INT_TTERM"),
            IntTermInfo::t("HCLK_L_TOP_UTURN", "INT_TTERM"),
            IntTermInfo::t("HCLK_R_TOP_UTURN", "INT_TTERM"),
            IntTermInfo::l("L_TERM_INT", "INT_LTERM"),
            IntTermInfo::l("L_TERM_INT_BRAM", "INT_LTERM"),
            IntTermInfo::l("INT_INTERFACE_PSS_L", "INT_LTERM"),
            IntTermInfo::l("GTP_INT_INTERFACE_L", "INT_LTERM"),
            IntTermInfo::l("GTP_INT_INT_TERM_L", "INT_LTERM"),
            IntTermInfo::r("R_TERM_INT", "INT_RTERM"),
            IntTermInfo::r("R_TERM_INT_GTX", "INT_RTERM"),
            IntTermInfo::r("GTP_INT_INTERFACE_R", "INT_RTERM"),
            IntTermInfo::r("GTP_INT_INT_TERM_R", "INT_RTERM"),
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
            extra_cell_col_injectors: [
                ("CFG_CENTER_BOT", &[-10, -9, -6, -5, -2, -1][..]),
                (
                    "PSS0",
                    &[
                        -28, -27, -21, -20, -17, -16, -11, -10, -7, -6, -1, 0, 3, 4, 9, 10, 13, 14,
                    ][..],
                ),
            ]
            .iter()
            .copied()
            .collect(),
            int_terms,
            int_bufs: Vec::new(),
            int_dbufs: Vec::new(),
            int_passes: Vec::new(),
            int_pass_combine: HashMap::new(),
            tie_sites,
            tiles,
        };
        let mut res = Series7GeomMaker {
            builder: GeomBuilder::new(family.to_string(), cfg),
            pcls_int_slv: None,
            pcls_int_clk: None,
        };
        res.setup();
        res
    }

    fn setup_int_ll(&mut self) {
        // The long wires.
        let lh: Vec<_> = (0..13)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LH{}", i), "INT.LH", true);
                self.builder.register_int_wire(w, &[&format!("LH{}", i)]);
                w
            })
            .collect();
        for i in 0..6 {
            self.builder.connect_int_wire(lh[i], Dir::W, lh[i + 1]);
            self.builder
                .connect_int_wire(lh[12 - i], Dir::E, lh[12 - i - 1]);
        }

        let lv: Vec<_> = (0..19)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LV{}", i), "INT.LV", true);
                self.builder
                    .register_int_wire(w, &[&format!("LV{}", i), &format!("LV_L{}", i)]);
                w
            })
            .collect();
        for i in 0..9 {
            self.builder.connect_int_wire(lv[i], Dir::N, lv[i + 1]);
            self.builder
                .connect_int_wire(lv[18 - i], Dir::S, lv[18 - i - 1]);
        }

        let lvb: Vec<_> = (0..13)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LVB{}", i), "INT.LVB", true);
                self.builder
                    .register_int_wire(w, &[&format!("LVB{}", i), &format!("LVB_L{}", i)]);
                w
            })
            .collect();
        for i in 0..6 {
            self.builder.connect_int_wire(lvb[i], Dir::N, lvb[i + 1]);
            self.builder
                .connect_int_wire(lvb[12 - i], Dir::S, lvb[12 - i - 1]);
        }
        self.builder
            .register_int_wire(lvb[6], &["LVB_L6_SLV", "LVB6_SLV"]);

        // The SLV port.
        let pslot_int_slv_n = self.builder.geomdb.make_port_slot("INT_SLV_N");
        let pslot_int_slv_s = self.builder.geomdb.make_port_slot("INT_SLV_S");
        let (pcls_int_slv_n, pcls_int_slv_s) = self.builder.geomdb.make_port_pair(
            ("INT_SLV_N", "INT_SLV_S"),
            (pslot_int_slv_n, pslot_int_slv_s),
        );
        self.pcls_int_slv = Some((pcls_int_slv_n, pcls_int_slv_s));

        self.builder
            .geomdb
            .make_simple_pconn(lvb[6], lvb[6], pcls_int_slv_n, pcls_int_slv_s);

        self.builder.mark_pcls_filled(pcls_int_slv_n);
        self.builder.mark_pcls_filled(pcls_int_slv_s);
    }

    fn setup_int_hex(&mut self) {
        for (name, da, db, dend) in [
            ("NN6", Dir::N, Dir::N, Some((Dir::S, 1))),
            ("NE6", Dir::N, Dir::E, None),
            ("NW6", Dir::N, Dir::W, Some((Dir::S, 0))),
            ("SS6", Dir::S, Dir::S, Some((Dir::N, 0))),
            ("SE6", Dir::S, Dir::E, None),
            ("SW6", Dir::S, Dir::W, Some((Dir::N, 0))),
        ]
        .iter()
        .copied()
        {
            for i in 0..4 {
                let beg = self.builder.make_int_wire(
                    &format!("INT.{}BEG{}", name, i),
                    "INT.HEX",
                    &[&format!("{}BEG{}", name, i)],
                );
                let a = self.builder.make_int_wire(
                    &format!("INT.{}A{}", name, i),
                    "INT.HEX",
                    &[&format!("{}A{}", name, i)],
                );
                let b = self.builder.make_int_wire(
                    &format!("INT.{}B{}", name, i),
                    "INT.HEX",
                    &[&format!("{}B{}", name, i)],
                );
                let c = self.builder.make_int_wire(
                    &format!("INT.{}C{}", name, i),
                    "INT.HEX",
                    &[&format!("{}C{}", name, i)],
                );
                let d = self.builder.make_int_wire(
                    &format!("INT.{}D{}", name, i),
                    "INT.HEX",
                    &[&format!("{}D{}", name, i)],
                );
                let e = self.builder.make_int_wire(
                    &format!("INT.{}E{}", name, i),
                    "INT.HEX",
                    &[&format!("{}E{}", name, i)],
                );
                let end = self.builder.make_int_wire(
                    &format!("INT.{}END{}", name, i),
                    "INT.HEX",
                    &[&format!("{}END{}", name, i)],
                );
                self.builder.connect_int_wire(a, db, beg);
                self.builder.connect_int_wire(b, da, a);
                self.builder.connect_int_wire(c, da, b);
                self.builder.connect_int_wire(d, da, c);
                self.builder.connect_int_wire(e, da, d);
                self.builder.connect_int_wire(end, db, e);
                if i == 0 {
                    if let Some((Dir::S, x)) = dend {
                        let end_s = self.builder.make_int_wire(
                            &format!("INT.{}END_S{}", name, i),
                            "INT.HEX",
                            &[&format!("{}END_S{}_{}", name, x, i)],
                        );
                        self.builder.connect_int_wire(end_s, Dir::S, end);
                    }
                }
                if i == 3 {
                    if let Some((Dir::N, x)) = dend {
                        let end_n = self.builder.make_int_wire(
                            &format!("INT.{}END_N{}", name, i),
                            "INT.HEX",
                            &[&format!("{}END_N{}_{}", name, x, i)],
                        );
                        self.builder.connect_int_wire(end_n, Dir::N, end);
                    }
                }
            }
        }
    }

    fn setup_int_quad(&mut self) {
        for (name, dir, dend) in [("EE4", Dir::E, None), ("WW4", Dir::W, Some((Dir::S, 0)))]
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
                self.builder.connect_int_wire(a, dir, beg);
                self.builder.connect_int_wire(b, dir, a);
                self.builder.connect_int_wire(c, dir, b);
                self.builder.connect_int_wire(end, dir, c);
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
                &[&format!("CLK{}", i), &format!("CLK_L{}", i)],
            );
        }
        for i in 0..2 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.CTRL{}", i),
                "INT.IMUX.CTRL",
                &[&format!("CTRL{}", i), &format!("CTRL_L{}", i)],
            );
        }
        // XXX
        for i in 0..8 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.BYP{}", i),
                "INT.IMUX.BYP",
                &[&format!("BYP_ALT{}", i)],
            );
            self.builder.make_int_wire(
                &format!("INT.IMUX.BYP{}.SITE", i),
                "INT.IMUX.BYP_SITE",
                &[&format!("BYP{}", i), &format!("BYP_L{}", i)],
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
                &[&format!("FAN_ALT{}", i)],
            );
            self.builder.make_int_wire(
                &format!("INT.IMUX.FAN{}.SITE", i),
                "INT.IMUX.FAN_SITE",
                &[&format!("FAN{}", i), &format!("FAN_L{}", i)],
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
                &[&format!("IMUX{}", i), &format!("IMUX_L{}", i)],
            );
        }
    }

    fn setup_int_out(&mut self) {
        for i in 0..24 {
            self.builder.make_int_out_wire(
                &format!("INT.OUT{}", i),
                "INT.OUT",
                &[&format!("LOGIC_OUTS{}", i), &format!("LOGIC_OUTS_L{}", i)],
            );
        }
    }

    fn setup_int_clk(&mut self) {
        let pslot_int_clk_e = self.builder.geomdb.make_port_slot("INT_CLK_E");
        let pslot_int_clk_w = self.builder.geomdb.make_port_slot("INT_CLK_W");
        let (pcls_int_clk_e, pcls_int_clk_w) = self.builder.geomdb.make_port_pair(
            ("INT_CLK_E", "INT_CLK_W"),
            (pslot_int_clk_e, pslot_int_clk_w),
        );
        self.pcls_int_clk = Some((pcls_int_clk_e, pcls_int_clk_w));

        for i in 0..6 {
            let w = self.builder.make_int_wire(
                &format!("HCLK.CLK{}.INT", i),
                "HCLK.CLK.INT",
                &[&format!("GCLK_L_B{}", i), &format!("GCLK_B{}_EAST", i)],
            );
            let w2 = self.builder.make_int_wire(
                &format!("HCLK.CLK{}.INT.OTHER", i),
                "HCLK.CLK.INT",
                &[&format!("GCLK_B{}_WEST", i)],
            );
            self.builder
                .geomdb
                .make_simple_pconn(w, w2, pcls_int_clk_w, pcls_int_clk_e);
        }
        for i in 6..12 {
            let w = self.builder.make_int_wire(
                &format!("HCLK.CLK{}.INT", i),
                "HCLK.CLK.INT",
                &[&format!("GCLK_L_B{}_WEST", i), &format!("GCLK_B{}", i)],
            );
            let w2 = self.builder.make_int_wire(
                &format!("HCLK.CLK{}.INT.OTHER", i),
                "HCLK.CLK.INT",
                &[&format!("GCLK_B{}_EAST", i)],
            );
            self.builder
                .geomdb
                .make_simple_pconn(w, w2, pcls_int_clk_e, pcls_int_clk_w);
        }

        self.builder.mark_pcls_filled(pcls_int_clk_e);
        self.builder.mark_pcls_filled(pcls_int_clk_w);
    }

    fn setup_int(&mut self) {
        self.builder.setup_int();

        self.setup_int_ll();
        self.setup_int_hex();
        self.setup_int_quad();
        self.setup_int_dbl();
        self.setup_int_single();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();
        self.setup_int_clk();
    }

    fn setup_buses_clk(&mut self) {
        // XXX

        // Final clock distribution tile: from HROW bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..6 {
            let w = self.builder.geomdb.make_vbus_wire(
                &format!("HCLK.CLK{}", i),
                "HCLK.CLK",
                vbus_hclk,
                false,
            );
            self.builder
                .register_int_wire(w, &[&format!("GCLK_B{}", i)]);
        }
        for i in 6..12 {
            let w = self.builder.geomdb.make_vbus_wire(
                &format!("HCLK.CLK{}", i),
                "HCLK.CLK",
                vbus_hclk,
                false,
            );
            self.builder
                .register_int_wire(w, &[&format!("GCLK_L_B{}", i)]);
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
        let sp = if part.starts_with("xc7") || part.starts_with("xq7") || part.starts_with("xa7") {
            if part.ends_with("_CIV") {
                &part[2..part.len() - 4]
            } else if part.ends_with('l') || part.ends_with('i') {
                &part[2..part.len() - 1]
            } else {
                &part[2..]
            }
        } else {
            panic!("unregognized part name {}", part);
        };
        match sp {
            "7s25" => "7a25t",
            "7a12t" => "7a25t",
            "7s50" => "7a50t",
            "7a15t" => "7a50t",
            "7a35t" => "7a50t",
            "7a75t" => "7a100t",
            "7s6" => "7s15",
            "7s75" => "7s100",
            "7k420t" => "7k480t",
            "7vx550t" => "7vx690t",
            "7z007s" => "7z010",
            "7z012s" => "7z015",
            "7z014s" => "7z020",
            "7z035" => "7z045",
            _ => sp,
        }
        .to_string()
    }

    fn curse_tiles(&mut self, grid: &mut PartBuilder) {
        for x in 0..grid.rd.width {
            let crd_a = rawdump::Coord { x, y: 1 };
            let tile_a = &grid.rd.tiles[&crd_a];
            if tile_a.kind == "T_TERM_INT" || tile_a.kind == "T_TERM_INT_SLV" {
                grid.curse_tile(crd_a);
            }
            let crd_b = rawdump::Coord {
                x,
                y: grid.rd.height - 2,
            };
            let tile_b = &grid.rd.tiles[&crd_b];
            if tile_b.kind == "B_TERM_INT" || tile_b.kind == "B_TERM_INT_SLV" {
                grid.curse_tile(crd_b);
            }
        }
        for x in 0..grid.rd.width {
            for y in 0..grid.rd.height {
                let crd_a = rawdump::Coord { x, y };
                let tile_a = &grid.rd.tiles[&crd_a];
                if tile_a.kind == "BRKH_INT_PSS" {
                    let crd_b = rawdump::Coord { x, y: y + 104 };
                    let tile_b = &grid.rd.tiles[&crd_b];
                    assert_eq!(tile_b.kind, "T_TERM_INT");
                    grid.curse_tile(crd_b);
                }
            }
        }
    }

    fn fill_grid_int_clk(&mut self, part: &mut PartBuilder) {
        // INT GCLK connections special.
        let xy: Vec<(usize, usize)> = part
            .find_anchors(&TileAnchor::int(&["INT_L", "INT_L_SLV", "INT_L_SLV_FLY"]))
            .into_iter()
            .map(|(_, xy)| xy)
            .collect();
        for (x, y) in xy {
            let ox = x + 1;
            part.grid.fill_port_pair(
                &self.builder.geomdb,
                (x, y),
                (ox, y),
                self.pcls_int_clk.unwrap(),
            );
        }
    }

    fn fill_grid_int_slv(&mut self, part: &mut PartBuilder) {
        // INT SLV connections special.
        let xy: HashSet<(usize, usize)> = part
            .find_anchors(&TileAnchor::int(&["INT_L_SLV_FLY", "INT_R_SLV_FLY"]))
            .into_iter()
            .map(|(_, xy)| xy)
            .collect();
        for (x, y) in xy.iter().copied() {
            if !xy.contains(&(x, y + 1)) {
                continue;
            }
            let ys = y - 49;
            let yn = y + 2;
            assert_eq!(ys % 50, 0);
            for i in 0..49 {
                part.grid.fill_port_pair(
                    &self.builder.geomdb,
                    (x, ys + i),
                    (x, yn + i),
                    self.pcls_int_slv.unwrap(),
                );
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
        self.fill_grid_int_clk(part);
        self.fill_grid_int_slv(part);
        self.fill_grid_bus(part);
        self.fill_grid_site_conns(part);
        part.fill_tiles(&self.builder);
        self.fill_grid_cols(part);
    }

    fn verify(&self, _part: &PartBuilder) {
        // XXX
    }
}

impl RdGeomMakerImpl for Series7GeomMaker {
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
