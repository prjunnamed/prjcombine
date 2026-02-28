use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, WireSlotIdExt},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit, xlat_enum_raw};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    self, bcls, bslots,
    virtex6::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip, FuzzIntfDelay},
        props::{
            mutex::{WireMutexExclusive, WireMutexShared},
            pip::PinFar,
            relation::{Delta, Related, TileRelation},
        },
    },
    virtex4::specials,
};

#[derive(Clone, Debug)]
struct Cmt;

impl TileRelation for Cmt {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(tcrd.with_col(edev.col_clk).tile(defs::tslots::BEL))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::HCLK);
        for co in 0..2 {
            for o in 0..8 {
                let dst = wires::LCLK[o].cell(co);
                for i in 0..12 {
                    let src = wires::HCLK_ROW[i].cell(1);
                    ctx.build()
                        .global_mutex("HCLK", "USE")
                        .row_mutex("BUFH_TEST", format!("USED_HCLK{i}"))
                        .prop(Related::new(
                            Cmt,
                            BaseIntPip::new(
                                wires::BUFH_TEST_W_IN.cell(20),
                                wires::HCLK_CMT_W[i].cell(20),
                            ),
                        ))
                        .prop(Related::new(
                            Cmt,
                            BaseIntPip::new(
                                wires::BUFH_TEST_E_IN.cell(20),
                                wires::HCLK_CMT_E[i].cell(20),
                            ),
                        ))
                        .prop(WireMutexExclusive::new(dst))
                        .tile_mutex(format!("HCLK{i}"), dst)
                        .test_routing(dst, src.pos())
                        .prop(FuzzIntPip::new(dst, src))
                        .commit();
                }
                for i in 0..6 {
                    let src = wires::RCLK_ROW[i].cell(1);
                    ctx.build()
                        .global_mutex("RCLK", "USE")
                        .row_mutex("BUFH_TEST", format!("USED_RCLK{i}"))
                        .prop(Related::new(
                            Cmt,
                            BaseIntPip::new(
                                wires::BUFH_TEST_W_IN.cell(20),
                                wires::RCLK_CMT_W[i].cell(20),
                            ),
                        ))
                        .prop(Related::new(
                            Cmt,
                            BaseIntPip::new(
                                wires::BUFH_TEST_E_IN.cell(20),
                                wires::RCLK_CMT_E[i].cell(20),
                            ),
                        ))
                        .prop(WireMutexExclusive::new(dst))
                        .tile_mutex(format!("RCLK{i}"), dst)
                        .test_routing(dst, src.pos())
                        .prop(FuzzIntPip::new(dst, src))
                        .commit();
                }
            }
        }
    }

    for (tcid, base, dy, c) in [(tcls::CLK_BUFG_S, 0, 2, 1), (tcls::CLK_BUFG_N, 16, 0, 0)] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..16 {
            let mut bctx = ctx.bel(bslots::BUFGCTRL[base + i]);
            let mode = "BUFGCTRL";
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode(mode)
                .commit();
            for pin in [
                bcls::BUFGCTRL::CE0,
                bcls::BUFGCTRL::CE1,
                bcls::BUFGCTRL::S0,
                bcls::BUFGCTRL::S1,
                bcls::BUFGCTRL::IGNORE0,
                bcls::BUFGCTRL::IGNORE1,
            ] {
                bctx.mode(mode).test_bel_input_inv_auto(pin);
            }
            bctx.mode(mode)
                .test_bel_attr_bool_auto(bcls::BUFGCTRL::PRESELECT_I0, "FALSE", "TRUE");
            bctx.mode(mode)
                .test_bel_attr_bool_auto(bcls::BUFGCTRL::PRESELECT_I1, "FALSE", "TRUE");
            bctx.mode(mode)
                .test_bel_attr_bool_auto(bcls::BUFGCTRL::CREATE_EDGE, "FALSE", "TRUE");
            bctx.mode(mode)
                .test_bel_attr_bool_auto(bcls::BUFGCTRL::INIT_OUT, "0", "1");
            // test buffers not fuzzed: ISE bug causes pips to be reversed? ugh.
        }
        for i in 0..16 {
            let dst = wires::OUT_BUFG_GFB[i].cell(c);
            let src = wires::OUT_BUFG[i].cell(c);
            ctx.build()
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexShared::new(src))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
            let dst = wires::GCLK[base + i].cell(0);
            ctx.build()
                .null_bits()
                .extra_tile_routing(
                    Delta::new(0, dy - 20, tcls::CMT),
                    wires::GCLK_CMT[base + i].cell(20),
                    wires::GCLK[base + i].cell(20).pos(),
                )
                .extra_tile_routing(
                    Delta::new(0, dy + 20, tcls::CMT),
                    wires::GCLK_CMT[base + i].cell(20),
                    wires::GCLK[base + i].cell(20).pos(),
                )
                .global_mutex("GCLK", "TEST")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexShared::new(src))
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
        for i in 0..32 {
            let dst = wires::IMUX_BUFG_O[i].cell(c);
            let mux = &backend.edev.db_index[tcid].muxes[&dst];
            for &src in mux.src.keys() {
                ctx.build()
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexShared::new(src.tw))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        let gio_base = base / 4;
        for i in gio_base..(gio_base + 4) {
            let dst = wires::GIOB[i].cell(0);
            let src = backend.edev.db_index[tcid].pips_bwd[&dst]
                .iter()
                .next()
                .copied()
                .unwrap();
            bctx.build()
                .null_bits()
                .extra_tile_routing(
                    Delta::new(0, dy - 20, tcls::CMT),
                    wires::GIOB_CMT[i].cell(20),
                    wires::GIOB[i].cell(20).pos(),
                )
                .extra_tile_routing(
                    Delta::new(0, dy + 20, tcls::CMT),
                    wires::GIOB_CMT[i].cell(20),
                    wires::GIOB[i].cell(20).pos(),
                )
                .global_mutex("GIO", "TEST")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexShared::new(src.tw))
                .test_routing(dst, src)
                .pip(format!("GIO{i}_CMT"), format!("GIO{i}"))
                .commit();
        }
    }
    {
        let tcid = tcls::HCLK_IO;
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::BUFIO[i]);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("BUFIODQS")
                .commit();
            bctx.mode("BUFIODQS").test_bel_attr_bool_auto(
                bcls::BUFIO::DQSMASK_ENABLE,
                "FALSE",
                "TRUE",
            );
            bctx.build()
                .test_bel_attr_bits(bcls::BUFIO::ENABLE)
                .pip((PinFar, "O"), "O")
                .commit();
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::BUFR[i]);
            bctx.build()
                .global_mutex("RCLK", "BUFR")
                .test_bel_attr_bits(bcls::BUFR::ENABLE)
                .mode("BUFR")
                .commit();
            bctx.mode("BUFR")
                .global_mutex("RCLK", "BUFR")
                .test_bel_attr_rename("BUFR_DIVIDE", bcls::BUFR::DIVIDE);
        }
        {
            let mut bctx = ctx.bel(bslots::IDELAYCTRL);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("IDELAYCTRL")
                .commit();
            bctx.mode("IDELAYCTRL")
                .test_bel_attr_auto(bcls::IDELAYCTRL::RESET_STYLE);
            bctx.mode("IDELAYCTRL").test_bel_attr_bool_auto(
                bcls::IDELAYCTRL::HIGH_PERFORMANCE_MODE,
                "FALSE",
                "TRUE",
            );
            bctx.mode("IDELAYCTRL")
                .tile_mutex("IDELAYCTRL", "TEST")
                .test_bel_special(specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY)
                .attr("IDELAYCTRL_EN", "DEFAULT")
                .attr("BIAS_MODE", "2")
                .commit();
            bctx.mode("IDELAYCTRL")
                .tile_mutex("IDELAYCTRL", "TEST")
                .test_bel_special(specials::IDELAYCTRL_IODELAY_FULL)
                .attr("IDELAYCTRL_EN", "ENABLE")
                .attr("BIAS_MODE", "0")
                .commit();
            bctx.mode("IDELAYCTRL")
                .tile_mutex("IDELAYCTRL", "TEST")
                .attr("IDELAYCTRL_EN", "ENABLE")
                .test_bel_attr_bits(bcls::IDELAYCTRL::BIAS_MODE)
                .attr_diff("BIAS_MODE", "0", "1")
                .commit();
        }
        {
            let BelInfo::SwitchBox(ref sb) = backend.edev.db[tcid].bels[bslots::HCLK_IO_INT] else {
                unreachable!()
            };
            for item in &sb.items {
                let SwitchBoxItem::ProgDelay(delay) = item else {
                    continue;
                };
                for val in 0..2 {
                    ctx.build()
                        .test_raw(DiffKey::ProgDelay(tcid, delay.dst, val))
                        .prop(WireMutexExclusive::new(delay.dst))
                        .prop(WireMutexExclusive::new(delay.src.tw))
                        .prop(FuzzIntfDelay::new(delay.clone(), val != 0))
                        .commit();
                }
            }
            for i in 0..2 {
                let dst = wires::IMUX_BUFR[i].cell(4);
                let odst = wires::IMUX_BUFR[i ^ 1].cell(4);
                let mux = &backend.edev.db_index[tcid].muxes[&dst];
                for &src in mux.src.keys() {
                    if wires::MGT_ROW.contains(src.wire) {
                        ctx.build()
                            .row_mutex("MGT", "USE")
                            .prop(WireMutexExclusive::new(dst))
                            .prop(WireMutexExclusive::new(odst))
                            .prop(BaseIntPip::new(odst, src.tw))
                            .test_routing(dst, src)
                            .prop(FuzzIntPip::new(dst, src.tw))
                            .commit();
                    } else {
                        ctx.build()
                            .prop(WireMutexExclusive::new(dst))
                            .test_routing(dst, src)
                            .prop(FuzzIntPip::new(dst, src.tw))
                            .commit();
                    }
                }
            }
            for i in 0..12 {
                ctx.build()
                    .global_mutex("HCLK", "USE")
                    .row_mutex("BUFH_TEST", format!("USED_HCLK{i}"))
                    .prop(Related::new(
                        Cmt,
                        BaseIntPip::new(
                            wires::BUFH_TEST_W_IN.cell(20),
                            wires::HCLK_CMT_W[i].cell(20),
                        ),
                    ))
                    .prop(Related::new(
                        Cmt,
                        BaseIntPip::new(
                            wires::BUFH_TEST_E_IN.cell(20),
                            wires::HCLK_CMT_E[i].cell(20),
                        ),
                    ))
                    .test_routing(wires::HCLK_IO[i].cell(4), wires::HCLK_ROW[i].cell(4).pos())
                    .prop(FuzzIntPip::new(
                        wires::HCLK_IO[i].cell(4),
                        wires::HCLK_ROW[i].cell(4),
                    ))
                    .commit();
            }
            for i in 0..6 {
                ctx.build()
                    .global_mutex("RCLK", "USE")
                    .row_mutex("BUFH_TEST", format!("USED_RCLK{i}"))
                    .prop(Related::new(
                        Cmt,
                        BaseIntPip::new(
                            wires::BUFH_TEST_W_IN.cell(20),
                            wires::RCLK_CMT_W[i].cell(20),
                        ),
                    ))
                    .prop(Related::new(
                        Cmt,
                        BaseIntPip::new(
                            wires::BUFH_TEST_E_IN.cell(20),
                            wires::RCLK_CMT_E[i].cell(20),
                        ),
                    ))
                    .test_routing(wires::RCLK_IO[i].cell(4), wires::RCLK_ROW[i].cell(4).pos())
                    .prop(FuzzIntPip::new(
                        wires::RCLK_IO[i].cell(4),
                        wires::RCLK_ROW[i].cell(4),
                    ))
                    .commit();

                let dst = wires::RCLK_ROW[i].cell(4);
                let mux = &backend.edev.db_index[tcid].muxes[&dst];
                for &src in mux.src.keys() {
                    ctx.build()
                        .global_mutex("RCLK", "USE")
                        .prop(WireMutexExclusive::new(dst))
                        .row_mutex("BUFH_TEST", format!("USED_RCLK{i}"))
                        .prop(Related::new(
                            Cmt,
                            BaseIntPip::new(
                                wires::BUFH_TEST_W_IN.cell(20),
                                wires::RCLK_CMT_W[i].cell(20),
                            ),
                        ))
                        .prop(Related::new(
                            Cmt,
                            BaseIntPip::new(
                                wires::BUFH_TEST_E_IN.cell(20),
                                wires::RCLK_CMT_E[i].cell(20),
                            ),
                        ))
                        .test_routing(dst, src)
                        .prop(FuzzIntPip::new(dst, src.tw))
                        .commit();
                }
            }
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::PMVIOB);
        let mut bctx = ctx.bel(bslots::PMVIOB_CLK);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMVIOB")
            .commit();
        bctx.mode("PMVIOB")
            .test_bel_attr_bool_auto(bcls::PMVIOB::HSLEW4_IN, "FALSE", "TRUE");
        bctx.mode("PMVIOB")
            .test_bel_attr_bool_auto(bcls::PMVIOB::PSLEW4_IN, "FALSE", "TRUE");
        bctx.mode("PMVIOB")
            .test_bel_attr_bool_auto(bcls::PMVIOB::HYS_IN, "FALSE", "TRUE");
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    {
        let tcid = tcls::HCLK;
        let tdst0 = wires::LCLK[0].cell(0);
        let tdst1 = wires::LCLK[0].cell(1);
        let mut bits_hclk = vec![];
        let mut bits_rclk = vec![];
        for i in 0..12 {
            let src = wires::HCLK_ROW[i].cell(1).pos();
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_routing(tcid, tdst0, src).clone(),
                ctx.peek_diff_routing(tcid, tdst1, src).clone(),
            );
            let bit = xlat_bit(diff);
            bits_hclk.push(bit);
            ctx.insert_progbuf(tcid, wires::HCLK_BUF[i].cell(1), src, bit);
        }
        for i in 0..6 {
            let src = wires::RCLK_ROW[i].cell(1).pos();
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_routing(tcid, tdst0, src).clone(),
                ctx.peek_diff_routing(tcid, tdst1, src).clone(),
            );
            let bit = xlat_bit(diff);
            bits_rclk.push(bit);
            ctx.insert_progbuf(tcid, wires::RCLK_BUF[i].cell(1), src, bit);
        }
        for c in 0..2 {
            for o in 0..8 {
                let mut diffs = vec![(None, Diff::default())];
                let dst = wires::LCLK[o].cell(c);
                for i in 0..12 {
                    let mut diff =
                        ctx.get_diff_routing(tcid, dst, wires::HCLK_ROW[i].cell(1).pos());
                    diff.apply_bit_diff(bits_hclk[i], true, false);
                    diffs.push((Some(wires::HCLK_BUF[i].cell(1).pos()), diff));
                }
                for i in 0..6 {
                    let mut diff =
                        ctx.get_diff_routing(tcid, dst, wires::RCLK_ROW[i].cell(1).pos());
                    diff.apply_bit_diff(bits_rclk[i], true, false);
                    diffs.push((Some(wires::RCLK_BUF[i].cell(1).pos()), diff));
                }
                ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
            }
        }
    }
    for (tcid, base, c) in [(tcls::CLK_BUFG_S, 0, 1), (tcls::CLK_BUFG_N, 16, 0)] {
        for i in 0..16 {
            let bslot = bslots::BUFGCTRL[base + i];
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
            for attr in [
                bcls::BUFGCTRL::PRESELECT_I0,
                bcls::BUFGCTRL::PRESELECT_I1,
                bcls::BUFGCTRL::CREATE_EDGE,
                bcls::BUFGCTRL::INIT_OUT,
            ] {
                ctx.collect_bel_attr_bi(tcid, bslot, attr);
            }
        }
        for i in 0..16 {
            let dst = wires::OUT_BUFG_GFB[i].cell(c);
            let src = wires::OUT_BUFG[i].cell(c);
            ctx.collect_progbuf(tcid, dst, src.pos());
        }
        for i in 0..32 {
            let dst = wires::IMUX_BUFG_O[i].cell(c);
            // sigh. fucking. ise.
            let odst = wires::IMUX_BUFG_O[i ^ 1].cell(c);
            let tdst = ctx.edev.db_index[tcid].pips_fwd[&odst]
                .iter()
                .copied()
                .next()
                .unwrap();
            let mut item = xlat_bit(
                ctx.peek_diff_routing(tcid, dst, wires::IMUX_BUFG_I[i].cell(c).pos())
                    .clone(),
            );
            item.bit.bit += 1;
            ctx.insert_progbuf(tcid, tdst.tw, odst.pos(), item);
            ctx.collect_mux(tcid, dst);
        }
    }
    {
        let tcid = tcls::PMVIOB;
        let bslot = bslots::PMVIOB_CLK;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::PMVIOB::HYS_IN);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::PMVIOB::HSLEW4_IN);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::PMVIOB::PSLEW4_IN);
    }
    {
        let tcid = tcls::HCLK_IO;
        for i in 0..4 {
            let bslot = bslots::BUFIO[i];
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFIO::DQSMASK_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFIO::ENABLE);
        }
        for i in 0..2 {
            let bslot = bslots::BUFR[i];
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFR::ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFR::DIVIDE);
        }
        {
            let bslot = bslots::IDELAYCTRL;
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IDELAYCTRL::HIGH_PERFORMANCE_MODE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IDELAYCTRL::RESET_STYLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IDELAYCTRL::BIAS_MODE);
            let vctl_sel = vec![TileBit::new(0, 38, 27).pos(), TileBit::new(0, 38, 28).pos()];
            let diff_full =
                ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_FULL);
            let mut diff_default =
                ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY);
            diff_default.apply_bitvec_diff_int(&vctl_sel, 2, 0);
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
        for i in 0..4 {
            ctx.collect_delay(tcid, wires::IOCLK[i].cell(4), 2);
        }
        for i in 4..8 {
            let dst = wires::IOCLK[i].cell(4);
            let diff0 = ctx.get_diff_raw(&DiffKey::ProgDelay(tcid, dst, 0));
            let diff1 = ctx.get_diff_raw(&DiffKey::ProgDelay(tcid, dst, 1));
            let (diff0, diff1, common) = Diff::split(diff0, diff1);
            ctx.insert_delay(
                tcid,
                dst,
                xlat_enum_raw(vec![(0, diff0), (1, diff1)], OcdMode::ValueOrder),
            );
            let (dst, src) = if i < 6 {
                (wires::VIOCLK_S_BUF[i - 4], wires::VIOCLK_S[i - 4])
            } else {
                (wires::VIOCLK_N_BUF[i - 6], wires::VIOCLK_N[i - 6])
            };
            ctx.insert_progbuf(tcid, dst.cell(4), src.cell(4).pos(), xlat_bit(common));
        }
        for i in 0..2 {
            ctx.collect_mux(tcid, wires::IMUX_BUFR[i].cell(4));
        }
        for i in 0..12 {
            ctx.collect_progbuf(
                tcid,
                wires::HCLK_IO[i].cell(4),
                wires::HCLK_ROW[i].cell(4).pos(),
            );
        }
        for i in 0..6 {
            ctx.collect_progbuf(
                tcid,
                wires::RCLK_IO[i].cell(4),
                wires::RCLK_ROW[i].cell(4).pos(),
            );
            ctx.collect_mux(tcid, wires::RCLK_ROW[i].cell(4));
        }
    }
    {
        let tcid = tcls::CMT;
        for i in 0..32 {
            ctx.collect_progbuf(
                tcid,
                wires::GCLK_CMT[i].cell(20),
                wires::GCLK[i].cell(20).pos(),
            );
        }
        for i in 0..8 {
            ctx.collect_progbuf(
                tcid,
                wires::GIOB_CMT[i].cell(20),
                wires::GIOB[i].cell(20).pos(),
            );
        }
    }
}
