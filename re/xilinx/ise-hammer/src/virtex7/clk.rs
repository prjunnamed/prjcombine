use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{RowId, TileCoord},
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, OcdMode, xlat_bit, xlat_enum_ocd};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::bels;
use prjcombine_entity::EntityId;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            bel::BelMutex,
            relation::{Delta, Related, TileRelation},
        },
    },
};

#[derive(Clone, Copy, Debug)]
pub struct ColPair(pub &'static str);

impl TileRelation for ColPair {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = match edev.col_side(tcrd.col) {
            DirH::W => tcrd.col + 1,
            DirH::E => tcrd.col - 1,
        };
        backend
            .edev
            .find_tile_by_class(tcrd.with_col(col), |kind| kind == self.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct CmtDir(DirH);

impl TileRelation for CmtDir {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let scol = match self.0 {
            DirH::W => edev.col_lio.unwrap() + 1,
            DirH::E => edev.col_rio.unwrap() - 1,
        };
        backend
            .edev
            .find_tile_by_class(tcrd.with_col(scol), |kind| kind == "CMT")
    }
}

#[derive(Clone, Copy, Debug)]
struct ClkRebuf(DirV);

impl TileRelation for ClkRebuf {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let mut cell = tcrd.cell;
        loop {
            match self.0 {
                DirV::S => {
                    if cell.row.to_idx() == 0 {
                        if cell.die.to_idx() == 0 {
                            return None;
                        }
                        cell.die -= 1;
                        cell.row = backend.edev.rows(cell.die).next_back().unwrap();
                    } else {
                        cell.row -= 1;
                    }
                }
                DirV::N => {
                    if cell.row == backend.edev.rows(cell.die).next_back().unwrap() {
                        cell.row = RowId::from_idx(0);
                        cell.die += 1;
                        if cell.die == backend.edev.die.next_id() {
                            return None;
                        }
                    } else {
                        cell.row += 1;
                    }
                }
            }
            if let Some(ntcrd) = backend.edev.find_tile_by_class(cell, |kind| {
                matches!(kind, "CLK_BUFG_REBUF" | "CLK_BALI_REBUF")
            }) {
                return Some(ntcrd);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct HclkSide(DirV);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for HclkSide {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];

        match self.0 {
            DirV::S => {
                if tcrd.row >= chip.row_hclk(tcrd.row) {
                    return None;
                }
            }
            DirV::N => {
                if tcrd.row < chip.row_hclk(tcrd.row) {
                    return None;
                }
            }
        }

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    bali_only: bool,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    if !bali_only {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK");
        let mut bctx = ctx.bel(bels::HCLK_W);
        for i in 6..12 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    bctx.build()
                        .tile_mutex("MODE", "TEST")
                        .global_mutex("HCLK", "USE")
                        .tile_mutex(format!("MUX.LCLK{i}_{ud}_L"), format!("HCLK{j}"))
                        .tile_mutex(format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}_L"))
                        .has_related(Delta::new(0, -1, "INT"))
                        .has_related(Delta::new(-2, -1, "INT"))
                        .has_related(Delta::new(2, -1, "INT"))
                        .has_related(Delta::new(0, 1, "INT"))
                        .has_related(Delta::new(-2, 1, "INT"))
                        .has_related(Delta::new(2, 1, "INT"))
                        .related_tile_mutex(Delta::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            Delta::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}_I")
                            } else {
                                format!("HCLK{j}")
                            },
                        )
                        .related_tile_mutex(Delta::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            Delta::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}_I")
                            } else {
                                format!("HCLK{j}")
                            },
                        )
                        .test_manual(format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"))
                        .pip(
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}_I")
                            } else {
                                format!("HCLK{j}")
                            },
                        )
                        .commit();
                }
                for j in 0..4 {
                    bctx.build()
                        .tile_mutex("MODE", "TEST")
                        .global_mutex("RCLK", "USE")
                        .tile_mutex(format!("MUX.LCLK{i}_{ud}_L"), format!("RCLK{j}"))
                        .tile_mutex(format!("RCLK{j}"), format!("MUX.LCLK{i}_{ud}_L"))
                        .has_related(Delta::new(0, -1, "INT"))
                        .has_related(Delta::new(-2, -1, "INT"))
                        .has_related(Delta::new(2, -1, "INT"))
                        .has_related(Delta::new(0, 1, "INT"))
                        .has_related(Delta::new(-2, 1, "INT"))
                        .has_related(Delta::new(2, 1, "INT"))
                        .related_tile_mutex(Delta::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            Delta::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}"),
                        )
                        .related_tile_mutex(Delta::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            Delta::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}"),
                        )
                        .test_manual(format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .pip(format!("LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .commit();
                }
            }
        }
        let mut bctx = ctx.bel(bels::HCLK_E);
        for i in 0..6 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    bctx.build()
                        .tile_mutex("MODE", "TEST")
                        .global_mutex("HCLK", "USE")
                        .tile_mutex(format!("MUX.LCLK{i}_{ud}_R"), format!("HCLK{j}"))
                        .tile_mutex(format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}_R"))
                        .has_related(Delta::new(0, -1, "INT"))
                        .has_related(Delta::new(-2, -1, "INT"))
                        .has_related(Delta::new(2, -1, "INT"))
                        .has_related(Delta::new(0, 1, "INT"))
                        .has_related(Delta::new(-2, 1, "INT"))
                        .has_related(Delta::new(2, 1, "INT"))
                        .related_tile_mutex(Delta::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            Delta::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}")
                            } else {
                                format!("HCLK{j}_I")
                            },
                        )
                        .related_tile_mutex(Delta::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            Delta::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}")
                            } else {
                                format!("HCLK{j}_I")
                            },
                        )
                        .test_manual(format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"))
                        .pip(
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}")
                            } else {
                                format!("HCLK{j}_I")
                            },
                        )
                        .commit();
                }
                for j in 0..4 {
                    bctx.build()
                        .tile_mutex("MODE", "TEST")
                        .global_mutex("RCLK", "USE")
                        .tile_mutex(format!("MUX.LCLK{i}_{ud}_R"), format!("RCLK{j}"))
                        .tile_mutex(format!("RCLK{j}"), format!("MUX.LCLK{i}_{ud}_R"))
                        .has_related(Delta::new(0, -1, "INT"))
                        .has_related(Delta::new(-2, -1, "INT"))
                        .has_related(Delta::new(2, -1, "INT"))
                        .has_related(Delta::new(0, 1, "INT"))
                        .has_related(Delta::new(-2, 1, "INT"))
                        .has_related(Delta::new(2, 1, "INT"))
                        .related_tile_mutex(Delta::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            Delta::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}_I"),
                        )
                        .related_tile_mutex(Delta::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            Delta::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}_I"),
                        )
                        .test_manual(format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .pip(format!("LCLK{i}_{ud}"), format!("RCLK{j}_I"))
                        .commit();
                }
            }
        }
    }
    if !bali_only {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_BUFG");
        for i in 0..16 {
            let mut bctx = ctx.bel(bels::BUFGCTRL[i]);
            bctx.test_manual("ENABLE", "1").mode("BUFGCTRL").commit();
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                bctx.mode("BUFGCTRL").test_inv(pin);
            }
            bctx.mode("BUFGCTRL")
                .test_enum("PRESELECT_I0", &["FALSE", "TRUE"]);
            bctx.mode("BUFGCTRL")
                .test_enum("PRESELECT_I1", &["FALSE", "TRUE"]);
            bctx.mode("BUFGCTRL")
                .test_enum("CREATE_EDGE", &["FALSE", "TRUE"]);
            bctx.mode("BUFGCTRL").test_enum("INIT_OUT", &["0", "1"]);
            bctx.build()
                .tile_mutex("FB", "TEST")
                .test_manual("ENABLE.FB", "1")
                .pip("FB", "O")
                .commit();
            if edev.chips.first().unwrap().regs == 1 {
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_attr(
                        ClkRebuf(DirV::S),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    )
                    .test_manual("ENABLE.GCLK", "1")
                    .pip("GCLK", "O")
                    .commit();
            } else {
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_attr(
                        ClkRebuf(DirV::S),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    )
                    .extra_tile_attr(
                        ClkRebuf(DirV::N),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_D"),
                        "1",
                    )
                    .prop(HclkSide(DirV::N))
                    .test_manual("ENABLE.GCLK", "1")
                    .pip("GCLK", "O")
                    .commit();
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_attr(
                        ClkRebuf(DirV::S),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{ii}_U", ii = i + 16),
                        "1",
                    )
                    .extra_tile_attr(
                        ClkRebuf(DirV::N),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{ii}_D", ii = i + 16),
                        "1",
                    )
                    .prop(HclkSide(DirV::S))
                    .test_manual("ENABLE.GCLK", "1")
                    .pip("GCLK", "O")
                    .commit();
            }
            // ISE bug causes pips to be reversed?
            bctx.build()
                .mutex("MUX.I0", "FB_TEST")
                .test_manual("TEST_I0", "1")
                .pip("I0", "FB_TEST0")
                .commit();
            bctx.build()
                .mutex("MUX.I1", "FB_TEST")
                .test_manual("TEST_I1", "1")
                .pip("I1", "FB_TEST1")
                .commit();
            for j in 0..2 {
                bctx.build()
                    .mutex(format!("MUX.I{j}"), "CASCI")
                    .test_manual(format!("MUX.I{j}"), "CASCI")
                    .pip(format!("I{j}"), format!("CASCI{j}"))
                    .commit();
                for k in 0..2 {
                    bctx.build()
                        .mutex(format!("MUX.I{j}"), format!("CKINT{k}"))
                        .test_manual(format!("MUX.I{j}"), format!("CKINT{k}"))
                        .pip(format!("I{j}"), format!("CKINT{k}"))
                        .commit();
                }

                let obel_prev = bels::BUFGCTRL[(i + 15) % 16];
                bctx.build()
                    .tile_mutex("FB", "USE")
                    .mutex(format!("MUX.I{j}"), "FB_PREV")
                    .pip((obel_prev, "FB"), (obel_prev, "O"))
                    .test_manual(format!("MUX.I{j}"), "FB_PREV")
                    .pip(format!("I{j}"), (obel_prev, "FB"))
                    .commit();
                let obel_next = bels::BUFGCTRL[(i + 1) % 16];
                bctx.build()
                    .tile_mutex("FB", "USE")
                    .mutex(format!("MUX.I{j}"), "FB_NEXT")
                    .pip((obel_next, "FB"), (obel_next, "O"))
                    .test_manual(format!("MUX.I{j}"), "FB_NEXT")
                    .pip(format!("I{j}"), (obel_next, "FB"))
                    .commit();
            }
        }
    }
    if !bali_only {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_HROW");
        for i in 0..32 {
            let mut bctx = ctx.bel(bels::GCLK_TEST_BUF_HROW_GCLK[i]);
            bctx.test_manual("ENABLE", "1")
                .mode("GCLK_TEST_BUF")
                .commit();
            bctx.mode("GCLK_TEST_BUF")
                .test_enum("GCLK_TEST_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("GCLK_TEST_BUF")
                .test_enum("INVERT_INPUT", &["FALSE", "TRUE"]);
        }
        for (lr, bufhce, gclk_test_buf) in [
            ('L', bels::BUFHCE_W, bels::GCLK_TEST_BUF_HROW_BUFH_W),
            ('R', bels::BUFHCE_E, bels::GCLK_TEST_BUF_HROW_BUFH_E),
        ] {
            for i in 0..12 {
                let mut bctx = ctx.bel(bufhce[i]);
                bctx.test_manual("ENABLE", "1").mode("BUFHCE").commit();
                bctx.mode("BUFHCE").test_inv("CE");
                bctx.mode("BUFHCE").test_enum("INIT_OUT", &["0", "1"]);
                bctx.mode("BUFHCE").test_enum("CE_TYPE", &["SYNC", "ASYNC"]);

                let ckints = if (lr == 'R' && i < 6) || (lr == 'L' && i >= 6) {
                    0..2
                } else {
                    2..4
                };
                for j in ckints {
                    bctx.build()
                        .tile_mutex(format!("CKINT{j}"), format!("BUFHCE_{lr}{i}"))
                        .mutex("MUX.I", format!("CKINT{j}"))
                        .test_manual("MUX.I", format!("CKINT{j}"))
                        .pip("I", (bels::CLK_HROW, format!("BUFHCE_CKINT{j}")))
                        .commit();
                }
                for olr in ['L', 'R'] {
                    bctx.build()
                        .mutex("MUX.I", format!("HCLK_TEST_{olr}"))
                        .test_manual("MUX.I", format!("HCLK_TEST_{olr}"))
                        .pip("I", (bels::CLK_HROW, format!("HCLK_TEST_OUT_{olr}")))
                        .commit();
                    for j in 0..14 {
                        bctx.build()
                            .tile_mutex(format!("HIN{j}_{olr}"), format!("BUFHCE_{lr}{i}"))
                            .mutex("MUX.I", format!("HIN{j}_{olr}"))
                            .test_manual("MUX.I", format!("HIN{j}_{olr}"))
                            .pip("I", (bels::CLK_HROW, format!("HIN{j}_{olr}")))
                            .commit();
                    }
                }
                for j in 0..32 {
                    bctx.build()
                        .global_mutex("GCLK", "TEST")
                        .extra_tile_attr(
                            Delta::new(0, -13, "CLK_BUFG_REBUF"),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{j}_U"),
                            "1",
                        )
                        .extra_tile_attr(
                            Delta::new(0, 11, "CLK_BUFG_REBUF"),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{j}_D"),
                            "1",
                        )
                        .tile_mutex(format!("GCLK{j}"), format!("BUFHCE_{lr}{i}"))
                        .mutex("MUX.I", format!("GCLK{j}"))
                        .test_manual("MUX.I", format!("GCLK{j}"))
                        .pip("I", (bels::CLK_HROW, format!("GCLK{j}")))
                        .commit();
                }
            }
            let mut bctx = ctx.bel(gclk_test_buf);
            bctx.test_manual("ENABLE", "1")
                .mode("GCLK_TEST_BUF")
                .commit();
            bctx.mode("GCLK_TEST_BUF")
                .test_enum("GCLK_TEST_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("GCLK_TEST_BUF")
                .test_enum("INVERT_INPUT", &["FALSE", "TRUE"]);
            for j in 0..14 {
                bctx.build()
                    .tile_mutex(format!("HIN{j}_{lr}"), "TEST")
                    .mutex("MUX.I", format!("HIN{j}"))
                    .test_manual("MUX.I", format!("HIN{j}"))
                    .pip(
                        (bels::CLK_HROW, format!("HCLK_TEST_IN_{lr}")),
                        (bels::CLK_HROW, format!("HIN{j}_{lr}")),
                    )
                    .commit();
            }
        }
        {
            let mut bctx = ctx.bel(bels::CLK_HROW);
            let cmt = backend.edev.db.get_tile_class("CMT");
            let has_lio = backend.edev.tile_index[cmt]
                .iter()
                .any(|loc| loc.cell.col <= edev.col_clk);
            let has_rio = backend.edev.tile_index[cmt]
                .iter()
                .any(|loc| loc.cell.col > edev.col_clk);
            for i in 0..32 {
                bctx.build()
                    .mutex("CASCO", "CASCO")
                    .mutex(format!("MUX.CASCO{i}"), "CASCI")
                    .test_manual(format!("MUX.CASCO{i}"), "CASCI")
                    .pip(format!("CASCO{i}"), format!("CASCI{i}"))
                    .commit();
                for j in [i, i ^ 1] {
                    bctx.build()
                        .mutex(format!("MUX.CASCO{i}"), format!("GCLK_TEST{j}"))
                        .test_manual(format!("MUX.CASCO{i}"), format!("GCLK_TEST{j}"))
                        .pip(format!("GCLK_TEST{i}"), format!("GCLK{j}_TEST_OUT"))
                        .commit();
                }
                for lr in ['L', 'R'] {
                    bctx.build()
                        .mutex("CASCO", "CASCO")
                        .mutex(format!("MUX.CASCO{i}"), format!("HCLK_TEST_{lr}"))
                        .test_manual(format!("MUX.CASCO{i}"), format!("HCLK_TEST_{lr}"))
                        .pip(format!("CASCO{i}"), format!("HCLK_TEST_OUT_{lr}"))
                        .commit();
                    for j in 0..14 {
                        bctx.build()
                            .mutex("CASCO", "CASCO")
                            .tile_mutex(format!("HIN{j}_{lr}"), format!("CASCO{i}"))
                            .mutex(format!("MUX.CASCO{i}"), format!("HIN{j}_{lr}"))
                            .test_manual(format!("MUX.CASCO{i}"), format!("HIN{j}_{lr}"))
                            .pip(format!("CASCO{i}"), format!("HIN{j}_{lr}"))
                            .commit();
                    }
                    for j in 0..4 {
                        bctx.build()
                            .mutex("CASCO", "CASCO")
                            .global_mutex("RCLK", "USE")
                            .mutex(format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}"))
                            .mutex(format!("MUX.CASCO{}", i ^ 1), format!("RCLK{j}_{lr}"))
                            .pip(format!("CASCO{}", i ^ 1), format!("RCLK{j}_{lr}"))
                            .test_manual(format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}"))
                            .pip(format!("CASCO{i}"), format!("RCLK{j}_{lr}"))
                            .commit();
                    }
                }
                bctx.build()
                    .mutex("CASCO", "TEST_IN")
                    .global_mutex("GCLK", "USE")
                    .tile_mutex(format!("GCLK{i}"), "BUFHCE_L0")
                    .bel_mutex(bels::BUFHCE_W[0], "MUX.I", format!("GCLK{i}"))
                    .pip((bels::BUFHCE_W[0], "I"), format!("GCLK{i}"))
                    .test_manual(format!("GCLK{i}_TEST_IN"), "1")
                    .pip(format!("GCLK{i}_TEST_IN"), format!("GCLK{i}"))
                    .commit();
            }
            for lr in ['L', 'R'] {
                for j in 0..4 {
                    let mut builder = bctx
                        .build()
                        .mutex("CASCO", "CASCO")
                        .global_mutex("RCLK", "TEST_HROW")
                        .mutex("MUX.CASCO0", format!("RCLK{j}_{lr}"));
                    if lr == 'L' {
                        if has_lio {
                            builder = builder.extra_tile_attr(
                                CmtDir(DirH::W),
                                "HCLK_CMT",
                                format!("ENABLE.RCLK{j}"),
                                "HROW",
                            );
                        }
                    } else {
                        if has_rio {
                            builder = builder.extra_tile_attr(
                                CmtDir(DirH::E),
                                "HCLK_CMT",
                                format!("ENABLE.RCLK{j}"),
                                "HROW",
                            );
                        }
                    }
                    builder
                        .test_manual("MUX.CASCO0", format!("RCLK{j}_{lr}.EXCL"))
                        .pip("CASCO0", format!("RCLK{j}_{lr}"))
                        .commit();
                }
            }
        }
    }
    for tile in ["CLK_BUFG_REBUF", "CLK_BALI_REBUF"] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        if tile == "CLK_BUFG_REBUF" && bali_only {
            continue;
        }
        let mut bctx = ctx.bel(bels::CLK_REBUF);
        for i in 0..32 {
            let bel_d = bels::GCLK_TEST_BUF_REBUF_S[i / 2];
            let bel_u = bels::GCLK_TEST_BUF_REBUF_N[i / 2];
            if i.is_multiple_of(2) {
                if edev.chips.values().any(|grid| grid.regs > 1) {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_D0")
                        .pip((bel_d, "CLKIN"), format!("GCLK{i}_D"))
                        .pip(format!("GCLK{i}_U"), (bel_u, "CLKOUT"))
                        .test_manual(format!("BUF.GCLK{i}_D"), "1")
                        .pip(format!("GCLK{i}_D"), format!("GCLK{i}_U"))
                        .commit();
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_U0")
                        .prop(HclkSide(DirV::S))
                        .related_pip(ClkRebuf(DirV::S), format!("GCLK{i}_U"), (bel_u, "CLKOUT"))
                        .related_pip(ClkRebuf(DirV::N), (bel_d, "CLKIN"), format!("GCLK{i}_D"))
                        .test_manual(format!("BUF.GCLK{i}_U"), "1")
                        .pip(format!("GCLK{i}_U"), format!("GCLK{i}_D"))
                        .commit();
                }
                if tile == "CLK_BALI_REBUF" {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_BALI")
                        .prop(HclkSide(DirV::S))
                        .extra_tile_attr(
                            ClkRebuf(DirV::S),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{i}_U"),
                            "1",
                        )
                        .test_manual(format!("ENABLE.GCLK{i}_D"), "1")
                        .pip((bel_d, "CLKIN"), format!("GCLK{i}_D"))
                        .commit();
                }
            } else {
                if edev.chips.values().any(|grid| grid.regs > 1) {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_U1")
                        .pip((bel_u, "CLKIN"), format!("GCLK{i}_U"))
                        .pip(format!("GCLK{i}_D"), (bel_d, "CLKOUT"))
                        .test_manual(format!("BUF.GCLK{i}_U"), "1")
                        .pip(format!("GCLK{i}_U"), format!("GCLK{i}_D"))
                        .commit();
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_D1")
                        .prop(HclkSide(DirV::N))
                        .related_pip(ClkRebuf(DirV::N), format!("GCLK{i}_D"), (bel_d, "CLKOUT"))
                        .related_pip(ClkRebuf(DirV::S), (bel_u, "CLKIN"), format!("GCLK{i}_U"))
                        .test_manual(format!("BUF.GCLK{i}_D"), "1")
                        .pip(format!("GCLK{i}_D"), format!("GCLK{i}_U"))
                        .commit();
                }
                if tile == "CLK_BALI_REBUF" {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_BALI")
                        .prop(HclkSide(DirV::S))
                        .extra_tile_attr(
                            ClkRebuf(DirV::S),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{i}_U"),
                            "1",
                        )
                        .test_manual(format!("ENABLE.GCLK{i}_D"), "1")
                        .pip(format!("GCLK{i}_D"), (bel_d, "CLKOUT"))
                        .commit();
                }
            }
        }
        for slots in [bels::GCLK_TEST_BUF_REBUF_S, bels::GCLK_TEST_BUF_REBUF_N] {
            for i in 0..16 {
                let mut bctx = ctx.bel(slots[i]);
                bctx.test_manual("ENABLE", "1")
                    .mode("GCLK_TEST_BUF")
                    .commit();
                bctx.mode("GCLK_TEST_BUF")
                    .test_enum("GCLK_TEST_ENABLE", &["FALSE", "TRUE"]);
                bctx.mode("GCLK_TEST_BUF")
                    .test_enum("INVERT_INPUT", &["FALSE", "TRUE"]);
            }
        }
    }
    if !bali_only {
        for tile in ["HCLK_IOI_HR", "HCLK_IOI_HP"] {
            let mut ctx = FuzzCtx::new(session, backend, tile);
            for i in 0..4 {
                let mut bctx = ctx.bel(bels::BUFIO[i]);
                bctx.test_manual("ENABLE", "1").mode("BUFIO").commit();
                bctx.mode("BUFIO")
                    .test_enum("DELAY_BYPASS", &["FALSE", "TRUE"]);
                bctx.build()
                    .mutex("MUX.I", "CCIO")
                    .related_tile_mutex(ColPair("CMT"), "CCIO", "USE_IO")
                    .prop(Related::new(
                        ColPair("CMT"),
                        BelMutex::new(
                            bels::HCLK_CMT,
                            format!("MUX.FREQ_BB{i}"),
                            format!("CCIO{i}"),
                        ),
                    ))
                    .related_pip(
                        ColPair("CMT"),
                        (bels::HCLK_CMT, format!("FREQ_BB{i}_MUX")),
                        (bels::HCLK_CMT, format!("CCIO{i}")),
                    )
                    .test_manual("MUX.I", "CCIO")
                    .pip(
                        (bels::HCLK_IOI, format!("IOCLK_IN{i}")),
                        (bels::HCLK_IOI, format!("IOCLK_IN{i}_PAD")),
                    )
                    .commit();
                bctx.build()
                    .mutex("MUX.I", "PERF")
                    .related_tile_mutex(ColPair("CMT"), "PERF", "USE_IO")
                    .related_pip(
                        ColPair("CMT"),
                        (bels::HCLK_CMT, format!("PERF{i}")),
                        (bels::HCLK_CMT, "PHASER_IN_RCLK0"),
                    )
                    .test_manual("MUX.I", "PERF")
                    .pip(
                        (bels::HCLK_IOI, format!("IOCLK_IN{i}")),
                        (bels::HCLK_IOI, format!("IOCLK_IN{i}_PERF")),
                    )
                    .commit();
            }
            for i in 0..4 {
                let mut bctx = ctx.bel(bels::BUFR[i]);
                bctx.test_manual("ENABLE", "1")
                    .mode("BUFR")
                    .attr("BUFR_DIVIDE", "BYPASS")
                    .commit();
                bctx.mode("BUFR").test_enum(
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                for j in 0..4 {
                    bctx.build()
                        .mutex("MUX.I", format!("CKINT{j}"))
                        .test_manual("MUX.I", format!("CKINT{j}"))
                        .pip("I", (bels::HCLK_IOI, format!("BUFR_CKINT{j}")))
                        .commit();
                    bctx.build()
                        .mutex("MUX.I", format!("BUFIO{j}_I"))
                        .test_manual("MUX.I", format!("BUFIO{j}_I"))
                        .pip("I", (bels::HCLK_IOI, format!("IOCLK_IN{j}_BUFR")))
                        .commit();
                }
            }
            {
                let mut bctx = ctx.bel(bels::IDELAYCTRL);
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        bctx.build()
                            .mutex("MUX.REFCLK", format!("HCLK_IO_{ud}{i}"))
                            .test_manual("MUX.REFCLK", format!("HCLK_IO_{ud}{i}"))
                            .pip("REFCLK", (bels::HCLK_IOI, format!("HCLK_IO_{ud}{i}")))
                            .commit();
                    }
                }
                bctx.test_manual("PRESENT", "1").mode("IDELAYCTRL").commit();
                bctx.mode("IDELAYCTRL")
                    .test_enum("HIGH_PERFORMANCE_MODE", &["FALSE", "TRUE"]);
                bctx.mode("IDELAYCTRL")
                    .tile_mutex("IDELAYCTRL", "TEST")
                    .test_manual("MODE", "DEFAULT")
                    .attr("IDELAYCTRL_EN", "DEFAULT")
                    .attr("BIAS_MODE", "2")
                    .commit();
                bctx.mode("IDELAYCTRL")
                    .tile_mutex("IDELAYCTRL", "TEST")
                    .test_manual("MODE", "FULL_0")
                    .attr("IDELAYCTRL_EN", "ENABLE")
                    .attr("BIAS_MODE", "0")
                    .commit();
                bctx.mode("IDELAYCTRL")
                    .tile_mutex("IDELAYCTRL", "TEST")
                    .test_manual("MODE", "FULL_1")
                    .attr("IDELAYCTRL_EN", "ENABLE")
                    .attr("BIAS_MODE", "1")
                    .commit();
            }
            {
                let mut bctx = ctx.bel(bels::HCLK_IOI);
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        for j in 0..12 {
                            bctx.build()
                                .mutex(format!("MUX.HCLK_IO_{ud}{i}"), format!("HCLK{j}"))
                                .mutex(format!("HCLK{j}"), format!("MUX.HCLK_IO_{ud}{i}"))
                                .test_manual(format!("MUX.HCLK_IO_{ud}{i}"), format!("HCLK{j}"))
                                .pip(format!("HCLK_IO_{ud}{i}"), format!("HCLK{j}_BUF"))
                                .commit();
                        }
                    }
                }
                for i in 0..4 {
                    let li = i % 2;
                    let ud = if i < 2 { 'U' } else { 'D' };
                    bctx.build()
                        .global_mutex("RCLK", "USE")
                        .prop(Related::new(
                            ColPair("CMT"),
                            BelMutex::new(
                                bels::HCLK_CMT,
                                format!("MUX.LCLK{li}_{ud}"),
                                format!("RCLK{i}"),
                            ),
                        ))
                        .related_pip(
                            ColPair("CMT"),
                            (bels::HCLK_CMT, format!("LCLK{li}_CMT_{ud}")),
                            (bels::HCLK_CMT, format!("RCLK{i}")),
                        )
                        .test_manual(format!("BUF.RCLK{i}"), "1")
                        .pip(format!("RCLK{i}_IO"), format!("RCLK{i}"))
                        .commit();
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, bali_only: bool) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    if !bali_only {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..12 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, "HCLK_E", "MUX.LCLK0_D", format!("HCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, "HCLK_E", "MUX.LCLK0_U", format!("HCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
        }
        for i in 0..4 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, "HCLK_E", "MUX.LCLK0_D", format!("RCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, "HCLK_E", "MUX.LCLK0_U", format!("RCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.RCLK{i}"), xlat_bit(diff));
        }
        for i in 0..12 {
            let sbel = if i < 6 { "HCLK_E" } else { "HCLK_W" };
            for ud in ['U', 'D'] {
                let mux = &format!("MUX.LCLK{i}_{ud}");
                let mut diffs = vec![("NONE".to_string(), Diff::default())];
                for i in 0..12 {
                    let val = format!("HCLK{i}");
                    let mut diff = ctx.state.get_diff(tile, sbel, mux, &val);
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.HCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                for i in 0..4 {
                    let val = format!("RCLK{i}");
                    let mut diff = ctx.state.get_diff(tile, sbel, mux, &val);
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.RCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                ctx.tiledb
                    .insert(tile, bel, mux, xlat_enum_ocd(diffs, OcdMode::Mux));
            }
        }
    }
    if !bali_only {
        let tile = "CLK_BUFG";
        for i in 0..16 {
            let bel = &format!("BUFGCTRL{i}");
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                ctx.collect_inv(tile, bel, pin);
            }
            for attr in ["PRESELECT_I0", "PRESELECT_I1", "CREATE_EDGE"] {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
            ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

            for attr in ["MUX.I0", "MUX.I1"] {
                ctx.collect_enum_ocd(
                    tile,
                    bel,
                    attr,
                    &["CASCI", "CKINT0", "CKINT1", "FB_PREV", "FB_NEXT"],
                    OcdMode::Mux,
                );
            }
            ctx.collect_bit(tile, bel, "ENABLE.FB", "1");
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE.GCLK", "1");
            ctx.collect_bit(tile, bel, "TEST_I0", "1");
            ctx.collect_bit(tile, bel, "TEST_I1", "1");
        }
    }
    if !bali_only {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        for i in 0..32 {
            let (_, _, diff) = Diff::split(
                ctx.state
                    .peek_diff(tile, "BUFHCE_W0", "MUX.I", format!("GCLK{i}"))
                    .clone(),
                ctx.state
                    .peek_diff(tile, "BUFHCE_E0", "MUX.I", format!("GCLK{i}"))
                    .clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.GCLK{i}"), xlat_bit(diff));
        }
        for lr in ['L', 'R'] {
            for i in 0..14 {
                let (_, _, diff) = Diff::split(
                    ctx.state
                        .peek_diff(tile, "BUFHCE_W0", "MUX.I", format!("HIN{i}_{lr}"))
                        .clone(),
                    ctx.state
                        .peek_diff(tile, "BUFHCE_E0", "MUX.I", format!("HIN{i}_{lr}"))
                        .clone(),
                );
                ctx.tiledb
                    .insert(tile, bel, format!("ENABLE.HIN{i}_{lr}"), xlat_bit(diff));
            }
        }
        for (pin, sbel_a, sbel_b) in [
            ("CKINT0", "BUFHCE_E0", "BUFHCE_E1"),
            ("CKINT1", "BUFHCE_E0", "BUFHCE_E1"),
            ("CKINT2", "BUFHCE_W0", "BUFHCE_W1"),
            ("CKINT3", "BUFHCE_W0", "BUFHCE_W1"),
        ] {
            let (_, _, diff) = Diff::split(
                ctx.state.peek_diff(tile, sbel_a, "MUX.I", pin).clone(),
                ctx.state.peek_diff(tile, sbel_b, "MUX.I", pin).clone(),
            );
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit(diff));
        }
        for we in ['W', 'E'] {
            for i in 0..12 {
                let bel = &format!("BUFHCE_{we}{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_inv(tile, bel, "CE");
                ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");
                ctx.collect_enum(tile, bel, "CE_TYPE", &["SYNC", "ASYNC"]);
                let mut diffs = vec![];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(tile, bel, "MUX.I", format!("GCLK{j}"));
                    diff.apply_bit_diff(
                        ctx.tiledb
                            .item(tile, "CLK_HROW", &format!("ENABLE.GCLK{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("GCLK{j}"), diff));
                }
                for lr in ['L', 'R'] {
                    for j in 0..14 {
                        let mut diff =
                            ctx.state
                                .get_diff(tile, bel, "MUX.I", format!("HIN{j}_{lr}"));
                        diff.apply_bit_diff(
                            ctx.tiledb
                                .item(tile, "CLK_HROW", &format!("ENABLE.HIN{j}_{lr}")),
                            true,
                            false,
                        );
                        diffs.push((format!("HIN{j}_{lr}"), diff));
                    }
                    let diff = ctx
                        .state
                        .get_diff(tile, bel, "MUX.I", format!("HCLK_TEST_{lr}"));
                    diffs.push((format!("HCLK_TEST_{lr}"), diff));
                }
                let ckints = if (we == 'E' && i < 6) || (we == 'W' && i >= 6) {
                    0..2
                } else {
                    2..4
                };
                for j in ckints {
                    let mut diff = ctx.state.get_diff(tile, bel, "MUX.I", format!("CKINT{j}"));
                    diff.apply_bit_diff(
                        ctx.tiledb
                            .item(tile, "CLK_HROW", &format!("ENABLE.CKINT{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("CKINT{j}"), diff));
                }
                diffs.push(("NONE".to_string(), Diff::default()));
                ctx.tiledb
                    .insert(tile, bel, "MUX.I", xlat_enum_ocd(diffs, OcdMode::Mux));
            }
        }
        for (lr, we) in [('L', 'W'), ('R', 'E')] {
            let sbel = &format!("GCLK_TEST_BUF_HROW_BUFH_{we}");
            ctx.state.get_diff(tile, sbel, "ENABLE", "1").assert_empty();
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                .assert_empty();
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                .assert_empty();
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.HCLK_TEST_{we}"), item);
            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for j in 0..14 {
                let mut diff = ctx.state.get_diff(tile, sbel, "MUX.I", format!("HIN{j}"));
                diff.apply_bit_diff(
                    ctx.tiledb
                        .item(tile, "CLK_HROW", &format!("ENABLE.HIN{j}_{lr}")),
                    true,
                    false,
                );
                diffs.push((format!("HIN{j}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.HCLK_TEST_{we}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
        for i in 0..32 {
            let sbel = &format!("GCLK_TEST_BUF_HROW_GCLK{i}");
            let item = ctx.extract_bit(tile, sbel, "ENABLE", "1");
            ctx.tiledb
                .insert(tile, bel, format!("ENABLE.GCLK_TEST{i}"), item);
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                .assert_empty();
            ctx.state
                .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                .assert_empty();
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.GCLK_TEST{i}"), item);

            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for val in ["CASCI", "HCLK_TEST_L", "HCLK_TEST_R"] {
                diffs.push((
                    val.to_string(),
                    ctx.state.get_diff(tile, bel, format!("MUX.CASCO{i}"), val),
                ));
            }
            for lr in ['L', 'R'] {
                for j in 0..14 {
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.CASCO{i}"),
                        format!("HIN{j}_{lr}"),
                    );
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("ENABLE.HIN{j}_{lr}")),
                        true,
                        false,
                    );
                    diffs.push((format!("HIN{j}_{lr}"), diff));
                }
                for j in 0..4 {
                    let diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.CASCO{i}"),
                        format!("RCLK{j}_{lr}"),
                    );
                    if i == 0 {
                        let xdiff = ctx
                            .state
                            .get_diff(
                                tile,
                                bel,
                                format!("MUX.CASCO{i}"),
                                format!("RCLK{j}_{lr}.EXCL"),
                            )
                            .combine(&!&diff);
                        ctx.tiledb.insert(
                            tile,
                            bel,
                            format!("ENABLE.RCLK{j}_{lr}"),
                            xlat_bit(xdiff),
                        );
                    }
                    diffs.push((format!("RCLK{j}_{lr}"), diff));
                }
            }
            for j in [i, i ^ 1] {
                let mut diff = ctx
                    .state
                    .peek_diff(tile, bel, format!("GCLK{j}_TEST_IN"), "1")
                    .clone();
                diff.bits
                    .retain(|&bit, _| diffs.iter().any(|(_, odiff)| odiff.bits.contains_key(&bit)));
                diff = diff.combine(&ctx.state.get_diff(
                    tile,
                    bel,
                    format!("MUX.CASCO{i}"),
                    format!("GCLK_TEST{j}"),
                ));
                diffs.push((format!("GCLK_TEST{j}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.CASCO{i}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
        for i in 0..32 {
            // slurped above bit by bit
            ctx.state
                .get_diff(tile, bel, format!("GCLK{i}_TEST_IN"), "1");
        }
    }
    for tile in ["CLK_BUFG_REBUF", "CLK_BALI_REBUF"] {
        if bali_only && tile == "CLK_BUFG_REBUF" {
            continue;
        }
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "CLK_REBUF";
        for i in 0..16 {
            let sbel = &format!("GCLK_TEST_BUF_REBUF_S{i}");
            let item = if tile == "CLK_BUFG_REBUF" {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                    .assert_empty();
                ctx.extract_bit(tile, sbel, "ENABLE", "1")
            } else {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state.get_diff(tile, sbel, "ENABLE", "1").assert_empty();
                ctx.extract_bit(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
            };
            let s = i * 2;
            let d = i * 2 + 1;
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{d}_D.GCLK{s}_D"), item);
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.GCLK{d}_D.GCLK{s}_D"), item);

            let sbel = &format!("GCLK_TEST_BUF_REBUF_N{i}");
            let item = if tile == "CLK_BUFG_REBUF" {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                    .assert_empty();
                ctx.extract_bit(tile, sbel, "ENABLE", "1")
            } else {
                ctx.state
                    .get_diff(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.state.get_diff(tile, sbel, "ENABLE", "1").assert_empty();
                ctx.extract_bit(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
            };
            let s = i * 2 + 1;
            let d = i * 2;
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{d}_U.GCLK{s}_U"), item);
            let item = ctx.extract_enum_bool(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.tiledb
                .insert(tile, bel, format!("INV.GCLK{d}_U.GCLK{s}_U"), item);
        }
        for i in 0..32 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.GCLK{i}_D"), "1");
            ctx.collect_bit(tile, bel, &format!("ENABLE.GCLK{i}_U"), "1");
            if edev.chips.values().any(|grid| grid.regs > 1) {
                ctx.collect_bit(tile, bel, &format!("BUF.GCLK{i}_D"), "1");
                ctx.collect_bit(tile, bel, &format!("BUF.GCLK{i}_U"), "1");
            }
        }
    }
    if !bali_only {
        let tile = "CMT";
        let bel = "HCLK_CMT";
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("ENABLE.RCLK{i}"), "HROW");
        }
        for tile in ["HCLK_IOI_HP", "HCLK_IOI_HR"] {
            for i in 0..4 {
                let bel = &format!("BUFIO{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_enum_bool(tile, bel, "DELAY_BYPASS", "FALSE", "TRUE");
                ctx.collect_enum(tile, bel, "MUX.I", &["CCIO", "PERF"]);
            }
            for i in 0..4 {
                let bel = &format!("BUFR{i}");
                ctx.collect_bit(tile, bel, "ENABLE", "1");
                ctx.collect_enum(
                    tile,
                    bel,
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                ctx.collect_enum_default(
                    tile,
                    bel,
                    "MUX.I",
                    &[
                        "BUFIO0_I", "BUFIO1_I", "BUFIO2_I", "BUFIO3_I", "CKINT0", "CKINT1",
                        "CKINT2", "CKINT3",
                    ],
                    "NONE",
                );
            }
            {
                let bel = "IDELAYCTRL";
                ctx.collect_enum_default(
                    tile,
                    bel,
                    "MUX.REFCLK",
                    &[
                        "HCLK_IO_D0",
                        "HCLK_IO_D1",
                        "HCLK_IO_D2",
                        "HCLK_IO_D3",
                        "HCLK_IO_D4",
                        "HCLK_IO_D5",
                        "HCLK_IO_U0",
                        "HCLK_IO_U1",
                        "HCLK_IO_U2",
                        "HCLK_IO_U3",
                        "HCLK_IO_U4",
                        "HCLK_IO_U5",
                    ],
                    "NONE",
                );
                ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
                ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
                ctx.collect_enum_default(
                    tile,
                    bel,
                    "MODE",
                    &["DEFAULT", "FULL_0", "FULL_1"],
                    "NONE",
                );
            }
            {
                let bel = "HCLK_IOI";
                for i in 0..4 {
                    ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
                }
                for i in 0..12 {
                    let (_, _, diff) = Diff::split(
                        ctx.state
                            .peek_diff(tile, bel, "MUX.HCLK_IO_D0", format!("HCLK{i}"))
                            .clone(),
                        ctx.state
                            .peek_diff(tile, bel, "MUX.HCLK_IO_U0", format!("HCLK{i}"))
                            .clone(),
                    );
                    ctx.tiledb
                        .insert(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit(diff));
                }
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        let mux = &format!("MUX.HCLK_IO_{ud}{i}");
                        let mut diffs = vec![("NONE".to_string(), Diff::default())];
                        for i in 0..12 {
                            let val = format!("HCLK{i}");
                            let mut diff = ctx.state.get_diff(tile, bel, mux, &val);
                            diff.apply_bit_diff(
                                ctx.tiledb.item(tile, bel, &format!("ENABLE.HCLK{i}")),
                                true,
                                false,
                            );
                            diffs.push((val, diff));
                        }
                        ctx.tiledb
                            .insert(tile, bel, mux, xlat_enum_ocd(diffs, OcdMode::Mux));
                    }
                }
            }
        }
    }
}
