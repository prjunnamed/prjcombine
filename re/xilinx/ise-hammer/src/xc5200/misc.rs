use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttribute, BelAttributeEnum},
    grid::{CellCoord, DieId},
};
use prjcombine_re_fpga_hammer::{OcdMode, xlat_enum_raw};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bits;
use prjcombine_xc2000::xc5200::{bcls, bslots, enums, tcls, tslots};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    xc5200::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_SW);
        let mut bctx = ctx.bel(bslots::MISC_SW);
        for (val, vname) in &backend.edev.db[enums::SCAN_TEST].values {
            bctx.build()
                .test_bel_attr_val(bcls::MISC_SW::SCAN_TEST, val)
                .global("SCANTEST", vname)
                .commit();
        }
        let mut bctx = ctx.bel(bslots::BUFG);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("CLK")
            .commit();
        let mut bctx = ctx.bel(bslots::RDBK);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("RDBK")
            .commit();
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
        for (val, vname) in &backend.edev.db[enums::RDBK_MUX_CLK].values {
            bctx.mode("RDBK")
                .pin("CK")
                .test_bel_attr_val(bcls::RDBK::MUX_CLK, val)
                .global("READCLK", vname)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_NW);
        let mut bctx = ctx.bel(bslots::MISC_NW);
        for (val, vname) in &backend.edev.db[enums::IO_INPUT_MODE].values {
            bctx.build()
                .test_bel_attr_val(bcls::MISC_NW::IO_INPUT_MODE, val)
                .global("INPUT", vname)
                .commit();
        }
        let mut bctx = ctx.bel(bslots::BUFG);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("CLK")
            .commit();
        let mut bctx = ctx.bel(bslots::BSCAN);
        bctx.build()
            .test_bel_attr_bits(bcls::BSCAN::ENABLE)
            .mode("BSCAN")
            .commit();
        bctx.build().test_global_attr_bool_rename(
            "BSRECONFIG",
            bcls::BSCAN::RECONFIG,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "BSREADBACK",
            bcls::BSCAN::READBACK,
            "DISABLE",
            "ENABLE",
        );
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_SE);
        let mut bctx = ctx.bel(bslots::MISC_SE);
        bctx.build().test_global_attr_bool_rename(
            "PROGPIN",
            bcls::MISC_SE::PROG_PULLUP,
            "PULLNONE",
            "PULLUP",
        );
        bctx.build().test_global_attr_bool_rename(
            "DONEPIN",
            bcls::MISC_SE::DONE_PULLUP,
            "PULLNONE",
            "PULLUP",
        );
        bctx.build()
            .test_global_attr_bool_rename("TCTEST", bcls::MISC_SE::TCTEST, "OFF", "ON");
        let mut bctx = ctx.bel(bslots::BUFG);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("CLK")
            .commit();
        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("STARTUP")
            .commit();
        for (val, rval) in [(false, "GR"), (true, "GRNOT")] {
            bctx.mode("STARTUP")
                .pin("GR")
                .test_bel_input_inv(bcls::STARTUP::GR, val)
                .attr("GRMUX", rval)
                .commit();
        }
        for (val, rval) in [(false, "GTS"), (true, "GTSNOT")] {
            bctx.mode("STARTUP")
                .pin("GTS")
                .test_bel_input_inv(bcls::STARTUP::GTS, val)
                .attr("GTSMUX", rval)
                .commit();
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
        for (val, rval, phase) in [(false, "NO", "C4"), (true, "YES", "DI_PLUS_1")] {
            bctx.build()
                .global("STARTUPCLK", "CCLK")
                .global("DONEACTIVE", "C1")
                .test_bel_attr_enum_bool(bcls::STARTUP::SYNC_TO_DONE, val)
                .global_diff("GSRINACTIVE", "C4", phase)
                .global_diff("OUTPUTSACTIVE", "C4", phase)
                .global_diff("SYNCTODONE", "NO", rval)
                .commit();
        }
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

        bctx.build()
            .test_global_attr_bool_rename("CRC", bcls::STARTUP::CRC, "DISABLE", "ENABLE");
        for (val, vname) in &backend.edev.db[enums::CONFIG_RATE].values {
            bctx.build()
                .test_bel_attr_val(bcls::STARTUP::CONFIG_RATE, val)
                .global("CONFIGRATE", vname)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new_id(session, backend, tcls::CNR_NE);
        let mut bctx = ctx.bel(bslots::MISC_NE);
        bctx.build()
            .test_global_attr_bool_rename("TAC", bcls::MISC_NE::TAC, "OFF", "ON");
        bctx.build()
            .test_global_attr_bool_rename("TLC", bcls::MISC_NE::TLC, "OFF", "ON");
        let mut bctx = ctx.bel(bslots::BUFG);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("CLK")
            .commit();

        // pins located in NE, config in SE.
        let mut bctx = ctx.bel(bslots::OSC_NE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::ENABLE)
            .mode("OSC")
            .commit();
        let cnr_se = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_s())
            .tile(tslots::MAIN);
        for (val, rval) in [
            (enums::OSC1_DIV::D2, "4"),
            (enums::OSC1_DIV::D4, "16"),
            (enums::OSC1_DIV::D6, "64"),
            (enums::OSC1_DIV::D8, "256"),
        ] {
            bctx.mode("OSC")
                .null_bits()
                .extra_fixed_bel_attr_val(cnr_se, bslots::OSC_SE, bcls::OSC_SE::OSC1_DIV, val)
                .test_bel_attr_val(bcls::OSC_SE::OSC1_DIV, val)
                .attr("OSC1_ATTR", rval)
                .commit();
        }
        for (val, rval) in [
            (enums::OSC2_DIV::D1, "2"),
            (enums::OSC2_DIV::D3, "8"),
            (enums::OSC2_DIV::D5, "32"),
            (enums::OSC2_DIV::D7, "128"),
            (enums::OSC2_DIV::D10, "1024"),
            (enums::OSC2_DIV::D12, "4096"),
            (enums::OSC2_DIV::D14, "16384"),
            (enums::OSC2_DIV::D16, "65536"),
        ] {
            bctx.mode("OSC")
                .null_bits()
                .extra_fixed_bel_attr_val(cnr_se, bslots::OSC_SE, bcls::OSC_SE::OSC2_DIV, val)
                .test_bel_attr_val(bcls::OSC_SE::OSC2_DIV, val)
                .attr("OSC2_ATTR", rval)
                .commit();
        }
        for (val, vname) in &backend.edev.db[enums::OSC_MUX_CLK].values {
            bctx.mode("OSC")
                .null_bits()
                .extra_fixed_bel_attr_val(cnr_se, bslots::OSC_SE, bcls::OSC_SE::MUX_CLK, val)
                .pin("C")
                .test_bel_attr_val(bcls::OSC_SE::MUX_CLK, val)
                .attr("CMUX", vname)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    ctx.collect_bel_attr(tcls::CNR_SW, bslots::MISC_SW, bcls::MISC_SW::SCAN_TEST);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_SW, bslots::RDBK, bcls::RDBK::READ_ABORT);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_SW, bslots::RDBK, bcls::RDBK::READ_CAPTURE);
    ctx.collect_bel_attr(tcls::CNR_SW, bslots::RDBK, bcls::RDBK::MUX_CLK);

    ctx.collect_bel_attr(tcls::CNR_NW, bslots::MISC_NW, bcls::MISC_NW::IO_INPUT_MODE);
    ctx.collect_bel_attr(tcls::CNR_NW, bslots::BSCAN, bcls::BSCAN::ENABLE);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_NW, bslots::BSCAN, bcls::BSCAN::READBACK);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_NW, bslots::BSCAN, bcls::BSCAN::RECONFIG);

    ctx.collect_bel_attr_enum_bool(tcls::CNR_SE, bslots::MISC_SE, bcls::MISC_SE::TCTEST);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_SE, bslots::MISC_SE, bcls::MISC_SE::DONE_PULLUP);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_SE, bslots::MISC_SE, bcls::MISC_SE::PROG_PULLUP);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::CRC);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::CONFIG_RATE);
    ctx.collect_bel_input_inv_bi(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GTS);
    ctx.collect_bel_input_inv_bi(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GR);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::DONE_TIMING);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::GTS_TIMING);
    {
        let mut diffs = vec![];
        for val in ctx.edev.db[enums::GTS_GSR_TIMING].values.ids() {
            diffs.push((
                val,
                ctx.get_diff_attr_val(
                    tcls::CNR_SE,
                    bslots::STARTUP,
                    bcls::STARTUP::GSR_TIMING,
                    val,
                ),
            ));
        }
        let (bits, mut values) = xlat_enum_raw(diffs, OcdMode::ValueOrder);
        // sigh. DI has identical value to DI_PLUS_2, which is obviously bogus.
        // not *completely* sure this is the right fixup, but it seems to be the most
        // likely option.
        assert_eq!(bits.len(), 2);
        values.insert(enums::GTS_GSR_TIMING::DONE_IN, bits![0; 2]);
        let attr = BelAttribute::Enum(BelAttributeEnum {
            bits,
            values: values.into_iter().collect(),
        });
        ctx.insert_bel_attr_raw(
            tcls::CNR_SE,
            bslots::STARTUP,
            bcls::STARTUP::GSR_TIMING,
            attr,
        );
    }
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::MUX_CLK);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_SE, bslots::STARTUP, bcls::STARTUP::SYNC_TO_DONE);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::OSC_SE, bcls::OSC_SE::OSC1_DIV);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::OSC_SE, bcls::OSC_SE::OSC2_DIV);
    ctx.collect_bel_attr(tcls::CNR_SE, bslots::OSC_SE, bcls::OSC_SE::MUX_CLK);

    ctx.collect_bel_attr_enum_bool(tcls::CNR_NE, bslots::MISC_NE, bcls::MISC_NE::TLC);
    ctx.collect_bel_attr_enum_bool(tcls::CNR_NE, bslots::MISC_NE, bcls::MISC_NE::TAC);
}
