use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{DieId, TileCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_bit_wide, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::bels;
use prjcombine_virtex4::tslots;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            relation::{Delta, FixedRelation, TileRelation},
        },
    },
};

#[derive(Copy, Clone, Debug)]
struct ClkTerm;

impl TileRelation for ClkTerm {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];
        let row = if tcrd.row < chip.row_bufg() {
            chip.rows().first().unwrap()
        } else {
            chip.rows().last().unwrap()
        };
        Some(tcrd.with_row(row).tile(tslots::HROW))
    }
}

#[derive(Copy, Clone, Debug)]
struct ClkHrow;

impl TileRelation for ClkHrow {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(tcrd.with_col(edev.col_clk).tile(tslots::HROW))
    }
}

#[derive(Copy, Clone, Debug)]
struct HclkTerm(DirH);

impl TileRelation for HclkTerm {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = match self.0 {
            DirH::W => edev.chips[tcrd.die].columns.first_id().unwrap(),
            DirH::E => edev.chips[tcrd.die].columns.last_id().unwrap(),
        };
        Some(tcrd.with_col(col).tile(tslots::HROW))
    }
}

#[derive(Copy, Clone, Debug)]
struct Rclk;

impl TileRelation for Rclk {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = if tcrd.col <= edev.col_clk {
            edev.col_lio.unwrap()
        } else {
            edev.col_rio.unwrap()
        };
        Some(tcrd.with_col(col).tile(tslots::HCLK_BEL))
    }
}

#[derive(Copy, Clone, Debug)]
struct Ioclk(DirV);

impl TileRelation for Ioclk {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];
        let row = match self.0 {
            DirV::S => {
                if tcrd.col == edev.col_cfg && tcrd.row == chip.row_bufg() + 8 {
                    return None;
                }
                if tcrd.row.to_idx() < 16 {
                    return None;
                }
                tcrd.row - 16
            }
            DirV::N => {
                if tcrd.col == edev.col_cfg && tcrd.row == chip.row_bufg() - 8 {
                    return None;
                }
                if tcrd.row.to_idx() + 16 >= chip.rows().len() {
                    return None;
                }
                tcrd.row + 16
            }
        };
        Some(tcrd.with_row(row).tile(tslots::HCLK_BEL))
    }
}

#[derive(Clone, Debug)]
struct ExtraHclkDcmAttr(DirV, &'static str, String, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraHclkDcmAttr {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let rows = match self.0 {
            DirV::N => [tcrd.row, tcrd.row + 4],
            DirV::S => [tcrd.row - 8, tcrd.row - 4],
        };
        let mut sad = true;
        for row in rows {
            if let Some(ntcrd) = backend
                .edev
                .find_tile_by_class(tcrd.with_row(row), |kind| kind == self.1)
            {
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: self.1.into(),
                        bel: if self.1 == "DCM" { "DCM0" } else { self.1 }.into(),
                        attr: self.2.clone(),
                        val: self.3.into(),
                    },
                    rects: edev.tile_bits(ntcrd),
                });
                sad = false;
            }
        }
        Some((fuzzer, sad))
    }
}

#[derive(Clone, Debug)]
struct ExtraMgtRepeaterAttr(DirH, String, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraMgtRepeaterAttr {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        for &col in &edev.chips[DieId::from_idx(0)].cols_vbrk {
            if (col < edev.col_cfg) == (self.0 == DirH::W) {
                let rcol = if self.0 == DirH::W { col } else { col - 1 };
                let ntcrd = tcrd.with_col(rcol).tile(tslots::CLK);
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: "HCLK_MGT_REPEATER".into(),
                        bel: "HCLK_MGT_REPEATER".into(),
                        attr: self.1.clone(),
                        val: self.2.into(),
                    },
                    rects: edev.tile_bits(ntcrd),
                });
            }
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    for tile in ["CLK_IOB_B", "CLK_IOB_T"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::CLK_IOB);
        let giob: Vec<_> = (0..16).map(|i| format!("GIOB{i}")).collect();
        for i in 0..16 {
            bctx.build()
                .global_mutex("GIOB", "TEST")
                .tile_mutex("GIOB_TEST", &giob[i])
                .extra_tile_attr(ClkTerm, "CLK_TERM", "GIOB_ENABLE", "1")
                .test_manual(format!("BUF.GIOB{i}"), "1")
                .pip(&giob[i], format!("PAD_BUF{i}"))
                .commit();
        }
        let clk_dcm = match tile {
            "CLK_IOB_B" => Delta::new(0, -8, "CLK_DCM_B"),
            "CLK_IOB_T" => Delta::new(0, 16, "CLK_DCM_T"),
            _ => unreachable!(),
        };
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mout = format!("MUXBUS_O{i}");
            let min = format!("MUXBUS_I{i}");
            for j in 0..16 {
                bctx.build()
                    .global_mutex("CLK_IOB_MUXBUS", "TEST")
                    .tile_mutex(&mout, &giob[j])
                    .test_manual(&mux, &giob[j])
                    .pip(&mout, format!("PAD_BUF{j}"))
                    .commit();
            }
            bctx.build()
                .global_mutex("CLK_IOB_MUXBUS", "TEST")
                .tile_mutex(&mout, &min)
                .related_pip(
                    clk_dcm.clone(),
                    (bels::CLK_DCM, &mout),
                    (bels::CLK_DCM, "DCM0"),
                )
                .related_tile_mutex(clk_dcm.clone(), "MUXBUS", "USE")
                .test_manual(&mux, "PASS")
                .pip(&mout, &min)
                .commit();
        }
    }
    for tile in ["CLK_DCM_B", "CLK_DCM_T"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::CLK_DCM);
        let dcm: Vec<_> = (0..24).map(|i| format!("DCM{i}")).collect();
        let clk_dcm = match tile {
            "CLK_DCM_B" => Delta::new(0, -8, "CLK_DCM_B"),
            "CLK_DCM_T" => Delta::new(0, 8, "CLK_DCM_T"),
            _ => unreachable!(),
        };
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mout = format!("MUXBUS_O{i}");
            let min = format!("MUXBUS_I{i}");
            for j in 0..24 {
                bctx.build()
                    .tile_mutex("MUXBUS", "TEST")
                    .tile_mutex(&mout, &dcm[j])
                    .test_manual(&mux, &dcm[j])
                    .pip(&mout, &dcm[j])
                    .commit();
            }
            let has_other = if tile == "CLK_DCM_T" {
                edev.chips
                    .values()
                    .any(|grid| grid.regs - grid.reg_clk.to_idx() > 2)
            } else {
                edev.chips.values().any(|grid| grid.reg_clk.to_idx() > 2)
            };
            if has_other {
                bctx.build()
                    .tile_mutex("MUXBUS", "TEST")
                    .tile_mutex(&mout, &min)
                    .related_pip(
                        clk_dcm.clone(),
                        (bels::CLK_DCM, &mout),
                        (bels::CLK_DCM, "DCM0"),
                    )
                    .related_tile_mutex(clk_dcm.clone(), "MUXBUS", "USE")
                    .test_manual(&mux, "PASS")
                    .pip(&mout, &min)
                    .commit();
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_HROW");
        let mut bctx = ctx.bel(bels::CLK_HROW);
        let gclk: Vec<_> = (0..32).map(|i| format!("GCLK{i}")).collect();
        for (dir, lr) in [(DirH::W, 'L'), (DirH::E, 'R')] {
            for i in 0..8 {
                let hclk = format!("HCLK_{lr}{i}");
                for j in 0..32 {
                    let bel_bufg = bels::BUFGCTRL[j];
                    let cfg = FixedRelation(edev.tile_cfg(DieId::from_idx(0)));
                    bctx.build()
                        .global_mutex("BUFGCTRL_OUT", "USE")
                        .tile_mutex("MODE", "TEST")
                        .tile_mutex("IN", &gclk[j])
                        .tile_mutex("OUT", &hclk)
                        .related_pip(cfg, (bel_bufg, "GCLK"), (bel_bufg, "O"))
                        .extra_tile_attr(HclkTerm(dir), "HCLK_TERM", "HCLK_ENABLE", "1")
                        .test_manual(format!("MUX.HCLK_{lr}{i}"), &gclk[j])
                        .pip(&hclk, &gclk[j])
                        .commit();
                }
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK");
        let mut bctx = ctx.bel(bels::HCLK);
        for i in 0..8 {
            let hclk_i = format!("HCLK_I{i}");
            let hclk_o = format!("HCLK_O{i}");
            let hclk_l = format!("HCLK_L{i}");
            let hclk_r = format!("HCLK_R{i}");
            let obel = bels::CLK_HROW;
            bctx.build()
                .global_mutex("BUFGCTRL_OUT", "USE")
                .related_tile_mutex(ClkHrow, "MODE", "USE")
                .related_pip(ClkHrow, (obel, hclk_l), (obel, "GCLK0"))
                .related_pip(ClkHrow, (obel, hclk_r), (obel, "GCLK0"))
                .test_manual(format!("BUF.HCLK{i}"), "1")
                .pip(hclk_o, hclk_i)
                .commit();
        }
        for i in 0..2 {
            let rclk_i = format!("RCLK_I{i}");
            let rclk_o = format!("RCLK_O{i}");
            bctx.build()
                .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                .related_pip(
                    Rclk,
                    (bels::RCLK, format!("RCLK{i}")),
                    (bels::RCLK, "VRCLK0"),
                )
                .test_manual(format!("BUF.RCLK{i}"), "1")
                .pip(rclk_o, rclk_i)
                .commit();
        }
    }
    for tile in [
        "HCLK_IOIS_LVDS",
        "HCLK_IOIS_DCI",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let tcid = backend.edev.db.get_tile_class(tile);
        let tcls = &backend.edev.db[tcid];
        if tcls.bels.contains_id(bels::RCLK) {
            let mut bctx = ctx.bel(bels::RCLK);
            for opin in ["RCLK0", "RCLK1"] {
                for ipin in [
                    "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                ] {
                    bctx.build()
                        .tile_mutex("RCLK_MODE", "TEST")
                        .tile_mutex(opin, ipin)
                        .test_manual(format!("MUX.{opin}"), ipin)
                        .pip(opin, ipin)
                        .commit();
                }
            }
            let obel_rclk = bels::RCLK;
            for bel in [bels::BUFR0, bels::BUFR1] {
                let mut bctx = ctx.bel(bel);
                bctx.test_manual("PRESENT", "1").mode("BUFR").commit();
                bctx.mode("BUFR")
                    .test_manual("ENABLE", "1")
                    .pin("O")
                    .commit();
                bctx.mode("BUFR").test_enum(
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                for ckint in ["CKINT0", "CKINT1"] {
                    bctx.mode("BUFR")
                        .mutex("MUX.I", ckint)
                        .test_manual("MUX.I", ckint)
                        .pip("I", (obel_rclk, ckint))
                        .commit();
                }
                for (obel, obel_name) in [(bels::BUFIO0, "BUFIO0"), (bels::BUFIO1, "BUFIO1")] {
                    bctx.mode("BUFR")
                        .mutex("MUX.I", obel_name)
                        .test_manual("MUX.I", obel_name)
                        .pip("I", (obel, "O"))
                        .commit();
                }
            }
        }
        {
            let mut bctx = ctx.bel(bels::IOCLK);
            for i in 0..8 {
                let hclk_i = format!("HCLK_I{i}");
                let hclk_o = format!("HCLK_O{i}");
                let hclk_l = format!("HCLK_L{i}");
                let hclk_r = format!("HCLK_R{i}");
                let obel = bels::CLK_HROW;
                bctx.build()
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow, "MODE", "USE")
                    .related_pip(ClkHrow, (obel, hclk_l), (obel, "GCLK0"))
                    .related_pip(ClkHrow, (obel, hclk_r), (obel, "GCLK0"))
                    .test_manual(format!("BUF.HCLK{i}"), "1")
                    .pip(hclk_o, hclk_i)
                    .commit();
            }
            for i in 0..2 {
                let rclk_i = format!("RCLK_I{i}");
                let rclk_o = format!("RCLK_O{i}");
                bctx.build()
                    .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                    .related_pip(
                        Rclk,
                        (bels::RCLK, format!("RCLK{i}")),
                        (bels::RCLK, "VRCLK0"),
                    )
                    .test_manual(format!("BUF.RCLK{i}"), "1")
                    .pip(rclk_o, rclk_i)
                    .commit();
            }

            for (obel, vioclk) in [(bels::BUFIO0, "VIOCLK0"), (bels::BUFIO1, "VIOCLK1")] {
                bctx.build()
                    .tile_mutex("VIOCLK", vioclk)
                    .test_manual(format!("BUF.{vioclk}"), "1")
                    .pip(vioclk, (obel, "O"))
                    .commit();
            }
            let (has_s, has_n) = match tile {
                "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => (true, true),
                "HCLK_DCMIOB" | "HCLK_CENTER_ABOVE_CFG" => (false, true),
                "HCLK_IOBDCM" => (true, false),
                "HCLK_CENTER" => (
                    true,
                    edev.chips[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                for (o, i, oi, oo) in [
                    ("IOCLK_N0", "VIOCLK_N0", "VIOCLK0", "IOCLK0"),
                    ("IOCLK_N1", "VIOCLK_N1", "VIOCLK1", "IOCLK1"),
                ] {
                    bctx.build()
                        .related_tile_mutex(Ioclk(DirV::S), "VIOCLK", "USE")
                        .related_pip(Ioclk(DirV::S), oo, oi)
                        .test_manual(format!("BUF.{o}"), "1")
                        .pip(o, i)
                        .commit();
                }
            }
            if has_n {
                for (o, i, oi, oo) in [
                    ("IOCLK_S0", "VIOCLK_S0", "VIOCLK0", "IOCLK0"),
                    ("IOCLK_S1", "VIOCLK_S1", "VIOCLK1", "IOCLK1"),
                ] {
                    bctx.build()
                        .related_tile_mutex(Ioclk(DirV::N), "VIOCLK", "USE")
                        .related_pip(Ioclk(DirV::N), oo, oi)
                        .test_manual(format!("BUF.{o}"), "1")
                        .pip(o, i)
                        .commit();
                }
            }
        }
        {
            let mut bctx = ctx.bel(bels::IDELAYCTRL);
            bctx.test_manual("ENABLE", "1").mode("IDELAYCTRL").commit();
            for i in 0..8 {
                let hclk = format!("HCLK{i}");
                let hclk_o = format!("HCLK_O{i}");
                bctx.build()
                    .mutex("REFCLK", &hclk)
                    .test_manual("MUX.REFCLK", &hclk)
                    .pip("REFCLK", (bels::IOCLK, hclk_o))
                    .commit();
            }
        }
    }

    let ccm = backend.edev.db.get_tile_class("CCM");
    let num_ccms = backend.edev.tile_index[ccm].len();
    let sysmon = backend.edev.db.get_tile_class("SYSMON");
    let has_hclk_dcm = !backend.edev.tile_index[sysmon].is_empty();
    let has_gt = edev.col_lgt.is_some();
    for (tile, bel) in [
        ("HCLK_DCM", bels::HCLK_DCM),
        ("HCLK_DCMIOB", bels::HCLK_DCM_S),
        ("HCLK_IOBDCM", bels::HCLK_DCM_N),
    ] {
        if tile == "HCLK_DCM" && !has_hclk_dcm {
            continue;
        }
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bel);
        let rel_dcm_ccm = |dir| {
            let dy = match dir {
                DirV::N => 0,
                DirV::S => -8,
            };
            Delta::new_any(0, dy, &["CCM", "DCM"])
        };
        for dir in [DirV::S, DirV::N] {
            if dir == DirV::S && bel == bels::HCLK_DCM_N {
                continue;
            }
            if dir == DirV::N && bel == bels::HCLK_DCM_S {
                continue;
            }
            let ud = match dir {
                DirV::S => 'D',
                DirV::N => 'U',
            };
            for i in 0..16 {
                let mut builder = bctx
                    .build()
                    .global_mutex("HCLK_DCM", "TEST")
                    .tile_mutex("HCLK_DCM", format!("GIOB_O_{ud}{i}"));
                if tile == "HCLK_DCM" || num_ccms < 4 {
                    builder =
                        builder.prop(ExtraHclkDcmAttr(dir, "DCM", format!("ENABLE.GIOB{i}"), "1"));
                }
                if tile != "HCLK_DCM" && num_ccms != 0 {
                    builder =
                        builder.prop(ExtraHclkDcmAttr(dir, "CCM", format!("ENABLE.GIOB{i}"), "1"));
                }
                builder
                    .test_manual(format!("BUF.GIOB_{ud}{i}"), "1")
                    .pip(format!("GIOB_O_{ud}{i}"), format!("GIOB_I{i}"))
                    .commit();
            }
            for i in 0..8 {
                let mut builder = bctx
                    .build()
                    .global_mutex("HCLK_DCM", "TEST")
                    .tile_mutex("HCLK_DCM", format!("HCLK_O_{ud}{i}"))
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow, "MODE", "USE")
                    .related_pip(
                        ClkHrow,
                        (bels::CLK_HROW, format!("HCLK_L{i}")),
                        (bels::CLK_HROW, "GCLK0"),
                    )
                    .has_related(rel_dcm_ccm(dir));
                if tile == "HCLK_DCM" || num_ccms < 4 {
                    builder =
                        builder.prop(ExtraHclkDcmAttr(dir, "DCM", format!("ENABLE.HCLK{i}"), "1"));
                }
                if tile != "HCLK_DCM" && num_ccms != 0 {
                    builder =
                        builder.prop(ExtraHclkDcmAttr(dir, "CCM", format!("ENABLE.HCLK{i}"), "1"));
                }
                builder
                    .test_manual(format!("BUF.HCLK_{ud}{i}"), "1")
                    .pip(format!("HCLK_O_{ud}{i}"), format!("HCLK_I{i}"))
                    .commit();
            }
            if has_gt || tile == "HCLK_DCM" {
                for i in 0..4 {
                    if tile == "HCLK_DCM" {
                        let mut builder = bctx
                            .build()
                            .global_mutex("HCLK_DCM", "TEST")
                            .tile_mutex("HCLK_DCM", format!("MGT_O_{ud}{i}"))
                            .has_related(rel_dcm_ccm(DirV::S))
                            .has_related(rel_dcm_ccm(DirV::N));
                        if tile == "HCLK_DCM" || num_ccms < 4 {
                            builder = builder.prop(ExtraHclkDcmAttr(
                                dir,
                                "DCM",
                                format!("ENABLE.MGT{i}"),
                                "1",
                            ));
                        }
                        if tile != "HCLK_DCM" && num_ccms != 0 {
                            builder = builder.prop(ExtraHclkDcmAttr(
                                dir,
                                "CCM",
                                format!("ENABLE.MGT{i}"),
                                "1",
                            ));
                        }
                        builder
                            .test_manual(format!("BUF.MGT_{ud}{i}"), "1")
                            .pip(format!("MGT_O_{ud}{i}"), format!("MGT{i}"))
                            .commit();
                    } else {
                        let mut builder = bctx
                            .build()
                            .global_mutex("MGT_OUT", "USE")
                            .global_mutex("HCLK_DCM", "TEST")
                            .tile_mutex("HCLK_DCM", format!("MGT_O_{ud}{i}"))
                            .prop(ExtraMgtRepeaterAttr(
                                if i < 2 { DirH::W } else { DirH::E },
                                format!("BUF.MGT{idx}.DCM", idx = i % 2),
                                "1",
                            ));
                        if tile == "HCLK_DCM" || num_ccms < 4 {
                            builder = builder.prop(ExtraHclkDcmAttr(
                                dir,
                                "DCM",
                                format!("ENABLE.MGT{i}"),
                                "1",
                            ));
                        }
                        if tile != "HCLK_DCM" && num_ccms != 0 {
                            builder = builder.prop(ExtraHclkDcmAttr(
                                dir,
                                "CCM",
                                format!("ENABLE.MGT{i}"),
                                "1",
                            ));
                        }
                        builder
                            .test_manual(format!("BUF.MGT_{ud}{i}"), "1")
                            .pip(format!("MGT_O_{ud}{i}"), format!("MGT_I{i}"))
                            .commit();
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in ["CLK_IOB_B", "CLK_IOB_T"] {
        let bel = "CLK_IOB";
        for i in 0..16 {
            ctx.collect_bit_wide(tile, bel, &(format!("BUF.GIOB{i}")), "1");
        }
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mut vals = vec![("NONE".to_string(), Diff::default())];
            for j in 0..16 {
                let giob = format!("GIOB{j}");
                vals.push((giob.clone(), ctx.state.get_diff(tile, bel, &mux, &giob)));
            }
            vals.push((
                "PASS".to_string(),
                ctx.state.get_diff(tile, bel, &mux, "PASS"),
            ));
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
    }
    {
        let tile = "CLK_TERM";
        let bel = "CLK_TERM";
        ctx.collect_bit(tile, bel, "GIOB_ENABLE", "1");
    }
    for tile in ["CLK_DCM_B", "CLK_DCM_T"] {
        let bel = "CLK_DCM";
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mut vals = vec![("NONE".to_string(), Diff::default())];
            for j in 0..24 {
                let dcm = format!("DCM{j}");
                vals.push((dcm.clone(), ctx.state.get_diff(tile, bel, &mux, &dcm)));
            }
            let has_other = if tile == "CLK_DCM_T" {
                edev.chips
                    .values()
                    .any(|grid| grid.regs - grid.reg_clk.to_idx() > 2)
            } else {
                edev.chips.values().any(|grid| grid.reg_clk.to_idx() > 2)
            };
            if has_other {
                vals.push((
                    "PASS".to_string(),
                    ctx.state.get_diff(tile, bel, &mux, "PASS"),
                ));
            }
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
    }
    {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        let gclk: Vec<_> = (0..32).map(|i| format!("GCLK{i}")).collect();
        let hclk_l: Vec<_> = (0..8).map(|i| format!("MUX.HCLK_L{i}")).collect();
        let hclk_r: Vec<_> = (0..8).map(|i| format!("MUX.HCLK_R{i}")).collect();
        let mut inp_diffs = vec![];
        for i in 0..32 {
            let diff_l = ctx.state.peek_diff(tile, bel, &hclk_l[0], &gclk[i]).clone();
            let diff_r = ctx.state.peek_diff(tile, bel, &hclk_r[0], &gclk[i]).clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for hclk in [hclk_l, hclk_r] {
            for i in 0..8 {
                let mut inps = vec![("NONE", Diff::default())];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(tile, bel, &hclk[i], &gclk[j]);
                    diff = diff.combine(&!&inp_diffs[j]);
                    inps.push((&gclk[j], diff));
                }
                ctx.tiledb
                    .insert(tile, bel, &hclk[i], xlat_enum_ocd(inps, OcdMode::Mux));
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{i}"), xlat_bit(diff));
        }
    }
    {
        let tile = "HCLK_TERM";
        let bel = "HCLK_TERM";
        ctx.collect_bit_wide(tile, bel, "HCLK_ENABLE", "1");
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
        }
        for i in 0..2 {
            ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
        }
    }
    for tile in [
        "HCLK_IOIS_LVDS",
        "HCLK_IOIS_DCI",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
    ] {
        let tcid = ctx.edev.db.get_tile_class(tile);
        let tcls = &ctx.edev.db[tcid];
        if tcls.bels.contains_id(bels::RCLK) {
            let bel = "RCLK";
            for mux in ["MUX.RCLK0", "MUX.RCLK1"] {
                ctx.collect_enum_default_ocd(
                    tile,
                    bel,
                    mux,
                    &[
                        "VRCLK_N0", "VRCLK0", "VRCLK_S0", "VRCLK_N1", "VRCLK1", "VRCLK_S1",
                    ],
                    "NONE",
                    OcdMode::Mux,
                );
            }
            for bel in ["BUFR0", "BUFR1"] {
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_enum(
                    tile,
                    bel,
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                ctx.collect_enum(
                    tile,
                    bel,
                    "MUX.I",
                    &["CKINT0", "CKINT1", "BUFIO0", "BUFIO1"],
                );
            }
        }
        {
            let bel = "IOCLK";
            for i in 0..8 {
                ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
            }
            for i in 0..2 {
                ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
            }
            let diff0 = ctx.state.get_diff(tile, bel, "BUF.VIOCLK0", "1");
            let diff1 = ctx.state.get_diff(tile, bel, "BUF.VIOCLK1", "1");
            let (diff0, diff1, diffc) = Diff::split(diff0, diff1);
            ctx.tiledb.insert(tile, bel, "BUF.VIOCLK0", xlat_bit(diff0));
            ctx.tiledb.insert(tile, bel, "BUF.VIOCLK1", xlat_bit(diff1));
            ctx.tiledb
                .insert(tile, bel, "VIOCLK_ENABLE", xlat_bit_wide(diffc));
            let (has_s, has_n) = match tile {
                "HCLK_IOIS_DCI" | "HCLK_IOIS_LVDS" => (true, true),
                "HCLK_DCMIOB" | "HCLK_CENTER_ABOVE_CFG" => (false, true),
                "HCLK_IOBDCM" => (true, false),
                "HCLK_CENTER" => (
                    true,
                    edev.chips[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                ctx.collect_bit(tile, bel, "BUF.IOCLK_N0", "1");
                ctx.collect_bit(tile, bel, "BUF.IOCLK_N1", "1");
            }
            if has_n {
                ctx.collect_bit(tile, bel, "BUF.IOCLK_S0", "1");
                ctx.collect_bit(tile, bel, "BUF.IOCLK_S1", "1");
            }
        }
        {
            let bel = "IDELAYCTRL";
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_enum(
                tile,
                bel,
                "MUX.REFCLK",
                &[
                    "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                ],
            )
        }
    }
    for tile in [
        "HCLK_IOIS_LVDS",
        "HCLK_IOIS_DCI",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
    ] {
        for attr in [
            "BUF.IOCLK_N0",
            "BUF.IOCLK_N1",
            "BUF.IOCLK_S0",
            "BUF.IOCLK_S1",
        ] {
            let item = ctx.tiledb.item("HCLK_IOIS_LVDS", "IOCLK", attr).clone();
            ctx.tiledb.insert(tile, "IOCLK", attr, item);
        }
    }
    {
        let tile = "HCLK_DCM";
        let bel = "HCLK_DCM";
        let (_, _, common) = Diff::split(
            ctx.state.peek_diff(tile, bel, "BUF.MGT_D0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "BUF.GIOB_D0", "1").clone(),
        );
        let (_, _, hclk_giob) = Diff::split(
            ctx.state.peek_diff(tile, bel, "BUF.HCLK_D0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "BUF.GIOB_D0", "1").clone(),
        );
        let (_, _, common_mgt) = Diff::split(
            ctx.state.peek_diff(tile, bel, "BUF.MGT_D0", "1").clone(),
            ctx.state.peek_diff(tile, bel, "BUF.MGT_D1", "1").clone(),
        );
        for ud in ['U', 'D'] {
            for i in 0..8 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.HCLK_{ud}{i}"), "1");
                let diff = diff.combine(&!&hclk_giob);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.HCLK_{ud}{i}"), xlat_bit(diff));
            }
            for i in 0..16 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.GIOB_{ud}{i}"), "1");
                let diff = diff.combine(&!&hclk_giob);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.GIOB_{ud}{i}"), xlat_bit(diff));
            }
            for i in 0..4 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.MGT_{ud}{i}"), "1");
                let diff = diff.combine(&!&common_mgt);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.MGT_{ud}{i}"), xlat_bit(diff));
            }
        }
        let hclk_giob = hclk_giob.combine(&!&common);
        let common_mgt = common_mgt.combine(&!&common);
        ctx.tiledb
            .insert(tile, bel, "COMMON", xlat_bit_wide(common));
        ctx.tiledb
            .insert(tile, bel, "COMMON_HCLK_GIOB", xlat_bit_wide(hclk_giob));
        ctx.tiledb
            .insert(tile, bel, "COMMON_MGT", xlat_bit_wide(common_mgt));
    }
    for (tile, bel, ud) in [
        ("HCLK_DCMIOB", "HCLK_DCM_S", 'D'),
        ("HCLK_IOBDCM", "HCLK_DCM_N", 'U'),
    ] {
        let (_, _, common) = Diff::split(
            ctx.state
                .peek_diff(tile, bel, format!("BUF.HCLK_{ud}0"), "1")
                .clone(),
            ctx.state
                .peek_diff(tile, bel, format!("BUF.GIOB_{ud}0"), "1")
                .clone(),
        );
        for i in 0..8 {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("BUF.HCLK_{ud}{i}"), "1");
            let diff = diff.combine(&!&common);
            ctx.tiledb
                .insert(tile, bel, format!("BUF.HCLK_{ud}{i}"), xlat_bit(diff));
        }
        for i in 0..16 {
            let diff = ctx
                .state
                .get_diff(tile, bel, format!("BUF.GIOB_{ud}{i}"), "1");
            let diff = diff.combine(&!&common);
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GIOB_{ud}{i}"), xlat_bit(diff));
        }
        if edev.col_lgt.is_some() {
            let (_, _, common_mgt) = Diff::split(
                ctx.state
                    .peek_diff(tile, bel, format!("BUF.MGT_{ud}0"), "1")
                    .clone(),
                ctx.state
                    .peek_diff(tile, bel, format!("BUF.MGT_{ud}1"), "1")
                    .clone(),
            );
            for i in 0..4 {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("BUF.MGT_{ud}{i}"), "1");
                let diff = diff.combine(&!&common_mgt);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.MGT_{ud}{i}"), xlat_bit(diff));
            }
            let common_mgt = common_mgt.combine(&!&common);
            ctx.tiledb
                .insert(tile, bel, "COMMON_MGT", xlat_bit_wide(common_mgt));
        }
        ctx.tiledb
            .insert(tile, bel, "COMMON", xlat_bit_wide(common));
    }
    {
        let tile = "DCM";
        let bel = "DCM0";
        for i in 0..16 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GIOB{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.HCLK{i}"), "1");
        }
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.MGT{i}"), "1");
        }
    }
    let ccm = edev.db.get_tile_class("CCM");
    let num_ccms = edev.tile_index[ccm].len();
    if num_ccms != 0 {
        let tile = "CCM";
        let bel = "CCM";
        for i in 0..16 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GIOB{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.HCLK{i}"), "1");
        }
        if edev.col_lgt.is_some() {
            for i in 0..4 {
                ctx.collect_bit(tile, bel, &format!("ENABLE.MGT{i}"), "1");
            }
        }
    }
    if edev.col_lgt.is_some() && !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tile = "HCLK_MGT_REPEATER";
        let bel = "HCLK_MGT_REPEATER";
        let item = ctx.extract_bit(tile, bel, "BUF.MGT0.DCM", "1");
        ctx.tiledb.insert(tile, bel, "BUF.MGT0", item);
        let item = ctx.extract_bit(tile, bel, "BUF.MGT1.DCM", "1");
        ctx.tiledb.insert(tile, bel, "BUF.MGT1", item);
    }
}
