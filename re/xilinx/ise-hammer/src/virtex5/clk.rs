use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{DieId, TileCoord};
use prjcombine_re_fpga_hammer::{
    backend::{FuzzerFeature, FuzzerProp},
    diff::{Diff, DiffKey, FeatureId, OcdMode, xlat_bit, xlat_bit_wide, xlat_enum_ocd},
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::defs;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{DynProp, pip::PinFar, relation::TileRelation},
    },
};

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
struct HclkCmt;

impl TileRelation for HclkCmt {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let row = edev.chips[tcrd.die].row_hclk(tcrd.row);
        Some(tcrd.with_row(row).tile(defs::tslots::HCLK_CMT))
    }
}

#[derive(Clone, Debug)]
struct HclkBramMgtPrev(String, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for HclkBramMgtPrev {
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
        let col = if tcrd.col < edev.col_clk {
            let mut range = chip.cols_mgt_buf.range(..tcrd.col);
            range.next_back()
        } else {
            let mut range = chip.cols_mgt_buf.range((tcrd.col + 1)..);
            range.next()
        };
        let mut sad = true;
        if let Some(&col) = col {
            let ntcrd = tcrd.with_col(col).tile(defs::tslots::CLK);
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "HCLK_MGT_BUF".into(),
                    bel: "HCLK_MGT_BUF".into(),
                    attr: self.0.clone(),
                    val: self.1.into(),
                }),
                rects: edev.tile_bits(ntcrd),
            });
            sad = false;
        }
        Some((fuzzer, sad))
    }
}

#[derive(Clone, Debug)]
struct HclkIoiCenter(&'static str, &'static str, String, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for HclkIoiCenter {
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
        let mut sad = true;
        if tcrd.col <= edev.col_clk
            && let Some(ntcrd) = backend
                .edev
                .find_tile_by_class(tcrd.with_col(edev.col_clk), |kind| kind == self.0)
        {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: self.0.into(),
                    bel: self.1.into(),
                    attr: self.2.clone(),
                    val: self.3.into(),
                }),
                rects: edev.tile_bits(ntcrd),
            });
            sad = false;
        }
        Some((fuzzer, sad))
    }
}

#[derive(Clone, Debug)]
struct AllIodelay(&'static str, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for AllIodelay {
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
        let bot = chip.row_reg_bot(chip.row_to_reg(tcrd.row));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            let Some(ntcrd) =
                edev.find_tile_by_bel(tcrd.with_row(row).bel(defs::bslots::IODELAY[0]))
            else {
                continue;
            };
            for bel in [defs::bslots::IODELAY[0], defs::bslots::IODELAY[1]] {
                if let Some(site) = backend.ngrid.get_bel_name(ntcrd.bel(bel)) {
                    fuzzer = fuzzer.fuzz(Key::SiteMode(site), None, "IODELAY");
                    fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "IDELAY_TYPE".into()), None, self.0);
                }
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "IO".into(),
                    bel: "IODELAY_BOTH".into(),
                    attr: "IDELAYCTRL_MODE".into(),
                    val: self.1.into(),
                }),
                rects: edev.tile_bits(ntcrd),
            });
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
            bctx.build()
                .test_manual("PRESENT", "1")
                .mode("BUFGCTRL")
                .commit();
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

            for j in 0..2 {
                for val in ["CKINT0", "CKINT1"] {
                    bctx.build()
                        .mutex(format!("MUX.I{j}"), val)
                        .test_manual(format!("MUX.I{j}"), val)
                        .pip(format!("I{j}MUX"), val)
                        .commit();
                }
                bctx.build()
                    .mutex(format!("MUX.I{j}"), "MUXBUS")
                    .test_manual(format!("MUX.I{j}"), "MUXBUS")
                    .pip(format!("I{j}MUX"), format!("MUXBUS{j}"))
                    .commit();
                for k in 0..16 {
                    let obel = defs::bslots::BUFGCTRL[if i < 16 { k } else { k + 16 }];
                    let val = format!("GFB{k}");
                    bctx.build()
                        .mutex(format!("MUX.I{j}"), &val)
                        .test_manual(format!("MUX.I{j}"), val)
                        .pip(format!("I{j}MUX"), (obel, "GFB"))
                        .commit();
                }
                for k in 0..5 {
                    for lr in ['L', 'R'] {
                        let val = format!("MGT_{lr}{k}");
                        let pin = format!("MGT_O_{lr}{k}");
                        let obel = if i < 16 {
                            defs::bslots::BUFG_MGTCLK_S
                        } else {
                            defs::bslots::BUFG_MGTCLK_N
                        };
                        bctx.build()
                            .mutex(format!("MUX.I{j}"), &val)
                            .test_manual(format!("MUX.I{j}"), &val)
                            .pip(format!("I{j}MUX"), (obel, pin))
                            .commit();
                    }
                }
            }
            bctx.build()
                .test_manual("I0_FABRIC_OUT", "1")
                .pin_pips("I0MUX")
                .commit();
            bctx.build()
                .test_manual("I1_FABRIC_OUT", "1")
                .pin_pips("I1MUX")
                .commit();
        }
        for bel in [defs::bslots::BUFG_MGTCLK_S, defs::bslots::BUFG_MGTCLK_N] {
            let mut bctx = ctx.bel(bel);
            for i in 0..5 {
                for lr in ['L', 'R'] {
                    if lr == 'L' && edev.col_lgt.is_none() {
                        continue;
                    }
                    bctx.build()
                        .test_manual(format!("BUF.MGT_{lr}{i}"), "1")
                        .pip(format!("MGT_O_{lr}{i}"), format!("MGT_I_{lr}{i}"))
                        .commit();
                }
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK");
        let mut bctx = ctx.bel(defs::bslots::HCLK);
        for i in 0..10 {
            bctx.test_manual(format!("BUF.HCLK{i}"), "1")
                .pip(format!("HCLK_O{i}"), format!("HCLK_I{i}"))
                .commit();
        }
        for i in 0..4 {
            bctx.build()
                .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                .related_pip(
                    Rclk,
                    (defs::bslots::IOCLK, format!("RCLK_I{i}")),
                    (defs::bslots::IOCLK, "VRCLK0"),
                )
                .test_manual(format!("BUF.RCLK{i}"), "1")
                .pip(format!("RCLK_O{i}"), format!("RCLK_I{i}"))
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_HROW");
        let mut bctx = ctx.bel(defs::bslots::CLK_HROW);
        for lr in ['L', 'R'] {
            for i in 0..10 {
                for j in 0..32 {
                    bctx.build()
                        .tile_mutex(format!("IN_HCLK_{lr}{i}"), format!("GCLK{j}"))
                        .tile_mutex(format!("OUT_GCLK{j}"), format!("HCLK_{lr}{i}"))
                        .test_manual(format!("MUX.HCLK_{lr}{i}"), format!("GCLK{j}"))
                        .pip(format!("HCLK_{lr}{i}"), format!("GCLK{j}"))
                        .commit();
                }
            }
        }
    }
    for (tile, bel) in [
        ("CLK_IOB_S", defs::bslots::CLK_IOB),
        ("CLK_IOB_N", defs::bslots::CLK_IOB),
        ("CLK_CMT_S", defs::bslots::CLK_CMT),
        ("CLK_CMT_N", defs::bslots::CLK_CMT),
        ("CLK_MGT_S", defs::bslots::CLK_MGT),
        ("CLK_MGT_N", defs::bslots::CLK_MGT),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(bel);
        for i in 0..32 {
            let mux = format!("MUX.MUXBUS{i}");
            let mout = format!("MUXBUS_O{i}");
            let min = format!("MUXBUS_I{i}");
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() && bel != defs::bslots::CLK_IOB {
                    continue;
                }
                for j in 0..5 {
                    bctx.build()
                        .tile_mutex(&mux, format!("MGT_{lr}{j}"))
                        .tile_mutex(format!("MGT_{lr}{j}"), &mout)
                        .test_manual(&mux, format!("MGT_{lr}{j}"))
                        .pip(&mout, format!("MGT_{lr}{j}"))
                        .commit();
                }
            }
            if bel == defs::bslots::CLK_CMT {
                for j in 0..28 {
                    bctx.build()
                        .related_tile_mutex(HclkCmt, "ENABLE", "NOPE")
                        .tile_mutex(&mux, format!("CMT_CLK{j}"))
                        .test_manual(&mux, format!("CMT_CLK{j}"))
                        .pip(&mout, format!("CMT_CLK{j}"))
                        .commit();
                }
            }
            if bel == defs::bslots::CLK_IOB {
                for j in 0..10 {
                    bctx.build()
                        .tile_mutex(&mux, format!("GIOB{j}"))
                        .test_manual(&mux, format!("GIOB{j}"))
                        .pip(&mout, format!("PAD_BUF{j}"))
                        .commit();
                }
            }
            bctx.build()
                .tile_mutex(&mux, "PASS")
                .test_manual(&mux, "PASS")
                .pip(&mout, &min)
                .commit();
        }
        if bel == defs::bslots::CLK_IOB {
            for i in 0..10 {
                bctx.build()
                    .test_manual(format!("BUF.GIOB{i}"), "1")
                    .pip(format!("GIOB{i}"), format!("PAD_BUF{i}"))
                    .commit();
            }
        }
    }
    for tile in [
        "HCLK_IO",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_S",
        "HCLK_IO_CFG_N",
        "HCLK_IO_CMT_S",
        "HCLK_IO_CMT_N",
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let tcid = backend.edev.db.get_tile_class(tile);
        let tcls = &backend.edev.db[tcid];

        for i in 0..4 {
            let bel = defs::bslots::BUFIO[i];
            if !tcls.bels.contains_id(bel) {
                continue;
            }
            let mut bctx = ctx.bel(bel);
            bctx.test_manual("PRESENT", "1").mode("BUFIO").commit();
            bctx.mode("BUFIO")
                .tile_mutex("BUFIO", format!("TEST_BUFIO{i}"))
                .test_manual("ENABLE", "1")
                .pin("O")
                .commit();
        }
        for i in 0..2 {
            let bel = defs::bslots::BUFR[i];
            if !tcls.bels.contains_id(bel) {
                continue;
            }
            let mut bctx = ctx.bel(bel);

            bctx.test_manual("ENABLE", "1").mode("BUFR").commit();
            bctx.mode("BUFR").test_enum(
                "BUFR_DIVIDE",
                &["BYPASS", "1", "2", "3", "4", "5", "6", "7", "8"],
            );
            for pin in ["MGT0", "MGT1", "MGT2", "MGT3", "MGT4", "CKINT0", "CKINT1"] {
                bctx.mode("BUFR")
                    .mutex("MUX.I", pin)
                    .test_manual("MUX.I", pin)
                    .pip("I", (defs::bslots::RCLK, pin))
                    .commit();
            }
            for j in 0..4 {
                bctx.mode("BUFR")
                    .mutex("MUX.I", format!("BUFIO{j}"))
                    .test_manual("MUX.I", format!("BUFIO{j}"))
                    .pip("I", (PinFar, defs::bslots::BUFIO[j], "I"))
                    .commit();
            }
        }
        {
            let mut bctx = ctx.bel(defs::bslots::IOCLK);
            for i in 0..10 {
                bctx.test_manual(format!("BUF.HCLK{i}"), "1")
                    .pip(format!("HCLK_O{i}"), format!("HCLK_I{i}"))
                    .commit();
            }
            for i in 0..4 {
                bctx.build()
                    .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                    .related_pip(
                        Rclk,
                        (defs::bslots::IOCLK, format!("RCLK_I{i}")),
                        (defs::bslots::IOCLK, "VRCLK0"),
                    )
                    .test_manual(format!("BUF.RCLK{i}"), "1")
                    .pip(format!("RCLK_O{i}"), format!("RCLK_I{i}"))
                    .commit();
            }
            if tile == "HCLK_IO" {
                for i in 0..4 {
                    for inp in [
                        "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                    ] {
                        let mut extras: Vec<Box<DynProp>> = vec![];
                        for otile in [
                            "HCLK_IO_CENTER",
                            "HCLK_IO_CFG_S",
                            "HCLK_IO_CFG_N",
                            "HCLK_IO_CMT_S",
                            "HCLK_IO_CMT_N",
                        ] {
                            let otcls = backend.edev.db.get_tile_class(otile);
                            if !backend.edev.tile_index[otcls].is_empty() {
                                extras.push(Box::new(HclkIoiCenter(
                                    otile,
                                    "IOCLK",
                                    format!("ENABLE.RCLK{i}"),
                                    "1",
                                )));
                            }
                        }
                        bctx.build()
                            .tile_mutex("RCLK_MODE", "TEST")
                            .mutex(format!("MUX.RCLK{i}"), inp)
                            .props(extras)
                            .test_manual(format!("MUX.RCLK{i}"), inp)
                            .pip(format!("RCLK_I{i}"), inp)
                            .commit();
                    }
                }
            }
        }
        {
            let mut bctx = ctx.bel(defs::bslots::IDELAYCTRL);
            for i in 0..10 {
                bctx.build()
                    .mutex("MUX.REFCLK", format!("HCLK{i}"))
                    .test_manual("MUX.REFCLK", format!("HCLK{i}"))
                    .pip("REFCLK", (defs::bslots::IOCLK, format!("HCLK_O{i}")))
                    .commit();
            }
            bctx.build()
                .global("LEGIDELAY", "DISABLE")
                .unused()
                .prop(AllIodelay("DEFAULT", "DEFAULT_ONLY"))
                .test_manual("MODE", "DEFAULT_ONLY")
                .commit();
            bctx.build()
                .global("LEGIDELAY", "DISABLE")
                .prop(AllIodelay("FIXED", "FULL"))
                .test_manual("MODE", "FULL")
                .mode("IDELAYCTRL")
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK_CMT");
        let mut bctx = ctx.bel(defs::bslots::HCLK_CMT_HCLK);
        for i in 0..10 {
            bctx.build()
                .global_mutex("HCLK_CMT", "TEST")
                .test_manual(format!("BUF.HCLK{i}"), "1")
                .pip(format!("HCLK_O{i}"), format!("HCLK_I{i}"))
                .commit();
        }
        let mut bctx = ctx.bel(defs::bslots::HCLK_CMT_GIOB);
        for i in 0..10 {
            bctx.build()
                .global_mutex("HCLK_CMT", "TEST")
                .test_manual(format!("BUF.GIOB{i}"), "1")
                .pip(format!("GIOB_O{i}"), format!("GIOB_I{i}"))
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "HCLK_MGT_BUF") {
        let mut bctx = ctx.bel(defs::bslots::HCLK_MGT_BUF);
        for i in 0..5 {
            let mut extra = None;
            let cols_mgt_buf = &edev.chips[DieId::from_idx(0)].cols_mgt_buf;
            let num_l = cols_mgt_buf
                .iter()
                .copied()
                .filter(|&col| col < edev.col_clk)
                .count();
            let num_r = cols_mgt_buf
                .iter()
                .copied()
                .filter(|&col| col > edev.col_clk)
                .count();
            if num_l > 1 || num_r > 1 {
                extra = Some(HclkBramMgtPrev(format!("BUF.MGT{i}"), "1"));
            }
            bctx.build()
                // overzealous, but I don't care
                .global_mutex_here("HCLK_MGT")
                .maybe_prop(extra)
                .test_manual(format!("BUF.MGT{i}"), "1")
                .pip(format!("MGT_O{i}"), format!("MGT_I{i}"))
                .commit();
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
            ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                ctx.collect_inv(tile, bel, pin);
            }
            ctx.collect_enum_bool(tile, bel, "PRESELECT_I0", "FALSE", "TRUE");
            ctx.collect_enum_bool(tile, bel, "PRESELECT_I1", "FALSE", "TRUE");
            ctx.collect_enum_bool(tile, bel, "CREATE_EDGE", "FALSE", "TRUE");
            ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

            for attr in ["MUX.I0", "MUX.I1"] {
                ctx.collect_enum_default_ocd(
                    tile,
                    bel,
                    attr,
                    &[
                        "MUXBUS", "CKINT0", "CKINT1", "GFB0", "GFB1", "GFB2", "GFB3", "GFB4",
                        "GFB5", "GFB6", "GFB7", "GFB8", "GFB9", "GFB10", "GFB11", "GFB12", "GFB13",
                        "GFB14", "GFB15", "MGT_L0", "MGT_L1", "MGT_L2", "MGT_L3", "MGT_L4",
                        "MGT_R0", "MGT_R1", "MGT_R2", "MGT_R3", "MGT_R4",
                    ],
                    "NONE",
                    OcdMode::Mux,
                );
            }

            ctx.collect_bit(tile, bel, "I0_FABRIC_OUT", "1");
            ctx.collect_bit(tile, bel, "I1_FABRIC_OUT", "1");
        }

        for bel in ["BUFG_MGTCLK_S", "BUFG_MGTCLK_N"] {
            for i in 0..5 {
                for lr in ['L', 'R'] {
                    if lr == 'L' && edev.col_lgt.is_none() {
                        continue;
                    }
                    ctx.collect_bit(tile, bel, &format!("BUF.MGT_{lr}{i}"), "1");
                }
            }
        }
    }
    {
        let tile = "HCLK";
        let bel = "HCLK";
        for i in 0..10 {
            ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
        }
        for i in 0..4 {
            ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
        }
    }
    {
        let tile = "CLK_HROW";
        let bel = "CLK_HROW";
        let mut inp_diffs = vec![];
        for i in 0..32 {
            let diff_l = ctx
                .peek_diff(tile, bel, "MUX.HCLK_L0", format!("GCLK{i}"))
                .clone();
            let diff_r = ctx
                .peek_diff(tile, bel, "MUX.HCLK_R0", format!("GCLK{i}"))
                .clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for i in 0..10 {
            for lr in ['L', 'R'] {
                let mut inps = vec![("NONE".to_string(), Diff::default())];
                for j in 0..32 {
                    let mut diff =
                        ctx.get_diff(tile, bel, format!("MUX.HCLK_{lr}{i}"), format!("GCLK{j}"));
                    diff = diff.combine(&!&inp_diffs[j]);
                    inps.push((format!("GCLK{j}"), diff));
                }
                ctx.insert(
                    tile,
                    bel,
                    format!("MUX.HCLK_{lr}{i}"),
                    xlat_enum_ocd(inps, OcdMode::Mux),
                );
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.insert(tile, bel, format!("BUF.GCLK{i}"), xlat_bit(diff));
        }
    }
    for (tile, bel) in [
        ("CLK_IOB_S", "CLK_IOB"),
        ("CLK_IOB_N", "CLK_IOB"),
        ("CLK_CMT_S", "CLK_CMT"),
        ("CLK_CMT_N", "CLK_CMT"),
        ("CLK_MGT_S", "CLK_MGT"),
        ("CLK_MGT_N", "CLK_MGT"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        for i in 0..5 {
            for lr in ['L', 'R'] {
                if lr == 'L' && edev.col_lgt.is_none() && bel != "CLK_IOB" {
                    continue;
                }
                let diff_a = ctx
                    .peek_diff(tile, bel, "MUX.MUXBUS0", format!("MGT_{lr}{i}"))
                    .clone();
                let diff_b = ctx
                    .peek_diff(tile, bel, "MUX.MUXBUS1", format!("MGT_{lr}{i}"))
                    .clone();
                let (_, _, diff) = Diff::split(diff_a, diff_b);
                ctx.insert(tile, bel, format!("BUF.MGT_{lr}{i}"), xlat_bit(diff));
            }
        }
        for i in 0..32 {
            let mut diffs = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    "PASS".to_string(),
                    ctx.get_diff(tile, bel, format!("MUX.MUXBUS{i}"), "PASS"),
                ),
            ];
            for j in 0..5 {
                for lr in ['L', 'R'] {
                    if lr == 'L' && edev.col_lgt.is_none() && bel != "CLK_IOB" {
                        continue;
                    }
                    let mut diff =
                        ctx.get_diff(tile, bel, format!("MUX.MUXBUS{i}"), format!("MGT_{lr}{j}"));
                    diff.apply_bit_diff(
                        ctx.item(tile, bel, &format!("BUF.MGT_{lr}{j}")),
                        true,
                        false,
                    );
                    diffs.push((format!("MGT_{lr}{j}"), diff));
                }
            }
            if bel == "CLK_CMT" {
                for j in 0..28 {
                    diffs.push((
                        format!("CMT_CLK{j}"),
                        ctx.get_diff(tile, bel, format!("MUX.MUXBUS{i}"), format!("CMT_CLK{j}")),
                    ));
                }
            }
            if bel == "CLK_IOB" {
                for j in 0..10 {
                    diffs.push((
                        format!("GIOB{j}"),
                        ctx.get_diff(tile, bel, format!("MUX.MUXBUS{i}"), format!("GIOB{j}")),
                    ));
                }
            }
            ctx.insert(
                tile,
                bel,
                format!("MUX.MUXBUS{i}"),
                xlat_enum_ocd(diffs, OcdMode::Mux),
            );
        }
        if bel == "CLK_IOB" {
            for i in 0..10 {
                ctx.collect_bit_wide(tile, bel, &format!("BUF.GIOB{i}"), "1");
            }
        }
    }
    for tile in [
        "HCLK_IO",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_S",
        "HCLK_IO_CFG_N",
        "HCLK_IO_CMT_S",
        "HCLK_IO_CMT_N",
    ] {
        let tcid = edev.db.get_tile_class(tile);
        let tcls = &edev.db[tcid];

        if !ctx.has_tile(tile) {
            continue;
        }
        let mut diffs = vec![];
        for i in 0..4 {
            let bel = format!("BUFIO[{i}]");
            if !tcls.bels.contains_id(defs::bslots::BUFIO[i]) {
                continue;
            }
            ctx.get_diff(tile, &bel, "PRESENT", "1").assert_empty();
            let diff = ctx.get_diff(tile, &bel, "ENABLE", "1");
            diffs.push((bel, diff));
        }
        let (_, _, enable) = Diff::split(diffs[0].1.clone(), diffs[1].1.clone());
        for (bel, mut diff) in diffs {
            diff = diff.combine(&!&enable);
            ctx.insert(tile, bel, "ENABLE", xlat_bit(diff));
        }
        ctx.insert(tile, "IOCLK", "IOCLK_ENABLE", xlat_bit_wide(enable));

        if tile == "HCLK_IO" {
            for i in 0..2 {
                let bel = format!("BUFR[{i}]");
                let bel = &bel;
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
                        "CKINT0", "CKINT1", "BUFIO0", "BUFIO1", "BUFIO2", "BUFIO3", "MGT0", "MGT1",
                        "MGT2", "MGT3", "MGT4",
                    ],
                    "NONE",
                );
            }
            for i in 0..4 {
                let item = ctx.extract_enum_default_ocd(
                    tile,
                    "IOCLK",
                    &format!("MUX.RCLK{i}"),
                    &[
                        "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                    ],
                    "NONE",
                    OcdMode::Mux,
                );
                ctx.insert(tile, "RCLK", format!("MUX.RCLK{i}"), item);
            }
        } else {
            for i in 0..4 {
                ctx.collect_bit(tile, "IOCLK", &format!("ENABLE.RCLK{i}"), "1");
            }
        }
        {
            let bel = "IOCLK";
            for i in 0..10 {
                ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
            }
            for i in 0..4 {
                ctx.collect_bit(tile, bel, &format!("BUF.RCLK{i}"), "1");
            }
        }
        {
            let bel = "IDELAYCTRL";
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.REFCLK",
                &[
                    "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                    "HCLK8", "HCLK9",
                ],
                "NONE",
            );
            ctx.collect_enum_default(tile, bel, "MODE", &["FULL", "DEFAULT_ONLY"], "NONE");
        }
    }
    {
        let tile = "IO";
        let bel = "IODELAY_BOTH";
        // don't worry about it kitten
        ctx.get_diff(tile, bel, "IDELAYCTRL_MODE", "DEFAULT_ONLY");
        ctx.get_diff(tile, bel, "IDELAYCTRL_MODE", "FULL");
    }
    {
        let tile = "HCLK_CMT";
        let bel = "HCLK_CMT";
        for i in 0..10 {
            let item = ctx.extract_bit(tile, "HCLK_CMT_HCLK", &format!("BUF.HCLK{i}"), "1");
            ctx.insert(tile, bel, format!("BUF.HCLK{i}"), item);
            let item = ctx.extract_bit(tile, "HCLK_CMT_GIOB", &format!("BUF.GIOB{i}"), "1");
            ctx.insert(tile, bel, format!("BUF.GIOB{i}"), item);
        }
    }
    if ctx.has_tile("HCLK_MGT_BUF") {
        let tile = "HCLK_MGT_BUF";
        let bel = "HCLK_MGT_BUF";
        for i in 0..5 {
            ctx.collect_bit(tile, bel, &format!("BUF.MGT{i}"), "1");
        }
    }
}
