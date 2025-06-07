use prjcombine_interconnect::grid::{DieId, NodeLoc};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_bit_wide, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_virtex4::bels;
use unnamed_entity::EntityId;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{DynProp, pip::PinFar, relation::NodeRelation},
    },
};

#[derive(Copy, Clone, Debug)]
struct Rclk;

impl NodeRelation for Rclk {
    fn resolve(&self, backend: &IseBackend, nloc: NodeLoc) -> Option<NodeLoc> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = if nloc.1 <= edev.col_clk {
            edev.col_lio.unwrap()
        } else {
            edev.col_rio.unwrap()
        };
        Some(
            edev.egrid
                .get_tile_by_bel((nloc.0, (col, nloc.2), bels::IOCLK)),
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct Hclk(&'static str);

impl NodeRelation for Hclk {
    fn resolve(&self, backend: &IseBackend, nloc: NodeLoc) -> Option<NodeLoc> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let row = edev.chips[nloc.0].row_hclk(nloc.2);
        edev.egrid
            .find_tile_by_class(nloc.0, (nloc.1, row), |kind| kind == self.0)
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
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[nloc.0];
        let col = if nloc.1 < edev.col_clk {
            let mut range = chip.cols_mgt_buf.range(..nloc.1);
            range.next_back()
        } else {
            let mut range = chip.cols_mgt_buf.range((nloc.1 + 1)..);
            range.next()
        };
        let mut sad = true;
        if let Some(&col) = col {
            let nnloc = edev
                .egrid
                .get_tile_by_bel((nloc.0, (col, nloc.2), bels::HCLK_BRAM_MGT));
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: "HCLK_BRAM_MGT".into(),
                    bel: "HCLK_BRAM_MGT".into(),
                    attr: self.0.clone(),
                    val: self.1.into(),
                },
                tiles: edev.tile_bits(nnloc),
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
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let mut sad = true;
        if nloc.1 <= edev.col_clk {
            if let Some(nnloc) =
                backend
                    .egrid
                    .find_tile_by_class(nloc.0, (edev.col_clk, nloc.2), |kind| kind == self.0)
            {
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: self.0.into(),
                        bel: self.1.into(),
                        attr: self.2.clone(),
                        val: self.3.into(),
                    },
                    tiles: edev.tile_bits(nnloc),
                });
                sad = false;
            }
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
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[nloc.0];
        let bot = chip.row_reg_bot(chip.row_to_reg(nloc.2));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            let Some(nnloc) = edev
                .egrid
                .find_tile_by_bel((nloc.0, (nloc.1, row), bels::IODELAY0))
            else {
                continue;
            };
            for bel in [bels::IODELAY0, bels::IODELAY1] {
                if let Some(site) = backend.ngrid.get_bel_name((nloc.0, (nloc.1, row), bel)) {
                    fuzzer = fuzzer.fuzz(Key::SiteMode(site), None, "IODELAY");
                    fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "IDELAY_TYPE".into()), None, self.0);
                }
            }
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: "IO".into(),
                    bel: "IODELAY_BOTH".into(),
                    attr: "IDELAYCTRL_MODE".into(),
                    val: self.1.into(),
                },
                tiles: edev.tile_bits(nnloc),
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
        let mut ctx = FuzzCtx::new(session, backend, "HCLK");
        let mut bctx = ctx.bel(bels::HCLK);
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
                    (bels::IOCLK, format!("RCLK_I{i}")),
                    (bels::IOCLK, "VRCLK0"),
                )
                .test_manual(format!("BUF.RCLK{i}"), "1")
                .pip(format!("RCLK_O{i}"), format!("RCLK_I{i}"))
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "CLK_HROW");
        let mut bctx = ctx.bel(bels::CLK_HROW);
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
        ("CLK_IOB_B", bels::CLK_IOB),
        ("CLK_IOB_T", bels::CLK_IOB),
        ("CLK_CMT_B", bels::CLK_CMT),
        ("CLK_CMT_T", bels::CLK_CMT),
        ("CLK_MGT_B", bels::CLK_MGT),
        ("CLK_MGT_T", bels::CLK_MGT),
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
                if lr == 'L' && edev.col_lgt.is_none() && bel != bels::CLK_IOB {
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
            if bel == bels::CLK_CMT {
                for j in 0..28 {
                    bctx.build()
                        .related_tile_mutex(Hclk("HCLK_CMT"), "ENABLE", "NOPE")
                        .tile_mutex(&mux, format!("CMT_CLK{j}"))
                        .test_manual(&mux, format!("CMT_CLK{j}"))
                        .pip(&mout, format!("CMT_CLK{j}"))
                        .commit();
                }
            }
            if bel == bels::CLK_IOB {
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
        if bel == bels::CLK_IOB {
            for i in 0..10 {
                bctx.build()
                    .test_manual(format!("BUF.GIOB{i}"), "1")
                    .pip(format!("GIOB{i}"), format!("PAD_BUF{i}"))
                    .commit();
            }
        }
    }
    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_BOTCEN",
        "HCLK_IOI_CMT",
        "HCLK_CMT_IOI",
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let node_kind = backend.egrid.db.get_tile_class(tile);
        let node_data = &backend.egrid.db.tile_classes[node_kind];

        for i in 0..4 {
            let bel = bels::BUFIO[i];
            if !node_data.bels.contains_id(bel) {
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
            let bel = bels::BUFR[i];
            if !node_data.bels.contains_id(bel) {
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
                    .pip("I", (bels::RCLK, pin))
                    .commit();
            }
            for j in 0..4 {
                bctx.mode("BUFR")
                    .mutex("MUX.I", format!("BUFIO{j}"))
                    .test_manual("MUX.I", format!("BUFIO{j}"))
                    .pip("I", (PinFar, bels::BUFIO[j], "I"))
                    .commit();
            }
        }
        {
            let mut bctx = ctx.bel(bels::IOCLK);
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
                        (bels::IOCLK, format!("RCLK_I{i}")),
                        (bels::IOCLK, "VRCLK0"),
                    )
                    .test_manual(format!("BUF.RCLK{i}"), "1")
                    .pip(format!("RCLK_O{i}"), format!("RCLK_I{i}"))
                    .commit();
            }
            if tile == "HCLK_IOI" {
                for i in 0..4 {
                    for inp in [
                        "VRCLK0", "VRCLK1", "VRCLK_S0", "VRCLK_S1", "VRCLK_N0", "VRCLK_N1",
                    ] {
                        let mut extras: Vec<Box<DynProp>> = vec![];
                        for otile in [
                            "HCLK_IOI_CENTER",
                            "HCLK_IOI_TOPCEN",
                            "HCLK_IOI_BOTCEN",
                            "HCLK_IOI_CMT",
                            "HCLK_CMT_IOI",
                        ] {
                            let onode = backend.egrid.db.get_tile_class(otile);
                            if !backend.egrid.tile_index[onode].is_empty() {
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
            let mut bctx = ctx.bel(bels::IDELAYCTRL);
            for i in 0..10 {
                bctx.build()
                    .mutex("MUX.REFCLK", format!("HCLK{i}"))
                    .test_manual("MUX.REFCLK", format!("HCLK{i}"))
                    .pip("REFCLK", (bels::IOCLK, format!("HCLK_O{i}")))
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
        let mut bctx = ctx.bel(bels::HCLK_CMT_HCLK);
        for i in 0..10 {
            bctx.build()
                .global_mutex("HCLK_CMT", "TEST")
                .test_manual(format!("BUF.HCLK{i}"), "1")
                .pip(format!("HCLK_O{i}"), format!("HCLK_I{i}"))
                .commit();
        }
        let mut bctx = ctx.bel(bels::HCLK_CMT_GIOB);
        for i in 0..10 {
            bctx.build()
                .global_mutex("HCLK_CMT", "TEST")
                .test_manual(format!("BUF.GIOB{i}"), "1")
                .pip(format!("GIOB_O{i}"), format!("GIOB_I{i}"))
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "HCLK_BRAM_MGT") {
        let mut bctx = ctx.bel(bels::HCLK_BRAM_MGT);
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
                .state
                .peek_diff(tile, bel, "MUX.HCLK_L0", format!("GCLK{i}"))
                .clone();
            let diff_r = ctx
                .state
                .peek_diff(tile, bel, "MUX.HCLK_R0", format!("GCLK{i}"))
                .clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for i in 0..10 {
            for lr in ['L', 'R'] {
                let mut inps = vec![("NONE".to_string(), Diff::default())];
                for j in 0..32 {
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.HCLK_{lr}{i}"),
                        format!("GCLK{j}"),
                    );
                    diff = diff.combine(&!&inp_diffs[j]);
                    inps.push((format!("GCLK{j}"), diff));
                }
                ctx.tiledb.insert(
                    tile,
                    bel,
                    format!("MUX.HCLK_{lr}{i}"),
                    xlat_enum_ocd(inps, OcdMode::Mux),
                );
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.tiledb
                .insert(tile, bel, format!("BUF.GCLK{i}"), xlat_bit(diff));
        }
    }
    for (tile, bel) in [
        ("CLK_IOB_B", "CLK_IOB"),
        ("CLK_IOB_T", "CLK_IOB"),
        ("CLK_CMT_B", "CLK_CMT"),
        ("CLK_CMT_T", "CLK_CMT"),
        ("CLK_MGT_B", "CLK_MGT"),
        ("CLK_MGT_T", "CLK_MGT"),
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
                    .state
                    .peek_diff(tile, bel, "MUX.MUXBUS0", format!("MGT_{lr}{i}"))
                    .clone();
                let diff_b = ctx
                    .state
                    .peek_diff(tile, bel, "MUX.MUXBUS1", format!("MGT_{lr}{i}"))
                    .clone();
                let (_, _, diff) = Diff::split(diff_a, diff_b);
                ctx.tiledb
                    .insert(tile, bel, format!("BUF.MGT_{lr}{i}"), xlat_bit(diff));
            }
        }
        for i in 0..32 {
            let mut diffs = vec![
                ("NONE".to_string(), Diff::default()),
                (
                    "PASS".to_string(),
                    ctx.state
                        .get_diff(tile, bel, format!("MUX.MUXBUS{i}"), "PASS"),
                ),
            ];
            for j in 0..5 {
                for lr in ['L', 'R'] {
                    if lr == 'L' && edev.col_lgt.is_none() && bel != "CLK_IOB" {
                        continue;
                    }
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        format!("MUX.MUXBUS{i}"),
                        format!("MGT_{lr}{j}"),
                    );
                    diff.apply_bit_diff(
                        ctx.tiledb.item(tile, bel, &format!("BUF.MGT_{lr}{j}")),
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
                        ctx.state.get_diff(
                            tile,
                            bel,
                            format!("MUX.MUXBUS{i}"),
                            format!("CMT_CLK{j}"),
                        ),
                    ));
                }
            }
            if bel == "CLK_IOB" {
                for j in 0..10 {
                    diffs.push((
                        format!("GIOB{j}"),
                        ctx.state
                            .get_diff(tile, bel, format!("MUX.MUXBUS{i}"), format!("GIOB{j}")),
                    ));
                }
            }
            ctx.tiledb.insert(
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
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_BOTCEN",
        "HCLK_IOI_CMT",
        "HCLK_CMT_IOI",
    ] {
        let node_kind = edev.egrid.db.get_tile_class(tile);
        let node_data = &edev.egrid.db.tile_classes[node_kind];

        if !ctx.has_tile(tile) {
            continue;
        }
        let mut diffs = vec![];
        for i in 0..4 {
            let bel = format!("BUFIO{i}");
            if !node_data.bels.contains_id(bels::BUFIO[i]) {
                continue;
            }
            ctx.state
                .get_diff(tile, &bel, "PRESENT", "1")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, &bel, "ENABLE", "1");
            diffs.push((bel, diff));
        }
        let (_, _, enable) = Diff::split(diffs[0].1.clone(), diffs[1].1.clone());
        for (bel, mut diff) in diffs {
            diff = diff.combine(&!&enable);
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
        }
        ctx.tiledb
            .insert(tile, "IOCLK", "IOCLK_ENABLE", xlat_bit_wide(enable));

        if tile == "HCLK_IOI" {
            for i in 0..2 {
                let bel = format!("BUFR{i}");
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
                ctx.tiledb
                    .insert(tile, "RCLK", format!("MUX.RCLK{i}"), item);
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
        ctx.state
            .get_diff(tile, bel, "IDELAYCTRL_MODE", "DEFAULT_ONLY");
        ctx.state.get_diff(tile, bel, "IDELAYCTRL_MODE", "FULL");
    }
    {
        let tile = "HCLK_CMT";
        let bel = "HCLK_CMT";
        for i in 0..10 {
            let item = ctx.extract_bit(tile, "HCLK_CMT_HCLK", &format!("BUF.HCLK{i}"), "1");
            ctx.tiledb.insert(tile, bel, format!("BUF.HCLK{i}"), item);
            let item = ctx.extract_bit(tile, "HCLK_CMT_GIOB", &format!("BUF.GIOB{i}"), "1");
            ctx.tiledb.insert(tile, bel, format!("BUF.GIOB{i}"), item);
        }
    }
    if ctx.has_tile("HCLK_BRAM_MGT") {
        let tile = "HCLK_BRAM_MGT";
        let bel = "HCLK_BRAM_MGT";
        for i in 0..5 {
            ctx.collect_bit(tile, bel, &format!("BUF.MGT{i}"), "1");
        }
    }
}
