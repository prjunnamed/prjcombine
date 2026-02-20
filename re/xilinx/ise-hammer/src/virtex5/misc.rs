use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::BelAttributeEnum,
    grid::{DieId, TileCoord},
};
use prjcombine_re_collector::diff::{DiffKey, OcdMode, SpecialId};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{self, bcls, bslots, enums, tslots, virtex5::tcls};
use prjcombine_xilinx_bitstream::{BitRect, Reg};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{DynProp, relation::Delta},
    },
    virtex4::specials,
};

#[derive(Clone, Debug)]
struct RegPresentSpecial(Reg, SpecialId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for RegPresentSpecial {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for die in backend.edev.die() {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::GlobalSpecial(self.1),
                rects: EntityVec::from_iter([BitRect::RegPresent(die, self.0)]),
            });
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let global = edev.tile_cfg(DieId::from_idx(0)).tile(tslots::GLOBAL);

    let mut ctx = FuzzCtx::new(session, backend, tcls::CFG);

    let mut bctx = ctx.bel(bslots::MISC_CFG);
    for (attr, opt) in [
        (bcls::MISC_CFG::CCLK_PULL, "CCLKPIN"),
        (bcls::MISC_CFG::DONE_PULL, "DONEPIN"),
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
        let mut bctx = ctx.bel(defs::bslots::BSCAN[i]);
        bctx.build()
            .test_bel_attr_bits(bcls::BSCAN::ENABLE)
            .mode("BSCAN")
            .commit();
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::ICAP[i]);
        bctx.build()
            .test_bel_attr_bits(bcls::ICAP_V4::ENABLE)
            .mode("ICAP")
            .commit();
        bctx.mode("ICAP")
            .global_mutex_here("ICAP")
            .test_bel(bslots::MISC_CFG)
            .test_bel_attr_auto(bcls::MISC_CFG::ICAP_WIDTH);
    }
    {
        let mut bctx = ctx.bel(defs::bslots::PMV_CFG[0]);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMV")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::STARTUP);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("STARTUP")
            .commit();
        for (val, vname) in [
            (enums::STARTUP_CLOCK::CCLK, "CCLK"),
            (enums::STARTUP_CLOCK::USERCLK, "USERCLK"),
            (enums::STARTUP_CLOCK::JTAGCLK, "JTAGCLK"),
        ] {
            bctx.mode("STARTUP")
                .null_bits()
                .pin("CLK")
                .extra_fixed_bel_attr_val(global, bslots::GLOBAL, bcls::GLOBAL::STARTUP_CLOCK, val)
                .test_bel_special_val(specials::STARTUP_CLOCK, val)
                .global("STARTUPCLK", vname)
                .commit();
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
        for attr in [bcls::STARTUP::GSR_SYNC, bcls::STARTUP::GTS_SYNC] {
            bctx.build().test_global_attr_bool_rename(
                backend.edev.db[bcls::STARTUP].attributes.key(attr),
                attr,
                "NO",
                "YES",
            );
        }
    }
    {
        let mut bctx = ctx.bel(defs::bslots::JTAGPPC);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("JTAGPPC")
            .commit();
        bctx.mode("JTAGPPC")
            .test_bel_attr_auto(bcls::JTAGPPC::NUM_PPC);
    }
    {
        let mut bctx = ctx.bel(defs::bslots::FRAME_ECC);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("FRAME_ECC")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::DCIRESET);
        bctx.build()
            .test_bel_attr_bits(bcls::DCIRESET::ENABLE)
            .mode("DCIRESET")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::CAPTURE);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("CAPTURE")
            .commit();
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
        let mut bctx = ctx.bel(defs::bslots::USR_ACCESS);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("USR_ACCESS")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::KEY_CLEAR);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("KEY_CLEAR")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::EFUSE_USR);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("EFUSE_USR")
            .commit();
    }
    {
        let mut bctx = ctx.bel(defs::bslots::SYSMON);
        bctx.build()
            .null_bits()
            .extra_tile_attr_bits(
                Delta::new(0, 10, tcls::HCLK_IO_CFG_N),
                bslots::HCLK_CMT_DRP,
                bcls::HCLK_CMT_DRP::DRP_MASK,
            )
            .test_bel_special(specials::PRESENT)
            .mode("SYSMON")
            .commit();
        bctx.mode("SYSMON")
            .test_bel_input_inv_auto(bcls::SYSMON_V5::DCLK);
        bctx.mode("SYSMON")
            .test_bel_input_inv_auto(bcls::SYSMON_V5::CONVSTCLK);
        for i in 0x40..0x58 {
            let base = (i - 0x40) * 0x10;
            bctx.mode("SYSMON")
                .test_bel_attr_bits_base(bcls::SYSMON_V5::INIT, base)
                .multi_attr(format!("INIT_{i:02X}"), MultiValue::Hex(0), 16);
        }
        for attr in [
            bcls::SYSMON_V5::SYSMON_TEST_A,
            bcls::SYSMON_V5::SYSMON_TEST_B,
            bcls::SYSMON_V5::SYSMON_TEST_C,
            bcls::SYSMON_V5::SYSMON_TEST_D,
            bcls::SYSMON_V5::SYSMON_TEST_E,
        ] {
            bctx.mode("SYSMON")
                .test_bel_attr_multi(attr, MultiValue::Hex(0));
        }
        bctx.build()
            .attr("SYSMON_TEST_A", "")
            .test_bel_special(specials::JTAG_SYSMON_DISABLE)
            .global("JTAG_SYSMON", "DISABLE")
            .commit();
    }

    let mut ctx = FuzzCtx::new(session, backend, tcls::GLOBAL);
    {
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
            "CRC",
            bcls::GLOBAL::CRC_ENABLE,
            "DISABLE",
            "ENABLE",
        );
        bctx.build()
            .test_global_attr_rename("CONFIGRATE", bcls::GLOBAL::CONFIG_RATE_V5);

        // COR1
        bctx.build()
            .test_global_attr_rename("BPI_PAGE_SIZE", bcls::GLOBAL::BPI_PAGE_SIZE);
        bctx.build()
            .global("BPI_PAGE_SIZE", "8")
            .test_global_attr_rename("BPI_1ST_READ_CYCLE", bcls::GLOBAL::BPI_1ST_READ_CYCLE);
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_EN",
            bcls::GLOBAL::POST_CRC_EN,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_NO_PIN",
            bcls::GLOBAL::POST_CRC_NO_PIN,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_RECONFIG",
            bcls::GLOBAL::POST_CRC_RECONFIG,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "RETAINCONFIGSTATUS",
            bcls::GLOBAL::RETAIN_CONFIG_STATUS,
            "NO",
            "YES",
        );
        bctx.build().test_global_attr_bool_rename(
            "POST_CRC_SEL",
            bcls::GLOBAL::POST_CRC_SEL,
            "0",
            "1",
        );

        // CTL
        bctx.build()
            .global("CONFIGFALLBACK", "DISABLE")
            .test_global_attr_bool_rename("ENCRYPT", bcls::GLOBAL::ENCRYPT, "NO", "YES");
        // persist not fuzzed — too much effort
        bctx.build()
            .test_global_attr_rename("SECURITY", bcls::GLOBAL::SECURITY);
        bctx.build()
            .test_global_attr_rename("ENCRYPTKEYSELECT", bcls::GLOBAL::ENCRYPT_KEY_SELECT);
        bctx.build().test_global_attr_bool_rename(
            "OVERTEMPPOWERDOWN",
            bcls::GLOBAL::OVERTEMP_POWERDOWN,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "CONFIGFALLBACK",
            bcls::GLOBAL::CONFIG_FALLBACK,
            "DISABLE",
            "ENABLE",
        );
        bctx.build().test_global_attr_bool_rename(
            "SELECTMAPABORT",
            bcls::GLOBAL::SELECTMAP_ABORT,
            "DISABLE",
            "ENABLE",
        );
        bctx.build()
            .test_global_attr_bool_rename("GLUTMASK_B", bcls::GLOBAL::GLUTMASK, "1", "0");

        for (attr, opt) in [
            (bcls::GLOBAL::VBG_SEL, "VBG_SEL"),
            (bcls::GLOBAL::VBG_DLL_SEL, "VBG_DLL_SEL"),
            (bcls::GLOBAL::VGG_SEL, "VGG_SEL"),
        ] {
            bctx.build()
                .test_bel_attr_bits(attr)
                .multi_global(opt, MultiValue::Bin, 5);
        }

        // TIMER
        bctx.build()
            .no_global("TIMER_USR")
            .test_bel_attr_bits(bcls::GLOBAL::TIMER_CFG)
            .global("TIMER_CFG", "0")
            .commit();
        bctx.build()
            .no_global("TIMER_CFG")
            .test_bel_attr_bits(bcls::GLOBAL::TIMER_USR)
            .global("TIMER_USR", "0")
            .commit();
        bctx.build()
            .no_global("TIMER_USR")
            .test_bel_attr_bits(bcls::GLOBAL::TIMER)
            .multi_global("TIMER_CFG", MultiValue::Hex(0), 24);

        // TESTMODE
        bctx.build()
            .prop(RegPresentSpecial(
                Reg::Testmode,
                specials::REG_TESTMODE_PRESENT,
            ))
            .test_bel_attr_bits(bcls::GLOBAL::DD_OVERRIDE)
            .global("DD_OVERRIDE", "YES")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CFG;

    {
        let bslot = bslots::MISC_CFG;
        for attr in [
            bcls::MISC_CFG::CCLK_PULL,
            bcls::MISC_CFG::DONE_PULL,
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
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_CFG::ICAP_WIDTH);
    }
    for bslot in bslots::BSCAN {
        ctx.collect_bel_attr(tcid, bslot, bcls::BSCAN::ENABLE);
    }
    for bslot in bslots::ICAP {
        ctx.collect_bel_attr(tcid, bslot, bcls::ICAP_V4::ENABLE);
    }
    ctx.collect_bel_attr(tcid, bslots::DCIRESET, bcls::DCIRESET::ENABLE);

    {
        let bslot = bslots::STARTUP;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GSR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::STARTUP::GTS_SYNC);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::USER_GTS_GSR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::STARTUP::USRCCLK_ENABLE);
    }

    {
        let bslot = bslots::JTAGPPC;
        ctx.collect_bel_attr(tcid, bslot, bcls::JTAGPPC::NUM_PPC);
    }

    {
        let bslot = bslots::SYSMON;
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SYSMON_V5::CONVSTCLK);
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::SYSMON_V5::DCLK);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::INIT);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_A);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_B);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_C);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_D);
        ctx.collect_bel_attr(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_E);

        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::JTAG_SYSMON_DISABLE);
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::SYSMON_V5::SYSMON_TEST_A),
            2,
            0,
        );
        diff.assert_empty();
    }

    {
        let tcid = tcls::HCLK_IO_CFG_N;
        let bslot = bslots::HCLK_CMT_DRP;
        ctx.collect_bel_attr(tcid, bslot, bcls::HCLK_CMT_DRP::DRP_MASK);
    }

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
        ctx.collect_bel_attr_ocd(tcid, bslot, bcls::GLOBAL::CONFIG_RATE_V5, OcdMode::BitOrder);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DRIVE_DONE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::DONE_PIPE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CRC_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CAPTURE_ONESHOT);

        // COR1
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::BPI_PAGE_SIZE);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::BPI_1ST_READ_CYCLE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_EN);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_NO_PIN);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_RECONFIG);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::RETAIN_CONFIG_STATUS);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::POST_CRC_SEL);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::PERSIST_DEASSERT_AT_DESYNCH,
            TileBit::new(1, 0, 17).pos(),
        );

        // CTL
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::SECURITY);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::ENCRYPT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::OVERTEMP_POWERDOWN);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::CONFIG_FALLBACK);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::SELECTMAP_ABORT);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::GLOBAL::GLUTMASK);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::ENCRYPT_KEY_SELECT);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::VBG_SEL);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::VBG_DLL_SEL);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::VGG_SEL);
        // these are too much trouble to deal with the normal way.
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::GTS_USR_B,
            TileBit::new(2, 0, 0).pos(),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::PERSIST,
            TileBit::new(2, 0, 3).pos(),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::GLOBAL::ICAP_SELECT,
            BelAttributeEnum {
                bits: vec![TileBit::new(2, 0, 30)],
                values: [
                    (enums::ICAP_SELECT::TOP, bits![0]),
                    (enums::ICAP_SELECT::BOTTOM, bits![1]),
                ]
                .into_iter()
                .collect(),
            },
        );

        // TIMER
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::TIMER);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::TIMER_CFG);
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::TIMER_USR);

        // WBSTAR
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::V5_NEXT_CONFIG_ADDR,
            (0..26).map(|i| TileBit::new(5, 0, i).pos()).collect(),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::GLOBAL::REVISION_SELECT_TRISTATE,
            TileBit::new(5, 0, 26).neg(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::GLOBAL::REVISION_SELECT,
            (27..29).map(|i| TileBit::new(5, 0, i).pos()).collect(),
        );

        // TESTMODE
        ctx.collect_bel_attr(tcid, bslot, bcls::GLOBAL::DD_OVERRIDE);
        ctx.get_diff_global_special(specials::REG_TESTMODE_PRESENT);
    }
}
