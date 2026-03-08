use prjcombine_interconnect::db::{BelAttributeId, TileWireCoord};
use prjcombine_re_collector::diff::{OcdMode, xlat_bitvec_sparse_u32};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex::{
    chip::ChipKind,
    defs::{
        bcls::{BSCAN, CAPTURE, GLOBAL, MISC_NE, MISC_NW, MISC_SE, MISC_SW, PCILOGIC, STARTUP},
        bslots, enums, tcls, wires,
    },
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx, FuzzCtxBel},
        props::mutex::WireMutexExclusive,
    },
    virtex::specials,
};

fn test_pull(bctx: &mut FuzzCtxBel, attr: BelAttributeId, opt: &'static str) {
    for (val, vname) in [
        (enums::IOB_PULL::NONE, "PULLNONE"),
        (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
        (enums::IOB_PULL::PULLUP, "PULLUP"),
    ] {
        bctx.build()
            .test_bel_attr_val(attr, val)
            .global(opt, vname)
            .commit();
    }
}
fn test_pullup(bctx: &mut FuzzCtxBel, attr: BelAttributeId, opt: &'static str) {
    for (val, vname) in [
        (enums::IOB_PULL::NONE, "PULLNONE"),
        (enums::IOB_PULL::PULLUP, "PULLUP"),
    ] {
        bctx.build()
            .test_bel_attr_val(attr, val)
            .global(opt, vname)
            .commit();
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for tcid in [tcls::PCI_W_V, tcls::PCI_E_V, tcls::PCI_W_VE, tcls::PCI_E_VE] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::PCILOGIC);
        bctx.build()
            .test_bel_special(specials::PRESENT)
            .mode("PCILOGIC")
            .commit();
        for pin in [PCILOGIC::I1, PCILOGIC::I2] {
            let pname = backend.edev.db[PCILOGIC].inputs.key(pin).0;
            for (val, vname) in [
                // TODO: this is DIFFERENT from other virtex muxes; ISE bug or actual difference?
                // not adding the PULLUP edge yet just in case.
                (false, "0"),
                (false, pname),
                (true, "1"),
                (true, &format!("{pname}_B")),
            ] {
                bctx.mode("PCILOGIC")
                    .pin(pname)
                    .test_bel_input_inv(pin, val)
                    .attr(format!("{pname}MUX"), vname)
                    .commit();
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for (val, vname) in [(0, "00"), (1, "01"), (2, "10"), (3, "11")] {
        ctx.build()
            .extra_tiles_by_bel_attr_u32(bslots::PCILOGIC, PCILOGIC::PCI_DELAY, val)
            .test_global_special(specials::PCI_DELAY)
            .global("PCIDELAY", vname)
            .commit();
    }

    let (cnr_sw, cnr_nw) = if edev.chip.kind == ChipKind::Spartan2 {
        (tcls::CNR_SW_S2, tcls::CNR_NW_S2)
    } else {
        (tcls::CNR_SW, tcls::CNR_NW)
    };

    {
        let mut ctx = FuzzCtx::new(session, backend, cnr_sw);
        let mut bctx = ctx.bel(bslots::MISC_SW);
        test_pull(&mut bctx, MISC_SW::M0_PULL, "M0PIN");
        test_pull(&mut bctx, MISC_SW::M1_PULL, "M1PIN");
        test_pull(&mut bctx, MISC_SW::M2_PULL, "M2PIN");
        test_pullup(&mut bctx, MISC_SW::POWERDOWN_PULL, "POWERDOWNPIN");
        test_pullup(&mut bctx, MISC_SW::PDSTATUS_PULL, "PDSTATUSPIN");
        bctx.build().test_global_attr_bool_rename(
            "DRIVEPDSTATUS",
            MISC_SW::DRIVE_PD_STATUS,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_rename("POWERUPDELAY", MISC_SW::POWERUP_DELAY);

        let mut bctx = ctx.bel(bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("CAPTURE")
            .commit();
        bctx.mode("CAPTURE")
            .pin("CLK")
            .test_bel_input_inv_enum("CLKINV", CAPTURE::CLK, "1", "0");
        for (val, vname) in [(false, "1"), (false, "CAP"), (true, "0"), (true, "CAP_B")] {
            bctx.mode("CAPTURE")
                .pin("CAP")
                .test_bel_input_inv(CAPTURE::CAP, val)
                .attr("CAPMUX", vname)
                .commit();
        }
        bctx.mode("CAPTURE")
            .null_bits()
            .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::CAPTURE_ONESHOT)
            .test_bel_special(specials::CAPTURE_ONESHOT)
            .attr("ONESHOT_ATTR", "ONE_SHOT")
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, cnr_nw);
        let mut bctx = ctx.bel(bslots::MISC_NW);
        test_pull(&mut bctx, MISC_NW::TCK_PULL, "TCKPIN");
        test_pull(&mut bctx, MISC_NW::TMS_PULL, "TMSPIN");
        bctx.build()
            .test_global_attr_rename("POWERUPCLK", MISC_NW::POWERUP_CLK);
        for (i, attr) in ["IBCLK_N2", "IBCLK_N4", "IBCLK_N8", "IBCLK_N16", "IBCLK_N32"]
            .into_iter()
            .enumerate()
        {
            for (val, vname) in [(false, "0"), (true, "1")] {
                bctx.build()
                    .test_bel_attr_bits_base_bi(MISC_NW::BCLK_DIV2, i, val)
                    .global(attr, vname)
                    .commit();
            }
        }

        let mut bctx = ctx.bel(bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("STARTUP")
            .commit();
        bctx.mode("STARTUP")
            .pin("CLK")
            .test_bel_input_inv_enum("CLKINV", STARTUP::CLK, "1", "0");
        for pin in [STARTUP::GSR, STARTUP::GWE, STARTUP::GTS] {
            let pname = backend.edev.db[STARTUP].inputs.key(pin).0;
            for (val, vname) in [
                (false, "1"),
                (false, pname),
                (true, "0"),
                (true, &format!("{pname}_B")),
            ] {
                bctx.mode("STARTUP")
                    .pin(pname)
                    .test_bel_input_inv(pin, val)
                    .attr(format!("{pname}MUX"), vname)
                    .commit();
            }
        }
        let wire_gwe = TileWireCoord::new_idx(0, wires::IMUX_STARTUP_GWE);
        let wire_gts = TileWireCoord::new_idx(0, wires::IMUX_STARTUP_GTS);
        let wire_gsr = TileWireCoord::new_idx(0, wires::IMUX_STARTUP_GSR);
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .no_pin("GWE")
            .test_bel_attr_bits(STARTUP::USER_GTS_GWE_GSR_ENABLE)
            .prop(WireMutexExclusive::new(wire_gwe))
            .prop(WireMutexExclusive::new(wire_gts))
            .prop(WireMutexExclusive::new(wire_gsr))
            .pin("GSR")
            .attr("GSRMUX", "GSR_B")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GSR")
            .no_pin("GWE")
            .test_bel_attr_bits(STARTUP::USER_GTS_GWE_GSR_ENABLE)
            .prop(WireMutexExclusive::new(wire_gwe))
            .prop(WireMutexExclusive::new(wire_gts))
            .prop(WireMutexExclusive::new(wire_gsr))
            .pin("GTS")
            .attr("GTSMUX", "GTS_B")
            .commit();
        bctx.mode("STARTUP")
            .no_pin("GTS")
            .no_pin("GSR")
            .test_bel_attr_bits(STARTUP::USER_GTS_GWE_GSR_ENABLE)
            .prop(WireMutexExclusive::new(wire_gwe))
            .prop(WireMutexExclusive::new(wire_gts))
            .prop(WireMutexExclusive::new(wire_gsr))
            .pin("GWE")
            .attr("GWEMUX", "GWE")
            .commit();
        bctx.build()
            .test_global_attr_bool_rename("GWE_SYNC", STARTUP::GWE_SYNC, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("GTS_SYNC", STARTUP::GTS_SYNC, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("GSR_SYNC", STARTUP::GSR_SYNC, "NO", "YES");
        for (val, vname) in [
            (enums::STARTUP_CLOCK::CCLK, "CCLK"),
            (enums::STARTUP_CLOCK::USERCLK, "USERCLK"),
            (enums::STARTUP_CLOCK::JTAGCLK, "JTAGCLK"),
        ] {
            bctx.mode("STARTUP")
                .null_bits()
                .pin("CLK")
                .extra_tiles_by_bel_attr_val(bslots::GLOBAL, GLOBAL::STARTUP_CLOCK, val)
                .test_bel_special(specials::STARTUPCLK)
                .global("STARTUPCLK", vname)
                .commit();
        }

        let mut bctx = ctx.bel(bslots::BSCAN);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("BSCAN")
            .commit();
        for pin in [BSCAN::TDO1, BSCAN::TDO2] {
            let pname = backend.edev.db[BSCAN].inputs.key(pin).0;
            for (val, vname) in [
                (false, "1"),
                (false, pname),
                (true, "0"),
                (true, &format!("{pname}_B")),
            ] {
                bctx.mode("BSCAN")
                    .pin(pname)
                    .test_bel_input_inv(pin, val)
                    .attr(format!("{pname}MUX"), vname)
                    .commit();
            }
        }
        bctx.build()
            .test_bel_attr_bits(BSCAN::USERCODE)
            .multi_global("USERID", MultiValue::HexPrefix, 32);
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_SE);
        let mut bctx = ctx.bel(bslots::MISC_SE);
        test_pullup(&mut bctx, MISC_SE::DONE_PULL, "DONEPIN");
        test_pullup(&mut bctx, MISC_SE::PROG_PULL, "PROGPIN");
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CNR_NE);
        let mut bctx = ctx.bel(bslots::MISC_NE);
        test_pull(&mut bctx, MISC_NE::TDI_PULL, "TDIPIN");
        test_pull(&mut bctx, MISC_NE::TDO_PULL, "TDOPIN");
        test_pullup(&mut bctx, MISC_NE::CCLK_PULL, "CCLKPIN");
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::GLOBAL);
        let mut bctx = ctx.bel(bslots::GLOBAL);
        // COR
        for (val, vname) in [
            (enums::STARTUP_CYCLE::_1, "1"),
            (enums::STARTUP_CYCLE::_2, "2"),
            (enums::STARTUP_CYCLE::_3, "3"),
            (enums::STARTUP_CYCLE::_4, "4"),
            (enums::STARTUP_CYCLE::_5, "5"),
            (enums::STARTUP_CYCLE::_6, "6"),
            (enums::STARTUP_CYCLE::DONE, "DONE"),
            (enums::STARTUP_CYCLE::KEEP, "KEEP"),
        ] {
            bctx.build()
                .test_bel_attr_val(GLOBAL::GSR_CYCLE, val)
                .global("GSR_CYCLE", vname)
                .commit();
            bctx.build()
                .test_bel_attr_val(GLOBAL::GWE_CYCLE, val)
                .global("GWE_CYCLE", vname)
                .commit();
            bctx.build()
                .test_bel_attr_val(GLOBAL::GTS_CYCLE, val)
                .global("GTS_CYCLE", vname)
                .commit();
            if val != enums::STARTUP_CYCLE::DONE {
                bctx.build()
                    .test_bel_attr_val(GLOBAL::DONE_CYCLE, val)
                    .global("DONE_CYCLE", vname)
                    .commit();
            }
        }
        for (val, vname) in [
            (enums::STARTUP_CYCLE::_0, "0"),
            (enums::STARTUP_CYCLE::_1, "1"),
            (enums::STARTUP_CYCLE::_2, "2"),
            (enums::STARTUP_CYCLE::_3, "3"),
            (enums::STARTUP_CYCLE::_4, "4"),
            (enums::STARTUP_CYCLE::_5, "5"),
            (enums::STARTUP_CYCLE::_6, "6"),
            (enums::STARTUP_CYCLE::NOWAIT, "NOWAIT"),
        ] {
            bctx.build()
                .test_bel_attr_val(GLOBAL::LOCK_CYCLE, val)
                .global("LCK_CYCLE", vname)
                .commit();
        }
        bctx.build()
            .test_global_attr_bool_rename("DRIVEDONE", GLOBAL::DRIVE_DONE, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("DONEPIPE", GLOBAL::DONE_PIPE, "NO", "YES");
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", GLOBAL::CONFIG_RATE);

        // CTL
        bctx.build()
            .test_global_attr_rename("SECURITY", GLOBAL::SECURITY);
        bctx.build()
            .test_global_attr_bool_rename("DISPMP1", GLOBAL::DISPMP1, "0", "1");
        bctx.build()
            .test_global_attr_bool_rename("DISPMP2", GLOBAL::DISPMP2, "0", "1");
        // persist not fuzzed — too much effort
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    for tcid in [tcls::PCI_W_V, tcls::PCI_W_VE, tcls::PCI_E_V, tcls::PCI_E_VE] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::PCILOGIC;
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        ctx.collect_bel_input_inv_bi(tcid, bslot, PCILOGIC::I1);
        ctx.collect_bel_input_inv_bi(tcid, bslot, PCILOGIC::I2);
        present.discard_polbits(&[
            ctx.bel_input_inv(tcid, bslot, PCILOGIC::I1),
            ctx.bel_input_inv(tcid, bslot, PCILOGIC::I2),
        ]);
        present.assert_empty();
        if matches!(tcid, tcls::PCI_W_V | tcls::PCI_E_V) {
            let d0 = ctx.get_diff_attr_u32(tcid, bslot, PCILOGIC::PCI_DELAY, 0);
            let d1 = ctx.get_diff_attr_u32(tcid, bslot, PCILOGIC::PCI_DELAY, 1);
            let d2 = ctx.get_diff_attr_u32(tcid, bslot, PCILOGIC::PCI_DELAY, 2);
            let d3 = ctx.get_diff_attr_u32(tcid, bslot, PCILOGIC::PCI_DELAY, 3);
            // bug? bug.
            assert_eq!(d0, d1);
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot,
                PCILOGIC::PCI_DELAY,
                xlat_bitvec_sparse_u32(vec![(0, d0), (2, d2), (3, d3)]),
            );
        } else {
            for val in 0..4 {
                ctx.get_diff_attr_u32(tcid, bslot, PCILOGIC::PCI_DELAY, val)
                    .assert_empty();
            }
        }
    }
    let (cnr_sw, cnr_nw) = if edev.chip.kind == ChipKind::Spartan2 {
        (tcls::CNR_SW_S2, tcls::CNR_NW_S2)
    } else {
        (tcls::CNR_SW, tcls::CNR_NW)
    };
    {
        let tcid = cnr_sw;
        let bslot = bslots::MISC_SW;
        for attr in [MISC_SW::M0_PULL, MISC_SW::M1_PULL, MISC_SW::M2_PULL] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }
        if tcid == tcls::CNR_SW_S2 {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                MISC_SW::POWERDOWN_PULL,
                &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP],
            );
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                MISC_SW::PDSTATUS_PULL,
                &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP],
            );
            ctx.collect_bel_attr(tcid, bslot, MISC_SW::POWERUP_DELAY);
            ctx.collect_bel_attr_bi(tcid, bslot, MISC_SW::DRIVE_PD_STATUS);
        } else {
            for (attr, val) in [
                (MISC_SW::POWERDOWN_PULL, enums::IOB_PULL::PULLUP),
                (MISC_SW::POWERDOWN_PULL, enums::IOB_PULL::NONE),
                (MISC_SW::PDSTATUS_PULL, enums::IOB_PULL::PULLUP),
                (MISC_SW::PDSTATUS_PULL, enums::IOB_PULL::NONE),
                (MISC_SW::POWERUP_DELAY, enums::POWERUP_DELAY::_100US),
                (MISC_SW::POWERUP_DELAY, enums::POWERUP_DELAY::_200US),
                (MISC_SW::POWERUP_DELAY, enums::POWERUP_DELAY::_400US),
            ] {
                ctx.get_diff_attr_val(tcid, bslot, attr, val).assert_empty();
            }
            for val in [false, true] {
                ctx.get_diff_attr_bool_bi(tcid, bslot, MISC_SW::DRIVE_PD_STATUS, val)
                    .assert_empty();
            }
        }

        let bslot = bslots::CAPTURE;
        ctx.collect_bel_input_inv_bi(tcid, bslot, CAPTURE::CAP);
        ctx.collect_bel_input_inv_bi(tcid, bslot, CAPTURE::CLK);
    }
    {
        let tcid = cnr_nw;
        let bslot = bslots::MISC_NW;
        for attr in [MISC_NW::TCK_PULL, MISC_NW::TMS_PULL] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }

        ctx.collect_bel_attr_bi(tcid, bslot, MISC_NW::BCLK_DIV2);
        if tcid == tcls::CNR_NW_S2 {
            ctx.collect_bel_attr(tcid, bslot, MISC_NW::POWERUP_CLK);
        } else {
            for (attr, val) in [
                (MISC_NW::POWERUP_CLK, enums::POWERUP_CLK::USERCLK),
                (MISC_NW::POWERUP_CLK, enums::POWERUP_CLK::INTOSC),
                (MISC_NW::POWERUP_CLK, enums::POWERUP_CLK::CCLK),
            ] {
                ctx.get_diff_attr_val(tcid, bslot, attr, val).assert_empty();
            }
        }

        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_bi(tcid, bslot, STARTUP::GWE_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, STARTUP::GSR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, STARTUP::GTS_SYNC);
        ctx.collect_bel_input_inv_bi(tcid, bslot, STARTUP::GSR);
        ctx.collect_bel_input_inv_bi(tcid, bslot, STARTUP::GWE);
        ctx.collect_bel_input_inv_bi(tcid, bslot, STARTUP::GTS);
        ctx.collect_bel_input_inv_bi(tcid, bslot, STARTUP::CLK);
        ctx.collect_bel_attr(tcid, bslot, STARTUP::USER_GTS_GWE_GSR_ENABLE);

        let bslot = bslots::BSCAN;
        ctx.collect_bel_input_inv_bi(tcid, bslot, BSCAN::TDO1);
        ctx.collect_bel_input_inv_bi(tcid, bslot, BSCAN::TDO2);
        ctx.collect_bel_attr(tcid, bslot, BSCAN::USERCODE);
    }
    {
        let tcid = tcls::CNR_SE;
        let bslot = bslots::MISC_SE;
        for attr in [MISC_SE::DONE_PULL, MISC_SE::PROG_PULL] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP],
            );
        }
    }
    {
        let tcid = tcls::CNR_NE;
        let bslot = bslots::MISC_NE;
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            MISC_NE::CCLK_PULL,
            &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP],
        );
        for attr in [MISC_NE::TDI_PULL, MISC_NE::TDO_PULL] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[
                    enums::IOB_PULL::NONE,
                    enums::IOB_PULL::PULLUP,
                    enums::IOB_PULL::PULLDOWN,
                ],
            );
        }
    }

    {
        let tcid = tcls::GLOBAL;
        let bslot = bslots::GLOBAL;

        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::GSR_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::GWE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::GTS_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::DONE,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::DONE_CYCLE,
            &[
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::KEEP,
            ],
        );
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            GLOBAL::LOCK_CYCLE,
            &[
                enums::STARTUP_CYCLE::_0,
                enums::STARTUP_CYCLE::_1,
                enums::STARTUP_CYCLE::_2,
                enums::STARTUP_CYCLE::_3,
                enums::STARTUP_CYCLE::_4,
                enums::STARTUP_CYCLE::_5,
                enums::STARTUP_CYCLE::_6,
                enums::STARTUP_CYCLE::NOWAIT,
            ],
        );
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::STARTUP_CLOCK);
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::SHUTDOWN, TileBit::new(0, 0, 15).pos());
        ctx.collect_bel_attr_ocd(tcid, bslot, GLOBAL::CONFIG_RATE, OcdMode::BitOrder);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::CAPTURE_ONESHOT);

        // CTL
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DISPMP1);
        ctx.collect_bel_attr_bi(tcid, bslot, GLOBAL::DISPMP2);
        ctx.collect_bel_attr(tcid, bslot, GLOBAL::SECURITY);
        // these are too much trouble to deal with the normal way.
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::PERSIST, TileBit::new(1, 0, 6).pos());
        ctx.insert_bel_attr_bool(tcid, bslot, GLOBAL::GTS_USR_B, TileBit::new(1, 0, 0).pos());
    }
}
