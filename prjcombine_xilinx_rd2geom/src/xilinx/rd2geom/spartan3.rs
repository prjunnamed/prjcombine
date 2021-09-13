use super::builder::GeomBuilder;
use super::cfg::{
    GeomBuilderConfig, IntBufInfo, IntDoubleBufInfo, IntPassInfo, IntTermInfo, IntTileInfo,
    TieSiteInfo, TileAnchor, TileInfo,
};
use super::part::PartBuilder;
use super::RdGeomMakerImpl;
use crate::xilinx::geomdb::builder::GeomDbBuilder;
use crate::xilinx::geomdb::{Dir, GeomDb, TieState};
use crate::xilinx::geomraw::GeomRaw;
use prjcombine_xilinx_rawdump as rawdump;
use std::collections::HashMap;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Family {
    S3,
    S3E,
    S3A,
    S3ADSP,
}

pub struct Spartan3GeomMaker {
    family: Family,
    builder: GeomBuilder,
}

impl Spartan3GeomMaker {
    pub fn new(fam: &str) -> Self {
        let family = match fam {
            "spartan3" => Family::S3,
            "spartan3e" => Family::S3E,
            "spartan3a" => Family::S3A,
            "spartan3adsp" => Family::S3ADSP,
            _ => panic!("unknown family {}", fam),
        };
        let tie_sites = vec![TieSiteInfo {
            kind: "VCC",
            pins: &[("VCCOUT", TieState::S1)],
        }];
        let mut int_tiles = vec![];
        let mut tiles = vec![];
        if family == Family::S3 {
            int_tiles.append(&mut vec![
                IntTileInfo::int("CENTER", "INT_CENTER"),
                IntTileInfo::int("CENTER_SMALL", "INT_CENTER_SMALL"),
                IntTileInfo::int("BRAM0", "INT_BRAM"),
                IntTileInfo::int("BRAM1", "INT_BRAM"),
                IntTileInfo::int("BRAM2", "INT_BRAM"),
                IntTileInfo::int("BRAM3", "INT_BRAM"),
                IntTileInfo::int("BRAM0_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM1_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM2_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM3_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM_IOIS", "INT_DCM"),
                IntTileInfo::int("BRAM_IOIS_NODCM", "INT_NODCM"),
            ]);
            tiles.append(&mut vec![
                TileInfo::site_int("SITE_CLB", &["CENTER", "CENTER_SMALL"]),
                TileInfo::site_vert_r("SITE_BRAM", &["BRAMSITE"], (4, 0)),
                TileInfo::site_int_extra("SITE_DCM_B", &["BRAM_IOIS"], vec![(0, -1, &["BBTERM"])]),
                TileInfo::site_int_extra("SITE_DCM_T", &["BRAM_IOIS"], vec![(0, 1, &["BTTERM"])]),
                // XXX
            ]);
        } else {
            int_tiles.append(&mut vec![
                IntTileInfo::int("CENTER_SMALL", "INT_CENTER_SMALL"),
                IntTileInfo::int("CENTER_SMALL_BRK", "INT_CENTER_SMALL"),
                IntTileInfo::int("BRAM0_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM1_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM2_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM3_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM3_SMALL_BRK", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM3_SMALL_TOP", "INT_BRAM_SMALL"),
                IntTileInfo::int("BRAM0_SMALL_BOT", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC0_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC1_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC2_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC3_SMALL", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC3_SMALL_BRK", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC3_SMALL_TOP", "INT_BRAM_SMALL"),
                IntTileInfo::int("MACC0_SMALL_BOT", "INT_BRAM_SMALL"),
            ]);
            tiles.append(&mut vec![TileInfo::site_int(
                "SITE_CLB",
                &["CENTER_SMALL", "CENTER_SMALL_BRK"],
            )]);
            if family == Family::S3ADSP {
                tiles.append(&mut vec![
                    TileInfo::site_rect(
                        "SITE_BRAM",
                        &[
                            "BRAMSITE2_3M",
                            "BRAMSITE2_3M_BRK",
                            "BRAMSITE2_3M_BOT",
                            "BRAMSITE2_3M_TOP",
                        ],
                        (1, 4),
                        (-1, 0),
                    ),
                    TileInfo::site_vert_r(
                        "SITE_DSP",
                        &[
                            "MACCSITE2",
                            "MACCSITE2_BRK",
                            "MACCSITE2_BOT",
                            "MACCSITE2_TOP",
                        ],
                        (4, 0),
                    ),
                ]);
            } else {
                tiles.append(&mut vec![TileInfo::site_rect(
                    "SITE_BRAM",
                    &[
                        "BRAMSITE2",
                        "BRAMSITE2_BRK",
                        "BRAMSITE2_BOT",
                        "BRAMSITE2_TOP",
                    ],
                    (1, 4),
                    (-1, 0),
                )]);
            }
            if family == Family::S3E {
                int_tiles.append(&mut vec![
                    IntTileInfo::int("DCMAUX_BL_CENTER", "INT_DCMAUX"),
                    IntTileInfo::int("DCMAUX_TL_CENTER", "INT_DCMAUX"),
                    IntTileInfo::int("DCM_BL_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_TL_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_BR_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_TR_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_H_BL_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_H_TL_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_H_BR_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_H_TR_CENTER", "INT_DCM"),
                ]);
                // XXX SITE
            } else {
                int_tiles.append(&mut vec![
                    IntTileInfo::int("DCM_BL_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_TL_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_BR_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_TR_CENTER", "INT_DCM"),
                    IntTileInfo::int("DCM_BGAP", "INT_DCM"),
                    IntTileInfo::int("DCM_SPLY", "INT_DCM"),
                ]);
                // XXX SITE
            }
        }

        if family == Family::S3 {
            int_tiles.append(&mut vec![
                IntTileInfo::int("LIOIS", "INT_IOI"),
                IntTileInfo::int("RIOIS", "INT_IOI"),
                IntTileInfo::int("BIOIS", "INT_IOI"),
                IntTileInfo::int("TIOIS", "INT_TIOI"),
            ]);
            // XXX IOI
            // XXX IOB
        } else if family == Family::S3E {
            int_tiles.append(&mut vec![
                IntTileInfo::int("LIOIS", "INT_IOI"),
                IntTileInfo::int("LIOIS_PCI", "INT_IOI"),
                IntTileInfo::int("LIOIS_CLK_PCI", "INT_IOI"),
                IntTileInfo::int("LIOIS_BRK", "INT_IOI"),
                IntTileInfo::int("LIBUFS", "INT_IOI"),
                IntTileInfo::int("LIBUFS_PCI", "INT_IOI"),
                IntTileInfo::int("LIBUFS_CLK_PCI", "INT_IOI"),
                IntTileInfo::int("RIOIS", "INT_IOI"),
                IntTileInfo::int("RIOIS_PCI", "INT_IOI"),
                IntTileInfo::int("RIOIS_CLK_PCI", "INT_IOI"),
                IntTileInfo::int("RIBUFS", "INT_IOI"),
                IntTileInfo::int("RIBUFS_PCI", "INT_IOI"),
                IntTileInfo::int("RIBUFS_CLK_PCI", "INT_IOI"),
                IntTileInfo::int("RIBUFS_BRK", "INT_IOI"),
                IntTileInfo::int("BIOIS", "INT_IOI"),
                IntTileInfo::int("BIBUFS", "INT_IOI"),
                IntTileInfo::int("TIOIS", "INT_TIOI"),
                IntTileInfo::int("TIBUFS", "INT_TIOI"),
            ]);
            // XXX IOI
            // XXX IOB
        } else {
            int_tiles.append(&mut vec![
                IntTileInfo::int("LIOIS", "INT_IOI2"),
                IntTileInfo::int("LIOIS_PCI", "INT_IOI2"),
                IntTileInfo::int("LIOIS_CLK_PCI", "INT_IOI2"),
                IntTileInfo::int("LIOIS_BRK", "INT_IOI2"),
                IntTileInfo::int("LIOIS_CLK_PCI_BRK", "INT_IOI2"),
                IntTileInfo::int("LIBUFS", "INT_IOI2"),
                IntTileInfo::int("LIBUFS_PCI", "INT_IOI2"),
                IntTileInfo::int("LIBUFS_CLK_PCI", "INT_IOI2"),
                IntTileInfo::int("RIOIS", "INT_IOI2"),
                IntTileInfo::int("RIOIS_PCI", "INT_IOI2"),
                IntTileInfo::int("RIOIS_CLK_PCI", "INT_IOI2"),
                IntTileInfo::int("RIBUFS", "INT_IOI2"),
                IntTileInfo::int("RIBUFS_PCI", "INT_IOI2"),
                IntTileInfo::int("RIBUFS_CLK_PCI", "INT_IOI2"),
                IntTileInfo::int("RIBUFS_BRK", "INT_IOI2"),
                IntTileInfo::int("RIBUFS_CLK_PCI_BRK", "INT_IOI2"),
                IntTileInfo::int("BIOIS", "INT_IOI"),
                IntTileInfo::int("BIOIB", "INT_IOI"),
                IntTileInfo::int("TIOIS", "INT_TIOI"),
                IntTileInfo::int("TIOIB", "INT_TIOI"),
            ]);
            // XXX IOI
            // XXX IOB
        }

        int_tiles.append(&mut vec![
            IntTileInfo::int("LL", "INT_CNR"),
            IntTileInfo::int("LR", "INT_CNR"),
            IntTileInfo::int("UL", "INT_CNR"),
            IntTileInfo::int("UR", "INT_CNR"),
        ]);
        tiles.append(&mut vec![
            TileInfo::site_int("SITE_CNR_LL", &["LL"]),
            TileInfo::site_int("SITE_CNR_LR", &["LR"]),
            TileInfo::site_int("SITE_CNR_UL", &["UL"]),
            TileInfo::site_int("SITE_CNR_UR", &["UR"]),
        ]);

        let mut int_terms = Vec::new();
        for t in &[
            "LTERM",
            "LTERMCLK",
            "LTERMCLKA",
            "LTERM1",
            "LTERM2",
            "LTERM3",
            "LTERM4",
            "LTERM4B",
            "LTERM4CLK",
            "CNR_LBTERM",
            "CNR_LTTERM",
        ] {
            int_terms.push(IntTermInfo::l(t, "INT_LTERM"));
        }
        for t in &[
            "RTERM",
            "RTERMCLKA",
            "RTERMCLKB",
            "RTERM1",
            "RTERM2",
            "RTERM3",
            "RTERM4",
            "RTERM4CLK",
            "RTERM4CLKB",
            "CNR_RBTERM",
            "CNR_RTTERM",
        ] {
            int_terms.push(IntTermInfo::r(t, "INT_RTERM"));
        }
        for t in &[
            "TTERM",
            "TTERM1",
            "TTERM1_MACC",
            "TTERM2",
            "TTERM2CLK",
            "TTERM3",
            "TTERM4",
            "TTERM4CLK",
            "TTERM4_BRAM2",
            "TTERMCLK",
            "TTERMCLKA",
            "TCLKTERM2",
            "TCLKTERM3",
            "CNR_TTERM",
            "BTTERM",
        ] {
            int_terms.push(IntTermInfo::t(t, "INT_TTERM"));
        }
        for t in &[
            "BTERM",
            "BTERM1",
            "BTERM1_MACC",
            "BTERM2",
            "BTERM2CLK",
            "BTERM3",
            "BTERM4",
            "BTERM4CLK",
            "BTERM4_BRAM2",
            "BTERMCLK",
            "BTERMCLKA",
            "BTERMCLKB",
            "BCLKTERM2",
            "BCLKTERM3",
            "CNR_BTERM",
            "BBTERM",
        ] {
            int_terms.push(IntTermInfo::b(t, "INT_BTERM"));
        }

        let mut int_passes = Vec::new();
        match family {
            Family::S3E => {
                // Those are missing some wires on 3e.
                int_passes.append(&mut vec![
                    IntPassInfo::v("CLKL_IOIS", "INT_CLKLR"),
                    IntPassInfo::v("CLKR_IOIS", "INT_CLKLR"),
                ]);
            }
            Family::S3A | Family::S3ADSP => {
                int_passes.append(&mut vec![
                    IntPassInfo::v_empty("BRAMSITE2"),
                    IntPassInfo::v_empty("BRAMSITE2_3M"),
                    IntPassInfo::v_empty("BRAM2_FEEDTHRU"),
                ]);
            }
            _ => (),
        }
        if family == Family::S3ADSP {
            int_passes.append(&mut vec![
                IntPassInfo::h("DCM_TERM_NOMEM", "INT_HDCM"),
                IntPassInfo::h("EMPTY_DCM_TERM", "INT_HDSPHOLE"),
                IntPassInfo::h("EMPTY_TIOI", "INT_HDSPHOLE"),
                IntPassInfo::h("EMPTY_BIOI", "INT_HDSPHOLE"),
            ]);
        }
        let mut int_bufs = Vec::new();
        if family != Family::S3 {
            int_bufs.append(&mut vec![
                IntBufInfo::h_fat("CLKV_LL", "INT_HLL"),
                IntBufInfo::h_fat("CLKV_DCM_LL", "INT_HLL"),
                IntBufInfo::h_fat("CLKB_LL", "INT_HLL"),
                IntBufInfo::h_fat("CLKT_LL", "INT_HLL"),
                IntBufInfo::v_fat("CLKH_LL", "INT_VLL"),
                IntBufInfo::v_fat("CLKH_DCM_LL", "INT_VLL"),
                IntBufInfo::v_fat("CLKLH_DCM_LL", "INT_VLL"),
                IntBufInfo::v_fat("CLKRH_DCM_LL", "INT_VLL"),
                IntBufInfo::v_fat("CLKL_IOIS_LL", "INT_VLL_CLKLR"),
                IntBufInfo::v_fat("CLKR_IOIS_LL", "INT_VLL_CLKLR"),
            ]);
        }
        let mut int_dbufs = Vec::new();
        if family == Family::S3E {
            int_dbufs.append(&mut vec![IntDoubleBufInfo::v(
                "COB_TERM_B",
                "COB_TERM_T",
                "INT_COB",
            )]);
        }

        let mut int_pass_combine = HashMap::new();
        if family == Family::S3ADSP {
            int_pass_combine.insert(
                ["INT_HDCM", "INT_HDSPHOLE"].iter().copied().collect(),
                "INT_HDCMDSP",
            );
        }

        let cfg = GeomBuilderConfig {
            int_tiles,
            extra_cell_col_injectors: HashMap::new(),
            int_terms,
            int_bufs,
            int_dbufs,
            int_passes,
            int_pass_combine,
            tie_sites,
            tiles,
        };
        let mut res = Spartan3GeomMaker {
            family,
            builder: GeomBuilder::new(fam.to_string(), cfg),
        };
        res.setup();
        res
    }

    fn setup_int_ll(&mut self) {
        // The long wires.
        let lh: Vec<_> = (0..24)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LH{}", i), "INT.LH", true);
                self.builder.register_int_wire(w, &[&format!("LH{}", i)]);
                w
            })
            .collect();
        for i in 0..24 {
            let d = lh[i];
            let u = if i == 0 { lh[23] } else { lh[i - 1] };
            self.builder.connect_int_wire(d, Dir::E, u);
        }

        let lv: Vec<_> = (0..24)
            .map(|i| {
                let w = self
                    .builder
                    .geomdb
                    .make_wire(&format!("INT.LV{}", i), "INT.LV", true);
                self.builder.register_int_wire(w, &[&format!("LV{}", i)]);
                w
            })
            .collect();
        for i in 0..24 {
            let d = lv[i];
            let u = if i == 23 { lv[0] } else { lv[i + 1] };
            self.builder.connect_int_wire(d, Dir::N, u);
        }
    }

    fn setup_int_hex(&mut self) {
        for (dir, d, cls) in [
            (Dir::W, "W", "INT.HHEX"),
            (Dir::E, "E", "INT.HHEX"),
            (Dir::S, "S", "INT.VHEX"),
            (Dir::N, "N", "INT.VHEX"),
        ]
        .iter()
        .copied()
        {
            for i in 0..8 {
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
                        last,
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
                                last,
                            );
                        }
                    }
                    Dir::W | Dir::N => {
                        if i >= 6 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}6END_N{}", d, i),
                                cls,
                                &[&format!("{}6END_N{}", d, i)],
                                Dir::N,
                                last,
                            );
                        }
                    }
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
        ]
        .iter()
        .copied()
        {
            for i in 0..8 {
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
                        last,
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
                                last,
                            );
                        }
                    }
                    Dir::W | Dir::N => {
                        if i >= 6 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}2END_N{}", d, i),
                                cls,
                                &[&format!("{}2END_N{}", d, i)],
                                Dir::N,
                                last,
                            );
                        }
                    }
                }
            }
        }
    }

    fn setup_int_omux(&mut self) {
        for (i, dirs) in [
            (0, "S"),
            (1, "WS"),
            (2, "E"), // also SE_S
            (3, "SE"),
            (4, "S"),
            (5, "SW"),
            (6, "W"),
            (7, "ES"),
            (8, "EN"),
            (9, "W"), // also NW_N
            (10, "NW"),
            (11, "N"),
            (12, "NE"),
            (13, "E"),
            (14, "WN"),
            (15, "N"),
        ]
        .iter()
        .copied()
        {
            let base = self.builder.make_int_wire(
                &format!("INT.OMUX{}", i),
                "INT.OMUX",
                &[&format!("OMUX{}", i)],
            );
            let mut last = base;
            let mut suf = "";
            for c in dirs.chars() {
                let (dir, nsuf) = match c {
                    'W' => (Dir::W, "W"),
                    'E' => (Dir::E, "E"),
                    'S' => (Dir::S, "S"),
                    'N' => (Dir::N, "N"),
                    _ => unreachable!(),
                };
                last = self.builder.make_int_wire_cont(
                    &format!("INT.OMUX{}.{}{}", i, suf, c),
                    "INT.OMUX",
                    &[&format!("OMUX_{}{}{}", suf, c, i)],
                    dir,
                    last,
                );
                suf = nsuf;
            }
            if i == 2 {
                self.builder
                    .make_int_wire_cont("INT.OMUX2.S", "INT.OMUX", &["SE_S"], Dir::S, base);
            }
            if i == 9 {
                self.builder
                    .make_int_wire_cont("INT.OMUX9.N", "INT.OMUX", &["NW_N"], Dir::N, base);
            }
        }
    }

    fn setup_int_tie(&mut self) {
        let vcc = self
            .builder
            .geomdb
            .make_tie_wire("TIE.VCC", "TIE", TieState::S1);
        self.builder.register_int_wire(
            vcc,
            &[
                "VCC_PINWIRE",
                "IOIS_VCC_WIRE",
                "BRAM_VCC_WIRE",
                "BRAM_IOIS_VCC_WIRE",
                "CNR_VCC_WIRE",
                "DCM_VCC_WIRE",
                "MACC_VCC_WIRE",
            ],
        );
    }

    fn setup_int_imux(&mut self) {
        // Input mux outputs: final wires that go from interconnect to the site.
        // There are several kinds of those, with various properties:
        //
        // - 8 "fan" muxes, that feed into "data" and other "fan" muxes for
        //   more routing options, and are also used for BX/BY inputs in CLBs
        // - 32 "data" muxes, the main inputs (LUT inputs in CLBs)
        // - 4 "clk" muxes, for clocking (feed from clock interconnect in addition to main
        //   interconnect, also special feed from VHEX #4)
        // - 4 "set/reset" muxes (special feed from VHEX #0)
        // - 4 "clock enable" muxes (special feed from VHEX #7)
        //
        // Compared to Virtex 2, the IMUX structure is much more consistent across various tile
        // types.  However, there are still some differences:
        //
        // - IO tiles have 8 clock muxes instead of 4
        // - DCM tiles have clock muxes that can source directly from clock pads

        // The set/reset inputs.
        for i in 0..4 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.SR{}", i),
                "INT.IMUX.SR",
                &[
                    &format!("SR{}", i),
                    &format!("IOIS_SR{}", i),
                    &format!("CNR_SR{}", i),
                    &format!("BRAM_SR{}", i),
                    &format!("MACC_SR{}", i),
                ],
            );
        }

        // The clock inputs.
        for i in 0..4 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.CLK{}", i),
                "INT.IMUX.CLK",
                &[
                    &format!("CLK{}", i),
                    &format!("CNR_CLK{}", i),
                    &format!("BRAM_CLK{}", i),
                    &format!("MACC_CLK{}", i),
                    ["", "BRAM_IOIS_PSCLK", "BRAM_IOIS_CLKIN", "BRAM_IOIS_CLKFB"][i], // has different mux
                    ["", "DCM_PSCLK", "DCM_CLKIN", "DCM_CLKFB"][i], // has different mux
                ],
            );
        }

        for i in 0..8 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.IOCLK{}", i),
                "INT.IMUX.IOCLK",
                &[&format!("IOIS_CLK{}", i)],
            );
        }

        // The clock enables.
        for i in 0..4 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.CE{}", i),
                "INT.IMUX.CE",
                &[
                    &format!("CE_B{}", i),
                    &format!("IOIS_CE_B{}", i),
                    &format!("CNR_CE_B{}", i),
                    &format!("BRAM_CE_B{}", i),
                    &format!("MACC_CE_B{}", i),
                ],
            );
        }

        for i in 0..4 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.FAN.BX{}", i),
                "INT.IMUX.FAN",
                &[
                    &format!("BX{}", i),
                    &format!("IOIS_FAN_BX{}", i),
                    &format!("CNR_BX{}", i),
                    &if self.family == Family::S3ADSP {
                        format!("BRAM_BX_B{}", i)
                    } else {
                        format!("BRAM_FAN_BX{}", i)
                    },
                    &format!("MACC_BX_B{}", i),
                    &format!("BRAM_IOIS_FAN_BX{}", i),
                    &format!("DCM_FAN_BX{}", i),
                ],
            );
            if self.family == Family::S3ADSP {
                self.builder.make_int_wire(
                    &format!("INT.IMUX.FAN.BX{}.BOUNCE", i),
                    "INT.IMUX.FAN",
                    &[&format!("BRAM_FAN_BX{}", i), &format!("MACC_FAN_BX{}", i)],
                );
            }
        }
        for i in 0..4 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.FAN.BY{}", i),
                "INT.IMUX.FAN",
                &[
                    &format!("BY{}", i),
                    &format!("IOIS_FAN_BY{}", i),
                    &format!("CNR_BY{}", i),
                    &if self.family == Family::S3ADSP {
                        format!("BRAM_BY_B{}", i)
                    } else {
                        format!("BRAM_FAN_BY{}", i)
                    },
                    &format!("MACC_BY_B{}", i),
                    &format!("BRAM_IOIS_FAN_BY{}", i),
                    &format!("DCM_FAN_BY{}", i),
                ],
            );
            if self.family == Family::S3ADSP {
                self.builder.make_int_wire(
                    &format!("INT.IMUX.FAN.BY{}.BOUNCE", i),
                    "INT.IMUX.FAN",
                    &[&format!("BRAM_FAN_BY{}", i), &format!("MACC_FAN_BY{}", i)],
                );
            }
        }

        for i in 0..32 {
            self.builder.make_int_wire(
                &format!("INT.IMUX.DATA{}", i),
                "INT.IMUX.DATA",
                &[
                    &format!("{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                    &format!("IOIS_{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                    &format!(
                        "IOIS_STUB_{}{}_B{}",
                        ["F", "G"][i >> 4],
                        (i >> 2 & 3) + 1,
                        i & 3
                    ),
                    &format!(
                        "TBIOIS_{}{}_B{}",
                        ["F", "G"][i >> 4],
                        (i >> 2 & 3) + 1,
                        i & 3
                    ),
                    &format!(
                        "LRIOIS_{}{}_B{}",
                        ["F", "G"][i >> 4],
                        (i >> 2 & 3) + 1,
                        i & 3
                    ),
                    &format!("CNR_DATA_IN{}", i),
                    [
                        "BRAM_DIA_B18",
                        "BRAM_MULTINA_B15",
                        "BRAM_MULTINB_B17",
                        "BRAM_DIA_B1",
                        "BRAM_ADDRB_B0",
                        "BRAM_DIB_B19",
                        "BRAM_DIB_B0",
                        "BRAM_ADDRA_B3",
                        "BRAM_DIA_B19",
                        "BRAM_DIPB_B",
                        "BRAM_MULTINA_B17",
                        "BRAM_DIA_B0",
                        "BRAM_ADDRB_B1",
                        "BRAM_DIB_B18",
                        "BRAM_DIB_B1",
                        "BRAM_ADDRA_B2",
                        "BRAM_DIA_B2",
                        "BRAM_MULTINA_B14",
                        "BRAM_MULTINB_B16",
                        "BRAM_DIA_B17",
                        "BRAM_ADDRA_B0",
                        "BRAM_DIB_B3",
                        "BRAM_DIB_B16",
                        "BRAM_ADDRB_B3",
                        "BRAM_DIA_B3",
                        "BRAM_DIPA_B",
                        "BRAM_MULTINA_B16",
                        "BRAM_DIA_B16",
                        "BRAM_ADDRA_B1",
                        "BRAM_DIB_B2",
                        "BRAM_DIB_B17",
                        "BRAM_ADDRB_B2",
                    ][i],
                    // 3A DSP version
                    [
                        "",
                        "BRAM_MULTINA_B1",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "BRAM_MULTINA_B3",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "BRAM_MULTINA_B0",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "BRAM_MULTINA_B2",
                        "",
                        "",
                        "",
                        "",
                        "",
                    ][i],
                    [
                        "MACC_DIA_B18",
                        "MACC_MULTINA_B1",
                        "MACC_MULTINB_B17",
                        "MACC_DIA_B1",
                        "MACC_ADDRB_B0",
                        "MACC_DIB_B19",
                        "MACC_DIB_B0",
                        "MACC_ADDRA_B3",
                        "MACC_DIA_B19",
                        "MACC_DIPB_B",
                        "MACC_MULTINA_B3",
                        "MACC_DIA_B0",
                        "MACC_ADDRB_B1",
                        "MACC_DIB_B18",
                        "MACC_DIB_B1",
                        "MACC_ADDRA_B2",
                        "MACC_DIA_B2",
                        "MACC_MULTINA_B0",
                        "MACC_MULTINB_B16",
                        "MACC_DIA_B17",
                        "MACC_ADDRA_B0",
                        "MACC_DIB_B3",
                        "MACC_DIB_B16",
                        "MACC_ADDRB_B3",
                        "MACC_DIA_B3",
                        "MACC_DIPA_B",
                        "MACC_MULTINA_B2",
                        "MACC_DIA_B16",
                        "MACC_ADDRA_B1",
                        "MACC_DIB_B2",
                        "MACC_DIB_B17",
                        "MACC_ADDRB_B2",
                    ][i],
                    &format!(
                        "BRAM_IOIS_{}{}_B{}",
                        ["F", "G"][i >> 4],
                        (i >> 2 & 3) + 1,
                        i & 3
                    ),
                    &format!("DCM_{}{}_B{}", ["F", "G"][i >> 4], (i >> 2 & 3) + 1, i & 3),
                    [
                        "",
                        "",
                        "DCM_CTLSEL0_STUB",
                        "DCM_CTLSEL1_STUB",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "DCM_DSSEN_STUB",
                        "DCM_PSEN_STUB",
                        "DCM_PSINCDEC_STUB",
                        "DCM_RST_STUB",
                        "DCM_STSADRS1_STUB",
                        "DCM_STSADRS2_STUB",
                        "DCM_STSADRS3_STUB",
                        "DCM_STSADRS4_STUB",
                        "DCM_CTLMODE_STUB",
                        "DCM_FREEZEDLL_STUB",
                        "DCM_FREEZEDFS_STUB",
                        "DCM_STSADRS0_STUB",
                        "DCM_CTLSEL2_STUB",
                        "DCM_CTLOSC2_STUB",
                        "DCM_CTLOSC1_STUB",
                        "DCM_CTLG0_STUB",
                    ][i],
                ],
            );
        }
    }

    fn setup_int_out(&mut self) {
        for i in 0..8 {
            self.builder.make_int_out_wire(
                &format!("INT.OUT.FAN{}", i),
                "INT.OUT.FAN",
                &[
                    // In CLBs, used for combinatorial outputs.
                    ["X0", "X1", "X2", "X3", "Y0", "Y1", "Y2", "Y3"][i],
                    [
                        "IOIS_X0", "IOIS_X1", "IOIS_X2", "IOIS_X3", "IOIS_Y0", "IOIS_Y1",
                        "IOIS_Y2", "IOIS_Y3",
                    ][i],
                    ["", "", "", "STUB_IOIS_X3", "", "", "", "STUB_IOIS_Y3"][i],
                    // In BRAM, used for low data outputs.
                    [
                        "BRAM_DOA0",
                        "BRAM_DOA1",
                        "BRAM_DOA2",
                        "BRAM_DOA3",
                        "BRAM_DOB0",
                        "BRAM_DOB1",
                        "BRAM_DOB2",
                        "BRAM_DOB3",
                    ][i],
                    [
                        "MACC_DOA0",
                        "MACC_DOA1",
                        "MACC_DOA2",
                        "MACC_DOA3",
                        "MACC_DOB0",
                        "MACC_DOB1",
                        "MACC_DOB2",
                        "MACC_DOB3",
                    ][i],
                    [
                        "BRAM_IOIS_CLK270",
                        "BRAM_IOIS_CLK180",
                        "BRAM_IOIS_CLK90",
                        "BRAM_IOIS_CLK0",
                        "BRAM_IOIS_CLKFX180",
                        "BRAM_IOIS_CLKFX",
                        "BRAM_IOIS_CLK2X180",
                        "BRAM_IOIS_CLK2X",
                    ][i],
                    [
                        "DCM_CLK270",
                        "DCM_CLK180",
                        "DCM_CLK90",
                        "DCM_CLK0",
                        "DCM_CLKFX180",
                        "DCM_CLKFX",
                        "DCM_CLK2X180",
                        "DCM_CLK2X",
                    ][i],
                    &format!("CNR_D_O_FAN_B{}", i),
                ],
            );
        }

        for i in 0..16 {
            self.builder.make_int_out_wire(
                &format!("INT.OUT.SEC{}", i),
                "INT.OUT.SEC",
                &[
                    [
                        "XB0", "XB1", "XB2", "XB3", "YB0", "YB1", "YB2", "YB3", "XQ0", "XQ1",
                        "XQ2", "XQ3", "YQ0", "YQ1", "YQ2", "YQ3",
                    ][i],
                    [
                        "", "", "", "", "", "", "", "", "IOIS_XQ0", "IOIS_XQ1", "IOIS_XQ2",
                        "IOIS_XQ3", "IOIS_YQ0", "IOIS_YQ1", "IOIS_YQ2", "IOIS_YQ3",
                    ][i],
                    [
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "STUB_IOIS_XQ3",
                        "",
                        "",
                        "",
                        "STUB_IOIS_YQ3",
                    ][i],
                    // sigh. this does not appear to actually be true.
                    [
                        "",
                        "",
                        "",
                        "",
                        "BRAM_DOPA",
                        "BRAM_DOPB",
                        "",
                        "BRAM_MOUT32",
                        "BRAM_MOUT7",
                        "BRAM_MOUT6",
                        "BRAM_MOUT5",
                        "BRAM_MOUT4",
                        "BRAM_MOUT3",
                        "BRAM_MOUT2",
                        "BRAM_MOUT1",
                        "BRAM_MOUT0",
                    ][i],
                    [
                        "",
                        "",
                        "",
                        "",
                        "MACC_DOPA",
                        "MACC_DOPB",
                        "",
                        "MACC_MOUT32",
                        "MACC_MOUT7",
                        "MACC_MOUT6",
                        "MACC_MOUT5",
                        "MACC_MOUT4",
                        "MACC_MOUT3",
                        "MACC_MOUT2",
                        "MACC_MOUT1",
                        "MACC_MOUT0",
                    ][i],
                    [
                        "BRAM_IOIS_PSDONE",
                        "BRAM_IOIS_CONCUR",
                        "BRAM_IOIS_LOCKED",
                        "BRAM_IOIS_CLKDV",
                        "BRAM_IOIS_STATUS4",
                        "BRAM_IOIS_STATUS5",
                        "BRAM_IOIS_STATUS6",
                        "BRAM_IOIS_STATUS7",
                        "BRAM_IOIS_STATUS0",
                        "BRAM_IOIS_STATUS1",
                        "BRAM_IOIS_STATUS2",
                        "BRAM_IOIS_STATUS3",
                        "BRAM_IOIS_PTE2OMUX0",
                        "BRAM_IOIS_PTE2OMUX1",
                        "BRAM_IOIS_PTE2OMUX2",
                        "BRAM_IOIS_PTE2OMUX3",
                    ][i],
                    [
                        "DCM_PSDONE",
                        "DCM_CONCUR",
                        "DCM_LOCKED",
                        "DCM_CLKDV",
                        "DCM_STATUS4",
                        "DCM_STATUS5",
                        "DCM_STATUS6",
                        "DCM_STATUS7",
                        "DCM_STATUS0",
                        "DCM_STATUS1",
                        "DCM_STATUS2",
                        "DCM_STATUS3",
                        "DCM_PTE2OMUX0",
                        "DCM_PTE2OMUX1",
                        "DCM_PTE2OMUX2",
                        "DCM_PTE2OMUX3",
                    ][i],
                    [
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "",
                        "DCM_PTE2OMUX0_STUB",
                        "DCM_PTE2OMUX1_STUB",
                        "DCM_PTE2OMUX2_STUB",
                        "DCM_PTE2OMUX3_STUB",
                    ][i],
                    &format!("CNR_D_OUT_B{}", i),
                ],
            );
        }

        for i in 0..4 {
            for j in 0..2 {
                self.builder.make_int_out_wire(
                    &format!("INT.OUT.HALF{}.{}", i, j),
                    "INT.OUT.HALF",
                    &[
                        [
                            "BRAM_DOA16",
                            "BRAM_DOA17",
                            "BRAM_DOA19",
                            "BRAM_DOA18",
                            "BRAM_DOB16",
                            "BRAM_DOB17",
                            "BRAM_DOB19",
                            "BRAM_DOB18",
                        ][i + j * 4],
                        [
                            "MACC_DOA16",
                            "MACC_DOA17",
                            "MACC_DOA19",
                            "MACC_DOA18",
                            "MACC_DOB16",
                            "MACC_DOB17",
                            "MACC_DOB19",
                            "MACC_DOB18",
                        ][i + j * 4],
                    ],
                );
            }
        }
        // XXX 3e/3a dcm, dsp
    }

    fn setup_int(&mut self) {
        self.builder.setup_int();

        // The wires.
        self.setup_int_ll();
        self.setup_int_hex();
        self.setup_int_dbl();
        self.setup_int_omux();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();
    }

    fn setup_buses_clk(&mut self) {
        // XXX CLKBT bus
        // XXX CLKC/CLKL/CLKR bus
        // XXX GCLKVM bus
        // XXX HROW bus

        // Final clock distribution tile: from HROW bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..8 {
            let w = self.builder.geomdb.make_vbus_wire(
                &format!("HCLK.GCLK{}", i),
                "HCLK.GCLK",
                vbus_hclk,
                false,
            );
            self.builder
                .register_int_wire(w, &[&format!("GCLK{}", i), &format!("GCLK{}_BRK", i)]);
        }
    }

    fn setup_buses_dcm(&mut self) {
        for i in 0..4 {
            let _w = self.builder.make_int_wire(
                &format!("DCM.CLKPAD{}", i),
                "DCM.CLKPAD",
                &[
                    &format!("BRAM_IOIS_DLL_CLKPAD{}", i),
                    &format!("DCM_DLL_CLKPAD{}", i),
                    &format!("DCM_H_DLL_CLKPAD{}", i),
                ],
            );
            // XXX
        }
        // XXX 3e/3a CLKPAD
        // XXX 3e/3a DCMOUT
    }

    fn setup_clk(&mut self) {
        // XXX
    }

    fn setup_site(&mut self) {
        // XXX
    }

    fn setup_iob(&mut self) {
        // XXX
    }

    fn setup(&mut self) {
        self.setup_int();
        self.builder.setup_tiles();
        self.setup_buses_clk();
        self.setup_buses_dcm();
        self.setup_clk();
        self.setup_site();
        self.setup_iob();
    }

    fn make_grid_name(part: &str) -> String {
        if part.starts_with("xc3s") || part.starts_with("xa3s") {
            if part.ends_with('n') || part.ends_with('l') {
                part[2..part.len() - 1].to_string()
            } else {
                part[2..].to_string()
            }
        } else {
            panic!("unregognized part name {}", part);
        }
    }

    fn fill_grid_bus(&mut self, _part: &mut PartBuilder) {
        // XXX fill CLKBT
        // XXX fill CLKC/CLKL/CLKR
        // XXX fill GCLKVM
        // XXX fill HROW
        // XXX fill HCLK
        // XXX fill DCMOUT
        // XXX fill CLKPAD
    }

    fn fill_grid_site_conns(&mut self, _part: &mut PartBuilder) {
        // XXX
    }

    fn fill_grid_cols(&self, part: &mut PartBuilder) {
        let bram_cols = part.find_anchor_gx_set(&TileAnchor::int(&[
            "BRAM0",
            "BRAM0_SMALL",
            "BRAM0_SMALL_BOT",
        ]));
        assert!(part.grid.columns.is_empty());
        let mut bram_data = HashMap::new();
        if self.family != Family::S3 {
            for x in bram_cols.iter().copied() {
                bram_data.insert(x + 1, 0);
                bram_data.insert(x + 2, 1);
                if self.family != Family::S3ADSP {
                    bram_data.insert(x + 3, 2);
                }
            }
        }
        for x in 0..part.width() {
            let kind = if bram_cols.contains(&x) {
                "BRAM".to_string()
            } else if let Some(idx) = bram_data.get(&x) {
                format!("BRAMDATA{}", idx)
            } else {
                "MAIN".to_string()
            };
            part.grid.columns.push(kind);
        }
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

impl RdGeomMakerImpl for Spartan3GeomMaker {
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
