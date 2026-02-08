use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{RowId, TileCoord},
};
use prjcombine_re_collector::{
    diff::{Diff, OcdMode},
    legacy::{xlat_bit_legacy, xlat_enum_legacy_ocd},
};
use prjcombine_re_fpga_hammer::FuzzerProp;
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
            bel::BelMutex,
            relation::{DeltaLegacy, Related, TileRelation},
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
                        cell.row = backend.edev.rows(cell.die).last().unwrap();
                    } else {
                        cell.row -= 1;
                    }
                }
                DirV::N => {
                    if cell.row == backend.edev.rows(cell.die).last().unwrap() {
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
        let mut ctx = FuzzCtx::new_legacy(session, backend, "HCLK");
        let mut bctx = ctx.bel(defs::bslots::HCLK_W);
        for i in 6..12 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    bctx.build()
                        .tile_mutex("MODE", "TEST")
                        .global_mutex("HCLK", "USE")
                        .tile_mutex(format!("MUX.LCLK{i}_{ud}_L"), format!("HCLK{j}"))
                        .tile_mutex(format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}_L"))
                        .has_related(DeltaLegacy::new(0, -1, "INT"))
                        .has_related(DeltaLegacy::new(-2, -1, "INT"))
                        .has_related(DeltaLegacy::new(2, -1, "INT"))
                        .has_related(DeltaLegacy::new(0, 1, "INT"))
                        .has_related(DeltaLegacy::new(-2, 1, "INT"))
                        .has_related(DeltaLegacy::new(2, 1, "INT"))
                        .related_tile_mutex(DeltaLegacy::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            DeltaLegacy::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}_I")
                            } else {
                                format!("HCLK{j}")
                            },
                        )
                        .related_tile_mutex(DeltaLegacy::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            DeltaLegacy::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}_I")
                            } else {
                                format!("HCLK{j}")
                            },
                        )
                        .test_manual_legacy(format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"))
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
                        .has_related(DeltaLegacy::new(0, -1, "INT"))
                        .has_related(DeltaLegacy::new(-2, -1, "INT"))
                        .has_related(DeltaLegacy::new(2, -1, "INT"))
                        .has_related(DeltaLegacy::new(0, 1, "INT"))
                        .has_related(DeltaLegacy::new(-2, 1, "INT"))
                        .has_related(DeltaLegacy::new(2, 1, "INT"))
                        .related_tile_mutex(DeltaLegacy::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            DeltaLegacy::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}"),
                        )
                        .related_tile_mutex(DeltaLegacy::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            DeltaLegacy::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}"),
                        )
                        .test_manual_legacy(format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .pip(format!("LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .commit();
                }
            }
        }
        let mut bctx = ctx.bel(defs::bslots::HCLK_E);
        for i in 0..6 {
            for ud in ['U', 'D'] {
                for j in 0..12 {
                    bctx.build()
                        .tile_mutex("MODE", "TEST")
                        .global_mutex("HCLK", "USE")
                        .tile_mutex(format!("MUX.LCLK{i}_{ud}_R"), format!("HCLK{j}"))
                        .tile_mutex(format!("HCLK{j}"), format!("MUX.LCLK{i}_{ud}_R"))
                        .has_related(DeltaLegacy::new(0, -1, "INT"))
                        .has_related(DeltaLegacy::new(-2, -1, "INT"))
                        .has_related(DeltaLegacy::new(2, -1, "INT"))
                        .has_related(DeltaLegacy::new(0, 1, "INT"))
                        .has_related(DeltaLegacy::new(-2, 1, "INT"))
                        .has_related(DeltaLegacy::new(2, 1, "INT"))
                        .related_tile_mutex(DeltaLegacy::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            DeltaLegacy::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}")
                            } else {
                                format!("HCLK{j}_I")
                            },
                        )
                        .related_tile_mutex(DeltaLegacy::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            DeltaLegacy::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            if j < 8 {
                                format!("HCLK{j}")
                            } else {
                                format!("HCLK{j}_I")
                            },
                        )
                        .test_manual_legacy(format!("MUX.LCLK{i}_{ud}"), format!("HCLK{j}"))
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
                        .has_related(DeltaLegacy::new(0, -1, "INT"))
                        .has_related(DeltaLegacy::new(-2, -1, "INT"))
                        .has_related(DeltaLegacy::new(2, -1, "INT"))
                        .has_related(DeltaLegacy::new(0, 1, "INT"))
                        .has_related(DeltaLegacy::new(-2, 1, "INT"))
                        .has_related(DeltaLegacy::new(2, 1, "INT"))
                        .related_tile_mutex(DeltaLegacy::new(-2, 0, "HCLK"), "MODE", "PIN_L")
                        .related_pip(
                            DeltaLegacy::new(-2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}_I"),
                        )
                        .related_tile_mutex(DeltaLegacy::new(2, 0, "HCLK"), "MODE", "PIN_R")
                        .related_pip(
                            DeltaLegacy::new(2, 0, "HCLK"),
                            format!("LCLK{i}_{ud}"),
                            format!("RCLK{j}_I"),
                        )
                        .test_manual_legacy(format!("MUX.LCLK{i}_{ud}"), format!("RCLK{j}"))
                        .pip(format!("LCLK{i}_{ud}"), format!("RCLK{j}_I"))
                        .commit();
                }
            }
        }
    }
    if !bali_only {
        let mut ctx = FuzzCtx::new_legacy(session, backend, "CLK_BUFG");
        for i in 0..16 {
            let mut bctx = ctx.bel(defs::bslots::BUFGCTRL[i]);
            bctx.test_manual_legacy("ENABLE", "1")
                .mode("BUFGCTRL")
                .commit();
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                bctx.mode("BUFGCTRL").test_inv_legacy(pin);
            }
            bctx.mode("BUFGCTRL")
                .test_enum_legacy("PRESELECT_I0", &["FALSE", "TRUE"]);
            bctx.mode("BUFGCTRL")
                .test_enum_legacy("PRESELECT_I1", &["FALSE", "TRUE"]);
            bctx.mode("BUFGCTRL")
                .test_enum_legacy("CREATE_EDGE", &["FALSE", "TRUE"]);
            bctx.mode("BUFGCTRL")
                .test_enum_legacy("INIT_OUT", &["0", "1"]);
            bctx.build()
                .tile_mutex("FB", "TEST")
                .test_manual_legacy("ENABLE.FB", "1")
                .pip("FB", "O")
                .commit();
            if edev.chips.first().unwrap().regs == 1 {
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_attr_legacy(
                        ClkRebuf(DirV::S),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    )
                    .test_manual_legacy("ENABLE.GCLK", "1")
                    .pip("GCLK", "O")
                    .commit();
            } else {
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_attr_legacy(
                        ClkRebuf(DirV::S),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_U"),
                        "1",
                    )
                    .extra_tile_attr_legacy(
                        ClkRebuf(DirV::N),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{i}_D"),
                        "1",
                    )
                    .prop(HclkSide(DirV::N))
                    .test_manual_legacy("ENABLE.GCLK", "1")
                    .pip("GCLK", "O")
                    .commit();
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_attr_legacy(
                        ClkRebuf(DirV::S),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{ii}_U", ii = i + 16),
                        "1",
                    )
                    .extra_tile_attr_legacy(
                        ClkRebuf(DirV::N),
                        "CLK_REBUF",
                        format!("ENABLE.GCLK{ii}_D", ii = i + 16),
                        "1",
                    )
                    .prop(HclkSide(DirV::S))
                    .test_manual_legacy("ENABLE.GCLK", "1")
                    .pip("GCLK", "O")
                    .commit();
            }
            // ISE bug causes pips to be reversed?
            bctx.build()
                .mutex("MUX.I0", "FB_TEST")
                .test_manual_legacy("TEST_I0", "1")
                .pip("I0", "FB_TEST0")
                .commit();
            bctx.build()
                .mutex("MUX.I1", "FB_TEST")
                .test_manual_legacy("TEST_I1", "1")
                .pip("I1", "FB_TEST1")
                .commit();
            for j in 0..2 {
                bctx.build()
                    .mutex(format!("MUX.I{j}"), "CASCI")
                    .test_manual_legacy(format!("MUX.I{j}"), "CASCI")
                    .pip(format!("I{j}"), format!("CASCI{j}"))
                    .commit();
                for k in 0..2 {
                    bctx.build()
                        .mutex(format!("MUX.I{j}"), format!("CKINT{k}"))
                        .test_manual_legacy(format!("MUX.I{j}"), format!("CKINT{k}"))
                        .pip(format!("I{j}"), format!("CKINT{k}"))
                        .commit();
                }

                let obel_prev = defs::bslots::BUFGCTRL[(i + 15) % 16];
                bctx.build()
                    .tile_mutex("FB", "USE")
                    .mutex(format!("MUX.I{j}"), "FB_PREV")
                    .pip((obel_prev, "FB"), (obel_prev, "O"))
                    .test_manual_legacy(format!("MUX.I{j}"), "FB_PREV")
                    .pip(format!("I{j}"), (obel_prev, "FB"))
                    .commit();
                let obel_next = defs::bslots::BUFGCTRL[(i + 1) % 16];
                bctx.build()
                    .tile_mutex("FB", "USE")
                    .mutex(format!("MUX.I{j}"), "FB_NEXT")
                    .pip((obel_next, "FB"), (obel_next, "O"))
                    .test_manual_legacy(format!("MUX.I{j}"), "FB_NEXT")
                    .pip(format!("I{j}"), (obel_next, "FB"))
                    .commit();
            }
        }
    }
    if !bali_only {
        let mut ctx = FuzzCtx::new_legacy(session, backend, "CLK_HROW");
        for i in 0..32 {
            let mut bctx = ctx.bel(defs::bslots::GCLK_TEST_BUF_HROW_GCLK[i]);
            bctx.test_manual_legacy("ENABLE", "1")
                .mode("GCLK_TEST_BUF")
                .commit();
            bctx.mode("GCLK_TEST_BUF")
                .test_enum_legacy("GCLK_TEST_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("GCLK_TEST_BUF")
                .test_enum_legacy("INVERT_INPUT", &["FALSE", "TRUE"]);
        }
        for (lr, bufhce, gclk_test_buf) in [
            (
                'L',
                defs::bslots::BUFHCE_W,
                defs::bslots::GCLK_TEST_BUF_HROW_BUFH_W,
            ),
            (
                'R',
                defs::bslots::BUFHCE_E,
                defs::bslots::GCLK_TEST_BUF_HROW_BUFH_E,
            ),
        ] {
            for i in 0..12 {
                let mut bctx = ctx.bel(bufhce[i]);
                bctx.test_manual_legacy("ENABLE", "1")
                    .mode("BUFHCE")
                    .commit();
                bctx.mode("BUFHCE").test_inv_legacy("CE");
                bctx.mode("BUFHCE")
                    .test_enum_legacy("INIT_OUT", &["0", "1"]);
                bctx.mode("BUFHCE")
                    .test_enum_legacy("CE_TYPE", &["SYNC", "ASYNC"]);

                let ckints = if (lr == 'R' && i < 6) || (lr == 'L' && i >= 6) {
                    0..2
                } else {
                    2..4
                };
                for j in ckints {
                    bctx.build()
                        .tile_mutex(format!("CKINT{j}"), format!("BUFHCE_{lr}{i}"))
                        .mutex("MUX.I", format!("CKINT{j}"))
                        .test_manual_legacy("MUX.I", format!("CKINT{j}"))
                        .pip("I", (defs::bslots::CLK_HROW_V7, format!("BUFHCE_CKINT{j}")))
                        .commit();
                }
                for olr in ['L', 'R'] {
                    bctx.build()
                        .mutex("MUX.I", format!("HCLK_TEST_{olr}"))
                        .test_manual_legacy("MUX.I", format!("HCLK_TEST_{olr}"))
                        .pip(
                            "I",
                            (defs::bslots::CLK_HROW_V7, format!("HCLK_TEST_OUT_{olr}")),
                        )
                        .commit();
                    for j in 0..14 {
                        bctx.build()
                            .tile_mutex(format!("HIN{j}_{olr}"), format!("BUFHCE_{lr}{i}"))
                            .mutex("MUX.I", format!("HIN{j}_{olr}"))
                            .test_manual_legacy("MUX.I", format!("HIN{j}_{olr}"))
                            .pip("I", (defs::bslots::CLK_HROW_V7, format!("HIN{j}_{olr}")))
                            .commit();
                    }
                }
                for j in 0..32 {
                    bctx.build()
                        .global_mutex("GCLK", "TEST")
                        .extra_tile_attr_legacy(
                            DeltaLegacy::new(0, -13, "CLK_BUFG_REBUF"),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{j}_U"),
                            "1",
                        )
                        .extra_tile_attr_legacy(
                            DeltaLegacy::new(0, 11, "CLK_BUFG_REBUF"),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{j}_D"),
                            "1",
                        )
                        .tile_mutex(format!("GCLK{j}"), format!("BUFHCE_{lr}{i}"))
                        .mutex("MUX.I", format!("GCLK{j}"))
                        .test_manual_legacy("MUX.I", format!("GCLK{j}"))
                        .pip("I", (defs::bslots::CLK_HROW_V7, format!("GCLK{j}")))
                        .commit();
                }
            }
            let mut bctx = ctx.bel(gclk_test_buf);
            bctx.test_manual_legacy("ENABLE", "1")
                .mode("GCLK_TEST_BUF")
                .commit();
            bctx.mode("GCLK_TEST_BUF")
                .test_enum_legacy("GCLK_TEST_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("GCLK_TEST_BUF")
                .test_enum_legacy("INVERT_INPUT", &["FALSE", "TRUE"]);
            for j in 0..14 {
                bctx.build()
                    .tile_mutex(format!("HIN{j}_{lr}"), "TEST")
                    .mutex("MUX.I", format!("HIN{j}"))
                    .test_manual_legacy("MUX.I", format!("HIN{j}"))
                    .pip(
                        (defs::bslots::CLK_HROW_V7, format!("HCLK_TEST_IN_{lr}")),
                        (defs::bslots::CLK_HROW_V7, format!("HIN{j}_{lr}")),
                    )
                    .commit();
            }
        }
        {
            let mut bctx = ctx.bel(defs::bslots::CLK_HROW_V7);
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
                    .test_manual_legacy(format!("MUX.CASCO{i}"), "CASCI")
                    .pip(format!("CASCO{i}"), format!("CASCI{i}"))
                    .commit();
                for j in [i, i ^ 1] {
                    bctx.build()
                        .mutex(format!("MUX.CASCO{i}"), format!("GCLK_TEST{j}"))
                        .test_manual_legacy(format!("MUX.CASCO{i}"), format!("GCLK_TEST{j}"))
                        .pip(format!("GCLK_TEST{i}"), format!("GCLK{j}_TEST_OUT"))
                        .commit();
                }
                for lr in ['L', 'R'] {
                    bctx.build()
                        .mutex("CASCO", "CASCO")
                        .mutex(format!("MUX.CASCO{i}"), format!("HCLK_TEST_{lr}"))
                        .test_manual_legacy(format!("MUX.CASCO{i}"), format!("HCLK_TEST_{lr}"))
                        .pip(format!("CASCO{i}"), format!("HCLK_TEST_OUT_{lr}"))
                        .commit();
                    for j in 0..14 {
                        bctx.build()
                            .mutex("CASCO", "CASCO")
                            .tile_mutex(format!("HIN{j}_{lr}"), format!("CASCO{i}"))
                            .mutex(format!("MUX.CASCO{i}"), format!("HIN{j}_{lr}"))
                            .test_manual_legacy(format!("MUX.CASCO{i}"), format!("HIN{j}_{lr}"))
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
                            .test_manual_legacy(format!("MUX.CASCO{i}"), format!("RCLK{j}_{lr}"))
                            .pip(format!("CASCO{i}"), format!("RCLK{j}_{lr}"))
                            .commit();
                    }
                }
                bctx.build()
                    .mutex("CASCO", "TEST_IN")
                    .global_mutex("GCLK", "USE")
                    .tile_mutex(format!("GCLK{i}"), "BUFHCE_L0")
                    .bel_mutex(defs::bslots::BUFHCE_W[0], "MUX.I", format!("GCLK{i}"))
                    .pip((defs::bslots::BUFHCE_W[0], "I"), format!("GCLK{i}"))
                    .test_manual_legacy(format!("GCLK{i}_TEST_IN"), "1")
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
                            builder = builder.extra_tile_attr_legacy(
                                CmtDir(DirH::W),
                                "HCLK_CMT",
                                format!("ENABLE.RCLK{j}"),
                                "HROW",
                            );
                        }
                    } else {
                        if has_rio {
                            builder = builder.extra_tile_attr_legacy(
                                CmtDir(DirH::E),
                                "HCLK_CMT",
                                format!("ENABLE.RCLK{j}"),
                                "HROW",
                            );
                        }
                    }
                    builder
                        .test_manual_legacy("MUX.CASCO0", format!("RCLK{j}_{lr}.EXCL"))
                        .pip("CASCO0", format!("RCLK{j}_{lr}"))
                        .commit();
                }
            }
        }
    }
    for tile in ["CLK_BUFG_REBUF", "CLK_BALI_REBUF"] {
        let Some(mut ctx) = FuzzCtx::try_new_legacy(session, backend, tile) else {
            continue;
        };
        if tile == "CLK_BUFG_REBUF" && bali_only {
            continue;
        }
        let mut bctx = ctx.bel(defs::bslots::CLK_REBUF);
        for i in 0..32 {
            let bel_d = defs::bslots::GCLK_TEST_BUF_REBUF_S[i / 2];
            let bel_u = defs::bslots::GCLK_TEST_BUF_REBUF_N[i / 2];
            if i.is_multiple_of(2) {
                if edev.chips.values().any(|grid| grid.regs > 1) {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_D0")
                        .pip((bel_d, "CLKIN"), format!("GCLK{i}_D"))
                        .pip(format!("GCLK{i}_U"), (bel_u, "CLKOUT"))
                        .test_manual_legacy(format!("BUF.GCLK{i}_D"), "1")
                        .pip(format!("GCLK{i}_D"), format!("GCLK{i}_U"))
                        .commit();
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_U0")
                        .prop(HclkSide(DirV::S))
                        .related_pip(ClkRebuf(DirV::S), format!("GCLK{i}_U"), (bel_u, "CLKOUT"))
                        .related_pip(ClkRebuf(DirV::N), (bel_d, "CLKIN"), format!("GCLK{i}_D"))
                        .test_manual_legacy(format!("BUF.GCLK{i}_U"), "1")
                        .pip(format!("GCLK{i}_U"), format!("GCLK{i}_D"))
                        .commit();
                }
                if tile == "CLK_BALI_REBUF" {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_BALI")
                        .prop(HclkSide(DirV::S))
                        .extra_tile_attr_legacy(
                            ClkRebuf(DirV::S),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{i}_U"),
                            "1",
                        )
                        .test_manual_legacy(format!("ENABLE.GCLK{i}_D"), "1")
                        .pip((bel_d, "CLKIN"), format!("GCLK{i}_D"))
                        .commit();
                }
            } else {
                if edev.chips.values().any(|grid| grid.regs > 1) {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_U1")
                        .pip((bel_u, "CLKIN"), format!("GCLK{i}_U"))
                        .pip(format!("GCLK{i}_D"), (bel_d, "CLKOUT"))
                        .test_manual_legacy(format!("BUF.GCLK{i}_U"), "1")
                        .pip(format!("GCLK{i}_U"), format!("GCLK{i}_D"))
                        .commit();
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_D1")
                        .prop(HclkSide(DirV::N))
                        .related_pip(ClkRebuf(DirV::N), format!("GCLK{i}_D"), (bel_d, "CLKOUT"))
                        .related_pip(ClkRebuf(DirV::S), (bel_u, "CLKIN"), format!("GCLK{i}_U"))
                        .test_manual_legacy(format!("BUF.GCLK{i}_D"), "1")
                        .pip(format!("GCLK{i}_D"), format!("GCLK{i}_U"))
                        .commit();
                }
                if tile == "CLK_BALI_REBUF" {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_BALI")
                        .prop(HclkSide(DirV::S))
                        .extra_tile_attr_legacy(
                            ClkRebuf(DirV::S),
                            "CLK_REBUF",
                            format!("ENABLE.GCLK{i}_U"),
                            "1",
                        )
                        .test_manual_legacy(format!("ENABLE.GCLK{i}_D"), "1")
                        .pip(format!("GCLK{i}_D"), (bel_d, "CLKOUT"))
                        .commit();
                }
            }
        }
        for slots in [
            defs::bslots::GCLK_TEST_BUF_REBUF_S,
            defs::bslots::GCLK_TEST_BUF_REBUF_N,
        ] {
            for i in 0..16 {
                let mut bctx = ctx.bel(slots[i]);
                bctx.test_manual_legacy("ENABLE", "1")
                    .mode("GCLK_TEST_BUF")
                    .commit();
                bctx.mode("GCLK_TEST_BUF")
                    .test_enum_legacy("GCLK_TEST_ENABLE", &["FALSE", "TRUE"]);
                bctx.mode("GCLK_TEST_BUF")
                    .test_enum_legacy("INVERT_INPUT", &["FALSE", "TRUE"]);
            }
        }
    }
    if !bali_only {
        for tile in ["HCLK_IO_HR", "HCLK_IO_HP"] {
            let mut ctx = FuzzCtx::new_legacy(session, backend, tile);
            for i in 0..4 {
                let mut bctx = ctx.bel(defs::bslots::BUFIO[i]);
                bctx.test_manual_legacy("ENABLE", "1")
                    .mode("BUFIO")
                    .commit();
                bctx.mode("BUFIO")
                    .test_enum_legacy("DELAY_BYPASS", &["FALSE", "TRUE"]);
                bctx.build()
                    .mutex("MUX.I", "CCIO")
                    .related_tile_mutex(ColPair("CMT"), "CCIO", "USE_IO")
                    .prop(Related::new(
                        ColPair("CMT"),
                        BelMutex::new(
                            defs::bslots::HCLK_CMT,
                            format!("MUX.FREQ_BB{i}"),
                            format!("CCIO{i}").into(),
                        ),
                    ))
                    .related_pip(
                        ColPair("CMT"),
                        (defs::bslots::HCLK_CMT, format!("FREQ_BB{i}_MUX")),
                        (defs::bslots::HCLK_CMT, format!("CCIO{i}")),
                    )
                    .test_manual_legacy("MUX.I", "CCIO")
                    .pip(
                        (defs::bslots::HCLK_IO, format!("IOCLK_IN{i}")),
                        (defs::bslots::HCLK_IO, format!("IOCLK_IN{i}_PAD")),
                    )
                    .commit();
                bctx.build()
                    .mutex("MUX.I", "PERF")
                    .related_tile_mutex(ColPair("CMT"), "PERF", "USE_IO")
                    .related_pip(
                        ColPair("CMT"),
                        (defs::bslots::HCLK_CMT, format!("PERF{i}")),
                        (defs::bslots::HCLK_CMT, "PHASER_IN_RCLK0"),
                    )
                    .test_manual_legacy("MUX.I", "PERF")
                    .pip(
                        (defs::bslots::HCLK_IO, format!("IOCLK_IN{i}")),
                        (defs::bslots::HCLK_IO, format!("IOCLK_IN{i}_PERF")),
                    )
                    .commit();
            }
            for i in 0..4 {
                let mut bctx = ctx.bel(defs::bslots::BUFR[i]);
                bctx.test_manual_legacy("ENABLE", "1")
                    .mode("BUFR")
                    .attr("BUFR_DIVIDE", "BYPASS")
                    .commit();
                bctx.mode("BUFR").test_enum_legacy(
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                for j in 0..4 {
                    bctx.build()
                        .mutex("MUX.I", format!("CKINT{j}"))
                        .test_manual_legacy("MUX.I", format!("CKINT{j}"))
                        .pip("I", (defs::bslots::HCLK_IO, format!("BUFR_CKINT{j}")))
                        .commit();
                    bctx.build()
                        .mutex("MUX.I", format!("BUFIO{j}_I"))
                        .test_manual_legacy("MUX.I", format!("BUFIO{j}_I"))
                        .pip("I", (defs::bslots::HCLK_IO, format!("IOCLK_IN{j}_BUFR")))
                        .commit();
                }
            }
            {
                let mut bctx = ctx.bel(defs::bslots::IDELAYCTRL);
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        bctx.build()
                            .mutex("MUX.REFCLK", format!("HCLK_IO_{ud}{i}"))
                            .test_manual_legacy("MUX.REFCLK", format!("HCLK_IO_{ud}{i}"))
                            .pip(
                                "REFCLK",
                                (defs::bslots::HCLK_IO, format!("HCLK_IO_{ud}{i}")),
                            )
                            .commit();
                    }
                }
                bctx.test_manual_legacy("PRESENT", "1")
                    .mode("IDELAYCTRL")
                    .commit();
                bctx.mode("IDELAYCTRL")
                    .test_enum_legacy("HIGH_PERFORMANCE_MODE", &["FALSE", "TRUE"]);
                bctx.mode("IDELAYCTRL")
                    .tile_mutex("IDELAYCTRL", "TEST")
                    .test_manual_legacy("MODE", "DEFAULT")
                    .attr("IDELAYCTRL_EN", "DEFAULT")
                    .attr("BIAS_MODE", "2")
                    .commit();
                bctx.mode("IDELAYCTRL")
                    .tile_mutex("IDELAYCTRL", "TEST")
                    .test_manual_legacy("MODE", "FULL_0")
                    .attr("IDELAYCTRL_EN", "ENABLE")
                    .attr("BIAS_MODE", "0")
                    .commit();
                bctx.mode("IDELAYCTRL")
                    .tile_mutex("IDELAYCTRL", "TEST")
                    .test_manual_legacy("MODE", "FULL_1")
                    .attr("IDELAYCTRL_EN", "ENABLE")
                    .attr("BIAS_MODE", "1")
                    .commit();
            }
            {
                let mut bctx = ctx.bel(defs::bslots::HCLK_IO);
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        for j in 0..12 {
                            bctx.build()
                                .mutex(format!("MUX.HCLK_IO_{ud}{i}"), format!("HCLK{j}"))
                                .mutex(format!("HCLK{j}"), format!("MUX.HCLK_IO_{ud}{i}"))
                                .test_manual_legacy(
                                    format!("MUX.HCLK_IO_{ud}{i}"),
                                    format!("HCLK{j}"),
                                )
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
                                defs::bslots::HCLK_CMT,
                                format!("MUX.LCLK{li}_{ud}"),
                                format!("RCLK{i}").into(),
                            ),
                        ))
                        .related_pip(
                            ColPair("CMT"),
                            (defs::bslots::HCLK_CMT, format!("LCLK{li}_CMT_{ud}")),
                            (defs::bslots::HCLK_CMT, format!("RCLK{i}")),
                        )
                        .test_manual_legacy(format!("BUF.RCLK{i}"), "1")
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
                ctx.peek_diff_legacy(tile, "HCLK_E", "MUX.LCLK0_D", format!("HCLK{i}"))
                    .clone(),
                ctx.peek_diff_legacy(tile, "HCLK_E", "MUX.LCLK0_U", format!("HCLK{i}"))
                    .clone(),
            );
            ctx.insert_legacy(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit_legacy(diff));
        }
        for i in 0..4 {
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_legacy(tile, "HCLK_E", "MUX.LCLK0_D", format!("RCLK{i}"))
                    .clone(),
                ctx.peek_diff_legacy(tile, "HCLK_E", "MUX.LCLK0_U", format!("RCLK{i}"))
                    .clone(),
            );
            ctx.insert_legacy(tile, bel, format!("ENABLE.RCLK{i}"), xlat_bit_legacy(diff));
        }
        for i in 0..12 {
            let sbel = if i < 6 { "HCLK_E" } else { "HCLK_W" };
            for ud in ['U', 'D'] {
                let mux = &format!("MUX.LCLK{i}_{ud}");
                let mut diffs = vec![("NONE".to_string(), Diff::default())];
                for i in 0..12 {
                    let val = format!("HCLK{i}");
                    let mut diff = ctx.get_diff_legacy(tile, sbel, mux, &val);
                    diff.apply_bit_diff_legacy(
                        ctx.item_legacy(tile, bel, &format!("ENABLE.HCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                for i in 0..4 {
                    let val = format!("RCLK{i}");
                    let mut diff = ctx.get_diff_legacy(tile, sbel, mux, &val);
                    diff.apply_bit_diff_legacy(
                        ctx.item_legacy(tile, bel, &format!("ENABLE.RCLK{i}")),
                        true,
                        false,
                    );
                    diffs.push((val, diff));
                }
                ctx.insert_legacy(tile, bel, mux, xlat_enum_legacy_ocd(diffs, OcdMode::Mux));
            }
        }
    }
    if !bali_only {
        let tile = "CLK_BUFG";
        for i in 0..16 {
            let bel = &format!("BUFGCTRL[{i}]");
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                ctx.collect_inv(tile, bel, pin);
            }
            for attr in ["PRESELECT_I0", "PRESELECT_I1", "CREATE_EDGE"] {
                ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
            }
            ctx.collect_bit_bi_legacy(tile, bel, "INIT_OUT", "0", "1");

            for attr in ["MUX.I0", "MUX.I1"] {
                ctx.collect_enum_legacy_ocd(
                    tile,
                    bel,
                    attr,
                    &["CASCI", "CKINT0", "CKINT1", "FB_PREV", "FB_NEXT"],
                    OcdMode::Mux,
                );
            }
            ctx.collect_bit_legacy(tile, bel, "ENABLE.FB", "1");
            ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
            ctx.collect_bit_legacy(tile, bel, "ENABLE.GCLK", "1");
            ctx.collect_bit_legacy(tile, bel, "TEST_I0", "1");
            ctx.collect_bit_legacy(tile, bel, "TEST_I1", "1");
        }
    }
    if !bali_only {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW_V7";
        for i in 0..32 {
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_legacy(tile, "BUFHCE_W[0]", "MUX.I", format!("GCLK{i}"))
                    .clone(),
                ctx.peek_diff_legacy(tile, "BUFHCE_E[0]", "MUX.I", format!("GCLK{i}"))
                    .clone(),
            );
            ctx.insert_legacy(tile, bel, format!("ENABLE.GCLK{i}"), xlat_bit_legacy(diff));
        }
        for lr in ['L', 'R'] {
            for i in 0..14 {
                let (_, _, diff) = Diff::split(
                    ctx.peek_diff_legacy(tile, "BUFHCE_W[0]", "MUX.I", format!("HIN{i}_{lr}"))
                        .clone(),
                    ctx.peek_diff_legacy(tile, "BUFHCE_E[0]", "MUX.I", format!("HIN{i}_{lr}"))
                        .clone(),
                );
                ctx.insert_legacy(
                    tile,
                    bel,
                    format!("ENABLE.HIN{i}_{lr}"),
                    xlat_bit_legacy(diff),
                );
            }
        }
        for (pin, sbel_a, sbel_b) in [
            ("CKINT0", "BUFHCE_E[0]", "BUFHCE_E[1]"),
            ("CKINT1", "BUFHCE_E[0]", "BUFHCE_E[1]"),
            ("CKINT2", "BUFHCE_W[0]", "BUFHCE_W[1]"),
            ("CKINT3", "BUFHCE_W[0]", "BUFHCE_W[1]"),
        ] {
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_legacy(tile, sbel_a, "MUX.I", pin).clone(),
                ctx.peek_diff_legacy(tile, sbel_b, "MUX.I", pin).clone(),
            );
            ctx.insert_legacy(tile, bel, format!("ENABLE.{pin}"), xlat_bit_legacy(diff));
        }
        for we in ['W', 'E'] {
            for i in 0..12 {
                let bel = &format!("BUFHCE_{we}[{i}]");
                ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
                ctx.collect_inv(tile, bel, "CE");
                ctx.collect_bit_bi_legacy(tile, bel, "INIT_OUT", "0", "1");
                ctx.collect_enum_legacy(tile, bel, "CE_TYPE", &["SYNC", "ASYNC"]);
                let mut diffs = vec![];
                for j in 0..32 {
                    let mut diff = ctx.get_diff_legacy(tile, bel, "MUX.I", format!("GCLK{j}"));
                    diff.apply_bit_diff_legacy(
                        ctx.item_legacy(tile, "CLK_HROW_V7", &format!("ENABLE.GCLK{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("GCLK{j}"), diff));
                }
                for lr in ['L', 'R'] {
                    for j in 0..14 {
                        let mut diff =
                            ctx.get_diff_legacy(tile, bel, "MUX.I", format!("HIN{j}_{lr}"));
                        diff.apply_bit_diff_legacy(
                            ctx.item_legacy(tile, "CLK_HROW_V7", &format!("ENABLE.HIN{j}_{lr}")),
                            true,
                            false,
                        );
                        diffs.push((format!("HIN{j}_{lr}"), diff));
                    }
                    let diff = ctx.get_diff_legacy(tile, bel, "MUX.I", format!("HCLK_TEST_{lr}"));
                    diffs.push((format!("HCLK_TEST_{lr}"), diff));
                }
                let ckints = if (we == 'E' && i < 6) || (we == 'W' && i >= 6) {
                    0..2
                } else {
                    2..4
                };
                for j in ckints {
                    let mut diff = ctx.get_diff_legacy(tile, bel, "MUX.I", format!("CKINT{j}"));
                    diff.apply_bit_diff_legacy(
                        ctx.item_legacy(tile, "CLK_HROW_V7", &format!("ENABLE.CKINT{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("CKINT{j}"), diff));
                }
                diffs.push(("NONE".to_string(), Diff::default()));
                ctx.insert_legacy(
                    tile,
                    bel,
                    "MUX.I",
                    xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
                );
            }
        }
        for (lr, we) in [('L', 'W'), ('R', 'E')] {
            let sbel = &format!("GCLK_TEST_BUF_HROW_BUFH_{we}");
            ctx.get_diff_legacy(tile, sbel, "ENABLE", "1")
                .assert_empty();
            ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                .assert_empty();
            ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                .assert_empty();
            let item = ctx.extract_bit_bi_legacy(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.insert_legacy(tile, bel, format!("INV.HCLK_TEST_{we}"), item);
            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for j in 0..14 {
                let mut diff = ctx.get_diff_legacy(tile, sbel, "MUX.I", format!("HIN{j}"));
                diff.apply_bit_diff_legacy(
                    ctx.item_legacy(tile, "CLK_HROW_V7", &format!("ENABLE.HIN{j}_{lr}")),
                    true,
                    false,
                );
                diffs.push((format!("HIN{j}"), diff));
            }
            ctx.insert_legacy(
                tile,
                bel,
                format!("MUX.HCLK_TEST_{we}"),
                xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
            );
        }
        for i in 0..32 {
            let sbel = &format!("GCLK_TEST_BUF_HROW_GCLK[{i}]");
            let item = ctx.extract_bit_legacy(tile, sbel, "ENABLE", "1");
            ctx.insert_legacy(tile, bel, format!("ENABLE.GCLK_TEST{i}"), item);
            ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                .assert_empty();
            ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                .assert_empty();
            let item = ctx.extract_bit_bi_legacy(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.insert_legacy(tile, bel, format!("INV.GCLK_TEST{i}"), item);

            let mut diffs = vec![("NONE".to_string(), Diff::default())];
            for val in ["CASCI", "HCLK_TEST_L", "HCLK_TEST_R"] {
                diffs.push((
                    val.to_string(),
                    ctx.get_diff_legacy(tile, bel, format!("MUX.CASCO{i}"), val),
                ));
            }
            for lr in ['L', 'R'] {
                for j in 0..14 {
                    let mut diff = ctx.get_diff_legacy(
                        tile,
                        bel,
                        format!("MUX.CASCO{i}"),
                        format!("HIN{j}_{lr}"),
                    );
                    diff.apply_bit_diff_legacy(
                        ctx.item_legacy(tile, bel, &format!("ENABLE.HIN{j}_{lr}")),
                        true,
                        false,
                    );
                    diffs.push((format!("HIN{j}_{lr}"), diff));
                }
                for j in 0..4 {
                    let diff = ctx.get_diff_legacy(
                        tile,
                        bel,
                        format!("MUX.CASCO{i}"),
                        format!("RCLK{j}_{lr}"),
                    );
                    if i == 0 {
                        let xdiff = ctx
                            .get_diff_legacy(
                                tile,
                                bel,
                                format!("MUX.CASCO{i}"),
                                format!("RCLK{j}_{lr}.EXCL"),
                            )
                            .combine(&!&diff);
                        ctx.insert_legacy(
                            tile,
                            bel,
                            format!("ENABLE.RCLK{j}_{lr}"),
                            xlat_bit_legacy(xdiff),
                        );
                    }
                    diffs.push((format!("RCLK{j}_{lr}"), diff));
                }
            }
            for j in [i, i ^ 1] {
                let mut diff = ctx
                    .peek_diff_legacy(tile, bel, format!("GCLK{j}_TEST_IN"), "1")
                    .clone();
                diff.bits
                    .retain(|&bit, _| diffs.iter().any(|(_, odiff)| odiff.bits.contains_key(&bit)));
                diff = diff.combine(&ctx.get_diff_legacy(
                    tile,
                    bel,
                    format!("MUX.CASCO{i}"),
                    format!("GCLK_TEST{j}"),
                ));
                diffs.push((format!("GCLK_TEST{j}"), diff));
            }
            ctx.insert_legacy(
                tile,
                bel,
                format!("MUX.CASCO{i}"),
                xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
            );
        }
        for i in 0..32 {
            // slurped above bit by bit
            ctx.get_diff_legacy(tile, bel, format!("GCLK{i}_TEST_IN"), "1");
        }
    }
    for tile in ["CLK_BUFG_REBUF", "CLK_BALI_REBUF"] {
        if bali_only && tile == "CLK_BUFG_REBUF" {
            continue;
        }
        if !ctx.has_tile_legacy(tile) {
            continue;
        }
        let bel = "CLK_REBUF";
        for i in 0..16 {
            let sbel = &format!("GCLK_TEST_BUF_REBUF_S[{i}]");
            let item = if tile == "CLK_BUFG_REBUF" {
                ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                    .assert_empty();
                ctx.extract_bit_legacy(tile, sbel, "ENABLE", "1")
            } else {
                ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.get_diff_legacy(tile, sbel, "ENABLE", "1")
                    .assert_empty();
                ctx.extract_bit_legacy(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
            };
            let s = i * 2;
            let d = i * 2 + 1;
            ctx.insert_legacy(tile, bel, format!("BUF.GCLK{d}_D.GCLK{s}_D"), item);
            let item = ctx.extract_bit_bi_legacy(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.insert_legacy(tile, bel, format!("INV.GCLK{d}_D.GCLK{s}_D"), item);

            let sbel = &format!("GCLK_TEST_BUF_REBUF_N[{i}]");
            let item = if tile == "CLK_BUFG_REBUF" {
                ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
                    .assert_empty();
                ctx.extract_bit_legacy(tile, sbel, "ENABLE", "1")
            } else {
                ctx.get_diff_legacy(tile, sbel, "GCLK_TEST_ENABLE", "FALSE")
                    .assert_empty();
                ctx.get_diff_legacy(tile, sbel, "ENABLE", "1")
                    .assert_empty();
                ctx.extract_bit_legacy(tile, sbel, "GCLK_TEST_ENABLE", "TRUE")
            };
            let s = i * 2 + 1;
            let d = i * 2;
            ctx.insert_legacy(tile, bel, format!("BUF.GCLK{d}_U.GCLK{s}_U"), item);
            let item = ctx.extract_bit_bi_legacy(tile, sbel, "INVERT_INPUT", "FALSE", "TRUE");
            ctx.insert_legacy(tile, bel, format!("INV.GCLK{d}_U.GCLK{s}_U"), item);
        }
        for i in 0..32 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.GCLK{i}_D"), "1");
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.GCLK{i}_U"), "1");
            if edev.chips.values().any(|grid| grid.regs > 1) {
                ctx.collect_bit_legacy(tile, bel, &format!("BUF.GCLK{i}_D"), "1");
                ctx.collect_bit_legacy(tile, bel, &format!("BUF.GCLK{i}_U"), "1");
            }
        }
    }
    if !bali_only {
        let tile = "CMT";
        let bel = "HCLK_CMT";
        for i in 0..4 {
            ctx.collect_bit_legacy(tile, bel, &format!("ENABLE.RCLK{i}"), "HROW");
        }
        for tile in ["HCLK_IO_HP", "HCLK_IO_HR"] {
            for i in 0..4 {
                let bel = &format!("BUFIO[{i}]");
                ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
                ctx.collect_bit_bi_legacy(tile, bel, "DELAY_BYPASS", "FALSE", "TRUE");
                ctx.collect_enum_legacy(tile, bel, "MUX.I", &["CCIO", "PERF"]);
            }
            for i in 0..4 {
                let bel = &format!("BUFR[{i}]");
                ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
                ctx.collect_enum_legacy(
                    tile,
                    bel,
                    "BUFR_DIVIDE",
                    &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
                );
                ctx.collect_enum_default_legacy(
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
                ctx.collect_enum_default_legacy(
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
                ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
                    .assert_empty();
                ctx.collect_bit_bi_legacy(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
                ctx.collect_enum_default_legacy(
                    tile,
                    bel,
                    "MODE",
                    &["DEFAULT", "FULL_0", "FULL_1"],
                    "NONE",
                );
            }
            {
                let bel = "HCLK_IO";
                for i in 0..4 {
                    ctx.collect_bit_legacy(tile, bel, &format!("BUF.RCLK{i}"), "1");
                }
                for i in 0..12 {
                    let (_, _, diff) = Diff::split(
                        ctx.peek_diff_legacy(tile, bel, "MUX.HCLK_IO_D0", format!("HCLK{i}"))
                            .clone(),
                        ctx.peek_diff_legacy(tile, bel, "MUX.HCLK_IO_U0", format!("HCLK{i}"))
                            .clone(),
                    );
                    ctx.insert_legacy(tile, bel, format!("ENABLE.HCLK{i}"), xlat_bit_legacy(diff));
                }
                for i in 0..6 {
                    for ud in ['U', 'D'] {
                        let mux = &format!("MUX.HCLK_IO_{ud}{i}");
                        let mut diffs = vec![("NONE".to_string(), Diff::default())];
                        for i in 0..12 {
                            let val = format!("HCLK{i}");
                            let mut diff = ctx.get_diff_legacy(tile, bel, mux, &val);
                            diff.apply_bit_diff_legacy(
                                ctx.item_legacy(tile, bel, &format!("ENABLE.HCLK{i}")),
                                true,
                                false,
                            );
                            diffs.push((val, diff));
                        }
                        ctx.insert_legacy(
                            tile,
                            bel,
                            mux,
                            xlat_enum_legacy_ocd(diffs, OcdMode::Mux),
                        );
                    }
                }
            }
        }
    }
}
