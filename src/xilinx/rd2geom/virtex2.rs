use std::collections::{HashSet, HashMap};
use std::iter;
use crate::xilinx::rawdump;
use crate::xilinx::geomdb::{GeomDb, TieState, Dir, Orient};
use crate::xilinx::geomdb::builder::{GeomDbBuilder, GridBuilder};
use crate::xilinx::geomraw::GeomRaw;
use super::RdGeomMakerImpl;
use super::cfg::{GeomBuilderConfig, IntTileInfo, IntTermInfo, IntDoubleBufInfo, TileAnchor, TileInfo, TieSiteInfo};
use super::builder::GeomBuilder;
use super::part::PartBuilder;

pub struct Virtex2GeomMaker {
    builder: GeomBuilder,
    pcls_bram_addr: Option<(usize, usize)>,
    pcls_center: Option<(usize, usize)>,
    pcls_tbuf: Option<(usize, usize)>,
}

impl Virtex2GeomMaker {
    pub fn new(family: &str) -> Self {
        let mut int_tiles = vec![
            IntTileInfo::int("CENTER", "INT_CLB"),

            IntTileInfo::int("LR_IOIS", "INT_IOI"),
            IntTileInfo::int("TB_IOIS", "INT_IOI"),
            IntTileInfo::int("ML_TB_IOIS", "INT_IOI"),
            IntTileInfo::int("ML_TBS_IOIS", "INT_IOI"),
            IntTileInfo::int("GIGABIT_IOI", "INT_IOI"),
            IntTileInfo::int("GIGABIT10_IOI", "INT_IOI"),
            IntTileInfo::int("MK_B_IOIS", "INT_IOI"),
            IntTileInfo::int("MK_T_IOIS", "INT_IOI"),

            IntTileInfo::int("BRAM0", "INT_BRAM"),
            IntTileInfo::int("BRAM1", "INT_BRAM"),
            IntTileInfo::int("BRAM2", "INT_BRAM"),
            IntTileInfo::int("BRAM3", "INT_BRAM"),

            IntTileInfo::int("BRAM_IOIS", "INT_DCM"),
            IntTileInfo::int("ML_BRAM_IOIS", "INT_DCM"),

            IntTileInfo::int("LL", "INT_CNR"),
            IntTileInfo::int("LR", "INT_CNR"),
            IntTileInfo::int("UL", "INT_CNR"),
            IntTileInfo::int("UR", "INT_CNR"),
        ];

        let mut int_terms = Vec::new();
        for t in &[
            "LTERM321", "LTERM010", "LTERM323", "LTERM210",
            "LTERM210_PCI", "LTERM323_PCI",
            "CNR_LTERM",
        ] {
            int_terms.push(IntTermInfo::l_fat(t, "INT_LTERM"));
        }
        for t in &[
            "RTERM321", "RTERM010", "RTERM323", "RTERM210",
            "RTERM210_PCI", "RTERM323_PCI",
            "CNR_RTERM",
        ] {
            int_terms.push(IntTermInfo::r_fat(t, "INT_RTERM"));
        }
        for t in &[
            "TTERM321", "TTERM010", "TTERM323", "TTERM210",
            "TCLKTERM321", "TCLKTERM210",
            "ML_TTERM010", "ML_TCLKTERM210",
            "BTTERM",
            "CNR_TTERM",
            "TGIGABIT_IOI_TERM", "TGIGABIT_INT_TERM",
            "TGIGABIT10_IOI_TERM", "TGIGABIT10_INT_TERM",
        ] {
            int_terms.push(IntTermInfo::t_fat(t, "INT_TTERM"));
        }
        for t in &[
            "BTERM010", "BTERM123", "BTERM012", "BTERM323",
            "BCLKTERM123", "BCLKTERM012",
            "ML_BCLKTERM123", "ML_BCLKTERM012",
            "BBTERM",
            "CNR_BTERM", "ML_CNR_BTERM",
            "BGIGABIT_IOI_TERM", "BGIGABIT_INT_TERM",
            "BGIGABIT10_IOI_TERM", "BGIGABIT10_INT_TERM",
        ] {
            int_terms.push(IntTermInfo::b_fat(t, "INT_BTERM"));
        }

        let mut int_dbufs = vec![];

        let tie_sites = vec![
            TieSiteInfo {
                kind: "VCC",
                pins: &[
                    ("VCCOUT", TieState::S1),
                ],
            },
        ];

        let mut tiles = vec![
            // XXX intif?
            TileInfo::hclk("HCLK", &["GCLKH", "LR_GCLKH"]),
            // XXX CLKC
            // XXX CLKT/CLKB
            // XXX DCMOUT
            // XXX IOB
            TileInfo::site_int("SITE_CLB", &["CENTER"]),
            TileInfo::site_vert_r("SITE_BRAM", &["BRAMSITE"], (4, 0)),

            TileInfo::site_int("SITE_IOI", &["LR_IOIS", "TB_IOIS", "ML_TB_IOIS", "ML_TBS_IOIS", "GIGABIT_IOI", "GIGBIT10_IOI"]),

            TileInfo::site_int_extra("SITE_DCM_B", &["BRAM_IOIS", "ML_BRAM_IOIS"], vec![(0, -1, &["BBTERM"])]),
            TileInfo::site_int_extra("SITE_DCM_T", &["BRAM_IOIS", "ML_BRAM_IOIS"], vec![(0, 1, &["BTTERM"])]),
            TileInfo::site_int("SITE_CNR_LL", &["LL"]),
            TileInfo::site_int("SITE_CNR_LR", &["LR"]),
            TileInfo::site_int("SITE_CNR_UL", &["UL"]),
            TileInfo::site_int("SITE_CNR_UR", &["UR"]),
            TileInfo::hclk_site_l("HCLK_SITE_PCI", &["REG_L"], (4, 2)),
            TileInfo::hclk_site_r("HCLK_SITE_PCI", &["REG_R"], (4, 2)),
        ];

        if family == "virtex2p" {
            int_tiles.append(&mut vec![
                IntTileInfo::int("BGIGABIT_INT0", "INT_PPC"),
                IntTileInfo::int("BGIGABIT_INT1", "INT_PPC"),
                IntTileInfo::int("BGIGABIT_INT2", "INT_PPC"),
                IntTileInfo::int("BGIGABIT_INT3", "INT_PPC"),
                IntTileInfo::int("BGIGABIT_INT4", "INT_GT_CLKPAD"),
                IntTileInfo::int("TGIGABIT_INT0", "INT_PPC"),
                IntTileInfo::int("TGIGABIT_INT1", "INT_PPC"),
                IntTileInfo::int("TGIGABIT_INT2", "INT_PPC"),
                IntTileInfo::int("TGIGABIT_INT3", "INT_PPC"),
                IntTileInfo::int("TGIGABIT_INT4", "INT_GT_CLKPAD"),
                IntTileInfo::int("BGIGABIT10_INT0", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT1", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT2", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT3", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT4", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT5", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT6", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT7", "INT_PPC"),
                IntTileInfo::int("BGIGABIT10_INT8", "INT_GT_CLKPAD"),
                IntTileInfo::int("TGIGABIT10_INT0", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT1", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT2", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT3", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT4", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT5", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT6", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT7", "INT_PPC"),
                IntTileInfo::int("TGIGABIT10_INT8", "INT_GT_CLKPAD"),

                IntTileInfo::int("BPPC_X0Y0_INT", "INT_PPC"),
                IntTileInfo::int("BPPC_X1Y0_INT", "INT_PPC"),
                IntTileInfo::int("LLPPC_X0Y0_INT", "INT_PPC"),
                IntTileInfo::int("LLPPC_X1Y0_INT", "INT_PPC"),
                IntTileInfo::int("TPPC_X0Y0_INT", "INT_PPC"),
                IntTileInfo::int("TPPC_X1Y0_INT", "INT_PPC"),
                IntTileInfo::int("ULPPC_X0Y0_INT", "INT_PPC"),
                IntTileInfo::int("ULPPC_X1Y0_INT", "INT_PPC"),
                IntTileInfo::int("LPPC_X0Y0_INT", "INT_PPC"),
                IntTileInfo::int("LPPC_X1Y0_INT", "INT_PPC"),
                IntTileInfo::int("RPPC_X0Y0_INT", "INT_PPC"),
                IntTileInfo::int("RPPC_X1Y0_INT", "INT_PPC"),
            ]);
            int_dbufs.append(&mut vec![
                IntDoubleBufInfo::h_fat("LPPC_X0Y0_INT", "PTERMR", "INT_HPPC"),
                IntDoubleBufInfo::h_fat("LPPC_X1Y0_INT", "PTERMR", "INT_HPPC"),
                IntDoubleBufInfo::h_fat("LLPPC_X0Y0_INT", "PTERMBR", "INT_HPPC"),
                IntDoubleBufInfo::h_fat("LLPPC_X1Y0_INT", "PTERMBR", "INT_HPPC"),
                IntDoubleBufInfo::h_fat("ULPPC_X0Y0_INT", "PTERMTR", "INT_HPPC"),
                IntDoubleBufInfo::h_fat("ULPPC_X1Y0_INT", "PTERMTR", "INT_HPPC"),
                IntDoubleBufInfo::v_fat("PTERMB", "PTERMT", "INT_VPPC"),
            ]);
            tiles.append(&mut vec![
                TileInfo::site_int("SITE_IOI_CLK_B", &["MK_B_IOIS"]),
                TileInfo::site_int("SITE_IOI_CLK_T", &["MK_T_IOIS"]),
                TileInfo::site_rect("SITE_PPC", &["LBPPC", "RBPPC"], (10, 16), (-6, -9)),
                TileInfo::site_vert_r("SITE_TGIGABIT", &["TGIGABIT"], (5, 0)),
                TileInfo::site_vert_r("SITE_BGIGABIT", &["BGIGABIT"], (5, 1)),
                TileInfo::site_vert_r("SITE_TGIGABIT10", &["TGIGABIT10"], (9, 0)),
                TileInfo::site_vert_r("SITE_BGIGABIT10", &["BGIGABIT10"], (9, 1)),
            ]);
        }
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
        let mut res = Virtex2GeomMaker {
            builder: GeomBuilder::new(family.to_string(), cfg),
            pcls_bram_addr: None,
            pcls_center: None,
            pcls_tbuf: None,
        };
        res.setup();
        res
    }

    fn is_2vp(&self) -> bool {
        self.builder.geomdb.name == "virtex2p"
    }

    fn setup_int_ll(&mut self) {
        // The long wires.
        let lh: Vec<_> = (0..24).map(|i| {
            let w = self.builder.geomdb.make_wire(&format!("INT.LH{}", i), "INT.LH", true);
            self.builder.register_int_wire(w, &[
                &format!("LH{}", i),
                &format!("LPPC_INT_LH{}", i),
            ]);
            w
        }).collect();
        for i in 0..24 {
            let d = lh[i];
            let u = if i == 0 { lh[23] } else { lh[i-1] };
            self.builder.connect_int_wire(d, Dir::E, u);
        }

        let lv: Vec<_> = (0..24).map(|i| {
            let w = self.builder.geomdb.make_wire(&format!("INT.LV{}", i), "INT.LV", true);
            self.builder.register_int_wire(w, &[
                &format!("LV{}", i),
            ]);
            w
        }).collect();
        for i in 0..24 {
            let d = lv[i];
            let u = if i == 23 { lv[0] } else { lv[i+1] };
            self.builder.connect_int_wire(d, Dir::N, u);
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
                    &[
                        &format!("{}6BEG{}", d, i),
                        &format!("LR_IOIS_{}6BEG{}", d, i),
                        &format!("TB_IOIS_{}6BEG{}", d, i),
                        &format!("LPPC_INT_{}6BEG{}", d, i),
                    ],
                );
                for seg in ["A", "B", "MID", "C", "D", "END"].iter().copied() {
                    let cur = self.builder.make_int_wire_cont(
                        &format!("INT.{}6{}{}", d, seg, i),
                        cls,
                        &[
                            &format!("{}6{}{}", d, seg, i),
                            &format!("LR_IOIS_{}6{}{}", d, seg, i),
                            &format!("TB_IOIS_{}6{}{}", d, seg, i),
                            &format!("LPPC_INT_{}6{}{}", d, seg, i),
                        ],
                        dir,
                        last
                    );
                    if self.is_2vp() {
                        // For skipping over PPC
                        // XXX register
                        self.builder.geomdb.make_wire(
                            &format!("INT.{}6{}{}.FAR", d, seg, i),
                            cls, false,
                        );
                    }
                    last = cur;
                }
                match dir {
                    Dir::E | Dir::S => {
                        if i < 2 {
                            self.builder.make_int_wire_cont(
                                &format!("INT.{}6END_S{}", d, i),
                                cls,
                                &[
                                    &format!("{}6END_S{}", d, i),
                                    &format!("LR_IOIS_{}6END_S{}", d, i),
                                    &format!("TB_IOIS_{}6END_S{}", d, i),
                                    &format!("LPPC_INT_{}6END_S{}", d, i),
                                ],
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
                                &[
                                    &format!("{}6END_N{}", d, i),
                                    &format!("LR_IOIS_{}6END_N{}", d, i),
                                    &format!("TB_IOIS_{}6END_N{}", d, i),
                                    &format!("LPPC_INT_{}6END_N{}", d, i),
                                ],
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
                    &[
                        &format!("{}2BEG{}", d, i),
                        &format!("LPPC_INT_{}2BEG{}", d, i),
                    ],
                );
                for seg in ["MID", "END"].iter().copied() {
                    last = self.builder.make_int_wire_cont(
                        &format!("INT.{}2{}{}", d, seg, i),
                        cls,
                        &[
                            &format!("{}2{}{}", d, seg, i),
                            &format!("LPPC_INT_{}2{}{}", d, seg, i),
                        ],
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
                                &[
                                    &format!("{}2END_S{}", d, i),
                                    &format!("LPPC_INT_{}2END_S{}", d, i),
                                ],
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
                                &[
                                    &format!("{}2END_N{}", d, i),
                                    &format!("LPPC_INT_{}2END_N{}", d, i),
                                ],
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
            let base = self.builder.make_int_wire(&format!("INT.OMUX{}", i), "INT.OMUX", &[
                &format!("OMUX{}", i),
                &format!("LPPC_INT_OMUX{}", i),
            ]);
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
                last = self.builder.make_int_wire_cont(&format!("INT.OMUX{}.{}{}", i, suf, c), "INT.OMUX", &[
                    &format!("OMUX_{}{}{}", suf, c, i),
                    &format!("LPPC_INT_OMUX_{}{}{}", suf, c, i),
                ], dir, last);
                suf = nsuf;
            }
        }
    }

    fn setup_int_tie(&mut self) {
        let vcc = self.builder.geomdb.make_tie_wire("TIE.VCC", "TIE", TieState::S1);
        self.builder.register_int_wire(vcc, &[
            "VCC_PINWIRE",
            "IOIS_VCC_WIRE",
            "BRAM_VCC_WIRE",
            "BRAM_IOIS_VCC_WIRE",
            "CNR_VCC_WIRE",
            "GIGABIT_INT_VCC_WIRE",
        ]);
    }

    fn setup_int_imux(&mut self) {
        // Input mux outputs: final wires that go from interconnect to the site.
        // There are several kinds of those, with various properties:
        //
        // - 8 "fan" muxes, that feed into "data" and other "fan" muxes for
        //   more routing options, and are also used for BX/BY inputs in CLBs
        // - 32 "data" muxes, the main inputs (LUT inputs in CLBs)
        // - 4 "clk" muxes, for clocking (feed from clock interconnect in addition to main
        //   interconnect, also special feed from VHEX #6)
        // - 4 "set/reset" muxes (special feed from VHEX #0)
        // - 4 "clock enable" muxes (special feed from VHEX #9)
        // - 2 "tristate input" and 2 "tristate enable" muxes (special feed from VHEX #3)
        //
        // However, this is only the "base" version, and some types of tiles are special:
        //
        // - corners have the "base" version
        // - PPC and inner GT interconnects seem to be identical to corners, but have
        //   differently-numbered wires
        // - DCM and outer GT interconnects lack the TS and two CE imuxes in favor
        //   of bigger CLK imuxes that can source directly from the clock pads
        // - CLB interconnect has the same set of fan and data inputs as corner, but with
        //   different bitstream encoding and layout, so they don't really correspond
        //   to one another
        // - in BRAM, 8 of the "data" imuxes are effectively replaced with "bram addr"
        //   imuxes, that can source from the output of "bram addr" imuxes 4 tiles
        //   below and above, in addition to usual inputs
        // - in IOIS:
        //
        //   - usual clock muxes are used for output clocks
        //   - TI/TS imuxes are replaced with 4 more clock muxes, for input
        //     clocks (special feed from VHEX #3)
        //
        //   - 16 "data" imuxes are replaced with further control imuxes:
        //
        //     - 4 "I/O tristate 1" muxes (special feed from VHEX #1)
        //     - 4 "I/O tristate 2" muxes (special feed from VHEX #4)
        //     - 4 "input CE" muxes (special feed from VHEX #5)
        //     - 4 "tristate CE" muxes (special feed from VHEX #8)

        // The BRAM interconnect tiles have a funny arrangement where the address input muxes
        // can source from the output of the same mux 4 tiles above or below.
        let pslot_bram_addr_n = self.builder.geomdb.make_port_slot("BRAM_ADDR_N");
        let pslot_bram_addr_s = self.builder.geomdb.make_port_slot("BRAM_ADDR_S");
        let (pcls_bram_addr_n, pcls_bram_addr_s) = self.builder.geomdb.make_port_pair(("BRAM_ADDR_N_PASS", "BRAM_ADDR_S_PASS"), (pslot_bram_addr_n, pslot_bram_addr_s));
        self.pcls_bram_addr = Some((pcls_bram_addr_n, pcls_bram_addr_s));

        // The set/reset inputs.
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.SR{}", i), "INT.IMUX.SR", &[
                &format!("SR{}", i),
                // Note: IOIS SR numbering follows associated IOB order.
                &format!("IOIS_SR_B{}", [1, 2, 0, 3][i]),
                &format!("CNR_SR{}", i),
                &format!("BRAM_SR{}", i),
                &format!("BRAM_IOIS_SR{}", i),
                &format!("LRPPC_INT_SR{}", i),
                &format!("BPPC_INT_SR{}", i),
                &format!("TPPC_INT_SR{}", i),
                &format!("GIGABIT_INT_SR{}", i),
            ]);
        }

        // The tristate inputs.
        for i in 0..2 {
            self.builder.make_int_wire(&format!("INT.IMUX.TI{}", i), "INT.IMUX.TI", &[
                &format!("TI{}", i),
                &format!("IOIS_CK{}_B0", [2, 1][i]),
                &format!("CNR_TI{}", i),
                &format!("BRAM_TI{}", i),
                &format!("BRAM_IOIS_TI{}", i),
                &format!("LRPPC_INT_TI{}", i),
                &format!("BPPC_INT_TI{}", i),
                &format!("TPPC_INT_TI{}", i),
                &format!("GIGABIT_INT_TI{}", i),
            ]);
        }

        // The tristate enables.
        for i in 0..2 {
            self.builder.make_int_wire(&format!("INT.IMUX.TS{}", i), "INT.IMUX.TS", &[
                &format!("TS{}", i),
                &format!("IOIS_CK{}_B2", [1, 2][i]),
                &format!("CNR_TS{}", i),
                &format!("BRAM_TS{}", i),
                &format!("LRPPC_INT_TS{}", i),
                &format!("BPPC_INT_TS{}", i),
                &format!("TPPC_INT_TS{}", i),
                &format!("GIGABIT_INT_TS{}", i),
            ]);
        }

        // The clock inputs.
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.CLK{}", i), "INT.IMUX.CLK", &[
                &format!("CLK{}", i),
                &format!("IOIS_CK{}_B{}", [2, 1, 2, 1][i], [1, 1, 3, 3][i]),
                &format!("CNR_CLK{}", i),
                &format!("BRAM_CLK{}", i),
                ["BRAM_IOIS_CLKFB", "BRAM_IOIS_CLKIN", "BRAM_IOIS_PSCLK", ""][i], // has different mux
                &format!("LRPPC_INT_CLK{}", i),
                &format!("BPPC_INT_CLK{}", i),
                &format!("TPPC_INT_CLK{}", i),
                &format!("GIGABIT_INT_CLK{}", i), // sometimes has different mux
            ]);
        }

        // The clock enables.
        for i in 0..4 {
            self.builder.make_int_wire(&format!("INT.IMUX.CE{}", i), "INT.IMUX.CE", &[
                &format!("CE_B{}", i),
                &format!("OCE_B{}", [1, 0, 3, 2][i]), // IOIS
                &format!("CNR_CE_B{}", i),
                &format!("BRAM_CE_B{}", i),
                &format!("BRAM_IOIS_CE_B{}", i), // only valid for 2, 3
                &format!("LRPPC_INT_CE_B{}", i),
                &format!("BPPC_INT_CE_B{}", i),
                &format!("TPPC_INT_CE_B{}", i),
                &format!("GIGABIT_INT_CE_B{}", i),
            ]);
        }

        // The fan muxes.
        for i in 0..4 {
            for j in 0..2 {
                let ri = 3 - i;
                self.builder.make_int_wire(&format!("INT.IMUX.G{}.FAN{}", i, j), "INT.IMUX.FAN", &[
                    match (i, j) { // has different mux
                        (0, 0) => "BX0",
                        (0, 1) => "BX2",
                        (1, 0) => "BY0",
                        (1, 1) => "BY2",
                        (2, 0) => "BY1",
                        (2, 1) => "BY3",
                        (3, 0) => "BX1",
                        (3, 1) => "BX3",
                        _ => unreachable!(),
                    },
                    match (i, j) {
                        (0, 0) => "IOIS_FAN_BX0",
                        (0, 1) => "IOIS_FAN_BX2",
                        (1, 0) => "IOIS_FAN_BY0",
                        (1, 1) => "IOIS_FAN_BY2",
                        (2, 0) => "IOIS_FAN_BY1",
                        (2, 1) => "IOIS_FAN_BY3",
                        (3, 0) => "IOIS_FAN_BX1",
                        (3, 1) => "IOIS_FAN_BX3",
                        _ => unreachable!(),
                    },
                    &format!("CNR_FAN{}{}", ri, j),
                    &format!("BRAM_FAN{}{}", ri, j),
                    &format!("LRPPC_INT_FAN{}{}", ri, j),
                    &format!("BPPC_INT_FAN{}{}", ri, j),
                    &format!("TPPC_INT_FAN{}{}", ri, j),
                    &format!("GIGABIT_INT_FAN{}{}", ri, j),
                ]);
            }
        }

        // The data inputs.
        for i in 0..4 {
            for j in 0..8 {
                let w = self.builder.make_int_wire(&format!("INT.IMUX.G{}.DATA{}", i, j), "INT.IMUX.DATA", &[
                    match (i, j) { // has different mux
                        (0, 0) => "G4_B2",
                        (0, 1) => "F4_B2",
                        (0, 2) => "F4_B0",
                        (0, 3) => "G4_B0",
                        (0, 4) => "G1_B3",
                        (0, 5) => "F1_B3",
                        (0, 6) => "F1_B1",
                        (0, 7) => "G1_B1",

                        (1, 0) => "G3_B1",
                        (1, 1) => "F3_B1",
                        (1, 2) => "F3_B3",
                        (1, 3) => "G3_B3",
                        (1, 4) => "G2_B0",
                        (1, 5) => "F2_B0",
                        (1, 6) => "F2_B2",
                        (1, 7) => "G2_B2",

                        (2, 0) => "G3_B2",
                        (2, 1) => "F3_B2",
                        (2, 2) => "F3_B0",
                        (2, 3) => "G3_B0",
                        (2, 4) => "G2_B3",
                        (2, 5) => "F2_B3",
                        (2, 6) => "F2_B1",
                        (2, 7) => "G2_B1",

                        (3, 0) => "G4_B1",
                        (3, 1) => "F4_B1",
                        (3, 2) => "F4_B3",
                        (3, 3) => "G4_B3",
                        (3, 4) => "G1_B0",
                        (3, 5) => "F1_B0",
                        (3, 6) => "F1_B2",
                        (3, 7) => "G1_B2",

                        _ => unreachable!(),
                    },
                    &match (i, j) {
                        (0, _) if j < 4 => format!("TS1_B{}", 3 - j), // has different mux
                        (1, _) if j < 4 => format!("TS2_B{}", 3 - j), // has different mux
                        (2, _) if j < 4 => format!("ICE_B{}", 3 - j), // has different mux
                        (3, _) if j < 4 => format!("TCE_B{}", 3 - j), // has different mux
                        (_, 5) => format!("IOIS_REV_B{}", i),
                        (_, 6) => format!("O2_B{}", i),
                        (_, 7) => format!("O1_B{}", i),
                        _ => "".to_string(),
                    },
                    &format!("DATA_IN{}", i * 8 + j), // CNR
                    &match (i, j) {
                        (_, 0) => format!("BRAM_ADDRB_B{}", i), // has different mux
                        (_, 1) => format!("BRAM_ADDRA_B{}", i), // has different mux
                        (0, 2) => "BRAM_DIPB".to_string(),
                        (0, 3) => "BRAM_DIPA".to_string(),
                        (2, 2) => "BRAM_MULTINB16".to_string(),
                        (2, 3) => "BRAM_MULTINB17".to_string(),
                        (3, 2) => "BRAM_MULTINA16".to_string(),
                        (3, 3) => "BRAM_MULTINA17".to_string(),
                        (_, 4) => format!("BRAM_DIB{}", i),
                        (_, 5) => format!("BRAM_DIB{}", 16 + i),
                        (_, 6) => format!("BRAM_DIA{}", i),
                        (_, 7) => format!("BRAM_DIA{}", 16 + i),
                        _ => "".to_string(),
                    },
                    &match (i, j) {
                        (0, 0) => "BRAM_IOIS_DSSEN".to_string(),
                        (0, 1) => "BRAM_IOIS_CTLSEL0".to_string(),
                        (0, 2) => "BRAM_IOIS_CTLSEL1".to_string(),
                        (0, 3) => "BRAM_IOIS_CTLSEL2".to_string(),
                        (1, 0) => "BRAM_IOIS_PSEN".to_string(),
                        (1, 1) => "BRAM_IOIS_CTLOSC2".to_string(),
                        (1, 2) => "BRAM_IOIS_CTLOSC1".to_string(),
                        (1, 3) => "BRAM_IOIS_CTLGO".to_string(),
                        (2, 0) => "BRAM_IOIS_PSINCDEC".to_string(),
                        (2, 1) => "BRAM_IOIS_CTLMODE".to_string(),
                        (2, 2) => "BRAM_IOIS_FREEZEDLL".to_string(),
                        (2, 3) => "BRAM_IOIS_FREEZEDFS".to_string(),
                        (3, 0) => "BRAM_IOIS_RST".to_string(),
                        (3, 1) => "BRAM_IOIS_STSADRS0".to_string(),
                        (3, 2) => "BRAM_IOIS_STSADRS1".to_string(),
                        (3, 3) => "BRAM_IOIS_STSADRS2".to_string(),
                        (3, 4) => "BRAM_IOIS_STSADRS3".to_string(),
                        (3, 5) if self.is_2vp() => "BRAM_IOIS_STSADRS4".to_string(),
                        _ => format!("BRAM_IOIS_DATA{}", i * 8 + j),
                    },
                    &format!("LRPPC_INT_DATA_IN{}", j * 4 + i),
                    &format!("BPPC_INT_DATA_IN{}", j * 4 + i),
                    &format!("TPPC_INT_DATA_IN{}", j * 4 + i),
                    &format!("GIGABIT_INT_DATA_IN{}", j * 4 + i),
                ]);
                if j < 2 {
                    let w_s = self.builder.make_int_wire(&format!("INT.IMUX.G{}.DATA{}.S", i, j), "INT.IMUX.DATA", &[
                        &format!("BRAM_ADDR{}_SEND{}", ["B", "A"][j], i),
                    ]);
                    let w_n = self.builder.make_int_wire(&format!("INT.IMUX.G{}.DATA{}.N", i, j), "INT.IMUX.DATA", &[
                        &format!("BRAM_ADDR{}_NEND{}", ["B", "A"][j], i),
                    ]);
                    self.builder.geomdb.make_simple_pconn(w_s, w, pcls_bram_addr_s, pcls_bram_addr_n);
                    self.builder.geomdb.make_simple_pconn(w_n, w, pcls_bram_addr_n, pcls_bram_addr_s);
                    // XXX ppc to BRAM addr special conn
                }
            }
        }
        self.builder.mark_pcls_filled(pcls_bram_addr_n);
        self.builder.mark_pcls_filled(pcls_bram_addr_s);
    }

    fn setup_int_out(&mut self) {
        // The logic outputs to interconnect are much less regular than imuxes, with the set
        // of outputs varying between tile types.
        //
        // There are several kind of outputs:
        //
        // - primary (high-fanout) outputs, which go to all OMUXes and double/hex interconnect
        // - secondary outputs, which go to all OMUXes
        // - tertiary outputs, which go to half the available OMUXes
        // - test outputs, which go to 2 of 16 OMUXes
        //
        // Each OMUX has 24 inputs (though some are unused in some tiles).  First 8
        // of them correspond to primary outputs (except in DCMs), but
        // the exact mapping between double/hex inputs and OMUX inputs varies between
        // tile types.
        //
        // We call the primary outputs by their corner tile order, and merge them based
        // on same mapping to double/hex inputs (even though they're routed differently
        // to OMUX inputs).
        //
        // OMUX inputs are:
        //
        // CENTER   IOIS    DCM         PPC         CNR         BRAM
        // X0       I0      -           PPC10       DOUT_FAN0   DOA0
        // X1       I1      -           PPC11       DOUT_FAN1   DOA1
        // X3       I3      CLKFX180    PPC12       DOUT_FAN3   DOA3
        // X2       I2      CLKFX       PPC13       DOUT_FAN2   DOA2
        // Y0               CLKDV       PPC14       DOUT_FAN4   DOB0
        // Y1               CLK2X180    PPC15       DOUT_FAN5   DOB1
        //
        // Y2               CLK2X       PPC16       DOUT_FAN6   DOB2
        // Y3               CLK270      PPC17       DOUT_FAN7   DOB3
        // YB0+             CLK180      PPC27       DOUT18*     DOx16*
        // YB1+     IQ21    CLK90       PPC26       DOUT16*     DOx17*
        // YB3+     IQ23    CLK0        PPC25       DOUT14*     DOx19*
        // YB2+             CONCUR      PPC24       DOUT12*     DOx18*
        //
        // XB1+     TS1     PSDONE      PPC23       DOUT10*     DOPA
        // XB2+     TS2     LOCKED      PPC22       DOUT8*      DOPB
        // XB3+     TS3     STATUS3*    PPC21       DOUT6*
        // YQ0      IQ20    STATUS2*    PPC20       DOUT4*      MOUT32
        // YQ1              STATUS1*    TEST1#      DOUT2*      MOUT7
        // XB0+     TS0     STATUS0*    TEST0#      DOUT0*      MOUT6
        //
        // YQ2      IQ22    UTURN.C0#   UTURN.C0#   UTURN.C0#   MOUT5
        // YQ3              UTURN.C1#   UTURN.C1#   UTURN.C1#   MOUT4
        // XQ0      IQ10    UTURN.D0#   UTURN.D0#   UTURN.D0#   MOUT3
        // XQ1      IQ11    UTURN.D1#   UTURN.D1#   UTURN.D1#   MOUT2
        // XQ2      IQ12    UTURN.D2#   UTURN.D2#   UTURN.D2#   MOUT1
        // XQ3      IQ13    UTURN.D3#   UTURN.D3#   UTURN.D3#   MOUT0
        //
        // * means a tertiary output (ie. there are two possibilities depending on OMUX)
        // # means a test output (ie. there are 8 possibilities depending on OMUX)
        // + means one of those is replaced by TBUS (depending on OMUX)
        for i in 0..8 {
            let w = self.builder.make_int_out_wire(&format!("INT.OUT.FAN{}", i), "INT.OUT.FAN", &[
                // In CLBs, used for combinatorial outputs.
                ["X0", "X1", "X2", "X3", "Y0", "Y1", "Y2", "Y3"][i],
                // In IOIS, used for combinatorial inputs.  4-7 are unused.
                ["I0", "I1", "I2", "I3", "", "", "", ""][i],
                // In BRAM, used for low data outputs.
                [
                    "BRAM_DOA2",
                    "BRAM_DOA3",
                    "BRAM_DOA0",
                    "BRAM_DOA1",
                    "BRAM_DOB1",
                    "BRAM_DOB0",
                    "BRAM_DOB3",
                    "BRAM_DOB2",
                ][i],
                &format!("DOUT_FAN{}", i),
                &format!("LRPPC_INT_PPC1{}", i),
                &format!("BPPC_INT_PPC1{}", i),
                &format!("TPPC_INT_PPC1{}", i),
                &format!("GIGABIT_INT_PPC1{}", i),
            ]);
            if self.is_2vp() {
                if i == 0 {
                    self.builder.register_tile_wire(w, &["INT_IOI.MK_T_IOIS"], "IOIS_BREFCLK_SE", (0, 0));
                }
                if i == 2 {
                    self.builder.register_tile_wire(w, &["INT_IOI.MK_B_IOIS"], "IOIS_BREFCLK_SE", (0, 0));
                }
            }
        }

        // We call secondary outputs by their OMUX index.
        for i in 2..24 {
            self.builder.make_int_out_wire(&format!("INT.OUT.SEC{}", i), "INT.OUT.SEC", &[
                &[
                    "", "", "", "", "", "",
                    "", "", "YB0", "YB1", "YB3", "YB2",
                    "XB1", "XB2", "XB3", "YQ0", "YQ1", "XB0",
                    "YQ2", "YQ3", "XQ0", "XQ1", "XQ2", "XQ3",
                ][i],
                &[
                    "", "", "", "", "", "",
                    "", "", "", "I_Q21", "I_Q23", "",
                    "TS_FDBK1", "TS_FDBK2", "TS_FDBK3", "I_Q20", "", "TS_FDBK0",
                    "I_Q22", "", "I_Q10", "I_Q11", "I_Q12", "I_Q13",
                ][i],
                &[
                    "", "", "", "", "", "",
                    "", "", "", "", "", "",
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
                &[
                    "", "",
                    "BRAM_IOIS_CLKFX180",
                    "BRAM_IOIS_CLKFX",
                    "BRAM_IOIS_CLKDV",
                    "BRAM_IOIS_CLK2X180",
                    "BRAM_IOIS_CLK2X",
                    "BRAM_IOIS_CLK270",
                    "BRAM_IOIS_CLK180",
                    "BRAM_IOIS_CLK90",
                    "BRAM_IOIS_CLK0",
                    "BRAM_IOIS_CONCUR",
                    "BRAM_IOIS_PSDONE",
                    "BRAM_IOIS_LOCKED",
                    "", "", "", "",
                    "", "", "", "", "", "",
                ][i],
                &if (8..16).contains(&i) {format!("LRPPC_INT_PPC2{}", 15-i)} else {format!("")},
                &if (8..16).contains(&i) {format!("BPPC_INT_PPC2{}", 15-i)} else {format!("")},
                &if (8..16).contains(&i) {format!("TPPC_INT_PPC2{}", 15-i)} else {format!("")},
                &if (8..16).contains(&i) {format!("GIGABIT_INT_PPC2{}", 15-i)} else {format!("")},
            ]);
        }

        // Same for tertiary.
        for i in 8..18 {
            for j in 0..2 {
                self.builder.make_int_out_wire(&format!("INT.OUT.HALF{}.{}", i, j), "INT.OUT.HALF", &[
                    &format!("DOUT{}", (17 - i) * 2 + j),
                    if i < 12 {[
                        "BRAM_DOA16",
                        "BRAM_DOA17",
                        "BRAM_DOA19",
                        "BRAM_DOA18",
                        "BRAM_DOB16",
                        "BRAM_DOB17",
                        "BRAM_DOB19",
                        "BRAM_DOB18",
                    ][i-8 + j * 4]} else {""},
                    &if i >= 14 {format!("BRAM_IOIS_STATUS{}", (i - 14) + j * 4)} else {format!("")},
                ]);
            }
        }

        if self.is_2vp() {
            for i in 0..16 {
                self.builder.make_int_out_wire(&format!("INT.OUT.TEST{}", i), "INT.OUT.TEST", &[
                    &format!("LRPPC_INT_TEST{}", i),
                    &format!("BPPC_INT_TEST{}", i),
                    &format!("TPPC_INT_TEST{}", i),
                    &format!("GIGABIT_INT_TEST{}", i),
                ]);
            }

            // For PPC, there is a test mux between site outputs and interconnect inputs,
            // for u-turn testing.  These are the actual site outputs.
            // XXX register
            for i in 0..8 {
                self.builder.geomdb.make_wire(&format!("INT.OUT.FAN{}.SITE", i), "INT.OUT.FAN.SITE", false);
            }
            for i in 8..16 {
                self.builder.geomdb.make_wire(&format!("INT.OUT.SEC{}.SITE", i), "INT.OUT.SEC.SITE", false);
            }
        }

        // And the tristate bus output.
        self.builder.make_int_out_wire("INT.OUT.TBUS", "INT.OUT.TBUS", &["TBUS"]);

        // PCI needs special output wires.
        // XXX register
        self.builder.geomdb.make_wire("INT.OUT.PCI0", "INT.OUT.PCI", false);
        self.builder.geomdb.make_wire("INT.OUT.PCI1", "INT.OUT.PCI", false);
    }

    fn setup_int(&mut self) {
        // Main interconnect tile:
        // - input muxes to SITE
        // - output muxes from SITE [OMUX]
        // - routing muxes for double, hex, and long lines
        // - for CENTER tiles, also includes the SLICEs
        // - for IOI tiles, also includes the I/O logic (but not I/O buffers)
        self.builder.setup_int();

        // The wires.
        self.setup_int_ll();
        self.setup_int_hex();
        self.setup_int_dbl();
        self.setup_int_omux();
        self.setup_int_tie();
        self.setup_int_imux();
        self.setup_int_out();

        if self.is_2vp() {
            // XXX change this
            // Interconnect interface tile: not used much on Virtex 2, but provides
            // imux-to-omux test mux for PPC/GT interconnect tiles.
            let tslot_int_if = self.builder.geomdb.make_tile_slot("INT_IF");
            self.builder.geomdb.make_tile_single("INT_IF_PPC", tslot_int_if);
            self.builder.geomdb.make_tile_single("INT_IF_GT_0", tslot_int_if);
            self.builder.geomdb.make_tile_single("INT_IF_GT_123", tslot_int_if);
            self.builder.geomdb.make_tile_single("INT_IF_GT_4", tslot_int_if);
        }
    }

    fn setup_buses_clk(&mut self) {
        // Clock distribution buses — 4 stages.
        let vbus_clkbt = self.builder.geomdb.make_vert_bus("CLKBT");
        for i in 0..8 {
            self.builder.geomdb.make_vbus_wire(&format!("CLKBT.GCLK{}", i), "CLKBT.GCLK", vbus_clkbt, false);
        }

        let vbus_clkc = self.builder.geomdb.make_vert_bus("CLKC");
        for i in 0..8 {
            self.builder.geomdb.make_vbus_wire(&format!("CLKC.GCLKB{}", i), "CLKC.GCLK", vbus_clkc, false);
            self.builder.geomdb.make_vbus_wire(&format!("CLKC.GCLKT{}", i), "CLKC.GCLK", vbus_clkc, false);
        }

        let hbus_hrow = self.builder.geomdb.make_horiz_bus("HROW");
        for i in 0..8 {
            self.builder.geomdb.make_hbus_wire(&format!("HROW.GCLK{}", i), "HROW.GCLK", hbus_hrow, false);
        }

        // Final clock distribution tile: from HROW bus to HCLK bus.
        let vbus_hclk = self.builder.geomdb.make_vert_bus("HCLK");
        for i in 0..8 {
            let w = self.builder.geomdb.make_vbus_wire(&format!("HCLK.GCLK{}", i), "HCLK.GCLK", vbus_hclk, false);
            self.builder.register_int_wire(w, &[&format!("GCLK{}", i)]);
        }
    }

    fn setup_buses_dcm(&mut self) {
        // Buses involving DCMs.
        let hbus_dcm = self.builder.geomdb.make_horiz_bus("DCM");
        for i in 0..8 {
            let w = self.builder.geomdb.make_hbus_wire(&format!("DCM.CLKPAD{}", i), "DCM.CLKPAD", hbus_dcm, false);
            self.builder.register_int_wire(w, &[
                &format!("BRAM_IOIS_DLL_CLKPAD{}", i),
                &format!("GIGABIT_INT_DLL_CLKPAD{}", i),
            ]);
        }
        for i in 0..8 {
            self.builder.geomdb.make_hbus_wire(&format!("DCM.DCMOUT{}", i), "DCM.DCMOUT", hbus_dcm, true);
        }
    }

    fn setup_buses_gt(&mut self) {
        // GT buses.
        if self.is_2vp() {
            let hbus_brefclk = self.builder.geomdb.make_horiz_bus("BREFCLK");
            self.builder.geomdb.make_hbus_wire("BREFCLK.BREFCLK", "BREFCLK", hbus_brefclk, false);
            self.builder.geomdb.make_hbus_wire("BREFCLK.BREFCLK2", "BREFCLK", hbus_brefclk, false);
            let hbus_brefclkx = self.builder.geomdb.make_horiz_bus("BREFCLKX");
            self.builder.geomdb.make_hbus_wire("BREFCLKX.P", "BREFCLKX", hbus_brefclkx, false);
            self.builder.geomdb.make_hbus_wire("BREFCLKX.N", "BREFCLKX", hbus_brefclkx, false);
        }
    }

    fn setup_clk(&mut self) {
        // XXX needs to die
        let tslot_clk = self.builder.geomdb.make_tile_slot("CLK");
        self.builder.geomdb.make_tile("CLKC", &[(0, 0, tslot_clk), (0, 1, tslot_clk)]);
        self.builder.geomdb.make_tile("CLK_B", &[(0, 0, tslot_clk), (1, 0, tslot_clk)]);
        self.builder.geomdb.make_tile("CLK_T", &[(0, 0, tslot_clk), (1, 0, tslot_clk)]);
        self.builder.geomdb.make_tile_single("CLK_DCMOUT_B", tslot_clk);
        self.builder.geomdb.make_tile_single("CLK_DCMOUT_T", tslot_clk);

        // Middle clock distribution tile: from CLKC bus to HROW bus.
        let tslot_hrow = self.builder.geomdb.make_tile_slot("HROW");
        self.builder.geomdb.make_tile("HROW_M", &[(0, 0, tslot_hrow), (1, 0, tslot_hrow)]);
        if !self.is_2vp() {
            self.builder.geomdb.make_tile("HROW_B", &[(0, 0, tslot_hrow), (1, 0, tslot_hrow)]);
            self.builder.geomdb.make_tile("HROW_T", &[(0, 0, tslot_hrow), (1, 0, tslot_hrow)]);
        }
    }

    fn setup_clkbt(&mut self) {
        for i in 0..8 {
            self.builder.geomdb.make_wire(&format!("CLKBT.IMUX.SEL{}", i), "CLKBT.IMUX.SEL", false);
        }
        for i in 0..8 {
            self.builder.geomdb.make_wire(&format!("CLKBT.IMUX.CLKDUB{}", i), "CLKBT.IMUX.CLKDUB", false);
        }
        for i in 0..8 {
            self.builder.geomdb.make_wire(&format!("CLKBT.IMUX.CLK{}", i), "CLKBT.IMUX.CLK", false);
        }
    }

    fn setup_site_clb(&mut self) {
        // For CENTER tiles dedicated east/west interconnect (SOPOUT).  Skips over GTs and BRAMs,
        // but broken by PPC.
        let pslot_center_e = self.builder.geomdb.make_port_slot("CENTER_E");
        let pslot_center_w = self.builder.geomdb.make_port_slot("CENTER_W");
        let (pcls_center_e, pcls_center_w) = self.builder.geomdb.make_port_pair(("CENTER_E_PASS", "CENTER_W_PASS"), (pslot_center_e, pslot_center_w));
        self.pcls_center = Some((pcls_center_e, pcls_center_w));

        // The carry output:
        //
        // COUT0 -> CIN1
        // COUT1 -> CIN0 tile up
        // COUT2 -> CIN3
        // COUT3 -> CIN2 tile up
        for i in 0..4 {
            let co = self.builder.make_tile_wire(&format!("CLB.COUT{}", i), "CLB.COUT", &["SITE_CLB"], &format!("COUT{}", i), (0, 0));
            if i == 1 || i == 3 {
                let co_n = self.builder.make_tile_wire(&format!("CLB.COUT{}.N", i), "CLB.COUT", &["SITE_CLB"], &format!("CIN{}", i-1), (0, 0));
                self.builder.connect_int_wire(co_n, Dir::N, co);
            }
        }

        // The sum-of-products output:
        //
        // SOPOUT0 -> SOPIN2
        // SOPOUT1 -> SOPIN3
        // SOPOUT2 -> SOPIN0 tile right
        // SOPOUT3 -> SOPIN1 tile right
        for i in 0..4 {
            let so = self.builder.make_tile_wire(&format!("CLB.SOPOUT{}", i), "CLB.SOPOUT", &["SITE_CLB"], &format!("SOPOUT{}", i), (0, 0));
            if i >= 2 {
                let so_e = self.builder.make_tile_wire(&format!("CLB.SOPOUT{}.E", i), "CLB.SOPOUT", &["SITE_CLB"], &format!("SOPIN{}", i-2), (0, 0));
                self.builder.geomdb.make_simple_pconn(so_e, so, pcls_center_e, pcls_center_w);
            }
        }

        // The F5/FX outputs:
        //
        // F50 -> FXINA0
        // F51 -> FXINB0
        // F52 -> FXINA2
        // F53 -> FXINB2
        // FX0 -> FXINB1
        // FX1 -> FXINB3 and FXINA3 tile down
        // FX2 -> FXINA1
        // FX3 -> discarded
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("CLB.F5OUT{}", i), "CLB.F5OUT", &["SITE_CLB"], &format!("F5{}", i), (0, 0));
        }
        for i in 0..4 {
            let fx = self.builder.make_tile_wire(&format!("CLB.FXOUT{}", i), "CLB.FXOUT", &["SITE_CLB"], &format!("FX{}", i), (0, 0));
            if i == 1 {
                let fx_s = self.builder.make_tile_wire(&format!("CLB.FXOUT{}.S", i), "CLB.FXOUT", &["SITE_CLB"], "FXINA3", (0, 0));
                self.builder.connect_int_wire(fx_s, Dir::S, fx);
            }
        }

        // The shift output:
        //
        // SHIFTOUT0 -> discarded; there is some evidence that it could be meant to be SHIFTIN3
        //              tile down, but that doesn't seem to be supported by ISE
        // SHIFTOUT1 -> SHIFTIN0
        // SHIFTOUT2 -> SHIFTIN1
        // SHIFTOUT3 -> SHIFTIN2
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("CLB.SHIFTOUT{}", i), "CLB.SHIFTOUT", &["SITE_CLB"], &format!("SHIFTOUT{}", i), (0, 0));
        }

        // The BXOUT and SLICEWE0.
        //
        // BXOUT0 -> SLICEWE0[02]
        // BXOUT1 -> SLICEWE0[13]
        // BXOUT[23] -> discarded
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("CLB.BXOUT{}", i), "CLB.BXOUT", &["SITE_CLB"], &format!("BXOUT{}", i), (0, 0));
        }

        // The BYOUT/BYINVOUT and SLICE write enables.
        // BYOUT0 -> SLICEWE1[02]
        // BYINVOUT0 -> SLICEWE1[13]
        // BYOUT1 -> SLICEWE2[01]
        // BYINVOUT1 -> SLICEWE2[23]
        // others -> discarded
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("CLB.BYOUT{}", i), "CLB.BYOUT", &["SITE_CLB"], &format!("BYOUT{}", i), (0, 0));
        }
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("CLB.BYINVOUT{}", i), "CLB.BYINVOUT", &["SITE_CLB"], &format!("BYINVOUT{}", i), (0, 0));
        }

        // The DIG output:
        //
        // DIG1 -> ALTDIG0
        // DIG3 -> ALTDIG[12] and ALTDIG3 tile down
        // others -> discarded
        for i in 0..4 {
            let dig = self.builder.make_tile_wire(&format!("CLB.DIG{}", i), "CLB.DIG", &["SITE_CLB"], &format!("DIG{}", i), (0, 0));
            if i == 3 {
                let dig_s = self.builder.make_tile_wire(&format!("CLB.DIG{}.S", i), "CLB.DIG", &["SITE_CLB"], "DIG_S3", (0, 0));
                self.builder.connect_int_wire(dig_s, Dir::S, dig);
            }
        }

        self.builder.mark_pcls_filled(pcls_center_e);
        self.builder.mark_pcls_filled(pcls_center_w);

        // For CENTER tiles TBUF interconnect.  Skips over GTs, BRAMs, PPC.
        let pslot_tbuf_e = self.builder.geomdb.make_port_slot("TBUF_E");
        let pslot_tbuf_w = self.builder.geomdb.make_port_slot("TBUF_W");
        let (pcls_tbuf_e, pcls_tbuf_w) = self.builder.geomdb.make_port_pair(("TBUF_E_PASS", "TBUF_W_PASS"), (pslot_tbuf_e, pslot_tbuf_w));
        self.pcls_tbuf = Some((pcls_tbuf_e, pcls_tbuf_w));

        // The immediate tristate buffer outputs.
        for i in 0..2 {
            self.builder.make_tile_wire(&format!("CLB.TOUT{}", i), "CLB.TOUT", &["SITE_CLB"], &format!("TOUT{}", i), (0, 0));
        }

        // The tristate bus.
        let tbuf: Vec<_> = (0..4).map(|i| self.builder.make_tile_wire(&format!("CLB.TBUF{}", i), "CLB.TBUF", &["SITE_CLB"], &format!("TBUF{}", i), (0, 0))).collect();
        let tbuf0_w = self.builder.make_tile_wire("CLB.TBUF0.W", "CLB.TBUF", &["SITE_CLB"], "TBUF3_E", (0, 0));

        self.builder.geomdb.make_simple_pconn(tbuf0_w, tbuf[0], pcls_tbuf_w, pcls_tbuf_e);
        self.builder.geomdb.make_simple_pconn(tbuf[0], tbuf[1], pcls_tbuf_w, pcls_tbuf_e);
        self.builder.geomdb.make_simple_pconn(tbuf[1], tbuf[2], pcls_tbuf_w, pcls_tbuf_e);
        self.builder.geomdb.make_simple_pconn(tbuf[3], tbuf[2], pcls_tbuf_e, pcls_tbuf_w);

        self.builder.mark_pcls_filled(pcls_tbuf_e);
        self.builder.mark_pcls_filled(pcls_tbuf_w);
    }

    fn setup_iob(&mut self) {
        let tslot_site = self.builder.geomdb.tile_slots.idx("SITE");
        self.builder.geomdb.make_tile("SITE_IOB_T321010", &[(0, 0, tslot_site), (1, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_T323210", &[(0, 0, tslot_site), (1, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_T323210_MK", &[(0, 0, tslot_site), (1, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_T210", &[(0, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_T321", &[(0, 0, tslot_site)]);

        self.builder.geomdb.make_tile("SITE_IOB_B010123", &[(0, 0, tslot_site), (1, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_B010123_MK", &[(0, 0, tslot_site), (1, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_B012323", &[(0, 0, tslot_site), (1, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_B012", &[(0, 0, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_B123", &[(0, 0, tslot_site)]);

        self.builder.geomdb.make_tile("SITE_IOB_L321010", &[(0, 0, tslot_site), (0, 1, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_L323210", &[(0, 0, tslot_site), (0, 1, tslot_site)]);

        self.builder.geomdb.make_tile("SITE_IOB_R321010", &[(0, 0, tslot_site), (0, 1, tslot_site)]);
        self.builder.geomdb.make_tile("SITE_IOB_R323210", &[(0, 0, tslot_site), (0, 1, tslot_site)]);

        // Make some wires for I/O.
        let ioi_tiles = &[
            "SITE_IOI",
            "SITE_IOI_CLK_B",
            "SITE_IOI_CLK_T",
        ];
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("IOI.I{}", i), "IOI.I", ioi_tiles, &format!("I{}_PINWIRE", i), (0, 0));
        }
        for i in 0..4 {
            self.builder.make_tile_wire(&format!("IOI.IBUF{}", i), "IOI.IBUF", ioi_tiles, &format!("IOIS_IBUF{}", i), (0, 0));
        }
    }

    fn setup(&mut self) {
        self.setup_int();
        self.builder.setup_tiles();
        self.setup_buses_clk();
        self.setup_buses_dcm();
        self.setup_buses_gt();
        self.setup_clk();
        self.setup_clkbt();
        self.setup_site_clb();
        self.setup_iob();
    }

    fn make_grid_name(part: &str) -> String {
        if part.starts_with("xc2v") || part.starts_with("xq2v") {
            part[2..].to_string()
        } else if part.starts_with("xqr2v") {
            part[3..].to_string()
        } else {
            panic!("unregognized part name {}", part);
        }
    }

    fn fill_grid_int_bram(&self, part: &mut PartBuilder) {
        // BRAM connections special.
        let xy: HashSet<(usize, usize)> = part.find_anchors(&TileAnchor::int(&["BRAM0", "BRAM1", "BRAM2", "BRAM3"])).into_iter().map(|(_, xy)| xy).collect();
        for (x, y) in xy.iter().copied() {
            let oy = y + 4;
            if !xy.contains(&(x, oy)) {
                continue;
            }
            part.grid.fill_port_pair(&self.builder.geomdb, (x, y), (x, oy), self.pcls_bram_addr.unwrap());
        }
    }

    fn fill_grid_bus(&self, part: &mut PartBuilder) {
        // The HCLK bus.
        let hclk_rows = part.find_anchor_gy_set(&TileAnchor::snap_n(&["GCLKH"]));
        let hbrk_rows = part.find_anchor_gy_set(&TileAnchor::snap_n(&["BRKH", "CLKH"]));
        part.fill_srcbrk_bus_split(&self.builder, Orient::V, "HCLK", hclk_rows.iter().copied(), hbrk_rows.iter().copied());

        let clkcs = part.find_anchors(&TileAnchor::snap_ne(&["CLKC"]));
        assert_eq!(clkcs.len(), 1);
        let (clkc_x, clkc_y) = clkcs[0].1;

        part.fill_srcbrk_bus_split(&self.builder, Orient::H, "HROW", iter::once(clkc_x), iter::empty());
        part.fill_srcbrk_bus_split(&self.builder, Orient::V, "CLKC", iter::once(clkc_y), iter::empty());
        part.fill_bus(&self.builder, Orient::V, "CLKBT", vec![0, clkc_y, part.height()], vec![0, part.height()-1]);
        part.fill_srcbrk_bus_split(&self.builder, Orient::H, "DCM", iter::once(clkc_x), iter::empty());
        if self.is_2vp() {
            part.fill_srcbrk_bus(&self.builder, Orient::H, "BREFCLK", iter::once(clkc_x), iter::empty());
            part.fill_srcbrk_bus(&self.builder, Orient::H, "BREFCLKX", iter::once(clkc_x - 1), iter::empty());
        }
    }

    fn fill_grid_site_conns(&self, part: &mut PartBuilder) {
        let bram_cols = part.find_anchor_gx_set(&TileAnchor::int(&["BRAM0"]));
        let xy: HashSet<(usize, usize)> = part.find_anchors(&TileAnchor::int(&["CENTER"])).into_iter().map(|(_, xy)| xy).collect();
        for (x, y) in xy.iter().copied() {
            // Look for the CLB to the right, if any.
            // For the main port (SOPOUT), only consider direct neighbours, or neighbours
            // separated only by a BRAM column.
            // For the TBUF port, go as far as needed.
            let mut direct = true;
            for ox in (x+1)..part.width() {
                if xy.contains(&(ox, y)) {
                    part.grid.fill_port_pair(&self.builder.geomdb, (x, y), (ox, y), self.pcls_tbuf.unwrap());
                    if direct {
                        part.grid.fill_port_pair(&self.builder.geomdb, (x, y), (ox, y), self.pcls_center.unwrap());
                    }
                    break
                }
                if !bram_cols.contains(&ox) {
                    direct = false;
                }
            }
        }
    }

    fn fill_grid_cols(&self, part: &mut PartBuilder) {
        let bram_cols = part.find_anchor_gx_set(&TileAnchor::int(&["BRAM0"]));
        assert!(part.grid.columns.is_empty());
        for x in 0..part.width() {
            let kind = if bram_cols.contains(&x) {
                "BRAM"
            } else {
                "MAIN"
            };
            part.grid.columns.push(kind.to_string());
        }
    }

    fn fill_grid(&mut self, part: &mut PartBuilder) {
        part.fill_int(&mut self.builder);
        self.fill_grid_int_bram(part);
        self.fill_grid_bus(part);
        self.fill_grid_site_conns(part);
        part.fill_tiles(&self.builder);
        self.fill_grid_cols(part);
    }

    fn verify(&self, _part: &PartBuilder) {
        // XXX
    }
}

impl RdGeomMakerImpl for Virtex2GeomMaker {
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
