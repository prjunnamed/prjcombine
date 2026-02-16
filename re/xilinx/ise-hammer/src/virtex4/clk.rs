use std::collections::{BTreeSet, btree_map};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, TileClassId, TileWireCoord, WireSlotIdExt},
    dir::{DirH, DirV},
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, xlat_bit, xlat_bit_wide, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls,
    virtex4::{bslots, tcls, tslots, wires},
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
            relation::{Delta, FixedRelation, Related, TileRelation},
        },
    },
    virtex4::specials,
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
struct ExtraHclkDcmSupport(DirV, TileClassId, TileWireCoord);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraHclkDcmSupport {
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
            let ntcrd = tcrd.with_row(row).tile(tslots::BEL);
            if let Some(tile) = backend.edev.get_tile(ntcrd)
                && tile.class == self.1
            {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingSpecial(self.1, self.2, specials::SUPPORT),
                    rects: edev.tile_bits(ntcrd),
                });
                sad = false;
            }
        }
        Some((fuzzer, sad))
    }
}

#[derive(Clone, Debug)]
struct ExtraMgtRepeaterAttr(DirH, TileWireCoord, SpecialId);

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
                    key: DiffKey::RoutingSpecial(tcls::HCLK_MGT_BUF, self.1, self.2),
                    rects: edev.tile_bits(ntcrd),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct MgtRepeater(DirH, DirV, TileWireCoord, SpecialId);

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
                let ntcrd = tcrd.with_cr(rcol, rrow).tile(tslots::CLK);
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::RoutingSpecial(tcls::HCLK_MGT_BUF, self.2, self.3),
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
        let tcid = tcls::CLK_BUFG;
        let tcls = &edev.db[tcid];
        let muxes = &backend.edev.db_index.tile_classes[tcid].muxes;

        let mut ctx = FuzzCtx::new(session, backend, tcid);

        for i in 0..32 {
            let bslot = bslots::BUFGCTRL[i];
            let mut bctx = ctx.bel(bslot);
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

            let BelInfo::Bel(ref bel) = tcls.bels[bslot] else {
                unreachable!()
            };
            let wire_i0 = bel.inputs[bcls::BUFGCTRL::I0].wire();
            let wire_i1 = bel.inputs[bcls::BUFGCTRL::I1].wire();
            let mux_i0 = &muxes[&wire_i0];
            let mux_i1 = &muxes[&wire_i1];

            for midx in 0..2 {
                let mux = [mux_i0, mux_i1][midx];

                for &src in mux.src.keys() {
                    let mut builder = bctx.build().mutex("IxMUX", mux.dst);

                    if let Some(idx) = wires::IMUX_BUFG_I.index_of(src.wire) {
                        let clk_iob = CellCoord::new(
                            DieId::from_idx(0),
                            edev.col_clk,
                            if src.cell.to_idx() == 0 {
                                edev.row_dcmiob.unwrap()
                            } else {
                                edev.row_iobdcm.unwrap() - 16
                            },
                        )
                        .tile(tslots::CLK);

                        builder = builder.global_mutex("CLK_IOB_MUXBUS", "USE").related_pip(
                            FixedRelation(clk_iob),
                            wires::IMUX_BUFG_O[idx].cell(0),
                            wires::OUT_CLKPAD.cell(0),
                        )
                    } else if wires::MGT_ROW.contains(src.wire) {
                        let obel_bufg = bslots::BUFGCTRL[i ^ 1];
                        let idx = wires::IMUX_SPEC.index_of(mux.dst.wire).unwrap();
                        let odst = TileWireCoord {
                            wire: wires::IMUX_SPEC[idx ^ 2],
                            cell: mux.dst.cell,
                        };
                        builder = builder
                            .global_mutex("BUFG_MGTCLK", "USE")
                            .bel_mutex(obel_bufg, "IxMUX", odst)
                            .prop(BaseIntPip::new(odst, src.tw))
                            .prop(WireMutexExclusive::new(odst));
                    }

                    builder
                        .test_routing(mux.dst, src)
                        .prop(FuzzIntPip::new(mux.dst, src.tw))
                        .prop(WireMutexExclusive::new(mux.dst))
                        .prop(WireMutexShared::new(src.tw))
                        .commit();
                }
            }
            bctx.mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .test_bel_special(specials::CLK_BUFG_O)
                .pin("O")
                .commit();
            bctx.mode(mode)
                .null_bits()
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .pin("O")
                .test_bel_special(specials::BUFGCTRL_PIN_O_GFB)
                .pip("GFB", "O")
                .commit();
            let mut builder = bctx
                .mode(mode)
                .null_bits()
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .global_mutex("BUFGCTRL_O_GCLK", format!("BUFGCTRL{i}"))
                .pin("O");
            if !matches!(i, 19 | 30) {
                builder = builder.extra_tiles_by_class_bel_special(
                    tcls::CLK_TERM,
                    bslots::HROW_INT,
                    specials::CLK_GCLK_TERM,
                );
            }
            builder
                .test_bel_special(specials::BUFGCTRL_PIN_O_GCLK)
                .pip("GCLK", "O")
                .commit();
        }
        if edev.col_lgt.is_some() {
            let mut bctx = ctx.bel(bslots::SPEC_INT);
            for (c, which, h, v) in [
                (0, "SW", DirH::W, DirV::S),
                (8, "NW", DirH::W, DirV::N),
                (16, "SE", DirH::E, DirV::S),
                (17, "NE", DirH::E, DirV::N),
            ] {
                for i in 0..2 {
                    bctx.build()
                        .global_mutex("BUFG_MGTCLK", "TEST")
                        .test_routing_special(wires::MGT_ROW[i].cell(c), specials::SUPPORT)
                        .pip(
                            format!("MGT_{which}{i}_HROW_O"),
                            format!("MGT_{which}{i}_HROW_I"),
                        )
                        .commit();
                    bctx.build()
                        .global_mutex("MGT_OUT", "USE")
                        .null_bits()
                        .prop(MgtRepeater(
                            h,
                            v,
                            wires::MGT_ROW[i].cell(0),
                            specials::MGT_BUF_BUFG,
                        ))
                        .test_routing_special(wires::MGT_ROW[i].cell(c), specials::MGT_BUF_BUFG)
                        .pip(
                            format!("MGT_{which}{i}_HCLK_O"),
                            format!("MGT_{which}{i}_HCLK_I"),
                        )
                        .commit();
                }
            }
        }
    }

    for tcid in [tcls::CLK_IOB_S, tcls::CLK_IOB_N] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        for i in 0..16 {
            let wt = wires::GIOB[i].cell(0);
            let wf = wires::OUT_CLKPAD.cell(i);
            bctx.build()
                .global_mutex("GIOB", "TEST")
                .tile_mutex_exclusive("GIOB_TEST")
                .extra_tile_bel_special(ClkTerm, bslots::HROW_INT, specials::CLK_GIOB_TERM)
                .test_routing(wt, wf.pos())
                .pip(wt, wf)
                .commit();
        }
        let clk_dcm = match tcid {
            tcls::CLK_IOB_S => Delta::new(0, -8, tcls::CLK_DCM_S),
            tcls::CLK_IOB_N => Delta::new(0, 16, tcls::CLK_DCM_N),
            _ => unreachable!(),
        };
        for i in 0..32 {
            let mout = wires::IMUX_BUFG_O[i].cell(0);
            let min = wires::IMUX_BUFG_I[i].cell(0);
            for j in 0..16 {
                let giob = wires::OUT_CLKPAD.cell(j);
                bctx.build()
                    .global_mutex("CLK_IOB_MUXBUS", "TEST")
                    .test_routing(mout, giob.pos())
                    .prop(FuzzIntPip::new(mout, giob))
                    .prop(WireMutexExclusive::new(mout))
                    .prop(WireMutexShared::new(giob))
                    .commit();
            }
            bctx.build()
                .global_mutex("CLK_IOB_MUXBUS", "TEST")
                .related_pip(clk_dcm.clone(), mout, wires::OUT_DCM[0].cell(0))
                .related_tile_mutex(clk_dcm.clone(), "MUXBUS", "USE")
                .test_routing(mout, min.pos())
                .prop(FuzzIntPip::new(mout, min))
                .prop(WireMutexExclusive::new(mout))
                .prop(WireMutexShared::new(min))
                .commit();
        }
    }
    for tcid in [tcls::CLK_DCM_S, tcls::CLK_DCM_N] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::SPEC_INT);
        let clk_dcm = match tcid {
            tcls::CLK_DCM_S => Delta::new(0, -8, tcls::CLK_DCM_S),
            tcls::CLK_DCM_N => Delta::new(0, 8, tcls::CLK_DCM_N),
            _ => unreachable!(),
        };
        for i in 0..32 {
            let mout = wires::IMUX_BUFG_O[i].cell(0);
            let min = wires::IMUX_BUFG_I[i].cell(0);
            for j in 0..24 {
                let dcm = wires::OUT_DCM[j % 12].cell(j / 12 * 4);
                bctx.build()
                    .tile_mutex("MUXBUS", "TEST")
                    .test_routing(mout, dcm.pos())
                    .prop(FuzzIntPip::new(mout, dcm))
                    .prop(WireMutexExclusive::new(mout))
                    .prop(WireMutexShared::new(dcm))
                    .commit();
            }
            let has_other = edev.tile_index[tcid].len() > 1;
            if has_other {
                bctx.build()
                    .tile_mutex("MUXBUS", "TEST")
                    .related_pip(clk_dcm.clone(), mout, wires::OUT_DCM[0].cell(0))
                    .related_tile_mutex(clk_dcm.clone(), "MUXBUS", "USE")
                    .test_routing(mout, min.pos())
                    .prop(FuzzIntPip::new(mout, min))
                    .prop(WireMutexExclusive::new(mout))
                    .prop(WireMutexShared::new(min))
                    .commit();
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CLK_HROW);
        let mut bctx = ctx.bel(bslots::HROW_INT);
        for co in 0..2 {
            for o in 0..8 {
                let wt = wires::HCLK_ROW[o].cell(co);
                for i in 0..32 {
                    let wf = wires::GCLK[i].cell(0);
                    let bufg = FixedRelation(edev.tile_cfg(DieId::from_idx(0)).tile(tslots::BEL));
                    bctx.build()
                        .global_mutex("BUFGCTRL_OUT", "USE")
                        .tile_mutex("MODE", "TEST")
                        .tile_mutex_exclusive("HROW")
                        .prop(Related::new(
                            bufg,
                            BaseIntPip::new(wires::GCLK[i].cell(8), wires::OUT_BUFG[i].cell(8)),
                        ))
                        .extra_tile_bel_special(
                            HclkTerm([DirH::W, DirH::E][co]),
                            bslots::HROW_INT,
                            specials::CLK_HCLK_TERM,
                        )
                        .test_routing(wt, wf.pos())
                        .pip(wt, wf)
                        .commit();
                }
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::HCLK);
        let mut bctx = ctx.bel(bslots::HCLK);
        for i in 0..8 {
            let wt = wires::HCLK[i].cell(0);
            let wf = wires::HCLK_ROW[i].cell(0);
            bctx.build()
                .global_mutex("BUFGCTRL_OUT", "USE")
                .related_tile_mutex(ClkHrow, "MODE", "USE")
                .related_pip(ClkHrow, wires::HCLK_ROW[i].cell(0), wires::GCLK[0].cell(0))
                .related_pip(ClkHrow, wires::HCLK_ROW[i].cell(1), wires::GCLK[0].cell(0))
                .test_routing(wt, wf.pos())
                .pip(wt, wf)
                .commit();
        }
        for i in 0..2 {
            let wt = wires::RCLK[i].cell(0);
            let wf = wires::RCLK_ROW[i].cell(0);
            bctx.build()
                .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                .related_pip(Rclk, wires::RCLK_ROW[i].cell(2), wires::VRCLK[0].cell(2))
                .test_routing(wt, wf.pos())
                .pip(wt, wf)
                .commit();
        }
    }
    for tcid in [
        tcls::HCLK_IO_LVDS,
        tcls::HCLK_IO_DCI,
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let tcls = &backend.edev.db[tcid];
        if tcls.bels.contains_id(bslots::BUFR[0]) {
            let mut bctx = ctx.bel(bslots::HCLK_IO_INT);
            for opin in wires::RCLK_ROW {
                for ipin in [
                    wires::VRCLK[0],
                    wires::VRCLK[1],
                    wires::VRCLK_S[0],
                    wires::VRCLK_S[1],
                    wires::VRCLK_N[0],
                    wires::VRCLK_N[1],
                ] {
                    let wt = opin.cell(2);
                    let wf = ipin.cell(2);
                    bctx.build()
                        .tile_mutex("RCLK_MODE", "TEST")
                        .test_routing(wt, wf.pos())
                        .pip(wt, wf)
                        .prop(WireMutexExclusive::new(wt))
                        .commit();
                }
            }
            for bel in [bslots::BUFR[0], bslots::BUFR[1]] {
                let mut bctx = ctx.bel(bel);
                bctx.build()
                    .null_bits()
                    .test_bel_special(specials::PRESENT)
                    .mode("BUFR")
                    .commit();
                bctx.mode("BUFR")
                    .test_bel_attr_bits(bcls::BUFR::ENABLE)
                    .pin("O")
                    .commit();
                bctx.mode("BUFR")
                    .test_bel_attr_rename("BUFR_DIVIDE", bcls::BUFR::DIVIDE);
            }
        }
        {
            let mut bctx = ctx.bel(bslots::HCLK_IO_INT);
            for i in 0..8 {
                let wt = wires::HCLK_IO[i].cell(2);
                let wf = wires::HCLK_ROW[i].cell(2);
                bctx.build()
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow, "MODE", "USE")
                    .related_pip(ClkHrow, wires::HCLK_ROW[i].cell(0), wires::GCLK[0].cell(0))
                    .related_pip(ClkHrow, wires::HCLK_ROW[i].cell(1), wires::GCLK[0].cell(0))
                    .test_routing(wt, wf.pos())
                    .pip(wt, wf)
                    .commit();
            }
            for i in 0..2 {
                let wt = wires::RCLK_IO[i].cell(2);
                let wf = wires::RCLK_ROW[i].cell(2);
                bctx.build()
                    .related_tile_mutex(Rclk, "RCLK_MODE", "USE")
                    .related_pip(Rclk, wires::RCLK_ROW[i].cell(2), wires::VRCLK[0].cell(2))
                    .test_routing(wt, wf.pos())
                    .pip(wt, wf)
                    .commit();
            }

            for i in 0..2 {
                let wt = wires::IOCLK[i].cell(2);
                let wf = wires::OUT_CLKPAD.cell(2 - i);
                bctx.build()
                    .tile_mutex_exclusive("IOCLK")
                    .prop(WireMutexExclusive::new(wt))
                    .test_routing(wt, wf.pos())
                    .pip(wt, wf)
                    .commit();
            }
            let (has_s, has_n) = match tcid {
                tcls::HCLK_IO_DCI | tcls::HCLK_IO_LVDS => (true, true),
                tcls::HCLK_IO_DCM_N | tcls::HCLK_IO_CFG_N => (false, true),
                tcls::HCLK_IO_DCM_S => (true, false),
                tcls::HCLK_IO_CENTER => (
                    true,
                    edev.chips[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                for (o, i, oi, oo) in [
                    (
                        wires::IOCLK_N_IO[0].cell(2),
                        wires::IOCLK_N[0].cell(2),
                        "VIOCLK0",
                        "IOCLK0",
                    ),
                    (
                        wires::IOCLK_N_IO[1].cell(2),
                        wires::IOCLK_N[1].cell(2),
                        "VIOCLK1",
                        "IOCLK1",
                    ),
                ] {
                    bctx.build()
                        .prop(WireMutexShared::new(i))
                        .related_pip(Ioclk(DirV::S), oo, oi)
                        .related_tile_mutex(Ioclk(DirV::S), "IOCLK", "USE")
                        .test_routing(o, i.pos())
                        .pip(o, i)
                        .commit();
                }
            }
            if has_n {
                for (o, i, oi, oo) in [
                    (
                        wires::IOCLK_S_IO[0].cell(2),
                        wires::IOCLK_S[0].cell(2),
                        "VIOCLK0",
                        "IOCLK0",
                    ),
                    (
                        wires::IOCLK_S_IO[1].cell(2),
                        wires::IOCLK_S[1].cell(2),
                        "VIOCLK1",
                        "IOCLK1",
                    ),
                ] {
                    bctx.build()
                        .prop(WireMutexShared::new(i))
                        .related_pip(Ioclk(DirV::N), oo, oi)
                        .related_tile_mutex(Ioclk(DirV::N), "IOCLK", "USE")
                        .test_routing(o, i.pos())
                        .pip(o, i)
                        .commit();
                }
            }
        }
        {
            let mut bctx = ctx.bel(bslots::IDELAYCTRL);
            bctx.build()
                .test_bel_attr_bits(bcls::IDELAYCTRL::DLL_ENABLE)
                .mode("IDELAYCTRL")
                .commit();
        }
    }

    let num_ccms = backend.edev.tile_index[tcls::CCM].len();
    let has_gt = edev.col_lgt.is_some();
    for tcid in [tcls::HCLK_DCM, tcls::HCLK_IO_DCM_N, tcls::HCLK_IO_DCM_S] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::HCLK_IO_INT);
        let rel_dcm_ccm = |dir| {
            let dy = match dir {
                DirV::N => 0,
                DirV::S => -8,
            };
            Delta::new_any(0, dy, &[tcls::CCM, tcls::DCM])
        };
        for dir in [DirV::S, DirV::N] {
            if dir == DirV::S && tcid == tcls::HCLK_IO_DCM_S {
                continue;
            }
            if dir == DirV::N && tcid == tcls::HCLK_IO_DCM_N {
                continue;
            }
            let oc = match dir {
                DirV::S => 1,
                DirV::N => 2,
            };
            for i in 0..16 {
                let wt = wires::GIOB_DCM[i].cell(oc);
                let wf = wires::GIOB[i].cell(2);
                let mut builder = bctx
                    .build()
                    .global_mutex("HCLK_DCM", "TEST")
                    .tile_mutex("HCLK_DCM", wt);
                if tcid == tcls::HCLK_DCM || num_ccms < 4 {
                    builder = builder.prop(ExtraHclkDcmSupport(dir, tcls::DCM, wt.wire.cell(0)));
                }
                if tcid != tcls::HCLK_DCM && num_ccms != 0 {
                    builder = builder.prop(ExtraHclkDcmSupport(dir, tcls::CCM, wt.wire.cell(0)));
                }
                builder
                    .test_routing(wt, wf.pos())
                    .prop(FuzzIntPip::new(wt, wf))
                    .commit();
            }
            for i in 0..8 {
                let wt = wires::HCLK_DCM[i].cell(oc);
                let wf = wires::HCLK_ROW[i].cell(2);
                let mut builder = bctx
                    .build()
                    .global_mutex("HCLK_DCM", "TEST")
                    .tile_mutex("HCLK_DCM", wt)
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow, "MODE", "USE")
                    .related_pip(ClkHrow, wires::HCLK_ROW[i].cell(0), wires::GCLK[0].cell(0))
                    .has_related(rel_dcm_ccm(dir));
                if tcid == tcls::HCLK_DCM || num_ccms < 4 {
                    builder = builder.prop(ExtraHclkDcmSupport(dir, tcls::DCM, wt.wire.cell(0)));
                }
                if tcid != tcls::HCLK_DCM && num_ccms != 0 {
                    builder = builder.prop(ExtraHclkDcmSupport(dir, tcls::CCM, wt.wire.cell(0)));
                }
                builder
                    .test_routing(wt, wf.pos())
                    .prop(FuzzIntPip::new(wt, wf))
                    .commit();
            }
            if has_gt || tcid == tcls::HCLK_DCM {
                for i in 0..4 {
                    let wt = wires::MGT_DCM[i].cell(oc);
                    let wf = wires::MGT_ROW[i % 2].cell(2 + i / 2 * 2);
                    if tcid == tcls::HCLK_DCM {
                        let mut builder = bctx
                            .build()
                            .global_mutex("HCLK_DCM", "TEST")
                            .tile_mutex("HCLK_DCM", wt)
                            .has_related(rel_dcm_ccm(DirV::S))
                            .has_related(rel_dcm_ccm(DirV::N));
                        if tcid == tcls::HCLK_DCM || num_ccms < 4 {
                            builder =
                                builder.prop(ExtraHclkDcmSupport(dir, tcls::DCM, wt.wire.cell(0)));
                        }
                        if tcid != tcls::HCLK_DCM && num_ccms != 0 {
                            builder =
                                builder.prop(ExtraHclkDcmSupport(dir, tcls::CCM, wt.wire.cell(0)));
                        }
                        builder
                            .test_routing(wt, wf.pos())
                            .prop(FuzzIntPip::new(wt, wf))
                            .commit();
                    } else {
                        let mut builder = bctx
                            .build()
                            .global_mutex("MGT_OUT", "USE")
                            .global_mutex("HCLK_DCM", "TEST")
                            .tile_mutex("HCLK_DCM", wt)
                            .prop(ExtraMgtRepeaterAttr(
                                if i < 2 { DirH::W } else { DirH::E },
                                wires::MGT_ROW[i % 2].cell(0),
                                specials::MGT_BUF_DCM,
                            ));
                        if tcid == tcls::HCLK_DCM || num_ccms < 4 {
                            builder =
                                builder.prop(ExtraHclkDcmSupport(dir, tcls::DCM, wt.wire.cell(0)));
                        }
                        if tcid != tcls::HCLK_DCM && num_ccms != 0 {
                            builder =
                                builder.prop(ExtraHclkDcmSupport(dir, tcls::CCM, wt.wire.cell(0)));
                        }
                        builder
                            .test_routing(wt, wf.pos())
                            .prop(FuzzIntPip::new(wt, wf))
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
        let tcid = tcls::CLK_BUFG;
        let tcls = &edev.db[tcid];
        let muxes = &ctx.edev.db_index.tile_classes[tcid].muxes;
        for i in 0..32 {
            let bslot = bslots::BUFGCTRL[i];
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

            let BelInfo::Bel(ref bel) = tcls.bels[bslot] else {
                unreachable!()
            };
            let wire_i0 = bel.inputs[bcls::BUFGCTRL::I0].wire();
            let wire_i1 = bel.inputs[bcls::BUFGCTRL::I1].wire();
            let mux_i0 = &muxes[&wire_i0];
            let mux_i1 = &muxes[&wire_i1];

            let (_, _, ien_diff) = Diff::split(
                ctx.peek_diff_routing(tcid, wire_i0, wires::OUT_BUFG[i].cell(8).pos())
                    .clone(),
                ctx.peek_diff_routing(tcid, wire_i1, wires::OUT_BUFG[i].cell(8).pos())
                    .clone(),
            );
            let ien_bit = xlat_bit(ien_diff);
            for mux in [mux_i0, mux_i1] {
                let mut diffs = vec![(None, Diff::default())];
                for &src in mux.src.keys() {
                    let mut diff = ctx.get_diff_routing(tcid, mux.dst, src);
                    diff.apply_bit_diff(ien_bit, true, false);
                    diffs.push((Some(src), diff));
                }
                ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
            }
            ctx.insert_support(tcid, BTreeSet::from_iter([wire_i0, wire_i1]), vec![ien_bit]);

            let bits = xlat_bit_wide(ctx.get_diff_bel_special(tcid, bslot, specials::CLK_BUFG_O));
            ctx.insert_support(
                tcid,
                BTreeSet::from_iter([wires::OUT_BUFG[i].cell(8)]),
                bits,
            );
        }

        for c in [0, 8, 16, 17] {
            for i in 0..2 {
                let wire = wires::MGT_ROW[i].cell(c);
                let bit = xlat_bit(ctx.get_diff_routing_special(tcid, wire, specials::SUPPORT));
                ctx.insert_support(tcid, BTreeSet::from_iter([wire]), vec![bit]);
            }
        }
    }

    if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tcid = tcls::HCLK_MGT_BUF;
        for i in 0..2 {
            let wire = wires::MGT_ROW[i].cell(0);
            let diff = ctx.get_diff_routing_special(tcid, wire, specials::MGT_BUF_BUFG);
            ctx.insert_support(tcid, BTreeSet::from_iter([wire]), xlat_bit_wide(diff));
        }
    }

    {
        let tcid = tcls::CLK_TERM;
        let bits = xlat_bit_wide(ctx.get_diff_bel_special(
            tcid,
            bslots::HROW_INT,
            specials::CLK_GIOB_TERM,
        ));
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(wires::GIOB.into_iter().map(|w| w.cell(0))),
            bits,
        );
        let bits = xlat_bit_wide(ctx.get_diff_bel_special(
            tcid,
            bslots::HROW_INT,
            specials::CLK_GCLK_TERM,
        ));
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(wires::GCLK.into_iter().map(|w| w.cell(0))),
            bits,
        );
    }

    for tcid in [tcls::CLK_IOB_S, tcls::CLK_IOB_N] {
        for i in 0..16 {
            ctx.collect_mux(tcid, wires::GIOB[i].cell(0));
        }
    }

    // hack for small devices (without chain pip in CLK_DCM_*)
    for tcid in [tcls::CLK_DCM_S, tcls::CLK_DCM_N] {
        for i in 0..32 {
            let diff11 = ctx.peek_diff_routing(
                tcid,
                wires::IMUX_BUFG_O[i].cell(0),
                wires::OUT_DCM[11].cell(0).pos(),
            );
            let diff12 = ctx.peek_diff_routing(
                tcid,
                wires::IMUX_BUFG_O[i].cell(0),
                wires::OUT_DCM[0].cell(4).pos(),
            );
            let diff18 = ctx.peek_diff_routing(
                tcid,
                wires::IMUX_BUFG_O[i].cell(0),
                wires::OUT_DCM[6].cell(4).pos(),
            );
            let (_, _, mut diff) = Diff::split(diff11.clone(), diff12.clone());
            let (_, diff1, _) = Diff::split(diff12.clone(), diff18.clone());
            assert_eq!(diff1.bits.len(), 1);
            for (bit, val) in diff1.bits {
                if tcid == tcls::CLK_DCM_S {
                    diff.bits.insert(
                        TileBit {
                            bit: bit.bit - 3,
                            ..bit
                        },
                        val,
                    );
                    diff.bits.insert(
                        TileBit {
                            bit: bit.bit - 4,
                            ..bit
                        },
                        val,
                    );
                } else {
                    diff.bits.insert(
                        TileBit {
                            bit: bit.bit + 3,
                            ..bit
                        },
                        val,
                    );
                    diff.bits.insert(
                        TileBit {
                            bit: bit.bit + 4,
                            ..bit
                        },
                        val,
                    );
                }
            }
            match ctx.diffs.entry(DiffKey::Routing(
                tcid,
                wires::IMUX_BUFG_O[i].cell(0),
                wires::IMUX_BUFG_I[i].cell(0).pos(),
            )) {
                btree_map::Entry::Vacant(e) => {
                    e.insert(vec![diff]);
                }
                btree_map::Entry::Occupied(e) => {
                    assert_eq!(*e.get(), vec![diff]);
                }
            }
        }
    }
    for tcid in [
        tcls::CLK_IOB_S,
        tcls::CLK_IOB_N,
        tcls::CLK_DCM_S,
        tcls::CLK_DCM_N,
    ] {
        for w in wires::IMUX_BUFG_O {
            ctx.collect_mux(tcid, w.cell(0));
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
            for o in 0..8 {
                let wt = wires::HCLK_ROW[o].cell(co);
                let mut inps = vec![(None, Diff::default())];
                for i in 0..32 {
                    let wf = wires::GCLK_BUF[i].cell(0);
                    let fwf = wires::GCLK[i].cell(0);
                    let mut diff = ctx.get_diff_routing(tcid, wt, fwf.pos());
                    diff = diff.combine(&!&inp_diffs[i]);
                    inps.push((Some(wf.pos()), diff));
                }
                ctx.insert_mux(tcid, wt, xlat_enum_raw(inps, OcdMode::Mux));
            }
        }
        for (i, diff) in inp_diffs.into_iter().enumerate() {
            ctx.insert_progbuf(
                tcid,
                wires::GCLK_BUF[i].cell(0),
                wires::GCLK[i].cell(0).pos(),
                xlat_bit(diff),
            );
        }
    }
    {
        let tcid = tcls::HCLK_TERM;
        let bits = xlat_bit_wide(ctx.get_diff_bel_special(
            tcid,
            bslots::HROW_INT,
            specials::CLK_HCLK_TERM,
        ));
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(wires::HCLK_ROW.into_iter().map(|w| w.cell(0))),
            bits,
        );
    }
    {
        let tcid = tcls::HCLK;
        for i in 0..8 {
            ctx.collect_progbuf(
                tcid,
                wires::HCLK[i].cell(0),
                wires::HCLK_ROW[i].cell(0).pos(),
            );
        }
        for i in 0..2 {
            ctx.collect_progbuf(
                tcid,
                wires::RCLK[i].cell(0),
                wires::RCLK_ROW[i].cell(0).pos(),
            );
        }
    }
    for tcid in [
        tcls::HCLK_IO_LVDS,
        tcls::HCLK_IO_DCI,
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
    ] {
        for i in 0..8 {
            ctx.collect_progbuf(
                tcid,
                wires::HCLK_IO[i].cell(2),
                wires::HCLK_ROW[i].cell(2).pos(),
            );
        }
        for i in 0..2 {
            ctx.collect_progbuf(
                tcid,
                wires::RCLK_IO[i].cell(2),
                wires::RCLK_ROW[i].cell(2).pos(),
            );
        }
        let tcls = &ctx.edev.db[tcid];
        if tcls.bels.contains_id(bslots::BUFR[0]) {
            for w in wires::RCLK_ROW {
                ctx.collect_mux(tcid, w.cell(2));
            }
            for i in 0..2 {
                let bslot = bslots::BUFR[i];
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFR::ENABLE);
                ctx.collect_bel_attr(tcid, bslot, bcls::BUFR::DIVIDE);
            }
        }
        {
            let diff0 = ctx.get_diff_routing(
                tcid,
                wires::IOCLK[0].cell(2),
                wires::OUT_CLKPAD.cell(2).pos(),
            );
            let diff1 = ctx.get_diff_routing(
                tcid,
                wires::IOCLK[1].cell(2),
                wires::OUT_CLKPAD.cell(1).pos(),
            );
            let (diff0, diff1, diffc) = Diff::split(diff0, diff1);
            ctx.insert_bel_attr_bool(tcid, bslots::BUFIO[0], bcls::BUFIO::ENABLE, xlat_bit(diff0));
            ctx.insert_bel_attr_bool(tcid, bslots::BUFIO[1], bcls::BUFIO::ENABLE, xlat_bit(diff1));
            ctx.insert_support(
                tcid,
                BTreeSet::from_iter(wires::IOCLK.into_iter().map(|w| w.cell(2))),
                xlat_bit_wide(diffc),
            );
            let (has_s, has_n) = match tcid {
                tcls::HCLK_IO_DCI | tcls::HCLK_IO_LVDS => (true, true),
                tcls::HCLK_IO_DCM_N | tcls::HCLK_IO_CFG_N => (false, true),
                tcls::HCLK_IO_DCM_S => (true, false),
                tcls::HCLK_IO_CENTER => (
                    true,
                    edev.chips[DieId::from_idx(0)].row_bufg() - edev.row_dcmiob.unwrap() > 24,
                ),
                _ => unreachable!(),
            };
            if has_s {
                ctx.collect_progbuf(
                    tcid,
                    wires::IOCLK_N_IO[0].cell(2),
                    wires::IOCLK_N[0].cell(2).pos(),
                );
                ctx.collect_progbuf(
                    tcid,
                    wires::IOCLK_N_IO[1].cell(2),
                    wires::IOCLK_N[1].cell(2).pos(),
                );
            }
            if has_n {
                ctx.collect_progbuf(
                    tcid,
                    wires::IOCLK_S_IO[0].cell(2),
                    wires::IOCLK_S[0].cell(2).pos(),
                );
                ctx.collect_progbuf(
                    tcid,
                    wires::IOCLK_S_IO[1].cell(2),
                    wires::IOCLK_S[1].cell(2).pos(),
                );
            }
        }
        {
            let bslot = bslots::IDELAYCTRL;
            ctx.collect_bel_attr(tcid, bslot, bcls::IDELAYCTRL::DLL_ENABLE);
        }
    }
    for tcid in [
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
    ] {
        for (wt, wf) in [
            (
                wires::IOCLK_S_IO[0].cell(2),
                wires::IOCLK_S[0].cell(2).pos(),
            ),
            (
                wires::IOCLK_S_IO[1].cell(2),
                wires::IOCLK_S[1].cell(2).pos(),
            ),
            (
                wires::IOCLK_N_IO[0].cell(2),
                wires::IOCLK_N[0].cell(2).pos(),
            ),
            (
                wires::IOCLK_N_IO[1].cell(2),
                wires::IOCLK_N[1].cell(2).pos(),
            ),
        ] {
            let bit = ctx.sb_progbuf(tcls::HCLK_IO_LVDS, wt, wf);
            ctx.insert_progbuf(tcid, wt, wf, bit);
        }
    }
    {
        let tcid = tcls::HCLK_DCM;
        let (_, _, common) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::MGT_DCM[0].cell(1),
                wires::MGT_ROW[0].cell(2).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::GIOB_DCM[0].cell(1),
                wires::GIOB[0].cell(2).pos(),
            )
            .clone(),
        );
        let (_, _, hclk_giob) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::HCLK_DCM[0].cell(1),
                wires::HCLK_ROW[0].cell(2).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::GIOB_DCM[0].cell(1),
                wires::GIOB[0].cell(2).pos(),
            )
            .clone(),
        );
        let (_, _, common_mgt) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::MGT_DCM[0].cell(1),
                wires::MGT_ROW[0].cell(2).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::MGT_DCM[1].cell(1),
                wires::MGT_ROW[1].cell(2).pos(),
            )
            .clone(),
        );
        for oc in [1, 2] {
            for i in 0..8 {
                let wt = wires::HCLK_DCM[i].cell(oc);
                let wf = wires::HCLK_ROW[i].cell(2).pos();
                let diff = ctx.get_diff_routing(tcid, wt, wf);
                let diff = diff.combine(&!&hclk_giob);
                ctx.insert_progbuf(tcid, wt, wf, xlat_bit(diff));
            }
            for i in 0..16 {
                let wt = wires::GIOB_DCM[i].cell(oc);
                let wf = wires::GIOB[i].cell(2).pos();
                let diff = ctx.get_diff_routing(tcid, wt, wf);
                let diff = diff.combine(&!&hclk_giob);
                ctx.insert_progbuf(tcid, wt, wf, xlat_bit(diff));
            }
            for i in 0..4 {
                let wt = wires::MGT_DCM[i].cell(oc);
                let wf = wires::MGT_ROW[i % 2].cell(2 + i / 2 * 2).pos();
                let diff = ctx.get_diff_routing(tcid, wt, wf);
                let diff = diff.combine(&!&common_mgt);
                ctx.insert_progbuf(tcid, wt, wf, xlat_bit(diff));
            }
        }
        let hclk_giob = hclk_giob.combine(&!&common);
        let common_mgt = common_mgt.combine(&!&common);
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(
                wires::HCLK_DCM
                    .into_iter()
                    .chain(wires::GIOB_DCM)
                    .chain(wires::MGT_DCM)
                    .flat_map(|w| [w.cell(1), w.cell(2)]),
            ),
            xlat_bit_wide(common),
        );
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(
                wires::HCLK_DCM
                    .into_iter()
                    .chain(wires::GIOB_DCM)
                    .flat_map(|w| [w.cell(1), w.cell(2)]),
            ),
            xlat_bit_wide(hclk_giob),
        );
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(
                wires::MGT_DCM
                    .into_iter()
                    .flat_map(|w| [w.cell(1), w.cell(2)]),
            ),
            xlat_bit_wide(common_mgt),
        );
    }
    for (tcid, oc) in [(tcls::HCLK_IO_DCM_N, 1), (tcls::HCLK_IO_DCM_S, 2)] {
        let (_, _, common) = Diff::split(
            ctx.peek_diff_routing(
                tcid,
                wires::HCLK_DCM[0].cell(oc),
                wires::HCLK_ROW[0].cell(2).pos(),
            )
            .clone(),
            ctx.peek_diff_routing(
                tcid,
                wires::GIOB_DCM[0].cell(oc),
                wires::GIOB[0].cell(2).pos(),
            )
            .clone(),
        );

        for i in 0..8 {
            let wt = wires::HCLK_DCM[i].cell(oc);
            let wf = wires::HCLK_ROW[i].cell(2).pos();
            let diff = ctx.get_diff_routing(tcid, wt, wf);
            let diff = diff.combine(&!&common);
            ctx.insert_progbuf(tcid, wt, wf, xlat_bit(diff));
        }
        for i in 0..16 {
            let wt = wires::GIOB_DCM[i].cell(oc);
            let wf = wires::GIOB[i].cell(2).pos();
            let diff = ctx.get_diff_routing(tcid, wt, wf);
            let diff = diff.combine(&!&common);
            ctx.insert_progbuf(tcid, wt, wf, xlat_bit(diff));
        }
        if edev.col_lgt.is_some() {
            let (_, _, common_mgt) = Diff::split(
                ctx.peek_diff_routing(
                    tcid,
                    wires::MGT_DCM[0].cell(oc),
                    wires::MGT_ROW[0].cell(2).pos(),
                )
                .clone(),
                ctx.peek_diff_routing(
                    tcid,
                    wires::MGT_DCM[1].cell(oc),
                    wires::MGT_ROW[1].cell(2).pos(),
                )
                .clone(),
            );

            for i in 0..4 {
                let wt = wires::MGT_DCM[i].cell(oc);
                let wf = wires::MGT_ROW[i % 2].cell(2 + i / 2 * 2).pos();
                let diff = ctx.get_diff_routing(tcid, wt, wf);
                let diff = diff.combine(&!&common_mgt);
                ctx.insert_progbuf(tcid, wt, wf, xlat_bit(diff));
            }

            let common_mgt = common_mgt.combine(&!&common);
            ctx.insert_support(
                tcid,
                BTreeSet::from_iter(wires::MGT_DCM.into_iter().map(|w| w.cell(oc))),
                xlat_bit_wide(common_mgt),
            );
        }
        ctx.insert_support(
            tcid,
            BTreeSet::from_iter(
                wires::HCLK_DCM
                    .into_iter()
                    .chain(wires::GIOB_DCM)
                    .chain(wires::MGT_DCM)
                    .map(|w| w.cell(oc)),
            ),
            xlat_bit_wide(common),
        );
    }

    for tcid in [tcls::DCM, tcls::CCM] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for w in wires::HCLK_DCM
            .into_iter()
            .chain(wires::GIOB_DCM)
            .chain(wires::MGT_DCM)
        {
            if tcid == tcls::CCM && wires::MGT_DCM.contains(w) && !edev.col_lgt.is_some() {
                continue;
            }
            let w = w.cell(0);
            let diff = ctx.get_diff_routing_special(tcid, w, specials::SUPPORT);
            ctx.insert_support(tcid, BTreeSet::from_iter([w]), vec![xlat_bit(diff)]);
        }
    }

    if edev.col_lgt.is_some() && !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tcid = tcls::HCLK_MGT_BUF;
        for i in 0..2 {
            let wire = wires::MGT_ROW[i].cell(0);
            let diff = ctx.get_diff_routing_special(tcid, wire, specials::MGT_BUF_DCM);
            ctx.insert_support(tcid, BTreeSet::from_iter([wire]), xlat_bit_wide(diff));
        }
    }
}
