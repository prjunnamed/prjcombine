use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_collector::{
    diff::{Diff, DiffKey, FeatureId, OcdMode},
    legacy::{xlat_bit_legacy, xlat_bit_wide_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::defs;

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
        Some(tcrd.with_row(row).tile(defs::tslots::HROW))
    }
}

#[derive(Copy, Clone, Debug)]
struct ClkHrow;

impl TileRelation for ClkHrow {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(tcrd.with_col(edev.col_clk).tile(defs::tslots::HROW))
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
        Some(tcrd.with_col(col).tile(defs::tslots::HROW))
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
        Some(tcrd.with_col(col).tile(defs::tslots::HCLK_BEL))
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
        Some(tcrd.with_row(row).tile(defs::tslots::HCLK_BEL))
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
                    key: DiffKey::Legacy(FeatureId {
                        tile: self.1.into(),
                        bel: if self.1 == "DCM" { "DCM0" } else { self.1 }.into(),
                        attr: self.2.clone(),
                        val: self.3.into(),
                    }),
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
                let ntcrd = tcrd.with_col(rcol).tile(defs::tslots::CLK);
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Legacy(FeatureId {
                        tile: "HCLK_MGT_BUF".into(),
                        bel: "HCLK_MGT_BUF".into(),
                        attr: self.1.clone(),
                        val: self.2.into(),
                    }),
                    rects: edev.tile_bits(ntcrd),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct MgtRepeater(DirH, DirV, String, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for MgtRepeater {
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
        let chip = edev.chips[tcrd.die];
        let rrow = match self.1 {
            DirV::S => chip.row_bufg() - 8,
            DirV::N => chip.row_bufg() + 8,
        };
        for &col in &edev.chips[DieId::from_idx(0)].cols_vbrk {
            if (col < edev.col_cfg) == (self.0 == DirH::W) {
                let rcol = if self.0 == DirH::W { col } else { col - 1 };
                let ntcrd = tcrd.with_cr(rcol, rrow).tile(defs::tslots::CLK);
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Legacy(FeatureId {
                        tile: "HCLK_MGT_BUF".into(),
                        bel: "HCLK_MGT_BUF".into(),
                        attr: self.2.clone(),
                        val: self.3.into(),
                    }),
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

    {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_BUFG");

        for i in 0..32 {
            let mut bctx = ctx.bel(defs::bslots::BUFGCTRL[i]);
            let mode = "BUFGCTRL";
            bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                bctx.mode(mode).test_inv(pin);
            }
            bctx.mode(mode)
                .test_enum_legacy("PRESELECT_I0", &["FALSE", "TRUE"]);
            bctx.mode(mode)
                .test_enum_legacy("PRESELECT_I1", &["FALSE", "TRUE"]);
            bctx.mode(mode).test_enum_legacy("CREATE_EDGE", &["FALSE", "TRUE"]);
            bctx.mode(mode).test_enum_legacy("INIT_OUT", &["0", "1"]);

            for midx in 0..2 {
                let bus = format!("MUXBUS{midx}");
                let mux = format!("MUX.I{midx}");
                let opin = format!("I{midx}MUX");
                for val in ["CKINT0", "CKINT1"] {
                    bctx.build()
                        .mutex("IxMUX", &mux)
                        .mutex(&mux, val)
                        .test_manual_legacy(&mux, val)
                        .pip(&opin, val)
                        .commit();
                }
                let mb_idx = 2 * (i % 16) + midx;
                let mb_out = format!("MUXBUS_O{mb_idx}");
                let clk_iob = CellCoord::new(
                    DieId::from_idx(0),
                    edev.col_clk,
                    if i < 16 {
                        edev.row_dcmiob.unwrap()
                    } else {
                        edev.row_iobdcm.unwrap() - 16
                    },
                )
                .tile(defs::tslots::CLK);
                bctx.build()
                    .mutex("IxMUX", &mux)
                    .mutex(&mux, "MUXBUS")
                    .global_mutex("CLK_IOB_MUXBUS", "USE")
                    .related_pip(
                        FixedRelation(clk_iob),
                        (defs::bslots::CLK_IOB, mb_out),
                        (defs::bslots::CLK_IOB, "PAD_BUF0"),
                    )
                    .test_manual_legacy(&mux, "MUXBUS")
                    .pip(&opin, bus)
                    .commit();
                for j in 0..16 {
                    let obel = defs::bslots::BUFGCTRL[if i < 16 { j } else { j + 16 }];
                    let val = format!("GFB{j}");
                    bctx.build()
                        .mutex("IxMUX", &mux)
                        .mutex(&mux, &val)
                        .test_manual_legacy(&mux, &val)
                        .pip(&opin, (obel, "GFB"))
                        .commit();
                }
                for val in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
                    let obel = if i < 16 {
                        defs::bslots::BUFG_MGTCLK_S
                    } else {
                        defs::bslots::BUFG_MGTCLK_N
                    };
                    let obel_bufg = defs::bslots::BUFGCTRL[i ^ 1];
                    bctx.build()
                        .mutex("IxMUX", &mux)
                        .mutex(&mux, val)
                        .global_mutex("BUFG_MGTCLK", "USE")
                        .bel_mutex(obel_bufg, "IxMUX", &mux)
                        .bel_mutex(obel_bufg, &mux, val)
                        .pip((obel_bufg, &opin), (obel, val))
                        .test_manual_legacy(&mux, val)
                        .pip(&opin, (obel, val))
                        .commit();
                }
            }
            bctx.mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .test_manual_legacy("ENABLE", "1")
                .pin("O")
                .commit();
            bctx.mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .pin("O")
                .test_manual_legacy("PIN_O_GFB", "1")
                .pip("GFB", "O")
                .commit();
            let mut builder = bctx
                .mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .global_mutex("BUFGCTRL_O_GCLK", format!("BUFGCTRL{i}"))
                .pin("O");
            if !matches!(i, 19 | 30) {
                builder =
                    builder.extra_tiles_attr_by_kind("CLK_TERM", "CLK_TERM", "GCLK_ENABLE", "1")
            }
            builder
                .test_manual_legacy("PIN_O_GCLK", "1")
                .pip("GCLK", "O")
                .commit();
        }
        if edev.col_lgt.is_some() {
            for bel in [
                defs::bslots::BUFG_MGTCLK_S_HROW,
                defs::bslots::BUFG_MGTCLK_N_HROW,
            ] {
                let mut bctx = ctx.bel(bel);
                for (name, o, i) in [
                    ("BUF.MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                    ("BUF.MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                    ("BUF.MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                    ("BUF.MGT_R1", "MGT_R1_O", "MGT_R1_I"),
                ] {
                    bctx.build()
                        .global_mutex("BUFG_MGTCLK", "TEST")
                        .test_manual_legacy(name, "1")
                        .pip(o, i)
                        .commit();
                }
            }
            for (bel, dir_row) in [
                (defs::bslots::BUFG_MGTCLK_S_HCLK, DirV::S),
                (defs::bslots::BUFG_MGTCLK_N_HCLK, DirV::N),
            ] {
                let mut bctx = ctx.bel(bel);
                for (name, o, i) in [
                    ("MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                    ("MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                    ("MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                    ("MGT_R1", "MGT_R1_O", "MGT_R1_I"),
                ] {
                    let idx = if name.ends_with('1') { 1 } else { 0 };
                    bctx.build()
                        .global_mutex("MGT_OUT", "USE")
                        .null_bits()
                        .prop(MgtRepeater(
                            if name.starts_with("MGT_L") {
                                DirH::W
                            } else {
                                DirH::E
                            },
                            dir_row,
                            format!("BUF.MGT{idx}.CFG"),
                            "1",
                        ))
                        .test_manual_legacy(name, "1")
                        .pip(o, i)
                        .commit();
                }
            }
        }
    }

    for tile in ["CLK_IOB_S", "CLK_IOB_N"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(defs::bslots::CLK_IOB);
        let giob: Vec<_> = (0..16).map(|i| format!("GIOB{i}")).collect();
        for i in 0..16 {
            bctx.build()
                .global_mutex("GIOB", "TEST")
                .tile_mutex("GIOB_TEST", &giob[i])
                .extra_tile_attr(ClkTerm, "CLK_TERM", "GIOB_ENABLE", "1")
                .test_manual_legacy(format!("BUF.GIOB{i}"), "1")
                .pip(&giob[i], format!("PAD_BUF{i}"))
                .commit();
        }
        let clk_dcm = match tile {
            "CLK_IOB_S" => Delta::new(0, -8, "CLK_DCM_S"),
            "CLK_IOB_N" => Delta::new(0, 16, "CLK_DCM_N"),
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
                    .test_manual_legacy(&mux, &giob[j])
                    .pip(&mout, format!("PAD_BUF{j}"))
                    .commit();
            }
            bctx.build()
                .global_mutex("CLK_IOB_MUXBUS", "TEST")
                .tile_mutex(&mout, &min)
                .related_pip(
                    clk_dcm.clone(),
                    (defs::bslots::CLK_DCM, &mout),
                    (defs::bslots::CLK_DCM, "DCM0"),
                )
                .related_tile_mutex(clk_dcm.clone(), "MUXBUS", "USE")
                .test_manual_legacy(&mux, "PASS")
                .pip(&mout, &min)
                .commit();
        }
    }
    for tile in ["CLK_DCM_S", "CLK_DCM_N"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(defs::bslots::CLK_DCM);
        let dcm: Vec<_> = (0..24).map(|i| format!("DCM{i}")).collect();
        let clk_dcm = match tile {
            "CLK_DCM_S" => Delta::new(0, -8, "CLK_DCM_S"),
            "CLK_DCM_N" => Delta::new(0, 8, "CLK_DCM_N"),
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
                    .test_manual_legacy(&mux, &dcm[j])
                    .pip(&mout, &dcm[j])
                    .commit();
            }
            let has_other = if tile == "CLK_DCM_N" {
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
                        (defs::bslots::CLK_DCM, &mout),
                        (defs::bslots::CLK_DCM, "DCM0"),
                    )
                    .related_tile_mutex(clk_dcm.clone(), "MUXBUS", "USE")
                    .test_manual_legacy(&mux, "PASS")
                    .pip(&mout, &min)
                    .commit();
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_HROW");
        let mut bctx = ctx.bel(defs::bslots::CLK_HROW);
        let gclk: Vec<_> = (0..32).map(|i| format!("GCLK{i}")).collect();
        for (dir, lr) in [(DirH::W, 'L'), (DirH::E, 'R')] {
            for i in 0..8 {
                let hclk = format!("HCLK_{lr}{i}");
                for j in 0..32 {
                    let bel_bufg = defs::bslots::BUFGCTRL[j];
                    let cfg =
                        FixedRelation(edev.tile_cfg(DieId::from_idx(0)).tile(defs::tslots::BEL));
                    bctx.build()
                        .global_mutex("BUFGCTRL_OUT", "USE")
                        .tile_mutex("MODE", "TEST")
                        .tile_mutex("IN", &gclk[j])
                        .tile_mutex("OUT", &hclk)
                        .related_pip(cfg, (bel_bufg, "GCLK"), (bel_bufg, "O"))
                        .extra_tile_attr(HclkTerm(dir), "HCLK_TERM", "HCLK_ENABLE", "1")
                        .test_manual_legacy(format!("MUX.HCLK_{lr}{i}"), &gclk[j])
                        .pip(&hclk, &gclk[j])
                        .commit();
                }
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK");
        let mut bctx = ctx.bel(defs::bslots::HCLK);
        for i in 0..8 {
            let hclk_i = format!("HCLK_I{i}");
            let hclk_o = format!("HCLK_O{i}");
            let hclk_l = format!("HCLK_L{i}");
            let hclk_r = format!("HCLK_R{i}");
            let obel = defs::bslots::CLK_HROW;
            bctx.build()
                .global_mutex("BUFGCTRL_OUT", "USE")
                .related_tile_mutex(ClkHrow, "MODE", "USE")
                .related_pip(ClkHrow, (obel, hclk_l), (obel, "GCLK0"))
                .related_pip(ClkHrow, (obel, hclk_r), (obel, "GCLK0"))
                .test_manual_legacy(format!("BUF.HCLK{i}"), "1")
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
                    (defs::bslots::RCLK, format!("RCLK{i}")),
                    (defs::bslots::RCLK, "VRCLK0"),
                )
                .test_manual_legacy(format!("BUF.RCLK{i}"), "1")
                .pip(rclk_o, rclk_i)
                .commit();
        }
    }
    for tile in [
        "HCLK_IO_LVDS",
        "HCLK_IO_DCI",
        "HCLK_IO_DCM_S",
        "HCLK_IO_DCM_N",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_N",
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let tcid = backend.edev.db.get_tile_class(tile);
        let tcls = &backend.edev.db[tcid];
        if tcls.bels.contains_id(defs::bslots::RCLK) {
            let mut bctx = ctx.bel(defs::bslots::RCLK);
            for opin in ["RCLK0", "RCLK1"] {
                for ipin in [
                    "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                ] {
                    bctx.build()
                        .tile_mutex("RCLK_MODE", "TEST")
                        .tile_mutex(opin, ipin)
                        .test_manual_legacy(format!("MUX.{opin}"), ipin)
                        .pip(opin, ipin)
                        .commit();
                }
            }
            let obel_rclk = defs::bslots::RCLK;
            for bel in [defs::bslots::BUFR[0], defs::bslots::BUFR[1]] {
                let mut bctx = ctx.bel(bel);
                bctx.test_manual_legacy("PRESENT", "1").mode("BUFR").commit();
                bctx.mode("BUFR")
                    .test_manual_legacy("ENABLE", "1")
                    .pin("O")
                    .commit();
                bctx.mode("BUFR").test_enum_legacy(
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                for ckint in ["CKINT0", "CKINT1"] {
                    bctx.mode("BUFR")
                        .mutex("MUX.I", ckint)
                        .test_manual_legacy("MUX.I", ckint)
                        .pip("I", (obel_rclk, ckint))
                        .commit();
                }
                for (obel, obel_name) in [
                    (defs::bslots::BUFIO[0], "BUFIO0"),
                    (defs::bslots::BUFIO[1], "BUFIO1"),
                ] {
                    bctx.mode("BUFR")
                        .mutex("MUX.I", obel_name)
                        .test_manual_legacy("MUX.I", obel_name)
                        .pip("I", (obel, "O"))
                        .commit();
                }
            }
        }
        {
            let mut bctx = ctx.bel(defs::bslots::IOCLK);
            for i in 0..8 {
                let hclk_i = format!("HCLK_I{i}");
                let hclk_o = format!("HCLK_O{i}");
                let hclk_l = format!("HCLK_L{i}");
                let hclk_r = format!("HCLK_R{i}");
                let obel = defs::bslots::CLK_HROW;
                bctx.build()
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow, "MODE", "USE")
                    .related_pip(ClkHrow, (obel, hclk_l), (obel, "GCLK0"))
                    .related_pip(ClkHrow, (obel, hclk_r), (obel, "GCLK0"))
                    .test_manual_legacy(format!("BUF.HCLK{i}"), "1")
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
                        (defs::bslots::RCLK, format!("RCLK{i}")),
                        (defs::bslots::RCLK, "VRCLK0"),
                    )
                    .test_manual_legacy(format!("BUF.RCLK{i}"), "1")
                    .pip(rclk_o, rclk_i)
                    .commit();
            }

            for (obel, vioclk) in [
                (defs::bslots::BUFIO[0], "VIOCLK0"),
                (defs::bslots::BUFIO[1], "VIOCLK1"),
            ] {
                bctx.build()
                    .tile_mutex("VIOCLK", vioclk)
                    .test_manual_legacy(format!("BUF.{vioclk}"), "1")
                    .pip(vioclk, (obel, "O"))
                    .commit();
            }
            let (has_s, has_n) = match tile {
                "HCLK_IO_DCI" | "HCLK_IO_LVDS" => (true, true),
                "HCLK_IO_DCM_N" | "HCLK_IO_CFG_N" => (false, true),
                "HCLK_IO_DCM_S" => (true, false),
                "HCLK_IO_CENTER" => (
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
                        .test_manual_legacy(format!("BUF.{o}"), "1")
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
                        .test_manual_legacy(format!("BUF.{o}"), "1")
                        .pip(o, i)
                        .commit();
                }
            }
        }
        {
            let mut bctx = ctx.bel(defs::bslots::IDELAYCTRL);
            bctx.test_manual_legacy("ENABLE", "1").mode("IDELAYCTRL").commit();
            for i in 0..8 {
                let hclk = format!("HCLK{i}");
                let hclk_o = format!("HCLK_O{i}");
                bctx.build()
                    .mutex("REFCLK", &hclk)
                    .test_manual_legacy("MUX.REFCLK", &hclk)
                    .pip("REFCLK", (defs::bslots::IOCLK, hclk_o))
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
        ("HCLK_DCM", defs::bslots::HCLK_DCM),
        ("HCLK_IO_DCM_N", defs::bslots::HCLK_DCM_S),
        ("HCLK_IO_DCM_S", defs::bslots::HCLK_DCM_N),
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
            if dir == DirV::S && bel == defs::bslots::HCLK_DCM_N {
                continue;
            }
            if dir == DirV::N && bel == defs::bslots::HCLK_DCM_S {
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
                    .test_manual_legacy(format!("BUF.GIOB_{ud}{i}"), "1")
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
                        (defs::bslots::CLK_HROW, format!("HCLK_L{i}")),
                        (defs::bslots::CLK_HROW, "GCLK0"),
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
                    .test_manual_legacy(format!("BUF.HCLK_{ud}{i}"), "1")
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
                            .test_manual_legacy(format!("BUF.MGT_{ud}{i}"), "1")
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
                            .test_manual_legacy(format!("BUF.MGT_{ud}{i}"), "1")
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
    {
        let tile = "CLK_BUFG";
        for i in 0..32 {
            let bel = format!("BUFGCTRL[{i}]");
            let bel = &bel;
            ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
                .assert_empty();
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                ctx.collect_inv(tile, bel, pin);
            }
            ctx.collect_bit_bi_legacy(tile, bel, "PRESELECT_I0", "FALSE", "TRUE");
            ctx.collect_bit_bi_legacy(tile, bel, "PRESELECT_I1", "FALSE", "TRUE");
            ctx.collect_bit_bi_legacy(tile, bel, "CREATE_EDGE", "FALSE", "TRUE");
            ctx.collect_bit_bi_legacy(tile, bel, "INIT_OUT", "0", "1");

            let (_, _, ien_diff) = Diff::split(
                ctx.peek_diff_legacy(tile, bel, "MUX.I0", "CKINT0").clone(),
                ctx.peek_diff_legacy(tile, bel, "MUX.I1", "CKINT0").clone(),
            );
            let ien_item = xlat_bit_legacy(ien_diff);
            for mux in ["MUX.I0", "MUX.I1"] {
                let mut vals = vec![("NONE", Diff::default())];
                for val in [
                    "GFB0", "GFB1", "GFB2", "GFB3", "GFB4", "GFB5", "GFB6", "GFB7", "GFB8", "GFB9",
                    "GFB10", "GFB11", "GFB12", "GFB13", "GFB14", "GFB15", "CKINT0", "CKINT1",
                    "MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1", "MUXBUS",
                ] {
                    let mut diff = ctx.get_diff_legacy(tile, bel, mux, val);
                    diff.apply_bit_diff_legacy(&ien_item, true, false);
                    vals.push((val, diff));
                }
                ctx.insert(tile, bel, mux, xlat_enum_legacy_ocd(vals, OcdMode::Mux));
            }
            ctx.insert(tile, bel, "IMUX_ENABLE", ien_item);

            ctx.get_diff_legacy(tile, bel, "PIN_O_GFB", "1")
                .assert_empty();
            ctx.collect_bit_wide_legacy(tile, bel, "ENABLE", "1");
            ctx.get_diff_legacy(tile, bel, "PIN_O_GCLK", "1")
                .assert_empty();
        }

        for bel in ["BUFG_MGTCLK_S", "BUFG_MGTCLK_N"] {
            for attr in ["BUF.MGT_L0", "BUF.MGT_L1", "BUF.MGT_R0", "BUF.MGT_R1"] {
                let item = ctx.extract_bit_legacy(tile, &format!("{bel}_HROW"), attr, "1");
                ctx.insert(tile, bel, attr, item);
            }
        }
    }

    if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tile = "HCLK_MGT_BUF";
        let bel = "HCLK_MGT_BUF";
        let item = ctx.extract_bit_legacy(tile, bel, "BUF.MGT0.CFG", "1");
        ctx.insert(tile, bel, "BUF.MGT0", item);
        let item = ctx.extract_bit_legacy(tile, bel, "BUF.MGT1.CFG", "1");
        ctx.insert(tile, bel, "BUF.MGT1", item);
    }

    {
        let tile = "CLK_TERM";
        let bel = "CLK_TERM";
        ctx.collect_bit_legacy(tile, bel, "GCLK_ENABLE", "1");
    }

    for tile in ["CLK_IOB_S", "CLK_IOB_N"] {
        let bel = "CLK_IOB";
        for i in 0..16 {
            ctx.collect_bit_wide_legacy(tile, bel, &(format!("BUF.GIOB{i}")), "1");
        }
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mut vals = vec![("NONE".to_string(), Diff::default())];
            for j in 0..16 {
                let giob = format!("GIOB{j}");
                vals.push((giob.clone(), ctx.get_diff_legacy(tile, bel, &mux, &giob)));
            }
            vals.push((
                "PASS".to_string(),
                ctx.get_diff_legacy(tile, bel, &mux, "PASS"),
            ));
            ctx.insert(tile, bel, mux, xlat_enum_legacy_ocd(vals, OcdMode::Mux));
        }
    }
    {
        let tile = "CLK_TERM";
        let bel = "CLK_TERM";
        ctx.collect_bit_legacy(tile, bel, "GIOB_ENABLE", "1");
    }
    for tile in ["CLK_DCM_S", "CLK_DCM_N"] {
        let bel = "CLK_DCM";
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mut vals = vec![("NONE".to_string(), Diff::default())];
            for j in 0..24 {
                let dcm = format!("DCM{j}");
                vals.push((dcm.clone(), ctx.get_diff_legacy(tile, bel, &mux, &dcm)));
            }
            let has_other = if tile == "CLK_DCM_N" {
                edev.chips
                    .values()
                    .any(|grid| grid.regs - grid.reg_clk.to_idx() > 2)
            } else {
                edev.chips.values().any(|grid| grid.reg_clk.to_idx() > 2)
            };
            if has_other {
                vals.push((
                    "PASS".to_string(),
                    ctx.get_diff_legacy(tile, bel, &mux, "PASS"),
                ));
            }
            ctx.insert(tile, bel, mux, xlat_enum_legacy_ocd(vals, OcdMode::Mux));
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
            let diff_l = ctx
                .peek_diff_legacy(tile, bel, &hclk_l[0], &gclk[i])
                .clone();
            let diff_r = ctx
                .peek_diff_legacy(tile, bel, &hclk_r[0], &gclk[i])
                .clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for hclk in [hclk_l, hclk_r] {
            for i in 0..8 {
                let mut inps = vec![("NONE", Diff::default())];
                for j in 0..32 {
                    let mut diff = ctx.get_diff_legacy(tile, bel, &hclk[i], &gclk[j]);
                    diff = diff.combine(&!&inp_diffs[j]);
                    inps.push((&gclk[j], diff));
                }
                ctx.insert(
                    tile,
                    bel,
                    &hclk[i],
                    xlat_enum_legacy_ocd(inps, OcdMode::Mux),
                );
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.insert(tile, bel, format!("BUF.GCLK{i}"), xlat_bit_legacy(diff));
        }
    }
    {
        let tile = "HCLK_TERM";
        let bel = "HCLK_TERM";
        ctx.collect_bit_wide_legacy(tile, bel, "HCLK_ENABLE", "1");
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..8 {
            ctx.collect_bit_legacy(tile, bel, &format!("BUF.HCLK{i}"), "1");
        }
        for i in 0..2 {
            ctx.collect_bit_legacy(tile, bel, &format!("BUF.RCLK{i}"), "1");
        }
    }
    for tile in [
        "HCLK_IO_LVDS",
        "HCLK_IO_DCI",
        "HCLK_IO_DCM_S",
        "HCLK_IO_DCM_N",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_N",
    ] {
        let tcid = ctx.edev.db.get_tile_class(tile);
        let tcls = &ctx.edev.db[tcid];
        if tcls.bels.contains_id(defs::bslots::RCLK) {
            let bel = "RCLK";
            for mux in ["MUX.RCLK0", "MUX.RCLK1"] {
                ctx.collect_enum_default_legacy_ocd(
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
            for bel in ["BUFR[0]", "BUFR[1]"] {
                ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
                    .assert_empty();
                ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
                ctx.collect_enum_legacy(
                    tile,
                    bel,
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                ctx.collect_enum_legacy(
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
                ctx.collect_bit_legacy(tile, bel, &format!("BUF.HCLK{i}"), "1");
            }
            for i in 0..2 {
                ctx.collect_bit_legacy(tile, bel, &format!("BUF.RCLK{i}"), "1");
            }
            let diff0 = ctx.get_diff_legacy(tile, bel, "BUF.VIOCLK0", "1");
            let diff1 = ctx.get_diff_legacy(tile, bel, "BUF.VIOCLK1", "1");
            let (diff0, diff1, diffc) = Diff::split(diff0, diff1);
            ctx.insert(tile, bel, "BUF.VIOCLK0", xlat_bit_legacy(diff0));
            ctx.insert(tile, bel, "BUF.VIOCLK1", xlat_bit_legacy(diff1));
            ctx.insert(tile, bel, "VIOCLK_ENABLE", xlat_bit_wide_legacy(diffc));
            let (has_s, has_n) = match tile {
                "HCLK_IO_DCI" | "HCLK_IO_LVDS" => (true, true),
                "HCLK_IO_DCM_N" | "HCLK_IO_CFG_N" => (false, true),
                "HCLK_IO_DCM_S" => (true, false),
                "HCLK_IO_CENTER" => (
                    true,
                    edev.chips[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                ctx.collect_bit_legacy(tile, bel, "BUF.IOCLK_N0", "1");
                ctx.collect_bit_legacy(tile, bel, "BUF.IOCLK_N1", "1");
            }
            if has_n {
                ctx.collect_bit_legacy(tile, bel, "BUF.IOCLK_S0", "1");
                ctx.collect_bit_legacy(tile, bel, "BUF.IOCLK_S1", "1");
            }
        }
        {
            let bel = "IDELAYCTRL";
            ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
            ctx.collect_enum_legacy(
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
        "HCLK_IO_LVDS",
        "HCLK_IO_DCI",
        "HCLK_IO_DCM_S",
        "HCLK_IO_DCM_N",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_N",
    ] {
        for attr in [
            "BUF.IOCLK_N0",
            "BUF.IOCLK_N1",
            "BUF.IOCLK_S0",
            "BUF.IOCLK_S1",
        ] {
            let item = ctx.item("HCLK_IO_LVDS", "IOCLK", attr).clone();
            ctx.insert(tile, "IOCLK", attr, item);
        }
    }
    {
        let tile = "HCLK_DCM";
        let bel = "HCLK_DCM";
        let (_, _, common) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "BUF.MGT_D0", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "BUF.GIOB_D0", "1").clone(),
        );
        let (_, _, hclk_giob) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "BUF.HCLK_D0", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "BUF.GIOB_D0", "1").clone(),
        );
        let (_, _, common_mgt) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "BUF.MGT_D0", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "BUF.MGT_D1", "1").clone(),
        );
        for ud in ['U', 'D'] {
            for i in 0..8 {
                let diff = ctx.get_diff_legacy(tile, bel, format!("BUF.HCLK_{ud}{i}"), "1");
                let diff = diff.combine(&!&hclk_giob);
                ctx.insert(
                    tile,
                    bel,
                    format!("BUF.HCLK_{ud}{i}"),
                    xlat_bit_legacy(diff),
                );
            }
            for i in 0..16 {
                let diff = ctx.get_diff_legacy(tile, bel, format!("BUF.GIOB_{ud}{i}"), "1");
                let diff = diff.combine(&!&hclk_giob);
                ctx.insert(
                    tile,
                    bel,
                    format!("BUF.GIOB_{ud}{i}"),
                    xlat_bit_legacy(diff),
                );
            }
            for i in 0..4 {
                let diff = ctx.get_diff_legacy(tile, bel, format!("BUF.MGT_{ud}{i}"), "1");
                let diff = diff.combine(&!&common_mgt);
                ctx.insert(tile, bel, format!("BUF.MGT_{ud}{i}"), xlat_bit_legacy(diff));
            }
        }
        let hclk_giob = hclk_giob.combine(&!&common);
        let common_mgt = common_mgt.combine(&!&common);
        ctx.insert(tile, bel, "COMMON", xlat_bit_wide_legacy(common));
        ctx.insert(
            tile,
            bel,
            "COMMON_HCLK_GIOB",
            xlat_bit_wide_legacy(hclk_giob),
        );
        ctx.insert(tile, bel, "COMMON_MGT", xlat_bit_wide_legacy(common_mgt));
    }
    for (tile, bel, ud) in [
        ("HCLK_IO_DCM_N", "HCLK_DCM_S", 'D'),
        ("HCLK_IO_DCM_S", "HCLK_DCM_N", 'U'),
    ] {
        let (_, _, common) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, format!("BUF.HCLK_{ud}0"), "1")
                .clone(),
            ctx.peek_diff_legacy(tile, bel, format!("BUF.GIOB_{ud}0"), "1")
                .clone(),
        );
        for i in 0..8 {
            let diff = ctx.get_diff_legacy(tile, bel, format!("BUF.HCLK_{ud}{i}"), "1");
            let diff = diff.combine(&!&common);
            ctx.insert(
                tile,
                bel,
                format!("BUF.HCLK_{ud}{i}"),
                xlat_bit_legacy(diff),
            );
        }
        for i in 0..16 {
            let diff = ctx.get_diff_legacy(tile, bel, format!("BUF.GIOB_{ud}{i}"), "1");
            let diff = diff.combine(&!&common);
            ctx.insert(
                tile,
                bel,
                format!("BUF.GIOB_{ud}{i}"),
                xlat_bit_legacy(diff),
            );
        }
        if edev.col_lgt.is_some() {
            let (_, _, common_mgt) = Diff::split(
                ctx.peek_diff_legacy(tile, bel, format!("BUF.MGT_{ud}0"), "1")
                    .clone(),
                ctx.peek_diff_legacy(tile, bel, format!("BUF.MGT_{ud}1"), "1")
                    .clone(),
            );
            for i in 0..4 {
                let diff = ctx.get_diff_legacy(tile, bel, format!("BUF.MGT_{ud}{i}"), "1");
                let diff = diff.combine(&!&common_mgt);
                ctx.insert(tile, bel, format!("BUF.MGT_{ud}{i}"), xlat_bit_legacy(diff));
            }
            let common_mgt = common_mgt.combine(&!&common);
            ctx.insert(tile, bel, "COMMON_MGT", xlat_bit_wide_legacy(common_mgt));
        }
        ctx.insert(tile, bel, "COMMON", xlat_bit_wide_legacy(common));
    }
    {
        let tile = "DCM";
        let bel = "DCM0";
        for i in 0..16 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.GIOB{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.HCLK{i}"), "1");
        }
        for i in 0..4 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.MGT{i}"), "1");
        }
    }
    let ccm = edev.db.get_tile_class("CCM");
    let num_ccms = edev.tile_index[ccm].len();
    if num_ccms != 0 {
        let tile = "CCM";
        let bel = "CCM";
        for i in 0..16 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.GIOB{i}"), "1");
        }
        for i in 0..8 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.HCLK{i}"), "1");
        }
        if edev.col_lgt.is_some() {
            for i in 0..4 {
                ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.MGT{i}"), "1");
            }
        }
    }
    if edev.col_lgt.is_some() && !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tile = "HCLK_MGT_BUF";
        let bel = "HCLK_MGT_BUF";
        let item = ctx.extract_bit_legacy(tile, bel, "BUF.MGT0.DCM", "1");
        ctx.insert(tile, bel, "BUF.MGT0", item);
        let item = ctx.extract_bit_legacy(tile, bel, "BUF.MGT1.DCM", "1");
        ctx.insert(tile, bel, "BUF.MGT1", item);
    }
}
