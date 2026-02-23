use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{PolTileWireCoord, TileClassId, TileWireCoord, WireSlotIdExt},
    grid::{DieId, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, extract_common_diff, xlat_bit, xlat_bit_wide, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    self, bcls, bslots, tslots,
    virtex5::{tcls, wires},
};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            DynProp,
            mutex::{WireMutexExclusive, WireMutexShared},
            relation::{Related, TileRelation},
        },
    },
    virtex4::specials,
};

#[derive(Copy, Clone, Debug)]
struct Rclk;

impl TileRelation for Rclk {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = if tcrd.col <= edev.col_clk {
            edev.col_io_w.unwrap()
        } else {
            edev.col_io_e.unwrap()
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
        Some(tcrd.with_row(row).tile(defs::tslots::HCLK_BEL))
    }
}

#[derive(Clone, Debug)]
struct HclkBramMgtPrev(TileWireCoord, PolTileWireCoord);

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
                key: DiffKey::Routing(tcls::HCLK_MGT_BUF, self.0, self.1),
                rects: edev.tile_bits(ntcrd),
            });
            sad = false;
        }
        Some((fuzzer, sad))
    }
}

#[derive(Clone, Debug)]
struct HclkIoiCenterSupport(TileClassId, TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for HclkIoiCenterSupport {
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
            && let ntcrd = tcrd.with_col(edev.col_clk).tile(tslots::HCLK_BEL)
            && let Some(ntile) = edev.get_tile(ntcrd)
            && ntile.class == self.0
        {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::RoutingSpecial(self.0, self.1, specials::SUPPORT),
                rects: edev.tile_bits(ntcrd),
            });
            sad = false;
        }
        Some((fuzzer, sad))
    }
}

#[derive(Clone, Debug)]
struct AllIodelay(&'static str, SpecialId);

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
            let Some(ntcrd) = edev.find_tile_by_bel(tcrd.with_row(row).bel(bslots::IODELAY[0]))
            else {
                continue;
            };
            for bel in [bslots::IODELAY[0], bslots::IODELAY[1]] {
                if let Some(site) = backend.ngrid.get_bel_name(ntcrd.bel(bel)) {
                    fuzzer = fuzzer.fuzz(Key::SiteMode(site), None, "IODELAY");
                    fuzzer = fuzzer.fuzz(Key::SiteAttr(site, "IDELAY_TYPE".into()), None, self.0);
                }
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecial(tcls::IO, bslots::IODELAY[0], self.1),
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
        let mut ctx = FuzzCtx::new(session, backend, tcls::CLK_BUFG);
        for i in 0..32 {
            let mut bctx = ctx.bel(bslots::BUFGCTRL[i]);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("BUFGCTRL")
                .commit();
            for pin in [
                bcls::BUFGCTRL::CE0,
                bcls::BUFGCTRL::CE1,
                bcls::BUFGCTRL::S0,
                bcls::BUFGCTRL::S1,
                bcls::BUFGCTRL::IGNORE0,
                bcls::BUFGCTRL::IGNORE1,
            ] {
                bctx.mode("BUFGCTRL").test_bel_input_inv_auto(pin);
            }
            bctx.mode("BUFGCTRL").test_bel_attr_bool_auto(
                bcls::BUFGCTRL::PRESELECT_I0,
                "FALSE",
                "TRUE",
            );
            bctx.mode("BUFGCTRL").test_bel_attr_bool_auto(
                bcls::BUFGCTRL::PRESELECT_I1,
                "FALSE",
                "TRUE",
            );
            bctx.mode("BUFGCTRL").test_bel_attr_bool_auto(
                bcls::BUFGCTRL::CREATE_EDGE,
                "FALSE",
                "TRUE",
            );
            bctx.mode("BUFGCTRL")
                .test_bel_attr_bool_auto(bcls::BUFGCTRL::INIT_OUT, "0", "1");
        }
        for (ct, co, cf) in [(0, 0, 0), (0, 5, 20), (10, 0, 10), (10, 5, 21)] {
            if co == 0 && edev.col_gt_w.is_none() {
                continue;
            }
            for i in 0..5 {
                let wt = wires::MGT_BUF[co + i].cell(ct);
                let wf = wires::MGT_ROW_I[i].cell(cf);
                ctx.build()
                    .test_routing(wt, wf.pos())
                    .prop(FuzzIntPip::new(wt, wf))
                    .commit();
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::HCLK);
        for i in 0..4 {
            let wt = wires::RCLK[i].cell(0);
            let wf = wires::RCLK_ROW[i].cell(0);
            ctx.build()
                .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                .prop(Related::new(
                    Rclk,
                    BaseIntPip::new(wires::RCLK_ROW[i].cell(2), wires::VRCLK[0].cell(2)),
                ))
                .test_routing(wt, wf.pos())
                .prop(FuzzIntPip::new(wt, wf))
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CLK_HROW);
        for co in 0..2 {
            let lr = ['L', 'R'][co];
            for i in 0..10 {
                let wt = wires::HCLK_ROW[i].cell(co);
                for j in 0..32 {
                    let wf = wires::GCLK[j].cell(0);
                    ctx.build()
                        .tile_mutex(format!("IN_HCLK_{lr}{i}"), format!("GCLK{j}"))
                        .tile_mutex(format!("OUT_GCLK{j}"), format!("HCLK_{lr}{i}"))
                        .test_routing(wt, wf.pos())
                        .prop(FuzzIntPip::new(wt, wf))
                        .commit();
                }
            }
        }
    }
    for tcid in [
        tcls::CLK_IOB_S,
        tcls::CLK_IOB_N,
        tcls::CLK_CMT_S,
        tcls::CLK_CMT_N,
        tcls::CLK_MGT_S,
        tcls::CLK_MGT_N,
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::CLK_INT);
        for i in 0..32 {
            let dst = wires::IMUX_BUFG_O[i].cell(0);

            for &src in edev.db_index.tile_classes[tcid].muxes[&dst].src.keys() {
                let mut builder = bctx.build().prop(WireMutexExclusive::new(dst));
                if let Some(idx) = wires::MGT_BUF.index_of(src.wire) {
                    if idx < 5
                        && edev.col_gt_w.is_none()
                        && !matches!(tcid, tcls::CLK_IOB_S | tcls::CLK_IOB_N)
                    {
                        continue;
                    }
                    let fake_src = edev.db_index.tile_classes[tcid].pips_bwd[&src.tw]
                        .iter()
                        .next()
                        .copied()
                        .unwrap();
                    builder = builder
                        .prop(WireMutexExclusive::new(src.tw))
                        .prop(FuzzIntPip::new(dst, fake_src.tw));
                } else {
                    builder = builder
                        .prop(WireMutexShared::new(src.tw))
                        .prop(FuzzIntPip::new(dst, src.tw));
                }
                if wires::OUT_CMT.contains(src.wire) {
                    builder = builder.related_tile_mutex(HclkCmt, "ENABLE", "NOPE");
                }

                builder.test_routing(dst, src).commit();
            }
        }
    }
    for tcid in [
        tcls::HCLK_IO,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_S,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_CMT_S,
        tcls::HCLK_IO_CMT_N,
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let tcls = &backend.edev.db[tcid];

        for i in 0..4 {
            let bel = bslots::BUFIO[i];
            if !tcls.bels.contains_id(bel) {
                continue;
            }
            let mut bctx = ctx.bel(bel);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("BUFIO")
                .commit();
            bctx.mode("BUFIO")
                .tile_mutex("BUFIO", format!("TEST_BUFIO{i}"))
                .test_bel_attr_bits(bcls::BUFIO::ENABLE)
                .pin("O")
                .commit();
        }
        for i in 0..2 {
            let bel = bslots::BUFR[i];
            if !tcls.bels.contains_id(bel) {
                continue;
            }
            let mut bctx = ctx.bel(bel);

            bctx.build()
                .test_bel_attr_bits(bcls::BUFR::ENABLE)
                .mode("BUFR")
                .commit();
            bctx.mode("BUFR")
                .test_bel_attr_rename("BUFR_DIVIDE", bcls::BUFR::DIVIDE);
        }
        for i in 0..4 {
            let wt = wires::RCLK_IO[i].cell(2);
            let wf = wires::RCLK_ROW[i].cell(2);
            ctx.build()
                .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                .prop(Related::new(
                    Rclk,
                    BaseIntPip::new(wires::RCLK_ROW[i].cell(2), wires::VRCLK[0].cell(2)),
                ))
                .test_routing(wt, wf.pos())
                .prop(FuzzIntPip::new(wt, wf))
                .commit();
        }
        if tcid == tcls::HCLK_IO {
            for i in 0..4 {
                let dst = wires::RCLK_ROW[i].cell(2);
                for src in [
                    wires::VRCLK[0].cell(2),
                    wires::VRCLK[1].cell(2),
                    wires::VRCLK_S[0].cell(2),
                    wires::VRCLK_S[1].cell(2),
                    wires::VRCLK_N[0].cell(2),
                    wires::VRCLK_N[1].cell(2),
                ] {
                    let mut extras: Vec<Box<DynProp>> = vec![];
                    for otcls in [
                        tcls::HCLK_IO_CENTER,
                        tcls::HCLK_IO_CFG_S,
                        tcls::HCLK_IO_CFG_N,
                        tcls::HCLK_IO_CMT_S,
                        tcls::HCLK_IO_CMT_N,
                    ] {
                        if !backend.edev.tile_index[otcls].is_empty() {
                            extras.push(Box::new(HclkIoiCenterSupport(otcls, dst)));
                        }
                    }
                    ctx.build()
                        .tile_mutex("RCLK_MODE", "TEST")
                        .prop(WireMutexExclusive::new(dst))
                        .props(extras)
                        .test_routing(dst, src.pos())
                        .prop(FuzzIntPip::new(dst, src))
                        .commit();
                }
            }
        }
        {
            let mut bctx = ctx.bel(bslots::IDELAYCTRL);
            bctx.build()
                .global("LEGIDELAY", "DISABLE")
                .unused()
                .prop(AllIodelay(
                    "DEFAULT",
                    specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY,
                ))
                .test_bel_special(specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY)
                .commit();
            bctx.build()
                .global("LEGIDELAY", "DISABLE")
                .prop(AllIodelay("FIXED", specials::IDELAYCTRL_IODELAY_FULL))
                .test_bel_special(specials::IDELAYCTRL_IODELAY_FULL)
                .mode("IDELAYCTRL")
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::HCLK_MGT_BUF) {
        let mut bctx = ctx.bel(bslots::CLK_INT);
        for i in 0..5 {
            let dst = wires::MGT_ROW_O[i].cell(0);
            let src = wires::MGT_ROW_I[i].cell(0).pos();
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
                extra = Some(HclkBramMgtPrev(dst, src));
            }
            bctx.build()
                // overzealous, but I don't care
                .global_mutex_here("HCLK_MGT")
                .maybe_prop(extra)
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    {
        let tcid = tcls::CLK_BUFG;
        for bslot in bslots::BUFGCTRL {
            for pin in [
                bcls::BUFGCTRL::CE0,
                bcls::BUFGCTRL::CE1,
                bcls::BUFGCTRL::S0,
                bcls::BUFGCTRL::S1,
                bcls::BUFGCTRL::IGNORE0,
                bcls::BUFGCTRL::IGNORE1,
            ] {
                ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
            }
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGCTRL::PRESELECT_I0);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGCTRL::PRESELECT_I1);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGCTRL::CREATE_EDGE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGCTRL::INIT_OUT);
        }

        for (ct, co, cf) in [(0, 0, 0), (0, 5, 20), (10, 0, 10), (10, 5, 21)] {
            if co == 0 && edev.col_gt_w.is_none() {
                continue;
            }
            for i in 0..5 {
                let wt = wires::MGT_BUF[co + i].cell(ct);
                let wf = wires::MGT_ROW_I[i].cell(cf);
                ctx.collect_progbuf(tcid, wt, wf.pos());
            }
        }
    }
    {
        let tcid = tcls::HCLK;
        for i in 0..4 {
            ctx.collect_progbuf(
                tcid,
                wires::RCLK[i].cell(0),
                wires::RCLK_ROW[i].cell(0).pos(),
            );
        }
    }
    {
        let tcid = tcls::CLK_HROW;
        let mut inp_diffs = vec![];
        for i in 0..32 {
            let diff_l = ctx
                .peek_diff_routing(
                    tcid,
                    wires::HCLK_ROW[0].cell(0),
                    wires::GCLK[i].cell(0).pos(),
                )
                .clone();
            let diff_r = ctx
                .peek_diff_routing(
                    tcid,
                    wires::HCLK_ROW[0].cell(1),
                    wires::GCLK[i].cell(0).pos(),
                )
                .clone();
            let (_, _, diff) = Diff::split(diff_l, diff_r);
            inp_diffs.push(diff);
        }
        for co in 0..2 {
            for i in 0..10 {
                let dst = wires::HCLK_ROW[i].cell(co);
                let mut diffs = vec![(None, Diff::default())];
                for j in 0..32 {
                    let fake_src = wires::GCLK[j].cell(0).pos();
                    let src = wires::GCLK_BUF[j].cell(0).pos();
                    let mut diff = ctx.get_diff_routing(tcid, dst, fake_src);
                    diff = diff.combine(&!&inp_diffs[j]);
                    diffs.push((Some(src), diff));
                }
                ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
            }
        }

        for (i, diff) in inp_diffs.into_iter().enumerate() {
            let dst = wires::GCLK_BUF[i].cell(0);
            let src = wires::GCLK[i].cell(0).pos();

            ctx.insert_progbuf(tcid, dst, src, xlat_bit(diff));
        }
    }
    for tcid in [
        tcls::CLK_IOB_S,
        tcls::CLK_IOB_N,
        tcls::CLK_CMT_S,
        tcls::CLK_CMT_N,
        tcls::CLK_MGT_S,
        tcls::CLK_MGT_N,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for i in 0..10 {
            if i < 5
                && edev.col_gt_w.is_none()
                && !matches!(tcid, tcls::CLK_IOB_S | tcls::CLK_IOB_N)
            {
                continue;
            }
            let dst = wires::MGT_BUF[i].cell(0);
            let src = if i < 5 {
                wires::MGT_ROW_I[i].cell(0)
            } else {
                wires::MGT_ROW_I[i - 5].cell(10)
            };
            let diff_a = ctx
                .peek_diff_routing(tcid, wires::IMUX_BUFG_O[0].cell(0), dst.pos())
                .clone();
            let diff_b = ctx
                .peek_diff_routing(tcid, wires::IMUX_BUFG_O[1].cell(0), dst.pos())
                .clone();
            let (_, _, diff) = Diff::split(diff_a, diff_b);
            ctx.insert_progbuf(tcid, dst, src.pos(), xlat_bit(diff));
        }
        for i in 0..32 {
            let dst = wires::IMUX_BUFG_O[i].cell(0);

            let mut diffs = vec![(None, Diff::default())];
            for &src in edev.db_index.tile_classes[tcid].muxes[&dst].src.keys() {
                if let Some(idx) = wires::MGT_BUF.index_of(src.wire) {
                    let fake_src = if idx < 5 {
                        wires::MGT_ROW_I[idx].cell(0)
                    } else {
                        wires::MGT_ROW_I[idx - 5].cell(10)
                    };
                    if idx < 5
                        && edev.col_gt_w.is_none()
                        && !matches!(tcid, tcls::CLK_IOB_S | tcls::CLK_IOB_N)
                    {
                        continue;
                    }
                    let mut diff = ctx.get_diff_routing(tcid, dst, src);
                    diff.apply_bit_diff(ctx.sb_progbuf(tcid, src.tw, fake_src.pos()), true, false);
                    diffs.push((Some(src), diff));
                } else {
                    let diff = ctx.get_diff_routing(tcid, dst, src);
                    diffs.push((Some(src), diff));
                }
            }
            ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
        }
    }
    for tcid in [
        tcls::HCLK_IO,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_S,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_CMT_S,
        tcls::HCLK_IO_CMT_N,
    ] {
        let tcls = &edev.db[tcid];

        if !ctx.has_tcls(tcid) {
            continue;
        }
        let mut diffs = vec![];
        for i in 0..4 {
            let bslot = bslots::BUFIO[i];
            if !tcls.bels.contains_id(bslot) {
                continue;
            }
            let diff = ctx.get_diff_attr_bool(tcid, bslot, bcls::BUFIO::ENABLE);
            diffs.push((bslot, diff));
        }
        let enable = extract_common_diff(&mut diffs);
        for (bslot, diff) in diffs {
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::BUFIO::ENABLE, xlat_bit(diff));
        }
        ctx.insert_support(
            tcid,
            wires::IOCLK.into_iter().map(|w| w.cell(2)).collect(),
            xlat_bit_wide(enable),
        );

        if tcid == tcls::HCLK_IO {
            for i in 0..2 {
                let bslot = bslots::BUFR[i];
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFR::ENABLE);
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFR::DIVIDE);
            }
            for i in 0..4 {
                ctx.collect_mux(tcid, wires::RCLK_ROW[i].cell(2));
            }
        } else {
            for i in 0..4 {
                let wire = wires::RCLK_ROW[i].cell(2);
                let bit = xlat_bit(ctx.get_diff_routing_special(tcid, wire, specials::SUPPORT));
                ctx.insert_support(tcid, BTreeSet::from_iter([wire]), vec![bit]);
            }
        }
        for i in 0..4 {
            ctx.collect_progbuf(
                tcid,
                wires::RCLK_IO[i].cell(2),
                wires::RCLK_ROW[i].cell(2).pos(),
            );
        }
        {
            let bslot = bslots::IDELAYCTRL;
            let vctl_sel = vec![TileBit::new(0, 36, 13).pos(), TileBit::new(0, 36, 14).pos()];
            let mut diff_full =
                ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_FULL);
            let mut diff_default =
                ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY);
            diff_full.apply_bitvec_diff_int(&vctl_sel, 1, 0);
            diff_default.apply_bitvec_diff_int(&vctl_sel, 3, 0);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::IDELAYCTRL::DLL_ENABLE,
                xlat_bit(diff_full.combine(&!&diff_default)),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::IDELAYCTRL::DELAY_ENABLE,
                xlat_bit(diff_default),
            );
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IDELAYCTRL::VCTL_SEL, vctl_sel);
        }
    }
    {
        let tcid = tcls::IO;
        let bslot = bslots::IODELAY[0];
        // don't worry about it kitten
        ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY);
        ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_FULL);
    }
    if ctx.has_tcls(tcls::HCLK_MGT_BUF) {
        let tcid = tcls::HCLK_MGT_BUF;
        for i in 0..5 {
            ctx.collect_progbuf(
                tcid,
                wires::MGT_ROW_O[i].cell(0),
                wires::MGT_ROW_I[i].cell(0).pos(),
            );
        }
    }
}
