use std::collections::HashSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{dir::DirV, grid::TileCoord};
use prjcombine_re_fpga_hammer::{
    FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_bit_wide, xlat_enum,
    xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_spartan6::{bels, chip::Gts};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{DynProp, pip::PinFar, relation::TileRelation},
    },
};

#[derive(Clone, Copy, Debug)]
struct HclkInt(DirV);

impl TileRelation for HclkInt {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        backend.edev.find_tile_by_class(
            tcrd.delta(
                0,
                match self.0 {
                    DirV::S => -1,
                    DirV::N => 0,
                },
            ),
            |kind| kind.starts_with("INT"),
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct HclkHasCmt;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for HclkHasCmt {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        if tcrd.row == edev.chip.row_clk() {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct BufpllPll(DirV, &'static str, &'static str, String, String);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BufpllPll {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        let mut cell = tcrd.cell;
        loop {
            match self.0 {
                DirV::S => {
                    if cell.row.to_idx() == 0 {
                        return Some((fuzzer, true));
                    }
                    cell.row -= 1;
                }
                DirV::N => {
                    cell.row += 1;
                    if cell.row == edev.chip.rows.next_id() {
                        return Some((fuzzer, true));
                    }
                }
            }
            if let Some(ntcrd) = backend
                .edev
                .find_tile_by_class(cell, |kind| kind.starts_with("PLL_BUFPLL"))
            {
                if edev.db.tile_classes.key(edev[ntcrd].class) != self.1 {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: self.1.into(),
                        bel: self.2.into(),
                        attr: self.3.clone(),
                        val: self.4.clone(),
                    },
                    rects: edev.tile_bits(ntcrd),
                });
                return Some((fuzzer, false));
            }
        }
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    if devdata_only {
        let mut ctx = FuzzCtx::new(session, backend, "PCILOGICSE");
        let mut bctx = ctx.bel(bels::PCILOGICSE);
        bctx.build()
            .no_global("PCI_CE_DELAY_LEFT")
            .test_manual("PRESENT", "1")
            .mode("PCILOGICSE")
            .commit();
        return;
    }
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK");
        let mut bctx = ctx.bel(bels::HCLK);
        for i in 0..16 {
            let gclk_i = format!("GCLK{i}_I");
            let gclk_o_d = format!("GCLK{i}_O_D");
            let gclk_o_u = format!("GCLK{i}_O_U");
            bctx.build()
                .has_related(HclkInt(DirV::S))
                .test_manual(&gclk_o_d, "1")
                .pip(&gclk_o_d, &gclk_i)
                .commit();
            bctx.build()
                .has_related(HclkInt(DirV::N))
                .test_manual(&gclk_o_u, "1")
                .pip(&gclk_o_u, &gclk_i)
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "HCLK_H_MIDBUF") {
        let mut bctx = ctx.bel(bels::HCLK_H_MIDBUF);
        for i in 0..16 {
            bctx.build()
                .null_bits()
                .test_manual(format!("GCLK{i}_M"), "1")
                .pip(format!("GCLK{i}_M"), format!("GCLK{i}_I"))
                .commit();
            bctx.build()
                .null_bits()
                .test_manual(format!("GCLK{i}_O"), "1")
                .pip(format!("GCLK{i}_O"), format!("GCLK{i}_M"))
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK_ROW");
        for slots in [bels::BUFH_W, bels::BUFH_E] {
            for i in 0..16 {
                let mut bctx = ctx.bel(slots[i]);
                let obel = bels::HCLK_ROW;
                bctx.build()
                    .mutex("I", "BUFG")
                    .test_manual("I", "BUFG")
                    .pip((PinFar, "I"), (obel, format!("BUFG{i}")))
                    .commit();
                bctx.build()
                    .mutex("I", "CMT")
                    .prop(HclkHasCmt)
                    .test_manual("I", "CMT")
                    .pip((PinFar, "I"), (obel, format!("CMT{i}")))
                    .commit();
                bctx.build()
                    .null_bits()
                    .test_manual("PRESENT", "1")
                    .mode("BUFH")
                    .commit();
                bctx.build()
                    .null_bits()
                    .test_manual("OUTPUT", "1")
                    .pip((PinFar, "O"), "O")
                    .commit();
                bctx.build()
                    .null_bits()
                    .test_manual("INPUT", "1")
                    .pip("I", (PinFar, "I"))
                    .commit();
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK_V_MIDBUF");
        let mut bctx = ctx.bel(bels::HCLK_V_MIDBUF);
        for i in 0..16 {
            bctx.build()
                .null_bits()
                .test_manual(format!("GCLK{i}_M"), "1")
                .pip(format!("GCLK{i}_M"), format!("GCLK{i}_I"))
                .commit();
            bctx.build()
                .null_bits()
                .test_manual(format!("GCLK{i}_O"), "1")
                .pip(format!("GCLK{i}_O"), format!("GCLK{i}_M"))
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CLKC");
        for i in 0..16 {
            let mut bctx = ctx.bel(bels::BUFGMUX[i]);
            bctx.test_manual("PRESENT", "1").mode("BUFGMUX").commit();
            bctx.mode("BUFGMUX").test_inv("S");
            bctx.mode("BUFGMUX")
                .test_enum("CLK_SEL_TYPE", &["SYNC", "ASYNC"]);
            bctx.mode("BUFGMUX")
                .test_enum("DISABLE_ATTR", &["LOW", "HIGH"]);
        }
        let mut bctx = ctx.bel(bels::CLKC);
        for i in 0..16 {
            for inp in [
                format!("CKPIN_H{i}"),
                format!("CKPIN_V{i}"),
                format!("CMT_D{i}"),
                format!("CMT_U{i}"),
            ] {
                bctx.build()
                    .tile_mutex(format!("IMUX{i}"), &inp)
                    .test_manual(format!("IMUX{i}"), &inp)
                    .pip(format!("MUX{i}"), inp)
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bels::CLKC_BUFPLL);
        for (out, altout) in [
            ("OUTL_CLKOUT0", "OUTL_CLKOUT1"),
            ("OUTL_CLKOUT1", "OUTL_CLKOUT0"),
            ("OUTR_CLKOUT0", "OUTR_CLKOUT1"),
            ("OUTR_CLKOUT1", "OUTR_CLKOUT0"),
            ("OUTD_CLKOUT0", "OUTD_CLKOUT1"),
            ("OUTD_CLKOUT1", "OUTD_CLKOUT0"),
            ("OUTU_CLKOUT0", "OUTU_CLKOUT1"),
            ("OUTU_CLKOUT1", "OUTU_CLKOUT0"),
        ] {
            for inp in [
                "PLL0D_CLKOUT0",
                "PLL0D_CLKOUT1",
                "PLL1D_CLKOUT0",
                "PLL1D_CLKOUT1",
                "PLL0U_CLKOUT0",
                "PLL0U_CLKOUT1",
                "PLL1U_CLKOUT0",
                "PLL1U_CLKOUT1",
            ] {
                if out.starts_with("OUTU") && (inp.starts_with("PLL0U") || inp.starts_with("PLL1U"))
                {
                    continue;
                }
                if out.starts_with("OUTD") && (inp.starts_with("PLL0D") || inp.starts_with("PLL1D"))
                {
                    continue;
                }
                let mut builder = bctx
                    .build()
                    .global_mutex_here("BUFPLL_CLK")
                    .tile_mutex(out, inp)
                    .tile_mutex(altout, inp)
                    .pip(altout, inp);
                if out.starts_with("OUTD") {
                    let tt = if edev.chip.rows.len() < 128 {
                        "PLL_BUFPLL_OUT1"
                    } else {
                        "PLL_BUFPLL_OUT0"
                    };

                    builder = builder.prop(BufpllPll(
                        DirV::S,
                        tt,
                        "PLL_BUFPLL",
                        format!("CLKC_CLKOUT{i}", i = &out[11..]),
                        "1".into(),
                    ));
                }

                builder.test_manual(out, inp).pip(out, inp).commit();
            }
        }
        for out in [
            "OUTL_LOCKED0",
            "OUTL_LOCKED1",
            "OUTR_LOCKED0",
            "OUTR_LOCKED1",
            "OUTD_LOCKED",
            "OUTU_LOCKED",
        ] {
            for inp in [
                "PLL0D_LOCKED",
                "PLL1D_LOCKED",
                "PLL0U_LOCKED",
                "PLL1U_LOCKED",
            ] {
                if out.starts_with("OUTU") && (inp.starts_with("PLL0U") || inp.starts_with("PLL1U"))
                {
                    continue;
                }
                if out.starts_with("OUTD") && (inp.starts_with("PLL0D") || inp.starts_with("PLL1D"))
                {
                    continue;
                }
                bctx.build()
                    .tile_mutex(out, inp)
                    .test_manual(out, inp)
                    .pip(out, inp)
                    .commit();
            }
        }
    }
    for (i, tile) in ["PLL_BUFPLL_OUT0", "PLL_BUFPLL_OUT1"]
        .into_iter()
        .enumerate()
    {
        if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) {
            let mut bctx = ctx.bel(bels::PLL_BUFPLL);
            for out in ["CLKOUT0", "CLKOUT1", "LOCKED"] {
                for ud in ['U', 'D'] {
                    bctx.build()
                        .test_manual(format!("PLL{i}_{out}_{ud}"), "1")
                        .pip(format!("{out}_{ud}"), out)
                        .commit();
                }
            }
        }
    }
    for (tile, bel) in [
        ("DCM_BUFPLL_BUF_S", bels::DCM_BUFPLL_BUF_S),
        ("DCM_BUFPLL_BUF_S_MID", bels::DCM_BUFPLL_BUF_S_MID),
        ("DCM_BUFPLL_BUF_N", bels::DCM_BUFPLL_BUF_N),
        ("DCM_BUFPLL_BUF_N_MID", bels::DCM_BUFPLL_BUF_N_MID),
    ] {
        if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) {
            let mut bctx = ctx.bel(bel);
            for src in ["PLL0", "PLL1", "CLKC"] {
                let (bufs_d, bufs_u) = match (tile, edev.chip.rows.len() / 64) {
                    ("DCM_BUFPLL_BUF_S", _) => (vec![], vec!["PLL_BUFPLL_OUT1"]),
                    ("DCM_BUFPLL_BUF_S_MID", 2) => {
                        (vec!["PLL_BUFPLL_OUT1"], vec!["PLL_BUFPLL_OUT0"])
                    }
                    ("DCM_BUFPLL_BUF_S_MID", 3) => (
                        vec!["PLL_BUFPLL_OUT1", "PLL_BUFPLL_B"],
                        vec!["PLL_BUFPLL_OUT0", "PLL_BUFPLL_B"],
                    ),
                    ("DCM_BUFPLL_BUF_N", 1) => (vec![], vec!["PLL_BUFPLL_OUT1"]),
                    ("DCM_BUFPLL_BUF_N", 2 | 3) => (vec![], vec!["PLL_BUFPLL_OUT0"]),
                    ("DCM_BUFPLL_BUF_N_MID", 2) => {
                        (vec!["PLL_BUFPLL_OUT0"], vec!["PLL_BUFPLL_OUT1"])
                    }
                    ("DCM_BUFPLL_BUF_N_MID", 3) => (
                        vec!["PLL_BUFPLL_OUT0", "PLL_BUFPLL_T"],
                        vec!["PLL_BUFPLL_OUT1", "PLL_BUFPLL_T"],
                    ),
                    _ => unreachable!(),
                };
                for wire in ["CLKOUT0", "CLKOUT1"] {
                    let mut builder = bctx.build().global_mutex_here("BUFPLL_CLK");
                    for (dir, bufs) in [(DirV::S, &bufs_d), (DirV::N, &bufs_u)] {
                        for &buf in bufs {
                            if buf == "PLL_BUFPLL_OUT0" && src == "PLL0" {
                                continue;
                            }
                            if buf == "PLL_BUFPLL_OUT1" && src == "PLL1" {
                                continue;
                            }
                            builder = builder.prop(BufpllPll(
                                dir,
                                buf,
                                "PLL_BUFPLL",
                                format!("{src}_{wire}"),
                                "1".into(),
                            ));
                        }
                    }

                    builder
                        .test_manual(format!("{src}_{wire}"), "1")
                        .pip(format!("{src}_{wire}_O"), format!("{src}_{wire}_I"))
                        .commit();
                }
                bctx.build()
                    .test_manual(format!("{src}_LOCKED"), "1")
                    .pip(format!("{src}_LOCKED_O"), format!("{src}_LOCKED_I"))
                    .commit();
            }
        }
    }
    for (tile, is_lr) in [
        ("REG_B", false),
        ("REG_T", false),
        ("REG_L", true),
        ("REG_R", true),
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for i in 0..8 {
            let mut bctx = ctx.bel(bels::BUFIO2[i]);
            bctx.test_manual("PRESENT", "BUFIO2")
                .mode("BUFIO2")
                .commit();
            bctx.test_manual("PRESENT", "BUFIO2_2CLK")
                .mode("BUFIO2_2CLK")
                .commit();
            bctx.mode("BUFIO2")
                .attr("DIVIDE", "")
                .test_enum("DIVIDE_BYPASS", &["FALSE", "TRUE"]);
            bctx.mode("BUFIO2")
                .attr("DIVIDE_BYPASS", "FALSE")
                .test_enum("DIVIDE", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            for val in ["1", "2", "3", "4", "5", "6", "7", "8"] {
                bctx.mode("BUFIO2_2CLK")
                    .attr("POS_EDGE", "")
                    .attr("NEG_EDGE", "")
                    .attr("R_EDGE", "")
                    .test_manual("DIVIDE.2CLK", val)
                    .attr("DIVIDE", val)
                    .commit();
            }
            bctx.mode("BUFIO2_2CLK")
                .attr("DIVIDE", "")
                .test_enum("POS_EDGE", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("BUFIO2_2CLK")
                .attr("DIVIDE", "")
                .test_enum("NEG_EDGE", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("BUFIO2_2CLK")
                .attr("DIVIDE", "")
                .test_enum("R_EDGE", &["FALSE", "TRUE"]);

            for (val, pin) in [
                ("CLKPIN0", format!("CLKPIN{i}")),
                ("CLKPIN1", format!("CLKPIN{ii}", ii = i ^ 1)),
                ("CLKPIN4", format!("CLKPIN{ii}", ii = i ^ 4)),
                ("CLKPIN5", format!("CLKPIN{ii}", ii = i ^ 5)),
                (
                    "DQS0",
                    format!("DQS{pn}{ii}", pn = ['P', 'N'][i & 1], ii = i >> 1),
                ),
                (
                    "DQS2",
                    format!("DQS{pn}{ii}", pn = ['P', 'N'][i & 1], ii = i >> 1 ^ 2),
                ),
                ("DFB", format!("DFB{i}")),
                (
                    "GTPCLK",
                    format!(
                        "GTPCLK{ii}",
                        ii = match (tile, i) {
                            // ???
                            ("REG_L" | "REG_R", 1) => 0,
                            ("REG_L" | "REG_R", 3) => 2,
                            _ => i,
                        }
                    ),
                ),
            ] {
                let obel = bels::BUFIO2_INS;
                bctx.mode("BUFIO2")
                    .mutex("I", val)
                    .test_manual("I", val)
                    .pin("I")
                    .pip("I", (obel, &pin))
                    .commit();
            }
            for (val, pin) in [
                ("CLKPIN0", format!("CLKPIN{i}")),
                ("CLKPIN1", format!("CLKPIN{ii}", ii = i ^ 1)),
                ("CLKPIN4", format!("CLKPIN{ii}", ii = i ^ 4)),
                ("CLKPIN5", format!("CLKPIN{ii}", ii = i ^ 5)),
                (
                    "DQS0",
                    format!("DQS{pn}{ii}", pn = ['N', 'P'][i & 1], ii = i >> 1),
                ),
                (
                    "DQS2",
                    format!("DQS{pn}{ii}", pn = ['N', 'P'][i & 1], ii = i >> 1 ^ 2),
                ),
                ("DFB", format!("DFB{ii}", ii = i ^ 1)),
            ] {
                let obel = bels::BUFIO2_INS;
                bctx.mode("BUFIO2_2CLK")
                    .mutex("IB", val)
                    .test_manual("IB", val)
                    .pin("IB")
                    .pip("IB", (obel, &pin))
                    .commit();
            }

            bctx.mode("BUFIO2")
                .global_mutex("IOCLK_OUT", "TEST")
                .test_manual("IOCLK_ENABLE", "1")
                .pin("IOCLK")
                .pip((PinFar, "IOCLK"), "IOCLK")
                .commit();
            bctx.mode("BUFIO2")
                .global_mutex("BUFIO2_CMT_OUT", "TEST_BUFIO2")
                .test_manual("CMT_ENABLE", "1")
                .pin("DIVCLK")
                .pip((PinFar, "DIVCLK"), "DIVCLK")
                .pip("CMT", (PinFar, "DIVCLK"))
                .commit();
            bctx.mode("BUFIO2")
                .mutex("CKPIN", "DIVCLK")
                .test_manual("CKPIN", "DIVCLK")
                .pin("DIVCLK")
                .pip((PinFar, "DIVCLK"), "DIVCLK")
                .pip("CKPIN", (PinFar, "DIVCLK"))
                .commit();
            let obel = bels::BUFIO2_CKPIN;
            bctx.build()
                .mutex("CKPIN", "CLKPIN")
                .test_manual("CKPIN", "CLKPIN")
                .pip((obel, format!("CKPIN{i}")), (obel, format!("CLKPIN{i}")))
                .commit();
            let obel_tie = bels::TIEOFF_REG;
            bctx.build()
                .mutex("CKPIN", "VCC")
                .test_manual("CKPIN", "VCC")
                .pip("CKPIN", (obel_tie, "HARD1"))
                .commit();

            let mut bctx = ctx.bel(bels::BUFIO2FB[i]);
            bctx.test_manual("PRESENT", "BUFIO2FB")
                .mode("BUFIO2FB")
                .commit();
            bctx.test_manual("PRESENT", "BUFIO2FB_2CLK")
                .mode("BUFIO2FB_2CLK")
                .commit();
            bctx.mode("BUFIO2FB")
                .test_enum("DIVIDE_BYPASS", &["FALSE", "TRUE"]);

            let obel = bels::BUFIO2_INS;
            for (val, pin) in [
                ("CLKPIN", format!("CLKPIN{ii}", ii = i ^ 1)),
                ("DFB", format!("DFB{i}")),
                ("CFB", format!("CFB0_{i}")),
                ("GTPFB", format!("GTPFB{i}")),
            ] {
                bctx.mode("BUFIO2FB")
                    .mutex("I", val)
                    .test_manual("I", val)
                    .pin("I")
                    .attr("INVERT_INPUTS", "FALSE")
                    .pip("I", (obel, &pin))
                    .commit();
            }
            bctx.mode("BUFIO2FB")
                .mutex("I", "CFB_INVERT")
                .test_manual("I", "CFB_INVERT")
                .pin("I")
                .attr("INVERT_INPUTS", "TRUE")
                .pip("I", (obel, format!("CFB0_{i}")))
                .commit();

            bctx.mode("BUFIO2FB")
                .global_mutex("BUFIO2_CMT_OUT", "TEST_BUFIO2FB")
                .test_manual("CMT_ENABLE", "1")
                .pin("O")
                .pip("CMT", "O")
                .commit();
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::BUFPLL[i]);
            bctx.build()
                .tile_mutex("BUFPLL", "PLAIN")
                .test_manual("PRESENT", "1")
                .mode("BUFPLL")
                .commit();
            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", "PLAIN")
                .test_enum("DIVIDE", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", "PLAIN")
                .test_enum("DATA_RATE", &["SDR", "DDR"]);

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", "PLAIN")
                .no_pin("IOCLK")
                .test_enum("ENABLE_SYNC", &["FALSE", "TRUE"]);

            let obel_out = bels::BUFPLL_OUT;
            let obel_ins = if is_lr {
                bels::BUFPLL_INS_LR
            } else {
                bels::BUFPLL_INS_BT
            };
            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", format!("SINGLE{i}"))
                .global_mutex("PLLCLK", "TEST")
                .attr("ENABLE_SYNC", "FALSE")
                .test_manual("ENABLE_NONE_SYNC", "1")
                .pin("IOCLK")
                .pip((obel_out, format!("PLLCLK{i}")), "IOCLK")
                .commit();

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", format!("SINGLE{i}"))
                .global_mutex("BUFPLL_CLK", "USE")
                .global_mutex("PLLCLK", "TEST")
                .attr("ENABLE_SYNC", "TRUE")
                .pip(
                    "PLLIN",
                    (
                        obel_ins,
                        if is_lr {
                            format!("PLLIN{i}_CMT")
                        } else {
                            "PLLIN0".to_string()
                        },
                    ),
                )
                .test_manual("ENABLE_BOTH_SYNC", "1")
                .pin("IOCLK")
                .pip((obel_out, format!("PLLCLK{i}")), "IOCLK")
                .commit();

            if !is_lr {
                let obel = bels::BUFPLL[i ^ 1];
                for i in 0..6 {
                    bctx.mode("BUFPLL")
                        .tile_mutex("BUFPLL", "PLAIN")
                        .tile_mutex("PLLIN", format!("BUFPLL{i}"))
                        .global_mutex("BUFPLL_CLK", "USE")
                        .mutex("PLLIN", format!("PLLIN{i}"))
                        .attr("ENABLE_SYNC", "FALSE")
                        .pin("PLLIN")
                        .pip((obel, "PLLIN"), (obel_ins, format!("PLLIN{i}")))
                        .test_manual("PLLIN", format!("PLLIN{i}"))
                        .pip("PLLIN", (obel_ins, format!("PLLIN{i}")))
                        .commit();
                }
                for i in 0..3 {
                    bctx.mode("BUFPLL")
                        .tile_mutex("BUFPLL", "PLAIN")
                        .mutex("LOCKED", format!("LOCKED{i}"))
                        .test_manual("LOCKED", format!("LOCKED{i}"))
                        .pin("LOCKED")
                        .pip("LOCKED", (obel_ins, format!("LOCKED{i}")))
                        .commit();
                }
            }

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", format!("SINGLE_{i}"))
                .test_manual("ENABLE", "1")
                .pin("IOCLK")
                .pip((obel_out, format!("PLLCLK{i}")), "IOCLK")
                .commit();
        }
        {
            let mut bctx = ctx.bel(bels::BUFPLL_MCB);
            bctx.build()
                .tile_mutex("BUFPLL", "MCB")
                .test_manual("PRESENT", "1")
                .mode("BUFPLL_MCB")
                .commit();
            bctx.build()
                .tile_mutex("BUFPLL", "MCB")
                .mode("BUFPLL_MCB")
                .test_enum("DIVIDE", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            bctx.build()
                .tile_mutex("BUFPLL", "MCB")
                .mode("BUFPLL_MCB")
                .test_enum("LOCK_SRC", &["LOCK_TO_0", "LOCK_TO_1"]);

            if is_lr {
                let obel = bels::BUFPLL_INS_LR;
                bctx.build()
                    .tile_mutex("BUFPLL", "MCB")
                    .mutex("PLLIN", "GCLK")
                    .mode("BUFPLL_MCB")
                    .test_manual("PLLIN", "GCLK")
                    .pin("PLLIN0")
                    .pin("PLLIN1")
                    .pip("PLLIN0", (obel, "PLLIN0_GCLK"))
                    .pip("PLLIN1", (obel, "PLLIN1_GCLK"))
                    .commit();
                bctx.build()
                    .tile_mutex("BUFPLL", "MCB")
                    .mutex("PLLIN", "CMT")
                    .mode("BUFPLL_MCB")
                    .test_manual("PLLIN", "CMT")
                    .pin("PLLIN0")
                    .pin("PLLIN1")
                    .pip("PLLIN0", (obel, "PLLIN0_CMT"))
                    .pip("PLLIN1", (obel, "PLLIN1_CMT"))
                    .commit();
            }

            let obel = bels::BUFPLL_OUT;
            bctx.build()
                .tile_mutex("BUFPLL", "MCB_OUT0")
                .mode("BUFPLL_MCB")
                .test_manual("ENABLE.0", "1")
                .pin("IOCLK0")
                .pip((obel, "PLLCLK0"), "IOCLK0")
                .commit();
            bctx.build()
                .tile_mutex("BUFPLL", "MCB_OUT1")
                .mode("BUFPLL_MCB")
                .test_manual("ENABLE.1", "1")
                .pin("IOCLK1")
                .pip((obel, "PLLCLK1"), "IOCLK1")
                .commit();
        }
        if !is_lr {
            let n = &tile[4..];
            ctx.build()
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "N")
                .test_manual("MISC", "MISR_ENABLE", "1")
                .global(format!("MISR_{n}M_EN"), "Y")
                .commit();
            ctx.build()
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "Y")
                .test_manual("MISC", "MISR_ENABLE_RESET", "1")
                .global(format!("MISR_{n}M_EN"), "Y")
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "PCILOGICSE");
        let mut bctx = ctx.bel(bels::PCILOGICSE);
        bctx.build()
            .no_global("PCI_CE_DELAY_LEFT")
            .test_manual("PRESENT", "1")
            .mode("PCILOGICSE")
            .commit();
        for val in 2..=31 {
            bctx.build()
                .global("PCI_CE_DELAY_LEFT", format!("TAP{val}"))
                .test_manual("PRESENT", format!("TAP{val}"))
                .mode("PCILOGICSE")
                .commit();
        }
    }
    for (tile, bel) in [
        ("PCI_CE_TRUNK_BUF", bels::PCI_CE_TRUNK_BUF),
        ("PCI_CE_V_BUF", bels::PCI_CE_V_BUF),
        ("PCI_CE_SPLIT", bels::PCI_CE_SPLIT),
        ("PCI_CE_H_BUF", bels::PCI_CE_H_BUF),
    ] {
        if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) {
            let mut bctx = ctx.bel(bel);
            bctx.build()
                .null_bits()
                .test_manual("BUF", "1")
                .pip("PCI_CE_O", "PCI_CE_I")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };
    if devdata_only {
        let tile = "PCILOGICSE";
        let bel = "PCILOGICSE";
        let default = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        let item = ctx.tiledb.item(tile, bel, "PCI_CE_DELAY");
        let val: BitVec = item
            .bits
            .iter()
            .map(|bit| default.bits.contains_key(bit))
            .collect();
        let TileItemKind::Enum { ref values } = item.kind else {
            unreachable!()
        };
        for (k, v) in values {
            if *v == val {
                ctx.insert_device_data("PCI_CE_DELAY", k.clone());
                break;
            }
        }
        return;
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..16 {
            ctx.collect_bit(tile, bel, &format!("GCLK{i}_O_D"), "1");
            ctx.collect_bit(tile, bel, &format!("GCLK{i}_O_U"), "1");
        }
        // TODO: mask bits
    }
    {
        let tile = "HCLK_ROW";
        for i in 0..16 {
            for we in ['W', 'E'] {
                let bel = format!("BUFH_{we}{i}");
                ctx.collect_enum_default(tile, &bel, "I", &["BUFG", "CMT"], "NONE");
            }
        }
    }
    {
        let tile = "CLKC";
        for i in 0..16 {
            let bel = format!("BUFGMUX{i}");
            ctx.state
                .get_diff(tile, &bel, "PRESENT", "1")
                .assert_empty();
            ctx.collect_enum(tile, &bel, "CLK_SEL_TYPE", &["SYNC", "ASYNC"]);
            ctx.collect_enum_bool(tile, &bel, "DISABLE_ATTR", "LOW", "HIGH");
            ctx.collect_inv(tile, &bel, "S");
        }
        let bel = "CLKC";
        for i in 0..16 {
            ctx.collect_enum_default(
                tile,
                bel,
                &format!("IMUX{i}"),
                &[
                    &format!("CKPIN_H{i}"),
                    &format!("CKPIN_V{i}"),
                    &format!("CMT_D{i}"),
                    &format!("CMT_U{i}"),
                ],
                "NONE",
            );
        }
        for out in [
            "OUTL_CLKOUT0",
            "OUTL_CLKOUT1",
            "OUTR_CLKOUT0",
            "OUTR_CLKOUT1",
        ] {
            let item = ctx.extract_enum(
                tile,
                "CLKC_BUFPLL",
                out,
                &[
                    "PLL0U_CLKOUT0",
                    "PLL0U_CLKOUT1",
                    "PLL1U_CLKOUT0",
                    "PLL1U_CLKOUT1",
                    "PLL0D_CLKOUT0",
                    "PLL0D_CLKOUT1",
                    "PLL1D_CLKOUT0",
                    "PLL1D_CLKOUT1",
                ],
            );
            ctx.tiledb.insert(tile, bel, out, item);
        }
        for out in ["OUTD_CLKOUT0", "OUTD_CLKOUT1"] {
            let item = ctx.extract_enum(
                tile,
                "CLKC_BUFPLL",
                out,
                &[
                    "PLL0U_CLKOUT0",
                    "PLL0U_CLKOUT1",
                    "PLL1U_CLKOUT0",
                    "PLL1U_CLKOUT1",
                ],
            );
            ctx.tiledb.insert(tile, bel, out, item);
        }
        for out in ["OUTU_CLKOUT0", "OUTU_CLKOUT1"] {
            let item = ctx.extract_enum(
                tile,
                "CLKC_BUFPLL",
                out,
                &[
                    "PLL0D_CLKOUT0",
                    "PLL0D_CLKOUT1",
                    "PLL1D_CLKOUT0",
                    "PLL1D_CLKOUT1",
                ],
            );
            ctx.tiledb.insert(tile, bel, out, item);
        }
        for out in [
            "OUTL_LOCKED0",
            "OUTL_LOCKED1",
            "OUTR_LOCKED0",
            "OUTR_LOCKED1",
        ] {
            let item = ctx.extract_enum(
                tile,
                "CLKC_BUFPLL",
                out,
                &[
                    "PLL0D_LOCKED",
                    "PLL1D_LOCKED",
                    "PLL0U_LOCKED",
                    "PLL1U_LOCKED",
                ],
            );
            ctx.tiledb.insert(tile, bel, out, item);
        }
        let item = ctx.extract_enum(
            tile,
            "CLKC_BUFPLL",
            "OUTD_LOCKED",
            &["PLL0U_LOCKED", "PLL1U_LOCKED"],
        );
        ctx.tiledb.insert(tile, bel, "OUTD_LOCKED", item);
        let item = ctx.extract_enum(
            tile,
            "CLKC_BUFPLL",
            "OUTU_LOCKED",
            &["PLL0D_LOCKED", "PLL1D_LOCKED"],
        );
        ctx.tiledb.insert(tile, bel, "OUTU_LOCKED", item);
    }
    for tile in [
        "DCM_BUFPLL_BUF_S",
        "DCM_BUFPLL_BUF_S_MID",
        "DCM_BUFPLL_BUF_N",
        "DCM_BUFPLL_BUF_N_MID",
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = tile;
        for attr in [
            "PLL0_CLKOUT0",
            "PLL0_CLKOUT1",
            "PLL1_CLKOUT0",
            "PLL1_CLKOUT1",
            "CLKC_CLKOUT0",
            "CLKC_CLKOUT1",
        ] {
            ctx.collect_bit(tile, bel, attr, "1");
        }
        for attr in ["PLL0_LOCKED", "PLL1_LOCKED", "CLKC_LOCKED"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
    }
    if ctx.has_tile("PLL_BUFPLL_OUT0") {
        let tile = "PLL_BUFPLL_OUT0";
        let bel = "PLL_BUFPLL";
        for attr in [
            "PLL0_CLKOUT0_D",
            "PLL0_CLKOUT1_D",
            "PLL0_CLKOUT0_U",
            "PLL0_CLKOUT1_U",
            "PLL1_CLKOUT0",
            "PLL1_CLKOUT1",
            "CLKC_CLKOUT0",
            "CLKC_CLKOUT1",
        ] {
            ctx.collect_bit(tile, bel, attr, "1");
        }
        for attr in ["PLL0_LOCKED_D", "PLL0_LOCKED_U"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
    }
    {
        let tile = "PLL_BUFPLL_OUT1";
        let bel = "PLL_BUFPLL";
        for attr in [
            "PLL0_CLKOUT0",
            "PLL0_CLKOUT1",
            "PLL1_CLKOUT0_D",
            "PLL1_CLKOUT1_D",
            "PLL1_CLKOUT0_U",
            "PLL1_CLKOUT1_U",
            "CLKC_CLKOUT0",
            "CLKC_CLKOUT1",
        ] {
            ctx.collect_bit(tile, bel, attr, "1");
        }
        for attr in ["PLL1_LOCKED_D", "PLL1_LOCKED_U"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
    }
    for tile in ["PLL_BUFPLL_B", "PLL_BUFPLL_T"] {
        if ctx.has_tile(tile) {
            let bel = "PLL_BUFPLL";
            for attr in [
                "PLL0_CLKOUT0",
                "PLL0_CLKOUT1",
                "PLL1_CLKOUT0",
                "PLL1_CLKOUT1",
                "CLKC_CLKOUT0",
                "CLKC_CLKOUT1",
            ] {
                ctx.collect_bit(tile, bel, attr, "1");
            }
        }
    }
    for (tile, is_lr) in [
        ("REG_B", false),
        ("REG_T", false),
        ("REG_L", true),
        ("REG_R", true),
    ] {
        for i in 0..8 {
            let bel = format!("BUFIO2_{i}");
            let bel_fb = format!("BUFIO2FB_{i}");
            let bel = &bel;
            let bel_fb = &bel_fb;
            ctx.state
                .get_diff(tile, bel, "PRESENT", "BUFIO2")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, bel, "CMT_ENABLE", "1");
            assert_eq!(diff, ctx.state.get_diff(tile, bel_fb, "CMT_ENABLE", "1"));
            ctx.tiledb.insert(tile, bel, "CMT_ENABLE", xlat_bit(diff));
            ctx.collect_bit(tile, bel, "IOCLK_ENABLE", "1");
            ctx.collect_enum(tile, bel, "CKPIN", &["VCC", "DIVCLK", "CLKPIN"]);
            ctx.collect_enum_bool(tile, bel, "R_EDGE", "FALSE", "TRUE");
            ctx.collect_enum_bool(tile, bel, "DIVIDE_BYPASS", "FALSE", "TRUE");
            let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "BUFIO2_2CLK");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "R_EDGE"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DIVIDE_BYPASS"), false, true);
            ctx.tiledb.insert(tile, bel, "ENABLE_2CLK", xlat_bit(diff));

            let mut pos_edge = vec![];
            let mut pos_bits = HashSet::new();
            let mut neg_edge = vec![];
            let mut neg_bits = HashSet::new();
            for i in 1..=8 {
                let val = format!("{i}");
                let diff = ctx.state.get_diff(tile, bel, "POS_EDGE", &val);
                pos_bits.extend(diff.bits.keys().copied());
                pos_edge.push((format!("POS_EDGE_{i}"), diff));
                let diff = ctx.state.get_diff(tile, bel, "NEG_EDGE", &val);
                neg_bits.extend(diff.bits.keys().copied());
                neg_edge.push((format!("NEG_EDGE_{i}"), diff));
            }
            let mut divide = vec![];
            for i in 1..=8 {
                let val = format!("{i}");
                let mut diff = ctx.state.get_diff(tile, bel, "DIVIDE", &val);
                if matches!(i, 2 | 4 | 6 | 8) {
                    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "R_EDGE"), true, false);
                }
                let diff2 = ctx.state.get_diff(tile, bel, "DIVIDE.2CLK", &val);
                assert_eq!(diff, diff2);
                let pos = diff.split_bits(&pos_bits);
                let neg = diff.split_bits(&neg_bits);
                pos_edge.push((format!("DIVIDE_{i}"), pos));
                neg_edge.push((format!("DIVIDE_{i}"), neg));
                divide.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, "POS_EDGE", xlat_enum(pos_edge));
            ctx.tiledb
                .insert(tile, bel, "NEG_EDGE", xlat_enum(neg_edge));
            ctx.tiledb.insert(tile, bel, "DIVIDE", xlat_enum(divide));

            let enable = ctx.state.peek_diff(tile, bel, "I", "CLKPIN0").clone();
            let mut diffs = vec![];
            for val in [
                "CLKPIN0", "CLKPIN1", "CLKPIN4", "CLKPIN5", "DFB", "DQS0", "DQS2", "GTPCLK",
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, "I", val);
                diff = diff.combine(&!&enable);
                diffs.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, "I", xlat_enum_ocd(diffs, OcdMode::BitOrder));
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(enable));
            ctx.collect_enum_ocd(
                tile,
                bel,
                "IB",
                &[
                    "CLKPIN0", "CLKPIN1", "CLKPIN4", "CLKPIN5", "DFB", "DQS0", "DQS2",
                ],
                OcdMode::BitOrder,
            );

            ctx.state
                .get_diff(tile, bel_fb, "PRESENT", "BUFIO2FB")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel_fb, "DIVIDE_BYPASS", "TRUE")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, bel_fb, "DIVIDE_BYPASS", "FALSE");
            ctx.tiledb
                .insert(tile, bel, "FB_DIVIDE_BYPASS", xlat_bit_wide(!diff));

            let enable = ctx.state.peek_diff(tile, bel_fb, "I", "CLKPIN").clone();
            let mut diffs = vec![];
            for val in ["CLKPIN", "DFB", "CFB", "CFB_INVERT", "GTPFB"] {
                let mut diff = ctx.state.get_diff(tile, bel_fb, "I", val);
                diff = diff.combine(&!&enable);
                diffs.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, "FB_I", xlat_enum_ocd(diffs, OcdMode::BitOrder));
            ctx.tiledb.insert(tile, bel, "FB_ENABLE", xlat_bit(enable));

            let mut present = ctx.state.get_diff(tile, bel_fb, "PRESENT", "BUFIO2FB_2CLK");
            present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "FB_DIVIDE_BYPASS"), 0, 0xf);
            present.apply_enum_diff(ctx.tiledb.item(tile, bel, "FB_I"), "CFB", "CLKPIN");
            present.assert_empty();
        }
        for val in ["1", "2", "3", "4", "5", "6", "7", "8"] {
            let diff = ctx.state.get_diff(tile, "BUFPLL_MCB", "DIVIDE", val);
            let diff0 = ctx.state.peek_diff(tile, "BUFPLL0", "DIVIDE", val);
            let diff1 = ctx.state.peek_diff(tile, "BUFPLL1", "DIVIDE", val);
            assert_eq!(diff, diff0.combine(diff1));
        }
        for i in 0..2 {
            let bel = format!("BUFPLL{i}");
            let bel = &bel;
            ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
            ctx.collect_enum(tile, bel, "DATA_RATE", &["SDR", "DDR"]);
            ctx.collect_enum_ocd(
                tile,
                bel,
                "DIVIDE",
                &["1", "2", "3", "4", "5", "6", "7", "8"],
                OcdMode::BitOrder,
            );
            let enable = ctx.extract_bit(tile, bel, "ENABLE", "1");
            let mut diff = ctx.state.get_diff(tile, bel, "ENABLE_NONE_SYNC", "1");
            diff.apply_bit_diff(&enable, true, false);
            ctx.tiledb
                .insert(tile, bel, "ENABLE_NONE_SYNC", xlat_bit_wide(diff));
            let mut diff = ctx.state.get_diff(tile, bel, "ENABLE_BOTH_SYNC", "1");
            diff.apply_bit_diff(&enable, true, false);
            ctx.tiledb
                .insert(tile, bel, "ENABLE_BOTH_SYNC", xlat_bit_wide(diff));
            ctx.tiledb.insert(tile, "BUFPLL_COMMON", "ENABLE", enable);
            ctx.collect_enum_bool(tile, bel, "ENABLE_SYNC", "FALSE", "TRUE");

            if !is_lr {
                ctx.collect_enum(tile, bel, "LOCKED", &["LOCKED0", "LOCKED1", "LOCKED2"]);
                ctx.collect_enum(
                    tile,
                    bel,
                    "PLLIN",
                    &["PLLIN0", "PLLIN1", "PLLIN2", "PLLIN3", "PLLIN4", "PLLIN5"],
                );
            }
        }
        {
            let bel = "BUFPLL_MCB";
            let item = ctx.extract_bit(tile, bel, "ENABLE.0", "1");
            ctx.tiledb.insert(tile, "BUFPLL_COMMON", "ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "ENABLE.1", "1");
            ctx.tiledb.insert(tile, "BUFPLL_COMMON", "ENABLE", item);
            if is_lr {
                let item = ctx.extract_enum(tile, bel, "PLLIN", &["GCLK", "CMT"]);
                ctx.tiledb.insert(tile, "BUFPLL_COMMON", "PLLIN", item);
            }
            let mut diff0 = ctx.state.get_diff(tile, bel, "LOCK_SRC", "LOCK_TO_0");
            diff0.apply_bit_diff(ctx.tiledb.item(tile, "BUFPLL1", "ENABLE_SYNC"), false, true);
            let mut diff1 = ctx.state.get_diff(tile, bel, "LOCK_SRC", "LOCK_TO_1");
            diff1.apply_bit_diff(ctx.tiledb.item(tile, "BUFPLL0", "ENABLE_SYNC"), false, true);
            ctx.tiledb.insert(
                tile,
                bel,
                "LOCK_SRC",
                xlat_enum(vec![("LOCK_TO_0", diff0), ("LOCK_TO_1", diff1)]),
            );
            let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, "BUFPLL0", "ENABLE_BOTH_SYNC"), 7, 0);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, "BUFPLL1", "ENABLE_BOTH_SYNC"), 7, 0);
            diff.assert_empty();
        }
        if !is_lr {
            let bel = "MISC";
            let has_gt = match tile {
                "REG_B" => matches!(edev.chip.gts, Gts::Quad(_, _)),
                "REG_T" => edev.chip.gts != Gts::None,
                _ => unreachable!(),
            };
            if has_gt && !ctx.device.name.starts_with("xa") {
                ctx.collect_bit(tile, bel, "MISR_ENABLE", "1");
                let mut diff = ctx.state.get_diff(tile, bel, "MISR_ENABLE_RESET", "1");
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "MISR_ENABLE"), true, false);
                ctx.tiledb.insert(tile, bel, "MISR_RESET", xlat_bit(diff));
            } else {
                // they're sometimes working, sometimes not, in nonsensical ways; just kill them
                ctx.state.get_diff(tile, bel, "MISR_ENABLE", "1");
                ctx.state.get_diff(tile, bel, "MISR_ENABLE_RESET", "1");
            }
        }
    }
    {
        let tile = "PCILOGICSE";
        let bel = "PCILOGICSE";
        let default = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        let mut diffs = vec![];
        for i in 2..=31 {
            let val = format!("TAP{i}");
            let diff = ctx.state.get_diff(tile, bel, "PRESENT", &val);
            if diff == default {
                ctx.insert_device_data("PCI_CE_DELAY", val.clone());
            }
            diffs.push((val, diff));
        }
        diffs.reverse();
        let mut bits: HashSet<_> = diffs[0].1.bits.keys().copied().collect();
        for (_, diff) in &diffs {
            bits.retain(|b| diff.bits.contains_key(b));
        }
        assert_eq!(bits.len(), 1);
        for (_, diff) in &mut diffs {
            let enable = diff.split_bits(&bits);
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(enable));
        }
        ctx.tiledb
            .insert(tile, bel, "PCI_CE_DELAY", xlat_enum(diffs));
    }
    for (tile, frame, bit, target) in [
        // used in CLEXL (incl. spine) and DSP columns; also used on PCIE sides and left GTP side
        ("HCLK_CLEXL", 16, 0, 21),
        ("HCLK_CLEXL", 17, 0, 22),
        ("HCLK_CLEXL", 18, 0, 23),
        ("HCLK_CLEXL", 19, 0, 24),
        ("HCLK_CLEXL", 16, 1, 26),
        ("HCLK_CLEXL", 17, 1, 27),
        ("HCLK_CLEXL", 18, 1, 28),
        ("HCLK_CLEXL", 19, 1, 29),
        // used in CLEXM columns
        ("HCLK_CLEXM", 16, 0, 21),
        ("HCLK_CLEXM", 17, 0, 22),
        ("HCLK_CLEXM", 18, 0, 24),
        ("HCLK_CLEXM", 19, 0, 25),
        ("HCLK_CLEXM", 16, 1, 27),
        ("HCLK_CLEXM", 17, 1, 28),
        ("HCLK_CLEXM", 18, 1, 29),
        ("HCLK_CLEXM", 19, 1, 30),
        // used in IOI columns
        ("HCLK_IOI", 16, 0, 25),
        ("HCLK_IOI", 18, 0, 23),
        ("HCLK_IOI", 19, 0, 24),
        ("HCLK_IOI", 16, 1, 21),
        ("HCLK_IOI", 17, 1, 27),
        ("HCLK_IOI", 18, 1, 28),
        ("HCLK_IOI", 19, 1, 29),
        // used on right GTP side
        ("HCLK_GTP", 16, 0, 25),
        ("HCLK_GTP", 17, 0, 22),
        ("HCLK_GTP", 18, 0, 23),
        ("HCLK_GTP", 19, 0, 24),
        // BRAM columns do not have masking
    ] {
        ctx.tiledb.insert(
            tile,
            "GLUTMASK",
            format!("FRAME{target}"),
            TileItem::from_bit(TileBit::new(0, frame, bit), false),
        )
    }
}
