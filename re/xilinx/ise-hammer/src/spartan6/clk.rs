use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, PolTileWireCoord, SwitchBoxItem, TileClassId, TileWireCoord},
    dir::DirV,
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, extract_bitvec_val_part, extract_common_diff, xlat_bit, xlat_bit_wide,
    xlat_bit_wide_bi, xlat_enum_attr, xlat_enum_attr_ocd, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_spartan6::{
    chip::Gts,
    defs::{bcls, bslots, devdata, enums, tcls, tslots, wires},
};
use prjcombine_types::{bits, bitvec::BitVec};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            DynProp,
            mutex::{WireMutexExclusive, WireMutexShared},
            pip::PinFar,
        },
    },
    spartan6::specials,
};

#[derive(Clone, Debug)]
struct BufpllPll(DirV, TileClassId, TileWireCoord, PolTileWireCoord);

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
            let ntcrd = cell.tile(tslots::CMT_BUF);
            if let Some(ntile) = backend.edev.get_tile(ntcrd) {
                if ntile.class != self.1 {
                    return Some((fuzzer, true));
                }
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Routing(self.1, self.2, self.3),
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
        let mut ctx = FuzzCtx::new(session, backend, tcls::PCILOGICSE);
        let mut bctx = ctx.bel(bslots::PCILOGICSE);
        bctx.build()
            .no_global("PCI_CE_DELAY_LEFT")
            .test_bel_special(specials::PRESENT)
            .mode("PCILOGICSE")
            .commit();
        return;
    }
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    {
        let tcid = tcls::CLKC;
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..16 {
            let mut bctx = ctx.bel(bslots::BUFGMUX[i]);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("BUFGMUX")
                .commit();
            bctx.mode("BUFGMUX")
                .test_bel_input_inv_auto(bcls::BUFGMUX::S);
            bctx.mode("BUFGMUX")
                .test_bel_attr(bcls::BUFGMUX::CLK_SEL_TYPE);
            bctx.mode("BUFGMUX").test_bel_attr_bool_rename(
                "DISABLE_ATTR",
                bcls::BUFGMUX::INIT_OUT,
                "LOW",
                "HIGH",
            );
        }

        let mut bctx = ctx.bel(bslots::CLKC_INT);
        let BelInfo::SwitchBox(ref sb) = backend.edev.db[tcid].bels[bslots::CLKC_INT] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::Mux(mux) = item else {
                continue;
            };
            let alt_dst = if let Some(idx) = wires::CMT_BUFPLL_V_CLKOUT_S.index_of(mux.dst.wire) {
                wires::CMT_BUFPLL_V_CLKOUT_S[idx ^ 1]
            } else if let Some(idx) = wires::CMT_BUFPLL_V_CLKOUT_N.index_of(mux.dst.wire) {
                wires::CMT_BUFPLL_V_CLKOUT_N[idx ^ 1]
            } else if let Some(idx) = wires::CMT_BUFPLL_H_CLKOUT.index_of(mux.dst.wire) {
                wires::CMT_BUFPLL_H_CLKOUT[idx ^ 1]
            } else {
                continue;
            };
            let alt_dst = TileWireCoord {
                cell: mux.dst.cell,
                wire: alt_dst,
            };
            for &src in mux.src.keys() {
                let mut builder = bctx
                    .build()
                    .global_mutex_here("BUFPLL_CLK")
                    .test_raw(DiffKey::Routing(tcid, mux.dst, src))
                    .prop(WireMutexShared::new(src.tw))
                    .prop(WireMutexExclusive::new(mux.dst))
                    .prop(WireMutexExclusive::new(alt_dst))
                    .prop(BaseIntPip::new(alt_dst, src.tw))
                    .prop(FuzzIntPip::new(mux.dst, src.tw));
                if let Some(idx) = wires::CMT_BUFPLL_V_CLKOUT_S.index_of(mux.dst.wire) {
                    let tt = if edev.chip.rows.len() < 128 {
                        tcls::PLL_BUFPLL_OUT1_S
                    } else {
                        tcls::PLL_BUFPLL_OUT0_S
                    };
                    builder = builder.prop(BufpllPll(
                        DirV::S,
                        tt,
                        TileWireCoord::new_idx(0, wires::CMT_BUFPLL_V_CLKOUT_S[idx]),
                        TileWireCoord::new_idx(0, wires::CMT_BUFPLL_V_CLKOUT_N[idx]).pos(),
                    ));
                }
                builder.commit();
            }
        }
    }
    for tcid in [
        tcls::PLL_BUFPLL_OUT0_S,
        tcls::PLL_BUFPLL_OUT0_N,
        tcls::PLL_BUFPLL_OUT1_S,
        tcls::PLL_BUFPLL_OUT1_N,
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let BelInfo::SwitchBox(ref sb) = backend.edev.db[tcid].bels[bslots::CMT_BUF] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            if !wires::OUT_PLL_CLKOUT.contains(buf.src.wire) {
                continue;
            }
            ctx.build()
                .test_raw(DiffKey::Routing(tcid, buf.dst, buf.src))
                .prop(WireMutexShared::new(buf.src.tw))
                .prop(WireMutexExclusive::new(buf.dst))
                .prop(FuzzIntPip::new(buf.dst, buf.src.tw))
                .commit();
        }
    }
    for tcid in [
        tcls::DCM_BUFPLL_BUF_S,
        tcls::DCM_BUFPLL_BUF_S_MID,
        tcls::DCM_BUFPLL_BUF_N,
        tcls::DCM_BUFPLL_BUF_N_MID,
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::CMT_BUF);
        let BelInfo::SwitchBox(ref sb) = backend.edev.db[tcid].bels[bslots::CMT_BUF] else {
            unreachable!()
        };
        let (bufs_s, bufs_n) = match (tcid, edev.chip.rows.len() / 64) {
            (tcls::DCM_BUFPLL_BUF_S, _) => (vec![], vec![tcls::PLL_BUFPLL_OUT1_S]),
            (tcls::DCM_BUFPLL_BUF_S_MID, 2) => {
                (vec![tcls::PLL_BUFPLL_OUT1_S], vec![tcls::PLL_BUFPLL_OUT0_S])
            }
            (tcls::DCM_BUFPLL_BUF_S_MID, 3) => (
                vec![tcls::PLL_BUFPLL_OUT1_S, tcls::PLL_BUFPLL_S],
                vec![tcls::PLL_BUFPLL_OUT0_S, tcls::PLL_BUFPLL_S],
            ),
            (tcls::DCM_BUFPLL_BUF_N, 1) => (vec![], vec![tcls::PLL_BUFPLL_OUT1_N]),
            (tcls::DCM_BUFPLL_BUF_N, 2 | 3) => (vec![], vec![tcls::PLL_BUFPLL_OUT0_N]),
            (tcls::DCM_BUFPLL_BUF_N_MID, 2) => {
                (vec![tcls::PLL_BUFPLL_OUT0_N], vec![tcls::PLL_BUFPLL_OUT1_N])
            }
            (tcls::DCM_BUFPLL_BUF_N_MID, 3) => (
                vec![tcls::PLL_BUFPLL_OUT0_N, tcls::PLL_BUFPLL_N],
                vec![tcls::PLL_BUFPLL_OUT1_N, tcls::PLL_BUFPLL_N],
            ),
            _ => unreachable!(),
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            let (idx, pip_dir) =
                if let Some(idx) = wires::CMT_BUFPLL_V_CLKOUT_S.index_of(buf.dst.wire) {
                    (idx, DirV::S)
                } else if let Some(idx) = wires::CMT_BUFPLL_V_CLKOUT_N.index_of(buf.dst.wire) {
                    (idx, DirV::N)
                } else {
                    unreachable!()
                };
            let mut builder = bctx
                .build()
                .global_mutex_here("BUFPLL_CLK")
                .test_raw(DiffKey::Routing(tcid, buf.dst, buf.src))
                .prop(WireMutexShared::new(buf.src.tw))
                .prop(WireMutexExclusive::new(buf.dst))
                .prop(FuzzIntPip::new(buf.dst, buf.src.tw));

            for (dir, bufs) in [(DirV::S, &bufs_s), (DirV::N, &bufs_n)] {
                for &buf in bufs {
                    if matches!(buf, tcls::PLL_BUFPLL_OUT0_S | tcls::PLL_BUFPLL_OUT0_N)
                        && matches!(idx, 0..2)
                    {
                        continue;
                    }
                    if matches!(buf, tcls::PLL_BUFPLL_OUT1_S | tcls::PLL_BUFPLL_OUT1_N)
                        && matches!(idx, 2..4)
                    {
                        continue;
                    }
                    let (wt, wf) = match pip_dir {
                        DirV::S => (wires::CMT_BUFPLL_V_CLKOUT_S, wires::CMT_BUFPLL_V_CLKOUT_N),
                        DirV::N => (wires::CMT_BUFPLL_V_CLKOUT_N, wires::CMT_BUFPLL_V_CLKOUT_S),
                    };
                    builder = builder.prop(BufpllPll(
                        dir,
                        buf,
                        TileWireCoord::new_idx(0, wt[idx]),
                        TileWireCoord::new_idx(0, wf[idx]).pos(),
                    ));
                }
            }
            builder.commit();
        }
    }
    for (tcid, is_we) in [
        (tcls::CLK_S, false),
        (tcls::CLK_N, false),
        (tcls::CLK_W, true),
        (tcls::CLK_E, true),
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let tcls = &edev.db[tcid];
        let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::CLK_INT] else {
            unreachable!()
        };
        let mut muxes = HashMap::new();
        for item in &sb.items {
            let SwitchBoxItem::Mux(mux) = item else {
                continue;
            };
            muxes.insert(mux.dst, mux);
        }
        for i in 0..8 {
            let bslot = bslots::BUFIO2[i];
            let BelInfo::Bel(ref bel) = tcls.bels[bslot] else {
                unreachable!()
            };
            let mut bctx = ctx.bel(bslot);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("BUFIO2")
                .commit();
            bctx.build()
                .test_bel_special(specials::BUFIO2_2CLK)
                .mode("BUFIO2_2CLK")
                .commit();
            bctx.mode("BUFIO2")
                .attr("DIVIDE", "")
                .test_bel_attr_bool_auto(bcls::BUFIO2::DIVIDE_BYPASS, "FALSE", "TRUE");
            bctx.mode("BUFIO2")
                .attr("DIVIDE_BYPASS", "FALSE")
                .test_bel_attr(bcls::BUFIO2::DIVIDE);
            for (val, vname) in &backend.edev.db[enums::BUFIO2_DIVIDE].values {
                bctx.mode("BUFIO2_2CLK")
                    .attr("POS_EDGE", "")
                    .attr("NEG_EDGE", "")
                    .attr("R_EDGE", "")
                    .test_bel_attr_special_val(bcls::BUFIO2::DIVIDE, specials::BUFIO2_2CLK, val)
                    .attr("DIVIDE", vname.strip_prefix('_').unwrap())
                    .commit();
                bctx.mode("BUFIO2_2CLK")
                    .attr("DIVIDE", "")
                    .test_bel_attr_val(bcls::BUFIO2::POS_EDGE, val)
                    .attr("POS_EDGE", vname.strip_prefix('_').unwrap())
                    .commit();
                bctx.mode("BUFIO2_2CLK")
                    .attr("DIVIDE", "")
                    .test_bel_attr_val(bcls::BUFIO2::NEG_EDGE, val)
                    .attr("NEG_EDGE", vname.strip_prefix('_').unwrap())
                    .commit();
            }
            bctx.mode("BUFIO2_2CLK")
                .attr("DIVIDE", "")
                .test_bel_attr_bool_auto(bcls::BUFIO2::R_EDGE, "FALSE", "TRUE");

            bctx.mode("BUFIO2")
                .global_mutex("IOCLK_OUT", "TEST")
                .test_bel_attr_bits(bcls::BUFIO2::IOCLK_ENABLE)
                .pin("IOCLK")
                .pip((PinFar, "IOCLK"), "IOCLK")
                .commit();
            bctx.mode("BUFIO2")
                .global_mutex("BUFIO2_CMT_OUT", "TEST_BUFIO2")
                .test_bel_attr_bits(bcls::BUFIO2::CMT_ENABLE)
                .pin("DIVCLK")
                .pip((PinFar, "DIVCLK"), "DIVCLK")
                .pip("DIVCLK_CMT", (PinFar, "DIVCLK"))
                .commit();

            let wire_i = bel.inputs[bcls::BUFIO2::I].wire();
            let mux_i = muxes[&wire_i];
            for &src in mux_i.src.keys() {
                if matches!(tcid, tcls::CLK_W | tcls::CLK_E)
                    && wires::GTPCLK.contains(src.wire)
                    && matches!(i, 1 | 3)
                {
                    continue;
                }
                bctx.mode("BUFIO2")
                    .test_raw(DiffKey::Routing(tcid, wire_i, src))
                    .pin("I")
                    .prop(WireMutexShared::new(src.tw))
                    .prop(WireMutexExclusive::new(wire_i))
                    .prop(FuzzIntPip::new(wire_i, src.tw))
                    .commit();
            }

            let wire_ib = bel.inputs[bcls::BUFIO2::IB].wire();
            let mux_ib = muxes[&wire_ib];
            for &src in mux_ib.src.keys() {
                bctx.mode("BUFIO2_2CLK")
                    .test_raw(DiffKey::Routing(tcid, wire_ib, src))
                    .pin("IB")
                    .prop(WireMutexShared::new(src.tw))
                    .prop(WireMutexExclusive::new(wire_ib))
                    .prop(FuzzIntPip::new(wire_ib, src.tw))
                    .commit();
            }

            let bslot = bslots::BUFIO2FB[i];
            let BelInfo::Bel(ref bel) = tcls.bels[bslot] else {
                unreachable!()
            };
            let mut bctx = ctx.bel(bslot);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("BUFIO2FB")
                .commit();
            bctx.build()
                .test_bel_special(specials::BUFIO2_2CLK)
                .mode("BUFIO2FB_2CLK")
                .commit();
            bctx.mode("BUFIO2FB").test_bel_attr_bool_auto(
                bcls::BUFIO2FB::DIVIDE_BYPASS,
                "FALSE",
                "TRUE",
            );

            let wire_i = bel.inputs[bcls::BUFIO2FB::I].wire();
            let mux_i = muxes[&wire_i];
            for &src in mux_i.src.keys() {
                let (raw_src, invert_inputs) =
                    if let Some(idx) = wires::OUT_CLKPAD_CFB1.index_of(src.wire) {
                        (
                            TileWireCoord {
                                cell: src.cell,
                                wire: wires::OUT_CLKPAD_CFB0[idx],
                            }
                            .pos(),
                            "TRUE",
                        )
                    } else {
                        (src, "FALSE")
                    };
                bctx.mode("BUFIO2FB")
                    .test_raw(DiffKey::Routing(tcid, wire_i, src))
                    .pin("I")
                    .prop(WireMutexShared::new(raw_src.tw))
                    .prop(WireMutexExclusive::new(wire_i))
                    .prop(FuzzIntPip::new(wire_i, raw_src.tw))
                    .attr("INVERT_INPUTS", invert_inputs)
                    .commit();
            }

            bctx.mode("BUFIO2FB")
                .global_mutex("BUFIO2_CMT_OUT", "TEST_BUFIO2FB")
                .test_bel_special(specials::BUFIO2_CMT_ENABLE)
                .pin("O")
                .pip((PinFar, "O"), "O")
                .commit();
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::BUFPLL).sub(i + 1);
            bctx.build()
                .null_bits()
                .tile_mutex("BUFPLL", "PLAIN")
                .test_bel_special([specials::BUFPLL_BUFPLL0, specials::BUFPLL_BUFPLL1][i])
                .mode("BUFPLL")
                .commit();
            for (val, vname) in &edev.db[enums::BUFIO2_DIVIDE].values {
                bctx.mode("BUFPLL")
                    .tile_mutex("BUFPLL", "PLAIN")
                    .test_bel_attr_val([bcls::BUFPLL::DIVIDE0, bcls::BUFPLL::DIVIDE1][i], val)
                    .attr("DIVIDE", vname.strip_prefix('_').unwrap())
                    .commit();
            }
            for (val, vname) in &edev.db[enums::BUFPLL_DATA_RATE].values {
                bctx.mode("BUFPLL")
                    .tile_mutex("BUFPLL", "PLAIN")
                    .test_bel_attr_val([bcls::BUFPLL::DATA_RATE0, bcls::BUFPLL::DATA_RATE1][i], val)
                    .attr("DATA_RATE", vname)
                    .commit();
            }

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", "PLAIN")
                .no_pin("IOCLK")
                .test_bel_attr_bool_rename(
                    "ENABLE_SYNC",
                    [bcls::BUFPLL::ENABLE_SYNC0, bcls::BUFPLL::ENABLE_SYNC1][i],
                    "FALSE",
                    "TRUE",
                );

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", format!("SINGLE{i}"))
                .global_mutex("PLLCLK", "TEST")
                .attr("ENABLE_SYNC", "FALSE")
                .test_bel_attr_bits(
                    [
                        bcls::BUFPLL::ENABLE_NONE_SYNC0,
                        bcls::BUFPLL::ENABLE_NONE_SYNC1,
                    ][i],
                )
                .pin("IOCLK")
                .pip(format!("PLLCLK{i}"), format!("BUFPLL{i}_IOCLK"))
                .commit();

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", format!("SINGLE{i}"))
                .global_mutex("BUFPLL_CLK", "USE")
                .global_mutex("PLLCLK", "TEST")
                .attr("ENABLE_SYNC", "TRUE")
                .pip(
                    format!("BUFPLL{i}_PLLIN"),
                    if is_we {
                        format!("PLLIN_CMT{i}")
                    } else {
                        "PLLIN_SN0".to_string()
                    },
                )
                .test_bel_attr_bits(
                    [
                        bcls::BUFPLL::ENABLE_BOTH_SYNC0,
                        bcls::BUFPLL::ENABLE_BOTH_SYNC1,
                    ][i],
                )
                .pin("IOCLK")
                .pip(format!("PLLCLK{i}"), format!("BUFPLL{i}_IOCLK"))
                .commit();

            if !is_we {
                let (ci, w_pllin, w_locked) = if tcid == tcls::CLK_S {
                    (
                        1,
                        wires::CMT_BUFPLL_V_CLKOUT_N,
                        wires::CMT_BUFPLL_V_LOCKED_N,
                    )
                } else {
                    (
                        0,
                        wires::CMT_BUFPLL_V_CLKOUT_S,
                        wires::CMT_BUFPLL_V_LOCKED_S,
                    )
                };
                for j in 0..6 {
                    bctx.mode("BUFPLL")
                        .tile_mutex("BUFPLL", "PLAIN")
                        .tile_mutex("PLLIN", format!("BUFPLL{j}"))
                        .global_mutex("BUFPLL_CLK", "USE")
                        .mutex("PLLIN", format!("PLLIN{j}"))
                        .attr("ENABLE_SYNC", "FALSE")
                        .pin("PLLIN")
                        .pip(
                            format!("BUFPLL{ii}_PLLIN", ii = i ^ 1),
                            format!("PLLIN_SN{j}"),
                        )
                        .test_raw(DiffKey::Routing(
                            tcid,
                            TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_PLLIN[i]),
                            TileWireCoord::new_idx(ci, w_pllin[j]).pos(),
                        ))
                        .pip(format!("BUFPLL{i}_PLLIN"), format!("PLLIN_SN{j}"))
                        .commit();
                }
                for j in 0..3 {
                    bctx.mode("BUFPLL")
                        .tile_mutex("BUFPLL", "PLAIN")
                        .mutex("LOCKED", format!("LOCKED{j}"))
                        .test_raw(DiffKey::Routing(
                            tcid,
                            TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_LOCKED[i]),
                            TileWireCoord::new_idx(ci, w_locked[j]).pos(),
                        ))
                        .pin("LOCKED")
                        .pip(format!("BUFPLL{i}_LOCKED"), format!("LOCKED_SN{j}"))
                        .commit();
                }
            }

            bctx.mode("BUFPLL")
                .tile_mutex("BUFPLL", format!("SINGLE_{i}"))
                .test_bel_attr_bits(bcls::BUFPLL::ENABLE)
                .pin("IOCLK")
                .pip(format!("PLLCLK{i}"), format!("BUFPLL{i}_IOCLK"))
                .commit();
        }
        {
            let mut bctx = ctx.bel(bslots::BUFPLL);
            bctx.build()
                .tile_mutex("BUFPLL", "MCB")
                .test_bel_special(specials::PRESENT)
                .mode("BUFPLL_MCB")
                .commit();
            for (val, vname) in &edev.db[enums::BUFIO2_DIVIDE].values {
                bctx.build()
                    .tile_mutex("BUFPLL", "MCB")
                    .mode("BUFPLL_MCB")
                    .test_bel_attr_special_val(bcls::BUFPLL::DIVIDE0, specials::BUFPLL_MCB, val)
                    .attr("DIVIDE", vname.strip_prefix('_').unwrap())
                    .commit();
            }
            bctx.build()
                .tile_mutex("BUFPLL", "MCB")
                .mode("BUFPLL_MCB")
                .test_bel_attr_default(bcls::BUFPLL::LOCK_SRC, enums::BUFPLL_LOCK_SRC::NONE);

            if is_we {
                bctx.build()
                    .tile_mutex("BUFPLL", "MCB")
                    .mutex("PLLIN", "GCLK")
                    .mode("BUFPLL_MCB")
                    .test_bel_attr_val(bcls::BUFPLL::MUX_PLLIN, enums::BUFPLL_MUX_PLLIN::GCLK)
                    .pin("PLLIN0")
                    .pin("PLLIN1")
                    .pip("BUFPLL_MCB_PLLIN0", "PLLIN_GCLK0")
                    .pip("BUFPLL_MCB_PLLIN1", "PLLIN_GCLK1")
                    .commit();
                bctx.build()
                    .tile_mutex("BUFPLL", "MCB")
                    .mutex("PLLIN", "CMT")
                    .mode("BUFPLL_MCB")
                    .test_bel_attr_val(bcls::BUFPLL::MUX_PLLIN, enums::BUFPLL_MUX_PLLIN::CMT)
                    .pin("PLLIN0")
                    .pin("PLLIN1")
                    .pip("BUFPLL_MCB_PLLIN0", "PLLIN_CMT0")
                    .pip("BUFPLL_MCB_PLLIN1", "PLLIN_CMT1")
                    .commit();
            }

            bctx.build()
                .tile_mutex("BUFPLL", "MCB_OUT0")
                .mode("BUFPLL_MCB")
                .test_bel_attr_bits(bcls::BUFPLL::ENABLE)
                .pin("IOCLK0")
                .pip("PLLCLK0", "BUFPLL_MCB_IOCLK0")
                .commit();
            bctx.build()
                .tile_mutex("BUFPLL", "MCB_OUT1")
                .mode("BUFPLL_MCB")
                .test_bel_attr_bits(bcls::BUFPLL::ENABLE)
                .pin("IOCLK1")
                .pip("PLLCLK1", "BUFPLL_MCB_IOCLK1")
                .commit();
        }
        if !is_we {
            let n = match tcid {
                tcls::CLK_W => "L",
                tcls::CLK_E => "R",
                tcls::CLK_S => "B",
                tcls::CLK_N => "T",
                _ => unreachable!(),
            };
            let mut bctx = ctx.bel(bslots::MISR_CLK);
            bctx.build()
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "N")
                .test_bel_attr_bits(bcls::MISR::ENABLE)
                .global(format!("MISR_{n}M_EN"), "Y")
                .commit();
            bctx.build()
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "Y")
                .test_bel_attr_bits(bcls::MISR::RESET)
                .global(format!("MISR_{n}M_EN"), "Y")
                .commit();
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::PCILOGICSE);
        let mut bctx = ctx.bel(bslots::PCILOGICSE);
        bctx.build()
            .no_global("PCI_CE_DELAY_LEFT")
            .test_bel_special(specials::PRESENT)
            .mode("PCILOGICSE")
            .commit();
        for (val, vname) in &edev.db[enums::PCILOGICSE_PCI_CE_DELAY].values {
            bctx.build()
                .global("PCI_CE_DELAY_LEFT", vname)
                .test_bel_attr_val(bcls::PCILOGICSE::PCI_CE_DELAY, val)
                .mode("PCILOGICSE")
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };
    if devdata_only {
        let tcid = tcls::PCILOGICSE;
        let bslot = bslots::PCILOGICSE;
        let default = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let item = ctx.bel_attr_enum(tcid, bslot, bcls::PCILOGICSE::PCI_CE_DELAY);
        let val: BitVec = item
            .bits
            .iter()
            .map(|bit| default.bits.contains_key(bit))
            .collect();
        for (k, v) in &item.values {
            if *v == val {
                ctx.insert_devdata_enum(devdata::PCILOGICSE_PCI_CE_DELAY, k);
                break;
            }
        }
        return;
    }
    {
        let tcid = tcls::CLKC;
        for i in 0..16 {
            let bslot = bslots::BUFGMUX[i];
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::BUFGMUX::S);
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFGMUX::CLK_SEL_TYPE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFGMUX::INIT_OUT);
        }
        let BelInfo::SwitchBox(ref sb) = ctx.edev.db[tcid].bels[bslots::CLKC_INT] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::Mux(mux) = item else {
                continue;
            };
            if !(wires::CMT_BUFPLL_V_CLKOUT_S.contains(mux.dst.wire)
                || wires::CMT_BUFPLL_V_CLKOUT_N.contains(mux.dst.wire)
                || wires::CMT_BUFPLL_H_CLKOUT.contains(mux.dst.wire))
            {
                continue;
            }
            let mut diffs = vec![];
            for &src in mux.src.keys() {
                diffs.push((Some(src), ctx.get_diff_routing(tcid, mux.dst, src)));
            }
            ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::BitOrder));
        }
    }
    for tcid in [
        tcls::PLL_BUFPLL_S,
        tcls::PLL_BUFPLL_N,
        tcls::PLL_BUFPLL_OUT0_S,
        tcls::PLL_BUFPLL_OUT0_N,
        tcls::PLL_BUFPLL_OUT1_S,
        tcls::PLL_BUFPLL_OUT1_N,
        tcls::DCM_BUFPLL_BUF_S,
        tcls::DCM_BUFPLL_BUF_S_MID,
        tcls::DCM_BUFPLL_BUF_N,
        tcls::DCM_BUFPLL_BUF_N_MID,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let BelInfo::SwitchBox(ref sb) = edev.db[tcid].bels[bslots::CMT_BUF] else {
            unreachable!()
        };
        for item in &sb.items {
            let SwitchBoxItem::ProgBuf(buf) = item else {
                continue;
            };
            ctx.collect_progbuf(tcid, buf.dst, buf.src);
        }
    }

    for (tcid, is_we) in [
        (tcls::CLK_S, false),
        (tcls::CLK_N, false),
        (tcls::CLK_W, true),
        (tcls::CLK_E, true),
    ] {
        let tcls = &edev.db[tcid];
        let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::CLK_INT] else {
            unreachable!()
        };
        let mut muxes = HashMap::new();
        for item in &sb.items {
            let SwitchBoxItem::Mux(mux) = item else {
                continue;
            };
            muxes.insert(mux.dst, mux);
        }

        for i in 0..8 {
            let bslot = bslots::BUFIO2[i];
            let BelInfo::Bel(ref bel) = tcls.bels[bslot] else {
                unreachable!()
            };

            let bslot_fb = bslots::BUFIO2FB[i];
            let BelInfo::Bel(ref bel_fb) = tcls.bels[bslot_fb] else {
                unreachable!()
            };

            ctx.collect_bel_attr(tcid, bslot, bcls::BUFIO2::CMT_ENABLE);
            let diff = ctx.get_diff_bel_special(tcid, bslot_fb, specials::BUFIO2_CMT_ENABLE);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::BUFIO2::CMT_ENABLE, xlat_bit(diff));

            ctx.collect_bel_attr(tcid, bslot, bcls::BUFIO2::IOCLK_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFIO2::R_EDGE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFIO2::DIVIDE_BYPASS);
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::BUFIO2_2CLK);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::BUFIO2::R_EDGE),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::BUFIO2::DIVIDE_BYPASS),
                false,
                true,
            );
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::BUFIO2::ENABLE_2CLK, xlat_bit(diff));

            let mut pos_edge = vec![];
            let mut neg_edge = vec![];
            for val in ctx.edev.db[enums::BUFIO2_DIVIDE].values.ids() {
                let diff = ctx.get_diff_attr_val(tcid, bslot, bcls::BUFIO2::POS_EDGE, val);
                pos_edge.push((val, diff));
                let diff = ctx.get_diff_attr_val(tcid, bslot, bcls::BUFIO2::NEG_EDGE, val);
                neg_edge.push((val, diff));
            }
            let pos_edge = xlat_enum_raw(pos_edge, OcdMode::BitOrder);
            assert_eq!(pos_edge.bits.len(), 3);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_1], bits![0, 0, 0]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_2], bits![1, 0, 0]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_3], bits![0, 0, 0]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_4], bits![1, 1, 0]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_5], bits![0, 0, 0]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_6], bits![1, 0, 1]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_7], bits![0, 1, 1]);
            assert_eq!(pos_edge.values[&enums::BUFIO2_DIVIDE::_8], bits![1, 1, 1]);
            let pos_edge = Vec::from_iter(pos_edge.bits.into_iter().map(|bit| bit.pos()));
            let neg_edge = xlat_enum_raw(neg_edge, OcdMode::BitOrder);
            assert_eq!(neg_edge.bits.len(), 2);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_1], bits![0, 0]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_2], bits![1, 0]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_3], bits![0, 1]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_4], bits![0, 0]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_5], bits![0, 0]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_6], bits![0, 0]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_7], bits![0, 0]);
            assert_eq!(neg_edge.values[&enums::BUFIO2_DIVIDE::_8], bits![0, 0]);
            let neg_edge = Vec::from_iter(neg_edge.bits.into_iter().map(|bit| bit.pos()));

            let mut divide = vec![];
            for val in ctx.edev.db[enums::BUFIO2_DIVIDE].values.ids() {
                let mut diff = ctx.get_diff_attr_val(tcid, bslot, bcls::BUFIO2::DIVIDE, val);
                if matches!(
                    val,
                    enums::BUFIO2_DIVIDE::_2
                        | enums::BUFIO2_DIVIDE::_4
                        | enums::BUFIO2_DIVIDE::_6
                        | enums::BUFIO2_DIVIDE::_8
                ) {
                    diff.apply_bit_diff(
                        ctx.bel_attr_bit(tcid, bslot, bcls::BUFIO2::R_EDGE),
                        true,
                        false,
                    );
                }
                let diff2 = ctx.get_diff_attr_special_val(
                    tcid,
                    bslot,
                    bcls::BUFIO2::DIVIDE,
                    specials::BUFIO2_2CLK,
                    val,
                );
                assert_eq!(diff, diff2);
                let pos = extract_bitvec_val_part(&pos_edge, &bits![0, 0, 0], &mut diff);
                assert_eq!(
                    pos,
                    match val {
                        enums::BUFIO2_DIVIDE::_1 => bits![0, 0, 0],
                        enums::BUFIO2_DIVIDE::_2 => bits![1, 0, 0],
                        enums::BUFIO2_DIVIDE::_3 => bits![0, 1, 0],
                        enums::BUFIO2_DIVIDE::_4 => bits![1, 1, 0],
                        enums::BUFIO2_DIVIDE::_5 => bits![0, 0, 1],
                        enums::BUFIO2_DIVIDE::_6 => bits![1, 0, 1],
                        enums::BUFIO2_DIVIDE::_7 => bits![0, 1, 1],
                        enums::BUFIO2_DIVIDE::_8 => bits![1, 1, 1],
                        _ => unreachable!(),
                    }
                );
                let neg = extract_bitvec_val_part(&neg_edge, &bits![0, 0], &mut diff);
                assert_eq!(
                    neg,
                    match val {
                        enums::BUFIO2_DIVIDE::_1 => bits![0, 0],
                        enums::BUFIO2_DIVIDE::_2 => bits![1, 0],
                        enums::BUFIO2_DIVIDE::_3 => bits![0, 0],
                        enums::BUFIO2_DIVIDE::_4 => bits![0, 0],
                        enums::BUFIO2_DIVIDE::_5 => bits![1, 0],
                        enums::BUFIO2_DIVIDE::_6 => bits![1, 0],
                        enums::BUFIO2_DIVIDE::_7 => bits![0, 1],
                        enums::BUFIO2_DIVIDE::_8 => bits![0, 1],
                        _ => unreachable!(),
                    }
                );
                divide.push((val, diff));
            }
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BUFIO2::POS_EDGE, pos_edge);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BUFIO2::NEG_EDGE, neg_edge);
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::BUFIO2::DIVIDE,
                xlat_enum_attr_ocd(divide, OcdMode::BitOrder),
            );

            let wire_i = bel.inputs[bcls::BUFIO2::I].wire();
            let mux_i = muxes[&wire_i];
            let mut diffs = vec![];
            let mut fixup_src = None;
            for &src in mux_i.src.keys() {
                if matches!(tcid, tcls::CLK_W | tcls::CLK_E)
                    && wires::GTPCLK.contains(src.wire)
                    && matches!(i, 1 | 3)
                {
                    fixup_src = Some(src);
                    continue;
                }
                diffs.push((Some(src), ctx.get_diff_routing(tcid, wire_i, src)));
            }
            let enable = xlat_bit(extract_common_diff(&mut diffs));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::BUFIO2::ENABLE, enable);
            let mut item = xlat_enum_raw(diffs, OcdMode::BitOrder);
            if let Some(fixup_src) = fixup_src {
                assert_eq!(item.bits.len(), 3);
                item.values.insert(Some(fixup_src), bits![1, 1, 1]);
            }
            ctx.insert_mux(tcid, wire_i, item);

            let wire_ib = bel.inputs[bcls::BUFIO2::IB].wire();
            let mux_ib = muxes[&wire_ib];
            let mut diffs = vec![];
            for &src in mux_ib.src.keys() {
                diffs.push((Some(src), ctx.get_diff_routing(tcid, wire_ib, src)));
            }
            ctx.insert_mux(tcid, wire_ib, xlat_enum_raw(diffs, OcdMode::BitOrder));

            // BUFIO2FB

            let divide_bypass = xlat_bit_wide_bi(
                ctx.get_diff_attr_bool_bi(tcid, bslot_fb, bcls::BUFIO2FB::DIVIDE_BYPASS, false),
                ctx.get_diff_attr_bool_bi(tcid, bslot_fb, bcls::BUFIO2FB::DIVIDE_BYPASS, true),
            );
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot_fb,
                bcls::BUFIO2FB::DIVIDE_BYPASS,
                divide_bypass,
            );

            let wire_i = bel_fb.inputs[bcls::BUFIO2FB::I].wire();
            let mux_i = muxes[&wire_i];
            let mut diffs = vec![];
            let mut diff_cfb = None;
            for &src in mux_i.src.keys() {
                let diff = ctx.get_diff_routing(tcid, wire_i, src);
                if wires::OUT_CLKPAD_CFB0.contains(src.wire) {
                    diff_cfb = Some(diff.clone());
                }
                diffs.push((Some(src), diff));
            }
            let diff_enable = extract_common_diff(&mut diffs);
            let diff_cfb = diff_cfb.unwrap().combine(&!&diff_enable);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot_fb,
                bcls::BUFIO2FB::ENABLE,
                xlat_bit(diff_enable),
            );
            ctx.insert_mux(tcid, wire_i, xlat_enum_raw(diffs, OcdMode::BitOrder));

            let mut present = ctx.get_diff_bel_special(tcid, bslot_fb, specials::BUFIO2_2CLK);
            present.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot_fb, bcls::BUFIO2FB::DIVIDE_BYPASS),
                0,
                0xf,
            );
            present = present.combine(&!diff_cfb);
            present.assert_empty();
        }
        {
            let bslot = bslots::BUFPLL;
            for val in edev.db[enums::BUFIO2_DIVIDE].values.ids() {
                let diff = ctx.get_diff_attr_special_val(
                    tcid,
                    bslot,
                    bcls::BUFPLL::DIVIDE0,
                    specials::BUFPLL_MCB,
                    val,
                );
                let diff0 = ctx.peek_diff_attr_val(tcid, bslot, bcls::BUFPLL::DIVIDE0, val);
                let diff1 = ctx.peek_diff_attr_val(tcid, bslot, bcls::BUFPLL::DIVIDE1, val);
                assert_eq!(diff, diff0.combine(diff1));
            }
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFPLL::DATA_RATE0);
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFPLL::DATA_RATE1);
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::BUFPLL::DIVIDE0, OcdMode::BitOrder);
            ctx.collect_bel_attr_ocd(tcid, bslot, bcls::BUFPLL::DIVIDE1, OcdMode::BitOrder);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFPLL::ENABLE_SYNC0);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::BUFPLL::ENABLE_SYNC1);
            ctx.collect_bel_attr(tcid, bslot, bcls::BUFPLL::ENABLE);
            let enable = ctx.bel_attr_bit(tcid, bslot, bcls::BUFPLL::ENABLE);
            for i in 0..2 {
                let mut diff = ctx.get_diff_attr_bool(
                    tcid,
                    bslot,
                    [
                        bcls::BUFPLL::ENABLE_NONE_SYNC0,
                        bcls::BUFPLL::ENABLE_NONE_SYNC1,
                    ][i],
                );
                diff.apply_bit_diff(enable, true, false);
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslot,
                    [
                        bcls::BUFPLL::ENABLE_NONE_SYNC0,
                        bcls::BUFPLL::ENABLE_NONE_SYNC1,
                    ][i],
                    xlat_bit_wide(diff),
                );
                let mut diff = ctx.get_diff_attr_bool(
                    tcid,
                    bslot,
                    [
                        bcls::BUFPLL::ENABLE_BOTH_SYNC0,
                        bcls::BUFPLL::ENABLE_BOTH_SYNC1,
                    ][i],
                );
                diff.apply_bit_diff(enable, true, false);
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslot,
                    [
                        bcls::BUFPLL::ENABLE_BOTH_SYNC0,
                        bcls::BUFPLL::ENABLE_BOTH_SYNC1,
                    ][i],
                    xlat_bit_wide(diff),
                );

                if !is_we {
                    let (ci, w_pllin, w_locked) = if tcid == tcls::CLK_S {
                        (
                            1,
                            wires::CMT_BUFPLL_V_CLKOUT_N,
                            wires::CMT_BUFPLL_V_LOCKED_N,
                        )
                    } else {
                        (
                            0,
                            wires::CMT_BUFPLL_V_CLKOUT_S,
                            wires::CMT_BUFPLL_V_LOCKED_S,
                        )
                    };

                    let dst = TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_PLLIN[i]);
                    let mut diffs = vec![];
                    for j in 0..6 {
                        let src = TileWireCoord::new_idx(ci, w_pllin[j]).pos();
                        diffs.push((Some(src), ctx.get_diff_routing(tcid, dst, src)));
                    }
                    ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::BitOrder));

                    let dst = TileWireCoord::new_idx(ci, wires::IMUX_BUFPLL_LOCKED[i]);
                    let mut diffs = vec![];
                    for j in 0..3 {
                        let src = TileWireCoord::new_idx(ci, w_locked[j]).pos();
                        diffs.push((Some(src), ctx.get_diff_routing(tcid, dst, src)));
                    }
                    ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::BitOrder));
                }
            }
            if is_we {
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFPLL::MUX_PLLIN);
            }
            let mut diff0 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::BUFPLL::LOCK_SRC,
                enums::BUFPLL_LOCK_SRC::LOCK_TO_0,
            );
            diff0.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::BUFPLL::ENABLE_SYNC1),
                false,
                true,
            );
            let mut diff1 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::BUFPLL::LOCK_SRC,
                enums::BUFPLL_LOCK_SRC::LOCK_TO_1,
            );
            diff1.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::BUFPLL::ENABLE_SYNC0),
                false,
                true,
            );
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::BUFPLL::LOCK_SRC,
                xlat_enum_attr(vec![
                    (enums::BUFPLL_LOCK_SRC::NONE, Diff::default()),
                    (enums::BUFPLL_LOCK_SRC::LOCK_TO_0, diff0),
                    (enums::BUFPLL_LOCK_SRC::LOCK_TO_1, diff1),
                ]),
            );
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            diff.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::BUFPLL::ENABLE_BOTH_SYNC0),
                7,
                0,
            );
            diff.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::BUFPLL::ENABLE_BOTH_SYNC1),
                7,
                0,
            );
            diff.assert_empty();
        }
        if !is_we {
            let bslot = bslots::MISR_CLK;
            let has_gt = match tcid {
                tcls::CLK_S => matches!(edev.chip.gts, Gts::Quad(_, _)),
                tcls::CLK_N => edev.chip.gts != Gts::None,
                _ => unreachable!(),
            };
            if has_gt && !ctx.device.name.starts_with("xa") {
                ctx.collect_bel_attr(tcid, bslot, bcls::MISR::ENABLE);
                let mut diff = ctx.get_diff_attr_bool(tcid, bslot, bcls::MISR::RESET);
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, bcls::MISR::ENABLE),
                    true,
                    false,
                );
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::MISR::RESET, xlat_bit(diff));
            } else {
                // they're sometimes working, sometimes not, in nonsensical ways; just kill them
                ctx.get_diff_attr_bool(tcid, bslot, bcls::MISR::ENABLE);
                ctx.get_diff_attr_bool(tcid, bslot, bcls::MISR::RESET);
            }
        }
    }
    {
        let tcid = tcls::PCILOGICSE;
        let bslot = bslots::PCILOGICSE;
        let default = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let mut diffs = vec![];
        for val in edev.db[enums::PCILOGICSE_PCI_CE_DELAY].values.ids() {
            let diff = ctx.get_diff_attr_val(tcid, bslot, bcls::PCILOGICSE::PCI_CE_DELAY, val);
            if diff == default {
                ctx.insert_devdata_enum(devdata::PCILOGICSE_PCI_CE_DELAY, val);
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
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::PCILOGICSE::ENABLE, xlat_bit(enable));
        }
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::PCILOGICSE::PCI_CE_DELAY,
            xlat_enum_attr(diffs),
        );
    }
}
