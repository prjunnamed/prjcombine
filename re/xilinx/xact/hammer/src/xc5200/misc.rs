use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{CellCoord, DieId};
use prjcombine_re_hammer::Session;
use prjcombine_xc2000::xc5200::{bcls, bslots, enums, tcls, tslots};

use crate::{
    backend::{Key, XactBackend},
    collector::CollectorCtx,
    fbuild::FuzzCtx,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SW);
    let mut bctx = ctx.bel(bslots::MISC_SW);
    for (val, vname) in [
        (enums::SCAN_TEST::DISABLE, "DISABLE"),
        (enums::SCAN_TEST::ENABLE, "ENABLE"),
        (enums::SCAN_TEST::ENLL, "ENLL"),
        (enums::SCAN_TEST::NE7, "NE7"),
    ] {
        bctx.build()
            .mutex("SCANTEST", vname)
            .test_bel_attr_val(bcls::MISC_SW::SCAN_TEST, val)
            .raw_diff(
                Key::BlockConfig("_cfg5200_", "SCANTEST".into(), vname.into()),
                false,
                true,
            )
            .commit();
    }

    let mut bctx = ctx.bel(bslots::RDBK);
    bctx.test_attr_global_enum_bool_as("READABORT", bcls::RDBK::READ_ABORT, "DISABLE", "ENABLE");
    bctx.test_attr_global_enum_bool_as(
        "READCAPTURE",
        bcls::RDBK::READ_CAPTURE,
        "DISABLE",
        "ENABLE",
    );
    bctx.test_attr_global_as("READCLK", bcls::RDBK::MUX_CLK);

    let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NW);
    let mut bctx = ctx.bel(bslots::MISC_NW);
    bctx.test_attr_global_as("INPUT", bcls::MISC_NW::IO_INPUT_MODE);
    let mut bctx = ctx.bel(bslots::BSCAN);
    bctx.test_attr_global_enum_bool_as("BSRECONFIG", bcls::BSCAN::RECONFIG, "DISABLE", "ENABLE");
    bctx.test_attr_global_enum_bool_as("BSREADBACK", bcls::BSCAN::READBACK, "DISABLE", "ENABLE");
    bctx.mode("BSCAN")
        .test_bel_attr_bits(bcls::BSCAN::ENABLE)
        .cfg("BSCAN", "USED")
        .commit();

    let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SE);
    let mut bctx = ctx.bel(bslots::MISC_SE);
    for (val, vname) in [(false, "OFF"), (true, "ON")] {
        bctx.build()
            .mutex("TCTEST", vname)
            .test_bel_attr_enum_bool(bcls::MISC_SE::TCTEST, val)
            .raw_diff(
                Key::BlockConfig("_cfg5200_", "TCTEST".into(), vname.into()),
                false,
                true,
            )
            .commit();
    }
    bctx.test_attr_global_enum_bool_as("PROGPIN", bcls::MISC_SE::PROG_PULLUP, "NOPULLUP", "PULLUP");
    bctx.test_attr_global_enum_bool_as("DONEPIN", bcls::MISC_SE::DONE_PULLUP, "NOPULLUP", "PULLUP");
    let mut bctx = ctx.bel(bslots::STARTUP);
    bctx.test_attr_global_enum_bool_as("CRC", bcls::STARTUP::CRC, "DISABLE", "ENABLE");
    bctx.test_attr_global_as("CONFIGRATE", bcls::STARTUP::CONFIG_RATE);
    bctx.build()
        .global("SYNCTODONE", "NO")
        .global("DONEACTIVE", "C1")
        .test_bel_attr_val(bcls::STARTUP::MUX_CLK, enums::STARTUP_MUX_CLK::USERCLK)
        .global_diff("GSRINACTIVE", "C4", "U3")
        .global_diff("OUTPUTSACTIVE", "C4", "U3")
        .global_diff("STARTUPCLK", "CCLK", "USERCLK")
        .bel_out("STARTUP", "CK")
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
            .bel_out("STARTUP", "CK")
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
                .bel_out("STARTUP", "CK")
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
                .bel_out("STARTUP", "CK")
                .test_bel_attr_val(attr, val)
                .global_diff(opt, "DI_PLUS_1", rval)
                .commit();
        }
    }

    let mut bctx = ctx.bel(bslots::STARTUP);
    bctx.mode("STARTUP")
        .test_bel_input_inv(bcls::STARTUP::GR, true)
        .cfg("GCLR", "NOT")
        .commit();
    bctx.mode("STARTUP")
        .test_bel_input_inv(bcls::STARTUP::GTS, true)
        .cfg("GTS", "NOT")
        .commit();

    let mut bctx = ctx.bel(bslots::OSC_SE);
    bctx.build()
        .bel_out("OSC", "CK")
        .test_attr_global_as("OSCCLK", bcls::OSC_SE::MUX_CLK);

    let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NE);
    let mut bctx = ctx.bel(bslots::MISC_NE);
    for (attr, aname) in [(bcls::MISC_NE::TLC, "TLC"), (bcls::MISC_NE::TAC, "TAC")] {
        for (val, vname) in [(false, "OFF"), (true, "ON")] {
            bctx.build()
                .mutex(aname, vname)
                .test_bel_attr_enum_bool(attr, val)
                .raw_diff(
                    Key::BlockConfig("_cfg5200_", aname.into(), vname.into()),
                    false,
                    true,
                )
                .commit();
        }
    }
    let mut bctx = ctx.bel(bslots::OSC_NE);
    let cnr_se = CellCoord::new(
        DieId::from_idx(0),
        backend.edev.chip.col_e(),
        backend.edev.chip.row_s(),
    )
    .tile(tslots::MAIN);
    for (val, vname) in &backend.edev.db.enum_classes[enums::OSC1_DIV].values {
        bctx.mode("OSC")
            .null_bits()
            .extra_fixed_bel_attr_val(cnr_se, bslots::OSC_SE, bcls::OSC_SE::OSC1_DIV, val)
            .mutex("OSC1", vname)
            .test_bel_attr_val(bcls::OSC_SE::OSC1_DIV, val)
            .cfg("OSC1", vname)
            .commit();
    }
    for (val, vname) in &backend.edev.db.enum_classes[enums::OSC2_DIV].values {
        bctx.mode("OSC")
            .null_bits()
            .extra_fixed_bel_attr_val(cnr_se, bslots::OSC_SE, bcls::OSC_SE::OSC2_DIV, val)
            .mutex("OSC2", vname)
            .test_bel_attr_val(bcls::OSC_SE::OSC2_DIV, val)
            .cfg("OSC2", vname)
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    ctx.collect_bel_attr(tcls::CNR_SW, bslots::MISC_SW, bcls::MISC_SW::SCAN_TEST);
    ctx.collect_bel_attr_bi(tcls::CNR_SW, bslots::RDBK, bcls::RDBK::READ_ABORT);
    ctx.collect_bel_attr_bi(tcls::CNR_SW, bslots::RDBK, bcls::RDBK::READ_CAPTURE);
    ctx.collect_bel_attr(tcls::CNR_SW, bslots::RDBK, bcls::RDBK::MUX_CLK);

    ctx.collect_bel_attr(tcls::CNR_NW, bslots::MISC_NW, bcls::MISC_NW::IO_INPUT_MODE);
    ctx.collect_bel_attr(tcls::CNR_NW, bslots::BSCAN, bcls::BSCAN::ENABLE);
    ctx.collect_bel_attr_bi(tcls::CNR_NW, bslots::BSCAN, bcls::BSCAN::READBACK);
    ctx.collect_bel_attr_bi(tcls::CNR_NW, bslots::BSCAN, bcls::BSCAN::RECONFIG);

    ctx.collect_bel_attr_bi(tcls::CNR_SE, bslots::MISC_SE, bcls::MISC_SE::TCTEST);
    ctx.collect_bel_attr_bi(tcls::CNR_SE, bslots::MISC_SE, bcls::MISC_SE::DONE_PULLUP);
    ctx.collect_bel_attr_bi(tcls::CNR_SE, bslots::MISC_SE, bcls::MISC_SE::PROG_PULLUP);
    ctx.collect_bel_attr_bi(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::CRC);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::CONFIG_RATE);
    ctx.collect_bel_input_inv(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GTS);
    ctx.collect_bel_input_inv(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GR);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::DONE_TIMING);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GTS_TIMING);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GSR_TIMING);
    ctx.collect_bel_attr_default(
        tcls::CNR_SE,
        bslots::STARTUP,
        bcls::STARTUP::MUX_CLK,
        enums::STARTUP_MUX_CLK::CCLK,
    );
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::SYNC_TO_DONE);

    ctx.collect_bel_attr(tcls::CNR_SE, bslots::OSC_SE, bcls::OSC_SE::OSC1_DIV);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::OSC_SE, bcls::OSC_SE::OSC2_DIV);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::OSC_SE, bcls::OSC_SE::MUX_CLK);

    ctx.collect_bel_attr_bi(tcls::CNR_NE, bslots::MISC_NE, bcls::MISC_NE::TLC);
    ctx.collect_bel_attr_bi(tcls::CNR_NE, bslots::MISC_NE, bcls::MISC_NE::TAC);
}
