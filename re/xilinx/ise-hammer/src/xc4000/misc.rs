use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::BelSlotId,
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_fpga_hammer::{
    backend::{FuzzerFeature, FuzzerProp},
    diff::{DiffKey, xlat_bit_raw},
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_xc2000::{
    chip::ChipKind,
    xc4000::{bslots, enums, tslots, xc4000::bcls, xc4000::tcls},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    xc4000::specials,
};

#[derive(Clone, Debug)]
struct ExtraTilesIoW;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesIoW {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let DiffKey::GlobalSpecial(spec) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        for (tile_class, locs) in &backend.edev.tile_index {
            if !backend.edev.db[tile_class].bels.contains_id(bslots::MISC_W) {
                continue;
            }
            for &tcrd in locs {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::BelAttrValue(
                        tile_class,
                        bslots::MISC_W,
                        bcls::MISC_W::PUMP,
                        match spec {
                            specials::PUMP_EXTERNAL => enums::PUMP::EXTERNAL,
                            specials::PUMP_INTERNAL => enums::PUMP::INTERNAL,
                            _ => unreachable!(),
                        },
                    ),
                    rects: EntityVec::from_iter(backend.edev.tile_bits(tcrd).into_values().take(1)),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ExtraTilesAllIo;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesAllIo {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let DiffKey::GlobalSpecial(spec) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        for (tile_class, locs) in &backend.edev.tile_index {
            if !backend.edev.db[tile_class].bels.contains_id(bslots::IO[0]) {
                continue;
            }
            for &tcrd in locs {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::BelSpecial(tile_class, bslots::IO[0], spec),
                    rects: EntityVec::from_iter(backend.edev.tile_bits(tcrd).into_values().take(1)),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTileSingle {
    pub tcrd: TileCoord,
    pub bel: BelSlotId,
}

impl ExtraTileSingle {
    pub fn new(tcrd: TileCoord, bel: BelSlotId) -> Self {
        Self { tcrd, bel }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTileSingle {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let DiffKey::GlobalSpecial(spec) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::BelSpecial(backend.edev[self.tcrd].class, self.bel, spec),
            rects: EntityVec::from_iter(backend.edev.tile_bits(self.tcrd).into_values().take(1)),
        });
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_SW);
        let mut bctx = ctx.bel(bslots::RDBK);
        bctx.build().test_global_attr_bool_rename(
            "READABORT",
            bcls::RDBK::READ_ABORT,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "READCAPTURE",
            bcls::RDBK::READ_CAPTURE,
            "DISABLE",
            "ENABLE",
        );
        for (opt, bslot, attr) in [
            ("M0PIN", bslots::MD0, bcls::IBUF::PULL),
            ("M1PIN", bslots::MD1, bcls::MD1::PULL),
            (
                if edev.chip.kind == ChipKind::SpartanXl {
                    "POWERDOWN"
                } else {
                    "M2PIN"
                },
                bslots::MD2,
                bcls::IBUF::PULL,
            ),
        ] {
            let mut bctx = ctx.bel(bslot);
            for (val, vname) in [
                (enums::IO_PULL::PULLUP, "PULLUP"),
                (enums::IO_PULL::PULLDOWN, "PULLDOWN"),
                (enums::IO_PULL::NONE, "PULLNONE"),
            ] {
                bctx.build()
                    .test_bel_attr_val(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }

        let mut bctx = ctx.bel(bslots::MISC_SW);
        bctx.build()
            .test_global_attr_bool_rename("TMBOT", bcls::MISC_SW::TM_BOT, "OFF", "ON");
        if matches!(edev.chip.kind, ChipKind::Xc4000Xla | ChipKind::Xc4000Xv) {
            bctx.build().test_global_attr_bool_rename(
                "5V_TOLERANT_IO",
                bcls::MISC_SW::IO_5V_TOLERANT,
                "OFF",
                "ON",
            );
        }
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_SE);
        let mut bctx = ctx.bel(bslots::MISC_SE);
        bctx.build()
            .test_global_attr_bool_rename("TCTEST", bcls::MISC_SE::TCTEST, "OFF", "ON");
        bctx.build().test_global_attr_bool_rename(
            "DONEPIN",
            bcls::MISC_SE::DONE_PULLUP,
            "PULLNONE",
            "PULLUP",
        );
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
        ) {
            bctx.build().test_global_attr_bool_rename(
                "FIXDISCHARGE",
                bcls::MISC_SE::FIX_DISCHARGE,
                "OFF",
                "ON",
            );
        }
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            bctx.build()
                .test_global_attr_bool_rename("TMOSC", bcls::MISC_SE::TM_OSC, "OFF", "ON");
            bctx.build()
                .test_global_attr_rename("OSCCLK", bcls::MISC_SE::OSC_CLK);
        }
        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.mode("STARTUP")
            .pin("GSR")
            .test_bel_input_inv(bcls::STARTUP::GSR, true)
            .attr("GSRATTR", "NOT")
            .commit();
        bctx.mode("STARTUP")
            .pin("GTS")
            .test_bel_input_inv(bcls::STARTUP::GTS, true)
            .attr("GTSATTR", "NOT")
            .commit();
        bctx.build()
            .test_global_attr_bool_rename("CRC", bcls::STARTUP::CRC, "DISABLE", "ENABLE");
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", bcls::STARTUP::CONFIG_RATE);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            bctx.build()
                .global("CRC", "DISABLE")
                .test_global_attr_bool_rename(
                    "EXPRESSMODE",
                    bcls::STARTUP::EXPRESS_MODE,
                    "DISABLE",
                    "ENABLE",
                );
        }
        for (val, vname, phase) in [
            (enums::STARTUP_MUX_CLK::CCLK, "CCLK", "C4"),
            (enums::STARTUP_MUX_CLK::USERCLK, "USERCLK", "U3"),
        ] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .global("SYNCTODONE", "NO")
                .global("DONEACTIVE", "C1")
                .test_bel_attr_val(bcls::STARTUP::MUX_CLK, val)
                .global_diff("GSRINACTIVE", "C4", phase)
                .global_diff("OUTPUTSACTIVE", "C4", phase)
                .global_diff("STARTUPCLK", "CCLK", vname)
                .commit();
        }
        bctx.build()
            .global("STARTUPCLK", "CCLK")
            .global("DONEACTIVE", "C1")
            .test_bel_attr_bits(bcls::STARTUP::SYNC_TO_DONE)
            .global_diff("GSRINACTIVE", "C4", "DI_PLUS_1")
            .global_diff("OUTPUTSACTIVE", "C4", "DI_PLUS_1")
            .global_diff("SYNCTODONE", "NO", "YES")
            .commit();
        for (val, rval) in [
            (enums::DONE_TIMING::Q0, "C1"),
            (enums::DONE_TIMING::Q1Q4, "C2"),
            (enums::DONE_TIMING::Q2, "C3"),
            (enums::DONE_TIMING::Q3, "C4"),
        ] {
            bctx.build()
                .global("SYNCTODONE", "NO")
                .global("STARTUPCLK", "CCLK")
                .test_bel_attr_val(bcls::STARTUP::DONE_TIMING, val)
                .global_diff("DONEACTIVE", "C1", rval)
                .commit();
        }
        for (val, rval) in [
            (enums::DONE_TIMING::Q2, "U2"),
            (enums::DONE_TIMING::Q3, "U3"),
            (enums::DONE_TIMING::Q1Q4, "U4"),
        ] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .global("SYNCTODONE", "NO")
                .global("STARTUPCLK", "USERCLK")
                .test_bel_attr_val(bcls::STARTUP::DONE_TIMING, val)
                .global_diff("DONEACTIVE", "C1", rval)
                .commit();
        }
        for (attr, opt) in [
            (bcls::STARTUP::GTS_TIMING, "OUTPUTSACTIVE"),
            (bcls::STARTUP::GSR_TIMING, "GSRINACTIVE"),
        ] {
            for (val, rval) in [
                (enums::GTS_GSR_TIMING::Q1Q4, "C2"),
                (enums::GTS_GSR_TIMING::Q2, "C3"),
                (enums::GTS_GSR_TIMING::Q3, "C4"),
            ] {
                bctx.build()
                    .global("SYNCTODONE", "NO")
                    .global("STARTUPCLK", "CCLK")
                    .test_bel_attr_val(attr, val)
                    .global_diff(opt, "C4", rval)
                    .commit();
            }
            for (val, rval) in [
                (enums::GTS_GSR_TIMING::Q2, "U2"),
                (enums::GTS_GSR_TIMING::Q3, "U3"),
                (enums::GTS_GSR_TIMING::Q1Q4, "U4"),
            ] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .global("SYNCTODONE", "NO")
                    .global("STARTUPCLK", "USERCLK")
                    .test_bel_attr_val(attr, val)
                    .global_diff(opt, "U3", rval)
                    .commit();
            }
            for (val, rval) in [
                (enums::GTS_GSR_TIMING::DONE_IN, "DI"),
                (enums::GTS_GSR_TIMING::Q3, "DI_PLUS_1"),
                (enums::GTS_GSR_TIMING::Q1Q4, "DI_PLUS_2"),
            ] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .global("SYNCTODONE", "YES")
                    .global("STARTUPCLK", "USERCLK")
                    .test_bel_attr_val(attr, val)
                    .global_diff(opt, "DI_PLUS_1", rval)
                    .commit();
            }
        }
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_NW);
        let mut bctx = ctx.bel(bslots::MISC_NW);
        bctx.build()
            .test_global_attr_bool_rename("TMLEFT", bcls::MISC_NW::TM_LEFT, "OFF", "ON");
        bctx.build()
            .test_global_attr_bool_rename("TMTOP", bcls::MISC_NW::TM_TOP, "OFF", "ON");
        bctx.build()
            .test_global_attr_rename("INPUT", bcls::MISC_NW::IO_ISTD);
        bctx.build()
            .test_global_attr_rename("OUTPUT", bcls::MISC_NW::IO_OSTD);
        if edev.chip.kind != ChipKind::Xc4000E {
            bctx.build()
                .test_global_attr_bool_rename("3V", bcls::MISC_NW::_3V, "OFF", "ON");
        }
        let mut bctx = ctx.bel(bslots::BSCAN);
        bctx.build()
            .extra_fixed_bel_attr_bits(
                CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_n())
                    .tile(tslots::MAIN),
                bslots::TDO,
                bcls::TDO::BSCAN_ENABLE,
            )
            .test_bel_attr_bits(bcls::BSCAN::ENABLE)
            .mode("BSCAN")
            .attr("BSCAN", "USED")
            .commit();
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            bctx.build().test_global_attr_bool_rename(
                "BSCAN_CONFIG",
                bcls::BSCAN::CONFIG,
                "DISABLE",
                "ENABLE",
            );
        }
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_NE);
        let mut bctx = ctx.bel(bslots::OSC);
        let cnr_se = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_s())
            .tile(tslots::MAIN);
        for (attr, spec, pin, opin) in [
            (
                bcls::MISC_SE::OSC_MUX_OUT0,
                specials::OSC_MUX_OUT0,
                "OUT0",
                "OUT1",
            ),
            (
                bcls::MISC_SE::OSC_MUX_OUT1,
                specials::OSC_MUX_OUT1,
                "OUT1",
                "OUT0",
            ),
        ] {
            for (val, spin) in [
                (enums::OSC_MUX_OUT::F15, "F15"),
                (enums::OSC_MUX_OUT::F490, "F490"),
                (enums::OSC_MUX_OUT::F16K, "F16K"),
                (enums::OSC_MUX_OUT::F500K, "F500K"),
            ] {
                bctx.build()
                    .mutex("MODE", "USE")
                    .mutex(format!("MUX.{pin}"), spin)
                    .mutex(format!("MUX.{opin}"), spin)
                    .pip(opin, spin)
                    .extra_fixed_bel_attr_val(cnr_se, bslots::MISC_SE, attr, val)
                    .null_bits()
                    .test_bel_special_val(spec, val)
                    .pip(pin, spin)
                    .commit();
            }
        }
        bctx.build()
            .mutex("MODE", "TEST")
            .extra_fixed_bel_attr_bits(cnr_se, bslots::MISC_SE, bcls::MISC_SE::OSC_ENABLE)
            .null_bits()
            .test_bel_special(specials::OSC_ENABLE)
            .pip("OUT0", "F15")
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_NE);
        let mut bctx = ctx.bel(bslots::MISC_NE);
        bctx.build()
            .test_global_attr_bool_rename("TMRIGHT", bcls::MISC_NE::TM_RIGHT, "OFF", "ON");
        if edev.chip.kind != ChipKind::Xc4000E {
            bctx.build()
                .test_global_attr_bool_rename("TAC", bcls::MISC_NE::TAC, "OFF", "ON");
            for (val, vname) in [
                (enums::ADDRESS_LINES::_18, "18"),
                (enums::ADDRESS_LINES::_22, "22"),
            ] {
                bctx.build()
                    .test_bel_attr_val(bcls::MISC_NE::ADDRESS_LINES, val)
                    .global("ADDRESSLINES", vname)
                    .commit();
            }
        }

        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            let mut bctx = ctx.bel(bslots::TDO);
            bctx.build().test_global_attr_bool_rename(
                "BSCAN_STATUS",
                bcls::TDO::BSCAN_STATUS,
                "DISABLE",
                "ENABLE",
            );
        }

        let mut bctx = ctx.bel(bslots::TDO);
        for (val, vname) in [
            (enums::IO_PULL::PULLUP, "PULLUP"),
            (enums::IO_PULL::PULLDOWN, "PULLDOWN"),
            (enums::IO_PULL::NONE, "PULLNONE"),
        ] {
            bctx.build()
                .test_bel_attr_val(bcls::TDO::PULL, val)
                .global("TDOPIN", vname)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_SE);
        let mut bctx = ctx.bel(bslots::READCLK);
        for (val, vname) in [
            (enums::RDBK_MUX_CLK::CCLK, "CCLK"),
            (enums::RDBK_MUX_CLK::RDBK, "RDBK"),
        ] {
            let tcrd = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_n())
                .tile(tslots::MAIN);
            bctx.mode("READCLK")
                .pin("I")
                .extra_fixed_bel_attr_val(tcrd, bslots::MISC_NE, bcls::MISC_NE::READCLK, val)
                .null_bits()
                .test_bel_special_val(specials::READCLK, val)
                .global("READCLK", vname)
                .commit();
        }
    }

    {
        let tcid = if edev.chip.kind.is_xl() {
            tcls::LLVC_IO_E
        } else {
            tcls::LLV_IO_E
        };
        let mut ctx = FuzzCtx::new_id(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::MISC_E);
        bctx.build()
            .test_global_attr_bool_rename("TLC", bcls::MISC_E::TLC, "OFF", "ON");
    }

    if edev.chip.kind == ChipKind::SpartanXl {
        let mut ctx = FuzzCtx::new_null(session, backend);
        let cnr_sw = CellCoord::new(DieId::from_idx(0), edev.chip.col_w(), edev.chip.row_s())
            .tile(tslots::MAIN);
        let cnr_se = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_s())
            .tile(tslots::MAIN);
        let cnr_ne = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_n())
            .tile(tslots::MAIN);
        ctx.build()
            .prop(ExtraTileSingle::new(cnr_sw, bslots::MISC_SW))
            .prop(ExtraTileSingle::new(cnr_se, bslots::MISC_SE))
            .prop(ExtraTileSingle::new(cnr_ne, bslots::MISC_NE))
            .prop(ExtraTilesAllIo)
            .test_global_special(specials::_5V_TOLERANT_IO_OFF)
            .global("5V_TOLERANT_IO", "OFF")
            .commit();
    }
    if edev.chip.kind == ChipKind::Xc4000Ex {
        let mut ctx = FuzzCtx::new_null(session, backend);
        for (spec, val) in [
            (specials::PUMP_EXTERNAL, "EXTERNAL"),
            (specials::PUMP_INTERNAL, "INTERNAL"),
        ] {
            ctx.build()
                .prop(ExtraTilesIoW)
                .test_global_special(spec)
                .global("PUMP", val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };

    {
        let tcid = tcls::CNR_SW;
        ctx.collect_bel_attr_enum_bool(tcid, bslots::RDBK, bcls::RDBK::READ_ABORT);
        ctx.collect_bel_attr_enum_bool(tcid, bslots::RDBK, bcls::RDBK::READ_CAPTURE);
        ctx.collect_bel_attr(tcid, bslots::MD0, bcls::IBUF::PULL);
        ctx.collect_bel_attr(tcid, bslots::MD1, bcls::MD1::PULL);
        ctx.collect_bel_attr(tcid, bslots::MD2, bcls::IBUF::PULL);
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_SW, bcls::MISC_SW::TM_BOT);
        if matches!(edev.chip.kind, ChipKind::Xc4000Xla | ChipKind::Xc4000Xv) {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_SW, bcls::MISC_SW::IO_5V_TOLERANT);
        }
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff =
                ctx.get_diff_bel_special(tcid, bslots::MISC_SW, specials::_5V_TOLERANT_IO_OFF);
            let diff_m0 = diff.split_bits_by(|bit| bit.frame.to_idx() == 21);
            let diff_m1 = diff.split_bits_by(|bit| bit.frame.to_idx() == 22);
            let diff_m2 = diff.split_bits_by(|bit| bit.frame.to_idx() == 20);
            diff.assert_empty();
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MD0,
                bcls::IBUF::_5V_TOLERANT,
                xlat_bit_raw(!diff_m0),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MD1,
                bcls::MD1::_5V_TOLERANT,
                xlat_bit_raw(!diff_m1),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MD2,
                bcls::IBUF::_5V_TOLERANT,
                xlat_bit_raw(!diff_m2),
            );
        }
    }

    {
        let tcid = tcls::CNR_SE;
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_SE, bcls::MISC_SE::TCTEST);
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_SE, bcls::MISC_SE::DONE_PULLUP);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
        ) {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_SE, bcls::MISC_SE::FIX_DISCHARGE);
        }
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_SE, bcls::MISC_SE::TM_OSC);
            ctx.collect_bel_attr(tcid, bslots::MISC_SE, bcls::MISC_SE::OSC_CLK);
        }
        ctx.collect_bel_attr(tcid, bslots::MISC_SE, bcls::MISC_SE::OSC_ENABLE);
        ctx.collect_bel_attr(tcid, bslots::MISC_SE, bcls::MISC_SE::OSC_MUX_OUT0);
        ctx.collect_bel_attr(tcid, bslots::MISC_SE, bcls::MISC_SE::OSC_MUX_OUT1);

        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff =
                ctx.get_diff_bel_special(tcid, bslots::MISC_SE, specials::_5V_TOLERANT_IO_OFF);
            let diff_prog = diff.split_bits_by(|bit| bit.frame.to_idx() == 8);
            let diff_done = diff.split_bits_by(|bit| bit.frame.to_idx() == 3);
            diff.assert_empty();
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_SE,
                bcls::MISC_SE::PROG_5V_TOLERANT,
                xlat_bit_raw(!diff_prog),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_SE,
                bcls::MISC_SE::DONE_5V_TOLERANT,
                xlat_bit_raw(!diff_done),
            );
        }

        ctx.collect_bel_attr_enum_bool(tcid, bslots::STARTUP, bcls::STARTUP::CRC);
        ctx.collect_bel_attr(tcid, bslots::STARTUP, bcls::STARTUP::CONFIG_RATE);
        ctx.collect_bel_attr(tcid, bslots::STARTUP, bcls::STARTUP::SYNC_TO_DONE);
        ctx.collect_bel_attr(tcid, bslots::STARTUP, bcls::STARTUP::DONE_TIMING);
        ctx.collect_bel_attr(tcid, bslots::STARTUP, bcls::STARTUP::GTS_TIMING);
        ctx.collect_bel_attr(tcid, bslots::STARTUP, bcls::STARTUP::GSR_TIMING);
        ctx.collect_bel_attr(tcid, bslots::STARTUP, bcls::STARTUP::MUX_CLK);
        ctx.collect_bel_input_inv(tcid, bslots::STARTUP, bcls::STARTUP::GSR);
        ctx.collect_bel_input_inv(tcid, bslots::STARTUP, bcls::STARTUP::GTS);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::STARTUP, bcls::STARTUP::EXPRESS_MODE);
        }
    }

    {
        let tcid = tcls::CNR_NW;
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_NW, bcls::MISC_NW::TM_LEFT);
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_NW, bcls::MISC_NW::TM_TOP);
        ctx.collect_bel_attr(tcid, bslots::MISC_NW, bcls::MISC_NW::IO_ISTD);
        ctx.collect_bel_attr(tcid, bslots::MISC_NW, bcls::MISC_NW::IO_OSTD);
        if edev.chip.kind != ChipKind::Xc4000E {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_NW, bcls::MISC_NW::_3V);
        }
        ctx.collect_bel_attr(tcid, bslots::BSCAN, bcls::BSCAN::ENABLE);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::BSCAN, bcls::BSCAN::CONFIG);
        }
    }

    {
        let tcid = tcls::CNR_NE;
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_NE, bcls::MISC_NE::TM_RIGHT);
        if edev.chip.kind == ChipKind::Xc4000E {
            // ??? mysteriously not supported in ISE
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_NE,
                bcls::MISC_NE::TAC,
                TileBit::new(0, 15, 4).neg(),
            );
        } else {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_NE, bcls::MISC_NE::TAC);
            ctx.collect_bel_attr(tcid, bslots::MISC_NE, bcls::MISC_NE::ADDRESS_LINES);
        }
        ctx.collect_bel_attr(tcid, bslots::TDO, bcls::TDO::PULL);
        ctx.collect_bel_attr(tcid, bslots::TDO, bcls::TDO::BSCAN_ENABLE);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_bel_attr_enum_bool(tcid, bslots::TDO, bcls::TDO::BSCAN_STATUS);
        }

        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff =
                ctx.get_diff_bel_special(tcid, bslots::MISC_NE, specials::_5V_TOLERANT_IO_OFF);
            let diff_tdo = diff.split_bits_by(|bit| bit.frame.to_idx() == 12);
            let diff_cclk = diff.split_bits_by(|bit| bit.frame.to_idx() == 13);
            diff.assert_empty();
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::TDO,
                bcls::TDO::_5V_TOLERANT,
                xlat_bit_raw(!diff_tdo),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_NE,
                bcls::MISC_NE::CCLK_5V_TOLERANT,
                xlat_bit_raw(!diff_cclk),
            );
        }
        ctx.collect_bel_attr(tcid, bslots::MISC_NE, bcls::MISC_NE::READCLK);
    }

    {
        let tcid = if edev.chip.kind.is_xl() {
            tcls::LLVC_IO_E
        } else {
            tcls::LLV_IO_E
        };
        ctx.collect_bel_attr_enum_bool(tcid, bslots::MISC_E, bcls::MISC_E::TLC);
    }

    if edev.chip.kind == ChipKind::SpartanXl {
        for (tcid, tcname, tcls) in &edev.db.tile_classes {
            if !tcls.bels.contains_id(bslots::IO[0]) {
                continue;
            }
            if !ctx.has_tile_id(tcid) {
                continue;
            }
            let mut diff =
                ctx.get_diff_bel_special(tcid, bslots::IO[0], specials::_5V_TOLERANT_IO_OFF);
            let (f0, f1) = if tcname.starts_with("IO_W") {
                (19, 20)
            } else if tcname.starts_with("IO_E") {
                (6, 5)
            } else if tcname.starts_with("IO_S") || tcname.starts_with("IO_N") {
                (13, 12)
            } else {
                unreachable!()
            };
            let diff_iob0 = diff.split_bits_by(|bit| bit.frame.to_idx() == f0);
            let diff_iob1 = diff.split_bits_by(|bit| bit.frame.to_idx() == f1);
            diff.assert_empty();
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::IO[0],
                bcls::IO::_5V_TOLERANT,
                xlat_bit_raw(!diff_iob0),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::IO[1],
                bcls::IO::_5V_TOLERANT,
                xlat_bit_raw(!diff_iob1),
            );
        }
    }
    if edev.chip.kind == ChipKind::Xc4000Ex {
        for (tcid, _, tcls) in &edev.db.tile_classes {
            if !tcls.bels.contains_id(bslots::MISC_W) {
                continue;
            }
            if !ctx.has_tile_id(tcid) {
                continue;
            }
            ctx.collect_bel_attr(tcid, bslots::MISC_W, bcls::MISC_W::PUMP);
        }
    }
}
