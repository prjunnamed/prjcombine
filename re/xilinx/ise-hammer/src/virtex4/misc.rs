use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelInfo, WireSlotIdExt},
    grid::{DieId, TileCoord},
};
use prjcombine_re_collector::diff::{OcdMode, xlat_enum_raw};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    self, bcls, bslots, enums, tslots,
    virtex4::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{mutex::WireMutexExclusive, relation::TileRelation},
    },
    virtex4::specials,
};

#[derive(Clone, Copy, Debug)]
struct HclkDcm;

impl TileRelation for HclkDcm {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(
            tcrd.cell
                .with_cr(edev.col_clk, edev.chips[tcrd.die].row_hclk(tcrd.row))
                .tile(defs::tslots::HCLK_BEL),
        )
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let global = edev.tile_cfg(DieId::from_idx(0)).tile(tslots::GLOBAL);

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::CFG);

        let mut bctx = ctx.bel(bslots::MISC_CFG);
        for (val, vname) in [
            (enums::PROBESEL::_0, "0"),
            (enums::PROBESEL::_1, "1"),
            (enums::PROBESEL::_2, "2"),
            (enums::PROBESEL::_3, "3"),
        ] {
            bctx.build()
                .test_bel_attr_val(bcls::MISC_CFG::PROBESEL, val)
                .global("PROBESEL", vname)
                .commit();
        }
        for (attr, opt) in [
            (bcls::MISC_CFG::CCLK_PULL, "CCLKPIN"),
            (bcls::MISC_CFG::DONE_PULL, "DONEPIN"),
            (bcls::MISC_CFG::POWERDOWN_PULL, "POWERDOWNPIN"),
            (bcls::MISC_CFG::PROG_PULL, "PROGPIN"),
            (bcls::MISC_CFG::INIT_PULL, "INITPIN"),
        ] {
            for (val, vname) in [
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::NONE, "PULLNONE"),
            ] {
                bctx.build()
                    .test_bel_attr_val(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }
        for (attr, opt) in [
            (bcls::MISC_CFG::HSWAPEN_PULL, "HSWAPENPIN"),
            (bcls::MISC_CFG::M0_PULL, "M0PIN"),
            (bcls::MISC_CFG::M1_PULL, "M1PIN"),
            (bcls::MISC_CFG::M2_PULL, "M2PIN"),
            (bcls::MISC_CFG::CS_PULL, "CSPIN"),
            (bcls::MISC_CFG::DIN_PULL, "DINPIN"),
            (bcls::MISC_CFG::BUSY_PULL, "BUSYPIN"),
            (bcls::MISC_CFG::RDWR_PULL, "RDWRPIN"),
            (bcls::MISC_CFG::TCK_PULL, "TCKPIN"),
            (bcls::MISC_CFG::TDI_PULL, "TDIPIN"),
            (bcls::MISC_CFG::TDO_PULL, "TDOPIN"),
            (bcls::MISC_CFG::TMS_PULL, "TMSPIN"),
        ] {
            for (val, vname) in [
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
                (enums::IOB_PULL::NONE, "PULLNONE"),
            ] {
                bctx.build()
                    .test_bel_attr_val(attr, val)
                    .global(opt, vname)
                    .commit();
            }
        }
        bctx.build()
            .test_bel_attr_bits(bcls::MISC_CFG::USERCODE)
            .multi_global("USERID", MultiValue::HexPrefix, 32);

        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::BSCAN[i]);
            bctx.build()
                .test_bel_attr_bits(bcls::BSCAN::ENABLE)
                .mode("BSCAN")
                .commit();
        }

        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::ICAP[i]);
            let obel = bslots::ICAP[i ^ 1];
            bctx.build()
                .bel_unused(obel)
                .test_bel_attr_bits(bcls::ICAP_V4::ENABLE)
                .mode("ICAP")
                .commit();
            bctx.mode("ICAP")
                .test_bel_input_inv_auto(bcls::ICAP_V4::CLK);
            bctx.mode("ICAP").test_bel_input_inv_auto(bcls::ICAP_V4::CE);
            bctx.mode("ICAP")
                .test_bel_input_inv_auto(bcls::ICAP_V4::WRITE);
            for (val, vname) in [
                (enums::ICAP_WIDTH::X8, "X8"),
                (enums::ICAP_WIDTH::X32, "X32"),
            ] {
                bctx.mode("ICAP")
                    .bel_unused(obel)
                    .test_bel(bslots::MISC_CFG)
                    .test_bel_attr_val(bcls::MISC_CFG::ICAP_WIDTH, val)
                    .attr("ICAP_WIDTH", vname)
                    .commit();
            }
        }

        {
            let mut bctx = ctx.bel(bslots::PMV_CFG[0]);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("PMV")
                .commit();
        }

        {
            let mut bctx = ctx.bel(bslots::STARTUP);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("STARTUP")
                .commit();
            for pin in [
                bcls::STARTUP::CLK,
                bcls::STARTUP::GTS,
                bcls::STARTUP::GSR,
                bcls::STARTUP::USRCCLKTS,
                bcls::STARTUP::USRCCLKO,
                bcls::STARTUP::USRDONETS,
                bcls::STARTUP::USRDONEO,
            ] {
                bctx.mode("STARTUP").test_bel_input_inv_auto(pin);
            }
            bctx.mode("STARTUP")
                .no_pin("GSR")
                .test_bel_attr_bits(bcls::STARTUP::USER_GTS_GSR_ENABLE)
                .pin("GTS")
                .commit();
            bctx.mode("STARTUP")
                .no_pin("GTS")
                .test_bel_attr_bits(bcls::STARTUP::USER_GTS_GSR_ENABLE)
                .pin("GSR")
                .commit();
            bctx.mode("STARTUP")
                .test_bel_attr_bits(bcls::STARTUP::USRCCLK_ENABLE)
                .pin("USRCCLKO")
                .commit();
            for attr in [
                bcls::STARTUP::GSR_SYNC,
                bcls::STARTUP::GWE_SYNC,
                bcls::STARTUP::GTS_SYNC,
            ] {
                bctx.build().test_global_attr_bool_rename(
                    backend.edev.db[bcls::STARTUP].attributes.key(attr),
                    attr,
                    "NO",
                    "YES",
                );
            }
            for (val, vname) in [
                (enums::STARTUP_CLOCK::CCLK, "CCLK"),
                (enums::STARTUP_CLOCK::USERCLK, "USERCLK"),
                (enums::STARTUP_CLOCK::JTAGCLK, "JTAGCLK"),
            ] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .null_bits()
                    .extra_fixed_bel_attr_val(
                        global,
                        bslots::GLOBAL,
                        bcls::GLOBAL::STARTUP_CLOCK,
                        val,
                    )
                    .test_bel_special_val(specials::STARTUP_CLOCK, val)
                    .global("STARTUPCLK", vname)
                    .commit();
            }
        }

        {
            let mut bctx = ctx.bel(bslots::JTAGPPC);
            bctx.build()
                .test_bel_attr_bits(bcls::JTAGPPC::ENABLE)
                .mode("JTAGPPC")
                .commit();
        }

        {
            let mut bctx = ctx.bel(bslots::FRAME_ECC);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("FRAME_ECC")
                .commit();
        }

        {
            let mut bctx = ctx.bel(bslots::DCIRESET);
            bctx.build()
                .test_bel_attr_bits(bcls::DCIRESET::ENABLE)
                .mode("DCIRESET")
                .commit();
        }

        {
            let mut bctx = ctx.bel(bslots::CAPTURE);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("CAPTURE")
                .commit();
            bctx.mode("CAPTURE")
                .test_bel_input_inv_auto(bcls::CAPTURE::CLK);
            bctx.mode("CAPTURE")
                .test_bel_input_inv_auto(bcls::CAPTURE::CAP);
            for val in [false, true] {
                bctx.mode("CAPTURE")
                    .null_bits()
                    .extra_fixed_bel_attr_bits_bi(
                        global,
                        bslots::GLOBAL,
                        bcls::GLOBAL::CAPTURE_ONESHOT,
                        val,
                    )
                    .test_bel_special(specials::CAPTURE_ONESHOT)
                    .attr("ONESHOT", if val { "TRUE" } else { "FALSE" })
                    .commit();
            }
        }

        {
            let mut bctx = ctx.bel(bslots::USR_ACCESS);
            bctx.build()
                .null_bits()
                .test_bel_special(specials::PRESENT)
                .mode("USR_ACCESS")
                .commit();
        }
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
                .test_bel_attr_val(bcls::GLOBAL::GWE_CYCLE, val)
                .global("GWE_CYCLE", vname)
                .commit();
            bctx.build()
                .test_bel_attr_val(bcls::GLOBAL::GTS_CYCLE, val)
                .global("GTS_CYCLE", vname)
                .commit();
            if val != enums::STARTUP_CYCLE::DONE {
                bctx.build()
                    .test_bel_attr_val(bcls::GLOBAL::DONE_CYCLE, val)
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
                .test_bel_attr_val(bcls::GLOBAL::LOCK_CYCLE, val)
                .global("LCK_CYCLE", vname)
                .commit();
            bctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_bel_attr_val(bcls::GLOBAL::MATCH_CYCLE, val)
                .global("MATCH_CYCLE", vname)
                .commit();
        }
        bctx.build().test_global_attr_bool_rename(
            "DRIVEDONE",
            bcls::GLOBAL::DRIVE_DONE,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_bool_rename("DONEPIPE", bcls::GLOBAL::DONE_PIPE, "NO", "YES");
        bctx.build().test_global_attr_bool_rename(
            "DCMSHUTDOWN",
            bcls::GLOBAL::DCM_SHUTDOWN,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "POWERDOWNSTATUS",
            bcls::GLOBAL::POWERDOWN_STATUS,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "CRC",
            bcls::GLOBAL::CRC_ENABLE,
            "DISABLE",
            "ENABLE",
        );
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", bcls::GLOBAL::CONFIG_RATE_V4);

        // CTL
        bctx.build().test_global_attr_bool_rename(
            "GTS_USR_B",
            bcls::GLOBAL::GTS_USR_B,
            "NO",
            "YES",
        );
        bctx.build()
            .test_global_attr_bool_rename("VGG_TEST", bcls::GLOBAL::VGG_TEST, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("ENCRYPT", bcls::GLOBAL::ENCRYPT, "NO", "YES");
        bctx.build()
            .test_global_attr_bool_rename("EN_VTEST", bcls::GLOBAL::EN_VTEST, "NO", "YES");
        // persist not fuzzed — too much effort
        bctx.build()
            .test_global_attr_rename("SECURITY", bcls::GLOBAL::SECURITY);

        // TODO: more crap
        for val in ["NO", "YES"] {
            bctx.build()
                .null_bits()
                .test_bel_special(specials::DISABLE_BANDGAP)
                .global("DISABLEBANDGAP", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            bctx.build()
                .null_bits()
                .test_bel_special(specials::DCI_SHUTDOWN)
                .global("DCISHUTDOWN", val)
                .commit();
        }
    }

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::SYSMON) {
        let tcid = tcls::SYSMON;
        let tcls = &backend.edev.db[tcid];
        let muxes = &backend.edev.db_index.tile_classes[tcid].muxes;

        let mut bctx = ctx.bel(bslots::SYSMON);
        let mode = "MONITOR";
        bctx.build()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for i in 0x40..0x70 {
            bctx.mode(mode)
                .test_bel_attr_bits_base(bcls::SYSMON_V4::INIT, (i - 0x40) * 16)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 16);
        }
        bctx.mode(mode)
            .global_mutex("MONITOR_GLOBAL", "NONE")
            .test_bel_attr_auto(bcls::SYSMON_V4::MONITOR_MODE);
        for pin in [
            bcls::SYSMON_V4::DEN,
            // DCLK?
            bcls::SYSMON_V4::DWE,
            bcls::SYSMON_V4::SCANTESTENA,
            bcls::SYSMON_V4::SCANTESTENB,
            // SCANMEMCLK?
            bcls::SYSMON_V4::SCANMEMWE,
            bcls::SYSMON_V4::ROMTESTENABLE,
            bcls::SYSMON_V4::RST,
            bcls::SYSMON_V4::CONVST,
            // SCLK[AB]?
            bcls::SYSMON_V4::SEA,
            bcls::SYSMON_V4::SEB,
        ] {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }
        for (attr, width) in [
            (bcls::SYSMON_V4::DCLK_DIVID_2, 1),
            (bcls::SYSMON_V4::LW_DIVID_2_4, 1),
            (bcls::SYSMON_V4::MCCLK_DIVID, 8),
            (bcls::SYSMON_V4::OVER_TEMPERATURE, 10),
            (bcls::SYSMON_V4::OVER_TEMPERATURE_OFF, 1),
            (bcls::SYSMON_V4::OVER_TEMPERATURE_DELAY, 8),
            (bcls::SYSMON_V4::BLOCK_ENABLE, 5),
            (bcls::SYSMON_V4::DCLK_MISSING, 10),
            (bcls::SYSMON_V4::FEATURE_ENABLE, 8),
            (bcls::SYSMON_V4::PROM_DATA, 8),
        ] {
            let aname = backend.edev.db[bcls::SYSMON_V4].attributes.key(attr);
            bctx.mode(mode)
                .global_mutex_here("MONITOR_GLOBAL")
                .attr("MONITOR_MODE", "ADC")
                .test_bel_attr_bits(attr)
                .multi_global(format!("ADC_{aname}"), MultiValue::Bin, width);
        }

        let BelInfo::Bel(ref bel) = tcls.bels[bslots::SYSMON] else {
            unreachable!()
        };
        let mux = &muxes[&bel.inputs[bcls::SYSMON_V4::CONVST].wire()];

        for out in ["CONVST", "CONVST_TEST"] {
            for &src in mux.src.keys() {
                let mut builder = bctx
                    .build()
                    .mutex("CONVST_OUT", out)
                    .prop(WireMutexExclusive::new(mux.dst));
                if let Some(idx) = wires::GIOB_DCM.index_of(src.wire) {
                    builder = builder
                        .related_tile_mutex(HclkDcm, "HCLK_DCM", "USE")
                        .related_pip(
                            HclkDcm,
                            wires::GIOB_DCM[idx].cell(1),
                            wires::GIOB[idx].cell(2),
                        )
                        .related_pip(
                            HclkDcm,
                            wires::GIOB_DCM[idx].cell(2),
                            wires::GIOB[idx].cell(2),
                        );
                }
                builder.test_routing(mux.dst, src).pip(out, src.tw).commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    {
        let tcid = tcls::CFG;
        let bslot = bslots::MISC_CFG;
        ctx.collect_bel_attr_default(tcid, bslot, bcls::MISC_CFG::PROBESEL, enums::PROBESEL::NONE);
        for attr in [
            bcls::MISC_CFG::CCLK_PULL,
            bcls::MISC_CFG::DONE_PULL,
            bcls::MISC_CFG::POWERDOWN_PULL,
            bcls::MISC_CFG::PROG_PULL,
            bcls::MISC_CFG::INIT_PULL,
        ] {
            ctx.collect_bel_attr_subset(
                tcid,
                bslot,
                attr,
                &[enums::IOB_PULL::NONE, enums::IOB_PULL::PULLUP],
            );
        }
        for attr in [
            bcls::MISC_CFG::HSWAPEN_PULL,
            bcls::MISC_CFG::M0_PULL,
            bcls::MISC_CFG::M1_PULL,
            bcls::MISC_CFG::M2_PULL,
            bcls::MISC_CFG::CS_PULL,
            bcls::MISC_CFG::DIN_PULL,
            bcls::MISC_CFG::BUSY_PULL,
            bcls::MISC_CFG::RDWR_PULL,
            bcls::MISC_CFG::TCK_PULL,
            bcls::MISC_CFG::TDI_PULL,
            bcls::MISC_CFG::TDO_PULL,
            bcls::MISC_CFG::TMS_PULL,
        ] {
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
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_CFG::USERCODE);
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::MISC_CFG::ICAP_WIDTH,
            &[enums::ICAP_WIDTH::X8, enums::ICAP_WIDTH::X32],
        );

        for bslot in bslots::BSCAN {
            ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::ENABLE);
        }
        for bslot in bslots::ICAP {
            ctx.collect_bel_attr(tcid, bslot, bcls::ICAP_V4::ENABLE);
        }
        ctx.collect_bel_attr(tcid, bslots::JTAGPPC, bcls::JTAGPPC::ENABLE);
        ctx.collect_bel_attr(tcid, bslots::DCIRESET, bcls::DCIRESET::ENABLE);

        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GSR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GWE_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GTS_SYNC);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::USER_GTS_GSR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::USRCCLK_ENABLE);
        for pin in [
            bcls::STARTUP::CLK,
            bcls::STARTUP::GSR,
            bcls::STARTUP::GTS,
            bcls::STARTUP::USRDONETS,
            bcls::STARTUP::USRDONEO,
            bcls::STARTUP::USRCCLKTS,
            bcls::STARTUP::USRCCLKO,
        ] {
            ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 16], tcid, bslot, pin);
        }

        for bslot in bslots::ICAP {
            for pin in [bcls::ICAP_V4::CLK, bcls::ICAP_V4::CE, bcls::ICAP_V4::WRITE] {
                ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 16], tcid, bslot, pin);
            }
        }

        let bslot = bslots::CAPTURE;
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 16], tcid, bslot, bcls::CAPTURE::CLK);
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 16], tcid, bslot, bcls::CAPTURE::CAP);
    }

    // config regs
    {
        let tcid = tcls::GLOBAL;
        let bslot = bslots::GLOBAL;

        // COR
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::GWE_CYCLE,
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
            bcls::GLOBAL::GTS_CYCLE,
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
            bcls::GLOBAL::DONE_CYCLE,
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
            bcls::GLOBAL::LOCK_CYCLE,
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
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            bcls::GLOBAL::MATCH_CYCLE,
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
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::STARTUP_CLOCK);
        ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::CONFIG_RATE_V4, OcdMode::BitOrder);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DCM_SHUTDOWN);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POWERDOWN_STATUS);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CRC_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CAPTURE_ONESHOT);

        // CTL
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::GTS_USR_B);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::VGG_TEST);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::EN_VTEST);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::ENCRYPT);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SECURITY);
        // these are too much trouble to deal with the normal way.
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::PERSIST,
            TileBit::new(1, 0, 3).pos(),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::GLUTMASK,
            TileBit::new(1, 0, 8).neg(),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::GLOBAL::ICAP_SELECT,
            BelAttributeEnum {
                bits: vec![TileBit::new(1, 0, 30)],
                values: [
                    (enums::ICAP_SELECT::TOP, bits![0]),
                    (enums::ICAP_SELECT::BOTTOM, bits![1]),
                ]
                .into_iter()
                .collect(),
            },
        );
    }

    if ctx.has_tcls(tcls::SYSMON) {
        let tcid = tcls::SYSMON;
        let muxes = &ctx.edev.db_index.tile_classes[tcid].muxes;

        let bslot = bslots::SYSMON;
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V4::MONITOR_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V4::INIT);
        for pin in [
            bcls::SYSMON_V4::DEN,
            bcls::SYSMON_V4::DWE,
            bcls::SYSMON_V4::SCANTESTENA,
            bcls::SYSMON_V4::SCANTESTENB,
            bcls::SYSMON_V4::SCANMEMWE,
            bcls::SYSMON_V4::ROMTESTENABLE,
            bcls::SYSMON_V4::RST,
            bcls::SYSMON_V4::SEA,
            bcls::SYSMON_V4::SEB,
        ] {
            ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 8], tcid, bslot, pin);
        }
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SYSMON_V4::CONVST);
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        for (attr, val) in [
            (bcls::SYSMON_V4::DCLK_DIVID_2, 0),
            (bcls::SYSMON_V4::LW_DIVID_2_4, 0),
            (bcls::SYSMON_V4::MCCLK_DIVID, 0xc8),
            (bcls::SYSMON_V4::OVER_TEMPERATURE, 0x31e),
            (bcls::SYSMON_V4::OVER_TEMPERATURE_OFF, 0),
            (bcls::SYSMON_V4::OVER_TEMPERATURE_DELAY, 0),
            (bcls::SYSMON_V4::BLOCK_ENABLE, 0x1e),
            (bcls::SYSMON_V4::DCLK_MISSING, 0x320),
            (bcls::SYSMON_V4::FEATURE_ENABLE, 0),
            (bcls::SYSMON_V4::PROM_DATA, 0),
        ] {
            ctx.collect_bel_attr(tcid, bslot, attr);
            present.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, attr), val, 0);
        }
        present.assert_empty();

        let BelInfo::Bel(ref bel) = ctx.edev.db[tcid].bels[bslots::SYSMON] else {
            unreachable!()
        };
        let mux = &muxes[&bel.inputs[bcls::SYSMON_V4::CONVST].wire()];

        let mut diffs = vec![];
        for &src in mux.src.keys() {
            let mut diff = ctx.get_diff_routing(tcid, mux.dst, src);
            if wires::IMUX_CLK_OPTINV.contains(src.wire) {
                let item = ctx.item_int_inv_raw(&[tcls::INT; 8], src.tw);
                diff.apply_bit_diff(item, false, true);
            }
            diffs.push((Some(src), diff));
        }
        ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
    }
}
