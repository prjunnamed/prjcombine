use std::collections::BTreeSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{TileClassId, WireSlotIdExt},
    dir::{DirH, DirV},
    grid::{RowId, TileCoord},
};
use prjcombine_re_collector::diff::{Diff, DiffKey, OcdMode, xlat_bit, xlat_enum_raw};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls::{BUFGCTRL, BUFHCE, BUFIO, BUFR, IDELAYCTRL},
    bslots, tslots,
    virtex7::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            DynProp,
            mutex::{WireMutexExclusive, WireMutexShared},
            relation::{Delta, Related, TileRelation},
        },
    },
    virtex4::specials,
};

#[derive(Clone, Copy, Debug)]
pub struct ColPair(pub TileClassId);

impl TileRelation for ColPair {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = match edev.col_side(tcrd.col) {
            DirH::W => tcrd.col + 1,
            DirH::E => tcrd.col - 1,
        };
        let ntcrd = tcrd.with_col(col).tile(edev.db[self.0].slot);
        if edev.get_tile(ntcrd)?.class == self.0 {
            Some(ntcrd)
        } else {
            None
        }
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
            DirH::W => edev.col_io_w.unwrap() + 1,
            DirH::E => edev.col_io_e.unwrap() - 1,
        };
        let ntcrd = tcrd.with_col(scol).tile(tslots::BEL);
        if edev.get_tile(ntcrd)?.class == tcls::CMT {
            Some(ntcrd)
        } else {
            None
        }
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
            let ntcrd = cell.tile(tslots::BEL);
            if let Some(ntile) = backend.edev.get_tile(ntcrd)
                && matches!(ntile.class, tcls::CLK_BUFG_REBUF | tcls::CLK_BALI_REBUF)
            {
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

fn add_fuzzers_hclk<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::HCLK);
    let mut bctx = ctx.bel(bslots::HCLK);
    for c in 0..2 {
        let sn = ['S', 'N'][c];
        for o in 0..12 {
            let dst = wires::LCLK[o].cell(c);
            for i in 0..12 {
                let src = wires::HCLK_ROW[i].cell(1);
                let sname = &if (i < 8) == (o < 6) {
                    format!("HCLK{i}")
                } else {
                    format!("HCLK{i}_I")
                };
                bctx.build()
                    .tile_mutex("MODE", "TEST")
                    .global_mutex("HCLK", "USE")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(src))
                    .has_related(Delta::new(0, -1, tcls::INT))
                    .has_related(Delta::new(-2, -1, tcls::INT))
                    .has_related(Delta::new(2, -1, tcls::INT))
                    .has_related(Delta::new(0, 1, tcls::INT))
                    .has_related(Delta::new(-2, 1, tcls::INT))
                    .has_related(Delta::new(2, 1, tcls::INT))
                    .related_tile_mutex(Delta::new(-2, 0, tcls::HCLK), "MODE", "PIN_L")
                    .related_pip(
                        Delta::new(-2, 0, tcls::HCLK),
                        format!("LCLK{o}_{sn}"),
                        sname,
                    )
                    .related_tile_mutex(Delta::new(2, 0, tcls::HCLK), "MODE", "PIN_R")
                    .related_pip(Delta::new(2, 0, tcls::HCLK), format!("LCLK{o}_{sn}"), sname)
                    .test_routing(dst, src.pos())
                    .pip(format!("LCLK{o}_{sn}"), sname)
                    .commit();
            }
            for i in 0..4 {
                let src = wires::RCLK_ROW[i].cell(1);
                let sname = &if o < 6 {
                    format!("RCLK{i}_I")
                } else {
                    format!("RCLK{i}")
                };
                bctx.build()
                    .tile_mutex("MODE", "TEST")
                    .global_mutex("RCLK", "USE")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(src))
                    .has_related(Delta::new(0, -1, tcls::INT))
                    .has_related(Delta::new(-2, -1, tcls::INT))
                    .has_related(Delta::new(2, -1, tcls::INT))
                    .has_related(Delta::new(0, 1, tcls::INT))
                    .has_related(Delta::new(-2, 1, tcls::INT))
                    .has_related(Delta::new(2, 1, tcls::INT))
                    .related_tile_mutex(Delta::new(-2, 0, tcls::HCLK), "MODE", "PIN_L")
                    .related_pip(
                        Delta::new(-2, 0, tcls::HCLK),
                        format!("LCLK{o}_{sn}"),
                        sname,
                    )
                    .related_tile_mutex(Delta::new(2, 0, tcls::HCLK), "MODE", "PIN_R")
                    .related_pip(Delta::new(2, 0, tcls::HCLK), format!("LCLK{o}_{sn}"), sname)
                    .test_routing(dst, src.pos())
                    .pip(format!("LCLK{o}_{sn}"), sname)
                    .commit();
            }
        }
    }
}

fn add_fuzzers_clk_bufg<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    for tcid in [tcls::CLK_BUFG_S, tcls::CLK_BUFG_N] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..16 {
            let mut bctx = ctx.bel(bslots::BUFGCTRL[i]);
            bctx.build()
                .test_bel_attr_bits(BUFGCTRL::ENABLE)
                .mode("BUFGCTRL")
                .commit();
            for pin in [
                BUFGCTRL::CE0,
                BUFGCTRL::CE1,
                BUFGCTRL::S0,
                BUFGCTRL::S1,
                BUFGCTRL::IGNORE0,
                BUFGCTRL::IGNORE1,
            ] {
                bctx.mode("BUFGCTRL").test_bel_input_inv_auto(pin);
            }
            bctx.mode("BUFGCTRL")
                .test_bel_attr_bool_auto(BUFGCTRL::PRESELECT_I0, "FALSE", "TRUE");
            bctx.mode("BUFGCTRL")
                .test_bel_attr_bool_auto(BUFGCTRL::PRESELECT_I1, "FALSE", "TRUE");
            bctx.mode("BUFGCTRL")
                .test_bel_attr_bool_auto(BUFGCTRL::CREATE_EDGE, "FALSE", "TRUE");
            bctx.mode("BUFGCTRL")
                .test_bel_attr_bool_auto(BUFGCTRL::INIT_OUT, "0", "1");

            let dst = wires::OUT_BUFG_GFB[i].cell(0);
            let src = wires::OUT_BUFG[i].cell(0);
            bctx.build()
                .tile_mutex("FB", "TEST")
                .test_routing(dst, src.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
            if edev.chips.first().unwrap().regs == 1 {
                let dst = wires::GCLK[i].cell(0);
                let src = wires::OUT_BUFG[i].cell(0);
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_routing_special(
                        ClkRebuf(DirV::S),
                        wires::GCLK[i].cell(1),
                        specials::PRESENT,
                    )
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            } else if tcid == tcls::CLK_BUFG_S {
                let dst = wires::GCLK[i].cell(0);
                let src = wires::OUT_BUFG[i].cell(0);
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_routing_special(
                        ClkRebuf(DirV::S),
                        wires::GCLK[i].cell(1),
                        specials::PRESENT,
                    )
                    .extra_tile_routing_special(
                        ClkRebuf(DirV::N),
                        wires::GCLK[i].cell(0),
                        specials::PRESENT,
                    )
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            } else {
                let dst = wires::GCLK[i + 16].cell(0);
                let src = wires::OUT_BUFG[i].cell(0);
                bctx.build()
                    .global_mutex("GCLK", "TEST")
                    .extra_tile_routing_special(
                        ClkRebuf(DirV::S),
                        wires::GCLK[i + 16].cell(1),
                        specials::PRESENT,
                    )
                    .extra_tile_routing_special(
                        ClkRebuf(DirV::N),
                        wires::GCLK[i + 16].cell(0),
                        specials::PRESENT,
                    )
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
            for dst in [
                wires::IMUX_BUFG_O[i * 2].cell(0),
                wires::IMUX_BUFG_O[i * 2 + 1].cell(0),
            ] {
                let tst = backend.edev.db_index[tcid].only_fwd(dst).tw;
                // ISE bug causes pips to be reversed?
                bctx.build()
                    .prop(WireMutexExclusive::new(dst))
                    .test_routing(tst, dst.pos())
                    .prop(FuzzIntPip::new(dst, tst))
                    .commit();

                for &src in backend.edev.db_index[tcid].muxes[&dst].src.keys() {
                    if let Some(idx) = wires::OUT_BUFG_GFB.index_of(src.wire) {
                        let fsrc = wires::OUT_BUFG[idx].cell(0);
                        bctx.build()
                            .tile_mutex("FB", "USE")
                            .prop(WireMutexExclusive::new(dst))
                            .prop(BaseIntPip::new(src.tw, fsrc))
                            .test_routing(dst, src)
                            .prop(FuzzIntPip::new(dst, src.tw))
                            .commit();
                    } else {
                        bctx.build()
                            .prop(WireMutexExclusive::new(dst))
                            .test_routing(dst, src)
                            .prop(FuzzIntPip::new(dst, src.tw))
                            .commit();
                    }
                }
            }
        }
    }
}

fn add_fuzzers_clk_hrow<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let tcid = tcls::CLK_HROW;
    let mut ctx = FuzzCtx::new(session, backend, tcid);

    // GCLK_TEST buffers
    for i in 0..32 {
        let mut bctx = ctx.bel(bslots::SPEC_INT).sub(i);
        let dst = wires::GCLK_TEST[i].cell(1);
        let buf = wires::GCLK_TEST_IN[i].cell(1);
        let src = wires::GCLK_HROW[i].cell(1);
        bctx.build()
            .test_routing(buf, src.pos())
            .mode("GCLK_TEST_BUF")
            .commit();
        for (val, vname) in [(false, "FALSE"), (true, "TRUE")] {
            bctx.build()
                .null_bits()
                .mode("GCLK_TEST_BUF")
                .test_bel_special(specials::PRESENT)
                .attr("GCLK_TEST_ENABLE", vname)
                .commit();
            bctx.mode("GCLK_TEST_BUF")
                .test_raw(DiffKey::RoutingInv(tcid, dst, val))
                .attr("INVERT_INPUT", vname)
                .commit();
        }
    }

    // BUFH_TEST buffers
    for (sub, dst) in [(32, wires::BUFH_TEST_W), (33, wires::BUFH_TEST_E)] {
        let dst = dst.cell(1);
        let mut bctx = ctx.bel(bslots::SPEC_INT).sub(sub);

        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("GCLK_TEST_BUF")
            .commit();
        for (val, vname) in [(false, "FALSE"), (true, "TRUE")] {
            bctx.build()
                .null_bits()
                .mode("GCLK_TEST_BUF")
                .test_bel_special(specials::PRESENT)
                .attr("GCLK_TEST_ENABLE", vname)
                .commit();
            bctx.mode("GCLK_TEST_BUF")
                .test_raw(DiffKey::RoutingInv(tcid, dst, val))
                .attr("INVERT_INPUT", vname)
                .commit();
        }
    }

    // BUFHCE
    for slots in [bslots::BUFHCE_W, bslots::BUFHCE_E] {
        for i in 0..12 {
            let bslot = slots[i];
            let mut bctx = ctx.bel(bslot);
            bctx.build()
                .test_bel_attr_bits(BUFHCE::ENABLE)
                .mode("BUFHCE")
                .commit();
            bctx.mode("BUFHCE").test_bel_input_inv_auto(BUFHCE::CE);
            bctx.mode("BUFHCE")
                .test_bel_attr_bool_auto(BUFHCE::INIT_OUT, "0", "1");
            bctx.mode("BUFHCE").test_bel_attr_auto(BUFHCE::CE_TYPE);
        }
    }

    let mut bctx = ctx.bel(bslots::SPEC_INT);
    // IMUX_BUFHCE
    for w in [wires::IMUX_BUFHCE_W, wires::IMUX_BUFHCE_E] {
        for i in 0..12 {
            let dst = w[i].cell(1);
            for &src in backend.edev.db_index[tcid].muxes[&dst].src.keys() {
                if let Some(idx) = wires::GCLK_HROW.index_of(src.wire) {
                    bctx.build()
                        .global_mutex("GCLK", "TEST")
                        .extra_tile_routing_special(
                            Delta::new(0, -12, tcls::CLK_BUFG_REBUF),
                            wires::GCLK[idx].cell(1),
                            specials::PRESENT,
                        )
                        .extra_tile_routing_special(
                            Delta::new(0, 12, tcls::CLK_BUFG_REBUF),
                            wires::GCLK[idx].cell(0),
                            specials::PRESENT,
                        )
                        .prop(WireMutexExclusive::new(dst))
                        .prop(WireMutexExclusive::new(src.tw))
                        .test_routing(dst, src)
                        .prop(FuzzIntPip::new(dst, src.tw))
                        .commit();
                } else {
                    bctx.build()
                        .prop(WireMutexExclusive::new(dst))
                        .prop(WireMutexExclusive::new(src.tw))
                        .test_routing(dst, src)
                        .prop(FuzzIntPip::new(dst, src.tw))
                        .commit();
                }
            }
        }
    }
    for w in [wires::BUFH_TEST_W_IN, wires::BUFH_TEST_E_IN] {
        let dst = w.cell(1);
        for &src in backend.edev.db_index[tcid].muxes[&dst].src.keys() {
            bctx.build()
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(src.tw))
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();
        }
    }

    let has_io_w = backend.edev.tile_index[tcls::CMT]
        .iter()
        .any(|loc| loc.cell.col <= edev.col_clk);
    let has_io_e = backend.edev.tile_index[tcls::CMT]
        .iter()
        .any(|loc| loc.cell.col > edev.col_clk);

    for i in 0..32 {
        let dst = wires::IMUX_BUFG_O[i].cell(1);
        for &src in backend.edev.db_index[tcid].muxes[&dst].src.keys() {
            if wires::GCLK_TEST.contains(src.wire) {
                // TODO
            } else if wires::RCLK_HROW_W.contains(src.wire) || wires::RCLK_HROW_E.contains(src.wire)
            {
                let odst = wires::IMUX_BUFG_O[i ^ 1].cell(1);
                bctx.build()
                    .mutex("CASCO", "CASCO")
                    .global_mutex("RCLK", "USE")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(odst))
                    .prop(WireMutexExclusive::new(src.tw))
                    .prop(BaseIntPip::new(odst, src.tw))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            } else if wires::HROW_I_HROW_W.contains(src.wire)
                || wires::HROW_I_HROW_E.contains(src.wire)
            {
                bctx.build()
                    .mutex("CASCO", "CASCO")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(src.tw))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            } else {
                bctx.build()
                    .mutex("CASCO", "CASCO")
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexShared::new(src.tw))
                    .test_routing(dst, src)
                    .prop(FuzzIntPip::new(dst, src.tw))
                    .commit();
            }
        }
        for j in [i, i ^ 1] {
            let src = wires::GCLK_TEST[j].cell(1).pos();
            bctx.build()
                .prop(WireMutexExclusive::new(dst))
                .test_routing(dst, src)
                .pip(format!("GCLK_TEST{i}"), format!("GCLK{j}_TEST_OUT"))
                .commit();
        }
        let gclk = wires::GCLK_HROW[i].cell(1);
        let gclk_test = wires::GCLK_TEST[i].cell(1);
        let bufhce = wires::IMUX_BUFHCE_W[i % 12].cell(1);
        bctx.build()
            .mutex("CASCO", "TEST_IN")
            .global_mutex("GCLK", "USE")
            .prop(WireMutexExclusive::new(gclk))
            .prop(WireMutexExclusive::new(bufhce))
            .prop(BaseIntPip::new(bufhce, gclk))
            .test_routing_special(gclk_test, specials::GCLK_TEST_IN)
            .pip(format!("GCLK{i}_TEST_IN"), format!("GCLK{i}"))
            .commit();
    }
    for (side, w, c, base) in [
        (DirH::W, wires::RCLK_HROW_W, 1, 0),
        (DirH::E, wires::RCLK_HROW_E, 2, 4),
    ] {
        for i in 0..4 {
            let dst = wires::IMUX_BUFG_O[base + i].cell(1);
            let src = w[i].cell(1);
            let fsrc = wires::RCLK_ROW[i].cell(c);
            let mut builder = bctx
                .build()
                .mutex("CASCO", "CASCO")
                .global_mutex("RCLK", "TEST_HROW")
                .prop(WireMutexExclusive::new(dst))
                .prop(WireMutexExclusive::new(src));
            if side == DirH::W {
                if has_io_w {
                    builder = builder.extra_tile_routing(
                        CmtDir(DirH::W),
                        wires::RCLK_CMT[i].cell(25),
                        wires::RCLK_ROW[i].cell(25).pos(),
                    );
                }
            } else {
                if has_io_e {
                    builder = builder.extra_tile_routing(
                        CmtDir(DirH::E),
                        wires::RCLK_CMT[i].cell(25),
                        wires::RCLK_ROW[i].cell(25).pos(),
                    );
                }
            }
            builder
                .test_routing(dst, fsrc.pos())
                .prop(FuzzIntPip::new(dst, src))
                .commit();
        }
    }
}

fn add_fuzzers_clk_bufg_rebuf<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    bali_only: bool,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let is_single_reg = edev.chips.values().all(|chip| chip.regs == 1);

    for tcid in [tcls::CLK_BUFG_REBUF, tcls::CLK_BALI_REBUF] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        if tcid == tcls::CLK_BUFG_REBUF && bali_only {
            continue;
        }
        for i in 0..32 {
            let mut bctx = ctx.bel(bslots::SPEC_INT).sub(i);
            let dst = if i.is_multiple_of(2) {
                wires::GCLK[i + 1].cell(0)
            } else {
                wires::GCLK[i - 1].cell(1)
            };
            let src = wires::GCLK_REBUF_TEST[i].cell(0);
            bctx.build()
                .null_bits()
                .mode("GCLK_TEST_BUF")
                .test_bel_special(specials::PRESENT)
                .attr("GCLK_TEST_ENABLE", "FALSE")
                .commit();
            if tcid == tcls::CLK_BUFG_REBUF {
                bctx.build()
                    .test_routing(dst, src.pos())
                    .mode("GCLK_TEST_BUF")
                    .commit();
                bctx.build()
                    .null_bits()
                    .mode("GCLK_TEST_BUF")
                    .test_bel_special(specials::PRESENT)
                    .attr("GCLK_TEST_ENABLE", "TRUE")
                    .commit();
            } else {
                bctx.build()
                    .null_bits()
                    .test_bel_special(specials::PRESENT)
                    .mode("GCLK_TEST_BUF")
                    .commit();
                bctx.mode("GCLK_TEST_BUF")
                    .test_routing(dst, src.pos())
                    .attr("GCLK_TEST_ENABLE", "TRUE")
                    .commit();
            }
            for (val, vname) in [(false, "FALSE"), (true, "TRUE")] {
                bctx.mode("GCLK_TEST_BUF")
                    .test_raw(DiffKey::RoutingInv(tcid, src, val))
                    .attr("INVERT_INPUT", vname)
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        for i in 0..32 {
            let ws = wires::GCLK[i].cell(0);
            let wn = wires::GCLK[i].cell(1);
            let ii = i / 2;
            if i.is_multiple_of(2) {
                if !is_single_reg {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_D0")
                        .pip(format!("BUF{ii}_S_CLKIN"), format!("GCLK{i}_S"))
                        .pip(format!("GCLK{i}_N"), format!("BUF{ii}_N_CLKOUT"))
                        .test_routing(ws, wn.pos())
                        .pip(format!("GCLK{i}_S"), format!("GCLK{i}_N"))
                        .commit();
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_U0")
                        .prop(HclkSide(DirV::S))
                        .related_pip(
                            ClkRebuf(DirV::S),
                            format!("GCLK{i}_N"),
                            format!("BUF{ii}_N_CLKOUT"),
                        )
                        .related_pip(
                            ClkRebuf(DirV::N),
                            format!("BUF{ii}_S_CLKIN"),
                            format!("GCLK{i}_S"),
                        )
                        .test_routing(wn, ws.pos())
                        .pip(format!("GCLK{i}_N"), format!("GCLK{i}_S"))
                        .commit();
                }
                if tcid == tcls::CLK_BALI_REBUF {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_BALI")
                        .prop(HclkSide(DirV::S))
                        .extra_tile_routing_special(ClkRebuf(DirV::S), wn, specials::PRESENT)
                        .test_routing_special(ws, specials::PRESENT)
                        .pip(format!("BUF{ii}_S_CLKIN"), format!("GCLK{i}_S"))
                        .commit();
                }
            } else {
                if !is_single_reg {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_U1")
                        .pip(format!("BUF{ii}_N_CLKIN"), format!("GCLK{i}_N"))
                        .pip(format!("GCLK{i}_S"), format!("BUF{ii}_S_CLKOUT"))
                        .test_routing(wn, ws.pos())
                        .pip(format!("GCLK{i}_N"), format!("GCLK{i}_S"))
                        .commit();
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_D1")
                        .prop(HclkSide(DirV::N))
                        .related_pip(
                            ClkRebuf(DirV::N),
                            format!("GCLK{i}_S"),
                            format!("BUF{ii}_S_CLKOUT"),
                        )
                        .related_pip(
                            ClkRebuf(DirV::S),
                            format!("BUF{ii}_N_CLKIN"),
                            format!("GCLK{i}_N"),
                        )
                        .test_routing(ws, wn.pos())
                        .pip(format!("GCLK{i}_S"), format!("GCLK{i}_N"))
                        .commit();
                }
                if tcid == tcls::CLK_BALI_REBUF {
                    bctx.build()
                        .global_mutex("GCLK", "REBUF_BALI")
                        .prop(HclkSide(DirV::S))
                        .extra_tile_routing_special(ClkRebuf(DirV::S), wn, specials::PRESENT)
                        .test_routing_special(ws, specials::PRESENT)
                        .pip(format!("GCLK{i}_S"), format!("BUF{ii}_S_CLKOUT"))
                        .commit();
                }
            }
        }
    }
}

fn add_fuzzers_hclk_io<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::HCLK_IO_HR, tcls::HCLK_IO_HP] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::BUFIO[i]);
            bctx.build()
                .test_bel_attr_bits(BUFIO::ENABLE)
                .mode("BUFIO")
                .commit();
            bctx.mode("BUFIO").test_bel_attr_bool_rename(
                "DELAY_BYPASS",
                BUFIO::DELAY_ENABLE,
                "TRUE",
                "FALSE",
            );
            let dst = wires::IMUX_BUFIO[i].cell(4);
            let src_ccio = wires::OUT_CLKPAD.cell([5, 7, 1, 3][i]);
            bctx.build()
                .prop(WireMutexExclusive::new(dst))
                .related_tile_mutex(ColPair(tcls::CMT), "CCIO", "USE_IO")
                .prop(Related::new(
                    ColPair(tcls::CMT),
                    WireMutexExclusive::new(wires::OMUX_HCLK_FREQ_BB[i].cell(25)),
                ))
                .prop(Related::new(
                    ColPair(tcls::CMT),
                    BaseIntPip::new(
                        wires::OMUX_HCLK_FREQ_BB[i].cell(25),
                        wires::CCIO_CMT[i].cell(25),
                    ),
                ))
                .test_routing(dst, src_ccio.pos())
                .prop(FuzzIntPip::new(dst, src_ccio))
                .commit();
            let src_perf = wires::PERF_IO[i].cell(4);
            bctx.build()
                .prop(WireMutexExclusive::new(dst))
                .related_tile_mutex(ColPair(tcls::CMT), "PERF", "USE_IO")
                .prop(Related::new(
                    ColPair(tcls::CMT),
                    BaseIntPip::new(wires::PERF[i].cell(75), wires::PERF_IN_PHASER[i].cell(25)),
                ))
                .test_routing(dst, src_perf.pos())
                .prop(FuzzIntPip::new(dst, src_perf))
                .commit();
        }
        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::BUFR[i]);
            bctx.build()
                .test_bel_attr_bits(BUFR::ENABLE)
                .mode("BUFR")
                .attr("BUFR_DIVIDE", "BYPASS")
                .commit();
            bctx.mode("BUFR")
                .test_bel_attr_rename("BUFR_DIVIDE", BUFR::DIVIDE);
        }
        {
            let mut bctx = ctx.bel(bslots::IDELAYCTRL);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("IDELAYCTRL")
                .commit();
            bctx.mode("IDELAYCTRL").test_bel_attr_bool_auto(
                IDELAYCTRL::HIGH_PERFORMANCE_MODE,
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
                .test_bel_attr_bits(IDELAYCTRL::BIAS_MODE)
                .attr_diff("BIAS_MODE", "0", "1")
                .commit();
        }
        {
            for c in [3, 4] {
                for o in 0..6 {
                    let dst = wires::LCLK_IO[o].cell(c);
                    for i in 0..12 {
                        let src = wires::HCLK_IO[i].cell(4);
                        let far_src = wires::HCLK_ROW[i].cell(4);
                        ctx.build()
                            .prop(WireMutexExclusive::new(dst))
                            .prop(WireMutexExclusive::new(src))
                            .test_routing(dst, far_src.pos())
                            .prop(FuzzIntPip::new(dst, src))
                            .commit();
                    }
                }
            }
            for i in 0..4 {
                let dst = wires::RCLK_IO[i].cell(4);
                let src = wires::RCLK_ROW[i].cell(4);
                let cmt_lclk = if i < 2 {
                    wires::LCLK_CMT_S[i].cell(25)
                } else {
                    wires::LCLK_CMT_N[i - 2].cell(25)
                };
                ctx.build()
                    .global_mutex("RCLK", "USE")
                    .prop(Related::new(
                        ColPair(tcls::CMT),
                        WireMutexExclusive::new(cmt_lclk),
                    ))
                    .prop(Related::new(
                        ColPair(tcls::CMT),
                        BaseIntPip::new(cmt_lclk, wires::RCLK_CMT[i].cell(25)),
                    ))
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
            }
        }
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    bali_only: bool,
) {
    if !bali_only {
        add_fuzzers_hclk(session, backend);
        add_fuzzers_clk_bufg(session, backend);
        add_fuzzers_clk_hrow(session, backend);
    }
    add_fuzzers_clk_bufg_rebuf(session, backend, bali_only);
    if !bali_only {
        add_fuzzers_hclk_io(session, backend);
    }
}

fn collect_fuzzers_hclk(ctx: &mut CollectorCtx) {
    let tcid = tcls::HCLK;
    let mut hclk_buf_bit = vec![];
    let mut rclk_buf_bit = vec![];
    for i in 0..12 {
        let (_, _, diff) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::LCLK[0].cell(0),
                wires::HCLK_ROW[i].cell(1).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::LCLK[0].cell(1),
                wires::HCLK_ROW[i].cell(1).pos(),
            )
            .clone(),
        );
        let bit = xlat_bit(diff);
        hclk_buf_bit.push(bit);
        ctx.insert_progbuf(
            tcid,
            wires::HCLK_BUF[i].cell(1),
            wires::HCLK_ROW[i].cell(1).pos(),
            bit,
        );
    }
    for i in 0..4 {
        let (_, _, diff) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::LCLK[0].cell(0),
                wires::RCLK_ROW[i].cell(1).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::LCLK[0].cell(1),
                wires::RCLK_ROW[i].cell(1).pos(),
            )
            .clone(),
        );
        let bit = xlat_bit(diff);
        rclk_buf_bit.push(bit);
        ctx.insert_progbuf(
            tcid,
            wires::RCLK_BUF[i].cell(1),
            wires::RCLK_ROW[i].cell(1).pos(),
            bit,
        );
    }
    for c in 0..2 {
        for o in 0..12 {
            let dst = wires::LCLK[o].cell(c);
            let mut diffs = vec![(None, Diff::default())];
            for i in 0..12 {
                let src = wires::HCLK_BUF[i].cell(1);
                let far_src = wires::HCLK_ROW[i].cell(1);
                let mut diff = ctx.get_diff_routing(tcid, dst, far_src.pos());
                diff.apply_bit_diff(hclk_buf_bit[i], true, false);
                diffs.push((Some(src.pos()), diff));
            }
            for i in 0..4 {
                let src = wires::RCLK_BUF[i].cell(1);
                let far_src = wires::RCLK_ROW[i].cell(1);
                let mut diff = ctx.get_diff_routing(tcid, dst, far_src.pos());
                diff.apply_bit_diff(rclk_buf_bit[i], true, false);
                diffs.push((Some(src.pos()), diff));
            }
            ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
        }
    }
}

fn collect_fuzzers_clk_bufg(ctx: &mut CollectorCtx) {
    for (tcid, base) in [(tcls::CLK_BUFG_S, 0), (tcls::CLK_BUFG_N, 16)] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for i in 0..16 {
            let bslot = bslots::BUFGCTRL[i];
            for pin in [
                BUFGCTRL::CE0,
                BUFGCTRL::CE1,
                BUFGCTRL::S0,
                BUFGCTRL::S1,
                BUFGCTRL::IGNORE0,
                BUFGCTRL::IGNORE1,
            ] {
                ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
            }
            ctx.collect_bel_attr(tcid, bslot, BUFGCTRL::ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFGCTRL::PRESELECT_I0);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFGCTRL::PRESELECT_I1);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFGCTRL::CREATE_EDGE);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFGCTRL::INIT_OUT);

            ctx.collect_progbuf(
                tcid,
                wires::OUT_BUFG_GFB[i].cell(0),
                wires::OUT_BUFG[i].cell(0).pos(),
            );
            ctx.collect_progbuf(
                tcid,
                wires::GCLK[i + base].cell(0),
                wires::OUT_BUFG[i].cell(0).pos(),
            );
        }

        for i in 0..32 {
            let dst = wires::IMUX_BUFG_O[i].cell(0);
            ctx.collect_mux(tcid, dst);
            let tst = ctx.edev.db_index[tcid].only_fwd(dst).tw;
            ctx.collect_progbuf(tcid, tst, dst.pos());
        }
    }
}

fn collect_fuzzers_clk_hrow(ctx: &mut CollectorCtx) {
    let tcid = tcls::CLK_HROW;

    for i in 0..32 {
        let dst = wires::GCLK_TEST[i].cell(1);
        let buf = wires::GCLK_TEST_IN[i].cell(1);
        let src = wires::GCLK_HROW[i].cell(1);
        ctx.collect_progbuf(tcid, buf, src.pos());
        ctx.collect_inv_bi(tcid, dst);
    }
    ctx.collect_inv_bi(tcid, wires::BUFH_TEST_W.cell(1));
    ctx.collect_inv_bi(tcid, wires::BUFH_TEST_E.cell(1));

    for slots in [bslots::BUFHCE_W, bslots::BUFHCE_E] {
        for i in 0..12 {
            let bslot = slots[i];
            ctx.collect_bel_attr(tcid, bslot, BUFHCE::ENABLE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, BUFHCE::CE);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFHCE::INIT_OUT);
            ctx.collect_bel_attr(tcid, bslot, BUFHCE::CE_TYPE);
        }
    }

    // buffers
    for i in 0..32 {
        let dst = wires::GCLK_HROW[i].cell(1);
        let src = wires::GCLK[i].cell(1);

        let (_, _, diff) = Diff::split(
            ctx.peek_diff_routing(tcid, wires::IMUX_BUFHCE_W[0].cell(1), dst.pos())
                .clone(),
            ctx.peek_diff_routing(tcid, wires::IMUX_BUFHCE_E[0].cell(1), dst.pos())
                .clone(),
        );
        ctx.insert_progbuf(tcid, dst, src.pos(), xlat_bit(diff));
    }
    for (wt, c) in [(wires::HROW_I_HROW_W, 1), (wires::HROW_I_HROW_E, 2)] {
        for i in 0..14 {
            let dst = wt[i].cell(1);
            let src = wires::HROW_I[i].cell(c);
            let (_, _, diff) = Diff::split(
                ctx.peek_diff_routing(tcid, wires::IMUX_BUFHCE_W[0].cell(1), dst.pos())
                    .clone(),
                ctx.peek_diff_routing(tcid, wires::IMUX_BUFHCE_E[0].cell(1), dst.pos())
                    .clone(),
            );
            ctx.insert_progbuf(tcid, dst, src.pos(), xlat_bit(diff));
        }
    }
    for (i, wh0, wh1) in [
        (
            0,
            wires::IMUX_BUFHCE_E[0].cell(1),
            wires::IMUX_BUFHCE_E[1].cell(1),
        ),
        (
            1,
            wires::IMUX_BUFHCE_E[0].cell(1),
            wires::IMUX_BUFHCE_E[1].cell(1),
        ),
        (
            2,
            wires::IMUX_BUFHCE_W[0].cell(1),
            wires::IMUX_BUFHCE_W[1].cell(1),
        ),
        (
            3,
            wires::IMUX_BUFHCE_W[0].cell(1),
            wires::IMUX_BUFHCE_W[1].cell(1),
        ),
    ] {
        let dst = wires::CKINT_HROW[i].cell(1);
        let src = ctx.edev.db_index[tcid].only_bwd(dst);
        let (_, _, diff) = Diff::split(
            ctx.peek_diff_routing(tcid, wh0, dst.pos()).clone(),
            ctx.peek_diff_routing(tcid, wh1, dst.pos()).clone(),
        );
        ctx.insert_progbuf(tcid, dst, src.pos(), xlat_bit(diff));
    }
    for (w, c, base) in [(wires::RCLK_HROW_W, 1, 0), (wires::RCLK_HROW_E, 2, 4)] {
        for i in 0..4 {
            let dst = wires::IMUX_BUFG_O[base + i].cell(1);
            let src = w[i].cell(1);
            let fsrc = wires::RCLK_ROW[i].cell(c);
            let diff = ctx
                .get_diff_routing(tcid, dst, fsrc.pos())
                .combine(&!ctx.peek_diff_routing(tcid, dst, src.pos()));
            ctx.insert_progbuf(tcid, src, fsrc.pos(), xlat_bit(diff));
        }
    }

    // IMUX_BUFHCE_*
    for w in wires::IMUX_BUFHCE_W
        .into_iter()
        .chain(wires::IMUX_BUFHCE_E)
        .chain([wires::BUFH_TEST_W_IN, wires::BUFH_TEST_E_IN])
    {
        let dst = w.cell(1);
        let mut diffs = vec![(None, Diff::default())];
        for &src in ctx.edev.db_index[tcid].muxes[&dst].src.keys() {
            let mut diff = ctx.get_diff_routing(tcid, dst, src);
            if !matches!(src.wire, wires::BUFH_TEST_W | wires::BUFH_TEST_E) {
                let fsrc = ctx.edev.db_index[tcid].only_bwd(src.tw);
                diff.apply_bit_diff(ctx.sb_progbuf(tcid, src.tw, fsrc), true, false);
            }
            diffs.push((Some(src), diff));
        }
        ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
    }

    for i in 0..32 {
        let dst = wires::IMUX_BUFG_O[i].cell(1);
        let mut diffs = vec![(None, Diff::default())];
        for &src in ctx.edev.db_index[tcid].muxes[&dst].src.keys() {
            if wires::GCLK_TEST.contains(src.wire) {
                // handled later
            } else {
                let mut diff = ctx.get_diff_routing(tcid, dst, src);
                if wires::HROW_I_HROW_W.contains(src.wire)
                    || wires::HROW_I_HROW_E.contains(src.wire)
                {
                    let fsrc = ctx.edev.db_index[tcid].only_bwd(src.tw);
                    diff.apply_bit_diff(ctx.sb_progbuf(tcid, src.tw, fsrc), true, false);
                }
                diffs.push((Some(src), diff));
            }
        }

        for j in [i, i ^ 1] {
            let src = wires::GCLK_TEST[j].cell(1);
            let mut diff = ctx
                .peek_diff_routing_special(tcid, src, specials::GCLK_TEST_IN)
                .clone();
            diff.bits
                .retain(|&bit, _| diffs.iter().any(|(_, odiff)| odiff.bits.contains_key(&bit)));
            diff = diff.combine(&ctx.get_diff_routing(tcid, dst, src.pos()));
            diffs.push((Some(src.pos()), diff));
        }

        ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
    }

    for i in 0..32 {
        let src = wires::GCLK_TEST[i].cell(1);
        // slurped above bit by bit
        ctx.get_diff_routing_special(tcid, src, specials::GCLK_TEST_IN);
    }

    let tcid = tcls::CMT;
    for i in 0..4 {
        ctx.collect_progbuf(
            tcid,
            wires::RCLK_CMT[i].cell(25),
            wires::RCLK_ROW[i].cell(25).pos(),
        );
    }
}

fn collect_fuzzers_clk_bufg_rebuf(ctx: &mut CollectorCtx, bali_only: bool) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let is_single_reg = edev.chips.values().all(|chip| chip.regs == 1);

    for tcid in [tcls::CLK_BUFG_REBUF, tcls::CLK_BALI_REBUF] {
        if bali_only && tcid == tcls::CLK_BUFG_REBUF {
            continue;
        }
        if !ctx.has_tcls(tcid) {
            continue;
        }

        for i in 0..32 {
            let src = wires::GCLK_REBUF_TEST[i].cell(0);
            let dst = if i.is_multiple_of(2) {
                wires::GCLK[i + 1].cell(0)
            } else {
                wires::GCLK[i - 1].cell(1)
            };
            ctx.collect_progbuf(tcid, dst, src.pos());
            ctx.collect_inv_bi(tcid, src);
        }

        for i in 0..32 {
            let ws = wires::GCLK[i].cell(0);
            let wn = wires::GCLK[i].cell(1);
            if !is_single_reg {
                ctx.collect_progbuf(tcid, ws, wn.pos());
                ctx.collect_progbuf(tcid, wn, ws.pos());
            }
            for w in [ws, wn] {
                let bit = xlat_bit(ctx.get_diff_routing_special(tcid, w, specials::PRESENT));
                ctx.insert_support(tcid, BTreeSet::from([w]), vec![bit]);
            }
        }
    }
}

fn collect_fuzzers_hclk_io(ctx: &mut CollectorCtx) {
    for tcid in [tcls::HCLK_IO_HP, tcls::HCLK_IO_HR] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for i in 0..4 {
            let bslot = bslots::BUFIO[i];
            ctx.collect_bel_attr(tcid, bslot, BUFIO::ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, BUFIO::DELAY_ENABLE);
            ctx.collect_mux(tcid, wires::IMUX_BUFIO[i].cell(4));
        }
        for i in 0..4 {
            let bslot = bslots::BUFR[i];
            ctx.collect_bel_attr(tcid, bslot, BUFR::ENABLE);
            ctx.collect_bel_attr(tcid, bslot, BUFR::DIVIDE);
        }
        {
            let bslot = bslots::IDELAYCTRL;
            ctx.collect_bel_attr_bi(tcid, bslot, IDELAYCTRL::HIGH_PERFORMANCE_MODE);
            ctx.collect_bel_attr(tcid, bslot, IDELAYCTRL::BIAS_MODE);
            let diff_full =
                ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_FULL);
            let diff_default =
                ctx.get_diff_bel_special(tcid, bslot, specials::IDELAYCTRL_IODELAY_DEFAULT_ONLY);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                IDELAYCTRL::DLL_ENABLE,
                xlat_bit(diff_full.combine(&!&diff_default)),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                IDELAYCTRL::DELAY_ENABLE,
                xlat_bit(diff_default),
            );
            let vctl_sel = vec![TileBit::new(0, 37, 23).pos(), TileBit::new(0, 37, 25).pos()];
            ctx.insert_bel_attr_bitvec(tcid, bslot, IDELAYCTRL::VCTL_SEL, vctl_sel);
        }
        {
            for i in 0..4 {
                ctx.collect_progbuf(
                    tcid,
                    wires::RCLK_IO[i].cell(4),
                    wires::RCLK_ROW[i].cell(4).pos(),
                );
            }
            let mut bits_hclk_buf = vec![];
            for i in 0..12 {
                let (_, _, diff) = Diff::split(
                    ctx.peek_diff_routing(
                        tcid,
                        wires::LCLK_IO[0].cell(3),
                        wires::HCLK_ROW[i].cell(4).pos(),
                    )
                    .clone(),
                    ctx.peek_diff_routing(
                        tcid,
                        wires::LCLK_IO[0].cell(4),
                        wires::HCLK_ROW[i].cell(4).pos(),
                    )
                    .clone(),
                );
                let bit = xlat_bit(diff);
                ctx.insert_progbuf(
                    tcid,
                    wires::HCLK_IO[i].cell(4),
                    wires::HCLK_ROW[i].cell(4).pos(),
                    bit,
                );
                bits_hclk_buf.push(bit);
            }
            for c in [3, 4] {
                for o in 0..6 {
                    let dst = wires::LCLK_IO[o].cell(c);
                    let mut diffs = vec![(None, Diff::default())];
                    for i in 0..12 {
                        let src = wires::HCLK_IO[i].cell(4);
                        let far_src = wires::HCLK_ROW[i].cell(4);
                        let mut diff = ctx.get_diff_routing(tcid, dst, far_src.pos());
                        diff.apply_bit_diff(bits_hclk_buf[i], true, false);
                        diffs.push((Some(src.pos()), diff));
                    }
                    ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, bali_only: bool) {
    if !bali_only {
        collect_fuzzers_hclk(ctx);
        collect_fuzzers_clk_bufg(ctx);
        collect_fuzzers_clk_hrow(ctx);
    }
    collect_fuzzers_clk_bufg_rebuf(ctx, bali_only);

    if !bali_only {
        collect_fuzzers_hclk_io(ctx);
    }
}
