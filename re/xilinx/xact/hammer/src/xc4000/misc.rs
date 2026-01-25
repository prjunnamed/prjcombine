use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{CellCoord, DieId};
use prjcombine_re_fpga_hammer::diff::{Diff, xlat_bit_raw, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc4000::{bslots, enums, tslots, xc4000::bcls, xc4000::tcls};

use crate::{
    backend::{Key, XactBackend},
    collector::CollectorCtx,
    fbuild::{FuzzCtx, FuzzCtxBel},
    specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let chip = backend.edev.chip;
    let test_cfg4000_off_on = |bctx: &mut FuzzCtxBel, opt: &str, attr| {
        bctx.build()
            .raw(Key::GlobalMutex(opt.into()), "OFF")
            .test_bel_attr_enum_bool(attr, false)
            .raw_diff(
                Key::BlockConfig("_cfg4000_", opt.into(), "OFF".into()),
                false,
                true,
            )
            .commit();
        bctx.build()
            .raw(Key::GlobalMutex(opt.into()), "ON")
            .test_bel_attr_enum_bool(attr, true)
            .raw_diff(
                Key::BlockConfig("_cfg4000_", opt.into(), "ON".into()),
                false,
                true,
            )
            .commit();
    };
    for (tcid, _, tcls) in &backend.edev.db.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for slots in [bslots::PULLUP_DEC_H, bslots::PULLUP_DEC_V] {
            for bslot in slots {
                if !tcls.bels.contains_id(bslot) {
                    continue;
                }
                let mut bctx = ctx.bel(bslot);
                bctx.build()
                    .bidir_mutex_exclusive(bcls::PULLUP::O)
                    .test_bel_attr_bits(bcls::PULLUP::ENABLE)
                    .pip_pin("O", "O")
                    .commit();
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SW);
        let mut bctx = ctx.bel(bslots::RDBK);
        bctx.build().test_attr_global_enum_bool_as(
            "READABORT",
            bcls::RDBK::READ_ABORT,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_attr_global_enum_bool_as(
            "READCAPTURE",
            bcls::RDBK::READ_CAPTURE,
            "DISABLE",
            "ENABLE",
        );
        let mut bctx = ctx.bel(bslots::MD1);
        bctx.build()
            .test_attr_global_default_as("M1PIN", bcls::MD1::PULL, enums::IO_PULL::NONE);
        let mut bctx = ctx.bel(bslots::MISC_SW);
        test_cfg4000_off_on(&mut bctx, "TMBOT", bcls::MISC_SW::TM_BOT);
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SE);
        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.build()
            .test_attr_global_enum_bool_as("CRC", bcls::STARTUP::CRC, "DISABLE", "ENABLE");
        bctx.build()
            .test_attr_global_as("CONFIGRATE", bcls::STARTUP::CONFIG_RATE);
        bctx.build()
            .global("SYNCTODONE", "NO")
            .global("DONEACTIVE", "C1")
            .test_bel_attr_val(bcls::STARTUP::MUX_CLK, enums::STARTUP_MUX_CLK::USERCLK)
            .global_diff("GSRINACTIVE", "C4", "U3")
            .global_diff("OUTPUTSACTIVE", "C4", "U3")
            .global_diff("STARTUPCLK", "CCLK", "USERCLK")
            .bel_out("STARTUP", "CLK")
            .commit();
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
                .global("STARTUPCLK", "CCLK")
                .global("SYNCTODONE", "NO")
                .global("GSRINACTIVE", "C4")
                .global("OUTPUTSACTIVE", "C4")
                .test_bel_attr_val(bcls::STARTUP::DONE_TIMING, val)
                .global_diff("DONEACTIVE", "C1", rval)
                .commit();
        }
        for (val, rval) in [
            (enums::DONE_TIMING::Q2, "U2"),
            (enums::DONE_TIMING::Q3, "U3"),
            (enums::DONE_TIMING::Q1Q4, "U4"),
        ] {
            bctx.build()
                .global("STARTUPCLK", "USERCLK")
                .global("SYNCTODONE", "NO")
                .global("GSRINACTIVE", "U3")
                .global("OUTPUTSACTIVE", "U3")
                .bel_out("STARTUP", "CLK")
                .test_bel_attr_val(bcls::STARTUP::DONE_TIMING, val)
                .global_diff("DONEACTIVE", "C1", rval)
                .commit();
        }
        for (attr, opt, oopt) in [
            (bcls::STARTUP::GTS_TIMING, "OUTPUTSACTIVE", "GSRINACTIVE"),
            (bcls::STARTUP::GSR_TIMING, "GSRINACTIVE", "OUTPUTSACTIVE"),
        ] {
            for (val, rval) in [
                (enums::GTS_GSR_TIMING::Q1Q4, "C2"),
                (enums::GTS_GSR_TIMING::Q2, "C3"),
                (enums::GTS_GSR_TIMING::Q3, "C4"),
            ] {
                bctx.build()
                    .global("STARTUPCLK", "CCLK")
                    .global("SYNCTODONE", "NO")
                    .global("DONEACTIVE", "C1")
                    .global(oopt, "C4")
                    .test_bel_attr_val(attr, val)
                    .global_diff(opt, "C4", rval)
                    .commit();
            }
            for (val, rval) in [
                (enums::GTS_GSR_TIMING::Q2, "U2"),
                (enums::GTS_GSR_TIMING::Q3, "U3"),
                (enums::GTS_GSR_TIMING::Q1Q4, "U4"),
            ] {
                bctx.build()
                    .global("STARTUPCLK", "USERCLK")
                    .global("SYNCTODONE", "NO")
                    .global("DONEACTIVE", "C1")
                    .global(oopt, "U3")
                    .bel_out("STARTUP", "CLK")
                    .test_bel_attr_val(attr, val)
                    .global_diff(opt, "U3", rval)
                    .commit();
            }
            for (val, rval) in [
                (enums::GTS_GSR_TIMING::DONE_IN, "DI"),
                (enums::GTS_GSR_TIMING::Q3, "DI_PLUS_1"),
                (enums::GTS_GSR_TIMING::Q1Q4, "DI_PLUS_2"),
            ] {
                bctx.build()
                    .global("STARTUPCLK", "USERCLK")
                    .global("SYNCTODONE", "YES")
                    .global("DONEACTIVE", "C1")
                    .global(oopt, "DI_PLUS_1")
                    .bel_out("STARTUP", "CLK")
                    .test_bel_attr_val(attr, val)
                    .global_diff(opt, "DI_PLUS_1", rval)
                    .commit();
            }
        }
        bctx.mode("STARTUP")
            .test_bel_input_inv(bcls::STARTUP::GTS, true)
            .cfg("GTS", "NOT")
            .commit();
        bctx.mode("STARTUP")
            .test_bel_input_inv(bcls::STARTUP::GSR, true)
            .cfg("GSR", "NOT")
            .commit();
        let mut bctx = ctx.bel(bslots::MISC_SE);
        bctx.build().test_attr_global_enum_bool_as(
            "DONEPIN",
            bcls::MISC_SE::DONE_PULLUP,
            "NOPULLUP",
            "PULLUP",
        );
        test_cfg4000_off_on(&mut bctx, "TCTEST", bcls::MISC_SE::TCTEST);
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NW);
        let mut bctx = ctx.bel(bslots::BSCAN);
        bctx.mode("BSCAN")
            .extra_fixed_bel_attr_bits(
                CellCoord::new(DieId::from_idx(0), chip.col_e(), chip.row_n()).tile(tslots::MAIN),
                bslots::TDO,
                bcls::TDO::BSCAN_ENABLE,
            )
            .test_bel_attr_bits(bcls::BSCAN::ENABLE)
            .cfg("BSCAN", "USED")
            .commit();

        let mut bctx = ctx.bel(bslots::MISC_NW);
        test_cfg4000_off_on(&mut bctx, "TMLEFT", bcls::MISC_NW::TM_LEFT);
        test_cfg4000_off_on(&mut bctx, "TMTOP", bcls::MISC_NW::TM_TOP);
        bctx.build()
            .mutex("TTLBAR", "OFF")
            .test_bel_attr_val(bcls::MISC_NW::IO_ISTD, enums::IO_STD::TTL)
            .raw_diff(
                Key::BlockConfig("_cfg4000_", "TTLBAR".into(), "OFF".into()),
                false,
                true,
            )
            .commit();
        bctx.build()
            .mutex("TTLBAR", "ON")
            .test_bel_attr_val(bcls::MISC_NW::IO_ISTD, enums::IO_STD::CMOS)
            .raw_diff(
                Key::BlockConfig("_cfg4000_", "TTLBAR".into(), "ON".into()),
                false,
                true,
            )
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NE);

        let mut bctx = ctx.bel(bslots::TDO);
        bctx.build()
            .test_attr_global_default_as("TDOPIN", bcls::TDO::PULL, enums::IO_PULL::NONE);

        let mut bctx = ctx.bel(bslots::OSC);
        for (attr, out) in [
            (bcls::MISC_SE::OSC_MUX_OUT0, "OUT0"),
            (bcls::MISC_SE::OSC_MUX_OUT1, "OUT1"),
        ] {
            for (val, pin) in [
                (enums::OSC_MUX_OUT::F500K, "F500K"),
                (enums::OSC_MUX_OUT::F16K, "F16K"),
                (enums::OSC_MUX_OUT::F490, "F490"),
                (enums::OSC_MUX_OUT::F15, "F15"),
            ] {
                bctx.build()
                    .extra_fixed_bel_attr_val(
                        CellCoord::new(DieId::from_idx(0), chip.col_e(), chip.row_s())
                            .tile(tslots::MAIN),
                        bslots::MISC_SE,
                        attr,
                        val,
                    )
                    .mutex("MODE", "TEST")
                    .mutex("MUXOUT", out)
                    .mutex("MUXIN", pin)
                    .null_bits()
                    .test_bel_special(specials::OSC_NULL)
                    .pip_pin(format!("{out}_{pin}"), pin)
                    .commit();
            }
        }

        let mut bctx = ctx.bel(bslots::MISC_NE);
        test_cfg4000_off_on(&mut bctx, "TMRIGHT", bcls::MISC_NE::TM_RIGHT);
        test_cfg4000_off_on(&mut bctx, "TAC", bcls::MISC_NE::TAC);
        bctx.test_attr_global_as("READCLK", bcls::MISC_NE::READCLK);
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::LLV_IO_E);
        let mut bctx = ctx.bel(bslots::MISC_E);
        test_cfg4000_off_on(&mut bctx, "TLC", bcls::MISC_E::TLC);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, _, tcls) in &ctx.edev.db.tile_classes {
        if !ctx.has_tile(tcid) {
            continue;
        }
        for slots in [bslots::PULLUP_DEC_H, bslots::PULLUP_DEC_V] {
            for bslot in slots {
                if !tcls.bels.contains_id(bslot) {
                    continue;
                }
                ctx.collect_bel_attr(tcid, bslot, bcls::PULLUP::ENABLE);
            }
        }
    }
    {
        let tcid = tcls::CNR_SW;
        let bslot = bslots::RDBK;
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::RDBK::READ_ABORT);
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::RDBK::READ_CAPTURE);
        let bslot = bslots::MD1;
        ctx.collect_bel_attr_default(tcid, bslot, bcls::MD1::PULL, enums::IO_PULL::NONE);
        let bslot = bslots::MISC_SW;
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_SW::TM_BOT);
    }
    {
        let tcid = tcls::CNR_SE;
        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::STARTUP::CRC);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::CONFIG_RATE);
        ctx.collect_bel_input_inv(tcid, bslot, bcls::STARTUP::GSR);
        ctx.collect_bel_input_inv(tcid, bslot, bcls::STARTUP::GTS);
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            bcls::STARTUP::MUX_CLK,
            enums::STARTUP_MUX_CLK::CCLK,
        );
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::SYNC_TO_DONE);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::DONE_TIMING);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::GTS_TIMING);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::GSR_TIMING);

        let bslot = bslots::MISC_SE;
        let mut diffs0 = vec![];
        let mut diffs1 = vec![];
        for val in [
            enums::OSC_MUX_OUT::F500K,
            enums::OSC_MUX_OUT::F16K,
            enums::OSC_MUX_OUT::F490,
            enums::OSC_MUX_OUT::F15,
        ] {
            let diff0 = ctx.get_diff_attr_val(tcid, bslot, bcls::MISC_SE::OSC_MUX_OUT0, val);
            let diff1 = ctx.get_diff_attr_val(tcid, bslot, bcls::MISC_SE::OSC_MUX_OUT1, val);
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            diffs0.push((val, diff0));
            diffs1.push((val, diff1));
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::MISC_SE::OSC_ENABLE,
                xlat_bit_raw(diff_en),
            );
        }
        ctx.insert_bel_attr_raw(
            tcid,
            bslot,
            bcls::MISC_SE::OSC_MUX_OUT0,
            xlat_enum_attr(diffs0),
        );
        ctx.insert_bel_attr_raw(
            tcid,
            bslot,
            bcls::MISC_SE::OSC_MUX_OUT1,
            xlat_enum_attr(diffs1),
        );

        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_SE::DONE_PULLUP);
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_SE::TCTEST);
    }
    {
        let tcid = tcls::CNR_NW;
        let bslot = bslots::BSCAN;
        ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::ENABLE);
        let bslot = bslots::MISC_NW;
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_NW::TM_LEFT);
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_NW::TM_TOP);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_NW::IO_ISTD);
    }
    {
        let tcid = tcls::CNR_NE;

        let bslot = bslots::TDO;
        ctx.collect_bel_attr_default(tcid, bslot, bcls::TDO::PULL, enums::IO_PULL::NONE);
        ctx.collect_bel_attr(tcid, bslot, bcls::TDO::BSCAN_ENABLE);

        let bslot = bslots::MISC_NE;
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_NE::TM_RIGHT);
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_NE::TAC);
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_NE::READCLK);
    }
    {
        let tcid = tcls::LLV_IO_E;
        let bslot = bslots::MISC_E;
        ctx.collect_bel_attr_enum_bool(tcid, bslot, bcls::MISC_E::TLC);
    }
}
